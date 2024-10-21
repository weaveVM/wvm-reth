use eyre::Error;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Response {
    pub data: Data,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Data {
    pub transactions: Transactions,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Transactions {
    pub edges: Vec<Edge>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Edge {
    pub node: Node,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Tag {
    pub name: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Node {
    pub id: String,
    pub data: Option<NodeData>,
    pub tags: Option<Vec<Tag>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NodeData {
    pub size: String,
}

pub fn build_transaction_query(
    ids: Option<&[String]>,
    tags: Option<&[(String, Vec<String>)]>,
    owners: Option<&[String]>,
    order: Option<String>,
    include_data_size: bool, // New parameter to control inclusion of data { size }
) -> String {
    let mut query = String::new();

    query.push_str("query {\n  transactions(");

    if let Some(order) = order {
        query.push_str(&format!("order: {},\n", order));
    }

    if let Some(ids) = ids {
        let ids_str = ids.iter().map(|id| format!("\"{}\"", id)).collect::<Vec<_>>().join(", ");
        query.push_str(&format!("ids: [{}],\n", ids_str));
    }

    if let Some(tags) = tags {
        let tags_str = tags
            .iter()
            .map(|(name, values)| {
                let values_str =
                    values.iter().map(|v| format!("\"{}\"", v)).collect::<Vec<_>>().join(", ");
                format!("{{name: \"{}\", values: [{}]}}", name, values_str)
            })
            .collect::<Vec<_>>()
            .join(",\n");
        query.push_str(&format!("tags: [{}],\n", tags_str));
    }

    if let Some(owners) = owners {
        let owners_str =
            owners.iter().map(|owner| format!("\"{}\"", owner)).collect::<Vec<_>>().join(", ");
        query.push_str(&format!("owners: [{}],\n", owners_str));
    }

    query.push_str(") {\n    edges {\n      node {\n        id\n");

    if tags.is_some() {
        query.push_str("        tags {\n          name\n          value\n        }\n");
    }

    if include_data_size {
        // Conditionally include data { size }
        query.push_str("        data {\n          size\n        }\n");
    }

    query.push_str("      }\n    }\n  }\n}\n");

    query
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
