use crate::{dupsort_table, erigon::models::*, table};
use bytes::Bytes;
use ethereum_types::{Address, H256, U256};
use roaring::RoaringTreemap;

// --- Erigon db schema version 6.0.0 ---

// || indicates concatenation

// Table name                   => Key          => Value
// key: bytes("LastHeader"). val: hash of current canonical head header. erigon: HeadHeaderKey
table!(LastHeader               => LastHeaderKey => H256);
// key: bytes("LastBlock"). val: hash of current canonical head block. erigon: HeadBlockKey
table!(LastBlock                => LastBlockKey => H256);
// key: address. val: incarnation of account when it was last deleted
table!(IncarnationMap           => Address      => Incarnation);
// key: tx_hash. val: blocknum containing the tx. erigon: TxLookup
table!(BlockTransactionLookup   => H256         => U256);
// key: header_hash. val: blocknum
table!(HeaderNumber             => H256         => BlockNumber);
// key: blocknum||blockhash. val: rlp(header). erigon: Headers
table!(Header                   => HeaderKey    => BlockHeader, seek_key = BlockNumber);
// key: blocknum||blockhash. val: encode(block_body)
table!(BlockBody                => HeaderKey    => BodyForStorage, seek_key = BlockNumber);
// key: address||incarnation. val: code_hash. erigon: PlainContractCode
table!(PlainCodeHash            => PlainCodeKey => H256);
// key: blocknum||blockhash. val: senders list. erigon: Senders
table!(TxSender                 => HeaderKey    => Vec<Address>);
// key: blocknum. val: blockhash. erigon: HeaderCanonical
table!(CanonicalHeader          => BlockNumber  => H256);
// key: index. val: rlp(tx). transaction. erigon: EthTx
table!(BlockTransaction         => TxIndex      => Transaction);
// key: index. val: rlp(tx). erigon: NonCanonicalTxs
table!(NonCanonicalTransaction  => TxIndex      => Transaction);
// key: address||shard_id_u64. val: bitmap of blocks w/ change. erigon: AccountsHistory
table!(AccountHistory           => AccountHistKey => RoaringTreemap);
// key: address||slot||shard_id_u64. val: bitmap of blocks w/ change.
table!(StorageHistory           => StorageHistKey => RoaringTreemap);
// key: blocknum. val: address||encode(account)
dupsort_table!(AccountChangeSet => BlockNumber  => AccountCSVal, subkey = Address);
// key: blocknum||address||incarnation. val: slot||slot_value
dupsort_table!(StorageChangeSet => StorageCSKey => StorageCSVal, subkey = H256);
// key: address. val: encode(account). PlainState table also contains Storage.
table!(PlainState               => Address      => Account);
// key: address||incarnation. val: slot||slot_value (dupsorted). erigon: PlainState
dupsort_table!(
    Storage => StorageKey => (H256, U256),
    subkey = H256,
    rename = PlainState
);

// key: keccak(address). val: encode(account). erigon: HashedAcccounts
table!(HashedAccount            => H256             => Account);
//TODO: also dupsorted
// key: keccak(address)||incarnation||keccak(slot). val: slot_value
table!(HashedStorage            => HashStorageKey   => U256);
// key: code_hash. val: contract code
table!(Code                     => H256             => Bytecode);
// key: keccak256(address)||incarnation. val: code_hash. erigon: ContractCode
table!(HashedCodeHash           => ContractCodeKey  => H256);
// key: bytestring. val: bytestring. erigon: DatabaseInfo
table!(DbInfo                   => Bytes            => Bytes);
// key: blocknum||blockhash. val: rlp(total_difficulty big.Int). erigon: HeaderTD
table!(HeadersTotalDifficulty   => HeaderKey        => TotalDifficulty);
// key: blocknum. val: total_issued
table!(Issuance                 => BlockNumber      => U256);
// key: bytes("burnt")||bloknum. val: total_burnt. erigon: Issuance
table!(Burnt                    => BurntKey         => U256, rename = Issuance);
// key: code_hash. value: contract_TEVM_code. erigon: ContractTEVMCode. Unused.
table!(TEVMCode                 => H256             => Bytes);

type Todo = Bytes;
// erigon: TrieOfAccounts
table!(TrieAccount => Todo => Todo);
// erigon: TrieOfStorage
table!(TrieStorage => Todo => Todo);
// key: blocknum. val: cbor(receipt). erigon: Receipts
table!(Receipt => BlockNumber => Todo);
// key: blocknum||log_index_in_tx. val: cbor(log). erigon: Log
table!(TransactionLog => (BlockNumber, u32) => Todo);
table!(LogTopicIndex => Todo => Todo);
table!(LogAddressIndex => Todo => Todo);
// key: blocknum||address.
dupsort_table!(CallTraceSet => Todo => Todo, subkey = Todo);
