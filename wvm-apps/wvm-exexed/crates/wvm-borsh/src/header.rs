use crate::{
    address::BorshAddress,
    b256::{BorshB256, BorshU256},
    bloom::BorshBloom,
};
use alloy_primitives::{Bytes, B64};
use borsh::{BorshDeserialize, BorshSerialize};
use std::io::{Read, Write};
use wvm_tx::wvm::{
    v1::{header::V1WvmHeader, V1WvmSealedHeader},
    MagicIdentifier, WvmHeader, WvmSealedHeader,
};

pub struct BorshHeader(pub WvmHeader);
pub struct BorshSealedHeader(pub WvmSealedHeader);

impl BorshSerialize for BorshHeader {
    fn serialize<W: Write>(&self, writer: &mut W) -> std::io::Result<()> {
        self.0.magic_identifier().serialize(writer)?;

        match &self.0 {
            WvmHeader::V1(header) => {
                BorshB256(header.parent_hash).serialize(writer)?;
                BorshB256(header.ommers_hash).serialize(writer)?;
                BorshAddress(header.beneficiary).serialize(writer)?;
                BorshB256(header.state_root).serialize(writer)?;
                BorshB256(header.transactions_root).serialize(writer)?;
                BorshB256(header.receipts_root).serialize(writer)?;
                header.withdrawals_root.map(|v| BorshB256(v)).serialize(writer)?;
                BorshBloom(header.logs_bloom).serialize(writer)?;
                BorshU256(header.difficulty).serialize(writer)?;
                header.number.serialize(writer)?;
                header.gas_limit.serialize(writer)?;
                header.gas_used.serialize(writer)?;
                header.timestamp.serialize(writer)?;
                BorshB256(header.mix_hash.clone()).serialize(writer)?;
                header.nonce.serialize(writer)?;
                header.base_fee_per_gas.serialize(writer)?;
                header.blob_gas_used.serialize(writer)?;
                header.excess_blob_gas.serialize(writer)?;
                header.parent_beacon_block_root.map(|v| BorshB256(v)).serialize(writer)?;
                header.requests_hash.map(|v| BorshB256(v)).serialize(writer)?;
                header.extra_data.0.serialize(writer)?;
            }
        }

        Ok(())
    }
}

impl BorshDeserialize for BorshHeader {
    fn deserialize_reader<R: Read>(reader: &mut R) -> std::io::Result<Self> {
        let magic_identifier: u8 = u8::deserialize_reader(reader)?;

        match magic_identifier {
            0u8 => {
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
                let parent_beacon_block_root: Option<BorshB256> =
                    Option::deserialize_reader(reader)?;
                let requests_hash: Option<BorshB256> = Option::deserialize_reader(reader)?;
                let extra_data = Vec::<u8>::deserialize_reader(reader)?;

                let header = V1WvmHeader {
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
                    nonce: B64::from(nonce),
                    base_fee_per_gas,
                    blob_gas_used,
                    excess_blob_gas,
                    parent_beacon_block_root: parent_beacon_block_root.map(|i| i.0),
                    requests_hash: requests_hash.map(|i| i.0),
                    extra_data: Bytes::from(extra_data),
                };

                Ok(BorshHeader(WvmHeader::V1(header)))
            }
            _ => Err(std::io::Error::new(std::io::ErrorKind::Other, "Invalid Magic Identifier")),
        }
    }
}

impl BorshSerialize for BorshSealedHeader {
    fn serialize<W: Write>(&self, writer: &mut W) -> std::io::Result<()> {
        self.0.magic_identifier().serialize(writer)?;

        match &self.0 {
            WvmSealedHeader::V1(sealed_header) => {
                BorshB256(sealed_header.hash).serialize(writer)?;
                BorshHeader(WvmHeader::V1(sealed_header.header.clone())).serialize(writer)?;
            }
        }

        Ok(())
    }
}

impl BorshDeserialize for BorshSealedHeader {
    fn deserialize_reader<R: Read>(reader: &mut R) -> std::io::Result<Self> {
        let magic_identifier: u8 = u8::deserialize_reader(reader)?;

        match magic_identifier {
            0u8 => {
                let hash = BorshB256::deserialize_reader(reader)?;
                let header = BorshHeader::deserialize_reader(reader)?;
                let v1_header: V1WvmHeader = header.0.as_v1().unwrap().clone();

                Ok(BorshSealedHeader(WvmSealedHeader::V1(V1WvmSealedHeader {
                    hash: hash.0,
                    header: v1_header,
                })))
            }
            _ => Err(std::io::Error::new(std::io::ErrorKind::Other, "Invalid Magic Identifier")),
        }
    }
}

#[cfg(test)]
mod header_tests {
    use crate::header::BorshSealedHeader;
    use reth::primitives::SealedHeader;
    use wvm_tx::wvm::{v1::V1WvmSealedHeader, WvmSealedHeader};

    #[test]
    pub fn test_sealed_header() {
        let block: SealedHeader = SealedHeader::default();
        let borsh_block =
            BorshSealedHeader(WvmSealedHeader::V1(V1WvmSealedHeader::from(block.clone())));
        let to_borsh = borsh::to_vec(&borsh_block).unwrap();
        let from_borsh: BorshSealedHeader = borsh::from_slice(to_borsh.as_slice()).unwrap();
        assert_eq!(block, from_borsh.0.as_v1().unwrap().clone().into());
    }
}
