import { Anchors, atLeast, type Anchored, type Level } from './anchor.js';
import { InlineBackend, isNoVerifier, type AnchorStart, type Backend } from './backend.js';
import { BEACON_APIS, chooseCheckpoint, chooseStarts, type BnbStart, type Checkpoint } from './checkpoint.js';
import { WillowRpcError, WillowUnverifiableError, WillowVerificationError } from './errors.js';
import { hexToBytes, isAddress, isHash, keccakHex, sameHex, slotKey, toNumber, toQuantity } from './hex.js';
import { Batcher, MAX_BATCH, type Reply } from './http.js';
import { Lru } from './lru.js';
import { spawnWorker } from './platform.js';
import { defaultStorage, isNode, type KeyValueStorage } from './storage.js';
import { decodeSent } from './tx.js';
import type {
  CallProven,
  FeeHistoryProven,
  FeesProven,
  ItemKind,
  ItemVerified,
  LogsVerified,
  Requested,
  StateKind,
  StateProven,
  StateVerified,
  TraceProven,
  VerifierModules,
} from './verifiers.js';
import { packageModules } from './wasm.js';
import { WorkerBackend } from './worker-rpc.js';

export type { Level } from './anchor.js';

export interface WillowOptions {
  /**
   * The chain this client verifies: Ethereum (1); anchored on Ethereum, Base (8453), OP Mainnet
   * (10), Unichain (130), Ink (57073), Soneium (1868), Zora (7777777), World Chain (480), Arbitrum
   * One (42161), Robinhood Chain (4663); anchored on Sepolia, OP Sepolia (11155420), Base Sepolia
   * (84532), Unichain Sepolia (1301), Ink Sepolia (763373), Arbitrum Sepolia (421614), Robinhood
   * Chain testnet (46630); anchored on its own validators, BNB Smart Chain (56).
   */
  chainId: number;
  /** API key; without one, the public path, which serves blocks, transactions, receipts and logs but no state. */
  key?: string;
  /** The RPC URL; default `https://api.willow.tech/v1/rpc/{chainId}/{key}`. */
  url?: string;
  /** Ethereum's route, for the light client; default `url` with chain 1. */
  lightClientUrl?: string;
  /** The lowest anchor level a response may rest on; default `signed`. */
  minLevel?: Level;
  /** A value that can't be verified: `throw` (default) raises WillowUnverifiableError; `flag` returns it with an event. */
  unverifiable?: 'throw' | 'flag';
  /** Called once per response: what was verified, at which block and level, or why not. */
  onVerification?: (event: VerificationEvent) => void;
  /** How long the newest block counts before it is anchored again; default Ethereum 12000, L2 2000. */
  headMaxAgeMs?: number;
  /** How old the newest block Willow anchors may be, in seconds; default 600. */
  maxHeadAgeSec?: number;
  /** The light client's starting point, in place of the shipped one. */
  checkpoint?: Checkpoint;
  /** Where a newer checkpoint is read once the shipped one is too old. */
  checkpointUrl?: string;
  /** Sepolia's light client's starting point (the testnets), in place of the shipped one. */
  sepoliaCheckpoint?: Checkpoint;
  /** BNB's light client's starting point, an epoch block, in place of the shipped one. */
  bnbStart?: BnbStart;
  /** Beacon APIs that must confirm a fetched checkpoint; default two in Node, none in browsers. */
  beaconApis?: string[] | false;
  /** Where the light client's committees are kept; default localStorage, or ~/.cache/willow-rpc in Node; false: nowhere. */
  storage?: KeyValueStorage | false;
  /** Verify in a Web Worker; default true in browsers, false in Node. */
  worker?: boolean;
  /** Requests within this many ms go out as one batch; default 5. */
  batchWindowMs?: number;
  /** The verifier modules (I6); default the package's own wasm builds. */
  verifiers?: VerifierModules;
  fetch?: typeof fetch;
  /** How long eth_getLogs may wait for Willow to prove blocks; default 120000. */
  logsTimeoutMs?: number;
  /** The clock, in ms (tests). */
  now?: () => number;
  /** Waits between asks for a block newer than the signed head (default 4000, or the server's `retryAfterSeconds`) and between proof-job polls. */
  waitMs?: { anchor?: number; jobs?: number };
  /**
   * How long a request waits for the anchor of a block newer than the newest one anchored; default
   * 36000 (three Ethereum slots: the light client signs block N in slot N+1, and Willow's RPC
   * serves state at the tip, a slot ahead of it). Past it the request is unverifiable
   * (`anchor-waiting`, -32099).
   */
  anchorWaitMs?: number;
}

export interface VerificationEvent {
  method: string;
  /**
   * `verified`: built from verified proofs. `local`: served by the client itself. `absent`: a
   * null or not-found passed through. `refused`: a server error passed through. `unverifiable`:
   * passed with `flags` (under `unverifiable: "flag"`). `failed`: an error was raised.
   */
  status: 'verified' | 'local' | 'absent' | 'refused' | 'unverifiable' | 'failed';
  chainId: number;
  /** The block the response was verified at (the newest, when several). */
  block?: { number: number; hash: string };
  /** The lowest anchor level the response rests on. */
  level?: Level;
  /** Why it is unverifiable or failed (the first reason), and all reasons. */
  kind?: string;
  detail?: string;
  flags?: { kind: string; detail?: string }[];
  /** Fields the proofs do not cover, left out of the response. */
  unproven?: string[];
  ms: number;
}

export interface WillowVerifier {
  readonly chainId: number;
  /** EIP-1193 `request`: the standard JSON-RPC result, verified; or throws. */
  request(args: { method: string; params?: unknown }): Promise<unknown>;
  close(): void;
}

export const VERSION = '0.1.0';
const ORIGIN = 'https://api.willow.tech';
const RPC_PATH = /^(.*\/rpc\/)(\d+)(\/.*)?$/;
/** eth_getLogs over at most this many blocks goes by block-receipts proofs (design 1.3.12). */
export const LOGS_BY_RECEIPTS = 8;
const FEE_HISTORY_MAX = 1024;
/** Blocks per fee-history proof with rewards (W1 `MAX_HISTORY_WITH_REWARDS`). */
export const FEE_HISTORY_WITH_REWARDS = 16;
const EMPTY_CODE_HASH = '0xc5d2460186f7233c927e7db2dcc703c0e500b653ca82273b7bfad8045d85a470';
const NOT_FOUND_TX = 'not found in this store: transaction ';
const FILTER_IDLE_MS = 5 * 60 * 1000;
const NITRO = new Set([42161, 4663, 421614, 46630]);
const PROOF_DOWNLOADS = 4;
/** The chains whose traces are proven (trace-verify): Ethereum and Unichain. */
const TRACED = new Set([1, 130]);
/** The chains whose calls' traces are proven (debug_traceCall, trace_call): Ethereum. */
const CALL_TRACED = new Set([1]);
/** The trace methods that name a block, and those that name a transaction, each proven by `willow_getTraceProof`. */
const BLOCK_TRACES = new Set(['debug_traceBlockByNumber', 'debug_traceBlockByHash', 'trace_block', 'trace_replayBlockTransactions']);
const TX_TRACES = new Set(['debug_traceTransaction', 'trace_transaction', 'trace_replayTransaction']);
/** The most blocks one trace_filter spans (the server's `trace_filter_max_blocks` default). */
export const TRACE_FILTER_BLOCKS = 16;

/** The app's request, as the client tracks it for its event. */
class Call {
  readonly started = Date.now();
  status: VerificationEvent['status'] = 'verified';
  block?: { number: number; hash: string };
  level?: Level;
  flags: { kind: string; detail?: string }[] = [];
  unproven = new Set<string>();
  /** The error thrown is the client's own verified outcome (a revert), not a refusal. */
  provenError = false;
  constructor(
    readonly method: string,
    readonly params: unknown[],
    readonly raw: unknown,
  ) {}

  anchored(a: Anchored): void {
    if (!this.block || a.number >= this.block.number) this.block = { number: a.number, hash: a.hash };
    if (!this.level || !atLeast(a.level, this.level)) this.level = a.level;
  }

  flag(kind: string, detail?: string): void {
    this.flags.push({ kind, detail });
    this.status = 'unverifiable';
  }

  leaves(fields: string[] | undefined): void {
    for (const f of fields ?? []) this.unproven.add(f);
  }
}

/** A value from the server's standard method, returned under `unverifiable: "flag"`. */
class Fallback {
  constructor(readonly value: unknown) {}
}

/** The client's own outcomes, passed on unchanged through any wrapping. */
const isOutcome = (e: unknown) => e instanceof WillowRpcError || e instanceof WillowVerificationError || e instanceof WillowUnverifiableError || e instanceof Fallback;

interface Resolved {
  number?: number;
  hash?: string;
}

interface BlockEntry {
  text: string;
  proven: Record<string, unknown>;
  full?: unknown;
  unproven: string[];
  fullUnproven?: string[];
  number: number;
  hash: string;
}

type StateRead = { error: { code?: unknown; message?: unknown; data?: unknown } } | { proven: StateProven; v: StateVerified; keys: string[] };

interface StateGroup {
  address: string;
  at: Resolved;
  keys: Map<string, string>;
  code: boolean;
  method: string;
  waiters: { resolve: (r: StateRead) => void; reject: (e: unknown) => void }[];
}

interface Filter {
  kind: 'logs' | 'blocks';
  filter?: Record<string, unknown>;
  next: number;
  touched: number;
}

const withChain = (url: string, id: number): string | null => {
  try {
    const u = new URL(url);
    const m = RPC_PATH.exec(u.pathname);
    if (!m) return null;
    u.pathname = `${m[1]}${id}${m[3] ?? '/'}`;
    return u.toString();
  } catch {
    return null;
  }
};

const isTag = (x: unknown) => x === undefined || x === null || x === 'latest' || x === 'pending';
const emptyObject = (x: unknown) => x === undefined || x === null || (typeof x === 'object' && !Array.isArray(x) && Object.keys(x as object).length === 0);

/** A filter's address list, lowercased (empty: any). */
function addressList(x: unknown): Set<string> {
  const list = x === undefined || x === null ? [] : Array.isArray(x) ? x : [x];
  return new Set(list.filter((a): a is string => typeof a === 'string').map((a) => a.toLowerCase()));
}

/**
 * `trace_filter`'s traces of the blocks' `trace_block` results, as the log RPC filters them
 * (trace-verify's `answer::filtered`, alloy's `TraceFilterMatcher`): matched by mode, `after`
 * skipped, at most `count`.
 */
export function traceFiltered(filter: Record<string, unknown>, blocks: unknown[]): unknown[] {
  const fromSet = addressList(filter.fromAddress);
  const toSet = addressList(filter.toAddress);
  const any = (s: Set<string>, a: unknown) => s.size === 0 || (typeof a === 'string' && s.has(a.toLowerCase()));
  const union = filter.mode !== 'intersection';
  const all = blocks.flatMap((b) => (Array.isArray(b) ? b : [])).filter((t: any) => {
    const a = t?.action ?? {};
    let from: boolean;
    let to: boolean;
    switch (t?.type) {
      case 'call':
        [from, to] = [any(fromSet, a.from), any(toSet, a.to)];
        break;
      case 'create':
        from = any(fromSet, a.from);
        to = t.result && typeof t.result.address === 'string' ? any(toSet, t.result.address) : toSet.size === 0;
        break;
      case 'suicide':
        [from, to] = [any(fromSet, a.address), any(toSet, a.refundAddress)];
        break;
      case 'reward':
        [from, to] = [fromSet.size === 0, any(toSet, a.author)];
        break;
      default:
        return false;
    }
    if (!union) return from && to;
    if (fromSet.size === 0) return to;
    if (toSet.size === 0) return from;
    return from || to;
  });
  const after = toNumber(filter.after);
  const count = toNumber(filter.count);
  let out = all;
  if (after !== null) {
    if (after >= out.length) return [];
    out = out.slice(after);
  }
  if (count !== null) out = out.slice(0, count);
  return out;
}

function matches(log: any, filter: Record<string, unknown>): boolean {
  const a = filter.address;
  if (a !== undefined && a !== null) {
    const list = Array.isArray(a) ? a : [a];
    if (list.length && !list.some((x) => sameHex(x, log.address))) return false;
  }
  const topics = filter.topics;
  if (Array.isArray(topics)) {
    if (topics.length > (log.topics?.length ?? 0)) return false;
    for (let i = 0; i < topics.length; i++) {
      const want = topics[i];
      if (want === null || want === undefined) continue;
      const any = Array.isArray(want) ? want : [want];
      if (any.length && !any.some((t) => t === null || sameHex(t, log.topics[i]))) return false;
    }
  }
  return true;
}

class Client implements WillowVerifier {
  readonly chainId: number;
  private readonly url: string;
  private readonly lcUrl: string;
  private readonly batcher: Batcher;
  private readonly backend: Backend;
  private readonly anchors: Anchors;
  private readonly flagMode: boolean;
  private readonly now: () => number;
  private checkpoint: Promise<AnchorStart> | null = null;
  private chainCheck: Promise<void> | null = null;
  private memo = new Map<string, Promise<unknown>>();
  private results = new Lru<unknown>(1024, 16 << 20);
  private blocks = new Lru<BlockEntry>(256, 64 << 20);
  private codes = new Lru<string>(512, 16 << 20);
  private codeOf = new Lru<string>(4096);
  private sent = new Map<string, bigint>();
  private stateQueue = new Map<string, StateGroup>();
  private filters = new Map<string, Filter>();
  private filterIds = 0;

  constructor(private readonly o: WillowOptions) {
    if (!Number.isSafeInteger(o.chainId) || o.chainId <= 0) throw new Error('chainId must be a positive integer');
    if (o.minLevel && !['signed', 'committed', 'final'].includes(o.minLevel)) throw new Error('minLevel is signed, committed or final');
    if (o.unverifiable && !['throw', 'flag'].includes(o.unverifiable)) throw new Error('unverifiable is throw or flag');
    this.chainId = o.chainId;
    this.flagMode = o.unverifiable === 'flag';
    this.now = o.now ?? Date.now;
    this.url = o.url ?? `${ORIGIN}/v1/rpc/${o.chainId}/${encodeURIComponent(o.key ?? 'public')}`;
    const lc = o.lightClientUrl ?? (o.chainId === 1 ? this.url : withChain(this.url, 1));
    if (!lc) throw new Error('lightClientUrl is needed: url does not name a chain as /rpc/{chainId}/');
    this.lcUrl = lc;
    const fetchImpl = o.fetch ?? globalThis.fetch.bind(globalThis);
    this.batcher = new Batcher({ fetch: fetchImpl, windowMs: o.batchWindowMs ?? 5, maxBatch: MAX_BATCH });
    const nowSec = () => Math.floor(this.now() / 1000);
    const checkpoint = () =>
      (this.checkpoint ??= Promise.all([
        chooseCheckpoint({
          checkpoint: o.checkpoint,
          checkpointUrl: o.checkpointUrl,
          beaconApis: o.beaconApis ?? (isNode() ? BEACON_APIS : false),
          fetch: fetchImpl,
          nowSec,
        }),
        chooseStarts({ sepolia: o.sepoliaCheckpoint, bnb: o.bnbStart, checkpointUrl: o.checkpointUrl, fetch: fetchImpl, nowSec }),
      ]).then(([c, starts]) => ({ ...c, ...starts })));
    const worker = o.worker ?? (!isNode() && typeof Worker !== 'undefined' && !o.verifiers);
    if (worker && o.verifiers) throw new Error('verifiers are injected into this thread: pass worker: false');
    this.backend = worker ? new WorkerBackend(spawnWorker() as any, checkpoint) : new InlineBackend(o.verifiers ?? packageModules(), checkpoint);
    const storage = o.storage === false ? null : (o.storage ?? defaultStorage());
    this.anchors = new Anchors({
      backend: this.backend,
      rpc: (chainId, method, params) => this.batcher.call(chainId === 1 ? this.lcUrl : chainId === this.chainId ? this.url : withChain(this.url, chainId)!, method, params),
      storage,
      checkpointRoot: async (lc) => {
        const c = await checkpoint();
        return lc === 'sepolia' ? c.sepolia!.root : lc === 'bnb' ? c.bnb!.hash : c.root;
      },
      minLevel: o.minLevel ?? 'signed',
      headMaxAgeMs: (c) => o.headMaxAgeMs ?? (c === 1 ? 12_000 : 2_000),
      maxHeadAgeSec: o.maxHeadAgeSec ?? 600,
      nowMs: this.now,
      waitingMs: o.waitMs?.anchor,
      anchorWaitMs: o.anchorWaitMs,
    });
  }

  async request(args: { method: string; params?: unknown }): Promise<unknown> {
    const method = args?.method;
    if (typeof method !== 'string') throw new WillowRpcError(-32600, 'a request names its method');
    const params = Array.isArray(args.params) ? args.params : [];
    const call = new Call(method, params, args.params);
    const checked = this.chainChecked();
    checked.catch(() => undefined);
    try {
      let value: unknown;
      try {
        value = await this.dispatch(call);
      } catch (e) {
        if (!(e instanceof Fallback)) throw e;
        value = e.value;
      }
      await checked;
      if (value === null && call.status === 'verified' && !call.block) call.status = 'absent';
      this.emit(call);
      return value;
    } catch (e) {
      if (e instanceof WillowVerificationError || e instanceof WillowUnverifiableError) {
        call.status = 'failed';
        call.flags.unshift({ kind: e.data.kind, detail: e.data.detail });
      } else if (call.status === 'verified' && !call.provenError) call.status = 'refused';
      this.emit(call);
      throw e;
    }
  }

  close(): void {
    this.backend.close();
  }

  private emit(call: Call): void {
    if (!this.o.onVerification) return;
    const e: VerificationEvent = { method: call.method, status: call.status, chainId: this.chainId, ms: Date.now() - call.started };
    if (call.block) e.block = call.block;
    if (call.level) e.level = call.level;
    if (call.flags.length) {
      e.kind = call.flags[0].kind;
      e.detail = call.flags[0].detail;
      e.flags = call.flags;
    }
    if (call.unproven.size) e.unproven = [...call.unproven].sort();
    try {
      this.o.onVerification(e);
    } catch {}
  }

  /** One eth_chainId at start: a wrong endpoint fails every request (design 1.3.11). */
  private chainChecked(): Promise<void> {
    if (!this.chainCheck) {
      const p = (async () => {
        const r = await this.batcher.call(this.url, 'eth_chainId', []);
        if (r.value.error) throw this.rpcError(r.value.error);
        if (toNumber(r.value.result) !== this.chainId) {
          throw new WillowVerificationError('wrong-chain', `the endpoint serves chain ${String(r.value.result)}, not ${this.chainId}`);
        }
      })();
      this.chainCheck = p;
      p.catch((e) => {
        if (!(e instanceof WillowVerificationError) && this.chainCheck === p) this.chainCheck = null;
      });
    }
    return this.chainCheck;
  }

  private dispatch(call: Call): Promise<unknown> {
    const p = call.params;
    switch (call.method) {
      case 'eth_chainId':
        call.status = 'local';
        return Promise.resolve(toQuantity(this.chainId));
      case 'net_version':
        call.status = 'local';
        return Promise.resolve(String(this.chainId));
      case 'net_listening':
        call.status = 'local';
        return Promise.resolve(true);
      case 'net_peerCount':
        call.status = 'local';
        return Promise.resolve('0x0');
      case 'web3_clientVersion':
        call.status = 'local';
        return Promise.resolve(`willow-rpc/${VERSION}`);
      case 'eth_syncing':
        call.status = 'local';
        return Promise.resolve(false);
      case 'eth_accounts':
        call.status = 'local';
        return Promise.resolve([]);
      case 'eth_blockNumber':
        return this.head(call, true).then((h) => toQuantity(h.number!));
      case 'eth_getBlockByNumber':
        return this.resolve(call, p[0]).then((at) => this.getBlock(call, at, p[1] === true));
      case 'eth_getBlockByHash':
        return this.getBlock(call, { hash: this.hashParam(p[0]) }, p[1] === true);
      case 'eth_getBlockTransactionCountByNumber':
        return this.resolve(call, p[0]).then((at) => this.txCount(call, at));
      case 'eth_getBlockTransactionCountByHash':
        return this.txCount(call, { hash: this.hashParam(p[0]) });
      case 'eth_getBalance':
      case 'eth_getTransactionCount':
      case 'eth_getCode':
      case 'eth_getStorageAt':
        return this.state(call);
      case 'eth_getProof':
        return this.getProof(call);
      case 'eth_call':
        return this.evm(call, 'call');
      case 'eth_estimateGas':
        return this.evm(call, 'estimate');
      case 'eth_gasPrice':
      case 'eth_maxPriorityFeePerGas':
      case 'eth_blobBaseFee':
        return this.fees(call);
      case 'eth_feeHistory':
        return this.feeHistory(call);
      case 'eth_sendRawTransaction':
        return this.send(call);
      case 'eth_getTransactionByHash':
        return this.item(call, 'transaction', this.hashParam(p[0]), true);
      case 'eth_getTransactionReceipt':
        return this.item(call, 'receipt', this.hashParam(p[0]), true);
      case 'eth_getTransactionByBlockHashAndIndex':
        return this.item(call, 'transaction', `${this.hashParam(p[0])}:${this.indexParam(p[1])}`, false);
      case 'eth_getTransactionByBlockNumberAndIndex':
        return this.resolve(call, p[0]).then((at) => this.item(call, 'transaction', `${this.target(at)}:${this.indexParam(p[1])}`, false));
      case 'eth_getBlockReceipts':
        return this.resolve(call, p[0]).then((at) => this.item(call, 'blockReceipts', this.target(at), false));
      case 'eth_getLogs':
        return this.getLogs(call, p[0]);
      case 'debug_traceTransaction':
      case 'trace_transaction':
      case 'trace_replayTransaction':
      case 'debug_traceBlockByNumber':
      case 'debug_traceBlockByHash':
      case 'trace_block':
      case 'trace_replayBlockTransactions':
        return this.trace(call);
      case 'trace_filter':
        return this.traceFilter(call);
      case 'debug_traceCall':
      case 'trace_call':
        return this.callTrace(call);
      case 'eth_newFilter':
      case 'eth_newBlockFilter':
      case 'eth_getFilterChanges':
      case 'eth_getFilterLogs':
      case 'eth_uninstallFilter':
        return this.filter(call);
      default:
        return this.forward(call);
    }
  }

  // ---------- errors and the unverifiable policy (section 3.3) ----------

  private rpcError(e: { code?: unknown; message?: unknown; data?: unknown }): WillowRpcError {
    return new WillowRpcError(Number(e.code) || -32603, String(e.message ?? ''), e.data);
  }

  /** A value that can't be verified: throw, or (flag) return the server's standard response, marked. */
  private async policy(call: Call, kind: string, detail: string, fallback: () => Promise<unknown>): Promise<never> {
    if (!this.flagMode) throw new WillowUnverifiableError(kind, detail, call.method);
    const v = await fallback();
    call.flag(kind, detail);
    throw new Fallback(v);
  }

  /** The app's own request, sent as it is. */
  private async standard(call: Call): Promise<unknown> {
    const r = await this.batcher.call(this.url, call.method, call.raw === undefined ? [] : (call.raw as unknown[]));
    if (r.value.error) throw this.rpcError(r.value.error);
    return r.value.result;
  }

  /** A method with no proof: its errors pass through; a value is unverifiable. */
  private async forward(call: Call): Promise<unknown> {
    const r = await this.batcher.call(this.url, call.method, call.raw === undefined ? [] : (call.raw as unknown[]));
    if (r.value.error) {
      call.status = 'refused';
      throw this.rpcError(r.value.error);
    }
    return this.policy(call, 'method', `${call.method} has no proof`, async () => r.value.result);
  }

  /** A proof method's refusal: not served for this key or path (-32601) is unverifiable; the rest pass through. */
  private async refused(call: Call, e: { code?: unknown; message?: unknown; data?: unknown }): Promise<never> {
    if (Number(e.code) === -32601) return this.policy(call, 'not-on-plan', String(e.message ?? ''), () => this.standard(call));
    call.status = 'refused';
    throw this.rpcError(e);
  }

  private async noVerifier(call: Call, e: unknown): Promise<never> {
    return this.policy(call, 'no-verifier', e instanceof Error ? e.message : String(e), () => this.standard(call));
  }

  /** A verifier's own failure to run: a missing module is unverifiable, anything else (a trap) fails closed. */
  private verifierCall<T>(call: Call, p: Promise<T>): Promise<T> {
    return p.catch((e) => {
      if (isOutcome(e)) throw e;
      if (isNoVerifier(e)) return this.noVerifier(call, e);
      throw new WillowVerificationError('internal', e instanceof Error ? e.message : String(e), call.method);
    });
  }

  // ---------- blocks and anchors (sections 1.1, 2, 3.2) ----------

  private hashParam(x: unknown): string {
    if (!isHash(x)) throw new WillowRpcError(-32602, `expected a 32-byte hash, not ${JSON.stringify(x)?.slice(0, 80)}`);
    return x.toLowerCase();
  }

  private indexParam(x: unknown): string {
    const n = toNumber(x);
    if (n === null) throw new WillowRpcError(-32602, `expected a transaction index, not ${JSON.stringify(x)?.slice(0, 80)}`);
    return toQuantity(n);
  }

  private target(at: Resolved): string {
    return at.hash ?? toQuantity(at.number!);
  }

  private blockParam(at: Resolved): unknown {
    return at.hash ? { blockHash: at.hash } : toQuantity(at.number!);
  }

  /** The chain's newest anchored block (section 2); under `flag`, the server's number when it can't be anchored. */
  private async head(call: Call, force = false): Promise<Resolved> {
    try {
      const h = await this.anchors.head(this.chainId, 'latest', force, call.method);
      call.anchored(h);
      return { number: h.number, hash: h.hash };
    } catch (e) {
      if (!(e instanceof WillowUnverifiableError) || !this.flagMode) throw e;
      const r = await this.batcher.call(this.url, 'eth_blockNumber', []);
      if (r.value.error) throw this.rpcError(r.value.error);
      const n = toNumber(r.value.result);
      if (n === null) throw new WillowRpcError(-32603, 'eth_blockNumber returned no number');
      call.flag(e.data.kind, e.data.detail);
      return { number: n };
    }
  }

  /** A block parameter → the block the proofs must name (section 3.2 step 2). */
  private async resolve(call: Call, b: unknown): Promise<Resolved> {
    if (isTag(b)) return this.head(call);
    if (b === 'earliest') return { number: 0 };
    if (b === 'finalized' || b === 'safe') {
      if (this.chainId === 1) {
        if (b === 'safe') throw new WillowRpcError(-32000, 'not found in this store: the "safe" block tag');
        const f = await this.anchors.head(1, 'finalized', false, call.method);
        call.anchored(f);
        return { number: f.number, hash: f.hash };
      }
      // An L2's tags: the block's contents verify, the tag's meaning does not.
      if (!this.flagMode) throw new WillowUnverifiableError('tag', `"${b}" on chain ${this.chainId} can't be verified`, call.method);
      const r = await this.batcher.call(this.url, 'eth_getBlockByNumber', [b, false]);
      if (r.value.error) throw this.rpcError(r.value.error);
      const n = toNumber((r.value.result as { number?: unknown } | null)?.number);
      if (n === null) throw new WillowRpcError(-32000, `not found in this store: the "${b}" block tag`);
      call.flag('tag', `"${b}" resolved by the server to block ${n}`);
      return { number: n };
    }
    if (isHash(b)) return { hash: b.toLowerCase() };
    if (b && typeof b === 'object') {
      const o = b as { blockHash?: unknown; blockNumber?: unknown };
      if (o.blockHash !== undefined && o.blockNumber === undefined) return { hash: this.hashParam(o.blockHash) };
      if (o.blockNumber !== undefined && o.blockHash === undefined) return this.resolve(call, o.blockNumber);
    }
    const n = toNumber(b);
    if (n === null || typeof b !== 'string') throw new WillowRpcError(-32602, `bad block number ${JSON.stringify(b)?.slice(0, 80)}`);
    // A number already anchored goes out by its hash, so no other block can take its place (3.2 step 2).
    const hash = this.anchors.hashOf(this.chainId, n);
    return hash ? { number: n, hash } : { number: n };
  }

  /** Anchors a verified block; under `flag`, a block that can't be anchored is marked and kept. */
  private async anchor(call: Call, number: number, hash: string, prefetched?: Promise<Reply>): Promise<Anchored | null> {
    try {
      const a = await this.anchors.block(this.chainId, number, hash, prefetched, call.method);
      call.anchored(a);
      return a;
    } catch (e) {
      if (!(e instanceof WillowUnverifiableError) || !this.flagMode) throw e;
      call.flag(e.data.kind, e.data.detail);
      call.block ??= { number, hash };
      return null;
    }
  }

  /** The anchor of block `n` asked for with its proof, when it is neither the head nor anchored (3.2 step 3). */
  private prefetch(at: Resolved): Promise<Reply> | undefined {
    if (at.hash || at.number === undefined || this.anchors.known(this.chainId, at.number)) return undefined;
    const p = this.anchors.body(this.chainId, toQuantity(at.number));
    p.catch(() => undefined);
    return p;
  }

  private async itemRequest(call: Call, kind: ItemKind, target: string): Promise<Requested> {
    const r = await this.verifierCall(call, this.backend.itemRequest(this.chainId, kind, target));
    if (r.ok) return r;
    if (r.kind === 'invalid-request') throw new WillowRpcError(-32602, r.message);
    throw new WillowVerificationError('internal', r.message, call.method);
  }

  private async verifyItem(call: Call, kind: ItemKind, target: string, reply: Reply, full = false) {
    const r = await this.verifierCall(call, this.backend.itemVerify(this.chainId, kind, target, reply.text, full));
    if (!r.ok) {
      if (r.kind === 'internal' || r.kind === 'invalid-request') throw new WillowVerificationError('internal', r.message, call.method);
      throw new WillowVerificationError(r.fault ?? (r.kind === 'malformed-input' ? 'malformed' : 'rejected'), r.message, call.method);
    }
    call.leaves(r.report.unproven);
    return r;
  }

  /** A proof by an items kind, requested, fetched and verified (not anchored); a refusal comes back as `error`. */
  private async fetchItem(call: Call, kind: ItemKind, target: string): Promise<{ error: NonNullable<Reply['value']['error']> } | { v: ItemVerified }> {
    const req = await this.itemRequest(call, kind, target);
    const reply = await this.batcher.call(this.url, req.method, req.params);
    if (reply.value.error) return { error: reply.value.error };
    return { v: await this.verifyItem(call, kind, target, reply) };
  }

  private async proveItem(call: Call, kind: ItemKind, target: string): Promise<ItemVerified> {
    const r = await this.fetchItem(call, kind, target);
    if ('error' in r) return this.refused(call, r.error);
    return r.v;
  }

  private async blockReply(call: Call, at: Resolved): Promise<Reply> {
    const req = await this.itemRequest(call, 'block', this.target(at));
    return this.batcher.call(this.url, req.method, req.params);
  }

  /**
   * Block `at`, verified and anchored (cached by hash), its parent's hash anchored with it.
   * `reply`: its proof, already asked for; `prefetch: false`: no anchor asked for ahead.
   */
  private async block(call: Call, at: Resolved, full: boolean, opts: { reply?: Promise<Reply>; prefetch?: boolean } = {}): Promise<BlockEntry | null> {
    let entry = at.hash ? this.blocks.get(at.hash) : undefined;
    if (!entry) {
      let prefetched: Promise<Reply> | undefined;
      let replied: Promise<Reply>;
      if (opts.reply) {
        prefetched = opts.prefetch === false ? undefined : this.prefetch(at);
        replied = opts.reply;
      } else {
        const req = await this.itemRequest(call, 'block', this.target(at));
        // The anchor and the proof asked for in one turn, so they share a batch however slow the
        // verifier was to write the request (3.2 step 3).
        prefetched = opts.prefetch === false ? undefined : this.prefetch(at);
        replied = this.batcher.call(this.url, req.method, req.params);
      }
      const reply = await replied;
      if (reply.value.error) return this.refused(call, reply.value.error);
      // A number past the server's newest block: `null`, as the standard method's (an absence).
      if (reply.value.result === null && !at.hash) {
        call.status = 'absent';
        return null;
      }
      const v = await this.verifyItem(call, 'block', this.target(at), reply, full);
      const b = v.report.block;
      entry = {
        text: reply.text,
        proven: v.proven as Record<string, unknown>,
        full: v.rendered?.proven,
        unproven: v.report.unproven ?? [],
        fullUnproven: v.rendered?.unproven,
        number: toNumber(b.number)!,
        hash: b.hash.toLowerCase(),
      };
      const a = await this.anchor(call, entry.number, entry.hash, prefetched);
      if (a) {
        this.anchors.parent(a, String(b.parentHash ?? entry.proven.parentHash ?? ''));
        this.blocks.set(entry.hash, entry, reply.text.length);
      }
    } else {
      const a = await this.anchor(call, entry.number, entry.hash);
      call.leaves(entry.unproven);
      if (a) this.anchors.parent(a, String(entry.proven.parentHash ?? ''));
    }
    if (full && entry.full === undefined) {
      const v = await this.verifyItem(call, 'block', entry.hash, { text: entry.text, value: {} }, true);
      entry.full = v.rendered?.proven;
      entry.fullUnproven = v.rendered?.unproven;
    }
    if (full) call.leaves(entry.fullUnproven);
    return entry;
  }

  private async getBlock(call: Call, at: Resolved, full: boolean): Promise<unknown> {
    const e = await this.block(call, at, full);
    if (!e) return null;
    // A copy: what the app does with it can't change the verified block kept here.
    return structuredClone(full ? e.full : e.proven);
  }

  private async txCount(call: Call, at: Resolved): Promise<string | null> {
    const e = await this.block(call, at, false);
    if (!e) return null;
    const txs = e.proven.transactions;
    if (!Array.isArray(txs)) throw new WillowVerificationError('malformed', 'the proven block has no transaction list', call.method);
    return toQuantity(txs.length);
  }

  // ---------- state (1.3.3) ----------

  /** Reads of one account at one block within a tick become one willow_getStateProof. */
  private stateRead(call: Call, address: string, at: Resolved, key?: string, code = false): Promise<StateRead> {
    const k = `${address.toLowerCase()}@${at.hash ?? `n${at.number}`}`;
    let g = this.stateQueue.get(k);
    if (!g) {
      g = { address, at, keys: new Map(), code: false, method: call.method, waiters: [] };
      this.stateQueue.set(k, g);
      setTimeout(() => this.flushState(k), 0);
    }
    if (key !== undefined) {
      const norm = slotKey(key);
      if (!g.keys.has(norm)) g.keys.set(norm, key);
    }
    g.code ||= code;
    const group = g;
    return new Promise((resolve, reject) => group.waiters.push({ resolve, reject }));
  }

  private flushState(k: string): void {
    const g = this.stateQueue.get(k);
    if (!g) return;
    this.stateQueue.delete(k);
    const keys = [...g.keys.values()];
    this.proveState(g.method, g.address, g.at, keys, g.code).then(
      (r) => g.waiters.forEach((w) => w.resolve(r)),
      (e) => g.waiters.forEach((w) => w.reject(e)),
    );
  }

  private async proveState(method: string, address: string, at: Resolved, keys: string[], code: boolean): Promise<StateRead> {
    const target = JSON.stringify([address, keys, this.blockParam(at), { code }]);
    const req = await this.backend.stateRequest(this.chainId, 'state', target);
    if (!req.ok) {
      if (req.kind === 'invalid-request') throw new WillowRpcError(-32602, req.message);
      throw new WillowVerificationError(req.kind, req.message, method);
    }
    const reply = await this.batcher.call(this.url, req.method, req.params);
    if (reply.value.error) return { error: reply.value.error };
    const v = await this.backend.stateVerify(this.chainId, 'state', target, reply.text);
    if (!v.ok) throw new WillowVerificationError(v.kind === 'invalid-request' ? 'internal' : v.kind, v.message, method);
    const proven = v.proven as StateProven;
    if (proven.eth_getCode !== null && proven.eth_getCode !== undefined) {
      const h = proven.eth_getProof.codeHash.toLowerCase();
      this.codes.set(h, proven.eth_getCode, proven.eth_getCode.length);
      this.codeOf.set(address.toLowerCase(), h);
    }
    return { proven, v, keys: keys.map(slotKey) };
  }

  /** A state read's outcome for one app request: refused, or verified and anchored. */
  private async stateOf(call: Call, read: Promise<StateRead>): Promise<{ proven: StateProven; keys: string[] }> {
    // A merged proof's failure is each request's, by its own method.
    const r = await this.verifierCall(call, read).catch((e) => {
      throw e instanceof WillowVerificationError && e.data.method !== call.method ? new WillowVerificationError(e.data.kind, e.data.detail, call.method) : e;
    });
    if ('error' in r) return this.refused(call, r.error);
    await this.anchor(call, toNumber(r.v.report.block.number)!, r.v.report.block.hash);
    return { proven: r.proven, keys: r.keys };
  }

  private async state(call: Call): Promise<unknown> {
    const [address, a1, a2] = call.params;
    if (!isAddress(address)) throw new WillowRpcError(-32602, `expected an address, not ${JSON.stringify(address)?.slice(0, 80)}`);
    const blockArg = call.method === 'eth_getStorageAt' ? a2 : a1;
    let slot: string | undefined;
    if (call.method === 'eth_getStorageAt') {
      try {
        slot = slotKey(a1 as string);
      } catch {
        throw new WillowRpcError(-32602, `bad storage slot ${JSON.stringify(a1)?.slice(0, 80)}`);
      }
    }
    const at = await this.resolve(call, blockArg);
    switch (call.method) {
      case 'eth_getBalance':
        return (await this.stateOf(call, this.stateRead(call, address, at))).proven.eth_getBalance;
      case 'eth_getTransactionCount': {
        const nonce = (await this.stateOf(call, this.stateRead(call, address, at))).proven.eth_getTransactionCount;
        if (blockArg !== 'pending') return nonce;
        // The pending nonce is never the server's (1.3.3): the proven one, or past this client's own sends.
        const own = this.sent.get(address.toLowerCase());
        return own !== undefined && own + 1n > BigInt(nonce) ? toQuantity(own + 1n) : nonce;
      }
      case 'eth_getCode': {
        const hash = this.codeOf.get(address.toLowerCase());
        const cached = hash ? this.codes.get(hash) : undefined;
        const { proven: p } = await this.stateOf(call, this.stateRead(call, address, at, undefined, cached === undefined));
        const codeHash = p.eth_getProof.codeHash.toLowerCase();
        if (codeHash === EMPTY_CODE_HASH) return '0x';
        if (p.eth_getCode) return p.eth_getCode;
        // Code by code hash, verified against it when it was sent.
        const kept = this.codes.get(codeHash);
        if (kept !== undefined && keccakHex(hexToBytes(kept)) === codeHash) return kept;
        const again = (await this.stateOf(call, this.stateRead(call, address, at, undefined, true))).proven;
        if (!again.eth_getCode) throw new WillowVerificationError('code', `the code of ${address} was asked for and not proven`, call.method);
        return again.eth_getCode;
      }
      default: {
        const { proven, keys } = await this.stateOf(call, this.stateRead(call, address, at, a1 as string));
        const i = keys.indexOf(slot!);
        const value = proven.eth_getStorageAt[i];
        if (i < 0 || typeof value !== 'string') throw new WillowVerificationError('slot', `slot ${String(a1)} is not in the proof`, call.method);
        return value;
      }
    }
  }

  private async getProof(call: Call): Promise<unknown> {
    const [address, keys, b] = call.params;
    if (!isAddress(address) || !Array.isArray(keys) || !keys.every((k) => typeof k === 'string' && /^0x[0-9a-fA-F]{0,64}$/.test(k))) {
      throw new WillowRpcError(-32602, 'expected an address and a list of storage keys');
    }
    const at = await this.resolve(call, b);
    return structuredClone((await this.stateOf(call, this.proveState(call.method, address, at, keys as string[], false))).proven.eth_getProof);
  }

  // ---------- calls and estimates (1.3.4, 1.3.5) ----------

  private async evm(call: Call, kind: StateKind): Promise<unknown> {
    const [tx, b, ...rest] = call.params;
    if (!tx || typeof tx !== 'object' || Array.isArray(tx)) throw new WillowRpcError(-32602, 'expected a call object');
    const at = await this.resolve(call, b);
    // The caller's state and block overrides go with the proof request and are laid over its
    // witness here: they are the caller's own, never proven. The verifier refuses a chain, a call or
    // overrides it doesn't run (`unsupported`): those are the server's response, by the rules for
    // methods without a proof.
    while (rest.length && emptyObject(rest[rest.length - 1])) rest.pop();
    const target = JSON.stringify([tx, this.blockParam(at), ...rest.map((o) => o ?? null)]);
    const run = async () => {
      const req = await this.verifierCall(call, this.backend.stateRequest(this.chainId, kind, target));
      if (!req.ok) {
        if (req.kind === 'unsupported') return { forward: true as const };
        if (req.kind === 'invalid-request') throw new WillowRpcError(-32602, req.message);
        throw new WillowVerificationError(req.kind, req.message, call.method);
      }
      const reply = await this.batcher.call(this.url, req.method, req.params);
      if (reply.value.error) return { error: reply.value.error };
      const v = await this.verifierCall(call, this.backend.stateVerify(this.chainId, kind, target, reply.text));
      if (!v.ok) throw new WillowVerificationError(v.kind === 'invalid-request' ? 'internal' : v.kind, v.message, call.method);
      return { v };
    };
    const r = await (at.hash ? this.once(`${kind}:${at.hash}:${target}`, run) : run());
    if ('forward' in r) return this.forward(call);
    if ('error' in r) return this.refused(call, r.error!);
    await this.anchor(call, toNumber(r.v.report.block.number)!, r.v.report.block.hash);
    const out = r.v.proven as CallProven;
    if ('result' in out) return out.result;
    // A revert, halt or failed search, verified: the error the standard method sends.
    call.provenError = true;
    throw new WillowRpcError(out.error.code, out.error.message, out.error.data);
  }

  /** `run` once per key, for results fixed by a block hash; a failure or a refusal (`error`) is not kept. */
  private once<T extends object>(key: string, run: () => Promise<T>): Promise<T> {
    if (this.results.has(key)) return Promise.resolve(this.results.get(key) as T);
    const going = this.memo.get(key);
    if (going) return going as Promise<T>;
    const p = run().then((v) => {
      if (!('error' in v)) this.results.set(key, v, 1024);
      return v;
    });
    this.memo.set(key, p);
    const clear = () => this.memo.delete(key);
    p.then(clear, clear);
    return p;
  }

  // ---------- traces (willow_getTraceProof) ----------

  /**
   * A trace method's response, from the client's own run of the block (trace-verify): the block a
   * block trace names goes out by the hash the client resolved it to; the block a transaction's
   * trace is in is the one the proof names, anchored. A tracer or chain a proof does not cover is
   * the server's response, by the rules for methods without a proof.
   */
  private async trace(call: Call): Promise<unknown> {
    const v = await this.provenTrace(call, call.method, call.params);
    if (v === null) return this.forward(call);
    await this.anchor(call, toNumber(v.report.block.number)!, v.report.block.hash);
    return (v.proven as TraceProven).result;
  }

  /**
   * A call's trace (`debug_traceCall`, `trace_call`), from the client's own run of the call over
   * the state the proof proves at the block the client resolved (a call proof's witness, the
   * caller's overrides laid over it here and never sent): the call runs as `eth_call` runs it,
   * traced. Calls' traces are proven on Ethereum; elsewhere, and for what a proof does not cover,
   * the server's response by the rules for methods without a proof.
   */
  private async callTrace(call: Call): Promise<unknown> {
    if (!CALL_TRACED.has(this.chainId)) return this.forward(call);
    const p = [...call.params];
    if (!p[0] || typeof p[0] !== 'object' || Array.isArray(p[0])) throw new WillowRpcError(-32602, 'expected a call object');
    const i = call.method === 'debug_traceCall' ? 1 : 2;
    const at = await this.resolve(call, p[i]);
    for (let k = p.length; k < i; k++) p[k] = null;
    p[i] = this.blockParam(at);
    const v = await this.provenTrace(call, call.method, p, at);
    if (v === null) return this.forward(call);
    await this.anchor(call, toNumber(v.report.block.number)!, v.report.block.hash);
    return (v.proven as TraceProven).result;
  }

  /** `method(params)`'s trace proof, verified (not anchored); `null` when no proof covers it. */
  private async provenTrace(call: Call, method: string, params: unknown[], resolved?: Resolved): Promise<StateVerified | null> {
    if (!TRACED.has(this.chainId)) return null;
    const p = [...params];
    let at: Resolved | undefined = resolved;
    if (BLOCK_TRACES.has(method)) {
      at = method === 'debug_traceBlockByHash' ? { hash: this.hashParam(p[0]) } : await this.resolve(call, p[0]);
      p[0] = method === 'debug_traceBlockByHash' ? at.hash : this.blockParam(at);
    } else if (TX_TRACES.has(method)) {
      p[0] = this.hashParam(p[0]);
    }
    const target = JSON.stringify([method, p]);
    const run = async () => {
      const req = await this.verifierCall(call, this.backend.stateRequest(this.chainId, 'trace', target));
      if (!req.ok) {
        if (req.kind === 'unsupported') return { forward: true as const };
        if (req.kind === 'invalid-request') throw new WillowRpcError(-32602, req.message);
        throw new WillowVerificationError(req.kind, req.message, call.method);
      }
      const reply = await this.batcher.call(this.url, req.method, req.params);
      if (reply.value.error) return { error: reply.value.error };
      const v = await this.verifierCall(call, this.backend.stateVerify(this.chainId, 'trace', target, reply.text));
      if (!v.ok) throw new WillowVerificationError(v.kind === 'invalid-request' ? 'internal' : v.kind, v.message, call.method);
      return { v };
    };
    const r = await (at?.hash ? this.once(`trace:${target}`, run) : run());
    if ('forward' in r) return null;
    if ('error' in r) return this.refused(call, r.error!);
    return r.v;
  }

  /**
   * `trace_filter`, from each block's `trace_block` proof (one batch), filtered here as the server
   * filters (alloy's `TraceFilterMatcher`); the blocks linked by their proven parent hashes, so
   * anchoring the newest anchors them all.
   */
  private async traceFilter(call: Call): Promise<unknown> {
    if (!TRACED.has(this.chainId)) return this.forward(call);
    const f = call.params[0];
    if (!f || typeof f !== 'object' || Array.isArray(f)) throw new WillowRpcError(-32602, 'expected a trace filter object');
    const filter = f as Record<string, unknown>;
    const from = await this.resolve(call, filter.fromBlock ?? 'earliest');
    const to = await this.resolve(call, filter.toBlock ?? 'latest');
    if (from.number === undefined || to.number === undefined) throw new WillowRpcError(-32602, 'fromBlock and toBlock are numbers or tags');
    if (from.number > to.number) throw new WillowRpcError(-32602, 'invalid parameters: fromBlock cannot be greater than toBlock');
    if (to.number - from.number >= TRACE_FILTER_BLOCKS) {
      throw new WillowRpcError(-32602, `Block range too large; currently limited to ${TRACE_FILTER_BLOCKS} blocks`);
    }
    const blocks: Resolved[] = [];
    for (let n = from.number; n <= to.number; n++) blocks.push(n === to.number && to.hash ? to : n === from.number && from.hash ? from : { number: n });
    const top = blocks.length - 1;
    const prefetched = this.prefetch(blocks[top]);
    const verified = await Promise.all(blocks.map((at) => this.provenTrace(call, 'trace_block', [at.hash ? at.hash : toQuantity(at.number!)])));
    if (verified.some((v) => v === null)) return this.forward(call);
    const named = verified.map((v) => ({ number: toNumber(v!.report.block.number)!, hash: v!.report.block.hash.toLowerCase(), parentHash: v!.report.block.parentHash }));
    const linked = named.every((b, i) => i === top || (named[i + 1].number === b.number + 1 && typeof named[i + 1].parentHash === 'string' && named[i + 1].parentHash!.toLowerCase() === b.hash));
    if (linked) {
      const a = await this.anchor(call, named[top].number, named[top].hash, prefetched);
      for (let i = top; a && i > 0; i--) this.anchors.parent({ ...a, number: named[i].number, hash: named[i].hash }, named[i].parentHash!);
    } else {
      await Promise.all(named.map((b, i) => this.anchor(call, b.number, b.hash, i === top ? prefetched : this.prefetch({ number: b.number }))));
    }
    return traceFiltered(filter, verified.map((v) => (v!.proven as TraceProven).result));
  }

  // ---------- fees (1.3.6 to 1.3.8) ----------

  private async fees(call: Call): Promise<unknown> {
    const at = await this.head(call);
    const target = this.target(at);
    // eth_gasPrice and eth_maxPriorityFeePerGas at one head share one proof; each call handles a refusal itself.
    const r = await (at.hash ? this.once(`fees:${at.hash}`, () => this.fetchItem(call, 'fees', target)) : this.fetchItem(call, 'fees', target));
    if ('error' in r) return this.refused(call, r.error);
    const v = r.v;
    call.leaves(v.report.unproven);
    await this.anchor(call, toNumber(v.report.block.number)!, v.report.block.hash);
    const f = v.proven as FeesProven;
    if (f.formula !== 'willow-fee-v1') throw new WillowVerificationError('unknown-formula', `formula ${String(f.formula)}`, call.method);
    if (call.method === 'eth_gasPrice') return f.eth_gasPrice;
    if (call.method === 'eth_maxPriorityFeePerGas') return f.eth_maxPriorityFeePerGas;
    if (f.eth_blobBaseFee === null || f.eth_blobBaseFee === undefined) {
      throw new WillowRpcError(-32601, `the method eth_blobBaseFee is not available on chain ${this.chainId}: its blocks carry no blob gas`);
    }
    return f.eth_blobBaseFee;
  }

  private async feeHistory(call: Call): Promise<unknown> {
    const [countArg, newestArg, pctArg] = call.params;
    const count0 = toNumber(countArg);
    if (count0 === null) throw new WillowRpcError(-32602, 'bad blockCount');
    const percentiles = pctArg === undefined || pctArg === null ? [] : pctArg;
    if (!Array.isArray(percentiles) || !percentiles.every((x) => typeof x === 'number')) throw new WillowRpcError(-32602, 'bad rewardPercentiles');
    if (count0 === 0) {
      call.status = 'local';
      return { oldestBlock: '0x0', baseFeePerGas: [], gasUsedRatio: [] };
    }
    const at = await this.resolve(call, newestArg);
    if (at.number === undefined) throw new WillowRpcError(-32602, 'eth_feeHistory names its newest block by number or tag');
    const newest = at.number;
    const count = Math.min(count0, FEE_HISTORY_MAX, newest + 1);
    // Nitro's rewards are zeros with no bodies, so only the other chains' proofs stop at 16 blocks.
    const step = percentiles.length && !NITRO.has(this.chainId) ? FEE_HISTORY_WITH_REWARDS : count;
    const chunks: { newest: number; count: number }[] = [];
    for (let n = newest, left = count; left > 0; n -= step, left -= step) chunks.push({ newest: n, count: Math.min(step, left) });
    const parts = await Promise.all(
      chunks.map(async (c) => {
        const target = JSON.stringify([toQuantity(c.count), toQuantity(c.newest), percentiles]);
        const v = await this.proveItem(call, 'feeHistory', target);
        await this.anchor(call, toNumber(v.report.block.number)!, v.report.block.hash);
        const h = v.proven as FeeHistoryProven;
        if (toNumber(h.oldestBlock) !== c.newest - c.count + 1 || toNumber(v.report.block.number) !== c.newest) {
          throw new WillowVerificationError('wrong-range', `a fee-history proof for blocks ${c.newest - c.count + 1}-${c.newest} covers ${h.oldestBlock}-${v.report.block.number}`, call.method);
        }
        return h;
      }),
    );
    // Oldest first; each part's last base fee is the next part's first.
    parts.reverse();
    const out: Record<string, unknown> = { oldestBlock: parts[0].oldestBlock };
    const join = <T>(f: (p: FeeHistoryProven) => T[] | undefined, nextOne: boolean): T[] | undefined => {
      if (parts.some((p) => f(p) === undefined)) return undefined;
      return parts.flatMap((p, i) => (nextOne && i < parts.length - 1 ? f(p)!.slice(0, -1) : f(p)!));
    };
    const reward = join((p) => p.reward, false);
    if (reward) out.reward = reward;
    out.baseFeePerGas = join((p) => p.baseFeePerGas, true);
    out.gasUsedRatio = join((p) => p.gasUsedRatio, false);
    const blobFee = join((p) => p.baseFeePerBlobGas, true);
    if (blobFee) out.baseFeePerBlobGas = blobFee;
    const blobRatio = join((p) => p.blobGasUsedRatio, false);
    if (blobRatio) out.blobGasUsedRatio = blobRatio;
    return out;
  }

  // ---------- transactions (1.3.9, 1.3.10) ----------

  private async send(call: Call): Promise<string> {
    const raw = call.params[0];
    let bytes: Uint8Array;
    try {
      bytes = hexToBytes(raw as string);
    } catch {
      throw new WillowRpcError(-32602, "expected a signed transaction's bytes");
    }
    let hash: string;
    let sender: { from: string | null; nonce: bigint; chainId: bigint | null } | null = null;
    try {
      const t = decodeSent(bytes);
      hash = t.hash;
      sender = t;
    } catch {
      // A type this client does not decode: its hash is still that of its bytes (the server's sent_hash).
      if (bytes[0] === 3) throw new WillowRpcError(-32602, 'a blob transaction this client cannot decode');
      hash = keccakHex(bytes);
    }
    const r = await this.batcher.call(this.url, 'eth_sendRawTransaction', [raw]);
    if (r.value.error) {
      // Broadcast can't be proven; the provider's error passes through, marked.
      call.flag('provider-error', String(r.value.error.message ?? ''));
      throw this.rpcError(r.value.error);
    }
    if (!sameHex(r.value.result, hash)) throw new WillowVerificationError('tx-hash', `the server says ${String(r.value.result)}, the bytes hash to ${hash}`, call.method);
    if (sender?.from && (sender.chainId === null || sender.chainId === BigInt(this.chainId))) {
      const k = sender.from.toLowerCase();
      const had = this.sent.get(k);
      if (had === undefined || sender.nonce > had) this.sent.set(k, sender.nonce);
    }
    call.status = 'local';
    return hash;
  }

  private async item(call: Call, kind: ItemKind, target: string, nullWhenMissing: boolean): Promise<unknown> {
    const req = await this.itemRequest(call, kind, target);
    const reply = await this.batcher.call(this.url, req.method, req.params);
    const e = reply.value.error;
    if (e) {
      // An unknown transaction is an absence, as the standard methods return null for it.
      if (nullWhenMissing && Number(e.code) === -32000 && String(e.message ?? '').startsWith(NOT_FOUND_TX)) {
        call.status = 'absent';
        return null;
      }
      return this.refused(call, e);
    }
    if (reply.value.result === null) {
      call.status = 'absent';
      return null;
    }
    const v = await this.verifyItem(call, kind, target, reply);
    await this.anchor(call, toNumber(v.report.block.number)!, v.report.block.hash);
    return v.proven;
  }

  // ---------- logs (1.3.12) ----------

  private async getLogs(call: Call, f: unknown): Promise<unknown[]> {
    if (!f || typeof f !== 'object' || Array.isArray(f)) throw new WillowRpcError(-32602, 'expected a filter object');
    const filter = f as Record<string, unknown>;
    if (filter.blockHash !== undefined) {
      if (filter.fromBlock !== undefined || filter.toBlock !== undefined) throw new WillowRpcError(-32602, 'blockHash cannot be combined with fromBlock/toBlock');
      return this.logsByReceipts(call, filter, [{ hash: this.hashParam(filter.blockHash) }]);
    }
    const from = await this.resolve(call, filter.fromBlock ?? 'latest');
    const to = await this.resolve(call, filter.toBlock ?? 'latest');
    if (from.number === undefined || to.number === undefined) throw new WillowRpcError(-32602, 'fromBlock and toBlock are numbers or tags');
    return this.logsRange(call, filter, from, to);
  }

  private async logsRange(call: Call, filter: Record<string, unknown>, from: Resolved, to: Resolved): Promise<unknown[]> {
    if (from.number! > to.number!) throw new WillowRpcError(-32000, 'invalid block range params');
    const count = to.number! - from.number! + 1;
    if (count <= LOGS_BY_RECEIPTS) {
      const blocks: Resolved[] = [];
      for (let n = from.number!; n <= to.number!; n++) blocks.push(n === to.number && to.hash ? to : n === from.number && from.hash ? from : { number: n });
      return this.logsByReceipts(call, filter, blocks);
    }
    return this.logsByProof(call, filter, from.number!, to.number!);
  }

  /**
   * Logs of consecutive blocks from their block-receipts proofs, asked for at once. When each
   * block's proven parent hash is the block below it, anchoring the newest anchors them all;
   * otherwise (a verifier without `parentHash`) each is anchored, at once.
   */
  private async logsByReceipts(call: Call, filter: Record<string, unknown>, blocks: Resolved[]): Promise<unknown[]> {
    const top = blocks.length - 1;
    const prefetched = this.prefetch(blocks[top]);
    const verified = await Promise.all(blocks.map((at) => this.proveItem(call, 'blockReceipts', this.target(at))));
    const named = verified.map((v) => ({ number: toNumber(v.report.block.number)!, hash: v.report.block.hash.toLowerCase(), parentHash: v.report.block.parentHash }));
    const linked = named.every((b, i) => i === top || (named[i + 1].number === b.number + 1 && typeof named[i + 1].parentHash === 'string' && named[i + 1].parentHash!.toLowerCase() === b.hash));
    if (linked) {
      const a = await this.anchor(call, named[top].number, named[top].hash, prefetched);
      for (let i = top; a && i > 0; i--) this.anchors.parent({ ...a, number: named[i].number, hash: named[i].hash }, named[i].parentHash!);
    } else {
      await Promise.all(named.map((b, i) => this.anchor(call, b.number, b.hash, i === top ? prefetched : this.prefetch({ number: b.number }))));
    }
    return verified.flatMap((v) => {
      if (!Array.isArray(v.proven)) throw new WillowVerificationError('malformed', 'the proven receipts are not a list', call.method);
      return (v.proven as { logs?: unknown[] }[]).flatMap((r) => r?.logs ?? []).filter((l) => matches(l, filter));
    });
  }

  private async logsByProof(call: Call, filter: Record<string, unknown>, from: number, to: number): Promise<unknown[]> {
    const f: Record<string, unknown> = { fromBlock: toQuantity(from), toBlock: toQuantity(to) };
    if (filter.address !== undefined) f.address = filter.address;
    if (filter.topics !== undefined) f.topics = filter.topics;
    const deadline = this.now() + (this.o.logsTimeoutMs ?? 120_000);
    let reply: Reply;
    for (;;) {
      reply = await this.batcher.call(this.url, 'willow_getLogsProof', [f]);
      const e = reply.value.error;
      if (!e) break;
      const d = e.data as { status?: unknown; jobIds?: unknown; jobId?: unknown; retryAfterSeconds?: unknown } | undefined;
      const status = d && typeof d === 'object' ? d.status : undefined;
      if (Number(e.code) === -32000 && (status === 'queued' || status === 'running' || status === 'waiting')) {
        if (this.now() >= deadline) return this.policy(call, 'logs-timeout', `Willow had not proven blocks ${from}-${to} in time`, () => this.standard(call));
        const ids = Array.isArray(d!.jobIds) ? d!.jobIds.filter((x): x is string => typeof x === 'string') : typeof d!.jobId === 'string' ? [d!.jobId] : [];
        await this.waitForJobs(call, ids, toNumber(d!.retryAfterSeconds), deadline);
        continue;
      }
      if (Number(e.code) === -32601 || Number(e.code) === -32005) {
        return this.policy(call, 'logs-range', `${to - from + 1} blocks without a log proof: ${String(e.message ?? '')}`, () => this.standard(call));
      }
      call.status = 'refused';
      throw this.rpcError(e);
    }
    const proof = reply.value.result as { parts?: { proof?: unknown }[] } | null;
    if (!proof || !Array.isArray(proof.parts)) throw new WillowVerificationError('malformed', 'willow_getLogsProof sent no parts', call.method);
    const ids = [...new Set(proof.parts.map((p) => p?.proof).filter((x): x is string => typeof x === 'string'))];
    const proofs: Record<string, Uint8Array> = {};
    for (let i = 0; i < ids.length; i += PROOF_DOWNLOADS) {
      await Promise.all(
        ids.slice(i, i + PROOF_DOWNLOADS).map(async (id) => {
          const r = await this.batcher.single(this.url, 'willow_getProof', [id]);
          if (r.value.error) return this.refused(call, r.value.error);
          try {
            proofs[id] = hexToBytes((r.value.result as { proof: string }).proof);
          } catch {
            throw new WillowVerificationError('malformed', `willow_getProof sent no proof bytes for ${id}`, call.method);
          }
        }),
      );
    }
    const v = (await this.verifierCall(call, this.backend.logsVerify(this.chainId, reply.text, JSON.stringify(f), proofs))) as LogsVerified | { ok: false; kind: string; message: string };
    if (!v.ok) {
      if (v.kind === 'invalid-filter') throw new WillowRpcError(-32602, v.message);
      throw new WillowVerificationError(v.kind === 'rejected' ? 'logs' : v.kind === 'malformed-input' ? 'malformed' : 'internal', v.message, call.method);
    }
    const last = v.report.lastProvenBlock;
    if (toNumber(last.number) !== to) throw new WillowVerificationError('wrong-range', `the log proof ends at block ${last.number}, not ${to}`, call.method);
    await this.anchor(call, to, last.hash);
    call.leaves(['[].blockHash', '[].transactionHash']);
    return v.logs;
  }

  private async waitForJobs(call: Call, ids: string[], retryAfter: number | null, deadline: number): Promise<void> {
    const every = this.o.waitMs?.jobs ?? Math.min(Math.max(retryAfter ?? 15, 2), 15) * 1000;
    const reaskAt = this.now() + 60_000;
    for (;;) {
      await new Promise((r) => setTimeout(r, every));
      if (!ids.length || this.now() >= reaskAt || this.now() >= deadline) return;
      const jobs = await Promise.all(ids.map((id) => this.batcher.call(this.url, 'willow_getProofJob', [id]).catch(() => null)));
      const status = jobs.map((j) => (j && !j.value.error ? (j.value.result as { status?: unknown } | null)?.status : undefined));
      if (status.includes('failed')) return this.policy(call, 'logs-job-failed', `Willow's proof job ${ids[status.indexOf('failed')]} failed`, () => this.standard(call));
      if (status.every((s) => s === 'done')) return;
    }
  }

  // ---------- local filters (3.2) ----------

  private async filter(call: Call): Promise<unknown> {
    const now = this.now();
    for (const [id, f] of this.filters) if (now - f.touched > FILTER_IDLE_MS) this.filters.delete(id);
    const [arg] = call.params;
    if (call.method === 'eth_newFilter' || call.method === 'eth_newBlockFilter') {
      if (call.method === 'eth_newFilter' && (!arg || typeof arg !== 'object')) throw new WillowRpcError(-32602, 'expected a filter object');
      const head = await this.head(call);
      const id = toQuantity(BigInt(++this.filterIds) * 0x10000n + BigInt(Math.floor(Math.random() * 0x10000)));
      this.filters.set(id, { kind: call.method === 'eth_newFilter' ? 'logs' : 'blocks', filter: arg as Record<string, unknown>, next: head.number! + 1, touched: now });
      return id;
    }
    const f = typeof arg === 'string' ? this.filters.get(arg) : undefined;
    if (call.method === 'eth_uninstallFilter') {
      call.status = 'local';
      return typeof arg === 'string' && this.filters.delete(arg);
    }
    if (!f) throw new WillowRpcError(-32000, 'filter not found');
    f.touched = now;
    if (call.method === 'eth_getFilterLogs') {
      if (f.kind !== 'logs') throw new WillowRpcError(-32000, 'filter not found');
      return this.getLogs(call, f.filter);
    }
    const head = await this.head(call);
    const top = head.number!;
    if (top < f.next) return [];
    if (f.kind === 'blocks') {
      const at = (n: number): Resolved => (n === top && head.hash ? head : { number: n });
      // Every proof asked for at once (one batch); verified newest first, so each block's parent
      // hash anchors the one below it without an anchor of its own.
      const replies = new Map<number, Promise<Reply>>();
      for (let n = top; n >= f.next; n--) {
        const p = this.blockReply(call, at(n));
        p.catch(() => undefined);
        replies.set(n, p);
      }
      const out: string[] = [];
      for (let n = top; n >= f.next; n--) {
        const b = await this.block(call, at(n), false, { reply: replies.get(n), prefetch: false });
        if (!b) throw new WillowVerificationError('missing', `block ${n} of the range is absent`, call.method);
        out.unshift(b.hash);
      }
      f.next = top + 1;
      return out;
    }
    const filter = f.filter ?? {};
    const lo = Math.max(f.next, typeof filter.fromBlock === 'string' && toNumber(filter.fromBlock) !== null ? toNumber(filter.fromBlock)! : 0);
    const hi = typeof filter.toBlock === 'string' && toNumber(filter.toBlock) !== null ? Math.min(top, toNumber(filter.toBlock)!) : top;
    const logs = lo > hi ? [] : await this.logsRange(call, filter, { number: lo }, hi === top ? head : { number: hi });
    // Only once the logs are in: a failed poll is asked again from the same block.
    f.next = top + 1;
    return logs;
  }
}

/** The verifying client: `request` as EIP-1193, every response verified (design section 3). */
export function createVerifier(options: WillowOptions): WillowVerifier {
  return new Client(options);
}
