use crate::wvm::v1::{
    header::V1WvmHeader, transaction::V1WvmTransactionSigned, V1WvmSealedBlock,
    V1WvmSealedBlockWithSenders, V1WvmSealedHeader,
};

use reth_primitives::Block;
use reth_primitives_traits::RecoveredBlock;

pub mod v1;

// Define a trait for `magic_identifier`
pub trait MagicIdentifier {
    fn magic_identifier(&self) -> u8;
}

// Macro to generate enums and their `MagicIdentifier` trait implementation
macro_rules! define_wvm_enum {
    ($name:ident, { $($variant:ident($inner:ty) => $magic:expr),* $(,)? }) => {
        pub enum $name {
            $($variant($inner)),*
        }

        impl MagicIdentifier for $name {
            fn magic_identifier(&self) -> u8 {
                match self {
                    $(Self::$variant(_) => $magic),*
                }
            }
        }

        impl $name {
            $(
                paste::paste! {
                    pub fn [<as_ $variant:snake> ](&self) -> Option<&$inner> {
                        match self {
                    Self::$variant(value) => Some(value),
                    _ => None
                }
                    }
                }
            )*
        }
    };
}

// Define enums with versioned magic identifiers
define_wvm_enum!(WvmSealedBlock, {
    V1(V1WvmSealedBlock) => 0u8,
});

define_wvm_enum!(WvmSealedBlockWithSenders, {
    V1(V1WvmSealedBlockWithSenders) => 0u8,
});

impl From<RecoveredBlock<Block>> for WvmSealedBlockWithSenders {
    fn from(value: RecoveredBlock<Block>) -> Self {
        WvmSealedBlockWithSenders::V1(V1WvmSealedBlockWithSenders::from(value))
    }
}

define_wvm_enum!(WvmHeader, {
    V1(V1WvmHeader) => 0u8,
});

define_wvm_enum!(WvmSealedHeader, {
    V1(V1WvmSealedHeader) => 0u8,
});

define_wvm_enum!(WvmTransactionSigned, {
    V1(V1WvmTransactionSigned) => 0u8,
});
