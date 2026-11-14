//! BNB Smart Chain's forks that change what a header carries or how it is sealed and voted,
//! from `params/config.go` (`BSCChainConfig`, bnb-chain/bsc v1.8.2).

/// BNB Smart Chain mainnet.
pub const BSC_MAINNET: u64 = 56;

/// The fork schedule the rules depend on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Spec {
    pub chain_id: u64,
    /// Validators carry BLS vote keys; headers may carry vote attestations.
    pub luban_block: u64,
    /// A bad vote attestation makes the header invalid (before it, one was ignored).
    pub plato_block: u64,
    /// `baseFeePerGas` (London, with Hertz).
    pub london_block: u64,
    /// `withdrawalsRoot`, `blobGasUsed`, `excessBlobGas` (Cancun, with Haber).
    pub cancun_time: u64,
    /// `parentBeaconBlockRoot` (zero), turn length, per-validator sign counts (BEP-404).
    pub bohr_time: u64,
    /// `requestsHash` (Prague, with Pascal).
    pub prague_time: u64,
    /// Milliseconds in `mixHash`, 1.5 s blocks, 500-block epochs, gas limit bound 1/1024.
    pub lorentz_time: u64,
    /// 0.75 s blocks, 1000-block epochs, sign counts forgotten at finality (BEP-524).
    pub maxwell_time: u64,
    /// 0.45 s blocks; a vote may target any of the last 3 blocks.
    pub fermi_time: u64,
    /// `blockAccessListHash` and `slotNumber`; `None` = not scheduled (they must be absent).
    pub amsterdam_time: Option<u64>,
}

pub const MAINNET: Spec = Spec {
    chain_id: BSC_MAINNET,
    luban_block: 29_020_050,
    plato_block: 30_720_096,
    london_block: 31_302_048,
    cancun_time: 1_718_863_500,
    bohr_time: 1_727_317_200,
    prague_time: 1_742_436_600,
    lorentz_time: 1_745_903_100,
    maxwell_time: 1_751_250_600,
    fermi_time: 1_768_357_800,
    amsterdam_time: None,
};

/// Blocks per epoch before Lorentz, from Lorentz, and from Maxwell.
pub const DEFAULT_EPOCH: u64 = 200;
pub const LORENTZ_EPOCH: u64 = 500;
pub const MAXWELL_EPOCH: u64 = 1000;

/// Milliseconds between blocks, by fork.
pub const DEFAULT_INTERVAL_MS: u64 = 3000;
pub const LORENTZ_INTERVAL_MS: u64 = 1500;
pub const MAXWELL_INTERVAL_MS: u64 = 750;
pub const FERMI_INTERVAL_MS: u64 = 450;

impl Spec {
    pub fn is_luban(&self, number: u64) -> bool {
        number >= self.luban_block
    }
    pub fn is_plato(&self, number: u64) -> bool {
        number >= self.plato_block
    }
    pub fn is_london(&self, number: u64) -> bool {
        number >= self.london_block
    }
    // Every timed fork comes after London, so a block's time alone decides it.
    pub fn is_cancun(&self, time: u64) -> bool {
        time >= self.cancun_time
    }
    pub fn is_bohr(&self, time: u64) -> bool {
        time >= self.bohr_time
    }
    pub fn is_prague(&self, time: u64) -> bool {
        time >= self.prague_time
    }
    pub fn is_lorentz(&self, time: u64) -> bool {
        time >= self.lorentz_time
    }
    pub fn is_maxwell(&self, time: u64) -> bool {
        time >= self.maxwell_time
    }
    pub fn is_fermi(&self, time: u64) -> bool {
        time >= self.fermi_time
    }
    pub fn is_amsterdam(&self, time: u64) -> bool {
        self.amsterdam_time.is_some_and(|t| time >= t)
    }

    /// How many ancestors back a vote's target may be.
    pub fn vote_depth(&self, time: u64) -> u64 {
        if self.is_fermi(time) {
            3
        } else {
            1
        }
    }

    /// Bound divisor of the gas limit's change from the parent's.
    pub fn gas_limit_divisor(&self, time: u64) -> u64 {
        if self.is_lorentz(time) {
            1024
        } else {
            256
        }
    }
}
