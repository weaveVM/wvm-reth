use irys::irys::IrysRequest;
use reth::primitives::revm_primitives::{
    Precompile, PrecompileError, PrecompileOutput, PrecompileResult,
};
use reth::primitives::Bytes;
use reth::revm::precompile::{u64_to_address, PrecompileWithAddress};
use reth_revm::precompile::PrecompileErrors;
use reth_revm::primitives::B256;

pub const PC_ADDRESS: u64 = 0x99;
pub const ARWEAVE_PC_BASE: u64 = 3_450;

pub const ARWEAVE_UPLOAD_PC: PrecompileWithAddress =
    PrecompileWithAddress(u64_to_address(PC_ADDRESS), Precompile::Standard(arweave_upload));

fn arweave_upload(input: &Bytes, gas_limit: u64) -> PrecompileResult {
    let data_size = input.len();
    let gas_used: u64 = (10_000 + data_size * 3) as u64;

    if gas_used > gas_limit {
        return Err(PrecompileErrors::Error(PrecompileError::OutOfGas));
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
                .set_data(input.0.to_vec())
                .send()
                .await
        },
    );

    let byte_resp =
        if let Ok(tx_id) = res { B256::from_slice(tx_id.as_bytes()) } else { B256::ZERO };

    let out = PrecompileOutput::new(gas_used, byte_resp.into());
    Ok(out)
}

#[cfg(test)]
mod irys_pc_tests {
    use crate::inner::arweave_precompile::arweave_upload;
    use reth::primitives::revm_primitives::PrecompileOutput;
    use reth::primitives::Bytes;

    #[test]
    pub fn test_irys_precompile() {
        let input = Bytes::from("Hello world".as_bytes());
        let PrecompileOutput { gas_used, bytes } = arweave_upload(&input, 0 as u64).unwrap();
    }
}
