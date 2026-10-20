# Промпт для Claude: изучение формата FLP и проектирование реализации

**Назначение:** передать Claude (или другому агенту с веб-доступом) задачу глубокого изучения проприетарного формата FL Studio (`.flp`) и проектирования его реализации в проекте VibeStudio (модуль `flp_compat`).

**Как использовать:**
1. Скопируйте блок между `PROMPT START` и `PROMPT END` целиком в новую сессию Claude (желательно с веб-доступом и доступом к файловой системе проекта).
2. Промпт самодостаточен: в него вшиты проверенные находки аудита от 2026-10-20, поэтому повторное исследование «с нуля» не требуется — требуется углубление и проектирование.
3. Детальные заметки, на которых основан промпт: `OSS_AUDIT.md` (разделы 3 и 7), `docs/research/flpdiff.md`, `docs/research/flpkit.md`, `docs/research/pyflp.md` — если агент имеет доступ к проекту, попросите их прочитать до старта.

---

## PROMPT START

Ты — инженер обратного проектирования файловых форматов и Rust-архитектор. Задача: изучить проприетарный формат проектов FL Studio (`.flp`) и спроектировать реализацию его чтения и безопасной точечной записи для DAW VibeStudio. Сейчас НЕ нужно писать финальный продакшн-код всего модуля — нужно: (1) проверить и углубить знания о формате, (2) выдать дизайн `flp_compat`-крейта и (3) план тестирования с оракулами. Проект будет реализовываться итерациями человеком + AI-агентами.

### Контекст проекта VibeStudio

Кроссплатформенная DAW (macOS/Windows/Linux) на Rust в духе FL Studio. Внутренняя модель проекта — собственная; FLP — импортно-экспортный формат, НЕ внутренняя модель. Принципы: real-time safety аудио, никакой ложной совместимости (каждая возможность имеет статус и тесты), никогда не разрушать пользовательские файлы (импорт в копию, backup, отчёт), лицензионная дисциплина. Полный бриф — в `~/Downloads/DAW_Research_and_Claude_Roadmap_Prompt.md` (если доступен), аудит экосистемы — `OSS_AUDIT.md`.

### Проверенные находки предыдущего исследования (2026-10-20) — отталкивайся от них

Следующее подтверждено чтением исходников трёх FLP-парсеров. Перепроверяй выборочно, но не начинай с нуля.

**Структура контейнера:**
- Файл = чанк `FLhd` (итого 14 байт: сигнатура `FLhd`, размер 6, поля u16: format-версия, число каналов, PPQ) + чанк `FLdt` с потоком событий.
- События — TLV-поток: байт-opcode + payload, размер кодируется диапазоном opcode: `< 0x40` → 1 байт (BYTE), `0x40–0x7F` → 2 байта (WORD), `0x80–0xBF` → 4 байта (DWORD), `>= 0xC0` → varint-длина (LEB128) + payload (TEXT/DATA-события).

**Карта известных событий (номера в hex — выверены по спеке flpdiff 2026-10-20, включая полный проход исправлений; всё равно перепроверяй то, на что опираешься):**
- Scope-модель (ядро любого walker'а): `0x40` (WORD u16) открывает новый канал — iid'ы сплошные `0..n-1`, события канала идут до следующего `0x40` или mixer-границы `0x62`. `0x41` (WORD) объявляет id паттерна и **срабатывает дважды на паттерн** — дедуплицировать по id.
- Каналы: `0x15` тип (BYTE: 0=sampler, 2=instrument/«Native», 3=layer, 4=instrument, 5=automation), `0x80` цвет (RGBA, u32), `0xC4` sample path (null-terminated UTF-16LE, отсутствует без сэмпла), `0xCB` — **scope-sensitive имя**: в scope канала (до любого `0x62`) это имя канала, после `0x62` — display name плагина слота; `0xDB` Levels (24 байта: int32 pan, uint32 volume, int32 pitch_shift, filter_mod_x/y, filter_type; дефолты pan 6400, volume 10000).
- Паттерны: `0xC1` имя (UTF-16LE), `0x96` цвет (u32, только если менял пользователь), `0xA4` длина, `0x1A` looped, `0xE0` notes (записи по 24 байта), `0xDF` controllers (12 байт).
- Playlist/аранжировка: `0xE9` clip records — **отсутствуют при пустом playlist** (не бывает блоба нулевой длины); записи 80 B (FL 25) / 60 B (FL 21–24) / 32 B (pre-21). Спека выбирает размер по версии FL (≥25→80, ≥21→60, иначе 32), потому что 80 и 60 делятся друг на друга на кратных 240; неверный выбор = misaligned-парс (нулевые клипы, краш на playback). FL25-запись: `pattern_base` всегда 20480, `_u1` всегда 120, `item_flags` 0x0040 (старший байт 0x80 = selected), `_u2` = `40 64 80 80`, float32 start/end_offset (NaN/0xFFFFFFFF — pattern-клипы, -1.0 — audio), последовательный `clip_id` в рамках аранжировки, float64 `scale` (1.0, offset 64). Также `0xEE` (track data, opaque ~70 байт), `0x63`, `0xF1`.
- Микшер: insert'ы накапливаются до `0x93` (u32 — закрывает insert И несёт его output routing; сентинелы -1 и self-index = «default»); `0xCC` имя insert'а (UTF-16LE); слоты эффектов закрываются `0x62` (u16 индекс слота — события слота K, включая имя и `0xD5` state, приходят ДО его `0x62`; всегда группами по 10 на insert, даже пустые); `0xE1` MixerParams — один блоб на проект из 12-байтовых записей с полной атрибуцией (`insertIdx = (channel_data >> 6) & 0x7F`, `slotIdx = channel_data & 0x3F`; id: 0=SlotEnabled, 1=SlotMix, 64..191=RouteVol, 192=Volume, 193=Pan, 194=StereoSeparation, 208..226=EQ gains/freqs/Qs); `0xEC` insert flags; `0x9A` audio input insert'а (те же сентинелы); `0xE7` routing-related.
- Прочее: `0x9C` tempo (BPM×1000), `0x42`+`0x5D` legacy tempo pair (FL < 3.4.0), `0xC9` внутреннее имя класса (например `"Fruity Wrapper"`); `0xAC` = **3 байта** (override `byte3` — классическое правило DWORD прочтёт 4 и съест следующий opcode; пример payload `01 01 00`, flpdiff issue #1); time markers: `0x94` позиция (u32; старший бит 0x08000000 = signature kind, младшие 27 бит = тик), `0xCD` имя, `0x21`/`0x22` числитель/знаменатель сигнатуры.
- Отсутствуют в спеке (нужен второй источник): `0xD4` (flpkit называет событие 212 «wrapper»; спека описывает wrapper как значение `0xC9` плюс поток записей `0xD5`), `0x16` routing, 88-байтовая запись FL 2026 (только flpkit), точная роль `0xC0` между версиями (FL 25: UTF-16 version banner; FL 24: per-channel names по спеке).
- Self-check'и парсера: `FLhd.n_channels` помечен в спеке как «legacy, ignored by modern parsers» — НЕ использовать. Вместо него структурные инварианты: сплошные iid от `0x40`, `0x62` группами по 10, стоп ровно на `22 + data_len`, плюс кандидат-детектор phantom-события при ложном чтении `0xAC` (см. Задачу 1).

**Версионные ловушки:**
- FL 25 релокации opcode: notes `0xD0`→`0xE0`, track `0xDE`→`0xEE`, Levels `0xCB`→`0xDB`, insert flags `0xDC`→`0xEC`.
- Playlist clip records растут по эпохам: 32 B (ранние) → 60 B (FL 21–24) → 80 B (FL 25) → 88 B (FL 2026, по данным flpkit — единственный источник, не проверено). Сама спека выбирает размер по версии FL (≥25→80, ≥21→60, иначе 32) и предупреждает, что 80/60 неоднозначны на кратных 240 — поэтому совмещай выбор по версии с сигнатурной валидацией выбранного размера (`pattern_base == 20480` у flpkit плюс константные поля из карты выше).
- Событие `0xAC` (172): размер payload спорен — flpdiff и PyFLP (issue #208, hex-dump `01 01 00`) говорят 3 байта, flpkit — 1 байт. Спека подтверждает **3 байта** (override `byte3`). Пойми, почему «full-stream consistency» flpkit не различает эти интерпретации, и спроектируй self-check парсера — структурные инварианты (сплошные iid, `0x62` группами по 10, стоп на `22 + data_len`), а не ненадёжный `FLhd.n_channels`, — который ловит desync «тихо и тотально» (формулировка issue #208).

**Plugin state (жёсткое ограничение):**
- `0xD5` — VST-wrapper: декодируются name/vendor/path/FourCC, сам state-блоб остаётся opaque. Нативные плагины FL — opaque. Восстановить звук чужого проекта в другом хосте в принципе нельзя — только ноты/аранжировка/структура. Это надо честно отражать в compatibility report.

**Automation — самое большое белое пятно:**
- Полные envelope-модели (PAHDSR сэмплеров, automation clips) в open source отсутствуют (PyFLP issue #29); flpdiff объявил write-автоматизацию non-goal; flpkit умеет только заменять точки в существующем клипе (delta-encoded f64, заголовок/трейлер carried verbatim).

**Три существующих инструмента и лицензионные правила:**
- [flpdiff](https://github.com/dawhubapp/flpdiff) — **MIT**. Главная ценность: clean-room спецификация [`docs/fl-format/flp-format-spec.md`](https://github.com/dawhubapp/flpdiff/blob/main/docs/fl-format/flp-format-spec.md) (517 строк, уровни доверия opcode'ов, parity-отчёты), плюс [`fl25-event-format.md`](https://github.com/dawhubapp/flpdiff/blob/main/docs/fl-format/fl25-event-format.md), [`plugin-layout-registry.md`](https://github.com/dawhubapp/flpdiff/blob/main/docs/fl-format/plugin-layout-registry.md), [`fst-format.md`](https://github.com/dawhubapp/flpdiff/blob/main/docs/fl-format/fst-format.md). CLI с exit-кодами 0/1/2 — готовый CI-оракул; есть 5 публичных фикстур FL 25.2.4 (`tests/corpus/re_base/fl25/`). Проект спит с 07.2026. ВНИМАНИЕ: та же организация держит GPL-форк PyFLP — при копировании любых файлов проверяй LICENSE каждого файла.
- [flpkit](https://github.com/clarkipeng/flpkit) — **MIT**, zero-dependency Python. Главная ценность: методология записи — `locate → encode → splice → _bump_fldt_length → перечитать файл → verify_identical`; «detect-don't-assume» (stride/флаги/размеры берутся из самого файла); template-splicing (новые записи строятся из байтов существующих). Ключевые файлы: `src/flpkit/codec.py`, `detect.py`, `formats/*.py`. Код можно читать и портировать (MIT, ~1.8K строк).
- [PyFLP](https://github.com/demberto/PyFLP) — **GPL-3.0**. Код НЕ читать при написании своей реализации (чистая комната) и НЕ копировать никогда. Допустимо: использовать как чёрный ящик-оракул в CI (отдельный процесс) для legacy-файлов FL ≤ 21; читать issue-трекер (факты формата, не код). Заморожен с 2023, FL 2024+ не читает (issues #208, #205, #200).
- Полный разбор трёх инструментов: `docs/research/flpdiff.md`, `docs/research/flpkit.md`, `docs/research/pyflp.md` (если есть доступ к проекту).

**Методология, подтверждённая практикой (переносить в Rust-дизайн):**
1. **Never reserialize** — только сплайсинг байтов; неизвестные байты выживают by construction. Полный ресериализм (путь PyFLP `save()`) — документированный анти-паттерн: реальный FL такие файлы не открывает.
2. **Verify-by-readback** — операция записи не считается успешной, пока перечитанный файл не декодируется в отправленные данные (контракт flpkit `patch()`).
3. **Detect-don't-assume** — размеры/stride/флаги определяются из самого файла, fallback-таблицы переопределяемы.
4. **Goldens замораживаются** и не регенерируются после рефакторинга (дифференциальная защита).
5. **Уровни поддержки** (из брифа VibeStudio): 1) распознавание/версия/безопасное чтение → 2) tempo/PPQ/patterns/notes → 3) каналы → 4) playlist/аудиоклипы → 5) automation → 6) mixer graph → 7) plugin states → 8) точечное изменение → 9) round-trip сохранение → 10) дифференциальные тесты + открытие в живой FL Studio.

### Задача 1 — углубить знание формата

- Прочитай спеку flpdiff полностью; построй сводную таблицу opcode'ов: номер | смысл | размер/тип | версии FL | уровень доверия | источник (файл+строка). Сверь расхождения между flpdiff/flpkit/PyFLP-issue.
- Разберись в спорных местах: `0xAC` (3 vs 1 байт — объясни механику ложной консистентности), 88-байтовая playlist-запись FL 2026 (что за дополнительные поля vs 80 B FL 25), состав `0xC0` project-properties blob, `0xEE` track data, `0xD5` нативных плагинов, PAHDSR-структуры automation, музыкальные тайминги (PyFLP #75).
- Спроектируй **протокол чистой комнаты**: какие источники можно читать (MIT-спека, flpkit-код, issue-трекеры), какие нет (GPL-код PyFLP), и как это фиксировать (журнал provenance).
- Если есть доступ к живой FL Studio: предложи minimal-pair эксперимент для `0xAC` и методику сбора собственного корпуса (свежие сохранения разных версий + целевые фичи в каждом файле).

### Задача 2 — спроектировать `flp_compat` (Rust)

Выдай ADR-уровень дизайна, не финальный код:
- Крейты/модули: TLV-ридер (streaming, без panics на битых данных, byte-offset в ошибках), IR (`FlpDocument` — события с оригинальными payload-байтами, неизвестные opcode'ы как opaque), маппинг IR → внутренняя модель проекта, compatibility report (поддержано/частично/opaque/нет).
- Порядок реализации по уровням 1–10; для каждого уровня — критерий готовности и тесты.
- Writer: surgery-API (операции уровня 1: set_tempo, write_notes; уровень 2: playlist clips, channel levels), контракт verify-by-readback, отказ вместо записи при сомнении. Сравни две стратегии записи — сплайсинг (flpkit) vs полный рерайт с byte-exact round-trip (flpdiff) — и обоснуй выбор для VibeStudio.
- Ошибки: типы (Format/Unsupported/Corrupted/Ambiguous), политики (строгая vs best-effort), отчёт импорта для UI.

### Задача 3 — тестовая стратегия

- Корпус: собственные фикстуры (что положить в каждый файл), публичные фикстуры flpdiff, golden-файлы и правила их заморозки.
- Оракулы в CI: `flpdiff` CLI (MIT, standalone-бинарники) как дифференциальный оракул на современных файлах; PyFLP (GPL, чёрный ящик, отдельный процесс, только legacy FL ≤ 21); согласуй, как сравнивать выводы разных оракулов.
- Round-trip тесты: `parse → write → parse` идентичность; corruption-тесты (усечённый файл, битый varint, desync после `0xAC`).
- Критерий истины: открытие изменённого файла в живой FL Studio — опиши, как встроить это в процесс (ручной чеклист/скрипт), учитывая, что автоматизировать нельзя.

### Задача 4 — риски и первые шаги

- Risk register для FLP-направления: дрейф формата, спячка flpdiff, GPL-загрязнение, отсутствие живой FL в CI, ложные ожидания совместимости.
- Первые 10 задач для coding-агента в правильном порядке; для первой — точные команды проверки и критерии завершения.

### Правила достоверности

1. Каждый факт формата — со ссылкой (файл+строка в репо или URL); помечай [ПОДТВЕРЖДЕНО]/[ВЫВОД]/[ГИПОТЕЗА] и дату проверки.
2. GPL-код PyFLP не открывай при проектировании; форматные факты — только из MIT-спеки, flpkit-кода и issue-обсуждений.
3. Не обещай совместимость: результат формулируй как таблицу «возможность → статус → доказательство».
4. Если что-то проверить не удалось (нет живой FL, корпус закрыт) — честно перечисли в «открытых вопросах».
5. Пиши по-русски, технические термины на английском. Итоговый документ: `docs/FLP_COMPATIBILITY.md` (проектная) + при необходимости `docs/research/flp-format-notes.md` (заметки исследования).

## PROMPT END

---

### Приложение: быстрые ссылки

| Что | Ссылка |
|---|---|
| Спецификация формата (MIT) | https://github.com/dawhubapp/flpdiff/blob/main/docs/fl-format/flp-format-spec.md |
| FL25-формат событий | https://github.com/dawhubapp/flpdiff/blob/main/docs/fl-format/fl25-event-format.md |
| Реестр layout'ов нативных плагинов | https://github.com/dawhubapp/flpdiff/blob/main/docs/fl-format/plugin-layout-registry.md |
| Формат .fst-пресетов | https://github.com/dawhubapp/flpdiff/blob/main/docs/fl-format/fst-format.md |
| Публичные фикстуры FL 25 | https://github.com/dawhubapp/flpdiff/tree/main/tests/corpus/re_base/fl25 |
| flpkit (методология writer'а) | https://github.com/clarkipeng/flpkit — ключевые файлы: `src/flpkit/codec.py`, `detect.py`, `formats/` |
| PyFLP issues (карта белых пятен, GPL — только чтение issues) | https://github.com/demberto/PyFLP/issues — №208 (`0xAC`), №205/#200 (playlist 80 B), №29 (automation), №75 (тайминги) |
| Официальный мануал Image-Line по FLP | https://www.image-line.com/fl-studio-learning/fl-studio-online-manual/html/fformats_open_flp.htm |
| Аудит экосистемы | `OSS_AUDIT.md` (разделы 3, 7, 8) |
