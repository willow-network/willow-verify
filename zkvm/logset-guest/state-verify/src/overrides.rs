








use alloy_primitives::{Address, U256};
pub use alloy_rpc_types_eth::{state::StateOverride, BlockOverrides};
use revm::state::Bytecode;
use serde_json::Value;
use std::collections::HashSet;


pub const MAX_ACCOUNTS: usize = 256;
pub const MAX_SLOTS: usize = 8_192;
pub const MAX_CODE_BYTES: usize = 512 << 10;


#[derive(Clone, Debug, Default, PartialEq)]
pub struct Overrides {
    pub state: StateOverride,
    pub block: BlockOverrides,

    pub blob_base_fee: Option<U256>,
}

impl Overrides {
    pub fn is_empty(&self) -> bool {
        self.state.is_empty() && self.block.is_empty() && self.blob_base_fee.is_none()
    }


    pub fn whole_accounts(&self) -> HashSet<Address> {
        self.state
            .iter()
            .filter(|(_, o)| o.nonce.is_some() && o.balance.is_some() && o.code.is_some())
            .map(|(a, _)| *a)
            .collect()
    }


    pub fn own_code(&self) -> HashSet<Address> {
        self.state
            .iter()
            .filter(|(_, o)| o.code.is_some())
            .map(|(a, _)| *a)
            .collect()
    }



















}



#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Unread {
    Invalid(String),
    TooLarge(String),
    NotServed(String),
}



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
