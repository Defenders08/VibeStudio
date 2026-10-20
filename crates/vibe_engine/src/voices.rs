//! Пул голосов фиксированного размера: в аудио-callback ни одной аллокации.
//! Источник сэмпла Sampler — Arc (клон без аллокаций).

use crate::model::ChannelKind;
use crate::synth;
use std::sync::Arc;

pub const POOL_SIZE: usize = 64;

/// Звучащий голос. `delay` — семплов тишины до старта (sample-accurate).
pub struct Voice {
    pub kind: ChannelKind,
    freq: f32,
    sr: f32,
    delay: usize,
    frames_left: usize,
    frames_total: usize,
    gain: f32,
    pan: f32,
    phase: f32,
    sample: Option<Arc<Vec<f32>>>,
    sample_pos: usize,
}

pub struct VoicePool {
    slots: Vec<Option<Voice>>,
}

impl VoicePool {
    /// Выделение один раз; в колбэке ни одной аллокации.
    pub fn new() -> Self {
        VoicePool { slots: (0..POOL_SIZE).map(|_| None).collect() }
    }

    /// Положить голос в свободный слот. Если пул полон — вытесняем самый
    /// «старый» (первый занятый): правило прототипа, без роста буфера.
    pub fn spawn(
        &mut self,
        kind: ChannelKind,
        key: u16,
        dur_secs: f32,
        gain: f32,
        pan: f32,
        delay: usize,
        sr: f32,
        sample: Option<Arc<Vec<f32>>>,
    ) {
        let voice = Voice {
            kind,
            freq: synth::key_freq(key),
            sr,
            delay,
            frames_left: (dur_secs.max(0.01) * sr) as usize,
            frames_total: (dur_secs.max(0.01) * sr) as usize,
            gain,
            pan: pan.clamp(-1.0, 1.0),
            phase: 0.0,
            sample,
            sample_pos: 0,
        };
        match self.slots.iter_mut().position(|s| s.is_none()) {
            Some(i) => self.slots[i] = Some(voice),
            None => {
                // вытеснение: ищем голос с наибольшим frames_left (дольше звучит)
                let victim = self
                    .slots
                    .iter()
                    .enumerate()
                    .max_by_key(|(_, v)| v.as_ref().map(|v| v.frames_left).unwrap_or(0))
                    .map(|(i, _)| i);
                if let Some(i) = victim {
                    self.slots[i] = Some(voice);
                }
            }
        }
    }

    /// Смешать все голоса в стерео-блок; освободившиеся слоты обнуляются.
    pub fn mix_block(&mut self, out: &mut [f32], kick: &[f32]) {
        for slot in self.slots.iter_mut() {
            let Some(v) = slot.as_mut() else { continue };
            // задержка старта (sample-accurate планирование)
            if v.delay > 0 {
                let skip = v.delay.min(out.len() / 2);
                v.delay -= skip;
                if v.delay > 0 {
                    continue;
                }
                // обрезаем первую часть блока: смешиваем с offset
                let off = skip * 2;
                mix_one(v, &mut out[off..], kick);
            } else {
                mix_one(v, out, kick);
            }
            if v.frames_left == 0 {
                *slot = None;
            }
        }
    }

    /// Заглушить все голоса (Stop/Pause).
    pub fn kill_all(&mut self) {
        for slot in self.slots.iter_mut() {
            *slot = None;
        }
    }
}

fn mix_one(v: &mut Voice, out: &mut [f32], kick: &[f32]) {
    let angle = (v.pan + 1.0) * std::f32::consts::FRAC_PI_4;
    let (gl, gr) = (angle.cos(), angle.sin());
    for frame in out.chunks_exact_mut(2) {
        if v.frames_left == 0 {
            break;
        }
        v.frames_left -= 1;
        let t = 1.0 - v.frames_left as f32 / v.frames_total as f32;
        let attack = ((1.0 - t) * v.frames_total as f32 / v.sr / 0.002).min(1.0);
        let env = attack * (1.0 - t) * (1.0 - t);
        let s = match v.kind {
            ChannelKind::Kick => {
                let val = kick.get(v.sample_pos).copied().unwrap_or(0.0);
                v.sample_pos += 1;
                val
            }
            ChannelKind::Synth => {
                // пила + лёгкая гармоника
                v.phase = (v.phase + v.freq / v.sr) % 1.0;
                let saw = 2.0 * v.phase - 1.0;
                let sine = (v.phase * std::f32::consts::TAU).sin();
                0.7 * saw + 0.3 * sine
            }
            ChannelKind::Sampler => match &v.sample {
                Some(s) => {
                    let val = s.get(v.sample_pos).copied().unwrap_or(0.0);
                    v.sample_pos += 1;
                    val
                }
                None => {
                    // нет сэмпла — фолбэк на кик
                    let val = kick.get(v.sample_pos).copied().unwrap_or(0.0);
                    v.sample_pos += 1;
                    val
                }
            },
        };
        let x = s * env * v.gain;
        frame[0] += x * gl;
        frame[1] += x * gr;
    }
}
