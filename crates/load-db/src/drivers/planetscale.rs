use crate::{LoadDbConnection, RawState};
use async_trait::async_trait;
use eyre::eyre;
use planetscale_driver::{query, Database, PSConnection};
use reth_primitives::SealedBlockWithSenders;
use serde::{Deserialize, Serialize};
use std::{
    fmt::Debug,
    time::{SystemTime, UNIX_EPOCH},
};

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
        let table_name = "state";
        let select_clause =
            format!("SELECT block_number, arweave_id, timestamp, block_hash FROM {}", table_name);

        let get_query = if let Ok(num) = block_id.parse::<i128>() {
            format!("{} WHERE block_number = {}", select_clause, num)
        } else {
            format!(
                "{} WHERE arweave_id = '{}' OR block_hash = '{}'",
                select_clause, block_id, block_id
            )
        };

        let conn = self.get_conn();
        let fetch = query(get_query.as_str()).fetch_one::<RawState>(&conn).await;

        fetch.ok()
    }

    async fn query_state(&self, block_id: String) -> Option<String> {
        self.query_raw_state(block_id).await.map(|e| e.sealed_block_with_senders)
    }

    async fn save_hashes(&self, hashes: &[String], block_number: u64) -> eyre::Result<()> {
        let timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?;

        let timestamp = timestamp.as_millis();

        let in_clause = hashes
            .iter()
            .map(|hash| format!("'{}'", hash.replace('\'', "''")))
            .collect::<Vec<String>>()
            .join(", ");

        let insert_query = format!(
            "INSERT INTO {} (hash, tags, block_id, created_at)
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

    async fn save_block(
        &self,
        block: &SealedBlockWithSenders,
        block_number: u64,
        arweave_id: String,
        block_hash: String,
    ) -> eyre::Result<()> {
        let insert_query = format!(
            "INSERT INTO state (
                block_number,
                arweave_id,
                block_hash
            ) VALUES (
                {},
                '{}',
                '{}'
            );",
            block_number,
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

    async fn query_transaction_by_tags(&self, tag: (String, String)) -> Option<String> {
        #[derive(Serialize, Deserialize, Database)]
        struct QueryTxByTagResult {
            pub hash: String,
        }

        let query_str = format!(
            "SELECT t.hash hash
FROM {} t,
JSON_TABLE(
  t.tags, '$[*]'
  COLUMNS (
    tag_key VARCHAR(255) PATH '$[0]',
    tag_value VARCHAR(255) PATH '$[1]'
  )
) AS tags_flat
WHERE tags_flat.tag_key = '{}'
  AND tags_flat.tag_value = '{}'
LIMIT 1;",
            "confirmed_tags", tag.0, tag.1
        );

        let conn = self.get_conn();

        let fetch_tag = query(query_str.as_str()).fetch_one::<QueryTxByTagResult>(&conn).await;

        fetch_tag.ok().map(|e| e.hash)
    }
}
