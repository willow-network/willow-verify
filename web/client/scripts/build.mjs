// Builds dist/esm (ES modules, with types) and dist/cjs (CommonJS, for require in Node). The CJS
// build compiles a copy of src/ with scripts/platform.cjs.ts in place of src/platform.ts, which
// uses import.meta. The wasm modules (wasm/) come from W5's build and are not made here.
import { execFileSync } from 'node:child_process';
import { cpSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = join(dirname(fileURLToPath(import.meta.url)), '..');
const tsc = join(root, 'node_modules', 'typescript', 'bin', 'tsc');
rmSync(join(root, 'dist'), { recursive: true, force: true });

execFileSync(process.execPath, [tsc, '-p', join(root, 'tsconfig.json')], { stdio: 'inherit' });
writeFileSync(join(root, 'dist/esm/package.json'), '{"type":"module"}\n');

const tmp = mkdtempSync(join(tmpdir(), 'willow-rpc-cjs-'));
try {
  cpSync(join(root, 'src'), join(tmp, 'src'), { recursive: true });
  cpSync(join(root, 'scripts/platform.cjs.ts'), join(tmp, 'src/platform.ts'));
  writeFileSync(
    join(tmp, 'tsconfig.json'),
    JSON.stringify({
      compilerOptions: {
        target: 'ES2022',
        module: 'CommonJS',
        moduleResolution: 'Node10',
        lib: ['ES2022', 'DOM', 'WebWorker'],
        strict: true,
        esModuleInterop: true,
        skipLibCheck: true,
        types: ['node'],
        typeRoots: [join(root, 'node_modules/@types')],
        baseUrl: '.',
        paths: { '*': [join(root, 'node_modules/*')] },
        declaration: false,
        sourceMap: true,
        rootDir: 'src',
        outDir: join(root, 'dist/cjs'),
      },
      include: ['src'],
    }),
  );
  execFileSync(process.execPath, [tsc, '-p', join(tmp, 'tsconfig.json')], { stdio: 'inherit' });
} finally {
  rmSync(tmp, { recursive: true, force: true });
}
writeFileSync(join(root, 'dist/cjs/package.json'), '{"type":"commonjs"}\n');
console.log('built dist/esm and dist/cjs');
