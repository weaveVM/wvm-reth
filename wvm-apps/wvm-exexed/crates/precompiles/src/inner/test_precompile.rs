use alloy_primitives::Bytes;

use reth::revm::precompile::{PrecompileError, PrecompileFn, PrecompileOutput, PrecompileResult};

pub const HELLO_WORLD_PC: PrecompileFn = hello_world_pc as PrecompileFn;

fn hello_world_pc(_input: &Bytes, _gas_limit: u64) -> PrecompileResult {
    Ok(PrecompileOutput::new(0 as u64, "Hello World".into()))
}
