use alloy_consensus::Header;
use alloy_primitives::{Address, BlockNumber, Bloom, Bytes, B256, B64, U256};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct V1WvmHeader {
    pub parent_hash: B256,
    pub ommers_hash: B256,
    pub beneficiary: Address,
    pub state_root: B256,
    pub transactions_root: B256,
    pub receipts_root: B256,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub withdrawals_root: Option<B256>,
    pub logs_bloom: Bloom,
    pub difficulty: U256,
    #[serde(with = "alloy_serde::quantity")]
    pub number: BlockNumber,
    #[serde(with = "alloy_serde::quantity")]
    pub gas_limit: u64,
    #[serde(with = "alloy_serde::quantity")]
    pub gas_used: u64,
    #[serde(with = "alloy_serde::quantity")]
    pub timestamp: u64,
    pub mix_hash: B256,
    pub nonce: B64,
    #[serde(default, with = "alloy_serde::quantity::opt", skip_serializing_if = "Option::is_none")]
    pub base_fee_per_gas: Option<u64>,
    #[serde(default, with = "alloy_serde::quantity::opt", skip_serializing_if = "Option::is_none")]
    pub blob_gas_used: Option<u64>,
    #[serde(default, with = "alloy_serde::quantity::opt", skip_serializing_if = "Option::is_none")]
    pub excess_blob_gas: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_beacon_block_root: Option<B256>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub requests_hash: Option<B256>,
    pub extra_data: Bytes,
}

impl Into<Header> for V1WvmHeader {
    fn into(self) -> Header {
        Header {
            parent_hash: self.parent_hash,
            ommers_hash: self.ommers_hash,
            beneficiary: self.beneficiary,
            state_root: self.state_root,
            transactions_root: self.transactions_root,
            receipts_root: self.receipts_root,
            withdrawals_root: self.withdrawals_root,
            logs_bloom: self.logs_bloom,
            difficulty: self.difficulty,
            number: self.number,
            gas_limit: self.gas_limit,
            gas_used: self.gas_used,
            timestamp: self.timestamp,
            mix_hash: self.mix_hash,
            nonce: self.nonce,
            base_fee_per_gas: self.base_fee_per_gas,
            blob_gas_used: self.blob_gas_used,
            excess_blob_gas: self.excess_blob_gas,
            parent_beacon_block_root: self.parent_beacon_block_root,
            requests_hash: self.requests_hash,
            extra_data: self.extra_data,
        }
    }
}

impl From<Header> for V1WvmHeader {
    fn from(value: Header) -> Self {
        V1WvmHeader {
            parent_hash: value.parent_hash,
            ommers_hash: value.ommers_hash,
            beneficiary: value.beneficiary,
            state_root: value.state_root,
            transactions_root: value.transactions_root,
            receipts_root: value.receipts_root,
            withdrawals_root: value.withdrawals_root,
            logs_bloom: value.logs_bloom,
            difficulty: value.difficulty,
            number: value.number,
            gas_limit: value.gas_limit,
            gas_used: value.gas_used,
            timestamp: value.timestamp,
            mix_hash: value.mix_hash,
            nonce: value.nonce,
            base_fee_per_gas: value.base_fee_per_gas,
            blob_gas_used: value.blob_gas_used,
            excess_blob_gas: value.excess_blob_gas,
            parent_beacon_block_root: value.parent_beacon_block_root,
            requests_hash: value.requests_hash,
            extra_data: value.extra_data,
        }
    }
}
