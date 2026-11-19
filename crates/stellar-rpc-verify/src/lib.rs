





















pub mod filter;
pub mod render;

use std::collections::HashMap;

use base64::{engine::general_purpose::STANDARD as B64, Engine};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use stellar_lane_core::{
    blob::AnchorWitness, hash::sha256, results::soroban_successes, run_anchor, MAX_LEDGER_VERSION,
    MIN_LEDGER_VERSION, NETWORK_ID,
};
use stellar_xdr::{
    GeneralizedTransactionSet, InnerTransactionResultResult, InvokeHostFunctionResult,
    InvokeHostFunctionSuccessPreImage, LedgerHeader, LedgerHeaderHistoryEntry,
    LedgerHeaderHistoryEntryExt, Limits, OperationResult, OperationResultTr, ReadXdr,
    TransactionEnvelope, TransactionPhase, TransactionResult, TransactionResultPair,
    TransactionResultResult, TransactionResultSet, TxSetComponent, WriteXdr,
};

pub use stellar_lane_core::PUBNET_PASSPHRASE;



pub const PINNED_QUORUM_SET: &[u8] = include_bytes!("../pins/tier1-qset.xdr");

pub const MAX_LEDGERS: u32 = 64;



const DEPTH: u32 = 256;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LedgerProof {
    pub sequence: u32,

    pub header: String,

    pub results: String,

    pub preimages: Vec<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tx_set: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AnchorProof {

    pub ledger: u32,

    pub tx_set_next: String,

    pub envelopes: Vec<String>,

    pub quorum_set: String,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LedgerBundle {
    pub network: String,
    pub start_ledger: u32,
    pub end_ledger: u32,
    pub ledgers: Vec<LedgerProof>,
    pub anchor: Option<AnchorProof>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pending: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum VerifyError {

    Pending(String),

    Invalid(String),
}

impl std::fmt::Display for VerifyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VerifyError::Pending(m) => write!(f, "not anchored yet: {m}"),
            VerifyError::Invalid(m) => write!(f, "does not verify: {m}"),
        }
    }
}

impl std::error::Error for VerifyError {}

fn bad(m: impl Into<String>) -> VerifyError {
    VerifyError::Invalid(m.into())
}

fn unb64(s: &str, what: &str) -> Result<Vec<u8>, VerifyError> {
    B64.decode(s)
        .map_err(|_| bad(format!("{what} is not base64")))
}

fn xdr<T: ReadXdr>(bytes: &[u8], what: &str) -> Result<T, VerifyError> {
    T::from_xdr(bytes, Limits::depth(DEPTH)).map_err(|_| bad(format!("{what} is not XDR")))
}


#[derive(Clone, Debug)]
pub struct Invocation {

    pub tx: u32,
    pub op: u32,
    pub preimage: InvokeHostFunctionSuccessPreImage,
}

#[derive(Clone, Debug)]
pub struct VerifiedLedger {
    pub sequence: u32,
    pub hash: [u8; 32],
    pub header: LedgerHeader,
    pub header_xdr: Vec<u8>,
    pub results: Vec<TransactionResultPair>,
    pub invocations: Vec<Invocation>,

    pub envelopes: Option<Vec<TransactionEnvelope>>,
}

impl VerifiedLedger {
    pub fn close_time(&self) -> u64 {
        self.header.scp_value.close_time.0
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AnchorInfo {
    pub ledger: u32,
    pub ledger_hash: [u8; 32],
    pub quorum_set_hash: [u8; 32],
    pub next_quorum_set_hash: [u8; 32],
    pub signed_entries: u64,
    pub total_entries: u64,
    pub signers: usize,
    pub source: Option<String>,
}

#[derive(Clone, Debug)]
pub struct Verified {
    pub ledgers: Vec<VerifiedLedger>,
    pub anchor: AnchorInfo,
}


pub fn successful_ops(r: &TransactionResult) -> Option<&[OperationResult]> {
    match &r.result {
        TransactionResultResult::TxSuccess(ops) => Some(ops.as_slice()),
        TransactionResultResult::TxFeeBumpInnerSuccess(inner) => match &inner.result.result {
            InnerTransactionResultResult::TxSuccess(ops) => Some(ops.as_slice()),
            _ => None,
        },
        _ => None,
    }
}


pub fn invocations_of(results: &[TransactionResultPair]) -> Vec<(u32, u32, [u8; 32])> {
    let mut out = Vec::new();
    for (i, p) in results.iter().enumerate() {
        for (j, op) in successful_ops(&p.result).unwrap_or(&[]).iter().enumerate() {
            if let OperationResult::OpInner(OperationResultTr::InvokeHostFunction(
                InvokeHostFunctionResult::Success(h),
            )) = op
            {
                out.push((i as u32, j as u32, h.0));
            }
        }
    }
    out
}


pub fn tx_set_envelopes(ts: &GeneralizedTransactionSet) -> Vec<&TransactionEnvelope> {
    let GeneralizedTransactionSet::V1(v1) = ts;
    let mut out = Vec::new();
    for phase in v1.phases.iter() {
        match phase {
            TransactionPhase::V0(components) => {
                for c in components.iter() {
                    let TxSetComponent::TxsetCompTxsMaybeDiscountedFee(c) = c;
                    out.extend(c.txs.iter());
                }
            }
            TransactionPhase::V1(p) => {
                for stage in p.execution_stages.iter() {
                    for cluster in stage.0.iter() {
                        out.extend(cluster.0.iter());
                    }
                }
            }
        }
    }
    out
}


pub fn envelopes_in_apply_order(
    ts: &GeneralizedTransactionSet,
    results: &[TransactionResultPair],
) -> Result<Vec<TransactionEnvelope>, VerifyError> {
    let all = tx_set_envelopes(ts);
    if all.len() != results.len() {
        return Err(bad(format!(
            "the transaction set has {} transactions, the results {}",
            all.len(),
            results.len()
        )));
    }
    let mut by_hash: HashMap<[u8; 32], &TransactionEnvelope> = HashMap::with_capacity(all.len());
    for e in all {
        let h = e
            .hash(NETWORK_ID)
            .map_err(|_| bad("a transaction does not encode"))?;
        if by_hash.insert(h, e).is_some() {
            return Err(bad("a transaction appears twice in the set"));
        }
    }
    results
        .iter()
        .map(|p| {
            by_hash
                .remove(&p.transaction_hash.0)
                .cloned()
                .ok_or_else(|| {
                    bad(format!(
                        "result {} has no transaction in the set",
                        hex::encode(p.transaction_hash.0)
                    ))
                })
        })
        .collect()
}



pub fn verify_bundle(
    b: &LedgerBundle,
    trusted_qset: Option<&[u8]>,
) -> Result<Verified, VerifyError> {
    if b.network != PUBNET_PASSPHRASE {
        return Err(bad(format!(
            "network {:?} is not Stellar pubnet",
            b.network
        )));
    }
    if b.start_ledger > b.end_ledger || b.end_ledger - b.start_ledger >= MAX_LEDGERS {
        return Err(bad(format!(
            "range {}..={} is empty or over {MAX_LEDGERS} ledgers",
            b.start_ledger, b.end_ledger
        )));
    }
    if b.ledgers.len() as u32 != b.end_ledger - b.start_ledger + 1 {
        return Err(bad("the bundle does not carry every ledger of its range"));
    }
    let anchor = match &b.anchor {
        Some(a) => a,
        None => {
            return Err(VerifyError::Pending(
                b.pending.clone().unwrap_or_else(|| "no anchor".into()),
            ))
        }
    };
    if anchor.ledger != b.end_ledger {
        return Err(bad("the anchor is not for the bundle's last ledger"));
    }

    let mut ledgers = Vec::with_capacity(b.ledgers.len());
    let mut prev: Option<[u8; 32]> = None;
    for (i, lp) in b.ledgers.iter().enumerate() {
        let seq = b.start_ledger + i as u32;
        if lp.sequence != seq {
            return Err(bad(format!("ledger {i} is {} not {seq}", lp.sequence)));
        }
        let header_xdr = unb64(&lp.header, "header")?;
        let hash = sha256(&header_xdr);
        let header: LedgerHeader = xdr(&header_xdr, "header")?;
        if header.ledger_seq != seq {
            return Err(bad(format!(
                "header of ledger {seq} names {}",
                header.ledger_seq
            )));
        }
        if !(MIN_LEDGER_VERSION..=MAX_LEDGER_VERSION).contains(&header.ledger_version) {
            return Err(bad(format!(
                "ledger {seq} is protocol {}; this verifier checks {MIN_LEDGER_VERSION} to {MAX_LEDGER_VERSION}",
                header.ledger_version
            )));
        }
        if let Some(p) = prev {
            if header.previous_ledger_hash.0 != p {
                return Err(bad(format!(
                    "ledger {seq} does not follow ledger {}",
                    seq - 1
                )));
            }
        }

        let results_xdr = unb64(&lp.results, "results")?;
        if sha256(&results_xdr) != header.tx_set_result_hash.0 {
            return Err(bad(format!(
                "ledger {seq}: the results do not hash to txSetResultHash"
            )));
        }

        let walked =
            soroban_successes(&results_xdr).map_err(|e| bad(format!("ledger {seq}: {e}")))?;
        let set: TransactionResultSet = xdr(&results_xdr, "results")?;
        let results = set.results.to_vec();
        let found = invocations_of(&results);
        if walked.len() != found.len()
            || walked
                .iter()
                .zip(&found)
                .any(|(w, f)| w.tx != f.0 || w.hash != f.2)
        {
            return Err(bad(format!(
                "ledger {seq}: the result walk disagrees with the decoder"
            )));
        }
        if lp.preimages.len() != found.len() {
            return Err(bad(format!(
                "ledger {seq}: {} Soroban preimages for {} successful invocations",
                lp.preimages.len(),
                found.len()
            )));
        }
        let mut invocations = Vec::with_capacity(found.len());
        for ((tx, op, h), p) in found.into_iter().zip(&lp.preimages) {
            let bytes = unb64(p, "preimage")?;
            if sha256(&bytes) != h {
                return Err(bad(format!(
                    "ledger {seq}: transaction {} events do not hash to its result",
                    tx + 1
                )));
            }
            invocations.push(Invocation {
                tx,
                op,
                preimage: xdr(&bytes, "preimage")?,
            });
        }

        let envelopes = match &lp.tx_set {
            None => None,
            Some(s) => {
                let ts_xdr = unb64(s, "txSet")?;
                if sha256(&ts_xdr) != header.scp_value.tx_set_hash.0 {
                    return Err(bad(format!(
                        "ledger {seq}: the transaction set does not hash to txSetHash"
                    )));
                }
                let ts: GeneralizedTransactionSet = xdr(&ts_xdr, "txSet")?;
                let GeneralizedTransactionSet::V1(v1) = &ts;
                if v1.previous_ledger_hash.0 != header.previous_ledger_hash.0 {
                    return Err(bad(format!(
                        "ledger {seq}: the transaction set names another parent"
                    )));
                }
                Some(envelopes_in_apply_order(&ts, &results)?)
            }
        };
        prev = Some(hash);
        ledgers.push(VerifiedLedger {
            sequence: seq,
            hash,
            header,
            header_xdr,
            results,
            invocations,
            envelopes,
        });
    }

    let tx_set_next = unb64(&anchor.tx_set_next, "txSetNext")?;
    let declared = unb64(&anchor.quorum_set, "quorumSet")?;
    let qset = trusted_qset.unwrap_or(PINNED_QUORUM_SET).to_vec();
    let next_qset = if sha256(&declared) == sha256(&qset) {
        Vec::new()
    } else {
        declared
    };
    let envelopes = anchor
        .envelopes
        .iter()
        .map(|e| unb64(e, "envelope"))
        .collect::<Result<Vec<_>, _>>()?;
    let last = ledgers.last().expect("one ledger at least");
    let w = AnchorWitness {
        network_id: NETWORK_ID,
        ledger: anchor.ledger,
        qset: qset.clone(),
        next_qset,
        header: last.header_xdr.clone(),
        tx_set_next,
        envelopes,
    };
    let out = run_anchor(&w.encode())
        .map_err(|e| bad(format!("SCP anchor for ledger {}: {e}", anchor.ledger)))?;
    if out.ledger_hash != last.hash || out.qset_hash != sha256(&qset) {
        return Err(bad("the anchor binds another ledger or quorum set"));
    }
    Ok(Verified {
        anchor: AnchorInfo {
            ledger: out.ledger,
            ledger_hash: out.ledger_hash,
            quorum_set_hash: out.qset_hash,
            next_quorum_set_hash: out.next_qset_hash,
            signed_entries: out.signed_entries,
            total_entries: out.total_entries,
            signers: anchor.envelopes.len(),
            source: anchor.source.clone(),
        },
        ledgers,
    })
}


pub fn header_history_xdr(hash: [u8; 32], header: &LedgerHeader) -> Vec<u8> {
    LedgerHeaderHistoryEntry {
        hash: stellar_xdr::Hash(hash),
        header: header.clone(),
        ext: LedgerHeaderHistoryEntryExt::V0,
    }
    .to_xdr(Limits::none())
    .expect("XDR of a decoded header")
}


pub fn committed_events(l: &VerifiedLedger) -> Vec<Value> {
    let mut out = Vec::new();
    for inv in &l.invocations {
        let tx_hash = l.results[inv.tx as usize].transaction_hash.0;
        for (k, ev) in inv.preimage.events.iter().enumerate() {
            let at = render::EventAt {
                ledger: l.sequence,
                tx: inv.tx + 1,
                op: inv.op,
                index: k as u32,
            };
            out.push(render::event_json(at, l.close_time(), &tx_hash, true, ev));
        }
    }
    out
}



pub fn proven_transactions(l: &VerifiedLedger) -> Vec<Value> {
    let ret: HashMap<u32, String> = l
        .invocations
        .iter()
        .map(|i| (i.tx, render::b64_xdr(&i.preimage.return_value)))
        .collect();
    l.results
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let mut t = json!({
                "status": if render::tx_successful(&p.result) { "SUCCESS" } else { "FAILED" },
                "txHash": hex::encode(p.transaction_hash.0),
                "applicationOrder": i + 1,
                "feeBump": match &l.envelopes {
                    Some(envs) => matches!(envs[i], TransactionEnvelope::TxFeeBump(_)),
                    None => matches!(p.result.result, TransactionResultResult::TxFeeBumpInnerSuccess(_) | TransactionResultResult::TxFeeBumpInnerFailed(_)),
                },
                "resultXdr": render::b64_xdr(&p.result),
                "ledger": l.sequence,
                "createdAt": l.close_time().to_string(),
            });
            if let Some(envs) = &l.envelopes {
                t["envelopeXdr"] = json!(render::b64_xdr(&envs[i]));
            }
            if let Some(r) = ret.get(&(i as u32)) {
                t["returnValueXdr"] = json!(r);
            }
            t
        })
        .collect()
}















pub fn verified_json(v: &Verified) -> Value {
    let a = &v.anchor;
    json!({
        "network": PUBNET_PASSPHRASE,
        "anchor": {
            "ledger": a.ledger,
            "hash": hex::encode(a.ledger_hash),
            "quorumSetHash": hex::encode(a.quorum_set_hash),
            "nextQuorumSetHash": hex::encode(a.next_quorum_set_hash),
            "signedEntries": a.signed_entries,
            "totalEntries": a.total_entries,
            "signers": a.signers,
            "source": a.source,
        },
        "ledgers": v.ledgers.iter().map(|l| json!({
            "sequence": l.sequence,
            "hash": hex::encode(l.hash),
            "previousHash": hex::encode(l.header.previous_ledger_hash.0),
            "closeTime": l.close_time().to_string(),
            "protocolVersion": l.header.ledger_version,
            "headerXdr": B64.encode(&l.header_xdr),
            "headerHistoryXdr": B64.encode(header_history_xdr(l.hash, &l.header)),
            "transactionCount": l.results.len(),
            "transactions": proven_transactions(l),
            "events": committed_events(l),
        })).collect::<Vec<_>>(),
    })
}



