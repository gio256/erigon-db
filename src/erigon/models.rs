use bytes::Buf;
use ethers::types::{Address, H256, U256};
use eyre::{eyre, Result};

use crate::kv::traits::{TableDecode, TableEncode};

const KECCAK_LENGTH: usize = H256::len_bytes();
const ADDRESS_LENGTH: usize = Address::len_bytes();
const U64_LENGTH: usize = std::mem::size_of::<u64>();

#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct Account {
    pub nonce: u64,
    pub incarnation: u64,
    pub balance: U256,
    pub codehash: H256, // hash of the bytecode
}

impl TableDecode for Account {
    fn decode(mut enc: &[u8]) -> Result<Self> {
        let mut acct = Self::default();

        if enc.is_empty() {
            return Ok(acct);
        }

        let fieldset = enc.get_u8();

        // has nonce
        if fieldset & 1 > 0 {
            acct.nonce = parse_u64_with_len(&mut enc);
        }

        // has balance
        if fieldset & 2 > 0 {
            let bal_len = enc.get_u8();
            acct.balance = enc[..bal_len.into()].into();
            enc.advance(bal_len.into());
        }

        // has incarnation
        if fieldset & 4 > 0 {
            acct.incarnation = parse_u64_with_len(&mut enc);
        }

        // has codehash
        if fieldset & 8 > 0 {
            let len: usize = enc.get_u8().into();
            if len != KECCAK_LENGTH {
                eyre::bail!(
                    "codehash should be {} bytes long. Got {} instead",
                    KECCAK_LENGTH,
                    len
                );
            }
            acct.codehash = H256::from_slice(&enc[..KECCAK_LENGTH]);
            enc.advance(KECCAK_LENGTH)
        }

        // TODO: erigon docs mention storage hash field, code seems to disagree
        if enc.remaining() > 0 {
            eyre::bail!("unexpected account field")
        }

        Ok(acct)
    }
}
//TODO: dummy impl as we only need to decode for now, but need the trait bound
impl TableEncode for Account {
    type Encoded = Vec<u8>;
    fn encode(self) -> Self::Encoded {
        Self::Encoded::default()
    }
}

pub fn parse_u64_with_len(enc: &mut &[u8]) -> u64 {
    let len = enc.get_u8().into();
    let val = bytes_to_u64(&enc[..len]);
    enc.advance(len);
    val
}
// https://github.com/akula-bft/akula/blob/a9aed09b31bb41c89832149bcad7248f7fcd70ca/src/models/account.rs#L47
pub fn bytes_to_u64(buf: &[u8]) -> u64 {
    let mut decoded = [0u8; 8];
    for (i, b) in buf.iter().rev().enumerate() {
        decoded[i] = *b;
    }
    u64::from_le_bytes(decoded)
}

impl Account {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn nonce(mut self, nonce: u64) -> Self {
        self.nonce = nonce;
        self
    }
    pub fn incarnation(mut self, inc: u64) -> Self {
        self.incarnation = inc;
        self
    }
    pub fn balance(mut self, bal: U256) -> Self {
        self.balance = bal;
        self
    }
    pub fn codehash(mut self, hash: H256) -> Self {
        self.codehash = hash;
        self
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct StorageKey {
    pub address: Address,
    pub incarnation: u64,
}
impl StorageKey {
    pub fn new(address: Address, incarnation: u64) -> Self {
        Self {
            address,
            incarnation,
        }
    }
}

impl TableEncode for StorageKey {
    type Encoded = [u8; ADDRESS_LENGTH + U64_LENGTH];

    fn encode(self) -> Self::Encoded {
        let mut out = [0; ADDRESS_LENGTH + U64_LENGTH];
        out[..ADDRESS_LENGTH].copy_from_slice(&self.address.encode());
        out[ADDRESS_LENGTH..].copy_from_slice(&self.incarnation.encode());
        out
    }
}
//TODO: dummy impl as we only need to encode for now, but need the trait bound
impl TableDecode for StorageKey {
    fn decode(_enc: &[u8]) -> Result<Self> {
        Ok(Default::default())
    }
}
