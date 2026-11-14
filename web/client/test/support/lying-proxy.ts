// The lying server (design 6.3, W7): an HTTP proxy in front of an upstream (the mock server now;
// api.willow.tech once W3 is live) that applies one mutation from a table to its responses.
import { createServer, type Server } from 'node:http';
import type { AddressInfo } from 'node:net';
import { readBody } from './mock-server.js';

export interface Req {
  method: string;
  params: unknown[];
}

export interface Mutation {
  name: string;
  /** Changes one response item (its `result` or `error`) in place; true when it applied. */
  item?: (resp: any, req: Req, chainId: number) => boolean;
  /** Changes the whole batch response in place; true when it applied. */
  batch?: (resps: any[], reqs: (Req & { id: unknown })[], chainId: number) => boolean;
}

export interface Proxy {
  url: (chainId: number, key?: string) => string;
  set: (m: Mutation | null) => void;
  /** How many times the mutation applied since `set`. */
  applied: () => number;
  /** Every response item the proxy passed on, by `${method} ${JSON.stringify(params)}` (for replays). */
  seen: Map<string, any[]>;
  close: () => Promise<void>;
}

export async function startProxy(upstream: string): Promise<Proxy> {
  let mutation: Mutation | null = null;
  let applied = 0;
  const seen = new Map<string, any[]>();
  const server: Server = createServer(async (req, res) => {
    const body = await readBody(req);
    const r = await fetch(upstream + (req.url ?? ''), { method: 'POST', headers: { 'content-type': 'application/json' }, body });
    let text = await r.text();
    const chainId = Number(/\/rpc\/(\d+)\//.exec(req.url ?? '')?.[1] ?? 0);
    try {
      const reqs = JSON.parse(body);
      const resps = JSON.parse(text);
      const list = Array.isArray(resps) ? resps : [resps];
      const asked: (Req & { id: unknown })[] = (Array.isArray(reqs) ? reqs : [reqs]).map((x: any) => ({ id: x.id, method: x.method, params: Array.isArray(x.params) ? x.params : [] }));
      for (const it of list) {
        const q = asked.find((a) => a.id === it.id);
        const k = q && `${q.method} ${JSON.stringify(q.params)}`;
        if (k) seen.set(k, [...(seen.get(k) ?? []), structuredClone(it)]);
      }
      if (mutation?.batch && mutation.batch(list, asked, chainId)) applied++;
      if (mutation?.item) {
        for (const it of list) {
          const q = asked.find((a) => a.id === it.id);
          if (q && mutation.item(it, q, chainId)) applied++;
        }
      }
      text = JSON.stringify(Array.isArray(resps) ? list : list[0]);
    } catch {}
    res.writeHead(r.status, { 'content-type': 'application/json' }).end(text);
  });
  await new Promise<void>((r) => server.listen(0, '127.0.0.1', r));
  const base = `http://127.0.0.1:${(server.address() as AddressInfo).port}`;
  return {
    url: (chainId, key = 'test-key') => `${base}/v1/rpc/${chainId}/${key}`,
    set: (m) => {
      mutation = m;
      applied = 0;
    },
    applied: () => applied,
    seen,
    close: () => new Promise((r) => server.close(() => r())),
  };
}

/** A hex string with one nibble near its end changed (W2's flip_last_hex). */
export function flip(h: string, fromEnd = 1): string {
  const i = h.length - fromEnd;
  const c = h[i];
  return h.slice(0, i) + (c === '0' ? '1' : '0') + h.slice(i + 1);
}
