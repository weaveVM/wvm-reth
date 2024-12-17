use crate::inner::{
    graphql_util::{build_transaction_query, send_graphql},
    util::{clean_gateway_url, download_tx, DEFAULT_ARWEAVE_TX_ENDPOINT},
};
use alloy_primitives::Bytes;
use reth::primitives::revm_primitives::{
    Precompile, PrecompileError, PrecompileErrors, PrecompileResult,
};
use serde::{Deserialize, Serialize};
use std::time::Instant;
use wvm_static::internal_block;

pub const ARWEAVE_PC_READ_BASE: u64 = 10_000;

pub const TX_MAX_SIZE: usize = 18_874_368; // 18MB

pub const ARWEAVE_READ_PC: Precompile = Precompile::Standard(arweave_read);

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
        return Err(PrecompileErrors::Error(PrecompileError::OutOfGas));
    }

    if input.is_empty() {
        return Err(PrecompileErrors::Error(PrecompileError::Other(
            "Arweave Transaction Id cannot be empty".to_string(),
        )));
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
                return Err(PrecompileErrors::Error(PrecompileError::Other(
                    "Arweave Transaction size is greater than allowed (18mb)".to_string(),
                )));
            }

            Ok(download_tx(gas_used, clean_gateway, tx_id).map_err(|_| {
                PrecompileError::Other("Tokio runtime could not block_on for operation".to_string())
            })?)
        }
        Err(_) => Err(PrecompileErrors::Error(PrecompileError::Other(
            "Transaction id could not be parsed".to_string(),
        ))),
    };

    res
}

#[cfg(test)]
mod arweave_read_pc_tests {
    use crate::inner::arweave_read_precompile::{arweave_read, parse_gateway_content};
    use alloy_primitives::Bytes;
    use reth::primitives::revm_primitives::PrecompileOutput;
    use std::time::Instant;
    use borsh::BorshDeserialize;
    use wvm_borsh::block::BorshSealedBlockWithSenders;

    #[test]
    pub fn test_unbrotli_borsh_sealed_header() {
        let unbrotli: Vec<u8> = vec![32, 0, 0, 0, 224, 32, 31, 30, 40, 79, 190, 111, 160, 201, 14, 129, 17, 148, 161, 26, 105, 74, 8, 210, 64, 244, 105, 25, 150, 185, 24, 47, 46, 118, 127, 238, 32, 0, 0, 0, 1, 42, 212, 101, 218, 96, 77, 182, 93, 149, 188, 2, 217, 211, 199, 154, 181, 219, 168, 85, 146, 153, 39, 197, 116, 85, 56, 155, 83, 168, 151, 21, 32, 0, 0, 0, 29, 204, 77, 232, 222, 199, 93, 122, 171, 133, 181, 103, 182, 204, 212, 26, 211, 18, 69, 27, 148, 138, 116, 19, 240, 161, 66, 253, 64, 212, 147, 71, 32, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 162, 160, 217, 119, 132, 120, 5, 254, 34, 75, 120, 157, 140, 77, 61, 113, 26, 178, 81, 231, 32, 0, 0, 0, 146, 91, 119, 23, 227, 245, 86, 135, 249, 169, 217, 35, 44, 10, 178, 224, 39, 254, 174, 207, 60, 8, 101, 105, 231, 41, 162, 157, 216, 19, 159, 126, 32, 0, 0, 0, 86, 232, 31, 23, 27, 204, 85, 166, 255, 131, 69, 230, 146, 192, 248, 110, 91, 72, 224, 27, 153, 108, 173, 192, 1, 98, 47, 181, 227, 99, 180, 33, 32, 0, 0, 0, 86, 232, 31, 23, 27, 204, 85, 166, 255, 131, 69, 230, 146, 192, 248, 110, 91, 72, 224, 27, 153, 108, 173, 192, 1, 98, 47, 181, 227, 99, 180, 33, 1, 32, 0, 0, 0, 86, 232, 31, 23, 27, 204, 85, 166, 255, 131, 69, 230, 146, 192, 248, 110, 91, 72, 224, 27, 153, 108, 173, 192, 1, 98, 47, 181, 227, 99, 180, 33, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 32, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 39, 54, 17, 0, 0, 0, 0, 0, 0, 163, 225, 17, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 115, 163, 225, 102, 0, 0, 0, 0, 32, 0, 0, 0, 209, 71, 205, 129, 188, 81, 149, 75, 86, 168, 107, 105, 94, 169, 250, 114, 165, 85, 125, 70, 221, 184, 163, 200, 20, 112, 110, 104, 149, 217, 206, 192, 0, 0, 0, 0, 0, 0, 0, 0, 1, 7, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 1, 32, 0, 0, 0, 233, 87, 13, 226, 56, 43, 80, 54, 119, 131, 88, 82, 3, 39, 89, 22, 121, 22, 15, 110, 203, 82, 219, 221, 106, 193, 244, 9, 36, 232, 26, 20, 0, 17, 0, 0, 0, 114, 101, 116, 104, 47, 118, 49, 46, 48, 46, 53, 47, 108, 105, 110, 117, 120, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0];
        let mut slice: &[u8] = &unbrotli;
        let obj = BorshSealedBlockWithSenders::deserialize(&mut slice).unwrap();
        println!("Deserialized {:?}", obj.0);
        let borsh_sealed_block_with_sender = borsh::from_slice::<BorshSealedBlockWithSenders>(&unbrotli).unwrap();
    }

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
