<<<<<<< HEAD
use reth_chainspec::ChainInfo;
use reth_errors::{RethError, RethResult};
use reth_evm::ConfigureEvm;
use reth_network_api::NetworkInfo;
use reth_primitives::{Address, U256, U64};
use reth_provider::{BlockReaderIdExt, ChainSpecProvider, EvmEnvProvider, StateProviderFactory};
use reth_rpc_eth_api::helpers::EthApiSpec;
use reth_rpc_types::{SyncInfo, SyncStatus};
=======
use reth_network_api::NetworkInfo;
use reth_primitives::U256;
use reth_provider::{BlockNumReader, ChainSpecProvider, StageCheckpointReader};
use reth_rpc_eth_api::helpers::EthApiSpec;
>>>>>>> c4b5f5e9c9a88783b2def3ab1cc880b8d41867e1
use reth_transaction_pool::TransactionPool;

use crate::EthApi;

impl<Provider, Pool, Network, EvmConfig> EthApiSpec for EthApi<Provider, Pool, Network, EvmConfig>
where
    Pool: TransactionPool + 'static,
<<<<<<< HEAD
    Provider:
        BlockReaderIdExt + ChainSpecProvider + StateProviderFactory + EvmEnvProvider + 'static,
    Network: NetworkInfo + 'static,
    EvmConfig: ConfigureEvm,
{
    /// Returns the current ethereum protocol version.
    ///
    /// Note: This returns an [`U64`], since this should return as hex string.
    async fn protocol_version(&self) -> RethResult<U64> {
        let status = self.network().network_status().await.map_err(RethError::other)?;
        Ok(U64::from(status.protocol_version))
    }

    /// Returns the chain id
    fn chain_id(&self) -> U64 {
        U64::from(self.network().chain_id())
    }

    /// Returns the current info for the chain
    fn chain_info(&self) -> RethResult<ChainInfo> {
        Ok(self.provider().chain_info()?)
    }

    fn accounts(&self) -> Vec<Address> {
        self.inner.signers().read().iter().flat_map(|s| s.accounts()).collect()
    }

    fn is_syncing(&self) -> bool {
        self.network().is_syncing()
    }

    /// Returns the [`SyncStatus`] of the network
    fn sync_status(&self) -> RethResult<SyncStatus> {
        let status = if self.is_syncing() {
            let current_block = U256::from(
                self.provider().chain_info().map(|info| info.best_number).unwrap_or_default(),
            );
            SyncStatus::Info(SyncInfo {
                starting_block: self.inner.starting_block(),
                current_block,
                highest_block: current_block,
                warp_chunks_amount: None,
                warp_chunks_processed: None,
            })
        } else {
            SyncStatus::None
        };
        Ok(status)
=======
    Provider: ChainSpecProvider + BlockNumReader + StageCheckpointReader + 'static,
    Network: NetworkInfo + 'static,
    EvmConfig: Send + Sync,
{
    fn provider(&self) -> impl ChainSpecProvider + BlockNumReader + StageCheckpointReader {
        self.inner.provider()
    }

    fn network(&self) -> impl NetworkInfo {
        self.inner.network()
    }

    fn starting_block(&self) -> U256 {
        self.inner.starting_block()
    }

    fn signers(&self) -> &parking_lot::RwLock<Vec<Box<dyn reth_rpc_eth_api::helpers::EthSigner>>> {
        self.inner.signers()
>>>>>>> c4b5f5e9c9a88783b2def3ab1cc880b8d41867e1
    }
}
