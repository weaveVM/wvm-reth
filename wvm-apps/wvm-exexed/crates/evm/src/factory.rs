use reth::{api::ConfigureEvm, revm::context::BlockEnv};
use reth_ethereum_payload_builder;
use std::sync::Arc;

use alloy_evm::{EvmFactory, eth::EthEvmContext};
use alloy_genesis::Genesis;
use alloy_primitives::{Address, Bytes, address};
use reth::{
    builder::{
        BuilderContext, NodeBuilder,
        components::{BasicPayloadServiceBuilder, ExecutorBuilder, PayloadBuilderBuilder},
    },
    payload::{EthBuiltPayload, EthPayloadBuilderAttributes},
    revm::{
        MainBuilder, MainContext,
        context::{Cfg, Context, TxEnv},
        context_interface::{
            ContextTr,
            result::{EVMError, HaltReason},
        },
        handler::{EthPrecompiles, PrecompileProvider},
        inspector::{Inspector, NoOpInspector},
        interpreter::{InterpreterResult, interpreter::EthInterpreter},
        precompile::{
            PrecompileError, PrecompileFn, PrecompileOutput, PrecompileResult, Precompiles,
        },
        primitives::hardfork::SpecId,
    },
    rpc::types::engine::PayloadAttributes,
    tasks::TaskManager,
    transaction_pool::{PoolTransaction, TransactionPool},
};
use reth_chainspec::{Chain, ChainSpec};
use reth_evm::{Database, EvmEnv};
use reth_evm_ethereum::{EthEvm, EthEvmConfig};
use reth_node_api::{FullNodeTypes, NodeTypes, NodeTypesWithEngine, PayloadTypes};
use reth_node_core::{args::RpcServerArgs, node_config::NodeConfig};
use reth_node_ethereum::{
    BasicBlockExecutorProvider, EthereumNode,
    node::{EthereumAddOns, EthereumPayloadBuilder},
};
use reth_primitives::{EthPrimitives, TransactionSigned};
use reth_tracing::{RethTracer, Tracer};
use std::sync::OnceLock;

/// Custom EVM configuration.
#[derive(Debug, Clone, Default)]
#[non_exhaustive]
pub struct WvmEvmFactory;

impl EvmFactory for WvmEvmFactory {
    type Evm<DB: Database, I: Inspector<EthEvmContext<DB>, EthInterpreter>> =
        EthEvm<DB, I, WvmPrecompiles>;
    type Tx = TxEnv;
    type Error<DBError: core::error::Error + Send + Sync + 'static> = EVMError<DBError>;
    type HaltReason = HaltReason;
    type Context<DB: Database> = EthEvmContext<DB>;
    type Spec = SpecId;

    fn create_evm<DB: Database>(&self, db: DB, input: EvmEnv) -> Self::Evm<DB, NoOpInspector> {
        let evm = Context::mainnet()
            .with_db(db)
            .with_cfg(input.cfg_env)
            .with_block(input.block_env)
            .build_mainnet_with_inspector(NoOpInspector {})
            .with_precompiles(WvmPrecompiles::new());

        EthEvm::new(evm, false)
    }

    fn create_evm_with_inspector<DB: Database, I: Inspector<Self::Context<DB>, EthInterpreter>>(
        &self,
        db: DB,
        input: EvmEnv,
        inspector: I,
    ) -> Self::Evm<DB, I> {
        EthEvm::new(self.create_evm(db, input).into_inner().with_inspector(inspector), true)
    }
}

/// Builds a regular ethereum block executor that uses the custom EVM.
#[derive(Debug, Default, Clone, Copy)]
#[non_exhaustive]
pub struct WvmExecutorBuilder;

impl<Node> ExecutorBuilder<Node> for WvmExecutorBuilder
where
    Node: FullNodeTypes<Types: NodeTypes<ChainSpec = ChainSpec, Primitives = EthPrimitives>>,
{
    type EVM = EthEvmConfig<WvmEvmFactory>;
    type Executor = BasicBlockExecutorProvider<Self::EVM>;

    async fn build_evm(
        self,
        ctx: &BuilderContext<Node>,
    ) -> eyre::Result<(Self::EVM, Self::Executor)> {
        let evm_config =
            EthEvmConfig::new_with_evm_factory(ctx.chain_spec(), WvmEvmFactory::default());
        Ok((evm_config.clone(), BasicBlockExecutorProvider::new(evm_config)))
    }
}

/// Builds a regular ethereum block executor that uses the custom EVM.
#[derive(Debug, Default, Clone)]
#[non_exhaustive]
pub struct WvmPayloadBuilder {
    inner: EthereumPayloadBuilder,
}

impl<Types, Node, Pool> PayloadBuilderBuilder<Node, Pool> for WvmPayloadBuilder
where
    Types: NodeTypesWithEngine<ChainSpec = ChainSpec, Primitives = EthPrimitives>,
    Node: FullNodeTypes<Types = Types>,
    Pool: TransactionPool<Transaction: PoolTransaction<Consensus = TransactionSigned>>
        + Unpin
        + 'static,
    Types::Engine: PayloadTypes<
            BuiltPayload = EthBuiltPayload,
            PayloadAttributes = PayloadAttributes,
            PayloadBuilderAttributes = EthPayloadBuilderAttributes,
        >,
{
    type PayloadBuilder = reth_ethereum_payload_builder::EthereumPayloadBuilder<
        Pool,
        Node::Provider,
        EthEvmConfig<WvmEvmFactory>,
    >;

    async fn build_payload_builder(
        self,
        ctx: &BuilderContext<Node>,
        pool: Pool,
    ) -> eyre::Result<Self::PayloadBuilder> {
        let evm_config =
            EthEvmConfig::new_with_evm_factory(ctx.chain_spec(), WvmEvmFactory::default());
        self.inner.build(evm_config, ctx, pool)
    }
}

/// A custom precompile that contains static precompiles.
#[derive(Clone)]
pub struct WvmPrecompiles {
    pub precompiles: EthPrecompiles,
}

impl WvmPrecompiles {
    /// Given a [`PrecompileProvider`] and cache for a specific precompiles, create a
    /// wrapper that can be used inside Evm.
    fn new() -> Self {
        Self { precompiles: EthPrecompiles::default() }
    }
}

/// Returns precompiles for Fjor spec.
pub fn prague_custom() -> &'static Precompiles {
    static INSTANCE: OnceLock<Precompiles> = OnceLock::new();
    INSTANCE.get_or_init(|| {
        let mut precompiles = Precompiles::prague().clone();
        // Custom precompile.
        precompiles.extend([(
            address!("0x0000000000000000000000000000000000000999"),
            |_, _| -> PrecompileResult {
                PrecompileResult::Ok(PrecompileOutput::new(0, Bytes::new()))
            } as PrecompileFn,
        )
            .into()]);
        precompiles
    })
}

impl<CTX: ContextTr> PrecompileProvider<CTX> for WvmPrecompiles {
    type Output = InterpreterResult;

    fn set_spec(&mut self, spec: <CTX::Cfg as Cfg>::Spec) {
        let spec_id = spec.clone().into();
        if spec_id == SpecId::PRAGUE {
            self.precompiles = EthPrecompiles { precompiles: prague_custom() }
        } else {
            PrecompileProvider::<CTX>::set_spec(&mut self.precompiles, spec);
        }
    }

    fn run(
        &mut self,
        context: &mut CTX,
        address: &Address,
        bytes: &Bytes,
        gas_limit: u64,
    ) -> Result<Option<Self::Output>, PrecompileError> {
        self.precompiles.run(context, address, bytes, gas_limit)
    }

    fn contains(&self, address: &Address) -> bool {
        self.precompiles.contains(address)
    }

    fn warm_addresses(&self) -> Box<impl Iterator<Item = Address>> {
        self.precompiles.warm_addresses()
    }
}
