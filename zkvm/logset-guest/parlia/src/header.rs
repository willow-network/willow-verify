





use crate::codec::{next, open_list, ListWriter};
use crate::spec::Spec;

use crate::Error;
use alloy_primitives::{keccak256, Address, Bytes, Signature, B256, B64, U256};



pub const EXTRA_VANITY: usize = 32;

pub const EXTRA_SEAL: usize = 65;

pub const VALIDATOR_BYTES: usize = 20 + 48;

pub const VALIDATOR_BYTES_BEFORE_LUBAN: usize = 20;









const BARE_EXTRA: usize = EXTRA_VANITY + EXTRA_SEAL;



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



#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EpochSet {
    pub validators: Vec<(Address, [u8; 48])>,
    pub turn_length: Option<u8>,
}


pub(crate) fn on_epoch(number: u64, epoch_len: u64) -> bool {
    epoch_len != 0 && number % epoch_len == 0
}

impl Header {





























































    pub fn rlp(&self) -> Vec<u8> {
        let mut list = ListWriter::new();
        self.write_base(&mut list, &self.extra);
        self.write_tail(&mut list);
        list.finish()
    }


    pub fn hash(&self) -> B256 {
        keccak256(self.rlp())
    }




    pub fn seal_hash(&self, chain_id: u64) -> Option<B256> {
        let unsealed = self.extra.len().checked_sub(EXTRA_SEAL)?;
        let mut list = ListWriter::new();
        list.item(&chain_id);
        self.write_base(&mut list, &self.extra[..unsealed]);
        if let Some(beacon) = &self.parent_beacon_root {
            list.item(&self.base_fee.unwrap_or(U256::ZERO))
                .item(&self.withdrawals_hash.unwrap_or(B256::ZERO))
                .item(&self.blob_gas_used.unwrap_or(0))
                .item(&self.excess_blob_gas.unwrap_or(0))
                .item(beacon);
            if let Some(requests) = &self.requests_hash {
                list.item(requests);
            }
        }
        Some(keccak256(list.finish()))
    }


    pub fn signer(&self, chain_id: u64) -> Result<Address, Error> {
        let n = self.number;
        let digest = self
            .seal_hash(chain_id)
            .ok_or_else(|| malformed(format!("block {n}: extraData holds no seal")))?;
        let seal = &self.extra[self.extra.len() - EXTRA_SEAL..];
        let recovery_id = seal[64];
        if recovery_id > 1 {
            return Err(Error::BadSeal(format!(
                "block {n}: seal recovery id {recovery_id}"
            )));
        }
        Signature::from_bytes_and_parity(&seal[..64], recovery_id == 1)
            .recover_address_from_prehash(&digest)
            .map_err(|e| Error::BadSeal(format!("block {n}: {e}")))
    }


    pub fn seal(&self) -> Option<&[u8]> {
        let at = self.extra.len().checked_sub(EXTRA_SEAL)?;
        Some(&self.extra[at..])
    }


    pub fn from_rlp(bytes: &[u8]) -> Result<(Header, B256), Error> {
        let header =
            Self::read_rlp(bytes).map_err(|why| malformed(format!("header RLP: {why}")))?;
        Ok((header, keccak256(bytes)))
    }















    pub fn validator_bytes(&self, spec: &Spec, epoch: u64) -> &[u8] {
        let x = &self.extra;
        if x.len() <= BARE_EXTRA {
            return &[];
        }
        let n = self.number;
        if !spec.is_luban(n) {
            let body = &x[EXTRA_VANITY..x.len() - EXTRA_SEAL];
            if on_epoch(n, epoch) && body.len() % VALIDATOR_BYTES_BEFORE_LUBAN != 0 {
                return &[];
            }
            return body;
        }
        if !on_epoch(n, epoch) {
            return &[];
        }
        let count = x[EXTRA_VANITY] as usize;
        let end = EXTRA_VANITY + 1 + count * VALIDATOR_BYTES;
        let needed = end + EXTRA_SEAL + usize::from(spec.is_bohr(self.time));
        if count == 0 || x.len() < needed {
            return &[];
        }
        &x[EXTRA_VANITY + 1..end]
    }


    pub fn epoch_set(&self, spec: &Spec, epoch: u64) -> Result<EpochSet, Error> {
        let n = self.number;
        let section = self.validator_bytes(spec, epoch);
        if section.is_empty() {
            return Err(Error::BadExtra(format!(
                "epoch block {n} names no validators"
            )));
        }
        let validators = if spec.is_luban(n) {
            section
                .chunks_exact(VALIDATOR_BYTES)
                .map(|entry| {
                    let mut key = [0u8; 48];
                    key.copy_from_slice(&entry[20..]);
                    (Address::from_slice(&entry[..20]), key)
                })
                .collect()
        } else {
            section
                .chunks_exact(VALIDATOR_BYTES_BEFORE_LUBAN)
                .map(|entry| (Address::from_slice(entry), [0u8; 48]))
                .collect()
        };
        let mut turn_length = None;
        if on_epoch(n, epoch) && spec.is_bohr(self.time) {
            let at = self.after_validators(false);
            if self.extra.len() <= at + EXTRA_SEAL {
                return Err(Error::BadExtra(format!(
                    "epoch block {n} has no turn length"
                )));
            }
            turn_length = Some(self.extra[at]);
        }
        Ok(EpochSet {
            validators,
            turn_length,
        })
    }
























    fn after_validators(&self, with_turn_length: bool) -> usize {
        let count = self.extra[EXTRA_VANITY] as usize;
        EXTRA_VANITY + 1 + count * VALIDATOR_BYTES + usize::from(with_turn_length)
    }

    fn write_base(&self, list: &mut ListWriter, extra: &[u8]) {
        list.item(&self.parent_hash)
            .item(&self.uncle_hash)
            .item(&self.coinbase)
            .item(&self.state_root)
            .item(&self.transactions_root)
            .item(&self.receipts_root)
            .item(&self.logs_bloom[..])
            .item(&self.difficulty)
            .item(&self.number)
            .item(&self.gas_limit)
            .item(&self.gas_used)
            .item(&self.time)
            .item(extra)
            .item(&self.mix_digest)
            .item(&self.nonce);
    }

    fn write_tail(&self, list: &mut ListWriter) {
        let Some(base_fee) = &self.base_fee else {
            return;
        };
        list.item(base_fee);
        let Some(withdrawals) = &self.withdrawals_hash else {
            return;
        };
        list.item(withdrawals);
        let Some(blob_gas) = &self.blob_gas_used else {
            return;
        };
        list.item(blob_gas);
        let Some(excess) = &self.excess_blob_gas else {
            return;
        };
        list.item(excess);
        let Some(beacon) = &self.parent_beacon_root else {
            return;
        };
        list.item(beacon);
        if let Some(requests) = &self.requests_hash {
            list.item(requests);
        }
    }













    fn read_rlp(bytes: &[u8]) -> Result<Header, String> {
        let mut input = bytes;
        let payload = match open_list(&mut input) {
            Ok(Some(payload)) if input.is_empty() => payload,
            Ok(_) => return Err("not exactly one list".into()),
            Err(why) => return Err(why),
        };
        let mut p = payload;
        let parent_hash: B256 = next(&mut p)?;
        let uncle_hash: B256 = next(&mut p)?;
        let coinbase: Address = next(&mut p)?;
        let state_root: B256 = next(&mut p)?;
        let transactions_root: B256 = next(&mut p)?;
        let receipts_root: B256 = next(&mut p)?;
        let bloom: Bytes = next(&mut p)?;
        let logs_bloom: [u8; 256] = bloom
            .as_ref()
            .try_into()
            .map_err(|_| "bloom length".to_string())?;
        let difficulty: U256 = next(&mut p)?;
        let number: u64 = next(&mut p)?;
        let gas_limit: u64 = next(&mut p)?;
        let gas_used: u64 = next(&mut p)?;
        let time: u64 = next(&mut p)?;
        let extra: Bytes = next(&mut p)?;
        let mix_digest: B256 = next(&mut p)?;
        let nonce: B64 = next(&mut p)?;
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
            extra: extra.to_vec(),
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
            h.base_fee = Some(next(&mut p)?);
        }
        if !p.is_empty() {
            h.withdrawals_hash = Some(next(&mut p)?);
        }
        if !p.is_empty() {
            h.blob_gas_used = Some(next(&mut p)?);
        }
        if !p.is_empty() {
            h.excess_blob_gas = Some(next(&mut p)?);
        }
        if !p.is_empty() {
            h.parent_beacon_root = Some(next(&mut p)?);
        }
        if !p.is_empty() {
            h.requests_hash = Some(next(&mut p)?);
        }
        if !p.is_empty() {
            return Err("fields past requestsHash".into());
        }
        if h.rlp() != bytes {
            return Err("does not re-encode to the same bytes".into());
        }
        Ok(h)
    }
}

fn malformed(message: String) -> Error {
    Error::Malformed(message)
}





















































































































