//! The browser verifier's block confirmation (`web/verify/TRUSTLESS-ANCHOR.md`): whether a block
//! hash a proof ended at is block N of Ethereum, verified in the page's Web Worker against
//! Ethereum's validators alone.
//!
//! - [`lc`]: the light client, from the checkpoint the page pins: bootstrap, one update per
//!   period, finality updates.
//! - [`history`]: a block's hash from EIP-2935's history under a verified execution state root.
//! - `beacon`: older blocks through the beacon state (`window_anchor_core::run`, the witness the
//!   SP1 window-anchor guest checks), under a header the light client verified as final.
//!
//! As in the other two verifiers, no method throws a verdict: each returns JSON with `ok` and,
//! when it is false, a `kind` the page decides by, never the message. Host and wasm32, with no
//! wasm-bindgen: `anchor-wasm` is the page's face of [`Anchor`], and `willow-eth-anchor` checks
//! what it serves with [`confirm`].

mod bls;
pub mod call;
pub mod history;
pub mod l2;
pub mod lc;
pub mod signed;

use alloy_primitives::B256;
use lc::{Finality, LightClient};
use serde_json::{json, Value};

/// Why a check did not give a verdict, or what it needs first.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// Data failed verification: a signature, a branch, a proof, or the wrong block.
    Rejected,
    /// An answer could not be read.
    MalformedInput,
    /// Ethereum's data is from a fork this page does not know.
    UnknownFork,
    /// The page's starting point is too old to start from.
    StaleCheckpoint,
    /// The data cannot be followed from this page's starting point (signed before it, or an
    /// update that hands over no committee): not a verdict on the block.
    Unreachable,
    /// The signed head is too old to say anything about a block that is not final yet.
    StaleHead,
    /// An L2 claim this page does not count on (not by the pinned proposer, blacklisted, retired,
    /// paused, not respected, lost): not a verdict on the block.
    Declined,
    /// An L2 claim another claim on Ethereum contests: not a verdict on the block.
    Disputed,
    /// Willow's data does not reach the block (a claim or signed block before it, links left
    /// out): not a verdict on the block.
    NotCovered,
    /// The light client needs the checkpoint's bootstrap first (not a verdict).
    NeedBootstrap,
    /// The light client needs the updates of these periods first (not a verdict).
    NeedUpdates,
    /// Used out of order: a fault in the page, never caused by an answer.
    Internal,
}

impl Kind {
    pub fn as_str(self) -> &'static str {
        match self {
            Kind::Rejected => "rejected",
            Kind::MalformedInput => "malformed-input",
            Kind::UnknownFork => "unknown-fork",
            Kind::StaleCheckpoint => "stale-checkpoint",
            Kind::Unreachable => "unreachable",
            Kind::StaleHead => "stale-head",
            Kind::Declined => "declined",
            Kind::Disputed => "disputed",
            Kind::NotCovered => "not-covered",
            Kind::NeedBootstrap => "need-bootstrap",
            Kind::NeedUpdates => "need-updates",
            Kind::Internal => "internal",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Failure {
    pub kind: Kind,
    pub message: String,
    /// Fields the page reads with the kind (the periods a `need-updates` names).
    pub extra: Option<Value>,
}

impl Failure {
    pub fn new(kind: Kind, message: impl Into<String>) -> Self {
        Failure {
            kind,
            message: message.into(),
            extra: None,
        }
    }

    pub fn with(mut self, extra: Value) -> Self {
        self.extra = Some(extra);
        self
    }

    /// `{"ok": false, "kind", "message", ...extra}`, as the page receives it.
    pub fn json(&self) -> Value {
        let mut v = json!({"ok": false, "kind": self.kind.as_str(), "message": self.message});
        if let (Some(Value::Object(e)), Value::Object(m)) = (&self.extra, &mut v) {
            for (k, x) in e {
                m.insert(k.clone(), x.clone());
            }
        }
        v
    }
}

/// The `result` of a JSON-RPC response body as the server sent it. Unreadable, or nested past
/// serde_json's 128 levels: `malformed-input`.
pub fn result_of(body: &str, method: &str) -> Result<Value, Failure> {
    let v: Value = serde_json::from_str(body)
        .map_err(|e| Failure::new(Kind::MalformedInput, format!("{method} answer: {e}")))?;
    v.get("result").cloned().ok_or_else(|| {
        Failure::new(
            Kind::MalformedInput,
            format!("{method} answer has no result"),
        )
    })
}

/// What the anchor showed about block `number`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Confirmed {
    /// `final` (Ethereum finalized it) or `signed` (in the signed head's chain, not final yet).
    pub level: &'static str,
    /// `head`, `history` or `beacon`.
    pub path: &'static str,
    /// Block `number`'s hash on Ethereum, as proven.
    pub hash: [u8; 32],
    pub finalized: u64,
    pub head: u64,
    pub head_time: u64,
    pub signers: usize,
}

/// A signed head older than this says nothing about a block not final yet: a head the validators
/// signed and Ethereum later dropped could otherwise be shown forever.
pub const MAX_SIGNED_AGE_SECS: u64 = 600;

/// Block `number`'s hash on Ethereum from a `willow_getBlockAnchor` result, through the light
/// client's verified finality update.
pub fn confirm(
    lc: &LightClient,
    number: u64,
    answer: &Value,
    now: u64,
) -> Result<Confirmed, Failure> {
    lc.check_age(now)?;
    let asked = answer
        .get("blockNumber")
        .and_then(Value::as_str)
        .and_then(|s| s.strip_prefix("0x"))
        .and_then(|h| u64::from_str_radix(h, 16).ok())
        .ok_or_else(|| Failure::new(Kind::MalformedInput, "the anchor has no blockNumber"))?;
    if asked != number {
        return Err(Failure::new(
            Kind::Rejected,
            format!("the anchor is for block {asked}, not {number}"),
        ));
    }
    let update = answer
        .get("update")
        .ok_or_else(|| Failure::new(Kind::MalformedInput, "the anchor has no update"))?;
    let f: Finality = lc.finality(update, now)?;
    let proof = answer
        .get("proof")
        .ok_or_else(|| Failure::new(Kind::MalformedInput, "the anchor has no proof"))?;
    let path = proof.get("kind").and_then(Value::as_str).unwrap_or("");
    let head = f.attested.execution;
    let fin = f.finalized.execution;
    let (path, hash, level) = match path {
        "head" => {
            if number != head.number {
                return Err(Failure::new(
                    Kind::Rejected,
                    format!("block {number} is not the signed head {}", head.number),
                ));
            }
            (
                "head",
                head.hash,
                if number <= fin.number {
                    "final"
                } else {
                    "signed"
                },
            )
        }
        "history" => {
            let p = proof
                .get("proof")
                .ok_or_else(|| Failure::new(Kind::MalformedInput, "no history proof"))?;
            let h = history::block_hash(
                head.state_root,
                head.number,
                number,
                history::ETHEREUM_RING,
                history::ethereum_code_hash(),
                p,
            )?;
            (
                "history",
                h,
                if number <= fin.number {
                    "final"
                } else {
                    "signed"
                },
            )
        }
        "beacon" => {
            let w = proof
                .get("witness")
                .and_then(Value::as_str)
                .ok_or_else(|| Failure::new(Kind::MalformedInput, "no beacon witness"))?;
            let blob = lc::bytes_of(w, "witness")?;
            let s = window_anchor_core::run(&blob)
                .map_err(|e| Failure::new(Kind::Rejected, format!("the beacon proof: {e:?}")))?;
            if s.anchor_beacon_root != f.finalized.beacon.root()
                || s.anchor_slot != f.finalized.beacon.slot
            {
                return Err(Failure::new(
                    Kind::Rejected,
                    "the beacon proof does not end at the finalized header",
                ));
            }
            if s.el_block_number != number {
                return Err(Failure::new(
                    Kind::Rejected,
                    format!(
                        "the beacon proof is for block {}, not {number}",
                        s.el_block_number
                    ),
                ));
            }
            ("beacon", s.el_block_hash, "final")
        }
        _ => {
            return Err(Failure::new(
                Kind::MalformedInput,
                "the anchor's proof is of no kind this page knows",
            ))
        }
    };
    if level == "signed" && now.saturating_sub(head.timestamp) > MAX_SIGNED_AGE_SECS {
        return Err(Failure::new(
            Kind::StaleHead,
            format!(
                "Ethereum's newest signed block that Willow sent is {} minutes old",
                now.saturating_sub(head.timestamp) / 60
            ),
        ));
    }
    Ok(Confirmed {
        level,
        path,
        hash,
        finalized: fin.number,
        head: head.number,
        head_time: head.timestamp,
        signers: f.signers,
    })
}

fn hash_arg(s: &str) -> Option<[u8; 32]> {
    let b = hex::decode(s.strip_prefix("0x")?).ok()?;
    b.try_into().ok()
}

/// The light client and its checks as JSON in, JSON out: `Anchor::new(checkpointRoot,
/// checkpointSlot)`, then `restore`, `bootstrap`, `updates` and `verify` as the page fetches what
/// `verify` says it needs.
pub struct Anchor {
    lc: Option<LightClient>,
}

fn out(r: Result<Value, Failure>) -> String {
    match r {
        Ok(v) => v.to_string(),
        Err(f) => f.json().to_string(),
    }
}

impl Anchor {
    pub fn new(checkpoint_root: &str, checkpoint_slot: f64) -> Anchor {
        let slot = (checkpoint_slot.fract() == 0.0 && checkpoint_slot >= 0.0)
            .then_some(checkpoint_slot as u64);
        Anchor {
            lc: hash_arg(checkpoint_root)
                .zip(slot)
                .map(|(r, s)| LightClient::new(r, s)),
        }
    }

    fn client(&mut self) -> Result<&mut LightClient, Failure> {
        self.lc.as_mut().ok_or_else(|| {
            Failure::new(Kind::Internal, "the page's checkpoint is not a block root")
        })
    }

    /// The light-client chain kept from an earlier visit (`saved()`'s JSON), verified again:
    /// `{"ok": true, "period"}`.
    pub fn restore(&mut self, saved: &str, now: f64) -> String {
        out((|| {
            let v: Value = serde_json::from_str(saved).map_err(|e| {
                Failure::new(Kind::MalformedInput, format!("the kept light client: {e}"))
            })?;
            let lc = self.client()?;
            let p = lc.restore(&v, now as u64).inspect_err(|_| lc.reset())?;
            Ok(json!({"ok": true, "period": p}))
        })())
    }

    /// The checkpoint's `willow_lightClientBootstrap` response body.
    pub fn bootstrap(&mut self, response: &str, now: f64) -> String {
        out((|| {
            let v = result_of(response, "willow_lightClientBootstrap")?;
            let lc = self.client()?;
            lc.check_age(now as u64)?;
            let p = lc.bootstrap(&v)?;
            Ok(json!({"ok": true, "period": p}))
        })())
    }

    /// A `willow_lightClientUpdates` response body: each update verified and applied in order.
    pub fn updates(&mut self, response: &str) -> String {
        out((|| {
            let v = result_of(response, "willow_lightClientUpdates")?;
            let list = v
                .as_array()
                .filter(|l| l.len() <= lc::MAX_UPDATES)
                .ok_or_else(|| {
                    Failure::new(Kind::MalformedInput, "updates are not a list of at most 16")
                })?;
            let lc = self.client()?;
            let mut newest = None;
            for u in list {
                newest = lc.apply_update(u)?.or(newest);
            }
            Ok(json!({"ok": true, "period": newest}))
        })())
    }

    /// Whether `hash` is block `number` of chain `chain_id`, from its `willow_getBlockAnchor`
    /// response body: `{"ok": true, "level", "path", "hash", "matches", ...}`, or `need-*` for
    /// what to fetch first, or a failure.
    pub fn verify(
        &mut self,
        chain_id: u32,
        number: f64,
        hash: &str,
        response: &str,
        now: f64,
    ) -> String {
        out((|| {
            if !(number.fract() == 0.0 && (0.0..9.0e15).contains(&number)) {
                return Err(Failure::new(
                    Kind::Internal,
                    "the block number is not an integer",
                ));
            }
            let want = hash_arg(hash)
                .ok_or_else(|| Failure::new(Kind::Internal, "the block hash is not 32 bytes"))?;
            let v = result_of(response, "willow_getBlockAnchor")?;
            if chain_id != 1 && v.get("signed").is_some() {
                let c = signed::confirm_signed(
                    self.client()?,
                    chain_id.into(),
                    number as u64,
                    &v,
                    now as u64,
                )?;
                return Ok(json!({
                    "ok": true,
                    "level": "signed",
                    "path": "sequencer",
                    "hash": format!("{}", B256::from(c.hash)),
                    "matches": c.hash == want,
                    "anchorBlock": c.anchor_block,
                    "signer": format!("{}", c.signer),
                    "l1Block": c.l1_block,
                }));
            }
            if l2::NITRO_CHAINS
                .iter()
                .any(|c| c.chain_id == u64::from(chain_id))
            {
                let c = l2::confirm_nitro(
                    self.client()?,
                    chain_id.into(),
                    number as u64,
                    &v,
                    now as u64,
                )?;
                return Ok(json!({
                    "ok": true,
                    "level": c.level,
                    "path": "assertion",
                    "hash": format!("{}", B256::from(c.hash)),
                    "matches": c.hash == want,
                    "anchorBlock": c.anchor_block,
                    "assertion": format!("{}", B256::from(c.assertion)),
                    "l1Block": c.l1_block,
                }));
            }
            if chain_id != 1 {
                let c = l2::confirm_op(
                    self.client()?,
                    chain_id.into(),
                    number as u64,
                    &v,
                    now as u64,
                )?;
                return Ok(json!({
                    "ok": true,
                    "level": c.level,
                    "path": "game",
                    "hash": format!("{}", B256::from(c.hash)),
                    "matches": c.hash == want,
                    "anchorBlock": c.anchor_block,
                    "game": format!("{}", c.game),
                    "l1Block": c.l1_block,
                }));
            }
            let c = confirm(self.client()?, number as u64, &v, now as u64)?;
            Ok(json!({
                "ok": true,
                "level": c.level,
                "path": c.path,
                "hash": format!("{}", B256::from(c.hash)),
                "matches": c.hash == want,
                "finalized": c.finalized,
                "head": c.head,
                "headTime": c.head_time,
                "signers": c.signers,
            }))
        })())
    }

    /// The newest committees to keep for the next visit (JSON), or `[]`.
    pub fn saved(&self) -> String {
        self.lc
            .as_ref()
            .map_or_else(|| "null".into(), |lc| lc.saved().to_string())
    }

    /// Forgets the committees, after a `need-bootstrap` for an older one.
    pub fn reset(&mut self) {
        if let Some(lc) = self.lc.as_mut() {
            lc.reset();
        }
    }
}

#[cfg(test)]
mod tests;
