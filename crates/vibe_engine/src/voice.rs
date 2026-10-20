//! Голос: один звучащий инструмент-инстанс. Рендерится покадрово без аллокаций.
//! Панорама — равная мощность (constant power).

use crate::registry::VoiceSrc;
use crate::synth;

pub struct Voice {
    pub src: VoiceSrc,
    freq: f32,
    sr: f32,
    frames_left: usize,
    frames_total: usize,
    pub gain: f32,
    /// −1..=1; равная мощность: L=cos((p+1)·π/4), R=sin((p+1)·π/4).
    pan: f32,
    phase: f32,
    kick_pos: usize,
}

impl Voice {
    pub fn new(src: VoiceSrc, key: u16, dur_secs: f32, gain: f32, pan: f32, sr: f32) -> Self {
        let total = ((dur_secs.max(0.01)) * sr) as usize;
        Voice {
            src,
            freq: synth::key_freq(key),
            sr,
            frames_left: total,
            frames_total: total,
            gain,
            pan: pan.clamp(-1.0, 1.0),
            phase: 0.0,
            kick_pos: 0,
        }
    }

    pub fn finished(&self) -> bool {
        self.frames_left == 0
    }

    /// Добавить голос в стерео-блок (interleaved). Голос сам не удаляется —
    /// вызывающий отбрасывает по `finished()`.
    pub fn mix_block(&mut self, out: &mut [f32], kick: &[f32]) {
        let angle = (self.pan + 1.0) * (std::f32::consts::FRAC_PI_4);
        let (gl, gr) = (angle.cos(), angle.sin());
        for frame in out.chunks_exact_mut(2) {
            if self.frames_left == 0 {
                break;
            }
            self.frames_left -= 1;
            let t = 1.0 - self.frames_left as f32 / self.frames_total as f32;
            // атака 2 мс + спад (1-t)^2 — без щелчков
            let attack = ((1.0 - t) * self.frames_total as f32 / self.sr / 0.002).min(1.0);
            let env = attack * (1.0 - t) * (1.0 - t);
            let s = match self.src {
                VoiceSrc::Kick => {
                    let v = kick.get(self.kick_pos).copied().unwrap_or(0.0);
                    self.kick_pos += 1;
                    v
                }
                VoiceSrc::Tone => {
                    self.phase = (self.phase + self.freq / self.sr) % 1.0;
                    let fundamental = (self.phase * std::f32::consts::TAU).sin();
                    let harm = (self.phase * 2.0 * std::f32::consts::TAU).sin();
                    0.75 * fundamental + 0.25 * harm
                }
            };
            let v = s * env * self.gain;
            frame[0] += v * gl;
            frame[1] += v * gr;
        }
    }
}
