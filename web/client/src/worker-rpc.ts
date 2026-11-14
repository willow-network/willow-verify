import type { AnchorStart, Backend } from './backend.js';

/** Either end of a worker: a Web Worker, its global scope, or a MessagePort (Node's too). */
export interface Port {
  postMessage(message: unknown, transfer?: Transferable[]): void;
  addEventListener?(type: 'message', listener: (e: { data: unknown }) => void): void;
  on?(type: 'message', listener: (data: unknown) => void): void;
  start?(): void;
  terminate?(): void;
  close?(): void;
}

type Op = Exclude<keyof Backend, 'close'>;
interface Call {
  id: number;
  op: Op | 'init';
  args: unknown[];
}
type Done = { id: number; ok: true; value: unknown } | { id: number; ok: false; message: string; name?: string };

function listen(port: Port, f: (data: any) => void): void {
  if (port.addEventListener) {
    port.addEventListener('message', (e) => f(e.data));
    port.start?.();
  } else if (port.on) port.on('message', f);
  else throw new Error('not a message port');
}

/**
 * The worker's side: runs each call on `make`'s backend, one at a time, in order (the light client
 * is one object), as web/verify/worker.js does. The checkpoint comes in an `init` call before the
 * first anchor call; only the light client needs it.
 */
export function serveBackend(port: Port, make: (checkpoint: () => Promise<AnchorStart>) => Backend): void {
  let checkpoint: AnchorStart | null = null;
  const backend = make(async () => {
    if (!checkpoint) throw new Error('the worker was not given its checkpoint');
    return checkpoint;
  });
  let queue = Promise.resolve();
  listen(port, (m: Call) => {
    queue = queue.then(async () => {
      let out: Done;
      try {
        if (m.op === 'init') {
          checkpoint ??= m.args[0] as AnchorStart;
          out = { id: m.id, ok: true, value: null };
        } else {
          const value = await (backend[m.op] as (...a: unknown[]) => Promise<unknown>)(...m.args);
          out = { id: m.id, ok: true, value };
        }
      } catch (e) {
        out = { id: m.id, ok: false, message: e instanceof Error ? e.message : String(e), name: e instanceof Error ? e.name : undefined };
      }
      port.postMessage(out);
    });
  });
}

/** The client's side of a worker running {@link serveBackend}. */
export class WorkerBackend implements Backend {
  private next = 1;
  private waiting = new Map<number, { resolve: (v: any) => void; reject: (e: unknown) => void }>();
  private ready: Promise<unknown> | null = null;

  constructor(
    private readonly port: Port,
    private readonly checkpoint: () => Promise<AnchorStart>,
  ) {
    listen(port, (d: Done) => {
      const w = this.waiting.get(d.id);
      if (!w) return;
      this.waiting.delete(d.id);
      if (d.ok) w.resolve(d.value);
      else {
        const e = new Error(d.message);
        if (d.name) e.name = d.name;
        w.reject(e);
      }
    });
  }

  private post(op: Call['op'], args: unknown[], transfer: Transferable[] = []): Promise<any> {
    const id = this.next++;
    return new Promise((resolve, reject) => {
      this.waiting.set(id, { resolve, reject });
      this.port.postMessage({ id, op, args } satisfies Call, transfer);
    });
  }

  private call(op: Op, args: unknown[], transfer?: Transferable[]): Promise<any> {
    if (!op.startsWith('anchor')) return this.post(op, args, transfer);
    if (!this.ready) {
      const r = this.checkpoint().then((cp) => this.post('init', [cp]));
      this.ready = r;
      r.catch(() => {
        if (this.ready === r) this.ready = null;
      });
    }
    return this.ready.then(() => this.post(op, args, transfer));
  }

  itemRequest(...a: Parameters<Backend['itemRequest']>) {
    return this.call('itemRequest', a);
  }
  itemVerify(...a: Parameters<Backend['itemVerify']>) {
    return this.call('itemVerify', a);
  }
  stateRequest(...a: Parameters<Backend['stateRequest']>) {
    return this.call('stateRequest', a);
  }
  stateVerify(...a: Parameters<Backend['stateVerify']>) {
    return this.call('stateVerify', a);
  }
  anchorRestore(...a: Parameters<Backend['anchorRestore']>) {
    return this.call('anchorRestore', a);
  }
  anchorBootstrap(...a: Parameters<Backend['anchorBootstrap']>) {
    return this.call('anchorBootstrap', a);
  }
  anchorUpdates(...a: Parameters<Backend['anchorUpdates']>) {
    return this.call('anchorUpdates', a);
  }
  anchorVerify(...a: Parameters<Backend['anchorVerify']>) {
    return this.call('anchorVerify', a);
  }
  // Proof bytes move to the worker (transferred, not copied), as the page does.
  logsVerify(...a: Parameters<Backend['logsVerify']>) {
    return this.call('logsVerify', a, [...new Set(Object.values(a[3]).map((b) => b.buffer as ArrayBuffer))]);
  }

  close(): void {
    for (const w of this.waiting.values()) w.reject(new Error('the verifier worker was closed'));
    this.waiting.clear();
    this.port.terminate?.();
    this.port.close?.();
  }
}
