// The ESM build's view of where it runs. scripts/build.mjs swaps in platform.cjs.ts for the CJS build.

/** The package's root directory (dist/esm/platform.js → ../../). */
export const packageBase = (): URL => new URL('../../', import.meta.url);

/** `import()` that bundlers leave alone (the wasm glue is found at run time). */
export const importModule = (specifier: string): Promise<any> => import(/* @vite-ignore */ /* webpackIgnore: true */ specifier);

/** The browser worker that runs the wasm verifiers, as web/verify/worker.js does for the page. */
export const spawnWorker = (): Worker => new Worker(new URL('./worker.js', import.meta.url), { type: 'module', name: 'willow-rpc' });
