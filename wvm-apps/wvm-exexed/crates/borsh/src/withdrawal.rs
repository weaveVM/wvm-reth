use crate::address::BorshAddress;
use borsh::{BorshDeserialize, BorshSerialize};
use reth::primitives::Withdrawal;
use std::io::{Read, Write};

pub struct BorshWithdrawal(pub Withdrawal);

impl BorshSerialize for BorshWithdrawal {
    fn serialize<W: Write>(&self, writer: &mut W) -> std::io::Result<()> {
        self.0.index.serialize(writer)?;
        self.0.validator_index.serialize(writer)?;
        BorshAddress(self.0.address).serialize(writer)?;
        self.0.amount.serialize(writer)?;
        Ok(())
    }
}

impl BorshDeserialize for BorshWithdrawal {
    fn deserialize_reader<R: Read>(reader: &mut R) -> std::io::Result<Self> {
        let index: u64 = BorshDeserialize::deserialize_reader(reader)?;
        let validator_index: u64 = BorshDeserialize::deserialize_reader(reader)?;
        let address = BorshAddress::deserialize_reader(reader)?;
        let amount: u64 = BorshDeserialize::deserialize_reader(reader)?;

        Ok(BorshWithdrawal(Withdrawal { index, validator_index, address: address.0, amount }))
    }
}

#[cfg(test)]
mod withdrawal_tests {
    use crate::withdrawal::BorshWithdrawal;
    use reth::primitives::Withdrawal;

    #[test]
    pub fn test_sealed_header() {
        let data = Withdrawal::default();
        let borsh_data = BorshWithdrawal(data.clone());
        let to_borsh = borsh::to_vec(&borsh_data).unwrap();
        let from_borsh: BorshWithdrawal = borsh::from_slice(to_borsh.as_slice()).unwrap();
        assert_eq!(data, from_borsh.0);
    }
}
