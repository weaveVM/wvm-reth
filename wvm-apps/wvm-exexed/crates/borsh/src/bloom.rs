use std::io::{Read, Write};
use borsh::{BorshDeserialize, BorshSerialize};
use reth::primitives::Bloom;

pub struct BorshBloom(pub Bloom);

impl BorshSerialize for BorshBloom {
    fn serialize<W: Write>(&self, writer: &mut W) -> std::io::Result<()> {
        let vc = self.0.data().to_vec();
        vc.serialize(writer)
    }
}

impl BorshDeserialize for BorshBloom {
    fn deserialize_reader<R: Read>(reader: &mut R) -> std::io::Result<Self> {
        let bytes = Vec::<u8>::deserialize_reader(reader)?;
        let bloom = Bloom::from_slice(bytes.as_slice());
        Ok(BorshBloom(bloom))
    }
}

#[cfg(test)]
mod tests {
    use reth::primitives::Bloom;
    use crate::bloom::BorshBloom;

    #[test]
    pub fn test_bloom_ser_der() {
        let bloom = Bloom::random();
        let borsh_bloom = BorshBloom(bloom);
        let ser = borsh::to_vec(&borsh_bloom).unwrap();
        let der: BorshBloom = borsh::from_slice(ser.as_slice()).unwrap();
        assert_eq!(bloom, der.0);
    }

}

