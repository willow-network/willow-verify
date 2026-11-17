// The CJS build's platform.ts (scripts/build.mjs copies it over src/platform.ts). CJS runs in Node, without a worker.
import { join } from 'node:path';
import { pathToFileURL } from 'node:url';

export const packageBase = (): URL => pathToFileURL(join(__dirname, '../../') + '/');

// A real dynamic import from CJS: the wasm glue modules are ESM.
const dynamicImport = new Function('s', 'return import(s)') as (s: string) => Promise<any>;
export const importModule = (specifier: string): Promise<any> => dynamicImport(specifier);

export const spawnWorker = (): Worker => {
  throw new Error('the CommonJS build runs in Node, where the verifiers run without a worker (worker: false)');
};
