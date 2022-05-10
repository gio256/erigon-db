use bytes::Buf;
use ethereum_types::{H256, U256};
use eyre::Result;
use fastrlp::{RlpDecodable, RlpEncodable};
use serde::{Deserialize, Serialize};

use crate::{
    erigon::{
        utils::{consts::*, *},
        Incarnation,
    },
    kv::traits::{TableDecode, TableEncode},
};

#[derive(
    Clone, Copy, Debug, PartialEq, Eq, Default, Deserialize, Serialize, RlpEncodable, RlpDecodable,
)]
pub struct Account {
    pub nonce: u64,
    pub incarnation: Incarnation,
    pub balance: U256,
    pub codehash: H256, // hash of the bytecode
}

impl TableDecode for Account {
    fn decode(mut buf: &[u8]) -> Result<Self> {
        let mut acct = Self::default();

        if buf.is_empty() {
            return Ok(acct);
        }

        let fieldset = buf.get_u8();

        // has nonce
        if fieldset & 1 > 0 {
            acct.nonce = take_u64_rlp(&mut buf)?;
        }

        // has balance
        if fieldset & 2 > 0 {
            let bal_len = buf.get_u8();
            acct.balance = buf[..bal_len.into()].into();
            buf.advance(bal_len.into());
        }

        // has incarnation
        if fieldset & 4 > 0 {
            acct.incarnation = take_u64_rlp(&mut buf)?.into();
        }

        // has codehash
        if fieldset & 8 > 0 {
            let len: usize = buf.get_u8().into();
            if len != KECCAK_LENGTH {
                eyre::bail!(
                    "codehash should be {} bytes long. Got {} instead",
                    KECCAK_LENGTH,
                    len
                );
            }
            acct.codehash = H256::from_slice(&buf[..KECCAK_LENGTH]);
            buf.advance(KECCAK_LENGTH)
        }
        Ok(acct)
    }
}
//TODO: dummy impl as we only need to decode for now, but need the trait bound
impl TableEncode for Account {
    type Encoded = Vec<u8>;
    fn encode(self) -> Self::Encoded {
        unreachable!("Can't encode Account")
    }
}

impl Account {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn nonce(mut self, nonce: u64) -> Self {
        self.nonce = nonce;
        self
    }
    pub fn incarnation(mut self, inc: Incarnation) -> Self {
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
