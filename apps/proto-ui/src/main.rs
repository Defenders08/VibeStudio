//! VibeStudio — рабочий прототип FL-подобного интерфейса (egui).
//!
//! UX повторяет FL Studio 20.8 (референс-скриншот + research/fl-studio-clean-room):
//! однорядный тулбар с transport-кластером, вторая строка (паттерн/snap/окна),
//! Browser прикован слева, Channel Rack прикован сверху, Playlist/Piano Roll/Mixer —
//! плавающие окна, внизу статусная строка с версией.
//!
//! Реальные данные: base_one_pattern.flp разбирается крейтом flp_compat.
//! Всё, для чего нет движка (звук, сохранение, плагины), — интерактивные
//! заглушки в памяти. Дизайн (цвета) — собственный тёмный, не копия FL.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use eframe::egui;
use egui::{Align2, Color32, Key, Sense};
use flp_compat::model::{color_to_rgb8, Arrangement, Channel, Insert, Project};
use std::collections::{HashMap, HashSet};
use std::time::Instant;
use vibe_engine::{render_to_wav, Engine, NoteEvent, VoiceSrc};

/// Собственные иконки VibeStudio (icons/own/, генерируются icons/gen_icons.py —
/// монохромные глифы в духе FL; замена иконок Image-Line из соображений лицензий).
const ICONS_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/icons/own");

const PPQ: f32 = 96.0; // PPQ фикстуры (из FLhd)
const BAR_TICKS: f32 = 4.0 * PPQ;
const PATTERN_TICKS: f32 = BAR_TICKS; // паттерн длины 0 = один такт
const SONG_BARS: usize = 16;
const STEP_TICKS: f32 = PPQ / 4.0; // шаг = 1/16
const VERSION_LINE: &str =
    "VibeStudio Prototype v0.2.0 — flp_compat · vibe_engine (cpal) · звук включён";

// ---- Палитра FL 21 (Platinum-скин), снята со скриншотов мануала ----
const C_BG: Color32 = Color32::from_rgb(0x23, 0x25, 0x27); // рабочий стол
const C_PANEL: Color32 = Color32::from_rgb(0x4A, 0x4D, 0x52); // панели/тулбар ряд 2
const C_PANEL_DARK: Color32 = Color32::from_rgb(0x2E, 0x30, 0x33); // вставки/LCD/шапки
const C_WINDOW: Color32 = Color32::from_rgb(0x3E, 0x41, 0x45); // заливка плавающих окон
const C_GRID: Color32 = Color32::from_rgb(0x43, 0x46, 0x4B); // линии 1/16
const C_GRID_STRONG: Color32 = Color32::from_rgb(0x5A, 0x5D, 0x62); // линии тактов
const C_TEXT_DIM: Color32 = Color32::from_rgb(0x9E, 0xA1, 0xA5);
const C_ACCENT: Color32 = Color32::from_rgb(0xF0, 0xA4, 0x4A); // янтарный PAT/выделение
const C_PLAY: Color32 = Color32::from_rgb(0x96, 0xD9, 0x46); // зелёный LED FL
const C_LED_OFF: Color32 = Color32::from_rgb(0x3A, 0x4A, 0x3A);
const C_REC: Color32 = Color32::from_rgb(0xE0, 0x43, 0x3E);
const C_STEP_OFF: Color32 = Color32::from_rgb(0x43, 0x2F, 0x31); // тёмный мармелад
const C_STEP_BEAT: Color32 = Color32::from_rgb(0x4E, 0x38, 0x3A);
const C_STEP_ON: Color32 = Color32::from_rgb(0xDE, 0x8F, 0x8A); // зажатая клетка
const C_STEP_ON_BEAT: Color32 = Color32::from_rgb(0xE8, 0xA4, 0x9E);
const C_BEVEL_HI: Color32 = Color32::from_rgb(0x5E, 0x61, 0x66); // светлая грань 3D
const C_BEVEL_LO: Color32 = Color32::from_rgb(0x26, 0x28, 0x2A); // тёмная грань 3D
const C_BTN: Color32 = Color32::from_rgb(0x56, 0x5A, 0x5F); // кнопка канала
const C_BTN_HI: Color32 = Color32::from_rgb(0x6A, 0x6E, 0x74);
const C_KEY_BLACK: Color32 = Color32::from_rgb(0x3A, 0x3D, 0x40);
const C_KEY_WHITE: Color32 = Color32::from_rgb(0xDC, 0xDC, 0xDC);
const C_LCD_BG: Color32 = Color32::from_rgb(0x17, 0x18, 0x1A);
const C_LCD_TEXT: Color32 = Color32::from_rgb(0xF2, 0xF2, 0xF2);
const C_COMBO_LIGHT: Color32 = Color32::from_rgb(0xC9, 0xCC, 0xCE); // светлый бокс Pattern
// Цвета групп треков Playlist (FL красит по 4 трека: зелёный/красный/фиолетовый/янтарный)
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

/// Загрузить PNG из icons/own/ в текстуру egui.
fn load_icon(ctx: &egui::Context, name: &str) -> Option<egui::TextureHandle> {
    let path = format!("{ICONS_DIR}/{name}.png");
    let bytes = std::fs::read(&path).ok()?;
    let img = image::load_from_memory(&bytes).ok()?.to_rgba8();
    let size = [img.width() as usize, img.height() as usize];
    let color = egui::ColorImage::from_rgba_unmultiplied(size, &img);
    Some(ctx.load_texture(format!("icon_{name}"), color, egui::TextureOptions::LINEAR))
}

/// Иконка-кнопка (серые 80×80 иконки FL масштабируются до size; on = подсветка).
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
    let btn = egui::ImageButton::new(
        egui::Image::from_texture(tex).fit_to_exact_size(egui::vec2(size, size)),
    );
    ui.add(btn).on_hover_text(tip)
}

/// Просто картинка-иконка (без кнопки).
fn icon_img(
    ui: &mut egui::Ui,
    icons: &HashMap<&'static str, egui::TextureHandle>,
    name: &str,
    size: f32,
) {
    if let Some(tex) = icons.get(name) {
        ui.add(
            egui::Image::from_texture(tex).fit_to_exact_size(egui::vec2(size, size)),
        );
    }
}

fn setup_style(ctx: &egui::Context) {
    ctx.set_theme(egui::Theme::Dark);
    let mut v = egui::Visuals::dark();
    v.panel_fill = C_PANEL;
    v.window_fill = C_WINDOW;
    v.extreme_bg_color = C_PANEL_DARK;
    v.faint_bg_color = C_PANEL_DARK;
    v.selection.stroke = egui::Stroke::new(1.0, C_ACCENT);
    v.widgets.noninteractive.fg_stroke =
        egui::Stroke::new(1.0, Color32::from_rgb(0xC8, 0xC8, 0xCB));
    ctx.set_visuals(v);
    let mut style = (*ctx.style()).clone();
    style.spacing.item_spacing = egui::vec2(6.0, 3.0);
    style.spacing.button_padding = egui::vec2(6.0, 3.0);
    ctx.set_style(style);
}

/// Хелпер: залить прямоугольник (обходит разнобой Rounding/CornerRadius между версиями egui).
fn painter_rect(ui: &egui::Ui, rect: egui::Rect, color: Color32) {
    ui.painter().rect_filled(rect, 1.0, color);
}

/// Позиция ноты → индекс шага 1/16.
fn orig_steps_insert(set: &mut HashSet<(u16, u32)>, iid: u16, position: u32) {
    set.insert((iid, (position as f32 / STEP_TICKS) as u32));
}

/// Круглая ручка в стиле FL: тёмный круг + индикатор; драг по вертикали меняет значение.
fn knob_ui(ui: &mut egui::Ui, value: &mut f32, lo: f32, hi: f32, tip: &'static str) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(20.0, 20.0), Sense::drag());
    let t = ((*value - lo) / (hi - lo)).clamp(0.0, 1.0);
    let p = ui.painter_at(rect);
    p.circle_filled(rect.center(), 9.0, C_PANEL_DARK);
    p.circle(
        rect.center(),
        9.0,
        Color32::TRANSPARENT,
        egui::Stroke::new(1.0, C_BEVEL_LO),
    );
    let ang = (-135.0 + 270.0 * t).to_radians();
    let dir = egui::vec2(ang.cos(), ang.sin());
    p.line_segment(
        [rect.center() - dir * 3.0, rect.center() + dir * 7.0],
        egui::Stroke::new(2.0, C_PLAY),
    );
    if resp.dragged() {
        let dy = ui.input(|i| i.pointer.delta().y);
        *value = (*value - dy * 0.005 * (hi - lo)).clamp(lo, hi);
    }
    resp.on_hover_text(tip)
}

#[derive(Clone, Copy, PartialEq)]
enum Tool {
    Draw,
    Delete,
}

/// Активный жест в Piano Roll (interface-map §5.2: drag — перемещение,
/// правый край — длина).
#[derive(Clone, Copy)]
enum RollDrag {
    Move { idx: usize, dpos: u32, dkey: u16 },
    Resize { idx: usize },
}

#[derive(Clone, Copy)]
struct UiNote {
    iid: u16,
    pos: u32,
    len: u32,
    key: u16,
    vel: u8,
}

struct Strip {
    vol: f32,
    pan: f32,
    mute: bool,
    solo: bool,
    level: f32,
    name: Option<String>,
    slots: Vec<String>,
}

/// Снапшот редактируемого состояния для undo (ноты/шаги/клипы/темп).
#[derive(Clone)]
struct UndoSnap {
    notes: Vec<UiNote>,
    user_steps: HashSet<(u16, u32)>,
    demo_clips: Vec<(usize, usize)>,
    bpm: f64,
}

struct App {
    project: Option<Project>,
    load_error: Option<String>,
    /// Собственные иконки (icons/own/*.png).
    icons: HashMap<&'static str, egui::TextureHandle>,
    show_browser: bool,
    show_playlist: bool,
    show_rack: bool,
    show_roll: bool,
    show_mixer: bool,
    hint: String,
    // transport
    playing: bool,
    recording: bool,
    pattern_mode: bool,
    metronome: bool,
    bpm: f64,
    pos_ticks: f32,
    last_frame: Instant,
    time_as_clock: bool,
    // данные
    current_pattern: usize,
    notes: Vec<UiNote>,
    user_steps: HashSet<(u16, u32)>,
    orig_steps: HashSet<(u16, u32)>,
    selected_note: Option<usize>,
    active_channel: usize,
    last_note_len: u32,
    route: std::collections::HashMap<u16, u16>,
    // playlist
    demo_clips: Vec<(usize, usize)>,
    tool: Tool,
    playlist_zoom: f32,
    // mixer
    strips: Vec<Strip>,
    selected_insert: usize,
    // прочее
    show_picker: bool,
    muted_tracks: HashSet<usize>,
    roll_drag: Option<RollDrag>,
    snap: usize,
    channel_knobs: std::collections::HashMap<u16, (f32, f32)>,
    cpu_fake: f32,
    mem_fake: u32,
    phase: f32,
    // звук
    engine: Engine,
    undo: Vec<UndoSnap>,
    redo: Vec<UndoSnap>,
    tap_times: Vec<Instant>,
    show_piano: bool,
}

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

        let raw = include_bytes!("../../../crates/flp_compat/tests/fixtures/base_one_pattern.flp");
        let (project, load_error) = match flp_compat::parse(raw) {
            Ok(p) => (Some(p), None),
            Err(e) => (None, Some(format!("{e}"))),
        };

        let mut app = Self {
            project: None,
            load_error,
            icons,
            show_browser: true,
            show_playlist: true,
            show_rack: true,
            show_roll: false,
            show_mixer: false,
            hint: "Space — play/stop · шаги/ноты звучат · Ctrl+O — открыть .flp · Ctrl+T — клавиатура-пиано".into(),
            playing: false,
            recording: false,
            pattern_mode: true,
            metronome: false,
            bpm: 120.0,
            pos_ticks: 0.0,
            last_frame: Instant::now(),
            time_as_clock: false,
            current_pattern: 0,
            notes: Vec::new(),
            user_steps: HashSet::new(),
            orig_steps: HashSet::new(),
            selected_note: None,
            active_channel: 0,
            last_note_len: 48,
            route: std::collections::HashMap::new(),
            demo_clips: Vec::new(),
            tool: Tool::Draw,
            playlist_zoom: 1.0,
            strips: Vec::new(),
            selected_insert: 0,
            show_picker: false,
            muted_tracks: HashSet::new(),
            roll_drag: None,
            snap: 1, // Line
            channel_knobs: std::collections::HashMap::new(),
            cpu_fake: 2.0,
            mem_fake: 173,
            phase: 0.0,
            engine: Engine::new(),
            undo: Vec::new(),
            redo: Vec::new(),
            tap_times: Vec::new(),
            show_piano: false,
        };
        if let Some(p) = project {
            app.reload_from_project(p);
        }
        app
    }

    /// Загрузить/перезагрузить проект из модели flp_compat (старт, New, Open).
    fn reload_from_project(&mut self, p: Project) {
        self.engine.stop();
        self.playing = false;
        self.pos_ticks = 0.0;
        self.project = Some(p);
        self.notes.clear();
        self.user_steps.clear();
        self.orig_steps.clear();
        self.route.clear();
        let p = self.project.as_ref().unwrap();
        if let Some(pat) = p.patterns.first() {
            for n in &pat.notes {
                if (n.position as f32) < PATTERN_TICKS {
                    orig_steps_insert(&mut self.orig_steps, n.channel_iid, n.position);
                }
                self.notes.push(UiNote {
                    iid: n.channel_iid,
                    pos: n.position,
                    len: n.length_ticks.max(1),
                    key: n.key,
                    vel: n.velocity,
                });
            }
        }
        for c in &p.channels {
            self.route.insert(c.iid, c.iid + 1); // заглушка: реальный routing (0x16) не декодирован
        }
        self.strips = p
            .inserts
            .iter()
            .map(|ins| Strip {
                vol: 0.78,
                pan: 0.5,
                mute: false,
                solo: false,
                level: 0.0,
                name: ins.name.clone(),
                slots: ins.slots.iter().filter_map(|s| s.name.clone()).collect(),
            })
            .collect();
        if self.strips.is_empty() {
            self.strips.push(Strip {
                vol: 0.78,
                pan: 0.5,
                mute: false,
                solo: false,
                level: 0.0,
                name: Some("Master".into()),
                slots: vec![],
            });
        }
        self.current_pattern = 0;
        self.selected_note = None;
        self.muted_tracks.clear();
        self.roll_drag = None;
        self.demo_clips = vec![(0, 0)];
        self.active_channel = p.channels.first().map(|c| c.iid as usize).unwrap_or(0);
        if let Some(bpm) = p.tempo_bpm {
            self.bpm = bpm;
            self.engine.set_bpm(bpm);
        }
        self.engine.set_master_gain(self.strips[0].vol);
        self.undo.clear();
        self.redo.clear();
    }

    /// Пустой проект «New» (как дефолт FL: один канал Sampler, без паттернов).
    fn default_project() -> Project {
        Project {
            header: flp_compat::Header { format: 0, channels_field: 1, ppq: 96 },
            data_len: 0,
            trailing_bytes: 0,
            tempo_bpm: Some(120.0),
            legacy_tempo: None,
            time_signature: Some((4, 4)),
            version_string: None,
            banner: None,
            artists: None,
            ac_payload: None,
            channels: vec![Channel {
                iid: 0,
                kind: Some(0),
                name: Some("Sampler".into()),
                color: None,
                sample_path: None,
                levels: None,
                enabled: None,
                plugin_internal: None,
                plugin_state_len: None,
                plugin_runtime_len: None,
            }],
            patterns: Vec::new(),
            arrangements: vec![Arrangement {
                id: 0,
                name: Some("Arrangement".into()),
                clips: Vec::new(),
                track_event_count: 0,
            }],
            inserts: vec![Insert {
                index: 0,
                name: None,
                color: None,
                routing: None,
                slots: Vec::new(),
                flags_raw: None,
            }],
            mixer_params: Vec::new(),
            warnings: Vec::new(),
            unknown_opcodes: Default::default(),
        }
    }

    // ---- диалоги FILE ----

    fn open_flp_dialog(&mut self) {
        let file = rfd::FileDialog::new()
            .add_filter("FL Studio project", &["flp"])
            .set_title("Открыть .flp");
        if let Some(path) = file.pick_file() {
            match std::fs::read(&path)
                .map_err(|e| format!("{e}"))
                .and_then(|bytes| flp_compat::parse(&bytes).map_err(|e| format!("{e}")))
            {
                Ok(p) => {
                    let name = path
                        .file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_default();
                    self.reload_from_project(p);
                    self.hint = format!(
                        "Открыт {name}: {} каналов, {} паттернов, {} нот",
                        self.project.as_ref().map(|p| p.channels.len()).unwrap_or(0),
                        self.project.as_ref().map(|p| p.patterns.len()).unwrap_or(0),
                        self.notes.len()
                    );
                }
                Err(e) => {
                    self.load_error = Some(e.clone());
                    self.hint = format!("Не удалось открыть: {e}");
                }
            }
        }
    }

    fn export_wav_dialog(&mut self) {
        let file = rfd::FileDialog::new()
            .add_filter("WAV", &["wav"])
            .set_file_name("vibestudio_export.wav")
            .set_title("Экспорт WAV (offline render)");
        if let Some(path) = file.save_file() {
            let (events, loop_ticks) = self.build_events();
            let loops = if self.pattern_mode { 2 } else { 1 };
            match render_to_wav(&events, loop_ticks, self.bpm, loops, &path) {
                Ok(frames) => {
                    let secs = frames as f64 / 44_100.0;
                    self.hint = format!("Экспорт: {} кадров ({secs:.1} c) → {}", frames, path.display());
                }
                Err(e) => self.hint = format!("Экспорт не удался: {e}"),
            }
        }
    }

    // ---- звук: события движка из текущего состояния ----

    /// Источник голоса канала: sampler (kind 0) → сэмпл-кик, иначе тон.
    fn src_for_channel(&self, iid: u16) -> VoiceSrc {
        let kind = self
            .project
            .as_ref()
            .and_then(|p| p.channels.iter().find(|c| c.iid == iid))
            .and_then(|c| c.kind);
        if kind == Some(0) { VoiceSrc::Kick } else { VoiceSrc::Tone }
    }

    fn gain_for_channel(&self, iid: u16) -> f32 {
        self.channel_knobs.get(&iid).map(|k| k.1).unwrap_or(0.78)
    }

    fn secs_per_tick(&self) -> f32 {
        60.0 / self.bpm as f32 / 96.0
    }

    /// События воспроизведения: PAT — текущий паттерн (ноты + пользовательские
    /// шаги), SONG — клипы плейлиста (клип = экземпляр паттерна со смещением).
    fn build_events(&self) -> (Vec<NoteEvent>, f32) {
        let mut ev = Vec::new();
        let mut push_pattern = |ev: &mut Vec<NoteEvent>, offset: f32| {
            for n in &self.notes {
                ev.push(NoteEvent {
                    tick: offset + n.pos as f32,
                    key: n.key,
                    vel: n.vel,
                    dur_ticks: n.len as f32,
                    gain: self.gain_for_channel(n.iid),
                    src: self.src_for_channel(n.iid),
                });
            }
            for &(iid, step) in &self.user_steps {
                ev.push(NoteEvent {
                    tick: offset + step as f32 * STEP_TICKS,
                    key: 60,
                    vel: 100,
                    dur_ticks: STEP_TICKS,
                    gain: self.gain_for_channel(iid),
                    src: self.src_for_channel(iid),
                });
            }
        };
        if self.pattern_mode {
            push_pattern(&mut ev, 0.0);
            (ev, PATTERN_TICKS)
        } else {
            for &(tr, bar) in &self.demo_clips {
                if !self.muted_tracks.contains(&tr) {
                    push_pattern(&mut ev, bar as f32 * BAR_TICKS);
                }
            }
            (ev, SONG_BARS as f32 * BAR_TICKS)
        }
    }

    // ---- undo/redo (снапшоты редактируемого состояния) ----

    fn snapshot(&self) -> UndoSnap {
        UndoSnap {
            notes: self.notes.clone(),
            user_steps: self.user_steps.clone(),
            demo_clips: self.demo_clips.clone(),
            bpm: self.bpm,
        }
    }

    fn push_undo(&mut self) {
        self.redo.clear();
        self.undo.push(self.snapshot());
        if self.undo.len() > 50 {
            self.undo.remove(0);
        }
    }

    fn apply_snap(&mut self, s: UndoSnap) {
        self.notes = s.notes;
        self.user_steps = s.user_steps;
        self.demo_clips = s.demo_clips;
        if self.bpm != s.bpm {
            self.bpm = s.bpm;
            self.engine.set_bpm(s.bpm);
        }
        self.selected_note = None;
        self.roll_drag = None;
    }

    fn undo(&mut self) {
        if let Some(s) = self.undo.pop() {
            self.redo.push(self.snapshot());
            self.apply_snap(s);
            self.hint = "Undo (Ctrl+Z)".into();
        } else {
            self.hint = "Undo: история пуста".into();
        }
    }

    fn redo(&mut self) {
        if let Some(s) = self.redo.pop() {
            self.undo.push(self.snapshot());
            self.apply_snap(s);
            self.hint = "Redo (Ctrl+Shift+Z)".into();
        }
    }

    /// Tap tempo: среднее последних ≤4 интервалов.
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
            let avg = sum / (t.len() - 1) as f32;
            let bpm = (60.0 / avg).clamp(10.0, 522.0);
            self.bpm = bpm as f64;
            self.engine.set_bpm(bpm as f64);
            self.hint = format!("Tap tempo: {bpm:.1} BPM");
        } else {
            self.hint = "Tap tempo: продолжай нажимать в ритм…".into();
        }
    }

    fn channel_name(&self, iid: u16) -> String {
        self.project
            .as_ref()
            .and_then(|p| p.channels.iter().find(|c| c.iid == iid))
            .map(|c| c.name.clone().unwrap_or_else(|| format!("Channel {}", c.iid)))
            .unwrap_or_else(|| format!("Channel {iid}"))
    }

    fn channel_color(&self, iid: u16) -> Color32 {
        let base = self
            .project
            .as_ref()
            .and_then(|p| p.channels.iter().find(|c| c.iid == iid))
            .and_then(|c| c.color)
            .unwrap_or(0x5C8A50);
        let (r, g, b) = color_to_rgb8(base);
        let lum = 0.2126 * r as f32 + 0.7152 * g as f32 + 0.0722 * b as f32;
        if lum < 60.0 {
            Color32::from_rgb(0x6E, 0x9B, 0x5E)
        } else {
            Color32::from_rgb(r, g, b)
        }
    }

    fn pattern_color_bright(&self) -> Color32 {
        let base = self
            .project
            .as_ref()
            .and_then(|p| p.patterns.get(self.current_pattern))
            .and_then(|pt| pt.color)
            .unwrap_or(0x5C8A50);
        let (r, g, b) = color_to_rgb8(base);
        let f = |x: u8| (x as f32 * 0.55 + 90.0).clamp(40.0, 240.0) as u8;
        Color32::from_rgb(f(r), f(g), f(b))
    }

    fn pattern_name(&self) -> String {
        self.project
            .as_ref()
            .and_then(|p| p.patterns.get(self.current_pattern))
            .map(|pt| pt.name.clone().unwrap_or_else(|| format!("Pattern {}", pt.id)))
            .unwrap_or_else(|| "—".into())
    }

    fn loop_ticks(&self) -> f32 {
        if self.pattern_mode {
            PATTERN_TICKS
        } else {
            SONG_BARS as f32 * BAR_TICKS
        }
    }

    /// Global Snap (behavior-catalog §6): Line/Cell = 1/16 (сетка), Beat = 96,
    /// доли бита — соответствующие доли, Bar = такт, (none) = свободно.
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
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Тема — всегда тёмная, каждый кадр: eframe может переapply системную
        // тему (прототип отрисовывался светлым на macOS в light mode).
        ctx.set_theme(egui::Theme::Dark);
        let now = Instant::now();
        let dt = now.duration_since(self.last_frame).as_secs_f32().min(0.1);
        self.last_frame = now;

        // Транспорт — источник истины теперь движок: позиция читается атомарно.
        let eng_playing = self.engine.playing();
        if eng_playing {
            self.playing = true;
            self.pos_ticks = self.engine.position();
        } else if self.playing {
            // движок сам остановился (сброс после Stop) — синхронизируем UI
            self.playing = false;
            self.pos_ticks = 0.0;
        }
        if self.playing {
            self.phase += dt * 9.0;
            for (i, s) in self.strips.iter_mut().enumerate() {
                let base = if s.solo { 0.0 } else { s.vol };
                s.level = if s.mute || base <= 0.01 {
                    0.0
                } else {
                    (0.35 + 0.3 * (self.phase + i as f32 * 1.7).sin()).clamp(0.0, 1.0) * base
                };
            }
            self.cpu_fake =
                3.0 + 2.5 * (self.phase * 0.4).sin().abs() + 0.7 * (self.phase * 5.0).sin().abs();
        } else {
            for s in self.strips.iter_mut() {
                s.level *= 0.85;
            }
            self.cpu_fake += (2.0 - self.cpu_fake) * 0.05;
        }

        // Горячие клавиши (behavior-catalog §1–2; macOS: Ctrl = Cmd)
        ctx.input(|i| {
            let cmd = i.modifiers.command;
            if i.key_pressed(Key::Space) {
                if self.engine.playing() {
                    self.engine.stop();
                } else {
                    let (events, loop_ticks) = self.build_events();
                    self.engine.play(events, loop_ticks);
                    self.hint = if self.pattern_mode {
                        "PAT: играет текущий паттерн (цикл)".into()
                    } else {
                        "SONG: играет аранжировка клипов".into()
                    };
                }
            }
            if i.key_pressed(Key::L) {
                self.pattern_mode = !self.pattern_mode;
                // перезапуск в новом режиме, если играет
                if self.engine.playing() {
                    let (events, loop_ticks) = self.build_events();
                    self.engine.play(events, loop_ticks);
                }
            }
            if i.key_pressed(Key::R) && !cmd {
                self.recording = !self.recording;
                self.hint = "Record toggle (R) — заглушка: записи нет".into();
            }
            if i.key_pressed(Key::H) && cmd {
                self.engine.stop();
                self.playing = false;
                self.pos_ticks = 0.0;
                self.hint = "Panic (Ctrl+H) — стоп".into();
            }
            if i.key_pressed(Key::M) && cmd {
                self.metronome = !self.metronome;
            }
            if i.key_pressed(Key::Z) && cmd && !i.modifiers.shift {
                self.undo();
            }
            if (i.key_pressed(Key::Z) && cmd && i.modifiers.shift)
                || (i.key_pressed(Key::Y) && cmd)
            {
                self.redo();
            }
            if i.key_pressed(Key::T) && cmd {
                self.show_piano = !self.show_piano;
            }
            if i.key_pressed(Key::Home) {
                self.engine.seek(0.0);
                self.pos_ticks = 0.0;
                self.hint = "Home — playhead в начало".into();
            }
            if i.key_pressed(Key::Backspace) {
                self.snap = if self.snap == 0 { 1 } else { 0 };
                self.hint = format!("Global Snap: {} (Backspace — toggle Line/None)", SNAP_OPTIONS[self.snap]);
            }
            if i.key_pressed(Key::Plus) || i.key_pressed(Key::Equals) {
                if self.current_pattern + 1 < self.project.as_ref().map(|p| p.patterns.len()).unwrap_or(1) {
                    self.current_pattern += 1;
                } else {
                    self.hint = "Следующий паттерн: в файле один паттерн (создание — заглушка)".into();
                }
            }
            if i.key_pressed(Key::Minus) && self.current_pattern > 0 {
                self.current_pattern -= 1;
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
                self.hint = "Plugin Picker (F8) — заглушка: плагины не загружаются".into();
            }
            if i.key_pressed(Key::F9) {
                self.show_mixer = !self.show_mixer;
            }
            if i.key_pressed(Key::F12) {
                self.show_playlist = false;
                self.show_roll = false;
                self.show_mixer = false;
                self.show_picker = false;
                self.hint = "F12 — закрыть все окна".into();
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
            if i.key_pressed(Key::S) && cmd {
                self.hint = "Save (Ctrl+S) — заглушка: собственный формат проекта не реализован".into();
            }
            if i.key_pressed(Key::O) && cmd {
                self.open_flp_dialog();
            }
        });
        ctx.request_repaint_after(std::time::Duration::from_millis(16));

        // Порядок панелей = FL: тулбар сверху, Channel Rack прикован под ним,
        // Browser слева, снизу статусная строка; центр — рабочий стол с плавающими окнами.
        self.top_toolbar(ctx);
        if self.show_rack {
            self.channel_rack_dock(ctx);
        }
        if self.show_browser {
            self.browser(ctx);
        }
        self.status_bar(ctx);
        self.desktop(ctx);
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
    }
}

impl App {
    /// Ряд инструментов FL (P/B/D/T/C/E/Y): Draw, Paint, Delete, Mute, Slice,
    /// Select, Playback. Реально работают Draw/Delete; остальные — заглушки.
    fn tool_buttons(&mut self, ui: &mut egui::Ui) {
        let entries: [(&str, &str, &str, bool); 8] = [
            ("draw", "Draw (P)", "Draw: ЛКМ — поставить (ноту/клип), ПКМ — удалить; drag — перемещение", self.tool == Tool::Draw),
            ("paint", "Paint (B)", "Paint — серия нот при drag (заглушка)", false),
            ("paint_drum", "Paint drum (N)", "Paint drum — для перкуссии (заглушка)", false),
            ("delete", "Delete (D)", "Delete: стирать ноты/клипы", self.tool == Tool::Delete),
            ("mute", "Mute (T)", "Mute — заглушить без удаления (заглушка)", false),
            ("slice", "Slice (C)", "Slice — разрез (заглушка)", false),
            ("select", "Select (E)", "Select — прямоугольное выделение (заглушка)", false),
            ("playback", "Playback (Y)", "Playback — проиграть с позиции (заглушка)", false),
        ];
        for (icon, _label, tip, active) in entries {
            let mut r = icon_button(ui, &self.icons, icon, 17.0, tip);
            if active {
                ui.painter().rect_filled(
                    r.rect.expand(2.0),
                    2.0,
                    C_ACCENT.gamma_multiply(0.35),
                );
                r.mark_changed();
            }
            if r.clicked() {
                match icon {
                    "draw" => self.tool = Tool::Draw,
                    "delete" => self.tool = Tool::Delete,
                    other => {
                        self.hint = format!("Инструмент «{other}» — заглушка (реализованы Draw и Delete)");
                    }
                }
            }
        }
    }

    // ---------- Тулбар: ряд 1 (меню + transport), ряд 2 (паттерн/snap/окна) ----------
    fn top_toolbar(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("toolbar").show(ctx, |ui| {
            ui.vertical(|ui| {
                // ===== РЯД 1 =====
                ui.horizontal(|ui| {
                    // FILE: рабочие пункты + заглушки
                    ui.menu_button("FILE", |ui| {
                        if ui.button("New — пустой проект").clicked() {
                            self.reload_from_project(Self::default_project());
                            self.hint = "Новый проект (Sampler, 120 BPM)".into();
                            ui.close_menu();
                        }
                        if ui.button("Open… (Ctrl+O)").clicked() {
                            self.open_flp_dialog();
                            ui.close_menu();
                        }
                        if ui.button("Export → WAV…").clicked() {
                            self.export_wav_dialog();
                            ui.close_menu();
                        }
                        for it in ["Save (Ctrl+S)", "Save as…", "Import", "Export → MP3/MIDI"] {
                            if ui.button(it).clicked() {
                                self.hint = format!("«{it}» — заглушка");
                            }
                        }
                    });
                    for (name, items) in [
                        ("EDIT", vec!["Undo (Ctrl+Z)", "Redo (Ctrl+Shift+Z)", "Cut", "Copy", "Paste"]),
                        ("ADD", vec!["Channel — заглушка", "Effect — заглушка", "More plugins… (F8)", "Pattern"]),
                        ("PATTERNS", vec!["Rename (F2)", "Clone (Alt+C)", "Delete", "Find next empty (F4)", "Render as audio clip"]),
                        ("VIEW", vec!["Playlist (F5)", "Channel Rack (F6)", "Piano Roll (F7)", "Mixer (F9)", "Browser"]),
                        ("OPTIONS", vec!["MIDI settings (F10)", "Audio settings", "General settings", "Theme settings"]),
                        ("TOOLS", vec!["Macros → Panic (Ctrl+H)", "Riff Machine", "Browser find (Ctrl+F)"]),
                        ("HELP", vec!["Справка (F1)", "About"]),
                    ] {
                        ui.menu_button(name, |ui| {
                            for it in items {
                                if ui.button(it).clicked() {
                                    self.hint =
                                        format!("«{it}» — заглушка (меню не функционально)");
                                }
                            }
                        });
                    }
                    ui.separator();

                    // PAT/SONG — вертикальный стек (как в FL)
                    ui.vertical(|ui| {
                        let pat = egui::Button::new(
                            egui::RichText::new("PAT").size(9.0).color(if self.pattern_mode {
                                Color32::BLACK
                            } else {
                                C_TEXT_DIM
                            }),
                        )
                        .fill(if self.pattern_mode { C_ACCENT } else { C_PANEL_DARK })
                        .min_size(egui::vec2(38.0, 12.0));
                        if ui.add(pat).clicked() {
                            self.pattern_mode = true;
                        }
                        let song = egui::Button::new(
                            egui::RichText::new("SONG").size(9.0).color(if !self.pattern_mode {
                                Color32::BLACK
                            } else {
                                C_TEXT_DIM
                            }),
                        )
                        .fill(if !self.pattern_mode { C_ACCENT } else { C_PANEL_DARK })
                        .min_size(egui::vec2(38.0, 12.0));
                        if ui.add(song).clicked() {
                            self.pattern_mode = false;
                        }
                    })
                    .response
                    .on_hover_text("PAT — играет текущий паттерн, SONG — Playlist (клавиша L)");
                    ui.separator();

                    // transport
                    let play_label = if self.playing { "⏸" } else { "▶" };
                    let play = if self.icons.contains_key("play") {
                        let mut r = icon_button(ui, &self.icons, "play", 16.0, "");
                        if self.playing {
                            // поверх иконки — индикация «играет» (рамка)
                            ui.painter().rect_stroke(
                                r.rect.expand(1.5),
                                2.0,
                                egui::Stroke::new(1.5, C_PLAY),
                                egui::StrokeKind::Inside,
                            );
                            r = r.on_hover_text("");
                        }
                        r
                    } else {
                        ui.add(egui::Button::new(egui::RichText::new(play_label).size(15.0)))
                    };
                    self.set_hint(
                        &play,
                        "Play/Pause — Space (движок cpal: играет паттерн/аранжировку)",
                    );
                    if play.clicked() {
                        if self.engine.playing() {
                            self.engine.stop();
                        } else {
                            let (events, loop_ticks) = self.build_events();
                            self.engine.play(events, loop_ticks);
                        }
                    }
                    let stop = ui.add(egui::Button::new(egui::RichText::new("■").size(13.0)));
                    self.set_hint(&stop, "Stop — возврат в начало");
                    if stop.clicked() {
                        self.engine.stop();
                        self.playing = false;
                        self.pos_ticks = 0.0;
                    }
                    let rec = ui.add(egui::Button::new(
                        egui::RichText::new("●")
                            .color(if self.recording { C_REC } else { C_TEXT_DIM })
                            .size(13.0),
                    ));
                    self.set_hint(&rec, "Record (R) — заглушка: записи нет");
                    if rec.clicked() {
                        self.recording = !self.recording;
                    }

                    // Tempo LCD — 3 знака после запятой, как в FL (130.000)
                    let bpm_r = ui.add(
                        egui::DragValue::new(&mut self.bpm)
                            .range(10.0..=522.0)
                            .speed(0.01)
                            .custom_formatter(|v, _| format!("{v:.3}")),
                    )
                    .on_hover_text("Темп 10–522 BPM; в FL точность 1/1000 BPM (правый клик — Tap/Half speed)");
                    if bpm_r.drag_started() {
                        self.push_undo();
                    }
                    if bpm_r.changed() {
                        self.engine.set_bpm(self.bpm);
                    }
                    let tap = ui.add(egui::Button::new(egui::RichText::new("TAP").size(10.0)));
                    self.set_hint(&tap, "Tap tempo: нажимай в ритм (пауза > 2 c сбрасывает)");
                    if tap.clicked() {
                        self.tap();
                    }
                    ui.separator();

                    // метроном: индикатор мигает на сильную долю при игре
                    let beat_flash = self.playing && self.metronome && (self.pos_ticks % PPQ) < 8.0;
                    let met = ui.selectable_label(
                        self.metronome,
                        egui::RichText::new("⏱ Метроном").color(if beat_flash { C_ACCENT } else { Color32::PLACEHOLDER }),
                    );
                    self.set_hint(&met, "Метроном (Ctrl+M) — заглушка: щелчков нет, только индикатор");
                    if met.clicked() {
                        self.metronome = !self.metronome;
                    }

                    // Song position LCD
                    ui.separator();
                    let bar = (self.pos_ticks / BAR_TICKS).floor() as usize + 1;
                    let beat = ((self.pos_ticks % BAR_TICKS) / PPQ).floor() as usize + 1;
                    let step = ((self.pos_ticks % PPQ) / STEP_TICKS).floor() as usize + 1;
                    let secs = self.pos_ticks / PPQ * (60.0 / self.bpm as f32);
                    let (lcd, mode) = if self.time_as_clock {
                        (
                            format!(
                                "{:02}:{:02}.{:02}",
                                (secs / 60.0) as usize,
                                (secs % 60.0) as usize,
                                ((secs * 100.0) % 100.0) as usize
                            ),
                            "M:S.C",
                        )
                    } else {
                        (format!("{bar}:{beat}:{step}"), "B:B.S")
                    };
                    // Song position LCD: тёмный бокс, белые цифры (как в FL)
                    let (lcd_rect, _) = ui.allocate_exact_size(egui::vec2(108.0, 22.0), Sense::click());
                    ui.painter().rect_filled(lcd_rect, 2.0, C_LCD_BG);
                    ui.painter().text(
                        lcd_rect.center(),
                        Align2::CENTER_CENTER,
                        format!("{lcd}  {mode}"),
                        egui::FontId::monospace(14.0),
                        C_LCD_TEXT,
                    );
                    let lcd_resp = ui.interact(lcd_rect, egui::Id::new("pos_lcd"), Sense::click());
                    self.set_hint(&lcd_resp, "Song Position LCD (клик — сменить формат бары/время)");
                    if lcd_resp.clicked() {
                        self.time_as_clock = !self.time_as_clock;
                    }
                    ui.separator();

                    // Output + CPU + память
                    let (scope_rect, scope_resp) =
                        ui.allocate_exact_size(egui::vec2(100.0, 24.0), Sense::hover());
                    self.set_hint(
                        &scope_resp,
                        "Output Panel — заглушка: осциллоскоп и пик-метр мастер-выхода",
                    );
                    let pt = ui.painter_at(scope_rect);
                    pt.rect_filled(scope_rect, 1.0, C_PANEL_DARK);
                    let mid = scope_rect.center().y;
                    let mut prev = scope_rect.left_center();
                    for x in 0..=100 {
                        let t = x as f32 / 100.0;
                        let amp = if self.playing { 8.0 } else { 0.6 };
                        let y = mid + amp * (t * 12.0 + self.phase * 2.0).sin();
                        let next = egui::pos2(scope_rect.left() + t * scope_rect.width(), y);
                        pt.line_segment([prev, next], egui::Stroke::new(1.2, C_PLAY.linear_multiply(0.9)));
                        prev = next;
                    }
                    let lvl = self.strips.first().map(|s| s.level).unwrap_or(0.0);
                    let mh = lvl * scope_rect.height();
                    pt.rect_filled(
                        egui::Rect::from_min_size(
                            egui::pos2(scope_rect.right() - 6.0, scope_rect.bottom() - mh),
                            egui::vec2(4.0, mh),
                        ),
                        1.0,
                        if lvl > 0.9 { C_REC } else { C_PLAY },
                    );

                    let mem = ui
                        .label(egui::RichText::new(format!("{} MB", self.mem_fake)).monospace())
                        .on_hover_text("Память — заглушка (в FL: голоса/память; двойной клик — Performance Monitor)");
                    self.set_hint(&mem, "Память — заглушка");
                    ui.label(egui::RichText::new(format!("CPU {:>4.1}%", self.cpu_fake)).monospace())
                        .on_hover_text("CPU — заглушка (в FL: % времени на заполнение аудиобуфера)");
                });

                ui.separator();

                // ===== РЯД 2: имя паттерна, snap, селектор паттерна, инструменты, окна =====
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.label(egui::RichText::new("VibeStudio").size(11.0));
                        ui.label(egui::RichText::new(self.pattern_name()).strong().size(13.0));
                    })
                    .response
                    .on_hover_text("Имя проекта/паттерна (F2 rename — заглушка)");
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

                    let minus = ui.add(egui::Button::new("−").small());
                    self.set_hint(&minus, "Предыдущий паттерн (−)");
                    if minus.clicked() && self.current_pattern > 0 {
                        self.current_pattern -= 1;
                    }
                    // Светлый бокс селектора паттерна — фирменный вид FL
                    ui.visuals_mut().widgets.inactive.bg_fill = C_COMBO_LIGHT;
                    ui.visuals_mut().widgets.inactive.fg_stroke =
                        egui::Stroke::new(1.0, Color32::BLACK);
                    ui.visuals_mut().widgets.hovered.bg_fill = C_COMBO_LIGHT;
                    egui::ComboBox::from_id_salt("pat_sel")
                        .selected_text(egui::RichText::new(self.pattern_name()).color(Color32::BLACK))
                        .width(96.0)
                        .show_ui(ui, |ui| {
                            let pats: Vec<(usize, String)> = self
                                .project
                                .as_ref()
                                .map(|p| {
                                    p.patterns
                                        .iter()
                                        .enumerate()
                                        .map(|(i, pt)| {
                                            (i, pt.name.clone().unwrap_or_else(|| format!("Pattern {}", pt.id)))
                                        })
                                        .collect()
                                })
                                .unwrap_or_default();
                            for (i, nm) in pats {
                                ui.selectable_value(&mut self.current_pattern, i, nm);
                            }
                        });
                    let plus = ui.add(egui::Button::new("+").small());
                    self.set_hint(&plus, "Следующий паттерн (+); создание нового — заглушка");
                    if plus.clicked() {
                        let n = self.project.as_ref().map(|p| p.patterns.len()).unwrap_or(0);
                        if self.current_pattern + 1 < n {
                            self.current_pattern += 1;
                        } else {
                            self.hint = "Заглушка: создание паттернов не реализовано".into();
                        }
                    }
                    ui.separator();

                    // Ряд инструментов с иконками FL (interface-map §4.3/§5.2):
                    // Draw/Draw(B)/Delete/Mute/Slice/Select/Zoom/Playback
                    self.tool_buttons(ui);
                    ui.separator();

                    // Undo/Redo (снапшоты нот/шагов/клипов/темпа)
                    let u = ui.add(egui::Button::new(egui::RichText::new("↶").size(14.0)));
                    self.set_hint(&u, "Undo (Ctrl+Z)");
                    if u.clicked() {
                        self.undo();
                    }
                    let r = ui.add(egui::Button::new(egui::RichText::new("↷").size(14.0)));
                    self.set_hint(&r, "Redo (Ctrl+Shift+Z)");
                    if r.clicked() {
                        self.redo();
                    }
                    ui.separator();

                    // окна справа (+ мастер-громкость — большая ручка FL справа ряда 2)
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let master_vol = ui
                            .add_sized(
                                [64.0, 18.0],
                                egui::DragValue::new(&mut self.strips[0].vol)
                                    .range(0.0..=1.0)
                                    .speed(0.01)
                                    .prefix("Master "),
                            )
                            .on_hover_text(
                                "Мастер-громкость (в FL — большая ручка справа); связана с фейдером Master в Mixer (F9)",
                            );
                        self.set_hint(&master_vol, "Мастер-громкость — заглушка (связана с Master-фейдером)");
                        ui.separator();
                        let b = ui.selectable_label(self.show_browser, "Browser");
                        self.set_hint(&b, "Browser (док слева)");
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
                            self.set_hint(&btn, "Показать/скрыть окно (плавающий стол, как в FL)");
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

    // ---------- Channel Rack: прикованная панель под тулбаром (UX скриншота) ----------
    fn channel_rack_dock(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("rack_dock")
            .resizable(false)
            .show(ctx, |ui| {
                let mut head_hovered = false;
                ui.horizontal(|ui| {
                    // Channel Options Menu (мануал, channelrack.htm п.1) — ▾-якорь в шапке
                    ui.menu_button("▾", |ui| {
                        for item in [
                            "Add one — инструменты из favorites (заглушка)",
                            "Piano roll — открыть ролл канала",
                            "Rename / color / icon (F2) — заглушка",
                            "Load sample — заглушка",
                            "Fill each 2 / 4 / 8 steps — заглушка",
                            "Clone selected (Alt+C) — заглушка",
                            "Delete selected (Alt+Del) — заглушка",
                            "Zip selected (Alt+Z) / Unzip all (Alt+U) — заглушка",
                            "Sort by Name/Type/Label/Mixer — заглушка",
                        ] {
                            if ui.button(item).clicked() {
                                self.hint = format!("«{item}» — пункт меню канала, заглушка");
                            }
                        }
                    })
                    .response
                    .on_hover_text("Channel Options Menu (якорь-стрелка в заголовке, как в FL)");
                    egui::ComboBox::from_id_salt("rack_filter")
                        .selected_text("All")
                        .width(90.0)
                        .show_ui(ui, |ui| {
                            ui.selectable_label(true, "All");
                        })
                        .response
                        .on_hover_text("Display Filter — фильтр-группы каналов (заглушка)");
                    let head = ui.label(egui::RichText::new("Channel rack").strong());
                    self.set_hint(&head, "Shift+колесо здесь — листать паттерны (TIP из мануала)");
                    head_hovered = head.hovered();
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let close = ui.add(egui::Button::new(egui::RichText::new("×").size(13.0)));
                        self.set_hint(&close, "Скрыть Channel Rack (F6 — вернуть)");
                        if close.clicked() {
                            self.show_rack = false;
                        }
                    });
                });
                // Shift+колесо на шапке река — листать паттерны (мануал, TIP)
                if head_hovered {
                    let scroll = ui.input(|i| {
                        if i.modifiers.shift {
                            i.raw_scroll_delta.y
                        } else {
                            0.0
                        }
                    });
                    let n_pats = self.project.as_ref().map(|p| p.patterns.len()).unwrap_or(1);
                    if scroll < -1.0 && self.current_pattern > 0 {
                        self.current_pattern -= 1;
                    }
                    if scroll > 1.0 && self.current_pattern + 1 < n_pats {
                        self.current_pattern += 1;
                    }
                }
                ui.separator();

                egui::ScrollArea::vertical().max_height(150.0).show(ui, |ui| {
                    let iids: Vec<u16> = self
                        .project
                        .as_ref()
                        .map(|p| p.channels.iter().map(|c| c.iid).collect())
                        .unwrap_or_default();
                    for iid in iids {
                        self.rack_row(ui, iid);
                        ui.separator();
                    }
                    if ui.button("+ Add channel — заглушка").clicked() {
                        self.hint = "Заглушка: добавление каналов не реализовано (нужны Channel Settings/плагины)".into();
                    }
                });
            });
    }

    fn rack_row(&mut self, ui: &mut egui::Ui, iid: u16) {
        ui.horizontal(|ui| {
            // Mute LED
            let led = ui.add(egui::Button::new(egui::RichText::new("●").color(C_PLAY).size(11.0)));
            self.set_hint(&led, "Mute LED канала (заглушка: звук не реализован)");
            if led.clicked() {
                self.toggle_channel_mute(iid);
            }
            // Pan / Volume — круглые ручки (драг по вертикали), как в FL
            let (pan_r, vol_r) = {
                let k = self.channel_knobs.entry(iid).or_insert((0.5, 0.78));
                let p = knob_ui(ui, &mut k.0, -1.0, 1.0, "Pan канала — драг по вертикали (заглушка: в памяти)");
                let v = knob_ui(ui, &mut k.1, 0.0, 1.0, "Volume канала — драг по вертикали (заглушка)");
                (p, v)
            };
            self.set_hint(&pan_r, "Pan канала — заглушка");
            self.set_hint(&vol_r, "Volume канала — заглушка");
            // Mixer track routing (тёмный бокс с номером, как на скриншоте)
            let mut rt = *self.route.get(&iid).unwrap_or(&(iid + 1));
            let rt_r = {
                ui.visuals_mut().widgets.inactive.bg_fill = C_PANEL_DARK;
                ui.visuals_mut().widgets.inactive.fg_stroke = egui::Stroke::new(1.0, C_LCD_TEXT);
                ui.add_sized([34.0, 17.0], egui::DragValue::new(&mut rt).range(1..=18))
            }
            .on_hover_text("Mixer track routing — заглушка: реальный routing в 0x16 (не в спеке); значение в памяти");
            self.set_hint(&rt_r, "Mixer track routing — заглушка");
            self.route.insert(iid, rt);
            // Channel button: серая 3D-кнопка с белым текстом + цветной стрип канала
            // Перед кнопкой — типовая иконка FL (audio = sampler, instrument = generator)
            let kind = self
                .project
                .as_ref()
                .and_then(|p| p.channels.iter().find(|c| c.iid == iid))
                .and_then(|c| c.kind);
            match kind {
                Some(0) => icon_img(ui, &self.icons, "audio", 14.0),
                Some(2) | Some(4) => icon_img(ui, &self.icons, "instrument", 14.0),
                _ => {}
            }
            let name = self.channel_name(iid);
            let col = self.channel_color(iid);
            let btn = ui.add(
                egui::Button::new(egui::RichText::new(format!(" {name} ")).color(Color32::WHITE)).fill(C_BTN),
            );
            let brect = btn.rect;
            ui.painter().rect_filled(
                egui::Rect::from_min_size(
                    brect.left_top() + egui::vec2(2.0, 2.0),
                    egui::vec2(4.0, brect.height() - 4.0),
                ),
                1.0,
                col,
            );
            ui.painter().line_segment(
                [brect.left_top() + egui::vec2(1.0, 1.0), brect.right_top() + egui::vec2(-1.0, 1.0)],
                egui::Stroke::new(1.0, C_BTN_HI),
            );
            self.set_hint(
                &btn,
                "Кнопка канала: ЛКМ — выбрать (в FL открывает Channel Settings/плагин — заглушка); ПКМ — меню канала (заглушка)",
            );
            if btn.clicked() {
                self.active_channel = iid as usize;
            }
            if btn.double_clicked() {
                self.hint = "Channel Settings (вкладки Main/SMP/INS/MISC) — заглушка".into();
            }
            if btn.secondary_clicked() {
                self.hint = "Меню канала: Piano roll, Rename, Load sample, Clone, Delete, Fill each 2/4/8 — заглушка".into();
            }
            // Channel selector — вертикальный зелёный LED (как в FL)
            let sel_on = self.active_channel == iid as usize;
            let (srect, sresp) = ui.allocate_exact_size(egui::vec2(7.0, 22.0), Sense::click());
            ui.painter().rect_filled(srect, 2.0, if sel_on { C_PLAY } else { C_PANEL_DARK });
            ui.painter().rect_stroke(
                srect,
                2.0,
                egui::Stroke::new(1.0, if sel_on { C_PLAY } else { C_BEVEL_LO }),
                egui::StrokeKind::Inside,
            );
            self.set_hint(&sresp, "Channel Selector — цель Piano Roll");
            if sresp.clicked() {
                self.active_channel = iid as usize;
            }
            ui.separator();

            // 16 шагов: рельефные клетки 22×20 (мармелад с фаской, как в FL 21)
            for step in 0..16u32 {
                let orig = self.orig_steps.contains(&(iid, step));
                let user = self.user_steps.contains(&(iid, step));
                let lit = orig || user;
                let is_beat = step % 4 == 0;
                let fill = if lit {
                    if is_beat { C_STEP_ON_BEAT } else { C_STEP_ON }
                } else if is_beat {
                    C_STEP_BEAT
                } else {
                    C_STEP_OFF
                };
                let (rect, resp) = ui.allocate_exact_size(egui::vec2(22.0, 20.0), Sense::click());
                ui.painter().rect_filled(rect, 2.0, fill);
                // фаска: светлая верхняя грань, тёмная нижняя
                ui.painter().line_segment(
                    [rect.left_top() + egui::vec2(1.0, 0.5), rect.right_top() + egui::vec2(-1.0, 0.5)],
                    egui::Stroke::new(1.0, C_BEVEL_HI),
                );
                ui.painter().line_segment(
                    [rect.left_bottom() + egui::vec2(1.0, -0.5), rect.right_bottom() + egui::vec2(-1.0, -0.5)],
                    egui::Stroke::new(1.0, C_BEVEL_LO),
                );
                self.set_hint(
                    &resp,
                    "Step Sequencer: ЛКМ — включить шаг (нота 1/16 + превью звука), ПКМ — выключить",
                );
                if resp.clicked() && !orig {
                    self.push_undo();
                    self.user_steps.insert((iid, step));
                    // превью: sampler-каналы — кик, прочие — тон C4
                    let (key, src) = if self.src_for_channel(iid) == VoiceSrc::Kick {
                        (60, VoiceSrc::Kick)
                    } else {
                        (60, VoiceSrc::Tone)
                    };
                    self.engine
                        .preview(key, 100, 0.3, self.gain_for_channel(iid), src);
                }
                if resp.secondary_clicked() && user {
                    self.push_undo();
                    self.user_steps.remove(&(iid, step));
                }
            }
            // Piano roll preview (interface-map §6.1: если у канала ролл — клетки
            // заменяются миниатюрой; клик открывает Piano Roll)
            let has_roll = self
                .notes
                .iter()
                .any(|n| n.iid == iid && (n.len as f32 % STEP_TICKS != 0.0 || n.len > 24));
            if has_roll {
                let (prev_rect, prev_resp) =
                    ui.allocate_exact_size(egui::vec2(22.0 * 16.0, 20.0), Sense::click());
                ui.painter().rect_filled(prev_rect, 2.0, C_PANEL_DARK);
                ui.painter()
                    .rect_stroke(prev_rect, 2.0, egui::Stroke::new(0.5, C_GRID), egui::StrokeKind::Inside);
                let col = self.channel_color(iid);
                for n in self.notes.iter().filter(|n| n.iid == iid) {
                    let x0 = prev_rect.left() + n.pos as f32 / PATTERN_TICKS * prev_rect.width();
                    let w = (n.len as f32 / PATTERN_TICKS * prev_rect.width()).max(2.0);
                    // высота строки: key 36..84 → верх/низ миниатюры
                    let frac = 1.0 - (n.key as f32 - 36.0) / 48.0;
                    let yc = prev_rect.top() + frac.clamp(0.05, 0.95) * prev_rect.height();
                    painter_rect(
                        ui,
                        egui::Rect::from_center_size(egui::pos2(x0 + w / 2.0, yc), egui::vec2(w, 2.5)),
                        col,
                    );
                }
                self.set_hint(
                    &prev_resp,
                    "Piano roll preview (миниатюра ролла вместо шагов): клик — открыть Piano Roll (F7)",
                );
                if prev_resp.clicked() {
                    self.show_roll = true;
                    self.active_channel = iid as usize;
                }
            }
        });
    }

    fn toggle_channel_mute(&mut self, _iid: u16) {
        self.hint = "Заглушка: mute канала не реализован (нет аудиодвижка)".into();
    }

    // ---------- Browser: прикованная панель слева, FL-дерево (interface-map §8) ----------
    fn browser(&mut self, ctx: &egui::Context) {
        egui::SidePanel::left("browser")
            .resizable(true)
            .default_width(215.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Browser").strong());
                    ui.label(
                        egui::RichText::new(format!("- Snap {}", SNAP_OPTIONS[self.snap]))
                            .weak()
                            .small(),
                    );
                });
                ui.separator();
                egui::ScrollArea::vertical().show(ui, |ui| {
                    let section = |ui: &mut egui::Ui, name: &str| {
                        ui.button(egui::RichText::new(format!("▾ {name}")).weak());
                    };
                    section(ui, "Generators");
                    section(ui, "Remote control");
                    section(ui, "Recent files");
                    section(ui, "Plugin database");
                    ui.indent("pdb", |ui| {
                        for nm in ["Fruity Parametric EQ 2", "Fruity Reeverb 2", "3x Osc"] {
                            if ui.button(egui::RichText::new(nm).weak().small()).clicked() {
                                self.hint = format!("«{nm}» — заглушка: Plugin Picker (F8) и загрузка плагинов не реализованы");
                            }
                        }
                    });
                    for s in [
                        "Plugin presets",
                        "Channel presets",
                        "Mixer presets",
                        "Scores",
                        "Backup",
                        "Clipboard files",
                        "Demo projects",
                        "Envelopes",
                        "IL shared data",
                        "Impulses",
                        "Misc",
                        "My projects",
                    ] {
                        section(ui, s);
                    }
                    section(ui, "Packs");
                    ui.indent("packs", |ui| {
                        for pack in ["CARNIVAL", "Fingertips Drums", "Best Free Drum Loops", "Bleep", "Analog RadioKot"] {
                            if ui.button(egui::RichText::new(pack).weak().small()).clicked() {
                                self.hint = "Заглушка: содержимое пака не реализовано (нужен файловый браузер)".into();
                            }
                        }
                        ui.indent("drums", |ui| {
                            ui.label(egui::RichText::new("Drums").weak().small());
                            ui.indent("kicks", |ui| {
                                if ui.button(egui::RichText::new("Kicks / 909 Kick.wav").weak().small()).clicked() {
                                    self.hint = "Заглушка: preview сэмпла (ЛКМ до 5 сек) и drag в Rack не реализованы".into();
                                }
                            });
                        });
                    });
                });
            });
    }

    fn desktop(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE.fill(C_BG))
            .show(ctx, |ui| {
                if let Some(err) = &self.load_error {
                    ui.label(egui::RichText::new(format!("Ошибка разбора FLP: {err}")).color(C_REC));
                }
                ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
                    ui.add_space(ui.available_height() / 2.0 - 40.0);
                    ui.label(egui::RichText::new("VibeStudio").size(38.0).weak().strong());
                    ui.label(
                        egui::RichText::new(
                            "Плавающие окна перетаскиваются за заголовки · F5/F6/F7/F9 — панели · Space — play (звук — заглушка)\nДанные загружены реальным парсером flp_compat из base_one_pattern.flp",
                        )
                        .weak()
                        .size(12.0),
                    );
                });
            });
    }

    // ---------- Статусная строка снизу (hint + версия, как строка Edition в FL) ----------
    fn status_bar(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::bottom("statusbar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("💡").size(11.0));
                ui.label(egui::RichText::new(&self.hint).weak().size(11.0));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(egui::RichText::new(VERSION_LINE).weak().size(11.0));
                });
            });
        });
    }

    // ---------- Playlist: плавающее окно (interface-map §4; UX скриншота) ----------
    fn playlist_window(&mut self, ctx: &egui::Context) {
        let mut open = self.show_playlist;
        let bar_w = 80.0 * self.playlist_zoom;
        const ROW_H: f32 = 30.0;
        const VISIBLE_TRACKS: usize = 12;
        let grid_w = SONG_BARS as f32 * bar_w;
        let grid_h = VISIBLE_TRACKS as f32 * ROW_H;

                let title = format!("🔊 Playlist - Arrangement ▸ {}", self.pattern_name());
                egui::Window::new(title)
                    .open(&mut open)
                    .default_pos([620.0, 240.0])
                    .default_size([820.0, 420.0])
                    .show(ctx, |ui| {
                        // меню-якорь окна (interface-map §8: Edit/Tools/View/Snap/Zoom/Detached)
                        ui.menu_button("▾", |ui| {
                            for section in ["Edit: Cut/Copy/Paste/Duplicate, Delete, Shift left/right…", "Tools: Quick quantize, Consolidate, Clone track(s)", "View: Grid color, Time segments, Mini preview", "Snap / Select / Group / Zoom / Time markers", "Picker panel (Alt+P)", "Detached — заглушка"] {
                                if ui.button(section).clicked() {
                                    self.hint = format!("Меню Playlist → {section} — заглушка");
                                }
                            }
                        })
                        .response
                        .on_hover_text("Меню окна Playlist (якорь-стрелка, как в FL)");
                ui.horizontal(|ui| {
                    // тул-иконки окна (как в FL-заголовке Playlist)
                    self.tool_buttons(ui);
                    ui.separator();
                    ui.label(egui::RichText::new("клипы-заглушки в памяти; сохранение не реализовано").weak().small());
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.add(
                            egui::Slider::new(&mut self.playlist_zoom, 0.5..=3.0)
                                .show_value(false)
                                .text("🔍"),
                        )
                        .on_hover_text("Zoom (Ctrl+колесо в FL)");
                    });
                });
                ui.separator();

                // пикер-панель слева + сетка справа (как на скриншоте)
                ui.horizontal(|ui| {
                    // Picker Panel: источник клипов
                    ui.vertical(|ui| {
                        ui.set_min_width(110.0);
                        ui.label(egui::RichText::new("PICKER").weak().size(9.0));
                        let chip = egui::Button::new(
                            egui::RichText::new(format!("▶ {}", self.pattern_name())).color(Color32::WHITE),
                        )
                        .fill(self.pattern_color_bright())
                        .min_size(egui::vec2(100.0, 22.0));
                        if ui.add(chip).clicked() {
                            self.hint = "Источник клипов: текущий паттерн (клик по сетке — поставить)".into();
                        }
                        ui.with_layout(egui::Layout::bottom_up(egui::Align::Min), |ui| {
                            if ui.button("+").clicked() {
                                self.hint = "Заглушка: создание паттернов не реализовано".into();
                            }
                        });
                    });
                    ui.separator();

                    // сетка
                    let ruler_h = 18.0;
                    let track_col_w = 66.0;
                    let (resp, painter) =
                        ui.allocate_painter(egui::vec2(track_col_w + grid_w, ruler_h + grid_h), Sense::click());
                    let origin = resp.rect.min + egui::vec2(track_col_w, ruler_h);

                    // линейка тактов
                    painter.rect_filled(
                        egui::Rect::from_min_size(resp.rect.left_top(), egui::vec2(resp.rect.width(), ruler_h)),
                        0.0,
                        C_PANEL,
                    );
                    for bar in 0..SONG_BARS {
                        let x = origin.x + bar as f32 * bar_w;
                        painter.line_segment(
                            [egui::pos2(x, resp.rect.top()), egui::pos2(x, origin.y + grid_h)],
                            egui::Stroke::new(1.0, C_GRID_STRONG),
                        );
                        painter.text(
                            egui::pos2(x + 4.0, resp.rect.top() + 2.0),
                            Align2::LEFT_TOP,
                            format!("{}", bar + 1),
                            egui::FontId::monospace(10.0),
                            C_LCD_TEXT,
                        );
                    }

                    // треки: цветные заголовки-группы по 4 (как в FL) + LED
                    for tr in 0..VISIBLE_TRACKS {
                        let y = origin.y + tr as f32 * ROW_H;
                        let group_col = TRACK_GROUP_COLORS[(tr / 4) % 4];
                        let head = egui::Rect::from_min_size(
                            egui::pos2(resp.rect.left(), y),
                            egui::vec2(track_col_w - 8.0, ROW_H - 2.0),
                        );
                        painter.rect_filled(head, 2.0, group_col);
                        painter.text(
                            egui::pos2(resp.rect.left() + 6.0, y + ROW_H / 2.0),
                            Align2::LEFT_CENTER,
                            format!("Track {}", tr + 1),
                            egui::FontId::proportional(10.0),
                            Color32::WHITE,
                        );
                        let led_c = egui::pos2(resp.rect.left() + track_col_w - 4.0, y + ROW_H / 2.0);
                        let muted = self.muted_tracks.contains(&tr);
                        painter.circle_filled(led_c, 3.0, if muted { C_LED_OFF } else { C_PLAY });
                        let led_rect =
                            egui::Rect::from_center_size(led_c, egui::vec2(12.0, ROW_H));
                        let led_resp = ui.interact(led_rect, egui::Id::new(("trled", tr)), Sense::click());
                        self.set_hint(&led_resp, "Mute LED трека: клик — приглушить (клипы трека тускнеют) — заглушка");
                        if led_resp.clicked() {
                            if muted {
                                self.muted_tracks.remove(&tr);
                            } else {
                                self.muted_tracks.insert(tr);
                            }
                        }
                        painter.rect_filled(
                            egui::Rect::from_min_size(egui::pos2(origin.x, y), egui::vec2(grid_w, ROW_H - 2.0)),
                            0.0,
                            Color32::from_rgb(0x26, 0x28, 0x2B),
                        );
                    }
                    for b in 0..=(SONG_BARS * 4) {
                        let x = origin.x + b as f32 * (bar_w / 4.0);
                        let strong = b % 4 == 0;
                        painter.line_segment(
                            [egui::pos2(x, origin.y), egui::pos2(x, origin.y + grid_h)],
                            egui::Stroke::new(
                                if strong { 1.0 } else { 0.4 },
                                if strong { C_GRID_STRONG } else { C_GRID },
                            ),
                        );
                    }

                    // клипы
                    let pat_col = self.pattern_color_bright();
                    let mut to_remove: Option<(usize, usize)> = None;
                    for (i, (tr, bar)) in self.demo_clips.clone().iter().enumerate() {
                        let dim = self.muted_tracks.contains(tr);
                        let r = egui::Rect::from_min_size(
                            egui::pos2(origin.x + *bar as f32 * bar_w + 1.0, origin.y + *tr as f32 * ROW_H + 1.0),
                            egui::vec2(bar_w - 2.0, ROW_H - 4.0),
                        );
                        painter.rect_filled(r, 3.0, if dim { pat_col.gamma_multiply(0.4) } else { pat_col });
                        painter.text(
                            egui::pos2(r.left() + 4.0, r.center().y),
                            Align2::LEFT_CENTER,
                            format!("{} (демо)", self.pattern_name()),
                            egui::FontId::proportional(10.0),
                            Color32::WHITE,
                        );
                        let clip_resp = ui.interact(r, egui::Id::new(("clip", i)), Sense::click());
                        self.set_hint(&clip_resp, "Pattern Clip (демо-заглушка): ПКМ — удалить; сохранение не реализовано");
                        if clip_resp.secondary_clicked() {
                            to_remove = Some((*tr, *bar));
                        }
                    }
                    if let Some(rc) = to_remove {
                        self.demo_clips.retain(|c| c != &rc);
                    }

                    // playhead
                    if self.playing || self.pos_ticks > 0.0 {
                        let x = origin.x + self.pos_ticks / (SONG_BARS as f32 * BAR_TICKS) * grid_w;
                        painter.line_segment(
                            [egui::pos2(x, origin.y), egui::pos2(x, origin.y + grid_h)],
                            egui::Stroke::new(1.5, C_PLAY),
                        );
                    }

                    // клики по сетке
                    if let Some(pos) = resp.interact_pointer_pos() {
                        if pos.x >= origin.x && pos.y >= origin.y {
                            let bar = (((pos.x - origin.x) / bar_w) as usize).min(SONG_BARS - 1);
                            let tr = (((pos.y - origin.y) / ROW_H) as usize).min(VISIBLE_TRACKS - 1);
                            if self.tool == Tool::Draw && resp.clicked() && !self.demo_clips.contains(&(tr, bar)) {
                                self.push_undo();
                                self.demo_clips.push((tr, bar));
                                self.hint = "Клип добавлен (участвует в SONG-воспроизведении и экспорте)".into();
                            } else if self.tool == Tool::Delete && resp.clicked() {
                                self.push_undo();
                                self.demo_clips.retain(|c| c != &(tr, bar));
                            }
                            if pos.y < origin.y && resp.clicked() {
                                self.pos_ticks = bar as f32 * BAR_TICKS;
                            }
                        }
                    }
                });
            });
        self.show_playlist = open;
    }

    // ---------- Piano Roll (F7, interface-map §5) ----------
    fn piano_roll_window(&mut self, ctx: &egui::Context) {
        let mut open = self.show_roll;
        const KEY_TOP: u16 = 84;
        const KEY_BOTTOM: u16 = 36;
        const ROW_H: f32 = 11.0;
        const PX_PER_TICK: f32 = 1.4;
        let width = (PATTERN_TICKS * PX_PER_TICK) as usize;

        let title = format!(
            "🎹 Piano Roll - {} ▸ {}",
            self.pattern_name(),
            self.channel_name(self.active_channel as u16)
        );
        egui::Window::new(title)
            .open(&mut open)
            .default_pos([420.0, 320.0])
            .default_size([900.0, 480.0])
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Target Channel:");
                    let targets: Vec<(u16, String)> = self
                        .project
                        .as_ref()
                        .map(|p| {
                            p.channels
                                .iter()
                                .map(|c| (c.iid, c.name.clone().unwrap_or_else(|| format!("Channel {}", c.iid))))
                                .collect()
                        })
                        .unwrap_or_default();
                    let cur = self.channel_name(self.active_channel as u16);
                    egui::ComboBox::from_id_salt("roll_target")
                        .selected_text(cur)
                        .show_ui(ui, |ui| {
                            for (iid, nm) in targets {
                                ui.selectable_value(&mut self.active_channel, iid as usize, nm);
                            }
                        });
                    ui.separator();
                    self.tool_buttons(ui);
                    ui.separator();
                    let snap_label = format!("snap {}", SNAP_OPTIONS[self.snap]);
                    ui.label(egui::RichText::new(snap_label).weak().small());
                });
                ui.separator();

                let ruler_h = 16.0;
                let keys_w = 52.0;
                let grid_h = (KEY_TOP - KEY_BOTTOM + 1) as f32 * ROW_H;
                let grid_size = egui::vec2(width as f32, grid_h);
                let (resp, painter) =
                    ui.allocate_painter(grid_size + egui::vec2(keys_w, ruler_h), Sense::click());
                let origin = resp.rect.min + egui::vec2(keys_w, ruler_h);
                let key_y = |key: u16| origin.y + (KEY_TOP - key) as f32 * ROW_H;

                for bar in 0..(PATTERN_TICKS / BAR_TICKS) as u32 {
                    let x = origin.x + bar as f32 * BAR_TICKS * PX_PER_TICK;
                    painter.line_segment(
                        [egui::pos2(x, resp.rect.top()), egui::pos2(x, resp.rect.bottom())],
                        egui::Stroke::new(1.0, C_GRID_STRONG),
                    );
                    painter.text(
                        egui::pos2(x + 3.0, resp.rect.top() + 1.0),
                        Align2::LEFT_TOP,
                        format!("{}", bar + 1),
                        egui::FontId::monospace(10.0),
                        C_TEXT_DIM,
                    );
                }

                for key in KEY_BOTTOM..=KEY_TOP {
                    let y = key_y(key);
                    let is_black = matches!(key % 12, 1 | 3 | 6 | 8 | 10);
                    painter.rect_filled(
                        egui::Rect::from_min_size(egui::pos2(resp.rect.left(), y), egui::vec2(keys_w - 2.0, ROW_H)),
                        1.0,
                        if is_black { C_KEY_BLACK } else { C_KEY_WHITE },
                    );
                    if key % 12 == 0 {
                        painter.text(
                            egui::pos2(resp.rect.left() + 4.0, y + ROW_H / 2.0),
                            Align2::LEFT_CENTER,
                            format!("C{}", key / 12 - 1),
                            egui::FontId::monospace(9.0),
                            Color32::BLACK,
                        );
                    }
                    let row_col = if is_black {
                        Color32::from_rgb(0x2C, 0x2F, 0x33)
                    } else {
                        Color32::from_rgb(0x35, 0x39, 0x3E)
                    };
                    painter.rect_filled(
                        egui::Rect::from_min_size(egui::pos2(origin.x, y), egui::vec2(grid_size.x, ROW_H)),
                        0.0,
                        row_col,
                    );
                    painter.line_segment(
                        [egui::pos2(origin.x, y + ROW_H), egui::pos2(origin.x + grid_size.x, y + ROW_H)],
                        egui::Stroke::new(0.5, C_GRID),
                    );
                }
                for t in 0..=(PATTERN_TICKS / STEP_TICKS) as u32 {
                    let x = origin.x + t as f32 * STEP_TICKS * PX_PER_TICK;
                    let strong = t % 4 == 0;
                    painter.line_segment(
                        [egui::pos2(x, origin.y), egui::pos2(x, origin.y + grid_h)],
                        egui::Stroke::new(
                            if strong { 1.0 } else { 0.5 },
                            if strong { C_GRID_STRONG } else { C_GRID },
                        ),
                    );
                }

                for n in &self.notes {
                    let col = self.channel_color(n.iid);
                    let ghost = n.iid as usize != self.active_channel;
                    let x = origin.x + n.pos as f32 * PX_PER_TICK;
                    let y = key_y(n.key);
                    let r = egui::Rect::from_min_size(
                        egui::pos2(x, y + 1.0),
                        egui::vec2(n.len as f32 * PX_PER_TICK, ROW_H - 2.0),
                    );
                    painter.rect_filled(r, 2.0, if ghost { col.gamma_multiply(0.35) } else { col });
                    // подпись клавиши на ноте (как в FL: «B5», «C5»)
                    if n.len as f32 * PX_PER_TICK > 24.0 && !ghost {
                        let names = ["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"];
                        painter.text(
                            egui::pos2(r.left() + 3.0, r.center().y),
                            Align2::LEFT_CENTER,
                            format!("{}{}", names[(n.key % 12) as usize], n.key / 12 - 1),
                            egui::FontId::monospace(8.0),
                            Color32::from_rgb(0x1E, 0x28, 0x1C),
                        );
                    }
                }

                if self.playing {
                    let x = origin.x + self.pos_ticks * PX_PER_TICK;
                    painter.line_segment(
                        [egui::pos2(x, origin.y), egui::pos2(x, origin.y + grid_h)],
                        egui::Stroke::new(1.5, C_PLAY),
                    );
                }

                // Редактирование (interface-map §5.2): Draw — ЛКМ ставит ноту
                // (длина последней), drag — перемещение, правый край — длина;
                // Delete — удалить выбранную; Alt — обход snap. Всё в памяти.
                let alt = ui.input(|i| i.modifiers.alt);
                let snap_t = if alt { 1.0 } else { self.snap_ticks() as f32 };
                if self.tool == Tool::Delete {
                    if let Some(pos) = resp.interact_pointer_pos() {
                        if pos.x >= origin.x && pos.y >= origin.y && ui.input(|i| i.pointer.primary_down()) {
                            let tick = (pos.x - origin.x) / PX_PER_TICK;
                            let key = KEY_TOP - ((pos.y - origin.y) / ROW_H).floor() as u16;
                            let hit = self.notes.iter().position(|n| {
                                (tick as f32) >= n.pos as f32
                                    && (tick as f32) < n.pos as f32 + n.len as f32
                                    && n.key == key
                            });
                            if let Some(i) = hit {
                                self.push_undo();
                                self.notes.remove(i);
                                if self.selected_note == Some(i) {
                                    self.selected_note = None;
                                }
                            }
                        }
                    }
                } else {
                    // Draw
                    if resp.drag_started() {
                        if let Some(pos) = resp.interact_pointer_pos() {
                            let tick_f = (pos.x - origin.x) / PX_PER_TICK;
                            let key_under =
                                KEY_TOP - ((pos.y - origin.y) / ROW_H).floor() as u16;
                            let mut hit: Option<usize> = None;
                            let mut edge = false;
                            for (i, n) in self.notes.iter().enumerate() {
                                let x0 = origin.x + n.pos as f32 * PX_PER_TICK;
                                let x1 = x0 + n.len as f32 * PX_PER_TICK;
                                let y0 = key_y(n.key);
                                if pos.x >= x0 && pos.x <= x1 && pos.y >= y0 && pos.y <= y0 + ROW_H {
                                    hit = Some(i);
                                    edge = pos.x > x1 - 6.0;
                                    break;
                                }
                            }
                            match hit {
                                Some(i) => {
                                    self.push_undo();
                                    self.selected_note = Some(i);
                                    if edge {
                                        self.roll_drag = Some(RollDrag::Resize { idx: i });
                                    } else {
                                        let (npos, nkey) = (self.notes[i].pos, self.notes[i].key);
                                        self.roll_drag = Some(RollDrag::Move {
                                            idx: i,
                                            dpos: npos.saturating_sub((tick_f / snap_t).floor() as u32 * snap_t as u32),
                                            dkey: nkey.saturating_sub(key_under),
                                        });
                                    }
                                }
                                None => {
                                    // новая нота: pos/key под курсором, длина последней + звук
                                    let pos_t =
                                        ((tick_f / snap_t).floor() * snap_t) as u32;
                                    let iid = self.active_channel as u16;
                                    self.push_undo();
                                    self.notes.push(UiNote {
                                        iid,
                                        pos: pos_t.min(383),
                                        len: self.last_note_len,
                                        key: key_under,
                                        vel: 100,
                                    });
                                    self.selected_note = Some(self.notes.len() - 1);
                                    self.engine.preview(
                                        key_under,
                                        100,
                                        self.last_note_len as f32 * self.secs_per_tick(),
                                        self.gain_for_channel(iid),
                                        self.src_for_channel(iid),
                                    );
                                    self.roll_drag = Some(RollDrag::Move {
                                        idx: self.notes.len() - 1,
                                        dpos: 0,
                                        dkey: 0,
                                    });
                                    self.hint =
                                        "Нота создана (drag — переместить, правый край — длина)"
                                            .into();
                                }
                            }
                        }
                    }
                    let down = ui.input(|i| i.pointer.primary_down());
                    if let Some(d) = self.roll_drag {
                        if down {
                            if let Some(pos) = resp.interact_pointer_pos() {
                                let tick_snapped =
                                    ((pos.x - origin.x) / PX_PER_TICK / snap_t).floor() * snap_t;
                                let key_now = KEY_TOP - ((pos.y - origin.y) / ROW_H).floor() as u16;
                                match d {
                                    RollDrag::Move { idx, dpos, dkey } => {
                                        if let Some(n) = self.notes.get_mut(idx) {
                                            let new_pos = (tick_snapped as u32).saturating_sub(dpos);
                                            n.pos = new_pos.min(383);
                                            n.key = (key_now + dkey).min(127);
                                        }
                                    }
                                    RollDrag::Resize { idx } => {
                                        if let Some(n) = self.notes.get_mut(idx) {
                                            let end = (tick_snapped as f32)
                                                .max(n.pos as f32 + snap_t)
                                                .min(PATTERN_TICKS);
                                            n.len = ((end - n.pos as f32) as u32).max(1);
                                        }
                                    }
                                }
                            }
                        } else {
                            if let RollDrag::Move { idx, .. } = d {
                                if let Some(n) = self.notes.get(idx) {
                                    self.last_note_len = n.len;
                                }
                            }
                            self.roll_drag = None;
                        }
                    }
                }
                // Delete — удалить выбранную
                if ui.input(|i| i.key_pressed(Key::Delete)) {
                    if let Some(i) = self.selected_note {
                        if i < self.notes.len() {
                            self.push_undo();
                            self.notes.remove(i);
                        }
                        self.selected_note = None;
                    }
                }

                // Velocity lane (панель свойств-«леденцов», interface-map §5.1):
                // бары velocity по нотам; драг меняет velocity (в памяти).
                ui.separator();
                let lane_h = 52.0;
                let (lane_rect, lane_resp) = ui.allocate_exact_size(
                    egui::vec2(keys_w + grid_size.x, lane_h),
                    Sense::drag(),
                );
                let lane_p = ui.painter_at(lane_rect);
                lane_p.rect_filled(lane_rect, 1.0, C_PANEL_DARK);
                lane_p.text(
                    egui::pos2(lane_rect.left() + 4.0, lane_rect.top() + 2.0),
                    Align2::LEFT_TOP,
                    "Velocity",
                    egui::FontId::monospace(9.0),
                    C_TEXT_DIM,
                );
                let lane_origin = lane_rect.min + egui::vec2(keys_w, 0.0);
                for (i, n) in self.notes.iter().enumerate() {
                    let x = lane_origin.x + n.pos as f32 * PX_PER_TICK;
                    let w = (n.len as f32 * PX_PER_TICK).max(3.0);
                    let h = (n.vel as f32 / 127.0) * (lane_h - 12.0);
                    let col = self
                        .channel_color(n.iid)
                        .gamma_multiply(if self.selected_note == Some(i) { 1.0 } else { 0.7 });
                    lane_p.rect_filled(
                        egui::Rect::from_min_size(
                            egui::pos2(x, lane_rect.bottom() - h - 2.0),
                            egui::vec2(w, h),
                        ),
                        1.0,
                        col,
                    );
                }
                let dragging_vel = lane_resp.dragged()
                    && lane_resp.interact_pointer_pos().map_or(false, |p| {
                        p.x >= lane_origin.x && p.y >= lane_rect.top() && p.x <= lane_rect.right()
                    });
                if dragging_vel {
                    if let Some(p) = lane_resp.interact_pointer_pos() {
                        let vel = (((lane_rect.bottom() - 2.0 - p.y) / (lane_h - 12.0)).clamp(0.0, 1.0) * 127.0) as u8;
                        let tick = ((p.x - lane_origin.x) / PX_PER_TICK) as u32;
                        let mut changed = 0usize;
                        for n in self.notes.iter_mut() {
                            let x0 = n.pos as f32;
                            let x1 = x0 + n.len as f32;
                            if tick as f32 >= x0 && (tick as f32) < x1 {
                                n.vel = vel;
                                changed += 1;
                            }
                        }
                        if changed > 0 {
                            self.hint = format!("Velocity → {vel} (заглушка: в памяти)");
                        }
                    }
                }
                lane_resp.on_hover_text("Velocity lane: драг по бару ноты меняет velocity (заглушка: в памяти)");

                // панель свойств
                ui.separator();
                ui.horizontal(|ui| {
                    ui.label("Свойства ноты:");
                    if let Some(i) = self.selected_note {
                        if let Some(n) = self.notes.get_mut(i) {
                            ui.add(egui::DragValue::new(&mut n.pos).prefix("pos ").range(0..=383));
                            let mut len = n.len;
                            ui.add(egui::DragValue::new(&mut len).prefix("len ").range(1..=384));
                            n.len = len;
                            self.last_note_len = n.len;
                            ui.add(egui::DragValue::new(&mut n.key).prefix("key ").range(0..=127));
                            ui.add(egui::DragValue::new(&mut n.vel).prefix("vel ").range(0..=127));
                            if ui.button("удалить").clicked() {
                                self.push_undo();
                                self.notes.remove(i);
                                self.selected_note = None;
                            }
                        }
                    } else {
                        ui.label(egui::RichText::new("кликните ноту (заглушка: правки только в памяти)").weak());
                    }
                });
            });
        self.show_roll = open;
    }

    // ---------- Typing keyboard to piano (Ctrl+T, behavior-catalog §2) ----------
    fn typing_piano_window(&mut self, ctx: &egui::Context) {
        let mut open = self.show_piano;
        egui::Window::new("🎹 Typing keyboard to piano — Ctrl+T")
            .open(&mut open)
            .default_pos([560.0, 420.0])
            .default_size([520.0, 140.0])
            .show(ctx, |ui| {
                ui.label(
                    egui::RichText::new(format!(
                        "Клавиши A S D F G H J K = ноты от C{} · целевой канал: {}",
                        4,
                        self.channel_name(self.active_channel as u16)
                    ))
                    .weak()
                    .small(),
                );
                // 8 «клавиш»-кнопок; клик или нажатие — нота
                let keys = [
                    (Key::A, 60usize, "A"),
                    (Key::S, 62, "S"),
                    (Key::D, 64, "D"),
                    (Key::F, 65, "F"),
                    (Key::G, 67, "G"),
                    (Key::H, 69, "H"),
                    (Key::J, 71, "J"),
                    (Key::K, 72, "K"),
                ];
                ui.horizontal(|ui| {
                    let mut hit: Option<usize> = None;
                    for (i, (k, _, label)) in keys.iter().enumerate() {
                        let pressed = ui.input(|i| i.key_pressed(*k));
                        if pressed {
                            hit = Some(i);
                        }
                        if ui.add(
                            egui::Button::new(egui::RichText::new(*label).size(14.0))
                                .min_size(egui::vec2(48.0, 52.0))
                                .fill(if pressed { C_ACCENT } else { C_PANEL_DARK }),
                        )
                        .clicked()
                        {
                            hit = Some(i);
                        }
                    }
                    if let Some(i) = hit {
                        let (_, key, _) = keys[i];
                        let iid = self.active_channel as u16;
                        self.engine.preview(
                            key as u16,
                            100,
                            0.4,
                            self.gain_for_channel(iid),
                            self.src_for_channel(iid),
                        );
                    }
                });
            });
        self.show_piano = open;
    }

    // ---------- Plugin Picker (F8, interface-map §9) ----------
    fn plugin_picker(&mut self, ctx: &egui::Context) {
        let mut open = self.show_picker;
        egui::Window::new("Plugin Picker — F8")
            .open(&mut open)
            .default_pos([520.0, 260.0])
            .default_size([420.0, 240.0])
            .show(ctx, |ui| {
                ui.label(egui::RichText::new("Favorites (генераторы слева, эффекты справа). Клик — заглушка.").weak().small());
                ui.separator();
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.label(egui::RichText::new("Generators").strong().size(10.0));
                        for nm in ["3x Osc", "Fruit Kick", "FL Keys", "Sytrus", "Serum (VST)"] {
                            ui.horizontal(|ui| {
                                icon_img(ui, &self.icons, "instrument", 13.0);
                                if ui.button(nm).clicked() {
                                    self.hint = format!("«{nm}» — заглушка: инструменты не реализованы (нет аудиодвижка)");
                                }
                            });
                        }
                    });
                    ui.separator();
                    ui.vertical(|ui| {
                        ui.label(egui::RichText::new("Effects").strong().size(10.0));
                        for nm in [
                            "Fruity Parametric EQ 2",
                            "Fruity Reeverb 2",
                            "Fruity Limiter",
                            "Edison",
                            "Gross Beat",
                        ] {
                            ui.horizontal(|ui| {
                                icon_img(ui, &self.icons, "audio", 13.0);
                                if ui.button(nm).clicked() {
                                    self.hint = format!("«{nm}» — заглушка: эффекты не реализованы (нет аудиодвижка)");
                                }
                            });
                        }
                    });
                });
            });
        self.show_picker = open;
    }

    // ---------- Mixer (F9, interface-map §7) ----------
    fn mixer_window(&mut self, ctx: &egui::Context) {
        let mut open = self.show_mixer;
        egui::Window::new("🎛 Mixer — F9")
            .open(&mut open)
            .default_pos([380.0, 240.0])
            .default_size([1000.0, 400.0])
            .show(ctx, |ui| {
                ui.label(
                    egui::RichText::new(
                        "Master + insert'ы из файла (18). Фейдеры/пан/слоты — заглушки в памяти; звука нет.",
                    )
                    .weak()
                    .small(),
                );
                ui.separator();
                egui::ScrollArea::horizontal().show(ui, |ui| {
                    ui.horizontal(|ui| {
                        let mut clicked_insert: Option<usize> = None;
                        let mut slot_message: Option<String> = None;
                        for (i, strip) in self.strips.iter_mut().enumerate() {
                            let selected = self.selected_insert == i;
                            // FL: тёмные стрипы; выбранный — зеленоватая подсветка
                            let fill = if selected {
                                Color32::from_rgb(0x3B, 0x4A, 0x35)
                            } else {
                                Color32::from_rgb(0x2C, 0x2E, 0x31)
                            };
                            let frame = egui::Frame::default().fill(fill).inner_margin(6.0);
                            frame.show(ui, |ui| {
                                ui.set_width(74.0);
                                ui.set_min_height(320.0);
                                ui.vertical(|ui| {
                                    let label = if i == 0 {
                                        "Master".to_string()
                                    } else {
                                        strip.name.clone().unwrap_or_else(|| format!("Insert {i}"))
                                    };
                                    let nm = ui
                                        .add(
                                            egui::Label::new(egui::RichText::new(label).size(10.0).strong())
                                                .sense(Sense::click())
                                                .truncate(),
                                        )
                                        .on_hover_text("Имя insert'а из файла (0xCC); клик — выбрать трек");
                                    if nm.clicked() {
                                        clicked_insert = Some(i);
                                    }
                                    for s in &strip.slots {
                                        if ui
                                            .add(
                                                egui::Label::new(
                                                    egui::RichText::new(format!("● {s}")).size(9.0).color(C_PLAY),
                                                )
                                                .sense(Sense::click())
                                                .truncate(),
                                            )
                                            .on_hover_text("FX-слот (0xCB/0xD5 из файла): клик — заглушка GUI плагина")
                                            .clicked()
                                        {
                                            slot_message = Some(format!(
                                                "«{s}» — заглушка: GUI плагинов не реализован (state 0xD5 opaque)"
                                            ));
                                        }
                                    }
                                    ui.with_layout(egui::Layout::bottom_up(egui::Align::Center), |ui| {
                                        let (mrect, _) =
                                            ui.allocate_exact_size(egui::vec2(30.0, 90.0), Sense::hover());
                                        let pt = ui.painter_at(mrect);
                                        pt.rect_filled(mrect, 1.0, C_BG);
                                        for ch in 0..2 {
                                            let h =
                                                strip.level * mrect.height() * if ch == 0 { 0.95 } else { 0.8 };
                                            let r = egui::Rect::from_min_size(
                                                egui::pos2(mrect.left() + 4.0 + ch as f32 * 12.0, mrect.bottom() - h),
                                                egui::vec2(9.0, h),
                                            );
                                            pt.rect_filled(r, 1.0, if strip.level > 0.92 { C_REC } else { C_PLAY });
                                        }
                                        // FL: фейдер выбранного трека — зелёный
                                        if selected {
                                            ui.visuals_mut().widgets.inactive.fg_stroke =
                                                egui::Stroke::new(3.0, C_PLAY);
                                            ui.visuals_mut().widgets.active.fg_stroke =
                                                egui::Stroke::new(3.0, C_PLAY);
                                            ui.visuals_mut().widgets.hovered.fg_stroke =
                                                egui::Stroke::new(3.0, C_PLAY);
                                        }
                                        ui.add(
                                            egui::Slider::new(&mut strip.vol, 0.0..=1.0)
                                                .vertical()
                                                .show_value(false),
                                        )
                                        .on_hover_text("Fader (post-fader) — заглушка");
                                        ui.add(
                                            egui::Slider::new(&mut strip.pan, 0.0..=1.0)
                                                .show_value(false)
                                                .text("Pan"),
                                        )
                                        .on_hover_text("Pan — заглушка");
                                        ui.horizontal(|ui| {
                                            if ui
                                                .selectable_label(strip.mute, "M")
                                                .on_hover_text("Mute — заглушка")
                                                .clicked()
                                            {
                                                strip.mute = !strip.mute;
                                            }
                                            if ui
                                                .selectable_label(strip.solo, "S")
                                                .on_hover_text("Solo — заглушка")
                                                .clicked()
                                            {
                                                strip.solo = !strip.solo;
                                            }
                                        });
                                    });
                                });
                            });
                            ui.separator();
                        }
                        if let Some(i) = clicked_insert {
                            self.selected_insert = i;
                            self.hint = format!("Выбран insert {i} (заглушка: Edit-зона FL не реализована)");
                        }
                        if let Some(msg) = slot_message {
                            self.hint = msg;
                        }
                    });
                });
            });
        self.show_mixer = open;
    }
}
