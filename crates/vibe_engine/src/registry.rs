//! Registry-first ядро VibeStudio (вывод аудита OSS_AUDIT §7: единая таблица
//! команд порождает UI/CLI/MCP — иначе AI-слой потом не достроить).
//!
//! `EngineCommand` — единственный способ повлиять на движок; каждая варианта
//! описана в статической таблице REGISTRY (имя/док/мутирует). Покрытие
//! проверяется exhaustive-тестом (паттерн vibez).

/// Источник голоса: сэмпл кика (sampler-каналы, kind 0) или тон по MIDI-ключу.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum VoiceSrc {
    /// Синтезированный кик (замещает сэмпл FL, пока путь %FLStudioFactoryData%
    /// не резолвится; см. docs/research/provenance.md).
    Kick,
    /// Бип-синт: синус + 2-я гармоника, частота по MIDI-ключу.
    Tone,
}

/// Событие ноты в тиках PPQ (для live и offline — одна и та же модель).
#[derive(Debug, Clone, Copy)]
pub struct NoteEvent {
    /// Позиция в тиках от начала (для SONG — с учётом смещения клипа).
    pub tick: f32,
    pub key: u16,
    /// 0..=127.
    pub vel: u8,
    /// Длительность в тиках.
    pub dur_ticks: f32,
    /// Итоговое усиление (канальная громкость UI * мастер).
    pub gain: f32,
    pub src: VoiceSrc,
}

/// Все команды движка. Расширение списка = новая варианта + строка в REGISTRY.
#[derive(Debug)]
pub enum EngineCommand {
    /// Запуск воспроизведения набора событий (PAT — один паттерн, SONG —
    /// аранжировка клипов); `loop_ticks` — длина цикла.
    Play { events: Vec<NoteEvent>, loop_ticks: f32 },
    /// Стоп и сброс курсора.
    Stop,
    /// Перемотка в позицию (тики); работает и на стопе.
    Seek { tick: f32 },
    /// Темп (меняет скорость курсора на лету).
    SetBpm { bpm: f64 },
    /// Мгновенное превью ноты (шаг/клавиша/создание в ролле) вне транспорта.
    PreviewNote { key: u16, vel: u8, dur_secs: f32, gain: f32, src: VoiceSrc },
    /// Мастер-громкость 0..=1.
    SetMasterGain { gain: f32 },
}

/// Спецификация команды для реестра (будущее: генерация CLI/MCP-инструментов).
pub struct CommandSpec {
    pub name: &'static str,
    pub doc: &'static str,
    pub mutates: bool,
}

/// Единая таблица команд движка.
pub const REGISTRY: [CommandSpec; 6] = [
    CommandSpec { name: "transport.play", doc: "Запустить события (PAT/SONG) с циклом loop_ticks", mutates: true },
    CommandSpec { name: "transport.stop", doc: "Стоп и сброс курсора", mutates: true },
    CommandSpec { name: "transport.seek", doc: "Перемотка (тики)", mutates: true },
    CommandSpec { name: "transport.setBpm", doc: "Темп BPM на лету", mutates: true },
    CommandSpec { name: "note.preview", doc: "Мгновенное превью ноты (шаг/клавиша/ролл)", mutates: true },
    CommandSpec { name: "master.setGain", doc: "Мастер-громкость", mutates: true },
];

impl EngineCommand {
    /// Имя для реестра. Exhaustive: новая варианта без имени не скомпилируется,
    /// а тест сверяет соответствие REGISTRY.
    pub fn spec_name(&self) -> &'static str {
        match self {
            EngineCommand::Play { .. } => "transport.play",
            EngineCommand::Stop => "transport.stop",
            EngineCommand::Seek { .. } => "transport.seek",
            EngineCommand::SetBpm { .. } => "transport.setBpm",
            EngineCommand::PreviewNote { .. } => "note.preview",
            EngineCommand::SetMasterGain { .. } => "master.setGain",
        }
    }
}
