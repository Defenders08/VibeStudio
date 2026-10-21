//! Тесты фазы 1 (модель-в-движке): sample-accurate, Stop, Pause→Play, SONG,
//! BPM на лету, WAV, отсутствие аллокаций в колбэке.

use std::sync::atomic::{AtomicUsize, Ordering};
use vibe_engine::commands::{Command, PlayMode, TransportStatus};
use vibe_engine::engine::Shared;
use vibe_engine::TICKS_PER_BAR;
use vibe_engine::render::render_to_wav;

const SR: f32 = 44_100.0;

fn shared_with_kick_on_steps(steps: &[u32]) -> Shared {
    let mut sh = Shared::new(SR);
    sh.model.bpm = 120.0;
    for &s in steps {
        sh.apply(&Command::SetStep { pattern_id: 1, channel_id: 0, step: s, on: true });
    }
    sh
}

#[test]
fn kick_hits_at_expected_sample_offsets() {
    let mut sh = shared_with_kick_on_steps(&[0, 4, 8, 12]);
    sh.apply(&Command::Play);
    let fpt = SR as f64 * 60.0 / (120.0 * 96.0);
    let expected: Vec<usize> = [0usize, 4, 8, 12]
        .iter()
        .map(|&s| (s as f64 * 24.0 * fpt) as usize)
        .collect();

    let total = (TICKS_PER_BAR as f64 * fpt) as usize + 512;
    let mut energy: Vec<f32> = Vec::new();
    let mut done = 0usize;
    while done < total {
        let n = 512.min(total - done);
        let mut block = vec![0.0f32; n * 2];
        sh.render_frame(&mut block);
        energy.push(block.iter().fold(0.0f32, |m, x| m.max(x.abs())));
        done += n;
    }
    let find_peak = |center: usize| -> usize {
        let c = center / 512;
        let lo = c.saturating_sub(2);
        let hi = (c + 2).min(energy.len() - 1);
        (lo..=hi).max_by_key(|&i| (energy[i] * 1000.0) as usize).unwrap()
    };
    for (i, &exp) in expected.iter().enumerate() {
        let peak_block = find_peak(exp);
        let exp_block = exp / 512;
        assert!(
            peak_block.abs_diff(exp_block) <= 1,
            "удар {}: пик в блоке {}, ожидание {}",
            i, peak_block, exp_block
        );
    }
    let hit = energy[expected[1] / 512];
    let quiet_idx = ((2 * 24 * fpt as usize) / 512).min(energy.len() - 1);
    let quiet = energy[quiet_idx];
    assert!(hit > quiet * 4.0, "удар {hit} должен быть громче паузы {quiet}");
}

#[test]
fn stop_silences_and_resets() {
    let mut sh = shared_with_kick_on_steps(&[0, 2, 4, 6, 8, 10, 12, 14]);
    sh.apply(&Command::Play);
    let mut block = vec![0.0f32; 512 * 2];
    sh.render_frame(&mut block);
    assert!(sh.tick_now() > 0.0);
    sh.apply(&Command::Stop);
    assert_eq!(sh.status, TransportStatus::Stopped);
    assert_eq!(sh.tick_now(), 0.0);
    for _ in 0..4 {
        let mut b = vec![0.0f32; 512 * 2];
        sh.render_frame(&mut b);
        assert!(b.iter().all(|x| x.abs() < 1e-9), "после Stop не тишина");
    }
}

#[test]
fn pause_then_play_resumes_position() {
    let mut sh = shared_with_kick_on_steps(&[0, 4, 8, 12]);
    sh.apply(&Command::Play);
    let mut block = vec![0.0f32; 512 * 2];
    for _ in 0..100 {
        sh.render_frame(&mut block);
    }
    let paused_at = sh.tick_now();
    assert!(paused_at > 100.0);
    sh.apply(&Command::Pause);
    assert_eq!(sh.status, TransportStatus::Paused);
    sh.render_frame(&mut block);
    sh.render_frame(&mut block);
    assert_eq!(sh.tick_now(), paused_at);
    sh.apply(&Command::Play);
    assert!(
        (sh.tick_now() - paused_at).abs() < 1.0,
        "Play после Pause прыгнул: {paused_at} → {}",
        sh.tick_now()
    );
}

#[test]
fn song_clips_play_in_own_ranges() {
    let mut sh = Shared::new(SR);
    sh.model.bpm = 120.0;
    for s in 0..16u32 {
        sh.apply(&Command::SetStep { pattern_id: 1, channel_id: 0, step: s, on: true });
    }
    sh.apply(&Command::SetMode { mode: PlayMode::Song });
    sh.apply(&Command::AddClip { track: 0, bar: 2, len_bars: 1, pattern_id: 1 });
    sh.apply(&Command::AddClip { track: 0, bar: 4, len_bars: 1, pattern_id: 1 });
    sh.apply(&Command::Play);

    // seek на начало клипа (такт 3, 0-based 2) и мерить первый блок
    sh.apply(&Command::Seek { tick: 2.0 * 384.0 });
    let mut block = vec![0.0f32; 512 * 2];
    sh.render_frame(&mut block);
    let in_clip = block.iter().fold(0.0f32, |m, x| m.max(x.abs()));
    assert!(in_clip > 0.005, "клип на такте 3 не звучит ({in_clip})");
}

#[test]
fn bpm_change_does_not_jump_position() {
    let mut sh = shared_with_kick_on_steps(&[0, 4, 8, 12]);
    sh.apply(&Command::Play);
    let mut block = vec![0.0f32; 512 * 2];
    for _ in 0..4 {
        sh.render_frame(&mut block);
    }
    let before = sh.tick_now();
    sh.apply(&Command::SetBpm { bpm: 90.0 });
    let after = sh.tick_now();
    assert!(
        (before - after).abs() < 1.0,
        "скачок позиции при смене BPM: {before} → {after}"
    );
    let t0 = sh.tick_now();
    for _ in 0..4 {
        sh.render_frame(&mut block);
    }
    assert!(sh.tick_now() > t0);
}

#[test]
fn wav_export_roundtrip() {
    let mut sh = shared_with_kick_on_steps(&[0, 4, 8, 12]);
    sh.apply(&Command::Play);
    let path = std::env::temp_dir().join("vibe_phase1.wav");
    let loop_ticks = sh.loop_ticks;
    let frames = vibe_engine::render_to_wav(&mut sh, &[], loop_ticks, 120.0, 1, &path)
        .expect("рендер");
    assert!((176_000..=176_800).contains(&frames), "кадров {frames}");
    let reader = hound::WavReader::open(&path).unwrap();
    assert_eq!(reader.spec().sample_rate, 44_100);
    assert_eq!(reader.spec().channels, 2);
    let duration_secs = reader.duration() as f64 / reader.spec().sample_rate as f64;
    assert!((duration_secs - 4.0).abs() < 0.05, "длительность {duration_secs}");
    std::fs::remove_file(&path).ok();
}

// ---- счётчик аллокаций (TLS-фильтр: тесты параллельны) ----

thread_local! {
    static MEASURE: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}
static COUNT: AtomicUsize = AtomicUsize::new(0);

struct Tracking;

unsafe impl std::alloc::GlobalAlloc for Tracking {
    unsafe fn alloc(&self, layout: std::alloc::Layout) -> *mut u8 {
        if MEASURE.with(|m| m.get()) {
            COUNT.fetch_add(1, Ordering::Relaxed);
        }
        std::alloc::System.alloc(layout)
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: std::alloc::Layout) {
        std::alloc::System.dealloc(ptr, layout)
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: std::alloc::Layout, new_size: usize) -> *mut u8 {
        if MEASURE.with(|m| m.get()) {
            COUNT.fetch_add(1, Ordering::Relaxed);
        }
        std::alloc::System.realloc(ptr, layout, new_size)
    }
}

#[global_allocator]
static TRACKER: Tracking = Tracking;

#[test]
fn callback_frame_allocates_nothing() {
    let mut sh = shared_with_kick_on_steps(&[0, 4, 8, 12]);
    sh.apply(&Command::Play);
    let mut block = vec![0.0f32; 512 * 2];
    sh.render_frame(&mut block);
    sh.render_frame(&mut block);

    MEASURE.with(|m| m.set(true));
    COUNT.store(0, Ordering::Relaxed);
    for _ in 0..8 {
        sh.render_frame(&mut block);
    }
    let allocs = COUNT.load(Ordering::Relaxed);
    MEASURE.with(|m| m.set(false));
    assert_eq!(allocs, 0, "render_frame делает {allocs} аллокаций (должно быть 0)");
}
