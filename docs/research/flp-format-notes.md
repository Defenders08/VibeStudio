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

## 5. Подтверждение байтами и новые находки (прототип flp_compat, 2026-10-20 вечер)

Первый запуск собственного Rust-парсера на 5 публичных фикстурах (golden-тесты в `crates/flp_compat/tests/`). Всё, что ниже, — наш собственный замер, не пересказ спеки:

1. **Спека подтверждена байтами на всех 5 файлах**, не только на документированной base_empty: события с offset 22; первый `0xC7` ("25.2.4.4960"), `0x9F` (build 4960), `0xAC`@48 = `01 01 00`, следом `0xC0` varint 54 = UTF-16 "FL Studio 25.2.4.4960.4960". PPQ = 96, format = 0, `data_len` покрывает файл ровно.
2. **Правка спеки: `0xE9` на пустом playlist СУЩЕСТВУЕТ** (base_empty offset 1241, payload длиной 0). Спека §3.1 утверждала «simply absent... produces no event rather than a zero-record blob» — на FL 25.2.4 событие есть, blob пустой. Декодер: пустой blob = «клипов нет», без предупреждения.
3. **MixerParams `0xE1` разложены по сырым байтам**: 6924 B = 577 записей × 12 B на базовых файлах. Структура: FL insert-ID 0 (Master, 1 запись) + ID 53 и 64..80 (18 видимых insert'ов × 32 записи = 576). 32 записи на insert = 10 слотов × {id 0 SlotEnabled (msg=1), id 1 SlotMix (msg=12800)} + 12 insert-уровневых (Volume/Pan/StereoSeparation/EQ gains/freqs/Qs). **Главный вывод: `insert_idx` в 0xE1 — внутренний FL ID, он НЕ равен порядку накопления `0x93`.** Маппинг ID→0x93 не установлен: дифференциальный эксперимент base_empty vs base_one_insert показал, что добавление EQ2 не меняет ни одну группу 0xE1 (состояние плагина живёт в `0xD5`). Для маппинга нужны файлы с изменёнными volume/pan insert'ов.
4. **Нота (24 B), base_one_pattern**: `position` u32@0 = 0 ✓ (flpkit); `raw[4..6]` = 0x4000 — флаги (независимое подтверждение flpkit fallback); [ГИПОТЕЗА] `u32@8` = 0x30 = MIDI key 48, `u16@6` = длина; хвост `40 64 80 80` — те же константы, что `_u2` playlist-записи. Поля не декодируем до сверки с оракулом.
5. **Дефолтный проект не пуст**: даже base_empty содержит канал «Sampler» (iid 0, kind 0, цвет 0x484441, Levels=дефолт) — FL всегда создаёт его; golden-тесты это фиксируют.
6. `0xC9` встречается в scope канала (internal class name по каналам), `0xD4`/`0xD5` тоже — добавлены в модель как plugin_internal/plugin_runtime_len/plugin_state_len (у SerumTest state в channel scope, не в mixer slot).
7. `0xEC` (insert flags, 12 B) присутствует ×18 (по одному на insert) — хранится сырым.

Следствия обновлены в `crates/flp_compat/src/model.rs` (док-комментарии) и тестах.

## 5.1. Оракул flpdiff CLI: дифференциальная валидация и layout ноты (2026-10-20, ночь)

Подключён оракул: `flpdiff` CLI v0.1.2 (darwin-arm64, официальный релиз; перед запуском нужна ad-hoc подпись `codesign -s -`, иначе kill/137).

**Текстовый `info` совпал с нашим разбором по всем пунктам** на base_empty: версия, 120.0 BPM, 4/4, PPQ 96, 1 sampler-канал, 18 insert'ов, 1 аранжировка (500 треков, 0 клипов).

**Layout ноты (24 B) выверен полностью** через `info --format json` (base_one_pattern, оракул: `channel_iid=1, key=63, length=48, fine_pitch=120, pan=64, release=64, velocity=100, position=0` — все значения найдены в наших байтах):

| Offset | Тип | Поле | Значение в фикстуре |
|---|---|---|---|
| 0 | u32 | position | 0 |
| 4 | u16 | flags (дефолт 0x4000) | 0x4000 |
| 6 | u16 | channel_iid | 1 |
| 8 | u32 | length_ticks | 48 |
| 12 | u16 | key (MIDI) | 63 |
| 14 | u16 | reserved (0) | 0 |
| 16 | u16 | fine_pitch | 120 |
| 18 | u16 | pan (64 = центр) | 64 |
| 20 | u8 | release | 64 |
| 21 | u8 | velocity | 100 |
| 22 | u8×2 | tail (0x80 0x80) | 0x80, 0x80 |

**Цвет**: u32, младшие LE-байты = R, G, B. Проверено на трёх цветах оракулом: канал Sampler 4736321 → (65,69,72), Kick 6972764 → (92,101,106), паттерн P1 3815732 → (52,57,58) — все совпали с RGBA из JSON.

**Writer-валидация оракулом**: наш splice `set_tempo(140)` на base_one_pattern → `flpdiff <modified> <original>` = ровно 1 изменение «Tempo decreased from 140.0 to 120.0 BPM» (exit 1), сам с собой = «No changes» (exit 0). Сплайс-запись семантически валидна по мнению референс-парсера.

Note: u16@6 в ноте — это **channel_iid** (а не «длина в шагах», как я предполагал ранее): паттерн-blob хранит ноты всех каналов вместе, каждая с ссылкой на канал.

## 6. Рекомендуемое продолжение для Claude (проверено и отправляется пользователем)

```text
Yes — continue in your proposed order (1→4). Additions:

1. In the consolidated opcode table, add a "verified by" column (spec §/line, flpkit file, PyFLP issue). Your corrections are recorded in our project docs; I re-verified them against the spec myself.
2. 88-byte FL 2026 record: check flpkit's detect.py playlist stride-signature approach as the second source; if it only restates the same observation, tag it [SINGLE-SOURCE].
3. Wrapper question (0xD4 absent from the spec; flpkit lists 212): reconcile flpkit formats/effects.py captured references with the spec's "0xC9 value + 0xD5 record stream" description; PyFLP issues on WrapperEvent are fair game (issues only, no code).
4. The spec's 0xDB entry reads self-contradictory ("pre-FL-25 saves emit Levels at 0xCB, but 0xCB is taken by Name") — resolve where legacy Levels actually live, via fl25-event-format.md and pre-25 sources.
5. Evaluate your phantom-0x01 detector idea against the 5 public FL 25 fixtures (byte-level reasoning without executing code is fine); if it holds, include it in the Task 2 design alongside the structural invariants: contiguous iids from 0x40, 0x62 in groups of 10 per insert, stopping exactly at 22 + data_len.
6. After Task 1, proceed to Tasks 2–4, then write the final documents: docs/FLP_COMPATIBILITY.md in Russian (project convention); drafting in English and translating at the end is fine.
```
