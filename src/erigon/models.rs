use bytes::{Buf, Bytes};
use derive_more::{Deref, DerefMut};
use ethereum_types::{Address, Bloom, H256, H64, U256};
use eyre::{eyre, Result};
use fastrlp::*;
use parity_scale_codec::{Decode, Encode};
use serde::{Deserialize, Serialize};

use crate::kv::traits::{TableDecode, TableEncode};

const KECCAK_LENGTH: usize = H256::len_bytes();
const ADDRESS_LENGTH: usize = Address::len_bytes();
const U64_LENGTH: usize = std::mem::size_of::<u64>();

#[derive(
    Clone,
    Debug,
    PartialEq,
    Eq,
    Default,
    Serialize,
    Deserialize,
    Encode,
    Decode,
    RlpEncodable,
    RlpDecodable,
)]
pub struct Rlp(Bytes);

#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct Account {
    pub nonce: u64,
    pub incarnation: u64,
    pub balance: U256,
    pub codehash: H256, // hash of the bytecode
}

impl TableDecode for Account {
    fn decode(mut enc: &[u8]) -> Result<Self> {
        let mut acct = Self::default();

        if enc.is_empty() {
            return Ok(acct);
        }

        let fieldset = enc.get_u8();

        // has nonce
        if fieldset & 1 > 0 {
            acct.nonce = parse_u64_with_len(&mut enc);
        }

        // has balance
        if fieldset & 2 > 0 {
            let bal_len = enc.get_u8();
            acct.balance = enc[..bal_len.into()].into();
            enc.advance(bal_len.into());
        }

        // has incarnation
        if fieldset & 4 > 0 {
            acct.incarnation = parse_u64_with_len(&mut enc);
        }

        // has codehash
        if fieldset & 8 > 0 {
            let len: usize = enc.get_u8().into();
            if len != KECCAK_LENGTH {
                eyre::bail!(
                    "codehash should be {} bytes long. Got {} instead",
                    KECCAK_LENGTH,
                    len
                );
            }
            acct.codehash = H256::from_slice(&enc[..KECCAK_LENGTH]);
            enc.advance(KECCAK_LENGTH)
        }

        // TODO: erigon docs mention storage hash field, code seems to disagree
        if enc.remaining() > 0 {
            eyre::bail!("unexpected account field")
        }

        Ok(acct)
    }
}
//TODO: dummy impl as we only need to decode for now, but need the trait bound
impl TableEncode for Account {
    type Encoded = Vec<u8>;
    fn encode(self) -> Self::Encoded {
        Self::Encoded::default()
    }
}

pub fn parse_u64_with_len(enc: &mut &[u8]) -> u64 {
    let len = enc.get_u8().into();
    let val = bytes_to_u64(&enc[..len]);
    enc.advance(len);
    val
}
// https://github.com/akula-bft/akula/blob/a9aed09b31bb41c89832149bcad7248f7fcd70ca/src/models/account.rs#L47
pub fn bytes_to_u64(buf: &[u8]) -> u64 {
    let mut decoded = [0u8; 8];
    for (i, b) in buf.iter().rev().enumerate() {
        decoded[i] = *b;
    }
    u64::from_le_bytes(decoded)
}

impl Account {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn nonce(mut self, nonce: u64) -> Self {
        self.nonce = nonce;
        self
    }
    pub fn incarnation(mut self, inc: u64) -> Self {
        self.incarnation = inc;
        self
    }
    pub fn balance(mut self, bal: U256) -> Self {
        self.balance = bal;
        self
    }
    pub fn codehash(mut self, hash: H256) -> Self {
        self.codehash = hash;
        self
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct StorageKey {
    pub address: Address,
    pub incarnation: u64,
}
impl StorageKey {
    pub fn new(address: Address, incarnation: u64) -> Self {
        Self {
            address,
            incarnation,
        }
    }
}

impl TableEncode for StorageKey {
    type Encoded = [u8; ADDRESS_LENGTH + U64_LENGTH];

    fn encode(self) -> Self::Encoded {
        let mut out = [0; ADDRESS_LENGTH + U64_LENGTH];
        out[..ADDRESS_LENGTH].copy_from_slice(&self.address.encode());
        out[ADDRESS_LENGTH..].copy_from_slice(&self.incarnation.encode());
        out
    }
}
//TODO: dummy impl as we only need to encode for now, but need the trait bound
impl TableDecode for StorageKey {
    fn decode(_enc: &[u8]) -> Result<Self> {
        Ok(Default::default())
    }
}

////

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Encode, Decode, RlpDecodable)]
pub struct BodyForStorage {
    pub base_tx_id: u64,
    pub tx_amount: u32,
    // pub uncles: Vec<BlockHeader>,
}
// #[derive(Clone, Debug, PartialEq, Eq, Deref, DerefMut)]
// struct BlockNumber(u64);
crate::u64_table_object!(BlockNumber);
// u64_table_object!(TxIndex);

#[derive(Clone, Debug, PartialEq, Eq, Default, Serialize, Deserialize, Encode, Decode)]
pub struct BlockHeader {
    pub parent_hash: H256,
    pub uncle_hash: H256,
    pub coinbase: Address,
    pub root: H256,
    pub tx_hash: H256,
    pub receipts_hash: H256,
    pub bloom: Bloom,
    pub difficulty: U256,
    pub number: U256, // TODO: erigon stores as big.Int, then casts, which returns 0 if > u64 (technically big.Int says result is undefined)
    pub gas_limit: u64,
    pub gas_used: u64,
    pub time: u64,
    pub extra: Bytes,
    pub mix_digest: H256,
    pub nonce: H64,
    pub base_fee: U256,
    pub eip1559: bool,
    pub seal: Rlp,
    pub with_seal: bool,
}

macro_rules! impl_from {
    ($type:ty, $other:ty) => {
        impl From<$type> for $other {
            #[inline(always)]
            fn from(x: $type) -> $other {
                x.0 as $other
            }
        }
    };
}

macro_rules! u64_wrapper {
    ($ty:ident) => {
        #[derive(
            Clone,
            Copy,
            Debug,
            Deref,
            DerefMut,
            Default,
            ::derive_more::Display,
            Eq,
            ::derive_more::From,
            ::derive_more::FromStr,
            PartialEq,
            PartialOrd,
            Ord,
            Hash,
            Serialize,
            Deserialize,
        )]
        #[serde(transparent)]
        #[repr(transparent)]
        pub struct $ty(pub u64);

        impl ::parity_scale_codec::WrapperTypeEncode for $ty {}
        impl ::parity_scale_codec::EncodeLike for $ty {}
        impl ::parity_scale_codec::EncodeLike<u64> for $ty {}
        impl ::parity_scale_codec::EncodeLike<$ty> for u64 {}
        impl ::parity_scale_codec::WrapperTypeDecode for $ty {
            type Wrapped = u64;
        }
        impl From<::parity_scale_codec::Compact<$ty>> for $ty {
            #[inline(always)]
            fn from(x: ::parity_scale_codec::Compact<$ty>) -> $ty {
                x.0
            }
        }
        impl ::parity_scale_codec::CompactAs for $ty {
            type As = u64;
            #[inline(always)]
            fn encode_as(&self) -> &Self::As {
                &self.0
            }
            #[inline(always)]
            fn decode_from(v: Self::As) -> Result<Self, ::parity_scale_codec::Error> {
                Ok(Self(v))
            }
        }
        impl PartialOrd<usize> for $ty {
            #[inline(always)]
            fn partial_cmp(&self, other: &usize) -> Option<std::cmp::Ordering> {
                self.0.partial_cmp(&(*other as u64))
            }
        }
        impl PartialEq<usize> for $ty {
            #[inline(always)]
            fn eq(&self, other: &usize) -> bool {
                self.0 == *other as u64
            }
        }
        impl std::ops::Add<i32> for $ty {
            type Output = Self;
            #[inline(always)]
            fn add(self, other: i32) -> Self {
                Self(self.0 + u64::try_from(other).unwrap())
            }
        }

        impl_from!($ty, u64);
        impl_from!($ty, usize);
    };
}

u64_wrapper!(BlockNumber);
// u64_wrapper!(ChainId);
// u64_wrapper!(NetworkId);
u64_wrapper!(TxIndex);
