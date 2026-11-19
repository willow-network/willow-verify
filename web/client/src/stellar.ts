/**
 * `@willow-network/rpc/stellar`: Stellar's RPC (stellar-rpc's JSON-RPC methods, as SDF's
 * stellar-rpc writes them) verified against Stellar consensus. Every ledger a response rests on
 * comes in a `willow_getLedgerProof` bundle that the Stellar wasm module checks: a quorum of the
 * pinned Tier-1 quorum set signed SCP `EXTERNALIZE` for the slot after the bundle's last ledger,
 * the header chain links every ledger to it, and each ledger's result set, Soroban events and
 * (when asked) transaction set hash to what its header commits. The server's standard response
 * must equal the proven data, and the client returns the proven data. README "Stellar" lists what
 * Stellar's header does not commit to, which is never returned as verified.
 */
import { WillowRpcError, WillowUnverifiableError, WillowVerificationError } from './errors.js';
import { Batcher, type JsonRpcParams } from './http.js';
import { Lru } from './lru.js';
import type {
  Failure,
  StellarAnchor,
  StellarEvent,
  StellarProvenLedger,
  StellarProvenTransaction,
  StellarVerified,
  StellarVerifier,
  VerifierModules,
} from './verifiers.js';
import { packageModules } from './wasm.js';

export type { StellarAnchor, StellarEvent, StellarProvenLedger, StellarProvenTransaction } from './verifiers.js';

/** Stellar pubnet's network passphrase: the only network this client verifies. */
export const STELLAR_PUBNET = 'Public Global Stellar Network ; September 2015';
/** Most ledgers in one proof bundle (stellar-rpc-verify `MAX_LEDGERS`). */
export const MAX_BUNDLE_LEDGERS = 64;
/** Most ledgers in one bundle with transaction sets (the server's `MAX_LEDGERS_WITH_TXS`). */
export const MAX_BUNDLE_LEDGERS_WITH_TXS = 16;
const ORIGIN = 'https://api.willow.tech';
const TX_MASK = 0xfffff;
const OP_MASK = 0xfff;
const IDX_MAX = 0xffffffff;
const PROOF_DOWNLOADS = 4;
/** How long a ledger seen without an anchor counts as unanchorable before it is asked again. */
const PENDING_TTL_MS = 10_000;
/** Lower ledgers tried, one checkpoint apart, when a range's newest ledger is not anchored yet. */
const FRONTIER_TRIES = 4;

export interface StellarOptions {
  /** API key (Stellar has no keyless path; without one every call is refused). */
  key?: string;
  /** The RPC URL; default `https://api.willow.tech/v1/rpc/stellar/{key}`. */
  url?: string;
  /** A value that can't be verified: `throw` (default) raises WillowUnverifiableError; `flag` returns it, marked in `onVerification`. */
  unverifiable?: 'throw' | 'flag';
  /** Called once per response: what was verified, at which ledger and anchor, or why not. */
  onVerification?: (event: StellarVerificationEvent) => void;
  /** Base64 `ScpQuorumSet` XDR to trust in place of the pinned Tier-1 set (after following a handoff). */
  trustedQuorumSet?: string;
  /**
   * getLatestLedger: `anchored` (default) returns the newest ledger the client verified, at or
   * below the server's latest (the event says how far behind); `tip` returns the server's latest
   * once it is anchored, waiting up to `anchorWaitMs`.
   */
  latest?: 'anchored' | 'tip';
  /**
   * How long getTransaction (and getLatestLedger with `latest: "tip"`) waits for a ledger newer
   * than the newest anchored one; default 420000 (7 minutes: a 64-ledger checkpoint closes and is
   * published) under `throw`, 0 under `flag`. Past it the value is unverifiable (`pending`).
   */
  anchorWaitMs?: number;
  /** Between asks while waiting for an anchor; default 15000. */
  waitMs?: number;
  /** Most ledgers one getLedgers, getTransactions or getEvents page is verified over; default 128. A longer page is cut, its cursor moved back. */
  maxScanLedgers?: number;
  /** The verifier modules; default the package's own wasm builds (`wasm/stellar.js`). */
  verifiers?: VerifierModules;
  fetch?: typeof fetch;
  /** Requests within this many ms go out as one batch; default 5. */
  batchWindowMs?: number;
}

export interface StellarVerificationEvent {
  method: string;
  /**
   * `verified`: built from verified proofs. `unproven`: the server describing itself (getHealth,
   * getNetwork, getVersionInfo), passed through. `unverifiable`: returned with `flags` under
   * `unverifiable: "flag"`. `refused`: a server error passed through. `failed`: an error was raised.
   */
  status: 'verified' | 'unproven' | 'unverifiable' | 'refused' | 'failed';
  network: 'stellar';
  /** The newest ledger the response rests on, and the anchor that proves it. */
  ledger?: { sequence: number; hash: string };
  anchor?: StellarAnchor;
  /** The ledgers verified for this response. */
  ledgers?: { from: number; to: number };
  /** getLatestLedger (`anchored`): the server's latest ledger, and how many ledgers the verified one is behind it. */
  serverLatest?: number;
  behind?: number;
  /** Why it is unverifiable or failed (the first reason), and all reasons. */
  kind?: string;
  detail?: string;
  flags?: { kind: string; detail?: string }[];
  /** What the response leaves out because Stellar's header does not commit to it. */
  unproven?: string[];
  /** getEvents: events the server returned that no proof covers (fee events, classic operations' events), left out. */
  dropped?: number;
  ms: number;
}

export interface StellarRpc {
  /** A stellar-rpc JSON-RPC request (params by name): the standard result, verified; or throws. */
  request(args: { method: string; params?: unknown }): Promise<unknown>;
  close(): void;
}

/** A Stellar RPC client that verifies every response against Stellar consensus. */
export function createStellarVerifier(options: StellarOptions = {}): StellarRpc {
  return new StellarClient(options);
}

// ---------- event and transaction positions (stellar-rpc's TOID cursors) ----------

/** Where an event sits: ledger, transaction (1-based, or a stage sentinel), operation, index. */
export interface EventPos {
  l: number;
  t: number;
  o: number;
  i: number;
}

/** SEP-23 TOID: `ledger << 32 | tx << 12 | op`. */
export function toid(l: number, t: number, o: number): bigint {
  return (BigInt(l) << 32n) | (BigInt(t & TX_MASK) << 12n) | BigInt(o & OP_MASK);
}

/** stellar-rpc's event id and getEvents cursor: `%019d-%010d` of the TOID and the index. */
export function eventId(p: EventPos): string {
  return `${toid(p.l, p.t, p.o).toString().padStart(19, '0')}-${String(p.i).padStart(10, '0')}`;
}

export function parseEventId(id: unknown): EventPos | null {
  if (typeof id !== 'string') return null;
  const m = /^(\d{1,19})-(\d{1,10})$/.exec(id);
  if (!m) return null;
  const t = BigInt(m[1]);
  const i = Number(m[2]);
  if (t >= 1n << 63n || i > IDX_MAX) return null;
  return { l: Number(t >> 32n), t: Number((t >> 12n) & BigInt(TX_MASK)), o: Number(t & BigInt(OP_MASK)), i };
}

const cmpEvent = (a: EventPos, b: EventPos) => a.l - b.l || a.t - b.t || a.o - b.o || a.i - b.i;
/** The cursor after every event of ledger `l` (stellar-rpc's `MaxCursor` at `l`). */
const afterLedger = (l: number): EventPos => ({ l, t: TX_MASK, o: OP_MASK, i: IDX_MAX });

interface TxPos {
  l: number;
  t: number;
}
const cmpTx = (a: TxPos, b: TxPos) => a.l - b.l || a.t - b.t;

/** getTransactions' cursor: the TOID of the last transaction covered, operation 1, in decimal. */
export function txCursor(p: TxPos): string {
  return toid(p.l, p.t, 1).toString();
}

export function parseTxCursor(c: unknown): TxPos | null {
  if (typeof c !== 'string' || !/^\d{1,19}$/.test(c)) return null;
  const t = BigInt(c);
  if (t >= 1n << 63n) return null;
  return { l: Number(t >> 32n), t: Number((t >> 12n) & BigInt(TX_MASK)) };
}

// ---------- checkpoints ----------

/** The checkpoint ledger whose history-archive files hold ledger `x` (checkpoints are 63, 127, …). */
export const checkpointOf = (x: number): number => Math.ceil((x + 1) / 64) * 64 - 1;

/**
 * The newest ledger anchorable while ledger `p`'s anchor is not: anchoring ledger L needs SCP for
 * slot L+1, published with checkpoint `checkpointOf(L+1)`, so this is the newest L whose slot is
 * in the checkpoint before `p + 1`'s.
 */
export const anchorableBefore = (p: number): number => checkpointOf(p + 1) - 65;

/** The proof window of `size` (64 or 16) holding ledger `a`: windows end one before a checkpoint, where the newest anchorable ledger is. */
const windowEnd = (a: number, size: number): number => a - ((a + 1) % size) + size - 1;

// ---------- the client ----------

/** The app's request, as the client tracks it for its event. */
class Call {
  readonly started = Date.now();
  status: StellarVerificationEvent['status'] = 'verified';
  flags: { kind: string; detail?: string }[] = [];
  unproven = new Set<string>();
  ledger?: { sequence: number; hash: string };
  anchor?: StellarAnchor;
  range?: { from: number; to: number };
  serverLatest?: number;
  behind?: number;
  dropped = 0;
  /** The server's standard result, once fetched: what `flag` returns when nothing can be verified. */
  plain?: unknown;
  constructor(
    readonly method: string,
    readonly params: Record<string, any>,
    readonly raw: Record<string, unknown> | undefined,
  ) {}

  flag(kind: string, detail?: string): void {
    if (!this.flags.some((f) => f.kind === kind && f.detail === detail)) this.flags.push({ kind, detail });
    this.status = 'unverifiable';
  }

  rests(l: StellarProvenLedger, a: StellarAnchor): void {
    if (!this.ledger || l.sequence >= this.ledger.sequence) {
      this.ledger = { sequence: l.sequence, hash: l.hash };
      this.anchor = a;
    }
    this.range = this.range ? { from: Math.min(this.range.from, l.sequence), to: Math.max(this.range.to, l.sequence) } : { from: l.sequence, to: l.sequence };
  }
}

/** A value from the server's standard method, returned under `unverifiable: "flag"`. */
class Fallback {
  constructor(readonly value: unknown) {}
}

/** No proof can be had (the proof method refused, no verifier module): unverifiable, not a verdict. */
class ProofUnavailable extends Error {
  constructor(
    readonly kind: string,
    readonly detail: string,
  ) {
    super(detail);
  }
}

interface Cached {
  ledger: StellarProvenLedger;
  anchor: StellarAnchor;
  txs: boolean;
}

const NOT_SERVED: Record<string, string> = {
  getLedgerEntries: "getLedgerEntries is not served: Stellar's header commits to ledger entries only through bucketListHash, over whole buckets, so a single entry can't be proven",
  simulateTransaction: 'simulateTransaction is not served: a simulation runs over ledger entries, which cannot be proven from the header',
  sendTransaction: 'sendTransaction is not served: submit through another endpoint, then verify the result with getTransaction',
};

const sleep = (ms: number) => new Promise((r) => setTimeout(r, ms));
const isObject = (x: unknown): x is Record<string, any> => x !== null && typeof x === 'object' && !Array.isArray(x);
const same = (a: unknown, b: unknown) => JSON.stringify(a) === JSON.stringify(b);
const isOutcome = (e: unknown) => e instanceof WillowRpcError || e instanceof WillowVerificationError || e instanceof WillowUnverifiableError;

async function mapLimit<T, R>(items: T[], limit: number, f: (x: T) => Promise<R>): Promise<R[]> {
  const out: R[] = new Array(items.length);
  let next = 0;
  await Promise.all(
    Array.from({ length: Math.min(limit, items.length) }, async () => {
      while (next < items.length) {
        const k = next++;
        out[k] = await f(items[k]);
      }
    }),
  );
  return out;
}

class StellarClient implements StellarRpc {
  private readonly url: string;
  private readonly batcher: Batcher;
  private readonly flagMode: boolean;
  private readonly anchorWaitMs: number;
  private readonly waitMs: number;
  private readonly maxScan: number;
  private mod: Promise<StellarVerifier> | null = null;
  private cache = new Lru<Cached>(512, 128 << 20);
  /** The lowest ledger lately seen unanchorable, and when. */
  private pendingAt: { ledger: number; at: number } | null = null;

  constructor(private readonly o: StellarOptions) {
    if (o.unverifiable && !['throw', 'flag'].includes(o.unverifiable)) throw new Error('unverifiable is throw or flag');
    if (o.latest && !['anchored', 'tip'].includes(o.latest)) throw new Error('latest is anchored or tip');
    if (o.maxScanLedgers !== undefined && (!Number.isSafeInteger(o.maxScanLedgers) || o.maxScanLedgers < 1)) throw new Error('maxScanLedgers is a positive integer');
    this.flagMode = o.unverifiable === 'flag';
    this.url = o.url ?? `${ORIGIN}/v1/rpc/stellar/${encodeURIComponent(o.key ?? 'public')}`;
    const fetchImpl = o.fetch ?? globalThis.fetch.bind(globalThis);
    this.batcher = new Batcher({ fetch: fetchImpl, windowMs: o.batchWindowMs ?? 5 });
    this.anchorWaitMs = o.anchorWaitMs ?? (this.flagMode ? 0 : 420_000);
    this.waitMs = o.waitMs ?? 15_000;
    this.maxScan = o.maxScanLedgers ?? 128;
  }

  async request(args: { method: string; params?: unknown }): Promise<unknown> {
    const method = args?.method;
    if (typeof method !== 'string') throw new WillowRpcError(-32600, 'a request names its method');
    const raw = args.params;
    if (raw !== undefined && raw !== null && !isObject(raw)) throw new WillowRpcError(-32602, 'stellar-rpc takes params by name: an object');
    const call = new Call(method, (raw ?? {}) as Record<string, any>, (raw ?? undefined) as Record<string, unknown> | undefined);
    try {
      let value: unknown;
      try {
        value = await this.dispatch(call);
      } catch (e) {
        if (e instanceof Fallback) value = e.value;
        else if (e instanceof ProofUnavailable) value = this.unavailable(call, e);
        else if (isOutcome(e)) throw e;
        else throw new WillowVerificationError('internal', e instanceof Error ? e.message : String(e), method);
      }
      this.emit(call);
      return value;
    } catch (e) {
      if (e instanceof WillowVerificationError || e instanceof WillowUnverifiableError) {
        call.status = 'failed';
        call.flags.unshift({ kind: e.data.kind, detail: e.data.detail });
      } else if (call.status === 'verified') call.status = 'refused';
      this.emit(call);
      throw e;
    }
  }

  close(): void {
    this.cache = new Lru<Cached>(512, 128 << 20);
  }

  private emit(call: Call): void {
    if (!this.o.onVerification) return;
    const e: StellarVerificationEvent = { method: call.method, status: call.status, network: 'stellar', ms: Date.now() - call.started };
    if (call.ledger) e.ledger = call.ledger;
    if (call.anchor) e.anchor = call.anchor;
    if (call.range) e.ledgers = call.range;
    if (call.serverLatest !== undefined) e.serverLatest = call.serverLatest;
    if (call.behind !== undefined) e.behind = call.behind;
    if (call.flags.length) {
      e.kind = call.flags[0].kind;
      e.detail = call.flags[0].detail;
      e.flags = call.flags;
    }
    if (call.unproven.size) e.unproven = [...call.unproven].sort();
    if (call.dropped) e.dropped = call.dropped;
    try {
      this.o.onVerification(e);
    } catch {}
  }

  private dispatch(call: Call): Promise<unknown> {
    switch (call.method) {
      case 'getHealth':
      case 'getVersionInfo':
        return this.selfDescribed(call);
      case 'getNetwork':
        return this.network(call);
      case 'getLatestLedger':
        return this.latestLedger(call);
      case 'getLedgers':
        return this.ledgers(call);
      case 'getTransaction':
        return this.transaction(call);
      case 'getTransactions':
        return this.transactions(call);
      case 'getEvents':
        return this.events(call);
      default:
        if (NOT_SERVED[call.method]) throw new WillowRpcError(-32601, NOT_SERVED[call.method]);
        return this.forward(call);
    }
  }

  // ---------- the unverifiable policy ----------

  /** A value that can't be verified: throw, or (flag) return the server's, marked. */
  private unverifiable(call: Call, kind: string, detail: string, fallback: unknown): never {
    if (!this.flagMode) throw new WillowUnverifiableError(kind, detail, call.method);
    call.flag(kind, detail);
    throw new Fallback(fallback);
  }

  private unavailable(call: Call, e: ProofUnavailable): unknown {
    if (!this.flagMode || call.plain === undefined) throw new WillowUnverifiableError(e.kind, e.detail, call.method);
    call.flag(e.kind, e.detail);
    return call.plain;
  }

  /** A field Stellar's header does not commit to: left out (listed in `unproven`), or under `flag` kept and flagged. */
  private unprovable(call: Call, target: Record<string, unknown>, key: string, value: unknown, path: string, why: string): void {
    if (this.flagMode && value !== undefined) {
      target[key] = value;
      call.flag('unprovable', `${path}: ${why}`);
    } else call.unproven.add(path);
  }

  /** The server's own range fields (latestLedger, oldestLedger, …): passed through, listed as unproven. */
  private envelope(call: Call, p: Record<string, any>, keys: string[]): Record<string, unknown> {
    const out: Record<string, unknown> = {};
    for (const k of keys) {
      if (p[k] === undefined) continue;
      out[k] = p[k];
      call.unproven.add(k);
    }
    return out;
  }

  // ---------- plain methods ----------

  /** The app's request, sent as it is (`params` may be the client's own rewrite). */
  private async plain(call: Call, params: JsonRpcParams, large = false): Promise<Record<string, any>> {
    const r = large ? await this.batcher.single(this.url, call.method, params) : await this.batcher.call(this.url, call.method, params);
    if (r.value.error) {
      call.status = 'refused';
      const e = r.value.error;
      throw new WillowRpcError(Number(e.code) || -32603, String(e.message ?? ''), e.data);
    }
    call.plain = r.value.result;
    if (!isObject(r.value.result)) throw this.page(call, `the ${call.method} result is not an object`);
    return r.value.result;
  }

  private page(call: Call, detail: string): WillowVerificationError {
    return new WillowVerificationError('page', detail, call.method);
  }

  private int(call: Call, x: unknown, what: string): number {
    if (typeof x !== 'number' || !Number.isSafeInteger(x) || x < 0) throw this.page(call, `${what} is not a ledger number`);
    return x;
  }

  private list(call: Call, x: unknown, what: string): Record<string, any>[] {
    if (!Array.isArray(x) || !x.every(isObject)) throw this.page(call, `${what} is not a list of objects`);
    return x;
  }

  private jsonFormat(call: Call, p: unknown): void {
    if (call.params.xdrFormat !== undefined && call.params.xdrFormat !== 'base64') {
      this.unverifiable(call, 'json-format', 'only base64 XDR responses are verified (xdrFormat "json" is not)', p);
    }
  }

  private async selfDescribed(call: Call): Promise<unknown> {
    const p = await this.plain(call, call.raw);
    call.status = 'unproven';
    return p;
  }

  private async network(call: Call): Promise<unknown> {
    const p = await this.plain(call, call.raw);
    if (p.passphrase !== STELLAR_PUBNET) throw new WillowVerificationError('wrong-network', `the server names network ${JSON.stringify(p.passphrase)}, not Stellar pubnet`, call.method);
    call.status = 'unproven';
    return p;
  }

  private async forward(call: Call): Promise<unknown> {
    const p = await this.plain(call, call.raw);
    return this.unverifiable(call, 'method', `${call.method} has no proof`, p);
  }

  // ---------- proofs ----------

  private module(): Promise<StellarVerifier> {
    if (!this.mod) {
      const f = (this.o.verifiers ?? packageModules()).stellar;
      const p = (f ? Promise.resolve().then(() => f()) : Promise.reject(new Error('this build has none'))).catch((e) => {
        this.mod = null;
        throw new ProofUnavailable('no-verifier', `no stellar verifier module: ${e instanceof Error ? e.message : String(e)}`);
      });
      this.mod = p;
    }
    return this.mod;
  }

  private pendingFloor(): number | null {
    const p = this.pendingAt;
    return p && Date.now() - p.at < PENDING_TTL_MS ? p.ledger : null;
  }

  private notePending(l: number): void {
    const floor = this.pendingFloor();
    if (floor === null || l < floor) this.pendingAt = { ledger: l, at: Date.now() };
  }

  /** Every ledger of `a..b` from the cache (with transaction sets when `txs`), or null. */
  private cached(call: Call, a: number, b: number, txs: boolean): StellarProvenLedger[] | null {
    const out: Cached[] = [];
    for (let s = a; s <= b; s++) {
      const c = this.cache.get(String(s));
      if (!c || (txs && !c.txs)) return null;
      out.push(c);
    }
    for (const c of out) call.rests(c.ledger, c.anchor);
    return out.map((c) => c.ledger);
  }

  /** One `willow_getLedgerProof` for `a..b`, anchored at `b`, verified; null when it has no anchor yet. */
  private async bundle(call: Call, a: number, b: number, txs: boolean): Promise<StellarProvenLedger[] | null> {
    const r = await this.batcher.single(this.url, 'willow_getLedgerProof', { startLedger: a, endLedger: b, transactions: txs });
    if (r.value.error) throw new ProofUnavailable('proof-refused', `willow_getLedgerProof ${a}..${b}: ${String(r.value.error.message ?? '')}`);
    const res = r.value.result;
    // No anchor is no claim: nothing in the bundle is read.
    if (isObject(res) && (res.anchor === null || res.anchor === undefined)) return null;
    const m = await this.module();
    let v: StellarVerified | Failure;
    try {
      v = JSON.parse(m.verifyLedgerBundle(r.text, this.o.trustedQuorumSet ?? null));
    } catch (e) {
      throw new WillowVerificationError('internal', `the Stellar verifier failed to run: ${e instanceof Error ? e.message : String(e)}`, call.method);
    }
    if (!v.ok) {
      if (v.kind === 'pending') return null;
      if (v.kind === 'invalid-request') throw new WillowRpcError(-32602, `trustedQuorumSet: ${v.message}`);
      throw new WillowVerificationError('ledger-proof', `ledgers ${a}..${b}: ${v.message}`, call.method);
    }
    const { ledgers, anchor, network } = v.verified;
    if (network !== STELLAR_PUBNET || ledgers.length !== b - a + 1 || ledgers.some((l, k) => l.sequence !== a + k) || anchor.ledger !== b) {
      throw new WillowVerificationError('ledger-proof', `the proof asked for ledgers ${a}..${b} covers others`, call.method);
    }
    for (const l of ledgers) {
      if (txs && l.transactions.some((t) => t.envelopeXdr === undefined)) {
        throw new WillowVerificationError('ledger-proof', `the proof of ledger ${l.sequence}, asked with transactions, carries no transaction set`, call.method);
      }
    }
    const bytes = Math.ceil(r.text.length / ledgers.length);
    for (const l of ledgers) {
      const old = this.cache.get(String(l.sequence));
      if (!old || txs || !old.txs) this.cache.set(String(l.sequence), { ledger: l, anchor, txs }, bytes);
      call.rests(l, anchor);
    }
    const floor = this.pendingFloor();
    if (floor !== null && b >= floor) this.pendingAt = null;
    return ledgers;
  }

  /** Ledger `l`, proven alone (or from the cache); null when it is not anchorable yet. `ask`: ask even if lately seen unanchorable. */
  private async ledgerAt(call: Call, l: number, txs: boolean, ask = false): Promise<StellarProvenLedger | null> {
    const hit = this.cached(call, l, l, txs);
    if (hit) return hit[0];
    const floor = this.pendingFloor();
    if (!ask && floor !== null && l > anchorableBefore(floor)) return null;
    const got = await this.bundle(call, l, l, txs);
    if (got) return got[0];
    this.notePending(l);
    return null;
  }

  /**
   * The ledgers of one proof window `a..b`, proven: through `b`, or, when `b` is not anchored yet,
   * through the newest anchorable ledger below it (tried one checkpoint at a time), or none.
   */
  private async window(call: Call, a: number, b: number, txs: boolean): Promise<{ ledgers: StellarProvenLedger[]; through: number }> {
    let end = b;
    for (let tries = 0; tries < FRONTIER_TRIES; ) {
      const floor = this.pendingFloor();
      if (floor !== null && end > anchorableBefore(floor)) end = anchorableBefore(floor);
      if (end < a) break;
      const hit = this.cached(call, a, end, txs);
      if (hit) return { ledgers: hit, through: end };
      const got = await this.bundle(call, a, end, txs);
      if (got) return { ledgers: got, through: end };
      this.notePending(end);
      tries++;
      end = anchorableBefore(end);
    }
    return { ledgers: [], through: a - 1 };
  }

  /** Ledgers `from..to` proven, in windows of 64 (16 with transaction sets): `through` is the last of the proven run from `from`. */
  private async prove(call: Call, from: number, to: number, txs: boolean): Promise<{ ledgers: Map<number, StellarProvenLedger>; through: number }> {
    const windows: [number, number][] = [];
    for (let a = from; a <= to; ) {
      const b = Math.min(to, windowEnd(a, txs ? MAX_BUNDLE_LEDGERS_WITH_TXS : MAX_BUNDLE_LEDGERS));
      windows.push([a, b]);
      a = b + 1;
    }
    const got = await mapLimit(windows, PROOF_DOWNLOADS, ([a, b]) => this.window(call, a, b, txs));
    const ledgers = new Map<number, StellarProvenLedger>();
    let through = from - 1;
    for (let k = 0; k < windows.length; k++) {
      for (const l of got[k].ledgers) ledgers.set(l.sequence, l);
      through = got[k].through;
      if (through < windows[k][1]) break;
    }
    for (const s of [...ledgers.keys()]) if (s > through) ledgers.delete(s);
    return { ledgers, through };
  }

  /** Ledger `l` proven, waiting up to `anchorWaitMs` for its anchor; else unverifiable (`pending`). */
  private async waitFor(call: Call, l: number, txs: boolean): Promise<StellarProvenLedger> {
    const deadline = Date.now() + this.anchorWaitMs;
    for (;;) {
      const got = await this.ledgerAt(call, l, txs, true);
      if (got) return got;
      const left = deadline - Date.now();
      if (left <= 0) {
        return this.unverifiable(
          call,
          'pending',
          `ledger ${l} is not anchored yet: SCP messages reach the history archive once per 64-ledger checkpoint (checkpoint ${checkpointOf(l + 1)})`,
          call.plain,
        );
      }
      await sleep(Math.min(this.waitMs, left));
    }
  }

  private async match(events: StellarEvent[], filters: unknown): Promise<boolean[]> {
    const m = await this.module();
    const r = JSON.parse(m.matchEvents(JSON.stringify(events), JSON.stringify(filters ?? null)));
    if (!r.ok) throw new WillowRpcError(-32602, `filters: ${r.message}`);
    if (!Array.isArray(r.matches) || r.matches.length !== events.length) throw new Error('matchEvents answered for another number of events');
    return r.matches;
  }

  // ---------- ledgers ----------

  private latestOut(call: Call, l: StellarProvenLedger, p: Record<string, any> | undefined): Record<string, unknown> {
    const out: Record<string, unknown> = { id: l.hash, protocolVersion: l.protocolVersion, sequence: l.sequence, closeTime: l.closeTime, headerXdr: l.headerXdr };
    this.unprovable(call, out, 'metadataXdr', p?.metadataXdr, 'metadataXdr', 'LedgerCloseMeta is not committed by the header');
    return out;
  }

  private sameLatest(call: Call, p: Record<string, any>, l: StellarProvenLedger): void {
    const diff =
      p.id !== l.hash ? 'id' : String(p.closeTime) !== l.closeTime ? 'closeTime' : p.headerXdr !== l.headerXdr ? 'headerXdr' : p.protocolVersion !== l.protocolVersion ? 'protocolVersion' : null;
    if (diff) throw new WillowVerificationError('ledger-mismatch', `ledger ${l.sequence}: the server's ${diff} is not the proven one`, call.method);
  }

  private async latestLedger(call: Call): Promise<unknown> {
    const p = await this.plain(call, call.raw);
    const seq = this.int(call, p.sequence, 'sequence');
    if (this.o.latest === 'tip') {
      const l = await this.waitFor(call, seq, false);
      this.sameLatest(call, p, l);
      return this.latestOut(call, l, p);
    }
    let l: StellarProvenLedger | null = null;
    for (let c = seq, tries = 0; tries < FRONTIER_TRIES && c > 0; ) {
      const floor = this.pendingFloor();
      if (floor !== null && c > anchorableBefore(floor)) {
        c = anchorableBefore(floor);
        continue;
      }
      l = await this.ledgerAt(call, c, false);
      if (l) break;
      tries++;
      c = anchorableBefore(c);
    }
    if (!l) return this.unverifiable(call, 'pending', `no ledger up to ${seq} could be anchored yet`, p);
    if (l.sequence === seq) this.sameLatest(call, p, l);
    call.serverLatest = seq;
    call.behind = seq - l.sequence;
    return this.latestOut(call, l, l.sequence === seq ? p : undefined);
  }

  private async ledgers(call: Call): Promise<unknown> {
    const q = call.params;
    const p = await this.plain(call, call.raw, true);
    this.jsonFormat(call, p);
    const served = this.list(call, p.ledgers, 'ledgers');
    const cursor = q.pagination?.cursor;
    const start = cursor !== undefined && cursor !== '' ? Number(cursor) + 1 : q.startLedger;
    if (!Number.isSafeInteger(start) || start < 1) throw this.page(call, 'the page has no start: startLedger or pagination.cursor');
    served.forEach((s, k) => {
      if (s.sequence !== start + k) throw this.page(call, `the ledgers served are not consecutive from ${start}`);
    });
    const env = () => this.envelope(call, p, ['latestLedger', 'latestLedgerCloseTime', 'oldestLedger', 'oldestLedgerCloseTime']);
    const n = Math.min(served.length, this.maxScan);
    if (n === 0) return { ledgers: [], ...env(), cursor: cursor ?? String(start - 1) };
    const end = start + n - 1;
    const proof = await this.prove(call, start, end, false);
    const out: Record<string, unknown>[] = [];
    for (let k = 0; k < n; k++) {
      const s = served[k];
      const l = proof.ledgers.get(start + k);
      if (!l) {
        if (!this.flagMode) break;
        call.flag('pending', `ledgers ${start + k}..${end} are not anchored yet`);
        out.push(s);
        continue;
      }
      const diff =
        s.hash !== l.hash ? 'hash' : String(s.ledgerCloseTime) !== l.closeTime ? 'ledgerCloseTime' : s.headerXdr !== l.headerHistoryXdr ? 'headerXdr' : null;
      if (diff) throw new WillowVerificationError('ledger-mismatch', `ledger ${l.sequence}: the server's ${diff} is not the proven one`, call.method);
      const o: Record<string, unknown> = { hash: l.hash, sequence: l.sequence, ledgerCloseTime: l.closeTime, headerXdr: l.headerHistoryXdr };
      this.unprovable(call, o, 'metadataXdr', s.metadataXdr, 'ledgers[].metadataXdr', 'LedgerCloseMeta is not committed by the header');
      out.push(o);
    }
    return { ledgers: out, ...env(), cursor: out.length ? String(start + out.length - 1) : (cursor ?? String(start - 1)) };
  }

  // ---------- transactions ----------

  private sameTx(call: Call, s: Record<string, any>, t: StellarProvenTransaction): void {
    const diff =
      typeof s.txHash !== 'string' || s.txHash.toLowerCase() !== t.txHash
        ? 'txHash'
        : s.status !== t.status
          ? 'status'
          : s.applicationOrder !== t.applicationOrder
            ? 'applicationOrder'
            : s.feeBump !== t.feeBump
              ? 'feeBump'
              : s.resultXdr !== t.resultXdr
                ? 'resultXdr'
                : s.envelopeXdr !== t.envelopeXdr
                  ? 'envelopeXdr'
                  : s.ledger !== t.ledger
                    ? 'ledger'
                    : String(s.createdAt) !== t.createdAt
                      ? 'createdAt'
                      : null;
    if (diff) throw new WillowVerificationError('transaction-mismatch', `transaction ${t.txHash}: the server's ${diff} is not the proven one`, call.method);
  }

  /** A transaction as stellar-rpc writes it, from proven fields; the rest left out or (flag) kept and flagged. */
  private txOut(call: Call, s: Record<string, any>, t: StellarProvenTransaction, l: StellarProvenLedger, at: string, createdAt: string | number): Record<string, unknown> {
    const out: Record<string, unknown> = {
      status: t.status,
      txHash: t.txHash,
      applicationOrder: t.applicationOrder,
      feeBump: t.feeBump,
      envelopeXdr: t.envelopeXdr,
      resultXdr: t.resultXdr,
    };
    this.unprovable(call, out, 'resultMetaXdr', s.resultMetaXdr, `${at}resultMetaXdr`, 'TransactionMeta is not committed by the header');
    this.unprovable(call, out, 'diagnosticEventsXdr', s.diagnosticEventsXdr, `${at}diagnosticEventsXdr`, 'diagnostic events are not committed');
    const served = isObject(s.events) ? s.events : undefined;
    const events: Record<string, unknown> = {};
    const inv = l.invocations.find((i) => i.tx === t.applicationOrder);
    if (inv) {
      const ops = served?.contractEventsXdr;
      const mine = Array.isArray(ops) ? ops[inv.op] : undefined;
      if (!same(mine ?? [], inv.eventsXdr)) {
        throw new WillowVerificationError('event-mismatch', `transaction ${t.txHash}: the server's contract events are not the proven ones`, call.method);
      }
      events.contractEventsXdr = Array.from({ length: inv.op + 1 }, (_, k) => (k === inv.op ? inv.eventsXdr : []));
    } else {
      this.unprovable(call, events, 'contractEventsXdr', served?.contractEventsXdr, `${at}events.contractEventsXdr`, "classic operations' events are not committed (CAP-67)");
    }
    this.unprovable(call, events, 'transactionEventsXdr', served?.transactionEventsXdr, `${at}events.transactionEventsXdr`, 'fee events are not committed (CAP-67)');
    if (Object.keys(events).length) out.events = events;
    if (t.returnValueXdr !== undefined) out.returnValueXdr = t.returnValueXdr;
    out.ledger = t.ledger;
    out.createdAt = createdAt;
    return out;
  }

  private async transaction(call: Call): Promise<unknown> {
    const hash = call.params.hash;
    const p = await this.plain(call, call.raw);
    this.jsonFormat(call, p);
    if (p.status === 'NOT_FOUND') {
      return this.unverifiable(call, 'not-found', `that ${String(hash)} is in no ledger of the server's retention window is not provable`, p);
    }
    if (p.status !== 'SUCCESS' && p.status !== 'FAILED') throw new WillowVerificationError('transaction-mismatch', `status ${JSON.stringify(p.status)}`, call.method);
    if (typeof hash !== 'string' || !/^[0-9a-fA-F]{64}$/.test(hash)) throw new WillowRpcError(-32602, 'hash is a transaction hash: 64 hex digits');
    const seq = this.int(call, p.ledger, 'ledger');
    const order = this.int(call, p.applicationOrder, 'applicationOrder');
    const l = await this.waitFor(call, seq, true);
    const t = l.transactions[order - 1];
    if (!t || t.txHash !== hash.toLowerCase()) {
      throw new WillowVerificationError('transaction-mismatch', `ledger ${seq} has no transaction ${hash.toLowerCase()} at application order ${order}`, call.method);
    }
    this.sameTx(call, p, t);
    return {
      ...this.envelope(call, p, ['latestLedger', 'latestLedgerCloseTime', 'oldestLedger', 'oldestLedgerCloseTime']),
      ...this.txOut(call, p, t, l, '', t.createdAt),
    };
  }

  private async transactions(call: Call): Promise<unknown> {
    const q = call.params;
    const p = await this.plain(call, call.raw, true);
    this.jsonFormat(call, p);
    const served = this.list(call, p.transactions, 'transactions');
    const cursor = q.pagination?.cursor;
    let start: TxPos;
    if (cursor !== undefined && cursor !== '') {
      const c = parseTxCursor(cursor);
      if (!c) throw new WillowRpcError(-32602, 'pagination.cursor is not a transaction cursor');
      start = { l: c.l, t: c.t + 1 };
    } else {
      if (!Number.isSafeInteger(q.startLedger) || q.startLedger < 1) throw this.page(call, 'the page has no start: startLedger or pagination.cursor');
      start = { l: q.startLedger, t: 1 };
    }
    const serverEnd = parseTxCursor(p.cursor);
    if (!serverEnd) throw this.page(call, `the cursor ${JSON.stringify(p.cursor)} is not a transaction cursor`);
    const at = served.map((s) => ({ l: this.int(call, s.ledger, 'ledger'), t: this.int(call, s.applicationOrder, 'applicationOrder') }));
    at.forEach((x, k) => {
      if (cmpTx(x, start) < 0 || cmpTx(x, serverEnd) > 0 || (k > 0 && cmpTx(x, at[k - 1]) <= 0)) {
        throw this.page(call, `transaction ${x.l}:${x.t} is out of order or outside the page's cursors`);
      }
    });
    const env = () => this.envelope(call, p, ['latestLedger', 'latestLedgerCloseTimestamp', 'oldestLedger', 'oldestLedgerCloseTimestamp']);
    // The page covers start..end; past `maxScanLedgers` it is cut at a ledger's end.
    let end: TxPos = serverEnd;
    if (end.l - start.l + 1 > this.maxScan) end = { l: start.l + this.maxScan - 1, t: TX_MASK };
    const before = () => cursor ?? txCursor({ l: start.l, t: start.t - 1 });
    if (cmpTx(end, start) < 0) return { transactions: [], ...env(), cursor: before() };
    const proof = await this.prove(call, start.l, end.l, true);
    const verifiedEnd: TxPos = proof.through < end.l ? { l: proof.through, t: TX_MASK } : end;
    const expected: { t: StellarProvenTransaction; l: StellarProvenLedger }[] = [];
    for (let s = start.l; s <= Math.min(end.l, proof.through); s++) {
      const l = proof.ledgers.get(s)!;
      for (const t of l.transactions) {
        const x = { l: s, t: t.applicationOrder };
        if (cmpTx(x, start) >= 0 && cmpTx(x, verifiedEnd) <= 0) expected.push({ t, l });
      }
    }
    const out: Record<string, unknown>[] = [];
    let k = 0;
    for (let j = 0; j < served.length; j++) {
      if (cmpTx(at[j], verifiedEnd) > 0) {
        if (this.flagMode && cmpTx(at[j], end) <= 0) {
          call.flag('pending', `ledgers after ${proof.through} are not anchored yet`);
          out.push(served[j]);
        }
        continue;
      }
      const e = expected[k];
      if (!e || cmpTx(at[j], { l: e.t.ledger, t: e.t.applicationOrder }) < 0) {
        throw new WillowVerificationError('transactions', `the server's transaction at ${at[j].l}:${at[j].t} is not in the proven ledger`, call.method);
      }
      if (cmpTx(at[j], { l: e.t.ledger, t: e.t.applicationOrder }) > 0) {
        throw new WillowVerificationError('transactions', `transaction ${e.t.ledger}:${e.t.applicationOrder} (${e.t.txHash}) is proven but left out`, call.method);
      }
      k++;
      this.sameTx(call, served[j], e.t);
      out.push(this.txOut(call, served[j], e.t, e.l, 'transactions[].', Number(e.t.createdAt)));
    }
    if (k < expected.length) {
      const e = expected[k].t;
      throw new WillowVerificationError('transactions', `transaction ${e.ledger}:${e.applicationOrder} (${e.txHash}) is proven but left out`, call.method);
    }
    const outEnd = this.flagMode ? end : verifiedEnd;
    let next: string;
    if (cmpTx(outEnd, start) < 0) next = before();
    else if (outEnd === serverEnd) next = String(p.cursor);
    else next = outEnd.t === TX_MASK ? txCursor({ l: outEnd.l + 1, t: 0 }) : txCursor(outEnd);
    return { transactions: out, ...env(), cursor: next };
  }

  // ---------- events ----------

  private async events(call: Call): Promise<unknown> {
    const q = call.params;
    const cursor = q.pagination?.cursor;
    let start: EventPos;
    let params: JsonRpcParams = call.raw;
    if (cursor !== undefined && cursor !== null && cursor !== '') {
      const c = parseEventId(cursor);
      if (!c) throw new WillowRpcError(-32602, 'pagination.cursor is not an event id');
      start = { ...c, i: c.i + 1 };
    } else {
      if (!Number.isSafeInteger(q.startLedger) || q.startLedger < 1) throw new WillowRpcError(-32602, 'startLedger or pagination.cursor is required');
      start = { l: q.startLedger, t: 0, o: 0, i: 0 };
      // Ask for no more than this client verifies: the server's cursor then says where the page ends.
      const capped = q.startLedger + this.maxScan;
      if (q.endLedger === undefined || q.endLedger === null || q.endLedger === 0 || (typeof q.endLedger === 'number' && q.endLedger > capped)) params = { ...q, endLedger: capped };
    }
    const p = await this.plain(call, params, true);
    this.jsonFormat(call, p);
    const served = this.list(call, p.events, 'events');
    const serverEnd = parseEventId(p.cursor);
    if (!serverEnd) throw this.page(call, `the cursor ${JSON.stringify(p.cursor)} is not an event id`);
    const at = served.map((s) => {
      const x = parseEventId(s.id);
      if (!x) throw this.page(call, `event id ${JSON.stringify(s.id)} is not one`);
      return x;
    });
    at.forEach((x, k) => {
      if (cmpEvent(x, start) < 0 || cmpEvent(x, serverEnd) > 0 || (k > 0 && cmpEvent(x, at[k - 1]) <= 0)) {
        throw this.page(call, `event ${eventId(x)} is out of order or outside the page's cursors`);
      }
    });
    const env = () => this.envelope(call, p, ['latestLedger', 'oldestLedger', 'latestLedgerCloseTime', 'oldestLedgerCloseTime']);
    // The page covers start..end: what the server says it scanned, cut at maxScanLedgers and the endLedger asked.
    let end = serverEnd;
    const lastLedger = Math.min(start.l + this.maxScan - 1, typeof q.endLedger === 'number' && q.endLedger > 0 ? q.endLedger - 1 : Infinity);
    if (end.l > lastLedger) end = afterLedger(lastLedger);
    const before = () => (cursor !== undefined && cursor !== null && cursor !== '' ? String(cursor) : eventId(afterLedger(start.l - 1)));
    if (cmpEvent(end, start) < 0) {
      await this.match([], q.filters);
      return { events: [], cursor: before(), ...env() };
    }
    const proof = await this.prove(call, start.l, end.l, false);
    const verifiedEnd = proof.through < end.l ? afterLedger(proof.through) : end;
    // Every proven event in start..verifiedEnd, and every place a proven invocation holds.
    const proven = new Map<string, StellarEvent>();
    const committed = new Set<string>();
    for (let s = start.l; s <= Math.min(end.l, proof.through); s++) {
      const l = proof.ledgers.get(s)!;
      for (const inv of l.invocations) committed.add(`${s}:${inv.tx}:${inv.op}`);
      for (const ev of l.events) {
        const x = parseEventId(ev.id);
        if (x && cmpEvent(x, start) >= 0 && cmpEvent(x, verifiedEnd) <= 0) proven.set(eventId(x), ev);
      }
    }
    const candidates = [...proven.values()];
    const matches = await this.match(candidates, q.filters);
    const expected = new Map(candidates.filter((_, k) => matches[k]).map((e) => [eventId(parseEventId(e.id)!), e]));
    const out: unknown[] = [];
    const seen = new Set<string>();
    for (let j = 0; j < served.length; j++) {
      const x = at[j];
      if (cmpEvent(x, verifiedEnd) > 0) {
        if (this.flagMode && cmpEvent(x, end) <= 0) {
          call.flag('pending', `ledgers after ${proof.through} are not anchored yet`);
          out.push(served[j]);
        }
        continue;
      }
      if (!committed.has(`${x.l}:${x.t}:${x.o}`)) {
        // Not a successful Soroban invocation's event: a fee event, a classic operation's (CAP-67).
        if (this.flagMode) {
          call.flag('unprovable', 'fee events and the events of classic operations are not committed (CAP-67)');
          out.push(served[j]);
        } else call.dropped++;
        continue;
      }
      const id = eventId(x);
      const want = expected.get(id);
      if (!want) {
        throw new WillowVerificationError('event-extra', `event ${id}: ${proven.has(id) ? 'does not match the filters' : 'no such event is proven'}`, call.method);
      }
      for (const [k, v] of Object.entries(want)) {
        const s = k === 'contractId' ? (served[j][k] ?? '') : served[j][k];
        if (!same(s, v)) throw new WillowVerificationError('event-mismatch', `event ${id}: the server's ${k} is not the proven one`, call.method);
      }
      seen.add(id);
      out.push(want);
    }
    for (const [id, e] of expected) {
      if (!seen.has(id)) throw new WillowVerificationError('event-missing', `event ${id} (transaction ${e.txHash}) matches the filters and is proven, but the response leaves it out`, call.method);
    }
    if (call.dropped) call.unproven.add('events no Soroban result commits to (fee events, events of classic operations)');
    const outEnd = this.flagMode ? end : verifiedEnd;
    return { events: out, cursor: cmpEvent(outEnd, start) < 0 ? before() : eventId(outEnd), ...env() };
  }
}
