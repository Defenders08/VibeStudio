# Ландшафт: MCP-серверы для музыки и DAW

**Дата проверки: 2026-10-20.** Все звёзды/лицензии/даты — из GitHub API
(`api.github.com/repos/...`), описания — из README репозиториев. Никакой код не запускался.

Смысл для VibeStudio: мост «AI ↔ DAW» сегодня строят все вокруг чужих DAW через внешние
скриптинговые API. VibeStudio с нативным MCP-движком может дать агенту то, чего ни один
мост дать не может: прямой offline-рендер и доступ к состоянию плагинов.

---

## 1. Сводная таблица

| Проект | Ссылка | Что делает | Язык | Лицензия | ★ | Активность | Урок для VibeStudio |
|---|---|---|---|---|---|---|---|
| ableton-mcp | github.com/ahujasid/ableton-mcp | Управление Ableton Live через MCP: треки, клипы, аранжировка | Python | MIT | 3211 | создан 2025-03-19, push 2026-09-22 | Дистрибуция через PyPI/uvx + Remote Script внутри DAW; самый большой спрос |
| ableton-live-mcp-server | github.com/Simon-Kansara/ableton-live-mcp-server | Ableton Live по OSC (поверх AbletonOSC) | Python | MIT | 394 | push 2025-03-26 (застыл) | OSC — рабочий, но «толстый» мост; ранняя волна проектов уже заброшена |
| fl-studio-mcp | github.com/karl-andres/fl-studio-mcp | FL Studio через MIDI + Piano Roll скрипты | Python | MIT | 228 | push 2026-09-30 | Даже без официального API DAW мост строят через MIDI-скриптинг; «нативность» = преимущество |
| logic-pro-mcp | github.com/MongLong0214/logic-pro-mcp | Stateful, fail-closed контроль Logic Pro, live readback | Swift | MIT | 141 | push 2026-10-20 (ежедневно), v3.18 | Safety-паттерн fail-closed + верификация; отбор в Anthropic Claude for Open Source |
| Reaper-MCP | github.com/xDarkzx/Reaper-MCP | Продакшн/пост в REAPER, 181 tool через ReaScript | Python | Apache-2.0 | 103 | push 2026-10-19 | Много мелких tools (181) — агент сам строит цепочки действий |
| WigAI | github.com/fabb/WigAI | MCP-сервер как Bitwig Controller Extension, HTTP :61169/mcp | Java | MIT | 62 | заархивирован 10.2026, 5+ активных форков | In-DAW MCP-сервер (streamable HTTP) — паттерн подтверждён; спрос на больше tools |
| daw-mcp | github.com/ptaczek/daw-mcp | Мульти-DAW: Bitwig + Ableton из одного MCP | TypeScript | MIT | 21 | push 2026-01-01 (застыл) | Абстракция над несколькими DAW возможна, но нишевая |
| osc-bridge | github.com/roomi-fields/osc-bridge | OSC ↔ MIDI/SysEx мост к 849 железным синтам | Rust | NOASSERTION | 9 | push 2026-08-20 | MCP как шлюз к железу — отдельная ниша для VibeStudio (MIDI-out из движка) |
| droplets | github.com/poucet/droplets | MCP-сервер-плагин (VST3/CLAP) внутри любого DAW | Rust | MIT | 13 | push 2026-09-04 | Плагин-как-MCP-сервер: AI играет MIDI в реальном времени поверх транспорта DAW |

---

## 2. По проектам

**ableton-mcp (ahujasid)** — флагман ниши, 3211★. Архитектура: MCP-сервер ставится из PyPI
(`uvx ableton-mcp`), внутри Ableton Live устанавливается Remote Script (Control Surface),
они общаются по сокету. Умеет: треки, MIDI-клипы, инструменты/эффекты, аранжировку
(«intro/buildup/drop/breakdown/outro»), транспорт. Есть раздел Limitations & Security
Considerations и телеметрия (opt-out). Урок: спрос огромный (3k+ звёзд за год), дистрибуция
через пакетный менеджер + установщик скрипта в DAW — обязательный UX; отдельный репо-паттерн
«один инсталлер-команда».

**ableton-live-mcp-server (Simon-Kansara)** — ранняя альтернатива поверх OSC
(AbletonOSC), 394★, но последний push 2025-03-26 — заброшен. Урок: мосты первого поколения
умирают, когда автор теряет интерес; экосистема смещается к более богатым интеграциям.

**fl-studio-mcp (karl-andres)** — 228★, активен. FL Studio не даёт полноценного внешнего
API, поэтому сервер общается с DAW через MIDI-коммуникацию и скрипты Piano Roll (MIDI
Scripting). Контролирует: транспорт, микшер, channel rack, плагины, степ-секвенсор. Урок:
сообщество выкручивает MCP даже из закрытых DAW; нативный движок с настоящим MCP —
качественно более сильная позиция (VibeStudio может давать агенту состояние семплера,
автоматизацию и рендер, а не только ноты в пианоролле).

**logic-pro-mcp (MongLong0214)** — 141★, Swift, macOS 14+, MCP SDK 0.10, развивается
ежедневно, v3.18.0, отобран в Anthropic «Claude for Open Source». Заявляет **fail-closed**
дизайн (при сомнении в состоянии — отказ действия, а не рискованное предположение) и
**live project readback** (агент читает реальное состояние проекта и верифицирует результат).
Урок: безопасность (fail-closed) и обратная связь (readback/verify) — то, что отличает
зрелый сервер от демо; VibeStudio стоит закладывать readback в ядро, а не надстраивать.

**Reaper-MCP (xDarkzx)** — 103★, Apache-2.0, 181 tool для композиции/микса/поста через
ReaScript. Рядом ещё минимум 5 REAPER-серверов (bonfire-systems 159★, dschuler36 131★,
total-reaper-mcp 87★ «100% ReaScript coverage» и др.) — REAPER из-за честного скриптинга
самая насыщенная MCP-ниша. Урок: рынок проголосовал за **широкий tool-surface** (сотни
мелких операций) — агент сам комбинирует их; альтернатива — 8 «dispatcher»-tools
(koltyj/logic-pro-mcp) coarse-grained. Оба подхода жизнеспособны.

**WigAI (fabb)** — 62★, Java, MCP-сервер внутри Bitwig как Controller Extension,
endpoint `http://localhost:61169/mcp` (streamable HTTP). Архивирован в октябре 2026, но
автор собрал 5+ активных форков с расширениями (devices, presets, browser, notes, arranger,
event streaming, project switching). Урок: (1) in-DAW MCP-сервер по HTTP — проверенный
паттерн; (2) сам факт множества форков — доказательство спроса на глубокий tool-surface;
(3) разрабатывался «BMAD v2 method with AI Agents» — ещё один пример агентной разработки.

**daw-mcp (ptaczek)** — 21★, TypeScript, один MCP поверх Bitwig и Ableton. Застыл с
2026-01-01. Урок: унифицированный API над чужими DAW — упорная работа с малой отдачей;
нативный DAW получает такую унификацию бесплатно.

**osc-bridge (roomi-fields)** — 9★, Rust, мост OSC ↔ MIDI/SysEx к 849 аппаратным синтам и
софту. Лицензия не стандартная (NOASSERTION). Урок: MCP как универсальный музыкальный шлюз
выходит за пределы DAW; для VibeStudio это аргумент держать MIDI/OSC-out движка чистым
API, чтобы агенты могли рулить и внешним железом.

**droplets (poucet)** — 13★, Rust, MIT; VST3/CLAP MIDI-эффект-плагин, который сам является
MCP-сервером (`http://localhost:9999/mcp`, streamable HTTP); «Fugues» — зацикленные
паттерны, синхронизированные с транспортом DAW; MIDI 2.0 (per-note pitch bend, pressure,
16-bit velocity); работает в Bitwig/Ableton/Logic. Автор Nexus. Урок: гибридный паттерн —
даже нативный DAW захочет «MCP-плагин» для внутрь-проектных сущностей; и наоборот, это
показывает, чего лишены все мосты: синхронизации с транспортом хоста.

---

## 3. Паттерны дизайна MCP для DAW

**Архитектура связи.** Доминирует схема «внешний MCP-сервер (stdio) + коннектор внутри
DAW (Remote Script / ReaScript / Extension / OSC) + сокет между ними». Причина: у DAW нет
встроенного MCP, а агентские клиенты (Claude Desktop) исторически запускают stdio-серверы.
In-DAW серверы (WigAI, Droplets) и нативные движки (Nexus) выставляют streamable HTTP на
localhost. Вывод для VibeStudio: **поддерживать оба транспорта** — stdio для совместимости
и HTTP для долгоживущих сессий/веб-UI.

**Tool-surface.** Общий словарь почти всех серверов: транспорт (play/stop/position),
треки/каналы (create/rename/route), клипы и ноты (запись MIDI), микшер (volume/pan/mute/
solo), устройства/параметры (get/set), браузер/загрузка инструментов, сохранение проекта.
Расхождение — гранулярность: сотни мелких tools (Reaper-MCP 181, jpoindexter/ableton-mcp
200+) против немногих coarse «dispatcher» (koltyj 8 tools). Nexus добавляет уровень,
недоступный мостам: `render_audio` с метриками (peak/rms/silence) и state-инструменты
(decode/edit/diff/snapshot по JSON Pointer).

**Safety.** Практики, встречающиеся в зрелых проектах: fail-closed выполнение
(logic-pro-mcp), обязательный readback после действия и верификация (рендер/скриншот →
проверка), секция Limitations/Disclaimer в README, единый экземпляр сервера (ableton-mcp
предупреждает про два клиента одновременно). Разрушительные операции (структура проекта,
удаление треков) — главный риск, аудио-железо — минимальный.

**Дистрибуция.** Python-мир: PyPI + `uvx` одной командой + инсталлер коннектора в DAW.
Java/Bitwig: `.bwextension` файл. Rust (nexus, droplets): `cargo build --release` из
исходников — это трение. Вывод: VibeStudio как приложение вообще не требует установки
сервера — MCP просто включается в настройках; это главное UX-преимущество нативного
подхода.

---

## 4. Выводы для VibeStudio

1. **Ниша доказана деньгами внимания:** только ableton-mcp — 3211★ за ~1.5 года; люди
   реально хотят «сказать DAW, что делать». Но все они — костыли над чужими DAW.
2. **Нативный MCP — не фича, а позиционирование.** Ничто из мостового мира не может дать
   агенту offline-рендер с метриками, структурный diff состояния плагинов и синхронизацию
   с транспортом движка. VibeStudio должен вести tool-surface от ядра: render, state,
   params, presets, MIDI — как у Nexus, плюс DAW-уровень (клипы, аранжировка, микшер).
3. **Оба транспорта с первого дня** (stdio + streamable HTTP на localhost), tools
   версионируемы, каждый tool возвращает достаточно данных для верификации (readback).
4. **Гранулярность tools:** начать с ~20–30 содержательных tools (уровня Nexus 13 +
   транспорт/клипы) и расширять по запросам агентов; dispatcher-подход оставить на потом.
5. **Safety по умолчанию:** dry-run/undo для деструктивных операций, fail-closed,
   метрики после каждого рендера — паттерны уже ожидаемы пользователями.
6. **Процесс:** минимум три проекта из девяти (WigAI, nexus, Reaper-серверы) разработаны
   AI-агентами по формальным методологиям (BMAD, AGENTS.md) — это подтверждает модель
   VibeStudio «1 разработчик + агенты» как рабочую норму отрасли.

### Источники (проверены 2026-10-20)
GitHub API: ahujasid/ableton-mcp, Simon-Kansara/ableton-live-mcp-server,
karl-andres/fl-studio-mcp, MongLong0214/logic-pro-mcp, xDarkzx/Reaper-MCP, fabb/WigAI,
ptaczek/daw-mcp, roomi-fields/osc-bridge, poucet/droplets + README каждого репо.
