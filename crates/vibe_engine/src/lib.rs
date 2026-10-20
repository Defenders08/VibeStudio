//! `vibe_engine` — аудиодвижок VibeStudio.
//!
//! Слои: commands (единый enum) → dispatch (модель+undo) → engine (cpal live
//! + offline) → render (WAV). Движок владеет моделью проекта; UI читает
//! снапшоты и шлёт команды.

pub mod commands;
pub mod dispatch;
pub mod engine;
pub mod model;
pub mod render;
pub mod synth;
pub mod voices;

pub use commands::{Command, NoteEvent, PlayMode, TransportStatus};
pub use engine::{Engine, Shared, TICKS_PER_BAR};
pub use render::render_to_wav;
pub use model::{Channel, ChannelKind, Clip, ModelNote, Pattern, ProjectModel, FORMAT_VERSION};
