//! OP-Reth RPC support.

#![doc(
    html_logo_url = "https://raw.githubusercontent.com/paradigmxyz/reth/main/assets/reth-docs.png",
    html_favicon_url = "https://avatars0.githubusercontent.com/u/97369466?s=256",
    issue_tracker_base_url = "https://github.com/paradigmxyz/reth/issues/"
)]
<<<<<<< HEAD
#![cfg_attr(not(test), warn(unused_crate_dependencies))]
#![cfg_attr(docsrs, feature(doc_cfg, doc_auto_cfg))]

pub mod eth;
=======
#![cfg_attr(all(not(test), feature = "optimism"), warn(unused_crate_dependencies))]
#![cfg_attr(docsrs, feature(doc_cfg, doc_auto_cfg))]
// The `optimism` feature must be enabled to use this crate.
#![cfg(feature = "optimism")]

pub mod api;
pub mod error;
pub mod eth;

pub use api::OpEthApiServer;
pub use error::OpEthApiError;
pub use eth::{receipt::op_receipt_fields, transaction::OptimismTxMeta, OpEthApi};
>>>>>>> c4b5f5e9c9a88783b2def3ab1cc880b8d41867e1
