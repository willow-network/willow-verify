//! Calls over a hand-made ArbOS state: Nitro's opcodes, ArbSys, the version gate and what is
//! refused. (Live calls against a Nitro node are the log RPC's tape tests.)

use super::*;
use crate::state::SLOTS;
use alloy_primitives::{keccak256, Bytes};
use revm::database::{CacheDB, EmptyDB};
use revm::state::{AccountInfo, Bytecode};

const NOBODY: Address = address!("0x6c3f9e1a2b7d4c58e0f1a9b3d2c4e6f8a1b3c5d7");
const CODE: Address = address!("0x00000000000000000000000000000000000c0de0");
const L1_BLOCK: u64 = 26_401_152;

fn block() -> Block {
    let mut mix = [0u8; 32];
    mix[8..16].copy_from_slice(&L1_BLOCK.to_be_bytes());
    mix[16..24].copy_from_slice(&61u64.to_be_bytes());
    Block {
        number: 112_795_805,
        timestamp: 1_794_513_600,
        gas_limit: 1 << 50,
        base_fee: 20_068_000,
        difficulty: U256::from(1),
        mix_hash: B256::from(mix),
        beneficiary: BATCH_POSTER,
    }
}

/// ArbOS's state as Robinhood Chain held it, the L1 hash of each kept block its number's hash.
fn db(code: &[u8]) -> CacheDB<EmptyDB> {
    let mut db = CacheDB::new(EmptyDB::default());
    let s = &*SLOTS;
    db.insert_account_info(
        ARBOS_STATE,
        AccountInfo {
            nonce: 1,
            ..Default::default()
        },
    );
    for (slot, v) in [
        (s.version, 61u64),
        (s.brotli_level, 1),
        (s.l1_price_per_unit, 2_536_105),
        (s.l2_min_base_fee, 20_000_000),
        (s.l1_block_number, L1_BLOCK),
        (s.l1_units_since_update, 120_850_094),
    ] {
        db.insert_account_storage(ARBOS_STATE, slot.into(), U256::from(v))
            .unwrap();
    }
    for n in L1_BLOCK - 256..L1_BLOCK {
        let h = keccak256(n.to_be_bytes());
        db.insert_account_storage(ARBOS_STATE, s.l1_block_hash(n).into(), h.into())
            .unwrap();
    }
    let code = Bytes::copy_from_slice(code);
    db.insert_account_info(
        CODE,
        AccountInfo {
            code_hash: keccak256(&code),
            code: Some(Bytecode::new_raw(code)),
            ..Default::default()
        },
    );
    db
}

fn call_code(code: &[u8], data: &[u8]) -> Outcome {
    let call = Call {
        from: NOBODY,
        to: Some(CODE),
        data: Bytes::copy_from_slice(data),
        ..Default::default()
    };
    run(
        db(code),
        &block(),
        &call,
        &BlockOverrides::default(),
        &HashSet::new(),
    )
    .unwrap()
}

/// Code that runs `op` (one opcode, after pushing `arg` if any) and returns the word it leaves.
fn returning(pre: &[u8], op: u8) -> Vec<u8> {
    let mut c = pre.to_vec();
    c.push(op);
    c.extend([0x60, 0x00, 0x52, 0x60, 0x20, 0x60, 0x00, 0xf3]);
    c
}

fn word(v: u64) -> Bytes {
    Bytes::from(U256::from(v).to_be_bytes::<32>().to_vec())
}

#[test]
fn number_is_the_l1_block_arbos_keeps() {
    assert_eq!(
        call_code(&returning(&[], 0x43), &[]),
        Outcome::Returned(word(L1_BLOCK))
    );
}

#[test]
fn blockhash_reads_arbos_l1_hashes_and_nothing_past_them() {
    let at = |n: u64| {
        let mut pre = vec![0x63];
        pre.extend((n as u32).to_be_bytes());
        call_code(&returning(&pre, 0x40), &[])
    };
    let kept = L1_BLOCK - 3;
    assert_eq!(
        at(kept),
        Outcome::Returned(Bytes::from(keccak256(kept.to_be_bytes()).to_vec()))
    );
    assert_eq!(at(L1_BLOCK), Outcome::Returned(word(0)));
    assert_eq!(at(L1_BLOCK - 257), Outcome::Returned(word(0)));
}

#[test]
fn gasprice_is_the_base_fee_paid_and_prevrandao_the_difficulty() {
    assert_eq!(
        call_code(&returning(&[], 0x3a), &[]),
        Outcome::Returned(word(0))
    );
    assert_eq!(
        call_code(&returning(&[], 0x44), &[]),
        Outcome::Returned(word(1))
    );
}

#[test]
fn blobbasefee_fails_as_nitro_fails_it() {
    assert_eq!(
        call_code(&returning(&[], 0x4a), &[]),
        Outcome::HaltedSaying("BLOBBASEFEE is not supported on Arbitrum".into())
    );
}

#[test]
fn a_stylus_program_is_refused_but_its_code_may_be_read() {
    let stylus = [0xef, 0xf0, 0x00, 0x00, 0x01, 0x02];
    assert!(matches!(call_code(&stylus, &[]), Outcome::Refused(_)));
    // A fragment is no program: Nitro runs it as EVM code, and 0xEF is no opcode.
    let fragment = [0xef, 0xf0, 0x01, 0x00, 0x01];
    assert_eq!(
        call_code(&fragment, &[]),
        Outcome::Halted(HaltReason::OpcodeNotFound)
    );
}

/// Code calling `to` with the call's own data, returning what it returned.
fn proxy(to: u8) -> Vec<u8> {
    // calldatacopy(0, 0, calldatasize); call(gas, to, 0, 0, calldatasize, 0, 0);
    // returndatacopy(0, 0, returndatasize); return(0, returndatasize)
    vec![
        0x36, 0x60, 0x00, 0x60, 0x00, 0x37, 0x60, 0x00, 0x60, 0x00, 0x36, 0x60, 0x00, 0x60, 0x00,
        0x60, to, 0x5a, 0xf1, 0x50, 0x3d, 0x60, 0x00, 0x60, 0x00, 0x3e, 0x3d, 0x60, 0x00, 0xf3,
    ]
}

fn sel(sig: &str) -> Vec<u8> {
    keccak256(sig.as_bytes())[..4].to_vec()
}

#[test]
fn arbsys_answers_from_the_block_and_arbos() {
    let via = |sig: &str| call_code(&proxy(0x64), &sel(sig));
    assert_eq!(
        via("arbBlockNumber()"),
        Outcome::Returned(word(112_795_805))
    );
    assert_eq!(via("arbChainID()"), Outcome::Returned(word(ROBINHOOD)));
    assert_eq!(via("arbOSVersion()"), Outcome::Returned(word(116)));
    // Called from the top-level contract: depth 2, top level.
    assert_eq!(via("isTopLevelCall()"), Outcome::Returned(word(1)));
    // No such method: reverted, nothing returned.
    assert_eq!(via("nope()"), Outcome::Returned(Bytes::new()));
}

#[test]
fn other_arbos_precompiles_and_methods_are_refused() {
    let out = call_code(&proxy(0x6e), &sel("getLifetime()"));
    assert!(matches!(out, Outcome::Refused(_)), "{out:?}");
    let out = call_code(&proxy(0x6c), &sel("getMultiGasBaseFee()"));
    assert!(matches!(out, Outcome::Refused(_)), "{out:?}");
}

#[test]
fn an_overridden_precompile_is_its_code() {
    let mut db = db(&proxy(0x64));
    let code = Bytes::from(returning(&[], 0x30));
    db.insert_account_info(
        address!("0x0000000000000000000000000000000000000064"),
        AccountInfo {
            code_hash: keccak256(&code),
            code: Some(Bytecode::new_raw(code)),
            ..Default::default()
        },
    );
    let overridden = HashSet::from([address!("0x0000000000000000000000000000000000000064")]);
    let call = Call {
        from: NOBODY,
        to: Some(CODE),
        data: Bytes::from(sel("arbChainID()")),
        ..Default::default()
    };
    let out = run(db, &block(), &call, &BlockOverrides::default(), &overridden).unwrap();
    let mut addr = [0u8; 32];
    addr[31] = 0x64;
    assert_eq!(out, Outcome::Returned(Bytes::from(addr.to_vec())));
}

#[test]
fn only_arbos_61_is_run() {
    let mut b = block();
    b.mix_hash.0[23] = 60;
    let out = run(
        db(&[]),
        &b,
        &Call::default(),
        &BlockOverrides::default(),
        &HashSet::new(),
    );
    assert!(matches!(out, Ok(Outcome::Refused(_))));
}

#[test]
fn arbos_revert_reasons_render_as_nitro_renders_them() {
    let mut data = keccak256(b"InvalidBlockNumber(uint256,uint256)")[..4].to_vec();
    data.extend(word(5).iter());
    data.extend(word(7).iter());
    assert_eq!(
        crate::precompiles::render_error(&data).as_deref(),
        Some("error InvalidBlockNumber(5, 7)")
    );
    assert_eq!(crate::precompiles::render_error(&[1, 2, 3, 4]), None);
}

#[test]
fn a_price_without_the_balance_is_refused_as_nitro_words_it() {
    let call = Call {
        from: NOBODY,
        to: Some(CODE),
        gas: Some(100_000),
        gas_price: Some(30_000_000),
        ..Default::default()
    };
    let out = run(
        db(&[]),
        &block(),
        &call,
        &BlockOverrides::default(),
        &HashSet::new(),
    )
    .unwrap();
    assert_eq!(
        out,
        Outcome::Invalid(format!(
            "err: insufficient funds for gas * price + value: address {} have 0 want {} \
             (supplied gas 100000)",
            NOBODY.to_checksum(None),
            100_000u64 * 30_000_000
        ))
    );
}

#[test]
fn an_account_delegated_to_an_arbos_precompile_is_refused() {
    let mut db = db(&[]);
    let mut code = vec![0xef, 0x01, 0x00];
    code.extend(address!("0x0000000000000000000000000000000000000064").as_slice());
    let code = Bytes::from(code);
    db.insert_account_info(
        CODE,
        AccountInfo {
            code_hash: keccak256(&code),
            code: Some(Bytecode::new_raw(code)),
            ..Default::default()
        },
    );
    let arbsys = address!("0x0000000000000000000000000000000000000064");
    db.insert_account_info(
        arbsys,
        AccountInfo {
            code_hash: keccak256([0xfe]),
            code: Some(Bytecode::new_raw(Bytes::from_static(&[0xfe]))),
            ..Default::default()
        },
    );
    let call = Call {
        from: NOBODY,
        to: Some(CODE),
        ..Default::default()
    };
    let out = run(
        db,
        &block(),
        &call,
        &BlockOverrides::default(),
        &HashSet::new(),
    )
    .unwrap();
    assert!(
        matches!(out, Outcome::Refused(ref m) if m.contains("delegated")),
        "{out:?}"
    );
    // Code that is 0xFE alone, undelegated, halts as on Nitro.
    assert_eq!(
        call_code(&[0xfe], &[]),
        Outcome::Halted(HaltReason::InvalidFEOpcode)
    );
}

/// `call` as one of an estimate's runs at `gas`, over `db` with ArbOS's per-transaction gas
/// limit as Robinhood Chain sets it (32M).
fn estimating(mut db: CacheDB<EmptyDB>, call: &Call, gas: u64) -> Measured {
    db.insert_account_storage(
        ARBOS_STATE,
        SLOTS.l2_per_tx_gas_limit.into(),
        U256::from(32_000_000),
    )
    .unwrap();
    estimate_run(
        db,
        &block(),
        call,
        &BlockOverrides::default(),
        &HashSet::new(),
        gas,
    )
    .unwrap()
}

fn to_code() -> Call {
    Call {
        from: NOBODY,
        to: Some(CODE),
        ..Default::default()
    }
}

#[test]
fn an_estimates_run_pays_its_l1_data_as_an_estimate_prices_it() {
    let m = estimating(db(&[0x00]), &to_code(), 1_000_000);
    assert!(matches!(m.outcome, Outcome::Returned(_)), "{m:?}");
    let poster = m.used - 21_000;
    assert!(poster > 0);
    assert_eq!(m.peak, m.used);
    // The cap's raise prices the same stand-in at the header's base fee: the same gas.
    let mut d = db(&[0x00]);
    let posting = estimate_posting_gas(&mut d, &block(), &to_code(), &BlockOverrides::default())
        .unwrap()
        .unwrap();
    assert_eq!(posting, poster);
    // Just enough to start: the intrinsic gas and the L1 data's; one less and the search raises it.
    assert!(matches!(
        estimating(db(&[0x00]), &to_code(), 21_000 + poster).outcome,
        Outcome::Returned(_)
    ));
    for gas in [21_000 + poster - 1, 21_000, 20_999] {
        let m = estimating(db(&[0x00]), &to_code(), gas);
        assert!(m.intrinsic, "{gas}: {m:?}");
        assert!(
            matches!(m.outcome, Outcome::Invalid(ref w) if w.starts_with("intrinsic gas too low")),
            "{gas}: {m:?}"
        );
    }
    // Skipping L1 charging: 21000 is enough.
    let mut skip = to_code();
    skip.skip_l1_charging = true;
    let m = estimating(db(&[0x00]), &skip, 21_000);
    assert_eq!((m.used, m.intrinsic), (21_000, false), "{m:?}");
}

#[test]
fn an_estimates_computation_is_held_to_the_per_transaction_limit() {
    // GAS, MSTORE at 0, RETURN 32 bytes: the gas the call's frame starts with, less 2.
    let code = [0x5a, 0x5f, 0x52, 0x60, 0x20, 0x5f, 0xf3];
    let m = estimating(db(&code), &to_code(), 50_000_000);
    let Outcome::Returned(word) = &m.outcome else {
        panic!("{m:?}")
    };
    assert_eq!(
        U256::from_be_slice(word),
        U256::from(32_000_000 - 21_000 - 2)
    );
    // The held gas is returned, not used.
    let poster = estimating(db(&[0x00]), &to_code(), 50_000_000).used - 21_000;
    assert!(m.used < 21_000 + poster + 100, "{m:?}");
    // Below the limit nothing is held.
    let m = estimating(db(&code), &to_code(), 1_000_000);
    let Outcome::Returned(word) = &m.outcome else {
        panic!("{m:?}")
    };
    assert_eq!(
        U256::from_be_slice(word),
        U256::from(1_000_000 - 21_000 - poster - 2)
    );
}

#[test]
fn an_estimates_refund_leaves_out_the_l1_datas_gas() {
    // Two slots cleared (SSTORE 0 at 1 and 2): 9,600 gas refunded where a fifth of the gas used
    // allows, the L1 data's gas left out of that fifth (`NonrefundableGas`).
    let code = [0x5f, 0x60, 0x01, 0x55, 0x5f, 0x60, 0x02, 0x55, 0x00];
    let mut d = db(&code);
    // A dear L1 price, so the L1 data's gas is most of the gas used.
    d.insert_account_storage(
        ARBOS_STATE,
        SLOTS.l1_price_per_unit.into(),
        U256::from(2_536_105_000u64),
    )
    .unwrap();
    for slot in [1u64, 2] {
        d.insert_account_storage(CODE, U256::from(slot), U256::from(7))
            .unwrap();
    }
    let noop = {
        let mut n = d.clone();
        n.insert_account_info(
            CODE,
            AccountInfo {
                code_hash: keccak256([0x00]),
                code: Some(Bytecode::new_raw(Bytes::from_static(&[0x00]))),
                ..Default::default()
            },
        );
        estimating(n, &to_code(), 5_000_000)
    };
    let poster = noop.used - 21_000;
    let m = estimating(d, &to_code(), 5_000_000);
    assert!(matches!(m.outcome, Outcome::Returned(_)), "{m:?}");
    let refund = m.peak - m.used;
    assert_eq!(refund, (m.peak - poster) / 5);
    assert!(refund < 9_600 && poster > 9_600 * 5, "{m:?} {poster}");
}

#[test]
fn an_estimate_where_arbos_raises_calldata_prices_is_refused() {
    let mut d = db(&[0x00]);
    d.insert_account_storage(ARBOS_STATE, SLOTS.features.into(), U256::from(1))
        .unwrap();
    let m = estimating(d, &to_code(), 1_000_000);
    assert!(
        matches!(m.outcome, Outcome::Refused(ref w) if w.contains("EIP-7623")),
        "{m:?}"
    );
}
