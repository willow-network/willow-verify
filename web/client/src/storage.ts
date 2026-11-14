import { importModule } from './platform.js';

/** Where the light client's verified committees are kept between runs. Values are verified again when read. */
export interface KeyValueStorage {
  get(key: string): Promise<string | null> | string | null;
  set(key: string, value: string): Promise<void> | void;
}

export const LIGHT_CLIENT_KEY = 'willow-rpc:light-client';

export const isNode = (): boolean =>
  typeof process !== 'undefined' && typeof process.versions === 'object' && !!process.versions?.node && typeof (globalThis as any).window === 'undefined';

/** The browser's localStorage; it can be missing or throw (private windows, blocked storage). */
function browserStorage(): KeyValueStorage | null {
  try {
    const ls = (globalThis as any).localStorage as Storage | undefined;
    if (!ls) return null;
    return {
      get: (k) => {
        try {
          return ls.getItem(k);
        } catch {
          return null;
        }
      },
      set: (k, v) => {
        try {
          ls.setItem(k, v);
        } catch {}
      },
    };
  } catch {
    return null;
  }
}

/** Node: one file per key in `dir` (default ~/.cache/willow-rpc), `willow-rpc:light-client` → light-client.json. */
export function fileStorage(dir?: string): KeyValueStorage {
  const file = async (key: string) => {
    const path = await importModule('node:path');
    const os = await importModule('node:os');
    const base = dir ?? path.join(os.homedir(), '.cache', 'willow-rpc');
    return { base, name: path.join(base, `${key.replace(/^willow-rpc:/, '').replace(/[^a-z0-9.-]/gi, '_')}.json`) };
  };
  return {
    async get(key) {
      try {
        const fs = await importModule('node:fs/promises');
        return await fs.readFile((await file(key)).name, 'utf8');
      } catch {
        return null;
      }
    },
    async set(key, value) {
      try {
        const fs = await importModule('node:fs/promises');
        const { base, name } = await file(key);
        await fs.mkdir(base, { recursive: true });
        await fs.writeFile(`${name}.tmp`, value);
        await fs.rename(`${name}.tmp`, name);
      } catch {}
    },
  };
}

export function defaultStorage(): KeyValueStorage | null {
  return isNode() ? fileStorage() : browserStorage();
}
