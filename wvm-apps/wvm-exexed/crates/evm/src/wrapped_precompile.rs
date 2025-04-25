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
use std::{collections::HashMap, sync::Arc};
