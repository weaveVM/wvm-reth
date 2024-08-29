use eyre::Error;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Response {
    pub data: Data,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Data {
    pub transactions: Transactions,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Transactions {
    pub edges: Vec<Edge>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Edge {
    pub node: Node,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tag {
    pub name: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Node {
    pub id: String,
    pub data: Option<NodeData>,
    pub tags: Option<Vec<Tag>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeData {
    pub size: String,
}

pub async fn send_graphql(gateway: &str, query: &str) -> Result<Response, Error> {
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
