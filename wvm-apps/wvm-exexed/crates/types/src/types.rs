use alloy_primitives;
use reth::providers::ExecutionOutcome;
use serde::{Serialize, Deserialize};
use reth::primitives::SealedBlockWithSenders;

#[derive(Debug, Serialize, Deserialize)]
pub struct ExecutionTipState {
    pub block_number: alloy_primitives::BlockNumber,
    pub sealed_block_with_senders: SealedBlockWithSenders,
}

