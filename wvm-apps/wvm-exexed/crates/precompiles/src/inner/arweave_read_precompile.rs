use serde::{Deserialize, Serialize};
use reth::primitives::Bytes;
use reth::primitives::revm_primitives::{Precompile, PrecompileError, PrecompileErrors, PrecompileOutput, PrecompileResult};
use reth::revm::precompile::{PrecompileWithAddress, u64_to_address};

pub const PC_ADDRESS: u64 = 0x18;
pub const ARWEAVE_PC_READ_BASE: u64 = 3_450;

pub const TX_MAX_SIZE: usize = 18_874_368; // 18MB

pub const ARWEAVE_READ_PC: PrecompileWithAddress =
    PrecompileWithAddress(u64_to_address(PC_ADDRESS), Precompile::Standard(arweave_read));

pub const ARWEAVE_TX_ENDPOINT: &str = "https://arweave.net/";

pub const ARWEAVE_GRAPHQL_ENDPOINT: &str = "https://arweave.net/graphql";

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

fn arweave_read(input: &Bytes, gas_limit: u64) -> PrecompileResult {
    let data_size = input.len();
    let gas_used: u64 = (10_000 + data_size * 3) as u64;

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
            tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap().block_on(
                async {

                    let query = r#"query {
  transactions(ids: ["$ID"]) {
    edges {
      node {
        id
        data {
          size
        }
      }
    }
  }
}"#;

                    let query = query.replace("$ID", id.as_str());

                    let graphql_client = reqwest_graphql::Client::new(ARWEAVE_GRAPHQL_ENDPOINT);
                    let data = graphql_client.query::<Response>(query.as_str()).await;

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
                        let download_tx = reqwest::get(format!("{}{}", ARWEAVE_TX_ENDPOINT, id.as_str())).await;
                        match download_tx {
                            Ok(tx) => {
                                Ok(PrecompileOutput::new(gas_used, tx.bytes().await.unwrap().into()))
                            }
                            Err(_) => {
                                Err(PrecompileErrors::Error(PrecompileError::Other(
                                    "Arweave Transaction was not found".to_string(),
                                )))
                            }
                        }
                    } else {
                        Err(PrecompileErrors::Error(PrecompileError::Other(
                            "Arweave Transaction size is greater than allowed (18mb)".to_string(),
                        )))
                    }
                },
            )
        }
        Err(_) => {
            Err(PrecompileErrors::Error(PrecompileError::Other(
                "Transaction id could not be parsed".to_string(),
            )))
        }
    };

    res
}

#[cfg(test)]
mod arweave_read_pc_tests {
    use reth::primitives::Bytes;
    use reth::primitives::revm_primitives::PrecompileOutput;
    use crate::inner::arweave_read_precompile::arweave_read;

    #[test]
    pub fn test_arweave_read_precompile() {
        let input = Bytes::from("bs318IdjLWQK7pF_bNIbJnpade8feD7yGAS8xIffJDI".as_bytes());
        let PrecompileOutput { gas_used, bytes } = arweave_read(&input, 100_000).unwrap();
        assert_eq!(bytes.len(), 11);
        assert_eq!(bytes.to_vec(), "Hello world".as_bytes().to_vec());
    }

}