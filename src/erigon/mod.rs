use crate::kv::{
    tables::TableHandle,
    traits::{DefaultFlags, Mode, Table},
    EnvFlags, MdbxCursor, MdbxEnv, MdbxTx,
};
use ethereum_types::{Address, H256, H64, U256};
use eyre::{eyre, Result};
use fastrlp::{Decodable, Encodable};
use mdbx::{TransactionKind, RO, RW};

// mod foo {
//     macro_rules! bar { () => () }
//     pub(crate) use bar;
// }
mod macros;
// macros::bar!();


pub mod models;
pub mod tables;
mod utils;

use tables::*;
use models::*;

pub const NUM_TABLES: usize = 50;
// https://github.com/ledgerwatch/erigon-lib/blob/625c9f5385d209dc2abfadedf6e4b3914a26ed3e/kv/mdbx/kv_mdbx.go#L154
pub const ENV_FLAGS: EnvFlags = EnvFlags {
    no_rdahead: true,
    coalesce: true,
    accede: true,
    no_sub_dir: false,
    exclusive: false,
    no_meminit: false,
    liforeclaim: false,
};

/// Open an mdbx env with Erigon-specific configuration.
pub fn env_open<M: Mode>(path: &std::path::Path) -> Result<MdbxEnv<M>> {
    MdbxEnv::<M>::open(path, NUM_TABLES, ENV_FLAGS)
}

/// Erigon wraps an `MdbxTx` and provides Erigon-specific access methods.
pub struct Erigon<'env, K: TransactionKind>(pub MdbxTx<'env, K>);

impl<'env> Erigon<'env, RO> {
    pub fn begin(env: &'env MdbxEnv<RO>) -> Result<Self> {
        env.begin().map(Self)
    }
}
impl<'env> Erigon<'env, RW> {
    pub fn begin_rw(env: &'env MdbxEnv<RW>) -> Result<Self> {
        env.begin_rw().map(Self)
    }
}
impl<'env, K: TransactionKind> Erigon<'env, K> {
    pub fn new(inner: MdbxTx<'env, K>) -> Self {
        Self(inner)
    }
}

impl<'env, K: Mode> Erigon<'env, K> {
    /// Opens and reads from the db table with the table's default flags
    pub fn read<'tx, T>(&'tx self, key: T::Key) -> Result<Option<T::Value>>
    where
        T: Table<'tx> + DefaultFlags,
    {
        self.0.get::<T, T::Flags>(self.0.open_db()?, key)
    }
    /// Opens a table with the table's default flags and creates a cursor into
    /// the opened table.
    pub fn cursor<'tx, T>(&'tx self) -> Result<MdbxCursor<'tx, K, T>>
    where
        T: Table<'tx> + DefaultFlags,
    {
        self.0.cursor::<T, T::Flags>(self.0.open_db()?)
    }

    /// Returns the hash of the current canonical head header.
    pub fn read_head_header_hash(&self) -> Result<Option<H256>> {
        self.read::<LastHeader>(LastHeader)
    }

    /// Returns the hash of the current canonical head block.
    pub fn read_head_block_hash(&self) -> Result<Option<H256>> {
        self.read::<LastBlock>(LastBlock)
    }

    /// Returns the incarnation of the account when it was last deleted.
    pub fn read_incarnation(&self, adr: Address) -> Result<Option<Incarnation>> {
        self.read::<IncarnationMap>(adr)
    }

    /// Returns the decoded account data as stored in the PlainState table.
    pub fn read_account(&self, adr: Address) -> Result<Option<Account>> {
        self.read::<PlainState>(adr)
    }

    /// Returns the number of the block containing the specified transaction.
    pub fn read_transaction_block_number(&self, hash: H256) -> Result<Option<U256>> {
        self.read::<BlockTransactionLookup>(hash)
    }

    /// Returns the block header identified by the (block number, block hash) key
    pub fn read_header(&self, key: impl Into<HeaderKey>) -> Result<Option<BlockHeader>> {
        self.read::<Header>(key.into())
    }

    /// Returns header total difficulty
    pub fn read_total_difficulty(
        &self,
        key: impl Into<HeaderKey>,
    ) -> Result<Option<TotalDifficulty>> {
        self.read::<HeadersTotalDifficulty>(key.into())
    }

    /// Returns the decoding of the body as stored in the BlockBody table
    pub fn read_body_for_storage(
        &self,
        key: impl Into<HeaderKey>,
    ) -> Result<Option<BodyForStorage>> {
        let key = key.into();
        self.read::<BlockBody>(key)?
            .map(|mut body| {
                // Skip 1 system tx at the beginning of the block and 1 at the end
                // https://github.com/ledgerwatch/erigon/blob/f56d4c5881822e70f65927ade76ef05bfacb1df4/core/rawdb/accessors_chain.go#L602-L605
                // https://github.com/ledgerwatch/erigon-lib/blob/625c9f5385d209dc2abfadedf6e4b3914a26ed3e/kv/tables.go#L28
                body.base_tx_id += 1;
                body.tx_amount = body.tx_amount.checked_sub(2).ok_or_else(|| {
                    eyre!(
                        "Block body has too few txs: {}. HeaderKey: {:?}",
                        body.tx_amount,
                        key,
                    )
                })?;
                Ok(body)
            })
            .transpose()
    }

    /// Returns the header number assigned to a hash.
    pub fn read_header_number(&self, hash: H256) -> Result<Option<BlockNumber>> {
        self.read::<HeaderNumber>(hash)
    }

    /// Returns the number of the current canonical block header.
    pub fn read_head_block_number(&self) -> Result<Option<BlockNumber>> {
        let hash = self.read_head_header_hash()?.ok_or(eyre!("No value"))?;
        self.read_header_number(hash)
    }

    /// Returns the signers of each transaction in the block.
    pub fn read_senders(&self, key: impl Into<HeaderKey>) -> Result<Option<Vec<Address>>> {
        self.read::<TxSender>(key.into())
    }

    /// Returns the hash assigned to a canonical block number.
    pub fn read_canonical_hash(&self, num: impl Into<BlockNumber>) -> Result<Option<H256>> {
        self.read::<CanonicalHeader>(num.into())
    }

    /// Determines whether a header with the given hash is on the canonical chain.
    pub fn is_canonical_hash(&self, hash: H256) -> Result<bool> {
        let num = self.read_header_number(hash)?.ok_or(eyre!("No value"))?;
        let canon = self.read_canonical_hash(num)?.ok_or(eyre!("No value"))?;
        Ok(canon != Default::default() && canon == hash)
    }

    /// Returns the value of the storage for account `adr` indexed by `slot`.
    pub fn read_storage(
        &self,
        adr: Address,
        inc: impl Into<Incarnation>,
        slot: H256,
    ) -> Result<Option<U256>> {
        let bucket = StorageKey(adr, inc.into());
        let mut cur = self.cursor::<Storage>()?;
        cur.seek_dup(bucket, slot)
            .map(|kv| kv.and_then(|(k, v)| if k == slot { Some(v) } else { None }))
    }

    /// Returns an iterator over all of the storage (key, value) pairs for the
    /// given address and account incarnation.
    pub fn walk_storage(
        &self,
        adr: Address,
        inc: impl Into<Incarnation>,
    ) -> Result<impl Iterator<Item = Result<(H256, U256)>>> {
        let start_key = StorageKey(adr, inc.into());
        self.cursor::<Storage>()?.walk_dup(start_key)
    }

    /// Returns the code associated with the given codehash.
    pub fn read_code(&self, codehash: H256) -> Result<Option<Bytecode>> {
        if codehash == models::EMPTY_HASH {
            return Ok(Default::default());
        }
        self.read::<Code>(codehash)
    }

    /// Returns the codehash at the `adr` with incarnation `inc`
    pub fn read_codehash(&self, adr: Address, inc: impl Into<Incarnation>) -> Result<Option<H256>> {
        let key = PlainCodeKey(adr, inc.into());
        self.read::<PlainCodeHash>(key)
    }

    // The `AccountChangeSet` table at block `N` stores the state of all accounts
    // changed in block `N` *before* block `N` changed them.
    //
    // The state of an account *after* the most recent change is always stored in the `PlainState` table.
    //
    // If Account A was changed in block 5 and again in block 25, the state of A for any
    // block `[5, 25)` is stored in the `AccountChangeSet` table addressed by the
    // block number 25. If we want to find the state of account `A` at block `B`,
    // we first use the `AccountHistory` table to figure out which block to look for
    // in the `AccountChangeSet` table. That is, we look for the smallest
    // block >= `B` in which account `A` was changed, then we lookup the state
    // of account `A` immediately before that change in the `AccountChangeSet` table.
    //
    // The `AccountHistory` table stores a roaring bitmap of the block numbers
    // in which account `A` was changed. We search the bitmap for the smallest
    // block number it contains which is `>= B`, then we read the state of account
    // `A` at this block from the `AccountChangeSet` table.
    //
    // The confusing thing is, the block number in `AccountHistory` seems to
    // be basically unused. For account `A`, every time a change is made, the
    // bitmap stored at key `(A, u64::MAX)` is updated. Presumably this is used to
    // grow the bitmap, and that's why akula and erigon both do some crazy mapping
    // over the bitmap tables
    //
    // Notes:
    // - `AccountHistory` and `StorageHistory` are written [here](https://github.com/ledgerwatch/erigon/blob/f9d7cb5ca9e8a135a76ddcb6fa4ee526ea383554/core/state/db_state_writer.go#L179).
    // - `GetAsOf()` Erigon implementation [here](https://github.com/ledgerwatch/erigon/blob/f9d7cb5ca9e8a135a76ddcb6fa4ee526ea383554/core/state/history.go#L19).
    //
    /// Returns the state of account `adr` at the given block number.
    pub fn read_account_hist(
        &self,
        adr: Address,
        block: impl Into<BlockNumber>,
    ) -> Result<Option<Account>> {
        let block = block.into();
        let mut hist_cur = self.cursor::<AccountHistory>()?;
        let (_, bitmap) = hist_cur
            .seek((adr, block).into())?
            .ok_or(eyre!("No value"))?;
        let cs_block = match utils::find_gte(bitmap, *block) {
            Some(changeset) => BlockNumber(changeset),
            _ => return Ok(None),
        };
        let mut cs_cur = self.cursor::<AccountChangeSet>()?;
        if let Some(AccountCSVal(k, mut acct)) = cs_cur.seek_dup(cs_block, adr)? {
            if k == adr {
                // recover the codehash
                if acct.incarnation > 0 && acct.codehash == Default::default() {
                    acct.codehash = self
                        .read_codehash(adr, acct.incarnation)?
                        .ok_or(eyre!("No value"))?
                }
                return Ok(Some(acct));
            }
        }
        Ok(None)
    }

    /// Returns the value of an address's storage at the given block number. Returns `None` if the state
    /// is not found in history (e.g., if it's in the PlainState table instead).
    pub fn read_storage_hist(
        &self,
        adr: Address,
        inc: impl Into<Incarnation>,
        slot: H256,
        block: impl Into<BlockNumber>,
    ) -> Result<Option<U256>> {
        let block = block.into();
        let mut hist_cur = self.cursor::<StorageHistory>()?;
        let (_, bitmap) = hist_cur
            .seek((adr, slot, block).into())?
            .ok_or(eyre!("No value"))?;
        let cs_block = match utils::find_gte(bitmap, *block) {
            Some(changeset) => BlockNumber(changeset),
            _ => return Ok(None),
        };
        let cs_key = (cs_block, adr, inc.into()).into();
        let mut cs_cur = self.cursor::<StorageChangeSet>()?;
        if let Some(StorageCSVal(k, v)) = cs_cur.seek_dup(cs_key, slot)? {
            if k == slot {
                return Ok(Some(v));
            }
        }
        Ok(None)
    }
}

impl<'env> Erigon<'env, mdbx::RW> {
    /// Opens and writes to the db table with the table's default flags.
    pub fn write<'tx, T>(&'tx self, key: T::Key, val: T::Value) -> Result<()>
    where
        T: Table<'tx> + DefaultFlags,
    {
        self.0.put::<T, T::Flags>(self.0.open_db()?, key, val)
    }

    pub fn write_head_header_hash(&self, v: H256) -> Result<()> {
        self.write::<LastHeader>(LastHeader, v)
    }
    pub fn write_head_block_hash(&self, v: H256) -> Result<()> {
        self.write::<LastBlock>(LastBlock, v)
    }
    pub fn write_incarnation(&self, k: Address, v: Incarnation) -> Result<()> {
        self.write::<IncarnationMap>(k, v)
    }
    pub fn write_account(&self, k: Address, v: Account) -> Result<()> {
        self.write::<PlainState>(k, v)
    }
    pub fn write_transaction_block_number(&self, k: H256, v: U256) -> Result<()> {
        self.write::<BlockTransactionLookup>(k, v)
    }
    pub fn write_header_number(&self, k: H256, v: BlockNumber) -> Result<()> {
        self.write::<HeaderNumber>(k, v)
    }
    pub fn write_header(&self, k: HeaderKey, v: BlockHeader) -> Result<()> {
        self.write::<Header>(k, v)
    }
    pub fn write_body_for_storage(&self, k: HeaderKey, v: BodyForStorage) -> Result<()> {
        self.write::<BlockBody>(k, v)
    }
}
