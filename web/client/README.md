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
| `chainId` | required | 1, 8453, 10, 130, 57073, 42161 or 4663 |
| `key` | `public` | the API key; the public path serves blocks, transactions, receipts, logs and fees, not state |
| `url` | `https://api.willow.tech/v1/rpc/{chainId}/{key}` | |
| `lightClientUrl` | `url` with chain 1 | Ethereum's route, for the light client |
| `minLevel` | `signed` | `signed`, `committed` or `final`: the lowest anchor a response may rest on |
| `unverifiable` | `throw` | `flag` returns a value that can't be verified, marked in `onVerification` |
| `onVerification` | none | one event per response (below) |
| `headMaxAgeMs` | Ethereum 12000, L2 2000 | how long the newest block counts before it is anchored again |
| `maxHeadAgeSec` | 600 | how old the newest block Willow anchors may be |
| `checkpoint` | the shipped one | the light client's start `{root, slot}` |
| `checkpointUrl`, `beaconApis` | Willow's `checkpoint.json`; two beacon APIs in Node | a newer start once the shipped one is 29 days old, confirmed by the beacon APIs |
| `storage` | localStorage, or `~/.cache/willow-rpc/` in Node | where the light client's committees are kept (verified again when read); `false` for none |
| `worker` | browsers true, Node false | verify in a Web Worker |
| `batchWindowMs` | 5 | requests this close together go out as one batch |
| `verifiers` | the package's wasm builds | the verifier modules (below), injected (tests, other builds) |
| `logsTimeoutMs` | 120000 | how long `eth_getLogs` waits for Willow to prove blocks |

**What each method is verified by:**

| Method | Proof |
| :--- | :--- |
| `eth_getBlockByNumber`, `ByHash`, `eth_getBlockTransactionCountBy*` | `willow_getBlockProof`, cached by hash |
| `eth_blockNumber` | the anchored head: `willow_getBlockAnchor ["latest"]` |
| `eth_getBalance`, `eth_getTransactionCount`, `eth_getCode`, `eth_getStorageAt`, `eth_getProof` | `willow_getStateProof`; reads of one account at one block go as one proof; code is cached by code hash |
| `eth_call`, `eth_estimateGas` (Ethereum) | `willow_getCallProof`, `willow_getEstimateGasProof` |
| `eth_gasPrice`, `eth_maxPriorityFeePerGas`, `eth_blobBaseFee` | one `willow_getFeeProof` at the head (formula `willow-fee-v1`) |
| `eth_feeHistory` | `willow_getFeeHistoryProof`, in proofs of 16 blocks when rewards are asked for |
| `eth_getTransactionByHash`, `ByBlock*AndIndex`, `eth_getTransactionReceipt`, `eth_getBlockReceipts` | the shipped transaction and receipt proofs; an unknown hash is `null` |
| `eth_getLogs` | up to 8 blocks: `willow_getBlockReceiptsProof` per block, the range anchored from its newest block through parent hashes; more: `willow_getLogsProof` (plans with log proofs), its proof jobs waited for |
| `eth_sendRawTransaction` | none: the hash is computed here from the bytes and must be the server's |
| `eth_newFilter`, `eth_newBlockFilter`, `eth_getFilterChanges`, `eth_getFilterLogs`, `eth_uninstallFilter` | served here over the verified methods |
| `eth_chainId`, `net_version`, `net_listening`, `net_peerCount`, `web3_clientVersion`, `eth_syncing`, `eth_accounts` | served here; one `eth_chainId` at start must name the configured chain |

Every proven block is anchored on Ethereum by the light client (one for all seven chains), at
`minLevel` or above. `latest` is the anchored head; `finalized` on Ethereum is the light client's
finalized block; `safe` on Ethereum is refused, as the server refuses it. The pending nonce is
the proven one, or one past this client's own last sent nonce, never a server's.

**Errors.** Every error has `code`, `message` and `data`. Decide by `data.kind`, never by the
message.

- **`WillowVerificationError`, code -32098.** A proof failed: the server lied or Willow has a
  bug. Never passed through, also under `unverifiable: "flag"`.
- **`WillowUnverifiableError`, code -32099.** A value the client can't verify, under
  `unverifiable: "throw"`. With `flag`, the server's value is returned and the event says why.
- **`WillowRpcError`**, any other code: a server refusal passed through as sent, or a verified
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
| `method` | -32099 | a value from a method with no proof (overrides, eth_call on an L2 that serves one, any method not above) |
| `not-on-plan` | -32099 | the proof method is not served for this key or path (-32601) |
| `logs-range`, `logs-timeout`, `logs-job-failed` | -32099 | `eth_getLogs` over more than 8 blocks with no log proof |
| `no-verifier` | -32099 | a verifier module is missing from this build |
| `anchor-level` | -32099 | the block is anchored below `minLevel`, or an L2 `latest` above `signed` |
| `anchor-stale-head`, `anchor-not-covered`, `anchor-declined`, `anchor-disputed`, `anchor-unreachable`, `anchor-stale-checkpoint`, `anchor-unknown-fork`, `anchor-refused` | -32099 | the block can't be anchored now; a replayed or stale head lands here, as no forged value can |

Absences and refusals pass through as they are: `null` receipts and transactions, every JSON-RPC
error (L2 `eth_call` and `eth_estimateGas` are -32601), and provider errors from
`eth_sendRawTransaction`.

**Events** (`onVerification`): `{method, status, chainId, block?, level?, kind?, detail?, flags?,
unproven?, ms}`. `status` is `verified`, `local`, `absent`, `refused`, `unverifiable` (passed under
`flag`) or `failed`. `unproven` lists the fields the proofs do not cover, left out of the response
(for example Nitro's `gasUsedForL1`, and a log proof's block and transaction hashes).

## Tests

`npm test` runs the client against a mock server built from recorded fixtures (`test/fixtures/`;
sources in `test/fixtures/SOURCES.json`) with mock
verifiers (`test/support/mock-verifiers.ts`). They verify the chain, the header against its hash
and number, the block asked for, the fee headers' links and a stand-in anchor signature for real,
and the rest by equality with the fixtures, reporting the real modules' failure kinds.

With `WILLOW_VERIFY_DIST` set to a verify page build, the real anchor module runs on the saved anchor
fixtures, and the real items module on the shipped transaction and receipt proofs.

`test/lying.test.ts` is the lying server: a proxy that applies one mutation from a table
(`test/support/mutations.ts`) to a response. It also runs against the live server:
`WILLOW_LIVE_URL=https://api.willow.tech WILLOW_KEY=… WILLOW_VERIFY_DIST=…`.

`npm run build` writes `dist/esm` and `dist/cjs`. The wasm modules go in `wasm/` as
`<name>.js` and `<name>_bg.wasm` (`items`, `state`, `anchor`, `verifier`, `openvm`), from the verify page's build
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

- `request(kind, target)` → `{ok, method, params}`; `verify(response)` → `{ok, report, proven}`
  with `report.block` `{number, hash, timestamp, parentHash}`; `compare(method, response)` →
  `{ok, result: {exact, differ}}` against another server's standard response.
- Failure kinds: `invalid-request`, `malformed`, `unsupported`,
  `wrong-chain`, `header`, `wrong-block`, `limits`, `account`, `slot`, `code`, `history`,
  `missing-read`, `outcome`, `estimate`, `algorithm`, `internal`.

**Anchor module** (`anchor.js`): Ethereum's light client and the L2 anchors; `sequencer` results
carry `anchorTime`.
**Logs module** (`verifier.js`) and **OpenVM module** (`openvm.js`): log proofs.
