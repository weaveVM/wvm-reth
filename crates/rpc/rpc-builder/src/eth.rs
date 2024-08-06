use reth_evm::ConfigureEvm;
use reth_provider::{BlockReader, CanonStateSubscriptions, EvmEnvProvider, StateProviderFactory};
use reth_rpc::{EthFilter, EthPubSub};
use reth_rpc_eth_types::{
    cache::cache_new_blocks_task, EthApiBuilderCtx, EthConfig, EthStateCache,
};
use reth_tasks::TaskSpawner;

/// Alias for `eth` namespace API builder.
pub type DynEthApiBuilder<Provider, Pool, EvmConfig, Network, Tasks, Events, EthApi> =
    Box<dyn Fn(&EthApiBuilderCtx<Provider, Pool, EvmConfig, Network, Tasks, Events>) -> EthApi>;

/// Handlers for core, filter and pubsub `eth` namespace APIs.
#[derive(Debug, Clone)]
pub struct EthHandlers<Provider, Pool, Network, Events, EthApi> {
    /// Main `eth_` request handler
    pub api: EthApi,
    /// The async caching layer used by the eth handlers
    pub cache: EthStateCache,
    /// Polling based filter handler available on all transports
    pub filter: EthFilter<Provider, Pool>,
    /// Handler for subscriptions only available for transports that support it (ws, ipc)
    pub pubsub: EthPubSub<Provider, Pool, Events, Network>,
}

impl<Provider, Pool, Network, Events, EthApi> EthHandlers<Provider, Pool, Network, Events, EthApi> {
    /// Returns a new [`EthHandlers`] builder.
    #[allow(clippy::too_many_arguments)]
    pub fn builder<EvmConfig, Tasks>(
        provider: Provider,
        pool: Pool,
        network: Network,
        evm_config: EvmConfig,
        config: EthConfig,
        executor: Tasks,
        events: Events,
        eth_api_builder: DynEthApiBuilder<
            Provider,
            Pool,
            EvmConfig,
            Network,
            Tasks,
            Events,
            EthApi,
        >,
    ) -> EthHandlersBuilder<Provider, Pool, Network, Tasks, Events, EvmConfig, EthApi> {
        EthHandlersBuilder {
            provider,
            pool,
            network,
            evm_config,
            config,
            executor,
            events,
            eth_api_builder,
        }
    }
}

/// Builds [`EthHandlers`] for core, filter, and pubsub `eth_` apis.
#[allow(missing_debug_implementations)]
pub struct EthHandlersBuilder<Provider, Pool, Network, Tasks, Events, EvmConfig, EthApi> {
    provider: Provider,
    pool: Pool,
    network: Network,
    evm_config: EvmConfig,
    config: EthConfig,
    executor: Tasks,
    events: Events,
    eth_api_builder: DynEthApiBuilder<Provider, Pool, EvmConfig, Network, Tasks, Events, EthApi>,
}

impl<Provider, Pool, Network, Tasks, Events, EvmConfig, EthApi>
    EthHandlersBuilder<Provider, Pool, Network, Tasks, Events, EvmConfig, EthApi>
where
    Provider: StateProviderFactory + BlockReader + EvmEnvProvider + Clone + Unpin + 'static,
    Pool: Send + Sync + Clone + 'static,
    EvmConfig: ConfigureEvm,
    Network: Clone + 'static,
    Tasks: TaskSpawner + Clone + 'static,
    Events: CanonStateSubscriptions + Clone + 'static,
    EthApi: 'static,
{
    /// Returns a new instance with handlers for `eth` namespace.
    pub fn build(self) -> EthHandlers<Provider, Pool, Network, Events, EthApi> {
        let Self { provider, pool, network, evm_config, config, executor, events, eth_api_builder } =
            self;

        let cache = EthStateCache::spawn_with(
            provider.clone(),
            config.cache,
            executor.clone(),
            evm_config.clone(),
        );

        let new_canonical_blocks = events.canonical_state_stream();
        let c = cache.clone();
        executor.spawn_critical(
            "cache canonical blocks task",
            Box::pin(async move {
                cache_new_blocks_task(c, new_canonical_blocks).await;
            }),
        );

        let ctx = EthApiBuilderCtx {
            provider,
            pool,
            network,
            evm_config,
            config,
            executor,
            events,
            cache,
        };

        let api = eth_api_builder(&ctx);

        let filter = EthFilterApiBuilder::build(&ctx);

        let pubsub = EthPubSubApiBuilder::build(&ctx);

        EthHandlers { api, cache: ctx.cache, filter, pubsub }
    }
}

/// Builds the `eth_` namespace API [`EthFilterApiServer`](reth_rpc_eth_api::EthFilterApiServer).
#[derive(Debug)]
pub struct EthFilterApiBuilder;

impl EthFilterApiBuilder {
    /// Builds the [`EthFilterApiServer`](reth_rpc_eth_api::EthFilterApiServer), for given context.
    pub fn build<Provider, Pool, EvmConfig, Network, Tasks, Events>(
        ctx: &EthApiBuilderCtx<Provider, Pool, EvmConfig, Network, Tasks, Events>,
    ) -> EthFilter<Provider, Pool>
    where
        Provider: Send + Sync + Clone + 'static,
        Pool: Send + Sync + Clone + 'static,
        Tasks: TaskSpawner + Clone + 'static,
    {
        EthFilter::new(
            ctx.provider.clone(),
            ctx.pool.clone(),
            ctx.cache.clone(),
            ctx.config.filter_config(),
            Box::new(ctx.executor.clone()),
        )
    }
}

/// Builds the `eth_` namespace API [`EthPubSubApiServer`](reth_rpc_eth_api::EthFilterApiServer).
#[derive(Debug)]
pub struct EthPubSubApiBuilder;

impl EthPubSubApiBuilder {
    /// Builds the [`EthPubSubApiServer`](reth_rpc_eth_api::EthPubSubApiServer), for given context.
    pub fn build<Provider, Pool, EvmConfig, Network, Tasks, Events>(
        ctx: &EthApiBuilderCtx<Provider, Pool, EvmConfig, Network, Tasks, Events>,
    ) -> EthPubSub<Provider, Pool, Events, Network>
    where
        Provider: Clone,
        Pool: Clone,
        Events: Clone,
        Network: Clone,
        Tasks: TaskSpawner + Clone + 'static,
    {
        EthPubSub::with_spawner(
            ctx.provider.clone(),
            ctx.pool.clone(),
            ctx.events.clone(),
            ctx.network.clone(),
            Box::new(ctx.executor.clone()),
        )
    }

    /// Configures the maximum proof window for historical proof generation.
    pub const fn eth_proof_window(mut self, window: u64) -> Self {
        self.eth_proof_window = window;
        self
    }

    /// Configures the number of getproof requests
    pub const fn proof_permits(mut self, permits: usize) -> Self {
        self.proof_permits = permits;
        self
    }
}

/// Context for building the `eth` namespace API.
#[derive(Debug, Clone)]
pub struct EthApiBuilderCtx<Provider, Pool, EvmConfig, Network, Tasks, Events> {
    /// Database handle.
    pub provider: Provider,
    /// Mempool handle.
    pub pool: Pool,
    /// Network handle.
    pub network: Network,
    /// EVM configuration.
    pub evm_config: EvmConfig,
    /// RPC config for `eth` namespace.
    pub config: EthConfig,
    /// Runtime handle.
    pub executor: Tasks,
    /// Events handle.
    pub events: Events,
    /// RPC cache handle.
    pub cache: EthStateCache,
}

/// Ethereum layer one `eth` RPC server builder.
#[derive(Default, Debug, Clone, Copy)]
pub struct EthApiBuild;

impl EthApiBuild {
    /// Builds the [`EthApiServer`](reth_rpc_eth_api::EthApiServer), for given context.
    pub fn build<Provider, Pool, EvmConfig, Network, Tasks, Events>(
        ctx: &EthApiBuilderCtx<Provider, Pool, EvmConfig, Network, Tasks, Events>,
    ) -> EthApi<Provider, Pool, Network, EvmConfig>
    where
        Provider: FullRpcProvider,
        Pool: TransactionPool,
        Network: NetworkInfo + Clone,
        Tasks: TaskSpawner + Clone + 'static,
        Events: CanonStateSubscriptions,
        EvmConfig: ConfigureEvm,
    {
        let gas_oracle = GasPriceOracleBuilder::build(ctx);
        let fee_history_cache = FeeHistoryCacheBuilder::build(ctx);

        EthApi::with_spawner(
            ctx.provider.clone(),
            ctx.pool.clone(),
            ctx.network.clone(),
            ctx.cache.clone(),
            gas_oracle,
            ctx.config.rpc_gas_cap,
            ctx.config.eth_proof_window,
            Box::new(ctx.executor.clone()),
            BlockingTaskPool::build().expect("failed to build blocking task pool"),
            fee_history_cache,
            ctx.evm_config.clone(),
            None,
            ctx.config.proof_permits,
        )
    }
}

/// Builds the `eth_` namespace API [`EthFilterApiServer`](reth_rpc_eth_api::EthFilterApiServer).
#[derive(Debug)]
pub struct EthFilterApiBuilder;

impl EthFilterApiBuilder {
    /// Builds the [`EthFilterApiServer`](reth_rpc_eth_api::EthFilterApiServer), for given context.
    pub fn build<Provider, Pool, EvmConfig, Network, Tasks, Events>(
        ctx: &EthApiBuilderCtx<Provider, Pool, EvmConfig, Network, Tasks, Events>,
    ) -> EthFilter<Provider, Pool>
    where
        Provider: Send + Sync + Clone + 'static,
        Pool: Send + Sync + Clone + 'static,
        Tasks: TaskSpawner + Clone + 'static,
    {
        EthFilter::new(
            ctx.provider.clone(),
            ctx.pool.clone(),
            ctx.cache.clone(),
            ctx.config.filter_config(),
            Box::new(ctx.executor.clone()),
        )
    }
}

/// Builds the `eth_` namespace API [`EthPubSubApiServer`](reth_rpc_eth_api::EthFilterApiServer).
#[derive(Debug)]
pub struct EthPubSubApiBuilder;

impl EthPubSubApiBuilder {
    /// Builds the [`EthPubSubApiServer`](reth_rpc_eth_api::EthPubSubApiServer), for given context.
    pub fn build<Provider, Pool, EvmConfig, Network, Tasks, Events>(
        ctx: &EthApiBuilderCtx<Provider, Pool, EvmConfig, Network, Tasks, Events>,
    ) -> EthPubSub<Provider, Pool, Events, Network>
    where
        Provider: Clone,
        Pool: Clone,
        Events: Clone,
        Network: Clone,
        Tasks: TaskSpawner + Clone + 'static,
    {
        EthPubSub::with_spawner(
            ctx.provider.clone(),
            ctx.pool.clone(),
            ctx.events.clone(),
            ctx.network.clone(),
            Box::new(ctx.executor.clone()),
        )
    }
}

/// Builds `eth_` core api component [`GasPriceOracle`], for given context.
#[derive(Debug)]
pub struct GasPriceOracleBuilder;

impl GasPriceOracleBuilder {
    /// Builds a [`GasPriceOracle`], for given context.
    pub fn build<Provider, Pool, EvmConfig, Network, Tasks, Events>(
        ctx: &EthApiBuilderCtx<Provider, Pool, EvmConfig, Network, Tasks, Events>,
    ) -> GasPriceOracle<Provider>
    where
        Provider: BlockReaderIdExt + Clone,
    {
        GasPriceOracle::new(ctx.provider.clone(), ctx.config.gas_oracle, ctx.cache.clone())
    }
}

/// Builds `eth_` core api component [`FeeHistoryCache`], for given context.
#[derive(Debug)]
pub struct FeeHistoryCacheBuilder;

impl FeeHistoryCacheBuilder {
    /// Builds a [`FeeHistoryCache`], for given context.
    pub fn build<Provider, Pool, EvmConfig, Network, Tasks, Events>(
        ctx: &EthApiBuilderCtx<Provider, Pool, EvmConfig, Network, Tasks, Events>,
    ) -> FeeHistoryCache
    where
        Provider: ChainSpecProvider + BlockReaderIdExt + Clone + 'static,
        Tasks: TaskSpawner,
        Events: CanonStateSubscriptions,
    {
        let fee_history_cache =
            FeeHistoryCache::new(ctx.cache.clone(), ctx.config.fee_history_cache);

        let new_canonical_blocks = ctx.events.canonical_state_stream();
        let fhc = fee_history_cache.clone();
        let provider = ctx.provider.clone();
        ctx.executor.spawn_critical(
            "cache canonical blocks for fee history task",
            Box::pin(async move {
                fee_history_cache_new_blocks_task(fhc, new_canonical_blocks, provider).await;
            }),
        );

        fee_history_cache
    }
}
