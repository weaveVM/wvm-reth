pub mod address;
pub mod b256;
pub mod block;
pub mod bloom;
pub mod header;

pub mod signature;
pub mod transaction;
pub mod withdrawal;

#[cfg(test)]
mod tests {
    use crate::block::{BorshSealedBlock, BorshSealedBlockWithSenders};
    use reth::primitives::{SealedBlock, Withdrawals};
    use reth_primitives::BlockBody;
    use std::{fs::File, io::Read};
    use wvm_tx::wvm::{v1::V1WvmSealedBlock, WvmSealedBlock};

    #[test]
    fn test_borsh_block() {
        let withdrawals = Withdrawals::new(vec![Default::default()]);

        let block = SealedBlock {
            header: Default::default(),
            body: BlockBody {
                transactions: vec![Default::default()],
                ommers: Default::default(),
                withdrawals: Some(withdrawals),
            },
        };

        let borsh_block =
            BorshSealedBlock(WvmSealedBlock::V1(V1WvmSealedBlock::from(block.clone())));

        let serde_json_serialize = serde_json::to_vec(&block).unwrap();

        let borsh_serialize = borsh::to_vec(&borsh_block).unwrap();

        assert_eq!(serde_json_serialize.len(), 1828);
        assert_eq!(borsh_serialize.len(), 907);
    }

    #[test]
    fn test_borsh_deserialize_block() {
        let withdrawals = Withdrawals::new(vec![Default::default()]);

        let block = SealedBlock {
            header: Default::default(),
            body: BlockBody {
                transactions: vec![Default::default()],
                ommers: Default::default(),
                withdrawals: Some(withdrawals),
            },
        };

        let borsh_block =
            BorshSealedBlock(WvmSealedBlock::V1(V1WvmSealedBlock::from(block.clone())));
        let borsh_serialize = borsh::to_vec(&borsh_block).unwrap();
        let _: BorshSealedBlock = borsh::from_slice(borsh_serialize.as_slice()).unwrap();
    }
}
