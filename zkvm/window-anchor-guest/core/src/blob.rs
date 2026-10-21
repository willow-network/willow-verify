//! Witness blob v1 — the byte layout the host (`genblob`) writes and the guest
//! reads. Hand-rolled little-endian; no serde, no bincode, nothing that can be
//! coaxed into allocating from a length field it has not bounds-checked.
//!
//! ```text
//!  off  size  field
//!    0     4  magic u32 LE = 0x57414E31 ("WAN1")
//!    4     8  anchor_slot u64 LE
//!   12    32  anchor_state_root
//!   44    96  branch_state_root      3 x [32], leaf->root (gindex 11 -> R)
//!  140     8  target_slot u64 LE
//!  148     1  path_kind u8           0 = Recent, 1 = Historical
//!  149     2  n_siblings u16 LE
//!  151  32n  siblings                leaf->root, target block root -> anchor_state_root
//!  151+32n     1  n_payload_leaves u8   15 Capella, 17 Deneb+
//!  152+32n  32m  payload_leaves        ExecutionPayload field HTRs, declaration order
//!  152+32n+32m 224  branch_payload    7 x [32], leaf->root (composed gindex 201)
//! ```
//!
//! Total = `376 + 32n + 32m`.
//!
//! Decoding is FAIL-CLOSED: bad magic, a short read, a byte left over, an unknown
//! `path_kind` or a payload-leaf count that is not an `ExecutionPayload` shape all
//! return `Err`. The sibling count is *not* validated here — its only legal value
//! is the depth of the generalized index the guest computes itself, so that check
//! lives in [`crate::run`] where the index exists.

use crate::error::Error;
use crate::fork_schedule::{CAPELLA_PAYLOAD_FIELDS, DENEB_PAYLOAD_FIELDS};
use crate::gindex::PathKind;
use beacon_cl_core::ssz::Bytes32;

/// `"WAN1"` read as a little-endian `u32`.
pub const BLOB_MAGIC: u32 = 0x5741_4E31;

/// Byte offsets of the fixed-position fields, so a test can corrupt one field
/// without re-deriving the layout (and so the layout has exactly one definition).
pub const OFF_MAGIC: usize = 0;
/// Offset of `anchor_slot`.
pub const OFF_ANCHOR_SLOT: usize = 4;
/// Offset of `anchor_state_root`.
pub const OFF_ANCHOR_STATE_ROOT: usize = 12;
/// Offset of `branch_state_root`.
pub const OFF_BRANCH_STATE_ROOT: usize = 44;
/// Offset of `target_slot`.
pub const OFF_TARGET_SLOT: usize = 140;
/// Offset of `path_kind`.
pub const OFF_PATH_KIND: usize = 148;
/// Offset of `n_siblings`.
pub const OFF_N_SIBLINGS: usize = 149;
/// Offset of the first sibling.
pub const OFF_SIBLINGS: usize = 151;
/// Blob size with zero siblings and zero payload leaves.
pub const BLOB_FIXED_LEN: usize = 376;

/// The decoded witness. Public so the tests (and `genblob`) can build one, mutate
/// a single field, and re-encode — the shape every adversarial case is a one-field
/// edit away from a passing control.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Witness {
    /// Slot of the anchor beacon block, whose root is what EIP-4788 returns.
    pub anchor_slot: u64,
    /// `BeaconBlock.state_root` at `anchor_slot`.
    pub anchor_state_root: Bytes32,
    /// Siblings from `anchor_state_root` (gindex 11) up to the anchor block root.
    pub branch_state_root: [Bytes32; 3],
    /// Slot the prover claims for the target block. Committed for traceability;
    /// see the soundness note in [`crate::gindex`].
    pub target_slot: u64,
    /// Which state shape the target block root is reached through.
    pub path: PathKind,
    /// Siblings from the target block root up to `anchor_state_root`, leaf->root.
    pub siblings: Vec<Bytes32>,
    /// `ExecutionPayload` field hash-tree-roots in declaration order.
    pub payload_leaves: Vec<Bytes32>,
    /// Siblings from the `ExecutionPayload` root (composed gindex 201) up to the
    /// target block root.
    pub branch_payload: [Bytes32; 7],
}

/// Serialize a [`Witness`]. Host/test side only — it validates capacity (the wire
/// counts are `u16` and `u8`) but deliberately NOT semantics, so a test can encode
/// a deliberately malformed witness and watch [`decode_blob`] reject it.
pub fn encode_blob(w: &Witness) -> Result<Vec<u8>, Error> {
    if w.siblings.len() > u16::MAX as usize {
        return Err(Error::SiblingCountMismatch {
            gindex: 0,
            expected: u16::MAX as usize,
            got: w.siblings.len(),
        });
    }
    if w.payload_leaves.len() > u8::MAX as usize {
        return Err(Error::PayloadLeafCountInvalid { got: u8::MAX });
    }

    let mut out =
        Vec::with_capacity(BLOB_FIXED_LEN + 32 * (w.siblings.len() + w.payload_leaves.len()));
    out.extend_from_slice(&BLOB_MAGIC.to_le_bytes());
    out.extend_from_slice(&w.anchor_slot.to_le_bytes());
    out.extend_from_slice(&w.anchor_state_root);
    for n in &w.branch_state_root {
        out.extend_from_slice(n);
    }
    out.extend_from_slice(&w.target_slot.to_le_bytes());
    out.push(w.path.as_u8());
    out.extend_from_slice(&(w.siblings.len() as u16).to_le_bytes());
    for n in &w.siblings {
        out.extend_from_slice(n);
    }
    out.push(w.payload_leaves.len() as u8);
    for n in &w.payload_leaves {
        out.extend_from_slice(n);
    }
    for n in &w.branch_payload {
        out.extend_from_slice(n);
    }
    Ok(out)
}

/// A bounds-checked forward cursor. Every read names its field, so a truncation
/// says which one ran off the end.
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
        // Bounds-check the WHOLE run before allocating, so a lying count cannot
        // make the guest reserve memory it will never fill.
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

/// Parse a witness blob. Fail-closed on every structural defect; see the module
/// docs for what is and is not checked here.
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
