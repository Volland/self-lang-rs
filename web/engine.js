// The Self engine for the playground and the book.
//
// Self runs in a Web Worker, so a program that loops forever can be stopped
// (which also starts a fresh world). Browsers without module workers run the
// engine on the page instead, where it can't be stopped but works the same.
//
//   const self = createSelf();
//   const r = await self.eval("3 + 4");   // { ok, result, printed, trace, ms }
//
// Every call shares one world, like one Self shell.

const base = new URL('./', import.meta.url).href;

export function createSelf() {
  let worker = null;
  let direct = null;          // the engine module, when running on the page
  let nextId = 1;
  const pending = new Map();
  let generation = 0;         // goes up each time the world is thrown away
  const listeners = new Set();

  const notify = (event) => listeners.forEach((f) => f(event));

  function startWorker() {
    try {
      worker = new Worker(new URL('engine-worker.js', import.meta.url), { type: 'module' });
    } catch (e) {
      worker = null;
      return false;
    }
    worker.onmessage = (e) => {
      const { id, ok, r, error, fatal } = e.data;
      const p = pending.get(id);
      if (!p) return;
      pending.delete(id);
      if (ok) p.resolve(r);
      else if (fatal) {
        const reason = 'The program crashed the engine (' + error + '), so the world was restarted.';
        restart(reason);
        p.reject(new Error(reason));
      } else p.reject(new Error(error));
    };
    worker.onerror = (e) => {
      // A worker that can't even load (old browser, file:// page) falls back
      // to running on the page.
      e.preventDefault?.();
      const waiting = [...pending.values()];
      pending.clear();
      worker.terminate();
      worker = null;
      useDirect().then(() => waiting.forEach((p) => call(p.op, p.args).then(p.resolve, p.reject)),
        (err) => waiting.forEach((p) => p.reject(err)));
    };
    return true;
  }

  async function useDirect() {
    if (!direct) {
      direct = (async () => {
        const m = await import(base + 'selfworld.js');
        await m.default({ module_or_path: base + 'selfworld_bg.wasm' });
        return m;
      })();
    }
    return direct;
  }

  function restart(reason) {
    if (worker) worker.terminate();
    for (const p of pending.values()) p.reject(new Error(reason));
    pending.clear();
    generation++;
    startWorker();
    notify({ type: 'restart', reason, generation });
  }

  async function call(op, args = {}) {
    if (worker) {
      return new Promise((resolve, reject) => {
        const id = nextId++;
        pending.set(id, { resolve, reject, op, args });
        worker.postMessage({ id, op, base, ...args });
      });
    }
    const m = await useDirect();
    if (op === 'init') return { examples: m.example_list() };
    if (op === 'eval') return m.eval_self(args.code);
    if (op === 'load') return m.load_self(args.name, args.text);
    if (op === 'reset') { m.reset_world(); return { ok: true }; }
    throw new Error('unknown operation ' + op);
  }

  if (typeof Worker === 'undefined' || location.protocol === 'file:') {
    // no workers here; the engine loads on the page when first used
  } else {
    startWorker();
  }

  return {
    /** Evaluate Self code in the shared world. */
    eval: (code) => call('eval', { code }),
    /** Load a .self file's text, statement by statement. */
    load: (name, text) => call('load', { name, text }),
    /** The built-in example programs, as [[title, code], ...]. */
    examples: async () => (await call('init')).examples,
    /** Start again from a fresh lobby. */
    async reset() { await call('reset'); generation++; notify({ type: 'reset', generation }); },
    /** Stop whatever is running. Only possible when Self runs in a worker. */
    stop() {
      if (!worker) return false;
      restart('Stopped. The world was restarted, so objects made earlier are gone.');
      return true;
    },
    get canStop() { return !!worker; },
    get generation() { return generation; },
    on(f) { listeners.add(f); return () => listeners.delete(f); },
  };
}

/** Encode code for a #code= link (UTF-8 safe base64, URL friendly). */
export function encodeCode(code) {
  const bytes = new TextEncoder().encode(code);
  let s = '';
  for (const b of bytes) s += String.fromCharCode(b);
  return btoa(s).replace(/\+/g, '-').replace(/\//g, '_').replace(/=+$/, '');
}

/** Decode code from a #code= link; returns null when it isn't valid. */
export function decodeCode(text) {
  try {
    const s = atob(text.replace(/-/g, '+').replace(/_/g, '/'));
    return new TextDecoder().decode(Uint8Array.from(s, (c) => c.charCodeAt(0)));
  } catch (e) {
    return null;
  }
}
