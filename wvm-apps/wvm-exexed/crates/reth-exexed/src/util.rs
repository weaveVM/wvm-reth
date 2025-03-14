use precompiles::{
    inner::graphql_util::{build_transaction_query, send_graphql},
    WVM_DATA_PUBLISHERS,
};

pub const AR_GRAPHQL_GATEWAY: &str = "https://arweave.mainnet.irys.xyz";

pub(crate) fn check_block_existence(block_hash: &str, irys: bool) -> bool {
    let mut query = build_transaction_query(
        None,
        Some(&[
            ("Block-Hash".to_string(), vec![block_hash.to_string()]),
            ("Protocol".to_string(), vec!["WeaveVM-ExEx".to_string()]),
            // this check is disabled, but once we activate it, we should
            // append `("Protocol".to_string(), vec!["LN-ExEx".to_string()])`
        ]),
        Some(&WVM_DATA_PUBLISHERS.map(|i| i.to_string())),
        Some("DESC".to_string()),
        false,
    );

    let gateway = {
        if !irys {
            query = query.replace("order:", "sort:");
            query = query.replace("DESC", "HEIGHT_DESC");
            String::from("https://arweave.net")
        } else {
            String::from(AR_GRAPHQL_GATEWAY)
        }
    };

    let data = send_graphql(gateway.as_str(), query.as_str());
    if let Ok(data) = data {
        let resp = data.data.transactions.edges.get(0);
        resp.is_some()
    } else {
        if !irys {
            check_block_existence(block_hash, true)
        } else {
            false
        }
    }
}

// TODO: Uncomment test when gateways issues go away.

#[cfg(test)]
mod tests {
    use crate::util::check_block_existence;

    // #[tokio::test]
    // async fn test_check_block_existence() {
    //     let block_2 = check_block_existence(
    //         "0x2685fbb2e5b93cea32e7e51334d3cc746e1a6790b901eddb3df8214be18899a2",
    //         false,
    //     );
    //     let block_1 = check_block_existence(
    //         "0x2685fbb2e5b93cea32e7e51334d3cc746e1a6790b901eddb3df8214be18899a1",
    //         false,
    //     );
    //     assert!(block_1);
    //     assert!(!block_2);
    // }
}
