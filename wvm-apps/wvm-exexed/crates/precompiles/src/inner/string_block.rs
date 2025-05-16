use reth::primitives::SealedBlockWithSenders;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use wvm_borsh::block::BorshSealedBlockWithSenders;
use wvm_tx::wvm::WvmSealedBlockWithSenders;

#[derive(Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Block {
    pub base_fee_per_gas: Option<String>,         // "baseFeePerGas"
    pub blob_gas_used: Option<String>,            // "blobGasUsed"
    pub difficulty: Option<String>,               // "difficulty"
    pub excess_blob_gas: Option<String>,          // "excessBlobGas"
    pub extra_data: Option<String>,               // "extraData"
    pub gas_limit: Option<String>,                // "gasLimit"
    pub gas_used: Option<String>,                 // "gasUsed"
    pub hash: Option<String>,                     // "hash"
    pub logs_bloom: Option<String>,               // "logsBloom"
    pub miner: Option<String>,                    // "miner"
    pub mix_hash: Option<String>,                 // "mixHash"
    pub nonce: Option<String>,                    // "nonce"
    pub number: Option<String>,                   // "number"
    pub parent_beacon_block_root: Option<String>, // "parentBeaconBlockRoot"
    pub parent_hash: Option<String>,              // "parentHash"
    pub receipts_root: Option<String>,            // "receiptsRoot"
    pub seal_fields: Vec<String>,                 // "sealFields" as an array of strings
    pub sha3_uncles: Option<String>,              // "sha3Uncles"
    pub size: Option<String>,                     // "size"
    pub state_root: Option<String>,               // "stateRoot"
    pub timestamp: Option<String>,                // "timestamp"
    pub total_difficulty: Option<String>,         // "totalDifficulty"
    pub transactions: Vec<String>,                // "transactions" as an array of strings
}

pub fn from_sealed_block_senders(sealed_block: SealedBlockWithSenders) -> Block {
    Block {
        base_fee_per_gas: sealed_block.base_fee_per_gas.map(|i| i.to_string()),
        blob_gas_used: sealed_block.blob_gas_used.map(|i| i.to_string()),
        difficulty: Some(sealed_block.difficulty.to_string()),
        excess_blob_gas: sealed_block.excess_blob_gas.map(|i| i.to_string()),
        extra_data: Some(sealed_block.extra_data.to_string()),
        gas_limit: Some(sealed_block.gas_limit.to_string()),
        gas_used: Some(sealed_block.gas_used.to_string()),
        hash: Some(sealed_block.hash().to_string()),
        logs_bloom: Some(sealed_block.logs_bloom.to_string()),
        miner: None,
        mix_hash: Some(sealed_block.mix_hash.to_string()),
        nonce: Some(sealed_block.nonce.to_string()),
        number: Some(sealed_block.number.to_string()),
        parent_beacon_block_root: sealed_block.parent_beacon_block_root.map(|i| i.to_string()),
        parent_hash: Some(sealed_block.parent_hash.to_string()),
        receipts_root: Some(sealed_block.receipts_root.to_string()),
        seal_fields: vec![],
        sha3_uncles: None,
        size: Some(sealed_block.size().to_string()),
        state_root: Some(sealed_block.state_root.to_string()),
        timestamp: Some(sealed_block.timestamp.to_string()),
        total_difficulty: None,
        transactions: sealed_block
            .into_body()
            .transactions()
            .map(|i| i.hash().to_string())
            .collect(),
    }
}

fn json_value_to_string(value: &Value) -> String {
    match value {
        Value::String(s) => s.clone(), // For strings, return the raw string
        _ => value.to_string(),        // For other types, use the default serialization
    }
}

pub fn from_sealed_block_senders_value(sealed_block: Value) -> Block {
    Block {
        base_fee_per_gas: sealed_block["block"]["header"]["header"]
            .get("base_fee_per_gas")
            .map(|i| json_value_to_string(i)),
        blob_gas_used: sealed_block["block"]["header"]["header"]
            .get("blob_gas_used")
            .map(|i| json_value_to_string(i)),
        difficulty: Some(json_value_to_string(
            &sealed_block["block"]["header"]["header"]["difficulty"],
        )),
        excess_blob_gas: sealed_block["block"]["header"]["header"]
            .get("excess_blob_gas")
            .map(|i| json_value_to_string(i)),
        extra_data: Some(json_value_to_string(
            &sealed_block["block"]["header"]["header"]["extra_data"],
        )),
        gas_limit: Some(json_value_to_string(
            &sealed_block["block"]["header"]["header"]["gas_limit"],
        )),
        gas_used: Some(json_value_to_string(
            &sealed_block["block"]["header"]["header"]["gas_used"],
        )),
        hash: Some(json_value_to_string(&sealed_block["block"]["header"]["hash"])),
        logs_bloom: Some(json_value_to_string(
            &sealed_block["block"]["header"]["header"]["logs_bloom"],
        )),
        miner: None,
        mix_hash: Some(json_value_to_string(
            &sealed_block["block"]["header"]["header"]["mix_hash"],
        )),
        nonce: Some(json_value_to_string(&sealed_block["block"]["header"]["header"]["nonce"])),
        number: Some(json_value_to_string(&sealed_block["block"]["header"]["header"]["number"])),
        parent_beacon_block_root: sealed_block["block"]["header"]["header"]
            .get("parent_beacon_block_root")
            .map(|i| json_value_to_string(i)),
        parent_hash: Some(json_value_to_string(
            &sealed_block["block"]["header"]["header"]["parent_hash"],
        )),
        receipts_root: Some(json_value_to_string(
            &sealed_block["block"]["header"]["header"]["receipts_root"],
        )),
        seal_fields: vec![],
        sha3_uncles: None,
        size: None,
        state_root: Some(json_value_to_string(
            &sealed_block["block"]["header"]["header"]["state_root"],
        )),
        timestamp: Some(json_value_to_string(
            &sealed_block["block"]["header"]["header"]["timestamp"],
        )),
        total_difficulty: None,
        transactions: vec![],
    }
}

impl From<BorshSealedBlockWithSenders> for Block {
    fn from(value: BorshSealedBlockWithSenders) -> Self {
        let sealed_block = value.0;
        match sealed_block {
            WvmSealedBlockWithSenders::V1(data) => from_sealed_block_senders(data.into()),
        }
    }
}

impl From<String> for Block {
    fn from(value: String) -> Self {
        let block = serde_json::from_str::<SealedBlockWithSenders>(&value).unwrap();
        from_sealed_block_senders(block)
    }
}
