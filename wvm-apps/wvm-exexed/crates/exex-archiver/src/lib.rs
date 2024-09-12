use eyre::{Error, Report};
use rbrotli::to_brotli;
use reth::api::FullNodeComponents;
use reth::primitives::SealedBlockWithSenders;
use reth_exex::ExExContext;
use wevm_borsh::block::BorshSealedBlockWithSenders;
use async_trait::async_trait;
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

    async fn send_wvm_calldata(&self, block_data: Vec<u8>) -> Result<String, WvmDataSettlerError> {
        send_wvm_calldata(block_data).await.map_err(|_| WvmDataSettlerError::InvalidSendRequest)
    }

    async fn exex<Node: FullNodeComponents>(
        &self,
        mut ctx: ExExContext<Node>
    ) -> eyre::Result<()> {
        while let Some(notification) = ctx.notifications.recv().await {
            if let Some(committed_chain) = notification.committed_chain() {
                let sealed_block_with_senders = committed_chain.tip();
                let block_data = self.process_block(sealed_block_with_senders)?;
                self.send_wvm_calldata(block_data).await.map_err(|e| Report::msg("Invalid Settle Request"))?;
            }
        }

        Ok(())
    }

}

impl WvmDataSettler for DefaultWvmDataSettler {}