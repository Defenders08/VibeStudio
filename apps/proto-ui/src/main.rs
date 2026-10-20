//! VibeStudio — прототип FL-подобного интерфейса (egui) на модели движка.
//!
//! Фаза 2: данные проекта живут в vibe_engine (модель serde), UI шлёт
//! команды через engine.execute() и рендерит снапшоты. Undo/redo — команды
//! движка. UX по research/fl-studio-clean-room; скин FL 21.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use eframe::egui;
use egui::{Align2, Color32, Key, Sense};
use std::collections::HashSet;
use std::time::Instant;
use vibe_engine::model::{Channel, ChannelKind, ModelNote, ProjectModel};
use vibe_engine::{Command, Engine, PlayMode, TransportStatus};

const PPQ: u32 = 96;
const TICKS_PER_BAR: f32 = 384.0;
const STEP_TICKS: f32 = 24.0; // 1/16
const SONG_BARS: usize = 16;
const VERSION_LINE: &str = "VibeStudio Prototype v0.3.0 — модель в движке · команды · undo · WAV";

// ---- палитра FL 21 ----
const C_BG: Color32 = Color32::from_rgb(0x23, 0x25, 0x27);
const C_PANEL: Color32 = Color32::from_rgb(0x4A, 0x4D, 0x52);
const C_PANEL_DARK: Color32 = Color32::from_rgb(0x2E, 0x30, 0x33);
const C_WINDOW: Color32 = Color32::from_rgb(0x3E, 0x41, 0x45);
const C_GRID: Color32 = Color32::from_rgb(0x43, 0x46, 0x4B);
const C_GRID_STRONG: Color32 = Color32::from_rgb(0x5A, 0x5D, 0x62);
const C_TEXT_DIM: Color32 = Color32::from_rgb(0xB4, 0xB7, 0xBB);
const C_ACCENT: Color32 = Color32::from_rgb(0xF0, 0xA4, 0x4A);
const C_PLAY: Color32 = Color32::from_rgb(0x96, 0xD9, 0x46);
const C_LED_OFF: Color32 = Color32::from_rgb(0x3A, 0x4A, 0x3A);
const C_REC: Color32 = Color32::from_rgb(0xE0, 0x43, 0x3E);
const C_STEP_OFF: Color32 = Color32::from_rgb(0x43, 0x2F, 0x31);
const C_STEP_BEAT: Color32 = Color32::from_rgb(0x4E, 0x38, 0x3A);
const C_STEP_ON: Color32 = Color32::from_rgb(0xDE, 0x8F, 0x8A);
const C_STEP_ON_BEAT: Color32 = Color32::from_rgb(0xE8, 0xA4, 0x9E);
const C_BEVEL_HI: Color32 = Color32::from_rgb(0x5E, 0x61, 0x66);
const C_BEVEL_LO: Color32 = Color32::from_rgb(0x26, 0x28, 0x2A);
const C_BTN: Color32 = Color32::from_rgb(0x56, 0x5A, 0x5F);
const C_BTN_HI: Color32 = Color32::from_rgb(0x6A, 0x6E, 0x74);
const C_KEY_BLACK: Color32 = Color32::from_rgb(0x3A, 0x3D, 0x40);
const C_KEY_WHITE: Color32 = Color32::from_rgb(0xDC, 0xDC, 0xDC);
const C_LCD_BG: Color32 = Color32::from_rgb(0x17, 0x18, 0x1A);
const C_LCD_TEXT: Color32 = Color32::from_rgb(0xF2, 0xF2, 0xF2);
const C_COMBO_LIGHT: Color32 = Color32::from_rgb(0xC9, 0xCC, 0xCE);
const TRACK_GROUP_COLORS: [Color32; 4] = [
    Color32::from_rgb(0x4E, 0x7A, 0x46),
    Color32::from_rgb(0x7A, 0x3D, 0x3D),
    Color32::from_rgb(0x5E, 0x44, 0x68),
    Color32::from_rgb(0x7A, 0x5A, 0x35),
];

const SNAP_OPTIONS: [&str; 8] =
    ["(none)", "Line", "Cell", "Beat", "1/2 beat", "1/4 beat", "1/16", "Bar"];

fn main() -> eframe::Result<()> {
    let opts = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1440.0, 880.0])
            .with_min_inner_size([1100.0, 640.0]),
        ..Default::default()
    };
    eframe::run_native(
        "VibeStudio — прототип интерфейса",
        opts,
        Box::new(|cc| {
            setup_style(&cc.egui_ctx);
            Ok(Box::new(App::new(&cc.egui_ctx)))
        }),
    )
}

fn setup_style(ctx: &egui::Context) {
    ctx.set_theme(egui::Theme::Dark);
    let mut v = egui::Visuals::dark();
    v.panel_fill = C_PANEL;
    v.window_fill = C_WINDOW;
    v.extreme_bg_color = C_PANEL_DARK;
    v.faint_bg_color = C_PANEL_DARK;
    v.selection.stroke = egui::Stroke::new(1.0_f32, C_ACCENT);
    v.widgets.noninteractive.fg_stroke =
        egui::Stroke::new(1.0_f32, Color32::from_rgb(0xD0, 0xD2, 0xD5));
    ctx.set_visuals(v);
    let mut style = (*ctx.style()).clone();
    style.spacing.item_spacing = egui::vec2(6.0, 3.0);
    style.spacing.button_padding = egui::vec2(6.0, 3.0);
    ctx.set_style(style);
}

#[derive(Clone, Copy, PartialEq)]
enum Tool {
    Draw,
    Delete,
    Select,
    Mute,
}

/// Контекстное меню (правая кнопка): позиция + цель.
#[derive(Clone, Copy, PartialEq)]
enum CtxTarget {
    Clip { track: u32, bar: u32 },
    Channel { id: u32 },
    PlaylistEmpty { track: u32, bar: u32 },
}

#[derive(Clone, Copy)]
enum RollDrag {
    Move { idx: usize, dstep: i32, dkey: i16 },
    Resize { idx: usize },
}

struct App {
    engine: Engine,
    // зеркало транспорта (только чтение; источник — engine.transport())
    status: TransportStatus,
    pos_ticks: f64,
    // окна
    show_browser: bool,
    show_playlist: bool,
    show_rack: bool,
    show_roll: bool,
    show_mixer: bool,
    show_picker: bool,
    show_piano: bool,
    hint: String,
    // транспорт
    bpm: f64,
    pattern_mode: bool,
    recording: bool,
    metronome: bool,
    time_as_clock: bool,
    // выбор
    current_pattern: usize,
    active_channel: u32,
    last_note_len: u32,
    selected_note: Option<usize>,
    roll_drag: Option<RollDrag>,
    tool: Tool,
    snap: usize,
    // ui-прочее
    playlist_zoom: f32,
    /// Выделение клипов (track, bar) — инструмент Select.
    selection: HashSet<(u32, u32)>,
    /// Прямоугольник выделения (пиксели, экран-координаты сетки).
    select_rect: Option<(egui::Pos2, egui::Pos2)>,
    confirm_delete_channel: Option<u32>,
    ctx_menu: Option<(egui::Pos2, CtxTarget)>,
    ctx_menu_rect: Option<egui::Rect>,
    tap_times: Vec<Instant>,
    icons: HashMap<&'static str, egui::TextureHandle>,
    current_file: Option<String>,
    load_error: Option<String>,
    phase: f32,
    last_frame: Instant,
}

use std::collections::HashMap;

impl App {
    fn new(ctx: &egui::Context) -> Self {
        let mut icons = HashMap::new();
        for name in [
            "draw", "paint", "paint_drum", "delete", "mute", "slice", "select",
            "playback", "play", "note", "portamento", "interpolate", "audio", "instrument",
        ] {
            if let Some(tex) = load_icon(ctx, name) {
                icons.insert(name, tex);
            }
        }
        let mut app = Self {
            engine: Engine::new(),
            status: TransportStatus::Stopped,
            pos_ticks: 0.0,
            show_browser: true,
            show_playlist: true,
            show_rack: true,
            show_roll: false,
            show_mixer: false,
            show_picker: false,
            show_piano: false,
            hint: "Space — play/pause · Enter — стоп · Ctrl+O — открыть .flp · Ctrl+T — клавиатура-пиано".into(),
            bpm: 120.0,
            pattern_mode: true,
            recording: false,
            metronome: false,
            time_as_clock: false,
            current_pattern: 0,
            active_channel: 0,
            last_note_len: 4,
            selected_note: None,
            roll_drag: None,
            tool: Tool::Draw,
            snap: 1,
            playlist_zoom: 1.0,
            selection: HashSet::new(),
            select_rect: None,
            confirm_delete_channel: None,
            ctx_menu: None,
            ctx_menu_rect: None,
            tap_times: Vec::new(),
            icons,
            current_file: None,
            load_error: None,
            phase: 0.0,
            last_frame: Instant::now(),
        };
        // демо-бит: 4 шага кика в Pattern 1 — сразу есть что играть
        for s in [0u32, 4, 8, 12] {
            app.engine
                .execute(Command::SetStep { pattern_id: 1, channel_id: 0, step: s, on: true });
        }
        app
    }

    fn model(&self) -> ProjectModel {
        self.engine.model_snapshot()
    }

    fn channel_color(&self, ch: &Channel) -> Color32 {
        let (r, g, b) = match ch.kind {
            ChannelKind::Kick => (0xC4, 0x51, 0x3E),
            ChannelKind::Synth => (0x6E, 0x9B, 0x5E),
            ChannelKind::Sampler => (0x7A, 0x8A, 0xB0),
        };
        let seed = (ch.id as u32).wrapping_mul(0x9E3779B9);
        let dr = (seed & 0x3F) as i32 - 24;
        let dg = ((seed >> 8) & 0x3F) as i32 - 24;
        let db = ((seed >> 16) & 0x3F) as i32 - 24;
        Color32::from_rgb(
            (r as i32 + dr).clamp(60, 240) as u8,
            (g as i32 + dg).clamp(60, 240) as u8,
            (b as i32 + db).clamp(60, 240) as u8,
        )
    }

    fn pattern_color(&self, _m: &ProjectModel, id: u32) -> Color32 {
        let base = match id % 4 {
            0 => (0x6E, 0x9B, 0x5E),
            1 => (0xC4, 0x7A, 0x4A),
            2 => (0x7A, 0x6A, 0xB0),
            _ => (0x4A, 0x8A, 0xA0),
        };
        Color32::from_rgb(base.0, base.1, base.2)
    }

    fn snap_ticks(&self) -> u32 {
        match self.snap {
            0 => 1,
            1 | 2 => 24,
            3 => 96,
            4 => 48,
            5 => 24,
            6 => 6,
            _ => 384,
        }
    }

    fn set_hint(&mut self, r: &egui::Response, text: &str) {
        if r.hovered() {
            self.hint = text.to_string();
        }
    }

    fn transport_toggle(&mut self) {
        match self.engine.transport().0 {
            TransportStatus::Playing => {
                self.engine.execute(Command::Pause);
                self.hint = "Пауза (позиция сохранена)".into();
            }
            _ => {
                self.engine.execute(Command::Play);
                self.hint = if self.pattern_mode {
                    "PAT: играет текущий паттерн".into()
                } else {
                    "SONG: играет аранжировка".into()
                };
            }
        }
    }

    fn set_mode_live(&mut self) {
        let mode = if self.pattern_mode { PlayMode::Pat } else { PlayMode::Song };
        self.engine.execute(Command::SetMode { mode });
    }

    fn tap(&mut self) {
        let now = Instant::now();
        if let Some(last) = self.tap_times.last() {
            if now.duration_since(*last).as_secs_f32() > 2.0 {
                self.tap_times.clear();
            }
        }
        self.tap_times.push(now);
        if self.tap_times.len() >= 2 {
            let t = &self.tap_times;
            let sum: f32 = t.windows(2).map(|w| w[1].duration_since(w[0]).as_secs_f32()).sum();
            let bpm = (60.0 / (sum / (t.len() - 1) as f32)).clamp(10.0, 522.0);
            self.bpm = bpm as f64;
            self.engine.execute(Command::SetBpm { bpm: bpm as f64 });
            self.hint = format!("Tap tempo: {bpm:.1} BPM");
        } else {
            self.hint = "Tap tempo: продолжай нажимать в ритм…".into();
        }
    }

    fn open_flp_dialog(&mut self) {
        let Some(path) = rfd::FileDialog::new()
            .add_filter("FL Studio project", &["flp"])
            .set_title("Открыть .flp")
            .pick_file()
        else {
            return;
        };
        match std::fs::read(&path)
            .map_err(|e| format!("{e}"))
            .and_then(|b| flp_compat::parse(&b).map_err(|e| format!("{e}")))
        {
            Ok(flp) => {
                let model = convert_flp(&flp);
                self.engine.execute(Command::LoadProject { model });
                self.current_file = path.file_name().map(|n| n.to_string_lossy().to_string());
                self.bpm = self.engine.model_snapshot().bpm;
                self.hint = format!("Открыт {}: {} каналов, {} паттернов",
                    self.current_file.clone().unwrap_or_default(),
                    self.engine.model_snapshot().channels.len(),
                    self.engine.model_snapshot().patterns.len());
            }
            Err(e) => {
                self.load_error = Some(e.clone());
                self.hint = format!("Не удалось открыть: {e}");
            }
        }
    }

    fn save_dialog(&mut self) {
        let Some(path) = rfd::FileDialog::new()
            .add_filter("VibeStudio project", &["vibe"])
            .set_file_name("project.vibe")
            .set_title("Сохранить проект")
            .save_file()
        else {
            return;
        };
        match self.engine.save_project(&path.to_string_lossy()) {
            Ok(()) => {
                self.current_file = path.file_name().map(|n| n.to_string_lossy().to_string());
                self.hint = format!("Сохранено: {}", path.display());
            }
            Err(e) => self.hint = format!("Не сохранено: {e}"),
        }
    }

    fn open_vibe_dialog(&mut self) {
        let Some(path) = rfd::FileDialog::new()
            .add_filter("VibeStudio project", &["vibe"])
            .set_title("Открыть .vibe")
            .pick_file()
        else {
            return;
        };
        match self.engine.load_project_file(&path.to_string_lossy()) {
            Ok(()) => {
                self.bpm = self.engine.model_snapshot().bpm;
                self.hint = format!("Открыт {}", path.display());
            }
            Err(e) => self.hint = format!("Не открыт: {e}"),
        }
    }

    fn export_wav_dialog(&mut self) {
        let Some(path) = rfd::FileDialog::new()
            .add_filter("WAV", &["wav"])
            .set_file_name("vibestudio_export.wav")
            .set_title("Экспорт WAV (offline render)")
            .save_file()
        else {
            return;
        };
        let mut sh = self.engine.lock();
        sh.replan();
        let events = sh.events.clone();
        let loop_ticks = sh.loop_ticks;
        let bpm = sh.model.bpm;
        let loops = if self.pattern_mode { 2 } else { 1 };
        match vibe_engine::render_to_wav(&mut sh, &events, loop_ticks, bpm, loops, &path) {
            Ok(frames) => {
                let secs = frames as f64 / 44_100.0;
                self.hint = format!("Экспорт: {frames} кадров ({secs:.1} c) → {}", path.display());
            }
            Err(e) => self.hint = format!("Экспорт не удался: {e}"),
        }
    }
}

/// Конвертация flp_compat::Project → модель движка.
fn convert_flp(p: &flp_compat::Project) -> ProjectModel {
    let mut m = ProjectModel::default();
    m.bpm = p.tempo_bpm.unwrap_or(120.0);
    m.channels.clear();
    m.patterns.clear();
    m.next_id = 0;
    for ch in &p.channels {
        let kind = match ch.kind {
            Some(0) => ChannelKind::Sampler,
            Some(2) | Some(4) => ChannelKind::Synth,
            _ => ChannelKind::Sampler,
        };
        m.channels.push(Channel {
            id: ch.iid as u32,
            name: ch.name.clone().unwrap_or_else(|| format!("Channel {}", ch.iid)),
            kind,
            sample_path: ch.sample_path.clone().unwrap_or_default(),
            volume: ch
                .levels
                .map(|l| (l.volume as f32 / 12800.0).clamp(0.0, 1.2))
                .unwrap_or(0.78),
            pan: ch
                .levels
                .map(|l| (l.pan as f32 / 6400.0 - 1.0).clamp(-1.0, 1.0))
                .unwrap_or(0.0),
            muted: ch.enabled.map(|e| !e).unwrap_or(false),
        });
        m.next_id = m.next_id.max(ch.iid as u32 + 1);
    }
    if m.channels.is_empty() {
        m.channels.push(Channel {
            id: 0,
            name: "Sampler".into(),
            kind: ChannelKind::Kick,
            sample_path: String::new(),
            volume: 0.78,
            pan: 0.0,
            muted: false,
        });
        m.next_id = 1;
    }
    for (i, pt) in p.patterns.iter().enumerate() {
        let pid = (i + 1) as u32;
        let mut steps = Vec::new();
        let mut notes = Vec::new();
        for n in &pt.notes {
            let step = (n.position as f32 / STEP_TICKS).round() as u32;
            steps.push((n.channel_iid as u32, step));
            notes.push((
                n.channel_iid as u32,
                ModelNote {
                    step,
                    key: n.key,
                    vel: n.velocity,
                    len_steps: ((n.length_ticks as f32 / STEP_TICKS).round() as u32).max(1),
                },
            ));
        }
        m.patterns.push(vibe_engine::model::Pattern {
            id: pid,
            name: pt.name.clone().unwrap_or_else(|| format!("Pattern {}", pid)),
            steps,
            notes,
        });
        m.next_id = m.next_id.max(pid + 1);
    }
    if m.patterns.is_empty() {
        m.patterns.push(vibe_engine::model::Pattern {
            id: 1,
            name: "Pattern 1".into(),
            steps: Vec::new(),
            notes: Vec::new(),
        });
        m.next_id = m.next_id.max(2);
    }
    m
}

fn load_icon(ctx: &egui::Context, name: &str) -> Option<egui::TextureHandle> {
    let path = format!("{}/icons/own/{name}.png", env!("CARGO_MANIFEST_DIR"));
    let bytes = std::fs::read(&path).ok()?;
    let img = image::load_from_memory(&bytes).ok()?.to_rgba8();
    let size = [img.width() as usize, img.height() as usize];
    Some(ctx.load_texture(
        format!("icon_{name}"),
        egui::ColorImage::from_rgba_unmultiplied(size, &img),
        egui::TextureOptions::LINEAR,
    ))
}

fn icon_button(
    ui: &mut egui::Ui,
    icons: &HashMap<&'static str, egui::TextureHandle>,
    name: &str,
    size: f32,
    tip: &str,
) -> egui::Response {
    let tex = icons
        .get(name)
        .unwrap_or_else(|| panic!("иконка {name} не загружена"));
    ui.add(egui::ImageButton::new(
        egui::Image::from_texture(tex).fit_to_exact_size(egui::vec2(size, size)),
    ))
    .on_hover_text(tip)
}

fn icon_img(ui: &mut egui::Ui, icons: &HashMap<&'static str, egui::TextureHandle>, name: &str, size: f32) {
    if let Some(tex) = icons.get(name) {
        ui.add(egui::Image::from_texture(tex).fit_to_exact_size(egui::vec2(size, size)));
    }
}

/// Круглая ручка в стиле FL.
fn knob_ui(ui: &mut egui::Ui, value: &mut f32, lo: f32, hi: f32, tip: &str) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(20.0, 20.0), Sense::drag());
    let t = ((*value - lo) / (hi - lo)).clamp(0.0, 1.0);
    let p = ui.painter_at(rect);
    p.circle_filled(rect.center(), 9.0, C_PANEL_DARK);
    p.circle(rect.center(), 9.0, Color32::TRANSPARENT, egui::Stroke::new(1.0_f32, C_BEVEL_LO));
    let ang = (-135.0 + 270.0 * t).to_radians();
    let dir = egui::vec2(ang.cos(), ang.sin());
    p.line_segment(
        [rect.center() - dir * 3.0, rect.center() + dir * 7.0],
        egui::Stroke::new(2.0_f32, C_PLAY),
    );
    if resp.dragged() {
        let dy = ui.input(|i| i.pointer.delta().y);
        *value = (*value - dy * 0.005 * (hi - lo)).clamp(lo, hi);
    }
    resp.on_hover_text(tip)
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.set_theme(egui::Theme::Dark);
        let now = Instant::now();
        let dt = now.duration_since(self.last_frame).as_secs_f32().min(0.1);
        self.last_frame = now;

        // ЕДИНЫЙ ТРАНСПОРТ: позиция/статус — из атомиков движка, каждый кадр
        let (status, tick) = self.engine.transport();
        self.status = status;
        self.pos_ticks = tick;
        if status == TransportStatus::Playing {
            self.phase += dt * 9.0;
        }

        ctx.input(|i| {
            let cmd = i.modifiers.command;
            if i.key_pressed(Key::Space) {
                self.transport_toggle();
            }
            if i.key_pressed(Key::Enter) {
                self.engine.execute(Command::Stop);
                self.hint = "Стоп: позиция 1:1:0 (Enter)".into();
            }
            if i.key_pressed(Key::L) {
                self.pattern_mode = !self.pattern_mode;
                self.set_mode_live();
            }
            if i.key_pressed(Key::Z) && cmd && !i.modifiers.shift {
                if self.engine.undo() {
                    self.hint = "Undo (Ctrl+Z)".into();
                } else {
                    self.hint = "Undo: история пуста".into();
                }
            }
            if (i.key_pressed(Key::Z) && cmd && i.modifiers.shift) || (i.key_pressed(Key::Y) && cmd) {
                if self.engine.redo() {
                    self.hint = "Redo (Ctrl+Y)".into();
                }
            }
            if i.key_pressed(Key::S) && cmd {
                self.save_dialog();
            }
            if i.key_pressed(Key::O) && cmd {
                self.open_flp_dialog();
            }
            if i.key_pressed(Key::T) && cmd {
                self.show_piano = !self.show_piano;
            }
            if i.key_pressed(Key::H) && cmd {
                self.engine.execute(Command::Stop);
                self.hint = "Panic (Ctrl+H) — стоп".into();
            }
            if i.key_pressed(Key::M) && cmd {
                self.metronome = !self.metronome;
            }
            if i.key_pressed(Key::Home) {
                self.engine.execute(Command::Seek { tick: 0.0 });
            }
            if i.key_pressed(Key::Backspace) {
                self.snap = if self.snap == 0 { 1 } else { 0 };
            }
            if i.key_pressed(Key::F5) {
                self.show_playlist = !self.show_playlist;
            }
            if i.key_pressed(Key::F6) {
                self.show_rack = !self.show_rack;
            }
            if i.key_pressed(Key::F7) {
                self.show_roll = !self.show_roll;
            }
            if i.key_pressed(Key::F8) {
                self.show_picker = !self.show_picker;
            }
            if i.key_pressed(Key::F9) {
                self.show_mixer = !self.show_mixer;
            }
            if i.key_pressed(Key::F12) {
                self.show_playlist = false;
                self.show_roll = false;
                self.show_mixer = false;
                self.show_picker = false;
            }
            if i.key_pressed(Key::Escape) {
                if self.show_picker {
                    self.show_picker = false;
                } else if self.show_roll {
                    self.show_roll = false;
                } else if self.show_mixer {
                    self.show_mixer = false;
                } else if self.show_playlist {
                    self.show_playlist = false;
                }
            }
        });
        if status == TransportStatus::Playing {
            ctx.request_repaint_after(std::time::Duration::from_millis(8));
        } else {
            ctx.request_repaint_after(std::time::Duration::from_millis(30));
        }

        self.top_toolbar(ctx);
        if self.show_browser {
            self.browser(ctx);
        }
        self.status_bar(ctx);
        self.desktop(ctx);
        if self.show_rack {
            self.channel_rack_window(ctx);
        }
        if self.show_playlist {
            self.playlist_window(ctx);
        }
        if self.show_roll {
            self.piano_roll_window(ctx);
        }
        if self.show_mixer {
            self.mixer_window(ctx);
        }
        if self.show_picker {
            self.plugin_picker(ctx);
        }
        if self.show_piano {
            self.typing_piano_window(ctx);
        }
        self.confirm_delete_window(ctx);
        self.context_menu_window(ctx);
    }
}

impl App {
    fn top_toolbar(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("toolbar").show(ctx, |ui| {
            ui.vertical(|ui| {
                // РЯД 1: меню + transport + LCD
                ui.horizontal(|ui| {
                    ui.menu_button("FILE", |ui| {
                        if ui.button("New — пустой проект").clicked() {
                            self.engine.execute(Command::LoadProject {
                                model: ProjectModel::default(),
                            });
                            self.hint = "Новый проект".into();
                            ui.close_menu();
                        }
                        if ui.button("Open… (Ctrl+O)").clicked() {
                            self.open_flp_dialog();
                            ui.close_menu();
                        }
                        if ui.button("Open .vibe…").clicked() {
                            self.open_vibe_dialog();
                            ui.close_menu();
                        }
                        if ui.button("Save (Ctrl+S)").clicked() {
                            self.save_dialog();
                            ui.close_menu();
                        }
                        if ui.button("Export → WAV…").clicked() {
                            self.export_wav_dialog();
                            ui.close_menu();
                        }
                    });
                    for (name, items) in [
                        ("EDIT", vec!["Undo (Ctrl+Z)", "Redo (Ctrl+Y)"]),
                        ("ADD", vec!["Sampler", "Synth / Generator"]),
                        ("PATTERNS", vec!["Rename (F2) — заглушка", "Clone (Alt+C) — заглушка"]),
                        ("VIEW", vec!["Playlist (F5)", "Channel Rack (F6)", "Piano Roll (F7)", "Mixer (F9)"]),
                        ("HELP", vec!["About — см. статус-бар"]),
                    ] {
                        ui.menu_button(name, |ui| {
                            for it in items {
                                if ui.button(it).clicked() {
                                    match it {
                                        "Undo (Ctrl+Z)" => {
                                            self.engine.undo();
                                        }
                                        "Redo (Ctrl+Y)" => {
                                            self.engine.redo();
                                        }
                                        "Sampler" => self.engine.execute(Command::AddChannel {
                                            kind: ChannelKind::Sampler,
                                            name: "Sampler".into(),
                                        }),
                                        "Synth / Generator" => self.engine.execute(Command::AddChannel {
                                            kind: ChannelKind::Synth,
                                            name: "Generator".into(),
                                        }),
                                        "Playlist (F5)" => self.show_playlist = !self.show_playlist,
                                        "Channel Rack (F6)" => self.show_rack = !self.show_rack,
                                        "Piano Roll (F7)" => self.show_roll = !self.show_roll,
                                        "Mixer (F9)" => self.show_mixer = !self.show_mixer,
                                        _ => {}
                                    }
                                }
                            }
                        });
                    }
                    ui.separator();
                    ui.vertical(|ui| {
                        let pat = egui::Button::new(
                            egui::RichText::new("PAT").size(9.0).color(if self.pattern_mode { Color32::BLACK } else { C_TEXT_DIM }),
                        )
                        .fill(if self.pattern_mode { C_ACCENT } else { C_PANEL_DARK })
                        .min_size(egui::vec2(38.0, 12.0));
                        if ui.add(pat).clicked() {
                            let was = self.pattern_mode;
                            self.pattern_mode = true;
                            if was != self.pattern_mode {
                                self.set_mode_live();
                            }
                        }
                        let song = egui::Button::new(
                            egui::RichText::new("SONG").size(9.0).color(if !self.pattern_mode { Color32::BLACK } else { C_TEXT_DIM }),
                        )
                        .fill(if !self.pattern_mode { C_ACCENT } else { C_PANEL_DARK })
                        .min_size(egui::vec2(38.0, 12.0));
                        if ui.add(song).clicked() {
                            let was = self.pattern_mode;
                            self.pattern_mode = false;
                            if was != self.pattern_mode {
                                self.set_mode_live();
                            }
                        }
                    })
                    .response
                    .on_hover_text("PAT — текущий паттерн, SONG — аранжировка (L); переключение на лету");
                    ui.separator();
                    let is_playing = self.status == TransportStatus::Playing;
                    let play = ui.add(egui::Button::new(
                        egui::RichText::new(if is_playing { "⏸" } else { "▶" }).size(15.0),
                    ));
                    self.set_hint(&play, "Play/Pause — Space");
                    if play.clicked() {
                        self.transport_toggle();
                    }
                    let stop = ui.add(egui::Button::new(egui::RichText::new("■").size(13.0)));
                    self.set_hint(&stop, "Stop — позиция 1:1:0 (Enter)");
                    if stop.clicked() {
                        self.engine.execute(Command::Stop);
                    }
                    ui.add(
                        egui::DragValue::new(&mut self.bpm)
                            .range(10.0..=522.0)
                            .speed(0.01)
                            .custom_formatter(|v, _| format!("{v:.3}")),
                    )
                    .on_hover_text("Темп 10–522 BPM");
                    if self.bpm != self.engine.model_snapshot().bpm {
                        self.engine.execute(Command::SetBpm { bpm: self.bpm });
                    }
                    let tap = ui.add(egui::Button::new(egui::RichText::new("TAP").size(10.0)));
                    self.set_hint(&tap, "Tap tempo");
                    if tap.clicked() {
                        self.tap();
                    }
                    ui.separator();
                    let bar = (self.pos_ticks / 384.0).floor() as usize + 1;
                    let beat = ((self.pos_ticks % 384.0) / 96.0).floor() as usize + 1;
                    let step = ((self.pos_ticks % 96.0) / 24.0).floor() as usize + 1;
                    let secs = self.pos_ticks / 96.0 * (60.0 / self.bpm as f64);
                    let (lcd, mode) = if self.time_as_clock {
                        (format!("{:02}:{:02}.{:02}", (secs / 60.0) as usize, (secs % 60.0) as usize, ((secs * 100.0) % 100.0) as usize), "M:S.C")
                    } else {
                        (format!("{bar}:{beat}:{step}"), "B:B.S")
                    };
                    let (lcd_rect, lcd_resp) = ui.allocate_exact_size(egui::vec2(108.0, 22.0), Sense::click());
                    ui.painter().rect_filled(lcd_rect, 2.0, C_LCD_BG);
                    ui.painter().text(lcd_rect.center(), Align2::CENTER_CENTER, format!("{lcd}  {mode}"), egui::FontId::monospace(14.0), C_LCD_TEXT);
                    self.set_hint(&lcd_resp, "Song Position LCD (клик — формат бары/время)");
                    if lcd_resp.clicked() {
                        self.time_as_clock = !self.time_as_clock;
                    }
                    ui.separator();
                    // осциллоскоп + peak из атомика движка
                    let peak = self.engine.peak();
                    let (scope_rect, scope_resp) = ui.allocate_exact_size(egui::vec2(100.0, 24.0), Sense::hover());
                    self.set_hint(&scope_resp, "Выход: peak из атомика движка");
                    let pt = ui.painter_at(scope_rect);
                    pt.rect_filled(scope_rect, 1.0, C_LCD_BG);
                    let mid = scope_rect.center().y;
                    let mut prev = scope_rect.left_center();
                    let amp = 3.0 + 8.0 * peak;
                    for x in 0..=100 {
                        let t = x as f32 / 100.0;
                        let y = mid + amp * (t * 12.0 + self.phase * 2.0).sin();
                        let next = egui::pos2(scope_rect.left() + t * scope_rect.width(), y);
                        pt.line_segment([prev, next], egui::Stroke::new(1.2_f32, C_PLAY));
                        prev = next;
                    }
                    let mh = peak * scope_rect.height();
                    pt.rect_filled(
                        egui::Rect::from_min_size(egui::pos2(scope_rect.right() - 6.0, scope_rect.bottom() - mh), egui::vec2(4.0, mh)),
                        1.0,
                        if peak > 0.92 { C_REC } else { C_PLAY },
                    );
                    ui.label(egui::RichText::new("CPU (n/a)").monospace())
                        .on_hover_text("CPU-метр появится с профилированием движка");
                });
                ui.separator();
                // РЯД 2
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.label(egui::RichText::new("VibeStudio").size(11.0));
                        let m = self.engine.model_snapshot();
                        let pname = m.patterns.get(self.current_pattern).map(|p| p.name.clone()).unwrap_or_default();
                        ui.label(egui::RichText::new(pname).strong().size(13.0));
                    });
                    ui.separator();
                    ui.label("Snap:");
                    egui::ComboBox::from_id_salt("snap_sel")
                        .selected_text(SNAP_OPTIONS[self.snap])
                        .width(76.0)
                        .show_ui(ui, |ui| {
                            for (i, s) in SNAP_OPTIONS.iter().enumerate() {
                                ui.selectable_value(&mut self.snap, i, *s);
                            }
                        });
                    ui.separator();
                    let m = self.engine.model_snapshot();
                    let minus = ui.add(egui::Button::new("−").small())
                        .on_hover_text("Предыдущий паттерн (−)");
                    if minus.clicked() && self.current_pattern > 0 {
                        self.current_pattern -= 1;
                    }
                    let n_pats = m.patterns.len();
                    egui::ComboBox::from_id_salt("pat_sel")
                        .selected_text(
                            m.patterns.get(self.current_pattern).map(|p| p.name.clone()).unwrap_or_default(),
                        )
                        .width(96.0)
                        .show_ui(ui, |ui| {
                            for (i, p) in m.patterns.iter().enumerate() {
                                ui.selectable_value(&mut self.current_pattern, i, p.name.clone());
                            }
                        });
                    let plus = ui.add(egui::Button::new("+").small())
                        .on_hover_text("Следующий паттерн (+)");
                    if plus.clicked() && self.current_pattern + 1 < n_pats {
                        self.current_pattern += 1;
                    }
                    ui.separator();
                    self.tool_buttons(ui);
                    ui.separator();
                    let u = ui.add(egui::Button::new(egui::RichText::new("↶").size(14.0)))
                        .on_hover_text("Undo (Ctrl+Z)");
                    if u.clicked() {
                        self.engine.undo();
                    }
                    let r = ui.add(egui::Button::new(egui::RichText::new("↷").size(14.0)))
                        .on_hover_text("Redo (Ctrl+Y)");
                    if r.clicked() {
                        self.engine.redo();
                    }
                    ui.separator();
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let b = ui.selectable_label(self.show_browser, "Browser")
                            .on_hover_text("Browser (панель слева)");
                        if b.clicked() {
                            self.show_browser = !self.show_browser;
                        }
                        for (label, shown, toggle) in [
                            ("Mixer F9", self.show_mixer, 0),
                            ("Roll F7", self.show_roll, 1),
                            ("Playlist F5", self.show_playlist, 2),
                            ("Rack F6", self.show_rack, 3),
                        ] {
                            let btn = ui.selectable_label(shown, label);
                            self.set_hint(&btn, "Показать/скрыть окно");
                            if btn.clicked() {
                                match toggle {
                                    0 => self.show_mixer = !self.show_mixer,
                                    1 => self.show_roll = !self.show_roll,
                                    2 => self.show_playlist = !self.show_playlist,
                                    _ => self.show_rack = !self.show_rack,
                                }
                            }
                        }
                    });
                });
            });
        });
    }

    fn tool_buttons(&mut self, ui: &mut egui::Ui) {
        // Ряд как в FL: Draw / Delete / Select / Mute
        let entries: [(&str, &str, bool, bool); 4] = [
            ("draw", "Draw (P): ЛКМ — поставить, drag — переместить/растянуть", self.tool == Tool::Draw, false),
            ("delete", "Delete (D): стирать клипы/ноты", self.tool == Tool::Delete, false),
            ("select", "Select (E): рамка — выделить клипы; Del — удалить выделенные; drag выделенного — двигать группу", self.tool == Tool::Select, false),
            ("mute", "Mute (T): клик по клипу — приглушить (слышно в SONG)", self.tool == Tool::Mute, false),
        ];
        for (icon, tip, active, _stub) in entries {
            let mut r = icon_button(ui, &self.icons, icon, 17.0, tip);
            if active {
                ui.painter().rect_filled(r.rect.expand(2.0), 2.0, C_ACCENT.gamma_multiply(0.35));
                r.mark_changed();
            }
            if r.clicked() {
                self.tool = match icon {
                    "draw" => Tool::Draw,
                    "delete" => Tool::Delete,
                    "select" => Tool::Select,
                    _ => Tool::Mute,
                };
            }
        }
    }

    /// Кнопка/пункт, функционал которого не реализован: запрещающий курсор.
    fn stub_item(&mut self, ui: &mut egui::Ui, label: &str) {
        let r = ui.button(label).on_hover_cursor(egui::CursorIcon::NotAllowed);
        if r.hovered() {
            self.hint = format!("«{label}» — не реализовано в прототипе");
        }
        if r.clicked() {
            self.hint = format!("«{label}» — не реализовано (заглушка)");
        }
    }

    fn channel_rack_window(&mut self, ctx: &egui::Context) {
        let mut open = self.show_rack;
        egui::Window::new("🔊 Channel rack — F6")
            .open(&mut open)
            .default_pos([340.0, 120.0])
            .default_size([820.0, 300.0])
            .min_width(680.0)
            .min_height(180.0)
            .resizable(true)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.menu_button("▾", |ui| {
                        for it in ["Rename (F2) — заглушка", "Fill each 2/4/8 — заглушка", "Clone (Alt+C) — заглушка"] {
                            if ui.button(it).clicked() {
                                self.hint = format!("«{it}»");
                            }
                        }
                    })
                    .response
                    .on_hover_text("Меню каналов");
                    egui::ComboBox::from_id_salt("rack_filter")
                        .selected_text("All")
                        .width(90.0)
                        .show_ui(ui, |ui| {
                            ui.selectable_label(true, "All");
                        });
                    ui.label(egui::RichText::new("Channel rack").strong());
                    ui.menu_button("+ Add channel", |ui| {
                        if ui.button("Sampler").clicked() {
                            self.engine.execute(Command::AddChannel {
                                kind: ChannelKind::Sampler,
                                name: "Sampler".into(),
                            });
                            ui.close_menu();
                        }
                        if ui.button("Synth / Generator").clicked() {
                            self.engine.execute(Command::AddChannel {
                                kind: ChannelKind::Synth,
                                name: "Generator".into(),
                            });
                            ui.close_menu();
                        }
                    })
                    .response
                    .on_hover_text("Добавить канал внизу списка (уникальный id, 16 шагов)");
                });
                ui.separator();
                egui::ScrollArea::vertical().show(ui, |ui| {
                    let m = self.engine.model_snapshot();
                    let pat = m.patterns.get(self.current_pattern).cloned();
                    for ch in &m.channels {
                        self.rack_row(ui, ch, pat.as_ref());
                        ui.separator();
                    }
                });
            });
        self.show_rack = open;
    }

    /// Ряд канала: фиксированные ширины колонок → шаги всех каналов строго
    /// друг под другом (одинаковая сетка).
    fn rack_row(&mut self, ui: &mut egui::Ui, ch: &Channel, pat: Option<&vibe_engine::model::Pattern>) {
        ui.horizontal(|ui| {
            // фиксированные колонки (общая сетка для всех каналов)
            ui.allocate_ui_with_layout(egui::vec2(20.0, 22.0), egui::Layout::left_to_right(egui::Align::Center), |ui| {
                let led = ui.add(egui::Button::new(egui::RichText::new("●").color(if ch.muted { C_LED_OFF } else { C_PLAY }).size(11.0)))
                    .on_hover_text("Mute канала (клик)");
                if led.clicked() {
                    self.engine.execute(Command::SetChannelMute { channel_id: ch.id, muted: !ch.muted });
                }
            });
            ui.allocate_ui_with_layout(egui::vec2(24.0, 22.0), egui::Layout::left_to_right(egui::Align::Center), |ui| {
                let mut vol = ch.volume;
                let r = knob_ui(ui, &mut vol, 0.0, 1.5, "Громкость канала — реально меняет звук");
                if r.drag_stopped() || r.dragged() {
                    self.engine.execute(Command::SetChannelVolume { channel_id: ch.id, volume: vol });
                }
            });
            ui.allocate_ui_with_layout(egui::vec2(24.0, 22.0), egui::Layout::left_to_right(egui::Align::Center), |ui| {
                let mut pan = ch.pan;
                let r = knob_ui(ui, &mut pan, -1.0, 1.0, "Pan канала — реально панорамирует (равная мощность)");
                if r.drag_stopped() || r.dragged() {
                    self.engine.execute(Command::SetChannelPan { channel_id: ch.id, pan });
                }
            });
            ui.allocate_ui_with_layout(egui::vec2(34.0, 22.0), egui::Layout::left_to_right(egui::Align::Center), |ui| {
                ui.label(egui::RichText::new(format!("→ M{}", (ch.id % 18) + 1)).monospace().size(11.0))
                    .on_hover_text("Routing канала (заглушка: все каналы в мастер)");
            });
            ui.allocate_ui_with_layout(egui::vec2(110.0, 22.0), egui::Layout::left_to_right(egui::Align::Center), |ui| {
                let kind_icon = match ch.kind {
                    ChannelKind::Sampler => "audio",
                    ChannelKind::Kick => "audio",
                    ChannelKind::Synth => "instrument",
                };
                icon_img(ui, &self.icons, kind_icon, 14.0);
                let col = self.channel_color(ch);
                let btn = ui.add(egui::Button::new(egui::RichText::new(format!(" {} ", ch.name)).color(Color32::WHITE)).fill(C_BTN));
                let brect = btn.rect;
                ui.painter().rect_filled(
                    egui::Rect::from_min_size(brect.left_top() + egui::vec2(2.0, 2.0), egui::vec2(4.0, brect.height() - 4.0)),
                    1.0,
                    col,
                );
                ui.painter().line_segment(
                    [brect.left_top() + egui::vec2(1.0, 1.0), brect.right_top() + egui::vec2(-1.0, 1.0)],
                    egui::Stroke::new(1.0_f32, C_BTN_HI),
                );
                self.set_hint(&btn, "ЛКМ — выбрать канал (Target Piano Roll); ПКМ — меню канала");
                if btn.clicked() {
                    self.active_channel = ch.id;
                }
                if btn.secondary_clicked() {
                    self.ctx_menu = Some((btn.interact_pointer_pos().unwrap_or(brect.center()), CtxTarget::Channel { id: ch.id }));
                }
            });
            ui.allocate_ui_with_layout(egui::vec2(9.0, 22.0), egui::Layout::left_to_right(egui::Align::Center), |ui| {
                let (srect, sresp) = ui.allocate_exact_size(egui::vec2(7.0, 22.0), Sense::click());
                let on = self.active_channel == ch.id;
                ui.painter().rect_filled(srect, 2.0, if on { C_PLAY } else { C_PANEL_DARK });
                self.set_hint(&sresp, "Channel Selector");
                if sresp.clicked() {
                    self.active_channel = ch.id;
                }
            });
            ui.separator();

            // шаги 1..16 — одна и та же сетка для всех каналов
            let pat_steps_on: Vec<u32> = pat
                .map(|p| {
                    p.steps
                        .iter()
                        .filter(|(ch_id, _)| *ch_id == ch.id)
                        .map(|(_, s)| *s)
                        .collect()
                })
                .unwrap_or_default();
            let cur_step = if self.status == TransportStatus::Playing {
                ((self.pos_ticks / STEP_TICKS as f64).floor() as u32) % 16
            } else {
                u32::MAX
            };
            for step in 0..16u32 {
                let on = pat_steps_on.contains(&step);
                let is_beat = step % 4 == 0;
                let fill = if on {
                    if is_beat { C_STEP_ON_BEAT } else { C_STEP_ON }
                } else if is_beat {
                    C_STEP_BEAT
                } else {
                    C_STEP_OFF
                };
                let (rect, resp) = ui.allocate_exact_size(egui::vec2(22.0, 20.0), Sense::click());
                ui.painter().rect_filled(rect, 2.0, fill);
                ui.painter().line_segment(
                    [rect.left_top() + egui::vec2(1.0, 0.5), rect.right_top() + egui::vec2(-1.0, 0.5)],
                    egui::Stroke::new(1.0_f32, C_BEVEL_HI),
                );
                ui.painter().line_segment(
                    [rect.left_bottom() + egui::vec2(1.0, -0.5), rect.right_bottom() + egui::vec2(-1.0, -0.5)],
                    egui::Stroke::new(1.0_f32, C_BEVEL_LO),
                );
                if cur_step == step {
                    ui.painter().rect_stroke(rect.expand(1.5), 3.0, egui::Stroke::new(2.0_f32, Color32::WHITE), egui::StrokeKind::Inside);
                }
                self.set_hint(&resp, "ЛКМ — шаг (нота 1/16 + звук), ПКМ — убрать");
                if resp.clicked() {
                    let pid = self.engine.model_snapshot().patterns.get(self.current_pattern).map(|p| p.id).unwrap_or(1);
                    self.engine.execute(Command::SetStep {
                        pattern_id: pid,
                        channel_id: ch.id,
                        step,
                        on: !on,
                    });
                    if !on {
                        self.engine.execute(Command::PreviewNote {
                            channel_id: ch.id,
                            key: 60,
                            vel: 100,
                            dur_secs: 0.3,
                        });
                    }
                }
                if resp.secondary_clicked() && on {
                    let pid = self.engine.model_snapshot().patterns.get(self.current_pattern).map(|p| p.id).unwrap_or(1);
                    self.engine.execute(Command::SetStep { pattern_id: pid, channel_id: ch.id, step, on: false });
                }
            }
        });
    }

    fn browser(&mut self, ctx: &egui::Context) {
        egui::SidePanel::left("browser")
            .resizable(true)
            .min_width(180.0)
            .default_width(215.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Browser").strong());
                });
                ui.separator();
                egui::ScrollArea::vertical().show(ui, |ui| {
                    let section = |ui: &mut egui::Ui, name: &str| {
                        ui.button(egui::RichText::new(format!("▾ {name}")).weak());
                    };
                    section(ui, "Generators");
                    ui.indent("gen", |ui| {
                        ui.horizontal(|ui| {
                            icon_img(ui, &self.icons, "instrument", 13.0);
                            if ui.button("Generator (Synth)").clicked() {
                                self.engine.execute(Command::AddChannel { kind: ChannelKind::Synth, name: "Generator".into() });
                            }
                        });
                    });
                    section(ui, "Recent files");
                    section(ui, "Plugin database");
                    section(ui, "My projects");
                    if let Some(f) = &self.current_file {
                        ui.indent("myproj", |ui| {
                            ui.label(egui::RichText::new(f.clone()).weak().small());
                        });
                    }
                    section(ui, "Packs");
                    ui.indent("packs", |ui| {
                        ui.label(egui::RichText::new("Drums / Kicks").weak().small());
                        if ui.button("909 Kick.wav").double_clicked() {
                            // двойной клик по WAV → канал-Sampler с сэмплом
                            let path = self.sample_search_path("909 Kick.wav");
                            let new_id = self.engine.model_snapshot().next_id;
                            self.engine.execute(Command::AddChannel {
                                kind: ChannelKind::Sampler,
                                name: "909 Kick".into(),
                            });
                            match self.engine.load_sample(new_id, &path) {
                                Ok(n) => self.hint = format!("Канал Sampler с 909 Kick.wav ({n} семплов)"),
                                Err(e) => self.hint = format!("WAV не загружен: {e} — канал создан (синт-фолбэк)"),
                            }
                        }
                        ui.label(egui::RichText::new("(двойной клик — создать Sampler-канал)").weak().small());
                    });
                });
            });
    }

    fn sample_search_path(&self, name: &str) -> String {
        // поиск фикстуры в репо (для прототипа)
        let candidates = [
            format!("{}/../../crates/flp_compat/tests/fixtures/{name}", env!("CARGO_MANIFEST_DIR")),
            format!("{name}"),
        ];
        for c in candidates {
            if std::path::Path::new(&c).exists() {
                return c;
            }
        }
        name.to_string()
    }

    fn desktop(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().frame(egui::Frame::NONE.fill(C_BG)).show(ctx, |ui| {
            if let Some(err) = &self.load_error {
                ui.label(egui::RichText::new(format!("Ошибка: {err}")).color(C_REC));
            }
            ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
                ui.add_space(ui.available_height() / 2.0 - 40.0);
                ui.label(egui::RichText::new("VibeStudio").size(38.0).weak().strong());
                ui.label(egui::RichText::new("Плавающие окна · F5/F6/F7/F9 · Space — play (звук работает)\nМодель проекта в движке · команды · undo/redo · экспорт WAV").weak().size(12.0));
            });
        });
    }

    fn status_bar(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::bottom("statusbar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("💡").size(11.0));
                ui.label(egui::RichText::new(&self.hint).weak().size(11.5));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(egui::RichText::new(VERSION_LINE).weak().size(11.5));
                });
            });
        });
    }

    fn playlist_window(&mut self, ctx: &egui::Context) {
        let mut open = self.show_playlist;
        // Ctrl+колесо — зум
        let wheel_zoom = ctx.input(|i| {
            if i.modifiers.command {
                i.raw_scroll_delta.y * 0.002
            } else {
                0.0
            }
        });
        if wheel_zoom != 0.0 {
            self.playlist_zoom = (self.playlist_zoom + wheel_zoom).clamp(0.4, 4.0);
        }
        let bar_w = 80.0 * self.playlist_zoom;
        const ROW_H: f32 = 52.0;
        const VISIBLE_TRACKS: usize = 9;
        let grid_w = SONG_BARS as f32 * bar_w;
        let grid_h = VISIBLE_TRACKS as f32 * ROW_H;

        egui::Window::new("🔊 Playlist")
            .open(&mut open)
            .default_pos([640.0, 260.0])
            .default_size([840.0, 470.0])
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    self.tool_buttons(ui);
                    ui.separator();
                    ui.label(egui::RichText::new("клипы паттернов; SONG играет их по позициям").weak().small());
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.add(egui::Slider::new(&mut self.playlist_zoom, 0.4..=4.0).show_value(false).text("🔍"))
                            .on_hover_text("Зум (Ctrl+колесо)");
                    });
                });
                ui.separator();
                egui::ScrollArea::both().show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.vertical(|ui| {
                            ui.set_min_width(110.0);
                            ui.label(egui::RichText::new("PICKER").weak().size(9.0));
                            let m = self.engine.model_snapshot();
                            let pat = m.patterns.get(self.current_pattern).cloned();
                            if let Some(p) = pat {
                                let chip = egui::Button::new(egui::RichText::new(format!("▶ {}", p.name)).color(Color32::WHITE))
                                    .fill(self.pattern_color(&m, p.id))
                                    .min_size(egui::vec2(100.0, 24.0));
                                if ui.add(chip).clicked() {
                                    self.hint = "Источник клипов: текущий паттерн".into();
                                }
                            }
                        });
                        ui.separator();
                        let ruler_h = 20.0;
                        let track_col_w = 72.0;
                        let (resp, painter) = ui.allocate_painter(egui::vec2(track_col_w + grid_w, ruler_h + grid_h), Sense::click_and_drag());
                        let origin = resp.rect.min + egui::vec2(track_col_w, ruler_h);
                        painter.rect_filled(egui::Rect::from_min_size(resp.rect.left_top(), egui::vec2(resp.rect.width(), ruler_h)), 0.0, C_PANEL);
                        for bar in 0..SONG_BARS {
                            let x = origin.x + bar as f32 * bar_w;
                            painter.line_segment([egui::pos2(x, resp.rect.top()), egui::pos2(x, origin.y + grid_h)], egui::Stroke::new(1.5_f32, C_GRID_STRONG));
                            painter.text(egui::pos2(x + 4.0, resp.rect.top() + 2.0), Align2::LEFT_TOP, format!("{}", bar + 1), egui::FontId::monospace(12.0), C_TEXT_DIM);
                            // доли тоньше (при зуме виднее)
                            if bar_w > 50.0 {
                                for b in 1..4 {
                                    let x = origin.x + (bar as f32 + b as f32 / 4.0) * bar_w;
                                    painter.line_segment([egui::pos2(x, origin.y), egui::pos2(x, origin.y + grid_h)], egui::Stroke::new(0.6_f32, C_GRID));
                                }
                            }
                        }
                        let m = self.engine.model_snapshot();
                        for tr in 0..VISIBLE_TRACKS {
                            let y = origin.y + tr as f32 * ROW_H;
                            let head = egui::Rect::from_min_size(egui::pos2(resp.rect.left(), y), egui::vec2(track_col_w - 8.0, ROW_H - 2.0));
                            painter.rect_filled(head, 2.0, TRACK_GROUP_COLORS[(tr / 4) % 4]);
                            painter.text(egui::pos2(resp.rect.left() + 6.0, y + ROW_H / 2.0), Align2::LEFT_CENTER, format!("Track {}", tr + 1), egui::FontId::proportional(12.5), Color32::WHITE);
                            let led_c = egui::pos2(resp.rect.left() + track_col_w - 4.0, y + ROW_H / 2.0);
                            let muted = m.muted_tracks.contains(&(tr as u32));
                            painter.circle_filled(led_c, 3.0, if muted { C_LED_OFF } else { C_PLAY });
                            let led_rect = egui::Rect::from_center_size(led_c, egui::vec2(12.0, ROW_H));
                            let led_resp = ui.interact(led_rect, egui::Id::new(("trled", tr)), Sense::click());
                            self.set_hint(&led_resp, "Mute LED трека: глушит все клипы дорожки (SONG)");
                            if led_resp.clicked() {
                                self.engine.execute(Command::SetTrackMute { track: tr as u32, muted: !muted });
                            }
                            painter.rect_filled(egui::Rect::from_min_size(egui::pos2(origin.x, y), egui::vec2(grid_w, ROW_H - 2.0)), 0.0, Color32::from_rgb(0x26, 0x28, 0x2B));
                        }
                        for b in 0..=(SONG_BARS * 4) {
                            let x = origin.x + b as f32 * (bar_w / 4.0);
                            let strong = b % 4 == 0;
                            painter.line_segment([egui::pos2(x, origin.y), egui::pos2(x, origin.y + grid_h)], egui::Stroke::new(if strong { 1.0 } else { 0.4_f32 }, if strong { C_GRID_STRONG } else { C_GRID }));
                        }
                        // клипы из модели
                        let clips = m.clips.clone();
                        for (i, c) in clips.iter().enumerate() {
                            if (c.track as usize) >= VISIBLE_TRACKS {
                                continue;
                            }
                            let dim = c.muted || m.muted_tracks.contains(&c.track);
                            let selected = self.selection.contains(&(c.track, c.bar));
                            let r = egui::Rect::from_min_size(
                                egui::pos2(origin.x + c.bar as f32 * bar_w + 1.0, origin.y + c.track as f32 * ROW_H + 1.0),
                                egui::vec2(c.len_bars as f32 * bar_w - 2.0, ROW_H - 4.0),
                            );
                            let col = self.pattern_color(&m, c.pattern_id);
                            painter.rect_filled(r, 3.0, if dim { col.gamma_multiply(0.35) } else { col });
                            if selected {
                                painter.rect_stroke(r, 3.0, egui::Stroke::new(2.0, Color32::WHITE), egui::StrokeKind::Inside);
                            }
                            let pname = m.patterns.iter().find(|p| p.id == c.pattern_id).map(|p| p.name.clone()).unwrap_or_default();
                            painter.text(egui::pos2(r.left() + 4.0, r.center().y), Align2::LEFT_CENTER, if c.muted { format!("🔉 {pname}") } else { pname }, egui::FontId::proportional(11.5), Color32::WHITE);
                            let clip_resp = ui.interact(r, egui::Id::new(("clip", i)), Sense::click_and_drag());
                            self.set_hint(&clip_resp, "Клип: drag — переместить, правый край — растянуть, ПКМ — меню");
                            if clip_resp.secondary_clicked() {
                                self.ctx_menu = Some((clip_resp.interact_pointer_pos().unwrap_or(r.center()), CtxTarget::Clip { track: c.track, bar: c.bar }));
                                break;
                            }
                            match self.tool {
                                Tool::Draw => {
                                    // drag выбранного клипа двигает всю группу выделения
                                    if clip_resp.drag_started() && selected && self.selection.len() > 1 {
                                        self.hint = format!("Перетаскивание группы: {} клипов", self.selection.len());
                                    }
                                    if clip_resp.dragged() {
                                        if let Some(p) = clip_resp.interact_pointer_pos() {
                                            let nb = (((p.x - origin.x) / bar_w) as u32).min((SONG_BARS - 1) as u32);
                                            let nt = (((p.y - origin.y) / ROW_H) as u32).min((VISIBLE_TRACKS - 1) as u32);
                                            if nb != c.bar || nt != c.track {
                                                let group: Vec<(u32, u32)> = if selected && self.selection.len() > 1 {
                                                    self.selection.iter().copied().collect()
                                                } else {
                                                    vec![(c.track, c.bar)]
                                                };
                                                let d_bar = nb as i64 - c.bar as i64;
                                                let d_tr = nt as i64 - c.track as i64;
                                                for (t0, b0) in group {
                                                    let nt2 = ((t0 as i64 + d_tr).max(0)) as u32;
                                                    let nb2 = ((b0 as i64 + d_bar).max(0)) as u32;
                                                    if (nt2, nb2) != (t0, b0) {
                                                        self.engine.execute(Command::MoveClip { track: t0, bar: b0, new_track: nt2, new_bar: nb2 });
                                                    }
                                                }
                                            }
                                        }
                                    }
                                    // resize за правый край
                                    let edge = egui::Rect::from_min_size(r.right_center() - egui::vec2(6.0, ROW_H / 2.0), egui::vec2(8.0, ROW_H));
                                    let edge_resp = ui.interact(edge, egui::Id::new(("clipedge", i)), Sense::drag());
                                    self.set_hint(&edge_resp, "Растянуть клип за правый край");
                                    if edge_resp.dragged() {
                                        if let Some(p) = edge_resp.interact_pointer_pos() {
                                            let nb = (((p.x - origin.x) / bar_w) as u32) + 1;
                                            if nb != c.len_bars {
                                                self.engine.execute(Command::ResizeClip { track: c.track, bar: c.bar, new_len_bars: nb.max(1) });
                                            }
                                        }
                                    }
                                }
                                Tool::Delete => {
                                    if clip_resp.clicked() {
                                        self.engine.execute(Command::RemoveClip { track: c.track, bar: c.bar });
                                    }
                                }
                                Tool::Select => {
                                    if clip_resp.clicked() {
                                        self.selection.insert((c.track, c.bar));
                                    }
                                }
                                Tool::Mute => {
                                    if clip_resp.clicked() {
                                        self.engine.execute(Command::SetClipMute { track: c.track, bar: c.bar, muted: !c.muted });
                                    }
                                }
                            }
                        }
                        // playhead
                        if self.status == TransportStatus::Playing || self.pos_ticks > 0.0 {
                            let x = origin.x + (self.pos_ticks as f32 / (SONG_BARS as f32 * 384.0)) * grid_w;
                            painter.line_segment([egui::pos2(x, origin.y), egui::pos2(x, origin.y + grid_h)], egui::Stroke::new(1.5_f32, C_PLAY));
                        }
                        // SELECT-рамка: drag по сетке рисует прямоугольник выделения
                        if self.tool == Tool::Select {
                            if resp.drag_started() {
                                self.select_rect = resp.interact_pointer_pos().map(|p| (p, p));
                                self.selection.clear();
                            }
                            if resp.dragged() {
                                if let (Some((_, _)), Some(p)) = (self.select_rect, resp.interact_pointer_pos()) {
                                    if let Some((_, _)) = self.select_rect.as_mut() {
                                        self.select_rect = Some((self.select_rect.unwrap().0, p));
                                    }
                                }
                            }
                            if let Some((a, b)) = self.select_rect {
                                let rect = egui::Rect::from_two_pos(a, b);
                                painter.rect_filled(rect, 1.0, C_ACCENT.gamma_multiply(0.15));
                                painter.rect_stroke(rect, 1.0, egui::Stroke::new(1.5, C_ACCENT), egui::StrokeKind::Inside);
                            }
                            if resp.drag_stopped() {
                                if let Some((a, b)) = self.select_rect {
                                    let sel = egui::Rect::from_two_pos(a, b);
                                    let m = self.engine.model_snapshot();
                                    for c in &m.clips {
                                        if (c.track as usize) >= VISIBLE_TRACKS {
                                            continue;
                                        }
                                        let cr = egui::Rect::from_min_size(
                                            egui::pos2(origin.x + c.bar as f32 * bar_w, origin.y + c.track as f32 * ROW_H),
                                            egui::vec2(c.len_bars as f32 * bar_w, ROW_H),
                                        );
                                        if sel.intersects(cr) {
                                            self.selection.insert((c.track, c.bar));
                                        }
                                    }
                                    if self.selection.is_empty() {
                                        self.hint = "Выделение пустое".into();
                                    } else {
                                        self.hint = format!("Выделено клипов: {} (Del — удалить, drag — переместить)", self.selection.len());
                                    }
                                }
                                self.select_rect = None;
                            }
                        } else {
                            self.select_rect = None;
                        }
                        // Del — удалить выделенные
                        if self.tool == Tool::Select
                            && ui.input(|i| i.key_pressed(egui::Key::Delete))
                            && !self.selection.is_empty()
                        {
                            for (tr, bar) in self.selection.drain() {
                                self.engine.execute(Command::RemoveClip { track: tr, bar });
                            }
                            self.hint = "Выделенные клипы удалены".into();
                        }

                        // сетка: draw/delete/seek
                        if let Some(pos) = resp.interact_pointer_pos() {
                            if pos.x >= origin.x && pos.y >= origin.y {
                                let bar_f = (pos.x - origin.x) / bar_w;
                                let bar = ((bar_f as usize).min(SONG_BARS - 1)) as u32;
                                let tr = (((pos.y - origin.y) / ROW_H) as usize).min(VISIBLE_TRACKS - 1);
                                if pos.y < origin.y && resp.clicked() {
                                    self.pos_ticks = bar as f64 * 384.0;
                                    self.engine.execute(Command::Seek { tick: self.pos_ticks as f32 });
                                } else if pos.y >= origin.y {
                                    if resp.secondary_clicked() {
                                        self.ctx_menu = Some((pos, CtxTarget::PlaylistEmpty { track: tr as u32, bar }));
                                    } else if resp.clicked() {
                                    match self.tool {
                                        Tool::Draw => {
                                            let m = self.engine.model_snapshot();
                                            let pid = m.patterns.get(self.current_pattern).map(|p| p.id).unwrap_or(1);
                                            if !m.clips.iter().any(|c| c.track as usize == tr && c.bar == bar) {
                                                self.engine.execute(Command::AddClip { track: tr as u32, bar, len_bars: 1, pattern_id: pid });
                                                self.hint = "Клип добавлен (играет в SONG)".into();
                                            }
                                        }
                                        Tool::Delete => {
                                            let m = self.engine.model_snapshot();
                                            if let Some(c) = m.clips.iter().find(|c| c.track as usize == tr && c.bar == bar).cloned() {
                                                self.engine.execute(Command::RemoveClip { track: c.track, bar: c.bar });
                                            }
                                        }
                                        Tool::Select => {
                                            self.selection.clear();
                                        }
                                        Tool::Mute => {
                                            self.hint = "Mute: кликайте по клипам (не по пустому месту)".into();
                                        }
                                    }
                                }
                            }
                            }
                        }
                    });
                });
            });
        self.show_playlist = open;
    }

    fn piano_roll_window(&mut self, ctx: &egui::Context) {
        let mut open = self.show_roll;
        const KEY_TOP: u16 = 84;
        const KEY_BOTTOM: u16 = 36;
        const ROW_H: f32 = 11.0;
        const STEPS: u32 = 16;
        const PX_PER_STEP: f32 = 34.0;
        let width = (STEPS as f32 * PX_PER_STEP) as usize;
        let m = self.engine.model_snapshot();
        let target = m.channels.iter().find(|c| c.id == self.active_channel).cloned()
            .or_else(|| m.channels.first().cloned());
        let Some(target_ch) = target else { self.show_roll = open; return; };
        let pat = m.patterns.get(self.current_pattern).cloned();
        let pid = pat.as_ref().map(|p| p.id).unwrap_or(1);

        let title = format!("🎹 Piano Roll - {} ▸ {}", pat.as_ref().map(|p| p.name.clone()).unwrap_or_default(), target_ch.name);
        egui::Window::new(title)
            .open(&mut open)
            .default_pos([430.0, 320.0])
            .default_size([940.0, 480.0])
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Target Channel:");
                    egui::ComboBox::from_id_salt("roll_target")
                        .selected_text(target_ch.name.clone())
                        .show_ui(ui, |ui| {
                            for ch in &m.channels {
                                ui.selectable_value(&mut self.active_channel, ch.id, ch.name.clone());
                            }
                        });
                    ui.separator();
                    let d = ui.selectable_label(self.tool == Tool::Draw, "✎ Draw");
                    if d.clicked() {
                        self.tool = Tool::Draw;
                    }
                    let del = ui.selectable_label(self.tool == Tool::Delete, "⌫ Delete");
                    if del.clicked() {
                        self.tool = Tool::Delete;
                    }
                    ui.separator();
                    ui.label(egui::RichText::new(format!("snap {}", SNAP_OPTIONS[self.snap])).weak().small());
                });
                ui.separator();
                let ruler_h = 16.0;
                let keys_w = 52.0;
                let grid_h = (KEY_TOP - KEY_BOTTOM + 1) as f32 * ROW_H;
                let (resp, painter) = ui.allocate_painter(egui::vec2(keys_w + width as f32, ruler_h + grid_h), Sense::click_and_drag());
                let origin = resp.rect.min + egui::vec2(keys_w, ruler_h);
                let key_y = |key: u16| origin.y + (KEY_TOP - key) as f32 * ROW_H;
                for s in 0..=STEPS {
                    let x = origin.x + s as f32 * PX_PER_STEP;
                    let strong = s % 4 == 0;
                    painter.line_segment([egui::pos2(x, resp.rect.top()), egui::pos2(x, resp.rect.bottom())], egui::Stroke::new(if strong { 1.0 } else { 0.5_f32 }, if strong { C_GRID_STRONG } else { C_GRID }));
                    if strong {
                        painter.text(egui::pos2(x + 3.0, resp.rect.top() + 1.0), Align2::LEFT_TOP, format!("{}", s / 4 + 1), egui::FontId::monospace(10.0), C_TEXT_DIM);
                    }
                }
                for key in KEY_BOTTOM..=KEY_TOP {
                    let y = key_y(key);
                    let is_black = matches!(key % 12, 1 | 3 | 6 | 8 | 10);
                    painter.rect_filled(egui::Rect::from_min_size(egui::pos2(resp.rect.left(), y), egui::vec2(keys_w - 2.0, ROW_H)), 1.0, if is_black { C_KEY_BLACK } else { C_KEY_WHITE });
                    if key % 12 == 0 {
                        painter.text(egui::pos2(resp.rect.left() + 4.0, y + ROW_H / 2.0), Align2::LEFT_CENTER, format!("C{}", key / 12 - 1), egui::FontId::monospace(9.0), Color32::BLACK);
                    }
                    painter.rect_filled(egui::Rect::from_min_size(egui::pos2(origin.x, y), egui::vec2(width as f32, ROW_H)), 0.0, if is_black { Color32::from_rgb(0x2C, 0x2F, 0x33) } else { Color32::from_rgb(0x35, 0x39, 0x3E) });
                }
                // ноты целевого канала + ghost чужих
                let notes: Vec<(usize, ModelNote)> = pat
                    .as_ref()
                    .map(|p| {
                        p.notes
                            .iter()
                            .enumerate()
                            .filter(|(_, (ch_id, _))| *ch_id == self.active_channel)
                            .map(|(i, (_, n))| (i, *n))
                            .collect()
                    })
                    .unwrap_or_default();
                let col = self.channel_color(&target_ch);
                for (_, n) in &notes {
                    let x = origin.x + n.step as f32 * PX_PER_STEP;
                    let y = key_y(n.key);
                    let r = egui::Rect::from_min_size(egui::pos2(x, y + 1.0), egui::vec2(n.len_steps as f32 * PX_PER_STEP, ROW_H - 2.0));
                    painter.rect_filled(r, 2.0, col);
                    if n.len_steps as f32 * PX_PER_STEP > 24.0 {
                        let names = ["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"];
                        painter.text(egui::pos2(r.left() + 3.0, r.center().y), Align2::LEFT_CENTER, format!("{}{}", names[(n.key % 12) as usize], n.key / 12 - 1), egui::FontId::monospace(8.0), Color32::from_rgb(0x1E, 0x28, 0x1C));
                    }
                }
                // playhead по шагам
                if self.status == TransportStatus::Playing {
                    let x = origin.x + ((self.pos_ticks / STEP_TICKS as f64) % STEPS as f64) as f32 * PX_PER_STEP;
                    painter.line_segment([egui::pos2(x, origin.y), egui::pos2(x, origin.y + grid_h)], egui::Stroke::new(1.5_f32, C_PLAY));
                }
                // редактирование (Draw: клик/драг; Delete)
                let snap_steps = (self.snap_ticks() as f32 / STEP_TICKS).max(1.0);
                let grid = |pos: egui::Pos2| -> (u32, u16) {
                    let step = (((pos.x - origin.x) / PX_PER_STEP / snap_steps).floor() * snap_steps) as u32;
                    let key = KEY_TOP - ((pos.y - origin.y) / ROW_H).floor() as u16;
                    (step.min(STEPS - 1), key)
                };
                if self.tool == Tool::Delete {
                    if let Some(pos) = resp.interact_pointer_pos() {
                        if pos.x >= origin.x && pos.y >= origin.y && ui.input(|i| i.pointer.primary_down()) {
                            let (step, key) = grid(pos);
                            if let Some((idx, _)) = notes.iter().find(|(_, n)| n.step == step && n.key == key) {
                                self.engine.execute(Command::RemoveNote { pattern_id: pid, channel_id: self.active_channel, step });
                                let _ = idx;
                            }
                        }
                    }
                } else if resp.drag_started() {
                    if let Some(pos) = resp.interact_pointer_pos() {
                        let (step, key) = grid(pos);
                        let hit = notes.iter().find(|(_, n)| n.step == step && n.key == key).copied();
                        match hit {
                            Some((idx, n)) => {
                                self.selected_note = Some(idx);
                                self.roll_drag = Some(RollDrag::Move { idx, dstep: n.step as i32 - step as i32, dkey: n.key as i16 - key as i16 });
                            }
                            None => {
                                self.engine.execute(Command::SetNote {
                                    pattern_id: pid,
                                    channel_id: self.active_channel,
                                    note: ModelNote { step, key, vel: 100, len_steps: self.last_note_len },
                                });
                                self.engine.execute(Command::PreviewNote { channel_id: self.active_channel, key, vel: 100, dur_secs: 0.3 });
                                self.hint = "Нота создана".into();
                            }
                        }
                    }
                } else if let Some(d) = self.roll_drag {
                    if ui.input(|i| i.pointer.primary_down()) {
                        if let Some(pos) = resp.interact_pointer_pos() {
                            let (step, key) = grid(pos);
                            match d {
                                RollDrag::Move { idx, dstep, dkey } => {
                                    if let Some((_, n)) = notes.get(idx) {
                                        let ns = (step as i32 + dstep).max(0) as u32;
                                        let nk = (key as i32 + i32::from(dkey)).clamp(0, 127) as u16;
                                        let moved = ModelNote { step: ns, key: nk, vel: n.vel, len_steps: n.len_steps };
                                        self.engine.execute(Command::RemoveNote { pattern_id: pid, channel_id: self.active_channel, step: n.step });
                                        self.engine.execute(Command::SetNote { pattern_id: pid, channel_id: self.active_channel, note: moved });
                                    }
                                }
                                RollDrag::Resize { idx } => {
                                    if let Some((_, n)) = notes.get(idx) {
                                        let len = (step.saturating_sub(n.step)).max(1);
                                        self.engine.execute(Command::SetNote {
                                            pattern_id: pid,
                                            channel_id: self.active_channel,
                                            note: ModelNote { step: n.step, key: n.key, vel: n.vel, len_steps: len },
                                        });
                                        self.last_note_len = len;
                                    }
                                }
                            }
                        }
                    } else {
                        self.roll_drag = None;
                    }
                }
                // resize: правый край ноты (простая версия — dragStarted на краю)
                // velocity lane
                ui.separator();
                let lane_h = 48.0;
                let (lane_rect, lane_resp) = ui.allocate_exact_size(egui::vec2(keys_w + width as f32, lane_h), Sense::drag());
                let lp = ui.painter_at(lane_rect);
                lp.rect_filled(lane_rect, 1.0, C_PANEL_DARK);
                lp.text(egui::pos2(lane_rect.left() + 4.0, lane_rect.top() + 2.0), Align2::LEFT_TOP, "Velocity", egui::FontId::monospace(9.0), C_TEXT_DIM);
                let lane_origin = lane_rect.min + egui::vec2(keys_w, 0.0);
                for (_, n) in &notes {
                    let x = lane_origin.x + n.step as f32 * PX_PER_STEP;
                    let w = (n.len_steps as f32 * PX_PER_STEP).max(3.0);
                    let h = (n.vel as f32 / 127.0) * (lane_h - 12.0);
                    lp.rect_filled(egui::Rect::from_min_size(egui::pos2(x, lane_rect.bottom() - h - 2.0), egui::vec2(w, h)), 1.0, col);
                }
                if lane_resp.dragged() {
                    if let Some(p) = lane_resp.interact_pointer_pos() {
                        if p.x >= lane_origin.x && p.y >= lane_rect.top() && p.x <= lane_rect.right() {
                            let vel = (((lane_rect.bottom() - 2.0 - p.y) / (lane_h - 12.0)).clamp(0.0, 1.0) * 127.0) as u8;
                            let step = ((p.x - lane_origin.x) / PX_PER_STEP) as u32;
                            for (_, n) in &notes {
                                if n.step == step {
                                    self.engine.execute(Command::SetNote {
                                        pattern_id: pid,
                                        channel_id: self.active_channel,
                                        note: ModelNote { step: n.step, key: n.key, vel, len_steps: n.len_steps },
                                    });
                                }
                            }
                            self.hint = format!("Velocity → {vel}");
                        }
                    }
                }
                lane_resp.on_hover_text("Velocity lane: драг по бару ноты меняет velocity");
            });
        self.show_roll = open;
    }

    fn mixer_window(&mut self, ctx: &egui::Context) {
        let mut open = self.show_mixer;
        egui::Window::new("🎛 Mixer — F9")
            .open(&mut open)
            .default_pos([380.0, 240.0])
            .default_size([1000.0, 400.0])
            .show(ctx, |ui| {
                ui.label(egui::RichText::new("Каналы проекта + Master. Фейдеры пишут volume/pan в модель (звук). Метры — из атомиков движка.").weak().small());
                ui.separator();
                egui::ScrollArea::horizontal().show(ui, |ui| {
                    ui.horizontal(|ui| {
                        let m = self.engine.model_snapshot();
                        let master_peak = self.engine.peak();
                        let active = self.engine.active_mask();
                        // Master
                        let master_vol = m.master_volume;
                        self.mixer_strip(ui, 0, "Master", master_vol, 0.5, false, master_peak, true, &mut |this, vol, pan| {
                            this.engine.execute(Command::SetMasterVolume { volume: vol });
                            let _ = pan;
                        });
                        ui.separator();
                        // каналы (первые 10)
                        for ch in m.channels.iter().take(10) {
                            let active = active & (1u32 << ch.id.min(31)) != 0;
                            let peak = if active { master_peak * ch.volume } else { 0.0 };
                            let name = ch.name.clone();
                            let id = ch.id;
                            let muted = ch.muted;
                            let pan = ch.pan;
                            self.mixer_strip(ui, id as usize, &name, ch.volume, pan, muted, peak, false, &mut |this, vol, pan| {
                                this.engine.execute(Command::SetChannelVolume { channel_id: id, volume: vol });
                                this.engine.execute(Command::SetChannelPan { channel_id: id, pan });
                                let _ = muted;
                            });
                            ui.separator();
                        }
                    });
                });
            });
        self.show_mixer = open;
    }

    #[allow(clippy::too_many_arguments)]
    fn mixer_strip(
        &mut self,
        ui: &mut egui::Ui,
        idx: usize,
        name: &str,
        vol: f32,
        pan: f32,
        _muted: bool,
        peak: f32,
        is_master: bool,
        on_change: &mut dyn FnMut(&mut Self, f32, f32),
    ) {
        let selected = if is_master { idx == 0 } else { false };
        let fill = if selected { Color32::from_rgb(0x3B, 0x4A, 0x35) } else { Color32::from_rgb(0x2C, 0x2E, 0x31) };
        egui::Frame::default().fill(fill).inner_margin(6.0).show(ui, |ui| {
            ui.set_width(74.0);
            ui.set_min_height(300.0);
            ui.vertical(|ui| {
                ui.label(egui::RichText::new(name).size(10.5).strong());
                ui.with_layout(egui::Layout::bottom_up(egui::Align::Center), |ui| {
                    let (mrect, _) = ui.allocate_exact_size(egui::vec2(30.0, 90.0), Sense::hover());
                    let pt = ui.painter_at(mrect);
                    pt.rect_filled(mrect, 1.0, C_BG);
                    for ch in 0..2 {
                        let h = peak * mrect.height() * if ch == 0 { 0.95 } else { 0.8 };
                        pt.rect_filled(egui::Rect::from_min_size(egui::pos2(mrect.left() + 4.0 + ch as f32 * 12.0, mrect.bottom() - h), egui::vec2(9.0, h)), 1.0, if peak > 0.92 { C_REC } else { C_PLAY });
                    }
                    let mut v = vol;
                    let vr = ui.add_sized([70.0, 16.0], egui::Slider::new(&mut v, 0.0..=1.0).text("Vol"))
                        .on_hover_text("Громкость канала (реально меняет звук)");
                    if vr.drag_stopped() || vr.dragged() {
                        on_change(self, v, pan);
                    }
                    let mut p = pan;
                    let pr = ui.add_sized([70.0, 16.0], egui::Slider::new(&mut p, -1.0..=1.0).text("Pan"))
                        .on_hover_text("Панорама (реально)");
                    if pr.drag_stopped() || pr.dragged() {
                        on_change(self, v, p);
                    }
                });
            });
        });
    }

    fn plugin_picker(&mut self, ctx: &egui::Context) {
        let mut open = self.show_picker;
        egui::Window::new("Plugin Picker — F8")
            .open(&mut open)
            .default_pos([520.0, 260.0])
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.label(egui::RichText::new("Generators").strong().size(10.0));
                        for nm in ["Synth (saw)", "Kick (synth)"] {
                            ui.horizontal(|ui| {
                                icon_img(ui, &self.icons, "instrument", 13.0);
                                if ui.button(nm).clicked() {
                                    let kind = if nm.contains("Kick") { ChannelKind::Kick } else { ChannelKind::Synth };
                                    self.engine.execute(Command::AddChannel { kind, name: nm.split('(').next().unwrap_or("Gen").trim().into() });
                                }
                            });
                        }
                    });
                    ui.separator();
                    ui.vertical(|ui| {
                        ui.label(egui::RichText::new("Effects").strong().size(10.0));
                        ui.label(egui::RichText::new("эффекты появятся с FX-хостом").weak().small());
                    });
                });
            });
        self.show_picker = open;
    }

    fn typing_piano_window(&mut self, ctx: &egui::Context) {
        let mut open = self.show_piano;
        egui::Window::new("🎹 Typing keyboard to piano — Ctrl+T")
            .open(&mut open)
            .default_pos([560.0, 430.0])
            .show(ctx, |ui| {
                let ch_name = self.engine.model_snapshot().channels.iter().find(|c| c.id == self.active_channel).map(|c| c.name.clone()).unwrap_or_default();
                ui.label(egui::RichText::new(format!("A S D F G H J K = C4..C5 · канал: {ch_name}")).weak().small());
                ui.horizontal(|ui| {
                    let keys = [
                        (Key::A, 60usize, "A"), (Key::S, 62, "S"), (Key::D, 64, "D"), (Key::F, 65, "F"),
                        (Key::G, 67, "G"), (Key::H, 69, "H"), (Key::J, 71, "J"), (Key::K, 72, "K"),
                    ];
                    let mut hit = None;
                    for (i, (k, _, label)) in keys.iter().enumerate() {
                        let pressed = ui.input(|i| i.key_pressed(*k));
                        if pressed {
                            hit = Some(i);
                        }
                        if ui.add(egui::Button::new(egui::RichText::new(*label).size(14.0)).min_size(egui::vec2(44.0, 48.0)).fill(if pressed { C_ACCENT } else { C_PANEL_DARK })).clicked() {
                            hit = Some(i);
                        }
                    }
                    if let Some(i) = hit {
                        self.engine.execute(Command::PreviewNote { channel_id: self.active_channel, key: keys[i].1 as u16, vel: 100, dur_secs: 0.4 });
                    }
                });
            });
        self.show_piano = open;
    }

    /// Контекстное меню правой кнопки (как в FL): позиция у курсора,
    /// закрывается кликом вне; пункты-заглушки — с запрещающим курсором.
    fn context_menu_window(&mut self, ctx: &egui::Context) {
        let Some((pos, target)) = self.ctx_menu else { return };
        let mut keep_open = true;
        let mut rect_out = None;
        egui::Area::new(egui::Id::new("ctx_menu"))
            .fixed_pos(pos)
            .order(egui::Order::Tooltip)
            .show(ctx, |ui| {
                egui::Frame::popup(ui.style()).show(ui, |ui| {
                    ui.set_min_width(190.0);
                    match target {
                        CtxTarget::Clip { track, bar } => {
                            let m = self.engine.model_snapshot();
                            let pid = m
                                .clips
                                .iter()
                                .find(|c| c.track == track && c.bar == bar)
                                .map(|c| c.pattern_id);
                            let pname = pid
                                .and_then(|id| {
                                    m.patterns.iter().find(|p| p.id == id).map(|p| p.name.clone())
                                })
                                .unwrap_or_default();
                            ui.label(egui::RichText::new(format!("Клип: {pname}")).weak().small());
                            ui.separator();
                            self.stub_item(ui, "Preview");
                            self.stub_item(ui, "Rename / color…");
                            self.stub_item(ui, "Make unique");
                            if let Some(id) = pid {
                                if ui.button("Select all similar").clicked() {
                                    self.tool = Tool::Select;
                                    self.selection = m
                                        .clips
                                        .iter()
                                        .filter(|c| c.pattern_id == id)
                                        .map(|c| (c.track, c.bar))
                                        .collect();
                                    self.hint =
                                        format!("Выделено похожих клипов: {}", self.selection.len());
                                    keep_open = false;
                                }
                            }
                            ui.separator();
                            let is_muted = m
                                .clips
                                .iter()
                                .find(|c| c.track == track && c.bar == bar)
                                .map(|c| c.muted)
                                .unwrap_or(false);
                            if ui
                                .button(if is_muted { "🔊 Unmute" } else { "🔇 Mute" })
                                .clicked()
                            {
                                self.engine
                                    .execute(Command::SetClipMute { track, bar, muted: !is_muted });
                                keep_open = false;
                            }
                            if ui
                                .button(egui::RichText::new("🗑 Delete").color(C_REC))
                                .clicked()
                            {
                                self.engine.execute(Command::RemoveClip { track, bar });
                                self.selection.remove(&(track, bar));
                                keep_open = false;
                            }
                        }
                        CtxTarget::Channel { id } => {
                            let name = self
                                .engine
                                .model_snapshot()
                                .channels
                                .iter()
                                .find(|c| c.id == id)
                                .map(|c| c.name.clone())
                                .unwrap_or_default();
                            ui.label(egui::RichText::new(format!("Канал: {name}")).weak().small());
                            ui.separator();
                            if ui.button("🎹 Piano roll").clicked() {
                                self.active_channel = id;
                                self.show_roll = true;
                                keep_open = false;
                            }
                            self.stub_item(ui, "Rename / color / icon (F2)");
                            self.stub_item(ui, "Load sample…");
                            self.stub_item(ui, "Clone (Alt+C)");
                            ui.separator();
                            if ui
                                .button(egui::RichText::new("🗑 Delete…").color(C_REC))
                                .clicked()
                            {
                                self.confirm_delete_channel = Some(id);
                                keep_open = false;
                            }
                        }
                        CtxTarget::PlaylistEmpty { track, bar } => {
                            ui.label(egui::RichText::new(format!(
                                "Track {} · такт {}",
                                track + 1,
                                bar + 1
                            ))
                            .weak()
                            .small());
                            ui.separator();
                            if ui.button("Insert pattern clip").clicked() {
                                let m = self.engine.model_snapshot();
                                let pid = m
                                    .patterns
                                    .get(self.current_pattern)
                                    .map(|p| p.id)
                                    .unwrap_or(1);
                                self.engine.execute(Command::AddClip {
                                    track,
                                    bar,
                                    len_bars: 1,
                                    pattern_id: pid,
                                });
                                keep_open = false;
                            }
                            self.stub_item(ui, "Paste");
                            self.stub_item(ui, "Insert time…");
                        }
                    }
                });
                rect_out = Some(ui.min_rect());
            });
        self.ctx_menu_rect = rect_out;
        if keep_open {
            let clicked = ctx.input(|i| i.pointer.primary_clicked());
            let inside = ctx
                .input(|i| i.pointer.interact_pos())
                .zip(self.ctx_menu_rect)
                .map(|(p, r)| r.contains(p))
                .unwrap_or(false);
            if clicked && !inside {
                keep_open = false;
            }
        }
        if !keep_open {
            self.ctx_menu = None;
        }
    }

    fn confirm_delete_window(&mut self, ctx: &egui::Context) {
        if let Some(iid) = self.confirm_delete_channel {
            let mut open = true;
            let name = self.engine.model_snapshot().channels.iter().find(|c| c.id == iid).map(|c| c.name.clone()).unwrap_or_default();
            egui::Window::new(format!("Удалить канал {name}?"))
                .open(&mut open)
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    ui.label("Канал и его ноты/шаги будут удалены (Ctrl+Z вернёт).");
                    ui.add_space(6.0);
                    ui.horizontal(|ui| {
                        if ui.button(egui::RichText::new("Удалить").color(C_REC)).clicked() {
                            self.engine.execute(Command::RemoveChannel { channel_id: iid });
                            self.confirm_delete_channel = None;
                        }
                        if ui.button("Отмена").clicked() {
                            self.confirm_delete_channel = None;
                        }
                    });
                });
            if !open {
                self.confirm_delete_channel = None;
            }
        }
    }
}
