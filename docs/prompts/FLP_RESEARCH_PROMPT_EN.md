# Prompt for Claude: Studying the FLP Format and Designing Its Implementation

> Русская версия промпта: `FLP_RESEARCH_PROMPT.md`. Эта английская версия предназначена для передачи Claude — она полностью самодостаточна.

**Purpose:** hand Claude (or any other web-capable agent) the task of deeply studying the proprietary FL Studio project format (`.flp`) and designing its implementation for the VibeStudio DAW (the `flp_compat` module).

**How to use:**
1. Copy the block between `PROMPT START` and `PROMPT END` verbatim into a fresh Claude session (ideally with web access and access to the project filesystem).
2. The prompt is self-contained: it embeds the verified findings of the 2026-10-20 audit, so no from-scratch research is needed — the job is deepening and design.
3. The detailed research notes this prompt is based on: `OSS_AUDIT.md` (sections 3 and 7), `docs/research/flpdiff.md`, `docs/research/flpkit.md`, `docs/research/pyflp.md`. If the agent has access to the project, ask it to read those files before starting.

---

## PROMPT START

You are a file-format reverse engineer and Rust architect. Your mission: study the proprietary FL Studio project format (`.flp`) and design its reading and safe targeted-write implementation for the VibeStudio DAW. Do NOT write the final production code for the whole module yet. Deliverables: (1) verified and deepened knowledge of the format, (2) the design of a `flp_compat` Rust crate, and (3) a testing plan with differential oracles. The implementation itself will be done iteratively by a human developer working with AI coding agents.

### VibeStudio project context

A cross-platform DAW (macOS/Windows/Linux) in Rust, FL Studio-style. The internal project model is our own; FLP is strictly an import/export interchange format, NOT the internal model. Core principles: real-time-safe audio, no false compatibility claims (every capability has a status and tests), never destroy user files (import into a copy, backups, change reports), and strict license discipline. The full project brief lives at `~/Downloads/DAW_Research_and_Claude_Roadmap_Prompt.md` (if accessible); the ecosystem audit is in `OSS_AUDIT.md`.

### Verified findings from the previous research (2026-10-20) — build on these

The following was confirmed by reading the source code of three FLP parsers. Re-verify selectively, but do not start from zero.

**Container structure:**
- File = an `FLhd` chunk (14 bytes total: `FLhd` signature, size = 6, three u16 fields: format version, channel count, PPQ) followed by an `FLdt` chunk containing the event stream.
- Events are a TLV stream: an opcode byte plus payload; the payload size is encoded by opcode range: `< 0x40` → 1 byte (BYTE), `0x40–0x7F` → 2 bytes (WORD), `0x80–0xBF` → 4 bytes (DWORD), `>= 0xC0` → varint length (LEB128) + payload (TEXT/DATA events).

**Map of known events (hex numbers — verified against flpdiff's spec on 2026-10-20, including a full correction pass; still re-check anything you rely on):**
- Scope model (the core of any walker): `0x40` (WORD u16) opens a new channel — iids are contiguous `0..n-1`, and channel-scoped events follow until the next `0x40` or the mixer-boundary `0x62`. `0x41` (WORD) announces the pattern id and **fires twice per pattern** — dedup by id.
- Channels: `0x15` type (BYTE: 0=sampler, 2=instrument/"Native", 3=layer, 4=instrument, 5=automation), `0x80` color (RGBA packed u32), `0xC4` sample path (null-terminated UTF-16LE, absent when no sample), `0xCB` — **scope-sensitive name**: in channel scope (before any `0x62`) it is the channel name, after `0x62` it is the hosted plugin's display name; `0xDB` Levels (24-byte record: int32 pan, uint32 volume, int32 pitch_shift, filter_mod_x/y, filter_type; defaults pan 6400, volume 10000).
- Patterns: `0xC1` name (UTF-16LE), `0x96` color (u32, only when user-set), `0xA4` length, `0x1A` looped, `0xE0` notes (24-byte records), `0xDF` controllers (12 bytes).
- Playlist/arrangement: `0xE9` clip records — **absent when the playlist is empty** (no zero-record blob); 80-byte records (FL 25), 60-byte (FL 21–24), 32-byte (pre-21). The spec picks the size by FL major version (≥25 → 80, ≥21 → 60, else 32) because 80 and 60 are co-divisible at multiples of 240; a wrong pick misaligns the parse (zero-length clips, playback crash). FL 25 record: `pattern_base` always 20480, `_u1` always 120, `item_flags` 0x0040 (high byte 0x80 = selected), `_u2` = `40 64 80 80`, float32 start/end_offset (NaN/0xFFFFFFFF for pattern clips, -1.0 for audio), sequential per-arrangement `clip_id`, float64 `scale` (1.0, at offset 64). Also `0xEE` (track data, opaque ~70 bytes), `0x63`, `0xF1`.
- Mixer: inserts accumulate until `0x93` (u32 — closes the insert AND carries its output routing; sentinels -1 and self-index mean "default"); `0xCC` insert name (UTF-16LE); effect slots close at `0x62` (u16 slot index — slot K's events, including name and `0xD5` state, fire BEFORE its `0x62`; always in groups of 10 per insert, even empty slots); `0xE1` MixerParams — one project-wide blob of 12-byte records with complete attribution (`insertIdx = (channel_data >> 6) & 0x7F`, `slotIdx = channel_data & 0x3F`; ids: 0=SlotEnabled, 1=SlotMix, 64..191=RouteVol, 192=Volume, 193=Pan, 194=StereoSeparation, 208..226=EQ gains/freqs/Qs); `0xEC` insert flags; `0x9A` insert audio input source (same -1 sentinel); `0xE7` routing-related.
- Misc: `0x9C` tempo (BPM×1000), `0x42`+`0x5D` legacy tempo pair (FL < 3.4.0), `0xC9` internal class name (e.g. "Fruity Wrapper"); `0xAC` = **3 raw bytes** (a `byte3` override — the classic DWORD rule reads 4 and swallows the next opcode; example payload `01 01 00`, flpdiff issue #1); time markers: `0x94` position (u32; high bit 0x08000000 = signature kind, low 27 bits = tick), `0xCD` name, `0x21`/`0x22` signature numerator/denominator.
- Not in the spec (need a second source): `0xD4` (flpkit calls event 212 "wrapper"; the spec describes the wrapper as a `0xC9` value plus the `0xD5` record stream), `0x16` routing, the 88-byte FL 2026 playlist record (flpkit only), and the exact cross-version role of `0xC0` (FL 25: UTF-16 version banner; FL 24: per-channel names per the spec).
- Parser self-checks: `FLhd.n_channels` is marked "legacy, ignored by modern parsers" and must NOT be used. Use structural invariants instead: contiguous channel iids from `0x40`, `0x62` in groups of 10 per insert, stopping exactly at `22 + data_len`, plus a candidate phantom-event detector for `0xAC` misreads (see Task 1).

**Versioning traps:**
- FL 25 opcode relocations: notes `0xD0`→`0xE0`, track `0xDE`→`0xEE`, Levels `0xCB`→`0xDB`, insert flags `0xDC`→`0xEC`.
- Playlist clip record sizes grew across eras: 32 B (early) → 60 B (FL 21–24) → 80 B (FL 25) → 88 B (FL 2026, per flpkit — single source, unverified). The spec itself picks the size by FL major version (≥25 → 80, ≥21 → 60, else 32) and warns that 80/60 are ambiguous at multiples of 240 — so combine version selection with a signature validation of the chosen size (flpkit's `pattern_base == 20480` plus the constant fields listed in the map above).
- Event `0xAC` (172): payload size is disputed — flpdiff and PyFLP (issue #208, hex dump `01 01 00`) say 3 bytes, flpkit says 1 byte. The spec confirms **3 bytes** (a `byte3` override). Explain why flpkit's "full-stream consistency" check cannot distinguish the two interpretations, and design parser self-checks — structural invariants (contiguous iids, `0x62` groups of 10, stop at `22 + data_len`) rather than the unreliable `FLhd.n_channels` — that catch the "silent and total" desync described in issue #208.

**Plugin state (hard limitation):**
- `0xD5` is the VST wrapper: name/vendor/path/FourCC are decodable; the state blob itself stays opaque. FL-native plugins are opaque too. Reproducing another project's sound in a foreign host is impossible in principle — only notes/arrangement/structure transfer. The compatibility report must state this honestly.

**Automation is the biggest white spot:**
- Full envelope models (sampler PAHDSR, automation clips) are absent from all open source (PyFLP issue #29); flpdiff declared writing automation a non-goal; flpkit can only replace points inside an existing clip (delta-encoded f64, header/trailer carried verbatim).

**The three existing tools and the license rules:**
- [flpdiff](https://github.com/dawhubapp/flpdiff) — **MIT**. Main value: the clean-room specification [`docs/fl-format/flp-format-spec.md`](https://github.com/dawhubapp/flpdiff/blob/main/docs/fl-format/flp-format-spec.md) (517 lines, per-opcode confidence levels, parity reports), plus [`fl25-event-format.md`](https://github.com/dawhubapp/flpdiff/blob/main/docs/fl-format/fl25-event-format.md), [`plugin-layout-registry.md`](https://github.com/dawhubapp/flpdiff/blob/main/docs/fl-format/plugin-layout-registry.md), and [`fst-format.md`](https://github.com/dawhubapp/flpdiff/blob/main/docs/fl-format/fst-format.md). Its CLI (exit codes 0/1/2) is a ready-made CI oracle; the repo ships 5 public FL 25.2.4 fixtures (`tests/corpus/re_base/fl25/`). The project has been dormant since 2026-07. WARNING: the same GitHub org also hosts a GPL fork of PyFLP — check the LICENSE of every individual file before copying anything.
- [flpkit](https://github.com/clarkipeng/flpkit) — **MIT**, zero-dependency Python. Main value: the write methodology — `locate → encode → splice → _bump_fldt_length → re-read the file → verify_identical`; "detect-don't-assume" (stride/flags/sizes are derived from the file itself); template splicing (new records are built from existing in-file bytes). Key files: `src/flpkit/codec.py`, `detect.py`, `formats/*.py`. The code may be read and ported (~1.8K lines).
- [PyFLP](https://github.com/demberto/PyFLP) — **GPL-3.0**. Do NOT read its code while designing your own implementation (clean room) and NEVER copy it. Permitted: using it as a black-box CI oracle (separate process) for legacy files (FL ≤ 21), and reading its issue tracker (format facts, not code). Frozen since 2023; cannot read FL 2024+ files (issues #208, #205, #200).
- Full breakdowns of the three tools: `docs/research/flpdiff.md`, `docs/research/flpkit.md`, `docs/research/pyflp.md` (if you have project access).

**Field-proven methodology to carry into the Rust design:**
1. **Never reserialize** — splice bytes only; unknown bytes survive by construction. Full re-serialization (PyFLP's `save()` path) is a documented anti-pattern: real FL Studio refuses to open such files.
2. **Verify-by-readback** — a write is not considered successful until the re-read file decodes back into exactly the submitted data (flpkit's `patch()` contract).
3. **Detect-don't-assume** — sizes/strides/flags are derived from the file itself; fallback tables are overridable.
4. **Freeze goldens** and never regenerate them after refactors (differential protection).
5. **Support levels** (from the VibeStudio brief): 1) file/version recognition and safe reading → 2) tempo/PPQ/patterns/notes → 3) channels → 4) playlist/audio clips → 5) automation → 6) mixer graph → 7) plugin states → 8) targeted edits → 9) round-trip saving → 10) differential tests + opening in live FL Studio.

### Task 1 — deepen the format knowledge

- Read the flpdiff spec end to end; produce a consolidated opcode table: number | meaning | size/type | FL versions | confidence level | source (file+line). Reconcile the discrepancies between flpdiff / flpkit / PyFLP issues.
- Dig into the contested areas: `0xAC` (3 vs 1 bytes — explain the false-consistency mechanism), the 88-byte FL 2026 playlist record (what extra fields versus the 80-byte FL 25 record), the `0xC0` project-properties blob, `0xEE` track data, `0xD5` for FL-native plugins, PAHDSR automation structures, musical timing (PyFLP #75).
- Design a **clean-room protocol**: which sources may be read (the MIT spec, flpkit code, issue trackers), which may not (GPL PyFLP code), and how to record it (a provenance log).
- If you have access to a live FL Studio: propose a minimal-pair experiment for `0xAC` and a methodology for building our own fixture corpus (fresh saves across versions, one targeted feature per file).

### Task 2 — design `flp_compat` (Rust)

Deliver ADR-level design, not final code:
- Crates/modules: a TLV reader (streaming, panic-free on corrupt input, byte offsets in errors), an IR (`FlpDocument` — events with original payload bytes preserved, unknown opcodes as opaque blobs), the IR → internal project model mapping, and a compatibility report (supported / partial / opaque / unsupported).
- The implementation order across support levels 1–10; for each level: definition of done and required tests.
- Writer: a surgery-style API (level 1 ops: `set_tempo`, `write_notes`; level 2: playlist clips, channel levels), the verify-by-readback contract, and refusal-to-write semantics when in doubt. Compare the two write strategies — byte splicing (flpkit) vs full rewrite with byte-exact round-trip (flpdiff) — and recommend one for VibeStudio with reasoning.
- Errors: error taxonomy (Format / Unsupported / Corrupted / Ambiguous), strictness policies (strict vs best-effort), and the import report surfaced to the UI.

### Task 3 — testing strategy

- Fixtures: our own corpus (what each file should contain), flpdiff's public fixtures, golden files and the rules for freezing them.
- CI oracles: the `flpdiff` CLI (MIT, standalone binaries) as the differential oracle for modern files; PyFLP (GPL, black box, separate process, legacy FL ≤ 21 only); define how to reconcile outputs from different oracles.
- Round-trip tests: `parse → write → parse` identity; corruption tests (truncated file, broken varint, desync after `0xAC`).
- Ground truth: opening the modified file in live FL Studio — describe how to fold this into the process (manual checklist/script), acknowledging it cannot be automated.

### Task 4 — risks and first steps

- A risk register for the FLP track: format drift, flpdiff dormancy, GPL contamination, no live FL in CI, false compatibility expectations.
- The first 10 tasks for a coding agent, in order; for task #1, give exact verification commands and completion criteria.

### Rules of accuracy

1. Every format fact needs a citation (file+line in a repo, or a URL); tag findings [CONFIRMED] / [INFERENCE] / [HYPOTHESIS] with the verification date.
2. Do not open GPL PyFLP code while designing; format facts come only from the MIT spec, flpkit code, and issue discussions.
3. Promise nothing: express the outcome as a table of "capability → status → evidence".
4. If something could not be verified (no live FL, closed corpus), list it honestly under "open questions".
5. Language: do your research and reasoning in English; the final documents (`docs/FLP_COMPATIBILITY.md`, plus `docs/research/flp-format-notes.md` for research notes if needed) must be written in Russian per project convention, keeping technical terms in English.

## PROMPT END

---

### Appendix: quick links

| What | Link |
|---|---|
| Format specification (MIT) | https://github.com/dawhubapp/flpdiff/blob/main/docs/fl-format/flp-format-spec.md |
| FL25 event format | https://github.com/dawhubapp/flpdiff/blob/main/docs/fl-format/fl25-event-format.md |
| Native plugin layout registry | https://github.com/dawhubapp/flpdiff/blob/main/docs/fl-format/plugin-layout-registry.md |
| .fst preset format | https://github.com/dawhubapp/flpdiff/blob/main/docs/fl-format/fst-format.md |
| Public FL 25 fixtures | https://github.com/dawhubapp/flpdiff/tree/main/tests/corpus/re_base/fl25 |
| flpkit (writer methodology) | https://github.com/clarkipeng/flpkit — key files: `src/flpkit/codec.py`, `detect.py`, `formats/` |
| PyFLP issues (white-spot map; GPL — issues only, no code) | https://github.com/demberto/PyFLP/issues — #208 (`0xAC`), #205/#200 (80-byte playlist), #29 (automation), #75 (timing) |
| Official Image-Line FLP manual | https://www.image-line.com/fl-studio-learning/fl-studio-online-manual/html/fformats_open_flp.htm |
| Ecosystem audit | `OSS_AUDIT.md` (sections 3, 7, 8) |
