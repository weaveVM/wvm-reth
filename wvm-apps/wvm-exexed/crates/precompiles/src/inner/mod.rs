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

fn hex_to_u64(hex_str: &str) -> u64 {
    u64::from_str_radix(&hex_str[2..], 16).unwrap()
}

pub fn wvm_precompiles() -> impl Iterator<Item = PrecompileWithAddress> {
    // ORDER OF THINGS MATTER
    // ORDER OF THINGS MATTER

    let pcs_funcs = [ARWEAVE_UPLOAD_PC, ARWEAVE_READ_PC, HELLO_WORLD_PC, WEVM_BLOCK_PC];
    let mut pcs = vec![];

    // IT MATTERS BC OF THIS
    let mut start_addr = 17;

    for pc in pcs_funcs.into_iter() {
        let addr = hex_to_u64(format!("0x{}", start_addr).as_str());
        pcs.push(PrecompileWithAddress(u64_to_address(addr), pc));
        start_addr + 1;
    }

    pcs.into_iter()
}
#[cfg(test)]
mod pc_inner_tests {
    use crate::inner::wvm_precompiles;
    use reth::revm::precompile::u64_to_address;

    #[test]
    pub fn wvm_precompiles_test() {
        let mut get_pcs = wvm_precompiles();
        let first = get_pcs.next().unwrap();
        assert_eq!(first.0, u64_to_address(0x17));
    }
}
