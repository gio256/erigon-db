use crate::kv::traits::{DefaultFlags, Mode, Table};
use crate::kv::{tables::TableHandle, MdbxTx};
use ethereum_types::{Address, H256, H64, U256};
use eyre::{eyre, Result};
use fastrlp::{Decodable, Encodable};
use mdbx::TransactionKind;

use tables::*;

pub mod models;
pub mod tables;

use models::{Account, BlockHeader, BodyForStorage, HeaderKey, Rlp};

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
    pub fn read_incarnation(&self, who: Address) -> Result<u64> {
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
    pub fn read_header_number(&self, hash: H256) -> Result<u64> {
        self.read::<HeaderNumber>(hash)?.ok_or(eyre!("No value"))
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
    pub fn write_header(&self, k: HeaderKey, v: BlockHeader) -> Result<()> {
        self.write::<Header>(k, v)
    }
    pub fn write_body_for_storage(&self, k: HeaderKey, v: BodyForStorage) -> Result<()> {
        self.write::<BlockBody>(k, v)
    }
}
