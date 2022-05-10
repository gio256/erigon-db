use crate::erigon::{
    macros::{cbor_wrapper, tuple_key},
    models::BlockNumber,
};
use bytes::Bytes;
use ethereum_types::{Address, H256};
use serde::{Deserialize, Serialize};

cbor_wrapper!(CborReceipts(Option<Vec<CborReceipt>>));

// blocknum||log_index_in_tx
tuple_key!(LogsKey(BlockNumber, u32));
cbor_wrapper!(CborLogs(Option<Vec<CborLog>>));

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct CborLog {
    address: Address,
    topics: Vec<H256>,
    data: Bytes,
    // block_number: u64,
    // tx_hash: H256,
    // tx_index: usize,
    // block_hash: H256,
    // index: usize,
    // removed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct CborReceipt {
    tx_type: u8, //omitempty
    post_state: Option<H256>,
    status: u64,
    cumulative_gas_used: u64,
}
