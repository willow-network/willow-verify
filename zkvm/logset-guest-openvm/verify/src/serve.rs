//! The line protocol `willow-logs-verify` (the SP1 CLI, which cannot link OpenVM: see
//! Cargo.toml) speaks to this crate's binary, `willow-logs-verify-openvm serve`, to verify the
//! OpenVM proofs of an answer. One line each way, in order:
//!
//! - first, from the binary: `ready <leaf key> <l1 key> <fold key>` (the minted program keys,
//!   64 hex digits each, the only ones it verifies under), or `error <why>` before it exits 1;
//! - then, per proof, the request `<program key> <public values> <proof>`, all hex, and the answer
//!   `ok` or `fail <why>` (on one line).
//!
//! End of input ends it. Trust stays in [`pinned`](crate::pinned): a key that is not minted is
//! refused like any other failure, whatever the caller sends.

use crate::pinned::{self, Kind};
use crate::OvmCheck;
use logset_query::ProofCheck;
use std::io::{self, BufRead, Write};

/// The first line: the minted keys this check verifies under.
pub fn ready_line() -> Result<String, String> {
    let keys = Kind::ALL
        .iter()
        .map(|k| pinned::key(*k).map(hex::encode))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(format!("ready {}", keys.join(" ")))
}

/// The answer to one request line.
pub fn answer(check: &mut OvmCheck, line: &str) -> String {
    match request(line).and_then(|(key, pv, proof)| check.check(&key, &proof, &pv)) {
        Ok(()) => "ok".into(),
        Err(e) => format!("fail {}", e.replace(['\n', '\r'], " ")),
    }
}

fn request(line: &str) -> Result<([u8; 32], Vec<u8>, Vec<u8>), String> {
    let mut f = line.split_ascii_whitespace();
    let (Some(key), Some(pv), Some(proof), None) = (f.next(), f.next(), f.next(), f.next()) else {
        return Err("request is not `<key> <public values> <proof>`".into());
    };
    let key: [u8; 32] = hex::decode(key)
        .ok()
        .and_then(|k| k.try_into().ok())
        .ok_or("program key is not 32 hex bytes")?;
    let pv = hex::decode(pv).map_err(|e| format!("public values: {e}"))?;
    let proof = hex::decode(proof).map_err(|e| format!("proof: {e}"))?;
    Ok((key, pv, proof))
}

/// Serves requests from `input` until it ends.
pub fn run(input: impl BufRead, mut output: impl Write) -> io::Result<()> {
    let ready = ready_line().and_then(|r| pinned::check().map(|c| (r, c)));
    let (ready, mut check) = match ready {
        Ok(x) => x,
        Err(e) => {
            writeln!(output, "error {}", e.replace(['\n', '\r'], " "))?;
            output.flush()?;
            return Err(io::Error::other(e));
        }
    };
    writeln!(output, "{ready}")?;
    output.flush()?;
    for line in input.lines() {
        let line = line?;
        writeln!(output, "{}", answer(&mut check, &line))?;
        output.flush()?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn answers_each_line_in_order_under_the_minted_keys_only() {
        let leaf = hex::encode(pinned::key(Kind::Leaf).unwrap());
        let input = format!(
            "{leaf} 00 4f564d32\n{} 00 4f564d32\ntwo fields\n{leaf} zz 00\n",
            "07".repeat(32)
        );
        let mut out = Vec::new();
        run(input.as_bytes(), &mut out).unwrap();
        let out = String::from_utf8(out).unwrap();
        let lines: Vec<&str> = out.lines().collect();
        assert_eq!(lines.len(), 5, "{out}");
        assert_eq!(lines[0], ready_line().unwrap());
        assert!(lines[0].starts_with("ready 28b81964"), "{}", lines[0]);
        assert!(lines[1].starts_with("fail proof bytes"), "{}", lines[1]);
        assert!(
            lines[2].starts_with("fail no OpenVM program with key 0x0707"),
            "{}",
            lines[2]
        );
        assert_eq!(
            lines[3],
            "fail request is not `<key> <public values> <proof>`"
        );
        assert!(lines[4].starts_with("fail public values"), "{}", lines[4]);
    }
}
