use crate::kv::traits::{DefaultFlags, Mode, Table};
use crate::kv::{tables::TableHandle, MdbxTx};
use ethereum_types::{Address, H256, H64, U256};
use eyre::{eyre, Result};
use mdbx::TransactionKind;
use fastrlp::{Encodable, Decodable};

use tables::*;

pub mod models;
pub mod tables;

use models::{Account, Rlp, BlockHeader, HeaderKey};

/// Erigon wraps an MdbxTx and provides Erigon-specific access methods.
pub struct Erigon<'env, K: TransactionKind>(pub MdbxTx<'env, K>);

impl<'env, K: Mode> Erigon<'env, K> {
    /// Opens and reads from the db table with the table's default flags
    pub fn read<'tx, T>(&'tx self, key: T::Key) -> Result<Option<T::Value>>
    where
        T: Table<'tx> + DefaultFlags,
    {
        self.0.get::<T, T::Flags>(self.0.open_db()?, key)
    }

    pub fn read_head_header_hash(&self) -> Result<H256> {
        self.read::<LastHeader>(LastHeader)?
            .ok_or(eyre!("No LastHeader"))
    }
    pub fn read_head_block_hash(&self) -> Result<H256> {
        self.read::<LastBlock>(LastBlock)?
            .ok_or(eyre!("No LastHeader"))
    }
    /// Returns the incarnation of the account when it was last deleted.
    /// If the account is not in the db, returns 0.
    pub fn read_incarnation(&self, who: Address) -> Result<u64> {
        self.read::<IncarnationMap>(who)
            .map(|v| v.unwrap_or_default())
    }
    pub fn read_account_data(&self, who: Address) -> Result<Account> {
        self.read::<PlainState>(who).map(|v| v.unwrap_or_default())
    }
    /// Returns the number of the block containing the specified transaction.
    pub fn read_transaction_block_number(&self, hash: H256) -> Result<U256> {
        self.read::<BlockTransactionLookup>(hash)?
            .ok_or(eyre!("No transaction"))
    }
    pub fn read_header(&self, num: u64, hash: H256) -> Result<BlockHeader> {
        self.read::<Header>(HeaderKey(num, hash))?.ok_or(eyre!("No header"))
    }
    pub fn read_header_number(&self, hash: H256) -> Result<u64> {
        self.read::<HeaderNumber>(hash)?.ok_or(eyre!("No header number"))
    }
}
impl<'env> Erigon<'env, mdbx::RW> {
    /// Opens and writes to the db table with the table's default flags
    pub fn write<'tx, T>(&'tx self, key: T::Key, val: T::Value) -> Result<()>
    where
        T: Table<'tx> + DefaultFlags,
    {
        self.0.set::<T, T::Flags>(self.0.open_db()?, key, val)
    }

    pub fn write_head_header_hash(&self, v: H256) -> Result<()> {
        self.write::<LastHeader>(LastHeader, v)
    }
    pub fn write_head_block_hash(&self, v: H256) -> Result<()> {
        self.write::<LastBlock>(LastBlock, v)
    }
    pub fn write_incarnation(&self, k: Address, v: u64) -> Result<()> {
        self.write::<IncarnationMap>(k, v)
    }
    pub fn write_account_data(&self, k: Address, v: Account) -> Result<()> {
        self.write::<PlainState>(k, v)
    }
    pub fn write_transaction_block_number(&self, k: H256, v: U256) -> Result<()> {
        self.write::<BlockTransactionLookup>(k, v)
    }
    pub fn write_header_number(&self, k: H256, v: u64) -> Result<()> {
        self.write::<HeaderNumber>(k, v)
    }
    pub fn write_header(&self, num: u64, hash: H256, header: BlockHeader) -> Result<()> {
        self.write::<Header>(HeaderKey(num, hash), header)
    }
}

// // https://github.com/ledgerwatch/erigon-lib/blob/625c9f5385d209dc2abfadedf6e4b3914a26ed3e/kv/mdbx/kv_mdbx.go#L154
// const ENV_FLAGS: EnvFlags = EnvFlags {
//     // Disable readahead. Improves performance when db size > RAM.
//     no_rdahead: true,
//     // Try to coalesce while garbage collecting. (https://en.wikipedia.org/wiki/Coalescing_(computer_science))
//     coalesce: true,
//     // If another process is using the db with different flags, open in
//     // compatibility mode instead of MDBX_INCOMPATIBLE error.
//     accede: true,
//     no_sub_dir: false,
//     exclusive: false,
//     no_meminit: false,
//     liforeclaim: false,
// };
