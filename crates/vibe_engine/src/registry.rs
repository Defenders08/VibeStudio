//! Registry-first ядро VibeStudio (вывод аудита OSS_AUDIT §7: единая таблица
//! команд порождает UI/CLI/MCP — иначе AI-слой потом не достроить).
//!
//! `EngineCommand` — единственный способ повлиять на движок; каждая варианта
//! описана в статической таблице REGISTRY (имя/док/мутирует). Покрытие
//! проверяется exhaustive-тестом (паттерн vibez).

/// Режим воспроизведения (переключение на лету не сбрасывает позицию).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayMode {
    /// Текущий паттерн в цикле.
    Pat,
    /// Аранжировка: клипы триггерят свои паттерны по позициям на дорожках.
    Song,
}

/// Источник голоса: синт-кик (sampler-каналы, kind 0) или тон по MIDI-ключу.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ChannelVoice {
    pub src: VoiceSrc,
    /// −1..=1 (равная мощность).
    pub pan: f32,
}

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
    pub channel: ChannelVoice,
}

/// Все команды движка. Расширение списка = новая варианта + строка в REGISTRY.
#[derive(Debug)]
pub enum EngineCommand {
    /// Запуск воспроизведения набора событий; `loop_ticks` — длина цикла
    /// (PAT — один паттерн, SONG — аранжировка).
    Play { events: Vec<NoteEvent>, loop_ticks: f32, mode: PlayMode },
    /// Пауза: голоса заглушаются, позиция сохраняется.
    Pause,
    /// Стоп: позиция → 0 (в UI это «1:1:0»).
    Stop,
    /// Перемотка в позицию (тики); работает на стопе/паузе/игре.
    Seek { tick: f32 },
    /// Темп (меняет скорость курсора на лету, позиция в тиках сохраняется).
    SetBpm { bpm: f64 },
    /// Смена режима без перезапуска: события подменяются, позиция сохраняется.
    SetMode { events: Vec<NoteEvent>, loop_ticks: f32, mode: PlayMode },
    /// Мгновенное превью ноты (шаг/клавиша/создание в ролле) вне транспорта.
    PreviewNote { key: u16, vel: u8, dur_secs: f32, gain: f32, pan: f32, src: VoiceSrc },
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
pub const REGISTRY: [CommandSpec; 8] = [
    CommandSpec { name: "transport.play", doc: "Запустить события (PAT/SONG) с циклом loop_ticks", mutates: true },
    CommandSpec { name: "transport.pause", doc: "Пауза: позиция сохраняется", mutates: true },
    CommandSpec { name: "transport.stop", doc: "Стоп и сброс курсора в 0 (1:1:0)", mutates: true },
    CommandSpec { name: "transport.seek", doc: "Перемотка (тики)", mutates: true },
    CommandSpec { name: "transport.setBpm", doc: "Темп BPM на лету", mutates: true },
    CommandSpec { name: "transport.setMode", doc: "Смена PAT/SONG без сброса позиции", mutates: true },
    CommandSpec { name: "note.preview", doc: "Мгновенное превью ноты (шаг/клавиша/ролл)", mutates: true },
    CommandSpec { name: "master.setGain", doc: "Мастер-громкость", mutates: true },
];

impl EngineCommand {
    /// Имя для реестра. Exhaustive: новая варианта без имени не скомпилируется,
    /// а тест сверяет соответствие REGISTRY.
    pub fn spec_name(&self) -> &'static str {
        match self {
            EngineCommand::Play { .. } => "transport.play",
            EngineCommand::Pause => "transport.pause",
            EngineCommand::Stop => "transport.stop",
            EngineCommand::Seek { .. } => "transport.seek",
            EngineCommand::SetBpm { .. } => "transport.setBpm",
            EngineCommand::SetMode { .. } => "transport.setMode",
            EngineCommand::PreviewNote { .. } => "note.preview",
            EngineCommand::SetMasterGain { .. } => "master.setGain",
        }
    }
}
