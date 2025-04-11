use crate::{LoadDbConnection, RawState};
use async_trait::async_trait;
use eyre::eyre;
use planetscale_driver::PSConnection;
use serde::Serialize;
use std::fmt::Debug;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Default, Clone)]
pub struct PlanetScaleDriver {
    pub host: String,
    pub username: String,
    pub password: String,
}

impl PlanetScaleDriver {
    pub fn new(host: String, username: String, password: String) -> Self {
        Self { host, username, password }
    }

    pub fn get_conn(&self) -> PSConnection {
        PSConnection::new(&self.host, &self.username, &self.password)
    }
}

#[async_trait]
impl LoadDbConnection for PlanetScaleDriver {
    async fn query_raw_state(&self, block_id: String) -> Option<RawState> {
        todo!()
    }

    async fn query_state(&self, block_id: String) -> Option<String> {
        todo!()
    }

    async fn save_hashes(&self, hashes: &[String], block_number: u64) -> eyre::Result<()> {
        let timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?;

        let timestamp = timestamp.as_millis();

        let in_clause = hashes
            .into_iter()
            .map(|hash| format!("'{}'", hash.replace('\'', "''")))
            .collect::<Vec<String>>()
            .join(", ");

        let insert_query = format!(
            "INSERT INTO {} (tx_hash, tags, block_id, `timestamp`)
     SELECT t.hash, t.tags, {}, {}
     FROM {} t
     WHERE t.hash IN ({}) AND t.created_at <= {}",
            "confirmed_tags", // e.g. "tags"
            block_number,     // e.g. 123
            timestamp,        // e.g. 1712800000
            "tags",           // e.g. "temp_tags"
            in_clause,        // e.g. "'abc', 'def', 'ghi'"
            timestamp         // e.g. 1712799999
        );

        let conn = self.get_conn();
        conn.execute(&insert_query).await.map_err(|e| eyre!(e.to_string()))?;

        Ok(())
    }

    async fn save_block<T>(
        &self,
        block: &T,
        block_number: u64,
        arweave_id: String,
        block_hash: String,
    ) -> eyre::Result<()>
    where
        T: ?Sized + Serialize + Send + Sync,
    {
        let sealed_json = serde_json::to_string(block)?;
        let escaped_json = sealed_json.replace('\'', "''");

        let insert_query = format!(
            "INSERT INTO state (
                block_number,
                sealed_block_with_senders,
                arweave_id,
                block_hash
            ) VALUES (
                {},
                '{}',
                '{}',
                '{}'
            );",
            block_number,
            escaped_json,
            arweave_id.replace('\'', "''"),
            block_hash.replace('\'', "''")
        );

        let conn = self.get_conn();
        conn.execute(&insert_query).await.map_err(|e| eyre!(e.to_string()))?;

        Ok(())
    }

    async fn save_tx_tag(
        &self,
        tx_hash: String,
        tags: Vec<(String, String)>,
        created_at: u128,
    ) -> eyre::Result<()> {
        let sealed_json = serde_json::to_string(&tags)?;
        let escaped_json_tags = sealed_json.replace('\'', "''");

        let insert_query = format!(
            "INSERT INTO tags (
                hash,
                tags,
                created_at
            ) VALUES (
                '{}',
                '{}',
                {}
            );",
            tx_hash, escaped_json_tags, created_at
        );

        let conn = self.get_conn();
        conn.execute(&insert_query).await.map_err(|e| eyre!(e.to_string()))?;

        Ok(())
    }
}
