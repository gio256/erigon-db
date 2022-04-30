use std::fmt::Debug;

pub trait TableEncode: Send + Sync + Sized {
    type Encoded: AsRef<[u8]> + Send + Sync;
    fn encode(self) -> Self::Encoded;
}

pub trait TableDecode: Send + Sync + Sized {
    fn decode(b: &[u8]) -> eyre::Result<Self>;
}

pub trait TableObject: TableEncode + TableDecode {}

impl<T> TableObject for T where T: TableEncode + TableDecode {}

pub trait DbName {
    fn db_name() -> Option<&'static str>;
}

pub trait Table<'tx>: Send + Sync + Debug + 'static {
    type Key: TableEncode;
    type Value: TableObject;
    type SeekKey: TableEncode;
    type Dbi: AsRef<mdbx::Database<'tx>>;
}

pub trait DupSort<'tx>: Table<'tx> {
    type SeekBothKey: TableObject;
}

pub trait Mode {
    fn is_writeable() -> bool;
}
impl<'env> Mode for mdbx::RO {
    fn is_writeable() -> bool {
        false
    }
}
impl<'env> Mode for mdbx::RW {
    fn is_writeable() -> bool {
        true
    }
}
