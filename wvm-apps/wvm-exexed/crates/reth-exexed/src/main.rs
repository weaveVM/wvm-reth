//! WVM node main

#![doc(issue_tracker_base_url = "https://github.com/weaveVM/wvm-reth/issues/")]

mod constant;
mod network_tag;
mod util;

use crate::{network_tag::get_network_tag, util::check_block_existence};
use arweave_upload::{ArweaveRequest, UploaderProvider};
use exex_wvm_bigquery::{init_bigquery_db, repository::StateRepository, BigQueryConfig};
use exex_wvm_da::{DefaultWvmDataSettler, WvmDataSettler};
use futures::{Stream, StreamExt};
use lambda::lambda::exex_lambda_processor;
use precompiles::node::WvmEthExecutorBuilder;
use reth::{api::FullNodeComponents, args::PruningArgs, builder::NodeBuilder};
use reth_exex::{ExExContext, ExExEvent, ExExNotification};
use reth_node_ethereum::{node::EthereumAddOns, EthereumNode};
use reth_primitives::constants::SLOT_DURATION;
use std::sync::Arc;
use tracing::{error, info};
use wvm_borsh::block::BorshSealedBlockWithSenders;
use wvm_static::WVM_BIGQUERY;

async fn exex_etl_processor<Node: FullNodeComponents>(
    mut ctx: ExExContext<Node>,
    state_repository: StateRepository,
    irys_provider: UploaderProvider,
    _state_processor: exex_etl::state_processor::StateProcessor,
) -> eyre::Result<()> {
    while let Some(notification) = ctx.notifications.poll_next().await {
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
                    committed_chain.tip().number
                );
                continue;
            }
        }

        if let Some(committed_chain) = notification.committed_chain() {
            let data_settler = DefaultWvmDataSettler;
            let sealed_block_with_senders = committed_chain.tip();
            let borsh_sealed_block = BorshSealedBlockWithSenders(sealed_block_with_senders.clone());
            let brotli_borsh = match data_settler.process_block(&borsh_sealed_block) {
                Ok(data) => data,
                Err(err) => {
                    error!(target: "wvm::exex", %err, "Failed to do brotli encoding for block {}", sealed_block_with_senders.number);
                    continue;
                }
            };

            let blk_str_hash = sealed_block_with_senders.block.hash().to_string();
            let block_hash = blk_str_hash.as_str();
            let does_block_exist = check_block_existence(block_hash, false).await;

            if !does_block_exist {
                let res = ArweaveRequest::new()
                    .set_tag("Content-Type", "application/octet-stream")
                    .set_tag("WeaveVM:Encoding", "Borsh-Brotli")
                    .set_tag("Block-Number", sealed_block_with_senders.number.to_string().as_str())
                    .set_tag("Block-Hash", block_hash)
                    .set_tag("Client-Version", reth_primitives::constants::RETH_CLIENT_VERSION)
                    .set_tag("Network", get_network_tag().as_str())
                    .set_tag("WeaveVM:Internal-Chain", notification_type)
                    .set_data(brotli_borsh)
                    .send_with_provider(&irys_provider)
                    .await;

                let arweave_id = match res {
                    Ok(arweave_id) => arweave_id,
                    Err(err) => {
                        error!(target: "wvm::exex", %err, "Failed to construct arweave_id for block {}", sealed_block_with_senders.number);
                        continue;
                    }
                };

                info!(target: "wvm::exex", "irys id: {}, for block: {}", arweave_id, sealed_block_with_senders.number);

                if let Err(err) = exex_wvm_bigquery::save_block(
                    &state_repository,
                    &sealed_block_with_senders,
                    committed_chain.tip().block.number,
                    arweave_id.clone(),
                )
                .await
                {
                    error!(target: "wvm::exex", %err, "Failed to write to bigquery, block {}", sealed_block_with_senders.number);
                    continue;
                };
            }
        }
    }

    Ok(())
}

/// Main loop of the exexed WVM node
fn main() -> eyre::Result<()> {
    reth::cli::Cli::parse_args().run(|builder, _| async move {
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
            let big_query_client = (&*WVM_BIGQUERY).clone();
            // init state repository
            let state_repo = StateRepository::new(big_query_client);
            // init state processor
            let state_processor = exex_etl::state_processor::StateProcessor::new();
            // init irys provider
            let ar_uploader_provider = UploaderProvider::new(None);

             let handle = handle
                .install_exex("exex-etl", |ctx| async move {
                    Ok(exex_etl_processor(ctx, state_repo, ar_uploader_provider, state_processor))
                })
                 .install_exex("exex-lambda", |ctx| async move { Ok(exex_lambda_processor(ctx)) })
                 .launch()
                 .await?;

            handle.wait_for_node_exit().await
        } else {
            let handle = handle.launch().await?;
            handle.wait_for_node_exit().await
        }
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
