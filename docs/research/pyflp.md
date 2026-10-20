# PyFLP (demberto/PyFLP) — аудит исходников

Дата проверки: 2026-10-20. Репозиторий: https://github.com/demberto/PyFLP
Метод: shallow clone (`--depth 200`, полный — 252 коммита с 2023) в /tmp/oss_audit/PyFLP, только чтение. Код не запускался.

## Краткая сводка (вердикт)

- **GPL-3.0** подтверждён чтением LICENSE (полный текст GPLv3, 35 KB) и GPL-заголовков в каждом модуле. Любое копирование кода в VibeStudio запрещено — только чистая комната. [ПОДТВЕРЖДЕНО]
- Самый полный open-source **парсер** FLP: модели channel/sampler/instrument, plugin/VST-wrapper, mixer, patterns, arrangements, timemarkers; ~6.5K строк Python на construct. Но это **парсер 2023 года**: FL 2024/2025/2026-файлы не парсятся (desync на event 172, playlist-записи 80/88 байт).
- Фактически заморожен: последний содержательный коммит — середина 2023; с тех пор только pre-commit autoupdate и bump зависимостей; PyPI 2.2.1 от 2023-06-05; 39 открытых issues, майлстоун 2.3.0 висит.
- Сериализатор (`pyflp.save`) — полный ресериализм потока событий; flpkit сообщает, что такие файлы реальный FL Studio отказывается открывать. Писать через PyFLP нельзя.
- Ценность для VibeStudio: справочник «какие свойства есть у сущностей FLP» и оракул-на-старых-файлах; **не** источник кода и не основа формата.

## Метаданные

| Параметр | Значение | Источник |
|---|---|---|
| Звёзды / форки / watchers | 220★ / 32 fork / 13 | gh api repos/demberto/PyFLP, 2026-10-20 |
| Язык | Python (>= 3.8), typed | pyproject.toml |
| Лицензия (метаданные) | GPL-3.0 | gh api; pyproject |
| Создан / последний push | 2021-09-10 / 2026-02-16 | gh api |
| Последний содержательный коммит | ~2023 (затем pre-commit autoupdate, dep bumps) | git log |
| Открытые issues | **39** | gh api |
| Releases / PyPI | v2.2.1, 2023-06-05 (и GitHub, и PyPI) | gh api releases; pypi json |
| Мilestone | 2.3.0: 4 open issues; **бранча 3.0 в репо нет** (только master) | gh api milestones; git branch -a |
| Документация | pyflp.rtfd.io (Read the Docs) | pyproject urls |

## Лицензия и риски

- `LICENSE` — канонический текст GNU GPL v3 (прочитан, 35149 байт). `pyflp/__init__.py`, `pyflp/_events.py` и др. несут заголовок «Copyright (C) 2022 demberto … GNU General Public License … version 3 … or any later version». [ПОДТВЕРЖДЕНО]
- Для закрытого/коммерческого VibeStudio:
  - **Копирование кода (включая перевод на Rust как derivative work) — запрещено.** [ПОДТВЕРЖДЕНО по тексту GPL]
  - **Использование как тестового оракула — допустимо**: GPL ограничивает распространение производных работ, а не запуск; если GPL-инструмент гоняется в CI как отдельный процесс и его код не попадает в ваш дистрибутив/репозиторий продукта, derivative-связи не возникает (общепринятая интерпретация; не юридическая консультация). [ПРЕДПОЛОЖЕНИЕ]
  - **Чтение исходников при реализации своего парсера** — классический чисто-комнатный вопрос: подсмотренные «format facts» (номера opcode, layout'ы) — факты, копирование структуры/имён/кода — нет. flpdiff демонстрирует дисциплину такого подхода; VibeStudio лучше опираться на MIT-спеку flpdiff и собственные дампы, а PyFLP использовать как чёрный ящик. [ПРЕДПОЛОЖЕНИЕ / политика]
- Нюанс: `dawhubapp/PyFLP` — форк demberto/PyFLP (gh api `fork:true`), т.е. MIT-экосистема flpdiff активно сверяется с этим GPL-кодом. Учитывать при chain-of-provenance. [ПОДТВЕРЖДЕНО]

## Активность и здоровье

- 252 коммита; содержательная разработка (2.0 → 2.2.1) — март–июнь 2023; дальше в git log только `[pre-commit.ci] autoupdate`, dependabot-бампы (`typing-extensions`, `furo`), мелкие фиксы (`edfb438 fix: python 3.8 compatibility`). [ПОДТВЕРЖДЕНО]
- README-бейдж: «Maintenance: yes/2023». [ПОДТВЕРЖДЕНО]
- CI `.github/workflows/test.yml`: матрица python 3.8–3.11 + pypy3.8/3.9 × macOS/Windows/Ubuntu, tox + codecov; `codeql-analysis.yml`; `publish.yml`. Воркфлоу используют устаревшие action-версии (checkout@v3, upload-artifact@v3 — последний уже удалён из GitHub). [ПОДТВЕРЖДЕНО]
- Открытые issues (39) — тематика:
  - **Новые версии FL ломают парсер**: #208 «fix: parse FL Studio 2024+ projects (event 172 is 3 bytes, not 4)» (создан 2026-09-13, в теле — hex-dump FL 26.1.6, «The failure is silent and total»); #205 «Parse FL Studio 2024/2025 playlist items (80-byte records)» (2026-07); #200 «Missing playlist data in FL Studio 2025 project files»; #202 «Doesnt work anymore?» (2025-11); #203 Unicode Error (2026-01). [ПОДТВЕРЖДЕНО]
  - Python 3.12+ несовместимость Enum: #183, #207. [ПОДТВЕРЖДЕНО]
  - Запросы функций: Patcher (#197), Fruity Parametric EQ 2 (#159), Fruity Slicer (#158), streams для parse/save (#155).
  - Долгие «unknown»-исследования: #95/#93 (VSTPluginEvent/WrapperEvent поля), #91 (Track), #29 (Automations & controllers), #75 (музыкальные тайминги), #26, #25.
  - Прочее: #140/#119 (unlock trial-проектов — сомнительная для нас фича), #6 «Improve testing».
- Single-maintainer: автор demberto (профиль без имени/локации); репо популярно (220★) но фактически в maintenance-freeze. [ПОДТВЕРЖДЕНО]

## Сборка и зависимости

- pyproject: setuptools + setuptools_scm (версия из git-тега); runtime-зависимости: [ПОДТВЕРЖДЕНО]
  - `construct-typing>=0.5.6` — MIT (PyPI);
  - `sortedcontainers>=2.4.0` — Apache-2.0;
  - `typing_extensions>=4.6.1` — PSF-2.0;
  - `f-enum>=0.2.0` (только python<=3.10) — MIT.
  Все пермиссивные — GPL-статус PyFLP определён исключительно его собственным кодом.
- docs/requirements.txt (sphinx, furo…) — только для RTD. dev-экстра: coverage/pre-commit/pytest/tox. [ПОДТВЕРЖДЕНО]
- Python 3.8+ (после `97c656a deps: drop python 3.7 support`), но на 3.12 сломан Enum-lookup (#183/#207 открыты). [ПОДТВЕРЖДЕНО]

## Архитектура (дерево ключевых исходников)

```
pyflp/                        ~6500 строк
  __init__.py   (199)  parse(path) -> Project; save(project, path) — полный
                        ресериализм (FL_HEADER.pack("FLhd",6,format,n_channels,ppq)
                        + FLdt + bytes(event), запись total_size)
  _events.py    (601)  EventEnum(int, Enum): диапазоны BYTE=0/WORD=64/DWORD=128/
                        TEXT=192/DATA=208; U8/U16/U32/Ascii/Unicode/Indexed/
                        UnknownDataEvent; EventTree (SortedList) — событийное
                        дерево с "subtree" выборками
  _models.py / _descriptors.py / _adapters.py   property-дескрипторы над событиями
  project.py    (545)  Project: version (FLVersion), format (FileFormat enum),
                        ppq (VALID_PPQS), tempo (логика по версиям FL), main/max vol,
                        title/artists/comments (RTF!), панлай и пр.
  channel.py   (1637)  Channel и подклассы: Sampler/Instrument/Audio/Layer/
                        Automation...; уровни, питч, фейды, zipped-цвета и т.д.
  plugin.py    (1139)  PluginID, get_event_by_internal_name, VSTPluginEvent,
                        WrapperEvent (FL VST-host records)
  mixer.py      (667)  Insert: слоты, routing, colors, иконки
  arrangement.py(548)  Playlist items (PlaylistEvent, 32/60-byte записи эпох),
                        Track, DisplayGroup
  pattern.py    (383)  Pattern + notes (Note/NoteBase), паттерн-цвета
  timemarker.py ( 82)  TimeMarker (позиция/текст/сигнатуры)
  controller.py ( 76)  ControlController (automation-контроллеры)
  exceptions.py/types.py
tests/
  assets/FL 20.8.4.flp + channels/ insertpatterns/... (коммитнутые .flp)
  test_{project,channel,pattern,mixer,arrangement,plugin,events,models,corrupted}.py
docs/  (sphinx: features.rst — таблица покрытия, limitations.rst, architecture.rst)
```

Как парсится FLP [ПОДТВЕРЖДЕНО, pyflp/_events.py + __init__.py]: тот же `FLhd`/`FLdt`, event-based TLV по классическим диапазонам; события складываются в EventTree (отсортированный список с parent/child subtree). Отличие от двух других: **это конструктовые Struct-схемы** — почти каждое поле уже типизировано construct-Struct'ом; словарь имён событий самый подробный в open source (например `ArrangementID.Name` = TEXT+49, `TrackID.Name` = TEXT+47).

Поддерживаемые версии FL [ПОДТВЕРЖДЕНО по коду/тестам]: исторически широкий диапазон (код ссылается на FL 1.3.9, 3.4.0, 11, 20; фикстура тестов — FL 20.8.4); **FL 2024/2025/2026 не парсятся** (#208: event 172 читается как 4 байта вместо 3 → «the rest of the file read as garbage»; #205: playlist-записи 80 байт). Документация «as of its latest version» относится к 2023-му.

## Зеркало вопросов (issues как карта белых пятен формата)

Открытые issues PyFLP — фактически список нерешённых проблем FLP, полезный при планировании RE в VibeStudio:

| Кластер | Issues | Урок для VibeStudio |
|---|---|---|
| Event 172 / FL2024+ desync | #208 | Нестандартные размеры событий ломают парсер «тихо и тотально»; нужны self-check'и (число каналов из FLhd vs найденное) |
| Playlist-записи новых эпох | #205, #200, #199, #108 | Записи растут (32→60→80→88?) — нужен detect-stride (как у flpkit), а не версия-таблица |
| Python 3.12 Enum | #183, #207 | Косвенно: реализация на «умных» enum-ах хрупка — в Rust держать opcode-карту данными |
| Plugin-внутренности | #95, #93, #116, #190 | VST-wrapper record stream до сих пор не полностью каталогизирован |
| Автоматизация/контроллеры | #29, #193 | Модель automation в open source отсутствует — самое большое белое пятно |
| Музыкальные тайминги | #75 | Формулы (обратные PPQ-преобразования) не выведены |

## Реальная готовность функций

| Возможность | Статус | Доказательство |
|---|---|---|
| Чтение metadata (version, tempo, PPQ, title/artists, time sig, pan law) | да (уровень 2023) | project.py; tests/test_project.py |
| Чтение channels (Sampler/Instrument/Audio/Layer/Automation, уровни, питч) | да, самый детальный в OSS | channel.py (1637 стр.) |
| Чтение patterns + notes (позиция/ключ/длина/velocity...) | да; добавление нот возможно, item-level add/remove — нет | pattern.py; docs/limitations.rst «Items cannot be added or removed, only modified» |
| Чтение mixer (inserts, слоты, routing) | да | mixer.py |
| Чтение arrangements/playlist (клипы 32/60B) | да для FL ≤21; FL24+ (80B) — нет (#205, #108) | arrangement.py; issues |
| Чтение plugin state: VST-wrapper (name/vendor/FourCC/GUID/state) | да; нативные плагины — точечно (#159/#158 открыты) | plugin.py; VSTPluginEvent |
| Automation | частично (контроллеры есть, полноценных envelope-моделей нет — #29) | controller.py; issue #29 |
| Сериализация | формально да; **практически опасна** — полный ресериализм, n_channels в заголовок; flpkit: реальный FL отказывается открывать результат; docs/limitations: «always have a backup» | pyflp/__init__.py `save()`; README flpkit; limitations.rst |
| Добавление/удаление сущностей | нет (только modify; события можно добавлять вручную) | docs/limitations.rst |
| Trial-unlock | заявлен, но сломан на новых FL | #140, #119 |
| FL 2024+ файлы | **нет** (#208, #205, #200) | issues |
| Python 3.12+ | нет (#183, #207) | issues |
| Score/preset-форматы (FLm, .fst, .state) | parse принимает разные FileFormat-значения заголовка (None/Project/Score/Automation/ChannelState/PluginState/GeneratorState/FXState/InsertState/Patcher), но модели заточены под Project | codec-проверка в flpkit detect; pyflp/types FileFormat |

## Тесты и fixtures

- tests/assets: коммитнутые реальные .flp — `FL 20.8.4.flp` + каталоги channels/inserts/patterns/plugins/corrupted (модифицированные варианты). Это редкость: фикстуры не стеснены NDCA. [ПОДТВЕРЖДЕНО]
- Тесты per-model (test_channel/test_mixer/...), corrupted-тесты на битых файлах. Порог качества приличный, но покрытие «новых» версий FL нулевое — потому и лавина issue 2025–2026. [ПОДТВЕРЖДЕНО]
- codecov/codefactor/bandit/mypy — подключены, но CI-конфиг устарел (upload-artifact@v3 сломан с 2024). [ПОДТВЕРЖДЕНО]

## Ограничения / платформенные нюансы

- Документированное честное ограничение (docs/limitations.rst): «Whatever PyFLP does, is on a best-effort level»; свойства «discovered, representations aren't» (формулы volume/frequency не выведены); musical timings — issue #75; медленно из-за construct + SortedList + enum (f-enum ускорял parse на 50%). [ПОДТВЕРЖДЕНО]
- Cross-check с flpkit: pyflp использует тот же классический 4-байтовый reading события 172 → «cannot parse fresh FL 2026 saves at all» (detect.py flpkit) — совпадает с #208. [ПОДТВЕРЖДЕНО]
- Событие 172: PyFLP #208 и flpdiff говорят «3 байта»; flpkit — «1 байт». См. flpkit.md; для VibeStudio рабочая гипотеза — 3 байта (2 независимых источника + hex-dump `01 01 00`).

## Выводы для VibeStudio

1. **Код не трогать** (GPL-3.0): ни копировать, ни портировать, ни «вдохновляться» структурой при открытом окне с кодом — только чистая комната. [ПОДТВЕРЖДЕНО]
2. **Использовать как чёрный ящик-оракул** на корпусе старых файлов (FL ≤ 21): `pyflp.parse` + собственные снапшоты полей против Rust-парсера VibeStudio; запуск в CI отдельным процессом, без включения в поставку. Для FL 2024+ PyFLP как оракул непригоден (сам сломан). [ПОДТВЕРЖДЕНО ограничения / [ПРЕДПОЛОЖЕНИЕ] лицензионная схема]
3. **Как справочник моделей** — читать README/docs/issue-обсуждения (не код): таблица свойств channel/plugin/mixer показывает, какая поверхность API стоит ожидать от FLP. Номера opcode'ов можно перепроверять по MIT-спеке flpdiff и собственным дампам.
4. Практический анти-паттерн, подтверждённый этим проектом: полный ресериализатор + «best-effort» семантика записывают битые файлы. Архитектура записи VibeStudio должна быть surgery/round-trip-верифицированной (см. flpkit.md).
5. Не рассчитывать на возрождение: 3 года без фич, майлстоун 2.3.0 с 4 issues, breaking-фиксы новых FL лежат открытые с лета 2026.

## Открытые вопросы

- Почему 220★-проект с активными users-репортами (2025–2026) не получает даже 3-байтового фикса #208 — поддержка брошена или автор ждёт 3.0? В репо нет ветки/майлстоуна 3.0; планы приходилось бы искать вне GitHub (форум/дискорд). [ПРЕДПОЛОЖЕНИЕ]
- Юридически безопасная граница «читал GPL-код → реализовал своё» для единственного разработчика VibeStudio: если код PyFLP когда-либо изучался, фиксировать протокол чистой комнаты (что смотрели, когда, на что опирались — только format facts). [ПРЕДПОЛОЖЕНИЕ / политика]
- Годится ли PyFLP-фикстуры (tests/assets/*.flp) как тест-корпус: сами файлы — данные FL Studio, лицензия содержимого не указана; для внутреннего CI — ок, в открытый репо VibeStudio класть осторожно. [ПРЕДПОЛОЖЕНИЕ]
