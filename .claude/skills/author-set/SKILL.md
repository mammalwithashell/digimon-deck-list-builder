---
name: author-set
description: Author an entire Digimon TCG release set (a booster like BT17 or an EX set) as Rust DSL cards. Resolves the set, refreshes it from digimoncard.io, runs the DCGO-oracle keyword gate to flag/ingest new keywords BEFORE implementation, clusters the ~100 cards into archetype slices, then dispatches the author-set Workflow to mass-implement and combo-test each slice. Use when asked to implement/author a whole set or booster (not a single archetype — for that use /implement-rust-dsl-archetype).
argument-hint: <SET_PREFIX> (e.g. BT17, EX12, ST3) [--no-pull]
---

# Author Set — release-set authoring orchestrator

Author a complete release set as tested Rust DSL cards. A booster is a **union of
archetype slices plus orphan staples**; this skill is the set-scoped orchestrator
that reuses the per-archetype skills as stages. It owns the human-in-loop Phases
1–3 (ingest, keyword barrier, clustering + approval) and then launches the
`author-set` **Workflow** for the heavy Phases 4–6 fan-out.

**REQUIRED SUB-SKILLS:** the Workflow stages invoke `/batch-implement-cards-rust-dsl`
(authoring) and `/archetype-interaction-test-author` (combo testing). Do NOT
reimplement either here.

## When to use

- "Author/implement all of BT17", "do the whole EX12 set", "implement <booster>".
- **Not for** a single archetype/deck (use `/implement-rust-dsl-archetype` or
  `/batch-implement-cards-rust-dsl`), and not for gameplay QA (use `/gameplay-qa`).

## Tooling (deterministic, Phases 1–3)

All under `code/tools/author_set/` (run with `PYTHONPATH=code`):

| Concern | Module / command |
|---|---|
| Dry-run preview (resolve + diff + keyword gate + cluster) | `python -m tools.author_set.report_set <SET> [--pull]` |
| Set resolver | `tools.author_set.set_resolver.resolve_set` |
| Ingest diff / merge | `tools.author_set.ingest_diff` |
| DCGO keyword manifest | `data/dcgo_keyword_manifest.json` (regen: `python -m tools.author_set.dcgo_manifest`) |
| Lexicons | `data/author_set_lexicons.json` (regen: `python -m tools.author_set.lexicons`) |
| Keyword gate | `tools.author_set.keyword_gate` |
| Clusterer | `tools.author_set.clusterer` |
| Flag router | `tools.author_set.gap_router` |

## Procedure

### Phase 1 — Ingest-diff
Run `python -m tools.author_set.report_set <SET> --pull` (omit `--pull` only with
`--no-pull`). Show the user the diff. If cards were added/changed, the merge step
has refreshed `data/cards.json`; never author against a stale snapshot. If
`digimoncard.io` is unreachable, the tool warns and falls back to local — relay
that warning.

### Phase 2 — Keyword gate (BARRIER)
Read the report's keyword section:
- **covered** — already a Rust `Keyword` variant. Nothing to do.
- **auto_ingest (simple)** — DCGO models it as a passive/static flag the Rust enum
  lacks. Port it from the DCGO C# reference into a `Keyword` variant + DSL lowering +
  `keyword_effects.rs` wiring + a green DebugRunner test (TDD against DCGO). **Hard
  barrier**: it MUST land before Phase 4. If a keyword exposes a new player choice,
  trigger the rule-27 `ActionSpace.cs` codegen. If a candidate is actually
  card-specific behavior, reclassify it as a normal card clause (do not add a `Keyword`).
- **auto_ingest_subsystem** — DCGO has a `KeyWordEffects/` file BUT models the keyword
  as an `ActivateClass` that selects a permanent / mutates board state (e.g. `Link`).
  "DCGO has a file" is necessary but NOT sufficient: these are **subsystems** (new
  action-space entry + attachment state + selection + timing + rule-check), comparable
  to DigiXros — NOT cheap ports. Do **not** naively auto-ingest. Assess the keyword
  (read its DCGO C# + the rules), log it to `docs/RUST_ENGINE_GAPS.md`, and treat it as
  scheduled engine work. The set is BLOCKED for cards depending on it until it lands.
  Worked example + full primitive breakdown: the `[Link]` entry in `RUST_ENGINE_GAPS.md`.
- **flag_for_human** — in neither the Rust enum nor the DCGO manifest. **HALT.** For
  each, call `tools.author_set.gap_router.route_flagged_keyword` (records the gap in
  `docs/RUST_ENGINE_GAPS.md` + a `.claude/plans/` stub) and `cards_using_keyword` to
  list the affected cards. Ask the user for the keyword's behavior / a reference to
  port from. Do NOT proceed to author cards that depend on a flagged keyword.

### Phase 3 — Cluster + approval
The report prints the slice partition (slices + orphan-staples bucket). Present it
and get explicit user approval (AskUserQuestion) before dispatching anything.
Adjust slice membership/names if the user requests. Orphan staples are authored but
combo-tested case-by-case (ask the user which, if any, warrant interaction tests).

### Phases 4–6 — Launch the Workflow
Once no `flag_for_human` keyword remains, every needed keyword is covered/ingested,
and the partition is approved, launch the Workflow (this is a large, opt-in,
many-agent run — confirm with the user first):

```
Workflow({ name: "author-set", args: { set_prefix: "<SET>",
  slices: [ { name, card_ids: [...Lv2->Lv7 order...], is_orphan }, ... ] } })
```

It pipelines per slice: Phase 4 implements via `/batch-implement-cards-rust-dsl`,
Phase 5 combo-tests non-orphan slices via `/archetype-interaction-test-author`
(lazy cross-set pull), Phase 6 runs the set coverage gate. Relay its final report;
record the set-level verdict.

## Invariants

- No-approximations applies throughout — every choice via `pending_selection`.
- The keyword barrier is non-negotiable: never mass-implement cards on top of a
  missing keyword primitive.
- Cross-set authoring deps are ~zero; combo-test deps are lazy (synthesized
  fixtures for evo prereqs; single-card pull only on behavioral need).
- Reuse the existing skills as stages; this skill never reimplements card authoring
  or interaction testing.

## Design

`openspec/changes/add-author-set-workflow/` (proposal, design w/ DCGO fidelity audit,
spec). Tooling reference: `code/tools/author_set/README.md`.
