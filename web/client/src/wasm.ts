import { importModule, packageBase } from './platform.js';
import { isNode } from './storage.js';
import type { GlueModule, VerifierModules } from './verifiers.js';

/** Glue and wasm of one module: URLs (browser) or file URLs (Node). */
export interface ModuleFiles {
  glue: URL;
  wasm: URL;
}

/** Imports a wasm-bindgen `--target web` glue module and initialises it with its wasm. */
export async function loadGlue<T extends GlueModule>(files: ModuleFiles): Promise<T> {
  const glue = (await importModule(files.glue.href)) as T;
  if (isNode()) {
    const fs = await importModule('node:fs/promises');
    glue.initSync({ module: await fs.readFile(files.wasm) });
  } else {
    await glue.default({ module_or_path: files.wasm });
  }
  return glue;
}

const NAMES = { items: 'items', state: 'state', anchor: 'anchor', logs: 'verifier', openvm: 'openvm' } as const;

/** Modules from `files(name)`: `items`, `state`, `anchor`, `verifier` (logs), `openvm`. */
export function modulesFrom(files: (name: string) => ModuleFiles | Promise<ModuleFiles>): VerifierModules {
  const load = (name: string) => async () => loadGlue<any>(await files(name));
  return {
    items: load(NAMES.items),
    state: load(NAMES.state),
    anchor: load(NAMES.anchor),
    logs: load(NAMES.logs),
    openvm: load(NAMES.openvm),
  };
}

/** The package's own builds: `wasm/<name>.js` and `wasm/<name>_bg.wasm` at its root (W5 builds them, W8 ships them). */
export function packageModules(base: URL = packageBase()): VerifierModules {
  return modulesFrom((name) => ({ glue: new URL(`wasm/${name}.js`, base), wasm: new URL(`wasm/${name}_bg.wasm`, base) }));
}

/**
 * Node: the modules of a verify page build (web/verify-dist, as build.sh writes it, with content
 * hashes in the names), for tests and for running against a page build.
 */
export function distModules(dir: string): VerifierModules {
  return modulesFrom(async (name) => {
    const fs = await importModule('node:fs');
    const path = await importModule('node:path');
    const url = await importModule('node:url');
    const files: string[] = fs.readdirSync(dir);
    const pick = (ext: string) => {
      const f = files.find((x) => new RegExp(`^${name}\\.[0-9a-f]+\\.${ext}$`).test(x));
      if (!f) throw new Error(`no ${name} module in ${dir}`);
      return url.pathToFileURL(path.join(dir, f)) as URL;
    };
    return { glue: pick('js'), wasm: pick('wasm') };
  });
}
