//! Implements the `GetPooledTransactions` and `PooledTransactions` message types.

use alloy_primitives::B256;
use alloy_rlp::{RlpDecodableWrapper, RlpEncodableWrapper};
use derive_more::{Constructor, Deref, IntoIterator};
use reth_codecs_derive::add_arbitrary_tests;
use reth_primitives::{
    transaction::TransactionConversionError, PooledTransactionsElement, TransactionSigned,
};

/// A list of transaction hashes that the peer would like transaction bodies for.
#[derive(Clone, Debug, PartialEq, Eq, RlpEncodableWrapper, RlpDecodableWrapper, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(any(test, feature = "arbitrary"), derive(arbitrary::Arbitrary))]
#[add_arbitrary_tests(rlp)]
pub struct GetPooledTransactions(
    /// The transaction hashes to request transaction bodies for.
    pub Vec<B256>,
);

impl<T> From<Vec<T>> for GetPooledTransactions
where
    T: Into<B256>,
{
    fn from(hashes: Vec<T>) -> Self {
        Self(hashes.into_iter().map(|h| h.into()).collect())
    }
}

/// The response to [`GetPooledTransactions`], containing the transaction bodies associated with
/// the requested hashes.
///
/// This response may not contain all bodies requested, but the bodies should be in the same order
/// as the request's hashes. Hashes may be skipped, and the client should ensure that each body
/// corresponds to a requested hash. Hashes may need to be re-requested if the bodies are not
/// included in the response.
// #[derive_arbitrary(rlp, 10)]
#[derive(
    Clone,
    Debug,
    PartialEq,
    Eq,
    RlpEncodableWrapper,
    RlpDecodableWrapper,
    Default,
    IntoIterator,
    Deref,
    Constructor,
)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PooledTransactions(
    /// The transaction bodies, each of which should correspond to a requested hash.
    pub Vec<PooledTransactionsElement>,
);

impl PooledTransactions {
    /// Returns an iterator over the transaction hashes in this response.
    pub fn hashes(&self) -> impl Iterator<Item = &B256> + '_ {
        self.0.iter().map(|tx| tx.hash())
    }
}

impl TryFrom<Vec<TransactionSigned>> for PooledTransactions {
    type Error = TransactionConversionError;

    fn try_from(txs: Vec<TransactionSigned>) -> Result<Self, Self::Error> {
        txs.into_iter().map(PooledTransactionsElement::try_from).collect()
    }
}

impl FromIterator<PooledTransactionsElement> for PooledTransactions {
    fn from_iter<I: IntoIterator<Item = PooledTransactionsElement>>(iter: I) -> Self {
        Self(iter.into_iter().collect())
    }
}

#[cfg(test)]
mod tests {
    use crate::{message::RequestPair, GetPooledTransactions, PooledTransactions};
    use alloy_consensus::{TxEip1559, TxLegacy};
    use alloy_primitives::{hex, Parity, Signature, TxKind, U256};
    use alloy_rlp::{Decodable, Encodable};
    use reth_chainspec::MIN_TRANSACTION_GAS;
    use reth_primitives::{PooledTransactionsElement, Transaction, TransactionSigned};
    use std::str::FromStr;

    #[test]
    // Test vector from: https://eips.ethereum.org/EIPS/eip-2481
    fn encode_get_pooled_transactions() {
        let expected = hex!("f847820457f842a000000000000000000000000000000000000000000000000000000000deadc0dea000000000000000000000000000000000000000000000000000000000feedbeef");
        let mut data = vec![];
        let request = RequestPair::<GetPooledTransactions> {
            request_id: 1111,
            message: GetPooledTransactions(vec![
                hex!("00000000000000000000000000000000000000000000000000000000deadc0de").into(),
                hex!("00000000000000000000000000000000000000000000000000000000feedbeef").into(),
            ]),
        };
        request.encode(&mut data);
        assert_eq!(data, expected);
    }

    #[test]
    // Test vector from: https://eips.ethereum.org/EIPS/eip-2481
    fn decode_get_pooled_transactions() {
        let data = hex!("f847820457f842a000000000000000000000000000000000000000000000000000000000deadc0dea000000000000000000000000000000000000000000000000000000000feedbeef");
        let request = RequestPair::<GetPooledTransactions>::decode(&mut &data[..]).unwrap();
        assert_eq!(
            request,
            RequestPair::<GetPooledTransactions> {
                request_id: 1111,
                message: GetPooledTransactions(vec![
                    hex!("00000000000000000000000000000000000000000000000000000000deadc0de").into(),
                    hex!("00000000000000000000000000000000000000000000000000000000feedbeef").into(),
                ])
            }
        );
    }

    #[test]
    // Test vector from: https://eips.ethereum.org/EIPS/eip-2481
    fn encode_pooled_transactions() {
        let expected = hex!("f8d7820457f8d2f867088504a817c8088302e2489435353535353535353535353535353535353535358202008025a064b1702d9298fee62dfeccc57d322a463ad55ca201256d01f62b45b2e1c21c12a064b1702d9298fee62dfeccc57d322a463ad55ca201256d01f62b45b2e1c21c10f867098504a817c809830334509435353535353535353535353535353535353535358202d98025a052f8f61201b2b11a78d6e866abc9c3db2ae8631fa656bfe5cb53668255367afba052f8f61201b2b11a78d6e866abc9c3db2ae8631fa656bfe5cb53668255367afb");
        let mut data = vec![];
        let txs = vec![
            TransactionSigned::from_transaction_and_signature(
                Transaction::Legacy(TxLegacy {
                    chain_id: Some(1),
                    nonce: 0x8u64,
                    gas_price: 0x4a817c808,
                    gas_limit: 0x2e248,
                    to: TxKind::Call(hex!("3535353535353535353535353535353535353535").into()),
                    value: U256::from(0x200u64),
                    input: Default::default(),
                }),
                Signature::new(
                    U256::from_str(
                        "0x64b1702d9298fee62dfeccc57d322a463ad55ca201256d01f62b45b2e1c21c12",
                    )
                    .unwrap(),
                    U256::from_str(
                        "0x64b1702d9298fee62dfeccc57d322a463ad55ca201256d01f62b45b2e1c21c10",
                    )
                    .unwrap(),
                    Parity::Parity(false),
                ),
            ),
            TransactionSigned::from_transaction_and_signature(
                Transaction::Legacy(TxLegacy {
                    chain_id: Some(1),
                    nonce: 0x09u64,
                    gas_price: 0x4a817c809,
                    gas_limit: 0x33450,
                    to: TxKind::Call(hex!("3535353535353535353535353535353535353535").into()),
                    value: U256::from(0x2d9u64),
                    input: Default::default(),
                }),
                Signature::new(
                    U256::from_str(
                        "0x52f8f61201b2b11a78d6e866abc9c3db2ae8631fa656bfe5cb53668255367afb",
                    )
                    .unwrap(),
                    U256::from_str(
                        "0x52f8f61201b2b11a78d6e866abc9c3db2ae8631fa656bfe5cb53668255367afb",
                    )
                    .unwrap(),
                    Parity::Parity(false),
                ),
            ),
        ];
        let message: Vec<PooledTransactionsElement> = txs
            .into_iter()
            .map(|tx| {
                PooledTransactionsElement::try_from(tx)
                    .expect("Failed to convert TransactionSigned to PooledTransactionsElement")
            })
            .collect();
        let request = RequestPair::<PooledTransactions> {
            request_id: 1111,
            message: PooledTransactions(message), /* Assuming PooledTransactions wraps a
                                                   * Vec<PooledTransactionsElement> */
        };
        request.encode(&mut data);
        assert_eq!(data, expected);
    }

    #[test]
    // Test vector from: https://eips.ethereum.org/EIPS/eip-2481
    fn decode_pooled_transactions() {
        let data = hex!("f8d7820457f8d2f867088504a817c8088302e2489435353535353535353535353535353535353535358202008025a064b1702d9298fee62dfeccc57d322a463ad55ca201256d01f62b45b2e1c21c12a064b1702d9298fee62dfeccc57d322a463ad55ca201256d01f62b45b2e1c21c10f867098504a817c809830334509435353535353535353535353535353535353535358202d98025a052f8f61201b2b11a78d6e866abc9c3db2ae8631fa656bfe5cb53668255367afba052f8f61201b2b11a78d6e866abc9c3db2ae8631fa656bfe5cb53668255367afb");
        let txs = vec![
            TransactionSigned::from_transaction_and_signature(
                Transaction::Legacy(TxLegacy {
                    chain_id: Some(1),
                    nonce: 0x8u64,
                    gas_price: 0x4a817c808,
                    gas_limit: 0x2e248,
                    to: TxKind::Call(hex!("3535353535353535353535353535353535353535").into()),
                    value: U256::from(0x200u64),
                    input: Default::default(),
                }),
                Signature::new(
                    U256::from_str(
                        "0x64b1702d9298fee62dfeccc57d322a463ad55ca201256d01f62b45b2e1c21c12",
                    )
                    .unwrap(),
                    U256::from_str(
                        "0x64b1702d9298fee62dfeccc57d322a463ad55ca201256d01f62b45b2e1c21c10",
                    )
                    .unwrap(),
                    Parity::Eip155(37),
                ),
            ),
            TransactionSigned::from_transaction_and_signature(
                Transaction::Legacy(TxLegacy {
                    chain_id: Some(1),
                    nonce: 0x09u64,
                    gas_price: 0x4a817c809,
                    gas_limit: 0x33450,
                    to: TxKind::Call(hex!("3535353535353535353535353535353535353535").into()),
                    value: U256::from(0x2d9u64),
                    input: Default::default(),
                }),
                Signature::new(
                    U256::from_str(
                        "0x52f8f61201b2b11a78d6e866abc9c3db2ae8631fa656bfe5cb53668255367afb",
                    )
                    .unwrap(),
                    U256::from_str(
                        "0x52f8f61201b2b11a78d6e866abc9c3db2ae8631fa656bfe5cb53668255367afb",
                    )
                    .unwrap(),
                    Parity::Eip155(37),
                ),
            ),
        ];
        let message: Vec<PooledTransactionsElement> = txs
            .into_iter()
            .map(|tx| {
                PooledTransactionsElement::try_from(tx)
                    .expect("Failed to convert TransactionSigned to PooledTransactionsElement")
            })
            .collect();
        let expected = RequestPair::<PooledTransactions> {
            request_id: 1111,
            message: PooledTransactions(message),
        };

        let request = RequestPair::<PooledTransactions>::decode(&mut &data[..]).unwrap();
        assert_eq!(request, expected);
    }

    #[test]
    fn decode_pooled_transactions_network() {
        // WVM: hex data changed to fix test: MIN_GAS_LIMIT up to 500k
        let data = hex!("f9022a80f90226f8650f84832156008287fb94cf7f9e66af820a19257a2108375b180b0ec491678204d2802ca035b7bfeb9ad9ece2cbafaaf8e202e706b4cfaeb233f46198f00b44d4a566a981a0612638fb29427ca33b9a3be2a0a561beecfe0269655be160d35e72d366a6a860b87602f873041a8459682f008459682f0d8307a1209461815774383099e24810ab832a5b2a5425c154d58829a2241af62c000080c001a059e6b67f48fb32e7e570dfb11e042b5ad2e55e3ce3ce9cd989c7e06e07feeafda0016b83f4f980694ed2eee4d10667242b1f40dc406901b34125b008d334d47469f86b0384773594008398968094d3e8763675e4c425df46cc3b5c0f6cbdac39604687038d7ea4c68000802ba0ce6834447c0a4193c40382e6c57ae33b241379c5418caac9cdc18d786fd12071a03ca3ae86580e94550d7c071e3a02eadb5a77830947c9225165cf9100901bee88f86b01843b9aca00830186a094d3e8763675e4c425df46cc3b5c0f6cbdac3960468702769bb01b2a00802ba0e24d8bd32ad906d6f8b8d7741e08d1959df021698b19ee232feba15361587d0aa05406ad177223213df262cb66ccbb2f46bfdccfdfbbb5ffdda9e2c02d977631daf86b02843b9aca00830186a094d3e8763675e4c425df46cc3b5c0f6cbdac39604687038d7ea4c68000802ba00eb96ca19e8a77102767a41fc85a36afd5c61ccb09911cec5d3e86e193d9c5aea03a456401896b1b6055311536bf00a718568c744d8c1f9df59879e8350220ca18");

        let decoded_transactions =
            RequestPair::<PooledTransactions>::decode(&mut &data[..]).unwrap();
        let txs = vec![
            TransactionSigned::from_transaction_and_signature(
                Transaction::Legacy(TxLegacy {
                    chain_id: Some(4),
                    nonce: 15u64,
                    gas_price: 2200000000,
                    gas_limit: 34811,
                    to: TxKind::Call(hex!("cf7f9e66af820a19257a2108375b180b0ec49167").into()),
                    value: U256::from(1234u64),
                    input: Default::default(),
                }),
                Signature::new(
                    U256::from_str(
                        "0x35b7bfeb9ad9ece2cbafaaf8e202e706b4cfaeb233f46198f00b44d4a566a981",
                    )
                    .unwrap(),
                    U256::from_str(
                        "0x612638fb29427ca33b9a3be2a0a561beecfe0269655be160d35e72d366a6a860",
                    )
                    .unwrap(),
                    Parity::Eip155(44),
                ),
            ),
            TransactionSigned::from_transaction_and_signature(
                Transaction::Eip1559(TxEip1559 {
                    chain_id: 4,
                    nonce: 26u64,
                    max_priority_fee_per_gas: 1500000000,
                    max_fee_per_gas: 1500000013,
                    gas_limit: MIN_TRANSACTION_GAS,
                    to: TxKind::Call(hex!("61815774383099e24810ab832a5b2a5425c154d5").into()),
                    value: U256::from(3000000000000000000u64),
                    input: Default::default(),
                    access_list: Default::default(),
                }),
                Signature::new(
                    U256::from_str(
                        "0x59e6b67f48fb32e7e570dfb11e042b5ad2e55e3ce3ce9cd989c7e06e07feeafd",
                    )
                    .unwrap(),
                    U256::from_str(
                        "0x016b83f4f980694ed2eee4d10667242b1f40dc406901b34125b008d334d47469",
                    )
                    .unwrap(),
                    Parity::Parity(true),
                ),
            ),
            TransactionSigned::from_transaction_and_signature(
                Transaction::Legacy(TxLegacy {
                    chain_id: Some(4),
                    nonce: 3u64,
                    gas_price: 2000000000,
                    gas_limit: 10000000,
                    to: TxKind::Call(hex!("d3e8763675e4c425df46cc3b5c0f6cbdac396046").into()),
                    value: U256::from(1000000000000000u64),
                    input: Default::default(),
                }),
                Signature::new(
                    U256::from_str(
                        "0xce6834447c0a4193c40382e6c57ae33b241379c5418caac9cdc18d786fd12071",
                    )
                    .unwrap(),
                    U256::from_str(
                        "0x3ca3ae86580e94550d7c071e3a02eadb5a77830947c9225165cf9100901bee88",
                    )
                    .unwrap(),
                    Parity::Eip155(43),
                ),
            ),
            TransactionSigned::from_transaction_and_signature(
                Transaction::Legacy(TxLegacy {
                    chain_id: Some(4),
                    nonce: 1u64,
                    gas_price: 1000000000,
                    gas_limit: 100000,
                    to: TxKind::Call(hex!("d3e8763675e4c425df46cc3b5c0f6cbdac396046").into()),
                    value: U256::from(693361000000000u64),
                    input: Default::default(),
                }),
                Signature::new(
                    U256::from_str(
                        "0xe24d8bd32ad906d6f8b8d7741e08d1959df021698b19ee232feba15361587d0a",
                    )
                    .unwrap(),
                    U256::from_str(
                        "0x5406ad177223213df262cb66ccbb2f46bfdccfdfbbb5ffdda9e2c02d977631da",
                    )
                    .unwrap(),
                    Parity::Eip155(43),
                ),
            ),
            TransactionSigned::from_transaction_and_signature(
                Transaction::Legacy(TxLegacy {
                    chain_id: Some(4),
                    nonce: 2u64,
                    gas_price: 1000000000,
                    gas_limit: 100000,
                    to: TxKind::Call(hex!("d3e8763675e4c425df46cc3b5c0f6cbdac396046").into()),
                    value: U256::from(1000000000000000u64),
                    input: Default::default(),
                }),
                Signature::new(
                    U256::from_str(
                        "0xeb96ca19e8a77102767a41fc85a36afd5c61ccb09911cec5d3e86e193d9c5ae",
                    )
                    .unwrap(),
                    U256::from_str(
                        "0x3a456401896b1b6055311536bf00a718568c744d8c1f9df59879e8350220ca18",
                    )
                    .unwrap(),
                    Parity::Eip155(43),
                ),
            ),
        ];

        let message: Vec<PooledTransactionsElement> = txs
            .into_iter()
            .map(|tx| {
                PooledTransactionsElement::try_from(tx)
                    .expect("Failed to convert TransactionSigned to PooledTransactionsElement")
            })
            .collect();
        let expected_transactions = RequestPair::<PooledTransactions> {
            request_id: 0,
            message: PooledTransactions(message),
        };

        let decoded_transactions =
            RequestPair::<PooledTransactions>::decode(&mut &data[..]).unwrap();

        // checking tx by tx for easier debugging if there are any regressions
        for (decoded, expected) in
            decoded_transactions.message.0.iter().zip(expected_transactions.message.0.iter())
        {
            assert_eq!(decoded, expected);
        }

        assert_eq!(decoded_transactions, expected_transactions);
    }

    #[test]
    fn encode_pooled_transactions_network() {
        // WVM: hex data changed to fix test: MIN_GAS_LIMIT up to 500k
        let expected = hex!("f9022a80f90226f8650f84832156008287fb94cf7f9e66af820a19257a2108375b180b0ec491678204d2802ca035b7bfeb9ad9ece2cbafaaf8e202e706b4cfaeb233f46198f00b44d4a566a981a0612638fb29427ca33b9a3be2a0a561beecfe0269655be160d35e72d366a6a860b87602f873041a8459682f008459682f0d8307a1209461815774383099e24810ab832a5b2a5425c154d58829a2241af62c000080c001a059e6b67f48fb32e7e570dfb11e042b5ad2e55e3ce3ce9cd989c7e06e07feeafda0016b83f4f980694ed2eee4d10667242b1f40dc406901b34125b008d334d47469f86b0384773594008398968094d3e8763675e4c425df46cc3b5c0f6cbdac39604687038d7ea4c68000802ba0ce6834447c0a4193c40382e6c57ae33b241379c5418caac9cdc18d786fd12071a03ca3ae86580e94550d7c071e3a02eadb5a77830947c9225165cf9100901bee88f86b01843b9aca00830186a094d3e8763675e4c425df46cc3b5c0f6cbdac3960468702769bb01b2a00802ba0e24d8bd32ad906d6f8b8d7741e08d1959df021698b19ee232feba15361587d0aa05406ad177223213df262cb66ccbb2f46bfdccfdfbbb5ffdda9e2c02d977631daf86b02843b9aca00830186a094d3e8763675e4c425df46cc3b5c0f6cbdac39604687038d7ea4c68000802ba00eb96ca19e8a77102767a41fc85a36afd5c61ccb09911cec5d3e86e193d9c5aea03a456401896b1b6055311536bf00a718568c744d8c1f9df59879e8350220ca18");
        let txs = vec![
            TransactionSigned::from_transaction_and_signature(
                Transaction::Legacy(TxLegacy {
                    chain_id: Some(4),
                    nonce: 15u64,
                    gas_price: 2200000000,
                    gas_limit: 34811,
                    to: TxKind::Call(hex!("cf7f9e66af820a19257a2108375b180b0ec49167").into()),
                    value: U256::from(1234u64),
                    input: Default::default(),
                }),
                Signature::new(
                    U256::from_str(
                        "0x35b7bfeb9ad9ece2cbafaaf8e202e706b4cfaeb233f46198f00b44d4a566a981",
                    )
                    .unwrap(),
                    U256::from_str(
                        "0x612638fb29427ca33b9a3be2a0a561beecfe0269655be160d35e72d366a6a860",
                    )
                    .unwrap(),
                    Parity::Parity(true),
                ),
            ),
            TransactionSigned::from_transaction_and_signature(
                Transaction::Eip1559(TxEip1559 {
                    chain_id: 4,
                    nonce: 26u64,
                    max_priority_fee_per_gas: 1500000000,
                    max_fee_per_gas: 1500000013,
                    gas_limit: MIN_TRANSACTION_GAS,
                    to: TxKind::Call(hex!("61815774383099e24810ab832a5b2a5425c154d5").into()),
                    value: U256::from(3000000000000000000u64),
                    input: Default::default(),
                    access_list: Default::default(),
                }),
                Signature::new(
                    U256::from_str(
                        "0x59e6b67f48fb32e7e570dfb11e042b5ad2e55e3ce3ce9cd989c7e06e07feeafd",
                    )
                    .unwrap(),
                    U256::from_str(
                        "0x016b83f4f980694ed2eee4d10667242b1f40dc406901b34125b008d334d47469",
                    )
                    .unwrap(),
                    Parity::Parity(true),
                ),
            ),
            TransactionSigned::from_transaction_and_signature(
                Transaction::Legacy(TxLegacy {
                    chain_id: Some(4),
                    nonce: 3u64,
                    gas_price: 2000000000,
                    gas_limit: 10000000,
                    to: TxKind::Call(hex!("d3e8763675e4c425df46cc3b5c0f6cbdac396046").into()),
                    value: U256::from(1000000000000000u64),
                    input: Default::default(),
                }),
                Signature::new(
                    U256::from_str(
                        "0xce6834447c0a4193c40382e6c57ae33b241379c5418caac9cdc18d786fd12071",
                    )
                    .unwrap(),
                    U256::from_str(
                        "0x3ca3ae86580e94550d7c071e3a02eadb5a77830947c9225165cf9100901bee88",
                    )
                    .unwrap(),
                    Parity::Parity(false),
                ),
            ),
            TransactionSigned::from_transaction_and_signature(
                Transaction::Legacy(TxLegacy {
                    chain_id: Some(4),
                    nonce: 1u64,
                    gas_price: 1000000000,
                    gas_limit: 100000,
                    to: TxKind::Call(hex!("d3e8763675e4c425df46cc3b5c0f6cbdac396046").into()),
                    value: U256::from(693361000000000u64),
                    input: Default::default(),
                }),
                Signature::new(
                    U256::from_str(
                        "0xe24d8bd32ad906d6f8b8d7741e08d1959df021698b19ee232feba15361587d0a",
                    )
                    .unwrap(),
                    U256::from_str(
                        "0x5406ad177223213df262cb66ccbb2f46bfdccfdfbbb5ffdda9e2c02d977631da",
                    )
                    .unwrap(),
                    Parity::Parity(false),
                ),
            ),
            TransactionSigned::from_transaction_and_signature(
                Transaction::Legacy(TxLegacy {
                    chain_id: Some(4),
                    nonce: 2u64,
                    gas_price: 1000000000,
                    gas_limit: 100000,
                    to: TxKind::Call(hex!("d3e8763675e4c425df46cc3b5c0f6cbdac396046").into()),
                    value: U256::from(1000000000000000u64),
                    input: Default::default(),
                }),
                Signature::new(
                    U256::from_str(
                        "0xeb96ca19e8a77102767a41fc85a36afd5c61ccb09911cec5d3e86e193d9c5ae",
                    )
                    .unwrap(),
                    U256::from_str(
                        "0x3a456401896b1b6055311536bf00a718568c744d8c1f9df59879e8350220ca18",
                    )
                    .unwrap(),
                    Parity::Parity(false),
                ),
            ),
        ];
        let message: Vec<PooledTransactionsElement> = txs
            .into_iter()
            .map(|tx| {
                PooledTransactionsElement::try_from(tx)
                    .expect("Failed to convert TransactionSigned to PooledTransactionsElement")
            })
            .collect();
        let transactions = RequestPair::<PooledTransactions> {
            request_id: 0,
            message: PooledTransactions(message),
        };

        let mut encoded = vec![];
        transactions.encode(&mut encoded);
        assert_eq!(encoded.len(), transactions.length());
        let encoded_str = hex::encode(encoded);
        let expected_str = hex::encode(expected);
        assert_eq!(encoded_str.len(), expected_str.len());
        assert_eq!(encoded_str, expected_str);
    }
}
