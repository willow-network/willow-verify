//! `eth_estimateGas` on Ethereum: geth v1.17.10's `Estimate` (eth/gasestimator/gasestimator.go),
//! line for line, with the call as `DoEstimateGas` and `CallDefaults` (internal/ethapi) make it,
//! each run the same EVM as `eth_call` ([`call::execute`]). The search is deterministic over the
//! witness, so a client that runs it over the proof's witness gets the very number Willow sent.

use crate::call::{self, Block, Call, Exec, Missing, Mode, Ran, Reads, Witness, GAS_CAP};
use alloy_primitives::{Address, Bytes, B256, U256};
use revm::primitives::hardfork::SpecId;

/// `params.TxGas`, `params.CallStipend`, `params.MaxTxGas` (EIP-7825).
const TX_GAS: u64 = 21_000;
const CALL_STIPEND: u64 = 2_300;
const MAX_TX_GAS: u64 = 1 << 24;
/// `estimateGasErrorRatio` (internal/ethapi/api.go).
const ERROR_RATIO: f64 = 0.015;

/// What the search came to.
#[derive(Clone, Debug, PartialEq)]
pub enum Estimated {
    Gas(u64),
    /// Reverted at the most gas it may have: the revert data.
    Reverted(Bytes),
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

/// The lowest gas limit at which `call` runs at `block` without failing, as geth searches for it.
pub fn estimate(
    witness: &Witness,
    block: &Block,
    call: &Call,
    hash_of: &dyn Fn(u64) -> Option<B256>,
    mode: Mode,
    reads: &mut Reads,
) -> Estimated {
    let peak = |x: &Exec| x.peak;
    search(witness, block, call, hash_of, mode, reads, &peak).unwrap_or_else(|e| e)
}

/// The search, with geth's `MaxUsedGas` of a run taken by `peak` (the tests try another mapping).
pub(crate) fn search(
    witness: &Witness,
    block: &Block,
    call: &Call,
    hash_of: &dyn Fn(u64) -> Option<B256>,
    mode: Mode,
    reads: &mut Reads,
    peak: &dyn Fn(&Exec) -> u64,
) -> Result<Estimated, Estimated> {
    let execute = |gas: u64, measure: bool, reads: &mut Reads| -> Result<Executed, Estimated> {
        let x = call::execute(witness, block, call, gas, hash_of, mode, reads, measure);
        let invalid = match &x.ran {
            Ran::Missing(m) => return Err(Estimated::Missing(m.clone())),
            Ran::NoBlockHash(n) => return Err(Estimated::NoBlockHash(*n)),
            Ran::Internal(m) => return Err(Estimated::Internal(m.clone())),
            Ran::Invalid(m) => Some(m.clone()),
            Ran::Returned(_) | Ran::Reverted(_) | Ran::Halted(_) => None,
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
    };
    let spec = call::spec(block.number, block.timestamp);
    // CallDefaults: no 1559 fields before London (geth: the header has no base fee).
    let london = block.base_fee.is_some();
    if !london && (call.max_fee_per_gas.is_some() || call.max_priority_fee_per_gas.is_some()) {
        return Ok(Estimated::Error(
            "maxFeePerGas and maxPriorityFeePerGas are not valid before London is active".into(),
        ));
    }
    // DoEstimateGas: a gas not given is 0; CallDefaults caps one given at the RPC gas cap.
    let call_gas = call.gas.map_or(0, |g| g.min(GAS_CAP));

    // Determine the highest gas limit can be used during the estimation.
    let mut hi = block.gas_limit;
    if call_gas >= TX_GAS {
        hi = call_gas;
    }
    // Cap the maximum gas allowance according to EIP-7825 if the estimation targets Osaka (the
    // fork table has no Amsterdam).
    if hi > MAX_TX_GAS && spec.is_enabled_in(SpecId::OSAKA) {
        hi = MAX_TX_GAS;
    }
    // Normalize the max fee per gas the call is willing to spend (ToMessage: gasPrice, else
    // maxFeePerGas, else zero).
    let fee_cap = call.gas_price.or(call.max_fee_per_gas).unwrap_or(0);
    // Recap the highest gas limit with account's available balance.
    if fee_cap != 0 {
        let balance = account(witness, call.from, reads)?.map_or(U256::ZERO, |p| p.balance);
        let mut available = balance;
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
            let code = account(witness, to, reads)?.is_some_and(|p| p.has_code());
            if !code {
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
        if let Some(r) = &x.result {
            if !r.out_of_gas {
                return Ok(match &r.ran {
                    Ran::Reverted(data) => Estimated::Reverted(data.clone()),
                    Ran::Halted(m) => Estimated::Error(call::halt_message(m)),
                    other => Estimated::Internal(format!("a failed run came to {other:?}")),
                });
            }
        }
        return Ok(Estimated::Error(format!(
            "gas required exceeds allowance ({hi})"
        )));
    }
    let Some(result) = x.result else {
        return Ok(Estimated::Internal(
            "a run that did not fail has no result".into(),
        ));
    };
    // For almost any transaction, the gas consumed by the unconstrained execution above
    // lower-bounds the gas limit required for it to succeed.
    let mut lo = result.used.saturating_sub(1);
    // There's a fairly high chance for the transaction to execute successfully with gasLimit set
    // to the first execution's usedGas + gasRefund.
    let optimistic = (peak(&result) + CALL_STIPEND) * 64 / 63;
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
