## Context

BG Imperial is a 24-card archetype; all cards have YAML under `code/digimon-engine/cards/`. Current `validated_cards_dsl.json` verdicts: 9 IMPLEMENTED, 13 PARTIAL, 2 BLOCKED (BT12-028, ST9-06).

The gap-tracker inputs predate three substrate waves (Phase 2 Tracks A–J / PR #480; DNA Omnimon completion; Puppets sweep). Source verification during the prior `/opsx:explore` pass confirmed the following primitives — listed as "open" in the BG trackers — already exist:

- `binding_present` / `binding_is_none` alias — `code/digimon-dsl/src/predicate.rs:277`
- `self_color_count_gte` — `predicate.rs:110`
- `event_card_color_only` / `event_card_color_count` — `predicate.rs:246`
- `source_dp` formula — `code/digimon-dsl/src/formula.rs:33`
- `event_target_owner` — `predicate.rs:214`
- `dp_lte` / `play_cost_lte` filter constraints — `predicate.rs:71,81`
- `play_union_bound_free` step (PUPPETS-G014) — `code/digimon-dsl/src/step.rs:322`
- `schedule_delayed` step — `step.rs:412`
- `grant_keyword` `active_when` now consumed — `code/digimon-engine/src/dsl_cards/lower_grant_keyword.rs:18-36`
- `may_attack_now` — `code/digimon-engine/src/effect_context/mod.rs:4865`

Verified still-missing: `stack_size_lte_source`, `select_opponent_sources`, `carrier_has_keyword`, `is_carrier_of_source`, `self_digivolution_contains_trait` (no source hits). `G-COST-REDUCE-ALLY-DIGIVOLVE` is explicitly DEFERRED by Phase 2 Track H.

Constraint: CLAUDE.md §17–18 (no-approximations; TDD via `DebugRunner`), and the standing acceptance criteria in the BG cross-archetype doc — no `ACTION_SPACE_SIZE`/tensor changes; every player-visible decision through masks or `PendingSelection`; trackers must distinguish engine primitives (`docs/RUST_ENGINE_GAPS.md`) from DSL vocab (`qa/dsl-vocab-gaps.md`).

## Goals / Non-Goals

**Goals:**
- Replace the stale BG Imperial gap picture with a verified, card-by-card classification (Phase 0).
- Close the verified-missing substrate primitives that block BG Imperial card text.
- Drive BG Imperial verdicts from 9/13/2 toward maximum IMPLEMENTED, with behavioral test proof per card.
- Leave `docs/RUST_ENGINE_GAPS.md`, `qa/dsl-vocab-gaps.md`, `qa/resolved-gaps.md`, and `validated_cards_dsl.json` accurate.

**Non-Goals:**
- ~~`G-COST-REDUCE-ALLY-DIGIVOLVE` (BT3-103)~~ — **scope extended 2026-05-21.** This was the original non-goal (BT3-103 is not in either assessed BG Imperial meta list). After the in-scope substrate landed, the user directed closing the two remaining engine gaps as well — `G-COST-REDUCE-ALLY-DIGIVOLVE` and `G-DSL-COST-RETURN-SELF-DIGI-CARD-BY-NAME` were both implemented, taking BG Imperial to 24/24 IMPLEMENTED. See `phase-0-audit.md` § "Final update".
- Action-space or observation-tensor contract changes.
- Cross-archetype card authoring beyond BG Imperial. Reusable primitives added here may unblock other archetypes, but only BG Imperial cards are authored/tested in this change.
- Python-engine parity work — Rust is the source of truth; these cards migrate one direction only.

## Decisions

### D1: Phase 0 re-audit is a hard gate before any code

The trackers cannot be trusted. Phase 0 produces, for each of the 24 cards, a per-clause classification: `stale-ignore` (primitive exists — re-author YAML + un-`#[ignore]`), `genuine-gap` (cite verified-missing primitive), or `out-of-scope` (BT3-103 Clause 0). No predicate/verb/engine work begins until this exists, because an unknown fraction of "substrate work" is actually re-authoring. Alternative considered: implement straight from the existing gap docs — rejected; it would re-implement already-shipped primitives and miss the real residual.

### D2: Substrate split into three tiers by blast radius

- **Tier 1 — DSL-only predicate leaves.** Engine already holds the data; only `PredicateSpec`/`CompiledPredicate`/`compile`/`validator` wiring plus a runtime eval branch in `dsl_cards/predicate.rs` is missing. Members: `stack_size_lte_source`, `carrier_has_keyword`, `is_carrier_of_source` (aura target filter), `self_digivolution_contains_trait`, opponent/any-scoped `effect_suspended` variant. Batchable as one PR; lowest risk.
- **Tier 2 — engine-touching DSL verbs.** Need new `EffectContext` surface or result-binding plumbing: `select_opponent_sources` (mirror of `select_own_sources`), selected-trash-card → deck-top movement, `any_returned_card` result-set predicate + BT17-077 player-choice-of-trash branch.
- **Tier 3 — engine event payloads.** DNA-origin material/result payloads (G-BG-03 residual) only if Phase 0 confirms a BG card needs more than the already-shipped basic `dna_origin` predicate.

Rationale: tiers are independently shippable and reviewable; Tier 1 unblocks the most cards for the least risk, so it goes first.

### D3: `select_opponent_sources` mirrors the existing `select_own_sources`

`select_own_sources` already supports exact-N / up-to-N, PASS-after-minimum, stable source refs, `filter:`, and `target:`/`from:` host restriction. The opponent variant reuses that machinery with the candidate set drawn from the opponent's battle-area stacks. This keeps the new verb consistent and avoids a parallel selection codepath. Alternative: a generic `select_sources` with an `of:` player parameter — deferred; a focused mirror is lower-risk and matches the established `select_own_*` / `select_opponent_*` naming.

### D4: `stack_size_lte_source` compares against the effect's source permanent

The predicate evaluates `candidate.card_sources.len() <= source_permanent.card_sources.len()` at runtime (`G-PRED-STACK-SIZE-LTE-SOURCE`). It is distinct from the existing literal `stack_size_lte: <u8>`. It is used inside `select_opponent_permanent` / `select_permanent` filters by BT16-027 and BT16-025.

### D5: Trackers updated in the same change as the code

Each closed primitive moves from `qa/dsl-vocab-gaps.md` / `docs/RUST_ENGINE_GAPS.md` to `qa/resolved-gaps.md` with the passing `cargo test` command. The BG cross-archetype doc gets a closure header. Phase 0's re-audit output is committed so the next reader sees the corrected state. This prevents the staleness that motivated this change from recurring.

## Risks / Trade-offs

- **[Phase 0 reveals more genuine gaps than expected]** → Phase 0 output is a gate; if a new engine-level gap surfaces it is filed in `docs/RUST_ENGINE_GAPS.md` and the affected card's clause is explicitly scoped out of this change rather than stubbed. The no-approximations policy forbids shipping a partial primitive.
- **[`#[ignore]` reasons cite gap IDs whose primitive now exists]** → expected and is the point of Phase 0; each such test is un-`#[ignore]`'d and the YAML re-authored, not treated as new work.
- **[DNA-origin payload (Tier 3) balloons into a cross-cutting engine feature]** → scope Tier 3 strictly to what a named BG card's printed text requires; if it needs the full material/result-permanent payload fan-out, split it into its own change and leave those clauses omitted.
- **[Re-authored YAML regresses an already-IMPLEMENTED card]** → every touched card keeps/gains behavioral tests; run the full `cards_behavioral` suite, not just BG tests, before each tier lands.
- **[BT17-077 player-choice-of-trash adds a pending selection]** → reuse `select_effect_choice` (already shipped) wired to an `if`; no new mask category, satisfying the no-tensor-change constraint.

## Open Questions

- Does any BG Imperial card in the 24-card pool actually require the DNA-origin *material/result* payload, or does basic `dna_origin` + the now-shipped `before_pay_cost_observe` cover all of BT12-022 / BT12-050 / BT12-028 / BT16-025 / BT16-085? Resolved by Phase 0.
- ST9-06 / LM-030 cite `G-OPTIONAL-SELECTION-CONTINUE-TAIL` — Phase 2 Track H landed a `select_trash` declined-optional outer-tail continuation. Phase 0 must confirm whether that closes it or a residual remains.
- `G-DECLARATIVE-KEYWORD` (BT20-016 `#[ignore]` line 705) claims declarative `grant_keyword` clauses never fire at runtime, yet `dsl_cards/mod.rs:99` lowers `GrantKeyword`. Phase 0 must determine whether this is a stale ignore or a real firing bug.
