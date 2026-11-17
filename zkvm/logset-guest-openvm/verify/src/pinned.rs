
















#[cfg(feature = "verifier")]
use crate::{program_key, AggVk, OvmCheck};
#[cfg(feature = "verifier")]
use openvm_verify_stark_host::vk::VerificationBaseline;


pub const PROGRAM_KEYS: &str = include_str!("../../pins/program-keys.txt");


pub const AGG_VK: &[u8] = include_bytes!("../../pins/vk/agg.vk");


#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {

    Leaf,

    L1,


}

impl Kind {
    pub const ALL: [Kind; 2] = [Kind::Leaf, Kind::L1];


    pub fn guest(self) -> &'static str {
        match self {
            Kind::Leaf => "logset-leaf",
            Kind::L1 => "logset-idxagg",

        }
    }


    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "leaf" => Some(Kind::Leaf),
            "l1" => Some(Kind::L1),

            _ => None,
        }
    }


    pub fn elf_sha256(self) -> &'static str {
        match self {
            Kind::Leaf => include_str!("../../pins/logset-leaf.sha256"),
            Kind::L1 => include_str!("../../pins/logset-idxagg.sha256"),

        }
        .trim()
    }


    pub fn baseline_bytes(self) -> &'static [u8] {
        match self {
            Kind::Leaf => include_bytes!("../../pins/vk/logset-leaf.baseline"),
            Kind::L1 => include_bytes!("../../pins/vk/logset-idxagg.baseline"),

        }
    }
}


#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Minted {

    pub guest: &'static str,

    pub elf: &'static str,

    pub elf_sha256_prefix: &'static str,

    pub key: [u8; 32],

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




pub fn minted() -> Result<Vec<Minted>, String> {
    parse(PROGRAM_KEYS).map_err(|e| format!("pins/program-keys.txt: {e}"))
}


fn parse(text: &'static str) -> Result<Vec<Minted>, String> {
    const GUESTS: [&str; 3] = ["logset-leaf", "logset-index", "logset-idxagg"];
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


pub fn key(kind: Kind) -> Result<[u8; 32], String> {
    minted()?
        .into_iter()
        .find(|m| m.guest == kind.guest())
        .map(|m| m.key)
        .ok_or_else(|| format!("pins/program-keys.txt has no {}", kind.guest()))
}



pub fn client_keys() -> Result<logset_query::Keys, String> {
    Ok(logset_query::Keys {
        segment: vec![key(Kind::L1)?],
        leaf: vec![key(Kind::Leaf)?],
    })
}




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



#[cfg(feature = "verifier")]
pub fn check() -> Result<OvmCheck, String> {
    let (agg, programs) = verifying_keys()?;
    Ok(OvmCheck::new(&agg, programs.into_iter().map(|(_, b)| b)))
}


































































































































