use crate::kv::{
    tables::TableHandle,
    traits::{DefaultFlags, Mode, Table},
    MdbxCursor, MdbxTx,
};
use ethereum_types::{Address, H256, H64, U256};
use eyre::{eyre, Result};
use fastrlp::{Decodable, Encodable};
use mdbx::TransactionKind;

use tables::*;

pub mod models;
pub mod tables;

use models::{
    Account, BlockHeader, BlockNumber, BodyForStorage, Bytecode, HeaderKey, Incarnation, Rlp,
    StorageKey,
};

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
    pub fn cursor<'tx, T>(&'tx self) -> Result<MdbxCursor<'tx, K, T>>
    where
        T: Table<'tx> + DefaultFlags,
    {
        self.0.cursor::<T, T::Flags>(self.0.open_db()?)
    }

    /// Returns the hash of the current canonical head header.
    pub fn read_head_header_hash(&self) -> Result<H256> {
        self.read::<LastHeader>(LastHeader)?
            .ok_or(eyre!("No value"))
    }

    /// Returns the hash of the current canonical head block.
    pub fn read_head_block_hash(&self) -> Result<H256> {
        self.read::<LastBlock>(LastBlock)?.ok_or(eyre!("No value"))
    }

    /// Returns the incarnation of the account when it was last deleted.
    pub fn read_incarnation(&self, who: Address) -> Result<Incarnation> {
        self.read::<IncarnationMap>(who)?.ok_or(eyre!("No value"))
    }

    /// Returns the decoded account data as stored in the PlainState table.
    pub fn read_account_data(&self, who: Address) -> Result<Account> {
        self.read::<PlainState>(who)?.ok_or(eyre!("No value"))
    }

    /// Returns the number of the block containing the specified transaction.
    pub fn read_transaction_block_number(&self, hash: H256) -> Result<U256> {
        self.read::<BlockTransactionLookup>(hash)?
            .ok_or(eyre!("No value"))
    }

    /// Returns the block header identified by the (block number, block hash) key
    pub fn read_header(&self, key: HeaderKey) -> Result<BlockHeader> {
        self.read::<Header>(key)?.ok_or(eyre!("No value"))
    }

    /// Returns the decoding of the body as stored in the BlockBody table
    pub fn read_body_for_storage(&self, key: HeaderKey) -> Result<BodyForStorage> {
        let mut body = self.read::<BlockBody>(key)?.ok_or(eyre!("No value"))?;

        // Skip 1 system tx at the beginning of the block and 1 at the end
        // https://github.com/ledgerwatch/erigon/blob/f56d4c5881822e70f65927ade76ef05bfacb1df4/core/rawdb/accessors_chain.go#L602-L605
        body.base_tx_id += 1;
        body.tx_amount = body.tx_amount.checked_sub(2).ok_or_else(|| {
            eyre!(
                "Block body has too few txs: {}. HeaderKey: {:?}",
                body.tx_amount,
                key,
            )
        })?;
        Ok(body)
    }

    /// Returns the header number assigned to a hash.
    pub fn read_header_number(&self, hash: H256) -> Result<BlockNumber> {
        self.read::<HeaderNumber>(hash)?.ok_or(eyre!("No value"))
    }

    /// Returns the number of the current canonical block header
    pub fn read_head_block_number(&mut self) -> Result<BlockNumber> {
        let hash = self.read_head_header_hash()?;
        self.read_header_number(hash)
    }

    /// Returns the signers of each transaction in the block.
    pub fn read_senders(&mut self, key: HeaderKey) -> Result<Vec<Address>> {
        self.read::<TxSender>(key)?.ok_or(eyre!("No value"))
    }

    /// Returns the hash assigned to a canonical block number.
    pub fn read_canonical_hash(&mut self, num: BlockNumber) -> Result<H256> {
        self.read::<CanonicalHeader>(num)?.ok_or(eyre!("No value"))
    }

    /// Determines whether a header with the given hash is on the canonical chain.
    pub fn is_canonical_hash(&mut self, hash: H256) -> Result<bool> {
        let num = self.read_header_number(hash)?;
        let can_hash = self.read_canonical_hash(num)?;
        Ok(can_hash != Default::default() && can_hash == hash)
    }

    /// Returns the value of the storage for account `who` indexed by `key`.
    pub fn read_storage(
        &mut self,
        who: Address,
        inc: Incarnation,
        key: H256,
    ) -> Result<Option<U256>> {
        let bucket = StorageKey::new(who, inc);
        let mut cur = self.cursor::<Storage>()?;
        cur.seek_both_range(bucket, key)
            .map(|kv| kv.and_then(|(k, v)| if k == key { Some(v) } else { None }))
    }

    /// Returns an iterator over all of the storage (key, value) pairs for the
    /// given address and account incarnation.
    pub fn walk_storage(
        &mut self,
        who: Address,
        inc: Incarnation,
    ) -> Result<impl Iterator<Item = Result<(H256, U256)>>> {
        let start_key = StorageKey::new(who, inc);
        self.cursor::<Storage>()?.walk_dup(start_key)
    }

    /// Returns the code associated with the given codehash.
    pub fn read_code(&mut self, codehash: H256) -> Result<Bytecode> {
        if codehash == models::EMPTY_HASH {
            return Ok(Default::default());
        }
        self.read::<Code>(codehash)?.ok_or(eyre!("No value"))
    }

    /// Returns the length of the code associated with the given codehash.
    pub fn read_code_size(&mut self, codehash: H256) -> Result<usize> {
        Ok(self.read_code(codehash)?.len())
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
    pub fn write_incarnation(&self, k: Address, v: Incarnation) -> Result<()> {
        self.write::<IncarnationMap>(k, v)
    }
    pub fn write_account_data(&self, k: Address, v: Account) -> Result<()> {
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
