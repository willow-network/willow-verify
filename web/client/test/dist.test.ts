// The built package (npm run build): its CommonJS and ES module entry points load and verify, the
// CJS one importing the wasm glue (ES modules) at run time. Skipped without a build.
import { existsSync } from 'node:fs';
import { createRequire } from 'node:module';
import { join } from 'node:path';
import { pathToFileURL } from 'node:url';
import { afterEach, describe, expect, it } from 'vitest';
import { mockModules, VERIFY_DIST } from './support/mock-verifiers.js';
import { serve, type Running } from './support/mock-server.js';
import { blockOf, fullWorld, headAt } from './support/setup.js';
import { fixture, World } from './support/world.js';

const ROOT = join(__dirname, '..');
const built = existsSync(join(ROOT, 'dist/cjs/index.js')) && existsSync(join(ROOT, 'dist/esm/index.js'));
const require = createRequire(import.meta.url);

let server: Running | null = null;
afterEach(async () => {
  await server?.close();
  server = null;
});

async function baseBlock(createVerifier: (o: any) => any) {
  const w = fullWorld();
  const head = blockOf('base-56049971.json');
  headAt(w, 8453, head);
  server = await serve(w);
  const v = createVerifier({ chainId: 8453, url: server.url(8453), verifiers: mockModules(w), storage: false, checkpoint: { root: `0x${'11'.repeat(32)}`, slot: 1 }, now: () => w.nowSec * 1000 });
  const b = await v.request({ method: 'eth_getBlockByNumber', params: ['latest', false] });
  v.close();
  return { b, head };
}

describe.skipIf(!built)('the built package', () => {
  it('CommonJS: require loads the core, viem and ethers entry points, and verifies', async () => {
    const core = require(join(ROOT, 'dist/cjs/index.js'));
    expect(typeof require(join(ROOT, 'dist/cjs/viem.js')).willowTransport).toBe('function');
    expect(typeof require(join(ROOT, 'dist/cjs/ethers.js')).WillowProvider).toBe('function');
    const { b, head } = await baseBlock(core.createVerifier);
    expect((b as any).hash).toBe(head.hash);
  });

  it('ES modules: import loads the entry points, and verifies', async () => {
    const core = await import(pathToFileURL(join(ROOT, 'dist/esm/index.js')).href);
    expect(typeof (await import(pathToFileURL(join(ROOT, 'dist/esm/viem.js')).href)).willowTransport).toBe('function');
    expect(typeof (await import(pathToFileURL(join(ROOT, 'dist/esm/ethers.js')).href)).WillowProvider).toBe('function');
    const { b, head } = await baseBlock(core.createVerifier);
    expect((b as any).hash).toBe(head.hash);
  });

  it.skipIf(!VERIFY_DIST)("CommonJS: distModules imports the real anchor module's ES glue and anchors the head", async () => {
    const core = require(join(ROOT, 'dist/cjs/index.js'));
    const w = new World();
    const tag = (n: string) => fixture(`anchor-tags/${n}.json`);
    w.nowSec = tag('latest-1').savedAt;
    w.lightClient = { bootstrap: tag('bootstrap'), updates: tag('updates') };
    w.realAnchors.set('1:latest', tag('latest-1').response.result);
    server = await serve(w);
    const v = core.createVerifier({ chainId: 1, url: server.url(1), verifiers: core.distModules(VERIFY_DIST), storage: false, checkpoint: tag('checkpoint'), now: () => w.nowSec * 1000 });
    expect(await v.request({ method: 'eth_blockNumber' })).toBe(tag('latest-1').response.result.blockNumber);
    v.close();
  });
});
