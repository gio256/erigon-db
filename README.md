Much of the code has been taken and modified from [Akula](https://github.com/akula-bft/akula) in order to enable its use with the stable rust toolchain.

```rust
# use erigon_db::Erigon;
# use eyre::Result;
# use std::path::Path;
# fn main() -> Result<()> {
let path = Path::new(env!("ERIGON_CHAINDATA"));
let env = Erigon::open_ro(path)?;
let db = Erigon::begin(&env)?;

let head_hash = db.read_head_block_hash()?;
let head_num = db.read_header_number(head_hash)?;
# Ok(())
# }
```
