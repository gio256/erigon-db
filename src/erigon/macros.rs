macro_rules! declare_tuple {
    ($name:ident($($t:ty),+)) => {
        #[derive(
            Clone,
            Copy,
            Debug,
            PartialEq,
            Eq,
            Default,
            ::derive_more::From,
            ::serde::Serialize,
            ::serde::Deserialize,
            ::fastrlp::RlpEncodable,
            ::fastrlp::RlpDecodable,
        )]
        pub struct $name($(pub $t),+);
    }
}
pub(crate) use declare_tuple;

macro_rules! size_tuple_aux {
    ($t0:ty) => {
        pub const MIN_SIZE: usize = 0;
        pub const SIZE_T0: usize = std::mem::size_of::<$t0>();
    };
    ($t0:ty, $t1:ty) => {
        pub const MIN_SIZE: usize = Self::SIZE_T0;
        pub const SIZE_T0: usize = std::mem::size_of::<$t0>();
        pub const SIZE_T1: usize = std::mem::size_of::<$t1>();
    };
    ($t0:ty, $t1:ty, $t2:ty) => {
        pub const MIN_SIZE: usize = Self::SIZE_T0 + Self::SIZE_T1;
        pub const SIZE_T0: usize = std::mem::size_of::<$t0>();
        pub const SIZE_T1: usize = std::mem::size_of::<$t1>();
        pub const SIZE_T2: usize = std::mem::size_of::<$t2>();
    };
    ($t0:ty, $t1:ty, $t2:ty, $t3:ty) => {
        pub const MIN_SIZE: usize = Self::SIZE_T0 + Self::SIZE_T1 + Self::SIZE_T2;
        pub const SIZE_T0: usize = std::mem::size_of::<$t0>();
        pub const SIZE_T1: usize = std::mem::size_of::<$t1>();
        pub const SIZE_T2: usize = std::mem::size_of::<$t2>();
        pub const SIZE_T3: usize = std::mem::size_of::<$t3>();
    };
}
pub(crate) use size_tuple_aux;

macro_rules! size_tuple {
    ($name:ident($($t:ty),+)) => {
        impl $name {
            pub const SIZE: usize = 0 $(+ std::mem::size_of::<$t>())+;
            $crate::erigon::macros::size_tuple_aux!($($t),+);
        }
    }
}
pub(crate) use size_tuple;

macro_rules! impl_encode_tuple {
    ($name:ident($($t:ty),+), $n:literal) => {
        impl $crate::kv::traits::TableEncode for $name {
            type Encoded = $crate::kv::tables::VariableVec<{ Self::SIZE }>;
            fn encode(self) -> Self::Encoded {
                let mut out = Self::Encoded::default();
                ::seq_macro::seq! { N in 0..=$n {
                    out.try_extend_from_slice(&$crate::kv::traits::TableEncode::encode(self.N)).unwrap();
                }}
                out
            }
        }
    }
}
pub(crate) use impl_encode_tuple;

macro_rules! impl_decode_tuple {
    ($name:ident($($t:ty),+), $n:literal) => {
        impl $crate::kv::traits::TableDecode for $name {
            fn decode(b: &[u8]) -> ::eyre::Result<Self> {
                if b.len() > Self::SIZE {
                    return Err(
                        $crate::kv::tables::TooLong::<{ Self::SIZE }> { got: b.len() }.into(),
                    );
                }
                if b.len() < Self::MIN_SIZE {
                    return Err($crate::kv::tables::TooShort::<{ Self::MIN_SIZE }> {
                        got: b.len(),
                    }
                    .into());
                }
                let remainder = b;
                ::seq_macro::seq! { N in 0..$n {
                    #( let (b~N, remainder) = remainder.split_at(Self::SIZE_T~N); )*

                    Ok(Self(
                        #( $crate::kv::traits::TableDecode::decode(b~N)?,)*
                        $crate::kv::traits::TableDecode::decode(remainder)?,
                    ))
                }}
            }
        }
    };
}
pub(crate) use impl_decode_tuple;

macro_rules! make_tuple_key {
    ($name:ident($($t:ty),+), $n:literal) => {
        $crate::erigon::macros::declare_tuple!($name($($t),+));
        $crate::erigon::macros::size_tuple!($name($($t),+));
        $crate::erigon::macros::impl_encode_tuple!($name($($t),+), $n);
        $crate::erigon::macros::impl_decode_tuple!($name($($t),+), $n);
    }
}
pub(crate) use make_tuple_key;

macro_rules! tuple_key {
    ($name:ident($t0:ty)) => {
        $crate::erigon::macros::make_tuple_key!($name($t0), 0);
    };
    ($name:ident($t0:ty, $t1:ty)) => {
        $crate::erigon::macros::make_tuple_key!($name($t0, $t1), 1);
    };
    ($name:ident($t0:ty, $t1:ty, $t2:ty)) => {
        $crate::erigon::macros::make_tuple_key!($name($t0, $t1, $t2), 2);
    };
}
pub(crate) use tuple_key;

macro_rules! bytes_wrapper {
    ($name:ident($t:ty)) => {
        #[derive(
            Clone,
            Debug,
            PartialEq,
            Eq,
            Default,
            ::derive_more::Deref,
            ::derive_more::DerefMut,
            ::serde::Serialize,
            ::serde::Deserialize,
            ::fastrlp::RlpEncodable,
            ::fastrlp::RlpDecodable,
        )]
        pub struct $name(pub $t);

        impl $crate::kv::traits::TableEncode for $name {
            type Encoded = bytes::Bytes;
            fn encode(self) -> Self::Encoded {
                self.0
            }
        }

        impl $crate::kv::traits::TableDecode for $name {
            fn decode(b: &[u8]) -> Result<Self> {
                $crate::kv::traits::TableDecode::decode(b).map(Self)
            }
        }
    };
}
pub(crate) use bytes_wrapper;

macro_rules! constant_key {
    ($name:ident, $encoded:ident) => {
        impl $crate::kv::traits::TableEncode for $name {
            type Encoded = Vec<u8>;
            fn encode(self) -> Self::Encoded {
                String::from(stringify!($encoded)).into_bytes()
            }
        }
    };
    ($name:ident) => {
        $crate::erigon::macros::constant_key!($name, $name);
    };
}
pub(crate) use constant_key;
