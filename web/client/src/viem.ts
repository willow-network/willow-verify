import { custom, type CustomTransport, type CustomTransportConfig } from 'viem';
import { createVerifier, type WillowOptions, type WillowVerifier } from './client.js';

export type WillowTransportOptions = WillowOptions & {
  /** viem's own transport settings. Retries default to 0: a failed verification is never retried. */
  transport?: CustomTransportConfig;
};

/**
 * A viem `custom()` (EIP-1193) transport that verifies every response:
 * `createPublicClient({ chain: mainnet, transport: willowTransport({ chainId: 1, key }) })`.
 */
export function willowTransport(options: WillowTransportOptions | WillowVerifier): CustomTransport {
  const v = 'request' in options && typeof options.request === 'function' ? (options as WillowVerifier) : createVerifier(options as WillowOptions);
  const config = 'transport' in options ? (options as WillowTransportOptions).transport : undefined;
  return custom({ request: (args: { method: string; params?: unknown }) => v.request(args) }, { key: 'willow', name: 'Willow (verified)', retryCount: 0, ...config });
}

export { createVerifier } from './client.js';
export type { VerificationEvent, WillowOptions, WillowVerifier } from './client.js';
export { WillowRpcError, WillowUnverifiableError, WillowVerificationError } from './errors.js';
