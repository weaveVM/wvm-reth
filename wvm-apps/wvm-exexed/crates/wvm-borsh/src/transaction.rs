use crate::{b256::BorshB256, signature::BorshSignature};
use borsh::{BorshDeserialize, BorshSerialize};
use reth::primitives::{Transaction, TransactionSigned};

use std::io::{Read, Write};
use wvm_tx::{
    wvm::{v1::transaction::V1WvmTransactionSigned, MagicIdentifier, WvmTransactionSigned},
    WvmTransaction,
};

pub struct BorshTransactionSigned(pub WvmTransactionSigned);
pub struct BorshTransaction(pub WvmTransaction);

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
        let tx: WvmTransaction = serde_json::from_slice(bytes.as_slice()).unwrap();
        Ok(BorshTransaction(tx))
    }
}

impl BorshSerialize for BorshTransactionSigned {
    fn serialize<W: Write>(&self, writer: &mut W) -> std::io::Result<()> {
        self.0.magic_identifier().serialize(writer)?;

        match &self.0 {
            WvmTransactionSigned::V1(transaction_signed) => {
                BorshB256(transaction_signed.hash).serialize(writer)?;
                BorshSignature(transaction_signed.signature).serialize(writer)?;
                BorshTransaction(transaction_signed.transaction.clone()).serialize(writer)?;
            }
        }

        Ok(())
    }
}

impl BorshDeserialize for BorshTransactionSigned {
    fn deserialize_reader<R: Read>(reader: &mut R) -> std::io::Result<Self> {
        let magic_identifier: u8 = u8::deserialize_reader(reader)?;

        match magic_identifier {
            0u8 => {
                let hash = BorshB256::deserialize_reader(reader)?;
                let bytes_signature = BorshSignature::deserialize_reader(reader)?;
                let tx = BorshTransaction::deserialize_reader(reader)?;

                Ok(BorshTransactionSigned(WvmTransactionSigned::V1(V1WvmTransactionSigned {
                    hash: hash.0,
                    signature: bytes_signature.0,
                    transaction: tx.0,
                })))
            }
            _ => Err(std::io::Error::new(std::io::ErrorKind::Other, "Invalid Magic Identifier")),
        }
    }
}

#[cfg(test)]
mod txs_tests {
    use crate::transaction::BorshTransactionSigned;
    use reth::primitives::TransactionSigned;

    use reth::primitives::Transaction;
    use serde_json::Value;
    use wvm_tx::{
        wvm::{v1::transaction::V1WvmTransactionSigned, WvmTransactionSigned},
        WvmTransaction,
    };

    #[test]
    pub fn test_sealed_header() {
        let data = TransactionSigned::default();
        let borsh_data = BorshTransactionSigned(WvmTransactionSigned::V1(
            V1WvmTransactionSigned::from(data.clone()),
        ));
        let to_borsh = borsh::to_vec(&borsh_data).unwrap();
        let from_borsh: BorshTransactionSigned = borsh::from_slice(to_borsh.as_slice()).unwrap();
        assert_eq!(*data.hash(), from_borsh.0.as_v1().unwrap().hash);
    }

    #[test]
    pub fn test_serialize_legacy() {
        let legacy_data = r#"
        {
  "Legacy": {
    "chain_id": 9496,
    "gas_limit": 28647,
    "gas_price": 1000000007,
    "input": "0x1b6905401c07768cbc769037274298db8131a7d1124d94122c21bc879b1a1b306e8b7feae6f98bbcf93736d20335702d5f83b3c22cf4c4b6061c4ae28f9eb96bb71a14b59774a20b24ca316980bf72c21f2004a4b39a8ab1ba4724f790d6b527f5e573d1d6d0353187fc855234919a0d62b323e5d97c6bfea2e106aa96bd60b4ea2df0aa9c55fa863cb49201d9a47d1a90c5a63d2d8190c7d128b3c5ec71f744cb1e53f68d6b8175e896ca3ae07faea23292f800ff5a4863f3d5af7de6bd836692d65b63d0ae52f5a857a9b2dbd96575b9113733758b836105e20ac086ff4c2624710c36121d430f5134b70394712096c1841ca6232e400bfd8f100c00f90d054c84befabf859ec7e734f7f5db07bb7437e2006d66f41134788e08733d537fedbe5bcf5b50cdad6ec6af74cc9bc31f810a70e18ee6b8695d5ebd014c54392fd3923c0663652afb543904cc91f278b2df07b2f3a79b677db64dc632cba544adb47b18731e120b56b0a731be8344a61b86e1786c7fc39473cadba3084723facc942ac40bd034def5d55d1ade034d2b25214b7fa820f11a8d7a70e3c3e8a95680c7be648dc840eafe82e786ddf992c4ef1537e126cd00e8614e56323b129763b13132ec155ab746e3dbef459000",
    "nonce": 15974,
    "to": "0xa2a0d977847805fe224b789d8c4d3d711ab251e7",
    "value": "0x0"
  }
}"#;
        let to_tx: WvmTransaction = serde_json::from_str(legacy_data).unwrap();
        let newer_data = r#"
        {
  "Legacy": {
    "chainId": 9496,
    "gasLimit": 28647,
    "gasPrice": 1000000007,
    "input": "0x1b6905401c07768cbc769037274298db8131a7d1124d94122c21bc879b1a1b306e8b7feae6f98bbcf93736d20335702d5f83b3c22cf4c4b6061c4ae28f9eb96bb71a14b59774a20b24ca316980bf72c21f2004a4b39a8ab1ba4724f790d6b527f5e573d1d6d0353187fc855234919a0d62b323e5d97c6bfea2e106aa96bd60b4ea2df0aa9c55fa863cb49201d9a47d1a90c5a63d2d8190c7d128b3c5ec71f744cb1e53f68d6b8175e896ca3ae07faea23292f800ff5a4863f3d5af7de6bd836692d65b63d0ae52f5a857a9b2dbd96575b9113733758b836105e20ac086ff4c2624710c36121d430f5134b70394712096c1841ca6232e400bfd8f100c00f90d054c84befabf859ec7e734f7f5db07bb7437e2006d66f41134788e08733d537fedbe5bcf5b50cdad6ec6af74cc9bc31f810a70e18ee6b8695d5ebd014c54392fd3923c0663652afb543904cc91f278b2df07b2f3a79b677db64dc632cba544adb47b18731e120b56b0a731be8344a61b86e1786c7fc39473cadba3084723facc942ac40bd034def5d55d1ade034d2b25214b7fa820f11a8d7a70e3c3e8a95680c7be648dc840eafe82e786ddf992c4ef1537e126cd00e8614e56323b129763b13132ec155ab746e3dbef459000",
    "nonce": 15974,
    "to": "0xa2a0d977847805fe224b789d8c4d3d711ab251e7",
    "value": "0x0"
  }
}"#;

        let newer_tx: WvmTransaction = serde_json::from_str(newer_data).unwrap();
        println!("{:?}", newer_tx);

        let txs: (Transaction, Transaction) = (to_tx.into(), newer_tx.into());
        println!("{:?}", txs.0);
        println!("{:?}", txs.1);
        assert_eq!(txs.0.as_legacy().unwrap().chain_id.unwrap(), 9496);
        assert_eq!(txs.1.as_legacy().unwrap().chain_id.unwrap(), 9496);
    }
}
