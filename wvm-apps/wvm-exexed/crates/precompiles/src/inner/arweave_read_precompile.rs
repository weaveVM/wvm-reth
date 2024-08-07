use eyre::Error;
use reth::{
    primitives::{
        revm_primitives::{
            Precompile, PrecompileError, PrecompileErrors, PrecompileOutput, PrecompileResult,
        },
        Bytes,
    },
    revm::precompile::{u64_to_address, PrecompileWithAddress},
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub const PC_ADDRESS: u64 = 0x18;
pub const ARWEAVE_PC_READ_BASE: u64 = 10_000;

pub const TX_MAX_SIZE: usize = 18_874_368; // 18MB

pub const ARWEAVE_READ_PC: PrecompileWithAddress =
    PrecompileWithAddress(u64_to_address(PC_ADDRESS), Precompile::Standard(arweave_read));

pub const ARWEAVE_TX_ENDPOINT: &str = "https://arweave.net/";

#[derive(Debug, Serialize, Deserialize)]
struct Response {
    data: Data,
}

#[derive(Debug, Serialize, Deserialize)]
struct Data {
    transactions: Transactions,
}

#[derive(Debug, Serialize, Deserialize)]
struct Transactions {
    edges: Vec<Edge>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Edge {
    node: Node,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Node {
    id: String,
    data: NodeData,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct NodeData {
    size: String,
}

fn parse_url(input: &str) -> (String, String) {
    let default_endpoint = ARWEAVE_TX_ENDPOINT;
    let mut parts = input.split(';');
    let first_part = parts.next().unwrap_or(default_endpoint);
    let second_part = parts.next().unwrap_or(first_part);

    let endpoint = if input.contains(';') { first_part } else { default_endpoint };

    (endpoint.to_string(), second_part.to_string())
}

fn clean_gateway_url(gateway: &str) -> String {
    let clean_gateway =
        if gateway.ends_with('/') { &gateway[..gateway.len() - 1] } else { gateway };

    clean_gateway.to_string()
}

async fn send_graphql(gateway: &str, tx_id: &str) -> Result<Response, Error> {
    // Define the GraphQL query
    let mut query = "{\n  transactions(ids: [\"$id\"]) {\n    edges {\n      node {\n        id\n        data {\n          size\n        }\n      }\n    }\n  }\n}\n";
    let query = query.replace("$id", tx_id);

    let query = serde_json::json!({
        "variables": {},
        "query": query
    });

    // Create a client
    let client = reqwest::Client::new();

    // Send the request
    let res = client
        .post(format!("{}/{}", gateway, "graphql"))
        .header("Content-Type", "application/json")
        .json(&query)
        .send()
        .await?;

    Ok(res.json::<Response>().await?)
}

fn arweave_read(input: &Bytes, gas_limit: u64) -> PrecompileResult {
    let data_size = input.len();
    let gas_used: u64 = (ARWEAVE_PC_READ_BASE as usize + data_size * 3) as u64;

    if gas_used > gas_limit {
        return Err(PrecompileErrors::Error(PrecompileError::OutOfGas));
    }

    if input.is_empty() {
        return Err(PrecompileErrors::Error(PrecompileError::Other(
            "Arweave Transaction Id cannot be empty".to_string(),
        )));
    }

    let id_str = unsafe { String::from_utf8(input.0.to_vec()) };

    let res = match id_str {
        Ok(id) => {
            let (gateway, tx_id) = parse_url(id.as_str());
            tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap().block_on(
                async {
                    let clean_gateway = clean_gateway_url(gateway.as_str());
                    let data = send_graphql(clean_gateway.as_str(), tx_id.as_str()).await;

                    let tx_size = if let Ok(data) = data {
                        let resp = data.data;
                        let tx = resp.transactions.edges.get(0);
                        if let Some(&ref tx) = tx {
                            let tx_size = tx.clone().node.data.size;
                            let tx_size = tx_size.parse::<usize>().unwrap();
                            tx_size
                        } else {
                            0
                        }
                    } else {
                        0
                    };

                    if TX_MAX_SIZE >= tx_size {
                        let download_tx =
                            reqwest::get(format!("{}/{}", clean_gateway, tx_id.as_str())).await;
                        match download_tx {
                            Ok(tx) => Ok(PrecompileOutput::new(
                                gas_used,
                                tx.bytes().await.unwrap().into(),
                            )),
                            Err(_) => Err(PrecompileErrors::Error(PrecompileError::Other(
                                "Arweave Transaction was not found".to_string(),
                            ))),
                        }
                    } else {
                        Err(PrecompileErrors::Error(PrecompileError::Other(
                            "Arweave Transaction size is greater than allowed (18mb)".to_string(),
                        )))
                    }
                },
            )
        }
        Err(_) => Err(PrecompileErrors::Error(PrecompileError::Other(
            "Transaction id could not be parsed".to_string(),
        ))),
    };

    res
}

#[cfg(test)]
mod arweave_read_pc_tests {
    use crate::inner::arweave_read_precompile::{arweave_read, parse_url};
    use reth::primitives::{revm_primitives::PrecompileOutput, Bytes};

    #[test]
    pub fn test_arweave_read_precompile() {
        let input = Bytes::from("bs318IdjLWQK7pF_bNIbJnpade8feD7yGAS8xIffJDI".as_bytes());
        let PrecompileOutput { gas_used, bytes } = arweave_read(&input, 100_000).unwrap();
        assert_eq!(bytes.len(), 11);
        assert_eq!(bytes.to_vec(), "Hello world".as_bytes().to_vec());
    }

    #[test]
    pub fn test_arweave_read_precompile_custom_gateway() {
        let input =
            Bytes::from("https://ar-io.dev;bs318IdjLWQK7pF_bNIbJnpade8feD7yGAS8xIffJDI".as_bytes());
        let PrecompileOutput { gas_used, bytes } = arweave_read(&input, 100_000).unwrap();
        assert_eq!(bytes.len(), 11);
        assert_eq!(bytes.to_vec(), "Hello world".as_bytes().to_vec());
    }

    #[test]
    pub fn test_parse_url() {
        let input = "http://arweave-custom.net;bs318IdjLWQK7pF_bNIbJnpade8feD7yGAS8xIffJDI";
        let parse_url_data = parse_url(input);
        assert_eq!(parse_url_data.0, "http://arweave-custom.net");
        assert_eq!(parse_url_data.1, "bs318IdjLWQK7pF_bNIbJnpade8feD7yGAS8xIffJDI");

        let input = "bs318IdjLWQK7pF_bNIbJnpade8feD7yGAS8xIffJDI";
        let parse_url_data = parse_url(input);
        assert_eq!(parse_url_data.0, "https://arweave.net/");
        assert_eq!(parse_url_data.1, "bs318IdjLWQK7pF_bNIbJnpade8feD7yGAS8xIffJDI");
    }
}
