use std::io::Write;
use borsh::BorshSerialize;
use reth::primitives::{Header, SealedHeader};
use crate::b256::{BorshB256, BorshU256};

pub struct BorshHeader(pub Header);
pub struct BorshSealedHeader(pub SealedHeader);

impl BorshSerialize for BorshHeader {
    fn serialize<W: Write>(&self, writer: &mut W) -> std::io::Result<()> {
        BorshB256(self.0.parent_hash).serialize(writer)?;
        BorshB256(self.0.ommers_hash).serialize(writer)?;
        self.0.beneficiary.into_array().serialize(writer)?;
        BorshB256(self.0.state_root).serialize(writer)?;
        BorshB256(self.0.transactions_root).serialize(writer)?;
        BorshB256(self.0.receipts_root).serialize(writer)?;
        self.0.withdrawals_root.map(|v| BorshB256(v)).serialize(writer)?;
        self.0.logs_bloom.0.0.serialize(writer)?;
        BorshU256(self.0.difficulty).serialize(writer)?;
        self.0.number.serialize(writer)?;
        self.0.gas_limit.serialize(writer)?;
        self.0.gas_used.serialize(writer)?;
        self.0.timestamp.serialize(writer)?;
        BorshB256(self.0.mix_hash).serialize(writer)?;
        self.0.nonce.serialize(writer)?;
        self.0.base_fee_per_gas.serialize(writer)?;
        self.0.blob_gas_used.serialize(writer)?;
        self.0.excess_blob_gas.serialize(writer)?;
        self.0.parent_beacon_block_root.map(|v| BorshB256(v)).serialize(writer)?;
        self.0.requests_root.map(|v| BorshB256(v)).serialize(writer)?;
        self.0.extra_data.0.serialize(writer)?;

        Ok(())
    }
}

impl BorshSerialize for BorshSealedHeader {
    fn serialize<W: Write>(&self, writer: &mut W) -> std::io::Result<()> {
        BorshB256(self.0.hash()).serialize(writer)?;
        BorshHeader(self.0.header().clone()).serialize(writer)?;

        Ok(())
    }
}