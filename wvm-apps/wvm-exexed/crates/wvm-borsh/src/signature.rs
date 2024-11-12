use alloy_primitives::{Parity, Signature, U256};
use borsh::{BorshDeserialize, BorshSerialize};
use std::io::{Error, ErrorKind, Read, Write};

pub struct BorshSignature(pub Signature);

pub fn to_signature(bytes: &[u8]) -> std::io::Result<Signature> {
    if bytes.len() != 65 {
        return Err(Error::from(ErrorKind::UnexpectedEof));
    }

    let signature = Signature::try_from(bytes).unwrap();

    Ok(signature)
}

impl BorshSerialize for BorshSignature {
    fn serialize<W: Write>(&self, writer: &mut W) -> std::io::Result<()> {
        self.0.as_bytes().to_vec().serialize(writer)
    }
}

impl BorshDeserialize for BorshSignature {
    fn deserialize_reader<R: Read>(reader: &mut R) -> std::io::Result<Self> {
        let sig_vec = Vec::<u8>::deserialize_reader(reader)?;
        let sig = to_signature(sig_vec.as_slice()).unwrap();
        Ok(BorshSignature(sig))
    }
}

#[cfg(test)]
mod signature_tests {
    use alloy_primitives::Parity;
    use crate::signature::BorshSignature;
    use reth::primitives::Signature;

    #[test]
    pub fn test_sealed_header() {
        let data = Signature::test_signature();
        let b = data.as_bytes();
        println!("{:?}", b);
        let borsh_data = BorshSignature(data.clone());
        let to_borsh = borsh::to_vec(&borsh_data).unwrap();
        let from_borsh: BorshSignature = borsh::from_slice(to_borsh.as_slice()).unwrap();
        let from_bytes = from_borsh.0.as_bytes();
        println!("{:?}", from_bytes);
        assert_eq!(b, from_bytes);
        let parity = Parity::try_from(from_bytes[64] as u64).unwrap();
        assert_eq!(Parity::NonEip155(false), parity)

    }
}
