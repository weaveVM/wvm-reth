use crate::{
    address::BorshAddress,
    header::{BorshHeader, BorshSealedHeader},
    transaction::BorshTransactionSigned,
    withdrawal::BorshWithdrawal,
};
use borsh::{BorshDeserialize, BorshSerialize};

use std::io::{Read, Write};
use wvm_tx::wvm::{
    v1::{V1WvmBlockBody, V1WvmSealedBlock, V1WvmSealedBlockWithSenders},
    MagicIdentifier, WvmHeader, WvmSealedBlock, WvmSealedBlockWithSenders, WvmSealedHeader,
    WvmTransactionSigned,
};

pub struct BorshSealedBlock(pub WvmSealedBlock);
pub struct BorshSealedBlockWithSenders(pub WvmSealedBlockWithSenders);

impl BorshSerialize for BorshSealedBlock {
    fn serialize<W: Write>(&self, writer: &mut W) -> std::io::Result<()> {
        self.0.magic_identifier().serialize(writer)?;

        match &self.0 {
            WvmSealedBlock::V1(sealed_block) => {
                let borsh_sealed_header =
                    BorshSealedHeader(WvmSealedHeader::V1(sealed_block.header.clone()));
                let borsh_transactions: Vec<BorshTransactionSigned> = sealed_block
                    .body
                    .clone()
                    .transactions
                    .into_iter()
                    .map(|e| BorshTransactionSigned(WvmTransactionSigned::V1(e)))
                    .collect();
                let borsh_ommers: Vec<BorshHeader> = sealed_block
                    .body
                    .clone()
                    .ommers
                    .clone()
                    .into_iter()
                    .map(|e| BorshHeader(WvmHeader::V1(e)))
                    .collect();
                let withdrawal = sealed_block.body.clone().withdrawals.clone().map(|i| {
                    let withdrawals: Vec<BorshWithdrawal> =
                        i.into_inner().into_iter().map(BorshWithdrawal).collect();

                    withdrawals
                });

                borsh_sealed_header.serialize(writer)?;
                borsh_transactions.serialize(writer)?;
                borsh_ommers.serialize(writer)?;
                withdrawal.serialize(writer)?;
            }
        }

        Ok(())
    }
}

impl BorshDeserialize for BorshSealedBlock {
    fn deserialize_reader<R: Read>(reader: &mut R) -> std::io::Result<Self> {
        let magic_identifier: u8 = u8::deserialize_reader(reader)?;

        match magic_identifier {
            0u8 => {
                let sealed_header = BorshSealedHeader::deserialize_reader(reader)?;
                let borsh_transactions = Vec::<BorshTransactionSigned>::deserialize_reader(reader)?;
                let borsh_ommers = Vec::<BorshHeader>::deserialize_reader(reader)?;
                let withdrawal = Option::<Vec<BorshWithdrawal>>::deserialize_reader(reader)?;

                Ok(BorshSealedBlock(WvmSealedBlock::V1(V1WvmSealedBlock {
                    header: sealed_header.0.as_v1().unwrap().clone(),
                    body: V1WvmBlockBody {
                        transactions: borsh_transactions
                            .into_iter()
                            .map(|i| i.0.as_v1().unwrap().clone())
                            .collect(),
                        ommers: borsh_ommers
                            .into_iter()
                            .map(|i| i.0.as_v1().unwrap().clone())
                            .collect(),
                        withdrawals: withdrawal
                            .map(|i| {
                                let original_withdrawals: Vec<alloy_eips::eip4895::Withdrawal> =
                                    i.into_iter().map(|e| e.0).collect();
                                original_withdrawals
                            })
                            .map(|i| reth_primitives::Withdrawals::new(i)),
                    },
                })))
            }
            _ => Err(std::io::Error::new(std::io::ErrorKind::Other, "Invalid Magic Identifier")),
        }
    }
}

impl BorshSerialize for BorshSealedBlockWithSenders {
    fn serialize<W: Write>(&self, writer: &mut W) -> std::io::Result<()> {
        self.0.magic_identifier().serialize(writer)?;

        match &self.0 {
            WvmSealedBlockWithSenders::V1(sealed_block_with_senders) => {
                let borsh_sealed_block =
                    BorshSealedBlock(WvmSealedBlock::V1(sealed_block_with_senders.block.clone()));
                let senders: Vec<BorshAddress> = sealed_block_with_senders
                    .senders
                    .clone()
                    .into_iter()
                    .map(BorshAddress)
                    .collect();

                borsh_sealed_block.serialize(writer)?;
                senders.serialize(writer)?;
            }
        }

        Ok(())
    }
}

impl BorshDeserialize for BorshSealedBlockWithSenders {
    fn deserialize_reader<R: Read>(reader: &mut R) -> std::io::Result<Self> {
        let magic_identifier: u8 = u8::deserialize_reader(reader)?;

        match magic_identifier {
            0u8 => {
                let sealed_block: BorshSealedBlock = BorshDeserialize::deserialize_reader(reader)?;
                let block: V1WvmSealedBlock = sealed_block.0.as_v1().unwrap().clone();
                let senders: Vec<BorshAddress> = BorshDeserialize::deserialize_reader(reader)?;

                Ok(BorshSealedBlockWithSenders(WvmSealedBlockWithSenders::V1(
                    V1WvmSealedBlockWithSenders {
                        block,
                        senders: senders.into_iter().map(|i| i.0).collect(),
                    },
                )))
            }
            _ => Err(std::io::Error::new(std::io::ErrorKind::Other, "Invalid Magic Identifier")),
        }
    }
}

#[cfg(test)]
mod block_tests {
    use crate::block::{BorshSealedBlock, BorshSealedBlockWithSenders};
    use reth::primitives::{SealedBlock, SealedBlockWithSenders};
    use wvm_tx::wvm::{
        v1::{V1WvmSealedBlock, V1WvmSealedBlockWithSenders},
        WvmSealedBlockWithSenders,
    };

    #[test]
    pub fn test_sealed_block() {
        let block = SealedBlock::default();
        let block_clone = SealedBlock::clone(&block);
        let wvm_block = V1WvmSealedBlock::from(block);
        let borsh_block = BorshSealedBlock(wvm_tx::wvm::WvmSealedBlock::V1(wvm_block));
        let to_borsh = borsh::to_vec(&borsh_block).unwrap();
        let from_borsh: BorshSealedBlock = borsh::from_slice(to_borsh.as_slice()).unwrap();
        assert_eq!(block_clone, from_borsh.0.as_v1().unwrap().clone().into());
    }

    #[test]
    pub fn test_sealed_block_w_senders() {
        let block = SealedBlockWithSenders::default();
        let block_clone = SealedBlockWithSenders::clone(&block);
        let wvm_block = V1WvmSealedBlockWithSenders::from(block);
        let borsh_block = BorshSealedBlockWithSenders(WvmSealedBlockWithSenders::V1(wvm_block));
        let to_borsh = borsh::to_vec(&borsh_block).unwrap();
        let from_borsh: BorshSealedBlockWithSenders =
            borsh::from_slice(to_borsh.as_slice()).unwrap();
        assert_eq!(block_clone, from_borsh.0.as_v1().unwrap().clone().into());
    }
}
