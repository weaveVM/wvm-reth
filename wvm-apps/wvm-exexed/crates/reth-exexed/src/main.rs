//! WVM node main

#![doc(issue_tracker_base_url = "https://github.com/weaveVM/wvm-reth/issues/")]

mod constant;
mod exex;
mod network_tag;
mod util;

use alloy_consensus;
use evm::{self, WvmExecutorBuilder, WvmPayloadBuilder};
use futures::StreamExt;
use lambda::lambda::exex_lambda_processor;
use node;
use reth::{api::FullNodeComponents, args::PruningArgs, builder::NodeBuilder};
use reth_chainspec;
use reth_exex::{ExExContext, ExExEvent, ExExNotification};
use reth_node_ethereum::{node::EthereumAddOns, EthereumNode};
use reth_primitives_traits::{constants::SLOT_DURATION, AlloyBlockHeader};
use std::sync::Arc;
use tracing::{error, info};

use exex::ar_actor::ArweaveActorHandle;
use exex_wvm_bigquery::{BigQueryClient, BigQueryConfig};
use wvm_static::{PRECOMPILE_WVM_BIGQUERY_CLIENT, SUPERVISOR_RT};

async fn exex_etl_processor<Node>(
    mut ctx: ExExContext<Node>,
    ar_actor_handle: Arc<ArweaveActorHandle>,
) -> eyre::Result<()>
where
    Node: FullNodeComponents,
    <Node::Types as reth::api::NodeTypes>::Primitives: reth_primitives_traits::NodePrimitives<
        Block = alloy_consensus::Block<reth_primitives::TransactionSigned>,
    >,
{
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

        let notification_type = match &notification {
            ExExNotification::ChainCommitted { new } => {
                info!(committed_chain = ?new.range(), "Received commit");
                "ChainCommitted"
            }
            ExExNotification::ChainReorged { old, new } => {
                info!(from_chain = ?old.range(), to_chain = ?new.range(), "Received reorg");
                "ChainReorged"
            }
            ExExNotification::ChainReverted { old } => {
                info!(reverted_chain = ?old.range(), "Received revert");
                continue;
            }
        };

        if let Some(committed_chain) = notification.committed_chain() {
            let block = committed_chain.tip().clone();
            let block_number = block.header().number();

            if let Err(err) =
                ctx.events.send(ExExEvent::FinishedHeight(committed_chain.tip().num_hash()))
            {
                error!(
                    target: "wvm::exex",
                    %err,
                    "Failed to send FinishedHeight event for block {}",
                   block_number
                );
                continue;
            }

            if let Err(err) =
                ar_actor_handle.process_block(block, notification_type.to_string()).await
            {
                error!(
                    target: "wvm::exex",
                    %err,
                    block_number,
                    "Failed to send block to arweave actor"
                );
            }
        }
    }

    info!(target: "wvm::exex", "ETL processor shutting down");

    // Add .await here
    if let Err(e) = ar_actor_handle.shutdown().await {
        error!(target: "wvm::exex", %e, "Failed to shutdown ArweaveActor gracefully");
    }
    Ok(())
}

/// Main loop of the exexed WVM node
fn main() -> eyre::Result<()> {
    let _rt = &*SUPERVISOR_RT;
    let _bc = &*PRECOMPILE_WVM_BIGQUERY_CLIENT;

    reth::cli::Cli::parse_args().run(|builder, _| async move {
        // Initializations
        let arweave_actor_buffer_size = std::env::var("ARWEAVE_ACTOR_BUFFER_SIZE")
            .unwrap_or_else(|_| "1024".to_string())
            .parse::<usize>()
            .unwrap_or(1024);

        let ar_actor_handle = Arc::new(
            ArweaveActorHandle::new_parallel(
                std::env::var("ARWEAVE_ACTOR_BUFFER_SIZE")
                    .unwrap_or_else(|_| "1024".to_string())
                    .parse()
                    .unwrap_or(1024),
                std::env::var("ARWEAVE_ACTOR_WORKERS")
                    .unwrap_or_else(|_| "10".to_string())
                    .parse()
                    .unwrap_or(10),
            )
            .await,
        );

        let _init_fee_manager = &*reth_chainspec::WVM_FEE_MANAGER;
        // Original config
        let mut config = builder.config().clone();
        let pruning_args = config.pruning.clone();
        let prune_node = std::env::var("WVM_PRUNE");

        if let Ok(prune_conf) = prune_node {
            config = config.with_pruning(PruningArgs {
                block_interval: Some(parse_prune_config(prune_conf.as_str())),
                ..pruning_args
            });
        }

        let mut handle = NodeBuilder::new(config)
            .with_database(builder.db().clone())
            .with_launch_context(builder.task_executor().clone())
            .with_types::<EthereumNode>()
            .with_components(EthereumNode::components().executor(WvmExecutorBuilder::default()))
            .with_add_ons(EthereumAddOns::default());

        let run_exex = (std::env::var("RUN_EXEX").unwrap_or(String::from("false"))).to_lowercase();
        if run_exex == "true" {
            handle = handle
                .install_exex("exex-etl", |ctx| async move {
                    Ok(exex_etl_processor(ctx, ar_actor_handle))
                })
                .install_exex("exex-lambda", |ctx| async move { Ok(exex_lambda_processor(ctx)) });
        }
        let handle = handle.launch().await?;

        handle.wait_for_node_exit().await
    })
}

fn parse_prune_config(prune_conf: &str) -> u64 {
    let mut d = "";
    if prune_conf == "true" {
        d = "30d"
    } else {
        d = prune_conf;
    }

    let duration = parse_duration::parse(d).unwrap();
    let secs = duration.as_secs();
    SLOT_DURATION.as_secs() * secs
}

async fn new_etl_exex_biguery_client() -> BigQueryClient {
    if let Ok(env_config) = std::env::var("BIGQUERY_CONFIG") {
        info!(target: "wvm::exex", "exex big_query config found in evnironment variable");
        match serde_json::from_str::<BigQueryConfig>(&env_config) {
            Ok(bq_config) => {
                info!(target: "wvm::exex", "etl exex bigquery config parsed from environment variable");
                match BigQueryClient::new(&bq_config).await {
                    Ok(bgc) => {
                        info!(target: "wvm::exex", "etl exex bigquery client initialized from environment variable");
                        return bgc
                    }
                    Err(e) => {
                        panic!("Failed to initialize BigQuery client from environment variable, falling back to file: {e}");
                    }
                }
            }
            Err(e) => {
                panic!( "Failed to parse BIGQUERY_CONFIG environment variable, falling back to file: {e}");
            }
        }
    }

    let config_path: String =
        std::env::var("CONFIG").unwrap_or_else(|_| "./bq-config.json".to_string());

    info!(target: "wvm::exex","etl exex big_query config applied from: {}", config_path);

    let config_file = std::fs::File::open(config_path).expect("bigquery config path exists");
    let reader = std::io::BufReader::new(config_file);

    let bq_config: BigQueryConfig =
        serde_json::from_reader(reader).expect("bigquery config read from file");

    let bgc = BigQueryClient::new(&bq_config).await.unwrap();

    info!(target: "wvm::exex", "etl exex bigquery client initialized");

    bgc
}

#[cfg(test)]
mod tests {
    use crate::parse_prune_config;

    #[test]
    pub fn check_prune_config() {
        let true_prune = parse_prune_config("true");
        assert_eq!(true_prune, 2_592_000);

        let thirty_days_prune = parse_prune_config("30d");
        assert_eq!(thirty_days_prune, 2_592_000);

        let thirty_days_prune = parse_prune_config("15d");
        assert_eq!(thirty_days_prune, 1296000);
    }
}
