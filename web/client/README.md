# @willow-network/rpc

A drop-in verifying client for Willow's RPC: a viem transport and an ethers v6 provider. For every
request it fetches a proof, verifies it, and returns the standard JSON-RPC response built from the
proven data alone.

```ts
import { createPublicClient } from 'viem';
import { mainnet } from 'viem/chains';
import { willowTransport } from '@willow-network/rpc/viem';

const client = createPublicClient({ chain: mainnet, transport: willowTransport({ chainId: 1, key }) });
await client.getBalance({ address }); // verified against Ethereum's validators, or it throws
```

```ts
import { WillowProvider } from '@willow-network/rpc/ethers';
const provider = new WillowProvider({ chainId: 8453, key });
```

```ts
import { createVerifier } from '@willow-network/rpc';
const willow = createVerifier({ chainId: 1, key });
await willow.request({ method: 'eth_getBlockByNumber', params: ['latest', false] }); // EIP-1193
```

ESM and CommonJS; Node 18 and later, and evergreen browsers (the verifiers in a Web Worker).

## Public API

`createVerifier(options)` returns `{ chainId, request({method, params}), close() }`.
`willowTransport(options | verifier)` is a viem `custom()` transport; viem's own retries default
to 0, so a failed verification is never asked again. `new WillowProvider(options)` extends ethers'
`JsonRpcApiProvider`, with `staticNetwork` set to the chain and batches of up to 100.

**Options:**

| Option | Default | |
| :--- | :--- | :--- |
| `chainId` | required | 1; 8453, 10, 130, 57073, 1868, 7777777, 480, 42161, 4663 (anchored on Ethereum); 42220, 196, 5000, 747474 (anchored on Ethereum by their claims; no newest block yet); 11155111, 11155420, 84532, 1301, 763373, 421614, 46630 (on Sepolia); 56 (on BNB's validators); 137 (Polygon PoS's checkpoints on Ethereum: transactions, receipts and logs, not block hashes or state) |
| `key` | `public` | the API key; the public path serves blocks, transactions, receipts, logs and fees, not state |
| `url` | `https://api.willow.tech/v1/rpc/{chainId}/{key}` | |
| `lightClientUrl` | `url` with chain 1 | Ethereum's route, for the light client |
| `minLevel` | `signed` | `signed`, `committed` or `final`: the lowest anchor a response may rest on |
| `unverifiable` | `throw` | `flag` returns a value that can't be verified, marked in `onVerification` |
| `onVerification` | none | one event per response (below) |
| `headMaxAgeMs` | Ethereum 12000, L2 2000 | how long the newest block counts before it is anchored again |
| `maxHeadAgeSec` | 600 | how old the newest block Willow anchors may be |
| `checkpoint` | the shipped one | the light client's start `{root, slot}` |
| `checkpointUrl`, `beaconApis` | Willow's `checkpoint.json`; two beacon APIs in Node | a newer start once the shipped one is 29 days old: at least one beacon API confirms it and none disagrees (one that fails or has no header for that slot is skipped) |
| `sepoliaCheckpoint` | the shipped one | Sepolia's light client's start `{root, slot}` (Sepolia and the testnets); `checkpoint.json`'s `sepolia` once the shipped one is 29 days old |
| `berachainStart`, `cometApis` | the shipped one; two CometBFT APIs on Berachain (browsers too: a Content-Security-Policy must allow `berachain-beacon-rpc.publicnode.com` and `rpc.berachain.com`) | Berachain's light client's start `{height, hash, time}`, a CometBFT header; `checkpoint.json`'s `berachain` once the shipped one is 6 hours old (the verifier refuses a day), when at least one CometBFT API has that header hash and none another. A client running past its start's life takes `checkpoint.json`'s newer one |
| `bnbStart` | the shipped one | BNB's light client's start `{number, hash, time}`, an epoch block; `checkpoint.json`'s `bnb` once the shipped one is a day old (the verifier refuses 7; a cold client walks every epoch since its start) |
| `storage` | localStorage, or `~/.cache/willow-rpc/` in Node | where the light client's committees are kept (verified again when read); `false` for none |
| `worker` | browsers true, Node false | verify in a Web Worker |
| `batchWindowMs` | 5 | requests this close together go out as one batch |
| `verifiers` | the package's wasm builds | the verifier modules (below), injected (tests, other builds) |
| `logsTimeoutMs` | 120000 | how long `eth_getLogs` waits for Willow to prove blocks |
| `anchorWaitMs` | 36000 | how long a request waits for the anchor of a block newer than the newest one anchored (below) |

**What each method is verified by:**

| Method | Proof |
| :--- | :--- |
| `eth_getBlockByNumber`, `ByHash`, `eth_getBlockTransactionCountBy*` | `willow_getBlockProof`, cached by hash |
| `eth_blockNumber` | the anchored head: `willow_getBlockAnchor ["latest"]` |
| `eth_getBalance`, `eth_getTransactionCount`, `eth_getCode`, `eth_getStorageAt`, `eth_getProof` | `willow_getStateProof`; reads of one account at one block go as one proof; code is cached by code hash |
| `eth_call`, `eth_estimateGas` (Ethereum; OP stack: Unichain, OP Mainnet, Ink, World Chain, Soneium; Arbitrum: Robinhood Chain, Arbitrum One) | `willow_getCallProof`, `willow_getEstimateGasProof`, with the call's state and block overrides: the client lays them over the proven state itself; Robinhood Chain's estimate is the search Nitro runs (`nitro-v3.12.2-estimate`) |
| `eth_gasPrice`, `eth_maxPriorityFeePerGas`, `eth_blobBaseFee` | one `willow_getFeeProof` at the head (formula `willow-fee-v1`) |
| `eth_feeHistory` | `willow_getFeeHistoryProof`, in proofs of 16 blocks when rewards are asked for |
| `eth_getTransactionByHash`, `ByBlock*AndIndex`, `eth_getTransactionReceipt`, `eth_getBlockReceipts` | the shipped transaction and receipt proofs; an unknown hash is `null` |
| `eth_getLogs` | up to 8 blocks: `willow_getBlockReceiptsProof` per block, the range anchored from its newest block through parent hashes; more: `willow_getLogsProof` (plans with log proofs), its proof jobs waited for |
| `debug_traceTransaction`, `debug_traceBlockByNumber`, `debug_traceBlockByHash`, `trace_block`, `trace_transaction`, `trace_replayBlockTransactions`, `trace_replayTransaction` (Ethereum, Unichain) | `willow_getTraceProof [method, params]`: the block run here over the proven parent state, every transaction checked against its receipt, the trace written by the server's own tracers (`trace-verify`) |
| `debug_traceCall`, `trace_call` (Ethereum) | `willow_getTraceProof [method, params]`: a call proof's witness at the block the client resolved; the call run here as `eth_call` runs it, under the caller's overrides, traced by the same tracers |
| `trace_filter` (Ethereum, Unichain) | `willow_getTraceProof` of `trace_block` per block (at most 16), filtered here, the range anchored from its newest block through parent hashes |
| `eth_sendRawTransaction` | none: the hash is computed here from the bytes and must be the server's |
| `eth_newFilter`, `eth_newBlockFilter`, `eth_getFilterChanges`, `eth_getFilterLogs`, `eth_uninstallFilter` | served here over the verified methods; a block filter returns at most 2,000 blocks a poll (`BLOCK_FILTER_MAX`), oldest first, the next poll going on from there |
| `eth_chainId`, `net_version`, `net_listening`, `net_peerCount`, `web3_clientVersion`, `eth_syncing`, `eth_accounts` | served here; one `eth_chainId` at start must name the configured chain |

Every proven block is anchored, at `minLevel` or above, by one of three light clients in the anchor
module: Ethereum's (Ethereum and the L2s that settle on it), Sepolia's (Sepolia itself, as Ethereum, and the
testnets; its data on the chain's own route) and BNB's validator set (BNB Smart Chain, on its own route; its `latest` is its
newest finalized block, so it is served at any `minLevel`; a block only sealed counts as `signed` for
10 minutes). `latest` (and `pending`, or no block) is the anchored head, as Helios reads it: the
newest block the light client has seen signed, not the server's newest; `finalized` on Ethereum is
the light client's finalized block; `safe` on Ethereum is refused, as the server refuses it. The
pending nonce is the proven one, or one past this client's own last sent nonce, never a server's.

**The newest block.** The light client signs Ethereum block N only in slot N+1, and Willow's RPC
serves state at its own node's newest block, one slot ahead of it. A request that names such a
block by number waits for its anchor: while `willow_getBlockAnchor [n]` says `waiting`, it is
asked again every `waitMs.anchor` (else the server's `retryAfterSeconds`, else 4 s) until
`anchorWaitMs` (default 36 s, three slots) has passed; then the request fails as unverifiable
(-32099, `anchor-waiting`).

**Errors.** Every error has `code`, `message` and `data`. Decide by `data.kind`, never by the
message.

- **`WillowVerificationError`, code -32098.** A proof failed: the server lied or Willow has a
  bug. Never passed through, also under `unverifiable: "flag"`.
- **`WillowUnverifiableError`, code -32099.** A value the client can't verify, under
  `unverifiable: "throw"`. With `flag`, the server's value is returned and the event says why.
  The server has no filters: a log filter's logs fall back to the server's `eth_getLogs` over
  the same blocks, and anything else a filter can't verify (a block filter's blocks) is raised
  under either setting.
- **`WillowRpcError`**, any other code: a server refusal passed through as sent, the client's own
  refusal of bad params or options (-32602, such as a `chainId` over 32 bits), or a verified
  revert (code 3, with its data) the client computed itself.

| `data.kind` | Error | Meaning |
| :--- | :--- | :--- |
| `header`, `wrong-block`, `wrong-chain`, `transactions`, `receipts`, `withdrawals`, `uncles`, `missing`, `wrong-item`, `broken-link`, `wrong-range`, `body`, `unknown-formula`, `malformed` | -32098 | a block, fee or item proof failed (the items module's `fault`) |
| `account`, `slot`, `code`, `history`, `missing-read`, `outcome`, `estimate`, `algorithm`, `limits` | -32098 | a state, call or estimate proof failed (the state module's `kind`) |
| `logs` | -32098 | a log proof failed |
| `rejected` | -32098 | a proof failed in a verifier build without `fault` |
| `anchor-rejected`, `anchor-malformed` | -32098 | the anchor's data failed verification |
| `anchor-mismatch` | -32098 | Ethereum proves another hash for that block |
| `anchor-head` | -32098 | `latest` or `finalized` anchored by the wrong path |
| `batch` | -32098 | a batch response without, or with twice, an item asked for |
| `tx-hash` | -32098 | the server's transaction hash is not the hash of the bytes |
| `internal` | -32098 | a verifier failed to run (a trap): fail closed |
| `tag` | -32099 | `safe` or `finalized` on an L2: the block verifies, the tag's meaning does not |
| `method` | -32099 | a value from a method with no proof (a call or overrides the verifier doesn't run, such as `movePrecompileToAddress`; `eth_call` on an L2 without call proofs; any method not above) |
| `not-on-plan` | -32099 | the proof method is not served for this key or path (-32601) |
| `logs-range`, `logs-timeout`, `logs-job-failed` | -32099 | `eth_getLogs` over more than 8 blocks with no log proof |
| `no-verifier` | -32099 | a verifier module is missing from this build |
| `anchor-level` | -32099 | the block is anchored below `minLevel`, or an L2 `latest` above `signed` |
| `anchor-covers` | -32099 | the block's anchor covers its transactions and receipts roots, not its hash (Polygon PoS): blocks, state, calls, fees, ranges by log proof and `latest` |
| `anchor-waiting` | -32099 | the block's anchor was still waiting for the light client after `anchorWaitMs` |
| `anchor-stale-head`, `anchor-not-covered`, `anchor-declined`, `anchor-disputed`, `anchor-unreachable`, `anchor-stale-checkpoint`, `anchor-unknown-fork`, `anchor-refused` | -32099 | the block can't be anchored now; a replayed or stale head lands here, as no forged value can |

Absences and refusals pass through as they are: `null` receipts and transactions, every JSON-RPC
error (`eth_call` and `eth_estimateGas` where Willow doesn't run them are -32601), and provider errors from
`eth_sendRawTransaction`.

**Events** (`onVerification`): `{method, status, chainId, block?, level?, kind?, detail?, flags?,
unproven?, ms}`. `status` is `verified`, `local`, `absent`, `refused`, `unverifiable` (passed under
`flag`) or `failed`. `unproven` lists the fields the proofs do not cover, left out of the response
(for example Nitro's `gasUsedForL1`, a log proof's block and transaction hashes, and on Polygon PoS
every `blockHash`: its checkpoints prove a block's transactions and receipts, not its hash).

## Stellar

```ts
import { createStellarVerifier } from '@willow-network/rpc/stellar';
const stellar = createStellarVerifier({ key });
await stellar.request({ method: 'getEvents', params: { startLedger, filters } }); // verified, or it throws
```

`createStellarVerifier(options)` returns `{ request({method, params}), close() }`: stellar-rpc's
JSON-RPC methods, params by name, each result as SDF's stellar-rpc writes it (base64 XDR) and
built from proven data. It talks to `https://api.willow.tech/v1/rpc/stellar/{key}` and verifies
in the `stellar` wasm module, in the calling thread. Stellar needs a key: it has no keyless path.

**How a ledger is verified.** For each range of ledgers a response covers, the client asks
`willow_getLedgerProof {startLedger, endLedger, transactions}` for bundles of at most 64 ledgers
(16 with transaction sets), cached by ledger, and the module checks, with no ZK proof, what a Stellar light client checks:

1. **Consensus.** A quorum of the pinned Tier-1 quorum set signed SCP `EXTERNALIZE` for the slot
   after the bundle's last ledger, whose value names that ledger's hash as its previous ledger.
   The envelopes come from SDF's history archive.
2. **Header chain.** Each header hashes to its ledger's hash and names the one before it.
3. **Results.** Each ledger's `TransactionResultSet` hashes to the header's `txSetResultHash`:
   every transaction's hash, result, fee and status, in application order.
4. **Soroban events.** One `InvokeHostFunctionSuccessPreImage` per successful `InvokeHostFunction`
   hashes to the hash its result commits: its contract events and return value.
5. **Transactions** (when asked): the transaction set hashes to the header's `txSetHash`, and its
   envelopes are the results' transactions, one for one.

**Trust.** The quorum set every Tier-1 validator declares, compiled into the module: 10
organisations of 3 validators, 7 of 10 needed, each organisation counting when 2 of its 3 sign
(sha256 `040355b7…841d`, `pinnedQuorumSet()`). An anchor whose signers declare another set
reports its hash as `nextQuorumSetHash`; after checking a handoff, pass the new set as
`trustedQuorumSet` (base64 `ScpQuorumSet` XDR).

**What each method returns:**

| Method | Verified by | Left out (Stellar's header does not commit to it) |
| :--- | :--- | :--- |
| `getLatestLedger` | the newest ledger the client can anchor (option `latest`, below) | `metadataXdr` |
| `getLedgers` | a bundle per 64 ledgers of the page; `hash`, `sequence`, `ledgerCloseTime`, `headerXdr` must equal the server's | `metadataXdr` |
| `getTransaction` | its ledger's bundle with the transaction set: the transaction at `applicationOrder` must have the hash asked; `status`, `resultXdr`, `envelopeXdr`, `feeBump`, `ledger`, `createdAt` must equal the server's; a Soroban success's `events.contractEventsXdr` too, and its proven `returnValueXdr` is added | `resultMetaXdr`, `diagnosticEventsXdr`, `events.transactionEventsXdr` (fee events), a classic transaction's `events.contractEventsXdr` |
| `getTransactions` | as `getTransaction`, for every ledger the page covers: the page must be every transaction from its start to its cursor, none left out or added | as `getTransaction` |
| `getEvents` | every ledger the page covers (from its start to its cursor): every proven Soroban event matching the filters must be in the response and equal, field for field | fee events, classic operations' events (CAP-67), and any event not at a successful invocation's place: left out of the response (`dropped` in the event) |
| `getHealth`, `getNetwork`, `getVersionInfo` | passed through as the server's own description (`status: "unproven"`); `getNetwork`'s passphrase must be pubnet's | |
| `getLedgerEntries`, `simulateTransaction`, `sendTransaction` | refused (-32601): a single ledger entry can't be proven (`bucketListHash` commits to whole buckets), and the server does not serve them | |
| any other method | unverifiable (`method`) | |

Every response also passes through the server's own range (`latestLedger`, `oldestLedger` and
their close times), listed in `unproven`. With `unverifiable: "flag"`, the left-out fields and
events are kept and flagged (`unprovable`). `xdrFormat: "json"` is unverifiable.

**Completeness.** A page covers from its start (`startLedger`, or just after `pagination.cursor`)
to its returned cursor, as stellar-rpc defines it: a full page ends at its last item, otherwise at
the end of the last ledger scanned. Everything proven in that span must be in the page. The client
never verifies more than `maxScanLedgers` ledgers for one page: it asks getEvents for no more (an
`endLedger`, when the request names a `startLedger`), and cuts a longer page at a ledger's end,
moving its cursor back. Pagination stays correct: the next page starts where the verified one ends.

**The newest ledgers.** SCP messages reach the history archive once per 64-ledger checkpoint
(about every 6 minutes), and a ledger is anchored by the next slot's, so the newest ledgers,
typically 1 to 6 minutes' worth, can't be anchored yet. Then:

- `getLedgers`, `getTransactions` and `getEvents` end the page at the newest anchored ledger, its
  cursor there, and the next page picks up from it (under `flag`: the rest is kept, flagged
  `pending`). A page with nothing anchored is empty, its cursor unchanged.
- `getLatestLedger` returns the newest anchored ledger (`latest: "anchored"`, the default); the
  event's `serverLatest` and `behind` say how far it trails the server's. `latest: "tip"` returns
  the server's latest once it is anchored.
- `getTransaction` (and `latest: "tip"`) wait for the ledger's anchor, asking every `waitMs`, up to
  `anchorWaitMs`; then the value is unverifiable (`pending`).

A bundle without an anchor is never read. A NOT_FOUND transaction is unverifiable (`not-found`):
absence over the server's retention window is not provable.

**Options:**

| Option | Default | |
| :--- | :--- | :--- |
| `key` | `public` | the API key |
| `url` | `https://api.willow.tech/v1/rpc/stellar/{key}` | |
| `unverifiable` | `throw` | `flag` returns what can't be verified, marked in `onVerification` |
| `onVerification` | none | one event per response (below) |
| `latest` | `anchored` | getLatestLedger: the newest anchored ledger, or (`tip`) the server's once anchored |
| `anchorWaitMs` | 420000 (`throw`), 0 (`flag`) | how long a ledger newer than the newest anchored one is waited for |
| `waitMs` | 15000 | between asks while waiting |
| `maxScanLedgers` | 128 | most ledgers one page is verified over |
| `trustedQuorumSet` | the pinned Tier-1 set | base64 `ScpQuorumSet` XDR, after a handoff |
| `verifiers` | the package's wasm builds | `{stellar}`, injected (tests, other builds) |
| `fetch`, `batchWindowMs` | global fetch, 5 | |

**Events:** `{method, status, network: "stellar", ledger?: {sequence, hash}, anchor?, ledgers?:
{from, to}, serverLatest?, behind?, kind?, detail?, flags?, unproven?, dropped?, ms}`. `anchor`
is the newest bundle's: `{ledger, hash, quorumSetHash, nextQuorumSetHash, signedEntries,
totalEntries, signers, source}`. Errors are the same three classes as above; `data.kind`:

| `data.kind` | Error | Meaning |
| :--- | :--- | :--- |
| `ledger-proof` | -32098 | a bundle failed: SCP signatures, header chain, results, events or transaction set |
| `ledger-mismatch` | -32098 | the server's ledger (hash, close time, header) is not the proven one |
| `transaction-mismatch` | -32098 | a transaction field is not the proven one, or the transaction is not where the server says |
| `transactions` | -32098 | a getTransactions page leaves out or adds a transaction |
| `event-missing`, `event-extra`, `event-mismatch` | -32098 | a proven event matching the filters is left out; an event at a Soroban invocation's place is not proven or does not match the filters; an event differs |
| `page` | -32098 | a page out of order, outside its own cursors, or malformed |
| `wrong-network` | -32098 | getNetwork names another network |
| `internal` | -32098 | the verifier failed to run: fail closed |
| `pending` | -32099 | the ledger is not anchored yet (after `anchorWaitMs`) |
| `not-found` | -32099 | getTransaction NOT_FOUND |
| `unprovable` | flag only | a field or event the header does not commit to, kept |
| `proof-refused`, `no-verifier`, `method`, `json-format` | -32099 | no proof served, no `stellar` module, a method without proof, `xdrFormat: "json"` |

## Tests

`npm test` runs the client against a mock server built from recorded fixtures (`test/fixtures/`)
with mock
verifiers (`test/support/mock-verifiers.ts`). They verify the chain, the header against its hash
and number, the block asked for, the fee headers' links and a stand-in anchor signature for real,
and the rest by equality with the fixtures, reporting the real modules' failure kinds.

With `WILLOW_VERIFY_DIST` set to a verify page build, the real anchor module runs on the saved anchor
fixtures, and the real items module on the shipped transaction and receipt proofs.

`test/lying.test.ts` is the lying server: a proxy that applies one mutation from a table
(`test/support/mutations.ts`) to a response. It also runs against the live server:
`WILLOW_LIVE_URL=https://api.willow.tech WILLOW_KEY=… WILLOW_VERIFY_DIST=…`.

`test/stellar.test.ts` runs the Stellar client against a mock stellar-rpc over a made-up chain
with a fake Stellar module (`test/support/stellar-world.ts`), lying server included.
`test/stellar.real.test.ts` runs the real module against a live endpoint, 256 ledgers and the lying
server on real data: `STELLAR_WASM_DIR=<dir with stellar.js and stellar_bg.wasm, or a page build>
STELLAR_RPC_URL=https://api.willow.tech/v1/rpc/stellar/<key>`.

`npm run build` writes `dist/esm` and `dist/cjs`. The wasm modules go in `wasm/` as
`<name>.js` and `<name>_bg.wasm` (`items`, `state`, `anchor`, `verifier`, `openvm`, `stellar`), from the verify page's build
(`web/verify/build.sh`).

## Wasm verifier APIs

The client verifies in the same wasm modules as the verify page (`web/verify/build.sh`). Their JS
APIs are typed in `src/verifiers.ts`. Every method returns JSON text with `ok`, never throws a verdict, and is
decided by `kind` (and `fault`), never by `message`.

**Items module** (`items.js`, `willow-items-verify-wasm`). `new ItemVerifier(chainId)`, then
`request(kind, target)` and `verify(response)`. Block and fee kinds:

| kind | method | `target` | `verify` → `proven` |
| :--- | :--- | :--- | :--- |
| `block` | `willow_getBlockProof` | a block: number, 32-byte hash or `latest` | `eth_getBlockByNumber(n, false)` |
| `fees` | `willow_getFeeProof` | a block | `{eth_gasPrice, eth_maxPriorityFeePerGas, eth_blobBaseFee, baseFeePerGas, nextBaseFeePerGas, blockNumber, blockHash, formula}` |
| `feeHistory` | `willow_getFeeHistoryProof` | JSON text `[blockCount, newestBlock, rewardPercentiles]` | the `eth_feeHistory` result |

- `block(full)`: the last verified block proof rendered with or without every transaction
  object, `{ok, proven, unproven}`. Senders are recovered only when it is called with `true`.
- Every failure may carry `fault` (`header`, `transactions`,
  `withdrawals`, `uncles`, `wrong-block`, `wrong-chain`, `broken-link`, `wrong-range`, `body`, …).
  `kind` is one of (`rejected`, `malformed-input`, `invalid-request`, `internal`).
- `report.block` `{number, hash, timestamp, parentHash}` is the block to anchor: the proof's block,
  H for `fees`, the newest for `feeHistory`. `parentHash` (the header's, for every kind of both
  modules) lets the client anchor a range of blocks through parent hashes instead of one anchor
  each.
- With percentiles, a `feeHistory` proof covers at most 16 blocks on chains with tips; a larger
  count is `invalid-request`.

**State module** (`state.js`, `willow-state-verify-wasm`, `zkvm/logset-guest/state-wasm`,
over the `state-verify` crate). `new StateVerifier(chainId)`:

| kind | method | `target` (JSON text of the proof method's params) | `verify` → `proven` |
| :--- | :--- | :--- | :--- |
| `state` | `willow_getStateProof` | `[address, storageKeys, block, {"code": bool}]` | `{eth_getBalance, eth_getTransactionCount, eth_getCode, eth_getStorageAt[], eth_getProof}` |
| `call` | `willow_getCallProof` | `[call, block]` | `{result}` or `{error}` |
| `estimate` | `willow_getEstimateGasProof` | `[call, block]` | `{result}` or `{error}` |
| `trace` | `willow_getTraceProof` | `[method, params]` (a trace method a proof covers) | `{result}`: the trace method's response (`trace_verify::verify_trace`); the report adds `traced` and `algorithm` (`willow-trace-v1`) |

- `request(kind, target)` → `{ok, method, params}`; `verify(response)` → `{ok, report, proven}`
  with `report.block` `{number, hash, timestamp, parentHash}`; `compare(method, response)` →
  `{ok, result: {exact, differ}}` against another server's standard response.
- Failure kinds: `invalid-request`, `malformed`, `unsupported`,
  `wrong-chain`, `header`, `wrong-block`, `limits`, `account`, `slot`, `code`, `history`,
  `missing-read`, `outcome`, `estimate`, `algorithm`, `internal`.

**Anchor module** (`anchor.js`): Ethereum's light client and the L2 anchors; `sequencer` results
carry `anchorTime`.
**Logs module** (`verifier.js`) and **OpenVM module** (`openvm.js`): log proofs.

**Stellar module** (`stellar.js`, `willow-stellar-verify-wasm`, `crates/stellar-verify-wasm`, over
`crates/stellar-rpc-verify`). Functions, no class:

- `verifyLedgerBundle(response, trustedQuorumSet?)`: a `willow_getLedgerProof` response body (or
  the bundle alone) → `{ok, verified: {network, anchor, ledgers}}`, each ledger as
  `stellar-rpc-verify`'s `verified_json` writes it (`sequence`, `hash`, `previousHash`,
  `closeTime`, `protocolVersion`, `headerXdr`, `headerHistoryXdr`, `transactionCount`,
  `transactions`, `events`) plus `invocations: [{tx, op, eventsXdr}]`, every successful
  `InvokeHostFunction`'s place and proven `ContractEvent`s. Failure kinds: `pending` (no anchor
  yet), `invalid`, `invalid-request` (the trusted set).
- `matchEvents(events, filters)` → `{ok, matches: boolean[]}`: getEvents filters, by the code the
  server filters with; `invalid-request` for filters it can't read.
- `pinnedQuorumSet()` → `{ok, xdr, hash, network, maxLedgers}`.

## License

MIT. `THIRD-PARTY-NOTICES.md` credits the MIT-licensed projects parts of the verifiers follow, with
their notices; `THIRD-PARTY-LICENSES.md` holds the licenses of the third-party code compiled into
`wasm/`.

## Patents

Willow Core, Inc. has patents pending on methods behind Willow's proofs. The MIT license covers using,
copying, changing and distributing this software. Nothing here grants a license under those patents
beyond that.
