<<<<<<<< HEAD:crates/ethereum/engine/src/lib.rs
//! Ethereum engine implementation.
========
//! Reth CLI implementation.
>>>>>>>> c4b5f5e9c9a88783b2def3ab1cc880b8d41867e1:crates/ethereum/cli/src/lib.rs

#![doc(
    html_logo_url = "https://raw.githubusercontent.com/paradigmxyz/reth/main/assets/reth-docs.png",
    html_favicon_url = "https://avatars0.githubusercontent.com/u/97369466?s=256",
    issue_tracker_base_url = "https://github.com/paradigmxyz/reth/issues/"
)]
#![cfg_attr(not(test), warn(unused_crate_dependencies))]
#![cfg_attr(docsrs, feature(doc_cfg, doc_auto_cfg))]

<<<<<<<< HEAD:crates/ethereum/engine/src/lib.rs
/// Ethereum engine service.
pub mod service;
========
/// Chain specification parser.
pub mod chainspec;
>>>>>>>> c4b5f5e9c9a88783b2def3ab1cc880b8d41867e1:crates/ethereum/cli/src/lib.rs
