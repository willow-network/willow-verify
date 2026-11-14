//! State, call and estimate proofs: what Willow sends for `willow_getStateProof`,
//! `willow_getCallProof` and `willow_getEstimateGasProof` ([`wire`]), and the checks a client runs
//! on them ([`verify_state`], [`verify_call`], [`verify_estimate`]). Every check binds the proof
//! to the client's own request and to the header it names; the client anchors that header's hash
//! itself. The server runs the same EVM ([`call`], [`estimate`]) and checks what it sends with the
//! same functions.

pub mod call;
pub mod estimate;
pub mod history;
pub mod proof;
#[cfg(test)]
mod tests;
pub mod wire;
pub mod witness;

use alloy_primitives::{keccak256, Bytes, B256, U256};
use call::{Mode, Ran, Reads};
use estimate::Estimated;
use proof::{hx, Proven};
use revm::primitives::hardfork::SpecId;
use serde_json::{json, Value};

/// Storage keys one state proof may ask for (the server's limit for `eth_getProof`).
pub const MAX_PROOF_SLOTS: usize = 256;

/// Why a proof did not verify. The client reports `kind`; the tamper suite expects one per
/// mutation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// The client's own request can't be read.
    InvalidRequest,
    /// The result can't be read (a field missing, not hex, the wrong length).
    Malformed,
    /// Not verified here: calls on a chain other than Ethereum.
    Unsupported,
    /// The proof is for another chain.
    WrongChain,
    /// The header does not hash to `blockHash`, does not decode, or is not `blockNumber`.
    Header,
    /// The proof is for another block than the one asked for.
    WrongBlock,
    /// The witness is larger than one call may read.
    Limits,
    /// An account's proof does not verify, is of another account, or proves an absent account
    /// with a balance or nonce.
    Account,
    /// A slot's proof does not verify, is of another slot, or a slot asked for is not proven.
    Slot,
    /// Code that is not its account's code hash, or code left out.
    Code,
    /// The EIP-2935 history account is not EIP-2935's.
    History,
    /// The call read an account, slot, code or block hash the witness does not hold.
    MissingRead,
    /// The call comes to another outcome than the result says.
    Outcome,
    /// The search comes to another estimate than the result says.
    Estimate,
    /// An estimate by an algorithm this client doesn't know.
    Algorithm,
    /// The EVM refused the block or itself, not the transaction.
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

fn invalid(m: impl Into<String>) -> Failure {
    Failure::new(Kind::InvalidRequest, m)
}

fn malformed(m: impl Into<String>) -> Failure {
    Failure::new(Kind::Malformed, m)
}

/// A block as a request names it (number, tag, hash, or an EIP-1898 object).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum At {
    Number(u64),
    Hash(B256),
    /// `latest`, `pending`, `safe`, `finalized`: any block the server resolved it to (the client
    /// decides whether that block is the one the tag means).
    Tag,
}

impl At {
    /// `v` as geth reads a block parameter; absent or null is `latest`.
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

/// The block a proof names, checked: the header hashes to its hash and decodes canonically, and
/// it is the chain's and the block asked for.
#[derive(Clone, Debug)]
pub struct Named {
    pub chain_id: u64,
    pub number: u64,
    pub hash: B256,
    pub header: alloy::consensus::Header,
}

/// [`Named`] from a result's `chainId`, `blockNumber`, `blockHash` and `header`.
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

/// A verified `willow_getStateProof`.
#[derive(Clone, Debug)]
pub struct ProvenState {
    pub block: Named,
    pub account: Proven,
    /// The account's code, when asked for (empty when it has none).
    pub code: Option<Bytes>,
    /// The standard responses at the block, as Willow's own state methods send them:
    /// `eth_getBalance`, `eth_getTransactionCount`, `eth_getCode` (null unless asked for),
    /// `eth_getStorageAt` (one per key, in order) and `eth_getProof`.
    pub responses: Value,
}

/// `result` checked as the answer to `willow_getStateProof` `params` on chain `chain_id`.
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

/// A call request: `[call, block]`, without overrides (the wasm's `request` reads it too).
pub fn call_request(chain_id: u64, params: &Value) -> Result<(call::Call, At), Failure> {
    if chain_id != 1 {
        return Err(Failure::new(
            Kind::Unsupported,
            format!("calls are verified on Ethereum only, not chain {chain_id}"),
        ));
    }
    let p = |i: usize| params.get(i).unwrap_or(&Value::Null);
    let given = |o: &Value| !o.is_null() && o.as_object().is_none_or(|m| !m.is_empty());
    if given(p(2)) || given(p(3)) {
        return Err(invalid(
            "a call with state or block overrides is not proven",
        ));
    }
    let c = call::parse(p(0)).map_err(|e| match e {
        call::Unrunnable::Invalid(m) | call::Unrunnable::NotServed(m) => invalid(m),
    })?;
    Ok((c, At::parse(p(1))?))
}

/// The witness of a call proof, verified against the block's state root.
fn proven_witness(block: &Named, w: &wire::Witness) -> Result<call::Witness, Failure> {
    witness::verify(block.header.state_root, w)
}

/// A strict run's failure to complete over a verified witness.
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
        Ran::Internal(m) => Some(Failure::new(Kind::Internal, m.clone())),
        _ => None,
    }
}

/// A verified `willow_getCallProof`.
#[derive(Clone, Debug)]
pub struct ProvenCall {
    pub block: Named,
    pub outcome: wire::Outcome,
    /// The standard `eth_call` response: `{"result"}` or `{"error"}`, as Willow's sends it.
    pub response: Value,
}

/// `result` checked as the answer to `willow_getCallProof` `params` on chain `chain_id`: the
/// client's own run over the witness comes to the result sent.
pub fn verify_call(chain_id: u64, params: &Value, result: &Value) -> Result<ProvenCall, Failure> {
    let (c, at) = call_request(chain_id, params)?;
    let r: wire::CallProof = parse(result)?;
    let block = named(
        chain_id,
        at,
        (&r.chain_id, &r.block_number, &r.block_hash, &r.header),
    )?;
    let w = proven_witness(&block, &r.witness)?;
    let env = call::Block::from_header(
        &block.header,
        &logset_blockproof::derive::BlobSchedule::builtin(1),
    );
    let hash_of = hashes(&w, &env);
    let ran = call::run_with(&w, &env, &c, &hash_of, Mode::Strict, &mut Reads::default());
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
    let response = outcome.response();
    Ok(ProvenCall {
        block,
        outcome,
        response,
    })
}

/// Block hashes as the call block's state holds them (EIP-2935): from Prague on, else none.
fn hashes<'a>(w: &'a call::Witness, env: &'a call::Block) -> impl Fn(u64) -> Option<B256> + 'a {
    let prague = call::spec(env.number, env.timestamp).is_enabled_in(SpecId::PRAGUE);
    move |n| {
        prague
            .then(|| history::block_hash(w, env.number, n))
            .flatten()
    }
}

/// A verified `willow_getEstimateGasProof`.
#[derive(Clone, Debug)]
pub struct ProvenEstimate {
    pub block: Named,
    pub estimate: wire::Estimate,
    /// The standard `eth_estimateGas` response: `{"result"}` or `{"error"}`, as geth sends it.
    pub response: Value,
}

/// `result` checked as the answer to `willow_getEstimateGasProof` `params` on chain `chain_id`:
/// the client's own search over the witness comes to the estimate sent, and the call succeeds
/// with the gas it names.
pub fn verify_estimate(
    chain_id: u64,
    params: &Value,
    result: &Value,
) -> Result<ProvenEstimate, Failure> {
    let (c, at) = call_request(chain_id, params)?;
    let r: wire::EstimateProof = parse(result)?;
    if r.algorithm != wire::ESTIMATE_ALGORITHM {
        return Err(Failure::new(
            Kind::Algorithm,
            format!(
                "an estimate by {:?}, not {}",
                r.algorithm,
                wire::ESTIMATE_ALGORITHM
            ),
        ));
    }
    let block = named(
        chain_id,
        at,
        (&r.chain_id, &r.block_number, &r.block_hash, &r.header),
    )?;
    let w = proven_witness(&block, &r.witness)?;
    let env = call::Block::from_header(
        &block.header,
        &logset_blockproof::derive::BlobSchedule::builtin(1),
    );
    let hash_of = hashes(&w, &env);
    let found = estimate::estimate(&w, &env, &c, &hash_of, Mode::Strict, &mut Reads::default());
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
        let mut at_gas = c.clone();
        at_gas.gas = Some(gas);
        let ran = call::run_with(
            &w,
            &env,
            &at_gas,
            &hash_of,
            Mode::Strict,
            &mut Reads::default(),
        );
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
    let response = estimate.response();
    Ok(ProvenEstimate {
        block,
        estimate,
        response,
    })
}
