//! Модель проекта движка (serde, версия формата). UI не хранит свою копию —
//! читает снапшоты и шлёт команды.

use serde::{Deserialize, Serialize};

pub const FORMAT_VERSION: u32 = 1;

/// Тип источника канала.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChannelKind {
    Sampler,
    Kick,
    Synth,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Channel {
    pub id: u32,
    pub name: String,
    pub kind: ChannelKind,
    /// Путь к WAV для Sampler (пусто — синт-фолбэк).
    #[serde(default)]
    pub sample_path: String,
    /// 0..=1.
    #[serde(default = "def_vol")]
    pub volume: f32,
    /// −1..=1.
    #[serde(default)]
    pub pan: f32,
    #[serde(default = "def_true")]
    pub muted: bool,
}

fn def_vol() -> f32 {
    0.78
}
fn def_true() -> bool {
    true
}

/// Одна нота паттерна (позиция/длина в шагах 1/16, ключ MIDI).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ModelNote {
    pub step: u32,
    pub key: u16,
    pub vel: u8,
    pub len_steps: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pattern {
    pub id: u32,
    pub name: String,
    /// Шаги 1/16 (step sequencer): channel_id → список шагов.
    #[serde(default)]
    pub steps: Vec<(u32, u32)>, // (channel_id, step)
    /// Ноты piano roll (для Synth и мелодий).
    #[serde(default)]
    pub notes: Vec<(u32, ModelNote)>, // (channel_id, note)
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Clip {
    pub track: u32,
    /// Позиция в тактах.
    pub bar: u32,
    /// Длина в тактах (пока всегда ≥1).
    pub len_bars: u32,
    pub pattern_id: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectModel {
    pub version: u32,
    pub bpm: f64,
    pub ppq: u32,
    pub master_volume: f32,
    pub channels: Vec<Channel>,
    pub patterns: Vec<Pattern>,
    pub clips: Vec<Clip>,
    /// Следующий свободный id (каналы и паттерны делят пространство).
    pub next_id: u32,
}

impl Default for ProjectModel {
    fn default() -> Self {
        ProjectModel {
            version: FORMAT_VERSION,
            bpm: 120.0,
            ppq: 96,
            master_volume: 0.8,
            channels: vec![Channel {
                id: 0,
                name: "Sampler".into(),
                kind: ChannelKind::Kick,
                sample_path: String::new(),
                volume: 0.78,
                pan: 0.0,
                muted: false,
            }],
            patterns: vec![Pattern {
                id: 1,
                name: "Pattern 1".into(),
                steps: Vec::new(),
                notes: Vec::new(),
            }],
            clips: Vec::new(),
            next_id: 2,
        }
    }
}

impl ProjectModel {
    pub fn channel(&self, id: u32) -> Option<&Channel> {
        self.channels.iter().find(|c| c.id == id)
    }

    pub fn pattern(&self, id: u32) -> Option<&Pattern> {
        self.patterns.iter().find(|p| p.id == id)
    }
}
