//! WVM node main

#![doc(issue_tracker_base_url = "https://github.com/weaveVM/wvm-reth/issues/")]

pub mod util;

use crate::util::to_brotli;
use bigquery::client::BigQueryConfig;
use irys::irys::IrysRequest;
use lambda::lambda::exex_lambda_processor;
use precompiles::node::WvmEthExecutorBuilder;
use repository::state_repository;
use reth::api::FullNodeComponents;
use reth::builder::Node;
use reth_exex::{ExExContext, ExExEvent, ExExNotification};
use reth_node_ethereum::node::EthereumExecutorBuilder;
use reth_node_ethereum::EthereumNode;
use reth_tracing::tracing::info;
use serde_json::to_string;
use types::types::ExecutionTipState;
use wevm_borsh::block::BorshSealedBlockWithSenders;

async fn exex_etl_processor<Node: FullNodeComponents>(
    mut ctx: ExExContext<Node>,
    state_repository: state_repository::StateRepository,
    irys_provider: irys::irys::IrysProvider,
    _state_processor: exex_etl::state_processor::StateProcessor,
) -> eyre::Result<()> {
    while let Some(notification) = ctx.notifications.recv().await {
        match &notification {
            ExExNotification::ChainCommitted { new } => {
                info!(committed_chain = ?new.range(), "Received commit");
            }
            ExExNotification::ChainReorged { old, new } => {
                info!(from_chain = ?old.range(), to_chain = ?new.range(), "Received reorg");
            }
            ExExNotification::ChainReverted { old } => {
                info!(reverted_chain = ?old.range(), "Received revert");
            }
        };

        if let Some(committed_chain) = notification.committed_chain() {
            ctx.events.send(ExExEvent::FinishedHeight(committed_chain.tip().number))?;
        }

        if let Some(committed_chain) = notification.committed_chain() {
            let sealed_block_with_senders = committed_chain.tip();
            let clone_block = BorshSealedBlockWithSenders(sealed_block_with_senders.clone());
            let borsh_data = borsh::to_vec(&clone_block)?;
            let brotli_borsh = to_brotli(borsh_data);
            let json_str = to_string(&sealed_block_with_senders)?;

            let arweave_id = IrysRequest::new()
                .set_tag("Content-Type", "application/octet-stream")
                .set_tag("WeaveVM:Encoding", "Borsh-Brotli")
                .set_tag("Block-Number", sealed_block_with_senders.number.to_string().as_str())
                .set_tag("Block-Hash", sealed_block_with_senders.block.hash().to_string().as_str())
                .set_data(brotli_borsh)
                .send_with_provider(&irys_provider)
                .await?;

            println!("irys id: {}", arweave_id);

            state_repository
                .save(ExecutionTipState {
                    block_number: committed_chain.tip().block.number,
                    arweave_id: arweave_id.clone(),
                    sealed_block_with_senders_serialized: json_str,
                })
                .await?;
        }
    }

    Ok(())
}

/// Main loop of the exexed WVM node
fn main() -> eyre::Result<()> {
    reth::cli::Cli::parse_args().run(|builder, _| async move {
        // let mut builder = builder.with_types::<EthereumNode>();
        // let builder = builder.with_components(EthereumNode::components().executor())
        // let node = EthereumNode::default();
        //
        // let mut handle = builder.node(node);

        let mut handle = builder
            .with_types::<EthereumNode>()
            .with_components(EthereumNode::components().executor(WvmEthExecutorBuilder::default()));

        let run_exex = (std::env::var("RUN_EXEX").unwrap_or(String::from("false"))).to_lowercase();
        if run_exex == "true" {
            handle = handle
                .install_exex("exex-etl", |ctx| async move {
                    let config_path: String =
                        std::env::var("CONFIG").unwrap_or_else(|_| "./bq-config.json".to_string());
                    println!("config: {}", config_path);

                    let config_file =
                        std::fs::File::open(config_path).expect("bigquery config path exists");
                    let reader = std::io::BufReader::new(config_file);

                    let bq_config: BigQueryConfig =
                        serde_json::from_reader(reader).expect("bigquery config read from file");

                    // init bigquery client
                    let bigquery_client = bigquery::client::init_bigquery_db(&bq_config)
                        .await
                        .expect("bigquery client initialized");

                    println!("bigquery client initialized");

                    // init state repository
                    let state_repo = state_repository::StateRepository::new(bigquery_client);
                    // init state processor
                    let state_processor = exex_etl::state_processor::StateProcessor::new();

                    // init irys provider
                    let irys_provider = irys::irys::IrysProvider::new(None);

                    Ok(exex_etl_processor(ctx, state_repo, irys_provider, state_processor))
                })
                .install_exex("exex-lambda", |ctx| async move { Ok(exex_lambda_processor(ctx)) })
        }
        let handle = handle.launch().await?;

        handle.wait_for_node_exit().await
    })
}
