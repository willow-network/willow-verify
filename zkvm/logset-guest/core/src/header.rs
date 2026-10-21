//! Execution-layer block header parsing.
//!
//! The guest hashes the header bytes it was handed and reads every field it uses
//! OUT OF THOSE SAME BYTES, so there is no seam through which a prover can smuggle
//! a `receiptsRoot` from one block and a `logsBloom` from another. The hash chain
//! (`parentHash[i] == keccak256(header[i-1])`) plus the committed `endBlockHash`
//! pins the whole walk to the canonical chain once the consumer checks
//! `endBlockHash` against EIP-2935.
//!
//! Only the first nine fields are read; forks append, so a header with more items
//! parses unchanged. That is deliberate — a typed decoder pinned to a field count
//! would start rejecting canonical mainnet headers at the next fork.
//!
//! ```text
//! idx  field
//!   0  parentHash        32
//!   1  ommersHash
//!   2  beneficiary
//!   3  stateRoot
//!   4  transactionsRoot
//!   5  receiptsRoot      32
//!   6  logsBloom        256
//!   7  difficulty
//!   8  number           u64, minimal big-endian
//! ```

use crate::error::Error;
use crate::rlp;

/// The four header fields this guest reads, borrowed from the hashed bytes.
#[derive(Clone, Copy, Debug)]
pub struct HeaderFields<'a> {
    pub parent_hash: &'a [u8; 32],
    pub receipts_root: &'a [u8; 32],
    pub logs_bloom: &'a [u8; 256],
    pub number: u64,
}

const IDX_PARENT_HASH: usize = 0;
const IDX_RECEIPTS_ROOT: usize = 5;
const IDX_LOGS_BLOOM: usize = 6;
const IDX_NUMBER: usize = 8;
/// The lowest field index this guest reads, plus one.
pub const MIN_HEADER_FIELDS: usize = IDX_NUMBER + 1;

fn fixed<'a, const N: usize>(
    item: &rlp::Item<'a>,
    block: u32,
    field: &'static str,
) -> Result<&'a [u8; N], Error> {
    if item.is_list {
        return Err(Error::HeaderFieldLen {
            block,
            field,
            got: usize::MAX,
        });
    }
    <&[u8; N]>::try_from(item.payload).map_err(|_| Error::HeaderFieldLen {
        block,
        field,
        got: item.payload.len(),
    })
}

/// Parse the fields the guest needs out of a header's RLP. `block` is the index in
/// the range, used only to name the failing header in an `Err`.
pub fn parse_header(bytes: &[u8], block: u32) -> Result<HeaderFields<'_>, Error> {
    let items = rlp::list(bytes)?;
    if items.len() < MIN_HEADER_FIELDS {
        return Err(Error::HeaderFieldCount {
            block,
            got: items.len(),
        });
    }
    let parent_hash = fixed::<32>(&items[IDX_PARENT_HASH], block, "parentHash")?;
    let receipts_root = fixed::<32>(&items[IDX_RECEIPTS_ROOT], block, "receiptsRoot")?;
    let logs_bloom = fixed::<256>(&items[IDX_LOGS_BLOOM], block, "logsBloom")?;
    let num_item = &items[IDX_NUMBER];
    if num_item.is_list {
        return Err(Error::BlockNumberNotCanonical { block });
    }
    let number = rlp::as_u64(num_item.payload).ok_or(Error::BlockNumberNotCanonical { block })?;
    Ok(HeaderFields {
        parent_hash,
        receipts_root,
        logs_bloom,
        number,
    })
}
