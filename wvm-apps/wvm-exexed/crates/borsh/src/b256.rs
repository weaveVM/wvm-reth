use std::io::Write;
use borsh::BorshSerialize;
use reth::primitives::{B256, U256};

pub struct BorshB256(pub B256);
pub struct BorshU256(pub U256);

impl BorshSerialize for BorshB256 {
    fn serialize<W: Write>(&self, writer: &mut W) -> std::io::Result<()> {
        self.0.as_slice().serialize(writer)
    }
}

impl BorshSerialize for BorshU256 {
    fn serialize<W: Write>(&self, writer: &mut W) -> std::io::Result<()> {
        BorshB256(B256::from(self.0)).serialize(writer)
    }
}


