import { isNoVerifier, lightClientOf, type Backend } from './backend.js';
import { WillowUnverifiableError, WillowVerificationError } from './errors.js';
import { isHash, toNumber, toQuantity, ZERO_HASH } from './hex.js';
import type { Reply } from './http.js';
import { Lru } from './lru.js';
import { LIGHT_CLIENT_KEY, type KeyValueStorage } from './storage.js';
import type { AnchorFailure, AnchorVerified, LightClientName } from './verifiers.js';

export type Level = 'signed' | 'committed' | 'final';
const RANK: Record<Level, number> = { signed: 0, committed: 1, final: 2 };
export const atLeast = (l: Level, min: Level): boolean => RANK[l] >= RANK[min];

/** A block hash anchored on Ethereum, as the client keeps it. */
export interface Anchored {
  chainId: number;
  number: number;
  hash: string;
  level: Level;
  path: string;
  /** The signed head's (Ethereum) or signed block's (L2 sequencer) timestamp, seconds. */
  time?: number;
  /** When the anchor stops counting (ms); Infinity for final ones. */
  expires: number;
}

export interface AnchorsOptions {
  backend: Backend;
  /** JSON-RPC on chain `chainId`'s route (1: the light client's). */
  rpc: (chainId: number, method: string, params: unknown[]) => Promise<Reply>;
  storage: KeyValueStorage | null;
  /** What a light client's bootstrap is asked for: Ethereum's checkpoint root, Sepolia's, or BNB's start hash. */
  checkpointRoot: (lightClient?: LightClientName) => Promise<string>;
  minLevel: Level;
  headMaxAgeMs: (chainId: number) => number;
  /** How old the newest block may be (appendix D rules 1 and 2). */
  maxHeadAgeSec: number;
  nowMs: () => number;
  /** Waits before asking again for a block newer than the signed head (default: the server's `retryAfterSeconds`, else 4000). */
  waitingMs?: number;
  /** How long a block newer than the signed head is waited for, in all (default 36000); then it is unverifiable (`anchor-waiting`). */
  anchorWaitMs?: number;
}

/** Light-client updates asked for at once (one per period; anchor-verify lc.rs `MAX_UPDATES`; BNB: one per epoch, bnb.rs). */
const UPDATES_PER_ASK = 16;
const BNB_UPDATES_PER_ASK = 64;
/** BNB's signed level: one validator's seal, counted at most 10 minutes (anchor-verify bnb.rs). */
const BNB_SIGNED_SECS = 600;
/** Where each light client's kept data goes: Ethereum's at the key it always had. */
const keyOf = (lc?: LightClientName) => (lc ? `${LIGHT_CLIENT_KEY}:${lc}` : LIGHT_CLIENT_KEY);
/** Chains whose newest block is final, not signed (BNB's `latest` is its newest finalized block). */
const LATEST_IS_FINAL = new Set([56]);
/** Catch-up rounds per anchor, as the page's confirmOnEthereum. */
const CATCH_UP_ROUNDS = 3;
const WAITING_MS = 4000;
const ANCHOR_WAIT_MS = 36_000;
const ETHEREUM_SIGNED_SECS = 600;
const L2_SIGNED_SECS = 3 * 3600;

/** The anchor's failure as the client's error: a verdict, or why the block can't be anchored. */
export function anchorFailure(r: AnchorFailure, method?: string): Error {
  if (r.kind === 'rejected') return new WillowVerificationError('anchor-rejected', r.message, method);
  if (r.kind === 'malformed-input') return new WillowVerificationError('anchor-malformed', r.message, method);
  if (r.kind === 'internal') return new WillowVerificationError('internal', r.message, method);
  return new WillowUnverifiableError(`anchor-${r.kind}`, r.message, method);
}

const sleep = (ms: number) => new Promise((r) => setTimeout(r, ms));

/** The anchor module failing to run: missing is unverifiable, anything else (a trap) fails closed. */
function ran<T>(p: Promise<T>, method?: string): Promise<T> {
  return p.catch((e) => {
    if (e instanceof WillowVerificationError || e instanceof WillowUnverifiableError) throw e;
    const m = e instanceof Error ? e.message : String(e);
    throw isNoVerifier(e) ? new WillowUnverifiableError('no-verifier', m, method) : new WillowVerificationError('internal', m, method);
  });
}

/**
 * Section 2: one light client anchors every chain. Heads per chain from `willow_getBlockAnchor
 * ["latest"]` (and `["finalized"]` on Ethereum), specific blocks from `[n]`, each verified in the
 * anchor module and kept until its level expires.
 */
export class Anchors {
  private restored: Promise<void> | null = null;
  // What each light client last kept (by keyOf): it changes only when that client catches up.
  private kept = new Map<string, string>();
  // One verify (and its catch-up) at a time: catch-up resets and advances the one light client.
  private lock: Promise<unknown> = Promise.resolve();
  private cache = new Lru<Anchored>(8192);
  private byNumber = new Lru<string>(8192);
  private heads = new Map<string, { value: Anchored; at: number; started: number }>();
  private inflight = new Map<string, Promise<Anchored>>();

  constructor(private readonly o: AnchorsOptions) {}

  private nowSec = () => Math.floor(this.o.nowMs() / 1000);

  private restore(): Promise<void> {
    this.restored ??= (async () => {
      for (const lc of [undefined, 'sepolia', 'bnb'] as const) {
        const saved = await Promise.resolve(this.o.storage?.get(keyOf(lc))).catch(() => null);
        // Kept data is verified again from the start; a failure only means starting over.
        if (saved) await this.o.backend.anchorRestore(saved, this.nowSec(), lc).catch(() => undefined);
      }
    })();
    return this.restored;
  }

  /**
   * The light client's bootstrap: Ethereum's on chain 1's route, another's on the verified chain's
   * own route (where Willow serves that light client's data).
   */
  private async bootstrap(chainId: number, lc: LightClientName | undefined, method?: string): Promise<void> {
    const root = await this.o.checkpointRoot(lc);
    const reply = await this.o.rpc(lc ? chainId : 1, 'willow_lightClientBootstrap', [root]);
    if (reply.value.error) throw this.refused(reply, 'willow_lightClientBootstrap', method);
    const r = await ran(this.o.backend.anchorBootstrap(reply.text, this.nowSec(), lc), method);
    if (!r.ok) throw anchorFailure(r as AnchorFailure, method);
  }

  private async updates(chainId: number, lc: LightClientName | undefined, from: number, count: number, method?: string): Promise<void> {
    if (!Number.isSafeInteger(from) || !Number.isSafeInteger(count) || from < 0 || count < 1) {
      throw new WillowVerificationError('internal', `the light client asked for periods ${from} (${count})`, method);
    }
    for (let left = count; left > 0; ) {
      const k = Math.min(left, lc === 'bnb' ? BNB_UPDATES_PER_ASK : UPDATES_PER_ASK);
      const reply = await this.o.rpc(lc ? chainId : 1, 'willow_lightClientUpdates', [from, k]);
      if (reply.value.error) throw this.refused(reply, 'willow_lightClientUpdates', method);
      const r = await ran(this.o.backend.anchorUpdates(reply.text, lc), method);
      if (!r.ok) throw anchorFailure(r as AnchorFailure, method);
      if (r.period == null) throw new WillowUnverifiableError('anchor-unreachable', `Willow sent no light-client update for period ${from}`, method);
      from += k;
      left -= k;
    }
  }

  private refused(reply: Reply, what: string, method?: string): Error {
    const e = reply.value.error!;
    return new WillowUnverifiableError('anchor-refused', `${what}: ${String(e.message ?? '')} (${String(e.code)})`, method);
  }

  /** `Anchor.verify` of a `willow_getBlockAnchor` response body, fetching light-client data as it asks. */
  verify(chainId: number, number: number, hash: string, body: string, method?: string): Promise<AnchorVerified> {
    const run = async (): Promise<AnchorVerified> => {
      await this.restore();
      let caughtUp = false;
      const key = keyOf(lightClientOf(chainId));
      for (let round = 0; ; round++) {
        const wantSaved = !!this.o.storage && (caughtUp || !this.kept.has(key));
        const { result: r, saved } = await ran(this.o.backend.anchorVerify(chainId, number, hash, body, this.nowSec(), wantSaved), method);
        if (r.ok) {
          if (saved && saved !== 'null' && saved !== this.kept.get(key)) {
            this.kept.set(key, saved);
            void Promise.resolve(this.o.storage?.set(key, saved)).catch(() => undefined);
          }
          return r;
        }
        if (r.kind !== 'need-bootstrap' && r.kind !== 'need-updates') throw anchorFailure(r, method);
        if (round >= CATCH_UP_ROUNDS) {
          throw new WillowUnverifiableError('anchor-unreachable', "Willow's light-client data did not reach the validators who signed this block", method);
        }
        const lc = r.lightClient;
        if (r.kind === 'need-bootstrap') await this.bootstrap(chainId, lc, method);
        else await this.updates(chainId, lc, r.from as number, r.count as number, method);
        caughtUp = true;
      }
    };
    const p = this.lock.then(run, run);
    this.lock = p.catch(() => undefined);
    return p;
  }

  /**
   * A `willow_getBlockAnchor` response. A block newer than the signed head (the server says
   * `waiting`: the light client signs block N only in slot N+1) is asked for again until
   * `anchorWaitMs` has passed, then it is unverifiable (`anchor-waiting`, -32099).
   */
  async body(chainId: number, param: string, method?: string): Promise<Reply> {
    const started = Date.now();
    for (;;) {
      const reply = await this.o.rpc(chainId, 'willow_getBlockAnchor', [param]);
      const e = reply.value.error;
      if (!e) return reply;
      const data = e.data && typeof e.data === 'object' ? (e.data as { status?: unknown; retryAfterSeconds?: unknown }) : null;
      if (data?.status !== 'waiting') return reply;
      const bound = this.o.anchorWaitMs ?? ANCHOR_WAIT_MS;
      const left = bound - (Date.now() - started);
      if (left <= 0) {
        throw new WillowUnverifiableError('anchor-waiting', `block ${param} of chain ${chainId} was not anchored within ${bound} ms: ${String(e.message ?? '')}`, method);
      }
      const after = Number(data.retryAfterSeconds);
      const every = this.o.waitingMs ?? (after > 0 && after <= 60 ? after * 1000 : WAITING_MS);
      await sleep(Math.min(every, left));
    }
  }

  private expiry(chainId: number, r: AnchorVerified): number {
    const now = this.o.nowMs();
    if (r.level === 'final') return Infinity;
    if (r.level === 'committed') return now + this.o.headMaxAgeMs(chainId);
    if (chainId === 1) return r.headTime !== undefined ? (r.headTime + ETHEREUM_SIGNED_SECS) * 1000 : now + this.o.headMaxAgeMs(chainId);
    if (r.path === 'parlia') return r.anchorTime !== undefined ? (r.anchorTime + BNB_SIGNED_SECS) * 1000 : now + this.o.headMaxAgeMs(chainId);
    return r.anchorTime !== undefined ? (r.anchorTime + L2_SIGNED_SECS) * 1000 : now + this.o.headMaxAgeMs(chainId);
  }

  private keep(a: Anchored): void {
    this.cache.set(`${a.chainId}:${a.number}:${a.hash.toLowerCase()}`, a);
    this.byNumber.set(`${a.chainId}:${a.number}`, a.hash.toLowerCase());
  }

  /** The anchored hash of block `number`, when an anchor of it still counts (the head's included). */
  hashOf(chainId: number, number: number): string | undefined {
    const h = this.heads.get(`${chainId}:latest`)?.value;
    const now = this.o.nowMs();
    if (h && h.number === number && h.expires > now) return h.hash;
    const hash = this.byNumber.get(`${chainId}:${number}`);
    const a = hash ? this.cache.get(`${chainId}:${number}:${hash}`) : undefined;
    return a && a.expires > now ? a.hash : undefined;
  }

  /** Whether block `number` has an anchor that still counts (any hash): no anchor need be fetched ahead. */
  known(chainId: number, number: number): boolean {
    const h = this.heads.get(`${chainId}:latest`);
    if (h && h.value.number === number) return true;
    const hash = this.byNumber.get(`${chainId}:${number}`);
    const a = hash ? this.cache.get(`${chainId}:${number}:${hash}`) : undefined;
    return !!a && a.expires > this.o.nowMs();
  }

  /** The newest block (`latest`), or on Ethereum the finalized one, anchored at `minLevel` or above. */
  head(chainId: number, which: 'latest' | 'finalized', force = false, method?: string): Promise<Anchored> {
    // Ethereum's newest block at "final" is its finalized one; an L2 has no tag for a newer
    // block above "signed".
    const tag = which === 'latest' && chainId === 1 && this.o.minLevel !== 'signed' ? 'finalized' : which;
    if (which === 'latest' && chainId !== 1 && !LATEST_IS_FINAL.has(chainId) && this.o.minLevel !== 'signed') {
      return Promise.reject(new WillowUnverifiableError('anchor-level', `chain ${chainId} has no newest block anchored at ${this.o.minLevel}; name a block`, method));
    }
    const key = `${chainId}:${which}`;
    const h = this.heads.get(key);
    const now = this.o.nowMs();
    if (h && !force && now - h.at < this.o.headMaxAgeMs(chainId) && h.value.expires > now) return Promise.resolve(h.value);
    // An eth_blockNumber burst shares one refresh.
    if (h && force && now - h.started < 500 && h.value.expires > now) return Promise.resolve(h.value);
    const running = this.inflight.get(key);
    if (running) return running;
    const p = this.fetchHead(chainId, which, tag, method).finally(() => this.inflight.delete(key));
    this.inflight.set(key, p);
    return p;
  }

  private async fetchHead(chainId: number, which: 'latest' | 'finalized', tag: string, method?: string): Promise<Anchored> {
    const started = this.o.nowMs();
    const reply = await this.body(chainId, tag, method);
    if (reply.value.error) throw this.refused(reply, `willow_getBlockAnchor ["${tag}"]`, method);
    const n = toNumber((reply.value.result as { blockNumber?: unknown } | null)?.blockNumber);
    if (n === null) throw new WillowVerificationError('anchor-malformed', `the anchor for "${tag}" names no block number`, method);
    const r = await this.verify(chainId, n, ZERO_HASH, reply.text, method);
    let time: number | undefined;
    if (chainId === 1) {
      // Appendix D rule 3: Ethereum's newest block is the signed head itself.
      if (tag === 'latest' && r.path !== 'head') throw new WillowVerificationError('anchor-head', `"latest" was anchored by path ${r.path}, not the signed head`, method);
      // Rule 2: "finalized" is the light client's finalized block, at level final.
      if (tag === 'finalized' && (r.level !== 'final' || r.finalized !== n)) {
        throw new WillowVerificationError('anchor-head', `"finalized" named block ${n}, but the light client's finalized block is ${String(r.finalized)}`, method);
      }
      time = r.headTime;
    } else {
      // Rule 1: an L2's newest block is the newest its sequencer signed, aged by its own timestamp;
      // BNB's is its newest finalized block, aged the same way.
      time = r.path === 'sequencer' || r.path === 'parlia' ? r.anchorTime : undefined;
    }
    if (time === undefined) throw new WillowUnverifiableError('anchor-stale-head', `the anchor for "${tag}" on chain ${chainId} gives no time to bound its age`, method);
    const age = this.nowSec() - time;
    if (age > this.o.maxHeadAgeSec) throw new WillowUnverifiableError('anchor-stale-head', `the newest block Willow anchored on chain ${chainId} is ${age} s old`, method);
    if (!atLeast(r.level, this.o.minLevel)) throw new WillowUnverifiableError('anchor-level', `the newest block is ${r.level}, below ${this.o.minLevel}`, method);
    const value: Anchored = { chainId, number: n, hash: r.hash.toLowerCase(), level: r.level, path: r.path, time, expires: this.expiry(chainId, r) };
    this.keep(value);
    this.heads.set(`${chainId}:${which}`, { value, at: this.o.nowMs(), started });
    return value;
  }

  /**
   * Block `number` with `hash`, anchored at `minLevel` or above: the cached head or anchor, else
   * `willow_getBlockAnchor [n]` (`prefetched` when it was asked for with the proof).
   */
  block(chainId: number, number: number, hash: string, prefetched?: Promise<Reply>, method?: string): Promise<Anchored> {
    const h = hash.toLowerCase();
    const now = this.o.nowMs();
    const key = `${chainId}:${number}:${h}`;
    const head = this.heads.get(`${chainId}:latest`)?.value;
    if (head && head.number === number && head.hash === h && head.expires > now) return Promise.resolve(head);
    const c = this.cache.get(key);
    if (c && c.expires > now) return Promise.resolve(c);
    const running = this.inflight.get(key);
    if (running) return running;
    const p = this.fetchBlock(chainId, number, h, prefetched, method).finally(() => this.inflight.delete(key));
    this.inflight.set(key, p);
    return p;
  }

  private async fetchBlock(chainId: number, number: number, hash: string, prefetched: Promise<Reply> | undefined, method?: string): Promise<Anchored> {
    for (let attempt = 0; ; attempt++) {
      const reply = attempt === 0 && prefetched ? await prefetched : await this.body(chainId, toQuantity(number), method);
      if (reply.value.error) throw this.refused(reply, `willow_getBlockAnchor [${number}]`, method);
      let r: AnchorVerified;
      try {
        r = await this.verify(chainId, number, hash, reply.text, method);
      } catch (e) {
        // A stale signed head: refresh this chain's head, and ask once more (section 2).
        if (attempt === 0 && e instanceof WillowUnverifiableError && e.data.kind === 'anchor-stale-head') {
          this.heads.delete(`${chainId}:latest`);
          continue;
        }
        throw e;
      }
      if (!r.matches) throw new WillowVerificationError('anchor-mismatch', `block ${number} of chain ${chainId} is ${r.hash} on Ethereum, not ${hash}`, method);
      if (!atLeast(r.level, this.o.minLevel)) throw new WillowUnverifiableError('anchor-level', `block ${number} is ${r.level}, below ${this.o.minLevel}`, method);
      const a: Anchored = { chainId, number, hash, level: r.level, path: r.path, time: r.anchorTime ?? r.headTime, expires: this.expiry(chainId, r) };
      this.keep(a);
      return a;
    }
  }

  /** A verified block's parent is in the same anchored history: its hash counts at the child's level. */
  parent(child: Anchored, parentHash: string): void {
    if (child.number === 0 || !isHash(parentHash)) return;
    const c = this.cache.get(`${child.chainId}:${child.number - 1}:${parentHash.toLowerCase()}`);
    if (c && c.expires >= child.expires) return;
    this.keep({ ...child, number: child.number - 1, hash: parentHash.toLowerCase() });
  }
}
