use borsh::{BorshDeserialize, BorshSerialize};
use reth::primitives::Request;
use std::io::{Read, Write};

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

#[cfg(test)]
mod request_tests {
    use crate::request::BorshRequest;
    use alloy_eips::eip6110::DepositRequest as DepReq;
    use reth::primitives::Request;

    #[test]
    pub fn test_sealed_header() {
        let data = Request::DepositRequest(DepReq::default());
        let borsh_data = BorshRequest(data.clone());
        let to_borsh = borsh::to_vec(&borsh_data).unwrap();
        let from_borsh: BorshRequest = borsh::from_slice(to_borsh.as_slice()).unwrap();
        assert_eq!(data, from_borsh.0);
    }
}
