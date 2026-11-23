---
name: batch-implement-cards-rust-dsl
description: Archetype- or pool-scoped TDD pipeline that authors YAML card specs in `code/digimon-engine/cards/<set>/` plus DebugRunner tests in `code/digimon-engine/tests/cards_behavioral/card_text/`. Three-wave architecture (Sonnet scout → Sonnet implementer/auditor → Opus reviewer) with per-card mode auto-detection (IMPLEMENT for new YAML / AUDIT for existing YAML / SKIP for prior verdicts). Engine-gap and DSL-vocab-gap routed to separate trackers. Verdicts in `validated_cards_dsl.json`.
argument-hint: <ARCHETYPE_NAME|--pool> [--cards CARD1,CARD2,...] [--batch-size N] [--report-only] [--implementer-model {sonnet,opus}] [--no-audit] [--skip-tests]
---

# Batch Implement Cards (Rust DSL) — Archetype/Pool-Scoped Test-First DSL Pipeline

Author YAML card specs and behavioral tests for an entire archetype (or the curated DSL test pool) using the engine's declarative DSL. Cards are processed in batches of 4 with parallel sub-agents in isolated git worktrees: a Sonnet scout pre-curates context, a Sonnet implementer (or Sonnet auditor for existing YAML) writes tests-first then YAML, and an Opus reviewer audits each batch. **The orchestrator wires test-discovery `mod.rs` files** after merging — agents never touch shared registration.

This is the DSL-flavored sibling of `/batch-implement-cards-rust`. Cards already implemented as hand-written Rust `CardEffect` structs are not affected; cards already shipping YAML are routed to AUDIT mode.

## When to Use

- Authoring DSL YAML for a new archetype on the Rust engine.
- Running the curated `qa/dsl-test-pool.md` end-to-end (pattern-coverage smoke).
- Auditing existing YAML for drift after a DSL phase lands new vocabulary.

**Not for:** hand-written Rust `CardEffect` work (use `/batch-implement-cards-rust`). Not for Python scripts (use `/batch-fix-cards`). Not for gameplay testing (use `/gameplay-qa`). Not for unrelated full rewrites. AUDIT fixes clear printed-text mismatches using existing DSL vocabulary; ambiguous rulings or missing vocabulary are findings to escalate.

## Flags

| Flag | Default | Purpose |
|---|---|---|
| `--cards CARD1,CARD2,...` | unset | Explicit comma-separated card list. Wins over both archetype and `--pool`. |
| `--batch-size N` | `4` | Per-batch worker count. |
| `--report-only` | off | Phases 1–2 only — resolve, classify, plan, exit. No agents dispatched. |
| `--implementer-model {sonnet,opus}` | `sonnet` | Escape hatch for hard archetypes. Scout stays Sonnet, reviewer stays Opus regardless. |
| `--no-audit` | off | Skip AUDIT-mode dispatch for cards already shipping YAML. They become `SKIP`. |
| `--skip-tests` | off | Not compatible with IMPLEMENT/AUDIT card-text work; reject rather than bypass the first-run gate. |

The positional argument is either an archetype name from `data/deck_library.json` or the literal token `--pool` (which targets every card in `qa/dsl-test-pool.md`).

## Per-Card Mode (auto-detected)

- **IMPLEMENT** — no YAML at `code/digimon-engine/cards/<set>/<CARD_ID>.yaml`. Full scout → implementer → reviewer pipeline.
- **AUDIT** — YAML exists. Single Sonnet auditor → reviewer. No scout wave.
- **SKIP** — prior `IMPLEMENTED` or `AUDITED-OK` verdict in `validated_cards_dsl.json`, or `--no-audit` is set and YAML exists.

## Card-text-first contract (IMPLEMENT and AUDIT)

Before the worker has YAML or existing tests in context, it reads only `data/card_bundles/<ID>.md` plus rules, writes `tests/cards_behavioral/card_text/<id>.rs`, and runs it once. Quote all printed sections and official Q&A, use at least one test per printed clause, cover every “may” decline and each Q&A, and drive real actions with explicit prompt helpers. Template: `card_text/bt17_015.rs` and docs/RUST_DSL_TEST_API.md, “Card-text tests”. No `auto_resolve*`, wrappers, or `enqueue_triggered`.

The orchestrator must withhold scout implementation analysis, YAML bodies, existing test bodies and DCGO bodies until the first run is recorded. Do not embed them in the initial worker prompt. Keep the scout brief for a second handoff after that run (or provide a deferred path only). In AUDIT, a failing card-text test is a finding: fix clear YAML mismatches with existing vocabulary, preserve correct printed-text assertions, and report ambiguous or capability-blocked cases rather than approximating.

## Quick Reference

| Resource | Path |
|----------|------|
| DSL test API (canonical for test patterns) | `docs/RUST_DSL_TEST_API.md` |
| DSL syntax + compile pipeline | `docs/superpowers/specs/2026-04-21-card-scripting-dsl.md` |
| Engine API (`EffectContext`) | `docs/RUST_ENGINE_API.md` |
| Production YAML directory | `code/digimon-engine/cards/<set>/<CARD_ID>.yaml` |
| Card test directory | `code/digimon-engine/tests/cards_behavioral/card_text/<card_id_lower>.rs` |
| Test discovery root | `code/digimon-engine/tests/cards_behavioral/main.rs` |
| Card-text test discovery | `code/digimon-engine/tests/cards_behavioral/card_text/mod.rs` |
| Printed text, evolution data and official Q&A (initial authoring source) | `data/card_bundles/<ID>.md` |
| Supplemental per-card metadata (after first test run) | `code/digimon-engine/cards/<set>/<CARD_ID>.json` |
| Aggregate card metadata (fallback only) | `data/cards.json` |
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
mkdir -p code/digimon-engine/tests/cards_behavioral/card_text
mkdir -p qa/archetype-qa/dsl
```

For each unique `<set>` in the planned cards (extracted via `set_prefix(card_id)`):

```bash
mkdir -p code/digimon-engine/cards/<set>
```

### 3a. Pre-wire test discovery for the run (orchestrator)

To let workers run their own tests during the TDD loop in 4C without touching shared state mid-flight, the orchestrator pre-wires `mod.rs` registrations for every non-skipped card in the run **before** dispatching the first batch. Workers can then `cargo test --test cards_behavioral -- <card_id_lower>` immediately.

For each non-skipped card in this skill run:

1. Ensure `code/digimon-engine/tests/cards_behavioral/card_text/mod.rs` exists. Append `mod <card_id_lower>;` if not already present.
2. Ensure `code/digimon-engine/tests/cards_behavioral/main.rs` contains `mod card_text;`. Append if missing.
3. Ensure `code/digimon-engine/tests/cards_behavioral/card_text/<card_id_lower>.rs` exists as an empty file (zero bytes) so the `mod` declaration resolves before the worker writes content. The empty file compiles as a no-op module.

If a worker returns BLOCKED and writes nothing, the orchestrator removes the `mod <card_id_lower>;` line and deletes the empty `.rs` file at merge time (Phase 4E).

**This pre-wire is the only orchestrator write to shared state before agents run.** All other shared-state mutations remain in Phase 4E.

---

## Phase 4: Batch Loop

Repeat for each batch from Phase 2.

### 4A. Per-Card Context Gather (orchestrator)

For each card in the current batch, collect:

1. **Initial authoring context:** `data/card_bundles/<ID>.md` (identity, evolution requirements, every printed section and official Q&A), plus relevant rules. If the bundle is missing, report a data blocker rather than silently changing sources.
2. **Deferred implementation context:** scout analysis, DSL examples, DCGO reference, prior verdict, and supplemental metadata. Hold this out of the worker's context until it has written and run the card-text tests.
3. **AUDIT only, also deferred:** existing YAML and existing tests (both canonical and legacy set-based tests). Pass paths, not bodies, initially; permit reading only after the recorded first run.

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

## DCGO C# reference (deferred implementation reference)
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

Produce a curated brief for the implementer's post-first-run phase. Do not embed YAML or existing tests in the brief; cite deferred paths only. The orchestrator must not pass this implementation analysis to the worker before the printed-text tests are written and run. The brief must:

1. **Classify the card's mechanics** against the pattern taxonomy in `docs/RUST_DSL_TEST_API.md` §4.3 (Groups A–H). List every applicable row tag.

2. **Identify the DSL verbs / step kinds the implementer will need.** For each verb, cite the section of the DSL spec where it is defined. If the verb does not appear in the spec, mark it as a candidate DSL-vocab gap (do NOT speculate it exists).

3. **Find 1–2 closest exemplar YAMLs** from `code/digimon-engine/cards/` (production) or `code/digimon-engine/cards/_examples/` (curated). Cite paths and a one-line "why this is the closest match."

4. **Identify the target engine APIs.** For each DSL verb, name the `EffectContext` method it lowers to per `docs/RUST_ENGINE_API.md`.

5. **Sketch post-first-run coverage review per test API §5.** Enumerate printed clauses, every choice/decline and Q&A, negative cases, OPT and duration checks, and cost events. Structural assertions are optional and discouraged. Do not derive initial test expectations from YAML.

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
- Printed clauses and Q&A: <enumerate from bundle>
- Per-branch: <enumerate>
- Negative tests: <enumerate>
- OPT lockout: <yes/no — if yes, which clause>
- Event-log assertions: <yes/no — if yes, which events>

### Pre-flight gap suspicion
NONE | ENGINE-GAP: <description> | DSL-GAP: <description> | HYBRID: <description>
```
````

The orchestrator validates that the returned brief contains all six section headings before passing it to the implementer. If validation fails, dispatch a single retry with the same prompt. If the retry also fails, provide the DSL spec path for the post-first-run handoff for that card. Never bypass the printed-text-first gate.

**Pre-flight gap short-circuit:** if a scout returns `ENGINE-GAP`, `DSL-GAP`, or `HYBRID` with high confidence (the brief explicitly states the missing API/verb), the orchestrator MAY skip the implementer wave for that card and emit a `BLOCKED` verdict directly. The reviewer still confirms the gap in Phase 4D.

---

### 4C. Implementer / Auditor Wave

For each non-skipped card in the batch, dispatch one Agent call in a single assistant message. Each agent runs with `isolation: "worktree"`. Model is `sonnet` by default; if `--implementer-model opus` was set, IMPLEMENT-mode workers use `opus`. AUDIT-mode workers always use `sonnet`.

#### 4C.1 Implementer prompt (IMPLEMENT mode)

````
You are implementing a single Digimon TCG card effect using the engine's declarative YAML DSL via TDD.

# Your card

Read `data/card_bundles/{{CARD_ID}}.md` for printed text, evolution data and official Q&A.

## Deferred context (after first test run only)
DCGO path: {{CSHARP_PATH}}. The controller withholds the scout brief and prior verdict for the post-first-run handoff. Do not read or request those bodies before writing and running the bundle-derived tests.

# Engine context pack

## RUST_DSL_TEST_API.md (full — canonical for test patterns)
{{RUST_DSL_TEST_API_BODY}}

## Skill positive-rules appendix
{{SKILL_POSITIVE_RULES_APPENDIX}}

## Hybrid checklist (apply to every clause)
The hybrid checklist is the union of:
- All anti-patterns in `docs/RUST_DSL_TEST_API.md` §11 (already embedded above).
- All positive rules in the appendix (already embedded above).

## Current engine gaps (consult before declaring BLOCKED)
{{ENGINE_GAPS_BODY}}

## Current DSL vocab gaps (consult before declaring BLOCKED)
{{DSL_VOCAB_GAPS_BODY}}

## Read-on-demand
- DSL spec (verb definitions, lowering rules): `docs/superpowers/specs/2026-04-21-card-scripting-dsl.md`
- Engine API (`EffectContext`, `Effect`, modifier types, timing enums): `docs/RUST_ENGINE_API.md`

When you need a specific verb's parameters or an `EffectContext` method signature, Read the relevant section of the document above. Do not guess.

# Source priority (for behavioral questions)

1. Printed card text — authoritative.
2. `docs/RULES_CONTEXT.md` and fandom wiki — keyword + interaction semantics.
3. DCGO C# — implementation-detail tiebreaker only.

Do NOT cite Python scripts (`code/engine_py_legacy/`).

# Your task

Deliverables (and ONLY these — do NOT touch any `mod.rs`, `main.rs`, `cards.rs`, or any tracker file):

1. `code/digimon-engine/cards/{{SET_LOWER}}/{{CARD_ID}}.yaml` — the DSL card spec.
2. `code/digimon-engine/tests/cards_behavioral/card_text/{{CARD_ID_LOWER}}.rs` — DebugRunner behavioral tests.

`{{CARD_ID_LOWER}}` is `{{CARD_ID}}.replace("-", "_").lower()` (e.g. `BT15-003` → `bt15_003`).

## Workflow (TDD-strict — follow in order)

**Step 1 — Read only the bundle and rules; decompose printed clauses.**
Before reading YAML, old tests, scout implementation analysis or DCGO, capture each timing, exact printed text, expected behavior, every “may” and official Q&A.

**Step 2 — Write the test file FIRST.**

Create `code/digimon-engine/tests/cards_behavioral/card_text/{{CARD_ID_LOWER}}.rs` per the test API's Card-text tests section and §5. The mandatory header quotes the bundle's printed sections and official Q&A verbatim and states its source path. Template: `card_text/bt17_015.rs`.

Cover, at minimum:
- Section 1: Optional structural assertions are discouraged; they restate YAML and do not prove printed behavior.
- Section 2: Condition gating — one positive AND one negative test per condition (splitting is non-negotiable).
- Section 3: Real-action behavioral outcomes per printed clause, every choice and “may” decline, and each official Q&A. Assert exact candidates and every prompt with the explicit helpers; no `auto_resolve*` or `enqueue_triggered`.
- Section 4: For cost-firing clauses, an event-log assertion via `events_since(checkpoint)`.
- Section 5: For OPT clauses, an explicit lockout test (second activation gated; lockout clears after `end_turn`).

Use `dsl_card("{{CARD_ID}}")` to register the card under test. Do NOT inline-paste production YAML.

Use `digimon_engine::action::space::*` constants for action IDs. Do NOT hard-code.

**Step 3 — Run the tests, confirm expected failures.**

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- {{CARD_ID_LOWER}}
```

The orchestrator has pre-wired `tests/cards_behavioral/card_text/mod.rs` and `main.rs` so this command resolves your test file immediately. Confirm the tests fail with the expected reasons (the YAML doesn't exist yet, so the embedded pack lookup fails when `dsl_card("{{CARD_ID}}")` runs).

**Step 4 — Author the YAML.**

Only after recording the first run, obtain the deferred scout brief and implementation references. You may now read example YAML, old tests and DCGO. Create `code/digimon-engine/cards/{{SET_LOWER}}/{{CARD_ID}}.yaml`. Start from one of the exemplar YAML(s) the scout cited as the closest match. Adapt the structure to the printed card text. Use only DSL verbs the scout cited (or that you have verified by Reading the DSL spec).

**Security icons — check before authoring.** A pink `{Security}` effect (in the official text: `{Security} [Timing] …`, or the older `[Security] [Timing] …` inside the main effect box) is active while the card is FACE UP in the security stack → `scope: security` + its printed timing, or `kind: aura, scope: security`. A blue `[Security]` effect (the "Security Effect" section, `[Security] Play this card …`) fires on a security check → `when: on_security`. Never author a pink effect as `when: on_security`. Tests for pink effects must stage the card face up (`face_up_security.insert(card_index)`). The CI lint `tests/dsl/security_icon_lint.rs` fails the build on a mismatch — see `docs/RUST_DSL_AGENT_GUIDE.md` §"Security Icons".

If you discover a needed verb is not in the DSL spec, STOP and emit verdict `BLOCKED` with `gap_kind: dsl` (or `engine` / `hybrid` per the diagnosis below).

**Step 5 — Re-run tests until green.**

Iterate. If a test reveals a YAML bug, fix the YAML. If a test reveals a test bug, fix the test. Both are valid.

**Step 6 — Faithfulness self-audit against the hybrid checklist.**

For each item in `docs/RUST_DSL_TEST_API.md` §11 plus the skill positive-rules appendix, confirm compliance. Note any unresolved item in your verdict block.

**Step 7 — Diagnose `gap_kind` if BLOCKED.**

If you reached `BLOCKED`:
- `gap_kind: engine` — the DSL would need a verb that lowers to an `EffectContext` method that does NOT exist (verify by reading `docs/RUST_ENGINE_API.md`).
- `gap_kind: dsl` — the `EffectContext` method exists, but no DSL verb / step kind / predicate maps to it (verify by reading the DSL spec).
- `gap_kind: hybrid` — both: a new DSL verb is needed AND a new engine method is needed.

**Step 8 — Emit verdict.**

# Verdicts

- `IMPLEMENTED` — every clause implemented faithfully; all tests pass.
- `PARTIAL` — core clauses work; some nuance deferred with explicit comment. Explain precisely what's missing and why.
- `BLOCKED` — a required mechanic is unavailable. Set `gap_kind` per Step 7.

**Never ship stubs, auto-selections, or silent drops.** If a clause requires a player choice the engine cannot yet surface, the card is BLOCKED — not PARTIAL.

# Output format (return EXACTLY this structure)

```
## {{CARD_ID}} — {{CARD_NAME}}

### Verdict: IMPLEMENTED | PARTIAL | BLOCKED
### Gap kind (if BLOCKED): engine | dsl | hybrid
### Scout-disagreement (if any): <description>

### Clause analysis
Clause 1 (<timing>): "<exact text>"
  Expected: <behavior>
  YAML location: <CARD_ID>.yaml lines X–Y
  Tests: <test_fn_name_1>, <test_fn_name_2>, ...
  Status: MATCH | PARTIAL | BLOCKED
Clause 2 ...

### Files written
- code/digimon-engine/cards/{{SET_LOWER}}/{{CARD_ID}}.yaml (N clauses, M lines)
- code/digimon-engine/tests/cards_behavioral/card_text/{{CARD_ID_LOWER}}.rs (N tests)

### Test output (final cargo test summary, trimmed)
<paste the relevant lines from your final cargo test run>

### Engine gaps discovered (if any)
## {{CARD_ID}} — <clause name>
Missing API: <description>
Suggested addition: <signature on EffectContext>

### DSL vocab gaps discovered (if any)
## {{CARD_ID}} — <clause name>
Missing verb / step kind / predicate: <description>
Lowers to engine API: <which existing EffectContext method>
Suggested DSL syntax: <YAML shape>

### New patterns worth documenting in RUST_DSL_TEST_API.md (if any)
- <pattern>: <description>
```
````

---

#### 4C.2 Auditor prompt (AUDIT mode)

AUDIT-mode workers always use `sonnet` (the `--implementer-model` flag does not affect them — auditing is a bounded task).

````
You are auditing an existing DSL YAML card against printed text. First write and run independent card-text tests from the bundle and rules alone. A failing test is a finding; fix clear YAML mismatches using existing DSL vocabulary, and report ambiguous or capability-blocked cases.

# Your card

Read `data/card_bundles/{{CARD_ID}}.md` for all printed sections and official Q&A.

## Deferred references — do NOT open before writing and running tests
- YAML: `code/digimon-engine/cards/{{SET_LOWER}}/{{CARD_ID}}.yaml`
- Canonical tests: `code/digimon-engine/tests/cards_behavioral/card_text/{{CARD_ID_LOWER}}.rs`
- Legacy tests: `code/digimon-engine/tests/cards_behavioral/{{SET_LOWER}}/{{CARD_ID_LOWER}}.rs`
- DCGO: {{CSHARP_PATH}}
No reference bodies or scout implementation analysis belong in this initial prompt.

# Engine context pack

## RUST_DSL_TEST_API.md (full)
{{RUST_DSL_TEST_API_BODY}}

## Skill positive-rules appendix
{{SKILL_POSITIVE_RULES_APPENDIX}}

# Source priority (for behavioral questions)

1. Printed card text — authoritative.
2. `docs/RULES_CONTEXT.md` and fandom wiki — keyword + interaction semantics.
3. DCGO C# — implementation-detail tiebreaker only.

Do NOT cite Python scripts.

# Your task

0. **Independent card-text tests FIRST.** Before opening any deferred reference, write `code/digimon-engine/tests/cards_behavioral/card_text/{{CARD_ID_LOWER}}.rs` from the bundle and rules, quoting all printed text and Q&A. Cover §5 sections 2–5: conditions, real-action outcomes, every “may” decline and Q&A, cost events, OPT and duration. Structural assertions (§5 section 1) are optional and discouraged. Use explicit prompt helpers; no `auto_resolve*` or `enqueue_triggered`. Run `cargo test -p digimon-engine --test cards_behavioral card_text::{{CARD_ID_LOWER}}` and record the output before opening YAML or old tests. If canonical tests already exist, keep their body out of context until this independent first pass; then reconcile without weakening correct assertions.

1. **Faithfulness diff (after first run).** Walk every clause in the printed text. For each, locate the corresponding section in the YAML. Identify any:
   - Silently dropped clauses (printed text says it, YAML doesn't model it).
   - Missing branches (printed text offers a choice the YAML reduces to one option).
   - Optionality mismatches (printed text "you may"; YAML mandatory, or vice versa).
   - Condition gaps (printed text "if X"; YAML unconditional, or wrong condition).
   - OPT misses ([Once Per Turn] in text but YAML lacks `once_per_turn: true`).

2. **Behavioral fidelity diff against DCGO C#.** For nuances printed text doesn't pin down (e.g., processing order of an interaction, exact target eligibility), confirm the YAML matches DCGO. Per CLAUDE.md source priority, DCGO is a tiebreaker — printed text wins on disagreements. Note any printed-vs-DCGO disagreement explicitly.

3. **Test coverage inventory.** Compare the existing test file (if any) against the test API §5 expected coverage:
   - Section 1: compiled-shape assertions are optional and discouraged, not a coverage requirement.
   - Section 2: each condition has BOTH positive AND negative test?
   - Section 3: every printed clause, choice, “may” decline and official Q&A has a real-action test with explicit prompts?
   - Section 4: cost-firing clauses have event-log assertions?
   - Section 5: OPT clauses have an explicit lockout test?

4. **Emit verdict:**
   - `AUDITED-OK` — YAML faithful, tests cover §5 expectations.
   - `AUDITED-MISSING-TESTS` — YAML faithful, but tests are incomplete. Add the missing tests; emit the new file.
   - `AUDITED-DRIFT` — a printed-text failure exposed YAML drift. Fix it with existing vocabulary when the text is clear, rerun canonical and existing per-card tests, and report the applied diff. Escalate ambiguous rulings or missing vocabulary instead of approximating.
   - `BLOCKED` — same diagnosis criteria as the implementer (gap_kind required).

# Output format

```
## {{CARD_ID}} — {{CARD_NAME}} — AUDIT

### Verdict: AUDITED-OK | AUDITED-MISSING-TESTS | AUDITED-DRIFT | BLOCKED
### Gap kind (if BLOCKED): engine | dsl | hybrid

### Faithfulness diff (if AUDITED-DRIFT or AUDITED-OK with notes)
Clause 1 (<timing>): "<printed text>"
  YAML says: <what's there>
  Should say: <correction>
  Source: printed text | DCGO C# line N

(Repeat per drifted clause. If AUDITED-OK, this section may be empty or note "no drift detected.")

### Test coverage inventory
- Structural: <optional; not a coverage requirement>
- Printed clauses, every choice/decline and Q&A: <covered | missing — list>
- Condition gating: <present | missing — list>
- Behavioral integrated: <present | missing — list>
- Event-log (if applicable): <present | missing | n/a>
- OPT lockout (if applicable): <present | missing | n/a>

### Tests added or corrected (including drift regressions)
- <test_fn_name_1>
- <test_fn_name_2>

### Files written/modified
- code/digimon-engine/tests/cards_behavioral/card_text/{{CARD_ID_LOWER}}.rs (added N tests, total now M)
- code/digimon-engine/cards/{{SET_LOWER}}/{{CARD_ID}}.yaml (applied printed-text fix, or unchanged)

### Test output
<initial card-text summary before reading YAML/old tests>
<final card-text and existing per-card summaries after any fix>
```
````

---

### 4D. Review Wave (Opus, single agent, no isolation, read-only)

After all worker agents in 4C return, the orchestrator copies their files out of the worktrees into the main tree (Phase 4E will document this; the reviewer reads from main tree). Then dispatch ONE Opus reviewer.

**Reviewer prompt template:**

````
You are reviewing DSL YAML and behavioral tests authored by N worker agents in a TDD pipeline. Your job is to confirm faithfulness, completeness, and adherence to the hybrid checklist.

# Engine context pack

## RUST_DSL_TEST_API.md (full)
{{RUST_DSL_TEST_API_BODY}}

## Skill positive-rules appendix
{{SKILL_POSITIVE_RULES_APPENDIX}}

## Hybrid checklist
The hybrid checklist is the union of `docs/RUST_DSL_TEST_API.md` §11 anti-patterns plus the positive rules in the appendix. Apply both in full to every card.

# Per-card review materials

For each card in this batch, you have:
- The worker's verdict block (below).
- The files they wrote: `code/digimon-engine/cards/<set>/<CARD_ID>.yaml` and `code/digimon-engine/tests/cards_behavioral/card_text/<card_id_lower>.rs`.
- The card's authoritative metadata + printed text.
- The DCGO C# reference body.

{{PER_CARD_MATERIALS — for each card, repeat the metadata block + worker verdict + paths to written files}}

# Source priority (for behavioral questions)

1. Printed card text — authoritative.
2. `docs/RULES_CONTEXT.md` and fandom wiki — keyword + interaction semantics.
3. DCGO C# — implementation-detail tiebreaker only.

# Your task — for each card, emit ONE of:

```
<CARD_ID>: APPROVED
```

or

```
<CARD_ID>: NEEDS-FIX
  - Issue 1: <description> — Fix: <file:line directive>
  - Issue 2: ...
```

Be precise — the orchestrator applies your fix directives verbatim. Each directive must specify exact file path, line range, and the change required.

# What to check

For IMPLEMENT-mode cards:
- Hybrid checklist: every item applied. No silent drops, no auto-selections, no missing branches.
- Test API §5 coverage: printed clauses, every choice/decline and Q&A, positive/negative conditions, OPT/duration and cost events. Structural assertions are optional and discouraged.
- Scout-vs-implementer disagreement: if scout flagged a gap and implementer disagreed, adjudicate. Check whether the engine API or DSL verb the implementer used actually exists.
- Faithfulness: every clause in printed text is modeled in YAML; printed-vs-DCGO disagreements resolved per source priority.
- TDD discipline: test file exists; tests are split positive/negative; behavioral tests are integrated (not just clause-isolated).

For AUDIT-mode cards:
- AUDITED-OK is real: re-walk the diff yourself; confirm no drift hidden.
- AUDITED-MISSING-TESTS: confirm the added tests cover the gaps the auditor identified.
- AUDITED-DRIFT: confirm printed-text findings, applied existing-vocabulary YAML fixes and test evidence; unresolved ambiguity/capability gaps must be explicit.
- BLOCKED: same gap-kind discipline as implementer.
````

---

### 4E. Merge and Wire (orchestrator)

Steps in order:

1. **Copy files out of worker worktrees into the main tree.** For each worker that did not return BLOCKED:
   ```bash
   cp <worktree>/code/digimon-engine/cards/<set>/<CARD_ID>.yaml \
      code/digimon-engine/cards/<set>/<CARD_ID>.yaml
   cp <worktree>/code/digimon-engine/tests/cards_behavioral/card_text/<card_id_lower>.rs \
      code/digimon-engine/tests/cards_behavioral/card_text/<card_id_lower>.rs
   ```
   Reject worker output if it wrote files OUTSIDE the two expected paths. Re-dispatch once; on second drift, escalate.

2. **Apply review fixes** for any `NEEDS-FIX` card. Each directive specifies file:line; apply verbatim.

3. **Reconcile `code/digimon-engine/tests/cards_behavioral/card_text/mod.rs`.** The orchestrator pre-wired `mod <card_id_lower>;` lines in Phase 3a for every planned card in the run. After this batch's workers return:
   - For any card with verdict BLOCKED that wrote no test file: remove that card's `mod <card_id_lower>;` line and delete the empty `<card_id_lower>.rs` placeholder.
   - All other cards: leave the existing line in place — the worker filled in the file content.

4. **Reconcile `main.rs`.** Retain `mod card_text;` and existing card-text modules; never delete unrelated tests when a batch is blocked.

5. **Run targeted batch tests:**

   ```bash
   cargo test --manifest-path code/digimon-engine/Cargo.toml \
              --test cards_behavioral -- card_text
   ```

   On failure: dispatch ONE Sonnet "fix" agent with the failing output + the reviewer's directives (if any). Worktree-isolated. Allow it to modify only the same two file paths per card. Re-merge and re-run. If it still fails, escalate to the user with the failing output and exit. Do NOT loop.

6. **Update `qa/qa-reports/validated_cards_dsl.json`.** Append one entry per processed card per the schema below in "validated_cards_dsl.json Schema". Bump `last_updated` to today's date.

7. **Append batch summary to per-archetype QA artifact:**
   - For archetype runs: `qa/archetype-qa/dsl/<archetype_slug>.md`. Create from the template below in "Per-Archetype QA Artifact Template" on first batch; append batch row to the per-card table on subsequent batches.
   - For `--pool` runs: `qa/dsl-test-pool-progress.md`. Single accumulating file, last verdict per card wins.

8. **Append gap entries** to the right trackers:
   - `gap_kind: engine` → `qa/archetype-qa/engine-gaps.md`
   - `gap_kind: dsl` → `qa/dsl-vocab-gaps.md`
   - `gap_kind: hybrid` → both, with cross-references in each entry

9. **Print the batch summary table to the user:**

   ```
   Batch <N> complete (<n>/<total> cards)
   | Card ID   | Mode      | Verdict             | Review   | Tests | Notes |
   | <CARD>    | IMPLEMENT | IMPLEMENTED         | APPROVED | 7/7   | <one-line> |
   | <CARD>    | AUDIT     | AUDITED-OK          | APPROVED | 5/5   | |
   | <CARD>    | IMPLEMENT | BLOCKED (engine)    | APPROVED | 0/0   | <gap summary> |

   Running totals: IMPLEMENTED=<n> AUDITED-OK=<n> ... BLOCKED=<n>
   ```

10. **Auto-continue to next batch.** The user can interrupt between batches.

---

## Phase 5: Final Report

After all batches complete:

1. **Whole-archetype summary** — counts by verdict (IMPLEMENTED, PARTIAL, AUDITED-OK, AUDITED-MISSING-TESTS, AUDITED-DRIFT, BLOCKED-engine, BLOCKED-dsl, BLOCKED-hybrid, SKIPPED).

2. **Per-card results table** — `Card ID | Name | Mode | Verdict | Review | Tests | Notes`.

3. **Files created/modified, grouped by set.**

4. **Blocked cards split into two sections:** engine-gap blocked cards (with affected clauses + suggested API) and DSL-vocab-gap blocked cards (with affected clauses + suggested verb + the engine API it lowers to).

5. **Full-suite green check:**

   ```bash
   cargo test --manifest-path code/digimon-engine/Cargo.toml
   ```

   Must pass. If not, the skill has left the tree broken — escalate without auto-fixing. The per-batch fix round in 4E.5 is the only fix loop.

6. **Finalize per-archetype QA artifact** from the template below ("Per-Archetype QA Artifact Template"). Path is `qa/archetype-qa/dsl/<archetype_slug>.md` for archetype runs, `qa/dsl-test-pool-progress.md` for `--pool` runs.

---

## Phase 6: Idempotency

Re-running on the same archetype (or `--pool`) is safe: the `validated_cards_dsl.json` lookup in Phase 1c short-circuits cards with `IMPLEMENTED` or `AUDITED-OK` verdicts. Other verdicts (`PARTIAL`, `AUDITED-MISSING-TESTS`, `AUDITED-DRIFT`, `BLOCKED`) are re-attempted on the next run — the user is expected to address the underlying issue (engine gap closed, DSL vocab landed, drift triaged) before re-invocation.

---

## Skill Positive-Rules Appendix

This appendix is the "C" half of the hybrid checklist. The "A" half is `docs/RUST_DSL_TEST_API.md` §11 (the anti-pattern list). Both halves are embedded into the implementer, auditor, and reviewer prompts in Phase 4.

1. **Printed-text-first ordering is strict in IMPLEMENT and AUDIT.** Write and run bundle-derived card-text tests before YAML or old tests enter context; withhold implementation/scout bodies until then. Record the initial output (AUDIT may already pass), triage failures, then record final output.
2. **File header is mandatory** (test API §5): quote every printed section and official Q&A verbatim from `data/card_bundles/<ID>.md`, and state that source path. DCGO paths and pattern tags are supplemental only.
3. **One positive AND one negative test per condition.** Splitting is non-negotiable per test API §11.3. A single test asserting both directions is rejected.
4. **Every clause gets ≥1 integrated test** driven through `play` / `attack` / `end_turn`. Clause-isolated `EffectContext` tests (per §7) are *additional*, not substitutes.
5. **OPT clauses get an explicit lockout test** (test API §5 Section 5). Test that the second activation in the same turn is gated, and that the lockout clears after `end_turn`.
6. **Cost-firing clauses get an event-log test** (test API §5 Section 4). When an effect's cost has side effects (trash security, lose security, deletion), assert the corresponding `GameEvent` fires via `events_since(checkpoint)`.
7. **Use `dsl_card(id)`, never inline-paste production YAML** (test API §11.1). Inline fixtures are reserved for the cases enumerated in §10.
8. **Use `digimon_engine::action::space::*` constants, never hard-code action IDs** (test API §11.12).
9. **No approximations.** Every player choice surfaces through `pending_selection`. No `.iter().next()`, no `[0]`, no `min`/`max` over targets, no auto-resolutions of multi-option choices.
10. **No Python references.** Do not cite `code/engine_py_legacy/`, do not import Python script structure as ground truth. Ground truth is printed text + `docs/RULES_CONTEXT.md` / fandom wiki + DCGO C#.
11. **Engine-gap vs DSL-vocab-gap discipline.** Before declaring `BLOCKED`, confirm: does the engine *really* lack the primitive (read `docs/RUST_ENGINE_API.md`), or does only the DSL lack a verb that would lower to it (read the DSL spec)? Set `gap_kind` accordingly.
12. **No `place_on_field` shortcuts when testing OnPlay paths** (test API §11.11). `place_on_field` is for post-play state only.
13. **No `auto_resolve*`, wrappers or `enqueue_triggered` in `card_text/`.** Assert and answer every prompt with the explicit helpers (test API Card-text tests). The legacy baseline elsewhere can only shrink.

---

## `validated_cards_dsl.json` Schema

```json
{
  "version": 1,
  "last_updated": "YYYY-MM-DD",
  "cards": {
    "BT15-003": {
      "card_name": "Nyaromon",
      "validated_date": "YYYY-MM-DD",
      "report": "batch-implement-cards-rust-dsl",
      "status": "IMPLEMENTED",
      "gap_kind": null,
      "archetype": "Slice — Nokia/Greymon/Omnimon",
      "yaml_path": "code/digimon-engine/cards/bt15/BT15-003.yaml",
      "test_path": "code/digimon-engine/tests/cards_behavioral/card_text/bt15_003.rs",
      "test_count": 7,
      "patterns": ["G4", "E2", "F5"],
      "notes": "Inherited When Attacking + OPT + top/bottom branch"
    }
  }
}
```

| Field | Domain |
|---|---|
| `status` | `IMPLEMENTED` / `PARTIAL` / `AUDITED-OK` / `AUDITED-MISSING-TESTS` / `AUDITED-DRIFT` / `BLOCKED` |
| `gap_kind` | `null` / `"engine"` / `"dsl"` / `"hybrid"` (only set when `status == BLOCKED`) |
| `patterns` | Array of test-API §4.3 row tags from the scout brief |

The Python-side `qa/qa-reports/validated_cards.json` is **never modified**.

---

## Per-Archetype QA Artifact Template

Path: `qa/archetype-qa/dsl/<archetype_slug>.md` (or `qa/dsl-test-pool-progress.md` for `--pool` runs).

```markdown
# Archetype DSL Implementation: {Archetype Name}
Date: {YYYY-MM-DD}
Total cards in pool: {N}
Processed this run: {M}
Pipeline: batch-implement-cards-rust-dsl

## Summary
- IMPLEMENTED: {n}
- PARTIAL: {n}
- AUDITED-OK: {n}
- AUDITED-MISSING-TESTS: {n}
- AUDITED-DRIFT: {n}
- BLOCKED (engine): {n}
- BLOCKED (dsl): {n}
- BLOCKED (hybrid): {n}
- SKIPPED (prior verdict): {n}

## Per-Card Verdicts
| Card ID | Name | Mode | Verdict | Review | Tests | Notes |
|---------|------|------|---------|--------|-------|-------|

## Engine-Gap Blocked Cards
### {CARD_ID} {card_name}
- Effect text: "..."
- Missing engine API: ...
- Suggested addition: ...

## DSL-Vocab-Gap Blocked Cards
### {CARD_ID} {card_name}
- Effect text: "..."
- Missing DSL verb: ...
- Lowers to engine API: ...
- Suggested DSL syntax: ...

## New Patterns Discovered
- {pattern}: {description} — propose adding to RUST_DSL_TEST_API.md
```

---

## Invariants (orchestrator enforces)

- Workers never edit `main.rs`, any `mod.rs`, `cards.rs`, `validated_cards_dsl.json`, or any tracker file. Orchestrator owns all shared state. Worker output that touches forbidden paths is rejected and re-dispatched once; second drift escalates to user.
- Card-ID conventions: YAML files use original-case dashed IDs (`BT15-003.yaml`); Rust test files use lowercase-underscore (`bt15_003.rs`). Pack registry key is the original-case ID (`"BT15-003"`).
- The Python `qa/qa-reports/validated_cards.json` is never touched.
- No Notion calls.
- No Pinecone calls.

---

## Known Limitations (v1)

- **Bounded AUDIT fixes.** Fix clear printed-text mismatches with existing vocabulary only; escalate ambiguous semantics or missing DSL/engine capabilities.
- **No Notion sync.**
- **No Pinecone retrieval.** Agents use scout brief + read-on-demand only.
- **Single fix round per batch.** Orchestrator does one re-dispatch on cargo failure, then escalates.
- **Workers cannot add a new test binary or new set's `mod.rs`.** Orchestrator-only territory.
- **`--report-only` is plan-only.** Does not run scouts to pre-classify gaps.
- **Sonnet implementer is default; Opus is the escape hatch via `--implementer-model opus`.** No automatic escalation on review-failure loops in v1; user re-dispatches by hand.
