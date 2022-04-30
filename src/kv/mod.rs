use eyre::{eyre, Result};
use mdbx::{
    DatabaseFlags, EnvironmentKind, NoWriteMap, Transaction, TransactionKind, WriteFlags, RO, RW,
};
use std::{borrow::Cow, path::Path};

pub mod tables;
pub mod traits;

use tables::TableHandle;
use traits::{DbFlags, DbName, Mode, Table, TableDecode, TableEncode};

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

pub struct MdbxEnv<M> {
    // Force NoWriteMap. MDBX_WRITEMAP mode maps data into memory with
    // write permission. This means stray writes through pointers can silently
    // corrupt the db. It's also slower when db size > RAM, so we ignore it
    inner: mdbx::Environment<NoWriteMap>,
    _mode: std::marker::PhantomData<M>,
}
impl<M> MdbxEnv<M> {
    pub fn inner(&self) -> &mdbx::Environment<NoWriteMap> {
        &self.inner
    }
    /// Create a read-only mdbx transaction
    pub fn begin_ro(&self) -> Result<MdbxTx<'_, RO>> {
        Ok(MdbxTx::new(self.inner.begin_ro_txn()?))
    }
}

impl MdbxEnv<RO> {
    /// Open an mdbx environment in read-only mode. Mdbx will still modify the
    /// LCK-file, unless the filesystem is read-only.
    pub fn open_ro(path: &Path, num_tables: usize, flags: EnvFlags) -> Result<Self> {
        let flags = flags.with_mode(mdbx::Mode::ReadOnly);
        Ok(Self {
            inner: open_env(path, num_tables, flags)?,
            _mode: std::marker::PhantomData,
        })
    }
}

impl MdbxEnv<RW> {
    /// Open an mdbx environment in read-write mode.
    pub fn open_rw(path: &Path, num_tables: usize, flags: EnvFlags) -> Result<Self> {
        let flags = flags.with_mode(mdbx::Mode::ReadWrite {
            sync_mode: mdbx::SyncMode::Durable,
        });
        Ok(Self {
            inner: open_env(path, num_tables, flags)?,
            _mode: std::marker::PhantomData,
        })
    }

    /// Create a read-write mdbx transaction. Blocks if another rw transaction is open.
    pub fn begin_rw(&self) -> Result<MdbxTx<'_, RW>> {
        Ok(MdbxTx::new(self.inner.begin_rw_txn()?))
        // Ok(self.inner().begin_rw_txn()?)
    }
}

/// Holds all of mdbx::EnvironmentFlags except the `mode` field.
#[derive(Clone, Copy, Debug, Default)]
pub struct EnvFlags {
    pub no_sub_dir: bool,
    pub exclusive: bool,
    pub accede: bool,
    pub no_rdahead: bool,
    pub no_meminit: bool,
    pub coalesce: bool,
    pub liforeclaim: bool,
}
impl EnvFlags {
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
}

impl<'env> MdbxTx<'env, RW> {
    pub fn set<'tx, T, F>(
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
}
