pub mod drivers;

use async_trait::async_trait;
use planetscale_driver::Database;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fmt::Debug;

#[derive(Serialize, Deserialize, Database)]
pub struct RawState {
    pub block_number: i128,
    pub sealed_block_with_senders: String,
    pub arweave_id: String,
    pub timestamp: i128,
    pub block_hash: String,
}

#[async_trait]
pub trait LoadDbConnection: Send + Sync {
    async fn query_raw_state(&self, block_id: String) -> Option<RawState>;
    async fn query_state(&self, block_id: String) -> Option<String>;
    async fn save_hashes(&self, hashes: &[String], block_number: u64) -> eyre::Result<()>;

    async fn save_block<T>(
        &self,
        block: &T,
        block_number: u64,
        arweave_id: String,
        block_hash: String,
    ) -> eyre::Result<()>
    where
        T: ?Sized + Serialize + Send + Sync;

    async fn save_tx_tag(
        &self,
        tx_hash: String,
        tags: Vec<(String, String)>,
        created_at: u128,
    ) -> eyre::Result<()>;

    async fn query_transaction_by_tags(&self, tag: (String, String)) -> Option<String>;
}
