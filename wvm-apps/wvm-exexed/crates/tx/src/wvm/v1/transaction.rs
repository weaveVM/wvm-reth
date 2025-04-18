use crate::WvmTransaction;
use alloy_primitives::{PrimitiveSignature, TxHash};
use reth_primitives::TransactionSigned;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct V1WvmTransactionSigned {
    pub hash: TxHash,
    pub signature: PrimitiveSignature,
    pub transaction: WvmTransaction,
}

impl V1WvmTransactionSigned {
    pub fn tx_signed(self) -> TransactionSigned {
        TransactionSigned::new(self.transaction.into(), self.signature.into(), self.hash.into())
    }
}

impl Into<TransactionSigned> for V1WvmTransactionSigned {
    fn into(self) -> TransactionSigned {
        TransactionSigned::new(self.transaction.into(), self.signature.into(), self.hash.into())
    }
}

impl From<TransactionSigned> for V1WvmTransactionSigned {
    fn from(value: TransactionSigned) -> Self {
        V1WvmTransactionSigned {
            hash: *value.hash(),
            signature: *value.signature(),
            transaction: value.transaction().clone().into(),
        }
    }
}
