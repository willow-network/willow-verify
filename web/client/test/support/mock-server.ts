// A JSON-RPC server over a World, at /v1/rpc/{chainId}/{key}, responding to batches as Willow does.
import { createServer, type IncomingMessage, type Server } from 'node:http';
import type { AddressInfo } from 'node:net';
import { toNumber, toQuantity } from '../../src/hex.js';
import { World } from './world.js';

export interface Running {
  base: string;
  url: (chainId: number, key?: string) => string;
  close: () => Promise<void>;
}

const STATE_METHODS = new Set(['willow_getStateProof', 'willow_getCallProof', 'willow_getEstimateGasProof', 'willow_getTraceProof', 'eth_getBalance', 'eth_call', 'eth_estimateGas']);

export function readBody(req: IncomingMessage): Promise<string> {
  return new Promise((resolve, reject) => {
    const parts: Buffer[] = [];
    req.on('data', (c) => parts.push(c));
    req.on('end', () => resolve(Buffer.concat(parts).toString('utf8')));
    req.on('error', reject);
  });
}

export function respond(world: World, chainId: number, key: string, it: { id: unknown; method: string; params?: unknown }): unknown {
  const ok = (result: unknown) => ({ jsonrpc: '2.0', id: it.id, result });
  const err = (code: number, message: string, data?: unknown) => ({ jsonrpc: '2.0', id: it.id, error: data === undefined ? { code, message } : { code, message, data } });
  const params = (Array.isArray(it.params) ? it.params : []) as unknown[];
  const hook = world.intercept?.(chainId, it.method, params);
  if (hook) return 'error' in hook ? err(hook.error.code, hook.error.message, hook.error.data) : ok(hook.result);
  if (key === 'public' && STATE_METHODS.has(it.method)) {
    return err(-32601, `the method ${it.method} is not served on the public path, use an API key`);
  }
  const std = world.standard.get(world.key(chainId, it.method, params)) ?? world.standard.get(`${chainId}|${it.method}|*`);
  if (std) return std.error ? err(std.error.code, std.error.message, std.error.data) : ok(std.result);
  switch (it.method) {
    case 'eth_chainId':
      return ok(toQuantity(chainId));
    case 'willow_getBlockAnchor': {
      const p = params[0];
      const real = world.realAnchors.get(`${chainId}:${String(p)}`);
      if (real) return ok(real);
      const e = p === 'latest' ? world.heads.get(chainId) : p === 'finalized' ? world.finalized.get(chainId) : world.anchors.get(`${chainId}:${toNumber(p)}`);
      return e ? ok(world.anchorBody(e)) : err(-32000, `not found in this store: no anchor for block ${String(p)}`);
    }
    case 'willow_lightClientBootstrap':
      return world.lightClient.bootstrap ? ok(world.lightClient.bootstrap) : err(-32601, 'no light client here');
    case 'willow_lightClientUpdates':
      return world.lightClient.updates ? ok(world.lightClient.updates) : err(-32601, 'no light client here');
    case 'willow_getStateProof': {
      const s = world.state(chainId, params);
      return s ? ok(s.result) : err(-32000, 'not found in this store: no state fixture');
    }
    case 'willow_getTransactionProof':
    case 'willow_getReceiptProof':
    case 'willow_getBlockReceiptsProof':
    case 'willow_getBlockProof':
    case 'willow_getFeeProof':
    case 'willow_getFeeHistoryProof':
    case 'willow_getCallProof':
    case 'willow_getEstimateGasProof':
    case 'willow_getTraceProof': {
      const e = world.proofs.get(world.key(chainId, it.method, params));
      if (e) return ok(e.result);
      if ((it.method === 'willow_getTransactionProof' || it.method === 'willow_getReceiptProof') && params.length === 1) {
        return err(-32000, `not found in this store: transaction ${String(params[0])} is not in the full-block tier`);
      }
      return err(-32000, `not found in this store: no fixture for ${it.method} ${JSON.stringify(params).slice(0, 200)}`);
    }
    default:
      return err(-32601, `the method ${it.method} does not exist/is not available`);
  }
}

export async function serve(world: World): Promise<Running> {
  const server: Server = createServer(async (req, res) => {
    const m = /^\/v1\/rpc\/(\d+)\/([^/]+)\/?$/.exec(req.url ?? '');
    if (!m || req.method !== 'POST') {
      res.writeHead(404).end();
      return;
    }
    const chainId = Number(m[1]);
    const key = decodeURIComponent(m[2]);
    let body: any;
    try {
      body = JSON.parse(await readBody(req));
    } catch {
      res.writeHead(200, { 'content-type': 'application/json' }).end(JSON.stringify({ jsonrpc: '2.0', id: null, error: { code: -32700, message: 'parse error' } }));
      return;
    }
    const items = Array.isArray(body) ? body : [body];
    world.log.push({ chainId, items: items.map((i: any) => ({ method: i.method, params: i.params })) });
    const out = items.map((i: any) => respond(world, chainId, key, i));
    res.writeHead(200, { 'content-type': 'application/json' }).end(JSON.stringify(Array.isArray(body) ? out : out[0]));
  });
  await new Promise<void>((r) => server.listen(0, '127.0.0.1', r));
  const base = `http://127.0.0.1:${(server.address() as AddressInfo).port}`;
  return {
    base,
    url: (chainId, key = 'test-key') => `${base}/v1/rpc/${chainId}/${key}`,
    close: () => new Promise((r) => server.close(() => r())),
  };
}
