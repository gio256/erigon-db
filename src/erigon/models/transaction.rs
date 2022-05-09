use bytes::{Buf, Bytes, BytesMut};
use ethereum_types::{Address, H256, U256};
use fastrlp::{BufMut, Decodable, DecodeError, Encodable, RlpDecodable, RlpEncodable};
use serde::{Deserialize, Serialize};
use std::borrow::Cow;

use crate::erigon::{
    macros::u256_wrapper,
    utils::{consts as C, keccak256},
};

// https://github.com/akula-bft/akula/blob/e5af0ab9cea24c7ff4713b1e61c60a918abc6fef/src/models/transaction.rs#L41
/// The `to` address in an rlp-encoded transaction is either the 1-byte encoded length
/// of the string (0x80 + 0x14 bytes = 0x94), or it is the 1-byte encoded length of
/// the empty string (0x80) if the transaction is creating a contract. TxAction
/// is used to implement this encoding/decoding scheme.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TxAction {
    Call(Address),
    Create,
}
impl From<TxAction> for Option<Address> {
    fn from(src: TxAction) -> Self {
        match src {
            TxAction::Call(adr) => Some(adr),
            TxAction::Create => None,
        }
    }
}
impl From<Option<Address>> for TxAction {
    fn from(src: Option<Address>) -> Self {
        match src {
            Some(adr) => Self::Call(adr),
            None => Self::Create,
        }
    }
}

impl Encodable for TxAction {
    fn length(&self) -> usize {
        match self {
            Self::Call(_) => 1 + C::ADDRESS_LENGTH,
            Self::Create => 1,
        }
    }

    fn encode(&self, out: &mut dyn BufMut) {
        match self {
            Self::Call(adr) => {
                fastrlp::Header {
                    list: false,
                    payload_length: C::ADDRESS_LENGTH,
                }
                .encode(out);
                out.put_slice(adr.as_bytes());
            }
            Self::Create => {
                out.put_u8(fastrlp::EMPTY_STRING_CODE);
            }
        }
    }
}

impl Decodable for TxAction {
    fn decode(buf: &mut &[u8]) -> Result<Self, DecodeError> {
        if buf.is_empty() {
            return Err(DecodeError::InputTooShort);
        }
        const RLP_ADDRESS_CODE: u8 = fastrlp::EMPTY_STRING_CODE + C::ADDRESS_LENGTH as u8;

        Ok(match buf.get_u8() {
            fastrlp::EMPTY_STRING_CODE => Self::Create,
            RLP_ADDRESS_CODE => {
                let slice = buf
                    .get(..C::ADDRESS_LENGTH)
                    .ok_or(DecodeError::InputTooShort)?;
                buf.advance(C::ADDRESS_LENGTH);
                Self::Call(Address::from_slice(slice))
            }
            _ => return Err(DecodeError::UnexpectedLength),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, RlpDecodable, RlpEncodable)]
pub struct AccessTuple {
    pub address: Address,
    pub slots: Vec<H256>,
}
pub type AccessList = Vec<AccessTuple>;

// For legacy transactions, v is packed with the Eip155 chain id
u256_wrapper!(VPackChainId);

impl VPackChainId {
    // Eip155 defines v as either {0,1} + 27 (no chain id) OR {0,1} + chain_id * 2 + 35
    pub fn derive_chain_id(&self) -> Option<U256> {
        if self.0 == U256::from(27) || self.0 == U256::from(28) {
            None
        } else {
            Some(
                (self
                    .0
                    .checked_sub(35.into())
                    .expect("invalid eip155 chainid"))
                    / 2,
            )
        }
    }
    //TODO
    pub fn derive_v(&self) -> U256 {
        if let Some(chain_id) = self.derive_chain_id() {
            self.0 - (chain_id * 2 + 35) + 27
        } else {
            self.0
        }
    }
}

// rlp([nonce, gas_price, gas_limit, to, value, data, v, r, s])
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, RlpDecodable, RlpEncodable)]
pub struct LegacyTx {
    pub nonce: u64,
    pub gas_price: U256,
    pub gas: u64,
    pub to: TxAction,
    pub value: U256,
    pub data: Bytes,
    pub v: VPackChainId,
    pub r: U256,
    pub s: U256,
}

// Eip2930 transaction
// 0x01 || rlp([chain_id, nonce, gas_price, gas_limit, to, value, data, access_list, sig_y_parity, sig_r, sig_s])
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, RlpDecodable, RlpEncodable)]
pub struct AccessListTx {
    pub chain_id: U256,
    pub nonce: u64,
    pub gas_price: U256,
    pub gas: u64,
    pub to: TxAction,
    pub value: U256,
    pub data: Bytes,
    pub access_list: AccessList,
    pub v: U256,
    pub r: U256,
    pub s: U256,
}

// Eip1559 transaction
// 0x02 || rlp([chain_id, nonce, max_priority_fee_per_gas, max_fee_per_gas, gas_limit, destination, amount, data, access_list, sig_y_parity, sig_r, sig_s])
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, RlpDecodable, RlpEncodable)]
pub struct DynamicFeeTx {
    pub chain_id: U256,
    pub nonce: u64,
    pub tip: U256,
    pub fee_cap: U256,
    pub gas: u64,
    pub to: TxAction,
    pub value: U256,
    pub data: Bytes,
    pub access_list: AccessList,
    pub v: U256,
    pub r: U256,
    pub s: U256,
}

crate::erigon::macros::rlp_table_value!(Transaction);
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Transaction {
    Legacy(LegacyTx),
    AccessList(AccessListTx),
    DynamicFee(DynamicFeeTx),
}

impl DynamicFeeTx {
    pub const TYPE: u8 = 0x02;
}
impl AccessListTx {
    pub const TYPE: u8 = 0x01;
}

impl Decodable for Transaction {
    fn decode(buf: &mut &[u8]) -> Result<Self, fastrlp::DecodeError> {
        // if input is rlp encoded as a list, interpret as a legacy transaction
        // rlp([nonce, gas_price, gas_limit, to, value, data, v, r, s])
        if buf[0] >= 0xc0 {
            return Decodable::decode(buf).map(Self::Legacy);
        }
        // strip string length and length of length
        fastrlp::Header::decode(buf)?;

        // Eip2718 Typed Transaction. TransactionType || TransactionPayload
        match buf.get_u8() {
            AccessListTx::TYPE => Decodable::decode(buf).map(Self::AccessList),
            DynamicFeeTx::TYPE => Decodable::decode(buf).map(Self::DynamicFee),
            _ => Err(DecodeError::Custom("Unknown transaction type")),
        }
    }
}

impl Encodable for Transaction {
    fn encode(&self, out: &mut dyn BufMut) {
        match self {
            Self::Legacy(tx) => tx.encode(out),
            Self::AccessList(tx) => tx.encode(out),
            Self::DynamicFee(tx) => tx.encode(out),
        }
    }
}

impl Transaction {
    pub fn tx_type(&self) -> Option<u8> {
        match self {
            Self::AccessList(_) => Some(AccessListTx::TYPE),
            Self::DynamicFee(_) => Some(DynamicFeeTx::TYPE),
            Self::Legacy(_) => None,
        }
    }
    pub fn hash(&self) -> H256 {
        match self {
            Self::Legacy(tx) => tx.hash(),
            Self::AccessList(tx) => tx.hash(),
            Self::DynamicFee(tx) => tx.hash(),
        }
    }
    pub fn nonce(&self) -> u64 {
        match self {
            Self::Legacy(tx) => tx.nonce,
            Self::AccessList(tx) => tx.nonce,
            Self::DynamicFee(tx) => tx.nonce,
        }
    }
    pub fn to(&self) -> TxAction {
        match self {
            Self::Legacy(tx) => tx.to,
            Self::AccessList(tx) => tx.to,
            Self::DynamicFee(tx) => tx.to,
        }
    }
    pub fn value(&self) -> U256 {
        match self {
            Self::Legacy(tx) => tx.value,
            Self::AccessList(tx) => tx.value,
            Self::DynamicFee(tx) => tx.value,
        }
    }
    pub fn gas_price(&self) -> Option<U256> {
        match self {
            Self::Legacy(tx) => Some(tx.gas_price),
            Self::AccessList(tx) => Some(tx.gas_price),
            Self::DynamicFee(_) => None,
        }
    }
    pub fn chain_id(&self) -> Option<U256> {
        match self {
            Self::Legacy(tx) => tx.v.derive_chain_id(),
            Self::AccessList(tx) => Some(tx.chain_id),
            Self::DynamicFee(tx) => Some(tx.chain_id),
        }
    }
    pub fn tip(&self) -> Option<U256> {
        match self {
            Self::DynamicFee(tx) => Some(tx.tip),
            _ => None,
        }
    }
    pub fn fee_cap(&self) -> Option<U256> {
        match self {
            Self::DynamicFee(tx) => Some(tx.fee_cap),
            _ => None,
        }
    }
    pub fn gas(&self) -> u64 {
        match self {
            Self::Legacy(tx) => tx.gas,
            Self::AccessList(tx) => tx.gas,
            Self::DynamicFee(tx) => tx.gas,
        }
    }
    pub fn data(&self) -> &Bytes {
        match self {
            Self::Legacy(tx) => &tx.data,
            Self::AccessList(tx) => &tx.data,
            Self::DynamicFee(tx) => &tx.data,
        }
    }
    pub fn r(&self) -> U256 {
        match self {
            Self::Legacy(tx) => tx.r,
            Self::AccessList(tx) => tx.r,
            Self::DynamicFee(tx) => tx.r,
        }
    }
    pub fn s(&self) -> U256 {
        match self {
            Self::Legacy(tx) => tx.s,
            Self::AccessList(tx) => tx.s,
            Self::DynamicFee(tx) => tx.s,
        }
    }
    //TODO
    pub fn v(&self) -> U256 {
        match self {
            Self::Legacy(tx) => tx.v.derive_v(),
            Self::AccessList(tx) => tx.v,
            Self::DynamicFee(tx) => tx.v,
        }
    }

    pub fn access_list(&self) -> Option<Cow<'_, AccessList>> {
        match self {
            Self::AccessList(tx) => Some(Cow::Borrowed(&tx.access_list)),
            Self::DynamicFee(tx) => Some(Cow::Borrowed(&tx.access_list)),
            Self::Legacy(_) => None,
        }
    }
}

impl LegacyTx {
    /// Computes the (signing) hash of the transaction
    pub fn hash(&self) -> H256 {
        #[derive(RlpEncodable)]
        struct AsHash<'a> {
            nonce: u64,
            gas_price: &'a U256,
            gas: u64,
            to: &'a TxAction,
            value: &'a U256,
            data: &'a Bytes,
        }

        #[derive(RlpEncodable)]
        struct AsHashWithChainId<'a> {
            nonce: u64,
            gas_price: &'a U256,
            gas: u64,
            to: &'a TxAction,
            value: &'a U256,
            data: &'a Bytes,
            chain_id: U256,
            _pad1: u8,
            _pad2: u8,
        }

        let mut buf = BytesMut::new();
        if let Some(chain_id) = self.v.derive_chain_id() {
            AsHashWithChainId {
                nonce: self.nonce,
                gas_price: &self.gas_price,
                gas: self.gas,
                to: &self.to,
                value: &self.value,
                data: &self.data,
                chain_id,
                _pad1: 0,
                _pad2: 0,
            }
            .encode(&mut buf);
        } else {
            AsHash {
                nonce: self.nonce,
                gas_price: &self.gas_price,
                gas: self.gas,
                to: &self.to,
                value: &self.value,
                data: &self.data,
            }
            .encode(&mut buf);
        }
        keccak256(buf).into()
    }
}

impl AccessListTx {
    /// Computes the (signing) hash of the transaction
    pub fn hash(&self) -> H256 {
        #[derive(RlpEncodable)]
        struct AsHash<'a> {
            chain_id: U256,
            nonce: u64,
            gas_price: &'a U256,
            gas: u64,
            to: &'a TxAction,
            value: &'a U256,
            data: &'a Bytes,
            access_list: &'a AccessList,
        }

        let mut buf = BytesMut::new();
        buf.put_u8(Self::TYPE);

        AsHash {
            chain_id: self.chain_id,
            nonce: self.nonce,
            gas_price: &self.gas_price,
            gas: self.gas,
            to: &self.to,
            value: &self.value,
            data: &self.data,
            access_list: &self.access_list,
        }
        .encode(&mut buf);

        keccak256(buf).into()
    }
}

impl DynamicFeeTx {
    /// Computes the (signing) hash of the transaction
    pub fn hash(&self) -> H256 {
        #[derive(RlpEncodable)]
        struct AsHash<'a> {
            chain_id: U256,
            nonce: u64,
            tip: &'a U256,
            fee_cap: &'a U256,
            gas: u64,
            to: &'a TxAction,
            value: &'a U256,
            data: &'a Bytes,
            access_list: &'a AccessList,
        }

        let mut buf = BytesMut::new();
        buf.put_u8(Self::TYPE);

        AsHash {
            chain_id: self.chain_id,
            nonce: self.nonce,
            tip: &self.tip,
            fee_cap: &self.fee_cap,
            gas: self.gas,
            to: &self.to,
            value: &self.value,
            data: &self.data,
            access_list: &self.access_list,
        }
        .encode(&mut buf);

        keccak256(buf).into()
    }
}

pub struct TransactionWithSigner {
    pub msg: Transaction,
    pub signer: Address,
}

#[cfg(feature = "ethers-types")]
impl From<TransactionWithSigner> for ethers::types::Transaction {
    fn from(tx: TransactionWithSigner) -> Self {
        Self {
            hash: tx.msg.hash(),
            nonce: tx.msg.nonce().into(),
            from: tx.signer,
            to: tx.msg.to().into(),
            value: tx.msg.value(),
            gas_price: tx.msg.gas_price(),
            gas: tx.msg.gas().into(),
            input: tx.msg.data().clone().into(),
            transaction_type: tx.msg.tx_type().map(From::from),
            access_list: tx.msg.access_list().map(|al| {
                al.iter()
                    .map(|it| it.clone().into())
                    .collect::<Vec<_>>()
                    .into()
            }),
            chain_id: tx.msg.chain_id(),
            max_fee_per_gas: tx.msg.fee_cap(),
            max_priority_fee_per_gas: tx.msg.tip(),
            v: tx.msg.v().as_u64().into(),
            r: tx.msg.r(),
            s: tx.msg.s(),
            ..Default::default()
        }
    }
}

#[cfg(feature = "ethers-types")]
impl From<AccessTuple> for ethers::types::transaction::eip2930::AccessListItem {
    fn from(src: AccessTuple) -> Self {
        Self {
            address: src.address,
            storage_keys: src.slots,
        }
    }
}
