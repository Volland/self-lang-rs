// Builds the web version of Self World into dist/:
// compiles the Rust crate to WebAssembly, generates the JavaScript glue with
// wasm-bindgen and copies the page. Run with `npm run build`.
import { execFileSync } from 'node:child_process';
import { cpSync, mkdirSync, rmSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = join(dirname(fileURLToPath(import.meta.url)), '..');
const run = (cmd, args) => execFileSync(cmd, args, { cwd: root, stdio: 'inherit' });

run('cargo', ['build', '--release', '--target', 'wasm32-unknown-unknown', '--lib']);

const dist = join(root, 'dist');
rmSync(dist, { recursive: true, force: true });
mkdirSync(dist, { recursive: true });

const wasm = join(root, 'target', 'wasm32-unknown-unknown', 'release', 'selfworld.wasm');
try {
  run('wasm-bindgen', ['--target', 'web', '--no-typescript', '--out-dir', dist, wasm]);
} catch (e) {
  console.error('\nwasm-bindgen is needed: cargo install wasm-bindgen-cli --version 0.2.128\n');
  process.exit(1);
}
// the homepage and the shared engine loader sit next to the program files
cpSync(join(root, 'web', 'index.html'), join(dist, 'index.html'));
cpSync(join(root, 'web', 'engine.js'), join(dist, 'engine.js'));
cpSync(join(root, 'web', 'engine-worker.js'), join(dist, 'engine-worker.js'));
// each other page gets its own folder, so it has a clean address (try/, play/, book/)
for (const [page, folder] of [['try.html', 'try'], ['play.html', 'play'], ['book.html', 'book']]) {
  mkdirSync(join(dist, folder), { recursive: true });
  cpSync(join(root, 'web', page), join(dist, folder, 'index.html'));
}
console.log('Built Self World for the web in dist/');
