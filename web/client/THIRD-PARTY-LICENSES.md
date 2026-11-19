# Third-party licenses of the wasm modules

The WebAssembly modules in `wasm/` are compiled from Willow's verifier crates (MIT, `LICENSE`), the Rust standard library and the third-party Rust crates below, which are linked into them. For each crate: its license as its package states it, and the license files its package carries, word for word; for a package that carries none, those of the repository it comes from, or the standard text of its license under its authors. Generated from the modules' `Cargo.lock` files.

| Crate | Version | License | In |
| :-- | :-- | :-- | :-- |
| addr2line | 0.25.1 | Apache-2.0 OR MIT | verifier |
| addr2line | 0.27.1 | Apache-2.0 OR MIT | openvm |
| adler2 | 2.0.1 | 0BSD OR MIT OR Apache-2.0 | openvm, verifier |
| ahash | 0.8.12 | MIT OR Apache-2.0 | anchor, openvm, state, verifier |
| aho-corasick | 1.1.5 | Unlicense OR MIT | openvm |
| alloc-no-stdlib | 2.0.4 | BSD-3-Clause | state |
| alloc-stdlib | 0.2.4 | BSD-3-Clause | state |
| allocator-api2 | 0.2.21 | MIT OR Apache-2.0 | anchor, state, verifier |
| alloy | 1.5.2 | MIT OR Apache-2.0 | items, state |
| alloy-chains | 0.2.34 | MIT OR Apache-2.0 | state |
| alloy-consensus | 1.8.3 | MIT OR Apache-2.0 | items, state |
| alloy-consensus | 2.0.4 | MIT OR Apache-2.0 | state |
| alloy-consensus-any | 1.8.3 | MIT OR Apache-2.0 | items, state |
| alloy-consensus-any | 2.0.4 | MIT OR Apache-2.0 | state |
| alloy-core | 1.5.7 | MIT OR Apache-2.0 | state |
| alloy-core | 1.7.1 | MIT OR Apache-2.0 | items |
| alloy-eip2124 | 0.2.0 | MIT OR Apache-2.0 | items, state |
| alloy-eip2930 | 0.2.3 | MIT OR Apache-2.0 | state |
| alloy-eip2930 | 0.2.4 | MIT OR Apache-2.0 | anchor, items |
| alloy-eip7702 | 0.6.3 | MIT OR Apache-2.0 | anchor, items, state |
| alloy-eip7928 | 0.3.7 | MIT OR Apache-2.0 | items, state |
| alloy-eips | 1.8.3 | MIT OR Apache-2.0 | items, state |
| alloy-eips | 2.0.4 | MIT OR Apache-2.0 | state |
| alloy-evm | 0.21.3 | MIT OR Apache-2.0 | state |
| alloy-hardforks | 0.3.5 | MIT OR Apache-2.0 | state |
| alloy-network-primitives | 1.8.3 | MIT OR Apache-2.0 | items, state |
| alloy-network-primitives | 2.0.4 | MIT OR Apache-2.0 | state |
| alloy-primitives | 1.5.7 | MIT OR Apache-2.0 | state |
| alloy-primitives | 1.7.1 | MIT OR Apache-2.0 | items |
| alloy-primitives | 1.7.3 | MIT OR Apache-2.0 | anchor |
| alloy-rlp | 0.3.16 | MIT OR Apache-2.0 | anchor, items |
| alloy-rlp | 0.3.17 | MIT OR Apache-2.0 | state |
| alloy-rpc-types | 1.5.2 | MIT OR Apache-2.0 | state |
| alloy-rpc-types | 1.8.3 | MIT OR Apache-2.0 | items |
| alloy-rpc-types-eth | 1.5.2 | MIT OR Apache-2.0 | state |
| alloy-rpc-types-eth | 1.8.3 | MIT OR Apache-2.0 | items |
| alloy-rpc-types-eth | 2.0.4 | MIT OR Apache-2.0 | state |
| alloy-rpc-types-trace | 1.5.2 | MIT OR Apache-2.0 | state |
| alloy-rpc-types-trace | 2.0.4 | MIT OR Apache-2.0 | state |
| alloy-serde | 1.8.3 | MIT OR Apache-2.0 | items, state |
| alloy-serde | 2.0.4 | MIT OR Apache-2.0 | state |
| alloy-sol-types | 1.5.7 | MIT OR Apache-2.0 | state |
| alloy-sol-types | 1.7.1 | MIT OR Apache-2.0 | items |
| alloy-trie | 0.9.5 | MIT OR Apache-2.0 | items |
| alloy-trie | 0.9.8 | MIT OR Apache-2.0 | anchor |
| alloy-trie | 0.9.9 | MIT OR Apache-2.0 | state |
| ansi_term | 0.12.1 | MIT | openvm, verifier |
| anstyle | 1.0.14 | MIT OR Apache-2.0 | state |
| anyhow | 1.0.104 | MIT OR Apache-2.0 | items, state |
| ark-bls12-381 | 0.5.0 | MIT OR Apache-2.0 | anchor, state |
| ark-bn254 | 0.5.0 | MIT OR Apache-2.0 | anchor, state |
| ark-ec | 0.5.0 | MIT OR Apache-2.0 | anchor, state |
| ark-ff | 0.4.2 | MIT/Apache-2.0 | openvm |
| ark-ff | 0.5.0 | MIT OR Apache-2.0 | anchor, state |
| ark-poly | 0.5.0 | MIT OR Apache-2.0 | anchor, state |
| ark-serialize | 0.4.2 | MIT/Apache-2.0 | openvm |
| ark-serialize | 0.5.0 | MIT OR Apache-2.0 | anchor, state |
| ark-std | 0.4.0 | MIT/Apache-2.0 | openvm |
| ark-std | 0.5.0 | MIT/Apache-2.0 | anchor, state |
| arrayref | 0.3.9 | BSD-2-Clause | anchor, state, verifier |
| arrayvec | 0.7.8 | MIT OR Apache-2.0 | anchor, items, openvm, state, verifier |
| aurora-engine-modexp | 1.3.0 | CC0-1.0 | anchor, state |
| backtrace | 0.3.76 | MIT OR Apache-2.0 | verifier |
| backtrace | 0.3.77 | MIT OR Apache-2.0 | openvm |
| base16ct | 0.2.0 | Apache-2.0 OR MIT | anchor, items, state |
| base64 | 0.22.1 | MIT OR Apache-2.0 | anchor, stellar |
| bincode | 1.3.3 | MIT | verifier |
| bitcode | 0.6.9 | MIT OR Apache-2.0 | openvm |
| bitflags | 2.13.2 | MIT OR Apache-2.0 | anchor, state |
| bitvec | 1.1.1 | MIT | anchor, openvm, state, verifier |
| blake2 | 0.10.6 | MIT OR Apache-2.0 | openvm |
| blake2b_simd | 1.0.5 | MIT | openvm |
| blake3 | 1.8.7 | CC0-1.0 OR Apache-2.0 OR Apache-2.0 WITH LLVM-exception | verifier |
| block-buffer | 0.10.4 | MIT OR Apache-2.0 | anchor, items, openvm, state, stellar, verifier |
| block-buffer | 0.12.1 | MIT OR Apache-2.0 | anchor, items |
| block-buffer | 0.9.0 | MIT OR Apache-2.0 | anchor |
| bls12_381 | 0.7.1 | MIT/Apache-2.0 | openvm |
| bls12_381 | 0.8.0 | MIT/Apache-2.0 | anchor |
| brotli | 8.0.4 | BSD-3-Clause AND MIT | state |
| brotli-decompressor | 5.0.3 | BSD-3-Clause/MIT | state |
| bytemuck | 1.25.2 | Zlib OR Apache-2.0 OR MIT | openvm, verifier |
| byteorder | 1.5.0 | Unlicense OR MIT | openvm, stellar, verifier |
| bytes | 1.12.1 | MIT | anchor, items, state |
| bytesize | 2.7.0 | Apache-2.0 | openvm |
| cfg-if | 1.0.4 | MIT OR Apache-2.0 | items, verifier |
| cfg-if | 1.0.5 | MIT OR Apache-2.0 | anchor, openvm, state, stellar |
| cobs | 0.3.0 | MIT OR Apache-2.0 | openvm |
| colorchoice | 1.0.5 | MIT OR Apache-2.0 | state |
| const-hex | 1.19.1 | MIT OR Apache-2.0 | items |
| const-hex | 1.19.3 | MIT OR Apache-2.0 | anchor, state |
| const-oid | 0.10.2 | Apache-2.0 OR MIT | anchor |
| const-oid | 0.9.6 | Apache-2.0 OR MIT | anchor, items, state |
| constant_time_eq | 0.4.2 | CC0-1.0 OR MIT-0 OR Apache-2.0 | openvm, verifier |
| crc | 3.4.0 | MIT OR Apache-2.0 | items, state |
| crc-catalog | 2.5.0 | MIT OR Apache-2.0 | items, state |
| crossbeam | 0.8.4 | MIT OR Apache-2.0 | verifier |
| crossbeam-channel | 0.5.16 | MIT OR Apache-2.0 | verifier |
| crossbeam-deque | 0.8.7 | MIT OR Apache-2.0 | verifier |
| crossbeam-deque | 0.8.8 | MIT OR Apache-2.0 | openvm |
| crossbeam-epoch | 0.9.20 | MIT OR Apache-2.0 | verifier |
| crossbeam-epoch | 0.9.21 | MIT OR Apache-2.0 | openvm |
| crossbeam-queue | 0.3.13 | MIT OR Apache-2.0 | verifier |
| crossbeam-utils | 0.8.22 | MIT OR Apache-2.0 | verifier |
| crossbeam-utils | 0.8.23 | MIT OR Apache-2.0 | openvm |
| crunchy | 0.2.4 | MIT | items, openvm, state, stellar, verifier |
| crypto-bigint | 0.5.5 | Apache-2.0 OR MIT | anchor, items, state |
| crypto-common | 0.1.6 | MIT OR Apache-2.0 | items, state, verifier |
| crypto-common | 0.1.7 | MIT OR Apache-2.0 | anchor, openvm, stellar |
| crypto-common | 0.2.2 | MIT OR Apache-2.0 | anchor, items |
| curve25519-dalek | 4.1.3 | BSD-3-Clause | stellar |
| dashmap | 6.2.1 | MIT | openvm |
| data-encoding | 2.11.1 | MIT | stellar |
| deepsize2 | 0.1.0 | MIT | verifier |
| der | 0.7.10 | Apache-2.0 OR MIT | anchor, items, state |
| derive_more | 1.0.0 | MIT | openvm |
| derive_more | 2.1.1 | MIT | anchor, items, state |
| digest | 0.10.7 | MIT OR Apache-2.0 | anchor, items, openvm, state, stellar, verifier |
| digest | 0.11.3 | MIT OR Apache-2.0 | anchor, items |
| digest | 0.9.0 | MIT OR Apache-2.0 | anchor |
| dyn-clone | 1.0.20 | MIT OR Apache-2.0 | state |
| ecdsa | 0.16.9 | Apache-2.0 OR MIT | anchor, items, state |
| ed25519 | 2.2.3 | Apache-2.0 OR MIT | stellar |
| ed25519-dalek | 2.2.0 | BSD-3-Clause | stellar |
| either | 1.18.0 | MIT OR Apache-2.0 | items, verifier |
| either | 1.19.0 | MIT OR Apache-2.0 | anchor, openvm, state |
| elf | 0.7.4 | MIT/Apache-2.0 | verifier |
| elliptic-curve | 0.13.8 | Apache-2.0 OR MIT | anchor, items, state |
| endian-type | 0.1.2 | MIT | openvm |
| equivalent | 1.0.3 | Apache-2.0 OR MIT | openvm, state |
| escape-bytes | 0.1.1 | Apache-2.0 | stellar |
| ethnum | 1.5.3 | MIT OR Apache-2.0 | stellar |
| eyre | 0.6.14 | MIT OR Apache-2.0 | openvm |
| ff | 0.12.1 | MIT/Apache-2.0 | openvm |
| ff | 0.13.1 | MIT/Apache-2.0 | anchor, items, openvm, state, verifier |
| foldhash | 0.2.0 | Zlib | anchor, items, state |
| funty | 2.0.0 | MIT | anchor, openvm, state, verifier |
| futures | 0.3.34 | MIT OR Apache-2.0 | verifier |
| futures-channel | 0.3.34 | MIT OR Apache-2.0 | verifier |
| futures-core | 0.3.34 | MIT OR Apache-2.0 | anchor, items, openvm, state, verifier |
| futures-executor | 0.3.34 | MIT OR Apache-2.0 | verifier |
| futures-io | 0.3.34 | MIT OR Apache-2.0 | verifier |
| futures-sink | 0.3.34 | MIT OR Apache-2.0 | verifier |
| futures-task | 0.3.34 | MIT OR Apache-2.0 | anchor, items, openvm, state, verifier |
| futures-util | 0.3.34 | MIT OR Apache-2.0 | anchor, items, openvm, state, verifier |
| gcd | 2.3.0 | MIT/Apache-2.0 | verifier |
| gen_ops | 0.4.0 | MIT | verifier |
| generic-array | 0.14.7 | MIT | anchor, openvm, stellar |
| generic-array | 0.14.9 | MIT | items, state, verifier |
| getrandom | 0.2.17 | MIT OR Apache-2.0 | anchor, items, openvm, state, verifier |
| getrandom | 0.3.4 | MIT OR Apache-2.0 | openvm |
| gimli | 0.32.3 | MIT OR Apache-2.0 | verifier |
| gimli | 0.34.0 | MIT OR Apache-2.0 | openvm |
| group | 0.12.1 | MIT/Apache-2.0 | openvm |
| group | 0.13.0 | MIT/Apache-2.0 | anchor, items, openvm, state |
| halo2 | 0.1.0-beta.2 | see its license file | openvm |
| halo2_proofs | 0.1.0 | MIT OR Apache-2.0 | openvm |
| hash32 | 0.3.1 | MIT OR Apache-2.0 | stellar |
| hashbrown | 0.14.5 | MIT OR Apache-2.0 | openvm, verifier |
| hashbrown | 0.15.5 | MIT OR Apache-2.0 | anchor, state |
| hashbrown | 0.16.1 | MIT OR Apache-2.0 | state |
| hashbrown | 0.17.1 | MIT OR Apache-2.0 | anchor, items, openvm, state |
| heapless | 0.9.3 | MIT OR Apache-2.0 | stellar |
| hex | 0.4.3 | MIT OR Apache-2.0 | anchor, items, openvm, state, stellar, verifier |
| hex-literal | 1.1.0 | MIT OR Apache-2.0 | openvm |
| hmac | 0.12.1 | MIT OR Apache-2.0 | anchor, items, state |
| hybrid-array | 0.4.14 | MIT OR Apache-2.0 | items |
| hybrid-array | 0.4.15 | MIT OR Apache-2.0 | anchor |
| indenter | 0.3.4 | MIT OR Apache-2.0 | openvm |
| indexmap | 2.14.2 | Apache-2.0 OR MIT | openvm, state |
| itertools | 0.10.5 | MIT/Apache-2.0 | openvm |
| itertools | 0.12.1 | MIT OR Apache-2.0 | verifier |
| itertools | 0.13.0 | MIT OR Apache-2.0 | anchor, state |
| itertools | 0.14.0 | MIT OR Apache-2.0 | items, openvm, state, verifier |
| itoa | 1.0.18 | MIT OR Apache-2.0 | anchor, items, openvm, verifier |
| itoa | 1.0.19 | MIT OR Apache-2.0 | state, stellar |
| js-sys | 0.3.106 | MIT OR Apache-2.0 | anchor, items, openvm, state, verifier |
| jubjub | 0.9.0 | MIT/Apache-2.0 | openvm |
| k256 | 0.13.4 | Apache-2.0 OR MIT | anchor, items, state |
| keccak | 0.1.6 | Apache-2.0 OR MIT | openvm, state |
| keccak | 0.2.2 | Apache-2.0 OR MIT | anchor, items |
| lazy_static | 1.5.0 | MIT OR Apache-2.0 | verifier |
| lazy_static | 1.5.1 | MIT OR Apache-2.0 | openvm |
| libc | 0.2.189 | MIT OR Apache-2.0 | verifier |
| libc | 0.2.191 | MIT OR Apache-2.0 | openvm |
| lock_api | 0.4.14 | MIT OR Apache-2.0 | openvm |
| lockfree-object-pool | 0.1.6 | BSL-1.0 | openvm |
| log | 0.4.34 | MIT OR Apache-2.0 | openvm, verifier |
| matchers | 0.2.0 | MIT | openvm, verifier |
| memchr | 2.8.3 | Unlicense OR MIT | anchor, items, openvm, state, stellar, verifier |
| memuse | 0.2.2 | MIT/Apache-2.0 | openvm |
| metrics | 0.23.1 | MIT | openvm |
| metrics-tracing-context | 0.16.0 | MIT | openvm |
| metrics-util | 0.17.0 | MIT | openvm |
| miniz_oxide | 0.8.9 | MIT OR Zlib OR Apache-2.0 | verifier |
| miniz_oxide | 0.9.1 | MIT OR Zlib OR Apache-2.0 | openvm |
| nibble_vec | 0.1.0 | MIT | openvm |
| nu-ansi-term | 0.50.3 | MIT | openvm, verifier |
| num | 0.4.3 | MIT OR Apache-2.0 | anchor, state |
| num-bigint | 0.4.8 | MIT OR Apache-2.0 | anchor, openvm, state, verifier |
| num-complex | 0.4.6 | MIT OR Apache-2.0 | anchor, state |
| num-integer | 0.1.47 | MIT OR Apache-2.0 | anchor, openvm, state, verifier |
| num-iter | 0.1.46 | MIT OR Apache-2.0 | anchor, state |
| num-rational | 0.4.2 | MIT OR Apache-2.0 | anchor, state |
| num-traits | 0.2.19 | MIT OR Apache-2.0 | anchor, openvm, state, verifier |
| num_cpus | 1.17.0 | MIT OR Apache-2.0 | openvm, verifier |
| num_enum | 0.7.6 | BSD-3-Clause OR MIT OR Apache-2.0 | anchor, state |
| nybbles | 0.4.8 | MIT OR Apache-2.0 | anchor, items |
| nybbles | 0.4.9 | MIT OR Apache-2.0 | state |
| object | 0.37.3 | Apache-2.0 OR MIT | verifier |
| object | 0.40.0 | Apache-2.0 OR MIT | openvm |
| once_cell | 1.21.4 | MIT OR Apache-2.0 | anchor, items, openvm, state, stellar, verifier |
| op-revm | 20.0.0 | MIT OR Apache-2.0 | state |
| opaque-debug | 0.3.1 | MIT OR Apache-2.0 | anchor |
| openvm-circuit | 2.0.2 | MIT OR Apache-2.0 | openvm |
| openvm-circuit-primitives | 2.0.2 | MIT OR Apache-2.0 | openvm |
| openvm-cpu-backend | 2.0.1 | MIT OR Apache-2.0 | openvm |
| openvm-instructions | 2.0.2 | MIT OR Apache-2.0 | openvm |
| openvm-poseidon2-air | 2.0.2 | MIT OR Apache-2.0 | openvm |
| openvm-stark-backend | 2.0.1 | MIT OR Apache-2.0 | openvm |
| openvm-stark-sdk | 2.0.1 | MIT OR Apache-2.0 | openvm |
| openvm-verify-stark-host | 2.0.2 | MIT OR Apache-2.0 | openvm |
| ordered-float | 4.6.0 | MIT | openvm |
| p256 | 0.13.2 | Apache-2.0 OR MIT | anchor, state |
| p3-air | 0.4.3 | MIT OR Apache-2.0 | openvm |
| p3-air | 0.4.3-succinct | MIT OR Apache-2.0 | verifier |
| p3-baby-bear | 0.4.3 | MIT OR Apache-2.0 | openvm |
| p3-baby-bear | 0.4.3-succinct | MIT OR Apache-2.0 | verifier |
| p3-bn254-fr | 0.4.3-succinct | MIT OR Apache-2.0 | verifier |
| p3-challenger | 0.4.3 | MIT OR Apache-2.0 | openvm |
| p3-challenger | 0.4.3-succinct | MIT OR Apache-2.0 | verifier |
| p3-commit | 0.4.3-succinct | MIT OR Apache-2.0 | verifier |
| p3-dft | 0.4.3 | MIT OR Apache-2.0 | openvm |
| p3-dft | 0.4.3-succinct | MIT OR Apache-2.0 | verifier |
| p3-field | 0.4.3 | MIT OR Apache-2.0 | openvm |
| p3-field | 0.4.3-succinct | MIT OR Apache-2.0 | verifier |
| p3-fri | 0.4.3-succinct | MIT OR Apache-2.0 | verifier |
| p3-interpolation | 0.4.3 | MIT OR Apache-2.0 | openvm |
| p3-interpolation | 0.4.3-succinct | MIT OR Apache-2.0 | verifier |
| p3-koala-bear | 0.4.3-succinct | MIT OR Apache-2.0 | verifier |
| p3-matrix | 0.4.3 | MIT OR Apache-2.0 | openvm |
| p3-matrix | 0.4.3-succinct | MIT OR Apache-2.0 | verifier |
| p3-maybe-rayon | 0.4.3 | MIT OR Apache-2.0 | openvm |
| p3-maybe-rayon | 0.4.3-succinct | MIT OR Apache-2.0 | verifier |
| p3-mds | 0.4.3 | MIT OR Apache-2.0 | openvm |
| p3-mds | 0.4.3-succinct | MIT OR Apache-2.0 | verifier |
| p3-merkle-tree | 0.4.3-succinct | MIT OR Apache-2.0 | verifier |
| p3-monty-31 | 0.4.3 | MIT OR Apache-2.0 | openvm |
| p3-poseidon2 | 0.4.3 | MIT OR Apache-2.0 | openvm |
| p3-poseidon2 | 0.4.3-succinct | MIT OR Apache-2.0 | verifier |
| p3-poseidon2-air | 0.4.3 | MIT OR Apache-2.0 | openvm |
| p3-symmetric | 0.4.3 | MIT OR Apache-2.0 | openvm |
| p3-symmetric | 0.4.3-succinct | MIT OR Apache-2.0 | verifier |
| p3-uni-stark | 0.4.3-succinct | MIT OR Apache-2.0 | verifier |
| p3-util | 0.4.3 | MIT OR Apache-2.0 | openvm |
| p3-util | 0.4.3-succinct | MIT OR Apache-2.0 | verifier |
| pairing | 0.22.0 | MIT/Apache-2.0 | openvm |
| pairing | 0.23.0 | MIT/Apache-2.0 | anchor |
| parking_lot_core | 0.9.12 | MIT OR Apache-2.0 | openvm |
| pasta_curves | 0.4.1 | MIT OR Apache-2.0 | openvm |
| pasta_curves | 0.5.2 | MIT OR Apache-2.0 | openvm |
| pin-project | 1.1.13 | Apache-2.0 OR MIT | verifier |
| pin-project-lite | 0.2.17 | Apache-2.0 OR MIT | anchor, items, openvm, state, verifier |
| pkcs8 | 0.10.2 | Apache-2.0 OR MIT | items, state |
| portable-atomic | 1.15.0 | Apache-2.0 OR MIT | openvm |
| postcard | 1.1.3 | MIT OR Apache-2.0 | openvm |
| ppv-lite86 | 0.2.21 | MIT OR Apache-2.0 | anchor, openvm, state, verifier |
| primeorder | 0.13.6 | Apache-2.0 OR MIT | anchor, state |
| quanta | 0.12.6 | MIT | openvm |
| radium | 0.7.0 | MIT | anchor, openvm, state, verifier |
| radix_trie | 0.2.1 | MIT | openvm |
| rand | 0.8.8 | MIT OR Apache-2.0 | anchor, openvm, state, verifier |
| rand | 0.9.5 | MIT OR Apache-2.0 | openvm |
| rand_chacha | 0.3.1 | MIT OR Apache-2.0 | anchor, openvm, state, verifier |
| rand_chacha | 0.9.0 | MIT OR Apache-2.0 | openvm |
| rand_core | 0.6.4 | MIT OR Apache-2.0 | anchor, items, openvm, state, verifier |
| rand_core | 0.9.5 | MIT OR Apache-2.0 | openvm |
| range-set-blaze | 0.1.16 | MIT OR Apache-2.0 | verifier |
| rayon | 1.12.0 | MIT OR Apache-2.0 | openvm, verifier |
| rayon-core | 1.13.0 | MIT OR Apache-2.0 | openvm, verifier |
| rayon-scan | 0.1.1 | MIT OR Apache-2.0 | verifier |
| regex-automata | 0.4.18 | MIT OR Apache-2.0 | openvm, verifier |
| regex-syntax | 0.8.11 | MIT OR Apache-2.0 | openvm, verifier |
| revm | 29.0.1 | MIT | anchor, state |
| revm | 38.0.0 | MIT | state |
| revm-bytecode | 10.0.0 | MIT | state |
| revm-bytecode | 6.2.2 | MIT | anchor, state |
| revm-context | 16.0.1 | MIT | state |
| revm-context | 9.1.0 | MIT | anchor, state |
| revm-context-interface | 10.2.0 | MIT | anchor, state |
| revm-context-interface | 17.0.1 | MIT | state |
| revm-database | 13.0.1 | MIT | state |
| revm-database | 7.0.5 | MIT | anchor, state |
| revm-database-interface | 11.0.1 | MIT | state |
| revm-database-interface | 7.0.5 | MIT | anchor, state |
| revm-handler | 10.0.1 | MIT | anchor, state |
| revm-handler | 18.1.0 | MIT | state |
| revm-inspector | 10.0.1 | MIT | anchor, state |
| revm-inspector | 19.0.0 | MIT | state |
| revm-inspectors | 0.30.1 | MIT OR Apache-2.0 | state |
| revm-inspectors | 0.39.2 | MIT OR Apache-2.0 | state |
| revm-interpreter | 25.0.3 | MIT | anchor, state |
| revm-interpreter | 35.0.1 | MIT | state |
| revm-precompile | 27.0.0 | MIT | anchor, state |
| revm-precompile | 34.0.0 | MIT | state |
| revm-primitives | 20.2.1 | MIT | anchor, state |
| revm-primitives | 23.0.0 | MIT | state |
| revm-state | 11.0.1 | MIT | state |
| revm-state | 7.0.5 | MIT | anchor, state |
| rfc6979 | 0.4.0 | Apache-2.0 OR MIT | anchor, items, state |
| ripemd | 0.1.3 | MIT OR Apache-2.0 | anchor, state |
| ruint | 1.20.0 | MIT | items |
| ruint | 1.20.1 | MIT | anchor |
| ruint | 1.21.1 | MIT | state |
| rustc-demangle | 0.1.28 | MIT/Apache-2.0 | openvm, verifier |
| rustc-hash | 2.1.3 | Apache-2.0 OR MIT | openvm, state |
| rustc-hex | 2.1.0 | MIT/Apache-2.0 | verifier |
| scopeguard | 1.2.0 | MIT OR Apache-2.0 | openvm |
| sec1 | 0.7.3 | Apache-2.0 OR MIT | anchor, items, state |
| serde | 1.0.229 | MIT OR Apache-2.0 | items, openvm, state, stellar, verifier |
| serde-big-array | 0.5.1 | MIT OR Apache-2.0 | openvm |
| serde_core | 1.0.229 | MIT OR Apache-2.0 | anchor, items, openvm, state, stellar, verifier |
| serde_json | 1.0.151 | MIT OR Apache-2.0 | items, verifier |
| serde_json | 1.0.154 | MIT OR Apache-2.0 | anchor, openvm, state, stellar |
| serde_with | 3.22.0 | MIT OR Apache-2.0 | items |
| serde_with | 3.24.0 | MIT OR Apache-2.0 | state |
| serdect | 0.2.0 | Apache-2.0 OR MIT | items, state |
| sha2 | 0.10.9 | MIT OR Apache-2.0 | anchor, items, openvm, state, stellar, verifier |
| sha2 | 0.11.1 | MIT OR Apache-2.0 | anchor |
| sha2 | 0.9.9 | MIT OR Apache-2.0 | anchor |
| sha3 | 0.10.9 | MIT OR Apache-2.0 | openvm, state |
| sha3 | 0.11.0 | MIT OR Apache-2.0 | anchor, items |
| sharded-slab | 0.1.7 | MIT | openvm, verifier |
| signature | 2.2.0 | Apache-2.0 OR MIT | anchor, items, state, stellar |
| sketches-ddsketch | 0.2.2 | Apache-2.0 | openvm |
| slab | 0.4.12 | MIT | anchor, items, openvm, state, verifier |
| slop-air | 6.8.1 | MIT OR Apache-2.0 | verifier |
| slop-algebra | 6.8.1 | MIT OR Apache-2.0 | verifier |
| slop-alloc | 6.8.1 | MIT OR Apache-2.0 | verifier |
| slop-baby-bear | 6.8.1 | MIT OR Apache-2.0 | verifier |
| slop-basefold | 6.8.1 | MIT OR Apache-2.0 | verifier |
| slop-basefold-prover | 6.8.1 | MIT OR Apache-2.0 | verifier |
| slop-bn254 | 6.8.1 | MIT OR Apache-2.0 | verifier |
| slop-challenger | 6.8.1 | MIT OR Apache-2.0 | verifier |
| slop-commit | 6.8.1 | MIT OR Apache-2.0 | verifier |
| slop-dft | 6.8.1 | MIT OR Apache-2.0 | verifier |
| slop-fri | 6.8.1 | MIT OR Apache-2.0 | verifier |
| slop-futures | 6.8.1 | MIT OR Apache-2.0 | verifier |
| slop-jagged | 6.8.1 | MIT OR Apache-2.0 | verifier |
| slop-koala-bear | 6.8.1 | MIT OR Apache-2.0 | verifier |
| slop-matrix | 6.8.1 | MIT OR Apache-2.0 | verifier |
| slop-maybe-rayon | 6.8.1 | MIT OR Apache-2.0 | verifier |
| slop-merkle-tree | 6.8.1 | MIT OR Apache-2.0 | verifier |
| slop-multilinear | 6.8.1 | MIT OR Apache-2.0 | verifier |
| slop-poseidon2 | 6.8.1 | MIT OR Apache-2.0 | verifier |
| slop-primitives | 6.8.1 | MIT OR Apache-2.0 | verifier |
| slop-stacked | 6.8.1 | MIT OR Apache-2.0 | verifier |
| slop-sumcheck | 6.8.1 | MIT OR Apache-2.0 | verifier |
| slop-symmetric | 6.8.1 | MIT OR Apache-2.0 | verifier |
| slop-tensor | 6.8.1 | MIT OR Apache-2.0 | verifier |
| slop-uni-stark | 6.8.1 | MIT OR Apache-2.0 | verifier |
| slop-utils | 6.8.1 | MIT OR Apache-2.0 | verifier |
| slop-whir | 6.8.1 | MIT OR Apache-2.0 | verifier |
| smallvec | 1.15.2 | MIT OR Apache-2.0 | items, verifier |
| smallvec | 1.16.3 | MIT OR Apache-2.0 | anchor |
| smallvec | 1.16.4 | MIT OR Apache-2.0 | openvm |
| smallvec | 1.16.5 | MIT OR Apache-2.0 | state |
| sp1-hypercube | 6.8.1 | MIT OR Apache-2.0 | verifier |
| sp1-primitives | 6.8.1 | MIT OR Apache-2.0 | verifier |
| sp1-recursion-executor | 6.8.1 | MIT OR Apache-2.0 | verifier |
| sp1-recursion-machine | 6.8.1 | MIT OR Apache-2.0 | verifier |
| sp1-verifier | 6.8.1 | MIT OR Apache-2.0 | verifier |
| spin | 0.10.1 | MIT | openvm |
| spin | 0.9.9 | MIT | openvm, verifier |
| spki | 0.7.3 | Apache-2.0 OR MIT | anchor, items, state |
| stable_deref_trait | 1.2.1 | MIT OR Apache-2.0 | stellar |
| static_assertions | 1.1.0 | MIT OR Apache-2.0 | openvm, verifier |
| stellar-strkey | 1.0.0 | Apache-2.0 | stellar |
| stellar-xdr | 30.0.0 | Apache-2.0 | stellar |
| strength_reduce | 0.2.4 | MIT OR Apache-2.0 | openvm, verifier |
| struct-reflection | 0.1.0 | MIT | openvm, verifier |
| strum | 0.26.3 | MIT | openvm |
| strum | 0.27.2 | MIT | state, verifier |
| substrate-bn-succinct-rs | 0.6.0 | MIT OR Apache-2.0 | verifier |
| subtle | 2.6.1 | BSD-3-Clause | anchor, items, openvm, state, stellar, verifier |
| tap | 1.0.1 | MIT | anchor, openvm, state, verifier |
| thiserror | 1.0.69 | MIT OR Apache-2.0 | openvm, verifier |
| thiserror | 2.0.20 | MIT OR Apache-2.0 | items, verifier |
| thiserror | 2.0.21 | MIT OR Apache-2.0 | anchor, openvm, state |
| thousands | 0.2.0 | MIT/Apache-2.0 | verifier |
| thread_local | 1.1.10 | MIT OR Apache-2.0 | openvm, verifier |
| tiny-keccak | 2.0.2 | CC0-1.0 | items, openvm, state, stellar, verifier |
| tokio | 1.53.1 | MIT | verifier |
| tracing | 0.1.44 | MIT | anchor, items, openvm, state, verifier |
| tracing-core | 0.1.36 | MIT | anchor, items, openvm, state, verifier |
| tracing-forest | 0.1.6 | MIT | openvm, verifier |
| tracing-log | 0.2.0 | MIT | openvm, verifier |
| tracing-subscriber | 0.3.23 | MIT | openvm, verifier |
| transpose | 0.2.3 | MIT OR Apache-2.0 | openvm, verifier |
| typenum | 1.20.1 | MIT OR Apache-2.0 | anchor, items, openvm, state, stellar, verifier |
| unicode-ident | 1.0.24 | (MIT OR Apache-2.0) AND Unicode-3.0 | items, verifier |
| unicode-ident | 1.0.27 | (MIT OR Apache-2.0) AND Unicode-3.0 | anchor, openvm, state, stellar |
| wasm-bindgen | 0.2.129 | MIT OR Apache-2.0 | anchor, items, openvm, state, stellar, verifier |
| wasm-bindgen-shared | 0.2.129 | MIT OR Apache-2.0 | anchor, items, openvm, state, stellar, verifier |
| web-sys | 0.3.106 | MIT OR Apache-2.0 | openvm |
| wyz | 0.5.1 | MIT | anchor, openvm, state, verifier |
| zerocopy | 0.8.56 | BSD-2-Clause OR Apache-2.0 OR MIT | verifier |
| zerocopy | 0.8.63 | BSD-2-Clause OR Apache-2.0 OR MIT | anchor, openvm, state |
| zeroize | 1.9.0 | Apache-2.0 OR MIT | items |
| zeroize | 1.9.1 | Apache-2.0 OR MIT | anchor, openvm, state |
| zeroize | 1.9.2 | Apache-2.0 OR MIT | stellar |
| zkhash-axiom | 0.2.0 | MIT OR Apache-2.0 | openvm |
| zmij | 1.0.23 | MIT | anchor, items, openvm, state, stellar, verifier |
| zstd | 0.13.3 | MIT | openvm |
| zstd-safe | 7.3.0 | BSD-3-Clause | openvm |
| zstd-sys | 2.1.2+zstd.1.5.7 | BSD-3-Clause | openvm |

## The Rust standard library

`core`, `alloc`, `std` and the toolchain's other runtime crates (nightly-2026-04-12), compiled into every module. License: MIT OR Apache-2.0. Copyright: the Rust Project Developers, and the authors of the crates the library includes (among them dlmalloc and cfg-if, Alex Crichton; hashbrown, Amanieu d'Antras). `core`'s Unicode tables come from the Unicode Character Database: Unicode-3.0.

MIT:

    MIT License

    Copyright (c) The Rust Project Developers

    Permission is hereby granted, free of charge, to any person obtaining a copy of this software and associated documentation files (the "Software"), to deal in the Software without restriction, including without limitation the rights to use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of the Software, and to permit persons to whom the Software is furnished to do so, subject to the following conditions:

    The above copyright notice and this permission notice shall be included in all copies or substantial portions of the Software.

    THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

Apache-2.0:

    Apache License
    Version 2.0, January 2004
    http://www.apache.org/licenses/

    TERMS AND CONDITIONS FOR USE, REPRODUCTION, AND DISTRIBUTION

    1. Definitions.

    "License" shall mean the terms and conditions for use, reproduction, and distribution as defined by Sections 1 through 9 of this document.

    "Licensor" shall mean the copyright owner or entity authorized by the copyright owner that is granting the License.

    "Legal Entity" shall mean the union of the acting entity and all other entities that control, are controlled by, or are under common control with that entity. For the purposes of this definition, "control" means (i) the power, direct or indirect, to cause the direction or management of such entity, whether by contract or otherwise, or (ii) ownership of fifty percent (50%) or more of the outstanding shares, or (iii) beneficial ownership of such entity.

    "You" (or "Your") shall mean an individual or Legal Entity exercising permissions granted by this License.

    "Source" form shall mean the preferred form for making modifications, including but not limited to software source code, documentation source, and configuration files.

    "Object" form shall mean any form resulting from mechanical transformation or translation of a Source form, including but not limited to compiled object code, generated documentation, and conversions to other media types.

    "Work" shall mean the work of authorship, whether in Source or Object form, made available under the License, as indicated by a copyright notice that is included in or attached to the work (an example is provided in the Appendix below).

    "Derivative Works" shall mean any work, whether in Source or Object form, that is based on (or derived from) the Work and for which the editorial revisions, annotations, elaborations, or other modifications represent, as a whole, an original work of authorship. For the purposes of this License, Derivative Works shall not include works that remain separable from, or merely link (or bind by name) to the interfaces of, the Work and Derivative Works thereof.

    "Contribution" shall mean any work of authorship, including the original version of the Work and any modifications or additions to that Work or Derivative Works thereof, that is intentionally submitted to Licensor for inclusion in the Work by the copyright owner or by an individual or Legal Entity authorized to submit on behalf of the copyright owner. For the purposes of this definition, "submitted" means any form of electronic, verbal, or written communication sent to the Licensor or its representatives, including but not limited to communication on electronic mailing lists, source code control systems, and issue tracking systems that are managed by, or on behalf of, the Licensor for the purpose of discussing and improving the Work, but excluding communication that is conspicuously marked or otherwise designated in writing by the copyright owner as "Not a Contribution."

    "Contributor" shall mean Licensor and any individual or Legal Entity on behalf of whom a Contribution has been received by Licensor and subsequently incorporated within the Work.

    2. Grant of Copyright License. Subject to the terms and conditions of this License, each Contributor hereby grants to You a perpetual, worldwide, non-exclusive, no-charge, royalty-free, irrevocable copyright license to reproduce, prepare Derivative Works of, publicly display, publicly perform, sublicense, and distribute the Work and such Derivative Works in Source or Object form.

    3. Grant of Patent License. Subject to the terms and conditions of this License, each Contributor hereby grants to You a perpetual, worldwide, non-exclusive, no-charge, royalty-free, irrevocable (except as stated in this section) patent license to make, have made, use, offer to sell, sell, import, and otherwise transfer the Work, where such license applies only to those patent claims licensable by such Contributor that are necessarily infringed by their Contribution(s) alone or by combination of their Contribution(s) with the Work to which such Contribution(s) was submitted. If You institute patent litigation against any entity (including a cross-claim or counterclaim in a lawsuit) alleging that the Work or a Contribution incorporated within the Work constitutes direct or contributory patent infringement, then any patent licenses granted to You under this License for that Work shall terminate as of the date such litigation is filed.

    4. Redistribution. You may reproduce and distribute copies of the Work or Derivative Works thereof in any medium, with or without modifications, and in Source or Object form, provided that You meet the following conditions:

         (a) You must give any other recipients of the Work or Derivative Works a copy of this License; and

         (b) You must cause any modified files to carry prominent notices stating that You changed the files; and

         (c) You must retain, in the Source form of any Derivative Works that You distribute, all copyright, patent, trademark, and attribution notices from the Source form of the Work, excluding those notices that do not pertain to any part of the Derivative Works; and

         (d) If the Work includes a "NOTICE" text file as part of its distribution, then any Derivative Works that You distribute must include a readable copy of the attribution notices contained within such NOTICE file, excluding those notices that do not pertain to any part of the Derivative Works, in at least one of the following places: within a NOTICE text file distributed as part of the Derivative Works; within the Source form or documentation, if provided along with the Derivative Works; or, within a display generated by the Derivative Works, if and wherever such third-party notices normally appear. The contents of the NOTICE file are for informational purposes only and do not modify the License. You may add Your own attribution notices within Derivative Works that You distribute, alongside or as an addendum to the NOTICE text from the Work, provided that such additional attribution notices cannot be construed as modifying the License.

         You may add Your own copyright statement to Your modifications and may provide additional or different license terms and conditions for use, reproduction, or distribution of Your modifications, or for any such Derivative Works as a whole, provided Your use, reproduction, and distribution of the Work otherwise complies with the conditions stated in this License.

    5. Submission of Contributions. Unless You explicitly state otherwise, any Contribution intentionally submitted for inclusion in the Work by You to the Licensor shall be under the terms and conditions of this License, without any additional terms or conditions. Notwithstanding the above, nothing herein shall supersede or modify the terms of any separate license agreement you may have executed with Licensor regarding such Contributions.

    6. Trademarks. This License does not grant permission to use the trade names, trademarks, service marks, or product names of the Licensor, except as required for reasonable and customary use in describing the origin of the Work and reproducing the content of the NOTICE file.

    7. Disclaimer of Warranty. Unless required by applicable law or agreed to in writing, Licensor provides the Work (and each Contributor provides its Contributions) on an "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied, including, without limitation, any warranties or conditions of TITLE, NON-INFRINGEMENT, MERCHANTABILITY, or FITNESS FOR A PARTICULAR PURPOSE. You are solely responsible for determining the appropriateness of using or redistributing the Work and assume any risks associated with Your exercise of permissions under this License.

    8. Limitation of Liability. In no event and under no legal theory, whether in tort (including negligence), contract, or otherwise, unless required by applicable law (such as deliberate and grossly negligent acts) or agreed to in writing, shall any Contributor be liable to You for damages, including any direct, indirect, special, incidental, or consequential damages of any character arising as a result of this License or out of the use or inability to use the Work (including but not limited to damages for loss of goodwill, work stoppage, computer failure or malfunction, or any and all other commercial damages or losses), even if such Contributor has been advised of the possibility of such damages.

    9. Accepting Warranty or Additional Liability. While redistributing the Work or Derivative Works thereof, You may choose to offer, and charge a fee for, acceptance of support, warranty, indemnity, or other liability obligations and/or rights consistent with this License. However, in accepting such obligations, You may act only on Your own behalf and on Your sole responsibility, not on behalf of any other Contributor, and only if You agree to indemnify, defend, and hold each Contributor harmless for any liability incurred by, or claims asserted against, such Contributor by reason of your accepting any such warranty or additional liability.

    END OF TERMS AND CONDITIONS

    APPENDIX: How to apply the Apache License to your work.

    To apply the Apache License to your work, attach the following boilerplate notice, with the fields enclosed by brackets "[]" replaced with your own identifying information. (Don't include the brackets!)  The text should be enclosed in the appropriate comment syntax for the file format. We also recommend that a file or class name and description of purpose be included on the same "printed page" as the copyright notice for easier identification within third-party archives.

    Copyright [yyyy] [name of copyright owner]

    Licensed under the Apache License, Version 2.0 (the "License");
    you may not use this file except in compliance with the License.
    You may obtain a copy of the License at

    http://www.apache.org/licenses/LICENSE-2.0

    Unless required by applicable law or agreed to in writing, software
    distributed under the License is distributed on an "AS IS" BASIS,
    WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
    See the License for the specific language governing permissions and
    limitations under the License.

Unicode-3.0:

    UNICODE LICENSE V3

    COPYRIGHT AND PERMISSION NOTICE

    Copyright © 1991-2023 Unicode, Inc.

    NOTICE TO USER: Carefully read the following legal agreement. BY
    DOWNLOADING, INSTALLING, COPYING OR OTHERWISE USING DATA FILES, AND/OR
    SOFTWARE, YOU UNEQUIVOCALLY ACCEPT, AND AGREE TO BE BOUND BY, ALL OF THE
    TERMS AND CONDITIONS OF THIS AGREEMENT. IF YOU DO NOT AGREE, DO NOT
    DOWNLOAD, INSTALL, COPY, DISTRIBUTE OR USE THE DATA FILES OR SOFTWARE.

    Permission is hereby granted, free of charge, to any person obtaining a
    copy of data files and any associated documentation (the "Data Files") or
    software and any associated documentation (the "Software") to deal in the
    Data Files or Software without restriction, including without limitation
    the rights to use, copy, modify, merge, publish, distribute, and/or sell
    copies of the Data Files or Software, and to permit persons to whom the
    Data Files or Software are furnished to do so, provided that either (a)
    this copyright and permission notice appear with all copies of the Data
    Files or Software, or (b) this copyright and permission notice appear in
    associated Documentation.

    THE DATA FILES AND SOFTWARE ARE PROVIDED "AS IS", WITHOUT WARRANTY OF ANY
    KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF
    MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT OF
    THIRD PARTY RIGHTS.

    IN NO EVENT SHALL THE COPYRIGHT HOLDER OR HOLDERS INCLUDED IN THIS NOTICE
    BE LIABLE FOR ANY CLAIM, OR ANY SPECIAL INDIRECT OR CONSEQUENTIAL DAMAGES,
    OR ANY DAMAGES WHATSOEVER RESULTING FROM LOSS OF USE, DATA OR PROFITS,
    WHETHER IN AN ACTION OF CONTRACT, NEGLIGENCE OR OTHER TORTIOUS ACTION,
    ARISING OUT OF OR IN CONNECTION WITH THE USE OR PERFORMANCE OF THE DATA
    FILES OR SOFTWARE.

    Except as contained in this notice, the name of a copyright holder shall
    not be used in advertising or otherwise to promote the sale, use or other
    dealings in these Data Files or Software without prior written
    authorization of the copyright holder.

## addr2line 0.25.1

License: Apache-2.0 OR MIT. Source: https://github.com/gimli-rs/addr2line

`LICENSE-APACHE` (addr2line 0.25.1):

                                  Apache License
                            Version 2.0, January 2004
                         http://www.apache.org/licenses/

    TERMS AND CONDITIONS FOR USE, REPRODUCTION, AND DISTRIBUTION

    1. Definitions.

       "License" shall mean the terms and conditions for use, reproduction,
       and distribution as defined by Sections 1 through 9 of this document.

       "Licensor" shall mean the copyright owner or entity authorized by
       the copyright owner that is granting the License.

       "Legal Entity" shall mean the union of the acting entity and all
       other entities that control, are controlled by, or are under common
       control with that entity. For the purposes of this definition,
       "control" means (i) the power, direct or indirect, to cause the
       direction or management of such entity, whether by contract or
       otherwise, or (ii) ownership of fifty percent (50%) or more of the
       outstanding shares, or (iii) beneficial ownership of such entity.

       "You" (or "Your") shall mean an individual or Legal Entity
       exercising permissions granted by this License.

       "Source" form shall mean the preferred form for making modifications,
       including but not limited to software source code, documentation
       source, and configuration files.

       "Object" form shall mean any form resulting from mechanical
       transformation or translation of a Source form, including but
       not limited to compiled object code, generated documentation,
       and conversions to other media types.

       "Work" shall mean the work of authorship, whether in Source or
       Object form, made available under the License, as indicated by a
       copyright notice that is included in or attached to the work
       (an example is provided in the Appendix below).

       "Derivative Works" shall mean any work, whether in Source or Object
       form, that is based on (or derived from) the Work and for which the
       editorial revisions, annotations, elaborations, or other modifications
       represent, as a whole, an original work of authorship. For the purposes
       of this License, Derivative Works shall not include works that remain
       separable from, or merely link (or bind by name) to the interfaces of,
       the Work and Derivative Works thereof.

       "Contribution" shall mean any work of authorship, including
       the original version of the Work and any modifications or additions
       to that Work or Derivative Works thereof, that is intentionally
       submitted to Licensor for inclusion in the Work by the copyright owner
       or by an individual or Legal Entity authorized to submit on behalf of
       the copyright owner. For the purposes of this definition, "submitted"
       means any form of electronic, verbal, or written communication sent
       to the Licensor or its representatives, including but not limited to
       communication on electronic mailing lists, source code control systems,
       and issue tracking systems that are managed by, or on behalf of, the
       Licensor for the purpose of discussing and improving the Work, but
       excluding communication that is conspicuously marked or otherwise
       designated in writing by the copyright owner as "Not a Contribution."

       "Contributor" shall mean Licensor and any individual or Legal Entity
       on behalf of whom a Contribution has been received by Licensor and
       subsequently incorporated within the Work.

    2. Grant of Copyright License. Subject to the terms and conditions of
       this License, each Contributor hereby grants to You a perpetual,
       worldwide, non-exclusive, no-charge, royalty-free, irrevocable
       copyright license to reproduce, prepare Derivative Works of,
       publicly display, publicly perform, sublicense, and distribute the
       Work and such Derivative Works in Source or Object form.

    3. Grant of Patent License. Subject to the terms and conditions of
       this License, each Contributor hereby grants to You a perpetual,
       worldwide, non-exclusive, no-charge, royalty-free, irrevocable
       (except as stated in this section) patent license to make, have made,
       use, offer to sell, sell, import, and otherwise transfer the Work,
       where such license applies only to those patent claims licensable
       by such Contributor that are necessarily infringed by their
       Contribution(s) alone or by combination of their Contribution(s)
       with the Work to which such Contribution(s) was submitted. If You
       institute patent litigation against any entity (including a
       cross-claim or counterclaim in a lawsuit) alleging that the Work
       or a Contribution incorporated within the Work constitutes direct
       or contributory patent infringement, then any patent licenses
       granted to You under this License for that Work shall terminate
       as of the date such litigation is filed.

    4. Redistribution. You may reproduce and distribute copies of the
       Work or Derivative Works thereof in any medium, with or without
       modifications, and in Source or Object form, provided that You
       meet the following conditions:

       (a) You must give any other recipients of the Work or
           Derivative Works a copy of this License; and

       (b) You must cause any modified files to carry prominent notices
           stating that You changed the files; and

       (c) You must retain, in the Source form of any Derivative Works
           that You distribute, all copyright, patent, trademark, and
           attribution notices from the Source form of the Work,
           excluding those notices that do not pertain to any part of
           the Derivative Works; and

       (d) If the Work includes a "NOTICE" text file as part of its
           distribution, then any Derivative Works that You distribute must
           include a readable copy of the attribution notices contained
           within such NOTICE file, excluding those notices that do not
           pertain to any part of the Derivative Works, in at least one
           of the following places: within a NOTICE text file distributed
           as part of the Derivative Works; within the Source form or
           documentation, if provided along with the Derivative Works; or,
           within a display generated by the Derivative Works, if and
           wherever such third-party notices normally appear. The contents
           of the NOTICE file are for informational purposes only and
           do not modify the License. You may add Your own attribution
           notices within Derivative Works that You distribute, alongside
           or as an addendum to the NOTICE text from the Work, provided
           that such additional attribution notices cannot be construed
           as modifying the License.

       You may add Your own copyright statement to Your modifications and
       may provide additional or different license terms and conditions
       for use, reproduction, or distribution of Your modifications, or
       for any such Derivative Works as a whole, provided Your use,
       reproduction, and distribution of the Work otherwise complies with
       the conditions stated in this License.

    5. Submission of Contributions. Unless You explicitly state otherwise,
       any Contribution intentionally submitted for inclusion in the Work
       by You to the Licensor shall be under the terms and conditions of
       this License, without any additional terms or conditions.
       Notwithstanding the above, nothing herein shall supersede or modify
       the terms of any separate license agreement you may have executed
       with Licensor regarding such Contributions.

    6. Trademarks. This License does not grant permission to use the trade
       names, trademarks, service marks, or product names of the Licensor,
       except as required for reasonable and customary use in describing the
       origin of the Work and reproducing the content of the NOTICE file.

    7. Disclaimer of Warranty. Unless required by applicable law or
       agreed to in writing, Licensor provides the Work (and each
       Contributor provides its Contributions) on an "AS IS" BASIS,
       WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or
       implied, including, without limitation, any warranties or conditions
       of TITLE, NON-INFRINGEMENT, MERCHANTABILITY, or FITNESS FOR A
       PARTICULAR PURPOSE. You are solely responsible for determining the
       appropriateness of using or redistributing the Work and assume any
       risks associated with Your exercise of permissions under this License.

    8. Limitation of Liability. In no event and under no legal theory,
       whether in tort (including negligence), contract, or otherwise,
       unless required by applicable law (such as deliberate and grossly
       negligent acts) or agreed to in writing, shall any Contributor be
       liable to You for damages, including any direct, indirect, special,
       incidental, or consequential damages of any character arising as a
       result of this License or out of the use or inability to use the
       Work (including but not limited to damages for loss of goodwill,
       work stoppage, computer failure or malfunction, or any and all
       other commercial damages or losses), even if such Contributor
       has been advised of the possibility of such damages.

    9. Accepting Warranty or Additional Liability. While redistributing
       the Work or Derivative Works thereof, You may choose to offer,
       and charge a fee for, acceptance of support, warranty, indemnity,
       or other liability obligations and/or rights consistent with this
       License. However, in accepting such obligations, You may act only
       on Your own behalf and on Your sole responsibility, not on behalf
       of any other Contributor, and only if You agree to indemnify,
       defend, and hold each Contributor harmless for any liability
       incurred by, or claims asserted against, such Contributor by reason
       of your accepting any such warranty or additional liability.

    END OF TERMS AND CONDITIONS

    APPENDIX: How to apply the Apache License to your work.

       To apply the Apache License to your work, attach the following
       boilerplate notice, with the fields enclosed by brackets "[]"
       replaced with your own identifying information. (Don't include
       the brackets!)  The text should be enclosed in the appropriate
       comment syntax for the file format. We also recommend that a
       file or class name and description of purpose be included on the
       same "printed page" as the copyright notice for easier
       identification within third-party archives.

    Copyright [yyyy] [name of copyright owner]

    Licensed under the Apache License, Version 2.0 (the "License");
    you may not use this file except in compliance with the License.
    You may obtain a copy of the License at

    	http://www.apache.org/licenses/LICENSE-2.0

    Unless required by applicable law or agreed to in writing, software
    distributed under the License is distributed on an "AS IS" BASIS,
    WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
    See the License for the specific language governing permissions and
    limitations under the License.

`LICENSE-MIT` (addr2line 0.25.1):

    Copyright (c) 2016-2018 The gimli Developers

    Permission is hereby granted, free of charge, to any
    person obtaining a copy of this software and associated
    documentation files (the "Software"), to deal in the
    Software without restriction, including without
    limitation the rights to use, copy, modify, merge,
    publish, distribute, sublicense, and/or sell copies of
    the Software, and to permit persons to whom the Software
    is furnished to do so, subject to the following
    conditions:

    The above copyright notice and this permission notice
    shall be included in all copies or substantial portions
    of the Software.

    THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF
    ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED
    TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A
    PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT
    SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
    CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
    OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR
    IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
    DEALINGS IN THE SOFTWARE.

## addr2line 0.27.1

License: Apache-2.0 OR MIT. Source: https://github.com/gimli-rs/addr2line

`LICENSE-APACHE` (addr2line 0.27.1): the same text as `LICENSE-APACHE` (addr2line 0.25.1) above.

`LICENSE-MIT` (addr2line 0.27.1): the same text as `LICENSE-MIT` (addr2line 0.25.1) above.

## adler2 2.0.1

License: 0BSD OR MIT OR Apache-2.0. Source: https://github.com/oyvindln/adler2
Authors: Jonas Schievink <jonasschievink@gmail.com>, oyvindln <oyvindln@users.noreply.github.com>.

`LICENSE-0BSD` (adler2 2.0.1):

    Copyright (C) Jonas Schievink <jonasschievink@gmail.com>

    Permission to use, copy, modify, and/or distribute this software for
    any purpose with or without fee is hereby granted.

    THE SOFTWARE IS PROVIDED "AS IS" AND THE AUTHOR DISCLAIMS ALL WARRANTIES
    WITH REGARD TO THIS SOFTWARE INCLUDING ALL IMPLIED WARRANTIES OF
    MERCHANTABILITY AND FITNESS. IN NO EVENT SHALL THE AUTHOR BE LIABLE FOR
    ANY SPECIAL, DIRECT, INDIRECT, OR CONSEQUENTIAL DAMAGES OR ANY DAMAGES
    WHATSOEVER RESULTING FROM LOSS OF USE, DATA OR PROFITS, WHETHER IN
    AN ACTION OF CONTRACT, NEGLIGENCE OR OTHER TORTIOUS ACTION, ARISING OUT
    OF OR IN CONNECTION WITH THE USE OR PERFORMANCE OF THIS SOFTWARE.

`LICENSE-APACHE` (adler2 2.0.1):

                                  Apache License
                            Version 2.0, January 2004
                         https://www.apache.org/licenses/LICENSE-2.0

    TERMS AND CONDITIONS FOR USE, REPRODUCTION, AND DISTRIBUTION

    1. Definitions.

       "License" shall mean the terms and conditions for use, reproduction,
       and distribution as defined by Sections 1 through 9 of this document.

       "Licensor" shall mean the copyright owner or entity authorized by
       the copyright owner that is granting the License.

       "Legal Entity" shall mean the union of the acting entity and all
       other entities that control, are controlled by, or are under common
       control with that entity. For the purposes of this definition,
       "control" means (i) the power, direct or indirect, to cause the
       direction or management of such entity, whether by contract or
       otherwise, or (ii) ownership of fifty percent (50%) or more of the
       outstanding shares, or (iii) beneficial ownership of such entity.

       "You" (or "Your") shall mean an individual or Legal Entity
       exercising permissions granted by this License.

       "Source" form shall mean the preferred form for making modifications,
       including but not limited to software source code, documentation
       source, and configuration files.

       "Object" form shall mean any form resulting from mechanical
       transformation or translation of a Source form, including but
       not limited to compiled object code, generated documentation,
       and conversions to other media types.

       "Work" shall mean the work of authorship, whether in Source or
       Object form, made available under the License, as indicated by a
       copyright notice that is included in or attached to the work
       (an example is provided in the Appendix below).

       "Derivative Works" shall mean any work, whether in Source or Object
       form, that is based on (or derived from) the Work and for which the
       editorial revisions, annotations, elaborations, or other modifications
       represent, as a whole, an original work of authorship. For the purposes
       of this License, Derivative Works shall not include works that remain
       separable from, or merely link (or bind by name) to the interfaces of,
       the Work and Derivative Works thereof.

       "Contribution" shall mean any work of authorship, including
       the original version of the Work and any modifications or additions
       to that Work or Derivative Works thereof, that is intentionally
       submitted to Licensor for inclusion in the Work by the copyright owner
       or by an individual or Legal Entity authorized to submit on behalf of
       the copyright owner. For the purposes of this definition, "submitted"
       means any form of electronic, verbal, or written communication sent
       to the Licensor or its representatives, including but not limited to
       communication on electronic mailing lists, source code control systems,
       and issue tracking systems that are managed by, or on behalf of, the
       Licensor for the purpose of discussing and improving the Work, but
       excluding communication that is conspicuously marked or otherwise
       designated in writing by the copyright owner as "Not a Contribution."

       "Contributor" shall mean Licensor and any individual or Legal Entity
       on behalf of whom a Contribution has been received by Licensor and
       subsequently incorporated within the Work.

    2. Grant of Copyright License. Subject to the terms and conditions of
       this License, each Contributor hereby grants to You a perpetual,
       worldwide, non-exclusive, no-charge, royalty-free, irrevocable
       copyright license to reproduce, prepare Derivative Works of,
       publicly display, publicly perform, sublicense, and distribute the
       Work and such Derivative Works in Source or Object form.

    3. Grant of Patent License. Subject to the terms and conditions of
       this License, each Contributor hereby grants to You a perpetual,
       worldwide, non-exclusive, no-charge, royalty-free, irrevocable
       (except as stated in this section) patent license to make, have made,
       use, offer to sell, sell, import, and otherwise transfer the Work,
       where such license applies only to those patent claims licensable
       by such Contributor that are necessarily infringed by their
       Contribution(s) alone or by combination of their Contribution(s)
       with the Work to which such Contribution(s) was submitted. If You
       institute patent litigation against any entity (including a
       cross-claim or counterclaim in a lawsuit) alleging that the Work
       or a Contribution incorporated within the Work constitutes direct
       or contributory patent infringement, then any patent licenses
       granted to You under this License for that Work shall terminate
       as of the date such litigation is filed.

    4. Redistribution. You may reproduce and distribute copies of the
       Work or Derivative Works thereof in any medium, with or without
       modifications, and in Source or Object form, provided that You
       meet the following conditions:

       (a) You must give any other recipients of the Work or
           Derivative Works a copy of this License; and

       (b) You must cause any modified files to carry prominent notices
           stating that You changed the files; and

       (c) You must retain, in the Source form of any Derivative Works
           that You distribute, all copyright, patent, trademark, and
           attribution notices from the Source form of the Work,
           excluding those notices that do not pertain to any part of
           the Derivative Works; and

       (d) If the Work includes a "NOTICE" text file as part of its
           distribution, then any Derivative Works that You distribute must
           include a readable copy of the attribution notices contained
           within such NOTICE file, excluding those notices that do not
           pertain to any part of the Derivative Works, in at least one
           of the following places: within a NOTICE text file distributed
           as part of the Derivative Works; within the Source form or
           documentation, if provided along with the Derivative Works; or,
           within a display generated by the Derivative Works, if and
           wherever such third-party notices normally appear. The contents
           of the NOTICE file are for informational purposes only and
           do not modify the License. You may add Your own attribution
           notices within Derivative Works that You distribute, alongside
           or as an addendum to the NOTICE text from the Work, provided
           that such additional attribution notices cannot be construed
           as modifying the License.

       You may add Your own copyright statement to Your modifications and
       may provide additional or different license terms and conditions
       for use, reproduction, or distribution of Your modifications, or
       for any such Derivative Works as a whole, provided Your use,
       reproduction, and distribution of the Work otherwise complies with
       the conditions stated in this License.

    5. Submission of Contributions. Unless You explicitly state otherwise,
       any Contribution intentionally submitted for inclusion in the Work
       by You to the Licensor shall be under the terms and conditions of
       this License, without any additional terms or conditions.
       Notwithstanding the above, nothing herein shall supersede or modify
       the terms of any separate license agreement you may have executed
       with Licensor regarding such Contributions.

    6. Trademarks. This License does not grant permission to use the trade
       names, trademarks, service marks, or product names of the Licensor,
       except as required for reasonable and customary use in describing the
       origin of the Work and reproducing the content of the NOTICE file.

    7. Disclaimer of Warranty. Unless required by applicable law or
       agreed to in writing, Licensor provides the Work (and each
       Contributor provides its Contributions) on an "AS IS" BASIS,
       WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or
       implied, including, without limitation, any warranties or conditions
       of TITLE, NON-INFRINGEMENT, MERCHANTABILITY, or FITNESS FOR A
       PARTICULAR PURPOSE. You are solely responsible for determining the
       appropriateness of using or redistributing the Work and assume any
       risks associated with Your exercise of permissions under this License.

    8. Limitation of Liability. In no event and under no legal theory,
       whether in tort (including negligence), contract, or otherwise,
       unless required by applicable law (such as deliberate and grossly
       negligent acts) or agreed to in writing, shall any Contributor be
       liable to You for damages, including any direct, indirect, special,
       incidental, or consequential damages of any character arising as a
       result of this License or out of the use or inability to use the
       Work (including but not limited to damages for loss of goodwill,
       work stoppage, computer failure or malfunction, or any and all
       other commercial damages or losses), even if such Contributor
       has been advised of the possibility of such damages.

    9. Accepting Warranty or Additional Liability. While redistributing
       the Work or Derivative Works thereof, You may choose to offer,
       and charge a fee for, acceptance of support, warranty, indemnity,
       or other liability obligations and/or rights consistent with this
       License. However, in accepting such obligations, You may act only
       on Your own behalf and on Your sole responsibility, not on behalf
       of any other Contributor, and only if You agree to indemnify,
       defend, and hold each Contributor harmless for any liability
       incurred by, or claims asserted against, such Contributor by reason
       of your accepting any such warranty or additional liability.

    END OF TERMS AND CONDITIONS

    APPENDIX: How to apply the Apache License to your work.

       To apply the Apache License to your work, attach the following
       boilerplate notice, with the fields enclosed by brackets "[]"
       replaced with your own identifying information. (Don't include
       the brackets!)  The text should be enclosed in the appropriate
       comment syntax for the file format. We also recommend that a
       file or class name and description of purpose be included on the
       same "printed page" as the copyright notice for easier
       identification within third-party archives.

    Copyright [yyyy] [name of copyright owner]

    Licensed under the Apache License, Version 2.0 (the "License");
    you may not use this file except in compliance with the License.
    You may obtain a copy of the License at

    	https://www.apache.org/licenses/LICENSE-2.0

    Unless required by applicable law or agreed to in writing, software
    distributed under the License is distributed on an "AS IS" BASIS,
    WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
    See the License for the specific language governing permissions and
    limitations under the License.

`LICENSE-MIT` (adler2 2.0.1):

    Permission is hereby granted, free of charge, to any
    person obtaining a copy of this software and associated
    documentation files (the "Software"), to deal in the
    Software without restriction, including without
    limitation the rights to use, copy, modify, merge,
    publish, distribute, sublicense, and/or sell copies of
    the Software, and to permit persons to whom the Software
    is furnished to do so, subject to the following
    conditions:

    The above copyright notice and this permission notice
    shall be included in all copies or substantial portions
    of the Software.

    THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF
    ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED
    TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A
    PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT
    SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
    CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
    OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR
    IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
    DEALINGS IN THE SOFTWARE.

## ahash 0.8.12

License: MIT OR Apache-2.0. Source: https://github.com/tkaitchuck/ahash
Authors: Tom Kaitchuck <Tom.Kaitchuck@gmail.com>.

`LICENSE-APACHE` (ahash 0.8.12): the same text as `LICENSE-APACHE` (addr2line 0.25.1) above.

`LICENSE-MIT` (ahash 0.8.12):

    Copyright (c) 2018 Tom Kaitchuck

    Permission is hereby granted, free of charge, to any
    person obtaining a copy of this software and associated
    documentation files (the "Software"), to deal in the
    Software without restriction, including without
    limitation the rights to use, copy, modify, merge,
    publish, distribute, sublicense, and/or sell copies of
    the Software, and to permit persons to whom the Software
    is furnished to do so, subject to the following
    conditions:

    The above copyright notice and this permission notice
    shall be included in all copies or substantial portions
    of the Software.

    THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF
    ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED
    TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A
    PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT
    SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
    CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
    OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR
    IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
    DEALINGS IN THE SOFTWARE.

## aho-corasick 1.1.5

License: Unlicense OR MIT. Source: https://github.com/BurntSushi/aho-corasick
Authors: Andrew Gallant <jamslam@gmail.com>.

`COPYING` (aho-corasick 1.1.5):

    This project is dual-licensed under the Unlicense and MIT licenses.

    You may use this code under the terms of either license.

`LICENSE-MIT` (aho-corasick 1.1.5):

    The MIT License (MIT)

    Copyright (c) 2015 Andrew Gallant

    Permission is hereby granted, free of charge, to any person obtaining a copy
    of this software and associated documentation files (the "Software"), to deal
    in the Software without restriction, including without limitation the rights
    to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
    copies of the Software, and to permit persons to whom the Software is
    furnished to do so, subject to the following conditions:

    The above copyright notice and this permission notice shall be included in
    all copies or substantial portions of the Software.

    THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
    IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
    FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
    AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
    LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
    OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN
    THE SOFTWARE.

`UNLICENSE` (aho-corasick 1.1.5):

    This is free and unencumbered software released into the public domain.

    Anyone is free to copy, modify, publish, use, compile, sell, or
    distribute this software, either in source code form or as a compiled
    binary, for any purpose, commercial or non-commercial, and by any
    means.

    In jurisdictions that recognize copyright laws, the author or authors
    of this software dedicate any and all copyright interest in the
    software to the public domain. We make this dedication for the benefit
    of the public at large and to the detriment of our heirs and
    successors. We intend this dedication to be an overt act of
    relinquishment in perpetuity of all present and future rights to this
    software under copyright law.

    THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND,
    EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF
    MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.
    IN NO EVENT SHALL THE AUTHORS BE LIABLE FOR ANY CLAIM, DAMAGES OR
    OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE,
    ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR
    OTHER DEALINGS IN THE SOFTWARE.

    For more information, please refer to <http://unlicense.org/>

## alloc-no-stdlib 2.0.4

License: BSD-3-Clause. Source: https://github.com/dropbox/rust-alloc-no-stdlib
Authors: Daniel Reiter Horn <danielrh@dropbox.com>.

`LICENSE` (alloc-no-stdlib 2.0.4):

    Copyright (c) 2016 Dropbox, Inc.
    All rights reserved.

    Redistribution and use in source and binary forms, with or without modification, are permitted provided that the following conditions are met:

    1. Redistributions of source code must retain the above copyright notice, this list of conditions and the following disclaimer.

    2. Redistributions in binary form must reproduce the above copyright notice, this list of conditions and the following disclaimer in the documentation and/or other materials provided with the distribution.

    3. Neither the name of the copyright holder nor the names of its contributors may be used to endorse or promote products derived from this software without specific prior written permission.

    THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS IS" AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT HOLDER OR CONTRIBUTORS BE LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.

## alloc-stdlib 0.2.4

License: BSD-3-Clause. Source: https://github.com/dropbox/rust-alloc-no-stdlib
Authors: Daniel Reiter Horn <danielrh@dropbox.com>.

Its package carries no license file: the standard text of its license, under its authors.

BSD-3-Clause (alloc-stdlib 0.2.4):

    Copyright (c) Daniel Reiter Horn <danielrh@dropbox.com>

    Redistribution and use in source and binary forms, with or without modification, are permitted provided that the following conditions are met:

    1. Redistributions of source code must retain the above copyright notice, this list of conditions and the following disclaimer.

    2. Redistributions in binary form must reproduce the above copyright notice, this list of conditions and the following disclaimer in the documentation and/or other materials provided with the distribution.

    3. Neither the name of the copyright holder nor the names of its contributors may be used to endorse or promote products derived from this software without specific prior written permission.

    THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS IS" AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT HOLDER OR CONTRIBUTORS BE LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.

## allocator-api2 0.2.21

License: MIT OR Apache-2.0. Source: https://github.com/zakarumych/allocator-api2
Authors: Zakarum <zaq.dev@icloud.com>.

`LICENSE-APACHE` (allocator-api2 0.2.21):

                                  Apache License
                            Version 2.0, January 2004
                         http://www.apache.org/licenses/

    TERMS AND CONDITIONS FOR USE, REPRODUCTION, AND DISTRIBUTION

    1. Definitions.

       "License" shall mean the terms and conditions for use, reproduction,
       and distribution as defined by Sections 1 through 9 of this document.

       "Licensor" shall mean the copyright owner or entity authorized by
       the copyright owner that is granting the License.

       "Legal Entity" shall mean the union of the acting entity and all
       other entities that control, are controlled by, or are under common
       control with that entity. For the purposes of this definition,
       "control" means (i) the power, direct or indirect, to cause the
       direction or management of such entity, whether by contract or
       otherwise, or (ii) ownership of fifty percent (50%) or more of the
       outstanding shares, or (iii) beneficial ownership of such entity.

       "You" (or "Your") shall mean an individual or Legal Entity
       exercising permissions granted by this License.

       "Source" form shall mean the preferred form for making modifications,
       including but not limited to software source code, documentation
       source, and configuration files.

       "Object" form shall mean any form resulting from mechanical
       transformation or translation of a Source form, including but
       not limited to compiled object code, generated documentation,
       and conversions to other media types.

       "Work" shall mean the work of authorship, whether in Source or
       Object form, made available under the License, as indicated by a
       copyright notice that is included in or attached to the work
       (an example is provided in the Appendix below).

       "Derivative Works" shall mean any work, whether in Source or Object
       form, that is based on (or derived from) the Work and for which the
       editorial revisions, annotations, elaborations, or other modifications
       represent, as a whole, an original work of authorship. For the purposes
       of this License, Derivative Works shall not include works that remain
       separable from, or merely link (or bind by name) to the interfaces of,
       the Work and Derivative Works thereof.

       "Contribution" shall mean any work of authorship, including
       the original version of the Work and any modifications or additions
       to that Work or Derivative Works thereof, that is intentionally
       submitted to Licensor for inclusion in the Work by the copyright owner
       or by an individual or Legal Entity authorized to submit on behalf of
       the copyright owner. For the purposes of this definition, "submitted"
       means any form of electronic, verbal, or written communication sent
       to the Licensor or its representatives, including but not limited to
       communication on electronic mailing lists, source code control systems,
       and issue tracking systems that are managed by, or on behalf of, the
       Licensor for the purpose of discussing and improving the Work, but
       excluding communication that is conspicuously marked or otherwise
       designated in writing by the copyright owner as "Not a Contribution."

       "Contributor" shall mean Licensor and any individual or Legal Entity
       on behalf of whom a Contribution has been received by Licensor and
       subsequently incorporated within the Work.

    2. Grant of Copyright License. Subject to the terms and conditions of
       this License, each Contributor hereby grants to You a perpetual,
       worldwide, non-exclusive, no-charge, royalty-free, irrevocable
       copyright license to reproduce, prepare Derivative Works of,
       publicly display, publicly perform, sublicense, and distribute the
       Work and such Derivative Works in Source or Object form.

    3. Grant of Patent License. Subject to the terms and conditions of
       this License, each Contributor hereby grants to You a perpetual,
       worldwide, non-exclusive, no-charge, royalty-free, irrevocable
       (except as stated in this section) patent license to make, have made,
       use, offer to sell, sell, import, and otherwise transfer the Work,
       where such license applies only to those patent claims licensable
       by such Contributor that are necessarily infringed by their
       Contribution(s) alone or by combination of their Contribution(s)
       with the Work to which such Contribution(s) was submitted. If You
       institute patent litigation against any entity (including a
       cross-claim or counterclaim in a lawsuit) alleging that the Work
       or a Contribution incorporated within the Work constitutes direct
       or contributory patent infringement, then any patent licenses
       granted to You under this License for that Work shall terminate
       as of the date such litigation is filed.

    4. Redistribution. You may reproduce and distribute copies of the
       Work or Derivative Works thereof in any medium, with or without
       modifications, and in Source or Object form, provided that You
       meet the following conditions:

       (a) You must give any other recipients of the Work or
           Derivative Works a copy of this License; and

       (b) You must cause any modified files to carry prominent notices
           stating that You changed the files; and

       (c) You must retain, in the Source form of any Derivative Works
           that You distribute, all copyright, patent, trademark, and
           attribution notices from the Source form of the Work,
           excluding those notices that do not pertain to any part of
           the Derivative Works; and

       (d) If the Work includes a "NOTICE" text file as part of its
           distribution, then any Derivative Works that You distribute must
           include a readable copy of the attribution notices contained
           within such NOTICE file, excluding those notices that do not
           pertain to any part of the Derivative Works, in at least one
           of the following places: within a NOTICE text file distributed
           as part of the Derivative Works; within the Source form or
           documentation, if provided along with the Derivative Works; or,
           within a display generated by the Derivative Works, if and
           wherever such third-party notices normally appear. The contents
           of the NOTICE file are for informational purposes only and
           do not modify the License. You may add Your own attribution
           notices within Derivative Works that You distribute, alongside
           or as an addendum to the NOTICE text from the Work, provided
           that such additional attribution notices cannot be construed
           as modifying the License.

       You may add Your own copyright statement to Your modifications and
       may provide additional or different license terms and conditions
       for use, reproduction, or distribution of Your modifications, or
       for any such Derivative Works as a whole, provided Your use,
       reproduction, and distribution of the Work otherwise complies with
       the conditions stated in this License.

    5. Submission of Contributions. Unless You explicitly state otherwise,
       any Contribution intentionally submitted for inclusion in the Work
       by You to the Licensor shall be under the terms and conditions of
       this License, without any additional terms or conditions.
       Notwithstanding the above, nothing herein shall supersede or modify
       the terms of any separate license agreement you may have executed
       with Licensor regarding such Contributions.

    6. Trademarks. This License does not grant permission to use the trade
       names, trademarks, service marks, or product names of the Licensor,
       except as required for reasonable and customary use in describing the
       origin of the Work and reproducing the content of the NOTICE file.

    7. Disclaimer of Warranty. Unless required by applicable law or
       agreed to in writing, Licensor provides the Work (and each
       Contributor provides its Contributions) on an "AS IS" BASIS,
       WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or
       implied, including, without limitation, any warranties or conditions
       of TITLE, NON-INFRINGEMENT, MERCHANTABILITY, or FITNESS FOR A
       PARTICULAR PURPOSE. You are solely responsible for determining the
       appropriateness of using or redistributing the Work and assume any
       risks associated with Your exercise of permissions under this License.

    8. Limitation of Liability. In no event and under no legal theory,
       whether in tort (including negligence), contract, or otherwise,
       unless required by applicable law (such as deliberate and grossly
       negligent acts) or agreed to in writing, shall any Contributor be
       liable to You for damages, including any direct, indirect, special,
       incidental, or consequential damages of any character arising as a
       result of this License or out of the use or inability to use the
       Work (including but not limited to damages for loss of goodwill,
       work stoppage, computer failure or malfunction, or any and all
       other commercial damages or losses), even if such Contributor
       has been advised of the possibility of such damages.

    9. Accepting Warranty or Additional Liability. While redistributing
       the Work or Derivative Works thereof, You may choose to offer,
       and charge a fee for, acceptance of support, warranty, indemnity,
       or other liability obligations and/or rights consistent with this
       License. However, in accepting such obligations, You may act only
       on Your own behalf and on Your sole responsibility, not on behalf
       of any other Contributor, and only if You agree to indemnify,
       defend, and hold each Contributor harmless for any liability
       incurred by, or claims asserted against, such Contributor by reason
       of your accepting any such warranty or additional liability.

    END OF TERMS AND CONDITIONS

`LICENSE-MIT` (allocator-api2 0.2.21): the same text as `LICENSE-MIT` (adler2 2.0.1) above.

## alloy 1.5.2

License: MIT OR Apache-2.0. Source: https://github.com/alloy-rs/alloy
Authors: Alloy Contributors.

Its package carries no license file: the standard text of its license, under its authors.

Apache-2.0 (alloy 1.5.2): the same text as Apache-2.0 above.

MIT (alloy 1.5.2):

    MIT License

    Copyright (c) Alloy Contributors

    Permission is hereby granted, free of charge, to any person obtaining a copy of this software and associated documentation files (the "Software"), to deal in the Software without restriction, including without limitation the rights to use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of the Software, and to permit persons to whom the Software is furnished to do so, subject to the following conditions:

    The above copyright notice and this permission notice shall be included in all copies or substantial portions of the Software.

    THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

## alloy-chains 0.2.34

License: MIT OR Apache-2.0. Source: https://github.com/alloy-rs/chains
Authors: Alloy Contributors.

`LICENSE-APACHE` (alloy-chains 0.2.34): the same text as `LICENSE-APACHE` (allocator-api2 0.2.21) above.

`LICENSE-MIT` (alloy-chains 0.2.34): the same text as `LICENSE-MIT` (adler2 2.0.1) above.

## alloy-consensus 1.8.3

License: MIT OR Apache-2.0. Source: https://github.com/alloy-rs/alloy
Authors: Alloy Contributors.

Its package carries no license file: the standard text of its license, under its authors.

Apache-2.0 (alloy-consensus 1.8.3): the same text as Apache-2.0 above.

MIT (alloy-consensus 1.8.3): the same text as MIT (alloy 1.5.2) above.

## alloy-consensus 2.0.4

License: MIT OR Apache-2.0. Source: https://github.com/alloy-rs/alloy
Authors: Alloy Contributors.

Its package carries no license file: the standard text of its license, under its authors.

Apache-2.0 (alloy-consensus 2.0.4): the same text as Apache-2.0 above.

MIT (alloy-consensus 2.0.4): the same text as MIT (alloy 1.5.2) above.

## alloy-consensus-any 1.8.3

License: MIT OR Apache-2.0. Source: https://github.com/alloy-rs/alloy
Authors: Alloy Contributors.

Its package carries no license file: the standard text of its license, under its authors.

Apache-2.0 (alloy-consensus-any 1.8.3): the same text as Apache-2.0 above.

MIT (alloy-consensus-any 1.8.3): the same text as MIT (alloy 1.5.2) above.

## alloy-consensus-any 2.0.4

License: MIT OR Apache-2.0. Source: https://github.com/alloy-rs/alloy
Authors: Alloy Contributors.

Its package carries no license file: the standard text of its license, under its authors.

Apache-2.0 (alloy-consensus-any 2.0.4): the same text as Apache-2.0 above.

MIT (alloy-consensus-any 2.0.4): the same text as MIT (alloy 1.5.2) above.

## alloy-core 1.5.7

License: MIT OR Apache-2.0. Source: https://github.com/alloy-rs/core
Authors: Alloy Contributors.

Its package carries no license file: the standard text of its license, under its authors.

Apache-2.0 (alloy-core 1.5.7): the same text as Apache-2.0 above.

MIT (alloy-core 1.5.7): the same text as MIT (alloy 1.5.2) above.

## alloy-core 1.7.1

License: MIT OR Apache-2.0. Source: https://github.com/alloy-rs/core
Authors: Alloy Contributors.

Its package carries no license file: the standard text of its license, under its authors.

Apache-2.0 (alloy-core 1.7.1): the same text as Apache-2.0 above.

MIT (alloy-core 1.7.1): the same text as MIT (alloy 1.5.2) above.

## alloy-eip2124 0.2.0

License: MIT OR Apache-2.0. Source: https://github.com/alloy-rs/eips
Authors: Alloy Contributors.

Its package carries no license file: the standard text of its license, under its authors.

Apache-2.0 (alloy-eip2124 0.2.0): the same text as Apache-2.0 above.

MIT (alloy-eip2124 0.2.0): the same text as MIT (alloy 1.5.2) above.

## alloy-eip2930 0.2.3

License: MIT OR Apache-2.0. Source: https://github.com/alloy-rs/eips
Authors: Alloy Contributors.

Its package carries no license file: the standard text of its license, under its authors.

Apache-2.0 (alloy-eip2930 0.2.3): the same text as Apache-2.0 above.

MIT (alloy-eip2930 0.2.3): the same text as MIT (alloy 1.5.2) above.

## alloy-eip2930 0.2.4

License: MIT OR Apache-2.0. Source: https://github.com/alloy-rs/eips
Authors: Alloy Contributors.

Its package carries no license file: the standard text of its license, under its authors.

Apache-2.0 (alloy-eip2930 0.2.4): the same text as Apache-2.0 above.

MIT (alloy-eip2930 0.2.4): the same text as MIT (alloy 1.5.2) above.

## alloy-eip7702 0.6.3

License: MIT OR Apache-2.0. Source: https://github.com/alloy-rs/eips
Authors: Alloy Contributors.

Its package carries no license file: the standard text of its license, under its authors.

Apache-2.0 (alloy-eip7702 0.6.3): the same text as Apache-2.0 above.

MIT (alloy-eip7702 0.6.3): the same text as MIT (alloy 1.5.2) above.

## alloy-eip7928 0.3.7

License: MIT OR Apache-2.0. Source: https://github.com/alloy-rs/eips
Authors: Ishika Choudhury <ishikac720@gmail.com>, Soubhik Singha Mahapatra <soubhiksinghamahapatra@gmail.com>.

Its package carries no license file: the standard text of its license, under its authors.

Apache-2.0 (alloy-eip7928 0.3.7): the same text as Apache-2.0 above.

MIT (alloy-eip7928 0.3.7):

    MIT License

    Copyright (c) Ishika Choudhury <ishikac720@gmail.com>, Soubhik Singha Mahapatra <soubhiksinghamahapatra@gmail.com>

    Permission is hereby granted, free of charge, to any person obtaining a copy of this software and associated documentation files (the "Software"), to deal in the Software without restriction, including without limitation the rights to use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of the Software, and to permit persons to whom the Software is furnished to do so, subject to the following conditions:

    The above copyright notice and this permission notice shall be included in all copies or substantial portions of the Software.

    THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

## alloy-eips 1.8.3

License: MIT OR Apache-2.0. Source: https://github.com/alloy-rs/alloy
Authors: Alloy Contributors.

Its package carries no license file: the standard text of its license, under its authors.

Apache-2.0 (alloy-eips 1.8.3): the same text as Apache-2.0 above.

MIT (alloy-eips 1.8.3): the same text as MIT (alloy 1.5.2) above.

## alloy-eips 2.0.4

License: MIT OR Apache-2.0. Source: https://github.com/alloy-rs/alloy
Authors: Alloy Contributors.

Its package carries no license file: the standard text of its license, under its authors.

Apache-2.0 (alloy-eips 2.0.4): the same text as Apache-2.0 above.

MIT (alloy-eips 2.0.4): the same text as MIT (alloy 1.5.2) above.

## alloy-evm 0.21.3

License: MIT OR Apache-2.0. Source: https://github.com/alloy-rs/alloy-evm
Authors: Alloy Contributors.

Its package carries no license file: the standard text of its license, under its authors.

Apache-2.0 (alloy-evm 0.21.3): the same text as Apache-2.0 above.

MIT (alloy-evm 0.21.3): the same text as MIT (alloy 1.5.2) above.

## alloy-hardforks 0.3.5

License: MIT OR Apache-2.0. Source: https://github.com/alloy-rs/hardforks
Authors: Alloy Contributors.

Its package carries no license file: the standard text of its license, under its authors.

Apache-2.0 (alloy-hardforks 0.3.5): the same text as Apache-2.0 above.

MIT (alloy-hardforks 0.3.5): the same text as MIT (alloy 1.5.2) above.

## alloy-network-primitives 1.8.3

License: MIT OR Apache-2.0. Source: https://github.com/alloy-rs/alloy
Authors: Alloy Contributors.

Its package carries no license file: the standard text of its license, under its authors.

Apache-2.0 (alloy-network-primitives 1.8.3): the same text as Apache-2.0 above.

MIT (alloy-network-primitives 1.8.3): the same text as MIT (alloy 1.5.2) above.

## alloy-network-primitives 2.0.4

License: MIT OR Apache-2.0. Source: https://github.com/alloy-rs/alloy
Authors: Alloy Contributors.

Its package carries no license file: the standard text of its license, under its authors.

Apache-2.0 (alloy-network-primitives 2.0.4): the same text as Apache-2.0 above.

MIT (alloy-network-primitives 2.0.4): the same text as MIT (alloy 1.5.2) above.

## alloy-primitives 1.5.7

License: MIT OR Apache-2.0. Source: https://github.com/alloy-rs/core
Authors: Alloy Contributors.

Its package carries no license file: the standard text of its license, under its authors.

Apache-2.0 (alloy-primitives 1.5.7): the same text as Apache-2.0 above.

MIT (alloy-primitives 1.5.7): the same text as MIT (alloy 1.5.2) above.

## alloy-primitives 1.7.1

License: MIT OR Apache-2.0. Source: https://github.com/alloy-rs/core
Authors: Alloy Contributors.

Its package carries no license file: the standard text of its license, under its authors.

Apache-2.0 (alloy-primitives 1.7.1): the same text as Apache-2.0 above.

MIT (alloy-primitives 1.7.1): the same text as MIT (alloy 1.5.2) above.

## alloy-primitives 1.7.3

License: MIT OR Apache-2.0. Source: https://github.com/alloy-rs/core
Authors: Alloy Contributors.

Its package carries no license file: the standard text of its license, under its authors.

Apache-2.0 (alloy-primitives 1.7.3): the same text as Apache-2.0 above.

MIT (alloy-primitives 1.7.3): the same text as MIT (alloy 1.5.2) above.

## alloy-rlp 0.3.16

License: MIT OR Apache-2.0. Source: https://github.com/alloy-rs/rlp
Authors: Alloy Contributors.

Its package carries no license file: the standard text of its license, under its authors.

Apache-2.0 (alloy-rlp 0.3.16): the same text as Apache-2.0 above.

MIT (alloy-rlp 0.3.16): the same text as MIT (alloy 1.5.2) above.

## alloy-rlp 0.3.17

License: MIT OR Apache-2.0. Source: https://github.com/alloy-rs/rlp
Authors: Alloy Contributors.

Its package carries no license file: the standard text of its license, under its authors.

Apache-2.0 (alloy-rlp 0.3.17): the same text as Apache-2.0 above.

MIT (alloy-rlp 0.3.17): the same text as MIT (alloy 1.5.2) above.

## alloy-rpc-types 1.5.2

License: MIT OR Apache-2.0. Source: https://github.com/alloy-rs/alloy
Authors: Alloy Contributors.

Its package carries no license file: the standard text of its license, under its authors.

Apache-2.0 (alloy-rpc-types 1.5.2): the same text as Apache-2.0 above.

MIT (alloy-rpc-types 1.5.2): the same text as MIT (alloy 1.5.2) above.

## alloy-rpc-types 1.8.3

License: MIT OR Apache-2.0. Source: https://github.com/alloy-rs/alloy
Authors: Alloy Contributors.

Its package carries no license file: the standard text of its license, under its authors.

Apache-2.0 (alloy-rpc-types 1.8.3): the same text as Apache-2.0 above.

MIT (alloy-rpc-types 1.8.3): the same text as MIT (alloy 1.5.2) above.

## alloy-rpc-types-eth 1.5.2

License: MIT OR Apache-2.0. Source: https://github.com/alloy-rs/alloy
Authors: Alloy Contributors.

Its package carries no license file: the standard text of its license, under its authors.

Apache-2.0 (alloy-rpc-types-eth 1.5.2): the same text as Apache-2.0 above.

MIT (alloy-rpc-types-eth 1.5.2): the same text as MIT (alloy 1.5.2) above.

## alloy-rpc-types-eth 1.8.3

License: MIT OR Apache-2.0. Source: https://github.com/alloy-rs/alloy
Authors: Alloy Contributors.

Its package carries no license file: the standard text of its license, under its authors.

Apache-2.0 (alloy-rpc-types-eth 1.8.3): the same text as Apache-2.0 above.

MIT (alloy-rpc-types-eth 1.8.3): the same text as MIT (alloy 1.5.2) above.

## alloy-rpc-types-eth 2.0.4

License: MIT OR Apache-2.0. Source: https://github.com/alloy-rs/alloy
Authors: Alloy Contributors.

Its package carries no license file: the standard text of its license, under its authors.

Apache-2.0 (alloy-rpc-types-eth 2.0.4): the same text as Apache-2.0 above.

MIT (alloy-rpc-types-eth 2.0.4): the same text as MIT (alloy 1.5.2) above.

## alloy-rpc-types-trace 1.5.2

License: MIT OR Apache-2.0. Source: https://github.com/alloy-rs/alloy
Authors: Alloy Contributors.

Its package carries no license file: the standard text of its license, under its authors.

Apache-2.0 (alloy-rpc-types-trace 1.5.2): the same text as Apache-2.0 above.

MIT (alloy-rpc-types-trace 1.5.2): the same text as MIT (alloy 1.5.2) above.

## alloy-rpc-types-trace 2.0.4

License: MIT OR Apache-2.0. Source: https://github.com/alloy-rs/alloy
Authors: Alloy Contributors.

Its package carries no license file: the standard text of its license, under its authors.

Apache-2.0 (alloy-rpc-types-trace 2.0.4): the same text as Apache-2.0 above.

MIT (alloy-rpc-types-trace 2.0.4): the same text as MIT (alloy 1.5.2) above.

## alloy-serde 1.8.3

License: MIT OR Apache-2.0. Source: https://github.com/alloy-rs/alloy
Authors: Alloy Contributors.

Its package carries no license file: the standard text of its license, under its authors.

Apache-2.0 (alloy-serde 1.8.3): the same text as Apache-2.0 above.

MIT (alloy-serde 1.8.3): the same text as MIT (alloy 1.5.2) above.

## alloy-serde 2.0.4

License: MIT OR Apache-2.0. Source: https://github.com/alloy-rs/alloy
Authors: Alloy Contributors.

Its package carries no license file: the standard text of its license, under its authors.

Apache-2.0 (alloy-serde 2.0.4): the same text as Apache-2.0 above.

MIT (alloy-serde 2.0.4): the same text as MIT (alloy 1.5.2) above.

## alloy-sol-types 1.5.7

License: MIT OR Apache-2.0. Source: https://github.com/alloy-rs/core
Authors: Alloy Contributors.

Its package carries no license file: the standard text of its license, under its authors.

Apache-2.0 (alloy-sol-types 1.5.7): the same text as Apache-2.0 above.

MIT (alloy-sol-types 1.5.7): the same text as MIT (alloy 1.5.2) above.

## alloy-sol-types 1.7.1

License: MIT OR Apache-2.0. Source: https://github.com/alloy-rs/core
Authors: Alloy Contributors.

Its package carries no license file: the standard text of its license, under its authors.

Apache-2.0 (alloy-sol-types 1.7.1): the same text as Apache-2.0 above.

MIT (alloy-sol-types 1.7.1): the same text as MIT (alloy 1.5.2) above.

## alloy-trie 0.9.5

License: MIT OR Apache-2.0. Source: https://github.com/alloy-rs/trie
Authors: rkrasiuk <rokrassyuk@gmail.com>, gakonst <me@gakonst.com>, DaniPopes <57450786+DaniPopes@users.noreply.github.com>.

`LICENSE-APACHE` (alloy-trie 0.9.5): the same text as `LICENSE-APACHE` (allocator-api2 0.2.21) above.

`LICENSE-MIT` (alloy-trie 0.9.5): the same text as `LICENSE-MIT` (adler2 2.0.1) above.

## alloy-trie 0.9.8

License: MIT OR Apache-2.0. Source: https://github.com/alloy-rs/trie
Authors: rkrasiuk <rokrassyuk@gmail.com>, gakonst <me@gakonst.com>, DaniPopes <57450786+DaniPopes@users.noreply.github.com>.

`LICENSE-APACHE` (alloy-trie 0.9.8): the same text as `LICENSE-APACHE` (allocator-api2 0.2.21) above.

`LICENSE-MIT` (alloy-trie 0.9.8): the same text as `LICENSE-MIT` (adler2 2.0.1) above.

## alloy-trie 0.9.9

License: MIT OR Apache-2.0. Source: https://github.com/alloy-rs/trie
Authors: rkrasiuk <rokrassyuk@gmail.com>, gakonst <me@gakonst.com>, DaniPopes <57450786+DaniPopes@users.noreply.github.com>.

`LICENSE-APACHE` (alloy-trie 0.9.9): the same text as `LICENSE-APACHE` (allocator-api2 0.2.21) above.

`LICENSE-MIT` (alloy-trie 0.9.9): the same text as `LICENSE-MIT` (adler2 2.0.1) above.

## ansi_term 0.12.1

License: MIT. Source: https://github.com/ogham/rust-ansi-term
Authors: ogham@bsago.me, Ryan Scheel (Havvy) <ryan.havvy@gmail.com>, Josh Triplett <josh@joshtriplett.org>.

`LICENCE` (ansi_term 0.12.1):

    The MIT License (MIT)

    Copyright (c) 2014 Benjamin Sago

    Permission is hereby granted, free of charge, to any person obtaining a copy
    of this software and associated documentation files (the "Software"), to deal
    in the Software without restriction, including without limitation the rights
    to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
    copies of the Software, and to permit persons to whom the Software is
    furnished to do so, subject to the following conditions:

    The above copyright notice and this permission notice shall be included in all
    copies or substantial portions of the Software.

    THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
    IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
    FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
    AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
    LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
    OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
    SOFTWARE.

## anstyle 1.0.14

License: MIT OR Apache-2.0. Source: https://github.com/rust-cli/anstyle.git

`LICENSE-APACHE` (anstyle 1.0.14):

                                     Apache License
                               Version 2.0, January 2004
                            http://www.apache.org/licenses/

       TERMS AND CONDITIONS FOR USE, REPRODUCTION, AND DISTRIBUTION

       1. Definitions.

          "License" shall mean the terms and conditions for use, reproduction,
          and distribution as defined by Sections 1 through 9 of this document.

          "Licensor" shall mean the copyright owner or entity authorized by
          the copyright owner that is granting the License.

          "Legal Entity" shall mean the union of the acting entity and all
          other entities that control, are controlled by, or are under common
          control with that entity. For the purposes of this definition,
          "control" means (i) the power, direct or indirect, to cause the
          direction or management of such entity, whether by contract or
          otherwise, or (ii) ownership of fifty percent (50%) or more of the
          outstanding shares, or (iii) beneficial ownership of such entity.

          "You" (or "Your") shall mean an individual or Legal Entity
          exercising permissions granted by this License.

          "Source" form shall mean the preferred form for making modifications,
          including but not limited to software source code, documentation
          source, and configuration files.

          "Object" form shall mean any form resulting from mechanical
          transformation or translation of a Source form, including but
          not limited to compiled object code, generated documentation,
          and conversions to other media types.

          "Work" shall mean the work of authorship, whether in Source or
          Object form, made available under the License, as indicated by a
          copyright notice that is included in or attached to the work
          (an example is provided in the Appendix below).

          "Derivative Works" shall mean any work, whether in Source or Object
          form, that is based on (or derived from) the Work and for which the
          editorial revisions, annotations, elaborations, or other modifications
          represent, as a whole, an original work of authorship. For the purposes
          of this License, Derivative Works shall not include works that remain
          separable from, or merely link (or bind by name) to the interfaces of,
          the Work and Derivative Works thereof.

          "Contribution" shall mean any work of authorship, including
          the original version of the Work and any modifications or additions
          to that Work or Derivative Works thereof, that is intentionally
          submitted to Licensor for inclusion in the Work by the copyright owner
          or by an individual or Legal Entity authorized to submit on behalf of
          the copyright owner. For the purposes of this definition, "submitted"
          means any form of electronic, verbal, or written communication sent
          to the Licensor or its representatives, including but not limited to
          communication on electronic mailing lists, source code control systems,
          and issue tracking systems that are managed by, or on behalf of, the
          Licensor for the purpose of discussing and improving the Work, but
          excluding communication that is conspicuously marked or otherwise
          designated in writing by the copyright owner as "Not a Contribution."

          "Contributor" shall mean Licensor and any individual or Legal Entity
          on behalf of whom a Contribution has been received by Licensor and
          subsequently incorporated within the Work.

       2. Grant of Copyright License. Subject to the terms and conditions of
          this License, each Contributor hereby grants to You a perpetual,
          worldwide, non-exclusive, no-charge, royalty-free, irrevocable
          copyright license to reproduce, prepare Derivative Works of,
          publicly display, publicly perform, sublicense, and distribute the
          Work and such Derivative Works in Source or Object form.

       3. Grant of Patent License. Subject to the terms and conditions of
          this License, each Contributor hereby grants to You a perpetual,
          worldwide, non-exclusive, no-charge, royalty-free, irrevocable
          (except as stated in this section) patent license to make, have made,
          use, offer to sell, sell, import, and otherwise transfer the Work,
          where such license applies only to those patent claims licensable
          by such Contributor that are necessarily infringed by their
          Contribution(s) alone or by combination of their Contribution(s)
          with the Work to which such Contribution(s) was submitted. If You
          institute patent litigation against any entity (including a
          cross-claim or counterclaim in a lawsuit) alleging that the Work
          or a Contribution incorporated within the Work constitutes direct
          or contributory patent infringement, then any patent licenses
          granted to You under this License for that Work shall terminate
          as of the date such litigation is filed.

       4. Redistribution. You may reproduce and distribute copies of the
          Work or Derivative Works thereof in any medium, with or without
          modifications, and in Source or Object form, provided that You
          meet the following conditions:

          (a) You must give any other recipients of the Work or
              Derivative Works a copy of this License; and

          (b) You must cause any modified files to carry prominent notices
              stating that You changed the files; and

          (c) You must retain, in the Source form of any Derivative Works
              that You distribute, all copyright, patent, trademark, and
              attribution notices from the Source form of the Work,
              excluding those notices that do not pertain to any part of
              the Derivative Works; and

          (d) If the Work includes a "NOTICE" text file as part of its
              distribution, then any Derivative Works that You distribute must
              include a readable copy of the attribution notices contained
              within such NOTICE file, excluding those notices that do not
              pertain to any part of the Derivative Works, in at least one
              of the following places: within a NOTICE text file distributed
              as part of the Derivative Works; within the Source form or
              documentation, if provided along with the Derivative Works; or,
              within a display generated by the Derivative Works, if and
              wherever such third-party notices normally appear. The contents
              of the NOTICE file are for informational purposes only and
              do not modify the License. You may add Your own attribution
              notices within Derivative Works that You distribute, alongside
              or as an addendum to the NOTICE text from the Work, provided
              that such additional attribution notices cannot be construed
              as modifying the License.

          You may add Your own copyright statement to Your modifications and
          may provide additional or different license terms and conditions
          for use, reproduction, or distribution of Your modifications, or
          for any such Derivative Works as a whole, provided Your use,
          reproduction, and distribution of the Work otherwise complies with
          the conditions stated in this License.

       5. Submission of Contributions. Unless You explicitly state otherwise,
          any Contribution intentionally submitted for inclusion in the Work
          by You to the Licensor shall be under the terms and conditions of
          this License, without any additional terms or conditions.
          Notwithstanding the above, nothing herein shall supersede or modify
          the terms of any separate license agreement you may have executed
          with Licensor regarding such Contributions.

       6. Trademarks. This License does not grant permission to use the trade
          names, trademarks, service marks, or product names of the Licensor,
          except as required for reasonable and customary use in describing the
          origin of the Work and reproducing the content of the NOTICE file.

       7. Disclaimer of Warranty. Unless required by applicable law or
          agreed to in writing, Licensor provides the Work (and each
          Contributor provides its Contributions) on an "AS IS" BASIS,
          WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or
          implied, including, without limitation, any warranties or conditions
          of TITLE, NON-INFRINGEMENT, MERCHANTABILITY, or FITNESS FOR A
          PARTICULAR PURPOSE. You are solely responsible for determining the
          appropriateness of using or redistributing the Work and assume any
          risks associated with Your exercise of permissions under this License.

       8. Limitation of Liability. In no event and under no legal theory,
          whether in tort (including negligence), contract, or otherwise,
          unless required by applicable law (such as deliberate and grossly
          negligent acts) or agreed to in writing, shall any Contributor be
          liable to You for damages, including any direct, indirect, special,
          incidental, or consequential damages of any character arising as a
          result of this License or out of the use or inability to use the
          Work (including but not limited to damages for loss of goodwill,
          work stoppage, computer failure or malfunction, or any and all
          other commercial damages or losses), even if such Contributor
          has been advised of the possibility of such damages.

       9. Accepting Warranty or Additional Liability. While redistributing
          the Work or Derivative Works thereof, You may choose to offer,
          and charge a fee for, acceptance of support, warranty, indemnity,
          or other liability obligations and/or rights consistent with this
          License. However, in accepting such obligations, You may act only
          on Your own behalf and on Your sole responsibility, not on behalf
          of any other Contributor, and only if You agree to indemnify,
          defend, and hold each Contributor harmless for any liability
          incurred by, or claims asserted against, such Contributor by reason
          of your accepting any such warranty or additional liability.

       END OF TERMS AND CONDITIONS

       APPENDIX: How to apply the Apache License to your work.

          To apply the Apache License to your work, attach the following
          boilerplate notice, with the fields enclosed by brackets "{}"
          replaced with your own identifying information. (Don't include
          the brackets!)  The text should be enclosed in the appropriate
          comment syntax for the file format. We also recommend that a
          file or class name and description of purpose be included on the
          same "printed page" as the copyright notice for easier
          identification within third-party archives.

       Copyright {yyyy} {name of copyright owner}

       Licensed under the Apache License, Version 2.0 (the "License");
       you may not use this file except in compliance with the License.
       You may obtain a copy of the License at

           http://www.apache.org/licenses/LICENSE-2.0

       Unless required by applicable law or agreed to in writing, software
       distributed under the License is distributed on an "AS IS" BASIS,
       WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
       See the License for the specific language governing permissions and
       limitations under the License.

`LICENSE-MIT` (anstyle 1.0.14):

    Copyright (c) Individual contributors

    Permission is hereby granted, free of charge, to any person obtaining a copy
    of this software and associated documentation files (the "Software"), to deal
    in the Software without restriction, including without limitation the rights
    to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
    copies of the Software, and to permit persons to whom the Software is
    furnished to do so, subject to the following conditions:

    The above copyright notice and this permission notice shall be included in all
    copies or substantial portions of the Software.

    THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
    IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
    FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
    AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
    LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
    OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
    SOFTWARE.

## anyhow 1.0.104

License: MIT OR Apache-2.0. Source: https://github.com/dtolnay/anyhow
Authors: David Tolnay <dtolnay@gmail.com>.

`LICENSE-APACHE` (anyhow 1.0.104): the same text as `LICENSE-APACHE` (allocator-api2 0.2.21) above.

`LICENSE-MIT` (anyhow 1.0.104): the same text as `LICENSE-MIT` (adler2 2.0.1) above.

## ark-bls12-381 0.5.0

License: MIT OR Apache-2.0. Source: https://github.com/arkworks-rs/algebra
Authors: arkworks contributors.

`LICENSE-APACHE` (ark-bls12-381 0.5.0):

                                  Apache License
                            Version 2.0, January 2004
                         http://www.apache.org/licenses/

    TERMS AND CONDITIONS FOR USE, REPRODUCTION, AND DISTRIBUTION

    1. Definitions.

       "License" shall mean the terms and conditions for use, reproduction,
       and distribution as defined by Sections 1 through 9 of this document.

       "Licensor" shall mean the copyright owner or entity authorized by
       the copyright owner that is granting the License.

       "Legal Entity" shall mean the union of the acting entity and all
       other entities that control, are controlled by, or are under common
       control with that entity. For the purposes of this definition,
       "control" means (i) the power, direct or indirect, to cause the
       direction or management of such entity, whether by contract or
       otherwise, or (ii) ownership of fifty percent (50%) or more of the
       outstanding shares, or (iii) beneficial ownership of such entity.

       "You" (or "Your") shall mean an individual or Legal Entity
       exercising permissions granted by this License.

       "Source" form shall mean the preferred form for making modifications,
       including but not limited to software source code, documentation
       source, and configuration files.

       "Object" form shall mean any form resulting from mechanical
       transformation or translation of a Source form, including but
       not limited to compiled object code, generated documentation,
       and conversions to other media types.

       "Work" shall mean the work of authorship, whether in Source or
       Object form, made available under the License, as indicated by a
       copyright notice that is included in or attached to the work
       (an example is provided in the Appendix below).

       "Derivative Works" shall mean any work, whether in Source or Object
       form, that is based on (or derived from) the Work and for which the
       editorial revisions, annotations, elaborations, or other modifications
       represent, as a whole, an original work of authorship. For the purposes
       of this License, Derivative Works shall not include works that remain
       separable from, or merely link (or bind by name) to the interfaces of,
       the Work and Derivative Works thereof.

       "Contribution" shall mean any work of authorship, including
       the original version of the Work and any modifications or additions
       to that Work or Derivative Works thereof, that is intentionally
       submitted to Licensor for inclusion in the Work by the copyright owner
       or by an individual or Legal Entity authorized to submit on behalf of
       the copyright owner. For the purposes of this definition, "submitted"
       means any form of electronic, verbal, or written communication sent
       to the Licensor or its representatives, including but not limited to
       communication on electronic mailing lists, source code control systems,
       and issue tracking systems that are managed by, or on behalf of, the
       Licensor for the purpose of discussing and improving the Work, but
       excluding communication that is conspicuously marked or otherwise
       designated in writing by the copyright owner as "Not a Contribution."

       "Contributor" shall mean Licensor and any individual or Legal Entity
       on behalf of whom a Contribution has been received by Licensor and
       subsequently incorporated within the Work.

    2. Grant of Copyright License. Subject to the terms and conditions of
       this License, each Contributor hereby grants to You a perpetual,
       worldwide, non-exclusive, no-charge, royalty-free, irrevocable
       copyright license to reproduce, prepare Derivative Works of,
       publicly display, publicly perform, sublicense, and distribute the
       Work and such Derivative Works in Source or Object form.

    3. Grant of Patent License. Subject to the terms and conditions of
       this License, each Contributor hereby grants to You a perpetual,
       worldwide, non-exclusive, no-charge, royalty-free, irrevocable
       (except as stated in this section) patent license to make, have made,
       use, offer to sell, sell, import, and otherwise transfer the Work,
       where such license applies only to those patent claims licensable
       by such Contributor that are necessarily infringed by their
       Contribution(s) alone or by combination of their Contribution(s)
       with the Work to which such Contribution(s) was submitted. If You
       institute patent litigation against any entity (including a
       cross-claim or counterclaim in a lawsuit) alleging that the Work
       or a Contribution incorporated within the Work constitutes direct
       or contributory patent infringement, then any patent licenses
       granted to You under this License for that Work shall terminate
       as of the date such litigation is filed.

    4. Redistribution. You may reproduce and distribute copies of the
       Work or Derivative Works thereof in any medium, with or without
       modifications, and in Source or Object form, provided that You
       meet the following conditions:

       (a) You must give any other recipients of the Work or
           Derivative Works a copy of this License; and

       (b) You must cause any modified files to carry prominent notices
           stating that You changed the files; and

       (c) You must retain, in the Source form of any Derivative Works
           that You distribute, all copyright, patent, trademark, and
           attribution notices from the Source form of the Work,
           excluding those notices that do not pertain to any part of
           the Derivative Works; and

       (d) If the Work includes a "NOTICE" text file as part of its
           distribution, then any Derivative Works that You distribute must
           include a readable copy of the attribution notices contained
           within such NOTICE file, excluding those notices that do not
           pertain to any part of the Derivative Works, in at least one
           of the following places: within a NOTICE text file distributed
           as part of the Derivative Works; within the Source form or
           documentation, if provided along with the Derivative Works; or,
           within a display generated by the Derivative Works, if and
           wherever such third-party notices normally appear. The contents
           of the NOTICE file are for informational purposes only and
           do not modify the License. You may add Your own attribution
           notices within Derivative Works that You distribute, alongside
           or as an addendum to the NOTICE text from the Work, provided
           that such additional attribution notices cannot be construed
           as modifying the License.

       You may add Your own copyright statement to Your modifications and
       may provide additional or different license terms and conditions
       for use, reproduction, or distribution of Your modifications, or
       for any such Derivative Works as a whole, provided Your use,
       reproduction, and distribution of the Work otherwise complies with
       the conditions stated in this License.

    5. Submission of Contributions. Unless You explicitly state otherwise,
       any Contribution intentionally submitted for inclusion in the Work
       by You to the Licensor shall be under the terms and conditions of
       this License, without any additional terms or conditions.
       Notwithstanding the above, nothing herein shall supersede or modify
       the terms of any separate license agreement you may have executed
       with Licensor regarding such Contributions.

    6. Trademarks. This License does not grant permission to use the trade
       names, trademarks, service marks, or product names of the Licensor,
       except as required for reasonable and customary use in describing the
       origin of the Work and reproducing the content of the NOTICE file.

    7. Disclaimer of Warranty. Unless required by applicable law or
       agreed to in writing, Licensor provides the Work (and each
       Contributor provides its Contributions) on an "AS IS" BASIS,
       WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or
       implied, including, without limitation, any warranties or conditions
       of TITLE, NON-INFRINGEMENT, MERCHANTABILITY, or FITNESS FOR A
       PARTICULAR PURPOSE. You are solely responsible for determining the
       appropriateness of using or redistributing the Work and assume any
       risks associated with Your exercise of permissions under this License.

    8. Limitation of Liability. In no event and under no legal theory,
       whether in tort (including negligence), contract, or otherwise,
       unless required by applicable law (such as deliberate and grossly
       negligent acts) or agreed to in writing, shall any Contributor be
       liable to You for damages, including any direct, indirect, special,
       incidental, or consequential damages of any character arising as a
       result of this License or out of the use or inability to use the
       Work (including but not limited to damages for loss of goodwill,
       work stoppage, computer failure or malfunction, or any and all
       other commercial damages or losses), even if such Contributor
       has been advised of the possibility of such damages.

    9. Accepting Warranty or Additional Liability. While redistributing
       the Work or Derivative Works thereof, You may choose to offer,
       and charge a fee for, acceptance of support, warranty, indemnity,
       or other liability obligations and/or rights consistent with this
       License. However, in accepting such obligations, You may act only
       on Your own behalf and on Your sole responsibility, not on behalf
       of any other Contributor, and only if You agree to indemnify,
       defend, and hold each Contributor harmless for any liability
       incurred by, or claims asserted against, such Contributor by reason
       of your accepting any such warranty or additional liability.

    END OF TERMS AND CONDITIONS

    APPENDIX: How to apply the Apache License to your work.

       To apply the Apache License to your work, attach the following
       boilerplate notice, with the fields enclosed by brackets "[]"
       replaced with your own identifying information. (Don't include
       the brackets!)  The text should be enclosed in the appropriate
       comment syntax for the file format. We also recommend that a
       file or class name and description of purpose be included on the
       same "printed page" as the copyright notice for easier
       identification within third-party archives.

    Copyright 2022 arkworks contributors

    Licensed under the Apache License, Version 2.0 (the "License");
    you may not use this file except in compliance with the License.
    You may obtain a copy of the License at

          http://www.apache.org/licenses/LICENSE-2.0

    Unless required by applicable law or agreed to in writing, software
    distributed under the License is distributed on an "AS IS" BASIS,
    WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
    See the License for the specific language governing permissions and
    limitations under the License.

`LICENSE-MIT` (ark-bls12-381 0.5.0):

    The MIT License (MIT)

    Permission is hereby granted, free of charge, to any person obtaining a copy
    of this software and associated documentation files (the "Software"), to deal
    in the Software without restriction, including without limitation the rights
    to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
    copies of the Software, and to permit persons to whom the Software is
    furnished to do so, subject to the following conditions:

    The above copyright notice and this permission notice shall be included in
    all copies or substantial portions of the Software.

    THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
    IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
    FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
    AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
    LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
    OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN
    THE SOFTWARE.

## ark-bn254 0.5.0

License: MIT OR Apache-2.0. Source: https://github.com/arkworks-rs/algebra
Authors: arkworks contributors.

`LICENSE-APACHE` (ark-bn254 0.5.0): the same text as `LICENSE-APACHE` (ark-bls12-381 0.5.0) above.

`LICENSE-MIT` (ark-bn254 0.5.0): the same text as `LICENSE-MIT` (ark-bls12-381 0.5.0) above.

## ark-ec 0.5.0

License: MIT OR Apache-2.0. Source: https://github.com/arkworks-rs/algebra
Authors: arkworks contributors.

`LICENSE-APACHE` (ark-ec 0.5.0): the same text as `LICENSE-APACHE` (ark-bls12-381 0.5.0) above.

`LICENSE-MIT` (ark-ec 0.5.0): the same text as `LICENSE-MIT` (ark-bls12-381 0.5.0) above.

## ark-ff 0.4.2

License: MIT/Apache-2.0. Source: https://github.com/arkworks-rs/algebra
Authors: arkworks contributors.

`LICENSE-APACHE` (ark-ff 0.4.2):

                                  Apache License
                            Version 2.0, January 2004
                         http://www.apache.org/licenses/

    TERMS AND CONDITIONS FOR USE, REPRODUCTION, AND DISTRIBUTION

    1. Definitions.

       "License" shall mean the terms and conditions for use, reproduction,
       and distribution as defined by Sections 1 through 9 of this document.

       "Licensor" shall mean the copyright owner or entity authorized by
       the copyright owner that is granting the License.

       "Legal Entity" shall mean the union of the acting entity and all
       other entities that control, are controlled by, or are under common
       control with that entity. For the purposes of this definition,
       "control" means (i) the power, direct or indirect, to cause the
       direction or management of such entity, whether by contract or
       otherwise, or (ii) ownership of fifty percent (50%) or more of the
       outstanding shares, or (iii) beneficial ownership of such entity.

       "You" (or "Your") shall mean an individual or Legal Entity
       exercising permissions granted by this License.

       "Source" form shall mean the preferred form for making modifications,
       including but not limited to software source code, documentation
       source, and configuration files.

       "Object" form shall mean any form resulting from mechanical
       transformation or translation of a Source form, including but
       not limited to compiled object code, generated documentation,
       and conversions to other media types.

       "Work" shall mean the work of authorship, whether in Source or
       Object form, made available under the License, as indicated by a
       copyright notice that is included in or attached to the work
       (an example is provided in the Appendix below).

       "Derivative Works" shall mean any work, whether in Source or Object
       form, that is based on (or derived from) the Work and for which the
       editorial revisions, annotations, elaborations, or other modifications
       represent, as a whole, an original work of authorship. For the purposes
       of this License, Derivative Works shall not include works that remain
       separable from, or merely link (or bind by name) to the interfaces of,
       the Work and Derivative Works thereof.

       "Contribution" shall mean any work of authorship, including
       the original version of the Work and any modifications or additions
       to that Work or Derivative Works thereof, that is intentionally
       submitted to Licensor for inclusion in the Work by the copyright owner
       or by an individual or Legal Entity authorized to submit on behalf of
       the copyright owner. For the purposes of this definition, "submitted"
       means any form of electronic, verbal, or written communication sent
       to the Licensor or its representatives, including but not limited to
       communication on electronic mailing lists, source code control systems,
       and issue tracking systems that are managed by, or on behalf of, the
       Licensor for the purpose of discussing and improving the Work, but
       excluding communication that is conspicuously marked or otherwise
       designated in writing by the copyright owner as "Not a Contribution."

       "Contributor" shall mean Licensor and any individual or Legal Entity
       on behalf of whom a Contribution has been received by Licensor and
       subsequently incorporated within the Work.

    2. Grant of Copyright License. Subject to the terms and conditions of
       this License, each Contributor hereby grants to You a perpetual,
       worldwide, non-exclusive, no-charge, royalty-free, irrevocable
       copyright license to reproduce, prepare Derivative Works of,
       publicly display, publicly perform, sublicense, and distribute the
       Work and such Derivative Works in Source or Object form.

    3. Grant of Patent License. Subject to the terms and conditions of
       this License, each Contributor hereby grants to You a perpetual,
       worldwide, non-exclusive, no-charge, royalty-free, irrevocable
       (except as stated in this section) patent license to make, have made,
       use, offer to sell, sell, import, and otherwise transfer the Work,
       where such license applies only to those patent claims licensable
       by such Contributor that are necessarily infringed by their
       Contribution(s) alone or by combination of their Contribution(s)
       with the Work to which such Contribution(s) was submitted. If You
       institute patent litigation against any entity (including a
       cross-claim or counterclaim in a lawsuit) alleging that the Work
       or a Contribution incorporated within the Work constitutes direct
       or contributory patent infringement, then any patent licenses
       granted to You under this License for that Work shall terminate
       as of the date such litigation is filed.

    4. Redistribution. You may reproduce and distribute copies of the
       Work or Derivative Works thereof in any medium, with or without
       modifications, and in Source or Object form, provided that You
       meet the following conditions:

       (a) You must give any other recipients of the Work or
           Derivative Works a copy of this License; and

       (b) You must cause any modified files to carry prominent notices
           stating that You changed the files; and

       (c) You must retain, in the Source form of any Derivative Works
           that You distribute, all copyright, patent, trademark, and
           attribution notices from the Source form of the Work,
           excluding those notices that do not pertain to any part of
           the Derivative Works; and

       (d) If the Work includes a "NOTICE" text file as part of its
           distribution, then any Derivative Works that You distribute must
           include a readable copy of the attribution notices contained
           within such NOTICE file, excluding those notices that do not
           pertain to any part of the Derivative Works, in at least one
           of the following places: within a NOTICE text file distributed
           as part of the Derivative Works; within the Source form or
           documentation, if provided along with the Derivative Works; or,
           within a display generated by the Derivative Works, if and
           wherever such third-party notices normally appear. The contents
           of the NOTICE file are for informational purposes only and
           do not modify the License. You may add Your own attribution
           notices within Derivative Works that You distribute, alongside
           or as an addendum to the NOTICE text from the Work, provided
           that such additional attribution notices cannot be construed
           as modifying the License.

       You may add Your own copyright statement to Your modifications and
       may provide additional or different license terms and conditions
       for use, reproduction, or distribution of Your modifications, or
       for any such Derivative Works as a whole, provided Your use,
       reproduction, and distribution of the Work otherwise complies with
       the conditions stated in this License.

    5. Submission of Contributions. Unless You explicitly state otherwise,
       any Contribution intentionally submitted for inclusion in the Work
       by You to the Licensor shall be under the terms and conditions of
       this License, without any additional terms or conditions.
       Notwithstanding the above, nothing herein shall supersede or modify
       the terms of any separate license agreement you may have executed
       with Licensor regarding such Contributions.

    6. Trademarks. This License does not grant permission to use the trade
       names, trademarks, service marks, or product names of the Licensor,
       except as required for reasonable and customary use in describing the
       origin of the Work and reproducing the content of the NOTICE file.

    7. Disclaimer of Warranty. Unless required by applicable law or
       agreed to in writing, Licensor provides the Work (and each
       Contributor provides its Contributions) on an "AS IS" BASIS,
       WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or
       implied, including, without limitation, any warranties or conditions
       of TITLE, NON-INFRINGEMENT, MERCHANTABILITY, or FITNESS FOR A
       PARTICULAR PURPOSE. You are solely responsible for determining the
       appropriateness of using or redistributing the Work and assume any
       risks associated with Your exercise of permissions under this License.

    8. Limitation of Liability. In no event and under no legal theory,
       whether in tort (including negligence), contract, or otherwise,
       unless required by applicable law (such as deliberate and grossly
       negligent acts) or agreed to in writing, shall any Contributor be
       liable to You for damages, including any direct, indirect, special,
       incidental, or consequential damages of any character arising as a
       result of this License or out of the use or inability to use the
       Work (including but not limited to damages for loss of goodwill,
       work stoppage, computer failure or malfunction, or any and all
       other commercial damages or losses), even if such Contributor
       has been advised of the possibility of such damages.

    9. Accepting Warranty or Additional Liability. While redistributing
       the Work or Derivative Works thereof, You may choose to offer,
       and charge a fee for, acceptance of support, warranty, indemnity,
       or other liability obligations and/or rights consistent with this
       License. However, in accepting such obligations, You may act only
       on Your own behalf and on Your sole responsibility, not on behalf
       of any other Contributor, and only if You agree to indemnify,
       defend, and hold each Contributor harmless for any liability
       incurred by, or claims asserted against, such Contributor by reason
       of your accepting any such warranty or additional liability.

    END OF TERMS AND CONDITIONS

    APPENDIX: How to apply the Apache License to your work.

       To apply the Apache License to your work, attach the following
       boilerplate notice, with the fields enclosed by brackets "[]"
       replaced with your own identifying information. (Don't include
       the brackets!)  The text should be enclosed in the appropriate
       comment syntax for the file format. We also recommend that a
       file or class name and description of purpose be included on the
       same "printed page" as the copyright notice for easier
       identification within third-party archives.

    Copyright 2022 arkworks contributors

    Licensed under the Apache License, Version 2.0 (the "License");
    you may not use this file except in compliance with the License.
    You may obtain a copy of the License at

    	http://www.apache.org/licenses/LICENSE-2.0

    Unless required by applicable law or agreed to in writing, software
    distributed under the License is distributed on an "AS IS" BASIS,
    WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
    See the License for the specific language governing permissions and
    limitations under the License.

`LICENSE-MIT` (ark-ff 0.4.2): the same text as `LICENSE-MIT` (ark-bls12-381 0.5.0) above.

## ark-ff 0.5.0

License: MIT OR Apache-2.0. Source: https://github.com/arkworks-rs/algebra
Authors: arkworks contributors.

`LICENSE-APACHE` (ark-ff 0.5.0): the same text as `LICENSE-APACHE` (ark-bls12-381 0.5.0) above.

`LICENSE-MIT` (ark-ff 0.5.0): the same text as `LICENSE-MIT` (ark-bls12-381 0.5.0) above.

## ark-poly 0.5.0

License: MIT OR Apache-2.0. Source: https://github.com/arkworks-rs/algebra
Authors: arkworks contributors.

`LICENSE-APACHE` (ark-poly 0.5.0): the same text as `LICENSE-APACHE` (ark-bls12-381 0.5.0) above.

`LICENSE-MIT` (ark-poly 0.5.0): the same text as `LICENSE-MIT` (ark-bls12-381 0.5.0) above.

## ark-serialize 0.4.2

License: MIT/Apache-2.0. Source: https://github.com/arkworks-rs/algebra
Authors: arkworks contributors.

`LICENSE-APACHE` (ark-serialize 0.4.2): the same text as `LICENSE-APACHE` (ark-ff 0.4.2) above.

`LICENSE-MIT` (ark-serialize 0.4.2): the same text as `LICENSE-MIT` (ark-bls12-381 0.5.0) above.

## ark-serialize 0.5.0

License: MIT OR Apache-2.0. Source: https://github.com/arkworks-rs/algebra
Authors: arkworks contributors.

`LICENSE-APACHE` (ark-serialize 0.5.0): the same text as `LICENSE-APACHE` (ark-bls12-381 0.5.0) above.

`LICENSE-MIT` (ark-serialize 0.5.0): the same text as `LICENSE-MIT` (ark-bls12-381 0.5.0) above.

## ark-std 0.4.0

License: MIT/Apache-2.0. Source: https://github.com/arkworks-rs/std
Authors: arkworks contributors.

`LICENSE-APACHE` (ark-std 0.4.0): the same text as `LICENSE-APACHE` (addr2line 0.25.1) above.

`LICENSE-MIT` (ark-std 0.4.0): the same text as `LICENSE-MIT` (ark-bls12-381 0.5.0) above.

## ark-std 0.5.0

License: MIT/Apache-2.0. Source: https://github.com/arkworks-rs/std
Authors: arkworks contributors.

`LICENSE-APACHE` (ark-std 0.5.0): the same text as `LICENSE-APACHE` (addr2line 0.25.1) above.

`LICENSE-MIT` (ark-std 0.5.0): the same text as `LICENSE-MIT` (ark-bls12-381 0.5.0) above.

## arrayref 0.3.9

License: BSD-2-Clause. Source: https://github.com/droundy/arrayref
Authors: David Roundy <roundyd@physics.oregonstate.edu>.

`LICENSE` (arrayref 0.3.9):

    Copyright (c) 2015 David Roundy <roundyd@physics.oregonstate.edu>
    All rights reserved.

    Redistribution and use in source and binary forms, with or without
    modification, are permitted provided that the following conditions are
    met:

    1. Redistributions of source code must retain the above copyright
       notice, this list of conditions and the following disclaimer.

    2. Redistributions in binary form must reproduce the above copyright
       notice, this list of conditions and the following disclaimer in the
       documentation and/or other materials provided with the
       distribution.

    THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS
    "AS IS" AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT
    LIMITED TO, THE IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR
    A PARTICULAR PURPOSE ARE DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT
    HOLDER OR CONTRIBUTORS BE LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL,
    SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT
    LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES; LOSS OF USE,
    DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON ANY
    THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT
    (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE
    OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.

## arrayvec 0.7.8

License: MIT OR Apache-2.0. Source: https://github.com/bluss/arrayvec
Authors: bluss.

`LICENSE-APACHE` (arrayvec 0.7.8): the same text as `LICENSE-APACHE` (addr2line 0.25.1) above.

`LICENSE-MIT` (arrayvec 0.7.8):

    Copyright (c) Ulrik Sverdrup "bluss" 2015-2023

    Permission is hereby granted, free of charge, to any
    person obtaining a copy of this software and associated
    documentation files (the "Software"), to deal in the
    Software without restriction, including without
    limitation the rights to use, copy, modify, merge,
    publish, distribute, sublicense, and/or sell copies of
    the Software, and to permit persons to whom the Software
    is furnished to do so, subject to the following
    conditions:

    The above copyright notice and this permission notice
    shall be included in all copies or substantial portions
    of the Software.

    THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF
    ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED
    TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A
    PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT
    SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
    CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
    OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR
    IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
    DEALINGS IN THE SOFTWARE.

## aurora-engine-modexp 1.3.0

License: CC0-1.0. Source: https://github.com/aurora-is-near/aurora-engine
Authors: Aurora Labs <hello@aurora.dev>.

Its license asks for no notice.

## backtrace 0.3.76

License: MIT OR Apache-2.0. Source: https://github.com/rust-lang/backtrace-rs
Authors: The Rust Project Developers.

`LICENSE-APACHE` (backtrace 0.3.76): the same text as `LICENSE-APACHE` (addr2line 0.25.1) above.

`LICENSE-MIT` (backtrace 0.3.76):

    Copyright (c) 2014 Alex Crichton

    Permission is hereby granted, free of charge, to any
    person obtaining a copy of this software and associated
    documentation files (the "Software"), to deal in the
    Software without restriction, including without
    limitation the rights to use, copy, modify, merge,
    publish, distribute, sublicense, and/or sell copies of
    the Software, and to permit persons to whom the Software
    is furnished to do so, subject to the following
    conditions:

    The above copyright notice and this permission notice
    shall be included in all copies or substantial portions
    of the Software.

    THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF
    ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED
    TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A
    PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT
    SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
    CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
    OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR
    IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
    DEALINGS IN THE SOFTWARE.

## backtrace 0.3.77

License: MIT OR Apache-2.0. Source: https://github.com/rust-lang/backtrace-rs
Authors: The Rust Project Developers.

`LICENSE-APACHE` (backtrace 0.3.77): the same text as `LICENSE-APACHE` (addr2line 0.25.1) above.

`LICENSE-MIT` (backtrace 0.3.77): the same text as `LICENSE-MIT` (backtrace 0.3.76) above.

## base16ct 0.2.0

License: Apache-2.0 OR MIT. Source: https://github.com/RustCrypto/formats/tree/master/base16ct
Authors: RustCrypto Developers.

`LICENSE-APACHE` (base16ct 0.2.0):

                                  Apache License
                            Version 2.0, January 2004
                         http://www.apache.org/licenses/

    TERMS AND CONDITIONS FOR USE, REPRODUCTION, AND DISTRIBUTION

    1. Definitions.

       "License" shall mean the terms and conditions for use, reproduction,
       and distribution as defined by Sections 1 through 9 of this document.

       "Licensor" shall mean the copyright owner or entity authorized by
       the copyright owner that is granting the License.

       "Legal Entity" shall mean the union of the acting entity and all
       other entities that control, are controlled by, or are under common
       control with that entity. For the purposes of this definition,
       "control" means (i) the power, direct or indirect, to cause the
       direction or management of such entity, whether by contract or
       otherwise, or (ii) ownership of fifty percent (50%) or more of the
       outstanding shares, or (iii) beneficial ownership of such entity.

       "You" (or "Your") shall mean an individual or Legal Entity
       exercising permissions granted by this License.

       "Source" form shall mean the preferred form for making modifications,
       including but not limited to software source code, documentation
       source, and configuration files.

       "Object" form shall mean any form resulting from mechanical
       transformation or translation of a Source form, including but
       not limited to compiled object code, generated documentation,
       and conversions to other media types.

       "Work" shall mean the work of authorship, whether in Source or
       Object form, made available under the License, as indicated by a
       copyright notice that is included in or attached to the work
       (an example is provided in the Appendix below).

       "Derivative Works" shall mean any work, whether in Source or Object
       form, that is based on (or derived from) the Work and for which the
       editorial revisions, annotations, elaborations, or other modifications
       represent, as a whole, an original work of authorship. For the purposes
       of this License, Derivative Works shall not include works that remain
       separable from, or merely link (or bind by name) to the interfaces of,
       the Work and Derivative Works thereof.

       "Contribution" shall mean any work of authorship, including
       the original version of the Work and any modifications or additions
       to that Work or Derivative Works thereof, that is intentionally
       submitted to Licensor for inclusion in the Work by the copyright owner
       or by an individual or Legal Entity authorized to submit on behalf of
       the copyright owner. For the purposes of this definition, "submitted"
       means any form of electronic, verbal, or written communication sent
       to the Licensor or its representatives, including but not limited to
       communication on electronic mailing lists, source code control systems,
       and issue tracking systems that are managed by, or on behalf of, the
       Licensor for the purpose of discussing and improving the Work, but
       excluding communication that is conspicuously marked or otherwise
       designated in writing by the copyright owner as "Not a Contribution."

       "Contributor" shall mean Licensor and any individual or Legal Entity
       on behalf of whom a Contribution has been received by Licensor and
       subsequently incorporated within the Work.

    2. Grant of Copyright License. Subject to the terms and conditions of
       this License, each Contributor hereby grants to You a perpetual,
       worldwide, non-exclusive, no-charge, royalty-free, irrevocable
       copyright license to reproduce, prepare Derivative Works of,
       publicly display, publicly perform, sublicense, and distribute the
       Work and such Derivative Works in Source or Object form.

    3. Grant of Patent License. Subject to the terms and conditions of
       this License, each Contributor hereby grants to You a perpetual,
       worldwide, non-exclusive, no-charge, royalty-free, irrevocable
       (except as stated in this section) patent license to make, have made,
       use, offer to sell, sell, import, and otherwise transfer the Work,
       where such license applies only to those patent claims licensable
       by such Contributor that are necessarily infringed by their
       Contribution(s) alone or by combination of their Contribution(s)
       with the Work to which such Contribution(s) was submitted. If You
       institute patent litigation against any entity (including a
       cross-claim or counterclaim in a lawsuit) alleging that the Work
       or a Contribution incorporated within the Work constitutes direct
       or contributory patent infringement, then any patent licenses
       granted to You under this License for that Work shall terminate
       as of the date such litigation is filed.

    4. Redistribution. You may reproduce and distribute copies of the
       Work or Derivative Works thereof in any medium, with or without
       modifications, and in Source or Object form, provided that You
       meet the following conditions:

       (a) You must give any other recipients of the Work or
           Derivative Works a copy of this License; and

       (b) You must cause any modified files to carry prominent notices
           stating that You changed the files; and

       (c) You must retain, in the Source form of any Derivative Works
           that You distribute, all copyright, patent, trademark, and
           attribution notices from the Source form of the Work,
           excluding those notices that do not pertain to any part of
           the Derivative Works; and

       (d) If the Work includes a "NOTICE" text file as part of its
           distribution, then any Derivative Works that You distribute must
           include a readable copy of the attribution notices contained
           within such NOTICE file, excluding those notices that do not
           pertain to any part of the Derivative Works, in at least one
           of the following places: within a NOTICE text file distributed
           as part of the Derivative Works; within the Source form or
           documentation, if provided along with the Derivative Works; or,
           within a display generated by the Derivative Works, if and
           wherever such third-party notices normally appear. The contents
           of the NOTICE file are for informational purposes only and
           do not modify the License. You may add Your own attribution
           notices within Derivative Works that You distribute, alongside
           or as an addendum to the NOTICE text from the Work, provided
           that such additional attribution notices cannot be construed
           as modifying the License.

       You may add Your own copyright statement to Your modifications and
       may provide additional or different license terms and conditions
       for use, reproduction, or distribution of Your modifications, or
       for any such Derivative Works as a whole, provided Your use,
       reproduction, and distribution of the Work otherwise complies with
       the conditions stated in this License.

    5. Submission of Contributions. Unless You explicitly state otherwise,
       any Contribution intentionally submitted for inclusion in the Work
       by You to the Licensor shall be under the terms and conditions of
       this License, without any additional terms or conditions.
       Notwithstanding the above, nothing herein shall supersede or modify
       the terms of any separate license agreement you may have executed
       with Licensor regarding such Contributions.

    6. Trademarks. This License does not grant permission to use the trade
       names, trademarks, service marks, or product names of the Licensor,
       except as required for reasonable and customary use in describing the
       origin of the Work and reproducing the content of the NOTICE file.

    7. Disclaimer of Warranty. Unless required by applicable law or
       agreed to in writing, Licensor provides the Work (and each
       Contributor provides its Contributions) on an "AS IS" BASIS,
       WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or
       implied, including, without limitation, any warranties or conditions
       of TITLE, NON-INFRINGEMENT, MERCHANTABILITY, or FITNESS FOR A
       PARTICULAR PURPOSE. You are solely responsible for determining the
       appropriateness of using or redistributing the Work and assume any
       risks associated with Your exercise of permissions under this License.

    8. Limitation of Liability. In no event and under no legal theory,
       whether in tort (including negligence), contract, or otherwise,
       unless required by applicable law (such as deliberate and grossly
       negligent acts) or agreed to in writing, shall any Contributor be
       liable to You for damages, including any direct, indirect, special,
       incidental, or consequential damages of any character arising as a
       result of this License or out of the use or inability to use the
       Work (including but not limited to damages for loss of goodwill,
       work stoppage, computer failure or malfunction, or any and all
       other commercial damages or losses), even if such Contributor
       has been advised of the possibility of such damages.

    9. Accepting Warranty or Additional Liability. While redistributing
       the Work or Derivative Works thereof, You may choose to offer,
       and charge a fee for, acceptance of support, warranty, indemnity,
       or other liability obligations and/or rights consistent with this
       License. However, in accepting such obligations, You may act only
       on Your own behalf and on Your sole responsibility, not on behalf
       of any other Contributor, and only if You agree to indemnify,
       defend, and hold each Contributor harmless for any liability
       incurred by, or claims asserted against, such Contributor by reason
       of your accepting any such warranty or additional liability.

    END OF TERMS AND CONDITIONS

    APPENDIX: How to apply the Apache License to your work.

       To apply the Apache License to your work, attach the following
       boilerplate notice, with the fields enclosed by brackets "[]"
       replaced with your own identifying information. (Don't include
       the brackets!)  The text should be enclosed in the appropriate
       comment syntax for the file format. We also recommend that a
       file or class name and description of purpose be included on the
       same "printed page" as the copyright notice for easier
       identification within third-party archives.

    Copyright [yyyy] [name of copyright owner]

    Licensed under the Apache License, Version 2.0 (the "License");
    you may not use this file except in compliance with the License.
    You may obtain a copy of the License at

       http://www.apache.org/licenses/LICENSE-2.0

    Unless required by applicable law or agreed to in writing, software
    distributed under the License is distributed on an "AS IS" BASIS,
    WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
    See the License for the specific language governing permissions and
    limitations under the License.

`LICENSE-MIT` (base16ct 0.2.0):

    Copyright (c) 2014 Steve "Sc00bz" Thomas (steve at tobtu dot com)
    Copyright (c) 2022 The RustCrypto Project Developers

    Permission is hereby granted, free of charge, to any
    person obtaining a copy of this software and associated
    documentation files (the "Software"), to deal in the
    Software without restriction, including without
    limitation the rights to use, copy, modify, merge,
    publish, distribute, sublicense, and/or sell copies of
    the Software, and to permit persons to whom the Software
    is furnished to do so, subject to the following
    conditions:

    The above copyright notice and this permission notice
    shall be included in all copies or substantial portions
    of the Software.

    THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF
    ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED
    TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A
    PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT
    SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
    CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
    OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR
    IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
    DEALINGS IN THE SOFTWARE.

## base64 0.22.1

License: MIT OR Apache-2.0. Source: https://github.com/marshallpierce/rust-base64
Authors: Marshall Pierce <marshall@mpierce.org>.

`LICENSE-APACHE` (base64 0.22.1): the same text as `LICENSE-APACHE` (addr2line 0.25.1) above.

`LICENSE-MIT` (base64 0.22.1):

    The MIT License (MIT)

    Copyright (c) 2015 Alice Maz

    Permission is hereby granted, free of charge, to any person obtaining a copy
    of this software and associated documentation files (the "Software"), to deal
    in the Software without restriction, including without limitation the rights
    to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
    copies of the Software, and to permit persons to whom the Software is
    furnished to do so, subject to the following conditions:

    The above copyright notice and this permission notice shall be included in
    all copies or substantial portions of the Software.

    THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
    IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
    FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
    AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
    LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
    OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN
    THE SOFTWARE.

## bincode 1.3.3

License: MIT. Source: https://github.com/servo/bincode
Authors: Ty Overby <ty@pre-alpha.com>, Francesco Mazzoli <f@mazzo.li>, David Tolnay <dtolnay@gmail.com>, Zoey Riordan <zoey@dos.cafe>.

`LICENSE.md` (bincode 1.3.3):

    The MIT License (MIT)

    Copyright (c) 2014 Ty Overby

    Permission is hereby granted, free of charge, to any person obtaining a copy
    of this software and associated documentation files (the "Software"), to deal
    in the Software without restriction, including without limitation the rights
    to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
    copies of the Software, and to permit persons to whom the Software is
    furnished to do so, subject to the following conditions:

    The above copyright notice and this permission notice shall be included in all
    copies or substantial portions of the Software.

    THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
    IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
    FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
    AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
    LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
    OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
    SOFTWARE.

## bitcode 0.6.9

License: MIT OR Apache-2.0. Source: https://github.com/SoftbearStudios/bitcode
Authors: Cai Bear, Finn Bear.

`LICENSE-MIT` (bitcode 0.6.9):

    The MIT License (MIT)

    Copyright (c) 2023 Softbear Games

    Permission is hereby granted, free of charge, to any person obtaining a copy of this software and associated documentation files (the "Software"), to deal in the Software without restriction, including without limitation the rights to use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of the Software, and to permit persons to whom the Software is furnished to do so, subject to the following conditions:

    The above copyright notice and this permission notice shall be included in all copies or substantial portions of the Software.

    THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

## bitflags 2.13.2

License: MIT OR Apache-2.0. Source: https://github.com/bitflags/bitflags
Authors: The Rust Project Developers.

`LICENSE-APACHE` (bitflags 2.13.2): the same text as `LICENSE-APACHE` (addr2line 0.25.1) above.

`LICENSE-MIT` (bitflags 2.13.2):

    Copyright (c) 2014 The Rust Project Developers

    Permission is hereby granted, free of charge, to any
    person obtaining a copy of this software and associated
    documentation files (the "Software"), to deal in the
    Software without restriction, including without
    limitation the rights to use, copy, modify, merge,
    publish, distribute, sublicense, and/or sell copies of
    the Software, and to permit persons to whom the Software
    is furnished to do so, subject to the following
    conditions:

    The above copyright notice and this permission notice
    shall be included in all copies or substantial portions
    of the Software.

    THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF
    ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED
    TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A
    PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT
    SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
    CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
    OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR
    IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
    DEALINGS IN THE SOFTWARE.

## bitvec 1.1.1

License: MIT. Source: https://github.com/bitvecto-rs/bitvec

`LICENSE.txt` (bitvec 1.1.1):

    MIT License

    Copyright (c) 2018 myrrlyn (Alexander Payne)

    Permission is hereby granted, free of charge, to any person obtaining a copy
    of this software and associated documentation files (the "Software"), to deal
    in the Software without restriction, including without limitation the rights
    to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
    copies of the Software, and to permit persons to whom the Software is
    furnished to do so, subject to the following conditions:

    The above copyright notice and this permission notice shall be included in all
    copies or substantial portions of the Software.

    THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
    IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
    FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
    AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
    LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
    OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
    SOFTWARE.

## blake2 0.10.6

License: MIT OR Apache-2.0. Source: https://github.com/RustCrypto/hashes
Authors: RustCrypto Developers.

`LICENSE-APACHE` (blake2 0.10.6): the same text as `LICENSE-APACHE` (base16ct 0.2.0) above.

`LICENSE-MIT` (blake2 0.10.6):

    Copyright (c) 2015-2016 The blake2-rfc Developers, Cesar Barros
    Copyright (c) 2017 Artyom Pavlov

    Permission is hereby granted, free of charge, to any
    person obtaining a copy of this software and associated
    documentation files (the "Software"), to deal in the
    Software without restriction, including without
    limitation the rights to use, copy, modify, merge,
    publish, distribute, sublicense, and/or sell copies of
    the Software, and to permit persons to whom the Software
    is furnished to do so, subject to the following
    conditions:

    The above copyright notice and this permission notice
    shall be included in all copies or substantial portions
    of the Software.

    THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF
    ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED
    TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A
    PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT
    SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
    CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
    OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR
    IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
    DEALINGS IN THE SOFTWARE.

## blake2b_simd 1.0.5

License: MIT. Source: https://github.com/oconnor663/blake2_simd
Authors: Jack O'Connor.

Its package carries no license file: the standard text of its license, under its authors.

MIT (blake2b_simd 1.0.5):

    MIT License

    Copyright (c) Jack O'Connor

    Permission is hereby granted, free of charge, to any person obtaining a copy of this software and associated documentation files (the "Software"), to deal in the Software without restriction, including without limitation the rights to use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of the Software, and to permit persons to whom the Software is furnished to do so, subject to the following conditions:

    The above copyright notice and this permission notice shall be included in all copies or substantial portions of the Software.

    THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

## blake3 1.8.7

License: CC0-1.0 OR Apache-2.0 OR Apache-2.0 WITH LLVM-exception. Source: https://github.com/BLAKE3-team/BLAKE3
Authors: Jack O'Connor <oconnor663@gmail.com>, Samuel Neves.

`LICENSE_A2` (blake3 1.8.7):

                                     Apache License
                               Version 2.0, January 2004
                            http://www.apache.org/licenses/

       TERMS AND CONDITIONS FOR USE, REPRODUCTION, AND DISTRIBUTION

       1. Definitions.

          "License" shall mean the terms and conditions for use, reproduction,
          and distribution as defined by Sections 1 through 9 of this document.

          "Licensor" shall mean the copyright owner or entity authorized by
          the copyright owner that is granting the License.

          "Legal Entity" shall mean the union of the acting entity and all
          other entities that control, are controlled by, or are under common
          control with that entity. For the purposes of this definition,
          "control" means (i) the power, direct or indirect, to cause the
          direction or management of such entity, whether by contract or
          otherwise, or (ii) ownership of fifty percent (50%) or more of the
          outstanding shares, or (iii) beneficial ownership of such entity.

          "You" (or "Your") shall mean an individual or Legal Entity
          exercising permissions granted by this License.

          "Source" form shall mean the preferred form for making modifications,
          including but not limited to software source code, documentation
          source, and configuration files.

          "Object" form shall mean any form resulting from mechanical
          transformation or translation of a Source form, including but
          not limited to compiled object code, generated documentation,
          and conversions to other media types.

          "Work" shall mean the work of authorship, whether in Source or
          Object form, made available under the License, as indicated by a
          copyright notice that is included in or attached to the work
          (an example is provided in the Appendix below).

          "Derivative Works" shall mean any work, whether in Source or Object
          form, that is based on (or derived from) the Work and for which the
          editorial revisions, annotations, elaborations, or other modifications
          represent, as a whole, an original work of authorship. For the purposes
          of this License, Derivative Works shall not include works that remain
          separable from, or merely link (or bind by name) to the interfaces of,
          the Work and Derivative Works thereof.

          "Contribution" shall mean any work of authorship, including
          the original version of the Work and any modifications or additions
          to that Work or Derivative Works thereof, that is intentionally
          submitted to Licensor for inclusion in the Work by the copyright owner
          or by an individual or Legal Entity authorized to submit on behalf of
          the copyright owner. For the purposes of this definition, "submitted"
          means any form of electronic, verbal, or written communication sent
          to the Licensor or its representatives, including but not limited to
          communication on electronic mailing lists, source code control systems,
          and issue tracking systems that are managed by, or on behalf of, the
          Licensor for the purpose of discussing and improving the Work, but
          excluding communication that is conspicuously marked or otherwise
          designated in writing by the copyright owner as "Not a Contribution."

          "Contributor" shall mean Licensor and any individual or Legal Entity
          on behalf of whom a Contribution has been received by Licensor and
          subsequently incorporated within the Work.

       2. Grant of Copyright License. Subject to the terms and conditions of
          this License, each Contributor hereby grants to You a perpetual,
          worldwide, non-exclusive, no-charge, royalty-free, irrevocable
          copyright license to reproduce, prepare Derivative Works of,
          publicly display, publicly perform, sublicense, and distribute the
          Work and such Derivative Works in Source or Object form.

       3. Grant of Patent License. Subject to the terms and conditions of
          this License, each Contributor hereby grants to You a perpetual,
          worldwide, non-exclusive, no-charge, royalty-free, irrevocable
          (except as stated in this section) patent license to make, have made,
          use, offer to sell, sell, import, and otherwise transfer the Work,
          where such license applies only to those patent claims licensable
          by such Contributor that are necessarily infringed by their
          Contribution(s) alone or by combination of their Contribution(s)
          with the Work to which such Contribution(s) was submitted. If You
          institute patent litigation against any entity (including a
          cross-claim or counterclaim in a lawsuit) alleging that the Work
          or a Contribution incorporated within the Work constitutes direct
          or contributory patent infringement, then any patent licenses
          granted to You under this License for that Work shall terminate
          as of the date such litigation is filed.

       4. Redistribution. You may reproduce and distribute copies of the
          Work or Derivative Works thereof in any medium, with or without
          modifications, and in Source or Object form, provided that You
          meet the following conditions:

          (a) You must give any other recipients of the Work or
              Derivative Works a copy of this License; and

          (b) You must cause any modified files to carry prominent notices
              stating that You changed the files; and

          (c) You must retain, in the Source form of any Derivative Works
              that You distribute, all copyright, patent, trademark, and
              attribution notices from the Source form of the Work,
              excluding those notices that do not pertain to any part of
              the Derivative Works; and

          (d) If the Work includes a "NOTICE" text file as part of its
              distribution, then any Derivative Works that You distribute must
              include a readable copy of the attribution notices contained
              within such NOTICE file, excluding those notices that do not
              pertain to any part of the Derivative Works, in at least one
              of the following places: within a NOTICE text file distributed
              as part of the Derivative Works; within the Source form or
              documentation, if provided along with the Derivative Works; or,
              within a display generated by the Derivative Works, if and
              wherever such third-party notices normally appear. The contents
              of the NOTICE file are for informational purposes only and
              do not modify the License. You may add Your own attribution
              notices within Derivative Works that You distribute, alongside
              or as an addendum to the NOTICE text from the Work, provided
              that such additional attribution notices cannot be construed
              as modifying the License.

          You may add Your own copyright statement to Your modifications and
          may provide additional or different license terms and conditions
          for use, reproduction, or distribution of Your modifications, or
          for any such Derivative Works as a whole, provided Your use,
          reproduction, and distribution of the Work otherwise complies with
          the conditions stated in this License.

       5. Submission of Contributions. Unless You explicitly state otherwise,
          any Contribution intentionally submitted for inclusion in the Work
          by You to the Licensor shall be under the terms and conditions of
          this License, without any additional terms or conditions.
          Notwithstanding the above, nothing herein shall supersede or modify
          the terms of any separate license agreement you may have executed
          with Licensor regarding such Contributions.

       6. Trademarks. This License does not grant permission to use the trade
          names, trademarks, service marks, or product names of the Licensor,
          except as required for reasonable and customary use in describing the
          origin of the Work and reproducing the content of the NOTICE file.

       7. Disclaimer of Warranty. Unless required by applicable law or
          agreed to in writing, Licensor provides the Work (and each
          Contributor provides its Contributions) on an "AS IS" BASIS,
          WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or
          implied, including, without limitation, any warranties or conditions
          of TITLE, NON-INFRINGEMENT, MERCHANTABILITY, or FITNESS FOR A
          PARTICULAR PURPOSE. You are solely responsible for determining the
          appropriateness of using or redistributing the Work and assume any
          risks associated with Your exercise of permissions under this License.

       8. Limitation of Liability. In no event and under no legal theory,
          whether in tort (including negligence), contract, or otherwise,
          unless required by applicable law (such as deliberate and grossly
          negligent acts) or agreed to in writing, shall any Contributor be
          liable to You for damages, including any direct, indirect, special,
          incidental, or consequential damages of any character arising as a
          result of this License or out of the use or inability to use the
          Work (including but not limited to damages for loss of goodwill,
          work stoppage, computer failure or malfunction, or any and all
          other commercial damages or losses), even if such Contributor
          has been advised of the possibility of such damages.

       9. Accepting Warranty or Additional Liability. While redistributing
          the Work or Derivative Works thereof, You may choose to offer,
          and charge a fee for, acceptance of support, warranty, indemnity,
          or other liability obligations and/or rights consistent with this
          License. However, in accepting such obligations, You may act only
          on Your own behalf and on Your sole responsibility, not on behalf
          of any other Contributor, and only if You agree to indemnify,
          defend, and hold each Contributor harmless for any liability
          incurred by, or claims asserted against, such Contributor by reason
          of your accepting any such warranty or additional liability.

       END OF TERMS AND CONDITIONS

       APPENDIX: How to apply the Apache License to your work.

          To apply the Apache License to your work, attach the following
          boilerplate notice, with the fields enclosed by brackets "[]"
          replaced with your own identifying information. (Don't include
          the brackets!)  The text should be enclosed in the appropriate
          comment syntax for the file format. We also recommend that a
          file or class name and description of purpose be included on the
          same "printed page" as the copyright notice for easier
          identification within third-party archives.

       Copyright 2019 Jack O'Connor and Samuel Neves

       Licensed under the Apache License, Version 2.0 (the "License");
       you may not use this file except in compliance with the License.
       You may obtain a copy of the License at

           http://www.apache.org/licenses/LICENSE-2.0

       Unless required by applicable law or agreed to in writing, software
       distributed under the License is distributed on an "AS IS" BASIS,
       WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
       See the License for the specific language governing permissions and
       limitations under the License.

`LICENSE_A2LLVM` (blake3 1.8.7):

                                     Apache License
                               Version 2.0, January 2004
                            http://www.apache.org/licenses/

        TERMS AND CONDITIONS FOR USE, REPRODUCTION, AND DISTRIBUTION

        1. Definitions.

          "License" shall mean the terms and conditions for use, reproduction,
          and distribution as defined by Sections 1 through 9 of this document.

          "Licensor" shall mean the copyright owner or entity authorized by
          the copyright owner that is granting the License.

          "Legal Entity" shall mean the union of the acting entity and all
          other entities that control, are controlled by, or are under common
          control with that entity. For the purposes of this definition,
          "control" means (i) the power, direct or indirect, to cause the
          direction or management of such entity, whether by contract or
          otherwise, or (ii) ownership of fifty percent (50%) or more of the
          outstanding shares, or (iii) beneficial ownership of such entity.

          "You" (or "Your") shall mean an individual or Legal Entity
          exercising permissions granted by this License.

          "Source" form shall mean the preferred form for making modifications,
          including but not limited to software source code, documentation
          source, and configuration files.

          "Object" form shall mean any form resulting from mechanical
          transformation or translation of a Source form, including but
          not limited to compiled object code, generated documentation,
          and conversions to other media types.

          "Work" shall mean the work of authorship, whether in Source or
          Object form, made available under the License, as indicated by a
          copyright notice that is included in or attached to the work
          (an example is provided in the Appendix below).

          "Derivative Works" shall mean any work, whether in Source or Object
          form, that is based on (or derived from) the Work and for which the
          editorial revisions, annotations, elaborations, or other modifications
          represent, as a whole, an original work of authorship. For the purposes
          of this License, Derivative Works shall not include works that remain
          separable from, or merely link (or bind by name) to the interfaces of,
          the Work and Derivative Works thereof.

          "Contribution" shall mean any work of authorship, including
          the original version of the Work and any modifications or additions
          to that Work or Derivative Works thereof, that is intentionally
          submitted to Licensor for inclusion in the Work by the copyright owner
          or by an individual or Legal Entity authorized to submit on behalf of
          the copyright owner. For the purposes of this definition, "submitted"
          means any form of electronic, verbal, or written communication sent
          to the Licensor or its representatives, including but not limited to
          communication on electronic mailing lists, source code control systems,
          and issue tracking systems that are managed by, or on behalf of, the
          Licensor for the purpose of discussing and improving the Work, but
          excluding communication that is conspicuously marked or otherwise
          designated in writing by the copyright owner as "Not a Contribution."

          "Contributor" shall mean Licensor and any individual or Legal Entity
          on behalf of whom a Contribution has been received by Licensor and
          subsequently incorporated within the Work.

        2. Grant of Copyright License. Subject to the terms and conditions of
          this License, each Contributor hereby grants to You a perpetual,
          worldwide, non-exclusive, no-charge, royalty-free, irrevocable
          copyright license to reproduce, prepare Derivative Works of,
          publicly display, publicly perform, sublicense, and distribute the
          Work and such Derivative Works in Source or Object form.

        3. Grant of Patent License. Subject to the terms and conditions of
          this License, each Contributor hereby grants to You a perpetual,
          worldwide, non-exclusive, no-charge, royalty-free, irrevocable
          (except as stated in this section) patent license to make, have made,
          use, offer to sell, sell, import, and otherwise transfer the Work,
          where such license applies only to those patent claims licensable
          by such Contributor that are necessarily infringed by their
          Contribution(s) alone or by combination of their Contribution(s)
          with the Work to which such Contribution(s) was submitted. If You
          institute patent litigation against any entity (including a
          cross-claim or counterclaim in a lawsuit) alleging that the Work
          or a Contribution incorporated within the Work constitutes direct
          or contributory patent infringement, then any patent licenses
          granted to You under this License for that Work shall terminate
          as of the date such litigation is filed.

        4. Redistribution. You may reproduce and distribute copies of the
          Work or Derivative Works thereof in any medium, with or without
          modifications, and in Source or Object form, provided that You
          meet the following conditions:

          (a) You must give any other recipients of the Work or
              Derivative Works a copy of this License; and

          (b) You must cause any modified files to carry prominent notices
              stating that You changed the files; and

          (c) You must retain, in the Source form of any Derivative Works
              that You distribute, all copyright, patent, trademark, and
              attribution notices from the Source form of the Work,
              excluding those notices that do not pertain to any part of
              the Derivative Works; and

          (d) If the Work includes a "NOTICE" text file as part of its
              distribution, then any Derivative Works that You distribute must
              include a readable copy of the attribution notices contained
              within such NOTICE file, excluding those notices that do not
              pertain to any part of the Derivative Works, in at least one
              of the following places: within a NOTICE text file distributed
              as part of the Derivative Works; within the Source form or
              documentation, if provided along with the Derivative Works; or,
              within a display generated by the Derivative Works, if and
              wherever such third-party notices normally appear. The contents
              of the NOTICE file are for informational purposes only and
              do not modify the License. You may add Your own attribution
              notices within Derivative Works that You distribute, alongside
              or as an addendum to the NOTICE text from the Work, provided
              that such additional attribution notices cannot be construed
              as modifying the License.

          You may add Your own copyright statement to Your modifications and
          may provide additional or different license terms and conditions
          for use, reproduction, or distribution of Your modifications, or
          for any such Derivative Works as a whole, provided Your use,
          reproduction, and distribution of the Work otherwise complies with
          the conditions stated in this License.

        5. Submission of Contributions. Unless You explicitly state otherwise,
          any Contribution intentionally submitted for inclusion in the Work
          by You to the Licensor shall be under the terms and conditions of
          this License, without any additional terms or conditions.
          Notwithstanding the above, nothing herein shall supersede or modify
          the terms of any separate license agreement you may have executed
          with Licensor regarding such Contributions.

        6. Trademarks. This License does not grant permission to use the trade
          names, trademarks, service marks, or product names of the Licensor,
          except as required for reasonable and customary use in describing the
          origin of the Work and reproducing the content of the NOTICE file.

        7. Disclaimer of Warranty. Unless required by applicable law or
          agreed to in writing, Licensor provides the Work (and each
          Contributor provides its Contributions) on an "AS IS" BASIS,
          WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or
          implied, including, without limitation, any warranties or conditions
          of TITLE, NON-INFRINGEMENT, MERCHANTABILITY, or FITNESS FOR A
          PARTICULAR PURPOSE. You are solely responsible for determining the
          appropriateness of using or redistributing the Work and assume any
          risks associated with Your exercise of permissions under this License.

        8. Limitation of Liability. In no event and under no legal theory,
          whether in tort (including negligence), contract, or otherwise,
          unless required by applicable law (such as deliberate and grossly
          negligent acts) or agreed to in writing, shall any Contributor be
          liable to You for damages, including any direct, indirect, special,
          incidental, or consequential damages of any character arising as a
          result of this License or out of the use or inability to use the
          Work (including but not limited to damages for loss of goodwill,
          work stoppage, computer failure or malfunction, or any and all
          other commercial damages or losses), even if such Contributor
          has been advised of the possibility of such damages.

        9. Accepting Warranty or Additional Liability. While redistributing
          the Work or Derivative Works thereof, You may choose to offer,
          and charge a fee for, acceptance of support, warranty, indemnity,
          or other liability obligations and/or rights consistent with this
          License. However, in accepting such obligations, You may act only
          on Your own behalf and on Your sole responsibility, not on behalf
          of any other Contributor, and only if You agree to indemnify,
          defend, and hold each Contributor harmless for any liability
          incurred by, or claims asserted against, such Contributor by reason
          of your accepting any such warranty or additional liability.

        END OF TERMS AND CONDITIONS

        APPENDIX: How to apply the Apache License to your work.

          To apply the Apache License to your work, attach the following
          boilerplate notice, with the fields enclosed by brackets "[]"
          replaced with your own identifying information. (Don't include
          the brackets!)  The text should be enclosed in the appropriate
          comment syntax for the file format. We also recommend that a
          file or class name and description of purpose be included on the
          same "printed page" as the copyright notice for easier
          identification within third-party archives.

        Copyright 2019 Jack O'Connor and Samuel Neves

        Licensed under the Apache License, Version 2.0 (the "License");
        you may not use this file except in compliance with the License.
        You may obtain a copy of the License at

           http://www.apache.org/licenses/LICENSE-2.0

        Unless required by applicable law or agreed to in writing, software
        distributed under the License is distributed on an "AS IS" BASIS,
        WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
        See the License for the specific language governing permissions and
        limitations under the License.


    ---- LLVM Exceptions to the Apache 2.0 License ----

    As an exception, if, as a result of your compiling your source code, portions
    of this Software are embedded into an Object form of such source code, you
    may redistribute such embedded portions in such Object form without complying
    with the conditions of Sections 4(a), 4(b) and 4(d) of the License.

    In addition, if you combine or link compiled forms of this Software with
    software that is licensed under the GPLv2 ("Combined Software") and if a
    court of competent jurisdiction determines that the patent provision (Section
    3), the indemnity provision (Section 9) or other Section of the License
    conflicts with the conditions of the GPLv2, you may retroactively and
    prospectively choose to deem waived or otherwise exclude such Section(s) of
    the License, but only in their entirety and only with respect to the Combined
    Software.

`LICENSE_CC0` (blake3 1.8.7):

    Creative Commons Legal Code

    CC0 1.0 Universal

        CREATIVE COMMONS CORPORATION IS NOT A LAW FIRM AND DOES NOT PROVIDE
        LEGAL SERVICES. DISTRIBUTION OF THIS DOCUMENT DOES NOT CREATE AN
        ATTORNEY-CLIENT RELATIONSHIP. CREATIVE COMMONS PROVIDES THIS
        INFORMATION ON AN "AS-IS" BASIS. CREATIVE COMMONS MAKES NO WARRANTIES
        REGARDING THE USE OF THIS DOCUMENT OR THE INFORMATION OR WORKS
        PROVIDED HEREUNDER, AND DISCLAIMS LIABILITY FOR DAMAGES RESULTING FROM
        THE USE OF THIS DOCUMENT OR THE INFORMATION OR WORKS PROVIDED
        HEREUNDER.

    Statement of Purpose

    The laws of most jurisdictions throughout the world automatically confer
    exclusive Copyright and Related Rights (defined below) upon the creator
    and subsequent owner(s) (each and all, an "owner") of an original work of
    authorship and/or a database (each, a "Work").

    Certain owners wish to permanently relinquish those rights to a Work for
    the purpose of contributing to a commons of creative, cultural and
    scientific works ("Commons") that the public can reliably and without fear
    of later claims of infringement build upon, modify, incorporate in other
    works, reuse and redistribute as freely as possible in any form whatsoever
    and for any purposes, including without limitation commercial purposes.
    These owners may contribute to the Commons to promote the ideal of a free
    culture and the further production of creative, cultural and scientific
    works, or to gain reputation or greater distribution for their Work in
    part through the use and efforts of others.

    For these and/or other purposes and motivations, and without any
    expectation of additional consideration or compensation, the person
    associating CC0 with a Work (the "Affirmer"), to the extent that he or she
    is an owner of Copyright and Related Rights in the Work, voluntarily
    elects to apply CC0 to the Work and publicly distribute the Work under its
    terms, with knowledge of his or her Copyright and Related Rights in the
    Work and the meaning and intended legal effect of CC0 on those rights.

    1. Copyright and Related Rights. A Work made available under CC0 may be
    protected by copyright and related or neighboring rights ("Copyright and
    Related Rights"). Copyright and Related Rights include, but are not
    limited to, the following:

      i. the right to reproduce, adapt, distribute, perform, display,
         communicate, and translate a Work;
     ii. moral rights retained by the original author(s) and/or performer(s);
    iii. publicity and privacy rights pertaining to a person's image or
         likeness depicted in a Work;
     iv. rights protecting against unfair competition in regards to a Work,
         subject to the limitations in paragraph 4(a), below;
      v. rights protecting the extraction, dissemination, use and reuse of data
         in a Work;
     vi. database rights (such as those arising under Directive 96/9/EC of the
         European Parliament and of the Council of 11 March 1996 on the legal
         protection of databases, and under any national implementation
         thereof, including any amended or successor version of such
         directive); and
    vii. other similar, equivalent or corresponding rights throughout the
         world based on applicable law or treaty, and any national
         implementations thereof.

    2. Waiver. To the greatest extent permitted by, but not in contravention
    of, applicable law, Affirmer hereby overtly, fully, permanently,
    irrevocably and unconditionally waives, abandons, and surrenders all of
    Affirmer's Copyright and Related Rights and associated claims and causes
    of action, whether now known or unknown (including existing as well as
    future claims and causes of action), in the Work (i) in all territories
    worldwide, (ii) for the maximum duration provided by applicable law or
    treaty (including future time extensions), (iii) in any current or future
    medium and for any number of copies, and (iv) for any purpose whatsoever,
    including without limitation commercial, advertising or promotional
    purposes (the "Waiver"). Affirmer makes the Waiver for the benefit of each
    member of the public at large and to the detriment of Affirmer's heirs and
    successors, fully intending that such Waiver shall not be subject to
    revocation, rescission, cancellation, termination, or any other legal or
    equitable action to disrupt the quiet enjoyment of the Work by the public
    as contemplated by Affirmer's express Statement of Purpose.

    3. Public License Fallback. Should any part of the Waiver for any reason
    be judged legally invalid or ineffective under applicable law, then the
    Waiver shall be preserved to the maximum extent permitted taking into
    account Affirmer's express Statement of Purpose. In addition, to the
    extent the Waiver is so judged Affirmer hereby grants to each affected
    person a royalty-free, non transferable, non sublicensable, non exclusive,
    irrevocable and unconditional license to exercise Affirmer's Copyright and
    Related Rights in the Work (i) in all territories worldwide, (ii) for the
    maximum duration provided by applicable law or treaty (including future
    time extensions), (iii) in any current or future medium and for any number
    of copies, and (iv) for any purpose whatsoever, including without
    limitation commercial, advertising or promotional purposes (the
    "License"). The License shall be deemed effective as of the date CC0 was
    applied by Affirmer to the Work. Should any part of the License for any
    reason be judged legally invalid or ineffective under applicable law, such
    partial invalidity or ineffectiveness shall not invalidate the remainder
    of the License, and in such case Affirmer hereby affirms that he or she
    will not (i) exercise any of his or her remaining Copyright and Related
    Rights in the Work or (ii) assert any associated claims and causes of
    action with respect to the Work, in either case contrary to Affirmer's
    express Statement of Purpose.

    4. Limitations and Disclaimers.

     a. No trademark or patent rights held by Affirmer are waived, abandoned,
        surrendered, licensed or otherwise affected by this document.
     b. Affirmer offers the Work as-is and makes no representations or
        warranties of any kind concerning the Work, express, implied,
        statutory or otherwise, including without limitation warranties of
        title, merchantability, fitness for a particular purpose, non
        infringement, or the absence of latent or other defects, accuracy, or
        the present or absence of errors, whether or not discoverable, all to
        the greatest extent permissible under applicable law.
     c. Affirmer disclaims responsibility for clearing rights of other persons
        that may apply to the Work or any use thereof, including without
        limitation any person's Copyright and Related Rights in the Work.
        Further, Affirmer disclaims responsibility for obtaining any necessary
        consents, permissions or other rights required for any use of the
        Work.
     d. Affirmer understands and acknowledges that Creative Commons is not a
        party to this document and has no duty or obligation with respect to
        this CC0 or use of the Work.

## block-buffer 0.10.4

License: MIT OR Apache-2.0. Source: https://github.com/RustCrypto/utils
Authors: RustCrypto Developers.

`LICENSE-APACHE` (block-buffer 0.10.4): the same text as `LICENSE-APACHE` (base16ct 0.2.0) above.

`LICENSE-MIT` (block-buffer 0.10.4):

    Copyright (c) 2018-2019 The RustCrypto Project Developers

    Permission is hereby granted, free of charge, to any
    person obtaining a copy of this software and associated
    documentation files (the "Software"), to deal in the
    Software without restriction, including without
    limitation the rights to use, copy, modify, merge,
    publish, distribute, sublicense, and/or sell copies of
    the Software, and to permit persons to whom the Software
    is furnished to do so, subject to the following
    conditions:

    The above copyright notice and this permission notice
    shall be included in all copies or substantial portions
    of the Software.

    THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF
    ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED
    TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A
    PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT
    SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
    CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
    OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR
    IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
    DEALINGS IN THE SOFTWARE.

## block-buffer 0.12.1

License: MIT OR Apache-2.0. Source: https://github.com/RustCrypto/utils
Authors: RustCrypto Developers.

`LICENSE-APACHE` (block-buffer 0.12.1): the same text as `LICENSE-APACHE` (base16ct 0.2.0) above.

`LICENSE-MIT` (block-buffer 0.12.1):

    Copyright (c) 2018-2025 The RustCrypto Project Developers

    Permission is hereby granted, free of charge, to any
    person obtaining a copy of this software and associated
    documentation files (the "Software"), to deal in the
    Software without restriction, including without
    limitation the rights to use, copy, modify, merge,
    publish, distribute, sublicense, and/or sell copies of
    the Software, and to permit persons to whom the Software
    is furnished to do so, subject to the following
    conditions:

    The above copyright notice and this permission notice
    shall be included in all copies or substantial portions
    of the Software.

    THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF
    ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED
    TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A
    PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT
    SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
    CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
    OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR
    IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
    DEALINGS IN THE SOFTWARE.

## block-buffer 0.9.0

License: MIT OR Apache-2.0. Source: https://github.com/RustCrypto/utils
Authors: RustCrypto Developers.

`LICENSE-APACHE` (block-buffer 0.9.0): the same text as `LICENSE-APACHE` (base16ct 0.2.0) above.

`LICENSE-MIT` (block-buffer 0.9.0): the same text as `LICENSE-MIT` (block-buffer 0.10.4) above.

## bls12_381 0.7.1

License: MIT/Apache-2.0. Source: https://github.com/zkcrypto/bls12_381
Authors: Sean Bowe <ewillbefull@gmail.com>, Jack Grigg <thestr4d@gmail.com>.

`COPYRIGHT` (bls12_381 0.7.1):

    Copyrights in the "bls12_381" library are retained by their contributors. No
    copyright assignment is required to contribute to the "bls12_381" library.

    The "bls12_381" library is licensed under either of

     * Apache License, Version 2.0, (see ./LICENSE-APACHE or http://www.apache.org/licenses/LICENSE-2.0)
     * MIT license (see ./LICENSE-MIT or http://opensource.org/licenses/MIT)

    at your option.

    Unless you explicitly state otherwise, any contribution intentionally
    submitted for inclusion in the work by you, as defined in the Apache-2.0
    license, shall be dual licensed as above, without any additional terms or
    conditions.

`LICENSE-APACHE` (bls12_381 0.7.1): the same text as `LICENSE-APACHE` (addr2line 0.25.1) above.

`LICENSE-MIT` (bls12_381 0.7.1): the same text as `LICENSE-MIT` (adler2 2.0.1) above.

## bls12_381 0.8.0

License: MIT/Apache-2.0. Source: https://github.com/zkcrypto/bls12_381
Authors: Sean Bowe <ewillbefull@gmail.com>, Jack Grigg <thestr4d@gmail.com>.

`COPYRIGHT` (bls12_381 0.8.0): the same text as `COPYRIGHT` (bls12_381 0.7.1) above.

`LICENSE-APACHE` (bls12_381 0.8.0): the same text as `LICENSE-APACHE` (addr2line 0.25.1) above.

`LICENSE-MIT` (bls12_381 0.8.0): the same text as `LICENSE-MIT` (adler2 2.0.1) above.

## brotli 8.0.4

License: BSD-3-Clause AND MIT. Source: https://github.com/dropbox/rust-brotli
Authors: Daniel Reiter Horn <danielrh@dropbox.com>, The Brotli Authors.

`LICENSE.BSD-3-Clause` (brotli 8.0.4): the same text as `LICENSE` (alloc-no-stdlib 2.0.4) above.

`LICENSE.MIT` (brotli 8.0.4):

    Copyright (c) 2009, 2010, 2013-2016 by the Brotli Authors.

    Permission is hereby granted, free of charge, to any person obtaining a copy
    of this software and associated documentation files (the "Software"), to deal
    in the Software without restriction, including without limitation the rights
    to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
    copies of the Software, and to permit persons to whom the Software is
    furnished to do so, subject to the following conditions:

    The above copyright notice and this permission notice shall be included in
    all copies or substantial portions of the Software.

    THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
    IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
    FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.  IN NO EVENT SHALL THE
    AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
    LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
    OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN
    THE SOFTWARE.

## brotli-decompressor 5.0.3

License: BSD-3-Clause/MIT. Source: https://github.com/dropbox/rust-brotli-decompressor
Authors: Daniel Reiter Horn <danielrh@dropbox.com>, The Brotli Authors.

`LICENSE` (brotli-decompressor 5.0.3): the same text as `LICENSE` (alloc-no-stdlib 2.0.4) above.

## bytemuck 1.25.2

License: Zlib OR Apache-2.0 OR MIT. Source: https://github.com/Lokathor/bytemuck
Authors: Lokathor <zefria@gmail.com>.

`LICENSE-APACHE` (bytemuck 1.25.2):

    Apache License
    Version 2.0, January 2004
    http://www.apache.org/licenses/

    TERMS AND CONDITIONS FOR USE, REPRODUCTION, AND DISTRIBUTION

        1. Definitions.

            "License" shall mean the terms and conditions for use, reproduction, and distribution as defined by Sections 1 through 9 of this document.

            "Licensor" shall mean the copyright owner or entity authorized by the copyright owner that is granting the License.

            "Legal Entity" shall mean the union of the acting entity and all other entities that control, are controlled by, or are under common control with that entity. For the purposes of this definition, "control" means (i) the power, direct or indirect, to cause the direction or management of such entity, whether by contract or otherwise, or (ii) ownership of fifty percent (50%) or more of the outstanding shares, or (iii) beneficial ownership of such entity.

            "You" (or "Your") shall mean an individual or Legal Entity exercising permissions granted by this License.

            "Source" form shall mean the preferred form for making modifications, including but not limited to software source code, documentation source, and configuration files.

            "Object" form shall mean any form resulting from mechanical transformation or translation of a Source form, including but not limited to compiled object code, generated documentation, and conversions to other media types.

            "Work" shall mean the work of authorship, whether in Source or Object form, made available under the License, as indicated by a copyright notice that is included in or attached to the work (an example is provided in the Appendix below).

            "Derivative Works" shall mean any work, whether in Source or Object form, that is based on (or derived from) the Work and for which the editorial revisions, annotations, elaborations, or other modifications represent, as a whole, an original work of authorship. For the purposes of this License, Derivative Works shall not include works that remain separable from, or merely link (or bind by name) to the interfaces of, the Work and Derivative Works thereof.

            "Contribution" shall mean any work of authorship, including the original version of the Work and any modifications or additions to that Work or Derivative Works thereof, that is intentionally submitted to Licensor for inclusion in the Work by the copyright owner or by an individual or Legal Entity authorized to submit on behalf of the copyright owner. For the purposes of this definition, "submitted" means any form of electronic, verbal, or written communication sent to the Licensor or its representatives, including but not limited to communication on electronic mailing lists, source code control systems, and issue tracking systems that are managed by, or on behalf of, the Licensor for the purpose of discussing and improving the Work, but excluding communication that is conspicuously marked or otherwise designated in writing by the copyright owner as "Not a Contribution."

            "Contributor" shall mean Licensor and any individual or Legal Entity on behalf of whom a Contribution has been received by Licensor and subsequently incorporated within the Work.
        2. Grant of Copyright License. Subject to the terms and conditions of this License, each Contributor hereby grants to You a perpetual, worldwide, non-exclusive, no-charge, royalty-free, irrevocable copyright license to reproduce, prepare Derivative Works of, publicly display, publicly perform, sublicense, and distribute the Work and such Derivative Works in Source or Object form.
        3. Grant of Patent License. Subject to the terms and conditions of this License, each Contributor hereby grants to You a perpetual, worldwide, non-exclusive, no-charge, royalty-free, irrevocable (except as stated in this section) patent license to make, have made, use, offer to sell, sell, import, and otherwise transfer the Work, where such license applies only to those patent claims licensable by such Contributor that are necessarily infringed by their Contribution(s) alone or by combination of their Contribution(s) with the Work to which such Contribution(s) was submitted. If You institute patent litigation against any entity (including a cross-claim or counterclaim in a lawsuit) alleging that the Work or a Contribution incorporated within the Work constitutes direct or contributory patent infringement, then any patent licenses granted to You under this License for that Work shall terminate as of the date such litigation is filed.
        4. Redistribution. You may reproduce and distribute copies of the Work or Derivative Works thereof in any medium, with or without modifications, and in Source or Object form, provided that You meet the following conditions:
            (a) You must give any other recipients of the Work or Derivative Works a copy of this License; and
            (b) You must cause any modified files to carry prominent notices stating that You changed the files; and
            (c) You must retain, in the Source form of any Derivative Works that You distribute, all copyright, patent, trademark, and attribution notices from the Source form of the Work, excluding those notices that do not pertain to any part of the Derivative Works; and
            (d) If the Work includes a "NOTICE" text file as part of its distribution, then any Derivative Works that You distribute must include a readable copy of the attribution notices contained within such NOTICE file, excluding those notices that do not pertain to any part of the Derivative Works, in at least one of the following places: within a NOTICE text file distributed as part of the Derivative Works; within the Source form or documentation, if provided along with the Derivative Works; or, within a display generated by the Derivative Works, if and wherever such third-party notices normally appear. The contents of the NOTICE file are for informational purposes only and do not modify the License. You may add Your own attribution notices within Derivative Works that You distribute, alongside or as an addendum to the NOTICE text from the Work, provided that such additional attribution notices cannot be construed as modifying the License.

            You may add Your own copyright statement to Your modifications and may provide additional or different license terms and conditions for use, reproduction, or distribution of Your modifications, or for any such Derivative Works as a whole, provided Your use, reproduction, and distribution of the Work otherwise complies with the conditions stated in this License.
        5. Submission of Contributions. Unless You explicitly state otherwise, any Contribution intentionally submitted for inclusion in the Work by You to the Licensor shall be under the terms and conditions of this License, without any additional terms or conditions. Notwithstanding the above, nothing herein shall supersede or modify the terms of any separate license agreement you may have executed with Licensor regarding such Contributions.
        6. Trademarks. This License does not grant permission to use the trade names, trademarks, service marks, or product names of the Licensor, except as required for reasonable and customary use in describing the origin of the Work and reproducing the content of the NOTICE file.
        7. Disclaimer of Warranty. Unless required by applicable law or agreed to in writing, Licensor provides the Work (and each Contributor provides its Contributions) on an "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied, including, without limitation, any warranties or conditions of TITLE, NON-INFRINGEMENT, MERCHANTABILITY, or FITNESS FOR A PARTICULAR PURPOSE. You are solely responsible for determining the appropriateness of using or redistributing the Work and assume any risks associated with Your exercise of permissions under this License.
        8. Limitation of Liability. In no event and under no legal theory, whether in tort (including negligence), contract, or otherwise, unless required by applicable law (such as deliberate and grossly negligent acts) or agreed to in writing, shall any Contributor be liable to You for damages, including any direct, indirect, special, incidental, or consequential damages of any character arising as a result of this License or out of the use or inability to use the Work (including but not limited to damages for loss of goodwill, work stoppage, computer failure or malfunction, or any and all other commercial damages or losses), even if such Contributor has been advised of the possibility of such damages.
        9. Accepting Warranty or Additional Liability. While redistributing the Work or Derivative Works thereof, You may choose to offer, and charge a fee for, acceptance of support, warranty, indemnity, or other liability obligations and/or rights consistent with this License. However, in accepting such obligations, You may act only on Your own behalf and on Your sole responsibility, not on behalf of any other Contributor, and only if You agree to indemnify, defend, and hold each Contributor harmless for any liability incurred by, or claims asserted against, such Contributor by reason of your accepting any such warranty or additional liability.

    END OF TERMS AND CONDITIONS

    APPENDIX: How to apply the Apache License to your work.

    To apply the Apache License to your work, attach the following boilerplate notice, with the fields enclosed by brackets "[]" replaced with your own identifying information. (Don't include the brackets!) The text should be enclosed in the appropriate comment syntax for the file format. We also recommend that a file or class name and description of purpose be included on the same "printed page" as the copyright notice for easier identification within third-party archives.

    Copyright [yyyy] [name of copyright owner]

    Licensed under the Apache License, Version 2.0 (the "License");
    you may not use this file except in compliance with the License.
    You may obtain a copy of the License at

    http://www.apache.org/licenses/LICENSE-2.0

    Unless required by applicable law or agreed to in writing, software
    distributed under the License is distributed on an "AS IS" BASIS,
    WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
    See the License for the specific language governing permissions and
    limitations under the License.

`LICENSE-MIT` (bytemuck 1.25.2):

    MIT License

    Copyright (c) 2019 Daniel "Lokathor" Gee.

    Permission is hereby granted, free of charge, to any person obtaining a copy of this software and associated documentation files (the "Software"), to deal in the Software without restriction, including without limitation the rights to use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of the Software, and to permit persons to whom the Software is furnished to do so, subject to the following conditions:

    The above copyright notice and this permission notice (including the next paragraph) shall be included in all copies or substantial portions of the Software.

    THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

`LICENSE-ZLIB` (bytemuck 1.25.2):

    Copyright (c) 2019 Daniel "Lokathor" Gee.

    This software is provided 'as-is', without any express or implied warranty. In no event will the authors be held liable for any damages arising from the use of this software.

    Permission is granted to anyone to use this software for any purpose, including commercial applications, and to alter it and redistribute it freely, subject to the following restrictions:

    1. The origin of this software must not be misrepresented; you must not claim that you wrote the original software. If you use this software in a product, an acknowledgment in the product documentation would be appreciated but is not required.

    2. Altered source versions must be plainly marked as such, and must not be misrepresented as being the original software.

    3. This notice may not be removed or altered from any source distribution.

## byteorder 1.5.0

License: Unlicense OR MIT. Source: https://github.com/BurntSushi/byteorder
Authors: Andrew Gallant <jamslam@gmail.com>.

`COPYING` (byteorder 1.5.0): the same text as `COPYING` (aho-corasick 1.1.5) above.

`LICENSE-MIT` (byteorder 1.5.0): the same text as `LICENSE-MIT` (aho-corasick 1.1.5) above.

`UNLICENSE` (byteorder 1.5.0): the same text as `UNLICENSE` (aho-corasick 1.1.5) above.

## bytes 1.12.1

License: MIT. Source: https://github.com/tokio-rs/bytes
Authors: Carl Lerche <me@carllerche.com>, Sean McArthur <sean@seanmonstar.com>.

`LICENSE` (bytes 1.12.1):

    Copyright (c) 2018 Carl Lerche

    Permission is hereby granted, free of charge, to any
    person obtaining a copy of this software and associated
    documentation files (the "Software"), to deal in the
    Software without restriction, including without
    limitation the rights to use, copy, modify, merge,
    publish, distribute, sublicense, and/or sell copies of
    the Software, and to permit persons to whom the Software
    is furnished to do so, subject to the following
    conditions:

    The above copyright notice and this permission notice
    shall be included in all copies or substantial portions
    of the Software.

    THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF
    ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED
    TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A
    PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT
    SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
    CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
    OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR
    IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
    DEALINGS IN THE SOFTWARE.

## bytesize 2.7.0

License: Apache-2.0. Source: https://github.com/bytesize-rs/bytesize
Authors: Hyunsik Choi <hyunsik.choi@gmail.com>, MrCroxx <mrcroxx@outlook.com>, Rob Ede <robjtede@icloud.com>.

`LICENSE` (bytesize 2.7.0): the same text as `LICENSE-APACHE` (anstyle 1.0.14) above.

## cfg-if 1.0.4

License: MIT OR Apache-2.0. Source: https://github.com/rust-lang/cfg-if
Authors: Alex Crichton <alex@alexcrichton.com>.

`LICENSE-APACHE` (cfg-if 1.0.4): the same text as `LICENSE-APACHE` (addr2line 0.25.1) above.

`LICENSE-MIT` (cfg-if 1.0.4): the same text as `LICENSE-MIT` (backtrace 0.3.76) above.

## cfg-if 1.0.5

License: MIT OR Apache-2.0. Source: https://github.com/rust-lang/cfg-if
Authors: Alex Crichton <alex@alexcrichton.com>.

`LICENSE-APACHE` (cfg-if 1.0.5): the same text as `LICENSE-APACHE` (addr2line 0.25.1) above.

`LICENSE-MIT` (cfg-if 1.0.5): the same text as `LICENSE-MIT` (backtrace 0.3.76) above.

## cobs 0.3.0

License: MIT OR Apache-2.0. Source: https://github.com/jamesmunns/cobs.rs
Authors: Allen Welkie <>, James Munns <james@onevariable.com>.

`LICENSE-APACHE` (cobs 0.3.0): the same text as `LICENSE-APACHE` (anstyle 1.0.14) above.

`LICENSE-MIT` (cobs 0.3.0):

    Copyright (c) 2015 The cobs.rs Developers

    Permission is hereby granted, free of charge, to any person obtaining a copy
    of this software and associated documentation files (the "Software"), to deal
    in the Software without restriction, including without limitation the rights
    to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
    copies of the Software, and to permit persons to whom the Software is
    furnished to do so, subject to the following conditions:

    The above copyright notice and this permission notice shall be included in all
    copies or substantial portions of the Software.

    THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
    IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
    FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
    AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
    LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
    OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
    SOFTWARE.

## colorchoice 1.0.5

License: MIT OR Apache-2.0. Source: https://github.com/rust-cli/anstyle.git

`LICENSE-APACHE` (colorchoice 1.0.5): the same text as `LICENSE-APACHE` (anstyle 1.0.14) above.

`LICENSE-MIT` (colorchoice 1.0.5): the same text as `LICENSE-MIT` (anstyle 1.0.14) above.

## const-hex 1.19.1

License: MIT OR Apache-2.0. Source: https://github.com/danipopes/const-hex
Authors: DaniPopes <57450786+DaniPopes@users.noreply.github.com>.

`LICENSE-APACHE` (const-hex 1.19.1): the same text as `LICENSE-APACHE` (allocator-api2 0.2.21) above.

`LICENSE-MIT` (const-hex 1.19.1): the same text as `LICENSE-MIT` (adler2 2.0.1) above.

## const-hex 1.19.3

License: MIT OR Apache-2.0. Source: https://github.com/danipopes/const-hex
Authors: DaniPopes <57450786+DaniPopes@users.noreply.github.com>.

`LICENSE-APACHE` (const-hex 1.19.3): the same text as `LICENSE-APACHE` (allocator-api2 0.2.21) above.

`LICENSE-MIT` (const-hex 1.19.3): the same text as `LICENSE-MIT` (adler2 2.0.1) above.

## const-oid 0.10.2

License: Apache-2.0 OR MIT. Source: https://github.com/RustCrypto/formats
Authors: RustCrypto Developers.

`LICENSE-APACHE` (const-oid 0.10.2): the same text as `LICENSE-APACHE` (base16ct 0.2.0) above.

`LICENSE-MIT` (const-oid 0.10.2):

    Copyright (c) 2020-2026 The RustCrypto Project Developers

    Permission is hereby granted, free of charge, to any
    person obtaining a copy of this software and associated
    documentation files (the "Software"), to deal in the
    Software without restriction, including without
    limitation the rights to use, copy, modify, merge,
    publish, distribute, sublicense, and/or sell copies of
    the Software, and to permit persons to whom the Software
    is furnished to do so, subject to the following
    conditions:

    The above copyright notice and this permission notice
    shall be included in all copies or substantial portions
    of the Software.

    THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF
    ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED
    TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A
    PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT
    SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
    CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
    OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR
    IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
    DEALINGS IN THE SOFTWARE.

## const-oid 0.9.6

License: Apache-2.0 OR MIT. Source: https://github.com/RustCrypto/formats/tree/master/const-oid
Authors: RustCrypto Developers.

`LICENSE-APACHE` (const-oid 0.9.6): the same text as `LICENSE-APACHE` (base16ct 0.2.0) above.

`LICENSE-MIT` (const-oid 0.9.6):

    Copyright (c) 2020-2022 The RustCrypto Project Developers

    Permission is hereby granted, free of charge, to any
    person obtaining a copy of this software and associated
    documentation files (the "Software"), to deal in the
    Software without restriction, including without
    limitation the rights to use, copy, modify, merge,
    publish, distribute, sublicense, and/or sell copies of
    the Software, and to permit persons to whom the Software
    is furnished to do so, subject to the following
    conditions:

    The above copyright notice and this permission notice
    shall be included in all copies or substantial portions
    of the Software.

    THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF
    ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED
    TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A
    PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT
    SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
    CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
    OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR
    IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
    DEALINGS IN THE SOFTWARE.

## constant_time_eq 0.4.2

License: CC0-1.0 OR MIT-0 OR Apache-2.0. Source: https://github.com/cesarb/constant_time_eq
Authors: Cesar Eduardo Barros <cesarb@cesarb.eti.br>.

`LICENSE-APACHE` (constant_time_eq 0.4.2): the same text as `LICENSE-APACHE` (allocator-api2 0.2.21) above.

`LICENSE-CC0` (constant_time_eq 0.4.2): the same text as `LICENSE_CC0` (blake3 1.8.7) above.

`LICENSE-MIT0` (constant_time_eq 0.4.2):

    Permission is hereby granted, free of charge, to any person obtaining a
    copy of this software and associated documentation files (the
    "Software"), to deal in the Software without restriction, including
    without limitation the rights to use, copy, modify, merge, publish,
    distribute, sublicense, and/or sell copies of the Software, and to
    permit persons to whom the Software is furnished to do so.

    THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS
    OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF
    MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.
    IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
    CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT,
    TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE
    SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

## crc 3.4.0

License: MIT OR Apache-2.0. Source: https://github.com/mrhooray/crc-rs.git
Authors: Rui Hu <code@mrhooray.com>, Akhil Velagapudi <4@4khil.com>.

`LICENSE-APACHE` (crc 3.4.0):

                                  Apache License
                            Version 2.0 January 2004
                         http://www.apache.org/licenses/

    TERMS AND CONDITIONS FOR USE, REPRODUCTION, AND DISTRIBUTION

    1. Definitions.

       "License" shall mean the terms and conditions for use, reproduction,
       and distribution as defined by Sections 1 through 9 of this document.

       "Licensor" shall mean the copyright owner or entity authorized by
       the copyright owner that is granting the License.

       "Legal Entity" shall mean the union of the acting entity and all
       other entities that control, are controlled by, or are under common
       control with that entity. For the purposes of this definition,
       "control" means (i) the power, direct or indirect, to cause the
       direction or management of such entity, whether by contract or
       otherwise, or (ii) ownership of fifty percent (50%) or more of the
       outstanding shares, or (iii) beneficial ownership of such entity.

       "You" (or "Your") shall mean an individual or Legal Entity
       exercising permissions granted by this License.

       "Source" form shall mean the preferred form for making modifications,
       including but not limited to software source code, documentation
       source, and configuration files.

       "Object" form shall mean any form resulting from mechanical
       transformation or translation of a Source form, including but
       not limited to compiled object code, generated documentation,
       and conversions to other media types.

       "Work" shall mean the work of authorship, whether in Source or
       Object form, made available under the License, as indicated by a
       copyright notice that is included in or attached to the work
       (an example is provided in the Appendix below).

       "Derivative Works" shall mean any work, whether in Source or Object
       form, that is based on (or derived from) the Work and for which the
       editorial revisions, annotations, elaborations, or other modifications
       represent, as a whole, an original work of authorship. For the purposes
       of this License, Derivative Works shall not include works that remain
       separable from, or merely link (or bind by name) to the interfaces of,
       the Work and Derivative Works thereof.

       "Contribution" shall mean any work of authorship, including
       the original version of the Work and any modifications or additions
       to that Work or Derivative Works thereof, that is intentionally
       submitted to Licensor for inclusion in the Work by the copyright owner
       or by an individual or Legal Entity authorized to submit on behalf of
       the copyright owner. For the purposes of this definition, "submitted"
       means any form of electronic, verbal, or written communication sent
       to the Licensor or its representatives, including but not limited to
       communication on electronic mailing lists, source code control systems,
       and issue tracking systems that are managed by, or on behalf of, the
       Licensor for the purpose of discussing and improving the Work, but
       excluding communication that is conspicuously marked or otherwise
       designated in writing by the copyright owner as "Not a Contribution."

       "Contributor" shall mean Licensor and any individual or Legal Entity
       on behalf of whom a Contribution has been received by Licensor and
       subsequently incorporated within the Work.

    2. Grant of Copyright License. Subject to the terms and conditions of
       this License, each Contributor hereby grants to You a perpetual,
       worldwide, non-exclusive, no-charge, royalty-free, irrevocable
       copyright license to reproduce, prepare Derivative Works of,
       publicly display, publicly perform, sublicense, and distribute the
       Work and such Derivative Works in Source or Object form.

    3. Grant of Patent License. Subject to the terms and conditions of
       this License, each Contributor hereby grants to You a perpetual,
       worldwide, non-exclusive, no-charge, royalty-free, irrevocable
       (except as stated in this section) patent license to make, have made,
       use, offer to sell, sell, import, and otherwise transfer the Work,
       where such license applies only to those patent claims licensable
       by such Contributor that are necessarily infringed by their
       Contribution(s) alone or by combination of their Contribution(s)
       with the Work to which such Contribution(s) was submitted. If You
       institute patent litigation against any entity (including a
       cross-claim or counterclaim in a lawsuit) alleging that the Work
       or a Contribution incorporated within the Work constitutes direct
       or contributory patent infringement, then any patent licenses
       granted to You under this License for that Work shall terminate
       as of the date such litigation is filed.

    4. Redistribution. You may reproduce and distribute copies of the
       Work or Derivative Works thereof in any medium, with or without
       modifications, and in Source or Object form, provided that You
       meet the following conditions:

       (a) You must give any other recipients of the Work or
           Derivative Works a copy of this License; and

       (b) You must cause any modified files to carry prominent notices
           stating that You changed the files; and

       (c) You must retain, in the Source form of any Derivative Works
           that You distribute, all copyright, patent, trademark, and
           attribution notices from the Source form of the Work,
           excluding those notices that do not pertain to any part of
           the Derivative Works; and

       (d) If the Work includes a "NOTICE" text file as part of its
           distribution, then any Derivative Works that You distribute must
           include a readable copy of the attribution notices contained
           within such NOTICE file, excluding those notices that do not
           pertain to any part of the Derivative Works, in at least one
           of the following places: within a NOTICE text file distributed
           as part of the Derivative Works; within the Source form or
           documentation, if provided along with the Derivative Works; or,
           within a display generated by the Derivative Works, if and
           wherever such third-party notices normally appear. The contents
           of the NOTICE file are for informational purposes only and
           do not modify the License. You may add Your own attribution
           notices within Derivative Works that You distribute, alongside
           or as an addendum to the NOTICE text from the Work, provided
           that such additional attribution notices cannot be construed
           as modifying the License.

       You may add Your own copyright statement to Your modifications and
       may provide additional or different license terms and conditions
       for use, reproduction, or distribution of Your modifications, or
       for any such Derivative Works as a whole, provided Your use,
       reproduction, and distribution of the Work otherwise complies with
       the conditions stated in this License.

    5. Submission of Contributions. Unless You explicitly state otherwise,
       any Contribution intentionally submitted for inclusion in the Work
       by You to the Licensor shall be under the terms and conditions of
       this License, without any additional terms or conditions.
       Notwithstanding the above, nothing herein shall supersede or modify
       the terms of any separate license agreement you may have executed
       with Licensor regarding such Contributions.

    6. Trademarks. This License does not grant permission to use the trade
       names, trademarks, service marks, or product names of the Licensor,
       except as required for reasonable and customary use in describing the
       origin of the Work and reproducing the content of the NOTICE file.

    7. Disclaimer of Warranty. Unless required by applicable law or
       agreed to in writing, Licensor provides the Work (and each
       Contributor provides its Contributions) on an "AS IS" BASIS,
       WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or
       implied, including, without limitation, any warranties or conditions
       of TITLE, NON-INFRINGEMENT, MERCHANTABILITY, or FITNESS FOR A
       PARTICULAR PURPOSE. You are solely responsible for determining the
       appropriateness of using or redistributing the Work and assume any
       risks associated with Your exercise of permissions under this License.

    8. Limitation of Liability. In no event and under no legal theory,
       whether in tort (including negligence), contract, or otherwise,
       unless required by applicable law (such as deliberate and grossly
       negligent acts) or agreed to in writing, shall any Contributor be
       liable to You for damages, including any direct, indirect, special,
       incidental, or consequential damages of any character arising as a
       result of this License or out of the use or inability to use the
       Work (including but not limited to damages for loss of goodwill,
       work stoppage, computer failure or malfunction, or any and all
       other commercial damages or losses), even if such Contributor
       has been advised of the possibility of such damages.

    9. Accepting Warranty or Additional Liability. While redistributing
       the Work or Derivative Works thereof, You may choose to offer,
       and charge a fee for, acceptance of support, warranty, indemnity,
       or other liability obligations and/or rights consistent with this
       License. However, in accepting such obligations, You may act only
       on Your own behalf and on Your sole responsibility, not on behalf
       of any other Contributor, and only if You agree to indemnify,
       defend, and hold each Contributor harmless for any liability
       incurred by, or claims asserted against, such Contributor by reason
       of your accepting any such warranty or additional liability.

    END OF TERMS AND CONDITIONS

    APPENDIX: How to apply the Apache License to your work.

       To apply the Apache License to your work, attach the following
       boilerplate notice, with the fields enclosed by brackets "[]"
       replaced with your own identifying information. (Don't include
       the brackets!)  The text should be enclosed in the appropriate
       comment syntax for the file format. We also recommend that a
       file or class name and description of purpose be included on the
       same "printed page" as the copyright notice for easier
       identification within third-party archives.

    Copyright [yyyy] [name of copyright owner]

    Licensed under the Apache License, Version 2.0 (the "License");
    you may not use this file except in compliance with the License.
    You may obtain a copy of the License at

    	http://www.apache.org/licenses/LICENSE-2.0

    Unless required by applicable law or agreed to in writing, software
    distributed under the License is distributed on an "AS IS" BASIS,
    WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
    See the License for the specific language governing permissions and
    limitations under the License.

`LICENSE-MIT` (crc 3.4.0):

    MIT License

    Copyright (c) 2017 crc-rs Developers

    Permission is hereby granted, free of charge, to any person obtaining a copy
    of this software and associated documentation files (the "Software"), to deal
    in the Software without restriction, including without limitation the rights
    to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
    copies of the Software, and to permit persons to whom the Software is
    furnished to do so, subject to the following conditions:

    The above copyright notice and this permission notice shall be included in all
    copies or substantial portions of the Software.

    THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
    IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
    FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
    AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
    LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
    OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
    SOFTWARE.

## crc-catalog 2.5.0

License: MIT OR Apache-2.0. Source: https://github.com/akhilles/crc-catalog.git
Authors: Akhil Velagapudi <akhilvelagapudi@gmail.com>.

Its package carries no license file: the standard text of its license, under its authors.

Apache-2.0 (crc-catalog 2.5.0): the same text as Apache-2.0 above.

MIT (crc-catalog 2.5.0):

    MIT License

    Copyright (c) Akhil Velagapudi <akhilvelagapudi@gmail.com>

    Permission is hereby granted, free of charge, to any person obtaining a copy of this software and associated documentation files (the "Software"), to deal in the Software without restriction, including without limitation the rights to use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of the Software, and to permit persons to whom the Software is furnished to do so, subject to the following conditions:

    The above copyright notice and this permission notice shall be included in all copies or substantial portions of the Software.

    THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

## crossbeam 0.8.4

License: MIT OR Apache-2.0. Source: https://github.com/crossbeam-rs/crossbeam

`LICENSE-APACHE` (crossbeam 0.8.4):

                                  Apache License
                            Version 2.0, January 2004
                         http://www.apache.org/licenses/

    TERMS AND CONDITIONS FOR USE, REPRODUCTION, AND DISTRIBUTION

    1. Definitions.

       "License" shall mean the terms and conditions for use, reproduction,
       and distribution as defined by Sections 1 through 9 of this document.

       "Licensor" shall mean the copyright owner or entity authorized by
       the copyright owner that is granting the License.

       "Legal Entity" shall mean the union of the acting entity and all
       other entities that control, are controlled by, or are under common
       control with that entity. For the purposes of this definition,
       "control" means (i) the power, direct or indirect, to cause the
       direction or management of such entity, whether by contract or
       otherwise, or (ii) ownership of fifty percent (50%) or more of the
       outstanding shares, or (iii) beneficial ownership of such entity.

       "You" (or "Your") shall mean an individual or Legal Entity
       exercising permissions granted by this License.

       "Source" form shall mean the preferred form for making modifications,
       including but not limited to software source code, documentation
       source, and configuration files.

       "Object" form shall mean any form resulting from mechanical
       transformation or translation of a Source form, including but
       not limited to compiled object code, generated documentation,
       and conversions to other media types.

       "Work" shall mean the work of authorship, whether in Source or
       Object form, made available under the License, as indicated by a
       copyright notice that is included in or attached to the work
       (an example is provided in the Appendix below).

       "Derivative Works" shall mean any work, whether in Source or Object
       form, that is based on (or derived from) the Work and for which the
       editorial revisions, annotations, elaborations, or other modifications
       represent, as a whole, an original work of authorship. For the purposes
       of this License, Derivative Works shall not include works that remain
       separable from, or merely link (or bind by name) to the interfaces of,
       the Work and Derivative Works thereof.

       "Contribution" shall mean any work of authorship, including
       the original version of the Work and any modifications or additions
       to that Work or Derivative Works thereof, that is intentionally
       submitted to Licensor for inclusion in the Work by the copyright owner
       or by an individual or Legal Entity authorized to submit on behalf of
       the copyright owner. For the purposes of this definition, "submitted"
       means any form of electronic, verbal, or written communication sent
       to the Licensor or its representatives, including but not limited to
       communication on electronic mailing lists, source code control systems,
       and issue tracking systems that are managed by, or on behalf of, the
       Licensor for the purpose of discussing and improving the Work, but
       excluding communication that is conspicuously marked or otherwise
       designated in writing by the copyright owner as "Not a Contribution."

       "Contributor" shall mean Licensor and any individual or Legal Entity
       on behalf of whom a Contribution has been received by Licensor and
       subsequently incorporated within the Work.

    2. Grant of Copyright License. Subject to the terms and conditions of
       this License, each Contributor hereby grants to You a perpetual,
       worldwide, non-exclusive, no-charge, royalty-free, irrevocable
       copyright license to reproduce, prepare Derivative Works of,
       publicly display, publicly perform, sublicense, and distribute the
       Work and such Derivative Works in Source or Object form.

    3. Grant of Patent License. Subject to the terms and conditions of
       this License, each Contributor hereby grants to You a perpetual,
       worldwide, non-exclusive, no-charge, royalty-free, irrevocable
       (except as stated in this section) patent license to make, have made,
       use, offer to sell, sell, import, and otherwise transfer the Work,
       where such license applies only to those patent claims licensable
       by such Contributor that are necessarily infringed by their
       Contribution(s) alone or by combination of their Contribution(s)
       with the Work to which such Contribution(s) was submitted. If You
       institute patent litigation against any entity (including a
       cross-claim or counterclaim in a lawsuit) alleging that the Work
       or a Contribution incorporated within the Work constitutes direct
       or contributory patent infringement, then any patent licenses
       granted to You under this License for that Work shall terminate
       as of the date such litigation is filed.

    4. Redistribution. You may reproduce and distribute copies of the
       Work or Derivative Works thereof in any medium, with or without
       modifications, and in Source or Object form, provided that You
       meet the following conditions:

       (a) You must give any other recipients of the Work or
           Derivative Works a copy of this License; and

       (b) You must cause any modified files to carry prominent notices
           stating that You changed the files; and

       (c) You must retain, in the Source form of any Derivative Works
           that You distribute, all copyright, patent, trademark, and
           attribution notices from the Source form of the Work,
           excluding those notices that do not pertain to any part of
           the Derivative Works; and

       (d) If the Work includes a "NOTICE" text file as part of its
           distribution, then any Derivative Works that You distribute must
           include a readable copy of the attribution notices contained
           within such NOTICE file, excluding those notices that do not
           pertain to any part of the Derivative Works, in at least one
           of the following places: within a NOTICE text file distributed
           as part of the Derivative Works; within the Source form or
           documentation, if provided along with the Derivative Works; or,
           within a display generated by the Derivative Works, if and
           wherever such third-party notices normally appear. The contents
           of the NOTICE file are for informational purposes only and
           do not modify the License. You may add Your own attribution
           notices within Derivative Works that You distribute, alongside
           or as an addendum to the NOTICE text from the Work, provided
           that such additional attribution notices cannot be construed
           as modifying the License.

       You may add Your own copyright statement to Your modifications and
       may provide additional or different license terms and conditions
       for use, reproduction, or distribution of Your modifications, or
       for any such Derivative Works as a whole, provided Your use,
       reproduction, and distribution of the Work otherwise complies with
       the conditions stated in this License.

    5. Submission of Contributions. Unless You explicitly state otherwise,
       any Contribution intentionally submitted for inclusion in the Work
       by You to the Licensor shall be under the terms and conditions of
       this License, without any additional terms or conditions.
       Notwithstanding the above, nothing herein shall supersede or modify
       the terms of any separate license agreement you may have executed
       with Licensor regarding such Contributions.

    6. Trademarks. This License does not grant permission to use the trade
       names, trademarks, service marks, or product names of the Licensor,
       except as required for reasonable and customary use in describing the
       origin of the Work and reproducing the content of the NOTICE file.

    7. Disclaimer of Warranty. Unless required by applicable law or
       agreed to in writing, Licensor provides the Work (and each
       Contributor provides its Contributions) on an "AS IS" BASIS,
       WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or
       implied, including, without limitation, any warranties or conditions
       of TITLE, NON-INFRINGEMENT, MERCHANTABILITY, or FITNESS FOR A
       PARTICULAR PURPOSE. You are solely responsible for determining the
       appropriateness of using or redistributing the Work and assume any
       risks associated with Your exercise of permissions under this License.

    8. Limitation of Liability. In no event and under no legal theory,
       whether in tort (including negligence), contract, or otherwise,
       unless required by applicable law (such as deliberate and grossly
       negligent acts) or agreed to in writing, shall any Contributor be
       liable to You for damages, including any direct, indirect, special,
       incidental, or consequential damages of any character arising as a
       result of this License or out of the use or inability to use the
       Work (including but not limited to damages for loss of goodwill,
       work stoppage, computer failure or malfunction, or any and all
       other commercial damages or losses), even if such Contributor
       has been advised of the possibility of such damages.

    9. Accepting Warranty or Additional Liability. While redistributing
       the Work or Derivative Works thereof, You may choose to offer,
       and charge a fee for, acceptance of support, warranty, indemnity,
       or other liability obligations and/or rights consistent with this
       License. However, in accepting such obligations, You may act only
       on Your own behalf and on Your sole responsibility, not on behalf
       of any other Contributor, and only if You agree to indemnify,
       defend, and hold each Contributor harmless for any liability
       incurred by, or claims asserted against, such Contributor by reason
       of your accepting any such warranty or additional liability.

    END OF TERMS AND CONDITIONS

    APPENDIX: How to apply the Apache License to your work.

       To apply the Apache License to your work, attach the following
       boilerplate notice, with the fields enclosed by brackets "[]"
       replaced with your own identifying information. (Don't include
       the brackets!)  The text should be enclosed in the appropriate
       comment syntax for the file format. We also recommend that a
       file or class name and description of purpose be included on the
       same "printed page" as the copyright notice for easier
       identification within third-party archives.

    Copyright 2019 The Crossbeam Project Developers

    Licensed under the Apache License, Version 2.0 (the "License");
    you may not use this file except in compliance with the License.
    You may obtain a copy of the License at

    	http://www.apache.org/licenses/LICENSE-2.0

    Unless required by applicable law or agreed to in writing, software
    distributed under the License is distributed on an "AS IS" BASIS,
    WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
    See the License for the specific language governing permissions and
    limitations under the License.

`LICENSE-MIT` (crossbeam 0.8.4):

    The MIT License (MIT)

    Copyright (c) 2019 The Crossbeam Project Developers

    Permission is hereby granted, free of charge, to any
    person obtaining a copy of this software and associated
    documentation files (the "Software"), to deal in the
    Software without restriction, including without
    limitation the rights to use, copy, modify, merge,
    publish, distribute, sublicense, and/or sell copies of
    the Software, and to permit persons to whom the Software
    is furnished to do so, subject to the following
    conditions:

    The above copyright notice and this permission notice
    shall be included in all copies or substantial portions
    of the Software.

    THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF
    ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED
    TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A
    PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT
    SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
    CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
    OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR
    IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
    DEALINGS IN THE SOFTWARE.

## crossbeam-channel 0.5.16

License: MIT OR Apache-2.0. Source: https://github.com/crossbeam-rs/crossbeam

`LICENSE-APACHE` (crossbeam-channel 0.5.16): the same text as `LICENSE-APACHE` (addr2line 0.25.1) above.

`LICENSE-MIT` (crossbeam-channel 0.5.16): the same text as `LICENSE-MIT` (crossbeam 0.8.4) above.

`LICENSE-THIRD-PARTY` (crossbeam-channel 0.5.16):

    ===============================================================================

    matching.go
    https://creativecommons.org/licenses/by/3.0/legalcode

    Creative Commons Legal Code

    Attribution 3.0 Unported

        CREATIVE COMMONS CORPORATION IS NOT A LAW FIRM AND DOES NOT PROVIDE
        LEGAL SERVICES. DISTRIBUTION OF THIS LICENSE DOES NOT CREATE AN
        ATTORNEY-CLIENT RELATIONSHIP. CREATIVE COMMONS PROVIDES THIS
        INFORMATION ON AN "AS-IS" BASIS. CREATIVE COMMONS MAKES NO WARRANTIES
        REGARDING THE INFORMATION PROVIDED, AND DISCLAIMS LIABILITY FOR
        DAMAGES RESULTING FROM ITS USE.

    License

    THE WORK (AS DEFINED BELOW) IS PROVIDED UNDER THE TERMS OF THIS CREATIVE
    COMMONS PUBLIC LICENSE ("CCPL" OR "LICENSE"). THE WORK IS PROTECTED BY
    COPYRIGHT AND/OR OTHER APPLICABLE LAW. ANY USE OF THE WORK OTHER THAN AS
    AUTHORIZED UNDER THIS LICENSE OR COPYRIGHT LAW IS PROHIBITED.

    BY EXERCISING ANY RIGHTS TO THE WORK PROVIDED HERE, YOU ACCEPT AND AGREE
    TO BE BOUND BY THE TERMS OF THIS LICENSE. TO THE EXTENT THIS LICENSE MAY
    BE CONSIDERED TO BE A CONTRACT, THE LICENSOR GRANTS YOU THE RIGHTS
    CONTAINED HERE IN CONSIDERATION OF YOUR ACCEPTANCE OF SUCH TERMS AND
    CONDITIONS.

    1. Definitions

     a. "Adaptation" means a work based upon the Work, or upon the Work and
        other pre-existing works, such as a translation, adaptation,
        derivative work, arrangement of music or other alterations of a
        literary or artistic work, or phonogram or performance and includes
        cinematographic adaptations or any other form in which the Work may be
        recast, transformed, or adapted including in any form recognizably
        derived from the original, except that a work that constitutes a
        Collection will not be considered an Adaptation for the purpose of
        this License. For the avoidance of doubt, where the Work is a musical
        work, performance or phonogram, the synchronization of the Work in
        timed-relation with a moving image ("synching") will be considered an
        Adaptation for the purpose of this License.
     b. "Collection" means a collection of literary or artistic works, such as
        encyclopedias and anthologies, or performances, phonograms or
        broadcasts, or other works or subject matter other than works listed
        in Section 1(f) below, which, by reason of the selection and
        arrangement of their contents, constitute intellectual creations, in
        which the Work is included in its entirety in unmodified form along
        with one or more other contributions, each constituting separate and
        independent works in themselves, which together are assembled into a
        collective whole. A work that constitutes a Collection will not be
        considered an Adaptation (as defined above) for the purposes of this
        License.
     c. "Distribute" means to make available to the public the original and
        copies of the Work or Adaptation, as appropriate, through sale or
        other transfer of ownership.
     d. "Licensor" means the individual, individuals, entity or entities that
        offer(s) the Work under the terms of this License.
     e. "Original Author" means, in the case of a literary or artistic work,
        the individual, individuals, entity or entities who created the Work
        or if no individual or entity can be identified, the publisher; and in
        addition (i) in the case of a performance the actors, singers,
        musicians, dancers, and other persons who act, sing, deliver, declaim,
        play in, interpret or otherwise perform literary or artistic works or
        expressions of folklore; (ii) in the case of a phonogram the producer
        being the person or legal entity who first fixes the sounds of a
        performance or other sounds; and, (iii) in the case of broadcasts, the
        organization that transmits the broadcast.
     f. "Work" means the literary and/or artistic work offered under the terms
        of this License including without limitation any production in the
        literary, scientific and artistic domain, whatever may be the mode or
        form of its expression including digital form, such as a book,
        pamphlet and other writing; a lecture, address, sermon or other work
        of the same nature; a dramatic or dramatico-musical work; a
        choreographic work or entertainment in dumb show; a musical
        composition with or without words; a cinematographic work to which are
        assimilated works expressed by a process analogous to cinematography;
        a work of drawing, painting, architecture, sculpture, engraving or
        lithography; a photographic work to which are assimilated works
        expressed by a process analogous to photography; a work of applied
        art; an illustration, map, plan, sketch or three-dimensional work
        relative to geography, topography, architecture or science; a
        performance; a broadcast; a phonogram; a compilation of data to the
        extent it is protected as a copyrightable work; or a work performed by
        a variety or circus performer to the extent it is not otherwise
        considered a literary or artistic work.
     g. "You" means an individual or entity exercising rights under this
        License who has not previously violated the terms of this License with
        respect to the Work, or who has received express permission from the
        Licensor to exercise rights under this License despite a previous
        violation.
     h. "Publicly Perform" means to perform public recitations of the Work and
        to communicate to the public those public recitations, by any means or
        process, including by wire or wireless means or public digital
        performances; to make available to the public Works in such a way that
        members of the public may access these Works from a place and at a
        place individually chosen by them; to perform the Work to the public
        by any means or process and the communication to the public of the
        performances of the Work, including by public digital performance; to
        broadcast and rebroadcast the Work by any means including signs,
        sounds or images.
     i. "Reproduce" means to make copies of the Work by any means including
        without limitation by sound or visual recordings and the right of
        fixation and reproducing fixations of the Work, including storage of a
        protected performance or phonogram in digital form or other electronic
        medium.

    2. Fair Dealing Rights. Nothing in this License is intended to reduce,
    limit, or restrict any uses free from copyright or rights arising from
    limitations or exceptions that are provided for in connection with the
    copyright protection under copyright law or other applicable laws.

    3. License Grant. Subject to the terms and conditions of this License,
    Licensor hereby grants You a worldwide, royalty-free, non-exclusive,
    perpetual (for the duration of the applicable copyright) license to
    exercise the rights in the Work as stated below:

     a. to Reproduce the Work, to incorporate the Work into one or more
        Collections, and to Reproduce the Work as incorporated in the
        Collections;
     b. to create and Reproduce Adaptations provided that any such Adaptation,
        including any translation in any medium, takes reasonable steps to
        clearly label, demarcate or otherwise identify that changes were made
        to the original Work. For example, a translation could be marked "The
        original work was translated from English to Spanish," or a
        modification could indicate "The original work has been modified.";
     c. to Distribute and Publicly Perform the Work including as incorporated
        in Collections; and,
     d. to Distribute and Publicly Perform Adaptations.
     e. For the avoidance of doubt:

         i. Non-waivable Compulsory License Schemes. In those jurisdictions in
            which the right to collect royalties through any statutory or
            compulsory licensing scheme cannot be waived, the Licensor
            reserves the exclusive right to collect such royalties for any
            exercise by You of the rights granted under this License;
        ii. Waivable Compulsory License Schemes. In those jurisdictions in
            which the right to collect royalties through any statutory or
            compulsory licensing scheme can be waived, the Licensor waives the
            exclusive right to collect such royalties for any exercise by You
            of the rights granted under this License; and,
       iii. Voluntary License Schemes. The Licensor waives the right to
            collect royalties, whether individually or, in the event that the
            Licensor is a member of a collecting society that administers
            voluntary licensing schemes, via that society, from any exercise
            by You of the rights granted under this License.

    The above rights may be exercised in all media and formats whether now
    known or hereafter devised. The above rights include the right to make
    such modifications as are technically necessary to exercise the rights in
    other media and formats. Subject to Section 8(f), all rights not expressly
    granted by Licensor are hereby reserved.

    4. Restrictions. The license granted in Section 3 above is expressly made
    subject to and limited by the following restrictions:

     a. You may Distribute or Publicly Perform the Work only under the terms
        of this License. You must include a copy of, or the Uniform Resource
        Identifier (URI) for, this License with every copy of the Work You
        Distribute or Publicly Perform. You may not offer or impose any terms
        on the Work that restrict the terms of this License or the ability of
        the recipient of the Work to exercise the rights granted to that
        recipient under the terms of the License. You may not sublicense the
        Work. You must keep intact all notices that refer to this License and
        to the disclaimer of warranties with every copy of the Work You
        Distribute or Publicly Perform. When You Distribute or Publicly
        Perform the Work, You may not impose any effective technological
        measures on the Work that restrict the ability of a recipient of the
        Work from You to exercise the rights granted to that recipient under
        the terms of the License. This Section 4(a) applies to the Work as
        incorporated in a Collection, but this does not require the Collection
        apart from the Work itself to be made subject to the terms of this
        License. If You create a Collection, upon notice from any Licensor You
        must, to the extent practicable, remove from the Collection any credit
        as required by Section 4(b), as requested. If You create an
        Adaptation, upon notice from any Licensor You must, to the extent
        practicable, remove from the Adaptation any credit as required by
        Section 4(b), as requested.
     b. If You Distribute, or Publicly Perform the Work or any Adaptations or
        Collections, You must, unless a request has been made pursuant to
        Section 4(a), keep intact all copyright notices for the Work and
        provide, reasonable to the medium or means You are utilizing: (i) the
        name of the Original Author (or pseudonym, if applicable) if supplied,
        and/or if the Original Author and/or Licensor designate another party
        or parties (e.g., a sponsor institute, publishing entity, journal) for
        attribution ("Attribution Parties") in Licensor's copyright notice,
        terms of service or by other reasonable means, the name of such party
        or parties; (ii) the title of the Work if supplied; (iii) to the
        extent reasonably practicable, the URI, if any, that Licensor
        specifies to be associated with the Work, unless such URI does not
        refer to the copyright notice or licensing information for the Work;
        and (iv) , consistent with Section 3(b), in the case of an Adaptation,
        a credit identifying the use of the Work in the Adaptation (e.g.,
        "French translation of the Work by Original Author," or "Screenplay
        based on original Work by Original Author"). The credit required by
        this Section 4 (b) may be implemented in any reasonable manner;
        provided, however, that in the case of a Adaptation or Collection, at
        a minimum such credit will appear, if a credit for all contributing
        authors of the Adaptation or Collection appears, then as part of these
        credits and in a manner at least as prominent as the credits for the
        other contributing authors. For the avoidance of doubt, You may only
        use the credit required by this Section for the purpose of attribution
        in the manner set out above and, by exercising Your rights under this
        License, You may not implicitly or explicitly assert or imply any
        connection with, sponsorship or endorsement by the Original Author,
        Licensor and/or Attribution Parties, as appropriate, of You or Your
        use of the Work, without the separate, express prior written
        permission of the Original Author, Licensor and/or Attribution
        Parties.
     c. Except as otherwise agreed in writing by the Licensor or as may be
        otherwise permitted by applicable law, if You Reproduce, Distribute or
        Publicly Perform the Work either by itself or as part of any
        Adaptations or Collections, You must not distort, mutilate, modify or
        take other derogatory action in relation to the Work which would be
        prejudicial to the Original Author's honor or reputation. Licensor
        agrees that in those jurisdictions (e.g. Japan), in which any exercise
        of the right granted in Section 3(b) of this License (the right to
        make Adaptations) would be deemed to be a distortion, mutilation,
        modification or other derogatory action prejudicial to the Original
        Author's honor and reputation, the Licensor will waive or not assert,
        as appropriate, this Section, to the fullest extent permitted by the
        applicable national law, to enable You to reasonably exercise Your
        right under Section 3(b) of this License (right to make Adaptations)
        but not otherwise.

    5. Representations, Warranties and Disclaimer

    UNLESS OTHERWISE MUTUALLY AGREED TO BY THE PARTIES IN WRITING, LICENSOR
    OFFERS THE WORK AS-IS AND MAKES NO REPRESENTATIONS OR WARRANTIES OF ANY
    KIND CONCERNING THE WORK, EXPRESS, IMPLIED, STATUTORY OR OTHERWISE,
    INCLUDING, WITHOUT LIMITATION, WARRANTIES OF TITLE, MERCHANTIBILITY,
    FITNESS FOR A PARTICULAR PURPOSE, NONINFRINGEMENT, OR THE ABSENCE OF
    LATENT OR OTHER DEFECTS, ACCURACY, OR THE PRESENCE OF ABSENCE OF ERRORS,
    WHETHER OR NOT DISCOVERABLE. SOME JURISDICTIONS DO NOT ALLOW THE EXCLUSION
    OF IMPLIED WARRANTIES, SO SUCH EXCLUSION MAY NOT APPLY TO YOU.

    6. Limitation on Liability. EXCEPT TO THE EXTENT REQUIRED BY APPLICABLE
    LAW, IN NO EVENT WILL LICENSOR BE LIABLE TO YOU ON ANY LEGAL THEORY FOR
    ANY SPECIAL, INCIDENTAL, CONSEQUENTIAL, PUNITIVE OR EXEMPLARY DAMAGES
    ARISING OUT OF THIS LICENSE OR THE USE OF THE WORK, EVEN IF LICENSOR HAS
    BEEN ADVISED OF THE POSSIBILITY OF SUCH DAMAGES.

    7. Termination

     a. This License and the rights granted hereunder will terminate
        automatically upon any breach by You of the terms of this License.
        Individuals or entities who have received Adaptations or Collections
        from You under this License, however, will not have their licenses
        terminated provided such individuals or entities remain in full
        compliance with those licenses. Sections 1, 2, 5, 6, 7, and 8 will
        survive any termination of this License.
     b. Subject to the above terms and conditions, the license granted here is
        perpetual (for the duration of the applicable copyright in the Work).
        Notwithstanding the above, Licensor reserves the right to release the
        Work under different license terms or to stop distributing the Work at
        any time; provided, however that any such election will not serve to
        withdraw this License (or any other license that has been, or is
        required to be, granted under the terms of this License), and this
        License will continue in full force and effect unless terminated as
        stated above.

    8. Miscellaneous

     a. Each time You Distribute or Publicly Perform the Work or a Collection,
        the Licensor offers to the recipient a license to the Work on the same
        terms and conditions as the license granted to You under this License.
     b. Each time You Distribute or Publicly Perform an Adaptation, Licensor
        offers to the recipient a license to the original Work on the same
        terms and conditions as the license granted to You under this License.
     c. If any provision of this License is invalid or unenforceable under
        applicable law, it shall not affect the validity or enforceability of
        the remainder of the terms of this License, and without further action
        by the parties to this agreement, such provision shall be reformed to
        the minimum extent necessary to make such provision valid and
        enforceable.
     d. No term or provision of this License shall be deemed waived and no
        breach consented to unless such waiver or consent shall be in writing
        and signed by the party to be charged with such waiver or consent.
     e. This License constitutes the entire agreement between the parties with
        respect to the Work licensed here. There are no understandings,
        agreements or representations with respect to the Work not specified
        here. Licensor shall not be bound by any additional provisions that
        may appear in any communication from You. This License may not be
        modified without the mutual written agreement of the Licensor and You.
     f. The rights granted under, and the subject matter referenced, in this
        License were drafted utilizing the terminology of the Berne Convention
        for the Protection of Literary and Artistic Works (as amended on
        September 28, 1979), the Rome Convention of 1961, the WIPO Copyright
        Treaty of 1996, the WIPO Performances and Phonograms Treaty of 1996
        and the Universal Copyright Convention (as revised on July 24, 1971).
        These rights and subject matter take effect in the relevant
        jurisdiction in which the License terms are sought to be enforced
        according to the corresponding provisions of the implementation of
        those treaty provisions in the applicable national law. If the
        standard suite of rights granted under applicable copyright law
        includes additional rights not granted under this License, such
        additional rights are deemed to be included in the License; this
        License is not intended to restrict the license of any rights under
        applicable law.


    Creative Commons Notice

        Creative Commons is not a party to this License, and makes no warranty
        whatsoever in connection with the Work. Creative Commons will not be
        liable to You or any party on any legal theory for any damages
        whatsoever, including without limitation any general, special,
        incidental or consequential damages arising in connection to this
        license. Notwithstanding the foregoing two (2) sentences, if Creative
        Commons has expressly identified itself as the Licensor hereunder, it
        shall have all rights and obligations of Licensor.

        Except for the limited purpose of indicating to the public that the
        Work is licensed under the CCPL, Creative Commons does not authorize
        the use by either party of the trademark "Creative Commons" or any
        related trademark or logo of Creative Commons without the prior
        written consent of Creative Commons. Any permitted use will be in
        compliance with Creative Commons' then-current trademark usage
        guidelines, as may be published on its website or otherwise made
        available upon request from time to time. For the avoidance of doubt,
        this trademark restriction does not form part of this License.

        Creative Commons may be contacted at https://creativecommons.org/.

    ===============================================================================

    The Go Programming Language
    https://golang.org/LICENSE

    Copyright (c) 2009 The Go Authors. All rights reserved.

    Redistribution and use in source and binary forms, with or without
    modification, are permitted provided that the following conditions are
    met:

       * Redistributions of source code must retain the above copyright
    notice, this list of conditions and the following disclaimer.
       * Redistributions in binary form must reproduce the above
    copyright notice, this list of conditions and the following disclaimer
    in the documentation and/or other materials provided with the
    distribution.
       * Neither the name of Google Inc. nor the names of its
    contributors may be used to endorse or promote products derived from
    this software without specific prior written permission.

    THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS
    "AS IS" AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT
    LIMITED TO, THE IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR
    A PARTICULAR PURPOSE ARE DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT
    OWNER OR CONTRIBUTORS BE LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL,
    SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT
    LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES; LOSS OF USE,
    DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON ANY
    THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT
    (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE
    OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.

    ===============================================================================

    The Rust Programming Language
    https://github.com/rust-lang/rust/blob/master/LICENSE-MIT

    Permission is hereby granted, free of charge, to any
    person obtaining a copy of this software and associated
    documentation files (the "Software"), to deal in the
    Software without restriction, including without
    limitation the rights to use, copy, modify, merge,
    publish, distribute, sublicense, and/or sell copies of
    the Software, and to permit persons to whom the Software
    is furnished to do so, subject to the following
    conditions:

    The above copyright notice and this permission notice
    shall be included in all copies or substantial portions
    of the Software.

    THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF
    ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED
    TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A
    PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT
    SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
    CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
    OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR
    IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
    DEALINGS IN THE SOFTWARE.

    ===============================================================================

    The Rust Programming Language
    https://github.com/rust-lang/rust/blob/master/LICENSE-APACHE

                                  Apache License
                            Version 2.0, January 2004
                         http://www.apache.org/licenses/

    TERMS AND CONDITIONS FOR USE, REPRODUCTION, AND DISTRIBUTION

    1. Definitions.

       "License" shall mean the terms and conditions for use, reproduction,
       and distribution as defined by Sections 1 through 9 of this document.

       "Licensor" shall mean the copyright owner or entity authorized by
       the copyright owner that is granting the License.

       "Legal Entity" shall mean the union of the acting entity and all
       other entities that control, are controlled by, or are under common
       control with that entity. For the purposes of this definition,
       "control" means (i) the power, direct or indirect, to cause the
       direction or management of such entity, whether by contract or
       otherwise, or (ii) ownership of fifty percent (50%) or more of the
       outstanding shares, or (iii) beneficial ownership of such entity.

       "You" (or "Your") shall mean an individual or Legal Entity
       exercising permissions granted by this License.

       "Source" form shall mean the preferred form for making modifications,
       including but not limited to software source code, documentation
       source, and configuration files.

       "Object" form shall mean any form resulting from mechanical
       transformation or translation of a Source form, including but
       not limited to compiled object code, generated documentation,
       and conversions to other media types.

       "Work" shall mean the work of authorship, whether in Source or
       Object form, made available under the License, as indicated by a
       copyright notice that is included in or attached to the work
       (an example is provided in the Appendix below).

       "Derivative Works" shall mean any work, whether in Source or Object
       form, that is based on (or derived from) the Work and for which the
       editorial revisions, annotations, elaborations, or other modifications
       represent, as a whole, an original work of authorship. For the purposes
       of this License, Derivative Works shall not include works that remain
       separable from, or merely link (or bind by name) to the interfaces of,
       the Work and Derivative Works thereof.

       "Contribution" shall mean any work of authorship, including
       the original version of the Work and any modifications or additions
       to that Work or Derivative Works thereof, that is intentionally
       submitted to Licensor for inclusion in the Work by the copyright owner
       or by an individual or Legal Entity authorized to submit on behalf of
       the copyright owner. For the purposes of this definition, "submitted"
       means any form of electronic, verbal, or written communication sent
       to the Licensor or its representatives, including but not limited to
       communication on electronic mailing lists, source code control systems,
       and issue tracking systems that are managed by, or on behalf of, the
       Licensor for the purpose of discussing and improving the Work, but
       excluding communication that is conspicuously marked or otherwise
       designated in writing by the copyright owner as "Not a Contribution."

       "Contributor" shall mean Licensor and any individual or Legal Entity
       on behalf of whom a Contribution has been received by Licensor and
       subsequently incorporated within the Work.

    2. Grant of Copyright License. Subject to the terms and conditions of
       this License, each Contributor hereby grants to You a perpetual,
       worldwide, non-exclusive, no-charge, royalty-free, irrevocable
       copyright license to reproduce, prepare Derivative Works of,
       publicly display, publicly perform, sublicense, and distribute the
       Work and such Derivative Works in Source or Object form.

    3. Grant of Patent License. Subject to the terms and conditions of
       this License, each Contributor hereby grants to You a perpetual,
       worldwide, non-exclusive, no-charge, royalty-free, irrevocable
       (except as stated in this section) patent license to make, have made,
       use, offer to sell, sell, import, and otherwise transfer the Work,
       where such license applies only to those patent claims licensable
       by such Contributor that are necessarily infringed by their
       Contribution(s) alone or by combination of their Contribution(s)
       with the Work to which such Contribution(s) was submitted. If You
       institute patent litigation against any entity (including a
       cross-claim or counterclaim in a lawsuit) alleging that the Work
       or a Contribution incorporated within the Work constitutes direct
       or contributory patent infringement, then any patent licenses
       granted to You under this License for that Work shall terminate
       as of the date such litigation is filed.

    4. Redistribution. You may reproduce and distribute copies of the
       Work or Derivative Works thereof in any medium, with or without
       modifications, and in Source or Object form, provided that You
       meet the following conditions:

       (a) You must give any other recipients of the Work or
           Derivative Works a copy of this License; and

       (b) You must cause any modified files to carry prominent notices
           stating that You changed the files; and

       (c) You must retain, in the Source form of any Derivative Works
           that You distribute, all copyright, patent, trademark, and
           attribution notices from the Source form of the Work,
           excluding those notices that do not pertain to any part of
           the Derivative Works; and

       (d) If the Work includes a "NOTICE" text file as part of its
           distribution, then any Derivative Works that You distribute must
           include a readable copy of the attribution notices contained
           within such NOTICE file, excluding those notices that do not
           pertain to any part of the Derivative Works, in at least one
           of the following places: within a NOTICE text file distributed
           as part of the Derivative Works; within the Source form or
           documentation, if provided along with the Derivative Works; or,
           within a display generated by the Derivative Works, if and
           wherever such third-party notices normally appear. The contents
           of the NOTICE file are for informational purposes only and
           do not modify the License. You may add Your own attribution
           notices within Derivative Works that You distribute, alongside
           or as an addendum to the NOTICE text from the Work, provided
           that such additional attribution notices cannot be construed
           as modifying the License.

       You may add Your own copyright statement to Your modifications and
       may provide additional or different license terms and conditions
       for use, reproduction, or distribution of Your modifications, or
       for any such Derivative Works as a whole, provided Your use,
       reproduction, and distribution of the Work otherwise complies with
       the conditions stated in this License.

    5. Submission of Contributions. Unless You explicitly state otherwise,
       any Contribution intentionally submitted for inclusion in the Work
       by You to the Licensor shall be under the terms and conditions of
       this License, without any additional terms or conditions.
       Notwithstanding the above, nothing herein shall supersede or modify
       the terms of any separate license agreement you may have executed
       with Licensor regarding such Contributions.

    6. Trademarks. This License does not grant permission to use the trade
       names, trademarks, service marks, or product names of the Licensor,
       except as required for reasonable and customary use in describing the
       origin of the Work and reproducing the content of the NOTICE file.

    7. Disclaimer of Warranty. Unless required by applicable law or
       agreed to in writing, Licensor provides the Work (and each
       Contributor provides its Contributions) on an "AS IS" BASIS,
       WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or
       implied, including, without limitation, any warranties or conditions
       of TITLE, NON-INFRINGEMENT, MERCHANTABILITY, or FITNESS FOR A
       PARTICULAR PURPOSE. You are solely responsible for determining the
       appropriateness of using or redistributing the Work and assume any
       risks associated with Your exercise of permissions under this License.

    8. Limitation of Liability. In no event and under no legal theory,
       whether in tort (including negligence), contract, or otherwise,
       unless required by applicable law (such as deliberate and grossly
       negligent acts) or agreed to in writing, shall any Contributor be
       liable to You for damages, including any direct, indirect, special,
       incidental, or consequential damages of any character arising as a
       result of this License or out of the use or inability to use the
       Work (including but not limited to damages for loss of goodwill,
       work stoppage, computer failure or malfunction, or any and all
       other commercial damages or losses), even if such Contributor
       has been advised of the possibility of such damages.

    9. Accepting Warranty or Additional Liability. While redistributing
       the Work or Derivative Works thereof, You may choose to offer,
       and charge a fee for, acceptance of support, warranty, indemnity,
       or other liability obligations and/or rights consistent with this
       License. However, in accepting such obligations, You may act only
       on Your own behalf and on Your sole responsibility, not on behalf
       of any other Contributor, and only if You agree to indemnify,
       defend, and hold each Contributor harmless for any liability
       incurred by, or claims asserted against, such Contributor by reason
       of your accepting any such warranty or additional liability.

    END OF TERMS AND CONDITIONS

    APPENDIX: How to apply the Apache License to your work.

       To apply the Apache License to your work, attach the following
       boilerplate notice, with the fields enclosed by brackets "[]"
       replaced with your own identifying information. (Don't include
       the brackets!)  The text should be enclosed in the appropriate
       comment syntax for the file format. We also recommend that a
       file or class name and description of purpose be included on the
       same "printed page" as the copyright notice for easier
       identification within third-party archives.

    Copyright [yyyy] [name of copyright owner]

    Licensed under the Apache License, Version 2.0 (the "License");
    you may not use this file except in compliance with the License.
    You may obtain a copy of the License at

    	http://www.apache.org/licenses/LICENSE-2.0

    Unless required by applicable law or agreed to in writing, software
    distributed under the License is distributed on an "AS IS" BASIS,
    WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
    See the License for the specific language governing permissions and
    limitations under the License.

## crossbeam-deque 0.8.7

License: MIT OR Apache-2.0. Source: https://github.com/crossbeam-rs/crossbeam

`LICENSE-APACHE` (crossbeam-deque 0.8.7): the same text as `LICENSE-APACHE` (addr2line 0.25.1) above.

`LICENSE-MIT` (crossbeam-deque 0.8.7): the same text as `LICENSE-MIT` (crossbeam 0.8.4) above.

## crossbeam-deque 0.8.8

License: MIT OR Apache-2.0. Source: https://github.com/crossbeam-rs/crossbeam

`LICENSE-APACHE` (crossbeam-deque 0.8.8): the same text as `LICENSE-APACHE` (addr2line 0.25.1) above.

`LICENSE-MIT` (crossbeam-deque 0.8.8): the same text as `LICENSE-MIT` (crossbeam 0.8.4) above.

## crossbeam-epoch 0.9.20

License: MIT OR Apache-2.0. Source: https://github.com/crossbeam-rs/crossbeam

`LICENSE-APACHE` (crossbeam-epoch 0.9.20): the same text as `LICENSE-APACHE` (addr2line 0.25.1) above.

`LICENSE-MIT` (crossbeam-epoch 0.9.20): the same text as `LICENSE-MIT` (crossbeam 0.8.4) above.

## crossbeam-epoch 0.9.21

License: MIT OR Apache-2.0. Source: https://github.com/crossbeam-rs/crossbeam

`LICENSE-APACHE` (crossbeam-epoch 0.9.21): the same text as `LICENSE-APACHE` (addr2line 0.25.1) above.

`LICENSE-MIT` (crossbeam-epoch 0.9.21): the same text as `LICENSE-MIT` (crossbeam 0.8.4) above.

## crossbeam-queue 0.3.13

License: MIT OR Apache-2.0. Source: https://github.com/crossbeam-rs/crossbeam

`LICENSE-APACHE` (crossbeam-queue 0.3.13): the same text as `LICENSE-APACHE` (addr2line 0.25.1) above.

`LICENSE-MIT` (crossbeam-queue 0.3.13): the same text as `LICENSE-MIT` (crossbeam 0.8.4) above.

## crossbeam-utils 0.8.22

License: MIT OR Apache-2.0. Source: https://github.com/crossbeam-rs/crossbeam

`LICENSE-APACHE` (crossbeam-utils 0.8.22): the same text as `LICENSE-APACHE` (addr2line 0.25.1) above.

`LICENSE-MIT` (crossbeam-utils 0.8.22): the same text as `LICENSE-MIT` (crossbeam 0.8.4) above.

## crossbeam-utils 0.8.23

License: MIT OR Apache-2.0. Source: https://github.com/crossbeam-rs/crossbeam

`LICENSE-APACHE` (crossbeam-utils 0.8.23): the same text as `LICENSE-APACHE` (addr2line 0.25.1) above.

`LICENSE-MIT` (crossbeam-utils 0.8.23): the same text as `LICENSE-MIT` (crossbeam 0.8.4) above.

## crunchy 0.2.4

License: MIT. Source: https://github.com/eira-fransham/crunchy
Authors: Eira Fransham <jackefransham@gmail.com>.

`LICENSE` (crunchy 0.2.4):

    The MIT License (MIT)

    Copyright 2017-2023 Eira Fransham.

    Permission is hereby granted, free of charge, to any person obtaining a copy
    of this software and associated documentation files (the "Software"), to deal
    in the Software without restriction, including without limitation the rights
    to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
    copies of the Software, and to permit persons to whom the Software is
    furnished to do so, subject to the following conditions:

    The above copyright notice and this permission notice shall be included in all
    copies or substantial portions of the Software.

    THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
    IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
    FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
    AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
    LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
    OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
    SOFTWARE.

## crypto-bigint 0.5.5

License: Apache-2.0 OR MIT. Source: https://github.com/RustCrypto/crypto-bigint
Authors: RustCrypto Developers.

`LICENSE-APACHE` (crypto-bigint 0.5.5): the same text as `LICENSE-APACHE` (base16ct 0.2.0) above.

`LICENSE-MIT` (crypto-bigint 0.5.5):

    Copyright (c) 2021 The RustCrypto Project Developers

    Permission is hereby granted, free of charge, to any
    person obtaining a copy of this software and associated
    documentation files (the "Software"), to deal in the
    Software without restriction, including without
    limitation the rights to use, copy, modify, merge,
    publish, distribute, sublicense, and/or sell copies of
    the Software, and to permit persons to whom the Software
    is furnished to do so, subject to the following
    conditions:

    The above copyright notice and this permission notice
    shall be included in all copies or substantial portions
    of the Software.

    THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF
    ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED
    TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A
    PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT
    SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
    CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
    OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR
    IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
    DEALINGS IN THE SOFTWARE.

## crypto-common 0.1.6

License: MIT OR Apache-2.0. Source: https://github.com/RustCrypto/traits
Authors: RustCrypto Developers.

`LICENSE-APACHE` (crypto-common 0.1.6): the same text as `LICENSE-APACHE` (base16ct 0.2.0) above.

`LICENSE-MIT` (crypto-common 0.1.6):

    Copyright (c) 2021 RustCrypto Developers

    Permission is hereby granted, free of charge, to any
    person obtaining a copy of this software and associated
    documentation files (the "Software"), to deal in the
    Software without restriction, including without
    limitation the rights to use, copy, modify, merge,
    publish, distribute, sublicense, and/or sell copies of
    the Software, and to permit persons to whom the Software
    is furnished to do so, subject to the following
    conditions:

    The above copyright notice and this permission notice
    shall be included in all copies or substantial portions
    of the Software.

    THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF
    ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED
    TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A
    PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT
    SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
    CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
    OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR
    IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
    DEALINGS IN THE SOFTWARE.

## crypto-common 0.1.7

License: MIT OR Apache-2.0. Source: https://github.com/RustCrypto/traits
Authors: RustCrypto Developers.

`LICENSE-APACHE` (crypto-common 0.1.7): the same text as `LICENSE-APACHE` (base16ct 0.2.0) above.

`LICENSE-MIT` (crypto-common 0.1.7): the same text as `LICENSE-MIT` (crypto-common 0.1.6) above.

## crypto-common 0.2.2

License: MIT OR Apache-2.0. Source: https://github.com/RustCrypto/traits
Authors: RustCrypto Developers.

`LICENSE-APACHE` (crypto-common 0.2.2): the same text as `LICENSE-APACHE` (base16ct 0.2.0) above.

`LICENSE-MIT` (crypto-common 0.2.2):

    Copyright (c) 2021-2026 RustCrypto Developers

    Permission is hereby granted, free of charge, to any
    person obtaining a copy of this software and associated
    documentation files (the "Software"), to deal in the
    Software without restriction, including without
    limitation the rights to use, copy, modify, merge,
    publish, distribute, sublicense, and/or sell copies of
    the Software, and to permit persons to whom the Software
    is furnished to do so, subject to the following
    conditions:

    The above copyright notice and this permission notice
    shall be included in all copies or substantial portions
    of the Software.

    THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF
    ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED
    TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A
    PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT
    SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
    CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
    OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR
    IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
    DEALINGS IN THE SOFTWARE.

## curve25519-dalek 4.1.3

License: BSD-3-Clause. Source: https://github.com/dalek-cryptography/curve25519-dalek/tree/main/curve25519-dalek
Authors: Isis Lovecruft <isis@patternsinthevoid.net>, Henry de Valence <hdevalence@hdevalence.ca>.

`LICENSE` (curve25519-dalek 4.1.3):

    Copyright (c) 2016-2021 isis agora lovecruft. All rights reserved.
    Copyright (c) 2016-2021 Henry de Valence. All rights reserved.

    Redistribution and use in source and binary forms, with or without
    modification, are permitted provided that the following conditions are
    met:

    1. Redistributions of source code must retain the above copyright
    notice, this list of conditions and the following disclaimer.

    2. Redistributions in binary form must reproduce the above copyright
    notice, this list of conditions and the following disclaimer in the
    documentation and/or other materials provided with the distribution.

    3. Neither the name of the copyright holder nor the names of its
    contributors may be used to endorse or promote products derived from
    this software without specific prior written permission.

    THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS
    IS" AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED
    TO, THE IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A
    PARTICULAR PURPOSE ARE DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT
    HOLDER OR CONTRIBUTORS BE LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL,
    SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT LIMITED
    TO, PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES; LOSS OF USE, DATA, OR
    PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON ANY THEORY OF
    LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT (INCLUDING
    NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE OF THIS
    SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE. 

    ========================================================================

    Portions of curve25519-dalek were originally derived from Adam Langley's
    Go ed25519 implementation, found at <https://github.com/agl/ed25519/>,
    under the following licence:

    ========================================================================

    Copyright (c) 2012 The Go Authors. All rights reserved.

    Redistribution and use in source and binary forms, with or without
    modification, are permitted provided that the following conditions are
    met:

       * Redistributions of source code must retain the above copyright
    notice, this list of conditions and the following disclaimer.
       * Redistributions in binary form must reproduce the above
    copyright notice, this list of conditions and the following disclaimer
    in the documentation and/or other materials provided with the
    distribution.
       * Neither the name of Google Inc. nor the names of its
    contributors may be used to endorse or promote products derived from
    this software without specific prior written permission.

    THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS
    IS" AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED
    TO, THE IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A
    PARTICULAR PURPOSE ARE DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT OWNER
    OR CONTRIBUTORS BE LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL,
    EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT LIMITED TO,
    PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES; LOSS OF USE, DATA, OR
    PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON ANY THEORY OF
    LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT (INCLUDING
    NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE OF THIS
    SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.

## dashmap 6.2.1

License: MIT. Source: https://github.com/xacrimon/dashmap
Authors: Acrimon <joel.wejdenstal@gmail.com>.

`LICENSE` (dashmap 6.2.1):

    MIT License

    Copyright (c) 2019 Acrimon

    Permission is hereby granted, free of charge, to any person obtaining a copy
    of this software and associated documentation files (the "Software"), to deal
    in the Software without restriction, including without limitation the rights
    to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
    copies of the Software, and to permit persons to whom the Software is
    furnished to do so, subject to the following conditions:

    The above copyright notice and this permission notice shall be included in all
    copies or substantial portions of the Software.

    THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
    IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
    FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
    AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
    LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
    OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
    SOFTWARE.

## data-encoding 2.11.1

License: MIT. Source: https://github.com/ia0/data-encoding

`LICENSE` (data-encoding 2.11.1):

    The MIT License (MIT)

    Copyright (c) 2015-2020 Julien Cretin
    Copyright (c) 2017-2020 Google Inc.

    Permission is hereby granted, free of charge, to any person obtaining a copy
    of this software and associated documentation files (the "Software"), to deal
    in the Software without restriction, including without limitation the rights
    to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
    copies of the Software, and to permit persons to whom the Software is
    furnished to do so, subject to the following conditions:

    The above copyright notice and this permission notice shall be included in all
    copies or substantial portions of the Software.

    THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
    IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
    FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
    AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
    LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
    OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
    SOFTWARE.

## deepsize2 0.1.0

License: MIT. Source: https://github.com/nhtyy/deepsize/
Authors: Aeledfyr <aeledfyr@gmail.com>, Nathan <n@push0.dev>.

`LICENSE` (deepsize2 0.1.0):

    MIT License

    Copyright (c) 2019 Aeledfyr

    Permission is hereby granted, free of charge, to any person obtaining a copy
    of this software and associated documentation files (the "Software"), to deal
    in the Software without restriction, including without limitation the rights
    to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
    copies of the Software, and to permit persons to whom the Software is
    furnished to do so, subject to the following conditions:

    The above copyright notice and this permission notice shall be included in all
    copies or substantial portions of the Software.

    THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
    IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
    FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
    AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
    LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
    OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
    SOFTWARE.

## der 0.7.10

License: Apache-2.0 OR MIT. Source: https://github.com/RustCrypto/formats/tree/master/der
Authors: RustCrypto Developers.

`LICENSE-APACHE` (der 0.7.10): the same text as `LICENSE-APACHE` (base16ct 0.2.0) above.

`LICENSE-MIT` (der 0.7.10):

    Copyright (c) 2020-2023 The RustCrypto Project Developers

    Permission is hereby granted, free of charge, to any
    person obtaining a copy of this software and associated
    documentation files (the "Software"), to deal in the
    Software without restriction, including without
    limitation the rights to use, copy, modify, merge,
    publish, distribute, sublicense, and/or sell copies of
    the Software, and to permit persons to whom the Software
    is furnished to do so, subject to the following
    conditions:

    The above copyright notice and this permission notice
    shall be included in all copies or substantial portions
    of the Software.

    THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF
    ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED
    TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A
    PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT
    SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
    CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
    OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR
    IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
    DEALINGS IN THE SOFTWARE.

## derive_more 1.0.0

License: MIT. Source: https://github.com/JelteF/derive_more
Authors: Jelte Fennema <github-tech@jeltef.nl>.

`LICENSE` (derive_more 1.0.0):

    The MIT License (MIT)

    Copyright (c) 2016 Jelte Fennema

    Permission is hereby granted, free of charge, to any person obtaining a copy
    of this software and associated documentation files (the "Software"), to deal
    in the Software without restriction, including without limitation the rights
    to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
    copies of the Software, and to permit persons to whom the Software is
    furnished to do so, subject to the following conditions:

    The above copyright notice and this permission notice shall be included in all
    copies or substantial portions of the Software.

    THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
    IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
    FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
    AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
    LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
    OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
    SOFTWARE.

## derive_more 2.1.1

License: MIT. Source: https://github.com/JelteF/derive_more
Authors: Jelte Fennema <github-tech@jeltef.nl>.

`LICENSE` (derive_more 2.1.1): the same text as `LICENSE` (derive_more 1.0.0) above.

## digest 0.10.7

License: MIT OR Apache-2.0. Source: https://github.com/RustCrypto/traits
Authors: RustCrypto Developers.

`LICENSE-APACHE` (digest 0.10.7): the same text as `LICENSE-APACHE` (base16ct 0.2.0) above.

`LICENSE-MIT` (digest 0.10.7):

    Copyright (c) 2017 Artyom Pavlov

    Permission is hereby granted, free of charge, to any
    person obtaining a copy of this software and associated
    documentation files (the "Software"), to deal in the
    Software without restriction, including without
    limitation the rights to use, copy, modify, merge,
    publish, distribute, sublicense, and/or sell copies of
    the Software, and to permit persons to whom the Software
    is furnished to do so, subject to the following
    conditions:

    The above copyright notice and this permission notice
    shall be included in all copies or substantial portions
    of the Software.

    THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF
    ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED
    TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A
    PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT
    SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
    CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
    OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR
    IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
    DEALINGS IN THE SOFTWARE.

## digest 0.11.3

License: MIT OR Apache-2.0. Source: https://github.com/RustCrypto/traits
Authors: RustCrypto Developers.

`LICENSE-APACHE` (digest 0.11.3): the same text as `LICENSE-APACHE` (base16ct 0.2.0) above.

`LICENSE-MIT` (digest 0.11.3):

    Copyright (c) 2017-2025 RustCrypto Developers
    Copyright (c) 2017 Artyom Pavlov

    Permission is hereby granted, free of charge, to any
    person obtaining a copy of this software and associated
    documentation files (the "Software"), to deal in the
    Software without restriction, including without
    limitation the rights to use, copy, modify, merge,
    publish, distribute, sublicense, and/or sell copies of
    the Software, and to permit persons to whom the Software
    is furnished to do so, subject to the following
    conditions:

    The above copyright notice and this permission notice
    shall be included in all copies or substantial portions
    of the Software.

    THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF
    ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED
    TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A
    PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT
    SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
    CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
    OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR
    IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
    DEALINGS IN THE SOFTWARE.

## digest 0.9.0

License: MIT OR Apache-2.0. Source: https://github.com/RustCrypto/traits
Authors: RustCrypto Developers.

`LICENSE-APACHE` (digest 0.9.0): the same text as `LICENSE-APACHE` (base16ct 0.2.0) above.

`LICENSE-MIT` (digest 0.9.0): the same text as `LICENSE-MIT` (digest 0.10.7) above.

## dyn-clone 1.0.20

License: MIT OR Apache-2.0. Source: https://github.com/dtolnay/dyn-clone
Authors: David Tolnay <dtolnay@gmail.com>.

`LICENSE-APACHE` (dyn-clone 1.0.20): the same text as `LICENSE-APACHE` (allocator-api2 0.2.21) above.

`LICENSE-MIT` (dyn-clone 1.0.20): the same text as `LICENSE-MIT` (adler2 2.0.1) above.

## ecdsa 0.16.9

License: Apache-2.0 OR MIT. Source: https://github.com/RustCrypto/signatures/tree/master/ecdsa
Authors: RustCrypto Developers.

`LICENSE-APACHE` (ecdsa 0.16.9):

                                  Apache License
                            Version 2.0, January 2004
                         http://www.apache.org/licenses/

    TERMS AND CONDITIONS FOR USE, REPRODUCTION, AND DISTRIBUTION

    1. Definitions.

       "License" shall mean the terms and conditions for use, reproduction,
       and distribution as defined by Sections 1 through 9 of this document.

       "Licensor" shall mean the copyright owner or entity authorized by
       the copyright owner that is granting the License.

       "Legal Entity" shall mean the union of the acting entity and all
       other entities that control, are controlled by, or are under common
       control with that entity. For the purposes of this definition,
       "control" means (i) the power, direct or indirect, to cause the
       direction or management of such entity, whether by contract or
       otherwise, or (ii) ownership of fifty percent (50%) or more of the
       outstanding shares, or (iii) beneficial ownership of such entity.

       "You" (or "Your") shall mean an individual or Legal Entity
       exercising permissions granted by this License.

       "Source" form shall mean the preferred form for making modifications,
       including but not limited to software source code, documentation
       source, and configuration files.

       "Object" form shall mean any form resulting from mechanical
       transformation or translation of a Source form, including but
       not limited to compiled object code, generated documentation,
       and conversions to other media types.

       "Work" shall mean the work of authorship, whether in Source or
       Object form, made available under the License, as indicated by a
       copyright notice that is included in or attached to the work
       (an example is provided in the Appendix below).

       "Derivative Works" shall mean any work, whether in Source or Object
       form, that is based on (or derived from) the Work and for which the
       editorial revisions, annotations, elaborations, or other modifications
       represent, as a whole, an original work of authorship. For the purposes
       of this License, Derivative Works shall not include works that remain
       separable from, or merely link (or bind by name) to the interfaces of,
       the Work and Derivative Works thereof.

       "Contribution" shall mean any work of authorship, including
       the original version of the Work and any modifications or additions
       to that Work or Derivative Works thereof, that is intentionally
       submitted to Licensor for inclusion in the Work by the copyright owner
       or by an individual or Legal Entity authorized to submit on behalf of
       the copyright owner. For the purposes of this definition, "submitted"
       means any form of electronic, verbal, or written communication sent
       to the Licensor or its representatives, including but not limited to
       communication on electronic mailing lists, source code control systems,
       and issue tracking systems that are managed by, or on behalf of, the
       Licensor for the purpose of discussing and improving the Work, but
       excluding communication that is conspicuously marked or otherwise
       designated in writing by the copyright owner as "Not a Contribution."

       "Contributor" shall mean Licensor and any individual or Legal Entity
       on behalf of whom a Contribution has been received by Licensor and
       subsequently incorporated within the Work.

    2. Grant of Copyright License. Subject to the terms and conditions of
       this License, each Contributor hereby grants to You a perpetual,
       worldwide, non-exclusive, no-charge, royalty-free, irrevocable
       copyright license to reproduce, prepare Derivative Works of,
       publicly display, publicly perform, sublicense, and distribute the
       Work and such Derivative Works in Source or Object form.

    3. Grant of Patent License. Subject to the terms and conditions of
       this License, each Contributor hereby grants to You a perpetual,
       worldwide, non-exclusive, no-charge, royalty-free, irrevocable
       (except as stated in this section) patent license to make, have made,
       use, offer to sell, sell, import, and otherwise transfer the Work,
       where such license applies only to those patent claims licensable
       by such Contributor that are necessarily infringed by their
       Contribution(s) alone or by combination of their Contribution(s)
       with the Work to which such Contribution(s) was submitted. If You
       institute patent litigation against any entity (including a
       cross-claim or counterclaim in a lawsuit) alleging that the Work
       or a Contribution incorporated within the Work constitutes direct
       or contributory patent infringement, then any patent licenses
       granted to You under this License for that Work shall terminate
       as of the date such litigation is filed.

    4. Redistribution. You may reproduce and distribute copies of the
       Work or Derivative Works thereof in any medium, with or without
       modifications, and in Source or Object form, provided that You
       meet the following conditions:

       (a) You must give any other recipients of the Work or
           Derivative Works a copy of this License; and

       (b) You must cause any modified files to carry prominent notices
           stating that You changed the files; and

       (c) You must retain, in the Source form of any Derivative Works
           that You distribute, all copyright, patent, trademark, and
           attribution notices from the Source form of the Work,
           excluding those notices that do not pertain to any part of
           the Derivative Works; and

       (d) If the Work includes a "NOTICE" text file as part of its
           distribution, then any Derivative Works that You distribute must
           include a readable copy of the attribution notices contained
           within such NOTICE file, excluding those notices that do not
           pertain to any part of the Derivative Works, in at least one
           of the following places: within a NOTICE text file distributed
           as part of the Derivative Works; within the Source form or
           documentation, if provided along with the Derivative Works; or,
           within a display generated by the Derivative Works, if and
           wherever such third-party notices normally appear. The contents
           of the NOTICE file are for informational purposes only and
           do not modify the License. You may add Your own attribution
           notices within Derivative Works that You distribute, alongside
           or as an addendum to the NOTICE text from the Work, provided
           that such additional attribution notices cannot be construed
           as modifying the License.

       You may add Your own copyright statement to Your modifications and
       may provide additional or different license terms and conditions
       for use, reproduction, or distribution of Your modifications, or
       for any such Derivative Works as a whole, provided Your use,
       reproduction, and distribution of the Work otherwise complies with
       the conditions stated in this License.

    5. Submission of Contributions. Unless You explicitly state otherwise,
       any Contribution intentionally submitted for inclusion in the Work
       by You to the Licensor shall be under the terms and conditions of
       this License, without any additional terms or conditions.
       Notwithstanding the above, nothing herein shall supersede or modify
       the terms of any separate license agreement you may have executed
       with Licensor regarding such Contributions.

    6. Trademarks. This License does not grant permission to use the trade
       names, trademarks, service marks, or product names of the Licensor,
       except as required for reasonable and customary use in describing the
       origin of the Work and reproducing the content of the NOTICE file.

    7. Disclaimer of Warranty. Unless required by applicable law or
       agreed to in writing, Licensor provides the Work (and each
       Contributor provides its Contributions) on an "AS IS" BASIS,
       WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or
       implied, including, without limitation, any warranties or conditions
       of TITLE, NON-INFRINGEMENT, MERCHANTABILITY, or FITNESS FOR A
       PARTICULAR PURPOSE. You are solely responsible for determining the
       appropriateness of using or redistributing the Work and assume any
       risks associated with Your exercise of permissions under this License.

    8. Limitation of Liability. In no event and under no legal theory,
       whether in tort (including negligence), contract, or otherwise,
       unless required by applicable law (such as deliberate and grossly
       negligent acts) or agreed to in writing, shall any Contributor be
       liable to You for damages, including any direct, indirect, special,
       incidental, or consequential damages of any character arising as a
       result of this License or out of the use or inability to use the
       Work (including but not limited to damages for loss of goodwill,
       work stoppage, computer failure or malfunction, or any and all
       other commercial damages or losses), even if such Contributor
       has been advised of the possibility of such damages.

    9. Accepting Warranty or Additional Liability. While redistributing
       the Work or Derivative Works thereof, You may choose to offer,
       and charge a fee for, acceptance of support, warranty, indemnity,
       or other liability obligations and/or rights consistent with this
       License. However, in accepting such obligations, You may act only
       on Your own behalf and on Your sole responsibility, not on behalf
       of any other Contributor, and only if You agree to indemnify,
       defend, and hold each Contributor harmless for any liability
       incurred by, or claims asserted against, such Contributor by reason
       of your accepting any such warranty or additional liability.

    END OF TERMS AND CONDITIONS

    APPENDIX: How to apply the Apache License to your work.

       To apply the Apache License to your work, attach the following
       boilerplate notice, with the fields enclosed by brackets "[]"
       replaced with your own identifying information. (Don't include
       the brackets!)  The text should be enclosed in the appropriate
       comment syntax for the file format. We also recommend that a
       file or class name and description of purpose be included on the
       same "printed page" as the copyright notice for easier
       identification within third-party archives.

    Copyright 2018-2022 RustCrypto Developers

    Licensed under the Apache License, Version 2.0 (the "License");
    you may not use this file except in compliance with the License.
    You may obtain a copy of the License at

    	http://www.apache.org/licenses/LICENSE-2.0

    Unless required by applicable law or agreed to in writing, software
    distributed under the License is distributed on an "AS IS" BASIS,
    WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
    See the License for the specific language governing permissions and
    limitations under the License.

`LICENSE-MIT` (ecdsa 0.16.9):

    Copyright (c) 2018-2022 RustCrypto Developers

    Permission is hereby granted, free of charge, to any
    person obtaining a copy of this software and associated
    documentation files (the "Software"), to deal in the
    Software without restriction, including without
    limitation the rights to use, copy, modify, merge,
    publish, distribute, sublicense, and/or sell copies of
    the Software, and to permit persons to whom the Software
    is furnished to do so, subject to the following
    conditions:

    The above copyright notice and this permission notice
    shall be included in all copies or substantial portions
    of the Software.

    THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF
    ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED
    TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A
    PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT
    SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
    CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
    OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR
    IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
    DEALINGS IN THE SOFTWARE.

## ed25519 2.2.3

License: Apache-2.0 OR MIT. Source: https://github.com/RustCrypto/signatures/tree/master/ed25519
Authors: RustCrypto Developers.

`LICENSE-APACHE` (ed25519 2.2.3): the same text as `LICENSE-APACHE` (ecdsa 0.16.9) above.

`LICENSE-MIT` (ed25519 2.2.3):

    Copyright (c) 2018-2023 RustCrypto Developers

    Permission is hereby granted, free of charge, to any
    person obtaining a copy of this software and associated
    documentation files (the "Software"), to deal in the
    Software without restriction, including without
    limitation the rights to use, copy, modify, merge,
    publish, distribute, sublicense, and/or sell copies of
    the Software, and to permit persons to whom the Software
    is furnished to do so, subject to the following
    conditions:

    The above copyright notice and this permission notice
    shall be included in all copies or substantial portions
    of the Software.

    THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF
    ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED
    TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A
    PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT
    SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
    CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
    OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR
    IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
    DEALINGS IN THE SOFTWARE.

## ed25519-dalek 2.2.0

License: BSD-3-Clause. Source: https://github.com/dalek-cryptography/curve25519-dalek/tree/main/ed25519-dalek
Authors: isis lovecruft <isis@patternsinthevoid.net>, Tony Arcieri <bascule@gmail.com>, Michael Rosenberg <michael@mrosenberg.pub>.

`LICENSE` (ed25519-dalek 2.2.0):

    Copyright (c) 2017-2019 isis agora lovecruft. All rights reserved.

    Redistribution and use in source and binary forms, with or without
    modification, are permitted provided that the following conditions are
    met:

    1. Redistributions of source code must retain the above copyright
    notice, this list of conditions and the following disclaimer.

    2. Redistributions in binary form must reproduce the above copyright
    notice, this list of conditions and the following disclaimer in the
    documentation and/or other materials provided with the distribution.

    3. Neither the name of the copyright holder nor the names of its
    contributors may be used to endorse or promote products derived from
    this software without specific prior written permission.

    THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS
    IS" AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED
    TO, THE IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A
    PARTICULAR PURPOSE ARE DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT
    HOLDER OR CONTRIBUTORS BE LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL,
    SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT LIMITED
    TO, PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES; LOSS OF USE, DATA, OR
    PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON ANY THEORY OF
    LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT (INCLUDING
    NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE OF THIS
    SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE. 

## either 1.18.0

License: MIT OR Apache-2.0. Source: https://github.com/rayon-rs/either

`LICENSE-APACHE` (either 1.18.0): the same text as `LICENSE-APACHE` (addr2line 0.25.1) above.

`LICENSE-MIT` (either 1.18.0):

    Copyright (c) 2015

    Permission is hereby granted, free of charge, to any
    person obtaining a copy of this software and associated
    documentation files (the "Software"), to deal in the
    Software without restriction, including without
    limitation the rights to use, copy, modify, merge,
    publish, distribute, sublicense, and/or sell copies of
    the Software, and to permit persons to whom the Software
    is furnished to do so, subject to the following
    conditions:

    The above copyright notice and this permission notice
    shall be included in all copies or substantial portions
    of the Software.

    THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF
    ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED
    TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A
    PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT
    SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
    CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
    OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR
    IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
    DEALINGS IN THE SOFTWARE.

## either 1.19.0

License: MIT OR Apache-2.0. Source: https://github.com/rayon-rs/either

`LICENSE-APACHE` (either 1.19.0): the same text as `LICENSE-APACHE` (addr2line 0.25.1) above.

`LICENSE-MIT` (either 1.19.0): the same text as `LICENSE-MIT` (either 1.18.0) above.

## elf 0.7.4

License: MIT/Apache-2.0. Source: https://github.com/cole14/rust-elf/
Authors: Christopher Cole <chris.cole.09@gmail.com>.

`COPYRIGHT` (elf 0.7.4):

    rust-elf is licensed under either of

     * Apache License, Version 2.0 (LICENSE-APACHE or
       http://www.apache.org/licenses/LICENSE-2.0)
     * MIT license ([LICENSE-MIT](LICENSE-MIT) or
       http://opensource.org/licenses/MIT)

    at your option.

    Unless you explicitly state otherwise, any contribution intentionally submitted
    for inclusion in the work by you, as defined in the Apache-2.0 license, shall
    be dual licensed as above, without any additional terms or conditions.

`LICENSE-APACHE` (elf 0.7.4):

    Apache License
                               Version 2.0, January 2004
                            http://www.apache.org/licenses/

       TERMS AND CONDITIONS FOR USE, REPRODUCTION, AND DISTRIBUTION

       1. Definitions.

          "License" shall mean the terms and conditions for use, reproduction,
          and distribution as defined by Sections 1 through 9 of this document.

          "Licensor" shall mean the copyright owner or entity authorized by
          the copyright owner that is granting the License.

          "Legal Entity" shall mean the union of the acting entity and all
          other entities that control, are controlled by, or are under common
          control with that entity. For the purposes of this definition,
          "control" means (i) the power, direct or indirect, to cause the
          direction or management of such entity, whether by contract or
          otherwise, or (ii) ownership of fifty percent (50%) or more of the
          outstanding shares, or (iii) beneficial ownership of such entity.

          "You" (or "Your") shall mean an individual or Legal Entity
          exercising permissions granted by this License.

          "Source" form shall mean the preferred form for making modifications,
          including but not limited to software source code, documentation
          source, and configuration files.

          "Object" form shall mean any form resulting from mechanical
          transformation or translation of a Source form, including but
          not limited to compiled object code, generated documentation,
          and conversions to other media types.

          "Work" shall mean the work of authorship, whether in Source or
          Object form, made available under the License, as indicated by a
          copyright notice that is included in or attached to the work
          (an example is provided in the Appendix below).

          "Derivative Works" shall mean any work, whether in Source or Object
          form, that is based on (or derived from) the Work and for which the
          editorial revisions, annotations, elaborations, or other modifications
          represent, as a whole, an original work of authorship. For the purposes
          of this License, Derivative Works shall not include works that remain
          separable from, or merely link (or bind by name) to the interfaces of,
          the Work and Derivative Works thereof.

          "Contribution" shall mean any work of authorship, including
          the original version of the Work and any modifications or additions
          to that Work or Derivative Works thereof, that is intentionally
          submitted to Licensor for inclusion in the Work by the copyright owner
          or by an individual or Legal Entity authorized to submit on behalf of
          the copyright owner. For the purposes of this definition, "submitted"
          means any form of electronic, verbal, or written communication sent
          to the Licensor or its representatives, including but not limited to
          communication on electronic mailing lists, source code control systems,
          and issue tracking systems that are managed by, or on behalf of, the
          Licensor for the purpose of discussing and improving the Work, but
          excluding communication that is conspicuously marked or otherwise
          designated in writing by the copyright owner as "Not a Contribution."

          "Contributor" shall mean Licensor and any individual or Legal Entity
          on behalf of whom a Contribution has been received by Licensor and
          subsequently incorporated within the Work.

       2. Grant of Copyright License. Subject to the terms and conditions of
          this License, each Contributor hereby grants to You a perpetual,
          worldwide, non-exclusive, no-charge, royalty-free, irrevocable
          copyright license to reproduce, prepare Derivative Works of,
          publicly display, publicly perform, sublicense, and distribute the
          Work and such Derivative Works in Source or Object form.

       3. Grant of Patent License. Subject to the terms and conditions of
          this License, each Contributor hereby grants to You a perpetual,
          worldwide, non-exclusive, no-charge, royalty-free, irrevocable
          (except as stated in this section) patent license to make, have made,
          use, offer to sell, sell, import, and otherwise transfer the Work,
          where such license applies only to those patent claims licensable
          by such Contributor that are necessarily infringed by their
          Contribution(s) alone or by combination of their Contribution(s)
          with the Work to which such Contribution(s) was submitted. If You
          institute patent litigation against any entity (including a
          cross-claim or counterclaim in a lawsuit) alleging that the Work
          or a Contribution incorporated within the Work constitutes direct
          or contributory patent infringement, then any patent licenses
          granted to You under this License for that Work shall terminate
          as of the date such litigation is filed.

       4. Redistribution. You may reproduce and distribute copies of the
          Work or Derivative Works thereof in any medium, with or without
          modifications, and in Source or Object form, provided that You
          meet the following conditions:

          (a) You must give any other recipients of the Work or
              Derivative Works a copy of this License; and

          (b) You must cause any modified files to carry prominent notices
              stating that You changed the files; and

          (c) You must retain, in the Source form of any Derivative Works
              that You distribute, all copyright, patent, trademark, and
              attribution notices from the Source form of the Work,
              excluding those notices that do not pertain to any part of
              the Derivative Works; and

          (d) If the Work includes a "NOTICE" text file as part of its
              distribution, then any Derivative Works that You distribute must
              include a readable copy of the attribution notices contained
              within such NOTICE file, excluding those notices that do not
              pertain to any part of the Derivative Works, in at least one
              of the following places: within a NOTICE text file distributed
              as part of the Derivative Works; within the Source form or
              documentation, if provided along with the Derivative Works; or,
              within a display generated by the Derivative Works, if and
              wherever such third-party notices normally appear. The contents
              of the NOTICE file are for informational purposes only and
              do not modify the License. You may add Your own attribution
              notices within Derivative Works that You distribute, alongside
              or as an addendum to the NOTICE text from the Work, provided
              that such additional attribution notices cannot be construed
              as modifying the License.

          You may add Your own copyright statement to Your modifications and
          may provide additional or different license terms and conditions
          for use, reproduction, or distribution of Your modifications, or
          for any such Derivative Works as a whole, provided Your use,
          reproduction, and distribution of the Work otherwise complies with
          the conditions stated in this License.

       5. Submission of Contributions. Unless You explicitly state otherwise,
          any Contribution intentionally submitted for inclusion in the Work
          by You to the Licensor shall be under the terms and conditions of
          this License, without any additional terms or conditions.
          Notwithstanding the above, nothing herein shall supersede or modify
          the terms of any separate license agreement you may have executed
          with Licensor regarding such Contributions.

       6. Trademarks. This License does not grant permission to use the trade
          names, trademarks, service marks, or product names of the Licensor,
          except as required for reasonable and customary use in describing the
          origin of the Work and reproducing the content of the NOTICE file.

       7. Disclaimer of Warranty. Unless required by applicable law or
          agreed to in writing, Licensor provides the Work (and each
          Contributor provides its Contributions) on an "AS IS" BASIS,
          WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or
          implied, including, without limitation, any warranties or conditions
          of TITLE, NON-INFRINGEMENT, MERCHANTABILITY, or FITNESS FOR A
          PARTICULAR PURPOSE. You are solely responsible for determining the
          appropriateness of using or redistributing the Work and assume any
          risks associated with Your exercise of permissions under this License.

       8. Limitation of Liability. In no event and under no legal theory,
          whether in tort (including negligence), contract, or otherwise,
          unless required by applicable law (such as deliberate and grossly
          negligent acts) or agreed to in writing, shall any Contributor be
          liable to You for damages, including any direct, indirect, special,
          incidental, or consequential damages of any character arising as a
          result of this License or out of the use or inability to use the
          Work (including but not limited to damages for loss of goodwill,
          work stoppage, computer failure or malfunction, or any and all
          other commercial damages or losses), even if such Contributor
          has been advised of the possibility of such damages.

       9. Accepting Warranty or Additional Liability. While redistributing
          the Work or Derivative Works thereof, You may choose to offer,
          and charge a fee for, acceptance of support, warranty, indemnity,
          or other liability obligations and/or rights consistent with this
          License. However, in accepting such obligations, You may act only
          on Your own behalf and on Your sole responsibility, not on behalf
          of any other Contributor, and only if You agree to indemnify,
          defend, and hold each Contributor harmless for any liability
          incurred by, or claims asserted against, such Contributor by reason
          of your accepting any such warranty or additional liability.

       END OF TERMS AND CONDITIONS

       APPENDIX: How to apply the Apache License to your work.

          To apply the Apache License to your work, attach the following
          boilerplate notice, with the fields enclosed by brackets "{}"
          replaced with your own identifying information. (Don't include
          the brackets!)  The text should be enclosed in the appropriate
          comment syntax for the file format. We also recommend that a
          file or class name and description of purpose be included on the
          same "printed page" as the copyright notice for easier
          identification within third-party archives.

       Copyright 2016 Christopher Cole

       Licensed under the Apache License, Version 2.0 (the "License");
       you may not use this file except in compliance with the License.
       You may obtain a copy of the License at

           http://www.apache.org/licenses/LICENSE-2.0

       Unless required by applicable law or agreed to in writing, software
       distributed under the License is distributed on an "AS IS" BASIS,
       WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
       See the License for the specific language governing permissions and
       limitations under the License.

`LICENSE-MIT` (elf 0.7.4):

    Copyright (c) 2016 Christopher Cole

    Permission is hereby granted, free of charge, to any
    person obtaining a copy of this software and associated
    documentation files (the "Software"), to deal in the
    Software without restriction, including without
    limitation the rights to use, copy, modify, merge,
    publish, distribute, sublicense, and/or sell copies of
    the Software, and to permit persons to whom the Software
    is furnished to do so, subject to the following
    conditions:

    The above copyright notice and this permission notice
    shall be included in all copies or substantial portions
    of the Software.

    THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF
    ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED
    TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A
    PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT
    SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
    CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
    OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR
    IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
    DEALINGS IN THE SOFTWARE.

## elliptic-curve 0.13.8

License: Apache-2.0 OR MIT. Source: https://github.com/RustCrypto/traits/tree/master/elliptic-curve
Authors: RustCrypto Developers.

`LICENSE-APACHE` (elliptic-curve 0.13.8): the same text as `LICENSE-APACHE` (base16ct 0.2.0) above.

`LICENSE-MIT` (elliptic-curve 0.13.8):

    Copyright (c) 2020-2022 RustCrypto Developers

    Permission is hereby granted, free of charge, to any
    person obtaining a copy of this software and associated
    documentation files (the "Software"), to deal in the
    Software without restriction, including without
    limitation the rights to use, copy, modify, merge,
    publish, distribute, sublicense, and/or sell copies of
    the Software, and to permit persons to whom the Software
    is furnished to do so, subject to the following
    conditions:

    The above copyright notice and this permission notice
    shall be included in all copies or substantial portions
    of the Software.

    THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF
    ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED
    TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A
    PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT
    SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
    CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
    OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR
    IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
    DEALINGS IN THE SOFTWARE.

## endian-type 0.1.2

License: MIT. Source: https://github.com/Lolirofle/endian-type.git
Authors: Lolirofle <lolipopple@hotmail.com>.

`LICENSE` (endian-type 0.1.2):

    The MIT License (MIT)

    Copyright (c) 2015 

    Permission is hereby granted, free of charge, to any person obtaining a copy
    of this software and associated documentation files (the "Software"), to deal
    in the Software without restriction, including without limitation the rights
    to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
    copies of the Software, and to permit persons to whom the Software is
    furnished to do so, subject to the following conditions:

    The above copyright notice and this permission notice shall be included in all
    copies or substantial portions of the Software.

    THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
    IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
    FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
    AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
    LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
    OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
    SOFTWARE.

## equivalent 1.0.3

License: Apache-2.0 OR MIT. Source: https://github.com/indexmap-rs/equivalent

`LICENSE-APACHE` (equivalent 1.0.3): the same text as `LICENSE-APACHE` (addr2line 0.25.1) above.

`LICENSE-MIT` (equivalent 1.0.3):

    Copyright (c) 2016--2023

    Permission is hereby granted, free of charge, to any
    person obtaining a copy of this software and associated
    documentation files (the "Software"), to deal in the
    Software without restriction, including without
    limitation the rights to use, copy, modify, merge,
    publish, distribute, sublicense, and/or sell copies of
    the Software, and to permit persons to whom the Software
    is furnished to do so, subject to the following
    conditions:

    The above copyright notice and this permission notice
    shall be included in all copies or substantial portions
    of the Software.

    THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF
    ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED
    TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A
    PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT
    SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
    CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
    OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR
    IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
    DEALINGS IN THE SOFTWARE.

## escape-bytes 0.1.1

License: Apache-2.0. Source: https://github.com/stellar/escape-bytes
Authors: Stellar Development Foundation <info@stellar.org>.

`LICENSE` (escape-bytes 0.1.1):

                                     Apache License
                               Version 2.0, January 2004
                            http://www.apache.org/licenses/

       TERMS AND CONDITIONS FOR USE, REPRODUCTION, AND DISTRIBUTION

       1. Definitions.

          "License" shall mean the terms and conditions for use, reproduction,
          and distribution as defined by Sections 1 through 9 of this document.

          "Licensor" shall mean the copyright owner or entity authorized by
          the copyright owner that is granting the License.

          "Legal Entity" shall mean the union of the acting entity and all
          other entities that control, are controlled by, or are under common
          control with that entity. For the purposes of this definition,
          "control" means (i) the power, direct or indirect, to cause the
          direction or management of such entity, whether by contract or
          otherwise, or (ii) ownership of fifty percent (50%) or more of the
          outstanding shares, or (iii) beneficial ownership of such entity.

          "You" (or "Your") shall mean an individual or Legal Entity
          exercising permissions granted by this License.

          "Source" form shall mean the preferred form for making modifications,
          including but not limited to software source code, documentation
          source, and configuration files.

          "Object" form shall mean any form resulting from mechanical
          transformation or translation of a Source form, including but
          not limited to compiled object code, generated documentation,
          and conversions to other media types.

          "Work" shall mean the work of authorship, whether in Source or
          Object form, made available under the License, as indicated by a
          copyright notice that is included in or attached to the work
          (an example is provided in the Appendix below).

          "Derivative Works" shall mean any work, whether in Source or Object
          form, that is based on (or derived from) the Work and for which the
          editorial revisions, annotations, elaborations, or other modifications
          represent, as a whole, an original work of authorship. For the purposes
          of this License, Derivative Works shall not include works that remain
          separable from, or merely link (or bind by name) to the interfaces of,
          the Work and Derivative Works thereof.

          "Contribution" shall mean any work of authorship, including
          the original version of the Work and any modifications or additions
          to that Work or Derivative Works thereof, that is intentionally
          submitted to Licensor for inclusion in the Work by the copyright owner
          or by an individual or Legal Entity authorized to submit on behalf of
          the copyright owner. For the purposes of this definition, "submitted"
          means any form of electronic, verbal, or written communication sent
          to the Licensor or its representatives, including but not limited to
          communication on electronic mailing lists, source code control systems,
          and issue tracking systems that are managed by, or on behalf of, the
          Licensor for the purpose of discussing and improving the Work, but
          excluding communication that is conspicuously marked or otherwise
          designated in writing by the copyright owner as "Not a Contribution."

          "Contributor" shall mean Licensor and any individual or Legal Entity
          on behalf of whom a Contribution has been received by Licensor and
          subsequently incorporated within the Work.

       2. Grant of Copyright License. Subject to the terms and conditions of
          this License, each Contributor hereby grants to You a perpetual,
          worldwide, non-exclusive, no-charge, royalty-free, irrevocable
          copyright license to reproduce, prepare Derivative Works of,
          publicly display, publicly perform, sublicense, and distribute the
          Work and such Derivative Works in Source or Object form.

       3. Grant of Patent License. Subject to the terms and conditions of
          this License, each Contributor hereby grants to You a perpetual,
          worldwide, non-exclusive, no-charge, royalty-free, irrevocable
          (except as stated in this section) patent license to make, have made,
          use, offer to sell, sell, import, and otherwise transfer the Work,
          where such license applies only to those patent claims licensable
          by such Contributor that are necessarily infringed by their
          Contribution(s) alone or by combination of their Contribution(s)
          with the Work to which such Contribution(s) was submitted. If You
          institute patent litigation against any entity (including a
          cross-claim or counterclaim in a lawsuit) alleging that the Work
          or a Contribution incorporated within the Work constitutes direct
          or contributory patent infringement, then any patent licenses
          granted to You under this License for that Work shall terminate
          as of the date such litigation is filed.

       4. Redistribution. You may reproduce and distribute copies of the
          Work or Derivative Works thereof in any medium, with or without
          modifications, and in Source or Object form, provided that You
          meet the following conditions:

          (a) You must give any other recipients of the Work or
              Derivative Works a copy of this License; and

          (b) You must cause any modified files to carry prominent notices
              stating that You changed the files; and

          (c) You must retain, in the Source form of any Derivative Works
              that You distribute, all copyright, patent, trademark, and
              attribution notices from the Source form of the Work,
              excluding those notices that do not pertain to any part of
              the Derivative Works; and

          (d) If the Work includes a "NOTICE" text file as part of its
              distribution, then any Derivative Works that You distribute must
              include a readable copy of the attribution notices contained
              within such NOTICE file, excluding those notices that do not
              pertain to any part of the Derivative Works, in at least one
              of the following places: within a NOTICE text file distributed
              as part of the Derivative Works; within the Source form or
              documentation, if provided along with the Derivative Works; or,
              within a display generated by the Derivative Works, if and
              wherever such third-party notices normally appear. The contents
              of the NOTICE file are for informational purposes only and
              do not modify the License. You may add Your own attribution
              notices within Derivative Works that You distribute, alongside
              or as an addendum to the NOTICE text from the Work, provided
              that such additional attribution notices cannot be construed
              as modifying the License.

          You may add Your own copyright statement to Your modifications and
          may provide additional or different license terms and conditions
          for use, reproduction, or distribution of Your modifications, or
          for any such Derivative Works as a whole, provided Your use,
          reproduction, and distribution of the Work otherwise complies with
          the conditions stated in this License.

       5. Submission of Contributions. Unless You explicitly state otherwise,
          any Contribution intentionally submitted for inclusion in the Work
          by You to the Licensor shall be under the terms and conditions of
          this License, without any additional terms or conditions.
          Notwithstanding the above, nothing herein shall supersede or modify
          the terms of any separate license agreement you may have executed
          with Licensor regarding such Contributions.

       6. Trademarks. This License does not grant permission to use the trade
          names, trademarks, service marks, or product names of the Licensor,
          except as required for reasonable and customary use in describing the
          origin of the Work and reproducing the content of the NOTICE file.

       7. Disclaimer of Warranty. Unless required by applicable law or
          agreed to in writing, Licensor provides the Work (and each
          Contributor provides its Contributions) on an "AS IS" BASIS,
          WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or
          implied, including, without limitation, any warranties or conditions
          of TITLE, NON-INFRINGEMENT, MERCHANTABILITY, or FITNESS FOR A
          PARTICULAR PURPOSE. You are solely responsible for determining the
          appropriateness of using or redistributing the Work and assume any
          risks associated with Your exercise of permissions under this License.

       8. Limitation of Liability. In no event and under no legal theory,
          whether in tort (including negligence), contract, or otherwise,
          unless required by applicable law (such as deliberate and grossly
          negligent acts) or agreed to in writing, shall any Contributor be
          liable to You for damages, including any direct, indirect, special,
          incidental, or consequential damages of any character arising as a
          result of this License or out of the use or inability to use the
          Work (including but not limited to damages for loss of goodwill,
          work stoppage, computer failure or malfunction, or any and all
          other commercial damages or losses), even if such Contributor
          has been advised of the possibility of such damages.

       9. Accepting Warranty or Additional Liability. While redistributing
          the Work or Derivative Works thereof, You may choose to offer,
          and charge a fee for, acceptance of support, warranty, indemnity,
          or other liability obligations and/or rights consistent with this
          License. However, in accepting such obligations, You may act only
          on Your own behalf and on Your sole responsibility, not on behalf
          of any other Contributor, and only if You agree to indemnify,
          defend, and hold each Contributor harmless for any liability
          incurred by, or claims asserted against, such Contributor by reason
          of your accepting any such warranty or additional liability.

       END OF TERMS AND CONDITIONS

       APPENDIX: How to apply the Apache License to your work.

          To apply the Apache License to your work, attach the following
          boilerplate notice, with the fields enclosed by brackets "[]"
          replaced with your own identifying information. (Don't include
          the brackets!)  The text should be enclosed in the appropriate
          comment syntax for the file format. We also recommend that a
          file or class name and description of purpose be included on the
          same "printed page" as the copyright notice for easier
          identification within third-party archives.

       Copyright 2023 Stellar Development Foundation

       Licensed under the Apache License, Version 2.0 (the "License");
       you may not use this file except in compliance with the License.
       You may obtain a copy of the License at

           http://www.apache.org/licenses/LICENSE-2.0

       Unless required by applicable law or agreed to in writing, software
       distributed under the License is distributed on an "AS IS" BASIS,
       WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
       See the License for the specific language governing permissions and
       limitations under the License.

## ethnum 1.5.3

License: MIT OR Apache-2.0. Source: https://github.com/nlordell/ethnum-rs
Authors: Nicholas Rodrigues Lordello <nlordell@gmail.com>.

`LICENSE-APACHE` (ethnum 1.5.3):

                                     Apache License
                               Version 2.0, January 2004
                            http://www.apache.org/licenses/

       TERMS AND CONDITIONS FOR USE, REPRODUCTION, AND DISTRIBUTION

       1. Definitions.

          "License" shall mean the terms and conditions for use, reproduction,
          and distribution as defined by Sections 1 through 9 of this document.

          "Licensor" shall mean the copyright owner or entity authorized by
          the copyright owner that is granting the License.

          "Legal Entity" shall mean the union of the acting entity and all
          other entities that control, are controlled by, or are under common
          control with that entity. For the purposes of this definition,
          "control" means (i) the power, direct or indirect, to cause the
          direction or management of such entity, whether by contract or
          otherwise, or (ii) ownership of fifty percent (50%) or more of the
          outstanding shares, or (iii) beneficial ownership of such entity.

          "You" (or "Your") shall mean an individual or Legal Entity
          exercising permissions granted by this License.

          "Source" form shall mean the preferred form for making modifications,
          including but not limited to software source code, documentation
          source, and configuration files.

          "Object" form shall mean any form resulting from mechanical
          transformation or translation of a Source form, including but
          not limited to compiled object code, generated documentation,
          and conversions to other media types.

          "Work" shall mean the work of authorship, whether in Source or
          Object form, made available under the License, as indicated by a
          copyright notice that is included in or attached to the work
          (an example is provided in the Appendix below).

          "Derivative Works" shall mean any work, whether in Source or Object
          form, that is based on (or derived from) the Work and for which the
          editorial revisions, annotations, elaborations, or other modifications
          represent, as a whole, an original work of authorship. For the purposes
          of this License, Derivative Works shall not include works that remain
          separable from, or merely link (or bind by name) to the interfaces of,
          the Work and Derivative Works thereof.

          "Contribution" shall mean any work of authorship, including
          the original version of the Work and any modifications or additions
          to that Work or Derivative Works thereof, that is intentionally
          submitted to Licensor for inclusion in the Work by the copyright owner
          or by an individual or Legal Entity authorized to submit on behalf of
          the copyright owner. For the purposes of this definition, "submitted"
          means any form of electronic, verbal, or written communication sent
          to the Licensor or its representatives, including but not limited to
          communication on electronic mailing lists, source code control systems,
          and issue tracking systems that are managed by, or on behalf of, the
          Licensor for the purpose of discussing and improving the Work, but
          excluding communication that is conspicuously marked or otherwise
          designated in writing by the copyright owner as "Not a Contribution."

          "Contributor" shall mean Licensor and any individual or Legal Entity
          on behalf of whom a Contribution has been received by Licensor and
          subsequently incorporated within the Work.

       2. Grant of Copyright License. Subject to the terms and conditions of
          this License, each Contributor hereby grants to You a perpetual,
          worldwide, non-exclusive, no-charge, royalty-free, irrevocable
          copyright license to reproduce, prepare Derivative Works of,
          publicly display, publicly perform, sublicense, and distribute the
          Work and such Derivative Works in Source or Object form.

       3. Grant of Patent License. Subject to the terms and conditions of
          this License, each Contributor hereby grants to You a perpetual,
          worldwide, non-exclusive, no-charge, royalty-free, irrevocable
          (except as stated in this section) patent license to make, have made,
          use, offer to sell, sell, import, and otherwise transfer the Work,
          where such license applies only to those patent claims licensable
          by such Contributor that are necessarily infringed by their
          Contribution(s) alone or by combination of their Contribution(s)
          with the Work to which such Contribution(s) was submitted. If You
          institute patent litigation against any entity (including a
          cross-claim or counterclaim in a lawsuit) alleging that the Work
          or a Contribution incorporated within the Work constitutes direct
          or contributory patent infringement, then any patent licenses
          granted to You under this License for that Work shall terminate
          as of the date such litigation is filed.

       4. Redistribution. You may reproduce and distribute copies of the
          Work or Derivative Works thereof in any medium, with or without
          modifications, and in Source or Object form, provided that You
          meet the following conditions:

          (a) You must give any other recipients of the Work or
              Derivative Works a copy of this License; and

          (b) You must cause any modified files to carry prominent notices
              stating that You changed the files; and

          (c) You must retain, in the Source form of any Derivative Works
              that You distribute, all copyright, patent, trademark, and
              attribution notices from the Source form of the Work,
              excluding those notices that do not pertain to any part of
              the Derivative Works; and

          (d) If the Work includes a "NOTICE" text file as part of its
              distribution, then any Derivative Works that You distribute must
              include a readable copy of the attribution notices contained
              within such NOTICE file, excluding those notices that do not
              pertain to any part of the Derivative Works, in at least one
              of the following places: within a NOTICE text file distributed
              as part of the Derivative Works; within the Source form or
              documentation, if provided along with the Derivative Works; or,
              within a display generated by the Derivative Works, if and
              wherever such third-party notices normally appear. The contents
              of the NOTICE file are for informational purposes only and
              do not modify the License. You may add Your own attribution
              notices within Derivative Works that You distribute, alongside
              or as an addendum to the NOTICE text from the Work, provided
              that such additional attribution notices cannot be construed
              as modifying the License.

          You may add Your own copyright statement to Your modifications and
          may provide additional or different license terms and conditions
          for use, reproduction, or distribution of Your modifications, or
          for any such Derivative Works as a whole, provided Your use,
          reproduction, and distribution of the Work otherwise complies with
          the conditions stated in this License.

       5. Submission of Contributions. Unless You explicitly state otherwise,
          any Contribution intentionally submitted for inclusion in the Work
          by You to the Licensor shall be under the terms and conditions of
          this License, without any additional terms or conditions.
          Notwithstanding the above, nothing herein shall supersede or modify
          the terms of any separate license agreement you may have executed
          with Licensor regarding such Contributions.

       6. Trademarks. This License does not grant permission to use the trade
          names, trademarks, service marks, or product names of the Licensor,
          except as required for reasonable and customary use in describing the
          origin of the Work and reproducing the content of the NOTICE file.

       7. Disclaimer of Warranty. Unless required by applicable law or
          agreed to in writing, Licensor provides the Work (and each
          Contributor provides its Contributions) on an "AS IS" BASIS,
          WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or
          implied, including, without limitation, any warranties or conditions
          of TITLE, NON-INFRINGEMENT, MERCHANTABILITY, or FITNESS FOR A
          PARTICULAR PURPOSE. You are solely responsible for determining the
          appropriateness of using or redistributing the Work and assume any
          risks associated with Your exercise of permissions under this License.

       8. Limitation of Liability. In no event and under no legal theory,
          whether in tort (including negligence), contract, or otherwise,
          unless required by applicable law (such as deliberate and grossly
          negligent acts) or agreed to in writing, shall any Contributor be
          liable to You for damages, including any direct, indirect, special,
          incidental, or consequential damages of any character arising as a
          result of this License or out of the use or inability to use the
          Work (including but not limited to damages for loss of goodwill,
          work stoppage, computer failure or malfunction, or any and all
          other commercial damages or losses), even if such Contributor
          has been advised of the possibility of such damages.

       9. Accepting Warranty or Additional Liability. While redistributing
          the Work or Derivative Works thereof, You may choose to offer,
          and charge a fee for, acceptance of support, warranty, indemnity,
          or other liability obligations and/or rights consistent with this
          License. However, in accepting such obligations, You may act only
          on Your own behalf and on Your sole responsibility, not on behalf
          of any other Contributor, and only if You agree to indemnify,
          defend, and hold each Contributor harmless for any liability
          incurred by, or claims asserted against, such Contributor by reason
          of your accepting any such warranty or additional liability.

       END OF TERMS AND CONDITIONS

`LICENSE-MIT` (ethnum 1.5.3):

    Copyright (c) 2020-2022 Nicholas Rodrigues Lordello

    Permission is hereby granted, free of charge, to any
    person obtaining a copy of this software and associated
    documentation files (the "Software"), to deal in the
    Software without restriction, including without
    limitation the rights to use, copy, modify, merge,
    publish, distribute, sublicense, and/or sell copies of
    the Software, and to permit persons to whom the Software
    is furnished to do so, subject to the following
    conditions:

    The above copyright notice and this permission notice
    shall be included in all copies or substantial portions
    of the Software.

    THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF
    ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED
    TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A
    PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT
    SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
    CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
    OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR
    IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
    DEALINGS IN THE SOFTWARE.

## eyre 0.6.14

License: MIT OR Apache-2.0. Source: https://github.com/eyre-rs/eyre
Authors: David Tolnay <dtolnay@gmail.com>, Jane Lusby <jlusby42@gmail.com>.

`LICENSE-APACHE` (eyre 0.6.14): the same text as `LICENSE-APACHE` (addr2line 0.25.1) above.

`LICENSE-MIT` (eyre 0.6.14): the same text as `LICENSE-MIT` (adler2 2.0.1) above.

## ff 0.12.1

License: MIT/Apache-2.0. Source: https://github.com/zkcrypto/ff
Authors: Sean Bowe <ewillbefull@gmail.com>, Jack Grigg <thestr4d@gmail.com>.

`LICENSE-APACHE` (ff 0.12.1): the same text as `LICENSE-APACHE` (addr2line 0.25.1) above.

`LICENSE-MIT` (ff 0.12.1):

    The MIT License (MIT)

    Copyright (c) 2017 Sean Bowe

    Permission is hereby granted, free of charge, to any person obtaining a copy
    of this software and associated documentation files (the "Software"), to deal
    in the Software without restriction, including without limitation the rights
    to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
    copies of the Software, and to permit persons to whom the Software is
    furnished to do so, subject to the following conditions:

    The above copyright notice and this permission notice shall be included in
    all copies or substantial portions of the Software.

    THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
    IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
    FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
    AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
    LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
    OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN
    THE SOFTWARE.

## ff 0.13.1

License: MIT/Apache-2.0. Source: https://github.com/zkcrypto/ff
Authors: Sean Bowe <ewillbefull@gmail.com>, Jack Grigg <thestr4d@gmail.com>.

`LICENSE-APACHE` (ff 0.13.1): the same text as `LICENSE-APACHE` (addr2line 0.25.1) above.

`LICENSE-MIT` (ff 0.13.1): the same text as `LICENSE-MIT` (ff 0.12.1) above.

## foldhash 0.2.0

License: Zlib. Source: https://github.com/orlp/foldhash
Authors: Orson Peters <orsonpeters@gmail.com>.

`LICENSE` (foldhash 0.2.0):

    Copyright (c) 2024 Orson Peters

    This software is provided 'as-is', without any express or implied warranty. In
    no event will the authors be held liable for any damages arising from the use of
    this software.

    Permission is granted to anyone to use this software for any purpose, including
    commercial applications, and to alter it and redistribute it freely, subject to
    the following restrictions:

    1. The origin of this software must not be misrepresented; you must not claim
        that you wrote the original software. If you use this software in a product,
        an acknowledgment in the product documentation would be appreciated but is
        not required.

    2. Altered source versions must be plainly marked as such, and must not be
        misrepresented as being the original software.

    3. This notice may not be removed or altered from any source distribution.

## funty 2.0.0

License: MIT. Source: https://github.com/myrrlyn/funty
Authors: myrrlyn <self@myrrlyn.dev>.

`LICENSE.txt` (funty 2.0.0):

    MIT License

    Copyright (c) 2020 myrrlyn (Alexander Payne)

    Permission is hereby granted, free of charge, to any person obtaining a copy
    of this software and associated documentation files (the "Software"), to deal
    in the Software without restriction, including without limitation the rights
    to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
    copies of the Software, and to permit persons to whom the Software is
    furnished to do so, subject to the following conditions:

    The above copyright notice and this permission notice shall be included in all
    copies or substantial portions of the Software.

    THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
    IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
    FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
    AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
    LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
    OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
    SOFTWARE.

## futures 0.3.34

License: MIT OR Apache-2.0. Source: https://github.com/rust-lang/futures-rs

`LICENSE-APACHE` (futures 0.3.34):

                                  Apache License
                            Version 2.0, January 2004
                         http://www.apache.org/licenses/

    TERMS AND CONDITIONS FOR USE, REPRODUCTION, AND DISTRIBUTION

    1. Definitions.

       "License" shall mean the terms and conditions for use, reproduction,
       and distribution as defined by Sections 1 through 9 of this document.

       "Licensor" shall mean the copyright owner or entity authorized by
       the copyright owner that is granting the License.

       "Legal Entity" shall mean the union of the acting entity and all
       other entities that control, are controlled by, or are under common
       control with that entity. For the purposes of this definition,
       "control" means (i) the power, direct or indirect, to cause the
       direction or management of such entity, whether by contract or
       otherwise, or (ii) ownership of fifty percent (50%) or more of the
       outstanding shares, or (iii) beneficial ownership of such entity.

       "You" (or "Your") shall mean an individual or Legal Entity
       exercising permissions granted by this License.

       "Source" form shall mean the preferred form for making modifications,
       including but not limited to software source code, documentation
       source, and configuration files.

       "Object" form shall mean any form resulting from mechanical
       transformation or translation of a Source form, including but
       not limited to compiled object code, generated documentation,
       and conversions to other media types.

       "Work" shall mean the work of authorship, whether in Source or
       Object form, made available under the License, as indicated by a
       copyright notice that is included in or attached to the work
       (an example is provided in the Appendix below).

       "Derivative Works" shall mean any work, whether in Source or Object
       form, that is based on (or derived from) the Work and for which the
       editorial revisions, annotations, elaborations, or other modifications
       represent, as a whole, an original work of authorship. For the purposes
       of this License, Derivative Works shall not include works that remain
       separable from, or merely link (or bind by name) to the interfaces of,
       the Work and Derivative Works thereof.

       "Contribution" shall mean any work of authorship, including
       the original version of the Work and any modifications or additions
       to that Work or Derivative Works thereof, that is intentionally
       submitted to Licensor for inclusion in the Work by the copyright owner
       or by an individual or Legal Entity authorized to submit on behalf of
       the copyright owner. For the purposes of this definition, "submitted"
       means any form of electronic, verbal, or written communication sent
       to the Licensor or its representatives, including but not limited to
       communication on electronic mailing lists, source code control systems,
       and issue tracking systems that are managed by, or on behalf of, the
       Licensor for the purpose of discussing and improving the Work, but
       excluding communication that is conspicuously marked or otherwise
       designated in writing by the copyright owner as "Not a Contribution."

       "Contributor" shall mean Licensor and any individual or Legal Entity
       on behalf of whom a Contribution has been received by Licensor and
       subsequently incorporated within the Work.

    2. Grant of Copyright License. Subject to the terms and conditions of
       this License, each Contributor hereby grants to You a perpetual,
       worldwide, non-exclusive, no-charge, royalty-free, irrevocable
       copyright license to reproduce, prepare Derivative Works of,
       publicly display, publicly perform, sublicense, and distribute the
       Work and such Derivative Works in Source or Object form.

    3. Grant of Patent License. Subject to the terms and conditions of
       this License, each Contributor hereby grants to You a perpetual,
       worldwide, non-exclusive, no-charge, royalty-free, irrevocable
       (except as stated in this section) patent license to make, have made,
       use, offer to sell, sell, import, and otherwise transfer the Work,
       where such license applies only to those patent claims licensable
       by such Contributor that are necessarily infringed by their
       Contribution(s) alone or by combination of their Contribution(s)
       with the Work to which such Contribution(s) was submitted. If You
       institute patent litigation against any entity (including a
       cross-claim or counterclaim in a lawsuit) alleging that the Work
       or a Contribution incorporated within the Work constitutes direct
       or contributory patent infringement, then any patent licenses
       granted to You under this License for that Work shall terminate
       as of the date such litigation is filed.

    4. Redistribution. You may reproduce and distribute copies of the
       Work or Derivative Works thereof in any medium, with or without
       modifications, and in Source or Object form, provided that You
       meet the following conditions:

       (a) You must give any other recipients of the Work or
           Derivative Works a copy of this License; and

       (b) You must cause any modified files to carry prominent notices
           stating that You changed the files; and

       (c) You must retain, in the Source form of any Derivative Works
           that You distribute, all copyright, patent, trademark, and
           attribution notices from the Source form of the Work,
           excluding those notices that do not pertain to any part of
           the Derivative Works; and

       (d) If the Work includes a "NOTICE" text file as part of its
           distribution, then any Derivative Works that You distribute must
           include a readable copy of the attribution notices contained
           within such NOTICE file, excluding those notices that do not
           pertain to any part of the Derivative Works, in at least one
           of the following places: within a NOTICE text file distributed
           as part of the Derivative Works; within the Source form or
           documentation, if provided along with the Derivative Works; or,
           within a display generated by the Derivative Works, if and
           wherever such third-party notices normally appear. The contents
           of the NOTICE file are for informational purposes only and
           do not modify the License. You may add Your own attribution
           notices within Derivative Works that You distribute, alongside
           or as an addendum to the NOTICE text from the Work, provided
           that such additional attribution notices cannot be construed
           as modifying the License.

       You may add Your own copyright statement to Your modifications and
       may provide additional or different license terms and conditions
       for use, reproduction, or distribution of Your modifications, or
       for any such Derivative Works as a whole, provided Your use,
       reproduction, and distribution of the Work otherwise complies with
       the conditions stated in this License.

    5. Submission of Contributions. Unless You explicitly state otherwise,
       any Contribution intentionally submitted for inclusion in the Work
       by You to the Licensor shall be under the terms and conditions of
       this License, without any additional terms or conditions.
       Notwithstanding the above, nothing herein shall supersede or modify
       the terms of any separate license agreement you may have executed
       with Licensor regarding such Contributions.

    6. Trademarks. This License does not grant permission to use the trade
       names, trademarks, service marks, or product names of the Licensor,
       except as required for reasonable and customary use in describing the
       origin of the Work and reproducing the content of the NOTICE file.

    7. Disclaimer of Warranty. Unless required by applicable law or
       agreed to in writing, Licensor provides the Work (and each
       Contributor provides its Contributions) on an "AS IS" BASIS,
       WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or
       implied, including, without limitation, any warranties or conditions
       of TITLE, NON-INFRINGEMENT, MERCHANTABILITY, or FITNESS FOR A
       PARTICULAR PURPOSE. You are solely responsible for determining the
       appropriateness of using or redistributing the Work and assume any
       risks associated with Your exercise of permissions under this License.

    8. Limitation of Liability. In no event and under no legal theory,
       whether in tort (including negligence), contract, or otherwise,
       unless required by applicable law (such as deliberate and grossly
       negligent acts) or agreed to in writing, shall any Contributor be
       liable to You for damages, including any direct, indirect, special,
       incidental, or consequential damages of any character arising as a
       result of this License or out of the use or inability to use the
       Work (including but not limited to damages for loss of goodwill,
       work stoppage, computer failure or malfunction, or any and all
       other commercial damages or losses), even if such Contributor
       has been advised of the possibility of such damages.

    9. Accepting Warranty or Additional Liability. While redistributing
       the Work or Derivative Works thereof, You may choose to offer,
       and charge a fee for, acceptance of support, warranty, indemnity,
       or other liability obligations and/or rights consistent with this
       License. However, in accepting such obligations, You may act only
       on Your own behalf and on Your sole responsibility, not on behalf
       of any other Contributor, and only if You agree to indemnify,
       defend, and hold each Contributor harmless for any liability
       incurred by, or claims asserted against, such Contributor by reason
       of your accepting any such warranty or additional liability.

    END OF TERMS AND CONDITIONS

    APPENDIX: How to apply the Apache License to your work.

       To apply the Apache License to your work, attach the following
       boilerplate notice, with the fields enclosed by brackets "[]"
       replaced with your own identifying information. (Don't include
       the brackets!)  The text should be enclosed in the appropriate
       comment syntax for the file format. We also recommend that a
       file or class name and description of purpose be included on the
       same "printed page" as the copyright notice for easier
       identification within third-party archives.

    Copyright (c) 2016 Alex Crichton
    Copyright (c) 2017 The Tokio Authors

    Licensed under the Apache License, Version 2.0 (the "License");
    you may not use this file except in compliance with the License.
    You may obtain a copy of the License at

    	http://www.apache.org/licenses/LICENSE-2.0

    Unless required by applicable law or agreed to in writing, software
    distributed under the License is distributed on an "AS IS" BASIS,
    WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
    See the License for the specific language governing permissions and
    limitations under the License.

`LICENSE-MIT` (futures 0.3.34):

    Copyright (c) 2016 Alex Crichton
    Copyright (c) 2017 The Tokio Authors

    Permission is hereby granted, free of charge, to any
    person obtaining a copy of this software and associated
    documentation files (the "Software"), to deal in the
    Software without restriction, including without
    limitation the rights to use, copy, modify, merge,
    publish, distribute, sublicense, and/or sell copies of
    the Software, and to permit persons to whom the Software
    is furnished to do so, subject to the following
    conditions:

    The above copyright notice and this permission notice
    shall be included in all copies or substantial portions
    of the Software.

    THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF
    ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED
    TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A
    PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT
    SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
    CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
    OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR
    IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
    DEALINGS IN THE SOFTWARE.

## futures-channel 0.3.34

License: MIT OR Apache-2.0. Source: https://github.com/rust-lang/futures-rs

`LICENSE-APACHE` (futures-channel 0.3.34): the same text as `LICENSE-APACHE` (futures 0.3.34) above.

`LICENSE-MIT` (futures-channel 0.3.34): the same text as `LICENSE-MIT` (futures 0.3.34) above.

## futures-core 0.3.34

License: MIT OR Apache-2.0. Source: https://github.com/rust-lang/futures-rs

`LICENSE-APACHE` (futures-core 0.3.34): the same text as `LICENSE-APACHE` (futures 0.3.34) above.

`LICENSE-MIT` (futures-core 0.3.34): the same text as `LICENSE-MIT` (futures 0.3.34) above.

## futures-executor 0.3.34

License: MIT OR Apache-2.0. Source: https://github.com/rust-lang/futures-rs

`LICENSE-APACHE` (futures-executor 0.3.34): the same text as `LICENSE-APACHE` (futures 0.3.34) above.

`LICENSE-MIT` (futures-executor 0.3.34): the same text as `LICENSE-MIT` (futures 0.3.34) above.

## futures-io 0.3.34

License: MIT OR Apache-2.0. Source: https://github.com/rust-lang/futures-rs

`LICENSE-APACHE` (futures-io 0.3.34): the same text as `LICENSE-APACHE` (futures 0.3.34) above.

`LICENSE-MIT` (futures-io 0.3.34): the same text as `LICENSE-MIT` (futures 0.3.34) above.

## futures-sink 0.3.34

License: MIT OR Apache-2.0. Source: https://github.com/rust-lang/futures-rs

`LICENSE-APACHE` (futures-sink 0.3.34): the same text as `LICENSE-APACHE` (futures 0.3.34) above.

`LICENSE-MIT` (futures-sink 0.3.34): the same text as `LICENSE-MIT` (futures 0.3.34) above.

## futures-task 0.3.34

License: MIT OR Apache-2.0. Source: https://github.com/rust-lang/futures-rs

`LICENSE-APACHE` (futures-task 0.3.34): the same text as `LICENSE-APACHE` (futures 0.3.34) above.

`LICENSE-MIT` (futures-task 0.3.34): the same text as `LICENSE-MIT` (futures 0.3.34) above.

## futures-util 0.3.34

License: MIT OR Apache-2.0. Source: https://github.com/rust-lang/futures-rs

`LICENSE-APACHE` (futures-util 0.3.34): the same text as `LICENSE-APACHE` (futures 0.3.34) above.

`LICENSE-MIT` (futures-util 0.3.34): the same text as `LICENSE-MIT` (futures 0.3.34) above.

## gcd 2.3.0

License: MIT/Apache-2.0. Source: https://github.com/frewsxcv/rust-gcd
Authors: Corey Farwell <coreyf@rwell.org>.

`LICENSE-APACHE` (gcd 2.3.0): the same text as `LICENSE-APACHE` (addr2line 0.25.1) above.

`LICENSE-MIT` (gcd 2.3.0):

    Copyright (c) 2016 Corey Farwell

    Permission is hereby granted, free of charge, to any
    person obtaining a copy of this software and associated
    documentation files (the "Software"), to deal in the
    Software without restriction, including without
    limitation the rights to use, copy, modify, merge,
    publish, distribute, sublicense, and/or sell copies of
    the Software, and to permit persons to whom the Software
    is furnished to do so, subject to the following
    conditions:

    The above copyright notice and this permission notice
    shall be included in all copies or substantial portions
    of the Software.

    THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF
    ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED
    TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A
    PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT
    SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
    CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
    OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR
    IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
    DEALINGS IN THE SOFTWARE.

## gen_ops 0.4.0

License: MIT. Source: https://github.com/K-Vinayak/gen_ops
Authors: K Vinayak <kvinayakklly4@gmail.com>.

`LICENSE.md` (gen_ops 0.4.0):

    MIT License

    Copyright (c) 2022 K Vinayak

    Permission is hereby granted, free of charge, to any person obtaining a copy
    of this software and associated documentation files (the "Software"), to deal
    in the Software without restriction, including without limitation the rights
    to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
    copies of the Software, and to permit persons to whom the Software is
    furnished to do so, subject to the following conditions:

    The above copyright notice and this permission notice shall be included in all
    copies or substantial portions of the Software.

    THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
    IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
    FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
    AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
    LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
    OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
    SOFTWARE.

## generic-array 0.14.7

License: MIT. Source: https://github.com/fizyk20/generic-array.git
Authors: Bartłomiej Kamiński <fizyk20@gmail.com>, Aaron Trent <novacrazy@gmail.com>.

`LICENSE` (generic-array 0.14.7):

    The MIT License (MIT)

    Copyright (c) 2015 Bartłomiej Kamiński

    Permission is hereby granted, free of charge, to any person obtaining a copy
    of this software and associated documentation files (the "Software"), to deal
    in the Software without restriction, including without limitation the rights
    to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
    copies of the Software, and to permit persons to whom the Software is
    furnished to do so, subject to the following conditions:

    The above copyright notice and this permission notice shall be included in all
    copies or substantial portions of the Software.

    THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
    IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
    FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
    AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
    LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
    OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
    SOFTWARE.

## generic-array 0.14.9

License: MIT. Source: https://github.com/fizyk20/generic-array.git
Authors: Bartłomiej Kamiński <fizyk20@gmail.com>, Aaron Trent <novacrazy@gmail.com>.

`LICENSE` (generic-array 0.14.9): the same text as `LICENSE` (generic-array 0.14.7) above.

## getrandom 0.2.17

License: MIT OR Apache-2.0. Source: https://github.com/rust-random/getrandom
Authors: The Rand Project Developers.

`LICENSE-APACHE` (getrandom 0.2.17):

                                  Apache License
                            Version 2.0, January 2004
                         https://www.apache.org/licenses/

    TERMS AND CONDITIONS FOR USE, REPRODUCTION, AND DISTRIBUTION

    1. Definitions.

       "License" shall mean the terms and conditions for use, reproduction,
       and distribution as defined by Sections 1 through 9 of this document.

       "Licensor" shall mean the copyright owner or entity authorized by
       the copyright owner that is granting the License.

       "Legal Entity" shall mean the union of the acting entity and all
       other entities that control, are controlled by, or are under common
       control with that entity. For the purposes of this definition,
       "control" means (i) the power, direct or indirect, to cause the
       direction or management of such entity, whether by contract or
       otherwise, or (ii) ownership of fifty percent (50%) or more of the
       outstanding shares, or (iii) beneficial ownership of such entity.

       "You" (or "Your") shall mean an individual or Legal Entity
       exercising permissions granted by this License.

       "Source" form shall mean the preferred form for making modifications,
       including but not limited to software source code, documentation
       source, and configuration files.

       "Object" form shall mean any form resulting from mechanical
       transformation or translation of a Source form, including but
       not limited to compiled object code, generated documentation,
       and conversions to other media types.

       "Work" shall mean the work of authorship, whether in Source or
       Object form, made available under the License, as indicated by a
       copyright notice that is included in or attached to the work
       (an example is provided in the Appendix below).

       "Derivative Works" shall mean any work, whether in Source or Object
       form, that is based on (or derived from) the Work and for which the
       editorial revisions, annotations, elaborations, or other modifications
       represent, as a whole, an original work of authorship. For the purposes
       of this License, Derivative Works shall not include works that remain
       separable from, or merely link (or bind by name) to the interfaces of,
       the Work and Derivative Works thereof.

       "Contribution" shall mean any work of authorship, including
       the original version of the Work and any modifications or additions
       to that Work or Derivative Works thereof, that is intentionally
       submitted to Licensor for inclusion in the Work by the copyright owner
       or by an individual or Legal Entity authorized to submit on behalf of
       the copyright owner. For the purposes of this definition, "submitted"
       means any form of electronic, verbal, or written communication sent
       to the Licensor or its representatives, including but not limited to
       communication on electronic mailing lists, source code control systems,
       and issue tracking systems that are managed by, or on behalf of, the
       Licensor for the purpose of discussing and improving the Work, but
       excluding communication that is conspicuously marked or otherwise
       designated in writing by the copyright owner as "Not a Contribution."

       "Contributor" shall mean Licensor and any individual or Legal Entity
       on behalf of whom a Contribution has been received by Licensor and
       subsequently incorporated within the Work.

    2. Grant of Copyright License. Subject to the terms and conditions of
       this License, each Contributor hereby grants to You a perpetual,
       worldwide, non-exclusive, no-charge, royalty-free, irrevocable
       copyright license to reproduce, prepare Derivative Works of,
       publicly display, publicly perform, sublicense, and distribute the
       Work and such Derivative Works in Source or Object form.

    3. Grant of Patent License. Subject to the terms and conditions of
       this License, each Contributor hereby grants to You a perpetual,
       worldwide, non-exclusive, no-charge, royalty-free, irrevocable
       (except as stated in this section) patent license to make, have made,
       use, offer to sell, sell, import, and otherwise transfer the Work,
       where such license applies only to those patent claims licensable
       by such Contributor that are necessarily infringed by their
       Contribution(s) alone or by combination of their Contribution(s)
       with the Work to which such Contribution(s) was submitted. If You
       institute patent litigation against any entity (including a
       cross-claim or counterclaim in a lawsuit) alleging that the Work
       or a Contribution incorporated within the Work constitutes direct
       or contributory patent infringement, then any patent licenses
       granted to You under this License for that Work shall terminate
       as of the date such litigation is filed.

    4. Redistribution. You may reproduce and distribute copies of the
       Work or Derivative Works thereof in any medium, with or without
       modifications, and in Source or Object form, provided that You
       meet the following conditions:

       (a) You must give any other recipients of the Work or
           Derivative Works a copy of this License; and

       (b) You must cause any modified files to carry prominent notices
           stating that You changed the files; and

       (c) You must retain, in the Source form of any Derivative Works
           that You distribute, all copyright, patent, trademark, and
           attribution notices from the Source form of the Work,
           excluding those notices that do not pertain to any part of
           the Derivative Works; and

       (d) If the Work includes a "NOTICE" text file as part of its
           distribution, then any Derivative Works that You distribute must
           include a readable copy of the attribution notices contained
           within such NOTICE file, excluding those notices that do not
           pertain to any part of the Derivative Works, in at least one
           of the following places: within a NOTICE text file distributed
           as part of the Derivative Works; within the Source form or
           documentation, if provided along with the Derivative Works; or,
           within a display generated by the Derivative Works, if and
           wherever such third-party notices normally appear. The contents
           of the NOTICE file are for informational purposes only and
           do not modify the License. You may add Your own attribution
           notices within Derivative Works that You distribute, alongside
           or as an addendum to the NOTICE text from the Work, provided
           that such additional attribution notices cannot be construed
           as modifying the License.

       You may add Your own copyright statement to Your modifications and
       may provide additional or different license terms and conditions
       for use, reproduction, or distribution of Your modifications, or
       for any such Derivative Works as a whole, provided Your use,
       reproduction, and distribution of the Work otherwise complies with
       the conditions stated in this License.

    5. Submission of Contributions. Unless You explicitly state otherwise,
       any Contribution intentionally submitted for inclusion in the Work
       by You to the Licensor shall be under the terms and conditions of
       this License, without any additional terms or conditions.
       Notwithstanding the above, nothing herein shall supersede or modify
       the terms of any separate license agreement you may have executed
       with Licensor regarding such Contributions.

    6. Trademarks. This License does not grant permission to use the trade
       names, trademarks, service marks, or product names of the Licensor,
       except as required for reasonable and customary use in describing the
       origin of the Work and reproducing the content of the NOTICE file.

    7. Disclaimer of Warranty. Unless required by applicable law or
       agreed to in writing, Licensor provides the Work (and each
       Contributor provides its Contributions) on an "AS IS" BASIS,
       WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or
       implied, including, without limitation, any warranties or conditions
       of TITLE, NON-INFRINGEMENT, MERCHANTABILITY, or FITNESS FOR A
       PARTICULAR PURPOSE. You are solely responsible for determining the
       appropriateness of using or redistributing the Work and assume any
       risks associated with Your exercise of permissions under this License.

    8. Limitation of Liability. In no event and under no legal theory,
       whether in tort (including negligence), contract, or otherwise,
       unless required by applicable law (such as deliberate and grossly
       negligent acts) or agreed to in writing, shall any Contributor be
       liable to You for damages, including any direct, indirect, special,
       incidental, or consequential damages of any character arising as a
       result of this License or out of the use or inability to use the
       Work (including but not limited to damages for loss of goodwill,
       work stoppage, computer failure or malfunction, or any and all
       other commercial damages or losses), even if such Contributor
       has been advised of the possibility of such damages.

    9. Accepting Warranty or Additional Liability. While redistributing
       the Work or Derivative Works thereof, You may choose to offer,
       and charge a fee for, acceptance of support, warranty, indemnity,
       or other liability obligations and/or rights consistent with this
       License. However, in accepting such obligations, You may act only
       on Your own behalf and on Your sole responsibility, not on behalf
       of any other Contributor, and only if You agree to indemnify,
       defend, and hold each Contributor harmless for any liability
       incurred by, or claims asserted against, such Contributor by reason
       of your accepting any such warranty or additional liability.

    END OF TERMS AND CONDITIONS

    APPENDIX: How to apply the Apache License to your work.

       To apply the Apache License to your work, attach the following
       boilerplate notice, with the fields enclosed by brackets "[]"
       replaced with your own identifying information. (Don't include
       the brackets!)  The text should be enclosed in the appropriate
       comment syntax for the file format. We also recommend that a
       file or class name and description of purpose be included on the
       same "printed page" as the copyright notice for easier
       identification within third-party archives.

    Copyright [yyyy] [name of copyright owner]

    Licensed under the Apache License, Version 2.0 (the "License");
    you may not use this file except in compliance with the License.
    You may obtain a copy of the License at

    	https://www.apache.org/licenses/LICENSE-2.0

    Unless required by applicable law or agreed to in writing, software
    distributed under the License is distributed on an "AS IS" BASIS,
    WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
    See the License for the specific language governing permissions and
    limitations under the License.

`LICENSE-MIT` (getrandom 0.2.17):

    Copyright (c) 2018-2024 The rust-random Project Developers
    Copyright (c) 2014 The Rust Project Developers

    Permission is hereby granted, free of charge, to any
    person obtaining a copy of this software and associated
    documentation files (the "Software"), to deal in the
    Software without restriction, including without
    limitation the rights to use, copy, modify, merge,
    publish, distribute, sublicense, and/or sell copies of
    the Software, and to permit persons to whom the Software
    is furnished to do so, subject to the following
    conditions:

    The above copyright notice and this permission notice
    shall be included in all copies or substantial portions
    of the Software.

    THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF
    ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED
    TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A
    PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT
    SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
    CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
    OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR
    IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
    DEALINGS IN THE SOFTWARE.

## getrandom 0.3.4

License: MIT OR Apache-2.0. Source: https://github.com/rust-random/getrandom
Authors: The Rand Project Developers.

`LICENSE-APACHE` (getrandom 0.3.4): the same text as `LICENSE-APACHE` (getrandom 0.2.17) above.

`LICENSE-MIT` (getrandom 0.3.4):

    Copyright (c) 2018-2025 The rust-random Project Developers
    Copyright (c) 2014 The Rust Project Developers

    Permission is hereby granted, free of charge, to any
    person obtaining a copy of this software and associated
    documentation files (the "Software"), to deal in the
    Software without restriction, including without
    limitation the rights to use, copy, modify, merge,
    publish, distribute, sublicense, and/or sell copies of
    the Software, and to permit persons to whom the Software
    is furnished to do so, subject to the following
    conditions:

    The above copyright notice and this permission notice
    shall be included in all copies or substantial portions
    of the Software.

    THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF
    ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED
    TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A
    PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT
    SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
    CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
    OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR
    IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
    DEALINGS IN THE SOFTWARE.

## gimli 0.32.3

License: MIT OR Apache-2.0. Source: https://github.com/gimli-rs/gimli

`LICENSE-APACHE` (gimli 0.32.3): the same text as `LICENSE-APACHE` (addr2line 0.25.1) above.

`LICENSE-MIT` (gimli 0.32.3):

    Copyright (c) 2015 The Rust Project Developers

    Permission is hereby granted, free of charge, to any
    person obtaining a copy of this software and associated
    documentation files (the "Software"), to deal in the
    Software without restriction, including without
    limitation the rights to use, copy, modify, merge,
    publish, distribute, sublicense, and/or sell copies of
    the Software, and to permit persons to whom the Software
    is furnished to do so, subject to the following
    conditions:

    The above copyright notice and this permission notice
    shall be included in all copies or substantial portions
    of the Software.

    THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF
    ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED
    TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A
    PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT
    SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
    CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
    OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR
    IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
    DEALINGS IN THE SOFTWARE.

## gimli 0.34.0

License: MIT OR Apache-2.0. Source: https://github.com/gimli-rs/gimli

`LICENSE-APACHE` (gimli 0.34.0): the same text as `LICENSE-APACHE` (addr2line 0.25.1) above.

`LICENSE-MIT` (gimli 0.34.0): the same text as `LICENSE-MIT` (gimli 0.32.3) above.

## group 0.12.1

License: MIT/Apache-2.0. Source: https://github.com/zkcrypto/group
Authors: Sean Bowe <ewillbefull@gmail.com>, Jack Grigg <jack@z.cash>.

`COPYRIGHT` (group 0.12.1):

    Copyrights in the "group" library are retained by their contributors. No
    copyright assignment is required to contribute to the "group" library.

    The "group" library is licensed under either of

     * Apache License, Version 2.0, (see ./LICENSE-APACHE or http://www.apache.org/licenses/LICENSE-2.0)
     * MIT license (see ./LICENSE-MIT or http://opensource.org/licenses/MIT)

    at your option.

    Unless you explicitly state otherwise, any contribution intentionally
    submitted for inclusion in the work by you, as defined in the Apache-2.0
    license, shall be dual licensed as above, without any additional terms or
    conditions.

`LICENSE-APACHE` (group 0.12.1): the same text as `LICENSE-APACHE` (addr2line 0.25.1) above.

`LICENSE-MIT` (group 0.12.1): the same text as `LICENSE-MIT` (adler2 2.0.1) above.

## group 0.13.0

License: MIT/Apache-2.0. Source: https://github.com/zkcrypto/group
Authors: Sean Bowe <ewillbefull@gmail.com>, Jack Grigg <jack@z.cash>.

`COPYRIGHT` (group 0.13.0): the same text as `COPYRIGHT` (group 0.12.1) above.

`LICENSE-APACHE` (group 0.13.0): the same text as `LICENSE-APACHE` (addr2line 0.25.1) above.

`LICENSE-MIT` (group 0.13.0): the same text as `LICENSE-MIT` (adler2 2.0.1) above.

## halo2 0.1.0-beta.2

License: see below. Source: https://github.com/zcash/halo2
Authors: Jack Grigg <jack@electriccoin.co>.
An empty package: its library holds no code, so nothing of it is compiled into the module.

`COPYING` (halo2 0.1.0-beta.2):

    Copyright 2020-2022 The Electric Coin Company

    This package ("Original Work") is licensed under the terms of the Bootstrap Open
    Source License, version 1.0, or at your option, any later version ("BOSL"). See
    the file ./LICENSE-BOSL for the terms of the Bootstrap Open Source Licence,
    version 1.0.

    Only if this Original Work is included as part of the distribution of one of the
    following projects ("the Project"):

    - The Zcash projects published by the Electric Coin Company,
    - The Zebra project published by the Zcash Foundation,

    then License is granted to use this package under the BOSL as modified by the
    following clarification and special exception. This exception applies only to
    the Original Work when linked or combined with the Project and not to the
    Original Work when linked, combined, or included in or with any other software
    or project or on a standalone basis.

        Under the terms of the BOSL, linking or combining this Original Work with
        the Project creates a Derivative Work based upon the Original Work and the
        terms of the BOSL thus apply to both the Original Work and that Derivative
        Work. As a special exception to the BOSL, and to allow this Original Work to
        be linked and combined with the Project without having to apply the BOSL to
        the other portions of the Project, you are granted permission to link or
        combine this Original Work with the Project and to copy and distribute the
        resulting work ("Resulting Work") under the open source license applicable
        to the Project ("Project License"), provided that any portions of this
        Original Work included in the Resulting Work remain subject to the BOSL. For
        clarity, you may continue to treat all other portions of the Project under
        the Project License, provided that you comply with the BOSL with respect to
        the Original Work. If you modify this Original Work, your version of the
        Original Work must remain under the BOSL. You may also extend this exception
        to your version, but you are not obligated to do so. If you do not wish to
        do so, delete this exception statement from your version.

## halo2_proofs 0.1.0

License: MIT OR Apache-2.0. Source: https://github.com/zcash/halo2
Authors: Sean Bowe <sean@electriccoin.co>, Ying Tong Lai <yingtong@electriccoin.co>, Daira Hopwood <daira@electriccoin.co>, Jack Grigg <jack@electriccoin.co>.

Its package carries no license file: the standard text of its license, under its authors.

Apache-2.0 (halo2_proofs 0.1.0): the same text as Apache-2.0 above.

MIT (halo2_proofs 0.1.0):

    MIT License

    Copyright (c) Sean Bowe <sean@electriccoin.co>, Ying Tong Lai <yingtong@electriccoin.co>, Daira Hopwood <daira@electriccoin.co>, Jack Grigg <jack@electriccoin.co>

    Permission is hereby granted, free of charge, to any person obtaining a copy of this software and associated documentation files (the "Software"), to deal in the Software without restriction, including without limitation the rights to use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of the Software, and to permit persons to whom the Software is furnished to do so, subject to the following conditions:

    The above copyright notice and this permission notice shall be included in all copies or substantial portions of the Software.

    THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

## hash32 0.3.1

License: MIT OR Apache-2.0. Source: https://github.com/japaric/hash32
Authors: Jorge Aparicio <jorge@japaric.io>.

`LICENSE-APACHE` (hash32 0.3.1): the same text as `LICENSE-APACHE` (addr2line 0.25.1) above.

`LICENSE-MIT` (hash32 0.3.1):

    Copyright (c) 2018 Jorge Aparicio

    Permission is hereby granted, free of charge, to any
    person obtaining a copy of this software and associated
    documentation files (the "Software"), to deal in the
    Software without restriction, including without
    limitation the rights to use, copy, modify, merge,
    publish, distribute, sublicense, and/or sell copies of
    the Software, and to permit persons to whom the Software
    is furnished to do so, subject to the following
    conditions:

    The above copyright notice and this permission notice
    shall be included in all copies or substantial portions
    of the Software.

    THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF
    ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED
    TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A
    PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT
    SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
    CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
    OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR
    IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
    DEALINGS IN THE SOFTWARE.

## hashbrown 0.14.5

License: MIT OR Apache-2.0. Source: https://github.com/rust-lang/hashbrown
Authors: Amanieu d'Antras <amanieu@gmail.com>.

`LICENSE-APACHE` (hashbrown 0.14.5): the same text as `LICENSE-APACHE` (addr2line 0.25.1) above.

`LICENSE-MIT` (hashbrown 0.14.5):

    Copyright (c) 2016 Amanieu d'Antras

    Permission is hereby granted, free of charge, to any
    person obtaining a copy of this software and associated
    documentation files (the "Software"), to deal in the
    Software without restriction, including without
    limitation the rights to use, copy, modify, merge,
    publish, distribute, sublicense, and/or sell copies of
    the Software, and to permit persons to whom the Software
    is furnished to do so, subject to the following
    conditions:

    The above copyright notice and this permission notice
    shall be included in all copies or substantial portions
    of the Software.

    THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF
    ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED
    TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A
    PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT
    SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
    CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
    OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR
    IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
    DEALINGS IN THE SOFTWARE.

## hashbrown 0.15.5

License: MIT OR Apache-2.0. Source: https://github.com/rust-lang/hashbrown
Authors: Amanieu d'Antras <amanieu@gmail.com>.

`LICENSE-APACHE` (hashbrown 0.15.5): the same text as `LICENSE-APACHE` (addr2line 0.25.1) above.

`LICENSE-MIT` (hashbrown 0.15.5): the same text as `LICENSE-MIT` (hashbrown 0.14.5) above.

## hashbrown 0.16.1

License: MIT OR Apache-2.0. Source: https://github.com/rust-lang/hashbrown
Authors: Amanieu d'Antras <amanieu@gmail.com>.

`LICENSE-APACHE` (hashbrown 0.16.1): the same text as `LICENSE-APACHE` (addr2line 0.25.1) above.

`LICENSE-MIT` (hashbrown 0.16.1): the same text as `LICENSE-MIT` (hashbrown 0.14.5) above.

## hashbrown 0.17.1

License: MIT OR Apache-2.0. Source: https://github.com/rust-lang/hashbrown

`LICENSE-APACHE` (hashbrown 0.17.1): the same text as `LICENSE-APACHE` (addr2line 0.25.1) above.

`LICENSE-MIT` (hashbrown 0.17.1): the same text as `LICENSE-MIT` (hashbrown 0.14.5) above.

## heapless 0.9.3

License: MIT OR Apache-2.0. Source: https://github.com/rust-embedded/heapless
Authors: Jorge Aparicio <jorge@japaric.io>, Per Lindgren <per.lindgren@ltu.se>, Emil Fresk <emil.fresk@gmail.com>.

`LICENSE-APACHE` (heapless 0.9.3): the same text as `LICENSE-APACHE` (addr2line 0.25.1) above.

`LICENSE-MIT` (heapless 0.9.3):

    Copyright (c) 2017 Jorge Aparicio

    Permission is hereby granted, free of charge, to any
    person obtaining a copy of this software and associated
    documentation files (the "Software"), to deal in the
    Software without restriction, including without
    limitation the rights to use, copy, modify, merge,
    publish, distribute, sublicense, and/or sell copies of
    the Software, and to permit persons to whom the Software
    is furnished to do so, subject to the following
    conditions:

    The above copyright notice and this permission notice
    shall be included in all copies or substantial portions
    of the Software.

    THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF
    ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED
    TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A
    PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT
    SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
    CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
    OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR
    IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
    DEALINGS IN THE SOFTWARE.

## hex 0.4.3

License: MIT OR Apache-2.0. Source: https://github.com/KokaKiwi/rust-hex
Authors: KokaKiwi <kokakiwi@kokakiwi.net>.

`LICENSE-APACHE` (hex 0.4.3): the same text as `LICENSE-APACHE` (anstyle 1.0.14) above.

`LICENSE-MIT` (hex 0.4.3):

    Copyright (c) 2013-2014 The Rust Project Developers.
    Copyright (c) 2015-2020 The rust-hex Developers

    Permission is hereby granted, free of charge, to any person obtaining a copy
    of this software and associated documentation files (the "Software"), to deal
    in the Software without restriction, including without limitation the rights
    to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
    copies of the Software, and to permit persons to whom the Software is
    furnished to do so, subject to the following conditions:

    The above copyright notice and this permission notice shall be included in all
    copies or substantial portions of the Software.

    THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
    IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
    FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
    AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
    LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
    OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
    SOFTWARE.

## hex-literal 1.1.0

License: MIT OR Apache-2.0. Source: https://github.com/RustCrypto/utils
Authors: RustCrypto Developers.

`LICENSE-APACHE` (hex-literal 1.1.0): the same text as `LICENSE-APACHE` (base16ct 0.2.0) above.

`LICENSE-MIT` (hex-literal 1.1.0):

    Copyright (c) 2018-2025 The RustCrypto Project Developers
    Copyright (c) 2018 Artyom Pavlov

    Permission is hereby granted, free of charge, to any
    person obtaining a copy of this software and associated
    documentation files (the "Software"), to deal in the
    Software without restriction, including without
    limitation the rights to use, copy, modify, merge,
    publish, distribute, sublicense, and/or sell copies of
    the Software, and to permit persons to whom the Software
    is furnished to do so, subject to the following
    conditions:

    The above copyright notice and this permission notice
    shall be included in all copies or substantial portions
    of the Software.

    THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF
    ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED
    TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A
    PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT
    SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
    CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
    OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR
    IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
    DEALINGS IN THE SOFTWARE.

## hmac 0.12.1

License: MIT OR Apache-2.0. Source: https://github.com/RustCrypto/MACs
Authors: RustCrypto Developers.

`LICENSE-APACHE` (hmac 0.12.1): the same text as `LICENSE-APACHE` (base16ct 0.2.0) above.

`LICENSE-MIT` (hmac 0.12.1): the same text as `LICENSE-MIT` (digest 0.10.7) above.

## hybrid-array 0.4.14

License: MIT OR Apache-2.0. Source: https://github.com/RustCrypto/hybrid-array
Authors: RustCrypto Developers.

`LICENSE-APACHE` (hybrid-array 0.4.14): the same text as `LICENSE-APACHE` (base16ct 0.2.0) above.

`LICENSE-MIT` (hybrid-array 0.4.14):

    Copyright (c) 2022-2026 The RustCrypto Project Developers

    Permission is hereby granted, free of charge, to any
    person obtaining a copy of this software and associated
    documentation files (the "Software"), to deal in the
    Software without restriction, including without
    limitation the rights to use, copy, modify, merge,
    publish, distribute, sublicense, and/or sell copies of
    the Software, and to permit persons to whom the Software
    is furnished to do so, subject to the following
    conditions:

    The above copyright notice and this permission notice
    shall be included in all copies or substantial portions
    of the Software.

    THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF
    ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED
    TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A
    PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT
    SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
    CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
    OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR
    IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
    DEALINGS IN THE SOFTWARE.

## hybrid-array 0.4.15

License: MIT OR Apache-2.0. Source: https://github.com/RustCrypto/hybrid-array
Authors: RustCrypto Developers.

`LICENSE-APACHE` (hybrid-array 0.4.15): the same text as `LICENSE-APACHE` (base16ct 0.2.0) above.

`LICENSE-MIT` (hybrid-array 0.4.15): the same text as `LICENSE-MIT` (hybrid-array 0.4.14) above.

## indenter 0.3.4

License: MIT OR Apache-2.0. Source: https://github.com/yaahc/indenter
Authors: Jane Lusby <jlusby@yaah.dev>.

`LICENSE-APACHE` (indenter 0.3.4): the same text as `LICENSE-APACHE` (allocator-api2 0.2.21) above.

`LICENSE-MIT` (indenter 0.3.4): the same text as `LICENSE-MIT` (adler2 2.0.1) above.

## indexmap 2.14.2

License: Apache-2.0 OR MIT. Source: https://github.com/indexmap-rs/indexmap

`LICENSE-APACHE` (indexmap 2.14.2): the same text as `LICENSE-APACHE` (addr2line 0.25.1) above.

`LICENSE-MIT` (indexmap 2.14.2):

    Copyright (c) 2016--2017

    Permission is hereby granted, free of charge, to any
    person obtaining a copy of this software and associated
    documentation files (the "Software"), to deal in the
    Software without restriction, including without
    limitation the rights to use, copy, modify, merge,
    publish, distribute, sublicense, and/or sell copies of
    the Software, and to permit persons to whom the Software
    is furnished to do so, subject to the following
    conditions:

    The above copyright notice and this permission notice
    shall be included in all copies or substantial portions
    of the Software.

    THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF
    ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED
    TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A
    PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT
    SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
    CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
    OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR
    IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
    DEALINGS IN THE SOFTWARE.

## itertools 0.10.5

License: MIT/Apache-2.0. Source: https://github.com/rust-itertools/itertools
Authors: bluss.

`LICENSE-APACHE` (itertools 0.10.5): the same text as `LICENSE-APACHE` (addr2line 0.25.1) above.

`LICENSE-MIT` (itertools 0.10.5): the same text as `LICENSE-MIT` (either 1.18.0) above.

## itertools 0.12.1

License: MIT OR Apache-2.0. Source: https://github.com/rust-itertools/itertools
Authors: bluss.

`LICENSE-APACHE` (itertools 0.12.1): the same text as `LICENSE-APACHE` (addr2line 0.25.1) above.

`LICENSE-MIT` (itertools 0.12.1): the same text as `LICENSE-MIT` (either 1.18.0) above.

## itertools 0.13.0

License: MIT OR Apache-2.0. Source: https://github.com/rust-itertools/itertools
Authors: bluss.

`LICENSE-APACHE` (itertools 0.13.0): the same text as `LICENSE-APACHE` (addr2line 0.25.1) above.

`LICENSE-MIT` (itertools 0.13.0): the same text as `LICENSE-MIT` (either 1.18.0) above.

## itertools 0.14.0

License: MIT OR Apache-2.0. Source: https://github.com/rust-itertools/itertools
Authors: bluss.

`LICENSE-APACHE` (itertools 0.14.0): the same text as `LICENSE-APACHE` (addr2line 0.25.1) above.

`LICENSE-MIT` (itertools 0.14.0): the same text as `LICENSE-MIT` (either 1.18.0) above.

## itoa 1.0.18

License: MIT OR Apache-2.0. Source: https://github.com/dtolnay/itoa
Authors: David Tolnay <dtolnay@gmail.com>.

`LICENSE-APACHE` (itoa 1.0.18): the same text as `LICENSE-APACHE` (allocator-api2 0.2.21) above.

`LICENSE-MIT` (itoa 1.0.18): the same text as `LICENSE-MIT` (adler2 2.0.1) above.

## itoa 1.0.19

License: MIT OR Apache-2.0. Source: https://github.com/dtolnay/itoa
Authors: David Tolnay <dtolnay@gmail.com>.

`LICENSE-APACHE` (itoa 1.0.19): the same text as `LICENSE-APACHE` (allocator-api2 0.2.21) above.

`LICENSE-MIT` (itoa 1.0.19): the same text as `LICENSE-MIT` (adler2 2.0.1) above.

## js-sys 0.3.106

License: MIT OR Apache-2.0. Source: https://github.com/wasm-bindgen/wasm-bindgen/tree/master/crates/js-sys
Authors: The wasm-bindgen Developers.

`LICENSE-APACHE` (js-sys 0.3.106): the same text as `LICENSE-APACHE` (addr2line 0.25.1) above.

`LICENSE-MIT` (js-sys 0.3.106): the same text as `LICENSE-MIT` (backtrace 0.3.76) above.

## jubjub 0.9.0

License: MIT/Apache-2.0. Source: https://github.com/zkcrypto/jubjub
Authors: Sean Bowe <ewillbefull@gmail.com>, Eirik Ogilvie-Wigley <eowigley@gmail.com>, Jack Grigg <thestr4d@gmail.com>.

`COPYRIGHT` (jubjub 0.9.0):

    Copyrights in the "jubjub" library are retained by their contributors. No
    copyright assignment is required to contribute to the "jubjub" library.

    The "jubjub" library is licensed under either of

     * Apache License, Version 2.0, (see ./LICENSE-APACHE or http://www.apache.org/licenses/LICENSE-2.0)
     * MIT license (see ./LICENSE-MIT or http://opensource.org/licenses/MIT)

    at your option.

    Unless you explicitly state otherwise, any contribution intentionally
    submitted for inclusion in the work by you, as defined in the Apache-2.0
    license, shall be dual licensed as above, without any additional terms or
    conditions.

`LICENSE-APACHE` (jubjub 0.9.0): the same text as `LICENSE-APACHE` (addr2line 0.25.1) above.

`LICENSE-MIT` (jubjub 0.9.0): the same text as `LICENSE-MIT` (adler2 2.0.1) above.

## k256 0.13.4

License: Apache-2.0 OR MIT. Source: https://github.com/RustCrypto/elliptic-curves/tree/master/k256
Authors: RustCrypto Developers.

`LICENSE-APACHE` (k256 0.13.4): the same text as `LICENSE-APACHE` (base16ct 0.2.0) above.

`LICENSE-MIT` (k256 0.13.4):

    Copyright (c) 2020-2024 RustCrypto Developers

    Permission is hereby granted, free of charge, to any
    person obtaining a copy of this software and associated
    documentation files (the "Software"), to deal in the
    Software without restriction, including without
    limitation the rights to use, copy, modify, merge,
    publish, distribute, sublicense, and/or sell copies of
    the Software, and to permit persons to whom the Software
    is furnished to do so, subject to the following
    conditions:

    The above copyright notice and this permission notice
    shall be included in all copies or substantial portions
    of the Software.

    THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF
    ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED
    TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A
    PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT
    SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
    CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
    OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR
    IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
    DEALINGS IN THE SOFTWARE.

## keccak 0.1.6

License: Apache-2.0 OR MIT. Source: https://github.com/RustCrypto/sponges/tree/master/keccak
Authors: RustCrypto Developers.

`LICENSE-APACHE` (keccak 0.1.6): the same text as `LICENSE-APACHE` (base16ct 0.2.0) above.

`LICENSE-MIT` (keccak 0.1.6): the same text as `LICENSE-MIT` (ecdsa 0.16.9) above.

## keccak 0.2.2

License: Apache-2.0 OR MIT. Source: https://github.com/RustCrypto/sponges
Authors: RustCrypto Developers.

`LICENSE-APACHE` (keccak 0.2.2): the same text as `LICENSE-APACHE` (base16ct 0.2.0) above.

`LICENSE-MIT` (keccak 0.2.2):

    Copyright (c) 2018-2026 The RustCrypto Project Developers

    Permission is hereby granted, free of charge, to any
    person obtaining a copy of this software and associated
    documentation files (the "Software"), to deal in the
    Software without restriction, including without
    limitation the rights to use, copy, modify, merge,
    publish, distribute, sublicense, and/or sell copies of
    the Software, and to permit persons to whom the Software
    is furnished to do so, subject to the following
    conditions:

    The above copyright notice and this permission notice
    shall be included in all copies or substantial portions
    of the Software.

    THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF
    ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED
    TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A
    PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT
    SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
    CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
    OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR
    IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
    DEALINGS IN THE SOFTWARE.

## lazy_static 1.5.0

License: MIT OR Apache-2.0. Source: https://github.com/rust-lang-nursery/lazy-static.rs
Authors: Marvin Löbel <loebel.marvin@gmail.com>.

`LICENSE-APACHE` (lazy_static 1.5.0): the same text as `LICENSE-APACHE` (addr2line 0.25.1) above.

`LICENSE-MIT` (lazy_static 1.5.0):

    Copyright (c) 2010 The Rust Project Developers

    Permission is hereby granted, free of charge, to any
    person obtaining a copy of this software and associated
    documentation files (the "Software"), to deal in the
    Software without restriction, including without
    limitation the rights to use, copy, modify, merge,
    publish, distribute, sublicense, and/or sell copies of
    the Software, and to permit persons to whom the Software
    is furnished to do so, subject to the following
    conditions:

    The above copyright notice and this permission notice
    shall be included in all copies or substantial portions
    of the Software.

    THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF
    ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED
    TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A
    PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT
    SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
    CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
    OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR
    IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
    DEALINGS IN THE SOFTWARE.

## lazy_static 1.5.1

License: MIT OR Apache-2.0. Source: https://github.com/rust-lang-nursery/lazy-static.rs
Authors: Marvin Löbel <loebel.marvin@gmail.com>.

`LICENSE-APACHE` (lazy_static 1.5.1): the same text as `LICENSE-APACHE` (addr2line 0.25.1) above.

`LICENSE-MIT` (lazy_static 1.5.1): the same text as `LICENSE-MIT` (adler2 2.0.1) above.

## libc 0.2.189

License: MIT OR Apache-2.0. Source: https://github.com/rust-lang/libc

`LICENSE-APACHE` (libc 0.2.189): the same text as `LICENSE-APACHE` (allocator-api2 0.2.21) above.

`LICENSE-MIT` (libc 0.2.189):

    Copyright (c) The Rust Project Developers

    Permission is hereby granted, free of charge, to any
    person obtaining a copy of this software and associated
    documentation files (the "Software"), to deal in the
    Software without restriction, including without
    limitation the rights to use, copy, modify, merge,
    publish, distribute, sublicense, and/or sell copies of
    the Software, and to permit persons to whom the Software
    is furnished to do so, subject to the following
    conditions:

    The above copyright notice and this permission notice
    shall be included in all copies or substantial portions
    of the Software.

    THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF
    ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED
    TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A
    PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT
    SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
    CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
    OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR
    IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
    DEALINGS IN THE SOFTWARE.

## libc 0.2.191

License: MIT OR Apache-2.0. Source: https://github.com/rust-lang/libc

`LICENSE-APACHE` (libc 0.2.191): the same text as `LICENSE-APACHE` (allocator-api2 0.2.21) above.

`LICENSE-MIT` (libc 0.2.191): the same text as `LICENSE-MIT` (libc 0.2.189) above.

## lock_api 0.4.14

License: MIT OR Apache-2.0. Source: https://github.com/Amanieu/parking_lot
Authors: Amanieu d'Antras <amanieu@gmail.com>.

`LICENSE-APACHE` (lock_api 0.4.14): the same text as `LICENSE-APACHE` (addr2line 0.25.1) above.

`LICENSE-MIT` (lock_api 0.4.14):

    Copyright (c) 2016 The Rust Project Developers

    Permission is hereby granted, free of charge, to any
    person obtaining a copy of this software and associated
    documentation files (the "Software"), to deal in the
    Software without restriction, including without
    limitation the rights to use, copy, modify, merge,
    publish, distribute, sublicense, and/or sell copies of
    the Software, and to permit persons to whom the Software
    is furnished to do so, subject to the following
    conditions:

    The above copyright notice and this permission notice
    shall be included in all copies or substantial portions
    of the Software.

    THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF
    ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED
    TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A
    PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT
    SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
    CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
    OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR
    IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
    DEALINGS IN THE SOFTWARE.

## lockfree-object-pool 0.1.6

License: BSL-1.0. Source: https://github.com/EVaillant/lockfree-object-pool
Authors: Etienne Vaillant <vaillant.etienne@gmail.com>.

`LICENSE_1_0.txt` (lockfree-object-pool 0.1.6):

    Boost Software License - Version 1.0 - August 17th, 2003

    Permission is hereby granted, free of charge, to any person or organization
    obtaining a copy of the software and accompanying documentation covered by
    this license (the "Software") to use, reproduce, display, distribute,
    execute, and transmit the Software, and to prepare derivative works of the
    Software, and to permit third-parties to whom the Software is furnished to
    do so, all subject to the following:

    The copyright notices in the Software and this entire statement, including
    the above license grant, this restriction and the following disclaimer,
    must be included in all copies of the Software, in whole or in part, and
    all derivative works of the Software, unless such copies or derivative
    works are solely in the form of machine-executable object code generated by
    a source language processor.

    THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
    IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
    FITNESS FOR A PARTICULAR PURPOSE, TITLE AND NON-INFRINGEMENT. IN NO EVENT
    SHALL THE COPYRIGHT HOLDERS OR ANYONE DISTRIBUTING THE SOFTWARE BE LIABLE
    FOR ANY DAMAGES OR OTHER LIABILITY, WHETHER IN CONTRACT, TORT OR OTHERWISE,
    ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
    DEALINGS IN THE SOFTWARE.

## log 0.4.34

License: MIT OR Apache-2.0. Source: https://github.com/rust-lang/log
Authors: The Rust Project Developers.

`LICENSE-APACHE` (log 0.4.34): the same text as `LICENSE-APACHE` (addr2line 0.25.1) above.

`LICENSE-MIT` (log 0.4.34): the same text as `LICENSE-MIT` (bitflags 2.13.2) above.

## matchers 0.2.0

License: MIT. Source: https://github.com/hawkw/matchers
Authors: Eliza Weisman <eliza@buoyant.io>.

`LICENSE` (matchers 0.2.0):

    Copyright (c) 2019 Eliza Weisman

    Permission is hereby granted, free of charge, to any person obtaining a copy
    of this software and associated documentation files (the "Software"), to deal
    in the Software without restriction, including without limitation the rights
    to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
    copies of the Software, and to permit persons to whom the Software is
    furnished to do so, subject to the following conditions:

    The above copyright notice and this permission notice shall be included in all
    copies or substantial portions of the Software.

    THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
    IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
    FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
    AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
    LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
    OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
    SOFTWARE.

## memchr 2.8.3

License: Unlicense OR MIT. Source: https://github.com/BurntSushi/memchr
Authors: Andrew Gallant <jamslam@gmail.com>, bluss.

`COPYING` (memchr 2.8.3): the same text as `COPYING` (aho-corasick 1.1.5) above.

`LICENSE-MIT` (memchr 2.8.3): the same text as `LICENSE-MIT` (aho-corasick 1.1.5) above.

`UNLICENSE` (memchr 2.8.3): the same text as `UNLICENSE` (aho-corasick 1.1.5) above.

## memuse 0.2.2

License: MIT/Apache-2.0. Source: https://github.com/str4d/memuse
Authors: Jack Grigg <thestr4d@gmail.com>.

`LICENSE-APACHE` (memuse 0.2.2): the same text as `LICENSE-APACHE` (addr2line 0.25.1) above.

`LICENSE-MIT` (memuse 0.2.2):

    The MIT License (MIT)

    Copyright (c) 2021 Jack Grigg

    Permission is hereby granted, free of charge, to any person obtaining a copy
    of this software and associated documentation files (the "Software"), to deal
    in the Software without restriction, including without limitation the rights
    to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
    copies of the Software, and to permit persons to whom the Software is
    furnished to do so, subject to the following conditions:

    The above copyright notice and this permission notice shall be included in
    all copies or substantial portions of the Software.

    THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
    IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
    FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
    AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
    LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
    OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN
    THE SOFTWARE.

## metrics 0.23.1

License: MIT. Source: https://github.com/metrics-rs/metrics
Authors: Toby Lawrence <toby@nuclearfurnace.com>.

`LICENSE` (metrics 0.23.1):

    Copyright (c) 2021 Metrics Contributors

    Permission is hereby granted, free of charge, to any person obtaining a copy
    of this software and associated documentation files (the "Software"), to deal
    in the Software without restriction, including without limitation the rights
    to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
    copies of the Software, and to permit persons to whom the Software is
    furnished to do so, subject to the following conditions:

    The above copyright notice and this permission notice shall be included in all
    copies or substantial portions of the Software.

    THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
    IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
    FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
    AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
    LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
    OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
    SOFTWARE.

## metrics-tracing-context 0.16.0

License: MIT. Source: https://github.com/metrics-rs/metrics
Authors: MOZGIII <mike-n@narod.ru>.

`LICENSE` (metrics-tracing-context 0.16.0): the same text as `LICENSE` (metrics 0.23.1) above.

## metrics-util 0.17.0

License: MIT. Source: https://github.com/metrics-rs/metrics
Authors: Toby Lawrence <toby@nuclearfurnace.com>.

`LICENSE` (metrics-util 0.17.0): the same text as `LICENSE` (metrics 0.23.1) above.

## miniz_oxide 0.8.9

License: MIT OR Zlib OR Apache-2.0. Source: https://github.com/Frommi/miniz_oxide/tree/master/miniz_oxide
Authors: Frommi <daniil.liferenko@gmail.com>, oyvindln <oyvindln@users.noreply.github.com>, Rich Geldreich richgel99@gmail.com.

`LICENSE` (miniz_oxide 0.8.9):

    MIT License

    Copyright 2013-2014 RAD Game Tools and Valve Software
    Copyright 2010-2014 Rich Geldreich and Tenacious Software LLC
    Copyright (c) 2017 Frommi
    Copyright (c) 2017-2024 oyvindln


    Permission is hereby granted, free of charge, to any person obtaining a copy
    of this software and associated documentation files (the "Software"), to deal
    in the Software without restriction, including without limitation the rights
    to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
    copies of the Software, and to permit persons to whom the Software is
    furnished to do so, subject to the following conditions:

    The above copyright notice and this permission notice shall be included in all
    copies or substantial portions of the Software.

    THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
    IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
    FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
    AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
    LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
    OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
    SOFTWARE.

`LICENSE-APACHE.md` (miniz_oxide 0.8.9): the same text as `LICENSE-APACHE` (ethnum 1.5.3) above.

`LICENSE-MIT.md` (miniz_oxide 0.8.9):

    MIT License

    Copyright 2013-2014 RAD Game Tools and Valve Software
    Copyright 2010-2014 Rich Geldreich and Tenacious Software LLC
    Copyright (c) 2017 Frommi
    Copyright (c) 2017-2024 oyvindln

    Permission is hereby granted, free of charge, to any person obtaining a copy
    of this software and associated documentation files (the "Software"), to deal
    in the Software without restriction, including without limitation the rights
    to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
    copies of the Software, and to permit persons to whom the Software is
    furnished to do so, subject to the following conditions:

    The above copyright notice and this permission notice shall be included in all
    copies or substantial portions of the Software.

    THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
    IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
    FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
    AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
    LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
    OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
    SOFTWARE.

`LICENSE-ZLIB.md` (miniz_oxide 0.8.9):

    Copyright 2013-2014 RAD Game Tools and Valve Software
    Copyright 2010-2014 Rich Geldreich and Tenacious Software LLC
    Copyright (c) 2020 Frommi
    Copyright (c) 2017-2024 oyvindln

    This software is provided 'as-is', without any express or implied warranty. In no event will the authors be held liable for any damages arising from the use of this software.

    Permission is granted to anyone to use this software for any purpose, including commercial applications, and to alter it and redistribute it freely, subject to the following restrictions:

    1. The origin of this software must not be misrepresented; you must not claim that you wrote the original software. If you use this software in a product, an acknowledgment in the product documentation would be appreciated but is not required.

    2. Altered source versions must be plainly marked as such, and must not be misrepresented as being the original software.

    3. This notice may not be removed or altered from any source distribution.

## miniz_oxide 0.9.1

License: MIT OR Zlib OR Apache-2.0. Source: https://github.com/Frommi/miniz_oxide/tree/master/miniz_oxide
Authors: Frommi <daniil.liferenko@gmail.com>, oyvindln <oyvindln@users.noreply.github.com>, Rich Geldreich richgel99@gmail.com.

`LICENSE` (miniz_oxide 0.9.1): the same text as `LICENSE` (miniz_oxide 0.8.9) above.

`LICENSE-APACHE.md` (miniz_oxide 0.9.1): the same text as `LICENSE-APACHE` (ethnum 1.5.3) above.

`LICENSE-MIT.md` (miniz_oxide 0.9.1): the same text as `LICENSE-MIT.md` (miniz_oxide 0.8.9) above.

`LICENSE-ZLIB.md` (miniz_oxide 0.9.1): the same text as `LICENSE-ZLIB.md` (miniz_oxide 0.8.9) above.

## nibble_vec 0.1.0

License: MIT. Source: https://github.com/michaelsproul/rust_nibble_vec
Authors: Michael Sproul <micsproul@gmail.com>.

`LICENSE` (nibble_vec 0.1.0):

    The MIT License (MIT)

    Copyright (c) 2015 Michael Sproul

    Permission is hereby granted, free of charge, to any person obtaining a copy
    of this software and associated documentation files (the "Software"), to deal
    in the Software without restriction, including without limitation the rights
    to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
    copies of the Software, and to permit persons to whom the Software is
    furnished to do so, subject to the following conditions:

    The above copyright notice and this permission notice shall be included in
    all copies or substantial portions of the Software.

    THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
    IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
    FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
    AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
    LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
    OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN
    THE SOFTWARE.

## nu-ansi-term 0.50.3

License: MIT. Source: https://github.com/nushell/nu-ansi-term
Authors: ogham@bsago.me, Ryan Scheel (Havvy) <ryan.havvy@gmail.com>, Josh Triplett <josh@joshtriplett.org>, The Nushell Project Developers.

`LICENSE` (nu-ansi-term 0.50.3):

    The MIT License (MIT)

    Copyright (c) 2014 Benjamin Sago
    Copyright (c) 2021-2022 The Nushell Project Developers

    Permission is hereby granted, free of charge, to any person obtaining a copy
    of this software and associated documentation files (the "Software"), to deal
    in the Software without restriction, including without limitation the rights
    to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
    copies of the Software, and to permit persons to whom the Software is
    furnished to do so, subject to the following conditions:

    The above copyright notice and this permission notice shall be included in all
    copies or substantial portions of the Software.

    THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
    IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
    FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
    AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
    LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
    OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
    SOFTWARE.

## num 0.4.3

License: MIT OR Apache-2.0. Source: https://github.com/rust-num/num
Authors: The Rust Project Developers.

`LICENSE-APACHE` (num 0.4.3): the same text as `LICENSE-APACHE` (addr2line 0.25.1) above.

`LICENSE-MIT` (num 0.4.3): the same text as `LICENSE-MIT` (bitflags 2.13.2) above.

## num-bigint 0.4.8

License: MIT OR Apache-2.0. Source: https://github.com/rust-num/num-bigint
Authors: The Rust Project Developers.

`LICENSE-APACHE` (num-bigint 0.4.8): the same text as `LICENSE-APACHE` (addr2line 0.25.1) above.

`LICENSE-MIT` (num-bigint 0.4.8): the same text as `LICENSE-MIT` (bitflags 2.13.2) above.

## num-complex 0.4.6

License: MIT OR Apache-2.0. Source: https://github.com/rust-num/num-complex
Authors: The Rust Project Developers.

`LICENSE-APACHE` (num-complex 0.4.6): the same text as `LICENSE-APACHE` (addr2line 0.25.1) above.

`LICENSE-MIT` (num-complex 0.4.6): the same text as `LICENSE-MIT` (bitflags 2.13.2) above.

## num-integer 0.1.47

License: MIT OR Apache-2.0. Source: https://github.com/rust-num/num-integer
Authors: The Rust Project Developers.

`LICENSE-APACHE` (num-integer 0.1.47): the same text as `LICENSE-APACHE` (addr2line 0.25.1) above.

`LICENSE-MIT` (num-integer 0.1.47): the same text as `LICENSE-MIT` (bitflags 2.13.2) above.

## num-iter 0.1.46

License: MIT OR Apache-2.0. Source: https://github.com/rust-num/num-iter
Authors: The Rust Project Developers.

`LICENSE-APACHE` (num-iter 0.1.46): the same text as `LICENSE-APACHE` (addr2line 0.25.1) above.

`LICENSE-MIT` (num-iter 0.1.46): the same text as `LICENSE-MIT` (bitflags 2.13.2) above.

## num-rational 0.4.2

License: MIT OR Apache-2.0. Source: https://github.com/rust-num/num-rational
Authors: The Rust Project Developers.

`LICENSE-APACHE` (num-rational 0.4.2): the same text as `LICENSE-APACHE` (addr2line 0.25.1) above.

`LICENSE-MIT` (num-rational 0.4.2): the same text as `LICENSE-MIT` (bitflags 2.13.2) above.

## num-traits 0.2.19

License: MIT OR Apache-2.0. Source: https://github.com/rust-num/num-traits
Authors: The Rust Project Developers.

`LICENSE-APACHE` (num-traits 0.2.19): the same text as `LICENSE-APACHE` (addr2line 0.25.1) above.

`LICENSE-MIT` (num-traits 0.2.19): the same text as `LICENSE-MIT` (bitflags 2.13.2) above.

## num_cpus 1.17.0

License: MIT OR Apache-2.0. Source: https://github.com/seanmonstar/num_cpus
Authors: Sean McArthur <sean@seanmonstar.com>.

`LICENSE-APACHE` (num_cpus 1.17.0): the same text as `LICENSE-APACHE` (addr2line 0.25.1) above.

`LICENSE-MIT` (num_cpus 1.17.0):

    Copyright (c) 2015-2025 Sean McArthur

    Permission is hereby granted, free of charge, to any person obtaining a copy
    of this software and associated documentation files (the "Software"), to deal
    in the Software without restriction, including without limitation the rights
    to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
    copies of the Software, and to permit persons to whom the Software is
    furnished to do so, subject to the following conditions:

    The above copyright notice and this permission notice shall be included in
    all copies or substantial portions of the Software.

    THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
    IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
    FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
    AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
    LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
    OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN
    THE SOFTWARE.

## num_enum 0.7.6

License: BSD-3-Clause OR MIT OR Apache-2.0. Source: https://github.com/illicitonion/num_enum
Authors: Daniel Wagner-Hall <dawagner@gmail.com>, Daniel Henry-Mantilla <daniel.henry.mantilla@gmail.com>, Vincent Esche <regexident@gmail.com>.

`LICENSE-APACHE` (num_enum 0.7.6): the same text as `LICENSE-APACHE` (allocator-api2 0.2.21) above.

`LICENSE-BSD` (num_enum 0.7.6):

    Copyright (c) 2018, Daniel Wagner-Hall
    All rights reserved.

    Redistribution and use in source and binary forms, with or without
    modification, are permitted provided that the following conditions are met:

    * Redistributions of source code must retain the above copyright notice, this
      list of conditions and the following disclaimer.

    * Redistributions in binary form must reproduce the above copyright notice,
      this list of conditions and the following disclaimer in the documentation
      and/or other materials provided with the distribution.

    * Neither the name of num_enum nor the names of its
      contributors may be used to endorse or promote products derived from
      this software without specific prior written permission.

    THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS IS"
    AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE
    IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE
    DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT HOLDER OR CONTRIBUTORS BE LIABLE
    FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL
    DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR
    SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER
    CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY,
    OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE
    OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.

`LICENSE-MIT` (num_enum 0.7.6): the same text as `LICENSE-MIT` (adler2 2.0.1) above.

## nybbles 0.4.8

License: MIT OR Apache-2.0. Source: https://github.com/alloy-rs/nybbles
Authors: DaniPopes <57450786+DaniPopes@users.noreply.github.com>.

`LICENSE-APACHE` (nybbles 0.4.8): the same text as `LICENSE-APACHE` (allocator-api2 0.2.21) above.

`LICENSE-MIT` (nybbles 0.4.8): the same text as `LICENSE-MIT` (adler2 2.0.1) above.

## nybbles 0.4.9

License: MIT OR Apache-2.0. Source: https://github.com/alloy-rs/nybbles
Authors: DaniPopes <57450786+DaniPopes@users.noreply.github.com>.

`LICENSE-APACHE` (nybbles 0.4.9): the same text as `LICENSE-APACHE` (allocator-api2 0.2.21) above.

`LICENSE-MIT` (nybbles 0.4.9): the same text as `LICENSE-MIT` (adler2 2.0.1) above.

## object 0.37.3

License: Apache-2.0 OR MIT. Source: https://github.com/gimli-rs/object

`LICENSE-APACHE` (object 0.37.3): the same text as `LICENSE-APACHE` (addr2line 0.25.1) above.

`LICENSE-MIT` (object 0.37.3):

    Copyright (c) 2015 The Gimli Developers

    Permission is hereby granted, free of charge, to any
    person obtaining a copy of this software and associated
    documentation files (the "Software"), to deal in the
    Software without restriction, including without
    limitation the rights to use, copy, modify, merge,
    publish, distribute, sublicense, and/or sell copies of
    the Software, and to permit persons to whom the Software
    is furnished to do so, subject to the following
    conditions:

    The above copyright notice and this permission notice
    shall be included in all copies or substantial portions
    of the Software.

    THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF
    ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED
    TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A
    PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT
    SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
    CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
    OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR
    IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
    DEALINGS IN THE SOFTWARE.

## object 0.40.0

License: Apache-2.0 OR MIT. Source: https://github.com/gimli-rs/object

`LICENSE-APACHE` (object 0.40.0): the same text as `LICENSE-APACHE` (addr2line 0.25.1) above.

`LICENSE-MIT` (object 0.40.0): the same text as `LICENSE-MIT` (object 0.37.3) above.

## once_cell 1.21.4

License: MIT OR Apache-2.0. Source: https://github.com/matklad/once_cell
Authors: Aleksey Kladov <aleksey.kladov@gmail.com>.

`LICENSE-APACHE` (once_cell 1.21.4): the same text as `LICENSE-APACHE` (addr2line 0.25.1) above.

`LICENSE-MIT` (once_cell 1.21.4): the same text as `LICENSE-MIT` (adler2 2.0.1) above.

## op-revm 20.0.0

License: MIT OR Apache-2.0. Source: https://github.com/ethereum-optimism/optimism
Authors: Op Stack Contributors.

`LICENSE` (op-revm 20.0.0):

    MIT License

    Copyright (c) 2021-2026 draganrakita

    Permission is hereby granted, free of charge, to any person obtaining a copy
    of this software and associated documentation files (the "Software"), to deal
    in the Software without restriction, including without limitation the rights
    to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
    copies of the Software, and to permit persons to whom the Software is
    furnished to do so, subject to the following conditions:

    The above copyright notice and this permission notice shall be included in all
    copies or substantial portions of the Software.

    THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
    IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
    FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
    AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
    LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
    OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
    SOFTWARE.

## opaque-debug 0.3.1

License: MIT OR Apache-2.0. Source: https://github.com/RustCrypto/utils
Authors: RustCrypto Developers.

`LICENSE-APACHE` (opaque-debug 0.3.1): the same text as `LICENSE-APACHE` (base16ct 0.2.0) above.

`LICENSE-MIT` (opaque-debug 0.3.1):

    Copyright (c) 2018-2024 The RustCrypto Project Developers

    Permission is hereby granted, free of charge, to any
    person obtaining a copy of this software and associated
    documentation files (the "Software"), to deal in the
    Software without restriction, including without
    limitation the rights to use, copy, modify, merge,
    publish, distribute, sublicense, and/or sell copies of
    the Software, and to permit persons to whom the Software
    is furnished to do so, subject to the following
    conditions:

    The above copyright notice and this permission notice
    shall be included in all copies or substantial portions
    of the Software.

    THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF
    ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED
    TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A
    PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT
    SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
    CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
    OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR
    IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
    DEALINGS IN THE SOFTWARE.

## openvm-circuit 2.0.2

License: MIT OR Apache-2.0. Source: https://github.com/openvm-org/
Authors: OpenVM Authors.

Its package carries no license file; these are at the root of its repository's checkout.

`LICENSE-APACHE` (openvm-circuit 2.0.2):

                                     Apache License
                               Version 2.0, January 2004
                            http://www.apache.org/licenses/

       TERMS AND CONDITIONS FOR USE, REPRODUCTION, AND DISTRIBUTION

       1. Definitions.

          "License" shall mean the terms and conditions for use, reproduction,
          and distribution as defined by Sections 1 through 9 of this document.

          "Licensor" shall mean the copyright owner or entity authorized by
          the copyright owner that is granting the License.

          "Legal Entity" shall mean the union of the acting entity and all
          other entities that control, are controlled by, or are under common
          control with that entity. For the purposes of this definition,
          "control" means (i) the power, direct or indirect, to cause the
          direction or management of such entity, whether by contract or
          otherwise, or (ii) ownership of fifty percent (50%) or more of the
          outstanding shares, or (iii) beneficial ownership of such entity.

          "You" (or "Your") shall mean an individual or Legal Entity
          exercising permissions granted by this License.

          "Source" form shall mean the preferred form for making modifications,
          including but not limited to software source code, documentation
          source, and configuration files.

          "Object" form shall mean any form resulting from mechanical
          transformation or translation of a Source form, including but
          not limited to compiled object code, generated documentation,
          and conversions to other media types.

          "Work" shall mean the work of authorship, whether in Source or
          Object form, made available under the License, as indicated by a
          copyright notice that is included in or attached to the work
          (an example is provided in the Appendix below).

          "Derivative Works" shall mean any work, whether in Source or Object
          form, that is based on (or derived from) the Work and for which the
          editorial revisions, annotations, elaborations, or other modifications
          represent, as a whole, an original work of authorship. For the purposes
          of this License, Derivative Works shall not include works that remain
          separable from, or merely link (or bind by name) to the interfaces of,
          the Work and Derivative Works thereof.

          "Contribution" shall mean any work of authorship, including
          the original version of the Work and any modifications or additions
          to that Work or Derivative Works thereof, that is intentionally
          submitted to Licensor for inclusion in the Work by the copyright owner
          or by an individual or Legal Entity authorized to submit on behalf of
          the copyright owner. For the purposes of this definition, "submitted"
          means any form of electronic, verbal, or written communication sent
          to the Licensor or its representatives, including but not limited to
          communication on electronic mailing lists, source code control systems,
          and issue tracking systems that are managed by, or on behalf of, the
          Licensor for the purpose of discussing and improving the Work, but
          excluding communication that is conspicuously marked or otherwise
          designated in writing by the copyright owner as "Not a Contribution."

          "Contributor" shall mean Licensor and any individual or Legal Entity
          on behalf of whom a Contribution has been received by Licensor and
          subsequently incorporated within the Work.

       2. Grant of Copyright License. Subject to the terms and conditions of
          this License, each Contributor hereby grants to You a perpetual,
          worldwide, non-exclusive, no-charge, royalty-free, irrevocable
          copyright license to reproduce, prepare Derivative Works of,
          publicly display, publicly perform, sublicense, and distribute the
          Work and such Derivative Works in Source or Object form.

       3. Grant of Patent License. Subject to the terms and conditions of
          this License, each Contributor hereby grants to You a perpetual,
          worldwide, non-exclusive, no-charge, royalty-free, irrevocable
          (except as stated in this section) patent license to make, have made,
          use, offer to sell, sell, import, and otherwise transfer the Work,
          where such license applies only to those patent claims licensable
          by such Contributor that are necessarily infringed by their
          Contribution(s) alone or by combination of their Contribution(s)
          with the Work to which such Contribution(s) was submitted. If You
          institute patent litigation against any entity (including a
          cross-claim or counterclaim in a lawsuit) alleging that the Work
          or a Contribution incorporated within the Work constitutes direct
          or contributory patent infringement, then any patent licenses
          granted to You under this License for that Work shall terminate
          as of the date such litigation is filed.

       4. Redistribution. You may reproduce and distribute copies of the
          Work or Derivative Works thereof in any medium, with or without
          modifications, and in Source or Object form, provided that You
          meet the following conditions:

          (a) You must give any other recipients of the Work or
              Derivative Works a copy of this License; and

          (b) You must cause any modified files to carry prominent notices
              stating that You changed the files; and

          (c) You must retain, in the Source form of any Derivative Works
              that You distribute, all copyright, patent, trademark, and
              attribution notices from the Source form of the Work,
              excluding those notices that do not pertain to any part of
              the Derivative Works; and

          (d) If the Work includes a "NOTICE" text file as part of its
              distribution, then any Derivative Works that You distribute must
              include a readable copy of the attribution notices contained
              within such NOTICE file, excluding those notices that do not
              pertain to any part of the Derivative Works, in at least one
              of the following places: within a NOTICE text file distributed
              as part of the Derivative Works; within the Source form or
              documentation, if provided along with the Derivative Works; or,
              within a display generated by the Derivative Works, if and
              wherever such third-party notices normally appear. The contents
              of the NOTICE file are for informational purposes only and
              do not modify the License. You may add Your own attribution
              notices within Derivative Works that You distribute, alongside
              or as an addendum to the NOTICE text from the Work, provided
              that such additional attribution notices cannot be construed
              as modifying the License.

          You may add Your own copyright statement to Your modifications and
          may provide additional or different license terms and conditions
          for use, reproduction, or distribution of Your modifications, or
          for any such Derivative Works as a whole, provided Your use,
          reproduction, and distribution of the Work otherwise complies with
          the conditions stated in this License.

       5. Submission of Contributions. Unless You explicitly state otherwise,
          any Contribution intentionally submitted for inclusion in the Work
          by You to the Licensor shall be under the terms and conditions of
          this License, without any additional terms or conditions.
          Notwithstanding the above, nothing herein shall supersede or modify
          the terms of any separate license agreement you may have executed
          with Licensor regarding such Contributions.

       6. Trademarks. This License does not grant permission to use the trade
          names, trademarks, service marks, or product names of the Licensor,
          except as required for reasonable and customary use in describing the
          origin of the Work and reproducing the content of the NOTICE file.

       7. Disclaimer of Warranty. Unless required by applicable law or
          agreed to in writing, Licensor provides the Work (and each
          Contributor provides its Contributions) on an "AS IS" BASIS,
          WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or
          implied, including, without limitation, any warranties or conditions
          of TITLE, NON-INFRINGEMENT, MERCHANTABILITY, or FITNESS FOR A
          PARTICULAR PURPOSE. You are solely responsible for determining the
          appropriateness of using or redistributing the Work and assume any
          risks associated with Your exercise of permissions under this License.

       8. Limitation of Liability. In no event and under no legal theory,
          whether in tort (including negligence), contract, or otherwise,
          unless required by applicable law (such as deliberate and grossly
          negligent acts) or agreed to in writing, shall any Contributor be
          liable to You for damages, including any direct, indirect, special,
          incidental, or consequential damages of any character arising as a
          result of this License or out of the use or inability to use the
          Work (including but not limited to damages for loss of goodwill,
          work stoppage, computer failure or malfunction, or any and all
          other commercial damages or losses), even if such Contributor
          has been advised of the possibility of such damages.

       9. Accepting Warranty or Additional Liability. While redistributing
          the Work or Derivative Works thereof, You may choose to offer,
          and charge a fee for, acceptance of support, warranty, indemnity,
          or other liability obligations and/or rights consistent with this
          License. However, in accepting such obligations, You may act only
          on Your own behalf and on Your sole responsibility, not on behalf
          of any other Contributor, and only if You agree to indemnify,
          defend, and hold each Contributor harmless for any liability
          incurred by, or claims asserted against, such Contributor by reason
          of your accepting any such warranty or additional liability.

       END OF TERMS AND CONDITIONS

       APPENDIX: How to apply the Apache License to your work.

          To apply the Apache License to your work, attach the following
          boilerplate notice, with the fields enclosed by brackets "[]"
          replaced with your own identifying information. (Don't include
          the brackets!)  The text should be enclosed in the appropriate
          comment syntax for the file format. We also recommend that a
          file or class name and description of purpose be included on the
          same "printed page" as the copyright notice for easier
          identification within third-party archives.

       Copyright 2025 The OpenVM Authors

       Licensed under the Apache License, Version 2.0 (the "License");
       you may not use this file except in compliance with the License.
       You may obtain a copy of the License at

           http://www.apache.org/licenses/LICENSE-2.0

       Unless required by applicable law or agreed to in writing, software
       distributed under the License is distributed on an "AS IS" BASIS,
       WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
       See the License for the specific language governing permissions and
       limitations under the License.

`LICENSE-MIT` (openvm-circuit 2.0.2):

    MIT License

    Copyright (c) 2025 The OpenVM Authors

    Permission is hereby granted, free of charge, to any person obtaining a copy
    of this software and associated documentation files (the "Software"), to deal
    in the Software without restriction, including without limitation the rights
    to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
    copies of the Software, and to permit persons to whom the Software is
    furnished to do so, subject to the following conditions:

    The above copyright notice and this permission notice shall be included in all
    copies or substantial portions of the Software.

    THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
    IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
    FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
    AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
    LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
    OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
    SOFTWARE.

## openvm-circuit-primitives 2.0.2

License: MIT OR Apache-2.0. Source: https://github.com/openvm-org/
Authors: OpenVM Authors.

Its package carries no license file; these are at the root of its repository's checkout.

`LICENSE-APACHE` (openvm-circuit-primitives 2.0.2): the same text as `LICENSE-APACHE` (openvm-circuit 2.0.2) above.

`LICENSE-MIT` (openvm-circuit-primitives 2.0.2): the same text as `LICENSE-MIT` (openvm-circuit 2.0.2) above.

## openvm-cpu-backend 2.0.1

License: MIT OR Apache-2.0. Source: https://github.com/openvm-org/stark-backend
Authors: OpenVM contributors.

Its package carries no license file; these are at the root of its repository's checkout.

`LICENSE-APACHE` (openvm-cpu-backend 2.0.1): the same text as `LICENSE-APACHE` (openvm-circuit 2.0.2) above.

`LICENSE-MIT` (openvm-cpu-backend 2.0.1): the same text as `LICENSE-MIT` (openvm-circuit 2.0.2) above.

## openvm-instructions 2.0.2

License: MIT OR Apache-2.0. Source: https://github.com/openvm-org/
Authors: OpenVM Authors.

Its package carries no license file; these are at the root of its repository's checkout.

`LICENSE-APACHE` (openvm-instructions 2.0.2): the same text as `LICENSE-APACHE` (openvm-circuit 2.0.2) above.

`LICENSE-MIT` (openvm-instructions 2.0.2): the same text as `LICENSE-MIT` (openvm-circuit 2.0.2) above.

## openvm-poseidon2-air 2.0.2

License: MIT OR Apache-2.0. Source: https://github.com/openvm-org/
Authors: OpenVM Authors.

Its package carries no license file; these are at the root of its repository's checkout.

`LICENSE-APACHE` (openvm-poseidon2-air 2.0.2): the same text as `LICENSE-APACHE` (openvm-circuit 2.0.2) above.

`LICENSE-MIT` (openvm-poseidon2-air 2.0.2): the same text as `LICENSE-MIT` (openvm-circuit 2.0.2) above.

## openvm-stark-backend 2.0.1

License: MIT OR Apache-2.0. Source: https://github.com/openvm-org/stark-backend
Authors: OpenVM contributors.

Its package carries no license file; these are at the root of its repository's checkout.

`LICENSE-APACHE` (openvm-stark-backend 2.0.1): the same text as `LICENSE-APACHE` (openvm-circuit 2.0.2) above.

`LICENSE-MIT` (openvm-stark-backend 2.0.1): the same text as `LICENSE-MIT` (openvm-circuit 2.0.2) above.

## openvm-stark-sdk 2.0.1

License: MIT OR Apache-2.0. Source: https://github.com/openvm-org/stark-backend
Authors: OpenVM contributors.

Its package carries no license file; these are at the root of its repository's checkout.

`LICENSE-APACHE` (openvm-stark-sdk 2.0.1): the same text as `LICENSE-APACHE` (openvm-circuit 2.0.2) above.

`LICENSE-MIT` (openvm-stark-sdk 2.0.1): the same text as `LICENSE-MIT` (openvm-circuit 2.0.2) above.

## openvm-verify-stark-host 2.0.2

License: MIT OR Apache-2.0. Source: https://github.com/openvm-org/
Authors: OpenVM Authors.

Its package carries no license file; these are at the root of its repository's checkout.

`LICENSE-APACHE` (openvm-verify-stark-host 2.0.2): the same text as `LICENSE-APACHE` (openvm-circuit 2.0.2) above.

`LICENSE-MIT` (openvm-verify-stark-host 2.0.2): the same text as `LICENSE-MIT` (openvm-circuit 2.0.2) above.

## ordered-float 4.6.0

License: MIT. Source: https://github.com/reem/rust-ordered-float
Authors: Jonathan Reem <jonathan.reem@gmail.com>, Matt Brubeck <mbrubeck@limpet.net>.

`LICENSE-MIT` (ordered-float 4.6.0):

    Copyright (c) 2015 Jonathan Reem

    Permission is hereby granted, free of charge, to any
    person obtaining a copy of this software and associated
    documentation files (the "Software"), to deal in the
    Software without restriction, including without
    limitation the rights to use, copy, modify, merge,
    publish, distribute, sublicense, and/or sell copies of
    the Software, and to permit persons to whom the Software
    is furnished to do so, subject to the following
    conditions:

    The above copyright notice and this permission notice
    shall be included in all copies or substantial portions
    of the Software.

    THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF
    ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED
    TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A
    PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT
    SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
    CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
    OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR
    IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
    DEALINGS IN THE SOFTWARE.

## p256 0.13.2

License: Apache-2.0 OR MIT. Source: https://github.com/RustCrypto/elliptic-curves/tree/master/p256
Authors: RustCrypto Developers.

`LICENSE-APACHE` (p256 0.13.2): the same text as `LICENSE-APACHE` (base16ct 0.2.0) above.

`LICENSE-MIT` (p256 0.13.2):

    Copyright (c) 2020-2023 RustCrypto Developers

    Permission is hereby granted, free of charge, to any
    person obtaining a copy of this software and associated
    documentation files (the "Software"), to deal in the
    Software without restriction, including without
    limitation the rights to use, copy, modify, merge,
    publish, distribute, sublicense, and/or sell copies of
    the Software, and to permit persons to whom the Software
    is furnished to do so, subject to the following
    conditions:

    The above copyright notice and this permission notice
    shall be included in all copies or substantial portions
    of the Software.

    THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF
    ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED
    TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A
    PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT
    SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
    CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
    OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR
    IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
    DEALINGS IN THE SOFTWARE.

## p3-air 0.4.3

License: MIT OR Apache-2.0. Source: https://github.com/Plonky3/Plonky3

Its package carries no license file: the standard text of its license, under its authors.

Apache-2.0 (p3-air 0.4.3): the same text as Apache-2.0 above.

MIT (p3-air 0.4.3):

    MIT License

    Copyright (c) p3-air contributors

    Permission is hereby granted, free of charge, to any person obtaining a copy of this software and associated documentation files (the "Software"), to deal in the Software without restriction, including without limitation the rights to use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of the Software, and to permit persons to whom the Software is furnished to do so, subject to the following conditions:

    The above copyright notice and this permission notice shall be included in all copies or substantial portions of the Software.

    THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

## p3-air 0.4.3-succinct

License: MIT OR Apache-2.0. Source: https://github.com/Plonky3/Plonky3

Its package carries no license file: the standard text of its license, under its authors.

Apache-2.0 (p3-air 0.4.3-succinct): the same text as Apache-2.0 above.

MIT (p3-air 0.4.3-succinct): the same text as MIT (p3-air 0.4.3) above.

## p3-baby-bear 0.4.3

License: MIT OR Apache-2.0. Source: https://github.com/Plonky3/Plonky3

Its package carries no license file: the standard text of its license, under its authors.

Apache-2.0 (p3-baby-bear 0.4.3): the same text as Apache-2.0 above.

MIT (p3-baby-bear 0.4.3):

    MIT License

    Copyright (c) p3-baby-bear contributors

    Permission is hereby granted, free of charge, to any person obtaining a copy of this software and associated documentation files (the "Software"), to deal in the Software without restriction, including without limitation the rights to use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of the Software, and to permit persons to whom the Software is furnished to do so, subject to the following conditions:

    The above copyright notice and this permission notice shall be included in all copies or substantial portions of the Software.

    THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

## p3-baby-bear 0.4.3-succinct

License: MIT OR Apache-2.0. Source: https://github.com/Plonky3/Plonky3

Its package carries no license file: the standard text of its license, under its authors.

Apache-2.0 (p3-baby-bear 0.4.3-succinct): the same text as Apache-2.0 above.

MIT (p3-baby-bear 0.4.3-succinct): the same text as MIT (p3-baby-bear 0.4.3) above.

## p3-bn254-fr 0.4.3-succinct

License: MIT OR Apache-2.0. Source: https://github.com/Plonky3/Plonky3

Its package carries no license file: the standard text of its license, under its authors.

Apache-2.0 (p3-bn254-fr 0.4.3-succinct): the same text as Apache-2.0 above.

MIT (p3-bn254-fr 0.4.3-succinct):

    MIT License

    Copyright (c) p3-bn254-fr contributors

    Permission is hereby granted, free of charge, to any person obtaining a copy of this software and associated documentation files (the "Software"), to deal in the Software without restriction, including without limitation the rights to use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of the Software, and to permit persons to whom the Software is furnished to do so, subject to the following conditions:

    The above copyright notice and this permission notice shall be included in all copies or substantial portions of the Software.

    THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

## p3-challenger 0.4.3

License: MIT OR Apache-2.0. Source: https://github.com/Plonky3/Plonky3

Its package carries no license file: the standard text of its license, under its authors.

Apache-2.0 (p3-challenger 0.4.3): the same text as Apache-2.0 above.

MIT (p3-challenger 0.4.3):

    MIT License

    Copyright (c) p3-challenger contributors

    Permission is hereby granted, free of charge, to any person obtaining a copy of this software and associated documentation files (the "Software"), to deal in the Software without restriction, including without limitation the rights to use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of the Software, and to permit persons to whom the Software is furnished to do so, subject to the following conditions:

    The above copyright notice and this permission notice shall be included in all copies or substantial portions of the Software.

    THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

## p3-challenger 0.4.3-succinct

License: MIT OR Apache-2.0. Source: https://github.com/Plonky3/Plonky3

Its package carries no license file: the standard text of its license, under its authors.

Apache-2.0 (p3-challenger 0.4.3-succinct): the same text as Apache-2.0 above.

MIT (p3-challenger 0.4.3-succinct): the same text as MIT (p3-challenger 0.4.3) above.

## p3-commit 0.4.3-succinct

License: MIT OR Apache-2.0. Source: https://github.com/Plonky3/Plonky3

Its package carries no license file: the standard text of its license, under its authors.

Apache-2.0 (p3-commit 0.4.3-succinct): the same text as Apache-2.0 above.

MIT (p3-commit 0.4.3-succinct):

    MIT License

    Copyright (c) p3-commit contributors

    Permission is hereby granted, free of charge, to any person obtaining a copy of this software and associated documentation files (the "Software"), to deal in the Software without restriction, including without limitation the rights to use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of the Software, and to permit persons to whom the Software is furnished to do so, subject to the following conditions:

    The above copyright notice and this permission notice shall be included in all copies or substantial portions of the Software.

    THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

## p3-dft 0.4.3

License: MIT OR Apache-2.0. Source: https://github.com/Plonky3/Plonky3

Its package carries no license file: the standard text of its license, under its authors.

Apache-2.0 (p3-dft 0.4.3): the same text as Apache-2.0 above.

MIT (p3-dft 0.4.3):

    MIT License

    Copyright (c) p3-dft contributors

    Permission is hereby granted, free of charge, to any person obtaining a copy of this software and associated documentation files (the "Software"), to deal in the Software without restriction, including without limitation the rights to use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of the Software, and to permit persons to whom the Software is furnished to do so, subject to the following conditions:

    The above copyright notice and this permission notice shall be included in all copies or substantial portions of the Software.

    THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

## p3-dft 0.4.3-succinct

License: MIT OR Apache-2.0. Source: https://github.com/Plonky3/Plonky3

Its package carries no license file: the standard text of its license, under its authors.

Apache-2.0 (p3-dft 0.4.3-succinct): the same text as Apache-2.0 above.

MIT (p3-dft 0.4.3-succinct): the same text as MIT (p3-dft 0.4.3) above.

## p3-field 0.4.3

License: MIT OR Apache-2.0. Source: https://github.com/Plonky3/Plonky3

Its package carries no license file: the standard text of its license, under its authors.

Apache-2.0 (p3-field 0.4.3): the same text as Apache-2.0 above.

MIT (p3-field 0.4.3):

    MIT License

    Copyright (c) p3-field contributors

    Permission is hereby granted, free of charge, to any person obtaining a copy of this software and associated documentation files (the "Software"), to deal in the Software without restriction, including without limitation the rights to use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of the Software, and to permit persons to whom the Software is furnished to do so, subject to the following conditions:

    The above copyright notice and this permission notice shall be included in all copies or substantial portions of the Software.

    THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

## p3-field 0.4.3-succinct

License: MIT OR Apache-2.0. Source: https://github.com/Plonky3/Plonky3

Its package carries no license file: the standard text of its license, under its authors.

Apache-2.0 (p3-field 0.4.3-succinct): the same text as Apache-2.0 above.

MIT (p3-field 0.4.3-succinct): the same text as MIT (p3-field 0.4.3) above.

## p3-fri 0.4.3-succinct

License: MIT OR Apache-2.0. Source: https://github.com/Plonky3/Plonky3

Its package carries no license file: the standard text of its license, under its authors.

Apache-2.0 (p3-fri 0.4.3-succinct): the same text as Apache-2.0 above.

MIT (p3-fri 0.4.3-succinct):

    MIT License

    Copyright (c) p3-fri contributors

    Permission is hereby granted, free of charge, to any person obtaining a copy of this software and associated documentation files (the "Software"), to deal in the Software without restriction, including without limitation the rights to use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of the Software, and to permit persons to whom the Software is furnished to do so, subject to the following conditions:

    The above copyright notice and this permission notice shall be included in all copies or substantial portions of the Software.

    THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

## p3-interpolation 0.4.3

License: MIT OR Apache-2.0. Source: https://github.com/Plonky3/Plonky3

Its package carries no license file: the standard text of its license, under its authors.

Apache-2.0 (p3-interpolation 0.4.3): the same text as Apache-2.0 above.

MIT (p3-interpolation 0.4.3):

    MIT License

    Copyright (c) p3-interpolation contributors

    Permission is hereby granted, free of charge, to any person obtaining a copy of this software and associated documentation files (the "Software"), to deal in the Software without restriction, including without limitation the rights to use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of the Software, and to permit persons to whom the Software is furnished to do so, subject to the following conditions:

    The above copyright notice and this permission notice shall be included in all copies or substantial portions of the Software.

    THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

## p3-interpolation 0.4.3-succinct

License: MIT OR Apache-2.0. Source: https://github.com/Plonky3/Plonky3

Its package carries no license file: the standard text of its license, under its authors.

Apache-2.0 (p3-interpolation 0.4.3-succinct): the same text as Apache-2.0 above.

MIT (p3-interpolation 0.4.3-succinct): the same text as MIT (p3-interpolation 0.4.3) above.

## p3-koala-bear 0.4.3-succinct

License: MIT OR Apache-2.0. Source: https://github.com/Plonky3/Plonky3

Its package carries no license file: the standard text of its license, under its authors.

Apache-2.0 (p3-koala-bear 0.4.3-succinct): the same text as Apache-2.0 above.

MIT (p3-koala-bear 0.4.3-succinct):

    MIT License

    Copyright (c) p3-koala-bear contributors

    Permission is hereby granted, free of charge, to any person obtaining a copy of this software and associated documentation files (the "Software"), to deal in the Software without restriction, including without limitation the rights to use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of the Software, and to permit persons to whom the Software is furnished to do so, subject to the following conditions:

    The above copyright notice and this permission notice shall be included in all copies or substantial portions of the Software.

    THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

## p3-matrix 0.4.3

License: MIT OR Apache-2.0. Source: https://github.com/Plonky3/Plonky3

Its package carries no license file: the standard text of its license, under its authors.

Apache-2.0 (p3-matrix 0.4.3): the same text as Apache-2.0 above.

MIT (p3-matrix 0.4.3):

    MIT License

    Copyright (c) p3-matrix contributors

    Permission is hereby granted, free of charge, to any person obtaining a copy of this software and associated documentation files (the "Software"), to deal in the Software without restriction, including without limitation the rights to use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of the Software, and to permit persons to whom the Software is furnished to do so, subject to the following conditions:

    The above copyright notice and this permission notice shall be included in all copies or substantial portions of the Software.

    THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

## p3-matrix 0.4.3-succinct

License: MIT OR Apache-2.0. Source: https://github.com/Plonky3/Plonky3

Its package carries no license file: the standard text of its license, under its authors.

Apache-2.0 (p3-matrix 0.4.3-succinct): the same text as Apache-2.0 above.

MIT (p3-matrix 0.4.3-succinct): the same text as MIT (p3-matrix 0.4.3) above.

## p3-maybe-rayon 0.4.3

License: MIT OR Apache-2.0. Source: https://github.com/Plonky3/Plonky3

Its package carries no license file: the standard text of its license, under its authors.

Apache-2.0 (p3-maybe-rayon 0.4.3): the same text as Apache-2.0 above.

MIT (p3-maybe-rayon 0.4.3):

    MIT License

    Copyright (c) p3-maybe-rayon contributors

    Permission is hereby granted, free of charge, to any person obtaining a copy of this software and associated documentation files (the "Software"), to deal in the Software without restriction, including without limitation the rights to use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of the Software, and to permit persons to whom the Software is furnished to do so, subject to the following conditions:

    The above copyright notice and this permission notice shall be included in all copies or substantial portions of the Software.

    THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

## p3-maybe-rayon 0.4.3-succinct

License: MIT OR Apache-2.0. Source: https://github.com/Plonky3/Plonky3

Its package carries no license file: the standard text of its license, under its authors.

Apache-2.0 (p3-maybe-rayon 0.4.3-succinct): the same text as Apache-2.0 above.

MIT (p3-maybe-rayon 0.4.3-succinct): the same text as MIT (p3-maybe-rayon 0.4.3) above.

## p3-mds 0.4.3

License: MIT OR Apache-2.0. Source: https://github.com/Plonky3/Plonky3

Its package carries no license file: the standard text of its license, under its authors.

Apache-2.0 (p3-mds 0.4.3): the same text as Apache-2.0 above.

MIT (p3-mds 0.4.3):

    MIT License

    Copyright (c) p3-mds contributors

    Permission is hereby granted, free of charge, to any person obtaining a copy of this software and associated documentation files (the "Software"), to deal in the Software without restriction, including without limitation the rights to use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of the Software, and to permit persons to whom the Software is furnished to do so, subject to the following conditions:

    The above copyright notice and this permission notice shall be included in all copies or substantial portions of the Software.

    THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

## p3-mds 0.4.3-succinct

License: MIT OR Apache-2.0. Source: https://github.com/Plonky3/Plonky3

Its package carries no license file: the standard text of its license, under its authors.

Apache-2.0 (p3-mds 0.4.3-succinct): the same text as Apache-2.0 above.

MIT (p3-mds 0.4.3-succinct): the same text as MIT (p3-mds 0.4.3) above.

## p3-merkle-tree 0.4.3-succinct

License: MIT OR Apache-2.0. Source: https://github.com/Plonky3/Plonky3

Its package carries no license file: the standard text of its license, under its authors.

Apache-2.0 (p3-merkle-tree 0.4.3-succinct): the same text as Apache-2.0 above.

MIT (p3-merkle-tree 0.4.3-succinct):

    MIT License

    Copyright (c) p3-merkle-tree contributors

    Permission is hereby granted, free of charge, to any person obtaining a copy of this software and associated documentation files (the "Software"), to deal in the Software without restriction, including without limitation the rights to use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of the Software, and to permit persons to whom the Software is furnished to do so, subject to the following conditions:

    The above copyright notice and this permission notice shall be included in all copies or substantial portions of the Software.

    THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

## p3-monty-31 0.4.3

License: MIT OR Apache-2.0. Source: https://github.com/Plonky3/Plonky3

Its package carries no license file: the standard text of its license, under its authors.

Apache-2.0 (p3-monty-31 0.4.3): the same text as Apache-2.0 above.

MIT (p3-monty-31 0.4.3):

    MIT License

    Copyright (c) p3-monty-31 contributors

    Permission is hereby granted, free of charge, to any person obtaining a copy of this software and associated documentation files (the "Software"), to deal in the Software without restriction, including without limitation the rights to use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of the Software, and to permit persons to whom the Software is furnished to do so, subject to the following conditions:

    The above copyright notice and this permission notice shall be included in all copies or substantial portions of the Software.

    THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

## p3-poseidon2 0.4.3

License: MIT OR Apache-2.0. Source: https://github.com/Plonky3/Plonky3

Its package carries no license file: the standard text of its license, under its authors.

Apache-2.0 (p3-poseidon2 0.4.3): the same text as Apache-2.0 above.

MIT (p3-poseidon2 0.4.3):

    MIT License

    Copyright (c) p3-poseidon2 contributors

    Permission is hereby granted, free of charge, to any person obtaining a copy of this software and associated documentation files (the "Software"), to deal in the Software without restriction, including without limitation the rights to use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of the Software, and to permit persons to whom the Software is furnished to do so, subject to the following conditions:

    The above copyright notice and this permission notice shall be included in all copies or substantial portions of the Software.

    THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

## p3-poseidon2 0.4.3-succinct

License: MIT OR Apache-2.0. Source: https://github.com/Plonky3/Plonky3

Its package carries no license file: the standard text of its license, under its authors.

Apache-2.0 (p3-poseidon2 0.4.3-succinct): the same text as Apache-2.0 above.

MIT (p3-poseidon2 0.4.3-succinct): the same text as MIT (p3-poseidon2 0.4.3) above.

## p3-poseidon2-air 0.4.3

License: MIT OR Apache-2.0. Source: https://github.com/Plonky3/Plonky3

Its package carries no license file: the standard text of its license, under its authors.

Apache-2.0 (p3-poseidon2-air 0.4.3): the same text as Apache-2.0 above.

MIT (p3-poseidon2-air 0.4.3):

    MIT License

    Copyright (c) p3-poseidon2-air contributors

    Permission is hereby granted, free of charge, to any person obtaining a copy of this software and associated documentation files (the "Software"), to deal in the Software without restriction, including without limitation the rights to use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of the Software, and to permit persons to whom the Software is furnished to do so, subject to the following conditions:

    The above copyright notice and this permission notice shall be included in all copies or substantial portions of the Software.

    THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

## p3-symmetric 0.4.3

License: MIT OR Apache-2.0. Source: https://github.com/Plonky3/Plonky3

Its package carries no license file: the standard text of its license, under its authors.

Apache-2.0 (p3-symmetric 0.4.3): the same text as Apache-2.0 above.

MIT (p3-symmetric 0.4.3):

    MIT License

    Copyright (c) p3-symmetric contributors

    Permission is hereby granted, free of charge, to any person obtaining a copy of this software and associated documentation files (the "Software"), to deal in the Software without restriction, including without limitation the rights to use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of the Software, and to permit persons to whom the Software is furnished to do so, subject to the following conditions:

    The above copyright notice and this permission notice shall be included in all copies or substantial portions of the Software.

    THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

## p3-symmetric 0.4.3-succinct

License: MIT OR Apache-2.0. Source: https://github.com/Plonky3/Plonky3

Its package carries no license file: the standard text of its license, under its authors.

Apache-2.0 (p3-symmetric 0.4.3-succinct): the same text as Apache-2.0 above.

MIT (p3-symmetric 0.4.3-succinct): the same text as MIT (p3-symmetric 0.4.3) above.

## p3-uni-stark 0.4.3-succinct

License: MIT OR Apache-2.0. Source: https://github.com/Plonky3/Plonky3

Its package carries no license file: the standard text of its license, under its authors.

Apache-2.0 (p3-uni-stark 0.4.3-succinct): the same text as Apache-2.0 above.

MIT (p3-uni-stark 0.4.3-succinct):

    MIT License

    Copyright (c) p3-uni-stark contributors

    Permission is hereby granted, free of charge, to any person obtaining a copy of this software and associated documentation files (the "Software"), to deal in the Software without restriction, including without limitation the rights to use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of the Software, and to permit persons to whom the Software is furnished to do so, subject to the following conditions:

    The above copyright notice and this permission notice shall be included in all copies or substantial portions of the Software.

    THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

## p3-util 0.4.3

License: MIT OR Apache-2.0. Source: https://github.com/Plonky3/Plonky3

Its package carries no license file: the standard text of its license, under its authors.

Apache-2.0 (p3-util 0.4.3): the same text as Apache-2.0 above.

MIT (p3-util 0.4.3):

    MIT License

    Copyright (c) p3-util contributors

    Permission is hereby granted, free of charge, to any person obtaining a copy of this software and associated documentation files (the "Software"), to deal in the Software without restriction, including without limitation the rights to use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of the Software, and to permit persons to whom the Software is furnished to do so, subject to the following conditions:

    The above copyright notice and this permission notice shall be included in all copies or substantial portions of the Software.

    THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

## p3-util 0.4.3-succinct

License: MIT OR Apache-2.0. Source: https://github.com/Plonky3/Plonky3

Its package carries no license file: the standard text of its license, under its authors.

Apache-2.0 (p3-util 0.4.3-succinct): the same text as Apache-2.0 above.

MIT (p3-util 0.4.3-succinct): the same text as MIT (p3-util 0.4.3) above.

## pairing 0.22.0

License: MIT/Apache-2.0. Source: https://github.com/zkcrypto/pairing
Authors: Sean Bowe <ewillbefull@gmail.com>, Jack Grigg <jack@z.cash>.

`COPYRIGHT` (pairing 0.22.0):

    Copyrights in the "pairing" library are retained by their contributors. No
    copyright assignment is required to contribute to the "pairing" library.

    The "pairing" library is licensed under either of

     * Apache License, Version 2.0, (see ./LICENSE-APACHE or http://www.apache.org/licenses/LICENSE-2.0)
     * MIT license (see ./LICENSE-MIT or http://opensource.org/licenses/MIT)

    at your option.

    Unless you explicitly state otherwise, any contribution intentionally
    submitted for inclusion in the work by you, as defined in the Apache-2.0
    license, shall be dual licensed as above, without any additional terms or
    conditions.

`LICENSE-APACHE` (pairing 0.22.0): the same text as `LICENSE-APACHE` (addr2line 0.25.1) above.

`LICENSE-MIT` (pairing 0.22.0): the same text as `LICENSE-MIT` (adler2 2.0.1) above.

## pairing 0.23.0

License: MIT/Apache-2.0. Source: https://github.com/zkcrypto/pairing
Authors: Sean Bowe <ewillbefull@gmail.com>, Jack Grigg <jack@z.cash>.

`COPYRIGHT` (pairing 0.23.0): the same text as `COPYRIGHT` (pairing 0.22.0) above.

`LICENSE-APACHE` (pairing 0.23.0): the same text as `LICENSE-APACHE` (addr2line 0.25.1) above.

`LICENSE-MIT` (pairing 0.23.0): the same text as `LICENSE-MIT` (adler2 2.0.1) above.

## parking_lot_core 0.9.12

License: MIT OR Apache-2.0. Source: https://github.com/Amanieu/parking_lot
Authors: Amanieu d'Antras <amanieu@gmail.com>.

`LICENSE-APACHE` (parking_lot_core 0.9.12): the same text as `LICENSE-APACHE` (addr2line 0.25.1) above.

`LICENSE-MIT` (parking_lot_core 0.9.12): the same text as `LICENSE-MIT` (lock_api 0.4.14) above.

## pasta_curves 0.4.1

License: MIT OR Apache-2.0. Source: https://github.com/zcash/pasta_curves
Authors: Sean Bowe <sean@electriccoin.co>, Ying Tong Lai <yingtong@electriccoin.co>, Daira Hopwood <daira@electriccoin.co>, Jack Grigg <jack@electriccoin.co>.

`COPYING.md` (pasta_curves 0.4.1):

    # License

    Licensed under either of

     * Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
     * MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

    at your option.

    # Contribution

    Unless you explicitly state otherwise, any contribution intentionally
    submitted for inclusion in the work by you, as defined in the Apache-2.0
    license, shall be dual licensed as above, without any additional terms or
    conditions.

`LICENSE-APACHE` (pasta_curves 0.4.1): the same text as `LICENSE-APACHE` (addr2line 0.25.1) above.

`LICENSE-MIT` (pasta_curves 0.4.1):

    The MIT License (MIT)

    Copyright (c) 2020-2021 The Electric Coin Company

    Permission is hereby granted, free of charge, to any person obtaining a copy
    of this software and associated documentation files (the "Software"), to deal
    in the Software without restriction, including without limitation the rights
    to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
    copies of the Software, and to permit persons to whom the Software is
    furnished to do so, subject to the following conditions:

    The above copyright notice and this permission notice shall be included in
    all copies or substantial portions of the Software.

    THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
    IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
    FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
    AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
    LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
    OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN
    THE SOFTWARE.

## pasta_curves 0.5.2

License: MIT OR Apache-2.0. Source: https://github.com/zcash/pasta_curves
Authors: Sean Bowe <sean@electriccoin.co>, Ying Tong Lai <yingtong@electriccoin.co>, Daira Hopwood <daira@electriccoin.co>, Jack Grigg <jack@electriccoin.co>.

`COPYING.md` (pasta_curves 0.5.2): the same text as `COPYING.md` (pasta_curves 0.4.1) above.

`LICENSE-APACHE` (pasta_curves 0.5.2): the same text as `LICENSE-APACHE` (addr2line 0.25.1) above.

`LICENSE-MIT` (pasta_curves 0.5.2): the same text as `LICENSE-MIT` (pasta_curves 0.4.1) above.

## pin-project 1.1.13

License: Apache-2.0 OR MIT. Source: https://github.com/taiki-e/pin-project

`LICENSE-APACHE` (pin-project 1.1.13): the same text as `LICENSE-APACHE` (ethnum 1.5.3) above.

`LICENSE-MIT` (pin-project 1.1.13): the same text as `LICENSE-MIT` (adler2 2.0.1) above.

## pin-project-lite 0.2.17

License: Apache-2.0 OR MIT. Source: https://github.com/taiki-e/pin-project-lite

`LICENSE-APACHE` (pin-project-lite 0.2.17): the same text as `LICENSE-APACHE` (ethnum 1.5.3) above.

`LICENSE-MIT` (pin-project-lite 0.2.17): the same text as `LICENSE-MIT` (adler2 2.0.1) above.

## pkcs8 0.10.2

License: Apache-2.0 OR MIT. Source: https://github.com/RustCrypto/formats/tree/master/pkcs8
Authors: RustCrypto Developers.

`LICENSE-APACHE` (pkcs8 0.10.2): the same text as `LICENSE-APACHE` (base16ct 0.2.0) above.

`LICENSE-MIT` (pkcs8 0.10.2): the same text as `LICENSE-MIT` (der 0.7.10) above.

## portable-atomic 1.15.0

License: Apache-2.0 OR MIT. Source: https://github.com/taiki-e/portable-atomic

`LICENSE-APACHE` (portable-atomic 1.15.0): the same text as `LICENSE-APACHE` (ethnum 1.5.3) above.

`LICENSE-MIT` (portable-atomic 1.15.0): the same text as `LICENSE-MIT` (adler2 2.0.1) above.

## postcard 1.1.3

License: MIT OR Apache-2.0. Source: https://github.com/jamesmunns/postcard
Authors: James Munns <james@onevariable.com>.

`LICENSE-APACHE` (postcard 1.1.3): the same text as `LICENSE-APACHE` (addr2line 0.25.1) above.

`LICENSE-MIT` (postcard 1.1.3):

    Copyright (c) 2019 Anthony James Munns

    Permission is hereby granted, free of charge, to any
    person obtaining a copy of this software and associated
    documentation files (the "Software"), to deal in the
    Software without restriction, including without
    limitation the rights to use, copy, modify, merge,
    publish, distribute, sublicense, and/or sell copies of
    the Software, and to permit persons to whom the Software
    is furnished to do so, subject to the following
    conditions:

    The above copyright notice and this permission notice
    shall be included in all copies or substantial portions
    of the Software.

    THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF
    ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED
    TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A
    PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT
    SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
    CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
    OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR
    IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
    DEALINGS IN THE SOFTWARE.

## ppv-lite86 0.2.21

License: MIT OR Apache-2.0. Source: https://github.com/cryptocorrosion/cryptocorrosion
Authors: The CryptoCorrosion Contributors.

`LICENSE-APACHE` (ppv-lite86 0.2.21):

                                  Apache License
                            Version 2.0, January 2004
                         http://www.apache.org/licenses/

    TERMS AND CONDITIONS FOR USE, REPRODUCTION, AND DISTRIBUTION

    1. Definitions.

       "License" shall mean the terms and conditions for use, reproduction,
       and distribution as defined by Sections 1 through 9 of this document.

       "Licensor" shall mean the copyright owner or entity authorized by
       the copyright owner that is granting the License.

       "Legal Entity" shall mean the union of the acting entity and all
       other entities that control, are controlled by, or are under common
       control with that entity. For the purposes of this definition,
       "control" means (i) the power, direct or indirect, to cause the
       direction or management of such entity, whether by contract or
       otherwise, or (ii) ownership of fifty percent (50%) or more of the
       outstanding shares, or (iii) beneficial ownership of such entity.

       "You" (or "Your") shall mean an individual or Legal Entity
       exercising permissions granted by this License.

       "Source" form shall mean the preferred form for making modifications,
       including but not limited to software source code, documentation
       source, and configuration files.

       "Object" form shall mean any form resulting from mechanical
       transformation or translation of a Source form, including but
       not limited to compiled object code, generated documentation,
       and conversions to other media types.

       "Work" shall mean the work of authorship, whether in Source or
       Object form, made available under the License, as indicated by a
       copyright notice that is included in or attached to the work
       (an example is provided in the Appendix below).

       "Derivative Works" shall mean any work, whether in Source or Object
       form, that is based on (or derived from) the Work and for which the
       editorial revisions, annotations, elaborations, or other modifications
       represent, as a whole, an original work of authorship. For the purposes
       of this License, Derivative Works shall not include works that remain
       separable from, or merely link (or bind by name) to the interfaces of,
       the Work and Derivative Works thereof.

       "Contribution" shall mean any work of authorship, including
       the original version of the Work and any modifications or additions
       to that Work or Derivative Works thereof, that is intentionally
       submitted to Licensor for inclusion in the Work by the copyright owner
       or by an individual or Legal Entity authorized to submit on behalf of
       the copyright owner. For the purposes of this definition, "submitted"
       means any form of electronic, verbal, or written communication sent
       to the Licensor or its representatives, including but not limited to
       communication on electronic mailing lists, source code control systems,
       and issue tracking systems that are managed by, or on behalf of, the
       Licensor for the purpose of discussing and improving the Work, but
       excluding communication that is conspicuously marked or otherwise
       designated in writing by the copyright owner as "Not a Contribution."

       "Contributor" shall mean Licensor and any individual or Legal Entity
       on behalf of whom a Contribution has been received by Licensor and
       subsequently incorporated within the Work.

    2. Grant of Copyright License. Subject to the terms and conditions of
       this License, each Contributor hereby grants to You a perpetual,
       worldwide, non-exclusive, no-charge, royalty-free, irrevocable
       copyright license to reproduce, prepare Derivative Works of,
       publicly display, publicly perform, sublicense, and distribute the
       Work and such Derivative Works in Source or Object form.

    3. Grant of Patent License. Subject to the terms and conditions of
       this License, each Contributor hereby grants to You a perpetual,
       worldwide, non-exclusive, no-charge, royalty-free, irrevocable
       (except as stated in this section) patent license to make, have made,
       use, offer to sell, sell, import, and otherwise transfer the Work,
       where such license applies only to those patent claims licensable
       by such Contributor that are necessarily infringed by their
       Contribution(s) alone or by combination of their Contribution(s)
       with the Work to which such Contribution(s) was submitted. If You
       institute patent litigation against any entity (including a
       cross-claim or counterclaim in a lawsuit) alleging that the Work
       or a Contribution incorporated within the Work constitutes direct
       or contributory patent infringement, then any patent licenses
       granted to You under this License for that Work shall terminate
       as of the date such litigation is filed.

    4. Redistribution. You may reproduce and distribute copies of the
       Work or Derivative Works thereof in any medium, with or without
       modifications, and in Source or Object form, provided that You
       meet the following conditions:

       (a) You must give any other recipients of the Work or
           Derivative Works a copy of this License; and

       (b) You must cause any modified files to carry prominent notices
           stating that You changed the files; and

       (c) You must retain, in the Source form of any Derivative Works
           that You distribute, all copyright, patent, trademark, and
           attribution notices from the Source form of the Work,
           excluding those notices that do not pertain to any part of
           the Derivative Works; and

       (d) If the Work includes a "NOTICE" text file as part of its
           distribution, then any Derivative Works that You distribute must
           include a readable copy of the attribution notices contained
           within such NOTICE file, excluding those notices that do not
           pertain to any part of the Derivative Works, in at least one
           of the following places: within a NOTICE text file distributed
           as part of the Derivative Works; within the Source form or
           documentation, if provided along with the Derivative Works; or,
           within a display generated by the Derivative Works, if and
           wherever such third-party notices normally appear. The contents
           of the NOTICE file are for informational purposes only and
           do not modify the License. You may add Your own attribution
           notices within Derivative Works that You distribute, alongside
           or as an addendum to the NOTICE text from the Work, provided
           that such additional attribution notices cannot be construed
           as modifying the License.

       You may add Your own copyright statement to Your modifications and
       may provide additional or different license terms and conditions
       for use, reproduction, or distribution of Your modifications, or
       for any such Derivative Works as a whole, provided Your use,
       reproduction, and distribution of the Work otherwise complies with
       the conditions stated in this License.

    5. Submission of Contributions. Unless You explicitly state otherwise,
       any Contribution intentionally submitted for inclusion in the Work
       by You to the Licensor shall be under the terms and conditions of
       this License, without any additional terms or conditions.
       Notwithstanding the above, nothing herein shall supersede or modify
       the terms of any separate license agreement you may have executed
       with Licensor regarding such Contributions.

    6. Trademarks. This License does not grant permission to use the trade
       names, trademarks, service marks, or product names of the Licensor,
       except as required for reasonable and customary use in describing the
       origin of the Work and reproducing the content of the NOTICE file.

    7. Disclaimer of Warranty. Unless required by applicable law or
       agreed to in writing, Licensor provides the Work (and each
       Contributor provides its Contributions) on an "AS IS" BASIS,
       WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or
       implied, including, without limitation, any warranties or conditions
       of TITLE, NON-INFRINGEMENT, MERCHANTABILITY, or FITNESS FOR A
       PARTICULAR PURPOSE. You are solely responsible for determining the
       appropriateness of using or redistributing the Work and assume any
       risks associated with Your exercise of permissions under this License.

    8. Limitation of Liability. In no event and under no legal theory,
       whether in tort (including negligence), contract, or otherwise,
       unless required by applicable law (such as deliberate and grossly
       negligent acts) or agreed to in writing, shall any Contributor be
       liable to You for damages, including any direct, indirect, special,
       incidental, or consequential damages of any character arising as a
       result of this License or out of the use or inability to use the
       Work (including but not limited to damages for loss of goodwill,
       work stoppage, computer failure or malfunction, or any and all
       other commercial damages or losses), even if such Contributor
       has been advised of the possibility of such damages.

    9. Accepting Warranty or Additional Liability. While redistributing
       the Work or Derivative Works thereof, You may choose to offer,
       and charge a fee for, acceptance of support, warranty, indemnity,
       or other liability obligations and/or rights consistent with this
       License. However, in accepting such obligations, You may act only
       on Your own behalf and on Your sole responsibility, not on behalf
       of any other Contributor, and only if You agree to indemnify,
       defend, and hold each Contributor harmless for any liability
       incurred by, or claims asserted against, such Contributor by reason
       of your accepting any such warranty or additional liability.

    END OF TERMS AND CONDITIONS

    APPENDIX: How to apply the Apache License to your work.

       To apply the Apache License to your work, attach the following
       boilerplate notice, with the fields enclosed by brackets "[]"
       replaced with your own identifying information. (Don't include
       the brackets!)  The text should be enclosed in the appropriate
       comment syntax for the file format. We also recommend that a
       file or class name and description of purpose be included on the
       same "printed page" as the copyright notice for easier
       identification within third-party archives.

    Copyright 2019 The CryptoCorrosion Contributors

    Licensed under the Apache License, Version 2.0 (the "License");
    you may not use this file except in compliance with the License.
    You may obtain a copy of the License at

       http://www.apache.org/licenses/LICENSE-2.0

    Unless required by applicable law or agreed to in writing, software
    distributed under the License is distributed on an "AS IS" BASIS,
    WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
    See the License for the specific language governing permissions and
    limitations under the License.

`LICENSE-MIT` (ppv-lite86 0.2.21):

    Copyright (c) 2019 The CryptoCorrosion Contributors

    Permission is hereby granted, free of charge, to any
    person obtaining a copy of this software and associated
    documentation files (the "Software"), to deal in the
    Software without restriction, including without
    limitation the rights to use, copy, modify, merge,
    publish, distribute, sublicense, and/or sell copies of
    the Software, and to permit persons to whom the Software
    is furnished to do so, subject to the following
    conditions:

    The above copyright notice and this permission notice
    shall be included in all copies or substantial portions
    of the Software.

    THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF
    ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED
    TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A
    PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT
    SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
    CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
    OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR
    IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
    DEALINGS IN THE SOFTWARE.

## primeorder 0.13.6

License: Apache-2.0 OR MIT. Source: https://github.com/RustCrypto/elliptic-curves/tree/master/primeorder
Authors: RustCrypto Developers.

`LICENSE-APACHE` (primeorder 0.13.6): the same text as `LICENSE-APACHE` (base16ct 0.2.0) above.

`LICENSE-MIT` (primeorder 0.13.6): the same text as `LICENSE-MIT` (p256 0.13.2) above.

## quanta 0.12.6

License: MIT. Source: https://github.com/metrics-rs/quanta
Authors: Toby Lawrence <toby@nuclearfurnace.com>.

`LICENSE` (quanta 0.12.6):

    // Copyright (c) 2019 Nuclear Furnace
    //
    // Permission is hereby granted, free of charge, to any person obtaining a copy
    // of this software and associated documentation files (the "Software"), to deal
    // in the Software without restriction, including without limitation the rights
    // to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
    // copies of the Software, and to permit persons to whom the Software is
    // furnished to do so, subject to the following conditions:
    //
    // The above copyright notice and this permission notice shall be included in all
    // copies or substantial portions of the Software.
    //
    // THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
    // IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
    // FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
    // AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
    // LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
    // OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
    // SOFTWARE.

## radium 0.7.0

License: MIT. Source: https://github.com/bitvecto-rs/radium
Authors: Nika Layzell <nika@thelayzells.com>, myrrlyn <self@myrrlyn.dev>.

`LICENSE.txt` (radium 0.7.0):

    MIT License

    Copyright (c) 2019 kneecaw (Nika Layzell)

    Permission is hereby granted, free of charge, to any person obtaining a copy
    of this software and associated documentation files (the "Software"), to deal
    in the Software without restriction, including without limitation the rights
    to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
    copies of the Software, and to permit persons to whom the Software is
    furnished to do so, subject to the following conditions:

    The above copyright notice and this permission notice shall be included in all
    copies or substantial portions of the Software.

    THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
    IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
    FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
    AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
    LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
    OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
    SOFTWARE.

## radix_trie 0.2.1

License: MIT. Source: https://github.com/michaelsproul/rust_radix_trie
Authors: Michael Sproul <micsproul@gmail.com>.

`LICENSE` (radix_trie 0.2.1): the same text as `LICENSE` (nibble_vec 0.1.0) above.

## rand 0.8.8

License: MIT OR Apache-2.0. Source: https://github.com/rust-random/rand
Authors: The Rand Project Developers, The Rust Project Developers.

`COPYRIGHT` (rand 0.8.8):

    Copyrights in the Rand project are retained by their contributors. No
    copyright assignment is required to contribute to the Rand project.

    For full authorship information, see the version control history.

    Except as otherwise noted (below and/or in individual files), Rand is
    licensed under the Apache License, Version 2.0 <LICENSE-APACHE> or
    <http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
    <LICENSE-MIT> or <http://opensource.org/licenses/MIT>, at your option.

    The Rand project includes code from the Rust project
    published under these same licenses.

`LICENSE-APACHE` (rand 0.8.8):

                                  Apache License
                            Version 2.0, January 2004
                         https://www.apache.org/licenses/

    TERMS AND CONDITIONS FOR USE, REPRODUCTION, AND DISTRIBUTION

    1. Definitions.

       "License" shall mean the terms and conditions for use, reproduction,
       and distribution as defined by Sections 1 through 9 of this document.

       "Licensor" shall mean the copyright owner or entity authorized by
       the copyright owner that is granting the License.

       "Legal Entity" shall mean the union of the acting entity and all
       other entities that control, are controlled by, or are under common
       control with that entity. For the purposes of this definition,
       "control" means (i) the power, direct or indirect, to cause the
       direction or management of such entity, whether by contract or
       otherwise, or (ii) ownership of fifty percent (50%) or more of the
       outstanding shares, or (iii) beneficial ownership of such entity.

       "You" (or "Your") shall mean an individual or Legal Entity
       exercising permissions granted by this License.

       "Source" form shall mean the preferred form for making modifications,
       including but not limited to software source code, documentation
       source, and configuration files.

       "Object" form shall mean any form resulting from mechanical
       transformation or translation of a Source form, including but
       not limited to compiled object code, generated documentation,
       and conversions to other media types.

       "Work" shall mean the work of authorship, whether in Source or
       Object form, made available under the License, as indicated by a
       copyright notice that is included in or attached to the work
       (an example is provided in the Appendix below).

       "Derivative Works" shall mean any work, whether in Source or Object
       form, that is based on (or derived from) the Work and for which the
       editorial revisions, annotations, elaborations, or other modifications
       represent, as a whole, an original work of authorship. For the purposes
       of this License, Derivative Works shall not include works that remain
       separable from, or merely link (or bind by name) to the interfaces of,
       the Work and Derivative Works thereof.

       "Contribution" shall mean any work of authorship, including
       the original version of the Work and any modifications or additions
       to that Work or Derivative Works thereof, that is intentionally
       submitted to Licensor for inclusion in the Work by the copyright owner
       or by an individual or Legal Entity authorized to submit on behalf of
       the copyright owner. For the purposes of this definition, "submitted"
       means any form of electronic, verbal, or written communication sent
       to the Licensor or its representatives, including but not limited to
       communication on electronic mailing lists, source code control systems,
       and issue tracking systems that are managed by, or on behalf of, the
       Licensor for the purpose of discussing and improving the Work, but
       excluding communication that is conspicuously marked or otherwise
       designated in writing by the copyright owner as "Not a Contribution."

       "Contributor" shall mean Licensor and any individual or Legal Entity
       on behalf of whom a Contribution has been received by Licensor and
       subsequently incorporated within the Work.

    2. Grant of Copyright License. Subject to the terms and conditions of
       this License, each Contributor hereby grants to You a perpetual,
       worldwide, non-exclusive, no-charge, royalty-free, irrevocable
       copyright license to reproduce, prepare Derivative Works of,
       publicly display, publicly perform, sublicense, and distribute the
       Work and such Derivative Works in Source or Object form.

    3. Grant of Patent License. Subject to the terms and conditions of
       this License, each Contributor hereby grants to You a perpetual,
       worldwide, non-exclusive, no-charge, royalty-free, irrevocable
       (except as stated in this section) patent license to make, have made,
       use, offer to sell, sell, import, and otherwise transfer the Work,
       where such license applies only to those patent claims licensable
       by such Contributor that are necessarily infringed by their
       Contribution(s) alone or by combination of their Contribution(s)
       with the Work to which such Contribution(s) was submitted. If You
       institute patent litigation against any entity (including a
       cross-claim or counterclaim in a lawsuit) alleging that the Work
       or a Contribution incorporated within the Work constitutes direct
       or contributory patent infringement, then any patent licenses
       granted to You under this License for that Work shall terminate
       as of the date such litigation is filed.

    4. Redistribution. You may reproduce and distribute copies of the
       Work or Derivative Works thereof in any medium, with or without
       modifications, and in Source or Object form, provided that You
       meet the following conditions:

       (a) You must give any other recipients of the Work or
           Derivative Works a copy of this License; and

       (b) You must cause any modified files to carry prominent notices
           stating that You changed the files; and

       (c) You must retain, in the Source form of any Derivative Works
           that You distribute, all copyright, patent, trademark, and
           attribution notices from the Source form of the Work,
           excluding those notices that do not pertain to any part of
           the Derivative Works; and

       (d) If the Work includes a "NOTICE" text file as part of its
           distribution, then any Derivative Works that You distribute must
           include a readable copy of the attribution notices contained
           within such NOTICE file, excluding those notices that do not
           pertain to any part of the Derivative Works, in at least one
           of the following places: within a NOTICE text file distributed
           as part of the Derivative Works; within the Source form or
           documentation, if provided along with the Derivative Works; or,
           within a display generated by the Derivative Works, if and
           wherever such third-party notices normally appear. The contents
           of the NOTICE file are for informational purposes only and
           do not modify the License. You may add Your own attribution
           notices within Derivative Works that You distribute, alongside
           or as an addendum to the NOTICE text from the Work, provided
           that such additional attribution notices cannot be construed
           as modifying the License.

       You may add Your own copyright statement to Your modifications and
       may provide additional or different license terms and conditions
       for use, reproduction, or distribution of Your modifications, or
       for any such Derivative Works as a whole, provided Your use,
       reproduction, and distribution of the Work otherwise complies with
       the conditions stated in this License.

    5. Submission of Contributions. Unless You explicitly state otherwise,
       any Contribution intentionally submitted for inclusion in the Work
       by You to the Licensor shall be under the terms and conditions of
       this License, without any additional terms or conditions.
       Notwithstanding the above, nothing herein shall supersede or modify
       the terms of any separate license agreement you may have executed
       with Licensor regarding such Contributions.

    6. Trademarks. This License does not grant permission to use the trade
       names, trademarks, service marks, or product names of the Licensor,
       except as required for reasonable and customary use in describing the
       origin of the Work and reproducing the content of the NOTICE file.

    7. Disclaimer of Warranty. Unless required by applicable law or
       agreed to in writing, Licensor provides the Work (and each
       Contributor provides its Contributions) on an "AS IS" BASIS,
       WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or
       implied, including, without limitation, any warranties or conditions
       of TITLE, NON-INFRINGEMENT, MERCHANTABILITY, or FITNESS FOR A
       PARTICULAR PURPOSE. You are solely responsible for determining the
       appropriateness of using or redistributing the Work and assume any
       risks associated with Your exercise of permissions under this License.

    8. Limitation of Liability. In no event and under no legal theory,
       whether in tort (including negligence), contract, or otherwise,
       unless required by applicable law (such as deliberate and grossly
       negligent acts) or agreed to in writing, shall any Contributor be
       liable to You for damages, including any direct, indirect, special,
       incidental, or consequential damages of any character arising as a
       result of this License or out of the use or inability to use the
       Work (including but not limited to damages for loss of goodwill,
       work stoppage, computer failure or malfunction, or any and all
       other commercial damages or losses), even if such Contributor
       has been advised of the possibility of such damages.

    9. Accepting Warranty or Additional Liability. While redistributing
       the Work or Derivative Works thereof, You may choose to offer,
       and charge a fee for, acceptance of support, warranty, indemnity,
       or other liability obligations and/or rights consistent with this
       License. However, in accepting such obligations, You may act only
       on Your own behalf and on Your sole responsibility, not on behalf
       of any other Contributor, and only if You agree to indemnify,
       defend, and hold each Contributor harmless for any liability
       incurred by, or claims asserted against, such Contributor by reason
       of your accepting any such warranty or additional liability.

    END OF TERMS AND CONDITIONS

`LICENSE-MIT` (rand 0.8.8):

    Copyright 2018 Developers of the Rand project
    Copyright (c) 2014 The Rust Project Developers

    Permission is hereby granted, free of charge, to any
    person obtaining a copy of this software and associated
    documentation files (the "Software"), to deal in the
    Software without restriction, including without
    limitation the rights to use, copy, modify, merge,
    publish, distribute, sublicense, and/or sell copies of
    the Software, and to permit persons to whom the Software
    is furnished to do so, subject to the following
    conditions:

    The above copyright notice and this permission notice
    shall be included in all copies or substantial portions
    of the Software.

    THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF
    ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED
    TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A
    PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT
    SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
    CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
    OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR
    IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
    DEALINGS IN THE SOFTWARE.

## rand 0.9.5

License: MIT OR Apache-2.0. Source: https://github.com/rust-random/rand
Authors: The Rand Project Developers, The Rust Project Developers.

`COPYRIGHT` (rand 0.9.5): the same text as `COPYRIGHT` (rand 0.8.8) above.

`LICENSE-APACHE` (rand 0.9.5): the same text as `LICENSE-APACHE` (rand 0.8.8) above.

`LICENSE-MIT` (rand 0.9.5): the same text as `LICENSE-MIT` (rand 0.8.8) above.

## rand_chacha 0.3.1

License: MIT OR Apache-2.0. Source: https://github.com/rust-random/rand
Authors: The Rand Project Developers, The Rust Project Developers, The CryptoCorrosion Contributors.

`COPYRIGHT` (rand_chacha 0.3.1): the same text as `COPYRIGHT` (rand 0.8.8) above.

`LICENSE-APACHE` (rand_chacha 0.3.1): the same text as `LICENSE-APACHE` (getrandom 0.2.17) above.

`LICENSE-MIT` (rand_chacha 0.3.1): the same text as `LICENSE-MIT` (rand 0.8.8) above.

## rand_chacha 0.9.0

License: MIT OR Apache-2.0. Source: https://github.com/rust-random/rand
Authors: The Rand Project Developers, The Rust Project Developers, The CryptoCorrosion Contributors.

`COPYRIGHT` (rand_chacha 0.9.0): the same text as `COPYRIGHT` (rand 0.8.8) above.

`LICENSE-APACHE` (rand_chacha 0.9.0): the same text as `LICENSE-APACHE` (rand 0.8.8) above.

`LICENSE-MIT` (rand_chacha 0.9.0): the same text as `LICENSE-MIT` (rand 0.8.8) above.

## rand_core 0.6.4

License: MIT OR Apache-2.0. Source: https://github.com/rust-random/rand
Authors: The Rand Project Developers, The Rust Project Developers.

`COPYRIGHT` (rand_core 0.6.4): the same text as `COPYRIGHT` (rand 0.8.8) above.

`LICENSE-APACHE` (rand_core 0.6.4):

                                  Apache License
                            Version 2.0, January 2004
                         https://www.apache.org/licenses/

    TERMS AND CONDITIONS FOR USE, REPRODUCTION, AND DISTRIBUTION

    1. Definitions.

       "License" shall mean the terms and conditions for use, reproduction,
       and distribution as defined by Sections 1 through 9 of this document.

       "Licensor" shall mean the copyright owner or entity authorized by
       the copyright owner that is granting the License.

       "Legal Entity" shall mean the union of the acting entity and all
       other entities that control, are controlled by, or are under common
       control with that entity. For the purposes of this definition,
       "control" means (i) the power, direct or indirect, to cause the
       direction or management of such entity, whether by contract or
       otherwise, or (ii) ownership of fifty percent (50%) or more of the
       outstanding shares, or (iii) beneficial ownership of such entity.

       "You" (or "Your") shall mean an individual or Legal Entity
       exercising permissions granted by this License.

       "Source" form shall mean the preferred form for making modifications,
       including but not limited to software source code, documentation
       source, and configuration files.

       "Object" form shall mean any form resulting from mechanical
       transformation or translation of a Source form, including but
       not limited to compiled object code, generated documentation,
       and conversions to other media types.

       "Work" shall mean the work of authorship, whether in Source or
       Object form, made available under the License, as indicated by a
       copyright notice that is included in or attached to the work
       (an example is provided in the Appendix below).

       "Derivative Works" shall mean any work, whether in Source or Object
       form, that is based on (or derived from) the Work and for which the
       editorial revisions, annotations, elaborations, or other modifications
       represent, as a whole, an original work of authorship. For the purposes
       of this License, Derivative Works shall not include works that remain
       separable from, or merely link (or bind by name) to the interfaces of,
       the Work and Derivative Works thereof.

       "Contribution" shall mean any work of authorship, including
       the original version of the Work and any modifications or additions
       to that Work or Derivative Works thereof, that is intentionally
       submitted to Licensor for inclusion in the Work by the copyright owner
       or by an individual or Legal Entity authorized to submit on behalf of
       the copyright owner. For the purposes of this definition, "submitted"
       means any form of electronic, verbal, or written communication sent
       to the Licensor or its representatives, including but not limited to
       communication on electronic mailing lists, source code control systems,
       and issue tracking systems that are managed by, or on behalf of, the
       Licensor for the purpose of discussing and improving the Work, but
       excluding communication that is conspicuously marked or otherwise
       designated in writing by the copyright owner as "Not a Contribution."

       "Contributor" shall mean Licensor and any individual or Legal Entity
       on behalf of whom a Contribution has been received by Licensor and
       subsequently incorporated within the Work.

    2. Grant of Copyright License. Subject to the terms and conditions of
       this License, each Contributor hereby grants to You a perpetual,
       worldwide, non-exclusive, no-charge, royalty-free, irrevocable
       copyright license to reproduce, prepare Derivative Works of,
       publicly display, publicly perform, sublicense, and distribute the
       Work and such Derivative Works in Source or Object form.

    3. Grant of Patent License. Subject to the terms and conditions of
       this License, each Contributor hereby grants to You a perpetual,
       worldwide, non-exclusive, no-charge, royalty-free, irrevocable
       (except as stated in this section) patent license to make, have made,
       use, offer to sell, sell, import, and otherwise transfer the Work,
       where such license applies only to those patent claims licensable
       by such Contributor that are necessarily infringed by their
       Contribution(s) alone or by combination of their Contribution(s)
       with the Work to which such Contribution(s) was submitted. If You
       institute patent litigation against any entity (including a
       cross-claim or counterclaim in a lawsuit) alleging that the Work
       or a Contribution incorporated within the Work constitutes direct
       or contributory patent infringement, then any patent licenses
       granted to You under this License for that Work shall terminate
       as of the date such litigation is filed.

    4. Redistribution. You may reproduce and distribute copies of the
       Work or Derivative Works thereof in any medium, with or without
       modifications, and in Source or Object form, provided that You
       meet the following conditions:

       (a) You must give any other recipients of the Work or
           Derivative Works a copy of this License; and

       (b) You must cause any modified files to carry prominent notices
           stating that You changed the files; and

       (c) You must retain, in the Source form of any Derivative Works
           that You distribute, all copyright, patent, trademark, and
           attribution notices from the Source form of the Work,
           excluding those notices that do not pertain to any part of
           the Derivative Works; and

       (d) If the Work includes a "NOTICE" text file as part of its
           distribution, then any Derivative Works that You distribute must
           include a readable copy of the attribution notices contained
           within such NOTICE file, excluding those notices that do not
           pertain to any part of the Derivative Works, in at least one
           of the following places: within a NOTICE text file distributed
           as part of the Derivative Works; within the Source form or
           documentation, if provided along with the Derivative Works; or,
           within a display generated by the Derivative Works, if and
           wherever such third-party notices normally appear. The contents
           of the NOTICE file are for informational purposes only and
           do not modify the License. You may add Your own attribution
           notices within Derivative Works that You distribute, alongside
           or as an addendum to the NOTICE text from the Work, provided
           that such additional attribution notices cannot be construed
           as modifying the License.

       You may add Your own copyright statement to Your modifications and
       may provide additional or different license terms and conditions
       for use, reproduction, or distribution of Your modifications, or
       for any such Derivative Works as a whole, provided Your use,
       reproduction, and distribution of the Work otherwise complies with
       the conditions stated in this License.

    5. Submission of Contributions. Unless You explicitly state otherwise,
       any Contribution intentionally submitted for inclusion in the Work
       by You to the Licensor shall be under the terms and conditions of
       this License, without any additional terms or conditions.
       Notwithstanding the above, nothing herein shall supersede or modify
       the terms of any separate license agreement you may have executed
       with Licensor regarding such Contributions.

    6. Trademarks. This License does not grant permission to use the trade
       names, trademarks, service marks, or product names of the Licensor,
       except as required for reasonable and customary use in describing the
       origin of the Work and reproducing the content of the NOTICE file.

    7. Disclaimer of Warranty. Unless required by applicable law or
       agreed to in writing, Licensor provides the Work (and each
       Contributor provides its Contributions) on an "AS IS" BASIS,
       WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or
       implied, including, without limitation, any warranties or conditions
       of TITLE, NON-INFRINGEMENT, MERCHANTABILITY, or FITNESS FOR A
       PARTICULAR PURPOSE. You are solely responsible for determining the
       appropriateness of using or redistributing the Work and assume any
       risks associated with Your exercise of permissions under this License.

    8. Limitation of Liability. In no event and under no legal theory,
       whether in tort (including negligence), contract, or otherwise,
       unless required by applicable law (such as deliberate and grossly
       negligent acts) or agreed to in writing, shall any Contributor be
       liable to You for damages, including any direct, indirect, special,
       incidental, or consequential damages of any character arising as a
       result of this License or out of the use or inability to use the
       Work (including but not limited to damages for loss of goodwill,
       work stoppage, computer failure or malfunction, or any and all
       other commercial damages or losses), even if such Contributor
       has been advised of the possibility of such damages.

    9. Accepting Warranty or Additional Liability. While redistributing
       the Work or Derivative Works thereof, You may choose to offer,
       and charge a fee for, acceptance of support, warranty, indemnity,
       or other liability obligations and/or rights consistent with this
       License. However, in accepting such obligations, You may act only
       on Your own behalf and on Your sole responsibility, not on behalf
       of any other Contributor, and only if You agree to indemnify,
       defend, and hold each Contributor harmless for any liability
       incurred by, or claims asserted against, such Contributor by reason
       of your accepting any such warranty or additional liability.

    END OF TERMS AND CONDITIONS

    APPENDIX: How to apply the Apache License to your work.

       To apply the Apache License to your work, attach the following
       boilerplate notice, with the fields enclosed by brackets "[]"
       replaced with your own identifying information. (Don't include
       the brackets!)  The text should be enclosed in the appropriate
       comment syntax for the file format. We also recommend that a
       file or class name and description of purpose be included on the
       same "printed page" as the copyright notice for easier
       identification within third-party archives.

`LICENSE-MIT` (rand_core 0.6.4): the same text as `LICENSE-MIT` (rand 0.8.8) above.

## rand_core 0.9.5

License: MIT OR Apache-2.0. Source: https://github.com/rust-random/rand
Authors: The Rand Project Developers, The Rust Project Developers.

`COPYRIGHT` (rand_core 0.9.5): the same text as `COPYRIGHT` (rand 0.8.8) above.

`LICENSE-APACHE` (rand_core 0.9.5): the same text as `LICENSE-APACHE` (rand_core 0.6.4) above.

`LICENSE-MIT` (rand_core 0.9.5): the same text as `LICENSE-MIT` (rand 0.8.8) above.

## range-set-blaze 0.1.16

License: MIT OR Apache-2.0. Source: https://github.com/CarlKCarlK/range-set-blaze
Authors: Carl Kadie.

`LICENSE-APACHE` (range-set-blaze 0.1.16):

                                  Apache License
                            Version 2.0, January 2004
                         http://www.apache.org/licenses/

    TERMS AND CONDITIONS FOR USE, REPRODUCTION, AND DISTRIBUTION

    1. Definitions.

       "License" shall mean the terms and conditions for use, reproduction,
       and distribution as defined by Sections 1 through 9 of this document.

       "Licensor" shall mean the copyright owner or entity authorized by
       the copyright owner that is granting the License.

       "Legal Entity" shall mean the union of the acting entity and all
       other entities that control, are controlled by, or are under common
       control with that entity. For the purposes of this definition,
       "control" means (i) the power, direct or indirect, to cause the
       direction or management of such entity, whether by contract or
       otherwise, or (ii) ownership of fifty percent (50%) or more of the
       outstanding shares, or (iii) beneficial ownership of such entity.

       "You" (or "Your") shall mean an individual or Legal Entity
       exercising permissions granted by this License.

       "Source" form shall mean the preferred form for making modifications,
       including but not limited to software source code, documentation
       source, and configuration files.

       "Object" form shall mean any form resulting from mechanical
       transformation or translation of a Source form, including but
       not limited to compiled object code, generated documentation,
       and conversions to other media types.

       "Work" shall mean the work of authorship, whether in Source or
       Object form, made available under the License, as indicated by a
       copyright notice that is included in or attached to the work
       (an example is provided in the Appendix below).

       "Derivative Works" shall mean any work, whether in Source or Object
       form, that is based on (or derived from) the Work and for which the
       editorial revisions, annotations, elaborations, or other modifications
       represent, as a whole, an original work of authorship. For the purposes
       of this License, Derivative Works shall not include works that remain
       separable from, or merely link (or bind by name) to the interfaces of,
       the Work and Derivative Works thereof.

       "Contribution" shall mean any work of authorship, including
       the original version of the Work and any modifications or additions
       to that Work or Derivative Works thereof, that is intentionally
       submitted to Licensor for inclusion in the Work by the copyright owner
       or by an individual or Legal Entity authorized to submit on behalf of
       the copyright owner. For the purposes of this definition, "submitted"
       means any form of electronic, verbal, or written communication sent
       to the Licensor or its representatives, including but not limited to
       communication on electronic mailing lists, source code control systems,
       and issue tracking systems that are managed by, or on behalf of, the
       Licensor for the purpose of discussing and improving the Work, but
       excluding communication that is conspicuously marked or otherwise
       designated in writing by the copyright owner as "Not a Contribution."

       "Contributor" shall mean Licensor and any individual or Legal Entity
       on behalf of whom a Contribution has been received by Licensor and
       subsequently incorporated within the Work.

    2. Grant of Copyright License. Subject to the terms and conditions of
       this License, each Contributor hereby grants to You a perpetual,
       worldwide, non-exclusive, no-charge, royalty-free, irrevocable
       copyright license to reproduce, prepare Derivative Works of,
       publicly display, publicly perform, sublicense, and distribute the
       Work and such Derivative Works in Source or Object form.

    3. Grant of Patent License. Subject to the terms and conditions of
       this License, each Contributor hereby grants to You a perpetual,
       worldwide, non-exclusive, no-charge, royalty-free, irrevocable
       (except as stated in this section) patent license to make, have made,
       use, offer to sell, sell, import, and otherwise transfer the Work,
       where such license applies only to those patent claims licensable
       by such Contributor that are necessarily infringed by their
       Contribution(s) alone or by combination of their Contribution(s)
       with the Work to which such Contribution(s) was submitted. If You
       institute patent litigation against any entity (including a
       cross-claim or counterclaim in a lawsuit) alleging that the Work
       or a Contribution incorporated within the Work constitutes direct
       or contributory patent infringement, then any patent licenses
       granted to You under this License for that Work shall terminate
       as of the date such litigation is filed.

    4. Redistribution. You may reproduce and distribute copies of the
       Work or Derivative Works thereof in any medium, with or without
       modifications, and in Source or Object form, provided that You
       meet the following conditions:

       (a) You must give any other recipients of the Work or
           Derivative Works a copy of this License; and

       (b) You must cause any modified files to carry prominent notices
           stating that You changed the files; and

       (c) You must retain, in the Source form of any Derivative Works
           that You distribute, all copyright, patent, trademark, and
           attribution notices from the Source form of the Work,
           excluding those notices that do not pertain to any part of
           the Derivative Works; and

       (d) If the Work includes a "NOTICE" text file as part of its
           distribution, then any Derivative Works that You distribute must
           include a readable copy of the attribution notices contained
           within such NOTICE file, excluding those notices that do not
           pertain to any part of the Derivative Works, in at least one
           of the following places: within a NOTICE text file distributed
           as part of the Derivative Works; within the Source form or
           documentation, if provided along with the Derivative Works; or,
           within a display generated by the Derivative Works, if and
           wherever such third-party notices normally appear. The contents
           of the NOTICE file are for informational purposes only and
           do not modify the License. You may add Your own attribution
           notices within Derivative Works that You distribute, alongside
           or as an addendum to the NOTICE text from the Work, provided
           that such additional attribution notices cannot be construed
           as modifying the License.

       You may add Your own copyright statement to Your modifications and
       may provide additional or different license terms and conditions
       for use, reproduction, or distribution of Your modifications, or
       for any such Derivative Works as a whole, provided Your use,
       reproduction, and distribution of the Work otherwise complies with
       the conditions stated in this License.

    5. Submission of Contributions. Unless You explicitly state otherwise,
       any Contribution intentionally submitted for inclusion in the Work
       by You to the Licensor shall be under the terms and conditions of
       this License, without any additional terms or conditions.
       Notwithstanding the above, nothing herein shall supersede or modify
       the terms of any separate license agreement you may have executed
       with Licensor regarding such Contributions.

    6. Trademarks. This License does not grant permission to use the trade
       names, trademarks, service marks, or product names of the Licensor,
       except as required for reasonable and customary use in describing the
       origin of the Work and reproducing the content of the NOTICE file.

    7. Disclaimer of Warranty. Unless required by applicable law or
       agreed to in writing, Licensor provides the Work (and each
       Contributor provides its Contributions) on an "AS IS" BASIS,
       WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or
       implied, including, without limitation, any warranties or conditions
       of TITLE, NON-INFRINGEMENT, MERCHANTABILITY, or FITNESS FOR A
       PARTICULAR PURPOSE. You are solely responsible for determining the
       appropriateness of using or redistributing the Work and assume any
       risks associated with Your exercise of permissions under this License.

    8. Limitation of Liability. In no event and under no legal theory,
       whether in tort (including negligence), contract, or otherwise,
       unless required by applicable law (such as deliberate and grossly
       negligent acts) or agreed to in writing, shall any Contributor be
       liable to You for damages, including any direct, indirect, special,
       incidental, or consequential damages of any character arising as a
       result of this License or out of the use or inability to use the
       Work (including but not limited to damages for loss of goodwill,
       work stoppage, computer failure or malfunction, or any and all
       other commercial damages or losses), even if such Contributor
       has been advised of the possibility of such damages.

    9. Accepting Warranty or Additional Liability. While redistributing
       the Work or Derivative Works thereof, You may choose to offer,
       and charge a fee for, acceptance of support, warranty, indemnity,
       or other liability obligations and/or rights consistent with this
       License. However, in accepting such obligations, You may act only
       on Your own behalf and on Your sole responsibility, not on behalf
       of any other Contributor, and only if You agree to indemnify,
       defend, and hold each Contributor harmless for any liability
       incurred by, or claims asserted against, such Contributor by reason
       of your accepting any such warranty or additional liability.

    END OF TERMS AND CONDITIONSs

`LICENSE-MIT` (range-set-blaze 0.1.16): the same text as `LICENSE-MIT` (adler2 2.0.1) above.

## rayon 1.12.0

License: MIT OR Apache-2.0. Source: https://github.com/rayon-rs/rayon

`LICENSE-APACHE` (rayon 1.12.0): the same text as `LICENSE-APACHE` (addr2line 0.25.1) above.

`LICENSE-MIT` (rayon 1.12.0): the same text as `LICENSE-MIT` (lazy_static 1.5.0) above.

## rayon-core 1.13.0

License: MIT OR Apache-2.0. Source: https://github.com/rayon-rs/rayon

`LICENSE-APACHE` (rayon-core 1.13.0): the same text as `LICENSE-APACHE` (addr2line 0.25.1) above.

`LICENSE-MIT` (rayon-core 1.13.0): the same text as `LICENSE-MIT` (lazy_static 1.5.0) above.

## rayon-scan 0.1.1

License: MIT OR Apache-2.0. Source: https://github.com/arieluy/rayon-scan
Authors: Ariel Uy.

`LICENSE-APACHE` (rayon-scan 0.1.1): the same text as `LICENSE-APACHE` (addr2line 0.25.1) above.

`LICENSE-MIT` (rayon-scan 0.1.1):

    Copyright (c) 2024 Ariel Uy

    Permission is hereby granted, free of charge, to any
    person obtaining a copy of this software and associated
    documentation files (the "Software"), to deal in the
    Software without restriction, including without
    limitation the rights to use, copy, modify, merge,
    publish, distribute, sublicense, and/or sell copies of
    the Software, and to permit persons to whom the Software
    is furnished to do so, subject to the following
    conditions:

    The above copyright notice and this permission notice
    shall be included in all copies or substantial portions
    of the Software.

    THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF
    ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED
    TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A
    PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT
    SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
    CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
    OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR
    IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
    DEALINGS IN THE SOFTWARE.

`NOTICE` (rayon-scan 0.1.1):

    Copyright 2024 Ariel Uy

    Licensed under the Apache License, Version 2.0 (the "License");
    you may not use this file except in compliance with the License.
    You may obtain a copy of the License at

    	http://www.apache.org/licenses/LICENSE-2.0

    Unless required by applicable law or agreed to in writing, software
    distributed under the License is distributed on an "AS IS" BASIS,
    WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
    See the License for the specific language governing permissions and
    limitations under the License.

## regex-automata 0.4.18

License: MIT OR Apache-2.0. Source: https://github.com/rust-lang/regex
Authors: The Rust Project Developers, Andrew Gallant <jamslam@gmail.com>.

`LICENSE-APACHE` (regex-automata 0.4.18): the same text as `LICENSE-APACHE` (addr2line 0.25.1) above.

`LICENSE-MIT` (regex-automata 0.4.18): the same text as `LICENSE-MIT` (bitflags 2.13.2) above.

## regex-syntax 0.8.11

License: MIT OR Apache-2.0. Source: https://github.com/rust-lang/regex
Authors: The Rust Project Developers, Andrew Gallant <jamslam@gmail.com>.

`LICENSE-APACHE` (regex-syntax 0.8.11): the same text as `LICENSE-APACHE` (addr2line 0.25.1) above.

`LICENSE-MIT` (regex-syntax 0.8.11): the same text as `LICENSE-MIT` (bitflags 2.13.2) above.

## revm 29.0.1

License: MIT. Source: https://github.com/bluealloy/revm
Authors: Dragan Rakita <dragan0rakita@gmail.com>.

`LICENSE` (revm 29.0.1):

    MIT License

    Copyright (c) 2021-2025 draganrakita

    Permission is hereby granted, free of charge, to any person obtaining a copy
    of this software and associated documentation files (the "Software"), to deal
    in the Software without restriction, including without limitation the rights
    to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
    copies of the Software, and to permit persons to whom the Software is
    furnished to do so, subject to the following conditions:

    The above copyright notice and this permission notice shall be included in all
    copies or substantial portions of the Software.

    THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
    IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
    FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
    AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
    LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
    OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
    SOFTWARE.

## revm 38.0.0

License: MIT. Source: https://github.com/bluealloy/revm
Authors: Dragan Rakita <dragan0rakita@gmail.com>.

`LICENSE` (revm 38.0.0): the same text as `LICENSE` (op-revm 20.0.0) above.

## revm-bytecode 10.0.0

License: MIT. Source: https://github.com/bluealloy/revm
Authors: Dragan Rakita <dragan0rakita@gmail.com>.

`LICENSE` (revm-bytecode 10.0.0): the same text as `LICENSE` (op-revm 20.0.0) above.

## revm-bytecode 6.2.2

License: MIT. Source: https://github.com/bluealloy/revm
Authors: Dragan Rakita <dragan0rakita@gmail.com>.

`LICENSE` (revm-bytecode 6.2.2): the same text as `LICENSE` (revm 29.0.1) above.

## revm-context 16.0.1

License: MIT. Source: https://github.com/bluealloy/revm
Authors: Dragan Rakita <dragan0rakita@gmail.com>.

`LICENSE` (revm-context 16.0.1): the same text as `LICENSE` (op-revm 20.0.0) above.

## revm-context 9.1.0

License: MIT. Source: https://github.com/bluealloy/revm
Authors: Dragan Rakita <dragan0rakita@gmail.com>.

`LICENSE` (revm-context 9.1.0): the same text as `LICENSE` (revm 29.0.1) above.

## revm-context-interface 10.2.0

License: MIT. Source: https://github.com/bluealloy/revm
Authors: Dragan Rakita <dragan0rakita@gmail.com>.

`LICENSE` (revm-context-interface 10.2.0): the same text as `LICENSE` (revm 29.0.1) above.

## revm-context-interface 17.0.1

License: MIT. Source: https://github.com/bluealloy/revm
Authors: Dragan Rakita <dragan0rakita@gmail.com>.

`LICENSE` (revm-context-interface 17.0.1): the same text as `LICENSE` (op-revm 20.0.0) above.

## revm-database 13.0.1

License: MIT. Source: https://github.com/bluealloy/revm
Authors: Dragan Rakita <dragan0rakita@gmail.com>.

`LICENSE` (revm-database 13.0.1): the same text as `LICENSE` (op-revm 20.0.0) above.

## revm-database 7.0.5

License: MIT. Source: https://github.com/bluealloy/revm
Authors: Dragan Rakita <dragan0rakita@gmail.com>.

`LICENSE` (revm-database 7.0.5): the same text as `LICENSE` (revm 29.0.1) above.

## revm-database-interface 11.0.1

License: MIT. Source: https://github.com/bluealloy/revm
Authors: Dragan Rakita <dragan0rakita@gmail.com>.

`LICENSE` (revm-database-interface 11.0.1): the same text as `LICENSE` (op-revm 20.0.0) above.

## revm-database-interface 7.0.5

License: MIT. Source: https://github.com/bluealloy/revm
Authors: Dragan Rakita <dragan0rakita@gmail.com>.

`LICENSE` (revm-database-interface 7.0.5): the same text as `LICENSE` (revm 29.0.1) above.

## revm-handler 10.0.1

License: MIT. Source: https://github.com/bluealloy/revm
Authors: Dragan Rakita <dragan0rakita@gmail.com>.

`LICENSE` (revm-handler 10.0.1): the same text as `LICENSE` (revm 29.0.1) above.

## revm-handler 18.1.0

License: MIT. Source: https://github.com/bluealloy/revm
Authors: Dragan Rakita <dragan0rakita@gmail.com>.

`LICENSE` (revm-handler 18.1.0): the same text as `LICENSE` (op-revm 20.0.0) above.

## revm-inspector 10.0.1

License: MIT. Source: https://github.com/bluealloy/revm
Authors: Dragan Rakita <dragan0rakita@gmail.com>.

`LICENSE` (revm-inspector 10.0.1): the same text as `LICENSE` (revm 29.0.1) above.

## revm-inspector 19.0.0

License: MIT. Source: https://github.com/bluealloy/revm
Authors: Dragan Rakita <dragan0rakita@gmail.com>.

`LICENSE` (revm-inspector 19.0.0): the same text as `LICENSE` (op-revm 20.0.0) above.

## revm-inspectors 0.30.1

License: MIT OR Apache-2.0. Source: https://github.com/paradigmxyz/revm-inspectors

`LICENSE-APACHE` (revm-inspectors 0.30.1): the same text as `LICENSE-APACHE` (allocator-api2 0.2.21) above.

`LICENSE-MIT` (revm-inspectors 0.30.1): the same text as `LICENSE-MIT` (adler2 2.0.1) above.

## revm-inspectors 0.39.2

License: MIT OR Apache-2.0. Source: https://github.com/paradigmxyz/revm-inspectors

`LICENSE-APACHE` (revm-inspectors 0.39.2): the same text as `LICENSE-APACHE` (allocator-api2 0.2.21) above.

`LICENSE-MIT` (revm-inspectors 0.39.2): the same text as `LICENSE-MIT` (adler2 2.0.1) above.

## revm-interpreter 25.0.3

License: MIT. Source: https://github.com/bluealloy/revm
Authors: Dragan Rakita <dragan0rakita@gmail.com>.

`LICENSE` (revm-interpreter 25.0.3): the same text as `LICENSE` (revm 29.0.1) above.

## revm-interpreter 35.0.1

License: MIT. Source: https://github.com/bluealloy/revm
Authors: Dragan Rakita <dragan0rakita@gmail.com>.

`LICENSE` (revm-interpreter 35.0.1): the same text as `LICENSE` (op-revm 20.0.0) above.

## revm-precompile 27.0.0

License: MIT. Source: https://github.com/bluealloy/revm
Authors: Dragan Rakita <dragan0rakita@gmail.com>.

`LICENSE` (revm-precompile 27.0.0): the same text as `LICENSE` (revm 29.0.1) above.

## revm-precompile 34.0.0

License: MIT. Source: https://github.com/bluealloy/revm
Authors: Dragan Rakita <dragan0rakita@gmail.com>.

`LICENSE` (revm-precompile 34.0.0): the same text as `LICENSE` (op-revm 20.0.0) above.

## revm-primitives 20.2.1

License: MIT. Source: https://github.com/bluealloy/revm
Authors: Dragan Rakita <dragan0rakita@gmail.com>.

`LICENSE` (revm-primitives 20.2.1): the same text as `LICENSE` (revm 29.0.1) above.

## revm-primitives 23.0.0

License: MIT. Source: https://github.com/bluealloy/revm
Authors: Dragan Rakita <dragan0rakita@gmail.com>.

`LICENSE` (revm-primitives 23.0.0): the same text as `LICENSE` (op-revm 20.0.0) above.

## revm-state 11.0.1

License: MIT. Source: https://github.com/bluealloy/revm
Authors: Dragan Rakita <dragan0rakita@gmail.com>.

`LICENSE` (revm-state 11.0.1): the same text as `LICENSE` (op-revm 20.0.0) above.

## revm-state 7.0.5

License: MIT. Source: https://github.com/bluealloy/revm
Authors: Dragan Rakita <dragan0rakita@gmail.com>.

`LICENSE` (revm-state 7.0.5): the same text as `LICENSE` (revm 29.0.1) above.

## rfc6979 0.4.0

License: Apache-2.0 OR MIT. Source: https://github.com/RustCrypto/signatures/tree/master/rfc6979
Authors: RustCrypto Developers.

`LICENSE-APACHE` (rfc6979 0.4.0): the same text as `LICENSE-APACHE` (ecdsa 0.16.9) above.

`LICENSE-MIT` (rfc6979 0.4.0): the same text as `LICENSE-MIT` (ecdsa 0.16.9) above.

## ripemd 0.1.3

License: MIT OR Apache-2.0. Source: https://github.com/RustCrypto/hashes
Authors: RustCrypto Developers.

`LICENSE-APACHE` (ripemd 0.1.3): the same text as `LICENSE-APACHE` (base16ct 0.2.0) above.

`LICENSE-MIT` (ripemd 0.1.3): the same text as `LICENSE-MIT` (crypto-common 0.1.6) above.

## ruint 1.20.0

License: MIT. Source: https://github.com/alloy-rs/ruint
Authors: Remco Bloemen <remco@wicked.ventures>.

Its package carries no license file: the standard text of its license, under its authors.

MIT (ruint 1.20.0):

    MIT License

    Copyright (c) Remco Bloemen <remco@wicked.ventures>

    Permission is hereby granted, free of charge, to any person obtaining a copy of this software and associated documentation files (the "Software"), to deal in the Software without restriction, including without limitation the rights to use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of the Software, and to permit persons to whom the Software is furnished to do so, subject to the following conditions:

    The above copyright notice and this permission notice shall be included in all copies or substantial portions of the Software.

    THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

## ruint 1.20.1

License: MIT. Source: https://github.com/alloy-rs/ruint
Authors: Remco Bloemen <remco@wicked.ventures>.

Its package carries no license file: the standard text of its license, under its authors.

MIT (ruint 1.20.1): the same text as MIT (ruint 1.20.0) above.

## ruint 1.21.1

License: MIT. Source: https://github.com/alloy-rs/ruint
Authors: Remco Bloemen <remco@wicked.ventures>.

Its package carries no license file: the standard text of its license, under its authors.

MIT (ruint 1.21.1): the same text as MIT (ruint 1.20.0) above.

## rustc-demangle 0.1.28

License: MIT/Apache-2.0. Source: https://github.com/rust-lang/rustc-demangle
Authors: Alex Crichton <alex@alexcrichton.com>.

`LICENSE-APACHE` (rustc-demangle 0.1.28): the same text as `LICENSE-APACHE` (addr2line 0.25.1) above.

`LICENSE-MIT` (rustc-demangle 0.1.28): the same text as `LICENSE-MIT` (backtrace 0.3.76) above.

## rustc-hash 2.1.3

License: Apache-2.0 OR MIT. Source: https://github.com/rust-lang/rustc-hash
Authors: The Rust Project Developers.

`LICENSE-APACHE` (rustc-hash 2.1.3): the same text as `LICENSE-APACHE` (allocator-api2 0.2.21) above.

`LICENSE-MIT` (rustc-hash 2.1.3): the same text as `LICENSE-MIT` (adler2 2.0.1) above.

## rustc-hex 2.1.0

License: MIT/Apache-2.0. Source: https://github.com/debris/rustc-hex
Authors: The Rust Project Developers, debris <marek.kotewicz@gmail.com>, Robert Habermeier.

`LICENSE-APACHE` (rustc-hex 2.1.0): the same text as `LICENSE-APACHE` (addr2line 0.25.1) above.

`LICENSE-MIT` (rustc-hex 2.1.0): the same text as `LICENSE-MIT` (bitflags 2.13.2) above.

## scopeguard 1.2.0

License: MIT OR Apache-2.0. Source: https://github.com/bluss/scopeguard
Authors: bluss.

`LICENSE-APACHE` (scopeguard 1.2.0): the same text as `LICENSE-APACHE` (addr2line 0.25.1) above.

`LICENSE-MIT` (scopeguard 1.2.0):

    Copyright (c) 2016-2019 Ulrik Sverdrup "bluss" and scopeguard developers

    Permission is hereby granted, free of charge, to any
    person obtaining a copy of this software and associated
    documentation files (the "Software"), to deal in the
    Software without restriction, including without
    limitation the rights to use, copy, modify, merge,
    publish, distribute, sublicense, and/or sell copies of
    the Software, and to permit persons to whom the Software
    is furnished to do so, subject to the following
    conditions:

    The above copyright notice and this permission notice
    shall be included in all copies or substantial portions
    of the Software.

    THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF
    ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED
    TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A
    PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT
    SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
    CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
    OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR
    IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
    DEALINGS IN THE SOFTWARE.

## sec1 0.7.3

License: Apache-2.0 OR MIT. Source: https://github.com/RustCrypto/formats/tree/master/sec1
Authors: RustCrypto Developers.

`LICENSE-APACHE` (sec1 0.7.3): the same text as `LICENSE-APACHE` (base16ct 0.2.0) above.

`LICENSE-MIT` (sec1 0.7.3):

    Copyright (c) 2021-2022 The RustCrypto Project Developers

    Permission is hereby granted, free of charge, to any
    person obtaining a copy of this software and associated
    documentation files (the "Software"), to deal in the
    Software without restriction, including without
    limitation the rights to use, copy, modify, merge,
    publish, distribute, sublicense, and/or sell copies of
    the Software, and to permit persons to whom the Software
    is furnished to do so, subject to the following
    conditions:

    The above copyright notice and this permission notice
    shall be included in all copies or substantial portions
    of the Software.

    THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF
    ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED
    TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A
    PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT
    SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
    CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
    OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR
    IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
    DEALINGS IN THE SOFTWARE.

## serde 1.0.229

License: MIT OR Apache-2.0. Source: https://github.com/serde-rs/serde
Authors: Erick Tryzelaar <erick.tryzelaar@gmail.com>, David Tolnay <dtolnay@gmail.com>.

`LICENSE-APACHE` (serde 1.0.229): the same text as `LICENSE-APACHE` (allocator-api2 0.2.21) above.

`LICENSE-MIT` (serde 1.0.229): the same text as `LICENSE-MIT` (adler2 2.0.1) above.

## serde-big-array 0.5.1

License: MIT OR Apache-2.0. Source: https://github.com/est31/serde-big-array
Authors: est31 <MTest31@outlook.com>, David Tolnay <dtolnay@gmail.com>.

`LICENSE-APACHE` (serde-big-array 0.5.1):

    Copyright 2018

    Licensed under the Apache License, Version 2.0 (the "License");
    you may not use this file except in compliance with the License.
    You may obtain a copy of the License at

        http://www.apache.org/licenses/LICENSE-2.0

    Unless required by applicable law or agreed to in writing, software
    distributed under the License is distributed on an "AS IS" BASIS,
    WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
    See the License for the specific language governing permissions and
    limitations under the License.

`LICENSE-MIT` (serde-big-array 0.5.1):

    The MIT License (MIT)

    Copyright (c) 2018

    Permission is hereby granted, free of charge, to any person obtaining a copy
    of this software and associated documentation files (the "Software"), to deal
    in the Software without restriction, including without limitation the rights
    to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
    copies of the Software, and to permit persons to whom the Software is
    furnished to do so, subject to the following conditions:

    The above copyright notice and this permission notice shall be included in all
    copies or substantial portions of the Software.

    THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
    IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
    FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
    AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
    LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
    OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
    SOFTWARE.

## serde_core 1.0.229

License: MIT OR Apache-2.0. Source: https://github.com/serde-rs/serde
Authors: Erick Tryzelaar <erick.tryzelaar@gmail.com>, David Tolnay <dtolnay@gmail.com>.

`LICENSE-APACHE` (serde_core 1.0.229): the same text as `LICENSE-APACHE` (allocator-api2 0.2.21) above.

`LICENSE-MIT` (serde_core 1.0.229): the same text as `LICENSE-MIT` (adler2 2.0.1) above.

## serde_json 1.0.151

License: MIT OR Apache-2.0. Source: https://github.com/serde-rs/json
Authors: Erick Tryzelaar <erick.tryzelaar@gmail.com>, David Tolnay <dtolnay@gmail.com>.

`LICENSE-APACHE` (serde_json 1.0.151): the same text as `LICENSE-APACHE` (allocator-api2 0.2.21) above.

`LICENSE-MIT` (serde_json 1.0.151): the same text as `LICENSE-MIT` (adler2 2.0.1) above.

## serde_json 1.0.154

License: MIT OR Apache-2.0. Source: https://github.com/serde-rs/json

`LICENSE-APACHE` (serde_json 1.0.154): the same text as `LICENSE-APACHE` (allocator-api2 0.2.21) above.

`LICENSE-MIT` (serde_json 1.0.154): the same text as `LICENSE-MIT` (adler2 2.0.1) above.

## serde_with 3.22.0

License: MIT OR Apache-2.0. Source: https://github.com/jonasbb/serde_with/
Authors: Jonas Bushart, Marcin Kaźmierczak.

`LICENSE-APACHE` (serde_with 3.22.0): the same text as `LICENSE-APACHE` (addr2line 0.25.1) above.

`LICENSE-MIT` (serde_with 3.22.0): the same text as `LICENSE-MIT` (either 1.18.0) above.

## serde_with 3.24.0

License: MIT OR Apache-2.0. Source: https://github.com/jonasbb/serde_with/
Authors: Jonas Bushart, Marcin Kaźmierczak.

`LICENSE-APACHE` (serde_with 3.24.0): the same text as `LICENSE-APACHE` (addr2line 0.25.1) above.

`LICENSE-MIT` (serde_with 3.24.0): the same text as `LICENSE-MIT` (either 1.18.0) above.

## serdect 0.2.0

License: Apache-2.0 OR MIT. Source: https://github.com/RustCrypto/formats/tree/master/serdect
Authors: RustCrypto Developers.

`LICENSE-APACHE` (serdect 0.2.0):

                                     Apache License
                               Version 2.0, January 2004
                            http://www.apache.org/licenses/

       TERMS AND CONDITIONS FOR USE, REPRODUCTION, AND DISTRIBUTION

       1. Definitions.

          "License" shall mean the terms and conditions for use, reproduction,
          and distribution as defined by Sections 1 through 9 of this document.

          "Licensor" shall mean the copyright owner or entity authorized by
          the copyright owner that is granting the License.

          "Legal Entity" shall mean the union of the acting entity and all
          other entities that control, are controlled by, or are under common
          control with that entity. For the purposes of this definition,
          "control" means (i) the power, direct or indirect, to cause the
          direction or management of such entity, whether by contract or
          otherwise, or (ii) ownership of fifty percent (50%) or more of the
          outstanding shares, or (iii) beneficial ownership of such entity.

          "You" (or "Your") shall mean an individual or Legal Entity
          exercising permissions granted by this License.

          "Source" form shall mean the preferred form for making modifications,
          including but not limited to software source code, documentation
          source, and configuration files.

          "Object" form shall mean any form resulting from mechanical
          transformation or translation of a Source form, including but
          not limited to compiled object code, generated documentation,
          and conversions to other media types.

          "Work" shall mean the work of authorship, whether in Source or
          Object form, made available under the License, as indicated by a
          copyright notice that is included in or attached to the work
          (an example is provided in the Appendix below).

          "Derivative Works" shall mean any work, whether in Source or Object
          form, that is based on (or derived from) the Work and for which the
          editorial revisions, annotations, elaborations, or other modifications
          represent, as a whole, an original work of authorship. For the purposes
          of this License, Derivative Works shall not include works that remain
          separable from, or merely link (or bind by name) to the interfaces of,
          the Work and Derivative Works thereof.

          "Contribution" shall mean any work of authorship, including
          the original version of the Work and any modifications or additions
          to that Work or Derivative Works thereof, that is intentionally
          submitted to Licensor for inclusion in the Work by the copyright owner
          or by an individual or Legal Entity authorized to submit on behalf of
          the copyright owner. For the purposes of this definition, "submitted"
          means any form of electronic, verbal, or written communication sent
          to the Licensor or its representatives, including but not limited to
          communication on electronic mailing lists, source code control systems,
          and issue tracking systems that are managed by, or on behalf of, the
          Licensor for the purpose of discussing and improving the Work, but
          excluding communication that is conspicuously marked or otherwise
          designated in writing by the copyright owner as "Not a Contribution."

          "Contributor" shall mean Licensor and any individual or Legal Entity
          on behalf of whom a Contribution has been received by Licensor and
          subsequently incorporated within the Work.

       2. Grant of Copyright License. Subject to the terms and conditions of
          this License, each Contributor hereby grants to You a perpetual,
          worldwide, non-exclusive, no-charge, royalty-free, irrevocable
          copyright license to reproduce, prepare Derivative Works of,
          publicly display, publicly perform, sublicense, and distribute the
          Work and such Derivative Works in Source or Object form.

       3. Grant of Patent License. Subject to the terms and conditions of
          this License, each Contributor hereby grants to You a perpetual,
          worldwide, non-exclusive, no-charge, royalty-free, irrevocable
          (except as stated in this section) patent license to make, have made,
          use, offer to sell, sell, import, and otherwise transfer the Work,
          where such license applies only to those patent claims licensable
          by such Contributor that are necessarily infringed by their
          Contribution(s) alone or by combination of their Contribution(s)
          with the Work to which such Contribution(s) was submitted. If You
          institute patent litigation against any entity (including a
          cross-claim or counterclaim in a lawsuit) alleging that the Work
          or a Contribution incorporated within the Work constitutes direct
          or contributory patent infringement, then any patent licenses
          granted to You under this License for that Work shall terminate
          as of the date such litigation is filed.

       4. Redistribution. You may reproduce and distribute copies of the
          Work or Derivative Works thereof in any medium, with or without
          modifications, and in Source or Object form, provided that You
          meet the following conditions:

          (a) You must give any other recipients of the Work or
              Derivative Works a copy of this License; and

          (b) You must cause any modified files to carry prominent notices
              stating that You changed the files; and

          (c) You must retain, in the Source form of any Derivative Works
              that You distribute, all copyright, patent, trademark, and
              attribution notices from the Source form of the Work,
              excluding those notices that do not pertain to any part of
              the Derivative Works; and

          (d) If the Work includes a "NOTICE" text file as part of its
              distribution, then any Derivative Works that You distribute must
              include a readable copy of the attribution notices contained
              within such NOTICE file, excluding those notices that do not
              pertain to any part of the Derivative Works, in at least one
              of the following places: within a NOTICE text file distributed
              as part of the Derivative Works; within the Source form or
              documentation, if provided along with the Derivative Works; or,
              within a display generated by the Derivative Works, if and
              wherever such third-party notices normally appear. The contents
              of the NOTICE file are for informational purposes only and
              do not modify the License. You may add Your own attribution
              notices within Derivative Works that You distribute, alongside
              or as an addendum to the NOTICE text from the Work, provided
              that such additional attribution notices cannot be construed
              as modifying the License.

          You may add Your own copyright statement to Your modifications and
          may provide additional or different license terms and conditions
          for use, reproduction, or distribution of Your modifications, or
          for any such Derivative Works as a whole, provided Your use,
          reproduction, and distribution of the Work otherwise complies with
          the conditions stated in this License.

       5. Submission of Contributions. Unless You explicitly state otherwise,
          any Contribution intentionally submitted for inclusion in the Work
          by You to the Licensor shall be under the terms and conditions of
          this License, without any additional terms or conditions.
          Notwithstanding the above, nothing herein shall supersede or modify
          the terms of any separate license agreement you may have executed
          with Licensor regarding such Contributions.

       6. Trademarks. This License does not grant permission to use the trade
          names, trademarks, service marks, or product names of the Licensor,
          except as required for reasonable and customary use in describing the
          origin of the Work and reproducing the content of the NOTICE file.

       7. Disclaimer of Warranty. Unless required by applicable law or
          agreed to in writing, Licensor provides the Work (and each
          Contributor provides its Contributions) on an "AS IS" BASIS,
          WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or
          implied, including, without limitation, any warranties or conditions
          of TITLE, NON-INFRINGEMENT, MERCHANTABILITY, or FITNESS FOR A
          PARTICULAR PURPOSE. You are solely responsible for determining the
          appropriateness of using or redistributing the Work and assume any
          risks associated with Your exercise of permissions under this License.

       8. Limitation of Liability. In no event and under no legal theory,
          whether in tort (including negligence), contract, or otherwise,
          unless required by applicable law (such as deliberate and grossly
          negligent acts) or agreed to in writing, shall any Contributor be
          liable to You for damages, including any direct, indirect, special,
          incidental, or consequential damages of any character arising as a
          result of this License or out of the use or inability to use the
          Work (including but not limited to damages for loss of goodwill,
          work stoppage, computer failure or malfunction, or any and all
          other commercial damages or losses), even if such Contributor
          has been advised of the possibility of such damages.

       9. Accepting Warranty or Additional Liability. While redistributing
          the Work or Derivative Works thereof, You may choose to offer,
          and charge a fee for, acceptance of support, warranty, indemnity,
          or other liability obligations and/or rights consistent with this
          License. However, in accepting such obligations, You may act only
          on Your own behalf and on Your sole responsibility, not on behalf
          of any other Contributor, and only if You agree to indemnify,
          defend, and hold each Contributor harmless for any liability
          incurred by, or claims asserted against, such Contributor by reason
          of your accepting any such warranty or additional liability.

       END OF TERMS AND CONDITIONS

       APPENDIX: How to apply the Apache License to your work.

          To apply the Apache License to your work, attach the following
          boilerplate notice, with the fields enclosed by brackets "[]"
          replaced with your own identifying information. (Don't include
          the brackets!)  The text should be enclosed in the appropriate
          comment syntax for the file format. We also recommend that a
          file or class name and description of purpose be included on the
          same "printed page" as the copyright notice for easier
          identification within third-party archives.

       Copyright [yyyy] [name of copyright owner]

       Licensed under the Apache License, Version 2.0 (the "License");
       you may not use this file except in compliance with the License.
       You may obtain a copy of the License at

           http://www.apache.org/licenses/LICENSE-2.0

       Unless required by applicable law or agreed to in writing, software
       distributed under the License is distributed on an "AS IS" BASIS,
       WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
       See the License for the specific language governing permissions and
       limitations under the License.

`LICENSE-MIT` (serdect 0.2.0):

    MIT License

    Copyright (c) 2022 The RustCrypto Project Developers

    Permission is hereby granted, free of charge, to any person obtaining a copy
    of this software and associated documentation files (the "Software"), to deal
    in the Software without restriction, including without limitation the rights
    to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
    copies of the Software, and to permit persons to whom the Software is
    furnished to do so, subject to the following conditions:

    The above copyright notice and this permission notice shall be included in all
    copies or substantial portions of the Software.

    THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
    IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
    FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
    AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
    LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
    OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
    SOFTWARE.

## sha2 0.10.9

License: MIT OR Apache-2.0. Source: https://github.com/RustCrypto/hashes
Authors: RustCrypto Developers.

`LICENSE-APACHE` (sha2 0.10.9): the same text as `LICENSE-APACHE` (base16ct 0.2.0) above.

`LICENSE-MIT` (sha2 0.10.9):

    Copyright (c) 2006-2009 Graydon Hoare
    Copyright (c) 2009-2013 Mozilla Foundation
    Copyright (c) 2016 Artyom Pavlov

    Permission is hereby granted, free of charge, to any
    person obtaining a copy of this software and associated
    documentation files (the "Software"), to deal in the
    Software without restriction, including without
    limitation the rights to use, copy, modify, merge,
    publish, distribute, sublicense, and/or sell copies of
    the Software, and to permit persons to whom the Software
    is furnished to do so, subject to the following
    conditions:

    The above copyright notice and this permission notice
    shall be included in all copies or substantial portions
    of the Software.

    THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF
    ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED
    TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A
    PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT
    SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
    CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
    OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR
    IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
    DEALINGS IN THE SOFTWARE.

## sha2 0.11.1

License: MIT OR Apache-2.0. Source: https://github.com/RustCrypto/hashes
Authors: RustCrypto Developers.

`LICENSE-APACHE` (sha2 0.11.1): the same text as `LICENSE-APACHE` (base16ct 0.2.0) above.

`LICENSE-MIT` (sha2 0.11.1):

    Copyright (c) 2016-2026 The RustCrypto Project Developers
    Copyright (c) 2016 Artyom Pavlov
    Copyright (c) 2009-2013 Mozilla Foundation
    Copyright (c) 2006-2009 Graydon Hoare

    Permission is hereby granted, free of charge, to any
    person obtaining a copy of this software and associated
    documentation files (the "Software"), to deal in the
    Software without restriction, including without
    limitation the rights to use, copy, modify, merge,
    publish, distribute, sublicense, and/or sell copies of
    the Software, and to permit persons to whom the Software
    is furnished to do so, subject to the following
    conditions:

    The above copyright notice and this permission notice
    shall be included in all copies or substantial portions
    of the Software.

    THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF
    ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED
    TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A
    PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT
    SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
    CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
    OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR
    IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
    DEALINGS IN THE SOFTWARE.

## sha2 0.9.9

License: MIT OR Apache-2.0. Source: https://github.com/RustCrypto/hashes
Authors: RustCrypto Developers.

`LICENSE-APACHE` (sha2 0.9.9): the same text as `LICENSE-APACHE` (base16ct 0.2.0) above.

`LICENSE-MIT` (sha2 0.9.9): the same text as `LICENSE-MIT` (sha2 0.10.9) above.

## sha3 0.10.9

License: MIT OR Apache-2.0. Source: https://github.com/RustCrypto/hashes
Authors: RustCrypto Developers.

`LICENSE-APACHE` (sha3 0.10.9): the same text as `LICENSE-APACHE` (base16ct 0.2.0) above.

`LICENSE-MIT` (sha3 0.10.9):

    Copyright (c) 2006-2009 Graydon Hoare
    Copyright (c) 2009-2013 Mozilla Foundation
    Copyright (c) 2014 Sébastien Martini
    Copyright (c) 2016-2023 Artyom Pavlov, Marek Kotewicz

    Permission is hereby granted, free of charge, to any
    person obtaining a copy of this software and associated
    documentation files (the "Software"), to deal in the
    Software without restriction, including without
    limitation the rights to use, copy, modify, merge,
    publish, distribute, sublicense, and/or sell copies of
    the Software, and to permit persons to whom the Software
    is furnished to do so, subject to the following
    conditions:

    The above copyright notice and this permission notice
    shall be included in all copies or substantial portions
    of the Software.

    THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF
    ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED
    TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A
    PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT
    SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
    CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
    OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR
    IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
    DEALINGS IN THE SOFTWARE.

## sha3 0.11.0

License: MIT OR Apache-2.0. Source: https://github.com/RustCrypto/hashes
Authors: RustCrypto Developers.

`LICENSE-APACHE` (sha3 0.11.0): the same text as `LICENSE-APACHE` (base16ct 0.2.0) above.

`LICENSE-MIT` (sha3 0.11.0):

    Copyright (c) 2020-2026 The RustCrypto Project Developers
    Copyright (c) 2016-2023 Artyom Pavlov, Marek Kotewicz
    Copyright (c) 2014 Sébastien Martini
    Copyright (c) 2009-2013 Mozilla Foundation
    Copyright (c) 2006-2009 Graydon Hoare

    Permission is hereby granted, free of charge, to any
    person obtaining a copy of this software and associated
    documentation files (the "Software"), to deal in the
    Software without restriction, including without
    limitation the rights to use, copy, modify, merge,
    publish, distribute, sublicense, and/or sell copies of
    the Software, and to permit persons to whom the Software
    is furnished to do so, subject to the following
    conditions:

    The above copyright notice and this permission notice
    shall be included in all copies or substantial portions
    of the Software.

    THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF
    ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED
    TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A
    PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT
    SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
    CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
    OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR
    IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
    DEALINGS IN THE SOFTWARE.

## sharded-slab 0.1.7

License: MIT. Source: https://github.com/hawkw/sharded-slab
Authors: Eliza Weisman <eliza@buoyant.io>.

`LICENSE` (sharded-slab 0.1.7):

    Copyright (c) 2019 Eliza Weisman

    Permission is hereby granted, free of charge, to any person obtaining a copy
    of this software and associated documentation files (the "Software"), to deal
    in the Software without restriction, including without limitation the rights
    to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
    copies of the Software, and to permit persons to whom the Software is
    furnished to do so, subject to the following conditions:

    The above copyright notice and this permission notice shall be included in
    all copies or substantial portions of the Software.

    THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
    IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
    FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
    AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
    LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
    OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN
    THE SOFTWARE.

## signature 2.2.0

License: Apache-2.0 OR MIT. Source: https://github.com/RustCrypto/traits/tree/master/signature
Authors: RustCrypto Developers.

`LICENSE-APACHE` (signature 2.2.0): the same text as `LICENSE-APACHE` (base16ct 0.2.0) above.

`LICENSE-MIT` (signature 2.2.0): the same text as `LICENSE-MIT` (ed25519 2.2.3) above.

## sketches-ddsketch 0.2.2

License: Apache-2.0. Source: https://github.com/mheffner/rust-sketches-ddsketch
Authors: Mike Heffner <mikeh@fesnel.com>.

`LICENSE` (sketches-ddsketch 0.2.2):

                                     Apache License
                               Version 2.0, January 2004
                            http://www.apache.org/licenses/

       TERMS AND CONDITIONS FOR USE, REPRODUCTION, AND DISTRIBUTION

       1. Definitions.

          "License" shall mean the terms and conditions for use, reproduction,
          and distribution as defined by Sections 1 through 9 of this document.

          "Licensor" shall mean the copyright owner or entity authorized by
          the copyright owner that is granting the License.

          "Legal Entity" shall mean the union of the acting entity and all
          other entities that control, are controlled by, or are under common
          control with that entity. For the purposes of this definition,
          "control" means (i) the power, direct or indirect, to cause the
          direction or management of such entity, whether by contract or
          otherwise, or (ii) ownership of fifty percent (50%) or more of the
          outstanding shares, or (iii) beneficial ownership of such entity.

          "You" (or "Your") shall mean an individual or Legal Entity
          exercising permissions granted by this License.

          "Source" form shall mean the preferred form for making modifications,
          including but not limited to software source code, documentation
          source, and configuration files.

          "Object" form shall mean any form resulting from mechanical
          transformation or translation of a Source form, including but
          not limited to compiled object code, generated documentation,
          and conversions to other media types.

          "Work" shall mean the work of authorship, whether in Source or
          Object form, made available under the License, as indicated by a
          copyright notice that is included in or attached to the work
          (an example is provided in the Appendix below).

          "Derivative Works" shall mean any work, whether in Source or Object
          form, that is based on (or derived from) the Work and for which the
          editorial revisions, annotations, elaborations, or other modifications
          represent, as a whole, an original work of authorship. For the purposes
          of this License, Derivative Works shall not include works that remain
          separable from, or merely link (or bind by name) to the interfaces of,
          the Work and Derivative Works thereof.

          "Contribution" shall mean any work of authorship, including
          the original version of the Work and any modifications or additions
          to that Work or Derivative Works thereof, that is intentionally
          submitted to Licensor for inclusion in the Work by the copyright owner
          or by an individual or Legal Entity authorized to submit on behalf of
          the copyright owner. For the purposes of this definition, "submitted"
          means any form of electronic, verbal, or written communication sent
          to the Licensor or its representatives, including but not limited to
          communication on electronic mailing lists, source code control systems,
          and issue tracking systems that are managed by, or on behalf of, the
          Licensor for the purpose of discussing and improving the Work, but
          excluding communication that is conspicuously marked or otherwise
          designated in writing by the copyright owner as "Not a Contribution."

          "Contributor" shall mean Licensor and any individual or Legal Entity
          on behalf of whom a Contribution has been received by Licensor and
          subsequently incorporated within the Work.

       2. Grant of Copyright License. Subject to the terms and conditions of
          this License, each Contributor hereby grants to You a perpetual,
          worldwide, non-exclusive, no-charge, royalty-free, irrevocable
          copyright license to reproduce, prepare Derivative Works of,
          publicly display, publicly perform, sublicense, and distribute the
          Work and such Derivative Works in Source or Object form.

       3. Grant of Patent License. Subject to the terms and conditions of
          this License, each Contributor hereby grants to You a perpetual,
          worldwide, non-exclusive, no-charge, royalty-free, irrevocable
          (except as stated in this section) patent license to make, have made,
          use, offer to sell, sell, import, and otherwise transfer the Work,
          where such license applies only to those patent claims licensable
          by such Contributor that are necessarily infringed by their
          Contribution(s) alone or by combination of their Contribution(s)
          with the Work to which such Contribution(s) was submitted. If You
          institute patent litigation against any entity (including a
          cross-claim or counterclaim in a lawsuit) alleging that the Work
          or a Contribution incorporated within the Work constitutes direct
          or contributory patent infringement, then any patent licenses
          granted to You under this License for that Work shall terminate
          as of the date such litigation is filed.

       4. Redistribution. You may reproduce and distribute copies of the
          Work or Derivative Works thereof in any medium, with or without
          modifications, and in Source or Object form, provided that You
          meet the following conditions:

          (a) You must give any other recipients of the Work or
              Derivative Works a copy of this License; and

          (b) You must cause any modified files to carry prominent notices
              stating that You changed the files; and

          (c) You must retain, in the Source form of any Derivative Works
              that You distribute, all copyright, patent, trademark, and
              attribution notices from the Source form of the Work,
              excluding those notices that do not pertain to any part of
              the Derivative Works; and

          (d) If the Work includes a "NOTICE" text file as part of its
              distribution, then any Derivative Works that You distribute must
              include a readable copy of the attribution notices contained
              within such NOTICE file, excluding those notices that do not
              pertain to any part of the Derivative Works, in at least one
              of the following places: within a NOTICE text file distributed
              as part of the Derivative Works; within the Source form or
              documentation, if provided along with the Derivative Works; or,
              within a display generated by the Derivative Works, if and
              wherever such third-party notices normally appear. The contents
              of the NOTICE file are for informational purposes only and
              do not modify the License. You may add Your own attribution
              notices within Derivative Works that You distribute, alongside
              or as an addendum to the NOTICE text from the Work, provided
              that such additional attribution notices cannot be construed
              as modifying the License.

          You may add Your own copyright statement to Your modifications and
          may provide additional or different license terms and conditions
          for use, reproduction, or distribution of Your modifications, or
          for any such Derivative Works as a whole, provided Your use,
          reproduction, and distribution of the Work otherwise complies with
          the conditions stated in this License.

       5. Submission of Contributions. Unless You explicitly state otherwise,
          any Contribution intentionally submitted for inclusion in the Work
          by You to the Licensor shall be under the terms and conditions of
          this License, without any additional terms or conditions.
          Notwithstanding the above, nothing herein shall supersede or modify
          the terms of any separate license agreement you may have executed
          with Licensor regarding such Contributions.

       6. Trademarks. This License does not grant permission to use the trade
          names, trademarks, service marks, or product names of the Licensor,
          except as required for reasonable and customary use in describing the
          origin of the Work and reproducing the content of the NOTICE file.

       7. Disclaimer of Warranty. Unless required by applicable law or
          agreed to in writing, Licensor provides the Work (and each
          Contributor provides its Contributions) on an "AS IS" BASIS,
          WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or
          implied, including, without limitation, any warranties or conditions
          of TITLE, NON-INFRINGEMENT, MERCHANTABILITY, or FITNESS FOR A
          PARTICULAR PURPOSE. You are solely responsible for determining the
          appropriateness of using or redistributing the Work and assume any
          risks associated with Your exercise of permissions under this License.

       8. Limitation of Liability. In no event and under no legal theory,
          whether in tort (including negligence), contract, or otherwise,
          unless required by applicable law (such as deliberate and grossly
          negligent acts) or agreed to in writing, shall any Contributor be
          liable to You for damages, including any direct, indirect, special,
          incidental, or consequential damages of any character arising as a
          result of this License or out of the use or inability to use the
          Work (including but not limited to damages for loss of goodwill,
          work stoppage, computer failure or malfunction, or any and all
          other commercial damages or losses), even if such Contributor
          has been advised of the possibility of such damages.

       9. Accepting Warranty or Additional Liability. While redistributing
          the Work or Derivative Works thereof, You may choose to offer,
          and charge a fee for, acceptance of support, warranty, indemnity,
          or other liability obligations and/or rights consistent with this
          License. However, in accepting such obligations, You may act only
          on Your own behalf and on Your sole responsibility, not on behalf
          of any other Contributor, and only if You agree to indemnify,
          defend, and hold each Contributor harmless for any liability
          incurred by, or claims asserted against, such Contributor by reason
          of your accepting any such warranty or additional liability.

       END OF TERMS AND CONDITIONS

       APPENDIX: How to apply the Apache License to your work.

          To apply the Apache License to your work, attach the following
          boilerplate notice, with the fields enclosed by brackets "[]"
          replaced with your own identifying information. (Don't include
          the brackets!)  The text should be enclosed in the appropriate
          comment syntax for the file format. We also recommend that a
          file or class name and description of purpose be included on the
          same "printed page" as the copyright notice for easier
          identification within third-party archives.

       Copyright [2019] [Mike Heffner]

       Licensed under the Apache License, Version 2.0 (the "License");
       you may not use this file except in compliance with the License.
       You may obtain a copy of the License at

           http://www.apache.org/licenses/LICENSE-2.0

       Unless required by applicable law or agreed to in writing, software
       distributed under the License is distributed on an "AS IS" BASIS,
       WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
       See the License for the specific language governing permissions and
       limitations under the License.

## slab 0.4.12

License: MIT. Source: https://github.com/tokio-rs/slab
Authors: Carl Lerche <me@carllerche.com>.

`LICENSE` (slab 0.4.12):

    Copyright (c) 2019 Carl Lerche

    Permission is hereby granted, free of charge, to any
    person obtaining a copy of this software and associated
    documentation files (the "Software"), to deal in the
    Software without restriction, including without
    limitation the rights to use, copy, modify, merge,
    publish, distribute, sublicense, and/or sell copies of
    the Software, and to permit persons to whom the Software
    is furnished to do so, subject to the following
    conditions:

    The above copyright notice and this permission notice
    shall be included in all copies or substantial portions
    of the Software.

    THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF
    ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED
    TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A
    PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT
    SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
    CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
    OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR
    IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
    DEALINGS IN THE SOFTWARE.

## slop-air 6.8.1

License: MIT OR Apache-2.0. Source: https://github.com/succinctlabs/sp1

Its package carries no license file: the standard text of its license, under its authors.

Apache-2.0 (slop-air 6.8.1): the same text as Apache-2.0 above.

MIT (slop-air 6.8.1):

    MIT License

    Copyright (c) slop-air contributors

    Permission is hereby granted, free of charge, to any person obtaining a copy of this software and associated documentation files (the "Software"), to deal in the Software without restriction, including without limitation the rights to use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of the Software, and to permit persons to whom the Software is furnished to do so, subject to the following conditions:

    The above copyright notice and this permission notice shall be included in all copies or substantial portions of the Software.

    THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

## slop-algebra 6.8.1

License: MIT OR Apache-2.0. Source: https://github.com/succinctlabs/sp1

Its package carries no license file: the standard text of its license, under its authors.

Apache-2.0 (slop-algebra 6.8.1): the same text as Apache-2.0 above.

MIT (slop-algebra 6.8.1):

    MIT License

    Copyright (c) slop-algebra contributors

    Permission is hereby granted, free of charge, to any person obtaining a copy of this software and associated documentation files (the "Software"), to deal in the Software without restriction, including without limitation the rights to use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of the Software, and to permit persons to whom the Software is furnished to do so, subject to the following conditions:

    The above copyright notice and this permission notice shall be included in all copies or substantial portions of the Software.

    THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

## slop-alloc 6.8.1

License: MIT OR Apache-2.0. Source: https://github.com/succinctlabs/sp1

Its package carries no license file: the standard text of its license, under its authors.

Apache-2.0 (slop-alloc 6.8.1): the same text as Apache-2.0 above.

MIT (slop-alloc 6.8.1):

    MIT License

    Copyright (c) slop-alloc contributors

    Permission is hereby granted, free of charge, to any person obtaining a copy of this software and associated documentation files (the "Software"), to deal in the Software without restriction, including without limitation the rights to use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of the Software, and to permit persons to whom the Software is furnished to do so, subject to the following conditions:

    The above copyright notice and this permission notice shall be included in all copies or substantial portions of the Software.

    THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

## slop-baby-bear 6.8.1

License: MIT OR Apache-2.0. Source: https://github.com/succinctlabs/sp1

Its package carries no license file: the standard text of its license, under its authors.

Apache-2.0 (slop-baby-bear 6.8.1): the same text as Apache-2.0 above.

MIT (slop-baby-bear 6.8.1):

    MIT License

    Copyright (c) slop-baby-bear contributors

    Permission is hereby granted, free of charge, to any person obtaining a copy of this software and associated documentation files (the "Software"), to deal in the Software without restriction, including without limitation the rights to use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of the Software, and to permit persons to whom the Software is furnished to do so, subject to the following conditions:

    The above copyright notice and this permission notice shall be included in all copies or substantial portions of the Software.

    THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

## slop-basefold 6.8.1

License: MIT OR Apache-2.0. Source: https://github.com/succinctlabs/sp1

Its package carries no license file: the standard text of its license, under its authors.

Apache-2.0 (slop-basefold 6.8.1): the same text as Apache-2.0 above.

MIT (slop-basefold 6.8.1):

    MIT License

    Copyright (c) slop-basefold contributors

    Permission is hereby granted, free of charge, to any person obtaining a copy of this software and associated documentation files (the "Software"), to deal in the Software without restriction, including without limitation the rights to use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of the Software, and to permit persons to whom the Software is furnished to do so, subject to the following conditions:

    The above copyright notice and this permission notice shall be included in all copies or substantial portions of the Software.

    THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

## slop-basefold-prover 6.8.1

License: MIT OR Apache-2.0. Source: https://github.com/succinctlabs/sp1

Its package carries no license file: the standard text of its license, under its authors.

Apache-2.0 (slop-basefold-prover 6.8.1): the same text as Apache-2.0 above.

MIT (slop-basefold-prover 6.8.1):

    MIT License

    Copyright (c) slop-basefold-prover contributors

    Permission is hereby granted, free of charge, to any person obtaining a copy of this software and associated documentation files (the "Software"), to deal in the Software without restriction, including without limitation the rights to use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of the Software, and to permit persons to whom the Software is furnished to do so, subject to the following conditions:

    The above copyright notice and this permission notice shall be included in all copies or substantial portions of the Software.

    THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

## slop-bn254 6.8.1

License: MIT OR Apache-2.0. Source: https://github.com/succinctlabs/sp1

Its package carries no license file: the standard text of its license, under its authors.

Apache-2.0 (slop-bn254 6.8.1): the same text as Apache-2.0 above.

MIT (slop-bn254 6.8.1):

    MIT License

    Copyright (c) slop-bn254 contributors

    Permission is hereby granted, free of charge, to any person obtaining a copy of this software and associated documentation files (the "Software"), to deal in the Software without restriction, including without limitation the rights to use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of the Software, and to permit persons to whom the Software is furnished to do so, subject to the following conditions:

    The above copyright notice and this permission notice shall be included in all copies or substantial portions of the Software.

    THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

## slop-challenger 6.8.1

License: MIT OR Apache-2.0. Source: https://github.com/succinctlabs/sp1

Its package carries no license file: the standard text of its license, under its authors.

Apache-2.0 (slop-challenger 6.8.1): the same text as Apache-2.0 above.

MIT (slop-challenger 6.8.1):

    MIT License

    Copyright (c) slop-challenger contributors

    Permission is hereby granted, free of charge, to any person obtaining a copy of this software and associated documentation files (the "Software"), to deal in the Software without restriction, including without limitation the rights to use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of the Software, and to permit persons to whom the Software is furnished to do so, subject to the following conditions:

    The above copyright notice and this permission notice shall be included in all copies or substantial portions of the Software.

    THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

## slop-commit 6.8.1

License: MIT OR Apache-2.0. Source: https://github.com/succinctlabs/sp1

Its package carries no license file: the standard text of its license, under its authors.

Apache-2.0 (slop-commit 6.8.1): the same text as Apache-2.0 above.

MIT (slop-commit 6.8.1):

    MIT License

    Copyright (c) slop-commit contributors

    Permission is hereby granted, free of charge, to any person obtaining a copy of this software and associated documentation files (the "Software"), to deal in the Software without restriction, including without limitation the rights to use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of the Software, and to permit persons to whom the Software is furnished to do so, subject to the following conditions:

    The above copyright notice and this permission notice shall be included in all copies or substantial portions of the Software.

    THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

## slop-dft 6.8.1

License: MIT OR Apache-2.0. Source: https://github.com/succinctlabs/sp1

Its package carries no license file: the standard text of its license, under its authors.

Apache-2.0 (slop-dft 6.8.1): the same text as Apache-2.0 above.

MIT (slop-dft 6.8.1):

    MIT License

    Copyright (c) slop-dft contributors

    Permission is hereby granted, free of charge, to any person obtaining a copy of this software and associated documentation files (the "Software"), to deal in the Software without restriction, including without limitation the rights to use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of the Software, and to permit persons to whom the Software is furnished to do so, subject to the following conditions:

    The above copyright notice and this permission notice shall be included in all copies or substantial portions of the Software.

    THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

## slop-fri 6.8.1

License: MIT OR Apache-2.0. Source: https://github.com/succinctlabs/sp1

Its package carries no license file: the standard text of its license, under its authors.

Apache-2.0 (slop-fri 6.8.1): the same text as Apache-2.0 above.

MIT (slop-fri 6.8.1):

    MIT License

    Copyright (c) slop-fri contributors

    Permission is hereby granted, free of charge, to any person obtaining a copy of this software and associated documentation files (the "Software"), to deal in the Software without restriction, including without limitation the rights to use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of the Software, and to permit persons to whom the Software is furnished to do so, subject to the following conditions:

    The above copyright notice and this permission notice shall be included in all copies or substantial portions of the Software.

    THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

## slop-futures 6.8.1

License: MIT OR Apache-2.0. Source: https://github.com/succinctlabs/sp1

Its package carries no license file: the standard text of its license, under its authors.

Apache-2.0 (slop-futures 6.8.1): the same text as Apache-2.0 above.

MIT (slop-futures 6.8.1):

    MIT License

    Copyright (c) slop-futures contributors

    Permission is hereby granted, free of charge, to any person obtaining a copy of this software and associated documentation files (the "Software"), to deal in the Software without restriction, including without limitation the rights to use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of the Software, and to permit persons to whom the Software is furnished to do so, subject to the following conditions:

    The above copyright notice and this permission notice shall be included in all copies or substantial portions of the Software.

    THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

## slop-jagged 6.8.1

License: MIT OR Apache-2.0. Source: https://github.com/succinctlabs/sp1

Its package carries no license file: the standard text of its license, under its authors.

Apache-2.0 (slop-jagged 6.8.1): the same text as Apache-2.0 above.

MIT (slop-jagged 6.8.1):

    MIT License

    Copyright (c) slop-jagged contributors

    Permission is hereby granted, free of charge, to any person obtaining a copy of this software and associated documentation files (the "Software"), to deal in the Software without restriction, including without limitation the rights to use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of the Software, and to permit persons to whom the Software is furnished to do so, subject to the following conditions:

    The above copyright notice and this permission notice shall be included in all copies or substantial portions of the Software.

    THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

## slop-koala-bear 6.8.1

License: MIT OR Apache-2.0. Source: https://github.com/succinctlabs/sp1

Its package carries no license file: the standard text of its license, under its authors.

Apache-2.0 (slop-koala-bear 6.8.1): the same text as Apache-2.0 above.

MIT (slop-koala-bear 6.8.1):

    MIT License

    Copyright (c) slop-koala-bear contributors

    Permission is hereby granted, free of charge, to any person obtaining a copy of this software and associated documentation files (the "Software"), to deal in the Software without restriction, including without limitation the rights to use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of the Software, and to permit persons to whom the Software is furnished to do so, subject to the following conditions:

    The above copyright notice and this permission notice shall be included in all copies or substantial portions of the Software.

    THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

## slop-matrix 6.8.1

License: MIT OR Apache-2.0. Source: https://github.com/succinctlabs/sp1

Its package carries no license file: the standard text of its license, under its authors.

Apache-2.0 (slop-matrix 6.8.1): the same text as Apache-2.0 above.

MIT (slop-matrix 6.8.1):

    MIT License

    Copyright (c) slop-matrix contributors

    Permission is hereby granted, free of charge, to any person obtaining a copy of this software and associated documentation files (the "Software"), to deal in the Software without restriction, including without limitation the rights to use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of the Software, and to permit persons to whom the Software is furnished to do so, subject to the following conditions:

    The above copyright notice and this permission notice shall be included in all copies or substantial portions of the Software.

    THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

## slop-maybe-rayon 6.8.1

License: MIT OR Apache-2.0. Source: https://github.com/succinctlabs/sp1

Its package carries no license file: the standard text of its license, under its authors.

Apache-2.0 (slop-maybe-rayon 6.8.1): the same text as Apache-2.0 above.

MIT (slop-maybe-rayon 6.8.1):

    MIT License

    Copyright (c) slop-maybe-rayon contributors

    Permission is hereby granted, free of charge, to any person obtaining a copy of this software and associated documentation files (the "Software"), to deal in the Software without restriction, including without limitation the rights to use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of the Software, and to permit persons to whom the Software is furnished to do so, subject to the following conditions:

    The above copyright notice and this permission notice shall be included in all copies or substantial portions of the Software.

    THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

## slop-merkle-tree 6.8.1

License: MIT OR Apache-2.0. Source: https://github.com/succinctlabs/sp1

Its package carries no license file: the standard text of its license, under its authors.

Apache-2.0 (slop-merkle-tree 6.8.1): the same text as Apache-2.0 above.

MIT (slop-merkle-tree 6.8.1):

    MIT License

    Copyright (c) slop-merkle-tree contributors

    Permission is hereby granted, free of charge, to any person obtaining a copy of this software and associated documentation files (the "Software"), to deal in the Software without restriction, including without limitation the rights to use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of the Software, and to permit persons to whom the Software is furnished to do so, subject to the following conditions:

    The above copyright notice and this permission notice shall be included in all copies or substantial portions of the Software.

    THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

## slop-multilinear 6.8.1

License: MIT OR Apache-2.0. Source: https://github.com/succinctlabs/sp1

Its package carries no license file: the standard text of its license, under its authors.

Apache-2.0 (slop-multilinear 6.8.1): the same text as Apache-2.0 above.

MIT (slop-multilinear 6.8.1):

    MIT License

    Copyright (c) slop-multilinear contributors

    Permission is hereby granted, free of charge, to any person obtaining a copy of this software and associated documentation files (the "Software"), to deal in the Software without restriction, including without limitation the rights to use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of the Software, and to permit persons to whom the Software is furnished to do so, subject to the following conditions:

    The above copyright notice and this permission notice shall be included in all copies or substantial portions of the Software.

    THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

## slop-poseidon2 6.8.1

License: MIT OR Apache-2.0. Source: https://github.com/succinctlabs/sp1

Its package carries no license file: the standard text of its license, under its authors.

Apache-2.0 (slop-poseidon2 6.8.1): the same text as Apache-2.0 above.

MIT (slop-poseidon2 6.8.1):

    MIT License

    Copyright (c) slop-poseidon2 contributors

    Permission is hereby granted, free of charge, to any person obtaining a copy of this software and associated documentation files (the "Software"), to deal in the Software without restriction, including without limitation the rights to use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of the Software, and to permit persons to whom the Software is furnished to do so, subject to the following conditions:

    The above copyright notice and this permission notice shall be included in all copies or substantial portions of the Software.

    THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

## slop-primitives 6.8.1

License: MIT OR Apache-2.0. Source: https://github.com/succinctlabs/sp1

Its package carries no license file: the standard text of its license, under its authors.

Apache-2.0 (slop-primitives 6.8.1): the same text as Apache-2.0 above.

MIT (slop-primitives 6.8.1):

    MIT License

    Copyright (c) slop-primitives contributors

    Permission is hereby granted, free of charge, to any person obtaining a copy of this software and associated documentation files (the "Software"), to deal in the Software without restriction, including without limitation the rights to use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of the Software, and to permit persons to whom the Software is furnished to do so, subject to the following conditions:

    The above copyright notice and this permission notice shall be included in all copies or substantial portions of the Software.

    THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

## slop-stacked 6.8.1

License: MIT OR Apache-2.0. Source: https://github.com/succinctlabs/sp1

Its package carries no license file: the standard text of its license, under its authors.

Apache-2.0 (slop-stacked 6.8.1): the same text as Apache-2.0 above.

MIT (slop-stacked 6.8.1):

    MIT License

    Copyright (c) slop-stacked contributors

    Permission is hereby granted, free of charge, to any person obtaining a copy of this software and associated documentation files (the "Software"), to deal in the Software without restriction, including without limitation the rights to use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of the Software, and to permit persons to whom the Software is furnished to do so, subject to the following conditions:

    The above copyright notice and this permission notice shall be included in all copies or substantial portions of the Software.

    THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

## slop-sumcheck 6.8.1

License: MIT OR Apache-2.0. Source: https://github.com/succinctlabs/sp1

Its package carries no license file: the standard text of its license, under its authors.

Apache-2.0 (slop-sumcheck 6.8.1): the same text as Apache-2.0 above.

MIT (slop-sumcheck 6.8.1):

    MIT License

    Copyright (c) slop-sumcheck contributors

    Permission is hereby granted, free of charge, to any person obtaining a copy of this software and associated documentation files (the "Software"), to deal in the Software without restriction, including without limitation the rights to use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of the Software, and to permit persons to whom the Software is furnished to do so, subject to the following conditions:

    The above copyright notice and this permission notice shall be included in all copies or substantial portions of the Software.

    THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

## slop-symmetric 6.8.1

License: MIT OR Apache-2.0. Source: https://github.com/succinctlabs/sp1

Its package carries no license file: the standard text of its license, under its authors.

Apache-2.0 (slop-symmetric 6.8.1): the same text as Apache-2.0 above.

MIT (slop-symmetric 6.8.1):

    MIT License

    Copyright (c) slop-symmetric contributors

    Permission is hereby granted, free of charge, to any person obtaining a copy of this software and associated documentation files (the "Software"), to deal in the Software without restriction, including without limitation the rights to use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of the Software, and to permit persons to whom the Software is furnished to do so, subject to the following conditions:

    The above copyright notice and this permission notice shall be included in all copies or substantial portions of the Software.

    THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

## slop-tensor 6.8.1

License: MIT OR Apache-2.0. Source: https://github.com/succinctlabs/sp1

Its package carries no license file: the standard text of its license, under its authors.

Apache-2.0 (slop-tensor 6.8.1): the same text as Apache-2.0 above.

MIT (slop-tensor 6.8.1):

    MIT License

    Copyright (c) slop-tensor contributors

    Permission is hereby granted, free of charge, to any person obtaining a copy of this software and associated documentation files (the "Software"), to deal in the Software without restriction, including without limitation the rights to use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of the Software, and to permit persons to whom the Software is furnished to do so, subject to the following conditions:

    The above copyright notice and this permission notice shall be included in all copies or substantial portions of the Software.

    THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

## slop-uni-stark 6.8.1

License: MIT OR Apache-2.0. Source: https://github.com/succinctlabs/sp1

Its package carries no license file: the standard text of its license, under its authors.

Apache-2.0 (slop-uni-stark 6.8.1): the same text as Apache-2.0 above.

MIT (slop-uni-stark 6.8.1):

    MIT License

    Copyright (c) slop-uni-stark contributors

    Permission is hereby granted, free of charge, to any person obtaining a copy of this software and associated documentation files (the "Software"), to deal in the Software without restriction, including without limitation the rights to use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of the Software, and to permit persons to whom the Software is furnished to do so, subject to the following conditions:

    The above copyright notice and this permission notice shall be included in all copies or substantial portions of the Software.

    THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

## slop-utils 6.8.1

License: MIT OR Apache-2.0. Source: https://github.com/succinctlabs/sp1

Its package carries no license file: the standard text of its license, under its authors.

Apache-2.0 (slop-utils 6.8.1): the same text as Apache-2.0 above.

MIT (slop-utils 6.8.1):

    MIT License

    Copyright (c) slop-utils contributors

    Permission is hereby granted, free of charge, to any person obtaining a copy of this software and associated documentation files (the "Software"), to deal in the Software without restriction, including without limitation the rights to use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of the Software, and to permit persons to whom the Software is furnished to do so, subject to the following conditions:

    The above copyright notice and this permission notice shall be included in all copies or substantial portions of the Software.

    THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

## slop-whir 6.8.1

License: MIT OR Apache-2.0. Source: https://github.com/succinctlabs/sp1

Its package carries no license file: the standard text of its license, under its authors.

Apache-2.0 (slop-whir 6.8.1): the same text as Apache-2.0 above.

MIT (slop-whir 6.8.1):

    MIT License

    Copyright (c) slop-whir contributors

    Permission is hereby granted, free of charge, to any person obtaining a copy of this software and associated documentation files (the "Software"), to deal in the Software without restriction, including without limitation the rights to use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of the Software, and to permit persons to whom the Software is furnished to do so, subject to the following conditions:

    The above copyright notice and this permission notice shall be included in all copies or substantial portions of the Software.

    THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

## smallvec 1.15.2

License: MIT OR Apache-2.0. Source: https://github.com/servo/rust-smallvec
Authors: The Servo Project Developers.

`LICENSE-APACHE` (smallvec 1.15.2): the same text as `LICENSE-APACHE` (addr2line 0.25.1) above.

`LICENSE-MIT` (smallvec 1.15.2):

    Copyright (c) 2018 The Servo Project Developers

    Permission is hereby granted, free of charge, to any
    person obtaining a copy of this software and associated
    documentation files (the "Software"), to deal in the
    Software without restriction, including without
    limitation the rights to use, copy, modify, merge,
    publish, distribute, sublicense, and/or sell copies of
    the Software, and to permit persons to whom the Software
    is furnished to do so, subject to the following
    conditions:

    The above copyright notice and this permission notice
    shall be included in all copies or substantial portions
    of the Software.

    THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF
    ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED
    TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A
    PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT
    SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
    CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
    OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR
    IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
    DEALINGS IN THE SOFTWARE.

## smallvec 1.16.3

License: MIT OR Apache-2.0. Source: https://github.com/servo/rust-smallvec
Authors: The Servo Project Developers.

`LICENSE-APACHE` (smallvec 1.16.3): the same text as `LICENSE-APACHE` (addr2line 0.25.1) above.

`LICENSE-MIT` (smallvec 1.16.3): the same text as `LICENSE-MIT` (smallvec 1.15.2) above.

## smallvec 1.16.4

License: MIT OR Apache-2.0. Source: https://github.com/servo/rust-smallvec
Authors: The Servo Project Developers.

`LICENSE-APACHE` (smallvec 1.16.4): the same text as `LICENSE-APACHE` (addr2line 0.25.1) above.

`LICENSE-MIT` (smallvec 1.16.4): the same text as `LICENSE-MIT` (smallvec 1.15.2) above.

## smallvec 1.16.5

License: MIT OR Apache-2.0. Source: https://github.com/servo/rust-smallvec
Authors: The Servo Project Developers.

`LICENSE-APACHE` (smallvec 1.16.5): the same text as `LICENSE-APACHE` (addr2line 0.25.1) above.

`LICENSE-MIT` (smallvec 1.16.5): the same text as `LICENSE-MIT` (smallvec 1.15.2) above.

## sp1-hypercube 6.8.1

License: MIT OR Apache-2.0. Source: https://github.com/succinctlabs/sp1

Its package carries no license file: the standard text of its license, under its authors.

Apache-2.0 (sp1-hypercube 6.8.1): the same text as Apache-2.0 above.

MIT (sp1-hypercube 6.8.1):

    MIT License

    Copyright (c) sp1-hypercube contributors

    Permission is hereby granted, free of charge, to any person obtaining a copy of this software and associated documentation files (the "Software"), to deal in the Software without restriction, including without limitation the rights to use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of the Software, and to permit persons to whom the Software is furnished to do so, subject to the following conditions:

    The above copyright notice and this permission notice shall be included in all copies or substantial portions of the Software.

    THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

## sp1-primitives 6.8.1

License: MIT OR Apache-2.0. Source: https://github.com/succinctlabs/sp1

Its package carries no license file: the standard text of its license, under its authors.

Apache-2.0 (sp1-primitives 6.8.1): the same text as Apache-2.0 above.

MIT (sp1-primitives 6.8.1):

    MIT License

    Copyright (c) sp1-primitives contributors

    Permission is hereby granted, free of charge, to any person obtaining a copy of this software and associated documentation files (the "Software"), to deal in the Software without restriction, including without limitation the rights to use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of the Software, and to permit persons to whom the Software is furnished to do so, subject to the following conditions:

    The above copyright notice and this permission notice shall be included in all copies or substantial portions of the Software.

    THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

## sp1-recursion-executor 6.8.1

License: MIT OR Apache-2.0. Source: https://github.com/succinctlabs/sp1

Its package carries no license file: the standard text of its license, under its authors.

Apache-2.0 (sp1-recursion-executor 6.8.1): the same text as Apache-2.0 above.

MIT (sp1-recursion-executor 6.8.1):

    MIT License

    Copyright (c) sp1-recursion-executor contributors

    Permission is hereby granted, free of charge, to any person obtaining a copy of this software and associated documentation files (the "Software"), to deal in the Software without restriction, including without limitation the rights to use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of the Software, and to permit persons to whom the Software is furnished to do so, subject to the following conditions:

    The above copyright notice and this permission notice shall be included in all copies or substantial portions of the Software.

    THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

## sp1-recursion-machine 6.8.1

License: MIT OR Apache-2.0. Source: https://github.com/succinctlabs/sp1

Its package carries no license file: the standard text of its license, under its authors.

Apache-2.0 (sp1-recursion-machine 6.8.1): the same text as Apache-2.0 above.

MIT (sp1-recursion-machine 6.8.1):

    MIT License

    Copyright (c) sp1-recursion-machine contributors

    Permission is hereby granted, free of charge, to any person obtaining a copy of this software and associated documentation files (the "Software"), to deal in the Software without restriction, including without limitation the rights to use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of the Software, and to permit persons to whom the Software is furnished to do so, subject to the following conditions:

    The above copyright notice and this permission notice shall be included in all copies or substantial portions of the Software.

    THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

## sp1-verifier 6.8.1

License: MIT OR Apache-2.0. Source: https://github.com/succinctlabs/sp1

Its package carries no license file: the standard text of its license, under its authors.

Apache-2.0 (sp1-verifier 6.8.1): the same text as Apache-2.0 above.

MIT (sp1-verifier 6.8.1):

    MIT License

    Copyright (c) sp1-verifier contributors

    Permission is hereby granted, free of charge, to any person obtaining a copy of this software and associated documentation files (the "Software"), to deal in the Software without restriction, including without limitation the rights to use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of the Software, and to permit persons to whom the Software is furnished to do so, subject to the following conditions:

    The above copyright notice and this permission notice shall be included in all copies or substantial portions of the Software.

    THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

## spin 0.10.1

License: MIT. Source: https://github.com/mvdnes/spin-rs.git
Authors: Mathijs van de Nes <git@mathijs.vd-nes.nl>, John Ericson <git@JohnEricson.me>, Joshua Barretto <joshua.s.barretto@gmail.com>.

`LICENSE` (spin 0.10.1):

    The MIT License (MIT)

    Copyright (c) 2014 Mathijs van de Nes

    Permission is hereby granted, free of charge, to any person obtaining a copy
    of this software and associated documentation files (the "Software"), to deal
    in the Software without restriction, including without limitation the rights
    to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
    copies of the Software, and to permit persons to whom the Software is
    furnished to do so, subject to the following conditions:

    The above copyright notice and this permission notice shall be included in all
    copies or substantial portions of the Software.

    THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
    IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
    FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
    AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
    LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
    OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
    SOFTWARE.

## spin 0.9.9

License: MIT. Source: https://github.com/mvdnes/spin-rs.git
Authors: Mathijs van de Nes <git@mathijs.vd-nes.nl>, John Ericson <git@JohnEricson.me>, Joshua Barretto <joshua.s.barretto@gmail.com>.

`LICENSE` (spin 0.9.9): the same text as `LICENSE` (spin 0.10.1) above.

## spki 0.7.3

License: Apache-2.0 OR MIT. Source: https://github.com/RustCrypto/formats/tree/master/spki
Authors: RustCrypto Developers.

`LICENSE-APACHE` (spki 0.7.3): the same text as `LICENSE-APACHE` (base16ct 0.2.0) above.

`LICENSE-MIT` (spki 0.7.3):

    Copyright (c) 2021-2023 The RustCrypto Project Developers

    Permission is hereby granted, free of charge, to any
    person obtaining a copy of this software and associated
    documentation files (the "Software"), to deal in the
    Software without restriction, including without
    limitation the rights to use, copy, modify, merge,
    publish, distribute, sublicense, and/or sell copies of
    the Software, and to permit persons to whom the Software
    is furnished to do so, subject to the following
    conditions:

    The above copyright notice and this permission notice
    shall be included in all copies or substantial portions
    of the Software.

    THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF
    ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED
    TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A
    PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT
    SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
    CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
    OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR
    IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
    DEALINGS IN THE SOFTWARE.

## stable_deref_trait 1.2.1

License: MIT OR Apache-2.0. Source: https://github.com/storyyeller/stable_deref_trait
Authors: Robert Grosse <n210241048576@gmail.com>.

`LICENSE-APACHE` (stable_deref_trait 1.2.1): the same text as `LICENSE-APACHE` (addr2line 0.25.1) above.

`LICENSE-MIT` (stable_deref_trait 1.2.1):

    Copyright (c) 2017 Robert Grosse

    Permission is hereby granted, free of charge, to any
    person obtaining a copy of this software and associated
    documentation files (the "Software"), to deal in the
    Software without restriction, including without
    limitation the rights to use, copy, modify, merge,
    publish, distribute, sublicense, and/or sell copies of
    the Software, and to permit persons to whom the Software
    is furnished to do so, subject to the following
    conditions:

    The above copyright notice and this permission notice
    shall be included in all copies or substantial portions
    of the Software.

    THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF
    ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED
    TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A
    PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT
    SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
    CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
    OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR
    IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
    DEALINGS IN THE SOFTWARE.

## static_assertions 1.1.0

License: MIT OR Apache-2.0. Source: https://github.com/nvzqz/static-assertions-rs
Authors: Nikolai Vazquez.

`LICENSE-APACHE` (static_assertions 1.1.0): the same text as `LICENSE-APACHE` (serdect 0.2.0) above.

`LICENSE-MIT` (static_assertions 1.1.0):

    MIT License

    Copyright (c) 2017 Nikolai Vazquez

    Permission is hereby granted, free of charge, to any person obtaining a copy
    of this software and associated documentation files (the "Software"), to deal
    in the Software without restriction, including without limitation the rights
    to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
    copies of the Software, and to permit persons to whom the Software is
    furnished to do so, subject to the following conditions:

    The above copyright notice and this permission notice shall be included in all
    copies or substantial portions of the Software.

    THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
    IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
    FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
    AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
    LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
    OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
    SOFTWARE.

## stellar-strkey 1.0.0

License: Apache-2.0. Source: https://github.com/stellar/rs-stellar-strkey
Authors: Stellar Development Foundation <info@stellar.org>.

`LICENSE` (stellar-strkey 1.0.0):

                                     Apache License
                               Version 2.0, January 2004
                            http://www.apache.org/licenses/

       TERMS AND CONDITIONS FOR USE, REPRODUCTION, AND DISTRIBUTION

       1. Definitions.

          "License" shall mean the terms and conditions for use, reproduction,
          and distribution as defined by Sections 1 through 9 of this document.

          "Licensor" shall mean the copyright owner or entity authorized by
          the copyright owner that is granting the License.

          "Legal Entity" shall mean the union of the acting entity and all
          other entities that control, are controlled by, or are under common
          control with that entity. For the purposes of this definition,
          "control" means (i) the power, direct or indirect, to cause the
          direction or management of such entity, whether by contract or
          otherwise, or (ii) ownership of fifty percent (50%) or more of the
          outstanding shares, or (iii) beneficial ownership of such entity.

          "You" (or "Your") shall mean an individual or Legal Entity
          exercising permissions granted by this License.

          "Source" form shall mean the preferred form for making modifications,
          including but not limited to software source code, documentation
          source, and configuration files.

          "Object" form shall mean any form resulting from mechanical
          transformation or translation of a Source form, including but
          not limited to compiled object code, generated documentation,
          and conversions to other media types.

          "Work" shall mean the work of authorship, whether in Source or
          Object form, made available under the License, as indicated by a
          copyright notice that is included in or attached to the work
          (an example is provided in the Appendix below).

          "Derivative Works" shall mean any work, whether in Source or Object
          form, that is based on (or derived from) the Work and for which the
          editorial revisions, annotations, elaborations, or other modifications
          represent, as a whole, an original work of authorship. For the purposes
          of this License, Derivative Works shall not include works that remain
          separable from, or merely link (or bind by name) to the interfaces of,
          the Work and Derivative Works thereof.

          "Contribution" shall mean any work of authorship, including
          the original version of the Work and any modifications or additions
          to that Work or Derivative Works thereof, that is intentionally
          submitted to Licensor for inclusion in the Work by the copyright owner
          or by an individual or Legal Entity authorized to submit on behalf of
          the copyright owner. For the purposes of this definition, "submitted"
          means any form of electronic, verbal, or written communication sent
          to the Licensor or its representatives, including but not limited to
          communication on electronic mailing lists, source code control systems,
          and issue tracking systems that are managed by, or on behalf of, the
          Licensor for the purpose of discussing and improving the Work, but
          excluding communication that is conspicuously marked or otherwise
          designated in writing by the copyright owner as "Not a Contribution."

          "Contributor" shall mean Licensor and any individual or Legal Entity
          on behalf of whom a Contribution has been received by Licensor and
          subsequently incorporated within the Work.

       2. Grant of Copyright License. Subject to the terms and conditions of
          this License, each Contributor hereby grants to You a perpetual,
          worldwide, non-exclusive, no-charge, royalty-free, irrevocable
          copyright license to reproduce, prepare Derivative Works of,
          publicly display, publicly perform, sublicense, and distribute the
          Work and such Derivative Works in Source or Object form.

       3. Grant of Patent License. Subject to the terms and conditions of
          this License, each Contributor hereby grants to You a perpetual,
          worldwide, non-exclusive, no-charge, royalty-free, irrevocable
          (except as stated in this section) patent license to make, have made,
          use, offer to sell, sell, import, and otherwise transfer the Work,
          where such license applies only to those patent claims licensable
          by such Contributor that are necessarily infringed by their
          Contribution(s) alone or by combination of their Contribution(s)
          with the Work to which such Contribution(s) was submitted. If You
          institute patent litigation against any entity (including a
          cross-claim or counterclaim in a lawsuit) alleging that the Work
          or a Contribution incorporated within the Work constitutes direct
          or contributory patent infringement, then any patent licenses
          granted to You under this License for that Work shall terminate
          as of the date such litigation is filed.

       4. Redistribution. You may reproduce and distribute copies of the
          Work or Derivative Works thereof in any medium, with or without
          modifications, and in Source or Object form, provided that You
          meet the following conditions:

          (a) You must give any other recipients of the Work or
              Derivative Works a copy of this License; and

          (b) You must cause any modified files to carry prominent notices
              stating that You changed the files; and

          (c) You must retain, in the Source form of any Derivative Works
              that You distribute, all copyright, patent, trademark, and
              attribution notices from the Source form of the Work,
              excluding those notices that do not pertain to any part of
              the Derivative Works; and

          (d) If the Work includes a "NOTICE" text file as part of its
              distribution, then any Derivative Works that You distribute must
              include a readable copy of the attribution notices contained
              within such NOTICE file, excluding those notices that do not
              pertain to any part of the Derivative Works, in at least one
              of the following places: within a NOTICE text file distributed
              as part of the Derivative Works; within the Source form or
              documentation, if provided along with the Derivative Works; or,
              within a display generated by the Derivative Works, if and
              wherever such third-party notices normally appear. The contents
              of the NOTICE file are for informational purposes only and
              do not modify the License. You may add Your own attribution
              notices within Derivative Works that You distribute, alongside
              or as an addendum to the NOTICE text from the Work, provided
              that such additional attribution notices cannot be construed
              as modifying the License.

          You may add Your own copyright statement to Your modifications and
          may provide additional or different license terms and conditions
          for use, reproduction, or distribution of Your modifications, or
          for any such Derivative Works as a whole, provided Your use,
          reproduction, and distribution of the Work otherwise complies with
          the conditions stated in this License.

       5. Submission of Contributions. Unless You explicitly state otherwise,
          any Contribution intentionally submitted for inclusion in the Work
          by You to the Licensor shall be under the terms and conditions of
          this License, without any additional terms or conditions.
          Notwithstanding the above, nothing herein shall supersede or modify
          the terms of any separate license agreement you may have executed
          with Licensor regarding such Contributions.

       6. Trademarks. This License does not grant permission to use the trade
          names, trademarks, service marks, or product names of the Licensor,
          except as required for reasonable and customary use in describing the
          origin of the Work and reproducing the content of the NOTICE file.

       7. Disclaimer of Warranty. Unless required by applicable law or
          agreed to in writing, Licensor provides the Work (and each
          Contributor provides its Contributions) on an "AS IS" BASIS,
          WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or
          implied, including, without limitation, any warranties or conditions
          of TITLE, NON-INFRINGEMENT, MERCHANTABILITY, or FITNESS FOR A
          PARTICULAR PURPOSE. You are solely responsible for determining the
          appropriateness of using or redistributing the Work and assume any
          risks associated with Your exercise of permissions under this License.

       8. Limitation of Liability. In no event and under no legal theory,
          whether in tort (including negligence), contract, or otherwise,
          unless required by applicable law (such as deliberate and grossly
          negligent acts) or agreed to in writing, shall any Contributor be
          liable to You for damages, including any direct, indirect, special,
          incidental, or consequential damages of any character arising as a
          result of this License or out of the use or inability to use the
          Work (including but not limited to damages for loss of goodwill,
          work stoppage, computer failure or malfunction, or any and all
          other commercial damages or losses), even if such Contributor
          has been advised of the possibility of such damages.

       9. Accepting Warranty or Additional Liability. While redistributing
          the Work or Derivative Works thereof, You may choose to offer,
          and charge a fee for, acceptance of support, warranty, indemnity,
          or other liability obligations and/or rights consistent with this
          License. However, in accepting such obligations, You may act only
          on Your own behalf and on Your sole responsibility, not on behalf
          of any other Contributor, and only if You agree to indemnify,
          defend, and hold each Contributor harmless for any liability
          incurred by, or claims asserted against, such Contributor by reason
          of your accepting any such warranty or additional liability.

       END OF TERMS AND CONDITIONS

       APPENDIX: How to apply the Apache License to your work.

          To apply the Apache License to your work, attach the following
          boilerplate notice, with the fields enclosed by brackets "[]"
          replaced with your own identifying information. (Don't include
          the brackets!)  The text should be enclosed in the appropriate
          comment syntax for the file format. We also recommend that a
          file or class name and description of purpose be included on the
          same "printed page" as the copyright notice for easier
          identification within third-party archives.

       Copyright 2022 Stellar Development Foundation

       Licensed under the Apache License, Version 2.0 (the "License");
       you may not use this file except in compliance with the License.
       You may obtain a copy of the License at

           http://www.apache.org/licenses/LICENSE-2.0

       Unless required by applicable law or agreed to in writing, software
       distributed under the License is distributed on an "AS IS" BASIS,
       WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
       See the License for the specific language governing permissions and
       limitations under the License.

## stellar-xdr 30.0.0

License: Apache-2.0. Source: https://github.com/stellar/rs-stellar-xdr
Authors: Stellar Development Foundation <info@stellar.org>.

`LICENSE` (stellar-xdr 30.0.0): the same text as `LICENSE` (stellar-strkey 1.0.0) above.

## strength_reduce 0.2.4

License: MIT OR Apache-2.0. Source: http://github.com/ejmahler/strength_reduce
Authors: Elliott Mahler <join.together@gmail.com>.

`LICENSE-APACHE` (strength_reduce 0.2.4): the same text as `LICENSE-APACHE` (anstyle 1.0.14) above.

`LICENSE-MIT` (strength_reduce 0.2.4):

    Copyright (c) 2015 The RustFFT Developers

    Permission is hereby granted, free of charge, to any person obtaining a copy
    of this software and associated documentation files (the "Software"), to deal
    in the Software without restriction, including without limitation the rights
    to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
    copies of the Software, and to permit persons to whom the Software is
    furnished to do so, subject to the following conditions:

    The above copyright notice and this permission notice shall be included in all
    copies or substantial portions of the Software.

    THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
    IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
    FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
    AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
    LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
    OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
    SOFTWARE.

## struct-reflection 0.1.0

License: MIT. Source: https://github.com/tu-user/struct-reflection-rs
Authors: Gaston Zanitti <gzanitti@gmail.com>.

Its package carries no license file: the standard text of its license, under its authors.

MIT (struct-reflection 0.1.0):

    MIT License

    Copyright (c) Gaston Zanitti <gzanitti@gmail.com>

    Permission is hereby granted, free of charge, to any person obtaining a copy of this software and associated documentation files (the "Software"), to deal in the Software without restriction, including without limitation the rights to use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of the Software, and to permit persons to whom the Software is furnished to do so, subject to the following conditions:

    The above copyright notice and this permission notice shall be included in all copies or substantial portions of the Software.

    THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

## strum 0.26.3

License: MIT. Source: https://github.com/Peternator7/strum
Authors: Peter Glotfelty <peter.glotfelty@microsoft.com>.

`LICENSE` (strum 0.26.3):

    MIT License

    Copyright (c) 2019 Peter Glotfelty

    Permission is hereby granted, free of charge, to any person obtaining a copy
    of this software and associated documentation files (the "Software"), to deal
    in the Software without restriction, including without limitation the rights
    to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
    copies of the Software, and to permit persons to whom the Software is
    furnished to do so, subject to the following conditions:

    The above copyright notice and this permission notice shall be included in all
    copies or substantial portions of the Software.

    THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
    IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
    FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
    AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
    LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
    OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
    SOFTWARE.

## strum 0.27.2

License: MIT. Source: https://github.com/Peternator7/strum
Authors: Peter Glotfelty <peter.glotfelty@microsoft.com>.

`LICENSE` (strum 0.27.2): the same text as `LICENSE` (strum 0.26.3) above.

## substrate-bn-succinct-rs 0.6.0

License: MIT OR Apache-2.0. Source: https://github.com/paritytech/bn
Authors: Sean Bowe <ewillbefull@gmail.com>, Parity Technologies <admin@parity.io>, Nathan <n@push0.dev>.

`LICENSE-APACHE` (substrate-bn-succinct-rs 0.6.0):

    Copyright (c) 2016 Zcash Electric Coin Company

                                     Apache License
                               Version 2.0, January 2004
                            http://www.apache.org/licenses/

       TERMS AND CONDITIONS FOR USE, REPRODUCTION, AND DISTRIBUTION

       1. Definitions.

          "License" shall mean the terms and conditions for use, reproduction,
          and distribution as defined by Sections 1 through 9 of this document.

          "Licensor" shall mean the copyright owner or entity authorized by
          the copyright owner that is granting the License.

          "Legal Entity" shall mean the union of the acting entity and all
          other entities that control, are controlled by, or are under common
          control with that entity. For the purposes of this definition,
          "control" means (i) the power, direct or indirect, to cause the
          direction or management of such entity, whether by contract or
          otherwise, or (ii) ownership of fifty percent (50%) or more of the
          outstanding shares, or (iii) beneficial ownership of such entity.

          "You" (or "Your") shall mean an individual or Legal Entity
          exercising permissions granted by this License.

          "Source" form shall mean the preferred form for making modifications,
          including but not limited to software source code, documentation
          source, and configuration files.

          "Object" form shall mean any form resulting from mechanical
          transformation or translation of a Source form, including but
          not limited to compiled object code, generated documentation,
          and conversions to other media types.

          "Work" shall mean the work of authorship, whether in Source or
          Object form, made available under the License, as indicated by a
          copyright notice that is included in or attached to the work
          (an example is provided in the Appendix below).

          "Derivative Works" shall mean any work, whether in Source or Object
          form, that is based on (or derived from) the Work and for which the
          editorial revisions, annotations, elaborations, or other modifications
          represent, as a whole, an original work of authorship. For the purposes
          of this License, Derivative Works shall not include works that remain
          separable from, or merely link (or bind by name) to the interfaces of,
          the Work and Derivative Works thereof.

          "Contribution" shall mean any work of authorship, including
          the original version of the Work and any modifications or additions
          to that Work or Derivative Works thereof, that is intentionally
          submitted to Licensor for inclusion in the Work by the copyright owner
          or by an individual or Legal Entity authorized to submit on behalf of
          the copyright owner. For the purposes of this definition, "submitted"
          means any form of electronic, verbal, or written communication sent
          to the Licensor or its representatives, including but not limited to
          communication on electronic mailing lists, source code control systems,
          and issue tracking systems that are managed by, or on behalf of, the
          Licensor for the purpose of discussing and improving the Work, but
          excluding communication that is conspicuously marked or otherwise
          designated in writing by the copyright owner as "Not a Contribution."

          "Contributor" shall mean Licensor and any individual or Legal Entity
          on behalf of whom a Contribution has been received by Licensor and
          subsequently incorporated within the Work.

       2. Grant of Copyright License. Subject to the terms and conditions of
          this License, each Contributor hereby grants to You a perpetual,
          worldwide, non-exclusive, no-charge, royalty-free, irrevocable
          copyright license to reproduce, prepare Derivative Works of,
          publicly display, publicly perform, sublicense, and distribute the
          Work and such Derivative Works in Source or Object form.

       3. Grant of Patent License. Subject to the terms and conditions of
          this License, each Contributor hereby grants to You a perpetual,
          worldwide, non-exclusive, no-charge, royalty-free, irrevocable
          (except as stated in this section) patent license to make, have made,
          use, offer to sell, sell, import, and otherwise transfer the Work,
          where such license applies only to those patent claims licensable
          by such Contributor that are necessarily infringed by their
          Contribution(s) alone or by combination of their Contribution(s)
          with the Work to which such Contribution(s) was submitted. If You
          institute patent litigation against any entity (including a
          cross-claim or counterclaim in a lawsuit) alleging that the Work
          or a Contribution incorporated within the Work constitutes direct
          or contributory patent infringement, then any patent licenses
          granted to You under this License for that Work shall terminate
          as of the date such litigation is filed.

       4. Redistribution. You may reproduce and distribute copies of the
          Work or Derivative Works thereof in any medium, with or without
          modifications, and in Source or Object form, provided that You
          meet the following conditions:

          (a) You must give any other recipients of the Work or
              Derivative Works a copy of this License; and

          (b) You must cause any modified files to carry prominent notices
              stating that You changed the files; and

          (c) You must retain, in the Source form of any Derivative Works
              that You distribute, all copyright, patent, trademark, and
              attribution notices from the Source form of the Work,
              excluding those notices that do not pertain to any part of
              the Derivative Works; and

          (d) If the Work includes a "NOTICE" text file as part of its
              distribution, then any Derivative Works that You distribute must
              include a readable copy of the attribution notices contained
              within such NOTICE file, excluding those notices that do not
              pertain to any part of the Derivative Works, in at least one
              of the following places: within a NOTICE text file distributed
              as part of the Derivative Works; within the Source form or
              documentation, if provided along with the Derivative Works; or,
              within a display generated by the Derivative Works, if and
              wherever such third-party notices normally appear. The contents
              of the NOTICE file are for informational purposes only and
              do not modify the License. You may add Your own attribution
              notices within Derivative Works that You distribute, alongside
              or as an addendum to the NOTICE text from the Work, provided
              that such additional attribution notices cannot be construed
              as modifying the License.

          You may add Your own copyright statement to Your modifications and
          may provide additional or different license terms and conditions
          for use, reproduction, or distribution of Your modifications, or
          for any such Derivative Works as a whole, provided Your use,
          reproduction, and distribution of the Work otherwise complies with
          the conditions stated in this License.

       5. Submission of Contributions. Unless You explicitly state otherwise,
          any Contribution intentionally submitted for inclusion in the Work
          by You to the Licensor shall be under the terms and conditions of
          this License, without any additional terms or conditions.
          Notwithstanding the above, nothing herein shall supersede or modify
          the terms of any separate license agreement you may have executed
          with Licensor regarding such Contributions.

       6. Trademarks. This License does not grant permission to use the trade
          names, trademarks, service marks, or product names of the Licensor,
          except as required for reasonable and customary use in describing the
          origin of the Work and reproducing the content of the NOTICE file.

       7. Disclaimer of Warranty. Unless required by applicable law or
          agreed to in writing, Licensor provides the Work (and each
          Contributor provides its Contributions) on an "AS IS" BASIS,
          WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or
          implied, including, without limitation, any warranties or conditions
          of TITLE, NON-INFRINGEMENT, MERCHANTABILITY, or FITNESS FOR A
          PARTICULAR PURPOSE. You are solely responsible for determining the
          appropriateness of using or redistributing the Work and assume any
          risks associated with Your exercise of permissions under this License.

       8. Limitation of Liability. In no event and under no legal theory,
          whether in tort (including negligence), contract, or otherwise,
          unless required by applicable law (such as deliberate and grossly
          negligent acts) or agreed to in writing, shall any Contributor be
          liable to You for damages, including any direct, indirect, special,
          incidental, or consequential damages of any character arising as a
          result of this License or out of the use or inability to use the
          Work (including but not limited to damages for loss of goodwill,
          work stoppage, computer failure or malfunction, or any and all
          other commercial damages or losses), even if such Contributor
          has been advised of the possibility of such damages.

       9. Accepting Warranty or Additional Liability. While redistributing
          the Work or Derivative Works thereof, You may choose to offer,
          and charge a fee for, acceptance of support, warranty, indemnity,
          or other liability obligations and/or rights consistent with this
          License. However, in accepting such obligations, You may act only
          on Your own behalf and on Your sole responsibility, not on behalf
          of any other Contributor, and only if You agree to indemnify,
          defend, and hold each Contributor harmless for any liability
          incurred by, or claims asserted against, such Contributor by reason
          of your accepting any such warranty or additional liability.

       END OF TERMS AND CONDITIONS

       APPENDIX: How to apply the Apache License to your work.

          To apply the Apache License to your work, attach the following
          boilerplate notice, with the fields enclosed by brackets "{}"
          replaced with your own identifying information. (Don't include
          the brackets!)  The text should be enclosed in the appropriate
          comment syntax for the file format. We also recommend that a
          file or class name and description of purpose be included on the
          same "printed page" as the copyright notice for easier
          identification within third-party archives.

       Copyright {yyyy} {name of copyright owner}

       Licensed under the Apache License, Version 2.0 (the "License");
       you may not use this file except in compliance with the License.
       You may obtain a copy of the License at

           http://www.apache.org/licenses/LICENSE-2.0

       Unless required by applicable law or agreed to in writing, software
       distributed under the License is distributed on an "AS IS" BASIS,
       WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
       See the License for the specific language governing permissions and
       limitations under the License.

`LICENSE-MIT` (substrate-bn-succinct-rs 0.6.0):

    Copyright (c) 2016 Zcash Electric Coin Company

    Permission is hereby granted, free of charge, to any person obtaining a copy
    of this software and associated documentation files (the "Software"), to deal
    in the Software without restriction, including without limitation the rights
    to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
    copies of the Software, and to permit persons to whom the Software is
    furnished to do so, subject to the following conditions:

    The above copyright notice and this permission notice shall be included in all
    copies or substantial portions of the Software.

    THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
    IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
    FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
    AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
    LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
    OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
    SOFTWARE.

## subtle 2.6.1

License: BSD-3-Clause. Source: https://github.com/dalek-cryptography/subtle
Authors: Isis Lovecruft <isis@patternsinthevoid.net>, Henry de Valence <hdevalence@hdevalence.ca>.

`LICENSE` (subtle 2.6.1):

    Copyright (c) 2016-2017 Isis Agora Lovecruft, Henry de Valence. All rights reserved.
    Copyright (c) 2016-2024 Isis Agora Lovecruft. All rights reserved.

    Redistribution and use in source and binary forms, with or without
    modification, are permitted provided that the following conditions are
    met:

    1. Redistributions of source code must retain the above copyright
    notice, this list of conditions and the following disclaimer.

    2. Redistributions in binary form must reproduce the above copyright
    notice, this list of conditions and the following disclaimer in the
    documentation and/or other materials provided with the distribution.

    3. Neither the name of the copyright holder nor the names of its
    contributors may be used to endorse or promote products derived from
    this software without specific prior written permission.

    THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS
    IS" AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED
    TO, THE IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A
    PARTICULAR PURPOSE ARE DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT
    HOLDER OR CONTRIBUTORS BE LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL,
    SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT LIMITED
    TO, PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES; LOSS OF USE, DATA, OR
    PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON ANY THEORY OF
    LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT (INCLUDING
    NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE OF THIS
    SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE. 

## tap 1.0.1

License: MIT. Source: https://github.com/myrrlyn/tap
Authors: Elliott Linder <elliott.darfink@gmail.com>, myrrlyn <self@myrrlyn.dev>.

`LICENSE.txt` (tap 1.0.1):

    MIT License

    Copyright (c) 2017 Elliot Linder <darfink@gmail.com>

    Permission is hereby granted, free of charge, to any person obtaining a copy
    of this software and associated documentation files (the "Software"), to deal
    in the Software without restriction, including without limitation the rights
    to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
    copies of the Software, and to permit persons to whom the Software is
    furnished to do so, subject to the following conditions:

    The above copyright notice and this permission notice shall be included in all
    copies or substantial portions of the Software.

    THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
    IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
    FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
    AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
    LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
    OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
    SOFTWARE.

## thiserror 1.0.69

License: MIT OR Apache-2.0. Source: https://github.com/dtolnay/thiserror
Authors: David Tolnay <dtolnay@gmail.com>.

`LICENSE-APACHE` (thiserror 1.0.69): the same text as `LICENSE-APACHE` (allocator-api2 0.2.21) above.

`LICENSE-MIT` (thiserror 1.0.69): the same text as `LICENSE-MIT` (adler2 2.0.1) above.

## thiserror 2.0.20

License: MIT OR Apache-2.0. Source: https://github.com/dtolnay/thiserror
Authors: David Tolnay <dtolnay@gmail.com>.

`LICENSE-APACHE` (thiserror 2.0.20): the same text as `LICENSE-APACHE` (allocator-api2 0.2.21) above.

`LICENSE-MIT` (thiserror 2.0.20): the same text as `LICENSE-MIT` (adler2 2.0.1) above.

## thiserror 2.0.21

License: MIT OR Apache-2.0. Source: https://github.com/dtolnay/thiserror
Authors: David Tolnay <dtolnay@gmail.com>.

`LICENSE-APACHE` (thiserror 2.0.21): the same text as `LICENSE-APACHE` (allocator-api2 0.2.21) above.

`LICENSE-MIT` (thiserror 2.0.21): the same text as `LICENSE-MIT` (adler2 2.0.1) above.

## thousands 0.2.0

License: MIT/Apache-2.0. Source: https://github.com/tov/thousands-rs
Authors: Jesse A. Tov <jesse.tov@gmail.com>.

`LICENSE-APACHE` (thousands 0.2.0): the same text as `LICENSE-APACHE` (addr2line 0.25.1) above.

`LICENSE-MIT` (thousands 0.2.0):

    Copyright (c) 2018 Jesse A. Tov

    Permission is hereby granted, free of charge, to any
    person obtaining a copy of this software and associated
    documentation files (the "Software"), to deal in the
    Software without restriction, including without
    limitation the rights to use, copy, modify, merge,
    publish, distribute, sublicense, and/or sell copies of
    the Software, and to permit persons to whom the Software
    is furnished to do so, subject to the following
    conditions:

    The above copyright notice and this permission notice
    shall be included in all copies or substantial portions
    of the Software.

    THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF
    ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED
    TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A
    PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT
    SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
    CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
    OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR
    IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
    DEALINGS IN THE SOFTWARE.

## thread_local 1.1.10

License: MIT OR Apache-2.0. Source: https://github.com/Amanieu/thread_local-rs
Authors: Amanieu d'Antras <amanieu@gmail.com>.

`LICENSE-APACHE` (thread_local 1.1.10): the same text as `LICENSE-APACHE` (addr2line 0.25.1) above.

`LICENSE-MIT` (thread_local 1.1.10): the same text as `LICENSE-MIT` (lock_api 0.4.14) above.

## tiny-keccak 2.0.2

License: CC0-1.0. Source: https://github.com/debris/tiny-keccak
Authors: debris <marek.kotewicz@gmail.com>.

`LICENSE` (tiny-keccak 2.0.2): the same text as `LICENSE_CC0` (blake3 1.8.7) above.

## tokio 1.53.1

License: MIT. Source: https://github.com/tokio-rs/tokio
Authors: Tokio Contributors <team@tokio.rs>.

`LICENSE` (tokio 1.53.1):

    MIT License

    Copyright (c) Tokio Contributors

    Permission is hereby granted, free of charge, to any person obtaining a copy
    of this software and associated documentation files (the "Software"), to deal
    in the Software without restriction, including without limitation the rights
    to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
    copies of the Software, and to permit persons to whom the Software is
    furnished to do so, subject to the following conditions:

    The above copyright notice and this permission notice shall be included in all
    copies or substantial portions of the Software.

    THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
    IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
    FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
    AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
    LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
    OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
    SOFTWARE.

## tracing 0.1.44

License: MIT. Source: https://github.com/tokio-rs/tracing
Authors: Eliza Weisman <eliza@buoyant.io>, Tokio Contributors <team@tokio.rs>.

`LICENSE` (tracing 0.1.44):

    Copyright (c) 2019 Tokio Contributors

    Permission is hereby granted, free of charge, to any
    person obtaining a copy of this software and associated
    documentation files (the "Software"), to deal in the
    Software without restriction, including without
    limitation the rights to use, copy, modify, merge,
    publish, distribute, sublicense, and/or sell copies of
    the Software, and to permit persons to whom the Software
    is furnished to do so, subject to the following
    conditions:

    The above copyright notice and this permission notice
    shall be included in all copies or substantial portions
    of the Software.

    THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF
    ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED
    TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A
    PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT
    SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
    CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
    OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR
    IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
    DEALINGS IN THE SOFTWARE.

## tracing-core 0.1.36

License: MIT. Source: https://github.com/tokio-rs/tracing
Authors: Tokio Contributors <team@tokio.rs>.

`LICENSE` (tracing-core 0.1.36): the same text as `LICENSE` (tracing 0.1.44) above.

## tracing-forest 0.1.6

License: MIT. Source: https://github.com/QnnOkabayashi/tracing-forest
Authors: Quinn Okabayashi.

Its package carries no license file: the standard text of its license, under its authors.

MIT (tracing-forest 0.1.6):

    MIT License

    Copyright (c) Quinn Okabayashi

    Permission is hereby granted, free of charge, to any person obtaining a copy of this software and associated documentation files (the "Software"), to deal in the Software without restriction, including without limitation the rights to use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of the Software, and to permit persons to whom the Software is furnished to do so, subject to the following conditions:

    The above copyright notice and this permission notice shall be included in all copies or substantial portions of the Software.

    THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

## tracing-log 0.2.0

License: MIT. Source: https://github.com/tokio-rs/tracing
Authors: Tokio Contributors <team@tokio.rs>.

`LICENSE` (tracing-log 0.2.0): the same text as `LICENSE` (tracing 0.1.44) above.

## tracing-subscriber 0.3.23

License: MIT. Source: https://github.com/tokio-rs/tracing
Authors: Eliza Weisman <eliza@buoyant.io>, David Barsky <me@davidbarsky.com>, Tokio Contributors <team@tokio.rs>.

`LICENSE` (tracing-subscriber 0.3.23): the same text as `LICENSE` (tracing 0.1.44) above.

## transpose 0.2.3

License: MIT OR Apache-2.0. Source: https://github.com/ejmahler/transpose
Authors: Elliott Mahler <join.together@gmail.com>.

`LICENSE-APACHE` (transpose 0.2.3):

                                  Apache License
                            Version 2.0, January 2004
                         http://www.apache.org/licenses/

    TERMS AND CONDITIONS FOR USE, REPRODUCTION, AND DISTRIBUTION

    1. Definitions.

       "License" shall mean the terms and conditions for use, reproduction,
       and distribution as defined by Sections 1 through 9 of this document.

       "Licensor" shall mean the copyright owner or entity authorized by
       the copyright owner that is granting the License.

       "Legal Entity" shall mean the union of the acting entity and all
       other entities that control, are controlled by, or are under common
       control with that entity. For the purposes of this definition,
       "control" means (i) the power, direct or indirect, to cause the
       direction or management of such entity, whether by contract or
       otherwise, or (ii) ownership of fifty percent (50%) or more of the
       outstanding shares, or (iii) beneficial ownership of such entity.

       "You" (or "Your") shall mean an individual or Legal Entity
       exercising permissions granted by this License.

       "Source" form shall mean the preferred form for making modifications,
       including but not limited to software source code, documentation
       source, and configuration files.

       "Object" form shall mean any form resulting from mechanical
       transformation or translation of a Source form, including but
       not limited to compiled object code, generated documentation,
       and conversions to other media types.

       "Work" shall mean the work of authorship, whether in Source or
       Object form, made available under the License, as indicated by a
       copyright notice that is included in or attached to the work
       (an example is provided in the Appendix below).

       "Derivative Works" shall mean any work, whether in Source or Object
       form, that is based on (or derived from) the Work and for which the
       editorial revisions, annotations, elaborations, or other modifications
       represent, as a whole, an original work of authorship. For the purposes
       of this License, Derivative Works shall not include works that remain
       separable from, or merely link (or bind by name) to the interfaces of,
       the Work and Derivative Works thereof.

       "Contribution" shall mean any work of authorship, including
       the original version of the Work and any modifications or additions
       to that Work or Derivative Works thereof, that is intentionally
       submitted to Licensor for inclusion in the Work by the copyright owner
       or by an individual or Legal Entity authorized to submit on behalf of
       the copyright owner. For the purposes of this definition, "submitted"
       means any form of electronic, verbal, or written communication sent
       to the Licensor or its representatives, including but not limited to
       communication on electronic mailing lists, source code control systems,
       and issue tracking systems that are managed by, or on behalf of, the
       Licensor for the purpose of discussing and improving the Work, but
       excluding communication that is conspicuously marked or otherwise
       designated in writing by the copyright owner as "Not a Contribution."

       "Contributor" shall mean Licensor and any individual or Legal Entity
       on behalf of whom a Contribution has been received by Licensor and
       subsequently incorporated within the Work.

    2. Grant of Copyright License. Subject to the terms and conditions of
       this License, each Contributor hereby grants to You a perpetual,
       worldwide, non-exclusive, no-charge, royalty-free, irrevocable
       copyright license to reproduce, prepare Derivative Works of,
       publicly display, publicly perform, sublicense, and distribute the
       Work and such Derivative Works in Source or Object form.

    3. Grant of Patent License. Subject to the terms and conditions of
       this License, each Contributor hereby grants to You a perpetual,
       worldwide, non-exclusive, no-charge, royalty-free, irrevocable
       (except as stated in this section) patent license to make, have made,
       use, offer to sell, sell, import, and otherwise transfer the Work,
       where such license applies only to those patent claims licensable
       by such Contributor that are necessarily infringed by their
       Contribution(s) alone or by combination of their Contribution(s)
       with the Work to which such Contribution(s) was submitted. If You
       institute patent litigation against any entity (including a
       cross-claim or counterclaim in a lawsuit) alleging that the Work
       or a Contribution incorporated within the Work constitutes direct
       or contributory patent infringement, then any patent licenses
       granted to You under this License for that Work shall terminate
       as of the date such litigation is filed.

    4. Redistribution. You may reproduce and distribute copies of the
       Work or Derivative Works thereof in any medium, with or without
       modifications, and in Source or Object form, provided that You
       meet the following conditions:

       (a) You must give any other recipients of the Work or
           Derivative Works a copy of this License; and

       (b) You must cause any modified files to carry prominent notices
           stating that You changed the files; and

       (c) You must retain, in the Source form of any Derivative Works
           that You distribute, all copyright, patent, trademark, and
           attribution notices from the Source form of the Work,
           excluding those notices that do not pertain to any part of
           the Derivative Works; and

       (d) If the Work includes a "NOTICE" text file as part of its
           distribution, then any Derivative Works that You distribute must
           include a readable copy of the attribution notices contained
           within such NOTICE file, excluding those notices that do not
           pertain to any part of the Derivative Works, in at least one
           of the following places: within a NOTICE text file distributed
           as part of the Derivative Works; within the Source form or
           documentation, if provided along with the Derivative Works; or,
           within a display generated by the Derivative Works, if and
           wherever such third-party notices normally appear. The contents
           of the NOTICE file are for informational purposes only and
           do not modify the License. You may add Your own attribution
           notices within Derivative Works that You distribute, alongside
           or as an addendum to the NOTICE text from the Work, provided
           that such additional attribution notices cannot be construed
           as modifying the License.

       You may add Your own copyright statement to Your modifications and
       may provide additional or different license terms and conditions
       for use, reproduction, or distribution of Your modifications, or
       for any such Derivative Works as a whole, provided Your use,
       reproduction, and distribution of the Work otherwise complies with
       the conditions stated in this License.

    5. Submission of Contributions. Unless You explicitly state otherwise,
       any Contribution intentionally submitted for inclusion in the Work
       by You to the Licensor shall be under the terms and conditions of
       this License, without any additional terms or conditions.
       Notwithstanding the above, nothing herein shall supersede or modify
       the terms of any separate license agreement you may have executed
       with Licensor regarding such Contributions.

    6. Trademarks. This License does not grant permission to use the trade
       names, trademarks, service marks, or product names of the Licensor,
       except as required for reasonable and customary use in describing the
       origin of the Work and reproducing the content of the NOTICE file.

    7. Disclaimer of Warranty. Unless required by applicable law or
       agreed to in writing, Licensor provides the Work (and each
       Contributor provides its Contributions) on an "AS IS" BASIS,
       WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or
       implied, including, without limitation, any warranties or conditions
       of TITLE, NON-INFRINGEMENT, MERCHANTABILITY, or FITNESS FOR A
       PARTICULAR PURPOSE. You are solely responsible for determining the
       appropriateness of using or redistributing the Work and assume any
       risks associated with Your exercise of permissions under this License.

    8. Limitation of Liability. In no event and under no legal theory,
       whether in tort (including negligence), contract, or otherwise,
       unless required by applicable law (such as deliberate and grossly
       negligent acts) or agreed to in writing, shall any Contributor be
       liable to You for damages, including any direct, indirect, special,
       incidental, or consequential damages of any character arising as a
       result of this License or out of the use or inability to use the
       Work (including but not limited to damages for loss of goodwill,
       work stoppage, computer failure or malfunction, or any and all
       other commercial damages or losses), even if such Contributor
       has been advised of the possibility of such damages.

    9. Accepting Warranty or Additional Liability. While redistributing
       the Work or Derivative Works thereof, You may choose to offer,
       and charge a fee for, acceptance of support, warranty, indemnity,
       or other liability obligations and/or rights consistent with this
       License. However, in accepting such obligations, You may act only
       on Your own behalf and on Your sole responsibility, not on behalf
       of any other Contributor, and only if You agree to indemnify,
       defend, and hold each Contributor harmless for any liability
       incurred by, or claims asserted against, such Contributor by reason
       of your accepting any such warranty or additional liability.

    END OF TERMS AND CONDITIONS

    APPENDIX: How to apply the Apache License to your work.

       To apply the Apache License to your work, attach the following
       boilerplate notice, with the fields enclosed by brackets "[]"
       replaced with your own identifying information. (Don't include
       the brackets!)  The text should be enclosed in the appropriate
       comment syntax for the file format. We also recommend that a
       file or class name and description of purpose be included on the
       same "printed page" as the copyright notice for easier
       identification within third-party archives.

    Copyright 2022 The transpose developers

    Licensed under the Apache License, Version 2.0 (the "License");
    you may not use this file except in compliance with the License.
    You may obtain a copy of the License at

    	http://www.apache.org/licenses/LICENSE-2.0

    Unless required by applicable law or agreed to in writing, software
    distributed under the License is distributed on an "AS IS" BASIS,
    WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
    See the License for the specific language governing permissions and
    limitations under the License.

`LICENSE-MIT` (transpose 0.2.3):

    Copyright (c) 2022 The transpose Developers

    Permission is hereby granted, free of charge, to any
    person obtaining a copy of this software and associated
    documentation files (the "Software"), to deal in the
    Software without restriction, including without
    limitation the rights to use, copy, modify, merge,
    publish, distribute, sublicense, and/or sell copies of
    the Software, and to permit persons to whom the Software
    is furnished to do so, subject to the following
    conditions:

    The above copyright notice and this permission notice
    shall be included in all copies or substantial portions
    of the Software.

    THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF
    ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED
    TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A
    PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT
    SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
    CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
    OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR
    IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
    DEALINGS IN THE SOFTWARE.

## typenum 1.20.1

License: MIT OR Apache-2.0. Source: https://github.com/paholg/typenum

`LICENSE` (typenum 1.20.1):

    MIT OR Apache-2.0

`LICENSE-APACHE` (typenum 1.20.1):

                                  Apache License
                            Version 2.0, January 2004
                         http://www.apache.org/licenses/

    TERMS AND CONDITIONS FOR USE, REPRODUCTION, AND DISTRIBUTION

    1. Definitions.

       "License" shall mean the terms and conditions for use, reproduction,
       and distribution as defined by Sections 1 through 9 of this document.

       "Licensor" shall mean the copyright owner or entity authorized by
       the copyright owner that is granting the License.

       "Legal Entity" shall mean the union of the acting entity and all
       other entities that control, are controlled by, or are under common
       control with that entity. For the purposes of this definition,
       "control" means (i) the power, direct or indirect, to cause the
       direction or management of such entity, whether by contract or
       otherwise, or (ii) ownership of fifty percent (50%) or more of the
       outstanding shares, or (iii) beneficial ownership of such entity.

       "You" (or "Your") shall mean an individual or Legal Entity
       exercising permissions granted by this License.

       "Source" form shall mean the preferred form for making modifications,
       including but not limited to software source code, documentation
       source, and configuration files.

       "Object" form shall mean any form resulting from mechanical
       transformation or translation of a Source form, including but
       not limited to compiled object code, generated documentation,
       and conversions to other media types.

       "Work" shall mean the work of authorship, whether in Source or
       Object form, made available under the License, as indicated by a
       copyright notice that is included in or attached to the work
       (an example is provided in the Appendix below).

       "Derivative Works" shall mean any work, whether in Source or Object
       form, that is based on (or derived from) the Work and for which the
       editorial revisions, annotations, elaborations, or other modifications
       represent, as a whole, an original work of authorship. For the purposes
       of this License, Derivative Works shall not include works that remain
       separable from, or merely link (or bind by name) to the interfaces of,
       the Work and Derivative Works thereof.

       "Contribution" shall mean any work of authorship, including
       the original version of the Work and any modifications or additions
       to that Work or Derivative Works thereof, that is intentionally
       submitted to Licensor for inclusion in the Work by the copyright owner
       or by an individual or Legal Entity authorized to submit on behalf of
       the copyright owner. For the purposes of this definition, "submitted"
       means any form of electronic, verbal, or written communication sent
       to the Licensor or its representatives, including but not limited to
       communication on electronic mailing lists, source code control systems,
       and issue tracking systems that are managed by, or on behalf of, the
       Licensor for the purpose of discussing and improving the Work, but
       excluding communication that is conspicuously marked or otherwise
       designated in writing by the copyright owner as "Not a Contribution."

       "Contributor" shall mean Licensor and any individual or Legal Entity
       on behalf of whom a Contribution has been received by Licensor and
       subsequently incorporated within the Work.

    2. Grant of Copyright License. Subject to the terms and conditions of
       this License, each Contributor hereby grants to You a perpetual,
       worldwide, non-exclusive, no-charge, royalty-free, irrevocable
       copyright license to reproduce, prepare Derivative Works of,
       publicly display, publicly perform, sublicense, and distribute the
       Work and such Derivative Works in Source or Object form.

    3. Grant of Patent License. Subject to the terms and conditions of
       this License, each Contributor hereby grants to You a perpetual,
       worldwide, non-exclusive, no-charge, royalty-free, irrevocable
       (except as stated in this section) patent license to make, have made,
       use, offer to sell, sell, import, and otherwise transfer the Work,
       where such license applies only to those patent claims licensable
       by such Contributor that are necessarily infringed by their
       Contribution(s) alone or by combination of their Contribution(s)
       with the Work to which such Contribution(s) was submitted. If You
       institute patent litigation against any entity (including a
       cross-claim or counterclaim in a lawsuit) alleging that the Work
       or a Contribution incorporated within the Work constitutes direct
       or contributory patent infringement, then any patent licenses
       granted to You under this License for that Work shall terminate
       as of the date such litigation is filed.

    4. Redistribution. You may reproduce and distribute copies of the
       Work or Derivative Works thereof in any medium, with or without
       modifications, and in Source or Object form, provided that You
       meet the following conditions:

       (a) You must give any other recipients of the Work or
           Derivative Works a copy of this License; and

       (b) You must cause any modified files to carry prominent notices
           stating that You changed the files; and

       (c) You must retain, in the Source form of any Derivative Works
           that You distribute, all copyright, patent, trademark, and
           attribution notices from the Source form of the Work,
           excluding those notices that do not pertain to any part of
           the Derivative Works; and

       (d) If the Work includes a "NOTICE" text file as part of its
           distribution, then any Derivative Works that You distribute must
           include a readable copy of the attribution notices contained
           within such NOTICE file, excluding those notices that do not
           pertain to any part of the Derivative Works, in at least one
           of the following places: within a NOTICE text file distributed
           as part of the Derivative Works; within the Source form or
           documentation, if provided along with the Derivative Works; or,
           within a display generated by the Derivative Works, if and
           wherever such third-party notices normally appear. The contents
           of the NOTICE file are for informational purposes only and
           do not modify the License. You may add Your own attribution
           notices within Derivative Works that You distribute, alongside
           or as an addendum to the NOTICE text from the Work, provided
           that such additional attribution notices cannot be construed
           as modifying the License.

       You may add Your own copyright statement to Your modifications and
       may provide additional or different license terms and conditions
       for use, reproduction, or distribution of Your modifications, or
       for any such Derivative Works as a whole, provided Your use,
       reproduction, and distribution of the Work otherwise complies with
       the conditions stated in this License.

    5. Submission of Contributions. Unless You explicitly state otherwise,
       any Contribution intentionally submitted for inclusion in the Work
       by You to the Licensor shall be under the terms and conditions of
       this License, without any additional terms or conditions.
       Notwithstanding the above, nothing herein shall supersede or modify
       the terms of any separate license agreement you may have executed
       with Licensor regarding such Contributions.

    6. Trademarks. This License does not grant permission to use the trade
       names, trademarks, service marks, or product names of the Licensor,
       except as required for reasonable and customary use in describing the
       origin of the Work and reproducing the content of the NOTICE file.

    7. Disclaimer of Warranty. Unless required by applicable law or
       agreed to in writing, Licensor provides the Work (and each
       Contributor provides its Contributions) on an "AS IS" BASIS,
       WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or
       implied, including, without limitation, any warranties or conditions
       of TITLE, NON-INFRINGEMENT, MERCHANTABILITY, or FITNESS FOR A
       PARTICULAR PURPOSE. You are solely responsible for determining the
       appropriateness of using or redistributing the Work and assume any
       risks associated with Your exercise of permissions under this License.

    8. Limitation of Liability. In no event and under no legal theory,
       whether in tort (including negligence), contract, or otherwise,
       unless required by applicable law (such as deliberate and grossly
       negligent acts) or agreed to in writing, shall any Contributor be
       liable to You for damages, including any direct, indirect, special,
       incidental, or consequential damages of any character arising as a
       result of this License or out of the use or inability to use the
       Work (including but not limited to damages for loss of goodwill,
       work stoppage, computer failure or malfunction, or any and all
       other commercial damages or losses), even if such Contributor
       has been advised of the possibility of such damages.

    9. Accepting Warranty or Additional Liability. While redistributing
       the Work or Derivative Works thereof, You may choose to offer,
       and charge a fee for, acceptance of support, warranty, indemnity,
       or other liability obligations and/or rights consistent with this
       License. However, in accepting such obligations, You may act only
       on Your own behalf and on Your sole responsibility, not on behalf
       of any other Contributor, and only if You agree to indemnify,
       defend, and hold each Contributor harmless for any liability
       incurred by, or claims asserted against, such Contributor by reason
       of your accepting any such warranty or additional liability.

    END OF TERMS AND CONDITIONS

    APPENDIX: How to apply the Apache License to your work.

       To apply the Apache License to your work, attach the following
       boilerplate notice, with the fields enclosed by brackets "[]"
       replaced with your own identifying information. (Don't include
       the brackets!)  The text should be enclosed in the appropriate
       comment syntax for the file format. We also recommend that a
       file or class name and description of purpose be included on the
       same "printed page" as the copyright notice for easier
       identification within third-party archives.

    Copyright 2014 Paho Lurie-Gregg

    Licensed under the Apache License, Version 2.0 (the "License");
    you may not use this file except in compliance with the License.
    You may obtain a copy of the License at

    	http://www.apache.org/licenses/LICENSE-2.0

    Unless required by applicable law or agreed to in writing, software
    distributed under the License is distributed on an "AS IS" BASIS,
    WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
    See the License for the specific language governing permissions and
    limitations under the License.

`LICENSE-MIT` (typenum 1.20.1):

    The MIT License (MIT)

    Copyright (c) 2014 Paho Lurie-Gregg

    Permission is hereby granted, free of charge, to any person obtaining a copy
    of this software and associated documentation files (the "Software"), to deal
    in the Software without restriction, including without limitation the rights
    to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
    copies of the Software, and to permit persons to whom the Software is
    furnished to do so, subject to the following conditions:

    The above copyright notice and this permission notice shall be included in all
    copies or substantial portions of the Software.

    THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
    IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
    FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
    AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
    LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
    OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
    SOFTWARE.

## unicode-ident 1.0.24

License: (MIT OR Apache-2.0) AND Unicode-3.0. Source: https://github.com/dtolnay/unicode-ident
Authors: David Tolnay <dtolnay@gmail.com>.

`LICENSE-APACHE` (unicode-ident 1.0.24): the same text as `LICENSE-APACHE` (allocator-api2 0.2.21) above.

`LICENSE-MIT` (unicode-ident 1.0.24): the same text as `LICENSE-MIT` (adler2 2.0.1) above.

`LICENSE-UNICODE` (unicode-ident 1.0.24): the same text as Unicode-3.0 above.

## unicode-ident 1.0.27

License: (MIT OR Apache-2.0) AND Unicode-3.0. Source: https://github.com/dtolnay/unicode-ident
Authors: David Tolnay <dtolnay@gmail.com>.

`LICENSE-APACHE` (unicode-ident 1.0.27): the same text as `LICENSE-APACHE` (allocator-api2 0.2.21) above.

`LICENSE-MIT` (unicode-ident 1.0.27): the same text as `LICENSE-MIT` (adler2 2.0.1) above.

`LICENSE-UNICODE` (unicode-ident 1.0.27): the same text as Unicode-3.0 above.

## wasm-bindgen 0.2.129

License: MIT OR Apache-2.0. Source: https://github.com/wasm-bindgen/wasm-bindgen
Authors: The wasm-bindgen Developers.

`LICENSE-APACHE` (wasm-bindgen 0.2.129): the same text as `LICENSE-APACHE` (addr2line 0.25.1) above.

`LICENSE-MIT` (wasm-bindgen 0.2.129): the same text as `LICENSE-MIT` (backtrace 0.3.76) above.

## wasm-bindgen-shared 0.2.129

License: MIT OR Apache-2.0. Source: https://github.com/wasm-bindgen/wasm-bindgen/tree/master/crates/shared
Authors: The wasm-bindgen Developers.

`LICENSE-APACHE` (wasm-bindgen-shared 0.2.129): the same text as `LICENSE-APACHE` (addr2line 0.25.1) above.

`LICENSE-MIT` (wasm-bindgen-shared 0.2.129): the same text as `LICENSE-MIT` (backtrace 0.3.76) above.

## web-sys 0.3.106

License: MIT OR Apache-2.0. Source: https://github.com/wasm-bindgen/wasm-bindgen/tree/master/crates/web-sys
Authors: The wasm-bindgen Developers.

`LICENSE-APACHE` (web-sys 0.3.106): the same text as `LICENSE-APACHE` (addr2line 0.25.1) above.

`LICENSE-MIT` (web-sys 0.3.106): the same text as `LICENSE-MIT` (backtrace 0.3.76) above.

## wyz 0.5.1

License: MIT. Source: https://github.com/myrrlyn/wyz
Authors: myrrlyn <self@myrrlyn.dev>.

`LICENSE.txt` (wyz 0.5.1): the same text as `LICENSE.txt` (bitvec 1.1.1) above.

## zerocopy 0.8.56

License: BSD-2-Clause OR Apache-2.0 OR MIT. Source: https://github.com/google/zerocopy

`LICENSE-APACHE` (zerocopy 0.8.56):

                                     Apache License
                               Version 2.0, January 2004
                            http://www.apache.org/licenses/

       TERMS AND CONDITIONS FOR USE, REPRODUCTION, AND DISTRIBUTION

       1. Definitions.

          "License" shall mean the terms and conditions for use, reproduction,
          and distribution as defined by Sections 1 through 9 of this document.

          "Licensor" shall mean the copyright owner or entity authorized by
          the copyright owner that is granting the License.

          "Legal Entity" shall mean the union of the acting entity and all
          other entities that control, are controlled by, or are under common
          control with that entity. For the purposes of this definition,
          "control" means (i) the power, direct or indirect, to cause the
          direction or management of such entity, whether by contract or
          otherwise, or (ii) ownership of fifty percent (50%) or more of the
          outstanding shares, or (iii) beneficial ownership of such entity.

          "You" (or "Your") shall mean an individual or Legal Entity
          exercising permissions granted by this License.

          "Source" form shall mean the preferred form for making modifications,
          including but not limited to software source code, documentation
          source, and configuration files.

          "Object" form shall mean any form resulting from mechanical
          transformation or translation of a Source form, including but
          not limited to compiled object code, generated documentation,
          and conversions to other media types.

          "Work" shall mean the work of authorship, whether in Source or
          Object form, made available under the License, as indicated by a
          copyright notice that is included in or attached to the work
          (an example is provided in the Appendix below).

          "Derivative Works" shall mean any work, whether in Source or Object
          form, that is based on (or derived from) the Work and for which the
          editorial revisions, annotations, elaborations, or other modifications
          represent, as a whole, an original work of authorship. For the purposes
          of this License, Derivative Works shall not include works that remain
          separable from, or merely link (or bind by name) to the interfaces of,
          the Work and Derivative Works thereof.

          "Contribution" shall mean any work of authorship, including
          the original version of the Work and any modifications or additions
          to that Work or Derivative Works thereof, that is intentionally
          submitted to Licensor for inclusion in the Work by the copyright owner
          or by an individual or Legal Entity authorized to submit on behalf of
          the copyright owner. For the purposes of this definition, "submitted"
          means any form of electronic, verbal, or written communication sent
          to the Licensor or its representatives, including but not limited to
          communication on electronic mailing lists, source code control systems,
          and issue tracking systems that are managed by, or on behalf of, the
          Licensor for the purpose of discussing and improving the Work, but
          excluding communication that is conspicuously marked or otherwise
          designated in writing by the copyright owner as "Not a Contribution."

          "Contributor" shall mean Licensor and any individual or Legal Entity
          on behalf of whom a Contribution has been received by Licensor and
          subsequently incorporated within the Work.

       2. Grant of Copyright License. Subject to the terms and conditions of
          this License, each Contributor hereby grants to You a perpetual,
          worldwide, non-exclusive, no-charge, royalty-free, irrevocable
          copyright license to reproduce, prepare Derivative Works of,
          publicly display, publicly perform, sublicense, and distribute the
          Work and such Derivative Works in Source or Object form.

       3. Grant of Patent License. Subject to the terms and conditions of
          this License, each Contributor hereby grants to You a perpetual,
          worldwide, non-exclusive, no-charge, royalty-free, irrevocable
          (except as stated in this section) patent license to make, have made,
          use, offer to sell, sell, import, and otherwise transfer the Work,
          where such license applies only to those patent claims licensable
          by such Contributor that are necessarily infringed by their
          Contribution(s) alone or by combination of their Contribution(s)
          with the Work to which such Contribution(s) was submitted. If You
          institute patent litigation against any entity (including a
          cross-claim or counterclaim in a lawsuit) alleging that the Work
          or a Contribution incorporated within the Work constitutes direct
          or contributory patent infringement, then any patent licenses
          granted to You under this License for that Work shall terminate
          as of the date such litigation is filed.

       4. Redistribution. You may reproduce and distribute copies of the
          Work or Derivative Works thereof in any medium, with or without
          modifications, and in Source or Object form, provided that You
          meet the following conditions:

          (a) You must give any other recipients of the Work or
              Derivative Works a copy of this License; and

          (b) You must cause any modified files to carry prominent notices
              stating that You changed the files; and

          (c) You must retain, in the Source form of any Derivative Works
              that You distribute, all copyright, patent, trademark, and
              attribution notices from the Source form of the Work,
              excluding those notices that do not pertain to any part of
              the Derivative Works; and

          (d) If the Work includes a "NOTICE" text file as part of its
              distribution, then any Derivative Works that You distribute must
              include a readable copy of the attribution notices contained
              within such NOTICE file, excluding those notices that do not
              pertain to any part of the Derivative Works, in at least one
              of the following places: within a NOTICE text file distributed
              as part of the Derivative Works; within the Source form or
              documentation, if provided along with the Derivative Works; or,
              within a display generated by the Derivative Works, if and
              wherever such third-party notices normally appear. The contents
              of the NOTICE file are for informational purposes only and
              do not modify the License. You may add Your own attribution
              notices within Derivative Works that You distribute, alongside
              or as an addendum to the NOTICE text from the Work, provided
              that such additional attribution notices cannot be construed
              as modifying the License.

          You may add Your own copyright statement to Your modifications and
          may provide additional or different license terms and conditions
          for use, reproduction, or distribution of Your modifications, or
          for any such Derivative Works as a whole, provided Your use,
          reproduction, and distribution of the Work otherwise complies with
          the conditions stated in this License.

       5. Submission of Contributions. Unless You explicitly state otherwise,
          any Contribution intentionally submitted for inclusion in the Work
          by You to the Licensor shall be under the terms and conditions of
          this License, without any additional terms or conditions.
          Notwithstanding the above, nothing herein shall supersede or modify
          the terms of any separate license agreement you may have executed
          with Licensor regarding such Contributions.

       6. Trademarks. This License does not grant permission to use the trade
          names, trademarks, service marks, or product names of the Licensor,
          except as required for reasonable and customary use in describing the
          origin of the Work and reproducing the content of the NOTICE file.

       7. Disclaimer of Warranty. Unless required by applicable law or
          agreed to in writing, Licensor provides the Work (and each
          Contributor provides its Contributions) on an "AS IS" BASIS,
          WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or
          implied, including, without limitation, any warranties or conditions
          of TITLE, NON-INFRINGEMENT, MERCHANTABILITY, or FITNESS FOR A
          PARTICULAR PURPOSE. You are solely responsible for determining the
          appropriateness of using or redistributing the Work and assume any
          risks associated with Your exercise of permissions under this License.

       8. Limitation of Liability. In no event and under no legal theory,
          whether in tort (including negligence), contract, or otherwise,
          unless required by applicable law (such as deliberate and grossly
          negligent acts) or agreed to in writing, shall any Contributor be
          liable to You for damages, including any direct, indirect, special,
          incidental, or consequential damages of any character arising as a
          result of this License or out of the use or inability to use the
          Work (including but not limited to damages for loss of goodwill,
          work stoppage, computer failure or malfunction, or any and all
          other commercial damages or losses), even if such Contributor
          has been advised of the possibility of such damages.

       9. Accepting Warranty or Additional Liability. While redistributing
          the Work or Derivative Works thereof, You may choose to offer,
          and charge a fee for, acceptance of support, warranty, indemnity,
          or other liability obligations and/or rights consistent with this
          License. However, in accepting such obligations, You may act only
          on Your own behalf and on Your sole responsibility, not on behalf
          of any other Contributor, and only if You agree to indemnify,
          defend, and hold each Contributor harmless for any liability
          incurred by, or claims asserted against, such Contributor by reason
          of your accepting any such warranty or additional liability.

       END OF TERMS AND CONDITIONS

       APPENDIX: How to apply the Apache License to your work.

          To apply the Apache License to your work, attach the following
          boilerplate notice, with the fields enclosed by brackets "[]"
          replaced with your own identifying information. (Don't include
          the brackets!)  The text should be enclosed in the appropriate
          comment syntax for the file format. We also recommend that a
          file or class name and description of purpose be included on the
          same "printed page" as the copyright notice for easier
          identification within third-party archives.

       Copyright 2023 The Fuchsia Authors

       Licensed under the Apache License, Version 2.0 (the "License");
       you may not use this file except in compliance with the License.
       You may obtain a copy of the License at

           http://www.apache.org/licenses/LICENSE-2.0

       Unless required by applicable law or agreed to in writing, software
       distributed under the License is distributed on an "AS IS" BASIS,
       WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
       See the License for the specific language governing permissions and
       limitations under the License.

`LICENSE-BSD` (zerocopy 0.8.56):

    Copyright 2019 The Fuchsia Authors.

    Redistribution and use in source and binary forms, with or without
    modification, are permitted provided that the following conditions are
    met:

       * Redistributions of source code must retain the above copyright
    notice, this list of conditions and the following disclaimer.
       * Redistributions in binary form must reproduce the above
    copyright notice, this list of conditions and the following disclaimer
    in the documentation and/or other materials provided with the
    distribution.

    THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS
    "AS IS" AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT
    LIMITED TO, THE IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR
    A PARTICULAR PURPOSE ARE DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT
    OWNER OR CONTRIBUTORS BE LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL,
    SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT
    LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES; LOSS OF USE,
    DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON ANY
    THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT
    (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE
    OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.

`LICENSE-MIT` (zerocopy 0.8.56):

    Copyright 2023 The Fuchsia Authors

    Permission is hereby granted, free of charge, to any
    person obtaining a copy of this software and associated
    documentation files (the "Software"), to deal in the
    Software without restriction, including without
    limitation the rights to use, copy, modify, merge,
    publish, distribute, sublicense, and/or sell copies of
    the Software, and to permit persons to whom the Software
    is furnished to do so, subject to the following
    conditions:

    The above copyright notice and this permission notice
    shall be included in all copies or substantial portions
    of the Software.

    THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF
    ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED
    TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A
    PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT
    SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
    CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
    OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR
    IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
    DEALINGS IN THE SOFTWARE.

## zerocopy 0.8.63

License: BSD-2-Clause OR Apache-2.0 OR MIT. Source: https://github.com/google/zerocopy

`LICENSE-APACHE` (zerocopy 0.8.63): the same text as `LICENSE-APACHE` (zerocopy 0.8.56) above.

`LICENSE-BSD` (zerocopy 0.8.63): the same text as `LICENSE-BSD` (zerocopy 0.8.56) above.

`LICENSE-MIT` (zerocopy 0.8.63): the same text as `LICENSE-MIT` (zerocopy 0.8.56) above.

## zeroize 1.9.0

License: Apache-2.0 OR MIT. Source: https://github.com/RustCrypto/utils
Authors: The RustCrypto Project Developers.

`LICENSE-APACHE` (zeroize 1.9.0): the same text as `LICENSE-APACHE` (serdect 0.2.0) above.

`LICENSE-MIT` (zeroize 1.9.0): the same text as `LICENSE-MIT` (keccak 0.2.2) above.

## zeroize 1.9.1

License: Apache-2.0 OR MIT. Source: https://github.com/RustCrypto/utils
Authors: The RustCrypto Project Developers.

`LICENSE-APACHE` (zeroize 1.9.1): the same text as `LICENSE-APACHE` (serdect 0.2.0) above.

`LICENSE-MIT` (zeroize 1.9.1): the same text as `LICENSE-MIT` (keccak 0.2.2) above.

## zeroize 1.9.2

License: Apache-2.0 OR MIT. Source: https://github.com/RustCrypto/utils
Authors: The RustCrypto Project Developers.

`LICENSE-APACHE` (zeroize 1.9.2): the same text as `LICENSE-APACHE` (serdect 0.2.0) above.

`LICENSE-MIT` (zeroize 1.9.2): the same text as `LICENSE-MIT` (keccak 0.2.2) above.

## zkhash-axiom 0.2.0

License: MIT OR Apache-2.0. Source: https://github.com/axiom-crypto/poseidon2
Authors: Markus Schofnegger <mschofnegger@horizenlabs.io>, Roman Walch <roman.walch@iaik.tugraz.at>.

Its package carries no license file: the standard text of its license, under its authors.

Apache-2.0 (zkhash-axiom 0.2.0): the same text as Apache-2.0 above.

MIT (zkhash-axiom 0.2.0):

    MIT License

    Copyright (c) Markus Schofnegger <mschofnegger@horizenlabs.io>, Roman Walch <roman.walch@iaik.tugraz.at>

    Permission is hereby granted, free of charge, to any person obtaining a copy of this software and associated documentation files (the "Software"), to deal in the Software without restriction, including without limitation the rights to use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of the Software, and to permit persons to whom the Software is furnished to do so, subject to the following conditions:

    The above copyright notice and this permission notice shall be included in all copies or substantial portions of the Software.

    THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

## zmij 1.0.23

License: MIT. Source: https://github.com/dtolnay/zmij
Authors: David Tolnay <dtolnay@gmail.com>.

`LICENSE-MIT` (zmij 1.0.23): the same text as `LICENSE-MIT` (adler2 2.0.1) above.

## zstd 0.13.3

License: MIT. Source: https://github.com/gyscos/zstd-rs
Authors: Alexandre Bury <alexandre.bury@gmail.com>.

`LICENSE` (zstd 0.13.3):

    The MIT License (MIT)
    Copyright (c) 2016 Alexandre Bury

    Permission is hereby granted, free of charge, to any person obtaining a copy of this software and associated documentation files (the "Software"), to deal in the Software without restriction, including without limitation the rights to use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of the Software, and to permit persons to whom the Software is furnished to do so, subject to the following conditions:

    The above copyright notice and this permission notice shall be included in all copies or substantial portions of the Software.

    THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

## zstd-safe 7.3.0

License: BSD-3-Clause. Source: https://github.com/gyscos/zstd-rs
Authors: Alexandre Bury <alexandre.bury@gmail.com>.

`LICENSE` (zstd-safe 7.3.0):

    BSD 3-Clause License

    Copyright (c) 2026, Alexandre Bury

    Redistribution and use in source and binary forms, with or without
    modification, are permitted provided that the following conditions are met:

    1. Redistributions of source code must retain the above copyright notice, this
       list of conditions and the following disclaimer.

    2. Redistributions in binary form must reproduce the above copyright notice,
       this list of conditions and the following disclaimer in the documentation
       and/or other materials provided with the distribution.

    3. Neither the name of the copyright holder nor the names of its
       contributors may be used to endorse or promote products derived from
       this software without specific prior written permission.

    THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS IS"
    AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE
    IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE
    DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT HOLDER OR CONTRIBUTORS BE LIABLE
    FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL
    DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR
    SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER
    CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY,
    OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE
    OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.

## zstd-sys 2.1.2+zstd.1.5.7

License: BSD-3-Clause. Source: https://github.com/gyscos/zstd-rs
Authors: Alexandre Bury <alexandre.bury@gmail.com>.

`LICENSE` (zstd-sys 2.1.2+zstd.1.5.7): the same text as `LICENSE` (zstd-safe 7.3.0) above.

`LICENSE.BSD-3-Clause` (zstd-sys 2.1.2+zstd.1.5.7):

    The auto-generated bindings are under the 3-clause BSD license:

    BSD License

    For Zstandard software

    Copyright (c) Meta Platforms, Inc. and affiliates. All rights reserved.

    Redistribution and use in source and binary forms, with or without modification,
    are permitted provided that the following conditions are met:

     * Redistributions of source code must retain the above copyright notice, this
       list of conditions and the following disclaimer.

     * Redistributions in binary form must reproduce the above copyright notice,
       this list of conditions and the following disclaimer in the documentation
       and/or other materials provided with the distribution.

     * Neither the name Facebook, nor Meta, nor the names of its contributors may
       be used to endorse or promote products derived from this software without
       specific prior written permission.

    THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS IS" AND
    ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE IMPLIED
    WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE
    DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT HOLDER OR CONTRIBUTORS BE LIABLE FOR
    ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES
    (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES;
    LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON
    ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT
    (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE OF THIS
    SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.

