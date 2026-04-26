---
name: batch-implement-cards-rust-dsl
description: Archetype- or pool-scoped TDD pipeline that authors YAML card specs in `code/digimon-engine/cards/<set>/` plus DebugRunner tests in `code/digimon-engine/tests/cards_behavioral/<set>/`. Three-wave architecture (Sonnet scout → Sonnet implementer/auditor → Opus reviewer) with per-card mode auto-detection (IMPLEMENT for new YAML / AUDIT for existing YAML / SKIP for prior verdicts). Engine-gap and DSL-vocab-gap routed to separate trackers. Verdicts in `validated_cards_dsl.json`.
argument-hint: <ARCHETYPE_NAME|--pool> [--cards CARD1,CARD2,...] [--batch-size N] [--report-only] [--implementer-model {sonnet,opus}] [--no-audit] [--skip-tests]
---

# Batch Implement Cards (Rust DSL) — Archetype/Pool-Scoped Test-First DSL Pipeline

Author YAML card specs and behavioral tests for an entire archetype (or the curated DSL test pool) using the engine's declarative DSL. Cards are processed in batches of 4 with parallel sub-agents in isolated git worktrees: a Sonnet scout pre-curates context, a Sonnet implementer (or Sonnet auditor for existing YAML) writes tests-first then YAML, and an Opus reviewer audits each batch. **The orchestrator wires test-discovery `mod.rs` files** after merging — agents never touch shared registration.

This is the DSL-flavored sibling of `/batch-implement-cards-rust`. Cards already implemented as hand-written Rust `CardEffect` structs are not affected; cards already shipping YAML are routed to AUDIT mode.

## When to Use

- Authoring DSL YAML for a new archetype on the Rust engine.
- Running the curated `qa/dsl-test-pool.md` end-to-end (pattern-coverage smoke).
- Auditing existing YAML for drift after a DSL phase lands new vocabulary.

**Not for:** hand-written Rust `CardEffect` work (use `/batch-implement-cards-rust`). Not for Python scripts (use `/batch-fix-cards`). Not for gameplay testing (use `/gameplay-qa`). Not for full rewrites of drifted YAML (v1 is audit-only — emits diff proposals, does not modify shipping YAML).

## Flags

| Flag | Default | Purpose |
|---|---|---|
| `--cards CARD1,CARD2,...` | unset | Explicit comma-separated card list. Wins over both archetype and `--pool`. |
| `--batch-size N` | `4` | Per-batch worker count. |
| `--report-only` | off | Phases 1–2 only — resolve, classify, plan, exit. No agents dispatched. |
| `--implementer-model {sonnet,opus}` | `sonnet` | Escape hatch for hard archetypes. Scout stays Sonnet, reviewer stays Opus regardless. |
| `--no-audit` | off | Skip AUDIT-mode dispatch for cards already shipping YAML. They become `SKIP`. |
| `--skip-tests` | off | Emit YAML without behavioral tests. Strongly discouraged — breaks TDD discipline. |

The positional argument is either an archetype name from `data/deck_library.json` or the literal token `--pool` (which targets every card in `qa/dsl-test-pool.md`).

## Per-Card Mode (auto-detected)

- **IMPLEMENT** — no YAML at `code/digimon-engine/cards/<set>/<CARD_ID>.yaml`. Full scout → implementer → reviewer pipeline.
- **AUDIT** — YAML exists. Single Sonnet auditor → reviewer. No scout wave.
- **SKIP** — prior `IMPLEMENTED` or `AUDITED-OK` verdict in `validated_cards_dsl.json`, or `--no-audit` is set and YAML exists.

## Quick Reference

| Resource | Path |
|----------|------|
| DSL test API (canonical for test patterns) | `docs/RUST_DSL_TEST_API.md` |
| DSL syntax + compile pipeline | `docs/superpowers/specs/2026-04-21-card-scripting-dsl.md` |
| Engine API (`EffectContext`) | `docs/RUST_ENGINE_API.md` |
| Production YAML directory | `code/digimon-engine/cards/<set>/<CARD_ID>.yaml` |
| Card test directory | `code/digimon-engine/tests/cards_behavioral/<set>/<card_id_lower>.rs` |
| Test discovery root | `code/digimon-engine/tests/cards_behavioral/main.rs` |
| Test discovery per set | `code/digimon-engine/tests/cards_behavioral/<set>/mod.rs` |
| Card metadata | `data/cards.json` |
| Deck library | `data/deck_library.json` |
| DSL test pool (curated) | `qa/dsl-test-pool.md` |
| C# reference | `DCGO/Assets/Scripts/CardEffect/{SET}/{COLOR}/{CARD_ID_UNDERSCORE}.cs` |
| Verdict tracker | `qa/qa-reports/validated_cards_dsl.json` |
| Engine-gap tracker | `qa/archetype-qa/engine-gaps.md` |
| DSL-vocab-gap tracker | `qa/dsl-vocab-gaps.md` |
| Per-archetype QA artifact | `qa/archetype-qa/dsl/<archetype_slug>.md` |
| Pool-progress artifact | `qa/dsl-test-pool-progress.md` |

## Design spec

`docs/superpowers/specs/2026-04-26-batch-implement-cards-rust-dsl-design.md`

---

## Phase 1: Resolve Card Pool

### 1a. Parse the positional argument

Three resolution paths, in priority order:

1. If `--cards` is set: use the explicit list verbatim. Skip both archetype and pool resolution.
2. If the positional argument is `--pool`: parse `qa/dsl-test-pool.md`. The card IDs are in the first column of the markdown table under `## Pool`. Use this Python one-liner via Bash:

   ```python
   import re, pathlib
   text = pathlib.Path("qa/dsl-test-pool.md").read_text()
   pool_section = text.split("## Pool", 1)[1].split("## ", 1)[0]
   card_ids = re.findall(r"\|\s*`([A-Z0-9-]+)`\s+", pool_section)
   print("\n".join(card_ids))
   ```

3. Otherwise: treat the positional as an archetype name and call `code/tools/resolve_deck.py`:

   ```python
   import sys; sys.path.insert(0, 'code')
   from tools.resolve_deck import resolve_archetype
   manifest = resolve_archetype('ARCHETYPE_NAME')
   for c in manifest.unique_cards:
       print(c.card_id)
   ```

   `manifest.unique_cards` yields `CardEntry` objects with `card_id`, `card_name`, `card_kind`, `level`, `colors`, `traits`, `dp`, `play_cost`, `evo_costs`, `effect_text`, `inherited_text`, `security_text`, `csharp_path`, `deck_frequency`. `manifest.meta_share` and `manifest.best_decklist` come from scraped tournament data. The Python-side `script_status` / `script_path` fields are **not used** — they belong to the legacy Python pipeline and are out of scope here.

   If `resolve_archetype` raises `UnknownArchetype`, surface the message and tell the user to run:

   ```bash
   python code/tools/resolve_deck.py --list-archetypes --min-meta-share 0.01
   ```

   to find a valid archetype name. Exit cleanly.

### 1b. Classify each card by YAML existence

For each `card_id`, compute the lowercased set prefix and check whether YAML exists:

```python
def set_prefix(card_id: str) -> str:
    # 'BT17-001' -> 'bt17'; 'P-117' -> 'p'; 'LM-029' -> 'lm'; 'AD1-025' -> 'ad1'
    return card_id.split('-')[0].lower()
```

Then:

```python
import pathlib
yaml_path = pathlib.Path(f"code/digimon-engine/cards/{set_prefix(card_id)}/{card_id}.yaml")
mode = "AUDIT" if yaml_path.exists() else "IMPLEMENT"
```

If `--no-audit` is set and `mode == "AUDIT"`: change to `SKIP`.

### 1c. Cross-check the verdict tracker

Load `qa/qa-reports/validated_cards_dsl.json`. For each card:

- If the entry's `status` is `IMPLEMENTED` or `AUDITED-OK`: change `mode` to `SKIP` (idempotency). Other verdicts (`PARTIAL`, `AUDITED-MISSING-TESTS`, `AUDITED-DRIFT`, `BLOCKED`) do **not** trigger SKIP — those cards are re-attempted on the next run.

If the file is missing, treat as no prior verdicts (the file was created by the skeleton commit; if it has been deleted, recreate with `{"version":1,"last_updated":"YYYY-MM-DD","cards":{}}`).

### 1d. Build the cross-archetype reverse map

Scan `data/deck_library.json` to produce `{card_id: [archetype_name, ...]}`. This is used in the final report to surface "this card is also used by N other archetypes." Implementation:

```python
import json, pathlib
deck_lib = json.loads(pathlib.Path("data/deck_library.json").read_text())
reverse = {}
for archetype_name, archetype_data in deck_lib.items():
    for decklist in archetype_data.get("decklists", []):
        for entry in decklist.get("cards", []):
            reverse.setdefault(entry["card_id"], set()).add(archetype_name)
reverse = {k: sorted(v) for k, v in reverse.items()}
```

(Adjust the structural path if `deck_library.json` schema diverges; the actual schema lives in `code/tools/meta_loader.py`.)

---

## Phase 2: Batch and Plan

### 2a. Group cards into batches

Default batch size is `--batch-size` (default 4). Apply these grouping heuristics in order:

1. Cards that reference each other by name in their printed text → same batch.
2. A tamer + Digimon it explicitly buffs (by name match in the tamer's text) → same batch.
3. An option card + the Digimon(s) it explicitly targets by name → same batch.
4. Remaining slots filled in card-ID order.

Mixed-mode batches are allowed: a single batch may contain both IMPLEMENT-mode and AUDIT-mode cards. The orchestrator dispatches the right wave per card in Phase 4.

### 2b. Print plan and require approval

Emit the plan table in this exact shape:

```
Archetype: <name or "DSL test pool">
Total cards in pool: <N>
IMPLEMENT (no YAML yet):     <n>
AUDIT (existing YAML):       <n>
SKIP (prior verdict):        <n>
SKIP (--no-audit + YAML):    <n>

To process: <m> → <ceil(m / batch_size)> batches of <batch_size>

Batch 1: <ID1 [I|A]>, <ID2 [I|A]>, <ID3 [I|A]>, <ID4 [I|A]>
Batch 2: ...
...

Note: [I] = IMPLEMENT, [A] = AUDIT
```

**Require explicit user confirmation before Phase 4.** If `--report-only` is set, exit cleanly here.

---

## Phase 3: Pre-Read Shared Context (orchestrator)

Read these files **once** at the start of the skill run and hold them for embedding into every prompt in Phase 4:

1. `docs/RUST_DSL_TEST_API.md` (full)
2. The skill's positive-rules appendix (the section "Skill Positive-Rules Appendix" later in this file)
3. `qa/archetype-qa/engine-gaps.md` (current engine gaps)
4. `qa/dsl-vocab-gaps.md` (current DSL vocab gaps; was created during skill skeleton)

The DSL spec (`docs/superpowers/specs/2026-04-21-card-scripting-dsl.md`, ~58K tokens) and `docs/RUST_ENGINE_API.md` are **cited as paths**, not embedded — workers `Read` them on demand.

Pre-create directories if missing:

```bash
mkdir -p code/digimon-engine/cards
mkdir -p code/digimon-engine/tests/cards_behavioral
mkdir -p qa/archetype-qa/dsl
```

For each unique `<set>` in the planned cards (extracted via `set_prefix(card_id)`):

```bash
mkdir -p code/digimon-engine/cards/<set>
mkdir -p code/digimon-engine/tests/cards_behavioral/<set>
```

### 3a. Pre-wire test discovery for the run (orchestrator)

To let workers run their own tests during the TDD loop in 4C without touching shared state mid-flight, the orchestrator pre-wires `mod.rs` registrations for every non-skipped card in the run **before** dispatching the first batch. Workers can then `cargo test --test cards_behavioral -- <card_id_lower>` immediately.

For each non-skipped card in this skill run:

1. Ensure `code/digimon-engine/tests/cards_behavioral/<set>/mod.rs` exists. Append `mod <card_id_lower>;` if not already present.
2. Ensure `code/digimon-engine/tests/cards_behavioral/main.rs` contains `mod <set>;` for this set. Append if missing.
3. Ensure `code/digimon-engine/tests/cards_behavioral/<set>/<card_id_lower>.rs` exists as an empty file (zero bytes) so the `mod` declaration resolves before the worker writes content. The empty file compiles as a no-op module.

If a worker returns BLOCKED and writes nothing, the orchestrator removes the `mod <card_id_lower>;` line and deletes the empty `.rs` file at merge time (Phase 4E).

**This pre-wire is the only orchestrator write to shared state before agents run.** All other shared-state mutations remain in Phase 4E.

---

## Phase 4: Batch Loop

Repeat for each batch from Phase 2.

### 4A. Per-Card Context Gather (orchestrator)

For each card in the current batch, collect:

1. **Card metadata** from `data/cards.json`:
   - `card_name_eng`
   - `effect_description_eng`
   - `inherited_effect_description_eng`
   - `security_effect_eng`
   - `card_kind`, `level`, `dp`, `play_cost`, `card_colors`, `type_eng` (traits), `evo_costs`, `dna_costs`

2. **DCGO C# reference**: glob `DCGO/Assets/Scripts/CardEffect/<SET>/*/<CARD_ID_UNDERSCORE>.cs` where `<CARD_ID_UNDERSCORE> = card_id.replace("-", "_")` (e.g. `BT15-003` → `BT15_003.cs`). Read the file body if found; record "absent" if not. Promo cards (`P-...`) frequently lack DCGO files; that is acceptable, the worker proceeds with printed text only.

3. **Prior verdict** from `validated_cards_dsl.json` (if any).

4. **AUDIT-mode only** — also read:
   - `code/digimon-engine/cards/<set>/<CARD_ID>.yaml` (the existing YAML body)
   - `code/digimon-engine/tests/cards_behavioral/<set>/<card_id_lower>.rs` if present (existing tests)

`<card_id_lower>` is `card_id.replace("-", "_").lower()` (e.g. `BT15-003` → `bt15_003`, `P-117` → `p_117`, `LM-029` → `lm_029`, `AD1-025` → `ad1_025`).

---

### 4B. Scout Wave (Sonnet, parallel — IMPLEMENT-mode cards only)

For each IMPLEMENT-mode card in the batch, dispatch one Agent call. AUDIT-mode cards skip this wave entirely. All scout calls go in **a single assistant message** for true parallelism. Use `subagent_type: "general-purpose"`, model `sonnet`, no isolation (read-only).

**Scout prompt template:**

````
You are a scout sub-agent for the /batch-implement-cards-rust-dsl skill. Your job is to pre-curate context for an implementer agent that will author the YAML card spec and behavioral tests for a single Digimon TCG card. The implementer is bounded by token budget — your brief replaces the need to embed the full DSL spec.

# Your card

Card ID: {{CARD_ID}}
Card Name: {{CARD_NAME}}
Card Kind: {{CARD_KIND}}
Level: {{LEVEL}}    DP: {{DP}}    Play Cost: {{PLAY_COST}}
Colors: {{COLORS}}
Traits (type_eng): {{TRAITS}}
Evo Costs: {{EVO_COSTS}}
DNA Costs: {{DNA_COSTS}}

## Effect text (authoritative)
{{EFFECT_TEXT}}

## Inherited effect text
{{INHERITED_TEXT}}

## Security effect text
{{SECURITY_TEXT}}

## DCGO C# reference (behavioral source of truth)
Path: {{CSHARP_PATH}}
```
{{CSHARP_BODY}}
```

# Reference docs (cite paths — Read what you need, do not paste full bodies)

- DSL test API (test patterns + anti-patterns): `docs/RUST_DSL_TEST_API.md`
- DSL syntax + compile pipeline (vocabulary reference): `docs/superpowers/specs/2026-04-21-card-scripting-dsl.md`
- Engine API (`EffectContext` surface): `docs/RUST_ENGINE_API.md`
- Existing shipping YAMLs: `code/digimon-engine/cards/`
- Curated example pool: `code/digimon-engine/cards/_examples/`

# Your task

Produce a curated brief for the implementer. The brief must:

1. **Classify the card's mechanics** against the pattern taxonomy in `docs/RUST_DSL_TEST_API.md` §4.3 (Groups A–H). List every applicable row tag.

2. **Identify the DSL verbs / step kinds the implementer will need.** For each verb, cite the section of the DSL spec where it is defined. If the verb does not appear in the spec, mark it as a candidate DSL-vocab gap (do NOT speculate it exists).

3. **Find 1–2 closest exemplar YAMLs** from `code/digimon-engine/cards/` (production) or `code/digimon-engine/cards/_examples/` (curated). Cite paths and a one-line "why this is the closest match."

4. **Identify the target engine APIs.** For each DSL verb, name the `EffectContext` method it lowers to per `docs/RUST_ENGINE_API.md`.

5. **Sketch behavioral test scope per `docs/RUST_DSL_TEST_API.md` §5.** Enumerate: structural assertions (clauses by scope/timing), per-branch behavioral tests, negative tests, OPT enforcement test (if applicable), event-log test (if applicable).

6. **Pre-flight gap suspicion.** Emit one of:
   - `NONE` — no gap suspected.
   - `ENGINE-GAP: <description>` — engine lacks a primitive (the DSL verb you would use lowers to a method that does not exist in `EffectContext`).
   - `DSL-GAP: <description>` — engine has the primitive but no DSL verb maps to it.
   - `HYBRID: <description>` — both.
   You may return any verdict here; the implementer will confirm or refute.

# Source priority (for behavioral questions)

1. Printed card text (above) — authoritative.
2. `docs/RULES_CONTEXT.md` and fandom wiki — keyword + interaction semantics.
3. DCGO C# (above) — implementation-detail tiebreaker only.

Do NOT cite Python scripts (`code/engine_py_legacy/`) — they are out of scope for this skill.

# Output format (return EXACTLY this structure, nothing else)

```
## Brief: {{CARD_ID}}

### Pattern rows (test API §4.3)
- <row tag>, <row tag>, ...

### Required DSL verbs / step-kinds
- <verb_name> → DSL spec §X.Y [+optional usage note]
- ...

### Closest exemplar YAMLs
1. <path> — <one-line why>
2. <path> — <one-line why>

### Target engine APIs (from RUST_ENGINE_API.md)
- EffectContext::<method_name>
- ...

### Behavioral test scope (test API §5)
- Structural: <clause counts by scope/timing>
- Per-branch: <enumerate>
- Negative tests: <enumerate>
- OPT lockout: <yes/no — if yes, which clause>
- Event-log assertions: <yes/no — if yes, which events>

### Pre-flight gap suspicion
NONE | ENGINE-GAP: <description> | DSL-GAP: <description> | HYBRID: <description>
```
````

The orchestrator validates that the returned brief contains all six section headings before passing it to the implementer. If validation fails, dispatch a single retry with the same prompt. If the retry also fails, fall back to embedding `docs/superpowers/specs/2026-04-21-card-scripting-dsl.md` directly into the implementer prompt for that one card.

**Pre-flight gap short-circuit:** if a scout returns `ENGINE-GAP`, `DSL-GAP`, or `HYBRID` with high confidence (the brief explicitly states the missing API/verb), the orchestrator MAY skip the implementer wave for that card and emit a `BLOCKED` verdict directly. The reviewer still confirms the gap in Phase 4D.

---
