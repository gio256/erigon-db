use crate::{
    decl_table,
    kv::{tables::TableHandle, traits::*, EnvFlags},
    erigon::models::{StorageKey, Account},
};
use ethereum_types::{Address, H256, U256};
use mdbx::DatabaseFlags;

pub type HeaderKey = (u64, H256);

/// The latest header and latest block are stored in their own tables, addressed
/// by a dummy key ("LastHeader" and "LastBlock", respectively). We encode the
/// names of these tables as their own keys to prevent invalid accesses.
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

decl_table!(LastHeader              => LastHeader       => H256);
decl_table!(LastBlock               => LastBlock        => H256);
decl_table!(IncarnationMap          => Address          => u64);
decl_table!(BlockTransactionLookup  => H256             => U256);
decl_table!(PlainState              => Address          => Account);
decl_table!(HeaderNumber            => H256             => u64); // TODO: should be BlockNumber
decl_table!(PlainContractCode       => (Address, u64)   => H256);
decl_table!(Header       => HeaderKey => Vec<u8>); // RLP encoded headers
                                                   // decl_table!(BlockBody               => HeaderKey        => models::BodyForStorage, SeekKey = u64);

// type HeaderKey = (H256, U256);
// crate::decl_table!(PlainState => models::StorageKey  => HeaderKey, SeekKey = H256);

// Manually implement storage table because it overlaps with PlainState
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
impl Storage {
    pub const fn flags() -> u32 {
        DatabaseFlags::DUP_SORT.bits()
    }
}
impl std::fmt::Display for Storage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Storage")
    }
}
impl crate::kv::traits::DupSort<'_> for Storage {
    type SeekBothKey = H256;
}
