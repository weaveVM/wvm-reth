use irys::irys::IrysRequest;
use reth::primitives::Bytes;
use reth::primitives::revm_primitives::{Precompile, PrecompileOutput, PrecompileResult};
use reth::revm::precompile::{PrecompileWithAddress, u64_to_address};
use reth_revm::primitives::B256;

pub const PC_ADDRESS: u64 = 0x99;
pub const IRYS_PC_BASE: u64 = 3_450;

pub const IRYS_UPLOAD_PC: PrecompileWithAddress =
    PrecompileWithAddress(u64_to_address(PC_ADDRESS), Precompile::Standard(irys_upload));

fn irys_upload(input: &Bytes, gas_limit: u64) -> PrecompileResult {
    let res = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            IrysRequest::new()
                .set_tag("Content-Type", "application/octet-stream")
                .set_tag("WeaveVM:Encoding", "Borsh-Brotli")
                .set_tag("WeaveVM:Precompile", "true")
                .set_data(input.0.to_vec())
                .send()
                .await
        });

    let byte_resp = if let Ok(tx_id) = res {
        B256::from_slice(tx_id.as_bytes())
    } else {
        B256::ZERO
    };


    let out = PrecompileOutput::new(IRYS_PC_BASE, byte_resp.into());
    Ok(out)
}