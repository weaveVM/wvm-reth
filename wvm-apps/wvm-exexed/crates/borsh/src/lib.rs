pub mod b256;
pub mod block;
pub mod bloom;
pub mod header;
pub mod request;
pub mod signature;
pub mod transaction;
pub mod withdrawal;

#[cfg(test)]
mod tests {
    use crate::block::BorshSealedBlock;
    use reth::primitives::{SealedBlock, Withdrawals};

    #[test]
    fn test_borsh_block() {
        let withdrawals = Withdrawals::new(vec![Default::default()]);

        let block = SealedBlock {
            header: Default::default(),
            body: vec![Default::default()],
            ommers: Default::default(),
            withdrawals: Some(withdrawals),
            requests: None,
        };

        let borsh_block = BorshSealedBlock(block.clone());

        let serde_json_serialize = serde_json::to_vec(&block).unwrap();

        let borsh_serialize = borsh::to_vec(&borsh_block).unwrap();

        assert_eq!(serde_json_serialize.len(), 1847);
        assert_eq!(borsh_serialize.len(), 920);
    }

    #[test]
    fn test_borsh_deserialize_block() {
        let withdrawals = Withdrawals::new(vec![Default::default()]);

        let block = SealedBlock {
            header: Default::default(),
            body: vec![Default::default()],
            ommers: Default::default(),
            withdrawals: Some(withdrawals),
            requests: None,
        };

        let borsh_block = BorshSealedBlock(block.clone());
        let borsh_serialize = borsh::to_vec(&borsh_block).unwrap();
        let _: BorshSealedBlock = borsh::from_slice(borsh_serialize.as_slice()).unwrap();
    }
}
