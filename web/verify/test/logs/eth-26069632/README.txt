Data dir for test/real-openvm.mjs and the OpenVM verifiers' Rust tests: real OpenVM 2.0.2 proofs of
Ethereum blocks 26069632-26069695 (one 64-block window), proven on 2026-10-28 on an RTX PRO 6000 under
the minted programs (zkvm/logset-guest-openvm/pins/MINT.txt: leaf 0x28b81964..., L1 0x3b585e44...).

store/leaf/26069632-<hash>.proof  the leaf of block 26069632 (magic "OVM2", then zstd of VmStarkProof)
store/l1/26069632-26069695.proof   the L1 over the window's 64 one-block leaves, whose proofs it
                                   carries as deferrals
block.json    the willow_getLogsProof answer for block 26069632 alone (a block part), no filter
segment.json  the answer for 26069632-26069695 with address 0x000000000022d473030f116ddee9f6b43ac78ba3
              (a segment part with its range proofs)

Each .json is {"filter", "logs": <how many logs match>, "result": <the willow_getLogsProof result>},
made with `cargo run --example store_answer -- <store> 1 <from> <to> ['<filter>']` in
zkvm/logset-guest/query from the prover's store (the same SegmentIndex the log RPC serves from).
The same window was proven on three other cards; their L1 proofs differ byte for byte (OpenVM
proofs are not deterministic) and all verify under the same key.
