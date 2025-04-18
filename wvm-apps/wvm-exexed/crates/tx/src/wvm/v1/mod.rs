use crate::wvm::v1::{header::V1WvmHeader, transaction::V1WvmTransactionSigned};
use alloy_eips::eip4895::Withdrawals;
use alloy_primitives::{Address, BlockHash};

use reth_primitives::{BlockBody, SealedBlock, SealedBlockWithSenders, SealedHeader};
use serde::{Deserialize, Serialize};

pub mod header;
pub mod transaction;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct V1WvmSealedHeader {
    // B256
    pub hash: BlockHash,
    pub header: V1WvmHeader,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct V1WvmBlockBody {
    pub transactions: Vec<V1WvmTransactionSigned>,
    pub ommers: Vec<V1WvmHeader>,
    pub withdrawals: Option<Withdrawals>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct V1WvmSealedBlock {
    pub header: V1WvmSealedHeader,
    /// Block body.
    pub body: V1WvmBlockBody,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct V1WvmSealedBlockWithSenders {
    pub block: V1WvmSealedBlock,
    pub senders: Vec<Address>,
}

impl Into<SealedHeader> for V1WvmSealedHeader {
    fn into(self) -> SealedHeader {
        SealedHeader::new(self.header.into(), self.hash)
    }
}

impl Into<BlockBody> for V1WvmBlockBody {
    fn into(self) -> BlockBody {
        BlockBody {
            transactions: self.transactions.into_iter().map(|i| i.into()).collect(),
            ommers: self.ommers.into_iter().map(|i| i.into()).collect(),
            withdrawals: self.withdrawals,
        }
    }
}

impl From<BlockBody> for V1WvmBlockBody {
    fn from(value: BlockBody) -> Self {
        V1WvmBlockBody {
            transactions: value
                .transactions
                .into_iter()
                .map(|i| V1WvmTransactionSigned::from(i))
                .collect(),
            ommers: value.ommers.into_iter().map(|i| V1WvmHeader::from(i)).collect(),
            withdrawals: value.withdrawals,
        }
    }
}

// Add implementation for reference to BlockBody
impl From<&BlockBody> for V1WvmBlockBody {
    fn from(value: &BlockBody) -> Self {
        V1WvmBlockBody {
            transactions: value
                .transactions
                .iter()
                .map(|i| V1WvmTransactionSigned::from(i.clone()))
                .collect(),
            ommers: value.ommers.iter().map(|i| V1WvmHeader::from(i.clone())).collect(),
            withdrawals: value.withdrawals.clone(),
        }
    }
}

impl Into<SealedBlock> for V1WvmSealedBlock {
    fn into(self) -> SealedBlock {
        SealedBlock::from_sealed_parts(self.header.into(), self.body.into())
    }
}

impl From<SealedHeader> for V1WvmSealedHeader {
    fn from(value: SealedHeader) -> Self {
        V1WvmSealedHeader { hash: value.hash(), header: V1WvmHeader::from(value.header().clone()) }
    }
}

// Add implementation for reference to SealedHeader
impl From<&SealedHeader> for V1WvmSealedHeader {
    fn from(value: &SealedHeader) -> Self {
        V1WvmSealedHeader { hash: value.hash(), header: V1WvmHeader::from(value.header().clone()) }
    }
}

impl From<SealedBlock> for V1WvmSealedBlock {
    fn from(value: SealedBlock) -> Self {
        let (header, body) = value.split_sealed_header_body();
        V1WvmSealedBlock {
            header: V1WvmSealedHeader::from(header),
            body: V1WvmBlockBody::from(body),
        }
    }
}

// Add implementation for reference to SealedBlock
impl From<&SealedBlock> for V1WvmSealedBlock {
    fn from(value: &SealedBlock) -> Self {
        V1WvmSealedBlock {
            header: V1WvmSealedHeader::from(value.sealed_header()),
            body: V1WvmBlockBody::from(value.body()),
        }
    }
}

// Added reference implementation
impl From<&SealedBlockWithSenders> for V1WvmSealedBlockWithSenders {
    fn from(value: &SealedBlockWithSenders) -> Self {
        V1WvmSealedBlockWithSenders {
            block: V1WvmSealedBlock::from(value.sealed_block()),
            senders: value.senders().to_vec(),
        }
    }
}

impl From<SealedBlockWithSenders> for V1WvmSealedBlockWithSenders {
    fn from(value: SealedBlockWithSenders) -> Self {
        V1WvmSealedBlockWithSenders {
            block: V1WvmSealedBlock::from(value.sealed_block()),
            senders: value.senders().to_vec(),
        }
    }
}

impl Into<SealedBlockWithSenders> for V1WvmSealedBlockWithSenders {
    fn into(self) -> SealedBlockWithSenders {
        // Convert the block to the expected type first
        let block = alloy_consensus::Block::from(self.block.clone());
        SealedBlockWithSenders::new(block, self.senders, self.block.header.hash)
    }
}

// Add this conversion to fix the error in the previous implementation
impl From<V1WvmSealedBlock> for alloy_consensus::Block<reth_primitives::TransactionSigned> {
    fn from(value: V1WvmSealedBlock) -> Self {
        // First convert to SealedBlock
        let sealed_block: SealedBlock = value.into();
        // Then get the inner Block
        sealed_block.clone_block()
    }
}
