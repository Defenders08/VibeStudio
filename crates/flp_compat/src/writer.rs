//! Writer уровня L8 (прототип): сплайс-операции фиксированной длины поверх
//! исходных байтов + обязательный verify-by-readback (методология flpkit, MIT).
//!
//! Принципы (docs/FLP_COMPATIBILITY.md §6.3):
//! 1. Никогда не ресериализуем поток — патчим байты в копии файла;
//!    неизвестные/нетронутые байты выживают by construction.
//! 2. Операция не считается успешной, пока перечитанный файл не декодируется
//!    в ожидаемый результат и все прочие события не остались байт-в-байт равными.
//! 3. Сомнение → отказ (`FlpError`), а не «рискованная запись».

use crate::error::FlpError;
use crate::tokenizer::{self, Profile};

/// Изменить tempo проекта: патч payload первого `0x9C` (BPM×1000, u32 LE) на месте.
/// Длина события не меняется, поэтому `FLdt.data_len` остаётся прежним.
pub fn set_tempo(data: &[u8], bpm: f64, profile: &Profile) -> Result<Vec<u8>, FlpError> {
    if !(1.0..=65535.999).contains(&bpm) {
        return Err(FlpError::BpmOutOfRange(bpm));
    }
    let tk = tokenizer::tokenize(data, profile)?;
    let ev = tk
        .events
        .iter()
        .find(|e| e.opcode == 0x9C)
        .ok_or(FlpError::TempoNotFound)?;
    let value = (bpm * 1000.0).round() as u32;

    let start = ev.offset + 1;
    let mut out = data.to_vec();
    out[start..start + 4].copy_from_slice(&value.to_le_bytes());
    verify_tempo(data, &out, profile, ev.offset, value)?;
    Ok(out)
}

/// Verify-by-readback: перечитать оба файла, сверить число событий, байты
/// нетронутых событий и новое значение tempo.
fn verify_tempo(
    original: &[u8],
    modified: &[u8],
    profile: &Profile,
    patched_offset: usize,
    expected: u32,
) -> Result<(), FlpError> {
    let a = tokenizer::tokenize(original, profile)?;
    let b = tokenizer::tokenize(modified, profile)?;
    if a.events.len() != b.events.len() {
        return Err(FlpError::VerifyFailed(format!(
            "число событий изменилось: {} → {}",
            a.events.len(),
            b.events.len()
        )));
    }
    if a.data_len != b.data_len || a.trailing_bytes != b.trailing_bytes {
        return Err(FlpError::VerifyFailed("габариты FLdt изменились".into()));
    }
    for (ea, eb) in a.events.iter().zip(b.events.iter()) {
        if ea.opcode == 0x9C && ea.offset == patched_offset {
            if eb.payload_u32() != Some(expected) {
                return Err(FlpError::VerifyFailed(format!(
                    "tempo после записи: {:?}, ожидалось {expected}",
                    eb.payload_u32()
                )));
            }
        } else if ea != eb {
            return Err(FlpError::VerifyFailed(format!(
                "нетронутое событие 0x{:02X} на offset {} изменилось",
                ea.opcode, ea.offset
            )));
        }
    }
    Ok(())
}
