// The browser worker's entry (platform.ts spawnWorker): the wasm verifiers off the main thread.
import { InlineBackend } from './backend.js';
import { packageModules } from './wasm.js';
import { serveBackend } from './worker-rpc.js';

serveBackend(globalThis as any, (checkpoint) => new InlineBackend(packageModules(), checkpoint));
