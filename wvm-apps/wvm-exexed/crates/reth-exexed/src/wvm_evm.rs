use alloy_primitives::{Address, Bytes, TxKind, U256};
use precompiles;

use parking_lot::RwLock;

use reth_node_api::{ConfigureEvm, ConfigureEvmEnv, NextBlockEnvAttributes};
use reth_revm::{
    handler::register::EvmHandler,
    inspector_handle_register,
    precompile::{
        Precompile, PrecompileResult, PrecompileSpecId, PrecompileWithAddress,
        StatefulPrecompileMut,
    },
    primitives::{CfgEnvWithHandlerCfg, Env, SpecId, TxEnv},
    ContextPrecompile, ContextPrecompiles, Database, Evm, EvmBuilder, GetInspector,
};

use reth::{chainspec::EthereumHardfork, primitives::constants::EIP1559_INITIAL_BASE_FEE};

use reth_chainspec::ChainSpec;
use reth_primitives::{transaction::FillTxEnv, Header, TransactionSigned};
use revm_primitives::{BlobExcessGasAndPrice, BlockEnv, CfgEnv, EnvWithHandlerCfg};

use reth::{chainspec::Head, primitives::revm_primitives::AnalysisKind};
use reth_primitives::constants::ETHEREUM_BLOCK_GAS_LIMIT;
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
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct WvmEthEvmConfig {
    // pub evm_config: EthEvmConfig,
    pub precompile_cache: Arc<RwLock<PrecompileCache>>,
    pub exts: Vec<PrecompileWithAddress>,
    chain_spec: Arc<ChainSpec>,
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
        transaction.fill_tx_env(tx_env, sender);
    }

    fn fill_tx_env_system_contract_call(
        &self,
        env: &mut Env,
        caller: Address,
        contract: Address,
        data: Bytes,
    ) {
        #[allow(clippy::needless_update)] // side-effect of optimism fields
        let tx = TxEnv {
            caller,
            transact_to: TxKind::Call(contract),
            // Explicitly set nonce to None so revm does not do any nonce checks
            nonce: None,
            // WVM: 500_000_000 gas limit
            gas_limit: *ETHEREUM_BLOCK_GAS_LIMIT,
            value: U256::ZERO,
            data,
            // Setting the gas price to zero enforces that no value is transferred as part of the
            // call, and that the call will not count against the block's gas limit
            gas_price: U256::ZERO,
            // The chain ID check is not relevant here and is disabled if set to None
            chain_id: None,
            // Setting the gas priority fee to None ensures the effective gas price is derived from
            // the `gas_price` field, which we need to be zero
            gas_priority_fee: None,
            access_list: Vec::new(),
            // blob fields can be None for this tx
            blob_hashes: Vec::new(),
            max_fee_per_blob_gas: None,
            // TODO remove this once this crate is no longer built with optimism
            ..Default::default()
        };
        env.tx = tx;

        // ensure the block gas limit is >= the tx
        env.block.gas_limit = U256::from(env.tx.gas_limit);

        // disable the base fee check for this call by setting the base fee to zero
        env.block.basefee = U256::ZERO;
    }

    fn fill_cfg_env(
        &self,
        cfg_env: &mut CfgEnvWithHandlerCfg,
        header: &Header,
        total_difficulty: U256,
    ) {
        let spec_id = revm_spec(
            &self.chain_spec,
            &Head {
                number: header.number,
                timestamp: header.timestamp,
                difficulty: header.difficulty,
                total_difficulty,
                hash: Default::default(),
            },
        );

        cfg_env.chain_id = self.chain_spec.chain().id();
        cfg_env.perf_analyse_created_bytecodes = AnalysisKind::Analyse;

        cfg_env.handler_cfg.spec_id = spec_id;
    }

    fn next_cfg_and_block_env(
        &self,
        parent: &Self::Header,
        attributes: NextBlockEnvAttributes,
    ) -> (CfgEnvWithHandlerCfg, BlockEnv) {
        // configure evm env based on parent block
        let cfg = CfgEnv::default().with_chain_id(self.chain_spec.chain().id());

        // ensure we're not missing any timestamp based hardforks
        let spec_id = revm_spec(
            &self.chain_spec,
            &Head {
                number: parent.number + 1,
                timestamp: attributes.timestamp,
                ..Default::default()
            },
        );

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
            self.chain_spec.base_fee_params_at_timestamp(attributes.timestamp),
        );

        let mut gas_limit = U256::from(parent.gas_limit);

        // If we are on the London fork boundary, we need to multiply the parent's gas limit by the
        // elasticity multiplier to get the new gas limit.
        if self.chain_spec.fork(EthereumHardfork::London).transitions_at_block(parent.number + 1) {
            let elasticity_multiplier = self
                .chain_spec
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

    fn fill_block_env(&self, block_env: &mut BlockEnv, header: &Self::Header, after_merge: bool) {
        block_env.number = U256::from(header.number);
        block_env.coinbase = header.beneficiary;
        block_env.timestamp = U256::from(header.timestamp);
        if after_merge {
            block_env.prevrandao = Some(header.mix_hash);
            block_env.difficulty = U256::ZERO;
        } else {
            block_env.difficulty = header.difficulty;
            block_env.prevrandao = None;
        }
        block_env.basefee = U256::from(header.base_fee_per_gas.unwrap_or_default());
        block_env.gas_limit = U256::from(header.gas_limit);

        // EIP-4844 excess blob gas of this block, introduced in Cancun
        if let Some(excess_blob_gas) = header.excess_blob_gas {
            block_env.set_blob_excess_gas_and_price(excess_blob_gas);
        }
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

/// Determine the revm spec ID from the current block and reth chainspec.
fn revm_spec(chain_spec: &ChainSpec, block: &Head) -> reth_revm::primitives::SpecId {
    if chain_spec.fork(EthereumHardfork::Prague).active_at_head(block) {
        reth_revm::primitives::PRAGUE
    } else if chain_spec.fork(EthereumHardfork::Cancun).active_at_head(block) {
        reth_revm::primitives::CANCUN
    } else if chain_spec.fork(EthereumHardfork::Shanghai).active_at_head(block) {
        reth_revm::primitives::SHANGHAI
    } else if chain_spec.fork(EthereumHardfork::Paris).active_at_head(block) {
        reth_revm::primitives::MERGE
    } else if chain_spec.fork(EthereumHardfork::London).active_at_head(block) {
        reth_revm::primitives::LONDON
    } else if chain_spec.fork(EthereumHardfork::Berlin).active_at_head(block) {
        reth_revm::primitives::BERLIN
    } else if chain_spec.fork(EthereumHardfork::Istanbul).active_at_head(block) {
        reth_revm::primitives::ISTANBUL
    } else if chain_spec.fork(EthereumHardfork::Petersburg).active_at_head(block) {
        reth_revm::primitives::PETERSBURG
    } else if chain_spec.fork(EthereumHardfork::Byzantium).active_at_head(block) {
        reth_revm::primitives::BYZANTIUM
    } else if chain_spec.fork(EthereumHardfork::SpuriousDragon).active_at_head(block) {
        reth_revm::primitives::SPURIOUS_DRAGON
    } else if chain_spec.fork(EthereumHardfork::Tangerine).active_at_head(block) {
        reth_revm::primitives::TANGERINE
    } else if chain_spec.fork(EthereumHardfork::Homestead).active_at_head(block) {
        reth_revm::primitives::HOMESTEAD
    } else if chain_spec.fork(EthereumHardfork::Frontier).active_at_head(block) {
        reth_revm::primitives::FRONTIER
    } else {
        panic!(
            "invalid hardfork chainspec: expected at least one hardfork, got {:?}",
            chain_spec.hardforks
        )
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

        Self { chain_spec, precompile_cache, exts }
    }

    /// Sets the precompiles to the EVM handler
    ///
    /// This will be invoked when the EVM is created via [ConfigureEvm::evm] or
    /// [ConfigureEvm::evm_with_inspector]
    ///
    /// This will use the default mainnet precompiles and wrap them with a cache.
    pub fn set_precompiles<EXT, DB, PCI>(
        handler: &mut EvmHandler<'_, EXT, DB>,
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
