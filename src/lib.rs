#![allow(unused_imports)]
#![allow(unused)]
pub mod erigon;
pub mod kv;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{erigon::Erigon, kv::MdbxEnv};
    use ethereum_types::*;
    use fastrlp::*;
    use std::path::Path;
    #[test]
    fn test() -> eyre::Result<()> {
        let path = Path::new(env!("ERIGON_CHAINDATA"));
        let env = MdbxEnv::open_ro(path, 20, Default::default())?;
        let tx = env.begin_ro()?;
        let reader = Erigon(tx);
        let hash = reader.read_head_block_hash()?;
        dbg!(hash);
        let num = reader.read_header_number(hash)?;
        dbg!(num);
        let rlp = reader.read_header(num, hash)?;
        dbg!(rlp);
        // let header: erigon::models::BlockHeader = Decodable::decode(&mut &**rlp)?;
        // let header = dbg!(header);
        // let mut buf = vec![];
        // header.encode(&mut buf);
        // assert_eq!(erigon::models::Rlp(buf.into()), rlp);
        Ok(())
    }
}
