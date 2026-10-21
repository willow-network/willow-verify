//! **Per-chain profiles** for the SP1 log-set pipeline: Ethereum, Base and the
//! five fast chains (Arbitrum One, Robinhood Chain, OP Mainnet, Unichain, Ink).
//!
//! One table, one source. It says, per chain:
//!
//! - which receipt types the leaf must accept there (mirrors the guest's
//!   chain-gated lists; [`receipt_gate`]);
//! - the proving geometry: the leaf segment size `S = 2^seg_log2` blocks and the
//!   index scope `Q = 2^index_log2` blocks (the L1 index aggregate). Both are
//!   DERIVED from measured block composition by [`seg_log2_for`] and
//!   [`index_log2_for`], and a test pins every fast-chain row to its rule, so a
//!   number here is never a bare guess;
//! - how the chain's block hashes become provable on Ethereum ([`Anchor`]) and
//!   the measured anchor cadence and lag;
//! - where witness data comes from (an env var naming a private endpoint, else
//!   public read-only endpoints).
//!
//! The planner that turns a profile plus an anchor stream into segments and
//! anchored statements is [`plan`].
//!
//! Measured inputs: 1,000
//! blocks per chain drawn stratified-uniformly over the 24 h before the sample,
//! receipts/logs/topics/data bytes per block, priced with the per-block cycle
//! model fitted on 7,152 Ethereum blocks (R² 0.9996).

pub mod plan;

/// Execution-client family: decides the receipt formats and the anchor shape.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Family {
    /// Ethereum L1.
    Ethereum,
    /// OP-stack L2 (and Base, which left the OP Stack but kept its formats).
    OpStack,
    /// Arbitrum Nitro (Arbitrum One and Orbit chains).
    Orbit,
    /// Stellar: XDR ledgers hashed with SHA-256, its own leaf (`zkvm/stellar`).
    Stellar,
}

/// How a chain's block hash becomes provable against Ethereum.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Anchor {
    /// Ethereum itself: EIP-2935 (fresh) or the beacon-anchor proof + EIP-4788.
    Ethereum,
    /// An OP-stack type-9 SUPER-ROOT dispute game, judged by the chain's
    /// `AnchorStateRegistry.isGameClaimValid`. The game's `l2SequenceNumber` is
    /// an L2 TIMESTAMP; the root claim is `keccak(0x01 ‖ u64 ts ‖ (chainId ‖
    /// outputRoot)*)`.
    OpSuperRoot {
        factory: &'static str,
        registry: &'static str,
    },
    /// An output-root dispute game (Base type 621): `l2SequenceNumber` is the
    /// L2 block NUMBER, the root claim is the output root itself.
    OpOutputRoot {
        factory: &'static str,
        registry: &'static str,
    },
    /// A BoLD rollup's CONFIRMED assertion, whose after-state carries the L2
    /// block hash directly (no L2 block number on L1).
    Bold { rollup: &'static str },
    /// A Stellar anchor proof: a quorum of the pinned SCP quorum set externalized
    /// the next ledger, whose transaction set names the anchored ledger's hash
    /// (`StellarLogSetVerifier`). SCP messages come from the history archive.
    StellarScp { archive: &'static str },
}

/// Receipt types the leaf accepts on a chain, beyond Ethereum's 0x01–0x04 and
/// legacy. Mirrors the guest's chain-gated admission lists (`logset_core::receipt::profile_of`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ReceiptGate {
    /// OP-stack deposit receipts (`0x7E`, Bedrock/Regolith/Canyon tails).
    pub deposits: bool,
    /// Arbitrum Nitro internal and retryable types `0x64..=0x6A` (plain 4-field
    /// receipts behind the type byte).
    pub nitro: bool,
}

/// OP-stack deposit transaction type.
pub const DEPOSIT_TX_TYPE: u8 = 0x7E;
/// Arbitrum Nitro transaction types whose receipts can sit in a Nitro receipts
/// trie: deposit 0x64, unsigned 0x65, contract 0x66, retry 0x68,
/// submit-retryable 0x69, internal 0x6A. (0x67 is unused; 0x78 is the pre-Nitro
/// legacy type, which never appears in a Nitro block.)
pub const NITRO_TX_TYPES: [u8; 6] = [0x64, 0x65, 0x66, 0x68, 0x69, 0x6A];

/// The chains whose receipts tries may hold `0x7E`: Base, OP Mainnet, Unichain,
/// Ink. Byte-identical to the guest's `logset_core::receipt::OP_STACK_CHAIN_IDS`.
pub const OP_STACK_CHAIN_IDS: [u64; 4] = [8453, 10, 130, 57073];
/// The chains whose receipts tries may hold Nitro types: Arbitrum One, Robinhood
/// Chain. Byte-identical to the guest's `logset_core::receipt::NITRO_CHAIN_IDS`.
pub const NITRO_CHAIN_IDS: [u64; 2] = [42161, 4663];

pub fn receipt_gate(chain_id: u64) -> ReceiptGate {
    ReceiptGate {
        deposits: OP_STACK_CHAIN_IDS.contains(&chain_id),
        nitro: NITRO_CHAIN_IDS.contains(&chain_id),
    }
}

/// Measured block composition, the input to the geometry rules.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Composition {
    /// Blocks per day, from head timestamps.
    pub blocks_per_day: u64,
    pub receipts_per_block: f64,
    pub logs_per_block: f64,
    pub topics_per_block: f64,
    pub data_bytes_per_block: f64,
}

impl Composition {
    /// Mean leaf cycles per block, from the Ethereum-fitted model
    /// `6,433 + 11,945·r + 2,489·l + 815·t + 17.4·d` (applied to per-block means;
    /// the model is linear, so the mean of the model is the model of the means).
    pub fn model_cycles_per_block(&self) -> f64 {
        6_433.0
            + 11_945.0 * self.receipts_per_block
            + 2_489.0 * self.logs_per_block
            + 815.0 * self.topics_per_block
            + 17.4 * self.data_bytes_per_block
    }

    /// Seconds per block.
    pub fn block_secs(&self) -> f64 {
        86_400.0 / self.blocks_per_day as f64
    }
}

/// A leaf segment should carry at least this many cycles on average, so the
/// ~2.7 s fixed cost of one SP1 proof (plus ~1 s of recursion when the index
/// aggregator verifies it) stays near or under 5% of its GPU time.
pub const SEG_TARGET_CYCLES: f64 = 256e6;
/// …but a segment never waits longer than this to fill (freshness of the
/// batched off-chain product).
pub const SEG_MAX_FILL_SECS: f64 = 1_200.0;
/// An index tree (L1 aggregate) should hold about this many logs: the size the
/// Ethereum design runs at (64 blocks ≈ 55K logs)…
pub const INDEX_TARGET_LOGS: f64 = 65_536.0;
/// …but never span more than an hour of chain.
pub const INDEX_MAX_SECS: f64 = 3_600.0;
/// The L1 index aggregator's child cap.
pub const MAX_L1_CHILDREN_LOG2: u8 = 6;

/// `S`: the smallest power of two whose mean segment carries
/// [`SEG_TARGET_CYCLES`], halved while it would take longer than
/// [`SEG_MAX_FILL_SECS`] to fill.
pub fn seg_log2_for(c: &Composition) -> u8 {
    let per_block = c.model_cycles_per_block();
    let mut k: u8 = 0;
    while (per_block * (1u64 << k) as f64) < SEG_TARGET_CYCLES && k < 24 {
        k += 1;
    }
    while k > 0 && (1u64 << k) as f64 * c.block_secs() > SEG_MAX_FILL_SECS {
        k -= 1;
    }
    k
}

/// `Q`: the smallest power of two holding [`INDEX_TARGET_LOGS`] on average,
/// halved while it spans more than [`INDEX_MAX_SECS`], and kept within
/// `S ≤ Q ≤ 64·S`.
pub fn index_log2_for(c: &Composition, seg_log2: u8) -> u8 {
    let mut k: u8 = 0;
    while (c.logs_per_block * (1u64 << k) as f64) < INDEX_TARGET_LOGS && k < 30 {
        k += 1;
    }
    while k > 0 && (1u64 << k) as f64 * c.block_secs() > INDEX_MAX_SECS {
        k -= 1;
    }
    k.clamp(seg_log2, seg_log2 + MAX_L1_CHILDREN_LOG2)
}

/// One chain's row.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ChainProfile {
    pub chain_id: u64,
    pub name: &'static str,
    pub family: Family,
    /// Measured composition (None where no sample was taken).
    pub measured: Option<Composition>,
    /// `S = 2^seg_log2` blocks per leaf proof.
    pub seg_log2: u8,
    /// `Q = 2^index_log2` blocks per L1 index aggregate.
    pub index_log2: u8,
    pub anchor: Anchor,
    /// Mean L2 blocks between consecutive anchorable blocks (measured on L1).
    pub anchor_every_blocks: u64,
    /// Mean seconds from an L2 block to its anchor becoming usable on L1
    /// (confirmed assertion / valid game), measured.
    pub anchor_lag_secs: u64,
    /// Env var naming a private endpoint (comma-separated URLs; first used).
    /// Never rendered.
    pub rpc_env: &'static str,
    /// Public read-only endpoints that serve `eth_getBlockReceipts`, used when
    /// `rpc_env` is unset.
    pub public_rpc: &'static [&'static str],
}

impl ChainProfile {
    pub fn seg_blocks(&self) -> u64 {
        1 << self.seg_log2
    }
    pub fn index_blocks(&self) -> u64 {
        1 << self.index_log2
    }
    pub fn receipt_gate(&self) -> ReceiptGate {
        receipt_gate(self.chain_id)
    }
}

pub const ETHEREUM: u64 = 1;
pub const OP_MAINNET: u64 = 10;
pub const UNICHAIN: u64 = 130;
pub const BASE: u64 = 8453;
pub const ARBITRUM_ONE: u64 = 42161;
pub const ROBINHOOD: u64 = 4663;
pub const INK: u64 = 57073;
/// Stellar pubnet: the first 8 bytes of `sha256(network passphrase)`.
pub const STELLAR: u64 = 0x7ac3_3997_544e_3175;

const ROBINHOOD_2026_10_01: Composition = Composition {
    blocks_per_day: 858_772,
    receipts_per_block: 9.82,
    logs_per_block: 37.03,
    topics_per_block: 102.48,
    data_bytes_per_block: 4_916.0,
};
const ARBITRUM_ONE_2026_10_01: Composition = Composition {
    blocks_per_day: 312_462,
    receipts_per_block: 5.08,
    logs_per_block: 17.11,
    topics_per_block: 46.17,
    data_bytes_per_block: 2_343.0,
};
const OP_MAINNET_2026_10_01: Composition = Composition {
    blocks_per_day: 43_200,
    receipts_per_block: 39.15,
    logs_per_block: 386.20,
    topics_per_block: 1_125.65,
    data_bytes_per_block: 20_241.0,
};
const UNICHAIN_2026_10_01: Composition = Composition {
    blocks_per_day: 86_400,
    receipts_per_block: 8.99,
    logs_per_block: 9.74,
    topics_per_block: 16.36,
    data_bytes_per_block: 805.0,
};
/// 150 blocks drawn uniformly over the 24 h before 10/4 06:40 UTC (head 52,153,161).
/// Two whole 128-block leaves proved that day measured 5.9–6.2M cycles/block.
const BASE_2026_10_04: Composition = Composition {
    blocks_per_day: 43_200,
    receipts_per_block: 147.59,
    logs_per_block: 830.48,
    topics_per_block: 2_321.59,
    data_bytes_per_block: 59_770.0,
};
const INK_2026_10_01: Composition = Composition {
    blocks_per_day: 86_400,
    receipts_per_block: 6.58,
    logs_per_block: 26.77,
    topics_per_block: 95.68,
    data_bytes_per_block: 3_827.0,
};

/// Every chain the pipeline proves. Fast-chain geometry is pinned to the rules by
/// `tests::fast_chain_geometry_follows_the_rules`.
pub static CHAINS: [ChainProfile; 8] = [
    ChainProfile {
        chain_id: ETHEREUM,
        name: "ethereum",
        family: Family::Ethereum,
        measured: None,
        // Per-block leaves, 64-block index.
        seg_log2: 0,
        index_log2: 6,
        anchor: Anchor::Ethereum,
        anchor_every_blocks: 1,
        anchor_lag_secs: 0,
        rpc_env: "WILLOW_ETH_RPC_ENDPOINTS",
        public_rpc: &["https://ethereum-rpc.publicnode.com"],
    },
    ChainProfile {
        chain_id: BASE,
        name: "base",
        family: Family::OpStack,
        measured: Some(BASE_2026_10_04),
        seg_log2: 6,
        index_log2: 7,
        anchor: Anchor::OpOutputRoot {
            factory: "0x43edB88C4B80fDD2AdFF2412A7BebF9dF42cB40e",
            registry: "0x909f6cf47ed12f010A796527f562bFc26C7F4E72",
        },
        anchor_every_blocks: 0,
        anchor_lag_secs: 0,
        rpc_env: "WILLOW_RPC_8453",
        public_rpc: &["https://base-rpc.publicnode.com"],
    },
    ChainProfile {
        chain_id: ARBITRUM_ONE,
        name: "arbitrum-one",
        family: Family::Orbit,
        measured: Some(ARBITRUM_ONE_2026_10_01),
        seg_log2: 11,
        index_log2: 12,
        anchor: Anchor::Bold {
            rollup: "0x4DCeB440657f21083db8aDd07665f8ddBe1DCfc0",
        },
        // 24 confirmations in 7,200 L1 blocks; 13,238 L2 blocks apart; 6.45 d lag.
        anchor_every_blocks: 13_238,
        anchor_lag_secs: 557_487,
        rpc_env: "WILLOW_RPC_42161",
        public_rpc: &["https://arbitrum-one-rpc.publicnode.com"],
    },
    ChainProfile {
        chain_id: ROBINHOOD,
        name: "robinhood",
        family: Family::Orbit,
        measured: Some(ROBINHOOD_2026_10_01),
        seg_log2: 10,
        index_log2: 11,
        anchor: Anchor::Bold {
            rollup: "0x23A19d23e89166adedbDcB432518AB01e4272D94",
        },
        // 45 confirmations in 7,200 L1 blocks; 19,031 L2 blocks apart; 6.43 d lag.
        anchor_every_blocks: 19_031,
        anchor_lag_secs: 555_723,
        rpc_env: "WILLOW_RPC_4663",
        public_rpc: &["https://robinhood-rpc.publicnode.com"],
    },
    ChainProfile {
        chain_id: OP_MAINNET,
        name: "op",
        family: Family::OpStack,
        measured: Some(OP_MAINNET_2026_10_01),
        seg_log2: 7,
        index_log2: 8,
        anchor: Anchor::OpSuperRoot {
            factory: "0xe5965Ab5962eDc7477C8520243A95517CD252fA9",
            registry: "0x23B2C62946350F4246f9f9D027e071f0264FD113",
        },
        // Type-9 games every 3,590 s (median); valid at created + 3.5 d
        // (clock) + 3.5 d (finality delay); created ~1,282 s after the L2 time.
        anchor_every_blocks: 1_795,
        anchor_lag_secs: 606_082,
        rpc_env: "WILLOW_RPC_10",
        public_rpc: &[
            "https://optimism-rpc.publicnode.com",
            "https://optimism.drpc.org",
        ],
    },
    ChainProfile {
        chain_id: UNICHAIN,
        name: "unichain",
        family: Family::OpStack,
        measured: Some(UNICHAIN_2026_10_01),
        seg_log2: 10,
        index_log2: 11,
        anchor: Anchor::OpSuperRoot {
            factory: "0x2F12d621a16e2d3285929C9996f478508951dFe4",
            registry: "0x27Cf508E4E3Aa8d30b3226aC3b5Ea0e8bcaCAFF9",
        },
        // Type-9 games every 3,612 s; created ~1,335 s after the L2 time.
        anchor_every_blocks: 3_612,
        anchor_lag_secs: 606_135,
        rpc_env: "WILLOW_RPC_130",
        public_rpc: &["https://unichain-rpc.publicnode.com"],
    },
    ChainProfile {
        chain_id: INK,
        name: "ink",
        family: Family::OpStack,
        measured: Some(INK_2026_10_01),
        seg_log2: 10,
        index_log2: 11,
        anchor: Anchor::OpSuperRoot {
            factory: "0x10d7B35078d3baabB96Dd45a9143B94be65b12CD",
            registry: "0xEe018bAf058227872540AC60eFbd38b023d9dAe2",
        },
        // Type-9 games every ~12 h (43,563 s typical); created ~1,337 s after.
        anchor_every_blocks: 43_563,
        anchor_lag_secs: 606_137,
        rpc_env: "WILLOW_RPC_57073",
        public_rpc: &["https://rpc-gel.inkonchain.com", "https://ink.drpc.org"],
    },
    ChainProfile {
        chain_id: STELLAR,
        name: "stellar",
        family: Family::Stellar,
        // The EVM cycle model does not apply. Measured on the Stellar leaf
        // (zkvm/stellar, 2026-10-01): 5.5-7.5M cycles per ledger, ~190 records per
        // ledger, 17,280 ledgers a day. S = 64 ledgers (~420M cycles, 320 s to fill,
        // always even-aligned); Q = 512 ledgers (~97K records, 43 min).
        measured: None,
        seg_log2: 6,
        index_log2: 9,
        anchor: Anchor::StellarScp {
            archive: "https://history.stellar.org/prd/core-live/core_live_001",
        },
        // Every ledger is final under SCP; its EXTERNALIZE messages are archived per
        // 64-ledger checkpoint (~320 s), published at the checkpoint (seen once).
        anchor_every_blocks: 1,
        anchor_lag_secs: 320,
        rpc_env: "WILLOW_STELLAR_LEDGERS",
        public_rpc: &[
            "https://aws-public-blockchain.s3.us-east-2.amazonaws.com/v1.1/stellar/ledgers/pubnet",
        ],
    },
];

pub fn profile(chain_id: u64) -> Option<&'static ChainProfile> {
    CHAINS.iter().find(|p| p.chain_id == chain_id)
}

pub fn by_name(name: &str) -> Option<&'static ChainProfile> {
    CHAINS.iter().find(|p| p.name == name)
}

/// The fast chains.
pub const FAST_CHAINS: [u64; 5] = [ARBITRUM_ONE, ROBINHOOD, OP_MAINNET, UNICHAIN, INK];

#[cfg(test)]
mod tests;
