//! `vibe_engine` — аудиодвижок VibeStudio.
//!
//! Слои: registry (команды) → engine (cpal live) → render (offline WAV).
//! Данные проекта передаются снапшотами `NoteEvent` — движок не знает о UI
//! и о формате FLP.

pub mod engine;
pub mod registry;
pub mod render;
pub mod synth;
pub mod voice;

pub use engine::{Engine, TransportSnapshot, TransportStatus};
pub use registry::{ChannelVoice, EngineCommand, NoteEvent, PlayMode, VoiceSrc};
pub use render::render_to_wav;
