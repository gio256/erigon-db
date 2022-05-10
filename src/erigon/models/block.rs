use bytes::Bytes;
use ethereum_types::{Address, Bloom, H256, H64, U256};
use eyre::Result;
use fastrlp::{BufMut, Decodable, DecodeError, Encodable, RlpDecodable, RlpEncodable};
use serde::{Deserialize, Serialize};

use crate::erigon::{macros::*, utils::consts::*, Rlp};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, RlpEncodable, RlpDecodable)]
pub struct BodyForStorage {
    pub base_tx_id: u64,
    pub tx_amount: u32,
    pub uncles: Vec<BlockHeader>,
}
rlp_table_value!(BodyForStorage);

#[derive(Clone, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct BlockHeader {
    pub parent_hash: H256,
    pub uncle_hash: H256,
    pub coinbase: Address,
    pub root: H256,
    pub tx_hash: H256,
    pub receipts_hash: H256,
    pub bloom: Bloom,
    pub difficulty: U256,
    pub number: U256, // TODO: erigon stores as big.Int, then casts, which returns 0 if > u64 (technically big.Int says result is undefined)
    pub gas_limit: u64,
    pub gas_used: u64,
    pub time: u64,
    pub extra: Bytes,
    pub mix_digest: H256,
    pub nonce: H64,
    pub base_fee: Option<U256>,
    pub seal: Option<Rlp>,
}
rlp_table_value!(BlockHeader);

impl BlockHeader {
    fn rlp_header(&self) -> fastrlp::Header {
        let mut rlp_head = fastrlp::Header {
            list: true,
            payload_length: 0,
        };

        rlp_head.payload_length += KECCAK_LENGTH + 1; // parent_hash
        rlp_head.payload_length += KECCAK_LENGTH + 1; // ommers_hash
        rlp_head.payload_length += ADDRESS_LENGTH + 1; // beneficiary
        rlp_head.payload_length += KECCAK_LENGTH + 1; // state_root
        rlp_head.payload_length += KECCAK_LENGTH + 1; // transactions_root
        rlp_head.payload_length += KECCAK_LENGTH + 1; // receipts_root
        rlp_head.payload_length += BLOOM_BYTE_LENGTH + fastrlp::length_of_length(BLOOM_BYTE_LENGTH); // logs_bloom
        rlp_head.payload_length += self.difficulty.length(); // difficulty
        rlp_head.payload_length += self.number.length(); // block height
        rlp_head.payload_length += self.gas_limit.length(); // gas_limit
        rlp_head.payload_length += self.gas_used.length(); // gas_used
        rlp_head.payload_length += self.time.length(); // timestamp
        rlp_head.payload_length += self.extra.length(); // extra_data

        rlp_head.payload_length += KECCAK_LENGTH + 1; // mix_hash
        rlp_head.payload_length += 8 + 1; // nonce

        if let Some(base_fee) = self.base_fee {
            rlp_head.payload_length += base_fee.length();
        }

        rlp_head
    }
}

impl Encodable for BlockHeader {
    fn encode(&self, out: &mut dyn BufMut) {
        self.rlp_header().encode(out);
        Encodable::encode(&self.parent_hash, out);
        Encodable::encode(&self.uncle_hash, out);
        Encodable::encode(&self.coinbase, out);
        Encodable::encode(&self.root, out);
        Encodable::encode(&self.tx_hash, out);
        Encodable::encode(&self.receipts_hash, out);
        Encodable::encode(&self.bloom, out);
        Encodable::encode(&self.difficulty, out);
        Encodable::encode(&self.number, out);
        Encodable::encode(&self.gas_limit, out);
        Encodable::encode(&self.gas_used, out);
        Encodable::encode(&self.time, out);
        Encodable::encode(&self.extra, out);
        Encodable::encode(&self.mix_digest, out);
        Encodable::encode(&self.nonce, out);
        if let Some(base_fee) = self.base_fee {
            Encodable::encode(&base_fee, out);
        }
    }
    fn length(&self) -> usize {
        let rlp_head = self.rlp_header();
        fastrlp::length_of_length(rlp_head.payload_length) + rlp_head.payload_length
    }
}

// https://github.com/ledgerwatch/erigon/blob/156da607e7495d709c141aec40f66a2556d35dc0/core/types/block.go#L430
impl Decodable for BlockHeader {
    fn decode(buf: &mut &[u8]) -> Result<Self, DecodeError> {
        let rlp_head = fastrlp::Header::decode(buf)?;
        if !rlp_head.list {
            return Err(DecodeError::UnexpectedString);
        }
        let rest = buf.len() - rlp_head.payload_length;
        let parent_hash = Decodable::decode(buf)?;
        let uncle_hash = Decodable::decode(buf)?;
        let coinbase = Decodable::decode(buf)?;
        let root = Decodable::decode(buf)?;
        let tx_hash = Decodable::decode(buf)?;
        let receipts_hash = Decodable::decode(buf)?;
        let bloom = Decodable::decode(buf)?;
        let difficulty = Decodable::decode(buf)?;
        let number = Decodable::decode(buf)?;
        let gas_limit = Decodable::decode(buf)?;
        let gas_used = Decodable::decode(buf)?;
        let time = Decodable::decode(buf)?;
        let extra = Decodable::decode(buf)?;

        // TODO: seal fields
        let seal = None;
        let mix_digest = Decodable::decode(buf)?;
        let nonce = Decodable::decode(buf)?;
        let base_fee = if buf.len() > rest {
            Some(Decodable::decode(buf)?)
        } else {
            None
        };

        Ok(Self {
            parent_hash,
            uncle_hash,
            coinbase,
            root,
            tx_hash,
            receipts_hash,
            bloom,
            difficulty,
            number,
            gas_limit,
            gas_used,
            time,
            extra,
            mix_digest,
            nonce,
            base_fee,
            seal,
        })
    }
}
