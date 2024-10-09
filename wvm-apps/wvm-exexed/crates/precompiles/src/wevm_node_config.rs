use parking_lot::RwLock;
use reth::{
    api::{ConfigureEvm, ConfigureEvmEnv},
    primitives::{Header, TransactionSigned},
    revm::{
        handler::register::EvmHandler,
        inspector_handle_register,
        precompile::{
            Precompile, PrecompileResult, PrecompileSpecId, PrecompileWithAddress,
            StatefulPrecompileMut,
        },
        primitives::{CfgEnvWithHandlerCfg, Env, SpecId, TxEnv},
        ContextPrecompile, ContextPrecompiles, Database, Evm, EvmBuilder, GetInspector,
    },
};
use alloy_primitives::{Bytes, Address, U256};

use reth_evm_ethereum::{revm_spec_by_timestamp_after_merge};

use reth_chainspec::{ChainSpec};
use reth_node_ethereum::EthEvmConfig;
use revm_primitives::{BlobExcessGasAndPrice, BlockEnv, CfgEnv, EnvWithHandlerCfg};
use schnellru::{ByLength, LruMap};
use std::{collections::HashMap, sync::Arc};
use reth::api::NextBlockEnvAttributes;
use reth::chainspec::EthereumHardfork;
use reth::primitives::constants::EIP1559_INITIAL_BASE_FEE;

/// Type alias for the LRU cache used within the [`PrecompileCache`].
type PrecompileLRUCache = LruMap<(Bytes, u64), PrecompileResult>;

/// Type alias for the thread-safe `Arc<RwLock<_>>` wrapper around [`PrecompileCache`].
type CachedPrecompileResult = Arc<RwLock<PrecompileLRUCache>>;

#[derive(Debug, Default)]
pub struct PrecompileCache {
    /// Caches for each precompile input / output.
    cache: HashMap<(Address, SpecId), CachedPrecompileResult>,
}

/// Ethereum-related EVM configuration.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct WvmEthEvmConfig {
    pub evm_config: EthEvmConfig,
    pub precompile_cache: Arc<RwLock<PrecompileCache>>,
    pub exts: Vec<PrecompileWithAddress>,
}

/// A custom precompile that contains the cache and precompile it wraps.
#[derive(Clone)]
pub struct WrappedPrecompile {
    /// The precompile to wrap.
    precompile: Precompile,
    /// The cache to use.
    cache: Arc<RwLock<LruMap<(Bytes, u64), PrecompileResult>>>,
}

impl ConfigureEvmEnv for WvmEthEvmConfig {
    type Header = Header;

    fn fill_tx_env(&self, tx_env: &mut TxEnv, transaction: &TransactionSigned, sender: Address) {
        self.evm_config.fill_tx_env(tx_env, transaction, sender);
    }

    fn fill_tx_env_system_contract_call(
        &self,
        env: &mut Env,
        caller: Address,
        contract: Address,
        data: Bytes,
    ) {
        self.evm_config.fill_tx_env_system_contract_call(env, caller, contract, data);
    }

    fn fill_cfg_env(
        &self,
        cfg_env: &mut CfgEnvWithHandlerCfg,
        header: &Header,
        total_difficulty: U256,
    ) {
        self.evm_config.fill_cfg_env(cfg_env, header, total_difficulty);
    }


    fn next_cfg_and_block_env(
        &self,
        parent: &Self::Header,
        attributes: NextBlockEnvAttributes,
    ) -> (CfgEnvWithHandlerCfg, BlockEnv) {
        // configure evm env based on parent block
        let cfg = CfgEnv::default().with_chain_id(self.evm_config.chain_spec().chain().id());

        // ensure we're not missing any timestamp based hardforks
        let spec_id = revm_spec_by_timestamp_after_merge(&self.evm_config.chain_spec(), attributes.timestamp);

        // if the parent block did not have excess blob gas (i.e. it was pre-cancun), but it is
        // cancun now, we need to set the excess blob gas to the default value
        let blob_excess_gas_and_price = parent
            .next_block_excess_blob_gas()
            .or_else(|| {
                if spec_id == SpecId::CANCUN {
                    // default excess blob gas is zero
                    Some(0)
                } else {
                    None
                }
            })
            .map(BlobExcessGasAndPrice::new);

        let mut basefee = parent.next_block_base_fee(
            self.evm_config.chain_spec().base_fee_params_at_timestamp(attributes.timestamp),
        );

        let mut gas_limit = U256::from(parent.gas_limit);

        // If we are on the London fork boundary, we need to multiply the parent's gas limit by the
        // elasticity multiplier to get the new gas limit.
        if self.evm_config.chain_spec().fork(EthereumHardfork::London).transitions_at_block(parent.number + 1) {
            let elasticity_multiplier = self
                .evm_config.chain_spec()
                .base_fee_params_at_timestamp(attributes.timestamp)
                .elasticity_multiplier;

            // multiply the gas limit by the elasticity multiplier
            gas_limit *= U256::from(elasticity_multiplier);

            // set the base fee to the initial base fee from the EIP-1559 spec
            basefee = Some(EIP1559_INITIAL_BASE_FEE)
        }

        let block_env = BlockEnv {
            number: U256::from(parent.number + 1),
            coinbase: attributes.suggested_fee_recipient,
            timestamp: U256::from(attributes.timestamp),
            difficulty: U256::ZERO,
            prevrandao: Some(attributes.prev_randao),
            gas_limit,
            // calculate basefee based on parent block's gas usage
            basefee: basefee.map(U256::from).unwrap_or_default(),
            // calculate excess gas based on parent block's blob gas usage
            blob_excess_gas_and_price,
        };

        (CfgEnvWithHandlerCfg::new_with_spec_id(cfg, spec_id), block_env)
    }
}

impl WvmEthEvmConfig {
    pub fn new<PCI>(
        chain_spec: Arc<ChainSpec>,
        precompile_cache: Arc<RwLock<PrecompileCache>>,
        precompiles_ext: PCI,
    ) -> Self
    where
        PCI: Iterator<Item = PrecompileWithAddress>,
    {
        let exts: Vec<PrecompileWithAddress> = precompiles_ext.collect();

        Self { evm_config: EthEvmConfig::new(chain_spec), precompile_cache, exts }
    }

    /// Sets the precompiles to the EVM handler
    ///
    /// This will be invoked when the EVM is created via [ConfigureEvm::evm] or
    /// [ConfigureEvm::evm_with_inspector]
    ///
    /// This will use the default mainnet precompiles and wrap them with a cache.
    pub fn set_precompiles<EXT, DB, PCI>(
        handler: &mut EvmHandler<EXT, DB>,
        cache: Arc<RwLock<PrecompileCache>>,
        extensions: PCI,
    ) where
        DB: Database,
        PCI: Iterator<Item = PrecompileWithAddress>,
    {
        // first we need the evm spec id, which determines the precompiles
        let spec_id = handler.cfg.spec_id;

        let mut loaded_precompiles: ContextPrecompiles<DB> =
            ContextPrecompiles::new(PrecompileSpecId::from_spec_id(spec_id));

        loaded_precompiles.extend(extensions);
        for (address, precompile) in loaded_precompiles.to_mut().iter_mut() {
            // get or insert the cache for this address / spec
            let mut cache = cache.write();
            let cache = cache
                .cache
                .entry((*address, spec_id))
                .or_insert(Arc::new(RwLock::new(LruMap::new(ByLength::new(1024)))));

            *precompile = Self::wrap_precompile(precompile.clone(), cache.clone());
        }

        // install the precompiles
        handler.pre_execution.load_precompiles = Arc::new(move || loaded_precompiles.clone());
    }

    /// Given a [`ContextPrecompile`] and cache for a specific precompile, create a new precompile
    /// that wraps the precompile with the cache.
    fn wrap_precompile<DB>(
        precompile: ContextPrecompile<DB>,
        cache: Arc<RwLock<LruMap<(Bytes, u64), PrecompileResult>>>,
    ) -> ContextPrecompile<DB>
    where
        DB: Database,
    {
        let ContextPrecompile::Ordinary(precompile) = precompile else {
            // context stateful precompiles are not supported, due to lifetime issues or skill
            // issues
            panic!("precompile is not ordinary");
        };

        let wrapped = WrappedPrecompile { precompile, cache: cache.clone() };

        ContextPrecompile::Ordinary(Precompile::StatefulMut(Box::new(wrapped)))
    }
}

impl StatefulPrecompileMut for WrappedPrecompile {
    fn call_mut(&mut self, bytes: &Bytes, gas_price: u64, _env: &Env) -> PrecompileResult {
        let mut cache = self.cache.write();
        let key = (bytes.clone(), gas_price);

        // get the result if it exists
        if let Some(result) = cache.get(&key) {
            return result.clone();
        }

        // call the precompile if cache miss
        let output = self.precompile.call(bytes, gas_price, _env);
        cache.insert(key, output.clone());

        output
    }
}

impl ConfigureEvm for WvmEthEvmConfig {
    type DefaultExternalContext<'a> = ();

    fn evm<DB: Database>(&self, db: DB) -> Evm<'_, Self::DefaultExternalContext<'_>, DB> {
        let precompiles_cache = self.precompile_cache.clone();
        let exts = self.exts.clone().into_iter();

        EvmBuilder::default()
            .with_db(db)
            .append_handler_register_box(Box::new(move |handler| {
                WvmEthEvmConfig::set_precompiles(handler, precompiles_cache.clone(), exts.clone())
            }))
            .build()
    }

    fn evm_with_env_and_inspector<DB, I>(
        &self,
        db: DB,
        env: EnvWithHandlerCfg,
        inspector: I,
    ) -> Evm<'_, I, DB>
    where
        DB: Database,
        I: GetInspector<DB>,
    {
        let precompiles_cache = self.precompile_cache.clone();
        let exts = self.exts.clone().into_iter();

        EvmBuilder::default()
            .with_db(db)
            .with_env(env.env)
            .with_external_context(inspector)
            // add additional precompiles
            .append_handler_register_box(Box::new(move |handler| {
                WvmEthEvmConfig::set_precompiles(handler, precompiles_cache.clone(), exts.clone())
            }))
            .append_handler_register(inspector_handle_register)
            .build()
    }
    fn default_external_context<'a>(&self) -> Self::DefaultExternalContext<'a> {}
}
