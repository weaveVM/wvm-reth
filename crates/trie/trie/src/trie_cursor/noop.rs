use super::{TrieCursor, TrieCursorFactory};
use crate::{BranchNodeCompact, Nibbles};
<<<<<<< HEAD
use reth_db::DatabaseError;
use reth_primitives::B256;
=======
use reth_primitives::B256;
use reth_storage_errors::db::DatabaseError;
>>>>>>> c4b5f5e9c9a88783b2def3ab1cc880b8d41867e1

/// Noop trie cursor factory.
#[derive(Default, Debug)]
#[non_exhaustive]
pub struct NoopTrieCursorFactory;

impl TrieCursorFactory for NoopTrieCursorFactory {
    type AccountTrieCursor = NoopAccountTrieCursor;
    type StorageTrieCursor = NoopStorageTrieCursor;

<<<<<<< HEAD
    /// Generates a Noop account trie cursor.
=======
    /// Generates a noop account trie cursor.
>>>>>>> c4b5f5e9c9a88783b2def3ab1cc880b8d41867e1
    fn account_trie_cursor(&self) -> Result<Self::AccountTrieCursor, DatabaseError> {
        Ok(NoopAccountTrieCursor::default())
    }

<<<<<<< HEAD
    /// Generates a Noop storage trie cursor.
=======
    /// Generates a noop storage trie cursor.
>>>>>>> c4b5f5e9c9a88783b2def3ab1cc880b8d41867e1
    fn storage_trie_cursor(
        &self,
        _hashed_address: B256,
    ) -> Result<Self::StorageTrieCursor, DatabaseError> {
        Ok(NoopStorageTrieCursor::default())
    }
}

/// Noop account trie cursor.
#[derive(Default, Debug)]
#[non_exhaustive]
pub struct NoopAccountTrieCursor;

impl TrieCursor for NoopAccountTrieCursor {
    fn seek_exact(
        &mut self,
        _key: Nibbles,
    ) -> Result<Option<(Nibbles, BranchNodeCompact)>, DatabaseError> {
        Ok(None)
    }

    fn seek(
        &mut self,
        _key: Nibbles,
    ) -> Result<Option<(Nibbles, BranchNodeCompact)>, DatabaseError> {
        Ok(None)
    }

<<<<<<< HEAD
    /// Retrieves the current cursor position within the account trie.
=======
    fn next(&mut self) -> Result<Option<(Nibbles, BranchNodeCompact)>, DatabaseError> {
        Ok(None)
    }

>>>>>>> c4b5f5e9c9a88783b2def3ab1cc880b8d41867e1
    fn current(&mut self) -> Result<Option<Nibbles>, DatabaseError> {
        Ok(None)
    }
}

/// Noop storage trie cursor.
#[derive(Default, Debug)]
#[non_exhaustive]
pub struct NoopStorageTrieCursor;

impl TrieCursor for NoopStorageTrieCursor {
    fn seek_exact(
        &mut self,
        _key: Nibbles,
    ) -> Result<Option<(Nibbles, BranchNodeCompact)>, DatabaseError> {
        Ok(None)
    }

    fn seek(
        &mut self,
        _key: Nibbles,
    ) -> Result<Option<(Nibbles, BranchNodeCompact)>, DatabaseError> {
        Ok(None)
    }

<<<<<<< HEAD
    /// Retrieves the current cursor position within storage tries.
=======
    fn next(&mut self) -> Result<Option<(Nibbles, BranchNodeCompact)>, DatabaseError> {
        Ok(None)
    }

>>>>>>> c4b5f5e9c9a88783b2def3ab1cc880b8d41867e1
    fn current(&mut self) -> Result<Option<Nibbles>, DatabaseError> {
        Ok(None)
    }
}
