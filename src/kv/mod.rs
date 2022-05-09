use eyre::Result;
use mdbx::{DatabaseFlags, EnvironmentKind, NoWriteMap, TransactionKind, WriteFlags, RO, RW};
use std::{borrow::Cow, path::Path};

pub mod tables;
pub mod traits;

use tables::TableHandle;
use traits::{DbFlags, DbName, DupSort, Mode, Table, TableDecode, TableEncode};

fn open_env<E: EnvironmentKind>(
    path: &Path,
    num_tables: usize,
    flags: mdbx::EnvironmentFlags,
) -> Result<mdbx::Environment<E>> {
    mdbx::Environment::new()
        .set_max_dbs(num_tables)
        .set_flags(flags)
        .open(path)
        .map_err(From::from)
}

/// A wrapper around [`mdbx::Environment`].
///
/// We use this wrapper to make a few alterations on the default behavior:
/// - The mode the environment is opened in becomes part of the type signature.
/// You cannot open a read-write transaction using a `MdbxEnv<RO>`, and you
/// cannot get a `MdbxEnv<RW>` from a `MdbxEnv<RO>`. You can, however, move out
/// of the struct and do whatever you want with the inner `mdbx::Environment`,
/// safe or unsafe. You should only ever open a single mdbx environment from
/// the same process.
/// - The `mdbx::EnvironmentKind` is forced to `NoWriteMap`. MDBX_WRITEMAP
/// mode maps data into memory with write permission. This means stray writes
/// through pointers can silently corrupt the db. It's also slower when
/// db size > RAM, so we ignore it.
#[derive(Debug)]
pub struct MdbxEnv<M> {
    pub inner: mdbx::Environment<NoWriteMap>,
    _mode: std::marker::PhantomData<M>,
}
impl<M> MdbxEnv<M> {
    pub fn inner(&self) -> &mdbx::Environment<NoWriteMap> {
        &self.inner
    }
}

impl<M: Mode> MdbxEnv<M> {
    /// Open an mdbx environment. Note that even when opening an environment in
    /// read-only mode, mdbx will still modify the LCK-file, unless the filesystem
    /// is read-only.
    pub fn open(path: &Path, num_tables: usize, flags: EnvFlags) -> Result<Self> {
        let mode = if M::is_writeable() {
            mdbx::Mode::ReadWrite {
                sync_mode: mdbx::SyncMode::Durable,
            }
        } else {
            mdbx::Mode::ReadOnly
        };
        Ok(Self {
            inner: open_env(path, num_tables, flags.with_mode(mode))?,
            _mode: std::marker::PhantomData,
        })
    }

    /// Create a read-only mdbx transaction.
    pub fn begin_ro(&self) -> Result<MdbxTx<'_, RO>> {
        Ok(MdbxTx::new(self.inner.begin_ro_txn()?))
    }
}

impl MdbxEnv<RO> {
    /// Create a read-only mdbx transaction.
    pub fn begin(&self) -> Result<MdbxTx<'_, RO>> {
        Ok(MdbxTx::new(self.inner.begin_ro_txn()?))
    }
}

impl MdbxEnv<RW> {
    /// Create a read-write mdbx transaction. Blocks if another rw transaction is open.
    pub fn begin_rw(&self) -> Result<MdbxTx<'_, RW>> {
        Ok(MdbxTx::new(self.inner.begin_rw_txn()?))
    }
}

/// Holds all [`mdbx::EnvironmentFlags`] except the `mode` field.
#[derive(Clone, Copy, Debug, Default)]
pub struct EnvFlags {
    /// Disable readahead. Improves random read performance when db size > RAM.
    /// By default, mdbx will dynamically determine whether to disable readahead.
    pub no_rdahead: bool,
    /// Attempt to [coalesce](https://en.wikipedia.org/wiki/Coalescing_(computer_science)) while garbage collecting.
    pub coalesce: bool,
    /// If the environment is already in use by another process with unknown flags,
    /// by default an MDBX_INCOMPATIBLE error will be thrown. If `accede` is set,
    /// the requested table will instead be opened with the existing flags.
    pub accede: bool,
    /// By default, mdbx interprets the given path as a directory under which
    /// the lock file and storage file will be found or created. If `no_sub_dir`
    /// is set, this path is instead interpreted to be the storage file itself.
    pub no_sub_dir: bool,
    /// Attempt to take an exclusive lock on the environment. If another process
    /// is already using the environment, returns MDBX_BUSY.
    pub exclusive: bool,
    /// If enabled, don't initialize freshly malloc'd pages with zeroes. This can
    /// result in persisting garbage data.
    pub no_meminit: bool,
    /// Replace the default FIFO garbage collection policy with LIFO.
    pub liforeclaim: bool,
}
impl EnvFlags {
    /// Creates an [`mdbx::EnvironmentFlags`] struct with the requested mode.
    pub fn with_mode(self, mode: mdbx::Mode) -> mdbx::EnvironmentFlags {
        mdbx::EnvironmentFlags {
            mode,
            no_sub_dir: self.no_sub_dir,
            exclusive: self.exclusive,
            accede: self.accede,
            no_rdahead: self.no_rdahead,
            no_meminit: self.no_meminit,
            coalesce: self.coalesce,
            liforeclaim: self.liforeclaim,
        }
    }
}

/// A wrapper around [`mdbx::Transaction`].
#[derive(Debug)]
pub struct MdbxTx<'env, K: TransactionKind> {
    pub inner: mdbx::Transaction<'env, K, NoWriteMap>,
}
impl<'env, M> MdbxTx<'env, M>
where
    M: TransactionKind + Mode,
{
    pub fn open_db<'tx, Db: DbName, Flags: DbFlags>(
        &'tx self,
    ) -> Result<TableHandle<'tx, Db, Flags>> {
        let mut flags = Flags::FLAGS;
        // If the transaction is read-write, create the database if it does not exist already.
        if M::is_writeable() {
            flags |= DatabaseFlags::CREATE;
        }
        Ok(TableHandle::new(
            self.inner.open_db_with_flags(Some(Db::NAME), flags)?,
        ))
    }
}

impl<'env, K: TransactionKind> MdbxTx<'env, K> {
    pub fn new(inner: mdbx::Transaction<'env, K, NoWriteMap>) -> Self {
        Self { inner }
    }

    pub fn get<'tx, T, F>(
        &'tx self,
        db: TableHandle<'tx, T::Name, F>,
        key: T::Key,
    ) -> Result<Option<T::Value>>
    where
        T: Table<'tx>,
        F: DbFlags,
    {
        self.inner
            .get::<Cow<[u8]>>(db.as_ref(), key.encode().as_ref())?
            .map(|c| T::Value::decode(&c))
            .transpose()
    }

    pub fn cursor<'tx, T, F>(
        &'tx self,
        db: TableHandle<'tx, T::Name, F>,
    ) -> Result<MdbxCursor<'tx, K, T>>
    where
        T: Table<'tx>,
        F: DbFlags,
    {
        Ok(MdbxCursor::new(self.inner.cursor(db.as_ref())?))
    }
}

impl<'env> MdbxTx<'env, RW> {
    pub fn put<'tx, T, F>(
        &'tx self,
        db: TableHandle<'tx, T::Name, F>,
        key: T::Key,
        val: T::Value,
    ) -> Result<()>
    where
        T: Table<'tx>,
        F: DbFlags,
    {
        self.inner
            .put(db.as_ref(), key.encode(), val.encode(), WriteFlags::UPSERT)
            .map_err(From::from)
    }

    /// Commit the transaction. The Drop impl for mdbx::Transaction will take care
    /// of this, but use this method explicitly if you wish to handle any errors.
    pub fn commit(self) -> Result<bool> {
        self.inner.commit().map_err(From::from)
    }
}

/// A wrapper around [`mdbx::Cursor`].
#[derive(Debug)]
pub struct MdbxCursor<'tx, K, T>
where
    K: TransactionKind,
{
    pub inner: mdbx::Cursor<'tx, K>,
    _dbi: std::marker::PhantomData<T>,
}
impl<'tx, K, T> MdbxCursor<'tx, K, T>
where
    K: TransactionKind,
{
    pub fn new(inner: mdbx::Cursor<'tx, K>) -> Self {
        Self {
            inner,
            _dbi: std::marker::PhantomData,
        }
    }
}

impl<'tx, K, T> MdbxCursor<'tx, K, T>
where
    K: TransactionKind,
    T: Table<'tx>,
{
    /// Returns the (key, value) pair at the first key >= `key`
    pub fn seek(&mut self, key: T::SeekKey) -> Result<Option<(T::Key, T::Value)>>
    where
        T::Key: TableDecode,
    {
        self.inner
            .set_range::<Cow<_>, Cow<_>>(key.encode().as_ref())?
            .map(|(k, v)| Ok((T::Key::decode(&k)?, T::Value::decode(&v)?)))
            .transpose()
    }

    pub fn first(&mut self) -> Result<Option<(T::Key, T::Value)>>
    where
        T::Key: TableDecode,
    {
        self.inner
            .first::<Cow<_>, Cow<_>>()?
            .map(|(k, v)| Ok((T::Key::decode(&k)?, T::Value::decode(&v)?)))
            .transpose()
    }

    /// Returns an iterator over (key, value) pairs beginning at start_key. If the table
    /// is dupsorted (contains duplicate items for each key), all of the duplicates
    /// at a given key will be returned before moving on to the next key.
    pub fn walk(
        &mut self,
        start_key: T::Key,
    ) -> impl Iterator<Item = Result<(<T as Table<'tx>>::Key, <T as Table<'tx>>::Value)>> + '_
    where
        T::Key: TableDecode,
    {
        self.inner
            .iter_from::<Cow<_>, Cow<_>>(&start_key.encode().as_ref())
            .map(|res| {
                let (k, v) = res?;
                Ok((T::Key::decode(&k)?, T::Value::decode(&v)?))
            })
    }

    /// Returns an iterator over values beginning at start_key, without attempting
    /// to decode the returned keys (only the values). If the table is dupsorted
    /// (contains duplicate items for each key), all of the duplicates at a
    /// given key will be returned before moving on to the next key.
    pub fn walk_values(
        &mut self,
        start_key: T::Key,
    ) -> impl Iterator<Item = Result<<T as Table<'tx>>::Value>> + '_ {
        self.inner
            .iter_from::<Cow<_>, Cow<_>>(&start_key.encode().as_ref())
            .map(|res| {
                let (_, v) = res?;
                T::Value::decode(&v)
            })
    }
}

impl<'tx, K, T> MdbxCursor<'tx, K, T>
where
    K: TransactionKind,
    T: DupSort<'tx>,
{
    /// Finds the given key in the table, then the first duplicate entry at that
    /// key with data >= subkey, and returns this value. Note that the value
    /// returned includes the subkey prefix, meaning you likely want to decode
    /// it into `(subkey, value_at_subkey)`.
    ///
    /// If you want to find an exact subkey in the dupsort "sub table", you need
    /// to check that the returned value begins with your subkey. If it does not,
    /// then the cursor seeked past the requested subkey without a match, meaning
    /// the table does not contain a value that begins with the provided subkey.
    pub fn seek_dup(&mut self, key: T::Key, subkey: T::Subkey) -> Result<Option<T::Value>> {
        self.inner
            .get_both_range::<Cow<[u8]>>(key.encode().as_ref(), subkey.encode().as_ref())?
            .map(|c| T::Value::decode(&c))
            .transpose()
    }

    /// Returns the current key and the next duplicate value at that key. Note
    /// that the value returned includes the subkey prefix, meaning you likely
    /// want to decode it into `(subkey, value_at_subkey)`.
    pub fn next_dup(&mut self) -> Result<Option<(T::Key, T::Value)>>
    where
        T::Key: TableDecode,
    {
        self.inner
            .next_dup::<Cow<_>, Cow<_>>()?
            .map(|(k, v)| Ok((T::Key::decode(&k)?, T::Value::decode(&v)?)))
            .transpose()
    }

    /// Returns the next duplicate value at the current key, without attempting
    /// to decode the table key. Note that the value returned includes the
    /// subkey prefix, meaning you likely want to decode it into
    /// `(subkey, value_at_subkey)`.
    pub fn next_dup_value(&mut self) -> Result<Option<T::Value>> {
        self.inner
            .next_dup::<Cow<_>, Cow<_>>()?
            .map(|(_, v)| T::Value::decode(&v))
            .transpose()
    }

    /// Returns an iterator over duplicate values for the given key. Note that
    /// the values returned include the subkey prefix, meaning you likely want
    /// to decode them into `(subkey, value_at_subkey)`.
    pub fn walk_dup(
        mut self,
        start_key: T::Key,
    ) -> Result<impl Iterator<Item = Result<<T as Table<'tx>>::Value>>> {
        let first = self
            .inner
            .set::<Cow<_>>(start_key.encode().as_ref())?
            .map(|cow_val| T::Value::decode(&cow_val));

        Ok(DupWalker { cur: self, first })
    }
}

/// An internal struct for turning a cursor to a dupsorted table into an iterator
/// over values in that table.
///
/// See [Akula](https://github.com/akula-bft/akula/blob/1800ac77b979d410bea5ff3bcd2617cb302d66fe/src/kv/mdbx.rs#L432)
/// for a much more interesting approach using generators.
struct DupWalker<'tx, K, T>
where
    K: TransactionKind,
    T: Table<'tx>,
{
    pub cur: MdbxCursor<'tx, K, T>,
    pub first: Option<Result<T::Value>>,
}

impl<'tx, K, T> std::iter::Iterator for DupWalker<'tx, K, T>
where
    K: TransactionKind,
    T: DupSort<'tx>,
{
    type Item = Result<T::Value>;
    fn next(&mut self) -> Option<Self::Item> {
        let first = self.first.take();
        if first.is_some() {
            return first;
        }
        self.cur.next_dup_value().transpose()
    }
}
