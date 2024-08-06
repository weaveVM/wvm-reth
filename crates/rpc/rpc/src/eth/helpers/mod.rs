//! The entire implementation of the namespace is quite large, hence it is divided across several
//! files.

pub mod signer;

mod block;
mod call;
mod fees;
<<<<<<< HEAD
#[cfg(feature = "optimism")]
pub mod optimism;
#[cfg(not(feature = "optimism"))]
mod pending_block;
#[cfg(not(feature = "optimism"))]
=======
mod pending_block;
>>>>>>> c4b5f5e9c9a88783b2def3ab1cc880b8d41867e1
mod receipt;
mod spec;
mod state;
mod trace;
mod transaction;
