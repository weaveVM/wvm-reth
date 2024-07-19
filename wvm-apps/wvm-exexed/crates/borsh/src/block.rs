use std::io::Write;
use borsh::BorshSerialize;
use reth::primitives::SealedBlock;
use crate::header::{BorshHeader, BorshSealedHeader};
use crate::request::BorshRequest;
use crate::transaction::BorshTransactionSigned;
use crate::withdrawal::BorshWithdrawal;

pub struct BorshSealedBlock(pub SealedBlock);

impl BorshSerialize for BorshSealedBlock {
    fn serialize<W: Write>(&self, writer: &mut W) -> std::io::Result<()> {
        let borsh_sealed_header = BorshSealedHeader(self.0.header.clone());
        let borsh_transactions: Vec<BorshTransactionSigned> = self.0.body.clone()
            .into_iter()
            .map(BorshTransactionSigned)
            .collect();
        let borsh_ommers: Vec<BorshHeader> = self.0.ommers.clone()
            .into_iter()
            .map(BorshHeader)
            .collect();
        let withdrawal = self.0.withdrawals.clone().map(|i| {
            let withdrawals: Vec<BorshWithdrawal> = i.into_inner()
                .into_iter()
                .map(BorshWithdrawal)
                .collect();

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