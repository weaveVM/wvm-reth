use reth_chainspec::LOAD_NETWORK_BLOCK_GAS_LIMIT;
use std::{cell::LazyCell, env::VarError};

/// [`TX_SLOT_BYTE_SIZE`] is used to calculate how many data slots a single transaction
/// takes up based on its byte size. The slots are used as `DoS` protection, ensuring
/// that validating a new transaction remains a constant operation (in reality
/// O(maxslots), where max slots are 4 currently).
pub const TX_SLOT_BYTE_SIZE: usize = 32 * 1024;

/// [`DEFAULT_MAX_TX_INPUT_BYTES`] is the default maximum size a single transaction can have. This
/// field has non-trivial consequences: larger transactions are significantly harder and
/// more expensive to propagate; larger transactions also take more resources
/// to validate whether they fit into the pool or not. Default is 4 times [`TX_SLOT_BYTE_SIZE`],
/// which defaults to 32 KiB, so 128 KiB.
pub const DEFAULT_MAX_TX_INPUT_BYTES: LazyCell<usize> = LazyCell::new(|| {
    let default_value = {
        // 20k (gas) ----> 128kb
        // 500k (gas) ----> x // 3200
        let gas_limit = (&*LOAD_NETWORK_BLOCK_GAS_LIMIT).clone();

        (((gas_limit * 128_000) / 20_000) / 1000) as usize // -> to bytes (3.2mb)
    };

    let env_var = std::env::var("WVM_DEFAULT_MAX_TX_INPUT_BYTES");
    match env_var {
        Ok(res) => res.parse::<usize>().unwrap_or(default_value),
        Err(_) => default_value,
    }
}); // 128KB

/// This represents how big we want to allow a contract size to be in multiples of 24kb
/// 2 = 48kb
/// 3 = 72kb
pub const DEFAULT_MULTIPLY_VAL_FOR_CODE_SIZE: usize = 2;

/// Maximum bytecode to permit for a contract.
pub const MAX_CODE_BYTE_SIZE: usize = 24576 * DEFAULT_MULTIPLY_VAL_FOR_CODE_SIZE;

/// Maximum initcode to permit in a creation transaction and create instructions.
pub const MAX_INIT_CODE_BYTE_SIZE: usize = 2 * MAX_CODE_BYTE_SIZE;
