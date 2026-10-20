//! Диагностика транспорта: позиция из transport() (миллитики/1000).
use std::time::{Duration, Instant};
use vibe_engine::{Command, Engine, PlayMode};

fn main() {
    let mut eng = Engine::new();
    println!("silent = {}", eng.silent);
    eng.execute(Command::Play);
    let t0 = Instant::now();
    let mut last = 0.0f64;
    let mut changed = 0;
    while t0.elapsed() < Duration::from_secs(3) {
        let (status, tick) = eng.transport();
        if (tick - last).abs() > 0.5 {
            changed += 1;
        }
        let _ = status;
        last = tick;
        std::thread::sleep(Duration::from_millis(16));
    }
    let (_, tick) = eng.transport();
    println!("итог: tick={:.1} изменений за 3 c: {}", tick, changed);
    if changed > 20 {
        println!("OK: позиция движка течёт");
    } else {
        println!("BAD: позиция движка не течёт");
    }
    eng.execute(Command::Stop);
}
