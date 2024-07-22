use borsh::{BorshDeserialize, BorshSerialize};
use reth::primitives::{B256, U256};
use std::io::{Read, Write};

pub struct BorshB256(pub B256);
pub struct BorshU256(pub U256);

impl BorshSerialize for BorshB256 {
    fn serialize<W: Write>(&self, writer: &mut W) -> std::io::Result<()> {
        let ser = self.0.as_slice().to_vec();
        ser.serialize(writer)
    }
}

impl BorshDeserialize for BorshB256 {
    fn deserialize_reader<R: Read>(reader: &mut R) -> std::io::Result<Self> {
        let reader = Vec::<u8>::deserialize_reader(reader)?;
        let val: B256 = B256::from_slice(reader.as_slice());
        Ok(BorshB256(val))
    }
}

impl BorshSerialize for BorshU256 {
    fn serialize<W: Write>(&self, writer: &mut W) -> std::io::Result<()> {
        self.0.to_le_bytes_vec().serialize(writer)
    }
}

impl BorshDeserialize for BorshU256 {
    fn deserialize_reader<R: Read>(reader: &mut R) -> std::io::Result<Self> {
        let bu256_le_bytes = Vec::<u8>::deserialize_reader(reader)?;
        let u256 = U256::from_le_slice(bu256_le_bytes.as_slice());
        Ok(BorshU256(u256))
    }
}

#[cfg(test)]
mod b256_tests {
    use crate::b256::{BorshB256, BorshU256};
    use reth::primitives::{B256, U256};

    #[test]
    pub fn test_borsh_b256_ser_der() {
        let b256 = B256::random();
        let bclone = b256.clone();
        let borsh_b256 = BorshB256(b256);
        let borsh_ser = borsh::to_vec(&borsh_b256).unwrap();
        let borsh_der: BorshB256 = borsh::from_slice(borsh_ser.as_slice()).unwrap();
        assert_eq!(bclone, borsh_der.0);
    }

    #[test]
    pub fn test_borsh_u256_ser_der() {
        let u256 = U256::MAX;
        let uclone = u256.clone();
        let borsh_u256 = BorshU256(u256);
        let borsh_ser = borsh::to_vec(&borsh_u256).unwrap();
        let borsh_der: BorshU256 = borsh::from_slice(borsh_ser.as_slice()).unwrap();
        assert_eq!(uclone, borsh_der.0);
    }
}
