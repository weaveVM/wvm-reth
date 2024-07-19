use std::io::Write;
use borsh::BorshSerialize;
use reth::primitives::{Transaction, TransactionSigned, TxType};
use reth::primitives::alloy_primitives::private::alloy_rlp::Encodable;
use crate::b256::BorshB256;

pub struct BorshTransactionSigned(pub TransactionSigned);
pub struct BorshTransaction(pub Transaction);

impl BorshSerialize for BorshTransaction {
    fn serialize<W: Write>(&self, writer: &mut W) -> std::io::Result<()> {
        let mut buff: Vec<u8> = serde_json::to_vec(&self.0).unwrap();
        buff.serialize(writer)?;
        Ok(())
    }
}

impl BorshSerialize for BorshTransactionSigned {
    fn serialize<W: Write>(&self, writer: &mut W) -> std::io::Result<()> {
        BorshB256(self.0.hash).serialize(writer)?;
        self.0.signature.to_bytes().serialize(writer)?;
        BorshTransaction(self.0.transaction.clone()).serialize(writer)?;

        Ok(())
    }
}