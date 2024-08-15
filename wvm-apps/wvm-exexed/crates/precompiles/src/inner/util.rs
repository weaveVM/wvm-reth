use revm_primitives::{PrecompileError, PrecompileErrors, PrecompileOutput};

pub const DEFAULT_ARWEAVE_TX_ENDPOINT: &str = "https://arweave.net/";

pub fn clean_gateway_url(gateway: &str) -> String {
    let clean_gateway =
        if gateway.ends_with('/') { &gateway[..gateway.len() - 1] } else { gateway };

    clean_gateway.to_string()
}

pub async fn download_tx(
    gas_used: u64,
    clean_gateway: String,
    tx_id: String,
) -> Result<PrecompileOutput, PrecompileErrors> {
    let download_tx = reqwest::get(format!("{}/{}", clean_gateway, tx_id.as_str())).await;
    match download_tx {
        Ok(tx) => Ok(PrecompileOutput::new(gas_used, tx.bytes().await.unwrap().into())),
        Err(_) => Err(PrecompileErrors::Error(PrecompileError::Other(
            "Arweave Transaction was not found".to_string(),
        ))),
    }
}
