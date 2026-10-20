//! Диагностика транспорта: создаёт Engine, запускает Play и печатает позицию
//! из transport() — единый источник UI. Запуск:
//! cargo run -p vibe_engine --example transport_probe

use std::time::{Duration, Instant};
use vibe_engine::{Engine, NoteEvent, PlayMode, TransportStatus, VoiceSrc, ChannelVoice};

fn main() {
    let mut eng = Engine::new();
    println!("silent = {}", eng.silent);

    let events: Vec<NoteEvent> = (0..8)
        .map(|i| NoteEvent {
            tick: i as f32 * 24.0,
            key: 60,
            vel: 100,
            dur_ticks: 24.0,
            gain: 0.7,
            channel: ChannelVoice { src: VoiceSrc::Kick, pan: 0.0 },
        })
        .collect();
    eng.play(events, 384.0, PlayMode::Pat);

    let t0 = Instant::now();
    let mut last = 0.0f32;
    let mut changed = 0;
    while t0.elapsed() < Duration::from_secs(3) {
        let snap = eng.transport();
        if (snap.tick - last).abs() > 0.5 {
            changed += 1;
        }
        last = snap.tick;
        std::thread::sleep(Duration::from_millis(16));
    }
    let snap = eng.transport();
    println!(
        "итог: статус={:?} tick={:.1} изменений за 3 c: {}",
        snap.status, snap.tick, changed
    );
    if changed > 20 {
        println!("OK: позиция движка течёт — замирание на стороне UI");
    } else {
        println!("BAD: позиция движка не течёт — баг в публикации pos_bits");
    }
    eng.stop();
}
