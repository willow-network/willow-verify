//! Trace proofs: what Willow sends for `willow_getTraceProof [method, params]` ([`wire`]) and the
//! check a client runs on it ([`verify_trace`]). The proof names the block, carries its parent's
//! header (whose state root the block runs over), the headers back to the oldest block hash the
//! block reads, every transaction and receipt of the block, and the trie nodes and codes its run
//! reads under the parent's state root. The client checks the headers link, the transactions and
//! receipts against their roots, runs the block itself over those nodes (strictly: a read the
//! proof does not hold fails), checks every transaction it ran against its receipt, and writes
//! the trace with the same tracers the server's trace methods write it with. The server gathers
//! the proof with the same run ([`gather`]) and checks what it sends with [`verify_trace`]. A
//! call's traces (`debug_traceCall`, `trace_call`) are proven by a call proof's block and witness,
//! the call run over it as `eth_call` runs it, traced ([`call`]).

pub mod block;
pub mod call;
pub mod eth;
pub mod op;
pub mod output;
pub mod request;
pub mod response;
pub mod state;
#[cfg(test)]
mod tests;
pub mod trie;
pub mod wire;

use alloy_primitives::{Address, B256};
use block::Block;
use request::{Asked, Job, Tracer};
use serde_json::{json, Value};
use state::{Missing, Pre};
use std::collections::BTreeSet;

/// `willow_getTraceProof`'s `algorithm`: how the block is run and its traces written. The client
/// refuses a name it doesn't know.
pub const ALGORITHM: &str = "willow-trace-v1";
/// The chains whose traces are proven: Ethereum, and Unichain (as op-reth runs its blocks).
pub const ETHEREUM: u64 = 1;
pub const UNICHAIN: u64 = willow_op_exec::UNICHAIN;
/// What one request's traces may record, roughly in bytes (the server's trace methods' bound).
pub const MAX_TRACE_BYTES: usize = 128 << 20;
/// A run past [`MAX_TRACE_BYTES`], as the EVM's own error carries it.
pub const TOO_LARGE: &str = "the trace records more than one response may hold here";

/// Why a proof did not verify. The client decides by `kind`; the tamper suite expects one per
/// mutation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// The client's own request can't be read.
    InvalidRequest,
    /// The result can't be read (a field missing, not hex, the wrong length).
    Malformed,
    /// Not proven here: another chain, a call's trace, a tracer not run here.
    Unsupported,
    /// The proof is for another chain.
    WrongChain,
    /// The header does not hash to `blockHash`, does not decode, or is not `blockNumber`.
    Header,
    /// The proof is for another block than the one asked for, or one without the transaction.
    WrongBlock,
    /// The parent's header, or a header before it, is not the one the block links to.
    Parent,
    /// The transactions are not the block's (its transactionsRoot).
    Transactions,
    /// The receipts are not the block's (its receiptsRoot), or not one per transaction.
    Receipts,
    /// The witness is larger than one trace may read.
    Limits,
    /// The run read an account, slot, code or block hash the proof does not hold.
    MissingRead,
    /// A transaction ran otherwise than its receipt records.
    Disagrees,
    /// Traced by an algorithm this client doesn't know.
    Algorithm,
    /// The traces record more than one response may hold.
    TooLarge,
    /// A call's witness: an account's, a slot's, a code's or the history account's proof does
    /// not verify (`state_verify`'s kinds).
    Account,
    Slot,
    Code,
    History,
    /// The call comes to no trace (it is invalid at the block): no proof claims one.
    NoTrace,
    /// The EVM refused the block or itself, not a transaction.
    Internal,
}

impl Kind {
    pub fn as_str(self) -> &'static str {
        match self {
            Kind::InvalidRequest => "invalid-request",
            Kind::Malformed => "malformed",
            Kind::Unsupported => "unsupported",
            Kind::WrongChain => "wrong-chain",
            Kind::Header => "header",
            Kind::WrongBlock => "wrong-block",
            Kind::Parent => "parent",
            Kind::Transactions => "transactions",
            Kind::Receipts => "receipts",
            Kind::Limits => "limits",
            Kind::MissingRead => "missing-read",
            Kind::Disagrees => "disagrees",
            Kind::Algorithm => "algorithm",
            Kind::TooLarge => "too-large",
            Kind::Account => "account",
            Kind::Slot => "slot",
            Kind::Code => "code",
            Kind::History => "history",
            Kind::NoTrace => "no-trace",
            Kind::Internal => "internal",
        }
    }
}

/// A proof that did not verify: its kind, and what exactly.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Failure {
    pub kind: Kind,
    pub message: String,
}

impl Failure {
    pub fn new(kind: Kind, message: impl Into<String>) -> Self {
        Failure {
            kind,
            message: message.into(),
        }
    }

    /// `{"ok": false, "kind", "message"}`.
    pub fn json(&self) -> Value {
        json!({"ok": false, "kind": self.kind.as_str(), "message": self.message})
    }
}

impl std::fmt::Display for Failure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.kind.as_str(), self.message)
    }
}

impl std::error::Error for Failure {}

fn malformed(m: impl Into<String>) -> Failure {
    Failure::new(Kind::Malformed, m)
}

/// Why a run gave no trace.
#[derive(Debug)]
pub enum Stop {
    /// It read state the pre-state does not hold: what to prove.
    Missing(Missing),
    /// Transaction `i` ran otherwise than its receipt records.
    Disagrees(usize, String),
    /// The EVM refused the block, a transaction or the tracer: why.
    Failed(String),
    /// More than one trace may record.
    TooLarge,
    /// It read the hash of a block the pre-state does not prove.
    NoBlockHash(u64),
}

/// Each traced transaction's trace, in block order.
pub type Traces = Vec<(usize, Value)>;

/// `block` run over `pre` (its parent's state) through transaction `last`, the transactions
/// `wanted` says traced under `tracer`, every one checked against its receipt.
pub fn run(
    block: &Block,
    pre: &Pre<'_>,
    tracer: &Tracer,
    last: usize,
    wanted: &dyn Fn(usize) -> bool,
) -> Result<Traces, Stop> {
    let out = match block.chain_id {
        UNICHAIN => op::run_block(block, pre, tracer, last, wanted),
        _ => eth::run_block(block, pre, tracer, last, wanted),
    };
    // Whatever the run came to, a read the pre-state lacks comes first: it decides nothing.
    match pre.take_missing() {
        Some(m) => Err(Stop::Missing(m)),
        None => out,
    }
}

/// Which transactions a job runs: through `last`, those `wanted` says traced.
fn span(block: &Block, job: &Job) -> Result<(usize, Option<usize>), Failure> {
    match job {
        Job::Block { .. } => Ok((block.txs.len().saturating_sub(1), None)),
        Job::Call { .. } => Err(Failure::new(
            Kind::Unsupported,
            "a call's trace runs no block",
        )),
        Job::Tx { hash, .. } => {
            let i = block
                .txs
                .iter()
                .position(|t| t.hash == *hash)
                .ok_or_else(|| {
                    Failure::new(
                        Kind::WrongBlock,
                        format!("block {} has no transaction {hash}", block.number),
                    )
                })?;
            Ok((i, Some(i)))
        }
    }
}

/// What a run of `asked` over `pre` read, for the proof of it: the trie nodes and codes, and the
/// block hashes.
#[derive(Clone, Debug, Default)]
pub struct Reads {
    pub nodes: BTreeSet<B256>,
    pub codes: BTreeSet<B256>,
    pub block_hashes: BTreeSet<u64>,
}

/// The server's run of `asked` over `pre` (with nothing strict about it: what the pre-state lacks
/// is noted and read as empty): the response, or why there is none (with what to prove), and what
/// the run read.
pub fn gather(block: &Block, pre: &Pre<'_>, asked: &Asked) -> (Result<Value, Stop>, Reads) {
    let out = (|| {
        let (last, only) = span(block, &asked.job).map_err(|f| Stop::Failed(f.message))?;
        if block.txs.is_empty() {
            return Ok(response::block_response(&asked.method, block, vec![]));
        }
        let wanted = |i: usize| only.is_none_or(|x| x == i);
        let traces = run(block, pre, asked.job.tracer(), last, &wanted)?;
        Ok(response::block_response(&asked.method, block, traces))
    })();
    (out, pre.reads())
}

/// A verified `willow_getTraceProof`.
#[derive(Clone, Debug)]
pub struct ProvenTrace {
    pub chain_id: u64,
    pub number: u64,
    pub hash: B256,
    pub parent_hash: B256,
    pub timestamp: u64,
    /// The standard method asked for (`debug_traceTransaction`, `trace_block`, ...).
    pub method: String,
    /// Its standard response's `result`, as Willow's trace method sends it.
    pub result: Value,
}

/// `result` checked as the response to `willow_getTraceProof` `params` (`[method, params]`) on
/// chain `chain_id`: the client's own run of the block comes to the response it returns.
pub fn verify_trace(chain_id: u64, params: &Value, result: &Value) -> Result<ProvenTrace, Failure> {
    let asked = request::parse(chain_id, params)?;
    if let Job::Call { call, tracer } = &asked.job {
        return verify_call_trace(chain_id, &asked.method, call, tracer, result);
    }
    let proof: wire::TraceProof = serde_json::from_value(result.clone())
        .map_err(|e| malformed(format!("the result: {e}")))?;
    if proof.algorithm != ALGORITHM {
        return Err(Failure::new(
            Kind::Algorithm,
            format!("traces by {:?}, not {ALGORITHM}", proof.algorithm),
        ));
    }
    let mut block = Block::from_proof(chain_id, &proof)?;
    if let Job::Block { at, .. } = &asked.job {
        at.check(&block)?;
    }
    let (last, only) = span(&block, &asked.job)?;
    block.recover_through(last)?;
    let trie = trie::Trie::from_wire(&proof.witness)?;
    let result = if block.txs.is_empty() {
        response::block_response(&asked.method, &block, vec![])
    } else {
        let hash_of = |n: u64| block.hash_of(n);
        let pre = Pre::new(&trie, block.parent_root, &hash_of);
        let wanted = |i: usize| only.is_none_or(|x| x == i);
        let traces = run(&block, &pre, asked.job.tracer(), last, &wanted).map_err(|s| match s {
            Stop::Missing(m) => Failure::new(
                Kind::MissingRead,
                format!("the block read state the proof does not hold: {m:?}"),
            ),
            Stop::NoBlockHash(n) => Failure::new(
                Kind::MissingRead,
                format!("the block read the hash of block {n}, which the proof does not prove"),
            ),
            Stop::Disagrees(i, why) => Failure::new(
                Kind::Disagrees,
                format!("transaction {i} ran otherwise than its receipt records: {why}"),
            ),
            Stop::TooLarge => Failure::new(Kind::TooLarge, TOO_LARGE),
            Stop::Failed(m) => Failure::new(Kind::Internal, m),
        })?;
        response::block_response(&asked.method, &block, traces)
    };
    Ok(ProvenTrace {
        chain_id,
        number: block.number,
        hash: block.hash,
        parent_hash: block.header.parent_hash,
        timestamp: block.header.timestamp,
        method: asked.method,
        result,
    })
}

/// A call's trace proof (`result`) checked as the response to `method` asking for `req` traced
/// under `tracer`: the block named and its witness verified (`state_verify`, as a call proof's),
/// and the client's own strict run of the call, traced, is the response.
fn verify_call_trace(
    chain_id: u64,
    method: &str,
    req: &state_verify::CallRequest,
    tracer: &Tracer,
    result: &Value,
) -> Result<ProvenTrace, Failure> {
    let proof: wire::CallTraceProof = serde_json::from_value(result.clone())
        .map_err(|e| malformed(format!("the result: {e}")))?;
    if proof.algorithm != ALGORITHM {
        return Err(Failure::new(
            Kind::Algorithm,
            format!("traces by {:?}, not {ALGORITHM}", proof.algorithm),
        ));
    }
    let named = state_verify::named(
        chain_id,
        req.at,
        (
            &proof.chain_id,
            &proof.block_number,
            &proof.block_hash,
            &proof.header,
        ),
    )
    .map_err(call::failure)?;
    let w = state_verify::proven_witness(req, &named, &proof.witness).map_err(call::failure)?;
    let env = state_verify::env_of(&named);
    let hash_of = state_verify::hashes(chain_id, &w, &env);
    let run = state_verify::call::Run {
        engine: req.engine,
        witness: &w,
        block: &env,
        call: &req.call,
        overrides: &req.overrides,
        hash_of: &hash_of,
        mode: state_verify::call::Mode::Strict,
    };
    let (ran, trace) = call::run_call(&run, tracer, &mut Default::default());
    let Some(result) = trace else {
        return Err(call::unfinished(ran));
    };
    Ok(ProvenTrace {
        chain_id,
        number: named.number,
        hash: named.hash,
        parent_hash: named.header.parent_hash,
        timestamp: named.header.timestamp,
        method: method.to_string(),
        result,
    })
}

/// The block hashes a proof must carry headers for, and the code owners noted: helpers the server
/// fills a proof's gaps with.
pub fn to_prove(m: &Missing) -> std::collections::BTreeMap<Address, Vec<B256>> {
    let mut out: std::collections::BTreeMap<Address, Vec<B256>> = Default::default();
    for (a, slots) in m.accounts.iter().chain(&m.slots) {
        out.entry(*a).or_default().extend(slots.iter().copied());
    }
    for slots in out.values_mut() {
        slots.sort();
        slots.dedup();
    }
    out
}
