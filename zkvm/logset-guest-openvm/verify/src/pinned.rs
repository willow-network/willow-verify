//! The OpenVM programs a client trusts: the canonical mint's, and nothing else.
//!
//! Program keys can only be pinned from one host: a Mac build of the same source gives other
//! keys (cargo hashes the host into crate metadata, as the 10/27 GPU smoke run found). The
//! canonical keys come from the pinned x86_64 Linux mint, recorded in `pins/MINT.txt`.
//! They are read here from `pins/program-keys.txt`, which is MINT.txt's key table and field-LE
//! commits line for line (this module's tests fail otherwise), so a binary carries the keys and
//! none of the record's provenance. They are never retyped, and the verifying keys in `pins/vk/`
//! must be for exactly those keys: [`check`] refuses to build otherwise, and this module's tests
//! fail if the two drift apart.
//!
//! `pins/vk/` holds the internal-recursive verifying key every program shares (`agg.vk`) and one
//! baseline per production program (`<guest>.baseline`), bitcode-encoded. They are written from
//! the minted ELFs by `logset-tip-worker-openvm pin-vks <elf dir> <out dir>`, which first checks
//! each ELF against `pins/<guest>.sha256`; `--check` re-derives them and compares byte for byte.
//! Nothing here derives trust from an ELF a caller passes in.

#[cfg(feature = "verifier")]
use crate::{program_key, AggVk, OvmCheck};
#[cfg(feature = "verifier")]
use openvm_verify_stark_host::vk::VerificationBaseline;

/// `pins/program-keys.txt`: the trusted keys, MINT.txt's two key tables.
pub const PROGRAM_KEYS: &str = include_str!("../../pins/program-keys.txt");

/// `pins/vk/agg.vk`: the internal-recursive layer's verifying key.
pub const AGG_VK: &[u8] = include_bytes!("../../pins/vk/agg.vk");

/// The production programs, in pipeline order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// `logset-program-openvm`, a store's `leaf/` proofs.
    Leaf,
    /// `logset-idxagg-program-openvm`, a store's `l1/` proofs (the segment tier).
    L1,
    /// `logset-fold-program-openvm`, the fold over L1s (on chain after the EVM wrap).
    Fold,
}

impl Kind {
    pub const ALL: [Kind; 3] = [Kind::Leaf, Kind::L1, Kind::Fold];

    /// The guest's name in `pins/program-keys.txt` and `pins/<guest>.sha256`.
    pub fn guest(self) -> &'static str {
        match self {
            Kind::Leaf => "logset-leaf",
            Kind::L1 => "logset-idxagg",
            Kind::Fold => "logset-fold",
        }
    }

    /// `leaf`, `l1` or `fold`, as a store's tiers and the worker's commands name them.
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "leaf" => Some(Kind::Leaf),
            "l1" => Some(Kind::L1),
            "fold" => Some(Kind::Fold),
            _ => None,
        }
    }

    /// The minted ELF's sha256 (`pins/<guest>.sha256`), 64 hex digits.
    pub fn elf_sha256(self) -> &'static str {
        match self {
            Kind::Leaf => include_str!("../../pins/logset-leaf.sha256"),
            Kind::L1 => include_str!("../../pins/logset-idxagg.sha256"),
            Kind::Fold => include_str!("../../pins/logset-fold.sha256"),
        }
        .trim()
    }

    /// The pinned baseline's bytes (`pins/vk/<guest>.baseline`).
    pub fn baseline_bytes(self) -> &'static [u8] {
        match self {
            Kind::Leaf => include_bytes!("../../pins/vk/logset-leaf.baseline"),
            Kind::L1 => include_bytes!("../../pins/vk/logset-idxagg.baseline"),
            Kind::Fold => include_bytes!("../../pins/vk/logset-fold.baseline"),
        }
    }
}

/// One guest's rows in `pins/program-keys.txt` (and MINT.txt).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Minted {
    /// `logset-leaf`, `logset-index`, `logset-idxagg` or `logset-fold`.
    pub guest: &'static str,
    /// The ELF's file name.
    pub elf: &'static str,
    /// The first 8 hex digits of the ELF's sha256, as the record prints them.
    pub elf_sha256_prefix: &'static str,
    /// The program key: the exe commit's eight BabyBear limbs, each a big-endian u32.
    pub key: [u8; 32],
    /// The exe and VM commits, field-LE (each limb a little-endian u32: the `.hex` pin format).
    pub exe_field_le: [u8; 32],
    pub vm_field_le: [u8; 32],
}

fn hex32(s: &str) -> Option<[u8; 32]> {
    let h = s.strip_prefix("0x")?;
    if h.len() != 64 {
        return None;
    }
    hex::decode(h).ok()?.try_into().ok()
}

/// Every guest in `PROGRAM_KEYS`: its key-table row (`guest elf bytes sha256… key`) and its
/// field-LE commits row (under `commits, field-LE`). Each of the four guests must appear exactly
/// once in both tables.
pub fn minted() -> Result<Vec<Minted>, String> {
    parse(PROGRAM_KEYS).map_err(|e| format!("pins/program-keys.txt: {e}"))
}

/// The guests' rows in `text`, a MINT.txt or its key tables (see [`minted`]).
fn parse(text: &'static str) -> Result<Vec<Minted>, String> {
    const GUESTS: [&str; 4] = ["logset-leaf", "logset-index", "logset-idxagg", "logset-fold"];
    #[derive(PartialEq)]
    enum Table {
        None,
        Keys,
        FieldLe,
        Other,
    }
    let mut table = Table::None;
    let mut keys: Vec<(&'static str, &'static str, &'static str, [u8; 32])> = Vec::new();
    let mut commits: Vec<(&'static str, [u8; 32], [u8; 32])> = Vec::new();
    for line in text.lines() {
        let f: Vec<&'static str> = line.split_whitespace().collect();
        match f.first() {
            Some(&"guest") if line.contains("program key") => table = Table::Keys,
            Some(&"commits,") if line.contains("field-LE") => table = Table::FieldLe,
            Some(&"commits,") => table = Table::Other,
            Some(g) if g.starts_with("logset-") => match table {
                Table::Keys => {
                    let [guest, elf, _bytes, sha, key] = f[..] else {
                        return Err(format!("key row is not 5 fields: {line:?}"));
                    };
                    let key = hex32(key).ok_or(format!("key row: bad key in {line:?}"))?;
                    let sha = sha.trim_end_matches('…');
                    keys.push((guest, elf, sha, key));
                }
                Table::FieldLe => {
                    let [guest, exe, vm] = f[..] else {
                        return Err(format!("commits row is not 3 fields: {line:?}"));
                    };
                    let bad = || format!("commits row: bad commit in {line:?}");
                    commits.push((guest, hex32(exe).ok_or_else(bad)?, hex32(vm).ok_or_else(bad)?));
                }
                _ => {}
            },
            _ => {}
        }
    }
    GUESTS
        .iter()
        .map(|g| {
            let k: Vec<_> = keys.iter().filter(|r| r.0 == *g).collect();
            let c: Vec<_> = commits.iter().filter(|r| r.0 == *g).collect();
            let ([k], [c]) = (&k[..], &c[..]) else {
                return Err(format!(
                    "{} key rows and {} field-LE rows for {g} (want 1 and 1)",
                    k.len(),
                    c.len()
                ));
            };
            Ok(Minted {
                guest: k.0,
                elf: k.1,
                elf_sha256_prefix: k.2,
                key: k.3,
                exe_field_le: c.1,
                vm_field_le: c.2,
            })
        })
        .collect()
}

/// The minted program key of `kind`.
pub fn key(kind: Kind) -> Result<[u8; 32], String> {
    minted()?
        .into_iter()
        .find(|m| m.guest == kind.guest())
        .map(|m| m.key)
        .ok_or_else(|| format!("pins/program-keys.txt has no {}", kind.guest()))
}

/// The minted leaf and L1 program keys, as a client trusts OpenVM parts with them: an L1's
/// committed leaf key, and the leaves outside every segment, are the leaf's; segments the L1's.
pub fn client_keys() -> Result<logset_query::Keys, String> {
    Ok(logset_query::Keys {
        segment: vec![key(Kind::L1)?],
        leaf: vec![key(Kind::Leaf)?],
    })
}

/// The pinned internal-recursive key and the pinned baseline of each production program, after
/// checking that each baseline is for its minted key and that every baseline expects this
/// internal-recursive key.
#[cfg(feature = "verifier")]
pub fn verifying_keys() -> Result<(AggVk, Vec<(Kind, VerificationBaseline)>), String> {
    let agg: AggVk =
        bitcode::deserialize(AGG_VK).map_err(|e| format!("pins/vk/agg.vk does not decode: {e}"))?;
    let mut out = Vec::new();
    for kind in Kind::ALL {
        let g = kind.guest();
        let baseline: VerificationBaseline = bitcode::deserialize(kind.baseline_bytes())
            .map_err(|e| format!("pins/vk/{g}.baseline does not decode: {e}"))?;
        let (got, want) = (program_key(&baseline), key(kind)?);
        if got != want {
            return Err(format!(
                "pins/vk/{g}.baseline is for 0x{}, but the minted {g} key is 0x{}",
                hex::encode(got),
                hex::encode(want)
            ));
        }
        if baseline.internal_recursive_vk_commit.vk_pre_hash != agg.pre_hash {
            return Err(format!(
                "pins/vk/{g}.baseline expects another internal-recursive key than pins/vk/agg.vk"
            ));
        }
        out.push((kind, baseline));
    }
    Ok((agg, out))
}

/// The check every client of a stored OpenVM proof uses: the minted leaf, L1 and fold, under
/// the pinned verifying keys. No other program verifies.
#[cfg(feature = "verifier")]
pub fn check() -> Result<OvmCheck, String> {
    let (agg, programs) = verifying_keys()?;
    Ok(OvmCheck::new(&agg, programs.into_iter().map(|(_, b)| b)))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(feature = "verifier")]
    use logset_query::ProofCheck;
    #[cfg(feature = "verifier")]
    use openvm_stark_backend::p3_field::PrimeField32;

    fn pin(name: &str) -> String {
        let p = format!("{}/../pins/{name}", env!("CARGO_MANIFEST_DIR"));
        std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("{p}: {e}"))
    }

    #[test]
    #[cfg(feature = "verifier")]
    fn the_trusted_keys_are_exactly_the_minted_ones() {
        let c = check().expect("the pinned verifying keys load and match the minted keys");
        let want: Vec<[u8; 32]> = Kind::ALL.iter().map(|k| key(*k).unwrap()).collect();
        assert_eq!(c.keys(), want, "OvmCheck trusts other keys than the minted ones");
        // The values the 10/28 mint printed. A re-mint changes the keys and pins/vk/ together,
        // which the assert above can't see; this one makes that change a deliberate edit.
        let m = minted().unwrap();
        let printed = |g: &str| hex::encode(m.iter().find(|x| x.guest == g).unwrap().key);
        assert_eq!(printed("logset-leaf"), "28b819642cb4cbd26da3b3762408c71656423fd4267bb71e12a0c76e08490119");
        assert_eq!(printed("logset-idxagg"), "3b585e4468e56c4e4ee629bb67c85e26328301473b4e0d611dde318e47585789");
        assert_eq!(printed("logset-fold"), "5616fc714c626d144e19cb6d1f6dc17127e5e79f42aa3cd276c0589472134199");
    }

    #[test]
    #[cfg(feature = "verifier")]
    fn each_pinned_baseline_is_its_minted_exe_commit() {
        let (_, programs) = verifying_keys().unwrap();
        let m = minted().unwrap();
        for (kind, b) in programs {
            let row = m.iter().find(|x| x.guest == kind.guest()).unwrap();
            let mut le = [0u8; 32];
            for (i, limb) in b.app_exe_commit.iter().enumerate() {
                le[4 * i..4 * i + 4].copy_from_slice(&limb.as_canonical_u32().to_le_bytes());
            }
            assert_eq!(le, row.exe_field_le, "{}: baseline exe commit != minted", row.guest);
        }
    }

    #[test]
    fn the_program_keys_are_mint_txt_s_key_tables_line_for_line() {
        // pins/program-keys.txt (what binaries embed) against the mint's record, which no binary
        // embeds: from the key table's header to the CommitBytes caption, every line, in order.
        let record = pin("MINT.txt");
        let lines: Vec<&str> = record.lines().collect();
        let start = lines.iter().position(|l| l.starts_with("guest") && l.contains("program key"));
        let end = lines.iter().position(|l| l.starts_with("commits, CommitBytes"));
        let (Some(start), Some(end)) = (start, end) else {
            panic!("MINT.txt has no key table, or no CommitBytes table after it");
        };
        let ours: Vec<&str> = PROGRAM_KEYS.lines().filter(|l| !l.starts_with('#')).collect();
        assert_eq!(ours, lines[start..end], "pins/program-keys.txt != MINT.txt's key tables");
        assert_eq!(ours.len(), 11);
        // And the same rows parse out of either file.
        let record: &'static str = Box::leak(record.into_boxed_str());
        assert_eq!(parse(record).unwrap(), minted().unwrap());
    }

    #[test]
    fn mint_record_is_self_consistent_and_matches_the_build_pins() {
        let m = minted().unwrap();
        assert_eq!(m.len(), 4);
        for row in &m {
            // The key is the field-LE exe commit with each 4-byte limb byte-swapped.
            let mut swapped = row.exe_field_le;
            swapped.chunks_mut(4).for_each(|c| c.reverse());
            assert_eq!(swapped, row.key, "{}: key != exe commit limbs", row.guest);
            // The full ELF sha256 pin starts with the record's prefix.
            let sha = pin(&format!("{}.sha256", row.guest));
            assert!(
                sha.trim().starts_with(row.elf_sha256_prefix),
                "{}: pins/{}.sha256 is {} but the mint record says {}…",
                row.guest,
                row.guest,
                sha.trim(),
                row.elf_sha256_prefix
            );
        }
        let get = |g: &str| m.iter().find(|x| x.guest == g).unwrap();
        // The pins compiled into the idxagg (leaf VM) and the fold (idxagg exe and VM).
        let hex_pin = |n: &str| hex32(pin(n).trim()).unwrap_or_else(|| panic!("pins/{n}"));
        assert_eq!(hex_pin("leaf-vm-commit.hex"), get("logset-leaf").vm_field_le);
        assert_eq!(hex_pin("idxagg-exe-commit.hex"), get("logset-idxagg").exe_field_le);
        assert_eq!(hex_pin("idxagg-vm-commit.hex"), get("logset-idxagg").vm_field_le);
    }

    #[test]
    #[cfg(feature = "verifier")]
    fn builds_from_other_hosts_are_not_trusted() {
        // Keys of the same source built elsewhere (GPU-SMOKE-2026-10-27 §4: the Mac's leaf, L1
        // and fold) and the EC2 idxagg the mint replaced (MINT.txt "pins"): none may verify.
        let c = check().unwrap();
        for k in [
            "4540178666bc968d49c3a35d66f64e2156601dab5802fb452394b3b555f3478e",
            "288b79624b7e5e7d6432ccfc230600a44a48d3b414c831c51e5bd37c009f21f4",
            "70c939550bbf6c4670bc6ef035710b763a5464dd74e2e582125e606a3b9e0172",
        ] {
            let k: [u8; 32] = hex::decode(k).unwrap().try_into().unwrap();
            assert!(!c.keys().contains(&k));
        }
        assert!(c.keys().iter().all(|k| !hex::encode(k).starts_with("67e19d2c")
            && !hex::encode(k).starts_with("2c9de167")));
        // And a check under a key that isn't pinned refuses before reading the proof.
        let mut c = c;
        let other = [7u8; 32];
        assert!(c.check(&other, b"OVM2", b"").unwrap_err().starts_with("no OpenVM program"));
    }

    #[test]
    fn client_keys_are_the_minted_leaf_and_l1() {
        let k = client_keys().unwrap();
        assert_eq!(k.leaf, vec![key(Kind::Leaf).unwrap()]);
        assert_eq!(k.segment, vec![key(Kind::L1).unwrap()]);
        assert_eq!(hex::encode(&k.leaf[0][..4]), "28b81964");
        assert_eq!(hex::encode(&k.segment[0][..4]), "3b585e44");
    }

    #[test]
    fn kinds_name_the_store_tiers() {
        assert_eq!(Kind::parse("leaf"), Some(Kind::Leaf));
        assert_eq!(Kind::parse("l1"), Some(Kind::L1));
        assert_eq!(Kind::parse("fold"), Some(Kind::Fold));
        assert_eq!(Kind::parse("index"), None);
    }
}
