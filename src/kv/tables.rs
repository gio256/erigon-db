use crate::kv::{MdbxTx};
use ethers::types::H256;
use eyre::{eyre, Result};
use mdbx::{DatabaseFlags, NoWriteMap, TransactionKind};
use std::{convert::AsRef, fmt::Debug, ops::Deref};

use crate::kv::traits::*;

pub type DbFlags = u32;
pub struct TableHandle<'tx, Dbi, const FLAGS: DbFlags> {
    inner: mdbx::Database<'tx>,
    _dbi: std::marker::PhantomData<Dbi>,
}
impl<'tx, Dbi, const FLAGS: DbFlags> TableHandle<'tx, Dbi, FLAGS> {
    pub fn new(inner: mdbx::Database<'tx>) -> Self {
        Self {
            inner,
            _dbi: std::marker::PhantomData,
        }
    }
    pub fn inner(&self) -> &mdbx::Database<'tx> {
        &self.inner
    }
}
impl<'tx, Dbi, const FLAGS: DbFlags> Deref for TableHandle<'tx, Dbi, FLAGS> {
    type Target = mdbx::Database<'tx>;
    fn deref(&self) -> &Self::Target {
        self.inner()
    }
}
impl<'tx, Dbi, const FLAGS: DbFlags> AsRef<mdbx::Database<'tx>> for TableHandle<'tx, Dbi, FLAGS> {
    fn as_ref(&self) -> &mdbx::Database<'tx> {
        self.inner()
    }
}

impl DbName for LastHeader {
    fn db_name() -> Option<&'static str> {
        Some("LastHeader")
    }
}
#[derive(Debug, Default)]
pub struct LastHeaderKey;
impl TableEncode for LastHeaderKey {
    type Encoded = Vec<u8>;

    fn encode(self) -> Self::Encoded {
        String::from("LastHeader").into_bytes()
    }
}
#[derive(Debug)]
pub struct LastHeader;
impl<'tx> Table<'tx> for LastHeader {
    type Key = LastHeaderKey;
    type Value = H256;
    type SeekKey = LastHeaderKey;
    type Dbi = TableHandle<'tx, Self, { Self::flags() }>;
}
impl LastHeader {
    pub const fn flags() -> u32 {
        DatabaseFlags::DUP_SORT.bits()
    }
}

impl TableDecode for Vec<u8> {
    fn decode(b: &[u8]) -> eyre::Result<Self> {
        Ok(b.to_vec())
    }
}

impl TableEncode for Vec<u8> {
    type Encoded = Self;
    fn encode(self) -> Self::Encoded {
        self
    }
}

const KECCAK_LENGTH: usize = 32;
impl TableEncode for H256 {
    type Encoded = [u8; KECCAK_LENGTH];
    fn encode(self) -> Self::Encoded {
        self.0
    }
}

impl TableDecode for H256 {
    fn decode(b: &[u8]) -> Result<Self> {
        match b.len() {
            KECCAK_LENGTH => Ok(H256::from_slice(&*b)),
            other => Err(eyre!("bad")),
        }
    }
}
