### Overview
Fully typed access to the [Erigon](https://github.com/ledgerwatch/erigon) database in rust.

```rust
use erigon_db::{Erigon, env_open};
use ethereum_types::Address;

fn main() -> eyre::Result<()> {
    let path = std::path::Path::new(env!("ERIGON_CHAINDATA"));

    // Open an mdbx environment and begin a read-only database transaction
    let env = env_open(path)?;
    let db = Erigon::begin(&env)?;

    // get the canonical head block header
    let head_hash = db.read_head_header_hash()?.unwrap();
    let head_num = db.read_header_number(head_hash)?.unwrap();
    let header = db.read_header((head_num, head_hash))?.unwrap();

    // get the current state of an account
    let contract: Address = "0x0d4c6c6605a729a379216c93e919711a081beba2".parse()?;
    let account = db.read_account(contract)?.unwrap();
    let bytecode = db.read_code(account.codehash)?.unwrap();

    // get all of the contract's populated storage slots
    // (incarnation is an Erigon-specific value related to removal/revival of contracts)
    for read in db.walk_storage(contract, account.incarnation)? {
        let (slot, value) = read?;
        println!("The value at slot {} is {}", slot, value);
    }

    // get the state of the account at block 100
    let old_account = db.read_account_hist(contract, 100)?.unwrap_or_default();

    Ok(())
}
```

# Acknowledgements
Much of this code has been taken from the [Akula](https://github.com/akula-bft/akula) Ethereum client in order to enable its use with the stable rust toolchain.
In particular, it repurposes many of Akula's [`kv`](https://github.com/akula-bft/akula/blob/master/src/kv/mod.rs) utilities and abstractions for working with `libmdbx` and Ethereum data.
These abstractions are extremely high-quality in my opinion, so the primary modifications were increasing the strictness and expressiveness of the accessor types and tailoring to Erigon's data representations and database layout.

## Resources
- Erigon has an excellent [doc](https://github.com/ledgerwatch/erigon/blob/devel/docs/programmers_guide/db_walkthrough.MD) walking through their database layout, though it may not match the current implementation in some places.
- Erigon's [`core/rawdb/accessors_*.go`](https://github.com/ledgerwatch/erigon/blob/f9d7cb5ca9e8a135a76ddcb6fa4ee526ea383554/core/rawdb/accessors_chain.go#L39) contains many of their low-level database interactions.
- Some [comments](https://github.com/ledgerwatch/erigon/blob/devel/docs/programmers_guide/db_faq.md) from Erigon on the advisability of this endeavor.
- For some brief info on mdbx, see [doc/mdbx.md](./doc/mdbx.md).
- The Erigon database layout is defined in [src/erigon/tables.rs](./src/erigon/tables.rs), and you can see how these tables are read in [src/erigon/mod.rs](./src/erigon/mod.rs).
