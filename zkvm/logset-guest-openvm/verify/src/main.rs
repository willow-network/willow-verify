//! willow-logs-verify-openvm: OpenVM proofs of a Willow log RPC answer, under the minted programs
//! only (`pins/program-keys.txt`, `pins/vk/`).
//!
//! ```sh
//! willow-logs-verify-openvm serve                            # for willow-logs-verify (src/serve.rs)
//! willow-logs-verify-openvm keys                             # the minted leaf, L1 and fold keys
//! willow-logs-verify-openvm verify <leaf|l1|fold> <proof> <pv>  # one stored proof against its pv
//! ```
//! `willow-logs-verify` (the SP1 CLI) cannot link OpenVM's verifier, so it runs `serve` for the
//! OpenVM parts of an answer: put this binary next to it, or pass `--openvm-verifier <path>`.
//! Exit codes: 0 success; 1 a failed check (or `serve` could not load the pinned keys); 2 usage.

use logset_query::ProofCheck;
use std::time::Instant;
use willow_logs_verify_openvm::pinned::{self, Kind};

fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    match a.first().map(String::as_str) {
        Some("serve") if a.len() == 1 => {
            let stdin = std::io::stdin();
            if willow_logs_verify_openvm::serve::run(stdin.lock(), std::io::stdout()).is_err() {
                std::process::exit(1)
            }
        }
        Some("keys") if a.len() == 1 => {
            for k in Kind::ALL {
                match pinned::key(k) {
                    Ok(key) => println!("{} 0x{}", k.guest(), hex::encode(key)),
                    Err(e) => fail(e),
                }
            }
        }
        Some("verify") if a.len() == 4 => verify(&a[1], &a[2], &a[3]),
        _ => {
            eprintln!("usage: willow-logs-verify-openvm serve | keys | verify <leaf|l1|fold> <proof> <pv>");
            std::process::exit(2)
        }
    }
}

fn verify(kind: &str, proof: &str, pv: &str) {
    let Some(kind) = Kind::parse(kind) else {
        eprintln!("verify: tier {kind:?} is not leaf, l1 or fold");
        std::process::exit(2)
    };
    let (proof, pv) = match (std::fs::read(proof), std::fs::read(pv)) {
        (Ok(p), Ok(v)) => (p, v),
        (Err(e), _) => fail(format!("read {proof}: {e}")),
        (_, Err(e)) => fail(format!("read {pv}: {e}")),
    };
    let mut check = pinned::check().unwrap_or_else(|e| fail(e));
    let key = pinned::key(kind).unwrap_or_else(|e| fail(e));
    let t = Instant::now();
    match check.check(&key, &proof, &pv) {
        Ok(()) => println!(
            "OK {} 0x{} in {:.1} ms",
            kind.guest(),
            hex::encode(key),
            t.elapsed().as_secs_f64() * 1e3
        ),
        Err(e) => fail(format!(
            "not a proof of this pv under the minted {}: {e}",
            kind.guest()
        )),
    }
}

/// A failed check: one line on stdout, exit 1.
fn fail(why: String) -> ! {
    println!("FAIL {why}");
    std::process::exit(1)
}
