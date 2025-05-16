//! This example shows how to implement a node with a custom EVM that uses a stateful precompile

#![warn(unused_crate_dependencies)]

use alloy_evm::{EvmFactory, eth::EthEvmContext};
use alloy_genesis::Genesis;
use alloy_primitives::{Address, Bytes};
use parking_lot::RwLock;
use reth::{
    builder::{BuilderContext, NodeBuilder, components::ExecutorBuilder},
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
        precompile::PrecompileError,
        primitives::hardfork::SpecId,
    },
    tasks::TaskManager,
};
use reth_chainspec::{Chain, ChainSpec};
use reth_evm::{Database, EvmEnv};
use reth_node_api::{FullNodeTypes, NodeTypes};
use reth_node_core::{args::RpcServerArgs, node_config::NodeConfig};
use reth_node_ethereum::{
    BasicBlockExecutorProvider, EthEvmConfig, EthereumNode, evm::EthEvm, node::EthereumAddOns,
};
use reth_primitives::EthPrimitives;
use reth_tracing::{RethTracer, Tracer};
use schnellru::{ByLength, LruMap};
use std::{collections::HashMap, sync::Arc};

/// Type alias for the LRU cache used within the [`PrecompileCache`].
pub type PrecompileLRUCache =
    LruMap<(SpecId, Bytes, u64), Result<InterpreterResult, PrecompileError>>;

/// A cache for precompile inputs / outputs.
///
/// This assumes that the precompile is a standard precompile, as in `StandardPrecompileFn`, meaning
/// its inputs are only `(Bytes, u64)`.
///
/// NOTE: This does not work with "context stateful precompiles", ie `ContextStatefulPrecompile` or
/// `ContextStatefulPrecompileMut`. They are explicitly banned.
#[derive(Debug, Default)]
pub struct PrecompileCache {
    /// Caches for each precompile input / output.
    pub cache: HashMap<Address, PrecompileLRUCache>,
}
