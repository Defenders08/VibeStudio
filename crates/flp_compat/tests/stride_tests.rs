//! Тесты stride-детектора playlist-записей: сигнатура pattern_base == 0x5000,
//! разрешение неоднозначности 80/60 на кратных 240 (docs/FLP_COMPATIBILITY.md §4.2).

use flp_compat::model::detect_stride;

/// Собрать валидную запись заданного stride: u16@4 == 0x5000, остальное — заполним
/// так, чтобы misaligned-кадры на чужой сетке ломали сигнатуру.
fn record(stride: u8, position: u32) -> Vec<u8> {
    let mut v = vec![0u8; stride as usize];
    v[0..4].copy_from_slice(&position.to_le_bytes());
    v[4..6].copy_from_slice(&0x5000u16.to_le_bytes());
    // Осмысленный «шум» в середине записи (младшие байты scale-области у 80B).
    if stride == 80 {
        v[64..72].copy_from_slice(&1.0f64.to_le_bytes()); // 00 00 00 00 00 00 F0 3F
    }
    v
}

#[test]
fn detects_80_byte_records() {
    let mut blob = Vec::new();
    for i in 0..3u32 {
        blob.extend(record(80, i * 384));
    }
    assert_eq!(detect_stride(&blob), Some((80, 3)));
}

#[test]
fn detects_60_byte_records_rejecting_80() {
    // 4 записи по 60 B = 240 B. Кандидат 80 тоже делит 240 — но на сетке 80
    // offsets 4/84/164 попадают в чужие поля с нулями → сигнатура ломается.
    let mut blob = Vec::new();
    for i in 0..4u32 {
        blob.extend(record(60, i * 96));
    }
    assert_eq!(blob.len(), 240);
    assert_eq!(detect_stride(&blob), Some((60, 4)));
}

#[test]
fn ambiguous_multiple_of_240_prefers_80_when_true_stride_is_80() {
    // 3 записи по 80 B = 240 B: 60-сетка ломается о scale-область (u16@64 = 0x0000).
    let mut blob = Vec::new();
    for i in 0..3u32 {
        blob.extend(record(80, i * 64));
    }
    assert_eq!(blob.len(), 240);
    assert_eq!(detect_stride(&blob), Some((80, 3)));
}

#[test]
fn garbage_returns_none() {
    let blob = vec![0xABu8; 100];
    assert_eq!(detect_stride(&blob), None);
    assert_eq!(detect_stride(&[]), None);
}

#[test]
fn partial_signature_corruption_detected() {
    // 2 записи по 80 B, во второй сигнатура испорчена → 80 отпадает;
    // 160 % 60 != 0, % 32 != 0, % 88 != 0 → ни один кандидат не прошёл.
    let mut blob = Vec::new();
    blob.extend(record(80, 0));
    let mut bad = record(80, 384);
    bad[4..6].copy_from_slice(&0x1234u16.to_le_bytes());
    blob.extend(bad);
    assert_eq!(detect_stride(&blob), None);
}
