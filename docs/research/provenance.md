# Provenance log — чистая комната FLP

Журнал происхождения знаний и артефактов (обязателен по docs/FLP_COMPATIBILITY.md §1).
Формат: дата | источник | лицензия | что извлечено/куда попало.

| Дата | Источник | Лицензия | Использование |
|---|---|---|---|
| 2026-10-20 | dawhubapp/flpdiff, коммит `28aae8f66471be9bf83e342089bbf5b627cfe309` | MIT | Спецификация `docs/fl-format/flp-format-spec.md` (517 строк) — справочник формата; локальная копия `/tmp/oss_audit/flp-spec.md` |
| 2026-10-20 | там же: `tests/corpus/re_base/fl25/*.flp` (5 файлов FL 25.2.4) | MIT | Вендорены в `crates/flp_compat/tests/fixtures/` — тестовый корпус прототипа |
| 2026-10-20 | clarkipeng/flpkit README + заметки аудита (`docs/research/flpkit.md`) | MIT | Методология verify-by-readback / detect-don't-assume (в writer, будущий L8); `0x4000` fallback флагов нот |
| 2026-10-20 | PyFLP issue #208 (по пересказу из аудита, код не читался) | GPL (факты из issue) | Гипотеза `0xAC` = 3 байта; подтверждена нашими байтами |
| 2026-10-20 | Байты 5 фикстур (наш замер, xxd + парсер) | — (данные) | Все golden-значения в `crates/flp_compat/tests/fixtures_tests.rs` |

## Замеры на байтах (наша собственная верификация, 2026-10-20)

- Все 5 фикстур: `FLhd` валиден, `format=0`, `ppq=96`; `FLdt.data_len` покрывает файл ровно (trailing = 0).
- Все 5: события с offset 22; первый opcode `0xC7` (ASCII "25.2.4.4960"), затем `0x9F` (build 4960), `0xA9` (DWORD 7), `0x1C` (BYTE 1), `0xAC`@48 payload `01 01 00`, `0xC0`@52 varint 54 — UTF-16 "FL Studio 25.2.4.4960.4960".
- `n_channels` = 1 (empty/insert) или 2 (channel/pattern/serum) — не ноль, но ненадёжен; не используется.

## Инструментальная цепочка

- Rust toolchain установлен изолированно: `RUSTUP_HOME=/tmp/oss_audit/rustup`, `CARGO_HOME=/tmp/oss_audit/cargo`, `CARGO_TARGET_DIR=/tmp/oss_audit/target` — ничего в `$HOME` и системных путях; удаление = `rm -rf /tmp/oss_audit`.
- Оракул: `flpdiff` CLI v0.1.2 (darwin-arm64, официальный релиз-ассет) в `/tmp/oss_audit/flpdiff-cli`; перед запуском обязательна ad-hoc подпись `codesign -s -` (иначе SIGKILL/137). Использован для: текстового `info` (сверка структуры), `info --format json` (выверка layout ноты и цветов), семантического `diff` (валидация writer'а).
- Пример writer'а: `cargo run -p flp_compat --example set_tempo -- <in.flp> <out.flp>` (BPM 140).

## Ограничения чистой комнаты

- GPL-код PyFLP и форков не читался и не копировался; использовались только MIT-спека, MIT-код/README flpkit и пересказы issue (см. docs/research/flp-format-notes.md §«clean-room»).
- Известная оговорка: сама MIT-спека flpdiff консультировалась с GPL-таблицами (факты, не код) — учтено.
