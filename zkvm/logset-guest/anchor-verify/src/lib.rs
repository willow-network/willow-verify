

















pub mod bellatrix;
pub mod berachain;
mod bls;
pub mod bnb;
pub mod call;
pub mod history;
pub mod l2;
pub mod lc;
pub mod orbit;
pub mod polygon;
pub mod premerge;
pub mod signed;

use alloy_primitives::B256;
use lc::{Finality, LightClient};
use serde_json::{json, Value};


#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {

    Rejected,

    MalformedInput,

    UnknownFork,

    StaleCheckpoint,


    Unreachable,

    StaleHead,


    Declined,

    Disputed,


    NotCovered,

    NeedBootstrap,

    NeedUpdates,

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



pub fn result_of(body: &str, method: &str) -> Result<Value, Failure> {
    let v: Value = serde_json::from_str(body)
        .map_err(|e| Failure::new(Kind::MalformedInput, format!("{method} response: {e}")))?;
    v.get("result").cloned().ok_or_else(|| {
        Failure::new(
            Kind::MalformedInput,
            format!("{method} response has no result"),
        )
    })
}


#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Confirmed {

    pub level: &'static str,

    pub path: &'static str,

    pub hash: [u8; 32],
    pub finalized: u64,
    pub head: u64,
    pub head_time: u64,
    pub signers: usize,

    pub total_difficulty: Option<alloy_primitives::U256>,
}



pub const MAX_SIGNED_AGE_SECS: u64 = 600;


fn asked_block(number: u64, answer: &Value) -> Result<(), Failure> {
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
    Ok(())
}



pub fn is_premerge(number: u64, answer: &Value) -> bool {
    number < premerge::MERGE_BLOCK
        || answer
            .get("proof")
            .and_then(|p| p.get("kind"))
            .and_then(Value::as_str)
            == Some("accumulator")
}



pub fn confirm_premerge(number: u64, answer: &Value) -> Result<Confirmed, Failure> {
    asked_block(number, answer)?;
    let proof = answer
        .get("proof")
        .ok_or_else(|| Failure::new(Kind::MalformedInput, "the anchor has no proof"))?;
    if proof.get("kind").and_then(Value::as_str) != Some("accumulator") {
        return Err(Failure::new(
            Kind::Rejected,
            format!("block {number} is before the merge: only the accumulator proves it"),
        ));
    }
    let (hash, td) = premerge::verify(number, proof)?;
    Ok(Confirmed {
        level: "final",
        path: "accumulator",
        hash,
        finalized: 0,
        head: 0,
        head_time: 0,
        signers: 0,
        total_difficulty: Some(td),
    })
}



pub fn confirm(
    lc: &LightClient,
    number: u64,
    answer: &Value,
    now: u64,
) -> Result<Confirmed, Failure> {
    if is_premerge(number, answer) {
        return confirm_premerge(number, answer);
    }
    confirm_signed_chain(lc, number, answer, now)
}




pub fn confirm_sepolia(
    lc: &LightClient,
    number: u64,
    answer: &Value,
    now: u64,
) -> Result<Confirmed, Failure> {
    if lc.net != lc::Network::Sepolia {
        return Err(Failure::new(
            Kind::Internal,
            "Sepolia's blocks are checked with Sepolia's light client",
        ));
    }
    confirm_signed_chain(lc, number, answer, now)
}


fn confirm_signed_chain(
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
        "hops" if lc.net == lc::Network::Sepolia => {
            let list = |k: &str| {
                proof
                    .get(k)
                    .and_then(Value::as_array)
                    .ok_or_else(|| Failure::new(Kind::MalformedInput, format!("no {k} list")))
            };
            let h = history::hops(
                head.state_root,
                head.number,
                number,
                list("headers")?,
                list("proofs")?,
            )?;
            (
                "hops",
                h,
                if number <= fin.number {
                    "final"
                } else {
                    "signed"
                },
            )
        }
        "beacon" if lc.net == lc::Network::Mainnet => {
            let w = proof
                .get("witness")
                .and_then(Value::as_str)
                .ok_or_else(|| Failure::new(Kind::MalformedInput, "no beacon witness"))?;
            let blob = lc::bytes_of(w, "witness")?;


            let s = if bellatrix::is_historical_roots(&blob) {
                bellatrix::run(&blob)
                    .map_err(|e| Failure::new(Kind::Rejected, format!("the beacon proof: {e:?}")))?
            } else {
                window_anchor_core::run(&blob)
                    .map_err(|e| Failure::new(Kind::Rejected, format!("the beacon proof: {e:?}")))?
            };
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
                "{}'s newest signed block that Willow sent is {} minutes old",
                lc.net.name(),
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
        total_difficulty: None,
    })
}

fn hash_arg(s: &str) -> Option<[u8; 32]> {
    let b = hex::decode(s.strip_prefix("0x")?).ok()?;
    b.try_into().ok()
}






pub struct Anchor {
    lc: Option<LightClient>,
    sepolia: LightClient,
    bnb: bnb::Bnb,
    berachain: berachain::Berachain,
}


#[derive(Clone, Copy, PartialEq, Eq)]
enum Other {
    Sepolia,
    Bnb,
    Berachain,
}

fn other(name: &str) -> Result<Other, Failure> {
    match name {
        "sepolia" => Ok(Other::Sepolia),
        "bnb" => Ok(Other::Bnb),
        "berachain" => Ok(Other::Berachain),
        _ => Err(Failure::new(
            Kind::Internal,
            format!("no light client {name:?} here"),
        )),
    }
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
            sepolia: LightClient::on(
                lc::Network::Sepolia,
                lc::sepolia::PINNED_ROOT,
                lc::sepolia::PINNED_SLOT,
            ),
            bnb: bnb::Bnb::default(),
            berachain: berachain::Berachain::default(),
        }
    }




    pub fn start(&mut self, name: &str, start: &str) -> String {
        out((|| {
            let v: Value = serde_json::from_str(start)
                .map_err(|e| Failure::new(Kind::Internal, format!("the start: {e}")))?;
            match other(name)? {
                Other::Sepolia => {
                    let root = v.get("root").and_then(Value::as_str).and_then(hash_arg);
                    let slot = v.get("slot").and_then(Value::as_u64);
                    let (Some(root), Some(slot)) = (root, slot) else {
                        return Err(Failure::new(
                            Kind::Internal,
                            "a Sepolia start is {root, slot} of a finalized block",
                        ));
                    };
                    self.sepolia = LightClient::on(lc::Network::Sepolia, root, slot);
                }
                Other::Bnb => {
                    let s = bnb::start_of(&v).ok_or_else(|| {
                        Failure::new(
                            Kind::Internal,
                            "a BNB start is {number, hash} of an epoch block",
                        )
                    })?;
                    self.bnb = bnb::Bnb::new(s);
                }
                Other::Berachain => {
                    let s = berachain::start_of(&v).ok_or_else(|| {
                        Failure::new(
                            Kind::Internal,
                            "a Berachain start is {height, hash} of a CometBFT header",
                        )
                    })?;
                    self.berachain = berachain::Berachain::new(s);
                }
            }
            Ok(json!({"ok": true}))
        })())
    }


    pub fn bootstrap_for(&mut self, name: &str, response: &str, now: f64) -> String {
        out((|| {
            let v = result_of(response, "willow_lightClientBootstrap")?;
            let p = match other(name)? {
                Other::Sepolia => {
                    self.sepolia.check_age(now as u64)?;
                    self.sepolia.bootstrap(&v)?
                }
                Other::Bnb => self.bnb.bootstrap(&v, now as u64)?,
                Other::Berachain => self.berachain.bootstrap(&v, now as u64)?,
            };
            Ok(json!({"ok": true, "period": p}))
        })())
    }


    pub fn updates_for(&mut self, name: &str, response: &str) -> String {
        out((|| {
            let v = result_of(response, "willow_lightClientUpdates")?;
            let o = other(name)?;
            let max = match o {
                Other::Sepolia => lc::MAX_UPDATES,
                Other::Bnb => bnb::MAX_UPDATES,
                Other::Berachain => berachain::MAX_UPDATES,
            };
            let list = v.as_array().filter(|l| l.len() <= max).ok_or_else(|| {
                Failure::new(
                    Kind::MalformedInput,
                    format!("updates are not a list of at most {max}"),
                )
            })?;
            let mut newest = None;
            for u in list {
                newest = match o {
                    Other::Sepolia => self.sepolia.apply_update(u)?,
                    Other::Bnb => self.bnb.apply_update(u)?,
                    Other::Berachain => self.berachain.apply_update(u)?,
                }
                .or(newest);
            }
            Ok(json!({"ok": true, "period": newest}))
        })())
    }


    pub fn restore_for(&mut self, name: &str, saved: &str, now: f64) -> String {
        out((|| {
            let v: Value = serde_json::from_str(saved).map_err(|e| {
                Failure::new(Kind::MalformedInput, format!("the kept light client: {e}"))
            })?;
            let p = match other(name)? {
                Other::Sepolia => self
                    .sepolia
                    .restore(&v, now as u64)
                    .inspect_err(|_| self.sepolia.reset())?,
                Other::Bnb => self
                    .bnb
                    .restore(&v, now as u64)
                    .inspect_err(|_| self.bnb.reset())?,
                Other::Berachain => self
                    .berachain
                    .restore(&v, now as u64)
                    .inspect_err(|_| self.berachain.reset())?,
            };
            Ok(json!({"ok": true, "period": p}))
        })())
    }


    pub fn saved_for(&self, name: &str) -> String {
        match other(name) {
            Ok(Other::Sepolia) => self.sepolia.saved().to_string(),
            Ok(Other::Bnb) => self.bnb.saved().to_string(),
            Ok(Other::Berachain) => self.berachain.saved().to_string(),
            Err(_) => "null".into(),
        }
    }


    pub fn reset_for(&mut self, name: &str) {
        match other(name) {
            Ok(Other::Sepolia) => self.sepolia.reset(),
            Ok(Other::Bnb) => self.bnb.reset(),
            Ok(Other::Berachain) => self.berachain.reset(),
            Err(_) => {}
        }
    }


    fn l1_client(&mut self, chain_id: u64) -> Result<&mut LightClient, Failure> {
        if l2::on_sepolia(chain_id) {
            Ok(&mut self.sepolia)
        } else {
            self.client()
        }
    }

    fn client(&mut self) -> Result<&mut LightClient, Failure> {
        self.lc.as_mut().ok_or_else(|| {
            Failure::new(Kind::Internal, "the page's checkpoint is not a block root")
        })
    }



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


    pub fn bootstrap(&mut self, response: &str, now: f64) -> String {
        out((|| {
            let v = result_of(response, "willow_lightClientBootstrap")?;
            let lc = self.client()?;
            lc.check_age(now as u64)?;
            let p = lc.bootstrap(&v)?;
            Ok(json!({"ok": true, "period": p}))
        })())
    }


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
            if u64::from(chain_id) == bnb::CHAIN_ID {
                let c = self.bnb.confirm(number as u64, &v, now as u64)?;
                return Ok(json!({
                    "ok": true,
                    "level": c.level,
                    "path": "parlia",
                    "hash": format!("{}", B256::from(c.hash)),
                    "matches": c.hash == want,
                    "anchorBlock": c.anchor_block,
                    "anchorTime": c.time,
                    "signers": c.signers,
                    "of": c.of,
                }));
            }
            if u64::from(chain_id) == berachain::CHAIN_ID {
                let c = self.berachain.confirm(number as u64, &v, now as u64)?;
                return Ok(json!({
                    "ok": true,
                    "level": c.level,
                    "path": "cometbft",
                    "hash": format!("{}", B256::from(c.hash)),
                    "matches": c.hash == want,
                    "anchorBlock": c.height,
                    "anchorTime": c.time,
                    "signers": c.signers,
                    "of": c.of,
                }));
            }
            if u64::from(chain_id) == polygon::CHAIN_ID {

                let c = polygon::confirm_checkpoint(self.client()?, number as u64, &v, now as u64)?;
                return Ok(json!({
                    "ok": true,
                    "level": c.level,
                    "path": "checkpoint",
                    "covers": "roots",
                    "headerHash": format!("{}", B256::from(c.header_hash)),
                    "rootsMatch": c.header_hash == want,
                    "transactionsRoot": format!("{}", B256::from(c.transactions_root)),
                    "receiptsRoot": format!("{}", B256::from(c.receipts_root)),
                    "blockTime": c.timestamp,
                    "checkpoint": c.checkpoint,
                    "checkpointStart": c.start,
                    "checkpointEnd": c.end,
                    "l1Block": c.l1_block,
                }));
            }
            if u64::from(chain_id) == lc::sepolia::CHAIN_ID {
                let c = confirm_sepolia(&self.sepolia, number as u64, &v, now as u64)?;
                return Ok(json!({
                    "ok": true,
                    "level": c.level,
                    "path": c.path,
                    "hash": format!("{}", B256::from(c.hash)),
                    "matches": c.hash == want,
                    "finalized": c.finalized,
                    "head": c.head,
                    "headTime": c.head_time,
                    "signers": c.signers,
                }));
            }

            if orbit::orbit_chain(chain_id.into()).is_some() {
                let c = orbit::confirm_orbit(
                    self.client()?,
                    chain_id.into(),
                    number as u64,
                    &v,
                    now as u64,
                )?;
                return Ok(json!({
                    "ok": true,
                    "level": c.level,
                    "path": "node",
                    "hash": format!("{}", B256::from(c.hash)),
                    "matches": c.hash == want,
                    "anchorBlock": c.anchor_block,
                    "anchorTime": c.parent_time,
                    "node": c.node,
                    "parentBlock": c.parent_block,
                    "parentLevel": c.parent_level,
                    "l1Block": c.l1_block,
                }));
            }
            if chain_id != 1 && v.get("signed").is_some() {
                let c = signed::confirm_signed(
                    self.l1_client(chain_id.into())?,
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
                    "anchorTime": c.anchor_time,
                    "signer": format!("{}", c.signer),
                    "l1Block": c.l1_block,
                }));
            }
            if l2::NITRO_CHAINS
                .iter()
                .any(|c| c.chain_id == u64::from(chain_id))
            {
                let c = l2::confirm_nitro(
                    self.l1_client(chain_id.into())?,
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
            if let Some(o) = l2::ORACLE_CHAINS
                .iter()
                .find(|c| c.chain_id == u64::from(chain_id))
            {

                let c = if o.l1 == bnb::CHAIN_ID {
                    l2::confirm_oracle_on_bnb(
                        &self.bnb,
                        chain_id.into(),
                        number as u64,
                        &v,
                        now as u64,
                    )?
                } else {
                    l2::confirm_oracle(
                        self.l1_client(chain_id.into())?,
                        chain_id.into(),
                        number as u64,
                        &v,
                        now as u64,
                    )?
                };
                return Ok(json!({
                    "ok": true,
                    "level": c.level,
                    "path": "output",
                    "hash": format!("{}", B256::from(c.hash)),
                    "matches": c.hash == want,
                    "anchorBlock": c.anchor_block,
                    "outputIndex": c.output_index,
                    "l1Block": c.l1_block,
                }));
            }
            if chain_id != 1 {
                let c = l2::confirm_op(
                    self.l1_client(chain_id.into())?,
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
            if is_premerge(number as u64, &v) {
                let c = confirm_premerge(number as u64, &v)?;
                return Ok(json!({
                    "ok": true,
                    "level": c.level,
                    "path": c.path,
                    "hash": format!("{}", B256::from(c.hash)),
                    "matches": c.hash == want,
                    "totalDifficulty": c.total_difficulty.map(|t| format!("{t:#x}")),
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


    pub fn saved(&self) -> String {
        self.lc
            .as_ref()
            .map_or_else(|| "null".into(), |lc| lc.saved().to_string())
    }


    pub fn reset(&mut self) {
        if let Some(lc) = self.lc.as_mut() {
            lc.reset();
        }
    }
}



















mod feed;
mod nitro;
