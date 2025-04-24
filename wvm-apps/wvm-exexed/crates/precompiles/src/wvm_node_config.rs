use alloy_consensus::{BlockHeader, Header};
use alloy_evm::eth::EthBlockExecutionCtx;
use alloy_primitives::{Address, Bytes, U256};
extern crate alloc;

use alloc::{borrow::Cow, sync::Arc};

pub use config::{revm_spec, revm_spec_by_timestamp_and_block_number};
use parking_lot::RwLock;
use reth::{
    api::{ConfigureEvm, NodeTypesWithEngine},
    revm::{
        handler::{EthPrecompiles, PrecompileProvider},
        precompile::{PrecompileResult, PrecompileSpecId, PrecompileWithAddress},
        primitives::hardfork::SpecId,
    },
};
use reth_evm_ethereum::config;
use reth_primitives_traits::{SealedBlock, SealedHeader};

use reth_chainspec::{EthChainSpec, MAINNET};
use reth_ethereum_primitives::{Block, EthPrimitives, Receipt, TransactionSigned};

use alloy_evm::{eth::EthEvmContext, EthEvmFactory, EvmFactory, FromRecoveredTx};
use reth::{
    builder::{components::ExecutorBuilder, BuilderContext, NodeBuilder},
    revm::{
        context::{BlockEnv, Cfg, CfgEnv},
        context_interface::{block::BlobExcessGasAndPrice, ContextTr},
        interpreter::InterpreterResult,
        precompile::PrecompileError,
        MainBuilder, MainContext,
    },
    tasks::TaskManager,
};
use reth_evm::{Database, EvmEnv, InspectorFor, TransactionEnv};

use crate::node::WvmEthExecutorBuilder;
use alloy_eips::eip1559::INITIAL_BASE_FEE;
use alloy_evm::{
    block::{BlockExecutorFactory, BlockExecutorFor},
    eth::{EthBlockExecutor, EthBlockExecutorFactory},
};
use reth::{
    api::{NextBlockEnvAttributes, NodePrimitives},
    chainspec::EthereumHardfork,
    revm::{
        context::{
            result::{EVMError, HaltReason},
            TxEnv,
        },
        inspector::NoOpInspector,
        interpreter::interpreter::EthInterpreter,
        Context, Inspector, State,
    },
};
use reth_chainspec::ChainSpec;
use reth_evm_ethereum::{EthBlockAssembler, EthEvm, RethReceiptBuilder};
use reth_node_ethereum::EthEvmConfig;
use schnellru::{ByLength, LruMap};
use std::{collections::HashMap, convert::Infallible};

/// Type alias for the LRU cache used within the [`PrecompileCache`].
type PrecompileLRUCache = LruMap<(SpecId, Bytes, u64), Result<InterpreterResult, PrecompileError>>;
type WrappedEthEvm<DB, I> = EthEvm<DB, I, WrappedPrecompile<EthPrecompiles>>;

/// Type alias for the thread-safe `Arc<RwLock<_>>` wrapper around [`PrecompileCache`].
type CachedPrecompileResult = Arc<RwLock<PrecompileLRUCache>>;

#[derive(Debug, Default)]
pub struct PrecompileCache {
    /// Caches for each precompile input / output.
    cache: HashMap<Address, PrecompileLRUCache>,
}

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
    fn new(precompile: P, cache: Arc<RwLock<PrecompileCache>>) -> Self {
        WrappedPrecompile { precompile, cache: cache.clone(), spec: SpecId::LATEST }
    }
}

/// Custom EVM factory.
#[derive(Debug, Clone, Default)]
#[non_exhaustive]
pub struct WvmEvmFactory {
    precompile_cache: Arc<RwLock<PrecompileCache>>,
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

        let evm = Context::mainnet()
            .with_db(db)
            .with_cfg(input.cfg_env)
            .with_block(input.block_env)
            .build_mainnet_with_inspector(NoOpInspector {})
            .with_precompiles(WrappedPrecompile::new(EthPrecompiles::default(), new_cache));

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

        let output = self.precompile.run(context, address, bytes, gas_limit);

        if let Some(output) = output.clone().transpose() {
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

//
// impl ConfigureEvmEnv for WvmEthEvmConfig {
//     type Header = Header;
//     type Error = Infallible;
//
//     fn fill_tx_env(&self, tx_env: &mut TxEnv, transaction: &TransactionSigned, sender: Address) {
//         self.evm_config.fill_tx_env(tx_env, transaction, sender);
//     }
//
//     fn fill_tx_env_system_contract_call(
//         &self,
//         env: &mut Env,
//         caller: Address,
//         contract: Address,
//         data: Bytes,
//     ) {
//         self.evm_config.fill_tx_env_system_contract_call(env, caller, contract, data);
//     }
//
//     fn fill_cfg_env(
//         &self,
//         cfg_env: &mut CfgEnvWithHandlerCfg,
//         header: &Header,
//         total_difficulty: U256,
//     ) {
//         self.evm_config.fill_cfg_env(cfg_env, header, total_difficulty);
//     }
//
//     fn next_cfg_and_block_env(
//         &self,
//         parent: &Self::Header,
//         attributes: NextBlockEnvAttributes,
//     ) -> Result<(CfgEnvWithHandlerCfg, BlockEnv), Infallible> {
//         // configure evm env based on parent block
//         let cfg = CfgEnv::default().with_chain_id(self.evm_config.chain_spec().chain().id());
//
//         // ensure we're not missing any timestamp based hardforks
//         let spec_id =
//             revm_spec_by_timestamp_after_merge(&self.evm_config.chain_spec(),
// attributes.timestamp);
//
//         // if the parent block did not have excess blob gas (i.e. it was pre-cancun), but it is
//         // cancun now, we need to set the excess blob gas to the default value
//         let blob_excess_gas_and_price = parent
//             .next_block_excess_blob_gas()
//             .or_else(|| {
//                 if spec_id == SpecId::CANCUN {
//                     // default excess blob gas is zero
//                     Some(0)
//                 } else {
//                     None
//                 }
//             })
//             .map(BlobExcessGasAndPrice::new);
//
//         let mut basefee = parent.next_block_base_fee(
//             self.evm_config.chain_spec().base_fee_params_at_timestamp(attributes.timestamp),
//         );
//
//         let mut gas_limit = U256::from(parent.gas_limit);
//
//         // If we are on the London fork boundary, we need to multiply the parent's gas limit by
// the         // elasticity multiplier to get the new gas limit.
//         if self
//             .evm_config
//             .chain_spec()
//             .fork(EthereumHardfork::London)
//             .transitions_at_block(parent.number + 1)
//         {
//             let elasticity_multiplier = self
//                 .evm_config
//                 .chain_spec()
//                 .base_fee_params_at_timestamp(attributes.timestamp)
//                 .elasticity_multiplier;
//
//             // multiply the gas limit by the elasticity multiplier
//             gas_limit *= U256::from(elasticity_multiplier);
//
//             // set the base fee to the initial base fee from the EIP-1559 spec
//             basefee = Some(EIP1559_INITIAL_BASE_FEE)
//         }
//
//         let block_env = BlockEnv {
//             number: U256::from(parent.number + 1),
//             coinbase: attributes.suggested_fee_recipient,
//             timestamp: U256::from(attributes.timestamp),
//             difficulty: U256::ZERO,
//             prevrandao: Some(attributes.prev_randao),
//             gas_limit,
//             // calculate basefee based on parent block's gas usage
//             basefee: basefee.map(U256::from).unwrap_or_default(),
//             // calculate excess gas based on parent block's blob gas usage
//             blob_excess_gas_and_price,
//         };
//
//         Ok((CfgEnvWithHandlerCfg::new_with_spec_id(cfg, spec_id), block_env))
//     }
// }

/// Ethereum-related EVM configuration.
#[derive(Debug, Clone)]
pub struct WvmEthEvmConfig<EvmFactory = EthEvmFactory> {
    inner: EthEvmConfig,
}

pub struct CustomBlockExecutor<'a, Evm> {
    /// Inner Ethereum execution strategy.
    inner: WvmEthExecutorBuilder<'a, Evm, &'a Arc<ChainSpec>, &'a RethReceiptBuilder>,
}

impl BlockExecutorFactory for WvmEthEvmConfig {
    type EvmFactory = EthEvmFactory;
    type ExecutionCtx<'a> = EthBlockExecutionCtx<'a>;
    type Transaction = TransactionSigned;
    type Receipt = Receipt;

    fn evm_factory(&self) -> &Self::EvmFactory {
        self.inner.evm_factory()
    }

    fn create_executor<'a, DB, I>(
        &'a self,
        evm: EthEvm<&'a mut State<DB>, I>,
        ctx: EthBlockExecutionCtx<'a>,
    ) -> impl BlockExecutorFor<'a, Self, DB, I>
    where
        DB: Database + 'a,
        I: InspectorFor<Self, &'a mut State<DB>> + 'a,
    {
        BlockE {
            inner: EthBlockExecutor::new(
                evm,
                ctx,
                self.inner.chain_spec(),
                self.inner.executor_factory.receipt_builder(),
            ),
        }
    }
}

impl ConfigureEvm for WvmEthEvmConfig {
    type Primitives = <EthEvmConfig as ConfigureEvm>::Primitives;
    type Error = <EthEvmConfig as ConfigureEvm>::Error;
    type NextBlockEnvCtx = <EthEvmConfig as ConfigureEvm>::NextBlockEnvCtx;
    type BlockExecutorFactory = Self;
    type BlockAssembler = EthBlockAssembler<ChainSpec>;

    fn block_executor_factory(&self) -> &Self::BlockExecutorFactory {
        &self
    }

    fn block_assembler(&self) -> &Self::BlockAssembler {
        &self.block_assembler
    }

    fn evm_env(&self, header: &Header) -> EvmEnv {
        let spec = config::revm_spec(self.chain_spec(), header);

        // configure evm env based on parent block
        let cfg_env = CfgEnv::new().with_chain_id(self.chain_spec().chain().id()).with_spec(spec);

        let block_env = BlockEnv {
            number: header.number(),
            beneficiary: header.beneficiary(),
            timestamp: header.timestamp(),
            difficulty: if spec >= SpecId::MERGE { U256::ZERO } else { header.difficulty() },
            prevrandao: if spec >= SpecId::MERGE { header.mix_hash() } else { None },
            gas_limit: header.gas_limit(),
            basefee: header.base_fee_per_gas().unwrap_or_default(),
            // EIP-4844 excess blob gas of this block, introduced in Cancun
            blob_excess_gas_and_price: header.excess_blob_gas.map(|excess_blob_gas| {
                BlobExcessGasAndPrice::new(excess_blob_gas, spec >= SpecId::PRAGUE)
            }),
        };

        EvmEnv { cfg_env, block_env }
    }

    fn next_evm_env(
        &self,
        parent: &Header,
        attributes: &NextBlockEnvAttributes,
    ) -> Result<EvmEnv, Self::Error> {
        // ensure we're not missing any timestamp based hardforks
        let spec_id = revm_spec_by_timestamp_and_block_number(
            self.chain_spec(),
            attributes.timestamp,
            parent.number() + 1,
        );

        // configure evm env based on parent block
        let cfg = CfgEnv::new().with_chain_id(self.chain_spec().chain().id()).with_spec(spec_id);

        // if the parent block did not have excess blob gas (i.e. it was pre-cancun), but it is
        // cancun now, we need to set the excess blob gas to the default value(0)
        let blob_excess_gas_and_price = parent
            .maybe_next_block_excess_blob_gas(
                self.chain_spec().blob_params_at_timestamp(attributes.timestamp),
            )
            .or_else(|| (spec_id == SpecId::CANCUN).then_some(0))
            .map(|gas| BlobExcessGasAndPrice::new(gas, spec_id >= SpecId::PRAGUE));

        let mut basefee = parent.next_block_base_fee(
            self.chain_spec().base_fee_params_at_timestamp(attributes.timestamp),
        );

        let mut gas_limit = attributes.gas_limit;

        // If we are on the London fork boundary, we need to multiply the parent's gas limit by the
        // elasticity multiplier to get the new gas limit.
        if self.chain_spec().fork(EthereumHardfork::London).transitions_at_block(parent.number + 1)
        {
            let elasticity_multiplier = self
                .chain_spec()
                .base_fee_params_at_timestamp(attributes.timestamp)
                .elasticity_multiplier;

            // multiply the gas limit by the elasticity multiplier
            gas_limit *= elasticity_multiplier as u64;

            // set the base fee to the initial base fee from the EIP-1559 spec
            basefee = Some(INITIAL_BASE_FEE)
        }

        let block_env = BlockEnv {
            number: parent.number + 1,
            beneficiary: attributes.suggested_fee_recipient,
            timestamp: attributes.timestamp,
            difficulty: U256::ZERO,
            prevrandao: Some(attributes.prev_randao),
            gas_limit,
            // calculate basefee based on parent block's gas usage
            basefee: basefee.unwrap_or_default(),
            // calculate excess gas based on parent block's blob gas usage
            blob_excess_gas_and_price,
        };

        Ok((cfg, block_env).into())
    }

    fn context_for_block<'a>(&self, block: &'a SealedBlock<Block>) -> EthBlockExecutionCtx<'a> {
        EthBlockExecutionCtx {
            parent_hash: block.header().parent_hash,
            parent_beacon_block_root: block.header().parent_beacon_block_root,
            ommers: &block.body().ommers,
            withdrawals: block.body().withdrawals.as_ref().map(Cow::Borrowed),
        }
    }

    fn context_for_next_block(
        &self,
        parent: &SealedHeader,
        attributes: Self::NextBlockEnvCtx,
    ) -> EthBlockExecutionCtx<'_> {
        EthBlockExecutionCtx {
            parent_hash: parent.hash(),
            parent_beacon_block_root: attributes.parent_beacon_block_root,
            ommers: &[],
            withdrawals: attributes.withdrawals.map(Cow::Owned),
        }
    }
}
