use async_trait::async_trait;
use eyre::{Error, Report};
use rbrotli::to_brotli;
use reth::{api::FullNodeComponents, primitives::SealedBlockWithSenders};
use reth_exex::ExExContext;
use wevm_borsh::block::BorshSealedBlockWithSenders;
use wvm_archiver::utils::transaction::send_wvm_calldata;

pub struct DefaultWvmDataSettler;

pub enum WvmDataSettlerError {
    InvalidSendRequest,
}

#[async_trait]
pub trait WvmDataSettler {
    fn process_block(&self, data: &SealedBlockWithSenders) -> Result<Vec<u8>, Error> {
        let clone_block = BorshSealedBlockWithSenders(data.clone());
        let borsh_data = borsh::to_vec(&clone_block)?;
        let brotli_borsh = to_brotli(borsh_data);
        Ok(brotli_borsh)
    }

    async fn send_wvm_calldata(
        &mut self,
        block_data: Vec<u8>,
    ) -> Result<String, WvmDataSettlerError> {
        send_wvm_calldata(block_data).await.map_err(|_| WvmDataSettlerError::InvalidSendRequest)
    }

    async fn exex<Node: FullNodeComponents>(
        &mut self,
        mut ctx: ExExContext<Node>,
    ) -> eyre::Result<()> {
        while let Some(notification) = ctx.notifications.recv().await {
            if let Some(committed_chain) = notification.committed_chain() {
                let sealed_block_with_senders = committed_chain.tip();
                let block_data = self.process_block(sealed_block_with_senders)?;
                self.send_wvm_calldata(block_data)
                    .await
                    .map_err(|e| Report::msg("Invalid Settle Request"))?;
            }
        }

        Ok(())
    }
}

impl WvmDataSettler for DefaultWvmDataSettler {}

#[cfg(test)]
mod tests {
    use crate::{WvmDataSettler, WvmDataSettlerError};
    use async_trait::async_trait;
    use reth::providers::Chain;
    use reth_exex::{ExExContext, ExExNotification};
    use reth_exex_test_utils::test_exex_context;
    use reth_node_ethereum::EthereumNode;
    use std::sync::{Arc, RwLock};
    use tokio::sync::{mpsc, mpsc::unbounded_channel};

    #[tokio::test]
    pub async fn test_wvm_da() {
        struct TestWvmDa {
            called: bool,
        }

        #[async_trait]
        impl WvmDataSettler for TestWvmDa {
            async fn send_wvm_calldata(
                &mut self,
                block_data: Vec<u8>,
            ) -> Result<String, WvmDataSettlerError> {
                self.called = true;
                Ok("hello world".to_string())
            }
        }

        let context = test_exex_context().await.unwrap();

        let chain_def = Chain::from_block(Default::default(), Default::default(), None);

        context
            .1
            .notifications_tx
            .send(ExExNotification::ChainCommitted { new: Arc::new(chain_def) })
            .await
            .unwrap();

        let mut wvm_da = TestWvmDa { called: false };

        drop(context.1);

        wvm_da.exex(context.0).await.unwrap();

        assert!(wvm_da.called);
    }
}
