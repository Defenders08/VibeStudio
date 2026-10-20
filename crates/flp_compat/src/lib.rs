//! `flp_compat` — прототип чтения FL Studio `.flp` (уровни L1–L4 по `docs/FLP_COMPATIBILITY.md`).
//!
//! Границы прототипа (честно):
//! - читаем только профиль FL 25.2.x (профиль = таблица override'ов токенайзера);
//! - ноты декодируются только по `position`, остальное — сырые байты;
//! - plugin state (`0xD5`) — только длина, состояние opaque;
//! - записи FLP прототип не пишет (writer — уровень L8, не в этом заходе).
//!
//! Модель data-driven и не зависит ни от какого UI-тулкита: порядок каналов =
//! порядок Channel Rack, цвета/имена/позиции — готовые поля для FL-подобного UI.

pub mod error;
pub mod model;
pub mod report;
pub mod tokenizer;
pub mod writer;

pub use error::FlpError;
pub use model::{build, detect_stride, parse, Project};
pub use report::ImportReport;
pub use tokenizer::{tokenize, Event, Header, Profile};
pub use writer::set_tempo;

use std::path::Path;

/// Прочитать и разобрать `.flp` с диска (профиль FL25).
pub fn parse_file(path: impl AsRef<Path>) -> Result<Project, FlpError> {
    let data = std::fs::read(path)?;
    parse(&data)
}
