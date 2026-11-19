// The Stellar client with the REAL Stellar module against a live endpoint, end to end, and the
// lying server on real data. Needs STELLAR_WASM_DIR (wasm-bindgen's stellar.js and
// stellar_bg.wasm, or a verify page build) and STELLAR_RPC_URL (the full RPC URL, key included);
// skipped otherwise:
//   STELLAR_WASM_DIR=… STELLAR_RPC_URL=https://api.willow.tech/v1/rpc/stellar/<key> npx vitest run test/stellar.real.test.ts
import { readdirSync } from 'node:fs';
import { join } from 'node:path';
import { pathToFileURL } from 'node:url';
import { describe, expect, it } from 'vitest';
import { WillowVerificationError } from '../src/errors.js';
import { createStellarVerifier, parseEventId, type StellarOptions, type StellarVerificationEvent } from '../src/stellar.js';
import type { StellarVerified, StellarVerifier, VerifierModules } from '../src/verifiers.js';
import { modulesFrom } from '../src/wasm.js';

const DIR = process.env.STELLAR_WASM_DIR;
const URL_ = process.env.STELLAR_RPC_URL;
const PINNED = '040355b75766e6799d1ef69ce03f114530a06d922dd64b5e885da907a453841d';
const SLOW = 900_000;

function modules(dir: string): VerifierModules {
  return modulesFrom((name) => {
    const files = readdirSync(dir);
    const pick = (plain: string, ext: string) => {
      const f = files.includes(plain) ? plain : files.find((x) => new RegExp(`^${name}\\.[0-9a-f]+\\.${ext}$`).test(x));
      if (!f) throw new Error(`no ${name} module in ${dir}`);
      return pathToFileURL(join(dir, f)) as URL;
    };
    return { glue: pick(`${name}.js`, 'js'), wasm: pick(`${name}_bg.wasm`, 'wasm') };
  });
}

/** A fetch that applies `mutate` to the result of every `method` call (the lying server). */
function lying(method: string, mutate: (result: any) => void): typeof fetch {
  return async (url, init) => {
    const res = await fetch(url, init);
    const body = JSON.parse(String(init?.body));
    const text = await res.text();
    const sent = Array.isArray(body) ? body : [body];
    if (!sent.some((r: any) => r.method === method)) return new Response(text, { status: res.status, headers: res.headers });
    const parsed = JSON.parse(text);
    for (const item of Array.isArray(parsed) ? parsed : [parsed]) {
      const req = sent.find((r: any) => r.id === item.id);
      if (req?.method === method && item.result) mutate(item.result);
    }
    return new Response(JSON.stringify(parsed), { status: res.status, headers: { 'content-type': 'application/json' } });
  };
}

async function rpc(method: string, params?: unknown): Promise<any> {
  const res = await fetch(URL_!, { method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify({ jsonrpc: '2.0', id: 1, method, params }) });
  return res.json();
}

const flipB64 = (s: string, at: number) => s.slice(0, at) + (s[at] === 'A' ? 'B' : 'A') + s.slice(at + 1);

describe.skipIf(!DIR || !URL_)('the real Stellar module on live data', () => {
  const verifiers = DIR ? modules(DIR) : {};
  const client = (o: Partial<StellarOptions> = {}) => {
    const events: StellarVerificationEvent[] = [];
    const c = createStellarVerifier({ url: URL_, verifiers, onVerification: (e) => events.push(e), ...o });
    return { c, events, last: () => events[events.length - 1] };
  };
  const mod = async (): Promise<StellarVerifier> => verifiers.stellar!();
  let anchored = 0;
  /** 16 real ledgers, verified straight from willow_getLedgerProof with their transaction sets. */
  let sample: StellarVerified['verified'] | null = null;

  it('trusts the Tier-1 quorum set', async () => {
    expect(JSON.parse((await mod()).pinnedQuorumSet())).toMatchObject({ ok: true, hash: PINNED, maxLedgers: 64 });
  });

  it('getLatestLedger: the newest anchored ledger', async () => {
    const { c, last } = client();
    const l = (await c.request({ method: 'getLatestLedger' })) as any;
    anchored = l.sequence;
    expect(last()).toMatchObject({ status: 'verified', ledger: { sequence: anchored } });
    expect(last().behind).toBeGreaterThanOrEqual(0);
    expect(last().behind).toBeLessThan(200);
    expect(last().anchor!.signedEntries).toBeGreaterThanOrEqual(7);
    console.log(`anchored ${anchored}, ${last().behind} behind the server's ${last().serverLatest}; ${last().anchor!.signers} signers from ${last().anchor!.source}`);
  }, SLOW);

  it('256 ledgers straight from willow_getLedgerProof', async () => {
    const m = await mod();
    let ledgers = 0;
    const end = anchored - (anchored % 64) - 2;
    for (let w = 0; w < 4; w++) {
      const b = end - 64 * w;
      const r = await rpc('willow_getLedgerProof', { startLedger: b - 63, endLedger: b });
      const v = JSON.parse(m.verifyLedgerBundle(JSON.stringify(r)));
      expect(v.ok, v.message).toBe(true);
      ledgers += v.verified.ledgers.length;
    }
    expect(ledgers).toBe(256);
    // With transaction sets: at most 16 ledgers a bundle.
    const r = await rpc('willow_getLedgerProof', { startLedger: end - 15, endLedger: end, transactions: true });
    const v = JSON.parse(m.verifyLedgerBundle(JSON.stringify(r)));
    expect(v.ok, v.message).toBe(true);
    expect(v.verified.ledgers.every((l: any) => l.transactions.every((t: any) => t.envelopeXdr))).toBe(true);
    sample = v.verified;
  }, SLOW);

  it('getEvents over 256 ledgers, page by page, complete', async () => {
    const { c, events } = client({ maxScanLedgers: 64 });
    const from = anchored - 300;
    let r = (await c.request({ method: 'getEvents', params: { startLedger: from, pagination: { limit: 10000 } } })) as any;
    let got = r.events.length;
    for (let k = 0; k < 200 && parseEventId(r.cursor)!.l < from + 255; k++) {
      r = await c.request({ method: 'getEvents', params: { pagination: { cursor: r.cursor, limit: 10000 } } });
      got += r.events.length;
    }
    const covered = new Set<number>();
    for (const e of events) for (let s = e.ledgers!.from; s <= e.ledgers!.to; s++) covered.add(s);
    expect(events.every((e) => e.status === 'verified')).toBe(true);
    expect(covered.size).toBeGreaterThanOrEqual(256);
    console.log(`${got} committed events over ${covered.size} verified ledgers; ${events.reduce((n, e) => n + (e.dropped ?? 0), 0)} unprovable events left out`);
  }, SLOW);

  it('getLedgers, getTransactions, getTransaction', async () => {
    const { c, last } = client();
    const at = sample!.ledgers[10].sequence;
    const ls = (await c.request({ method: 'getLedgers', params: { startLedger: at, pagination: { limit: 3 } } })) as any;
    expect(ls.ledgers.map((l: any) => l.hash)).toEqual(sample!.ledgers.slice(10, 13).map((l) => l.hash));
    expect(last().status).toBe('verified');
    const txs = (await c.request({ method: 'getTransactions', params: { startLedger: at, pagination: { limit: 200 } } })) as any;
    expect(txs.transactions.length).toBeGreaterThan(0);
    expect(last().status).toBe('verified');
    const l = sample!.ledgers.find((x) => x.invocations.some((i) => i.eventsXdr.length))!;
    const inv = l.invocations.find((i) => i.eventsXdr.length)!;
    const t = l.transactions[inv.tx - 1];
    const got = (await c.request({ method: 'getTransaction', params: { hash: t.txHash } })) as any;
    expect(got).toMatchObject({ status: t.status, txHash: t.txHash, resultXdr: t.resultXdr, envelopeXdr: t.envelopeXdr, ledger: l.sequence });
    expect(got.events.contractEventsXdr[inv.op]).toEqual(inv.eventsXdr);
    expect(last().status).toBe('verified');
  }, SLOW);

  it('the lying server on real data is refused', async () => {
    const l = sample!.ledgers.find((x) => x.events.length >= 2)!;
    const params = { startLedger: l.sequence, endLedger: l.sequence + 1, pagination: { limit: 10000 } };
    const ids = new Set(l.events.map((e) => e.id));
    const committed = (r: any) => r.events.findIndex((e: any) => ids.has(e.id));
    const refused = async (method: string, mutate: (r: any) => void, req: { method: string; params?: unknown }, kind: string) => {
      const { c } = client({ fetch: lying(method, mutate) });
      const err = await c.request(req).then(
        () => null,
        (e) => e,
      );
      expect(err, `${kind}: passed`).toBeInstanceOf(WillowVerificationError);
      expect(err.data.kind).toBe(kind);
    };
    const ev = { method: 'getEvents', params };
    await refused('getEvents', (r) => r.events.splice(committed(r), 1), ev, 'event-missing');
    await refused('getEvents', (r) => (r.events[committed(r)].value = 'AAAAAQ=='), ev, 'event-mismatch');
    await refused(
      'getEvents',
      (r) => {
        // After the last event of a proven invocation, one more at its place.
        const k = committed(r);
        const toid = r.events[k].id.split('-')[0];
        let j = k;
        while (j + 1 < r.events.length && r.events[j + 1].id.startsWith(`${toid}-`)) j++;
        const i = parseEventId(r.events[j].id)!.i + 1000;
        r.events.splice(j + 1, 0, { ...r.events[k], id: `${toid}-${String(i).padStart(10, '0')}` });
      },
      ev,
      'event-extra',
    );
    const t = l.transactions[0];
    await refused('getTransaction', (r) => (r.resultXdr = flipB64(r.resultXdr, 8)), { method: 'getTransaction', params: { hash: t.txHash } }, 'transaction-mismatch');
    await refused('getLedgers', (r) => (r.ledgers[0].hash = r.ledgers[0].hash.replace(/^./, (x: string) => (x === '0' ? '1' : '0'))), { method: 'getLedgers', params: { startLedger: l.sequence, pagination: { limit: 1 } } }, 'ledger-mismatch');
    await refused('getTransactions', (r) => r.transactions.splice(1, 1), { method: 'getTransactions', params: { startLedger: l.sequence, pagination: { limit: 5 } } }, 'transactions');
    const ledgers = { method: 'getLedgers', params: { startLedger: l.sequence - 2, pagination: { limit: 3 } } };
    await refused('willow_getLedgerProof', (b) => (b.ledgers[1].header = flipB64(b.ledgers[1].header, 40)), ledgers, 'ledger-proof');
    await refused('willow_getLedgerProof', (b) => b.anchor.envelopes.splice(3), ledgers, 'ledger-proof');
    await refused('willow_getLedgerProof', (b) => b.ledgers[b.ledgers.length - 1].preimages.reverse().push('AAAA'), ledgers, 'ledger-proof');
  }, SLOW);
});
