use std::io::Write;
use borsh::BorshSerialize;
use reth::primitives::Withdrawal;

pub struct BorshWithdrawal(pub Withdrawal);

impl BorshSerialize for BorshWithdrawal {
    fn serialize<W: Write>(&self, writer: &mut W) -> std::io::Result<()> {
        self.0.index.serialize(writer)?;
        self.0.validator_index.serialize(writer)?;
        self.0.address.into_array().serialize(writer)?;
        self.0.amount.serialize(writer)?;
        Ok(())
    }
}