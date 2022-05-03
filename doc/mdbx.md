# MDBX primer
An mdbx interaction can be thought of in three levels of abstraction: one environment, one or more transactions, then one or more databases and cursors.

#### Environment
Opening an environment opens or creates the storage file, manages file locks, and initializes the specified configuration. You should be careful not to open the same mdbx environment more than once from the same process. If your process needs to access the database from multiple threads, you must share the same environment between them or mdbx will return `MDBX_BUSY`.

#### Transactions
From an environment, you create a transaction. Note that a transaction is needed even for read-only access in order to ensure a consistent view of the data. A read-write transaction must be committed to flush changes to the db, but Drop impls take care of this for us in rust.

The rust bindings this crate uses to interact with mdbx set [`MDBX_NOTLS`](https://github.com/vorot93/libmdbx-rs/blob/b69d3d988ad7afaa4070c83480b0b48572f93929/src/flags.rs#L158) by default, which prevents issues with opening multiple transactions across process-managed threads or passing read-only transactions across OS threads. However, you should in general avoid opening overlapping transactions on the same thread or sending a read-write transaction across threads. Also be wary of long-running read transactions, as they can create database bloat and impact performance.

#### Databases / Tables
From a transaction, you can create or open one or more named databases. These databases are referred to in this crate as tables, and they represent a logical separation of different key-value spaces within the environment. Once opened, a database handle, or dbi, can be shared across transactions and cursors and need not ever be closed. This crate makes an effort to enable related optimizations in a type-safe way by associating each [`TableHandle`] with an implementer of the [`DbName`] and [`DbFlags`] traits. A `TableHandle` can be shared across transactions, but a [`Table`] can never be accessed without a matching `TableHandle`.

[`TableHandle`]: `crate::kv::tables::TableHandle`
[`DbName`]: `crate::kv::traits::DbName`
[`DbFlags`]: `crate::kv::traits::DbFlags`
[`Table`]: `crate::kv::traits::Table`

#### Cursors
A transaction can be used to `get()` or `put()` individual key-value pairs.
For more complex interactions, you can also generate a cursor from a transaction.
A cursor is a stateful accessor that can be positioned, repositioned, and used to efficiently traverse key-value pairs in a table.

For an example of why cursors are useful, it is important to know that mdbx entries are sorted by their keys using a byte by byte [lexicographic order](https://cplusplus.com/reference/algorithm/lexicographical_compare/) (haskell [translation](https://en.wikipedia.org/wiki/Lexicographic_order#Monoid_of_words)).
This is the reason for the `SeekKey` associated type for [`Table`].
If the `Key` for the table is the concatenation of the block number and the block hash, then seeking by block number `N` will position the cursor at the first value in the table with a key that begins with `N`.
You can then use the cursor to yield successive key-value pairs until you get a key that starts with a different block number.

[`Table`]: `crate::kv::traits::Table`


## Dupsort
