# behavior-catalog.md — Каталог наблюдаемого поведения, горячих клавиш и меню

Все таблицы — только подтверждённое [A] (официальный мануал `basics_shortcuts.htm` + страницы разделов), если не помечено иное. macOS: Alt=Opt, Ctrl=Cmd. Важная оговорка мануала: **одиночные буквенные клавиши работают, только когда выключен Typing keyboard to piano (Ctrl+T)**; шорткаты контекстны (активное окно).

---

## 1. Глобальные / файлы / окна

| Клавиша | Действие |
|---|---|
| Ctrl+Z / Ctrl+Alt+Z | Undo последнего / шаг назад по истории |
| Ctrl+S / Ctrl+Shift+S / Ctrl+N | Save / Save as / **Save new version** (⚠ не «New»!) |
| Ctrl+O | Open |
| Ctrl+R / Ctrl+Shift+R / Ctrl+Shift+M | Export WAV / Export MP3 / Export MIDI |
| Alt+1..0 | Недавние проекты |
| F5 / F6 / F7 / F9 / F10 / F11 | Toggle Playlist / Channel Rack / Piano Roll / Mixer / MIDI settings / Project info |
| F8 / Alt+F8 / Ctrl+F8 / Shift+F8 | Plugin Picker (не toggle) / Browser / Project Picker / Plugin database |
| F12 / Alt+F12 / Ctrl+F12 / Shift+F12 | Закрыть все окна / все окна плагинов / нефокусированные / выровнять окна каналов |
| Ctrl+Shift+H | Сброс раскладки окон |
| Esc / Tab / Enter | Закрыть окно / цикл окон / maximize-minimize Playlist |
| Ctrl+F11 | Скрыть тулбар |
| F1 | Контекстная справка (по последнему клику) |
| F3 | Tool selector активного окна |

## 2. Transport / запись / паттерны

| Клавиша | Действие |
|---|---|
| Space / Ctrl+Space | Play-Stop / Play-пауза |
| R | Record toggle |
| L | PAT ↔ SONG |
| Ctrl+H | Паника (Stop sound) |
| Ctrl+M / Ctrl+P / Ctrl+I | Метроном / Precount / Wait for input |
| Ctrl+B | Blend record (ноты примешиваются) |
| Ctrl+E | Step edit mode |
| Ctrl+T | Typing keyboard to piano |
| Ctrl+J | Multilink to controllers |
| 0, *, / (NumPad) | Fast-forward / такт вперёд / такт назад |
| Alt+* , Alt+/ (NumPad) | След./пред. Time-маркер |
| Home | Playhead в начало |
| 1..9 (NumPad) | Выбор паттернов 1..9 |
| + / − | След./пред. паттерн |
| F4 / Shift+F4 / Ctrl+F4 | След. пустой паттерн (с именем) / первый пустой / след. пустой |
| F2 | Rename паттерна/канала/трека (повторное F2 — случайный цвет) |
| Alt+C | Clone паттерна (в Pattern Panel; в rack — clone канала) |
| Del | Удалить паттерн (в Pattern Panel) |
| Backspace | Toggle Global Snap Line/None |

## 3. Channel Rack / каналы

| Клавиша | Действие |
|---|---|
| Alt+C / Alt+Del | Clone / Delete выбранных каналов |
| Alt+↑ / Alt+↓ | Передвинуть выбранные каналы |
| Alt+G / Alt+Z / Alt+U | Group / Zip / Unzip каналы |
| Alt+M | Показать/скрыть mixer-селекторы |
| Ctrl+L / Ctrl+Shift+L | Каналы → свободный mixer-трек / последовательно от выбранного |
| Alt+1..0 | Mute/Solo каналов 1..10 |
| Ctrl+C / X / V | Копировать/вырезать/вставить данные шагов/скора каналов (между каналами и паттернами) |
| Ctrl+K | Graph Editor |
| Shift+Ctrl+← / → | Rotate данные канала влево/вправо |
| Shift+колесо (на канале) | Переупорядочить канал; на фоне заголовка — сменить паттерн |
| ЛКМ кнопка канала | Открыть Channel Settings + GUI плагина |
| Alt+ЛКМ / Ctrl+ЛКМ | Открыть без закрытия предыдущего / preview звука |
| ПКМ кнопка канала | Меню канала (Piano roll, Rename, Load sample, Cut self, Insert/Replace/Clone/Delete, Fill each 2/4/8, Rotate, Receive notes from…) |
| ЛКМ/ПКМ по шагу | Включить / выключить шаг |
| PageUp/PageDown | След./пред. фильтр-группа каналов |

## 4. Piano Roll (основное; полный список — в мануале #shortcuts_pianoroll)

| Клавиша | Действие |
|---|---|
| P / B / N / D / T / C / E / Z / Y | Draw / Paint / Paint-drum / Delete / Mute / Slice / Select / Zoom / Playback |
| S / O | Slide toggle / Portamento toggle |
| G / H / J / K / Shift+H / Shift+K | Соседние каналы (H/J — только с данными) |
| Ctrl+A / Ctrl+D / Shift+I / Shift+C | Select all / снять / инвертировать / по цвету |
| Del | Удалить выделенное (без выделения — всё) |
| Ctrl+B | Дублировать вправо |
| Ctrl+G | Glue (склеить ноты) |
| Ctrl+U / Alt+U | Quick chop / Chop |
| Ctrl+L / Alt+L | Quick legato / Articulate |
| Shift+D | Discard note lengths (ноты → шаги) |
| Shift+↑↓ / Ctrl+↑↓ | Транспонировать на полутон / октаву |
| Shift+←→ / Shift+Ctrl+←→ | Сдвиг на snap / rotate в такте |
| Alt+стрелки | Nudge (пиксели) |
| Alt+Q / Shift+Q | Quantize / quick quantize start times |
| Alt+A / Alt+S / Alt+F | Arpeggiate / Strum / Flam |
| Alt+R / Alt+X / Alt+O / Alt+K / Alt+Y | Randomize / Scale levels / LFO / Limit / Flip |
| Alt+E / Alt+P / Alt+W | Riff Machine / Chord progression / Claw machine |
| Alt+C / Alt+V / Ctrl+Alt+V | Сменить цвет / Ghost channels / Editable ghosts |
| Alt+B / Alt+N | Подсветка нот сетки / фоновая waveform |
| Ctrl+M / Shift+Ctrl+M | Import MIDI / Export MIDI (ролл) |
| Ctrl+Ins / Ctrl+Del / Ctrl+I | Вставить время / удалить время / вставить текущее значение контроллера |
| Alt+колесо (над нотой) | Менять свойство ноты |
| Double-ЛКМ (нота) | Свойства ноты |
| Double-ПКМ (ghost) | Перейти на канал ghost-ноты |
| Shift+Enter | Показать/скрыть панель свойств |
| Shift+F | Цикл свойства в панели свойств |
| Shift+1..5, Shift+0 | Пресеты зума / по выделению / центр на playhead |

## 5. Playlist (основное)

| Клавиша | Действие |
|---|---|
| P / B / D / S / C / E / T / Y / Z | Draw / Paint / Delete / Slip / Slice / Select / Mute / Playback / Zoom |
| Shift+drag / Shift+клик | Копия клипа / клон выделения |
| Ctrl+B | Дублировать вправо (шаг задаёт time-выделение) |
| Ctrl+G / Shift+Ctrl+G / Ctrl+Alt+G* | Merge pattern clips / merge similar / (*в плейлисте Ctrl+Alt+G = Grid color) |
| Shift+G / Alt+G | Group / Ungroup клипы |
| Alt+M / Alt+Shift+M | Mute / Unmute выделения |
| 7 / 8 / 9 | Аудио: транспонировать −1 / +1 полутон / Reverse |
| Shift+M | Режим stretch при растяжении аудио |
| Ctrl+Q или Shift+Q* | Quick quantize клипов (в мануале расхождение) |
| Ctrl+Ins / Ctrl+Del / Ctrl+Alt+Ins | Вставить время / удалить время / slice+вставить |
| Ctrl+Enter | Select time around selection |
| Ctrl+← / → | Сдвинуть time-выделение |
| Ctrl+drag (линейка) | Выделение области времени (loop/рендер) |
| Alt+T / Ctrl+T / Shift+Alt+T / Shift+T | Маркер / Auto-маркер / смена размера / Place loop |
| Ctrl+Alt+C / Ctrl+Alt+Shift+C | Consolidate от начала трека / от начала песни (freeze) |
| Alt+P | Picker Panel |
| Ctrl+P | Performance Mode |
| Alt+Right-click | Audition клипа |
| Shift+колесо (зона имён) | Переупорядочить треки |
| Shift+Alt+колесо (клип) | Nudge клипа |
| Insert | Slice клипа в позиции курсора |
| Ctrl+Shift+B | Clone selected playlist track(s) |

\* помечено расхождение внутри мануала.

## 6. Snap-система [A]

- **Global Snap Selector** (тулбар) действует на Piano Roll / Playlist / Event Editor, когда их локальный snap = Main. Значения: (none), Line, Cell, Beat 1/6·1/4·1/3·1/2·1, Step 1/6·1/4·1/3·1/2·1, Bar.
- Соответствие: 1/4 step = 64-я, 1/2 step = 32-я, 1/4 beat = 16-я, 1/3 beat = триоль, 1/2 beat = 8-я, 1 beat = четверть, Bar = целая.
- **Alt** — временно отключить snap; **Backspace** — toggle Line/None.
- Snap определяет **квантизацию MIDI-ввода**: Step/Beat/Bar — квантизирует; Line/Cell/none — нет.
- ПКМ по snap-селектору Piano Roll — длина ноты по умолчанию.

## 7. Поведение контролов (крутилки/слайдеры/LCD) [A]

| Действие | Результат |
|---|---|
| Вертикальный drag | изменение значения |
| Ctrl+drag / обе кнопки | точная подстройка |
| Shift+drag | игнорировать точки остановки |
| Alt+ЛКМ / средняя кнопка | сброс в дефолт |
| ПКМ | меню: Type in value, Reset, Create automation clip, Link to controller, Edit events, Init song with this position |
| У краёв LCD | ускоренный инкремент |
| Наведение | Hint bar: описание, значение, хоткей; красная точка = записываем; MIDI-иконка = линкуем |

## 8. Меню окон (якорь-стрелка в заголовке) — наблюдаемые составы

### Playlist menu [A]
- **Edit**: Cut/Copy/Paste/Duplicate, Delete, Shift left/right/up/down, Allow resizing from the left, Nudge clips with mouse wheel, Drop audio on track headers (as audio clips/audio tracks/instrument tracks/ask), Automatic crossfades, Reverse selected audio clips, Mute/Unmute, Insert/Delete space…
- **Tools**: Quick quantize, Consolidate (selection/track), Export all playlist tracks, Clone track(s)
- **View**: Grid color/contrast, Time segments, Behind clips (стили фона), Mini preview, Resize all tracks…
- **Snap / Select (all, by source, muted, overlapping, invert, time around) / Group / Zoom / Time markers / Picker panel**
- Прочее: Performance mode, Arrangements (клонировать/объединить), Change declicking, Detached.

### Piano Roll menu [A]
- **File**: Open/Save score (.fsc), Import/Export MIDI, Export as score sheet (PDF)
- **Edit**: Cut/Copy/Paste/Duplicate/Delete, Shift/Rotate, Transpose, Discard lengths, Change color, Mute, Insert/Delete space, Turn into automation clip
- **Tools**: Riff Machine, Chord progression, Quantize, Chop, Glue, Legato, Articulate, Strum, Flam, Arpeggiate, Claw machine, Limit, Flip, Randomize, Scale levels, LFO, Stamp, Script…
- **View**: Grid, Note colors (палитры), Helpers (note grid, scale highlighting, ghost channels, background waveform), Keyboard style/labels
- **Snap / Select / Group / Zoom / Time markers**, Target channel, Detached.

### Channel Rack menu [A]
Add one, More plugins…, Channel display filter (группы), Sort by (Name/Type/Label/Mixer), Set swingmix for selected, Set truncate swing notes, Find first empty, Clone/Delete selected, Zip/Unzip, Cut/Copy/Paste, Stepsequencer (Graph editor, Looping modes), Template (пресеты rack), Detached.

### Mixer menu [A]
File (Save/Open mixer track state, Browse states), Edit (Copy/Paste track state…), Add insert track(s), Track routing (Route to this track (only), Sidechain…, Create submix, Route selected channels), Group (create/auto color), View (Routing cables, Track inspector on left side), Plugin delay compensation, Disk recording, Detached.

### Меню клипа Playlist (иконка на клипе) [A]
Preview; Select source pattern/channel; Channel settings; Edit pattern; Rename and recolor; Change color; Make unique / Make unique as sample; Select all similar; **Chop** (по сетке/битам) / Chop into; Sample (Edit in Edison, Pitch correct, Time-warp, Extract stems, Detect/Fit to tempo); Automate (Volume/Pan), Crossfade with; Region; Delete.

## 9. Индикаторы и «язык состояния» [A]

- Зелёный LED = включено (канал, шаг mute, FX-слот, send); оранжевый = «особое состояние» (канал получает MIDI-лок, armed-запись, envelope активен).
- Красные кнопки каналов = отсутствующие сэмплы/плагины.
- Красная точка в hint = параметр автоматизируем; MIDI-вилка = линкуем.
- Выбранный mixer-трек = зелёный фейдер; playhead — вертикальная линия с треугольником.
- Peak-метр краснеет при клиппинге; CPU-метр = доля времени на аудиобуфер.

## 10. Мультитач и специальные режимы [A]

- Multi-touch: жесты масштабирования/перемещения окон, Mixer, Piano Roll/Playlist; touch-режимы тулбара (Alt/Ctrl/Shift-кнопки).
- Performance Mode: зона до Start-маркера = live-сетка; per-track: Press (Retrigger/Hold/Latch…), Motion (Stay/March…), Trigger sync.
- Typing keyboard to piano: раскладки, октавы (Alt+NumPad 2–6), snap to scale для ввода.

## 11. Известные расхождения и пробелы (честно)

- Quick quantize клипов: Ctrl+Q (playlist menu) vs Shift+Q (таблица шорткатов) — в мануале противоречие [A].
- «File menu (F3)» в menu_file.htm — артефакт (F3 = tool selector) [A/C].
- Кнопки Skip/FF в transport не описаны; «demo-режим» на тулбаре не найден; число треков Playlist не названо; список окон дефолтной раскладки отсутствует [A].
- Ripple editing: в описании slip tool упомянут Shift+S, но в таблице Shift+S = stretch mode [A].
