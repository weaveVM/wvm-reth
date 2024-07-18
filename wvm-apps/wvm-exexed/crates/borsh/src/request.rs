use std::io::Write;
use borsh::BorshSerialize;
use reth::primitives::Request;

pub struct BorshRequest(pub Request);
impl BorshSerialize for BorshRequest {
    fn serialize<W: Write>(&self, writer: &mut W) -> std::io::Result<()> {
        let buff = serde_json::to_vec(&self.0).unwrap();
        buff.serialize(writer)?;
        Ok(())
    }
}