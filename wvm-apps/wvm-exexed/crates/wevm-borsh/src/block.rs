use crate::address::BorshAddress;
use crate::header::{BorshHeader, BorshSealedHeader};
use crate::request::BorshRequest;
use crate::transaction::BorshTransactionSigned;
use crate::withdrawal::BorshWithdrawal;
use borsh::{BorshDeserialize, BorshSerialize};
use reth::primitives::{
    Request, Requests, SealedBlock, SealedBlockWithSenders, Withdrawal, Withdrawals,
};
use std::io::{Read, Write};

pub struct BorshSealedBlock(pub SealedBlock);
pub struct BorshSealedBlockWithSenders(pub SealedBlockWithSenders);

impl BorshSerialize for BorshSealedBlock {
    fn serialize<W: Write>(&self, writer: &mut W) -> std::io::Result<()> {
        let borsh_sealed_header = BorshSealedHeader(self.0.header.clone());
        let borsh_transactions: Vec<BorshTransactionSigned> =
            self.0.body.clone().into_iter().map(BorshTransactionSigned).collect();
        let borsh_ommers: Vec<BorshHeader> =
            self.0.ommers.clone().into_iter().map(BorshHeader).collect();
        let withdrawal = self.0.withdrawals.clone().map(|i| {
            let withdrawals: Vec<BorshWithdrawal> =
                i.into_inner().into_iter().map(BorshWithdrawal).collect();

            withdrawals
        });
        let requests = self.0.requests.clone().map(|i| {
            let reqs: Vec<BorshRequest> = i.0.into_iter().map(BorshRequest).collect();
            reqs
        });

        borsh_sealed_header.serialize(writer)?;
        borsh_transactions.serialize(writer)?;
        borsh_ommers.serialize(writer)?;
        withdrawal.serialize(writer)?;
        requests.serialize(writer)?;

        Ok(())
    }
}

impl BorshDeserialize for BorshSealedBlock {
    fn deserialize_reader<R: Read>(reader: &mut R) -> std::io::Result<Self> {
        let sealed_header = BorshSealedHeader::deserialize_reader(reader)?;
        let borsh_transactions = Vec::<BorshTransactionSigned>::deserialize_reader(reader)?;
        let borsh_ommers = Vec::<BorshHeader>::deserialize_reader(reader)?;
        let withdrawal = Option::<Vec<BorshWithdrawal>>::deserialize_reader(reader)?;

        let requests = Option::<Vec<BorshRequest>>::deserialize_reader(reader)?;

        let sealed_block = SealedBlock {
            header: sealed_header.0,
            body: borsh_transactions.into_iter().map(|i| i.0).collect(),
            ommers: borsh_ommers.into_iter().map(|i| i.0).collect(),
            withdrawals: withdrawal
                .map(|i| {
                    let original_withdrawals: Vec<Withdrawal> =
                        i.into_iter().map(|e| e.0).collect();
                    original_withdrawals
                })
                .map(|i| Withdrawals::new(i)),
            requests: requests
                .map(|i| {
                    let original_reqs: Vec<Request> = i.into_iter().map(|e| e.0).collect();
                    original_reqs
                })
                .map(|i| Requests(i)),
        };

        Ok(BorshSealedBlock(sealed_block))
    }
}

impl BorshSerialize for BorshSealedBlockWithSenders {
    fn serialize<W: Write>(&self, writer: &mut W) -> std::io::Result<()> {
        let borsh_sealed_block = BorshSealedBlock(self.0.block.clone());
        let senders: Vec<BorshAddress> =
            self.0.senders.clone().into_iter().map(BorshAddress).collect();

        borsh_sealed_block.serialize(writer)?;
        senders.serialize(writer)?;

        Ok(())
    }
}

impl BorshDeserialize for BorshSealedBlockWithSenders {
    fn deserialize_reader<R: Read>(reader: &mut R) -> std::io::Result<Self> {
        let sealed_block: BorshSealedBlock = BorshDeserialize::deserialize_reader(reader)?;
        let senders: Vec<BorshAddress> = BorshDeserialize::deserialize_reader(reader)?;

        let sealed_block_w_senders = SealedBlockWithSenders {
            block: sealed_block.0,
            senders: senders.into_iter().map(|i| i.0).collect(),
        };

        Ok(BorshSealedBlockWithSenders(sealed_block_w_senders))
    }
}

#[cfg(test)]
mod block_tests {
    use crate::block::{BorshSealedBlock, BorshSealedBlockWithSenders};
    use reth::primitives::{SealedBlock, SealedBlockWithSenders};

    #[test]
    pub fn test_sealed_block() {
        let block = SealedBlock::default();
        let borsh_block = BorshSealedBlock(block.clone());
        let to_borsh = borsh::to_vec(&borsh_block).unwrap();
        let from_borsh: BorshSealedBlock = borsh::from_slice(to_borsh.as_slice()).unwrap();
        assert_eq!(block, from_borsh.0);
    }

    #[test]
    pub fn test_sealed_block_w_senders() {
        let block = SealedBlockWithSenders::default();
        let borsh_block = BorshSealedBlockWithSenders(block.clone());
        let to_borsh = borsh::to_vec(&borsh_block).unwrap();
        let from_borsh: BorshSealedBlockWithSenders =
            borsh::from_slice(to_borsh.as_slice()).unwrap();
        assert_eq!(block, from_borsh.0);
    }
}
