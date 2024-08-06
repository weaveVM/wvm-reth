//! Helper provider traits to encapsulate all provider traits for simplicity.

use crate::{
<<<<<<< HEAD
    AccountReader, BlockReaderIdExt, CanonStateSubscriptions, ChainSpecProvider, ChangeSetReader,
    DatabaseProviderFactory, EvmEnvProvider, HeaderProvider, StageCheckpointReader,
    StateProviderFactory, StaticFileProviderFactory, TransactionsProvider,
=======
    AccountReader, BlockReaderIdExt, ChainSpecProvider, ChangeSetReader, DatabaseProviderFactory,
    EvmEnvProvider, HeaderProvider, StageCheckpointReader, StateProviderFactory,
    StaticFileProviderFactory, TransactionsProvider,
>>>>>>> c4b5f5e9c9a88783b2def3ab1cc880b8d41867e1
};
use reth_chain_state::CanonStateSubscriptions;
use reth_db_api::database::Database;

/// Helper trait to unify all provider traits for simplicity.
pub trait FullProvider<DB: Database>:
    DatabaseProviderFactory<DB>
    + StaticFileProviderFactory
    + BlockReaderIdExt
    + AccountReader
    + StateProviderFactory
    + EvmEnvProvider
    + ChainSpecProvider
    + ChangeSetReader
    + CanonStateSubscriptions
    + StageCheckpointReader
    + Clone
    + Unpin
    + 'static
{
}

impl<T, DB: Database> FullProvider<DB> for T where
    T: DatabaseProviderFactory<DB>
        + StaticFileProviderFactory
        + BlockReaderIdExt
        + AccountReader
        + StateProviderFactory
        + EvmEnvProvider
        + ChainSpecProvider
        + ChangeSetReader
        + CanonStateSubscriptions
        + StageCheckpointReader
        + Clone
        + Unpin
        + 'static
{
}

/// Helper trait to unify all provider traits required to support `eth` RPC server behaviour, for
/// simplicity.
pub trait FullRpcProvider:
    StateProviderFactory
    + EvmEnvProvider
    + ChainSpecProvider
    + BlockReaderIdExt
    + HeaderProvider
    + TransactionsProvider
<<<<<<< HEAD
=======
    + StageCheckpointReader
>>>>>>> c4b5f5e9c9a88783b2def3ab1cc880b8d41867e1
    + Clone
    + Unpin
    + 'static
{
}

impl<T> FullRpcProvider for T where
    T: StateProviderFactory
        + EvmEnvProvider
        + ChainSpecProvider
        + BlockReaderIdExt
        + HeaderProvider
        + TransactionsProvider
<<<<<<< HEAD
=======
        + StageCheckpointReader
>>>>>>> c4b5f5e9c9a88783b2def3ab1cc880b8d41867e1
        + Clone
        + Unpin
        + 'static
{
}
