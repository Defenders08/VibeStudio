//! Offline-рендер в WAV: тот же PlayState/голоса, что и live-колбэк
//! («what you hear is what you render»).

use crate::engine::PlayState;
use crate::registry::NoteEvent;
use hound::{WavSpec, WavWriter};
use std::io::BufWriter;
use std::path::Path;

/// Отрендерить события в 16-бит стерео WAV (44100).
/// `loops` — сколько раз прокрутить цикл (PAT: 2, SONG: 1).
pub fn render_to_wav(
    events: &[NoteEvent],
    loop_ticks: f32,
    bpm: f64,
    loops: u32,
    path: &Path,
) -> Result<u64, String> {
    const SR: u32 = 44_100;
    let spec = WavSpec {
        channels: 2,
        sample_rate: SR,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut st = PlayState::new(SR as f32);
    st.apply(crate::registry::EngineCommand::SetMasterGain { gain: 0.78 });
    st.apply(crate::registry::EngineCommand::Play {
        events: events.to_vec(),
        loop_ticks,
    });
    // устанавливаем bpm после Play (Play не сбрасывает bpm, но для ясности)
    st.apply(crate::registry::EngineCommand::SetBpm { bpm });

    let fpt = SR as f64 * 60.0 / (bpm * 96.0);
    let total_frames = (loop_ticks as f64 * fpt).ceil() as u64 * loops.max(1) as u64;

    let file = std::fs::File::create(path).map_err(|e| format!("файл: {e}"))?;
    let mut wav = WavWriter::new(BufWriter::new(file), spec).map_err(|e| format!("wav: {e}"))?;
    let mut block = [0.0f32; 1024];
    let mut written = 0u64;
    while written < total_frames {
        let n = ((total_frames - written).min(512) * 2) as usize;
        let out = &mut block[..n];
        out.fill(0.0);
        st.advance(n / 2);
        st.mix_block(out);
        for frame in out.chunks_exact(2) {
            let l = (frame[0].clamp(-1.0, 1.0) * i16::MAX as f32) as i16;
            let r = (frame[1].clamp(-1.0, 1.0) * i16::MAX as f32) as i16;
            wav.write_sample(l).map_err(|e| format!("wav: {e}"))?;
            wav.write_sample(r).map_err(|e| format!("wav: {e}"))?;
        }
        written += (n / 2) as u64;
    }
    wav.finalize().map_err(|e| format!("wav finalize: {e}"))?;
    Ok(written)
}
