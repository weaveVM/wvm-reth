use crate::network_tag::get_network_tag;

use alloy_primitives::BlockNumber;
use arweave_upload::{ArweaveRequest, UploaderProvider};
use exex_wvm_da::{DefaultWvmDataSettler, WvmDataSettler};
use futures::{stream::FuturesUnordered, StreamExt};
use load_db::LoadDbConnection;
use reth_primitives::SealedBlockWithSenders;
use reth_primitives_traits::{self, RecoveredBlock};
use std::{
    fmt,
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};
use tokio::sync::mpsc;
use tracing::{error, info};
use wvm_borsh::block::BorshSealedBlockWithSenders;
use wvm_tx::wvm::{v1::V1WvmSealedBlockWithSenders, WvmSealedBlockWithSenders};

#[derive(Clone)] // Add this
enum ArActorMessage {
    ProcessBlock { block: RecoveredBlock<reth_primitives::Block>, notification_type: String },
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
    StoreBlockDataError {
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
            ArActorError::StoreBlockDataError { block_number, operation, error } => {
                write!(f, "{} operation failed for block {}: {}", operation, block_number, error)
            }
            ArActorError::ActorUnavailable => write!(f, "Actor is unavailable"),
            ArActorError::ResponseError => write!(f, "Failed to receive response from actor"),
        }
    }
}

/// Main actor struct that maintains state and processes messages
struct ArActor {
    receiver: mpsc::Receiver<ArActorMessage>,
    load_db_repo: Arc<dyn LoadDbConnection + Send + Sync + 'static>,
    ar_uploader: UploaderProvider,
    worker_id: usize,
}

impl ArActor {
    fn new(
        receiver: mpsc::Receiver<ArActorMessage>,
        load_db_repo: Arc<dyn LoadDbConnection + Send + Sync + 'static>,
        worker_id: usize,
    ) -> Self {
        Self { receiver, load_db_repo, ar_uploader: UploaderProvider::new(None), worker_id }
    }

    async fn run(mut self) {
        info!(
            target: "wvm::exex",
            worker_id = self.worker_id,
            "ArActor worker started"
        );

        let mut in_flight = FuturesUnordered::new();

        loop {
            tokio::select! {
                Some(msg) = self.receiver.recv() => {
                    match msg {
                        ArActorMessage::Shutdown => {
                            info!(
                                target: "wvm::exex",
                                worker_id = self.worker_id,
                                "ArActor worker shutting down"
                            );
                            break;
                        }
                        ArActorMessage::ProcessBlock { block, notification_type } => {
                            let load_db_repo = self.load_db_repo.clone();
                            let ar_uploader = self.ar_uploader.clone();
                            let block_number = block.number;
                            let worker_id = self.worker_id;

                            info!(
                                target: "wvm::exex",
                                worker_id = self.worker_id,
                                block_number,
                                in_flight_count = in_flight.len(),
                                "Starting block processing"
                            );

                            in_flight.push(tokio::spawn(async move {
                                match handle_block(
                                    block,
                                    &notification_type,
                                load_db_repo,
                                    ar_uploader,
                                ).await {
                                    Ok(arweave_id) => {
                                        info!(
                                            target: "wvm::exex",
                                            worker_id,
                                            block_number,
                                            arweave_id,
                                            "Block processed successfully"
                                        );
                                        Ok(block_number)
                                    }
                                    Err(ArActorError::BlockExists) => {
                                        info!(
                                            target: "wvm::exex",
                                            worker_id,
                                            block_number,
                                            "Block already exists"
                                        );
                                        Ok(block_number)
                                    }
                                    Err(e) => {
                                        error!(
                                            target: "wvm::exex",
                                            worker_id,
                                            %e,
                                            block_number,
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
                            info!(
                                target: "wvm::exex",
                                worker_id = self.worker_id,
                                block_number,
                                "Completed block processing"
                            );
                        }
                        Ok(Err((block_number, e))) => {
                            error!(
                                target: "wvm::exex",
                                worker_id = self.worker_id,
                                block_number,
                                error = %e,
                                "Failed to process block"
                            );
                        }
                        Err(e) => {
                            error!(
                                target: "wvm::exex",
                                worker_id = self.worker_id,
                                error = %e,
                                "Task panicked"
                            );
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
                worker_id = self.worker_id,
                remaining,
                "Worker shutting down with unfinished blocks"
            );
        }
    }
}

// Keep in same file but separate from ArActor
async fn handle_block(
    block: RecoveredBlock<reth_primitives::Block>,
    notification_type: &str,
    load_db_repo: Arc<dyn LoadDbConnection + Send + Sync + 'static>,
    ar_uploader: UploaderProvider,
) -> ArActorResponse {
    let block_hash_str = block.hash().to_string();
    let block_number = block.number;

    // 1. Serialize block
    info!(target: "wvm::exex", "Block {} processing: starting serialization", block_number);
    let borsh_brotli = serialize_block(block.clone())?;
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

    // 3. Save Block
    info!(target: "wvm::exex", "Block {} processing: starting to save block in load db store", block_number);
    save_block(load_db_repo, &block, block.number, &arweave_id).await?;

    info!(target: "wvm::exex", "Block {} processing: completed", block_number);
    Ok(arweave_id)
}

fn serialize_block(msg: RecoveredBlock<reth_primitives::Block>) -> Result<Vec<u8>, ArActorError> {
    let data_settler = DefaultWvmDataSettler;
    let block_number = msg.clone().into_header().number;

    let data = WvmSealedBlockWithSenders::V1(V1WvmSealedBlockWithSenders::from(msg));

    let borsh_sealed_block = BorshSealedBlockWithSenders(data);

    data_settler
        .process_block(&borsh_sealed_block)
        .map_err(|e| ArActorError::SerializationFailed { block_number, error: e.to_string() })
}

async fn upload_to_arweave(
    ar_uploader: &UploaderProvider,
    data: &[u8],
    notification_type: &str,
    block_number: BlockNumber,
    block_hash: &str,
) -> Result<String, ArActorError> {
    let start_time = std::time::Instant::now();

    // First upload (Arweave data settlement)
    let mut request = ArweaveRequest::new();
    request
        .set_tag("Content-Type", "application/octet-stream")
        .set_tag("WeaveVM:Encoding", "Borsh-Brotli")
        .set_tag("LN:Encoding", "Borsh-Brotli")
        .set_tag("Block-Number", block_number.to_string().as_str())
        .set_tag("Block-Hash", block_hash)
        .set_tag("Client-Version", reth_primitives_traits::constants::RETH_CLIENT_VERSION)
        .set_tag("Network", get_network_tag().as_str())
        .set_tag("WeaveVM:Internal-Chain", notification_type)
        .set_tag("LN:Internal-Chain", notification_type)
        .set_data(data.to_vec());

    match request.send_with_provider(ar_uploader).await {
        Ok(response) => {
            let total_duration = start_time.elapsed();
            info!(
                target: "wvm::exex",
                block_number = block_number,
                total_duration = ?total_duration.as_millis(),
                "Arweave upload completed successfully: {}",
                response
            );

            // After first successful upload, attest Arweave data settlement to AO (WeaveDrive)
            // https://hackmd.io/@ao-docs/H1JK_WezR
            let second_start_time = std::time::Instant::now();
            let mut second_request = ArweaveRequest::new();
            second_request
                .set_tag("Content-Type", "application/json")
                .set_tag("LN:Encoding", "JSON")
                .set_tag("Data-Protocol", "ao")
                .set_tag("Type", "Attestation")
                .set_tag("Message", &response)
                .set_tag("Block-Number", block_number.to_string().as_str())
                .set_tag("Block-Hash", block_hash)
                .set_data(
                    serde_json::json!({
                        "data_settlement_tx_id": response,
                        "block_number": block_number,
                        "block_hash": block_hash,
                    })
                    .to_string()
                    .into_bytes(),
                );

            match second_request.send_with_provider(ar_uploader).await {
                Ok(second_tx_id) => {
                    info!(
                        target: "wvm::exex",
                        block_number = block_number,
                        primary_tx = response,
                        secondary_tx = second_tx_id,
                        secondary_duration = ?second_start_time.elapsed().as_millis(),
                        "Secondary Arweave upload completed successfully"
                    );
                }
                Err(e) => {
                    error!(
                        target: "wvm::exex",
                        block_number = block_number,
                        primary_tx = response,
                        error = %e,
                        duration = ?second_start_time.elapsed().as_millis(),
                        "Secondary Arweave upload failed (WeaveDrive attestation)"
                    );
                }
            }

            Ok(response)
        }
        Err(e) => {
            let total_duration = start_time.elapsed();
            error!(
                target: "wvm::exex",
                block_number = block_number,
                total_duration = ?total_duration.as_millis(),
                error = %e,
                "Failed to upload block to Arweave"
            );

            Err(ArActorError::ArweaveUploadFailed { block_number, error: e.to_string() })
        }
    }
}

async fn save_block(
    load_db_repo: Arc<dyn LoadDbConnection + Send + Sync + 'static>,
    block: &RecoveredBlock<reth_primitives::Block>,
    block_number: BlockNumber,
    arweave_id: &str,
) -> Result<(), ArActorError> {
    let start_time = std::time::Instant::now();

    update_tags(&load_db_repo, block).await?;

    let save_block_start_time = std::time::Instant::now();
    let block_hash = block.hash().to_string();

    let result =
        load_db_repo.save_block(block, block_number, arweave_id.to_string(), block_hash).await;

    let save_block_duration = save_block_start_time.elapsed();
    match result {
        Ok(_) => {
            let total_duration = start_time.elapsed();
            info!(
                target: "wvm::exex",
                block_number = block_number,
                total_duration = ?total_duration.as_millis(),
                "save block completed successfully"
            );
            Ok(())
        }
        Err(e) => {
            let total_duration = start_time.elapsed();
            error!(
                target: "wvm::exex",
                block_number = block_number,
                total_duration = ?total_duration.as_millis(),
                error = %e,
                "save block failed"
            );

            Err(ArActorError::StoreBlockDataError {
                block_number,
                operation: "save_block",
                error: e.to_string(),
            })
        }
    }
}

async fn update_tags(
    load_db_repo: &Arc<dyn LoadDbConnection + Send + Sync + 'static>,
    sealed_block: &RecoveredBlock<reth_primitives::Block>,
) -> Result<(), ArActorError> {
    let hashes: Vec<String> =
        sealed_block.body().transactions.iter().map(|e| e.hash().to_string()).collect();

    if hashes.is_empty() {
        return Ok(());
    }

    let start_time = std::time::Instant::now();
    load_db_repo.save_hashes(hashes.as_slice(), sealed_block.number).await.map_err(|e| {
        ArActorError::StoreBlockDataError {
            block_number: sealed_block.number,
            operation: "update tags",
            error: e.to_string(),
        }
    })?;
    let duration = start_time.elapsed();

    info!(
        target: "wvm::exex",
        block_number = sealed_block.number,
        duration = ?duration.as_millis(),
        "Tags updated successfully",
    );

    Ok(())
}

#[derive(Clone)]
pub struct ArweaveActorHandle {
    sender: mpsc::Sender<ArActorMessage>,
}

impl ArweaveActorHandle {
    pub async fn new(
        buffer_size: usize,
        load_db_repo: Arc<dyn LoadDbConnection + Send + Sync + 'static>,
    ) -> Self {
        info!(target: "wvm::exex", "Creating new ArweaveActor with buffer size {}", buffer_size);

        let (sender, receiver) = mpsc::channel(buffer_size);

        let actor = ArActor::new(receiver, load_db_repo, 0);

        tokio::spawn(async move {
            actor.run().await;
        });

        Self { sender }
    }

    pub async fn new_parallel(
        buffer_size: usize,
        num_workers: usize,
        load_db_repo: Arc<dyn LoadDbConnection + Send + Sync + 'static>,
    ) -> Self {
        info!(
            target: "wvm::exex",
            "Creating {} parallel ArActors with shared queue of size {}",
            num_workers,
            buffer_size
        );

        let mut worker_channels = Vec::with_capacity(num_workers);

        for id in 0..num_workers {
            let (worker_sender, worker_receiver) = mpsc::channel(buffer_size);

            let worker = ArActor::new(worker_receiver, load_db_repo.clone(), id);

            tokio::spawn(async move {
                worker.run().await;
            });

            worker_channels.push(worker_sender);
        }

        let (sender, mut distributor_receiver) = mpsc::channel(buffer_size);
        let worker_channels = Arc::new(worker_channels);

        let dist_channels = worker_channels.clone();
        tokio::spawn(async move {
            let mut current_worker = 0;

            while let Some(msg) = distributor_receiver.recv().await {
                if let Err(e) = dist_channels[current_worker].send(msg).await {
                    error!(
                        target: "wvm::exex",
                        worker_id = current_worker,
                        error = %e,
                        "Failed to forward message to worker"
                    );
                }
                current_worker = (current_worker + 1) % dist_channels.len();
            }

            for (id, worker) in dist_channels.iter().enumerate() {
                if let Err(e) = worker.send(ArActorMessage::Shutdown).await {
                    error!(
                        target: "wvm::exex",
                        worker_id = id,
                        error = %e,
                        "Failed to send shutdown to worker"
                    );
                }
            }
        });

        Self { sender }
    }

    pub async fn process_block(
        &self,
        block: RecoveredBlock<reth_primitives::Block>,
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
