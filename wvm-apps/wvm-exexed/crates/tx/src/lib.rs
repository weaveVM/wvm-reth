pub mod wvm;

use alloy_eips::{eip2930::AccessList, eip7702::SignedAuthorization};
use alloy_primitives::{Address, Bytes, ChainId, TxKind, B256, U256};
use reth_primitives::Transaction;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TxLegacy {
    #[serde(default, with = "alloy_serde::quantity::opt", skip_serializing_if = "Option::is_none")]
    #[serde(rename = "chainId", alias = "chain_id")]
    pub chain_id: Option<ChainId>,
    #[serde(with = "alloy_serde::quantity")]
    pub nonce: u64,
    #[serde(rename = "gasPrice", alias = "gas_price")]
    #[serde(with = "alloy_serde::quantity")]
    pub gas_price: u128,
    #[serde(rename = "gasLimit", alias = "gas", alias = "gas_limit")]
    #[serde(with = "alloy_serde::quantity")]
    pub gas_limit: u64,
    #[serde(default, skip_serializing_if = "TxKind::is_create")]
    pub to: TxKind,
    pub value: U256,
    pub input: Bytes,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TxEip7702 {
    #[serde(with = "alloy_serde::quantity")]
    #[serde(rename = "chainId", alias = "chain_id")]
    pub chain_id: ChainId,
    #[serde(with = "alloy_serde::quantity")]
    pub nonce: u64,
    #[serde(rename = "gasLimit", alias = "gas_limit")]
    #[serde(with = "alloy_serde::quantity")]
    pub gas_limit: u64,
    #[serde(rename = "maxFeePerGas", alias = "max_fee_per_gas")]
    #[serde(with = "alloy_serde::quantity")]
    pub max_fee_per_gas: u128,
    #[serde(rename = "maxPriorityFeePerGas", alias = "max_priority_fee_per_gas")]
    #[serde(with = "alloy_serde::quantity")]
    pub max_priority_fee_per_gas: u128,

    pub to: Address,
    pub value: U256,
    #[serde(rename = "accessList", alias = "access_list")]
    pub access_list: AccessList,
    #[serde(rename = "authorizationList", alias = "authorization_list")]
    pub authorization_list: Vec<SignedAuthorization>,
    pub input: Bytes,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TxEip4844 {
    #[serde(rename = "chainId", alias = "chain_id")]
    #[serde(with = "alloy_serde::quantity")]
    pub chain_id: ChainId,
    #[serde(with = "alloy_serde::quantity")]
    pub nonce: u64,

    #[serde(rename = "gasLimit", alias = "gas_limit")]
    #[serde(with = "alloy_serde::quantity")]
    pub gas_limit: u64,

    #[serde(rename = "maxFeePerGas", alias = "max_fee_per_gas")]
    #[serde(with = "alloy_serde::quantity")]
    pub max_fee_per_gas: u128,

    #[serde(rename = "maxPriorityFeePerGas", alias = "max_priority_fee_per_gas")]
    #[serde(with = "alloy_serde::quantity")]
    pub max_priority_fee_per_gas: u128,

    pub to: Address,

    pub value: U256,

    pub access_list: AccessList,

    pub blob_versioned_hashes: Vec<B256>,

    #[serde(rename = "maxFeePerBlobGas", alias = "max_fee_per_blob_gas")]
    #[serde(with = "alloy_serde::quantity")]
    pub max_fee_per_blob_gas: u128,
    pub input: Bytes,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TxEip2930 {
    #[serde(with = "alloy_serde::quantity")]
    #[serde(rename = "chainId", alias = "chain_id")]
    pub chain_id: ChainId,
    #[serde(with = "alloy_serde::quantity")]
    pub nonce: u64,
    #[serde(rename = "gasPrice", alias = "gas_price")]
    #[serde(with = "alloy_serde::quantity")]
    pub gas_price: u128,
    #[serde(rename = "gasLimit", alias = "gas_limit")]
    #[serde(with = "alloy_serde::quantity")]
    pub gas_limit: u64,
    #[serde(default, skip_serializing_if = "TxKind::is_create")]
    pub to: TxKind,
    pub value: U256,
    #[serde(rename = "accessList", alias = "access_list")]
    pub access_list: AccessList,
    pub input: Bytes,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TxEip1559 {
    #[serde(with = "alloy_serde::quantity")]
    #[serde(rename = "chainId", alias = "chain_id")]
    pub chain_id: ChainId,
    #[serde(with = "alloy_serde::quantity")]
    pub nonce: u64,
    #[serde(rename = "gasLimit", alias = "gas_limit")]
    #[serde(with = "alloy_serde::quantity")]
    pub gas_limit: u64,
    #[serde(rename = "maxFeePerGas", alias = "max_fee_per_gas")]
    #[serde(with = "alloy_serde::quantity")]
    pub max_fee_per_gas: u128,
    #[serde(rename = "maxPriorityFeePerGas", alias = "max_priority_fee_per_gas")]
    #[serde(with = "alloy_serde::quantity")]
    pub max_priority_fee_per_gas: u128,
    #[serde(default, skip_serializing_if = "TxKind::is_create")]
    pub to: TxKind,
    pub value: U256,
    #[serde(rename = "accessList", alias = "access_list")]
    pub access_list: AccessList,
    pub input: Bytes,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TxDeposit {
    #[serde(rename = "sourceHash", alias = "source_hash")]
    pub source_hash: B256,
    pub from: Address,
    #[serde(default, skip_serializing_if = "TxKind::is_create")]
    pub to: TxKind,
    #[serde(default, with = "alloy_serde::quantity::opt")]
    pub mint: Option<u128>,
    pub value: U256,
    #[serde(rename = "gasLimit", alias = "gas_limit")]
    #[serde(with = "alloy_serde::quantity")]
    pub gas_limit: u64,
    #[serde(
        with = "alloy_serde::quantity",
        rename = "isSystemTx",
        alias = "is_system_transaction"
    )]
    pub is_system_transaction: bool,
    pub input: Bytes,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum WvmTransaction {
    Legacy(TxLegacy),
    Eip2930(TxEip2930),
    Eip1559(TxEip1559),
    Eip4844(TxEip4844),
    Eip7702(TxEip7702),
    #[cfg(feature = "optimism")]
    Deposit(TxDeposit),
}

impl Into<Transaction> for WvmTransaction {
    fn into(self) -> Transaction {
        match self {
            WvmTransaction::Legacy(data) => Transaction::Legacy(alloy_consensus::TxLegacy {
                chain_id: data.chain_id,
                nonce: data.nonce,
                gas_price: data.gas_price,
                gas_limit: data.gas_limit,
                to: data.to,
                value: data.value,
                input: data.input,
            }),
            WvmTransaction::Eip2930(data) => Transaction::Eip2930(alloy_consensus::TxEip2930 {
                chain_id: data.chain_id,
                nonce: data.nonce,
                gas_price: data.gas_price,
                gas_limit: data.gas_limit,
                to: data.to,
                value: data.value,
                access_list: data.access_list,
                input: data.input,
            }),
            WvmTransaction::Eip1559(data) => Transaction::Eip1559(alloy_consensus::TxEip1559 {
                chain_id: data.chain_id,
                nonce: data.nonce,
                gas_limit: data.gas_limit,
                max_fee_per_gas: data.max_fee_per_gas,
                max_priority_fee_per_gas: data.max_priority_fee_per_gas,
                to: data.to,
                value: data.value,
                access_list: data.access_list,
                input: data.input,
            }),
            WvmTransaction::Eip4844(data) => Transaction::Eip4844(alloy_consensus::TxEip4844 {
                chain_id: data.chain_id,
                nonce: data.nonce,
                gas_limit: data.gas_limit,
                max_fee_per_gas: data.max_fee_per_gas,
                max_priority_fee_per_gas: data.max_priority_fee_per_gas,
                to: data.to,
                value: data.value,
                access_list: data.access_list,
                blob_versioned_hashes: data.blob_versioned_hashes,
                max_fee_per_blob_gas: data.max_fee_per_blob_gas,
                input: data.input,
            }),
            WvmTransaction::Eip7702(data) => Transaction::Eip7702(alloy_consensus::TxEip7702 {
                chain_id: data.chain_id,
                nonce: data.nonce,
                gas_limit: data.gas_limit,
                max_fee_per_gas: data.max_fee_per_gas,
                max_priority_fee_per_gas: data.max_priority_fee_per_gas,
                to: data.to,
                value: data.value,
                access_list: data.access_list,
                authorization_list: data.authorization_list,
                input: data.input,
            }),
            #[cfg(feature = "optimism")]
            WvmTransaction::Deposit(data) => Transaction::Deposit(op_alloy_consensus::TxDeposit {
                source_hash: data.source_hash,
                gas_limit: data.gas_limit,
                to: data.to,
                mint: data.mint,
                value: data.value,
                input: data.input,
                from: data.from,
                is_system_transaction: data.is_system_transaction,
            }),
        }
    }
}

impl From<Transaction> for WvmTransaction {
    fn from(value: Transaction) -> Self {
        match value {
            Transaction::Legacy(data) => WvmTransaction::Legacy(TxLegacy {
                chain_id: data.chain_id,
                nonce: data.nonce,
                gas_price: data.gas_price,
                gas_limit: data.gas_limit,
                to: data.to,
                value: data.value,
                input: data.input,
            }),
            Transaction::Eip2930(data) => WvmTransaction::Eip2930(TxEip2930 {
                chain_id: data.chain_id,
                nonce: data.nonce,
                gas_price: data.gas_price,
                gas_limit: data.gas_limit,
                to: data.to,
                value: data.value,
                access_list: data.access_list,
                input: data.input,
            }),
            Transaction::Eip1559(data) => WvmTransaction::Eip1559(TxEip1559 {
                chain_id: data.chain_id,
                nonce: data.nonce,
                gas_limit: data.gas_limit,
                max_fee_per_gas: data.max_fee_per_gas,
                max_priority_fee_per_gas: data.max_priority_fee_per_gas,
                to: data.to,
                value: data.value,
                access_list: data.access_list,
                input: data.input,
            }),
            Transaction::Eip4844(data) => WvmTransaction::Eip4844(TxEip4844 {
                chain_id: data.chain_id,
                nonce: data.nonce,
                gas_limit: data.gas_limit,
                max_fee_per_gas: data.max_fee_per_gas,
                max_priority_fee_per_gas: data.max_priority_fee_per_gas,
                to: data.to,
                value: data.value,
                access_list: data.access_list,
                blob_versioned_hashes: data.blob_versioned_hashes,
                max_fee_per_blob_gas: data.max_fee_per_blob_gas,
                input: data.input,
            }),
            Transaction::Eip7702(data) => WvmTransaction::Eip7702(TxEip7702 {
                chain_id: data.chain_id,
                nonce: data.nonce,
                gas_limit: data.gas_limit,
                max_fee_per_gas: data.max_fee_per_gas,
                max_priority_fee_per_gas: data.max_priority_fee_per_gas,
                to: data.to,
                value: data.value,
                access_list: data.access_list,
                authorization_list: data.authorization_list,
                input: data.input,
            }),
            #[cfg(feature = "optimism")]
            Transaction::Deposit(data) => WvmTransaction::Deposit(TxDeposit {
                source_hash: data.source_hash,
                gas_limit: data.gas_limit,
                to: data.to,
                mint: data.mint,
                value: data.value,
                input: data.input,
                from: data.from,
                is_system_transaction: data.is_system_transaction,
            }),
        }
    }
}
