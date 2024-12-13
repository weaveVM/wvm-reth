use crate::{
    network_tag::get_network_tag, new_etl_exex_biguery_client, util::check_block_existence,
};
use arweave_upload::{ArweaveRequest, UploaderProvider};
use exex_wvm_bigquery::{repository::StateRepository, BigQueryClient};
use exex_wvm_da::{DefaultWvmDataSettler, WvmDataSettler};
use tokio::sync::{mpsc, oneshot};

use reth::primitives::revm_primitives::alloy_primitives::BlockNumber;
use reth_primitives::SealedBlockWithSenders;
use std::{
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};
use tokio::{sync::mpsc::Sender, task::JoinHandle};
use tracing::{error, info};
use wvm_borsh::block::BorshSealedBlockWithSenders;
use wvm_static::SUPERVISOR_RT;

enum ArActorMessage {
    ProcessBlock {
        block: SealedBlockWithSenders,
        notification_type: String,
        respond_to: oneshot::Sender<ArActorResponse>,
    },
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

use std::fmt;

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

        while let Some(msg) = self.receiver.recv().await {
            match msg {
                ArActorMessage::Shutdown => {
                    info!(target: "wvm:exex:ArActor", "ArActor shutting down");
                    break;
                }
                ArActorMessage::ProcessBlock { block, notification_type, respond_to } => {
                    let result = self.handle_block(block, &notification_type).await;
                    let _ = respond_to.send(result);
                }
            }
        }
    }

    async fn handle_block(
        &self,
        sealed_block: SealedBlockWithSenders,
        notification_type: &str,
    ) -> ArActorResponse {
        let block_hash_str = sealed_block.hash().to_string();
        if check_block_existence(block_hash_str.as_str(), false).await {
            return Err(ArActorError::BlockExists);
        }

        // 1. Serialize block
        let borsh_brotli = self.serialize_block(&sealed_block)?;

        // 2. Upload to Arweave
        let arweave_id = self
            .upload_to_arweave(
                &borsh_brotli,
                notification_type,
                sealed_block.number,
                block_hash_str.as_str(),
            )
            .await?;

        // 3. Update BigQuery
        self.update_bigquery(&sealed_block, sealed_block.number, &arweave_id).await?;

        Ok(arweave_id)
    }

    fn serialize_block(
        &self,
        sealed_block: &SealedBlockWithSenders,
    ) -> Result<Vec<u8>, ArActorError> {
        let data_settler = DefaultWvmDataSettler;
        let borsh_sealed_block = BorshSealedBlockWithSenders(sealed_block.clone());

        data_settler.process_block(&borsh_sealed_block).map_err(|e| {
            ArActorError::SerializationFailed {
                block_number: sealed_block.number,
                error: e.to_string(),
            }
        })
    }

    async fn upload_to_arweave(
        &self,
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
            .send_with_provider(&self.ar_uploader)
            .await
            .map_err(|e| ArActorError::ArweaveUploadFailed { block_number, error: e.to_string() })
    }

    async fn update_bigquery(
        &self,
        block: &SealedBlockWithSenders,
        block_number: BlockNumber,
        arweave_id: &str,
    ) -> Result<(), ArActorError> {
        // Update tags
        self.update_bigquery_tags(block).await?;

        // Then save block
        exex_wvm_bigquery::save_block(
            self.state_repo.clone(),
            block,
            block_number,
            arweave_id.to_string(),
        )
        .await
        .map_err(|e| ArActorError::BigQueryError {
            block_number,
            operation: "block",
            error: e.to_string(),
        })?;

        Ok(())
    }

    async fn update_bigquery_tags(
        &self,
        sealed_block: &SealedBlockWithSenders,
    ) -> Result<(), ArActorError> {
        let hashes: Vec<String> =
            sealed_block.body.transactions.iter().map(|e| e.hash.to_string()).collect();

        if hashes.is_empty() {
            return Ok(());
        }

        // Generate BigQuery update query
        let query = self.generate_tags_query(hashes, sealed_block.number)?;

        self.big_query_client.bq_query(query.clone()).await.map_err(|e| {
            ArActorError::BigQueryError {
                block_number: sealed_block.number,
                operation: "tags",
                error: e.to_string(),
            }
        })?;

        info!(
            target: "wvm::exex",
            "Tags at block {} updated successfully",
            sealed_block.number
        );

        Ok(())
    }

    fn generate_tags_query(
        &self,
        hashes: Vec<String>,
        block_number: BlockNumber,
    ) -> Result<String, ArActorError> {
        let confirmed_tags_tbl_name = format!(
            "`{}.{}.{}`",
            self.big_query_client.project_id, self.big_query_client.dataset_id, "confirmed_tags"
        );
        let tags_tbl_name = format!(
            "`{}.{}.{}`",
            self.big_query_client.project_id, self.big_query_client.dataset_id, "tags"
        );

        // Generate the WHERE clause
        let in_clause = hashes
            .into_iter()
            .map(|hash| format!("\"{}\"", hash))
            .collect::<Vec<String>>()
            .join(", ");

        let ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| ArActorError::BigQueryError {
                block_number,
                operation: "tags",
                error: format!("Time error: {}", e),
            })?
            .as_millis();

        // Generate the final query
        Ok(
            format!(
                "INSERT INTO {} (tx_hash, tags, block_id, `timestamp`) SELECT t.hash, t.tags, {}, {} FROM {} t WHERE t.hash IN({}) AND t.created_at <= {}",
                confirmed_tags_tbl_name, block_number, ms, tags_tbl_name, in_clause, ms
            ))
    }
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
    ) -> Result<String, ArActorError> {
        let (send, recv) = oneshot::channel();

        let msg = ArActorMessage::ProcessBlock { block, notification_type, respond_to: send };

        self.sender.send(msg).await.map_err(|_| ArActorError::ActorUnavailable)?;

        recv.await.map_err(|_| ArActorError::ResponseError)?
    }

    pub async fn shutdown(&self) -> Result<(), ArActorError> {
        self.sender.send(ArActorMessage::Shutdown).await.map_err(|_| ArActorError::ActorUnavailable)
    }
}

pub struct ArProcess {
    buffer_size: usize,
    pub sender: Sender<(SealedBlockWithSenders, String)>,
    thread: JoinHandle<()>,
}

impl ArProcess {
    fn new(buffer_size: usize) -> Self {
        let (sender, mut receiver) =
            tokio::sync::mpsc::channel::<(SealedBlockWithSenders, String)>(buffer_size);

        let thread = SUPERVISOR_RT.spawn(async move {
            let big_query_client = new_etl_exex_biguery_client().await;
            let big_query_client = Arc::new(big_query_client);
            let state_repo = Arc::new(StateRepository::new(big_query_client.clone()));

            while let Some(msg) = receiver.recv().await {
                let state_repo = state_repo.clone();
                let big_query_client = big_query_client.clone();
                let ar_uploader_provider = UploaderProvider::new(None);
                tokio::spawn(async move {
                    let sealed_block = msg.0;
                    let notification_type = msg.1.as_str();

                    let block_number = sealed_block.block.header.header().number;
                    let block_hash = sealed_block.block.hash().to_string();
                    if check_block_existence(block_hash.as_str(), false).await {
                        return
                    }

                    let sealed_block_clone = sealed_block.clone();

                    let borsh_brotli = match Self::serialize_block(sealed_block_clone) {
                        Some(value) => value,
                        None => return,
                    };


                        let provider = ar_uploader_provider.clone();
                        let arweave_id = match Self::send_block_to_arweave(&provider, notification_type, block_number, &block_hash, borsh_brotli).await {
                            Some(ar_id) => ar_id,
                            None => return,
                        };

                        info!(target: "wvm::exex", "irys id: {}, for block: {}", arweave_id, block_number);

                        let _ = Self::bigquery_tags(big_query_client.clone(), &sealed_block).await;
                        let _ = Self::bigquery_task(state_repo.clone(), sealed_block, block_number, arweave_id).await;
                });
            }
        });

        Self { sender, buffer_size, thread }
    }

    async fn bigquery_tags(client: Arc<BigQueryClient>, sealed_block: &SealedBlockWithSenders) {
        let hashes: Vec<String> =
            sealed_block.body.transactions.iter().map(|e| e.hash.to_string()).collect();
        let block_number = sealed_block.block.header.header().number;
        let confirmed_tags_tbl_name =
            format!("`{}.{}.{}`", client.project_id, client.dataset_id, "confirmed_tags");
        let tags_tbl_name = format!("`{}.{}.{}`", client.project_id, client.dataset_id, "tags");

        if hashes.is_empty() {
            return;
        }

        let query = {
            // Generate the WHERE clause
            let in_clause = hashes
                .into_iter()
                .map(|hash| format!("\"{}\"", hash))
                .collect::<Vec<String>>()
                .join(", ");

            let ms = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis();

            // Generate the final query
            format!(
                "INSERT INTO {} (tx_hash, tags, block_id, `timestamp`) SELECT t.hash, t.tags, {}, {} FROM {} t WHERE t.hash IN({}) AND t.created_at <= {}",
                confirmed_tags_tbl_name, block_number, ms, tags_tbl_name, in_clause, ms
            )
        };

        let run_q = client.bq_query(query.clone()).await;
        if let Err(e) = run_q {
            error!(target: "wvm::exex", %e, "Failed to write to bigquery, block {}. Query: {}", block_number, query);
        } else {
            info!(target: "wvm::exex", "Tags at block {} updated successfully", block_number);
        };
    }

    async fn bigquery_task(
        state_repo: Arc<StateRepository>,
        sealed_block: SealedBlockWithSenders,
        block_number: BlockNumber,
        arweave_id: String,
    ) {
        if let Err(err) = exex_wvm_bigquery::save_block(
            state_repo,
            &sealed_block,
            block_number,
            arweave_id.clone(),
        )
        .await
        {
            error!(target: "wvm::exex", %err, "Failed to write to bigquery, block {}", block_number);
        };
    }

    async fn send_block_to_arweave(
        ar_uploader_provider: &UploaderProvider,
        notification_type: &str,
        block_number: BlockNumber,
        block_hash: &String,
        brotli_borsh: Vec<u8>,
    ) -> Option<String> {
        let res = ArweaveRequest::new()
            .set_tag("Content-Type", "application/octet-stream")
            .set_tag("WeaveVM:Encoding", "Borsh-Brotli")
            .set_tag("Block-Number", &block_number.to_string().as_str())
            .set_tag("Block-Hash", &block_hash)
            .set_tag("Client-Version", reth_primitives::constants::RETH_CLIENT_VERSION)
            .set_tag("Network", get_network_tag().as_str())
            .set_tag("WeaveVM:Internal-Chain", notification_type)
            .set_data(brotli_borsh)
            .send_with_provider(&ar_uploader_provider)
            .await;

        let arweave_id = match res {
            Ok(arweave_id) => arweave_id,
            Err(err) => {
                error!(target: "wvm::exex", %err, "Failed to construct arweave_id for block {}", block_number);
                return None;
            }
        };

        Some(arweave_id)
    }

    fn serialize_block(msg: SealedBlockWithSenders) -> Option<Vec<u8>> {
        let data_settler = DefaultWvmDataSettler;
        let block_number = msg.block.header.header().number;

        let borsh_sealed_block = BorshSealedBlockWithSenders(msg);
        let brotli_borsh = match data_settler.process_block(&borsh_sealed_block) {
            Ok(data) => data,
            Err(err) => {
                error!(target: "wvm::exex", %err, "Failed to do brotli encoding for block {}", block_number);
                return None;
            }
        };
        Some(brotli_borsh)
    }
}
