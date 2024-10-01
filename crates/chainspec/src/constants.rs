use std::sync::{Arc, LazyLock, Mutex, RwLock};
use std::sync::atomic::AtomicU64;
use crate::spec::DepositContract;
use alloy_primitives::{address, b256};
use fees::wvm_fee::{WvmFee, WvmFeeManager};

pub static MIN_TRANSACTION_GAS: LazyLock<Arc<RwLock<u64>>> = LazyLock::new(|| {
    Arc::new(RwLock::new(0))
});

pub(crate) static WVM_FEE_MANAGER: LazyLock<Arc<WvmFeeManager>> = LazyLock::new(|| {
    let fee = WvmFee::new(Some(Box::new(move |price| {
        let mut writer = MIN_TRANSACTION_GAS.write().unwrap();
        *writer = price as u64;
        Ok(())
    })));

    fee.init();

    let manager = WvmFeeManager::new(Arc::new(fee));
    manager.init();

    Arc::new(manager)
});

pub fn get_latest_gas_fee() -> u64 {
    let at_ref = (&*MIN_TRANSACTION_GAS).clone();
    let gas_ref_read = at_ref.read().unwrap();
    (&gas_ref_read as &u64).clone()
}

/// Gas per transaction not creating a contract.
// pub const MIN_TRANSACTION_GAS: u64 = 21_000u64;
/// Deposit contract address: `0x00000000219ab540356cbb839cbe05303d7705fa`
pub(crate) const MAINNET_DEPOSIT_CONTRACT: DepositContract = DepositContract::new(
    address!("00000000219ab540356cbb839cbe05303d7705fa"),
    11052984,
    b256!("649bbc62d0e31342afea4e5cd82d4049e7e1ee912fc0889aa790803be39038c5"),
);

#[cfg(test)]
mod wvm_tests_chainspec {
    use std::time::Duration;
    use crate::constants::{get_latest_gas_fee, WVM_FEE_MANAGER};
    use crate::MIN_TRANSACTION_GAS;

    #[tokio::test]
    pub async fn test_wvm_fee_manager() {
        let init = &*WVM_FEE_MANAGER;
        tokio::time::sleep(Duration::from_secs(10)).await;
        let latest_gas = get_latest_gas_fee();
        assert!(&latest_gas > &300_000);
        assert!(&latest_gas < &400_000);
    }

}