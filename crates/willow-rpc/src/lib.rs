//! A verifying client for Willow's RPC. For every request it fetches a proof, verifies it on your
//! machine with the same verifiers the verify page at <https://api.willow.tech/verify/> runs, ties
//! the block to its chain with a light client, and returns the standard JSON-RPC response built
//! from the proven data alone. Beyond the light clients' starting points, it trusts no server
//! or node.
//!
//! ```no_run
//! # async fn run() -> Result<(), willow_rpc::Error> {
//! use serde_json::json;
//! use willow_rpc::{Config, Verifier};
//!
//! let willow = Verifier::new(Config::new(1).key("<your API key>"))?;
//! let block = willow.request("eth_getBlockByNumber", json!(["latest", false])).await?;
//! # Ok(()) }
//! ```
//!
//! With alloy (feature `alloy`, on by default), [`alloy::WillowTransport`] is a transport:
//!
//! ```ignore
//! use alloy_provider::{Provider, ProviderBuilder};
//! use alloy_rpc_client::RpcClient;
//! use willow_rpc::{alloy::WillowTransport, Config};
//!
//! let transport = WillowTransport::new(Config::new(8453).key(key))?;
//! let provider = ProviderBuilder::new().connect_client(RpcClient::new(transport, false));
//! let balance = provider.get_balance(address).await?; // verified, or an error
//! ```
//!
//! The client needs a tokio runtime. Errors are [`Error`]: -32098 when a proof failed, -32099
//! when a value could not be verified, and the server's own JSON-RPC errors passed on as sent.

mod anchor;
pub mod checkpoint;
mod client;
mod config;
mod error;
mod hex;
mod http;
mod lru;
pub mod storage;
mod tx;
mod verifiers;

#[cfg(any(feature = "alloy", feature = "alloy-v1"))]
pub mod alloy;

pub use checkpoint::{BnbStart, Checkpoint};
pub use client::{
    BLOCK_FILTER_MAX, FEE_HISTORY_WITH_REWARDS, LOGS_BY_RECEIPTS, TRACE_FILTER_BLOCKS, VERSION,
};
pub use config::{
    AnchoredBy, Anchoring, BlockRef, Config, Event, Flag, Level, OpenVmVerifier, Status,
    StorageChoice, Unverifiable,
};
pub use error::{Error, ErrorData, UNVERIFIABLE, VERIFICATION_FAILED};
pub use http::MAX_BATCH;

use serde_json::Value;
use std::sync::Arc;

/// The verifying client for one chain. Cheap to clone; clones share caches, light clients and
/// the work in flight and HTTP connections, across runtimes too: a current-thread runtime that
/// isn't being driven can hold up other runtimes' requests, so runtimes that run on their own
/// should each have their own `Verifier`.
#[derive(Clone)]
pub struct Verifier {
    inner: Arc<client::Client>,
}

impl std::fmt::Debug for Verifier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Verifier")
            .field("chain_id", &self.inner.chain_id())
            .finish_non_exhaustive()
    }
}

impl Verifier {
    /// A client from `config`. Fails only on a config it can't use (a chain id of 0 or over 32
    /// bits, a URL that names no chain and no light-client URL).
    pub fn new(config: Config) -> Result<Verifier, Error> {
        Ok(Verifier {
            inner: client::Client::new(config)?,
        })
    }

    pub fn chain_id(&self) -> u64 {
        self.inner.chain_id()
    }

    /// One JSON-RPC request: the standard `result`, verified, or an error. `params` is the
    /// request's params (an array, or `null` for none).
    pub async fn request(&self, method: &str, params: Value) -> Result<Value, Error> {
        self.inner.request(method, params).await
    }
}

#[cfg(test)]
mod tests;
