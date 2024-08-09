use crate::{
    address::BorshAddress,
    b256::{BorshB256, BorshU256},
    bloom::BorshBloom,
};
use borsh::{BorshDeserialize, BorshSerialize};
use reth::primitives::{Bytes, Header, SealedHeader};
use std::io::{Read, Write};

pub struct BorshHeader(pub Header);
pub struct BorshSealedHeader(pub SealedHeader);

impl BorshSerialize for BorshHeader {
    fn serialize<W: Write>(&self, writer: &mut W) -> std::io::Result<()> {
        BorshB256(self.0.parent_hash).serialize(writer)?;
        BorshB256(self.0.ommers_hash).serialize(writer)?;
        BorshAddress(self.0.beneficiary).serialize(writer)?;
        BorshB256(self.0.state_root).serialize(writer)?;
        BorshB256(self.0.transactions_root).serialize(writer)?;
        BorshB256(self.0.receipts_root).serialize(writer)?;
        self.0.withdrawals_root.map(|v| BorshB256(v)).serialize(writer)?;
        BorshBloom(self.0.logs_bloom).serialize(writer)?;
        BorshU256(self.0.difficulty).serialize(writer)?;
        self.0.number.serialize(writer)?;
        self.0.gas_limit.serialize(writer)?;
        self.0.gas_used.serialize(writer)?;
        self.0.timestamp.serialize(writer)?;
        BorshB256(self.0.mix_hash.clone()).serialize(writer)?;
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

impl BorshDeserialize for BorshHeader {
    fn deserialize_reader<R: Read>(reader: &mut R) -> std::io::Result<Self> {
        let parent_hash = BorshB256::deserialize_reader(reader)?;
        let ommers_hash = BorshB256::deserialize_reader(reader)?;
        let beneficiary = BorshAddress::deserialize_reader(reader)?;
        let state_root = BorshB256::deserialize_reader(reader)?;
        let transactions_root = BorshB256::deserialize_reader(reader)?;
        let receipts_root = BorshB256::deserialize_reader(reader)?;
        let withdrawals_root: Option<BorshB256> = Option::deserialize_reader(reader)?;
        let bloom = BorshBloom::deserialize_reader(reader)?;
        let difficulty = BorshU256::deserialize_reader(reader)?;
        let number: u64 = BorshDeserialize::deserialize_reader(reader)?;
        let gas_limit: u64 = BorshDeserialize::deserialize_reader(reader)?;
        let gas_used: u64 = BorshDeserialize::deserialize_reader(reader)?;
        let timestamp: u64 = BorshDeserialize::deserialize_reader(reader)?;
        let mix_hash = BorshB256::deserialize_reader(reader)?;
        let nonce: u64 = BorshDeserialize::deserialize_reader(reader)?;
        let base_fee_per_gas: Option<u64> = Option::deserialize_reader(reader)?;
        let blob_gas_used: Option<u64> = Option::deserialize_reader(reader)?;
        let excess_blob_gas: Option<u64> = Option::deserialize_reader(reader)?;
        let parent_beacon_block_root: Option<BorshB256> = Option::deserialize_reader(reader)?;
        let requests_root: Option<BorshB256> = Option::deserialize_reader(reader)?;
        let extra_data = Vec::<u8>::deserialize_reader(reader)?;

        let header = Header {
            parent_hash: parent_hash.0,
            ommers_hash: ommers_hash.0,
            beneficiary: beneficiary.0,
            state_root: state_root.0,
            transactions_root: transactions_root.0,
            receipts_root: receipts_root.0,
            withdrawals_root: withdrawals_root.map(|i| i.0),
            logs_bloom: bloom.0,
            difficulty: difficulty.0,
            number,
            gas_limit,
            gas_used,
            timestamp,
            mix_hash: mix_hash.0,
            nonce,
            base_fee_per_gas,
            blob_gas_used,
            excess_blob_gas,
            parent_beacon_block_root: parent_beacon_block_root.map(|i| i.0),
            requests_root: requests_root.map(|i| i.0),
            extra_data: Bytes::from(extra_data),
        };

        Ok(BorshHeader(header))
    }
}

impl BorshSerialize for BorshSealedHeader {
    fn serialize<W: Write>(&self, writer: &mut W) -> std::io::Result<()> {
        BorshB256(self.0.hash()).serialize(writer)?;
        BorshHeader(self.0.header().clone()).serialize(writer)?;

        Ok(())
    }
}

impl BorshDeserialize for BorshSealedHeader {
    fn deserialize_reader<R: Read>(reader: &mut R) -> std::io::Result<Self> {
        let hash = BorshB256::deserialize_reader(reader)?;
        let header = BorshHeader::deserialize_reader(reader)?;

        Ok(BorshSealedHeader(SealedHeader::new(header.0, hash.0)))
    }
}

#[cfg(test)]
mod header_tests {
    use crate::header::BorshSealedHeader;
    use reth::primitives::SealedHeader;

    #[test]
    pub fn test_sealed_header() {
        let block = SealedHeader::default();
        let borsh_block = BorshSealedHeader(block.clone());
        let to_borsh = borsh::to_vec(&borsh_block).unwrap();
        let from_borsh: BorshSealedHeader = borsh::from_slice(to_borsh.as_slice()).unwrap();
        assert_eq!(block, from_borsh.0);
    }
}
