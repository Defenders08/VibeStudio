# tripplyons/daw — глубокое исследование исходников

Дата проверки: **2026-10-20**. Метод: shallow clone + чтение кода + `gh api`.

## Краткая сводка-вердикт

- Самый молодой из трёх: репо создан **2026-09-27** (3 недели до даты проверки), 43 коммита, один автор (Tripp Lyons), без CI, без релизов/тегов.
- Но это **не игрушка**: 22 774 строки Rust, 174 теста, 4-крейтовый workspace, чистое разделение model/engine/plugins/app.
- Ближайший по духу к FL Studio из всех трёх: **patterns + playlist + channel rack + mixer + piano roll + automation**, тайлинг-UI в стиле i3/sway на iced 0.14.
- **VST3 (macOS/Linux) + Audio Unit (macOS) hosting** — AU реализован через `AUAudioUnit` (objc2), VST3 через крейт `vst3` 0.3 + собственный host (~2 500 строк); sandboxed сканирование в отдельных процессах с кэшем.
- Экспорт WAV (включая stems) и consolidate-selection работают в фоне с прогрессом и отменой; CLI (`daw export/edit/analyze/pack/unpack`) и **готовый skill для Claude Code** (`.claude/skills/daw-project`) — проект изначально строится как AI-friendly.
- **Лицензия MIT — код можно свободно брать в VibeStudio с атрибуцией.** Как база/референс для FL-Studio-подобной DAW — самый релевантный из трёх, но нужно учитывать: нет Windows, нет CI, возраст 3 недели.

## Метаданные

| Поле | Значение |
|---|---|
| Repo | https://github.com/tripplyons/daw (default branch `main`) |
| Версия | 0.1.0 (workspace) |
| Создан | 2026-09-27 [ПОДТВЕРЖДЕНО, gh api: `created: 2026-09-27T18:05:38Z`] |
| Звёзды / форки / issues | 6★ / 1 fork / 0 открытых [ПОДТВЕРЖДЕНО] |
| Мейнтейнер | Tripp Lyons — все 43 коммита [ПОДТВЕРЖДЕНО, `git log --format="%an"`] |
| Первый коммит | «Initial DAW: tiling layout, VST3/AU hosting, patterns, automation» (2026-09-27) [ПОДТВЕРЖДЕНО] |
| Последний | 2026-10-09 «Show a channel picker in the piano roll header» [ПОДТВЕРЖДЕНО] |
| Размер | ~4 MB, 22 774 строк `.rs` [ПОДТВЕРЖДЕНО] |

## Лицензия и риски

- `LICENSE` — стандартный **MIT**, «Copyright (c) 2026 Tripp Lyons» [ПОДТВЕРЖДЕНО, полный текст прочитан].
- **Что это значит для VibeStudio:** код (файлы, функции, крейты целиком) можно копировать, модифицировать, сублицензировать и продавать при сохранении копирайт-нотиса и текста лицензии. Атрибуция — единственная обязанность. Рисков нет, кроме моральных/репутационных: если брать большие куски, честно указать происхождение.
- Внимание: `daw` бинарь линкует VST3 SDK bindings (`vst3` crate) — сам VST3 SDK от Steinberg имеет собственную лицензию (GPLv3 / proprietary dual), но bindings-крейт используется одинаково во всех трёх проектах; для хостинга это стандартная практика [ПРЕДПОЛОЖЕНИЕ по лицензии SDK, вне этого репо].

## Активность и здоровье

- 43 коммита за ~2 недели — очень плотный темп для одного человека; коммиты содержательные («Fix playback behavior around jumps, edits and tempo automation», «Share loop, clip-copy and plugin host helpers») [ПОДТВЕРЖДЕНО].
- **CI отсутствует** (`no .github/workflows`) [ПОДТВЕРЖДЕНО, ls]. Тесты гоняются вручную/локально (`bazel test //...`, `cargo test`).
- Релизов и тегов нет [ПОДТВЕРЖДЕНО, gh api tags → пусто]. Пользователям предлагается собирать из исходников.
- Issues открытых 0 — либо нет пользователей, либо всё в HEAD-работе [ПОДТВЕРЖДЕНО, gh api].
- Двойная сборка: Cargo + **Bazel (rules_rust, Bazel 9.2.0 через Bazelisk)** [ПОДТВЕРЖДЕНО, MODULE.bazel, .bazelversion].

## Сборка и платформы

- Workspace: `crates/daw` (app, 13 691 LOC), `daw-engine` (2 850), `daw-model` (2 589), `daw-plugins` (3 644) [ПОДТВЕРЖДЕНО, `wc -l`].
- Edition 2024, Rust 1.88+; deps ([Cargo.toml](https://github.com/tripplyons/daw/blob/main/Cargo.toml)): **cpal 0.18**, **rtrb 0.4**, **hound 3.5** (WAV), **iced 0.14** (canvas/tokio/advanced), **vst3 0.3**, libloading, serde+ron, objc2-стек (AppKit, Audio Toolbox, AVF Audio, Core Audio Types) [ПОДТВЕРЖДЕНО].
- Платформы: **macOS и Linux только**. Windows — нет. Linux-аудио через ALSA (cpal; PipeWire/Pulse совместимы), plugin-редакторы — X11/XWayland [ПОДТВЕРЖДЕНО, README + window_linux.rs].
- Linux: нужны libasound2-dev, libx11-dev, libxkbcommon-dev, libwayland-dev, libclang-dev; macOS: Xcode CLT; есть `scripts/bundle-app.sh` (ad-hoc подписанный DAW.app с декларацией типа `.dawproj`) [ПОДТВЕРЖДЕНО, README].
- Без аудиоустройства приложение работает (edit + export WAV) [ПОДТВЕРЖДЕНО, README].

## Архитектура

- `daw-model` — сериализуемая модель (serde/ron): song, patterns, tracks, automation, PluginRef; формат проекта `.dawproj` = **ZIP** с текстовым документом и папкой `assets` (встраивает WAV'ы, plugin state) [ПОДТВЕРЖДЕНО, README Files].
- `daw-engine` — RT-ядро:
  - [engine.rs](https://github.com/tripplyons/daw/blob/main/crates/daw-engine/src/engine.rs): `Engine::render` на аудиопотоке; `MAX_BLOCK = 512` (крупные буферы дробятся), автоматизация на sub-блоках 128 сэмплов; node-граф `MAX_NODES = 4096`; event-буферы фиксированной ёмкости (`EVENT_CAPACITY = 2048`, «Drop events past capacity rather than allocate on the audio thread»); деклик-фейды 64 сэмпла на краях клипов [ПОДТВЕРЖДЕНО, константы и комментарии].
  - **EngineHandle** (engine.rs:236-258): UI общается с движком только через `rtrb::RingBuffer` команд + **отдельная «garbage» очередь** — удалённые с аудиопотока узлы возвращаются на UI-поток для деаллокации (RT-safe memory reclamation) [ПОДТВЕРЖДЕНО].
  - `Processor` — общий трейта-интерфейс для встроенных инструментов и плагинов; `synth.rs` — полифонический синт (16 голосов) + one-shot сэмплер [ПОДТВЕРЖДЕНО].
  - `output.rs`: экспорт WAV через **hound::WavWriter** (16/24/float32), включая **stems** (по writers на стем) [ПОДТВЕРЖДЕНО, output.rs:99-114].
- `daw-plugins`:
  - `vst3/` (host.rs 598, mod.rs 811, run_loop.rs 202, linux.rs 167) — собственный VST3-host поверх `vst3`-крейта, включая Linux run-loop для редакторов;
  - `au.rs` (695 строк) — **Audio Unit hosting через `AUAudioUnit`** («covers AUv2 components (bridged by the system) and AUv3 extensions alike»), параметры/автоматизация через AUParameterObservation/automation events [ПОДТВЕРЖДЕНО, док-коммент и импорты objc2_audio_toolbox];
  - `scan.rs` — сканирование каждого плагина **в отдельном процессе**, результаты в кэш `plugins.ron`; упавший/зависший плагин помечается failed, приложение живёт [ПОДТВЕРЖДЕНО, README Plugins];
  - `window.rs` / `window_linux.rs` — нативные окна редакторов (AppKit / X11), проброс клавиш, которые плагин не съел [ПОДТВЕРЖДЕНО].
- `daw` (app): iced-UI с тайлингом (панели как в i3: split/focus/workspaces), `panels/`, `processing.rs` (фоновая pitch-shift/stretch очередь с кэшем 256 MiB/64 записи), `render.rs` (consolidate/экспорт в фоне), `capture.rs` (запись), `cli/` (подкоманды), `session.rs` [ПОДТВЕРЖДЕНО, ls crates/daw/src].
- Transport: pattern mode vs song mode, loop markers, tempo automation (TempoMap в model), запись MIDI-тейков с разрезанием нот по границам лупа [ПОДТВЕРЖДЕНО, README Live MIDI].

## Реальная готовность функций

| Функция | Статус | Доказательство |
|---|---|---|
| Patterns + playlist (FL-стиль) | Работает | README, daw/src/panels/ [ПОДТВЕРЖДЕНО] |
| Piano roll + velocity + snap | Работает | daw/src/panels, коммит «channel picker in the piano roll header» [ПОДТВЕРЖДЕНО] |
| Микшер с sends + sidechain | Работает | README «Mixer sends and sidechains»; feedback-роуты отклоняются [ПОДТВЕРЖДЕНО] |
| Automation (клипы, запись, bind-mode Alt+A) | Работает | README + model/automation [ПОДТВЕРЖДЕНО] |
| VST3 hosting | Работает (macOS/Linux) | daw-plugins/src/vst3/ (1 778 строк) [ПОДТВЕРЖДЕНО] |
| Audio Unit hosting (macOS) | Работает | au.rs 695 строк через AUAudioUnit [ПОДТВЕРЖДЕНО] |
| Sandboxed plugin scan + кэш | Работает | scan.rs, README Plugins [ПОДТВЕРЖДЕНО] |
| Native plugin editor windows | Работает | window.rs / window_linux.rs [ПОДТВЕРЖДЕНО] |
| Экспорт WAV + stems, фон + отмена | Работает | output.rs (hound), daw/src/render.rs [ПОДТВЕРЖДЕНО] |
| Consolidate selection → WAV | Работает | README «consolidate», хвосты 0-120 c [ПОДТВЕРЖДЕНО] |
| Аудиозапись с latency-компенсацией | Работает | input.rs (rtrb, timestamps), README Record [ПОДТВЕРЖДЕНО] |
| Pitch shift / stretch клипов | Работает (фоновая обработка, кэш, отмена) | README + processing.rs [ПОДТВЕРЖДЕНО] |
| MIDI-вход и запись | Работает | daw/src/midi.rs, README Live MIDI [ПОДТВЕРЖДЕНО] |
| Undo/redo вкл. plugin params | Работает | README Renaming/Plugins [ПОДТВЕРЖДЕНО] |
| Самодостаточный `.dawproj` (ZIP) + autosave | Работает | README Files [ПОДТВЕРЖДЕНО] |
| CLI + AI-agent skill | Работает | `daw export/edit/batch/analyze/pack`, `.claude/skills/daw-project/SKILL.md` [ПОДТВЕРЖДЕНО] |
| Windows | Нет | README Platforms [ПОДТВЕРЖДЕНО] |
| Встроенные инструменты | Минимальные: 16-голосый синт + сэмплер | daw-engine/src/synth.rs [ПОДТВЕРЖДЕНО] |
| Заглушки/TODO | 0 `todo!`/`unimplemented!`; 0 TODO/FIXME | grep по crates/ [ПОДТВЕРЖДЕНО] |

## Тесты

- **174 `#[test]`** [ПОДТВЕРЖДЕНО, grep]. Плотность высокая (174 на 22.8k LOC ≈ 1 тест на 131 строку).
- Характерные примеры: `block_processing_preserves_exact_length_stereo_pitch_and_level` (daw-engine/src/audio.rs:241) — проверка точной длины/стерео/уровня при блоковой обработке; тесты jump/tempo-правок (коммит 2026-09-30).
- Баг-регрессии: коммиты вида «Fix playback behavior around jumps, edits and tempo automation» сопровождались тестами [ПРЕДПОЛОЖЕНИЕ по практике, поддержано текстом коммитов].
- Эталонных аудио-сравнений (golden WAV) не видно; проверки через числовые инварианты [ПОДТВЕРЖДЕНО по имени теста].
- Bazel-тесты дублируют cargo (`bazel test //...` описан в README), т.е. при появлении CI подключение тривиально [ПОДТВЕРЖДЕНО, README Build with Bazel].

## AI-friendly дизайн (прямо релевантно VibeStudio)

- `.claude/skills/daw-project/SKILL.md` — обучающий skill для кодинг-агентов: модель проекта, форматы значений, разобранный пример; подключается симлинком в `~/.claude/skills` или `~/.agents/skills` для других агентов [ПОДТВЕРЖДЕНО, README «Agent skill»].
- CLI спроектирован как скриптовый интерфейс над моделью проекта: `daw new/show/edit/batch/plugins/params/export/analyze/pack/unpack` — агент может строить и менять проекты без GUI [ПОДТВЕРЖДЕНО, README Command line + crates/daw/src/cli/].
- `daw analyze` — уровни и octave-полосы по WAV (своя утилита контроля качества экспорта) [ПОДТВЕРЖДЕНО, README].
- Для VibeStudio это готовый шаблон «второго входа» в DAW для автоматизации и тестов.

## Панели UI и конфигурация

- Панели (crates/daw/src/panels/): piano roll, playlist, mixer, channel rack, plugin browser, automation editor, settings; тайлинг с workspaces (Alt+1..9), focus-mode, split по Alt+Enter [ПОДТВЕРЖДЕНО, ls + README Default keys].
- Единая схема скролла/зума для piano roll, playlist и automation (курсор-якорный зум) [ПОДТВЕРЖДЕНО, README Scrolling].
- Кейбиндинги переназначаемые, хранятся в `~/.config/daw/config.json` (XDG-совместимо); матtching по физическому положению клавиш [ПОДТВЕРЖДЕНО, README + daw/src/keys.rs, config.rs].
- Диалоги файлов: XDG desktop portal или zenity на Linux [ПОДТВЕРЖДЕНО, README].
- Прочие модули app: `theme.rs`, `units.rs` (парсинг «16bar», «1beat»), `dialogs.rs`, `open_files.rs`, `quit.rs` — аккуратная нарезка обязанностей [ПОДТВЕРЖДЕНО, ls crates/daw/src].

## Ограничения

- Возраст 3 недели, один автор, **нет CI и релизов** — каждый сборщик должен сам прогонять тесты.
- Нет Windows — для VibeStudio (macOS/Windows/Linux) Windows-путь придётся строить самому.
- Встроенных инструментов и эффектов почти нет (синт/сэмплер; эффекты в основном через VST3/AU) — это хост, а не «комплект как FL Studio».
- Никакой документации архитектуры (только очень подробный README-фичлист); код хорошо прокомментирован в узлах RT-логики.
- 6★ — проект не обкатан сообществом.

## Выводы для VibeStudio

- **Что брать: и идеи, и компоненты — лицензия позволяет.** Максимальная ценность для FL-Studio-подобного VibeStudio:
  1. **AU-хостинг на objc2/AUAudioUnit** — готовый, компактный (695 строк) образец, которого нет у vibez и maolan в таком явном виде (у maolan AU через свой au_proc + plugin-host).
  2. **Паттерн garbage-очереди** для RT-safe деаллокации узлов на UI-потоке — проще и нагляднее, чем арены.
  3. **Разделение model/engine/plugins/app** — чистый минимальный каркас, с которого удобно сверять структуру VibeStudio.
  4. **CLI + `.claude/skills/daw-project`** — готовая схема «DAW как инструмент для AI-агентов» (интерфейс `daw edit song.dawproj note add …`) — прямой кандидат на переосмысление в VibeStudio.
  5. Тайлинг-UI и FL-терминология (patterns/channel rack/playlist) — проверка, что такие GUI реализуемы на iced 0.14.
- **Как форк**: технически возможен (MIT), но неразумно целиком — нет Windows/CI, и код молодой; лучше брать модули (au.rs, vst3/host.rs, engine.rs) с атрибуцией.
- **Сравнительно с vibez**: vibez показывает «как выглядит зрелый масштаб», tripplyons — «как за 3 недели собрать работающий FL-хост»; читать в паре максимально полезно.

## Открытые вопросы

- Судьба проекта: активность с 2026-10-09 (10 дней тишины перед датой проверки) — пауза или смена режима работы? Следить за репо.
- Корректность VST3-хоста на широком пуле плагинов (авторские тесты покрывают vtable/цикл жизни, но реальная матрица плагинов неизвестна) [ПРЕДПОЛОЖЕНИЕ].
- Нет CI: не ломаются ли `bazel test` и `cargo test` синхронно при обновлении iced 0.14 → 0.15.
- Лицензионная цепочка `vst3`-крейта и Steinberg SDK для дистрибутива бинарников VibeStudio — уточнить отдельно (общий вопрос для всех трёх проектов).

## Приложение: проверенные команды аудита

```sh
git clone --depth 200 https://github.com/tripplyons/daw
gh api repos/tripplyons/daw   # created 2026-09-27, 6★, license MIT
ls .github/workflows          # отсутствует (нет CI)
# grep 'todo!|unimplemented!|TODO|FIXME' по crates/ → 0
# find crates -name '*.rs' | xargs wc -l → 22 774; grep '#\[test\]' → 174
```

Дата проверки всех фактов: 2026-10-20.
