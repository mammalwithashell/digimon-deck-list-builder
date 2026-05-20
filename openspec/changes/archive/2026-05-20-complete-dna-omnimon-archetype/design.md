## Context

DNA Omnimon is the lead pilot archetype for the Rust DSL card pipeline. Phase 2 Track F closed 6 of its 7 targeted DSL/substrate gaps, and most of the 11-item reusable gap backlog in `qa/archetype-qa/dsl/2026-05-03-dna-omnimon-dsl-engine-gaps.md` is implemented. A direct audit of `main` (2026-05-19) found:

- 64 unique cards across 66 decklists; **59/64 have YAML, 63/64 have behavioral tests**.
- **5 cards have no YAML**: BT22-084 Nokia Shiramine (63/66 decks), BT17-007 (9), ST2-13 (4), BT5-093 (2), AD1-019 (1, empty placeholder).
- **42 cards carry `#[ignore]`'d tests** (~112 `#[ignore = "pending: G-XYZ"]` markers).
- **8 cards carry 18 `raw_rust` escapes**.

The dominant problem is **drift between three sources of truth**:

```
  qa/dsl-vocab-gaps.md     →  "G-OPT-TRIGGERED ... RESOLVED (Track C)"
  engine code              →  the primitive EXISTS on main
  test files               →  #[ignore = "pending: G-OPT-TRIGGERED"]   ← never re-enabled
```

Substrate landed but tests were not re-enabled and card clauses were not authored. The per-card verdict ledger `validated_cards_dsl.json` that every Phase 2 plan cites **does not exist on `main`** — so the true `PARTIAL`/`BLOCKED` count is unknown. The `rust-engine-gaps-dna-omnimon.md` plan (62 gaps, 2026-04-17) is stale and superseded.

Constraints: no-approximations policy (CLAUDE.md §17), TDD for new effects (§18), no `ACTION_SPACE_SIZE`/tensor contract churn (Working Rule 1), cards migrate one direction Python→Rust (§21). Source priority for card behavior: printed text → `RULES_CONTEXT.md` → fandom wiki → DCGO (now initialized as a submodule).

## Goals / Non-Goals

**Goals:**

- Produce a verified, evidence-based per-card status for all 64 DNA Omnimon cards.
- Re-enable every test ignored for an already-closed gap; author the card clause it covers.
- Author production YAML + tests for the 5 missing cards.
- Close `G-DSL-INHERITED-SUBSTITUTE-RETURN-TRASH` (unblocks EX5-015 Clause C).
- Minimize and document `raw_rust` escapes in the archetype.
- Leave `qa/dsl-vocab-gaps.md`, `qa/resolved-gaps.md`, and the per-archetype gap doc consistent with reality.

**Non-Goals:**

- Closing every entry in `docs/RUST_ENGINE_GAPS.md` — only DNA-Omnimon-touching gaps are in scope.
- Cards not used by any DNA Omnimon decklist.
- Engine substrate gaps the reconciliation sweep proves *out of reach* for a single change — those are filed as scoped follow-ups, not absorbed.
- `ACTION_SPACE_SIZE` / observation-tensor changes, PyO3 binding changes, Python legacy engine changes.

## Decisions

### Decision 1: Phase A (reconciliation sweep) runs first and gates C/D

The sweep is the de-risking step. Until each `#[ignore]` is classified against *current code*, the scope of Phase C (residual substrate) and Phase D (`raw_rust`) is unknowable. Phase A is cheap — read test, grep `code/digimon-engine/src/` and `code/digimon-dsl/src/` for the cited primitive, classify.

**Alternative considered:** trust the trackers and skip the sweep. Rejected — the trackers are the proven-unreliable source; `qa/dsl-vocab-gaps.md` already records tags as RESOLVED that test files still mark `pending`.

### Decision 2: Verification is by code inspection, never by tracker

Each `#[ignore]` is classified by grepping the engine/DSL crates for the cited primitive (struct/verb/predicate/formula), not by reading a tracker that says "RESOLVED". A tracker claim is treated as a hypothesis to confirm against code. This is the rule that broke the drift in the first place.

Classification buckets per ignored test:
- **STALE** — cited gap's primitive exists in code → re-enable, author clause, verify pass.
- **OPEN-SUBSTRATE** — primitive genuinely absent → keep ignored, route to Phase C, fix the `#[ignore]` reason to be accurate.
- **AUTHORING-ONLY** — substrate present, test was a placeholder with no card body → author body, re-enable.

### Decision 3: The verdict ledger is the sweep's primary artifact

Phase A emits `qa/qa-reports/validated_cards_dsl.json` with one entry per DNA Omnimon card: `card_id`, `archetype`, `verdict` (`IMPLEMENTED`/`PARTIAL`/`BLOCKED`), `notes` naming any open gap. This replaces tracker guesswork and is the file every downstream plan already expects. Schema follows the `batch-implement-cards-rust-dsl` skill's existing format.

**Alternative considered:** skip the ledger, rely on test pass/fail. Rejected — a card can have all-green core tests yet omit a printed clause (`PARTIAL`); the ledger records faithfulness, not just compilation.

### Decision 4: Card authoring is TDD and ordered by deck frequency

For each card needing work, write `DebugRunner` behavioral tests from printed text *first*, then YAML, per CLAUDE.md §18. Order by deck presence so the highest-impact card lands first: **BT22-084 (63 decks) is the single most important deliverable** in the missing-YAML set.

### Decision 5: `G-DSL-INHERITED-SUBSTITUTE-RETURN-TRASH` lowers onto the existing replacement framework

The replacement-effect framework (PR #449) and Track B's `activation_cost` builder already exist. The new clause is a DSL surface that lowers onto them — scoped to an inherited-source card's would-be trash. The two entangled sub-gaps from the Track F deferral note (`G-SELECT-MULTI-MIN`, `G-ZONE-TRASH-TO-DECK`) are handled as part of this work: a min-bounded multi-pick selection and a trash→deck zone mover. EX5-015 Clause C is the regression fixture.

**Alternative considered:** a `raw_rust` escape for EX5-015 Clause C. Rejected — it is a 1-card, low-frequency card, but the clause is a reusable replacement primitive and the project direction is DSL-first; a documented DSL verb is the correct closure.

### Decision 6: Phase D (`raw_rust` migration) is review-then-migrate, not migrate-everything

Each of the 18 escapes is reviewed against current DSL vocabulary. Escapes now expressible are migrated; escapes that are not are documented with a reason. `raw_rust` is a tolerated escape hatch — the goal is "minimized and justified", not "zero".

## Risks / Trade-offs

- **[Phase A surfaces a large residual substrate backlog]** → Phase A is explicitly the scoping step. If it proves more than a handful of genuine substrate gaps, the change is re-scoped: file the large gaps as separate follow-ups rather than absorbing them. The proposal already declares this a non-goal.
- **[Cross-track-blocked tests masquerade as DNA Omnimon gaps]** → Some ignores may be blocked by unmerged or partial sibling-track work (e.g. inherited dispatch edge cases). The sweep tags these distinctly and leaves them ignored with an accurate reason; they are not counted against this change.
- **[`raw_rust` migration introduces behavioral regressions]** → Every migration keeps the card's existing behavioral test as the guard; a migration that changes test outcomes is reverted, not forced.
- **[Misreading printed card text]** → Follow source priority strictly: printed `data/cards.json` text → `RULES_CONTEXT.md` → fandom wiki → DCGO as tiebreaker only. DCGO is now initialized for the heavyweight top-ends (BT17-078, EX9-021, BT22-015).
- **[Trackers drift again after this change]** → The change's final step reconciles all three trackers; the verdict ledger becomes the durable source of truth, reducing future reliance on prose trackers.

## Migration Plan

This is engine/card-data work — no runtime deployment or rollback. Sequencing:

1. **Phase A** — reconciliation sweep; emit `validated_cards_dsl.json`; fix `#[ignore]` reasons; re-enable STALE tests (clauses authored inline where small).
2. **Phase B** — author the 5 missing cards, TDD, frequency-ordered (BT22-084 first). Runs in parallel with A.
3. **Phase C** — close `G-DSL-INHERITED-SUBSTITUTE-RETURN-TRASH` and any genuine substrate gap the sweep proved real; author the unblocked clauses. Depends on A.
4. **Phase D** — review and migrate `raw_rust` escapes. Depends on A and C.
5. **Tracker reconciliation** — move closed gaps to `qa/resolved-gaps.md`; annotate the per-archetype gap doc; finalize the ledger.

Each phase ends at a verifiable stop point: `cargo test --manifest-path code/digimon-engine/Cargo.toml` for the `cards_behavioral`, `dsl`, `dna_digivolve`, `digivolve`, and `dsl_eval_arm_coverage` suites must pass with no regressions.

## Phase A outcome (2026-05-19) — scope resolved

The reconciliation sweep classified all 87 ignored tests (41 gap items) against current
engine code. Result: **8 STALE, 14 AUTHORING-ONLY, 1 CROSS-TRACK, 18 OPEN-SUBSTRATE.**
Verdict ledger: 34 IMPLEMENTED / 25 PARTIAL / 5 BLOCKED. The maintainer elected **full
scope** — close all 18 substrate gaps in this change rather than filing the larger ones
as follow-ups. `tasks.md` sections 3–6 are the expanded Phase C: 6 small substrate gaps,
9 medium, 3 deep, plus the 14 authoring-only clauses. Per-item evidence with file:line
citations is in `.workdata/classification.json`.

## Open Questions

- **Exact Phase C substrate scope** — RESOLVED by the Phase A sweep above: 18 substrate gaps, enumerated in `tasks.md` §3–5.
- **Ledger location** — RESOLVED. `qa/qa-reports/validated_cards_dsl.json` already existed on `main` as a 304-card multi-archetype dict (`{version, last_updated, cards}`); the early-exploration claim that it was absent was wrong (a transient working-directory drift hid it). Phase E *merges* the 64 DNA Omnimon verdicts into that file's `cards` map, preserving all 241 other-archetype entries — it does not replace the file.
- **ST2-13 Hammer Spark** — RESOLVED. Its `[Main]`/`[Security]` clauses are pure `gain_memory` steps; the Option `[Main]` play-flow residual scopes only place-in-battle-area / Plug-In sub-cases, so ST2-13 landed fully `IMPLEMENTED`, not `PARTIAL`.
