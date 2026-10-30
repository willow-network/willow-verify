Data dir for test/real-openvm.mjs and the OpenVM verifiers' Rust tests: a real OpenVM 2.0.2 proof of
Base blocks 52969856-52969983, proven on 2026-10-28 on an RTX PRO 6000 under the minted programs
(zkvm/logset-guest-openvm/pins/MINT.txt: L1 0x3b585e44...).

store/l1/52969856-52969983.proof  the L1 over two 64-block leaves (52969856-52969919 and
                                  52969920-52969983), whose proofs it carries as deferrals
segment.json  the willow_getLogsProof answer for 52969856-52969983 with address
              0xd9aaec86b65d86f6a7b5b1b0c42ffa531710b6ca and topic0 Transfer (a segment part)

The .json is {"filter", "logs": <how many logs match>, "result": <the willow_getLogsProof result>},
made with `cargo run --example store_answer -- <store> 8453 52969856 52969983 '<filter>'` in
zkvm/logset-guest/query from the prover's store.
