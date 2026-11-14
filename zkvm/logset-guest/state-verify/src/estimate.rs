//! `eth_estimateGas`: geth v1.17.10's `Estimate` (eth/gasestimator/gasestimator.go), line for
//! line, with the call as `DoEstimateGas` and `CallDefaults` (internal/ethapi) make it. Each run on
//! Ethereum is geth's (`call::Run::at_gas`: gas bought at the price named); on Unichain each is the
//! chain's own `eth_call` run (op-reth's, nothing charged) at the gas tried, and the search is the
//! same. On Robinhood Chain the search is Nitro's ([`nitro_search`]: the Arbitrum go-ethereum
//! fork's `Estimate` as Nitro v3.12.2's `DoEstimateGas` calls it) over Nitro's gas estimation
//! runs. The caller's overrides are laid over every run, and the balance and code the search
//! reads itself are read under them. The search is deterministic over the witness, so a client
//! that runs it over the proof's witness gets the very number Willow sent.

use crate::call::{
    self, Block, Call, Engine, Exec, Missing, Mode, Ran, Reads, Run, Witness, GAS_CAP,
};
use crate::overrides::Overrides;
use alloy_primitives::{Address, Bytes, B256, U256};
use revm::primitives::hardfork::SpecId;

/// `params.TxGas`, `params.CallStipend`, `params.MaxTxGas` (EIP-7825).
const TX_GAS: u64 = 21_000;
const CALL_STIPEND: u64 = 2_300;
const MAX_TX_GAS: u64 = 1 << 24;
/// `estimateGasErrorRatio` (internal/ethapi/api.go).
const ERROR_RATIO: f64 = 0.015;

/// Whether `eth_estimateGas` is run (and its proof verified) on chains of `engine`.
pub fn estimated_on(engine: Engine) -> bool {
    matches!(engine, Engine::Ethereum | Engine::Op | Engine::Nitro)
}

/// Why an estimate the call request asks for is not searched here: on Robinhood Chain, a
/// difficulty or prevRandao block override (Nitro's estimate makes a header of them, whose mix
/// hash carries ArbOS's version and L1 block, unlike its `eth_call`, which isn't rebuilt).
pub fn not_searched(req: &crate::CallRequest) -> Option<String> {
    let o = &req.overrides.block;
    (req.engine == Engine::Nitro && (o.difficulty.is_some() || o.random.is_some())).then(|| {
        "eth_estimateGas with a difficulty or prevRandao block override is not run on Robinhood \
         Chain here"
            .into()
    })
}

/// What the search came to.
#[derive(Clone, Debug, PartialEq)]
pub enum Estimated {
    Gas(u64),
    /// Reverted at the most gas it may have: the revert data.
    Reverted(Bytes),
    /// Reverted with one of ArbOS's own errors: the data, and Nitro's reason for it.
    RevertedSaying(Bytes, String),
    /// Geth's error message.
    Error(String),
    /// A run read state the witness does not hold (see [`call::Ran::Missing`]).
    Missing(Missing),
    /// A run read a block hash `hash_of` does not have.
    NoBlockHash(u64),
    /// The EVM refused the block or itself.
    Internal(String),
}

/// geth's `execute`: whether the run failed for what might be a lack of gas, its result, and an
/// error unrelated to the gas limit. `Err`: the run itself could not be completed here.
struct Executed {
    failed: bool,
    result: Option<Exec>,
    err: Option<String>,
}

/// An account the estimate reads outside the EVM (the sender's balance, the recipient's code).
fn account<'w>(
    witness: &'w Witness,
    address: Address,
    reads: &mut Reads,
) -> Result<Option<&'w crate::proof::Proven>, Estimated> {
    match witness.account(&address) {
        Some(p) => {
            reads.accounts.entry(address).or_default();
            Ok(Some(p).filter(|p| p.exists))
        }
        None => {
            let mut m = Missing::default();
            m.accounts.entry(address).or_default();
            Err(Estimated::Missing(m))
        }
    }
}

/// `address`'s balance as the runs see it: the caller's, where the overrides set it.
fn balance(run: &Run, address: Address, reads: &mut Reads) -> Result<U256, Estimated> {
    match run.overrides.state.get(&address).and_then(|o| o.balance) {
        Some(b) => Ok(b),
        None => Ok(account(run.witness, address, reads)?.map_or(U256::ZERO, |p| p.balance)),
    }
}

/// Whether `address` has code as the runs see it: the caller's, where the overrides set it.
fn has_code(run: &Run, address: Address, reads: &mut Reads) -> Result<bool, Estimated> {
    match run
        .overrides
        .state
        .get(&address)
        .and_then(|o| o.code.as_ref())
    {
        Some(c) => Ok(!c.is_empty()),
        None => Ok(account(run.witness, address, reads)?.is_some_and(|p| p.has_code())),
    }
}

/// The lowest gas limit at which `call` runs at `block` on Ethereum without failing, as geth
/// searches for it (no overrides).
pub fn estimate(
    witness: &Witness,
    block: &Block,
    call: &Call,
    hash_of: &dyn Fn(u64) -> Option<B256>,
    mode: Mode,
    reads: &mut Reads,
) -> Estimated {
    let none = Overrides::default();
    let run = Run {
        engine: Engine::Ethereum,
        witness,
        block,
        call,
        overrides: &none,
        hash_of,
        mode,
    };
    estimate_run(&run, reads)
}

/// The lowest gas limit at which `run`'s call runs without failing, as geth searches for it.
pub fn estimate_run(run: &Run, reads: &mut Reads) -> Estimated {
    let peak = |x: &Exec| x.peak;
    search(run, reads, &peak).unwrap_or_else(|e| e)
}

/// geth's `execute`: one run at `gas` (`measure`: with its peak), read as the search reads it.
fn execute(run: &Run, gas: u64, measure: bool, reads: &mut Reads) -> Result<Executed, Estimated> {
    let x = run.at_gas(gas, reads, measure);
    let invalid = match &x.ran {
        Ran::Missing(m) => return Err(Estimated::Missing(m.clone())),
        Ran::NoBlockHash(n) => return Err(Estimated::NoBlockHash(*n)),
        Ran::Internal(m) | Ran::Refused(m) => return Err(Estimated::Internal(m.clone())),
        Ran::Invalid(m) => Some(m.clone()),
        Ran::Returned(_) | Ran::Reverted(_) | Ran::RevertedSaying(..) | Ran::Halted(_) => None,
    };
    if let Some(m) = invalid {
        // ErrIntrinsicGas: raise the gas limit. (ErrGasLimitTooHigh can't be: no cap here.)
        return Ok(Executed {
            failed: true,
            result: None,
            err: (!x.intrinsic).then(|| format!("failed with {gas} gas: {m}")),
        });
    }
    Ok(Executed {
        failed: !matches!(x.ran, Ran::Returned(_)),
        result: Some(x),
        err: None,
    })
}

/// What a run that failed at the most gas the search allows comes to: its revert or halt, or
/// (out of gas) geth's error naming that gas.
fn failed_at(x: &Executed, hi: u64) -> Estimated {
    if let Some(r) = &x.result {
        if !r.out_of_gas {
            return match &r.ran {
                Ran::Reverted(data) => Estimated::Reverted(data.clone()),
                Ran::RevertedSaying(data, said) => {
                    Estimated::RevertedSaying(data.clone(), said.clone())
                }
                Ran::Halted(m) => Estimated::Error(call::halt_message(m)),
                other => Estimated::Internal(format!("a failed run came to {other:?}")),
            };
        }
    }
    Estimated::Error(format!("gas required exceeds allowance ({hi})"))
}

/// The search, with geth's `MaxUsedGas` of a run taken by `peak` (the tests try another mapping).
pub(crate) fn search(
    run: &Run,
    reads: &mut Reads,
    peak: &dyn Fn(&Exec) -> u64,
) -> Result<Estimated, Estimated> {
    if !estimated_on(run.engine) {
        return Ok(Estimated::Internal(
            "eth_estimateGas is not run on this chain".into(),
        ));
    }
    if run.engine == Engine::Nitro {
        return nitro_search(run, reads, peak);
    }
    let (block, call) = (run.block, run.call);
    let execute = |gas: u64, measure: bool, reads: &mut Reads| execute(run, gas, measure, reads);
    // CallDefaults: no 1559 fields before London (geth: the header has no base fee).
    let london = block.base_fee.is_some();
    if !london && (call.max_fee_per_gas.is_some() || call.max_priority_fee_per_gas.is_some()) {
        return Ok(Estimated::Error(
            "maxFeePerGas and maxPriorityFeePerGas are not valid before London is active".into(),
        ));
    }
    // DoEstimateGas: a gas not given is 0; CallDefaults caps one given at the RPC gas cap.
    let call_gas = call.gas.map_or(0, |g| g.min(GAS_CAP));

    // Determine the highest gas limit can be used during the estimation (the header's, as the
    // caller's block override makes it: `MakeHeader`).
    let mut hi = run.overrides.block.gas_limit.unwrap_or(block.gas_limit);
    if call_gas >= TX_GAS {
        hi = call_gas;
    }
    // Cap the maximum gas allowance according to EIP-7825 if the estimation targets Osaka (the
    // fork table has no Amsterdam): Ethereum's.
    let osaka = run.engine == Engine::Ethereum
        && call::spec(block.number, block.timestamp).is_enabled_in(SpecId::OSAKA);
    if hi > MAX_TX_GAS && osaka {
        hi = MAX_TX_GAS;
    }
    // Normalize the max fee per gas the call is willing to spend (ToMessage: gasPrice, else
    // maxFeePerGas, else zero).
    let fee_cap = call.gas_price.or(call.max_fee_per_gas).unwrap_or(0);
    // Recap the highest gas limit with account's available balance.
    if fee_cap != 0 {
        let mut available = balance(run, call.from, reads)?;
        if call.value >= available {
            return Ok(Estimated::Error("insufficient funds for transfer".into()));
        }
        available -= call.value;
        let allowance = available / U256::from(fee_cap);
        // If the allowance is larger than maximum uint64, skip checking
        if let Ok(allowance) = u64::try_from(allowance) {
            if hi > allowance {
                hi = allowance;
            }
        }
    }
    // Recap the highest gas allowance with specified gascap.
    if hi > GAS_CAP {
        hi = GAS_CAP;
    }
    // If the transaction is a plain value transfer, short circuit estimation by executing it
    // once with a 21000 gas limit and returning the gas it used.
    if call.data.is_empty() {
        if let Some(to) = call.to {
            if !has_code(run, to, reads)? {
                let x = execute(TX_GAS, false, reads)?;
                if !x.failed && x.err.is_none() {
                    let used = x.result.map_or(TX_GAS, |r| r.used);
                    return Ok(Estimated::Gas(used));
                }
            }
        }
    }
    // We first execute the transaction at the highest allowable gas limit, since if this fails
    // we can return error immediately.
    let x = execute(hi, true, reads)?;
    if let Some(e) = x.err {
        return Ok(Estimated::Error(e));
    }
    if x.failed {
        return Ok(failed_at(&x, hi));
    }
    let Some(result) = x.result else {
        return Ok(Estimated::Internal(
            "a run that did not fail has no result".into(),
        ));
    };
    bisect(run, reads, &result, peak, hi)
}

/// The search after its first run at `hi` succeeded with `result`: the optimistic try, then the
/// bisection (geth's, and the Arbitrum fork's alike).
fn bisect(
    run: &Run,
    reads: &mut Reads,
    result: &Exec,
    peak: &dyn Fn(&Exec) -> u64,
    mut hi: u64,
) -> Result<Estimated, Estimated> {
    let execute = |gas: u64, measure: bool, reads: &mut Reads| execute(run, gas, measure, reads);
    // For almost any transaction, the gas consumed by the unconstrained execution above
    // lower-bounds the gas limit required for it to succeed.
    let mut lo = result.used.saturating_sub(1);
    // There's a fairly high chance for the transaction to execute successfully with gasLimit set
    // to the first execution's usedGas + gasRefund.
    let optimistic = (peak(result) + CALL_STIPEND) * 64 / 63;
    if optimistic < hi {
        let x = execute(optimistic, false, reads)?;
        if let Some(e) = x.err {
            return Ok(Estimated::Error(e));
        }
        if x.failed {
            lo = optimistic;
        } else {
            hi = optimistic;
        }
    }
    // Binary search for the smallest gas limit that allows the tx to execute successfully.
    while lo + 1 < hi {
        // It is a bit pointless to return a perfect estimation, as changing network conditions
        // require the caller to bump it up anyway.
        if (hi - lo) as f64 / (hi as f64) < ERROR_RATIO {
            break;
        }
        let mut mid = lo + (hi - lo) / 2;
        if mid > lo * 2 {
            // Most txs don't need much higher gas limit than their gas used.
            mid = lo * 2;
        }
        let x = execute(mid, false, reads)?;
        if let Some(e) = x.err {
            return Ok(Estimated::Error(e));
        }
        if x.failed {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    Ok(Estimated::Gas(hi))
}

/// Robinhood Chain's search: the Arbitrum go-ethereum fork's `Estimate`
/// (eth/gasestimator/gasestimator.go at the fork Nitro v3.12.2 builds with, 30456d24) as Nitro's
/// `DoEstimateGas` (internal/ethapi/api.go) calls it, each run Nitro's gas estimation mode. Where
/// it differs from geth's: the RPC gas cap is raised by the call's L1 data's gas
/// (`RPCPostingGasHook`), the gas limit the search starts from is the header's under the caller's
/// override, no EIP-7825 cap, and a plain transfer that runs at 21,000 is 21,000.
fn nitro_search(
    run: &Run,
    reads: &mut Reads,
    peak: &dyn Fn(&Exec) -> u64,
) -> Result<Estimated, Estimated> {
    let (block, call) = (run.block, run.call);
    let execute = |gas: u64, measure: bool, reads: &mut Reads| execute(run, gas, measure, reads);
    // DoEstimateGas: a gas not given is 0; CallDefaults caps one given at the RPC gas cap.
    let call_gas = call.gas.map_or(0, |g| g.min(GAS_CAP));
    // Arbitrum: raise the gas cap to ignore L1 costs so that it's compute-only.
    let posting = run.nitro_posting(reads).map_err(|r| match r {
        Ran::Missing(m) => Estimated::Missing(m),
        Ran::Refused(m) | Ran::Internal(m) => Estimated::Internal(m),
        other => Estimated::Internal(format!("the posting gas came to {other:?}")),
    })?;
    let gas_cap = GAS_CAP.wrapping_add(posting);
    // Determine the highest gas limit can be used during the estimation (the header's, as the
    // caller's block override makes it).
    let mut hi = run.overrides.block.gas_limit.unwrap_or(block.gas_limit);
    if call_gas >= TX_GAS {
        hi = call_gas;
    }
    // Normalize the max fee per gas the call is willing to spend.
    let fee_cap = call.gas_price.or(call.max_fee_per_gas).unwrap_or(0);
    // Recap the highest gas limit with account's available balance.
    if fee_cap != 0 {
        let mut available = balance(run, call.from, reads)?;
        if call.value >= available {
            return Ok(Estimated::Error("insufficient funds for transfer".into()));
        }
        available -= call.value;
        let allowance = available / U256::from(fee_cap);
        // If the allowance is larger than maximum uint64, skip checking
        if let Ok(allowance) = u64::try_from(allowance) {
            if hi > allowance {
                hi = allowance;
            }
        }
    }
    // Recap the highest gas allowance with specified gascap.
    if gas_cap != 0 && hi > gas_cap {
        hi = gas_cap;
    }
    // If the transaction is a plain value transfer, short circuit estimation and directly try
    // 21000.
    if call.data.is_empty() {
        if let Some(to) = call.to {
            if !has_code(run, to, reads)? {
                let x = execute(TX_GAS, false, reads)?;
                if !x.failed && x.err.is_none() {
                    return Ok(Estimated::Gas(TX_GAS));
                }
            }
        }
    }
    // We first execute the transaction at the highest allowable gas limit, since if this fails
    // we can return error immediately.
    let x = execute(hi, true, reads)?;
    if let Some(e) = x.err {
        return Ok(Estimated::Error(e));
    }
    if x.failed {
        return Ok(failed_at(&x, hi));
    }
    let Some(result) = x.result else {
        return Ok(Estimated::Internal(
            "a run that did not fail has no result".into(),
        ));
    };
    bisect(run, reads, &result, peak, hi)
}
