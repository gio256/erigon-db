use crate::{
    decl_table,
    kv::{tables::TableHandle, traits::*, EnvFlags},
};
use ethers::types::{Address, H256, U256};
use mdbx::DatabaseFlags;

pub mod models;

// https://github.com/ledgerwatch/erigon-lib/blob/625c9f5385d209dc2abfadedf6e4b3914a26ed3e/kv/mdbx/kv_mdbx.go#L154
const ENV_FLAGS: EnvFlags = EnvFlags {
    // Disable readahead. Improves performance when db size > RAM.
    no_rdahead: true,
    // Try to coalesce while garbage collecting. (https://en.wikipedia.org/wiki/Coalescing_(computer_science))
    coalesce: true,
    // If another process is using the db with different flags, open in
    // compatibility mode instead of MDBX_INCOMPATIBLE error.
    accede: true,
    no_sub_dir: false,
    exclusive: false,
    no_meminit: false,
    liforeclaim: false,
};

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

// every query of the LastHeader table takes the same key, LastHeader
encode_const!(LastHeader);
decl_table!(LastHeader => LastHeader => H256);

// every query of the LastBlock table takes the same key, LastBlock
encode_const!(LastBlock);
decl_table!(LastBlock => LastBlock => H256);

decl_table!(IncarnationMap          => Address  => u64);
decl_table!(BlockTransactionLookup  => H256     => U256);
decl_table!(PlainState              => Address  => models::Account);
decl_table!(HeaderNumber            => H256     => u64); // TODO: should be BlockNumber

// Manually implement storage table because it overlaps with PlainState
#[derive(Debug, Default, Clone, Copy)]
pub struct Storage;
impl<'tx> crate::kv::traits::Table<'tx> for Storage {
    type Key = models::StorageKey;
    type SeekKey = models::StorageKey;
    type Value = (H256, U256);
    type Dbi = crate::kv::tables::TableHandle<'tx, Self, { Self::flags() }>;
}
impl Storage {
    pub const fn flags() -> u32 {
        DatabaseFlags::DUP_SORT.bits()
    }
}
impl crate::kv::traits::DupSort<'_> for Storage {
    type SeekBothKey = H256;
}
