mod miscalleneous;

use crate::rpc::private_transaction::miscalleneous::{Builder, BuilderKind};
use alloy_primitives::{Bytes, B256};
use futures::future::join_all;
use jsonrpsee::{
    core::{async_trait, RpcResult},
    proc_macros::rpc,
    tracing::{error, info, warn},
    types::ErrorObject,
};
use reth_primitives::TransactionSigned;
use reth_rpc_eth_types::utils::recover_raw_transaction;
use strum::IntoEnumIterator;

#[derive(Debug, thiserror::Error)]
pub enum PrivateTransactionError {
    #[error("No builders available")]
    FailedToGetBuilders,
    #[error("All builders failed to send tx")]
    AllBuildersFailed,
}

#[derive(Debug, thiserror::Error)]
pub enum TxError {
    #[error("All builders failed to send tx")]
    AllBuildersFailed,
}

#[rpc(server, namespace = "eth")]
#[async_trait]
pub trait EthPrivateTransactionApi {
    #[method(name = "sendPrivateRawTransaction")]
    async fn send_private_raw_transaction(&self, tx: Bytes) -> RpcResult<B256>;
}
pub struct EthPrivateTransaction;

impl From<PrivateTransactionError> for ErrorObject<'_> {
    fn from(error: PrivateTransactionError) -> Self {
        match error {
            PrivateTransactionError::FailedToGetBuilders => {
                ErrorObject::owned(-32000, error.to_string(), None::<()>)
            }
            PrivateTransactionError::AllBuildersFailed => {
                ErrorObject::owned(-32001, error.to_string(), None::<()>)
            }
        }
    }
}

impl EthPrivateTransaction {
    fn builders(&self) -> Vec<Builder> {
        let mut builders = Vec::new();
        for kind in BuilderKind::iter() {
            match kind.builder() {
                Ok(builder) => {
                    info!(target: "builder", "Sending tx to builder: {}", kind);
                    builders.push(builder);
                }
                Err(e) => warn!(target: "builder", "Failed to create builder for {}: {}", kind, e),
            }
        }
        builders
    }

    async fn send_tx_to_builders(&self, tx: Bytes, builders: Vec<Builder>) -> RpcResult<()> {
        let results = join_all(builders.iter().map(|builder| builder.send_tx(tx.clone()))).await;

        if results.iter().all(|r| r.is_err()) {
            return Err(PrivateTransactionError::AllBuildersFailed.into());
        }

        Ok(())
    }
}

#[async_trait]
impl EthPrivateTransactionApiServer for EthPrivateTransaction {
    async fn send_private_raw_transaction(&self, tx: Bytes) -> RpcResult<B256> {
        let builders = self.builders();
        if builders.is_empty() {
            return Err(PrivateTransactionError::FailedToGetBuilders.into());
        }
        let recovered = recover_raw_transaction(tx.clone())?.into_transaction();
        let hash = *recovered.hash();
        self.send_tx_to_builders(tx, builders).await?;
        Ok(hash.into())
    }
}
