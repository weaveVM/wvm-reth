use std::io::{Read, Write};
use borsh::{BorshDeserialize, BorshSerialize};
use reth::primitives::Request;

pub struct BorshRequest(pub Request);
impl BorshSerialize for BorshRequest {
    fn serialize<W: Write>(&self, writer: &mut W) -> std::io::Result<()> {
        let buff = serde_json::to_vec(&self.0).unwrap();
        buff.serialize(writer)?;
        Ok(())
    }
}

impl BorshDeserialize for BorshRequest {
    fn deserialize_reader<R: Read>(reader: &mut R) -> std::io::Result<Self> {
        let val = Vec::<u8>::deserialize_reader(reader)?;
        Ok(BorshRequest(serde_json::from_slice(val.as_slice())?))
    }
}