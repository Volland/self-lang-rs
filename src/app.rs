//! The Self World desktop: a canvas of floating windows in the spirit of the
//! original Self/Morphic environment. Outliners show an object's slots;
//! clicking a value opens its outliner and draws an arrow to it, just like
//! the original. Shells evaluate Self code, the transcript collects output.

use crate::vm::{Event, SlotView, Value, Vm};
use eframe::egui::{
    self, Align, Color32, CornerRadius, FontId, Id, Key, Layout, Margin, Modifiers, Pos2, Rect, RichText, Sense,
    Shadow, Stroke, Vec2,
};
use std::collections::{HashMap, HashSet};

// ---------------------------------------------------------------- theme

#[derive(Clone, Copy)]
struct Palette {
    bg: Color32,
    grid: Color32,
    panel: Color32,
    window: Color32,
    raised: Color32,
    stroke: Color32,
    text: Color32,
    faint: Color32,
    accent: Color32,
    accent_soft: Color32,
    number: Color32,
    string: Color32,
    object: Color32,
    method: Color32,
    parent: Color32,
    error: Color32,
    ok: Color32,
    arrow: Color32,
}

const DARK: Palette = Palette {
    bg: Color32::from_rgb(0x0e, 0x10, 0x16),
    grid: Color32::from_rgb(0x1b, 0x1f, 0x2a),
    panel: Color32::from_rgb(0x13, 0x16, 0x1e),
    window: Color32::from_rgb(0x17, 0x1b, 0x24),
    raised: Color32::from_rgb(0x20, 0x25, 0x31),
    stroke: Color32::from_rgb(0x2c, 0x32, 0x40),
    text: Color32::from_rgb(0xe6, 0xe9, 0xf0),
    faint: Color32::from_rgb(0x8a, 0x93, 0xa6),
    accent: Color32::from_rgb(0x8b, 0x7b, 0xff),
    accent_soft: Color32::from_rgb(0x2a, 0x26, 0x4d),
    number: Color32::from_rgb(0xff, 0xb8, 0x6b),
    string: Color32::from_rgb(0x8f, 0xe3, 0xa1),
    object: Color32::from_rgb(0x7c, 0xc4, 0xff),
    method: Color32::from_rgb(0xd3, 0x9b, 0xff),
    parent: Color32::from_rgb(0xff, 0x8f, 0xb1),
    error: Color32::from_rgb(0xff, 0x6b, 0x6b),
    ok: Color32::from_rgb(0x5e, 0xd6, 0xa0),
    arrow: Color32::from_rgba_premultiplied(0x8b, 0x7b, 0xff, 0xb0),
};

const LIGHT: Palette = Palette {
    bg: Color32::from_rgb(0xf3, 0xf4, 0xf8),
    grid: Color32::from_rgb(0xe2, 0xe5, 0xee),
    panel: Color32::from_rgb(0xff, 0xff, 0xff),
    window: Color32::from_rgb(0xff, 0xff, 0xff),
    raised: Color32::from_rgb(0xf1, 0xf2, 0xf7),
    stroke: Color32::from_rgb(0xd9, 0xdc, 0xe6),
    text: Color32::from_rgb(0x1c, 0x1f, 0x2b),
    faint: Color32::from_rgb(0x6a, 0x71, 0x82),
    accent: Color32::from_rgb(0x5b, 0x45, 0xe0),
    accent_soft: Color32::from_rgb(0xe9, 0xe5, 0xff),
    number: Color32::from_rgb(0xb4, 0x5a, 0x00),
    string: Color32::from_rgb(0x1a, 0x7f, 0x3c),
    object: Color32::from_rgb(0x0b, 0x63, 0xc4),
    method: Color32::from_rgb(0x84, 0x2f, 0xc9),
    parent: Color32::from_rgb(0xc2, 0x2e, 0x6a),
    error: Color32::from_rgb(0xc9, 0x2a, 0x2a),
    ok: Color32::from_rgb(0x12, 0x8a, 0x55),
    arrow: Color32::from_rgba_premultiplied(0x5b, 0x45, 0xe0, 0xa0),
};

fn apply_theme(ctx: &egui::Context, dark: bool) {
    let p = if dark { DARK } else { LIGHT };
    let mut v = if dark { egui::Visuals::dark() } else { egui::Visuals::light() };
    v.panel_fill = p.panel;
    v.window_fill = p.window;
    v.window_stroke = Stroke::new(1.0, p.stroke);
    v.window_corner_radius = CornerRadius::same(12);
    v.menu_corner_radius = CornerRadius::same(10);
    v.window_shadow = Shadow { offset: [0, 10], blur: 28, spread: 0, color: Color32::from_black_alpha(if dark { 110 } else { 40 }) };
    v.popup_shadow = Shadow { offset: [0, 6], blur: 18, spread: 0, color: Color32::from_black_alpha(if dark { 100 } else { 35 }) };
    v.extreme_bg_color = if dark { Color32::from_rgb(0x0b, 0x0d, 0x12) } else { Color32::from_rgb(0xfa, 0xfb, 0xfd) };
    v.code_bg_color = p.raised;
    v.faint_bg_color = p.raised;
    v.hyperlink_color = p.accent;
    v.selection.bg_fill = p.accent.gamma_multiply(0.45);
    v.selection.stroke = Stroke::new(1.0, p.accent);
    v.override_text_color = Some(p.text);
    for w in [&mut v.widgets.inactive, &mut v.widgets.hovered, &mut v.widgets.active, &mut v.widgets.open] {
        w.corner_radius = CornerRadius::same(7);
    }
    v.widgets.inactive.bg_fill = p.raised;
    v.widgets.inactive.weak_bg_fill = p.raised;
    v.widgets.inactive.bg_stroke = Stroke::new(1.0, p.stroke);
    v.widgets.hovered.weak_bg_fill = p.accent_soft;
    v.widgets.hovered.bg_fill = p.accent_soft;
    v.widgets.hovered.bg_stroke = Stroke::new(1.0, p.accent);
    v.widgets.active.weak_bg_fill = p.accent;
    v.widgets.active.bg_fill = p.accent;
    v.widgets.noninteractive.bg_stroke = Stroke::new(1.0, p.stroke);
    ctx.set_visuals(v);
    ctx.style_mut(|s| {
        s.spacing.item_spacing = Vec2::new(8.0, 6.0);
        s.spacing.button_padding = Vec2::new(10.0, 4.0);
        s.spacing.window_margin = Margin::same(12);
        s.spacing.interact_size.y = 24.0;
        s.text_styles.insert(egui::TextStyle::Body, FontId::proportional(14.5));
        s.text_styles.insert(egui::TextStyle::Button, FontId::proportional(14.0));
        s.text_styles.insert(egui::TextStyle::Small, FontId::proportional(11.5));
        s.text_styles.insert(egui::TextStyle::Monospace, FontId::monospace(13.5));
        s.text_styles.insert(egui::TextStyle::Heading, FontId::proportional(19.0));
    });
}

// ---------------------------------------------------------------- windows

struct Outliner {
    value: Value,
    open_src: HashSet<String>,
    editing: HashMap<String, String>,
    edit_errors: HashMap<String, String>,
    data_edit: Option<(String, String)>,
    eval_text: String,
    eval_result: Option<(String, bool, Option<Value>)>,
    show_eval: bool,
    add_slot: Option<String>,
    add_error: Option<String>,
    filter: String,
}

impl Outliner {
    fn new(value: Value) -> Self {
        Outliner {
            value,
            open_src: HashSet::new(),
            editing: HashMap::new(),
            edit_errors: HashMap::new(),
            data_edit: None,
            eval_text: String::new(),
            eval_result: None,
            show_eval: false,
            add_slot: None,
            add_error: None,
            filter: String::new(),
        }
    }
}

struct ShellEntry {
    src: String,
    result: String,
    value: Option<Value>,
    is_err: bool,
    trace: Vec<String>,
    printed: String,
    ms: f64,
    /// Problems in a loaded file that didn't stop the rest from loading.
    notes: Vec<String>,
}

struct Shell {
    input: String,
    history: Vec<ShellEntry>,
    recall: Option<usize>,
}

struct Finder {
    query: String,
}

enum Kind {
    Outliner(Box<Outliner>),
    Shell(Box<Shell>),
    Transcript,
    Finder(Finder),
    Guide,
    About,
}

struct Win {
    id: u64,
    kind: Kind,
    open: bool,
    pos: Pos2,
    size: Vec2,
    force_pos: bool,
    focus: bool,
}

struct Arrow {
    from: u64,
    slot: String,
    to: u64,
}

enum Action {
    Inspect { from: Option<(u64, String)>, value: Value, near: Pos2 },
    OpenShellWith(String),
    RunInShell(String),
    /// Load a Self source file (its text, and its folder when known).
    LoadFile { name: String, text: String, dir: Option<std::path::PathBuf> },
    PickFile,
    /// Pick a Self file and run it as an app (the world only, tools hidden).
    PickApp,
    /// Save the objects added since start-up as a Self file.
    SaveObjects,
}

#[cfg(target_arch = "wasm32")]
thread_local! {
    /// Files chosen in the browser's file picker, waiting to be loaded.
    pub static PENDING_FILES: std::cell::RefCell<Vec<(String, String)>> = const { std::cell::RefCell::new(Vec::new()) };
}

pub struct SelfWorldApp {
    vm: Vm,
    wins: Vec<Win>,
    arrows: Vec<Arrow>,
    next_id: u64,
    transcript: String,
    dark: bool,
    themed: bool,
    win_rects: HashMap<u64, Rect>,
    slot_rects: HashMap<(u64, String), Rect>,
    actions: Vec<Action>,
    status: String,
    initialized_layout: bool,
    morphic: crate::morphic::Morphic,
    frame_no: u64,
    /// Native only: save a screenshot here after a short demo, then quit.
    shot_path: Option<String>,
    /// App mode: only the world and its morphs are shown, so a Self program
    /// that builds an interface runs like an ordinary app (`--app FILE.self`).
    app_mode: bool,
    /// The next file loaded starts app mode (World ▸ Run a Self file as an app).
    app_on_next_load: bool,
    /// Problems loading an app's files, shown along the bottom in app mode.
    app_errors: Vec<String>,
    /// The window title last set from `world title`.
    title_shown: String,
}

/// Apps written entirely in Self (the files in examples/), built in so they
/// run on the web too: (menu title, short name for `?app=`, file name, source).
pub const APPS: &[(&str, &str, &str, &str)] = &[
    ("A to-do list", "todo", "todo.self", include_str!("../examples/todo.self")),
    ("Catch the stars (a game)", "catch", "catch.self", include_str!("../examples/catch.self")),
    ("A sketch pad", "sketch", "sketch.self", include_str!("../examples/sketch.self")),
];

pub const EXAMPLES: &[(&str, &str)] = &[
    ("Big numbers", "100 factorial"),
    ("Points", "(3 @ 4) + (10 @ 20)"),
    (
        "Make an object",
        "lobby _AddSlots: (|\n  dog = (| parent* = traits clonable.\n          name <- 'Rex'.\n          speak = ( name, ' says woof!' ) |)\n|).\ndog speak",
    ),
    (
        "Copy a prototype",
        "| a. b |\na: bankAccount copy.\nb: bankAccount copy.\na deposit: 50.\nb deposit: 7.\na balance + b balance",
    ),
    (
        "Inherit and resend",
        "lobby _AddSlots: (|\n  animal = (| parent* = traits clonable.\n             speak = ( 'some noise' ) |)\n|).\nlobby _AddSlots: (|\n  puppy = (| parent* = animal.\n            speak = ( resend.speak, ', then wags its tail' ) |)\n|).\npuppy speak",
    ),
    (
        "Change a parent at run time",
        "| before |\nlobby _AddSlots: (|\n  quiet = (| parent* = traits clonable. talk = ( 'psst' ) |).\n  loud = (| parent* = traits clonable. talk = ( 'HELLO!' ) |)\n|).\nlobby _AddSlots: (| parrot = (| parent* <- quiet |) |).\nbefore: parrot talk.\nparrot parent: loud.\nbefore, ' then ', parrot talk",
    ),
    ("Blocks are closures", "| adder |\nadder: [| :n | [| :x | x + n ] ].\n(adder value: 10) value: 5"),
    ("Collections", "(1 to: 12 Collect: [| :i | i * i ]) select: [| :x | x even ]"),
    (
        "Return from a block",
        "lobby _AddSlots: (|\n  firstEven: v = ( v do: [| :x | x even ifTrue: [ ^ x ] ]. nil )\n|).\nfirstEven: (3 & 7 & 8 & 9) asVector",
    ),
    (
        "Print to the transcript",
        "1 to: 5 Do: [| :i |\n  (i printString, ' squared is ', i squared printString) printLine ]",
    ),
    ("Look at slots with a mirror", "(reflect: 3 @ 4) names"),
    ("Open an outliner from code", "(3 @ 4) inspect"),
    ("Catch an error", "[ 1 / 0 ] onError: [| :msg | 'Caught: ', msg ]"),
    ("Strings", "('the quick brown fox' words collect: [| :w | w capitalize ]) printString"),
    ("Dictionaries", "| d |\nd: dictionary copyRemoveAll.\nd at: 'one' Put: 1.\nd at: 'two' Put: 2.\nd"),
    ("Timing", "[ 25 fibonacci ] msecToRun"),
    (
        "Morphs: a counter window",
        "| w. count. label |\ncount: 0.\nlabel: labelMorph copyText: 'Count: 0'.\nlabel fontSize: 24.\nw: windowMorph copyTitle: 'Counter'.\nw addMorph: label.\nw addMorph: (buttonMorph copyLabel: 'Add one' Action: [\n  count: count + 1.\n  label text: 'Count: ', count printString ]).\nw addMorph: (buttonMorph copyLabel: 'Reset' Action: [\n  count: 0.\n  label text: 'Count: 0' ]).\nw openInWorld",
    ),
    (
        "Morphs: bouncing balls",
        "1 to: 5 Do: [| :i. ball |\n  ball: circleMorph copy.\n  ball color: (('tomato' & 'gold' & 'teal' & 'blue' & 'pink') asVector at: i - 1).\n  ball position: ((world width / 2) + (random integer: world width / 2) - 60) @ (random integer: world height - 80).\n  ball _AddSlots: (| dx <- 5. dy <- 3.\n    step = (\n      x: x + dx. y: y + dy.\n      ((x < 0) || [ (x + width) > world width ]) ifTrue: [ dx: dx negated ].\n      ((y < 0) || [ (y + height) > world height ]) ifTrue: [ dy: dy negated ] ) |).\n  ball dx: (random between: 2 And: 7). ball dy: (random between: 2 And: 7).\n  ball openInWorld.\n  ball startStepping ]",
    ),
    (
        "Morphs: temperature converter",
        "| w. answer |\nanswer: labelMorph copyText: '= 212.0 F'.\nw: windowMorph copyTitle: 'Celsius to Fahrenheit'.\nw addMorph: (labelMorph copyText: 'Type degrees Celsius:').\nw addMorph: ((textFieldMorph copyContents: '100') onChange: [| :text |\n  [ answer text: '= ', ((text asNumber * 9 / 5.0) + 32) printString, ' F' ]\n    onError: [ answer text: 'not a number' ] ]).\nw addMorph: answer.\nw openInWorld",
    ),
    (
        "Morphs: drawing canvas",
        "| w. canvas. size. row |\ncanvas: canvasMorph copy.\nsize: (sliderMorph copyMin: 10 Max: 80 Value: 30).\nrow: rowMorph copy.\nrow addMorph: (buttonMorph copyLabel: 'Add a circle' Action: [| c |\n  c: circleMorph copy.\n  c extent: size value @ size value.\n  c color: (('tomato' & 'gold' & 'teal' & 'blue' & 'pink') asVector at: (random integer: 5)).\n  c position: (random integer: 280) @ (random integer: 160).\n  canvas addMorph: c ]).\nrow addMorph: (buttonMorph copyLabel: 'Clear' Action: [\n  canvas morphs do: [| :m | m delete ] ]).\nw: windowMorph copyTitle: 'Drawing (drag the circles)'.\nw addMorph: row.\nw addMorph: size.\nw addMorph: canvas.\nw extent: 360 @ 330.\nw openInWorld",
    ),
    (
        "Morphs: a list to pick from",
        "| w. picked. list |\npicked: labelMorph copyText: 'Pick a planet'.\nlist: listMorph copyItems: ('Mercury' & 'Venus' & 'Earth' & 'Mars' & 'Jupiter') asVector.\nlist onSelect: [| :p | picked text: 'You picked ', p ].\nw: windowMorph copyTitle: 'Planets'.\nw addMorph: list.\nw addMorph: picked.\nw addMorph: ((dropDownMorph copyChoices: ('small' & 'large') asVector) onChange: [| :s | picked fontSize: (s = 'large' ifTrue: 28 False: 16) ]).\nw openInWorld",
    ),
    (
        "Morphs: steer with the arrow keys",
        "| ship |\nship: circleMorph copy.\nship color: 'gold'.\nship position: (world width / 2) @ (world height / 2).\nship _AddSlots: (| keyDown: k = (\n    k = 'ArrowLeft' ifTrue: [ x: x - 20 ].\n    k = 'ArrowRight' ifTrue: [ x: x + 20 ].\n    k = 'ArrowUp' ifTrue: [ y: y - 20 ].\n    k = 'ArrowDown' ifTrue: [ y: y + 20 ] ) |).\nship wantsKeys: true.\nship openInWorld.\n'Click the desktop, then use the arrow keys' printLine",
    ),
    (
        "Morphs: a morph that edits itself",
        "| box |\nbox: rectangleMorph copy.\nbox position: 300 @ 120.\nbox _AddSlots: (| clicks <- 0.\n  click = (\n    clicks: clicks + 1.\n    color: (('teal' & 'gold' & 'tomato') asVector at: clicks % 3).\n    width: width + 10 ) |).\nbox openInWorld.\nbox inspect",
    ),
];

impl SelfWorldApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let dark = cc.egui_ctx.style().visuals.dark_mode || true;
        let mut app = SelfWorldApp {
            vm: Vm::new(),
            wins: vec![],
            arrows: vec![],
            next_id: 1,
            transcript: String::new(),
            dark,
            themed: false,
            win_rects: HashMap::new(),
            slot_rects: HashMap::new(),
            actions: vec![],
            status: String::new(),
            initialized_layout: false,
            morphic: Default::default(),
            frame_no: 0,
            shot_path: None,
            app_mode: false,
            app_on_next_load: false,
            app_errors: vec![],
            title_shown: String::new(),
        };
        #[cfg(not(target_arch = "wasm32"))]
        {
            app.shot_path = std::env::var("SELF_WORLD_SCREENSHOT").ok();
            // `self-world --open FILE.self ...` loads files into the desktop;
            // `self-world --app FILE.self ...` runs them as an app (world only)
            let args: Vec<String> = std::env::args().skip(1).collect();
            app.app_mode = args.iter().any(|a| a == "--app");
            if let Some(i) = args.iter().position(|a| a == "--open" || a == "--app") {
                for path in args[i + 1..].iter().filter(|a| !a.starts_with("--")) {
                    let p = std::path::PathBuf::from(path);
                    if let Ok(text) = std::fs::read_to_string(&p) {
                        let name = p.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
                        app.actions.push(Action::LoadFile { name, text, dir: p.parent().map(|d| d.to_path_buf()) });
                    }
                }
            }
        }
        #[cfg(target_arch = "wasm32")]
        {
            // try/?app=todo opens one of the built-in apps in app mode
            let search = js_sys::eval("location.search").ok().and_then(|v| v.as_string()).unwrap_or_default();
            if let Some(want) = search.trim_start_matches('?').split('&').find_map(|kv| kv.strip_prefix("app=")) {
                if let Some((_, _, file, src)) = APPS.iter().find(|(_, short, _, _)| *short == want) {
                    app.app_mode = true;
                    app.actions.push(Action::LoadFile { name: file.to_string(), text: src.to_string(), dir: None });
                }
            }
        }
        app.transcript.push_str("Welcome to Self World.\nprintLine and transcript show: write here.\n");
        app
    }

    fn pal(&self) -> Palette {
        if self.dark {
            DARK
        } else {
            LIGHT
        }
    }

    fn add_win(&mut self, kind: Kind, pos: Pos2, size: Vec2) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        self.wins.push(Win { id, kind, open: true, pos, size, force_pos: true, focus: true });
        id
    }

    fn initial_layout(&mut self, screen: Rect) {
        let w = screen.width();
        let top = screen.top() + 12.0;
        let left = screen.left() + 12.0;
        if w < 700.0 {
            // phones: stack the essentials
            let ww = (w - 24.0).max(260.0);
            self.add_win(Kind::Shell(Box::new(Shell::new())), Pos2::new(left, top), Vec2::new(ww, 360.0));
            let lobby = Value::Ref(self.vm.sp.lobby);
            self.add_win(Kind::Outliner(Box::new(Outliner::new(lobby))), Pos2::new(left, top + 410.0), Vec2::new(ww, 320.0));
            return;
        }
        let gap = 28.0;
        let shell_w = (w * 0.40).clamp(380.0, 600.0);
        let h = screen.height();
        let shell_h = ((h - 60.0) * 0.6).clamp(300.0, 460.0);
        self.add_win(Kind::Shell(Box::new(Shell::new())), Pos2::new(left + 4.0, top + 4.0), Vec2::new(shell_w, shell_h));
        let lobby = Value::Ref(self.vm.sp.lobby);
        let ox = left + 4.0 + shell_w + gap;
        let lobby_id = self.add_win(Kind::Outliner(Box::new(Outliner::new(lobby))), Pos2::new(ox, top + 4.0), Vec2::new(300.0, 300.0));
        let th = (h - shell_h - gap - 40.0).clamp(120.0, 260.0);
        self.add_win(Kind::Transcript, Pos2::new(left + 4.0, top + 4.0 + shell_h + gap + 30.0), Vec2::new(shell_w, th));
        // the lobby's globals, linked by an arrow, as on the original desktop
        let globals = Value::Ref(self.vm.sp.globals);
        let gid = self.add_win(Kind::Outliner(Box::new(Outliner::new(globals))), Pos2::new(ox + 24.0, top + 340.0), Vec2::new(300.0, 300.0));
        self.arrows.push(Arrow { from: lobby_id, slot: "globals".into(), to: gid });
        let guide_x = ox + 340.0 + gap;
        let guide_w = screen.right() - guide_x - 16.0;
        if guide_w >= 330.0 {
            self.add_win(Kind::Guide, Pos2::new(guide_x, top + 4.0), Vec2::new(guide_w.min(460.0), (h - 60.0).min(620.0)));
        }
    }

    fn outliner_for(&self, v: Value) -> Option<u64> {
        self.wins.iter().find(|w| w.open && matches!(&w.kind, Kind::Outliner(o) if o.value.same(v))).map(|w| w.id)
    }

    fn inspect(&mut self, from: Option<(u64, String)>, value: Value, near: Pos2) {
        let target = if let Some(id) = self.outliner_for(value) {
            if let Some(w) = self.wins.iter_mut().find(|w| w.id == id) {
                w.focus = true;
            }
            id
        } else {
            let id = self.add_win(Kind::Outliner(Box::new(Outliner::new(value))), near, Vec2::new(320.0, 300.0));
            id
        };
        if let Some((fw, slot)) = from {
            if fw != target && !self.arrows.iter().any(|a| a.from == fw && a.slot == slot && a.to == target) {
                self.arrows.retain(|a| !(a.from == fw && a.slot == slot));
                self.arrows.push(Arrow { from: fw, slot, to: target });
            }
        }
    }

    fn shell_id(&mut self) -> u64 {
        if let Some(w) = self.wins.iter().find(|w| w.open && matches!(w.kind, Kind::Shell(_))) {
            return w.id;
        }
        self.add_win(Kind::Shell(Box::new(Shell::new())), Pos2::new(40.0, 80.0), Vec2::new(520.0, 420.0))
    }

    fn drain_events(&mut self, near: Pos2) -> String {
        let mut printed = String::new();
        let events: Vec<Event> = self.vm.events.drain(..).collect();
        for e in events {
            match e {
                Event::Print(s) => {
                    printed.push_str(&s);
                    self.transcript.push_str(&s);
                }
                Event::Inspect(v) => self.actions.push(Action::Inspect { from: None, value: v, near }),
                Event::ClearTranscript => self.transcript.clear(),
            }
        }
        if self.transcript.len() > 200_000 {
            let cut = self.transcript.len() - 150_000;
            let cut = (cut..self.transcript.len()).find(|i| self.transcript.is_char_boundary(*i)).unwrap_or(0);
            self.transcript.drain(..cut);
        }
        printed
    }

    fn gc_roots(&self) -> Vec<Value> {
        let mut roots = vec![];
        for w in &self.wins {
            match &w.kind {
                Kind::Outliner(o) => {
                    roots.push(o.value);
                    if let Some((_, _, Some(v))) = &o.eval_result {
                        roots.push(*v);
                    }
                }
                Kind::Shell(s) => roots.extend(s.history.iter().filter_map(|e| e.value)),
                _ => {}
            }
        }
        roots
    }

    fn maybe_gc(&mut self) {
        if self.vm.needs_gc() {
            let roots = self.gc_roots();
            let freed = self.vm.collect_garbage(&roots);
            self.status = format!("tidied memory: {freed} objects freed");
        }
    }

    fn run_shell(&mut self, shell_id: u64, src: String) {
        let receiver = Value::Ref(self.vm.sp.shell);
        let t0 = crate::vm::now_ms();
        let res = self.vm.eval_doit(&src, receiver);
        let ms = crate::vm::now_ms() - t0;
        let near = self.win_rects.get(&shell_id).map(|r| r.right_top() + Vec2::new(30.0, 20.0)).unwrap_or(Pos2::new(500.0, 100.0));
        let printed = self.drain_events(near);
        let entry = match res {
            Ok(v) => {
                let s = self.vm.print_string(v);
                self.drain_events(near);
                ShellEntry { src, result: s, value: Some(v), is_err: false, trace: vec![], printed, ms, notes: vec![] }
            }
            Err(e) => ShellEntry { src, result: e.msg, value: None, is_err: true, trace: e.trace, printed, ms, notes: vec![] },
        };
        self.status = format!("{} in {:.1} ms · {} objects", if entry.is_err { "error" } else { "done" }, ms, self.vm.object_count());
        if let Some(w) = self.wins.iter_mut().find(|w| w.id == shell_id) {
            if let Kind::Shell(s) = &mut w.kind {
                s.history.push(entry);
                if s.history.len() > 300 {
                    s.history.remove(0);
                }
                s.recall = None;
            }
        }
        self.maybe_gc();
    }

    /// Load a Self source file into the world and report it in the shell.
    fn load_file(&mut self, name: String, text: String, dir: Option<std::path::PathBuf>) {
        let shell_id = self.shell_id();
        self.vm.source_dir = dir;
        let t0 = crate::vm::now_ms();
        let res = self.vm.load_source(&text);
        let ms = crate::vm::now_ms() - t0;
        let near = self.win_rects.get(&shell_id).map(|r| r.right_top() + Vec2::new(30.0, 20.0)).unwrap_or(Pos2::new(500.0, 100.0));
        let printed = self.drain_events(near);
        let lines = text.lines().count();
        let src = format!("load {name}   ({lines} lines)");
        let entry = match res {
            Ok(rep) => {
                let failed = rep.failures.len();
                let mut result = format!(
                    "loaded {} statement{}{}",
                    rep.statements,
                    if rep.statements == 1 { "" } else { "s" },
                    if failed > 0 { format!(", {failed} failed") } else { String::new() }
                );
                if let (Some(v), 0) = (rep.last, failed) {
                    result = format!("{result} — last result: {}", self.vm.print_string(v));
                }
                self.drain_events(near);
                let mut notes: Vec<String> = rep.failures.iter().take(30).cloned().collect();
                if failed > 30 {
                    notes.push(format!("… and {} more", failed - 30));
                }
                ShellEntry { src, result, value: rep.last, is_err: false, trace: vec![], printed, ms, notes }
            }
            Err(e) => ShellEntry { src, result: e.msg, value: None, is_err: true, trace: vec![], printed, ms, notes: vec![] },
        };
        if self.app_mode {
            if entry.is_err {
                self.app_errors.push(format!("{name}: {}", entry.result));
            }
            self.app_errors.extend(entry.notes.iter().map(|n| format!("{name}: {n}")));
        }
        self.status = format!("loaded {name} in {:.0} ms · {} objects", ms, self.vm.object_count());
        if let Some(w) = self.wins.iter_mut().find(|w| w.id == shell_id) {
            w.focus = true;
            if let Kind::Shell(s) = &mut w.kind {
                s.history.push(entry);
            }
        }
        self.maybe_gc();
    }

    fn pick_file(&mut self) {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let picked = rfd::FileDialog::new()
                .set_title("Load a Self file")
                .add_filter("Self source", &["self"])
                .add_filter("All files", &["*"])
                .pick_file();
            if let Some(path) = picked {
                let name = path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
                match std::fs::read_to_string(&path) {
                    Ok(text) => {
                        let dir = path.parent().map(|p| p.to_path_buf());
                        self.actions.push(Action::LoadFile { name, text, dir });
                    }
                    Err(e) => self.status = format!("couldn't read {name}: {e}"),
                }
            }
        }
        #[cfg(target_arch = "wasm32")]
        {
            use eframe::wasm_bindgen::JsCast as _;
            if let Some(input) = web_sys::window()
                .and_then(|w| w.document())
                .and_then(|d| d.get_element_by_id("file_input"))
                .and_then(|e| e.dyn_into::<web_sys::HtmlElement>().ok())
            {
                input.click();
            }
        }
    }

    /// Files dropped on the window, or chosen in the browser's picker.
    fn incoming_files(&mut self, ctx: &egui::Context) {
        let dropped = ctx.input(|i| i.raw.dropped_files.clone());
        for f in dropped {
            let name = if !f.name.is_empty() {
                f.name.clone()
            } else {
                f.path.as_ref().and_then(|p| p.file_name()).map(|n| n.to_string_lossy().to_string()).unwrap_or("file".into())
            };
            let text = if let Some(bytes) = &f.bytes {
                Some(String::from_utf8_lossy(bytes).to_string())
            } else {
                f.path.as_ref().and_then(|p| std::fs::read_to_string(p).ok())
            };
            match text {
                Some(text) => {
                    let dir = f.path.as_ref().and_then(|p| p.parent()).map(|p| p.to_path_buf());
                    self.actions.push(Action::LoadFile { name, text, dir });
                }
                None => self.status = format!("couldn't read {name}"),
            }
        }
        #[cfg(target_arch = "wasm32")]
        {
            let pending: Vec<(String, String)> = PENDING_FILES.with(|p| p.borrow_mut().drain(..).collect());
            for (name, text) in pending {
                self.actions.push(Action::LoadFile { name, text, dir: None });
            }
        }
        if ctx.input(|i| !i.raw.hovered_files.is_empty()) {
            let p = self.pal();
            let screen = ctx.screen_rect();
            let painter = ctx.layer_painter(egui::LayerId::new(egui::Order::Foreground, Id::new("drop-hint")));
            painter.rect_filled(screen.shrink(10.0), 16.0, p.accent.gamma_multiply(0.18));
            painter.rect_stroke(screen.shrink(10.0), 16.0, Stroke::new(2.0, p.accent), egui::StrokeKind::Inside);
            painter.text(screen.center(), egui::Align2::CENTER_CENTER, "Drop a .self file to load it", FontId::proportional(24.0), p.text);
        }
    }

    /// Hand morph output to the transcript and outliners, and keep stepping
    /// morphs animated.
    fn morph_events(&mut self, ctx: &egui::Context) {
        for m in std::mem::take(&mut self.morphic.messages) {
            self.transcript.push_str(&m);
            self.status = m.trim().to_string();
            if self.app_mode {
                self.app_errors.push(m.trim().to_string());
            }
        }
        let near = ctx.screen_rect().center();
        for v in std::mem::take(&mut self.morphic.inspect) {
            self.actions.push(Action::Inspect { from: None, value: v, near });
        }
        if !self.vm.events.is_empty() {
            self.drain_events(near);
        }
        if self.frame_no % 120 == 0 {
            self.morphic.tidy(&self.vm);
        }
        if self.morphic.any_stepping {
            ctx.request_repaint_after(std::time::Duration::from_millis(30));
        }
    }

    // ------------------------------------------------------------ app mode

    /// The slim bar shown in app mode: the app's title, and a way back to the tools.
    fn app_bar(&mut self, ctx: &egui::Context) {
        let p = self.pal();
        egui::TopBottomPanel::top("app-bar")
            .frame(egui::Frame::new().fill(p.panel).inner_margin(Margin::symmetric(12, 5)).stroke(Stroke::new(1.0, p.stroke)))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    let title = if self.title_shown.is_empty() { "Self World".to_string() } else { self.title_shown.clone() };
                    ui.label(RichText::new(title).strong());
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if ui.button("🛠  Self World tools").on_hover_text("Show the shells, outliners and menus again").clicked() {
                            self.app_mode = false;
                            if self.wins.is_empty() {
                                self.initialized_layout = false;
                            }
                        }
                        let label = if self.dark { "☀" } else { "🌙" };
                        if ui.button(label).on_hover_text("Light or dark").clicked() {
                            self.dark = !self.dark;
                            self.themed = false;
                        }
                    });
                });
            });
        if !self.app_errors.is_empty() {
            egui::TopBottomPanel::bottom("app-errors")
                .frame(egui::Frame::new().fill(p.panel).inner_margin(Margin::symmetric(12, 8)).stroke(Stroke::new(1.0, p.error)))
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Problems loading the app").color(p.error).strong());
                        if ui.small_button("Dismiss").clicked() {
                            self.app_errors.clear();
                        }
                    });
                    egui::ScrollArea::vertical().max_height(120.0).show(ui, |ui| {
                        for e in &self.app_errors {
                            ui.label(RichText::new(e).monospace().small());
                        }
                    });
                });
        }
    }

    /// World ▸ Save my objects as a Self file…
    fn save_objects(&mut self) {
        let text = self.vm.file_out_changes();
        #[cfg(not(target_arch = "wasm32"))]
        {
            let picked = rfd::FileDialog::new()
                .set_title("Save my objects as a Self file")
                .set_file_name("my-objects.self")
                .add_filter("Self source", &["self"])
                .save_file();
            if let Some(path) = picked {
                self.status = match std::fs::write(&path, text) {
                    Ok(()) => format!("saved {}", path.display()),
                    Err(e) => format!("couldn't save {}: {e}", path.display()),
                };
            }
        }
        #[cfg(target_arch = "wasm32")]
        {
            // the browser saves it as a download
            let f = js_sys::Function::new_with_args(
                "name, text",
                "const a = document.createElement('a'); a.href = URL.createObjectURL(new Blob([text], {type: 'text/plain'})); a.download = name; document.body.appendChild(a); a.click(); setTimeout(() => { URL.revokeObjectURL(a.href); a.remove(); }, 1000);",
            );
            let _ = f.call2(&eframe::wasm_bindgen::JsValue::NULL, &"my-objects.self".into(), &text.into());
            self.status = "saved my-objects.self (see your downloads)".into();
        }
    }

    // ------------------------------------------------------------ top bar

    fn top_bar(&mut self, ctx: &egui::Context) {
        let p = self.pal();
        egui::TopBottomPanel::top("top")
            .frame(egui::Frame::new().fill(p.panel).inner_margin(Margin::symmetric(14, 8)).stroke(Stroke::new(1.0, p.stroke)))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    // a simple mark: two overlapping rounded shapes (an object and its parent)
                    let (r, _) = ui.allocate_exact_size(Vec2::new(26.0, 26.0), Sense::hover());
                    let painter = ui.painter();
                    painter.rect_filled(Rect::from_min_size(r.min + Vec2::new(8.0, 0.0), Vec2::splat(16.0)), 5.0, p.accent.gamma_multiply(0.55));
                    painter.rect_filled(Rect::from_min_size(r.min + Vec2::new(0.0, 9.0), Vec2::splat(16.0)), 5.0, p.accent);
                    ui.label(RichText::new("Self World").strong().size(16.5));
                    ui.add_space(10.0);
                    ui.menu_button("World", |ui| {
                        ui.set_min_width(210.0);
                        if ui.button("New shell").clicked() {
                            self.add_win(Kind::Shell(Box::new(Shell::new())), Pos2::new(60.0, 90.0), Vec2::new(520.0, 420.0));
                            ui.close_menu();
                        }
                        if ui.button("Outliner on the lobby").clicked() {
                            let lobby = Value::Ref(self.vm.sp.lobby);
                            self.actions.push(Action::Inspect { from: None, value: lobby, near: Pos2::new(420.0, 90.0) });
                            ui.close_menu();
                        }
                        if ui.button("Outliner on traits").clicked() {
                            let t = Value::Ref(self.vm.sp.traits);
                            self.actions.push(Action::Inspect { from: None, value: t, near: Pos2::new(460.0, 120.0) });
                            ui.close_menu();
                        }
                        if ui.button("Transcript").clicked() {
                            self.open_single(|k| matches!(k, Kind::Transcript), || Kind::Transcript, Vec2::new(480.0, 220.0));
                            ui.close_menu();
                        }
                        if ui.button("Load a Self file…").on_hover_text("Run a .self file; you can also drop files on the window").clicked() {
                            self.actions.push(Action::PickFile);
                            ui.close_menu();
                        }
                        if ui.button("Run a Self file as an app…").on_hover_text("Load a .self file that builds morphs and show only the world, like an ordinary app").clicked() {
                            self.actions.push(Action::PickApp);
                            ui.close_menu();
                        }
                        if ui.button("Save my objects as a Self file…").on_hover_text("Everything you added to the lobby, globals and traits, written as Self source you can load again").clicked() {
                            self.actions.push(Action::SaveObjects);
                            ui.close_menu();
                        }
                        if ui.button("Run the self-test").on_hover_text("Check that the language and world work").clicked() {
                            self.actions.push(Action::RunInShell("selfTest run".into()));
                            ui.close_menu();
                        }
                        ui.separator();
                        if ui.button("Find slots…").clicked() {
                            self.open_single(|k| matches!(k, Kind::Finder(_)), || Kind::Finder(Finder { query: String::new() }), Vec2::new(340.0, 380.0));
                            ui.close_menu();
                        }
                        ui.separator();
                        if ui.button("Tidy windows").clicked() {
                            self.tidy(ctx.screen_rect());
                            ui.close_menu();
                        }
                        if ui.button("Close all outliners").clicked() {
                            for w in &mut self.wins {
                                if matches!(w.kind, Kind::Outliner(_)) {
                                    w.open = false;
                                }
                            }
                            ui.close_menu();
                        }
                        if ui.button("Start a fresh world").clicked() {
                            self.vm = Vm::new();
                            self.morphic = Default::default();
                            self.wins.clear();
                            self.arrows.clear();
                            self.transcript = "A fresh world.\n".into();
                            self.initialized_layout = false;
                            ui.close_menu();
                        }
                    });
                    ui.menu_button("Examples", |ui| {
                        ui.set_min_width(240.0);
                        for (title, code) in EXAMPLES {
                            if ui.button(*title).on_hover_text(RichText::new(*code).monospace()).clicked() {
                                self.actions.push(Action::OpenShellWith(code.to_string()));
                                ui.close_menu();
                            }
                        }
                    });
                    ui.menu_button("Morphs", |ui| {
                        ui.set_min_width(230.0);
                        ui.label(RichText::new("Add to the world").small().color(p.faint));
                        for (title, code) in [
                            ("Rectangle", "| m | m: rectangleMorph copy. m position: (world width / 2) @ 80. m color: 'teal'. m openInWorld"),
                            ("Circle", "| m | m: circleMorph copy. m position: (world width / 2) @ 200. m color: 'tomato'. m openInWorld"),
                            ("Label", "| m | m: labelMorph copyText: 'Hello, world'. m position: (world width / 2) @ 300. m fontSize: 28. m openInWorld"),
                            ("Window with a button", "| w | w: windowMorph copyTitle: 'A new window'. w addMorph: (buttonMorph copyLabel: 'Say hello' Action: [ 'Hello from a morph' printLine ]). w openInWorld"),
                        ] {
                            if ui.button(title).on_hover_text(RichText::new(code).monospace()).clicked() {
                                self.actions.push(Action::RunInShell(code.to_string()));
                                ui.close_menu();
                            }
                        }
                        ui.separator();
                        ui.label(RichText::new("Little apps written in Self").small().color(p.faint));
                        for (title, code) in EXAMPLES.iter().filter(|(t, _)| t.starts_with("Morphs:")) {
                            if ui.button(title.trim_start_matches("Morphs: ")).on_hover_text(RichText::new(*code).monospace()).clicked() {
                                self.actions.push(Action::OpenShellWith(code.to_string()));
                                ui.close_menu();
                            }
                        }
                        ui.separator();
                        ui.label(RichText::new("Apps written in Self (run as an app)").small().color(p.faint));
                        for (title, _, file, src) in APPS {
                            if ui.button(*title).on_hover_text(format!("examples/{file}")).clicked() {
                                self.app_on_next_load = true;
                                self.actions.push(Action::LoadFile { name: file.to_string(), text: src.to_string(), dir: None });
                                ui.close_menu();
                            }
                        }
                        ui.separator();
                        if ui.button("Outliner on the world").clicked() {
                            let lobby = Value::Ref(self.vm.sp.lobby);
                            if let Ok(w) = self.vm.ui_send(lobby, "world", vec![]) {
                                self.actions.push(Action::Inspect { from: None, value: w, near: Pos2::new(500.0, 120.0) });
                            }
                            ui.close_menu();
                        }
                        if ui.button("Show only the world (app mode)").on_hover_text("Hide the tools; the bar at the top brings them back").clicked() {
                            self.app_mode = true;
                            ui.close_menu();
                        }
                        if ui.button("Stop all stepping").clicked() {
                            self.actions.push(Action::RunInShell("world morphs do: [| :m | m stopStepping ]".into()));
                            ui.close_menu();
                        }
                        if ui.button("Clear the world").clicked() {
                            self.actions.push(Action::RunInShell("world clear".into()));
                            ui.close_menu();
                        }
                    });
                    ui.menu_button("Help", |ui| {
                        ui.set_min_width(200.0);
                        if ui.button("Language guide").clicked() {
                            self.open_single(|k| matches!(k, Kind::Guide), || Kind::Guide, Vec2::new(440.0, 520.0));
                            ui.close_menu();
                        }
                        ui.hyperlink_to("Source code on GitHub", "https://github.com/Volland/self-lang-rs");
                        if ui.button("About Self World").clicked() {
                            self.open_single(|k| matches!(k, Kind::About), || Kind::About, Vec2::new(380.0, 260.0));
                            ui.close_menu();
                        }
                    });
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        let label = if self.dark { "☀  Light" } else { "🌙  Dark" };
                        if ui.button(label).clicked() {
                            self.dark = !self.dark;
                            self.themed = false;
                        }
                        if !self.status.is_empty() && ui.available_width() > 260.0 {
                            ui.label(RichText::new(&self.status).color(p.faint).small());
                        }
                    });
                });
            });
    }

    fn open_single(&mut self, is: impl Fn(&Kind) -> bool, make: impl FnOnce() -> Kind, size: Vec2) {
        if let Some(w) = self.wins.iter_mut().find(|w| w.open && is(&w.kind)) {
            w.focus = true;
            return;
        }
        let n = self.wins.len() as f32;
        self.add_win(make(), Pos2::new(120.0 + 18.0 * (n % 8.0), 90.0 + 18.0 * (n % 8.0)), size);
    }

    fn tidy(&mut self, screen: Rect) {
        let mut x = screen.left() + 16.0;
        let mut y = screen.top() + 60.0;
        let mut row_h: f32 = 0.0;
        for w in self.wins.iter_mut().filter(|w| w.open) {
            let r = self.win_rects.get(&w.id).copied().unwrap_or(Rect::from_min_size(w.pos, w.size));
            if x + r.width() > screen.right() - 16.0 && x > screen.left() + 16.0 {
                x = screen.left() + 16.0;
                y += row_h + 24.0;
                row_h = 0.0;
            }
            w.pos = Pos2::new(x, y);
            w.force_pos = true;
            x += r.width() + 24.0;
            row_h = row_h.max(r.height().min(500.0));
        }
    }

    // ------------------------------------------------------------ canvas & arrows

    fn canvas(&mut self, ctx: &egui::Context) {
        let p = self.pal();
        egui::CentralPanel::default().frame(egui::Frame::new().fill(p.bg)).show(ctx, |ui| {
            let rect = ui.max_rect();
            let painter = ui.painter();
            let step = 28.0;
            let mut x = if self.app_mode { rect.right() } else { rect.left() - (rect.left() % step) };
            while x < rect.right() {
                let mut y = rect.top() - (rect.top() % step);
                while y < rect.bottom() {
                    painter.circle_filled(Pos2::new(x, y), 1.1, p.grid);
                    y += step;
                }
                x += step;
            }
            let colors = morph_colors(&p);
            self.morphic.draw_world(ui, &mut self.vm, &colors);
            if !self.app_mode && self.wins.iter().all(|w| !w.open) {
                ui.centered_and_justified(|ui| {
                    ui.label(RichText::new("The world is empty. Open a shell or an outliner from the World menu.").color(p.faint));
                });
            }
        });
    }

    // ------------------------------------------------------------ window rendering

    /// Arrows from slots to the outliners they opened, drawn above the
    /// windows like the original Self desktop.
    fn arrows(&mut self, ctx: &egui::Context) {
        let p = self.pal();
        let painter = ctx.layer_painter(egui::LayerId::new(egui::Order::Foreground, Id::new("self-arrows")));
        for a in &self.arrows {
            let (Some(from), Some(to), Some(sw)) = (
                self.slot_rects.get(&(a.from, a.slot.clone())),
                self.win_rects.get(&a.to),
                self.win_rects.get(&a.from),
            ) else {
                continue;
            };
            let y = from.center().y.clamp(sw.top() + 30.0, sw.bottom() - 8.0);
            let ty = (to.top() + 22.0).min(to.bottom());
            let overlap_x = to.left() < sw.right() && to.right() > sw.left();
            let (start, end, c1, c2, dir) = if overlap_x {
                // loop around the right-hand side
                let x = sw.right().max(to.right()) + 60.0;
                let start = Pos2::new(sw.right(), y);
                let end = Pos2::new(to.right(), ty);
                (start, end, Pos2::new(x, y), Pos2::new(x, ty), -1.0)
            } else if to.center().x >= sw.center().x {
                let start = Pos2::new(sw.right(), y);
                let end = Pos2::new(to.left(), ty);
                let dx = ((end.x - start.x).abs() * 0.5).max(40.0);
                (start, end, Pos2::new(start.x + dx, y), Pos2::new(end.x - dx, ty), 1.0)
            } else {
                let start = Pos2::new(sw.left(), y);
                let end = Pos2::new(to.right(), ty);
                let dx = ((end.x - start.x).abs() * 0.5).max(40.0);
                (start, end, Pos2::new(start.x - dx, y), Pos2::new(end.x + dx, ty), -1.0)
            };
            let curve = egui::epaint::CubicBezierShape::from_points_stroke(
                [start, c1, c2, end],
                false,
                Color32::TRANSPARENT,
                Stroke::new(2.0, p.arrow),
            );
            painter.add(curve);
            painter.circle_filled(start, 3.5, p.accent);
            let tip = end;
            let a1 = Pos2::new(tip.x - 9.0 * dir, tip.y - 5.0);
            let a2 = Pos2::new(tip.x - 9.0 * dir, tip.y + 5.0);
            painter.add(egui::Shape::convex_polygon(vec![tip, a1, a2], p.accent, Stroke::NONE));
        }
    }

    fn windows(&mut self, ctx: &egui::Context) {
        self.slot_rects.clear();
        let p = self.pal();
        let mut wins = std::mem::take(&mut self.wins);
        let screen = ctx.screen_rect();
        for w in wins.iter_mut() {
            if !w.open {
                continue;
            }
            let title = self.window_title(w);
            let mut open = true;
            let mut window = egui::Window::new(RichText::new(title).strong())
                .id(Id::new(("win", w.id)))
                .open(&mut open)
                .default_size(w.size)
                .resizable(true)
                .collapsible(true)
                .constrain_to(screen)
                .frame(egui::Frame::window(&ctx.style()).inner_margin(Margin::same(12)));
            window = if w.force_pos { window.current_pos(w.pos) } else { window.default_pos(w.pos) };
            w.force_pos = false;
            match w.kind {
                Kind::Outliner(_) => window = window.max_width(520.0),
                Kind::Guide => window = window.default_width(w.size.x).max_width(w.size.x.max(380.0)),
                Kind::Shell(_) | Kind::Transcript => window = window.default_width(w.size.x),
                _ => {}
            }
            let wid = w.id;
            let resp = window.show(ctx, |ui| match &mut w.kind {
                Kind::Outliner(o) => self.outliner_ui(ui, wid, o, p),
                Kind::Shell(s) => self.shell_ui(ui, wid, s, p),
                Kind::Transcript => self.transcript_ui(ui, p),
                Kind::Finder(f) => self.finder_ui(ui, wid, f, p),
                Kind::Guide => self.guide_ui(ui, p),
                Kind::About => self.about_ui(ui, p),
            });
            if let Some(r) = resp {
                self.win_rects.insert(w.id, r.response.rect);
                if w.focus {
                    ctx.move_to_top(r.response.layer_id);
                    w.focus = false;
                }
            }
            if !open {
                w.open = false;
            }
        }
        wins.retain(|w| w.open);
        let alive: HashSet<u64> = wins.iter().map(|w| w.id).collect();
        self.arrows.retain(|a| alive.contains(&a.from) && alive.contains(&a.to));
        self.win_rects.retain(|k, _| alive.contains(k));
        // keep windows opened during this frame
        let mut added = std::mem::take(&mut self.wins);
        wins.append(&mut added);
        self.wins = wins;
    }

    fn window_title(&self, w: &Win) -> String {
        match &w.kind {
            Kind::Outliner(o) => {
                if self.vm.is_alive(o.value) {
                    truncate(&self.vm.describe(o.value), 40)
                } else {
                    "a freed object".into()
                }
            }
            Kind::Shell(_) => "Shell".into(),
            Kind::Transcript => "Transcript".into(),
            Kind::Finder(_) => "Find slots".into(),
            Kind::Guide => "The Self language".into(),
            Kind::About => "About Self World".into(),
        }
    }

    fn value_text(&mut self, v: Value, p: Palette) -> RichText {
        let kind = self.vm.kind_label(v);
        let s = match kind {
            "method" => "method".to_string(),
            "block" => "a block".to_string(),
            _ => self.vm.print_string(v),
        };
        let s = truncate(&s.replace('\n', " "), 48);
        let color = match kind {
            "integer" | "float" => p.number,
            "string" => p.string,
            "method" | "block" => p.method,
            _ => p.object,
        };
        RichText::new(s).monospace().color(color)
    }

    fn outliner_ui(&mut self, ui: &mut egui::Ui, wid: u64, o: &mut Outliner, p: Palette) {
        let v = o.value;
        if !self.vm.is_alive(v) {
            ui.label(RichText::new("This object no longer exists.").color(p.faint));
            return;
        }
        ui.set_min_width(280.0);
        // header: kind and printString
        ui.horizontal_wrapped(|ui| {
            let kind = self.vm.kind_label(v);
            badge(ui, kind, p.accent_soft, p.accent);
            let ps = self.vm.print_string(v);
            let d = self.vm.describe(v);
            if ps != d {
                ui.label(RichText::new(truncate(&ps, 60)).monospace().color(p.faint));
            }
        });
        if let Some(c) = self.vm.comment_of(v) {
            ui.label(RichText::new(c).italics().color(p.faint).size(12.5));
        }
        if let Some(src) = self.vm.source_of(v) {
            egui::Frame::new().fill(p.raised).corner_radius(8).inner_margin(Margin::same(8)).show(ui, |ui| {
                ui.label(RichText::new(src).monospace().size(12.5));
            });
        }
        ui.add_space(2.0);
        ui.horizontal(|ui| {
            if ui.small_button("+ Slot").on_hover_text("Add slots to this object").clicked() {
                o.add_slot = Some(o.add_slot.take().unwrap_or_default());
            }
            if ui.small_button("Evaluate").on_hover_text("Run Self code with this object as self").clicked() {
                o.show_eval = !o.show_eval;
            }
            if ui.small_button("Copy").on_hover_text("Make a copy of this object and open it").clicked() {
                let r = self.vm.eval_doit("copy", v);
                if let Ok(c) = r {
                    let near = self.win_rects.get(&wid).map(|r| r.right_top() + Vec2::new(24.0, 30.0)).unwrap_or(Pos2::new(300.0, 200.0));
                    self.actions.push(Action::Inspect { from: None, value: c, near });
                }
            }
            if ui.small_button("Shell ›").on_hover_text("Put this object's name into the shell").clicked() {
                let d = self.vm.describe(v);
                self.actions.push(Action::OpenShellWith(d));
            }
        });
        let mut add_clicked = false;
        let mut cancel_clicked = false;
        if let Some(text) = &mut o.add_slot {
            ui.add_space(4.0);
            ui.label(RichText::new("New slots, e.g.  x <- 3.  area = ( x * x )").small().color(p.faint));
            ui.add(egui::TextEdit::multiline(text).code_editor().desired_rows(2).desired_width(ui.available_width()));
            ui.horizontal(|ui| {
                add_clicked = ui.button("Add").clicked();
                cancel_clicked = ui.button("Cancel").clicked();
            });
        }
        if add_clicked {
            let text = o.add_slot.clone().unwrap_or_default();
            match self.vm.add_slots_from_source(v, &text) {
                Ok(()) => {
                    o.add_error = None;
                    o.add_slot = None;
                }
                Err(e) => o.add_error = Some(e),
            }
        }
        if cancel_clicked {
            o.add_slot = None;
            o.add_error = None;
        }
        if o.add_slot.is_some() {
            if let Some(e) = &o.add_error {
                ui.label(RichText::new(e).color(p.error).small());
            }
        }
        ui.separator();

        let slots = self.vm.slots_of(v);
        if slots.len() > 12 {
            ui.add(egui::TextEdit::singleline(&mut o.filter).hint_text("Filter slots…").desired_width(ui.available_width()));
        }
        let filter = o.filter.to_lowercase();
        let mut cats: Vec<String> = vec![];
        for s in &slots {
            if !cats.contains(&s.category) {
                cats.push(s.category.clone());
            }
        }
        cats.sort_by_key(|c| (!c.is_empty(), c.clone()));
        let items = self.vm.vector_items(v);
        let max_h = (ui.ctx().screen_rect().height() * 0.62).max(220.0);
        egui::ScrollArea::vertical().id_salt(("slots", wid)).max_height(max_h).auto_shrink([false, true]).show(ui, |ui| {
            for cat in &cats {
                let in_cat: Vec<&SlotView> = slots
                    .iter()
                    .filter(|s| &s.category == cat && (filter.is_empty() || s.name.to_lowercase().contains(&filter)))
                    .collect();
                if in_cat.is_empty() {
                    continue;
                }
                if cat.is_empty() {
                    for s in in_cat {
                        self.slot_row(ui, wid, o, s, p);
                    }
                } else {
                    let open_default = cats.len() <= 3 || !filter.is_empty();
                    egui::CollapsingHeader::new(RichText::new(format!("{cat}  ·  {}", in_cat.len())).color(p.faint).strong())
                        .id_salt(("cat", wid, cat))
                        .default_open(open_default)
                        .show(ui, |ui| {
                            for s in in_cat {
                                self.slot_row(ui, wid, o, s, p);
                            }
                        });
                }
            }
            if slots.is_empty() && items.is_none() {
                ui.label(RichText::new("No slots.").color(p.faint));
            }
            if let Some(items) = items {
                egui::CollapsingHeader::new(RichText::new(format!("elements  ·  {}", items.len())).color(p.faint).strong())
                    .id_salt(("elems", wid))
                    .default_open(true)
                    .show(ui, |ui| {
                        for (i, e) in items.iter().enumerate().take(200) {
                            ui.horizontal(|ui| {
                                ui.label(RichText::new(format!("{i}")).monospace().color(p.faint));
                                let t = self.value_text(*e, p);
                                let r = ui.add(egui::Button::new(t).frame(false));
                                let key = format!("[{i}]");
                                self.slot_rects.insert((wid, key.clone()), r.rect);
                                if r.clicked() {
                                    let near = r.rect.right_top() + Vec2::new(60.0, -10.0);
                                    self.actions.push(Action::Inspect { from: Some((wid, key)), value: *e, near });
                                }
                            });
                        }
                        if items.len() > 200 {
                            ui.label(RichText::new(format!("… and {} more", items.len() - 200)).color(p.faint));
                        }
                    });
            }
        });

        if o.show_eval {
            ui.separator();
            ui.label(RichText::new("Evaluate with this object as self").small().color(p.faint));
            let eid = Id::new(("eval", wid));
            let run = ui.ctx().memory(|m| m.has_focus(eid)) && ui.input_mut(|i| i.consume_key(Modifiers::COMMAND, Key::Enter));
            ui.add(egui::TextEdit::multiline(&mut o.eval_text).id(eid).code_editor().desired_rows(2).desired_width(ui.available_width()).hint_text("e.g. printString"));
            let mut do_it = run;
            let mut get_it = false;
            ui.horizontal(|ui| {
                if ui.button("Do it").on_hover_text("Ctrl/⌘ + Enter").clicked() {
                    do_it = true;
                }
                if ui.button("Get it").on_hover_text("Run and open the result").clicked() {
                    do_it = true;
                    get_it = true;
                }
            });
            if do_it && !o.eval_text.trim().is_empty() {
                let r = self.vm.eval_doit(&o.eval_text, v);
                let near = self.win_rects.get(&wid).map(|r| r.right_top() + Vec2::new(24.0, 40.0)).unwrap_or(Pos2::new(300.0, 200.0));
                self.drain_events(near);
                o.eval_result = Some(match r {
                    Ok(val) => {
                        if get_it {
                            self.actions.push(Action::Inspect { from: None, value: val, near });
                        }
                        (self.vm.print_string(val), false, Some(val))
                    }
                    Err(e) => (e.msg, true, None),
                });
            }
            if let Some((text, is_err, val)) = &o.eval_result {
                let color = if *is_err { p.error } else { p.ok };
                let r = ui.add(egui::Label::new(RichText::new(truncate(text, 400)).monospace().color(color)).sense(Sense::click()));
                if let Some(val) = val {
                    if r.on_hover_text("Click to open").clicked() {
                        let near = self.win_rects.get(&wid).map(|r| r.right_top() + Vec2::new(24.0, 40.0)).unwrap_or(Pos2::new(300.0, 200.0));
                        self.actions.push(Action::Inspect { from: None, value: *val, near });
                    }
                }
            }
        }
    }

    fn slot_row(&mut self, ui: &mut egui::Ui, wid: u64, o: &mut Outliner, s: &SlotView, p: Palette) {
        let (icon, icon_color, tip) = if s.parent {
            ("▲", p.parent, "parent slot: messages not found here are looked up there")
        } else if s.is_method {
            ("ƒ", p.method, "method slot")
        } else if s.arg {
            (":", p.faint, "argument slot")
        } else if s.assignable {
            ("←", p.ok, "assignable data slot")
        } else {
            ("=", p.faint, "constant slot")
        };
        let row = ui.horizontal(|ui| {
            ui.label(RichText::new(icon).color(icon_color).monospace()).on_hover_text(tip);
            let name = if s.parent { format!("{}*", s.name) } else { s.name.clone() };
            if s.is_method {
                let open = o.open_src.contains(&s.name);
                let r = ui.add(egui::Button::new(RichText::new(&name).monospace().color(p.method)).frame(false));
                if r.on_hover_text(if open { "Hide the code" } else { "Show and edit the code" }).clicked() {
                    if open {
                        o.open_src.remove(&s.name);
                        o.editing.remove(&s.name);
                    } else {
                        o.open_src.insert(s.name.clone());
                        if let Some(d) = self.vm.method_declaration(&s.name, s.value) {
                            o.editing.insert(s.name.clone(), d);
                        }
                    }
                }
                None
            } else {
                ui.label(RichText::new(&name).monospace());
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if s.assignable && !s.virtual_slot && ui.small_button("edit").on_hover_text("Change this value").clicked() {
                        let cur = self.vm.print_string(s.value);
                        o.data_edit = Some((s.name.clone(), cur));
                    }
                    let t = self.value_text(s.value, p);
                    let r = ui.add(egui::Button::new(t).frame(false).truncate()).on_hover_text("Open an outliner on this value");
                    if r.clicked() {
                        let near = r.rect.right_top() + Vec2::new(70.0, -20.0);
                        self.actions.push(Action::Inspect { from: Some((wid, s.name.clone())), value: s.value, near });
                    }
                    Some(r.rect)
                })
                .inner
            }
        });
        let rect = row.inner.unwrap_or(row.response.rect);
        self.slot_rects.insert((wid, s.name.clone()), rect);
        row.response.context_menu(|ui| {
            if ui.button("Open value").clicked() {
                self.actions.push(Action::Inspect { from: Some((wid, s.name.clone())), value: s.value, near: rect.right_top() + Vec2::new(60.0, 0.0) });
                ui.close_menu();
            }
            if !s.virtual_slot && ui.button("Remove slot").clicked() {
                self.vm.remove_slot(o.value, &s.name);
                ui.close_menu();
            }
        });

        // inline data editor
        if let Some((name, text)) = &mut o.data_edit {
            if *name == s.name {
                let mut done = false;
                ui.horizontal(|ui| {
                    let r = ui.add(egui::TextEdit::singleline(text).code_editor().desired_width(ui.available_width() - 60.0));
                    r.request_focus();
                    let enter = r.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter));
                    if ui.small_button("Set").clicked() || enter {
                        match self.vm.eval_doit(text, o.value) {
                            Ok(nv) => {
                                self.vm.set_slot_value(o.value, &s.name, nv);
                                o.edit_errors.remove(&s.name);
                            }
                            Err(e) => {
                                o.edit_errors.insert(s.name.clone(), e.msg);
                            }
                        }
                        done = true;
                    }
                    if ui.input(|i| i.key_pressed(Key::Escape)) {
                        done = true;
                    }
                });
                if done {
                    o.data_edit = None;
                }
            }
        }
        if let Some(e) = o.edit_errors.get(&s.name) {
            ui.label(RichText::new(e).color(p.error).small());
        }

        // method source editor
        if s.is_method && o.open_src.contains(&s.name) {
            let text = o.editing.entry(s.name.clone()).or_insert_with(|| self.vm.method_declaration(&s.name, s.value).unwrap_or_default());
            egui::Frame::new().fill(p.raised).corner_radius(8).inner_margin(Margin::same(6)).show(ui, |ui| {
                ui.add(egui::TextEdit::multiline(text).code_editor().desired_rows(2).desired_width(ui.available_width()).frame(false));
            });
            let mut accept = false;
            let mut run = false;
            ui.horizontal(|ui| {
                if ui.small_button("Accept").on_hover_text("Replace the method with this code").clicked() {
                    accept = true;
                }
                if ui.small_button("Revert").clicked() {
                    if let Some(d) = self.vm.method_declaration(&s.name, s.value) {
                        *text = d;
                    }
                    o.edit_errors.remove(&s.name);
                }
                let unary = s.name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_');
                if unary && ui.small_button("Run").on_hover_text("Send this message to the object").clicked() {
                    run = true;
                }
            });
            if accept {
                let decl = text.clone();
                match self.vm.add_slots_from_source(o.value, &decl) {
                    Ok(()) => {
                        o.edit_errors.remove(&s.name);
                    }
                    Err(e) => {
                        o.edit_errors.insert(s.name.clone(), e);
                    }
                }
            }
            if run {
                let r = self.vm.eval_doit(&s.name, o.value);
                let near = rect.right_top() + Vec2::new(60.0, 0.0);
                self.drain_events(near);
                o.show_eval = true;
                o.eval_text = s.name.clone();
                o.eval_result = Some(match r {
                    Ok(val) => (self.vm.print_string(val), false, Some(val)),
                    Err(e) => (e.msg, true, None),
                });
            }
        }
    }

    fn shell_ui(&mut self, ui: &mut egui::Ui, wid: u64, s: &mut Shell, p: Palette) {
        ui.set_min_width(300.0);
        let input_h = 120.0;
        let avail = ui.available_height();
        let hist_h = (avail - input_h - 40.0).max(120.0);
        egui::ScrollArea::vertical().id_salt(("shell", wid)).max_height(hist_h).auto_shrink([false, false]).stick_to_bottom(true).show(ui, |ui| {
            if s.history.is_empty() {
                ui.label(RichText::new("Type Self code below and press Enter (Shift + Enter for a new line). Drop a .self file on the window to load it.").color(p.faint));
                ui.add_space(6.0);
                ui.horizontal_wrapped(|ui| {
                    for ex in ["3 + 4", "100 factorial", "(3 @ 4) + (1 @ 1)", "'hello' reverse", "lobby inspect", "(1 & 2 & 3) asVector", "selfTest run"] {
                        if ui.button(RichText::new(ex).monospace().size(12.5)).clicked() {
                            self.actions.push(Action::RunInShell(ex.to_string()));
                        }
                    }
                });
            }
            let mut rerun: Option<String> = None;
            for (i, e) in s.history.iter().enumerate() {
                egui::Frame::new().inner_margin(Margin::symmetric(4, 3)).show(ui, |ui| {
                    ui.horizontal_top(|ui| {
                        ui.label(RichText::new("›").color(p.accent).monospace());
                        let r = ui.add(egui::Label::new(mono_job(ui, &e.src, p.text, 13.5)).wrap().sense(Sense::click()));
                        if r.on_hover_text("Click to edit again").clicked() {
                            rerun = Some(e.src.clone());
                        }
                    });
                    if !e.printed.is_empty() {
                        ui.add(egui::Label::new(mono_job(ui, &truncate(e.printed.trim_end(), 2000), p.faint, 12.5)).wrap());
                    }
                    ui.horizontal_top(|ui| {
                        ui.add_space(14.0);
                        if e.is_err {
                            ui.vertical(|ui| {
                                ui.add(egui::Label::new(RichText::new(format!("⚠ {}", e.result)).color(p.error)).wrap());
                                for t in &e.trace {
                                    ui.label(RichText::new(format!("   in {t}")).monospace().color(p.faint).size(12.0));
                                }
                            });
                        } else {
                            let r = ui.add(
                                egui::Label::new(mono_job(ui, &truncate(&e.result, 3000), p.ok, 13.5)).wrap().sense(Sense::click()),
                            );
                            if let Some(v) = e.value {
                                if r.on_hover_text("Click to open an outliner on the result").clicked() {
                                    let near = self.win_rects.get(&wid).map(|r| r.right_top() + Vec2::new(30.0, 20.0)).unwrap_or(Pos2::new(400.0, 120.0));
                                    self.actions.push(Action::Inspect { from: None, value: v, near });
                                }
                            }
                        }
                    });
                    for n in &e.notes {
                        ui.horizontal(|ui| {
                            ui.add_space(14.0);
                            ui.add(egui::Label::new(mono_job(ui, &format!("⚠ {n}"), p.number, 12.0)).wrap());
                        });
                    }
                    if !e.is_err {
                        ui.horizontal(|ui| {
                            ui.add_space(14.0);
                            ui.label(RichText::new(format!("{:.1} ms", e.ms)).small().color(p.faint));
                        });
                    }
                });
                if i + 1 < s.history.len() {
                    ui.add(egui::Separator::default().spacing(2.0));
                }
            }
            if let Some(src) = rerun {
                s.input = src;
            }
        });
        ui.separator();
        let iid = Id::new(("shell-input", wid));
        let focused = ui.ctx().memory(|m| m.has_focus(iid));
        let mut submit = false;
        if focused {
            submit = ui.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Enter) || i.consume_key(Modifiers::COMMAND, Key::Enter));
            let single_line = !s.input.contains('\n');
            if single_line && !s.history.is_empty() {
                if ui.input_mut(|i| i.consume_key(Modifiers::NONE, Key::ArrowUp)) {
                    let n = s.history.len();
                    let idx = match s.recall {
                        Some(k) if k > 0 => k - 1,
                        Some(k) => k,
                        None => n - 1,
                    };
                    s.recall = Some(idx);
                    s.input = s.history[idx].src.clone();
                }
                if ui.input_mut(|i| i.consume_key(Modifiers::NONE, Key::ArrowDown)) {
                    if let Some(k) = s.recall {
                        if k + 1 < s.history.len() {
                            s.recall = Some(k + 1);
                            s.input = s.history[k + 1].src.clone();
                        } else {
                            s.recall = None;
                            s.input.clear();
                        }
                    }
                }
            }
        }
        let te = egui::TextEdit::multiline(&mut s.input)
            .id(iid)
            .code_editor()
            .desired_rows(3)
            .desired_width(ui.available_width())
            .hint_text("Self code, e.g.  (3 @ 4) + (1 @ 1)");
        ui.add_sized(Vec2::new(ui.available_width(), 64.0), te);
        let mut get_it = false;
        ui.horizontal(|ui| {
            if ui.add(egui::Button::new(RichText::new("Run").strong().color(Color32::WHITE)).fill(p.accent)).clicked() {
                submit = true;
            }
            if ui.button("Get it").on_hover_text("Run and open an outliner on the result").clicked() {
                submit = true;
                get_it = true;
            }
            if ui.button("Clear").clicked() {
                s.history.clear();
            }
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.label(RichText::new("self = shell (inherits the lobby)").small().color(p.faint));
            });
        });
        if submit && !s.input.trim().is_empty() {
            let src = std::mem::take(&mut s.input);
            self.actions.push(Action::RunInShell(format!("{}{src}", if get_it { "\u{1}" } else { "" })));
            ui.ctx().memory_mut(|m| m.request_focus(iid));
        }
        let _ = wid;
    }

    fn transcript_ui(&mut self, ui: &mut egui::Ui, p: Palette) {
        ui.set_min_width(260.0);
        ui.horizontal(|ui| {
            ui.label(RichText::new("Output of printLine, print and transcript show:").small().color(p.faint));
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if ui.small_button("Clear").clicked() {
                    self.transcript.clear();
                }
            });
        });
        egui::ScrollArea::vertical().id_salt("transcript").auto_shrink([false, false]).stick_to_bottom(true).show(ui, |ui| {
            ui.label(RichText::new(&self.transcript).monospace().size(13.0));
        });
    }

    fn finder_ui(&mut self, ui: &mut egui::Ui, wid: u64, f: &mut Finder, p: Palette) {
        ui.set_min_width(280.0);
        ui.add(egui::TextEdit::singleline(&mut f.query).hint_text("Slot name, e.g. factorial or at:").desired_width(ui.available_width()));
        if f.query.trim().len() < 2 {
            ui.label(RichText::new("Type at least two letters to search every named object.").color(p.faint));
            return;
        }
        let results = self.vm.implementors(f.query.trim());
        ui.label(RichText::new(format!("{} slots", results.len())).small().color(p.faint));
        egui::ScrollArea::vertical().id_salt(("find", wid)).auto_shrink([false, false]).show(ui, |ui| {
            for (holder, owner, slot) in results {
                ui.horizontal(|ui| {
                    let r = ui.add(egui::Button::new(RichText::new(&slot).monospace().color(p.method)).frame(false));
                    ui.label(RichText::new(format!("in {owner}")).color(p.faint));
                    let key = format!("find:{owner}:{slot}");
                    self.slot_rects.insert((wid, key.clone()), r.rect);
                    if r.clicked() {
                        let near = r.rect.right_top() + Vec2::new(80.0, 0.0);
                        self.actions.push(Action::Inspect { from: Some((wid, key)), value: holder, near });
                    }
                });
            }
        });
    }

    fn guide_ui(&mut self, ui: &mut egui::Ui, p: Palette) {
        ui.set_min_width(300.0);
        egui::ScrollArea::vertical().id_salt("guide").auto_shrink([false, false]).show(ui, |ui| {
            let section = |ui: &mut egui::Ui, title: &str| {
                ui.add_space(6.0);
                ui.label(RichText::new(title).strong().size(15.5).color(p.accent));
            };
            let try_it = |ui: &mut egui::Ui, code: &str, actions: &mut Vec<Action>| {
                ui.horizontal(|ui| {
                    egui::Frame::new().fill(p.raised).corner_radius(6).inner_margin(Margin::symmetric(8, 4)).show(ui, |ui| {
                        ui.label(RichText::new(code).monospace().size(12.5));
                    });
                    if ui.small_button("Try").clicked() {
                        actions.push(Action::OpenShellWith(code.to_string()));
                    }
                });
            };
            let mut acts = vec![];
            ui.label("Self is a prototype-based object language: there are no classes, only objects made of slots. You make new objects by copying existing ones, and objects share behaviour by inheriting from parent objects.");
            section(ui, "Objects and slots");
            ui.label("An object is written between ( and ), with its slots between bars:");
            try_it(ui, "(| x <- 3. y = 4. sum = ( x + y ) |) sum", &mut acts);
            ui.label("x <- 3 is assignable (it also makes x:), y = 4 is constant, and sum is a method. A slot named with * is a parent.");
            section(ui, "Messages");
            ui.label("Unary messages bind tightest, then binary operators (left to right), then keyword messages:");
            try_it(ui, "3 + 4 factorial max: 20", &mut acts);
            ui.label("Keyword messages with several parts capitalise the later parts: at: 1 Put: 2. A message without a receiver goes to self.");
            section(ui, "Blocks");
            ui.label("Blocks are closures in square brackets. Arguments start with a colon:");
            try_it(ui, "[| :a. :b | a * b ] value: 6 With: 7", &mut acts);
            ui.label("^ inside a block returns from the method that made the block.");
            section(ui, "Prototypes and inheritance");
            try_it(ui, "| p | p: point copy. p x: 10. p + (1 @ 2)", &mut acts);
            ui.label("resend.foo calls the inherited version of foo, like super in other languages. Parent slots can be assignable, so an object can change what it inherits from while running.");
            section(ui, "Adding to the world");
            try_it(ui, "lobby _AddSlots: (| hi = 'hello' |). hi", &mut acts);
            ui.label("Messages starting with _ are primitives built into the virtual machine.");
            section(ui, "Interfaces made of objects (morphs)");
            ui.label("As in the original Self and Smalltalk, what you see on the desktop can be ordinary objects called morphs. Their slots (x, y, width, color, text...) are read live, so changing a slot from code or an outliner changes the picture at once.");
            try_it(ui, "(circleMorph copy color: 'tomato') openInWorld", &mut acts);
            try_it(ui, "(windowMorph copyTitle: 'Hi') openInWorld", &mut acts);
            ui.label("Give a morph a step method and send it startStepping to animate it; override click (or clickAt: for the point) to react to clicks, and set wantsKeys to true to get keyDown: and keyUp:. Right-click any morph to inspect, duplicate or delete it. The Morphs menu has more.");
            ui.label("Morphs to build with: labelMorph, buttonMorph, textFieldMorph, textAreaMorph, sliderMorph, checkboxMorph, dropDownMorph, listMorph, progressMorph, separatorMorph, rowMorph, columnMorph, windowMorph, and canvasMorph with rectangleMorph, circleMorph and lineMorph inside.");
            try_it(ui, "| w | w: windowMorph copyTitle: 'Pick one'. w addMorph: ((listMorph copyItems: ('red' & 'green' & 'blue') asVector) onSelect: [| :c | c printLine ]). w openInWorld", &mut acts);
            section(ui, "Apps written in Self");
            ui.label("A .self file that builds morphs is an app. World ▸ Run a Self file as an app… (or self-world --app file.self) shows only the world and its morphs; set world title: to name the window. The examples folder has a to-do list, a game and a sketch pad.");
            section(ui, "Saving your objects");
            ui.label("World ▸ Save my objects as a Self file… writes everything you added to the lobby, globals and traits as Self source; load the file again in any world. From code: fileOut changes, fileOut object: x, or x storeString.");
            try_it(ui, "fileOut changes", &mut acts);
            section(ui, "Your own files, and testing");
            ui.label("Write Self in any text editor, save it as a .self file, then use World ▸ Load a Self file… or drop it on the window. Statements end with a period or at the end of a line, as in the original Self files, so files from the original Self world can be loaded too.");
            try_it(ui, "selfTest run", &mut acts);
            ui.label("The self-test is itself written in Self: open an outliner on selfTest to read or extend its checks.");
            section(ui, "Using the desktop");
            ui.label("• Click any value in an outliner to open that object; an arrow shows where it came from.");
            ui.label("• Click a method (ƒ) to read or edit its code, then Accept.");
            ui.label("• ✎ changes an assignable slot; right-click a slot to remove it.");
            ui.label("• Evaluate runs code with the outliner's object as self.");
            ui.label("• Drag windows by their title; World ▸ Tidy windows lines them up.");
            self.actions.extend(acts);
        });
    }

    fn about_ui(&mut self, ui: &mut egui::Ui, p: Palette) {
        ui.set_min_width(300.0);
        ui.label(RichText::new("Self World").strong().size(18.0));
        ui.label("An implementation of the Self programming language written in Rust, with a modern take on Self's outliner desktop. It runs in the browser and as a native app on macOS, Linux and Windows.");
        ui.add_space(6.0);
        ui.label(RichText::new("Self was created by David Ungar and Randall B. Smith. This is an independent re-implementation; it follows the Self Language Reference but uses its own small starting world rather than the original world files.").color(p.faint));
        ui.add_space(6.0);
        ui.label(RichText::new(format!("{} objects in this world", self.vm.object_count())).small().color(p.faint));
    }

    fn handle_actions(&mut self) {
        let actions = std::mem::take(&mut self.actions);
        for a in actions {
            match a {
                Action::Inspect { from, value, near } => self.inspect(from, value, near),
                Action::OpenShellWith(code) => {
                    let id = self.shell_id();
                    if let Some(w) = self.wins.iter_mut().find(|w| w.id == id) {
                        w.focus = true;
                        if let Kind::Shell(s) = &mut w.kind {
                            s.input = code;
                        }
                    }
                }
                Action::LoadFile { name, text, dir } => {
                    if std::mem::take(&mut self.app_on_next_load) {
                        self.app_mode = true;
                        self.app_errors.clear();
                    }
                    self.load_file(name, text, dir)
                }
                Action::PickFile => self.pick_file(),
                Action::PickApp => {
                    self.app_on_next_load = true;
                    self.pick_file();
                }
                Action::SaveObjects => self.save_objects(),
                Action::RunInShell(code) => {
                    let id = self.shell_id();
                    let (get_it, code) = match code.strip_prefix('\u{1}') {
                        Some(c) => (true, c.to_string()),
                        None => (false, code),
                    };
                    self.run_shell(id, code);
                    if get_it {
                        let v = self.wins.iter().find(|w| w.id == id).and_then(|w| match &w.kind {
                            Kind::Shell(s) => s.history.last().and_then(|e| e.value),
                            _ => None,
                        });
                        if let Some(v) = v {
                            let near = self.win_rects.get(&id).map(|r| r.right_top() + Vec2::new(30.0, 20.0)).unwrap_or(Pos2::new(400.0, 120.0));
                            self.inspect(None, v, near);
                        }
                    }
                }
            }
        }
    }
}

impl SelfWorldApp {
    /// Drives a short scripted demo, then saves a screenshot and closes.
    fn screenshot_demo(&mut self, ctx: &egui::Context) {
        let demo = std::env::var("SELF_WORLD_DEMO").unwrap_or_default();
        ctx.request_repaint();
        if self.frame_no == 2 && demo == "morphs" {
            for w in &mut self.wins {
                if matches!(w.kind, Kind::Guide | Kind::Outliner(_)) {
                    w.open = false;
                }
            }
        }
        if self.frame_no == 30 && demo == "morphs" {
            let code = "| d. c |\n\
d: world morphs detect: [| :m | (m morphKind = 'window') && [ m title = 'Drawing (drag the circles)' ] ].\n\
1 to: 6 Do: [| :i | d morphs first morphs first click ].\n\
c: world morphs detect: [| :m | (m morphKind = 'window') && [ m title = 'Counter' ] ].\n\
1 to: 3 Do: [| :i | (c morphs at: 1) click ].\n\
c position: 640 @ 520";
            self.actions.push(Action::RunInShell(code.into()));
        }
        if self.frame_no == 3 && demo == "morphs" {
            for (t, code) in EXAMPLES {
                if t.starts_with("Morphs:") && !t.contains("edits itself") {
                    self.actions.push(Action::RunInShell(code.to_string()));
                }
            }
        }
        if self.frame_no == 3 && demo != "plain" && demo != "morphs" {
            for code in ["100 factorial", "(3 @ 4) + (10 @ 20)", "(1 to: 12 Collect: [| :i | i * i ]) select: [| :x | x even ]", "3 foo"] {
                self.actions.push(Action::RunInShell(code.to_string()));
            }
            let lobby_win = self.wins.iter().find(|w| matches!(&w.kind, Kind::Outliner(_))).map(|w| w.id);
            if let Some(lw) = lobby_win {
                if let Ok(p) = self.vm.eval_doit("point", Value::Ref(self.vm.sp.lobby)) {
                    let near = self.win_rects.get(&lw).map(|r| r.left_bottom() + Vec2::new(40.0, 30.0)).unwrap_or(Pos2::new(700.0, 480.0));
                    self.actions.push(Action::Inspect { from: Some((lw, "point".into())), value: p, near });
                }
            }
        }
        if self.frame_no == 6 && demo != "plain" && demo != "morphs" {
            // open the parent of point and one method's code
            let pw = self.wins.iter().find(|w| matches!(&w.kind, Kind::Outliner(o) if self.vm.describe(o.value) == "point")).map(|w| w.id);
            if let Some(pw) = pw {
                if let Ok(t) = self.vm.eval_doit("traits point", Value::Ref(self.vm.sp.lobby)) {
                    let near = self.win_rects.get(&pw).map(|r| r.right_top() + Vec2::new(50.0, 0.0)).unwrap_or(Pos2::new(900.0, 480.0));
                    self.actions.push(Action::Inspect { from: Some((pw, "parent".into())), value: t, near });
                }
            }
        }
        if self.frame_no == 9 && demo != "plain" && demo != "morphs" {
            for w in &mut self.wins {
                if let Kind::Outliner(o) = &mut w.kind {
                    if self.vm.describe(o.value) == "traits point" {
                        o.open_src.insert("printString".into());
                    }
                }
            }
        }
        if self.frame_no == if demo == "morphs" { 90 } else { 40 } {
            ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(Default::default()));
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let shots: Vec<std::sync::Arc<egui::ColorImage>> = ctx.input(|i| {
                i.raw
                    .events
                    .iter()
                    .filter_map(|e| if let egui::Event::Screenshot { image, .. } = e { Some(image.clone()) } else { None })
                    .collect()
            });
            if let Some(img) = shots.first() {
                let rgba: Vec<u8> = img.pixels.iter().flat_map(|c| c.to_array()).collect();
                let png = crate::png::encode_rgba(img.size[0] as u32, img.size[1] as u32, &rgba);
                if let Some(p) = &self.shot_path {
                    let _ = std::fs::write(p, png);
                }
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        }
        if self.frame_no > 400 {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }
}

impl Shell {
    fn new() -> Self {
        Shell { input: String::new(), history: vec![], recall: None }
    }
}

impl eframe::App for SelfWorldApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if !self.themed {
            apply_theme(ctx, self.dark);
            self.themed = true;
        }
        if self.app_mode {
            self.app_bar(ctx);
        } else {
            self.top_bar(ctx);
        }
        if !self.initialized_layout {
            if !self.app_mode {
                let r = ctx.available_rect();
                self.initial_layout(r);
            }
            self.initialized_layout = true;
        }
        self.morphic.keys(ctx, &mut self.vm);
        self.morphic.step(&mut self.vm);
        self.canvas(ctx);
        let colors = morph_colors(&self.pal());
        self.morphic.show_windows(ctx, &mut self.vm, &colors);
        if !self.app_mode {
            self.windows(ctx);
            self.arrows(ctx);
        }
        if self.frame_no % 4 == 1 {
            let t = crate::morphic::Morphic::world_title(&mut self.vm).unwrap_or_else(|| "Self World".into());
            if t != self.title_shown {
                ctx.send_viewport_cmd(egui::ViewportCommand::Title(t.clone()));
                self.title_shown = t;
            }
        }
        self.morph_events(ctx);
        self.incoming_files(ctx);
        self.handle_actions();
        self.frame_no += 1;
        if self.shot_path.is_some() {
            self.screenshot_demo(ctx);
        }
    }
}

/// Monospace text that wraps anywhere, so long results (like big numbers)
/// never stretch their window.
fn mono_job(ui: &egui::Ui, text: &str, color: Color32, size: f32) -> egui::text::LayoutJob {
    let mut job = egui::text::LayoutJob::simple(text.to_string(), FontId::monospace(size), color, ui.available_width().max(60.0));
    job.wrap.break_anywhere = true;
    job
}

fn morph_colors(p: &Palette) -> crate::morphic::Colors {
    crate::morphic::Colors {
        accent: p.accent,
        text: p.text,
        faint: p.faint,
        panel: p.window,
        background: p.raised,
        error: p.error,
    }
}

fn badge(ui: &mut egui::Ui, text: &str, bg: Color32, fg: Color32) {
    egui::Frame::new().fill(bg).corner_radius(20).inner_margin(Margin::symmetric(8, 1)).show(ui, |ui| {
        ui.label(RichText::new(text).small().color(fg).strong());
    });
}

fn truncate(s: &str, n: usize) -> String {
    if s.chars().count() <= n {
        s.to_string()
    } else {
        let t: String = s.chars().take(n.saturating_sub(1)).collect();
        format!("{t}…")
    }
}
