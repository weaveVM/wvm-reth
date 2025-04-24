use alloy_primitives::Bytes;
use arweave_upload::ArweaveRequest;
use eyre::eyre;
use rbrotli::to_brotli;
use reth::revm::precompile::{
    PrecompileError, PrecompileFn, PrecompileResult, PrecompileOutput,
};
use wvm_static::internal_block;

pub const PC_ADDRESS: u64 = 0x17;
pub const ARWEAVE_PC_BASE: u64 = 3_450;

pub const ARWEAVE_UPLOAD_PC: PrecompileFn =arweave_upload as PrecompileFn;

pub const SOLANA_SILLY_PRIVATE_KEY: &str =
    "kNykCXNxgePDjFbDWjPNvXQRa8U12Ywc19dFVaQ7tebUj3m7H4sF4KKdJwM7yxxb3rqxchdjezX9Szh8bLcQAjb";

fn arweave_upload(input: &Bytes, gas_limit: u64) -> PrecompileResult {
    let input = to_brotli(input.to_vec());
    let data_size = input.len();
    let gas_used: u64 = (10_000 + data_size * 3) as u64;

    if gas_used > gas_limit {
        return Err(PrecompileError::OutOfGas);
    }

    if input.is_empty() {
        return Err(PrecompileError::Other(
            "Data cannot be empty when uploading to arweave".to_string(),
        ));
    }

    /// We use 1012 as a measure to handle exceptions on Irys side.
    if data_size >= 100 * 1012 {
        return Err(PrecompileError::Other(
            "Data cannot exceed 101200 bytes".to_string(),
        ));
    }

    let res = internal_block(async {
        ArweaveRequest::new()
            .set_private_key(SOLANA_SILLY_PRIVATE_KEY.to_string())
            .set_tag("Content-Type", "application/octet-stream")
            // here it was safe to switch all "weaveVM:*" to "LN:*" because the precompiles
            // tx data on Arweave isn't read back into the EVM chain
            .set_tag("LN:Precompile", "true")
            .set_tag("LN:Encoding", "Brotli")
            .set_tag("LN:Precompile-Address", PC_ADDRESS.to_string().as_str())
            .set_data(input)
            .send()
            .await
    })
    .map_err(|_| {
       PrecompileError::Other(
            eyre!("Failed to build runtime to call arweave").to_string(),
        )
    })?;

    let byte_resp = if let Ok(tx_id) = res { tx_id.into_bytes() } else { vec![] };

    let out = PrecompileOutput::new(gas_used, byte_resp.into());
    Ok(out)
}

#[cfg(test)]
mod arupload_pc_tests {
    use crate::inner::arweave_precompile::{arweave_upload, SOLANA_SILLY_PRIVATE_KEY};
    use alloy_primitives::Bytes;
    use reth::revm::precompile::{
        PrecompileOutput,
    };
    use std::env;

    #[test]
    pub fn test_arweave_precompile() {
        let input = Bytes::from("Hello world".as_bytes());
        unsafe {
            env::set_var("irys_pk", SOLANA_SILLY_PRIVATE_KEY);
        }
        let PrecompileOutput { gas_used, bytes } = arweave_upload(&input, 100_000).unwrap();
        let tx_id = unsafe { String::from_utf8_unchecked(bytes.to_vec()) };
        println!("{}", tx_id)
    }
}
