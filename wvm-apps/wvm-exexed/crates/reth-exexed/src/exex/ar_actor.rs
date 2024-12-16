use crate::{network_tag::get_network_tag, new_etl_exex_biguery_client};

use arweave_upload::{ArweaveRequest, UploaderProvider};
use exex_wvm_bigquery::{repository::StateRepository, BigQueryClient};
use exex_wvm_da::{DefaultWvmDataSettler, WvmDataSettler};
use futures::{stream::FuturesUnordered, StreamExt};
use std::fmt;
use tokio::sync::mpsc;

use reth::primitives::revm_primitives::alloy_primitives::BlockNumber;
use reth_primitives::SealedBlockWithSenders;
use std::{
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};
use tracing::{error, info};
use wvm_borsh::block::BorshSealedBlockWithSenders;

enum ArActorMessage {
    ProcessBlock { block: SealedBlockWithSenders, notification_type: String },
    Shutdown,
}

// Response type wrapping results and errors
type ArActorResponse = Result<String, ArActorError>;

#[derive(Debug)]
pub enum ArActorError {
    BlockExists,
    SerializationFailed {
        block_number: u64,
        error: String,
    },
    ArweaveUploadFailed {
        block_number: u64,
        error: String,
    },
    BigQueryError {
        block_number: u64,
        operation: &'static str, // "tags" or "block"
        error: String,
    },
    ActorUnavailable,
    ResponseError,
}

impl std::error::Error for ArActorError {}

impl fmt::Display for ArActorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ArActorError::BlockExists => write!(f, "Block already exists"),
            ArActorError::SerializationFailed { block_number, error } => {
                write!(f, "Failed to serialize block {}: {}", block_number, error)
            }
            ArActorError::ArweaveUploadFailed { block_number, error } => {
                write!(f, "Failed to upload block {} to Arweave: {}", block_number, error)
            }
            ArActorError::BigQueryError { block_number, operation, error } => {
                write!(
                    f,
                    "BigQuery {} operation failed for block {}: {}",
                    operation, block_number, error
                )
            }
            ArActorError::ActorUnavailable => write!(f, "Actor is unavailable"),
            ArActorError::ResponseError => write!(f, "Failed to receive response from actor"),
        }
    }
}

/// Main actor struct that maintains state and processes messages
struct ArActor {
    receiver: mpsc::Receiver<ArActorMessage>,
    state_repo: Arc<StateRepository>,
    big_query_client: Arc<BigQueryClient>,
    ar_uploader: UploaderProvider,
}

impl ArActor {
    fn new(
        receiver: mpsc::Receiver<ArActorMessage>,
        state_repo: Arc<StateRepository>,
        big_query_client: Arc<BigQueryClient>,
    ) -> Self {
        Self { receiver, state_repo, big_query_client, ar_uploader: UploaderProvider::new(None) }
    }

    async fn run(mut self) {
        info!(target: "wvm::exex", "ArActor started");
        let mut in_flight = FuturesUnordered::new();

        loop {
            tokio::select! {
                Some(msg) = self.receiver.recv() => {
                    match msg {
                        ArActorMessage::Shutdown => {
                            info!(target: "wvm:exex:ArActor", "ArActor shutting down");
                            break;
                        }
                        ArActorMessage::ProcessBlock { block, notification_type } => {
                            let state_repo = self.state_repo.clone();
                            let big_query_client = self.big_query_client.clone();
                            let ar_uploader = self.ar_uploader.clone();
                            let block_number = block.number;

                            in_flight.push(tokio::spawn(async move {
                            match handle_block(
                                block,
                                &notification_type,
                                state_repo,
                                big_query_client,
                                ar_uploader,
                            ).await {
                                Ok(arweave_id) => {
                                    info!(
                                        target: "wvm::exex",
                                        block_number = block_number,
                                        arweave_id = arweave_id,
                                        "Block processed successfully"
                                    );
                                    Ok(block_number)
                                }
                                Err(ArActorError::BlockExists) => {
                                    info!(
                                        target: "wvm::exex",
                                        block_number = block_number,
                                        "Block already exists"
                                    );
                                    Ok(block_number)
                                }
                                Err(e) => {
                                    error!(
                                        target: "wvm::exex",
                                        %e,
                                        block_number = block_number,
                                        "Failed to process block"
                                    );
                                    Err((block_number, e))
                                }
                            }
                            }));
                       }
                    }
                }
                Some(result) = in_flight.next() => {
                    match result {
                        Ok(Ok(block_number)) => {
                            info!(target: "wvm::exex", "Completed block {}", block_number);
                        }
                        Ok(Err((block_number, e))) => {
                            error!(target: "wvm::exex", "Failed block {}: {:?}", block_number, e);
                        }
                        Err(e) => {
                            error!(target: "wvm::exex", "Task panicked: {}", e);
                        }
                    }
                }
                else => break,
            }
        }

        let remaining = in_flight.len();
        if remaining > 0 {
            info!(
                target: "wvm::exex",
                "Shutting down with {} unfinished blocks",
                remaining
            );
        }
    }
}

// Keep in same file but separate from ArActor
async fn handle_block(
    block: SealedBlockWithSenders,
    notification_type: &str,
    state_repo: Arc<StateRepository>,
    big_query_client: Arc<BigQueryClient>,
    ar_uploader: UploaderProvider,
) -> ArActorResponse {
    let block_hash_str = block.hash().to_string();
    let block_number = block.number;

    // 1. Serialize block
    info!(target: "wvm::exex", "Block {} processing: starting serialization", block_number);
    let borsh_brotli = serialize_block(&block)?;
    // 2. Upload to Arweave
    info!(target: "wvm::exex", "Block {} processing: starting Arweave upload", block_number);
    let arweave_id = upload_to_arweave(
        &ar_uploader,
        &borsh_brotli,
        notification_type,
        block_number,
        &block_hash_str,
    )
    .await?;

    // 3. Update BigQuery
    info!(target: "wvm::exex", "Block {} processing: starting BigQuery update", block_number);
    update_bigquery(&state_repo, &big_query_client, &block, block.number, &arweave_id).await?;

    info!(target: "wvm::exex", "Block {} processing: completed", block_number);
    Ok(arweave_id)
}

fn serialize_block(sealed_block: &SealedBlockWithSenders) -> Result<Vec<u8>, ArActorError> {
    let data_settler = DefaultWvmDataSettler;
    let borsh_sealed_block = BorshSealedBlockWithSenders(sealed_block.clone());

    data_settler.process_block(&borsh_sealed_block).map_err(|e| ArActorError::SerializationFailed {
        block_number: sealed_block.number,
        error: e.to_string(),
    })
}

async fn upload_to_arweave(
    ar_uploader: &UploaderProvider,
    data: &[u8],
    notification_type: &str,
    block_number: BlockNumber,
    block_hash: &str,
) -> Result<String, ArActorError> {
    ArweaveRequest::new()
        .set_tag("Content-Type", "application/octet-stream")
        .set_tag("WeaveVM:Encoding", "Borsh-Brotli")
        .set_tag("Block-Number", block_number.to_string().as_str())
        .set_tag("Block-Hash", block_hash)
        .set_tag("Client-Version", reth_primitives::constants::RETH_CLIENT_VERSION)
        .set_tag("Network", get_network_tag().as_str())
        .set_tag("WeaveVM:Internal-Chain", notification_type)
        .set_data(data.to_vec())
        .send_with_provider(ar_uploader)
        .await
        .map_err(|e| ArActorError::ArweaveUploadFailed { block_number, error: e.to_string() })
}

async fn update_bigquery(
    state_repo: &Arc<StateRepository>,
    big_query_client: &Arc<BigQueryClient>,
    block: &SealedBlockWithSenders,
    block_number: BlockNumber,
    arweave_id: &str,
) -> Result<(), ArActorError> {
    // Update tags
    update_bigquery_tags(big_query_client, block).await?;

    // Then save block
    exex_wvm_bigquery::save_block(state_repo.clone(), block, block_number, arweave_id.to_string())
        .await
        .map_err(|e| ArActorError::BigQueryError {
            block_number,
            operation: "block",
            error: e.to_string(),
        })?;

    Ok(())
}

async fn update_bigquery_tags(
    big_query_client: &Arc<BigQueryClient>,
    sealed_block: &SealedBlockWithSenders,
) -> Result<(), ArActorError> {
    let hashes: Vec<String> =
        sealed_block.body.transactions.iter().map(|e| e.hash.to_string()).collect();

    if hashes.is_empty() {
        return Ok(());
    }

    let query = generate_tags_query(big_query_client, hashes, sealed_block.number)?;

    big_query_client.bq_query(query.clone()).await.map_err(|e| ArActorError::BigQueryError {
        block_number: sealed_block.number,
        operation: "tags",
        error: e.to_string(),
    })?;

    info!(
        target: "wvm::exex",
        "Tags at block {} updated successfully",
        sealed_block.number
    );

    Ok(())
}

fn generate_tags_query(
    big_query_client: &Arc<BigQueryClient>,
    hashes: Vec<String>,
    block_number: BlockNumber,
) -> Result<String, ArActorError> {
    let confirmed_tags_tbl_name = format!(
        "`{}.{}.{}`",
        big_query_client.project_id, big_query_client.dataset_id, "confirmed_tags"
    );
    let tags_tbl_name =
        format!("`{}.{}.{}`", big_query_client.project_id, big_query_client.dataset_id, "tags");

    let in_clause =
        hashes.into_iter().map(|hash| format!("\"{}\"", hash)).collect::<Vec<String>>().join(", ");

    let ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| ArActorError::BigQueryError {
            block_number,
            operation: "tags",
            error: format!("Time error: {}", e),
        })?
        .as_millis();

    Ok(format!(
        "INSERT INTO {} (tx_hash, tags, block_id, `timestamp`) SELECT t.hash, t.tags, {}, {} FROM {} t WHERE t.hash IN({}) AND t.created_at <= {}",
        confirmed_tags_tbl_name, block_number, ms, tags_tbl_name, in_clause, ms
    ))
}

// Actor Handle
#[derive(Clone)]
pub struct ArweaveActorHandle {
    sender: mpsc::Sender<ArActorMessage>,
}

impl ArweaveActorHandle {
    pub async fn new(buffer_size: usize) -> Self {
        info!(target: "wvm::exex", "Creating new ArweaveActor with buffer size {}", buffer_size);
        let (sender, receiver) = mpsc::channel(buffer_size);
        let big_query_client = Arc::new(new_etl_exex_biguery_client().await);
        let state_repo = Arc::new(StateRepository::new(big_query_client.clone()));

        let actor = ArActor::new(receiver, state_repo, big_query_client);

        tokio::spawn(actor.run());

        Self { sender }
    }

    pub async fn process_block(
        &self,
        block: SealedBlockWithSenders,
        notification_type: String,
    ) -> Result<(), ArActorError> {
        self.sender
            .send(ArActorMessage::ProcessBlock { block, notification_type })
            .await
            .map_err(|_| ArActorError::ActorUnavailable)
    }

    pub async fn shutdown(&self) -> Result<(), ArActorError> {
        self.sender.send(ArActorMessage::Shutdown).await.map_err(|_| ArActorError::ActorUnavailable)
    }
}
