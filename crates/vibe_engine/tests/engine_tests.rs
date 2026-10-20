//! Тесты движка: кик, голоса, offline-рендер, WAV, registry-покрытие.

use vibe_engine::registry::{EngineCommand, NoteEvent, REGISTRY, VoiceSrc};
use vibe_engine::render::render_to_wav;
use vibe_engine::synth;

fn ev(tick: f32, key: u16) -> NoteEvent {
    NoteEvent { tick, key, vel: 100, dur_ticks: 48.0, gain: 0.78, src: VoiceSrc::Kick }
}

#[test]
fn kick_sample_sane() {
    let s = synth::kick_sample(44_100.0);
    assert!(s.len() > 10_000, "длина кика {} < 0.2 c", s.len());
    let peak = s.iter().fold(0.0f32, |m, x| m.max(x.abs()));
    assert!((0.5..=1.0).contains(&peak), "пик кика {peak}");
    // затухает к концу
    let tail: f32 = s[s.len() - 100..].iter().map(|x| x.abs()).sum();
    assert!(tail < 5.0, "хвост не затух: {tail}");
}

#[test]
fn registry_covers_all_commands() {
    let samples: Vec<EngineCommand> = vec![
        EngineCommand::Play { events: vec![], loop_ticks: 384.0 },
        EngineCommand::Stop,
        EngineCommand::Seek { tick: 0.0 },
        EngineCommand::SetBpm { bpm: 120.0 },
        EngineCommand::PreviewNote { key: 60, vel: 100, dur_secs: 0.2, gain: 0.8, src: VoiceSrc::Tone },
        EngineCommand::SetMasterGain { gain: 0.78 },
    ];
    let n_variants = samples.len();
    let names: Vec<&str> = REGISTRY.iter().map(|s| s.name).collect();
    for cmd in samples {
        assert!(names.contains(&cmd.spec_name()), "{} нет в REGISTRY", cmd.spec_name());
    }
    assert_eq!(names.len(), n_variants, "число вариантов и строк REGISTRY расходится");
}

#[test]
fn render_produces_sound_and_correct_length() {
    let events: Vec<NoteEvent> = (0..4).map(|i| ev(i as f32 * 96.0, 63)).collect();
    let tmp = std::env::temp_dir().join("vibe_render_test.wav");
    let frames = render_to_wav(&events, 384.0, 120.0, 1, &tmp).unwrap();
    // 384 тика при 120 BPM = 2 c → 88200 кадров
    assert!((88_000..=88_400).contains(&frames), "кадров {frames}");

    let reader = hound::WavReader::open(&tmp).unwrap();
    assert_eq!(reader.spec().channels, 2);
    assert_eq!(reader.spec().sample_rate, 44_100);
    let peak: f32 = reader
        .into_samples::<i16>()
        .take(20_000)
        .map(|s| (s.unwrap() as f32 / i16::MAX as f32).abs())
        .fold(0.0, f32::max);
    assert!(peak > 0.2, "рендер тихий: peak {peak}");
    std::fs::remove_file(&tmp).ok();
}

#[test]
fn render_silence_for_empty_events() {
    let tmp = std::env::temp_dir().join("vibe_render_empty.wav");
    let frames = render_to_wav(&[], 384.0, 120.0, 1, &tmp).unwrap();
    assert!(frames > 0);
    let reader = hound::WavReader::open(&tmp).unwrap();
    let peak: f32 = reader
        .into_samples::<i16>()
        .map(|s| (s.unwrap() as f32 / i16::MAX as f32).abs())
        .fold(0.0, f32::max);
    assert!(peak < 0.001, "пустой рендер не тихий: {peak}");
    std::fs::remove_file(&tmp).ok();
}
