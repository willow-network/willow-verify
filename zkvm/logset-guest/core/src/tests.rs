//! Native tests over the EXACT code the SP1 program runs.
//!
//! Three layers:
//! 1. **ERC KATs** — every constant and every preimage checked against the values
//!    `forge test -vvvv` prints for the ERC's reference `assets/test/Vectors.t.sol`.
//!    If Solidity and this crate disagree, an on-chain consumer rejects a proof
//!    that is internally consistent, so these are the contract between the two.
//! 2. **Oracles** — receipts roots against `alloy-trie`, keccak against
//!    `alloy-primitives`, the bloom against `alloy-primitives::Bloom`, and the
//!    padded Merkle tree against a naive reimplementation with no zero-cache.
//! 3. **Soundness surfaces** — every attack listed in `SPEC.md`, each next to the
//!    honest control it was derived from.

use super::*;
use crate::bloom::{bit_set, probe, FilterProbes};
use crate::erc::{
    abi_encode_statements, block_logs_hash, empty_block_logs_hash, filter_hash, log_set_root,
    node_hash, statement_hash, LogRecord, LogSetStatement, ARRAY_HEADER_LEN, BLOCK_TAG,
    FILTER_TYPEHASH, NODE_TAG, STATEMENT_TYPEHASH,
};
use crate::synth::{
    build_witness, dense_bloom, encode_header, lean_range, rlp_str, OwnedWitness, Rng, SynthBlock,
    SynthLog, SynthReceipt,
};

fn hx(s: &str) -> [u8; 32] {
    let mut out = [0u8; 32];
    out.copy_from_slice(&hex::decode(s).expect("hex"));
    out
}

fn hx20(s: &str) -> [u8; 20] {
    let mut out = [0u8; 20];
    out.copy_from_slice(&hex::decode(s).expect("hex"));
    out
}

/// `bytes32(uint256(v))` — right-aligned, the Solidity cast.
fn word(v: u64) -> [u8; 32] {
    let mut out = [0u8; 32];
    out[24..].copy_from_slice(&v.to_be_bytes());
    out
}

const TRANSFER: &str = "ddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef";
const USDC: &str = "a0b86991c6218b36c1d19d4a2e9eb0ce3606eb48";
const USDT: &str = "dac17f958d2ee523a2206206994597c13d831ec7";
/// A vault contract's address, left-padded into a 32-byte topic.
const VAULT_T: &str = "0000000000000000000000005a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a";

// ===========================================================================
// 1. ERC vector KATs — `forge test -vvvv` on assets/test/Vectors.t.sol
// ===========================================================================

#[test]
fn erc_constants_match_solidity() {
    assert_eq!(
        STATEMENT_TYPEHASH,
        hx("f2391921543743dfef3936c0ae97d382aa712cbdef6b85a02b365436e6ac6b8c")
    );
    assert_eq!(
        FILTER_TYPEHASH,
        hx("7da516d88ff342c0bec127d022bbb8bc6b014bb733e56f557d8bea57e8a50d70")
    );
    assert_eq!(
        BLOCK_TAG,
        hx("434f4d504c4554455f4c4f475f5345545f424c4f434b5f563100000000000000")
    );
    assert_eq!(
        NODE_TAG,
        hx("434f4d504c4554455f4c4f475f5345545f4e4f44455f56310000000000000000")
    );
    // The tags are `bytes32("...")`: ASCII, right-padded with zeros.
    let mut block_tag = [0u8; 32];
    block_tag[..25].copy_from_slice(b"COMPLETE_LOG_SET_BLOCK_V1");
    assert_eq!(BLOCK_TAG, block_tag);
    let mut node_tag = [0u8; 32];
    node_tag[..24].copy_from_slice(b"COMPLETE_LOG_SET_NODE_V1");
    assert_eq!(NODE_TAG, node_tag);
    // And the typehashes are the keccak of the EIP-712 type strings.
    assert_eq!(
        STATEMENT_TYPEHASH,
        keccak256(
            b"LogSetStatement(uint256 chainId,uint64 startBlock,uint64 endBlock,bytes32 startParentHash,bytes32 endBlockHash,bytes32 filterHash,bytes32 logSetRoot,uint64 logCount)"
        )
    );
    assert_eq!(
        FILTER_TYPEHASH,
        keccak256(
            b"LogFilter(address[] addresses,bytes32[] topics0,bytes32[] topics1,bytes32[] topics2,bytes32[] topics3)"
        )
    );
}

/// The vector filter: Transfer from two token contracts, any sender, any receiver.
fn vector_filter() -> LogFilter {
    LogFilter {
        addresses: vec![[0x11u8; 20], [0x22u8; 20]],
        topics: [vec![hx(TRANSFER)], Vec::new(), Vec::new(), Vec::new()],
    }
}

fn vector_transfer(
    tx_index: u32,
    log_index: u32,
    token: [u8; 20],
    from: u64,
    to: u64,
    amount: u64,
) -> LogRecord {
    LogRecord {
        tx_index,
        log_index,
        emitter: token,
        topics: vec![hx(TRANSFER), word(from), word(to)],
        data: word(amount).to_vec(),
    }
}

#[test]
fn erc_vector_filter_hash() {
    let f = vector_filter();
    assert_eq!(
        filter_hash(&f.addresses, &f.topics),
        hx("67d2a830e9146be8ecbbd9d0ecda8cca750380dcaf85512342788f61c181c018")
    );
    let any = LogFilter::default();
    assert_eq!(
        filter_hash(&any.addresses, &any.topics),
        hx("60aa730f0ed48d43f26704d47592fb7ad16f3cd86151c7daa00a26e29806d981")
    );
}

#[test]
fn erc_vector_record_bytes() {
    // 1 ether = 0x0de0b6b3a7640000.
    let r = vector_transfer(
        3,
        7,
        [0x11u8; 20],
        0xAAAA,
        0xBBBB,
        1_000_000_000_000_000_000,
    );
    let mut out = Vec::new();
    erc::push_log_record(
        &mut out,
        r.tx_index,
        r.log_index,
        &r.emitter,
        &r.topics,
        &r.data,
    );
    assert_eq!(
        hex::encode(&out),
        "0000000300000007111111111111111111111111111111111111111103\
         ddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef\
         000000000000000000000000000000000000000000000000000000000000aaaa\
         000000000000000000000000000000000000000000000000000000000000bbbb\
         000000200000000000000000000000000000000000000000000000000de0b6b3a7640000"
            .replace(['\\', ' ', '\n'], "")
    );
}

fn vector_leaves() -> [[u8; 32]; 3] {
    let b100 = [
        vector_transfer(
            3,
            7,
            [0x11u8; 20],
            0xAAAA,
            0xBBBB,
            1_000_000_000_000_000_000,
        ),
        vector_transfer(9, 21, [0x22u8; 20], 0xBBBB, 0xCCCC, 5),
    ];
    let b102 = [vector_transfer(0, 0, [0x11u8; 20], 0xCCCC, 0xAAAA, 42)];
    [
        block_logs_hash(100, &b100),
        empty_block_logs_hash(101),
        block_logs_hash(102, &b102),
    ]
}

#[test]
fn erc_vector_block_digests_and_root() {
    let leaves = vector_leaves();
    assert_eq!(
        leaves[0],
        hx("e2c7dcc7b9539a17cd3defdd5bdee458de42a6cb002fb0a49e9f5a5dcc6d07be")
    );
    assert_eq!(
        leaves[1],
        hx("2c04bc90b046755fe3a16e02cd9f4354349657a0e3c6642b628a9642330c3c52")
    );
    assert_eq!(
        leaves[2],
        hx("e8381948869f39c527679320642a86ac462537515b217d71a1d0e3d335478317")
    );
    // `blockLogsHash(n, [])` and `emptyBlockLogsHash(n)` are the same preimage.
    assert_eq!(empty_block_logs_hash(101), block_logs_hash(101, &[]));

    let root = log_set_root(&leaves).unwrap();
    assert_eq!(
        root,
        hx("4e316f543f08ede4792942ab33fa9837aa485541386224770069983213eaa4d3")
    );

    // The vector's inclusion proof for block 102: index 2 of 3, cap 4, siblings
    // [zero leaf, node(l0,l1)].
    let sib = [[0u8; 32], node_hash(&leaves[0], &leaves[1])];
    let mut node = leaves[2];
    let mut index = 2usize;
    for s in sib {
        node = if index & 1 == 0 {
            node_hash(&node, &s)
        } else {
            node_hash(&s, &node)
        };
        index >>= 1;
    }
    assert_eq!(node, root);
}

#[test]
fn erc_vector_statement_hash() {
    let f = vector_filter();
    let s = LogSetStatement {
        chain_id: 1,
        start_block: 100,
        end_block: 102,
        start_parent_hash: word(0x99),
        end_block_hash: word(0x102),
        filter_hash: filter_hash(&f.addresses, &f.topics),
        log_set_root: log_set_root(&vector_leaves()).unwrap(),
        log_count: 3,
    };
    assert_eq!(
        statement_hash(&s),
        hx("6ac9527fd5e79621843c9a41974d4c23e6d8611b9f65f63f535defd90effba91")
    );
}

// ===========================================================================
// 2. Oracles
// ===========================================================================

#[test]
fn keccak_matches_alloy() {
    for n in [0usize, 1, 31, 32, 135, 136, 137, 1000] {
        let data = vec![0xa5u8; n];
        assert_eq!(
            keccak256(&data),
            alloy_primitives::keccak256(&data).0,
            "keccak disagreed at len {n}"
        );
    }
}

#[test]
fn bloom_matches_alloy() {
    let mut rng = Rng(7);
    for _ in 0..64 {
        let v: [u8; 32] = rng.bytes();
        let mut ours = [0u8; 256];
        crate::synth::bloom_add(&mut ours, &v);
        let mut theirs = alloy_primitives::Bloom::ZERO;
        theirs.accrue(alloy_primitives::BloomInput::Raw(&v));
        assert_eq!(ours, theirs.0 .0, "bloom disagreed for {}", hex::encode(v));
    }
    let a: [u8; 20] = rng.bytes();
    let mut ours = [0u8; 256];
    crate::synth::bloom_add(&mut ours, &a);
    let mut theirs = alloy_primitives::Bloom::ZERO;
    theirs.accrue(alloy_primitives::BloomInput::Raw(&a));
    assert_eq!(ours, theirs.0 .0);
}

/// The bloom predicate, with the arithmetic written out.
#[test]
fn bloom_predicate_kat() {
    // keccak256(USDC address, 20 bytes)
    //   = 7b 58 55 bb 92 cd 7f 3f ...
    // idx0 = (0x7b << 8 | 0x58) & 0x7ff = 0x7b58 & 0x07ff = 0x0358 =  856
    // idx1 = (0x55 << 8 | 0xbb) & 0x7ff = 0x55bb & 0x07ff = 0x05bb = 1467
    // idx2 = (0x92 << 8 | 0xcd) & 0x7ff = 0x92cd & 0x07ff = 0x02cd =  717
    let addr = hx20(USDC);
    assert_eq!(
        hex::encode(keccak256(&addr)),
        "7b5855bb92cd7f3f78137497df02f6ccb9badda93d9782e0f230c807ba728be0"
    );
    let p = probe(&addr);
    assert_eq!(p.bits, [856, 1467, 717]);
    //  856 >> 3 = 107,  856 & 7 = 0  -> byte 255-107 = 148, mask 0x01
    // 1467 >> 3 = 183, 1467 & 7 = 3  -> byte 255-183 =  72, mask 0x08
    //  717 >> 3 =  89,  717 & 7 = 5  -> byte 255- 89 = 166, mask 0x20
    let mut bloom = [0u8; 256];
    bloom[148] |= 0x01;
    bloom[72] |= 0x08;
    bloom[166] |= 0x20;
    assert!(p.present_in(&bloom));
    let mut synth_bloom = [0u8; 256];
    crate::synth::bloom_add(&mut synth_bloom, &addr);
    assert_eq!(synth_bloom, bloom);

    // keccak256(Transfer topic0, 32 bytes) = ad a3 89 e1 fc 24 ...
    // idx0 = 0xada3 & 0x7ff = 0x05a3 = 1443 -> byte 255-180 =  75, mask 1<<3
    // idx1 = 0x89e1 & 0x7ff = 0x01e1 =  481 -> byte 255- 60 = 195, mask 1<<1
    // idx2 = 0xfc24 & 0x7ff = 0x0424 = 1060 -> byte 255-132 = 123, mask 1<<4
    let t = hx(TRANSFER);
    assert_eq!(
        hex::encode(keccak256(&t)),
        "ada389e1fc24a8587c776340efb91b36e675792ab631816100d55df0b5cf3cbc"
    );
    assert_eq!(probe(&t).bits, [1443, 481, 1060]);
    assert!(!probe(&t).present_in(&bloom));
    assert!(bit_set(&bloom, 856) && !bit_set(&bloom, 857));

    // Dropping ANY one of the three bits makes the value absent: that is why a
    // bloom-negative answer is a proof of non-membership.
    for drop in [(148usize, 0x01u8), (72, 0x08), (166, 0x20)] {
        let mut b = bloom;
        b[drop.0] &= !drop.1;
        assert!(!p.present_in(&b));
    }

    // A filter is possible only if EVERY non-empty list has a member present.
    let f = LogFilter {
        addresses: vec![addr],
        topics: [vec![t], Vec::new(), Vec::new(), Vec::new()],
    };
    let probes = FilterProbes::of(&f);
    assert!(!probes.possible(&bloom), "topic0 is absent");
    let mut both = bloom;
    crate::synth::bloom_add(&mut both, &t);
    assert!(probes.possible(&both));
    // The wildcard filter is possible everywhere, including an empty bloom.
    assert!(FilterProbes::of(&LogFilter::default()).possible(&[0u8; 256]));
}

#[test]
fn receipts_root_matches_alloy_trie() {
    let mut rng = Rng(0x5EED);
    for n in [1usize, 2, 3, 17, 128, 129, 200, 257] {
        let block = crate::synth::busy_block(&mut rng, n, 2, None, 0);
        let values: Vec<Vec<u8>> = block.receipts.iter().map(|r| r.value()).collect();
        let ours = willow_receipts_trie::build_trie(&values).root;
        let theirs = alloy_trie::root::ordered_trie_root_encoded(&values);
        assert_eq!(ours, theirs.0, "receipts root disagreed at N={n}");
    }
}

/// The padded tree, against a naive reimplementation that hashes every interior
/// position including the all-zero ones — no cache, no shortcut.
fn naive_root(leaves: &[[u8; 32]]) -> [u8; 32] {
    let mut cap = 1usize;
    while cap < leaves.len() {
        cap <<= 1;
    }
    let mut level: Vec<[u8; 32]> = leaves.to_vec();
    level.resize(cap, [0u8; 32]);
    while level.len() > 1 {
        level = level.chunks(2).map(|c| node_hash(&c[0], &c[1])).collect();
    }
    level[0]
}

#[test]
fn tree_padding_n_1_2_3_4_5_8() {
    for n in [1usize, 2, 3, 4, 5, 8] {
        let leaves: Vec<[u8; 32]> = (0..n).map(|i| keccak256(&[i as u8])).collect();
        assert_eq!(
            log_set_root(&leaves).unwrap(),
            naive_root(&leaves),
            "log_set_root disagreed with the naive tree at N={n}"
        );
    }
    // N = 1 is the leaf itself: no node hash at all.
    let leaf = keccak256(b"solo");
    assert_eq!(log_set_root(&[leaf]).unwrap(), leaf);
    // N = 3 pads with a ZERO leaf, not a hash of anything.
    let l: Vec<[u8; 32]> = (0..3).map(|i| keccak256(&[i as u8])).collect();
    assert_eq!(
        log_set_root(&l).unwrap(),
        node_hash(&node_hash(&l[0], &l[1]), &node_hash(&l[2], &[0u8; 32]))
    );
    assert_eq!(log_set_root(&[]), Err(Error::EmptyRange));
}

// ===========================================================================
// 3. The honest scenario every attack is derived from
// ===========================================================================

fn log(emitter: [u8; 20], topics: Vec<[u8; 32]>, data: Vec<u8>) -> SynthLog {
    SynthLog {
        emitter,
        topics,
        data,
    }
}

fn receipt(tx_type: u8, logs: Vec<SynthLog>) -> SynthReceipt {
    SynthReceipt {
        tx_type,
        status: true,
        cum_gas: 100_000,
        logs,
        deposit_tail: vec![],
    }
}

/// Two filters over one 6-block range:
/// * `fA` = USDC|USDT + Transfer + `topics1 = [vault]`  (the "out" leg)
/// * `fB` = USDC|USDT + Transfer + `topics2 = [vault]`  (the "in" leg)
///
/// block 0 lean; 1 opened, matches fA only; 2 lean; 3 opened, matches BOTH
/// (the shared block); 4 opened but matches NEITHER — a genuine bloom false
/// positive; 5 lean. N = 6, so the tree pads to 8.
fn filters() -> Vec<LogFilter> {
    let fa = LogFilter {
        addresses: vec![hx20(USDC), hx20(USDT)],
        topics: [
            vec![hx(TRANSFER)],
            vec![hx(VAULT_T)],
            Vec::new(),
            Vec::new(),
        ],
    };
    let fb = LogFilter {
        addresses: vec![hx20(USDC), hx20(USDT)],
        topics: [
            vec![hx(TRANSFER)],
            Vec::new(),
            vec![hx(VAULT_T)],
            Vec::new(),
        ],
    };
    vec![fa, fb]
}

const FIRST_PARENT: [u8; 32] = [0x9a; 32];
const START: u64 = 25_015_000;

fn scenario_blocks() -> Vec<SynthBlock> {
    let usdc = hx20(USDC);
    let transfer = hx(TRANSFER);
    let vault = hx(VAULT_T);
    let alice = word(0xA11CE);
    let bob = word(0xB0B);
    let mut rng = Rng(0xF00D);

    let lean = |rng: &mut Rng| {
        let probes: Vec<FilterProbes> = filters().iter().map(FilterProbes::of).collect();
        loop {
            let b = dense_bloom(rng, 200);
            if !probes.iter().any(|p| p.possible(&b)) {
                return SynthBlock {
                    receipts: Vec::new(),
                    bloom_override: Some(b),
                    receipts_root_override: Some(rng.bytes::<32>()),
                };
            }
        }
    };

    // block 1: one matching-fA transfer, plus a decoy receipt.
    let b1 = SynthBlock {
        receipts: vec![
            receipt(
                0,
                vec![log(
                    rng.bytes(),
                    vec![rng.bytes(), rng.bytes()],
                    vec![1, 2, 3],
                )],
            ),
            receipt(
                2,
                vec![log(usdc, vec![transfer, vault, alice], word(100).to_vec())],
            ),
        ],
        ..Default::default()
    };
    // block 3: the shared block — one fA match and one fB match, plus noise.
    let b3 = SynthBlock {
        receipts: vec![
            receipt(
                2,
                vec![log(usdc, vec![transfer, vault, bob], word(7).to_vec())],
            ),
            receipt(0, vec![log(rng.bytes(), vec![rng.bytes()], vec![])]),
            receipt(
                3,
                vec![
                    log(
                        rng.bytes(),
                        vec![rng.bytes(), rng.bytes(), rng.bytes()],
                        vec![9; 40],
                    ),
                    log(usdc, vec![transfer, alice, vault], word(8).to_vec()),
                ],
            ),
        ],
        ..Default::default()
    };
    // block 4: USDC+Transfer in one log and the vault topic in ANOTHER — the
    // bloom says "possible" for both filters and no log matches either.
    let b4 = SynthBlock {
        receipts: vec![
            receipt(
                2,
                vec![log(usdc, vec![transfer, alice, bob], word(1).to_vec())],
            ),
            receipt(
                0,
                vec![log(rng.bytes(), vec![rng.bytes(), vault], vec![0xff; 8])],
            ),
        ],
        ..Default::default()
    };
    vec![lean(&mut rng), b1, lean(&mut rng), b3, b4, lean(&mut rng)]
}

fn honest() -> OwnedWitness {
    build_witness(1, START, FIRST_PARENT, filters(), &scenario_blocks())
}

#[test]
fn honest_control_passes() {
    let w = honest();
    assert_eq!(
        w.opened.iter().map(|(i, _)| *i).collect::<Vec<_>>(),
        vec![1, 3, 4],
        "the builder and the guest must classify the same blocks"
    );
    let s = run_statements(&w.encode()).expect("honest witness must verify");
    assert_eq!(s.len(), 2);
    assert_eq!(s[0].chain_id, 1);
    assert_eq!(s[0].start_block, START);
    assert_eq!(s[0].end_block, START + 5);
    assert_eq!(s[0].start_parent_hash, FIRST_PARENT);
    assert_eq!(s[0].end_block_hash, w.end_block_hash());
    // fA matches in blocks 1 and 3; fB only in block 3.
    assert_eq!(s[0].log_count, 2);
    assert_eq!(s[1].log_count, 1);
    let f = filters();
    assert_eq!(s[0].filter_hash, filter_hash(&f[0].addresses, &f[0].topics));
    assert_eq!(s[1].filter_hash, filter_hash(&f[1].addresses, &f[1].topics));
    assert_ne!(s[0].log_set_root, s[1].log_set_root);
    // Both statements share the header walk: same range, same anchors.
    assert_eq!(s[0].start_parent_hash, s[1].start_parent_hash);
    assert_eq!(s[0].end_block_hash, s[1].end_block_hash);
}

#[test]
fn lean_only_range_verifies() {
    let f = filters().remove(0);
    let w = lean_range(4, f.clone(), 150);
    assert!(w.opened.is_empty());
    let s = run_statements(&w.encode()).unwrap();
    assert_eq!(s.len(), 1);
    assert_eq!(s[0].log_count, 0);
    // Every leaf is the empty digest of its own block number, so the root is a
    // pure function of the range — and it is NOT the all-zero tree.
    let leaves: Vec<[u8; 32]> = (0..4)
        .map(|i| empty_block_logs_hash(w.start_block + i))
        .collect();
    assert_eq!(s[0].log_set_root, log_set_root(&leaves).unwrap());
    assert_ne!(s[0].log_set_root, log_set_root(&[[0u8; 32]; 4]).unwrap());
}

/// `txIndex` and `logIndex` are POSITIONS. A one-block range makes the root equal
/// the leaf, so the digest the guest built can be compared record for record.
#[test]
fn positional_indices_are_not_witnessed() {
    let usdc = hx20(USDC);
    let transfer = hx(TRANSFER);
    let vault = hx(VAULT_T);
    let mut rng = Rng(0x1D_EE);
    let block = SynthBlock {
        receipts: vec![
            // tx 0: two logs, the second matches -> logIndex 1
            receipt(
                2,
                vec![
                    log(rng.bytes(), vec![rng.bytes()], vec![]),
                    log(usdc, vec![transfer, vault, word(1)], word(11).to_vec()),
                ],
            ),
            // tx 1: one non-matching log -> logIndex 2
            receipt(0, vec![log(rng.bytes(), vec![rng.bytes()], vec![7])]),
            // tx 2: one matching log -> logIndex 3
            receipt(
                2,
                vec![log(usdc, vec![transfer, vault, word(2)], word(22).to_vec())],
            ),
        ],
        ..Default::default()
    };
    let f = filters().remove(0);
    let w = build_witness(1, START, FIRST_PARENT, vec![f], &[block]);
    assert_eq!(w.opened.len(), 1);
    let s = run_statements(&w.encode()).unwrap();
    assert_eq!(s[0].log_count, 2);
    let expected = block_logs_hash(
        START,
        &[
            LogRecord {
                tx_index: 0,
                log_index: 1,
                emitter: usdc,
                topics: vec![transfer, vault, word(1)],
                data: word(11).to_vec(),
            },
            LogRecord {
                tx_index: 2,
                log_index: 3,
                emitter: usdc,
                topics: vec![transfer, vault, word(2)],
                data: word(22).to_vec(),
            },
        ],
    );
    // N = 1, so the root IS the leaf.
    assert_eq!(s[0].log_set_root, expected);
}

// ===========================================================================
// 4. Soundness surfaces
// ===========================================================================

fn err(w: &OwnedWitness) -> Error {
    run_statements(&w.encode()).expect_err("this witness must be rejected")
}

#[test]
fn attack_range_shrink() {
    let good = honest();
    let s_good = run_statements(&good.encode()).unwrap();
    // Drop the last block. The result is a perfectly valid statement — about a
    // DIFFERENT range, with a different endBlockHash, so it cannot be posted
    // against a chain whose head is the honest end block.
    let mut short = good.clone();
    short.headers.pop();
    short.n_blocks -= 1;
    short.opened.retain(|(i, _)| *i < short.n_blocks);
    let s_short = run_statements(&short.encode()).expect("a shorter range is its own statement");
    assert_eq!(s_short[0].end_block, START + 4);
    assert_ne!(s_short[0].end_block_hash, s_good[0].end_block_hash);
    assert_ne!(s_short[0].log_set_root, s_good[0].log_set_root);
    // Shrinking from the FRONT moves startParentHash instead.
    let mut front = good.clone();
    front.headers.remove(0);
    front.n_blocks -= 1;
    front.start_block += 1;
    front.opened = front
        .opened
        .iter()
        .filter(|(i, _)| *i > 0)
        .map(|(i, r)| (i - 1, r.clone()))
        .collect();
    let s_front = run_statements(&front.encode()).unwrap();
    assert_ne!(s_front[0].start_parent_hash, s_good[0].start_parent_hash);
    assert_ne!(s_front[0].log_set_root, s_good[0].log_set_root);
}

#[test]
fn attack_bloom_positive_declared_lean() {
    let mut w = honest();
    w.opened.retain(|(i, _)| *i != 1);
    assert_eq!(err(&w), Error::BloomPositiveNotOpened { index: 1 });
}

#[test]
fn attack_lean_block_opened() {
    let mut w = honest();
    w.opened.insert(0, (0, Vec::new()));
    assert_eq!(err(&w), Error::LeanBlockOpened { index: 0 });
}

#[test]
fn attack_receipt_dropped_added_reordered_tampered() {
    let good = honest();
    run_statements(&good.encode()).expect("control");

    let mut dropped = good.clone();
    dropped.opened[1].1.pop();
    assert_eq!(err(&dropped), Error::ReceiptsRootMismatch { index: 3 });

    let mut added = good.clone();
    let extra = receipt(0, vec![log([0x01; 20], vec![[0x02; 32]], vec![3])]).value();
    added.opened[1].1.push(extra);
    assert_eq!(err(&added), Error::ReceiptsRootMismatch { index: 3 });

    let mut reordered = good.clone();
    reordered.opened[1].1.swap(0, 1);
    assert_eq!(err(&reordered), Error::ReceiptsRootMismatch { index: 3 });

    let mut tampered = good.clone();
    let last = tampered.opened[1].1[0].len() - 1;
    tampered.opened[1].1[0][last] ^= 0x01;
    assert_eq!(err(&tampered), Error::ReceiptsRootMismatch { index: 3 });
}

#[test]
fn attack_typed_legacy_replay() {
    let good = honest();
    // block 3's first receipt is typed (0x02); block 3's second is legacy.
    let mut stripped = good.clone();
    assert_eq!(stripped.opened[1].1[0][0], 0x02);
    stripped.opened[1].1[0].remove(0);
    assert_eq!(err(&stripped), Error::ReceiptsRootMismatch { index: 3 });

    let mut wrapped = good.clone();
    assert!(
        wrapped.opened[1].1[1][0] >= 0xc0,
        "legacy receipts are bare rlp lists"
    );
    wrapped.opened[1].1[1].insert(0, 0x02);
    assert_eq!(err(&wrapped), Error::ReceiptsRootMismatch { index: 3 });

    // A type byte this guest does not know is rejected by name, not by root.
    let mut unknown = good.clone();
    unknown.opened[1].1[1].insert(0, 0x7e);
    assert_eq!(err(&unknown), Error::ReceiptsRootMismatch { index: 3 });
}

#[test]
fn attack_log_dropped_or_reordered() {
    let usdc = hx20(USDC);
    let transfer = hx(TRANSFER);
    let vault = hx(VAULT_T);
    let mk = |logs: Vec<SynthLog>| SynthBlock {
        receipts: vec![receipt(2, logs)],
        ..Default::default()
    };
    let l1 = log(usdc, vec![transfer, vault, word(1)], word(11).to_vec());
    let l2 = log(usdc, vec![transfer, vault, word(2)], word(22).to_vec());
    let f = filters().remove(0);
    let good = build_witness(
        1,
        START,
        FIRST_PARENT,
        vec![f.clone()],
        &[mk(vec![l1.clone(), l2.clone()])],
    );
    let s_good = run_statements(&good.encode()).unwrap();
    assert_eq!(s_good[0].log_count, 2);

    // Drop a matching log without touching the header: the receipts root no
    // longer matches.
    let mut dropped = good.clone();
    dropped.opened[0].1 = vec![receipt(2, vec![l1.clone()]).value()];
    assert_eq!(err(&dropped), Error::ReceiptsRootMismatch { index: 0 });

    // Drop it AND rebuild the header so the trie root matches again: now the
    // header hash moved, so the statement anchors to a block that is not the
    // canonical one.
    let rebuilt = build_witness(
        1,
        START,
        FIRST_PARENT,
        vec![f.clone()],
        &[mk(vec![l1.clone()])],
    );
    let s_rebuilt = run_statements(&rebuilt.encode()).unwrap();
    assert_eq!(s_rebuilt[0].log_count, 1);
    assert_ne!(s_rebuilt[0].end_block_hash, s_good[0].end_block_hash);
    assert_ne!(s_rebuilt[0].log_set_root, s_good[0].log_set_root);

    // Reorder the two logs: same set, different logIndex assignment, different
    // digest — and again a different header.
    let swapped = build_witness(1, START, FIRST_PARENT, vec![f], &[mk(vec![l2, l1])]);
    let s_swapped = run_statements(&swapped.encode()).unwrap();
    assert_eq!(s_swapped[0].log_count, 2);
    assert_ne!(s_swapped[0].log_set_root, s_good[0].log_set_root);
    assert_ne!(s_swapped[0].end_block_hash, s_good[0].end_block_hash);
}

#[test]
fn attack_filter_narrowed_reordered_or_swapped() {
    let good = honest();
    let s_good = run_statements(&good.encode()).unwrap();

    // The witness has NO filterHash field: the guest derives it from the filter
    // the scan used, so a narrowed filter yields a different commitment.
    let f = filters();
    let mut narrow = f[0].clone();
    narrow.topics[3] = vec![word(1)];
    assert_ne!(
        filter_hash(&narrow.addresses, &narrow.topics),
        filter_hash(&f[0].addresses, &f[0].topics)
    );

    // Swapping the two filters swaps the two statements, one for one: each root
    // travels with its own filterHash and cannot be detached from it.
    let mut swapped = good.clone();
    swapped.filters.swap(0, 1);
    let s_swapped = run_statements(&swapped.encode()).unwrap();
    assert_eq!(s_swapped[0], s_good[1]);
    assert_eq!(s_swapped[1], s_good[0]);

    // A non-canonical filter is rejected outright: descending addresses ...
    let mut desc = good.clone();
    desc.filters[0].addresses.swap(0, 1);
    assert_eq!(
        err(&desc),
        Error::FilterNotCanonical {
            filter: 0,
            list: 0,
            index: 1
        }
    );
    // ... and a duplicate topic.
    let mut dup = good.clone();
    dup.filters[1].topics[0] = vec![hx(TRANSFER), hx(TRANSFER)];
    assert_eq!(
        err(&dup),
        Error::FilterNotCanonical {
            filter: 1,
            list: 1,
            index: 1
        }
    );
    // ... and a descending topic list.
    let mut desc_t = good.clone();
    desc_t.filters[0].topics[1] = vec![word(9), word(2)];
    assert_eq!(
        err(&desc_t),
        Error::FilterNotCanonical {
            filter: 0,
            list: 2,
            index: 1
        }
    );
}

#[test]
fn attack_header_field_smuggling() {
    let good = honest();
    let s_good = run_statements(&good.encode()).unwrap();

    // Any byte of any header changes that header's hash. For a header with a
    // successor, the chain breaks at the NEXT block.
    let mut mid = good.clone();
    let off = mid.headers[2].len() - 1;
    mid.headers[2][off] ^= 0x01;
    assert_eq!(err(&mid), Error::ParentHashMismatch { index: 3 });

    // Swapping a header's receiptsRoot for another block's is the same story.
    let mut smuggle = good.clone();
    let (r1, r3) = (
        receipts_root_of(&smuggle.headers[1]),
        receipts_root_of(&smuggle.headers[3]),
    );
    assert_ne!(r1, r3);
    set_receipts_root(&mut smuggle.headers[3], &r1);
    assert_eq!(err(&smuggle), Error::ParentHashMismatch { index: 4 });

    // The LAST header has no successor — but it is the anchor, so touching a
    // field the guest never reads still moves `endBlockHash`.
    let mut tail = good.clone();
    let off = tail.headers[5].len() - 1;
    tail.headers[5][off] ^= 0x01;
    let s_tail = run_statements(&tail.encode()).unwrap();
    assert_ne!(s_tail[0].end_block_hash, s_good[0].end_block_hash);
}

/// Offset of the `receiptsRoot` payload inside a header's RLP: the list header,
/// then five 32/20-byte strings each with a one-byte header, then one more.
fn receipts_root_offset(h: &[u8]) -> usize {
    let mut off = if h[0] <= 0xf7 {
        1
    } else {
        1 + (h[0] - 0xf7) as usize
    };
    for _ in 0..5 {
        let len = (h[off] - 0x80) as usize;
        off += 1 + len;
    }
    off + 1
}

fn receipts_root_of(h: &[u8]) -> [u8; 32] {
    let off = receipts_root_offset(h);
    let mut out = [0u8; 32];
    out.copy_from_slice(&h[off..off + 32]);
    out
}

fn set_receipts_root(h: &mut [u8], root: &[u8; 32]) {
    let off = receipts_root_offset(h);
    h[off..off + 32].copy_from_slice(root);
}

#[test]
fn attack_number_discontinuity_and_parent_mismatch() {
    let good = honest();
    // Renumber block 3 and relink the chain so ONLY the number check can fire.
    let mut renum = good.clone();
    let root = receipts_root_of(&renum.headers[3]);
    let bloom = bloom_of(&renum.headers[3]);
    renum.headers[3] = encode_header(&[0u8; 32], START + 99, &root, &bloom);
    renum.relink(FIRST_PARENT);
    assert_eq!(
        err(&renum),
        Error::BlockNumberMismatch {
            index: 3,
            expected: START + 3,
            got: START + 99
        }
    );

    // Break a parentHash link directly.
    let mut broken = good.clone();
    let off = parent_hash_offset(&broken.headers[2]);
    broken.headers[2][off] ^= 0xff;
    assert_eq!(err(&broken), Error::ParentHashMismatch { index: 2 });
}

fn parent_hash_offset(h: &[u8]) -> usize {
    (if h[0] <= 0xf7 {
        1
    } else {
        1 + (h[0] - 0xf7) as usize
    }) + 1
}

fn bloom_of(h: &[u8]) -> [u8; 256] {
    // parentHash, ommersHash, beneficiary, stateRoot, transactionsRoot,
    // receiptsRoot: six one-byte-header strings, then the 256-byte bloom whose
    // header is `0xb9 0x01 0x00`.
    let mut off = if h[0] <= 0xf7 {
        1
    } else {
        1 + (h[0] - 0xf7) as usize
    };
    for _ in 0..6 {
        let len = (h[off] - 0x80) as usize;
        off += 1 + len;
    }
    assert_eq!(&h[off..off + 3], &[0xb9, 0x01, 0x00]);
    let mut out = [0u8; 256];
    out.copy_from_slice(&h[off + 3..off + 3 + 256]);
    out
}

#[test]
fn attack_opened_index_out_of_range_duplicate_nonascending() {
    let good = honest();
    let mut oob = good.clone();
    oob.opened[2].0 = 6;
    assert_eq!(
        err(&oob),
        Error::OpenedIndexOutOfRange {
            index: 6,
            n_blocks: 6
        }
    );

    let mut dup = good.clone();
    let entry = dup.opened[1].clone();
    dup.opened.insert(2, entry);
    assert_eq!(
        err(&dup),
        Error::OpenedIndexNotAscending { prev: 3, got: 3 }
    );

    let mut desc = good.clone();
    desc.opened.swap(0, 1);
    assert_eq!(
        err(&desc),
        Error::OpenedIndexNotAscending { prev: 3, got: 1 }
    );
}

#[test]
fn attack_blob_framing() {
    let good = honest();
    let bytes = good.encode();
    run_statements(&bytes).expect("control");

    let mut trailing = bytes.clone();
    trailing.push(0);
    assert_eq!(
        run_statements(&trailing),
        Err(Error::TrailingBytes { extra: 1 })
    );

    assert!(matches!(
        run_statements(&bytes[..bytes.len() - 1]),
        Err(Error::Truncated { .. })
    ));

    let mut magic = bytes.clone();
    magic[0] ^= 0xff;
    assert!(matches!(
        run_statements(&magic),
        Err(Error::BadMagic { .. })
    ));

    // The one mistake a host encoder is likely to make gets its own name.
    let mut reversed = bytes.clone();
    reversed[..4].copy_from_slice(&BLOB_MAGIC.to_be_bytes());
    assert_eq!(run_statements(&reversed), Err(Error::MagicByteOrder));
    assert_eq!(&bytes[..4], &[0x31, 0x53, 0x4c, 0x57]);

    // Zero blocks and zero filters.
    let mut zero_blocks = good.clone();
    zero_blocks.n_blocks = 0;
    zero_blocks.headers.clear();
    zero_blocks.opened.clear();
    assert_eq!(err(&zero_blocks), Error::ZeroBlocks);
    let mut zero_filters = good.clone();
    zero_filters.filters.clear();
    assert_eq!(err(&zero_filters), Error::ZeroFilters);
}

#[test]
fn blob_round_trips() {
    let good = honest();
    let bytes = good.encode();
    let decoded = decode_blob(&bytes).unwrap();
    assert_eq!(decoded, good.view());
    assert_eq!(encode_blob(&decoded), bytes);
}

/// The receipts-count guard exists because `willow_receipts_trie::build_trie`
/// PANICS above 65,536 values. The rejection happens at decode, before any
/// allocation the length field could drive.
#[test]
fn too_many_receipts_is_rejected_before_build_trie() {
    let good = honest();
    let mut bytes = good.encode();
    // Rewrite the first opened entry's n_receipts to 65,537. The opened section
    // starts after the headers; find it by decoding and re-encoding a witness
    // whose first opened block claims a huge count.
    // `start` lands on the `n_opened` word; +4 is the first entry's index, +8 its
    // receipt count.
    let start = bytes.len() - tail_len(&good);
    bytes[start + 8..start + 12]
        .copy_from_slice(&(crate::blob::MAX_RECEIPTS as u32 + 1).to_le_bytes());
    assert_eq!(
        run_statements(&bytes),
        Err(Error::TooManyReceipts {
            index: 1,
            n: crate::blob::MAX_RECEIPTS + 1
        })
    );
}

/// Byte length of the `opened` section including its count word.
fn tail_len(w: &OwnedWitness) -> usize {
    let mut n = 4;
    for (_, rs) in &w.opened {
        n += 8;
        for r in rs {
            n += 4 + r.len();
        }
    }
    n
}

#[test]
fn too_many_topics_is_rejected() {
    let usdc = hx20(USDC);
    let block = SynthBlock {
        receipts: vec![receipt(
            2,
            vec![log(
                usdc,
                vec![hx(TRANSFER), hx(VAULT_T), word(1), word(2), word(3)],
                vec![],
            )],
        )],
        ..Default::default()
    };
    let f = filters().remove(0);
    let w = build_witness(1, START, FIRST_PARENT, vec![f], &[block]);
    assert_eq!(
        err(&w),
        Error::TooManyTopics {
            block: 0,
            tx: 0,
            log: 0,
            got: 5
        }
    );
}

#[test]
fn malformed_receipt_shapes_are_rejected() {
    let good = honest();
    // A receipt whose body is not a 4-item list.
    let mut short = good.clone();
    short.opened[1].1[0] = {
        let body = crate::synth::rlp_list_of(&[
            crate::synth::rlp_u64(1),
            crate::synth::rlp_u64(2),
            rlp_str(&[0u8; 256]),
        ]);
        let mut v = vec![0x02];
        v.extend_from_slice(&body);
        v
    };
    // The root check fires first — the trie pins the bytes before they are parsed.
    assert_eq!(err(&short), Error::ReceiptsRootMismatch { index: 3 });

    // Reaching the parser needs a block whose root MATCHES but whose receipt is
    // malformed, which means building the header around the bad value.
    let bad = vec![{
        let body = crate::synth::rlp_list_of(&[
            crate::synth::rlp_u64(1),
            crate::synth::rlp_u64(2),
            rlp_str(&[0u8; 256]),
        ]);
        let mut v = vec![0x02];
        v.extend_from_slice(&body);
        v
    }];
    let mut bloom = [0u8; 256];
    crate::synth::bloom_add(&mut bloom, &hx20(USDC));
    crate::synth::bloom_add(&mut bloom, &hx(TRANSFER));
    crate::synth::bloom_add(&mut bloom, &hx(VAULT_T));
    let root = willow_receipts_trie::build_trie(&bad).root;
    let header = encode_header(&FIRST_PARENT, START, &root, &bloom);
    let f = filters().remove(0);
    let bwit = OwnedWitness {
        chain_id: 1,
        start_block: START,
        n_blocks: 1,
        filters: vec![f],
        headers: vec![header],
        opened: vec![(0, bad)],
    };
    assert_eq!(
        err(&bwit),
        Error::ReceiptFieldCount {
            block: 0,
            tx: 0,
            got: 3
        }
    );
}

#[test]
fn malformed_header_shapes_are_rejected() {
    let good = honest();
    let mut not_a_list = good.clone();
    not_a_list.headers[0] = rlp_str(&[0u8; 100]);
    assert!(matches!(err(&not_a_list), Error::Rlp { .. }));

    let mut too_few = good.clone();
    too_few.headers[0] = crate::synth::rlp_list_of(&[
        rlp_str(&[0u8; 32]),
        rlp_str(&[0u8; 32]),
        rlp_str(&[0u8; 32]),
    ]);
    assert_eq!(err(&too_few), Error::HeaderFieldCount { block: 0, got: 3 });

    let mut short_bloom = good.clone();
    short_bloom.headers[0] = crate::synth::rlp_list_of(&[
        rlp_str(&[0u8; 32]),
        rlp_str(&[0u8; 32]),
        rlp_str(&[0u8; 20]),
        rlp_str(&[0u8; 32]),
        rlp_str(&[0u8; 32]),
        rlp_str(&[0u8; 32]),
        rlp_str(&[0u8; 255]),
        crate::synth::rlp_u64(0),
        crate::synth::rlp_u64(START),
    ]);
    assert_eq!(
        err(&short_bloom),
        Error::HeaderFieldLen {
            block: 0,
            field: "logsBloom",
            got: 255
        }
    );
}

#[test]
fn non_canonical_rlp_is_rejected() {
    // 0x81 0x00 encodes the byte 0x00, which must be self-encoded.
    assert!(matches!(rlp::head(&[0x81, 0x00]), Err(Error::Rlp { .. })));
    // A long-form header for a 1-byte string.
    assert!(matches!(
        rlp::head(&[0xb8, 0x01, 0xff]),
        Err(Error::Rlp { .. })
    ));
    // A leading zero in a length-of-length.
    assert!(matches!(
        rlp::head(&[0xb9, 0x00, 0x38]),
        Err(Error::Rlp { .. })
    ));
    // Well-formed control.
    assert_eq!(rlp::head(&[0x00]).unwrap(), (false, &[0x00u8][..], 1));
    assert_eq!(rlp::as_u64(&[]), Some(0));
    assert_eq!(rlp::as_u64(&[0x00]), None);
    assert_eq!(rlp::as_u64(&[0x01, 0x02]), Some(0x0102));
}

// ===========================================================================
// 5. Public values
// ===========================================================================

#[test]
fn abi_encode_statements_layout() {
    let good = honest();
    let pv = run(&good.encode()).unwrap();
    let s = run_statements(&good.encode()).unwrap();
    assert_eq!(pv, abi_encode_statements(&s));
    assert_eq!(pv.len(), ARRAY_HEADER_LEN + 256 * 2);
    // word 0: the array's offset, 0x20.
    assert_eq!(pv[..32], word(0x20));
    // word 1: the element count.
    assert_eq!(pv[32..64], word(2));
    // Element 0, eight static words at fixed offsets.
    assert_eq!(pv[64..96], word(s[0].chain_id));
    assert_eq!(pv[96..128], word(s[0].start_block));
    assert_eq!(pv[128..160], word(s[0].end_block));
    assert_eq!(&pv[160..192], &s[0].start_parent_hash);
    assert_eq!(&pv[192..224], &s[0].end_block_hash);
    assert_eq!(&pv[224..256], &s[0].filter_hash);
    assert_eq!(&pv[256..288], &s[0].log_set_root);
    assert_eq!(pv[288..320], word(s[0].log_count));
    // Element 1 begins one 256-byte block later, same eight words.
    assert_eq!(pv[320..352], word(s[1].chain_id));
    assert_eq!(&pv[448..480], &s[1].end_block_hash);
    assert_eq!(&pv[480..512], &s[1].filter_hash);
    assert_eq!(&pv[512..544], &s[1].log_set_root);
    assert_eq!(pv[544..576], word(s[1].log_count));
    // A one-filter range is 64 + 256 bytes.
    let one = lean_range(2, filters().remove(0), 100);
    assert_eq!(run(&one.encode()).unwrap().len(), ARRAY_HEADER_LEN + 256);
}

// ===========================================================================
// 4. OP-stack deposit receipts (0x7E), chain-gated — Base KAT + synthetic
// ===========================================================================

fn match_all_filter() -> LogFilter {
    LogFilter {
        addresses: vec![],
        topics: [vec![], vec![], vec![], vec![]],
    }
}

/// Real Base block 48,932,815 (115 receipts: 1 deposit with nonce + version,
/// 7 legacy, 107 EIP-1559), from `fixtures/base/`. Expected values come from
/// `fixtures/base/oracle.py`, an independent RLP decoder + keccak.
fn base_kat_witness(chain_id: u64) -> OwnedWitness {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../fixtures/base");
    let header = std::fs::read(format!("{dir}/48932815.header.rlp")).expect("header");
    let raw = std::fs::read(format!("{dir}/48932815.receipts.bin")).expect("receipts");
    let mut values = Vec::new();
    let mut at = 0usize;
    while at < raw.len() {
        let n = u32::from_le_bytes(raw[at..at + 4].try_into().unwrap()) as usize;
        values.push(raw[at + 4..at + 4 + n].to_vec());
        at += 4 + n;
    }
    OwnedWitness {
        chain_id,
        start_block: 48_932_815,
        n_blocks: 1,
        filters: vec![match_all_filter()],
        headers: vec![header],
        opened: vec![(0, values)],
    }
}

const BASE_PARENT: &str = "eebcbb85839d1c87c60b6f14de7d0547dbcec53b6dd64371b1bd669a0ad6c103";
const BASE_HASH: &str = "ca210e851ceeb6381967294770337bfb7f52a20345bb4382c66d2d32de90b0e6";
const BASE_ROOT: &str = "cdfd19041d98792192bb59e3dff1b3dd2a157c4ceff617a598301ccdc6174033";

#[test]
fn base_real_block_kat() {
    let w = base_kat_witness(8453);
    assert_eq!(w.opened[0].1.len(), 115);
    assert_eq!(
        w.opened[0].1[0][0], 0x7e,
        "receipt 0 is the L1-attributes deposit"
    );
    let s = run_statements(&w.encode()).expect("real Base block must verify");
    assert_eq!(s.len(), 1);
    let s = &s[0];
    assert_eq!(s.chain_id, 8453);
    assert_eq!((s.start_block, s.end_block), (48_932_815, 48_932_815));
    assert_eq!(s.start_parent_hash, hx(BASE_PARENT));
    assert_eq!(s.end_block_hash, hx(BASE_HASH));
    assert_eq!(s.log_count, 378);
    assert_eq!(s.log_set_root, hx(BASE_ROOT));
    // The constants above are the oracle's output; keep them tied to the file.
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../fixtures/base");
    let j = std::fs::read_to_string(format!("{dir}/48932815.expected.json")).unwrap();
    for h in [BASE_PARENT, BASE_HASH, BASE_ROOT] {
        assert!(j.contains(h), "expected.json must carry {h}");
    }
    assert!(j.contains("\"log_count\": 378"));
}

#[test]
fn base_block_under_ethereum_chain_id_rejects_the_deposit() {
    // Same bytes, chain 1: the deposit type is unknown on Ethereum, as before.
    let w = base_kat_witness(1);
    assert_eq!(
        err(&w),
        Error::ReceiptTypeUnknown {
            block: 0,
            tx: 0,
            ty: 0x7e
        }
    );
}

fn deposit(tail: Vec<u64>, logs: Vec<SynthLog>) -> SynthReceipt {
    SynthReceipt {
        tx_type: 0x7e,
        status: true,
        cum_gas: 50_000,
        logs,
        deposit_tail: tail,
    }
}

/// A block whose first receipt is a deposit that emitted logs (a user deposit)
/// and whose second is an ordinary EIP-1559 receipt.
fn deposit_block(tail: Vec<u64>) -> SynthBlock {
    let usdc = hx20(USDC);
    let t = hx(TRANSFER);
    SynthBlock {
        receipts: vec![
            deposit(
                tail,
                vec![
                    log(usdc, vec![t, word(1), word(2)], word(7).to_vec()),
                    log([0x42; 20], vec![], vec![]),
                ],
            ),
            receipt(2, vec![log(usdc, vec![t, word(3), word(4)], vec![])]),
        ],
        ..Default::default()
    }
}

#[test]
fn deposit_logs_are_scanned_on_op_stack_chains() {
    let usdc_transfers = LogFilter {
        addresses: vec![hx20(USDC)],
        topics: [vec![hx(TRANSFER)], vec![], vec![], vec![]],
    };
    // Bedrock (4 items), Regolith (+nonce), Canyon (+nonce, version).
    for tail in [vec![], vec![7], vec![7, 1]] {
        for chain in super::receipt::OP_STACK_CHAIN_IDS {
            let w = build_witness(
                chain,
                1_000,
                [0x11; 32],
                vec![match_all_filter(), usdc_transfers.clone()],
                &[deposit_block(tail.clone())],
            );
            let s = run_statements(&w.encode()).expect("deposit block verifies");
            assert_eq!(s[0].log_count, 3, "match-all sees both deposit logs");
            assert_eq!(s[1].log_count, 2, "the filter matches the deposit's log");
        }
    }
}

#[test]
fn deposit_type_is_unknown_off_op_stack() {
    for chain in [1u64, 42161, 11155111] {
        let w = build_witness(
            chain,
            1_000,
            [0x11; 32],
            vec![match_all_filter()],
            &[deposit_block(vec![7, 1])],
        );
        assert_eq!(
            err(&w),
            Error::ReceiptTypeUnknown {
                block: 0,
                tx: 0,
                ty: 0x7e
            },
            "chain {chain}"
        );
    }
}

#[test]
fn deposit_tail_shape_is_enforced() {
    let run = |b: SynthBlock| {
        let w = build_witness(8453, 1_000, [0x11; 32], vec![match_all_filter()], &[b]);
        run_statements(&w.encode()).map(|_| ())
    };
    // Seven items: one too many.
    assert_eq!(
        run(deposit_block(vec![7, 1, 9])),
        Err(Error::ReceiptFieldCount {
            block: 0,
            tx: 0,
            got: 7
        })
    );
    // A tail on a NON-deposit receipt is still refused.
    let mut typed_tail = deposit_block(vec![]);
    typed_tail.receipts[1].deposit_tail = vec![7];
    assert_eq!(
        run(typed_tail),
        Err(Error::ReceiptFieldCount {
            block: 0,
            tx: 1,
            got: 5
        })
    );
}

/// Byte-level tail damage the synthesizer cannot express: a list nonce, a
/// non-canonical nonce and an over-wide version. The trie is rebuilt over the
/// damaged values so the root check passes and the PARSER must reject.
#[test]
fn deposit_tail_malformed_is_rejected_by_the_parser() {
    use crate::synth::{rlp_list_of, rlp_u64};
    let good = deposit_block(vec![7, 1]).receipts[0].clone();
    let logs: Vec<Vec<u8>> = good.logs.iter().map(|l| l.rlp()).collect();
    let head = vec![
        rlp_u64(1),
        rlp_u64(good.cum_gas),
        rlp_str(&good.bloom()),
        rlp_list_of(&logs),
    ];
    let with = |tail: Vec<Vec<u8>>| {
        let mut items = head.clone();
        items.extend(tail);
        let mut v = vec![0x7e];
        v.extend(rlp_list_of(&items));
        v
    };
    for (what, tail) in [
        ("list nonce", vec![rlp_list_of(&[rlp_u64(7)])]),
        ("leading-zero nonce", vec![rlp_str(&[0x00, 0x07])]),
        ("9-byte version", vec![rlp_u64(7), rlp_str(&[1u8; 9])]),
    ] {
        let values = vec![with(tail)];
        let root = willow_receipts_trie::build_trie(&values).root;
        let header = encode_header(&[0x11; 32], 1_000, &root, &[0xff; 256]);
        let w = OwnedWitness {
            chain_id: 8453,
            start_block: 1_000,
            n_blocks: 1,
            filters: vec![match_all_filter()],
            headers: vec![header],
            opened: vec![(0, values)],
        };
        assert_eq!(
            err(&w),
            Error::DepositTailMalformed { block: 0, tx: 0 },
            "{what}"
        );
    }
}

// ===========================================================================
// 5. The fast chains' receipt formats — real blocks of all five chains
// ===========================================================================

/// One range of `fixtures/fast-chains/expected.json` as a witness under `chain_id`.
fn fast_witness(chain: &str, chain_id: u64, start: u64, end: u64) -> OwnedWitness {
    use std::io::Read as _;
    let dir = format!(
        "{}/../fixtures/fast-chains/{chain}",
        env!("CARGO_MANIFEST_DIR")
    );
    let mut headers = Vec::new();
    let mut opened = Vec::new();
    for (i, n) in (start..=end).enumerate() {
        headers.push(std::fs::read(format!("{dir}/{n}.header.rlp")).expect("header"));
        let gz = std::fs::read(format!("{dir}/{n}.receipts.bin.gz")).expect("receipts");
        let mut raw = Vec::new();
        flate2::read::GzDecoder::new(&gz[..])
            .read_to_end(&mut raw)
            .unwrap();
        let mut values = Vec::new();
        let mut at = 0usize;
        while at < raw.len() {
            let len = u32::from_le_bytes(raw[at..at + 4].try_into().unwrap()) as usize;
            values.push(raw[at + 4..at + 4 + len].to_vec());
            at += 4 + len;
        }
        opened.push((i as u32, values));
    }
    OwnedWitness {
        chain_id,
        start_block: start,
        n_blocks: (end - start + 1) as u32,
        filters: vec![match_all_filter()],
        headers,
        opened,
    }
}

fn fast_ranges() -> Vec<serde_json::Value> {
    let p = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../fixtures/fast-chains/expected.json"
    );
    serde_json::from_str(&std::fs::read_to_string(p).unwrap()).unwrap()
}

/// Item count of an RLP list (a receipt body after its type byte).
fn rlp_list_items(b: &[u8]) -> usize {
    let be = |x: &[u8]| x.iter().fold(0usize, |a, &y| a << 8 | y as usize);
    let (mut at, end) = match b[0] {
        x @ 0xc0..=0xf7 => (1, 1 + (x - 0xc0) as usize),
        x @ 0xf8..=0xff => {
            let l = (x - 0xf7) as usize;
            (1 + l, 1 + l + be(&b[1..1 + l]))
        }
        _ => panic!("not an RLP list"),
    };
    let mut k = 0;
    while at < end {
        at += match b[at] {
            0x00..=0x7f => 1,
            x @ 0x80..=0xb7 => 1 + (x - 0x80) as usize,
            x @ 0xb8..=0xbf => {
                let l = (x - 0xb7) as usize;
                1 + l + be(&b[at + 1..at + 1 + l])
            }
            x @ 0xc0..=0xf7 => 1 + (x - 0xc0) as usize,
            x => {
                let l = (x - 0xf7) as usize;
                1 + l + be(&b[at + 1..at + 1 + l])
            }
        };
        k += 1;
    }
    k
}

/// Every range proves under its own chain id and matches the independent oracle
/// (`make_fixtures.py`); the receipts roots also match alloy-trie. Includes each
/// verifier's EARLIEST-FORMAT range, the real-block evidence for its first key entry
/// (`script/DeployLogSetMainnet.s.sol`): Ethereum at Byzantium, Base/OP before Canyon,
/// Unichain/Ink/Robinhood at genesis, Arbitrum One from the Nitro genesis; and pre-Byzantium
/// Ethereum (postState receipts) back to block 1.
#[test]
fn fast_chains_real_blocks_kat() {
    let ranges = fast_ranges();
    assert_eq!(ranges.len(), 24);
    let mut seen_types = std::collections::BTreeSet::new();
    let mut deposit_shapes = std::collections::BTreeSet::new();
    for r in &ranges {
        let chain = r["chain"].as_str().unwrap();
        let cid = r["chain_id"].as_u64().unwrap();
        let (s, e) = (r["start"].as_u64().unwrap(), r["end"].as_u64().unwrap());
        let w = fast_witness(chain, cid, s, e);
        for (_, values) in &w.opened {
            let root = willow_receipts_trie::build_trie(values).root;
            assert_eq!(root, alloy_trie::root::ordered_trie_root_encoded(values).0);
            for v in values.iter().filter(|v| v[0] == 0x7e) {
                deposit_shapes.insert((cid, rlp_list_items(&v[1..])));
            }
            // Pre-Byzantium receipt: an untyped list whose first item is a 32-byte postState.
            let header_len = |b: u8| {
                if b >= 0xf8 {
                    1 + (b - 0xf7) as usize
                } else {
                    1
                }
            };
            if cid == 1
                && values
                    .iter()
                    .any(|v| v[0] >= 0xc0 && v[header_len(v[0])] == 0xa0)
            {
                seen_types.insert("1:postState".to_string());
            }
        }
        let st = run_statements(&w.encode()).unwrap_or_else(|x| panic!("{chain} {s}: {x:?}"));
        let st = &st[0];
        let h = |k: &str| hx(&r[k].as_str().unwrap()[2..]);
        assert_eq!(st.chain_id, cid);
        assert_eq!((st.start_block, st.end_block), (s, e));
        assert_eq!(st.start_parent_hash, h("start_parent_hash"), "{chain} {s}");
        assert_eq!(st.end_block_hash, h("end_block_hash"), "{chain} {s}");
        assert_eq!(
            st.log_count,
            r["log_count"].as_u64().unwrap(),
            "{chain} {s}"
        );
        assert_eq!(st.log_set_root, h("log_set_root"), "{chain} {s}");
        for t in r["receipt_types"].as_object().unwrap().keys() {
            seen_types.insert(format!("{cid}:{t}"));
        }
    }
    // Real receipts of every Nitro type that occurred in the 250K-block scan,
    // and the deposit type on every OP-stack fast chain.
    for t in [
        "1:0x00",
        "1:postState",
        "42161:0x64",
        "42161:0x6a",
        "4663:0x64",
        "4663:0x68",
        "4663:0x69",
        "4663:0x6a",
        "10:0x7e",
        "130:0x7e",
        "57073:0x7e",
    ] {
        assert!(seen_types.contains(t), "no real receipt of {t}");
    }
    // Real deposit receipts in both consensus shapes: 4 items before Canyon (Base, OP;
    // op-geth's RPC reports a depositNonce there, but it is not in the receipt hash) and
    // 6 items from Canyon (Unichain and Ink from genesis, every recent OP-stack range).
    for shape in [(8453, 4), (10, 4), (10, 6), (130, 6), (57073, 6)] {
        assert!(
            deposit_shapes.contains(&shape),
            "no real {shape:?} deposit receipt"
        );
    }
}

/// The same bytes under any other profile fail at receipt 0 (0x6A or 0x7E).
#[test]
fn fast_chain_types_are_refused_off_their_profile() {
    for r in fast_ranges() {
        let chain = r["chain"].as_str().unwrap();
        let cid = r["chain_id"].as_u64().unwrap();
        let (s, e) = (r["start"].as_u64().unwrap(), r["end"].as_u64().unwrap());
        if cid == 1 {
            continue; // Ethereum's own types are accepted under every profile.
        }
        let nitro = super::receipt::NITRO_CHAIN_IDS.contains(&cid);
        let first_ty = if nitro { 0x6a } else { 0x7e };
        for other in [1u64, if nitro { 10 } else { 42161 }] {
            let w = fast_witness(chain, other, s, e);
            // The first block that has a receipt (the Nitro genesis block has none).
            let block = w.opened.iter().position(|(_, v)| !v.is_empty()).unwrap() as u32;
            assert_eq!(
                err(&w),
                Error::ReceiptTypeUnknown {
                    block,
                    tx: 0,
                    ty: first_ty
                },
                "{chain} {s} under chain {other}"
            );
        }
    }
}

fn typed_receipt(ty: u8, logs: Vec<SynthLog>, tail: Vec<u64>) -> SynthReceipt {
    SynthReceipt {
        tx_type: ty,
        status: true,
        cum_gas: 30_000,
        logs,
        deposit_tail: tail,
    }
}

/// Nitro types with no real occurrence in the scans (0x65, 0x66), with logs:
/// SYNTHETIC vectors. Plus the non-types 0x67 and 0x78, and arity.
#[test]
fn nitro_types_synthetic() {
    let usdc = hx20(USDC);
    let t = hx(TRANSFER);
    let block = |ty: u8, tail: Vec<u64>| SynthBlock {
        receipts: vec![
            typed_receipt(0x6a, vec![], vec![]),
            typed_receipt(
                ty,
                vec![log(usdc, vec![t, word(1), word(2)], word(9).to_vec())],
                tail,
            ),
        ],
        ..Default::default()
    };
    let run_on = |chain: u64, b: SynthBlock| {
        let w = build_witness(chain, 7_000, [0x22; 32], vec![match_all_filter()], &[b]);
        run_statements(&w.encode()).map(|s| s[0].log_count)
    };
    for ty in super::receipt::NITRO_TX_TYPES {
        for chain in super::receipt::NITRO_CHAIN_IDS {
            assert_eq!(
                run_on(chain, block(ty, vec![])),
                Ok(1),
                "type 0x{ty:02x} chain {chain}"
            );
        }
        assert_eq!(
            run_on(1, block(ty, vec![])),
            Err(Error::ReceiptTypeUnknown {
                block: 0,
                tx: 0,
                ty: 0x6a
            })
        );
    }
    for ty in [0x67u8, 0x78, 0x7e] {
        assert_eq!(
            run_on(42161, block(ty, vec![])),
            Err(Error::ReceiptTypeUnknown {
                block: 0,
                tx: 1,
                ty
            }),
            "0x{ty:02x} is not a Nitro receipt type"
        );
    }
    // A Nitro receipt never carries a deposit-style tail.
    assert_eq!(
        run_on(4663, block(0x64, vec![7])),
        Err(Error::ReceiptFieldCount {
            block: 0,
            tx: 1,
            got: 5
        })
    );
}

#[test]
fn profiles_are_the_seven_chains() {
    use super::receipt::{profile_of, ReceiptProfile::*};
    assert_eq!(profile_of(1), L1);
    for c in [8453, 10, 130, 57073] {
        assert_eq!(profile_of(c), OpStack);
    }
    for c in [42161, 4663] {
        assert_eq!(profile_of(c), Nitro);
    }
    // Chains outside the seven get the L1 set only.
    for c in [84532, 480, 1868, 42170, 421614, 11155111] {
        assert_eq!(profile_of(c), L1, "{c}");
    }
}
