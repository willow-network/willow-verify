# willow-rpc

A verifying client for Willow's RPC, for Rust. For every request it fetches a proof from Willow,
verifies it on your machine with the same Rust verifiers the verify page at
<https://api.willow.tech/verify/> runs, ties the block to its chain with a light client, and
returns the standard JSON-RPC response built from the proven data alone. A value that fails
verification never reaches your code.

It is the Rust counterpart of [`@willow-network/rpc`](https://www.npmjs.com/package/@willow-network/rpc):
the same methods, the same refusals and the same error codes.

```sh
cargo add willow-rpc
```

## With alloy

`WillowTransport` is an alloy transport, so every call a provider makes goes through the client:

```rust
use alloy_provider::{Provider, ProviderBuilder};
use alloy_rpc_client::RpcClient;
use willow_rpc::{alloy::WillowTransport, Config};

let transport = WillowTransport::new(Config::new(8453).key("<your API key>"))?;
let provider = ProviderBuilder::new().connect_client(RpcClient::new(transport, false));

let block = provider.get_block_number().await?; // the newest block the light client anchored
let balance = provider.get_balance(address).await?; // verified against Base's state root
```

The `alloy` feature (on by default) is for alloy 2. For alloy 1, turn on `alloy-v1` and use
`willow_rpc::alloy::v1::WillowTransport`.

## Without alloy

```rust
use serde_json::json;
use willow_rpc::{Config, Verifier};

let willow = Verifier::new(Config::new(1).key("<your API key>"))?;
let block = willow.request("eth_getBlockByNumber", json!(["latest", false])).await?;
```

`request` takes a method and its params and returns the verified `result`, or an `Error`. The
client needs a tokio runtime. Clones of a `Verifier` share its caches and light clients, and the
work in flight: a request joins an identical one already on its way, on whatever runtime that one
runs, and HTTP connections are pooled. So when one `Verifier` serves several runtimes, a
current-thread runtime that isn't being driven (outside its `block_on`) can hold up the others'
requests that joined its work or reuse a connection it opened (up to the request timeout), and one
that is dropped fails the requests that joined its work once (-32603). Runtimes that run on their
own should each have their own `Verifier`.

Without a key the client uses Willow's public path, which serves blocks, transactions, receipts
and logs. State, calls and traces need a key.

## Not verified in this release

This release doesn't verify `eth_gasPrice`, `eth_maxPriorityFeePerGas`, `eth_blobBaseFee`,
`eth_feeHistory` or `eth_estimateGas` on any chain, nor `eth_call` on Robinhood Chain (4663). Their
verifiers (the `willow-fee-v1` fee formula, the gas-estimate search and Robinhood Chain's EVM) are
held back from crates.io for now. Those requests are unverifiable (`no-verifier`, -32099), and under
`Unverifiable::Flag` the server's value is returned, flagged. Every other method is verified as
described below, and the verify page at <https://api.willow.tech/verify/> verifies all of them.
alloy's default fillers ask for gas estimates and fees when they fill a transaction, so with the
default `Unverifiable::Throw` give `send_transaction` its gas and fees yourself, or fill them from
another provider.

## What it trusts

Each response is verified in two steps:

1. **The proof.** The block header must hash to the block hash, and every value in the response
   must be proven against that header: transactions and receipts against their trie roots,
   account and storage values against the state root, calls by running them here over proven
   state, and logs over more than 8 blocks by Willow's SP1 and OpenVM log proofs.
2. **The anchor.** The block hash must be the chain's. A light client verifies it from a fixed
   starting point (the checkpoint): Ethereum's sync committee for Ethereum and the rollups that
   settle on it, Sepolia's for the testnets, and BNB Smart Chain's validators for BNB. A rollup
   block is anchored by its sequencer's signature, by a dispute game or an assertion on its L1,
   at the level you ask for.

What's left to trust:

- **The validators that sign.** A false Ethereum chain needs the signatures of 342 of the 512
  members of the current sync committee (Sepolia's, for the testnets), and a false BNB Smart Chain
  the votes of two thirds of its validator set.
- **Ethereum's history before the Merge**, through the root of EIP-7643's accumulator, which the
  client pins.
- **The light clients' starting points** (below).
- **On the rollups, the party behind each level**: the sequencer for `signed`, the proposer and its
  bond for `committed`, and for `final` the chain's fault proofs and governance.
- **This code**: its EVMs and the network upgrades they know, and the rules that
  render standard responses from proven bytes. A network upgrade that changes these needs a newer
  release.

Beyond the starting points it may read from Willow's `checkpoint.json` (below), the client doesn't
trust Willow's servers, Willow's providers or public nodes.

## The checkpoint

Each release ships a checkpoint in `checkpoint.json`: a finalized Ethereum beacon block root and
its slot, plus Sepolia's and BNB's starting points. The light client refuses a starting point
more than 30 days old, so a shipped checkpoint has a lifetime:

- Until it is 29 days old, the client starts from it. `willow_rpc::checkpoint::shipped_checkpoint_expires()`
  says when that ends.
- After that, the client reads Willow's published `checkpoint.json`
  (`https://api.willow.tech/verify/checkpoint.json`) and takes its checkpoint
  only when independent beacon APIs confirm the same root at that slot: at least one must
  confirm it and none may disagree. If that fails, every anchored response is unverifiable
  (`anchor-stale-checkpoint`, -32099) until you update the crate or pass a checkpoint.
- Every release is cut with a checkpoint at most 10 days old.

To start from a checkpoint you chose (say, one your own beacon node gave you):

```rust
use willow_rpc::{Checkpoint, Config};

let config = Config::new(1).checkpoint(Checkpoint {
    root: "0x…".into(), // a finalized beacon block root
    slot: 15_630_336,
});
```

Sepolia's and BNB's starting points follow the same pattern without the beacon APIs: once the
shipped Sepolia start is 29 days old, or the shipped BNB start a day old (a client that starts cold
walks every BNB epoch since its start, and the light client refuses a BNB start over 7 days old),
the client takes a newer one from Willow's `checkpoint.json` as published. For the testnets and BNB
Smart Chain that start is then trusted; pass your own with `sepolia_checkpoint` and `bnb_start` to
avoid it.

`sepolia_checkpoint`, `bnb_start`, `checkpoint_url` and `beacon_apis` set the others. The light
clients' verified committees are kept in `~/.cache/willow-rpc/` (the same files the npm client
uses in Node) and verified again from the checkpoint when they are read;
`storage(StorageChoice::None)` keeps nothing.

## Anchoring

The light clients are the default (`Anchoring::LightClient`). Two other choices, as the
`willow-logs-verify` CLI offers them:

- `Anchoring::Rpc(url)`: compare each proven block's hash with a node you trust, and take
  `latest`, `finalized` and `safe` from it.
- `Anchoring::None`: verify each proof against its own header and leave the block hash
  unverified. A proof then shows that a value is in a block, not that the block is the chain's.
  Every event says so (`anchored_by`).

## Chains

Ethereum (1); anchored on Ethereum: Base (8453), OP Mainnet (10), Unichain (130), Ink (57073),
Soneium (1868), Zora (7777777), World Chain (480), Arbitrum One (42161), Robinhood Chain (4663);
anchored on Sepolia: OP Sepolia (11155420), Base Sepolia (84532), Unichain Sepolia (1301), Ink
Sepolia (763373), Arbitrum Sepolia (421614), Robinhood Chain testnet (46630); anchored on its own
validators: BNB Smart Chain (56).

## Methods

| Method | Proof |
| :--- | :--- |
| `eth_getBlockByNumber`, `ByHash`, `eth_getBlockTransactionCountBy*` | `willow_getBlockProof`, cached by hash |
| `eth_blockNumber` | the anchored head: `willow_getBlockAnchor ["latest"]` |
| `eth_getBalance`, `eth_getTransactionCount`, `eth_getCode`, `eth_getStorageAt`, `eth_getProof` | `willow_getStateProof`; reads of one account at one block go as one proof |
| `eth_call` (Ethereum, Unichain; Robinhood Chain's not in this release) | `willow_getCallProof`, with the call's state and block overrides laid over the proven state here |
| `eth_estimateGas` | not in this release (the verify page: `willow_getEstimateGasProof`) |
| `eth_gasPrice`, `eth_maxPriorityFeePerGas`, `eth_blobBaseFee`, `eth_feeHistory` | not in this release (the verify page: `willow_getFeeProof`, `willow_getFeeHistoryProof`, `willow-fee-v1`) |
| `eth_getTransactionByHash`, `ByBlock*AndIndex`, `eth_getTransactionReceipt`, `eth_getBlockReceipts` | transaction and receipt proofs; an unknown hash is `null` |
| `eth_getLogs` | up to 8 blocks: `willow_getBlockReceiptsProof` per block; more: `willow_getLogsProof` (plans with log proofs) |
| `debug_traceTransaction`, `debug_traceBlockByNumber`, `debug_traceBlockByHash`, `trace_block`, `trace_transaction`, `trace_replayBlockTransactions`, `trace_replayTransaction` (Ethereum, Unichain) | `willow_getTraceProof`: the block run here over the proven parent state, each transaction verified against its receipt |
| `debug_traceCall`, `trace_call` (Ethereum) | `willow_getTraceProof`: the call run here over a call proof's state |
| `trace_filter` (Ethereum, Unichain) | `trace_block` proofs per block (at most 16), filtered here |
| `eth_sendRawTransaction` | the hash is computed here from the bytes and must be the server's |
| `eth_newFilter`, `eth_newBlockFilter`, `eth_getFilterChanges`, `eth_getFilterLogs`, `eth_uninstallFilter` | served here over the verified methods; a block filter returns at most 2,000 blocks a poll (`BLOCK_FILTER_MAX`) and goes on from there at the next |
| `eth_chainId`, `net_version`, `net_listening`, `net_peerCount`, `web3_clientVersion`, `eth_syncing`, `eth_accounts` | served here; one `eth_chainId` at start must name the configured chain |

`latest` (and `pending`, or no block) is the newest block the light client anchored, not the
server's newest. `finalized` on Ethereum is the light client's finalized block. A block newer
than the newest anchored one waits for its anchor up to `anchor_wait` (36 s), then the request is
unverifiable (`anchor-waiting`).

Log proofs: the SP1 ones are verified in-process. OpenVM's verifier can't share a dependency graph
with SP1's, so OpenVM-proven ranges are verified by the `willow-logs-verify-openvm` helper,
looked for next to your executable and on `PATH` (`Config::openvm_verifier` names another). It is
built from source, from `zkvm/logset-guest-openvm/verify` in
[willow-verify](https://github.com/willow-network/willow-verify). Without it, an OpenVM-proven
range is unverifiable (`no-verifier`); a helper that fails on a proof fails that check (`logs`)
and is started again for the next. The `logs` feature (on by default) can be turned off to
leave SP1's verifier out; log ranges over 8 blocks are then unverifiable too.

## Errors

`Error::Verification` is code -32098: a proof failed, so the server lied or Willow has a bug. It is
never passed through. `Error::Unverifiable` is code -32099: a value the client could not verify,
under `Unverifiable::Throw` (the default); with `Unverifiable::Flag` the server's value is returned
and the event says why. For a log filter that value is the server's `eth_getLogs` over the same
blocks; a block filter has no server value, so a block proof it can't get raises even under `Flag`.
`Error::Rpc` is any other code: a server refusal passed on as sent, a verified revert (code 3, with
its data) the client computed itself, or -32603 when the server couldn't be reached or the request
was cut off (a timeout, a runtime that ended). Decide by `error.kind()`, never by the message.

| `kind` | Code | Meaning |
| :--- | :--- | :--- |
| `header`, `wrong-block`, `wrong-chain`, `transactions`, `receipts`, `withdrawals`, `uncles`, `missing`, `wrong-item`, `broken-link`, `wrong-range`, `body`, `malformed` | -32098 | a block or item proof failed |
| `account`, `slot`, `code`, `history`, `missing-read`, `outcome`, `limits` | -32098 | a state or call proof failed |
| `logs` | -32098 | a log proof failed |
| `anchor-rejected`, `anchor-malformed`, `anchor-mismatch`, `anchor-head` | -32098 | the anchor failed, proved another hash, or `latest`/`finalized` came by the wrong path |
| `batch`, `tx-hash`, `internal` | -32098 | a batch response without (or with twice) an item, a transaction hash that isn't the bytes', a verifier that failed to run |
| `tag` | -32099 | `safe` or `finalized` on a rollup: the block verifies, the tag's meaning does not |
| `method` | -32099 | a value from a method or call with no proof |
| `not-on-plan` | -32099 | the proof method is not served for this key or path |
| `logs-range`, `logs-timeout`, `logs-job-failed` | -32099 | `eth_getLogs` over more than 8 blocks with no log proof |
| `no-verifier` | -32099 | a verifier this build or machine doesn't have (the methods held back from this release, OpenVM's helper, or the `logs` feature off) |
| `anchor-level`, `anchor-waiting`, `anchor-stale-head`, `anchor-not-covered`, `anchor-declined`, `anchor-disputed`, `anchor-unreachable`, `anchor-stale-checkpoint`, `anchor-unknown-fork`, `anchor-refused` | -32099 | the block can't be anchored now |

Absences and refusals pass through as they are: `null` receipts and transactions, every JSON-RPC
error, and provider errors from `eth_sendRawTransaction`.

`Config::on_verification` gets one `Event` per response: its status (`Verified`, `Local`,
`Absent`, `Refused`, `Unverifiable`, `Failed`), block, anchor level, why it was unverifiable or
failed, and the fields the proofs don't cover, left out of the response (for example Nitro's
`gasUsedForL1`).

## License

MIT
