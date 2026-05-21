# Puppets Substrate Sweep — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Close every remaining open Puppets substrate gap so the archetype reaches ~99% substrate-complete and the only remaining work is pure card authoring via `/batch-implement-cards-rust-dsl`.

**Architecture:** Three tiers of work — (1) DSL-vocabulary additions where the engine primitive already exists, (2) a shared origin-preserving union-zone play substrate consumed by three cards, (3) standalone engine-substrate gaps. Every task is TDD: a failing `DebugRunner` behavioral test (CLAUDE.md §18) precedes implementation; most target tests already exist `#[ignore]`-annotated, so the "write failing test" step is usually "un-ignore + confirm it fails for the right reason." No-approximations policy (CLAUDE.md §17) applies — every player choice surfaces through `pending_selection`.

**Tech Stack:** Rust engine (`code/digimon-engine/`), DSL crate (`code/digimon-dsl/`) + lowering (`code/digimon-engine/src/dsl_cards/`), YAML card specs (`code/digimon-engine/cards/`), `cargo test`.

---

## Verified gap inventory

State below was verified against the engine source on 2026-05-19 (the gap trackers were internally inconsistent — this table supersedes them). PUPPETS-G031 (EX4-074 End-of-Attack chain) is **already closed** and excluded. 15 gaps remain.

| Gap | Tier | Severity | Cards | Verified residual |
|---|---|---|---|---|
| G012 | DSL-vocab | 🟡 | EX11-020 | `event_cause` predicate compiles; OnDeletion builder sets `cause: None` — wiring fix |
| G016 (bind) | DSL-vocab | 🟡 | P-165 | `PlayTokenArgs` lacks `bind_as`; engine already returns the handle |
| G010 | DSL-vocab | 🟡 | BT15-003 | No `trash_bottom_security` / `trash_selected_security` verb |
| G023 | DSL-vocab | 🟡 | BT13-101, P-136 | suspend-self cost shipped (Track B); event-card color predicates missing |
| G025 | DSL-vocab | 🟡 | BT16-055 | No `rules_text_contains` predicate |
| G021 | DSL-vocab | 🟡 | EX11-022 | Union-zone filter doesn't eval `CardData.dp` for hand/trash cards |
| G014 | Union substrate | 🟡 | ST19-08, BT22-098 | Union selection binds a card but no play-verb routes it back to origin zone |
| G028 | Union substrate | 🟡 | BT22-088 | return-self cost shipped (Track B); chained branch-selector over origin-play missing |
| G020 | Union substrate | 🟡 | BT22-036 | Hand-`[Main]` trash-cost preflight + bottom-source + self-digivolve unproven |
| G003 | Provenance | 🔴 | EX11-022, EX11-061 | `ProvenanceToken` plumbing exists; no provenance-bound turn-end self-delete + DSL step |
| G016 (cleanup) | Provenance | 🔴 | P-165 | Scheduled delete of a bound token at end of opponent's turn |
| G009 | Engine | 🟡 | P-037, P-105, LM-035, LM-037, LM-054 | No main-phase player action to activate a Delayed Option |
| G018 | Engine | 🔴 | EX9-032 | `ctx.source_permanent` holds a stale index after a mid-body delete |
| G015 | Engine | 🟡 | ST19-11 | No conditional/threshold modifier *amount* shape |
| G024 | Engine | 🔴 | BT16-055 | `ImmuneFromDPMinus`+`CannotBeDeDigivolved` exist but no security-gated DSL grant pair |
| G030 | Engine | 🔴 | BT5-106 | No `suppress_on_play` flag on effect-play helpers |

## PR grouping

- **PR 1 — Wave 1: DSL-vocab quick wins** (G012, G016-bind, G010, G023, G025, G021). Low risk, no deep engine work.
- **PR 2 — Wave 2: Union-zone origin-preserving play substrate** (core + G014, G028, G020).
- **PR 3 — Wave 3: Effect-played provenance + scheduled cleanup** (G003, G016-cleanup).
- **PR 4 — Wave 4: Standalone engine substrate** (G009, G018, G015, G024, G030). Split further if review gets heavy — G009 and G018 each merit their own PR.

Waves are independent of each other and may run in parallel; within Wave 2 the core substrate task precedes G014/G028/G020. EX11-022 needs G021 (PR 1) **and** G014 (PR 2) **and** G003 (PR 3) before it fully clears — author its card last.

## Cross-cutting constraints

- **Working Rule §1:** No `ACTION_SPACE_SIZE` change. G009's Delay-activation action reuses the existing field-effect / pending-selection action surface.
- **CLAUDE.md §17:** No approximations. Every player choice (top-vs-bottom security, branch selection, optional costs, Delay activate-or-decline) surfaces through `pending_selection` / the action mask.
- **CLAUDE.md §18:** Failing `DebugRunner` behavioral test before implementation, every task.
- **Source priority:** printed card text → `docs/RULES_CONTEXT.md` → fandom wiki → DCGO. Cite the printed text for each card's effect when authoring.
- If a card surfaces a NEW gap not in PUPPETS-G001..G032, file it as `PUPPETS-G033+` in `qa/archetype-qa/dsl/puppets-2026-05-03-engine-dsl-gaps.md` and leave that card PARTIAL — do not absorb scope creep.

---

## Wave 1 — DSL-vocab quick wins (PR 1)

### Task 1: G012 — thread deletion cause into OnDeletion `TriggerContext`

The DSL `event_cause` predicate already parses, compiles, and evaluates (`code/digimon-dsl/src/predicate.rs:180,347`; `code/digimon-engine/src/dsl_cards/predicate.rs:989-1067`). The bug: the OnDeletion trigger is built via `TriggerSource::Permanent` whose `build` sets `cause: None` (`code/digimon-engine/src/effect_queue.rs:700-705`), so `EffectReadContext::event_cause()` returns `None` during OnDeletion resolution and a `deletion_cause_not: battle` gate never matches.

**Files:**
- Modify: `code/digimon-engine/src/effect_queue.rs:700-705` (OnDeletion `TriggerSource::Permanent` build)
- Modify: `code/digimon-engine/src/cards/` or `cards/ex11/EX11-020.yaml` — author the On Deletion clause
- Test: `code/digimon-engine/tests/cards_behavioral/ex11/ex11_020.rs`

- [ ] **Step 1: Un-ignore the failing test.** Remove `#[ignore]` from `ex11_020_on_deletion_does_not_fire_when_deleted_in_battle` (and siblings citing G012) in `ex11_020.rs`.
- [ ] **Step 2: Confirm it fails.** Run `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- ex11_020` — expect FAIL: the Shoemon free-play prompt appears even on battle deletion.
- [ ] **Step 3: Implement.** In `effect_queue.rs:700-705`, thread the deletion cause (battle / own-effect / opponent-effect) into the `TriggerContext` the OnDeletion trigger carries, mirroring how replacement-context cause is threaded. The cause is available where `delete_permanent` is invoked — pass it through to the trigger builder.
- [ ] **Step 4: Author EX11-020 YAML.** Add the `[On Deletion]` clause with `event_cause` (or `deletion_cause_not: battle`) gating the Shoemon-trait free-play.
- [ ] **Step 5: Confirm pass.** Run the Step 2 command — expect PASS for battle and non-battle causes.
- [ ] **Step 6: Commit.** `git add code/digimon-engine/src/effect_queue.rs code/digimon-engine/cards/ex11/EX11-020.yaml code/digimon-engine/tests/cards_behavioral/ex11/ex11_020.rs && git commit -m "fix(engine): thread deletion cause into OnDeletion TriggerContext (PUPPETS-G012)"`

### Task 2: G016 (binding half) — `bind_as` on `PlayTokenArgs`

`EffectContext::play_token` already returns `Some(PermanentHandle)` (`code/digimon-engine/src/cards/play_digivolve.rs:334`), but the DSL `PlayTokenArgs` struct (`code/digimon-dsl/src/step.rs:1211`) has only `controller` / `token_name` — no binding slot, so YAML can't reference the created token. (The end-of-opponent-turn *cleanup* of that token is Task 11, Wave 3.)

**Files:**
- Modify: `code/digimon-dsl/src/step.rs:1211` (`PlayTokenArgs` — add `bind_as: Option<String>`)
- Modify: `code/digimon-engine/src/dsl_cards/` play-token lowering (where `play_token`'s result is currently discarded) — register the returned handle into the binding table
- Test: `code/digimon-engine/tests/dsl/` — a new step-level test, plus `cards_behavioral/p/p_165.rs`

- [ ] **Step 1: Write the failing test.** Add `play_token_binds_created_handle` to the DSL test suite: a card that does `play_token: { token_name: familiar, bind_as: tok }` then a step referencing binding `tok` (e.g. `add_modifier: { target: tok, ... }`); assert the modifier lands on the created token.
- [ ] **Step 2: Confirm it fails.** `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- play_token_binds` — expect FAIL: unknown binding `tok`.
- [ ] **Step 3: Implement.** Add `bind_as: Option<String>` to `PlayTokenArgs`; in the lowering, when `bind_as` is set, insert the `PermanentHandle` returned by `play_token` into the effect's binding table under that name.
- [ ] **Step 4: Confirm pass.** Re-run Step 2 command — expect PASS.
- [ ] **Step 5: Commit.** `git commit -m "feat(dsl): add bind_as to play_token (PUPPETS-G016 binding half)"`

### Task 3: G010 — `trash_bottom_security` / `trash_selected_security` verb

Engine has `TrashTopSecurity`; no bottom or selected-index variant. BT15-003: "By trashing the top or bottom card of your security stack, gain 1 memory."

**Files:**
- Modify: `code/digimon-dsl/src/step.rs` — add the new step variant(s)
- Modify: `code/digimon-engine/src/dsl_cards/step/` — lowering, routing the card through the same trash hooks `TrashTopSecurity` uses
- Modify: `code/digimon-engine/cards/bt15/BT15-003.yaml`
- Test: `code/digimon-engine/tests/cards_behavioral/bt15/bt15_003.rs`

- [ ] **Step 1: Un-ignore the bottom-branch test** in `bt15_003.rs` (the test that picks "Trash bottom security").
- [ ] **Step 2: Confirm it fails.** `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- bt15_003` — expect FAIL.
- [ ] **Step 3: Implement.** Add a `trash_bottom_security` step (and/or `trash_selected_security` consuming a `select_security` index binding). Lower it to the same trash/move hooks as top-security trash, targeting the bottom index. The visible top-vs-bottom choice is a `select_effect_choice` in the YAML.
- [ ] **Step 4: Author BT15-003 YAML** using `select_effect_choice` → `trash_top_security` / `trash_bottom_security` branches, with a no-security gate.
- [ ] **Step 5: Confirm pass.** Re-run Step 2 command.
- [ ] **Step 6: Commit.** `git commit -m "feat(dsl): add trash_bottom_security verb (PUPPETS-G010)"`

### Task 4: G023 — event-card color predicates

Track B already shipped the suspend-self activation cost (`EffectBuilder::activation_cost`, `ctx.suspend_self_as_cost()` at `effect_context/mod.rs:2215`). Missing: predicates to detect "a 2-color black/yellow Digimon" on a play observer. P-136 already passes (0 ignored) — this task is BT13-101.

**Files:**
- Modify: `code/digimon-dsl/src/predicate.rs` — add `event_card_color_only: Vec<Color>` and `event_card_color_count: u8`
- Modify: `code/digimon-engine/src/dsl_cards/predicate.rs` — compiled eval against the triggering card's colors
- Modify: `code/digimon-engine/cards/bt13/BT13-101.yaml`
- Test: `code/digimon-engine/tests/cards_behavioral/bt13/bt13_101.rs`

- [ ] **Step 1: Un-ignore** the three G023-tagged tests in `bt13_101.rs`.
- [ ] **Step 2: Confirm fail.** `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- bt13_101`.
- [ ] **Step 3: Implement predicates.** Add `event_card_color_only` (every color of the triggering card is in the set) and `event_card_color_count` (exact distinct-color count) to the predicate spec + compiled eval, reading the triggering card's colors from the event payload.
- [ ] **Step 4: Author BT13-101** `[All Turns]` observer: `event_card_color_only: [black, yellow]` + `event_card_color_count: 2`, `activation_cost: { suspend_self: true }`, body = `<Draw 1>` + gain 1 memory.
- [ ] **Step 5: Confirm pass.** Re-run Step 2 command.
- [ ] **Step 6: Commit.** `git commit -m "feat(dsl): add event-card color predicates (PUPPETS-G023)"`

### Task 5: G025 — `rules_text_contains` predicate

BT16-055 inherited: "While this Digimon has [Pulsemon] in its text, it gets +1000 DP." Must inspect the carrier stack's printed rules text — not name/trait.

**Files:**
- Modify: `code/digimon-dsl/src/predicate.rs` — add `rules_text_contains: String`
- Modify: `code/digimon-engine/src/dsl_cards/predicate.rs` — eval against the carrier top-card / stack printed text in inherited-effect read context
- Modify: `code/digimon-engine/cards/bt16/BT16-055.yaml`
- Test: `code/digimon-engine/tests/cards_behavioral/bt16/bt16_055.rs`

- [ ] **Step 1: Un-ignore** the G025-tagged inherited-aura test in `bt16_055.rs`.
- [ ] **Step 2: Confirm fail.** `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- bt16_055`.
- [ ] **Step 3: Implement.** Add a `rules_text_contains` predicate that, during inherited-effect evaluation, reads the carrier's effective rules text (top card; extend to stack if a card needs it) and substring-matches case-insensitively.
- [ ] **Step 4: Author** the BT16-055 inherited `[Your Turn]` aura clause gated on `rules_text_contains: "[Pulsemon]"`.
- [ ] **Step 5: Confirm pass.** Re-run Step 2 command.
- [ ] **Step 6: Commit.** `git commit -m "feat(dsl): add rules_text_contains predicate (PUPPETS-G025)"`

### Task 6: G021 — hidden-zone DP eval in union-zone filters

`SelectUnionArgs.filter` (`code/digimon-dsl/src/step.rs:1606`) accepts a `PredicateSpec`; card predicates `dp_eq`/`dp_gte` exist (`predicate.rs:71,75`). They are not evaluated for hidden-zone (hand/trash) candidate cards, so a `dp <= 4000` filter over a hand/trash union does nothing.

**Files:**
- Modify: `code/digimon-engine/src/dsl_cards/predicate.rs` — make the DP predicate read `CardData.dp` by card id for hand/trash candidates
- Test: `code/digimon-engine/tests/cards_behavioral/ex11/ex11_022.rs` (selection-only test)

- [ ] **Step 1: Un-ignore** the EX11-022 union-selection test (the one citing `G-HAND-TRASH-CARD-DP-FILTER`).
- [ ] **Step 2: Confirm fail.** `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- ex11_022`.
- [ ] **Step 3: Implement.** In the compiled DP predicate, when the candidate is a hand/trash `CardSource`, resolve its `CardData` by card id and compare `dp`. Case the existing battle-area path unchanged.
- [ ] **Step 4: Confirm pass** for the selection test (full EX11-022 still needs G014 + G003 — leave those sub-tests ignored).
- [ ] **Step 5: Commit.** `git commit -m "feat(dsl): evaluate card DP for hidden-zone union candidates (PUPPETS-G021)"`

---

## Wave 2 — Union-zone origin-preserving play substrate (PR 2)

### Task 7: Core — origin-preserving union-zone play consumer

`SelectUnionZone` binds a `CardHandle` but loses which zone the card came from, and no play verb routes a union-bound card back to its origin. This substrate is shared by G014, G028, and (partly) G020.

**Files:**
- Modify: `code/digimon-dsl/src/step.rs:1606` (`SelectUnionArgs`) — ensure the binding records origin zone (`Hand` / `Trash`)
- Modify: `code/digimon-engine/src/dsl_cards/step/` — add a `play_union_bound_free` step that dispatches to `play_from_hand_free` or `play_from_trash_free` per recorded origin
- Modify: `code/digimon-engine/src/effect_context/mod.rs` if the binding type needs an origin field
- Test: `code/digimon-engine/tests/dsl/` — new `union_zone_origin_play` test

- [ ] **Step 1: Write the failing test.** A DSL fixture: one eligible card in hand, one in trash; `select_union_zone` then `play_union_bound_free`; assert the chosen card is played from its true origin and the other zone is untouched. Run both branches.
- [ ] **Step 2: Confirm fail.** `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- union_zone_origin_play`.
- [ ] **Step 3: Implement.** Carry origin in the union binding; add the `play_union_bound_free` step + lowering that branches to the correct `play_from_*_free` helper.
- [ ] **Step 4: Confirm pass.** Re-run Step 2 command.
- [ ] **Step 5: Commit.** `git commit -m "feat(dsl): origin-preserving union-zone play consumer"`

### Task 8: G014 — filtered hand-or-trash security free-play

ST19-08 `[Security]`: "play 1 [LIBERATOR] card with play cost 4 or less from your hand or trash without paying the cost." BT22-098 uses the same surface.

**Files:**
- Modify: `code/digimon-engine/cards/st19/ST19-08.yaml`, `code/digimon-engine/cards/bt22/BT22-098.yaml`
- Test: `code/digimon-engine/tests/cards_behavioral/st19/st19_08.rs`, `.../bt22/bt22_098.rs`

- [ ] **Step 1: Un-ignore** the G014-tagged tests in `st19_08.rs` (and the hand-or-trash test in `bt22_098.rs`).
- [ ] **Step 2: Confirm fail.** `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- st19_08 bt22_098`.
- [ ] **Step 3: Author** ST19-08 / BT22-098 using `select_union_zone` (filter: `[LIBERATOR]` trait + play cost ≤ 4) → `play_union_bound_free` from Task 7.
- [ ] **Step 4: Confirm pass.** Re-run Step 2 command.
- [ ] **Step 5: Commit.** `git commit -m "feat(cards): ST19-08/BT22-098 hand-or-trash security free-play (PUPPETS-G014)"`

### Task 9: G028 — return-self cost + chained branch selector

BT22-088 `[Start of Your Main Phase]`: return-this-Tamer-to-deck-bottom cost (shipped: `ctx.return_self_to_deck_bottom_as_cost()`, `effect_context/mod.rs:2243`), then a player choice between two named free-play branches.

**Files:**
- Modify: `code/digimon-engine/cards/bt22/BT22-088.yaml`
- Test: `code/digimon-engine/tests/cards_behavioral/bt22/bt22_088.rs`

- [ ] **Step 1: Un-ignore** the three G028-tagged Start-of-Main tests in `bt22_088.rs`.
- [ ] **Step 2: Confirm fail.** `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- bt22_088`.
- [ ] **Step 3: Implement / author.** Compose `activation_cost: { return_self_to_deck_bottom: true }` + `select_effect_choice` over two branches: (a) play a different-numbered [Arisa Kinosaki] from hand free; (b) play a [Shoemon] from hand-or-trash free (Task 7 consumer). If `select_effect_choice` cannot follow an activation cost today, add that sequencing in the lowering.
- [ ] **Step 4: Confirm pass.** Re-run Step 2 command.
- [ ] **Step 5: Commit.** `git commit -m "feat(cards): BT22-088 return-self cost + branch selector (PUPPETS-G028)"`

### Task 10: G020 — hand-`[Main]` trash-cost preflight + bottom-source + self-digivolve

BT22-036 `[Hand][Main]`: if you have [Arisa Kinosaki], place a [ShoeShoemon] from trash as one of your [Shoemon]'s bottom digivolution card, then that Shoemon digivolves into BT22-036 (from hand) for cost 3 ignoring requirements.

**Files:**
- Modify: `code/digimon-engine/src/action/mask.rs` — hand-`[Main]` activation must preflight "required trash card exists"
- Modify: `code/digimon-engine/src/dsl_cards/step/` — bind an exact trash `CardSource` for `place_as_bottom_source`; allow the chosen field Shoemon to digivolve into the resolving hand card
- Modify: `code/digimon-engine/cards/bt22/BT22-036.yaml`
- Test: `code/digimon-engine/tests/cards_behavioral/bt22/bt22_036.rs`

- [ ] **Step 1: Un-ignore** the `G-HAND-MAIN-TRASH-PREFLIGHT` / `G-HAND-MAIN-SELF-DIGIVOLVE` tests in `bt22_036.rs`.
- [ ] **Step 2: Confirm fail.** `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- bt22_036`.
- [ ] **Step 3: Implement.** Add hand-`[Main]` mask preflight for the card-specific trash cost + Arisa board condition; bind the selected trash `CardSource` and route it through `place_as_bottom_source`; let the chosen Shoemon digivolve into the resolving hand card at cost 3 ignoring requirements.
- [ ] **Step 4: Author BT22-036** hand-`[Main]` clause.
- [ ] **Step 5: Confirm pass.** Re-run Step 2 command.
- [ ] **Step 6: Commit.** `git commit -m "feat(engine): hand-Main trash-cost preflight + self-digivolve chain (PUPPETS-G020)"`

---

## Wave 3 — Effect-played provenance + scheduled cleanup (PR 3)

### Task 11: G003 — provenance-bound turn-end self-delete

Provenance plumbing exists: `ProvenanceToken` (`trigger_context.rs:28`), `provenance_token_for_card` / `resolve_provenance_token` (`game.rs:2683-2692`), `play_from_hand_free_with_provenance` (`effect_context/mod.rs:2582`), `schedule_delayed` + the `scheduled_effects.rs` queue. Missing: a helper that schedules deletion *of a provenance-bound permanent* at turn end, plus a DSL step.

**Files:**
- Modify: `code/digimon-engine/src/effect_context/mod.rs` — add `schedule_delete_at_end_of_turn(token: ProvenanceToken)`
- Modify: `code/digimon-engine/src/game.rs` — drain the scheduled provenance deletions inside `end_turn` (resolve the token; no-op if the permanent is already gone)
- Modify: `code/digimon-dsl/src/step.rs` + `code/digimon-engine/src/dsl_cards/step/` — DSL step `schedule_delete_played_at_turn_end` (consumes a provenance binding from a preceding free-play step)
- Modify: `code/digimon-engine/cards/ex11/EX11-022.yaml`, `code/digimon-engine/cards/ex11/EX11-061.yaml`
- Test: `code/digimon-engine/tests/cards_behavioral/ex11/ex11_022.rs`, `.../ex11/ex11_061.rs`

- [ ] **Step 1: Un-ignore** the cleanup-rider tests in `ex11_022.rs` and `ex11_061.rs`.
- [ ] **Step 2: Confirm fail.** `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- ex11_022 ex11_061`.
- [ ] **Step 3: Implement.** Add `schedule_delete_at_end_of_turn`; have the free-play step record a provenance token; drain at `end_turn` after `EndOfYourTurn` triggers, resolving the token to a live permanent (no-op if absent — shifted indices must not over-delete).
- [ ] **Step 4: Author** EX11-022 (free-play from hand/trash via Wave 2 + this cleanup rider) and EX11-061.
- [ ] **Step 5: Confirm pass.** Re-run Step 2 command. EX11-022 now needs G021 + G014 + G003 all merged.
- [ ] **Step 6: Commit.** `git commit -m "feat(engine): provenance-bound turn-end self-delete (PUPPETS-G003)"`

### Task 12: G016 (cleanup half) — scheduled delete of a bound token

P-165: "At the end of your opponent's turn, delete that token." Builds on Task 2's `bind_as` and Task 11's scheduling.

**Files:**
- Modify: `code/digimon-engine/src/dsl_cards/step/` — allow the turn-end-delete schedule to consume a token `PermanentHandle` binding at the `EndOfOpponentsTurn` boundary
- Modify: `code/digimon-engine/cards/p/P-165.yaml`
- Test: `code/digimon-engine/tests/cards_behavioral/p/p_165.rs`

- [ ] **Step 1: Un-ignore** the G016 token-cleanup test in `p_165.rs`.
- [ ] **Step 2: Confirm fail.** `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- p_165`.
- [ ] **Step 3: Implement.** Extend the Task 11 scheduling to also key on a bound `PermanentHandle` (the `bind_as` token) and an `EndOfOpponentsTurn` boundary. Verify a second Familiar Token from another effect is NOT deleted.
- [ ] **Step 4: Author P-165** using `play_token: { bind_as: tok }` + `schedule_delete_at_end_of_opponents_turn: { target: tok }`.
- [ ] **Step 5: Confirm pass.** Re-run Step 2 command.
- [ ] **Step 6: Commit.** `git commit -m "feat(engine): scheduled token cleanup at end of opponent turn (PUPPETS-G016)"`

---

## Wave 4 — Standalone engine substrate (PR 4 — split if review is heavy)

### Task 13: G009 — Standard Delay `[Main]`-phase activation action

Standard `<Delay>` Options (Memory Boost / Training / Scramble) currently auto-fire on a scheduled scan, hiding the controller's choice to activate or decline after the placing turn. Per Working Rule §1, reuse the existing field-effect action range — no `ACTION_SPACE_SIZE` change.

**Files:**
- Modify: `code/digimon-engine/src/option_lifecycle.rs` — an activation mode for standard `kind: delay` distinct from event/start/end scheduled triggers
- Modify: `code/digimon-engine/src/action/mask.rs` — emit a field-effect activation for each Delayed Option in the controller's battle area after the placing turn (PASS stays legal)
- Modify: action decode — activating trashes the Option as cost, then runs the Delay body; declining leaves it on field
- Test: `code/digimon-engine/tests/option_flow.rs` + `code/digimon-engine/tests/cards_behavioral/p/p_105.rs`

- [ ] **Step 1: Write the failing test.** In `option_flow.rs`: place a standard Memory Boost as `OptionState::Delayed`, advance past the placing turn into its controller's Main phase, assert the mask exposes a field-effect activation while PASS stays legal; activating trashes the Option and runs the body; declining leaves it.
- [ ] **Step 2: Confirm fail.** `cargo test --manifest-path code/digimon-engine/Cargo.toml --test option_flow`.
- [ ] **Step 3: Implement** the activation mode, mask emission, and decode.
- [ ] **Step 4: Un-ignore + re-author** the G009 tests in `p_105.rs` (and re-confirm P-037/LM-035/LM-037/LM-054 against the new lifecycle).
- [ ] **Step 5: Confirm pass.** `cargo test --manifest-path code/digimon-engine/Cargo.toml --test option_flow` and `--test cards_behavioral -- p_105 p_037 lm_035 lm_037 lm_054`.
- [ ] **Step 6: Commit.** `git commit -m "feat(engine): standard Delay main-phase activation action (PUPPETS-G009)"`

### Task 14: G018 — stable `source_permanent` across mid-body delete

EX9-032 Karakurumon: a cost step deletes a lower-indexed own permanent, shifting the battle area; `ctx.source_permanent` keeps the stale index and the later `target: self` digivolve hits the wrong slot. Per `docs/RUST_ENGINE_GAPS.md` "Costed self-digivolve stable source binding", use the lower-blast-radius approach: snapshot the source card identity.

**Files:**
- Modify: `code/digimon-engine/src/effect_context/mod.rs` — add a `source_permanent_card: CardHandle` snapshot alongside `source_permanent`
- Modify: `code/digimon-engine/src/binding_ref.rs` — `Source` resolution re-locates the live permanent by card handle when the cached index is stale
- Modify: `code/digimon-engine/cards/ex9/EX9-032.yaml` — restore the active On Play / When Digivolving costed self-digivolve clause
- Test: `code/digimon-engine/tests/cards_behavioral/ex9/ex9_032.rs`

- [ ] **Step 1: Un-ignore** `ex9_032_..._STABLE-SOURCE` and `..._PREFLIGHT` tests.
- [ ] **Step 2: Confirm fail.** `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- ex9_032`.
- [ ] **Step 3: Implement.** Snapshot `source_permanent_card`; have `binding_ref::Source` resolution search the live battle area for that card handle when the index no longer points at it. Cost preflight must prove a legal Token/other-[Puppet] body exists, excluding the source itself.
- [ ] **Step 4: Re-author EX9-032** active clause.
- [ ] **Step 5: Confirm pass.** Re-run Step 2 command (all 7 tests).
- [ ] **Step 6: Commit.** `git commit -m "fix(engine): stable source binding across mid-body delete (PUPPETS-G018)"`

### Task 15: G015 — conditional/threshold modifier amount

ST19-11: "1 of your opponent's Digimon gets -3000 DP for the turn. If there are 3 or more Digimon, increase the DP reduction by -3000." Subjectless `count_gte` exists (`predicate.rs:226`) but there is no clean way to branch a modifier *amount* on it.

**Files:**
- Modify: `code/digimon-dsl/src/step.rs` + `code/digimon-engine/src/dsl_cards/step/` — a conditional second `add_modifier` (or a formula-valued modifier amount) gated on the count predicate
- Modify: `code/digimon-engine/cards/st19/ST19-11.yaml`
- Test: `code/digimon-engine/tests/cards_behavioral/st19/st19_11.rs`

- [ ] **Step 1: Un-ignore** the threshold-branch test in `st19_11.rs`.
- [ ] **Step 2: Confirm fail.** `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- st19_11`.
- [ ] **Step 3: Implement.** Prefer a conditional second `add_modifier` step gated by `count_gte` over both battle areas (reuses verified-working subjectless aggregate eval) rather than a new formula amount shape — smaller blast radius.
- [ ] **Step 4: Author ST19-11.** Assert -3000 at 2 Digimon, -6000 at 3.
- [ ] **Step 5: Confirm pass.** Re-run Step 2 command.
- [ ] **Step 6: Commit.** `git commit -m "feat(dsl): count-threshold conditional modifier amount (PUPPETS-G015)"`

### Task 16: G024 — security-gated narrow opponent-effect protection

BT16-055: "While you have 3 or more security cards, this Digimon isn't affected by your opponent's DP reduction effects and can't be de-digivolved by their effects." Both `ModifierType::ImmuneFromDPMinus` (opponent-filtered via `effect_immunity_filter`, `enums.rs:677`) and `ModifierType::CannotBeDeDigivolved` (`enums.rs:530`) exist — they just need a DSL-grantable pairing gated on a live security-count predicate.

**Files:**
- Modify: `code/digimon-engine/src/dsl_cards/` — allow a declarative aura / `add_modifier` to grant the opponent-filtered `ImmuneFromDPMinus` + `CannotBeDeDigivolved` pair with a `while_condition` security-count gate
- Modify: `code/digimon-engine/cards/bt16/BT16-055.yaml`
- Test: `code/digimon-engine/tests/cards_behavioral/bt16/bt16_055.rs`

- [ ] **Step 1: Un-ignore** the G024 high-security protection test in `bt16_055.rs`.
- [ ] **Step 2: Confirm fail.** `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- bt16_055`.
- [ ] **Step 3: Implement.** Expose the two existing `ModifierType`s through the DSL grant surface (opponent-source filter set) gated by a `while_condition` on own security count ≥ 3, using the existing UntilCondition controller.
- [ ] **Step 4: Author** the BT16-055 high-security branch.
- [ ] **Step 5: Confirm pass.** Re-run Step 2 command — both effects blocked at 3 security, both apply at 2.
- [ ] **Step 6: Commit.** `git commit -m "feat(dsl): security-gated narrow opponent-effect protection (PUPPETS-G024)"`

### Task 17: G030 — `suppress_on_play` flag on effect-play helpers

BT5-106 `[Security]`: "play 1 level 3 purple Digimon from your trash without paying its memory cost. Any [On Play] effects on Digimon played with this effect don't activate."

**Files:**
- Modify: `code/digimon-engine/src/effect_context/mod.rs` (and/or `game.rs` play path) — a `suppress_on_play` option on effect-play helpers; carry it through the play event so On Play enqueue skips that specific permanent's On Play clauses
- Modify: `code/digimon-dsl/src/step.rs` + lowering — `suppress_on_play: true` on the play step
- Modify: `code/digimon-engine/cards/bt5/BT5-106.yaml`
- Test: `code/digimon-engine/tests/cards_behavioral/bt5/bt5_106.rs`

- [ ] **Step 1: Un-ignore** the two G030 Security tests in `bt5_106.rs`.
- [ ] **Step 2: Confirm fail.** `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- bt5_106`.
- [ ] **Step 3: Implement.** Add `suppress_on_play` to the play helper; thread it into the play event context; On Play enqueue skips only the just-played permanent for that event. Other permanents' On Play unaffected.
- [ ] **Step 4: Author BT5-106** Security clause with `suppress_on_play: true`.
- [ ] **Step 5: Confirm pass.** Re-run Step 2 command.
- [ ] **Step 6: Commit.** `git commit -m "feat(engine): suppress_on_play flag for effect-play (PUPPETS-G030)"`

---

## Final verification (after all waves)

```
cargo test --manifest-path code/digimon-dsl/Cargo.toml
cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl
cargo test --manifest-path code/digimon-engine/Cargo.toml --test option_flow
cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral
cargo test --manifest-path code/digimon-engine/Cargo.toml
DIGIMON_BACKEND=rust python -m pytest code/engine_py_legacy/tests/engine/test_rust_backend_parity.py -v
```

Expected: zero failures; the 15 PUPPETS-G target tests un-ignored and passing; no `ACTION_SPACE_SIZE` / `TENSOR_SIZE` change.

## Tracker discipline

Per PR, update:
- `qa/archetype-qa/dsl/puppets-2026-05-03-engine-dsl-gaps.md` — flip each closed `PUPPETS-G` entry's Gap Summary row **and** detailed `Status:` line to `closed`, with the PR # (the doc is currently internally inconsistent — fix both places).
- `docs/RUST_ENGINE_GAPS.md` — resolve the engine-side entries (G003, G009, G018, G024, G030); move bodies to `qa/resolved-gaps.md`.
- `qa/dsl-vocab-gaps.md` — close G010, G012, G014, G015, G020, G021, G023, G025, G028.
- `qa/qa-reports/validated_cards_dsl.json` — advance the Puppets cards (EX11-020, P-165, BT15-003, BT13-101, BT16-055, EX11-022, ST19-08, BT22-098, BT22-088, BT22-036, EX11-061, P-105, P-037, LM-035, LM-037, LM-054, EX9-032, ST19-11, BT5-106) from PARTIAL/BLOCKED toward IMPLEMENTED as their tests pass.

## Acceptance gate

Puppets is "~99% substrate-done" when all 15 target tests pass un-ignored and a `/batch-implement-cards-rust-dsl` Puppets run reports no substrate gap already filed in PUPPETS-G001..G032. Any newly-discovered gap files as `PUPPETS-G033+` and routes to a follow-up — it does not block this sweep's completion.

## Out of scope

- Pure card-authoring backlog (PUPPETS-G001) — that is Phase 3 authoring, not substrate.
- Non-Puppet consumers of shared gaps (e.g. the Dark Masters end-of-turn-deletion family that also wants G003) — this sweep closes the Puppet half; other archetypes' card authoring is separate.
- EX4-074's remaining `#[ignore]` tests — those belong to a distinct next-turn-modifier-expiry gap, not PUPPETS-G031.
