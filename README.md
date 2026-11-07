# Self World

**Website: <https://volland.github.io/self-lang-rs/>** — what Self is, a runnable five-minute tutorial, its history, family tree and influence on JavaScript.
**Playground: <https://volland.github.io/self-lang-rs/play/>** — a plain editor and transcript for running Self; works without WebGL, can stop runaway programs, and shares code as links.
**Book: <https://volland.github.io/self-lang-rs/book/>** — *No, I Am Your Father*, a mini book about Self, Smalltalk, prototypes, fast VMs and JavaScript, with runnable code.
**Desktop: <https://volland.github.io/self-lang-rs/try/>** — the full Self World desktop, the same Rust program compiled to WebAssembly.
**Self → JavaScript: <https://volland.github.io/self-lang-rs/js/>** — write Self, see it translated to JavaScript as you type, run it, and download it as a web app.

The web pages live in `web/`: `index.html` (home and tour), `play.html`, `book.html`, `js.html` (Self → JavaScript) and `try.html` (desktop).
`engine.js` loads the Self engine for the playground and the book, running it in a Web Worker
(`engine-worker.js`) so a program that never ends can be stopped. `npm run build` puts the site in `dist/`.

The [Self programming language](https://selflanguage.org) re-implemented in Rust, with a modern
take on Self's outliner desktop. One program runs in a web browser and as a native app on
macOS, Linux and Windows.

## What's inside

- **A Self interpreter** (`src/lexer.rs`, `src/parser.rs`, `src/vm.rs`) following the Self
  Language Reference: objects made of slots (constant `=`, assignable `<-`, parent `*`,
  arguments `:x`), unary / binary / keyword messages with Self's precedence, implicit-receiver
  sends, `resend.` and directed resends, blocks that are closures, `^` non-local returns,
  assignable parent slots (dynamic inheritance), primitives (`_IntAdd:`, `_AddSlots:`,
  `_Clone`, … with `IfFail:` variants), integers that grow into big integers, a lookup cache
  and a garbage collector.
- **A starting world written in Self** (`src/world.self`): the lobby, `defaultBehavior`,
  `traits` for numbers, booleans, strings, blocks, vectors, lists, dictionaries, sets, points,
  rectangles and mirrors, plus examples such as `bankAccount`. Much of the everyday protocol
  of the original core files is carried over under its original names: `doFirst:Middle:Last:IfEmpty:`,
  `ifNone:IfOne:IfMany:`, `reduceWith:`, `findFirst:IfPresent:IfAbsent:`, `copyFilteredBy:`,
  `union:`/`intersect:`, `/+ /- /~ /=`, `compare:IfLess:Equal:Greater:`, `printStringPrecision:`,
  `matchesPattern:`, `sendTo:With:`, `loopExit`, `untilTrue:`, `point # point` and more.
- **The desktop** (`src/app.rs`, built with egui): outliners on any object, with slots grouped
  by category, click-to-open values joined by arrows, method editors (edit and *Accept*),
  slot editing and adding, an evaluator per object, shells with history, a transcript, a slot
  finder, a language guide and an examples menu; dark and light themes.

## Download

Ready-made installers for the latest release (all releases: <https://github.com/Volland/self-lang-rs/releases>):

| System | Download | Notes |
|---|---|---|
| macOS 11+ (Apple silicon and Intel) | [SelfWorld-macOS.dmg](https://github.com/Volland/self-lang-rs/releases/latest/download/SelfWorld-macOS.dmg) | Not signed by Apple yet: the first time, right-click the app and choose **Open** (or `xattr -dr com.apple.quarantine "/Applications/Self World.app"`) |
| Windows 10/11, 64-bit | [SelfWorld-Windows-Setup.exe](https://github.com/Volland/self-lang-rs/releases/latest/download/SelfWorld-Windows-Setup.exe) · [portable .zip](https://github.com/Volland/self-lang-rs/releases/latest/download/SelfWorld-Windows.zip) | SmartScreen may warn about an unknown publisher: **More info → Run anyway** |
| Linux x86-64 | [AppImage](https://github.com/Volland/self-lang-rs/releases/latest/download/SelfWorld-Linux-x86_64.AppImage) · [.deb](https://github.com/Volland/self-lang-rs/releases/latest/download/SelfWorld-Linux-amd64.deb) · [.tar.gz](https://github.com/Volland/self-lang-rs/releases/latest/download/SelfWorld-Linux-x86_64.tar.gz) | AppImage: `chmod +x` it and run; .deb: `sudo apt install ./SelfWorld-Linux-amd64.deb` |
| Browser | <https://volland.github.io/self-lang-rs/try/> | nothing to install |

## Run it from source

| Where | How |
|---|---|
| macOS | double-click `run-mac.command`, or `cargo run --release` |
| Linux | `./run-linux.sh` (the file lists the few system packages it needs) |
| Windows | double-click `run-windows.bat`, or `cargo run --release` |
| Browser | `npm run build`, then serve `dist/` with any static web server |

All of them need Rust from <https://rustup.rs>. The web build also needs
`rustup target add wasm32-unknown-unknown` and `cargo install wasm-bindgen-cli --version 0.2.128`.
`.github/workflows/build.yml` builds and tests every platform on each push.

The installers are made by scripts you can also run yourself; each writes to `target/dist/`:

| Script | Makes |
|---|---|
| `scripts/package-mac.sh` (`--native` for this Mac only) | `Self World.app` and `SelfWorld-macOS.dmg` (universal) |
| `scripts/package-windows.ps1` | `SelfWorld-Windows-Setup.exe` (Inno Setup) and `SelfWorld-Windows.zip` |
| `scripts/package-linux.sh` | `.AppImage`, `.deb` (cargo-deb) and `.tar.gz` |

Pushing a version tag (`git tag v0.2.0 && git push origin v0.2.0`) runs
`.github/workflows/release.yml`, which builds all of them and publishes a GitHub Release.

Text mode works everywhere too:

```
self-world --repl                    # interactive shell
self-world -e '100 factorial'        # evaluate one expression
self-world my-program.self           # load and run files (add --repl to keep going)
self-world --open my-program.self    # open the desktop with files loaded
self-world --test                    # run the self-test (written in Self)
self-world --app examples/todo.self  # run a Self program's morphs as an app
self-world --js app.self             # print the program translated to JavaScript
self-world --js app.self --standalone > app.js   # runtime + world + program, for node
self-world --web-app app.self -o app.html        # one HTML file that runs it in any browser
```

## Writing and loading Self files

Write Self in any text editor and save it as a `.self` file. Load it with
**World ▸ Load a Self file…**, by dropping it on the window (desktop and browser), or from the
command line. Statements end with a period *or* at the end of a line outside brackets, exactly
like the original Self file reader, and each statement runs on its own: a failure is reported
with its line number and the rest of the file still loads.

Files from the original Self world (the `bootstrap addSlotsTo: bootstrap stub -> ... From: ...`
format) can be loaded too, including `bootstrap read: 'x' From: 'core'` sub-files on the
desktop. All 129 files of the original core world (`objects/core/allCore.self`) load — about
9,400 statements, of which around 160 fail — and core files such as `boolean`, `nil`, `block`
(with its tests), `point`, `vector` and `collection` load with no failures. Running the whole
original world afterwards needs more of the original virtual machine's primitives (mirrors,
byte vectors, processes, the file system), which this implementation doesn't have yet.

## Interfaces made of objects (Morphic)

As in the original Self and in Smalltalk, interfaces are built inside the world from ordinary
Self objects called *morphs* (`src/morphic.self`): `rectangleMorph`, `circleMorph`,
`labelMorph`, `buttonMorph`, `textFieldMorph`, `sliderMorph`, `checkboxMorph`, `rowMorph`,
`columnMorph`, `canvasMorph` and `windowMorph`, all children of the `world`. The desktop reads
their slots live every frame, so changing a slot from code or from an outliner changes the
screen at once, and user actions come back as messages (`click`, `x:`, `contents:`, `changed`,
`step`).

```self
| w. count. label |
count: 0.
label: labelMorph copyText: 'Count: 0'.
w: windowMorph copyTitle: 'Counter'.
w addMorph: label.
w addMorph: (buttonMorph copyLabel: 'Add one' Action: [
  count: count + 1.
  label text: 'Count: ', count printString ]).
w openInWorld
```

Give a morph a `step` method and send `startStepping` to animate it; override `click` to react
to clicks; right-click any morph to inspect, duplicate, stop or delete it. The **Morphs** menu
has a counter, bouncing balls, a temperature converter, a drawing canvas and a morph that
changes itself. (The browser's text-only fallback can build morphs but can't show them.)

## Apps and objects written in Self

**Apps.** A `.self` file that builds morphs is an app. Run it with only the world on screen
(no outliners) using `self-world --app examples/todo.self`, or **World ▸ Run a Self file as an
app…** on the desktop and in the browser. `world title: 'My app'` names the window, and
**Self World tools** brings the programming tools back. Morphs to build with: windows, rows and
columns, labels, buttons, text fields and text areas, lists, drop-downs, sliders, checkboxes,
progress bars, lines, canvases, rectangles and circles. Morphs react to `click`, `clickAt:`,
`keyDown:`/`keyUp:` (set `wantsKeys: true`) and `step` (after `startStepping`). The
[`examples/`](examples) folder has a to-do list (`todo.self`), a game (`catch.self`) and a
sketch pad (`sketch.self`). The installers include the examples.

**Saving objects.** **World ▸ Save my objects as a Self file…** writes everything you added to
the lobby, globals and traits (new objects, slots and edited methods) as ordinary Self source.
Load that file into any world to get them back. From code: `fileOut changes`,
`fileOut changesTo: 'my-objects.self'`, `fileOut object: x` or `x storeString`. On the desktop,
Self programs can read and write files too: `'notes.txt' asFileContents` and
`'notes.txt' setFileContentsTo: 'hello'`.

## Self to JavaScript

Self World translates Self to JavaScript (`src/jsgen.rs`). Object literals become `$.obj([...])`
with the same slots, methods become functions, every message becomes
`$.send(receiver, "selector", [arguments])` looked up through parent slots at run time, locals
and arguments become JavaScript variables, blocks become closures, and `^` inside a block returns
from its method. Integers are BigInts, so they grow as in Self. The runtime (`src/js/runtime.js`)
implements the object model and the primitives; the starting world (`world.self`,
`morphic.self`) is translated by the same compiler, so a translated program has the whole
library. The self-test passes when translated, except the few checks that need the Self compiler
at run time (`eval`, `storeString`, `fileOut`, bootstrap paths).

A **web app** is one HTML file: the runtime, the translated world and program, and a renderer that
draws morphs with the browser's own buttons, fields, lists and canvases (`src/js/morphic.js`).
In a web app, `asFileContents` and `setFileContentsTo:` use the browser's local storage, so the
to-do example keeps its tasks.

| Where | How |
|---|---|
| Website | <https://volland.github.io/self-lang-rs/js/>: live translation, **Run**, **Download web app** |
| Desktop | **World ▸ Export a Self file as a web app…** or **Translate a Self file to JavaScript…** |
| In Self | `'3 + 4' asJavaScript`, `aString asWebAppTitled: 'My app'` |
| Command line | `self-world --js`, `--js … --standalone`, `--web-app` |

## Testing

`selfTest run` (also **World ▸ Run the self-test**, or `self-world --test`) runs 86 checks
written in Self, from arithmetic and big integers to inheritance, resends, closures,
non-local returns, collections, protocol carried over from the original core world and the
original file format. `cargo test` runs the same
self-test plus engine tests in Rust, and (when Node is installed) the self-test translated to
JavaScript.

In the browser, if WebGL isn't available the page falls back to a text shell on the same engine.

## Publishing on GitHub

`./scripts/publish-github.sh` creates the public repository `self-lang-rs` on your GitHub account
(with the `gh` command line, logged in), pushes this folder and turns on GitHub Pages. From then on
`.github/workflows/pages.yml` rebuilds the WebAssembly playground on every push to `main`.

## A taste of Self

```self
lobby _AddSlots: (|
  dog = (| parent* = traits clonable.
           name <- 'Rex'.
           speak = ( name, ' says woof!' ) |)
|).
dog speak                                  "'Rex says woof!'"
(3 @ 4) + (10 @ 20)                        "13@24"
(1 to: 10 Collect: [| :i | i * i ]) sum    "385"
```

## Differences from the original Self

This is an independent implementation. Its starting world is smaller and written from
scratch; original Self source files can be loaded (see above), but saved snapshots can't, and
the original world's code doesn't fully run yet. There is no compiler to machine code, no
processes and no foreign-function interface, and its Morphic is a smaller, modern take rather
than the original UI2 library. Integers divide with `/` like the original (`7 / 2` is `3`).
`onError:` on blocks is an addition for convenience.

## Licence

BSD 3-Clause (see `LICENSE`). Self itself was created by David Ungar and Randall B. Smith; this
is an independent re-implementation and contains none of the original Self virtual machine code.
