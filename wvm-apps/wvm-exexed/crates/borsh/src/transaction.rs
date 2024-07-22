use std::io::{Error, ErrorKind, Read, Write};
use borsh::{BorshDeserialize, BorshSerialize};
use reth::primitives::{Signature, Transaction, TransactionSigned, TxType, U256};
use reth::primitives::alloy_primitives::private::alloy_rlp::Encodable;
use crate::b256::BorshB256;
use crate::signature::BorshSignature;

pub struct BorshTransactionSigned(pub TransactionSigned);
pub struct BorshTransaction(pub Transaction);

impl BorshSerialize for BorshTransaction {
    fn serialize<W: Write>(&self, writer: &mut W) -> std::io::Result<()> {
        let mut buff: Vec<u8> = serde_json::to_vec(&self.0).unwrap();
        buff.serialize(writer)?;
        Ok(())
    }
}

impl BorshDeserialize for BorshTransaction {
    fn deserialize_reader<R: Read>(reader: &mut R) -> std::io::Result<Self> {
        let bytes = Vec::<u8>::deserialize_reader(reader)?;
        let tx: Transaction = serde_json::from_slice(bytes.as_slice()).unwrap();
        Ok(BorshTransaction(tx))
    }
}

impl BorshSerialize for BorshTransactionSigned {
    fn serialize<W: Write>(&self, writer: &mut W) -> std::io::Result<()> {
        BorshB256(self.0.hash).serialize(writer)?;
        BorshSignature(self.0.signature).serialize(writer)?;
        BorshTransaction(self.0.transaction.clone()).serialize(writer)?;

        Ok(())
    }
}

impl BorshDeserialize for BorshTransactionSigned {
    fn deserialize_reader<R: Read>(reader: &mut R) -> std::io::Result<Self> {
        let hash = BorshB256::deserialize_reader(reader)?;
        let bytes_signature = BorshSignature::deserialize_reader(reader)?;
        let tx = BorshTransaction::deserialize_reader(reader)?;

        Ok(BorshTransactionSigned(TransactionSigned {
            hash: hash.0,
            signature: bytes_signature.0,
            transaction: tx.0
        }))
    }
}