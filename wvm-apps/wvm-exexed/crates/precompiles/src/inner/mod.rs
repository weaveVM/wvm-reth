use reth::revm::precompile::PrecompileWithAddress;
use crate::inner::irys_precompile::IRYS_UPLOAD_PC;

pub mod irys_precompile;

pub fn wvm_precompiles() -> impl Iterator<Item = PrecompileWithAddress> {
    [
        IRYS_UPLOAD_PC
    ].into_iter()
}