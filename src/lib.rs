#![doc = include_str!("../README.md")]
#![doc = include_str!("../doc/mdbx.md")]
pub mod erigon;
pub mod kv;
pub use erigon::*;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        erigon::{Erigon},
        kv::{MdbxEnv},
    };
    use ethereum_types::*;
    use once_cell::sync::Lazy;
    use std::{path::Path, sync::Arc};

    struct TempMdbxEnv<M> {
        pub inner: MdbxEnv<M>,
        _leak: tempfile::TempDir,
    }

    static ENV: Lazy<Arc<TempMdbxEnv<mdbx::RW>>> = Lazy::new(|| {
        let dir = tempfile::tempdir().unwrap();
        let inner = erigon::env_open(dir.path()).expect("failed to open mem db");
        Arc::new(TempMdbxEnv { inner, _leak: dir })
    });

    #[test]
    fn test_mem_db() -> eyre::Result<()> {
        let env = ENV.clone();
        let db = Erigon::begin_rw(&env.inner)?;
        let hash = H256::from_low_u64_be(u64::MAX);
        db.write_head_header_hash(hash)?;
        let res = db.read_head_header_hash()?.unwrap();
        assert_eq!(res, hash);
        Ok(())
    }

    #[test]
    fn test_live() -> eyre::Result<()> {
        let path = Path::new(env!("ERIGON_CHAINDATA"));
        let env = env_open(path)?;
        let db = Erigon::begin(&env)?;

        let _dst: Address = "0xa94f5374Fce5edBC8E2a8697C15331677e6EbF0B".parse()?;
        let contract: Address = "0x0d4c6c6605a729a379216c93e919711a081beba2".parse()?;
        let _res = db.read_account_hist(contract, 3)?;

        let slot = H256::from_low_u64_be(1);
        let res = db.read_storage_hist(contract, 1, slot, 0)?;
        let current = db.read_storage(contract, 2, slot)?;
        dbg!(res);
        dbg!(current);
        for read in db.walk_storage(contract, 1)? {
            let (key, val) = read?;
            dbg!(key, val);
        }

        let hash = db.read_head_header_hash()?.unwrap();
        let num = db.read_header_number(hash)?.unwrap();

        let td = db.read_total_difficulty((num, hash))?.unwrap();
        dbg!(td);

        // let burnt = db.read::<crate::tables::Burnt>(1.into())?.unwrap();
        // dbg!(burnt);
        Ok(())
    }
}
