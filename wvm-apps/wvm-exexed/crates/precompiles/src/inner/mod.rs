use crate::inner::arweave_precompile::ARWEAVE_UPLOAD_PC;
use crate::inner::arweave_read_precompile::ARWEAVE_READ_PC;
use crate::inner::test_precompile::HELLO_WORLD_PC;
use crate::inner::wevm_block_precompile::WEVM_BLOCK_PC;
use reth::revm::precompile::{u64_to_address, PrecompileWithAddress};

pub mod arweave_precompile;
mod arweave_read_precompile;
mod graphql_util;
mod string_block;
mod test_precompile;
mod util;
mod wevm_block_precompile;

pub fn wvm_precompiles() -> impl Iterator<Item = PrecompileWithAddress> {
    // ORDER OF THINGS MATTER
    // ORDER OF THINGS MATTER

    let pcs_funcs = [ARWEAVE_UPLOAD_PC, ARWEAVE_READ_PC, HELLO_WORLD_PC, WEVM_BLOCK_PC];
    let mut pcs = vec![];

    // IT MATTERS BC OF THIS
    let mut start_addr: u64 = 0x17;

    for pc in pcs_funcs.into_iter() {
        pcs.push(PrecompileWithAddress(u64_to_address(start_addr), pc));
        start_addr + 1;
    }

    pcs.into_iter()
}
