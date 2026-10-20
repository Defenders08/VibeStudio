//! ImportReport — сводка разбора для UI и тестов: что распознано, что opaque,
//! какие предупреждения. Пользователю показывается именно он (принцип
//! «никакой ложной совместимости» из брифа).

use crate::model::Project;
use std::collections::BTreeMap;

#[derive(Debug, Clone)]
pub struct ImportReport {
    pub version_string: Option<String>,
    pub banner: Option<String>,
    pub ppq: u16,
    pub tempo_bpm: Option<f64>,
    pub channels: usize,
    pub patterns: usize,
    pub notes: usize,
    pub arrangements: usize,
    pub clips: usize,
    pub inserts: usize,
    pub slots_with_names: usize,
    /// Число записей `0xE1` MixerParams (глобально, с FL insert-ID).
    pub mixer_params: usize,
    pub stride_used: Option<u8>,
    pub warnings: Vec<String>,
    pub unknown_opcodes: BTreeMap<u8, usize>,
    /// Байты после конца `FLdt` (0 у всех известных нам файлов).
    pub trailing_bytes: usize,
}

impl ImportReport {
    pub fn from(p: &Project) -> Self {
        ImportReport {
            version_string: p.version_string.clone(),
            banner: p.banner.clone(),
            ppq: p.header.ppq,
            tempo_bpm: p.tempo_bpm,
            channels: p.channels.len(),
            patterns: p.patterns.len(),
            notes: p.patterns.iter().map(|pt| pt.notes.len()).sum(),
            arrangements: p.arrangements.len(),
            clips: p.arrangements.iter().map(|ar| ar.clips.len()).sum(),
            inserts: p.inserts.len(),
            slots_with_names: p
                .inserts
                .iter()
                .flat_map(|i| i.slots.iter())
                .filter(|s| s.name.is_some())
                .count(),
            mixer_params: p.mixer_params.len(),
            stride_used: p
                .arrangements
                .iter()
                .flat_map(|ar| ar.clips.iter())
                .map(|c| c.stride)
                .next(),
            warnings: p.warnings.clone(),
            unknown_opcodes: p.unknown_opcodes.clone(),
            trailing_bytes: p.trailing_bytes,
        }
    }
}
