import type {
  AnchorFailure,
  AnchorVerified,
  BlockRendered,
  Failure,
  ItemFailure,
  ItemKind,
  ItemVerified,
  LogsVerified,
  Requested,
  StateKind,
  StateVerified,
  VerifierModules,
  Anchor,
} from './verifiers.js';

export type ItemOutcome = (ItemVerified & { rendered?: BlockRendered }) | ItemFailure;

/**
 * The verifiers as the client calls them: each call stateless but the light client's, so the same
 * calls run here or in a Web Worker (worker.ts). Results are the I6 JSON, parsed.
 */
export interface Backend {
  itemRequest(chainId: number, kind: ItemKind, target: string): Promise<Requested | ItemFailure>;
  /** `request`, `verify`, and for a block with `full`, `block(true)`, on one ItemVerifier. */
  itemVerify(chainId: number, kind: ItemKind, target: string, response: string, full?: boolean): Promise<ItemOutcome>;
  stateRequest(chainId: number, kind: StateKind, target: string): Promise<Requested | Failure>;
  stateVerify(chainId: number, kind: StateKind, target: string, response: string): Promise<StateVerified | Failure>;
  anchorRestore(saved: string, now: number): Promise<{ ok: boolean; kind?: string; message?: string }>;
  anchorBootstrap(response: string, now: number): Promise<{ ok: boolean; kind?: string; message?: string; period?: number }>;
  anchorUpdates(response: string): Promise<{ ok: boolean; kind?: string; message?: string; period?: number | null }>;
  /** `Anchor.verify`, and after a pass (with `wantSaved`) the committees to keep. A `need-bootstrap` resets the light client. */
  anchorVerify(chainId: number, number: number, hash: string, response: string, now: number, wantSaved?: boolean): Promise<{ result: AnchorVerified | AnchorFailure; saved: string | null }>;
  logsVerify(chainId: number, response: string, filter: string, proofs: Record<string, Uint8Array>): Promise<LogsVerified | Failure>;
  close(): void;
}

const parse = <T>(s: string): T => JSON.parse(s) as T;

// An OpenVM proof's first bytes ("OVM2"), only to decide whether to load the OpenVM module (worker.js).
const isOpenVm = (b: Uint8Array | undefined) => !!b && b.length >= 4 && b[0] === 0x4f && b[1] === 0x56 && b[2] === 0x4d && b[3] === 0x32;

/** A verifier module that is not there or did not load: the client can't verify with it (not a verdict). */
export class NoVerifierError extends Error {
  constructor(name: string, why: string) {
    super(`no ${name} verifier module: ${why}`);
    this.name = 'NoVerifierError';
  }
}

export const isNoVerifier = (e: unknown): boolean => e instanceof Error && (e.name === 'NoVerifierError' || /^no \w+ verifier module: /.test(e.message));

/** The verifiers in this thread, each module loaded on first use. */
export class InlineBackend implements Backend {
  private anchor: Anchor | null = null;
  private loaded = new Map<string, Promise<any>>();

  constructor(
    private readonly modules: VerifierModules,
    private readonly checkpoint: () => Promise<{ root: string; slot: number }>,
  ) {}

  private load<K extends keyof VerifierModules>(name: K): Promise<any> {
    let p = this.loaded.get(name);
    if (!p) {
      const f = this.modules[name] as (() => Promise<unknown>) | undefined;
      const loading: Promise<any> = (f ? Promise.resolve().then(() => f()) : Promise.reject(new Error('this build has none (pass createVerifier({verifiers}))'))).catch((e) => {
        // A failed load is tried again next time.
        this.loaded.delete(name);
        throw new NoVerifierError(name, e instanceof Error ? e.message : String(e));
      });
      p = loading;
      this.loaded.set(name, p);
    }
    return p;
  }

  async itemRequest(chainId: number, kind: ItemKind, target: string) {
    const { ItemVerifier } = await this.load('items');
    const v = new ItemVerifier(chainId);
    try {
      return parse<Requested | ItemFailure>(v.request(kind, target));
    } finally {
      v.free();
    }
  }

  async itemVerify(chainId: number, kind: ItemKind, target: string, response: string, full = false): Promise<ItemOutcome> {
    const { ItemVerifier } = await this.load('items');
    const v = new ItemVerifier(chainId);
    try {
      const req = parse<Requested | ItemFailure>(v.request(kind, target));
      if (!req.ok) return req;
      const r = parse<ItemVerified | ItemFailure>(v.verify(response));
      if (!r.ok || kind !== 'block' || !full) return r;
      const rendered = parse<BlockRendered | ItemFailure>(v.block(true));
      if (!rendered.ok) return rendered;
      return { ...r, rendered };
    } finally {
      v.free();
    }
  }

  async stateRequest(chainId: number, kind: StateKind, target: string) {
    const { StateVerifier } = await this.load('state');
    const v = new StateVerifier(chainId);
    try {
      return parse<Requested | Failure>(v.request(kind, target));
    } finally {
      v.free();
    }
  }

  async stateVerify(chainId: number, kind: StateKind, target: string, response: string) {
    const { StateVerifier } = await this.load('state');
    const v = new StateVerifier(chainId);
    try {
      const req = parse<Requested | Failure>(v.request(kind, target));
      if (!req.ok) return req;
      return parse<StateVerified | Failure>(v.verify(response));
    } finally {
      v.free();
    }
  }

  private async lightClient(): Promise<Anchor> {
    if (this.anchor) return this.anchor;
    const { Anchor } = await this.load('anchor');
    const cp = await this.checkpoint();
    this.anchor ??= new Anchor(cp.root, cp.slot);
    return this.anchor!;
  }

  async anchorRestore(saved: string, now: number) {
    return parse<{ ok: boolean }>((await this.lightClient()).restore(saved, now));
  }

  async anchorBootstrap(response: string, now: number) {
    return parse<{ ok: boolean }>((await this.lightClient()).bootstrap(response, now));
  }

  async anchorUpdates(response: string) {
    return parse<{ ok: boolean }>((await this.lightClient()).updates(response));
  }

  async anchorVerify(chainId: number, number: number, hash: string, response: string, now: number, wantSaved = true) {
    const a = await this.lightClient();
    const result = parse<AnchorVerified | AnchorFailure>(a.verify(chainId, number, hash, response, now));
    if (!result.ok && result.kind === 'need-bootstrap') a.reset();
    return { result, saved: result.ok && wantSaved ? a.saved() : null };
  }

  async logsVerify(chainId: number, response: string, filter: string, proofs: Record<string, Uint8Array>) {
    const { Verifier } = await this.load('logs');
    const v = new Verifier(chainId, '', '');
    let o: { keys(): string; check(k: Uint8Array, p: Uint8Array, pv: Uint8Array): string | undefined; free(): void } | null = null;
    try {
      let checkOpenVm: ((k: Uint8Array, p: Uint8Array, pv: Uint8Array) => string | undefined) | undefined;
      if (Object.values(proofs).some(isOpenVm)) {
        const { OpenVmVerifier } = await this.load('openvm');
        const ovm = new OpenVmVerifier();
        o = ovm;
        const k = parse<{ ok: boolean; leaf: string; l1: string }>(ovm.keys());
        const want = parse<{ pinned: { openvm: { leaf: string[]; segment: string[] } } }>(v.keys()).pinned.openvm;
        if (!k.ok || k.leaf !== want.leaf[0] || k.l1 !== want.segment[0]) {
          return { ok: false, kind: 'internal', message: 'the OpenVM module trusts other programs than the log verifier' } as Failure;
        }
        checkOpenVm = (key, proof, pv) => ovm.check(key, proof, pv);
      }
      return parse<LogsVerified | Failure>(v.verify(response, filter, (id: string) => proofs[id], () => {}, checkOpenVm));
    } finally {
      o?.free();
      v.free();
    }
  }

  close(): void {
    this.anchor?.free();
    this.anchor = null;
  }
}
