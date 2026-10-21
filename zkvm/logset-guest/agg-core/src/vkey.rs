//! Child-program vkeys compiled into an aggregator ELF from TRACKED files.
//!
//! An aggregator that pins its child program must carry the child's vkey in its
//! own ELF, so the aggregator's vkey transitively pins the child. The prototypes
//! read it from `env!("LOGSET_CHILD_VKEY")`, which `cargo prove build --docker`
//! cannot forward (sp1-build 6.3.1 `command/docker.rs` passes only its own fixed
//! env list), so no aggregator could be minted reproducibly. The pin now lives in
//! a tracked file under `zkvm/logset-guest/vkeys/`, read with `include_str!`:
//! the file is part of the source tree the docker build mounts, so the ELF is a
//! function of the committed source alone.
//!
//! File format: the KoalaBear form of the vkey, `words_to_bytes_be(vk.hash_u32())`,
//! as 64 lowercase-or-uppercase hex characters, optionally followed by ONE `\n`.
//! Anything else fails the build (a `const` evaluation panic), never the proof.
//! `logset-vkey <elf> --write <file>` writes it and `--check <file>` compares.

/// Parse a vkey file's contents into the eight `u32` words `verify_sp1_proof`
/// takes. `const`, so a malformed file is a compile error in the guest.
pub const fn parse_vkey_hex(s: &str) -> [u32; 8] {
    let b = s.as_bytes();
    let n = if b.len() == 65 && b[64] == b'\n' {
        64
    } else {
        b.len()
    };
    assert!(
        n == 64,
        "vkey file must be exactly 64 hex chars (optionally one trailing newline)"
    );
    let mut w = [0u32; 8];
    let mut i = 0;
    while i < 64 {
        w[i / 8] = (w[i / 8] << 4) | nib(b[i]);
        i += 1;
    }
    w
}

const fn nib(c: u8) -> u32 {
    match c {
        b'0'..=b'9' => (c - b'0') as u32,
        b'a'..=b'f' => (c - b'a' + 10) as u32,
        b'A'..=b'F' => (c - b'A' + 10) as u32,
        _ => panic!("vkey file: not hex"),
    }
}

/// The 32 bytes an aggregator COMMITS for a child vkey it verified against:
/// the eight words big-endian, i.e. exactly the bytes the hex file spells.
pub fn vkey_words_to_bytes(w: &[u32; 8]) -> [u8; 32] {
    let mut out = [0u8; 32];
    for (i, x) in w.iter().enumerate() {
        out[4 * i..4 * i + 4].copy_from_slice(&x.to_be_bytes());
    }
    out
}

/// Inverse of [`vkey_words_to_bytes`].
pub fn vkey_bytes_to_words(b: &[u8; 32]) -> [u32; 8] {
    let mut w = [0u32; 8];
    for (i, x) in w.iter_mut().enumerate() {
        *x = u32::from_be_bytes([b[4 * i], b[4 * i + 1], b[4 * i + 2], b[4 * i + 3]]);
    }
    w
}

#[cfg(test)]
mod tests {
    use super::*;

    const HEX: &str = "7bd797be0123456789abcdef00000000ffffffff1111111122222222deadc036";

    #[test]
    fn parses_bare_and_newline_forms() {
        let a = parse_vkey_hex(HEX);
        let with_nl = format!("{HEX}\n");
        let b = parse_vkey_hex(&with_nl);
        assert_eq!(a, b);
        assert_eq!(a[0], 0x7bd7_97be);
        assert_eq!(a[7], 0xdead_c036);
        assert_eq!(parse_vkey_hex(&HEX.to_uppercase()), a);
    }

    #[test]
    fn words_bytes_round_trip_and_spell_the_file() {
        let w = parse_vkey_hex(HEX);
        let b = vkey_words_to_bytes(&w);
        let hex: String = b.iter().map(|x| format!("{x:02x}")).collect();
        assert_eq!(hex, HEX);
        assert_eq!(vkey_bytes_to_words(&b), w);
    }

    #[test]
    #[should_panic]
    fn rejects_short() {
        parse_vkey_hex(&HEX[..63]);
    }

    #[test]
    #[should_panic]
    fn rejects_two_newlines() {
        parse_vkey_hex(&format!("{HEX}\n\n"));
    }

    #[test]
    #[should_panic]
    fn rejects_0x_prefix() {
        parse_vkey_hex(&format!("0x{}", &HEX[..62]));
    }

    #[test]
    #[should_panic]
    fn rejects_non_hex() {
        parse_vkey_hex(&HEX.replace('7', "g"));
    }

    /// Every tracked pin file must parse; a bad one would fail the guest build.
    #[test]
    fn tracked_pin_files_parse() {
        let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../vkeys");
        let mut n = 0;
        for e in std::fs::read_dir(dir).expect("vkeys dir") {
            let p = e.unwrap().path();
            if p.extension().and_then(|x| x.to_str()) == Some("koalabear") {
                let s = std::fs::read_to_string(&p).unwrap();
                let _ = parse_vkey_hex(&s);
                n += 1;
            }
        }
        assert!(n >= 1, "at least one tracked pin");
    }
}
