# Willow browser verifier

This is the source of the page at https://api.willow.tech/verify/. The page verifies Willow's proofs in your browser. With this repository you can read the code that runs, build the page yourself, and confirm that the files Willow serves are the files this source builds.

## What the page verifies

Everything runs in a Web Worker, in five WebAssembly modules built from the Rust crates here.

- **Logs** (`zkvm/logset-guest/query-wasm`, with `zkvm/logset-guest-openvm/verify-wasm`). Willow responds to a log query with zkVM proofs that each block's logs are complete, plus proofs over its log index. Each chain's proofs are SP1 proofs up to its switch to OpenVM, and OpenVM 2.0.2 proofs from it. The page tells the two apart by each proof's first bytes, verifies every proof against its own system's program keys pinned in this repository, works out from your filter which logs must be in the response, and compares Willow's plain `eth_getLogs` response with the proven logs. SP1's and OpenVM's verifiers cannot be built into one module (they pin different versions of a shared crate), so OpenVM proofs are verified in a second module, loaded only for a response that has one. The log module decides which key each proof is checked under; the OpenVM module checks it and verifies under the pinned OpenVM programs only.
- **Transactions, receipts, blocks and fees** (`zkvm/logset-guest/items-wasm`). Merkle-Patricia proofs: the header hashes to the block hash, each transaction or receipt is under the header's roots, and the block and transaction are the ones you asked for. The page then compares Willow's plain responses with the proven ones, field by field. The same module verifies block proofs, the header and every transaction, which rebuild its roots, and fee proofs: `eth_gasPrice`, `eth_maxPriorityFeePerGas`, `eth_blobBaseFee` and `eth_feeHistory`, computed by a published formula, `willow-fee-v1`, over verified blocks. No zero-knowledge proof is involved here.
- **Accounts, calls, gas estimates and traces** (`zkvm/logset-guest/state-wasm`, built from `zkvm/logset-guest/state-verify` and `zkvm/logset-guest/trace-verify`). An account proof is the account's path, and each storage slot's, in the state trie of a header that hashes to the block hash. For a call the page runs the EVM itself over every account, storage slot and code the call reads, each proven against that header's state root, under the chain's own rules: Ethereum's, Unichain's (`crates/op-exec`) or Robinhood Chain's (`crates/arb-exec`). It runs the chain's own gas estimation the same way: Willow's implementation of the search geth runs on Ethereum and Unichain, and of the one Nitro runs on Robinhood Chain, with their results. For a trace it runs the block's transactions over state proven against the parent block's state root, verifies each run against its receipt, and writes the trace with the tracer Willow's trace methods use. A read the proof does not hold fails verification. The page then compares Willow's plain responses with the proven ones.
- **Blocks** (`zkvm/logset-guest/anchor-wasm`, built from `zkvm/logset-guest/anchor-verify`). Every proof above ends at a block hash, and the page confirms that hash is block N of its chain. On Ethereum it runs Ethereum's light client from a checkpoint built into the page, then finds the block's hash in Ethereum's own block-hash history (EIP-2935) or in the beacon state. In the beacon state it reaches blocks from Capella on through `block_roots` and `historical_summaries`, and blocks from the Merge to Capella through `historical_roots`. Blocks before the Merge need no light client: the page finds the block's hash in EIP-7643's accumulator of every block before the Merge, whose root is a fixed hash built into the verifier. On Base, OP Mainnet, Unichain, Ink, World Chain, Soneium, and Zora it reads the chain's dispute games on Ethereum. On Arbitrum One and Robinhood Chain it reads the chain's assertions on Ethereum. It does this by running the contracts' own code over Ethereum state proven against the light client. A block newer than any claim on Ethereum is verified by the sequencer's signature, with the sequencer's key read from Ethereum the same way. On BNB Smart Chain it runs a light client of BNB's own validators (Parlia) from a starting point pinned in the page, or a newer one in `checkpoint.json`, which Willow refreshes daily, and takes each new validator set from an epoch block that two thirds of the set before it voted final. On the testnets the claims are on Sepolia, and the page reads them the same way over Sepolia state proven against Sepolia's light client, which also starts from the page or a newer start in `checkpoint.json`.

`web/verify/TRUSTLESS-ANCHOR.md` describes the block confirmation in full.

## What it trusts

- **The page's code.** Willow serves it, together with the program keys and the Ethereum checkpoint compiled into it, and the Sepolia and BNB Smart Chain starting points, built in or newer in `checkpoint.json` beside the page. This repository exists so you do not have to take that code on trust: build it and compare (see below).
- **The program keys.** SP1's two in `zkvm/logset-guest/vkeys/`, and OpenVM's in `zkvm/logset-guest-openvm/pins/` (`program-keys.txt` names the leaf and L1 program keys, copied from the record of the build that made them; `vk/` holds the verifying keys an OpenVM proof is checked under, which must be for exactly those keys). A log proof shows that the programs with these keys ran over the blocks and produced the proven logs. No SP1 key vouches for an OpenVM proof, or the reverse. The programs' source is not in this repository. You can enter other SP1 keys under Advanced; the OpenVM keys are always the pinned ones.
- **The dependencies** pinned in each crate's `Cargo.lock`, among them SP1's verifier (`sp1-verifier` 6.8.1), OpenVM's (`openvm-verify-stark-host` 2.0.2, from willow-network/openvm at commit `c016b6d8c5e1b749cb23c47e1f836cf4332e1831`: version 2.0.2 plus one change so it builds for wasm32, proposed upstream at https://github.com/openvm-org/openvm/pull/3178), and `bls12_381`.
- **Ethereum's sync committee.** 342 of its 512 members must be honest, the usual light-client assumption. The checkpoint must be under 30 days old. The page refuses an older one.
- **Sepolia's sync committee**, for the testnets. 342 of its 512 members must be honest, the usual light-client assumption. The starting point must be under 30 days old. The page refuses an older one.
- **BNB Smart Chain's validators.** Two thirds of each validator set must be honest. The page counts a block final when two thirds of the validators voted it, or a block after it, final, and takes each new set from an epoch block the set before it made final that way. The starting point comes from Willow, in the page or in `checkpoint.json`, and must be under 7 days old, BNB's unbonding period. The page refuses an older one. A block sealed by one validator and not yet voted final rests on that validator. The page uses such a block only while it is under 10 minutes old.
- **On an L2, what the level shown names.** "Settled" rests on the chain's fault-proof system and its governance, as any withdrawal from the chain does. "Posted" rests on the chain's proposer or validator. "Signed by the sequencer" rests on the sequencer.

## What it does not trust

- Willow's RPC, the beacon APIs, and the RPC nodes Willow reads from. Whatever they send is verified, and data that fails is shown as not verified.
- Willow's plain responses. They are compared with the proven ones.

## What it does not verify, and says so

- When Willow cannot send the data to confirm an L2 block, the page asks two public nodes of that chain and shows their response in gold, never as settled. A node you enter under Advanced is asked instead.
- An L2's own finality (Base treats a block as final about 20 minutes in, for example). The page says it does not verify that.
- Token symbols and decimals. They come from `eth_call` on public nodes.
- Which block is "latest". That comes from Willow.
- A few fields the proofs do not cover, which the page names: `gasUsedForL1` and `timeboosted` on Arbitrum One and Robinhood Chain, and a single receipt's `logs[].logIndex` from the third transaction in a block on.

## Build

You need:

- rustup. `rust-toolchain.toml` pins the compiler: `nightly-2026-04-12` (rustc 1.97.0-nightly, commit bf4fbfb7a 2026-04-11).
- The wasm32 target: `rustup target add wasm32-unknown-unknown --toolchain nightly-2026-04-12`
- wasm-bindgen-cli 0.2.129: `cargo install wasm-bindgen-cli --version 0.2.129 --locked`
- clang and llvm-ar with the wasm32 target, for the C code (zstd) in OpenVM's verifier: Ubuntu 24.04's `clang-18` and `llvm-18` (18.1.3-1ubuntu1) make the served bytes, since the OpenVM module names the clang that compiled it. Another clang with a wasm32 target builds the page with other bytes: Homebrew's `llvm` on a Mac, for example (Apple's clang has no wasm32 target). `WASM_CC` and `WASM_AR` name others.
- python3 (3.11 or later), rsync, curl, and shasum.
- No wasm-opt on your PATH. The served files are built without it, and `build.sh` runs it when it finds it.

Then:

```
web/verify/build.sh
```

The page lands in `web/verify-dist/`, with `SHA256SUMS.txt` listing every file. Each build reads a fresh Ethereum checkpoint from two independent beacon APIs (`web/verify/checkpoint.py`). `CHECKPOINT=<root>:<slot>` pins one instead.

The build makes the same bytes from any directory. It doesn't make the same bytes on every kind of machine, because cargo hashes the build machine's type into crates with build scripts or proc macros. Willow builds the page on x86-64 Ubuntu 24.04 with its clang 18.1.3, and `web/verify/Dockerfile` is that machine with the pinned toolchain.

## Verify the served page

```
web/verify/reproduce.sh
```

It reads the Ethereum checkpoint the served page carries, builds this source with it, then downloads every file `https://api.willow.tech/verify/` serves and verifies that each one is byte for byte the file the build made. On x86-64 Linux with the toolchain above, Ubuntu's clang 18.1.3 included, it builds directly. Anywhere else it builds in Docker, in the `Dockerfile`'s image (`DOCKER=1` uses Docker on Linux too). The first Docker run takes several minutes, and longer on Apple silicon, where Docker emulates x86-64.

The `main` branch is the page Willow serves. Each deploy updates it.

## Tests

- `cargo test` in `crates/willow-rpc` runs the Rust client on recorded proofs, tampered every way; `cargo test` in `zkvm/logset-guest/query-verify` and `zkvm/logset-guest/anchor-verify` runs those crates' own checks.
- `node web/verify/test/items.mjs web/verify-dist`, `node web/verify/test/real-proofs.mjs web/verify-dist web/verify/test/logs/eth-26226600 $(cat web/verify/test/logs/eth-26226600/LEAF_KEY)` (SP1) and `node web/verify/test/real-openvm.mjs web/verify-dist` (OpenVM) run the built wasm under Node.
- The browser tests drive the built page in a headless browser against a mock server (`web/verify/test/mock_rpc.py`). They need [Playwright](https://playwright.dev) and a build with the tests' saved checkpoint:

  ```
  CHECKPOINT=test web/verify/build.sh
  PLAYWRIGHT=<dir of the playwright package> node web/verify/test/browser.mjs web/verify-dist web/verify/test/logs/eth-26226600 $(cat web/verify/test/logs/eth-26226600/LEAF_KEY)
  ```

  `BROWSER=webkit` or `BROWSER=firefox` picks another engine.

## Layout

The directory layout is the same as in Willow's main repository, so the build script and the paths compiled into the wasm (`/willow/zkvm/...`) are the same too. `crates/network/src/beacon/` holds two files that the beacon crate reads at compile time, to guard its field counts. Some comments name files that are not in this repository.

## License

MIT (`LICENSE`). The fonts in `web/verify/fonts/` are under the SIL Open Font License, with their licenses beside them. `THIRD-PARTY-NOTICES.md` credits the MIT-licensed projects parts of this code follow, with their notices: op-revm's and Solady's FastLZ length (in `zkvm/logset-guest/blockproof/src/opfee.rs`) and helios's light client.

## Patents

Willow Core, Inc. has patents pending on methods behind Willow's proofs. The MIT license covers using, copying, changing and distributing this software. Nothing here grants a license under those patents beyond that.
