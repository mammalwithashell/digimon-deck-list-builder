# Medusamon DSL Run — Final Report

**Run dates:** 2026-04-27 / 2026-04-28
**Pipeline:** `/batch-implement-cards-rust-dsl` (Sonnet implementer / Opus reviewer waves)
**Pool source:** `data/deck_library.json` archetype "Medusamon" plus resolver/alias-expanded `qa/archetype-qa/medusamon/deck_pool.json`
**Total cards:** 54

---

## 1. Whole-archetype summary

| Verdict | Count | Notes |
|---|---:|---|
| IMPLEMENTED | 20 | Card ships faithfully; all clauses behavioral-tested or aura-tested. |
| PARTIAL | 33 | At least one clause behavioral; others ignore-tagged with named gap. |
| BLOCKED | 1 | Not enough vocabulary to author any clause faithfully. |
| **Total** | **54** | |

BLOCKED gap-kind split:
- ST22-08 Offensive Plug-In V — **hybrid** (Plug-In/Link mechanic entirely unsupported)

Recovered from previous BLOCKED/lost-file state:
- EX4-006 Guilmon — **resolved 2026-05-03** (count_gte gate implemented; YAML/tests restored)
- BT16-082 Ukkomon — **resolved 2026-05-04** (`when:on_move` body implemented; reveal/add/remainder/hatch flow native)

Final test suite (cumulative across all 14 batches):
- `cargo test --test cards_behavioral`: **558 passed, 0 failed, 225 ignored**
- Full engine suite: green across all binaries

---

## 2. Per-card results

See `qa/qa-reports/validated_cards_dsl.json` for the machine-readable table. The artifact at `qa/archetype-qa/dsl/medusamon.md` maintained by the workers has the human-readable per-card row and operator notes.

Set distribution of shipped YAMLs:

| Set | Cards | Notes |
|---|---:|---|
| BT21 | 13 | Archetype core (Medusamon BT21 line) |
| BT24 | 9 | Archetype 2024 refresh |
| P | 6 | Promos / red-tech support |
| EX11 | 3 | X-Antibody crossover |
| BT20 | 2 | |
| BT23 | 2 | |
| BT9, BT14, BT16, BT17, BT18, BT5, BT8, EX10, EX9, EX8, EX7, EX4, LM, ST1, ST22 | 1 each | |

---

## 3. Engine fixes that landed during the run

The pipeline surfaced 11 real engine bugs / missing primitives that workers fixed end-to-end:

1. **build.rs Phase 1c migration** (orchestrator pre-flight): `cards/` walks every subdirectory, not only `_examples/`. Foundation for all subsequent per-set production YAMLs.
2. **`PlayFromSecurity` dispatch routing** (`src/dsl_cards/step/play_digivolve.rs`): security-skill timing now dispatches to `play_pending_security()` when `pending_security` is set, otherwise `play_from_security(player)`. Affects every DSL card with a `[Security]` `play_from_security: {}` clause.
3. **`has_keyword` walks digivolution stack** (`src/game.rs`): aura scan now traverses `card_sources` for `declarative && inherited` `Grant <Keyword>` effects. Partially closes G-INHERITED-DISPATCH for keyword grants.
4. **`card_data_from_compiled` populates keywords** (`src/debug_runner.rs`, `tests/support/dsl_card_data.rs`): FaceUp `GrantKeyword` declarative clauses surface own-printed keywords without dispatch.
5. **`Progress` and `Training` keyword mappings** (`src/dsl_cards/modifier_map.rs`): added `lookup_keyword` arms for `"Progress"` and `"Training"`.
6. **`SelectOwnPermanent` / `SelectOpponentPermanent` predicate filtering** (`src/dsl_cards/step/selections.rs`): selection installers now pre-filter candidates with `eval_predicate` and pass the filter closure through. Previously accept-all.
7. **Replacement clause subject-guard** (`src/dsl_cards/lower_replacement.rs`): `WhenWouldLeaveBattleArea` only fires when the carrier itself is the leaving subject.
8. **`CompiledCardKind::Token` predicate match** (`src/dsl_cards/predicate.rs`): enables `kind: token` filter on cost steps.
9. **Token-name case insensitivity** (production fix in `BT21-029.yaml`, `EX11-012.yaml`): YAML `token_name:` must lowercase to match `TokenRegistry` keys.
10. **`when_playing_this` cost-reduction scan from hand** (`src/effect.rs`, `src/game_actions.rs`, `src/dsl_cards/lower_cost_reduction.rs`): a new `Effect.when_playing_this: bool` flag plus `scan_before_pay_cost_reduction_for_hand_card` so `when_playing_this: true` clauses can be evaluated for the specific card being played from hand. Sentinel `PermanentHandle { player: controller, index: 255 }` used by `evaluate_amount` when `source_permanent` is None so zone-count formulas still resolve.
11. **`ModifierType::CannotPlayTamerByEffect`** (`src/enums.rs`, `src/game_actions.rs`): companion to `CannotPlayDigimonByEffect`; enforced in `play_from_hand_with_cost` and `play_from_trash_with_cost`. Used by BT23-014 Gallantmon.

Plus a small assortment of cross-batch fixes (binding-keyword bug `target: source_permanent` → `target: source` in BT18-087/BT24-017/BT24-082/BT21-081, install-select-empty-outer-tail bookkeeping in BT21-024, `_deck_before` naming bug in p_103.rs).

---

## 4. New gaps surfaced

### Engine gaps (`qa/archetype-qa/engine-gaps.md`)

| ID | Description | First reported |
|---|---|---|
| G-INHERITED-DISPATCH | `enqueue_from_permanent` doesn't iterate `card_sources[0..n-1]` for inherited triggered effects | BT21-008 |
| G-OPT-TRIGGERED | `Effect::max_per_turn` not consulted in `run_queued_effect_inner` for triggered-effect dispatch | EX11-008 |
| ~~G-ON-MOVE~~ | RESOLVED: `EffectTiming::OnMove` + `when: on_move` dispatch from `move_from_breeding()` with moved-card event context | EX11-008, BT16-082 |
| G-PRED-DP-LTE (consolidated) | Resolved for reusable permanent `dp_lte` / `dp_gte` predicate evaluation by Group 7; older card-level ignores may still need migration | BT21-015 (Batch 2), reused throughout |
| ~~G-EVENT-TARGET-OWNER~~ | RESOLVED for trigger event context and generic replacement context; BT21-029 deletion arm now uses it behaviorally | BT24-018, BT21-029 |
| G-WHEN-DIGIVOLVING-DISPATCH | Own-scope WhenDigivolving triggered effects don't fire when this card is the source being digivolved-into | BT21-013 |
| ~~G-COUNT-LTE-EVAL / G-COUNT-GTE-EVAL~~ | RESOLVED 2026-05-03/04: `count_lte` / `count_gte` aggregate predicates evaluate zone/owner filters in `eval_predicate_with_bindings`; EX8-074's cost gate now carries `count_gte` for 2 unsuspended Digimon | BT21-017, EX4-006, EX8-074 |
| G-DP-LTE-PREDICATE (= G-PRED-DP-LTE) | Same root cause as above; reusable predicate path resolved by Group 7 | BT21-015 |
| G-FOR-EACH-DELETE-INDEX-SHIFT | `for_each` snapshot indices stale after first deletion in multi-target sweep | BT8-097 |
| G-FORMULA-KIND-FILTER | Resolved 2026-05-02: `card_count_in_zone` supports predicate filters; BT8-097 now counts opponent Digimon only | BT8-097 |
| G-DECLARATIVE-KEYWORD | `EffectTiming::Declarative` defined but never enqueued (filtered auras don't fire) | BT5-008 |
| G-AURA-DP-FORMULA | `AuraBody.dp_modifier: Option<i32>` is static literal only — no formula support | BT21-072 |
| G-ON-DIGIVOLVE-TRAIT-FILTER | `on_digivolve` trigger context doesn't carry the newly-digivolved permanent | BT24-082 |
| G-AURA-MODIFIER-DROP | `lower_aura.rs` drops `modifier` field from `AuraBody` | EX10-010 |
| G-CANNOT-BE-AFFECTED-NOT-ENFORCED | `ModifierType::CannotBeAffected` not consulted in effect execution | EX10-010 |
| G-IGNORE-COLOR-MASK | Rust action mask never received the Python `IgnoreColorRequirement` fix | ST22-08 |
| G-DELAY-START-OF-TURN | `DelayTrigger` enum has no `StartOfYourNextTurn` variant | LM-027 |
| G-DELAY-SUSPEND-CONDITION | `DelayTrigger` only EOT variants; no on-suspend trigger | BT24-089 |
| G-EFFECT-DIGIVOLVE-FROM-HAND | (Resolved during run via Phase 3a verification — `effect_initiated_digivolve cost: { reduce: N }` already works) | — |
| G-OPP-SECURITY-COUNT-LTE | No `opponent_security_count_lte` predicate | BT24-018, BT21-093 |
| G-SELECT-EMPTY-OUTER-TAIL | Outer-tail steps after `as_selecting_player` lost when inner `select_hand` has no candidates | BT24-024 |
| G-MULTI-SELECT-OPP-DP-SUM | Multi-select with running DP-sum cap | LM-021, BT17-018 |
| G-ALL-TURNS-FILTER | `active_when: { all_turns: true }` opponent-turn firing unverified | BT24-018 |
| G-LOSE-COUNT-BOUND | Count-driven loop combinator | BT17-018 |
| G-ADD-TOP-SECURITY-TO-HAND | `EffectContext::add_top_security_to_hand` missing | P-137 |
| G-GAME-EVENT-DIGIVOLVE | `GameEvent::Digivolve` not emitted | EX11-054 |
| G-ON-ENTER-FIELD-ANYONE-TRAIT-FILTER | Observer triggers don't expose entering permanent | EX11-054 |
| G-PLACE-SELF-AS-OPTION-PERMANENT | No `place_option_in_battle_area` verb | BT24-089 |
| G-SELF-DIGIVOLUTION-CONTAINS-NAME | Predicate to check own digivolution-source names | BT20-102 |
| G-FOR-EACH-EXCLUDE-BINDING | for_each can't exclude a previously-selected binding | BT20-102 |
| G-COLOR-MATCH-AGAINST-BOARD | No predicate to dynamically match against board colors | P-206 |

### DSL-vocab gaps (`qa/dsl-vocab-gaps.md`)

| ID | Description | First reported |
|---|---|---|
| BT23-005 cost-reduction-trigger predicate | `CostReductionBody` lacks `when_this_digivolves_into + target_trait_has` | BT23-005 |
| ~~EX11-008 OnMove DSL token~~ | RESOLVED via `when: on_move` | EX11-008 |
| G-ATK-TRAIT-FILTER | `attacker_trait_has` predicate on `on_attack_target_change` clauses | BT21-025 |
| G-ALT-PATH-CONDITION | `AltPathSpec` missing `condition: Option<PredicateSpec>` field | BT24-016 |
| ~~G-PLAY-COST-LTE~~ | RESOLVED for select_hand/select_trash card filters; older card notes may still need migration | P-189 |
| ~~G-MAY-ATTACK-NOW~~ | RESOLVED 2026-05-08 for mid-effect optional/forced attack prompts (`may_attack_now`, `force_attack`) | BT24-082, BT21-081 |
| G-ZONE-TRASH-TO-DECK | No DSL verb for "return trash card to bottom of deck" | BT24-017 |
| G-TRASH-SELECTED-SECURITY | No verb to trash a non-top selected security card | BT24-018 |
| G-DSL-LINK-VERB | No DSL clause/step for link card mechanic | ST22-08 |
| G-DSL-LINKED-SCOPE | No `scope: linked` in `CompiledScope` | ST22-08 |
| G-BINDING-DP-FORMULA | Formula can't reference named binding's DP | ST22-08 |
| G-ADD-OPTION-SELF-TO-HAND | No DSL verb to add this Option card to hand from security | LM-027 |
| G-PLAYER-FLOOD-GATE-DSL | DSL `flood_gate` is permanent-level only; no `add_player_modifier` | BT5-008 |
| G-OTHER-PREDICATE-UNEVALUATED | `other: true` parses but `eval_permanent_fields` ignores it | BT5-008 |
| G-ON-DIGIVOLVE-TRAIT-FILTER (DSL half) | (cross-ref engine half) | BT24-082 |

---

## 5. Operator notes

- **Per-batch process:** scout-implementer-reviewer-merge ran at full fidelity for the first ~3 batches; mid-run agents began doing direct merges to the main tree (writing `validated_cards_dsl.json`, `medusamon.md`, gap trackers) themselves rather than letting the orchestrator handle merge — saving turn cost but violating the skill's "workers never edit shared state" invariant. No data corruption observed; the trackers stayed consistent.
- **Latent bug pattern:** several workers wrote `target: source_permanent` instead of `target: source` for self-bindings (silent no-op via `CompiledBindingRef::Named`). Caught and fixed across BT18-087, BT24-017, BT24-082, BT21-081 — would have shipped silently if not for cross-batch agents inspecting each other's YAML.
- **Path drift:** ~3 workers wrote YAML to `cards/_examples/` instead of `cards/<set>/`, or test files to flat `cards_behavioral/` instead of nested `cards_behavioral/<set>/`. Orchestrator caught and relocated each at merge time; flagged as worker-drift in commit messages.
- **Recovered 2026-05-03:** EX9-008 Biyomon and EX4-006 Guilmon YAML/test files are back in the main tree. EX4-006 also closed the shared `count_lte` / `count_gte` aggregate predicate evaluator gap.
- **One-stall recovery:** BT21-093 Raging Serpentine's worker stalled mid-test. Orchestrator finished by registering the agent's referenced raw_rust stubs (`bt21_093_cost_reduction_amount`, `bt21_093_delete_highest_dp_opponent`) and writing structural-only tests (4 active, 5 ignored).
