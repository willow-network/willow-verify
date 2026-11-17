//! What a [`Verifier`](crate::Verifier) is made from, and the event it reports per response.

use crate::checkpoint::{BnbStart, Checkpoint};
use crate::storage::Storage;
use std::fmt;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

/// An anchor's level, lowest first: `signed` (by the chain's validators or sequencer), then
/// `committed` (an L2 block claimed on its L1), then `final`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Level {
    Signed,
    Committed,
    Final,
}

impl Level {
    pub fn as_str(self) -> &'static str {
        match self {
            Level::Signed => "signed",
            Level::Committed => "committed",
            Level::Final => "final",
        }
    }

    pub fn parse(s: &str) -> Option<Level> {
        match s {
            "signed" => Some(Level::Signed),
            "committed" => Some(Level::Committed),
            "final" => Some(Level::Final),
            _ => None,
        }
    }
}

impl fmt::Display for Level {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// What happens to a value the client can't verify.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Unverifiable {
    /// Raise [`Error::Unverifiable`](crate::Error::Unverifiable) (-32099). The default.
    #[default]
    Throw,
    /// Return the server's value, marked in the event (`status` `unverifiable`, with `flags`); for
    /// a log filter, the server's `eth_getLogs` over the same blocks. A block filter has no server
    /// value, so a block proof it can't get raises as under `Throw`.
    Flag,
}

/// How a proven block is tied to its chain.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum Anchoring {
    /// Willow's light clients, from the checkpoint (the default): nothing is trusted but the
    /// starting point.
    #[default]
    LightClient,
    /// Compare each proven block's hash with a node you trust (its `eth_getBlockByNumber`), and
    /// take `latest`, `finalized` and `safe` from it. Nothing is anchored by the light clients.
    Rpc(String),
    /// Leave block hashes unchecked: a proof shows a value is in a block, not that the block is
    /// the chain's. Every event says so (`anchored_by` [`AnchoredBy::None`]).
    None,
}

/// Which anchoring an event's block rests on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AnchoredBy {
    LightClient,
    Rpc,
    None,
}

/// Where the light clients' committees are kept between runs.
#[derive(Clone, Default)]
pub enum StorageChoice {
    /// `~/.cache/willow-rpc/` (the default).
    #[default]
    Default,
    /// Nowhere: every run starts from the checkpoint.
    None,
    Custom(Arc<dyn Storage>),
}

/// Where OpenVM log proofs are verified.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum OpenVmVerifier {
    /// `willow-logs-verify-openvm` next to this executable, else on `PATH` (the default).
    #[default]
    Default,
    Path(PathBuf),
    /// Nowhere: an OpenVM-proven log range is unverifiable (`no-verifier`).
    None,
}

/// A block a response rests on.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BlockRef {
    pub number: u64,
    pub hash: String,
}

/// What an event says of its response.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    /// Built from verified proofs.
    Verified,
    /// Served by the client itself.
    Local,
    /// A null or not-found passed on.
    Absent,
    /// A server error passed on.
    Refused,
    /// Passed with `flags`, under [`Unverifiable::Flag`].
    Unverifiable,
    /// An error was raised.
    Failed,
}

impl Status {
    pub fn as_str(self) -> &'static str {
        match self {
            Status::Verified => "verified",
            Status::Local => "local",
            Status::Absent => "absent",
            Status::Refused => "refused",
            Status::Unverifiable => "unverifiable",
            Status::Failed => "failed",
        }
    }
}

/// Why a response was unverifiable, or failed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Flag {
    pub kind: String,
    pub detail: Option<String>,
}

/// One per response: what was verified, at which block and level, or why not.
#[derive(Clone, Debug)]
pub struct Event {
    pub method: String,
    pub status: Status,
    pub chain_id: u64,
    /// The block the response was verified at (the newest, when several).
    pub block: Option<BlockRef>,
    /// The lowest anchor level the response rests on (light-client anchoring only).
    pub level: Option<Level>,
    pub anchored_by: AnchoredBy,
    /// Why it is unverifiable or failed: the first reason (`kind`, `detail`), and all of them.
    pub kind: Option<String>,
    pub detail: Option<String>,
    pub flags: Vec<Flag>,
    /// Fields the proofs do not cover, left out of the response.
    pub unproven: Vec<String>,
    pub elapsed: Duration,
}

pub(crate) type OnVerification = Arc<dyn Fn(&Event) + Send + Sync>;

/// The client's options. `Config::new(chain_id)` and the builder methods; every default is the
/// npm client's.
#[derive(Clone)]
pub struct Config {
    pub(crate) chain_id: u64,
    pub(crate) key: Option<String>,
    pub(crate) url: Option<String>,
    pub(crate) light_client_url: Option<String>,
    pub(crate) min_level: Level,
    pub(crate) unverifiable: Unverifiable,
    pub(crate) on_verification: Option<OnVerification>,
    pub(crate) head_max_age: Option<Duration>,
    pub(crate) max_head_age: Duration,
    pub(crate) checkpoint: Option<Checkpoint>,
    pub(crate) checkpoint_url: Option<String>,
    pub(crate) sepolia_checkpoint: Option<Checkpoint>,
    pub(crate) bnb_start: Option<BnbStart>,
    pub(crate) beacon_apis: Option<Vec<String>>,
    pub(crate) storage: StorageChoice,
    pub(crate) batch_window: Duration,
    pub(crate) logs_timeout: Duration,
    pub(crate) anchor_wait: Duration,
    pub(crate) wait_anchor: Option<Duration>,
    pub(crate) wait_jobs: Option<Duration>,
    pub(crate) anchoring: Anchoring,
    pub(crate) openvm: OpenVmVerifier,
    pub(crate) http: Option<reqwest::Client>,
    pub(crate) timeout: Duration,
    pub(crate) now_ms: Option<Arc<dyn Fn() -> u64 + Send + Sync>>,
    #[cfg(test)]
    pub(crate) anchor_factory: Option<crate::verifiers::AnchorFactory>,
    /// Tests: verify what the release holds back (see `client::HELD`), with the repo's verifiers.
    #[cfg(test)]
    pub(crate) unheld: bool,
}

impl fmt::Debug for Config {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Config")
            .field("chain_id", &self.chain_id)
            .field("url", &self.url)
            .field("min_level", &self.min_level)
            .field("unverifiable", &self.unverifiable)
            .field("anchoring", &self.anchoring)
            .finish_non_exhaustive()
    }
}

impl Config {
    /// A client for chain `chain_id`: Ethereum (1); anchored on Ethereum, Base (8453), OP Mainnet
    /// (10), Unichain (130), Ink (57073), Soneium (1868), Zora (7777777), World Chain (480),
    /// Arbitrum One (42161), Robinhood Chain (4663); anchored on Sepolia, OP Sepolia (11155420),
    /// Base Sepolia (84532), Unichain Sepolia (1301), Ink Sepolia (763373), Arbitrum Sepolia
    /// (421614), Robinhood Chain testnet (46630); anchored on its own validators, BNB Smart Chain
    /// (56).
    pub fn new(chain_id: u64) -> Config {
        Config {
            chain_id,
            key: None,
            url: None,
            light_client_url: None,
            min_level: Level::Signed,
            unverifiable: Unverifiable::Throw,
            on_verification: None,
            head_max_age: None,
            max_head_age: Duration::from_secs(600),
            checkpoint: None,
            checkpoint_url: None,
            sepolia_checkpoint: None,
            bnb_start: None,
            beacon_apis: None,
            storage: StorageChoice::Default,
            batch_window: Duration::from_millis(5),
            logs_timeout: Duration::from_secs(120),
            anchor_wait: Duration::from_secs(36),
            wait_anchor: None,
            wait_jobs: None,
            anchoring: Anchoring::LightClient,
            openvm: OpenVmVerifier::Default,
            http: None,
            timeout: Duration::from_secs(120),
            now_ms: None,
            #[cfg(test)]
            anchor_factory: None,
            #[cfg(test)]
            unheld: false,
        }
    }

    /// The API key; without one, the public path, which serves blocks, transactions, receipts,
    /// logs and fees, not state.
    pub fn key(mut self, key: impl Into<String>) -> Config {
        self.key = Some(key.into());
        self
    }

    /// The RPC URL; default `https://api.willow.tech/v1/rpc/{chainId}/{key}`.
    pub fn url(mut self, url: impl Into<String>) -> Config {
        self.url = Some(url.into());
        self
    }

    /// Ethereum's route, for the light client; default the URL with chain 1.
    pub fn light_client_url(mut self, url: impl Into<String>) -> Config {
        self.light_client_url = Some(url.into());
        self
    }

    /// The lowest anchor level a response may rest on; default `signed`.
    pub fn min_level(mut self, level: Level) -> Config {
        self.min_level = level;
        self
    }

    /// What happens to a value that can't be verified; default [`Unverifiable::Throw`].
    pub fn unverifiable(mut self, u: Unverifiable) -> Config {
        self.unverifiable = u;
        self
    }

    /// Called once per response.
    pub fn on_verification(mut self, f: impl Fn(&Event) + Send + Sync + 'static) -> Config {
        self.on_verification = Some(Arc::new(f));
        self
    }

    /// How long the newest block counts before it is anchored again; default Ethereum 12 s,
    /// other chains 2 s.
    pub fn head_max_age(mut self, d: Duration) -> Config {
        self.head_max_age = Some(d);
        self
    }

    /// How old the newest block Willow anchors may be; default 600 s.
    pub fn max_head_age(mut self, d: Duration) -> Config {
        self.max_head_age = d;
        self
    }

    /// Ethereum's light client starts here, in place of the shipped checkpoint.
    pub fn checkpoint(mut self, c: Checkpoint) -> Config {
        self.checkpoint = Some(c);
        self
    }

    /// Where a newer checkpoint is read once the shipped one is too old; default Willow's.
    pub fn checkpoint_url(mut self, url: impl Into<String>) -> Config {
        self.checkpoint_url = Some(url.into());
        self
    }

    /// Sepolia's light client (the testnets) starts here, in place of the shipped start.
    pub fn sepolia_checkpoint(mut self, c: Checkpoint) -> Config {
        self.sepolia_checkpoint = Some(c);
        self
    }

    /// BNB's light client starts at this epoch block, in place of the shipped one.
    pub fn bnb_start(mut self, b: BnbStart) -> Config {
        self.bnb_start = Some(b);
        self
    }

    /// Beacon APIs that must confirm a fetched checkpoint (one confirms, none disagrees); default
    /// two. Empty: a fetched checkpoint is taken unconfirmed.
    pub fn beacon_apis(mut self, apis: Vec<String>) -> Config {
        self.beacon_apis = Some(apis);
        self
    }

    pub fn storage(mut self, s: StorageChoice) -> Config {
        self.storage = s;
        self
    }

    /// Requests within this window go out as one batch; default 5 ms.
    pub fn batch_window(mut self, d: Duration) -> Config {
        self.batch_window = d;
        self
    }

    /// How long `eth_getLogs` waits for Willow to prove blocks; default 120 s.
    pub fn logs_timeout(mut self, d: Duration) -> Config {
        self.logs_timeout = d;
        self
    }

    /// How long a request waits for the anchor of a block newer than the newest one anchored;
    /// default 36 s (three Ethereum slots). Past it the request is unverifiable (`anchor-waiting`).
    pub fn anchor_wait(mut self, d: Duration) -> Config {
        self.anchor_wait = d;
        self
    }

    /// Waits between asks for a block newer than the signed head (default the server's
    /// `retryAfterSeconds`, else 4 s), and between proof-job polls.
    pub fn waits(mut self, anchor: Option<Duration>, jobs: Option<Duration>) -> Config {
        self.wait_anchor = anchor;
        self.wait_jobs = jobs;
        self
    }

    /// How proven blocks are anchored; default the light clients.
    pub fn anchoring(mut self, a: Anchoring) -> Config {
        self.anchoring = a;
        self
    }

    /// Where OpenVM log proofs are verified.
    pub fn openvm_verifier(mut self, o: OpenVmVerifier) -> Config {
        self.openvm = o;
        self
    }

    /// The HTTP client to use (proxies, TLS roots). Gzip should stay on: proofs are large.
    pub fn http_client(mut self, c: reqwest::Client) -> Config {
        self.http = Some(c);
        self
    }

    /// Each HTTP request's timeout, and the OpenVM helper's for starting or for one verdict (past
    /// it the helper is stopped: unavailable at its start, a failed check once it has a proof);
    /// default 120 s.
    pub fn timeout(mut self, d: Duration) -> Config {
        self.timeout = d;
        self
    }

    /// The clock, in Unix milliseconds (tests, replays).
    pub fn clock(mut self, now_ms: impl Fn() -> u64 + Send + Sync + 'static) -> Config {
        self.now_ms = Some(Arc::new(now_ms));
        self
    }

    pub fn chain_id(&self) -> u64 {
        self.chain_id
    }
}
