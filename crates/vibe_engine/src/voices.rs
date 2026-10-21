//! Пул голосов фиксированного размера: в аудио-callback ни одной аллокации.
//! Живые громкость/панорама канала читаются из срезов RT-массивов на каждый
//! блок (изменение ручки слышно на звучащих голосах, как в LMMS).
//! Sampler проигрывает сэмпл с питчем по MIDI-ключу (линейная интерполяция).

use crate::model::ChannelKind;
use std::sync::Arc;

pub const POOL_SIZE: usize = 64;
const HALF_PI: f32 = std::f32::consts::FRAC_PI_4;

/// Звучащий голос. `delay` — семплов тишины до старта (sample-accurate).
pub struct Voice {
    pub kind: ChannelKind,
    pub channel_id: u32,
    freq: f32,
    sr: f32,
    delay: usize,
    frames_left: usize,
    frames_total: usize,
    /// Гейн ноты (velocity × gain канала на момент спавна; живой volume
    /// канала умножается поверх на каждом блоке).
    note_gain: f32,
    phase: f32,
    sample: Option<Arc<Vec<f32>>>,
    /// Позиция в сэмпле (float — для интерполяции при pitch-сдвиге).
    sample_pos: f32,
    sample_ratio: f32,
}

pub struct VoicePool {
    slots: Vec<Option<Voice>>,
}

impl VoicePool {
    /// Выделение один раз; в колбэке ни одной аллокации.
    pub fn new() -> Self {
        VoicePool { slots: (0..POOL_SIZE).map(|_| None).collect() }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn spawn(
        &mut self,
        kind: ChannelKind,
        channel_id: u32,
        key: u16,
        dur_secs: f32,
        note_gain: f32,
        delay: usize,
        sr: f32,
        sample: Option<Arc<Vec<f32>>>,
    ) {
        let voice = Voice {
            kind,
            channel_id,
            freq: crate::synth::key_freq(key),
            sr,
            delay,
            frames_left: (dur_secs.max(0.01) * sr) as usize,
            frames_total: (dur_secs.max(0.01) * sr) as usize,
            note_gain,
            phase: 0.0,
            sample_ratio: 2.0f32.powf((key as f32 - 60.0) / 12.0),
            sample,
            sample_pos: 0.0,
        };
        match self.slots.iter_mut().position(|s| s.is_none()) {
            Some(i) => self.slots[i] = Some(voice),
            None => {
                // вытеснение: самый «долгий» голос (правило прототипа, без роста)
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

    /// Смешать все голоса; живые громкость/панорама — из срезов каналов;
    /// пики по каналам аккумулируются в `peaks` (спад делает caller).
    pub fn mix_block(
        &mut self,
        out: &mut [f32],
        kick: &[f32],
        ch_vol: &[f32],
        ch_pan: &[f32],
        peaks: &mut [f32],
    ) {
        for slot in self.slots.iter_mut() {
            let Some(v) = slot.as_mut() else { continue };
            if v.delay > 0 {
                let skip = v.delay.min(out.len() / 2);
                v.delay -= skip;
                if v.delay > 0 {
                    continue;
                }
                let off = skip * 2;
                mix_one(v, &mut out[off..], kick, ch_vol, ch_pan, peaks);
            } else {
                mix_one(v, out, kick, ch_vol, ch_pan, peaks);
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

fn mix_one(
    v: &mut Voice,
    out: &mut [f32],
    kick: &[f32],
    ch_vol: &[f32],
    ch_pan: &[f32],
    peaks: &mut [f32],
) {
    let vol = ch_vol.get(v.channel_id as usize).copied().unwrap_or(0.78);
    let pan = ch_pan.get(v.channel_id as usize).copied().unwrap_or(0.0);
    let angle = (pan + 1.0) * HALF_PI;
    let (gl, gr) = (angle.cos(), angle.sin());
    let gain = v.note_gain * vol;
    let mut peak = 0.0f32;
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
                let val = kick.get(v.sample_pos as usize).copied().unwrap_or(0.0);
                v.sample_pos += 1.0;
                val
            }
            ChannelKind::Synth => {
                v.phase = (v.phase + v.freq / v.sr) % 1.0;
                let saw = 2.0 * v.phase - 1.0;
                let sine = (v.phase * std::f32::consts::TAU).sin();
                0.7 * saw + 0.3 * sine
            }
            ChannelKind::Sampler => match &v.sample {
                Some(s) => {
                    // линейная интерполяция: питч по ключу
                    let i = v.sample_pos as usize;
                    let frac = v.sample_pos - i as f32;
                    let a = s.get(i).copied().unwrap_or(0.0);
                    let b = s.get(i + 1).copied().unwrap_or(a);
                    let val = a + (b - a) * frac;
                    v.sample_pos += v.sample_ratio;
                    val
                }
                None => {
                    let val = kick.get(v.sample_pos as usize).copied().unwrap_or(0.0);
                    v.sample_pos += 1.0;
                    val
                }
            },
        };
        let x = s * env * gain;
        frame[0] += x * gl;
        frame[1] += x * gr;
        peak = peak.max(x.abs());
    }
    let ch_idx = (v.channel_id as usize).min(peaks.len().saturating_sub(1));
    peaks[ch_idx] = peaks[ch_idx].max(peak);
}
