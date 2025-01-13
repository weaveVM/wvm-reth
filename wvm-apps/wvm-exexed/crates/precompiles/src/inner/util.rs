use alloy_primitives::Bytes;
use revm_primitives::{PrecompileError, PrecompileErrors, PrecompileOutput};
use std::io::Read;
use wvm_static::internal_block;
use crate::inner::REQ_TIMEOUT;

pub const DEFAULT_ARWEAVE_TX_ENDPOINT: &str = "https://arweave.net/";

pub fn clean_gateway_url(gateway: &str) -> String {
    let clean_gateway =
        if gateway.ends_with('/') { &gateway[..gateway.len() - 1] } else { gateway };

    clean_gateway.to_string()
}

pub fn download_tx(
    gas_used: u64,
    clean_gateway: String,
    tx_id: String,
) -> Result<PrecompileOutput, PrecompileErrors> {
    // LEGACY
    // internal_block(async {
    //     let download_tx = reqwest::get(format!("{}/{}", clean_gateway, tx_id.as_str())).await;
    //     match download_tx {
    //         Ok(tx) => Ok(PrecompileOutput::new(gas_used, tx.bytes().await.unwrap().into())),
    //         Err(_) => Err(PrecompileErrors::Error(PrecompileError::Other(
    //             "Arweave Transaction was not found".to_string(),
    //         ))),
    //     }
    // }).unwrap()
    let download_tx = ureq::get(format!("{}/{}", clean_gateway, tx_id.as_str()).as_str()).timeout((&*REQ_TIMEOUT).clone()).call();
    match download_tx {
        Ok(tx) => Ok(PrecompileOutput::new(gas_used, {
            let mut reader = tx.into_reader();
            let mut buffer = vec![];
            let _ = reader.read_to_end(&mut buffer).map_err(|_| {
                PrecompileError::Other("Arweave Transaction was not found".to_string())
            })?;

            Bytes::from(buffer)
        })),
        Err(_) => Err(PrecompileErrors::Error(PrecompileError::Other(
            "Arweave Transaction was not found".to_string(),
        ))),
    }
}
