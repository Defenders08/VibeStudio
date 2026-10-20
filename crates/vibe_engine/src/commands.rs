//! Единый enum команд + диспетчер. UI, тесты и (в будущем) CLI вызывают
//! только `dispatch`. Каждая мутирующая команда возвращает обратную команду —
//! на этом строится undo/redo.

use crate::model::{ChannelKind, ProjectModel};
use serde::{Deserialize, Serialize};

/// Режим воспроизведения.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlayMode {
    Pat,
    Song,
}

/// Статус транспорта.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TransportStatus {
    Stopped,
    Playing,
    Paused,
}

/// Событие ноты для планировщика (тики PPQ от начала цикла).
#[derive(Debug, Clone, Copy)]
pub struct NoteEvent {
    pub tick: f32,
    pub channel_id: u32,
    pub key: u16,
    pub vel: u8,
    pub dur_ticks: f32,
    pub gain: f32,
    pub pan: f32,
    pub kind: ChannelKind,
}

/// Все команды системы. Транспорт+проект в одном enum — единая дверь.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Command {
    // ---- транспорт ----
    Play,
    Pause,
    /// Stop: глушит голоса, позиция → 0 (1:1:0).
    Stop,
    /// Позиция в тиках.
    Seek { tick: f32 },
    SetBpm { bpm: f64 },
    SetMode { mode: PlayMode },
    /// Превью ноты вне транспорта (шаг/клавиша/ролл).
    PreviewNote { channel_id: u32, key: u16, vel: u8, dur_secs: f32 },
    SetMasterVolume { volume: f32 },

    // ---- каналы ----
    AddChannel { kind: ChannelKind, name: String },
    RemoveChannel { channel_id: u32 },
    SetChannelVolume { channel_id: u32, volume: f32 },
    SetChannelPan { channel_id: u32, pan: f32 },
    SetChannelMute { channel_id: u32, muted: bool },
    SetSamplePath { channel_id: u32, path: String },

    // ---- паттерны/шаги/ноты ----
    SetStep { pattern_id: u32, channel_id: u32, step: u32, on: bool },
    SetNote { pattern_id: u32, channel_id: u32, note: crate::model::ModelNote },
    RemoveNote { pattern_id: u32, channel_id: u32, step: u32 },
    RenamePattern { pattern_id: u32, name: String },

    // ---- клипы ----
    AddClip { track: u32, bar: u32, len_bars: u32, pattern_id: u32 },
    MoveClip { track: u32, bar: u32, new_track: u32, new_bar: u32 },
    ResizeClip { track: u32, bar: u32, new_len_bars: u32 },
    RemoveClip { track: u32, bar: u32 },
    SetClipMute { track: u32, bar: u32, muted: bool },
    SetTrackMute { track: u32, muted: bool },

    // ---- файлы ----
    /// Загрузить модель целиком (Open / New); обратная команда вернёт старую.
    LoadProject { model: ProjectModel },
}

/// Обратная команда для undo (None — команда необратима/идемпотентна).
pub fn inverse(cmd: &Command, model: &ProjectModel) -> Option<Command> {
    use Command as C;
    Some(match cmd {
        C::SetMasterVolume { volume } => C::SetMasterVolume { volume: model.master_volume },
        C::SetChannelVolume { channel_id, .. } => C::SetChannelVolume {
            channel_id: *channel_id,
            volume: model.channel(*channel_id)?.volume,
        },
        C::SetChannelPan { channel_id, .. } => C::SetChannelPan {
            channel_id: *channel_id,
            pan: model.channel(*channel_id)?.pan,
        },
        C::SetChannelMute { channel_id, .. } => C::SetChannelMute {
            channel_id: *channel_id,
            muted: model.channel(*channel_id)?.muted,
        },
        C::SetStep { pattern_id, channel_id, step, on } => {
            let p = model.pattern(*pattern_id)?;
            let is_on = p
                .steps
                .iter()
                .any(|(ch, s)| ch == channel_id && s == step);
            C::SetStep { pattern_id: *pattern_id, channel_id: *channel_id, step: *step, on: is_on }
        }
        C::SetBpm { .. } => C::SetBpm { bpm: model.bpm },
        C::AddClip { track, bar, .. } => C::RemoveClip { track: *track, bar: *bar },
        C::RemoveClip { track, bar } => {
            let c = model.clips.iter().find(|c| c.track == *track && c.bar == *bar)?;
            C::AddClip { track: c.track, bar: c.bar, len_bars: c.len_bars, pattern_id: c.pattern_id }
        }
        C::MoveClip { track, bar, new_track, new_bar } => {
            let c = model.clips.iter().find(|c| c.track == *new_track && c.bar == *new_bar)?;
            C::MoveClip { track: *new_track, bar: *new_bar, new_track: *track, new_bar: *bar }
        }
        C::RemoveChannel { .. } | C::LoadProject { .. } => {
            C::LoadProject { model: model.clone() }
        }
        _ => return None,
    })
}
