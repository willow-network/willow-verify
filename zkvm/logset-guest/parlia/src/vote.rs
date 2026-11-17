


use alloy_primitives::{keccak256, B256};
use alloy_rlp::{Decodable, Encodable, Header as RlpHeader};






pub const MAX_EXTRA: usize = 256;





#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VoteData {
    pub source_number: u64,
    pub source_hash: B256,
    pub target_number: u64,
    pub target_hash: B256,
}

impl VoteData {
    fn rlp(&self) -> Vec<u8> {
        let mut payload = Vec::with_capacity(84);
        self.source_number.encode(&mut payload);
        self.source_hash.encode(&mut payload);
        self.target_number.encode(&mut payload);
        self.target_hash.encode(&mut payload);
        let mut out = Vec::with_capacity(payload.len() + 2);
        RlpHeader {
            list: true,
            payload_length: payload.len(),
        }
        .encode(&mut out);
        out.extend_from_slice(&payload);
        out
    }


    pub fn hash(&self) -> B256 {
        keccak256(self.rlp())
    }
}


#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Attestation {

    pub voters: u64,
    pub signature: [u8; 96],
    pub data: VoteData,
    pub extra: Vec<u8>,
}

fn take_list<'a>(buf: &mut &'a [u8]) -> Result<&'a [u8], String> {
    let h = RlpHeader::decode(buf).map_err(|e| e.to_string())?;
    if !h.list || h.payload_length > buf.len() {
        return Err("not a list".into());
    }
    let whole: &'a [u8] = buf;
    let (inner, rest) = whole.split_at(h.payload_length);
    *buf = rest;
    Ok(inner)
}

impl Attestation {

    pub fn decode(bytes: &[u8]) -> Result<Attestation, String> {
        let mut buf = bytes;
        let mut p = take_list(&mut buf)?;
        if !buf.is_empty() {
            return Err(format!("{} bytes after it", buf.len()));
        }
        let voters = u64::decode(&mut p).map_err(|e| e.to_string())?;
        let sig = alloy_rlp::Bytes::decode(&mut p).map_err(|e| e.to_string())?;
        let signature: [u8; 96] = sig[..]
            .try_into()
            .map_err(|_| format!("signature is {} bytes", sig.len()))?;
        let mut d = take_list(&mut p)?;
        let data = VoteData {
            source_number: u64::decode(&mut d).map_err(|e| e.to_string())?,
            source_hash: B256::decode(&mut d).map_err(|e| e.to_string())?,
            target_number: u64::decode(&mut d).map_err(|e| e.to_string())?,
            target_hash: B256::decode(&mut d).map_err(|e| e.to_string())?,
        };
        if !d.is_empty() {
            return Err("vote data has more than 4 fields".into());
        }
        let extra = alloy_rlp::Bytes::decode(&mut p)
            .map_err(|e| e.to_string())?
            .to_vec();
        if !p.is_empty() {
            return Err("more than 4 fields".into());
        }
        Ok(Attestation {
            voters,
            signature,
            data,
            extra,
        })
    }
}







































