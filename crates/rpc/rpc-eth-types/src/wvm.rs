use alloy_primitives::Bytes;
use jsonrpsee_core::Serialize;
use serde::Deserialize;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WvmTransactionRequest {
    pub tx: Bytes,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<(String, String)>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetWvmTransactionByTagRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tag: Option<(String, String)>,
}
