use crate::WvmTransaction;
use alloy_primitives::{Signature, TxHash};
use derive_more::{AsRef, Deref};
use reth_primitives::{Transaction, TransactionSigned};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct V1WvmTransactionSigned {
    pub hash: TxHash,
    pub signature: Signature,
    pub transaction: WvmTransaction,
}

impl V1WvmTransactionSigned {
    pub fn tx_signed(self) -> TransactionSigned {
        TransactionSigned {
            hash: self.hash,
            signature: self.signature,
            transaction: self.transaction.into(),
        }
    }
}

impl Into<TransactionSigned> for V1WvmTransactionSigned {
    fn into(self) -> TransactionSigned {
        TransactionSigned {
            hash: self.hash,
            signature: self.signature,
            transaction: self.transaction.into(),
        }
    }
}

impl From<TransactionSigned> for V1WvmTransactionSigned {
    fn from(value: TransactionSigned) -> Self {
        V1WvmTransactionSigned {
            hash: value.hash,
            signature: value.signature,
            transaction: value.transaction.into(),
        }
    }
}
