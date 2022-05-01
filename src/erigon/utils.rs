use roaring::RoaringTreemap;

pub fn find_gte(map: RoaringTreemap, n: u64) -> u64 {
    // rank() returns the number of integers in the map <= n, i.e. the index
    // of n if it were in the bitmap.
    let rank = map.rank(n.saturating_sub(1));
    map.select(rank).unwrap()
}
