use parking_lot::RwLock;
use reth::{
    api::{ConfigureEvm, ConfigureEvmEnv},
    primitives::{Address, Bytes, Header, TransactionSigned, U256},
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
use reth_chainspec::ChainSpec;
use reth_node_ethereum::EthEvmConfig;
use schnellru::{ByLength, LruMap};
use std::{collections::HashMap, sync::Arc};

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
#[derive(Debug, Clone, Default)]
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
        chain_spec: &ChainSpec,
        header: &Header,
        total_difficulty: U256,
    ) {
        self.evm_config.fill_cfg_env(cfg_env, chain_spec, header, total_difficulty);
    }
}

impl WvmEthEvmConfig {
    pub fn new<PCI>(
        evm_config: EthEvmConfig,
        precompile_cache: Arc<RwLock<PrecompileCache>>,
        precompiles_ext: PCI,
    ) -> Self
    where
        PCI: Iterator<Item = PrecompileWithAddress>,
    {
        let exts: Vec<PrecompileWithAddress> = precompiles_ext.collect();

        Self { evm_config, precompile_cache, exts }
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

    fn evm<'a, DB: Database + 'a>(
        &self,
        db: DB,
    ) -> reth_revm::Evm<'a, Self::DefaultExternalContext<'a>, DB> {
        let precompiles_cache = self.precompile_cache.clone();
        let exts = self.exts.clone().into_iter();

        EvmBuilder::default()
            .with_db(db)
            .append_handler_register_box(Box::new(move |handler| {
                WvmEthEvmConfig::set_precompiles(handler, precompiles_cache.clone(), exts.clone())
            }))
            .build()
    }

    fn evm_with_inspector<'a, DB, I>(&self, db: DB, inspector: I) -> Evm<'a, I, DB>
    where
        DB: Database + 'a,
        I: GetInspector<DB>,
    {
        let precompiles_cache = self.precompile_cache.clone();
        let exts = self.exts.clone().into_iter();

        EvmBuilder::default()
            .with_db(db)
            .with_external_context(inspector)
            // add additional precompiles
            .append_handler_register_box(Box::new(move |handler| {
                WvmEthEvmConfig::set_precompiles(handler, precompiles_cache.clone(), exts.clone())
            }))
            .append_handler_register(inspector_handle_register)
            .build()
    }
}
