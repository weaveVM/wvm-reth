use crate::inner::{
    graphql_util::{build_transaction_query, send_graphql},
    util::{clean_gateway_url, download_tx, DEFAULT_ARWEAVE_TX_ENDPOINT},
};
use alloy_primitives::Bytes;

use reth::revm::precompile::{PrecompileError, PrecompileFn, PrecompileResult};

use serde::{Deserialize, Serialize};
use std::time::Instant;
use wvm_static::internal_block;

pub const ARWEAVE_PC_READ_BASE: u64 = 10_000;

pub const TX_MAX_SIZE: usize = 18_874_368; // 18MB

pub const ARWEAVE_READ_PC: PrecompileFn = arweave_read as PrecompileFn;

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

pub fn parse_gateway_content(input: &str) -> (String, String) {
    let default_endpoint = DEFAULT_ARWEAVE_TX_ENDPOINT;
    let mut parts = input.split(';');
    let first_part = parts.next().unwrap_or(default_endpoint);
    let second_part = parts.next().unwrap_or(first_part);

    let endpoint = if input.contains(';') { first_part } else { default_endpoint };

    (endpoint.to_string(), second_part.to_string())
}

fn arweave_read(input: &Bytes, gas_limit: u64) -> PrecompileResult {
    let data_size = input.len();
    let gas_used: u64 = (ARWEAVE_PC_READ_BASE as usize + data_size * 3) as u64;

    if gas_used > gas_limit {
        return Err(PrecompileError::OutOfGas);
    }

    if input.is_empty() {
        return Err(PrecompileError::Other("Arweave Transaction Id cannot be empty".to_string()));
    }

    let id_str = String::from_utf8(input.0.to_vec());

    let res = match id_str {
        Ok(id) => {
            let (gateway, tx_id) = parse_gateway_content(id.as_str());
            let clean_gateway = clean_gateway_url(gateway.as_str());
            let query = build_transaction_query(Some(&[tx_id.clone()]), None, None, None, true);

            let data = send_graphql(clean_gateway.as_str(), query.as_str());
            let tx_size = match data {
                Ok(data) => data
                    .data
                    .transactions
                    .edges
                    .get(0)
                    .and_then(|edge| edge.node.data.as_ref())
                    .and_then(|data| data.size.parse::<usize>().ok())
                    .unwrap_or(0),
                Err(_) => 0,
            };

            if tx_size > TX_MAX_SIZE {
                return Err(PrecompileError::Other(
                    "Arweave Transaction size is greater than allowed (18mb)".to_string(),
                ));
            }

            Ok(download_tx(gas_used, clean_gateway, tx_id).map_err(|_| {
                PrecompileError::Other("Tokio runtime could not block_on for operation".to_string())
            })?)
        }
        Err(_) => Err(PrecompileError::Other("Transaction id could not be parsed".to_string())),
    };

    res
}

#[cfg(test)]
mod arweave_read_pc_tests {
    use crate::inner::arweave_read_precompile::{arweave_read, parse_gateway_content};
    use alloy_primitives::Bytes;
    use reth::revm::precompile::PrecompileOutput;
    use std::time::Instant;

    #[test]
    pub fn test_arweave_read_precompile() {
        std::env::set_var("CAREFUL_TOKIO", "false");
        let input = Bytes::from("bs318IdjLWQK7pF_bNIbJnpade8feD7yGAS8xIffJDI".as_bytes());
        let now = Instant::now();
        let PrecompileOutput { gas_used, bytes } = arweave_read(&input, 100_000).unwrap();
        println!("Secs to run PC {}", now.elapsed().as_secs());
        assert_eq!(bytes.len(), 11);
        assert_eq!(bytes.to_vec(), "Hello world".as_bytes().to_vec());
    }

    #[test]
    pub fn test_arweave_read_precompile_custom_gateway() {
        std::env::set_var("DURATION_SECONDS", "20000");
        let input =
            Bytes::from("https://ar-io.dev;bs318IdjLWQK7pF_bNIbJnpade8feD7yGAS8xIffJDI".as_bytes());
        let PrecompileOutput { gas_used, bytes } = arweave_read(&input, 100_000).unwrap();
        assert_eq!(bytes.len(), 11);
        assert_eq!(bytes.to_vec(), "Hello world".as_bytes().to_vec());
    }

    #[test]
    pub fn test_parse_url() {
        let input = "http://arweave-custom.net;bs318IdjLWQK7pF_bNIbJnpade8feD7yGAS8xIffJDI";
        let parse_url_data = parse_gateway_content(input);
        assert_eq!(parse_url_data.0, "http://arweave-custom.net");
        assert_eq!(parse_url_data.1, "bs318IdjLWQK7pF_bNIbJnpade8feD7yGAS8xIffJDI");

        let input = "bs318IdjLWQK7pF_bNIbJnpade8feD7yGAS8xIffJDI";
        let parse_url_data = parse_gateway_content(input);
        assert_eq!(parse_url_data.0, "https://arweave.net/");
        assert_eq!(parse_url_data.1, "bs318IdjLWQK7pF_bNIbJnpade8feD7yGAS8xIffJDI");
    }

    #[tokio::test]
    pub async fn test_graphql() {
        let client = reqwest::Client::builder().build().unwrap();

        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert("Content-Type", "application/json".parse().unwrap());

        let data = r#"
{
    "query": "query {\n  transactions(ids: [\"bs318IdjLWQK7pF_bNIbJnpade8feD7yGAS8xIffJDI\"],\n) {\n    edges {\n      node {\n        id\n        data {\n          size\n        }\n      }\n    }\n  }\n}"
}
"#;
        let json: serde_json::Value = serde_json::from_str(&data).unwrap();

        let start_request = std::time::Instant::now();
        let request = client
            .request(reqwest::Method::POST, "http://arweave.net/graphql")
            .headers(headers)
            .json(&json);

        let response = request.send().await.unwrap();
        println!("Request sent: {:?}", start_request.elapsed());

        println!("Response headers {:?}", &response.headers());
        let start_read = std::time::Instant::now();
        let body = response.text().await.unwrap();
        println!("Response read: {:?}", start_read.elapsed());

        println!("{}", body);
    }

    #[tokio::test]
    pub async fn test_graphql_ureq() {
        let data = r#"
{
    "query": "query {\n  transactions(ids: [\"bs318IdjLWQK7pF_bNIbJnpade8feD7yGAS8xIffJDI\"],\n) {\n    edges {\n      node {\n        id\n        data {\n          size\n        }\n      }\n    }\n  }\n}"
}
"#;
        let json: serde_json::Value = serde_json::from_str(&data).unwrap();

        let start_request = std::time::Instant::now();
        let res = ureq::post("https://arweave.net/graphql").send_json(ureq::json!({
    "query": "query {\n  transactions(ids: [\"bs318IdjLWQK7pF_bNIbJnpade8feD7yGAS8xIffJDI\"],\n) {\n    edges {\n      node {\n        id\n        data {\n          size\n        }\n      }\n    }\n  }\n}"
})).unwrap();
        println!("Request sent: {:?}", start_request.elapsed());
        println!("{:?}", res.into_string().unwrap());
    }
}
