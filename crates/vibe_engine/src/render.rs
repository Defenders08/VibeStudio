//! Offline-рендер в WAV: тот же Shared/планировщик, что и в колбэке.

use crate::commands::NoteEvent;
use crate::engine::Shared;
use hound::{WavSpec, WavWriter};
use std::io::BufWriter;
use std::path::Path;

const SR: u32 = 44_100;

/// Отрендерить события в 16-бит стерео WAV. `loops` — число проходов цикла.
/// Работает без звуковой карты (та же модель/планировщик, что в колбэке).
pub fn render_to_wav(
    shared: &mut Shared,
    events: &[NoteEvent],
    loop_ticks: f64,
    bpm: f64,
    loops: u32,
    path: &Path,
) -> Result<u64, String> {
    let spec = WavSpec {
        channels: 2,
        sample_rate: SR,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    shared.set_sr(SR as f32);
    shared.model.bpm = bpm;
    shared.events = events.to_vec();
    shared.loop_ticks = loop_ticks;
    shared.epoch_samples = 0;
    shared.tick_base = 0.0;
    shared.next_event_index();
    shared.start_playing();

    let fpt = SR as f64 * 60.0 / (bpm * shared.model.ppq as f64);
    let total_frames = (loop_ticks * fpt).ceil() as u64 * loops.max(1) as u64;

    let file = std::fs::File::create(path).map_err(|e| format!("файл: {e}"))?;
    let mut wav = WavWriter::new(BufWriter::new(file), spec).map_err(|e| format!("wav: {e}"))?;
    let mut written = 0u64;
    while written < total_frames {
        let frames = ((total_frames - written).min(512)) as usize;
        let mut block = vec![0.0f32; frames * 2];
        shared.render_frame(&mut block);
        for frame in block.chunks_exact(2) {
            let l = (frame[0].clamp(-1.0, 1.0) * i16::MAX as f32) as i16;
            let r = (frame[1].clamp(-1.0, 1.0) * i16::MAX as f32) as i16;
            wav.write_sample(l).map_err(|e| format!("wav: {e}"))?;
            wav.write_sample(r).map_err(|e| format!("wav: {e}"))?;
        }
        written += frames as u64;
    }
    shared.status = crate::commands::TransportStatus::Stopped;
    wav.finalize().map_err(|e| format!("wav finalize: {e}"))?;
    Ok(written)
}
