use brotlic::{CompressorWriter, DecompressorReader};
use std::io::{Read, Write};

pub fn to_brotli(data: Vec<u8>) -> Vec<u8> {
    let buff: Vec<u8> = vec![];
    let mut compressor = CompressorWriter::new(buff);
    compressor.write_all(data.as_slice()).unwrap();
    compressor.into_inner().unwrap()
}

pub fn from_brotli(data: Vec<u8>) -> Vec<u8> {
    // create a wrapper around BufRead that supports on the fly brotli decompression.
    let mut decompressed_reader = DecompressorReader::new(data.as_slice());
    let mut decoded_input: Vec<u8> = Vec::new();

    decompressed_reader.read_to_end(&mut decoded_input).unwrap();

    decoded_input
}

#[cfg(test)]
mod brotlic_tests {
    use crate::{from_brotli, to_brotli};
    use reth::primitives::SealedBlockWithSenders;
    use wvm_borsh::block::BorshSealedBlockWithSenders;

    #[test]
    pub fn test_brotlic_block() {
        let sealed_block_with_senders = SealedBlockWithSenders::default();
        // TODO: fix it
        let borsh_block = BorshSealedBlockWithSenders(sealed_block_with_senders);
        let borsh_vec = borsh::to_vec(&borsh_block).unwrap();
        let brotli = to_brotli(borsh_vec.clone());
        assert!(brotli.len() < borsh_vec.len());
        let unbrotli = from_brotli(brotli);
        assert_eq!(borsh_vec, unbrotli);
    }
}
