I rebuilt the Self programming language in Rust — and you can try it in your browser.

Most developers have never written a line of Self, yet they use its ideas every day.

Self was created in 1986 by David Ungar and Randall B. Smith at Xerox PARC, then developed at Stanford and Sun. It took Smalltalk's "everything is an object" and went further: no classes at all. Just objects made of slots, new objects made by copying existing ones, and behaviour shared by delegating to parent objects.

Why it matters:

🧬 Prototypes → JavaScript. Brendan Eich has named Self as one of the main inspirations for JavaScript's prototype-based objects. Every `__proto__` and `Object.create` is a Self idea.

⚡ Fast dynamic languages. To make such a dynamic language fast, the Self team invented adaptive compilation, polymorphic inline caches and on-the-fly deoptimization. Those ideas travelled through Strongtalk into Java's HotSpot VM and, via Lars Bak, into Google's V8 — the engine behind Chrome and Node.js.

🖱️ Live, direct-manipulation UIs. Self's Morphic let you grab any object on screen, look inside it and change it while it runs. Morphic later went on to Squeak and Pharo Smalltalk.

So Self isn't the "father of every language" — but it's one of the quiet ancestors of the languages and runtimes we use most.

What I built — Self World:
• A Self interpreter in Rust: slots, parents, resends, closures, non-local returns, big integers
• An outliner desktop where every object can be opened, read and edited live
• Morphic: windows, buttons, sliders and animations built from Self objects
• It can even load the original Self source files
• Runs natively on macOS, Linux and Windows, and in the browser via WebAssembly

▶️ Try it: https://volland.github.io/self-lang-rs/
💻 Code: https://github.com/Volland/self-lang-rs

Type `(circleMorph copy color: 'tomato') openInWorld` and watch an object appear.

#ProgrammingLanguages #Rust #WebAssembly #JavaScript #Smalltalk #ComputerScience #OpenSource
