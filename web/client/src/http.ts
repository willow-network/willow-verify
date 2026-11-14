import { WillowRpcError, WillowVerificationError } from './errors.js';

/** The most items in one JSON-RPC batch (the server's `MAX_BATCH`, rpc.rs). */
export const MAX_BATCH = 100;

/** One item of a batch response: its exact text (passed to the wasm as sent) and its parse. */
export interface Reply {
  text: string;
  value: { id?: unknown; result?: unknown; error?: { code?: unknown; message?: unknown; data?: unknown } };
}

export interface BatcherOptions {
  fetch: typeof fetch;
  windowMs: number;
  maxBatch?: number;
  timeoutMs?: number;
  headers?: Record<string, string>;
}

interface Entry {
  method: string;
  params: unknown[];
  resolve: (r: Reply) => void;
  reject: (e: unknown) => void;
}

const RATE_LIMIT_WAITS_MS = [1000, 2000];

/**
 * The elements of a top-level JSON array, each as its exact text, or null when `text` is not
 * one. Strings and nesting are tracked; JSON.parse validates each element afterwards.
 */
export function splitArray(text: string): string[] | null {
  let i = 0;
  const ws = () => {
    while (i < text.length && ' \t\r\n'.includes(text[i])) i++;
  };
  ws();
  if (text[i] !== '[') return null;
  i++;
  const out: string[] = [];
  ws();
  if (text[i] === ']') {
    i++;
    ws();
    return i === text.length ? out : null;
  }
  for (;;) {
    ws();
    const start = i;
    let depth = 0;
    let inString = false;
    for (; i < text.length; i++) {
      const c = text[i];
      if (inString) {
        if (c === '\\') i++;
        else if (c === '"') inString = false;
      } else if (c === '"') inString = true;
      else if (c === '{' || c === '[') depth++;
      else if (c === '}' || c === ']') {
        if (depth === 0) break;
        depth--;
      } else if (c === ',' && depth === 0) break;
    }
    if (i >= text.length || inString) return null;
    out.push(text.slice(start, i).trimEnd());
    if (text[i] === ',') {
      i++;
      continue;
    }
    i++;
    ws();
    return i === text.length ? out : null;
  }
}

const sleep = (ms: number) => new Promise((r) => setTimeout(r, ms));

/**
 * JSON-RPC over HTTP, batched: calls made within `windowMs` of each other go out as one batch per
 * URL (at most `maxBatch` items), and identical calls in flight share one item.
 */
export class Batcher {
  private queues = new Map<string, Entry[]>();
  private timers = new Map<string, ReturnType<typeof setTimeout>>();
  private inflight = new Map<string, Promise<Reply>>();
  /** HTTP requests sent and items in them, for tests and benchmarks. */
  readonly sent: { url: string; items: number; bytes: number }[] = [];

  constructor(private readonly o: BatcherOptions) {}

  call(url: string, method: string, params: unknown[]): Promise<Reply> {
    const key = `${url}\n${method}\n${JSON.stringify(params)}`;
    const same = this.inflight.get(key);
    if (same) return same;
    const p = new Promise<Reply>((resolve, reject) => {
      const q = this.queues.get(url) ?? [];
      q.push({ method, params, resolve, reject });
      this.queues.set(url, q);
      if (q.length >= (this.o.maxBatch ?? MAX_BATCH)) this.flush(url);
      else if (!this.timers.has(url)) this.timers.set(url, setTimeout(() => this.flush(url), this.o.windowMs));
    });
    this.inflight.set(key, p);
    const done = () => {
      if (this.inflight.get(key) === p) this.inflight.delete(key);
    };
    p.then(done, done);
    return p;
  }

  /** One request on its own (not batched): for responses too large to share a batch, such as proofs of megabytes. */
  async single(url: string, method: string, params: unknown[]): Promise<Reply> {
    const { text, status } = await this.post(url, JSON.stringify({ jsonrpc: '2.0', id: 1, method, params }));
    this.sent.push({ url, items: 1, bytes: text.length });
    let value: any;
    try {
      value = JSON.parse(text);
    } catch {
      throw status < 200 || status >= 300
        ? new WillowRpcError(-32603, `${hostOf(url)} responded HTTP ${status}`)
        : new WillowVerificationError('batch', `the response to ${method} is not JSON`, method);
    }
    if (!value || typeof value !== 'object' || Array.isArray(value) || (value.result === undefined && (typeof value.error !== 'object' || value.error === null))) {
      throw new WillowVerificationError('batch', `the response to ${method} has neither a result nor an error`, method);
    }
    return { text, value };
  }

  private flush(url: string): void {
    const t = this.timers.get(url);
    if (t) clearTimeout(t);
    this.timers.delete(url);
    const q = this.queues.get(url) ?? [];
    this.queues.delete(url);
    const max = this.o.maxBatch ?? MAX_BATCH;
    for (let i = 0; i < q.length; i += max) void this.send(url, q.slice(i, i + max));
  }

  private async send(url: string, batch: Entry[]): Promise<void> {
    const body = JSON.stringify(batch.map((e, i) => ({ jsonrpc: '2.0', id: i + 1, method: e.method, params: e.params })));
    const failAll = (e: unknown) => batch.forEach((x) => x.reject(e));
    let text: string;
    let status: number;
    try {
      ({ text, status } = await this.post(url, body));
    } catch (e) {
      return failAll(e);
    }
    this.sent.push({ url, items: batch.length, bytes: text.length });
    const items = splitArray(text);
    if (!items) {
      let v: any = null;
      try {
        v = JSON.parse(text);
      } catch {}
      if (v && typeof v === 'object' && v.error && typeof v.error === 'object') {
        return failAll(new WillowRpcError(Number(v.error.code) || -32603, String(v.error.message ?? ''), v.error.data));
      }
      if (status < 200 || status >= 300) return failAll(new WillowRpcError(-32603, `${hostOf(url)} responded HTTP ${status}`));
      return failAll(new WillowVerificationError('batch', `the response to a batch of ${batch.length} is not a JSON-RPC batch`));
    }
    const byId = new Map<number, Reply>();
    const twice = new Set<number>();
    for (const t of items) {
      let value: any;
      try {
        value = JSON.parse(t);
      } catch {
        continue;
      }
      const id = value && typeof value === 'object' ? value.id : undefined;
      if (typeof id !== 'number') continue;
      if (byId.has(id)) twice.add(id);
      byId.set(id, { text: t, value });
    }
    batch.forEach((e, i) => {
      const id = i + 1;
      const r = byId.get(id);
      if (twice.has(id)) e.reject(new WillowVerificationError('batch', `the batch response holds item ${id} twice`, e.method));
      else if (!r) e.reject(new WillowVerificationError('batch', `the batch response has no item ${id} (${e.method})`, e.method));
      else if (r.value.result === undefined && (r.value.error === undefined || typeof r.value.error !== 'object' || r.value.error === null)) {
        e.reject(new WillowVerificationError('batch', `item ${id} has neither a result nor an error`, e.method));
      } else e.resolve(r);
    });
  }

  private async post(url: string, body: string): Promise<{ text: string; status: number }> {
    for (let attempt = 0; ; attempt++) {
      let res: Response;
      try {
        res = await this.o.fetch(url, {
          method: 'POST',
          headers: { 'content-type': 'application/json', accept: 'application/json', ...this.o.headers },
          body,
          signal: AbortSignal.timeout(this.o.timeoutMs ?? 120_000),
          credentials: 'omit',
          referrerPolicy: 'no-referrer',
        } as RequestInit);
      } catch (e) {
        throw new WillowRpcError(-32603, `${hostOf(url)} could not be reached: ${e instanceof Error ? e.message : String(e)}`);
      }
      if (res.status === 429 && attempt < RATE_LIMIT_WAITS_MS.length) {
        await res.text().catch(() => '');
        const after = Number(res.headers.get('retry-after'));
        await sleep(after > 0 && after <= 10 ? after * 1000 : RATE_LIMIT_WAITS_MS[attempt]);
        continue;
      }
      return { text: await res.text(), status: res.status };
    }
  }
}

export function hostOf(url: string): string {
  try {
    return new URL(url).host;
  } catch {
    return url;
  }
}
