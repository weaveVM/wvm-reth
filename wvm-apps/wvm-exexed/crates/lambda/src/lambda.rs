use reth::{api::FullNodeComponents, primitives::TransactionSigned};

use alloy_primitives::Address;

use futures::{Stream, StreamExt};
use reth_exex::ExExContext;
use serde_json::{self, json};
use tracing::{error, info};

pub const SEQ_ADDRESS: &str = "0x197f818c1313DC58b32D88078ecdfB40EA822614";
pub const LAMBDA_ENDPOINT: &str = "https://wvm-lambda-0755acbdae90.herokuapp.com";

fn is_transaction_to_sequencer(to: Address) -> bool {
    let addr_str = std::env::var("SEQUENCER_ADDRESS").unwrap_or(String::from(SEQ_ADDRESS));

    let addr = Address::parse_checksummed(addr_str, None).unwrap();

    to == addr
}

fn process_tx_sequencer(tx: &TransactionSigned) -> Option<String> {
    if let Some(to) = tx.transaction.to() {
        let is_tx_to_seq = is_transaction_to_sequencer(to);
        let is_input_empty = tx.transaction.input().is_empty();
        if is_tx_to_seq && !is_input_empty {
            return Some(tx.hash.to_string())
        }
    }

    None
}

pub async fn exex_lambda_processor<Node: FullNodeComponents>(
    mut ctx: ExExContext<Node>,
) -> eyre::Result<()> {
    let lambda_server = std::env::var("LAMBDA_ENDPOINT").unwrap_or(String::from(LAMBDA_ENDPOINT));
    let mut txs: Vec<String> = vec![];

    while let Some(notification_result) = ctx.notifications.next().await {
        let notification = match notification_result {
            Ok(notification) => notification,
            Err(e) => {
                error!(
                    target: "wvm::exex",
                    %e,
                    "Failed to receive notification from exex stream",
                );
                continue;
            }
        };

        if let Some(committed_chain) = notification.committed_chain() {
            let client = reqwest::Client::new();
            let last_block = committed_chain.tip();

            for tx in last_block.body.transactions.iter() {
                let potential_hash = process_tx_sequencer(tx);
                if let Some(tx_hash) = potential_hash {
                    txs.push(tx_hash);
                }
            }

            let payload = json!({
                "bulk": true,
                "txs": txs
            });

            // TODO: Handle errors
            let _ = client
                .post(format!("{}/tx", lambda_server))
                .json::<serde_json::Value>(&payload)
                .send()
                .await;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::lambda::is_transaction_to_sequencer;
    use alloy_primitives::address;

    #[test]
    fn check_for_seq_address() {
        let to_addr = address!("197f818c1313DC58b32D88078ecdfB40EA822614");
        assert!(is_transaction_to_sequencer(to_addr));
    }
}
