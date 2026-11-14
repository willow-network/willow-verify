//! A BNB Smart Chain header: geth's fields and optional tail, its hash, the producer's seal over
//! it, and what its `extraData` carries (`types.Header`, `types.EncodeSigHeader`,
//! `getValidatorBytesFromHeader`, `getVoteAttestationFromHeader` in bnb-chain/bsc).

use crate::spec::Spec;
use crate::vote::Attestation;
use crate::Error;
use alloy_primitives::{keccak256, Address, Signature, B256, B64, U256};
use alloy_rlp::{Decodable, Encodable, Header as RlpHeader};
use serde_json::Value;

pub const EXTRA_VANITY: usize = 32;
pub const EXTRA_SEAL: usize = 65;
/// One validator in an epoch block from Luban: its address and BLS vote key.
pub const VALIDATOR_BYTES: usize = 20 + 48;
pub const VALIDATOR_BYTES_BEFORE_LUBAN: usize = 20;

/// keccak(RLP([])): the only uncle hash Parlia allows.
pub const EMPTY_UNCLE_HASH: B256 = B256::new(hex_literal(
    "1dcc4de8dec75d7aab85b567b6ccd41ad312451b948a7413f0a142fd40d49347",
));
/// The root of an empty trie: the only withdrawals root Parlia allows.
pub const EMPTY_ROOT: B256 = B256::new(hex_literal(
    "56e81f171bcc55a6ff8345e692c0f86e5b48e01b996cadc001622fb5e363b421",
));

const fn hex_literal(s: &str) -> [u8; 32] {
    let b = s.as_bytes();
    let mut out = [0u8; 32];
    let mut i = 0;
    while i < 32 {
        out[i] = (nibble(b[2 * i]) << 4) | nibble(b[2 * i + 1]);
        i += 1;
    }
    out
}

const fn nibble(c: u8) -> u8 {
    match c {
        b'0'..=b'9' => c - b'0',
        b'a'..=b'f' => c - b'a' + 10,
        _ => panic!("not lowercase hex"),
    }
}

/// The header fields, in RLP order. A tail field is present only when every one before it is.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Header {
    pub parent_hash: B256,
    pub uncle_hash: B256,
    pub coinbase: Address,
    pub state_root: B256,
    pub transactions_root: B256,
    pub receipts_root: B256,
    pub logs_bloom: [u8; 256],
    pub difficulty: U256,
    pub number: u64,
    pub gas_limit: u64,
    pub gas_used: u64,
    pub time: u64,
    pub extra: Vec<u8>,
    pub mix_digest: B256,
    pub nonce: B64,
    pub base_fee: Option<U256>,
    pub withdrawals_hash: Option<B256>,
    pub blob_gas_used: Option<u64>,
    pub excess_blob_gas: Option<u64>,
    pub parent_beacon_root: Option<B256>,
    pub requests_hash: Option<B256>,
}

/// Fields of a block's JSON that are not header fields the hash covers.
const NOT_HEADER: [&str; 7] = [
    "hash",
    "size",
    "totalDifficulty",
    "milliTimestamp",
    "transactions",
    "uncles",
    "withdrawals",
];

fn field<'a>(v: &'a Value, k: &str) -> Result<&'a str, Error> {
    v.get(k)
        .and_then(Value::as_str)
        .ok_or_else(|| Error::Malformed(format!("header JSON has no {k}")))
}

fn bytes_of(s: &str, k: &str) -> Result<Vec<u8>, Error> {
    let h = s
        .strip_prefix("0x")
        .ok_or_else(|| Error::Malformed(format!("{k} is not 0x-hex")))?;
    hex::decode(h).map_err(|_| Error::Malformed(format!("{k} is not hex")))
}

fn fixed<const N: usize>(v: &Value, k: &str) -> Result<[u8; N], Error> {
    bytes_of(field(v, k)?, k)?
        .try_into()
        .map_err(|_| Error::Malformed(format!("{k} is not {N} bytes")))
}

fn quantity(s: &str, k: &str) -> Result<U256, Error> {
    let h = s
        .strip_prefix("0x")
        .ok_or_else(|| Error::Malformed(format!("{k} is not a 0x quantity")))?;
    if h.is_empty() || (h.len() > 1 && h.starts_with('0')) || h.len() > 64 {
        return Err(Error::Malformed(format!("{k} is not a canonical quantity")));
    }
    U256::from_str_radix(h, 16).map_err(|_| Error::Malformed(format!("{k} is not hex")))
}

fn u64_of(s: &str, k: &str) -> Result<u64, Error> {
    let q = quantity(s, k)?;
    u64::try_from(q).map_err(|_| Error::Malformed(format!("{k} exceeds 64 bits")))
}

impl Header {
    /// From an `eth_getHeaderByNumber` or `eth_getBlockByNumber` result, with the block hash it
    /// reports, which must be the hash of these fields. A header field this code does not know
    /// (a later fork's) is refused: the hash or the seal may not cover it.
    pub fn from_json(v: &Value) -> Result<(Header, B256), Error> {
        let o = v
            .as_object()
            .ok_or_else(|| Error::Malformed("header JSON is not an object".into()))?;
        const KNOWN: [&str; 21] = [
            "parentHash",
            "sha3Uncles",
            "miner",
            "stateRoot",
            "transactionsRoot",
            "receiptsRoot",
            "logsBloom",
            "difficulty",
            "number",
            "gasLimit",
            "gasUsed",
            "timestamp",
            "extraData",
            "mixHash",
            "nonce",
            "baseFeePerGas",
            "withdrawalsRoot",
            "blobGasUsed",
            "excessBlobGas",
            "parentBeaconBlockRoot",
            "requestsHash",
        ];
        if let Some(k) = o
            .keys()
            .find(|k| !KNOWN.contains(&k.as_str()) && !NOT_HEADER.contains(&k.as_str()))
        {
            return Err(Error::Malformed(format!("unknown header field {k}")));
        }
        let opt = |k: &str| o.get(k).filter(|x| !x.is_null());
        let mut h = Header {
            parent_hash: B256::new(fixed(v, "parentHash")?),
            uncle_hash: B256::new(fixed(v, "sha3Uncles")?),
            coinbase: Address::new(fixed(v, "miner")?),
            state_root: B256::new(fixed(v, "stateRoot")?),
            transactions_root: B256::new(fixed(v, "transactionsRoot")?),
            receipts_root: B256::new(fixed(v, "receiptsRoot")?),
            logs_bloom: fixed(v, "logsBloom")?,
            difficulty: quantity(field(v, "difficulty")?, "difficulty")?,
            number: u64_of(field(v, "number")?, "number")?,
            gas_limit: u64_of(field(v, "gasLimit")?, "gasLimit")?,
            gas_used: u64_of(field(v, "gasUsed")?, "gasUsed")?,
            time: u64_of(field(v, "timestamp")?, "timestamp")?,
            extra: bytes_of(field(v, "extraData")?, "extraData")?,
            mix_digest: B256::new(fixed(v, "mixHash")?),
            nonce: B64::new(fixed(v, "nonce")?),
            base_fee: None,
            withdrawals_hash: None,
            blob_gas_used: None,
            excess_blob_gas: None,
            parent_beacon_root: None,
            requests_hash: None,
        };
        let s = |x: &Value, k: &str| {
            x.as_str()
                .map(str::to_string)
                .ok_or_else(|| Error::Malformed(format!("{k} is not a string")))
        };
        if let Some(x) = opt("baseFeePerGas") {
            h.base_fee = Some(quantity(&s(x, "baseFeePerGas")?, "baseFeePerGas")?);
        }
        if opt("withdrawalsRoot").is_some() {
            h.withdrawals_hash = Some(B256::new(fixed(v, "withdrawalsRoot")?));
        }
        if let Some(x) = opt("blobGasUsed") {
            h.blob_gas_used = Some(u64_of(&s(x, "blobGasUsed")?, "blobGasUsed")?);
        }
        if let Some(x) = opt("excessBlobGas") {
            h.excess_blob_gas = Some(u64_of(&s(x, "excessBlobGas")?, "excessBlobGas")?);
        }
        if opt("parentBeaconBlockRoot").is_some() {
            h.parent_beacon_root = Some(B256::new(fixed(v, "parentBeaconBlockRoot")?));
        }
        if opt("requestsHash").is_some() {
            h.requests_hash = Some(B256::new(fixed(v, "requestsHash")?));
        }
        h.check_tail()?;
        let reported = B256::new(fixed(v, "hash")?);
        let hash = h.hash();
        if hash != reported {
            return Err(Error::Malformed(format!(
                "block {} hashes to {hash}, not the {reported} reported",
                h.number
            )));
        }
        Ok((h, hash))
    }

    /// A present tail field needs every one before it (RLP optional fields).
    fn check_tail(&self) -> Result<(), Error> {
        let present = [
            self.base_fee.is_some(),
            self.withdrawals_hash.is_some(),
            self.blob_gas_used.is_some(),
            self.excess_blob_gas.is_some(),
            self.parent_beacon_root.is_some(),
            self.requests_hash.is_some(),
        ];
        let n = present.iter().take_while(|p| **p).count();
        if present[n..].iter().any(|p| *p) {
            return Err(Error::Malformed(format!(
                "block {}: a header tail field is set after a missing one",
                self.number
            )));
        }
        Ok(())
    }

    fn base_fields(&self, out: &mut Vec<u8>, extra: &[u8], chain_id: Option<u64>) {
        if let Some(c) = chain_id {
            U256::from(c).encode(out);
        }
        self.parent_hash.encode(out);
        self.uncle_hash.encode(out);
        self.coinbase.encode(out);
        self.state_root.encode(out);
        self.transactions_root.encode(out);
        self.receipts_root.encode(out);
        self.logs_bloom.as_slice().encode(out);
        self.difficulty.encode(out);
        self.number.encode(out);
        self.gas_limit.encode(out);
        self.gas_used.encode(out);
        self.time.encode(out);
        extra.encode(out);
        self.mix_digest.encode(out);
        self.nonce.encode(out);
    }

    fn tail_fields(&self, out: &mut Vec<u8>) {
        if let Some(x) = self.base_fee {
            x.encode(out);
        } else {
            return;
        }
        if let Some(x) = self.withdrawals_hash {
            x.encode(out);
        } else {
            return;
        }
        if let Some(x) = self.blob_gas_used {
            x.encode(out);
        } else {
            return;
        }
        if let Some(x) = self.excess_blob_gas {
            x.encode(out);
        } else {
            return;
        }
        if let Some(x) = self.parent_beacon_root {
            x.encode(out);
        } else {
            return;
        }
        if let Some(x) = self.requests_hash {
            x.encode(out);
        }
    }

    /// The header's RLP: what its hash is the keccak of.
    pub fn rlp(&self) -> Vec<u8> {
        let mut payload = Vec::with_capacity(640 + self.extra.len());
        self.base_fields(&mut payload, &self.extra, None);
        self.tail_fields(&mut payload);
        wrap_list(payload)
    }

    pub fn hash(&self) -> B256 {
        keccak256(self.rlp())
    }

    /// What the producer signs: the header with its chain ID first and `extraData` without the
    /// seal; from Bohr (`parentBeaconBlockRoot` set) also the tail fields. `None` when
    /// `extraData` is too short to hold a seal.
    pub fn seal_hash(&self, chain_id: u64) -> Option<B256> {
        let unsealed = self.extra.len().checked_sub(EXTRA_SEAL)?;
        let mut payload = Vec::with_capacity(640 + self.extra.len());
        self.base_fields(&mut payload, &self.extra[..unsealed], Some(chain_id));
        if let Some(pbr) = self.parent_beacon_root {
            self.base_fee.unwrap_or_default().encode(&mut payload);
            self.withdrawals_hash.unwrap_or_default().encode(&mut payload);
            self.blob_gas_used.unwrap_or_default().encode(&mut payload);
            self.excess_blob_gas.unwrap_or_default().encode(&mut payload);
            pbr.encode(&mut payload);
            if let Some(r) = self.requests_hash {
                r.encode(&mut payload);
            }
        }
        Some(keccak256(wrap_list(payload)))
    }

    /// The address that sealed the header.
    pub fn signer(&self, chain_id: u64) -> Result<Address, Error> {
        let hash = self.seal_hash(chain_id).ok_or_else(|| {
            Error::Malformed(format!("block {}: extraData holds no seal", self.number))
        })?;
        let sig = &self.extra[self.extra.len() - EXTRA_SEAL..];
        let v = sig[64];
        if v > 1 {
            return Err(Error::BadSeal(format!(
                "block {}: seal recovery id {v}",
                self.number
            )));
        }
        let sig = Signature::from_bytes_and_parity(&sig[..64], v == 1);
        sig.recover_address_from_prehash(&hash)
            .map_err(|e| Error::BadSeal(format!("block {}: {e}", self.number)))
    }

    /// The seal itself (the last 65 bytes of `extraData`).
    pub fn seal(&self) -> Option<&[u8]> {
        self.extra
            .len()
            .checked_sub(EXTRA_SEAL)
            .map(|i| &self.extra[i..])
    }

    /// From the header's RLP, which must be exactly one header and re-encode to the same bytes.
    pub fn from_rlp(bytes: &[u8]) -> Result<(Header, B256), Error> {
        let bad = |m: &str| Error::Malformed(format!("header RLP: {m}"));
        let mut buf = bytes;
        let head = RlpHeader::decode(&mut buf).map_err(|e| bad(&e.to_string()))?;
        if !head.list || head.payload_length != buf.len() {
            return Err(bad("not exactly one list"));
        }
        let mut p = buf;
        let d = |p: &mut &[u8]| -> Result<B256, Error> {
            B256::decode(p).map_err(|e| bad(&e.to_string()))
        };
        let parent_hash = d(&mut p)?;
        let uncle_hash = d(&mut p)?;
        let coinbase = Address::decode(&mut p).map_err(|e| bad(&e.to_string()))?;
        let state_root = d(&mut p)?;
        let transactions_root = d(&mut p)?;
        let receipts_root = d(&mut p)?;
        let bloom = alloy_rlp::Bytes::decode(&mut p).map_err(|e| bad(&e.to_string()))?;
        let logs_bloom: [u8; 256] = bloom[..].try_into().map_err(|_| bad("bloom length"))?;
        let u = |p: &mut &[u8]| u64::decode(p).map_err(|e| bad(&e.to_string()));
        let difficulty = U256::decode(&mut p).map_err(|e| bad(&e.to_string()))?;
        let number = u(&mut p)?;
        let gas_limit = u(&mut p)?;
        let gas_used = u(&mut p)?;
        let time = u(&mut p)?;
        let extra = alloy_rlp::Bytes::decode(&mut p)
            .map_err(|e| bad(&e.to_string()))?
            .to_vec();
        let mix_digest = d(&mut p)?;
        let nonce = B64::decode(&mut p).map_err(|e| bad(&e.to_string()))?;
        let mut h = Header {
            parent_hash,
            uncle_hash,
            coinbase,
            state_root,
            transactions_root,
            receipts_root,
            logs_bloom,
            difficulty,
            number,
            gas_limit,
            gas_used,
            time,
            extra,
            mix_digest,
            nonce,
            base_fee: None,
            withdrawals_hash: None,
            blob_gas_used: None,
            excess_blob_gas: None,
            parent_beacon_root: None,
            requests_hash: None,
        };
        if !p.is_empty() {
            h.base_fee = Some(U256::decode(&mut p).map_err(|e| bad(&e.to_string()))?);
        }
        if !p.is_empty() {
            h.withdrawals_hash = Some(d(&mut p)?);
        }
        if !p.is_empty() {
            h.blob_gas_used = Some(u(&mut p)?);
        }
        if !p.is_empty() {
            h.excess_blob_gas = Some(u(&mut p)?);
        }
        if !p.is_empty() {
            h.parent_beacon_root = Some(d(&mut p)?);
        }
        if !p.is_empty() {
            h.requests_hash = Some(d(&mut p)?);
        }
        if !p.is_empty() {
            return Err(bad("fields past requestsHash"));
        }
        if h.rlp() != bytes {
            return Err(bad("does not re-encode to the same bytes"));
        }
        let hash = keccak256(bytes);
        Ok((h, hash))
    }

    /// The milliseconds part of the block time (`mixHash` from Lorentz), as geth reads it.
    pub fn millis(&self) -> u64 {
        u64::from_be_bytes(self.mix_digest[24..].try_into().expect("8 bytes"))
    }

    /// The block time in milliseconds (`MilliTimestamp`).
    pub fn milli_time(&self) -> u64 {
        self.time.saturating_mul(1000).saturating_add(self.millis())
    }

    /// The validator bytes an epoch block carries (empty if none), as
    /// `getValidatorBytesFromHeader` cuts them.
    pub fn validator_bytes(&self, spec: &Spec, epoch: u64) -> &[u8] {
        if self.extra.len() <= EXTRA_VANITY + EXTRA_SEAL {
            return &[];
        }
        let body = &self.extra[EXTRA_VANITY..self.extra.len() - EXTRA_SEAL];
        if !spec.is_luban(self.number) {
            if self.number % epoch == 0 && body.len() % VALIDATOR_BYTES_BEFORE_LUBAN != 0 {
                return &[];
            }
            return body;
        }
        if self.number % epoch != 0 {
            return &[];
        }
        let n = self.extra[EXTRA_VANITY] as usize;
        let start = EXTRA_VANITY + 1;
        let end = start + n * VALIDATOR_BYTES;
        let min = end + EXTRA_SEAL + usize::from(spec.is_bohr(self.time));
        if n == 0 || self.extra.len() < min {
            return &[];
        }
        &self.extra[start..end]
    }

    /// The validator set an epoch block names: each address and its BLS vote key (zero before
    /// Luban), in the order the block lists them, and its turn length (from Bohr).
    pub fn epoch_set(&self, spec: &Spec, epoch: u64) -> Result<EpochSet, Error> {
        let bytes = self.validator_bytes(spec, epoch);
        if bytes.is_empty() {
            return Err(Error::BadExtra(format!(
                "epoch block {} names no validators",
                self.number
            )));
        }
        let validators = if !spec.is_luban(self.number) {
            bytes
                .chunks_exact(VALIDATOR_BYTES_BEFORE_LUBAN)
                .map(|c| (Address::from_slice(c), [0u8; 48]))
                .collect()
        } else {
            bytes
                .chunks_exact(VALIDATOR_BYTES)
                .map(|c| {
                    let mut key = [0u8; 48];
                    key.copy_from_slice(&c[20..]);
                    (Address::from_slice(&c[..20]), key)
                })
                .collect()
        };
        let turn_length = if self.number % epoch == 0 && spec.is_bohr(self.time) {
            let n = self.extra[EXTRA_VANITY] as usize;
            let pos = EXTRA_VANITY + 1 + n * VALIDATOR_BYTES;
            if self.extra.len() <= pos + EXTRA_SEAL {
                return Err(Error::BadExtra(format!(
                    "epoch block {} has no turn length",
                    self.number
                )));
            }
            Some(self.extra[pos])
        } else {
            None
        };
        Ok(EpochSet {
            validators,
            turn_length,
        })
    }

    /// The vote attestation the header carries, if any (`getVoteAttestationFromHeader`).
    pub fn attestation(&self, spec: &Spec, epoch: u64) -> Result<Option<Attestation>, Error> {
        if self.extra.len() <= EXTRA_VANITY + EXTRA_SEAL || !spec.is_luban(self.number) {
            return Ok(None);
        }
        let end = self.extra.len() - EXTRA_SEAL;
        let start = if self.number % epoch != 0 {
            EXTRA_VANITY
        } else {
            let n = self.extra[EXTRA_VANITY] as usize;
            EXTRA_VANITY + 1 + n * VALIDATOR_BYTES + usize::from(spec.is_bohr(self.time))
        };
        if end <= start {
            return Ok(None);
        }
        Attestation::decode(&self.extra[start..end])
            .map(Some)
            .map_err(|e| Error::BadExtra(format!("block {}: attestation: {e}", self.number)))
    }
}

/// The set an epoch block names.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EpochSet {
    pub validators: Vec<(Address, [u8; 48])>,
    pub turn_length: Option<u8>,
}

fn wrap_list(payload: Vec<u8>) -> Vec<u8> {
    let mut out = Vec::with_capacity(payload.len() + 5);
    RlpHeader {
        list: true,
        payload_length: payload.len(),
    }
    .encode(&mut out);
    out.extend_from_slice(&payload);
    out
}
