Much of the code has been taken from the [Akula](https://github.com/akula-bft/akula) Ethereum client in order to enable its use with the stable rust toolchain. In particular, it repurposes many of Akula's [`kv`](https://github.com/akula-bft/akula/blob/master/src/kv/mod.rs) utilities and abstractions for working with `libmdbx` and Ethereum data.

```rust
use erigon_db::Erigon;
use ethereum_types::Address;

fn main() {
    let path = std::path::Path::new(env!("ERIGON_CHAINDATA"));
    let env = Erigon::open_ro(path).unwrap();
    let db = Erigon::begin(&env).unwrap();

    let head_hash = db.read_head_block_hash().unwrap().unwrap();
    let head_num = db.read_header_number(head_hash).unwrap();

    let contract: Address = "0x0d4c6c6605a729a379216c93e919711a081beba2"
        .parse()
        .unwrap();

    let account = db.read_account_data(contract).unwrap().unwrap();

    for read in db.walk_storage(contract, account.incarnation).unwrap() {
        let (slot, value) = read.unwrap();
        println!("The value at slot {} is {}", slot, value);
    }
}
```
