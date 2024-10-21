use crate::b256::BorshB256;
use alloy_primitives::Address;
use borsh::{BorshDeserialize, BorshSerialize};
use std::io::{Read, Write};

pub struct BorshAddress(pub Address);

impl BorshSerialize for BorshAddress {
    fn serialize<W: Write>(&self, writer: &mut W) -> std::io::Result<()> {
        BorshB256(self.0.into_word()).serialize(writer)
    }
}

impl BorshDeserialize for BorshAddress {
    fn deserialize_reader<R: Read>(reader: &mut R) -> std::io::Result<Self> {
        let address = BorshB256::deserialize_reader(reader)?;
        Ok(BorshAddress(Address::from_word(address.0)))
    }
}

#[cfg(test)]
mod address_tests {
    use crate::address::BorshAddress;
    use alloy_primitives::Address;

    #[test]
    pub fn test_sealed_header() {
        let data = Address::default();
        let borsh_data = BorshAddress(data.clone());
        let to_borsh = borsh::to_vec(&borsh_data).unwrap();
        let from_borsh: BorshAddress = borsh::from_slice(to_borsh.as_slice()).unwrap();
        assert_eq!(data, from_borsh.0);
    }
}
