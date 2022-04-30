#![allow(unused_imports)]
#![allow(unused)]
pub mod erigon;
pub mod kv;

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

    // https://github.com/ledgerwatch/erigon-lib/blob/625c9f5385d209dc2abfadedf6e4b3914a26ed3e/kv/mdbx/kv_mdbx.go#L154
    const ENV_FLAGS: EnvFlags = EnvFlags {
        // Disable readahead. Improves performance when db size > RAM.
        no_rdahead: true,
        // Try to coalesce while garbage collecting. (https://en.wikipedia.org/wiki/Coalescing_(computer_science))
        coalesce: true,
        // If another process is using the db with different flags, open in
        // compatibility mode instead of MDBX_INCOMPATIBLE error.
        accede: true,
        no_sub_dir: false,
        exclusive: false,
        no_meminit: false,
        liforeclaim: false,
    };

    #[test]
    fn test() -> eyre::Result<()> {
        let path = Path::new(env!("ERIGON_CHAINDATA"));
        let env = MdbxEnv::open_ro(path, 20, ENV_FLAGS)?;
        let tx = env.begin_ro()?;
        let reader = Erigon(tx);
        let hash = reader.read_head_block_hash()?;
        dbg!(hash);
        let num = reader.read_header_number(hash)?;
        dbg!(num);
        let key = HeaderKey(num, hash);
        let header = reader.read_header(key)?;
        dbg!(header);
        let body = reader.read_body_for_storage(key)?;
        dbg!(body);
        // let header: erigon::models::BlockHeader = Decodable::decode(&mut &**rlp)?;
        // let header = dbg!(header);
        // let mut buf = vec![];
        // header.encode(&mut buf);
        // assert_eq!(erigon::models::Rlp(buf.into()), rlp);
        // let foo = dbg!(PlainPrefix::SIZE);
        // let who: Address = "0x0d4c6c6605a729a379216c93e919711a081beba2".parse()?;
        // let pre = PlainPrefix(who, u64::MAX);
        // dbg!(pre.clone());
        // let enc = kv::traits::TableEncode::encode(pre.clone());
        // dbg!(enc.clone());
        // let dec: PlainPrefix = kv::traits::TableDecode::decode(&enc)?;
        // dbg!(dec);
        Ok(())
    }
}
