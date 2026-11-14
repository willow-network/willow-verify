//! What every proof mode shares: how its block is anchored (Willow's light clients, a node the
//! user trusts, or not at all), how a block the user names is resolved, and the other endpoint
//! a response is compared with.

use crate::anchor::Anchoring;
use crate::rpc::{self, Transport};
use serde_json::{json, Value};

/// How proven blocks are anchored.
pub enum AnchorBy<'t> {
    /// Willow's light clients (the default): nothing trusted but the start.
    LightClient(Box<Anchoring<'t>>),
    /// `--anchor-rpc`: the block hash compared with a node the user trusts.
    Rpc(String),
    /// `--no-anchor`, or offline: the block is not anchored, and the report says so.
    None(&'static str),
}

/// A block the user named, as the proof request names it.
#[derive(Clone, Debug, PartialEq)]
pub enum Named {
    Number(u64),
    Hash(String),
    /// Resolved by the server (only without the light client).
    Latest,
}

impl Named {
    /// As a block parameter of a proof method: a quantity, a hash, or `latest`.
    pub fn param(&self) -> Value {
        match self {
            Named::Number(n) => json!(format!("0x{n:x}")),
            Named::Hash(h) => json!(h),
            Named::Latest => json!("latest"),
        }
    }

    /// As text items-wasm's `parse_block` reads.
    pub fn text(&self) -> String {
        match self {
            Named::Number(n) => n.to_string(),
            Named::Hash(h) => h.clone(),
            Named::Latest => "latest".into(),
        }
    }
}

/// A whole number: decimal, or `0x` hex.
pub fn parse_number(s: &str) -> Option<u64> {
    let s = s.trim();
    match s.strip_prefix("0x") {
        Some(h) if !h.is_empty() && h.len() <= 16 => u64::from_str_radix(h, 16).ok(),
        Some(_) => None,
        None if !s.is_empty() && s.len() <= 20 && s.bytes().all(|b| b.is_ascii_digit()) => {
            s.parse().ok()
        }
        None => None,
    }
}

/// The chains Willow's RPC serves, by name.
pub const CHAINS: [(u64, &str); 17] = [
    (1, "ethereum"),
    (8453, "base"),
    (10, "op"),
    (130, "unichain"),
    (57073, "ink"),
    (42161, "arbitrum-one"),
    (4663, "robinhood"),
    (1868, "soneium"),
    (7777777, "zora"),
    (480, "world-chain"),
    (56, "bnb"),
    (11155420, "op-sepolia"),
    (84532, "base-sepolia"),
    (1301, "unichain-sepolia"),
    (763373, "ink-sepolia"),
    (421614, "arbitrum-sepolia"),
    (46630, "robinhood-testnet"),
];

pub fn chain_name(chain_id: u64) -> Option<&'static str> {
    CHAINS.iter().find(|(c, _)| *c == chain_id).map(|(_, n)| *n)
}

/// One check's connections and choices.
pub struct Ctx<'t> {
    pub http: &'t reqwest::blocking::Client,
    pub t: &'t dyn Transport,
    pub chain_id: u64,
    pub anchor: AnchorBy<'t>,
    pub compare_rpc: Option<String>,
    pub print: bool,
    /// The head's anchor, when the user's block was the newest (folded into the report).
    pub head: Option<Value>,
    /// The anchored head's number and hash.
    pub head_block: Option<(u64, String)>,
}

impl Ctx<'_> {
    /// A proof method's response body from Willow, on the chain's route.
    pub fn willow(&self, method: &str, params: &Value) -> Result<String, String> {
        self.t.body(self.chain_id, method, params)
    }

    /// Another endpoint's response body (`--compare-rpc`).
    pub fn other(&self, method: &str, params: &Value) -> Result<String, String> {
        let url = self.compare_rpc.as_deref().ok_or("no --compare-rpc")?;
        rpc::post(self.http, url, method, params)
    }

    /// The block `s` names (a number, a hash, `latest`, `finalized` or `earliest`). With the
    /// light client, `latest` and `finalized` are anchored first and named by hash, so the proof
    /// must be of the very block anchored; otherwise `latest` is the server's.
    pub fn resolve(&mut self, s: &str) -> Result<Named, ResolveError> {
        let s = s.trim();
        match s {
            "earliest" => return Ok(Named::Number(0)),
            "latest" | "finalized" | "pending" => {}
            _ => {
                if let Some(h) = s.strip_prefix("0x").filter(|h| h.len() == 64) {
                    if h.bytes().all(|b| b.is_ascii_hexdigit()) {
                        return Ok(Named::Hash(s.to_ascii_lowercase()));
                    }
                }
                return parse_number(s).map(Named::Number).ok_or_else(|| {
                    ResolveError::Usage(format!(
                        "{s:?} is not a block number, a block hash, \"latest\", \"finalized\" or \"earliest\""
                    ))
                });
            }
        }
        let which = if s == "finalized" {
            "finalized"
        } else {
            "latest"
        };
        match &mut self.anchor {
            AnchorBy::LightClient(a) => match a.head(which) {
                Ok(h) => {
                    self.head = Some(h.report.clone());
                    self.head_block = Some((h.number, h.hash.clone()));
                    Ok(Named::Hash(h.hash))
                }
                Err(f) => Err(ResolveError::Anchor(a.failed(&f))),
            },
            _ if which == "latest" => Ok(Named::Latest),
            _ => Err(ResolveError::Usage(
                "\"finalized\" is resolved by Willow's light client: drop --anchor-rpc / --no-anchor"
                    .into(),
            )),
        }
    }

    /// Block `number` with hash `hash` anchored: the report's `anchor` and whether it holds.
    pub fn anchor_block(&mut self, number: u64, hash: &str) -> (Value, bool) {
        match &mut self.anchor {
            AnchorBy::LightClient(a) => match a.block(number, hash) {
                Ok(r) => (r, true),
                Err(f) => (a.failed(&f), false),
            },
            AnchorBy::Rpc(url) => {
                let theirs = rpc::call(
                    self.http,
                    url,
                    "eth_getBlockByNumber",
                    json!([format!("0x{number:x}"), false]),
                );
                match theirs {
                    Ok(b) => {
                        let same = b["hash"]
                            .as_str()
                            .is_some_and(|h| h.eq_ignore_ascii_case(hash));
                        (
                            json!({"by": "--anchor-rpc", "anchored": same, "trusted": "the node at --anchor-rpc",
                                "hash": b["hash"]}),
                            same,
                        )
                    }
                    Err(e) => (
                        json!({"by": "--anchor-rpc", "anchored": false, "kind": "refused", "message": e}),
                        false,
                    ),
                }
            }
            AnchorBy::None(why) => (json!({"by": "none", "anchored": false, "note": why}), true),
        }
    }

    /// `report.anchor` and `report.block.canonical` for block `number`/`hash`; whether it holds.
    pub fn anchor_into(&mut self, report: &mut Value, number: u64, hash: &str) -> bool {
        let (a, ok) = self.anchor_block(number, hash);
        report["block"]["canonical"] = match (&self.anchor, ok) {
            (AnchorBy::None(_), _) => json!("not checked: the block is not anchored"),
            (_, ok) => json!(ok),
        };
        report["anchor"] = a;
        if let Some(h) = &self.head {
            if report["anchor"]["tag"].is_null() && h["hash"] != report["anchor"]["hash"] {
                report["head"] = h.clone();
            }
        }
        ok
    }

    /// The report's `chain` fields.
    pub fn base(&self, method: &str) -> Value {
        json!({
            "verified": false,
            "method": method,
            "chainId": self.chain_id,
            "chain": chain_name(self.chain_id),
        })
    }

    /// The report when Willow refused, or sent no proof (`null`).
    pub fn not_proven(&self, method: &str, body: &str) -> Option<Value> {
        let mut r = self.base(method);
        if let Some(e) = rpc::error_of(body) {
            r["refused"] = e;
            return Some(r);
        }
        let v: Value = serde_json::from_str(body).ok()?;
        if v.get("result").is_some_and(Value::is_null) {
            r["absent"] = json!(true);
            r["note"] = json!("Willow has no such item: an absence can't be proven, only a value");
            return Some(r);
        }
        None
    }
}

/// A comparison that could not be made (the other endpoint failed or sent no readable response):
/// shown, and not counted, as on the verify page. A `null` response is not this: it is counted.
pub fn problem(why: Value) -> Value {
    json!({"exact": false, "problem": why,
        "note": "not compared: the other endpoint sent no readable response"})
}

/// Whether a comparison counts toward the verdict: it was made.
pub fn counted(c: &Value) -> bool {
    c.get("problem").is_none()
}

/// Why a block could not be resolved: the user's text, or the anchor of the newest block.
pub enum ResolveError {
    Usage(String),
    Anchor(Value),
}

impl ResolveError {
    /// The failure report of mode `method`.
    pub fn report(self, ctx: &Ctx, method: &str) -> Result<Value, String> {
        match self {
            ResolveError::Usage(m) => Err(m),
            ResolveError::Anchor(a) => {
                let mut r = ctx.base(method);
                r["anchor"] = a;
                r["note"] =
                    json!("the newest block could not be anchored, so nothing was asked for");
                Ok(r)
            }
        }
    }
}
