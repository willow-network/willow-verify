// I6 implementations for tests, until W5 builds the wasm. They verify for real what is cheap in
// TypeScript (the chain, the header hashing to the block hash and naming its number, the block
// asked for, the fee headers' parent links, a stand-in signature on mock anchors) and the rest by
// equality with the fixtures, reporting the kind W1's and W2's own tamper suites expect.
import { readFileSync } from 'node:fs';
import { pathToFileURL } from 'node:url';
import { hexToBytes, isAddress, isHash, keccakHex, toNumber, toQuantity } from '../../src/hex.js';
import type {
  Anchor,
  AnchorConstructor,
  ItemKind,
  ItemVerifier,
  LogsVerifier,
  StateKind,
  StateVerifier,
  VerifierModules,
} from '../../src/verifiers.js';
import { anchorSig, header, World } from './world.js';

const out = (v: unknown) => JSON.stringify(v);
const fail = (kind: string, message: string, extra: Record<string, unknown> = {}) => out({ ok: false, kind, message, ...extra });

type Path = (string | number)[];

/** The first path where `a` and `b` differ, or null. */
export function diffPath(a: unknown, b: unknown, path: Path = []): Path | null {
  if (a === b) return null;
  if (typeof a !== typeof b || a === null || b === null || typeof a !== 'object') {
    if (typeof a === 'string' && typeof b === 'string' && a.toLowerCase() === b.toLowerCase()) return null;
    return path;
  }
  if (Array.isArray(a) !== Array.isArray(b)) return path;
  if (Array.isArray(a)) {
    const bb = b as unknown[];
    if (a.length !== bb.length) return [...path, 'length'];
    for (let i = 0; i < a.length; i++) {
      const d = diffPath(a[i], bb[i], [...path, i]);
      if (d) return d;
    }
    return null;
  }
  const ao = a as Record<string, unknown>;
  const bo = b as Record<string, unknown>;
  for (const k of [...new Set([...Object.keys(ao), ...Object.keys(bo)])].sort()) {
    if (!(k in ao) || !(k in bo)) return [...path, k, 'absent'];
    const d = diffPath(ao[k], bo[k], [...path, k]);
    if (d) return d;
  }
  return null;
}

function parseBlockText(s: string): { number?: number; hash?: string; tag?: true } | null {
  if (s === 'latest') return { tag: true };
  if (isHash(s)) return { hash: s.toLowerCase() };
  const n = toNumber(s);
  return n === null ? null : { number: n };
}

/** W1's fault for a difference in an items proof (tests_block.rs, tests_fees.rs). */
function itemFault(kind: string, p: Path, r?: any): string {
  const [top, ...rest] = p;
  const absent = rest[rest.length - 1] === 'absent' && rest.length === 1;
  if (kind === 'fees' || kind === 'feeHistory') {
    if (top === 'formula') return 'unknown-formula';
    if (top === 'headers') return rest[0] === 'length' ? 'wrong-range' : 'header';
    if (top === 'bodies') {
      if (rest[0] === 'length' || absent) return 'missing';
      if (rest.includes('receipts') || rest.includes('receiptProofs')) return 'receipts';
      if (rest.includes('transactions')) return 'transactions';
      return 'body';
    }
    return 'malformed';
  }
  if (kind === 'transaction' || kind === 'receipt') {
    if (top === 'transactionProofs' || top === 'transactions') return 'transactions';
    if (top === 'receiptProofs' || top === 'receipts') return 'receipts';
    return 'wrong-item';
  }
  if (top === 'transactions' || top === 'transactionProofs') return absent ? 'missing' : 'transactions';
  if (top === 'withdrawals') return absent ? 'missing' : 'withdrawals';
  // Uncles left unproven where there are none: uncles; where the proof gives them too: malformed.
  if (top === 'unproven') return r?.uncles ? 'malformed' : 'uncles';
  if (top === 'uncles') return 'uncles';
  if (top === 'receiptProofs' || top === 'receipts') return absent ? 'missing' : 'receipts';
  return 'malformed';
}

/** W2's kind for a difference in a state, call or estimate proof (state-verify tests.rs). */
function stateFault(kind: string, p: Path): string {
  const [top, ...rest] = p;
  if (kind === 'state') {
    if (top === 'code') return 'code';
    if (top === 'account') return rest[0] === 'storageProof' ? 'slot' : 'account';
    return 'malformed';
  }
  if (top === 'algorithm') return 'algorithm';
  if (top === 'estimate') return 'estimate';
  if (top === 'result') return 'outcome';
  if (top === 'witness') {
    if (rest[0] === 'codes') return 'code';
    if (rest[0] === 'nodes') return 'account';
    if (rest[0] === 'accounts') {
      if (rest[1] === 'length' || rest.includes('absent')) return 'missing-read';
      return rest.includes('storageProof') && rest.includes('value') ? 'slot' : 'account';
    }
  }
  return 'malformed';
}

/** What every proof verifies of its block: chain, header, the block asked for. Null when it passes. */
function named(chainId: number, r: any, headerHex: unknown, want: { number?: number; hash?: string } | undefined, fault: (k: string, m: string) => string): string | null {
  if (toNumber(r.chainId) !== chainId) return fault('wrong-chain', `the proof is for chain ${r.chainId}, not ${chainId}`);
  if (typeof headerHex !== 'string') return fault('malformed', 'no header');
  let h;
  try {
    h = header(headerHex);
  } catch {
    return fault('header', 'the header does not decode');
  }
  if (h.hash !== String(r.blockHash).toLowerCase()) return fault('header', 'blockHash is not the keccak of the header');
  if (h.number !== toNumber(r.blockNumber)) return fault('header', "blockNumber is not the header's");
  if (want?.hash && want.hash !== h.hash) return fault('wrong-block', `the proof is for block ${h.hash}, not ${want.hash}`);
  if (want?.number !== undefined && want.number !== h.number) return fault('wrong-block', `the proof is for block ${h.number}, not ${want.number}`);
  return null;
}

export function mockItems(world: World): new (chainId: number) => ItemVerifier {
  return class MockItemVerifier implements ItemVerifier {
    private s: { kind: ItemKind; method: string; params: unknown[]; block?: { number?: number; hash?: string } } | null = null;
    private last: any = null;
    constructor(private readonly chainId: number) {}

    request(kind: ItemKind, target: string): string {
      this.s = null;
      this.last = null;
      const bad = (m: string) => fail('invalid-request', m);
      let method: string;
      let params: unknown[];
      let block: { number?: number; hash?: string } | undefined;
      if (kind === 'transaction' || kind === 'receipt') {
        method = kind === 'transaction' ? 'willow_getTransactionProof' : 'willow_getReceiptProof';
        const i = target.lastIndexOf(':');
        if (i > 0) {
          const b = parseBlockText(target.slice(0, i));
          const idx = toNumber(target.slice(i + 1));
          if (!b || idx === null) return bad(`"${target}" is not <block>:<index>`);
          params = [b.hash ?? (b.tag ? 'latest' : toQuantity(b.number!)), toQuantity(idx)];
          block = b.tag ? undefined : b;
        } else if (isHash(target)) params = [target.toLowerCase()];
        else return bad(`"${target}" is not a transaction hash`);
      } else if (kind === 'blockReceipts' || kind === 'block' || kind === 'fees') {
        method = { blockReceipts: 'willow_getBlockReceiptsProof', block: 'willow_getBlockProof', fees: 'willow_getFeeProof' }[kind];
        const b = parseBlockText(target);
        if (!b) return bad(`"${target}" is not a block`);
        params = [b.hash ?? (b.tag ? 'latest' : toQuantity(b.number!))];
        block = b.tag ? undefined : b;
      } else if (kind === 'feeHistory') {
        method = 'willow_getFeeHistoryProof';
        let p: any;
        try {
          p = JSON.parse(target);
        } catch {
          return bad('not JSON');
        }
        const count = toNumber(p?.[0]);
        const pct = p?.[2] ?? [];
        if (count === null || count < 1 || typeof p[1] !== 'string' || !Array.isArray(pct)) return bad('expected [blockCount, newestBlock, rewardPercentiles]');
        if (pct.some((x: unknown, i: number) => typeof x !== 'number' || x < 0 || x > 100 || (i > 0 && x <= pct[i - 1]))) return bad('invalid reward percentile');
        const c = Math.min(count, 1024);
        if (pct.length && this.chainId !== 42161 && this.chainId !== 4663 && c > 16) return bad(`a fee-history proof with rewards covers at most 16 blocks, not ${c}`);
        params = [toQuantity(c), p[1], pct];
        const b = parseBlockText(p[1]);
        block = b && !b.tag ? b : undefined;
      } else return fail('internal', `unknown proof kind ${kind}`);
      this.s = { kind, method, params, block };
      return out({ ok: true, method, params });
    }

    verify(response: string): string {
      this.last = null;
      const s = this.s;
      if (!s) return fail('internal', 'verify called before request');
      let r: any;
      try {
        r = JSON.parse(response).result;
      } catch {
        return fail('malformed-input', 'not JSON');
      }
      if (!r || typeof r !== 'object') return fail('malformed-input', 'no result');
      const rejected = (fault: string, message: string) => fail('rejected', message, { fault });
      const fee = s.kind === 'fees' || s.kind === 'feeHistory';
      if (fee && Array.isArray(r.headers)) {
        for (let i = 1; i < r.headers.length; i++) {
          try {
            if (header(r.headers[i]).parentHash !== keccakHex(hexToBytes(r.headers[i - 1]))) return rejected('broken-link', `header ${i} is not its predecessor's child`);
          } catch {
            return rejected('header', `header ${i} does not decode`);
          }
        }
      }
      const e = named(this.chainId, r, fee ? r.headers?.[r.headers.length - 1] : r.header, s.block, rejected);
      if (e) return e;
      const entry = world.proofs.get(world.key(this.chainId, s.method, s.params));
      if (!entry) return rejected('missing', 'no such proof in the fixtures');
      const d = diffPath(r, entry.result);
      if (d) return rejected(itemFault(s.kind, d, r), `${d.join('.')} differs from the fixture`);
      this.last = entry;
      const o: any = { ok: true, report: entry.report, proven: entry.proven };
      if (entry.provenTransactions) o.provenTransactions = entry.provenTransactions;
      return out(o);
    }

    block(full: boolean): string {
      const e = this.last;
      if (!e || e.kind !== 'block') return fail('internal', 'no verified block proof');
      return out({ ok: true, proven: full ? e.full.proven : e.proven, unproven: e.full.unproven });
    }

    compare(): string {
      return fail('internal', 'compare is not in the mock');
    }

    free(): void {}
  };
}

export function mockState(world: World): new (chainId: number) => StateVerifier {
  return class MockStateVerifier implements StateVerifier {
    private s: { kind: StateKind; method: string; params: any[] } | null = null;
    constructor(private readonly chainId: number) {}

    request(kind: StateKind, target: string): string {
      this.s = null;
      let p: any;
      try {
        p = JSON.parse(target);
      } catch {
        return fail('invalid-request', 'not JSON');
      }
      if (!Array.isArray(p)) return fail('invalid-request', 'expected a list of params');
      if (kind === 'state') {
        if (!isAddress(p[0]) || !Array.isArray(p[1]) || p[1].length > 256) return fail('invalid-request', 'expected [address, storageKeys, block, {code}]');
        this.s = { kind, method: 'willow_getStateProof', params: p };
      } else if (kind === 'call' || kind === 'estimate') {
        if (this.chainId !== 1) return fail('unsupported', `calls are verified on Ethereum only, not chain ${this.chainId}`);
        if (!p[0] || typeof p[0] !== 'object' || p.slice(2).some((x: unknown) => x && typeof x === 'object' && Object.keys(x).length)) return fail('invalid-request', 'a call with state or block overrides is not proven');
        this.s = { kind, method: kind === 'call' ? 'willow_getCallProof' : 'willow_getEstimateGasProof', params: p };
      } else return fail('internal', `unknown kind ${kind}`);
      return out({ ok: true, method: this.s.method, params: p });
    }

    verify(response: string): string {
      const s = this.s;
      if (!s) return fail('internal', 'verify called before request');
      let r: any;
      try {
        r = JSON.parse(response).result;
      } catch {
        return fail('malformed', 'not JSON');
      }
      if (!r || typeof r !== 'object') return fail('malformed', 'no result');
      const k = (kind: string, message: string) => fail(kind, message);
      if (s.kind === 'estimate' && r.algorithm !== 'geth-v1.17.10-estimate') return k('algorithm', `an estimate by ${String(r.algorithm)}`);
      const b = s.kind === 'state' ? s.params[2] : s.params[1];
      const want = (() => {
        const o = b && typeof b === 'object' ? (b.blockHash ?? b.blockNumber) : b;
        const n = toNumber(o);
        return isHash(o) ? { hash: String(o).toLowerCase() } : n !== null ? { number: n } : undefined;
      })();
      const e = named(this.chainId, r, r.header, want, k);
      if (e) return e;
      let expected: { result: any; proven: any; report: any } | null;
      if (s.kind === 'state') expected = world.state(this.chainId, s.params);
      else {
        const entry = world.proofs.get(world.key(this.chainId, s.method, [s.params[0], world.blockHash(this.chainId, b) ?? b]));
        expected = entry ? { result: entry.result, proven: entry.proven, report: entry.report } : null;
      }
      if (!expected) return k('malformed', 'no such proof in the fixtures');
      const d = diffPath(r, expected.result);
      if (d) return k(stateFault(s.kind, d), `${d.join('.')} differs from the fixture`);
      return out({ ok: true, report: expected.report, proven: expected.proven });
    }

    compare(): string {
      return fail('internal', 'compare is not in the mock');
    }

    free(): void {}
  };
}

/** The mock anchor: verifies the stand-in signature, the block, the chain and the age, as Anchor.verify does for real. */
export class MockAnchor implements Anchor {
  constructor(_root: string, _slot: number) {}
  restore(): string {
    return out({ ok: true, period: 0 });
  }
  bootstrap(): string {
    return out({ ok: true, period: 0 });
  }
  updates(): string {
    return out({ ok: true, period: 1 });
  }
  verify(chainId: number, number: number, hash: string, response: string, now: number): string {
    let r: any;
    try {
      r = JSON.parse(response).result;
    } catch {
      return fail('malformed-input', 'not JSON');
    }
    const m = r?.mock;
    if (!m) return fail('malformed-input', 'not a mock anchor');
    if (anchorSig(m) !== r.sig) return fail('rejected', "the validators' signature does not verify");
    if (toNumber(r.blockNumber) !== m.number || m.chainId !== chainId) return fail('rejected', 'the anchor is for another chain or block');
    if (m.number !== number) return fail('rejected', `the anchor is for block ${m.number}, not ${number}`);
    if (m.level === 'signed' && now - m.time > (chainId === 1 ? 600 : 3 * 3600)) return fail('stale-head', `the signed head is ${Math.round((now - m.time) / 60)} minutes old`);
    const o: any = { ok: true, level: m.level, path: m.path, hash: m.hash, matches: m.hash === hash.toLowerCase() };
    if (chainId === 1) Object.assign(o, { headTime: m.time, head: m.path === 'head' ? number : number + 32, finalized: m.level === 'final' ? number : number - 64, signers: 500 });
    else Object.assign(o, { anchorBlock: number, l1Block: 1, ...(m.path === 'sequencer' ? { anchorTime: m.time } : {}) });
    return out(o);
  }
  saved(): string {
    return '[]';
  }
  reset(): void {}
  free(): void {}
}

/** Logs proofs by filter: the proof bytes must be the registered ones. */
export function mockLogs(world: World & { logs?: Map<string, { proofs: Record<string, string>; result: any; verified: any }> }): new (chainId: number, s: string, l: string) => LogsVerifier {
  return class MockLogsVerifier implements LogsVerifier {
    constructor(private readonly chainId: number) {}
    keys(): string {
      return out({ ok: true, pinned: { openvm: { leaf: [], segment: [] } } });
    }
    verify(response: string, filter: string, getProof: (id: string) => Uint8Array | undefined): string {
      const e = world.logs?.get(`${this.chainId}|${filter}`);
      if (!e) return fail('rejected', 'no such logs proof in the fixtures');
      let r: any;
      try {
        r = JSON.parse(response).result;
      } catch {
        return fail('malformed-input', 'not JSON');
      }
      if (diffPath(r, e.result)) return fail('rejected', 'the answer differs from the fixture');
      for (const [id, hex] of Object.entries(e.proofs)) {
        const b = getProof(id);
        if (!b || keccakHex(b) !== keccakHex(hexToBytes(hex))) return fail('rejected', `proof ${id} does not verify`);
      }
      return out({ ok: true, report: e.verified.report, logs: e.verified.logs });
    }
    compare(): string {
      return fail('internal', 'compare is not in the mock');
    }
    free(): void {}
  };
}

export interface Mocks {
  items?: 'mock' | 'none' | VerifierModules['items'];
  state?: 'mock' | 'none' | VerifierModules['state'];
  anchor?: 'mock' | VerifierModules['anchor'];
  logs?: 'mock' | VerifierModules['logs'];
}

export function mockModules(world: World, m: Mocks = {}): VerifierModules {
  const pick = <T>(v: 'mock' | 'none' | (() => Promise<T>) | undefined, mock: () => Promise<T>) => (v === 'none' ? undefined : v === undefined || v === 'mock' ? mock : v);
  return {
    items: pick(m.items, async () => ({ ItemVerifier: mockItems(world) })),
    state: pick(m.state, async () => ({ StateVerifier: mockState(world) })),
    anchor: pick(m.anchor, async () => ({ Anchor: MockAnchor as AnchorConstructor })),
    logs: pick(m.logs, async () => ({ Verifier: mockLogs(world as any) })),
  };
}

/** The verify page build to load real modules from (WILLOW_VERIFY_DIST), or null: tests that need it skip. */
export const VERIFY_DIST = process.env.WILLOW_VERIFY_DIST ?? null;

/** A real module of a page build, by name (anchor, items, state, verifier): its glue's exports, initialised. */
export async function realModule(name: string): Promise<any> {
  const { readdirSync } = await import('node:fs');
  const { join } = await import('node:path');
  if (!VERIFY_DIST) throw new Error('WILLOW_VERIFY_DIST is not set');
  const files = readdirSync(VERIFY_DIST);
  const glue = files.find((f) => new RegExp(`^${name}\\.[0-9a-f]+\\.js$`).test(f));
  const wasm = files.find((f) => new RegExp(`^${name}\\.[0-9a-f]+\\.wasm$`).test(f));
  if (!glue || !wasm) throw new Error(`no ${name} module in ${VERIFY_DIST}`);
  const m = await import(pathToFileURL(join(VERIFY_DIST, glue)).href);
  m.initSync({ module: readFileSync(join(VERIFY_DIST, wasm)) });
  return m;
}

/**
 * The verifiers a test's proofs go through: with WILLOW_VERIFY_DIST, the page build's real items
 * and state modules (anchors and logs stay the mock's: the mock server can't sign for Ethereum's
 * validators, and holds no log proofs); else the mocks. `m` overrides either, as for mockModules.
 */
export function proofModules(world: World, m: Mocks = {}): VerifierModules {
  if (!VERIFY_DIST) return mockModules(world, m);
  const items = async () => ({ ItemVerifier: (await realModule('items')).ItemVerifier });
  const state = async () => ({ StateVerifier: (await realModule('state')).StateVerifier });
  return mockModules(world, { items, state, ...m });
}

/**
 * The real Anchor with W4's `anchorTime` added to sequencer results where the build predates W4
 * (the 10/21 page build does): the value is the I5 fixture's own `expect.anchorTime`.
 */
export function withAnchorTime(Real: AnchorConstructor, anchorTime: (chainId: number) => number | undefined): AnchorConstructor {
  return class implements Anchor {
    private a: Anchor;
    constructor(root: string, slot: number) {
      this.a = new Real(root, slot);
    }
    restore(s: string, n: number) {
      return this.a.restore(s, n);
    }
    bootstrap(r: string, n: number) {
      return this.a.bootstrap(r, n);
    }
    updates(r: string) {
      return this.a.updates(r);
    }
    verify(c: number, n: number, h: string, r: string, now: number) {
      const o = JSON.parse(this.a.verify(c, n, h, r, now));
      if (o.ok && o.path === 'sequencer' && o.anchorTime === undefined && anchorTime(c) !== undefined) o.anchorTime = anchorTime(c);
      return JSON.stringify(o);
    }
    saved() {
      return this.a.saved();
    }
    reset() {
      this.a.reset();
    }
    free() {
      this.a.free();
    }
  };
}
