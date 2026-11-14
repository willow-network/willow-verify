/**
 * I6: the JS APIs of the wasm verifiers, as `@willow-network/rpc` and the verify page call them
 * (notes/verify-every-response-build-2026-11-11.md 4.1 and 6.1). Frozen 11/11 by W7; W5 builds
 * the modules to it.
 *
 * Every module is a wasm-bindgen 0.2.129 `--target web` build, as web/verify/build.sh makes them:
 * a glue module (`<name>.js`) exporting `default init({module_or_path})`, `initSync({module})` and
 * the classes below, and its `<name>_bg.wasm`. Names: `verifier` (logs), `openvm`, `items`,
 * `anchor`, and the new `state`.
 *
 * The rules every class keeps, unchanged from the shipped modules:
 * - Every method that returns `string` returns JSON text: `{"ok": true, ...}`, or a failure
 *   `{"ok": false, "kind", "message", ...}`. No method throws a verdict. Anything thrown is the
 *   runtime failing (a wasm trap, out of memory), never a verdict on a response.
 * - `response` arguments are JSON-RPC response bodies exactly as the server sent them. The wasm
 *   parses them; callers never re-serialise server data.
 * - Callers decide by `kind` (and `fault`) alone, never by `message`, which can carry text the
 *   server chose.
 *
 * What is new against the shipped modules:
 * - `ItemVerifier` gains the kinds `block`, `fees` and `feeHistory`, a `block(full)` method, and a
 *   `fault` field on failures.
 * - A new module, `willow-state-verify-wasm` (`state.js`, zkvm/logset-guest/state-wasm), exports
 *   `StateVerifier` over the `state-verify` crate.
 * - `Anchor.verify` results on the `sequencer` path carry `anchorTime` (W4, I5).
 */

/** JSON text, as every verifier method returns it. */
export type JsonText = string;

/** A failed call. `kind` is the module's own (listed per class below). */
export interface Failure<K extends string = string> {
  ok: false;
  kind: K;
  message: string;
}

/** What `request` returns: the JSON-RPC method and params to send, exactly. */
export interface Requested {
  ok: true;
  method: string;
  params: unknown[];
}

/** A block a proof names, proven: the block the caller anchors. */
export interface ProvenBlock {
  /** `0x` quantity. */
  number: string;
  /** `0x` + 64 hex digits, lowercase. */
  hash: string;
  /** The header's timestamp, in seconds (a JSON number). */
  timestamp: number;
  /**
   * The header's parentHash (added 11/11 after the freeze, additive). The client anchors a range
   * from its newest block down through parent hashes, with no anchor per block. Optional: a build
   * without it costs the client one anchor per block.
   */
  parentHash?: string;
}

// ---------------------------------------------------------------------------------------------
// Items module (`items.js`, willow-items-verify-wasm, zkvm/logset-guest/items-wasm)
// ---------------------------------------------------------------------------------------------

/**
 * Proof kinds of `ItemVerifier.request`. The first three ship today; `block`, `fees` and
 * `feeHistory` are new (W5, over W1's `Subject::Block`, `fees::verify_fees` and
 * `fees::verify_fee_history`).
 *
 * | kind            | method sent                     | `target` text                                   |
 * | :-------------- | :------------------------------ | :---------------------------------------------- |
 * | `transaction`   | `willow_getTransactionProof`    | a transaction hash, or `<block>:<index>`        |
 * | `receipt`       | `willow_getReceiptProof`        | a transaction hash, or `<block>:<index>`        |
 * | `blockReceipts` | `willow_getBlockReceiptsProof`  | a block                                         |
 * | `block`         | `willow_getBlockProof`          | a block                                         |
 * | `fees`          | `willow_getFeeProof`            | a block                                         |
 * | `feeHistory`    | `willow_getFeeHistoryProof`     | JSON text: `[blockCount, newestBlock, rewardPercentiles]` |
 *
 * "A block" is the shipped `parse_block` text: a decimal or `0x` number, a 32-byte `0x` hash, or
 * `latest`. `request` returns it as `block_param` writes it: a number as a `0x` quantity, a hash
 * as the plain 32-byte hex string (the server's `block_id` reads it as a hash, the same as
 * EIP-1898 `{"blockHash"}`), `latest` as `"latest"`.
 *
 * `feeHistory`'s target is eth_feeHistory's own params as JSON text:
 * - `blockCount`: a `0x` quantity, a decimal string or a JSON number, at least 1. Above 1,024 it
 *   is truncated to 1,024, as geth does (`FeeHistoryRequest::count`).
 * - `newestBlock`: a block, as a JSON string.
 * - `rewardPercentiles`: JSON numbers, at most 100, each in [0, 100], strictly increasing
 *   (`FeeHistoryRequest::check`). With any percentile, on a chain with tips (not Nitro), the
 *   proof covers at most 16 blocks (`MAX_HISTORY_WITH_REWARDS`): a larger count is
 *   `invalid-request`, and the client splits the request into proofs of 16.
 * - `params` come back as `[blockCount as a 0x quantity, newestBlock as written, percentiles]`,
 *   the form W1's fixtures were recorded with (`["0x4", "0x3574133", [10, 50, 90]]`).
 */
export type ItemKind = 'transaction' | 'receipt' | 'blockReceipts' | 'block' | 'fees' | 'feeHistory';

/**
 * `ItemVerifier` failure kinds, unchanged: `rejected` (a verification failed), `malformed-input` (the
 * response can't be read), `invalid-request` (the target can't be read; from `request`),
 * `internal` (used out of order).
 */
export type ItemFailureKind = 'rejected' | 'malformed-input' | 'invalid-request' | 'internal';

/**
 * W1's `Fault::as_str()` (zkvm/logset-guest/blockproof/src/fault.rs), what failed verification.
 * New on every `ItemVerifier` failure whose error carries one (`fault::fault`), for every kind of
 * proof, the shipped three included. The client reports it as `data.kind`.
 */
export type ItemFault =
  | 'malformed'
  | 'unknown-formula'
  | 'wrong-chain'
  | 'header'
  | 'wrong-block'
  | 'wrong-item'
  | 'transactions'
  | 'receipts'
  | 'withdrawals'
  | 'uncles'
  | 'missing'
  | 'broken-link'
  | 'wrong-range'
  | 'body';

export interface ItemFailure extends Failure<ItemFailureKind> {
  /** What failed verification, when blockproof names it. */
  fault?: ItemFault;
}

/** The `report` of every passing `verify`. */
export interface ItemReport {
  /** The proof method verified. */
  method: string;
  chainId: number;
  /** The block to anchor: the proof's block; for `fees` the block named (H); for `feeHistory` the newest. */
  block: ProvenBlock;
  /** Fields the proof does not cover, left out of `proven` (render.rs `Rendered.unproven`). */
  unproven: string[];
  provenTransactions?: number;
  provenReceipts?: number;
  transactionIndex?: string;
  transactionHash?: string | null;
  transactions?: number;
  /** `fees` and `feeHistory`: the formula verified (`willow-fee-v1`). */
  formula?: string;
}

/** `fees` → `proven`: W1's `Fees::json()`, exactly (fixtures `fees/*.fee.json` → `verified`). */
export interface FeesProven {
  formula: string;
  blockNumber: string;
  blockHash: string;
  eth_gasPrice: string;
  eth_maxPriorityFeePerGas: string;
  /** `null` where the chain's headers carry no blob gas (Nitro). */
  eth_blobBaseFee: string | null;
  baseFeePerGas: string | null;
  nextBaseFeePerGas: string | null;
}

/** `feeHistory` → `proven`: W1's `FeeHistory::json()`, the eth_feeHistory result. */
export interface FeeHistoryProven {
  oldestBlock: string;
  /** Present when percentiles were asked for. */
  reward?: string[][];
  baseFeePerGas: string[];
  gasUsedRatio: number[];
  baseFeePerBlobGas?: string[];
  blobGasUsedRatio?: number[];
}

/**
 * What a passing `verify` returns, by kind:
 * - `transaction`, `receipt`, `blockReceipts` (shipped): `proven` is the standard method's
 *   result; `provenTransactions` (blockReceipts: every transaction object), `provenTransaction`
 *   and `provenTransactionUnproven` (receipt: its transaction) as today.
 * - `block`: `proven` is `eth_getBlockByNumber(n, false)` (W1 `Verified::block(false)`), without
 *   the fields in `report.unproven`. `block(true)` renders the full form on demand.
 * - `fees`: `proven` is {@link FeesProven}.
 * - `feeHistory`: `proven` is {@link FeeHistoryProven}.
 */
export interface ItemVerified {
  ok: true;
  report: ItemReport;
  proven: unknown;
  provenTransactions?: unknown[];
  provenTransaction?: unknown;
  provenTransactionUnproven?: string[];
}

/** `ItemVerifier.block(full)`: the last verified block proof rendered, with what it leaves out. */
export interface BlockRendered {
  ok: true;
  /** `eth_getBlockByNumber(n, full)`; with `full`, every `from` recovered from its signature. */
  proven: unknown;
  /** As W1's `Rendered.unproven`, with `transactions[].<field>` entries when `full`. */
  unproven: string[];
}

/**
 * `new ItemVerifier(chainId)`; `request(kind, target)`, then `verify(response)` of that proof
 * method's response body, verified against that request (never the other way round).
 */
export interface ItemVerifier {
  /** `Requested` or `ItemFailure` (`invalid-request`, or `internal` for an unknown kind). */
  request(kind: ItemKind, target: string): JsonText;
  /** `ItemVerified` or `ItemFailure`. */
  verify(response: string): JsonText;
  /** Shipped: `which` is `answer` or `transactions`; `{ok: true, result}` or `ItemFailure`. */
  compare(which: 'answer' | 'transactions', response: string): JsonText;
  /**
   * New. The last passing `verify` of a `block` proof rendered with (`full`) or without every
   * transaction object: `BlockRendered`, or `ItemFailure` `internal` when the last `verify` was
   * not a passing block proof. Lazy, so a block's senders are recovered only when asked for.
   */
  block(full: boolean): JsonText;
  free(): void;
}

export interface ItemVerifierConstructor {
  new (chainId: number): ItemVerifier;
}

// ---------------------------------------------------------------------------------------------
// State module (`state.js`, willow-state-verify-wasm, zkvm/logset-guest/state-wasm): new
// ---------------------------------------------------------------------------------------------

/**
 * `StateVerifier.request` kinds. `target` is JSON text of the proof method's own params, which
 * W2's `verify_state`, `verify_call` and `verify_estimate` take as they are:
 *
 * | kind       | method sent                   | `target` (JSON text)                                   |
 * | :--------- | :---------------------------- | :----------------------------------------------------- |
 * | `state`    | `willow_getStateProof`        | `[address, storageKeys, block, {"code": bool}]` (the last may be left out) |
 * | `call`     | `willow_getCallProof`         | `[callObject, block]`                                  |
 * | `estimate` | `willow_getEstimateGasProof`  | `[callObject, block]`                                  |
 * | `trace`    | `willow_getTraceProof`        | `[method, params]`: a trace method and its own params   |
 *
 * `block` is any form W2's `At::parse` reads: a `0x` number, a tag, a 32-byte hash, or EIP-1898
 * `{"blockHash"}` / `{"blockNumber"}`. At most 256 storage keys (`MAX_PROOF_SLOTS`). `call` and
 * `estimate` take the caller's state and block overrides (a third and fourth param) on chains 1,
 * 130 and 4663 (else `unsupported`). `request` returns `params` as given, once they parse. `trace`
 * covers `debug_traceTransaction`, `debug_traceBlockByNumber`, `debug_traceBlockByHash`,
 * `trace_block`, `trace_transaction`, `trace_replayBlockTransactions` and
 * `trace_replayTransaction` on chains 1 and 130, and `debug_traceCall` and `trace_call` on chain 1
 * (trace-verify's `request::PROVEN`); `trace_filter` as one request, other chains and the tracers
 * a chain's trace methods don't run are `unsupported`.
 */
export type StateKind = 'state' | 'call' | 'estimate' | 'trace';

/** W2's `Kind::as_str()` (zkvm/logset-guest/state-verify/src/lib.rs), unchanged. */
export type StateFailureKind =
  | 'invalid-request'
  | 'malformed'
  | 'unsupported'
  | 'wrong-chain'
  | 'header'
  | 'wrong-block'
  | 'limits'
  | 'account'
  | 'slot'
  | 'code'
  | 'history'
  | 'missing-read'
  | 'outcome'
  | 'estimate'
  | 'algorithm'
  | 'internal'
  // `trace` only (trace-verify's `Kind::as_str()`):
  | 'parent'
  | 'transactions'
  | 'receipts'
  | 'disagrees'
  | 'too-large'
  | 'no-trace';

export interface StateReport {
  method: string;
  chainId: number;
  /** The block the proof names (W2's `Named`), to anchor. */
  block: ProvenBlock;
  /**
   * `estimate`: the algorithm verified (`geth-v1.17.10-estimate`; on Robinhood Chain
   * `nitro-v3.12.2-estimate`); `trace`: `willow-trace-v1`.
   */
  algorithm?: string;
  /** `trace`: the trace method proven. */
  traced?: string;
}

/**
 * `state` → `proven`: W2's `ProvenState.responses`, exactly (fixtures `state/*.json` → `expect`):
 * `eth_getStorageAt` holds one 32-byte value per key asked, in order; `eth_getCode` is `null`
 * unless the code was asked for (`"0x"` for an account without code).
 */
export interface StateProven {
  eth_getBalance: string;
  eth_getTransactionCount: string;
  eth_getCode: string | null;
  eth_getStorageAt: string[];
  eth_getProof: {
    address: string;
    balance: string;
    nonce: string;
    codeHash: string;
    storageHash: string;
    accountProof: string[];
    storageProof: { key: string; value: string; proof: string[] }[];
  };
}

/**
 * `call` and `estimate` → `proven`: the standard response's `result` or `error` member, as W2's
 * `Outcome::response()` and `Estimate::response()` build it (fixtures `call/*.json` → `expect`).
 * A revert is `{"error": {"code": 3, "message", "data"}}`; a halt, an invalid transaction or a
 * failed search is `{"error": {"code": -32000, "message"}}`.
 */
export type CallProven = { result: string } | { error: { code: number; message: string; data?: string } };

/** `trace` → `proven`: the trace method's standard `result`, from the client's own run of the block. */
export interface TraceProven {
  result: unknown;
}

export interface StateVerified {
  ok: true;
  report: StateReport;
  proven: StateProven | CallProven | TraceProven;
}

/**
 * `new StateVerifier(chainId)`; `request(kind, target)`, then `verify(response)` of that proof
 * method's response body, verified against that request.
 */
export interface StateVerifier {
  /** `Requested`, or `Failure<StateFailureKind>` (`invalid-request`, `unsupported`). */
  request(kind: StateKind, target: string): JsonText;
  /** `StateVerified`, or `Failure<StateFailureKind>`. */
  verify(response: string): JsonText;
  /**
   * Another server's standard response body against the last passing `verify`'s proven one:
   * `{"ok": true, "result": {"exact": boolean, "differ": string[]}}`, or a failure (`internal`
   * without a passing verify, `malformed` when `response` can't be read). `method` names which:
   * `eth_getBalance`, `eth_getTransactionCount`, `eth_getCode`, `eth_getProof`,
   * `eth_getStorageAt:<i>` (the i-th key asked), `eth_call`, `eth_estimateGas` or the trace method. The page and the
   * differential tests use it; the client does not.
   */
  compare(method: string, response: string): JsonText;
  free(): void;
}

export interface StateVerifierConstructor {
  new (chainId: number): StateVerifier;
}

// ---------------------------------------------------------------------------------------------
// Anchor module (`anchor.js`, willow-anchor-verify-wasm, zkvm/logset-guest/anchor-wasm)
// ---------------------------------------------------------------------------------------------

/**
 * Anchor failure kinds, unchanged (anchor-verify `Kind::as_str()`). `need-bootstrap` and
 * `need-updates` (with `from` and `count`) say what to fetch first; the others but `rejected`
 * and `malformed-input` are not verdicts on the block.
 */
export type AnchorFailureKind =
  | 'rejected'
  | 'malformed-input'
  | 'unknown-fork'
  | 'stale-checkpoint'
  | 'unreachable'
  | 'stale-head'
  | 'declined'
  | 'disputed'
  | 'not-covered'
  | 'need-bootstrap'
  | 'need-updates'
  | 'internal';

export interface AnchorFailure extends Failure<AnchorFailureKind> {
  /** `need-updates`: the first period and how many. */
  from?: number;
  count?: number;
  /**
   * `need-bootstrap` and `need-updates` of another light client than Ethereum's (#993, #994): its
   * data is fetched on the verified chain's own route, with the `*For` methods.
   */
  lightClient?: LightClientName;
}

/** The anchor's light clients besides Ethereum's: Sepolia's (the testnets) and BNB's validators. */
export type LightClientName = 'sepolia' | 'bnb';

/**
 * A passing `Anchor.verify`, by path (anchor-verify lib.rs `Anchor::verify`):
 * - Ethereum (`head`, `history`, `beacon`): `level` `signed` or `final`, `finalized`, `head`,
 *   `headTime` (the signed head's timestamp, seconds), `signers`.
 * - OP stack (`game`): `level` `committed` or `final`, `anchorBlock`, `game`, `l1Block`.
 * - Nitro (`assertion`): `level` `committed` or `final`, `anchorBlock`, `assertion`, `l1Block`.
 * - Any L2, sequencer-signed (`sequencer`): `level` `signed`, `anchorBlock`, `signer`,
 *   `l1Block`, and, from W4, `anchorTime` (the signed block's timestamp, seconds).
 */
export interface AnchorVerified {
  ok: true;
  level: 'signed' | 'committed' | 'final';
  path: 'head' | 'history' | 'beacon' | 'game' | 'assertion' | 'sequencer' | 'parlia' | 'accumulator';
  /** Block `number`'s hash, as proven. */
  hash: string;
  /** Whether `hash` is the one passed in. */
  matches: boolean;
  finalized?: number;
  head?: number;
  headTime?: number;
  signers?: number;
  anchorBlock?: number;
  anchorTime?: number;
  game?: string;
  assertion?: string;
  signer?: string;
  l1Block?: number;
  /** BNB (`parlia`): the validators who voted, of the set. */
  of?: number;
}

/**
 * `new Anchor(checkpointRoot, checkpointSlot)`, unchanged. One `Anchor` is one Ethereum light
 * client and anchors all seven chains. `now` is in seconds.
 */
export interface Anchor {
  /** `{ok: true, period}` or a failure; `saved` is what `saved()` returned, verified again. */
  restore(saved: string, now: number): JsonText;
  /** A `willow_lightClientBootstrap [checkpointRoot]` response body. */
  bootstrap(response: string, now: number): JsonText;
  /** A `willow_lightClientUpdates [from, count ≤ 16]` response body. */
  updates(response: string): JsonText;
  /** `AnchorVerified` or `AnchorFailure`, from a `willow_getBlockAnchor` response body. */
  verify(chainId: number, number: number, hash: string, response: string, now: number): JsonText;
  /** The committees to keep for next time (JSON), or `null`. */
  saved(): string;
  /** Forgets the committees, after a `need-bootstrap`. */
  reset(): void;
  /**
   * Additive (#993, #994): the other light clients. `start` sets where one starts (`sepolia`:
   * `{root, slot}`; `bnb`: `{number, hash}` of an epoch block) in place of the pin built in; the
   * rest are the methods above for it. A build before them has none.
   */
  start?(lightClient: LightClientName, start: string): JsonText;
  bootstrapFor?(lightClient: LightClientName, response: string, now: number): JsonText;
  updatesFor?(lightClient: LightClientName, response: string): JsonText;
  restoreFor?(lightClient: LightClientName, saved: string, now: number): JsonText;
  savedFor?(lightClient: LightClientName): string;
  resetFor?(lightClient: LightClientName): void;
  free(): void;
}

export interface AnchorConstructor {
  new (checkpointRoot: string, checkpointSlot: number): Anchor;
}

// ---------------------------------------------------------------------------------------------
// Logs module (`verifier.js`, willow-logs-verify-wasm, zkvm/logset-guest/query-wasm) and its
// OpenVM proof verifier (`openvm.js`): unchanged
// ---------------------------------------------------------------------------------------------

export type LogsFailureKind = 'rejected' | 'malformed-input' | 'invalid-filter' | 'invalid-keys' | 'unknown-chain' | 'internal';

/** A proven log: `eth_getLogs` JSON without block and transaction hashes (query-wasm `log_json`). */
export interface ProvenLog {
  address: string;
  topics: string[];
  data: string;
  blockNumber: string;
  transactionIndex: string;
  logIndex: string;
}

export interface LogsVerified {
  ok: true;
  report: {
    fromBlock: string;
    toBlock: string;
    logs: number;
    lastProvenBlock: { number: string; hash: string };
    [k: string]: unknown;
  };
  logs: ProvenLog[];
}

export interface LogsVerifier {
  keys(): JsonText;
  /**
   * A `willow_getLogsProof` response body against `filter` (JSON text). `getProof(id)` returns the
   * proof's bytes; `onChecked(id, error, system)` runs after each proof is verified; `checkOpenVm(key,
   * proof, publicValues)` returns `undefined` when an OpenVM proof holds. `LogsVerified` or a failure.
   */
  verify(
    response: string,
    filter: string,
    getProof: (id: string) => Uint8Array | undefined,
    onChecked: (id: string, error: string | undefined, system: string | undefined) => void,
    checkOpenVm?: (key: Uint8Array, proof: Uint8Array, publicValues: Uint8Array) => string | undefined,
  ): JsonText;
  compare(response: string): JsonText;
  free(): void;
}

export interface LogsVerifierConstructor {
  new (chainId: number, segmentKeys: string, leafKeys: string): LogsVerifier;
}

export interface OpenVmVerifier {
  /** `{ok, leaf, l1}`: the OpenVM keys it verifies under. */
  keys(): JsonText;
  check(key: Uint8Array, proof: Uint8Array, publicValues: Uint8Array): string | undefined;
  free(): void;
}

export interface OpenVmVerifierConstructor {
  new (): OpenVmVerifier;
}

// ---------------------------------------------------------------------------------------------
// The modules, as the client loads them
// ---------------------------------------------------------------------------------------------

/** A wasm-bindgen `--target web` glue module, as `import()` returns it. */
export interface GlueModule {
  default: (input?: { module_or_path: unknown }) => Promise<unknown>;
  initSync: (input: { module: BufferSource | WebAssembly.Module }) => unknown;
}

export interface ItemsModule extends GlueModule {
  ItemVerifier: ItemVerifierConstructor;
}
export interface StateModule extends GlueModule {
  StateVerifier: StateVerifierConstructor;
}
export interface AnchorModule extends GlueModule {
  Anchor: AnchorConstructor;
}
export interface LogsModule extends GlueModule {
  Verifier: LogsVerifierConstructor;
}
export interface OpenVmModule extends GlueModule {
  OpenVmVerifier: OpenVmVerifierConstructor;
}

/**
 * The verifier classes, initialised, by module. The client takes them from its own wasm builds by
 * default; `createVerifier({verifiers})` injects others (tests, a custom build). Each is loaded
 * on first use.
 */
export interface VerifierModules {
  items?: () => Promise<{ ItemVerifier: ItemVerifierConstructor }>;
  state?: () => Promise<{ StateVerifier: StateVerifierConstructor }>;
  anchor?: () => Promise<{ Anchor: AnchorConstructor }>;
  logs?: () => Promise<{ Verifier: LogsVerifierConstructor }>;
  openvm?: () => Promise<{ OpenVmVerifier: OpenVmVerifierConstructor }>;
}
