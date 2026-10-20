# flpkit (clarkipeng/flpkit) — аудит исходников

Дата проверки: 2026-10-20. Репозиторий: https://github.com/clarkipeng/flpkit
Метод: shallow clone (`--depth 200`, полный — 25 коммитов) в /tmp/oss_audit/flpkit, только чтение. Код не запускался.

## Краткая сводка (вердикт)

- MIT подтверждён чтением LICENSE («Copyright (c) 2026 Origami Research»). Копировать можно. [ПОДТВЕРЖДЕНО]
- Написан как «composition agent toolkit»: zero-dependency Python >= 3.11, **raw byte surgery** вместо ресериализации + **verify-by-readback** на каждой записи — самая безопасная из трёх архитектур записи.
- Умеет совсем немного, но то, что умеет, — верифицировано против реального FL Studio 2026 (live round-trip через родительский fl-studio-mcp) и против pyflp на 164 реальных проектах (differential-оракул, dev-only).
- Playlist-reading определяет record stride детекцией сигнатуры `pattern_base == 20480` (32/60/80/88 байт по эпохам) — это решает класс проблем, на которых сломались и PyFLP, и (частично) flpdiff.
- Главные риски: 1★, возраст < 2 месяцев на момент релиза 0.8.1, узкий скоуп (без plugin state, без mixer graph, без создания automation), жёсткая привязка к FL 2026 и, вероятно, один разработчик-продукт (AI-agent-ориентированный).

## Метаданные

| Параметр | Значение | Источник |
|---|---|---|
| Звёзды / форки | 1★ / — | gh api repos/clarkipeng/flpkit, 2026-10-20 |
| Язык | Python (>= 3.11), zero runtime deps | pyproject.toml |
| Лицензия (метаданные) | MIT | gh api; pyproject `license = "MIT"` |
| Создан / последний push | 2026-08-29 / 2026-09-07 | gh api |
| Коммиты | 25 (git rev-list), 2026-08 → 2026-09 | git log |
| Открытые issues | 0 | gh api |
| Releases / tags | GitHub Releases: 0; теги v0.6.2…v0.8.1 | gh api |
| PyPI | **flpkit 0.8.1**, автор «Origami Research», загружен 2026-09-06 | pypi.org/pypi/flpkit/json |
| Размер | ~190 KB | задача пользователя |

## Лицензия и риски

- `LICENSE`: стандартный MIT, «Copyright (c) 2026 Origami Research»; pyproject подтверждает. [ПОДТВЕРЖДЕНО]
- Копирование кода/идей в закрытый коммерческий продукт VibeStudio — разрешено (с сохранением notice). Использование как оракула — свободно. [ПОДТВЕРЖДЕНО по тексту лицензии]
- Важно: собственный dev-оракул flpkit — **GPL-шный pyflp** («Differential reading against pyflp across 164 FL-authored projects (dev-only oracle; flpkit ships with zero dependencies and no GPL code)», README). Это лицензионно корректная схема: GPL-код не попадает в дистрибутив, pyflp используется как внешний чёрный ящик в dev-окружении. Тот же паттерн подходит VibeStudio. [ПОДТВЕРЖДЕНО, README «How it was verified»]
- README прямо обвиняет pyflp: «its serializer … producing files that real FL Studio refuses to open» — подтверждается нашим чтением `pyflp.save()` (полный ресериализм, пишет `num_channels` в заголовок). Независимая оценка, не проверено на живом FL. [ПОДТВЕРЖДЕНО как заявление + код pyflp; эффект [ПРЕДПОЛОЖЕНИЕ]]

## Активность и здоровье

- 25 коммитов за ~2 недели (v0.1.0 «read/write FL Studio .flp files, dependency-free» → v0.8.1). История аккуратная, PR-поток (#3,#4,#6..#12). [ПОДТВЕРЖДЕНО]
- CI: только `.github/workflows/publish.yml` (uv build + trusted publishing на PyPI по тегу `v*`; environment pypi). **Обычного test-CI в репо нет** — тесты гоняются локально/перед PR. [ПОДТВЕРЖДЕНО]
- Issues: 0 открытых, 0 закрытых публично — почти нет внешних пользователей. [ПОДТВЕРЖДЕНО]
- Автор: аккаунт `clarkipeng` (личный, blog: clarkipeng.github.io), в pyproject/PyPI — «Origami Research». GitHub-юзер `origami-research` существует (репо `fl-studio-agent`); README ссылается на родительский проект `github.com/origami-research/fl-studio-mcp` (аналогичный репо `flstudio-mcp` лежит и в org dawhubapp). [ПОДТВЕРЖДЕНО связь через README/PyPI; кто такие «Origami Research» за пределами GitHub — [ПРЕДПОЛОЖЕНИЕ]: маленькая AI-инструментальная студия/инди]
- TASTE.md/AGENTS.md показывают AI-agent-центричную разработку («founder taste», правила для агентов) — стиль solo+AI, как у VibeStudio. [ПОДТВЕРЖДЕНО]
- Single-maintainer риск: максимальный из трёх проектов (1★, приватный вкус разработки, скоуп привязан к их fl-studio-mcp агенту).

## Сборка и зависимости

- `[project] dependencies = []` — рантайм-зависимостей нет вообще; стандартная библиотека (struct, dataclasses, hashlib, importlib.resources). Dev-группа: pytest>=8, ruff>=0.6; сборка hatchling; uv.lock. [ПОДТВЕРЖДЕНО]
- `requires-python = ">=3.11"`; классификаторы до 3.14; py.typed. OS Independent. [ПОДТВЕРЖДЕНО]
- Для VibeStudio это значит: портировать на Rust легко, внешних зависимостей тянуть не придётся.

## Архитектура (дерево ключевых исходников)

```
src/flpkit/
  __init__.py        (512 стр.) read(), FlpProject/FlpChannel/FlpNote,
                     публичные писатели: set_tempo, write_notes, write_playlist,
                     write_automation, set_channel_levels, add_effect, effects_at
  codec.py           (317) ЕДИНЫЙ движок read/patch: locate -> encode -> splice ->
                     _bump_fldt_length -> re-read -> verify. SpliceSite, Target,
                     Format-протокол, Stream (TLV-итератор с size_overrides),
                     frame_event (varint), verify_identical
  detect.py          (83) detect-don't-assume: EVENT_SIZE_OVERRIDES_FALLBACK {172:1},
                     детекция playlist stride по сигнатуре
  formats/
    tempo.py (50)    locate tempo-события (modern 156 + legacy word-pair), patch
    levels.py (97)   пан/громкость/pitch внутри 24-байтового Levels (event 219)
    notes.py (219)   PatternID.Notes blob; NOTE_STRUCT "<IHHIHHBBBBBBBB" (24 B);
                     скоуп = канал внутри паттерна (merge/replace)
    playlist.py (173) аранжировка-клипы; шаблонная запись от существующей записи
    automation.py (189) delta-encoded f64 точки; заголовок/трейлер carried verbatim
    effects.py (170) captured plugin references: {name,fl_build,kind,chunk_hex,sha256}
  data/plugins/      index.json + fruity-parametric-eq-2.json (единственный
                     захваченный эффект)
tests/               test_reader (38), test_writers (59), test_corpus (3, skip без FL),
                     test_differential (goldens), conftest (FL-templates, skip без FL),
                     golden/ (21 зафиксированных .flp)
```

Как парсится FLP [ПОДТВЕРЖДЕНО, codec.py]: тот же chunk-based `FLhd`/`FLdt` + event-based TLV-поток (id <64 → 1 байт, <128 → 2, <192 → 4, иначе varint+payload), поверх — таблица `event_size_overrides` (fallback `{172: 1}`). Чтение — полная модель потоковой; **запись — только сплайс**: патчатся/вставляются ровно байты события, `FLdt.length` корректируется, всё остальное не трогается — неизвестные байты выживают **by construction** (никакой ресериализации нет в принципе).

## «Self-verifying raw-surgery writers» — как устроено

[ПОДТВЕРЖДЕНО, src/flpkit/codec.py]

1. `Format`-протокол: `locate(stream, target) -> SpliceSite` (где лежит элемент), `encode(data, ppq)` (tool-units → байты, валидирует ввод), `decode(blob, ppq)` (байты → tool-units, byte-faithful), `verify(sent, readback)`.
2. `patch()`: читает файл → locate → encode поверх «kept» (merge: всё; replace: только out-of-scope `rest`) → сплайс `raw[base+head:base+end] = event` → `_bump_fldt_length` → `path.write_bytes` → **перечитывает сохранённый файл**, снова locate+decode → `verify_identical` → при расхождении `FlpError` с именем первого дрейфа. Возвращает readback.
3. Поля, которые безопасно патчить (по README/кодy): tempo (in-place или end-of-stream append), notes (скоуп = канал в паттерне; наивная замена всего блоба стирала 51 ноту чужого канала — комментарий в formats/notes.py), channel levels (pan/volume/pitch; legacy-файлы отклоняются), playlist clips (только добавление по шаблону от существующей записи), automation points (только замена внутри существующего блоба), captured effect-ссылки (opaque, hash-валидация).
4. detect-don't-assume: флаги новой ноты шаблонируются из нот самого файла (fallback 0x4000), cut-window новых клипов наследуется от существующей записи, stride плейлиста детектируется, size-таблицы переопределяются аргументом `event_size_overrides`.
5. Что НЕ сохраняется от неизвестного: на своей модели flpkit видит только свой скоуп, но т.к. файл не ресериализуется, всё смоделированное-неправильно не может быть записано. Единственная деструктивная зона — сам сплайснутый блоб (и то merge/replace скоупятся).

Ограничение подхода: нет никаких операций, требующих изменения размеров «чужих» событий (нельзя переупорядочить поток, создать новую аранжировку/канал — создание паттерна возможно только как «пишем notes в новый паттерн-блок за якорем event 146», есть golden `notes-create-pattern.flp`). [ПОДТВЕРЖДЕНО по тестам]

## Публичный API (по src/flpkit/__init__.py, сигнатуры упрощены)

[ПОДТВЕРЖДЕНО]

```python
read(path) -> FlpProject                  # ppq, tempo, channels (names/levels/
                                          # automation), patterns -> notes, playlist
notes_at(path, pattern, channel) -> list[FlpNote]
write_notes(path, [NoteSpec(key=60, start=0, length=1, velocity=0.8, pan=0.5)],
            pattern=1, channel=0, mode="merge"|"replace")   # beats; 0..1 units
write_playlist(path, [ClipSpec(...)], arrangement=0, mode=...)  # pattern clips
write_automation(path, points, channel=...)                     # только существующий клип
set_channel_levels(path, iid, volume=0.8, pan=-0.25)            # legacy -> отказ
set_tempo(path, 128.5)                    # возвращает фактический темп после записи
add_effect(path, insert, "Fruity Parametric EQ 2")              # captured reference
effects_at(path, insert) -> list[PluginReference]
# низкоуровнево: codec.read/codec.patch(path, NotesFormat(), Target(...), data, mode,
#                                        event_size_overrides={...})
```

Единицы измерения — «tool units» (beats, 0..1) с конверсией в файловые (PPQ-тики, pan/volume int32) внутри Format.encode; проверки диапазонов и валидности mode/value живут в Formats и кидают `FlpError` (CHANGELOG 0.8.0 «Validation now lives in Formats»). [ПОДТВЕРЖДЕНО по codec.py/formats + CHANGELOG]

Карта событий codec.py (номера — десятичные): 64 New channel, 65 Pattern new, 72/73 word vol/pan, 2/3 byte vol/pan, 146 CurGroupId (якорь нового паттерна), 156 Tempo, 192 Name legacy, 201 internal name, 203 user name, 212 wrapper, 213 state, 219 Levels, 224 notes, 234 automation points, 236 InsertID.Flags (граница mixer-секции). [ПОДТВЕРЖДЕНО]

## Реальная готовность функций

| Возможность | Статус | Доказательство |
|---|---|---|
| Чтение PPQ/tempo (вкл. legacy coarse/fine) | да | README «What it reads»; formats/tempo.py |
| Чтение channels: имена (user→legacy→internal каскад), Levels 4 поколений | да | README; codec.py CHANNEL_SCOPED |
| Чтение notes per pattern/channel (24B records, implicit channel 0) | да | formats/notes.py; test_reader |
| Чтение playlist clips (stride DETECTED 32/60/80/88) | да, FL 2026 = 88 байт | detect.py; README |
| Чтение automation-точек (delta f64, tension) | да, сверено с pyflp на 1100 блобах | README; formats/automation.py |
| Чтение mixer graph / plugin state | **нет** (opaque) | README «Scope, honestly» |
| Запись tempo | да; verify readback | set_tempo; formats/tempo.py |
| Запись notes (merge/replace, channel-scoped) | да; verify readback | write_notes; goldens notes-* |
| Запись channel levels | да; legacy отклоняется | set_channel_levels; goldens levels-* |
| Запись playlist (place pattern clips) | да; FL-2026 live-verified, byte-identical на FL re-save | README; goldens playlist-*; commit 880fae7 |
| Запись automation points | частично: только replace в существующем канале; создание клипа — нет (link bytes не декодированы) | README; commit 0cdf5d8 |
| Добавление mixer-эффектов | только захваченные референсы (Parametric EQ 2) | formats/effects.py; data/plugins/ |
| Создание каналов/аранжировок/паттернов из ничего | нет (кроме notes-в-новый-паттерн) | README «Scope, honestly» |
| Diff между двумя FLP | нет (не его задача) | — |

## Тесты и fixtures

- Иерархия «rungs» (test_corpus.py docstring): unit → goldens → corpus → live FL. [ПОДТВЕРЖДЕНО]
- Goldens: 21 .flp в tests/golden/, **заморожены с hand-rolled писателей v0.5.0 до рефакторинга codec** (test_differential.py: «Do NOT regenerate the goldens from post-refactor code») — дифференциальная защита рефакторинга. [ПОДТВЕРЖДЕНО]
- Corpus-тесты читают реальные .flp из установленного FL Studio (macOS-путь, skip если FL нет): контракт automation-письменного рерайта — byte identity. [ПОДТВЕРЖДЕНО]
- Live-верификация (FL Studio 2026 macOS, открытие файлов flpkit + контрольное чтение через fl-studio-mcp) — описана в README; харнес в приватном родительском проекте, в этом репо отсутствует. [ПОДТВЕРЖДЕНО описание / харнес не виден]
- CI с тестами в репо нет — только publish. [ПОДТВЕРЖДЕНО]

## Ограничения / платформенные нюансы

- Фл-версии: фактически FL 2026 (macOS) — первичная цель; «era-opaque» приёмы (детекция stride, шаблоны) дают переносимость назад на чтение, но API writers рассчитан на современные файлы; для legacy levels/структур писатели отказываются работать. [ПОДТВЕРЖДЕНО]
- Ключевое расхождение с flpdiff и PyFLP: **event 172 (0xAC)**. flpkit: «172 is a ONE-byte event» (detect.py, 16 FL-2026 файлов, full-stream consistency). flpdiff: 3 байта («byte3», issue #1 + FL 25 фикстуры). PyFLP issue #208: 3 байта, payload `01 01 00` на FL 26.1.6. Анализ [ПРЕДПОЛОЖЕНИЕ, но сильно обоснованное]: при payload `01 01 00` чтение «1 байт» разбирает хвост как три валидных 1-байтовых события — стрим остаётся консистентным, и «full-stream consistency» не различает эти интерпретации. Для VibeStudio: считать `0xAC` **3-байтовым** (2 голоса против 1 + прямой hex-dump в #208), а методологию flpkit «stream-consistency» применять с оглядкой.
- Python-only, >= 3.11; в Rust напрямую не переиспользуется, но архитектура переносима.

## Выводы для VibeStudio

1. **Методологию — брать целиком:** verify-by-readback как контракт каждой записи (пись не возвращает успех, пока перечитанный файл не декодируется в отправленное); detect-don't-assume (stride/флаги/размеры из самого файла); template-based сплайсинг (новые записи строятся из существующих байтов файла). Это переносится в Rust-DAW на свою модель проекта почти дословно. [ПОДТВЕРЖДЕНО]
2. **Политика записи «never reserialize»** — сильно контрастирует с PyFLP (full rewrite) и подтверждена живым FL: для FLP-экспорта из VibeStudio на ранних этапах это самый безопасный путь (unknown bytes не теряются). flpdiff идёт другим путём (полный рерайт + round-trip guarantee) — оба варианта рабочие, но surgery дешевле в верификации.
3. **Код:** MIT — можно портировать на Rust (объём ~1.8K строк Python, ноль зависимостей). Практичная выборка: NOTE_STRUCT, формат Levels, схема плейлист-записей, сигнатура `pattern_base=20480`.
4. **Оракул:** сам по себе слабый оракул (мало покрывает), но его *подход* (differential против более полного парсера + live-FL round-trip) — образец для CI VibeStudio.
5. Не строить на нём FLP-чтение для DAW — покрытие слишком узкое; берите flpdiff specs + собственный парсер.

## Открытые вопросы

- Реальная длина payload события 172/0xAC (1 vs 3 байта) — требуется свой minimal-pair эксперимент на живом FL (как предлагают в detect.py через `event_size_overrides`). [ПРЕДПОЛОЖЕНИЕ]
- 88-байтовый playlist record «FL 2026» против 80 у flpdiff (FL 25.x): разные билды FL или flpkit видит дополнительное поле? Нужна сверка на одном файле. [ПРЕДПОЛОЖЕНИЕ]
- Публичной CI с тестами нет — насколько регрессионно стабилен trunk между релизами, неизвестно.
- Кто стоит за «Origami Research»; plans/support на будущее — репо выглядит как вспомогательный модуль fl-studio-mcp.
