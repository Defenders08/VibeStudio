//! Аудиодвижок VibeStudio.
//!
//! ЕДИНЫЙ ИСТОЧНИК ВРЕМЕНИ: счётчик семплов двигает только audio-callback;
//! наружу — `AtomicU64` с позицией в миллитиках (tick × 1000) и атомик статуса.
//! UI/CLI/тесты читают только их. Планирование sample-accurate: события
//! стартуют с точным оффсетом внутри блока; смена BPM не ломает позицию.
//!
//! В колбэке нет аллокаций, блокировок (try_lock) и логирования. Голоса —
//! из заранее выделенного пула; тест проверяет это счётчиком аллокатора.
//! Offline-рендер использует тот же `Shared`/планировщик.

use crate::commands::{Command, NoteEvent, PlayMode, TransportStatus};
use crate::dispatch::{build_events, dispatch, History};
use crate::model::ProjectModel;
use crate::voices::VoicePool;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

pub const TICKS_PER_BAR: f64 = 384.0; // PPQ 96 × 4

/// Состояние, общее для колбэка, UI и offline-рендера.
pub struct Shared {
    pub model: ProjectModel,
    pub pattern_id: u32,
    pub bars: u32,
    pub mode: PlayMode,
    pub status: TransportStatus,
    /// Семплы с начала BPM-эпохи (двигает только колбэк / offline-цикл).
    pub epoch_samples: u64,
    /// Тик на границе эпохи.
    pub tick_base: f64,
    pub loop_ticks: f64,
    pub events: Vec<NoteEvent>,
    next_event: usize,
    pub pool: VoicePool,
    kick: Vec<f32>,
    /// Сэмплы каналов, загружаются вне колбэка.
    pub samples: std::collections::HashMap<u32, Arc<Vec<f32>>>,
    /// scratch событий блока (reused — в колбэке без аллокаций).
    due: Vec<(usize, NoteEvent)>,
    /// peak последнего блока (для UI-метров).
    pub peak: f32,
    /// битовая маска каналов с активными голосами (для метров каналов).
    pub active_mask: u32,
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
            events: Vec::new(),
            next_event: 0,
            pool: VoicePool::new(),
            kick: Vec::new(),
            samples: std::collections::HashMap::new(),
            due: Vec::with_capacity(64),
            peak: 0.0,
            active_mask: 0,
        };
        s.set_sr(sr);
        s
    }

    pub fn set_sr(&mut self, sr: f32) {
        self.kick = crate::synth::kick_sample(sr);
    }

    pub fn sr(&self) -> f32 {
        if self.kick.is_empty() {
            44_100.0
        } else {
            self.kick.len() as f32 / 0.35
        }
    }

    fn frames_per_tick(&self) -> f64 {
        self.sr() as f64 * 60.0 / (self.model.bpm * self.model.ppq as f64)
    }

    pub fn tick_now(&self) -> f64 {
        self.tick_base + self.epoch_samples as f64 / self.frames_per_tick()
    }

    /// Для offline-рендера: сброс next_event (после подмены events).
    pub fn next_event_index(&mut self) {
        self.next_event = 0;
    }

    /// Для offline-рендера: включить Playing без replan (events уже подменены).
    pub fn start_playing(&mut self) {
        self.status = TransportStatus::Playing;
    }

    fn resync(&mut self) {
        let t = self.tick_now() as f32;
        self.next_event = 0;
        while self.next_event < self.events.len() && self.events[self.next_event].tick <= t {
            self.next_event += 1;
        }
    }

    /// Перезапланировать события из модели (mode/bars/pattern).
    pub fn replan(&mut self) {
        let (events, loop_ticks) =
            build_events(&self.model, self.pattern_id, self.mode, self.bars);
        self.events = events;
        self.loop_ticks = loop_ticks as f64;
        self.resync();
    }

    /// Применить команду (общий путь колбэка, silent-режима и тестов).
    pub fn apply(&mut self, cmd: &Command) {
        match cmd {
            Command::Play => {
                if self.status == TransportStatus::Paused {
                    // Play после Pause продолжает с сохранённой позиции
                    self.status = TransportStatus::Playing;
                    return;
                }
                if self.mode == PlayMode::Song {
                    // длина цикла по самой дальней клип-границе (клипы могли
                    // добавиться после SetMode)
                    self.bars = self
                        .model
                        .clips
                        .iter()
                        .map(|c| c.bar + c.len_bars)
                        .max()
                        .unwrap_or(4)
                        .max(4);
                }
                self.replan();
                self.epoch_samples = 0;
                self.tick_base = 0.0;
                self.next_event = 0;
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
                self.next_event = 0;
            }
            Command::Seek { tick } => {
                self.tick_base = tick.max(0.0) as f64;
                self.epoch_samples = 0;
                self.resync();
            }
            Command::SetBpm { bpm } => {
                let t = self.tick_now();
                self.model.bpm = bpm.clamp(1.0, 1000.0);
                self.tick_base = t;
                self.epoch_samples = 0;
            }
            Command::SetMode { mode } => {
                self.mode = *mode;
                let pos = self.tick_now();
                self.bars = match self.mode {
                    PlayMode::Song => self
                        .model
                        .clips
                        .iter()
                        .map(|c| c.bar + c.len_bars)
                        .max()
                        .unwrap_or(4)
                        .max(4),
                    PlayMode::Pat => 2,
                };
                self.replan();
                self.tick_base = if pos >= self.loop_ticks {
                    pos % self.loop_ticks
                } else {
                    pos
                };
                self.epoch_samples = 0;
            }
            Command::PreviewNote { channel_id, key, vel, dur_secs } => {
                let (kind, gain, pan) = match self.model.channel(*channel_id) {
                    Some(ch) => (ch.kind, ch.volume, ch.pan),
                    None => return,
                };
                let sample = self.samples.get(channel_id).cloned();
                self.pool.spawn(
                    kind, *key, *dur_secs, gain * *vel as f32 / 127.0, pan, 0,
                    self.sr(), sample,
                );
            }
            Command::SetMasterVolume { volume } => {
                self.model.master_volume = volume.clamp(0.0, 1.5);
            }
            other => {
                dispatch(&mut self.model, other);
            }
        }
    }

    /// Собрать события блока (sample-accurate оффсеты в семплах).
    /// Пишет во внутренний scratch; после вызова позиция продвинута.
    pub fn plan_block(&mut self, frames: usize) {
        self.due.clear();
        if self.status != TransportStatus::Playing {
            return;
        }
        let fpt = self.frames_per_tick();
        let start_tick = self.tick_now();
        let end_tick = start_tick + frames as f64 / fpt;

        while self.next_event < self.events.len()
            && (self.events[self.next_event].tick as f64) < end_tick
        {
            let ev = self.events[self.next_event];
            self.next_event += 1;
            if (ev.tick as f64) < start_tick {
                continue;
            }
            let off = ((ev.tick as f64 - start_tick) * fpt) as usize;
            if off < frames {
                self.due.push((off, ev));
            }
        }

        if end_tick >= self.loop_ticks {
            // события за границей цикла перечитаются (tick_base → 0, next_event → 0)
            self.epoch_samples = 0;
            self.tick_base = 0.0;
            self.next_event = 0;
        } else {
            self.epoch_samples += frames as u64;
        }
    }

    pub fn spawn_block(&mut self) {
        // take→обработка→возврат: capacity возвращается (в колбэке без аллокаций)
        let due = std::mem::take(&mut self.due);
        self.active_mask = 0;
        for (off, ev) in due.iter() {
            if ev.channel_id < 32 {
                self.active_mask |= 1 << ev.channel_id;
            }
            let Some(ch) = self.model.channel(ev.channel_id) else { continue };
            let dur_secs = (ev.dur_ticks as f64 / self.frames_per_tick()).max(0.01) as f32;
            let sample = self.samples.get(&ev.channel_id).cloned();
            self.pool.spawn(
                ch.kind,
                ev.key,
                dur_secs,
                ev.gain * ev.vel as f32 / 127.0,
                ev.pan,
                *off,
                self.sr(),
                sample,
            );
        }
        self.due = due;
        self.due.clear();
    }

    /// Микс блока + мастер-цепочка (запас 0.8, мягкий лимитер) + peak.
    pub fn mix_block_master(&mut self, out: &mut [f32]) {
        self.pool.mix_block(out, &self.kick);
        let master = self.model.master_volume * 0.8;
        let mut peak = 0.0f32;
        for f in out.chunks_exact_mut(2) {
            f[0] = (f[0] * master) / (1.0 + (f[0] * master).abs());
            f[1] = (f[1] * master) / (1.0 + (f[1] * master).abs());
            peak = peak.max(f[0].abs()).max(f[1].abs());
        }
        // плавный спад
        self.peak = peak.max(self.peak * 0.85);
    }

    /// Планирование + спавн + микс одним вызовом (для offline-рендера).
    pub fn render_frame(&mut self, out: &mut [f32]) {
        self.plan_block(out.len() / 2);
        self.spawn_block();
        self.mix_block_master(out);
    }
}

/// Хэндл движка (владение у UI; Stream живёт, пока жив Engine).
pub struct Engine {
    cmd_tx: rtrb::Producer<Command>,
    milli: Arc<AtomicU64>,
    status_bits: Arc<AtomicU32>,
    peak_bits: Arc<AtomicU32>,
    active_bits: Arc<AtomicU32>,
    shared: Arc<Mutex<Shared>>,
    history: History,
    stream: Option<cpal::Stream>,
    /// Нет аудиоустройства: команды применяются напрямую, звук — offline.
    pub silent: bool,
}

impl Engine {
    pub fn new() -> Engine {
        let (tx, mut rx) = rtrb::RingBuffer::<Command>::new(256);
        let milli = Arc::new(AtomicU64::new(0));
        let status_bits = Arc::new(AtomicU32::new(0));
        let peak_bits = Arc::new(AtomicU32::new(0));
        let active_bits = Arc::new(AtomicU32::new(0));
        let shared = Arc::new(Mutex::new(Shared::new(44_100.0)));

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
                let active_c = active_bits.clone();
                let sh_c = shared.clone();
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
                    while let Ok(cmd) = rx.pop() {
                        sh.apply(&cmd);
                    }
                    sh.render_frame(data);
                    status_c.store(sh.status as u32, Ordering::Relaxed);
                    milli_c.store((sh.tick_now() * 1000.0).max(0.0) as u64, Ordering::Relaxed);
                    peak_c.store(sh.peak.to_bits(), Ordering::Relaxed);
                    active_c.store(sh.active_mask, Ordering::Relaxed);
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
            milli,
            status_bits,
            peak_bits,
            active_bits,
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

    /// Peak мастер-выхода (для метра Mixer) — атомик из колбэка.
    pub fn peak(&self) -> f32 {
        f32::from_bits(self.peak_bits.load(Ordering::Relaxed))
    }

    /// Маска каналов с активными голосами (для метров каналов).
    pub fn active_mask(&self) -> u32 {
        self.active_bits.load(Ordering::Relaxed)
    }

    pub fn status_direct(&self) -> TransportStatus {
        self.shared.lock().unwrap().status
    }

    pub fn tick_direct(&self) -> f64 {
        self.shared.lock().unwrap().tick_now()
    }

    /// Единая дверь для всех команд. Модельные проходят через dispatch
    /// (undo-стек получает обратную команду), транспортные — в кольцо.
    pub fn execute(&mut self, cmd: Command) {
        let is_model = matches!(
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
        );
        if is_model {
            if let Ok(mut sh) = self.shared.lock() {
                if let Some(inv) = dispatch(&mut sh.model, &cmd) {
                    self.history.push_undo(inv);
                }
            }
        }
        if self.silent {
            if let Ok(mut sh) = self.shared.lock() {
                sh.apply(&cmd);
            }
            return;
        }
        let _ = self.cmd_tx.push(cmd);
    }

    pub fn undo(&mut self) -> bool {
        match self.history.undo() {
            Some(cmd) => {
                if let Ok(mut sh) = self.shared.lock() {
                    dispatch(&mut sh.model, &cmd);
                }
                true
            }
            None => false,
        }
    }

    pub fn redo(&mut self) -> bool {
        match self.history.redo() {
            Some(cmd) => {
                if let Ok(mut sh) = self.shared.lock() {
                    dispatch(&mut sh.model, &cmd);
                }
                true
            }
            None => false,
        }
    }

    /// Доступ к Shared на UI-потоке (рендер, загрузка сэмплов).
    pub fn lock(&self) -> std::sync::MutexGuard<'_, Shared> {
        self.shared.lock().unwrap()
    }

    /// Снапшот модели (UI рендерит по нему).
    pub fn model_snapshot(&self) -> ProjectModel {
        self.shared.lock().unwrap().model.clone()
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

    /// Сохранить проект (JSON, версия формата в модели).
    pub fn save_project(&self, path: &str) -> Result<(), String> {
        let model = self.shared.lock().unwrap().model.clone();
        let json = serde_json::to_string_pretty(&model).map_err(|e| format!("{e}"))?;
        std::fs::write(path, json).map_err(|e| format!("{e}"))
    }

    /// Загрузить проект; обратная команда (старая модель) уйдёт в undo.
    pub fn load_project_file(&mut self, path: &str) -> Result<(), String> {
        let json = std::fs::read_to_string(path).map_err(|e| format!("{e}"))?;
        let model: ProjectModel =
            serde_json::from_str(&json).map_err(|e| format!("формат: {e}"))?;
        self.execute(Command::LoadProject { model });
        Ok(())
    }
}
