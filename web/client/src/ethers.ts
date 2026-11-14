import { JsonRpcApiProvider, Network, type JsonRpcError, type JsonRpcPayload, type JsonRpcResult } from 'ethers';
import { createVerifier, type WillowOptions, type WillowVerifier } from './client.js';
import { rpcErrorOf, WillowUnverifiableError, WillowVerificationError } from './errors.js';
import { MAX_BATCH } from './http.js';

/**
 * An ethers v6 provider that verifies every response: `new WillowProvider({ chainId: 1, key })`.
 * Its network is fixed to `chainId`; ethers' batches go to the verifier as they are.
 */
export class WillowProvider extends JsonRpcApiProvider {
  readonly verifier: WillowVerifier;
  readonly #errors = new Map<number, Error>();

  constructor(options: WillowOptions) {
    const network = Network.from(options.chainId);
    super(network, { staticNetwork: network, batchMaxCount: MAX_BATCH });
    this.verifier = createVerifier(options);
  }

  async _send(payload: JsonRpcPayload | JsonRpcPayload[]): Promise<(JsonRpcResult | JsonRpcError)[]> {
    const list = Array.isArray(payload) ? payload : [payload];
    return Promise.all(
      list.map(async (p): Promise<JsonRpcResult | JsonRpcError> => {
        try {
          return { id: p.id, result: await this.verifier.request({ method: p.method, params: p.params }) };
        } catch (e) {
          if (e instanceof WillowVerificationError || e instanceof WillowUnverifiableError) this.#errors.set(p.id, e);
          return { id: p.id, error: rpcErrorOf(e) };
        }
      }),
    );
  }

  /** The client's own errors reach the app as they are (not wrapped into an ethers error). */
  override getRpcError(payload: JsonRpcPayload, error: JsonRpcError): Error {
    const e = this.#errors.get(payload.id);
    if (e) {
      this.#errors.delete(payload.id);
      return e;
    }
    return super.getRpcError(payload, error);
  }

  override destroy(): void {
    this.verifier.close();
    super.destroy();
  }
}

export { createVerifier } from './client.js';
export type { VerificationEvent, WillowOptions, WillowVerifier } from './client.js';
export { WillowRpcError, WillowUnverifiableError, WillowVerificationError } from './errors.js';
