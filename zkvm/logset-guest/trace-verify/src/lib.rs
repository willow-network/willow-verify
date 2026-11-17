











pub mod block;
pub mod call;
pub mod eth;
pub mod op;
pub mod output;
pub mod request;
pub mod response;
pub mod state;


pub mod trie;
pub mod wire;

use alloy_primitives::B256;
use block::Block;
use request::{Job, Tracer};
use serde_json::{json, Value};
use state::{Missing, Pre};




pub const ALGORITHM: &str = "willow-trace-v1";

pub const ETHEREUM: u64 = 1;
pub const UNICHAIN: u64 = willow_op_exec::UNICHAIN;

pub const MAX_TRACE_BYTES: usize = 128 << 20;

pub const TOO_LARGE: &str = "the trace records more than one response may hold here";



#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {

    InvalidRequest,

    Malformed,

    Unsupported,

    WrongChain,

    Header,

    WrongBlock,

    Parent,

    Transactions,

    Receipts,

    Limits,

    MissingRead,

    Disagrees,

    Algorithm,

    TooLarge,


    Account,
    Slot,
    Code,
    History,

    NoTrace,

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


#[derive(Debug)]
pub enum Stop {

    Missing(Missing),

    Disagrees(usize, String),

    Failed(String),

    TooLarge,

    NoBlockHash(u64),
}


pub type Traces = Vec<(usize, Value)>;



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

    match pre.take_missing() {
        Some(m) => Err(Stop::Missing(m)),
        None => out,
    }
}


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



























#[derive(Clone, Debug)]
pub struct ProvenTrace {
    pub chain_id: u64,
    pub number: u64,
    pub hash: B256,
    pub parent_hash: B256,
    pub timestamp: u64,

    pub method: String,

    pub result: Value,
}



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














