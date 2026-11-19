// A made-up Stellar chain for the Stellar client's tests: a mock stellar-rpc server (the plain
// methods with stellar-rpc's pagination, cursors and event order) and `willow_getLedgerProof`
// bundles whose "XDR" is base64 JSON, and a fake Stellar module that checks those bundles the way
// the real one checks real ones: the header chain by hash, the results, preimages and transaction
// set against the header's commitments, and a stand-in quorum of signatures over the last hash.
import { createHash } from 'node:crypto';
import type { StellarVerifier } from '../../src/verifiers.js';

export const PUBNET = 'Public Global Stellar Network ; September 2015';
export const CONTRACT_A = `CA${'A'.repeat(54)}`;
export const CONTRACT_B = `CB${'B'.repeat(54)}`;
export const XLM_SAC = `CX${'X'.repeat(54)}`;
export const QSET = b64('tier1-qset');

export function b64(s: string): string {
  return Buffer.from(s).toString('base64');
}
const unb64 = (s: string) => Buffer.from(s, 'base64').toString();
export const sha = (s: string) => createHash('sha256').update(s).digest('hex');
export const sym = (s: string) => b64(`sym:${s}`);
const iso = (t: number) => new Date(t * 1000).toISOString().replace('.000Z', 'Z');

export function eventId(l: number, t: number, o: number, i: number): string {
  const toid = (BigInt(l) << 32n) | (BigInt(t & 0xfffff) << 12n) | BigInt(o & 0xfff);
  return `${toid.toString().padStart(19, '0')}-${String(i).padStart(10, '0')}`;
}
const maxCursor = (l: number) => eventId(l, 0xfffff, 0xfff, 0xffffffff);
const txToid = (l: number, t: number) => ((BigInt(l) << 32n) | (BigInt(t) << 12n) | 1n).toString();

export interface WEvent {
  type: 'contract' | 'system';
  contractId: string;
  topic: string[];
  value: string;
}

export interface WTx {
  hash: string;
  status: 'SUCCESS' | 'FAILED';
  feeBump: boolean;
  resultXdr: string;
  envelopeXdr: string;
  resultMetaXdr: string;
  /** A Soroban invocation (op 0): its events when it succeeded. */
  soroban?: { events: WEvent[]; returnValue: string };
  /** A classic operation's events (CAP-67): not committed. */
  classic?: WEvent[];
  fee: WEvent;
}

export interface WLedger {
  sequence: number;
  closeTime: number;
  txs: WTx[];
  header: string;
  hash: string;
  results: string;
  preimages: string[];
  txSet: string;
}

/** A served event, as stellar-rpc's getEvents writes it, with its place for ordering. */
interface Placed {
  pos: [number, number, number, number];
  json: Record<string, unknown>;
}

export function matchesFilters(filters: unknown, ev: Record<string, any>): boolean {
  const fs = (filters ?? []) as Record<string, any>[];
  if (!fs.length) return true;
  return fs.some((f) => {
    if (f.type && !String(f.type).split(',').map((x) => x.trim()).includes(ev.type)) return false;
    if (f.contractIds?.length && !f.contractIds.includes(ev.contractId)) return false;
    if (!f.topics?.length) return true;
    return f.topics.some((tf: string[]) => {
      const open = tf[tf.length - 1] === '**';
      const fixed = open ? tf.slice(0, -1) : tf;
      if (ev.topic.length < fixed.length || (!open && ev.topic.length !== fixed.length)) return false;
      return fixed.every((s, k) => s === '*' || s === ev.topic[k]);
    });
  });
}

function checkFilters(filters: unknown): string | null {
  if (filters === null || filters === undefined) return null;
  if (!Array.isArray(filters)) return 'filters must be an array';
  for (const f of filters) {
    if (!f || typeof f !== 'object') return 'a filter must be an object';
    for (const k of Object.keys(f)) if (!['type', 'contractIds', 'topics'].includes(k)) return `unknown filter field "${k}"`;
  }
  return null;
}

export class StellarWorld {
  ledgers = new Map<number, WLedger>();
  latest: number;
  /** The newest ledger a proof can be anchored at. */
  anchoredThrough: number;
  /** Changes the server's result for one call (the lying server). */
  tamper: ((method: string, params: any, result: any) => any) | null = null;
  /** Changes a proof bundle before it is served. */
  tamperProof: ((bundle: any) => any) | null = null;
  calls: { method: string; params: any }[] = [];
  passphrase = PUBNET;
  refuseProofs = false;

  constructor(
    readonly first: number,
    readonly last: number,
    anchoredThrough: number,
  ) {
    this.latest = last;
    this.anchoredThrough = anchoredThrough;
    let prev = sha('genesis');
    for (let s = first; s <= last; s++) {
      const l = this.make(s, prev);
      this.ledgers.set(s, l);
      prev = l.hash;
    }
  }

  private make(s: number, prev: string): WLedger {
    const closeTime = 1_794_000_000 + s * 5;
    const tx = (i: number, status: 'SUCCESS' | 'FAILED', extra: Partial<WTx>): WTx => {
      const hash = sha(`tx:${s}:${i}`);
      return {
        hash,
        status,
        feeBump: false,
        resultXdr: b64(`result:${hash}:${status}`),
        envelopeXdr: b64(`envelope:${hash}`),
        resultMetaXdr: b64(`meta:${hash}`),
        fee: { type: 'contract', contractId: XLM_SAC, topic: [sym('fee'), b64(`addr:${i}`)], value: b64(`i128:${100 + i}`) },
        ...extra,
      };
    };
    const transfer = (c: string, n: number): WEvent => ({ type: 'contract', contractId: c, topic: [sym('transfer'), b64(`addr:${n}`), b64(`addr:${n + 1}`)], value: b64(`i128:${s * 10 + n}`) });
    const txs: WTx[] = [
      tx(1, 'SUCCESS', {
        soroban: {
          events: [transfer(CONTRACT_A, 1), { type: 'contract', contractId: CONTRACT_B, topic: [sym('mint'), b64('addr:9')], value: b64(`i128:${s}`) }, ...(s % 3 === 0 ? [{ type: 'system' as const, contractId: CONTRACT_A, topic: [sym('upgrade')], value: b64('wasm') }] : [])],
          returnValue: b64(`ret:${s}:1`),
        },
      }),
      tx(2, 'SUCCESS', { classic: [transfer(XLM_SAC, 2)] }),
      tx(3, 'FAILED', { soroban: { events: [], returnValue: '' } }),
      tx(4, 'SUCCESS', { soroban: { events: [], returnValue: b64(`ret:${s}:4`) } }),
      ...(s % 2 === 0 ? [tx(5, 'SUCCESS', { feeBump: true, soroban: { events: [transfer(CONTRACT_B, 5)], returnValue: b64(`ret:${s}:5`) } })] : []),
    ];
    const invoked = (t: WTx) => t.status === 'SUCCESS' && t.soroban;
    const preimages = txs.filter(invoked).map((t) => b64(JSON.stringify({ events: t.soroban!.events, returnValue: t.soroban!.returnValue })));
    let k = 0;
    const results = b64(
      JSON.stringify(txs.map((t) => ({ txHash: t.hash, status: t.status, feeBump: t.feeBump, resultXdr: t.resultXdr, preimage: invoked(t) ? sha(preimages[k++]) : null }))),
    );
    // The set's own order is not the application order.
    const txSet = b64(JSON.stringify([...txs].reverse().map((t) => ({ hash: t.hash, envelopeXdr: t.envelopeXdr }))));
    const header = b64(JSON.stringify({ sequence: s, prev, closeTime, protocolVersion: 23, results: sha(results), txSet: sha(txSet) }));
    return { sequence: s, closeTime, txs, header, hash: sha(header), results, preimages, txSet };
  }

  ledger(s: number): WLedger {
    const l = this.ledgers.get(s);
    if (!l) throw new Error(`no ledger ${s} in the world`);
    return l;
  }

  headerHistory(l: WLedger): string {
    return b64(JSON.stringify({ hash: l.hash, header: l.header }));
  }

  /** Every event of a ledger in stellar-rpc's order: fee events (0, 0), then per transaction its operation events. */
  servedEvents(s: number): Placed[] {
    const l = this.ledger(s);
    const out: Placed[] = [];
    const json = (ev: WEvent, t: number, o: number, i: number, tx: WTx): Placed => ({
      pos: [s, t, o, i],
      json: {
        type: ev.type,
        ledger: s,
        ledgerClosedAt: iso(l.closeTime),
        contractId: ev.contractId,
        id: eventId(s, t, o, i),
        operationIndex: o,
        transactionIndex: t,
        txHash: tx.hash,
        inSuccessfulContractCall: tx.status === 'SUCCESS',
        topic: ev.topic,
        value: ev.value,
      },
    });
    l.txs.forEach((tx, k) => out.push(json(tx.fee, 0, 0, k, tx)));
    l.txs.forEach((tx, k) => {
      if (tx.status === 'SUCCESS' && tx.soroban) tx.soroban.events.forEach((e, i) => out.push(json(e, k + 1, 0, i, tx)));
      if (tx.classic) tx.classic.forEach((e, i) => out.push(json(e, k + 1, 0, i, tx)));
    });
    return out;
  }

  /** What the real verifier proves: the successful invocations' events. */
  committedEvents(s: number, filters?: unknown): Record<string, unknown>[] {
    const l = this.ledger(s);
    return this.servedEvents(s)
      .filter((p) => p.pos[1] > 0 && l.txs[p.pos[1] - 1].status === 'SUCCESS' && l.txs[p.pos[1] - 1].soroban)
      .map((p) => p.json)
      .filter((e) => matchesFilters(filters, e));
  }

  txJson(s: number, k: number, createdAtAsString: boolean): Record<string, unknown> {
    const l = this.ledger(s);
    const t = l.txs[k];
    return {
      status: t.status,
      txHash: t.hash,
      applicationOrder: k + 1,
      feeBump: t.feeBump,
      envelopeXdr: t.envelopeXdr,
      resultXdr: t.resultXdr,
      resultMetaXdr: t.resultMetaXdr,
      events: {
        transactionEventsXdr: [b64(JSON.stringify(t.fee))],
        contractEventsXdr: [t.status === 'SUCCESS' ? (t.soroban?.events ?? t.classic ?? []).map((e) => b64(JSON.stringify(e))) : []],
      },
      ledger: s,
      createdAt: createdAtAsString ? String(l.closeTime) : l.closeTime,
    };
  }

  private range() {
    const top = this.ledger(this.latest);
    return { latestLedger: this.latest, latestLedgerCloseTime: String(top.closeTime), oldestLedger: this.first, oldestLedgerCloseTime: String(this.ledger(this.first).closeTime) };
  }

  bundle(start: number, end: number, txs: boolean): Record<string, unknown> {
    const ledgers = [];
    for (let s = start; s <= end; s++) {
      const l = this.ledger(s);
      ledgers.push({ sequence: s, header: l.header, results: l.results, preimages: l.preimages, ...(txs ? { txSet: l.txSet } : {}) });
    }
    const last = this.ledger(end);
    if (end > this.anchoredThrough) {
      return { network: PUBNET, startLedger: start, endLedger: end, ledgers, anchor: null, pending: `checkpoint ${Math.ceil((end + 2) / 64) * 64 - 1} is not published yet` };
    }
    return {
      network: PUBNET,
      startLedger: start,
      endLedger: end,
      ledgers,
      anchor: { ledger: end, txSetNext: b64(`next:${end}`), envelopes: [0, 1, 2, 3, 4, 5, 6, 7].map((k) => signature(k, last.hash)), quorumSet: QSET, source: `scp/${end + 1}` },
    };
  }

  /** stellar-rpc's methods over the world; null for an unknown method. */
  serve(method: string, params: any): { result?: unknown; error?: { code: number; message: string } } {
    const p = params ?? {};
    const bad = (message: string) => ({ error: { code: -32602, message } });
    switch (method) {
      case 'getHealth':
        return { result: { status: 'healthy', ...this.range(), ledgerRetentionWindow: 120960 } };
      case 'getNetwork':
        return { result: { passphrase: this.passphrase, protocolVersion: 23 } };
      case 'getVersionInfo':
        return { result: { version: '23.0.0', protocolVersion: 23 } };
      case 'getFeeStats':
        return { result: { latestLedger: this.latest } };
      case 'getLatestLedger': {
        const l = this.ledger(this.latest);
        return { result: { id: l.hash, protocolVersion: 23, sequence: l.sequence, closeTime: String(l.closeTime), headerXdr: l.header, metadataXdr: b64(`lcm:${l.sequence}`) } };
      }
      case 'getLedgers': {
        const limit = p.pagination?.limit ?? 5;
        const start = p.pagination?.cursor ? Number(p.pagination.cursor) + 1 : p.startLedger;
        if (!start) return bad('startLedger or cursor');
        const ledgers = [];
        for (let s = start; s <= Math.min(this.latest, start + limit - 1); s++) {
          const l = this.ledger(s);
          ledgers.push({ hash: l.hash, sequence: s, ledgerCloseTime: String(l.closeTime), headerXdr: this.headerHistory(l), metadataXdr: b64(`lcm:${s}`) });
        }
        const cursor = ledgers.length ? String(ledgers[ledgers.length - 1].sequence) : (p.pagination?.cursor ?? String(start - 1));
        return { result: { ledgers, ...this.range(), cursor } };
      }
      case 'getTransaction': {
        for (const l of this.ledgers.values()) {
          const k = l.txs.findIndex((t) => t.hash === String(p.hash).toLowerCase());
          if (k >= 0) return { result: { ...this.range(), ...this.txJson(l.sequence, k, true), txHash: p.hash } };
        }
        return { result: { ...this.range(), status: 'NOT_FOUND', txHash: p.hash, applicationOrder: 0, feeBump: false, events: {}, ledger: 0, createdAt: '0' } };
      }
      case 'getTransactions': {
        const limit = p.pagination?.limit ?? 10;
        let at: [number, number];
        if (p.pagination?.cursor) {
          const c = BigInt(p.pagination.cursor);
          at = [Number(c >> 32n), Number((c >> 12n) & 0xfffffn) + 1];
        } else at = [p.startLedger, 1];
        const transactions = [];
        let cursor = txToid(at[0], at[1] - 1);
        for (let s = at[0]; s <= this.latest && transactions.length < limit; s++) {
          const l = this.ledger(s);
          const from = s === at[0] ? at[1] : 1;
          cursor = txToid(s, from - 1);
          for (let t = from; t <= l.txs.length && transactions.length < limit; t++) {
            transactions.push(this.txJson(s, t - 1, false));
            cursor = txToid(s, t);
          }
        }
        const r = this.range();
        return { result: { transactions, latestLedger: r.latestLedger, latestLedgerCloseTimestamp: Number(r.latestLedgerCloseTime), oldestLedger: r.oldestLedger, oldestLedgerCloseTimestamp: Number(r.oldestLedgerCloseTime), cursor } };
      }
      case 'getEvents': {
        const cursor = p.pagination?.cursor;
        if (cursor && (p.startLedger || p.endLedger)) return bad('ledger ranges and cursor cannot both be set');
        const why = checkFilters(p.filters);
        if (why) return bad(why);
        let start: [number, number, number, number];
        if (cursor) {
          const [a, b] = String(cursor).split('-');
          const t = BigInt(a);
          start = [Number(t >> 32n), Number((t >> 12n) & 0xfffffn), Number(t & 0xfffn), Number(b) + 1];
        } else start = [p.startLedger, 0, 0, 0];
        const limit = p.pagination?.limit ?? 100;
        const endLedger = Math.min(start[0] + 10000, this.latest + 1, p.endLedger || Infinity);
        const events: Record<string, unknown>[] = [];
        const ge = (a: number[], b: number[]) => a[0] - b[0] || a[1] - b[1] || a[2] - b[2] || a[3] - b[3];
        outer: for (let s = start[0]; s < endLedger; s++) {
          for (const e of this.servedEvents(s)) {
            if (ge(e.pos, start) < 0 || !matchesFilters(p.filters, e.json)) continue;
            events.push(e.json);
            if (events.length >= limit) break outer;
          }
        }
        const r = this.range();
        return {
          result: {
            events,
            cursor: events.length >= limit ? (events[events.length - 1].id as string) : maxCursor(endLedger - 1),
            latestLedger: r.latestLedger,
            oldestLedger: r.oldestLedger,
            latestLedgerCloseTime: r.latestLedgerCloseTime,
            oldestLedgerCloseTime: r.oldestLedgerCloseTime,
          },
        };
      }
      case 'willow_getLedgerProof': {
        if (this.refuseProofs) return { error: { code: -32601, message: 'willow_getLedgerProof is not on this plan' } };
        const end = p.endLedger ?? p.startLedger;
        if (end - p.startLedger >= (p.transactions ? 16 : 64) || end < p.startLedger) return bad('at most 64 ledgers, 16 with transactions');
        return { result: this.bundle(p.startLedger, end, !!p.transactions) };
      }
      default:
        return { error: { code: -32601, message: `method ${method} not found` } };
    }
  }

  /** A `fetch` serving the world (one call or a batch), through `tamper`. */
  fetch = async (_url: string | URL | Request, init?: RequestInit): Promise<Response> => {
    const body = JSON.parse(String(init?.body));
    const one = (req: any) => {
      this.calls.push({ method: req.method, params: req.params });
      const r = this.serve(req.method, req.params);
      let result = r.result === undefined ? undefined : structuredClone(r.result);
      if (result !== undefined && req.method === 'willow_getLedgerProof' && this.tamperProof) result = this.tamperProof(result);
      if (result !== undefined && this.tamper) result = this.tamper(req.method, req.params, result) ?? result;
      return r.error ? { jsonrpc: '2.0', id: req.id, error: r.error } : { jsonrpc: '2.0', id: req.id, result };
    };
    const out = Array.isArray(body) ? body.map(one) : one(body);
    return new Response(JSON.stringify(out), { status: 200, headers: { 'content-type': 'application/json' } });
  };
}

export const signature = (k: number, hash: string) => `sig:${k}:${sha(`validator ${k} externalizes ${hash}`)}`;

/** The fake Stellar module: checks a world bundle and renders what the real module renders. */
export function fakeStellarModule(): StellarVerifier {
  const fail = (kind: string, message: string) => JSON.stringify({ ok: false, kind, message });
  return {
    verifyLedgerBundle(response: string, trusted?: string | null): string {
      let v: any;
      try {
        v = JSON.parse(response);
      } catch {
        return fail('invalid', 'not JSON');
      }
      const b = v && typeof v === 'object' && 'jsonrpc' in v ? v.result : v;
      if (!b || typeof b !== 'object') return fail('invalid', 'no bundle');
      if (b.network !== PUBNET) return fail('invalid', 'not pubnet');
      if (b.startLedger > b.endLedger || b.endLedger - b.startLedger >= 64) return fail('invalid', 'range');
      if (!Array.isArray(b.ledgers) || b.ledgers.length !== b.endLedger - b.startLedger + 1) return fail('invalid', 'the bundle does not carry every ledger of its range');
      if (!b.anchor) return fail('pending', b.pending ?? 'no anchor');
      if (b.anchor.ledger !== b.endLedger) return fail('invalid', 'the anchor is not for the last ledger');
      const ledgers = [];
      let prev: string | null = null;
      for (let k = 0; k < b.ledgers.length; k++) {
        const lp = b.ledgers[k];
        const seq = b.startLedger + k;
        let h: any;
        try {
          h = JSON.parse(unb64(lp.header));
        } catch {
          return fail('invalid', 'header is not XDR');
        }
        const hash = sha(lp.header);
        if (lp.sequence !== seq || h.sequence !== seq) return fail('invalid', `ledger ${seq} is numbered wrong`);
        if (prev && h.prev !== prev) return fail('invalid', `ledger ${seq} does not follow ledger ${seq - 1}`);
        if (sha(lp.results) !== h.results) return fail('invalid', `ledger ${seq}: the results do not hash to txSetResultHash`);
        const results = JSON.parse(unb64(lp.results));
        const invoked = results.map((r: any, i: number) => [r, i]).filter(([r]: any) => r.preimage);
        if (invoked.length !== lp.preimages.length) return fail('invalid', `ledger ${seq}: preimages`);
        const invocations: unknown[] = [];
        const events: unknown[] = [];
        const ret = new Map<number, string>();
        for (let j = 0; j < invoked.length; j++) {
          const [r, i] = invoked[j];
          if (sha(lp.preimages[j]) !== r.preimage) return fail('invalid', `ledger ${seq}: events do not hash`);
          const pre = JSON.parse(unb64(lp.preimages[j]));
          invocations.push({ tx: i + 1, op: 0, eventsXdr: pre.events.map((e: WEvent) => b64(JSON.stringify(e))) });
          ret.set(i, pre.returnValue);
          pre.events.forEach((e: WEvent, n: number) =>
            events.push({
              type: e.type,
              ledger: seq,
              ledgerClosedAt: iso(h.closeTime),
              contractId: e.contractId,
              id: eventId(seq, i + 1, 0, n),
              operationIndex: 0,
              transactionIndex: i + 1,
              txHash: r.txHash,
              inSuccessfulContractCall: true,
              topic: e.topic,
              value: e.value,
            }),
          );
        }
        let envelopes: Map<string, string> | null = null;
        if (lp.txSet !== undefined) {
          if (sha(lp.txSet) !== h.txSet) return fail('invalid', `ledger ${seq}: the transaction set does not hash to txSetHash`);
          envelopes = new Map(JSON.parse(unb64(lp.txSet)).map((e: any) => [e.hash, e.envelopeXdr]));
        }
        const transactions = results.map((r: any, i: number) => ({
          status: r.status,
          txHash: r.txHash,
          applicationOrder: i + 1,
          feeBump: r.feeBump,
          resultXdr: r.resultXdr,
          ledger: seq,
          createdAt: String(h.closeTime),
          ...(envelopes ? { envelopeXdr: envelopes.get(r.txHash) } : {}),
          ...(ret.has(i) ? { returnValueXdr: ret.get(i) } : {}),
        }));
        ledgers.push({
          sequence: seq,
          hash,
          previousHash: h.prev,
          closeTime: String(h.closeTime),
          protocolVersion: h.protocolVersion,
          headerXdr: lp.header,
          headerHistoryXdr: b64(JSON.stringify({ hash, header: lp.header })),
          transactionCount: results.length,
          transactions,
          events,
          invocations,
        });
        prev = hash;
      }
      const last = ledgers[ledgers.length - 1].hash;
      const qset = trusted ?? QSET;
      const signed = new Set(b.anchor.envelopes).size;
      const valid = b.anchor.envelopes.every((e: string, k: number) => e === signature(k, last));
      if (qset !== QSET || !valid || signed < 7) return fail('invalid', `SCP anchor for ledger ${b.endLedger}: NoQuorum`);
      return JSON.stringify({
        ok: true,
        verified: {
          network: PUBNET,
          anchor: { ledger: b.endLedger, hash: last, quorumSetHash: sha(QSET), nextQuorumSetHash: '00'.repeat(32), signedEntries: signed, totalEntries: 10, signers: signed, source: b.anchor.source ?? null },
          ledgers,
        },
      });
    },
    matchEvents(events: string, filters: string): string {
      const f = JSON.parse(filters);
      const why = checkFilters(f);
      if (why) return fail('invalid-request', why);
      return JSON.stringify({ ok: true, matches: JSON.parse(events).map((e: any) => matchesFilters(f, e)) });
    },
    pinnedQuorumSet(): string {
      return JSON.stringify({ ok: true, xdr: QSET, hash: sha(QSET), network: PUBNET, maxLedgers: 64 });
    },
  };
}
