//! Write synthetic (shape-exact, contents-fake) witness blobs to a directory, so
//! `execrun` has something to run before `genblob`'s real mainnet fixtures land.
//!
//! ```sh
//! cargo run --release -p window-anchor-core --features synth --example dump_blob -- <out_dir>
//! ```
//!
//! Each blob is verified with the native `run()` before it is written, and the
//! statement it produces is printed as hex — the value `execrun` must see the
//! zkVM commit.

use window_anchor_core::synth::Synth;
use window_anchor_core::{encode_blob, run, PathKind, CAPELLA_SLOT, SLOTS_PER_HISTORICAL_ROOT};

fn main() {
    let out = std::env::args().nth(1).unwrap_or_else(|| {
        eprintln!("usage: dump_blob <out_dir>");
        std::process::exit(2);
    });
    std::fs::create_dir_all(&out).expect("create out dir");

    // A Fulu anchor at slot 13,300,000 for both, so the state container is the
    // current (depth-6) shape.
    let fulu_anchor = 13_300_000u64;
    let cases: [(&str, Synth); 3] = [
        // Recent: ~2.7 h behind the anchor, inside the 8,192-slot ring.
        ("synth-recent-fulu", Synth::recent(fulu_anchor, 800)),
        // Historical: a Deneb-era target, chunk 700.
        (
            "synth-historical-fulu",
            Synth::historical(
                fulu_anchor,
                CAPELLA_SLOT + 700 * SLOTS_PER_HISTORICAL_ROOT + 4_321,
            ),
        ),
        // Historical with a 15-leaf (Capella) ExecutionPayload — the smaller of
        // the two payload shapes, one tree level shallower.
        (
            "synth-historical-capella-target",
            Synth::historical(fulu_anchor, 6_300_000),
        ),
    ];

    for (name, s) in cases {
        let w = s.build();
        let blob = encode_blob(&w).expect("encode");
        let stmt = run(&blob).expect("synthetic fixture must verify natively");
        let path = format!("{out}/{name}.bin");
        std::fs::write(&path, &blob).expect("write blob");
        println!(
            "{path}\n  bytes={}  path={}  anchor_slot={}  target_slot={}  siblings={}  payload_leaves={}",
            blob.len(),
            if s.path == PathKind::Recent { "recent" } else { "historical" },
            s.anchor_slot,
            s.target_slot,
            w.siblings.len(),
            w.payload_leaves.len(),
        );
        println!("  statement=0x{}", hex_encode(&stmt.to_bytes()));
    }
}

fn hex_encode(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push_str(&format!("{b:02x}"));
    }
    s
}
