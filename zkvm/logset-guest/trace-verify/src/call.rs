//! A call's traces (`debug_traceCall`, `trace_call` on Ethereum): the call run as the log RPC's
//! call traces run it (`state/trace.rs` `run_call`: as `eth_call` runs it, reth's gas rules,
//! under the caller's overrides) with the trace methods' tracers attached. The proof is a call
//! proof's (`willow_getCallProof`'s block and witness, `state_verify`): the client verifies the
//! witness against the block's state root and runs the call over it strictly, so the trace it
//! writes is the one the proven state makes. A tracer reads nothing the call doesn't (the
//! prestate tracer's accounts are the call's, its sender's, recipient's and fee recipient's),
//! so the witness needs nothing more; the server gathers it with [`run_call`] too.

use crate::eth;
use crate::output::Budget;
use crate::request::Tracer;
use crate::{Failure, Kind, MAX_TRACE_BYTES, TOO_LARGE};
use alloy::rpc::types::TransactionInfo;
use alloy_evm::EvmEnv;
use serde_json::Value;
use state_verify::call::{self as vcall, Missing, Ran, Reads, Run};
use std::cell::RefCell;

/// `run`'s call traced under `tracer` (on Ethereum): what it came to and, where it ran, its trace;
/// what it read of the witness added to `reads`. A read the witness lacks comes first: then
/// `Ran::Missing` (gathering: what to prove; strict: the first such read) and no trace.
pub fn run_call(run: &Run<'_>, tracer: &Tracer, reads: &mut Reads) -> (Ran, Option<Value>) {
    let missing = RefCell::new(Missing::default());
    let read = RefCell::new(std::mem::take(reads));
    let out = traced(run, tracer, &missing, &read);
    *reads = read.into_inner();
    let missing = missing.into_inner();
    if !missing.is_empty() {
        return (Ran::Missing(missing), None);
    }
    match out {
        Err(ran) => (ran, None),
        Ok(Ok((trace, res))) => (
            vcall::ran(Ok(res.result), Missing::default(), vcall::halted),
            Some(trace),
        ),
        Ok(Err(e)) => (vcall::ran(Err(e), Missing::default(), vcall::halted), None),
    }
}

type Traced = Result<
    (Value, revm::context::result::ResultAndState),
    revm::context::result::EVMError<vcall::Unreadable>,
>;

fn traced<'b>(
    run: &'b Run<'_>,
    tracer: &Tracer,
    missing: &'b RefCell<Missing>,
    read: &'b RefCell<Reads>,
) -> Result<Traced, Ran> {
    if run.engine != vcall::Engine::Ethereum {
        return Err(Ran::Refused(
            "calls' traces are run on Ethereum only".into(),
        ));
    }
    let (cfg_env, block_env, tx, mut db) = run.eth_prepared(missing, read)?;
    let env = EvmEnv { cfg_env, block_env };
    let budget = Budget::new(MAX_TRACE_BYTES);
    Ok(eth::traced(
        tracer,
        &mut db,
        &env,
        tx,
        TransactionInfo::default(),
        &budget,
        true,
    ))
}

/// A strict run's failure to come to a trace, as the proof's failure.
pub fn unfinished(ran: Ran) -> Failure {
    match ran {
        Ran::Missing(m) => Failure::new(
            Kind::MissingRead,
            format!("the call read state the witness does not hold: {m:?}"),
        ),
        Ran::NoBlockHash(n) => Failure::new(
            Kind::MissingRead,
            format!("the call read the hash of block {n}, which the witness does not prove"),
        ),
        Ran::Refused(m) => {
            Failure::new(Kind::Unsupported, format!("the call is not run here: {m}"))
        }
        Ran::Internal(m) if m == TOO_LARGE => Failure::new(Kind::TooLarge, m),
        Ran::Internal(m) => Failure::new(Kind::Internal, m),
        // An invalid call has no trace: the trace method refuses it, and so does its proof.
        Ran::Invalid(m) => Failure::new(
            Kind::NoTrace,
            format!("the call is not run at this block, so it has no trace: {m}"),
        ),
        other => Failure::new(
            Kind::Internal,
            format!("the call came to {other:?} without a trace"),
        ),
    }
}

/// A `state_verify` failure of the call's block or witness, as this crate's.
pub fn failure(f: state_verify::Failure) -> Failure {
    use state_verify::Kind as S;
    let kind = match f.kind {
        S::InvalidRequest => Kind::InvalidRequest,
        S::Malformed => Kind::Malformed,
        S::Unsupported => Kind::Unsupported,
        S::WrongChain => Kind::WrongChain,
        S::Header => Kind::Header,
        S::WrongBlock => Kind::WrongBlock,
        S::Limits => Kind::Limits,
        S::Account => Kind::Account,
        S::Slot => Kind::Slot,
        S::Code => Kind::Code,
        S::History => Kind::History,
        S::MissingRead => Kind::MissingRead,
        S::Algorithm => Kind::Algorithm,
        S::Outcome | S::Estimate | S::Internal => Kind::Internal,
    };
    Failure::new(kind, f.message)
}
