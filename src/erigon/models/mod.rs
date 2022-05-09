use bytes::{Buf, Bytes};
use ethereum_types::{Address, Bloom, H256, H64, U256};
use eyre::Result;
use fastrlp::{
    BufMut, Decodable, DecodeError, Encodable, RlpDecodable, RlpDecodableWrapper, RlpEncodable,
    RlpMaxEncodedLen,
};
use serde::{Deserialize, Serialize};

use crate::{
    erigon::{macros::*, utils::*},
    kv::{
        tables::VariableVec,
        traits::{TableDecode, TableEncode},
    },
};

pub mod transaction;
pub use transaction::Transaction;

use crate::erigon::utils::consts::*;

// the LastHeader table stores only one key, bytes("LastHeader")
constant_key!(LastHeaderKey, LastHeader);
// the LastBlock table stores only one key, bytes("LastBlock")
constant_key!(LastBlockKey, LastBlock);

// blocknum||blockhash
tuple_key!(HeaderKey(BlockNumber, H256));
tuple_key!(AccountHistKey(Address, BlockNumber));
tuple_key!(StorageKey(Address, Incarnation));

// slot||value
tuple_key!(StorageCSVal(H256, U256));
// blocknum||address||incarnation
tuple_key!(StorageCSKey(BlockNumber, StorageKey));
impl<B, A, I> From<(B, A, I)> for StorageCSKey
where
    B: Into<BlockNumber>,
    (A, I): Into<StorageKey>,
{
    fn from(src: (B, A, I)) -> Self {
        Self(src.0.into(), (src.1, src.2).into())
    }
}

// address||encode(account)
tuple_key!(AccountCSVal(Address, Account));

// address||storage_slot||block_number
tuple_key!(StorageHistKey(Address, H256, BlockNumber));
// address||incarnation
tuple_key!(PlainCodeKey(Address, Incarnation));

// keccak(address)||incarnation
tuple_key!(ContractCodeKey(H256, Incarnation));
impl ContractCodeKey {
    pub fn make(who: Address, inc: impl Into<Incarnation>) -> Self {
        Self(keccak256(who).into(), inc.into())
    }
}

// keccak(address)||incarnation||keccak(storage_key)
tuple_key!(HashStorageKey(H256, Incarnation, H256));
impl HashStorageKey {
    pub fn make(who: Address, inc: impl Into<Incarnation>, key: H256) -> Self {
        Self(keccak256(who).into(), inc.into(), keccak256(key).into())
    }
}

// The Issuance table also stores the amount burnt, prefixing the encoded block number with "burnt"
// bytes("burnt")||blocknum
declare_tuple!(BurntKey(BlockNumber));
size_tuple!(BurntKey(BlockNumber));
impl TableEncode for BurntKey {
    type Encoded = VariableVec<{ Self::SIZE + 5 }>;
    fn encode(self) -> Self::Encoded {
        let mut out = Self::Encoded::default();
        let prefix = Bytes::from(&b"burnt"[..]);
        out.try_extend_from_slice(&prefix).unwrap();
        out.try_extend_from_slice(&self.0.encode()).unwrap();
        out
    }
}

bytes_wrapper!(Rlp(Bytes));
bytes_wrapper!(Bytecode(Bytes));

#[derive(
    Clone, Copy, Debug, PartialEq, Eq, Default, Deserialize, Serialize, RlpEncodable, RlpDecodable,
)]
pub struct Account {
    pub nonce: u64,
    pub incarnation: Incarnation,
    pub balance: U256,
    pub codehash: H256, // hash of the bytecode
}

impl TableDecode for Account {
    fn decode(mut buf: &[u8]) -> Result<Self> {
        let mut acct = Self::default();

        if buf.is_empty() {
            return Ok(acct);
        }

        let fieldset = buf.get_u8();

        // has nonce
        if fieldset & 1 > 0 {
            acct.nonce = take_u64_rlp(&mut buf)?;
        }

        // has balance
        if fieldset & 2 > 0 {
            let bal_len = buf.get_u8();
            acct.balance = buf[..bal_len.into()].into();
            buf.advance(bal_len.into());
        }

        // has incarnation
        if fieldset & 4 > 0 {
            acct.incarnation = take_u64_rlp(&mut buf)?.into();
        }

        // has codehash
        if fieldset & 8 > 0 {
            let len: usize = buf.get_u8().into();
            if len != KECCAK_LENGTH {
                eyre::bail!(
                    "codehash should be {} bytes long. Got {} instead",
                    KECCAK_LENGTH,
                    len
                );
            }
            acct.codehash = H256::from_slice(&buf[..KECCAK_LENGTH]);
            buf.advance(KECCAK_LENGTH)
        }
        Ok(acct)
    }
}
//TODO: dummy impl as we only need to decode for now, but need the trait bound
impl TableEncode for Account {
    type Encoded = Vec<u8>;
    fn encode(self) -> Self::Encoded {
        unreachable!("Can't encode Account")
    }
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

////

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
pub struct TotalDifficulty(U256);
rlp_table_value!(TotalDifficulty);

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, RlpEncodable, RlpDecodable)]
pub struct BodyForStorage {
    pub base_tx_id: u64,
    pub tx_amount: u32,
    pub uncles: Vec<BlockHeader>,
}
rlp_table_value!(BodyForStorage);

#[derive(Clone, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
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

u64_wrapper!(BlockNumber);
u64_table_key!(BlockNumber);

u64_wrapper!(Incarnation);
u64_table_key!(Incarnation);

u64_wrapper!(TxIndex);
u64_table_key!(TxIndex);
