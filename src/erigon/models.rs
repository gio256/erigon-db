use bytes::{Buf, Bytes};
use derive_more::{Deref, DerefMut};
use ethereum_types::{Address, Bloom, H256, H64, U256};
use eyre::{eyre, Result};
use fastrlp::{
    BufMut, Decodable, DecodeError, Encodable, RlpDecodable, RlpDecodableWrapper, RlpEncodable,
    RlpMaxEncodedLen,
};
use parity_scale_codec::{Decode, Encode};
use serde::{Deserialize, Serialize};

use crate::kv::{
    tables::{TooLong, TooShort, VariableVec},
    traits::{TableDecode, TableEncode},
};

pub const KECCAK_LENGTH: usize = H256::len_bytes();
pub const ADDRESS_LENGTH: usize = Address::len_bytes();
pub const U64_LENGTH: usize = std::mem::size_of::<u64>();
pub const BLOOM_BYTE_LENGTH: usize = 256;
// Keccak-256 hash of an empty string, KEC("").
pub const EMPTY_HASH: H256 = H256(hex_literal::hex!(
    "c5d2460186f7233c927e7db2dcc703c0e500b653ca82273b7bfad8045d85a470"
));

macro_rules! bytes_wrapper {
    ($t:ident) => {
        #[derive(
            Clone,
            Debug,
            PartialEq,
            Eq,
            Default,
            Deref,
            DerefMut,
            Serialize,
            Deserialize,
            Encode,
            Decode,
            RlpEncodable,
            RlpDecodable,
        )]
        pub struct $t(pub Bytes);
        impl TableEncode for $t {
            type Encoded = bytes::Bytes;
            fn encode(self) -> Self::Encoded {
                self.0
            }
        }

        impl TableDecode for $t {
            fn decode(b: &[u8]) -> Result<Self> {
                TableDecode::decode(b).map(Self)
            }
        }
    };
}
bytes_wrapper!(Rlp);
bytes_wrapper!(Bytecode);

macro_rules! table_key {
    ($name:ident($($t:ty),+)) => {
        #[derive(
            Clone,
            Copy,
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
        pub struct $name($(pub $t),+);

        impl $name {
            pub const SIZE: usize = 0 $(+ std::mem::size_of::<$t>())+;
        }
    }
}

table_key!(HeaderKey(BlockNumber, H256));

impl TableEncode for HeaderKey {
    type Encoded = VariableVec<{ Self::SIZE }>;
    fn encode(self) -> Self::Encoded {
        let mut out = Self::Encoded::default();
        out.try_extend_from_slice(&self.0.encode()).unwrap();
        out.try_extend_from_slice(&self.1.encode()).unwrap();
        out
    }
}

impl TableDecode for HeaderKey {
    fn decode(b: &[u8]) -> Result<Self> {
        if b.len() > Self::SIZE {
            return Err(TooLong::<{ Self::SIZE }> { got: b.len() }.into());
        }
        if b.len() < U64_LENGTH {
            return Err(TooShort::<{ U64_LENGTH }> { got: b.len() }.into());
        }
        let (num, hash) = b.split_at(U64_LENGTH);
        Ok(Self(TableDecode::decode(num)?, TableDecode::decode(hash)?))
    }
}

impl From<(BlockNumber, H256)> for HeaderKey {
    fn from(src: (BlockNumber, H256)) -> Self {
        Self(src.0, src.1)
    }
}

// (address, storage_key, block_number)
table_key!(StorageHistKey(Address, H256, BlockNumber));

impl TableEncode for StorageHistKey {
    type Encoded = VariableVec<{ Self::SIZE }>;
    fn encode(self) -> Self::Encoded {
        let mut out = Self::Encoded::default();
        out.try_extend_from_slice(&self.0.encode()).unwrap();
        out.try_extend_from_slice(&self.1.encode()).unwrap();
        out.try_extend_from_slice(&self.2.encode()).unwrap();
        out
    }
}

impl TableDecode for StorageHistKey {
    fn decode(b: &[u8]) -> Result<Self> {
        if b.len() > Self::SIZE {
            return Err(TooLong::<{ Self::SIZE }> { got: b.len() }.into());
        }
        if b.len() < ADDRESS_LENGTH + KECCAK_LENGTH {
            return Err(TooShort::<{ ADDRESS_LENGTH + KECCAK_LENGTH }> { got: b.len() }.into());
        }
        let (adr, rest) = b.split_at(ADDRESS_LENGTH);
        let (key, shard_id) = rest.split_at(KECCAK_LENGTH);
        Ok(Self(
            TableDecode::decode(adr)?,
            TableDecode::decode(key)?,
            TableDecode::decode(shard_id)?,
        ))
    }
}

/// Key for the PlainContractCode table (address | incarnation)
table_key!(PlainCodeKey(Address, Incarnation));

impl TableEncode for PlainCodeKey {
    type Encoded = VariableVec<{ Self::SIZE }>;
    fn encode(self) -> Self::Encoded {
        let mut out = Self::Encoded::default();
        out.try_extend_from_slice(&self.0.encode()).unwrap();
        out.try_extend_from_slice(&self.1.encode()).unwrap();
        out
    }
}

impl TableDecode for PlainCodeKey {
    fn decode(b: &[u8]) -> Result<Self> {
        if b.len() > Self::SIZE {
            return Err(TooLong::<{ Self::SIZE }> { got: b.len() }.into());
        }
        if b.len() < ADDRESS_LENGTH {
            return Err(TooShort::<{ ADDRESS_LENGTH }> { got: b.len() }.into());
        }
        let (fst, snd) = b.split_at(ADDRESS_LENGTH);
        Ok(Self(TableDecode::decode(fst)?, TableDecode::decode(snd)?))
    }
}

table_key!(ContractCodeKey(H256, Incarnation));

impl ContractCodeKey {
    fn new(who: Address, inc: Incarnation) -> Self {
        Self(ethers::utils::keccak256(who).into(), inc)
    }
}

impl TableEncode for ContractCodeKey {
    type Encoded = VariableVec<{ Self::SIZE }>;
    fn encode(self) -> Self::Encoded {
        let mut out = Self::Encoded::default();
        out.try_extend_from_slice(&self.0.encode()).unwrap();
        out.try_extend_from_slice(&self.1.encode()).unwrap();
        out
    }
}

impl TableDecode for ContractCodeKey {
    fn decode(b: &[u8]) -> Result<Self> {
        if b.len() > Self::SIZE {
            return Err(TooLong::<{ Self::SIZE }> { got: b.len() }.into());
        }
        if b.len() < KECCAK_LENGTH {
            return Err(TooShort::<{ KECCAK_LENGTH }> { got: b.len() }.into());
        }
        let (fst, snd) = b.split_at(KECCAK_LENGTH);
        Ok(Self(TableDecode::decode(fst)?, TableDecode::decode(snd)?))
    }
}

/// Key for the HashedStorage table (keccak(address) | incarnation | keccak(storage_key))
table_key!(HashStorageKey(H256, Incarnation, H256));

impl TableEncode for HashStorageKey {
    type Encoded = VariableVec<{ Self::SIZE }>;
    fn encode(self) -> Self::Encoded {
        let mut out = Self::Encoded::default();
        out.try_extend_from_slice(&self.0.encode()).unwrap();
        out.try_extend_from_slice(&self.1.encode()).unwrap();
        out.try_extend_from_slice(&self.2.encode()).unwrap();
        out
    }
}

impl HashStorageKey {
    pub fn new(who: Address, inc: Incarnation, key: H256) -> Self {
        Self(
            ethers::utils::keccak256(who).into(),
            inc,
            ethers::utils::keccak256(key).into(),
        )
    }
}

impl TableEncode for (Address, Account) {
    type Encoded = Vec<u8>;
    fn encode(self) -> Self::Encoded {
        let mut out = Self::Encoded::default();
        out.extend_from_slice(&self.0.encode());
        out.append(&mut self.1.encode());
        out
    }
}

impl TableDecode for (Address, Account) {
    fn decode(b: &[u8]) -> Result<Self> {
        if b.len() < ADDRESS_LENGTH {
            return Err(TooShort::<{ ADDRESS_LENGTH }> { got: b.len() }.into());
        }
        let (adr, acct) = b.split_at(ADDRESS_LENGTH);
        Ok((TableDecode::decode(adr)?, TableDecode::decode(acct)?))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct Account {
    pub nonce: u64,
    pub incarnation: Incarnation,
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
            acct.incarnation = parse_u64_with_len(&mut enc).into();
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
        // TODO: erigon docs mention additional storage hash field, code seems to disagree
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
    pub fn incarnation(mut self, inc: Incarnation) -> Self {
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
    pub incarnation: Incarnation,
}
impl StorageKey {
    pub fn new(address: Address, incarnation: Incarnation) -> Self {
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

macro_rules! rlp_table_value {
    ($t:ty) => {
        impl TableEncode for $t {
            type Encoded = ::bytes::Bytes;
            fn encode(self) -> Self::Encoded {
                let mut buf = ::bytes::BytesMut::new();
                ::fastrlp::Encodable::encode(&self, &mut buf);
                buf.into()
            }
        }
        impl TableDecode for $t {
            fn decode(mut b: &[u8]) -> Result<Self> {
                ::fastrlp::Decodable::decode(&mut b).map_err(From::from)
            }
        }
    };
}

#[derive(
    Clone, Debug, PartialEq, Serialize, Deserialize, Encode, Decode, RlpEncodable, RlpDecodable,
)]
pub struct BodyForStorage {
    pub base_tx_id: u64,
    pub tx_amount: u32,
    pub uncles: Vec<BlockHeader>,
}
rlp_table_value!(BodyForStorage);

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
    pub base_fee: Option<U256>,
    pub seal: Option<Rlp>,
}
rlp_table_value!(BlockHeader);

impl BlockHeader {
    fn rlp_header(&self) -> fastrlp::Header {
        let mut rlp_head = fastrlp::Header {
            list: true,
            payload_length: 0,
        };

        rlp_head.payload_length += KECCAK_LENGTH + 1; // parent_hash
        rlp_head.payload_length += KECCAK_LENGTH + 1; // ommers_hash
        rlp_head.payload_length += ADDRESS_LENGTH + 1; // beneficiary
        rlp_head.payload_length += KECCAK_LENGTH + 1; // state_root
        rlp_head.payload_length += KECCAK_LENGTH + 1; // transactions_root
        rlp_head.payload_length += KECCAK_LENGTH + 1; // receipts_root
        rlp_head.payload_length += BLOOM_BYTE_LENGTH + fastrlp::length_of_length(BLOOM_BYTE_LENGTH); // logs_bloom
        rlp_head.payload_length += self.difficulty.length(); // difficulty
        rlp_head.payload_length += self.number.length(); // block height
        rlp_head.payload_length += self.gas_limit.length(); // gas_limit
        rlp_head.payload_length += self.gas_used.length(); // gas_used
        rlp_head.payload_length += self.time.length(); // timestamp
        rlp_head.payload_length += self.extra.length(); // extra_data

        rlp_head.payload_length += KECCAK_LENGTH + 1; // mix_hash
        rlp_head.payload_length += 8 + 1; // nonce

        if let Some(base_fee) = self.base_fee {
            rlp_head.payload_length += base_fee.length();
        }

        rlp_head
    }
}

impl Encodable for BlockHeader {
    fn encode(&self, out: &mut dyn BufMut) {
        self.rlp_header().encode(out);
        Encodable::encode(&self.parent_hash, out);
        Encodable::encode(&self.uncle_hash, out);
        Encodable::encode(&self.coinbase, out);
        Encodable::encode(&self.root, out);
        Encodable::encode(&self.tx_hash, out);
        Encodable::encode(&self.receipts_hash, out);
        Encodable::encode(&self.bloom, out);
        Encodable::encode(&self.difficulty, out);
        Encodable::encode(&self.number, out);
        Encodable::encode(&self.gas_limit, out);
        Encodable::encode(&self.gas_used, out);
        Encodable::encode(&self.time, out);
        Encodable::encode(&self.extra, out);
        Encodable::encode(&self.mix_digest, out);
        Encodable::encode(&self.nonce, out);
        if let Some(base_fee) = self.base_fee {
            Encodable::encode(&base_fee, out);
        }
    }
    fn length(&self) -> usize {
        let rlp_head = self.rlp_header();
        fastrlp::length_of_length(rlp_head.payload_length) + rlp_head.payload_length
    }
}

// https://github.com/ledgerwatch/erigon/blob/156da607e7495d709c141aec40f66a2556d35dc0/core/types/block.go#L430
impl Decodable for BlockHeader {
    fn decode(buf: &mut &[u8]) -> Result<Self, DecodeError> {
        let rlp_head = fastrlp::Header::decode(buf)?;
        if !rlp_head.list {
            return Err(DecodeError::UnexpectedString);
        }
        let rest = buf.len() - rlp_head.payload_length;
        let parent_hash = Decodable::decode(buf)?;
        let uncle_hash = Decodable::decode(buf)?;
        let coinbase = Decodable::decode(buf)?;
        let root = Decodable::decode(buf)?;
        let tx_hash = Decodable::decode(buf)?;
        let receipts_hash = Decodable::decode(buf)?;
        let bloom = Decodable::decode(buf)?;
        let difficulty = Decodable::decode(buf)?;
        let number = Decodable::decode(buf)?;
        let gas_limit = Decodable::decode(buf)?;
        let gas_used = Decodable::decode(buf)?;
        let time = Decodable::decode(buf)?;
        let extra = Decodable::decode(buf)?;

        // TODO: seal fields
        let seal = None;
        let mix_digest = Decodable::decode(buf)?;
        let nonce = Decodable::decode(buf)?;
        let base_fee = if buf.len() > rest {
            Some(Decodable::decode(buf)?)
        } else {
            None
        };

        Ok(Self {
            parent_hash,
            uncle_hash,
            coinbase,
            root,
            tx_hash,
            receipts_hash,
            bloom,
            difficulty,
            number,
            gas_limit,
            gas_used,
            time,
            extra,
            mix_digest,
            nonce,
            base_fee,
            seal,
        })
    }
}

// The TxSender table stores addresses with no serialization format (new address every 20 bytes)
impl TableEncode for Vec<Address> {
    type Encoded = Vec<u8>;

    fn encode(self) -> Self::Encoded {
        let mut v = Vec::with_capacity(self.len() * ADDRESS_LENGTH);
        for addr in self {
            v.extend_from_slice(&addr.encode());
        }
        v
    }
}

impl TableDecode for Vec<Address> {
    fn decode(b: &[u8]) -> Result<Self> {
        if b.len() % ADDRESS_LENGTH != 0 {
            eyre::bail!("Slice len should be divisible by {}", ADDRESS_LENGTH);
        }

        let mut v = Vec::with_capacity(b.len() / ADDRESS_LENGTH);
        for i in 0..b.len() / ADDRESS_LENGTH {
            let offset = i * ADDRESS_LENGTH;
            v.push(TableDecode::decode(&b[offset..offset + ADDRESS_LENGTH])?);
        }

        Ok(v)
    }
}

// -- macros from Akula, largely unaltered

macro_rules! impl_ops {
    ($type:ty, $other:ty) => {
        impl std::ops::Add<$other> for $type {
            type Output = Self;
            #[inline(always)]
            fn add(self, other: $other) -> Self {
                Self(
                    self.0
                        + u64::try_from(other)
                            .unwrap_or_else(|_| unsafe { std::hint::unreachable_unchecked() }),
                )
            }
        }
        impl std::ops::Sub<$other> for $type {
            type Output = Self;
            #[inline(always)]
            fn sub(self, other: $other) -> Self {
                Self(
                    self.0
                        - u64::try_from(other)
                            .unwrap_or_else(|_| unsafe { std::hint::unreachable_unchecked() }),
                )
            }
        }
        impl std::ops::Mul<$other> for $type {
            type Output = Self;
            #[inline(always)]
            fn mul(self, other: $other) -> Self {
                Self(
                    self.0
                        * u64::try_from(other)
                            .unwrap_or_else(|_| unsafe { std::hint::unreachable_unchecked() }),
                )
            }
        }
        impl std::ops::Div<$other> for $type {
            type Output = Self;
            #[inline(always)]
            fn div(self, other: $other) -> Self {
                Self(
                    self.0
                        / u64::try_from(other)
                            .unwrap_or_else(|_| unsafe { std::hint::unreachable_unchecked() }),
                )
            }
        }
        impl std::ops::Rem<$other> for $type {
            type Output = Self;
            #[inline(always)]
            fn rem(self, other: $other) -> Self {
                Self(
                    self.0
                        % u64::try_from(other)
                            .unwrap_or_else(|_| unsafe { std::hint::unreachable_unchecked() }),
                )
            }
        }
        impl std::ops::AddAssign<$other> for $type {
            #[inline(always)]
            fn add_assign(&mut self, other: $other) {
                self.0 += u64::try_from(other)
                    .unwrap_or_else(|_| unsafe { std::hint::unreachable_unchecked() });
            }
        }
        impl std::ops::SubAssign<$other> for $type {
            #[inline(always)]
            fn sub_assign(&mut self, other: $other) {
                self.0 -= u64::try_from(other)
                    .unwrap_or_else(|_| unsafe { std::hint::unreachable_unchecked() });
            }
        }
        impl std::ops::MulAssign<$other> for $type {
            #[inline(always)]
            fn mul_assign(&mut self, other: $other) {
                self.0 *= u64::try_from(other)
                    .unwrap_or_else(|_| unsafe { std::hint::unreachable_unchecked() });
            }
        }
        impl std::ops::DivAssign<$other> for $type {
            #[inline(always)]
            fn div_assign(&mut self, other: $other) {
                self.0 /= u64::try_from(other)
                    .unwrap_or_else(|_| unsafe { std::hint::unreachable_unchecked() });
            }
        }
        impl std::ops::RemAssign<$other> for $type {
            #[inline(always)]
            fn rem_assign(&mut self, other: $other) {
                self.0 %= u64::try_from(other)
                    .unwrap_or_else(|_| unsafe { std::hint::unreachable_unchecked() });
            }
        }
    };
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
            derive_more::Deref,
            derive_more::DerefMut,
            Default,
            derive_more::Display,
            Eq,
            derive_more::From,
            derive_more::FromStr,
            PartialEq,
            PartialOrd,
            Ord,
            Hash,
            Serialize,
            Deserialize,
            RlpEncodable,
            RlpDecodableWrapper,
            RlpMaxEncodedLen,
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

        impl_ops!($ty, u8);
        impl_ops!($ty, u64);
        impl_ops!($ty, usize);
        impl_ops!($ty, $ty);
    };
}

u64_wrapper!(BlockNumber);
u64_wrapper!(Incarnation);
crate::u64_table_object!(BlockNumber);
crate::u64_table_object!(Incarnation);
