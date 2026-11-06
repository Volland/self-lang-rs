# Self World

**Try it in your browser: <https://volland.github.io/self-lang-rs/>** — the same Rust program, compiled to WebAssembly.

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
  `traits` for numbers, booleans, strings, blocks, vectors, lists, dictionaries, points and
  mirrors, plus examples such as `bankAccount`.
- **The desktop** (`src/app.rs`, built with egui): outliners on any object, with slots grouped
  by category, click-to-open values joined by arrows, method editors (edit and *Accept*),
  slot editing and adding, an evaluator per object, shells with history, a transcript, a slot
  finder, a language guide and an examples menu; dark and light themes.

## Run it

| Where | How |
|---|---|
| macOS | double-click `run-mac.command`, or `cargo run --release` |
| Linux | `./run-linux.sh` (the file lists the few system packages it needs) |
| Windows | double-click `run-windows.bat`, or `cargo run --release` |
| Browser | `npm run build`, then serve `dist/` with any static web server |

All of them need Rust from <https://rustup.rs>. The web build also needs
`rustup target add wasm32-unknown-unknown` and `cargo install wasm-bindgen-cli --version 0.2.128`.
`scripts/package-mac.sh` makes a double-clickable `Self World.app`.
`.github/workflows/build.yml` builds every platform automatically once the folder is pushed to GitHub.

Text mode works everywhere too:

```
self-world --repl                    # interactive shell
self-world -e '100 factorial'        # evaluate one expression
self-world my-program.self           # load and run files (add --repl to keep going)
self-world --open my-program.self    # open the desktop with files loaded
self-world --test                    # run the self-test (written in Self)
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

## Testing

`selfTest run` (also **World ▸ Run the self-test**, or `self-world --test`) runs 52 checks
written in Self, from arithmetic and big integers to inheritance, resends, closures,
non-local returns, collections and the original file format. `cargo test` runs the same
self-test plus engine tests in Rust.

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
