# sources.md — Источники исследования

Дата исследования: 2026-10-20. Метод: страницы официального мануала скачивались и читались полностью (curl → текст); PDF извлечён штатными средствами; видео — через доступные транскрипты/метаданные (честная сводка ниже).

Уровни доступа: **полностью прочитано** / **прочитано частично** / **метаданные без содержания** / **не открыт**.

---

## 1. Официальная документация Image-Line [A]

Базовый префикс: `https://www.image-line.com/fl-studio-learning/fl-studio-online-manual/html/`. Актуальность: тексты соответствуют FL Studio 21.3/2024–2025 (FL Cloud, Gopher, Communications Panel, 500 insert-треков, Elastique e3), с локальными фрагментами эпохи FL 20.

| Страница | Что установлено | Доступ |
|---|---|---|
| `main_windows.htm` | состав главных окон, F5–F9, Ctrl+Shift+H | полностью |
| `basics_interface.htm` | прилипание окон (Alt-отмена), multi-monitor, Detached, жесты мыши (крутилки), multi-touch, hint | полностью |
| `basics_workflow.htm` | общий workflow, дубликат текста main_windows | полностью |
| `basics_shortcuts.htm` | все таблицы горячих клавиш (глобальные, Playlist, Piano Roll, …) | полностью |
| `toolbar_panels.htm` | все панели тулбара: transport, time, pattern, output, CPU, snap, multilink, shortcut icons | полностью |
| `menu.htm`, `menu_file.htm`, `menu_edit.htm`, `menu_add.htm`, `menu_patterns.htm`, `menu_view.htm`, `menu_options.htm`, `menu_tools.htm`, `menu_help.htm` | состав главных меню | полностью |
| `playlist.htm` | треки, Track Modes, клипы, инструменты P/B/D/T/S/C/E/Y/Z, маркеры, меню Playlist, arrangements, performance zone, consolidate | полностью |
| `playlist_patterns.htm` | Pattern Clips | полностью |
| `playlist_audioclip.htm` | Audio Clips (SMP+MISC, declick, fades, chop) | полностью |
| `playlist_automationclip.htm` | Automation Clips, типы кривых, LFO, initialized controls | полностью |
| `playlist_performance.htm` | Performance Mode (Press/Motion/Sync) | полностью |
| `pianoroll.htm` | структура ролла, ghost notes, инструменты, слайды, snap to scale, цветовые группы | полностью |
| `pianoroll_menu.htm` | меню Piano Roll (File/Edit/Tools/View/Helpers) | полностью |
| `pianoroll_chordprogression.htm`, `pianoroll_riff.htm` | генераторы прогрессий/риффов | полностью |
| `chansettings.htm`, `chansettings_main.htm`, `chansettings_sampler.htm`, `chansettings_ins.htm`, `chansettings_misc.htm` | Channel Settings: sampler, envelopes/LFO, slide/mono/porta, cut-groups, swing | полностью |
| `channelrack.htm` | устройство rack, Step Sequencer, Graph Editor, swing, меню канала, длина паттерна | полностью |
| `browser.htm` | Browser: вкладки, Current project, Plugin database, preview, поиск, drag-and-drop | полностью |
| `mixer.htm`, `mixer_mixermenu.htm`, `mixer_trackprops.htm`, `mixer_levelsandmixing.htm` | Mixer: 500 insert, sends, sidechain, submix, EQ трека, PDC | полностью |
| `effects_plugins.htm` | FX-слоты, порядок, состояния | полностью |
| `automation_eventeditor.htm`, `automation_internal.htm`, `automation_linking.htm` | Event Editor, internal controllers, link formulas, MIDI-learn | полностью |
| `recording.htm`, `recording_scores.htm`, `recording_automation.htm`, `recording_audio.htm`, `recording_step.htm` | фильтры записи, Score Logger, запись аудио, step entry | полностью |
| `fformats_save_export.htm`, `fformats_save_zip.htm` | экспорт (форматы, Tail, стемы), Zipped loop package, bones | полностью |
| `envsettings_general.htm`, `songsettings_settings.htm` | General settings (масштаб, поведение плагинов, Startup/Default template), Project general (PPQ=96, размер) | полностью |
| `stepsequencer_advancedfilltool.htm` | Advanced fill | полностью |

## 2. Официальный PDF [A]

- **FL Studio 20 Getting Started Manual** (88 стр.): `https://www.image-line.com/fl-studio-learning/fl-studio-online-manual/boxinstaller/WebContent/FLStudio20_GettingStartedManual.pdf`
- Установлено: сценарии первых шагов (Channel Rack → шаги → паттерны → Playlist → Piano Roll → слайды → микшер → эффекты → автоматизация → Event Editor → Automation Clips → Edison → экспорт → контроллеры); дефолтные уроки на demo-проекте NewStuff.flp; «первый запуск» (F10, ASIO, буфер); состав стоковых плагинов. Извлечён полностью; противоречий с онлайн-мануалом не обнаружено (число insert-треков 125 — устаревшее относительно 500).

## 3. Видео [A/B] — честная сводка

| Материал | Доступ | Что использовано |
|---|---|---|
| **FL Studio (офиц. канал), «Learn FL Studio — Complete Beginner's Tutorial»**, 1:00:58 — `https://www.youtube.com/watch?v=3oKVpTHC-0M` | **полный текстовый транскрипт получен** (web_reader); как видео не просмотрено | последовательность обучения: welcome/шаблоны → Browser → Channel Rack → Piano Roll → аранжировка (Make unique, Split by channel, маркеры) → Mixer (Reeverb 2) → автоматизация → запись → экспорт (советы по Sinc/битрейту) |
| Busy Works Beats, «FL Studio Complete Beginner Basics Tutorial» — `https://www.youtube.com/watch?v=vORMQYdQjQc` | только страница видео (метаданные) | **не использовано по существу** |
| In The Mix, beginner-туториал | страница не открылась; видны только заголовок и список глав в выдаче | подтверждение универсального порядка уроков (menus → channel rack → patterns → playlist → mixer → export) |
| Kevin Stratvert, «FL Studio tutorial for beginners» — `https://kevinstratvert.com/2024/02/02/fl-studio-tutorial-for-beginners` | прочитана аннотация с главами (не транскрипт) | перечень тем 25-главного цикла |
| Официальные уроки `https://www.image-line.com/learn/lesson/making-a-beat-in-channel-rack` | прочитано описание урока | подтверждение дефолтного шаблона как базы первого урока |

## 4. Письменные сторонние гайды [B]

| Источник | Доступ | Что установлено |
|---|---|---|
| EDMProd — `https://www.edmprod.com/fl-studio-for-beginners` | полностью | дефолтные каналы Kick/Clap/Hat/Snare; практика шагов/свинг; типовые стоковые эффекты; «отдельный паттерн на партию» |
| mastering.com — `https://mastering.com/fl-studio-tutorial` | полностью | маршрутизация Ctrl+L, запись MIDI, sends для ревербов, EQ/компрессия, экспорт |
| Production Den — `https://productionden.com/how-to-use-fl-studio` | полностью | «чистый» старт (Minimal > Basic, F12), drag сэмплов, типовой цикл |
| Soundverse — `https://www.soundverse.ai/blog/article/how-to-make-beats-on-fl-studio-1105` | полностью (частично реклама) | типовые BPM жанров, выбор стоковых синтов |
| Официальный форум (сниппет поиска) — `https://forum.image-line.com/viewtopic.php?t=325040` | **не открыт**, только сниппет | дефолтный темп 130 BPM — низкая уверенность |
| Reddit r/FL_Studio (сниппет) — `https://www.reddit.com/r/FL_Studio/comments/4muh3s/` | **не открыт** | zipped loop package как «pro-tip» (подтверждено мануалом независимо) |
| Instructables — `https://www.instructables.com/How-to-Make-a-Beat-in-FL-Studio` | не прочитался (JS) | исключён |

## 5. Методические пометки

- Все URL мануала проверены реальной загрузкой; цитирование фактов — по скачанным копиям (кэш /tmp/flmanual, не сохраняется).
- Расхождения внутри самого мануала зафиксированы в `behavior-catalog.md` §11, а не «сглажены».
- Всё, что не удалось подтвердить (динамика интерфейса в видео, дефолтная раскладка окон, число треков Playlist), помечено «не найдено» и в выводы не попадало.
