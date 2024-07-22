use std::io::{Read, Write};
use borsh::{BorshDeserialize, BorshSerialize};
use reth::primitives::{Request, Requests, SealedBlock, Withdrawal, Withdrawals};
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
            withdrawals: withdrawal.map(|i| {
                let original_withdrawals: Vec<Withdrawal> = i.into_iter().map(|e| e.0).collect();
                original_withdrawals
            }).map(|i| Withdrawals::new(i)),
            requests: requests.map(|i| {
                let original_reqs: Vec<Request> = i.into_iter().map(|e| e.0).collect();
                original_reqs
            }).map(|i| Requests(i))
        };

        Ok(BorshSealedBlock(sealed_block))
    }
}