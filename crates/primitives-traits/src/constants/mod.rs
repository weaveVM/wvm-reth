//! Ethereum protocol-related constants

<<<<<<< HEAD
use alloy_primitives::{b256, B256, U256};
use core::time::Duration;
use fees::{
    util::raw_calculate_lowest_possible_gas_price,
    wvm_fee::{WvmFee, WvmFeeManager},
};
use std::{
    cell::LazyCell,
    sync::{
        atomic::{AtomicU64, Ordering::SeqCst},
        Arc, LazyLock,
    },
};

=======
>>>>>>> upstream-v1.2.0
/// Gas units, for example [`GIGAGAS`].
pub mod gas_units;
pub use gas_units::{GIGAGAS, KILOGAS, MEGAGAS};

/// The client version: `reth/v{major}.{minor}.{patch}`
pub const RETH_CLIENT_VERSION: &str = concat!("reth/v", env!("CARGO_PKG_VERSION"));

/// The first four bytes of the call data for a function call specifies the function to be called.
pub const SELECTOR_LEN: usize = 4;

/// Maximum extra data size in a block after genesis
pub const MAXIMUM_EXTRA_DATA_SIZE: usize = 32;

/// An EPOCH is a series of 32 slots.
pub const EPOCH_SLOTS: u64 = 32;

/// The duration of a slot in seconds.
///
/// This is the time period of 1 seconds in which a randomly chosen validator has time to propose a
/// block.
pub const SLOT_DURATION: Duration = Duration::from_secs(1); // wvm #356: 1s per block

/// An EPOCH is a series of 32 slots (~6.4min).
pub const EPOCH_DURATION: Duration = Duration::from_secs(12 * EPOCH_SLOTS);

/// The default block nonce in the beacon consensus
pub const BEACON_NONCE: u64 = 0u64;

/// The default Ethereum block gas limit.
// TODO: This should be a chain spec parameter.
/// See <https://github.com/paradigmxyz/reth/issues/3233>.
/// WVM: we set 300kk gas limit
pub const ETHEREUM_BLOCK_GAS_LIMIT: LazyCell<u64> = LazyCell::new(|| 500_000_000); // WVM: 500_000_000 gas limit

/// The minimum tx fee below which the txpool will reject the transaction.
///
/// Configured to `7` WEI which is the lowest possible value of base fee under mainnet EIP-1559
/// parameters. `BASE_FEE_MAX_CHANGE_DENOMINATOR` <https://eips.ethereum.org/EIPS/eip-1559>
/// is `8`, or 12.5%. Once the base fee has dropped to `7` WEI it cannot decrease further because
/// 12.5% of 7 is less than 1.
///
/// Note that min base fee under different 1559 parameterizations may differ, but there's no
/// significant harm in leaving this setting as is.
// pub const MIN_PROTOCOL_BASE_FEE: u64 = 7;

// WVM: min base fee 7 => 500k
pub static MIN_PROTOCOL_BASE_FEE: LazyLock<AtomicU64> =
    LazyLock::new(|| AtomicU64::new(500_000u64));

/// The WVM fee manager singleton that handles fee calculations and updates across the system.
/// This manager maintains the dynamic fee state and provides fee calculation services.
pub static WVM_FEE_MANAGER: LazyLock<Arc<WvmFeeManager>> = LazyLock::new(|| {
    let fee = WvmFee::new(Some(Box::new(move |price| {
        let original_price = price as f64 / 1_000_000_000f64;
        let lowest_possible_gas_price_in_gwei =
            raw_calculate_lowest_possible_gas_price(original_price, *ETHEREUM_BLOCK_GAS_LIMIT);
        let mut to_wei = lowest_possible_gas_price_in_gwei * 1e9;
        // WVM: minimum fee check
        if to_wei < 500_000f64 {
            to_wei = 500_000f64;
        }
        MIN_PROTOCOL_BASE_FEE.store(to_wei as u64, SeqCst);
        Ok(())
    })));

    fee.init();

    let manager = WvmFeeManager::new(Arc::new(fee));
    manager.init();

    Arc::new(manager)
});

/// Returns the current minimum protocol base fee
pub fn get_latest_min_protocol_base_fee() -> u64 {
    MIN_PROTOCOL_BASE_FEE.load(SeqCst)
}

/// Same as [`MIN_PROTOCOL_BASE_FEE`] but as a U256.
pub const MIN_PROTOCOL_BASE_FEE_U256: U256 = U256::from_limbs([640_000_000u64, 0u64, 0u64, 0u64]);

/// Initial base fee as defined in [EIP-1559](https://eips.ethereum.org/EIPS/eip-1559)
pub const EIP1559_INITIAL_BASE_FEE: u64 = 1_000_000_000;

/// Base fee max change denominator as defined in [EIP-1559](https://eips.ethereum.org/EIPS/eip-1559)
pub const EIP1559_DEFAULT_BASE_FEE_MAX_CHANGE_DENOMINATOR: u64 = 8;

/// Elasticity multiplier as defined in [EIP-1559](https://eips.ethereum.org/EIPS/eip-1559)
pub const EIP1559_DEFAULT_ELASTICITY_MULTIPLIER: u64 = 2;

/// Minimum gas limit allowed for transactions.
pub const MINIMUM_GAS_LIMIT: u64 = 5000;

/// The bound divisor of the gas limit, used in update calculations.
pub const GAS_LIMIT_BOUND_DIVISOR: u64 = 1024;

/// The number of blocks to unwind during a reorg that already became a part of canonical chain.
///
/// In reality, the node can end up in this particular situation very rarely. It would happen only
/// if the node process is abruptly terminated during ongoing reorg and doesn't boot back up for
/// long period of time.
///
/// Unwind depth of `3` blocks significantly reduces the chance that the reorged block is kept in
/// the database.
pub const BEACON_CONSENSUS_REORG_UNWIND_DEPTH: u64 = 3;

#[cfg(test)]
mod tests {
    use super::*;

    use crate::constants::{get_latest_min_protocol_base_fee, WVM_FEE_MANAGER};
    use std::time::Duration;

    #[tokio::test]
    pub async fn test_wvm_fee_manager() {
        let init = &*WVM_FEE_MANAGER;
        tokio::time::sleep(Duration::from_secs(10)).await;
        let latest_gas = get_latest_min_protocol_base_fee();
        assert!(&latest_gas > &630000000);
        println!("{}", latest_gas);
        assert!(&latest_gas < &650000000);
    }

    #[tokio::test]
    async fn min_protocol_sanity() {
        let init = &*WVM_FEE_MANAGER;
        tokio::time::sleep(Duration::from_secs(10)).await;
        assert_eq!(MIN_PROTOCOL_BASE_FEE_U256.to::<u64>(), get_latest_min_protocol_base_fee());
    }
}
