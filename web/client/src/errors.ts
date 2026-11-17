/** A proof that failed verification: the server lied, or Willow has a bug. Never passed through. */
export const VERIFICATION_FAILED = -32098;
/** A value the client could not verify, under `unverifiable: "throw"`. */
export const UNVERIFIABLE = -32099;

export interface WillowErrorData {
  /** What failed, by name (README "Error codes"). Decide by this, never by the message. */
  kind: string;
  /** What exactly; may carry text the server chose. */
  detail?: string;
  /** The app's method. */
  method?: string;
}

/** Code -32098: a proof failed verification. The app must stop: no value is returned. */
export class WillowVerificationError extends Error {
  readonly code = VERIFICATION_FAILED;
  readonly data: WillowErrorData;
  constructor(kind: string, detail?: string, method?: string) {
    super(`Willow's response failed verification (${kind})${detail ? `: ${detail}` : ''}`);
    this.name = 'WillowVerificationError';
    this.data = { kind, detail, method };
  }
}

/** Code -32099: the client could not verify a value, and `unverifiable` is `"throw"`. */
export class WillowUnverifiableError extends Error {
  readonly code = UNVERIFIABLE;
  readonly data: WillowErrorData;
  constructor(kind: string, detail?: string, method?: string) {
    super(`The response could not be verified (${kind})${detail ? `: ${detail}` : ''}`);
    this.name = 'WillowUnverifiableError';
    this.data = { kind, detail, method };
  }
}

/** A JSON-RPC error passed to the app: a server refusal, or a verified revert the client computed. */
export class WillowRpcError extends Error {
  readonly code: number;
  readonly data?: unknown;
  constructor(code: number, message: string, data?: unknown) {
    super(message);
    this.name = 'WillowRpcError';
    this.code = code;
    if (data !== undefined) this.data = data;
  }
}

/** The JSON-RPC error object for any of the client's errors. */
export function rpcErrorOf(e: unknown): { code: number; message: string; data?: unknown } {
  if (e instanceof WillowVerificationError || e instanceof WillowUnverifiableError) {
    return { code: e.code, message: e.message, data: e.data };
  }
  if (e instanceof WillowRpcError) return e.data === undefined ? { code: e.code, message: e.message } : { code: e.code, message: e.message, data: e.data };
  return { code: -32603, message: e instanceof Error ? e.message : String(e) };
}
