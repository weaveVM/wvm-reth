use precompiles::inner::graphql_util::send_graphql;

pub const AR_GRAPHQL_GATEWAY: &str = "https://arweave.mainnet.irys.xyz";
pub const WEVM_OWNER_WALLET: &str = "5JUE58yemNynRDeQDyVECKbGVCQbnX7unPrBRqCPVn5Z";

pub(crate) async fn check_block_existence(block_hash: &str) -> bool {
    let query = {
        let query = "query {\n    transactions(\n        order: DESC,\n        tags: [{\n            name: \"Block-Hash\",\n            values: [\"$block_hash\"]\n        }]\n        owners: [\"$owner_wallet\"]\n    ) {\n        edges {\n            node {\n                id\n                tags {\n                    name\n                    value\n                }\n            }\n        }\n    }\n}";
        let query = query.replace("$block_hash", block_hash);
        let query = query.replace("$owner_wallet", WEVM_OWNER_WALLET);
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
            "0xd579c6931a9d1744b2540eeb540540a5582f6befebc59871b1ba4a4d967bd794",
        )
        .await;
        let block_2 = check_block_existence(
            "0xd579c6931a9d1744b2540eeb540540a5582f6befebc59871b1ba4a4d967bd793",
        )
        .await;
        assert!(block_1);
        assert!(!block_2);
    }
}
