use crate::segments::{dataset_for_compression, prepare_jar, Segment};
use reth_db::{static_file::create_static_file_T1, tables};
use reth_db_api::{cursor::DbCursorRO, database::Database, transaction::DbTx};
use reth_primitives::{
    static_file::{SegmentConfig, SegmentHeader},
    BlockNumber, StaticFileSegment, TxNumber,
};
use reth_provider::{
    providers::{StaticFileProvider, StaticFileWriter},
    BlockReader, DatabaseProviderRO, TransactionsProviderExt,
};
use reth_storage_errors::provider::{ProviderError, ProviderResult};
use std::{ops::RangeInclusive, path::Path};

/// Static File segment responsible for [`StaticFileSegment::Receipts`] part of data.
#[derive(Debug, Default)]
pub struct Receipts;

impl<DB: Database> Segment<DB> for Receipts {
    fn segment(&self) -> StaticFileSegment {
        StaticFileSegment::Receipts
    }

    fn copy_to_static_files(
        &self,
        provider: DatabaseProviderRO<DB>,
        static_file_provider: StaticFileProvider,
        block_range: RangeInclusive<BlockNumber>,
    ) -> ProviderResult<()> {
        let mut static_file_writer =
            static_file_provider.get_writer(*block_range.start(), StaticFileSegment::Receipts)?;

        for block in block_range {
            let _static_file_block =
                static_file_writer.increment_block(StaticFileSegment::Receipts, block)?;
            debug_assert_eq!(_static_file_block, block);

            let block_body_indices = provider
                .block_body_indices(block)?
                .ok_or(ProviderError::BlockBodyIndicesNotFound(block))?;

            let mut receipts_cursor = provider.tx_ref().cursor_read::<tables::Receipts>()?;
            let receipts_walker = receipts_cursor.walk_range(block_body_indices.tx_num_range())?;

            for entry in receipts_walker {
                let (tx_number, receipt) = entry?;

                static_file_writer.append_receipt(tx_number, receipt)?;
            }
        }

        Ok(())
    }

    fn create_static_file_file(
        &self,
        provider: &DatabaseProviderRO<DB>,
        directory: &Path,
        config: SegmentConfig,
        block_range: RangeInclusive<BlockNumber>,
    ) -> ProviderResult<()> {
        let tx_range = provider.transaction_range_by_block_range(block_range.clone())?;
        let tx_range_len = tx_range.clone().count();

        let jar = prepare_jar::<DB, 1>(
            provider,
            directory,
            StaticFileSegment::Receipts,
            config,
            block_range,
            tx_range_len,
            || {
                Ok([dataset_for_compression::<DB, tables::Receipts>(
                    provider,
                    &tx_range,
                    tx_range_len,
                )?])
            },
        )?;

        // Generate list of hashes for filters & PHF
        let hashes = if config.filters.has_filters() {
            Some(
                provider
                    .transaction_hashes_by_range(*tx_range.start()..(*tx_range.end() + 1))?
                    .into_iter()
                    .map(|(tx, _)| Ok(tx)),
            )
        } else {
            None
        };

        create_static_file_T1::<tables::Receipts, TxNumber, SegmentHeader>(
            provider.tx_ref(),
            tx_range,
            None,
            // We already prepared the dictionary beforehand
            None::<Vec<std::vec::IntoIter<Vec<u8>>>>,
            hashes,
            tx_range_len,
            jar,
        )?;

        Ok(())
    }
}
