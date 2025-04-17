use crate::wvm::v1::{header::V1WvmHeader, transaction::V1WvmTransactionSigned};
use alloy_eips::eip4895::Withdrawals;
use alloy_primitives::{Address, BlockHash};
use std::ptr::hash;

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

impl Into<SealedBlock> for V1WvmSealedBlock {
    fn into(self) -> SealedBlock {
        let body = V1WvmBlockBody {
            transactions: self.body.transactions,
            ommers: self.body.ommers,
            withdrawals: self.body.withdrawals,
        };

        SealedBlock::new(self.header.into(), body.into())
    }
}

impl From<SealedHeader> for V1WvmSealedHeader {
    fn from(value: SealedHeader) -> Self {
        V1WvmSealedHeader { hash: value.hash(), header: V1WvmHeader::from(value.header().clone()) }
    }
}

impl From<SealedBlock> for V1WvmSealedBlock {
    fn from(value: SealedBlock) -> Self {
        V1WvmSealedBlock {
            header: V1WvmSealedHeader::from(value.header()),
            body: V1WvmBlockBody::from(value.body()),
        }
    }
}

impl Into<SealedBlockWithSenders> for V1WvmSealedBlockWithSenders {
    fn into(self) -> SealedBlockWithSenders {
        SealedBlockWithSenders::new(self.block.clone().into(), self.senders, self.block.header.hash)
    }
}

impl From<SealedBlockWithSenders> for V1WvmSealedBlockWithSenders {
    fn from(value: SealedBlockWithSenders) -> Self {
        V1WvmSealedBlockWithSenders {
            block: value.sealed_block().into(),
            senders: value.senders().into_vec(),
        }
    }
}
