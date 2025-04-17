use crate::spec::DepositContract;
use alloy_eips::eip6110::MAINNET_DEPOSIT_CONTRACT_ADDRESS;
use alloy_primitives::b256;

/// Gas per transaction not creating a contract.
/// @LOAD_NETWORK: Raised from 21k to 500_000
pub const MIN_TRANSACTION_GAS: u64 = 500_000u64;

/// Mainnet prune delete limit.
pub const MAINNET_PRUNE_DELETE_LIMIT: usize = 20000;

/// Deposit contract address: `0x00000000219ab540356cbb839cbe05303d7705fa`
pub(crate) const MAINNET_DEPOSIT_CONTRACT: DepositContract = DepositContract::new(
    MAINNET_DEPOSIT_CONTRACT_ADDRESS,
    11052984,
    b256!("649bbc62d0e31342afea4e5cd82d4049e7e1ee912fc0889aa790803be39038c5"),
);

/// @LOAD_NETWORK:
/// CONSTANTS, GAS LIMIT AND FEES
use alloy_primitives::U256;

use fees::{
    util::raw_calculate_lowest_possible_gas_price,
    wvm_fee::{WvmFee, WvmFeeManager},
};

#[cfg(feature = "std")]
use std::{
    cell::LazyCell,
    sync::{
        atomic::{AtomicU64, Ordering::SeqCst},
        Arc, LazyLock,
    },
    time::Duration,
};

#[cfg(not(feature = "std"))]
use {
    alloc::{boxed::Box, sync::Arc},
    core::cell::LazyCell,
    core::{
        sync::atomic::{AtomicU64, Ordering::SeqCst},
        time::Duration,
    },
    reth_primitives_traits::sync::LazyLock,
};

/// WVM: we set 500kk gas limit
pub const LOAD_NETWORK_BLOCK_GAS_LIMIT: LazyCell<u64> = LazyCell::new(|| 500_000_000); // WVM: 500_000_000 gas limit

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

/// Same as [`reth_primitives_traits::constants::MIN_PROTOCOL_BASE_FEE`] but as a U256.
pub const MIN_PROTOCOL_BASE_FEE_U256: U256 = U256::from_limbs([640_000_000u64, 0u64, 0u64, 0u64]);

/// The WVM fee manager singleton that handles fee calculations and updates across the system.
/// This manager maintains the dynamic fee state and provides fee calculation services.
pub static WVM_FEE_MANAGER: LazyLock<Arc<WvmFeeManager>> = LazyLock::new(|| {
    let fee = WvmFee::new(Some(Box::new(move |price| {
        let original_price = price as f64 / 1_000_000_000f64;
        let lowest_possible_gas_price_in_gwei =
            raw_calculate_lowest_possible_gas_price(original_price, *LOAD_NETWORK_BLOCK_GAS_LIMIT);
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
