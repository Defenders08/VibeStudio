//! Тесты writer'а (L8-прототип): сплайс `0x9C` + verify-by-readback,
//! round-trip на синтетике и на всех 5 фикстурах.

use flp_compat::{parse, set_tempo, tokenizer};
use flp_compat::FlpError;

/// Сборка минимального FLP (как в tokenizer_tests).
fn flp(events: &[u8]) -> Vec<u8> {
    let mut v = Vec::new();
    v.extend_from_slice(b"FLhd");
    v.extend_from_slice(&6u32.to_le_bytes());
    v.extend_from_slice(&0u16.to_le_bytes());
    v.extend_from_slice(&2u16.to_le_bytes());
    v.extend_from_slice(&96u16.to_le_bytes());
    v.extend_from_slice(b"FLdt");
    v.extend_from_slice(&(events.len() as u32).to_le_bytes());
    v.extend_from_slice(events);
    v
}

#[test]
fn set_tempo_on_synthetic() {
    let events = [
        0x9C, 0xA0, 0xFB, 0x01, 0x00, // 129952 … неважно, перезапишем
        0xC7, 0x02, b'9', b'9', // версия "99"
        0x00, 0x01, // BYTE-событие
    ];
    let original = flp(&events);
    let modified = set_tempo(&original, 140.0, &tokenizer::FL25).unwrap();

    // Readback: модель видит новый tempo, остальное без изменений.
    let p = parse(&modified).unwrap();
    assert_eq!(p.tempo_bpm, Some(140.0));
    assert_eq!(p.version_string.as_deref(), Some("99"));

    // Изменились байты payload 0x9C (offset 23..27): 129952 → 140000 отличаются
    // тремя байтами (старший у обоих 0x00), четвёртый байт совпадает.
    let diff: Vec<usize> = (0..original.len()).filter(|&i| original[i] != modified[i]).collect();
    assert_eq!(diff, vec![23, 24, 25], "патч должен лечь в payload 0x9C (offset 23..27)");

    // Обратная операция восстанавливает исходные байты.
    let restored = set_tempo(&modified, 129.952, &tokenizer::FL25).unwrap();
    assert_eq!(restored, original);
}

#[test]
fn set_tempo_without_tempo_event_refuses() {
    let original = flp(&[0x00, 0x01]);
    match set_tempo(&original, 140.0, &tokenizer::FL25) {
        Err(FlpError::TempoNotFound) => {}
        other => panic!("ожидался TempoNotFound, получено {other:?}"),
    }
}

#[test]
fn set_tempo_range_check() {
    let original = flp(&[0x9C, 0xA0, 0xFB, 0x01, 0x00]);
    assert!(matches!(
        set_tempo(&original, 0.5, &tokenizer::FL25),
        Err(FlpError::BpmOutOfRange(_))
    ));
    assert!(matches!(
        set_tempo(&original, 100000.0, &tokenizer::FL25),
        Err(FlpError::BpmOutOfRange(_))
    ));
}

#[test]
fn fixtures_set_tempo_round_trip() {
    const FIXTURES: [&str; 5] = [
        "base_empty.flp",
        "base_one_channel.flp",
        "base_one_insert.flp",
        "base_one_pattern.flp",
        "base_one_serum.flp",
    ];
    for name in FIXTURES {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join(name);
        let original = std::fs::read(&path).unwrap();

        let modified = set_tempo(&original, 140.0, &tokenizer::FL25)
            .unwrap_or_else(|e| panic!("{name}: {e}"));
        let p = parse(&modified).unwrap_or_else(|e| panic!("{name}: {e}"));
        assert_eq!(p.tempo_bpm, Some(140.0), "{name}");

        // Остальная модель не изменилась: те же каналы/паттерны/клипы.
        let before = parse(&original).unwrap();
        assert_eq!(before.channels.len(), p.channels.len(), "{name}");
        assert_eq!(before.patterns.len(), p.patterns.len(), "{name}");
        assert_eq!(before.inserts.len(), p.inserts.len(), "{name}");
        assert_eq!(before.banner, p.banner, "{name}");

        // Обратный сплайс возвращает байт-в-байт исходный файл.
        let restored = set_tempo(&modified, 120.0, &tokenizer::FL25).unwrap();
        assert_eq!(restored, original, "{name}: round-trip не байт-в-байт");
    }
}
