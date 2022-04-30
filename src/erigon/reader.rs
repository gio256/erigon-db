use crate::kv::{tables::TableHandle, MdbxTx};
use eyre::{eyre, Result};
use mdbx::TransactionKind;

use crate::erigon::*;

/// Erigon wraps an MdbxTx and provides Erigon-specific access methods.
pub struct Erigon<'env, K: TransactionKind>(MdbxTx<'env, K>);

impl<'env, K: Mode> Erigon<'env, K> {
    /// Opens and reads from the db table with the table's default flags
    pub fn read<'tx, T>(&'tx self, key: T::Key) -> Result<Option<T::Value>>
    where
        T: Table<'tx> + DefaultFlags,
    {
        self.0.get::<T, T::Flags>(self.0.open_db()?, key)
    }

    pub fn read_head_header_hash(&self) -> Result<H256> {
        self.read::<LastHeader>(LastHeader)?.ok_or(eyre!("No LastHeader"))
    }
    /// Returns the incarnation of the account when it was last deleted.
    /// If the account is not in the db, returns 0.
    pub fn read_incarnation(&self, who: Address) -> Result<u64> {
        self.read::<IncarnationMap>(who).map(|v| v.unwrap_or_default())
    }
}
impl<'env> Erigon<'env, mdbx::RW> {
    /// Opens and writes to the db table with the table's default flags
    pub fn write<'tx, T>(&'tx self, key: T::Key, val: T::Value) -> Result<()>
    where
        T: Table<'tx> + DefaultFlags,
    {
        self.0.set::<T, T::Flags>(self.0.open_db()?, key, val)
    }

    pub fn write_head_header_hash(&self, v: H256) -> Result<()> {
        self.write::<LastHeader>(LastHeader, v)
    }
    pub fn write_incarnation(&self, k: Address, v: u64) -> Result<()> {
        self.write::<IncarnationMap>(k, v)
    }
}