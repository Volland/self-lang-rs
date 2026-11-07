// Self World's JavaScript runtime: the Self object model for programs that
// Self World has translated to JavaScript (see src/jsgen.rs).
//
// Values: integers are BigInts (so they grow, as in Self), floats are JS
// numbers, strings are JS strings, everything else is an SObj with slots.
// A message send looks the selector up in the receiver and its parents,
// exactly like the Rust interpreter, and the starting world itself is Self
// code translated by the same compiler.
"use strict";
var SelfRT = (function () {
  let nextId = 0;
  class Slot {
    constructor(name, value, assignable, parent, arg) {
      this.name = name; this.value = value; this.assignable = assignable; this.parent = parent; this.arg = arg;
    }
  }
  class SObj {
    constructor(kind) {
      this.slots = new Map();
      this.kind = kind || "plain"; // plain, vector, block, method
      this.creator = null;
      this.comment = null;
      this.id = ++nextId;
    }
  }
  class SelfError extends Error {
    constructor(msg) { super(msg); this.msg = msg; }
  }
  class NLR { constructor(home, value) { this.home = home; this.value = value; } }
  class Fail { constructor(code) { this.code = code; } }

  const $ = {};
  $.SObj = SObj; $.SelfError = SelfError; $.NLR = NLR;

  // ------------------------------------------------------------ the objects every world starts with
  const mk = () => new SObj();
  const nil = mk(), lobby = mk(), globals = mk(), traits = mk(), T = mk(), F = mk(), db = mk();
  const tInt = mk(), tFloat = mk(), tStr = mk(), tBlock = mk(), tMethod = mk(), shell = mk();
  $.nil = nil; $.T = T; $.F = F; $.lobby = lobby; $.globals = globals;
  const put = (o, name, v, parent) => o.slots.set(name, new Slot(name, v, false, !!parent, false));
  put(lobby, "globals", globals, true); put(lobby, "defaultBehavior", db, true); put(lobby, "lobby", lobby);
  put(globals, "nil", nil); put(globals, "true", T); put(globals, "false", F); put(globals, "traits", traits); put(globals, "shell", shell);
  put(traits, "integer", tInt); put(traits, "float", tFloat); put(traits, "string", tStr); put(traits, "block", tBlock); put(traits, "method", tMethod);
  put(shell, "lobby", lobby, true);
  for (const [o, h, n] of [[globals, lobby, "globals"], [nil, globals, "nil"], [T, globals, "true"], [F, globals, "false"],
    [traits, globals, "traits"], [db, lobby, "defaultBehavior"], [shell, globals, "shell"], [tInt, traits, "integer"],
    [tFloat, traits, "float"], [tStr, traits, "string"], [tBlock, traits, "block"], [tMethod, traits, "method"]]) o.creator = [h, n];

  // ------------------------------------------------------------ making objects
  // slots: [name, flags, value] with flags 1 = assignable, 2 = parent, 4 = argument
  $.obj = function (slots, comment) {
    const o = new SObj();
    for (const [n, f, v] of slots) o.slots.set(n, new Slot(n, v, !!(f & 1), !!(f & 2), !!(f & 4)));
    for (const s of o.slots.values()) setCreator(s.value, o, s.name);
    if (comment) o.comment = comment;
    return o;
  };
  $.method = function (nargs, fn, source) {
    const m = new SObj("method"); m.nargs = nargs; m.fn = fn; m.source = source || ""; return m;
  };
  $.block = function (nargs, fn) {
    const b = new SObj("block"); b.nargs = nargs; b.fn = fn; return b;
  };
  function setCreator(v, holder, name) {
    if (v instanceof SObj && !v.creator && (v.kind === "plain" || v.kind === "vector") && v !== holder) v.creator = [holder, name];
  }

  // ------------------------------------------------------------ lookup and sending
  let version = 1;                 // bumped whenever slots are added, removed or a parent changes
  let sends = 0;
  const SEND_LIMIT = 60000000;
  $.resetSends = () => { sends = 0; };

  function implicitParent(v) {
    switch (typeof v) {
      case "bigint": return tInt;
      case "number": return tFloat;
      case "string": return tStr;
    }
    if (v.kind === "block") return tBlock;
    if (v.kind === "method") return tMethod;
    return null;
  }
  function lookupValue(v, sel, nargs, path, out) {
    if (v instanceof SObj) lookupIn(v, sel, nargs, path, out);
    else { const p = implicitParent(v); if (p) lookupIn(p, sel, nargs, path, out); }
  }
  function lookupIn(o, sel, nargs, path, out) {
    if (path.includes(o)) return;
    const s = o.slots.get(sel);
    if (s) { out.push({ holder: o, slot: s, assign: false }); return; }
    if (nargs === 1 && sel.endsWith(":")) {
      const t = o.slots.get(sel.slice(0, -1));
      if (t && t.assignable) { out.push({ holder: o, slot: t, assign: true }); return; }
    }
    path.push(o);
    for (const p of o.slots.values()) if (p.parent) lookupValue(p.value, sel, nargs, path, out);
    const ip = implicitParent(o);
    if (ip) lookupIn(ip, sel, nargs, path, out);
    path.pop();
  }
  function dedupe(found) {
    const out = [];
    for (const f of found) if (!out.some((g) => g.slot === f.slot && g.assign === f.assign)) out.push(f);
    return out;
  }
  function find(start, r, sel, nargs) {
    let c = start._cache;
    if (!c || c.v !== version) c = start._cache = { v: version, m: new Map() };
    const key = nargs === 1 ? sel + "\u0001" : sel;
    let f = c.m.get(key);
    if (f !== undefined) return f;
    let found = [];
    lookupValue(r, sel, nargs, [], found);
    found = dedupe(found);
    f = found.length === 1 ? found[0] : found.length === 0 ? null : "ambiguous";
    c.m.set(key, f);
    return f;
  }
  const isValueSel = (sel, n) =>
    (n === 0 && sel === "value") || (n === 1 && sel === "value:") || (n === 2 && sel === "value:With:") ||
    (n === 3 && sel === "value:With:With:") || (n === 4 && sel === "value:With:With:With:");

  function send(r, sel, args) {
    if (++sends > SEND_LIMIT) { sends = 0; throw new SelfError("stopped after " + SEND_LIMIT / 1000000 + " million message sends — is something looping forever?"); }
    if (sel.charCodeAt(0) === 95) return prim(r, sel, args);
    if (r instanceof SObj && r.kind === "block" && isValueSel(sel, args.length)) return callBlock(r, args);
    const start = r instanceof SObj ? r : implicitParent(r);
    const f = find(start, r, sel, args.length);
    if (f === null) throw new SelfError(describe(r) + " does not understand '" + sel + "'");
    if (f === "ambiguous") throw new SelfError("'" + sel + "' sent to " + describe(r) + " is ambiguous (found in several parents)");
    return evalSlot(r, f, sel, args);
  }
  function evalSlot(r, f, sel, args) {
    if (f.assign) {
      f.slot.value = args[0];
      if (f.slot.parent) version++;
      setCreator(args[0], f.holder, f.slot.name);
      return r;
    }
    const v = f.slot.value;
    if (v instanceof SObj && v.kind === "method") {
      if (v.nargs !== args.length) throw new SelfError("'" + sel + "' expects " + v.nargs + " argument(s) but got " + args.length);
      return v.fn(r, f.holder, args);
    }
    return v;
  }
  $.send = send;

  $.resend = function (self, holder, target, sel, args) {
    if (!holder) throw new SelfError("resend used outside of a method");
    let found = [];
    if (target === "resend") {
      const path = [holder];
      for (const s of holder.slots.values()) if (s.parent) lookupValue(s.value, sel, args.length, path, found);
      const ip = implicitParent(holder);
      if (ip) lookupIn(ip, sel, args.length, path, found);
    } else {
      const d = holder.slots.get(target);
      if (!d) throw new SelfError("directed resend: no parent slot named '" + target + "'");
      lookupValue(d.value, sel, args.length, [], found);
    }
    found = dedupe(found);
    if (found.length === 0) throw new SelfError("resend of '" + sel + "' found no inherited slot");
    if (found.length > 1) throw new SelfError("resend of '" + sel + "' is ambiguous");
    return evalSlot(self, found[0], sel, args);
  };

  function callBlock(b, args) {
    if (b.nargs > args.length) throw new SelfError("this block takes " + b.nargs + " argument(s) but was given " + args.length);
    return b.fn(args.length > b.nargs ? args.slice(0, b.nargs) : args);
  }
  function callBlockLenient(v, args) {
    if (v instanceof SObj && v.kind === "block") return callBlock(v, args.slice(0, v.nargs));
    return send(v, "value", []);
  }
  $.callBlock = callBlock;

  // a non-local return (^ inside a block) back to its method
  $.nlr = function (home, value) {
    if (!home.alive) throw new SelfError("non-local return from a block whose method has already returned");
    throw new NLR(home, value);
  };
  $.restart = function () { throw new SelfError("_Restart is not supported in translated code"); };

  // ------------------------------------------------------------ describing objects
  function quote(s) {
    let o = "'";
    for (const c of s) o += c === "'" ? "\\'" : c === "\\" ? "\\\\" : c === "\n" ? "\\n" : c === "\t" ? "\\t" : c;
    return o + "'";
  }
  function fmtFloat(f) {
    if (Number.isFinite(f) && f === Math.trunc(f) && Math.abs(f) < 1e15) return f.toFixed(1);
    if (Number.isFinite(f) && (Math.abs(f) >= 1e15 || (f !== 0 && Math.abs(f) < 1e-6))) return f.toExponential().replace("e+", "e");
    if (f === Infinity) return "inf";
    if (f === -Infinity) return "-inf";
    return String(f);
  }
  function pathName(o) {
    const parts = [];
    let cur = o;
    for (let i = 0; i < 8; i++) {
      if (cur === lobby || cur === globals) break;
      if (!cur.creator) return null;
      const [h, n] = cur.creator;
      const s = h.slots.get(n);
      if (!s || s.value !== cur) return null;
      parts.push(n);
      cur = h;
    }
    if (cur !== lobby && cur !== globals) return null;
    if (parts.length === 0) return o === globals ? "globals" : "lobby";
    return parts.reverse().join(" ");
  }
  function describe(v) {
    switch (typeof v) {
      case "bigint": return v.toString();
      case "number": return fmtFloat(v);
      case "string": return quote(v);
    }
    if (!(v instanceof SObj)) return String(v);
    if (v.kind === "block") return "a block";
    if (v.kind === "method") return "a method";
    const p = pathName(v);
    if (p) return p;
    for (const s of v.slots.values()) {
      if (s.parent && s.value instanceof SObj) {
        const pn = pathName(s.value);
        if (pn) {
          const base = pn.startsWith("traits ") ? pn.slice(7) : pn;
          if (base === "lobby" || base === "clonable" || base === "oddball") continue;
          return (/^[aeiou]/.test(base) ? "an " : "a ") + base;
        }
      }
    }
    return v.kind === "vector" ? "a vector" : "an object";
  }
  $.describe = describe; $.quote = quote;
  $.printString = function (v) {
    try { const s = send(v, "printString", []); return typeof s === "string" ? s : describe(v); }
    catch (e) { return describe(v); }
  };

  // ------------------------------------------------------------ output
  $.out = [];
  $.onPrint = null;
  function print(s) { $.out.push(s); if ($.onPrint) $.onPrint(s); }
  $.onClearTranscript = null;

  // ------------------------------------------------------------ files, kept in localStorage in the browser
  const store = (function () {
    try { if (typeof window !== "undefined" && window.localStorage) { window.localStorage.getItem("x"); return window.localStorage; } } catch (e) {}
    const m = new Map();
    return { getItem: (k) => (m.has(k) ? m.get(k) : null), setItem: (k, v) => m.set(k, String(v)) };
  })();
  const fileKey = (n) => "selfworld-file:" + n;

  // ------------------------------------------------------------ primitives
  const startMs = typeof performance !== "undefined" ? performance.now() : Date.now();
  const nowMs = () => (typeof performance !== "undefined" ? performance.now() : Date.now()) - startMs;
  const chars = (s) => Array.from(s);
  function int(v) { if (typeof v === "bigint") return v; throw new Fail("badTypeError"); }
  function num(v) { if (typeof v === "bigint") return Number(v); if (typeof v === "number") return v; throw new Fail("badTypeError"); }
  function str(v) { if (typeof v === "string") return v; throw new Fail("badTypeError"); }
  function ref(v) { if (v instanceof SObj) return v; throw new Fail("badTypeError"); }
  const bool = (b) => (b ? T : F);
  function same(a, b) { return typeof a === "number" && typeof b === "number" ? Object.is(a, b) : a === b; }
  function isTrue(v) {
    if (v === T) return true;
    if (v === F) return false;
    throw new SelfError("expected true or false, got " + describe(v));
  }
  function divE(x, y) { let q = x / y; if (x % y < 0n) q = y > 0n ? q - 1n : q + 1n; return q; }
  function modE(x, y) { let r = x % y; if (r < 0n) r += y < 0n ? -y : y; return r; }
  function newVector(items) {
    const proto = globals.slots.get("vector");
    const v = new SObj("vector");
    if (proto && proto.value instanceof SObj) for (const s of proto.value.slots.values()) v.slots.set(s.name, new Slot(s.name, s.value, s.assignable, s.parent, s.arg));
    v.items = items;
    return v;
  }
  $.newVector = newVector;
  function cloneObj(o) {
    const c = new SObj(o.kind);
    for (const s of o.slots.values()) c.slots.set(s.name, new Slot(s.name, s.value, s.assignable, s.parent, s.arg));
    c.comment = o.comment;
    if (o.kind === "vector") c.items = o.items.slice();
    if (o.kind === "block" || o.kind === "method") { c.fn = o.fn; c.nargs = o.nargs; c.source = o.source; }
    return c;
  }
  function addSlots(t, from, replace) {
    version++;
    for (const s of from.slots.values()) {
      if (s.value instanceof SObj && s.value.creator && s.value.creator[0] === from && s.value.creator[1] === s.name) s.value.creator = [t, s.name];
      const ex = t.slots.get(s.name);
      if (ex) { if (replace) t.slots.set(s.name, new Slot(s.name, s.value, s.assignable, s.parent, s.arg)); }
      else t.slots.set(s.name, new Slot(s.name, s.value, s.assignable, s.parent, s.arg));
    }
    if (!t.comment) t.comment = from.comment;
  }
  function kindLabel(v) {
    switch (typeof v) { case "bigint": return "integer"; case "number": return "float"; case "string": return "string"; }
    return { plain: "object", vector: "vector", block: "block", method: "method" }[v.kind] || "object";
  }
  function strHash(s) {
    let h = 1469598103;
    for (const b of new TextEncoder().encode(s)) h = Math.imul(h ^ b, 16777619) & 0x3fffffff;
    return BigInt(h);
  }
  function floatBitsHash(f) { const b = new DataView(new ArrayBuffer(8)); b.setFloat64(0, f); return b.getBigInt64(0); }

  const P = {
    "_Eq:": (r, a) => bool(same(r, a[0])),
    "_IdentityHash": (r) => (typeof r === "bigint" ? r : typeof r === "number" ? floatBitsHash(r) : typeof r === "string" ? strHash(r) : BigInt(r.id)),
    "_Clone": (r) => (r instanceof SObj ? cloneObj(r) : r),
    "_AddSlots:": (r, a) => { addSlots(ref(r), ref(a[0]), true); return r; },
    "_AddSlotsIfAbsent:": (r, a) => { addSlots(ref(r), ref(a[0]), false); return r; },
    "_Define:": (r, a) => { ref(r).slots.clear(); addSlots(r, ref(a[0]), true); return r; },
    "_RemoveSlot:": (r, a) => { version++; if (r instanceof SObj && r.slots.delete(str(a[0]))) return r; throw new Fail("slotNameError"); },
    "_ObjectName": (r) => describe(r),
    "_KindName": (r) => kindLabel(r),
    "_SlotNames": (r) => newVector(r instanceof SObj ? [...r.slots.keys()] : []),
    "_SlotValue:": (r, a) => { const s = r instanceof SObj && r.slots.get(str(a[0])); if (!s) throw new Fail("slotNameError"); return s.value; },
    "_SlotLookup:": (r, a) => {
      let found = []; lookupValue(r, str(a[0]), 0, [], found); found = dedupe(found);
      if (found.length !== 1) throw new Fail("slotNameError");
      return found[0].slot.value;
    },
    "_DefineSlot:Value:": (r, a) => { version++; const n = str(a[0]); ref(r).slots.set(n, new Slot(n, a[1], false, false, false)); setCreator(a[1], r, n); return r; },
    "_Perform:": (r, a) => send(r, str(a[0]), a.slice(1)),
    "_Perform:With:": (r, a) => send(r, str(a[0]), a.slice(1)),
    "_Perform:With:With:": (r, a) => send(r, str(a[0]), a.slice(1)),
    "_Perform:With:With:With:": (r, a) => send(r, str(a[0]), a.slice(1)),
    "_Error:": (r, a) => { throw new SelfError(typeof a[0] === "string" ? a[0] : describe(a[0])); },
    "_Inspect": (r) => r,
    "_TranscriptClear": (r) => { $.out = []; if ($.onClearTranscript) $.onClearTranscript(); return r; },
    "_CurrentTimeString": () => new Date().toUTCString(),
    "_TimeMsec": () => BigInt(Math.floor(nowMs())),
    "_Random:": (r, a) => { const n = int(a[0]); if (n <= 0n) throw new Fail("badSizeError"); return BigInt(Math.floor(Math.random() * Number(n))); },
    "_ObjectCount": () => BigInt(nextId),

    "_IntAsFloat": (r) => Number(int(r)),
    "_IntPrintString": (r) => int(r).toString(),
    "_IntAsCharacter": (r) => { const n = Number(int(r)); if (n < 0 || n > 0x10ffff || (n >= 0xd800 && n < 0xe000)) throw new Fail("badSizeError"); return String.fromCodePoint(n); },
    "_IntPrintStringBase:": (r, a) => { const b = int(a[0]); if (b < 2n || b > 36n) throw new Fail("badSizeError"); return int(r).toString(Number(b)); },
    "_IntAdd:": (r, a) => int(r) + int(a[0]),
    "_IntSub:": (r, a) => int(r) - int(a[0]),
    "_IntMul:": (r, a) => int(r) * int(a[0]),
    "_IntDiv:": (r, a) => { const y = int(a[0]); if (y === 0n) throw new Fail("divisionByZeroError"); return divE(int(r), y); },
    "_IntMod:": (r, a) => { const y = int(a[0]); if (y === 0n) throw new Fail("divisionByZeroError"); return modE(int(r), y); },
    "_IntRem:": (r, a) => { const y = int(a[0]); if (y === 0n) throw new Fail("divisionByZeroError"); return int(r) % y; },
    "_IntAnd:": (r, a) => int(r) & int(a[0]),
    "_IntOr:": (r, a) => int(r) | int(a[0]),
    "_IntXor:": (r, a) => int(r) ^ int(a[0]),
    "_IntArithmeticShiftLeft:": (r, a) => { const n = int(a[0]); if (n < 0n || n >= 100000n) throw new Fail("badSizeError"); return int(r) << n; },
    "_IntArithmeticShiftRight:": (r, a) => { const n = int(a[0]); if (n < 0n) throw new Fail("badSizeError"); return int(r) >> n; },
    "_IntLT:": (r, a) => bool(int(r) < int(a[0])),
    "_IntLE:": (r, a) => bool(int(r) <= int(a[0])),
    "_IntGT:": (r, a) => bool(int(r) > int(a[0])),
    "_IntGE:": (r, a) => bool(int(r) >= int(a[0])),
    "_IntEQ:": (r, a) => bool(int(r) === int(a[0])),
    "_IntNE:": (r, a) => bool(int(r) !== int(a[0])),

    "_FloatAdd:": (r, a) => num(r) + num(a[0]),
    "_FloatSub:": (r, a) => num(r) - num(a[0]),
    "_FloatMul:": (r, a) => num(r) * num(a[0]),
    "_FloatDiv:": (r, a) => { const y = num(a[0]); if (y === 0) throw new Fail("divisionByZeroError"); return num(r) / y; },
    "_FloatMod:": (r, a) => { const x = num(r), y = Math.abs(num(a[0])); return ((x % y) + y) % y; },
    "_FloatPow:": (r, a) => Math.pow(num(r), num(a[0])),
    "_FloatLT:": (r, a) => bool(num(r) < num(a[0])),
    "_FloatLE:": (r, a) => bool(num(r) <= num(a[0])),
    "_FloatGT:": (r, a) => bool(num(r) > num(a[0])),
    "_FloatGE:": (r, a) => bool(num(r) >= num(a[0])),
    "_FloatEQ:": (r, a) => bool(num(r) === num(a[0])),
    "_FloatNE:": (r, a) => bool(num(r) !== num(a[0])),
    "_FloatAsInt": (r) => { const f = num(r); if (!Number.isFinite(f) || Math.abs(f) > 9.2e18) throw new Fail("overflowError"); return BigInt(Math.trunc(f)); },
    "_FloatTruncate": (r) => P["_FloatAsInt"](r),
    "_FloatRound": (r) => { const f = num(r); return BigInt(Math.sign(f) * Math.round(Math.abs(f))); },
    "_FloatFloor": (r) => BigInt(Math.floor(num(r))),
    "_FloatCeil": (r) => BigInt(Math.ceil(num(r))),
    "_FloatSqrt": (r) => Math.sqrt(num(r)),
    "_FloatSin": (r) => Math.sin(num(r)),
    "_FloatCos": (r) => Math.cos(num(r)),
    "_FloatTan": (r) => Math.tan(num(r)),
    "_FloatArcTan": (r) => Math.atan(num(r)),
    "_FloatLn": (r) => Math.log(num(r)),
    "_FloatExp": (r) => Math.exp(num(r)),
    "_FloatPrintString": (r) => fmtFloat(num(r)),

    "_StringSize": (r) => BigInt(chars(str(r)).length),
    "_StringAt:": (r, a) => { const i = int(a[0]); const c = i >= 0n ? chars(str(r))[Number(i)] : undefined; if (c === undefined) throw new Fail("badIndexError"); return c; },
    "_StringConcat:": (r, a) => str(r) + str(a[0]),
    "_StringEQ:": (r, a) => bool(typeof a[0] === "string" && str(r) === a[0]),
    "_StringLT:": (r, a) => bool(str(r) < str(a[0])),
    "_StringHash": (r) => strHash(str(r)),
    "_StringCopyFrom:UpTo:": (r, a) => {
      const cs = chars(str(r)), from = int(a[0]), to = int(a[1]);
      if (from < 0n || to < from || Number(to) > cs.length) throw new Fail("badIndexError");
      return cs.slice(Number(from), Number(to)).join("");
    },
    "_StringIndexOf:": (r, a) => { const s = str(r), i = s.indexOf(str(a[0])); return i < 0 ? -1n : BigInt(chars(s.slice(0, i)).length); },
    "_StringAsInteger": (r) => { const s = str(r).trim(); if (!/^[+-]?\d+$/.test(s)) throw new Fail("badFormatError"); return BigInt(s); },
    "_StringAsFloat": (r) => {
      const s = str(r).trim();
      if (/^[+-]?(\d+\.?\d*|\.\d+)([eE][+-]?\d+)?$/.test(s)) return parseFloat(s);
      if (/^[+-]?inf(inity)?$/i.test(s)) return s.startsWith("-") ? -Infinity : Infinity;
      if (/^nan$/i.test(s)) return NaN;
      throw new Fail("badFormatError");
    },
    "_StringAsUppercase": (r) => str(r).toUpperCase(),
    "_StringAsLowercase": (r) => str(r).toLowerCase(),
    "_StringReverse": (r) => chars(str(r)).reverse().join(""),
    "_StringTrim": (r) => str(r).trim(),
    "_StringSplit:": (r, a) => {
      const s = str(r), sep = str(a[0]);
      let parts;
      if (sep === " ") parts = s.split(/\s+/).filter((x) => x !== "");
      else if (sep === "\n") parts = s.split("\n").map((x) => (x.endsWith("\r") ? x.slice(0, -1) : x));
      else if (sep === "") parts = chars(s);
      else parts = s.split(sep);
      return newVector(parts);
    },
    "_StringByteAt:": (r, a) => { const i = int(a[0]); const c = i >= 0n ? chars(str(r))[Number(i)] : undefined; if (c === undefined) throw new Fail("badIndexError"); return BigInt(c.codePointAt(0)); },
    "_StringPrintString": (r) => quote(str(r)),
    "_StringPrint": (r) => { print(str(r)); return r; },
    "_StringPrintLine": (r) => { print(str(r) + "\n"); return r; },
    "_StringEval": (r) => {
      if ($.evalHook) return $.evalHook(str(r));
      throw new SelfError("eval needs the Self compiler, which a translated program doesn't include");
    },

    "_Size": (r) => {
      if (typeof r === "string") return BigInt(chars(r).length);
      if (!(r instanceof SObj)) throw new Fail("badTypeError");
      return r.kind === "vector" ? BigInt(r.items.length) : 0n;
    },
    "_At:": (r, a) => {
      const i = int(a[0]);
      if (!(r instanceof SObj) || r.kind !== "vector") throw new Fail("badTypeError");
      if (i < 0n || i >= BigInt(r.items.length)) throw new Fail("badIndexError");
      return r.items[Number(i)];
    },
    "_At:Put:": (r, a) => {
      const i = int(a[0]);
      if (!(r instanceof SObj) || r.kind !== "vector") throw new Fail("badTypeError");
      if (i < 0n || i >= BigInt(r.items.length)) throw new Fail("badIndexError");
      r.items[Number(i)] = a[1];
      return r;
    },
    "_Clone:Filler:": (r, a) => {
      const n = int(a[0]);
      if (n < 0n || n > 50000000n) throw new Fail("badSizeError");
      if (!(r instanceof SObj) || r.kind !== "vector") throw new Fail("badTypeError");
      const c = cloneObj(r);
      c.items = r.items.slice(0, Number(n));
      while (c.items.length < Number(n)) c.items.push(a[1]);
      c.creator = null;
      return c;
    },
    "_MakeVector": (r) => {
      const v = new SObj("vector");
      if (r instanceof SObj) for (const s of r.slots.values()) v.slots.set(s.name, new Slot(s.name, s.value, s.assignable, s.parent, s.arg));
      v.items = [];
      return v;
    },

    "_BlockNumArgs": (r) => { if (!(r instanceof SObj) || r.kind !== "block") throw new Fail("badTypeError"); return BigInt(r.nargs); },
    "_WhileTrue:": (r, a) => { while (isTrue(callBlock(ref(r), []))) { send(a[0], "value", []); } return nil; },
    "_WhileFalse:": (r, a) => { while (!isTrue(callBlock(ref(r), []))) { send(a[0], "value", []); } return nil; },
    "_Loop": (r) => { const b = ref(r); for (;;) { callBlock(b, []); if (++sends > SEND_LIMIT) { sends = 0; throw new SelfError("stopped: this loop ran too long — is it infinite?"); } } },
    "_ValueWithArguments:": (r, a) => { const v = ref(a[0]); if (v.kind !== "vector") throw new Fail("badTypeError"); return callBlock(ref(r), v.items.slice()); },
    "_OnError:": (r, a) => {
      try { return callBlock(ref(r), []); }
      catch (e) {
        const m = errorMessage(e);
        if (m === null) throw e;
        return callBlockLenient(a[0], [m]);
      }
    },
    "_OnReturn:": (r, a) => {
      let v;
      try { v = callBlock(ref(r), []); }
      catch (e) { callBlockLenient(a[0], [nil]); throw e; }
      callBlockLenient(a[0], [v]);
      return v;
    },
    "_OnNonLocalReturn:": (r, a) => {
      try { return callBlock(ref(r), []); }
      catch (e) { callBlockLenient(a[0], [nil]); throw e; }
    },

    "_FileContents": (r) => { const t = store.getItem(fileKey(str(r))); if (t === null) throw new Fail("cannot read " + r + ": no such file"); return t; },
    "_SetFileContents:": (r, a) => { store.setItem(fileKey(str(r)), str(a[0])); return r; },
    "_FileExists": (r) => bool(store.getItem(fileKey(str(r))) !== null),
    "_SourceFolder": () => "browser-storage",
  };
  for (const n of ["_FileOut", "_FileOutChanges", "_LoadSubpart:From:", "_BootstrapStub", "_BootstrapArrow:", "_StringAsJavaScript", "_StringAsWebApp:"])
    P[n] = () => { throw new SelfError(n + " is not available in a translated program"); };

  // the message a Self error handler gets, or null for a non-local return
  function errorMessage(e) {
    if (e instanceof NLR) return null;
    if (e instanceof SelfError) return e.msg;
    if (e instanceof RangeError) return "stack overflow: too many nested calls";
    if (e instanceof Fail) return "primitive failed: " + e.code;
    return String((e && e.message) || e);
  }
  $.errorMessage = errorMessage;

  function prim(r, sel, args) {
    let base = sel, fb = null;
    if (sel.endsWith("IfFail:") && sel.length > 8) { base = sel.slice(0, -7); fb = args[args.length - 1]; args = args.slice(0, -1); }
    const f = P[base];
    if (!f) throw new SelfError("unknown primitive " + base);
    try { return f(r, args); }
    catch (e) {
      if (!(e instanceof Fail)) throw e;
      if (fb !== null) return callBlockLenient(fb, [e.code, base]);
      throw new SelfError("primitive " + base + " failed: " + e.code);
    }
  }
  $.prim = prim;

  // ------------------------------------------------------------ running translated programs
  // Each top-level statement runs on its own, like a Self file: a failure is
  // reported with its line and the rest still runs.
  $.failures = [];
  $.onFailure = null;
  $.stmt = function (line, f) {
    sends = 0;
    try { return f(); }
    catch (e) {
      const m = errorMessage(e) || "a block returned to a method that has already finished";
      const msg = "line " + line + ": " + m;
      $.failures.push(msg);
      if ($.onFailure) $.onFailure(msg);
      return nil;
    }
  };
  // run a message from outside (the user interface); errors come back as messages
  $.ui = function (r, sel, args) {
    sends = 0;
    try { return { value: send(r, sel, args || []) }; }
    catch (e) { return { error: errorMessage(e) || "a block returned to a method that has already finished" }; }
  };
  $.str = (v) => (typeof v === "string" ? v : null);
  $.num = (v) => (typeof v === "bigint" ? Number(v) : typeof v === "number" ? v : null);
  $.items = (v) => (v instanceof SObj && v.kind === "vector" ? v.items : null);
  $.int = (n) => BigInt(Math.round(n));
  $.isTrue = (v) => v === T;
  return $;
})();
if (typeof module !== "undefined") module.exports = SelfRT;
