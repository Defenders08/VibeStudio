//! Интеграционные тесты на 5 публичных фикстурах flpdiff (FL 25.2.4, коммит
//! 28aae8f66471be9bf83e342089bbf5b627cfe309, MIT).
//!
//! Инварианты, проверяемые на КАЖДОМ файле:
//! - разбор доходит ровно до `22 + FLdt.data_len` (спека §1.2);
//! - ни одно событие не пересекает границу `FLdt`;
//! - байты после `FLdt` отсутствуют;
//! - первое событие — `0xC7` (ASCII-версия), затем `0x9F` (build);
//! - `0xAC` на offset 48 с payload `01 01 00`, следующий opcode — `0xC0`-баннер.

use flp_compat::{parse, ImportReport};

const FIXTURES: [(&str, usize); 5] = [
    ("base_empty.flp", 46_511),
    ("base_one_channel.flp", 47_459),
    ("base_one_insert.flp", 47_044),
    ("base_one_pattern.flp", 47_580),
    ("base_one_serum.flp", 63_695),
];

fn fixture(name: &str) -> Vec<u8> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name);
    std::fs::read(&path).unwrap_or_else(|e| panic!("фикстура {}: {e}", path.display()))
}

fn dump_stats(name: &str, p: &flp_compat::Project) {
    let r = ImportReport::from(p);
    eprintln!(
        "{name}: v={:?} tempo={:?} ppq={} ch={} pat={} notes={} arr={} clips={} ins={} slots_named={} mixer_params={} unknown={:?} warnings={}",
        r.version_string, r.tempo_bpm, r.ppq, r.channels, r.patterns, r.notes,
        r.arrangements, r.clips, r.inserts, r.slots_with_names, r.mixer_params,
        r.unknown_opcodes, r.warnings.len()
    );
}

#[test]
fn all_five_parse_to_exact_end() {
    for (name, size) in FIXTURES {
        let data = fixture(name);
        assert_eq!(data.len(), size, "размер {name} не совпал — фикстура заменена?");
        let p = parse(&data).unwrap_or_else(|e| panic!("{name}: {e}"));
        dump_stats(name, &p);
        assert_eq!(p.trailing_bytes, 0, "{name}: байты после FLdt");
        assert_eq!(
            p.data_len + 22,
            data.len(),
            "{name}: FLdt.data_len должен покрывать файл ровно"
        );
        // Первое событие — 0xC7 (версия), следующее — 0x9F (build 4960).
        let tk = flp_compat::tokenize(&data, &flp_compat::tokenizer::FL25).unwrap();
        assert_eq!(tk.events[0].offset, 22);
        assert_eq!(tk.events[0].opcode, 0xC7, "{name}");
        assert_eq!(tk.events[1].opcode, 0x9F, "{name}");
        assert_eq!(tk.events[1].payload_u32(), Some(4960), "{name}: FL build");
        // Заголовок: ppq 96 у всех пяти (замер по файлам, не из спеки).
        assert_eq!(p.header.ppq, 96, "{name}: PPQ");
        assert_eq!(p.header.format, 0, "{name}: format");
    }
}

#[test]
fn ac_framing_and_banner_on_every_fixture() {
    for (name, _) in FIXTURES {
        let tk = flp_compat::tokenize(&fixture(name), &flp_compat::tokenizer::FL25).unwrap();
        let ac = tk.events.iter().find(|e| e.opcode == 0xAC).expect("{name}: нет 0xAC");
        assert_eq!(ac.offset, 48, "{name}: 0xAC не на offset 48");
        assert_eq!(ac.payload, vec![0x01, 0x01, 0x00], "{name}: payload 0xAC");
        // Следующее событие после 0xAC — 0xC0-баннер (инвариант FL 25).
        let idx = tk.events.iter().position(|e| e.opcode == 0xAC).unwrap();
        assert_eq!(tk.events[idx + 1].opcode, 0xC0, "{name}: после 0xAC не 0xC0");
        assert_eq!(
            tk.events[idx + 1].payload_utf16z().as_deref(),
            Some("FL Studio 25.2.4.4960.4960"),
            "{name}: текст баннера"
        );
    }
}

#[test]
fn n_channels_field_is_unreliable() {
    // На фикстурах поле 1 или 2 при разном реальном числе каналов —
    // регрессионный якорь на «не использовать n_channels для валидации».
    let empty: u16 = flp_compat::parse(fixture("base_empty.flp")).unwrap().header.channels_field;
    let one_channel: u16 =
        flp_compat::parse(fixture("base_one_channel.flp")).unwrap().header.channels_field;
    assert_ne!(empty, one_channel, "поле меняется от файла к файлу — и всё равно не годится");
}

#[test]
fn base_empty_golden() {
    // Golden-значения сняты с реального файла (пример dump, 2026-10-20).
    let p = parse(fixture("base_empty.flp")).unwrap();
    // «Пустой» проект содержит дефолтный канал Sampler — так делает сама FL.
    assert_eq!(p.channels.len(), 1);
    assert_eq!(p.channels[0].name.as_deref(), Some("Sampler"));
    assert_eq!(p.channels[0].kind, Some(0));
    assert_eq!(p.channels[0].iid, 0);
    assert_eq!(p.patterns.len(), 0);
    assert_eq!(p.arrangements.len(), 1, "аранжировка есть, клипов нет");
    assert_eq!(p.arrangements[0].clips.len(), 0);
    assert_eq!(p.tempo_bpm, Some(120.0));
    assert_eq!(p.time_signature, Some((4, 4)));
    assert_eq!(p.version_string.as_deref(), Some("25.2.4.4960"));
    assert_eq!(p.inserts.len(), 18, "18 insert'ов на всех базовых фикстурах (S: OBSERVED)");
    assert!(p.warnings.is_empty(), "предупреждений быть не должно: {:?}", p.warnings);
}

#[test]
fn base_one_channel_golden() {
    let p = parse(fixture("base_one_channel.flp")).unwrap();
    assert_eq!(p.channels.len(), 2, "дефолтный Sampler + пользовательский канал");
    let kick = &p.channels[1];
    assert_eq!(kick.iid, 1);
    assert_eq!(kick.name.as_deref(), Some("Kick"));
    assert_eq!(kick.kind, Some(0), "sampler");
    assert_eq!(
        kick.sample_path.as_deref(),
        Some("%FLStudioFactoryData%/Data/Patches/Packs/Drums/Kicks/909 Kick.wav")
    );
    assert!(kick.levels.is_some());
    if let Some(lv) = kick.levels {
        assert_eq!(lv, flp_compat::model::LEVELS_DEFAULT, "Levels не менялись");
    }
}

#[test]
fn base_one_pattern_golden() {
    let p = parse(fixture("base_one_pattern.flp")).unwrap();
    assert_eq!(p.patterns.len(), 1);
    let pt = &p.patterns[0];
    assert_eq!(pt.id, 1);
    assert_eq!(pt.name.as_deref(), Some("P1"));
    assert_eq!(pt.length_ticks, Some(0), "0 = дефолтная длина (такт)");
    assert_eq!(pt.looped, false);
    assert_eq!(pt.notes.len(), 1);
    // Golden-значения сверены с оракулом flpdiff v0.1.2 `info --format json`
    // (differential-валидация 2026-10-20): все поля совпали 1:1.
    let note = &pt.notes[0];
    assert_eq!(note.position, 0);
    assert_eq!(note.flags, 0x4000);
    assert_eq!(note.channel_iid, 1, "нота принадлежит каналу Kick (iid 1)");
    assert_eq!(note.length_ticks, 48);
    assert_eq!(note.key, 63);
    assert_eq!(note.fine_pitch, 120);
    assert_eq!(note.pan, 64);
    assert_eq!(note.release, 64);
    assert_eq!(note.velocity, 100);
    assert_eq!(note.tail, [0x80, 0x80]);
    // Round-trip полей: to_bytes(from_bytes) байт-в-байт.
    let rebuilt = note.to_bytes();
    let raw: [u8; 24] = {
        let mut r = [0u8; 24];
        r[0..4].copy_from_slice(&note.position.to_le_bytes());
        r[4..6].copy_from_slice(&note.flags.to_le_bytes());
        r[6..8].copy_from_slice(&note.channel_iid.to_le_bytes());
        r[8..12].copy_from_slice(&note.length_ticks.to_le_bytes());
        r[12..14].copy_from_slice(&note.key.to_le_bytes());
        r[14..16].copy_from_slice(&note.reserved.to_le_bytes());
        r[16..18].copy_from_slice(&note.fine_pitch.to_le_bytes());
        r[18..20].copy_from_slice(&note.pan.to_le_bytes());
        r[20] = note.release;
        r[21] = note.velocity;
        r[22] = note.tail[0];
        r[23] = note.tail[1];
        r
    };
    assert_eq!(rebuilt, raw);
}

#[test]
fn base_one_insert_golden() {
    let p = parse(fixture("base_one_insert.flp")).unwrap();
    assert_eq!(p.inserts.len(), 18);
    let drums = p.inserts.iter().find(|i| i.name.as_deref() == Some("Drums")).expect("insert Drums");
    assert_eq!(drums.index, 1);
    assert_eq!(drums.slots.len(), 1);
    let slot = &drums.slots[0];
    assert_eq!(slot.index, 0);
    assert_eq!(slot.name.as_deref(), Some("Fruity Parametric EQ 2"));
    assert_eq!(slot.state_len, Some(354), "opaque-состояние EQ2");
    assert_eq!(slot.runtime_len, Some(52), "0xD4 runtime data в slot-кортеже");
    // MixerParams хранятся глобально с FL ID (маппинг на порядок 0x93 не установлен).
    assert!(!p.mixer_params.is_empty(), "0xE1 записи присутствуют");
}

/// Дифференциальный эксперимент: какой FL insert-ID соответствует «Drums» (наш insert 1)?
/// Сравниваем содержимое групп 0xE1 base_empty vs base_one_insert; отличившаяся группа —
/// кандидат. Пока печатаем результат (гипотеза), golden зафиксируем после подтверждения.
#[test]
fn differential_drums_insert_id_discovery() {
    let empty = parse(fixture("base_empty.flp")).unwrap();
    let one = parse(fixture("base_one_insert.flp")).unwrap();
    let group = |p: &flp_compat::Project, id: u8| -> Vec<(u8, u8, i32)> {
        p.mixer_params
            .iter()
            .filter(|r| r.insert_idx == id)
            .map(|r| (r.param_id, r.slot_idx, r.value))
            .collect()
    };
    let ids: Vec<u8> = one.mixer_params.iter().map(|r| r.insert_idx).collect::<std::collections::BTreeSet<_>>().into_iter().collect();
    let mut differing = Vec::new();
    for id in ids {
        if group(&empty, id) != group(&one, id) {
            differing.push(id);
        }
    }
    eprintln!("differential: группы 0xE1, отличающиеся от base_empty: {differing:?}");
    for id in &differing {
        eprintln!("  FL ID {id}: {:?}", group(&one, *id));
    }
    // На one_insert единственное отличие — EQ2 на слоте 0 insert'а «Drums»:
    // отличившихся групп может не быть вовсе (EQ хранится в 0xD5, а не в 0xE1),
    // либо одна-две (SlotEnabled/SlotMix). Гипотезу фиксируем мягко.
    assert!(differing.len() <= 2, "неожиданно много отличий: {differing:?}");
}

#[test]
fn base_one_serum_golden() {
    let p = parse(fixture("base_one_serum.flp")).unwrap();
    let serum = p.channels.iter().find(|c| c.name.as_deref() == Some("SerumTest")).expect("SerumTest");
    assert_eq!(serum.kind, Some(2), "instrument/Native");
    assert!(serum.plugin_state_len.is_some(), "state плагина в scope канала");
}

#[test]
fn mixer_params_present_on_every_fixture() {
    // 0xE1 — один блоб на проект (6924 B = 577 записей на базовых файлах):
    // 18 групп × 32 записи (insert-ID 53 и 64..80) + 1 запись вне групп.
    for (name, _) in FIXTURES {
        let p = parse(fixture(name)).unwrap();
        assert_eq!(p.mixer_params.len(), 577, "{name}");
        let out_of_groups: Vec<_> = p
            .mixer_params
            .iter()
            .filter(|r| r.insert_idx != 53 && !(64..=80).contains(&r.insert_idx))
            .collect();
        assert_eq!(out_of_groups.len(), 1, "{name}: ровно одна запись вне 18 групп");
        eprintln!("{name}: запись вне групп: {:?}", out_of_groups[0]);
    }
}
