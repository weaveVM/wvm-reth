use reth::api::{FullNodeTypes, NodeTypes, PayloadTypes};
use reth::builder::{BuilderContext, Node};
use reth::builder::components::{ComponentsBuilder, ExecutorBuilder};
use reth::payload::{EthBuiltPayload, EthPayloadBuilderAttributes};
use reth_ethereum_engine_primitives::EthPayloadAttributes;
use reth_node_ethereum::{EthEngineTypes, EthEvmConfig, EthExecutorProvider};
use reth_node_ethereum::node::{EthereumConsensusBuilder, EthereumNetworkBuilder, EthereumPayloadBuilder, EthereumPoolBuilder};
use crate::inner::wvm_precompiles;
use crate::wevm_node_config::WvmEthEvmConfig;

/// Type configuration for a regular Ethereum node.
#[derive(Debug, Default, Clone, Copy)]
#[non_exhaustive]
pub struct WvmEthereumNode;

impl WvmEthereumNode {
    /// Returns a [`ComponentsBuilder`] configured for a regular Ethereum node.
    pub fn components<Node>() -> ComponentsBuilder<
        Node,
        EthereumPoolBuilder,
        EthereumPayloadBuilder,
        EthereumNetworkBuilder,
        WvmEthExecutorBuilder,
        EthereumConsensusBuilder,
    >
        where
            Node: FullNodeTypes,
            <Node as NodeTypes>::Engine: PayloadTypes<
                BuiltPayload = EthBuiltPayload,
                PayloadAttributes = EthPayloadAttributes,
                PayloadBuilderAttributes = EthPayloadBuilderAttributes,
            >,
    {
        ComponentsBuilder::default()
            .node_types::<Node>()
            .pool(EthereumPoolBuilder::default())
            .payload(EthereumPayloadBuilder::default())
            .network(EthereumNetworkBuilder::default())
            .executor(WvmEthExecutorBuilder::default())
            .consensus(EthereumConsensusBuilder::default())
    }
}

impl NodeTypes for WvmEthereumNode {
    type Primitives = ();
    type Engine = EthEngineTypes;
}

impl<N> Node<N> for WvmEthereumNode
    where
        N: FullNodeTypes<Engine = EthEngineTypes>,
{
    type ComponentsBuilder = ComponentsBuilder<
        N,
        EthereumPoolBuilder,
        EthereumPayloadBuilder,
        EthereumNetworkBuilder,
        WvmEthExecutorBuilder,
        EthereumConsensusBuilder,
    >;

    fn components_builder(self) -> Self::ComponentsBuilder {
        Self::components()
    }
}

/// A regular ethereum evm and executor builder.
#[derive(Debug, Default, Clone, Copy)]
#[non_exhaustive]
pub struct WvmEthExecutorBuilder;

impl<Node> ExecutorBuilder<Node> for WvmEthExecutorBuilder
    where
        Node: FullNodeTypes,
{
    type EVM = WvmEthEvmConfig;
    type Executor = EthExecutorProvider<Self::EVM>;

    async fn build_evm(
        self,
        ctx: &BuilderContext<Node>,
    ) -> eyre::Result<(Self::EVM, Self::Executor)> {
        let chain_spec = ctx.chain_spec();
        let evm_config = WvmEthEvmConfig::new(
            EthEvmConfig::default(),
            Default::default(),
            wvm_precompiles()
        );
        let executor = EthExecutorProvider::new(chain_spec, evm_config.clone());

        Ok((evm_config, executor))
    }
}