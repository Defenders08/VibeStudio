//! Юнит-тесты токенайзера на синтетических потоках: диапазонные правила,
//! varint, override `0xAC`, ошибки границ. Никаких внешних файлов.

use flp_compat::tokenizer::{tokenize, Override, Profile, EVENTS_START};
use flp_compat::FlpError;

/// Сборка минимального FLP: FLhd + FLdt + события.
fn flp(events: &[u8]) -> Vec<u8> {
    let mut v = Vec::new();
    v.extend_from_slice(b"FLhd");
    v.extend_from_slice(&6u32.to_le_bytes());
    v.extend_from_slice(&0u16.to_le_bytes()); // format
    v.extend_from_slice(&2u16.to_le_bytes()); // channels_field
    v.extend_from_slice(&96u16.to_le_bytes()); // ppq
    v.extend_from_slice(b"FLdt");
    v.extend_from_slice(&(events.len() as u32).to_le_bytes());
    v.extend_from_slice(events);
    v
}

#[test]
fn fixed_payload_sizes_by_range() {
    // 0x00 → 1 байт, 0x40 → 2, 0x9C → 4, 0xC0 → varint+data.
    let events = [
        0x00, 0x01, // BYTE
        0x40, 0x07, 0x00, // WORD iid 7
        0x9C, 0xD0, 0xFB, 0x01, 0x00, // DWORD 0x01FBD0 = 130000 = 130.0 BPM
        0xC1, 0x03, b'P', b'1', 0x00, // DATA: len 3
    ];
    let t = tokenize(&flp(&events), &Profile { name: "test", overrides: &[] }).unwrap();
    assert_eq!(t.header.ppq, 96);
    assert_eq!(t.events.len(), 4);
    assert_eq!(t.events[0].opcode, 0x00);
    assert_eq!(t.events[1].payload_u16(), Some(7));
    assert_eq!(t.events[2].payload_u32(), Some(130_000));
    assert_eq!(t.events[3].opcode, 0xC1);
    assert_eq!(t.events[3].payload, b"P1\0".to_vec());
    // Курсор кончился ровно на конце.
    assert_eq!(t.data_len, events.len());
}

#[test]
fn varint_multibyte_length() {
    // Длина 300: LEB128 = [0xAC, 0x02]. Payload из 300 нулей.
    let mut events = vec![0xF0u8, 0xAC, 0x02];
    events.extend(std::iter::repeat_n(0u8, 300));
    let t = tokenize(&flp(&events), &Profile { name: "test", overrides: &[] }).unwrap();
    assert_eq!(t.events.len(), 1);
    assert_eq!(t.events[0].opcode, 0xF0);
    assert_eq!(t.events[0].payload.len(), 300);
}

#[test]
fn ac_override_raw3() {
    // 0xAC на fl25-профиле: ровно 3 байта payload, следующий opcode не съедается.
    let events = [0xAC, 0x01, 0x01, 0x00, 0xC0, 0x02, 0x46, 0x00];
    let t = tokenize(&flp(&events), &flp_compat::tokenizer::FL25).unwrap();
    let ac = t.events.iter().find(|e| e.opcode == 0xAC).unwrap();
    assert_eq!(ac.payload, vec![0x01, 0x01, 0x00]);
    let banner = t.events.iter().find(|e| e.opcode == 0xC0).unwrap();
    assert_eq!(banner.payload, vec![0x46, 0x00]);
}

#[test]
fn ac_without_override_eats_next_opcode() {
    // Без override классическое правило DWORD читает 4 байта — ровно баг из спеки:
    // 4-байтовое чтение съедает opcode 0xC0, дальше поток уходит в мусор/ошибку.
    let events = [0xAC, 0x01, 0x01, 0x00, 0xC0, 0x02, 0x46, 0x00];
    let result = tokenize(&flp(&events), &Profile { name: "classic", overrides: &[] });
    let no_banner = match &result {
        Ok(t) => !t.events.iter().any(|e| e.opcode == 0xC0),
        Err(_) => true, // ошибка кадрирования — тоже приемлемый исход
    };
    assert!(no_banner, "0xC0 не должен был уцелеть при 4-байтовом чтении 0xAC");
}

#[test]
fn overrun_on_payload_crossing_fldt_end() {
    // 0x9C требует 4 байта, данных только 2 → Overrun с точными offset/need/end.
    let events = [0x9C, 0x01, 0x02];
    let err = tokenize(&flp(&events), &Profile { name: "test", overrides: &[] }).unwrap_err();
    match err {
        FlpError::Overrun { opcode, offset, need, end } => {
            assert_eq!(opcode, 0x9C);
            assert_eq!(offset, EVENTS_START);
            assert_eq!(need, 4);
            assert_eq!(end, EVENTS_START + 3);
        }
        other => panic!("ожидался Overrun, получено {other:?}"),
    }
}

#[test]
fn truncated_data_len_detected() {
    // data_len указывает дальше конца файла → TruncatedData, а не паника.
    let mut v = flp(&[]);
    let extra: [u8; 4] = 999u32.to_le_bytes();
    v[18..22].copy_from_slice(&extra); // поле data_len в заголовке FLdt (offset 18)
    assert!(matches!(
        tokenize(&v, &Profile { name: "test", overrides: &[] }),
        Err(FlpError::TruncatedData { .. })
    ));
}

#[test]
fn not_flp_detected() {
    // Вход длиннее заголовка, чтобы сработала именно проверка сигнатуры.
    let mut bad = Vec::new();
    bad.extend_from_slice(b"\x00\x01\x02\x03garbage-padding!");
    let err = tokenize(&bad, &Profile { name: "t", overrides: &[] }).unwrap_err();
    assert!(matches!(err, FlpError::NotFlp { .. }));
}

#[test]
fn trailing_bytes_after_fldt_recorded() {
    let mut v = flp(&[0x00, 0x01]);
    v.extend_from_slice(b"TAIL");
    let t = tokenize(&v, &Profile { name: "test", overrides: &[] }).unwrap();
    assert_eq!(t.trailing_bytes, 4);
}

#[test]
fn utf16_and_ascii_helpers() {
    // UTF-16LE "Kick\0": K=0x004B...
    let mut payload = vec![0x4B, 0x00, 0x69, 0x00, 0x63, 0x00, 0x6B, 0x00, 0x00, 0x00];
    payload.insert(0, 5); // varint-длина 5 юнитов = 10 байт? нет: длина в БАЙТАХ
    // Правильно: длина 10 (байт), потом 10 байт payload.
    let mut events = vec![0xC5, 10];
    events.extend_from_slice(&[0x4B, 0x00, 0x69, 0x00, 0x63, 0x00, 0x6B, 0x00, 0x00, 0x00]);
    let _ = payload; // (payload выше не используется, оставлен для читаемости истории)
    let t = tokenize(&flp(&events), &Profile { name: "test", overrides: &[] }).unwrap();
    let ev = &t.events[0];
    assert_eq!(ev.payload_utf16z().as_deref(), Some("Kick"));
}

#[test]
fn varint_override_profile_mechanics() {
    // Override::Raw применяется вместо диапазонного правила: 0x9C читается
    // как 2 байта, дальше поток продолжается корректно.
    let profile = Profile { name: "x", overrides: &[(0x9C, Override::Raw(2))] };
    let events = [0x9C, 0x40, 0x1F, 0x00, 0x00];
    let t = tokenize(&flp(&events), &profile).unwrap();
    assert_eq!(t.events[0].payload, vec![0x40, 0x1F]);
    assert_eq!(t.events[1].opcode, 0x00);
}
