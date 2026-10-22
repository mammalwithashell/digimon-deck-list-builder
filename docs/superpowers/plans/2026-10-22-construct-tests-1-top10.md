# Construct Tests 1 — Top-10 Meta Decks Implementation Plan

> Historical plan scope below. The [App Fuse OPT followup](../research/2026-10-22-construct-tests/app-fuse-opt-followup.md) subsequently closed the Rebootmon hold; current admission is 10/10 lists and 167/167 unique IDs.

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Give every DSL construct the DigiLab-11087 top-10 decks reference a behavioural mechanics test, so the lowering-coverage light gate admits 9 of those 10 decks (all but Rebootmon, blocked by the app-fuse substrate gap) without changing the gate.

**Architecture:** A new bucket of DSL tests, `code/digimon-engine/tests/dsl/constructs/<family>.rs`, one module per scenario family, each `#[test]` driving a minimal inline fixture card through `DebugRunner` to a clause resolution and asserting positive, negative and selection-surface cases (spec §5d). Nothing in the gate, the coverage tool or production card YAML changes; the two committed artifacts (`qa/impact/lowering_coverage.json`, `data/card_admissibility.json`) are regenerated after each family and the deck report against `qa/impact/decks/digilab-11087-top10.json` says which decks flipped.

**Tech Stack:** Rust integration tests (`cargo test -p digimon-engine --test dsl`), `DebugRunner` / `DebugRunnerBuilder::from_dsl_yaml`, the `digimon-dsl` compiler, Python 3.13 tooling (`tools.lowering_coverage`, `digimon_gym.agents.card_admissibility`, `scripts/verify --tier 0`).

**Spec:** `docs/superpowers/specs/2026-10-15-engine-rules-conformance-validation-design.md` §5d (construct tests), §5b (the gate it feeds). Semantics sheets (research inputs, committed): `docs/superpowers/research/2026-10-22-construct-tests/semantics/*.md` — each task's brief points the implementer at its family's sheet for lowering sites, YAML shapes and rule citations.

**Branch:** `claude/p3-construct-tests`, rebased onto `main` @ 3b6a2d8f4 (which carries PR #742 and the mechanics gate, PR #743 via #744); the campaign PR targets `main`. Worktree `D:/wt/p3-constructs`; `CARGO_TARGET_DIR=E:/cargo-target-wt/p3-constructs`.

## Execution record (2026-10-22)

The steps below preserve the original task prescription. This table records
completion with reviewed deviations; it does not claim every proposed test or
fix landed literally.

| Task | Completed result | Commit |
|---|---|---|
| 1 | Six-family scaffold and taxonomy | `bca75ef81` |
| 2 | Security: 7 constructs, 40 family tests; receipt, destination gate and play-removal observers | `5d8d91778` |
| 3 | Play/digivolve/link: 13 constructs, 44 family tests; all 9 existing App Fuse tests ported | `89a0742d8` |
| 4 | Cost/event: 13 constructs, 27 family tests; atomic source costs and replacement-aware deletion receipts | `e75bd45c2` |
| 5 | Identity/state: 10 constructs, 58 family tests including Q3185 followup | `576a4527f`, `bc7fdce71` |
| 6 | Zone moves: 5 constructs, 25 family tests; all 7 old grouped-return tests preserved | `b2129cd6f` |
| 7 | Protection: 3 constructs, 32 family tests; DP formulas, individual source return and unpayable prevention | `5744f1cea` |
| 8 | Original 51 plus 4 parsed-only tail keys now behavioral; 10/10 scanner coverage, 9/10 actual admission | `532a97bc4`, `45b79f418` |

Task 9 is complete: full core/card runs, baseline reproduction, 10 replay
goldens, tier 0, six integrated tail tests and 37 corrected card-module tests (`81d40b77d`)
are recorded in the completion report. The broad suites retain their documented
baseline failures; the card aggregate also retains its three corrected fixture
failures. Draft PR packaging targets `main`.

The owner-authorized English exclusion fix (`2f03d4da4`) implements all 12
English exact-name/query exclusions and shared DNA/DigiXros matching.
P-217's link clause correctly excludes Creation on the printed image, so the
proposed YAML addition was withdrawn (`7cbd3b976`). Face-down `source_count`
behavior was retained after the specified DCGO check. The self-proxy audit
found 18 actual users, not the estimated 25. App Fuse's result/refund gap stays
open despite behavioral coverage of existing operations. Protection table
rows requiring own DP-payment or keyword-removal DSL primitives remain
unexpressed/unmeasured. Nonbatch plural stack protection and queued sequential
deletion observer compaction have explicit gap entries and retained failing
probe evidence; their paths are not claimed fixed. See the committed gap
trackers and [completion report](../research/2026-10-22-construct-tests/completion-report.md) for validation boundaries.

## Global Constraints

- **Vehicle:** every construct test uses a same-file `const … : &str = r#"…"#` inline fixture YAML; no `format!` placeholder may touch the construct's YAML key or a `when:` value (the coverage scanner attributes by `yaml_key` in a plain string literal, following same-file consts by name; cross-file helpers and `include_str!` are NOT followed). Production cards may be added as a second witness via `.dsl_card("<ID>")`, never as the only vehicle.
- **Real play:** the construct is reached through a clause resolution driven by `DebugRunner` (trigger fires / `[Main]` activation / attack / play) and asserted on observable state. Direct `eval_predicate_with_bindings` / `run_steps` calls are extra assertions only. A test that only inspects compiled IR is `parsed_only` and does not count.
- **Positive + negative + selection surface** per construct (spec §5d items 4–5); timings also assert the adjacent timing does NOT fire. Selections are resolved with `execute_action` on a real `pending_selection`; `auto_resolve` is never the only path through a choice. Every selection test builds at least two candidates.
- **No `#[ignore]`, no stubs.** A defect found while writing a test is fixed in the same task when it is a one-file lowering/engine fix with its own test; otherwise the construct stays untested, the gap goes to `docs/RUST_ENGINE_GAPS.md` (engine) or `qa/dsl-vocab-gaps.md` (DSL), the task report names it, and the cards stay inadmissible. Never weaken an assertion to match the engine.
- **Rules first:** every module doc comment cites the CR 4.3 rule numbers it proves (`docs/digimon-rules/cr-4.3.txt`; cite 4.3 numbers — §16 keywords shifted +1 and §15-7-4 reversed vs 3.6, see `docs/digimon-rules/changelog-3.6-to-4.3.md`), or says "card-text idiom" and names two printing cards. DCGO (`$BASE_DCGO/Assets/Scripts/CardEffect/<SET>/<COLOR>/<ID_underscore>.cs`, base repo only — never init the worktree submodule) is the tiebreak for how a card resolves.
- **Build isolation:** `CARGO_TARGET_DIR=E:/cargo-target-wt/p3-constructs` on every cargo command. Run only the `dsl` test binary while iterating (`cargo test -p digimon-engine --test dsl constructs::<family>`); the full suite template runs once at the end (Task 9). Kill stale `dsl-*.exe` before a rebuild if cargo stalls (CLAUDE.md rule 33).
- **Python env:** `PYTHONIOENCODING=utf-8 PYTHONUTF8=1 PYTHONPATH=code` on every python command.
- **Never** `git stash`; never touch `qa/dcgo-exams/P/P-092-tail-*.yaml`; subagents never push; commits end with `Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>`.
- **Out of scope:** `verb:app_fuse` (substrate; separate task), the untested constructs EX13 lists reference but the top 10 do not, any change to `card_admissibility.py` / `lowering_coverage.py` semantics, any production card YAML edit except a fix the spec's rule basis demands (then its own commit + ledger row).

## Review Focus

1. **A fixture whose `when:` timing never fires on the board the test builds** (e.g. `when_linked` on a card that is played, not linked) — the positive branch would fail or, worse, the test would only assert the negative. Each positive case asserts a state delta only the construct can produce. Pinned per family by the `…_fires_and_…` tests.
2. **A predicate test whose negative board differs from the positive one in more than the predicate's input** — proves nothing about the predicate. Each negative case changes exactly one fact (one card id, one count, one orientation). Pinned by the paired `…_holds` / `…_does_not_hold` tests in every family.
3. **A protection grant that blocks the right path but also blocks the owner's own effect or a battle deletion** — the owner's named high-risk class. Pinned by Task 7's `…_does_not_block_own_effects` / `…_does_not_block_battle` tests.
4. **A selection the construct should surface that the fixture auto-resolves because there is exactly one candidate** — hides the action-space contract. Every selection test builds ≥2 candidates and asserts `pending_action_count() >= 2` before resolving. Pinned per family.
5. **Coverage credited by token presence without behaviour** — the scanner cannot tell. Pinned by the reviewer rubric in every task review (spec §5d items 3–5) and by Task 8's check that each new construct is `tested_behaviourally`, not `parsed_only`, in the regenerated index.

---

### Task 1: Scaffold the constructs bucket, commit the deck pool, baseline the measurement

**Files:**
- Create: `code/digimon-engine/tests/dsl/constructs/mod.rs`, six empty family files (doc comment only): `security_stack.rs`, `play_digivolve_link.rs`, `cost_target_event.rs`, `identity_state.rs`, `zone_moves.rs`, `protection.rs`
- Modify: `code/digimon-engine/tests/dsl/main.rs` (add `mod constructs;`)
- Commit as staged: `qa/impact/decks/digilab-11087-top10.json`, `qa/impact/decks/README.md`, spec §5d, this plan
- Modify: `docs/RUST_DSL_TEST_API.md` §2 (fifth bucket) and decision tree
- Create: `.superpowers/sdd/2026-10-22-construct-tests/progress.md` (ledger, git-ignored; first line `# SDD ledger — plan: docs/superpowers/plans/2026-10-22-construct-tests-1-top10.md`)

**Interfaces:**
- Produces: `tests/dsl/constructs/mod.rs` with `pub mod security_stack; pub mod play_digivolve_link; pub mod cost_target_event; pub mod identity_state; pub mod zone_moves; pub mod protection;`. No shared helper module: each family keeps its own private `fn` helpers in its file (the scanner follows same-file helpers only).

- [ ] **Step 1: Add the module and the six empty family files**; `main.rs` gains `mod constructs;`.
- [ ] **Step 2: Build and list**

Run: `CARGO_TARGET_DIR=E:/cargo-target-wt/p3-constructs cargo test -p digimon-engine --test dsl constructs:: -- --list`
Expected: exit 0, 0 tests.

- [ ] **Step 3: Scanner still clean**

Run: `PYTHONIOENCODING=utf-8 PYTHONUTF8=1 PYTHONPATH=code python -m tools.lowering_coverage --check`
Expected: exit 0.

- [ ] **Step 4: Baseline report**

Run: `PYTHONIOENCODING=utf-8 PYTHONUTF8=1 PYTHONPATH=code python -m tools.lowering_coverage --report .superpowers/sdd/2026-10-22-construct-tests/baseline-report.md --decks qa/impact/decks/digilab-11087-top10.json`
Expected: `Top-10 decks: 0/10 fully lowering-covered; union of untested constructs across them: 51.`

- [ ] **Step 5: `docs/RUST_DSL_TEST_API.md` §2** — add "### Construct tests (`tests/dsl/constructs/<family>.rs`)": vehicle = inline fixture const, reached by real play, positive/negative/selection, rule citations, link to spec §5d; add the decision-tree line "Is the test about one DSL construct's lowering, for the training gate? → **construct test**."
- [ ] **Step 6: Commit**

```bash
git add code/digimon-engine/tests/dsl qa/impact/decks docs/RUST_DSL_TEST_API.md docs/superpowers/specs/2026-10-15-engine-rules-conformance-validation-design.md docs/superpowers/plans/2026-10-22-construct-tests-1-top10.md
git commit -m "test(dsl): scaffold the construct-test bucket; commit the DigiLab-11087 top-10 deck pool; spec §5d + plan"
```

---

### Task 2: Family `security_stack` — 7 constructs (decks: Jupitermon ×2, TS Toolbox, Chronomon, Rebootmon, Beelstarmon, Medusamon, Ravemon)

**Sheet:** `docs/superpowers/research/2026-10-22-construct-tests/semantics/security-stack-and-zone-verbs.md`, sections 1–7 — read its "Shared conventions" first (security `Vec` index 0 = bottom, `last()` = top; `select_security` ids `SEL_MY_SECURITY_START (40) + i` / `SEL_OPP_SECURITY_START (50) + i` counted from the bottom; memory is read from player 0's side). CR 4.3: the security stack is §3-7 (not §3-5), area rules §3-1-3.

**Files:**
- Modify: `code/digimon-engine/tests/dsl/constructs/security_stack.rs`
- Modify (fix): the `play_security_card` / `play_from_security` path (`code/digimon-engine/src/dsl_cards/step/play_digivolve.rs` and the engine helper it calls) — fire the effect-removal observer chain (`OnLoseSecurity` with `effect_initiated = true`, then `OnOwnSecurityRemoved` / `OnOpponentSecurityRemoved`) when a card is played out of security; `security_place_top_card` (`src/dsl_cards/step/zone_moves.rs` / its engine primitive) — also consult the DESTINATION player's `CannotAddSecurityByEffect`, mirroring the Venusmon round-1 check in `place_on_security`; `add_top_security_to_hand` — `bindings.record_added_to_hand(h)` like `AddToHandFromSecurity` (`zone_moves.rs:196-205`)
- Modify: `docs/RUST_ENGINE_GAPS.md`, `qa/dsl-vocab-gaps.md`

**Interfaces:**
- Consumes: Task 1's scaffold.
- Produces: fixture ids `TST-ATSH`, `TST-ATSH-OPP`, `TST-RTSD`, `TST-SPTC`, `TST-TSS`, `TST-PSC`, `TST-OLS`, `TST-TRASHSEC`, `TST-TRASHSEC-OPP`, `TST-OPPREM`, `TST-OCAS`. Private to this file. Variants that only change `position:` / `face:` / `of:` may use `format!` on those VALUES only (never on a construct key or a `when:` value).

- [ ] **Step 1: Write the failing tests:**
  - `verb:add_top_security_to_hand` — `add_top_security_to_hand_moves_the_top_card_to_its_owners_hand_without_a_prompt` (security 2→1, `SEC-BOT` remains, hand +1 == `SEC-TOP`, memory +1, `pending_selection.is_none()`), `add_top_security_to_hand_of_opponent_moves_their_top_card`, `add_top_security_to_hand_routes_to_the_card_owner_not_the_stack_owner` (set the top card's `owner = 1`; lands in P1's hand, §3-1-3-8), `add_top_security_to_hand_clears_the_face_up_record`, `add_top_security_to_hand_on_empty_stack_is_a_no_op_that_does_not_abort_the_clause` (memory still +1), `add_top_security_to_hand_records_the_added_card` (a following `if: { effect_added_any_card_to_hand: true }` rider gains memory — red until the one-line fix).
  - `verb:return_top_security_to_deck` — `return_top_security_to_deck_bottom_puts_the_card_at_deck_index_zero_and_records_the_return` (memory +1 via `effect_returned_any_card`), `return_top_security_to_deck_top_puts_it_on_the_next_draw`, `return_top_security_to_deck_routes_to_the_card_owner`, `return_top_security_to_deck_on_empty_stack_records_nothing`, `return_top_security_to_deck_fires_on_lose_security_once_with_the_effect_bit` (observer on P1 fires once; `event_is_effect_initiated` true).
  - `verb:security_place_top_card` — `security_place_top_card_moves_the_visible_top_card_face_down_to_the_bottom_and_keeps_the_stack_in_play` (security +1 at index 0, `card_sources.len() == 1`, top now `UNDER`, `is_suspended` still true per §4-7-6), `security_place_top_card_top_and_face_up_variants`, `security_place_top_card_with_a_lone_card_is_a_no_op` (§4-7-1), `security_place_top_card_blocked_by_the_acting_players_cannot_add_security_by_effect`, `security_place_top_card_blocked_by_the_destination_players_restriction` (`of: opponent` with the modifier on P1 — red until the fix).
  - `verb:trash_selected_security` — `trash_selected_security_surfaces_a_real_security_pick_over_the_opponents_stack` (`kind == Security`, `zone_owner == Some(1)`, `is_optional`, ids `[50, 51, 52]`), `trash_selected_security_trashes_the_chosen_middle_card` (execute 51 → P1 security `["SA","SC"]`, `SB` in P1 trash), `trash_selected_security_pass_changes_nothing`, `trash_selected_security_with_empty_stack_installs_no_selection`, `trash_selected_security_fires_on_discard_security_on_the_trashed_card_only` (inline-YAML `SB` with a self clause).
  - `verb:play_security_card` — `play_security_card_offers_only_filter_matches_and_plays_the_pick_free` (ids `[40]`, Tamer on the field, security 1 == `DIG`, hand unchanged, memory unchanged), `play_security_card_pass_changes_nothing`, `play_security_card_with_a_binding_that_left_the_stack_is_a_no_op`, `play_security_card_fires_on_lose_security_once` (red until the fix; §15-15-3-2, DCGO `BT13_012.cs:113`).
  - `timing:on_lose_security` — `on_lose_security_fires_once_for_an_effect_removal_with_the_effect_bit` (memory +3), `on_lose_security_fires_for_a_security_check_without_the_effect_bit` (P1 attacks P0; memory +1 relative), `on_lose_security_does_not_fire_for_the_other_players_stack` (`TST-TRASHSEC-OPP`; pins the timing's "your security" contract), `on_lose_security_does_not_fire_when_security_is_added` (`recover`), `on_lose_security_and_on_opponent_security_removed_are_disjoint` (adjacent: own-stack trash leaves `TST-OPPREM` silent; opponent-stack trash fires `TST-OPPREM` once and `TST-OLS` zero), `on_lose_security_security_scoped_self_clause_fires_once_unscoped_does_not`, `on_lose_security_optional_variant_offers_a_real_prompt`.
  - `predicate:owner_can_add_security` — `owner_can_add_security_true_without_restrictions` (memory +1; 3 candidates), `owner_can_add_security_excludes_digimon_whose_owner_cannot_add_security` (modifier on P1 → memory unchanged; 2 candidates), `owner_can_add_security_reads_the_top_cards_owner_not_the_controller` (owner-swapped X0 excluded), `owner_can_add_security_ignores_the_acting_player` (modifier on P0 only → X1 still offered), `owner_can_add_security_false_inverts`.
- [ ] **Step 2: Run** `… --test dsl constructs::security_stack`. Expected reds: the three marked tests. Any other red → diagnose against the sheet's rule basis first.
- [ ] **Step 3: Fix the three** (observer chain on play-from-security; destination-player restriction; `record_added_to_hand`). Then `cards_behavioral` for `bt13_012`, `bt15_092`, `bt26_033`, `bt24_034`, `bt25_030`, `bt26_082` (and the `security_stack_operations` effect_context binary). Green, or test edits justified by §15-15-3-2 / §15-7-4 in the commit message.
- [ ] **Step 4: Coverage flip** — regenerate; assert the 7 constructs `tested_behaviourally`; `git checkout -- qa/impact/lowering_coverage.json`.
- [ ] **Step 5: Gaps and findings** — `docs/RUST_ENGINE_GAPS.md`: (i) `G-ENGINE-ON-LOSE-SECURITY-DEFENDER-ONLY-2026-10-22` — the timing scans only the stack owner's battle area; BT23-102 Mastemon prints "security stacks" (either player's) and DCGO scans both fields (`BT23_102.cs:337`); needs an either-player variant of the timing (substrate), not a global change (the "your security" users BT24-101 / BT25-044 / BT25-088 / BT26-089 are correct today); (ii) a parked-then-cancelled would-play from security restores the card to the TOP, not its original index (`game_actions/misc.rs:1151`); (iii) `select_security` can address only the bottom 10 cards of a stack (`selections.rs:1253-1259`). `qa/dsl-vocab-gaps.md`: (iv) `security_place_top_card` and `move_self_option_under_permanent` discard their success bool, so BT26-033's `cancel_replacement` and ST23-15 / ST24-15's draw run after a failed move (§15-7-2) — a "placed" receipt predicate is needed; (v) `position: random` silently means `top` in `return_top_security_to_deck` / `return_union_bound_to_deck` (validator should reject or the verbs implement it). Task report: BT13-012 / BT15-092 skip "Then, shuffle your security stack" on a declined optional pick (`ResumeDecline::None`) — a card-authoring question (`continue_on_decline`) for the controller.
- [ ] **Step 6: Commit**

```bash
git add code/digimon-engine/tests/dsl/constructs/security_stack.rs code/digimon-engine/src code/digimon-engine/tests/cards_behavioral docs/RUST_ENGINE_GAPS.md qa/dsl-vocab-gaps.md
git commit -m "test(dsl): construct tests for the security-stack family (7 constructs); playing a card out of security fires the removal observers (§15-15-3-2); top-card-to-security honours the destination player's restriction"
```

---

### Task 3: Family `play_digivolve_link` — 13 constructs (decks: Jupitermon ×2, TS Toolbox, Chronomon, Examon, Toho Braves, Ravemon, Beelstarmon, Rebootmon)

**Sheet:** `docs/superpowers/research/2026-10-22-construct-tests/semantics/play-digivolve-and-link.md` — read it first; it has the lowering sites, exact YAML shapes, the shared harness facts (how to link by player action, by hand, by effect API; `link_requirement`, `give_link_plus`, `push_linked_owned`) and every fixture below in full.

**Files:**
- Modify: `code/digimon-engine/tests/dsl/constructs/play_digivolve_link.rs` (all tests of this family; private helpers and fixture consts in-file)
- Modify (fix): `code/digimon-engine/src/game/suspend.rs:281` (hatch drain), `code/digimon-engine/src/dsl_cards/lower_triggered.rs` (`body_first_step_is_declinable` + `first_step_candidate_guard` arms for `MayDnaDigivolveNow`)
- Modify (card data, own commit): `code/digimon-engine/cards/p/P-217.yaml` (add `event_card_trait_has: Creation` to the `on_any_link` qualifier — verify against the card image first) + its `tests/cards_behavioral/p/p_217.rs` if present
- Modify: `docs/RUST_ENGINE_GAPS.md` (new entries, see Step 6), `docs/RUST_ENGINE_API.md:2645` (stale "follow-up resume slot" sentence)

**Interfaces:**
- Consumes: Task 1's module scaffold. Nothing from other families.
- Produces: fixture ids `TEST-POU-HAND`, `TEST-MAY-DNA`, `TEST-CDFS`, `TEST-HATCH`, `TEST-HATCH-OBS`, `TEST-ARM`, `TEST-LINKER`, `TEST-LINKER-2`, `TEST-HOST-OBS`, `TEST-ANYLINK`, `TEST-ANYLINK-RAW`, `TEST-WLINK-HOST`, `TEST-LCC`, `TEST-RLC-OUTSIDE`, `TEST-WLT-OUTSIDE`, and the app-fuse set ported from `tests/cards_behavioral/app_fuse_primitive.rs`. Private to this file.

- [ ] **Step 1: Write the failing tests, one `#[test]` per case below, in this order** (names are the contract the reviewer checks; assertions per the sheet's "Proposed behavioural test"):
  - `verb:play_or_use_from_hand` — `play_or_use_from_hand_plays_bound_digimon_with_reduced_cost` (battle area +1, memory −2), `play_or_use_from_hand_uses_bound_option_with_reduced_cost` (option in trash, `[Main]` draw ran, memory −2), `play_or_use_from_hand_without_cost_delta_pays_full_cost` (memory −5; pins the inverse of the `_from_sources` sibling), `play_or_use_from_hand_declined_select_hand_changes_nothing`, `play_or_use_from_hand_dual_surfaces_face_choice` (`EffectChoice` labels `["Play as Digimon", "Use as Option"]`, `is_optional == false`, `pending_selection_resume.is_some()`).
  - `verb:may_dna_digivolve_now` — `may_dna_partner_pick_offers_only_pairs_that_reach_a_hand_card` (anchor and off-colour excluded, ≥2 own Digimon on board), `may_dna_result_pick_then_commit_pays_printed_dna_cost` (memory −2, result on top, anchor + partner in sources), `may_dna_with_no_reachable_target_installs_nothing`, `may_dna_ignore_requirements_pays_authored_cost`, `may_dna_optional_pass_leaves_board_unchanged`, `may_dna_optional_clause_offers_one_prompt_and_none_when_ineligible` (the fix in Step 3: a clause `optional: true` led by this step shows no outer prompt when no pair exists, and exactly one PASS-able prompt when one does).
  - `predicate:can_digivolve_from_source` — `can_digivolve_from_source_offers_only_cards_with_a_route_from_this_permanent` (exactly `{ROUTE-OK}`), `can_digivolve_from_source_false_inverts_the_set`, `can_digivolve_from_source_with_cannot_digivolve_installs_no_selection`.
  - `verb:hatch` — `hatch_moves_top_egg_to_breeding_and_fires_on_hatch_after_the_body` (memory == 5, not 6 — §15-8-3-2 ordering; this fails until Step 3), `hatch_with_occupied_breeding_is_a_no_op`, `hatch_with_empty_egg_deck_is_a_no_op`, `hatch_of_opponent_fills_their_breeding_area`.
  - `verb:arm_digivolve_cost_reducer` — `armed_reducer_prompts_suspend_and_reduces_next_matching_digivolve` (optional `EffectChoice` → mandatory `OwnField` with ≥2 candidates → memory 9, reducer list empty), `armed_reducer_ignores_non_matching_colour_and_stays_armed`, `armed_reducer_declined_is_consumed_when_single_fire`, `armed_reducer_expires_at_end_of_turn`, `armed_reducer_without_suspend_cost_applies_silently`, `armed_reducer_multi_fire_reduces_twice`.
  - `timing:when_linked` — `when_linked_fires_once_when_this_card_becomes_a_link_card`, `when_linked_does_not_fire_on_play` (adjacent), `when_linked_does_not_refire_when_a_sibling_links_to_the_same_host`, `when_linked_fires_for_effect_api_links`.
  - `timing:when_card_linked_to_this` — `when_card_linked_to_this_fires_on_the_host_only` (+3 not +6), `when_card_linked_to_this_does_not_fire_when_this_becomes_the_link_card` (adjacent), `when_card_linked_to_this_does_not_fire_on_play`.
  - `timing:on_any_link` — `on_any_link_fires_for_own_link_with_owner_qualifier`, `on_any_link_without_qualifier_sees_opponent_links`, `on_any_link_observer_on_opponent_side_fires_for_our_link`, `on_any_link_does_not_fire_on_play_or_digivolve` (adjacent).
  - `timing:when_would_link_to_this` + `verb:reduce_link_cost` + `predicate:would_link_card_trait_any_of` — `would_link_window_opens_after_host_pick_before_payment` (optional prompt, memory unchanged, no link yet; `REPLACEMENT_ACCEPT` → memory 4, linked), `would_link_declined_pays_full_cost_and_still_links`, `would_link_does_not_open_for_a_different_host`, `would_link_does_not_reoffer_after_the_link` (adjacent), `reduce_link_cost_saturates_at_zero`, `reduce_link_cost_does_not_leak_to_the_next_link`, `reduce_link_cost_outside_the_window_is_a_no_op`, `would_link_card_trait_any_of_gates_by_linker_trait` (Social and Tool prompt; Beast no prompt), `would_link_card_trait_any_of_is_false_outside_the_window`, `would_link_card_trait_match_is_case_sensitive` (pins today's exact match deliberately).
  - `predicate:link_card_count` — `link_card_count_filters_hosts_by_matching_link_cards` (exactly H2), `link_card_count_unreachable_threshold_installs_no_selection`, `link_card_count_empty_filter_counts_all_link_cards`.
  - `verb:app_fuse` — port the eight tests of `tests/cards_behavioral/app_fuse_primitive.rs` verbatim into this module (rename with an `app_fuse_` prefix; they already use inline YAML and `DebugRunner`), plus `app_fuse_pass_at_either_pick_leaves_host_and_hand_unchanged`. **Do not** write a test asserting the `[Once Per Turn]` refund after a PASS: that is the known substrate gap (BT25-089), out of scope.
- [ ] **Step 2: Run and confirm the expected reds** — `CARGO_TARGET_DIR=E:/cargo-target-wt/p3-constructs cargo test -p digimon-engine --test dsl constructs::play_digivolve_link`. Expected: every test compiles; `hatch_…_after_the_body` and `may_dna_optional_clause_offers_one_prompt_…` fail (the two defects), everything else passes. Any other red is a finding: diagnose against the sheet's rule basis before touching the test.
- [ ] **Step 3: Fix the two defects** — (a) `suspend.rs:281`: `drain_effect_queue()` → `maybe_drain_effect_queue()` (the deferred-drain rule, `effect_queue.rs:1280-1290`); (b) `lower_triggered.rs`: add `CompiledStep::MayDnaDigivolveNow` to `body_first_step_is_declinable` and a `first_step_candidate_guard` arm that is false when no (partner, hand card) pair satisfies a DNA requirement (reuse the step's own eligibility code; do not duplicate it). Run the family again: all green. Then run the cards that carry an optional may-DNA clause: `cargo test … --test cards_behavioral bt12_021 ex13_008 ex12_001` and `cards_behavioral p_123` (hatch) — Expected: green, or test edits that match the new single-prompt flow with the sheet's §8 / §15-8-3-2 citations in the commit message.
- [x] **Step 4: P-217 printed-text verification — no card edit.** The official cached card face and `data/card_bundles/P-217.md` show `[Creation]` only in the On Play clause; the Your Turn link clause lists Social/Navi/Tool. Existing YAML is correct. The proposed addition was based on lossy ingested text and is withdrawn. Retain the P-217 regression run with Task 3.
- [ ] **Step 5: Coverage flip** — `PYTHONIOENCODING=utf-8 PYTHONUTF8=1 PYTHONPATH=code python -m tools.lowering_coverage` then assert in the regenerated JSON that all 13 constructs of this family are `tested_behaviourally: true` (a one-line python check). Do not commit the artifact here (Task 8 does); `git checkout -- qa/impact/lowering_coverage.json` after checking.
- [ ] **Step 6: Gaps and docs** — `docs/RUST_ENGINE_GAPS.md`: (i) `G-ENGINE-WOULD-LINK-WINDOW-EFFECT-LINKS-2026-10-22` — `link_cards` effect links and Plug-In Option links never open `WhenWouldLink` / set `pending_digimon_link`, so BT25-004 / BT25-045 cannot reduce BT25-089's `[Main]` link (sheet, `when_would_link_to_this` defect 1; blocks training: Rebootmon only, already blocked); (ii) `G-ENGINE-APP-FUSE-PROCEDURE-2026-10-22` — effect App Fuse draws no card (§8-4-3-3), assumes cost 0, consumes every link card (sheet, `app_fuse` defects 1–3; needs a DCGO `SetAppFusion` check + owner ruling on the link-card scope); (iii) note under the existing BT25-089 app-fuse entry where the result signal would be produced (sheet, "KNOWN substrate gap", three loss sites). `qa/dsl-vocab-gaps.md`: `link_card_count` on a card subject is silently ignored — validator should reject (no production user). Fix the stale sentence at `docs/RUST_ENGINE_API.md:2645`.
- [ ] **Step 7: Commit** (tests + fixes in one commit; P-217 separately; docs with the tests)

```bash
git add code/digimon-engine/tests/dsl/constructs/play_digivolve_link.rs code/digimon-engine/src/game/suspend.rs code/digimon-engine/src/dsl_cards/lower_triggered.rs code/digimon-engine/tests/cards_behavioral docs/RUST_ENGINE_GAPS.md docs/RUST_ENGINE_API.md qa/dsl-vocab-gaps.md
git commit -m "test(dsl): construct tests for the play/digivolve/link family (13 constructs); hatch defers its observer drain (§15-8-3-2); optional may_dna_digivolve_now is a single guarded prompt"
```

**Controller rulings carried into this task (owner may override):** `play_or_use_from_hand` ignoring `of` and not recording the played handle are documented, not fixed (no production consumer); `would_link_card_trait_any_of` stays case-sensitive and the test pins it; whether the armed reducer applies to DNA / App Fusion (§8-2-2-5, §8-4-2-3) is reported, not tested.

---

### Task 4: Family `cost_target_event` — 13 constructs (decks: Beelstarmon, Chronomon, Toho Braves, Medusamon, TS Toolbox, Ravemon, Examon)

**Sheet:** `docs/superpowers/research/2026-10-22-construct-tests/semantics/cost-target-and-event-predicates.md` — read it first. Two corrections it makes to common assumptions: "cost target" is the card whose play/digivolve/use cost is being calculated (§15-8-5-1 "would digivolve into" reducers), not a "By doing X" cost; the attack flow is CR 4.3 §11 (§13 is Security Checks).

**Files:**
- Modify: `code/digimon-engine/tests/dsl/constructs/cost_target_event.rs`
- Modify (fix): `code/digimon-engine/src/dsl_cards/step/permanent_mutations.rs` (`delete_bound_permanents` :135-161, `delete_for_cost_reduction` :93-127 — record only completed deletions, via the receipt path `deletion_results.rs` uses), `code/digimon-engine/src/effect_context/selections.rs` (~:2929-2962, :3223-3226) and `code/digimon-engine/src/dsl_cards/step/zone_moves.rs` (`trash_selected_sources_as_cost`: all-or-nothing, and no pick offered when eligible < `min`)
- Modify: `docs/RUST_ENGINE_GAPS.md`, ledger report (findings for the owner, Step 6)

**Interfaces:**
- Consumes: Task 1's scaffold. Nothing from other families (the `Evade` saves use `grant_keyword` directly).
- Produces: fixture ids `TST-CT-REDUCER`, `TST-CT-BASE`, `TST-CT-EVO-PROBE`, `TST-CT-EVO-PLAIN`, `TST-SICTP-BASE`, `TST-TSSC`, `TST-BCE`, `TST-BCK`, `TST-NNIB`, `TST-EDEL-OWN`, `TST-EDEL-OPP`, `TST-ADD-HOST`, `TST-ADD-OBS`, `TST-ADD-BASE`, `TST-ADD-EVO`, `TST-ETNC`, `TST-DURATK`, `TST-SOOMP`. Private to this file.

- [ ] **Step 1: Write the failing tests, one `#[test]` per case:**
  - `predicate:cost_target` — `cost_target_reducer_applies_when_the_digivolving_card_matches` (memory 5→3, no prompt), `cost_target_reducer_does_not_apply_to_a_non_matching_card` (5→2), `cost_target_reads_play_costs_too` (with/without reducer).
  - `predicate:source_is_cost_target_permanent` — `source_is_cost_target_permanent_reduces_only_this_permanents_digivolve` (−2 on index 0, −3 on index 1), `source_is_cost_target_permanent_is_false_for_a_play`, `source_is_cost_target_permanent_inherited_variant_follows_the_stack`.
  - `verb:trash_selected_sources_as_cost` — `trash_sources_as_cost_surfaces_source_multi_with_filtered_candidates` (`SourceMulti{min:0,max:2}`, `is_optional`, 3 ids = 2 FUEL + PASS), `trash_sources_as_cost_full_payment_runs_inner_and_outer_tails` (stack 4→2, trash +2, hand +1, memory 1), `trash_sources_as_cost_short_pick_aborts_both_tails` (§15-7-3), `trash_sources_as_cost_decline_aborts_both_tails`, `trash_sources_as_cost_two_cards_is_one_trash_event` (`on_digivolution_card_trashed` observer fires once, §15-5-2), `trash_sources_as_cost_is_all_or_nothing_when_one_pick_is_protected` (fails until Step 3: give one FUEL `ImmuneFromStackTrashing`-style protection via the modifier registry; after a 2-pick, either both or neither are trashed — §15-7-3), `trash_sources_as_cost_offers_no_pick_when_eligible_below_min` (fails until Step 3: `min: 2` with one eligible source installs no selection, §15-7-4).
  - `predicate:binding_count_eq` — `binding_count_eq_counts_single_and_list_bindings` (hand +1 for n:1; +2 memory for a 2-pick), `binding_count_eq_absent_binding_counts_zero` (PASS → memory +1 via `n: 0`), `binding_count_eq_one_of_two_is_not_two`.
  - `predicate:binding_card_kind` — `binding_card_kind_option_digimon_dual_tamer_table` (memory 1 / 2 / 3 / 0 per the sheet's table — four runners or one parametrised helper), `binding_card_kind_multi_card_list_fails_closed`.
  - `predicate:name_not_in_binding` — `name_not_in_binding_excludes_revealed_names_and_aliases` (exactly one valid action: Patamon; Koromon-treated-as-Gabumon excluded; mandatory), `name_not_in_binding_with_no_clash_offers_all`.
  - `predicate:effect_deleted_any_own_digimon` — `effect_deleted_own_digimon_true_after_this_effect_deletes_one` (mandatory 1-candidate pick is still a real pick), `effect_deleted_own_digimon_counts_tokens`, `effect_deleted_own_digimon_false_for_a_deleted_tamer`, `effect_deleted_own_digimon_false_when_evade_prevents_it` (opponent-side Evade prompt accepted; Digimon suspended, memory 0), `effect_deleted_own_digimon_false_when_an_opponent_digimon_was_deleted`.
  - `predicate:effect_deleted_any_opponent_digimon` — `effect_deleted_opponent_digimon_true_sets_memory_not_own_rider` (memory 1 not 6), `effect_deleted_opponent_digimon_false_when_declined_draws`, `effect_deleted_opponent_digimon_false_when_evade_prevents_it`, `effect_deleted_opponent_digimon_counts_tokens`, `effect_deleted_bound_permanents_does_not_count_a_prevented_deletion` (fails until Step 3: same fixture using `delete_bound_permanents` over a 2-target list where one has Evade), `delete_for_cost_reduction_does_not_credit_a_prevented_deletion` (fails until Step 3; §15-7-2).
  - `predicate:event_added_card_any` + `timing:on_add_digivolution_cards` — `on_add_digivolution_cards_fires_once_per_effect_batch` (two placements, memory +1 not +2), `event_added_card_any_matches_any_card_in_the_batch` (OPT present → fires; two DIGI → not), `event_added_card_any_is_false_on_other_timings`, `on_add_digivolution_cards_does_not_fire_for_normal_digivolution` (adjacent; memory exactly 5−2), `on_add_digivolution_cards_does_not_fire_when_a_source_is_trashed` (adjacent), `on_add_digivolution_cards_observer_resolves_after_the_placing_effect` (+10 then +1 ordering), `on_add_digivolution_cards_reaches_a_non_host_observer_once`.
  - `predicate:event_target_name_contains` — `event_target_name_contains_matches_case_insensitively_on_suspend` (Wingdramon fires; Agumon not), `event_target_name_contains_applies_the_indramon_exclusion` (pins the one exclusion that ships), `event_target_name_contains_reads_the_deleted_cards_last_names` (`on_any_deletion` + Sukamon). **Do not** assert BurningGreymon / KendoGarurumon / BeelStarmon either way (owner decision pending, Step 6).
  - `predicate:during_attack` — `during_attack_true_from_when_attacking_through_end_of_attack` (memory 1, hand +1, `pending_attack.is_none()` after), `during_attack_false_outside_an_attack` (OnPlay → memory +2), `during_attack_false_in_an_effect_battle`.
  - `timing:start_of_opponents_main_phase` — `start_of_opponents_main_phase_fires_for_the_non_turn_players_battle_area_only` (P1 hand +1 exactly; breeding copy silent; P0 copy silent), `start_of_opponents_main_phase_swaps_with_the_turn` (P0's copy fires on P1's turn), `start_of_opponents_main_phase_shares_the_pool_with_start_of_your_main_phase` (two P1 triggers → `TriggerOrder` surfaced to P1; pattern `tests/timing_dispatch.rs:387-455`).
- [ ] **Step 2: Run and confirm the expected reds** — `… --test dsl constructs::cost_target_event`. Expected reds: the two `trash_sources_as_cost_…` fixes and the two legacy-deletion-path tests. Everything else green; any other red is a finding to diagnose against the sheet first.
- [ ] **Step 3: Fix** — (a) `permanent_mutations.rs`: `delete_bound_permanents` and `delete_for_cost_reduction` record a deletion (and credit a cost) only for permanents the batch actually trashed (use the same receipt `delete_permanent` / `delete_permanents` use in `deletion_results.rs`; §15-7-2). (b) `trash_selected_sources_as_cost`: check every picked source is trashable before moving any (abort with no movement otherwise — §15-7-3), and the `select_own_sources … then:` installer does not install the pick when eligible candidates < `min` (§15-7-4; the clause then aborts as a decline). If (b) needs more than `selections.rs` + `zone_moves.rs`, stop, file `G-ENGINE-SOURCE-COST-PARTIAL-PAYMENT-2026-10-22` instead, and drop the two tests to the ledger as parked with the gap id (the construct still gets credit from its other five tests). Run the family green, then `cards_behavioral` for the 14 `trash_selected_sources_as_cost` users (`ex10_` prefix set in the sheet) and for `bt25_014`, `bt26_073`, `bt26_078`, `bt26_082`, `bt4_086`, `ex3_057`, `bt24_075`, `ex7_071` (deletion-rider users).
- [ ] **Step 4: Optional Decoy receipt case** — if a `<Decoy>` grant is one `grant_keyword` call on a bystander, add `effect_deleted_opponent_digimon_does_not_count_a_decoy_substitute` (§16-18: the substitute's deletion belongs to the Decoy effect; the target was not deleted → leaf false). Skip with a ledger note if it needs a fixture with a printed Decoy.
- [ ] **Step 5: Coverage flip** — regenerate, assert all 13 constructs `tested_behaviourally`, then `git checkout -- qa/impact/lowering_coverage.json`.
- [ ] **Step 6: Findings for the owner (report, do not fix)** — in the task report: (i) **Language Standardization exclusions**: `rules_name_contains` implements only Indramon≠[Dramon]; BurningGreymon reads as [Greymon] for EX4-061 / EX13-067 (Examon deck). The owner must decide whether to implement Bandai's full exclusion table (memory: `reference_name_aliases_and_dcgo_name_exclusions`). (ii) **EX13-045 DUAL**: `binding_card_kind: option` matches a DUAL, so the Examon card's "if Option use it, else play" never offers the DUAL face choice — read DCGO `EX13_045.cs` and report whether DCGO offers a choice. (iii) **BT25-014** inherited `[When Attacking]` carries `once_per_turn: true` but the printed text has no `[Once Per Turn]` — read the card image and report. (iv) Whether a `<Decoy>` substitute's deletion lands in the original effect's receipt, if Step 4 was skipped.
- [ ] **Step 7: Commit**

```bash
git add code/digimon-engine/tests/dsl/constructs/cost_target_event.rs code/digimon-engine/src/dsl_cards/step/permanent_mutations.rs code/digimon-engine/src/effect_context/selections.rs code/digimon-engine/src/dsl_cards/step/zone_moves.rs code/digimon-engine/tests/cards_behavioral docs/RUST_ENGINE_GAPS.md
git commit -m "test(dsl): construct tests for the cost-target/event family (13 constructs); prevented deletions are not recorded as deleted (§15-7-2); source-trash costs are all-or-nothing and never offered unpayable (§15-7-3/-4)"
```

---

### Task 5: Family `identity_state` — 10 constructs (decks: all ten; `card_number_is` alone blocks seven)

**Sheet:** `docs/superpowers/research/2026-10-22-construct-tests/semantics/identity-state-predicates-and-protection.md`, "Read this first" + sections 1–10. Note its point 2: the predicate SUBJECT (None / Card / Permanent / Source) decides which leaves are evaluated, and several permanent-only leaves are today silently skipped (vacuously true) on `None` and `Card` subjects.

**Files:**
- Modify: `code/digimon-engine/tests/dsl/constructs/identity_state.rs`
- Modify (fix): `code/digimon-engine/src/dsl_cards/predicate.rs` — (a) `source_count`, `can_attack`, `can_change_orientation`, `link_card_count` evaluate to **false** on `None` / `Card` subjects (the way `has_face_down_source` already does at `:962-972`), instead of being skipped; (b) `on_field` re-validates that the slot still holds the source card (`:774-778`); (c) `source_count` skips face-down sources (§4-7-9) **unless** DCGO counts them for a production user — the implementer reads the DCGO C# of BT25-085 (or another `source_count` user) first and reports; `code/digimon-engine/src/dsl_cards/lower_replacement.rs:448-497` — `predicate_reads_replacement_subject` also recognises `play_cost` (`eq` / `gte`), `trait_contains`, `self_color_count_gte`
- Modify: `docs/RUST_ENGINE_GAPS.md`, `qa/dsl-vocab-gaps.md`

**Interfaces:**
- Consumes: Task 1's scaffold.
- Produces: fixture ids `TST-CNUM`, `TST-NAMEIN`, `TST-ALIAS`, `TST-TRAITC`, `TST-PCOST`, `TST-SRCCOUNT`, `TST-COLORS`, `TST-FDTAMER`, `TST-ONFIELD`, `TST-CANATK`, `TST-ORIENT`. Private to this file.

- [ ] **Step 1: Write the failing tests:**
  - `predicate:card_number_is` — `card_number_is_offers_exactly_the_matching_printings` (2 of 4 hand cards; same-name different-number and prefix collision excluded), `card_number_is_gates_any_permanent_on_our_side_only`, `card_number_is_is_case_sensitive` (pins today's behaviour deliberately), `card_number_is_single_candidate_is_still_a_real_pick`.
  - `predicate:name_in` — `name_in_matches_exact_names_and_also_treated_as_aliases` (3 of 6 hand cards: Veemon, Wormmon, `TST-ALIAS`), `name_in_rejects_substrings_and_case_variants` (ExVeemon, DemiVeemon, "veemon"), `name_in_on_a_permanent_reads_the_overlay_name` (`ChangeBaseCardName` → Wormmon counts; → Gabumon fails Agumon), `name_in_gates_any_permanent`.
  - `predicate:trait_contains` — `trait_contains_matches_substrings_case_insensitively` (Dinosaur, Plesiosaur, Saurian; Dragonkin and no-trait excluded), `trait_contains_reads_granted_traits_on_permanents` (`ChangeTraits add Aquatic`), `trait_contains_versus_trait_has_contrast` (`trait_has: Dinosaur` vs `trait_contains: Dino`).
  - `predicate:play_cost` — `play_cost_eq_offers_cost_three_digimon_and_option`, `play_cost_range_gte_lte` (3 of 5), `play_cost_unmet_installs_no_prompt`, `play_cost_on_a_permanent_reads_the_top_card_not_the_sum`, `replacement_active_when_with_play_cost_gte_reads_the_subject` (red until the `lower_replacement.rs` fix: a replacement whose `active_when` uses only `play_cost: {op: gte}` must apply to another permanent, not just the carrier).
  - `predicate:source_count` — `source_count_counts_sources_below_the_top_card_matching_the_filter` (A offered; B and C not), `source_count_at_least_zero_is_always_true`, `source_count_reads_a_dual_source_as_an_option`, `source_count_does_not_count_face_down_sources` (red until fix (c), or dropped with the DCGO citation if DCGO counts them — then the sheet's D row is pinned as today's behaviour with the citation), `source_count_in_an_if_without_a_permanent_subject_is_false` (red until fix (a)).
  - `predicate:self_color_count_gte` — `self_color_count_gte_on_hand_cards_counts_printed_colours` (2 of 3), `self_color_count_gte_on_a_permanent_counts_effective_colours` (AddColor from sources → aura DP 5000), `self_color_count_gte_ignores_face_down_source_colours` (DP stays 4000), `self_color_count_gte_deduplicates`.
  - `predicate:face_down_sources_under_tamers_gte` — `face_down_under_tamers_counts_across_tamers`, `face_down_under_tamers_counts_two_under_one_tamer`, `face_down_under_tamers_ignores_face_up_digimon_sources_and_opponent_tamers` (three negatives, one fact each), `face_down_under_tamers_stops_at_the_first_protected_card_from_the_bottom` (bottom protected → 0; upper protected → 1; owner ruling 19), `face_down_under_tamers_ignores_an_opponent_only_immunity_for_our_own_effect`.
  - `predicate:on_field` — `on_field_true_in_the_battle_area_and_the_if_reads_the_source` (memory +1 exactly), `on_field_false_from_the_breeding_area` (direct eval on the breeding sentinel handle, as the sheet describes — allowed as an extra assertion beside the YAML fixture), `on_field_false_for_an_option_used_from_hand`, `on_field_false_after_the_source_left_the_field` (red until fix (b)).
  - `predicate:can_attack` — one table-driven test per row of the sheet's 13-row table INCLUDING row 8 (`CannotSuspend` + `without_suspending`: owner authorized the official Q3185 exception on 2026-10-22; separate followup commit), named `can_attack_row_<n>_<slug>`; plus `can_attack_in_an_if_without_a_permanent_subject_is_false` (red until fix (a)). Every "offered" row asserts `is_optional == true` and a real pick with one candidate.
  - `predicate:can_change_orientation` — `can_change_orientation_partitions_the_board` ("can" = U, S, ST, carrier; "cannot" = US, SU, SP), `can_change_orientation_excludes_breeding`, `can_change_orientation_all_cannot_installs_no_prompt`, `can_change_orientation_in_an_if_without_a_permanent_subject_is_false` (red until fix (a)).
- [ ] **Step 2: Run** `… --test dsl constructs::identity_state`. Expected reds: the five fix-gated tests. Any other red → diagnose first.
- [ ] **Step 3: Fix (a) (b) (c) + the `lower_replacement.rs` list.** Before (a), grep `qa/impact/dsl_impact_index.json` for production cards that use `source_count` / `can_attack` / `can_change_orientation` / `link_card_count` inside an `if:` condition or a hand/trash/source filter; if any relies on the vacuous true, leave that leaf's behaviour, report the card, and drop its test to the ledger. Then `cards_behavioral` for every user of the four leaves (from the impact index) and for the replacement cards whose `active_when` now reads the subject (grep `play_cost`/`trait_contains`/`self_color_count_gte` in `kind: replacement` clauses) — Expected: green, or test edits with the §4-7-9 / §15-8-5 citation.
- [ ] **Step 4: Coverage flip** — regenerate; assert the 10 constructs `tested_behaviourally`; `git checkout -- qa/impact/lowering_coverage.json`.
- [ ] **Step 5: Gaps and findings** — `qa/dsl-vocab-gaps.md`: (i) **`G-DSL-CARD-NUMBER-SELF-PROXY-2026-10-22`** — about 25 cards write `any_permanent { card_number_is: <own id>, is_unsuspended: true }` to mean "this card is unsuspended"; with a second unsuspended copy on the field a suspended carrier passes the gate and the `suspend: {target: source}` cost no-ops while the body runs (Homeros BT24-102, P-130, BT21-081; §15-7-2 / §15-7-4). The fix is a card-data sweep to `source_is_unsuspended: true` plus a `dsl-lint` rule — its own task; list the 25 cards in the entry. (ii) `on_field` means battle area, not the CR field (§3-4-6) — document in the vocabulary doc, do not rename. (iii) App-Fusion `name_in` (`card_name_equals`) ignores `also_treated_as` aliases and is case-insensitive, unlike the predicate path. `docs/RUST_ENGINE_GAPS.md`: (iv) `can_attack` does not consult §11-2-4 (no declaration during an attack); (v) Digi-Eggs read `play_cost` 0 (cards.json default) so `play_cost: {op: lte}` matches them; (vi) the §2-3-4-5 card-number alias is not modelled (RB1-004/006/007). Task report (owner decisions): **can_attack row 8** — official Q&A Q3185 (BT19-101) and DCGO allow a `CannotSuspend` Digimon to attack via an attack-without-suspending effect; the engine refuses it citing §11-2-5. Owner resolved 2026-10-22: official Q3185 governs; implement the exception and row 8 in a separate followup commit. Also ST24-10's extra "[DATA SQUAD] in hand" gate removes a legal choice (§15-7-5).
- [ ] **Step 6: Commit**

```bash
git add code/digimon-engine/tests/dsl/constructs/identity_state.rs code/digimon-engine/src/dsl_cards/predicate.rs code/digimon-engine/src/dsl_cards/lower_replacement.rs code/digimon-engine/tests/cards_behavioral docs/RUST_ENGINE_GAPS.md qa/dsl-vocab-gaps.md
git commit -m "test(dsl): construct tests for the identity/state predicate family (10 constructs); permanent-only leaves are false without a permanent subject; on_field re-validates its slot; replacement active_when recognises every cost/identity leaf"
```

---

### Task 6: Family `zone_moves` — 5 constructs (decks: TS Toolbox, Chronomon, Examon, Ravemon, Beelstarmon)

**Sheet:** `docs/superpowers/research/2026-10-22-construct-tests/semantics/security-stack-and-zone-verbs.md`, sections 8–12.

**Files:**
- Modify: `code/digimon-engine/tests/dsl/constructs/zone_moves.rs`
- Move: `code/digimon-engine/tests/grouped_return_contract.rs` → folded into `zone_moves.rs` (its `GROUP-CONTRACT` fixture and `act` / `fixture` helpers are already a complete behavioural `return_all_to_deck` suite; the top-level file is deleted so the nightly binary-rot glob stops running it twice)
- Modify (fix): `code/digimon-engine/src/game_actions/option_lifecycle.rs:407-471` (`move_field_option_under_permanent`: resolve the target by stable identity captured BEFORE the Option leaves the battle area)
- Modify: `docs/RUST_ENGINE_GAPS.md`

**Interfaces:**
- Consumes: Task 1's scaffold.
- Produces: fixture ids `TST-SZC-BUDGET`, `TST-SZC-COUNT`, `GROUP-CONTRACT`, `TST-RUBD`, `TST-MSO`, `TST-DELS`, `TST-DEL-SEQ`, `TST-DEL-OBS`. Private to this file.

- [ ] **Step 1: Write the failing tests:**
  - `verb:select_zone_cards` — `select_zone_cards_budget_prompt_offers_only_affordable_matches_and_shrinks` (`PlayCostBudget{remaining 5}` with P2/P3/P4 + PASS, Q1 absent; after P3 → remaining 2, only P2 + PASS; after P2 → no prompt, memory +1), `select_zone_cards_pass_leaves_the_binding_unbound`, `select_zone_cards_exact_count_with_optional_zero` (`CountCappedMultiSelect{min 2,max 2}` PASS present → after 1 pick no PASS → after 2 picks trash −2, deck +2 first pick deepest), `select_zone_cards_fewer_candidates_than_min_installs_no_prompt`, `select_zone_cards_min_one_without_optional_zero_is_mandatory`, `select_zone_cards_never_auto_picks_a_sole_remaining_option`.
  - `verb:return_all_to_deck` — the ported tests, renamed `return_all_to_deck_single_member_returns_without_a_prompt`, `return_all_to_deck_orders_two_members_with_a_mandatory_pick` (first pick = bottommost for `bottom`; top variant top-down), `return_all_to_deck_leaves_non_matching_and_wrong_owner_permanents` , `return_all_to_deck_skips_effect_immune_targets`, `return_all_to_deck_include_sources_variant`, `return_all_to_deck_is_one_event_for_observers` (add: an `on_any_deletion`-style return observer fires once, §15-5-2).
  - `verb:return_union_bound_to_deck` — `return_union_bound_to_deck_from_hand_to_top_records_the_return` (prompt offers hand + trash ids; `deck.last()` is PH, memory +1), `return_union_bound_to_deck_from_trash_to_bottom`, `return_union_bound_to_deck_sequence_pick_order_is_deck_order`, `return_union_bound_to_deck_pass_moves_nothing`, `return_union_bound_to_deck_is_not_a_discard` (`on_discard_hand` observer silent).
  - `verb:move_self_option_under_permanent` — `move_self_option_under_tamer_face_down_and_continues` (battle area 2→1, Tamer `card_sources[0]` is `TST-MSO` face-down, not in trash, memory +1), `move_self_option_under_tamer_face_up_variant`, `move_self_option_when_the_option_precedes_the_tamer_still_lands_under_the_tamer` (red until the fix — the index-shift defect; add a third permanent after the Tamer to show the wrong-host variant), `move_self_option_pass_leaves_it_on_the_field`, `move_self_option_from_a_non_field_option_is_a_no_op`.
  - `verb:delete_permanents` — `delete_permanents_deletes_separately_bound_targets_as_one_batch` (two mandatory picks; nothing deleted between them; witness fires ONCE → memory −1), `delete_permanent_twice_is_two_events_by_contrast` (`TST-DEL-SEQ` → −2), `delete_permanents_ignores_an_unset_binding`, `delete_permanents_collapses_duplicate_bindings`, `delete_permanents_skips_immune_targets`, `delete_permanents_adds_no_prompt_of_its_own`.
- [ ] **Step 2: Run** `… --test dsl constructs::zone_moves`. Expected red: the index-shift test only.
- [ ] **Step 3: Fix** `move_field_option_under_permanent` (capture the target's stable identity / `target_top` before the removal and re-resolve after). Then `cards_behavioral st23_15 st24_15`.
- [ ] **Step 4: Delete `tests/grouped_return_contract.rs`** once its tests pass inside the module; confirm `cargo test -p digimon-engine --test grouped_return_contract` now fails to resolve (the target is gone) and that `Cargo.toml` has no explicit `[[test]]` for it.
- [ ] **Step 5: Coverage flip** — regenerate; assert the 5 constructs `tested_behaviourally`; `git checkout -- qa/impact/lowering_coverage.json`.
- [ ] **Step 6: Gaps** — `docs/RUST_ENGINE_GAPS.md`: `select_zone_cards` drops candidates whose encoded id is out of range (hand ≥ 30, stack index ≥ 12, field slot ≥ 14 — `encode_zone_card` returns `None`), so a large hand or trash can hide a legal pick; a printed "return N cards" (EX7-043) runs as N single-card moves, not one simultaneous move (§3-1-3-3 / §15-5-2); the dead `permanent_mutations.rs:174` `DeletePermanents` arm duplicates `deletion_results::expand` without a receipt.
- [ ] **Step 7: Commit**

```bash
git add code/digimon-engine/tests/dsl/constructs/zone_moves.rs code/digimon-engine/tests/grouped_return_contract.rs code/digimon-engine/src/game_actions/option_lifecycle.rs code/digimon-engine/tests/cards_behavioral docs/RUST_ENGINE_GAPS.md
git commit -m "test(dsl): construct tests for the zone-move family (5 constructs; grouped_return_contract folded in); a field Option relocating under a Tamer resolves the target by identity, not by stale index"
```

---

### Task 7: Family `protection` — 3 constructs (decks: Jupitermon ×2, Examon) — slow track, extra rubric (spec §5d "Protection constructs get more")

**Sheet:** sections 11–13 of the identity/protection sheet. Each grant section has a MUST-block table and a must-NOT-block table; every row of both becomes a test. §15-7-4 is REVERSED in 4.3 (an unpayable optional condition can't be chosen) — read `docs/digimon-rules/changelog-3.6-to-4.3.md` before citing.

**Files:**
- Modify: `code/digimon-engine/tests/dsl/constructs/protection.rs`
- Modify (fix): `code/digimon-engine/src/combat/dp.rs:83-85` + `src/game/mod.rs:3624-3730` (`dynamic_dp_aura_bonus` / `live_declarative_formula_sum` honour `ImmuneFromDPMinus` for negative formula auras from the protected Digimon's opponent); `code/digimon-engine/src/dsl_cards/step/modifiers.rs` (`grant_stack_protection_from_opponent_effects` also installs an opponent-scoped `ImmuneFromStackReturn`, the modifier `return_card_source_to_hand` / `_to_deck` consult — DCGO `BT26_029.cs:115`); `code/digimon-engine/src/dsl_cards/lower_triggered.rs:1086` (`leading_cost_guard` recognises `SuspendAndCancelAllMatchingLeaves`: the replacement is offered only when `target` is unsuspended and can change orientation, §15-7-4)
- Modify: `docs/RUST_ENGINE_GAPS.md`, `qa/dsl-vocab-gaps.md`

**Interfaces:**
- Consumes: Task 1's scaffold.
- Produces: fixture ids `TST-NARROW`, `TST-DPMINUS`, `TST-DEDIGI`, `TST-KILL`, `TST-FORMULA-AURA`, `TST-STACKGUARD` + one attacker fixture per S-row, `TST-SUSPALL`. Private to this file.

- [ ] **Step 1: Write the failing tests:**
  - `verb:grant_narrow_opponent_effect_protection` — after the grant: `narrow_protection_installs_dp_minus_immunity_and_de_digivolve_cancel`; blocked rows `narrow_protection_blocks_opponent_dp_minus` (B1, DP stays 5000), `narrow_protection_blocks_opponent_security_effect_dp_minus` (B2), `narrow_protection_blocks_opponent_literal_aura_dp_minus` (B3), `narrow_protection_blocks_opponent_formula_aura_dp_minus` (B4 — red until the `dp.rs` fix; **no `#[ignore]`**), `narrow_protection_blocks_opponent_de_digivolve` (B6); must-not-block rows `narrow_protection_does_not_block_own_dp_minus` (N1, DP 2000), `narrow_protection_does_not_block_own_de_digivolve` (N2 — pins the BT26 "their effects" scope), `narrow_protection_does_not_block_opponent_deletion` (N3), `narrow_protection_does_not_block_battle` (N4), `narrow_protection_does_not_block_positive_dp` (N7), `narrow_protection_does_not_block_bounce_or_stack_trash` (adjacent paths outside the bundle); expiry `narrow_protection_holds_through_the_opponents_turn_and_ends_after_it`.
  - `verb:grant_stack_protection_from_opponent_effects` — blocked rows S1–S7 each as `stack_guard_blocks_opponent_<path>`, S8/S9 `stack_guard_blocks_opponent_return_of_individual_sources_to_hand` / `_to_deck` (red until the `modifiers.rs` fix; **no `#[ignore]`**); must-not-block `stack_guard_does_not_block_own_source_trash` (M1), `stack_guard_does_not_block_own_costs` (M2), `stack_guard_does_not_block_opponent_deletion_which_trashes_the_stack` (M3, §4-16-3), `stack_guard_does_not_block_battle_deletion` (M4), `stack_guard_does_not_block_own_bounce` (M5); expiry `stack_guard_ends_after_the_opponents_turn`. Every attacker fixture's pick is a real `pending_selection` for player 1.
  - `verb:suspend_and_cancel_all_matching_leaves` — `suspend_all_guard_one_prompt_saves_every_matching_member_and_suspends_the_payer` (G1, G2 stay; X trashed; Tamer suspended; exactly one prompt, no subject prompt; one `OnSuspend` event), `suspend_all_guard_pass_lets_the_whole_batch_leave`, `suspend_all_guard_not_offered_when_the_payer_is_already_suspended`, `suspend_all_guard_not_offered_when_the_payer_cannot_suspend`, `suspend_all_guard_without_hand_authored_gates_is_still_not_offered_when_unpayable` (the sheet's variant (d) — red until the `leading_cost_guard` fix; §15-7-4), `suspend_all_guard_not_offered_for_a_battle_leave`, `suspend_all_guard_reads_the_cause_per_member` (own-effect batch still offered), `suspend_all_guard_once_per_turn`, `suspend_all_guard_covers_a_simultaneous_departure_batch` (h: G1 stays, X bounced), `suspend_all_guard_does_not_fire_for_de_digivolve` (adjacent timing), `suspend_all_guard_chosen_payer_variant_lists_only_payable_digimon` (`target: chosen` after an optional `select_own_permanent`), `suspend_all_guard_probe_non_batch_simultaneous_leaves` (the sheet's unverified case: "place all of your opponent's Digimon under X" — assert what the rules require, §3-1-3-3 + §15-8-5-4; if red, it is a finding, not a test to weaken: file it and drop the test to the ledger).
- [ ] **Step 2: Run** `… --test dsl constructs::protection`. Expected reds: B4, S8, S9, variant (d), and possibly the non-batch probe.
- [ ] **Step 3: Fix the three sites** (formula-aura immunity; `ImmuneFromStackReturn` in the stack grant; the leading cost guard). Then `cards_behavioral bt16_055 bt26_029 bt26_085 ex13_024 ex13_051` and the `replacements` + `keyword_phase_*` binaries. Expected green, or test edits citing §15-7-4 / §4-7 / card text.
- [ ] **Step 4: Coverage flip** — regenerate; assert the 3 constructs `tested_behaviourally`; `git checkout -- qa/impact/lowering_coverage.json`.
- [ ] **Step 5: Gaps and findings** — `qa/dsl-vocab-gaps.md`: BT16-055 prints an **unqualified** "isn't affected by <De-Digivolve> effects" (DCGO `BT16_055.cs:65-70` has no opponent condition) while BT26-029/085 say "their effects" — the verb needs a `de_digivolve_scope: any | opponent` argument and BT16-055 a YAML change (not a top-10 card; own task). `docs/RUST_ENGINE_GAPS.md`: `trash_all_sources` pre-checks `self.player != target.player` before the protection, so an `Any`-scoped `ImmuneFromStackTrashing` never blocks the controller's own `trash_all_sources` while `trash_card_source` would; `suspend_and_cancel_all_matching_leaves` plural scope exists only inside a batch window (if the probe is red); a stricter `filter` than `active_when` strands the triggering subject (authoring hazard — a `dsl-lint` rule candidate); the opponent/own scoping of the narrow grant flips if control changes.
- [ ] **Step 6: Commit**

```bash
git add code/digimon-engine/tests/dsl/constructs/protection.rs code/digimon-engine/src/combat/dp.rs code/digimon-engine/src/game/mod.rs code/digimon-engine/src/dsl_cards/step/modifiers.rs code/digimon-engine/src/dsl_cards/lower_triggered.rs code/digimon-engine/tests/cards_behavioral docs/RUST_ENGINE_GAPS.md qa/dsl-vocab-gaps.md
git commit -m "test(dsl): construct tests for the protection family (3 constructs, every promised and every non-promised path); formula -DP auras honour DP-minus immunity; the stack guard protects individual stacked-card returns; an unpayable suspend-and-cancel is never offered (§15-7-4)"
```

---

### Task 8: Regenerate the gate artifacts and measure the deck flips

**Files:**
- Modify (generated): `qa/impact/lowering_coverage.json`, `data/card_admissibility.json`
- Create: `.superpowers/sdd/2026-10-22-construct-tests/final-report.md` (git-ignored)

- [ ] **Step 1: Regenerate**

```bash
PYTHONIOENCODING=utf-8 PYTHONUTF8=1 PYTHONPATH=code python -m tools.lowering_coverage
PYTHONIOENCODING=utf-8 PYTHONUTF8=1 PYTHONPATH=code python -m digimon_gym.agents.card_admissibility
```

- [ ] **Step 2: Every top-10 construct is behavioural, not parsed-only**

Run: `PYTHONIOENCODING=utf-8 PYTHONUTF8=1 PYTHONPATH=code python - <<'PY'` … load `qa/impact/lowering_coverage.json`, iterate the 50 construct keys (the 51 minus `verb:app_fuse`) from `docs/superpowers/research/2026-10-22-construct-tests/top10_untested_union.json`, assert `constructs[k]["tested_behaviourally"] is True`, print any that are not.
Expected: none printed.

- [ ] **Step 3: Deck report**

Run: `PYTHONIOENCODING=utf-8 PYTHONUTF8=1 PYTHONPATH=code python -m tools.lowering_coverage --report .superpowers/sdd/2026-10-22-construct-tests/final-report.md --decks qa/impact/decks/digilab-11087-top10.json`
Measure the actual result: Task 3 ports behavioral App Fuse tests, so all 51 constructs and 10/10 lowering-covered decks may be reported. Keep this separate from card admissibility: the BT25-089 failed verdict and App Fuse result-signal gap remain. Do not force the original 9/10 lowering-coverage estimate.

- [ ] **Step 4: Tier 0**

Run: `PYTHONIOENCODING=utf-8 PYTHONUTF8=1 python scripts/verify --tier 0`
Expected: 13/13 green (`lowering_coverage_check`, `card_admissibility_check`, `mechanics_readiness_check`, `impact_index_check` included).

- [ ] **Step 5: Admissibility by deck**

Run: `PYTHONIOENCODING=utf-8 PYTHONUTF8=1 PYTHONPATH=code python -m digimon_gym.agents.card_admissibility` then a short script that, for each deck in `qa/impact/decks/digilab-11087-top10.json`, prints inadmissible cards with `reason`.
Measured: 9 decks with zero inadmissible cards; Rebootmon's only inadmissible card is BT25-089 (`failed_verdict`, PARTIAL). All App Fuse users are lowering-covered; no pool card retains `untested_lowering`.

- [ ] **Step 6: Commit**

```bash
git add qa/impact/lowering_coverage.json data/card_admissibility.json
git commit -m "qa: regenerate lowering coverage + card admissibility after the top-10 construct tests (9/10 decks admissible)"
```

---

### Task 9: Verification, ledger, docs

- [ ] **Step 1: Full DSL binary** — `CARGO_TARGET_DIR=E:/cargo-target-wt/p3-constructs cargo test -p digimon-engine --test dsl` — Expected: all green except the two pre-existing reds (`official_rule_grants`, `trait_parity`; confirm they fail identically on `f24f26dfa` before attributing).
- [ ] **Step 2: Suites a construct fix could touch** — if any task changed `src/` (an engine or lowering fix), run `rules_conformance`, `timing_dispatch`, `cost_hooks`, `replacements`, `selection`, `clone_fuzz`, and `RUST_MIN_STACK=268435456 cargo test … --test cards_behavioral -- --test-threads=8`; record counts in the ledger. If no `src/` change landed, record "no src change; suites not rerun".
- [ ] **Step 3: Replay goldens** — only if `src/` changed: `cargo run --manifest-path code/digimon-engine/Cargo.toml --bin replay_corpus -- verify --goldens qa/replay-goldens --cards data/cards.json` 10/10 (regenerate + NOTES.md diagnosis per the Plan 2 forensics procedure if not).
- [ ] **Step 4: Docs** — `docs/TRAINING_RUNBOOK.md` gate section: one paragraph "where the pool stands" with the 9/10 figure and the regenerate commands; `qa/rules/mechanics-gaps-2026-10.md` or `docs/RUST_ENGINE_GAPS.md` entries for every gap a task filed.
- [ ] **Step 5: Ledger + PR body** — `.superpowers/sdd/2026-10-22-construct-tests/progress.md` complete lines per task; `pr-body.md` with per-family construct counts, deck flips, gaps filed, verification counts. Controller opens the PR (draft, base `main`).

## Execution order and parallelism

Task 1 first. Tasks 2–7 are independent (disjoint test files; the `src/` fixes touch disjoint sites except `lower_triggered.rs`, which Tasks 3 and 7 both edit in different functions — run Task 7 after Task 3, or rebase). Recommended order by deck unblock: 5 (`card_number_is` + `can_attack` free seven decks' largest blockers) → 2 → 3 → 4 → 6 → 7. A Workflow fan-out with worktree isolation (one implementer + one reviewer per family, the controller merging and regenerating) is the shape the EX13 card run used and fits here; the sequential SDD loop is the fallback.

## Owner decisions this plan surfaces (answer any time; none blocks a task)

1. **can_attack vs Q3185** (Task 5): a `CannotSuspend` Digimon attacking via "without suspending" — official Q&A + DCGO allow, engine refuses citing §11-2-5. **Resolved by owner 2026-10-22:** follow official Q3185 (`data/card_qa.json`, BT19-101, dated 2024-09-20). Implement row 8 and shared mask/execution eligibility in a separate Q3185 followup commit; see the resolved entry in `docs/RUST_ENGINE_GAPS.md`.
2. **Language Standardization exclusions** (Task 4): **owner authorized 2026-10-22** the separate production fix for all 12 English exact-name/query exclusion pairs, including the existing Indramon exclusion. This supersedes Step 6's report-only constraint and Step 1's prohibition on testing these exclusions. Authority, scope, and evidence: [English name standardization](../../digimon-rules/english-name-standardization.md). Multilingual mappings and the adjacent MarineChimairamon/Kimeramon inclusion remain outside this exclusion fix.
3. **`source_count` and face-down sources** (Task 5): §4-7-9 says face-down cards have no information; the implementer checks DCGO first and fixes unless DCGO counts them.
4. **App Fusion link-card scope** (Task 3 gap): the commit consumes every link card; §8-4 places only the material pair's card.

## Self-review notes

- **Spec coverage (§5d):** items 1–6 of the definition map to Global Constraints 1–5 and the per-test naming in every task; "Protection constructs get more" → Task 7's MUST / must-NOT tables and expiry tests; "What is not a construct test" → the no-`#[ignore]` constraint (Tasks 3, 7 explicitly forbid the sheet's suggested `#[ignore]` rows); "Families, not ranks" → Tasks 2–7 and the execution order; `app_fuse` port → Task 3; "Measurement and exit" → Tasks 8–9. Owner scope "top 10, deduplicated" → 51 constructs = 7 + 13 + 13 + 10 + 5 + 3 = 51 (Task 2 + 3 + 4 + 5 + 6 + 7). ✔
- **Step scan:** every test is named with its assertion stated in the task or in the referenced sheet section; every fix names a file and the rule it serves; the sheets carry the exact YAML so the plan does not transcribe it. Fix-or-file rules are explicit where a fix might balloon (Task 4 (b), Task 5 (a)/(c)). ✔
- **Type / name consistency:** module names in Task 1 (`security_stack`, `play_digivolve_link`, `cost_target_event`, `identity_state`, `zone_moves`, `protection`) match Tasks 2–7 and the `cargo test … constructs::<family>` filters; artifact regeneration commands match `scripts/verify` tier 0 (`tools.lowering_coverage`, `digimon_gym.agents.card_admissibility`). ✔
- **Review Focus:** 1 ↔ the `…_fires_…` / adjacent-timing tests in Tasks 2–4; 2 ↔ paired holds / does-not-hold tests (one fact per negative, stated in Task 5); 3 ↔ Task 7 must-NOT rows; 4 ↔ "≥2 candidates" in every selection test (Tasks 3, 5, 6 state it); 5 ↔ Task 8 Step 2. ✔
- **Proportion:** plan ≈ 330 lines against a 20-line spec section plus ~5,400 lines of sheets; the plan is an index into the sheets, not a transcript. ✔
- **Out of scope, recorded:** the 56 untested constructs the top 10 do not reference (next plan, once the training roster is chosen); the app-fuse result signal (separate substrate task); the `card_number_is` self-proxy card sweep (Task 5 gap, own task); BT16-055's De-Digivolve scope (Task 7 gap, own task); the would-link window for effect links (Task 3 gap, Rebootmon only).
