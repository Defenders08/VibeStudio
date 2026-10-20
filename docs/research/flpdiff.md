# flpdiff (dawhubapp/flpdiff) — аудит исходников

> **ЭРАТТА 2026-10-20 (вечер):** таблица «Реальная готовность функций» ниже цитирует карту opcode'ов из первого прохода, и часть атрибуций в ней неверна (это упрощения, замеченные при сверке со спекой): `0x40` = маркер нового канала (не имя; имя канала — scope-sensitive `0xCB`), `0x41` = id-маркер паттерна (срабатывает дважды; имя паттерна — `0xC1`, цвет — `0x96`), `0xCC` = имя insert'а (не слоты; слоты закрывает `0x62`, группами по 10), `0x93` = закрытие insert'а с output routing, `0xD4`/`0x16` в спеке отсутствуют. Выверенная карта: [flp-format-notes.md](flp-format-notes.md).

Дата проверки: 2026-10-20. Репозиторий: https://github.com/dawhubapp/flpdiff
Метод: shallow clone (`--depth 200`, полный клон — 166 коммитов) в /tmp/oss_audit/flpdiff, только чтение. Код не запускался и не собирался.

## Краткая сводка (вердикт)

- MIT-лицензия подтверждена чтением LICENSE — код можно копировать в VibeStudio (в т.ч. в закрытый коммерческий продукт). [ПОДТВЕРЖДЕНО]
- Это самый зрелый на сегодня open-source FLP-стек: parser + writer (mutations) + semantic diff + clean-room спецификация формата; заявлена поддержка FL 9–25 на чтение, мутации таргетируют FL 25.
- Спецификация `docs/fl-format/flp-format-spec.md` — главная ценность для VibeStudio: ~40 подтверждённых opcode'ов с уровнями доверия (confirmed/observed/unknown), история RE-находок, clean-room граница.
- Активный, но очень молодой проект (апр–июль 2026, один автор); после 2026-07-04 коммитов на main нет. 0.1.x, API нестабилен.
- Риск: организация dawhubapp держит **форк GPL-проекта PyFLP** — clean-room заявления specs надо принимать с оговоркой, а при копировании кода flpdiff в VibeStudio перепроверять файлы (код MIT, но репо соседствует с GPL-форком).
- Как тестовый оракул в CI — идеально: CLI с exit-кодами 0/1/2, standalone-бинарники без рантайма.

## Метаданные

| Параметр | Значение | Источник |
|---|---|---|
| Звёзды / форки | 12★ | gh api repos/dawhubapp/flpdiff, 2026-10-20 |
| Язык | TypeScript (Bun, ESM) | package.json |
| Лицензия (метаданные) | MIT | gh api |
| Создан / последний push | 2026-04-26 / 2026-10-08 | gh api |
| Последний коммит на main | 2026-07-04 (`28aae8f`) | git log; gh api /commits/main |
| Открытые issues | 0 | gh api |
| Releases / tags | 3 (v0.1.0–v0.1.2), бинарники 5 платформ | gh api releases/latest |
| npm | `flpdiff` latest **0.1.2** (в репо уже 0.1.3, на npm не опубликован); `@dawhub/flp-parser` **не опубликован** | npm view, 2026-10-20; CHANGELOG.md «publish guard» |
| Размер репо | ~4.7 MB (включая .flp-фикстуры) | задача пользователя |

## Лицензия и риски

- `LICENSE` (корень) и `packages/flp-parser/LICENSE`: стандартный текст MIT, «Copyright (c) 2026 Roman Pronskiy». Прочитан целиком, совпадает с MIT-шаблоном. [ПОДТВЕРЖДЕНО]
- Следствия для VibeStudio:
  - Копирование кода (перевод на Rust, дословный порт) в закрытый коммерческий продукт — разрешено, единственное условие — сохранить copyright-уведомление. [ПОДТВЕРЖДЕНО по тексту лицензии]
  - Использование как тестового оракула — безусловно допустимо.
- Нюанс: та же организация `dawhubapp` держит `dawhubapp/PyFLP` — **форк demberto/PyFLP (GPL-3.0)** (gh api: `fork:true, parent:demberto/PyFLP`, создан 2026-04-27). [ПОДТВЕРЖДЕНО]
  - Спека flpdiff явно декларирует clean-room: «No code was copied from any GPL FLP library» + правило «cite format facts, never code» (`docs/fl-format/flp-format-spec.md`, раздел Clean-room boundary). [ПОДТВЕРЖДЕНО]
  - Для VibeStudio это значит: текст specs формата читать безопасно, но при заимствовании кода флп-стека из этого org проверять LICENSE каждого файла; и не смешивать их GPL-форк в свой pipeline.
- Runtime-зависимость `typed-binary` ^4.3.3 — MIT (npm view, v4.3.4), zero-dep. [ПОДТВЕРЖДЕНО]

## Активность и здоровье

- 166 коммитов, все — Roman Pronskiy <roman@pronskiy.com> (git log, uniq -c: единственный автор). Single-maintainer. [ПОДТВЕРЖДЕНО]
- Темп: старт 2026-04-25, пик мая 2026 (17 коммитов 05-09), последний на main — 2026-07-04; pushed_at 2026-10-08 (веток, кроме main, API не показывает — вероятно, правки вне main/метаданные). Формально проект «спит» с июля. [ПОДТВЕРЖДЕНО]
- CI: `.github/workflows/ci.yml` — bun test + tsc на ubuntu-latest и macos-latest (Windows в тестовой матрице отсутствует, хотя win-бинарник собирается). [ПОДТВЕРЖДЕНО]
- `.github/workflows/release.yml` — при теге `v*` собирает standalone-бинарники через `bun build --compile` для darwin-arm64/x64, linux-x64/arm64, windows-x64 и публикует GitHub Release. Релиз v0.1.2 содержит все 5 ассетов. [ПОДТВЕРЖДЕНО]
- Issues: 0 открытых. Значимый закрытый: issue #1 (Holzchopf) — исправил классификацию FL 25 banner (`0xAC` = 3 байта, а не UTF-16-событие) — упомянут в changelog спеки. [ПОДТВЕРЖДЕНО]
- Организация `dawhubapp`: 3 репо — flpdiff, flstudio-mcp («MCP server for FL Studio. Let Claude, Cursor or any MCP client read and edit your FL Studio projects»), форк PyFLP. Продукт «dawhub» публично не находится (WebSearch от 2026-10-20 — пусто) → [ПРЕДПОЛОЖЕНИЕ]: приватный продукт/AI-инструментарий вокруг FL Studio одного разработчика. Roman Pronskiy известен по PHP-сообществу (pronskiy.com из package.json). [ПРЕДПОЛОЖЕНИЕ]

## Сборка и зависимости

- Рантайм: Bun >= 1.3 (engines, package.json). Модули `.ts` импортируются напрямую без сборки; для npm-потребителей вне Bun это нетипично. [ПОДТВЕРЖДЕНО]
- dependencies: `typed-binary` ^4.3.3 (MIT) — единственная runtime-зависимость; peer: `typescript` ^5; dev: `@types/bun`, workspace `@dawhub/flp-parser`. [ПОДТВЕРЖДЕНО]
- `packages/flp-parser` — фасад, реэкспортирует `flpdiff/parser`; его зависимость на `flpdiff` в репо указывает на `file:../..`, и CHANGELOG 0.1.3 прямо предупреждает, что перед `bun publish` её надо заменить на semver — то есть пакет ещё фактически не публикуем. [ПОДТВЕРЖДЕНО]
- Скрипты: `test` (bun test), `typecheck` (tsc --noEmit), `cli`, 6 вариантов `build:binary:*`. [ПОДТВЕРЖДЕНО]

## Архитектура (дерево ключевых исходников)

```
src/
  parser/
    primitives.ts        # схемы typed-binary: u8/u16/u32/varint/utf16-zterm
    event.ts             # FLPEventSchema: opcode-диапазоны 0x00-0xFF, blob для DATA
    flp-project.ts       # parseFLPFile: FLhd/FLdt -> header + events[]
    project-builder.ts   # события -> модель (channels/patterns/mixer/arrangements)
    flp-write.ts         # serializeFLPProject: полный рерайт, round-trip byte-exact
    vst-wrapper.ts       # декодер VST-wrapper blob (0xD5): name/vendor/path/FourCC
    errors.ts            # FLPParseError с byte-offset контекстом
  model/                 # metadata, channel, pattern, arrangement, mixer-insert
  diff/                  # comparator, matcher, note-diff, mixer-diff,
                         # arrangement-diff, automation-diff, colorize, headline
  mutations/index.ts     # ~3400 строк, ~40 экспортированных мутаций (writer)
  reorganize/, synth/    # planReorganize; .fst-extractor и синтез фикстур
  bridge.ts              # JSON-RPC (stdin/stdout) для flstudio-mcp
  cli.ts, info.ts, summary.ts, canonical.ts, git.ts
packages/flp-parser/     # npm-фасад
docs/fl-format/          # flp-format-spec.md (517 строк), fl25-event-format.md (618),
                         # plugin-layout-registry.md (213), fst-format.md (120)
tests/                   # 30 файлов, 490 test()-вызовов; corpus/
```

Как парсится FLP [ПОДТВЕРЖДЕНО, src/parser + spec §5]: chunk-based envelope (`FLhd` 14 байт, `FLdt`) + event-based TLV-поток: opcode-байт, размер по диапазону (<0x40 — 1 байт, <0x80 — 2, <0xC0 — 4, иначе varint-LEB128 + payload), плюс FL25-оверрайды (например `0xAC` = 3 байта). Все события хранятся с оригинальными payload-байтами; неизвестные opcode'ы проходят как `blob` — отсюда гарантия round-trip «serializeFLPProject(parse(bytes)) == bytes» (комментарий в `flp-write.ts`; гарантию в RL не проверяли — код не запускался).

Версии FL: чтение — FL 9…25 (badge README; parity-свип на 85 локальных файлах FL 9/11/12/20/21/24/25 — 85/85 Pass 1, 83/85 Pass 2, `docs/fl-format/flp-format-spec.md`, раздел Parity). Запись (mutations) — только FL 25: `encodeClipRecord` пишет 80-байтовые записи безусловно («v1 targets FL 25 only», CHANGELOG 0.1.3); `setTempo` кидает `LEGACY_TEMPO_FORMAT` на legacy-файлах (src/mutations/index.ts). [ПОДТВЕРЖДЕНО]

## Перекрёстные находки (важно для VibeStudio)

Три независимых проекта расходятся/сходятся в деталях формата — сводка на 2026-10-20:

| Факт | flpdiff | flpkit | PyFLP | Комментарий |
|---|---|---|---|---|
| Event 172 (`0xAC`) payload | **3 байта** (spec §2.2, issue #1) | **1 байт** (detect.py fallback) | **3 байта** (issue #208, hex `01 01 00`) | 2 против 1; «stream-consistency» flpkit не различает интерпретации при payload `01 01 00` |
| Playlist clip record | 80 B FL25 / 60 B FL21–24 / 32 B ранее (`0xE9`) | 32/60/80/**88 B**, stride детектируется | 32/60 известны; 80 B — открытый issue #205 | 88 B (flpkit, FL 2026) требует сверки с flpdiff-файлами |
| FL25-релокации opcode'ов | notes 0xD0→0xE0, track 0xDE→0xEE, Levels 0xCB→0xDB, insert flags 0xDC→0xEC (spec, changelog) | Levels=219 (0xDB), notes=224 (0xE0) — совпадает | не поддерживает FL25 (issues) | консистентно между flpdiff и flpkit |
| Запись файлов | полный рерайт + round-trip guarantee | raw surgery + verify-by-readback | полный рерайт (считается опасным) | flpkit обвинения против PyFLP-save; живой FL — арбитр |
| Tempo legacy | 0x42+0x5D fallback (FL <3.4.0) | «legacy pre-156 coarse/fine word pair» — совпадает (156=0x9C) | tempo-логика по версиям в project.py | консистентно |

## Реальная готовность функций

| Возможность | Статус | Доказательство |
|---|---|---|
| Чтение tempo/PPQ | да, + legacy 0x42/0x5D fallback | spec §3.1 `0x9C/0x42/0x5D`; mutations `setTempo` |
| Чтение channels (имя, тип, цвет, sample path, Levels pan/vol/pitch, routing) | да | spec `0x40/0x15/0x80/0xC4/0xDB/0x16`; src/model/channel.ts |
| Чтение patterns (имя, цвет, длина, looped, notes 24B, controllers 12B) | да | spec `0x41/0xC1/0xA4/0x1A/0xE0/0xDF`; src/model/pattern.ts |
| Чтение playlist/arrangements (clips, 80/60/32-байтовые записи) | да, FL25=80 байт (история бага 60→80) | spec `0xE9/0x63/0xF1/0xEE`; CHANGELOG 0.1.3 |
| Чтение mixer (inserts, слоты, имена, цвета, routing, MixerParams 0xE1) | да, атрибуция полная | spec `0x93/0xCC/0xE1/0xEC/0xE7`; project-builder.ts |
| Plugin state (0xD5): VST-wrapper name/vendor/path/FourCC | да (state-блоб остаётся opaque) | spec `0xD5`, src/parser/vst-wrapper.ts |
| Нативные плагины (EQ2 и т.п.) | state opaque; параметры — через реестр layout'ов (Fruity Limiter, Fruity Reeverb 2) | docs/fl-format/plugin-layout-registry.md; commits 7d..9a4f5fe |
| Automation | ключевые кадры (0xDF pattern controllers, automation-diff) | src/diff/automation-diff.ts; spec `0xDF` |
| Diff-движок (semantic diff, git textconv, exit-коды CI) | да, ядро продукта | README; src/diff/*; docs/parity-gaps.md |
| Запись: tempo, имена, цвета, time sig | да (FL25) | src/mutations/index.ts: setTempo/setPatternName/... |
| Запись: notes (add/remove/transpose/quantize/humanize/reverse/invert) | да | mutations: setPatternNotes и др. |
| Запись: playlist clips (add/remove/move), arrangeSong | да (FL25, 80B) | mutations: addClip/removeClip/moveClip/arrangeSong |
| Создание channel/pattern, instantiateNativePlugin (best-effort) | частично | mutations: createChannel/createPattern/instantiateNativePlugin |
| .fst-пресеты: извлечение и сплайс в .flp | да (эксперим. F9.x) | docs/fl-format/fst-format.md; src/synth/extract-fst.ts |
| Автоматизация: запись envelope | нет | spec §6 «Write path — non-goal»; automation только чтение/diff ключевых кадров |
| Plugin state запись (VST chunk) | только сплайс готовых .fst | mutations F9.2 |
| Time markers (позиция/сигнатуры/текст) | чтение | spec `0x94/0x21/0x22/0xCD` |
| git textconv-интеграция (`flpdiff git-setup`) | да | README; src/git.ts; tests/git.test.ts |
| JSON-мост для агентов (`flpdiff bridge`, stdin/stdout JSON-RPC) | да | src/bridge.ts; read kinds describe/get_tempo/list_* |
| Стandalone-бинарники 5 платформ без рантайма | да (GitHub Releases) | release.yml; релиз v0.1.2 |

## Тесты и fixtures

- 30 тестовых файлов, 490 `test()`-вызовов (grep -c). Домены: flp-write, mutations, patterns, channels, mixer, arrangements, diff-модель, git textconv, cli. [ПОДТВЕРЖДЕНО]
- Коммитнутые фикстуры: `tests/corpus/re_base/fl25/` — 5 синтетических базовых .flp из FL 25.2.4 (base_empty/one_pattern/one_channel/one_insert/one_serum) + `scratch/` (~15 sweep-файлов: tempo-свипы, master volume, EQ2 freq). Публичные фикстуры покрывают happy path. [ПОДТВЕРЖДЕНО]
- Личный корпус `tests/corpus/local/` (85 файлов FL 9–25) — **gitignored**, «Never commit real user projects»: свежий checkout его не содержит, parity-тесты скипаются. [ПОДТВЕРЖДЕНО, tests/corpus/README.md]
- Parity-оракул: приватный Python `flp-info`/`flpdiff` родительского проекта; 11 найденных и исправленных расхождений задокументированы в spec (раздел «Parity-harness corrections»), включая исторически неверные `0xD9→0xE9` и `0xCF→0xDF`. Дифф-parity: 5/6 пар byte-identical с Python-выводом (docs/parity-gaps.md; единственный разрыв — TS **лучше** достаёт vendor Kickstart). [ПОДТВЕРЖДЕНО как документация; запуски не воспроизводились]

## Ограничения / платформенные нюансы

- Bun-only разработка; для встраивания в Rust-проект npm-пакет малополезен, а CLI-бинарник — да. Тестовая CI не гоняется на Windows.
- Мутации бьют мимо на пред-FL25 файлах (частично защищаются: LEGACY_TEMPO_FORMAT; 80-байтовый энкодер не для старых файлов).
- `moveClip`/`removeClip` — bulk-match за один вызов (gotcha №1 в docs/mutations-gotchas.md); `0x16` routing — signed int8, максимум insert 127.
- Открытые в спеке дыры (spec §6): составной `0xC0` project-properties blob, 0xEE track data (opaque, 70 байт), `0xD5` нативных плагинов, PAHDSR-автоматизация сэмплеров.
- 0.1.x: npm-публикация отстаёт от репо; API не заморожен.

## Выводы для VibeStudio

1. **Спецификацию — как первичный справочник.** `docs/fl-format/flp-format-spec.md` + `fl25-event-format.md` — лучшая документированная карта FLP (включая FL25-релокации `0xE0/0xEE/0xDB/0xEC` и 80-байтовые clip records). MIT — читать и использовать свободно; она сама clean-room. [ПОДТВЕРЖДЕНО]
2. **flpdiff-CLI — как оракул в CI.** Standalone-бинарник, exit 0/1/2, stable text-вывод; можно сравнивать собственный Rust-парсер с ним на корпусе (как когда-то сравнивали TS с Python). Лицензионно чисто. [ПОДТВЕРЖДЕНО]
3. **Код — можно брать (MIT), но он на Bun/TS и под FL25-мутации;** для Rust это референс алгоритмов (scope-атрибуция 0xCB, дедуп двойных `0x41`, first-slot-straddle в `0x62`, сортировка notes перед энкодом), а не библиотека. Прямой перенос идей безопасен; копируйте файлы выборочно и ведите NOTICE.
4. **Не связываться с форком `dawhubapp/PyFLP`** (GPL) и при вычитке org-репозиториев не переносить GPL-код в VibeStudio.
5. Планирование: проект спит с 2026-07 — закладывать форк/вендоринг, если берёте код.
6. Практический контур для VibeStudio CI: `flpdiff A.flp B.flp` (exit 0/1) как стабильный текстовый оракул + его спека как форматный справочник + 5 публичных FL25-фикстур как стартовый корпус (файлы — продукты RE самого проекта, в их репо коммитятся свободно; перед публикацией корпуса у себя проверить желание распространять .flp-данные).

## Открытые вопросы

- Почему pushed_at (2026-10-08) позже последнего коммита main (2026-07-04)? Возможно, активность в приватных/побочных ветках — статус «жив ли проект» лучше перепроверить перед ставкой на него. [ПРЕДПОЛОЖЕНИЕ]
- Реально ли round-trip byte-exact на не-FL25 файлах после перехода энкодера на 80-байтовые записи (код comments утверждает, тестов на legacy round-trip записи мы не видели). [ПРЕДПОЛОЖЕНИЕ]
- Расхождение с flpkit по размеру события 172/`0xAC` (3 байта vs 1 байт) — см. flpkit.md; спека flpdiff фиксирует 3 байта по issue #1 и FL25 фикстурам.
