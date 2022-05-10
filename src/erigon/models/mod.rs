use bytes::Bytes;
use ethereum_types::{Address, H256, U256};
use eyre::Result;

use crate::{
    erigon::{macros::*, utils::*},
    kv::{
        tables::VariableVec,
        traits::{TableDecode, TableEncode},
    },
};

pub mod transaction;
pub use transaction::Transaction;
pub mod block;
pub use block::*;
pub mod account;
pub use account::*;
pub mod log;
pub use log::*;

use crate::erigon::utils::consts::*;

// the LastHeader table stores only one key, bytes("LastHeader")
constant_key!(LastHeaderKey, LastHeader);
// the LastBlock table stores only one key, bytes("LastBlock")
constant_key!(LastBlockKey, LastBlock);

// u64 newtype aliases
u64_wrapper!(BlockNumber);
u64_wrapper!(Incarnation);
u64_wrapper!(TxIndex);

// blocknum||blockhash
tuple_key!(HeaderKey(BlockNumber, H256));
tuple_key!(AccountHistKey(Address, BlockNumber));
tuple_key!(StorageKey(Address, Incarnation));

// values for the StorageChangeSet table. slot||value
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

// values for the AccountChangeSet table. address||encode(account)
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

decl_u256_wrapper!(TotalDifficulty);
rlp_table_value!(TotalDifficulty);

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
