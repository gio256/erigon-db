Much of the code has been taken from the [Akula](https://github.com/akula-bft/akula) Ethereum client in order to enable its use with the stable rust toolchain. In particular, it repurposes many of Akula's [`kv`](https://github.com/akula-bft/akula/blob/master/src/kv/mod.rs) utilities and abstractions for working with `libmdbx` and Ethereum data.

```rust
# use erigon_db::Erigon;
# use eyre::Result;
# use std::path::Path;
# fn main() -> Result<()> {
let path = Path::new(env!("ERIGON_CHAINDATA"));
let env = Erigon::open_ro(path)?;
let db = Erigon::begin(&env)?;

let head_hash = db.read_head_block_hash()?.unwrap();
let head_num = db.read_header_number(head_hash)?;
# Ok(())
# }
```
