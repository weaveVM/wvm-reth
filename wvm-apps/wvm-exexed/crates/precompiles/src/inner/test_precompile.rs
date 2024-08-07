use reth::primitives::Bytes;
use reth::primitives::revm_primitives::{Precompile, PrecompileOutput, PrecompileResult};
use reth::revm::precompile::{PrecompileWithAddress, u64_to_address};

pub const PC_ADDRESS: u64 = 0x19;
pub const HELLO_WORLD_PC: PrecompileWithAddress =
    PrecompileWithAddress(u64_to_address(PC_ADDRESS), Precompile::Standard(hello_world_pc));

fn hello_world_pc(_input: &Bytes, _gas_limit: u64) -> PrecompileResult {
    Ok(PrecompileOutput::new(0 as u64, "Hello World".into()))
}