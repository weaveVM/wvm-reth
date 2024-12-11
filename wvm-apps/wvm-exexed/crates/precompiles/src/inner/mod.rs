use crate::inner::{
    arweave_precompile::ARWEAVE_UPLOAD_PC, arweave_read_precompile::ARWEAVE_READ_PC,
    gbq_precompile::GBQ_READ_PC, kyve_precompile::KYVE_READ_PC, test_precompile::HELLO_WORLD_PC,
    wvm_block_precompile::WVM_BLOCK_PC,
};
use reth::revm::precompile::{u64_to_address, PrecompileWithAddress};

pub mod arweave_precompile;
mod arweave_read_precompile;
mod gbq_precompile;
pub mod graphql_util;
mod kyve_precompile;
mod string_block;
mod test_precompile;
mod util;
mod wvm_block_precompile;

fn hex_to_u64(hex_str: &str) -> u64 {
    u64::from_str_radix(&hex_str[2..], 16).unwrap()
}

fn is_pc_blocked(block_list: &Vec<&str>, addr: i32) -> bool {
    block_list.contains(&addr.to_string().as_str())
}

pub fn wvm_precompiles() -> impl Iterator<Item = PrecompileWithAddress> {
    // ORDER OF THINGS MATTER
    // ORDER OF THINGS MATTER

    let pcs_funcs = [
        ARWEAVE_UPLOAD_PC,
        ARWEAVE_READ_PC,
        HELLO_WORLD_PC,
        WVM_BLOCK_PC,
        KYVE_READ_PC,
        GBQ_READ_PC,
    ];
    let mut pcs = vec![];

    // IT MATTERS BC OF THIS
    let mut start_addr = 17;
    let blocked_pcs = std::env::var("BLOCKED_PC").unwrap_or("".to_string());
    let blocked_pcs = blocked_pcs.split(",").collect::<Vec<&str>>();

    for pc in pcs_funcs.into_iter() {
        if !is_pc_blocked(&blocked_pcs, start_addr) {
            let addr = hex_to_u64(format!("0x{}", start_addr).as_str());
            pcs.push(PrecompileWithAddress(u64_to_address(addr), pc));
        }
        start_addr = start_addr + 1;
    }

    pcs.into_iter()
}
#[cfg(test)]
mod pc_inner_tests {
    use crate::inner::{is_pc_blocked, wvm_precompiles};
    use reth::revm::precompile::u64_to_address;
    use reth_revm::precompile::PrecompileWithAddress;

    #[test]
    pub fn test_verify_pc() {
        std::env::set_var("BLOCKED_PC", "20,17");
        let blocked_pcs = std::env::var("BLOCKED_PC").unwrap_or("".to_string());
        let vec = blocked_pcs.split(",").collect::<Vec<&str>>();
        assert!(is_pc_blocked(&vec, 20));
        assert!(is_pc_blocked(&vec, 17));
        assert!(!is_pc_blocked(&vec, 1));
        let pcs = wvm_precompiles();
        let pcs: Vec<PrecompileWithAddress> = pcs.collect();
        assert_eq!(pcs.len(), 4);

        std::env::remove_var("BLOCKED_PC");
        let mut get_pcs = wvm_precompiles();
        let first = get_pcs.next().unwrap();
        assert_eq!(first.0, u64_to_address(0x17));
    }
}
