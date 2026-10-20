//! Дамп статистики по .flp-файлу — для сверки глазами и калибровки golden-тестов.
//! Запуск: cargo run -p flp_compat --example dump -- path/to/file.flp

use flp_compat::{parse, ImportReport};
use std::path::PathBuf;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let as_json = args.iter().any(|a| a == "--json");
    let path = args
        .iter()
        .skip(1)
        .find(|a| a.as_str() != "--json")
        .map(PathBuf::from)
        .expect("укажите путь к .flp");
    let data = std::fs::read(&path).expect("файл не читается");
    let project = parse(&data).expect("разбор не удался");
    if as_json {
        // Машинный вид модели — для UI/агентов; структура 1:1 с Rust-моделью.
        println!("{}", serde_json::to_string_pretty(&project).expect("сериализация"));
        return;
    }
    let report = ImportReport::from(&project);

    // Диагностика MixerParams: гистограмма insert_idx (FL внутренние ID).
    let mut hist: std::collections::BTreeMap<u8, usize> = std::collections::BTreeMap::new();
    for rec in &project.mixer_params {
        *hist.entry(rec.insert_idx).or_insert(0) += 1;
    }
    println!("гистограмма 0xE1 по FL insert-ID: {:?}", hist);

    // Первые записи 0xE1 в сыром виде (сверка layout'а).
    let tk = flp_compat::tokenize(&data, &flp_compat::tokenizer::FL25).unwrap();
    for ev in tk.events.iter().filter(|e| e.opcode == 0xE1) {
        println!("0xE1 @{} ({} байт): первые записи:", ev.offset, ev.payload.len());
        for (i, rec) in project.mixer_params.iter().take(8).enumerate() {
            println!(
                "  [{i}] id={} insert={} slot={} msg={}",
                rec.param_id, rec.insert_idx, rec.slot_idx, rec.value
            );
        }
        let raw = &ev.payload;
        for chunk in raw.chunks_exact(12).take(4) {
            println!("  raw: {:02x?}", chunk);
        }
        break; // блоб один на проект
    }

    println!("файл: {}", path.display());
    println!(
        "версия: {:?} | баннер: {:?} | ppq: {} | format: {} | channels_field: {} (legacy, не используется)",
        project.version_string, project.banner, project.header.ppq, project.header.format, project.header.channels_field
    );
    println!(
        "tempo: {:?} | legacy_tempo: {:?} | timesig: {:?} | data_len: {} | trailing: {}",
        report.tempo_bpm, project.legacy_tempo, project.time_signature, project.data_len, report.trailing_bytes
    );
    println!(
        "каналов: {} | паттернов: {} | нот: {} | аранжировок: {} | клипов: {} | insert'ов: {} | слотов с именами: {}",
        report.channels, report.patterns, report.notes, report.arrangements, report.clips, report.inserts, report.slots_with_names
    );
    println!("0xAC payload: {:?}", project.ac_payload);
    println!("stride первого клипа: {:?}", report.stride_used);
    println!("неизвестные opcode'ы: {:?}", report.unknown_opcodes);

    println!("--- каналы ---");
    for ch in &project.channels {
        println!(
            "  iid={} kind={:?} name={:?} color={:?} path={:?} levels={:?} enabled={:?} plugin={:?}/state={:?}/runtime={:?}",
            ch.iid, ch.kind, ch.name, ch.color, ch.sample_path, ch.levels, ch.enabled,
            ch.plugin_internal, ch.plugin_state_len, ch.plugin_runtime_len
        );
    }
    println!("--- паттерны ---");
    for pt in &project.patterns {
        println!(
            "  id={} name={:?} color={:?} len={:?} looped={} notes={} controllers={}",
            pt.id, pt.name, pt.color, pt.length_ticks, pt.looped, pt.notes.len(), pt.controller_records
        );
        for n in pt.notes.iter().take(8) {
            println!(
                "    note pos={} ch={} key={} len={} vel={} pan={} fine={}",
                n.position, n.channel_iid, n.key, n.length_ticks, n.velocity, n.pan, n.fine_pitch
            );
        }
        if pt.notes.len() > 8 {
            println!("    … ещё {} нот", pt.notes.len() - 8);
        }
    }
    println!("--- микшер (mixer_params={}) ---", project.mixer_params.len());
    for ins in &project.inserts {
        println!(
            "  insert={} name={:?} color={:?} routing={:?} flags={:?} слотов(записей)={}",
            ins.index, ins.name, ins.color, ins.routing, ins.flags_raw, ins.slots.len()
        );
        for slot in ins.slots.iter().filter(|s| s.name.is_some() || s.state_len.is_some()) {
            println!(
                "    slot={} name={:?} state_len={:?} runtime_len={:?}",
                slot.index, slot.name, slot.state_len, slot.runtime_len
            );
        }
    }
    println!("--- аранжировки ---");
    for ar in &project.arrangements {
        println!("  id={} name={:?} клипов={} track_events={}", ar.id, ar.name, ar.clips.len(), ar.track_event_count);
        for c in ar.clips.iter().take(16) {
            println!(
                "    clip pos={} len={} item_index={} rvidx={} flags={:#06x} id={} stride={}",
                c.position, c.length, c.item_index, c.track_rvidx, c.item_flags, c.clip_id, c.stride
            );
        }
    }
    if !report.warnings.is_empty() {
        println!("--- предупреждения ({}) ---", report.warnings.len());
        for w in &report.warnings {
            println!("  ! {}", w);
        }
    }
}
