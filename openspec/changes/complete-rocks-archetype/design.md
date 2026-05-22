## Context

The Rocks archetype is a 47-card pool resolved from `data/deck_library.json`. Its verdict ledger (`qa/qa-reports/validated_cards_dsl.json`) reports 2 BLOCKED + 30 PARTIAL + 15 IMPLEMENTED, but those verdicts froze at the 2026-05-04 `rocks-rust-dsl-pool-pass`. Phase 2 Tracks E/I/J have since closed most of the cited substrate gaps (`G-ROCKS-SOURCE-SELECTION-DSL`, `G-PLAY-COST-AGGREGATE`, `G-IGNORE-COLOR-MASK`, `G-DELAY-START-OF-TURN`, `G-SELECT-BREEDING-FILTER`, `force_attack`/`cancel_attack`, and more).

Verified post-merge facts (cargo test, not tracker trust):

- EX10-003 — tracked `BLOCKED`, but production YAML ships `select_own_sources` + `trash_selected_sources` + `cancel_attack`; test passes, 0 ignored. Already done.
- The whole 40-card processed pool has only **9 ignored tests across 4 cards** (BT21-021, BT23-096, EX11-065, P-130).
- BT20-055 has production YAML but two clauses are silently omitted (a YAML comment marks the face-up-security-flip rider as deliberately not approximated; the `[Your Turn]` clause is absent).

So the archetype splits into two populations: **stale-PARTIAL** (substrate landed; card is done or needs only authoring) and **genuinely-incomplete** (5 cards needing a real primitive). The constraint throughout is CLAUDE.md's no-approximations policy (§17): every player choice must surface through `pending_selection`; an omitted clause is BLOCKED, never silently proxied.

## Goals / Non-Goals

**Goals:**

- Drive every Rocks card to a verified `IMPLEMENTED` verdict with faithful DSL YAML and TDD behavioral tests.
- Close the 5 genuine substrate gaps (B1–B5) with reusable, cross-archetype primitives where natural.
- Reconcile `validated_cards_dsl.json` and the gap trackers to verified state; prune every stale `#[ignore]`.
- Keep the full `cargo test` suite green throughout.

**Non-Goals:**

- A general DSL/engine refactor. Each primitive is the minimum surface to faithfully express its card(s).
- Closing gaps for non-Rocks archetypes, except where B3/B4 are deliberately scoped generically (their card consumers in other archetypes stay out of this change's test scope).
- Changing the RL action-space or tensor contract beyond an additive pending-selection sub-range, should B3/B4 require one (handled per the existing Group-5 additive contract).
- Python-engine parity work — Rocks is Rust-DSL-owned.

## Decisions

### D1 — Two-phase structure: authoring re-audit, then substrate

Phase A re-audits the 30 PARTIAL cards (no engine code) before Phase B touches the engine. Rationale: the stale-vs-genuine split is unknown, and EX10-003 proves stale-PARTIAL exists. Auditing first turns "30 PARTIAL" into a precise list of genuinely-incomplete cards, so Phase B is scoped against verified need, not stale tracker notes. Alternative considered — substrate-first — was rejected: it risks building primitives for cards that are already done.

### D2 — Calibration spike gates the estimate

Task block A starts with a 5-card sample audit. If most return `AUDITED-OK`, Phase A is mostly reclassification; if most need a clause authored, Phase A is real authoring work. Rationale: the total effort swings entirely on this ratio, and a 5-card spike is cheap insurance against a wrong plan. Alternative — audit all 30 up front — is also fine but doesn't give an early go/no-go signal.

### D3 — B1 collapsed: not a substrate gap (spike outcome)

The calibration spike resolved Open Question #2: `source_permanent_trait_has` already exists (`predicate.rs:369`) and resolves against the carrier permanent for inherited clauses. BT21-021's inherited Rush aura needs no new predicate — its ignored test is a stale marker. **B1 is removed from Phase B** and folded into Phase A authoring: re-author the aura with `source_permanent_trait_has: "Xros Heart"` as its condition, verify the aura evaluation path consults it, and re-enable the test. If verification reveals the aura path does *not* consult the predicate, that narrow wiring fix is the only residual — but the predicate leaf itself is not new work.

### D4 — B2 move-from-breeding: DSL verb over the existing engine method

`EffectContext::move_from_breeding_by_effect` already exists and fires the move observers. B2 is purely a DSL surface: a `move_from_breeding` step plus an optional accept/decline prompt wrapper, and it pairs with the already-wired `SelectOwnBreedingPermanentArgs::filter` for the level-3 gate. Rationale: the engine substrate is done; only the lowering and the optional-prompt gate are missing. Alternative — a `raw_rust` bridge — is rejected: it cannot express the optional prompt or the level filter without violating §17.

### D5 — B3 union-zone cost selector: scope generically, validate against Royal Knights

EX11-065 needs hand ∪ own-digivolution-sources. Royal Knights `G-UNION-HAND-TRASH-SOURCE-COST` (BT20-021) needs hand ∪ trash. These are the same shape — "select 1 filtered card from a union of two zones as a cost." B3 builds one union-zone cost selector parameterised by zone set, covering both. Rationale: one primitive closes gaps across two archetypes. Trade-off: slightly larger surface than a Rocks-only selector; the BT20-021 consumer is built but not card-tested in this change. Alternative — a Rocks-only hand∪source selector — was rejected as duplicative.

### D6 — B4 face-up security: two independent primitives, watch for cross-archetype ownership

BT20-055 needs a `flip_security_face_up` no-choice step and a "checks a face-up security card" observer timing. `RUST_ENGINE_GAPS.md` flags face-up security lifecycle as cross-archetype (Dark Masters audit hits it too) and lists `flip_security_face_up` as a 🟡 residual. B4 builds exactly the two primitives BT20-055 needs and no more. Open risk: another track may already own the broader face-up lifecycle — see Open Questions.

### D7 — B5 Delay-on-attack: a three-system fix, treat as its own task block

The BT23-096 ignore reason is code-verified: (1) `lower_delay.rs` maps only a fixed timing set to `DelayTrigger`, silently degrading attack timings to `EndOfYourNextTurn`; (2) `effect_queue.rs` only fans `EventObserved`/`AttackTargetChanged` to event-gated delays, while `combat.rs` dispatches `OnAllyAttack` via `TriggerSource::PlayerBattleArea`; (3) `attacker_trait_has` reads the attacker only via `attack_target_change()`, unset for a plain attack. B5 fixes all three. Rationale: it is the largest and riskiest gap (combat dispatch is hot-path) and gets its own task block with isolated tests. Alternative — omit BT23-096's Delay clause permanently — violates the archetype-completion goal.

### D8 — TDD and the existing batch pipeline

Phase A authoring runs through the existing `/batch-implement-cards-rust-dsl` AUDIT mode; Phase B card-completion follows the TDD walkthrough in `RUST_DSL_TEST_API.md` (failing test first, then YAML). Rationale: reuse the proven pipeline and its scout/implementer/review discipline rather than inventing a parallel process.

## Risks / Trade-offs

- **Population 1 size is unknown** → D2's calibration spike resolves it before the full plan commits; the tasks file is structured so Phase A scales without re-planning.
- **B5 touches combat hot-path dispatch** → isolate B5 in its own task block; add `tests/replacements/` or `tests/combat/` coverage for the new fan-out before authoring BT23-096; run the full `combat` + `option_flow` suites as the regression gate.
- **B3/B4 may need a new pending-selection action sub-range** → if so, make it additive only (append to `ACTION_SPACE_SIZE`), never renumber existing actions, per the Group-5 contract note; update `ACTION_SPEC.md` in the same change.
- **B4 may be double-owned with a Dark Masters / face-up-security track** → see Open Questions; if owned elsewhere, B4 becomes a dependency rather than in-scope work.
- **Stale trackers mislead authoring agents** → every `#[ignore]` and tracker note is verified against current engine source, not trusted; the spec's "no ignore for a closed gap" requirement enforces this.
- **Re-auditing 30 cards is agent-time expensive** → batch in groups of 4 via the existing pipeline; the calibration spike (D2) prevents wasted effort if the population is mostly already-done.

## Migration Plan

1. **Phase A** — calibration spike (5 cards), then full AUDIT-mode pass over remaining PARTIAL cards in batches of 4. Reclassify stale-PARTIAL; author clauses whose substrate already landed. Update `validated_cards_dsl.json` per batch.
2. **Phase B** — land B2–B5 as independent substrate slices (B1 collapsed into Phase A per D3; any order, B5 last given risk). Each slice: failing test → primitive → green test → move gap entry to `qa/resolved-gaps.md`.
3. **Phase C** — author the final clause of the 5 substrate-blocked cards (BT21-021, P-130, EX11-065, BT20-055, BT23-096) against the new primitives, TDD.
4. **Phase D** — reconcile: prune stale `#[ignore]`, finalize `validated_cards_dsl.json`, update `qa/archetype-qa/dsl/rocks.md`, run the full `cargo test` suite as the completion gate.

Rollback: each phase is independently revertible. Phase B slices are isolated primitives; reverting one leaves the others and Phase A intact. No data migration, no deploy coupling.

## Open Questions

- **Is B4 (face-up security lifecycle) already owned by another track?** `RUST_ENGINE_GAPS.md` flags it as cross-archetype (Dark Masters). Before building B4, confirm no in-flight track owns `flip_security_face_up` / face-up extraction — if one does, B4 becomes a dependency.
- **~~Does B1 already exist?~~** RESOLVED by the spike: `source_permanent_trait_has` exists (`predicate.rs:369`) and resolves against the carrier for inherited clauses. B1 is not new substrate — folded into Phase A authoring (see D3).
- **Does B3 need its own pending-selection kind, or can it compose existing `SelectionKind`s?** Determines whether B3 is action-space-additive or purely a lowering change.
- **EX10-003 reclassification** — confirm it flips cleanly to `IMPLEMENTED` in Phase A (tracker says BLOCKED, tests say done); it is the canonical stale-PARTIAL/BLOCKED proof case.
