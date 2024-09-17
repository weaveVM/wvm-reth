use crate::{inner::wvm_precompiles, wevm_node_config::WvmEthEvmConfig};
use reth::{
    api::{FullNodeTypes, NodeTypes, PayloadTypes},
    builder::{
        components::{ComponentsBuilder, ExecutorBuilder},
        BuilderContext, Node, NodeTypesWithEngine,
    },
    payload::{EthBuiltPayload, EthPayloadBuilderAttributes},
};
use reth_chainspec::ChainSpec;
use reth_ethereum_engine_primitives::EthPayloadAttributes;
use reth_node_ethereum::{
    node::{
        EthereumAddOns, EthereumConsensusBuilder, EthereumNetworkBuilder, EthereumPayloadBuilder,
        EthereumPoolBuilder,
    },
    EthEngineTypes, EthEvmConfig, EthExecutorProvider,
};

use reth_chainspec::MAINNET;

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
        Node: FullNodeTypes<Types: NodeTypes<ChainSpec = ChainSpec>>,
        <Node::Types as NodeTypesWithEngine>::Engine: PayloadTypes<
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
    type ChainSpec = ChainSpec;
}

/// Configure the node types with the custom engine types
impl NodeTypesWithEngine for WvmEthereumNode {
    type Engine = EthEngineTypes;
}

impl<N> Node<N> for WvmEthereumNode
where
    N: FullNodeTypes<Types: NodeTypesWithEngine<Engine = EthEngineTypes, ChainSpec = ChainSpec>>,
{
    type ComponentsBuilder = ComponentsBuilder<
        N,
        EthereumPoolBuilder,
        EthereumPayloadBuilder,
        EthereumNetworkBuilder,
        WvmEthExecutorBuilder,
        EthereumConsensusBuilder,
    >;
    type AddOns = EthereumAddOns;

    fn components_builder(&self) -> Self::ComponentsBuilder {
        Self::components()
    }
}

/// A regular ethereum evm and executor builder.
#[derive(Debug, Default, Clone, Copy)]
#[non_exhaustive]
pub struct WvmEthExecutorBuilder;

impl<Node> ExecutorBuilder<Node> for WvmEthExecutorBuilder
where
    Node: FullNodeTypes<Types: NodeTypes<ChainSpec = ChainSpec>>,
{
    type EVM = WvmEthEvmConfig;
    type Executor = EthExecutorProvider<Self::EVM>;

    async fn build_evm(
        self,
        ctx: &BuilderContext<Node>,
    ) -> eyre::Result<(Self::EVM, Self::Executor)> {
        let evm_config =
            WvmEthEvmConfig::new(ctx.chain_spec(), Default::default(), wvm_precompiles());
        let executor = EthExecutorProvider::new(ctx.chain_spec(), evm_config.clone());

        Ok((evm_config, executor))
    }
}
