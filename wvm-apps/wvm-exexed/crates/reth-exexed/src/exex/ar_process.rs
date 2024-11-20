use crate::network_tag::get_network_tag;
use crate::new_etl_exex_biguery_client;
use crate::util::check_block_existence;
use arweave_upload::{ArweaveRequest, UploaderProvider};
use exex_wvm_bigquery::repository::StateRepository;
use exex_wvm_bigquery::BigQueryClient;
use exex_wvm_da::{DefaultWvmDataSettler, WvmDataSettler};
use reth::primitives::revm_primitives::alloy_primitives::private::serde::Serialize;
use reth::primitives::revm_primitives::alloy_primitives::BlockNumber;
use reth_primitives::SealedBlockWithSenders;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::mpsc::Sender;
use tokio::task::JoinHandle;
use tracing::{error, info};
use wvm_borsh::block::BorshSealedBlockWithSenders;
use wvm_static::SUPERVISOR_RT;

pub struct ArProcess {
    buffer_size: usize,
    pub sender: Sender<(SealedBlockWithSenders, String)>,
    thread: JoinHandle<()>,
}

impl ArProcess {
    pub fn new(buffer_size: usize) -> Self {
        let (sender, mut receiver) =
            tokio::sync::mpsc::channel::<(SealedBlockWithSenders, String)>(buffer_size);

        let thread = SUPERVISOR_RT.spawn(async move {
            let big_query_client = new_etl_exex_biguery_client().await;
            let big_query_client = Arc::new(big_query_client);
            let state_repo = Arc::new(StateRepository::new(big_query_client.clone()));

            while let Some(msg) = receiver.recv().await {
                let state_repo = state_repo.clone();
                let big_query_client = big_query_client.clone();
                let irys_provider = UploaderProvider::new(None);
                tokio::spawn(async move {
                    let sealed_block = msg.0;
                    let notification_type = msg.1.as_str();

                    let block_number = sealed_block.block.header.header().number;
                    let block_hash = sealed_block.block.hash().to_string();

                    let sealed_block_clone = sealed_block.clone();

                    let brotli_borsh = match Self::serialize_block(sealed_block_clone) {
                        Some(value) => value,
                        None => return,
                    };

                    let does_block_exist = check_block_existence(block_hash.as_str(), false).await;

                    if !does_block_exist {
                        let provider = irys_provider.clone();
                        let arweave_id = match Self::send_block_to_arweave(&provider, notification_type, block_number, &block_hash, brotli_borsh).await {
                            Some(ar_id) => ar_id,
                            None => return,
                        };

                        info!(target: "wvm::exex", "irys id: {}, for block: {}", arweave_id, block_number);

                        let _ = Self::bigquery_tags(big_query_client.clone(), &sealed_block);
                        let _ = Self::bigquery_task(state_repo.clone(), sealed_block, block_number, arweave_id);
                    }
                });
            }
        });

        Self { sender, buffer_size, thread }
    }

    fn bigquery_tags(client: Arc<BigQueryClient>, sealed_block: &SealedBlockWithSenders) {
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

        tokio::spawn(async move {
            let run_q = client.bq_query(query.clone()).await;
            if let Err(e) = run_q {
                error!(target: "wvm::exex", %e, "Failed to write to bigquery, block {}. Query: {}", block_number, query);
            } else {
                info!(target: "wvm::exex", "Tags at block {} updated successfully", block_number);
            }
        });
    }

    fn bigquery_task(
        state_repo: Arc<StateRepository>,
        sealed_block: SealedBlockWithSenders,
        block_number: BlockNumber,
        arweave_id: String,
    ) -> Result<JoinHandle<()>, ()> {
        Ok(tokio::spawn(async move {
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
        }))
    }

    async fn send_block_to_arweave(
        irys_provider: &UploaderProvider,
        notification_type: &str,
        block_number: BlockNumber,
        block_hash: &String,
        brotli_borsh: Vec<u8>,
    ) -> Option<String> {
        let res = ArweaveRequest::new()
            .set_tag("Content-Type", "application/octet-stream")
            .set_tag("WeaveVM:Encoding", "Borsh-Brotli")
            .set_tag("Block-Number", &block_hash)
            .set_tag("Block-Hash", &block_hash)
            .set_tag("Client-Version", reth_primitives::constants::RETH_CLIENT_VERSION)
            .set_tag("Network", get_network_tag().as_str())
            .set_tag("WeaveVM:Internal-Chain", notification_type)
            .set_data(brotli_borsh)
            .send_with_provider(&irys_provider)
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
