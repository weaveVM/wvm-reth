use std::io::{Error, ErrorKind, Read, Write};
use borsh::{BorshDeserialize, BorshSerialize};
use reth::primitives::{Signature, U256};

pub struct BorshSignature(pub Signature);

pub fn to_signature(bytes: &[u8]) -> std::io::Result<Signature> {
    if bytes.len() != 65 {
        return Err(Error::from(ErrorKind::UnexpectedEof));
    }

    let mut r_bytes = [0u8; 32];
    let mut s_bytes = [0u8; 32];

    r_bytes.copy_from_slice(&bytes[..32]);
    s_bytes.copy_from_slice(&bytes[32..64]);

    let r = U256::from_be_bytes(r_bytes);
    let s = U256::from_be_bytes(s_bytes);
    let odd_y_parity = bytes[64] - 27;

    let signature = Signature {
        r,
        s,
        odd_y_parity: odd_y_parity != 0,
    };

    Ok(signature)
}

impl BorshSerialize for BorshSignature {
    fn serialize<W: Write>(&self, writer: &mut W) -> std::io::Result<()> {
        self.0.to_bytes().to_vec().serialize(writer)
    }
}

impl BorshDeserialize for BorshSignature {
    fn deserialize_reader<R: Read>(reader: &mut R) -> std::io::Result<Self> {
        let sig_vec = Vec::<u8>::deserialize_reader(reader)?;
        let sig = to_signature(sig_vec.as_slice()).unwrap();
        Ok(BorshSignature(sig))
    }
}