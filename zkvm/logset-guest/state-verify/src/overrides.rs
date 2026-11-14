//! A call's state and block overrides (`eth_call`'s and `eth_estimateGas`'s third and fourth
//! parameters), read as reth reads them. What they set is the caller's own input: used as given,
//! never proven, never sent in a proof (the client has them). Everything else a call reads is
//! proven against the block's state root: an account whose storage is replaced (`state`) has no
//! slot proven, one whose code is set may leave its own code out, and its nonce and balance are
//! proven unless set too (all three set: proven only for slots the call reads and doesn't set).
//! The log RPC's `eth_call` reads them with [`parse`] too, so a proof and the standard response
//! read one request alike.

use alloy_primitives::{Address, B256, U256};
pub use alloy_rpc_types_eth::{state::StateOverride, BlockOverrides};
use revm::state::Bytecode;
use serde_json::Value;
use std::collections::HashSet;

/// Most accounts, slots and bytes of code across them one call's overrides may set.
pub const MAX_ACCOUNTS: usize = 256;
pub const MAX_SLOTS: usize = 8_192;
pub const MAX_CODE_BYTES: usize = 512 << 10;

/// A call's overrides: none when it names none.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Overrides {
    pub state: StateOverride,
    pub block: BlockOverrides,
    /// `blockOverrides.blobBaseFee`, which reth reads and this alloy's `BlockOverrides` lacks.
    pub blob_base_fee: Option<U256>,
}

impl Overrides {
    pub fn is_empty(&self) -> bool {
        self.state.is_empty() && self.block.is_empty() && self.blob_base_fee.is_none()
    }

    /// Accounts whose nonce, balance and code are all the caller's: their proof is never read.
    pub fn whole_accounts(&self) -> HashSet<Address> {
        self.state
            .iter()
            .filter(|(_, o)| o.nonce.is_some() && o.balance.is_some() && o.code.is_some())
            .map(|(a, _)| *a)
            .collect()
    }

    /// Accounts whose code is the caller's: their own code is never asked for or checked.
    pub fn own_code(&self) -> HashSet<Address> {
        self.state
            .iter()
            .filter(|(_, o)| o.code.is_some())
            .map(|(a, _)| *a)
            .collect()
    }

    /// Whether the call reads `slot` of `address` from the caller, not the chain.
    pub fn sets_slot(&self, address: &Address, slot: &B256) -> bool {
        self.state.get(address).is_some_and(|o| {
            o.state.is_some() || o.state_diff.as_ref().is_some_and(|d| d.contains_key(slot))
        })
    }

    /// Whether every account field and slot of `address` is the caller's.
    pub fn sets_all_of(&self, address: &Address) -> bool {
        self.state.get(address).is_some_and(|o| {
            o.nonce.is_some() && o.balance.is_some() && o.code.is_some() && o.state.is_some()
        })
    }

    /// The fee recipient the call runs with.
    pub fn coinbase(&self, chain: Address) -> Address {
        self.block.coinbase.unwrap_or(chain)
    }
}

/// Why overrides are not run: not valid (-32602), more than one call may set (a router asks
/// elsewhere), or of a kind not run here.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Unread {
    Invalid(String),
    TooLarge(String),
    NotServed(String),
}

/// A call's overrides from its third (`state`) and fourth (`block`) parameters; absent or null
/// is none.
pub fn parse(state: &Value, block: &Value) -> Result<Overrides, Unread> {
    let state: StateOverride = match state {
        Value::Null => StateOverride::default(),
        v => serde_json::from_value(v.clone())
            .map_err(|e| Unread::Invalid(format!("invalid state override: {e}")))?,
    };
    let slots: usize = state
        .values()
        .map(|o| {
            o.state.as_ref().map_or(0, |s| s.len()) + o.state_diff.as_ref().map_or(0, |s| s.len())
        })
        .sum();
    let code: usize = state
        .values()
        .map(|o| o.code.as_ref().map_or(0, |c| c.len()))
        .sum();
    if state.len() > MAX_ACCOUNTS || slots > MAX_SLOTS || code > MAX_CODE_BYTES {
        return Err(Unread::TooLarge(format!(
            "eth_call overrides: at most {MAX_ACCOUNTS} accounts, {MAX_SLOTS} slots and \
             {MAX_CODE_BYTES} bytes of code are run here in one call"
        )));
    }
    let mut ordered: Vec<_> = state.iter().collect();
    ordered.sort_by_key(|(a, _)| **a);
    for (address, o) in ordered {
        if o.state.is_some() && o.state_diff.is_some() {
            return Err(Unread::Invalid(format!(
                "account {address} has both 'state' and 'stateDiff'"
            )));
        }
        if let Some(code) = &o.code {
            Bytecode::new_raw_checked(code.clone())
                .map_err(|e| Unread::Invalid(format!("Invalid bytecode: {e}")))?;
        }
        // Reth's eth_call ignores it; geth's moves the precompile. Neither is guessed at here.
        if o.move_precompile_to.is_some() {
            return Err(Unread::NotServed(
                "eth_call with movePrecompileToAddress: such calls are not run here".into(),
            ));
        }
    }
    let (block, blob_base_fee) = match block {
        Value::Null => (BlockOverrides::default(), None),
        Value::Object(o) => {
            let mut o = o.clone();
            let blob_base_fee = o
                .remove("blobBaseFee")
                .filter(|v| !v.is_null())
                .map(|v| crate::proof::quantity(&v))
                .transpose()
                .map_err(|e| Unread::Invalid(format!("invalid block override: {e}")))?;
            // Read by reth's eth_simulateV1 only; an eth_call leaves them be.
            for k in ["beaconRoot", "parentBeaconBlockRoot", "withdrawals"] {
                o.remove(k);
            }
            let block = serde_json::from_value(Value::Object(o))
                .map_err(|e| Unread::Invalid(format!("invalid block override: {e}")))?;
            (block, blob_base_fee)
        }
        v => return Err(Unread::Invalid(format!("invalid block override: {v}"))),
    };
    Ok(Overrides {
        state,
        block,
        blob_base_fee,
    })
}
