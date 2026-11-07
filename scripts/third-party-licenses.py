#!/usr/bin/env python3
"""Generate THIRD_PARTY_LICENSES.md and web/licenses.html from the real
dependency graph of every build Self World ships (macOS, Linux, Windows and
the WebAssembly website), with the full licence texts of every crate and of
the fonts compiled into the program.

Run it from the repository root whenever Cargo.lock changes:

    python3 scripts/third-party-licenses.py
"""
import hashlib
import html
import json
import os
import re
import subprocess
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
TARGETS = {
    "aarch64-apple-darwin": "macOS",
    "x86_64-apple-darwin": "macOS",
    "x86_64-unknown-linux-gnu": "Linux",
    "x86_64-pc-windows-msvc": "Windows",
    "wasm32-unknown-unknown": "Web",
}
LICENSE_FILE = re.compile(r"^(licen[cs]e|copying|notice|unlicense|copyright)([-_.][\w.-]*)?$", re.I)


def metadata(target):
    out = subprocess.run(
        ["cargo", "metadata", "--format-version", "1", "--locked", "--filter-platform", target],
        cwd=ROOT, check=True, capture_output=True, text=True).stdout
    return json.loads(out)


def shipped(m):
    """Packages reachable from self-world through normal (not dev or build) dependencies."""
    nodes = {n["id"]: n for n in m["resolve"]["nodes"]}
    root = m["resolve"]["root"]
    seen, stack = set(), [root]
    while stack:
        i = stack.pop()
        if i in seen:
            continue
        seen.add(i)
        for d in nodes[i]["deps"]:
            if any(k["kind"] is None for k in d["dep_kinds"]):
                stack.append(d["pkg"])
    seen.discard(root)
    return seen


def licence_texts(pkg):
    d = os.path.dirname(pkg["manifest_path"])
    names = sorted(f for f in os.listdir(d) if LICENSE_FILE.match(f) and os.path.isfile(os.path.join(d, f)))
    if pkg.get("license_file"):
        lf = os.path.normpath(os.path.join(d, pkg["license_file"]))
        if os.path.isfile(lf) and os.path.basename(lf) not in names:
            names.append(os.path.relpath(lf, d))
    texts = []
    for n in names:
        try:
            t = open(os.path.join(d, n), encoding="utf-8", errors="replace").read().strip()
        except OSError:
            continue
        if t:
            texts.append((n, t[:120_000]))
    return texts


def main():
    crates = {}
    for target, platform in TARGETS.items():
        m = metadata(target)
        pk = {p["id"]: p for p in m["packages"]}
        for i in shipped(m):
            p = pk[i]
            key = (p["name"], p["version"])
            c = crates.setdefault(key, {"pkg": p, "platforms": set()})
            c["platforms"].add(platform)

    texts = {}  # hash -> {text, file, crates}
    rows = []
    for (name, version), c in sorted(crates.items()):
        p = c["pkg"]
        lic = p.get("license") or "see licence file"
        hashes = []
        for fname, t in licence_texts(p):
            h = hashlib.sha256(re.sub(r"\s+", " ", t).encode()).hexdigest()[:12]
            e = texts.setdefault(h, {"text": t, "file": fname, "crates": []})
            e["crates"].append(f"{name} {version}")
            hashes.append(h)
        rows.append({
            "name": name, "version": version, "license": lic,
            "repo": p.get("repository") or "", "platforms": sorted(c["platforms"]),
            "texts": hashes,
        })

    fonts_dir = None
    for r in rows:
        if r["name"] == "epaint_default_fonts":
            p = crates[(r["name"], r["version"])]["pkg"]
            fonts_dir = os.path.join(os.path.dirname(p["manifest_path"]), "fonts")
    fonts = [
        ("Hack Regular", "Hack-Regular.txt", "MIT License with the Bitstream Vera licence", "monospace text"),
        ("Ubuntu Light", "UFL.txt", "Ubuntu Font Licence 1.0", "proportional text"),
        ("Noto Emoji", "OFL.txt", "SIL Open Font License 1.1", "emoji"),
        ("emoji-icon-font", "emoji-icon-font-mit-license.txt", "MIT License", "icons"),
    ]
    font_texts = []
    for title, f, lic, use in fonts:
        t = open(os.path.join(fonts_dir, f), encoding="utf-8").read().strip() if fonts_dir else ""
        font_texts.append((title, lic, use, t))

    extras = [
        ("Rust standard library", "MIT OR Apache-2.0", "compiled into every build", "https://github.com/rust-lang/rust"),
        ("AppImage type 2 runtime", "MIT (includes components under other licences, among them libfuse under LGPL-2.1-or-later)",
         "the start-up code inside the Linux .AppImage file; its complete source code is available from the repository",
         "https://github.com/AppImage/type2-runtime"),
        ("Inno Setup", "Inno Setup License", "builds the Windows installer", "https://jrsoftware.org/isinfo.php"),
    ]

    write_markdown(rows, texts, font_texts, extras)
    write_html(rows, texts, font_texts, extras)
    print(f"{len(rows)} crates, {len(texts)} distinct licence texts, {len(font_texts)} fonts")


STANDARD = [
    ("MIT License (standard text; the copyright holders are the authors named in each crate's repository)", """Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE."""),
    ("Boost Software License 1.0", """Boost Software License - Version 1.0 - August 17th, 2003

Permission is hereby granted, free of charge, to any person or organization
obtaining a copy of the software and accompanying documentation covered by
this license (the "Software") to use, reproduce, display, distribute,
execute, and transmit the Software, and to prepare derivative works of the
Software, and to permit third-parties to whom the Software is furnished to
do so, all subject to the following:

The copyright notices in the Software and this entire statement, including
the above license grant, this restriction and the following disclaimer,
must be included in all copies of the Software, in whole or in part, and
all derivative works of the Software, unless such copies or derivative
works are solely in the form of machine-executable object code generated by
a source language processor.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE, TITLE AND NON-INFRINGEMENT. IN NO EVENT
SHALL THE COPYRIGHT HOLDERS OR ANYONE DISTRIBUTING THE SOFTWARE BE LIABLE
FOR ANY DAMAGES OR OTHER LIABILITY, WHETHER IN CONTRACT, TORT OR OTHERWISE,
ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
DEALINGS IN THE SOFTWARE."""),
]

HEADER = """Self World is distributed under the BSD 3-Clause License (see LICENSE). Parts of
its starting world are derived from the original Self system (see NOTICE). The
builds of Self World also contain the following third-party software. Each is
used under the licence shown; the full licence texts follow.

This file is generated by scripts/third-party-licenses.py from Cargo.lock."""


def write_markdown(rows, texts, font_texts, extras):
    o = ["# Third-party licences", "", HEADER, "", "## Fonts compiled into the program", ""]
    for title, lic, use, _ in font_texts:
        o.append(f"- **{title}** ({use}): {lic}")
    o += ["", "## Other components", ""]
    for name, lic, use, url in extras:
        o.append(f"- **{name}**: {lic}; {use}. <{url}>")
    o += ["", "## Rust crates", "", "| Crate | Version | Licence | In |", "|---|---|---|---|"]
    for r in rows:
        o.append(f"| {r['name']} | {r['version']} | {r['license']} | {', '.join(r['platforms'])} |")
    o += ["", "## Font licence texts", ""]
    for title, lic, _, t in font_texts:
        o += [f"### {title} ({lic})", "", "```", t, "```", ""]
    o += ["## Crate licence texts", ""]
    for h, e in sorted(texts.items(), key=lambda kv: kv[1]["crates"][0]):
        o += [f"### {e['file']}: used by {', '.join(e['crates'])}", "", "```", e["text"], "```", ""]
    missing = [r for r in rows if not r["texts"]]
    if missing:
        o += ["## Crates without a licence file in their package", "",
              "These crates declare their licence in their manifest but ship no licence file; the standard text of",
              "that licence (above) applies.", ""]
        for r in missing:
            o.append(f"- {r['name']} {r['version']}: {r['license']} ({r['repo']})")
        o += ["", "## Standard licence texts", ""]
        for title, t in STANDARD:
            o += [f"### {title}", "", "```", t, "```", ""]
    open(os.path.join(ROOT, "THIRD_PARTY_LICENSES.md"), "w").write("\n".join(o) + "\n")


def write_html(rows, texts, font_texts, extras):
    e = html.escape
    notice = open(os.path.join(ROOT, "NOTICE")).read()
    licence = open(os.path.join(ROOT, "LICENSE")).read()
    crate_rows = "\n".join(
        f"<tr><td>{('<a href=\"' + e(r['repo']) + '\">' + e(r['name']) + '</a>') if r['repo'] else e(r['name'])}</td>"
        f"<td>{e(r['version'])}</td><td>{e(r['license'])}</td><td>{e(', '.join(r['platforms']))}</td></tr>"
        for r in rows)
    font_list = "\n".join(f"<li><b>{e(t)}</b> ({e(u)}): {e(l)}</li>" for t, l, u, _ in font_texts)
    extra_list = "\n".join(f"<li><b>{e(n)}</b>: {e(l)}; {e(u)}. <a href=\"{e(url)}\">{e(url)}</a></li>" for n, l, u, url in extras)
    font_details = "\n".join(f"<details><summary>{e(t)}: {e(l)}</summary><pre>{e(x)}</pre></details>" for t, l, _, x in font_texts)
    crate_details = "\n".join(
        f"<details><summary>{e(v['file'])}: {e(', '.join(v['crates']))}</summary><pre>{e(v['text'])}</pre></details>"
        for _, v in sorted(texts.items(), key=lambda kv: kv[1]["crates"][0]))
    missing = [r for r in rows if not r["texts"]]
    if missing:
        crate_details += "\n<h2>Crates without a licence file</h2><p>These crates declare their licence in their manifest but ship no licence file; the standard text of that licence applies.</p><ul>"
        crate_details += "".join(f"<li>{e(r['name'])} {e(r['version'])}: {e(r['license'])}</li>" for r in missing) + "</ul>"
        crate_details += "".join(f"<details><summary>{e(t)}</summary><pre>{e(x)}</pre></details>" for t, x in STANDARD)
    page = TEMPLATE.format(
        licence=e(licence), notice=e(notice), fonts=font_list, extras=extra_list, n=len(rows),
        rows=crate_rows, font_details=font_details, crate_details=crate_details, ntexts=len(texts))
    open(os.path.join(ROOT, "web", "licenses.html"), "w").write(page)


TEMPLATE = """<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>Licences · Self World</title>
  <meta name="description" content="Self World's licence (BSD 3-Clause), the notices of the original Self system it builds on, and every third-party licence in its builds.">
  <meta name="theme-color" content="#0e1016">
  <link rel="icon" href="data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 32 32'%3E%3Crect x='12' y='2' width='18' height='18' rx='5' fill='%238b7bff' fill-opacity='.55'/%3E%3Crect x='2' y='12' width='18' height='18' rx='5' fill='%238b7bff'/%3E%3C/svg%3E" type="image/svg+xml">
  <style>
    :root {{ --bg: #0e1016; --card: #171b24; --raised: #20252f; --stroke: #2c3240; --text: #e6e9f0; --soft: #c2c7d4; --faint: #8a93a6; --accent: #8b7bff;
      --sans: -apple-system, "Segoe UI", system-ui, Helvetica, Arial, sans-serif; --mono: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace; }}
    * {{ box-sizing: border-box; }}
    html, body {{ margin: 0; background: var(--bg); color: var(--text); }}
    body {{ font: 16px/1.6 var(--sans); }}
    a {{ color: var(--accent); }}
    header.top {{ border-bottom: 1px solid var(--stroke); }}
    header.top .bar {{ display: flex; align-items: center; gap: 10px; height: 52px; padding: 0 16px; max-width: 1000px; margin: 0 auto; }}
    .brand {{ display: flex; align-items: center; gap: 10px; color: var(--text); text-decoration: none; font-weight: 700; }}
    .mark {{ position: relative; width: 22px; height: 22px; }}
    .mark i {{ position: absolute; width: 14px; height: 14px; border-radius: 4px; }}
    .mark i:first-child {{ left: 8px; top: 0; background: #8b7bff8c; }}
    .mark i:last-child {{ left: 0; top: 8px; background: var(--accent); }}
    main {{ max-width: 1000px; margin: 0 auto; padding: 28px 16px 40px; }}
    h1 {{ font-size: 30px; margin: 6px 0 6px; }}
    h2 {{ font-size: 21px; margin: 34px 0 8px; }}
    p, li {{ color: var(--soft); }}
    .lead {{ font-size: 17px; }}
    .ok {{ background: var(--card); border: 1px solid var(--stroke); border-left: 3px solid #5ed6a0; border-radius: 10px; padding: 12px 16px; }}
    pre {{ background: #0b0d12; border: 1px solid var(--stroke); border-radius: 8px; padding: 12px 14px; overflow: auto; font: 12.5px/1.5 var(--mono);
      white-space: pre-wrap; color: var(--soft); max-height: 420px; }}
    details {{ border: 1px solid var(--stroke); border-radius: 8px; margin: 6px 0; background: var(--card); }}
    summary {{ cursor: pointer; padding: 8px 12px; color: var(--soft); font-size: 14px; overflow-wrap: anywhere; }}
    details pre {{ margin: 0 10px 10px; }}
    .table {{ overflow-x: auto; border: 1px solid var(--stroke); border-radius: 10px; }}
    table {{ border-collapse: collapse; width: 100%; font-size: 14px; }}
    th, td {{ text-align: left; padding: 6px 10px; border-bottom: 1px solid var(--stroke); color: var(--soft); vertical-align: top; }}
    th {{ color: var(--text); background: var(--raised); position: sticky; top: 0; }}
    footer {{ border-top: 1px solid var(--stroke); color: var(--faint); font-size: 14px; }}
    footer .wrap {{ max-width: 1000px; margin: 0 auto; padding: 18px 16px 28px; display: flex; flex-wrap: wrap; gap: 6px 14px; }}
    footer a {{ color: var(--faint); }}
  </style>
</head>
<body>
  <header class="top"><div class="bar"><a class="brand" href="../"><span class="mark"><i></i><i></i></span>Self World</a></div></header>
  <main>
    <h1>Licences</h1>
    <p class="lead">Self World is free and open source under the <b>BSD 3-Clause License</b>. Parts of its starting world are
      derived from the original Self system, whose notices are reproduced below as its licence requires. Everything else
      in the builds is third-party software under permissive licences.</p>
    <p class="ok">Every component is under a permissive licence (MIT, Apache-2.0, BSD, Zlib, ISC, Unicode-3.0, BSL-1.0,
      CC0, SIL OFL, Ubuntu Font Licence). None of them is copyleft, so all of them can be shipped together with
      Self World under BSD 3-Clause, provided these notices travel with the program; they do, in every download, and
      in every web app Self World exports.</p>

    <h2>Self World</h2>
    <pre>{licence}</pre>

    <h2>The original Self system</h2>
    <p>Self World is an independent implementation written from scratch in Rust. It contains no source files from the
      original Self system, but some methods of its starting world (the protocol carried over from the original core
      world, in <code>world.self</code>) are adapted from the original Self source, so its copyright notices and
      disclaimers apply to those parts:</p>
    <pre>{notice}</pre>

    <h2>Fonts compiled into the program</h2>
    <ul>{fonts}</ul>
    {font_details}

    <h2>Other components</h2>
    <ul>{extras}</ul>

    <h2>Rust crates ({n})</h2>
    <p>Every crate compiled into a Self World build, with the builds it is in. The {ntexts} distinct licence texts follow
      the table.</p>
    <div class="table"><table>
      <thead><tr><th>Crate</th><th>Version</th><th>Licence</th><th>In</th></tr></thead>
      <tbody>
{rows}
      </tbody>
    </table></div>
    <h2>Licence texts</h2>
    {crate_details}
  </main>
  <footer><div class="wrap">
    <a href="../">Self World</a>
    <a href="../author/">About the author</a>
    <a href="../impressum/">Impressum</a>
    <a href="../datenschutz/">Datenschutz</a>
    <a href="../agb/">AGB</a>
    <a href="./" aria-current="page">Licences</a>
  </div></footer>
</body>
</html>
"""

if __name__ == "__main__":
    sys.exit(main())
