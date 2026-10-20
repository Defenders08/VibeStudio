//! Демо writer'а: изменить tempo .flp-файла сплайсом с verify-by-readback.
//! Запуск: cargo run -p flp_compat --example set_tempo -- <in.flp> <out.flp> [bpm]

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        eprintln!("использование: set_tempo <in.flp> <out.flp> [bpm=140]");
        std::process::exit(2);
    }
    let bpm: f64 = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(140.0);
    let data = std::fs::read(&args[1]).expect("входной файл не читается");
    match flp_compat::set_tempo(&data, bpm, &flp_compat::tokenizer::FL25) {
        Ok(out) => {
            std::fs::write(&args[2], out).expect("не удалось записать выходной файл");
            println!("ok: tempo → {bpm} BPM ({})", args[2]);
        }
        Err(e) => {
            eprintln!("отказ: {e}");
            std::process::exit(1);
        }
    }
}
