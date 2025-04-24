use crate::{inner::wvm_precompiles, wvm_node_config::WvmEthEvmConfig};
use reth::{
    api::{FullNodeTypes, NodeTypes, PayloadTypes},
    builder::{
        components::{ComponentsBuilder, ExecutorBuilder},
        BuilderContext, Node, NodeTypesWithDB, NodeTypesWithEngine,
    },
    payload::{EthBuiltPayload, EthPayloadBuilderAttributes},
    providers::EthStorage,
};
use reth_chainspec::ChainSpec;
use reth_ethereum_engine_primitives::EthPayloadAttributes;
use reth_evm_ethereum::{execute::EthExecutorProvider, EthEvmConfig};
use reth_node_builder::{
    components::BasicPayloadServiceBuilder, NodeAdapter, NodeComponentsBuilder,
};
use reth_node_ethereum::{
    node::{
        EthPrimitives, EthereumAddOns, EthereumConsensusBuilder, EthereumExecutorBuilder,
        EthereumNetworkBuilder, EthereumPayloadBuilder, EthereumPoolBuilder,
    },
    BasicBlockExecutorProvider, EthEngineTypes, EthereumNode,
};
use reth_trie_db::MerklePatriciaTrie;

/// Type configuration for a regular Ethereum node.
#[derive(Debug, Default, Clone, Copy)]
#[non_exhaustive]
pub struct WvmEthereumNode;

impl WvmEthereumNode {
    pub fn components<Node>() -> ComponentsBuilder<
        Node,
        EthereumPoolBuilder,
        BasicPayloadServiceBuilder<EthereumPayloadBuilder>,
        EthereumNetworkBuilder,
        WvmEthExecutorBuilder,
        EthereumConsensusBuilder,
    >
    where
        Node: FullNodeTypes<Types: NodeTypes<ChainSpec = ChainSpec, Primitives = EthPrimitives>>,
        <Node::Types as NodeTypesWithEngine>::Engine: PayloadTypes<
            BuiltPayload = EthBuiltPayload,
            PayloadAttributes = EthPayloadAttributes,
            PayloadBuilderAttributes = EthPayloadBuilderAttributes,
        >,
    {
        ComponentsBuilder::default()
            .node_types::<Node>()
            .pool(EthereumPoolBuilder::default())
            .payload(BasicPayloadServiceBuilder::default())
            .network(EthereumNetworkBuilder::default())
            .executor(WvmEthExecutorBuilder::default())
            .consensus(EthereumConsensusBuilder::default())
    }
}

impl NodeTypes for WvmEthereumNode {
    type Primitives = EthPrimitives;
    type ChainSpec = ChainSpec;
    type StateCommitment = MerklePatriciaTrie;
    type Storage = EthStorage;
}

/// Configure the node types with the custom engine types
impl NodeTypesWithEngine for WvmEthereumNode {
    type Engine = EthEngineTypes;
}

impl<N> Node<N> for WvmEthereumNode
where
    N: FullNodeTypes<Types = Self>,
{
    type ComponentsBuilder = ComponentsBuilder<
        N,
        EthereumPoolBuilder,
        BasicPayloadServiceBuilder<EthereumPayloadBuilder>,
        EthereumNetworkBuilder,
        WvmEthExecutorBuilder,
        EthereumConsensusBuilder,
    >;

    type AddOns = EthereumAddOns<
        NodeAdapter<N, <Self::ComponentsBuilder as NodeComponentsBuilder<N>>::Components>,
    >;

    fn components_builder(&self) -> Self::ComponentsBuilder {
        Self::components()
    }

    fn add_ons(&self) -> Self::AddOns {
        EthereumAddOns::default()
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
    type Executor = BasicBlockExecutorProvider<Self::EVM>;

    async fn build_evm(
        self,
        ctx: &BuilderContext<Node>,
    ) -> eyre::Result<(Self::EVM, Self::Executor)> {
        // let evm_config =
        //     WvmEthEvmConfig::new(ctx.chain_spec(), Default::default(), wvm_precompiles());
        let evm_config = WvmEthEvmConfig { inner: EthEvmConfig::new(ctx.chain_spec()) };
        let executor = BasicBlockExecutorProvider::new(evm_config.clone());

        Ok((evm_config.clone(), executor))
    }
}

impl<Types, Node> ExecutorBuilder<Node> for CustomExecutorBuilder
where
    Types: NodeTypesWithEngine<ChainSpec = ChainSpec, Primitives = EthPrimitives>,
    Node: FullNodeTypes<Types = Types>,
{
    type EVM = CustomEvmConfig;
    type Executor = BasicBlockExecutorProvider<Self::EVM>;

    async fn build_evm(
        self,
        ctx: &BuilderContext<Node>,
    ) -> eyre::Result<(Self::EVM, Self::Executor)> {
        let evm_config = CustomEvmConfig { inner: EthEvmConfig::new(ctx.chain_spec()) };
        let executor = BasicBlockExecutorProvider::new(evm_config.clone());

        Ok((evm_config, executor))
    }
}
