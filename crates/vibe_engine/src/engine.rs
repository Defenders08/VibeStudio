//! Аудиодвижок VibeStudio — архитектура по образцу LMMS AudioEngine.
//!
//! Принципы (проверено по lmms/src/core/AudioEngine.cpp, Song.cpp):
//! 1. МОДЕЛЬ ЖИВЁТ В RT-ПОТОКЕ. Транспорт и планирование нот — в аудио-колбэке
//!    (у LMMS: Song::processNextBuffer вызывается из renderStageNoteSetup
//!    каждый период). UI никогда не мутирует RT-состояние: только команды
//!    через SPSC-кольцо. Обратные команды для undo возвращаются RT → UI
//!    через второе кольцо.
//! 2. ПЛАНИРОВАНИЕ СКАНИРОВАНИЕМ: каждый блок сканируется окно тиков
//!    [tick, tick+frames/fpt) по живой модели — правки паттернов слышны
//!    мгновенно, без снапшотов и replan.
//! 3. ЕДИНЫЙ ИСТОЧНИК ВРЕМЕНИ: счётчик семплов двигает только колбэк; наружу
//!    AtomicU64 миллитиков + атомик статуса.
//! 4. В колбэке нет аллокаций/блокировок (try_lock с пропуском блока),
//!    голоса из пула (у LMMS: NotePlayHandleManager::acquire/release).
//! 5. Снапшот модели наружу — publish-слот (публикуется на модельную
//!    команду; Arc-клон, UI читает без ожидания RT).

use crate::commands::{Command, NoteEvent, PlayMode, TransportStatus};
use crate::dispatch::{dispatch, History};
use crate::model::{ChannelKind, ProjectModel};
use crate::voices::VoicePool;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

pub const TICKS_PER_BAR: f64 = 384.0; // PPQ 96 × 4
const MAX_CHANNELS: usize = 32;

/// Состояние RT (владеет аудио-поток; UI — только publish-снапшоты).
pub struct Shared {
    pub model: ProjectModel,
    pub pattern_id: u32,
    pub bars: u32, // длина цикла PAT (в тактах)
    pub mode: PlayMode,
    pub status: TransportStatus,
    /// Семплы с начала BPM-эпохи (двигает только колбэк / offline-цикл).
    pub epoch_samples: u64,
    /// Тик на границе эпохи.
    pub tick_base: f64,
    pub loop_ticks: f64,
    pub pool: VoicePool,
    kick: Vec<f32>,
    /// Сэмплы каналов (channel_id → моно f32), загружаются вне колбэка.
    pub samples: std::collections::HashMap<u32, Arc<Vec<f32>>>,
    /// Живые параметры каналов для голосов (синхронизируются из модели).
    pub ch_vol: [f32; MAX_CHANNELS],
    pub ch_pan: [f32; MAX_CHANNELS],
    /// Peak по каналам за последний блок (спад здесь же).
    pub ch_peak: [f32; MAX_CHANNELS],
    pub peak: f32,
    /// scratch планирования (reused — в колбэке без аллокаций).
    due: Vec<NoteEvent>,
    due_offs: Vec<usize>,
}

impl Shared {
    pub fn new(sr: f32) -> Self {
        let mut s = Shared {
            model: ProjectModel::default(),
            pattern_id: 1,
            bars: 2,
            mode: PlayMode::Pat,
            status: TransportStatus::Stopped,
            epoch_samples: 0,
            tick_base: 0.0,
            loop_ticks: 2.0 * TICKS_PER_BAR,
            pool: VoicePool::new(),
            kick: Vec::new(),
            samples: std::collections::HashMap::new(),
            ch_vol: [0.78; MAX_CHANNELS],
            ch_pan: [0.0; MAX_CHANNELS],
            ch_peak: [0.0; MAX_CHANNELS],
            peak: 0.0,
            due: Vec::with_capacity(64),
            due_offs: Vec::with_capacity(64),
        };
        s.set_sr(sr);
        s.sync_channel_params();
        s
    }

    pub fn set_sr(&mut self, sr: f32) {
        self.kick = crate::synth::kick_sample(sr);
    }

    pub fn sr(&self) -> f32 {
        if self.kick.is_empty() { 44_100.0 } else { self.kick.len() as f32 / 0.35 }
    }

    fn frames_per_tick(&self) -> f64 {
        self.sr() as f64 * 60.0 / (self.model.bpm * self.model.ppq as f64)
    }

    pub fn tick_now(&self) -> f64 {
        self.tick_base + self.epoch_samples as f64 / self.frames_per_tick()
    }

    /// Скопировать volume/pan из модели в RT-массивы (без аллокаций).
    fn sync_channel_params(&mut self) {
        for ch in &self.model.channels {
            if (ch.id as usize) < MAX_CHANNELS {
                self.ch_vol[ch.id as usize] = ch.volume;
                self.ch_pan[ch.id as usize] = ch.pan;
            }
        }
    }

    pub fn song_bars(&self) -> u32 {
        self.model
            .clips
            .iter()
            .map(|c| c.bar + c.len_bars)
            .max()
            .unwrap_or(4)
            .max(4)
    }

    /// Длина ПАТТЕРНА в тактах по содержимому: сколько шагов/нот написано,
    /// столько и играет (минимум 1 такт). «Сколько квадратов — столько играет».
    pub fn pat_bars(&self) -> u32 {
        let Some(p) = self.model.pattern(self.pattern_id) else { return 1 };
        let max_step = p
            .steps
            .iter()
            .map(|(_, s)| *s)
            .chain(p.notes.iter().map(|(_, n)| n.step + n.len_steps.max(1)))
            .max()
            .unwrap_or(0);
        ((max_step / 16) + 1).max(1) // 16 шагов на такт
    }

    /// Применить команду (общий путь колбэка, silent-режима и тестов).
    pub fn apply(&mut self, cmd: &Command) {
        match cmd {
            Command::Play => {
                if self.status == TransportStatus::Paused {
                    self.status = TransportStatus::Playing; // продолжить с позиции
                    return;
                }
                self.loop_ticks = match self.mode {
                    PlayMode::Song => self.song_bars() as f64 * TICKS_PER_BAR,
                    PlayMode::Pat => self.pat_bars() as f64 * TICKS_PER_BAR,
                };
                self.epoch_samples = 0;
                self.tick_base = 0.0;
                self.status = TransportStatus::Playing;
            }
            Command::Pause => {
                if self.status == TransportStatus::Playing {
                    self.status = TransportStatus::Paused;
                    self.pool.kill_all();
                }
            }
            Command::Stop => {
                self.status = TransportStatus::Stopped;
                self.pool.kill_all();
                self.epoch_samples = 0;
                self.tick_base = 0.0;
            }
            Command::Seek { tick } => {
                self.tick_base = tick.max(0.0) as f64;
                self.epoch_samples = 0;
            }
            Command::SetBpm { bpm } => {
                // позиция в тиках сохраняется; семплы эпохи обнуляются
                let t = self.tick_now();
                self.model.bpm = bpm.clamp(1.0, 1000.0);
                self.tick_base = t;
                self.epoch_samples = 0;
            }
            Command::SetMode { mode } => {
                self.mode = *mode;
                let pos = self.tick_now();
                self.loop_ticks = match self.mode {
                    PlayMode::Song => self.song_bars() as f64 * TICKS_PER_BAR,
                    PlayMode::Pat => self.pat_bars() as f64 * TICKS_PER_BAR,
                };
                self.tick_base = if pos >= self.loop_ticks { pos % self.loop_ticks } else { pos };
                self.epoch_samples = 0;
            }
            Command::PreviewNote { channel_id, key, vel, dur_secs } => {
                let (kind, gain, pan) = match self.model.channel(*channel_id) {
                    Some(ch) => (ch.kind, ch.volume, ch.pan),
                    None => return,
                };
                let sample = self.samples.get(channel_id).cloned();
                self.pool.spawn(
                    kind, *channel_id, *key, *dur_secs,
                    gain * *vel as f32 / 127.0, 0, self.sr(), sample,
                );
            }
            Command::SetMasterVolume { volume } => {
                self.model.master_volume = volume.clamp(0.0, 1.5);
            }
            // модельные — через диспетчер; клипы в SONG меняют длину цикла
            other => {
                let pattern_edit = matches!(
                    other,
                    Command::SetStep { .. }
                        | Command::SetNote { .. }
                        | Command::RemoveNote { .. }
                );
                dispatch(&mut self.model, other);
                self.sync_channel_params();
                if self.mode == PlayMode::Song {
                    self.loop_ticks = self.song_bars() as f64 * TICKS_PER_BAR;
                } else if pattern_edit {
                    self.loop_ticks = self.pat_bars() as f64 * TICKS_PER_BAR;
                }
            }
        }
    }

    pub fn spawn_block(&mut self, due: &[NoteEvent], offs: &[usize]) {
        for (ev, off) in due.iter().zip(offs.iter()) {
            let dur_secs = (ev.dur_ticks as f64 / self.frames_per_tick()).max(0.01) as f32;
            let sample = self.samples.get(&ev.channel_id).cloned();
            self.pool.spawn(
                ev.kind,
                ev.channel_id,
                ev.key,
                dur_secs,
                ev.gain * ev.vel as f32 / 127.0,
                *off,
                self.sr(),
                sample,
            );
        }
    }

    /// Микс + мастер (запас 0.8, мягкий лимитер) + пер-канальные пики.
    pub fn mix_block_master(&mut self, out: &mut [f32]) {
        self.pool
            .mix_block(out, &self.kick, &self.ch_vol, &self.ch_pan, &mut self.ch_peak);
        let master = self.model.master_volume * 0.8;
        let mut peak = 0.0f32;
        for f in out.chunks_exact_mut(2) {
            f[0] = (f[0] * master) / (1.0 + (f[0] * master).abs());
            f[1] = (f[1] * master) / (1.0 + (f[1] * master).abs());
            peak = peak.max(f[0].abs()).max(f[1].abs());
        }
        self.peak = peak.max(self.peak * 0.85);
    }

    /// Offline: играть заранее заданный набор (render.rs сам сканирует модель,
    /// но для явных events — старт Playing; события в render_to_wav задаются
    /// через модель, поэтому этот метод просто включает транспорт).
    pub fn start_playing_with(&mut self, _events: &[NoteEvent], loop_ticks: f64) {
        self.loop_ticks = loop_ticks;
        self.status = TransportStatus::Playing;
    }

    /// Один кадр offline-рендера (тот же путь, что колбэк).
    /// scratch-буферы внутренние — в колбэке без аллокаций.
    pub fn render_frame(&mut self, out: &mut [f32]) {
        let (due, offs) = self.plan_block(out.len() / 2);
        self.spawn_block(&due, &offs);
        self.mix_block_master(out);
    }

    fn plan_block(&mut self, frames: usize) -> (Vec<NoteEvent>, Vec<usize>) {
        self.due.clear();
        self.due_offs.clear();
        self.plan_block_into(frames);
        (std::mem::take(&mut self.due), std::mem::take(&mut self.due_offs))
    }

    /// Ядро планирования (out/offs — external scratch, для тестов).
    fn plan_block_into(&mut self, frames: usize) {
        self.due.clear();
        self.due_offs.clear();
        if std::env::var("VIBE_DBG").is_ok() {
            eprintln!(
                "[plan] start={:.1} dt={:.1} loop={:.1} mode={:?} clips={} steps={}",
                self.tick_now(),
                frames as f64 / self.frames_per_tick(),
                self.loop_ticks,
                self.mode,
                self.model.clips.len(),
                self.model.patterns.iter().map(|p| p.steps.len()).sum::<usize>(),
            );
        }
        for p in self.ch_peak.iter_mut() {
            *p *= 0.82; // спад метра
        }
        self.peak *= 0.82;
        if self.status != TransportStatus::Playing {
            return;
        }
        let fpt = self.frames_per_tick();
        let dt = frames as f64 / fpt;
        let start = self.tick_now();
        let end = start + dt;
        if end <= self.loop_ticks {
            scan_window_into(&self.model, fpt, self.mode, self.pattern_id, &self.model.muted_tracks, start, end, 0, &mut self.due, &mut self.due_offs);
            self.epoch_samples += frames as u64;
        } else {
            let wrap_at = ((self.loop_ticks - start) * fpt) as usize;
            scan_window_into(&self.model, fpt, self.mode, self.pattern_id, &self.model.muted_tracks, start, self.loop_ticks, 0, &mut self.due, &mut self.due_offs);
            scan_window_into(&self.model, fpt, self.mode, self.pattern_id, &self.model.muted_tracks, 0.0, end - self.loop_ticks, wrap_at, &mut self.due, &mut self.due_offs);
            self.epoch_samples = 0;
            self.tick_base = 0.0;
        }
    }
}

/// Сканировать окно тиков [s, e) по живой модели.
/// PAT — один паттерн; SONG — клипы (не muted), каждый со своим смещением.
#[allow(clippy::too_many_arguments)]
fn scan_window_into(
    model: &ProjectModel,
    fpt: f64,
    mode: PlayMode,
    pattern_id: u32,
    muted_tracks: &[u32],
    s: f64,
    e: f64,
    base_off: usize,
    out: &mut Vec<NoteEvent>,
    offs: &mut Vec<usize>,
) {
    if e <= 0.0 {
        return;
    }
    let emit = |p: &crate::model::Pattern, offset_ticks: f64, out: &mut Vec<NoteEvent>, offs: &mut Vec<usize>| {
        // клип = экземпляр паттерна: окно переводим в локальные тики паттерна
        let (ls, le) = (s - offset_ticks, e - offset_ticks);
        for (ch_id, step) in &p.steps {
            let t = *step as f64 * 24.0;
            if t >= ls && t < le {
                if let Some(ch) = model.channel(*ch_id) {
                    if ch.muted {
                        continue;
                    }
                    out.push(NoteEvent {
                        tick: (offset_ticks + t) as f32,
                        channel_id: *ch_id,
                        key: 60,
                        vel: 100,
                        dur_ticks: 24.0,
                        gain: ch.volume,
                        pan: ch.pan,
                        kind: ch.kind,
                    });
                    offs.push(base_off + (((t - s) * fpt) as usize));
                }
            }
        }
        for (ch_id, n) in &p.notes {
            let t = n.step as f64 * 24.0;
            if t >= ls && t < le {
                if let Some(ch) = model.channel(*ch_id) {
                    if ch.muted {
                        continue;
                    }
                    out.push(NoteEvent {
                        tick: (offset_ticks + t) as f32,
                        channel_id: *ch_id,
                        key: n.key,
                        vel: n.vel,
                        dur_ticks: n.len_steps as f32 * 24.0,
                        gain: ch.volume,
                        pan: ch.pan,
                        kind: ch.kind,
                    });
                    offs.push(base_off + (((t - s) * fpt) as usize));
                }
            }
        }
    };
    match mode {
        PlayMode::Pat => {
            if let Some(p) = model.pattern(pattern_id) {
                emit(p, 0.0, out, offs);
            }
        }
        PlayMode::Song => {
            for c in &model.clips {
                if c.muted || muted_tracks.contains(&c.track) {
                    continue;
                }
                for b in 0..c.len_bars {
                    let off_ticks = (c.bar + b) as f64 * TICKS_PER_BAR;
                    if let Some(p) = model.pattern(c.pattern_id) {
                        emit(p, off_ticks, out, offs);
                    }
                }
            }
        }
    }
}

/// Выдать события паттерна в окне [s, e) (локальные тики паттерна).
fn emit_pattern(
    sh: &mut Shared,
    p: &crate::model::Pattern,
    s: f64,
    e: f64,
    offset_ticks: f64,
    base_off: usize,
    fpt: f64,
    out: &mut Vec<NoteEvent>,
    offs: &mut Vec<usize>,
) {
    if e <= 0.0 {
        return;
    }
    for (ch_id, step) in &p.steps {
        let t = *step as f64 * 24.0; // шаг 1/16 = 24 тика
        if t >= s && t < e {
            if let Some(ch) = sh.model.channel(*ch_id) {
                if ch.muted {
                    continue;
                }
                out.push(NoteEvent {
                    tick: (offset_ticks + t) as f32,
                    channel_id: *ch_id,
                    key: 60,
                    vel: 100,
                    dur_ticks: 24.0,
                    gain: ch.volume,
                    pan: ch.pan,
                    kind: ch.kind,
                });
                offs.push(base_off + (((t - s) * fpt) as usize));
            }
        }
    }
    for (ch_id, n) in &p.notes {
        let t = n.step as f64 * 24.0;
        if t >= s && t < e {
            if let Some(ch) = sh.model.channel(*ch_id) {
                if ch.muted {
                    continue;
                }
                out.push(NoteEvent {
                    tick: (offset_ticks + t) as f32,
                    channel_id: *ch_id,
                    key: n.key,
                    vel: n.vel,
                    dur_ticks: n.len_steps as f32 * 24.0,
                    gain: ch.volume,
                    pan: ch.pan,
                    kind: ch.kind,
                });
                offs.push(base_off + (((t - s) * fpt) as usize));
            }
        }
    }
}

/// Хэндл движка (владение у UI; Stream живёт, пока жив Engine).
pub struct Engine {
    cmd_tx: rtrb::Producer<Command>,
    /// RT → UI: обратные команды для undo (вычисляет колбэк).
    hist_rx: rtrb::Consumer<Command>,
    milli: Arc<AtomicU64>,
    status_bits: Arc<AtomicU32>,
    peak_bits: Arc<AtomicU32>,
    peaks8_bits: Arc<AtomicU64>,
    /// Публикация модели наружу (UI читает снапшот, RT публикует после команд).
    publish: Arc<Mutex<Arc<ProjectModel>>>,
    shared: Arc<Mutex<Shared>>,
    history: History,
    stream: Option<cpal::Stream>,
    /// Нет аудиоустройства: команды применяются напрямую, звук — offline.
    pub silent: bool,
}

impl Engine {
    pub fn new() -> Engine {
        let (tx, mut rx) = rtrb::RingBuffer::<Command>::new(256);
        let (hist_tx, hist_rx) = rtrb::RingBuffer::<Command>::new(256);
        let mut hist_tx = hist_tx;
        let milli = Arc::new(AtomicU64::new(0));
        let status_bits = Arc::new(AtomicU32::new(0));
        let peak_bits = Arc::new(AtomicU32::new(0));
        let peaks8_bits = Arc::new(AtomicU64::new(0));
        let shared = Arc::new(Mutex::new(Shared::new(44_100.0)));
        let publish = Arc::new(Mutex::new(Arc::new(shared.lock().unwrap().model.clone())));
        let mut stream = None;
        let mut silent = true;
        {
            let host = cpal::default_host();
            let run = || -> Result<cpal::Stream, Box<dyn std::error::Error>> {
                let device = host.default_output_device().ok_or("нет output-устройства")?;
                let cfg = device.default_output_config()?;
                if cfg.sample_format() != cpal::SampleFormat::F32 || cfg.channels() < 2 {
                    return Err("устройство не f32/стерео".into());
                }
                let sr = cfg.sample_rate().0 as f32;
                {
                    let mut sh = shared.lock().unwrap();
                    sh.set_sr(sr);
                }
                let milli_c = milli.clone();
                let status_c = status_bits.clone();
                let peak_c = peak_bits.clone();
                let peaks8_c = peaks8_bits.clone();
                let sh_c = shared.clone();
                let pub_c = publish.clone();
                let mut hist_t = hist_tx; // move: Producer не Clone
                let err_cb = |e| eprintln!("[vibe_engine] stream error: {e}");
                let data_cb = move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                    // БЕЗ аллокаций/блокировок/логов: try_lock, занято — тишина.
                    let mut sh = match sh_c.try_lock() {
                        Ok(g) => g,
                        Err(_) => {
                            data.fill(0.0);
                            return;
                        }
                    };
                    let mut model_changed = false;
                    while let Ok(cmd) = rx.pop() {
                        sh.apply(&cmd);
                        if is_model_cmd(&cmd) {
                            model_changed = true;
                            if let Some(inv) = crate::dispatch::inverse_of(&cmd, &sh.model) {
                                let _ = hist_t.push(inv);
                            }
                        }
                    }
                    sh.render_frame(data);
                    status_c.store(sh.status as u32, Ordering::Relaxed);
                    milli_c.store((sh.tick_now() * 1000.0).max(0.0) as u64, Ordering::Relaxed);
                    peak_c.store(sh.peak.to_bits(), Ordering::Relaxed);
                    peaks8_c.store(pack_peaks8(&sh.ch_peak), Ordering::Relaxed);
                    // публикация модели — только на модельную команду
                    if model_changed {
                        if let Ok(mut p) = pub_c.try_lock() {
                            *p = Arc::new(sh.model.clone());
                        }
                    }
                };
                let stream =
                    device.build_output_stream(&cfg.into(), data_cb, err_cb, None)?;
                stream.play().ok();
                Ok(stream)
            };
            match run() {
                Ok(s) => {
                    silent = false;
                    stream = Some(s);
                }
                Err(e) => {
                    eprintln!("[vibe_engine] звук недоступен ({e}); offline-рендер работает");
                }
            }
        }

        Engine {
            cmd_tx: tx,
            hist_rx,
            milli,
            status_bits,
            peak_bits,
            peaks8_bits,
            publish,
            shared,
            history: History::default(),
            stream,
            silent,
        }
    }

    /// Транспорт-снапшот: (статус, позиция в тиках) — единый источник для UI.
    pub fn transport(&self) -> (TransportStatus, f64) {
        (
            match self.status_bits.load(Ordering::Relaxed) {
                1 => TransportStatus::Playing,
                2 => TransportStatus::Paused,
                _ => TransportStatus::Stopped,
            },
            self.milli.load(Ordering::Relaxed) as f64 / 1000.0,
        )
    }

    /// Peak мастер-выхода (метр Mixer).
    pub fn peak(&self) -> f32 {
        f32::from_bits(self.peak_bits.load(Ordering::Relaxed))
    }

    /// Peak канала (упаковка 8×8 бит; каналы 0..8).
    pub fn channel_peak(&self, id: u32) -> f32 {
        if id >= 8 {
            return 0.0;
        }
        let packed = self.peaks8_bits.load(Ordering::Relaxed);
        ((packed >> (id * 8)) & 0xFF) as f32 / 255.0
    }

    /// Перекачать обратные команды RT → undo-история (вызывать каждый кадр UI).
    pub fn pump_history(&mut self) {
        while let Ok(inv) = self.hist_rx.pop() {
            self.history.push_undo(inv);
        }
    }

    /// Единая дверь команд. Модельные — ТОЛЬКО в RT (у LMMS модель мутирует
    /// RT-поток; UI лишь просит изменение). Silent — напрямую + undo сразу.
    pub fn execute(&mut self, cmd: Command) {
        if self.silent {
            if let Ok(mut sh) = self.shared.lock() {
                sh.apply(&cmd);
                if is_model_cmd(&cmd) {
                    if let Some(inv) = crate::dispatch::inverse_of(&cmd, &sh.model) {
                        self.history.push_undo(inv);
                    }
                }
            }
            if is_model_cmd(&cmd) {
                if let Ok(mut p) = self.publish.lock() {
                    *p = Arc::new(self.shared.lock().unwrap().model.clone());
                }
            }
            return;
        }
        let _ = self.cmd_tx.push(cmd);
    }

    pub fn undo(&mut self) -> bool {
        match self.history.undo() {
            Some(cmd) => {
                self.execute(cmd);
                self.pump_history();
                true
            }
            None => false,
        }
    }

    pub fn redo(&mut self) -> bool {
        match self.history.redo() {
            Some(cmd) => {
                self.execute(cmd);
                self.pump_history();
                true
            }
            None => false,
        }
    }

    /// Снапшот модели для рендера (publish-слот; без ожидания RT).
    pub fn model_snapshot(&self) -> ProjectModel {
        (**self.publish.lock().unwrap()).clone()
    }

    /// Доступ к Shared на UI-потоке (загрузка сэмплов).
    pub fn lock(&self) -> std::sync::MutexGuard<'_, Shared> {
        self.shared.lock().unwrap()
    }

    /// Сохранить проект (JSON, версия формата в модели).
    pub fn save_project(&self, path: &str) -> Result<(), String> {
        let model = self.model_snapshot();
        let json = serde_json::to_string_pretty(&model).map_err(|e| format!("{e}"))?;
        std::fs::write(path, json).map_err(|e| format!("{e}"))
    }

    /// Загрузить проект; старая модель уйдёт в undo.
    pub fn load_project_file(&mut self, path: &str) -> Result<(), String> {
        let json = std::fs::read_to_string(path).map_err(|e| format!("{e}"))?;
        let model: ProjectModel =
            serde_json::from_str(&json).map_err(|e| format!("формат: {e}"))?;
        self.execute(Command::LoadProject { model });
        Ok(())
    }

    /// Загрузить WAV-сэмпл канала (UI-поток, hound; 16/24/32-bit).
    pub fn load_sample(&mut self, channel_id: u32, path: &str) -> Result<usize, String> {
        let mut reader = hound::WavReader::open(path).map_err(|e| format!("wav: {e}"))?;
        let spec = reader.spec();
        let mut data = Vec::new();
        for s in reader.samples::<i32>() {
            let v = match spec.bits_per_sample {
                16 => s.unwrap() as f32 / i16::MAX as f32,
                24 => s.unwrap() as f32 / ((1 << 23) - 1) as f32,
                32 => f32::from_bits(s.unwrap() as u32),
                _ => return Err("формат битов не 16/24/32".into()),
            };
            data.push(v);
        }
        let len = data.len();
        self.shared.lock().unwrap().samples.insert(channel_id, Arc::new(data));
        self.execute(Command::SetSamplePath { channel_id, path: path.into() });
        Ok(len)
    }
}

fn is_model_cmd(cmd: &Command) -> bool {
    matches!(
        cmd,
        Command::SetBpm { .. }
            | Command::SetMasterVolume { .. }
            | Command::AddChannel { .. }
            | Command::RemoveChannel { .. }
            | Command::SetChannelVolume { .. }
            | Command::SetChannelPan { .. }
            | Command::SetChannelMute { .. }
            | Command::SetSamplePath { .. }
            | Command::SetStep { .. }
            | Command::SetNote { .. }
            | Command::RemoveNote { .. }
            | Command::RenamePattern { .. }
            | Command::AddClip { .. }
            | Command::MoveClip { .. }
            | Command::ResizeClip { .. }
            | Command::RemoveClip { .. }
            | Command::SetClipMute { .. }
            | Command::SetTrackMute { .. }
            | Command::LoadProject { .. }
    )
}

/// Упаковать пики первых 8 каналов в u64 (байт на канал).
fn pack_peaks8(peaks: &[f32]) -> u64 {
    let mut v = 0u64;
    for (i, p) in peaks.iter().take(8).enumerate() {
        v |= ((p.clamp(0.0, 1.0) * 255.0) as u64) << (i * 8);
    }
    v
}
