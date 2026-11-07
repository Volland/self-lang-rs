// Morphic for Self programs translated to JavaScript: draws the world's
// morphs with the browser's DOM and canvases, reading their slots every
// frame exactly as the desktop does (see src/morphic.rs), and sends user
// actions back as messages (click / clickAtX:Y:, contents: + changed,
// x: and y: when dragged, keyDown: / keyUp:, step).
"use strict";
var SelfMorphic = (function () {
  const $ = SelfRT;
  const PALETTE = {
    accent: "#8b7bff", text: "#e6e8ef", faint: "#8a93a6", panel: "#1a1d27", background: "#232634",
    none: "transparent", transparent: "transparent", red: "#ef4444", tomato: "#ff6347", orange: "#f97316",
    yellow: "#facc15", gold: "#f5b01a", green: "#22c55e", teal: "#14b8a6", cyan: "#06b6d4", blue: "#3b82f6",
    navy: "#1e3a8a", purple: "#a855f7", magenta: "#d946ef", pink: "#ec4899", brown: "#92400e",
    gray: "#8a93a6", grey: "#8a93a6", black: "#000000", white: "#ffffff",
  };
  const CSS = `
  html,body{margin:0;height:100%;background:#0e1016;color:#e6e8ef;font:14px/1.4 system-ui,-apple-system,"Segoe UI",sans-serif;overflow:hidden}
  #sw-root{position:fixed;inset:0}
  #sw-shapes{position:absolute;inset:0}
  .sw-window{position:absolute;background:#1a1d27;border:1px solid #2e3344;border-radius:10px;box-shadow:0 10px 30px #0008;display:flex;flex-direction:column;min-width:140px;max-width:calc(100vw - 16px)}
  .sw-title{display:flex;align-items:center;gap:8px;padding:7px 10px;background:#232634;border-radius:10px 10px 0 0;cursor:move;user-select:none;font-weight:600}
  .sw-title span{flex:1;text-align:center}
  .sw-title button{background:none;border:0;color:#8a93a6;font-size:16px;cursor:pointer;padding:0 4px}
  .sw-body{padding:10px;display:flex;flex-direction:column;gap:7px;overflow:auto}
  .sw-row{display:flex;flex-wrap:wrap;gap:6px;align-items:center}
  .sw-col{display:flex;flex-direction:column;gap:7px}
  .sw-float{position:absolute;background:#1a1d27;border:1px solid #2e3344;border-radius:8px;padding:8px}
  .sw-body button,.sw-float button{background:#2a2f40;color:#e6e8ef;border:1px solid #39405a;border-radius:6px;padding:4px 10px;font:inherit;cursor:pointer}
  .sw-body button:hover,.sw-float button:hover{background:#343b52}
  input[type=text],textarea,select{background:#0e1016;color:#e6e8ef;border:1px solid #39405a;border-radius:6px;padding:4px 7px;font:inherit;box-sizing:border-box;max-width:100%}
  textarea{resize:vertical}
  .sw-list{border:1px solid #39405a;border-radius:6px;overflow:auto;background:#141720;max-width:100%;box-sizing:border-box}
  .sw-list div{padding:3px 8px;cursor:pointer;white-space:nowrap;overflow:hidden;text-overflow:ellipsis}
  .sw-list div:hover{background:#232634}
  .sw-list div.sel{background:#8b7bff44}
  .sw-progress{display:flex;align-items:center;gap:8px}
  .sw-progress progress{flex:1;accent-color:#8b7bff}
  .sw-slider{display:flex;align-items:center;gap:8px}
  .sw-slider input{accent-color:#8b7bff}
  hr.sw-sep{border:0;border-top:1px solid #2e3344;width:100%;margin:2px 0}
  #sw-errors{position:fixed;left:0;right:0;bottom:0;background:#2a1418;border-top:1px solid #ef4444;color:#fecaca;padding:8px 12px;font:12px ui-monospace,monospace;max-height:30vh;overflow:auto;display:none;z-index:10}
  #sw-errors button{float:right;background:none;border:1px solid #ef4444;color:#fecaca;border-radius:5px;cursor:pointer}
  #sw-transcript{position:fixed;right:10px;bottom:10px;width:min(420px,calc(100vw - 20px));max-height:35vh;background:#141720ee;border:1px solid #2e3344;border-radius:8px;font:12px ui-monospace,monospace;display:none;z-index:9}
  #sw-transcript header{display:flex;justify-content:space-between;padding:5px 9px;border-bottom:1px solid #2e3344;color:#8a93a6}
  #sw-transcript header button{background:none;border:0;color:#8a93a6;cursor:pointer}
  #sw-transcript pre{margin:0;padding:7px 9px;white-space:pre-wrap;overflow:auto;max-height:28vh}
  #sw-empty{position:absolute;inset:0;display:flex;align-items:center;justify-content:center;color:#8a93a6;pointer-events:none}
  `;
  let root, shapes, ctx, layer, errors, transcript, transcriptPre, empty;
  const els = new Map();
  let dragging = null;

  const color = (s, dflt) => {
    if (typeof s !== "string") return dflt;
    const k = s.trim().toLowerCase();
    if (PALETTE[k]) return PALETTE[k];
    if (/^#([0-9a-f]{3}|[0-9a-f]{6}|[0-9a-f]{8})$/.test(k)) return k;
    return dflt;
  };
  function get(m, sel) { const r = $.ui(m, sel, []); return r.error ? undefined : r.value; }
  const getn = (m, sel, d) => { const v = $.num(get(m, sel)); return v === null || !Number.isFinite(v) ? d : v; };
  const gets = (m, sel) => { const v = get(m, sel); return v === undefined ? "" : typeof v === "string" ? v : $.printString(v); };
  const getb = (m, sel) => $.isTrue(get(m, sel));
  const kids = (m) => $.items(get(m, "morphs")) || [];
  const itemText = (v) => (typeof v === "string" ? v : $.printString(v));
  function act(m, sel, args) {
    const r = $.ui(m, sel, args || []);
    if (r.error) report($.describe(m) + " " + sel + ": " + r.error);
    return r;
  }
  function report(msg) {
    errors.style.display = "block";
    if (errors.children.length > 40) errors.lastChild.remove();
    const d = document.createElement("div");
    d.textContent = "⚠ " + msg;
    errors.appendChild(d);
  }
  const world = () => get($.lobby, "world");

  function install(el, title) {
    document.title = title;
    const style = document.createElement("style");
    style.textContent = CSS;
    document.head.appendChild(style);
    root = document.createElement("div");
    root.id = "sw-root";
    el.appendChild(root);
    shapes = document.createElement("canvas");
    shapes.id = "sw-shapes";
    root.appendChild(shapes);
    ctx = shapes.getContext("2d");
    layer = document.createElement("div");
    root.appendChild(layer);
    empty = document.createElement("div");
    empty.id = "sw-empty";
    empty.textContent = "This program put no morphs in the world.";
    root.appendChild(empty);
    errors = document.createElement("div");
    errors.id = "sw-errors";
    const dismiss = document.createElement("button");
    dismiss.textContent = "Dismiss";
    dismiss.onclick = () => { errors.style.display = "none"; while (errors.children.length > 1) errors.lastChild.remove(); };
    errors.appendChild(dismiss);
    document.body.appendChild(errors);
    transcript = document.createElement("div");
    transcript.id = "sw-transcript";
    transcript.innerHTML = "<header><span>Transcript</span><button title='Close'>×</button></header><pre></pre>";
    transcript.querySelector("button").onclick = () => (transcript.style.display = "none");
    transcriptPre = transcript.querySelector("pre");
    document.body.appendChild(transcript);
    $.onPrint = (s) => {
      transcript.style.display = "block";
      transcriptPre.textContent += s;
      if (transcriptPre.textContent.length > 20000) transcriptPre.textContent = transcriptPre.textContent.slice(-15000);
      transcriptPre.scrollTop = transcriptPre.scrollHeight;
    };
    $.onClearTranscript = () => (transcriptPre.textContent = "");
    $.onFailure = report;
    shapes.addEventListener("pointerdown", (e) => pointerDown(e, null));
    window.addEventListener("pointermove", pointerMove);
    window.addEventListener("pointerup", () => (dragging = null));
    window.addEventListener("keydown", (e) => key(e, true));
    window.addEventListener("keyup", (e) => key(e, false));
  }

  // ------------------------------------------------------------ shapes (drawn on canvases)
  const isShape = (k) => k === "rectangle" || k === "circle" || k === "label" || k === "line";
  function shapeGeom(m, ox, oy) {
    const k = gets(m, "morphKind");
    const x = ox + getn(m, "x", 0), y = oy + getn(m, "y", 0);
    const w = getn(m, "width", 40), h = getn(m, "height", 40);
    return { m, k, x, y, w, h };
  }
  function drawShapes(c, list, ox, oy, depth, hits) {
    if (depth > 12) return;
    for (const m of list) {
      const g = shapeGeom(m, ox, oy);
      if (!isShape(g.k)) continue;
      const w = Math.max(1, g.w), h = Math.max(1, g.h);
      const fill = color(gets(m, "color"), g.k === "label" || g.k === "line" ? PALETTE.text : PALETTE.accent);
      const bw = getn(m, "borderWidth", 0);
      const bc = color(gets(m, "borderColor"), PALETTE.text);
      c.save();
      if (g.k === "circle") {
        c.beginPath(); c.arc(g.x + w / 2, g.y + h / 2, Math.min(w, h) / 2, 0, Math.PI * 2);
        c.fillStyle = fill; c.fill();
        if (bw > 0) { c.lineWidth = bw; c.strokeStyle = bc; c.stroke(); }
      } else if (g.k === "rectangle") {
        const r = Math.min(getn(m, "cornerRadius", 6), w / 2, h / 2);
        c.beginPath(); c.roundRect ? c.roundRect(g.x, g.y, w, h, r) : c.rect(g.x, g.y, w, h);
        c.fillStyle = fill; c.fill();
        if (bw > 0) { c.lineWidth = bw; c.strokeStyle = bc; c.stroke(); }
      } else if (g.k === "label") {
        const size = Math.min(200, Math.max(6, getn(m, "fontSize", 16)));
        c.font = size + "px system-ui,-apple-system,'Segoe UI',sans-serif";
        c.textBaseline = "top"; c.fillStyle = fill;
        const text = gets(m, "text");
        c.fillText(text, g.x, g.y);
        g.w = c.measureText(text).width; g.h = size * 1.2;
      } else if (g.k === "line") {
        c.beginPath(); c.moveTo(g.x, g.y); c.lineTo(g.x + g.w, g.y + g.h);
        c.lineWidth = Math.min(60, Math.max(0.5, getn(m, "lineWidth", 3))); c.strokeStyle = fill; c.lineCap = "round"; c.stroke();
      }
      c.restore();
      hits.push(g);
      if (g.k !== "line") drawShapes(c, kids(m), g.x, g.y, depth + 1, hits);
    }
  }
  function hitTest(hits, px, py) {
    for (let i = hits.length - 1; i >= 0; i--) {
      const g = hits[i];
      if (g.k === "circle") {
        const r = Math.min(g.w, g.h) / 2;
        if (Math.hypot(px - (g.x + g.w / 2), py - (g.y + g.h / 2)) <= r) return g;
      } else if (g.k === "line") {
        const dx = g.w, dy = g.h, len = dx * dx + dy * dy || 1;
        const t = Math.max(0, Math.min(1, ((px - g.x) * dx + (py - g.y) * dy) / len));
        if (Math.hypot(px - (g.x + t * dx), py - (g.y + t * dy)) <= 6) return g;
      } else if (px >= g.x && px <= g.x + g.w && py >= g.y && py <= g.y + g.h) return g;
    }
    return null;
  }
  let worldHits = [];
  function pointerDown(e, canvasMorph) {
    const target = e.currentTarget;
    const rect = target.getBoundingClientRect();
    const px = e.clientX - rect.left, py = e.clientY - rect.top;
    const hits = canvasMorph ? target._hits || [] : worldHits;
    const g = hitTest(hits, px, py);
    if (g) {
      dragging = { m: g.m, sx: e.clientX, sy: e.clientY, x: getn(g.m, "x", 0), y: getn(g.m, "y", 0), moved: false, px: px - g.x, py: py - g.y };
      e.preventDefault();
    } else if (canvasMorph) {
      act(canvasMorph, "clickAtX:Y:", [$.int(px), $.int(py)]);
    }
  }
  function pointerMove(e) {
    if (!dragging) return;
    const dx = e.clientX - dragging.sx, dy = e.clientY - dragging.sy;
    if (Math.abs(dx) + Math.abs(dy) > 3) dragging.moved = true;
    if (dragging.moved) {
      $.ui(dragging.m, "x:", [$.int(dragging.x + dx)]);
      $.ui(dragging.m, "y:", [$.int(dragging.y + dy)]);
    }
  }
  window.addEventListener("pointerup", () => {
    if (dragging && !dragging.moved) act(dragging.m, "clickAtX:Y:", [$.int(dragging.px), $.int(dragging.py)]);
    dragging = null;
  }, true);

  // ------------------------------------------------------------ widgets (DOM elements)
  function elFor(m, kind, make) {
    let e = els.get(m);
    if (!e || e._kind !== kind) {
      e = make();
      e._kind = kind;
      e._morph = m;
      els.set(m, e);
    }
    e._seen = frame;
    return e;
  }
  function sync(container, elements) {
    for (let i = 0; i < elements.length; i++) {
      if (container.children[i] !== elements[i]) container.insertBefore(elements[i], container.children[i] || null);
    }
    while (container.children.length > elements.length) container.lastChild.remove();
  }
  const focused = (e) => document.activeElement === e;

  function widget(m, depth) {
    const k = gets(m, "morphKind");
    if (depth > 16) return elFor(m, "deep", () => document.createElement("span"));
    switch (k) {
      case "label": {
        const e = elFor(m, k, () => document.createElement("div"));
        e.textContent = gets(m, "text");
        e.style.fontSize = Math.min(200, Math.max(6, getn(m, "fontSize", 16))) + "px";
        e.style.color = color(gets(m, "color"), PALETTE.text);
        return e;
      }
      case "button": {
        const e = elFor(m, k, () => { const b = document.createElement("button"); b.onclick = () => act(m, "click"); return b; });
        const t = gets(m, "label");
        if (e.textContent !== t) e.textContent = t;
        return e;
      }
      case "textField":
      case "textArea": {
        const e = elFor(m, k, () => {
          const i = document.createElement(k === "textArea" ? "textarea" : "input");
          if (k === "textField") i.type = "text";
          i.oninput = () => { act(m, "contents:", [i.value]); act(m, "changed"); };
          return i;
        });
        const v = gets(m, "contents");
        if (!focused(e) && e.value !== v) e.value = v;
        e.style.width = getn(m, "width", k === "textArea" ? 260 : 180) + "px";
        if (k === "textArea") e.style.height = getn(m, "height", 120) + "px";
        return e;
      }
      case "slider": {
        const e = elFor(m, k, () => {
          const d = document.createElement("div"); d.className = "sw-slider";
          const i = document.createElement("input"); i.type = "range";
          const out = document.createElement("span");
          d.append(i, out);
          i.oninput = () => {
            const ints = typeof get(m, "value") === "bigint";
            act(m, "value:", [ints ? $.int(Number(i.value)) : Number(i.value)]);
            act(m, "changed");
          };
          return d;
        });
        const [i, out] = e.children;
        const lo = getn(m, "min", 0), hi = getn(m, "max", 100), v = getn(m, "value", lo);
        i.min = Math.min(lo, hi); i.max = Math.max(lo, hi);
        i.step = typeof get(m, "value") === "bigint" ? 1 : "any";
        if (!focused(i)) i.value = v;
        i.style.width = getn(m, "width", 180) + "px";
        out.textContent = typeof get(m, "value") === "bigint" ? String(v) : v.toFixed(2);
        return e;
      }
      case "checkbox": {
        const e = elFor(m, k, () => {
          const l = document.createElement("label");
          const i = document.createElement("input"); i.type = "checkbox";
          const s = document.createElement("span");
          l.append(i, " ", s);
          i.onchange = () => { act(m, "checked:", [i.checked ? $.T : $.F]); act(m, "changed"); };
          return l;
        });
        e.children[0].checked = getb(m, "checked");
        e.children[1].textContent = gets(m, "label");
        return e;
      }
      case "dropDown": {
        const e = elFor(m, k, () => {
          const s = document.createElement("select");
          s.onchange = () => {
            const choices = $.items(get(m, "choices")) || [];
            const c = choices[s.selectedIndex];
            if (c !== undefined) { act(m, "selected:", [c]); act(m, "changed"); }
          };
          return s;
        });
        const choices = $.items(get(m, "choices")) || [];
        const labels = choices.map(itemText);
        const key = labels.join("\u0001");
        if (e._key !== key) {
          e.innerHTML = "";
          for (const l of labels) { const o = document.createElement("option"); o.textContent = l; e.appendChild(o); }
          e._key = key;
        }
        const sel = get(m, "selected");
        let idx = choices.findIndex((c) => c === sel);
        if (idx < 0 && sel !== undefined) idx = labels.indexOf(itemText(sel));
        if (!focused(e)) e.selectedIndex = idx;
        e.style.width = getn(m, "width", 180) + "px";
        return e;
      }
      case "list": {
        const e = elFor(m, k, () => { const d = document.createElement("div"); d.className = "sw-list"; return d; });
        const items = ($.items(get(m, "items")) || []).map(itemText);
        const sel = getn(m, "selectedIndex", -1);
        const key = items.join("\u0001") + "\u0002" + sel;
        if (e._key !== key) {
          e.innerHTML = "";
          items.forEach((t, i) => {
            const d = document.createElement("div");
            d.textContent = t;
            if (i === sel) d.className = "sel";
            d.onclick = () => { act(m, "selectedIndex:", [$.int(i)]); act(m, "changed"); };
            e.appendChild(d);
          });
          if (!items.length) { const d = document.createElement("div"); d.textContent = "(no items)"; d.style.color = PALETTE.faint; e.appendChild(d); }
          e._key = key;
        }
        e.style.width = getn(m, "width", 220) + "px";
        e.style.maxHeight = getn(m, "height", 140) + "px";
        return e;
      }
      case "progress": {
        const e = elFor(m, k, () => {
          const d = document.createElement("div"); d.className = "sw-progress";
          d.append(document.createElement("progress"), document.createElement("span"));
          return d;
        });
        const v = getn(m, "value", 0), max = getn(m, "max", 100);
        const f = Math.abs(max) > 1e-9 ? Math.min(1, Math.max(0, v / max)) : 0;
        e.children[0].value = f; e.children[0].max = 1;
        e.children[1].textContent = Math.round(f * 100) + "%";
        e.style.width = getn(m, "width", 200) + "px";
        return e;
      }
      case "separator":
        return elFor(m, k, () => { const h = document.createElement("hr"); h.className = "sw-sep"; return h; });
      case "row":
      case "column":
      case "window": {
        const e = elFor(m, k, () => { const d = document.createElement("div"); d.className = k === "row" ? "sw-row" : "sw-col"; return d; });
        sync(e, kids(m).map((c) => widget(c, depth + 1)));
        return e;
      }
      case "canvas":
      case "rectangle":
      case "circle":
      case "line": {
        const e = elFor(m, k, () => {
          const c = document.createElement("canvas");
          c.style.borderRadius = k === "canvas" ? "8px" : "0";
          c.style.touchAction = "none";
          c.addEventListener("pointerdown", (ev) => pointerDown(ev, k === "canvas" ? m : null));
          return c;
        });
        const w = Math.max(1, getn(m, "width", 320)), h = Math.max(1, getn(m, "height", 200));
        const dpr = window.devicePixelRatio || 1;
        if (e.width !== Math.round(w * dpr) || e.height !== Math.round(h * dpr)) {
          e.width = Math.round(w * dpr); e.height = Math.round(h * dpr);
          e.style.width = w + "px"; e.style.height = h + "px";
        }
        const c = e.getContext("2d");
        c.setTransform(dpr, 0, 0, dpr, 0, 0);
        c.clearRect(0, 0, w, h);
        const hits = [];
        if (k === "canvas") {
          c.fillStyle = color(gets(m, "color"), PALETTE.background);
          c.beginPath(); c.roundRect ? c.roundRect(0, 0, w, h, 8) : c.rect(0, 0, w, h); c.fill();
          drawShapes(c, kids(m), 0, 0, depth + 1, hits);
        } else {
          // a shape laid out in a window: drawn at its own origin
          const saved = [getn(m, "x", 0), getn(m, "y", 0)];
          c.translate(-saved[0], -saved[1]);
          drawShapes(c, [m], 0, 0, depth, hits);
          hits.forEach((g) => { g.x -= saved[0]; g.y -= saved[1]; });
        }
        e._hits = hits;
        return e;
      }
      default: {
        const e = elFor(m, "other", () => document.createElement("span"));
        e.textContent = $.describe(m);
        e.style.color = PALETTE.faint;
        return e;
      }
    }
  }

  function windowEl(m) {
    const e = elFor(m, "window-frame", () => {
      const w = document.createElement("div");
      w.className = "sw-window";
      w.innerHTML = "<div class='sw-title'><span></span><button title='Close'>×</button></div><div class='sw-body'></div>";
      const title = w.firstChild;
      title.querySelector("button").onclick = () => act(m, "close");
      title.addEventListener("pointerdown", (ev) => {
        if (ev.target.tagName === "BUTTON") return;
        const start = { sx: ev.clientX, sy: ev.clientY, x: getn(m, "x", 0), y: getn(m, "y", 0) };
        const move = (mv) => {
          $.ui(m, "x:", [$.int(Math.max(0, start.x + mv.clientX - start.sx))]);
          $.ui(m, "y:", [$.int(Math.max(0, start.y + mv.clientY - start.sy))]);
        };
        const up = () => { window.removeEventListener("pointermove", move); window.removeEventListener("pointerup", up); };
        window.addEventListener("pointermove", move);
        window.addEventListener("pointerup", up);
        ev.preventDefault();
      });
      return w;
    });
    const narrow = root.clientWidth < 520;
    const x = narrow ? 8 : getn(m, "x", 100), y = getn(m, "y", 100);
    e.style.left = x + "px";
    e.style.top = y + "px";
    e.style.width = (narrow ? root.clientWidth - 18 : Math.max(140, getn(m, "width", 280))) + "px";
    e.firstChild.firstChild.textContent = gets(m, "title") || "Window";
    const body = e.lastChild;
    const items = kids(m);
    if (!items.length) {
      sync(body, [elFor(m, "window-empty-note", () => { const s = document.createElement("div"); s.textContent = "An empty window. Add morphs with addMorph:"; s.style.color = PALETTE.faint; return s; })]);
    } else {
      sync(body, items.map((c) => widget(c, 1)));
    }
    return e;
  }

  // ------------------------------------------------------------ every frame
  let frame = 0;
  function allMorphs(w) {
    const out = [], stack = kids(w).slice().reverse();
    while (stack.length && out.length < 1500) {
      const m = stack.pop();
      if (out.includes(m)) continue;
      out.push(m);
      stack.push(...kids(m).slice().reverse());
    }
    return out;
  }
  function key(e, down) {
    const t = e.target;
    if (t && (t.tagName === "INPUT" || t.tagName === "TEXTAREA" || t.tagName === "SELECT")) return;
    const w = world();
    if (!w) return;
    const targets = [w, ...allMorphs(w)].filter((m) => getb(m, "wantsKeys"));
    if (!targets.length) return;
    let name = e.key;
    if (name === " ") name = "Space";
    else if (name.length === 1) name = name.toUpperCase();
    if (e.repeat && !down) return;
    for (const m of targets) act(m, down ? "keyDown:" : "keyUp:", [name]);
    if (["ArrowUp", "ArrowDown", "ArrowLeft", "ArrowRight", "Space"].includes(name)) e.preventDefault();
  }
  // stepping morphs get step about 30 times a second (on a timer, so it
  // keeps going even when the browser draws few frames)
  function stepAll() {
    const w = world();
    if (!w) return;
    for (const m of allMorphs(w)) {
      if (getb(m, "isStepping")) {
        const r = $.ui(m, "step", []);
        if (r.error) { report($.describe(m) + " step (stepping stopped): " + r.error); $.ui(m, "isStepping:", [$.F]); }
      }
    }
  }
  let lastDraw = 0;
  function safeDraw() {
    lastDraw = Date.now();
    try { draw(); } catch (e) { report("drawing: " + (e && e.message || e)); }
  }
  function tick() {
    safeDraw();
    requestAnimationFrame(tick);
  }
  function draw() {
    frame++;
    const w = world();
    if (w) {
      const W = root.clientWidth, H = root.clientHeight;
      if (getn(w, "width", 0) !== W) $.ui(w, "width:", [$.int(W)]);
      if (getn(w, "height", 0) !== H) $.ui(w, "height:", [$.int(H)]);
      const t = get(w, "title");
      if (typeof t === "string" && t && document.title !== t) document.title = t;
      // shapes in the world, on the full-window canvas
      const dpr = window.devicePixelRatio || 1;
      if (shapes.width !== Math.round(W * dpr) || shapes.height !== Math.round(H * dpr)) {
        shapes.width = Math.round(W * dpr); shapes.height = Math.round(H * dpr);
      }
      ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
      ctx.clearRect(0, 0, W, H);
      const top = kids(w);
      worldHits = [];
      drawShapes(ctx, top, 0, 0, 0, worldHits);
      // windows and free-floating widgets, as DOM elements
      const doms = [];
      for (const m of top) {
        const k = gets(m, "morphKind");
        if (k === "window") doms.push(windowEl(m));
        else if (!isShape(k)) {
          const f = elFor(m, "float-frame", () => { const d = document.createElement("div"); d.className = "sw-float"; return d; });
          f.style.left = getn(m, "x", 0) + "px";
          f.style.top = getn(m, "y", 0) + "px";
          sync(f, [widget(m, 1)]);
          doms.push(f);
        }
      }
      sync(layer, doms);
      empty.style.display = top.length ? "none" : "flex";
      if (frame % 120 === 0) for (const [m, e] of els) if (e._seen < frame - 2) els.delete(m);
    }
  }
  function start() {
    $.resetSends();
    safeDraw();
    setInterval(() => {
      try { stepAll(); } catch (e) { report("stepping: " + (e && e.message || e)); }
      // if the browser isn't giving us animation frames, draw from here
      if (Date.now() - lastDraw > 250) safeDraw();
    }, 33);
    requestAnimationFrame(tick);
  }
  return { install, start };
})();
