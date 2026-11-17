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
    self, Engine, Exec, Missing, Ran, Reads, Run, Witness, GAS_CAP,
};

use alloy_primitives::{Address, Bytes, U256};
use revm::primitives::hardfork::SpecId;


const TX_GAS: u64 = 21_000;
const CALL_STIPEND: u64 = 2_300;
const MAX_TX_GAS: u64 = 1 << 24;

const ERROR_RATIO: f64 = 0.015;


pub fn estimated_on(engine: Engine) -> bool {
    matches!(engine, Engine::Ethereum | Engine::Op | Engine::Nitro)
}




pub fn not_searched(req: &crate::CallRequest) -> Option<String> {
    let o = &req.overrides.block;
    (req.engine == Engine::Nitro && (o.difficulty.is_some() || o.random.is_some())).then(|| {
        "eth_estimateGas with a difficulty or prevRandao block override is not run on Robinhood \
         Chain here"
            .into()
    })
}


#[derive(Clone, Debug, PartialEq)]
pub enum Estimated {
    Gas(u64),

    Reverted(Bytes),

    RevertedSaying(Bytes, String),

    Error(String),

    Missing(Missing),

    NoBlockHash(u64),

    Internal(String),
}



struct Executed {
    failed: bool,
    result: Option<Exec>,
    err: Option<String>,
}


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


fn balance(run: &Run, address: Address, reads: &mut Reads) -> Result<U256, Estimated> {
    match run.overrides.state.get(&address).and_then(|o| o.balance) {
        Some(b) => Ok(b),
        None => Ok(account(run.witness, address, reads)?.map_or(U256::ZERO, |p| p.balance)),
    }
}


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

























pub fn estimate_run(run: &Run, reads: &mut Reads) -> Estimated {
    let peak = |x: &Exec| x.peak;
    search(run, reads, &peak).unwrap_or_else(|e| e)
}


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

    let london = block.base_fee.is_some();
    if !london && (call.max_fee_per_gas.is_some() || call.max_priority_fee_per_gas.is_some()) {
        return Ok(Estimated::Error(
            "maxFeePerGas and maxPriorityFeePerGas are not valid before London is active".into(),
        ));
    }

    let call_gas = call.gas.map_or(0, |g| g.min(GAS_CAP));



    let mut hi = run.overrides.block.gas_limit.unwrap_or(block.gas_limit);
    if call_gas >= TX_GAS {
        hi = call_gas;
    }


    let osaka = run.engine == Engine::Ethereum
        && call::spec(block.number, block.timestamp).is_enabled_in(SpecId::OSAKA);
    if hi > MAX_TX_GAS && osaka {
        hi = MAX_TX_GAS;
    }


    let fee_cap = call.gas_price.or(call.max_fee_per_gas).unwrap_or(0);

    if fee_cap != 0 {
        let mut available = balance(run, call.from, reads)?;
        if call.value >= available {
            return Ok(Estimated::Error("insufficient funds for transfer".into()));
        }
        available -= call.value;
        let allowance = available / U256::from(fee_cap);

        if let Ok(allowance) = u64::try_from(allowance) {
            if hi > allowance {
                hi = allowance;
            }
        }
    }

    if hi > GAS_CAP {
        hi = GAS_CAP;
    }


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



fn bisect(
    run: &Run,
    reads: &mut Reads,
    result: &Exec,
    peak: &dyn Fn(&Exec) -> u64,
    mut hi: u64,
) -> Result<Estimated, Estimated> {
    let execute = |gas: u64, measure: bool, reads: &mut Reads| execute(run, gas, measure, reads);


    let mut lo = result.used.saturating_sub(1);


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

    while lo + 1 < hi {


        if (hi - lo) as f64 / (hi as f64) < ERROR_RATIO {
            break;
        }
        let mut mid = lo + (hi - lo) / 2;
        if mid > lo * 2 {

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

    let call_gas = call.gas.map_or(0, |g| g.min(GAS_CAP));

    let posting = run.nitro_posting(reads).map_err(|r| match r {
        Ran::Missing(m) => Estimated::Missing(m),
        Ran::Refused(m) | Ran::Internal(m) => Estimated::Internal(m),
        other => Estimated::Internal(format!("the posting gas came to {other:?}")),
    })?;
    let gas_cap = GAS_CAP.wrapping_add(posting);


    let mut hi = run.overrides.block.gas_limit.unwrap_or(block.gas_limit);
    if call_gas >= TX_GAS {
        hi = call_gas;
    }

    let fee_cap = call.gas_price.or(call.max_fee_per_gas).unwrap_or(0);

    if fee_cap != 0 {
        let mut available = balance(run, call.from, reads)?;
        if call.value >= available {
            return Ok(Estimated::Error("insufficient funds for transfer".into()));
        }
        available -= call.value;
        let allowance = available / U256::from(fee_cap);

        if let Ok(allowance) = u64::try_from(allowance) {
            if hi > allowance {
                hi = allowance;
            }
        }
    }

    if gas_cap != 0 && hi > gas_cap {
        hi = gas_cap;
    }


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
