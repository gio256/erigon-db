use crate::{
    dupsort_table,
    erigon::models::*,
    kv::{tables::TableHandle, traits::*, EnvFlags},
    table,
};
use ethereum_types::{Address, H256, U256};
use mdbx::DatabaseFlags;

// The latest header and latest block are stored in their own tables, addressed
// by a dummy key ("LastHeader" and "LastBlock", respectively). We encode the
// names of these tables as their own keys to prevent invalid accesses.
macro_rules! encode_const {
    ($name:ident, $encoded:ident) => {
        impl TableEncode for $name {
            type Encoded = Vec<u8>;
            fn encode(self) -> Self::Encoded {
                String::from(stringify!($encoded)).into_bytes()
            }
        }
    };
    ($name:ident) => {
        encode_const!($name, $name);
    };
}

// every query of the LastHeader table takes the same key, "LastHeader"
encode_const!(LastHeader);
// every query of the LastBlock table takes the same key, "LastBlock"
encode_const!(LastBlock);

table!(LastHeader               => LastHeader   => H256);
table!(LastBlock                => LastBlock    => H256);
table!(IncarnationMap           => Address      => Incarnation);
table!(BlockTransactionLookup   => H256         => U256);
//TODO: PlainState dup sorts in reverse?
table!(PlainState               => Address      => Account);
table!(HeaderNumber             => H256         => BlockNumber);
table!(Header                   => HeaderKey    => BlockHeader, SeekKey = BlockNumber);
table!(BlockBody                => HeaderKey    => BodyForStorage, SeekKey = BlockNumber);
table!(PlainCodeHash            => PlainCodeKey => H256);
table!(TxSender                 => HeaderKey    => Vec<Address>);
// block number => header hash
table!(CanonicalHeader          => BlockNumber  => H256);

// keccak(address) => Account
table!(HashedAccount => H256 => Account);
// keccak(address) | incarnation | keccak(storage_key) => storage value
table!(HashedStorage => HashStorageKey => H256);
// table!(AccountsHistory);
// table!(StorageHistory);
table!(Code => H256 => Bytecode);
// keccak256(address) | incarnation => code hash
table!(ContractCode => ContractCodeKey => H256);

// block number => address | encoded account
// dupsort_table!(AccountChangeSet => u64 => (Address, Account), Subkey = Address);
// block number | address | incarnation => plain_storage_key | value
dupsort_table!(StorageChangeSet => (BlockNumber, StorageKey) => (H256, H256), Subkey = H256);

// Manually implement the storage table because it overlaps with PlainState
// (that is, there are two things stored in the table with different key encodings,
// and our macros are too simple to handle this).
#[derive(Debug, Default, Clone, Copy)]
pub struct Storage;
impl<'tx> crate::kv::traits::Table<'tx> for Storage {
    type Name = Self;
    type Key = StorageKey;
    type SeekKey = StorageKey;
    type Value = (H256, U256);
}
impl DbName for Storage {
    const NAME: &'static str = "PlainState";
}
impl crate::kv::traits::DupSort<'_> for Storage {
    type Subkey = H256;
}
impl crate::kv::traits::DefaultFlags for Storage {
    type Flags = crate::kv::tables::DupSortFlags;
}
impl std::fmt::Display for Storage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Storage (PlainState)")
    }
}
