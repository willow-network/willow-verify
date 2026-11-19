





use crate::codec::{next, open_list, ListWriter};
use alloy_primitives::{keccak256, Bytes, B256};






pub const MAX_EXTRA: usize = 256;





#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VoteData {
    pub source_number: u64,
    pub source_hash: B256,
    pub target_number: u64,
    pub target_hash: B256,
}

impl VoteData {

    pub fn hash(&self) -> B256 {
        let mut list = ListWriter::new();
        list.item(&self.source_number)
            .item(&self.source_hash)
            .item(&self.target_number)
            .item(&self.target_hash);
        keccak256(list.finish())
    }
}


#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Attestation {
    pub voters: u64,
    pub signature: [u8; 96],
    pub data: VoteData,
    pub extra: Vec<u8>,
}

impl Attestation {

    pub fn decode(bytes: &[u8]) -> Result<Attestation, String> {
        let mut input = bytes;
        let mut outer = open_list(&mut input)?.ok_or_else(|| "not a list".to_string())?;
        if !input.is_empty() {
            return Err(format!("{} bytes after it", input.len()));
        }
        let voters: u64 = next(&mut outer)?;
        let sig: Bytes = next(&mut outer)?;
        let signature: [u8; 96] = sig
            .as_ref()
            .try_into()
            .map_err(|_| format!("signature is {} bytes", sig.len()))?;
        let mut inner = open_list(&mut outer)?.ok_or_else(|| "not a list".to_string())?;
        let data = VoteData {
            source_number: next(&mut inner)?,
            source_hash: next(&mut inner)?,
            target_number: next(&mut inner)?,
            target_hash: next(&mut inner)?,
        };
        if !inner.is_empty() {
            return Err("vote data has more than 4 fields".into());
        }
        let extra: Bytes = next(&mut outer)?;
        if !outer.is_empty() {
            return Err("more than 4 fields".into());
        }
        Ok(Attestation {
            voters,
            signature,
            data,
            extra: extra.to_vec(),
        })
    }
}







































