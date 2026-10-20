//! Аудиодвижок: cpal-вывод + планировщик событий + голоса.
//!
//! Единый источник времени: курсор в `PlayState` (кадры), публикуется атомарно
//! (`pos_bits`) из аудио-колбэка; UI и offline-рендер читают только его.
//! RT-дисциплина прототипа: команды — через rtrb-кольцо (без аллокаций в
//! колбэке); состояние воспроизведения — под коротким Mutex (известный долг:
//! заменить на тройной буфер по мере роста; см. OSS_AUDIT — паттерн maolan).
//! Микширование и планирование общие с offline-рендером («what you hear is
//! what you render», паттерн vibez).

use crate::registry::{EngineCommand, NoteEvent, PlayMode, VoiceSrc};
use crate::synth;
use crate::voice::Voice;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};

/// Статус транспорта (публикуется атомарно; 0=Stopped, 1=Playing, 2=Paused).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransportStatus {
    Stopped,
    Playing,
    Paused,
}

impl TransportStatus {
    fn from_u8(v: u8) -> Self {
        match v {
            1 => TransportStatus::Playing,
            2 => TransportStatus::Paused,
            _ => TransportStatus::Stopped,
        }
    }
    fn to_u8(self) -> u8 {
        match self {
            TransportStatus::Stopped => 0,
            TransportStatus::Playing => 1,
            TransportStatus::Paused => 2,
        }
    }
}

/// Состояние воспроизведения: общее для live-колбэка и offline-рендера.
pub struct PlayState {
    pub bpm: f64,
    pub sample_rate: f32,
    pub cursor_frames: f64,
    pub loop_ticks: f32,
    pub mode: PlayMode,
    pub events: Vec<NoteEvent>,
    next_event: usize,
    voices: Vec<Voice>,
    kick: Vec<f32>,
    status: TransportStatus,
    pub master: f32,
    /// моно-скратч блока (выделяется один раз под максимальный блок).
    scratch: Vec<f32>,
}

impl PlayState {
    pub fn new(sample_rate: f32) -> Self {
        PlayState {
            bpm: 120.0,
            sample_rate,
            cursor_frames: 0.0,
            loop_ticks: 384.0,
            mode: PlayMode::Pat,
            events: Vec::new(),
            next_event: 0,
            voices: Vec::with_capacity(64),
            kick: synth::kick_sample(sample_rate),
            status: TransportStatus::Stopped,
            master: 0.78,
            scratch: vec![0.0; 4096],
        }
    }

    fn frames_per_tick(&self) -> f64 {
        self.sample_rate as f64 * 60.0 / (self.bpm * 96.0)
    }

    pub fn tick_now(&self) -> f32 {
        (self.cursor_frames / self.frames_per_tick()) as f32
    }

    pub fn status(&self) -> TransportStatus {
        self.status
    }

    fn resync_next_event(&mut self) {
        let tick = self.tick_now();
        self.next_event = 0;
        while self.next_event < self.events.len()
            && self.events[self.next_event].tick <= tick
        {
            self.next_event += 1;
        }
    }

    pub fn apply(&mut self, cmd: EngineCommand) {
        match cmd {
            EngineCommand::Play { events, loop_ticks, mode } => {
                self.events = events;
                self.loop_ticks = loop_ticks.max(1.0);
                self.mode = mode;
                self.next_event = 0;
                self.status = TransportStatus::Playing;
            }
            EngineCommand::Pause => {
                if self.status == TransportStatus::Playing {
                    self.status = TransportStatus::Paused;
                    self.voices.clear(); // заглушить звучащие голоса
                }
            }
            EngineCommand::Stop => {
                self.status = TransportStatus::Stopped;
                self.cursor_frames = 0.0;
                self.next_event = 0;
                self.voices.clear();
            }
            EngineCommand::SetMode { events, loop_ticks, mode } => {
                // позиция в тиках сохраняется; события/цикл подменяются
                self.events = events;
                self.loop_ticks = loop_ticks.max(1.0);
                self.mode = mode;
                if self.cursor_frames > self.loop_ticks as f64 * self.frames_per_tick() {
                    self.cursor_frames %= self.loop_ticks as f64 * self.frames_per_tick();
                }
                self.resync_next_event();
            }
            EngineCommand::Seek { tick } => {
                self.cursor_frames = (tick.max(0.0) as f64) * self.frames_per_tick();
                self.resync_next_event();
            }
            EngineCommand::SetBpm { bpm } => {
                if (1.0..=1000.0).contains(&bpm) {
                    // сохраняем позицию в тиках при смене темпа
                    let t = self.tick_now();
                    self.bpm = bpm;
                    self.cursor_frames = t as f64 * self.frames_per_tick();
                }
            }
            EngineCommand::PreviewNote { key, vel, dur_secs, gain, pan, src } => {
                self.spawn(Voice::new(
                    src,
                    key,
                    dur_secs,
                    gain * vel as f32 / 127.0,
                    pan,
                    self.sample_rate,
                ));
            }
            EngineCommand::SetMasterGain { gain } => {
                self.master = gain.clamp(0.0, 1.5);
            }
        }
    }

    fn spawn(&mut self, v: Voice) {
        // прототипный потолок 64 голоса — без роста буфера в колбэке
        if self.voices.len() >= 64 {
            self.voices.remove(0);
        }
        self.voices.push(v);
    }

    fn fire_due(&mut self) {
        let tick = self.tick_now();
        while self.next_event < self.events.len() && self.events[self.next_event].tick <= tick {
            let ev = self.events[self.next_event];
            self.next_event += 1;
            let dur = ev.dur_ticks as f32 / 96.0 * 60.0 / self.bpm as f32;
            self.spawn(Voice::new(
                ev.channel.src,
                ev.key,
                dur,
                ev.gain * ev.vel as f32 / 127.0,
                ev.channel.pan,
                self.sample_rate,
            ));
        }
    }

    /// Продвинуть планировщик на `frames` кадров (с обработкой цикла).
    /// На Stopped/Paused ничего не звучит и курсор не движется.
    pub fn advance(&mut self, frames: usize) {
        if self.status != TransportStatus::Playing {
            return;
        }
        self.fire_due();
        let fpt = self.frames_per_tick();
        let end_tick = (self.cursor_frames + frames as f64) / fpt;
        if end_tick >= self.loop_ticks as f64 {
            self.cursor_frames -= self.loop_ticks as f64 * fpt;
            self.next_event = 0;
            self.fire_due();
        }
        self.cursor_frames += frames as f64;
    }

    /// Смешать блок в стерео-буфер (interleaved), мягкий лимитер.
    pub fn mix_block(&mut self, out: &mut [f32]) {
        let n = out.len() / 2;
        if n == 0 || n > self.scratch.len() {
            return;
        }
        let scratch = &mut self.scratch[..n];
        scratch.fill(0.0);
        let mut i = 0;
        while i < self.voices.len() {
            self.voices[i].mix_block(scratch, &self.kick);
            if self.voices[i].finished() {
                self.voices.swap_remove(i);
            } else {
                i += 1;
            }
        }
        for frame in out.chunks_exact_mut(2).zip(scratch.iter()) {
            let (f, s) = frame;
            // мягкая сатурация x/(1+|x|) — без жёсткого клипа
            let v = s * self.master;
            let c = v / (1.0 + v.abs());
            f[0] = c;
            f[1] = c;
        }
    }

    pub fn is_playing(&self) -> bool {
        self.status == TransportStatus::Playing
    }
}

/// Публичный хэндл движка (владение у UI; Stream живёт, пока жив Engine).
pub struct Engine {
    cmd_tx: rtrb::Producer<EngineCommand>,
    status_bits: Arc<AtomicU32>,
    pos_bits: Arc<AtomicU32>,
    state: Arc<Mutex<PlayState>>,
    stream: Option<cpal::Stream>,
    /// Нет аудиоустройства: команды работают, звук — только offline-рендер.
    pub silent: bool,
}

/// Снапшот транспорта для UI: статус + позиция в тиках — единый источник.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TransportSnapshot {
    pub status: TransportStatus,
    pub tick: f32,
}

impl Engine {
    pub fn new() -> Engine {
        let (tx, mut rx) = rtrb::RingBuffer::<EngineCommand>::new(256);
        let status_bits = Arc::new(AtomicU32::new(0));
        let pos_bits = Arc::new(AtomicU32::new(0));

        // Состояние создаётся ДО стрима: колбэк получает клон Arc.
        let mut state_sr = 44100.0f32;
        let mut stream = None;
        let mut silent = true;
        let state = {
            let host = cpal::default_host();
            match (|| -> Result<cpal::Stream, Box<dyn std::error::Error>> {
                let device = host
                    .default_output_device()
                    .ok_or("нет output-устройства")?;
                let cfg = device.default_output_config()?;
                let sr = cfg.sample_rate().0 as f32;
                state_sr = sr;
                let st = Arc::new(Mutex::new(PlayState::new(sr)));
                let status = status_bits.clone();
                let pos = pos_bits.clone();
                let stc = st.clone();
                let err_cb = |e| eprintln!("[vibe_engine] stream error: {e}");
                let data_cb = move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                    let mut st = match stc.try_lock() {
                        Ok(g) => g,
                        Err(_) => {
                            data.fill(0.0);
                            return;
                        }
                    };
                    while let Ok(cmd) = rx.pop() {
                        st.apply(cmd);
                    }
                    let n = data.len() / 2;
                    st.advance(n);
                    st.mix_block(data);
                    status.store(st.status().to_u8() as u32, Ordering::Relaxed);
                    pos.store(st.tick_now().to_bits(), Ordering::Relaxed);
                };
                if cfg.sample_format() != cpal::SampleFormat::F32 || cfg.channels() < 2 {
                    return Err("устройство не f32/стерео".into());
                }
                let stream =
                    device.build_output_stream(&cfg.into(), data_cb, err_cb, None)?;
                stream.play().ok();
                Ok(stream)
            })() {
                Ok(s) => {
                    silent = false;
                    stream = Some(s);
                    Arc::new(Mutex::new(PlayState::new(state_sr)))
                }
                Err(e) => {
                    eprintln!("[vibe_engine] звук недоступен ({e}); offline-рендер работает");
                    Arc::new(Mutex::new(PlayState::new(state_sr)))
                }
            }
        };

        Engine {
            cmd_tx: tx,
            status_bits,
            pos_bits,
            state,
            stream,
            silent,
        }
    }

    /// Отправить команду в аудио-колбэк (или применить напрямую в silent-режиме).
    fn send(&mut self, cmd: EngineCommand) {
        if self.silent {
            if let Ok(mut st) = self.state.lock() {
                st.apply(cmd);
                self.status_bits
                    .store(st.status().to_u8() as u32, Ordering::Relaxed);
                self.pos_bits.store(st.tick_now().to_bits(), Ordering::Relaxed);
            }
            return;
        }
        let _ = self.cmd_tx.push(cmd);
    }

    /// Снапшот транспорта: ЕДИНЫЙ источник позиции и статуса для всего UI.
    pub fn transport(&self) -> TransportSnapshot {
        TransportSnapshot {
            status: TransportStatus::from_u8(self.status_bits.load(Ordering::Relaxed) as u8),
            tick: f32::from_bits(self.pos_bits.load(Ordering::Relaxed)),
        }
    }

    pub fn play(&mut self, events: Vec<NoteEvent>, loop_ticks: f32, mode: PlayMode) {
        self.send(EngineCommand::Play { events, loop_ticks, mode });
    }

    pub fn pause(&mut self) {
        self.send(EngineCommand::Pause);
    }

    pub fn stop(&mut self) {
        self.send(EngineCommand::Stop);
    }

    pub fn seek(&mut self, tick: f32) {
        self.send(EngineCommand::Seek { tick });
    }

    pub fn set_mode(&mut self, events: Vec<NoteEvent>, loop_ticks: f32, mode: PlayMode) {
        self.send(EngineCommand::SetMode { events, loop_ticks, mode });
    }

    pub fn set_bpm(&mut self, bpm: f64) {
        self.send(EngineCommand::SetBpm { bpm });
    }

    /// Мгновенное превью (шаг/клавиша/нота) — слышно в любом статусе.
    pub fn preview(
        &mut self,
        key: u16,
        vel: u8,
        dur_secs: f32,
        gain: f32,
        pan: f32,
        src: VoiceSrc,
    ) {
        self.send(EngineCommand::PreviewNote { key, vel, dur_secs, gain, pan, src });
    }

    pub fn set_master_gain(&mut self, gain: f32) {
        self.send(EngineCommand::SetMasterGain { gain });
    }
}
