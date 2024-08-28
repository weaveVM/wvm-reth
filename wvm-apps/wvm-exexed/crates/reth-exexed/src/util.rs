use precompiles::inner::graphql_util::send_graphql;

pub const AR_GRAPHQL_GATEWAY: &str = "https://arweave.net";

pub(crate) async fn check_block_existence(block_hash: &str) -> bool {
    let query = {
        let query = "{\n  transactions(tags: [{name: \"Block-Hash\", values: [\"$block_hash\"]}]) {\n    edges {\n      node {\n        id\n        tags {\n          name\n          value\n        }\n        data {\n          size\n        }\n      }\n    }\n  }\n}\n";
        let query = query.replace("$block_hash", block_hash);
        query
    };

    let data = send_graphql(AR_GRAPHQL_GATEWAY, query.as_str()).await;
    if let Ok(data) = data {
        let resp = data.data.transactions.edges.get(0);
        resp.is_some()
    } else {
        false
    }
}

#[cfg(test)]
mod tests {
    use crate::util::check_block_existence;

    #[tokio::test]
    async fn test_check_block_existence() {
        let block_1 = check_block_existence(
            "0xaf1c63505340e7c923a7cbc70b8353dfceab667100174943293896a9b75ea091",
        )
        .await;
        let block_2 = check_block_existence(
            "0xaf1c63505340e7c923a7cbc70b8353dfceab667100174943293896a9b75ea092",
        )
        .await;
        assert!(block_1);
        assert!(!block_2);
    }
}
