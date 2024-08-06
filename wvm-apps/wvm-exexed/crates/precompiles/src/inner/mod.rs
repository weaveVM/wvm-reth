use crate::inner::{
    arweave_precompile::ARWEAVE_UPLOAD_PC, arweave_read_precompile::ARWEAVE_READ_PC,
};
use reth::revm::precompile::PrecompileWithAddress;

pub mod arweave_precompile;
mod arweave_read_precompile;

pub fn wvm_precompiles() -> impl Iterator<Item = PrecompileWithAddress> {
    [ARWEAVE_UPLOAD_PC, ARWEAVE_READ_PC].into_iter()
}
