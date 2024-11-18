//! WVM node main

#![doc(issue_tracker_base_url = "https://github.com/weaveVM/wvm-reth/issues/")]

mod constant;
mod exex;
mod network_tag;
mod util;

use crate::{network_tag::get_network_tag, util::check_block_existence};
use arweave_upload::{ArweaveRequest, UploaderProvider};
use exex_wvm_bigquery::repository::StateRepository;
use exex_wvm_da::{DefaultWvmDataSettler, WvmDataSettler};
use futures::StreamExt;
use lambda::lambda::exex_lambda_processor;
use precompiles::node::WvmEthExecutorBuilder;
use reth::{api::FullNodeComponents, args::PruningArgs, builder::NodeBuilder};
use reth_exex::{ExExContext, ExExEvent, ExExNotification};
use reth_node_ethereum::{node::EthereumAddOns, EthereumNode};
use reth_primitives::constants::SLOT_DURATION;
use std::sync::Arc;
use tracing::{error, info};
use wvm_borsh::block::BorshSealedBlockWithSenders;

async fn exex_etl_processor<Node: FullNodeComponents>(
    mut ctx: ExExContext<Node>,
    ar_process: Arc<ArProcess>,
) -> eyre::Result<()> {
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

        let mut notification_type = "";
        match &notification {
            ExExNotification::ChainCommitted { new } => {
                notification_type = "ChainCommitted";
                info!(committed_chain = ?new.range(), "Received commit");
            }
            ExExNotification::ChainReorged { old, new } => {
                notification_type = "ChainReorged";
                info!(from_chain = ?old.range(), to_chain = ?new.range(), "Received reorg");
            }
            ExExNotification::ChainReverted { old } => {
                info!(reverted_chain = ?old.range(), "Received revert");
            }
        };

        if let Some(committed_chain) = notification.committed_chain() {
            if let Err(err) =
                ctx.events.send(ExExEvent::FinishedHeight(committed_chain.tip().num_hash()))
            {
                error!(
                    target: "wvm::exex",
                    %err,
                    "Failed to send FinishedHeight event for block {}",
                    committed_chain.tip().block.header.header().number
                );
                continue;
            }
        }

        if let Some(committed_chain) = notification.committed_chain() {
            let sealed_block_with_senders = committed_chain.tip().clone();
            let _ =
                ar_process.sender.send((sealed_block_with_senders, notification_type.to_string()));
            // Handle recovery if `receiver` is dropped
        }
    }

    Ok(())
}

/// Main loop of the exexed WVM node
fn main() -> eyre::Result<()> {
    let _rt = &*SUPERVISOR_RT;
    let _bc = &*PRECOMPILE_WVM_BIGQUERY_CLIENT;
    let ar_process = Arc::new(ArProcess::new(10));

    reth::cli::Cli::parse_args().run(|builder, _| async move {
        // Initializations
        let _init_fee_manager = &*reth_primitives::constants::WVM_FEE_MANAGER;
        // Original config
        let mut config = builder.config().clone();
        let pruning_args = config.pruning.clone();
        let prune_node = std::env::var("WVM_PRUNE");

        if let Ok(prune_conf) = prune_node {
            config = config.with_pruning(PruningArgs {
                block_interval: parse_prune_config(prune_conf.as_str()),
                ..pruning_args
            });
        }

        let mut handle = NodeBuilder::new(config)
            .with_database(builder.db().clone())
            .with_launch_context(builder.task_executor().clone())
            .with_types::<EthereumNode>()
            .with_components(EthereumNode::components().executor(WvmEthExecutorBuilder::default()))
            .with_add_ons(EthereumAddOns::default());

        let run_exex = (std::env::var("RUN_EXEX").unwrap_or(String::from("false"))).to_lowercase();
        if run_exex == "true" {
            handle = handle
                .install_exex("exex-etl", |ctx| async move {
                    Ok(exex_etl_processor(ctx, ar_process.clone()))
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

use crate::exex::ar_process::ArProcess;
use exex_wvm_bigquery::{BigQueryClient, BigQueryConfig};
use wvm_static::{PRECOMPILE_WVM_BIGQUERY_CLIENT, SUPERVISOR_RT};

async fn new_etl_exex_biguery_client() -> BigQueryClient {
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
