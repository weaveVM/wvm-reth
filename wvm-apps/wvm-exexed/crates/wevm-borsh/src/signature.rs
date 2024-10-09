use borsh::{BorshDeserialize, BorshSerialize};
use std::io::{Error, ErrorKind, Read, Write};
use alloy_primitives::{Parity, U256};
use alloy_primitives::Signature;



pub struct BorshSignature(pub Signature);

pub fn to_signature(bytes: &[u8]) -> std::io::Result<Signature> {
    if bytes.len() != 65 {
        return Err(Error::from(ErrorKind::UnexpectedEof))
    }

    let mut r_bytes = [0u8; 32];
    let mut s_bytes = [0u8; 32];

    r_bytes.copy_from_slice(&bytes[..32]);
    s_bytes.copy_from_slice(&bytes[32..64]);

    let r = U256::from_be_bytes(r_bytes);
    let s = U256::from_be_bytes(s_bytes);

    let odd_y_parity = bytes[64] - 27;
    let signature=  Signature::new(
        r,
        s,
        Parity::Parity(odd_y_parity != 0),
    );

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
    use crate::signature::BorshSignature;
    use reth::primitives::Signature;

    #[test]
    pub fn test_sealed_header() {
        let data =  Signature::test_signature();
        let borsh_data = BorshSignature(data.clone());
        let to_borsh = borsh::to_vec(&borsh_data).unwrap();
        let from_borsh: BorshSignature = borsh::from_slice(to_borsh.as_slice()).unwrap();
        assert_eq!(data, from_borsh.0);
    }
}
