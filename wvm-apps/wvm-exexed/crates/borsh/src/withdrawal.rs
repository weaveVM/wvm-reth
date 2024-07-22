use std::io::{Read, Write};
use borsh::{BorshDeserialize, BorshSerialize};
use reth::primitives::{Address, Withdrawal};
use crate::b256::BorshB256;

pub struct BorshWithdrawal(pub Withdrawal);

impl BorshSerialize for BorshWithdrawal {
    fn serialize<W: Write>(&self, writer: &mut W) -> std::io::Result<()> {
        self.0.index.serialize(writer)?;
        self.0.validator_index.serialize(writer)?;
        BorshB256(self.0.address.into_word()).serialize(writer)?;
        self.0.amount.serialize(writer)?;
        Ok(())
    }
}

impl BorshDeserialize for BorshWithdrawal {
    fn deserialize_reader<R: Read>(reader: &mut R) -> std::io::Result<Self> {
        let index: u64 = BorshDeserialize::deserialize_reader(reader)?;
        let validator_index: u64 = BorshDeserialize::deserialize_reader(reader)?;
        let address = BorshB256::deserialize_reader(reader)?;
        let amount: u64 = BorshDeserialize::deserialize_reader(reader)?;

        Ok(BorshWithdrawal(Withdrawal {
            index,
            validator_index,
            address: Address::from_word(address.0),
            amount
        }))
    }
}