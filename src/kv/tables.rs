use crate::kv::MdbxTx;
use arrayvec::ArrayVec;
use derive_more::{Deref, DerefMut};
use ethereum_types::{Address, H256, U256};
use eyre::{eyre, Result};
use mdbx::{DatabaseFlags, NoWriteMap, TransactionKind};
use roaring::RoaringTreemap;
use std::{
    convert::AsRef,
    fmt::{Debug, Display},
    ops::Deref,
};

use crate::kv::traits::*;

const KECCAK_LENGTH: usize = 32;
const ADDRESS_LENGTH: usize = 20;

pub struct TableHandle<'tx, Dbi, Flags> {
    inner: mdbx::Database<'tx>,
    _dbi: std::marker::PhantomData<(Dbi, Flags)>,
}
impl<'tx, Dbi, Flags: DbFlags> TableHandle<'tx, Dbi, Flags> {
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
impl<'tx, Dbi, Flags: DbFlags> Deref for TableHandle<'tx, Dbi, Flags> {
    type Target = mdbx::Database<'tx>;
    fn deref(&self) -> &Self::Target {
        self.inner()
    }
}
impl<'tx, Dbi, Flags: DbFlags> AsRef<mdbx::Database<'tx>> for TableHandle<'tx, Dbi, Flags> {
    fn as_ref(&self) -> &mdbx::Database<'tx> {
        self.inner()
    }
}

pub struct NoFlags;
impl DbFlags for NoFlags {
    const FLAGS: DatabaseFlags = DatabaseFlags::empty();
}
pub struct DupSortFlags;
impl DbFlags for DupSortFlags {
    const FLAGS: DatabaseFlags = DatabaseFlags::DUP_SORT;
}
#[macro_export]
macro_rules! table_without_flags {
    ($name:ident => $key:ty => $value:ty, SeekKey = $seek_key:ty) => {
        #[derive(Debug, Default, Clone, Copy)]
        pub struct $name;

        impl<'tx> $crate::kv::traits::Table<'tx> for $name {
            type Name = Self;
            type Key = $key;
            type SeekKey = $seek_key;
            type Value = $value;
        }

        impl $crate::kv::traits::DbName for $name {
            const NAME: &'static str = stringify!($name);
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}", stringify!($name))
            }
        }
    };
    ($name:ident => $key:ty => $value:ty) => {
        $crate::table_without_flags!($name => $key => $value, SeekKey = $key);
    };
}

#[macro_export]
macro_rules! table {
    ($name:ident => $($args:tt)*) => {
        $crate::table_without_flags!($name => $($args)*);
        impl $crate::kv::traits::DefaultFlags for $name {
            type Flags = $crate::kv::tables::NoFlags;
        }
    };
}
#[macro_export]
macro_rules! dupsort_table {
    ($name:ident => $key:ty => $value:ty, Subkey = $subkey:ty) => {
        $crate::table_without_flags!($name => $key => $value);
        impl $crate::kv::traits::DefaultFlags for $name {
            type Flags = $crate::kv::tables::DupSortFlags;
        }
        impl crate::kv::traits::DupSort<'_> for $name {
            type Subkey = $subkey;
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
        let mut buf = [0; 32];
        self.to_big_endian(&mut buf);
        buf.into_iter().skip_while(|&v| v == 0).collect()
    }
}

impl TableDecode for U256 {
    fn decode(b: &[u8]) -> Result<Self> {
        if b.len() > KECCAK_LENGTH {
            return Err(TooLong::<KECCAK_LENGTH> { got: b.len() }.into());
        }
        let mut v = [0; 32];
        v[KECCAK_LENGTH - b.len()..].copy_from_slice(b);
        Ok(Self::from_big_endian(&v))
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

impl<A, B, const A_LEN: usize, const B_LEN: usize> TableEncode for (A, B)
where
    A: TableObject<Encoded = [u8; A_LEN]>,
    B: TableObject<Encoded = [u8; B_LEN]>,
{
    type Encoded = VariableVec<256>;

    fn encode(self) -> Self::Encoded {
        let mut v = Self::Encoded::default();
        v.try_extend_from_slice(&self.0.encode()).unwrap();
        v.try_extend_from_slice(&self.1.encode()).unwrap();
        v
    }
}

impl<A, B, const A_LEN: usize, const B_LEN: usize> TableDecode for (A, B)
where
    A: TableObject<Encoded = [u8; A_LEN]>,
    B: TableObject<Encoded = [u8; B_LEN]>,
{
    fn decode(v: &[u8]) -> Result<Self> {
        if v.len() != A_LEN + B_LEN {
            eyre::bail!("Invalid len: {} != {} + {}", v.len(), A_LEN, B_LEN);
        }
        Ok((
            A::decode(&v[..A_LEN]).unwrap(),
            B::decode(&v[A_LEN..]).unwrap(),
        ))
    }
}

impl TableEncode for RoaringTreemap {
    type Encoded = Vec<u8>;
    fn encode(mut self) -> Self::Encoded {
        let mut buf = Vec::with_capacity(self.serialized_size());
        self.serialize_into(&mut buf).unwrap();
        buf
    }
}
impl TableDecode for RoaringTreemap {
    fn decode(b: &[u8]) -> Result<Self> {
        Ok(RoaringTreemap::deserialize_from(b)?)
    }
}

impl TableEncode for bytes::Bytes {
    type Encoded = Self;

    fn encode(self) -> Self::Encoded {
        self
    }
}

impl TableDecode for bytes::Bytes {
    fn decode(b: &[u8]) -> Result<Self> {
        Ok(b.to_vec().into())
    }
}

#[macro_export]
macro_rules! u64_table_object {
    ($ty:ident) => {
        impl $crate::kv::traits::TableEncode for $ty {
            type Encoded = [u8; 8];

            fn encode(self) -> Self::Encoded {
                self.to_be_bytes()
            }
        }

        impl $crate::kv::traits::TableDecode for $ty {
            fn decode(b: &[u8]) -> Result<Self> {
                match b.len() {
                    8 => Ok(u64::from_be_bytes(*::arrayref::array_ref!(&*b, 0, 8)).into()),
                    other => Err($crate::kv::tables::InvalidLength::<8> { got: other }.into()),
                }
            }
        }
    };
}

u64_table_object!(u64);
