// Runs the Self engine (Rust compiled to WebAssembly) off the page's main
// thread, so a program that never stops can be stopped by ending this worker.
// Messages in:  { id, op: 'init' | 'eval' | 'load' | 'reset', base, code, name, text }
// Messages out: { id, ok, r } or { id, ok: false, error, fatal }
let mod = null;

async function engine(base) {
  if (!mod) {
    const m = await import(base + 'selfworld.js');
    await m.default({ module_or_path: base + 'selfworld_bg.wasm' });
    mod = m;
  }
  return mod;
}

self.onmessage = async (e) => {
  const { id, op, base, code, name, text } = e.data;
  try {
    const m = await engine(base);
    let r;
    if (op === 'init') r = { examples: m.example_list() };
    else if (op === 'eval') r = m.eval_self(code);
    else if (op === 'load') r = m.load_self(name, text);
    else if (op === 'reset') { m.reset_world(); r = { ok: true }; }
    else throw new Error('unknown operation ' + op);
    self.postMessage({ id, ok: true, r });
  } catch (err) {
    // A WebAssembly trap (for example running out of stack) can leave the
    // engine's memory in a bad state, so the page replaces this worker.
    const fatal = err instanceof WebAssembly.RuntimeError || err instanceof RangeError;
    self.postMessage({ id, ok: false, error: String(err && err.message || err), fatal });
  }
};
