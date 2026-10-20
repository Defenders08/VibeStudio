# Simply Nexus — исследование (poucet/nexus)

**Дата проверки: 2026-10-20.** Все факты получены из живых источников: сайт simply-music.ai,
GitHub API (repos/commits/contents), raw.githubusercontent.com, crates.io API.

---

## 1. Сводка-вердикт

Simply Nexus — это **не конкурент VibeStudio, а почти готовый референс его фундамента**.
Это open-source (MIT) Rust-хостинг VST3/CLAP-плагинов от одного человека: библиотека-движок
(`nexus-host`), standalone GUI-хост (`nexus-app`, пока macOS-only) и **headless MCP-сервер
(`nexus-mcp`)** для AI-агентов. Стек почти идентичен планируемому стеку VibeStudio
(Rust + CLAP/VST3 hosting + MCP). Код полностью открыт, лицензия MIT — можно читать,
переиспользовать и форкать без ограничений. Главные оговорки: проект очень молодой
(~3 месяца активной разработки, 86 коммитов), активность остановилась 2026-09-04,
крейты на crates.io не опубликованы, бинарных релизов нет — только сборка из исходников.

---

## 2. Что за продукт

Источник: <https://simply-music.ai/nexus> [ПОДТВЕРЖДЕНО, WebFetch 2026-10-20]

Позиционирование: «Rust-native VST3 / CLAP plugin host: one engine that loads and plays
plugins, a standalone app, and a headless MCP server for AI agents». Строится послойно:

| Слой | Тип | Назначение |
|---|---|---|
| `nexus-host` | library | Движок: discovery, загрузка, запуск VST3/CLAP; offline-рендер без аудиодрайверов |
| `nexus-tools` | library | Транспорт-агностичные tool-трейты, реестр, state-кодеки, snapshot store, main-thread bridge |
| `nexus-mcp` | binary | Standalone MCP-сервер (stdio или streamable HTTP), полностью headless |
| `nexus-rest` | library | In-process REST API + браузерный debug UI (`http://127.0.0.1:9996/debug`) |
| `nexus-app` | binary | Standalone GUI-хост: egui/eframe (wgpu/Metal), CPAL audio out, MIDI in, редакторы плагинов в нативных окнах |
| `nexus-wrap` | cdylib | Плагин, который хостит другой плагин (стаб на момент проверки) |

Цели проекта (README): единый интерфейс над VST3/CLAP («callers should never branch»),
переиспользуемое ядро, «wrap anything», «programmable state» через MCP.

Экосистема автора «simplyaudio» (footer: *«a one-person research lab»*) [ПОДТВЕРЖДЕНО]:
- **Simply Flux** — модульный синтезатор с AI-ассистентом и собственным MCP-сервером;
  alpha, invite-only, macOS (<https://simply-music.ai/flux>).
- **Simply Droplets** — VST3/CLAP MIDI-эффект-плагин, который сам является MCP-сервером
  (streamable HTTP `localhost:9999/mcp`); macOS/Windows/Linux; MIT;
  репо `poucet/droplets`, 13★, создан 2026-04-18, push 2026-09-04 [ПОДТВЕРЖДЕНО, GitHub API].
- Блог сайта пуст («Nothing written yet») [ПОДТВЕРЖДЕНО].

## 3. Платформы, распространение, цена

- **GUI-приложение: только macOS сейчас** — README прямо пишет «macOS only for now»
  [ПОДТВЕРЖДЕНО]. Linux косвенно заложен: бандлер устанавливает плагины в `~/.clap` и `~/.vst3`.
  Windows в README не упомянута [ПРЕДПОЛОЖЕНИЕ: планируется, но не реализовано].
- **Распространение: только сборка из исходников** — `git clone` + `cargo build --release`.
  Крейтов на crates.io НЕТ (проверены `nexus-host`, `nexus-mcp`, `nexus-tools`, `nexus-app`,
  `nexus-wrap`, `nexus-rest` — все NOT FOUND) [ПОДТВЕРЖДЕНО, crates.io API 2026-10-20].
  GitHub Releases/бинарников нет. Требования: edition 2024, `rustc >= 1.90`.
- **Цена: бесплатный open source, MIT.** Никаких pricing/подписок. Страницы
  `/terms`, `/privacy`, `/pricing`, `/about`, `/docs` на сайте — 404; sitemap.xml и
  robots.txt отсутствуют [ПОДТВЕРЖДЕНО, curl 2026-10-20]. EULA не существует — действует
  только MIT ([LICENSE](https://github.com/poucet/nexus/blob/main/LICENSE)).

## 4. MCP-детали

Источники: README репо + страница /nexus [ПОДТВЕРЖДЕНО].

- **Бинарь:** `nexus-mcp`; headless — без GUI и без аудиодрайвера.
- **Транспорты:** stdio (`--stdio`; конфиг для Claude Desktop / Claude Code / Cursor /
  Antigravity) и **streamable HTTP** (`--port 9997`, endpoint `http://127.0.0.1:9997/mcp`,
  бинд на `127.0.0.1` и `[::1]`).
- **MCP SDK:** официальный Rust `rmcp = 0.9.1` (features `server`,
  `transport-streamable-http-server`, `transport-io`) [ПОДТВЕРЖДЕНО, Cargo.toml].
- **Заявленные tools** (13 шт., из README):
  - `list_plugins`, `load_plugin` — discovery + загрузка CLAP/VST3 из стандартных путей;
  - `list_presets`, `load_preset`, `save_preset` — файлы `.nexuspreset` (identity + state);
  - `list_params`, `get_params`, `set_params` — параметры плагина;
  - `render_audio` — offline-рендер (ноты, CC-автоматизация, parameter ramps) на полной
    скорости CPU → WAV 32-bit float 48 kHz (base64 или файл) + метрики
    `peak_db`, `rms_db`, `duration_s`, `is_silent`;
  - `decode_state`, `edit_state`, `diff_state`, `snapshot_state` — структурный просмотр
    состояния плагина и правки через JSON Pointer.

Ключевая идея: **`render_audio` + state-tools дают агенту «послушать и проверить» результат
без аудиоустройства** — цикл snapshot → tweak → diff описан в `docs/design/STATE_TOOLS.md`.

## 5. Открытость кода и лицензия

- **Код полностью открыт:** <https://github.com/poucet/nexus> [ПОДТВЕРЖДЕНО].
- **Лицензия MIT** (и в GitHub API, и в Cargo.toml: `license = "MIT"`) [ПОДТВЕРЖДЕНО].
- Это ключевой вывод: закрытых компонентов у Nexus нет. Даже Flux — закрытая alpha, но это
  отдельный продукт; сам Nexus и Droplets открыты.

## 6. Технологический стек (важно для VibeStudio)

Из Cargo.toml и README [ПОДТВЕРЖДЕНО]:
- CLAP hosting — [`clack`](https://github.com/prokopyl/clack) (host + plugin API, пин к rev);
- VST3 hosting — чисто-Rust crate `vst3` (COM-биндинги, SDK Штейнберга для хостинга не нужен);
- VST3 export (план) — `clap-wrapper`;
- Audio/MIDI — `cpal` 0.15 + `midir` 0.10 (только в `nexus-app`, движок без device I/O);
- UI — `egui`/`eframe` 0.31 на wgpu (Metal); плагинные редакторы — нативные child windows
  (objc2/AppKit); меню — `muda`; realtime-очередь — `rtrb`;
- REST — `axum` 0.7 + `tokio`;
- общая музыкальная лексика — `simply-fugue` из репо `poucet/music-rs` (git-зависимость).

Архитектурные решения из `docs/PROJECT.md` [ПОДТВЕРЖДЕНО]:
- движок UI-агностичен, app UI заменяем;
- device I/O живёт в app-слое, не в движке (нужно для будущего `nexus-wrap`);
- teardown плагина требует clear/ack-хендшейка с аудиопотоком — иначе segfault;
- цель — «full plugin-extension parity» через единый `PluginInstance`.

## 7. Кто за проектом

[ПОДТВЕРЖДЕНО, GitHub API + simplychris.ai 2026-10-20]
**Christophe Poucet** (GitHub `poucet`, аккаунт с 2009, Лондон). Staff Software Engineer
в **Mistral AI** (IAM-команда); до этого 17 лет в Google (SRE Gmail, Maps routing, Gerrit,
AI code review AutoReview). Персональный сайт <https://www.simplychris.ai>; YouTube
@simplychrisrl; Discord; Substack. Финансирование: **внешнего нет** — личный проект
«one-person research lab», делался в перерыве между Google и Mistral (2025–2026).

## 8. Зрелость

- Репо создан 2026-08-26, коммиты 2026-06-01 → **2026-09-04** (последний push), 86 коммитов,
  1 звезда, 0 форков, 0 issues [ПОДТВЕРЖДЕНО, GitHub API 2026-10-20].
- Версия в Cargo.toml — **0.3.0**; PROJECT.md: 0.1 «Load & Hear» завершён 2026-08-26
  (тег `v0.1.0`), фокус 0.2 «State tools», коммиты упоминают milestone 0.4.
- Третьего сентября 2026 разработка замерла (вероятно, из-за выхода на работу в Mistral
  в мае 2026 — [ПРЕДПОЛОЖЕНИЕ]).
- Отзывов/пресс о Nexus почти нет; у Droplets пост на r/Bitwig [ПОДТВЕРЖДЕНО, WebSearch].
- Вывод: **работающий прототип уровня «один энтузиаст, 3 месяца», не продукт.**
  Но прототип качественный: слоистая архитектура, дизайн-доки, тесты.

## 9. Бонус: AGENTS.md — методология AI-разработки

В корне репо лежит `AGENTS.md` — общая методология запуска AI-агентов на репо [ПОДТВЕРЖДЕНО,
raw.githubusercontent.com 2026-10-20]: 8 постоянных ролей (coder, researcher, tech-lead,
designer, foreman, scribe, integrator, patchbay) с правами на запись по областям; единственный
integrator двигает trunk; «lane rebases itself and stops»; doc-web (BOARD.md и др.);
VCS — jujutsu (jj). Для VibeStudio (1 разработчик + AI-агенты) это **готовый образец
процесса** — ценность не меньше кода.

## 10. Выводы для VibeStudio

1. **Код можно переиспользовать легально (MIT).** `nexus-host` закрывает ровно ту задачу,
   что планирует VibeStudio: CLAP/VST3 hosting в Rust. Варианты: (а) референс для чтения,
   (б) форк/вендоринг движка, (в) точечное заимствование решений (clack + `vst3` crate —
   рабочая связка, подтверждена на практике).
2. **MCP-слой — образец для копирования:** stdio + streamable HTTP, headless-бинарь отдельно
   от GUI, tools на трёх уровнях (plugins/presets → params → state), offline-рендер с
   аудио-метриками как «ухо» агента. Ничего из этого не требует DAW-обвязки — движок сам
   может всё, и это преимущество нативного MCP перед мостами к чужим DAW.
3. **Активность остановилась → не брать как живую зависимость.** Формировать собственный
   движок, используя nexus как снапшот-референс и чеклист подводных камней (clear/ack при
   teardown, main-thread bridge, кэширование probe плагинов, wgpu вместо GL из-за
   плагинных редакторов).
4. **Слоистость = переиспользуемость:** engine без I/O → tools → MCP/REST → app/wrap.
   VibeStudio стоит принять ту же схему, чтобы MCP-сервер был бесплатным следствием
   архитектуры, а не отдельным проектом.
5. **Процесс:** взять роли/doc-web из AGENTS.md как стартовую точку своей агентной
   методологии.

## 11. Открытые вопросы

- Возобновится ли разработка (Mistral AI занимает основное время автора)?
- Windows-сборка `nexus-app`: заявлена только macOS; есть ли планы?
- Будут ли крейты опубликованы на crates.io (сейчас — нет)?
- `nexus-wrap` — заявлен, но «still a stub»: насколько работоспособен?
- Экономика Simply Flux (closed alpha) — монетизирует ли автор вообще что-либо?

---

### Источники (проверены 2026-10-20)

- <https://simply-music.ai/nexus> — страница продукта
- <https://simply-music.ai/flux>, <https://simply-music.ai/droplets>, <https://simply-music.ai/blog> — остальные страницы
- <https://github.com/poucet/nexus> — код, README, Cargo.toml, AGENTS.md, docs/PROJECT.md
- <https://github.com/poucet/droplets> — метаданные
- <https://www.simplychris.ai/> — об авторе
- crates.io API — проверка публикации крейтов
