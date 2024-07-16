//! WVM node main

#![doc(issue_tracker_base_url = "https://github.com/weaveVM/wvm-reth/issues/")]

use bigquery::client::BigQueryConfig;
use lambda::lambda::exex_lambda_processor;
use repository::state_repository;
use reth::{api::FullNodeComponents, primitives::alloy_primitives::private::serde::Serialize};
use reth_exex::{ExExContext, ExExEvent, ExExNotification};
use reth_node_ethereum::EthereumNode;
use reth_tracing::tracing::info;
use types::types::ExecutionTipState;

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
            state_repository
                .save(ExecutionTipState {
                    block_number: committed_chain.tip().block.number,
                    sealed_block_with_senders: committed_chain.tip().clone(),
                })
                .await?;

            let id = irys_provider.upload_data_to_irys(b"pretend".to_vec()).await?;
            println!("irys id: {}", id);
        }
    }

    Ok(())
}

/// Main loop of the WVM node
fn main() -> eyre::Result<()> {
    reth::cli::Cli::parse_args().run(|builder, _| async move {
        let handle = builder
            .node(EthereumNode::default())
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
                let irys_provider = irys::irys::IrysProvider::new();

                Ok(exex_etl_processor(ctx, state_repo, irys_provider, state_processor))
            })
            .install_exex("exex-lambda", |ctx| async move { Ok(exex_lambda_processor(ctx)) })
            .launch()
            .await?;

        handle.wait_for_node_exit().await
    })
}
