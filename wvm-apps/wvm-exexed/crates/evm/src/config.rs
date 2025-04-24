use reth::{api::ConfigureEvm, revm::context::BlockEnv};
use std::sync::Arc;

use alloy_evm::{EvmFactory, eth::EthEvmContext};
use alloy_genesis::Genesis;
use alloy_primitives::{Address, Bytes, address};
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
use std::sync::OnceLock;

#[derive(Debug, Clone)]
pub struct WvmEvmConfig {
    pub inner: EthEvmConfig,
}

impl WvmEvmConfig {
    pub fn new(chain_spec: Arc<ChainSpec>) -> Self {
        Self { inner: EthEvmConfig::new(chain_spec) }
    }
}
