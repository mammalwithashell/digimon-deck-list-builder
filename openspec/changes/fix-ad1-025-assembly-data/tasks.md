## 1. Spike — DONE (material zone + DCGO flow + blocker)

- [x] 1.1 Material source CONFIRMED = **trash** (`RULES_CONTEXT.md` §7-3: "place specified cards from trash under it"; exact count 7-3-2-4; not mandatory 7-3-2-9). Matches the Q5 scenario.
- [x] 1.2 Schema confirmed (`alt_path.rs`: `MaterialSpec{ name_contains, zones, stack_under }`, `cost`). **BLOCKER found:** no engine Assembly executor — `CompiledAltPathKind::Assembly` is matched in no play path. Re-scoped to a hybrid engine+data+YAML change (user-approved).
- [x] 1.3 DCGO faithfulness reference read: `AD1_025.cs:214-255` (elements + `reduceCost:6`), `SelectAssemblyClass.cs` (`CanFulfillConditions` from trash; `SelectTrashCard` = SelectCardEffect root Trash, exact count, surfaced; `AddDigivolutiuonCards` = bottom placement at exact count), `AddAssemblyConditionClass.cs`.
- [x] 1.4 Log `G-ASSEMBLY-PLAY-EXECUTION` in `qa/archetype-qa/engine-gaps.md` (no engine Assembly executor; blocks Q5 + ~10 `[Assembly]` cards) so it's tracked while in progress

## 2. Engine — Assembly play execution (TDD, faithful to DCGO)

<!-- APPLY MAP (2026-05-29, from architecture spike). Integration points:
  - DNA executor model: game_actions.rs:5483-5614 / 5630-5745 + game.rs:2751 dna_digivolve_inner; stack merge game.rs:2810
  - Play mask + legality (gates on PRINTED cost): action/mask.rs:130-141
  - Play cost reduction: game_actions.rs:660-672 (play), 3729-3736 (digivolve) — `effective_cost = (base - reduction).max(0)`
  - Trash-source selection: effect_context/selections.rs:1373 select_count_capped_multi_min (zone=Trash, min=max=count, surfaced)
  - Bottom placement: permanent.rs:446 push_under (insert at 0)
  - Compiled form: compiled.rs:122 CompiledAltPath{ materials: [CompiledMaterial{ zones, stack_under }], cost: CompiledCost }
  FLOW (D8, DCGO screenshots): NO new action-space slot. Assembly rides existing PLAY_HAND →
    optional "use assembly?" gate (only if pieces in trash) → per-element surfaced trash selection
    → place under (push_under) → pay reduced cost. Reuses pending_selection action ranges.
  D5: assembly `cost` = REDUCTION (DNA's is absolute) — interpret in the assembly play flow. -->

- [x] 2.1 RED anchor test written + confirmed failing: `tests/dsl/assembly_play.rs::assembly_play_with_materials_in_trash_installs_selection` (synthetic Lv7 card with an `assembly` alt_path + 2 named materials `zones:[trash] stack_under`, reduction 6; both materials seeded in trash; `decode_action(PLAY_HAND_START,0)` → asserts a `pending_selection` surfaces). FAILS today (no flow) — locks the GREEN target. Remaining sub-assertions ((a) declare-then-pay mask, (c) exact-count selection, (d) `push_under` placement, (e) reduced cost) added as the executor lands.
- [x] 2.1b Implement the GREEN play-flow integration. DONE — `assembly_or_finish_play_from_hand` routes the cost-reduction-chain-complete site through `try_begin_assembly_flow` (`resolve_eligible_assembly` reads `alt_path_registry`, checks per-element distinct trash eligibility via `assembly_can_fulfill`), installs the optional gate (element 0, `min=0 is_optional_zero`) + required per-element trash selections (`select_count_capped_multi_min`), and on resolution sets `Game::pending_assembly_materials` so `commit_play_from_hand_card_no_replace` places the materials under the new permanent BEFORE its `[On Play]` fires, paying `base − (generic + assembly)` reduction. Verified by `tests/dsl/assembly_play.rs` (6 tests incl. real AD1-025). ORIGINAL RECIPE (kept for reference):
  - **Read the assembly path:** `self.alt_path_registry` (Game field, game.rs:250 — `HashMap<String, Vec<CompiledAltPath>>`) `.get(card_id)` → find `path.kind == CompiledAltPathKind::Assembly`. Its `materials: Vec<CompiledMaterial>` (each `filter`, `zones`, `stack_under`) and `cost: Option<CompiledCost>` (= REDUCTION per D5).
  - **Injection point:** at the cost-reduction-chain-complete site (game_actions.rs:543-553, where it calls `finish_play_from_hand_after_reductions`). Route through a new `assembly_or_finish(...)` that tries assembly first, else calls `finish`.
  - **Eligibility:** for each material element, count controller-trash `CardSource`s matching `material.filter` ≥ element count (mirror `compiled_dna_requirement` name/level matching). If not eligible → `finish` (normal play).
  - **Flow (faithful to the screenshots):** install element-0 trash selection via `EffectContext::new(&mut game, target_card, None, player).select_count_capped_multi_min(player, CountCappedZone::Trash, min=1, max=1, "Assembly: Select [X] from trash.", is_optional_zero=true, distinct, filter, cb)`. Decline (No Selection) → `finish` (full cost). Pick → chain element-1 selection (required). After the last element, in the callback: `finish_play_from_hand_after_reductions(... total_reduction + assembly_reduction)`; on `Played(perm_idx)`, move each picked trash card under the new permanent (remove from trash + `permanent.push_under(cs)`).
  - **Mask (declare-then-pay, D3):** in `action/mask.rs:130-141`, for a hand card with an eligible assembly path, gate legality on `memory - (base - reduction) >= memory_range.0` (reduced cost), not the printed cost.
  - **Helper:** add `move_trash_card_under(perm, handle)` (remove from owner trash by handle, `push_under`).
  - Note refinements: the `WhenPermanentWouldPlay` replacement window currently lives inside `finish` (reused — OK); the optional gate is modeled as is_optional_zero on element-0 (matches the screenshots' "No Selection"). Multi-element distinctness: exclude already-picked handles from later element candidates.
- [x] 2.2 `CompiledAltPathKind::Assembly` arm wired into the play execution; eligibility = per-element distinct trash assignment (`assembly_can_fulfill` — recursive system-of-distinct-representatives, mirrors DCGO `CanFulfillConditions`/`CanFulfillEachElementCondition`)
- [x] 2.3 Action mask (`action/mask.rs`): play legality now computed against the REDUCED cost when an assembly path is eligible (`assembly_play_reduction_for_hand_card`); declare-then-pay (D3). NO new action sub-range (D8 — rides PLAY_HAND); `ACTION_SPEC.md` unchanged
- [x] 2.4 Per element, a trash selection is installed via `select_count_capped_multi_min` (zone Trash, exact count, selecting player = controller, surfaced through `pending_selection` §17); element-0 is the optional "use assembly?" gate; candidates exclude already-chosen handles (distinctness)
- [x] 2.5 On resolution: materials placed at the digivolution-stack BOTTOM (`push_under`) via `pending_assembly_materials`, consumed in `commit_play_from_hand_card_no_replace` BEFORE `[On Play]` fires; play cost reduced by the reduction and paid; played card's own effects fire normally
- [x] 2.6 `cost:` semantic resolved (D5 — assembly `cost` = REDUCTION amount, distinct from DNA's absolute cost); interpreted in `resolve_eligible_assembly`/`try_begin_assembly_flow` and documented in the YAML + override comments
- [x] 2.7 Engine tests green: `tests/dsl` assembly_play (6) + full lib (3413) + all integration suites pass; the only failures are 4 PRE-EXISTING (confirmed failing on baseline with the assembly changes stashed): `dsl::select_materials::…batch_play_from_materials…`, `dsl_eval_arm_coverage::step_variants_have_exec_arms`, `selection::behavioral_end_to_end::permutation_then_opponent_union_zone_tech_flow`, `selection::opponent_permanent::mask_emits_only_valid_targets_plus_pass` — none touched by this change

## 3. Card data + YAML — AD1-025

- [x] 3.1 Added `[Assembly] -6 [WarGreymon] x [MetalGarurumon]` to AD1-025 via `data/card_overrides.json` (`effect_description_eng` override prepends the keyword + a §7-3-phrased reminder; existing text preserved verbatim; `_ad1_025_comment` records the DCGO source + that the official English parenthetical was not recoverable). DNA Digivolve (`xros_req`) untouched
- [x] 3.2 Authored the `assembly` alt_path in `cards/ad1/AD1-025.yaml` (materials WarGreymon + MetalGarurumon, `zones:[trash]`, `stack_under: true`, `cost: 6` reduction) alongside `dna_digivolve`; `name_contains` uses the specific material names (DCGO element names), avoiding the "(X Antibody)" aliasing pitfall
- [x] 3.3 Added the AD1-025 per-card behavioral test (`tests/dsl/assembly_play.rs::ad1_025_omnimon_plays_via_assembly_from_trash`): real card from the embedded pack, WarGreymon + MetalGarurumon in trash → declare Assembly → resolves with the 6-reduced cost (9) and both materials under Omnimon; `[On Play]` fires cleanly (empty opp board). The `dna_digivolve` path + other clauses are an independent alt-path/effects entry (additive change, untouched)

## 4. Pin Q5 + reconcile

- [x] 4.1 Un-ignored `judge_quiz::c_declare_then_pay::q5_assembly_declaration_legal_when_cost_can_be_made_payable` — now a live mask assertion (real AD1-025 + AD1-004 WarGreymon + AD1-014 MetalGarurumon in trash, memory 0 → `PLAY_HAND` legal via the reduced cost). PASSES
- [x] 4.2 Moved `G-ASSEMBLY-PLAY-EXECUTION` from `engine-gaps.md` (open list) to `qa/resolved-gaps.md` with the full resolution note + verification commands; the gap's reusability for the rest of the `[Assembly]` family is recorded
- [x] 4.3 Updated `qa/qa-reports/judge-quiz.md` (Q5 row + tally + gaps section: BLOCKED-DATA → PASS, PASS count 0→1) and `card-resolution.md` (Q5 entry + discovery-wave list): Q5 reclassified as a two-layer gap (data + engine executor), both resolved → PASS
- [x] 4.4 Full engine suite green except 4 PRE-EXISTING failures (confirmed failing on baseline with the assembly changes stashed; none in code this change touches): lib 3413 pass; `tests/dsl` 717 pass (1 pre-existing fail) incl. 6 new assembly_play; `tests/judge_quiz` 5 pass / 0 fail (Q5 now passing); all other integration suites pass
