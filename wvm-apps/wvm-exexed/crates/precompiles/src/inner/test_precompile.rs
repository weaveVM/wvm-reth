use reth::primitives::{
    revm_primitives::{Precompile, PrecompileOutput, PrecompileResult},
};
use alloy_primitives::Bytes;

pub const HELLO_WORLD_PC: Precompile = Precompile::Standard(hello_world_pc);

fn hello_world_pc(_input: &Bytes, _gas_limit: u64) -> PrecompileResult {
    Ok(PrecompileOutput::new(0 as u64, "Hello World".into()))
}
