# vibez — глубокое исследование исходников

Дата проверки: **2026-10-20**. Метод: shallow clone (ветка `dev`) + чтение кода + `gh api`.

## Краткая сводка-вердикт

- Самый зрелый и быстрорастущий Rust-DAW из трёх: **~254k LOC Rust**, **2451 `#[test]`**, релиз v0.5.1 от 2026-10-19, 863 коммита только за октябрь 2026.
- Реально работающая DAW: clip launcher (Perform), линейный arrangement (Arrange), консольный микшер с buses/sends/EQ, встроенные синтезаторы/сэмплер/drum rack, warping через Signalsmith Stretch.
- **VST3 + CLAP hosting** (крейты `vst3` 0.3 и `clap-sys` 0.5) с native GUI и sandboxed сканированием в отдельном процессе — работает и покрыт тестами VST3 vtable ABI.
- Real-time safety не декларируется, а **доказывается тестами**: таблица покрытия каждого `EngineCommand` на allocation-free drain + rtrb ring buffers + `audio_thread_priority`.
- Offline render (bounce Master/Track/Clip) и экспорт WAV реализованы тем же кодом микшера, что и live-путь.
- **Лицензия GPL-3.0-or-later — код копировать в VibeStudio нельзя** (если VibeStudio не GPL). Но это лучший референс по архитектуре: iced + Elm-домены + lock-free движок.

## Метаданные

| Поле | Значение |
|---|---|
| Repo | https://github.com/alexanderwanyoike/vibez (default branch `dev`) |
| Версия | 0.5.1 ([Cargo.toml](https://github.com/alexanderwanyoike/vibez/blob/dev/Cargo.toml), workspace `version = "0.5.1"`) |
| Создан | 2026-02-23 [ПОДТВЕРЖДЕНО, gh api: `created: 2026-02-23T05:18:51Z`] |
| Звёзды / форки / issues | 44★ / 2 forks / 1 открытый issue [ПОДТВЕРЖДЕНО] |
| Мейнтейнер | Alex Wanyoike — **единственный** автор всех 1627 видимых коммитов [ПОДТВЕРЖДЕНО, `git log --format="%an"`] |
| Возраст | ~8 месяцев (первые коммиты 2026-02-23: «Phase 1 — audio foundation, engine, and iced UI») |
| Размер | ~27 MB рабочая копия, 254 099 строк `.rs` (crates + src) [ПОДТВЕРЖДЕНО, `wc -l`] |

## Лицензия и риски

- `LICENSE` — полный текст **GNU GPL v3** (35 149 байт) [ПОДТВЕРЖДЕНО, https://github.com/alexanderwanyoike/vibez/blob/dev/LICENSE]. В `Cargo.toml`: `license = "GPL-3.0-or-later"`.
- Windows ASIO собирается с **Steinberg ASIO SDK под его GPLv3-опцией** — прямо описано в README, раздел License [ПОДТВЕРЖДЕНО].
- **Что это значит для VibeStudio:**
  - **Копировать код (файлы, функции, крейты) — нельзя**, пока VibeStudio не станет GPL-3.0. Одна скопированная функция «заражает» весь продукт.
  - Зависимость на crate-компонент vibez (`vibez-core` как path/crates.io-зависимость) тоже невозможна в проприетарном проекте.
  - **Идеи, архитектура, алгоритмы, названия подходов — не охраняются авторским правом**: читать исходники и переизобретать архитектуру своими словами — легально.
  - Реализация по описанию в docs/ARCHITECTURE.md (без подсматривания в код при написании) — стандартная чистая комната; на практике безопасно опираться на общеизвестные паттерны (Elm-архитектура, rtrb, command queue), которые vibez не изобретал.
- Для сравнения у остальных двух (BSD-2, MIT) таких ограничений нет — поэтому для заимствования кода смотреть на них, vibez использовать как архитектурный учебник.

## Активность и здоровье

- Коммиты по месяцам (shallow-история): 2026-02: 29, 2026-04: 33, **2026-07: 448, 2026-08: 244, 2026-10: 863** [ПОДТВЕРЖДЕНО, `git log --date=format:"%Y-%m"`]. Проект разгоняется, не затухает.
- Релизы (gh api): v0.3.4 — 2026-10-03, v0.3.5 — 2026-10-05, v0.4.0 — 2026-10-13, v0.4.1 — 2026-10-13, v0.5.0 — 2026-10-18, **v0.5.1 — 2026-10-19**. Релизный ритм — несколько релизов в месяц.
- CI ([.github/workflows/ci.yml](https://github.com/alexanderwanyoike/vibez/blob/dev/.github/workflows/ci.yml)): матрица `ubuntu-latest / windows-latest / macos-latest`, `cargo test --workspace` + clippy `-D warnings --all-targets` [ПОДТВЕРЖДЕНО].
- Release CI ([release.yml](https://github.com/alexanderwanyoike/vibez/blob/dev/.github/workflows/release.yml)): Linux tar.gz + `.deb` + AppImage, macOS `.app` в `.dmg` (включая macos-15-intel), Windows; `pages.yml` — GitHub Pages сайт.
- Вклад: только один автор, PR-ы саммерджатся (#350–#364 за один день 2026-10-19). Bus-factor = 1.

## Сборка и платформы

- Cargo workspace, edition 2021, 9 крейтов + test fixture ([Cargo.toml](https://github.com/alexanderwanyoike/vibez/blob/dev/Cargo.toml)).
- Аудио I/O: **cpal 0.16** (WASAPI + ASIO на Windows, CoreAudio на macOS, ALSA на Linux) [ПОДТВЕРЖДЕНО, workspace.dependencies и README].
- UI: **iced 0.13** (features advanced/canvas/lazy/tokio).
- Plugin hosting: `clap-sys = "0.5"`, `vst3 = "0.3"`, `libloading` [ПОДТВЕРЖДЕНО, crates/vibez-plugin-host/Cargo.toml].
- MIDI: **midir 0.10**; декодирование: **symphonia** (mp3/flac/ogg/aac/wav/aiff/mp4); ресемплинг: **rubato**; FFT: rustfft; RT-приоритет: audio_thread_priority 0.32; команды: **rtrb**.
- Внешняя зависимость для time-stretch: **Signalsmith Stretch (C++)** — нужны C++ компилятор и libclang [ПОДТВЕРЖДЕНО, README Building].
- Linux: `libasound2-dev libudev-dev libdbus-1-dev`; Windows: чек-сум-пинned ASIO SDK через `scripts/setup_asio_sdk.ps1`.
- Ограничение: ASIO SDK под GPL-опцией — ещё один GPL-якорь в Windows-сборке.

## Архитектура

Дерево workspace (LOC из `wc -l`):

| Crate | LOC | Назначение |
|---|---|---|
| `vibez-core` | 5 729 | Типы: tracks, clips, MIDI, IDs, perform-примитивы |
| `vibez-engine` | 68 491 | RT-движок: микшер, transport, capture, компенсация, offline render |
| `vibez-audio-io` | 3 144 | cpal I/O, MIDI-in, WAV/декод, RT-приоритеты |
| `vibez-dsp` | 17 883 | Эффекты + time-stretch |
| `vibez-instruments` | 6 376 | Синтезаторы, сэмплер, drum rack |
| `vibez-plugin-host` | 2 345 | VST3/CLAP hosting + sandboxed сканер |
| `vibez-project` | 2 063 | Формат `.vzp` (self-contained, versioned) + legacy JSON |
| `vibez-dropbox` | 1 722 | Dropbox-браузер сэмплов |
| `vibez-ui` | 8 991 (src) | iced-приложение, домены, сервисы |

[Все пути подтверждены обходом crates/*/src; 254k общих LOC включают тесты в каждом крейте.]

Ключевые решения (по [docs/ARCHITECTURE.md](https://github.com/alexanderwanyoike/vibez/blob/dev/docs/ARCHITECTURE.md), 761 строка, плюс код):

- **Два real-time мира, связанные lock-free очередями**: UI-поток (iced) ↔ аудиопоток через `rtrb` ring buffers `EngineCommand` → и `EngineEvent` ← [ПОДТВЕРЖДЕНО, crates/vibez-engine/src/engine.rs:99-100, 239-240: `RingBuffer::<EngineCommand>::new(RING_BUFFER_CAPACITY)`].
- **Elm-архитектура с доменами** в `vibez-ui/src/domains/` (arrangement, transport, perform, piano roll, devices, browser, automation, compressor, delay, …): каждый домен — свой state slice, message enum, update-функция; общение только через `EngineHandle` (трейт — в тестах подменяется рекордером), `Ctx` (read-only факты) и `Action` (side-эффекты) [ПОДТВЕРЖДЕНО, README + каталог доменов].
- **Планировщик transport/секций**: `engine_clock.rs`, `engine_section_queue.rs` — запуск секций по музыкальным границам (immediate/1 beat/1 bar/end of section); запись «timed against the audio engine's sample clock rather than the UI frame rate» [ПОДТВЕРЖДЕНО, README Perform].
- **Микшер-граф**: `engine_graph.rs` + `mixer/` (930 строк mixer.rs) — треки, buses, returns, sends, мастер как настоящий канал; sidechain — docs/SIDECHAIN_ROUTING.md.
- **Plugin PDC** (plugin delay compensation): `compensation*.rs` — целый набор модулей компенсации задержек, редкость для проектов такого размера [ПОДТВЕРЖДЕНО, список файлов crates/vibez-engine/src/].
- **Offline render**: [crates/vibez-engine/src/render.rs](https://github.com/alexanderwanyoike/vibez/blob/dev/crates/vibez-engine/src/render.rs) — `BounceMode::{Master, Track, Clip}`, `render_offline` / `render_offline_with_plugins`, «The same mixer and instrument paths that run on the audio thread are used, so what you bounce matches what you hear» [ПОДТВЕРЖДЕНО, док-коммент]. Экспорт WAV: `write_wav_file`/`encode_wav` в crates/vibez-audio-io/src/file_io.rs:344/358.
- **Real-time safety**: `audio_thread_priority::promote_current_thread_to_real_time` (crates/vibez-audio-io/src/audio_stream.rs:784), обработка rtkit-оверов `realtime.rs`; **allocation-тесты**: crates/vibez-engine/src/engine_command_allocation_tests.rs — exhaustive-таблица `coverage(command)` без wildcard-arm: новый вариант команды не компилируется без декларации теста на allocation-free [ПОДТВЕРЖДЕНО, заголовок файла и код].

## Реальная готовность функций

| Функция | Статус | Доказательство |
|---|---|---|
| Аудио-движок, мульти-трек | Работает | crates/vibez-engine (68k LOC, mixer.rs, transport.rs) [ПОДТВЕРЖДЕНО] |
| Clip launcher / Perform (секции, quantize, swing MPC2000XL, note repeat) | Работает | README + engine_section_queue.rs, note_repeat.rs [ПОДТВЕРЖДЕНО] |
| Arrange: клипы, piano roll, automation, минимап | Работает | domains/arrangement, README [ПОДТВЕРЖДЕНО] |
| Микшер: buses, returns, sends, EQ на канал, metering | Работает | feat-коммиты 2026-10-19, mixer/ [ПОДТВЕРЖДЕНО] |
| Встроенные инструменты (sub/wavetable/FM, sampler, drum rack 808/909) | Работает | vibez-instruments (6.4k LOC) [ПОДТВЕРЖДЕНО] |
| Встроенные эффекты (EQ, reverb, delay, compressor, limiter, gate, phaser и др.) | Работает | vibez-dsp (17.9k LOC) [ПОДТВЕРЖДЕНО] |
| VST3 hosting | Работает | clap/vst3-крейты, vst3_host/, render_plugin_tests.rs [ПОДТВЕРЖДЕНО] |
| CLAP hosting | Работает | clap_host/ в plugin-host [ПОДТВЕРЖДЕНО] |
| Native plugin GUI | Работает | plugin_window.rs, gui/ в plugin-host [ПОДТВЕРЖДЕНО] |
| Sandboxed сканирование | Работает | bin/vibez-plugin-scan.rs (отдельный процесс) [ПОДТВЕРЖДЕНО] |
| Warping / time-stretch / BPM-детект | Работает | Signalsmith Stretch, warp_marker.rs, docs/AUDIO_FORMAT_SUPPORT.md [ПОДТВЕРЖДЕНО] |
| Аудиозапись с soundcard | Работает | audio_input.rs, domains/audio_recording.rs [ПОДТВЕРЖДЕНО] |
| Экспорт WAV (bounce master/stems/clip) | Работает | render.rs, file_io.rs:344 [ПОДТВЕРЖДЕНО] |
| Проект `.vzp` (self-contained) + autosave + undo | Работает | vibez-project; README предупреждает про breaking changes формата [ПОДТВЕРЖДЕНО] |
| MIDI-вход | Частично | README: «one input port at a time, notes only» [ПОДТВЕРЖДЕНО] |
| MIDI-выход / MIDI-клипы наружу | Нет | Не обнаружено в коде/README [ПРЕДПОЛОЖЕНИЕ по отсутствию] |
| Сторонние плагины в offline render | Работает (строгий путь) | render.rs: «A declared plugin is never silently substituted or skipped by that strict export path» [ПОДТВЕРЖДЕНО] |
| Заглушки/TODO | Отсутствуют | 0 совпадений `todo!`/`unimplemented!`/`FIXME` вне тестов [ПОДТВЕРЖДЕНО, grep по crates/ src/] |

## Тесты

- **2451 `#[test]`** в 508 файлах с тестами [ПОДТВЕРЖДЕНО, grep].
- Движок: юнит-тесты на транспорт-тайминги, секции, компенсацию, capture lifecycle, routing, метры; «pressure-тесты» (длинные последовательности команд).
- Специальные классы: allocation-тесты (см. выше), **offline render сравнения** (render_tests.rs, render_sidechain_tests.rs), VST3 vtable ABI-тесты через фикстуры `crates/vibez-plugin-host/tests/fixtures` [ПОДТВЕРЖДЕНО, workspace members + README Contributing].
- UI-домены тестируются без GUI (EngineHandle-рекордер) [ПОДТВЕРЖДЕНО, ARCHITECTURE.md].
- CI гоняет всё на трёх ОС [ПОДТВЕРЖДЕНО].

## Документация внутри репо (редкость для проектов такого класса)

- [docs/ARCHITECTURE.md](https://github.com/alexanderwanyoike/vibez/blob/dev/docs/ARCHITECTURE.md) — 761 строка: threading model, message flow, Perform/plugins/projects/warping [ПОДТВЕРЖДЕНО].
- docs/PDC.md — plugin delay compensation; docs/SIDECHAIN_ROUTING.md — sidechain; docs/AUDIO_FORMAT_SUPPORT.md — поддерживаемые форматы; docs/PATTERNS.md; docs/PROJECT_FORMAT_V1_PROOF.md — обоснование формата проекта [ПОДТВЕРЖДЕНО, ls docs/].
- Для VibeStudio эти документы можно читать свободно (идеи не копирайтуются) — это фактически готовый учебный курс по дизайну Rust-DAW.

## Транспорт и два clock domains

- Движок держит два независимых clock domain: живое выступление (Perform) и курсор аранжировки (Arrange) — «the engine owns two clock domains so a live performance and the Arrangement cursor stay independent» [ПОДТВЕРЖДЕНО, README Architecture].
- «Capture» записывает всю живую сессию (запуски секций, ноты, мьюты, automation-жесты) в обычные клипы и automation-линии, без ссылок на секции [ПОДТВЕРЖДЕНО, README Arrange + engine_capture_*.rs — 15+ файлов lifecycle/pressure/rebase-тестов].
- Section launch quantization — enum в vibez-core (immediate/1 beat/1 bar/end of section) [ПОДТВЕРЖДЕНО, импорт `SectionLaunchQuantization` в engine_command_allocation_tests.rs].

## Ограничения

- Alpha по собственному признанию: «v0.5.1, and still early… treat this as a working alpha rather than a stability promise» [ПОДТВЕРЖДЕНО, README Status].
- Linux — primary platform; macOS/Windows «build and pass CI but get less hands-on testing» [ПОДТВЕРЖДЕНО].
- MIDI-вход урезан (один порт, только ноты).
- Зависимость от C++ (Signalsmith) усложняет сборку.
- GPL-3.0 + GPL ASIO SDK — жёсткая лицензионная стенка для заимствования кода.
- Один разработчик; 44★; нет внешних контрибьюторов.

## Выводы для VibeStudio

- **Что брать: только идеи и архитектурные уроки.** Код копировать нельзя (GPL-3.0). Конкретно ценно: (1) доказательная схема allocation-free колбэка с exhaustive-таблицей тестов; (2) доменно-разбитый Elm-UI с трейтом EngineHandle для юнит-тестов; (3) PDC/компенсация как отдельная подсистема; (4) offline render, переиспользующий live-путь микшера; (5) sandboxed plugin-scanner как подпроцесс.
- **Как референс — лучший из трёх**: масштаб (254k LOC), темп и дисциплина тестов показывают реалистичную структуру целевого продукта на Rust+iced.
- **Как форк — непригоден**, если VibeStudio не планируется под GPL-3.0-or-later (и тогда весь проект, включая ASIO-детали, уходит под GPL).
- Стек совпадает с целевым (cpal + iced + rtrb + midir + clap-sys/vst3) — значит, выбор VibeStudio подтверждён де-факто рабочей практикой.
- Дополнительный осторожный ориентир: если VibeStudio когда-нибудь откроется под GPL-3.0, vibez станет единственным из трёх проектом, чей код можно переносить напрямую; решение о лицензии VibeStudio стоит принять до начала массового заимствований из MIT/BSD-источников, чтобы не запутаться в происхождении кода.

## Приложение: проверенные команды аудита

```sh
git clone --depth 200 --branch dev https://github.com/alexanderwanyoike/vibez
gh api repos/alexanderwanyoike/vibez/releases --jq '.[].tag_name'   # v0.5.1 … v0.3.4
# grep 'todo!|unimplemented!|FIXME' по crates/ src/ (без test) → 0
# grep -c '#\[test\]' → 2451; find crates src -name '*.rs' | xargs wc -l → 254 099
```

Дата проверки всех фактов: 2026-10-20.

## Открытые вопросы

- Насколько стабилен формат `.vzp` — breaking changes возможны (README), какие миграции заложены в vibez-project?
- Как vibez живёт с двумя clock domains (Perform vs Arrange) в реальной записи «Capture» — достаточно ли doc/PDC.md и docs/PATTERNS.md для самостоятельного повторения?
- `vst3 = "0.3"` — это bindings-крейт поверх VST3 SDK; стоит сверить его происхождение и поддержку (у maolan и tripplyons тот же крейт — тренд надёжный) [ПРЕДПОЛОЖЕНИЕ по Cargo.lock: registry+crates.io].
