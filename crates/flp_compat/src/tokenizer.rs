//! Потоковый TLV-токенайзер FLP (уровень L1 из `docs/FLP_COMPATIBILITY.md` §6.2).
//!
//! Правила размера payload по диапазону opcode (все [CONFIRMED] на фикстурах FL 25.2.4):
//! `< 0x40` → 1 байт; `0x40..0x7F` → 2; `0x80..0xBF` → 4; `>= 0xC0` → varint-длина + payload.
//! Исключения (версионные override) задаются профилем: например `0xAC` на FL 25.2.x —
//! ровно 3 сырых байта (классическое правило DWORD прочтёт 4 и съест следующий opcode).

use crate::error::FlpError;
use serde::Serialize;

/// `FLhd` целиком: magic(4) + len(4) + format(2) + channels(2) + ppq(2).
pub const FLHD_LEN: usize = 14;
/// Первый event: 14 (`FLhd`) + 8 (заголовок `FLdt`). Подтверждено на 5 фикстурах.
pub const EVENTS_START: usize = 22;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Header {
    pub format: u16,
    /// Поле заголовка «n_channels» — legacy, ненадёжно (на фикстурах 1 или 2, не 0).
    /// Парсер его НЕ использует для валидации.
    pub channels_field: u16,
    pub ppq: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Event {
    /// Смещение события от начала файла (для отчётов и UI).
    pub offset: usize,
    pub opcode: u8,
    pub payload: Vec<u8>,
}

/// Версионный override размера payload (профиль поколения FL).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Override {
    /// Фиксированный размер payload вне диапазонного правила (`0xAC` на FL 25.2.x = 3).
    Raw(usize),
    /// DATA-кадрирование: varint-длина + payload (гипотеза для FL 26, README форка PyFLP).
    VarintData,
}

/// Профиль версии FL: таблица override'ов поверх диапазонного правила.
#[derive(Debug, Clone, Copy)]
pub struct Profile {
    pub name: &'static str,
    pub overrides: &'static [(u8, Override)],
}

/// Профиль FL 25.2.x — единственное поколение, подтверждённое нашими фикстурами.
pub const FL25: Profile = Profile {
    name: "fl25",
    overrides: &[(0xAC, Override::Raw(3))],
};

#[derive(Debug, Clone)]
pub struct Tokenized {
    pub header: Header,
    /// Значение `FLdt.data_len` — поток должен закончиться ровно на `EVENTS_START + data_len`.
    pub data_len: usize,
    pub events: Vec<Event>,
    /// Байты после конца `FLdt` (нестандартно; фиксируются в отчёт, не парсятся).
    pub trailing_bytes: usize,
}

impl Event {
    pub fn payload_u16(&self) -> Option<u16> {
        if self.payload.len() < 2 {
            return None;
        }
        Some(u16::from_le_bytes(self.payload[0..2].try_into().unwrap()))
    }

    pub fn payload_u32(&self) -> Option<u32> {
        if self.payload.len() < 4 {
            return None;
        }
        Some(u32::from_le_bytes(self.payload[0..4].try_into().unwrap()))
    }

    pub fn payload_i32(&self) -> Option<i32> {
        self.payload_u32().map(|v| v as i32)
    }

    /// Null-terminated UTF-16LE (0xC0/0xC1/0xC4/0xCB/0xCC/0xCD/0xCF/0xF1).
    pub fn payload_utf16z(&self) -> Option<String> {
        if self.payload.len() % 2 != 0 {
            return None;
        }
        let units: Vec<u16> = self
            .payload
            .chunks_exact(2)
            .map(|c| u16::from_le_bytes(c.try_into().unwrap()))
            .take_while(|&u| u != 0)
            .collect();
        Some(String::from_utf16_lossy(&units))
    }

    /// Null-terminated ASCII (0xC7 — строка версии).
    pub fn payload_ascii_z(&self) -> Option<String> {
        let end = self.payload.iter().position(|&b| b == 0).unwrap_or(self.payload.len());
        String::from_utf8(self.payload[..end].to_vec()).ok()
    }
}

fn read_header(data: &[u8]) -> Result<Header, FlpError> {
    if data.len() < FLHD_LEN {
        return Err(FlpError::ShortFile { need: FLHD_LEN, len: data.len() });
    }
    let mut magic = [0u8; 4];
    magic.copy_from_slice(&data[0..4]);
    if &magic != b"FLhd" {
        return Err(FlpError::NotFlp { found: magic });
    }
    let mut fdt = [0u8; 4];
    if data.len() < FLHD_LEN + 4 {
        return Err(FlpError::ShortFile { need: FLHD_LEN + 4, len: data.len() });
    }
    fdt.copy_from_slice(&data[14..18]);
    if &fdt != b"FLdt" {
        return Err(FlpError::NotFldt { found: fdt });
    }
    Ok(Header {
        format: u16::from_le_bytes(data[8..10].try_into().unwrap()),
        channels_field: u16::from_le_bytes(data[10..12].try_into().unwrap()),
        ppq: u16::from_le_bytes(data[12..14].try_into().unwrap()),
    })
}

/// LEB128 (little-endian base-128) — длина payload DATA/TEXT-событий.
fn read_varint(data: &[u8], pos: &mut usize, end: usize) -> Result<u64, FlpError> {
    let offset = *pos;
    let mut value: u64 = 0;
    let mut shift: u32 = 0;
    loop {
        if *pos >= end {
            return Err(FlpError::BadVarint { offset });
        }
        let byte = data[*pos];
        *pos += 1;
        value |= u64::from(byte & 0x7F) << shift;
        if byte & 0x80 == 0 {
            break;
        }
        shift += 7;
        if shift > 42 {
            // Длина payload не может превышать u32 в здоровом файле.
            return Err(FlpError::BadVarint { offset });
        }
    }
    Ok(value)
}

fn payload_len(opcode: u8, profile: &Profile) -> Option<usize> {
    if let Some((_, o)) = profile.overrides.iter().find(|(op, _)| *op == opcode) {
        return match o {
            Override::Raw(n) => Some(*n),
            // VarintData обрабатывается в общем DATA-пути ниже (None здесь = не fixed).
            Override::VarintData => None,
        };
    }
    Some(match opcode {
        0x00..=0x3F => 1,
        0x40..=0x7F => 2,
        0x80..=0xBF => 4,
        _ => return None, // DATA/TEXT: varint-длина + payload
    })
}

/// Разобрать поток событий. Гарантии (проверяются тестами):
/// курсор останавливается ровно на `EVENTS_START + data_len`; никакие payload
/// не пересекают границу `FLdt`; паники на мусорном входе исключены по построению.
pub fn tokenize(data: &[u8], profile: &Profile) -> Result<Tokenized, FlpError> {
    let header = read_header(data)?;
    if data.len() < EVENTS_START {
        return Err(FlpError::ShortFile { need: EVENTS_START, len: data.len() });
    }
    let data_len = u32::from_le_bytes(data[18..22].try_into().unwrap()) as usize;
    let end = EVENTS_START + data_len;
    if end > data.len() {
        return Err(FlpError::TruncatedData { data_len, len: data.len() });
    }

    let mut events = Vec::new();
    let mut pos = EVENTS_START;
    while pos < end {
        let opcode = data[pos];
        let ev_offset = pos;
        pos += 1;
        match payload_len(opcode, profile) {
            Some(fixed) => {
                if pos + fixed > end {
                    return Err(FlpError::Overrun { opcode, offset: ev_offset, need: fixed, end });
                }
                events.push(Event {
                    offset: ev_offset,
                    opcode,
                    payload: data[pos..pos + fixed].to_vec(),
                });
                pos += fixed;
            }
            None => {
                let len = read_varint(data, &mut pos, end)? as usize;
                if pos + len > end {
                    return Err(FlpError::Overrun {
                        opcode,
                        offset: ev_offset,
                        need: len,
                        end,
                    });
                }
                events.push(Event {
                    offset: ev_offset,
                    opcode,
                    payload: data[pos..pos + len].to_vec(),
                });
                pos += len;
            }
        }
    }
    if pos != end {
        return Err(FlpError::Trailing { actual: pos, expected: end });
    }

    Ok(Tokenized {
        header,
        data_len,
        events,
        trailing_bytes: data.len() - end,
    })
}
