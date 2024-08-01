use crate::inner::arweave_precompile::ARWEAVE_UPLOAD_PC;
use reth::revm::precompile::PrecompileWithAddress;

pub mod arweave_precompile;

pub fn wvm_precompiles() -> impl Iterator<Item = PrecompileWithAddress> {
    [ARWEAVE_UPLOAD_PC].into_iter()
}
