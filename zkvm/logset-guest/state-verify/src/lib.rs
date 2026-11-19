








pub mod call;
pub mod estimate;
pub mod history;
pub mod memguard;
pub mod overrides;
pub mod proof;


pub mod wire;
pub mod witness;

use alloy_primitives::{keccak256, Bytes, B256, U256};
use call::{Engine, Mode, Ran, Reads, Run};
use estimate::Estimated;
use history::History;
use overrides::{Overrides, Unread};
use proof::{hx, Proven};
use revm::primitives::hardfork::SpecId;
use serde_json::{json, Value};


pub const MAX_PROOF_SLOTS: usize = 256;



#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {

    InvalidRequest,

    Malformed,


    Unsupported,

    WrongChain,

    Header,

    WrongBlock,

    Limits,


    Account,

    Slot,

    Code,

    History,

    MissingRead,

    Outcome,

    Estimate,

    Algorithm,

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
            Kind::Limits => "limits",
            Kind::Account => "account",
            Kind::Slot => "slot",
            Kind::Code => "code",
            Kind::History => "history",
            Kind::MissingRead => "missing-read",
            Kind::Outcome => "outcome",
            Kind::Estimate => "estimate",
            Kind::Algorithm => "algorithm",
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

fn invalid(m: impl Into<String>) -> Failure {
    Failure::new(Kind::InvalidRequest, m)
}

fn malformed(m: impl Into<String>) -> Failure {
    Failure::new(Kind::Malformed, m)
}


#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum At {
    Number(u64),
    Hash(B256),


    Tag,
}

impl At {

    pub fn parse(v: &Value) -> Result<At, Failure> {
        match v {
            Value::Null => Ok(At::Tag),
            Value::String(s) => match s.as_str() {
                "latest" | "pending" | "safe" | "finalized" => Ok(At::Tag),
                "earliest" => Ok(At::Number(0)),
                h if h.len() == 66 => proof::data(v)
                    .map(|b| At::Hash(B256::from_slice(&b)))
                    .map_err(invalid),
                _ => proof::quantity(v)
                    .ok()
                    .and_then(|n| u64::try_from(n).ok())
                    .map(At::Number)
                    .ok_or_else(|| invalid(format!("not a block: {s}"))),
            },
            Value::Object(o) => match (o.get("blockHash"), o.get("blockNumber")) {
                (Some(h @ Value::String(s)), None) if s.len() == 66 => Self::parse(h),
                (None, Some(n @ Value::String(_))) => Self::parse(n),
                _ => Err(invalid(format!("not a block: {v}"))),
            },
            _ => Err(invalid(format!("not a block: {v}"))),
        }
    }
}



#[derive(Clone, Debug)]
pub struct Named {
    pub chain_id: u64,
    pub number: u64,
    pub hash: B256,
    pub header: alloy::consensus::Header,
}


pub fn named(
    chain_id: u64,
    at: At,
    (chain, number, hash, header): (&str, &str, &str, &str),
) -> Result<Named, Failure> {
    let got = proof::quantity(&json!(chain)).map_err(malformed)?;
    if got != U256::from(chain_id) {
        return Err(Failure::new(
            Kind::WrongChain,
            format!("the proof is for chain {got}, not {chain_id}"),
        ));
    }
    let rlp = proof::data(&json!(header)).map_err(malformed)?;
    let h = logset_blockproof::proof::decode_header(&rlp)
        .map_err(|e| Failure::new(Kind::Header, format!("{e:#}")))?;
    let block_hash = keccak256(&rlp);
    let said = proof::data(&json!(hash)).map_err(malformed)?;
    if said.as_ref() != block_hash.as_slice() {
        return Err(Failure::new(
            Kind::Header,
            "blockHash is not the keccak of the header",
        ));
    }
    if proof::quantity(&json!(number)).map_err(malformed)? != U256::from(h.number) {
        return Err(Failure::new(
            Kind::Header,
            "blockNumber is not the header's",
        ));
    }
    match at {
        At::Number(n) if n != h.number => Err(Failure::new(
            Kind::WrongBlock,
            format!("the proof is for block {}, not {n}", h.number),
        )),
        At::Hash(want) if want != block_hash => Err(Failure::new(
            Kind::WrongBlock,
            format!("the proof is for block {block_hash}, not {want}"),
        )),
        _ => Ok(Named {
            chain_id,
            number: h.number,
            hash: block_hash,
            header: h,
        }),
    }
}

fn parse<T: serde::de::DeserializeOwned>(result: &Value) -> Result<T, Failure> {
    serde_json::from_value(result.clone()).map_err(|e| malformed(format!("the result: {e}")))
}


#[derive(Clone, Debug)]
pub struct ProvenState {
    pub block: Named,
    pub account: Proven,

    pub code: Option<Bytes>,



    pub responses: Value,
}


pub fn verify_state(chain_id: u64, params: &Value, result: &Value) -> Result<ProvenState, Failure> {
    let p = |i: usize| params.get(i).unwrap_or(&Value::Null);
    let address = proof::address(p(0)).map_err(invalid)?;
    let keys = p(1)
        .as_array()
        .ok_or_else(|| invalid("expected a list of storage keys"))?;
    if keys.len() > MAX_PROOF_SLOTS {
        return Err(invalid(format!(
            "at most {MAX_PROOF_SLOTS} storage keys per state proof"
        )));
    }
    let slots: Vec<B256> = keys
        .iter()
        .map(|k| proof::slot(k).map_err(invalid))
        .collect::<Result<_, _>>()?;
    let at = At::parse(p(2))?;
    let want_code = match p(3) {
        Value::Null => false,
        o => o["code"].as_bool().unwrap_or(false),
    };
    let r: wire::StateProof = parse(result)?;
    let block = named(
        chain_id,
        at,
        (&r.chain_id, &r.block_number, &r.block_hash, &r.header),
    )?;
    let account = proof::check(block.header.state_root, address, &slots, &r.account)?;
    let given = match &r.code {
        Some(c) => Some(proof::data(&json!(c)).map_err(malformed)?),
        None => None,
    };
    let code = match (&given, account.has_code()) {
        (Some(c), _) if !proof::code_matches(&account, c) => {
            return Err(Failure::new(
                Kind::Code,
                format!("the code sent is not account {address}'s code hash"),
            ))
        }
        (Some(c), _) => Some(c.clone()),
        (None, true) if want_code => {
            return Err(Failure::new(
                Kind::Code,
                format!("the code of {address} was asked for and left out"),
            ))
        }
        (None, _) => None,
    };
    let code = want_code.then(|| code.unwrap_or_default());
    let responses = json!({
        "eth_getBalance": format!("{:#x}", account.balance),
        "eth_getTransactionCount": format!("0x{:x}", account.nonce),
        "eth_getCode": code.as_ref().map(|c| hx(c)),
        "eth_getStorageAt": account
            .slots
            .iter()
            .map(|(_, v)| hx(B256::from(*v).as_slice()))
            .collect::<Vec<_>>(),
        "eth_getProof": proof::response(&account, &r.account, keys.clone()),
    });
    Ok(ProvenState {
        block,
        account,
        code,
        responses,
    })
}



#[derive(Clone, Debug, PartialEq)]
pub struct CallRequest {
    pub engine: Engine,
    pub call: call::Call,
    pub at: At,
    pub overrides: Overrides,
}



pub fn call_request(chain_id: u64, params: &Value) -> Result<CallRequest, Failure> {
    let Some(engine) = call::engine(chain_id) else {
        return Err(Failure::new(
            Kind::Unsupported,
            format!("calls are verified on Ethereum, Unichain, OP Mainnet, Ink, World Chain, Soneium, Robinhood Chain and Arbitrum One, not chain {chain_id}"),
        ));
    };
    let unsupported = |m: String| Failure::new(Kind::Unsupported, m);
    let p = |i: usize| params.get(i).unwrap_or(&Value::Null);
    if engine.is_nitro() {

        if ["beaconRoot", "withdrawals"]
            .iter()
            .any(|k| !p(3)[*k].is_null())
        {
            return Err(unsupported(
                "a block override Nitro refuses for eth_call".into(),
            ));
        }
        let arbos = willow_arb_exec::ARBOS_STATE;
        if p(2).as_object().is_some_and(|m| {
            m.keys()
                .any(|k| proof::address(&json!(k)).ok() == Some(arbos))
        }) {
            return Err(unsupported(
                "ArbOS's state account can't be overridden".into(),
            ));
        }
    }
    let c = call::parse_on(p(0), chain_id).map_err(|e| match e {
        call::Unrunnable::Invalid(m) => invalid(m),
        call::Unrunnable::NotServed(m) => unsupported(m),
    })?;
    let overrides = overrides::parse(p(2), p(3)).map_err(|e| match e {
        Unread::Invalid(m) => invalid(m),
        Unread::TooLarge(m) | Unread::NotServed(m) => unsupported(m),
    })?;
    Ok(CallRequest {
        engine,
        call: c,
        at: At::parse(p(1))?,
        overrides,
    })
}




pub fn proven_witness(
    req: &CallRequest,
    block: &Named,
    w: &wire::Witness,
) -> Result<call::Witness, Failure> {
    let history = History::of(block.chain_id);
    witness::verify_on(
        history,
        block.header.state_root,
        w,
        &req.overrides.own_code(),
    )
}


pub fn env_of(block: &Named) -> call::Block {
    call::Block::from_header(
        &block.header,
        &logset_blockproof::derive::BlobSchedule::builtin(block.chain_id),
    )
}


fn unfinished(ran: &Ran) -> Option<Failure> {
    match ran {
        Ran::Missing(m) => Some(Failure::new(
            Kind::MissingRead,
            format!("the call read state the witness does not hold: {m:?}"),
        )),
        Ran::NoBlockHash(n) => Some(Failure::new(
            Kind::MissingRead,
            format!("the call read the hash of block {n}, which the witness does not prove"),
        )),
        Ran::Refused(m) => Some(Failure::new(
            Kind::Unsupported,
            format!("the call is not run here: {m}"),
        )),
        Ran::Internal(m) => Some(Failure::new(Kind::Internal, m.clone())),
        _ => None,
    }
}


#[derive(Clone, Debug)]
pub struct ProvenCall {
    pub block: Named,
    pub outcome: wire::Outcome,

    pub response: Value,
}



pub fn verify_call(chain_id: u64, params: &Value, result: &Value) -> Result<ProvenCall, Failure> {
    let req = call_request(chain_id, params)?;
    let r: wire::CallProof = parse(result)?;
    let block = named(
        chain_id,
        req.at,
        (&r.chain_id, &r.block_number, &r.block_hash, &r.header),
    )?;
    let w = proven_witness(&req, &block, &r.witness)?;
    let env = env_of(&block);
    let hash_of = hashes(chain_id, &w, &env);
    let run = Run {
        engine: req.engine,
        witness: &w,
        block: &env,
        call: &req.call,
        overrides: &req.overrides,
        hash_of: &hash_of,
        mode: Mode::Strict,
    };
    let ran = run.call(&mut Reads::default());
    if let Some(f) = unfinished(&ran) {
        return Err(f);
    }
    let outcome =
        wire::Outcome::of(&ran).ok_or_else(|| Failure::new(Kind::Internal, "no outcome"))?;
    if Some(&outcome) != r.result.normalized().as_ref() {
        return Err(Failure::new(
            Kind::Outcome,
            format!("the call comes to {outcome:?}, not {:?}", r.result),
        ));
    }
    let response = wire::Outcome::response_of(&ran)
        .ok_or_else(|| Failure::new(Kind::Internal, "no response"))?;
    Ok(ProvenCall {
        block,
        outcome,
        response,
    })
}



pub fn hashes<'a>(
    chain_id: u64,
    w: &'a call::Witness,
    env: &'a call::Block,
) -> impl Fn(u64) -> Option<B256> + 'a {
    let history = History::of(chain_id);
    let active =
        chain_id != 1 || call::spec(env.number, env.timestamp).is_enabled_in(SpecId::PRAGUE);
    move |n| {
        active
            .then(|| history.block_hash(w, env.number, n))
            .flatten()
    }
}


#[derive(Clone, Debug)]
pub struct ProvenEstimate {
    pub block: Named,
    pub estimate: wire::Estimate,

    pub response: Value,
}




pub fn verify_estimate(
    chain_id: u64,
    params: &Value,
    result: &Value,
) -> Result<ProvenEstimate, Failure> {
    let req = call_request(chain_id, params)?;
    if !estimate::estimated_on(req.engine) {
        return Err(Failure::new(
            Kind::Unsupported,
            format!("estimates are not verified on chain {chain_id}"),
        ));
    }
    if let Some(why) = estimate::not_searched(&req) {
        return Err(Failure::new(Kind::Unsupported, why));
    }
    let r: wire::EstimateProof = parse(result)?;
    let algorithm = wire::estimate_algorithm(chain_id);
    if r.algorithm != algorithm {
        return Err(Failure::new(
            Kind::Algorithm,
            format!("an estimate by {:?}, not {algorithm}", r.algorithm),
        ));
    }
    let block = named(
        chain_id,
        req.at,
        (&r.chain_id, &r.block_number, &r.block_hash, &r.header),
    )?;
    let w = proven_witness(&req, &block, &r.witness)?;
    let env = env_of(&block);
    let hash_of = hashes(chain_id, &w, &env);
    let run = Run {
        engine: req.engine,
        witness: &w,
        block: &env,
        call: &req.call,
        overrides: &req.overrides,
        hash_of: &hash_of,
        mode: Mode::Strict,
    };
    let found = estimate::estimate_run(&run, &mut Reads::default());
    let estimate = match &found {
        Estimated::Missing(m) => {
            return Err(Failure::new(
                Kind::MissingRead,
                format!("the search read state the witness does not hold: {m:?}"),
            ))
        }
        Estimated::NoBlockHash(n) => {
            return Err(Failure::new(
                Kind::MissingRead,
                format!("the search read the hash of block {n}, which the witness does not prove"),
            ))
        }
        Estimated::Internal(m) => return Err(Failure::new(Kind::Internal, m.clone())),
        e => wire::Estimate::of(e).ok_or_else(|| Failure::new(Kind::Internal, "no estimate"))?,
    };
    if Some(&estimate) != r.estimate.normalized().as_ref() {
        return Err(Failure::new(
            Kind::Estimate,
            format!("the search comes to {estimate:?}, not {:?}", r.estimate),
        ));
    }
    if let Estimated::Gas(gas) = found {
        let at_gas = call::Call {
            gas: Some(gas),
            ..req.call.clone()
        };
        let ran = Run {
            call: &at_gas,
            ..run
        }
        .at_gas(gas, &mut Reads::default(), false)
        .ran;
        if let Some(f) = unfinished(&ran) {
            return Err(f);
        }
        if !matches!(ran, Ran::Returned(_)) {
            return Err(Failure::new(
                Kind::Estimate,
                format!("the call does not succeed with the {gas} gas estimated: {ran:?}"),
            ));
        }
    }
    let response = wire::Estimate::response_of(&found)
        .ok_or_else(|| Failure::new(Kind::Internal, "no response"))?;
    Ok(ProvenEstimate {
        block,
        estimate,
        response,
    })
}
