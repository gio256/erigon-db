use crate::kv::MdbxTx;
use arrayref::array_ref;
use arrayvec::ArrayVec;
use derive_more::{Deref, DerefMut};
use ethers::types::{Address, H256, U256};
use eyre::{eyre, Result};
use mdbx::{DatabaseFlags, NoWriteMap, TransactionKind};
use std::{
    convert::AsRef,
    fmt::{Debug, Display},
    ops::Deref,
};

use crate::kv::traits::*;

const KECCAK_LENGTH: usize = 32;
const ADDRESS_LENGTH: usize = 20;

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

#[macro_export]
macro_rules! decl_table_without_flags {
    ($name:ident => $key:ty => $value:ty; seek_by: $seek_key:ty) => {
        #[derive(Debug, Default, Clone, Copy)]
        pub struct $name;

        impl<'tx> $crate::kv::traits::Table<'tx> for $name {
            type Key = $key;
            type SeekKey = $seek_key;
            type Value = $value;
            type Dbi = $crate::kv::tables::TableHandle<'tx, Self, { Self::flags() }>;
        }

        impl $crate::kv::traits::DbName for $name {
            fn db_name() -> Option<&'static str> {
                Some(stringify!($name))
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}", stringify!($name))
            }
        }
    };
    ($name:ident => $key:ty => $value:ty) => {
        $crate::decl_table_without_flags!($name => $key => $value; seek_by: $key);
    };
}

#[macro_export]
macro_rules! decl_table {
    ($name:ident => $($args:tt)*) => {
        $crate::decl_table_without_flags!($name => $($args)*);
        impl $name {
            pub const fn flags() -> u32 {
                0
            }
        }
    };
}
#[macro_export]
macro_rules! decl_dupsort_table {
    ($name:ident => $($args:tt)*) => {
        $crate::decl_table_without_flags!($name => $($args)*);
        impl $name {
            pub const fn flags() -> u32 {
                ::mdbx::DatabaseFlags::DUP_SORT.bits()
            }
        }
    };
}

// -- Key/Value Encoding/Decoding --

impl TableEncode for () {
    type Encoded = [u8; 0];
    fn encode(self) -> Self::Encoded {
        []
    }
}

impl TableDecode for () {
    fn decode(b: &[u8]) -> Result<Self> {
        if !b.is_empty() {
            return Err(TooLong::<0> { got: b.len() }.into());
        }
        Ok(())
    }
}

impl TableEncode for Vec<u8> {
    type Encoded = Self;

    fn encode(self) -> Self::Encoded {
        self
    }
}

impl TableDecode for Vec<u8> {
    fn decode(b: &[u8]) -> Result<Self> {
        Ok(b.to_vec())
    }
}

#[derive(Clone, Debug, Default, Deref, DerefMut, PartialEq, Eq, PartialOrd, Ord)]
pub struct VariableVec<const LEN: usize> {
    pub inner: ArrayVec<u8, LEN>,
}

impl<const LEN: usize> FromIterator<u8> for VariableVec<LEN> {
    fn from_iter<T: IntoIterator<Item = u8>>(iter: T) -> Self {
        Self {
            inner: ArrayVec::from_iter(iter),
        }
    }
}

impl<const LEN: usize> AsRef<[u8]> for VariableVec<LEN> {
    fn as_ref(&self) -> &[u8] {
        self.inner.as_ref()
    }
}

impl<const LEN: usize> TableEncode for VariableVec<LEN> {
    type Encoded = Self;

    fn encode(self) -> Self::Encoded {
        self
    }
}

impl<const LEN: usize> TableDecode for VariableVec<LEN> {
    fn decode(b: &[u8]) -> Result<Self> {
        let mut out = Self::default();
        out.try_extend_from_slice(b)?;
        Ok(out)
    }
}

impl<const LEN: usize> From<VariableVec<LEN>> for Vec<u8> {
    fn from(v: VariableVec<LEN>) -> Self {
        v.to_vec()
    }
}

#[derive(Clone, Debug)]
pub struct InvalidLength<const EXPECTED: usize> {
    pub got: usize,
}

impl<const EXPECTED: usize> Display for InvalidLength<EXPECTED> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Invalid length: {} != {}", EXPECTED, self.got)
    }
}

impl<const EXPECTED: usize> std::error::Error for InvalidLength<EXPECTED> {}

#[derive(Clone, Debug)]
pub struct TooShort<const MINIMUM: usize> {
    pub got: usize,
}

impl<const MINIMUM: usize> Display for TooShort<MINIMUM> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Too short: {} < {}", self.got, MINIMUM)
    }
}

impl<const MINIMUM: usize> std::error::Error for TooShort<MINIMUM> {}

#[derive(Clone, Debug)]
pub struct TooLong<const MAXIMUM: usize> {
    pub got: usize,
}
impl<const MAXIMUM: usize> Display for TooLong<MAXIMUM> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Too long: {} > {}", self.got, MAXIMUM)
    }
}

impl<const MAXIMUM: usize> std::error::Error for TooLong<MAXIMUM> {}

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

impl TableEncode for U256 {
    type Encoded = VariableVec<KECCAK_LENGTH>;
    fn encode(self) -> Self::Encoded {
        todo!() //TODO
        // self.to_be_bytes()
        //     .into_iter()
        //     .skip_while(|&v| v == 0)
        //     .collect()
    }
}

impl TableDecode for U256 {
    fn decode(b: &[u8]) -> Result<Self> {
        todo!() //TODO
        // if b.len() > KECCAK_LENGTH {
        //     return Err(TooLong::<KECCAK_LENGTH> { got: b.len() }.into());
        // }
        // let mut v = [0; 32];
        // v[KECCAK_LENGTH - b.len()..].copy_from_slice(b);
        // Ok(Self::from_be_bytes(v))
    }
}

impl TableEncode for Address {
    type Encoded = [u8; ADDRESS_LENGTH];

    fn encode(self) -> Self::Encoded {
        self.0
    }
}

impl TableDecode for Address {
    fn decode(b: &[u8]) -> Result<Self> {
        match b.len() {
            ADDRESS_LENGTH => Ok(Address::from_slice(&*b)),
            other => Err(InvalidLength::<ADDRESS_LENGTH> { got: other }.into()),
        }
    }
}

macro_rules! u64_table_object {
    ($ty:ident) => {
        impl TableEncode for $ty {
            type Encoded = [u8; 8];

            fn encode(self) -> Self::Encoded {
                self.to_be_bytes()
            }
        }

        impl TableDecode for $ty {
            fn decode(b: &[u8]) -> Result<Self> {
                match b.len() {
                    8 => Ok(u64::from_be_bytes(*array_ref!(&*b, 0, 8)).into()),
                    other => Err(InvalidLength::<8> { got: other }.into()),
                }
            }
        }
    };
}

u64_table_object!(u64);

impl TableEncode for (H256, U256) {
    type Encoded = VariableVec<{ KECCAK_LENGTH + KECCAK_LENGTH }>;

    fn encode(self) -> Self::Encoded {
        let mut out = Self::Encoded::default();
        out.try_extend_from_slice(&self.0.encode()).unwrap();
        out.try_extend_from_slice(&self.1.encode()).unwrap();
        out
    }
}

impl TableDecode for (H256, U256) {
    fn decode(b: &[u8]) -> Result<Self> {
        if b.len() > KECCAK_LENGTH + KECCAK_LENGTH {
            return Err(TooLong::<{ KECCAK_LENGTH + KECCAK_LENGTH }> { got: b.len() }.into());
        }

        if b.len() < KECCAK_LENGTH {
            return Err(TooShort::<{ KECCAK_LENGTH }> { got: b.len() }.into());
        }

        let (location, value) = b.split_at(KECCAK_LENGTH);

        Ok((H256::decode(location)?, U256::decode(value)?))
    }
}
