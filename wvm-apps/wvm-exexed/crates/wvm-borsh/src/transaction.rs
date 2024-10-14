use crate::{b256::BorshB256, signature::BorshSignature};
use borsh::{BorshDeserialize, BorshSerialize};
use reth::primitives::{Transaction, TransactionSigned};
use std::io::{Read, Write};

pub struct BorshTransactionSigned(pub TransactionSigned);
pub struct BorshTransaction(pub Transaction);

impl BorshSerialize for BorshTransaction {
    fn serialize<W: Write>(&self, writer: &mut W) -> std::io::Result<()> {
        let buff: Vec<u8> = serde_json::to_vec(&self.0).unwrap();
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
            transaction: tx.0,
        }))
    }
}

#[cfg(test)]
mod txs_tests {
    use crate::transaction::BorshTransactionSigned;
    use reth::primitives::TransactionSigned;

    #[test]
    pub fn test_sealed_header() {
        let data = TransactionSigned::default();
        let borsh_data = BorshTransactionSigned(data.clone());
        let to_borsh = borsh::to_vec(&borsh_data).unwrap();
        let from_borsh: BorshTransactionSigned = borsh::from_slice(to_borsh.as_slice()).unwrap();
        assert_eq!(data, from_borsh.0);
    }
}
