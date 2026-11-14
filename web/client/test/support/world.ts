// The fixtures as one world the mock server serves and the mock verifiers verify against.
import { readFileSync, readdirSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { bytesToHex, hexToBytes, keccakHex, slotKey, toNumber, toQuantity } from '../../src/hex.js';
import { decode, uint, type Rlp } from '../../src/rlp.js';

const here = dirname(fileURLToPath(import.meta.url));
export const FIXTURES = join(here, '..', 'fixtures');
/** The shipped transaction, receipt and block-receipts proofs (web/verify/test/items, master). */
export const ITEMS = join(here, '..', '..', '..', 'verify', 'test', 'items');

export const json = (path: string): any => JSON.parse(readFileSync(path, 'utf8'));
export const fixture = (rel: string): any => json(join(FIXTURES, rel));
export const fixtureNames = (dir: string): string[] => readdirSync(join(FIXTURES, dir)).filter((f) => f.endsWith('.json')).sort();

/** A JSON value with hex lowercased, `{blockHash}` read as the hash, keys sorted, nulls dropped: for matching requests. */
export function canon(v: unknown): unknown {
  if (typeof v === 'string') return /^0x[0-9a-fA-F]*$/.test(v) ? v.toLowerCase() : v;
  if (Array.isArray(v)) return v.map(canon);
  if (v && typeof v === 'object') {
    const o = v as Record<string, unknown>;
    if (Object.keys(o).length === 1 && typeof o.blockHash === 'string') return canon(o.blockHash);
    const out: Record<string, unknown> = {};
    for (const k of Object.keys(o).sort()) if (o[k] !== null && o[k] !== undefined) out[k] = canon(o[k]);
    return out;
  }
  return v;
}

export interface Header {
  parentHash: string;
  number: number;
  timestamp: number;
  hash: string;
}

export function header(rlpHex: string): Header {
  const h = decode(hexToBytes(rlpHex)) as Rlp[];
  return {
    parentHash: bytesToHex(h[0] as Uint8Array),
    number: Number(uint(h[8])),
    timestamp: Number(uint(h[11])),
    hash: keccakHex(hexToBytes(rlpHex)),
  };
}

export interface Entry {
  chainId: number;
  /** items: block, fees, feeHistory, transaction, receipt, blockReceipts; state: state, call, estimate. */
  kind: string;
  method: string;
  result: any;
  proven: any;
  report: any;
  full?: { proven: unknown; unproven: string[] };
  provenTransactions?: unknown[];
}

export interface AnchorEntry {
  chainId: number;
  number: number;
  hash: string;
  level: 'signed' | 'committed' | 'final';
  path: string;
  /** Seconds. */
  time: number;
}

/** The mock anchor's stand-in for a signature: the proxy can't make one, as it can't sign for Ethereum's validators. */
export const ANCHOR_SECRET = 'mock-validators';
export const anchorSig = (m: object): string => keccakHex(new TextEncoder().encode(ANCHOR_SECRET + JSON.stringify(canon(m))));

export class World {
  /** Proof responses by `${chainId}|${method}|${canon(params)}`. */
  proofs = new Map<string, Entry>();
  /** State fixtures by `${chainId}|${address}|${blockHash}`; the server serves any subset of their keys. */
  states = new Map<string, any>();
  /** Block hashes by `${chainId}:${number}`, for requests by number. */
  hashes = new Map<string, string>();
  heads = new Map<number, AnchorEntry>();
  finalized = new Map<number, AnchorEntry>();
  anchors = new Map<string, AnchorEntry>();
  /** Standard methods: `${chainId}|${method}|${canon(params)}` or `${chainId}|${method}|*`. */
  standard = new Map<string, { result?: unknown; error?: { code: number; message: string; data?: unknown } }>();
  /** Real anchor bodies (I5) by `${chainId}:${param}`, served as they are. */
  realAnchors = new Map<string, unknown>();
  lightClient: { bootstrap?: unknown; updates?: unknown } = {};
  /** Every HTTP request the server saw. */
  log: { chainId: number; items: { method: string; params: unknown }[] }[] = [];
  nowSec = 1_794_400_000;
  /** A test's own response to a request, before the fixtures. */
  intercept?: (chainId: number, method: string, params: unknown[]) => { result: unknown } | { error: { code: number; message: string; data?: unknown } } | undefined;

  key(chainId: number, method: string, params: unknown): string {
    // A call is matched on what decides it in the fixtures (libraries add fee, nonce and gas fields).
    if ((method === 'willow_getCallProof' || method === 'willow_getEstimateGasProof') && Array.isArray(params) && params[0] && typeof params[0] === 'object') {
      const c = params[0] as Record<string, unknown>;
      params = [{ from: c.from, to: c.to, value: c.value && c.value !== '0x0' ? c.value : undefined, data: c.data ?? c.input }, ...params.slice(1)];
    }
    return `${chainId}|${method}|${JSON.stringify(canon(params))}`;
  }

  blockHash(chainId: number, b: unknown): string | null {
    const c = canon(b);
    if (typeof c === 'string' && c.length === 66) return c;
    const n = toNumber(c);
    return n === null ? null : (this.hashes.get(`${chainId}:${n}`) ?? null);
  }

  addProof(params: unknown, e: Entry): void {
    this.proofs.set(this.key(e.chainId, e.method, params), e);
  }

  head(chainId: number, a: Omit<AnchorEntry, 'chainId' | 'time' | 'level' | 'path'> & Partial<AnchorEntry>): AnchorEntry {
    const e: AnchorEntry = { chainId, level: 'signed', path: chainId === 1 ? 'head' : 'sequencer', time: this.nowSec - 2, ...a };
    this.heads.set(chainId, e);
    this.anchor(e);
    return e;
  }

  anchor(e: AnchorEntry): void {
    this.anchors.set(`${e.chainId}:${e.number}`, e);
    this.hashes.set(`${e.chainId}:${e.number}`, e.hash.toLowerCase());
  }

  /** A mock anchor body: `{blockNumber, mock, sig}`. */
  anchorBody(e: AnchorEntry): unknown {
    const mock = { chainId: e.chainId, number: e.number, hash: e.hash.toLowerCase(), level: e.level, path: e.path, time: e.time };
    return { blockNumber: toQuantity(e.number), mock, sig: anchorSig(mock) };
  }

  // ---------- loaders ----------

  /** A W1 block fixture: willow_getBlockProof by hash and by number, rendered as publicnode's block. */
  addBlock(f: any, opts: { anchor?: Partial<AnchorEntry> | false } = {}): { number: number; hash: string; timestamp: number } {
    const r = f.result;
    const h = header(r.header);
    const node = structuredClone(f.node.eth_getBlockByNumber);
    delete node.totalDifficulty;
    const unproven: string[] = r.unproven ?? [];
    for (const k of unproven) delete node[k];
    const full = structuredClone(node);
    node.transactions = node.transactions.map((t: any) => (typeof t === 'string' ? t : t.hash));
    const entry: Entry = {
      chainId: f.chainId,
      kind: 'block',
      method: 'willow_getBlockProof',
      result: r,
      proven: node,
      report: { method: 'willow_getBlockProof', chainId: f.chainId, block: { number: r.blockNumber, hash: r.blockHash, timestamp: h.timestamp, parentHash: h.parentHash }, unproven, transactions: node.transactions.length },
      full: { proven: full, unproven },
    };
    this.addProof([r.blockHash], entry);
    this.addProof([r.blockNumber], entry);
    this.hashes.set(`${f.chainId}:${h.number}`, r.blockHash.toLowerCase());
    if (opts.anchor !== false) this.anchor({ chainId: f.chainId, number: h.number, hash: r.blockHash, level: 'signed', path: f.chainId === 1 ? 'history' : 'sequencer', time: this.nowSec - 5, ...opts.anchor });
    return { number: h.number, hash: r.blockHash.toLowerCase(), timestamp: h.timestamp, parentHash: h.parentHash };
  }

  /** A W1 fee fixture: willow_getFeeProof at H. */
  addFees(f: any): void {
    const r = f.result;
    const h = header(r.headers[r.headers.length - 1]);
    const entry: Entry = {
      chainId: f.chainId,
      kind: 'fees',
      method: 'willow_getFeeProof',
      result: r,
      proven: f.verified,
      report: { method: 'willow_getFeeProof', chainId: f.chainId, block: { number: r.blockNumber, hash: r.blockHash, timestamp: h.timestamp, parentHash: h.parentHash }, unproven: [], formula: r.formula },
    };
    this.addProof([r.blockHash], entry);
    this.addProof([r.blockNumber], entry);
  }

  /** A W1 fee-history fixture, by its recorded params. */
  addFeeHistory(f: any): void {
    const r = f.result;
    const h = header(r.headers[r.headers.length - 1]);
    this.addProof(f.request.params, {
      chainId: f.chainId,
      kind: 'feeHistory',
      method: 'willow_getFeeHistoryProof',
      result: r,
      proven: f.verified,
      report: { method: 'willow_getFeeHistoryProof', chainId: f.chainId, block: { number: r.blockNumber, hash: r.blockHash, timestamp: h.timestamp, parentHash: h.parentHash }, unproven: [], formula: r.formula },
    });
  }

  /** A W2 state fixture (any subset of its keys is served). */
  addState(f: any, chainId = 1): void {
    const r = f.result;
    this.states.set(`${chainId}|${r.account.address.toLowerCase()}|${r.blockHash.toLowerCase()}`, f);
    this.hashes.set(`${chainId}:${toNumber(r.blockNumber)}`, r.blockHash.toLowerCase());
  }

  /** A W2 call or estimate fixture. */
  addCall(f: any, chainId = 1): void {
    const r = f.result;
    const [call] = f.request.params;
    const kind = f.request.method === 'willow_getCallProof' ? 'call' : 'estimate';
    const h = header(r.header);
    const entry: Entry = {
      chainId,
      kind,
      method: f.request.method,
      result: r,
      proven: f.expect,
      report: { method: f.request.method, chainId, block: { number: r.blockNumber, hash: r.blockHash, timestamp: h.timestamp, parentHash: h.parentHash }, ...(r.algorithm ? { algorithm: r.algorithm } : {}) },
    };
    this.addProof([call, r.blockHash], entry);
    this.hashes.set(`${chainId}:${h.number}`, r.blockHash.toLowerCase());
  }

  /** A shipped items fixture (web/verify/test/items): a transaction, receipt or block-receipts proof. */
  addItem(c: any, opts: { anchor?: Partial<AnchorEntry> | false } = {}): void {
    const h = header(c.proof.result.header);
    const extra: Partial<Entry> = {};
    if (c.kind === 'blockReceipts') extra.provenTransactions = c.answers[1]?.result?.transactions;
    const entry: Entry = {
      chainId: c.chainId,
      kind: c.kind,
      method: c.proof.method,
      result: c.proof.result,
      proven: c.answers[0].result,
      report: { method: c.proof.method, chainId: c.chainId, block: { number: c.block.number, hash: c.block.hash, timestamp: h.timestamp, parentHash: h.parentHash }, unproven: [] },
      ...extra,
    };
    this.addProof(c.proof.params, entry);
    if (c.kind === 'blockReceipts') this.addProof([c.block.number], entry);
    this.hashes.set(`${c.chainId}:${h.number}`, c.block.hash.toLowerCase());
    if (opts.anchor !== false) this.anchor({ chainId: c.chainId, number: h.number, hash: c.block.hash, level: 'final', path: c.chainId === 1 ? 'beacon' : 'game', time: this.nowSec - 3600, ...opts.anchor });
  }

  /**
   * A state, call or estimate fixture moved to another block of the same chain: its block
   * fields replaced by that block's (the account and witness kept). For library flows that need
   * one head, as W1 and W2 recorded at different blocks.
   */
  rebase(f: any, onto: any): any {
    const g = structuredClone(f);
    for (const k of ['chainId', 'blockNumber', 'blockHash', 'header']) g.result[k] = onto.result[k];
    const p = g.request.params;
    for (let i = 0; i < p.length; i++) if (p[i] && typeof p[i] === 'object' && 'blockHash' in p[i]) p[i] = { blockHash: onto.result.blockHash };
    return g;
  }

  // ---------- state, as the server and the mock verifier derive it ----------

  /** The state proof result and proven responses for `[address, keys, block, {code}]`, or null. */
  state(chainId: number, params: unknown[]): { result: any; proven: any; report: any } | null {
    const [address, keys, b, opts] = params as [string, string[], unknown, { code?: boolean } | undefined];
    const hash = this.blockHash(chainId, b);
    if (!hash || typeof address !== 'string' || !Array.isArray(keys)) return null;
    const f = this.states.get(`${chainId}|${address.toLowerCase()}|${hash}`);
    if (!f) return null;
    const code = !!opts?.code;
    const r = f.result;
    const byKey = new Map<string, any>(r.account.storageProof.map((s: any) => [slotKey(s.key), s]));
    const values = new Map<string, string>(r.account.storageProof.map((s: any, i: number) => [slotKey(s.key), f.expect.eth_getStorageAt[i]]));
    if (!keys.every((k) => byKey.has(slotKey(k)))) return null;
    if (code && f.expect.eth_getCode === null) return null;
    const storageProof = keys.map((k) => ({ ...byKey.get(slotKey(k)), key: k }));
    const result: any = { ...r, account: { ...r.account, storageProof } };
    delete result.code;
    if (code && r.code !== undefined) result.code = r.code;
    const proven = {
      ...f.expect,
      eth_getCode: code ? f.expect.eth_getCode : null,
      eth_getStorageAt: keys.map((k) => values.get(slotKey(k))),
      eth_getProof: { ...f.expect.eth_getProof, storageProof },
    };
    const h = header(r.header);
    return { result, proven, report: { method: 'willow_getStateProof', chainId, block: { number: r.blockNumber, hash: r.blockHash, timestamp: h.timestamp, parentHash: h.parentHash } } };
  }
}

