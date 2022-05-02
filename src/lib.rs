#![doc = include_str!("../README.md")]
#![allow(unused_imports)]
#![allow(unused)]
pub mod erigon;
pub mod kv;
pub use erigon::*;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        erigon::{models::*, Erigon},
        kv::{EnvFlags, MdbxEnv},
    };
    use ethereum_types::*;
    use fastrlp::*;
    use std::path::Path;

    #[test]
    fn test() -> eyre::Result<()> {
        let path = Path::new(env!("ERIGON_CHAINDATA"));
        let env = Erigon::open_ro(path)?;
        let db = Erigon::begin(&env)?;

        let dst: Address = "0xa94f5374Fce5edBC8E2a8697C15331677e6EbF0B"
            .parse()
            .unwrap();
        let contract: Address = "0x0d4c6c6605a729a379216c93e919711a081beba2"
            .parse()
            .unwrap();
        let res = db.read_account_hist(contract, 3)?;

        let slot = H256::from_low_u64_be(1);
        let res = db.read_storage_hist(contract, 1, slot, 3)?;
        let current = db.read_storage(contract, 1, slot)?;

        let hash = db.read_head_block_hash()?.unwrap();
        let num = db.read_header_number(hash)?.unwrap();
        Ok(())
    }
}
