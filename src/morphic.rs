//! Draws the Self world's morphs on the desktop.
//!
//! Morphs are plain Self objects (see morphic.self). Every frame their slots
//! are read by sending messages (`morphKind`, `x`, `text`...), so a change
//! made from code or an outliner shows at once. User actions are sent back
//! as messages too: dragging sends `x:`/`y:`, a click sends `click`, editing
//! a field sends `contents:` then `changed`, and stepping morphs get `step`.

use crate::vm::{ObjId, Value, Vm};
use eframe::egui::{self, Align2, Color32, CornerRadius, FontId, Id, Margin, Pos2, Rect, RichText, Sense, Stroke, Vec2};
use std::collections::HashMap;

/// Most morphs visited per frame, so a runaway world can't freeze the desktop.
const MAX_MORPHS: usize = 1500;

#[derive(Clone, Copy)]
pub struct Colors {
    pub accent: Color32,
    pub text: Color32,
    pub faint: Color32,
    pub panel: Color32,
    pub background: Color32,
    pub error: Color32,
}

#[derive(Default)]
pub struct Morphic {
    /// Where each window morph was last seen, to tell user moves from code moves.
    win_pos: HashMap<ObjId, Pos2>,
    text_buf: HashMap<ObjId, String>,
    drag_rest: HashMap<ObjId, Vec2>,
    last_step_ms: f64,
    /// Morphs the person asked to inspect.
    pub inspect: Vec<Value>,
    /// Errors from clicks, edits and stepping, for the transcript.
    pub messages: Vec<String>,
    pub any_stepping: bool,
}

// ------------------------------------------------------------ reading slots

fn get(vm: &mut Vm, m: Value, sel: &str) -> Option<Value> {
    vm.ui_send(m, sel, vec![]).ok()
}

fn num(v: Option<Value>) -> Option<f32> {
    match v? {
        Value::Int(i) => Some(i as f32),
        Value::Float(f) => Some(f as f32),
        _ => None,
    }
}

fn getf(vm: &mut Vm, m: Value, sel: &str, default: f32) -> f32 {
    num(get(vm, m, sel)).filter(|f| f.is_finite()).unwrap_or(default)
}

fn gets(vm: &mut Vm, m: Value, sel: &str) -> String {
    match get(vm, m, sel) {
        Some(v) => match vm.string_of(v) {
            Some(s) => s.to_string(),
            None => vm.print_string(v),
        },
        None => String::new(),
    }
}

fn getb(vm: &mut Vm, m: Value, sel: &str) -> bool {
    let t = vm.boolean(true);
    get(vm, m, sel).map(|v| v.same(t)).unwrap_or(false)
}

fn kids(vm: &mut Vm, m: Value) -> Vec<Value> {
    get(vm, m, "morphs").and_then(|v| vm.vector_items(v)).unwrap_or_default()
}

fn whole(f: f32) -> Value {
    Value::Int(f.round() as i64)
}

fn id_of(m: Value) -> ObjId {
    m.id().unwrap_or(0)
}

pub fn parse_color(s: &str, c: &Colors) -> Option<Color32> {
    let s = s.trim().to_lowercase();
    let named = match s.as_str() {
        "accent" => c.accent,
        "text" => c.text,
        "faint" => c.faint,
        "panel" => c.panel,
        "background" => c.background,
        "none" | "transparent" => Color32::TRANSPARENT,
        "red" => Color32::from_rgb(0xef, 0x44, 0x44),
        "tomato" => Color32::from_rgb(0xff, 0x63, 0x47),
        "orange" => Color32::from_rgb(0xf9, 0x73, 0x16),
        "yellow" => Color32::from_rgb(0xfa, 0xcc, 0x15),
        "gold" => Color32::from_rgb(0xf5, 0xb0, 0x1a),
        "green" => Color32::from_rgb(0x22, 0xc5, 0x5e),
        "teal" => Color32::from_rgb(0x14, 0xb8, 0xa6),
        "cyan" => Color32::from_rgb(0x06, 0xb6, 0xd4),
        "blue" => Color32::from_rgb(0x3b, 0x82, 0xf6),
        "navy" => Color32::from_rgb(0x1e, 0x3a, 0x8a),
        "purple" => Color32::from_rgb(0xa8, 0x55, 0xf7),
        "magenta" => Color32::from_rgb(0xd9, 0x46, 0xef),
        "pink" => Color32::from_rgb(0xec, 0x48, 0x99),
        "brown" => Color32::from_rgb(0x92, 0x40, 0x0e),
        "gray" | "grey" => Color32::from_rgb(0x8a, 0x93, 0xa6),
        "black" => Color32::BLACK,
        "white" => Color32::WHITE,
        _ => {
            let h = s.strip_prefix('#')?;
            let hex = |t: &str| u8::from_str_radix(t, 16).ok();
            return match h.len() {
                3 => {
                    let d: Vec<u8> = h.chars().filter_map(|ch| hex(&ch.to_string())).map(|v| v * 17).collect();
                    (d.len() == 3).then(|| Color32::from_rgb(d[0], d[1], d[2]))
                }
                6 => Some(Color32::from_rgb(hex(&h[0..2])?, hex(&h[2..4])?, hex(&h[4..6])?)),
                8 => Some(Color32::from_rgba_unmultiplied(hex(&h[0..2])?, hex(&h[2..4])?, hex(&h[4..6])?, hex(&h[6..8])?)),
                _ => None,
            };
        }
    };
    Some(named)
}

impl Morphic {
    fn color(&self, vm: &mut Vm, m: Value, c: &Colors, default: Color32) -> Color32 {
        let s = gets(vm, m, "color");
        parse_color(&s, c).unwrap_or(default)
    }

    fn report(&mut self, what: &str, err: crate::vm::SelfError) {
        let msg = format!("⚠ {what}: {}\n", err.msg);
        if !self.messages.contains(&msg) {
            self.messages.push(msg);
        }
    }

    fn send(&mut self, vm: &mut Vm, m: Value, sel: &str, args: Vec<Value>) {
        if let Err(e) = vm.ui_send(m, sel, args) {
            let d = vm.describe(m);
            self.report(&format!("{d} {sel}"), e);
        }
    }

    fn world(vm: &mut Vm) -> Option<Value> {
        let lobby = Value::Ref(vm.sp.lobby);
        get(vm, lobby, "world")
    }

    /// Every morph in the world, depth first (bounded).
    fn all_morphs(vm: &mut Vm, world: Value) -> Vec<Value> {
        let mut out = vec![];
        let mut stack = kids(vm, world);
        stack.reverse();
        while let Some(m) = stack.pop() {
            if out.len() >= MAX_MORPHS || out.iter().any(|o: &Value| o.same(m)) {
                continue;
            }
            out.push(m);
            let mut k = kids(vm, m);
            k.reverse();
            stack.extend(k);
        }
        out
    }

    /// Send `step` to stepping morphs about 30 times a second.
    pub fn step(&mut self, vm: &mut Vm) {
        let now = crate::vm::now_ms();
        let Some(world) = Self::world(vm) else { return };
        if now - self.last_step_ms < 33.0 {
            return;
        }
        self.last_step_ms = now;
        let all = Self::all_morphs(vm, world);
        self.any_stepping = false;
        for m in all {
            if getb(vm, m, "isStepping") {
                self.any_stepping = true;
                if let Err(e) = vm.ui_send(m, "step", vec![]) {
                    let d = vm.describe(m);
                    self.report(&format!("{d} step (stepping stopped)"), e);
                    let f = vm.boolean(false);
                    let _ = vm.ui_send(m, "isStepping:", vec![f]);
                }
            }
        }
    }

    /// Free-floating morphs, drawn on the desktop behind the windows.
    pub fn draw_world(&mut self, ui: &mut egui::Ui, vm: &mut Vm, c: &Colors) {
        let Some(world) = Self::world(vm) else { return };
        let area = ui.max_rect();
        let (w, h) = (getf(vm, world, "width", 0.0), getf(vm, world, "height", 0.0));
        if (w - area.width()).abs() > 0.5 || (h - area.height()).abs() > 0.5 {
            let _ = vm.ui_send(world, "width:", vec![whole(area.width())]);
            let _ = vm.ui_send(world, "height:", vec![whole(area.height())]);
        }
        for m in kids(vm, world) {
            let kind = gets(vm, m, "morphKind");
            match kind.as_str() {
                "window" => {}
                "rectangle" | "circle" | "label" | "line" => self.draw_shape(ui, vm, area.min, m, c, 0),
                _ => {
                    // a widget placed directly in the world floats at its position
                    let pos = area.min + Vec2::new(getf(vm, m, "x", 0.0), getf(vm, m, "y", 0.0));
                    let ctx = ui.ctx().clone();
                    egui::Area::new(Id::new(("morph-area", id_of(m)))).fixed_pos(pos).show(&ctx, |ui| {
                        egui::Frame::window(ui.style()).inner_margin(Margin::same(8)).show(ui, |ui| {
                            self.widget(ui, vm, m, c, 0);
                        });
                    });
                }
            }
        }
    }

    fn draw_shape(&mut self, ui: &mut egui::Ui, vm: &mut Vm, origin: Pos2, m: Value, c: &Colors, depth: usize) {
        if depth > 12 {
            return;
        }
        let kind = gets(vm, m, "morphKind");
        let (x, y) = (getf(vm, m, "x", 0.0), getf(vm, m, "y", 0.0));
        let (raw_w, raw_h) = (getf(vm, m, "width", 40.0), getf(vm, m, "height", 40.0));
        let (w, h) = (raw_w.max(1.0), raw_h.max(1.0));
        let start = origin + Vec2::new(x, y);
        let line_w = getf(vm, m, "lineWidth", 3.0).clamp(0.5, 60.0);
        let rect = if kind == "line" {
            Rect::from_two_pos(start, start + Vec2::new(raw_w, raw_h)).expand(line_w / 2.0 + 4.0)
        } else {
            Rect::from_min_size(start, Vec2::new(w, h))
        };
        let painter = ui.painter().clone();
        let col = self.color(vm, m, c, c.accent);
        let id = Id::new(("morph", id_of(m)));
        let resp = ui.interact(rect, id, Sense::click_and_drag());
        let border = self.border(vm, m, c);
        match kind.as_str() {
            "circle" => {
                painter.circle_filled(rect.center(), w.min(h) / 2.0, col);
                if let Some(b) = border {
                    painter.circle_stroke(rect.center(), w.min(h) / 2.0 - b.width / 2.0, b);
                }
                if resp.hovered() {
                    painter.circle_stroke(rect.center(), w.min(h) / 2.0 + 1.5, Stroke::new(1.5, c.text));
                }
            }
            "line" => {
                let col = self.color(vm, m, c, c.text);
                painter.line_segment([start, start + Vec2::new(raw_w, raw_h)], Stroke::new(line_w, col));
                if resp.hovered() {
                    painter.circle_stroke(start, line_w + 2.0, Stroke::new(1.0, c.faint));
                }
            }
            "label" => {
                let text = gets(vm, m, "text");
                let size = getf(vm, m, "fontSize", 16.0).clamp(6.0, 200.0);
                let col = self.color(vm, m, c, c.text);
                painter.text(rect.min, Align2::LEFT_TOP, text, FontId::proportional(size), col);
            }
            "rectangle" => {
                let radius = CornerRadius::same(num(get(vm, m, "cornerRadius")).unwrap_or(6.0).clamp(0.0, 255.0) as u8);
                painter.rect_filled(rect, radius, col);
                if let Some(b) = border {
                    painter.rect_stroke(rect, radius, b, egui::StrokeKind::Inside);
                }
                if resp.hovered() {
                    painter.rect_stroke(rect, radius, Stroke::new(1.5, c.text), egui::StrokeKind::Outside);
                }
            }
            _ => {
                painter.rect_stroke(rect, CornerRadius::same(4), Stroke::new(1.0, c.faint), egui::StrokeKind::Inside);
                let d = vm.describe(m);
                painter.text(rect.left_top() + Vec2::splat(4.0), Align2::LEFT_TOP, d, FontId::proportional(12.0), c.faint);
            }
        }
        if resp.dragged() {
            let rest = self.drag_rest.entry(id_of(m)).or_default();
            *rest += resp.drag_delta();
            let (dx, dy) = (rest.x.trunc(), rest.y.trunc());
            *rest -= Vec2::new(dx, dy);
            if dx != 0.0 || dy != 0.0 {
                let _ = vm.ui_send(m, "x:", vec![whole(x + dx)]);
                let _ = vm.ui_send(m, "y:", vec![whole(y + dy)]);
            }
        }
        if resp.clicked() {
            self.click_at(vm, m, &resp, rect.min);
        }
        self.menu(&resp, vm, m);
        if kind == "line" {
            return;
        }
        for k in kids(vm, m) {
            self.draw_shape(ui, vm, rect.min, k, c, depth + 1);
        }
    }

    /// `borderWidth` and `borderColor`, when a morph has them.
    fn border(&mut self, vm: &mut Vm, m: Value, c: &Colors) -> Option<Stroke> {
        let w = num(get(vm, m, "borderWidth")).filter(|w| *w > 0.0)?;
        let col = get(vm, m, "borderColor")
            .and_then(|v| vm.string_of(v))
            .and_then(|s| parse_color(&s, c))
            .unwrap_or(c.text);
        Some(Stroke::new(w.min(40.0), col))
    }

    /// A click becomes `clickAtX:Y:` with the point inside the morph (its
    /// default sends `clickAt:`, then `click`).
    fn click_at(&mut self, vm: &mut Vm, m: Value, resp: &egui::Response, origin: Pos2) {
        let p = resp.interact_pointer_pos().unwrap_or(origin) - origin;
        self.send(vm, m, "clickAtX:Y:", vec![whole(p.x), whole(p.y)]);
    }

    /// Keys go to every morph (and the world) whose wantsKeys is true, as
    /// `keyDown:` and `keyUp:` with names like 'A', 'Space' or 'ArrowLeft' -
    /// unless a text field or shell is being typed in.
    pub fn keys(&mut self, ctx: &egui::Context, vm: &mut Vm) {
        if ctx.wants_keyboard_input() {
            return;
        }
        let events: Vec<(String, bool)> = ctx.input(|i| {
            i.events
                .iter()
                .filter_map(|e| match e {
                    egui::Event::Key { key, pressed, .. } => Some((key.name().to_string(), *pressed)),
                    _ => None,
                })
                .collect()
        });
        if events.is_empty() {
            return;
        }
        let Some(world) = Self::world(vm) else { return };
        let mut all = vec![world];
        all.extend(Self::all_morphs(vm, world));
        let targets: Vec<Value> = all.into_iter().filter(|m| getb(vm, *m, "wantsKeys")).collect();
        for (key, down) in events {
            for &m in &targets {
                let k = vm.new_string(&key);
                self.send(vm, m, if down { "keyDown:" } else { "keyUp:" }, vec![k]);
            }
        }
    }

    /// The world's title, set from Self to name the desktop window.
    pub fn world_title(vm: &mut Vm) -> Option<String> {
        let world = Self::world(vm)?;
        let t = get(vm, world, "title")?;
        vm.string_of(t).map(|s| s.to_string()).filter(|s| !s.is_empty())
    }

    fn menu(&mut self, resp: &egui::Response, vm: &mut Vm, m: Value) {
        resp.context_menu(|ui| {
            if ui.button("Inspect (outliner)").clicked() {
                self.inspect.push(m);
                ui.close_menu();
            }
            if ui.button("Duplicate").clicked() {
                if let Ok(c) = vm.ui_send(m, "copy", vec![]) {
                    let owner = get(vm, m, "owner").unwrap_or(vm.nil());
                    let target = if owner.same(vm.nil()) { Self::world(vm).unwrap_or(owner) } else { owner };
                    let x = getf(vm, c, "x", 0.0);
                    let y = getf(vm, c, "y", 0.0);
                    let _ = vm.ui_send(c, "x:", vec![whole(x + 20.0)]);
                    let _ = vm.ui_send(c, "y:", vec![whole(y + 20.0)]);
                    self.send(vm, target, "addMorph:", vec![c]);
                }
                ui.close_menu();
            }
            let stepping = getb(vm, m, "isStepping");
            if ui.button(if stepping { "Stop stepping" } else { "Start stepping" }).clicked() {
                self.send(vm, m, if stepping { "stopStepping" } else { "startStepping" }, vec![]);
                ui.close_menu();
            }
            if ui.button("Delete").clicked() {
                self.send(vm, m, "delete", vec![]);
                ui.close_menu();
            }
        });
    }

    /// Window morphs become windows on the desktop.
    pub fn show_windows(&mut self, ctx: &egui::Context, vm: &mut Vm, c: &Colors) {
        let Some(world) = Self::world(vm) else { return };
        let screen = ctx.screen_rect();
        let mut seen = vec![];
        for m in kids(vm, world) {
            if gets(vm, m, "morphKind") != "window" {
                continue;
            }
            let mid = id_of(m);
            seen.push(mid);
            let title = gets(vm, m, "title");
            let want = Pos2::new(getf(vm, m, "x", 100.0), getf(vm, m, "y", 100.0));
            let size = Vec2::new(getf(vm, m, "width", 280.0).max(120.0), getf(vm, m, "height", 160.0).max(60.0));
            let mut open = true;
            let mut window = egui::Window::new(RichText::new(if title.is_empty() { "Window".into() } else { title }).strong())
                .id(Id::new(("morph-window", mid)))
                .open(&mut open)
                .default_size(size)
                .min_width(120.0)
                .resizable(true)
                .collapsible(true)
                .constrain_to(screen);
            let moved_by_code = self.win_pos.get(&mid).map(|p| (*p - want).length() > 0.5).unwrap_or(true);
            window = if moved_by_code { window.current_pos(want) } else { window.default_pos(want) };
            let resp = window.show(ctx, |ui| {
                ui.set_min_width(size.x - 24.0);
                let items = kids(vm, m);
                if items.is_empty() {
                    ui.label(RichText::new("An empty window. Add morphs with addMorph:").color(c.faint));
                }
                for k in items {
                    self.widget(ui, vm, k, c, 0);
                }
            });
            if let Some(r) = resp {
                let actual = r.response.rect.min;
                if (actual - want).length() > 0.5 {
                    let _ = vm.ui_send(m, "x:", vec![whole(actual.x)]);
                    let _ = vm.ui_send(m, "y:", vec![whole(actual.y)]);
                }
                self.win_pos.insert(mid, Pos2::new(actual.x.round(), actual.y.round()));
                self.menu(&r.response, vm, m);
            }
            if !open {
                self.send(vm, m, "close", vec![]);
            }
        }
        self.win_pos.retain(|k, _| seen.contains(k));
    }

    /// One morph laid out inside a window, row or column.
    fn widget(&mut self, ui: &mut egui::Ui, vm: &mut Vm, m: Value, c: &Colors, depth: usize) {
        if depth > 16 {
            ui.label("…");
            return;
        }
        let mid = id_of(m);
        let kind = gets(vm, m, "morphKind");
        let resp = match kind.as_str() {
            "label" => {
                let text = gets(vm, m, "text");
                let size = getf(vm, m, "fontSize", 16.0).clamp(6.0, 200.0);
                let col = self.color(vm, m, c, c.text);
                ui.add(egui::Label::new(RichText::new(text).size(size).color(col)).wrap())
            }
            "button" => {
                let label = gets(vm, m, "label");
                let r = ui.button(label);
                if r.clicked() {
                    self.send(vm, m, "click", vec![]);
                }
                r
            }
            "textField" => {
                let current = gets(vm, m, "contents");
                let width = getf(vm, m, "width", 180.0);
                let edit_id = Id::new(("morph-field", mid));
                let focused = ui.ctx().memory(|mem| mem.has_focus(edit_id));
                let buf = self.text_buf.entry(mid).or_insert_with(|| current.clone());
                if !focused && *buf != current {
                    *buf = current;
                }
                let r = ui.add(egui::TextEdit::singleline(buf).id(edit_id).desired_width(width));
                if r.changed() {
                    let s = buf.clone();
                    let v = vm.new_string(&s);
                    self.send(vm, m, "contents:", vec![v]);
                    self.send(vm, m, "changed", vec![]);
                }
                r
            }
            "slider" => {
                let (lo, hi) = (getf(vm, m, "min", 0.0) as f64, getf(vm, m, "max", 100.0) as f64);
                let raw = get(vm, m, "value");
                let ints = matches!(raw, Some(Value::Int(_)) | None)
                    && lo.fract() == 0.0
                    && hi.fract() == 0.0;
                let mut v = num(raw).unwrap_or(lo as f32) as f64;
                let width = getf(vm, m, "width", 180.0);
                ui.spacing_mut().slider_width = width;
                let mut slider = egui::Slider::new(&mut v, lo.min(hi)..=hi.max(lo));
                if ints {
                    slider = slider.integer();
                }
                let r = ui.add(slider);
                if r.changed() {
                    let nv = if ints { Value::Int(v.round() as i64) } else { Value::Float(v) };
                    self.send(vm, m, "value:", vec![nv]);
                    self.send(vm, m, "changed", vec![]);
                }
                r
            }
            "checkbox" => {
                let mut on = getb(vm, m, "checked");
                let label = gets(vm, m, "label");
                let r = ui.checkbox(&mut on, label);
                if r.changed() {
                    let b = vm.boolean(on);
                    self.send(vm, m, "checked:", vec![b]);
                    self.send(vm, m, "changed", vec![]);
                }
                r
            }
            "row" => {
                let items = kids(vm, m);
                ui.horizontal_wrapped(|ui| {
                    for k in items {
                        self.widget(ui, vm, k, c, depth + 1);
                    }
                })
                .response
            }
            "column" | "window" => {
                let items = kids(vm, m);
                ui.vertical(|ui| {
                    for k in items {
                        self.widget(ui, vm, k, c, depth + 1);
                    }
                })
                .response
            }
            "canvas" => {
                let size = Vec2::new(getf(vm, m, "width", 320.0).max(10.0), getf(vm, m, "height", 200.0).max(10.0));
                let bg = self.color(vm, m, c, c.background);
                let (rect, r) = ui.allocate_exact_size(size, Sense::click());
                ui.painter().rect_filled(rect, CornerRadius::same(8), bg);
                if let Some(b) = self.border(vm, m, c) {
                    ui.painter().rect_stroke(rect, CornerRadius::same(8), b, egui::StrokeKind::Inside);
                }
                let items = kids(vm, m);
                let mut child = ui.new_child(egui::UiBuilder::new().max_rect(rect));
                child.set_clip_rect(rect.intersect(ui.clip_rect()));
                for k in items {
                    self.draw_shape(&mut child, vm, rect.min, k, c, depth + 1);
                }
                if r.clicked() {
                    self.click_at(vm, m, &r, rect.min);
                }
                r
            }
            "textArea" => {
                let current = gets(vm, m, "contents");
                let (width, height) = (getf(vm, m, "width", 260.0), getf(vm, m, "height", 120.0));
                let edit_id = Id::new(("morph-area-text", mid));
                let focused = ui.ctx().memory(|mem| mem.has_focus(edit_id));
                let buf = self.text_buf.entry(mid).or_insert_with(|| current.clone());
                if !focused && *buf != current {
                    *buf = current;
                }
                let rows = ((height - 8.0) / 18.0).max(2.0) as usize;
                let r = ui.add(egui::TextEdit::multiline(buf).id(edit_id).desired_width(width).desired_rows(rows));
                if r.changed() {
                    let s = buf.clone();
                    let v = vm.new_string(&s);
                    self.send(vm, m, "contents:", vec![v]);
                    self.send(vm, m, "changed", vec![]);
                }
                r
            }
            "list" => {
                let items = get(vm, m, "items").and_then(|v| vm.vector_items(v)).unwrap_or_default();
                let labels: Vec<String> = items.iter().map(|v| self.item_text(vm, *v)).collect();
                let sel = num(get(vm, m, "selectedIndex")).map(|f| f as i64).unwrap_or(-1);
                let (width, height) = (getf(vm, m, "width", 220.0), getf(vm, m, "height", 140.0));
                let mut picked = None;
                let r = egui::Frame::group(ui.style())
                    .show(ui, |ui| {
                        ui.set_width(width - 14.0);
                        egui::ScrollArea::vertical().id_salt(("morph-list", mid)).max_height(height).auto_shrink([false, true]).show(ui, |ui| {
                            if labels.is_empty() {
                                ui.label(RichText::new("(no items)").color(c.faint));
                            }
                            for (i, l) in labels.iter().enumerate() {
                                if ui.selectable_label(i as i64 == sel, l).clicked() {
                                    picked = Some(i);
                                }
                            }
                        });
                    })
                    .response;
                if let Some(i) = picked {
                    self.send(vm, m, "selectedIndex:", vec![Value::Int(i as i64)]);
                    self.send(vm, m, "changed", vec![]);
                }
                r
            }
            "dropDown" => {
                let choices = get(vm, m, "choices").and_then(|v| vm.vector_items(v)).unwrap_or_default();
                let labels: Vec<String> = choices.iter().map(|v| self.item_text(vm, *v)).collect();
                let selected = get(vm, m, "selected");
                let sel_idx = selected.and_then(|s| {
                    choices.iter().position(|c| c.same(s)).or_else(|| {
                        let t = self.item_text(vm, s);
                        labels.iter().position(|l| *l == t)
                    })
                });
                let width = getf(vm, m, "width", 180.0);
                let shown = sel_idx.map(|i| labels[i].clone()).unwrap_or_else(|| "Choose…".into());
                let mut picked = None;
                let r = egui::ComboBox::from_id_salt(("morph-dropdown", mid))
                    .width(width)
                    .selected_text(shown)
                    .show_ui(ui, |ui| {
                        for (i, l) in labels.iter().enumerate() {
                            if ui.selectable_label(Some(i) == sel_idx, l).clicked() {
                                picked = Some(i);
                            }
                        }
                    })
                    .response;
                if let Some(i) = picked {
                    self.send(vm, m, "selected:", vec![choices[i]]);
                    self.send(vm, m, "changed", vec![]);
                }
                r
            }
            "progress" => {
                let v = getf(vm, m, "value", 0.0);
                let max = getf(vm, m, "max", 100.0);
                let frac = if max.abs() > f32::EPSILON { (v / max).clamp(0.0, 1.0) } else { 0.0 };
                let width = getf(vm, m, "width", 200.0);
                ui.add(egui::ProgressBar::new(frac).desired_width(width).show_percentage())
            }
            "separator" => ui.separator(),
            "line" => {
                let (lw, lh) = (getf(vm, m, "width", 100.0), getf(vm, m, "height", 0.0));
                let line_w = getf(vm, m, "lineWidth", 3.0).clamp(0.5, 60.0);
                let size = Vec2::new(lw.abs().max(line_w), lh.abs().max(line_w));
                let (rect, r) = ui.allocate_exact_size(size, Sense::click());
                let col = self.color(vm, m, c, c.text);
                let a = Pos2::new(if lw < 0.0 { rect.right() } else { rect.left() }, if lh < 0.0 { rect.bottom() } else { rect.top() });
                let a = if lh.abs() < line_w { Pos2::new(a.x, rect.center().y) } else { a };
                ui.painter().line_segment([a, a + Vec2::new(lw, lh)], Stroke::new(line_w, col));
                r
            }
            "rectangle" | "circle" => {
                let size = Vec2::new(getf(vm, m, "width", 40.0).max(1.0), getf(vm, m, "height", 40.0).max(1.0));
                let col = self.color(vm, m, c, c.accent);
                let (rect, r) = ui.allocate_exact_size(size, Sense::click());
                if kind == "circle" {
                    ui.painter().circle_filled(rect.center(), size.x.min(size.y) / 2.0, col);
                } else {
                    ui.painter().rect_filled(rect, CornerRadius::same(6), col);
                }
                if r.clicked() {
                    self.click_at(vm, m, &r, rect.min);
                }
                r
            }
            _ => {
                let d = vm.describe(m);
                ui.label(RichText::new(d).color(c.faint))
            }
        };
        self.menu(&resp, vm, m);
    }

    /// How a list or drop-down shows an item: strings as they are, other objects printed.
    fn item_text(&self, vm: &mut Vm, v: Value) -> String {
        match vm.string_of(v) {
            Some(s) => s.to_string(),
            None => vm.print_string(v),
        }
    }

    /// Forget editing state for morphs that no longer exist.
    pub fn tidy(&mut self, vm: &Vm) {
        self.text_buf.retain(|k, _| vm.is_alive(Value::Ref(*k)));
        self.drag_rest.retain(|k, _| vm.is_alive(Value::Ref(*k)));
    }
}
