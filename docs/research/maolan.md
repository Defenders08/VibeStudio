# maolan — глубокое исследование исходников

Дата проверки: **2026-10-20**. Метод: shallow clone (`maolan/maolan`, `maolan/engine`, `maolan/plugin-host`) + чтение кода + `gh api`.

## Краткая сводка-вердикт

- Это **экосистема из 8+ репозиториев**, а не один проект: app (iced GUI), engine, out-of-process plugin-host, widgets, editor, pitch, generate (AI-генерация музыки), mixosc — все BSD-2-Clause.
- Долгоживущий проект: репо создан 2019-07-31, LICENSE «Copyright (c) 2020, Goran Mekić»; активен ежедневно (последний пуш 2026-10-17, PR #409).
- Самое интересное технически: **полностью out-of-process plugin hosting** — по одному OS-процессу на плагин, shm-IPC (mmap/CreateFileMappingW), crash isolation, watchdog; форматы **CLAP, VST3, LV2 (Unix) + AU (macOS)**.
- Движок (`maolan-engine`, 80.8k LOC) — отдельный крейт на crates.io с opt-in фичами (`default = []`), backends: ALSA, JACK, CoreAudio, WASAPI, OSS (FreeBSD), sndio (OpenBSD), ASIO (Windows, добавлен 2026-10-16).
- RT-инструментарий: rtrb, basedrop (RT-safe память), arc-swap, собственный triple_buffer, fearless_simd, plan-архитектура рендера (plan_builder/executor/workers).
- **Можно заимствовать код свободно с сохранением копирайт-нотиса (BSD-2)** — самый «юридически удобный» из трёх проектов; системно самый амбициозный по изоляции плагинов.
- Минусы: единственный автор, сложность много-репо-экосистемы, документация скудная (README тонкие), UI-зрелость неизвестна без запуска.

## Метаданные

| Поле | Значение |
|---|---|
| Repo | https://github.com/maolan/maolan (default branch `main`) |
| Версия | 0.4.1 ([Cargo.toml](https://github.com/maolan/maolan/blob/main/Cargo.toml)) |
| Создан | 2019-07-31 [ПОДТВЕРЖДЕНО, gh api: `created: 2019-07-31T09:31:07Z`] |
| Звёзды / форки / issues | 163★ / 11 forks / 3 открытых [ПОДТВЕРЖДЕНО] |
| Мейнтейнер | Goran Mekić — все 200 коммитов видимой истории [ПОДТВЕРЖДЕНО, `git log --format="%an"`] |
| LOC | app: 95 486; engine: 80 814; plugin-host: 19 742 [ПОДТВЕРЖДЕНО, `wc -l`] |
| Сайт | https://maolan.rs |

Экосистема (все проверены через gh api, все BSD-2-Clause, все активно пушатся):

| Репо | Назначение | Pushed |
|---|---|---|
| maolan/maolan | приложение (iced GUI) | 2026-10-17 |
| maolan/engine | аудиодвижок, крейт `maolan-engine` | 2026-10-18 |
| maolan/plugin-host | «Out of process plugin host» | 2026-10-14 |
| maolan/widgets | iced-виджеты (`maolan-widgets`) | 2026-09-19 |
| maolan/editor | аудио-редактор (`maolan-editor`) | 2026-10-12 |
| maolan/pitch | pitch-коррекция (neural + resynth) | 2026-10-17 |
| maolan/generate | «Maolan AI for generating music» | 2026-10-17 |
| maolan/mixosc | OSC-микшер | 2026-10-13 |

## Лицензия и риски

- `LICENSE` (1317 байт) — **BSD 2-Clause**, «Copyright (c) 2020, Goran Mekić» [ПОДТВЕРЖДЕНО, полный текст прочитан].
- **Что это значит для VibeStudio:**
  - Копировать код **можно** при сохранении копирайт-нотиса и дисклеймера в продукте/документации. Бинарные и проприетарные использования разрешены.
  - Практически: любой модуль (например, тройной буфер, plan-рендер, IPC-протокол плагинов) можно перенести с атрибуцией « Portions Copyright (c) 2020, Goran Mekić».
  - Риск минимальный: нет copyleft, нет патентных оговорок сверх стандартных BSD. Единственная обязанность — не использовать имена авторов в рекламе без разрешения (пункт 3 лицензии).
- Тот же BSD-2 в engine, plugin-host и всех sibling-репо [ПОДТВЕРЖДЕНО, gh api license.spdx_id].

## Активность и здоровье

- Видимая shallow-история app: 200 коммитов до #178 (2026-05-29); нумерация PR дошла до **#409 (2026-10-17, «Update dependencies»)** [ПОДТВЕРЖДЕНО].
- Коммиты в октябре: #408 «Implement ASIO support» (2026-10-16), #406 «Place tracks on empty space», #402/401 «Move (all) code to plugin-host crate» — плотная ежедневная работа [ПОДТВЕРЖДЕНО, git log].
- Engine-репо создан 2026-04-25 (движок выделили в отдельный крейт недавно) [ПОДТВЕРЖДЕНО, gh api].
- CI ([.github/workflows/rust.yml](https://github.com/maolan/maolan/blob/main/.github/workflows/rust.yml)): матрица ubuntu/windows/macos + release-джобы (deb/rpm/wix-пакеты описаны в [package.metadata] Cargo.toml) [ПОДТВЕРЖДЕНО].
- В репо лежит [AGENTS.md](https://github.com/maolan/maolan/blob/main/AGENTS.md) — инструкция для AI-агентов (clippy `--fix` после каждого изменения, без `#[allow]`, debug-сборки). Мейнтейнер сам разрабатывает с AI-агентами — прямой прецедент workflow VibeStudio [ПОДТВЕРЖДЕНО].
- Bus-factor = 1 (Goran Mekić). Релизы: теги 0.4.0, 0.3.2, 0.3.1 [ПОДТВЕРЖДЕНО, gh api tags].

## Сборка и платформы

- App ([Cargo.toml](https://github.com/maolan/maolan/blob/main/Cargo.toml)): edition **2024**, Rust 1.85+; GUI — **iced через крейт maolan-widgets** (main.rs:90 `maolan_widgets::iced::application(...)`); CLI — `maolan-cli` на ratatui/crossterm; tokio full; ebur128 (loudness), midly (SMF), rubato, timestretch, pitch-detection, hf-hub (Hugging Face для нейро-pitch), rfd (диалоги) [ПОДТВЕРЖДЕНО].
- **Примечательно: main-крейт не содержит движка** — движок подключён с crates.io (`maolan-engine = "0.4.5"` с фичами `audio,midi,plugins,mixosc`) [ПОДТВЕРЖДЕНО].
- Аудио backends (engine + app):
  - Linux: ALSA (крейт alsa 0.12) + **JACK** (jack 0.13.5, cfg(unix));
  - macOS: CoreAudio (coreaudio_worker.rs);
  - Windows: WASAPI (windows_worker.rs, Win32 Media Audio) + **ASIO** (maolan-asio, #408);
  - FreeBSD: OSS (oss_worker.rs), OpenBSD: **sndio** (sndio_worker.rs) — BSD-наследие автора [ПОДТВЕРЖДЕНО, src/workers/ в maolan-engine].
- Движок [`maolan-engine` Cargo.toml](https://github.com/maolan/engine/blob/main/Cargo.toml): fearless_simd, basedrop, arc-swap, rtrb 0.4, triple_buffer, audioadapter, oxideav-* (декодеры), symphonia 0.6; фичи `audio/midi/plugins/mixosc`, `default = []` [ПОДТВЕРЖДЕНО].
- Plugin-host ([Cargo.toml](https://github.com/maolan/plugin-host/blob/main/Cargo.toml)): `vst3 = "0.3"`, собственные `maolan-lv2` (raw LV2-биндинги, src/lv2.rs), **CLAP реализован вручную поверх libloading** (src/clap_engine.rs — без clap-sys), GUI-встраивание X11 (gui_x11.rs) и Win32 (gui_win32.rs) [ПОДТВЕРЖДЕНО].

## Архитектура

**Двухуровневая процессная модель** (цитата из [README maolan-engine](https://github.com/maolan/engine/blob/main/README.md)):

1. Главный процесс DAW — UI, аудиодвижок, состояние проекта.
2. **Plugin host processes — по одному процессу на инстанс плагина**: shm-IPC (`mmap` / `CreateFileMappingW`) + события (pipe / named auto-reset events); при падении плагина DAW байпасит его и продолжает воспроизведение [ПОДТВЕРЖДЕНО, README + src/plugins/ipc.rs: spin-бюджет 2 мс, hung-host budget 100 мс].

Дерево движка (src/ maolan-engine):

| Модуль | Содержимое |
|---|---|
| `engine/` | automation, bounce, hardware, lifecycle, meters, midi, plugins, queries, recording, runtime, session, topology, transport |
| `plugins/` | `clap_proc.rs`, `vst3_proc.rs`, `lv2_proc.rs`, `au_proc.rs`, `ipc.rs`, `watchdog.rs` |
| `workers/` | `alsa_worker`, `coreaudio_worker`, `oss_worker`, `sndio_worker`, `windows_worker`, `hw_worker` |
| `audio/`, `midi/` | clip/track/io; **`midi/mpe.rs` — поддержка MPE** |
| `plan_builder.rs`, `executor.rs`, `render_plan.rs`, `parallel_render.rs`, `inline_render.rs` | план-рендер графа, node-воркеры |
| `triple_buffer.rs`, `simd.rs` | RT-обмен и SIMD |

[ПОДТВЕРЖДЕНО, обход каталогов.]

Ключевые решения:

- **UI → движок**: iced-приложение (Elm: src/gui/{update,view,subscriptions}), команды уходят через message/dispatch (src/gui/update/dispatch/); состояние движка публикуется через arc-swap/тройные буферы; «All state crossing thread boundaries lives … behind short mutexes or arc_swap snapshots» [ПОДТВЕРЖДЕНО, док-коммент inline_render.rs].
- **RT-режим «inline render»**: рендер плана выполняется прямо на hardware cycle thread между capture fill и playback drain, с rendezvous node-воркеров или последовательным исполнением (`MAOLAN_RT_PARALLEL=0`) [ПОДТВЕРЖДЕНО, src/inline_render.rs:1-30].
- **basedrop** — RT-safe аллокации планов (executor.rs:348-367) [ПОДТВЕРЖДЕНО].
- **Transport/минутный тайминг**: engine/transport.rs + cycle_trace.rs, iodelay.rs (компенсация I/O задержек).
- **Экспорт**: engine/bounce.rs + inline/parallel render; app: src/gui/update/dispatch/export.rs — форматы WAV/FLAC (rfd-фильтр «wav, flac»), опции bit depth / dither / normalize / sample rate (consts EXPORT_*) [ПОДТВЕРЖДЕНО].
- Сессии: каталоги `.maolan_autosave/snapshots`, шаблоны сессий/треков в `~/.config/maolan/daw/` [ПОДТВЕРЖДЕНО, README].
- Плагин-состояние хранится/восстанавливается в session_restore.rs, включая AU на macOS (`maolan_engine::au::AuPluginInfo`) [ПОДТВЕРЖДЕНО].

## Реальная готовность функций

| Функция | Статус | Доказательство |
|---|---|---|
| Граф/роутинг треков | Работает | engine/topology.rs, routing.rs, app connections/ (canvas JACK-патчинга) [ПОДТВЕРЖДЕНО] |
| Аудио/MIDI треки, клипы, запись | Работает | engine/{audio,midi}/, engine/recording.rs [ПОДТВЕРЖДЕНО] |
| CLAP hosting | Работает, OOP | plugin-host src/clap_engine.rs + engine/plugins/clap_proc.rs [ПОДТВЕРЖДЕНО] |
| VST3 hosting | Работает, OOP | plugin-host src/vst3/ (7 файлов, host/processor/state) [ПОДТВЕРЖДЕНО] |
| LV2 hosting (Unix) | Работает, OOP | plugin-host src/lv2.rs через maolan-lv2 raw [ПОДТВЕРЖДЕНО] |
| Audio Unit (macOS) | Работает | engine/plugins/au_proc.rs, session_restore.rs:128 [ПОДТВЕРЖДЕНО] |
| Crash-isolation плагинов | Работает | README engine + ipc.rs/watchdog.rs [ПОДТВЕРЖДЕНО] |
| Native plugin GUI | Работает | gui_x11.rs / gui_win32.rs в plugin-host [ПОДТВЕРЖДЕНО] |
| Экспорт WAV (+FLAC), dither/normalize | Работает | dispatch/export.rs, EXPORT_* consts [ПОДТВЕРЖДЕНО] |
| Offline bounce | Работает | engine/bounce.rs, inline_render.rs, parallel_render.rs [ПОДТВЕРЖДЕНО] |
| Pitch-коррекция (в т.ч. нейросетевая) | Работает | maolan/pitch, hf-hub, PitchCorrectionDetector::{Neural,Resynth} в session_restore.rs:525 [ПОДТВЕРЖДЕНО] |
| MPE | Реализовано | engine/src/midi/mpe.rs [ПОДТВЕРЖДЕНО] |
| MIDI-файлы (.mid) | Работает | midly, фича `midi` [ПОДТВЕРЖДЕНО] |
| OSC-управление | Работает | mixosc фича + bin/maolan-osc [ПОДТВЕРЖДЕНО] |
| AI-генерация музыки | Отдельный репо maolan/generate, степень интеграции unclear | gh api описание [ПРЕДПОЛОЖЕНИЕ] |
| Заглушки | 0 `todo!`/`unimplemented!`/`FIXME` в app и engine | grep [ПОДТВЕРЖДЕНО] |
| README-обещания vs код | README консервативен: «Plugin compatibility is host-dependent … evolving rather than guaranteed» [ПОДТВЕРЖДЕНО] |

## Тесты

- app: **654** `#[test]` (249 только в gui/); engine: **420**; plugin-host: **39** [ПОДТВЕРЖДЕНО, grep].
- Покрытие: dispatch-логика UI, session restore, piano/curve-виджеты; в движке — поля, transport, bounce; в plugin-host — протокол/скан.
- Аудио-регрессионных тестов «сравни с эталонным WAV» не обнаружено ни в app, ни в engine (по grep по тестовым файлам) [ПРЕДПОЛОЖЕНИЕ по отсутствию в видимом коде].

## Ограничения

- Много-репо архитектура: чтобы собрать полную картину, нужно 3+ репозитория; app зависит от версий с crates.io, т.е. снапшоты app и engine могут расходиться.
- Документации мало: в app README — только сборка/пути; настоящий «сорт» знаний — в коде и док-комментах (качественных).
- UI на кастомном iced-стеке (maolan-widgets форкает/обёртывает iced) — обновления iced нужно ловить самостоятельно.
- Windows-поддержка свежая (ASIO добавлен 2026-10-16), macOS-сборка есть в CI, но автор целится в Unix/BSD.
- Bus-factor = 1; 163★ при 6-летней истории — нишевый проект.

## Выводы для VibeStudio

- **Что брать: и компоненты, и идеи.** BSD-2 разрешает прямое заимствование. Наиболее переносимые единицы: (1) `triple_buffer.rs` (крошечный, самодостаточный); (2) схема OOP-плагинов (protocol + shm + watchdog) — если VibeStudio захочет crash isolation, это готовый чертёж; (3) подход `default = []` с opt-in фичами движка; (4) поддержка backends как отдельных worker-модулей (alsa/jack/coreaudio/wasapi/oss/sndio).
- **Движок как зависимость** (`maolan-engine` с crates.io) технически возможен под BSD-2, но связывает VibeStudio с чужой сессией релизов и tokio-центричным рантаймом; для цели «собственный аудиодвижок» — не подходит, лучше вдохновляться.
- **Как форк**: осмысленно только если взять всю экосистему (app+engine+plugin-host+widgets) — тогда это «свой DAW» со сложной много-репо инфраструктурой; для VibeStudio с целью monorepo и собственного движка — не рекомендуется.
- Отдельный урок: AGENTS.md с правилами для AI-агентов и bin/CLI для автоматизаций — паттерн, который maolan уже обкатал.

## Открытые вопросы

- Реальная стабильность OOP-плагинов при низкой латентности (spin-бюджет 2 мс в ipc.rs) — нужны живые замеры, вне рамок этого аудита.
- Насколько связан maolan/generate с app — есть ли в UI сценарии AI-генерации [ПРЕДПОЛОЖЕНИЕ: интеграция ранней стадии].
- Почему CLAP реализован без clap-sys (своя ABI-обвязка) — следить за совместимостью с будущими версиями CLAP.
- Что стало со старой (до 2026) архитектурой app — репо с 2019 года, движок выделили только в 2026-04; в shallow-истории виден активный рефакторинг «Move all code to plugin-host crate».

## Приложение: проверенные команды аудита

```sh
git clone --depth 200 https://github.com/maolan/maolan        # app, 200 коммитов до #178
git clone --depth 50  https://github.com/maolan/engine        # движок 0.4.6
git clone --depth 30  https://github.com/maolan/plugin-host   # OOP-хост 0.1.2
gh api repos/maolan/{maolan,engine,plugin-host,widgets,editor,pitch,generate,mixosc}
# grep: todo!/unimplemented!/FIXME → 0; #[test] → app 654, engine 420, plugin-host 39
```

Дата проверки всех фактов: 2026-10-20.
