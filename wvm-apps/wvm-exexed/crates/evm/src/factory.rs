use reth::{api::ConfigureEvm, revm::context::BlockEnv};
use reth_ethereum_payload_builder;
use std::sync::Arc;

use alloy_evm::{EvmFactory, eth::EthEvmContext};
use alloy_genesis::Genesis;
use alloy_primitives::{Address, Bytes, address};
use parking_lot::RwLock;
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
use schnellru::{ByLength, LruMap};
use std::sync::OnceLock;

use precompiles::inner::wvm_precompiles;

type WrappedEthEvm<DB, I> = EthEvm<DB, I, WrappedPrecompile<EthPrecompiles>>;

/// Returns precompiles with WVM additions.
pub fn wvm_enhanced_precompiles() -> &'static Precompiles {
    static INSTANCE: OnceLock<Precompiles> = OnceLock::new();
    INSTANCE.get_or_init(|| {
        // Clone the latest standard precompiles
        let mut precompiles = Precompiles::latest().clone();

        // Add WVM precompiles
        precompiles.extend(wvm_precompiles());

        precompiles
    })
}

/// Custom EVM configuration.
#[derive(Debug, Clone, Default)]
#[non_exhaustive]
pub struct WvmEvmFactory {
    precompile_cache: Arc<RwLock<super::cache::PrecompileCache>>,
}

impl EvmFactory for WvmEvmFactory {
    type Evm<DB: Database, I: Inspector<EthEvmContext<DB>, EthInterpreter>> = WrappedEthEvm<DB, I>;

    type Tx = TxEnv;
    type Error<DBError: core::error::Error + Send + Sync + 'static> = EVMError<DBError>;
    type HaltReason = HaltReason;
    type Context<DB: Database> = EthEvmContext<DB>;
    type Spec = SpecId;

    fn create_evm<DB: Database>(&self, db: DB, input: EvmEnv) -> Self::Evm<DB, NoOpInspector> {
        let new_cache = self.precompile_cache.clone();

        let enchanced_precompiles = EthPrecompiles { precompiles: wvm_enhanced_precompiles() };
        let evm = Context::mainnet()
            .with_db(db)
            .with_cfg(input.cfg_env)
            .with_block(input.block_env)
            .build_mainnet_with_inspector(NoOpInspector {})
            .with_precompiles(WrappedPrecompile::new(enchanced_precompiles, new_cache));

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
#[derive(Debug, Default, Clone)]
#[non_exhaustive]
pub struct WvmExecutorBuilder {
    /// The precompile cache to use for all executors.
    precompile_cache: Arc<RwLock<super::cache::PrecompileCache>>,
}

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
        let evm_config = EthEvmConfig::new_with_evm_factory(
            ctx.chain_spec(),
            WvmEvmFactory { precompile_cache: self.precompile_cache.clone() },
        );

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

use super::cache::{PrecompileCache, PrecompileLRUCache};

/// A custom precompile that contains the cache and precompile it wraps.
#[derive(Clone)]
pub struct WrappedPrecompile<P> {
    /// The precompile to wrap.
    precompile: P,
    /// The cache to use.
    cache: Arc<RwLock<PrecompileCache>>,
    /// The spec id to use.
    spec: SpecId,
}

impl<P> WrappedPrecompile<P> {
    /// Given a [`PrecompileProvider`] and cache for a specific precompiles, create a
    /// wrapper that can be used inside Evm.
    pub fn new(precompile: P, cache: Arc<RwLock<PrecompileCache>>) -> Self {
        WrappedPrecompile { precompile, cache: cache.clone(), spec: SpecId::LATEST }
    }
}

impl<CTX: ContextTr, P: PrecompileProvider<CTX, Output = InterpreterResult>> PrecompileProvider<CTX>
    for WrappedPrecompile<P>
{
    type Output = P::Output;

    fn set_spec(&mut self, spec: <CTX::Cfg as Cfg>::Spec) {
        self.precompile.set_spec(spec.clone());
        self.spec = spec.into();
    }

    fn run(
        &mut self,
        context: &mut CTX,
        address: &Address,
        bytes: &Bytes,
        gas_limit: u64,
    ) -> Result<Option<Self::Output>, PrecompileError> {
        let mut cache = self.cache.write();
        let key = (self.spec, bytes.clone(), gas_limit);

        // get the result if it exists
        if let Some(precompiles) = cache.cache.get_mut(address) {
            if let Some(result) = precompiles.get(&key) {
                return result.clone().map(Some)
            }
        }

        // call the precompile if cache miss
        let output = self.precompile.run(context, address, bytes, gas_limit);

        if let Some(output) = output.clone().transpose() {
            // insert the result into the cache
            cache
                .cache
                .entry(*address)
                .or_insert(PrecompileLRUCache::new(ByLength::new(1024)))
                .insert(key, output);
        }

        output
    }

    fn contains(&self, address: &Address) -> bool {
        self.precompile.contains(address)
    }

    fn warm_addresses(&self) -> Box<impl Iterator<Item = Address>> {
        self.precompile.warm_addresses()
    }
}
