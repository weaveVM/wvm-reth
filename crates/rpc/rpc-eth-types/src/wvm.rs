use alloy_primitives::Bytes;
use alloy_rpc_types_eth::TransactionRequest;
use jsonrpsee_core::Serialize;
use serde::Deserialize;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WvmTransactionRequest {
    pub tx: Bytes,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<(String, String)>>,
}
