//! OP-Reth `eth_` endpoint implementation.

<<<<<<< HEAD
use alloy_primitives::{Address, U64};
use reth_chainspec::ChainInfo;
use reth_errors::RethResult;
use reth_evm::ConfigureEvm;
use reth_provider::{
    BlockReaderIdExt, ChainSpecProvider, EvmEnvProvider, HeaderProvider, StateProviderFactory,
};
use reth_rpc_eth_api::{
    helpers::{
        Call, EthApiSpec, EthBlocks, EthCall, EthFees, EthSigner, EthState, EthTransactions,
        LoadBlock, LoadFee, LoadPendingBlock, LoadReceipt, LoadState, LoadTransaction,
        SpawnBlocking, Trace,
    },
    RawTransactionForwarder,
};
use reth_rpc_eth_types::{EthStateCache, PendingBlock};
use reth_rpc_types::SyncStatus;
use reth_tasks::{pool::BlockingTaskPool, TaskSpawner};
use reth_transaction_pool::TransactionPool;
use std::future::Future;
use tokio::sync::{AcquireError, Mutex, OwnedSemaphorePermit};
=======
pub mod receipt;
pub mod transaction;

mod block;
mod call;
mod pending_block;

use std::{fmt, sync::Arc};

use alloy_primitives::U256;
use derive_more::Deref;
use reth_evm::ConfigureEvm;
use reth_network_api::NetworkInfo;
use reth_node_api::{BuilderProvider, FullNodeComponents, FullNodeTypes};
use reth_provider::{
    BlockIdReader, BlockNumReader, BlockReaderIdExt, ChainSpecProvider, HeaderProvider,
    StageCheckpointReader, StateProviderFactory,
};
use reth_rpc::eth::{core::EthApiInner, DevSigner};
use reth_rpc_eth_api::{
    helpers::{
        AddDevSigners, EthApiSpec, EthFees, EthState, LoadBlock, LoadFee, LoadState, SpawnBlocking,
        Trace,
    },
    EthApiTypes,
};
use reth_rpc_eth_types::{EthStateCache, FeeHistoryCache, GasPriceOracle};
use reth_tasks::{
    pool::{BlockingTaskGuard, BlockingTaskPool},
    TaskExecutor, TaskSpawner,
};
use reth_transaction_pool::TransactionPool;

use crate::OpEthApiError;

/// Adapter for [`EthApiInner`], which holds all the data required to serve core `eth_` API.
pub type EthApiNodeBackend<N> = EthApiInner<
    <N as FullNodeTypes>::Provider,
    <N as FullNodeComponents>::Pool,
    <N as FullNodeComponents>::Network,
    <N as FullNodeComponents>::Evm,
>;

/// Adapter for [`EthApiBuilderCtx`].
pub type EthApiBuilderCtx<N> = reth_rpc_eth_types::EthApiBuilderCtx<
    <N as FullNodeTypes>::Provider,
    <N as FullNodeComponents>::Pool,
    <N as FullNodeComponents>::Evm,
    <N as FullNodeComponents>::Network,
    TaskExecutor,
    <N as FullNodeTypes>::Provider,
>;
>>>>>>> upstream/main

/// OP-Reth `Eth` API implementation.
///
/// This type provides the functionality for handling `eth_` related requests.
///
/// This wraps a default `Eth` implementation, and provides additional functionality where the
/// optimism spec deviates from the default (ethereum) spec, e.g. transaction forwarding to the
/// sequencer, receipts, additional RPC fields for transaction receipts.
///
/// This type implements the [`FullEthApi`](reth_rpc_eth_api::helpers::FullEthApi) by implemented
/// all the `Eth` helper traits and prerequisite traits.
<<<<<<< HEAD
#[derive(Debug, Clone)]
pub struct OpEthApi<Eth> {
    inner: Eth,
}

impl<Eth> OpEthApi<Eth> {
    /// Creates a new `OpEthApi` from the provided `Eth` implementation.
    pub const fn new(inner: Eth) -> Self {
        Self { inner }
    }
}

impl<Eth: EthApiSpec> EthApiSpec for OpEthApi<Eth> {
    fn protocol_version(&self) -> impl Future<Output = RethResult<U64>> + Send {
        self.inner.protocol_version()
    }

    fn chain_id(&self) -> U64 {
        self.inner.chain_id()
    }

    fn chain_info(&self) -> RethResult<ChainInfo> {
        self.inner.chain_info()
    }

    fn accounts(&self) -> Vec<Address> {
        self.inner.accounts()
    }

    fn is_syncing(&self) -> bool {
        self.inner.is_syncing()
    }

    fn sync_status(&self) -> RethResult<SyncStatus> {
        self.inner.sync_status()
    }
}

impl<Eth: LoadBlock> LoadBlock for OpEthApi<Eth> {
    fn provider(&self) -> impl BlockReaderIdExt {
        LoadBlock::provider(&self.inner)
    }

    fn cache(&self) -> &reth_rpc_eth_types::EthStateCache {
        self.inner.cache()
    }
}

impl<Eth: LoadPendingBlock> LoadPendingBlock for OpEthApi<Eth> {
    fn provider(
        &self,
    ) -> impl BlockReaderIdExt + EvmEnvProvider + ChainSpecProvider + StateProviderFactory {
        self.inner.provider()
    }

    fn pool(&self) -> impl TransactionPool {
        self.inner.pool()
    }

    fn pending_block(&self) -> &Mutex<Option<PendingBlock>> {
        self.inner.pending_block()
    }

    fn evm_config(&self) -> &impl ConfigureEvm {
        self.inner.evm_config()
    }
}

impl<Eth: SpawnBlocking> SpawnBlocking for OpEthApi<Eth> {
    fn io_task_spawner(&self) -> impl TaskSpawner {
        self.inner.io_task_spawner()
    }

    fn tracing_task_pool(&self) -> &BlockingTaskPool {
        self.inner.tracing_task_pool()
    }

    fn acquire_owned(
        &self,
    ) -> impl Future<Output = Result<OwnedSemaphorePermit, AcquireError>> + Send {
        self.inner.acquire_owned()
    }

    fn acquire_many_owned(
        &self,
        n: u32,
    ) -> impl Future<Output = Result<OwnedSemaphorePermit, AcquireError>> + Send {
        self.inner.acquire_many_owned(n)
    }
}

impl<Eth: LoadReceipt> LoadReceipt for OpEthApi<Eth> {
    fn cache(&self) -> &EthStateCache {
        self.inner.cache()
    }
}

impl<Eth: LoadFee> LoadFee for OpEthApi<Eth> {
    fn provider(&self) -> impl reth_provider::BlockIdReader + HeaderProvider + ChainSpecProvider {
        LoadFee::provider(&self.inner)
    }

    fn cache(&self) -> &EthStateCache {
        LoadFee::cache(&self.inner)
    }

    fn gas_oracle(&self) -> &reth_rpc_eth_types::GasPriceOracle<impl BlockReaderIdExt> {
        self.inner.gas_oracle()
    }

    fn fee_history_cache(&self) -> &reth_rpc_eth_types::FeeHistoryCache {
        self.inner.fee_history_cache()
    }
}

impl<Eth: Call> Call for OpEthApi<Eth> {
    fn call_gas_limit(&self) -> u64 {
        self.inner.call_gas_limit()
    }

    fn evm_config(&self) -> &impl ConfigureEvm {
        self.inner.evm_config()
    }
}

impl<Eth: LoadState> LoadState for OpEthApi<Eth> {
    fn provider(&self) -> impl StateProviderFactory {
        LoadState::provider(&self.inner)
    }

    fn cache(&self) -> &EthStateCache {
        LoadState::cache(&self.inner)
    }

    fn pool(&self) -> impl TransactionPool {
        LoadState::pool(&self.inner)
    }
}

impl<Eth: LoadTransaction> LoadTransaction for OpEthApi<Eth> {
    type Pool = Eth::Pool;

    fn provider(&self) -> impl reth_provider::TransactionsProvider {
        LoadTransaction::provider(&self.inner)
    }

    fn cache(&self) -> &EthStateCache {
        LoadTransaction::cache(&self.inner)
    }

    fn pool(&self) -> &Self::Pool {
        LoadTransaction::pool(&self.inner)
    }
}

impl<Eth: EthTransactions> EthTransactions for OpEthApi<Eth> {
    fn provider(&self) -> impl BlockReaderIdExt {
        EthTransactions::provider(&self.inner)
    }

    fn raw_tx_forwarder(&self) -> Option<std::sync::Arc<dyn RawTransactionForwarder>> {
        self.inner.raw_tx_forwarder()
    }

    fn signers(&self) -> &parking_lot::RwLock<Vec<Box<dyn EthSigner>>> {
=======
#[derive(Clone, Deref)]
pub struct OpEthApi<N: FullNodeComponents> {
    inner: Arc<EthApiNodeBackend<N>>,
}

impl<N: FullNodeComponents> OpEthApi<N> {
    /// Creates a new instance for given context.
    #[allow(clippy::type_complexity)]
    pub fn with_spawner(ctx: &EthApiBuilderCtx<N>) -> Self {
        let blocking_task_pool =
            BlockingTaskPool::build().expect("failed to build blocking task pool");

        let inner = EthApiInner::new(
            ctx.provider.clone(),
            ctx.pool.clone(),
            ctx.network.clone(),
            ctx.cache.clone(),
            ctx.new_gas_price_oracle(),
            ctx.config.rpc_gas_cap,
            ctx.config.eth_proof_window,
            blocking_task_pool,
            ctx.new_fee_history_cache(),
            ctx.evm_config.clone(),
            ctx.executor.clone(),
            None,
            ctx.config.proof_permits,
        );

        Self { inner: Arc::new(inner) }
    }
}

impl<N> EthApiTypes for OpEthApi<N>
where
    Self: Send + Sync,
    N: FullNodeComponents,
{
    type Error = OpEthApiError;
}

impl<N> EthApiSpec for OpEthApi<N>
where
    N: FullNodeComponents,
{
    #[inline]
    fn provider(&self) -> impl ChainSpecProvider + BlockNumReader + StageCheckpointReader {
        self.inner.provider()
    }

    #[inline]
    fn network(&self) -> impl NetworkInfo {
        self.inner.network()
    }

    #[inline]
    fn starting_block(&self) -> U256 {
        self.inner.starting_block()
    }

    #[inline]
    fn signers(&self) -> &parking_lot::RwLock<Vec<Box<dyn reth_rpc_eth_api::helpers::EthSigner>>> {
>>>>>>> upstream/main
        self.inner.signers()
    }
}

<<<<<<< HEAD
impl<Eth: EthBlocks> EthBlocks for OpEthApi<Eth> {
    fn provider(&self) -> impl HeaderProvider {
        EthBlocks::provider(&self.inner)
    }
}

impl<Eth: EthState> EthState for OpEthApi<Eth> {
    fn max_proof_window(&self) -> u64 {
        self.inner.max_proof_window()
    }
}

impl<Eth: EthCall> EthCall for OpEthApi<Eth> {}

impl<Eth: EthFees> EthFees for OpEthApi<Eth> {}

impl<Eth: Trace> Trace for OpEthApi<Eth> {
=======
impl<N> SpawnBlocking for OpEthApi<N>
where
    Self: Send + Sync + Clone + 'static,
    N: FullNodeComponents,
{
    #[inline]
    fn io_task_spawner(&self) -> impl TaskSpawner {
        self.inner.task_spawner()
    }

    #[inline]
    fn tracing_task_pool(&self) -> &BlockingTaskPool {
        self.inner.blocking_task_pool()
    }

    #[inline]
    fn tracing_task_guard(&self) -> &BlockingTaskGuard {
        self.inner.blocking_task_guard()
    }
}

impl<N> LoadFee for OpEthApi<N>
where
    Self: LoadBlock,
    N: FullNodeComponents,
{
    #[inline]
    fn provider(&self) -> impl BlockIdReader + HeaderProvider + ChainSpecProvider {
        self.inner.provider()
    }

    #[inline]
    fn cache(&self) -> &EthStateCache {
        self.inner.cache()
    }

    #[inline]
    fn gas_oracle(&self) -> &GasPriceOracle<impl BlockReaderIdExt> {
        self.inner.gas_oracle()
    }

    #[inline]
    fn fee_history_cache(&self) -> &FeeHistoryCache {
        self.inner.fee_history_cache()
    }
}

impl<N> LoadState for OpEthApi<N>
where
    Self: Send + Sync,
    N: FullNodeComponents,
{
    #[inline]
    fn provider(&self) -> impl StateProviderFactory + ChainSpecProvider {
        self.inner.provider()
    }

    #[inline]
    fn cache(&self) -> &EthStateCache {
        self.inner.cache()
    }

    #[inline]
    fn pool(&self) -> impl TransactionPool {
        self.inner.pool()
    }
}

impl<N> EthState for OpEthApi<N>
where
    Self: LoadState + SpawnBlocking,
    N: FullNodeComponents,
{
    #[inline]
    fn max_proof_window(&self) -> u64 {
        self.inner.eth_proof_window()
    }
}

impl<N> EthFees for OpEthApi<N>
where
    Self: LoadFee,
    N: FullNodeComponents,
{
}

impl<N> Trace for OpEthApi<N>
where
    Self: LoadState,
    N: FullNodeComponents,
{
    #[inline]
>>>>>>> upstream/main
    fn evm_config(&self) -> &impl ConfigureEvm {
        self.inner.evm_config()
    }
}
<<<<<<< HEAD
=======

impl<N: FullNodeComponents> AddDevSigners for OpEthApi<N> {
    fn with_dev_accounts(&self) {
        *self.signers().write() = DevSigner::random_signers(20)
    }
}

impl<N> BuilderProvider<N> for OpEthApi<N>
where
    N: FullNodeComponents,
{
    type Ctx<'a> = &'a EthApiBuilderCtx<N>;

    fn builder() -> Box<dyn for<'a> Fn(Self::Ctx<'a>) -> Self + Send> {
        Box::new(|ctx| Self::with_spawner(ctx))
    }
}

impl<N: FullNodeComponents> fmt::Debug for OpEthApi<N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OpEthApi").finish_non_exhaustive()
    }
}
>>>>>>> upstream/main
