//! Real BNB Smart Chain headers: one either side of each fork that changed the header or its
//! seal (`eras.json.gz`), and blocks 133,194,000..=133,196,400 (2026-11-12), which cross two
//! epoch blocks and two validator-set switches (`headers-*.json.gz`).

use crate::checkpoint::{self, Checkpoint};
use crate::header::{Header, EMPTY_ROOT};
use crate::snapshot::{verify, Snapshot, Validator, Vote};
use crate::spec::{MAINNET, MAXWELL_EPOCH};
use crate::vote::fast_aggregate_verify;
use crate::Error;
use alloy_primitives::{keccak256, Address, B256, U256};
use k256::ecdsa::SigningKey;
use k256::elliptic_curve::sec1::ToEncodedPoint;
use serde_json::Value;
use std::io::Read;

fn load(name: &str) -> Vec<Value> {
    let path = format!("{}/testdata/{name}", env!("CARGO_MANIFEST_DIR"));
    let mut s = String::new();
    flate2::read::GzDecoder::new(std::fs::File::open(&path).unwrap())
        .read_to_string(&mut s)
        .unwrap();
    serde_json::from_str(&s).unwrap()
}

fn run() -> Vec<(Header, B256)> {
    load("headers-133194000-133196400.json.gz")
        .iter()
        .map(|v| Header::from_json(v).unwrap())
        .collect()
}

const FIRST: u64 = 133_194_000;

/// Follows `headers` from the first one's snapshot, keeping eight, and hands each check over.
fn follow(
    start: Snapshot,
    headers: &[(Header, B256)],
    mut each: impl FnMut(&[Snapshot], &Header, &crate::Checked),
) -> Vec<Snapshot> {
    let mut ring = vec![start];
    for (h, hash) in headers {
        let c = verify(&MAINNET, &ring, h, *hash, None)
            .unwrap_or_else(|e| panic!("block {}: {e}", h.number));
        each(&ring, h, &c);
        ring.push(c.snapshot);
        if ring.len() > 8 {
            ring.remove(0);
        }
    }
    ring
}

fn at(headers: &[(Header, B256)], n: u64) -> &(Header, B256) {
    &headers[(n - FIRST) as usize]
}

fn checkpoint_at(headers: &[(Header, B256)], n: u64) -> Snapshot {
    let cp = Checkpoint {
        chain_id: 56,
        number: n,
        hash: at(headers, n).1,
        epoch_length: MAXWELL_EPOCH,
    };
    let from = (cp.epoch() - FIRST) as usize;
    cp.snapshot(&MAINNET, &headers[from..=(n - FIRST) as usize])
        .unwrap()
}

fn set_named(headers: &[(Header, B256)], epoch: u64) -> Vec<Address> {
    let mut v: Vec<Address> = at(headers, epoch)
        .0
        .epoch_set(&MAINNET, MAXWELL_EPOCH)
        .unwrap()
        .validators
        .iter()
        .map(|x| x.0)
        .collect();
    v.sort();
    v
}

fn addresses(s: &Snapshot) -> Vec<Address> {
    s.validators.iter().map(|v| v.address).collect()
}

#[test]
fn hashes_and_seals_across_the_forks() {
    let eras = load("eras.json.gz");
    assert_eq!(eras.len(), 31);
    for v in &eras {
        let (h, hash) = Header::from_json(v).unwrap();
        assert_eq!(hash.to_string(), v["hash"].as_str().unwrap());
        // The RLP reads back to the same header.
        let (again, rehash) = Header::from_rlp(&h.rlp()).unwrap();
        assert_eq!((again, rehash), (h.clone(), hash));
        // The seal (with the tail fields from Bohr, without them before) recovers the miner.
        assert_eq!(h.signer(56).unwrap(), h.coinbase, "block {}", h.number);
        let t = h.time;
        assert_eq!(h.base_fee.is_some(), MAINNET.is_london(h.number));
        assert_eq!(h.withdrawals_hash.is_some(), MAINNET.is_cancun(t));
        assert_eq!(h.parent_beacon_root.is_some(), MAINNET.is_bohr(t));
        assert_eq!(h.requests_hash.is_some(), MAINNET.is_prague(t));
        if MAINNET.is_cancun(t) {
            assert_eq!(h.withdrawals_hash, Some(EMPTY_ROOT));
        }
    }
    // Epoch blocks of each layout: bare addresses before Luban, BLS keys after, a turn length from Bohr.
    let by = |n: u64| {
        eras.iter()
            .map(|v| Header::from_json(v).unwrap().0)
            .find(|h| h.number == n)
            .unwrap()
    };
    for (n, epoch, count, tl) in [
        (1_000_000, 200, 21, None),
        (29_020_200, 200, 21, None),
        (42_579_200, 200, 21, Some(1)),
        (48_774_000, 500, 21, Some(4)),
        (52_338_000, 1000, 21, Some(8)),
    ] {
        let set = by(n).epoch_set(&MAINNET, epoch).unwrap();
        assert_eq!(set.validators.len(), count, "block {n}");
        assert_eq!(set.turn_length, tl, "block {n}");
        // No vote keys before Luban; none registered yet at its first epoch.
        let keyed = n > 29_020_200;
        assert!(set.validators.iter().all(|v| (v.1 != [0; 48]) == keyed));
    }
    // A plain block names none; a header from before Luban carries no votes.
    assert!(by(1_000_001).validator_bytes(&MAINNET, 200).is_empty());
    assert_eq!(by(1_000_001).attestation(&MAINNET, 200), Ok(None));
    assert!(matches!(
        by(30_720_096).attestation(&MAINNET, 200),
        Ok(Some(_))
    ));
}

#[test]
fn follows_the_validators_across_two_epoch_switches() {
    let headers = run();
    let start = checkpoint_at(&headers, FIRST + 200);
    let mut verified = 0;
    let mut finalized_two_back = 0;
    let mut switches = vec![];
    let ring = follow(start.clone(), &headers[201..], |ring, h, c| {
        let parent = ring.last().unwrap();
        if addresses(parent) != addresses(&c.snapshot) {
            switches.push(h.number);
        }
        match &c.vote {
            Vote::Verified { voters, of, data } => {
                assert!(voters * 3 >= of * 2 && *of == 21);
                assert_eq!(data.target_number, h.number - 1);
                verified += 1;
            }
            Vote::Unchecked(_) => assert!(h.number <= FIRST + 204, "block {}", h.number),
            v => panic!("block {}: {v:?}", h.number),
        }
        if c.snapshot.finalized().map(|f| f.0) == Some(h.number - 2) {
            finalized_two_back += 1;
        }
    });
    // The set changes 87 blocks after each epoch block ((21 / 2 + 1) × 8 − 1), to the set it named.
    assert_eq!(switches, vec![133_195_087, 133_196_087]);
    assert_eq!(start.check_len(), 87);
    assert_eq!(
        addresses(ring.last().unwrap()),
        set_named(&headers, 133_196_000)
    );
    assert_ne!(
        set_named(&headers, 133_195_000),
        set_named(&headers, 133_196_000)
    );
    assert_eq!(addresses(&start), set_named(&headers, 133_194_000));
    // Every vote after the first few (whose validators are older than the checkpoint) verifies.
    assert!(verified >= 2_195, "{verified}");
    assert!(finalized_two_back >= 2_190, "{finalized_two_back}");
    let last = ring.last().unwrap();
    assert_eq!(last.number, 133_196_400);
    assert_eq!(last.hash, headers.last().unwrap().1);
}

#[test]
fn the_pinned_checkpoint_matches_the_chain() {
    let headers = run();
    let cp = checkpoint::MAINNET;
    let s = checkpoint_at(&headers, cp.number);
    assert_eq!((s.number, s.hash), (cp.number, cp.hash));
    // From the pinned checkpoint the chain verifies through the next switch.
    let from = (cp.number + 1 - FIRST) as usize;
    let ring = follow(s, &headers[from..], |_, _, _| {});
    assert_eq!(
        addresses(ring.last().unwrap()),
        set_named(&headers, 133_196_000)
    );
    // A wrong hash for the checkpoint is refused.
    let bad = Checkpoint {
        hash: B256::repeat_byte(1),
        ..cp
    };
    let lo = (cp.epoch() - FIRST) as usize;
    assert!(matches!(
        bad.snapshot(&MAINNET, &headers[lo..from]),
        Err(Error::Checkpoint(_))
    ));
}

/// The snapshots held just before `n`, from the checkpoint 200 past its epoch.
fn ring_before(headers: &[(Header, B256)], n: u64) -> Vec<Snapshot> {
    let start = checkpoint_at(&headers, FIRST + 200);
    follow(start, &headers[201..(n - FIRST) as usize], |_, _, _| {})
}

#[test]
fn a_tampered_header_is_refused() {
    let headers = run();
    let n = 133_195_500;
    let ring = ring_before(&headers, n);
    let (h, hash) = at(&headers, n).clone();
    assert!(verify(&MAINNET, &ring, &h, hash, None).is_ok());
    // Any covered field changed: the hash moves, and the seal no longer recovers the miner.
    let changes: [fn(&mut Header); 5] = [
        |h: &mut Header| h.gas_used += 1,
        |h: &mut Header| h.state_root = B256::repeat_byte(7),
        |h: &mut Header| h.receipts_root = B256::repeat_byte(7),
        |h: &mut Header| h.requests_hash = Some(B256::repeat_byte(7)),
        |h: &mut Header| h.extra[40] ^= 1,
    ];
    for change in changes {
        let mut t = h.clone();
        change(&mut t);
        let e = verify(&MAINNET, &ring, &t, t.hash(), None).unwrap_err();
        assert!(
            matches!(
                e,
                Error::CoinbaseMismatch(_) | Error::Unauthorized(_) | Error::BadExtra(_)
            ),
            "{e}"
        );
    }
    // JSON whose fields don't hash to the hash it reports.
    let mut v = load("headers-133194000-133196400.json.gz")[(n - FIRST) as usize].clone();
    v["gasUsed"] = Value::String("0x1".into());
    assert!(matches!(Header::from_json(&v), Err(Error::Malformed(_))));
    // A field from a later fork the seal doesn't cover.
    let mut v = load("headers-133194000-133196400.json.gz")[(n - FIRST) as usize].clone();
    v["blockAccessListHash"] = Value::String(format!("{}", B256::ZERO));
    assert!(matches!(Header::from_json(&v), Err(Error::Malformed(_))));
    // An RLP with a field past requestsHash.
    let mut rlp = h.rlp();
    let extra = [0xa0u8; 33];
    rlp.extend_from_slice(&extra);
    assert!(Header::from_rlp(&rlp).is_err());
    // Not the next block.
    let other = at(&headers, n + 1).clone();
    assert!(matches!(
        verify(&MAINNET, &ring, &other.0, other.1, None),
        Err(Error::NotChild(_))
    ));
    // From the future.
    assert!(matches!(
        verify(&MAINNET, &ring, &h, hash, Some(h.time - 10)),
        Err(Error::Field(_))
    ));
}

fn reseal(h: &mut Header, key: &SigningKey) {
    let hash = h.seal_hash(56).unwrap();
    let (sig, rec) = key.sign_prehash_recoverable(hash.as_slice()).unwrap();
    let at = h.extra.len() - 65;
    h.extra[at..at + 64].copy_from_slice(&sig.to_bytes());
    h.extra[at + 64] = rec.to_byte();
}

fn address_of(key: &SigningKey) -> Address {
    let p = key.verifying_key().to_encoded_point(false);
    Address::from_slice(&keccak256(&p.as_bytes()[1..])[12..])
}

#[test]
fn a_wrong_signer_and_a_key_outside_the_set_are_refused() {
    let headers = run();
    let n = 133_195_600;
    let ring = ring_before(&headers, n);
    let (h, _) = at(&headers, n).clone();
    let key = SigningKey::from_slice(&[0x42; 32]).unwrap();
    // Sealed by another key than the miner's.
    let mut t = h.clone();
    reseal(&mut t, &key);
    assert!(matches!(
        verify(&MAINNET, &ring, &t, t.hash(), None),
        Err(Error::CoinbaseMismatch(_))
    ));
    // Sealed by its miner, who is not a validator.
    t.coinbase = address_of(&key);
    reseal(&mut t, &key);
    assert_eq!(t.signer(56).unwrap(), t.coinbase);
    assert!(matches!(
        verify(&MAINNET, &ring, &t, t.hash(), None),
        Err(Error::Unauthorized(_))
    ));
    // The real seal with s flipped to the high half: the same signer, a malleated copy.
    let mut m = h.clone();
    let at = m.extra.len() - 65;
    let s = U256::from_be_slice(&m.extra[at + 32..at + 64]);
    let order = U256::from_str_radix(
        "fffffffffffffffffffffffffffffffebaaedce6af48a03bbfd25e8cd0364141",
        16,
    )
    .unwrap();
    m.extra[at + 32..at + 64].copy_from_slice(&(order - s).to_be_bytes::<32>());
    m.extra[at + 64] ^= 1;
    assert_eq!(m.signer(56).unwrap(), h.coinbase);
    assert!(matches!(
        verify(&MAINNET, &ring, &m, m.hash(), None),
        Err(Error::BadSeal(_))
    ));
}

#[test]
fn a_validator_removed_at_a_switch_cannot_seal_after_it() {
    let headers = run();
    // Seen by a client that missed the switch at 133,196,087 (still on the old set), a block
    // after it sealed by a validator added there is refused.
    let old = set_named(&headers, 133_195_000);
    let new = set_named(&headers, 133_196_000);
    let n = (133_196_090..133_196_400)
        .find(|n| !old.contains(&at(&headers, *n).0.coinbase))
        .unwrap();
    assert!(new.contains(&at(&headers, n).0.coinbase));
    let mut ring = ring_before(&headers, n);
    let stale = ring_before(&headers, 133_196_087);
    let parent = ring.last_mut().unwrap();
    parent.validators = stale.last().unwrap().validators.clone();
    let (h, hash) = at(&headers, n);
    assert!(matches!(
        verify(&MAINNET, &ring, h, *hash, None),
        Err(Error::Unauthorized(_))
    ));
}

#[test]
fn votes_need_the_justified_source_and_two_thirds() {
    let headers = run();
    let n = 133_195_700;
    let ring = ring_before(&headers, n);
    let (h, hash) = at(&headers, n).clone();
    let Vote::Verified { data, .. } = verify(&MAINNET, &ring, &h, hash, None).unwrap().vote else {
        panic!("no verified vote");
    };
    // The parent's justified block is not the vote's source.
    let mut r = ring.clone();
    r.last_mut()
        .unwrap()
        .attestation
        .as_mut()
        .unwrap()
        .target_number -= 1;
    assert!(matches!(
        verify(&MAINNET, &r, &h, hash, None),
        Err(Error::Attestation(_))
    ));
    // The target's set was larger: the same voters are under two thirds.
    let mut r = ring.clone();
    let i = r.len() - 1 - (n - 1 - data.target_number) as usize - 1;
    for b in 0..12u8 {
        r[i].validators.push(Validator {
            address: Address::repeat_byte(0xf0 + b),
            vote_key: [0; 48],
        });
    }
    assert!(matches!(
        verify(&MAINNET, &r, &h, hash, None),
        Err(Error::Attestation(_))
    ));
    // The aggregate signature itself: the voters' keys over the vote verify, and nothing else does.
    let a = h.attestation(&MAINNET, MAXWELL_EPOCH).unwrap().unwrap();
    let set = &ring[i].validators;
    let keys: Vec<[u8; 48]> = set
        .iter()
        .enumerate()
        .filter(|(j, _)| a.voters & (1 << j) != 0)
        .map(|(_, v)| v.vote_key)
        .collect();
    assert!(fast_aggregate_verify(&keys, &a.signature, &a.data.hash()));
    assert!(!fast_aggregate_verify(
        &keys[1..],
        &a.signature,
        &a.data.hash()
    ));
    assert!(!fast_aggregate_verify(
        &keys,
        &a.signature,
        &B256::repeat_byte(3)
    ));
    let mut sig = a.signature;
    sig[50] ^= 1;
    assert!(!fast_aggregate_verify(&keys, &sig, &a.data.hash()));
}

/// Three test validators, turn length 1, from a real Fermi-era header's fields.
fn synthetic() -> (Snapshot, Header, Vec<SigningKey>) {
    let headers = run();
    let (template, _) = at(&headers, 133_195_601).clone();
    let mut keys: Vec<SigningKey> = (1..=3u8)
        .map(|i| SigningKey::from_slice(&[i; 32]).unwrap())
        .collect();
    keys.sort_by_key(address_of);
    let s = Snapshot {
        number: 133_195_601,
        hash: B256::repeat_byte(9),
        time: template.time,
        milli_time: template.milli_time(),
        gas_limit: template.gas_limit,
        epoch_length: MAXWELL_EPOCH,
        block_interval_ms: 450,
        turn_length: 1,
        validators: keys
            .iter()
            .map(|k| Validator {
                address: address_of(k),
                vote_key: [0; 48],
            })
            .collect(),
        recents: Default::default(),
        attestation: None,
        next: None,
    };
    (s, template, keys)
}

fn child(parent: &Snapshot, template: &Header, key: &SigningKey, difficulty: u8) -> Header {
    let mut h = template.clone();
    h.number = parent.number + 1;
    h.parent_hash = parent.hash;
    h.time = (parent.milli_time + 450) / 1000;
    h.mix_digest = B256::left_padding_from(&((parent.milli_time + 450) % 1000).to_be_bytes());
    h.coinbase = address_of(key);
    h.difficulty = U256::from(difficulty);
    h.extra = [&template.extra[..32], &[0u8; 65][..]].concat();
    reseal(&mut h, key);
    h
}

#[test]
fn turn_rules_are_kept() {
    let (s, template, keys) = synthetic();
    let in_turn = keys
        .iter()
        .position(|k| address_of(k) == s.in_turn())
        .unwrap();
    // The in-turn validator claiming the out-of-turn difficulty, and the reverse.
    let h = child(&s, &template, &keys[in_turn], 1);
    assert!(matches!(
        verify(&MAINNET, &[s.clone()], &h, h.hash(), None),
        Err(Error::WrongDifficulty(_))
    ));
    let other = (in_turn + 1) % 3;
    let h = child(&s, &template, &keys[other], 2);
    assert!(matches!(
        verify(&MAINNET, &[s.clone()], &h, h.hash(), None),
        Err(Error::WrongDifficulty(_))
    ));
    // In turn, then the same validator again at once: over its turn length of 1.
    let h = child(&s, &template, &keys[in_turn], 2);
    let c = verify(&MAINNET, &[s.clone()], &h, h.hash(), None).unwrap();
    assert!(c.in_turn);
    let next = c.snapshot;
    let diff = if next.in_turn() == address_of(&keys[in_turn]) {
        2
    } else {
        1
    };
    let h2 = child(&next, &template, &keys[in_turn], diff);
    assert!(matches!(
        verify(&MAINNET, &[s, next.clone()], &h2, h2.hash(), None),
        Err(Error::RecentlySigned(_))
    ));
    // Another validator may seal it.
    let k = keys
        .iter()
        .find(|k| address_of(k) != address_of(&keys[in_turn]))
        .unwrap();
    let diff = if next.in_turn() == address_of(k) {
        2
    } else {
        1
    };
    let h3 = child(&next, &template, k, diff);
    assert!(verify(&MAINNET, &[next.clone()], &h3, h3.hash(), None).is_ok());
    // Too soon after its parent, and a mixHash that is not milliseconds.
    let mut early = h3.clone();
    early.time = next.time;
    early.mix_digest = B256::left_padding_from(&(next.milli_time % 1000).to_be_bytes());
    reseal(&mut early, k);
    assert!(matches!(
        verify(&MAINNET, &[next.clone()], &early, early.hash(), None),
        Err(Error::Field(_))
    ));
    let mut mix = h3.clone();
    mix.mix_digest = B256::repeat_byte(1);
    reseal(&mut mix, k);
    assert!(matches!(
        verify(&MAINNET, &[next], &mix, mix.hash(), None),
        Err(Error::Field(_))
    ));
}

#[test]
fn a_snapshot_reads_back_from_its_record() {
    let headers = run();
    for n in [133_195_001, 133_196_050, 133_196_090] {
        let ring = ring_before(&headers, n);
        let s = ring.last().unwrap();
        assert_eq!(
            &Snapshot::from_json(&s.to_json()).unwrap(),
            s,
            "after {}",
            n - 1
        );
    }
}
