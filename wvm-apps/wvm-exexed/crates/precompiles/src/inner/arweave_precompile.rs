use irys::irys::IrysRequest;
use reth::{
    primitives::{
        hex,
        revm_primitives::{Precompile, PrecompileError, PrecompileOutput, PrecompileResult},
        Bytes,
    },
    revm::precompile::{u64_to_address, PrecompileWithAddress},
};
use reth_revm::{precompile::PrecompileErrors, primitives::B256};
use std::str::FromStr;

pub const PC_ADDRESS: u64 = 0x17;
pub const ARWEAVE_PC_BASE: u64 = 3_450;

pub const ARWEAVE_UPLOAD_PC: PrecompileWithAddress =
    PrecompileWithAddress(u64_to_address(PC_ADDRESS), Precompile::Standard(arweave_upload));

pub const SOLANA_SILLY_PRIVATE_KEY: &str =
    "kNykCXNxgePDjFbDWjPNvXQRa8U12Ywc19dFVaQ7tebUj3m7H4sF4KKdJwM7yxxb3rqxchdjezX9Szh8bLcQAjb";

fn arweave_upload(input: &Bytes, gas_limit: u64) -> PrecompileResult {
    let data_size = input.len();
    let gas_used: u64 = (10_000 + data_size * 3) as u64;

    if gas_used > gas_limit {
        return Err(PrecompileErrors::Error(PrecompileError::OutOfGas))
    }

    if input.is_empty() {
        return Err(PrecompileErrors::Error(PrecompileError::Other(
            "Data cannot be empty when uploading to arweave".to_string(),
        )));
    }

    /// We use 1012 as a measure to handle exceptions on Irys side.
    if data_size >= 100 * 1012 {
        return Err(PrecompileErrors::Error(PrecompileError::Other(
            "Data cannot exceed 101200 bytes".to_string(),
        )));
    }

    let res = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap().block_on(
        async {
            IrysRequest::new()
                .set_tag("Content-Type", "application/octet-stream")
                .set_tag("WeaveVM:Precompile", "true")
                .set_tag("WeaveVM:Precompile-Address", PC_ADDRESS.to_string().as_str())
                .set_data(input.0.to_vec())
                .send()
                .await
        },
    );

    let byte_resp = if let Ok(tx_id) = res { tx_id.into_bytes() } else { vec![] };

    let out = PrecompileOutput::new(gas_used, byte_resp.into());
    Ok(out)
}

#[cfg(test)]
mod irys_pc_tests {
    use crate::inner::arweave_precompile::{arweave_upload, SOLANA_SILLY_PRIVATE_KEY};
    use reth::primitives::{revm_primitives::PrecompileOutput, Bytes};
    use std::env;

    #[test]
    pub fn test_arweave_precompile() {
        let input = Bytes::from("Hello world".as_bytes());
        env::set_var("irys_pk", SOLANA_SILLY_PRIVATE_KEY);
        let PrecompileOutput { gas_used, bytes } = arweave_upload(&input, 100_000).unwrap();
        let tx_id = unsafe { String::from_utf8_unchecked(bytes.to_vec()) };
        println!("{}", tx_id)
    }
}
