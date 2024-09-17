use crate::{
    inner::{
        graphql_util::{build_transaction_query, send_graphql},
        string_block::Block,
        util::{clean_gateway_url, download_tx, DEFAULT_ARWEAVE_TX_ENDPOINT},
    },
    WVM_DATA_PUBLISHERS,
};
use rbrotli::from_brotli;
use reth::primitives::{
    revm_primitives::{Precompile, PrecompileOutput, PrecompileResult},
    Bytes,
};
use revm_primitives::{PrecompileError, PrecompileErrors};
use wevm_borsh::block::BorshSealedBlockWithSenders;

pub const WVM_BLOCK_PC: Precompile = Precompile::Standard(wevm_read_block_pc);

pub const WVM_BLOCK_PC_READ_BASE: u64 = 10_000;

pub fn parse_req_input(input: &str) -> (String, String, String) {
    let default_endpoint = DEFAULT_ARWEAVE_TX_ENDPOINT;
    let mut parts = input.split(';');

    let first_part = parts.next().unwrap_or("");
    let (endpoint, second_part) = if parts.clone().count() == 1 {
        (default_endpoint.to_string(), first_part.to_string())
    } else {
        (first_part.to_string(), parts.next().unwrap_or("").to_string())
    };

    let third_part = parts.next().unwrap_or("").to_string();

    (endpoint, second_part, third_part)
}

fn wevm_read_block_pc(input: &Bytes, gas_limit: u64) -> PrecompileResult {
    let data_size = input.len();
    let gas_used: u64 = (WVM_BLOCK_PC_READ_BASE as usize + data_size * 3) as u64;

    if gas_used > gas_limit {
        return Err(PrecompileErrors::Error(PrecompileError::OutOfGas));
    }

    if input.is_empty() {
        return Err(PrecompileErrors::Error(PrecompileError::Other(
            "A block id must be provided".to_string(),
        )));
    }

    let block_id = unsafe { String::from_utf8(input.0.to_vec()) };

    match block_id {
        Ok(input_data) => {
            let (gateway, block_id, field) = parse_req_input(input_data.as_str());
            if field.len() == 0 {
                Err(PrecompileErrors::Error(PrecompileError::Other(
                    "A field must be specified".to_string(),
                )))
            } else {
                tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .unwrap()
                    .block_on(async {
                        let clean_gateway = clean_gateway_url(gateway.as_str());
                        let query = {
                            let query = build_transaction_query(
                                None,
                                Some(&[("Block-Number".to_string(), vec![block_id.to_string()])]),
                                Some(&WVM_DATA_PUBLISHERS.map(|i| i.to_string())),
                                None,
                                false,
                            );

                            query
                        };

                        println!("{}", query);

                        let data = send_graphql(clean_gateway.as_str(), query.as_str()).await;

                        let edge = match data {
                            Ok(res) => {
                                let resp = res.data.transactions.edges.get(0);
                                if let Some(&ref tx) = resp {
                                    Some(tx.clone())
                                } else {
                                    None
                                }
                            }
                            Err(e) => {
                                println!("{}", e);
                                None
                            }
                        };

                        if let Some(edge) = edge {
                            let tags = edge.node.tags.unwrap();
                            let encoding = tags
                                .iter()
                                .find(|i| i.name == String::from("WeaveVM:Encoding"))
                                .unwrap();
                            let get_data = download_tx(gas_used, clean_gateway, edge.node.id).await;

                            let output = match get_data {
                                Ok(resp) => {
                                    let bytes = resp.bytes.to_vec();
                                    match encoding.value.as_str() {
                                        "Borsh-Brotli" => {
                                            let unbrotli = from_brotli(bytes);
                                            let unborsh =
                                                borsh::from_slice::<BorshSealedBlockWithSenders>(
                                                    unbrotli.as_slice(),
                                                )
                                                .unwrap();
                                            let str_block = Block::from(unborsh);

                                            let data = match field.as_str() {
                                                "base_fee_per_gas" => Some(
                                                    str_block
                                                        .base_fee_per_gas
                                                        .unwrap()
                                                        .into_bytes(),
                                                ),
                                                "blob_gas_used" => Some(
                                                    str_block.blob_gas_used.unwrap().into_bytes(),
                                                ),
                                                "difficulty" => {
                                                    Some(str_block.difficulty.unwrap().into_bytes())
                                                }
                                                "excess_blob_gas" => Some(
                                                    str_block.excess_blob_gas.unwrap().into_bytes(),
                                                ),
                                                "extra_data" => {
                                                    Some(str_block.extra_data.unwrap().into_bytes())
                                                }
                                                "gas_limit" => {
                                                    Some(str_block.gas_limit.unwrap().into_bytes())
                                                }
                                                "gas_used" => {
                                                    Some(str_block.gas_used.unwrap().into_bytes())
                                                }
                                                "hash" => {
                                                    Some(str_block.hash.unwrap().into_bytes())
                                                }
                                                "logs_bloom" => {
                                                    Some(str_block.logs_bloom.unwrap().into_bytes())
                                                }
                                                "mix_hash" => {
                                                    Some(str_block.mix_hash.unwrap().into_bytes())
                                                }
                                                "nonce" => {
                                                    Some(str_block.nonce.unwrap().into_bytes())
                                                }
                                                "parent_beacon_block_root" => Some(
                                                    str_block
                                                        .parent_beacon_block_root
                                                        .unwrap()
                                                        .into_bytes(),
                                                ),
                                                "parent_hash" => Some(
                                                    str_block.parent_hash.unwrap().into_bytes(),
                                                ),
                                                "receipts_root" => Some(
                                                    str_block.receipts_root.unwrap().into_bytes(),
                                                ),
                                                "size" => {
                                                    Some(str_block.size.unwrap().into_bytes())
                                                }
                                                "state_root" => {
                                                    Some(str_block.state_root.unwrap().into_bytes())
                                                }
                                                "timestamp" => {
                                                    Some(str_block.timestamp.unwrap().into_bytes())
                                                }
                                                "transactions" => Some(
                                                    str_block.transactions.join(",").into_bytes(),
                                                ),
                                                _ => None,
                                            };

                                            if let Some(valid_data) = data {
                                                Ok(PrecompileOutput::new(
                                                    gas_used,
                                                    valid_data.into(),
                                                ))
                                            } else {
                                                Err(PrecompileErrors::Error(
                                                    PrecompileError::Other(
                                                        "Unknown field".to_string(),
                                                    ),
                                                ))
                                            }
                                        }
                                        _ => Err(PrecompileErrors::Error(PrecompileError::Other(
                                            "Unknown encoding".to_string(),
                                        ))),
                                    }
                                }
                                Err(_) => Err(PrecompileErrors::Error(PrecompileError::Other(
                                    "Invalid data".to_string(),
                                ))),
                            };

                            output
                        } else {
                            Err(PrecompileErrors::Error(PrecompileError::Other(
                                "Unknown Block".to_string(),
                            )))
                        }
                    })
            }
        }
        Err(_) => Err(PrecompileErrors::Error(PrecompileError::Other(
            "Block id could not be parsed".to_string(),
        ))),
    }
}

#[cfg(test)]
mod arweave_read_pc_tests {
    use crate::inner::wevm_block_precompile::wevm_read_block_pc;
    use reth::primitives::{revm_primitives::PrecompileOutput, Bytes};

    #[test]
    pub fn test_read_wvm_block() {
        let input = Bytes::from("https://arweave.mainnet.irys.xyz;1127975;hash".as_bytes());
        let PrecompileOutput { gas_used, bytes } = wevm_read_block_pc(&input, 100_000).unwrap();
        assert_eq!(bytes.len(), 66);
        assert_eq!(
            bytes.to_vec(),
            "0xe0201f1e284fbe6fa0c90e811194a11a694a08d240f4691996b9182f2e767fee"
                .as_bytes()
                .to_vec()
        );
    }

    #[test]
    pub fn test_read_wvm_block_arweave() {
        let input = Bytes::from("https://arweave.net;1399390;hash".as_bytes());
        let PrecompileOutput { gas_used, bytes } = wevm_read_block_pc(&input, 100_000).unwrap();
        assert_eq!(bytes.len(), 66);
        assert_eq!(
            bytes.to_vec(),
            "0xe0201f1e284fbe6fa0c90e811194a11a694a08d240f4691996b9182f2e767fee"
                .as_bytes()
                .to_vec()
        );
    }
}
