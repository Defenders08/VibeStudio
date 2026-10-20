# interface-map.md — Карта интерфейса FL Studio

Уровень A (наблюдение) с пометками уверенности: **[A]** официальный мануал/PDF/транскрипт, **[B]** сторонний источник, **[C]** вывод. Блоки **Уровень B** — независимые спецификации для собственной реализации. Полные URL — в `sources.md`.

---

## 1. Общая организация рабочего пространства

### 1.1 Модель «плавающего стола» [A]

- Рабочее место — **набор независимых окон**: перемещаемых (с перекрытием), изменяемых по размеру, масштабируемых, «прилипающих» (dockable) и переключаемых. Нет единой главной сетки — окна живут поверх общего фона-обоев.
- Главные окна: **Playlist (F5), Channel Rack (F6), Piano Roll (F7), Mixer (F9)**; вспомогательные: **Browser (Alt+F8)**, Plugin Picker (F8), Project Picker (Ctrl+F8), Project Info (F11), MIDI/Audio settings (F10), Tempo Tapper, Edison, окна плагинов.
- Окна **сцепляются друг с другом**, если их подтащить вплотную; **Alt при перетаскивании** отменяет сцепление [A].
- Любое окно можно **отцепить (Detach)** — перенести на другой монитор; «Attached»-режим держит окно в пределах рабочего стола [A].
- Сброс раскладки: **Ctrl+Shift+H**; пресеты раскладок (Desktop default / Alternate / Tablet / Custom) в View > Layout > Arrange windows [A].
- **F12** закрывает все окна; Alt+F12 — все окна плагинов; Ctrl+F12 — все нефокусированные; Shift+F12 — выравнивает окна каналов [A]. **Esc** закрывает активное окно, **Tab** циклически обходит окна [A].
- При старте: зависит от настройки Startup project (Empty / Default template / Last used) [A]; первое знакомство — окно Welcome с недавними проектами и шаблонами [A]. Список окон дефолтной раскладки в мануале явно не перечислен — **не найдено** [A].

**Уровень B — Спецификация для собственной реализации.**
Ядро UI = оконный менеджер поверх одного «рабочего стола»:
- `WindowManager`: реестр окон `{id, title, hotkey, visible, rect, detached, z-order}`; хоткей = toggle видимости (кроме «picker»-окон — они модально-всплывающие у курсора).
- «Прилипание»: при отпускании окна в пределах N px от края другого — пристыковка (снимается модификатором).
- Пресеты раскладок: сериализованный набор {id → rect, visible}; горячая клавиша полного сброса; «закрыть все, кроме…».
- Обязательное требование UX: каждая панель/окно отвечает на Esc (закрыть) и имеет единое меню-якорь (стрелка в заголовке) с типовым набором (File/Edit/View/Snap/Detached).

### 1.2 Верхняя панель (Toolbar) [A]

Единственная постоянно видимая часть UI. Составлена из **переставляемых панелей** (правый клик по пустому месту → Toolbars: Edit/Separators/Flat buttons/Hint bar/Hide Ctrl+F11/Presets). Слева направо по умолчанию:

1. **Menu Bar**: FILE, EDIT, ADD, PATTERNS, VIEW, OPTIONS, TOOLS, HELP (состав — §3).
2. **Title & Hint Bar**: верхняя строка — лицензия + имя проекта; нижняя — **Hint Bar**: при наведении на любой контрол показывает описание; при движении — значение; справа — горячая клавиша действия; иконки-индикаторы: «красная точка» = параметр записываем/автоматизируем, «MIDI-вилка» = параметр линкуем к контроллеру, «правая кнопка» = есть контекстное меню, happy/sad face = успех/ошибка [A]. Расширенная плавающая версия — Extended Hint Panel.
3. **Window control** (свернуть/восстановить/закрыть; правый клик по Restore — скрыть панель задач ОС; по Close — меню выхода).
4. **Transport** (см. §2).
5. **Song Position LCD** — позиция воспроизведения; клик-драг меняет; показывает позицию Playlist **или** Piano Roll в зависимости от режима PAT/SONG [A].
6. **Time Panel** — формат времени с переключателем **бары:доли:шаги:тики ↔ минуты:секунды:сотые** [A].
7. **Pattern Panel** — селектор паттерна: ±-кнопки, правый клик — список всех паттернов; максимум **999**; клик по имени открывает Playlist с этим паттерном; меню: Rename/recolor (F2), Clone (Alt+C), Delete, Insert one, Split by channel, Render as audio clip, Set time signature [A]. Дублируется **Picker Panel** у Playlist.
8. **Output Panel** — осциллоскоп + спектр + пик-метр мастер-выхода (красный = клиппинг).
9. **CPU & Memory** — % времени на заполнение аудиобуфера (не % нагрузки ОС!), голоса, память; двойной клик — Plugin Performance Monitor [A].
10. **Monitor Volume & Master Pitch** — громкость мониторинга (не пишется в рендер) и глобальный сдвиг питча проекта (полутонами, влияет на инструменты и аудиоклипы) [A].
11. **Global Snap Selector** — глобальная привязка для редакторов (см. behavior-catalog §5) [A].
12. **Add** — меню добавления каналов/эффектов; **Multilink to Controllers (Ctrl+J)** — пакетная привязка MIDI-контроллеров; **Chord Panel** — распознавание аккордов MIDI-ввода; **Online/Communications** — новости, загрузки, AI-ассистент Gopher [A].
13. **Shortcut Icons** — настраиваемый набор кнопок-ярлыков: View Browser/Channel Rack/Piano Roll/Playlist/Mixer, Undo, Metronome, Count-in, Typing keyboard to piano (Ctrl+T), Step editing (Ctrl+E), Render (Ctrl+R), Save as, Auto scroll, Wait for input (Ctrl+I), Recording blend/loop, Language [A].

**Уровень B.** Тулбар = конвейер независимых «виджетов-панелей» с декларативным описанием; пользовательская перестановка и скрытие; системная строка подсказок (hint) как обязательный элемент — она снижает порог входа и служит живой документацией (описание + значение + хоткей + признаки «автоматизируем/линкуем»).

---

## 2. Transport [A]

- **PAT/SONG переключатель**: PAT — играет **только текущий паттерн** (циклически); SONG — играет **Playlist**. Клавиша **L**. Правый клик по PAT открывает Channel Rack, по SONG — Playlist [A].
- **Play/Pause**: **Space** — старт/стоп (повторное нажатие = стоп и возврат), **Ctrl+Space** — пауза [A].
- **Stop**: двойной клик = «паника» (заглушить все ноты, Ctrl+H); правый клик — «Remember seek time» (после стопа плей вернётся к выбранной позиции) [A].
- **Record (R)**: тумблер; при активной записи Play запускает запись. Правый клик — **фильтр записи**: Notes / Automation / Audio / Clips + опции (quantize ввода, отсчёт, disarm on stop, маркеры записи) [A]. Alt+клик — диалог «что записывать».
- **Темп**: LCD **10–522 BPM** (драг; у краёв ускоренный инкремент) + мелкие цифры — дробная часть с точностью **1/1000 BPM** [A]. Правый клик: **Tap** (тап-темп, в т.ч. во время игры; опции nudge±, Sync, Mute), пресеты, **Half speed** (репетиция сложных партий в половинном темпе без смены значения), Create automation clip [A].
- **Метроном** (Ctrl+M), **Precount** (Ctrl+P — отсчёт тактов метрономом перед записью), **Wait for input** (Ctrl+I — старт по первой MIDI-ноте) [A].
- Перемотка: клик по линейке Playlist; Home — в начало; NumPad 0/*// — fast-forward/такт вперёд/назад [A]. Отдельных кнопок FF/RW в transport **не найдено** [A].
- Зацикливание песни — не кнопкой, а **Time-маркерами Playlist** (тип Loop) и/или выделением области времени (проигрывается циклически) [A].

---

## 3. Главное меню [A]

| Меню | Состав (ключевое) |
|---|---|
| **FILE** | New (из последнего шаблона), New from template (Genre/Minimal/Utility…), Open (Ctrl+O), Save (Ctrl+S), Save as (Ctrl+Shift+S), **Save new version (Ctrl+N** — MyProj_2, _3…), Import, Export (WAV Ctrl+R, MP3 Ctrl+Shift+R, MIDI Ctrl+Shift+M, **All playlist tracks** — стемы, **Zipped loop package**, Project bones/data files), Recent projects (Alt+1..0), Revert to last backup, Exit |
| **EDIT** | Контекстно-зависимое: Undo (Ctrl+Z / Ctrl+Alt+Z пошаговый), Cut/Copy/Paste для выделенного (например, каналов) |
| **ADD** | Channel (инструмент), Effect (в микшер), More plugins…, Plugin picker, Manage plugins, **Automation for last tweaked parameter**, Pattern |
| **PATTERNS** | Переименование/цвет/иконка, Clone (Alt+C), Delete, Insert one, Move up/down, **Split by channel**, Find next empty (F4), Set time signature, Transpose…, Render as audio clip / Render and replace, группы паттернов |
| **VIEW** | Окна (F5–F11, Browser, pickers, Tempo tapper, Edison), Layout (F12, Alt+F12, Ctrl+F12, Shift+F12, Arrange windows…), Toolbars, Browser-секции (Undo history, Patterns, Generators/Effects in use), Background (обои) |
| **OPTIONS** | MIDI settings (F10), Audio settings, General settings (масштаб UI, анимации, поведение плагинов, Default/Startup template), File settings (папки Browser, VST-пути), Theme settings, Project general settings (размер, PPQ), MIDI-свитчи (Typing to piano Ctrl+T, Metronome Ctrl+M, Precount Ctrl+P, Wait for input Ctrl+I, Blend record Ctrl+B, Step edit Ctrl+E, Loop record, Auto scrolling, Multilink Ctrl+J) |
| **TOOLS** | Browser find (Ctrl+F), One-click audio recording, Riff machine, Chord progression tool, Macros (Panic Ctrl+H, Prepare for performance mode, Purge unused audio clips, Select unused channels, Switch smart disable…), **Dump score log to selected pattern** |
| **HELP** | Справка (F1, контекстная), Gopher (AI-ассистент), Tutorials (интерактивные уроки), Registration, About |

**Уровень B.** Меню плоское и «плоское по смыслу»: всё, что касается содержимого проекта (паттерны, каналы, клипы) — верхнего уровня; всё системное — OPTIONS; «умные» операции — TOOLS/Macros. EDIT намеренно куцый, потому что честный Undo/Redo и буфер живут в каждом редакторе, а глобальный Undo покрывает «твики» параметров.

---

## 4. Playlist (F5)

### 4.1 Структура [A]

- Многоцелевые **Clip-треки**: любой тип клипа на любом треке, наложение разрешено (клипы рисуются полупрозрачно) [A]. Треки не привязаны к микшеру [A]. Число треков в мануале **не найдено** [A].
- **Track Modes** (опционально, правый клик по заголовку): Unassigned / **Audio track** (трек связан с insert-треком: drop аудио авторутится, запись с этого входа ложится на трек) / **Instrument track** (трек связан с инструментальным каналом + своим паттерном) [A].
- Заголовок трека: имя, цвет (F2/случайный), иконка, Mute (клик; Ctrl+клик/правый — Solo; Shift — lock), высота (Alt+клик по разделителю — сброс) [A].
- **Группы треков**: «Group with above track» (sub-tracks, схлопываются), mute/solo группой через Alt+клик [A].
- **Lock to content** — трек запирается на первый клип: клик всегда вставляет именно его (эмуляция «инструментальных» треков) [A].
- Слева — **Picker Panel** (Alt+P): список всех паттернов и аудио/автоматизация-каналов; клик — выбрать источник, клик по полю — поставить; drag тоже работает [A].
- **Arrangements** — несколько аранжировок в одном проекте (клонировать, объединять); делят каналы и микшер [A].

### 4.2 Типы клипов [A]

| Тип | Что содержит | Как создаётся |
|---|---|---|
| **Pattern Clip** | ноты + event-автоматизация паттерна (слоями) | Picker Panel / Draw; Alt+двойной клик — Channel Rack, двойной — Piano Roll (настраивается) |
| **Audio Clip** | аудио (спец-версия Sampler-канала) | перетащить файл из Browser в Playlist |
| **Automation Clip** | огибающая параметра (генератор-канал) | правый клик по параметру → Create automation clip |

Цвет клипа-паттерна переключается: ноты/канал/паттерн [A].

### 4.3 Инструменты редактирования (буквы — горячие клавиши) [A]

| Клавиша | Инструмент | Поведение |
|---|---|---|
| P | Draw (карандаш) | ЛКМ — поставить клип; ПКМ — удалить; Shift — «кисть»; Ctrl — выделение |
| B | Paint (кисть) | drag — серия копий источника; Shift+drag — копия выделения |
| D | Delete | стирание кликов/drag |
| T | Mute | клик — заглушить клип (не удаляя) |
| S | Slip edit | сдвиг **содержимого** клипа при сохранении границ |
| C | Slice | разрез клипа по вертикали; Left-Shift — по диагонали |
| E | Select | прямоугольное выделение; Shift — добавить/убрать |
| Z | Zoom (Shift+Z/Z) | зум к области |
| Y | Playback | клик — проиграть с этой позиции |

Прочее: Shift+drag — **копия** клипа; Ctrl+B — дублировать вправо; Alt+двойной клик — свойства/Channel Rack; Make unique (Shift+V+drag) — независимый клон; Shift+G/Alt+G — группировка клипов; Shift+M — режим time-stretch при растяжении аудио; растяжение за края клипа (клип = «окно» над данными) [A].

### 4.4 Время, маркеры, зум [A]

- Линейка тактов сверху; **Ctrl+drag по линейке** — выделение области времени (играется циклически, участвует в рендере); Ctrl+B с таким выделением задаёт шаг дублирования; **Ctrl+Ins / Ctrl+Del** — вставить/удалить время в выделении (сдвигая остальное) [A].
- **Time-маркеры** (Alt+T/Ctrl+T): типы None/Start/**Loop**/**Skip**/**Pause**/Time signature/Start-Stop recording; маркеры задают точки возврата и прыжков playback'а [A].
- Зум: Ctrl+колесо, PgUp/PgDn, Shift+1..4 — уровни, Shift+5 — по выделению, Shift+0 — центр на playhead; средняя кнопка — панорама [A].

**Уровень B.** Timeline-модель: `Arrangement → Track[] → Clip[]` без ограничений типов; клип = {источник, старт, длина, mute, оффсеты окна}; источник хранит «полную» длину, клип лишь показывает окно. Обязательные операции: draw/erase одним кликом, dup-вправо по выделенному интервалу, insert/delete времени, slip, slice, freeze (консолидация в аудио), маркеры-команды playback'а (loop/skip/pause), и отдельный слой «аудио-треков с входом» для записи.

---

## 5. Piano Roll (F7)

### 5.1 Структура окна [A]

- **Клавиатура слева**: ЛКМ — прослушать ноту; Ctrl+ЛКМ/drag — выделить диапазон по высоте; СКМ-drag — вертикальный зум; ПКМ — меню (переименовать клавишу, свои подписи) [A].
- **Нотное поле**: время по горизонтали (линейка тактов сверху), высота по вертикали; **слайд-ноты** — полоска с треугольником [A].
- **Панель свойств снизу** (Event editor): колонки-«леденцы» свойств нот (Velocity, Pan, Release, Mod X/Y, Pitch) или event-автоматизация; выбор цели — Target Control / Shift+F [A].
- **Target Channel** (dropdown в заголовке): чей ролл редактируем; каналы с данными в паттерне отмечены; колесо над контролом листает; клавиши **G/H/J/K** — соседние каналы (H/J — только с нотами) [A].
- **Ghost notes** (Alt+V): полупрозрачные ноты **других каналов того же паттерна** (и других паттернов при перекрытии клипов); editable-режим; двойной ПКМ по ghost — перейти на её канал [A].
- **Цветовые группы нот**: 16 цветов (= 16 MIDI-каналов); выделение по цвету (Shift+C), смена цвета (Alt+C); группировка нот Shift+G/Alt+G [A].

### 5.2 Редактирование нот [A]

- **Draw (P)**: ЛКМ — нота (длина по умолчанию = последней тронутой); drag — перемещение; за правый край — длина (левый край — только после включения опции); ПКМ — удалить; **Alt — обход snap**; Shift+drag — копия; Shift после клика — запрет вертикали, Ctrl — запрет горизонтали.
- **Paint (B)** — серия нот при drag (монофонично; сверху/снизу — аккорды); **Paint drum (N)** — для перкуссии; **Delete (D)**, **Mute (T)** — глушение без удаления; **Slice (C)** — разрез; Ctrl+G — склейка; **Select (E)**, **Zoom (Z)**, **Playback (Y)** — как в Playlist.
- **Stretch-хэндл** выделения: drag растягивает группу нот по времени [A].
- Выделение: Ctrl+drag — прямоугольник; Magic lasso (обвести без клика); по линейке времени; по клавиатуре (по высоте); Shift+I — инвертировать; Ctrl+Enter — время вокруг выделения [A].
- **Свойства ноты**: двойной клик — диалог (pan/vel/release/modx/mody, slide/porta, цвет, точное время/длительность); Alt+колесо над нотой — менять текущее свойство [A].

### 5.3 Инструменты и помощники [A]

- **Tools-меню**: Quantize (Alt+Q) / Quick quantize, Chop (Alt+U)/Quick chop (Ctrl+U), Glue (Ctrl+G), Quick legato (Ctrl+L), Articulate (Alt+L), Strum (Alt+S), Flam (Alt+F), Arpeggiate (Alt+A), Randomize (Alt+R), Scale levels (Alt+X), LFO (Alt+O), Limit (Alt+K), Flip (Alt+Y), Claw machine (Alt+W), **Riff Machine** (Alt+E — пошаговый генератор риффов), **Chord Progression Tool** (Alt+P), Stamp (аккорды/гаммы).
- **Snap to scale**: ЛКМ — вкл/выкл; ПКМ — тоника+гамма; ноты прилипают к гамме при переносе; опция «Snap incoming MIDI» [A].
- **Scale highlighting** + **Note grid highlights** (Alt+B) — подсветка тоники/гаммы на фоне; режим Automatic детектирует гамму по нотам [A].
- **Слайд-ноты (S)** и **портаменто (O)**: слайд-нота, перекрывая обычную, тянет её высоту (и другие свойства) к себе; работает **только с нативными инструментами** [A].

**Уровень B.** Ролл = 2D-сетка {pitch × time(PPQ)} + слой per-note параметров + слой event-кривых. Ключевые контракты: «длина новой ноты = последней тронутой», «клип-окно» не нужен — данные канала на весь паттерн; ghost-каналы как read-only подложка (переключение по двойному ПКМ); цветовая группа = одновременно визуальная grouping и MIDI-канал; инструменты — конечный автомат «активный инструмент × модификаторы (Alt/Shift/Ctrl)».

---

## 6. Channel Rack / Step Sequencer (F6)

### 6.1 Структура окна [A]

Строка канала (слева направо): **Mute LED → Pan → Volume → Mixer Track routing → Channel button → Channel Selector → Step-клетки или Piano Roll preview** [A].

- **Канал** = один инструмент/сэмпл/генератор. Все паттерны используют один и тот же набор каналов [A].
- **Step Sequencer**: клетки = 16-е доли; по умолчанию **16 шагов (1 такт 4/4)**; ЛКМ — включить, ПКМ — выключить; длина паттерна 1–512 шагов (тянуть правый нижний угол rack); у шагов **нет note-off** [A].
- **Graph Editor (Ctrl+K)**: per-step значения Note (pitch), Velocity, Release, Fine Pitch, Pan, Mod X (cutoff), Mod Y (resonance), Shift (сдвиг старта), Rep (ретриггер) [A].
- **Piano roll preview**: если канал содержит ролл, клетки заменяются миниатюрой; ПКМ по ней — открыть Piano Roll в позиции клика [A].
- **Swing**: глобальная крутилка (удлиняет нечётные 16-е) × канал-множитель (Misc > Swing); свинг активен, если **оба > 0** [A].
- **Pattern selector** в шапке + Shift+колесо по фону заголовка — листать паттерны [A].
- **Display Filter** — фильтр-группы каналов (например, «Drums»), PageUp/PageDown [A].

### 6.2 Кнопка канала и меню [A]

- **ЛКМ** — открыть Channel Settings + интерфейс плагина (новый открывается вместо предыдущего; Alt+ЛКМ — открыть, не закрывая старый; Ctrl+ЛКМ — preview звука без открытия) [A].
- **ПКМ — меню канала**: Piano roll, Graph editor, Rename/color/icon, Load sample, Cut self (нота срезает предыдущую самого канала — для «самозаглушения»), Insert/Replace/Clone/Delete, Cut/Copy/Paste данных, Fill each 2/4/8 steps, Advanced fill, Rotate left/right, **Receive notes from** (лок MIDI-контроллера к каналу), Create DirectWave instrument, Burn MIDI to pattern [A].
- **Cut/Cut by группы** (Misc-вкладка): реализация open/closed hi-hat — один канал глушит другой [A].

### 6.3 Channel Settings (вкладки) [A]

- **Main**: mute/solo, PAN/VOL, PITCH/RANGE, **Mixer track selector** («---» = Master; клик по TRACK — auto-assign; Ctrl+L — route to free mixer track) [A].
- **Sampler (SMP)**: waveform с маркерами Start/Loop (тянутся), Keep on disk, loop points + ping-pong, declicking, **time-stretch/pitch-shift** (Realtime / Elastique e3 / Special / Legacy; PITCH, MUL, TIME с автодетектом темпа), **Precomputed-эффекты** (normalize, fade, reverse, EQ, reverb, POGO…) [A].
- **INS (Instrument/Envelope)**: 4 огибающие (объём, cutoff, resonance, pitch) + LFO, фильтр, preview-клавиатура [A].
- **MISC**: полифония, **SLIDE/Porta/Mono** (слайды), Gate, Shift/**Swing**, **Cut/Cut by**, root note и диапазон клавиш [A].
- Типы каналов: Sampler, Audio Clip (только SMP+MISC), Automation, Layer, Generator (плагин в wrapper) [A].

**Уровень B.** Данные: `Pattern → Channel[] → (steps bitset | note[] + events[])`. Rack = «микшер ролей»: одна строка UI агрегирует mute/pan/vol/routing/данные. Step Sequencer — лишь проекция нот длины-в-шаг; конвертация steps↔roll обязана быть бесплатной (открытие ролла по каналу и обратно Shift+D). Cut-groups — простая и мощная замена sidechain-логики на нотном уровне.

---

## 7. Mixer (F9)

- **Состав**: Master + **500 insert-треков** + служебный **Current (C)** (хостит Edison/Wave Candy; применяется к выбранному треку — «ловушка» для новичков) [A]. Исторические числа (103/125) встречаются в старых текстах [B/C]. Отдельных send-треков **нет** — любой insert может быть send'ом [A].
- **Элементы трека**: пик-метры, Mute/Solo, **Pan**, **Fader** (post-fader), **Stereo Separation** (вправо → моно, влево → расширение), арметрия моно не предусмотрена [A].
- **10 FX-слотов** на трек: клик по стрелке — добавить из favorites; клик по имени — открыть GUI; LED — вкл/выкл; колесо над слотом — переставить; общий переключатель FX трека [A].
- **Интегрированные на каждом треке**: 3-полосный параметрический EQ, фаза, swap, stereo separation, volume/pan (в Track inspector справа) [A].
- **Маршрутизация**: у каждого insert по умолчанию send в Master. **Клик по send-переключателю** внизу трека-приёмника — создать/убрать send; появляется ручка Send Level. **ПКМ по переключателю → Sidechain only** (send уровня 0 для плагинов с sidechain-входом). Меню Track routing: Route to this track (only), **Create submix**, Route selected channels (Ctrl+L / Ctrl+Shift+L) [A].
- Feedback-петли блокируются: недоступные send'ы рисуются серыми [A].
- **Save/Load mixer track state** — сохранить всю цепочку трека как пресет; drag иконки состояния на другой трек — скопировать [A].
- Переупорядочивание: Alt+←/→ или Shift+колесо; доки слева/справа; группы треков с сепараторами и автопокраской [A].

**Уровень B.** Аудио-граф: `Channel → (один) Insert → send-мультиграф → … → Master → устройство`. Send = {from, to, level, post/pre}; sidechain = {level:0}; валидатор запрещает циклы. Состояние трека сериализуемо целиком (имя, цвет, слоты, EQ, маршрутизация) — это даёт «freezing пресетов цепочек».

---

## 8. Browser (Alt+F8)

- Дерево с **вкладками**: All, Current Project, Plugin Database, Online Content, Starred (избранное), Sounds (FL Cloud); свои вкладки и свои папки (до 30 extra search folders, зелёным цветом) [A].
- **Current Project**: History (undo-стек — клик = откат!), Patterns, Initialized controls, **Generators/Effects in use** (все автоматизируемые параметры с меню Create automation clip / Link to controller), Samples [A].
- **Plugin Database**: favorites-структура папок, которая **питает** Plugin Picker (F8) и все списки добавления; «Add to plugin database» — регистрация VST [A].
- **Preview**: ЛКМ по аудио — до 5 секунд; Alt+ЛКМ — целиком; синхронизация с темпом/тональностью проекта, если у wav есть метаданные [A].
- **Поиск**: Ctrl+F (wildcards, AND/OR/NOT), теги, фильтры по типу [A].
- **Drag-and-drop всюду**: сэмпл в Rack = Sampler-канал; в Playlist = Audio Clip; в микшер-трек = инструмент с авторутингом; ПКМ-меню: Send to selected channel, Open in new channel, Edit in Edison, Detect tempo, Favorite, Locate, Delete [A].
- **Shift+↑/↓** — горячая переборка сэмплов папки в выбранном канале (хот-своп на слух) [A].

**Уровень B.** Контент-браузер = объединённый реестр {файловая система + БД плагинов + содержимое проекта + избранное/теги}. Ключ: один и тот же drop-жест диспетчерится по **цели** (rack/playlist/mixer), а не по типу файла; preview с автосинком темпа; undo-history как browsable сущность.

---

## 9. Окна инструментов и плагинов

- **Channel Settings** (обёртка канала) + **GUI плагина** открываются вместе с кнопки канала [A].
- **VST Wrapper**: меню wrapper, пресет-селектор, настройки обработки (мульти-выходы, PDC), Program list [A].
- **Plugin Picker (F8)**: всплывающая таблица favorites (генераторы слева, эффекты справа) — клик-и-драг иконки на цель, ввод с клавиатуры фильтрует [A].
- **Edison** — рекордер/редактор в FX-слоте (Shift+E — авто-загрузка на выбранный трек); Send to Playlist as Audio Clip [A].
- Опции поведения: Auto select linked modules (один открытый GUI плагина за раз), лимит открытых редакторов 25, Detach на второй монитор [A].

**Уровень B.** Плагины — «гости»: для собственных инструментов GUI-обязательства минимальны (типовый хост-фрейм: пресеты, randomize, меню, parameter-меню с Create automation clip/Link — см. interaction-model §6).

---

## 10. Редакторы автоматизации

- **Event Editor** — ступенчатые «события-значения» одного параметра внутри паттерна; открытие: ПКМ по параметру → Edit events; инструменты Draw/Brush/Erase/Interpolate; LFO-инструмент (Ctrl+L) [A].
- **Automation Clip** — огибающая в Playlist: ПКМ — точка; типы сегментов (кривая single/double, **Hold**, Stairs, Pulse, Wave…); режимы Step/Slide; LFO-поверх; окно клипа растягивается [A].
- Создание клипа: ПКМ по параметру → Create automation clip; для VST — «Last tweaked»; из Event Editor — Turn into automation clip; Initialized controls хранят стартовые значения [A].
- **Link to controller** (ПКМ по параметру): MIDI-learn («подвигай ручку»), формулы маппинга, smoothing; Multi-link (Ctrl+J) для пакетной привязки [A].

**Уровень B.** Две природы автоматизации: pattern-bound event-данные (записываемые) и клип-огибающие (рисуемые) с односторонней конвертацией event→clip. Каждый параметр несёт метаданные: диапазон, дефолт, «автоматизируем/линкуем»; «last tweaked» — обязательный буфер последнего параметра для меню ADD.

---

## 11. Прочие окна

- **Project Info (F11)**: метаданные проекта; **Project General settings**: размер такта (time division vs time signature), **PPQ (по умолчанию 96)**, data folder [A].
- **Tempo Tapper**, **Welcome-окно**, **Plugin Performance Monitor**, **Macros** (Tools) [A].
- **Performance Mode (Ctrl+P)**: зона до Start-маркера = сетка live-триггеров клипов (один клип на трек; per-track Press/Motion/Sync) [A].
- **Gopher** — встроенный AI-ассистент по мануалу (Alt+F1) [A].

## 12. Сводка «какие окна что редактируют»

| Окно | Объект редактирования |
|---|---|
| Channel Rack | данные каналов **текущего паттерна** (шаги), параметры каналов |
| Piano Roll | ноты и per-note свойства **одного канала текущего паттерна** |
| Event Editor | event-автоматизация **внутри паттерна** |
| Playlist | аранжировка из клипов + Automation Clips + аудио |
| Mixer | аудио-граф: уровни, панорама, эффекты, маршрутизация |
| Channel Settings/Wrapper | звук одного канала (сэмплер/плагин) |
| Browser | контент, плагины, содержимое проекта (undo, параметры) |
