//! One block's data, checked against its block hash.
//!
//! - [`header`]: header JSON → header RLP (alloy's codec) whose keccak must be
//!   the block hash the JSON reports, read back by two decoders.
//! - [`receipts`]: receipts JSON → trie values (legacy and typed via alloy,
//!   hand-encoded OP-stack `0x7E` deposits and Nitro `0x64..=0x6A`, each gated
//!   per chain by `logset_chains::receipt_gate`) → the receipts root under two
//!   trie implementations, and the logs read back from the value bytes.
//! - [`transactions`]: full-block transactions JSON → canonical consensus bytes
//!   (alloy for the Ethereum types, hand-encoded OP-stack `0x7E` and Nitro
//!   `0x64..=0x6A`, gated per chain) → their hashes.
//! - [`trie`]: the ordered trie root, under two trie implementations.
//! - [`block`]: all together: a header that hashes to its block hash, a body
//!   (transactions, uncles, withdrawals) that rebuilds its roots, receipts that
//!   rebuild its receiptsRoot, and the logs at their positions
//!   (transactionIndex = receipt position, logIndex = position in the block).
//!
//! The SP1 feeder (`genblob`) builds witness blobs with these encoders, and the
//! log store verifies every block with them before serving its logs, so a block
//! the store serves is one the prover accepts.

pub mod block;
pub mod header;
pub mod receipts;
pub mod transactions;
pub mod trie;

pub use block::{
    block_logs, bloom_may_contain, verify_body, verify_header, verify_receipts, BlockLog,
    VerifiedBody, VerifiedHeader,
};
pub use receipts::Log;
