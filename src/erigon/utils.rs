use bytes::Buf;
use roaring::RoaringTreemap;
use tiny_keccak::{Hasher, Keccak};

// https://github.com/ledgerwatch/erigon/blob/f9d7cb5ca9e8a135a76ddcb6fa4ee526ea383554/ethdb/bitmapdb/dbutils.go#L313
pub fn find_gte(map: RoaringTreemap, n: u64) -> Option<u64> {
    // rank() returns the number of integers in the map <= n, i.e. the index
    // of n if it were in the bitmap.
    let rank = map.rank(n.saturating_sub(1));
    map.select(rank)
}

// From ethers: https://github.com/gakonst/ethers-rs/blob/master/ethers-core/src/utils/hash.rs#L26
pub fn keccak256<S>(bytes: S) -> [u8; 32]
where
    S: AsRef<[u8]>,
{
    let mut output = [0u8; 32];
    let mut hasher = Keccak::v256();
    hasher.update(bytes.as_ref());
    hasher.finalize(&mut output);
    output
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
