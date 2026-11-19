







use crate::call::{self, Engine, Exec, Missing, Ran, Reads, Run};

use crate::proof::Proven;
use alloy_primitives::{Address, Bytes, U256};
use revm::primitives::hardfork::SpecId;
use std::collections::BTreeMap;


const TX_GAS: u64 = 21_000;

const CALL_STIPEND: u64 = 2_300;

const MAX_TX_GAS: u64 = 1 << 24;

const ERROR_RATIO: f64 = 0.015;

const PRE_LONDON: &str =
    "maxFeePerGas and maxPriorityFeePerGas are not valid before London is active";
const NO_FUNDS: &str = "insufficient funds for transfer";
const NOT_ON_NITRO: &str = "eth_estimateGas with a difficulty or prevRandao block override is not run on Robinhood Chain here";


pub fn estimated_on(engine: Engine) -> bool {
    match engine {
        Engine::Ethereum | Engine::Op(_) | Engine::Nitro(_) => true,
    }
}


pub fn not_searched(req: &crate::CallRequest) -> Option<String> {
    let b = &req.overrides.block;
    let sets_randomness = b.difficulty.is_some() || b.random.is_some();
    (req.engine.is_nitro() && sets_randomness).then(|| NOT_ON_NITRO.to_string())
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
























pub fn estimate_run(run: &Run, reads: &mut Reads) -> Estimated {
    search(run, reads, &|x: &Exec| x.peak).unwrap_or_else(|aborted| aborted)
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
    match find(run, reads, peak) {
        Ok(gas) => Ok(Estimated::Gas(gas)),
        Err(Stop::Output(e)) => Ok(e),
        Err(Stop::Abort(e)) => Err(e),
    }
}


enum Stop {

    Output(Estimated),

    Abort(Estimated),
}

fn error(text: impl Into<String>) -> Stop {
    Stop::Output(Estimated::Error(text.into()))
}


enum Class {
    Succeeded,
    Failed,

    Bailed(String),
    Aborted(Estimated),
}

fn classify(x: &Exec) -> Class {
    match &x.ran {
        Ran::Returned(_) => Class::Succeeded,
        Ran::Reverted(_) | Ran::RevertedSaying(..) | Ran::Halted(_) => Class::Failed,
        Ran::Invalid(_) if x.intrinsic => Class::Failed,
        Ran::Invalid(why) => Class::Bailed(why.clone()),
        Ran::Missing(m) => Class::Aborted(Estimated::Missing(m.clone())),
        Ran::NoBlockHash(n) => Class::Aborted(Estimated::NoBlockHash(*n)),
        Ran::Refused(why) | Ran::Internal(why) => Class::Aborted(Estimated::Internal(why.clone())),
    }
}


fn find(run: &Run, reads: &mut Reads, peak: &dyn Fn(&Exec) -> u64) -> Result<u64, Stop> {
    let mut hi = upper_bound(run, reads)?;
    if let Some(gas) = transfer_probe(run, reads)? {
        return Ok(gas);
    }


    let x = run.at_gas(hi, reads, true);
    let (used, top) = match classify(&x) {
        Class::Succeeded => (x.used, peak(&x)),
        Class::Failed => return Err(Stop::Output(failure_at_bound(x, hi))),
        Class::Bailed(why) => return Err(error(format!("failed with {hi} gas: {why}"))),
        Class::Aborted(e) => return Err(Stop::Abort(e)),
    };
    let mut lo = used.saturating_sub(1);


    let optimistic = top.saturating_add(CALL_STIPEND).saturating_mul(64) / 63;
    if optimistic < hi {
        if succeeds(run, optimistic, reads)? {
            hi = optimistic;
        } else {
            lo = optimistic;
        }
    }


    while lo + 1 < hi {
        if ((hi - lo) as f64) / (hi as f64) < ERROR_RATIO {
            break;
        }
        let mid = (lo + (hi - lo) / 2).min(lo.saturating_mul(2));
        if succeeds(run, mid, reads)? {
            hi = mid;
        } else {
            lo = mid;
        }
    }
    Ok(hi)
}


fn failure_at_bound(x: Exec, hi: u64) -> Estimated {
    let out_of_gas = x.out_of_gas;
    match x.ran {
        Ran::Reverted(data) if !out_of_gas => Estimated::Reverted(data),
        Ran::RevertedSaying(data, reason) if !out_of_gas => Estimated::RevertedSaying(data, reason),
        Ran::Halted(m) if !out_of_gas => Estimated::Error(call::halt_message(&m)),
        _ => Estimated::Error(format!("gas required exceeds allowance ({hi})")),
    }
}


fn succeeds(run: &Run, gas: u64, reads: &mut Reads) -> Result<bool, Stop> {
    let x = run.at_gas(gas, reads, false);
    match classify(&x) {
        Class::Succeeded => Ok(true),
        Class::Failed => Ok(false),
        Class::Bailed(why) => Err(error(format!("failed with {gas} gas: {why}"))),
        Class::Aborted(e) => Err(Stop::Abort(e)),
    }
}


fn held<'w>(run: &Run<'w>, reads: &mut Reads, address: Address) -> Result<&'w Proven, Stop> {
    match run.witness.account(&address) {
        Some(account) => {
            reads.accounts.entry(address).or_default();
            Ok(account)
        }
        None => Err(Stop::Abort(Estimated::Missing(Missing {
            accounts: BTreeMap::from([(address, Default::default())]),
            slots: BTreeMap::new(),
        }))),
    }
}


fn upper_bound(run: &Run, reads: &mut Reads) -> Result<u64, Stop> {
    let (call, block) = (run.call, run.block);


    let nitro_cap = match run.engine {
        Engine::Nitro(_) => {
            let posting = run.nitro_posting(reads).map_err(|ran| {
                Stop::Abort(match ran {
                    Ran::Missing(m) => Estimated::Missing(m),
                    Ran::Refused(why) | Ran::Internal(why) => Estimated::Internal(why),
                    other => Estimated::Internal(format!("the posting gas came to {other:?}")),
                })
            })?;
            Some(call::GAS_CAP.wrapping_add(posting))
        }
        Engine::Ethereum | Engine::Op(_) => {
            let names_1559 =
                call.max_fee_per_gas.is_some() || call.max_priority_fee_per_gas.is_some();
            if block.base_fee.is_none() && names_1559 {
                return Err(error(PRE_LONDON));
            }
            None
        }
    };

    let named = call.gas.map_or(0, |g| g.min(call::GAS_CAP));
    let mut hi = run.overrides.block.gas_limit.unwrap_or(block.gas_limit);
    if named >= TX_GAS {
        hi = named;
    }
    let osaka = call::spec(block.number, block.timestamp).is_enabled_in(SpecId::OSAKA);
    if run.engine == Engine::Ethereum && hi > MAX_TX_GAS && osaka {
        hi = MAX_TX_GAS;
    }


    let price = call.gas_price.or(call.max_fee_per_gas).unwrap_or(0);
    if price != 0 {
        let balance = sender_balance(run, reads)?;
        if call.value >= balance {
            return Err(error(NO_FUNDS));
        }
        let allowance = (balance - call.value) / U256::from(price);
        if allowance <= U256::from(u64::MAX) {
            hi = hi.min(allowance.saturating_to::<u64>());
        }
    }

    match nitro_cap {
        None => hi = hi.min(call::GAS_CAP),
        Some(cap) if cap != 0 && hi > cap => hi = cap,
        Some(_) => {}
    }
    Ok(hi)
}


fn sender_balance(run: &Run, reads: &mut Reads) -> Result<U256, Stop> {
    let from = run.call.from;
    if let Some(balance) = run.overrides.state.get(&from).and_then(|o| o.balance) {
        return Ok(balance);
    }
    let account = held(run, reads, from)?;
    Ok(if account.exists {
        account.balance
    } else {
        U256::ZERO
    })
}



fn transfer_probe(run: &Run, reads: &mut Reads) -> Result<Option<u64>, Stop> {
    let call = run.call;
    let Some(to) = call.to else {
        return Ok(None);
    };
    if !call.data.is_empty() {
        return Ok(None);
    }
    let has_code = match run.overrides.state.get(&to).and_then(|o| o.code.as_ref()) {
        Some(code) => !code.is_empty(),
        None => held(run, reads, to)?.has_code(),
    };
    if has_code {
        return Ok(None);
    }
    let x = run.at_gas(TX_GAS, reads, false);
    match classify(&x) {
        Class::Succeeded if run.engine.is_nitro() => Ok(Some(TX_GAS)),
        Class::Succeeded => Ok(Some(x.used)),
        Class::Failed | Class::Bailed(_) => Ok(None),
        Class::Aborted(e) => Err(Stop::Abort(e)),
    }
}
