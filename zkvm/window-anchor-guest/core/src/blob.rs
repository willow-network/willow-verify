


























use crate::error::Error;
use crate::fork_schedule::{CAPELLA_PAYLOAD_FIELDS, DENEB_PAYLOAD_FIELDS};
use crate::gindex::PathKind;
use beacon_cl_core::ssz::Bytes32;


pub const BLOB_MAGIC: u32 = 0x5741_4E31;













pub const OFF_PATH_KIND: usize = 148;










#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Witness {

    pub anchor_slot: u64,

    pub anchor_state_root: Bytes32,

    pub branch_state_root: [Bytes32; 3],


    pub target_slot: u64,

    pub path: PathKind,

    pub siblings: Vec<Bytes32>,

    pub payload_leaves: Vec<Bytes32>,


    pub branch_payload: [Bytes32; 7],
}










































struct Reader<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, pos: 0 }
    }

    fn take(&mut self, n: usize, field: &'static str) -> Result<&'a [u8], Error> {
        let remaining = self.bytes.len() - self.pos;
        if n > remaining {
            return Err(Error::Truncated {
                field,
                need: n,
                have: remaining,
            });
        }
        let out = &self.bytes[self.pos..self.pos + n];
        self.pos += n;
        Ok(out)
    }

    fn u8(&mut self, field: &'static str) -> Result<u8, Error> {
        Ok(self.take(1, field)?[0])
    }

    fn u16(&mut self, field: &'static str) -> Result<u16, Error> {
        let b = self.take(2, field)?;
        Ok(u16::from_le_bytes([b[0], b[1]]))
    }

    fn u32(&mut self, field: &'static str) -> Result<u32, Error> {
        let b = self.take(4, field)?;
        Ok(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }

    fn u64(&mut self, field: &'static str) -> Result<u64, Error> {
        let b = self.take(8, field)?;
        let mut a = [0u8; 8];
        a.copy_from_slice(b);
        Ok(u64::from_le_bytes(a))
    }

    fn node(&mut self, field: &'static str) -> Result<Bytes32, Error> {
        let b = self.take(32, field)?;
        let mut a = [0u8; 32];
        a.copy_from_slice(b);
        Ok(a)
    }

    fn nodes(&mut self, n: usize, field: &'static str) -> Result<Vec<Bytes32>, Error> {


        let bytes = self.take(32 * n, field)?;
        Ok(bytes
            .chunks_exact(32)
            .map(|c| {
                let mut a = [0u8; 32];
                a.copy_from_slice(c);
                a
            })
            .collect())
    }

    fn finish(&self) -> Result<(), Error> {
        let extra = self.bytes.len() - self.pos;
        if extra != 0 {
            return Err(Error::TrailingBytes { extra });
        }
        Ok(())
    }
}



pub fn decode_blob(blob: &[u8]) -> Result<Witness, Error> {
    let mut r = Reader::new(blob);

    let magic = r.u32("magic")?;
    if magic != BLOB_MAGIC {
        return Err(Error::BadMagic { got: magic });
    }

    let anchor_slot = r.u64("anchor_slot")?;
    let anchor_state_root = r.node("anchor_state_root")?;
    let mut branch_state_root = [[0u8; 32]; 3];
    for (i, n) in branch_state_root.iter_mut().enumerate() {
        *n = r.node(if i == 0 {
            "branch_state_root[0]"
        } else if i == 1 {
            "branch_state_root[1]"
        } else {
            "branch_state_root[2]"
        })?;
    }

    let target_slot = r.u64("target_slot")?;
    let path = PathKind::from_u8(r.u8("path_kind")?)?;

    let n_siblings = r.u16("n_siblings")? as usize;
    let siblings = r.nodes(n_siblings, "siblings")?;

    let n_payload_leaves = r.u8("n_payload_leaves")?;
    if n_payload_leaves as usize != CAPELLA_PAYLOAD_FIELDS
        && n_payload_leaves as usize != DENEB_PAYLOAD_FIELDS
    {
        return Err(Error::PayloadLeafCountInvalid {
            got: n_payload_leaves,
        });
    }
    let payload_leaves = r.nodes(n_payload_leaves as usize, "payload_leaves")?;

    let mut branch_payload = [[0u8; 32]; 7];
    for n in branch_payload.iter_mut() {
        *n = r.node("branch_payload")?;
    }

    r.finish()?;

    Ok(Witness {
        anchor_slot,
        anchor_state_root,
        branch_state_root,
        target_slot,
        path,
        siblings,
        payload_leaves,
        branch_payload,
    })
}
