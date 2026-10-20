//! Аудиодвижок: cpal-вывод + планировщик событий + голоса.
//!
//! RT-дисциплина прототипа: команды — через rtrb-кольцо (без аллокаций в
//! колбэке); состояние воспроизведения — под коротким Mutex (известный долг:
//! заменить на тройной буфер по мере роста; см. OSS_AUDIT — паттерн maolan).
//! Микширование и планирование общие с offline-рендером («what you hear is
//! what you render», паттерн vibez).

use crate::registry::{EngineCommand, NoteEvent, VoiceSrc};
use crate::synth;
use crate::voice::Voice;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex};

/// Состояние воспроизведения: общее для live-колбэка и offline-рендера.
pub struct PlayState {
    pub bpm: f64,
    pub sample_rate: f32,
    pub cursor_frames: f64,
    pub loop_ticks: f32,
    pub events: Vec<NoteEvent>,
    next_event: usize,
    voices: Vec<Voice>,
    kick: Vec<f32>,
    playing: bool,
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
            events: Vec::new(),
            next_event: 0,
            voices: Vec::with_capacity(64),
            kick: synth::kick_sample(sample_rate),
            playing: false,
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

    pub fn apply(&mut self, cmd: EngineCommand) {
        match cmd {
            EngineCommand::Play { events, loop_ticks } => {
                self.events = events;
                self.loop_ticks = loop_ticks.max(1.0);
                self.cursor_frames = 0.0;
                self.next_event = 0;
                self.playing = true;
            }
            EngineCommand::Stop => {
                self.playing = false;
                self.cursor_frames = 0.0;
                self.next_event = 0;
                self.voices.clear();
            }
            EngineCommand::Seek { tick } => {
                self.cursor_frames = (tick.max(0.0) as f64) * self.frames_per_tick();
                self.next_event = 0;
                let tick = self.tick_now();
                while self.next_event < self.events.len()
                    && self.events[self.next_event].tick <= tick
                {
                    self.next_event += 1;
                }
            }
            EngineCommand::SetBpm { bpm } => {
                if (1.0..=1000.0).contains(&bpm) {
                    // сохраняем позицию в тиках при смене темпа
                    let t = self.tick_now();
                    self.bpm = bpm;
                    self.cursor_frames = t as f64 * self.frames_per_tick();
                }
            }
            EngineCommand::PreviewNote { key, vel, dur_secs, gain, src } => {
                self.spawn(Voice::new(
                    src,
                    key,
                    dur_secs,
                    gain * vel as f32 / 127.0,
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
                ev.src,
                ev.key,
                dur,
                ev.gain * ev.vel as f32 / 127.0,
                self.sample_rate,
            ));
        }
    }

    /// Продвинуть планировщик на `frames` кадров (с обработкой цикла).
    pub fn advance(&mut self, frames: usize) {
        if !self.playing {
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
        self.playing
    }
}

/// Публичный хэндл движка (владение у UI; Stream живёт, пока жив Engine).
pub struct Engine {
    cmd_tx: rtrb::Producer<EngineCommand>,
    playing: Arc<AtomicBool>,
    pos_bits: Arc<AtomicU32>,
    state: Arc<Mutex<PlayState>>,
    stream: Option<cpal::Stream>,
    /// Нет аудиоустройства: команды работают, звук — только offline-рендер.
    pub silent: bool,
}

impl Engine {
    pub fn new() -> Engine {
        let (tx, rx) = rtrb::RingBuffer::<EngineCommand>::new(256);
        let mut rx = rx;
        let playing = Arc::new(AtomicBool::new(false));
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
                let play = playing.clone();
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
                    play.store(st.is_playing(), Ordering::Relaxed);
                    pos.store(st.tick_now().to_bits(), Ordering::Relaxed);
                };
                if cfg.sample_format() != cpal::SampleFormat::F32 || cfg.channels() < 2 {
                    return Err("устройство не f32/стерео".into());
                }
                let stream = device.build_output_stream(
                    &cfg.into(),
                    data_cb,
                    err_cb,
                    None,
                )?;
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

        Engine { cmd_tx: tx, playing, pos_bits, state, stream, silent }
    }

    pub fn play(&mut self, events: Vec<NoteEvent>, loop_ticks: f32) {
        if self.silent {
            if let Ok(mut st) = self.state.lock() {
                st.apply(EngineCommand::Play { events, loop_ticks });
            }
            return;
        }
        let _ = self.cmd_tx.push(EngineCommand::Play { events, loop_ticks });
    }

    pub fn stop(&mut self) {
        if self.silent {
            if let Ok(mut st) = self.state.lock() {
                st.apply(EngineCommand::Stop);
            }
            self.playing.store(false, Ordering::Relaxed);
            return;
        }
        let _ = self.cmd_tx.push(EngineCommand::Stop);
    }

    pub fn seek(&mut self, tick: f32) {
        if !self.silent && self.playing() {
            let _ = self.cmd_tx.push(EngineCommand::Seek { tick });
            return;
        }
        if let Ok(mut st) = self.state.lock() {
            st.apply(EngineCommand::Seek { tick });
            self.pos_bits.store(st.tick_now().to_bits(), Ordering::Relaxed);
        }
    }

    pub fn set_bpm(&mut self, bpm: f64) {
        if self.silent {
            if let Ok(mut st) = self.state.lock() {
                st.apply(EngineCommand::SetBpm { bpm });
            }
            return;
        }
        let _ = self.cmd_tx.push(EngineCommand::SetBpm { bpm });
    }

    /// Мгновенное превью (шаг/клавиша/нота) — слышно и на стопе.
    pub fn preview(&mut self, key: u16, vel: u8, dur_secs: f32, gain: f32, src: VoiceSrc) {
        if self.silent || !self.playing() {
            // На стопе колбэк не микширует — кладём голос напрямую в состояние:
            // он подхватится следующим блоком, когда стрим жив; в silent — оффлайн.
            if let Ok(mut st) = self.state.lock() {
                st.apply(EngineCommand::PreviewNote { key, vel, dur_secs, gain, src });
            }
            return;
        }
        let _ = self.cmd_tx.push(EngineCommand::PreviewNote { key, vel, dur_secs, gain, src });
    }

    pub fn set_master_gain(&mut self, gain: f32) {
        if self.silent {
            if let Ok(mut st) = self.state.lock() {
                st.apply(EngineCommand::SetMasterGain { gain });
            }
            return;
        }
        let _ = self.cmd_tx.push(EngineCommand::SetMasterGain { gain });
    }

    pub fn playing(&self) -> bool {
        self.playing.load(Ordering::Relaxed)
    }

    pub fn position(&self) -> f32 {
        f32::from_bits(self.pos_bits.load(Ordering::Relaxed))
    }
}
