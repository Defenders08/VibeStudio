//! Диспетчер команд над моделью проекта. Единственная точка мутации;
//! возвращает обратную команду для undo/redo.

use crate::commands::{Command, PlayMode, TransportStatus};
use crate::model::{Channel, ChannelKind, ProjectModel};
use std::collections::VecDeque;

/// Undo/redo стеки на командах.
pub struct History {
    pub undo: VecDeque<Command>,
    pub redo: VecDeque<Command>,
    limit: usize,
}

impl Default for History {
    fn default() -> Self {
        History { undo: VecDeque::new(), redo: VecDeque::new(), limit: 100 }
    }
}

impl History {
    pub fn push_undo(&mut self, cmd: Command) {
        self.undo.push_back(cmd);
        if self.undo.len() > self.limit {
            self.undo.pop_front();
        }
        self.redo.clear();
    }

    pub fn undo(&mut self) -> Option<Command> {
        self.undo.pop_back().inspect(|c| self.redo.push_back(c.clone()))
    }

    pub fn redo(&mut self) -> Option<Command> {
        self.redo.pop_back().inspect(|c| self.undo.push_back(c.clone()))
    }
}

/// Применить команду к модели; вернуть обратную команду (для undo).
pub fn dispatch(model: &mut ProjectModel, cmd: &Command) -> Option<Command> {
    use Command as C;
    match cmd {
        C::SetMasterVolume { volume } => {
            let inv = C::SetMasterVolume { volume: model.master_volume };
            model.master_volume = volume.clamp(0.0, 1.5);
            Some(inv)
        }
        C::SetBpm { bpm } => {
            let inv = C::SetBpm { bpm: model.bpm };
            model.bpm = bpm.clamp(1.0, 1000.0);
            Some(inv)
        }
        C::AddChannel { kind, name } => {
            let id = model.next_id;
            model.next_id += 1;
            let inv = C::RemoveChannel { channel_id: id };
            model.channels.push(Channel {
                id,
                name: format!("{name} {id}"),
                kind: *kind,
                sample_path: String::new(),
                volume: 0.78,
                pan: 0.0,
                muted: false,
            });
            Some(inv)
        }
        C::RemoveChannel { channel_id } => {
            if model.channels.len() <= 1 {
                return None; // последний канал не удаляем
            }
            model.channels.retain(|c| c.id != *channel_id);
            for p in &mut model.patterns {
                p.steps.retain(|(ch, _)| ch != channel_id);
                p.notes.retain(|(ch, _)| ch != channel_id);
            }
            Some(C::LoadProject { model: model.clone() })
        }
        C::SetChannelVolume { channel_id, volume } => {
            let ch = model.channels.iter_mut().find(|c| c.id == *channel_id)?;
            let inv = C::SetChannelVolume { channel_id: *channel_id, volume: ch.volume };
            ch.volume = volume.clamp(0.0, 1.5);
            Some(inv)
        }
        C::SetChannelPan { channel_id, pan } => {
            let ch = model.channels.iter_mut().find(|c| c.id == *channel_id)?;
            let inv = C::SetChannelPan { channel_id: *channel_id, pan: ch.pan };
            ch.pan = pan.clamp(-1.0, 1.0);
            Some(inv)
        }
        C::SetChannelMute { channel_id, muted } => {
            let ch = model.channels.iter_mut().find(|c| c.id == *channel_id)?;
            let inv = C::SetChannelMute { channel_id: *channel_id, muted: ch.muted };
            ch.muted = *muted;
            Some(inv)
        }
        C::SetSamplePath { channel_id, path } => {
            let ch = model.channels.iter_mut().find(|c| c.id == *channel_id)?;
            let inv = C::SetSamplePath { channel_id: *channel_id, path: ch.sample_path.clone() };
            ch.sample_path = path.clone();
            Some(inv)
        }
        C::SetStep { pattern_id, channel_id, step, on } => {
            let p = model.patterns.iter_mut().find(|p| p.id == *pattern_id)?;
            let inv = C::SetStep {
                pattern_id: *pattern_id,
                channel_id: *channel_id,
                step: *step,
                on: p.steps.iter().any(|(ch, s)| ch == channel_id && s == step),
            };
            if *on {
                if !p.steps.iter().any(|(ch, s)| ch == channel_id && s == step) {
                    p.steps.push((*channel_id, *step));
                }
            } else {
                p.steps.retain(|(ch, s)| !(ch == channel_id && s == step));
            }
            Some(inv)
        }
        C::SetNote { pattern_id, channel_id, note } => {
            let p = model.patterns.iter_mut().find(|p| p.id == *pattern_id)?;
            let inv = C::RemoveNote {
                pattern_id: *pattern_id,
                channel_id: *channel_id,
                step: note.step,
            };
            p.notes.retain(|(ch, n)| !(ch == channel_id && n.step == note.step));
            p.notes.push((*channel_id, *note));
            Some(inv)
        }
        C::RemoveNote { pattern_id, channel_id, step } => {
            let p = model.patterns.iter_mut().find(|p| p.id == *pattern_id)?;
            let idx = p
                .notes
                .iter()
                .position(|(ch, n)| ch == channel_id && n.step == *step)?;
            let inv = C::SetNote {
                pattern_id: *pattern_id,
                channel_id: *channel_id,
                note: p.notes[idx].1,
            };
            p.notes.remove(idx);
            Some(inv)
        }
        C::RenamePattern { pattern_id, name } => {
            let p = model.patterns.iter_mut().find(|p| p.id == *pattern_id)?;
            let inv = C::RenamePattern { pattern_id: *pattern_id, name: p.name.clone() };
            p.name = name.clone();
            Some(inv)
        }
        C::AddClip { track, bar, len_bars, pattern_id } => {
            if model.clips.iter().any(|c| c.track == *track && c.bar == *bar) {
                return None;
            }
            model.clips.push(crate::model::Clip {
                track: *track,
                bar: *bar,
                len_bars: (*len_bars).max(1),
                pattern_id: *pattern_id,
                muted: false,
            });
            Some(C::RemoveClip { track: *track, bar: *bar })
        }
        C::MoveClip { track, bar, new_track, new_bar } => {
            let c = model.clips.iter_mut().find(|c| c.track == *track && c.bar == *bar)?;
            let inv = C::MoveClip { track: *new_track, bar: *new_bar, new_track: *track, new_bar: *bar };
            c.track = *new_track;
            c.bar = *new_bar;
            Some(inv)
        }
        C::ResizeClip { track, bar, new_len_bars } => {
            let c = model.clips.iter_mut().find(|c| c.track == *track && c.bar == *bar)?;
            let inv = C::ResizeClip { track: *track, bar: *bar, new_len_bars: c.len_bars };
            c.len_bars = (*new_len_bars).max(1);
            Some(inv)
        }
        C::RemoveClip { track, bar } => {
            let idx = model.clips.iter().position(|c| c.track == *track && c.bar == *bar)?;
            let c = model.clips.remove(idx);
            Some(C::AddClip { track: c.track, bar: c.bar, len_bars: c.len_bars, pattern_id: c.pattern_id })
        }
        C::SetClipMute { track, bar, muted } => {
            let c = model.clips.iter_mut().find(|c| c.track == *track && c.bar == *bar)?;
            let inv = C::SetClipMute { track: *track, bar: *bar, muted: c.muted };
            c.muted = *muted;
            Some(inv)
        }
        C::SetTrackMute { track, muted } => {
            let cur = model.muted_tracks.contains(track);
            let inv = C::SetTrackMute { track: *track, muted: cur };
            if *muted {
                if !cur {
                    model.muted_tracks.push(*track);
                }
            } else {
                model.muted_tracks.retain(|t| t != track);
            }
            Some(inv)
        }
        C::LoadProject { model: m } => {
            let inv = C::LoadProject { model: model.clone() };
            *model = m.clone();
            Some(inv)
        }
        // транспортные команды модель не меняют
        C::Play | C::Pause | C::Stop | C::Seek { .. } | C::SetMode { .. }
        | C::PreviewNote { .. } => None,
    }
}

/// Собрать события воспроизведения из модели (снапшот на момент вызова).
/// PAT — выбранный паттерн; SONG — клипы по позициям (клип триггерит паттерн
/// на своём диапазоне тактов).
pub fn build_events(
    model: &ProjectModel,
    pattern_id: u32,
    mode: PlayMode,
    bars: u32,
) -> (Vec<crate::commands::NoteEvent>, f32) {
    const TICKS_PER_BAR: f32 = 384.0; // PPQ 96 × 4
    const STEPS_PER_BAR: u32 = 16;
    let mut ev = Vec::new();
    let mut push_pattern = |ev: &mut Vec<crate::commands::NoteEvent>, offset_ticks: f32, pid: u32| {
        let Some(p) = model.pattern(pid) else { return };
        for (ch_id, step) in &p.steps {
            let Some(ch) = model.channel(*ch_id) else { continue };
            if ch.muted {
                continue;
            }
            ev.push(crate::commands::NoteEvent {
                tick: offset_ticks + *step as f32 * (TICKS_PER_BAR / STEPS_PER_BAR as f32),
                channel_id: *ch_id,
                key: 60,
                vel: 100,
                dur_ticks: TICKS_PER_BAR / STEPS_PER_BAR as f32,
                gain: ch.volume,
                pan: ch.pan,
                kind: ch.kind,
            });
        }
        for (ch_id, n) in &p.notes {
            let Some(ch) = model.channel(*ch_id) else { continue };
            if ch.muted {
                continue;
            }
            ev.push(crate::commands::NoteEvent {
                tick: offset_ticks + n.step as f32 * (TICKS_PER_BAR / STEPS_PER_BAR as f32),
                channel_id: *ch_id,
                key: n.key,
                vel: n.vel,
                dur_ticks: n.len_steps as f32 * (TICKS_PER_BAR / STEPS_PER_BAR as f32),
                gain: ch.volume,
                pan: ch.pan,
                kind: ch.kind,
            });
        }
    };
    match mode {
        PlayMode::Pat => {
            push_pattern(&mut ev, 0.0, pattern_id);
            (ev, bars as f32 * TICKS_PER_BAR)
        }
        PlayMode::Song => {
            for c in &model.clips {
                if c.muted || model.muted_tracks.contains(&c.track) {
                    continue;
                }
                for b in 0..c.len_bars {
                    push_pattern(&mut ev, (c.bar + b) as f32 * TICKS_PER_BAR, c.pattern_id);
                }
            }
            (ev, bars as f32 * TICKS_PER_BAR)
        }
    }
}

/// Источник семпла канала; None — сэмпл не загружен.
pub fn channel_sample(model: &ProjectModel, channel_id: u32) -> Option<(ChannelKind, String)> {
    let ch = model.channel(channel_id)?;
    Some((ch.kind, ch.sample_path.clone()))
}
