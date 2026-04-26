---
name: assess-archetype-rust
description: Archetype-scoped pre-flight audit of the Rust engine's ability to implement a Digimon TCG archetype. Dispatches parallel Opus sub-agents to classify each card's required primitives against the current `EffectContext` / `Effect` / `Keyword` / `ModifierType` surface. Emits a deduplicated gap log in `docs/RUST_ENGINE_GAPS.md` and a standalone fix-plan prompt under `.claude/plans/`. Writes no engine or card-script code. Use before running `/batch-implement-cards-rust` on a new archetype, or when you want to know which engine primitives an archetype will require before committing to a port.
argument-hint: <ARCHETYPE_NAME> [--cards CARD1,CARD2,...] [--batch-size N]
---

# Assess Archetype Rust Coverage — Pre-flight Engine Gap Audit

You are auditing whether archetype **$ARGUMENTS** can be implemented today in the Rust engine (`code/digimon-engine/`) without stubs, approximations, or auto-selections (per CLAUDE.md §17). **You do not write engine code and you do not write card scripts.** Your deliverables are:

1. Updates to `docs/RUST_ENGINE_GAPS.md` — deduplicated, capability-centric entries for each missing primitive.
2. A one-line link added to `docs/INDEX.md` if `RUST_ENGINE_GAPS.md` did not already exist.
3. A standalone fix-plan prompt written to `.claude/plans/rust-engine-gaps-{slug}.md` — the user feeds this to `superpowers:writing-plans` (or a fresh session) to design the engine additions that would unblock the archetype.

Anything else — engine code, card scripts, tests — is out of scope. `/batch-implement-cards-rust` is the skill that consumes the gaps this skill produces.

## When to Use

- Before running `/batch-implement-cards-rust` on an archetype you have not ported yet.
- When triaging which archetype to port next (smaller gap footprint = cheaper port).
- When deciding whether to invest in a Rust-engine feature (this tells you which cards it unblocks).

**Not for:** Fixing existing Rust scripts (use `/batch-implement-cards-rust`). Not for Python-side gaps (use `/implement-archetype` / `/review-archetype`; they maintain `qa/archetype-qa/engine-gaps.md`).

## Allowed Writes

The ONLY files you may create or modify:

- `docs/RUST_ENGINE_GAPS.md` (create on first run, append thereafter)
- `docs/INDEX.md` (add exactly one link line if missing)
- `.claude/plans/rust-engine-gaps-{slug}.md`

`git diff code/digimon-engine/` must be empty at the end of the run. `git diff code/digimon_gym/` must be empty. If you think you need to touch anything else, stop and ask the user.

## Quick Reference — inputs the orchestrator reads

| Purpose | Path |
|---|---|
| Archetype → card IDs (with `deck_frequency`, `meta_share`, `best_decklist`) | `code/tools/resolve_deck.py :: resolve_archetype()` |
| Deck library (consumed by `resolve_deck`) | `digimon_gym/engine/data/deck_library.json` |
| Deck library ingestion (DigimonMeta, Egman, DigimonCard.io, DigiLab) | `code/tools/meta_loader.py` — run `--build` to refresh before large audits |
| Card text/metadata | `digimon_gym/engine/data/cards.json` |
| Rust scripting API | `docs/RUST_ENGINE_API.md` (timings, EffectContext, Effect builder, Keyword, ModifierType) |
| Cross-engine divergences | `docs/RUST_PYTHON_PARITY.md` |
| Currently registered Rust cards | `code/digimon-engine/src/cards.rs` + `code/digimon-engine/src/cards/` |
| C# reference (authoritative behavior) | `DCGO/Assets/Scripts/CardEffect/{SET}/{COLOR}/{CARD_ID}.cs` |
| Existing gap log (to dedupe against) | `docs/RUST_ENGINE_GAPS.md` |
| Format reference (do not modify) | `qa/archetype-qa/engine-gaps.md` |

---

## Phase 1 — Resolve Card Pool

Resolve the card pool via the project's meta-ingestion pipeline. `deck_library.json` is built by `code/tools/meta_loader.py` from real tournament decklists (DigimonMeta, Egman, DigimonCard.io, DigiLab) and is the source of the `deck_frequency` / `meta_share` / `best_decklist` signals this skill relies on to prioritize audit batches.

**Before a large audit**, check `deck_library.json`'s freshness (`git log -1 digimon_gym/engine/data/deck_library.json`). If stale, ask the user whether to refresh:

```bash
python code/tools/meta_loader.py --scrape-digimonmeta <URL>   # or --scrape-egman / --scrape-digimoncard-io / --scrape-digilab
python code/tools/meta_loader.py --build
```

Then resolve the archetype the same way `/implement-archetype` does:

```python
import sys; sys.path.insert(0, '.')
from tools.resolve_deck import resolve_archetype
# --cards BT17-001,BT17-002 → cards_override=[...]
manifest = resolve_archetype('ARCHETYPE_NAME')
```

The returned `ArchetypeManifest` gives you `unique_cards` (each with `deck_frequency` sourced from `deck_library.json`), `meta_share`, `total_decklists`, and `best_decklist` — use `deck_frequency` to rank the audit pool by real-world blast radius, not just card-ID order.

Then:

1. Detect **already implemented in Rust**. Grep `code/digimon-engine/src/cards.rs` and every `code/digimon-engine/src/cards/*.rs` for each card ID (string literal and case-variant module name). A card counts as implemented iff it is inserted into `CardEffectRegistry` via a `register(...)` function — merely having a `card_data` entry does not count.
2. Produce the **audit pool** = `manifest.unique_cards` minus implemented cards. These are the only cards you analyze deeply.
3. Sort the audit pool by `deck_frequency` descending (tie-break on effect-text length descending, so complex cards are front-loaded), then chunk into batches of size `--batch-size` (default 5). High-frequency cards dominate archetype feasibility — if the most-played card in the archetype is blocked, the archetype is blocked.

Present the user a short plan table before dispatching:

```
Archetype: {name}  (meta_share: {pct}%, {total_decklists} decklists)
Unique cards: N
  Already in Rust registry: K (skipped)
  To audit: N-K
Top-frequency cards to audit: {first 5 by deck_frequency}
Batches: B (size {batch-size})
```

Pause for approval. If the user asks to narrow scope, re-compute batches.

---

## Phase 2 — Build the Tech-Lead Context Pack

Before dispatching sub-agents, the orchestrator (you) assembles one reusable context pack. Read these into the conversation so every sub-agent prompt can embed them verbatim:

- `docs/RUST_ENGINE_API.md` — sections: `EffectTiming` enum, `EffectContext` API, `Effect` builder, `Keyword` enum, `ModifierType` enum.
- `docs/RUST_PYTHON_PARITY.md` — full list of 🔴 and 🟡 entries. Sub-agents must distinguish a parity-driven gap (already tracked there) from a new capability gap.
- `docs/RUST_ENGINE_GAPS.md` if it exists — sub-agents must **cite existing gap titles** rather than re-file duplicates.

Sub-agents may additionally query Pinecone `engine-api` / `card-metadata` namespaces (index `digimon-engine`) for cross-card references or finer-grained API usage examples. Pinecone is supplementary, not primary.

---

## Phase 3 — Dispatch Parallel Opus Sub-agents

One Opus sub-agent per batch, dispatched in parallel via Agent tool calls in a single message. Each sub-agent's prompt must include the full context pack from Phase 2 and the per-card payload: card ID, card text (effect / inherited / security), C# reference path, deck frequency.

**Sub-agent mandate** (copy into each prompt verbatim):

> For each card in your batch:
>
> 1. Decompose the card's effect / inherited / security text into numbered clauses (the same decomposition pattern as `docs/RUST_ENGINE_API.md` §20 TDD walkthrough).
> 2. For each clause, enumerate the Rust primitives needed: timing variant, selection helper, modifier type, keyword, zone-manipulation helper, condition shape.
> 3. Cross-check each primitive against `docs/RUST_ENGINE_API.md`. If partially supported, read the C# reference to decide whether the partial support preserves faithful behavior.
> 4. Classify each clause:
>    - **🟢 SUPPORTED** — primitive present, clause is faithfully implementable today.
>    - **🟡 PARTIAL** — workaround exists but degrades fidelity or hides a choice from the RL action space.
>    - **🔴 BLOCKING** — no viable workaround, card cannot be faithfully implemented today.
> 5. Before filing a new gap, search `docs/RUST_ENGINE_GAPS.md` (provided in context) for an existing gap title that matches. If found, add this card to that gap instead of filing a duplicate.
> 6. Emit two artifacts:
>    - A per-card verdict line: `{card_id}: SUPPORTED | PARTIAL(gap_ids=[...]) | BLOCKED(gap_ids=[...])` with a one-sentence rationale.
>    - A gap entry for each NEW distinct gap, using the template below.
>
> Gap entry template:
> ```markdown
> ### <Gap Title> — capability-centric, not card-centric
> - **Severity:** 🔴 BLOCKING | 🟡 PARTIAL
> - **Discovered in:** {archetype} ({YYYY-MM-DD})
> - **Card(s):** {CARD_ID} — {card_name}, ...
> - **Effect text:** "{relevant clause, verbatim}"
> - **What's missing:** {Rust primitive / timing / helper needed; cite RUST_ENGINE_API.md § if partially present}
> - **Suggested API shape:** {one-line sketch, e.g. `ctx.activate_foreign_when_digivolving_effect(handle)`}
> - **Workaround:** "None — BLOCKED" | "{brief description of partial workaround}"
> - **Related:** {RUST_PYTHON_PARITY.md §X.Y, or other gap titles}
> ```
>
> Do NOT write any code. Do NOT modify files. Return only the two artifacts (per-card verdicts + gap entries) as your final message.

Dispatch all batches in parallel (single message, multiple Agent tool calls). Collect results.

---

## Phase 4 — Consolidate & Document

Back in the main session:

1. **Deduplicate gaps across batches.** Two sub-agents analyzing different cards may file the same gap under slightly different titles. Merge by semantic match, not string match. Union the `Card(s)` and `Discovered in:` lines.
2. **Merge against existing `docs/RUST_ENGINE_GAPS.md`.** For each consolidated gap:
   - If a matching entry already exists, append the new cards to its `Card(s)` line and append today's `{archetype} ({YYYY-MM-DD})` to its `Discovered in:` line only if that archetype+date pair is not already present (idempotency).
   - Otherwise, insert a new section under `## Open gaps`.
3. **Create `docs/RUST_ENGINE_GAPS.md` on first run.** Structure:
   ```markdown
   # Rust Engine Gaps

   Capability gaps in the Rust engine's scripting surface (`code/digimon-engine/`), discovered during archetype audits by `/assess-archetype-rust`. Distinct from `RUST_PYTHON_PARITY.md`, which tracks Rust↔Python divergences in shared subsystems.

   Format and conventions mirror `qa/archetype-qa/engine-gaps.md` (Python-scoped).

   ## Open gaps

   <entries appended here>

   ## Resolved gaps

   <move here when the gap is closed; keep the entry for historical context>
   ```
4. **Add one link to `docs/INDEX.md`** only if this is the first run. Place it in the Rust-engine docs group, immediately after the `RUST_PYTHON_PARITY.md` link. Use the same formatting as surrounding entries.

Do not edit `qa/archetype-qa/engine-gaps.md` — that is Python-scoped.

---

## Phase 5 — Emit the Fix-Plan Prompt

Write `.claude/plans/rust-engine-gaps-{slug}.md` where `{slug}` is the archetype name lowercased with spaces→dashes. Overwrite on re-run. The file must be **a standalone prompt** — readable by an agent that has no memory of this session.

Template:

```markdown
# Rust engine gap-closure plan — {archetype}

## Goal

Close the {N_blocking} blocking and {N_partial} partial engine gaps below so that {archetype} can be fully implemented in `code/digimon-engine/` without stubs, approximations, or auto-selections (per CLAUDE.md §17–18).

This prompt was generated by `/assess-archetype-rust` on {YYYY-MM-DD}. Re-run that skill if card text, deck list, or engine API has changed.

## Gaps to close (ordered by blast radius)

1. **{Gap Title}** (🔴, blocks {M} cards) — {one-line summary}. See `docs/RUST_ENGINE_GAPS.md` section "{Gap Title}".
2. **{Gap Title}** (🟡, affects {M} cards) — {one-line summary}. See `docs/RUST_ENGINE_GAPS.md` section "{Gap Title}".
3. ...

## Cards affected (verbatim list, for regression targeting)

- {CARD_ID} — {card_name}: gaps [{gap_titles}]
- ...

## Relevant references

- `docs/RUST_ENGINE_API.md` — existing scripting surface
- `docs/RUST_ENGINE_GAPS.md` — full gap entries with suggested API shapes
- `docs/RUST_PYTHON_PARITY.md` — cross-engine divergences (some gaps may be parity-driven)
- `code/digimon-engine/src/effect_context.rs` — where most new helpers land
- `code/digimon-engine/src/effect.rs` + `code/digimon-engine/src/effect_queue.rs` — timings + triggered-effect plumbing
- `code/digimon-engine/src/modifiers.rs` — modifier registry (new ModifierType variants land here)
- `code/digimon-engine/tests/test_cards_behavioral.rs` — TDD harness (new tests land alongside; CLAUDE.md §18)

## Ask

Produce a phased implementation plan (per `superpowers:writing-plans`) that:

1. Groups gaps by subsystem: new timings, new selection helpers, new modifier types, new keywords, triggered-effect plumbing, combat/interrupt phases, etc.
2. Orders phases by dependency (shared plumbing first) and blast radius (high-card-count gaps first within each dependency tier).
3. For each phase, lists:
   - API surface to add (exact function signatures / enum variants).
   - Tests to write FIRST (TDD per CLAUDE.md §18) — name the `code/digimon-engine/tests/*.rs` file and test case.
   - Affected Rust files.
   - Parity implications: does this resolve or create a `docs/RUST_PYTHON_PARITY.md` entry?
4. Flags any gap that is architectural (e.g., native keyword parsing on CardData, interrupt-phase machinery) and should have its own spec under `docs/superpowers/specs/` before implementation.
5. Does NOT attempt to close every gap in one phase. Prefer 3–6 phases with clear stop points.
```

Substitute the counts, gap titles, and card lists from Phase 4.

---

## Final User-Facing Message

Print exactly:

```
Audited {archetype}: {N_audited} cards.
  🟢 SUPPORTED: {n_supported}
  🟡 PARTIAL:   {n_partial}    ({g_partial} distinct gaps)
  🔴 BLOCKING:  {n_blocking}   ({g_blocking} distinct gaps)

Gaps appended to: docs/RUST_ENGINE_GAPS.md
Fix-plan prompt:  .claude/plans/rust-engine-gaps-{slug}.md

Next step: feed the fix-plan prompt to /superpowers:writing-plans (or a fresh session) to design the engine additions.
```

---

## Red Flags — STOP and Reset

- You are about to edit a file under `code/digimon-engine/` → STOP. This skill writes no engine code.
- You are about to edit a file under `code/engine_py_legacy/engine/data/scripts/` → STOP. This skill writes no card scripts.
- You are about to touch `qa/archetype-qa/engine-gaps.md` → STOP. That file is Python-scoped.
- You are about to file a new gap with a card-centric title (e.g. "BT17-042 cannot be implemented") → STOP. Gap titles are capability-centric.
- You are about to skip Phase 4's dedup against existing `docs/RUST_ENGINE_GAPS.md` → STOP. Duplicate entries destroy the doc's value across runs.
- Sub-agent returned a PARTIAL verdict without naming the workaround → send it back; the workaround is the entire point of the PARTIAL classification.
- Sub-agent returned a BLOCKING verdict without reading the C# reference → send it back; C# is the behavioral source of truth (CLAUDE.md Project Vision).

## Verification before finishing

1. `git status` — changes ONLY in `docs/RUST_ENGINE_GAPS.md`, optionally `docs/INDEX.md`, and `.claude/plans/rust-engine-gaps-{slug}.md`. No other files.
2. `docs/RUST_ENGINE_GAPS.md` parses cleanly (valid markdown, every entry has all template fields).
3. The fix-plan prompt is self-contained — an agent with no memory of this session could act on it using only the referenced files.
4. Re-running the same archetype does not produce duplicate gap sections or double-appended `Discovered in:` dates.
