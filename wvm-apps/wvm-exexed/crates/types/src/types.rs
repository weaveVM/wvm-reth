use alloy_primitives;

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct ExecutionTipState {
    pub block_number: alloy_primitives::BlockNumber,
    pub arweave_id: String,
    pub sealed_block_with_senders_serialized: String,
}
