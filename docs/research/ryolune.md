# ryolune — аудит исходников (AI-native DAW, Rust + MCP)

Дата проверки: 2026-10-20. Метод: shallow clone (`--depth 200`) в `/tmp/oss_audit/ryolune`, чтение кода, `gh api`. Код не запускался и не собирался.

## Вердикт (кратко)

Самый зрелый из двух изученных «AI-native DAW» и, вероятно, самый зрелый open-source пример связки «DAW + MCP» вообще. Заявления README почти полностью подтверждаются кодом: это рабочая бета-DAW (v0.17.0, подписанные macOS-сборки, 17 релизов за месяц), с настоящим окном на GPUI, и эталонно спроектированным MCP-слоем: единый command registry (245 команд) порождает и CLI, и MCP tools, и действия UI — одна модель проекта, одно undo. Реализация MCP — собственный JSON-RPC 2.0 over stdio без SDK, но с полной поддержкой tools/resources/prompts, checkpoint-безопасностью и контекстом «что сделал человек». Минусы: 1 автор, 2★, самохостед CI, часть заявленного (Windows, plugin GUI на Linux) ещё не готово. Код чистый: 0 `todo!`/`unimplemented!`.

## Метаданные

| Поле | Значение | Источник |
|---|---|---|
| Repo | https://github.com/ludovic111/ryolune | gh api, 2026-10-20 |
| Версия | 0.17.0 (Cargo.toml workspace) | [ПОДТВЕРЖДЕНО] `Cargo.toml` |
| Лицензия | MIT | [ПОДТВЕРЖДЕНО] `LICENSE`, 21 строка, «Copyright (c) 2026 Ludovic Marie» |
| Размер | ~72 MB (с ассетами/шрифтами; clone без LFS ~93 MB) | gh api + du |
| Звёзды/форки | 2 / 0 | gh api |
| Создан | 2026-09-12 | gh api |
| Последний push | 2026-10-12 | gh api |
| Открытые issues | 0 (issues фактически не используются — работа через PR) | gh api |
| Автор | Ludovic Marie (124 коммита «Ludovic Marie» + 26 «Ludovic» в видимом окне 200) | git log |
| Язык | Rust ~100K строк (engine+desktop+tools+sdk), edition 2021, rust-version 1.88 | wc -l по src |
| Сайт | https://lsuite.xyz/ryolune | README |

## Лицензия: что можно/нельзя

MIT — максимально свободная: можно копировать, модифицировать, использовать в проприетарном VibeStudio, продавать; единственное условие — сохранить copyright-уведомление. Шрифты (Chakra Petch, IBM Plex Mono) — SIL OFL 1.1; часть иконок Lucide (ISC), провайдер-иконки Lobe Icons (MIT) — при копировании ассетов проверять их NOTICE (`desktop/assets/logos/NOTICE.md`). [ПОДТВЕРЖДЕНО] README §License.

Для VibeStudio: код ryolune можно брать без ограничений (кроме обязательного указания авторства). CLA для контрибьюторов обратно в ryolune на VibeStudio не распространяется.

## Активность

- 17 GitHub releases за месяц: v0.1.0 (2026-09-12, под старым именем «Ondera») → v0.17.0 (2026-10-11). Переименование Ondera → ryolune в 0.11 с сохранением обратной совместимости формата `.ondera` [ПОДТВЕРЖДЕНО: `CLAUDE.md`, `releases` API].
- 100 коммитов за последние 30 дней; merge-поты через PR (#34–#40). [ПОДТВЕРЖДЕНО: git log]
- CI: `.github/workflows/native.yml` (fmt+clippy+test на self-hosted Linux runner `ludodesktop`; macOS на «Mac mini» автора вручную), `release.yml`, `resume-release.yml` (подпись Ed25519, нотаризация macOS). Windows-джобы упомянуты как «coming soon». [ПОДТВЕРЖДЕНО]
- Требование CLA (`CLA.md`) для PR. [ПОДТВЕРЖДЕНО]
- Следы AI-разработки: `CLAUDE.md` с детальними инструкциями для Claude Code; ветки `codex/**` в CI-триггерах; коммиты вида «MCP: remind the finish routine…». Код однородного высокого качества — AI-ассистированная разработка под жёстким контролем человека. [ПОДТВЕРЖДЕНО]

## Сборка / платформы

- Rust 1.88+, `cargo run --release` из корня workspace. Зависимости движка: cpal 0.16, rtrb 0.3, symphonia, hound, clap-sys 0.5, vst3 0.3, libloading, midir 0.10, midly, vorbis_rs, zip+roxmltree (DAWproject), resvg. [ПОДТВЕРЖДЕНО: `engine/Cargo.toml`]
- Desktop: gpui 0.2.2 (фреймворк Zed, GPU-рендер, `runtime_shaders`), objc2* для macOS (NSWindow/AU). [ПОДТВЕРЖДЕНО: `desktop/Cargo.toml`]
- Публикуемые сборки: Linux x86_64 + macOS (Apple Silicon и Intel, signed+notarized). Windows: «код собирается, сборки не публикуются» — в Cargo.toml есть `winresource`, в CI windows-джобы отключены. [ПОДТВЕРЖДЕНО: README §Getting it, native.yml]
- Ограничения по README: Linux — внешние плагины без собственных GUI-окон (только список параметров); `ui.screenshot` только macOS; нет VST2/AAX/MP3; нет time stretch/comping/смены размера такта внутри песни. [ПОДТВЕРЖДЕНО: README §Known limitations]

## Архитектура

```
ryolune/
├── engine/        # ryolune-engine: без окна — модель, store+undo, registry,
│   │              # DSP/рендер, аудио/MIDI устройства, plugin hosting, экспорт
│   ├── src/control.rs        # РЕЕСТР: 245 команд (Spec: name/doc/params/mutates)
│   ├── src/control/wire.rs   # live-мост: TCP 127.0.0.1 + токен + discovery-файл
│   ├── src/control_*.rs      # реализация команд по доменам (16 файлов)
│   ├── src/harness/          # агентский харнес: brief.md, 13 skills, look/measure
│   ├── src/host/             # clap.rs, vst3.rs, au.rs, native.rs, scan.rs
│   └── src/lsuite.rs         # discovery/handoff между приложениями lsuite
├── desktop/       # ryolune: GPUI-окно (ui/), встроенный агент (agent/),
│   │              # live-половина registry (control.rs), генерация, апдейтер
├── tools/         # ryolune-tools: ryolune-cli + ryolune-mcp (тонкие клиенты)
├── sdk/           # ryolune-plugin: Plugin trait, frozen C ABI (ffi.rs), тест-бенч
├── plugins/       # примеры native-плагинов: gain, bitcrusher, chorus, abi1-fixture
├── evals/         # 13 скриптовых music-jobs через Claude Code + MCP, автоскоринг
└── docs/          # ARCHITECTURE/AI_CONTROL/COMMANDS/HARNESS/SESSION_FORMAT/...
```

- **Потоки**: interface thread (GPUI) владеет Store/Library/plugin editors; control server (TCP, до 16 клиентов, bounded queue + wake) ставит команды в очередь — они исполняются на interface thread «между кадрами»; аудио-callback (cpal) получает команды через rtrb-кольцо (256 сообщений: Replace(Renderer), Mount, SetParam...) — RCU-подмена рендерера; старые рендереры возвращаются через second ring. В callback: «no allocation, locks, I/O or logging». [ПОДТВЕРЖДЕНО: docs/ARCHITECTURE.md §Threads, desktop/src/control.rs]
- **Модель проекта**: свой формат `.ryolune` — JSON с аудио внутри, один файл; store с revision-счётчиком, undo/redo, checkpoints (snapshot + revision). Есть recovery, snapshots, DAWproject import/export. [ПОДТВЕРЖДЕНО: engine/src/store.rs, docs/SESSION_FORMAT.md]
- **Plugin hosting**: CLAP (clap-sys 0.5), VST3 (vst3 0.3), AU — вручную через objc2-audio-toolbox (AudioComponent/AUComponent) на macOS; native-плагины через собственный SDK с frozen C ABI + crash-isolated сканирование в дочернем процессе (`--scan-plugin`). [ПОДТВЕРЖДЕНО: engine/src/host/*.rs, tools/src/lib.rs `scan_child`]
- **UI**: полностью нативный GPUI (Zed), «no webview, no JavaScript»; 35 stock-инструментов/эффектов с собственными панелями. [ПОДТВЕРЖДЕНО: desktop/src/ui/, engine/src/stock.rs]
- **AI-генерация звука**: НЕ локальные модели — облачные сервисы ElevenLabs, Stable Audio, fal.ai или кастомный endpoint (`generate.audio`), результаты как audio clip или Sample Keys-инструмент. Встроенный агент — обёртка над внешними провайдерами (Anthropic/OpenAI/Gemini/OpenRouter/Mistral/Groq/DeepSeek/xAI API-ключи, Ollama/LM Studio, Claude Code/Codex CLI). [ПОДТВЕРЖДЕНО: engine/src/control_generate.rs, desktop/src/agent/]
- **lsuite**: НЕ моно-репо кода — «suite of four free, open-source apps»: kimchi (video), ryolune (music), nori, folio. Интеграция через конвенцию: каждый app пишет `~/.lsuite/apps/<app>.json` (discovery, формат 1) и обменивается файлами через `~/.lsuite/handoff/`; ryolune умеет отдавать микс/стемы в kimchi и принимать «kimchi-cut» на скоринг (`session.scoreCut`). Репозитории остальных apps в этом clone не видны (в дереве только ryolune; CLAUDE.md упоминает `../lsuite/design/` в окружении автора). [ПОДТВЕРЖДЕНО: engine/src/lsuite.rs; ЧАСТИЧНО — состав apps]

## MCP-интеграция (главный фокус)

**Где**: `tools/src/bin/ryolune-mcp.rs` (636 строк) + `tools/src/lib.rs` (354 строки, общий Backend с CLI). Отдельный бинарник `ryolune-mcp`, собирается вместе с app и CLI («keep them in one folder»).

**SDK**: НЕТ внешнего MCP SDK — рукописный JSON-RPC 2.0 over **stdio** (зависимости крейта: только serde_json + base64 + ryolune-engine). Поддерживаемые protocolVersion: `2024-11-05`, `2025-03-26`, `2025-06-18`. [ПОДТВЕРЖДЕНО: ryolune-mcp.rs:15]

**Методы сервера**: initialize, ping, tools/list, tools/call, resources/list, resources/read, resources/templates/list, prompts/list, prompts/get, completion/complete (заглушка). [ПОДТВЕРЖДЕНО: dispatch()]

**Режимы запуска** (Backend из tools/lib.rs):
- live (по умолчанию / `--live`): подключается к работающему ryolune через discovery-файл `{port, token, pid}` и TCP на 127.0.0.1 с token-auth; агент и человек редактируют одну сессию с одним undo;
- headless (`--headless` / fallback): хостит сессию в своём процессе; с `--file song.ryolune` — автосохранение после каждого изменения + файловая блокировка (SessionFileLock, защита от гонки с окном).

**Таблица MCP tools**: tools/list генерируется из command registry — **245 tools**, имена `track.add` → `track_add`. Это НЕ ручной список; ниже — группы:

| Группа команд (prefix) | Кол-во | Примеры |
|---|---|---|
| session.* | 24 | session.overview, session.batch, session.bounce, session.importMidi/exportStems, session.importFrom (DAWproject) |
| track.* | 16 | track.add, track.setVolume, track.setPan, track.setMute/Solo |
| clip.* | 27 | clip.create (notes массивом), clip.setNotes, clip.split, clip.fades |
| note.* / marker.* / tempo.* / transport.* | 7/10/5/12 | note.add, marker.add, transport.setTempo, transport.play |
| strip.* / plugin.* / preset.* | 17/18/4 | strip.setPlugin, strip.setSendLevel, plugin.scan, plugin.install |
| automation.* / controller.* / take.* | 8/5/4 | automation.setPoints, take.comp |
| harness.* | 10 | harness.checkpoint, harness.look, harness.measure, harness.context, harness.revert |
| generate.* / agent.* / ui.* / app.* / history.* / settings.* / view.* / audio.* / rhythm.* / master.* / source.* / export.* / handoff.* | 6/22/10/18/3/3/3/7/2/1/1/1/1 | generate.audio, ui.screenshot, history.undo |

Полный список: docs/COMMANDS.md (генерируется из registry, 245 записей). [ПОДТВЕРЖДЕНО: grep count]

**Качество MCP-слоя — выше среднего по индустрии**:
- Каждый tool несёт `inputSchema` из type-информации registry, `title`, `description` из doc-комментария и **annotations**: readOnlyHint/destructiveHint/idempotentHint (destructive эвристически по подстрокам remove/delete/new/open/quit/reset/restore/setNotes/clear). [ПОДТВЕРЖДЕНО: fn tools()]
- **Resources**: 9 registry-backed (`ryolune://session/overview`, `ryolune://session`, `ryolune://catalog`, `ryolune://settings`, …) + `ryolune://brief` (system prompt музыкального продюсера) + 13 skills как `ryolune://skills/<name>`.
- **Prompts**: 3 встроенных (compose, mix-review, see-the-window) + 13 skills как prompts с аргументом request.
- **Безопасность агента**: перед первым мутирующим вызовом соединения автоматически берётся `harness.checkpoint` (весь джоб откатывается одним `harness_revert`); после каждого мутирующего вызова, пока агент не сделал `harness_look`/`harness_measure`, в результат добавляется напоминание про finish routine; в live-режиме в каждый результат вставляется «what the person changed since your last call» (harness.context key=mcp) и одна строка состояния песни. Всё дублируется в `structuredContent.harnessNotes`. [ПОДТВЕРЖДЕНО: call_tool()]
- **Мультимодальность**: `harness.look` рендерит диапазон песни офлайн (волна + лауднесс + спектр + piano roll) и возвращает MCP image content; `ui.screenshot` (macOS) тоже возвращает окно картинкой (PNG→base64, лимит 8MB). [ПОДТВЕРЖДЕНО]
- **RT-safety**: MCP-слой сам не трогает аудио-поток: команды проходят валидацию и store-транзакции на interface thread между кадрами; аудио-callback получает эффекты через rtrb-кольца. Транспорт (transport.play в live) слышим — это честно задокументировано в instructions сервера. Headless: transport_play недоступен, вместо него session_bounce. [ПОДТВЕРЖДЕНО]
- **Документация MCP**: docs/AI_CONTROL.md (4 способа входа, рецепты), docs/HARNESS.md, docs/AGENT_PARITY.md (аудит «каждое действие окна = команда»), команда `claude mcp add ryolune -- …/ryolune-mcp --live` в README. [ПОДТВЕРЖДЕНО]
- **Evals**: `evals/` — 13 скриптовых заданий (drums-house, master-streaming, …), гоняются через Claude Code + ryolune-mcp headless с автоскорингом по песне (темп, тональность, ноты в гамме, LUFS/true peak, «не тронул треки человека», «выполнил finish routine»); результаты в RESULTS.md (последний полный прогон Opus: 13/13). Это редчайший артефакт — публичный регресс-харнес для AI-управления DAW. [ПОДТВЕРЖДЕНО: evals/README.md, git log]

## Реальная готовность: заявка → реальность

| Заявка | Реальность | Доказательство |
|---|---|---|
| «The open-source DAW your AI can drive» | Подтверждается: 245 команд покрывают сессию целиком, parity-аудит окна→команды (AGENT_PARITY.md), 4 клиента на одном registry | docs/AGENT_PARITY.md, control.rs |
| «CLAP/VST3/AU plugins» | Реальный hosting-код на clap-sys/vst3/objc2; AU-плагин GUI только macOS; на Linux — только список параметров внешних плагинов | engine/src/host/{clap,vst3,au}.rs; README Known limitations |
| «Native Rust, window included» | Да, GPUI-окно, весь UI в desktop/src/ui (панели, микшер, piano roll, agent panel) | desktop/Cargo.toml, CLAUDE.md |
| «Crash-isolated plugin scanning» | Скан в дочернем процессе (`--scan-plugin` реэксекьютом бинарника) | tools/src/lib.rs scan_child |
| «Sounds from a description» | Только облачные сервисы (ElevenLabs/Stable Audio/fal.ai/custom); локальных моделей НЕТ | engine/src/control_generate.rs |
| «MCP: any MCP client» | Полный stdio-сервер: tools+resources+prompts, 3 версии протокола, изображения; тесты реальным бинарником over stdio | tools/tests/mcp.rs (9 тестов) |
| «Windows coming soon» | В коде есть winresource и упоминания, но CI/dispatch windows не включён — честная заявка, не готово | native.yml комментарии |
| «Beta» | Соответствует: известные ограничения перечислены честно (нет stretch/comping/MP3, buses не кормят buses) | README §Known limitations |

Заглушек `todo!`/`unimplemented!` в коде: **0** (единственный `unreachable!` — валидный инвариант try_send). [ПОДТВЕРЖДЕНО: grep]

## Тесты

- 593 `#[test]` по всему workspace (включая модульные + `engine/tests/`, `tools/tests/`, `scripts/tests/`). [ПОДТВЕРЖДЕНО: grep count]
- MCP покрыт интеграционно: `tools/tests/mcp.rs` — 9 тестов, которые стартуют реальный бинарник ryolune-mcp и говорят с ним JSON-RPC по stdio (проверяя initialize, tools/call, prompts, resources, autosave, JSON-RPC-ошибки, checkpoint-механику). [ПОДТВЕРЖДЕНО]
- CI гоняет `cargo fmt`, `clippy -D warnings`, `cargo test --workspace` на каждый push/PR (Linux), macOS вручную. [ПОДТВЕРЖДЕНО: native.yml, CONTRIBUTING.md]
- Плюс 13 AI-evals как отдельный уровень регресса харнеса.

## Выводы для VibeStudio

Взять идеями/паттернами (код MIT — можно и кодом, но полезнее паттерны):

1. **Один command registry — много клиентов.** Каждый аспект управления DAW объявляется как `Spec { name, doc, params, mutates }` в статической таблице; из неё генерируются MCP tools (со схемами и annotations), CLI, справка и даже паритет-аудит UI. Это лучший найденный ответ на вопрос «как дать AI управлять DAW, не создавая параллельный API».
2. **Undo как универсальная защита от AI**: каждое мутирующее действие — один undo-шаг; `session.batch` = много правок за один шаг; автоматический checkpoint перед первой правкой MCP-соединения + `harness_revert`.
3. **Замыкание сенсорного контура**: `harness.look` (офлайн-рендер песни в картинку: волна+лауднесс+спектр+ролл) и `harness.measure` (BS.1770 LUFS, true peak, findings) — агент «видит и слышит» результат без живого аудио. Плюс «finish routine» — принуждение агента проверить результат перед отчетом (nag-нот в каждом результате).
4. **Двухрежимный бэкенд**: live (токен-мост к запущенному приложению, обмен с человеком контекстом «что изменилось с твоего последнего шага») и headless (`--file` с автосохранением и файловым локом) — покрывает и «агент рядом с музыкантом», и CI/скрипты.
5. **MCP без SDK реально и компактно**: 636 строк на полный stdio-сервер (tools+resources+prompts+images+structuredContent). Для VibeStudio это аргумент либо за такой же тонкий слой, либо за rmcp — но генерация tools из registry обязательна.
6. **Evals для AI-DAW**: автоскоринг песни после агентской сессии — переносимый паттерн регресс-тестов «AI-качества».
7. **RT-safety**: команды AI никогда не идут в audio callback напрямую — только через store на interface thread + rtrb/RCU-подмену рендерера.

Прямо брать код: возможно `Spec/Kind/schema`-подход (микро-, ~200 строк), coerce-логику CLI. Движок целиком не берём — у VibeStudio свои приоритеты, но engine есть хороший чеклист фич (PDC, input monitoring, count-in, delay compensation).

## Открытые вопросы

1. Состав lsuite: репозитории kimchi/nori/folio публично не найдены из clone (только сайт lsuite.xyz) — насколько «suite» реален вне машины автора? [ПРЕДПОЛОЖЕНИЕ: как минимум kimchi существует — протокол handoff детализирован с обеих сторон]
2. Долгосрочная активность: весь код — один автор за один месяц; bus factor = 1. Станет ли ryolune поддерживаемым проектом или личным продуктом?
3. Самохостед CI (ludodesktop, Mac mini) — воспроизводимость сборки для сторонних контрибьюторов неочевидна.
4. Нагрузочные характеристики (большие сессии, сотни клипов) нигде не бенчмаркались публично — `session.overview` заявляет truncation, но цифр нет.
5. macOS `ui.screenshot`/`harness.look` через CoreGraphics — нет Linux-аналога скриншота окна (GPUI-рендер в картинку не сделан); для headless-агентов на Linux это пробел.
