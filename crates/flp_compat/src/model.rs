//! Семантическая модель поверх потока событий (уровни L2–L4).
//!
//! State-machine walker с явным scope (channel / pattern / arrangement / mixer):
//! атрибуция `0xCB` (имя канала vs имя плагина слота) невозможна без него
//! (docs/research/flp-format-notes.md §2).
//!
//! Модель сознательно «плоская» и data-driven: порядок каналов = порядок Channel Rack,
//! цвета/имена/позиции — готовое сырьё для FL-подобного UI. Поля, чей layout ещё не
//! выверен (notes кроме position, track_rvidx, 0xEE), хранятся сырыми байтами/счётчиком.

use crate::tokenizer::{self, Event, Header, Tokenized};
use serde::Serialize;
use std::collections::BTreeMap;

/// Цвет FLP хранится как u32, младшие LE-байты = R, G, B (выверено оракулом:
/// 4736321 → R65 G69 B72, совпадает с RGBA `info --format json`).
pub fn color_to_rgb8(v: u32) -> (u8, u8, u8) {
    (v as u8, (v >> 8) as u8, (v >> 16) as u8)
}

/// Значения по умолчанию Channel Levels (спека flpdiff §3.1, `0xDB`).
pub const LEVELS_DEFAULT: Levels = Levels {
    pan: 6400,
    volume: 10000,
    pitch: 0,
    filter_mod_x: 256,
    filter_mod_y: 0,
    filter_type: 0,
};

/// `pattern_base` в playlist-записях — константа 20480 = 0x5000 (сигнатура stride-детектора).
pub const PATTERN_BASE_SIGNATURE: u16 = 0x5000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Levels {
    pub pan: i32,
    pub volume: u32,
    pub pitch: i32,
    pub filter_mod_x: u32,
    pub filter_mod_y: u32,
    pub filter_type: u32,
}

#[derive(Debug, Clone, Serialize)]
pub struct Channel {
    pub iid: u16,
    /// `0x15`: 0=sampler, 2=native, 3=layer, 4=instrument, 5=automation.
    pub kind: Option<u8>,
    pub name: Option<String>,
    /// RGBA, упаковка LE: младшие байты = R, G, B (см. [`color_to_rgb8`]).
    pub color: Option<u32>,
    pub sample_path: Option<String>,
    pub levels: Option<Levels>,
    pub enabled: Option<bool>,
    /// `0xC9` в scope канала — internal class name плагина (например "Fruity Wrapper").
    pub plugin_internal: Option<String>,
    /// Длина opaque-состояния плагина канала (`0xD5` в scope канала, например Serum).
    pub plugin_state_len: Option<usize>,
    /// Длина `0xD4` в scope канала (runtime data slot-кортежа).
    pub plugin_runtime_len: Option<usize>,
}

/// Запись ноты (24 байта). Layout полностью выверен оракулом flpdiff v0.1.2
/// (`info --format json`, base_one_pattern, 2026-10-20): каждое поле совпало
/// с его выводом (channel_iid=1, key=63, length=48, fine_pitch=120, pan=64,
/// release=64, velocity=100).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct NoteRecord {
    pub position: u32,
    /// Флаги ноты; дефолт 0x4000 (flpkit fallback + наша фикстура).
    pub flags: u16,
    /// Канал-владелец (iid): ноты паттерна хранятся одним blob на все каналы.
    pub channel_iid: u16,
    /// Длина в тиках PPQ.
    pub length_ticks: u32,
    /// MIDI-ключ.
    pub key: u16,
    /// Назначение не установлено (в фикстуре 0).
    pub reserved: u16,
    /// Fine pitch (в фикстуре 120).
    pub fine_pitch: u16,
    /// Пан, 64 = центр.
    pub pan: u16,
    pub release: u8,
    pub velocity: u8,
    /// Два байта 0x80 (назначение не установлено).
    pub tail: [u8; 2],
}

impl NoteRecord {
    /// Полная lossless-раскладка 24 байтов в поля (все байты учтены).
    pub fn from_bytes(raw: &[u8; 24]) -> NoteRecord {
        NoteRecord {
            position: u32::from_le_bytes(raw[0..4].try_into().unwrap()),
            flags: u16::from_le_bytes(raw[4..6].try_into().unwrap()),
            channel_iid: u16::from_le_bytes(raw[6..8].try_into().unwrap()),
            length_ticks: u32::from_le_bytes(raw[8..12].try_into().unwrap()),
            key: u16::from_le_bytes(raw[12..14].try_into().unwrap()),
            reserved: u16::from_le_bytes(raw[14..16].try_into().unwrap()),
            fine_pitch: u16::from_le_bytes(raw[16..18].try_into().unwrap()),
            pan: u16::from_le_bytes(raw[18..20].try_into().unwrap()),
            release: raw[20],
            velocity: raw[21],
            tail: [raw[22], raw[23]],
        }
    }

    /// Обратная сборка (для будущего writer'а); байт-в-байт обратна `from_bytes`.
    pub fn to_bytes(&self) -> [u8; 24] {
        let mut raw = [0u8; 24];
        raw[0..4].copy_from_slice(&self.position.to_le_bytes());
        raw[4..6].copy_from_slice(&self.flags.to_le_bytes());
        raw[6..8].copy_from_slice(&self.channel_iid.to_le_bytes());
        raw[8..12].copy_from_slice(&self.length_ticks.to_le_bytes());
        raw[12..14].copy_from_slice(&self.key.to_le_bytes());
        raw[14..16].copy_from_slice(&self.reserved.to_le_bytes());
        raw[16..18].copy_from_slice(&self.fine_pitch.to_le_bytes());
        raw[18..20].copy_from_slice(&self.pan.to_le_bytes());
        raw[20] = self.release;
        raw[21] = self.velocity;
        raw[22] = self.tail[0];
        raw[23] = self.tail[1];
        raw
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Pattern {
    pub id: u16,
    pub name: Option<String>,
    pub color: Option<u32>,
    /// Длина в тиках PPQ (0 = «один такт по умолчанию»).
    pub length_ticks: Option<u32>,
    pub looped: bool,
    pub notes: Vec<NoteRecord>,
    /// Число 12-байтовых controller-записей (`0xDF`), layout не декодируется.
    pub controller_records: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct Clip {
    pub position: u32,
    pub pattern_base: u16,
    pub item_index: u16,
    pub length: u32,
    /// «Хранится реверснутым» (спека); декодирование в индекс дорожки UI — TODO.
    pub track_rvidx: u16,
    pub group: u16,
    pub item_flags: u16,
    pub start_offset: f32,
    pub end_offset: f32,
    pub clip_id: u32,
    /// Размер записи, которым разобран blob (stride из детектора).
    pub stride: u8,
}

#[derive(Debug, Clone, Serialize)]
pub struct Arrangement {
    pub id: u16,
    pub name: Option<String>,
    pub clips: Vec<Clip>,
    /// Число событий `0xEE` (track data, 70 B, layout не декодируется).
    pub track_event_count: usize,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct Slot {
    pub index: u16,
    pub name: Option<String>,
    pub color: Option<u32>,
    /// Длина opaque-состояния `0xD5`.
    pub state_len: Option<usize>,
    /// Длина `0xD4` (runtime data в slot-кортеже, FL 25).
    pub runtime_len: Option<usize>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Insert {
    /// Порядковый номер в файле (0 = первый, в FL-порядке накопления).
    pub index: usize,
    pub name: Option<String>,
    pub color: Option<u32>,
    /// Output routing из `0x93` (None = сентинел -1/self, «default»).
    pub routing: Option<i32>,
    pub slots: Vec<Slot>,
    /// Сырой payload `0xEC` insert flags (12 B; layout не декодируется).
    pub flags_raw: Option<Vec<u8>>,
}

/// Запись `0xE1` MixerParams: insertIdx = (channel_data >> 6) & 0x7F, slotIdx = channel_data & 0x3F.
#[derive(Debug, Clone, Copy, Serialize)]
pub struct ParamRecord {
    pub insert_idx: u8,
    pub slot_idx: u8,
    pub param_id: u8,
    pub value: i32,
}

#[derive(Debug, Clone, Serialize)]
pub struct Project {
    pub header: Header,
    pub data_len: usize,
    pub trailing_bytes: usize,
    /// `0x9C` BPM×1000.
    pub tempo_bpm: Option<f64>,
    /// `0x42` (coarse BPM) + `0x5D` (fine, milli-BPM) — legacy-пара.
    pub legacy_tempo: Option<(u16, u16)>,
    pub time_signature: Option<(u8, u8)>,
    /// `0xC7` ASCII-версия, например "25.2.4.4960".
    pub version_string: Option<String>,
    /// `0xC0` UTF-16 баннер "FL Studio …" (только первый в файле).
    pub banner: Option<String>,
    pub artists: Option<String>,
    /// Сырой payload первого `0xAC` (проектный флаг; кадрирование — см. tokenizer).
    pub ac_payload: Option<Vec<u8>>,
    pub channels: Vec<Channel>,
    pub patterns: Vec<Pattern>,
    pub arrangements: Vec<Arrangement>,
    pub inserts: Vec<Insert>,
    /// Записи `0xE1` MixerParams. `insert_idx` здесь — ВНУТРЕННИЙ FL ID insert'а
    /// (на дефолтном проекте: 53 и 64..80 = ровно 18 видимых insert'ов × 32 записи:
    /// 10 слотов × (SlotEnabled+SlotMix) + 12 insert-уровневых). Соответствие
    /// FL ID ↔ порядок накопления `0x93` НЕ установлено — маппинг на `inserts`
    /// не выполняется до дифференциального эксперимента (см. flp-format-notes.md).
    pub mixer_params: Vec<ParamRecord>,
    pub warnings: Vec<String>,
    pub unknown_opcodes: BTreeMap<u8, usize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Scope {
    Global,
    Channel,
    Pattern,
    Arrangement,
    Mixer,
}

/// Собрать семантическую модель из потока событий.
pub fn build(tk: Tokenized) -> Project {
    let mut p = Project {
        header: tk.header,
        data_len: tk.data_len,
        trailing_bytes: tk.trailing_bytes,
        tempo_bpm: None,
        legacy_tempo: None,
        time_signature: None,
        version_string: None,
        banner: None,
        artists: None,
        ac_payload: None,
        channels: Vec::new(),
        patterns: Vec::new(),
        arrangements: Vec::new(),
        inserts: Vec::new(),
        mixer_params: Vec::new(),
        warnings: Vec::new(),
        unknown_opcodes: BTreeMap::new(),
    };

    let mut scope = Scope::Global;
    let mut cur_channel: Option<usize> = None;
    let mut cur_pattern: Option<usize> = None;
    let mut cur_arrangement: Option<usize> = None;
    let mut mixer_open = false;
    let mut cur_insert = Insert { index: 0, name: None, color: None, routing: None, slots: Vec::new(), flags_raw: None };
    let mut cur_slot: u16 = 0;
    let mut ac_seen = false;

    for ev in &tk.events {
        match ev.opcode {
            // ---- Глобальные (обрабатываются в любом scope) ----
            0x9C => p.tempo_bpm = ev.payload_u32().map(|v| v as f64 / 1000.0),
            0x42 => {
                let fine = p.legacy_tempo.map_or(0, |t| t.1);
                p.legacy_tempo = Some((ev.payload_u16().unwrap_or(0), fine));
            }
            0x5D => {
                let coarse = p.legacy_tempo.map_or(0, |t| t.0);
                p.legacy_tempo = Some((coarse, ev.payload_u16().unwrap_or(0)));
            }
            0x11 => {
                let den = p.time_signature.map_or(4, |t| t.1);
                p.time_signature = Some((ev.payload.first().copied().unwrap_or(4), den));
            }
            0x12 => {
                let num = p.time_signature.map_or(4, |t| t.0);
                p.time_signature = Some((num, ev.payload.first().copied().unwrap_or(4)));
            }
            0xC7 => p.version_string = ev.payload_ascii_z(),
            0xCF => p.artists = ev.payload_utf16z(),
            0xC0 => {
                // Первый 0xC0 в файле на FL 25 — баннер версии (подтверждено байтами
                // на всех 5 фикстурах); остальные 0xC0 — версионно-зависимы.
                if p.banner.is_none() && ev.payload_utf16z().is_some_and(|t| t.starts_with("FL Studio")) {
                    p.banner = ev.payload_utf16z();
                } else {
                    p.warnings.push(format!(
                        "0xC0 вне баннера на offset {} (FL ≤ 24: имя канала; проектный blob) — оставлен opaque",
                        ev.offset
                    ));
                }
            }
            0xAC => {
                if !ac_seen {
                    ac_seen = true;
                    p.ac_payload = Some(ev.payload.clone());
                    if ev.payload.len() != 3 {
                        p.warnings.push(format!(
                            "0xAC на offset {}: payload {} байт(ы) вместо 3 (профиль fl25) — возможно другое поколение FL",
                            ev.offset,
                            ev.payload.len()
                        ));
                    }
                }
            }

            // ---- Переключатели scope ----
            0x40 => {
                let iid = ev.payload_u16().unwrap_or(u16::MAX);
                if let Some(prev) = p.channels.last() {
                    if iid != prev.iid + 1 {
                        p.warnings.push(format!(
                            "инвариант сплошных iid нарушен: 0x40 на offset {} даёт iid {}, предыдущий {}",
                            ev.offset, iid, prev.iid
                        ));
                    }
                }
                p.channels.push(Channel {
                    iid,
                    kind: None,
                    name: None,
                    color: None,
                    sample_path: None,
                    levels: None,
                    enabled: None,
                    plugin_internal: None,
                    plugin_state_len: None,
                    plugin_runtime_len: None,
                });
                cur_channel = Some(p.channels.len() - 1);
                scope = Scope::Channel;
            }
            0x41 => {
                let id = ev.payload_u16().unwrap_or(u16::MAX);
                // 0x41 срабатывает дважды на паттерн — дедуп по id.
                cur_pattern = match p.patterns.iter().position(|pt| pt.id == id) {
                    Some(i) => Some(i),
                    None => {
                        p.patterns.push(Pattern {
                            id,
                            name: None,
                            color: None,
                            length_ticks: None,
                            looped: false,
                            notes: Vec::new(),
                            controller_records: 0,
                        });
                        Some(p.patterns.len() - 1)
                    }
                };
                scope = Scope::Pattern;
            }
            0x63 => {
                let id = ev.payload_u16().unwrap_or(u16::MAX);
                p.arrangements.push(Arrangement {
                    id,
                    name: None,
                    clips: Vec::new(),
                    track_event_count: 0,
                });
                cur_arrangement = Some(p.arrangements.len() - 1);
                scope = Scope::Arrangement;
            }
            0x62 => {
                // Граница слота: контент, накопленный до этого `0x62`, принадлежит слоту v
                // (события слота K идут ПЕРЕД 0x62 со значением K).
                let v = ev.payload_u16().unwrap_or(0);
                if !mixer_open {
                    mixer_open = true;
                    scope = Scope::Mixer;
                }
                if cur_slot != v {
                    p.warnings.push(format!(
                        "0x62 на offset {}: ожидался слот {}, получен {}",
                        ev.offset, cur_slot, v
                    ));
                }
                cur_slot = v + 1;
            }

            // ---- Channel scope ----
            0x15 if scope == Scope::Channel => {
                if let Some(ch) = cur_channel.and_then(|i| p.channels.get_mut(i)) {
                    ch.kind = ev.payload.first().copied();
                }
            }
            0x80 if scope == Scope::Channel => {
                if let Some(ch) = cur_channel.and_then(|i| p.channels.get_mut(i)) {
                    ch.color = ev.payload_u32();
                }
            }
            0xC4 if scope == Scope::Channel => {
                if let Some(ch) = cur_channel.and_then(|i| p.channels.get_mut(i)) {
                    ch.sample_path = ev.payload_utf16z();
                }
            }
            0xCB if scope == Scope::Channel => {
                if let Some(ch) = cur_channel.and_then(|i| p.channels.get_mut(i)) {
                    ch.name = ev.payload_utf16z();
                }
            }
            0xDB if scope == Scope::Channel => {
                match decode_levels(ev) {
                    Some(lv) => {
                        if let Some(ch) = cur_channel.and_then(|i| p.channels.get_mut(i)) {
                            ch.levels = Some(lv);
                        }
                    }
                    None => p.warnings.push(format!("0xDB на offset {}: payload не 24 байта", ev.offset)),
                }
            }
            0x00 if scope == Scope::Channel => {
                if let Some(ch) = cur_channel.and_then(|i| p.channels.get_mut(i)) {
                    ch.enabled = ev.payload.first().map(|b| *b != 0);
                }
            }
            0xC9 if scope == Scope::Channel => {
                if let Some(ch) = cur_channel.and_then(|i| p.channels.get_mut(i)) {
                    ch.plugin_internal = ev.payload_utf16z();
                }
            }
            0xD5 if scope == Scope::Channel => {
                if let Some(ch) = cur_channel.and_then(|i| p.channels.get_mut(i)) {
                    ch.plugin_state_len = Some(ev.payload.len());
                }
            }
            0xD4 if scope == Scope::Channel => {
                if let Some(ch) = cur_channel.and_then(|i| p.channels.get_mut(i)) {
                    ch.plugin_runtime_len = Some(ev.payload.len());
                }
            }

            // ---- Pattern scope ----
            0xC1 if scope == Scope::Pattern => {
                if let Some(pt) = cur_pattern.and_then(|i| p.patterns.get_mut(i)) {
                    pt.name = ev.payload_utf16z();
                }
            }
            0x96 if scope == Scope::Pattern => {
                if let Some(pt) = cur_pattern.and_then(|i| p.patterns.get_mut(i)) {
                    pt.color = ev.payload_u32();
                }
            }
            0xA4 if scope == Scope::Pattern => {
                if let Some(pt) = cur_pattern.and_then(|i| p.patterns.get_mut(i)) {
                    pt.length_ticks = ev.payload_u32();
                }
            }
            0x1A if scope == Scope::Pattern => {
                if let Some(pt) = cur_pattern.and_then(|i| p.patterns.get_mut(i)) {
                    pt.looped = true;
                }
            }
            0xE0 if scope == Scope::Pattern => {
                if let Some(pt) = cur_pattern.and_then(|i| p.patterns.get_mut(i)) {
                    append_notes(pt, ev, &mut p.warnings);
                }
            }
            0xDF if scope == Scope::Pattern => {
                if let Some(pt) = cur_pattern.and_then(|i| p.patterns.get_mut(i)) {
                    if ev.payload.len() % 12 != 0 {
                        p.warnings.push(format!("0xDF на offset {}: payload не кратен 12", ev.offset));
                    }
                    pt.controller_records += ev.payload.len() / 12;
                }
            }

            // ---- Arrangement scope ----
            0xF1 if scope == Scope::Arrangement => {
                if let Some(ar) = cur_arrangement.and_then(|i| p.arrangements.get_mut(i)) {
                    ar.name = ev.payload_utf16z();
                }
            }
            0xE9 if scope == Scope::Arrangement => {
                if let Some(ar) = cur_arrangement.and_then(|i| p.arrangements.get_mut(i)) {
                    decode_playlist(ar, ev, &mut p.warnings);
                }
            }
            0xEE if scope == Scope::Arrangement => {
                if let Some(ar) = cur_arrangement.and_then(|i| p.arrangements.get_mut(i)) {
                    ar.track_event_count += 1;
                }
            }

            // ---- Mixer scope ----
            0xCC if scope == Scope::Mixer => {
                cur_insert.name = ev.payload_utf16z();
            }
            0x95 if scope == Scope::Mixer => {
                cur_insert.color = ev.payload_u32();
            }
            0x80 if scope == Scope::Mixer => {
                let idx = cur_slot as usize;
                if cur_insert.slots.len() <= idx {
                    cur_insert.slots.resize(idx + 1, Slot::default());
                }
                if let Some(slot) = cur_insert.slots.get_mut(idx) {
                    slot.index = cur_slot;
                    slot.color = ev.payload_u32();
                }
            }
            0xCB if scope == Scope::Mixer => {
                let idx = cur_slot as usize;
                if cur_insert.slots.len() <= idx {
                    cur_insert.slots.resize(idx + 1, Slot::default());
                }
                if let Some(slot) = cur_insert.slots.get_mut(idx) {
                    slot.index = cur_slot;
                    slot.name = ev.payload_utf16z();
                }
            }
            0xD5 if scope == Scope::Mixer => {
                let idx = cur_slot as usize;
                if cur_insert.slots.len() <= idx {
                    cur_insert.slots.resize(idx + 1, Slot::default());
                }
                if let Some(slot) = cur_insert.slots.get_mut(idx) {
                    slot.index = cur_slot;
                    slot.state_len = Some(ev.payload.len());
                }
            }
            0xD4 if scope == Scope::Mixer => {
                let idx = cur_slot as usize;
                if cur_insert.slots.len() <= idx {
                    cur_insert.slots.resize(idx + 1, Slot::default());
                }
                if let Some(slot) = cur_insert.slots.get_mut(idx) {
                    slot.index = cur_slot;
                    slot.runtime_len = Some(ev.payload.len());
                }
            }
            0xEC if scope == Scope::Mixer => {
                cur_insert.flags_raw = Some(ev.payload.clone());
            }
            0x93 => {
                // Закрытие insert'а + output routing; сентинелы -1/self = default → None.
                cur_insert.routing = match ev.payload_i32() {
                    Some(-1) | None => None,
                    Some(v) if v == cur_insert.index as i32 => None,
                    Some(v) => Some(v),
                };
                p.inserts.push(std::mem::replace(
                    &mut cur_insert,
                    Insert {
                        index: p.inserts.len() + 1,
                        name: None,
                        color: None,
                        routing: None,
                        slots: Vec::new(),
                        flags_raw: None,
                    },
                ));
                cur_insert.index = p.inserts.len();
                cur_slot = 0;
            }
            0xE1 => {
                // Проектный блоб MixerParams: layout записи подтверждён сырыми байтами,
                // но insert_idx — FL ID, не наш индекс; глобально, без маппинга.
                for rec in decode_mixer_params(ev) {
                    p.mixer_params.push(rec);
                }
            }

            other => {
                *p.unknown_opcodes.entry(other).or_insert(0) += 1;
            }
        }
    }

    // Хвостовой insert пушим только если успел накопить контент (после финального
    // 0x93 он всегда пустой — фантомный insert нам не нужен).
    let tail_has_content = cur_insert.name.is_some()
        || cur_insert.color.is_some()
        || cur_insert.routing.is_some()
        || !cur_insert.slots.is_empty();
    if tail_has_content {
        p.inserts.push(cur_insert);
    } else if p.inserts.is_empty() {
        // Микшерной секции не было вовсе — пустой master для единообразия модели.
        p.inserts.push(Insert { index: 0, name: None, color: None, routing: None, slots: Vec::new(), flags_raw: None });
    }

    p
}

fn decode_levels(ev: &Event) -> Option<Levels> {
    if ev.payload.len() != 24 {
        return None;
    }
    let b = &ev.payload;
    Some(Levels {
        pan: i32::from_le_bytes(b[0..4].try_into().unwrap()),
        volume: u32::from_le_bytes(b[4..8].try_into().unwrap()),
        pitch: i32::from_le_bytes(b[8..12].try_into().unwrap()),
        filter_mod_x: u32::from_le_bytes(b[12..16].try_into().unwrap()),
        filter_mod_y: u32::from_le_bytes(b[16..20].try_into().unwrap()),
        filter_type: u32::from_le_bytes(b[20..24].try_into().unwrap()),
    })
}

/// Blob нот может быть разбит на несколько `0xE0` — при чтении конкатенируются.
fn append_notes(pt: &mut Pattern, ev: &Event, warnings: &mut Vec<String>) {
    if ev.payload.len() % 24 != 0 {
        warnings.push(format!("0xE0 на offset {}: payload не кратен 24", ev.offset));
    }
    for chunk in ev.payload.chunks_exact(24) {
        let raw: [u8; 24] = chunk.try_into().unwrap();
        pt.notes.push(NoteRecord::from_bytes(&raw));
    }
}

/// Детект stride: кандидаты {88, 80, 60, 32}; blob кратен S и u16@4 каждой записи == 0x5000.
/// Спека выбирает по версии FL (80/60 неоднозначны на кратных 240) — сигнатура здесь валидатор.
/// Возвращает (stride, число записей); для пустого/нераспознанного blob — None.
pub fn detect_stride(blob: &[u8]) -> Option<(u8, usize)> {
    const CANDIDATES: [u8; 4] = [88, 80, 60, 32];
    if blob.is_empty() {
        return None;
    }
    let mut best: Option<(u8, usize)> = None;
    for s in CANDIDATES {
        let us = s as usize;
        if blob.len() % us != 0 {
            continue;
        }
        let n = blob.len() / us;
        let all_match = (0..n).all(|i| {
            u16::from_le_bytes(blob[i * us + 4..i * us + 6].try_into().unwrap())
                == PATTERN_BASE_SIGNATURE
        });
        if all_match && best.map_or(true, |(bs, _)| s > bs) {
            best = Some((s, n));
        }
    }
    best
}

fn decode_playlist(ar: &mut Arrangement, ev: &Event, warnings: &mut Vec<String>) {
    if ev.payload.is_empty() {
        // ФАКТ (наше замер, base_empty offset 1241): на FL 25.2.4 `0xE9` присутствует
        // с ПУСТЫМ payload — спека flpdiff §3.1 («simply absent») здесь неточна.
        // Нормальный случай: клипов нет, предупреждение не нужно.
        return;
    }
    let Some((stride, _)) = detect_stride(&ev.payload) else {
        warnings.push(format!(
            "0xE9 на offset {}: stride не определён ({} байт) — blob оставлен неразобранным",
            ev.offset,
            ev.payload.len()
        ));
        return;
    };
    let us = stride as usize;
    for chunk in ev.payload.chunks_exact(us) {
        ar.clips.push(Clip {
            position: u32::from_le_bytes(chunk[0..4].try_into().unwrap()),
            pattern_base: u16::from_le_bytes(chunk[4..6].try_into().unwrap()),
            item_index: u16::from_le_bytes(chunk[6..8].try_into().unwrap()),
            length: u32::from_le_bytes(chunk[8..12].try_into().unwrap()),
            track_rvidx: u16::from_le_bytes(chunk[12..14].try_into().unwrap()),
            group: u16::from_le_bytes(chunk[14..16].try_into().unwrap()),
            item_flags: u16::from_le_bytes(chunk[18..20].try_into().unwrap()),
            start_offset: f32::from_le_bytes(chunk[24..28].try_into().unwrap()),
            end_offset: f32::from_le_bytes(chunk[28..32].try_into().unwrap()),
            clip_id: u32::from_le_bytes(chunk[32..36].try_into().unwrap()),
            stride,
        });
    }
}

fn decode_mixer_params(ev: &Event) -> Vec<ParamRecord> {
    let mut out = Vec::new();
    for chunk in ev.payload.chunks_exact(12) {
        let channel_data = u16::from_le_bytes(chunk[6..8].try_into().unwrap());
        out.push(ParamRecord {
            param_id: chunk[4],
            insert_idx: ((channel_data >> 6) & 0x7F) as u8,
            slot_idx: (channel_data & 0x3F) as u8,
            value: i32::from_le_bytes(chunk[8..12].try_into().unwrap()),
        });
    }
    out
}

/// Быстрый вход: токенайзер с профилем FL25 + сборка модели.
pub fn parse(data: impl AsRef<[u8]>) -> Result<Project, crate::error::FlpError> {
    let tk = tokenizer::tokenize(data.as_ref(), &tokenizer::FL25)?;
    Ok(build(tk))
}
