# AURA (aura-daw) — аудит исходников (AI-native DAW, Rust + MCP)

Дата проверки: 2026-10-20. Метод: shallow clone (`--depth 200`) в `/tmp/oss_audit/aura-daw`, чтение кода, `gh api`. Код не запускался и не собирался.

## Вердикт (кратко)

Инженерно дисциплинированный проект с редкой честностью документации: это действительно «working vertical slice» FL-подобной DAW на Tauri v2 + Svelte 5 + Rust RT-движке, с встроенным MCP-сервером на официальном rmcp SDK (streamable-HTTP) и продуманной политикой безопасности (token + Origin + policy-режимы + UI-confirm). AI-часть реальна, но это НЕ локальные модели внутри движка, а Python-sidecar процессы (ACE-Step, Anticipatory Music Transformer, Demucs, Whisper) с `--simulate`-фолбэком; плюс впечатляющий детерминированный «Composer» на чистой теории музыки (не ML). MCP-поверхность маленькая: 10 frozen tools — только project/mix/transport/record/import/sidecar, **нет note-level редактирования**. Главные риски: проект **остановился** (последний коммит 2026-09-02, ~7 недель тишины на дату проверки), Linux-only на практике, bus factor = 1. Лицензия GPL-3.0 — код для VibeStudio использовать НЕЛЬЗЯ, только идеи.

## Метаданные

| Поле | Значение | Источник |
|---|---|---|
| Repo | https://github.com/knobo/aura-daw | gh api, 2026-10-20 |
| Версия | 0.1.0 (package.json; Cargo `aura-engine` 0.1.0) | [ПОДТВЕРЖДЕНО] |
| Лицензия | GPL-3.0 (полный текст, 674 строки) | [ПОДТВЕРЖДЕНО] `LICENSE` |
| Размер | ~18 MB по gh api (clone ~45 MB, вкл. .git) | gh api + du |
| Звёзды/форки | 3 / 0 | gh api |
| Создан | 2026-08-12 | gh api |
| Последний push | 2026-09-02 | gh api |
| Открытые issues | 4: #143 «Pure Genius» (похвала), #142 measurable coalesced launch, #104 Linux packaging/AppImage, #62 **ci(windows): release worker** | gh api |
| Автор | Knut Olav Bøhmer (bohmer@gmail.com) — 197 из 197 видимых коммитов | git log |
| Язык | Rust ~111K строк в `src-tauri/src` + ~1000 строк `aura-engine`; фронтенд 335 Svelte/TS файлов | wc -l / find |
| CI | GitHub-hosted ubuntu-24.04: `tests.yml` (vitest, svelte-check, cargo test), `engine.yml` | [ПОДТВЕРЖДЕНО] |

## Лицензия: что можно/нельзя

GPL-3.0 — strong copyleft. Для VibeStudio:
- **Нельзя** копировать код (Rust, Python, Svelte) в проприетарный проект — производная работа обязана стать GPL-3.0.
- **Можно** читать, изучать идеи/архитектуру/паттерны (авторское право не покрывает идеи), запускать, писать собственную реализацию по мотивам (чистая комната: лучше не держать GPL-код открытым при написании своего).
- Отдельные независимые модули (например, `aura-engine/` с Cranelift JIT) — тоже GPL, преимуществ не даёт.
- Документы (docs/ARCHITECTURE.md, GAP_ANALYSIS) — та же GPL (репо без отдельных лицензий на доки); цитировать факты можно, копировать текст — осторожно.

## Активность

- 197 коммитов за ~3 недели интенсивной работы (2026-08-12 → 2026-09-02), затем **тишина** (0 коммитов после 2026-09-02 в shallow-окне 200; push даты это подтверждают). [ПОДТВЕРЖДЕНО]
- Релизов/тегов нет; версионирование только в манифестах. CI активен на момент последнего пуша. [ПОДТВЕРЖДЕНО]
- Следы оркестрации Claude-агентов повсюду и это часть идентичности проекта: `CLAUDE.md` (протокол работы: worktrees, claim-таблица в `next-prompt.md`, инцидент 2026-08-21 с двумя агентами в одном worktree), `.claude/skills/run-aura/`, `.superpowers/sdd/`, `docs/handoff/*` (передачи между агентами), `docs/backlog/`, ARCHITECTURE §0 «Agent ownership boundaries» (зоны A, B, C, P1, P4…), реестр долга в SCALABILITY.md. [ПОДТВЕРЖДЕНО]
- Качество коммит-сообщений и кода высокое; единственный `todo!` в коде — на самом деле комментарий, объясняющий, почему автор сознательно НЕ написал `todo!()`. [ПОДТВЕРЖДЕНО: control/mod.rs:494]

## Сборка / платформы

- Rust stable + Node 18–24; `npm run tauri dev`. Linux-зависимости: libasound2-dev, **libwebkit2gtk-4.1-dev**, libsoup-3.0, liblilv-dev (LV2 host линкует системный lilv), ffmpeg, libayatana-appindicator. [ПОДТВЕРЖДЕНО: README §Setup]
- **Linux — основная и единственная обкатанная платформа** (WebKitGTK; в TRAPS/README — дебаг DMA-BUF рендера WebKit, wm_stack гонок окон плагинов). [ПОДТВЕРЖДЕНО]
- **Windows**: Tauri кроссплатформен, но release-воркер — открытый PR #62 «never exercised»; GAP_ANALYSIS: рантайм-кодген (Cranelift) на Windows будет триггерить EDR/антивирусы. [ПОДТВЕРЖДЕНО: GAP_ANALYSIS §454-461]
- **macOS**: «later» — для JIT-крейта нет MAP_JIT-пути в cranelift 0.134 (несовместимо с нотаризацией без широкого entitlement). [ПОДТВЕРЖДЕНО: GAP_ANALYSIS; ПРЕДПОЛОЖЕНИЕ относительно вывода об entitlement]
- AI-models: отдельный venv (`.venv-sidecars`), GPU-стек (torch cu126, ACE-Step ~7GB VRAM с offload) ставится опционально; без него все генераторы работают в `--simulate` (детерминированная заглушка-арпеджио). [ПОДТВЕРЖДЕНО: sidecars/requirements*.txt, ace_step_worker.py]

## Архитектура

```
aura-daw/
├── src/                    # Svelte 5 + Tailwind UI (335 файлов, 129 *.test.ts)
├── src-tauri/              # Rust-ядро (~111K строк), Tauri v2
│   ├── src/audio/          # engine.rs (cpal), rt.rs (SharedRt/граф), mixer, transport,
│   │                       # recorder, sampler*, meters, yin (питч), pdc, metronome
│   ├── src/control/        # ControlPlane — ЕДИНАЯ «передняя дверь»: ops, op.rs (журнал
│   │                       # NDJSON, v2), history, snapshot, vergraph (undo-граф),
│   │                       # composer, session, import/export, loopjam
│   ├── src/mcp/            # MCP-сервер (см. ниже), 2549 строк
│   ├── src/plugins/        # clap_host.rs (clack), lv2_host.rs (livi) + lv2_host_stub.rs,
│   │                       # scan_worker, wm_stack (окна плагинов), stderr_guard
│   ├── src/midi/           # tempo map (tick↔sample), synth, AMEV-бинарные чанки нот
│   ├── src/theory/         # чистые функции Composer-а (без I/O)
│   └── src/sidecars/       # JobManager: NDJSON-протокол к Python-воркерам
├── aura-engine/            # ОТДЕЛЬНЫЙ crate: Cranelift-JIT fused kernel, бенчи —
│   │                       # НЕ подключён к src-tauri (ничего не зависит от него)
├── sidecars/               # Python: ace_step_worker, amt_infill_worker, demucs_split,
│   │                       # elevenlabs_music_worker, stable_audio_sfz_worker,
│   │                       # hum_to_midi, whisper_transcribe
├── docs/                   # ARCHITECTURE (§0 ownership, §2 RT rules, §12 MCP),
│   │                       # GAP_ANALYSIS, SCALABILITY (долг), TRAPS, ADR, ipc-schemas/
└── benches/                # bulkbench, pdsbench, ui-probe
```

- **RT-движок**: cpal 0.16 (ALSA/JACK); контракт callback — «never allocate, never lock, never syscall, no panics»; связь через rtrb SPSC-кольца POD-сообщениями (`engine_cmd` 256, `engine_evt` 64, `meters` ~16 блоков); RCU-подмена указателя графа микса; STOP@END определяется в callback, решается в control plane. [ПОДТВЕРЖДЕНО: docs/ARCHITECTURE.md §2, audio/rt.rs]
- **Модель проекта**: `project.json` (schema v3) + NDJSON op-журнал (OP_FORMAT_VERSION=2, base64-блобы) с читателем-replay + снапшоты; ноты — бинарные AMEV-чанки; undo — не стек, а **VersionGraph** (дерево версий, «Undo to here» по ancestry). Harmony document — `tick → (key, chord)` рядом с tempo map. [ПОДТВЕРЖДЕНО: control/op.rs, vergraph.rs, midi/, docs/ipc-schemas/]
- **Plugin hosting**: CLAP — clack-host 0.1 + clack-extensions (params/audio-ports/note-ports/state/gui/latency/timer), CLAP 1.2; LV2 — livi 0.7 (обёртка lilv, нужен системный liblilv-dev), feature `lv2` с no-op стабом при выключении; сохранение LV2 state — «raw-lilv work on top», т.е. частично в долге. [ПОДТВЕРЖДЕНО: src-tauri/Cargo.toml комментарии]
- **UI**: webview (Svelte 5, Tailwind 4, Tauri invoke) — в отличие от ryolune это web-технологии; meters в UI через ~60 Hz стрим. [ПОДТВЕРЖДЕНО]
- **AI-генерация**: локальные модели — в sidecar-процессах: ACE-Step 3.5B (text-to-music, generate/repaint/audio2audio, CUDA), Anticipatory Music Transformer (MIDI infill региона, CPU), Demucs (стем-разделение), Whisper (транскрипция), plus cloud ElevenLabs Music; hum-to-midi — свой YIN-трекинг в Rust. **Composer** (аккорды/мелодия/бас/барабаны) — НЕ машинное обучение: детерминированные pure functions теории музыки (DP voice-leading, Euclidean ритмы, функциональная гармония) с объяснениями «почему». [ПОДТВЕРЖДЕНО: sidecars/*, src-tauri/src/theory/]
- **aura-engine (JIT)**: standalone crate «deliberately outside the Tauri app»; в исходниках src-tauri нет ни одной зависимости от него (только строка имени потока «aura-engine-control»). Заявлять JIT-ускорение как работающую часть DAW нельзя. [ПОДТВЕРЖДЕНО: aura-engine/Cargo.toml комментарий, grep]

## MCP-интеграция (главный фокус)

**Где**: `src-tauri/src/mcp/` — 6 файлов, 2549 строк: mod.rs (state+Tauri glue), server.rs (1131), handler.rs (552), http1.rs (343), auth.rs (145), policy.rs (126), tools.rs (48).

**SDK**: **rmcp 3.1** (официальный Rust MCP SDK, features `server`, `transport-streamable-http-server`) — `ServerHandler` через макросы `#[tool]`/`#[tool_router]`/`#[tool_handler]`, схемы через schemars. [ПОДТВЕРЖДЕНО: handler.rs, Cargo.toml]

**Transport**: **streamable-HTTP**, встроен в процесс Tauri-приложения, `http://127.0.0.1:41717/mcp` (порт через policy, bind только loopback). Особенность: замороженный Cargo.toml не содержит hyper/axum, поэтому поверх rmcp написан минимальный HTTP/1.1-слой (`http1.rs`), который перекладывает POST-тела в in-process duplex-pipe (newline JSON-RPC) к `rmcp::serve_server`-таску на сессию; `Mcp-Session-Id`, 202 для notifications, DELETE — завершение сессии; standalone GET/SSE отдаёт 405 (spec-допустимо). Лимиты: 64 сессии, 128 соединений, idle reaper 30 мин, request timeout 300 c (покрывает 60-секундный confirm), init timeout 10 c. [ПОДТВЕРЖДЕНО: server.rs, mod.rs]

**Аутентификация/безопасность** (auth.rs, ARCHITECTURE §12.3 — «MANDATORY, non-configurable»): 256-bit bearer-токен на каждый запуск (0600-файл `~/.local/share/aura/mcp-token`, в логах только 8-символьный fingerprint), constant-time сравнение, Origin-валидация (DNS-rebinding защита) до любого диспатча. [ПОДТВЕРЖДЕНО]

**Policy-слой** (policy.rs): режимы `readOnly` / `confirmDestructive` (default) / `full` + per-tool overrides (Allow/Confirm/Deny); неизвестные инструменты всегда Deny. `Confirm` паркует вызов как `PendingConfirmation`, кидает событие `mcp://confirm-requested` в UI, резолвится `mcp_confirm_pending` из окна; таймаут 60 с ⇒ deny. [ПОДТВЕРЖДЕНО]

**Таблица MCP tools** (ровно 10, имена frozen — ARCHITECTURE §12.2):

| Tool | Kind | Что делает / ControlPlane |
|---|---|---|
| `get_project_state` | read-only | Полный снапшот проекта (та же wire-форма, что у UI) — project_state() |
| `read_meters` | read-only | «Уши» агента: последний 60 Hz peak/RMS фрейм — read_meters() |
| `get_job_status` | read-only | Статус sidecar-джобов (job_status/list_jobs) |
| `create_project` | destructive | Создать проект/папку (parent_dir, name) |
| `add_track` | destructive | Трек audio/midi (name, kind) |
| `set_track_mix` | destructive | Батч изменений микса: gain dB, pan, mute, solo, arm (атомарно) |
| `transport_control` | destructive | play / stop (stop при записи финализирует тейк) / seek (позиция в сэмплах) |
| `record_take` | destructive | start / stop записи |
| `import_audio_clip` | destructive | Импорт аудиофайла клипом |
| `run_sidecar_job` | destructive | AI-джобы: aceStepGenerate/Repaint/Audio2Audio, elevenLabsMusic, amtInfill, stableAudioSfz (+ importToTrackId) |

[ПОДТВЕРЖДЕНО: tools.rs, handler.rs, docs/mcp-usage.md]

**Чего НЕТ в MCP**: редактирование MIDI-нот/клипов, automation, плагины (хостинг CLAP/LV2 недоступен агенту), Composer-генерация (гармония/прогрессии — только из UI), undo/версии, доступ к timeline-структуре. Для «DAW, которой играет AI» поверхность узкая: state/ears + транспорт + запись + импорт + AI-джобы.

**Связь с движком и RT-safety**: handler — тонкие обёртки над `ControlPlane` (никогда не над `#[tauri::command]` — «Anything only reachable from a tauri::command body is a bug»); ControlPlane — единственная «передняя дверь» и для UI, и для MCP (одинаковые wire-формы → агент и UI видят одно состояние). Команды уходят в RT-callback только POD-сообщениями через SPSC-кольца (`EngineCmd`, включая swap указателя графа); метры возвращаются кольцом — read_meters читает последний фрейм без блокировки аудио. Confirm-пауза реализована oneshot-waker'ами (parked call), не блокируя потоки. [ПОДТВЕРЖДЕНО: control/mod.rs, audio/engine.rs, mcp/handler.rs]

**Документация MCP**: docs/mcp-usage.md — эндпоинт, ротация токена, готовые конфиги для Claude Code (`claude mcp add --transport http aura …`) и Claude Desktop (через `mcp-remote`); ARCHITECTURE §12 — нормативные требования; wire-схемы в docs/ipc-schemas/mcp-policy.schema.json. В README — GIF реальной сессии «Claude composing a track over MCP». [ПОДТВЕРЖДЕНО]

## Реальная готовность: заявка → реальность

| Заявка | Реальность | Доказательство |
|---|---|---|
| «AI-native DAW … MCP-controllable» | Да: встроенный rmcp-сервер, 10 tools, security+policy продуманы; GIF из реального приложения | src-tauri/src/mcp/, docs/screenshots/mcp-composition.gif |
| «Rust RT engine» | Да: cpal + SPSC-кольца + RCU-граф, правила §2 соблюдаются в коде | audio/rt.rs, ARCHITECTURE §2 |
| «CLAP/LV2 hosting» | CLAP — да (clack); LV2 — через livi с системным lilv, state save/restore в долге; feature-gated | plugins/lv2_host.rs, Cargo.toml |
| «local AI music generation» | Да, но вне процесса: Python sidecars (ACE-Step GPU ~7GB, AMT CPU); без моделей — `--simulate` заглушки; ElevenLabs — облако | sidecars/*.py, requirements-models.txt |
| «Built by orchestrated Claude agents» | Подтверждается полностью: claim-протокол, ownership-зоны, handoff-доки, ретроспективы инцидентов | CLAUDE.md, next-prompt.md, docs/handoff/ |
| «FL Studio-class DAW» (вертикальный срез) | Честно назван slice: timeline/MIDI/микшер/запись/плагины/AI есть; нот-левел MCP-контроля, comping, инструмент-хостинг GUI — нет/в долге | README, GAP_ANALYSIS, SCALABILITY |
| Cranelift JIT-ядро (aura-engine) | Написан и отбенчен, но НЕ интегрирован в приложение; мак-проблема MAP_JIT задокументирована | aura-engine/Cargo.toml, GAP_ANALYSIS |
| Кроссплатформенность | Фактически Linux-only: Windows-воркер не exercise, macOS «later» | PR #62, GAP_ANALYSIS §454 |

Заглушки: 1 `todo!`-подобное место — и то комментарий «honest no-op rather than a todo!()». Честность заявлений — образцовая. [ПОДТВЕРЖДЕНО]

## Тесты

- Rust: **1656** `#[test]`/`#[tokio::test]` в src-tauri (+ отдельные tests/: journal_and_history, journal_replay, snapshot_store, v3_migration, channel_properties, identity_properties, pure_readers…); интеграционные тесты реальных моделей за флагом `AURA_REAL_MODELS=1` (src-tauri/tests/real_models.rs). [ПОДТВЕРЖДЕНО]
- Фронтенд: 129 `*.test.ts` (vitest) + svelte-check в CI. [ПОДТВЕРЖДЕНО]
- CI (tests.yml): ubuntu-24.04, frontend-job + rust-тесты, с комментариями-измерениями (кэш, apt). engine.yml — для JIT-крейта. [ПОДТВЕРЖДЕНО]
- MCP покрыт тестами напрямую: policy.rs (юнит), handler/server — тесты confirm-флоу, auth, лимитов (тестовые хуки: `confirm_timeout_ms`, ephemeral port). [ПОДТВЕРЖДЕНО: код тестов в policy.rs/mod.rs/server.rs]

## Выводы для VibeStudio

**Код брать нельзя (GPL-3.0)** — ни Rust, ни Python, ни Svelte, ни тексты доков. Ценность — архитектурные паттерны, которые можно независимо реализовать:

1. **ControlPlane как единственная дверь**: и Tauri/UI-команды, и MCP-хендлеры вызывают одни и те же методы с одинаковыми wire-типами — «агент и UI наблюдают идентичное состояние». Прямой аналог для VibeStudio независимо от того, будет ли UI нативным.
2. **Policy-слой MCP**: readOnly/confirmDestructive/full + per-tool overrides + unknown⇒deny + UI-confirm с таймаутом⇒deny. Это более развитая модель разрешений, чем у ryolune; worth adopting в своей проекции.
3. **Транспорт-безопасность loopback-HTTP**: per-launch токен (0600, fingerprint в логах), constant-time compare, Origin-проверка против DNS-rebinding, лимиты соединений/таймаутов. Готовый чеклист, если VibeStudio выберет HTTP-транспорт вместо stdio.
4. **rmcp 3.1** — рабочий выбор SDK для streamable-HTTP; трюк с duplex-pipe между своим HTTP-шэмом и `serve_server` пригодится, если не хочется тянуть hyper/axum.
5. **read_meters как «уши» агента** — дешёвый и эффективный канал обратной связи; в связке с ryolune-подобным offline-рендером/measure получится полный сенсорный контур.
6. **Sidecar-паттерн для тяжёлых AI-моделей**: Python-процессы + NDJSON-протокол + JobManager + `--simulate` для демо/CI + опциональный requirements-models. Разделяет RT-процесс и гигабайтные модели — правильная граница для VibeStudio.
7. **Deterministic Composer на теории музыки** (не ML) — сильный аргумент: объяснимые, сеидируемые генераторы гармонии/мелодии/барабанов как pure functions. Идея переносима, реализация — своя.
8. **Инфраструктура мультиагентной разработки** (claim-таблица, ownership-зоны, handoff-доки, ADR, debt register) — непосредственно применимый процесс для «1 разработчик + AI-агенты» VibeStudio.
9. Предостережение из их опыта: замороженный Cargo.toml и «фрозен» wire-схемы хорошо держат агентов в границах, но JIT-крейт в стороне от приложения — пример работы, опередившей интеграцию.

Контраст с ryolune: AURA сильнее в безопасности/политике и AI-sidecar-стеке; ryolune сильнее в полноте MCP-поверхности (245 vs 10), зрелости DAW и живости проекта. Для VibeStudio оптимальная точка — registry-подход ryolune + policy/auth-модель AURA.

## Открытые вопросы

1. Жив ли проект: 0 коммитов с 2026-09-02 и ни одного ответа автора на issue #143 (2026-10-19). Если VibeStudio что-то хочет от автора (например, реляйензировать) — связаться стоит немедленно, пока репо не архивировано. [ПРЕДПОЛОЖЕНИЕ о статусе]
2. Почему остановился: handoff-доки обрываются на «launch quantize» (последний PR #141) — следующая веха (Plan V-V3 polyphony) не начата.
3. Реальная стабильность WebKitGTK-UI под нагрузкой — в доках описаны белые вспышки/перезагрузки webview (DMA-BUF), revert коммита о GPU-рендере; для DAW это серьёзный риск webview-подхода вообще.
4. LV2 state и GUI-хостинг плагинов — насколько работоспособны (wm_stack/stderr_guard говорят о болезненном опыте, но глубина покрытия неизвестна без запуска).
5. `run_sidecar_job` с реальными моделями проверен ли end-to-end кроме env-gated тестов (нужны GPU и ручная установка) — по коду да, по CI нет. [ПРЕДПОЛОЖЕНИЕ]
