//! Ошибки разбора FLP. Каждая ошибка несёт byte offset, где это возможно,
//! чтобы ImportReport и UI могли показать пользователю место проблемы.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum FlpError {
    #[error("это не FLP-файл: ожидалась сигнатура `FLhd` на offset 0, найдено {found:?}")]
    NotFlp { found: [u8; 4] },

    #[error("сигнатура `FLdt` не найдена на offset 14, найдено {found:?}")]
    NotFldt { found: [u8; 4] },

    #[error("файл короче заголовка: нужно {need} байт, получено {len}")]
    ShortFile { need: usize, len: usize },

    #[error("FLdt.data_len = {data_len} выходит за конец файла ({len}): файл усечён")]
    TruncatedData { data_len: usize, len: usize },

    #[error("некорректный varint на offset {offset}")]
    BadVarint { offset: usize },

    #[error(
        "событие 0x{opcode:02X} на offset {offset}: payload {need} байт(ы) выходит за конец FLdt ({end})"
    )]
    Overrun {
        opcode: u8,
        offset: usize,
        need: usize,
        end: usize,
    },

    #[error("поток событий закончился на offset {actual}, ожидался конец {expected}")]
    Trailing {
        actual: usize,
        expected: usize,
    },

    #[error("ошибка чтения файла: {0}")]
    Io(#[from] std::io::Error),

    #[error("событие 0x9C (tempo) не найдено — writer отказался работать")]
    TempoNotFound,

    #[error("bpm вне диапазона 1..=65535.999: {0}")]
    BpmOutOfRange(f64),

    #[error("verify-by-readback не сошёлся: {0}")]
    VerifyFailed(String),
}
