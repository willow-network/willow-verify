// Publishes @willow-network/rpc only from the tree web/verify/publish.sh exports. npm runs it before
// `npm publish` and `npm pack` (package.json's prepublishOnly and prepack); it exits non-zero, and npm
// stops, unless all of these hold:
//
// (a) This package is in publish.sh's export: the export's EXPORT.sha256 lists every file it holds with
//     its sha256, each file listed is unchanged, package.json is not private, and the tree has no
//     web/verify/publish.sh or web/verify/public-repo (Willow's repository always has them; the export
//     never does).
// (b) dist/ is built again here (scripts/build.mjs) from the exported src/, and every file npm would pack
//     (`npm pack --dry-run --json`) is on the allowlist: package.json, README.md, LICENSE,
//     THIRD-PARTY-NOTICES.md and THIRD-PARTY-LICENSES.md as exported, dist/ for the exported src/, and wasm/
//     modules whose crate the export holds.
// (c) No packed file contains what publish.sh's DENY pattern matches. The pattern is read from
//     $WILLOW_REPO/web/verify/publish.sh (a checkout of Willow's repository), as release.py reads it:
//     publish.sh is not exported.
// (d) The license: package.json says "MIT", LICENSE is the export's MIT license, THIRD-PARTY-NOTICES.md is
//     packed as the export's root has it, and the export has no LICENSES/.
// (e) Every packed source map names its sources inside the package (relative): never a path of the machine
//     that built it.
//
// In Willow's repository package.json is private, so npm refuses to publish the repository's copy in any
// form, a tarball included. `npm pack` there (for local tests) passes while it is private.
import { createHash } from 'node:crypto';
import { spawnSync } from 'node:child_process';
import { existsSync, readFileSync } from 'node:fs';
import { dirname, join, posix } from 'node:path';
import { fileURLToPath } from 'node:url';

const PKG = join(dirname(fileURLToPath(import.meta.url)), '..');
const ROOT = join(PKG, '..', '..');
const AT = 'web/client/';
const MARKER = 'EXPORT.sha256';

/** The notices of the MIT-licensed projects parts of the verifiers follow: packed, as the export's root has it. */
const NOTICES = 'THIRD-PARTY-NOTICES.md';
/** The licenses of the third-party code compiled into wasm/, generated at each release: packed. */
const WASM_LICENSES = 'THIRD-PARTY-LICENSES.md';

/** The wasm modules: `wasm/<name>.js` and `wasm/<name>_bg.wasm`, each from its crate (web/verify/build.sh). */
const WASM = {
  verifier: 'zkvm/logset-guest/query-wasm',
  openvm: 'zkvm/logset-guest-openvm/verify-wasm',
  items: 'zkvm/logset-guest/items-wasm',
  state: 'zkvm/logset-guest/state-wasm',
  anchor: 'zkvm/logset-guest/anchor-wasm',
  stellar: 'crates/stellar-verify-wasm',
};

const HOW = `Publish @willow-network/rpc only from the tree web/verify/publish.sh exports, never from Willow's repository:
  1. On a release branch whose web/verify/public-repo (FILES, PRIVATE, STRIP) is current for the commit
     to release: web/verify/publish.sh <a git checkout: willow-verify, or an empty one>.
  2. In <checkout>/web/client: npm ci, and the verify page's wasm modules built from that export copied
     into wasm/ as <name>.js and <name>_bg.wasm (this check builds dist/ itself).
  3. There: WILLOW_REPO=<checkout of Willow's repository> npm publish (or npm pack, then npm publish
     <the tarball>). npm runs this check for both; never with --ignore-scripts.`;

const norm = (s) => s.replace(/\s+/g, ' ').trim();
const sha256 = (path) => createHash('sha256').update(readFileSync(path)).digest('hex');

function fail(problems, how = true) {
  console.error(`prepublish-guard: refused.\n${problems.map((p) => `  - ${p}`).join('\n')}${how ? `\n\n${HOW}` : ''}`);
  process.exit(1);
}

/** (a): the export's files by path (from the root), or the reasons this is not the export. */
function exported(pkg) {
  const problems = [];
  for (const p of ['web/verify/publish.sh', 'web/verify/public-repo']) {
    if (existsSync(join(ROOT, p))) problems.push(`${p} is here: this is Willow's repository, not publish.sh's export`);
  }
  if (pkg.private !== undefined) problems.push('package.json is private: the export drops "private"');
  const marker = join(ROOT, MARKER);
  if (!existsSync(marker)) {
    problems.push(`no ${MARKER} at ${ROOT}: publish.sh writes it into the export`);
    return { problems, files: new Map() };
  }
  const files = new Map();
  for (const line of readFileSync(marker, 'utf8').split('\n')) {
    if (!line) continue;
    const m = /^([0-9a-f]{64}) {2}([^/].*)$/.exec(line);
    if (!m || m[2].split('/').includes('..')) {
      problems.push(`${MARKER}: not a sha256 line: ${JSON.stringify(line.slice(0, 120))}`);
      continue;
    }
    files.set(m[2], m[1]);
  }
  for (const [path, want] of files) {
    const f = join(ROOT, path);
    if (!existsSync(f)) problems.push(`${path} is missing from the export`);
    else if (sha256(f) !== want) problems.push(`${path} is not the exported file (sha256 differs from ${MARKER})`);
  }
  for (const p of ['package.json', 'README.md', 'LICENSE', NOTICES, WASM_LICENSES, 'scripts/prepublish-guard.mjs']) {
    if (!files.has(AT + p)) problems.push(`${MARKER} does not list ${AT}${p}`);
  }
  return { problems, files };
}

/** (b): dist/ from the exported src/, whatever was there before. */
function build() {
  if (!existsSync(join(PKG, 'node_modules/typescript'))) fail(['no node_modules/typescript: run npm ci first']);
  const r = spawnSync(process.execPath, [join(PKG, 'scripts/build.mjs')], { cwd: PKG, encoding: 'utf8' });
  if (r.status !== 0) fail([`npm run build failed (${r.status}): ${`${r.stdout}${r.stderr}`.trim().slice(0, 600)}`], false);
}

/** (b): the files npm would pack, from npm itself. */
function packed() {
  const r = spawnSync('npm', ['pack', '--dry-run', '--json', '--ignore-scripts'], { cwd: PKG, encoding: 'utf8', maxBuffer: 64 << 20 });
  if (r.status !== 0) fail([`npm pack --dry-run failed (${r.status}): ${(r.stderr || r.error?.message || '').trim().slice(0, 400)}`], false);
  try {
    return JSON.parse(r.stdout)[0].files.map((f) => f.path);
  } catch (e) {
    fail([`npm pack --dry-run --json printed no file list: ${String(e).slice(0, 200)}`], false);
  }
}

function allowlist(files) {
  const allowed = new Set(['package.json', 'README.md', 'LICENSE', NOTICES, WASM_LICENSES, 'dist/esm/package.json', 'dist/cjs/package.json']);
  for (const path of files.keys()) {
    if (!path.startsWith(AT)) continue;
    const p = path.slice(AT.length);
    const src = /^src\/(.+)\.ts$/.exec(p);
    if (src && !p.endsWith('.d.ts')) {
      for (const ext of ['.js', '.js.map', '.d.ts']) allowed.add(`dist/esm/${src[1]}${ext}`);
      for (const ext of ['.js', '.js.map']) allowed.add(`dist/cjs/${src[1]}${ext}`);
    }
  }
  for (const [name, crate] of Object.entries(WASM)) {
    if (files.has(`${crate}/Cargo.toml`)) {
      allowed.add(`wasm/${name}.js`);
      allowed.add(`wasm/${name}_bg.wasm`);
    }
  }
  return allowed;
}

/** (c): publish.sh's DENY, from Willow's repository. */
function deny() {
  const repo = process.env.WILLOW_REPO;
  if (!repo) fail(['WILLOW_REPO is not set: name a checkout of Willow\'s repository, whose web/verify/publish.sh holds DENY']);
  const f = join(repo, 'web/verify/publish.sh');
  if (!existsSync(f)) fail([`${f} does not exist: WILLOW_REPO must name a checkout of Willow's repository`]);
  const m = /^DENY=((?:'[^']*'\\?\n?)+)/m.exec(readFileSync(f, 'utf8'));
  const src = m ? [...m[1].matchAll(/'([^']*)'/g)].map((x) => x[1]).join('') : '';
  if (!src) fail([`${f} has no DENY pattern`]);
  return new RegExp(src, 'i');
}

function denyHits(paths, rx) {
  const hits = [];
  for (const p of paths) {
    const lines = readFileSync(join(PKG, p)).toString('latin1').split('\n');
    lines.forEach((line, i) => {
      const m = rx.exec(line);
      if (m) hits.push(`${p}:${i + 1}: ${line.slice(Math.max(0, m.index - 40), m.index + 60).replace(/[^\x20-\x7e]/g, '.')}`);
    });
  }
  return hits;
}

/** (d) */
function license(pkg, files, paths) {
  const problems = [];
  const lic = norm(readFileSync(join(PKG, 'LICENSE'), 'utf8'));
  const mit = norm(readFileSync(join(ROOT, 'LICENSE'), 'utf8'));
  const texts = [...files.keys()].filter((p) => p.startsWith('LICENSES/')).sort();
  if (!paths.includes('LICENSE')) problems.push('LICENSE is not packed');
  if (pkg.license !== 'MIT') problems.push(`package.json "license" is ${JSON.stringify(pkg.license)}, not "MIT"`);
  if (lic !== mit) problems.push("LICENSE is not the export's MIT license");
  if (texts.length) problems.push(`the export has LICENSES/ (${texts.join(', ')}): its license is MIT alone`);
  if (!paths.includes(NOTICES)) problems.push(`${NOTICES} is not packed`);
  else if (!files.has(NOTICES) || files.get(AT + NOTICES) !== files.get(NOTICES)) problems.push(`${NOTICES} is not the export's ${NOTICES}`);
  if (!paths.includes(WASM_LICENSES)) problems.push(`${WASM_LICENSES} is not packed (the wasm/ modules' third-party licenses)`);
  return problems;
}

/** (e) */
function mapSources(paths) {
  const problems = [];
  const outside = (s) => /^[a-z][a-z0-9+.-]*:/i.test(s) || posix.isAbsolute(s);
  for (const p of paths.filter((x) => x.endsWith('.map'))) {
    let m;
    try {
      m = JSON.parse(readFileSync(join(PKG, p), 'utf8'));
    } catch {
      problems.push(`${p} is not a source map`);
      continue;
    }
    const root = typeof m.sourceRoot === 'string' ? m.sourceRoot : '';
    for (const s of Array.isArray(m.sources) ? m.sources.map(String) : []) {
      if (outside(root) || outside(s) || posix.normalize(posix.join(posix.dirname(p), root, s)).startsWith('..')) {
        problems.push(`${p} names a source outside the package: ${posix.join(root, s)}`);
        break;
      }
    }
  }
  return problems;
}

const hook = process.env.npm_lifecycle_event ?? '';
const pkg = JSON.parse(readFileSync(join(PKG, 'package.json'), 'utf8'));
const { problems, files } = exported(pkg);
if (problems.length) {
  if (hook === 'prepack' && pkg.private === true) {
    console.error("prepublish-guard: packing Willow's repository for local use; npm will not publish this package (package.json is private).");
    process.exit(0);
  }
  fail(problems);
}
const rx = deny();
build();
const paths = packed();
const allowed = allowlist(files);
const out = paths.filter((p) => !allowed.has(p)).map((p) => `${p} is not on the allowlist`);
out.push(...denyHits(paths, rx).map((h) => `DENY: ${h}`));
out.push(...license(pkg, files, paths));
out.push(...mapSources(paths));
if (out.length) fail(out);
console.error(`prepublish-guard: ${paths.length} files from publish.sh's export, dist/ built from it: allowlist, DENY, license and source maps checked.`);
