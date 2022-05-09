use ethereum_types::{Address, H256};

pub const KECCAK_LENGTH: usize = H256::len_bytes();
pub const ADDRESS_LENGTH: usize = Address::len_bytes();
pub const U64_LENGTH: usize = std::mem::size_of::<u64>();
pub const BLOOM_BYTE_LENGTH: usize = 256;

// keccak256("")
pub const EMPTY_HASH: H256 = H256(hex_literal::hex!(
    "c5d2460186f7233c927e7db2dcc703c0e500b653ca82273b7bfad8045d85a470"
));
