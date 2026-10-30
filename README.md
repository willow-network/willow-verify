# Willow browser verifier

This is the source of the page at https://api.willow.tech/verify/. The page verifies Willow's proofs in your browser. With this repository you can read the code that runs, build the page yourself, and confirm that the files Willow serves are the files this source builds.

## What the page verifies

Everything runs in a Web Worker, in four WebAssembly modules built from the Rust crates here.

- **Logs** (`zkvm/logset-guest/query-wasm`, with `zkvm/logset-guest-openvm/verify-wasm`). Willow answers a log query with zkVM proofs that each block's logs are complete, plus proofs over its log index. Each chain's proofs are SP1 proofs up to its switch to OpenVM, and OpenVM 2.0.2 proofs from it. The page tells the two apart by each proof's first bytes, verifies every proof against its own system's program keys pinned in this repository, works out from your filter which logs must be in the answer, and compares Willow's plain `eth_getLogs` answer with the proven logs. SP1's and OpenVM's verifiers cannot be built into one module (they pin different versions of a shared crate), so OpenVM proofs are verified in a second module, loaded only for an answer that has one. The log module decides which key each proof is checked under; the OpenVM module checks it and verifies under the pinned OpenVM programs only.
- **Transactions and receipts** (`zkvm/logset-guest/items-wasm`). Merkle-Patricia proofs: the header hashes to the block hash, each transaction or receipt is under the header's roots, and the block and transaction are the ones you asked for. The page then compares Willow's plain answers with the proven ones, field by field. No zero-knowledge proof is involved here.
- **Blocks** (`zkvm/logset-guest/anchor-wasm`, built from `zkvm/logset-guest/anchor-verify`). Every proof above ends at a block hash, and the page confirms that hash is block N of its chain. On Ethereum it runs Ethereum's light client from a checkpoint built into the page, then finds the block's hash in Ethereum's own block-hash history (EIP-2935) or in the beacon state. On Base, OP Mainnet, Unichain, and Ink it reads the chain's dispute games on Ethereum. On Arbitrum One and Robinhood Chain it reads the chain's assertions on Ethereum. It does this by running the contracts' own code over Ethereum state proven against the light client. A block newer than any claim on Ethereum is verified by the sequencer's signature, with the sequencer's key read from Ethereum the same way.

`web/verify/TRUSTLESS-ANCHOR.md` describes the block confirmation in full.

## What it trusts

- **The page's code.** Willow serves it, together with the program keys and the Ethereum checkpoint compiled into it. This repository exists so you do not have to take that code on trust: build it and compare (see below).
- **The program keys.** SP1's two in `zkvm/logset-guest/vkeys/`, and OpenVM's in `zkvm/logset-guest-openvm/pins/` (`program-keys.txt` names the leaf and L1 program keys, copied from the record of the mint that built them; `vk/` holds the verifying keys an OpenVM proof is checked under, which must be for exactly those keys). A log proof shows that the programs with these keys ran over the blocks and produced the proven logs. No SP1 key vouches for an OpenVM proof, or the reverse. The programs' source is not in this repository. You can enter other SP1 keys under Advanced; the OpenVM keys are always the pinned ones.
- **The dependencies** pinned in each crate's `Cargo.lock`, among them SP1's verifier (`sp1-verifier` 6.8.1), OpenVM's (`openvm-verify-stark-host` 2.0.2, from willow-network/openvm at commit `c016b6d8c5e1b749cb23c47e1f836cf4332e1831`: version 2.0.2 plus one change so it builds for wasm32, proposed upstream at https://github.com/openvm-org/openvm/pull/3178), and `bls12_381`.
- **Ethereum's sync committee.** 342 of its 512 members must be honest, the usual light-client assumption. The checkpoint must be under 30 days old. The page refuses an older one.
- **On an L2, what the level shown names.** "Settled" rests on the chain's fault-proof system and its governance, as any withdrawal from the chain does. "Posted" rests on the chain's proposer or validator. "Signed by the sequencer" rests on the sequencer.

## What it does not trust

- Willow's RPC, the beacon APIs, and the RPC nodes Willow reads from. Whatever they send is verified, and data that fails is shown as not verified.
- Willow's plain answers. They are compared with the proven ones.

## What it does not verify, and says so

- When Willow cannot send the data to confirm an L2 block, the page asks two public nodes of that chain and shows their answer in gold, never as settled. A node you enter under Advanced is asked instead.
- An L2's own finality (Base treats a block as final about 20 minutes in, for example). The page says it does not verify that.
- Token symbols and decimals. They come from `eth_call` on public nodes.
- Which block is "latest". That comes from Willow.
- A few fields the proofs do not cover, which the page names: `gasUsedForL1` and `timeboosted` on Arbitrum One and Robinhood Chain, and a single receipt's `logs[].logIndex` from the third transaction in a block on.

## Build

You need:

- rustup. `rust-toolchain.toml` pins the compiler: `nightly-2026-04-12` (rustc 1.97.0-nightly, commit bf4fbfb7a 2026-04-11).
- The wasm32 target: `rustup target add wasm32-unknown-unknown --toolchain nightly-2026-04-12`
- wasm-bindgen-cli 0.2.129: `cargo install wasm-bindgen-cli --version 0.2.129 --locked`
- clang and llvm-ar with the wasm32 target, for the C code (zstd) in OpenVM's verifier: Debian's `clang` and `llvm` packages, or Homebrew's `llvm` on a Mac (Apple's clang has no wasm32 target). `WASM_CC` and `WASM_AR` name others.
- python3 (3.11 or later), rsync, curl, and shasum.
- No wasm-opt on your PATH. The served files are built without it, and `build.sh` runs it when it finds it.

Then:

```
web/verify/build.sh
```

The page lands in `web/verify-dist/`, with `SHA256SUMS.txt` listing every file. Each build reads a fresh Ethereum checkpoint from two independent beacon APIs (`web/verify/checkpoint.py`). `CHECKPOINT=<root>:<slot>` pins one instead.

The build makes the same bytes from any directory. It doesn't make the same bytes on every kind of machine, because cargo hashes the build machine's type into crates with build scripts or proc macros. Willow builds the page on x86-64 Linux, and `web/verify/Dockerfile` is that machine with the pinned toolchain.

## Verify the served page

```
web/verify/reproduce.sh
```

It reads the Ethereum checkpoint the served page carries, builds this source with it, then downloads every file `https://api.willow.tech/verify/` serves and verifies that each one is byte for byte the file the build made. On x86-64 Linux with the toolchain above it builds directly. Anywhere else it builds in Docker, in the `Dockerfile`'s image (`DOCKER=1` uses Docker on Linux too). The first Docker run takes several minutes, and longer on Apple silicon, where Docker emulates x86-64.

The `main` branch is the page Willow serves. Each deploy updates it.

## Tests

- `cargo test` in `zkvm/logset-guest/anchor-verify` and `zkvm/logset-guest/items-wasm` runs those verifiers on real mainnet data saved in `web/verify/test/`, tampered every way. `cargo test` in `zkvm/logset-guest/query-wasm` runs the log verifier on synthetic chains, and in `zkvm/logset-guest-openvm/verify` the OpenVM check on the real OpenVM proofs in `web/verify/test/logs/`.
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

MIT (`LICENSE`). The fonts in `web/verify/fonts/` are under the SIL Open Font License, with their licenses beside them.
