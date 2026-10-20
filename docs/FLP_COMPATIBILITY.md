# FLP compatibility (`flp_compat`) — исследование формата, дизайн, тесты

**Дата верификации:** 2026-10-20. **Статус документа:** черновик исследования, не спецификация реализации.
**Метки:** [CONFIRMED] — прочитано в источнике и/или подтверждено байтами; [INFERENCE] — вывод из источников; [HYPOTHESIS] — нужен эксперимент; [SINGLE-SOURCE] — одно наблюдение, второго независимого источника нет.

> Номера строк в источниках недоступны (страницы GitHub получены как HTML без нумерации строк). Везде вместо `file:line` указаны раздел / строка таблицы / запись changelog. Перед публикацией в репозиторий их стоит дополнить из локальных клонов.

---

## 0. Что осталось непроверенным (читать первым)

1. **Исходники flpkit (`detect.py`, `codec.py`, `formats/levels.py`, `formats/effects.py`) не были прочитаны.** Автоматический доступ к файлам репозитория не получился (GitHub-ссылки на дерево/файлы не открылись, поиск вернул только README/PyPI). Всё о flpkit ниже — из его README (v0.8.1). Пункты «detect.py как второй источник для 88 B», «effects.py и 0xD4», «где живут legacy Levels» помечены соответственно. Нужно вручную положить эти 4 файла в сессию (лицензия MIT, читать можно).
2. **PyFLP issue #208 (`0xAC`, hex-дамп `01 01 00`) не получен**, только #200 (плейлист), #40, #72, #150. Версия FL, из которой взят дамп `01 01 00`, неизвестна.
3. **Фикстуры flpdiff (5 файлов FL 25.2.4) не открывались побайтно.** Единственный побайтовый дамп — `base_empty.flp` offset 48–55, приведённый в `fl25-event-format.md`. Утверждения «на всех пяти» взяты из текста спецификации.
4. Закрытый корпус автора flpdiff (85 файлов, FL 9–25) воспроизвести нельзя; всё, что опирается только на него, — максимум [INFERENCE] для нас.
5. Нет живого FL Studio: ни одного эксперимента я не проводил.

## 1. Clean-room протокол и provenance log

**Разрешено читать:** MIT-спецификация flpdiff (`docs/fl-format/*.md`), код flpkit (MIT), тексты issues/discussions PyFLP (факты формата, не код), README форков как документацию.
**Запрещено:** код GPL PyFLP и его форков; копирование структур/имён из них.

Записи provenance log (обязательно вести в `docs/research/provenance.md`: дата, URL, что прочитано, какой факт извлечён):

| Дата | Источник | Лицензия | Использовано для |
|---|---|---|---|
| 2026-10-20 | flpdiff `flp-format-spec.md` (517 строк) | MIT | таблица опкодов, envelope, 80/60/32 B |
| 2026-10-20 | flpdiff `fl25-event-format.md` (618 строк) | MIT | `0xAC`, `0xE3`, `0xEE`, `0xE0/0xDF`, порядок секций, `0xD4` в slot scope |
| 2026-10-20 | flpkit README (PyPI 0.8.1 и GitHub) | MIT | write-методология, `0xAC` для FL 2026, stride 88 |
| 2026-10-20 | Meowrium/PyFLP README (GPL-форк, только README) | GPL (не код) | утверждения об `0xAC` varint в FL26 и 88 B |
| 2026-10-20 | PyFLP issues #200, #40, #72, #150 (тексты) | — | 80 B запись; типы каналов |

**Инциденты загрязнения (зафиксировать честно):**
- Тело issue #200 содержит фрагмент Python-кода (struct playlist-записи) от автора обращения. Из него взят только факт «80 B, `pattern_base`=20480, поля `_u1.._u3`», который независимо присутствует в MIT-спецификации flpdiff. Структура кода не копировалась.
- Поисковая выдача вернула сниппеты GPL-файлов `pattern.py`/`arrangement.py` (номера строк, docstring-фрагменты). Не использовались; вывод в дизайн не попал.
- **Сама MIT-спецификация flpdiff признаёт, что сверялась с GPL-таблицами override'ов и со структурами PyFLP** (раздел «Clean-room boundary»; `fl25-event-format.md` прямо пишет «copied from PyFLP's `TrackEvent.STRUCT`» для `0xEE` и «cross-checked against PyFLP `NotesEvent.STRUCT`»). Факты формата (смещения) — допустимы, но «MIT» не равно «нулевое влияние GPL». Для юридической осторожности: реализуем поля `0xEE`/`0xE0` по своим экспериментам и фикстурам, а спецификацию используем как подсказку, а не как единственный источник.
- Форк Meowrium/PyFLP — не flpdiff-org; проверять LICENSE каждого файла перед любым заимствованием.

## 2. Контейнер и кадрирование событий

| Элемент | Значение | Метка | Подтверждено |
|---|---|---|---|
| `FLhd` | 14 байт: magic, `len`=6, `format` u16 (на практике 0), `n_channels` u16 (legacy), `ppq` u16 | [CONFIRMED] | flpdiff spec §1.1 |
| `FLdt` | magic, `data_len` u32; события с offset 22, парсинг до `22+data_len` | [CONFIRMED] | spec §1.2 |
| Размер payload по диапазону | `<0x40`:1; `0x40–0x7F`:2; `0x80–0xBF`:4; `≥0xC0`: LEB128 + N | [CONFIRMED] | spec §2.1 |
| `n_channels` | «legacy, always 0 on modern FL» | [CONFIRMED] | `fl25-event-format.md` («`header.n_channels` is legacy») |

Следствие: **проверка по `FLhd.n_channels` бесполезна** (поправка к исходному промпту).

## 3. Консолидированная таблица опкодов

«Verified by»: S = flpdiff `flp-format-spec.md`; E = `fl25-event-format.md`; K = flpkit README; I = issue PyFLP; F = форк PyFLP README. «Confirmed» в спецификации flpdiff означает: разобрано на 5 публичных фикстурах FL 25 и совпало с oracle `flp-info`; для немногих опкодов нет ни одного непустого значения в фикстурах (см. примечание после таблицы).

| Опкод | Смысл | Payload | Версии FL | Метка | Verified by |
|---|---|---|---|---|---|
| `0x00` | Channel IsEnabled | u8 bool | все | [CONFIRMED] | S §3.1 |
| `0x0C` | legacy main volume | u8 | до 25 | [CONFIRMED] (в FL25 отсутствует) | S §3.1 |
| `0x0F` | Channel IsZipped (отсутствие = false) | u8 | — | [CONFIRMED] | S §3.1 |
| `0x11`/`0x12` | Time signature num/den (проект) | u8 | — | [CONFIRMED] | S §3.1 |
| `0x14` | Channel PingPongLoop | u8 | — | [CONFIRMED] | S §3.1 |
| `0x15` | Channel type: 0 sampler, 2 native, 3 layer, 4 instrument, 5 automation | u8 | — | [CONFIRMED] | S §3.1; I #40 |
| `0x16` | «routing, int8, max 127» | — | — | **[UNKNOWN]** нет в S/E (в E упомянут как «defaults sensibly») | исходный промпт |
| `0x17` | Pan law (0 circular, 2 triangular) | u8 | — | [CONFIRMED] | S §3.1 |
| `0x1A` | Pattern looped (если не loop — события нет) | u8 | — | [CONFIRMED] | S §3.1 |
| `0x20` | Channel IsLocked | u8 | 12.3+ | [CONFIRMED] | S §3.1 |
| `0x21`/`0x22` | TimeMarker num/den | u8 | — | [CONFIRMED] | S §3.1 |
| `0x40` | **Новый канал** (iid u16) | u16 | — | [CONFIRMED] | S §3.1 |
| `0x41` | **Маркер паттерна** (id), срабатывает дважды на паттерн | u16 | — | [CONFIRMED] | S §3.1 |
| `0x42` + `0x5D` | Legacy tempo: coarse BPM + fine milli-BPM | u16+u16 | <3.4.0 / 3.4.0+ | [CONFIRMED] | S §3.1; K |
| `0x5F` | Insert icon id | u16 | — | [OBSERVED] (непустых нет) | S changelog 2026-04-20 |
| `0x62` | Закрытие mixer slot (idx u16); 10 на insert (FL 20–25); в FL 9 отсутствует | u16 | 20–25 | [CONFIRMED] | S §3.1, parity #8, #11 |
| `0x63` | Маркер arrangement (id) | u16 | — | [CONFIRMED] | S §3.1 |
| `0x80` | Цвет канала / слота (RGBA LE) | u32 | — | [CONFIRMED] | S §3.1 |
| `0x93` | Закрытие insert + output routing (int32; `-1` или self = default) | u32 | — | [CONFIRMED] граница; [OBSERVED] non-default | S §3.1, changelog 04-20 |
| `0x94` | Позиция TimeMarker; бит 27 = signature | u32 | — | [CONFIRMED] | S §3.1 |
| `0x95` | Цвет insert | u32 | — | [OBSERVED] | S changelog 04-20 |
| `0x96` | Цвет паттерна | u32 | — | [CONFIRMED] | S §3.1 |
| `0x9A` | Audio input insert (-1 default) | u32 | — | [OBSERVED] | S changelog 04-20 |
| `0x9C` | Tempo = BPM×1000 | u32 | modern | [CONFIRMED] (sweep 100–160) | S §3.1; E |
| `0x9F` | FL build | u32 | — | [CONFIRMED] | S §3.1 |
| `0xA4` | Pattern length (PPQ; 0 = default bar) | u32 | — | [CONFIRMED] | S §3.1 |
| **`0xAC`** | **см. §4.1** | 3 B (25.2.4) / varint? (FL26) | зависит от версии | [CONFIRMED] для 25.2.4 | S §2.2; E; K; F |
| `0xC0` | FL25: баннер версии UTF-16; FL≤24: имя канала; бывают крупные blobs (project properties) | DATA | — | [CONFIRMED] баннер; [UNKNOWN] blob | S §2.2, §3.2 |
| `0xC1` | Имя паттерна UTF-16 | DATA | — | [CONFIRMED] | S §3.1 |
| `0xC4` | Путь sample канала | DATA | — | [CONFIRMED] | S §3.1 |
| `0xC7` | Версия FL ASCII | DATA | — | [CONFIRMED] | S §3.1 |
| `0xC9` | Internal class name (`"Fruity Wrapper"` для VST) | DATA | — | [CONFIRMED] | S §3.1 |
| `0xCB` | **Имя канала (channel scope) / имя плагина (slot scope)** — зависит от scope | DATA | — | [CONFIRMED] | S §3.1 |
| `0xCC` | Имя insert | DATA | — | [CONFIRMED] | S §3.1 |
| `0xCD` | Имя TimeMarker | DATA | — | [CONFIRMED] | S §3.1 |
| `0xCF` | Project Artists (не controllers!) | DATA | — | [CONFIRMED] | S §3.1 |
| `0xD4` | В slot scope: «runtime data» плагина, часть кортежа `0xC9,0xD4,0x9B/0x80/0x29,0xD5` | DATA | FL25 | [OBSERVED] | E («Synthesizing FLPs») |
| `0xD5` | Plugin state: VST = record stream {u32 id,u64 len,data}; ids 54 name, 56 vendor, 55 path, 51 FourCC, 52 GUID, 53 state; native = per-plugin | DATA | — | [CONFIRMED] структура VST; state opaque | S §3.1; E |
| `0xD7` | «plugin-state blob» (в E, раздел createChannel) | — | — | **[UNKNOWN]**, вероятно устаревшая опечатка | E |
| `0xDB` | Channel Levels, 24 B: pan(i32), volume, pitch, filter_x, filter_y, filter_type; defaults 6400/10000/0/256/0/0 | DATA | FL25 | [CONFIRMED] | S §3.1; K («Levels 219») |
| `0xDF` | Pattern controllers, 12 B записи | DATA | — | [CONFIRMED] | S §3.1; E |
| `0xE0` | Pattern notes, 24 B записи; несколько событий в паттерне конкатенируются при чтении | DATA | FL25 (раньше `0xD0`) | [CONFIRMED] | S §3.1; E |
| `0xE1` | MixerParams, 12 B записи; `insert=(d>>6)&0x7F`, `slot=d&0x3F` | DATA | — | [CONFIRMED] | S §3.1 |
| `0xE3` | RemoteController (automation→target), 20 B; dest бит 13 = mixer slot | DATA | — | [OBSERVED] (800+ событий в закрытом корпусе) | E |
| `0xE7` | Routing bit-stream (не матрица) | DATA | — | [OBSERVED] | S §3.1 |
| `0xE9` | Playlist clips: 32/60/80/88 B | DATA | см. §4.2 | [CONFIRMED] 32/60/80 | S §3.1; I #200; K |
| `0xEC` | Insert flags, 12 B (FL25; раньше `0xDC`) | DATA | FL25 | [CONFIRMED] | S §3.1 |
| `0xEE` | Track data, 70 B × 500 на arrangement (раньше `0xDE`) | DATA | FL25 | [CONFIRMED] размер; поля [OBSERVED] | S §3.1; E |
| `0xEF` | Track name (упомянут) | DATA | — | [UNKNOWN] детали | E (раздел Section ordering) |
| `0xF1` | Имя arrangement | DATA | — | [CONFIRMED] | S §3.1 |

Примечания:
- Исправления к исходному промпту (подтверждены S): `0x40`=маркер канала (не имя), `0x41`=маркер паттерна (не имя), `0xC1`=имя паттерна (не цвет), `0xCC`=имя insert (не слоты), маркер времени `0x94` (не `0x54`).
- **Внутренняя неконсистентность E:** раздел createChannel перечисляет «`0xCB` levels» и «`0xD7` plugin-state blob»; S и K говорят `0xDB` и `0xD5`. Считать E там устаревшим.
- **Внутреннее противоречие S и E:** S §6 объявляет write path non-goal, тогда как E документирует encoder-side (`createChannel`, `addPatternNote`) и live-FL round-trip (D-50). Вывод: write фактически делался в flpdiff, но спецификацией не покрыт.
- Mixer slot: слот из файла `0x62 N` = слот `N+1` в runtime API FL; события до первого `0x62` = слот 0 (E, «Mixer slot index»). Это важно для live-oracle скриптов.

## 4. Спорные зоны

### 4.1 Событие `0xAC` — это зависит от версии FL, а не «кто прав»

Факты:
- FL 25.2.4: 3 байта, дамп `01 01 00`, далее `c0 36 …` баннер `"FL Studio 25.2.4.4960.4960\0"`. [CONFIRMED] (E, раздел `0xAC`, байты 48–55 `base_empty.flp`; исправлено по flpdiff issue #1).
- flpkit README: «event 172 is one byte on FL 2026, not the classic four» + `event_size_overrides`, «built-in FL-2026 table is fallback». [SINGLE-SOURCE] (документация, код не читан).
- Форк Meowrium README: «Event 0xAC is varint-length in FL26, not a fixed 4-byte DWORD — this single change fixes the stream desync». [SINGLE-SOURCE] (GPL-форк, только README).

**Вывод [HYPOTHESIS]:** расхождение «3 vs 1» объясняется двумя разными версиями FL: ≤25.2.x — 3 сырых байта; FL 26 — payload с префиксом длины (varint, вероятно длина 1 + 1 байт данных). flpkit «one byte» может считать только data-часть. Нужны фикстуры FL 26 с побайтовым дампом. Рабочая гипотеза «3 байта» из промпта верна только для 25.2.x.

**Проверка идеи «phantom `0x01`» (пункт 5 задания).**
Байтовое рассуждение на единственном документированном дампе `ac 01 01 00 c0 36 …`:
- Чтение A (3 B): `ac [01 01 00] | c0 36 …`.
- Чтение B (1 B): `ac [01] | 01 [00] | c0 36 …` — `01` попадает в BYTE-диапазон и «съедает» `00`; граница события `c0` совпадает.
- Чтение C (varint+data): `ac len=01 [01] | 00 [c0] | 36 …` — `00` как BYTE с payload `c0`, дальше `36` как BYTE в середине UTF-16 баннера: поток **рассинхронизируется**, баннер дробится.
- Чтение D (4 B DWORD): `ac [01 01 00 c0] | 36 …` — съедает опкод `c0`, тоже рассинхрон (то, что описывает E).

Итог: **A и B неразличимы по границам событий** для payload вида `xx 01 yy` и обеих «согласованных» разборов, поэтому детектор «phantom 0x01» сам по себе **не различает A и B** — он лишь констатирует неоднозначность. Он различает A/B от C/D, но это даёт и обычная проверка «следующий опкод `0xC0` с баннером `"FL Studio"`» (выполняется для A и B). Проверено только на 1 из 5 фикстур (на единственной с документированными байтами); для остальных четырёх [UNVERIFIED].

**Практическое следствие для дизайна:** т.к. IR хранит байты события дословно, различие A/B влияет на *идентичность событий*, но не на byte-exact round-trip. Опасны только C/D. Включаем в дизайн **не phantom-детектор, а candidate-framing с lookahead** (§6.2) плюс структурные инварианты. Эта оговорка важна: версия FL известна лишь по `0xC7/0x9F/0xC0`, которые в потоке идут *после* `0xAC` (offset 48 против баннера), поэтому «выбрать по версии» нельзя без пред-сканирования.

### 4.2 Playlist-запись `0xE9`: 32 → 60 → 80 → 88

- 80 B (FL 2024/25): [CONFIRMED] — S (запись с полями, truth-fixture 8 клипов: 640/8), I #200.
- 60 B (FL 21–24), 32 B (<21): [CONFIRMED] S, I #200.
- **88 B (FL 26, «8-байтовый хвост»): два заявления (flpkit README, форк-README), код/байты не видены → [SINGLE-SOURCE] (provisional).** Усиливающий факт: flpkit пишет, что клипы, записанные в 88 B формате, **загружены живым FL 2026 и перезаписаны им побайтно идентично** (K, «write_playlist … Live-verified»). Это измерение, а не пересказ.
- S выбирает stride **по версии FL** (≥25→80, ≥21→60, иначе 32; автоопределение 80→60→32 только если версия неизвестна). Это *противоречит* принципу detect-don't-assume; S сам предупреждает о неоднозначности на кратных 240.
- flpkit определяет stride по сигнатуре `pattern_base == 20480`.

**Дизайн детектора [INFERENCE]:** для кандидата S ∈ {32,60,80,88}: `len % S == 0` и u16 по смещению 4 каждой записи == 0x5000. Прочитанная неверным шагом запись попадает «в середину» чужой, и там (напр. float64 `1.0` = `00 00 00 00 00 00 F0 3F` в 80-B записи, смещение 64) значение ≠ 0x5000 — сигнатура ломается. Если подходят несколько S, выбирать наибольшую долю совпадений; при ничьей/пустом blob — `Ambiguous`, отказ записи.
`item_index <= pattern_base` — канал, иначе паттерн (`item_index - pattern_base`) (PyFLP discussion #83 — факт семантики; код не использован).

### 4.3 Wrapper: `0xD4` против «`0xC9` + `0xD5`»

- В промпте «`0xD4` wrapper» — **неверная атрибуция**. S: wrapper определяется по `0xC9 == "Fruity Wrapper"`, а детали VST лежат в record stream внутри `0xD5`.
- E (раздел Synthesizing): в slot scope есть 6-событийный кортеж `0xC9` (internal name), **`0xD4` (runtime data)**, `0x9B`/`0x80`/`0x29` (метаданные), `0xD5` (state). Все шесть нужно переносить вместе.
- flpkit: «plugin tuples are captured and spliced as opaque data», в записи `chunk_hex`/`sha256`. [INFERENCE]: «tuple» = именно этот набор, поэтому flpkit «знает» `212` (=`0xD4`), но не как wrapper, а как часть кортежа. Прочитать `formats/effects.py` для подтверждения — пока не сделано.
- E сообщает, что синтез кортежа из донорского FLP **не всегда даёт рабочий плагин в runtime API** (`getPluginName` → «Plugin not valid», `setParamValue` не биндится), хотя UI плагин видит. [CONFIRMED] как наблюдение flpdiff; причина (проектный metadata blob?) неизвестна. Для нас: `add_effect` по шаблону — только на уровне «splice кортежа, захваченного из FL-файла», с честным статусом partial.
- Реальные имя/vendor/path/FourCC VST декодируются из `0xD5` (record ids 54/56/55/51); state (id 53) opaque. Совпадает с S.

### 4.4 Где живут legacy Levels (противоречие в S)

S пишет: «pre-FL-25 saves emit Levels at `0xCB`, but `0xCB` is taken by Name; FL 25 relocates Levels to `0xDB`».
- Арифметика релокации +16 даёт `0xCB→0xDB`, **но** `0xCB` — это TEXT-диапазон, а S/E подтверждают, что имя канала в FL 25 лежит именно в `0xCB`. Одновременно «Levels» (24 B структура) и «Name» (UTF-16 строка) в `0xCB` — сомнительно, особенно что парсер flpdiff на корпусе FL 9–24 использует `0xCB` как имя (parity 85/85 по counts/kinds).
- flpkit README: уровни читаются «across four format generations (`Levels` 219, word events, byte events)» — т.е. 0xDB (219), потом **отдельные WORD-события и BYTE-события** (старые FL хранят pan/volume/pitch как раздельные события). Четвёртое поколение не названо. `set_channel_levels` **отказывается писать в legacy-файлы**.
- **Статус [UNRESOLVED]:** (а) утверждение S о `0xCB` для pre-25 Levels, по-видимому, ошибочно или неполно [INFERENCE]; (б) реальные опкоды WORD/BYTE-поколений и промежуточного DATA-варианта не получены; нужен `flpkit/formats/levels.py` и pre-25 фикстуры. До этого: Levels пишем **только для `0xDB`**, на legacy — `Unsupported`.

## 5. Таблица «возможность → статус → доказательство»

| Возможность | Статус знаний | Доказательство |
|---|---|---|
| L1 Распознавание файла, версия, безопасное чтение | Ready to implement | S §1–2, §4; 5 фикстур FL25 |
| L2 Tempo/PPQ/patterns/notes | Ready | `0x9C`, `0x42/0x5D`, `0xE0`; K live-verified для FL 2026 |
| L3 Channels (имена, типы, levels, цвет) | Ready для FL25; legacy levels — unresolved | §4.4 |
| L4 Playlist/audio clips | Partial: 32/60/80 подтверждены, 88 single-source | §4.2 |
| L5 Automation | Read: K (точки, delta-encoded f64, сверка с PyFLP на 1100 blob'ах [SINGLE-SOURCE: K]); create — нет | K; E (`0xE3` link не полностью) |
| L6 Mixer graph | Partial: insert/slot/params читаются; routing `0xE7` — не матрица | S §3.1 |
| L7 Plugin states | Opaque; VST метаданные читаются; native — только EQ2 раскрыт | S, E |
| L8 Targeted edits | tempo/notes/levels/clips: методология доказана (K), в Rust не реализовано | K |
| L9 Round-trip saving | Byte-exact parse→write возможен при хранении payload; **не проверено нами** | S §5 («carried through as Uint8Array»), E (D-50) |
| L10 Differential + live FL | Только вручную | §8 |

## 6. Дизайн `flp_compat` (ADR)

### 6.1 Структура крейтов

- `flp_compat_core` — `Tokenizer` (потоковый, без паник на мусоре, ошибки с byte offset), `FlpDocument { header, events: Vec<Event> }`, `Event { offset, opcode, kind, payload: Bytes, dirty: bool }`. Неизвестные опкоды — opaque blobs.
- `flp_compat_model` — декодеры scope: channels (`0x40`), patterns (`0x41`, дедуп по id — срабатывает дважды), arrangements (`0x63`), mixer (`0x93`/`0x62`); `Scope`-автомат для `0xCB`.
- `flp_compat_map` — IR → внутренняя модель VibeStudio (без обратной зависимости).
- `flp_compat_report` — `ImportReport` (supported / partial / opaque / unsupported, по сущностям).
- `flp_compat_write` — surgery API (§6.3).
Внутренняя модель проекта остаётся нашей; FLP — только interchange.

### 6.2 Токенайзер: candidate-framing для `0xAC`

1. Для опкодов вне override-таблицы — диапазонное правило.
2. Для `0xAC` пробуем кадрирования A (3 B), C (varint+data), B (1 B) в этом порядке приоритета и **для каждого** проверяем lookahead на N=8 следующих событий: корректные границы, отсутствие выхода за `22+data_len`, и для первого `0xAC` — следующее событие `0xC0` с UTF-16 префиксом `"FL Studio"`.
3. Если выживает ровно одно — берём; если A и B оба проходят (структурно эквивалентны для payload `xx 01 yy`) — берём A и пишем `Ambiguity::Framing0xAC` в отчёт (без влияния на byte-exact запись); если ни одного — `Corrupted`.
4. Таблица `event_size_overrides` оверрайдится профилем версии (по образцу K), измеренная таблица на версию — первичнее эвристики.

Структурные инварианты (self-check, как `Warn`/`Error` в зависимости от строгости):
- границы: курсор == `22 + data_len` ровно; хвост после `FLdt` фиксируется отдельно;
- iid каналов из `0x40` непрерывны `0..n-1` [CONFIRMED на фикстурах FL25, S §3.1];
- на FL ≥ 20 число `0x62` кратно 10 на insert (на FL 9 их нет) [CONFIRMED только как наблюдение S];
- на базовых FL25 фикстурах: 18 событий `0x93`, 500 `0xEE` на arrangement [OBSERVED, S];
- `n_channels` в FLhd **не** используется (legacy).

### 6.3 Writer: splice vs rewrite

| Критерий | Byte splice (flpkit) | Full rewrite c byte-exact round-trip (flpdiff) |
|---|---|---|
| Неизвестные байты | сохраняются по построению | сохраняются, если IR хранит payload |
| Риск | меньше всего затронутых байт | выше: любая ошибка сериализации ломает файл (flpkit наблюдал, что PyFLP-save пишет неверный channel count и портит UTF-16, FL отказывается открывать) |
| Эволюция формата | устойчив к новым опкодам | требует знать framing всех событий (`0xAC`!) |
| Покрытие | узкое, по операции | широкое |

**Рекомендация [INFERENCE]: гибрид «splice-by-construction».** IR хранит исходные байты каждого события; writer выдаёт байты нетронутых событий дословно (по сути splice), перекодирует только `dirty` события, затем пересчитывает `FLdt.data_len`. Полная реконструкция потока из семантической модели — запрещена. Каждая операция заканчивается **verify-by-readback**: повторный парс записанных байт даёт именно то, что передавали, и все не затронутые события байт-в-байт равны исходным. Любое сомнение (неоднозначный stride, `Ambiguous` framing, legacy Levels, пустой playlist blob без шаблона записи) → `RefuseToWrite` с причиной.

API (по уровням): L1 `set_tempo` (патч `0x9C`, при отсутствии — **append в конец потока**, как принимает FL, K); `write_notes(pattern, channel, mode)` — `replace` только в рамках канала, т.к. blob паттерна содержит ноты **всех** каналов (K); флаги ноты шаблонируются из существующих (`0x4000` по замеру K на 24 435 записях [SINGLE-SOURCE]). L2: `write_playlist` (новая запись строится из байтов существующей как шаблон, сортировка по позиции), `set_channel_levels` (только `0xDB`). Всегда: работа с копией, backup, change report.
Учесть из E: несколько событий одного опкода в паттерне конкатенируются при чтении; flpdiff при записи коалесцирует в одно — FL принимает оба варианта.

### 6.4 Ошибки и строгость

Таксономия: `Format` (не FLP/битый header), `Unsupported` (известная, но не реализованная возможность / неизвестная версия), `Corrupted` (нарушены инварианты, выход за границы), `Ambiguous` (несколько допустимых разборов). Политики: `Strict` — любой `Ambiguous`/`Corrupted` останавливает импорт; `BestEffort` — продолжить, непонятное → opaque, всё в `ImportReport`. Для записи действует только `Strict`.

## 7. Порядок реализации по уровням

| Ур. | Definition of done | Обязательные тесты |
|---|---|---|
| 1 | Токенайзер + версия; 5 фикстур разобраны до `22+data_len` | границы, truncated, битый varint, desync после `0xAC`; инварианты |
| 2 | tempo/PPQ/patterns/notes → внутренняя модель | сверка с `flpdiff` CLI, notes 24 B round-trip |
| 3 | channels, levels (`0xDB`), цвета | iid-инварианты; scope `0xCB` на `base_one_insert` |
| 4 | playlist с детектором stride | синтетические blob'ы 32/60/80/88 и кратные 240/880 |
| 5 | чтение automation clips | сверка точек с эталоном; запись — не обещаем |
| 6 | mixer graph (`0x93/0x62/0xE1/0xE7`) | 10 слотов/insert, off-by-one runtime (+1) |
| 7 | plugin metadata из `0xD5`, остальное opaque | VST-запись id 54/56/55/51 |
| 8 | targeted edits §6.3 | readback, «нетронутые события равны байт-в-байт» |
| 9 | round-trip сохранение | `parse→write→parse` идентичность; golden |
| 10 | дифф-тесты + ручное открытие в FL | чек-лист §8.4 |

## 8. Стратегия тестирования

**8.1 Фикстуры.** (а) публичные FL 25.2.4 из `tests/corpus/re_base/fl25/` (5 файлов, `base_empty`, `base_one_*`); (б) собственный корпус: по одному признаку на файл (минимальные пары): пустой проект, 1 канал, 1 паттерн/1 нота, tempo sweep, 1 insert с именем/цветом, VST (Serum-like), 1 playlist-клип, 1 automation clip, time marker, legacy FL (≤21), FL 24, FL 25.2.x, **FL 26**. Каждый — сохранён «свежим» FL, версия зафиксирована в имени. (в) Golden: хэши и декодированный JSON; **замораживаются и не пересоздаются после рефакторингов** (изменение golden = отдельный PR с обоснованием).
**8.2 Oracles.** `flpdiff` CLI (MIT, коды выхода 0/1/2) — основной для современных файлов; PyFLP (GPL) — чёрный ящик, отдельный процесс, только FL ≤ 21; fork-варианты не использовать как oracle без проверки лицензии. Примирение: сравниваем нормализованный JSON (ключевые поля), расхождение между oracle'ами → `Ambiguous` + ручная проверка, не авто-выбор. Известные ограничения oracle (напр. `flp-info` всегда `routes_to: []`, `pan_law: 0` — S) вносим в список исключений.
**8.3 Round-trip и коррупция.** `parse→write→parse`; усечённый файл; битый varint; `data_len` короче/длиннее; desync после `0xAC` (подмена на 4 B); неизвестные опкоды; stride на границе кратных.
**8.4 Live FL (не автоматизируется).** Чек-лист: открыть изменённый файл без предупреждений; tempo/ноты/клипы отображаются; пересохранить в FL и сравнить (flpdiff: после ресохранения меняются ~5 байт save-counter metadata — E, D-50); не падает воспроизведение (случай неверного stride 60 вместо 80 в S: нулевые клипы и crash).
**8.5 Эксперимент для `0xAC`.** Сохранить один и тот же пустой проект в FL 25.2.x и FL 26, зафиксировать байты после `ac`. Затем менять по одной настройке проекта (в пределах доступных в UI) и смотреть, какие из трёх байт меняются — это ответит, флаги ли это и как FL 26 кадрирует событие.

## 9. Реестр рисков

| Риск | Вероятность | Митигация |
|---|---|---|
| Дрейф формата (FL 26 сломал `0xAC` и stride 88) | высокая | профили версий, candidate-framing, Strict при `Ambiguous` |
| flpdiff неактивен с 2026-07, спецификация живая только формально | средняя | форк/слепок спецификации с датой, собственные фикстуры |
| GPL-загрязнение (форки, сниппеты в поиске, «copied from PyFLP» в E) | средняя | provenance log, запрет читать код, юридическая проверка полей `0xEE/0xE0` |
| Нет live FL в CI | высокая | ручной чек-лист, релизный гейт |
| Ложные ожидания совместимости | высокая | отчёт: плагины переносятся только как метаданные; state не воспроизводит звук в чужом хосте; automation create не поддерживается |
| Закрытый корпус нельзя воспроизвести | высокая | все «confirmed from corpus» понижаем до [INFERENCE] |
| Единственные источники (88 B, `0xAC` FL26, 0x4000 flags) | средняя | пометки [SINGLE-SOURCE], замена своими фикстурами |

## 10. Первые 10 задач для кодинг-агента

1. **Токенайзер + проверка на 5 фикстурах** (ниже).
2. Профиль версии и `event_size_overrides`; candidate-framing для `0xAC` с тестами A/B/C/D.
3. Scope-автомат (`0x40/0x41/0x63/0x93/0x62`), дедуп `0x41`.
4. Декодеры tempo/PPQ/notes/patterns (+ `0x42/0x5D`).
5. Декодер channels и `0xDB` (+ отказ на legacy).
6. Детектор stride playlist + синтетические тесты.
7. `ImportReport` и политики Strict/BestEffort.
8. Writer: `set_tempo`, `write_notes`, verify-by-readback.
9. Интеграция `flpdiff` CLI как differential oracle в CI.
10. Собственный корпус + golden и ручной live-чеклист; FL 26 фикстуры для `0xAC`/88 B.

**Задача #1 — команды проверки** (синтаксис CLI flpdiff сверить по `--help`, здесь не выдумывается):
```sh
git clone https://github.com/dawhubapp/flpdiff && git -C flpdiff rev-parse HEAD   # записать commit в provenance
ls flpdiff/tests/corpus/re_base/fl25/
xxd -s 48 -l 8 flpdiff/tests/corpus/re_base/fl25/base_empty.flp
# ожидается (по E): ac01 0100 c036 4600
cargo test -p flp_compat_core
```
Критерии завершения: на всех 5 файлах курсор заканчивается ровно на `22+data_len`; первые 4 байта `FLhd`/`FLdt` разобраны; на `base_empty.flp` событие по offset 48 — `0xAC` c кадрированием A (или `Ambiguity` A/B), следующее — `0xC0` баннер `"FL Studio 25.2.4.4960.4960"`; ни одной паники на усечённых входах (fuzz/proptest на 10⁵ мутаций); при отсутствии live-FL в CI всё помечено «не проверено в FL».

## 11. Открытые вопросы

1. Кадрирование `0xAC` на FL 26 (3/varint/1) и версия FL, из которой дамп PyFLP #208.
2. Структура 88-B записи (что в 8-байтовом хвосте).
3. Реальные опкоды legacy Levels (WORD/BYTE-поколения).
4. Содержимое `0xD4` и роль `0x9B`/`0x29` в slot-кортеже; почему синтез плагина не биндится в runtime.
5. Структура `0xC0` project-properties blob; `0xEE` поля 48+; `0xE3` старшие биты dest.
6. Automation: создание нового клипа (link bytes), PAHDSR.
7. Как FL 26 кодирует `0x16`, `0xD7` — отсутствуют в доступных источниках.
