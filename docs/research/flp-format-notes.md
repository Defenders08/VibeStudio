# Заметки по формату FLP: выверенная карта событий (v2)

**Дата:** 2026-10-20. Источники: [`flp-format-spec.md`](https://github.com/dawhubapp/flpdiff/blob/main/docs/fl-format/flp-format-spec.md) (скачана 2026-10-20, 517 строк) + первый прогон `FLP_RESEARCH_PROMPT_EN` + **независимая выверка каждым пунктом grep'ом по тексту спеки** (в таблице — номера строк локальной копии `/tmp/oss_audit/flp-spec.md`).

Этот файл **заменяет** карту opcode'ов из `OSS_AUDIT.md` §3.2, `docs/research/flpdiff.md` и ранних версий промптов — там карта была собрана из заметок и содержала ошибки атрибуции (перепутаны «имя/маркер», «слоты/имя insert'а»).

---

## 1. Исправления против первой карты (все проверены мной по спеке)

| Было в первой карте | В действительности (спека) | Строка |
|---|---|---|
| `0x40` = имя канала | `0x40` = **маркер нового канала** (WORD, iid; iid'ы сплошные `0..n-1`; события после `0x40` до следующего `0x40`/`0x62` принадлежат этому каналу) | 117 |
| `0x41` = имя паттерна | `0x41` = **маркер id паттерна**, срабатывает **дважды** на паттерн — walker обязан дедуплицировать по id | 118, 317–321 |
| `0xC1` = цвет паттерна | `0xC1` = **имя паттерна** (UTF-16LE); цвет — `0x96` (DWORD, только если пользователь менял) | 137, 128 |
| `0xCC` = слоты | `0xCC` = **имя mixer-insert'а** (UTF-16LE; отсутствует, если не переименован) | 149 |
| `0x93` = inserts | `0x93` = **закрытие insert'а** + output routing (int32; сентинелы `-1` и self-index = «default») | 124 |
| `0x54` time marker | позиция маркера — `0x94` (uint32; старший бит `0x08000000` = signature kind, младшие 27 бит = тик) | 123 |
| `0xCB` = «user name», `0xC9` = «internal» | `0xCB` = **scope-sensitive имя** (канал до `0x62` / display name плагина слота после `0x62`); `0xC9` = внутреннее имя класса (`"Fruity Wrapper"`) | 145 |
| `0xD4` = wrapper | **в спеке отсутствует**; wrapper = значение `0xC9` + поток записей `0xD5` | grep: 0 совпадений |
| `0x16` routing | **в спеке отсутствует** — нужен второй источник | grep: 0 совпадений |
| self-check через FLhd.n_channels | `n_channels` = «legacy, ignored by modern parsers» — **не годится** для самопроверки | 37 |

Дополнительно подтверждено: `0x15` (тип канала: 0=sampler, 2=instrument «Native», 3=layer, 4=instrument, 5=automation), `0x80` (цвет канала, упаковка как `0x96`), `0xC4` (sample path, null-terminated UTF-16LE, отсутствует без сэмпла), `0xDB` (Levels 24 байта: int32 pan, uint32 volume, int32 pitch_shift, filter_mod_x/y, filter_type; дефолты pan 6400, volume 10000; нормализация volume/12800, pan/6400), `0xE1` (MixerParams — один блоб на проект из 12-байтовых записей, 577 записей на `base_empty.flp`; атрибуция полная: `insertIdx = (channel_data >> 6) & 0x7F`, `slotIdx = channel_data & 0x3F`; id: 0=SlotEnabled, 1=SlotMix, 64..191=RouteVol, 192=Volume, 193=Pan, 194=StereoSeparation, 208..226=EQ gains/freqs/Qs), `0xEC`, `0x9A` (audio input insert'а, те же сентинелы `-1`), `0xE7`, `0x9C` (BPM×1000), `0x42`+`0x5D` (legacy tempo), все четыре FL25-релокации, `0xAC` = **3 байта** (override `byte3`; классическое правило DWORD прочтёт 4 и съест следующий opcode; пример payload `01 01 00`; flpdiff issue #1).

## 2. Ключевые структурные факты

- **События начинаются с offset 22** (FLhd 14 байт + заголовок FLdt 8). Парсер обязан остановиться ровно на `22 + data_len`.
- **Scope-модель — ядро парсинга.** Атрибуция `0xCB` (и вообще событий) требует конечного автомата: channel scope (после `0x40`, до любого `0x62`) → mixer/insert scope (до `0x93`) → slot scope (после `0x62`). «Наивный первый 0xCB на канал» крадёт имена плагинов (спека, строки 305–315).
- **Слоты**: события слота K (имя `0xCB`, `0xD5` state и т.д.) приходят **ДО** закрывающего `0x62` со значением K; `0x62` всегда группами по 10 на insert (0..9), даже пустые.
- **Playlist `0xE9`**: отсутствует при пустом playlist (не бывает блоба нулевой длины); записи 80 B (FL 25) / 60 B (FL 21–24) / 32 B (pre-21); спека выбирает размер по версии FL (≥25→80, ≥21→60, иначе 32), потому что **80 и 60 делятся друг на друга на кратных 240** — неверный выбор = misaligned-парс, нулевые клипы, краш на playback. FL25-запись: `pattern_base` всегда 20480, `_u1` всегда 120, `item_flags` 0x0040 (старший байт 0x80 = selected), `_u2` = `40 64 80 80`, start/end_offset float32 (NaN/0xFFFFFFFF — pattern-клипы, -1.0 — audio), `clip_id` последовательный в рамках аранжировки, `scale` float64 (1.0, offset 64).
- **Самопроверки парсера** (вместо n_channels): сплошные iid от `0x40`; `0x62` группами по 10; стоп на `22 + data_len`; [ГИПОТЕЗА, из прогона Claude] phantom-детектор: при ложном чтении `0xAC` как 1 байта хвост `01 00` парсится как валидный phantom-`0x01` BYTE event (0) — если после каждого `0xAC` встречается такой фантом, чтение 1 байтом неверное. Silent/loud зависит от байтов 2–3 (`<0x40, any`) — minimal-pair эксперимент должен варьировать именно их.

## 3. Открытые пункты (нужен второй источник)

1. `0xD4`/212: flpkit называет 212 «wrapper», спека о `0xD4` не знает. Сверить с `flpkit/formats/effects.py` и PyFLP issues (WrapperEvent).
2. `0x16` routing (int8, максимум 127) — источник: мои старые заметки; найти подтверждение.
3. 88-байтовая запись FL 2026 — только flpkit.
4. `0xC0` по версиям: FL 25 — UTF-16 version banner; FL 24 — per-channel names (из прогона Claude; сверить с `fl25-event-format.md`).
5. Внутреннее противоречие спеки в записи `0xDB`: «pre-FL-25 saves emit Levels at `0xCB`, but `0xCB` is taken by Name» — где реально лежат Levels в legacy-файлах, выяснить.

## 4. Следствия для дизайна `flp_compat` (обновление §7.2 OSS_AUDIT)

- Self-check'и — структурные (§2), не n_channels.
- Stride: версия-первый выбор + **валидация по сигнатуре** выбранного размера (константы 20480/120/0x0040/`40 64 80 80`) — гибрид подхода спеки и detect-don't-assume flpkit.
- Walker со state machine для scope — обязательное требование, не оптимизация.
- Evidence policy: «confirmed» спеки = 5 публичных фикстур FL 25 + parity с Python-оракулом; всё, что покрыто только закрытым корпусом 85 файлов (60/32-байт клипы, legacy mixer layouts), у нас тегируется [INFERENCE], не [CONFIRMED].
- Clean-room: спека сама консультировалась с GPL override-таблицами (факты, не код) — зафиксировать в provenance log; MIT не гарантирует нулевого GPL-влияния на факты.
- Урок из истории flpdiff (хардкод `0xD9`, найден parity-прогоном на реальном корпусе): синтетических фикстур недостаточно — дифф-оракул на реальных файлах обязателен.

## 5. Рекомендуемое продолжение для Claude (проверено и отправляется пользователем)

```text
Yes — continue in your proposed order (1→4). Additions:

1. In the consolidated opcode table, add a "verified by" column (spec §/line, flpkit file, PyFLP issue). Your corrections are recorded in our project docs; I re-verified them against the spec myself.
2. 88-byte FL 2026 record: check flpkit's detect.py playlist stride-signature approach as the second source; if it only restates the same observation, tag it [SINGLE-SOURCE].
3. Wrapper question (0xD4 absent from the spec; flpkit lists 212): reconcile flpkit formats/effects.py captured references with the spec's "0xC9 value + 0xD5 record stream" description; PyFLP issues on WrapperEvent are fair game (issues only, no code).
4. The spec's 0xDB entry reads self-contradictory ("pre-FL-25 saves emit Levels at 0xCB, but 0xCB is taken by Name") — resolve where legacy Levels actually live, via fl25-event-format.md and pre-25 sources.
5. Evaluate your phantom-0x01 detector idea against the 5 public FL 25 fixtures (byte-level reasoning without executing code is fine); if it holds, include it in the Task 2 design alongside the structural invariants: contiguous iids from 0x40, 0x62 in groups of 10 per insert, stopping exactly at 22 + data_len.
6. After Task 1, proceed to Tasks 2–4, then write the final documents: docs/FLP_COMPATIBILITY.md in Russian (project convention); drafting in English and translating at the end is fine.
```
