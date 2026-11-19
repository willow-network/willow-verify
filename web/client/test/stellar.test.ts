// The Stellar client (@willow-network/rpc/stellar) against a mock stellar-rpc over a made-up chain
// (test/support/stellar-world.ts) with the fake Stellar module: every method's happy path, the
// lying server (left-out, extra and changed events and transactions, wrong ledger hashes, tampered
// proofs), the newest ledgers not anchored yet, and the `unverifiable` policy.
import { describe, expect, it } from 'vitest';
import { WillowRpcError, WillowUnverifiableError, WillowVerificationError } from '../src/errors.js';
import {
  anchorableBefore,
  checkpointOf,
  createStellarVerifier,
  eventId,
  parseEventId,
  parseTxCursor,
  txCursor,
  type StellarOptions,
  type StellarVerificationEvent,
} from '../src/stellar.js';
import { b64, CONTRACT_A, CONTRACT_B, eventId as worldEventId, fakeStellarModule, StellarWorld, sym } from './support/stellar-world.js';

// Ledgers 64063..64400; checkpoint 64319 is the newest published, so 64318 is the newest anchorable.
const FIRST = 64063;
const LATEST = 64400;
const ANCHORED = 64318;

function setup(o: Partial<StellarOptions> = {}, w = new StellarWorld(FIRST, LATEST, ANCHORED)) {
  const events: StellarVerificationEvent[] = [];
  const client = createStellarVerifier({
    url: 'https://stellar.test/v1/rpc/stellar/public',
    fetch: w.fetch as typeof fetch,
    verifiers: { stellar: async () => fakeStellarModule() },
    onVerification: (e) => events.push(e),
    waitMs: 5,
    ...o,
  });
  return { w, client, events, last: () => events[events.length - 1] };
}

const kindOf = async (p: Promise<unknown>) => {
  try {
    await p;
  } catch (e) {
    if (e instanceof WillowVerificationError || e instanceof WillowUnverifiableError) return `${e.constructor.name}:${e.data.kind}`;
    if (e instanceof WillowRpcError) return `WillowRpcError:${e.code}`;
    throw e;
  }
  return 'passed';
};
const failed = (kind: string) => `WillowVerificationError:${kind}`;
const unverifiable = (kind: string) => `WillowUnverifiableError:${kind}`;

describe('stellar-rpc ids, cursors and checkpoints', () => {
  it('reads and writes the event ids and cursors SDF stellar-rpc serves', () => {
    expect(parseEventId('0281438783021252608-0000000000')).toEqual({ l: 65_527_573, t: 0, o: 0, i: 0 });
    expect(parseEventId('0281438783021535236-0000000007')).toEqual({ l: 65_527_573, t: 69, o: 4, i: 7 });
    expect(eventId({ l: 65_527_573, t: 69, o: 1, i: 0 })).toBe('0281438783021535233-0000000000');
    expect(eventId({ l: 65_527_573, t: 0xfffff, o: 0, i: 2 })).toBe('0281438787316215808-0000000002');
    expect(parseEventId('x-1')).toBeNull();
    expect(parseEventId('1-99999999999')).toBeNull();
    // getTransactions' cursor: ledger 65,527,573, the third transaction.
    expect(parseTxCursor('281438783021264897')).toEqual({ l: 65_527_573, t: 3 });
    expect(txCursor({ l: 65_527_573, t: 3 })).toBe('281438783021264897');
  });

  it('knows which ledgers wait for which checkpoint', () => {
    expect(checkpointOf(63)).toBe(63);
    expect(checkpointOf(64)).toBe(127);
    // Ledger 126's anchor is slot 127, in checkpoint 127; ledger 127's is slot 128, in checkpoint 191.
    expect(anchorableBefore(127)).toBe(126);
    expect(anchorableBefore(126)).toBe(62);
    expect(anchorableBefore(LATEST)).toBe(64382);
    expect(anchorableBefore(64382)).toBe(ANCHORED);
  });
});

describe('getLatestLedger', () => {
  it('returns the newest anchored ledger and says how far behind the server it is', async () => {
    const { w, client, last } = setup();
    const l = w.ledger(ANCHORED);
    expect(await client.request({ method: 'getLatestLedger' })).toEqual({ id: l.hash, protocolVersion: 23, sequence: ANCHORED, closeTime: String(l.closeTime), headerXdr: l.header });
    expect(last()).toMatchObject({ status: 'verified', ledger: { sequence: ANCHORED, hash: l.hash }, serverLatest: LATEST, behind: LATEST - ANCHORED, anchor: { ledger: ANCHORED } });
    expect(last().unproven).toContain('metadataXdr');
  });

  it('the tip, once anchored; unverifiable while it is not', async () => {
    const { w, client, last } = setup({ latest: 'tip', anchorWaitMs: 30 });
    expect(await kindOf(client.request({ method: 'getLatestLedger' }))).toBe(unverifiable('pending'));
    w.anchoredThrough = LATEST;
    const l = w.ledger(LATEST);
    expect(await client.request({ method: 'getLatestLedger' })).toMatchObject({ id: l.hash, sequence: LATEST });
    expect(last().status).toBe('verified');
  });

  it('waits for the tip to be anchored', async () => {
    const { w, client } = setup({ latest: 'tip', anchorWaitMs: 5000 });
    setTimeout(() => (w.anchoredThrough = LATEST), 40);
    expect(await client.request({ method: 'getLatestLedger' })).toMatchObject({ sequence: LATEST });
  });

  it('refuses a tip that is not the proven ledger', async () => {
    const { w, client } = setup({ latest: 'tip' });
    w.anchoredThrough = LATEST;
    w.tamper = (m, _p, r) => (m === 'getLatestLedger' ? { ...r, id: 'ab'.repeat(32) } : r);
    expect(await kindOf(client.request({ method: 'getLatestLedger' }))).toBe(failed('ledger-mismatch'));
  });

  it('flag: the unanchored tip comes back marked, with its metadata', async () => {
    const { client, last } = setup({ latest: 'tip', unverifiable: 'flag' });
    const r = (await client.request({ method: 'getLatestLedger' })) as any;
    expect(r.sequence).toBe(LATEST);
    expect(r.metadataXdr).toBeDefined();
    expect(last()).toMatchObject({ status: 'unverifiable', kind: 'pending' });
  });
});

describe('getLedgers', () => {
  it('returns the proven ledgers without their metadata', async () => {
    const { w, client, last } = setup();
    const r = (await client.request({ method: 'getLedgers', params: { startLedger: 64100, pagination: { limit: 10 } } })) as any;
    expect(r.ledgers).toHaveLength(10);
    r.ledgers.forEach((x: any, k: number) => {
      const l = w.ledger(64100 + k);
      expect(x).toEqual({ hash: l.hash, sequence: l.sequence, ledgerCloseTime: String(l.closeTime), headerXdr: w.headerHistory(l) });
    });
    expect(r.cursor).toBe('64109');
    expect(r.latestLedger).toBe(LATEST);
    expect(last()).toMatchObject({ status: 'verified', ledgers: { from: 64100, to: 64109 }, ledger: { sequence: 64109 } });
    expect(last().unproven).toEqual(expect.arrayContaining(['ledgers[].metadataXdr', 'latestLedger']));
    const next = (await client.request({ method: 'getLedgers', params: { pagination: { cursor: r.cursor, limit: 3 } } })) as any;
    expect(next.ledgers.map((x: any) => x.sequence)).toEqual([64110, 64111, 64112]);
  });

  it('spans proof windows', async () => {
    const { client, w } = setup();
    const calls = () => w.calls.filter((c) => c.method === 'willow_getLedgerProof').map((c) => [c.params.startLedger, c.params.endLedger]);
    const r = (await client.request({ method: 'getLedgers', params: { startLedger: 64120, pagination: { limit: 80 } } })) as any;
    expect(r.ledgers).toHaveLength(80);
    expect(calls()).toEqual([
      [64120, 64126],
      [64127, 64190],
      [64191, 64199],
    ]);
  });

  it('ends a page at the newest anchored ledger; flag keeps the rest, marked', async () => {
    const { client } = setup();
    const r = (await client.request({ method: 'getLedgers', params: { startLedger: 64310, pagination: { limit: 20 } } })) as any;
    expect(r.ledgers.map((x: any) => x.sequence)).toEqual(Array.from({ length: 9 }, (_, k) => 64310 + k));
    expect(r.cursor).toBe(String(ANCHORED));
    const f = setup({ unverifiable: 'flag' });
    const r2 = (await f.client.request({ method: 'getLedgers', params: { startLedger: 64310, pagination: { limit: 20 } } })) as any;
    expect(r2.ledgers).toHaveLength(20);
    expect(r2.ledgers[0].metadataXdr).toBeDefined();
    expect(r2.cursor).toBe('64329');
    expect(f.last().status).toBe('unverifiable');
    expect(f.last().flags!.map((x) => x.kind)).toEqual(expect.arrayContaining(['pending', 'unprovable']));
  });

  it('an empty page when nothing served is anchored yet', async () => {
    const { client } = setup();
    expect(await client.request({ method: 'getLedgers', params: { startLedger: 64390, pagination: { limit: 5 } } })).toMatchObject({ ledgers: [], cursor: '64389' });
  });

  it('refuses a wrong ledger hash, a wrong header, a gap', async () => {
    const lie = async (f: (r: any) => void) => {
      const { w, client } = setup();
      w.tamper = (m, _p, r) => {
        if (m === 'getLedgers') f(r);
        return r;
      };
      return kindOf(client.request({ method: 'getLedgers', params: { startLedger: 64100, pagination: { limit: 5 } } }));
    };
    expect(await lie((r) => (r.ledgers[2].hash = 'cd'.repeat(32)))).toBe(failed('ledger-mismatch'));
    expect(await lie((r) => (r.ledgers[1].headerXdr = r.ledgers[0].headerXdr))).toBe(failed('ledger-mismatch'));
    expect(await lie((r) => (r.ledgers[3].ledgerCloseTime = '1'))).toBe(failed('ledger-mismatch'));
    expect(await lie((r) => r.ledgers.splice(2, 1))).toBe(failed('page'));
  });

  it('refuses a tampered proof', async () => {
    const lie = async (f: (b: any) => void) => {
      const { w, client } = setup();
      w.tamperProof = (b) => {
        f(b);
        return b;
      };
      return kindOf(client.request({ method: 'getLedgers', params: { startLedger: 64100, pagination: { limit: 5 } } }));
    };
    // A header changed: the next one no longer follows it.
    expect(await lie((b) => (b.ledgers[1].header = b64(JSON.stringify({ ...JSON.parse(Buffer.from(b.ledgers[1].header, 'base64').toString()), closeTime: 1 }))))).toBe(failed('ledger-proof'));
    // Too few signatures for the anchor.
    expect(await lie((b) => b.anchor.envelopes.splice(3))).toBe(failed('ledger-proof'));
    // The results swapped for another ledger's.
    expect(await lie((b) => (b.ledgers[0].results = b.ledgers[1].results))).toBe(failed('ledger-proof'));
    // A proof of other ledgers than asked.
    expect(await lie((b) => ((b.startLedger = 1), (b.endLedger = 1), (b.ledgers = b.ledgers.slice(0, 1)), (b.anchor.ledger = 1)))).toBe(failed('ledger-proof'));
  });

  it('a trusted quorum set the signers are no quorum of fails', async () => {
    const { client } = setup({ trustedQuorumSet: b64('some other set') });
    expect(await kindOf(client.request({ method: 'getLedgers', params: { startLedger: 64100, pagination: { limit: 2 } } }))).toBe(failed('ledger-proof'));
  });

  it('xdrFormat json is not verified', async () => {
    const { client } = setup();
    expect(await kindOf(client.request({ method: 'getLedgers', params: { startLedger: 64100, xdrFormat: 'json' } }))).toBe(unverifiable('json-format'));
  });
});

describe('getTransaction', () => {
  it('a Soroban transaction: proven fields, its contract events and return value; meta left out', async () => {
    const { w, client, last } = setup();
    const l = w.ledger(64100);
    const t = l.txs[0];
    const r = (await client.request({ method: 'getTransaction', params: { hash: t.hash } })) as any;
    expect(r).toMatchObject({
      status: 'SUCCESS',
      txHash: t.hash,
      applicationOrder: 1,
      feeBump: false,
      envelopeXdr: t.envelopeXdr,
      resultXdr: t.resultXdr,
      events: { contractEventsXdr: [t.soroban!.events.map((e) => b64(JSON.stringify(e)))] },
      returnValueXdr: t.soroban!.returnValue,
      ledger: 64100,
      createdAt: String(l.closeTime),
      latestLedger: LATEST,
    });
    expect(r.resultMetaXdr).toBeUndefined();
    expect(r.events.transactionEventsXdr).toBeUndefined();
    expect(last()).toMatchObject({ status: 'verified', ledger: { sequence: 64100 } });
    expect(last().unproven).toEqual(expect.arrayContaining(['resultMetaXdr', 'events.transactionEventsXdr']));
  });

  it('a classic transaction: no events proven; flag keeps them, marked', async () => {
    const { w, client } = setup();
    const t = w.ledger(64100).txs[1];
    const r = (await client.request({ method: 'getTransaction', params: { hash: t.hash } })) as any;
    expect(r.events).toBeUndefined();
    expect(r.status).toBe('SUCCESS');
    const f = setup({ unverifiable: 'flag' });
    const r2 = (await f.client.request({ method: 'getTransaction', params: { hash: t.hash } })) as any;
    expect(r2.resultMetaXdr).toBe(t.resultMetaXdr);
    expect(r2.events.contractEventsXdr).toHaveLength(1);
    expect(f.last()).toMatchObject({ status: 'unverifiable', kind: 'unprovable' });
  });

  it('a failed and a fee-bumped transaction', async () => {
    const { w, client } = setup();
    expect(await client.request({ method: 'getTransaction', params: { hash: w.ledger(64100).txs[2].hash } })).toMatchObject({ status: 'FAILED', applicationOrder: 3 });
    expect(await client.request({ method: 'getTransaction', params: { hash: w.ledger(64100).txs[4].hash } })).toMatchObject({ status: 'SUCCESS', feeBump: true, applicationOrder: 5 });
  });

  it('refuses a changed result, envelope, contract event, place or time', async () => {
    const lie = async (f: (r: any) => void, k = 0) => {
      const { w, client } = setup();
      w.tamper = (m, _p, r) => {
        if (m === 'getTransaction') f(r);
        return r;
      };
      return kindOf(client.request({ method: 'getTransaction', params: { hash: w.ledger(64100).txs[k].hash } }));
    };
    expect(await lie((r) => (r.resultXdr = b64('result:forged')))).toBe(failed('transaction-mismatch'));
    expect(await lie((r) => (r.status = 'FAILED'))).toBe(failed('transaction-mismatch'));
    expect(await lie((r) => (r.envelopeXdr = b64('envelope:forged')))).toBe(failed('transaction-mismatch'));
    expect(await lie((r) => (r.createdAt = '5'))).toBe(failed('transaction-mismatch'));
    expect(await lie((r) => (r.applicationOrder = 2))).toBe(failed('transaction-mismatch'));
    expect(await lie((r) => (r.ledger = 64101))).toBe(failed('transaction-mismatch'));
    expect(await lie((r) => (r.events.contractEventsXdr[0] = r.events.contractEventsXdr[0].slice(1)))).toBe(failed('event-mismatch'));
    // A classic transaction claimed to be the Soroban one's.
    expect(await lie((r) => (r.events.contractEventsXdr = [[]]))).toBe(failed('event-mismatch'));
  });

  it('NOT_FOUND is unverifiable; flag passes it through', async () => {
    const { client } = setup();
    const hash = '00'.repeat(32);
    expect(await kindOf(client.request({ method: 'getTransaction', params: { hash } }))).toBe(unverifiable('not-found'));
    const f = setup({ unverifiable: 'flag' });
    expect(await f.client.request({ method: 'getTransaction', params: { hash } })).toMatchObject({ status: 'NOT_FOUND' });
    expect(f.last()).toMatchObject({ status: 'unverifiable', kind: 'not-found' });
  });

  it('a transaction in a ledger not anchored yet: waits, then unverifiable', async () => {
    const { w, client } = setup({ anchorWaitMs: 30 });
    const hash = w.ledger(64350).txs[0].hash;
    expect(await kindOf(client.request({ method: 'getTransaction', params: { hash } }))).toBe(unverifiable('pending'));
    const late = setup({ anchorWaitMs: 5000 }, w);
    setTimeout(() => (w.anchoredThrough = LATEST), 40);
    expect(await late.client.request({ method: 'getTransaction', params: { hash } })).toMatchObject({ status: 'SUCCESS', ledger: 64350 });
  });
});

describe('getTransactions', () => {
  it('a page of proven transactions, and the next page by its cursor', async () => {
    const { w, client, last } = setup();
    const r = (await client.request({ method: 'getTransactions', params: { startLedger: 64100, pagination: { limit: 10 } } })) as any;
    // 64100 has 5 transactions, 64101 has 4.
    expect(r.transactions.map((t: any) => `${t.ledger}:${t.applicationOrder}`)).toEqual(['64100:1', '64100:2', '64100:3', '64100:4', '64100:5', '64101:1', '64101:2', '64101:3', '64101:4', '64102:1']);
    expect(r.transactions[0]).toMatchObject({ txHash: w.ledger(64100).txs[0].hash, createdAt: w.ledger(64100).closeTime });
    expect(r.transactions[0].resultMetaXdr).toBeUndefined();
    expect(r.cursor).toBe(txCursor({ l: 64102, t: 1 }));
    expect(last()).toMatchObject({ status: 'verified', ledgers: { from: 64100, to: 64102 } });
    const next = (await client.request({ method: 'getTransactions', params: { pagination: { cursor: r.cursor, limit: 3 } } })) as any;
    expect(next.transactions.map((t: any) => `${t.ledger}:${t.applicationOrder}`)).toEqual(['64102:2', '64102:3', '64102:4']);
  });

  it('refuses a left-out, an extra and a changed transaction', async () => {
    const lie = async (f: (r: any) => void) => {
      const { w, client } = setup();
      w.tamper = (m, _p, r) => {
        if (m === 'getTransactions') f(r);
        return r;
      };
      return kindOf(client.request({ method: 'getTransactions', params: { startLedger: 64100, pagination: { limit: 10 } } }));
    };
    expect(await lie((r) => r.transactions.splice(3, 1))).toBe(failed('transactions'));
    expect(await lie((r) => r.transactions.pop())).toBe(failed('transactions'));
    expect(await lie((r) => r.transactions.splice(5, 0, { ...r.transactions[4], applicationOrder: 6 }))).toBe(failed('transactions'));
    expect(await lie((r) => (r.transactions[6].resultXdr = r.transactions[5].resultXdr))).toBe(failed('transaction-mismatch'));
    expect(await lie((r) => r.transactions.reverse())).toBe(failed('page'));
    // A cursor that claims more than the page holds.
    expect(await lie((r) => (r.cursor = txCursor({ l: 64103, t: 2 })))).toBe(failed('transactions'));
  });

  it('a long page, in proofs of 16 ledgers with their transaction sets', async () => {
    const { w, client, last } = setup();
    const r = (await client.request({ method: 'getTransactions', params: { startLedger: 64100, pagination: { limit: 200 } } })) as any;
    expect(r.transactions).toHaveLength(200);
    expect(last()).toMatchObject({ status: 'verified', ledgers: { from: 64100, to: r.transactions[199].ledger } });
    const proofs = w.calls.filter((c) => c.method === 'willow_getLedgerProof').map((c) => [c.params.startLedger, c.params.endLedger]);
    expect(proofs).toEqual([
      [64100, 64110],
      [64111, 64126],
      [64127, 64142],
      [64143, 64144],
    ]);
  });

  it('ends at the newest anchored ledger', async () => {
    const { client } = setup();
    const r = (await client.request({ method: 'getTransactions', params: { startLedger: ANCHORED, pagination: { limit: 10 } } })) as any;
    expect(r.transactions.every((t: any) => t.ledger === ANCHORED)).toBe(true);
    expect(r.transactions).toHaveLength(5);
    expect(r.cursor).toBe(txCursor({ l: ANCHORED + 1, t: 0 }));
  });
});

describe('getEvents', () => {
  const page = (client: any, params: any) => client.request({ method: 'getEvents', params }) as Promise<any>;
  const committed = (w: StellarWorld, from: number, to: number, filters?: unknown) => {
    const out = [];
    for (let s = from; s <= to; s++) out.push(...w.committedEvents(s, filters));
    return out;
  };

  it('every committed event of the range; fee and classic events left out', async () => {
    const { w, client, last } = setup();
    const r = await page(client, { startLedger: 64100, endLedger: 64103 });
    expect(r.events).toEqual(committed(w, 64100, 64102));
    expect(r.events).toHaveLength(9);
    expect(r.cursor).toBe(worldEventId(64102, 0xfffff, 0xfff, 0xffffffff));
    // Fee events (5 + 4 + 5) and one classic transfer per ledger.
    expect(last()).toMatchObject({ status: 'verified', dropped: 17, ledgers: { from: 64100, to: 64102 } });
    expect(last().unproven!.some((u) => u.startsWith('events no Soroban result commits to'))).toBe(true);
  });

  it('filters by contract, type and topic, as the server does', async () => {
    const { w, client } = setup();
    for (const filters of [[{ contractIds: [CONTRACT_B] }], [{ type: 'system' }], [{ topics: [[sym('transfer'), '*', '*']] }], [{ topics: [[sym('mint'), '**']] }, { contractIds: [CONTRACT_A], type: 'system' }]]) {
      const r = await page(client, { startLedger: 64100, endLedger: 64106, filters });
      expect(r.events).toEqual(committed(w, 64100, 64105, filters));
      expect(r.events.length).toBeGreaterThan(0);
    }
    expect(await kindOf(page(client, { startLedger: 64100, filters: [{ bogus: 1 }] }))).toBe('WillowRpcError:-32602');
  });

  it('pages: a full page covers up to its last event, and the pages add up', async () => {
    const { w, client } = setup();
    let r = await page(client, { startLedger: 64100, endLedger: 64103, pagination: { limit: 4 } });
    // Four fee events fill the first page: nothing committed is in it.
    expect(r.events).toEqual([]);
    expect(r.cursor).toBe(worldEventId(64100, 0, 0, 3));
    const got = [];
    for (let k = 0; k < 40 && parseEventId(r.cursor)!.l <= 64102; k++) {
      r = await page(client, { pagination: { cursor: r.cursor, limit: 4 } });
      got.push(...r.events);
      expect(w.calls.filter((c) => c.method === 'getEvents').at(-1)!.params.endLedger).toBeUndefined();
    }
    expect(got.filter((e: any) => e.ledger <= 64102)).toEqual(committed(w, 64100, 64102));
  });

  it('ends at the newest anchored ledger; flag keeps the rest, marked', async () => {
    const { w, client } = setup();
    const r = await page(client, { startLedger: 64300, pagination: { limit: 10000 } });
    expect(r.events).toEqual(committed(w, 64300, ANCHORED));
    expect(r.cursor).toBe(worldEventId(ANCHORED, 0xfffff, 0xfff, 0xffffffff));
    const f = setup({ unverifiable: 'flag' });
    const r2 = await page(f.client, { startLedger: 64300, pagination: { limit: 10000 } });
    expect(r2.events.at(-1).ledger).toBe(LATEST);
    expect(f.last().flags!.map((x) => x.kind)).toEqual(expect.arrayContaining(['pending', 'unprovable']));
    // Nothing anchored at all: an empty page that starts the same place next time.
    const e = await page(client, { startLedger: 64390 });
    expect(e).toMatchObject({ events: [], cursor: worldEventId(64389, 0xfffff, 0xfff, 0xffffffff) });
  });

  it('asks for no more ledgers than it verifies (maxScanLedgers)', async () => {
    const { w, client } = setup({ maxScanLedgers: 10 });
    const r = await page(client, { startLedger: 64100, pagination: { limit: 10000 } });
    expect(w.calls.find((c) => c.method === 'getEvents')!.params.endLedger).toBe(64110);
    expect(r.cursor).toBe(worldEventId(64109, 0xfffff, 0xfff, 0xffffffff));
    expect(r.events).toEqual(committed(w, 64100, 64109));
    // By cursor the request can't name an end: the page is cut here instead (10 ledgers from the cursor's).
    const r2 = await page(client, { pagination: { cursor: r.cursor, limit: 10000 } });
    expect(r2.cursor).toBe(worldEventId(64118, 0xfffff, 0xfff, 0xffffffff));
    expect(r2.events).toEqual(committed(w, 64110, 64118));
  });

  it('refuses a left-out, an extra, a changed and an unasked-for event', async () => {
    const lie = async (f: (r: any) => void, filters?: unknown) => {
      const { w, client } = setup();
      w.tamper = (m, _p, r) => {
        if (m === 'getEvents') f(r);
        return r;
      };
      return kindOf(page(client, { startLedger: 64100, endLedger: 64103, filters }));
    };
    const isCommitted = (e: any) => e.transactionIndex === 1;
    expect(await lie((r) => r.events.splice(r.events.findIndex(isCommitted), 1))).toBe(failed('event-missing'));
    expect(await lie((r) => (r.events = []))).toBe(failed('event-missing'));
    expect(
      await lie((r) => {
        const k = r.events.findIndex((e: any) => e.transactionIndex === 2);
        r.events.splice(k, 0, { ...r.events[k - 1], id: worldEventId(64100, 1, 0, 7) });
      }),
    ).toBe(failed('event-extra'));
    expect(await lie((r) => (r.events.find(isCommitted).value = b64('i128:1')))).toBe(failed('event-mismatch'));
    expect(await lie((r) => (r.events.find(isCommitted).topic = []))).toBe(failed('event-mismatch'));
    expect(await lie((r) => (r.events.find(isCommitted).txHash = 'ab'.repeat(32)))).toBe(failed('event-mismatch'));
    // A proven CONTRACT_A event served to a filter for CONTRACT_B.
    expect(
      await lie(
        (r) => {
          const w = new StellarWorld(FIRST, LATEST, ANCHORED);
          const a = w.committedEvents(64100).find((e) => e.contractId === CONTRACT_A)!;
          r.events.unshift(a);
        },
        [{ contractIds: [CONTRACT_B] }],
      ),
    ).toBe(failed('event-extra'));
    expect(await lie((r) => r.events.reverse())).toBe(failed('page'));
    // A cursor before an event it serves.
    expect(await lie((r) => (r.cursor = worldEventId(64100, 0, 0, 1)))).toBe(failed('page'));
  });

  it('an event no proof covers is left out (throw) or marked (flag), never passed as verified', async () => {
    // The first event served is a fee event, at (0, 0): no proof covers it.
    const forge = (w: StellarWorld) =>
      (w.tamper = (m, _p, r) => {
        if (m === 'getEvents') r.events[0].value = b64('forged');
        return r;
      });
    const t = setup();
    forge(t.w);
    const r = await page(t.client, { startLedger: 64100, endLedger: 64101 });
    expect(r.events.some((e: any) => e.value === b64('forged'))).toBe(false);
    const f = setup({ unverifiable: 'flag' });
    forge(f.w);
    const r2 = await page(f.client, { startLedger: 64100, endLedger: 64101 });
    expect(r2.events.some((e: any) => e.value === b64('forged'))).toBe(true);
    expect(f.last()).toMatchObject({ status: 'unverifiable', kind: 'unprovable' });
  });
});

describe('the other methods and the policy', () => {
  it('getHealth, getNetwork and getVersionInfo pass through, unproven', async () => {
    const { w, client, last } = setup();
    for (const method of ['getHealth', 'getNetwork', 'getVersionInfo']) {
      expect(await client.request({ method })).toBeTruthy();
      expect(last()).toMatchObject({ method, status: 'unproven' });
    }
    expect(w.calls.find((c) => c.method === 'getHealth')!.params).toBeUndefined();
    w.passphrase = 'Test SDF Network ; September 2015';
    expect(await kindOf(client.request({ method: 'getNetwork' }))).toBe(failed('wrong-network'));
  });

  it('refuses getLedgerEntries, simulateTransaction and sendTransaction; other methods are unverifiable', async () => {
    const { client } = setup();
    for (const method of ['getLedgerEntries', 'simulateTransaction', 'sendTransaction']) {
      expect(await kindOf(client.request({ method, params: {} }))).toBe('WillowRpcError:-32601');
    }
    expect(await kindOf(client.request({ method: 'getFeeStats' }))).toBe(unverifiable('method'));
    const f = setup({ unverifiable: 'flag' });
    expect(await f.client.request({ method: 'getFeeStats' })).toEqual({ latestLedger: LATEST });
    expect(f.last()).toMatchObject({ status: 'unverifiable', kind: 'method' });
    expect(await kindOf(client.request({ method: 'getLedgers', params: [1] }))).toBe('WillowRpcError:-32602');
  });

  it('no proofs on this plan, or no verifier module: unverifiable; flag passes the server through', async () => {
    const { w, client } = setup();
    w.refuseProofs = true;
    expect(await kindOf(client.request({ method: 'getLedgers', params: { startLedger: 64100 } }))).toBe(unverifiable('proof-refused'));
    const n = setup({ verifiers: {} });
    expect(await kindOf(n.client.request({ method: 'getLedgers', params: { startLedger: 64100 } }))).toBe(unverifiable('no-verifier'));
    const f = setup({ verifiers: {}, unverifiable: 'flag' });
    const r = (await f.client.request({ method: 'getLedgers', params: { startLedger: 64100 } })) as any;
    expect(r.ledgers[0].metadataXdr).toBeDefined();
    expect(f.last()).toMatchObject({ status: 'unverifiable', kind: 'no-verifier' });
  });

  it('server errors pass through as refusals; proofs are reused', async () => {
    const { w, client, last } = setup();
    expect(await kindOf(client.request({ method: 'getEvents', params: { startLedger: 64100, pagination: { cursor: worldEventId(64100, 0, 0, 0) } } }))).toBe('WillowRpcError:-32602');
    await client.request({ method: 'getLedgers', params: { startLedger: 64100, pagination: { limit: 5 } } });
    const before = w.calls.filter((c) => c.method === 'willow_getLedgerProof').length;
    await client.request({ method: 'getEvents', params: { startLedger: 64100, endLedger: 64105 } });
    expect(w.calls.filter((c) => c.method === 'willow_getLedgerProof').length).toBe(before);
    expect(last().status).toBe('verified');
  });
});
