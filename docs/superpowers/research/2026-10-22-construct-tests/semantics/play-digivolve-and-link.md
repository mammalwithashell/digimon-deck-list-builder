# Construct semantics — family `play-digivolve-and-link`

Date: 2026-10-22. Worktree `D:/wt/p3-constructs` @ f24f26dfa (read-only research; no cargo run).

Constructs (in order): `verb:play_or_use_from_hand`, `verb:may_dna_digivolve_now`,
`predicate:can_digivolve_from_source`, `verb:hatch`, `verb:arm_digivolve_cost_reducer`,
`timing:when_linked`, `timing:when_card_linked_to_this`, `timing:on_any_link`,
`timing:when_would_link_to_this`, `predicate:would_link_card_trait_any_of`,
`predicate:link_card_count`, `verb:reduce_link_cost`, `verb:app_fuse`.

Paths: `dsl/` = `code/digimon-dsl/src/`, `eng/` = `code/digimon-engine/src/`, tests relative to
`code/digimon-engine/`. Every proposed test goes in `tests/dsl/<name>.rs` (a scanned root) and must
use an inline YAML string with the construct's YAML key, so `lowering_coverage.py` credits it via
`yaml_key`, and must run the engine (`DebugRunner`).

Shared harness facts (verified in code):
- Fire a triggered fixture: `let h = r.place_on_field(0, "FIX", Some(0)); r.game.enqueue_triggered(EffectTiming::OnPlay, TriggerSource::Permanent(h)); r.game.drain_effect_queue();` (pattern from `tests/dsl/play_or_use_from_sources.rs`).
- Hand-selection action ids are `PLAY_HAND_START + i` (`eng/effect_context/selections.rs:411-418`). Own-field picks (link host, suspend cost, may-DNA partner) are `encode_attack(0, idx)`. Effect-choice entries are `HAND_EFFECT_START + n`. Decline an optional prompt with `PASS` (62). Accept a replacement prompt with `REPLACEMENT_ACCEPT` (59).
- Link by player action (standing linker): `r.game.enter_main_phase(); r.game.decode_action(FIELD_EFFECT_START + linker.index as u16 * EFFECTS_PER_PERMANENT + FIELD_EFFECT_SLOT_FOR_LINK, 0);` then resolve the host pick `encode_attack(0, host.index)` (pattern from `tests/cards_behavioral/bt25/bt25_004.rs:263-310`). Link from hand: `decode_action(HAND_EFFECT_START + hand_idx, 0)` (`eng/game_actions/link.rs:200`). Link by effect API (no cost, no would-link window): `r.game.link_chosen_card_into_host(host, card, LinkCardSource::Hand(0))` (`eng/game_actions/link.rs:843`).
- A linked card needs a printed `[Link]`, or the rules check trims it (§17-1-3-2-6, Q4533). In YAML use `kind: link_condition`, otherwise use the builder's `.link_requirement(id, cost)`. A Digimon holds 1 link card (§4-9-5); for 2+ use `r.give_link_plus(host, n)`. Stage existing links with `r.push_linked_owned(host, id, owner)`, then `r.game.tick_declarative_effects()`.

---

### `verb:play_or_use_from_hand`

- **Lowering site(s):**
  - Parse: `dsl/step.rs:328` (`StepSpec::PlayOrUseFromHand`), args struct `dsl/step.rs:2686` (`PlayOrUseFromHandArgs { of, hand_index, cost_delta }`).
  - Compile: `dsl/compile.rs:3389`. IR: `dsl/compiled.rs:2085`.
  - Runtime: step `eng/dsl_cards/step/play_digivolve.rs:434-449`, helper `eng/effect_context/action/play.rs:434-503` (`play_or_use_from_hand_with_cost`).
  ```rust
  CompiledStep::PlayOrUseFromHand { of: _, hand_index, cost_delta } => {
      if let Some(ResolvedBinding::HandIndex(owner, i)) = resolve_binding_ref(hand_index, ctx, bindings) {
          let delta = lower_cost_delta(cost_delta.as_ref(), ctx, bindings);
          ctx.play_or_use_from_hand_with_cost(owner, i as usize, delta);
      }
      true
  }
  // helper: match card_kind { Digimon|Tamer|DigiEgg => play_from_hand_with_cost(..),
  //   Option => use_option_from_hand_with_cost(..), Token => {},
  //   Dual => select_effect_choice("Play as Digimon or use as Option",
  //           ["Play as Digimon","Use as Option"]) + ResumeFrame::PlayOrUseDualChoice }
  ```
- **YAML shape** (ST23-04):
  ```yaml
  - select_hand: { of: you, bind_as: gd_pick, optional: true, filter: { trait_has: "Glowing Dawn" } }
  - play_or_use_from_hand: { of: you, hand_index: gd_pick, cost_delta: { reduce: 3 } }
  ```
  `cost_delta` also takes a formula, e.g. EX13-043 `reduce: { base: 4, per: { suspended_count: { of: any } }, delta: 1 }`.
- **Semantics:**
  - Reads the `HandIndex` binding written by an upstream `select_hand`. The candidate set and its optionality ("you may") belong to that `select_hand`. The verb surfaces no pick of its own.
  - It routes on the bound card's kind:
    - Digimon, Tamer or Digi-Egg: played via `play_from_hand_with_cost(PlaySource::ByEffect)`. The face cost minus `cost_delta` is paid, and `OnPlay` / enter-field triggers fire.
    - Option: used via `use_option_from_hand_with_cost`, with the full Option lifecycle and the reduced use cost.
    - DUAL: surfaces a mandatory `SelectionKind::EffectChoice` with exactly the labels `["Play as Digimon", "Use as Option"]`. It is resume-driven, so it is clone-safe.
    - Token: does nothing.
  - An absent binding (declined `select_hand`) is a silent no-op that returns `true`.
  - `cost_delta: None` means the full printed cost, not free.
  - `of` is ignored. The owner comes from the binding.
  - The played permanent's handle is not written to `bindings`.
  - Differences from the tested sibling `play_or_use_from_sources` (`tests/dsl/play_or_use_from_sources.rs`):

    | | `play_or_use_from_sources` | `play_or_use_from_hand` |
    |---|---|---|
    | Binding | `SourceRefs` from `select_own_sources` | `HandIndex` from `select_hand` |
    | `cost_delta: None` means | Free | Full printed cost |
    | `of` | Resolved and enforced: a source on another player's stack is rejected (that file's `honors_of_player` test) | Discarded |
    | Play path | `play_from_materials` | `play_from_hand_with_cost` |
    | Option origin | `OptionSource::Source` | Hand |
- **Rule basis:**
  - §9-1 (using an Option card; §9-1-1 colour requirement, §9-1-2 [Main] activates).
  - §6-5-1 / §5 (play cost paid on play).
  - Otherwise this is a card-text idiom ("play or use 1 … with the cost reduced by N"). The DUAL face is a player choice, so no auto-pick (CLAUDE.md no-approximations).
- **Production users:**
  - **BT26-033 Jupitermon** (TS Jupitermon / TS Toolbox / Chronomon lists).
    - Printed: "Then, if it's your turn, you may play or use 1 [Iliad] or [TS] trait card from your hand with the cost reduced by 5."
    - YAML: `select_hand { bind_as: iliad_pick, optional: true, filter: { any_of: [ {trait_has: Iliad}, {trait_has: TS} ] } }` → `play_or_use_from_hand { of: you, hand_index: iliad_pick, cost_delta: { reduce: 5 } }`.
  - **BT26-012 Manekimon** (Toho Braves).
    - Printed: "[Main] [Once Per Turn] You may play or use 1 [TB] trait card from your hand with the cost reduced by 2."
    - YAML: `select_hand { bind_as: pick, optional: true, filter: { trait_has: "TB" } }` → `if binding_present: pick then play_or_use_from_hand { of: you, hand_index: pick, cost_delta: { reduce: 2 } } else refund_opt`.
- **Existing near-miss tests:**
  - The coverage index lists only `src/dsl_cards/step/play_digivolve.rs`, which is the lowering itself (it names the variant, no test).
  - `tests/dsl/play_or_use_from_sources.rs` exercises only the sibling verb.
- **Proposed behavioural test** (`tests/dsl/play_or_use_from_hand.rs`):
  - **Fixture:**
    ```yaml
    card: TEST-POU-HAND
    name: Play Or Use From Hand
    kind: digimon
    level: 3
    color: [red]
    cost: 3
    dp: 3000
    effects:
      - when: on_play
        process:
          - select_hand:
              of: you
              bind_as: picked
              optional: true
              filter: { trait_has: Target }
              prompt: "Play or use 1 [Target] card with the cost reduced by 3"
          - play_or_use_from_hand:
              of: you
              hand_index: picked
              cost_delta: { reduce: 3 }
    ```
    Plus a second YAML variant without `cost_delta`.
  - **Setup:**
    - Reuse the sibling's CardData helpers `digimon("H-DIGI", 5)`, `option("H-OPT", 5)` and `dual("H-DUAL", 5, 5)`, each with `traits = ["Target"]`.
    - Register `OptionMainDraw` (copy from the sibling) on `H-OPT`.
    - `.hand(0, &[<one target card>])`, `.deck(0, &["DECK-PAD"; 5])`, `.memory(10)`.
    - Fire the fixture's OnPlay (shared harness).
  - **Positive (Digimon):**
    - Pending `select_hand` lists the target in `valid_action_ids` (`PLAY_HAND_START + idx`) and has `is_optional == true`.
    - Pick it. Battle area now contains `H-DIGI` and memory went down by exactly 2 (5 - 3).
  - **Positive (Option):** hand +1 from the `[Main]` draw, minus the used card. `H-OPT` is in trash and memory went down by 2.
  - **Positive (no `cost_delta`):** memory goes down by the full 5. This pins "None = full cost", the opposite of the sibling.
  - **Negative:** PASS the `select_hand`. Nothing is played, memory is unchanged and the hand is unchanged.
  - **Selection surface:** pick `H-DUAL`.
    - Pending is `SelectionKind::EffectChoice` with labels `["Play as Digimon", "Use as Option"]` and `is_optional == false` (the face is mandatory once picked).
    - Resolving `HAND_EFFECT_START` plays it to the battle area.
    - Assert `pending_selection_resume.is_some()` before resolving (clone-safety).
- **Suspected engine/DSL defects:**
  1. `of` is discarded (`of: _`). An authored `of: opponent` is silently treated as the binding's owner, whereas the sibling rejects it. No production card writes anything but `of: you`.
  2. The comment at `play_digivolve.rs:442-445` says "A played Digimon's handle is recorded for any later step", but the code never calls `bindings.record_played`, and the helper returns `()`. A later step cannot reference the played Digimon.
  3. No affordability gate in the verb. Producers must filter with `affordable_with_cost_reduce` (BT26-084 does; ST23-04 and BT26-033 do not). The test author should not assert behaviour for an unaffordable pick.

### `verb:may_dna_digivolve_now`

- **Lowering site(s):**
  - Parse: `dsl/step.rs:562`, args `dsl/step.rs:1911` (`anchor` defaults to `source`; `partner_filter`, `target_filter`, `cost`, `ignore_requirements`, `optional`, `prompt`).
  - Compile: `dsl/compile.rs:4527`. IR: `dsl/compiled.rs:3063`.
  - Runtime step: `eng/dsl_cards/step/dna_digivolve.rs:25-108`. Orchestrator: `eng/effect_context/action/digivolve.rs:888-1045`. Pair legality: `eng/effect_context/mod.rs:1714-1749` (`dna_pair_can_reach_hand_card`, `dna_pair_cost_for_hand_card`).
  ```rust
  // partner is offered only if some hand target is DNA-reachable by {anchor, partner}
  let partner_filter = Arc::new(move |game, partner| partner_filter(game, partner)
      && (0..hand.len()).any(|i| partner_target_filter(game, i)
          && (ignore_requirements || dna_pair_can_reach_hand_card(game, controller, anchor, partner, i))));
  if !has_partner { return; }  if !has_target { return; }      // silent no-op
  self.select_own_permanent(&partner_prompt, optional, |g,h| h != anchor && partner_filter(g,h), |ctx, partner| {
      ctx.select_hand(controller, &target_prompt, optional, /* target_filter && pair reaches i */, |ctx, hand_idx| {
          let charge = if ignore_requirements { cost as i32 }
                       else { dna_pair_cost_for_hand_card(..).unwrap_or(cost as i32) };   // printed DNA cost
          ctx.effect_initiated_dna_digivolve(anchor, partner, card, charge, ignore_requirements);
      })});
  // + ResumeFrame::MayDnaPartnerSelection (clone-safe)
  ```
- **YAML shape** (EX13-008 / BT12-021 inherited EOT):
  ```yaml
  - may_dna_digivolve_now:
      partner_filter: { all_of: [ { of: you }, { kind: digimon } ] }
      target_filter:  { all_of: [ { of: you }, { kind: digimon } ] }
      cost: 0
      ignore_requirements: false
      optional: true
      prompt: "DNA digivolve this Digimon and another of your Digimon into a Digimon card in your hand"
  ```
  BT25-028 supplies `anchor: { binding: dna_anchor }` from a prior `select_own_permanent`.
- **Semantics:**
  - Resolves `anchor` to a battle-area permanent. If it is unresolved or not a permanent, this is a silent no-op.
  - Surfaces up to two real selections:
    1. An own-field partner pick (`select_own_permanent`). The anchor is always excluded. A partner is offered only when the {anchor, partner} pair satisfies some filtered hand card's printed DNA requirement, unless `ignore_requirements`.
    2. A hand pick of the result, restricted to cards that pair can reach.
  - Both picks carry `is_optional = optional`.
  - If there is no eligible partner or no eligible target, nothing is installed: a silent no-op, matching DCGO `CanActivateCondition`.
  - Commit goes through `effect_initiated_dna_digivolve`, which fires `WhenDigivolving → OnDnaDigivolve → OnDigivolve`.
  - With `ignore_requirements: false`, the printed DNA cost of the matched requirement is paid and the authored `cost` is ignored. With `true`, the authored `cost` is paid unchecked and the recipe is not enforced.
  - Partner and result must be the controller's own cards.
- **Rule basis:**
  - §8-2-1, §8-2-3-1..3: DNA procedure — reveal, choose requirement and materials, pay the DNA cost, stack, draw 1.
  - §8-2-2-4: DNA can only be performed by an effect that specifically performs DNA.
  - §8-2-2-1-x: post-DNA state is unsuspended, a new card, OPT reset.
  - §8-2-2-2: stacking order.
  - §16-31 `<Blast DNA Digivolve>`. The brief says §16-30, but §16-30 is `<Collision>` in 4.3. §16-31-4 says it "performs DNA digivolution" and §16-31-5 says requirements can't be ignored. This is the analogue that justifies the `ignore_requirements: false` default.
- **Production users:**
  - **BT25-028 Dianamon** (both Jupitermon lists, TS Toolbox, Chronomon, Toho Braves).
    - Printed: "[All Turns] [Once Per Turn] When any Digimon are played or digivolve, you may trash any 4 digivolution cards from your opponent's Digimon. Then, 2 of your Digimon may DNA digivolve into [GraceNovamon] in the hand."
    - YAML: `select_own_permanent { bind_as: dna_anchor, optional: true, filter: {kind: digimon, zone: [battle_area]} }` → `may_dna_digivolve_now { anchor: { binding: dna_anchor }, partner_filter: {of: you, kind: digimon}, target_filter: {of: you, kind: digimon, name_contains: "GraceNovamon"}, cost: 0, ignore_requirements: false, optional: false }`.
  - **EX13-008 Dracomon** (Examon).
    - Inherited: "[End of Your Turn] This Digimon and any of your other Digimon may DNA digivolve into a Digimon card in the hand."
    - YAML as shown under YAML shape, on a `scope: inherited, when: end_of_your_turn, optional: true` clause.
- **Existing near-miss tests:**
  - `tests/dsl/phase3_dna_digivolve_triggers.rs::may_dna_digivolve_now_prompts_clone_faithfully` calls `EffectContext::may_dna_digivolve_now` directly with Rust closures. It is behavioural for the engine API (partner prompt → result prompt → merge, clone-safety) but has no YAML or `CompiledStep`, so it earns no credit.
  - `src/dsl_cards/step/dna_digivolve.rs` is the lowering itself.
  - `eng/effect_context/mod.rs` `#[cfg(test)] dna_pair_gating_requires_a_legal_dna_route_and_returns_printed_cost` covers only the helper.
- **Proposed behavioural test** (`tests/dsl/may_dna_digivolve_now.rs`):
  - **Fixture:**
    ```yaml
    card: TEST-MAY-DNA
    name: May DNA Anchor
    kind: digimon
    level: 3
    color: [red]
    cost: 3
    dp: 3000
    effects:
      - when: on_play
        process:
          - may_dna_digivolve_now:
              partner_filter: { all_of: [ { of: you }, { kind: digimon } ] }
              target_filter:  { all_of: [ { of: you }, { kind: digimon } ] }
              ignore_requirements: false
              optional: true
    ```
  - **Setup:**
    - Copy the `digimon(id, level, dna_costs, evo_costs)` / `red_level_req` helpers from `tests/dsl/phase3_dna_digivolve_triggers.rs`.
    - `SRC-B` is red Lv.3. `OFF-COLOR` is blue Lv.3 (a partner that cannot reach the recipe). `DNA-RESULT` is Lv.4 with `dna_costs: [red Lv3 + red Lv3, memory_cost: 2]`.
    - `.hand(0, &["DNA-RESULT"])`, `.memory(5)`, deck pad.
    - `place_on_field` `SRC-B` and `OFF-COLOR`, then place the anchor and fire its OnPlay.
  - **Positive:**
    - Pending partner pick has `is_optional == true`. Its `valid_action_ids` contains `encode_attack(0, SRC-B.idx)` and excludes the anchor's slot and `OFF-COLOR`'s slot (the pair-legality gate).
    - Pick `SRC-B`. The next pending is a hand pick that contains `PLAY_HAND_START + idx(DNA-RESULT)`.
    - Pick it. The battle area has one permanent fewer, with `DNA-RESULT` on top and the anchor and `SRC-B` among `card_sources`. Memory is `before - 2` (printed DNA cost, not the authored `cost: 0`).
  - **Negatives:**
    1. No reachable target: replace `DNA-RESULT` in hand with a Lv.4 card that has no `dna_costs`. After drain `pending_selection.is_none()`, the battle area is unchanged and memory is unchanged (silent no-op, no prompt).
    2. `ignore_requirements: true` variant with `cost: 1`: `OFF-COLOR` becomes a legal partner and memory goes down by exactly 1.
  - **Selection surface:** with `optional: false`, both picks report `is_optional == false`. With `optional: true`, PASS at the partner pick leaves the board unchanged. Assert `pending_selection_resume.is_some()` at each pick.
- **Suspected engine/DSL defects:**
  1. Double / unguarded "may". `MayDnaDigivolveNow` is not in `body_first_step_is_declinable` (`eng/dsl_cards/lower_triggered.rs:796-870`) and has no `first_step_candidate_guard` arm (`lower_triggered.rs:911-966`). An `optional: true` clause led by it (BT12-021, EX13-008, EX12-001: clause `optional: true` + step `optional: true`) therefore gets an outer accept/decline prompt:
     - It is shown even when no partner or target exists. DCGO `CanActivateCondition` = false would not offer it.
     - After accepting, a second PASS-able partner prompt follows.
     Worth a negative-case assertion in the clause-level test: no prompt when ineligible.
  2. Anchor/partner stacking order (§8-2-2-2: the requirement's left card goes on top) is decided by `effect_initiated_dna_digivolve(anchor, partner, …)` argument order, not by a player or requirement-ordered choice. Not verified.

### `predicate:can_digivolve_from_source`

- **Lowering site(s):**
  - Parse: `dsl/predicate.rs:195` (`Option<bool>`). Compile: `dsl/compile.rs:1028`. IR: `dsl/compiled.rs:431`.
  - Runtime (card subject only): `eng/dsl_cards/predicate.rs:3666-3670` inside `eval_card_fields` (`:3189`), plus `can_card_digivolve_from_source` `:3768` and `can_card_digivolve_onto` `:3783`.
  ```rust
  if let Some(want) = pred.can_digivolve_from_source {
      if can_card_digivolve_from_source(rctx, card) != want { return false; } }
  fn can_card_digivolve_from_source(rctx, card) -> bool {
      let Some(source_handle) = rctx.source_permanent else { return false; };
      can_card_digivolve_onto(rctx, card, source_handle) }
  fn can_card_digivolve_onto(rctx, card, target) -> bool {
      if permanent_for_handle(rctx, target).is_none() { return false; }
      if modifiers.has(target, CannotDigivolve)
         || modifiers.player_has(target.player, CannotDigivolveDigimonByEffect) { return false; }
      rctx.game.all_digivolve_routes_for_card(candidate, target).iter().any(|r| !r.app_fusion) }
  ```
- **YAML shape** (P-196):
  ```yaml
  filter:
    all_of:
      - kind: digimon
      - can_digivolve_from_source: true
      - any_of: [ { trait_has: Sea Beast }, { trait_has: TS } ]
  ```
- **Semantics:**
  - A card-subject leaf: true when the candidate card has at least one normal-digivolve route onto the effect's source permanent. Routes come from:
    - printed evo circles;
    - DSL `alt_paths: kind: digivolve`;
    - the `CanOnlyDigivolveInto` gate.
    App Fusion routes are excluded.
  - False when there is no source permanent (e.g. an Option body) or the source is gone.
  - False when the source has `CannotDigivolve`, or its controller has `CannotDigivolveDigimonByEffect`.
  - `false` inverts the test, so with no source every card matches `false`.
  - Never evaluated for a permanent subject. It is used as a hand/trash pick filter so the prompt offers only cards the following `effect_initiated_digivolve` commit would accept. Level/colour requirements are not bypassed.
- **Rule basis:**
  - §8-1-1 / §8-1-3-1: digivolution needs a met requirement.
  - §8-1-2-2: ignoring requirements only when an effect says so (then this leaf must not be used, see P-092 face text).
  - §8-4-2-2: App Fusion can't be performed by a standard-digivolve effect, hence it is excluded.
- **Production users:**
  - **P-196 Gomamon** (TS lists).
    - Printed: "[Start of Your Main Phase] If you have 4 or less memory, this Digimon may digivolve into a Digimon card with the [Sea Beast] or [TS] trait in the hand without paying the cost."
    - YAML snippet shown under YAML shape, followed by `effect_initiated_digivolve { target: source, from_hand: evo, cost: 0 }`.
  - **BT26-076 Crowmon** (DATA SQUAD Ravemon).
    - Printed: "…this Digimon may digivolve into [Ravemon] or a [DATA SQUAD] trait Digimon card in the trash with the cost reduced by 1."
    - YAML: `select_union_zone { zones: [trash], optional: true, filter: { all_of: [ {kind: digimon}, {can_digivolve_from_source: true}, {any_of: [ {name_is: Ravemon}, {trait_has: "DATA SQUAD"} ]} ] } }` → `effect_initiated_digivolve { target: source, source: evo, cost: { reduce: 1 } }`.
- **Existing near-miss tests:** the index lists only `src/dsl_cards/predicate.rs`, the evaluator itself. No test.
- **Proposed behavioural test** (`tests/dsl/can_digivolve_from_source.rs`):
  - **Fixture:**
    ```yaml
    card: TEST-CDFS
    name: Can Digivolve From Source
    kind: digimon
    level: 3
    color: [red]
    cost: 3
    dp: 3000
    effects:
      - when: on_play
        process:
          - select_hand:
              of: you
              bind_as: evo
              optional: true
              filter: { all_of: [ { kind: digimon }, { can_digivolve_from_source: true } ] }
              prompt: "Pick a card this Digimon can digivolve into"
    ```
    A second fixture uses `{ can_digivolve_from_source: false }`.
  - **Setup:** hand holds:
    - `ROUTE-OK`: Lv.4 red Digimon, YAML with `alt_paths: [{ kind: digivolve, from: { level_eq: 3, color_is: red }, cost: 2 }]` (memory note: DebugRunner cards need explicit evo routes);
    - `ROUTE-LV5`: Lv.5 with a route only from level 4;
    - `ROUTE-BLUE`: Lv.4 with a route only from blue Lv.3.
    Place the fixture (red Lv.3) and fire OnPlay.
  - **Positive:** the pending `select_hand` `valid_action_ids` equals exactly `{PLAY_HAND_START + idx(ROUTE-OK)}`.
  - **Negatives:**
    1. Add a `CannotDigivolve` modifier on the fixture permanent before firing. No selection is installed (zero candidates; `select_hand` installs nothing).
    2. With the `false` fixture, the candidate set is the other two cards.
    3. Optional, for the source-less case: evaluate the compiled predicate with `EffectReadContext::new(.., source_permanent: None, ..)` on a card. It must return false.
  - **Selection surface:** `is_optional == true` (from `select_hand`). The leaf itself surfaces nothing.
- **Suspected engine/DSL defects:**
  - The prohibition check uses `target.player`'s `CannotDigivolveDigimonByEffect`. The comment says it should match "the controller used by the DSL commit, which can differ from the player owning the initiating effect". Consistent for own-source use; untested for a cross-control source.
  - Otherwise none seen.

### `verb:hatch`

- **Lowering site(s):**
  - Parse: `dsl/step.rs:288` (`Hatch(PlayerArg)`). Compile: `dsl/compile.rs:3306`. IR: `dsl/compiled.rs:1999`.
  - Runtime: `eng/dsl_cards/step/draw.rs:69-73` → `EffectContext::hatch` `eng/effect_context/action/play.rs:1280` → `Game::hatch` `eng/game/suspend.rs:267-285` → `Player::hatch` `eng/player.rs:343-353`.
  ```rust
  CompiledStep::Hatch { of } => { let p = resolve_player(ctx, *of); ctx.hatch(p); true }
  pub fn hatch(&mut self, player_id) -> bool {           // Game
      let ok = self.player_mut(player_id).hatch(turn);
      if ok { self.mark_until_condition_dirty();
              for pid in 0..n { self.enqueue_triggered(OnHatch, TriggerSource::PlayerBattleArea(pid)); }
              self.drain_effect_queue();                  // <- not maybe_drain
              self.reevaluate_until_condition_modifiers_if_dirty(); }
      ok }
  pub fn hatch(&mut self, turn) -> bool {                // Player
      if self.breeding_area.is_some() { return false; }
      if let Some(egg) = self.digitama_deck.pop() { self.breeding_area = Some(Permanent::new(egg, turn)); true } else { false } }
  ```
- **YAML shape:** `- hatch: { of: you }`, usually behind `if: { condition: { can_hatch: you } }` (EX1-066, EX4-074) or a `select_effect_choice ["Hatch", "Don't hatch"]` for "you may" (P-123, BT16-082).
- **Semantics:**
  - Moves the top card of `of`'s Digi-Egg deck (`digitama_deck.pop()`, i.e. the last element) into that player's breeding area as a new permanent entered this turn.
  - No-op returning false when the breeding area is occupied or the Digi-Egg deck is empty. The step itself always returns `true` and binds nothing.
  - No phase restriction (it is an effect).
  - Not optional and surfaces no selection. "may" must be authored around it.
  - On success it enqueues `OnHatch` for every player's battle area. Breeding-area effects can't trigger, §3-4-7-4. It then drains the queue immediately.
  - `of: opponent` hatches the opponent's egg.
- **Rule basis:**
  - §4-18-1: hatching = top Digi-Egg card to the breeding area.
  - §4-18-2: can't hatch with an empty Digi-Egg deck.
  - §4-18-3: can't hatch with an occupied breeding area.
  - §6-4-1-1: the breeding-phase action.
  - §3-4-7-2: 1 card in the breeding area.
  - §15-8-3-2: trigger-type effects can't activate during the processing of an effect.
- **Production users:**
  - **EX4-074 ShineGreymon: Ruin Mode** (Toho Braves; also TS Toolbox).
    - Printed: "[End of Attack] … Then, if you have a Tamer in play, hatch 1 Digi-Egg card to an empty space in your breeding area."
    - YAML: `if: { condition: { all_of: [ { any_field_permanent: { of: you, kind: tamer } }, { can_hatch: you } ] }, then: [ { hatch: { of: you } } ] }`.
  - **EX1-066 Analog Youth** (Toho Braves; Chronomon).
    - Printed: "…you may suspend this Tamer. If you do, gain 1 memory, then hatch 1 Digi-Egg card to an empty space in your Breeding Area."
    - YAML: `activation_cost: { suspend_self: true }` → `gain_memory: 1` → `if: { condition: { can_hatch: you }, then: [ { hatch: { of: you } } ] }`.
- **Existing near-miss tests:** all call `Game::hatch` directly, never the DSL verb.
  - `tests/dsl/option_use_req.rs` (lines 233-381): `runner.game.hatch(0)` as setup for breeding-area Use-requirement tests.
  - `tests/dsl/phase1c_predicate.rs::can_hatch_predicate_requires_empty_breeding_and_digitama`: tests the `can_hatch` predicate, with `game.hatch` as a precondition.
  - `tests/dsl/phase3d_event_context.rs::hatch_does_not_fire_on_move_observers`: engine hatch, asserts no on_move.
  - `tests/timing_dispatch.rs::on_hatch_fires_when_egg_hatches`: the `OnHatch` timing via `game.hatch`.
  - `tests/effect_context/effect_initiated_dna_digivolve.rs`: false positive ("escape hatch" in a comment).
- **Proposed behavioural test** (`tests/dsl/hatch_verb.rs`):
  - **Fixtures:**
    ```yaml
    card: TEST-HATCH
    name: Hatch Fixture
    kind: tamer
    color: [red]
    cost: 0
    effects:
      - when: on_play
        process:
          - hatch: { of: you }
          - gain_memory: 1
    ---
    card: TEST-HATCH-OBS
    name: Hatch Observer
    kind: tamer
    color: [red]
    cost: 0
    effects:
      - when: on_hatch
        process:
          - set_memory: 5
    ```
    Register each with its own `from_dsl_yaml`.
  - **Setup:**
    - `make_test_card("EGG-A")` and `"EGG-B"` with `card_kind = DigiEgg`, `level = Some(2)`.
    - `.digitama(0, &["EGG-A", "EGG-B"])`, `.memory(2)`, deck pad.
    - `place_on_field` the observer, then the fixture, then fire the fixture's OnPlay.
  - **Positive:**
    - Record `top = players[0].digitama_deck.last()` before.
    - After drain, `players[0].breeding_area` is `Some` with top card == `top`, and the Digi-Egg deck length dropped by 1.
    - The observer fired, so memory ends at 5 or 6.
  - **Ordering assertion (§15-8-3-2):** memory must be 5, not 6. Memory 5 means the observer's `set_memory: 5` ran after the body's `gain_memory: 1`; memory 6 means it ran mid-body. This guards the suspected defect below.
  - **Negatives:**
    1. `place_in_breeding(0, "EGG-B")` first (occupied). The Digi-Egg deck is unchanged, the breeding area holds the same card, and the observer did not fire (memory = 2 + 1 = 3).
    2. Empty `.digitama(0, &[])`. Same outcome.
    3. Variant fixture with `of: opponent`: player 1's breeding area fills and player 0's stays empty.
  - **Selection surface:** none. Assert `pending_selection.is_none()` after drain (no auto-prompt).
- **Suspected engine/DSL defects:**
  1. `Game::hatch` calls `drain_effect_queue()`, not `maybe_drain_effect_queue()` (`eng/game/suspend.rs:281`; the same file's other observer sites at `:42` and `:222` do use `maybe_drain_effect_queue`). The deferred-drain rule (`effect_queue.rs:1280-1290`: "every fire_on_* observer helper that previously inline-drained should route through `maybe_drain_effect_queue()`") is not followed. The nested drain's guards (`effect_queue.rs:1739`, `:1890`, `:1904`) cover only security and Option-body cases. So an `OnHatch` observer may resolve in the middle of a Digimon/Tamer effect body that hatches. That violates §15-8-3-2: P-123 "may hatch … Then, gain 1 memory" would let observers act before the memory gain. Confirm with the ordering assertion above.
  2. No `GameEvent` is emitted for an effect hatch (cosmetic).

### `verb:arm_digivolve_cost_reducer`

- **Lowering site(s):**
  - Parse: `dsl/step.rs:554`, args `dsl/step.rs:1757` (`amount: i32`, `single_fire`, `target_color`, `suspend_cost`). Compile: `dsl/compile.rs:4670`. IR: `dsl/compiled.rs:3032`.
  - Runtime install: `eng/dsl_cards/step/effects.rs:118-131` → `eng/effect_context/action/digivolve.rs:152-170` (push onto `Game::player_digivolve_cost_reducers`, expiry `EndOfTurn`).
  - Consumption:
    - Collection: `eng/game_actions/cost.rs:456-533` (`try_prompt_digivolve_reducer_chain_mode`; filters with `PlayerDigivolveCostReducer::qualifies`, `eng/player_cost_reducer.rs:93-112`).
    - Resolution: `eng/game_actions/digivolve.rs:1319-1395` (`advance_player_digivolve_reducers`), prompt `:1642-1715`, accept and suspend pick `:1717-1760`, steps `:700-760`, consume `:1770-1800`.
    - Expiry: `eng/game_phases.rs:544-547`.
  ```rust
  DigivolveReducerChoice::Player { index, reducer } => {
      if prohibited || (reducer.suspend_cost && !payable) { consume(index, &reducer, reducer.single_fire); }
      else if !reducer.suspend_cost { state.reduction += reducer.amount; consume(..); }   // mandatory, no prompt
      else { state.remaining.push(..) }                                                     // optional prompt
  }
  // qualifies(): self.player == acting_player && kind == Digivolve
  //              && target_color.map_or(true, |c| top_card_colors.contains(&c))
  // run_digivolve_reducer_prompt_step(is_pass) => consume(single_fire) without reduction
  // run_digivolve_reducer_suspend_step: suspend picked own Digimon; state.reduction += amount; consume
  ```
- **YAML shape** (BT3-103):
  ```yaml
  - arm_digivolve_cost_reducer: { amount: 5, single_fire: true, target_color: green, suspend_cost: true }
  ```
  ST12-15 `<Delay>` uses `{ amount: 1, single_fire: true }`.
- **Semantics:**
  - Installs a player-scoped reducer owned by the effect controller. It lives "for the turn": it is dropped at the end of the installing player's turn.
  - At the controller's next digivolution, the reducer qualifies only if the digivolving permanent's pre-digivolve top card includes `target_color` (any colour when unset).
  - Without `suspend_cost`: the reduction is applied automatically with no prompt (a mandatory "reduce by N").
  - With `suspend_cost`:
    - An optional `EffectChoice` is surfaced ("Use … to reduce digivolution cost?", `is_optional: true`).
    - Accepting surfaces a mandatory `OwnField` pick of 1 unsuspended own Digimon (`encode_attack(0, i)`). It may be the one digivolving.
    - That Digimon is suspended and the reduction applies.
    - PASS gives no reduction.
    - With no suspendable Digimon, or a `CannotReduce*` prohibition, the reducer is silently consumed with no reduction.
  - `single_fire` removes the reducer at the first qualifying digivolution even if declined. A non-qualifying (wrong colour) digivolution does not consume it.
  - The opponent's digivolutions never qualify.
- **Rule basis:**
  - Card-text idiom ("For the turn, when one of your green Digimon would next digivolve, by suspending 1 of your Digimon, reduce the digivolution cost by 5").
  - §8-1-3-2: the digivolution cost is paid after reductions.
  - §16-10-1/-3 (`<Digisorption>`, same suspend-to-reduce shape the engine reuses): once suspended, the reduction is mandatory.
  - §15-7-4: optional processing condition.
- **Production users:**
  - **BT3-103 Hidden Potential Discovered!** (Rebootmon, Examon, TS Ceres lists).
    - Printed: "[Main] For the turn, when one of your green Digimon would next digivolve, by suspending 1 of your Digimon, reduce the digivolution cost by 5."
    - YAML snippet shown under YAML shape.
  - **ST12-15 From Master to Disciple** (Royal Knights).
    - Printed: "[Main] <Delay> … The next time one of your Digimon would digivolve this turn, reduce the digivolution cost by 1."
    - YAML: `kind: delay` → `arm_digivolve_cost_reducer: { amount: 1, single_fire: true }`.
- **Existing near-miss tests:** the index lists only `src/dsl_cards/step/effects.rs`, the lowering. The reducer machinery may have card tests under `cards_behavioral/bt3`, which don't count.
- **Proposed behavioural test** (`tests/dsl/arm_digivolve_cost_reducer.rs`):
  - **Fixture** (variant B: `amount: 1, single_fire: true`, no colour, no suspend; variant C: `single_fire: false`):
    ```yaml
    card: TEST-ARM
    name: Arm Reducer
    kind: tamer
    color: [green]
    cost: 0
    effects:
      - when: on_play
        process:
          - arm_digivolve_cost_reducer: { amount: 5, single_fire: true, target_color: green, suspend_cost: true }
    ```
  - **Setup:**
    - `GREEN3`: green Lv.3 Digimon on the field.
    - `RED3`: red Lv.3 Digimon on the field.
    - `OTHER`: an extra unsuspended green Digimon to pay the suspend.
    - Hand: `G4`, green Lv.4, YAML `alt_paths: [{ kind: digivolve, from: { level_eq: 3 }, cost: 6 }]`, plus a second copy.
    - `.memory(10)`, deck pad (digivolve draws). `enter_main_phase()`.
    - Fire the fixture's OnPlay.
    - Assert `r.game.player_digivolve_cost_reducers.len() == 1`.
  - **Positive (A):**
    - `r.game.digivolve_from_hand(0, idx(G4), GREEN3.idx, PlaySource::ByHand)`. Pending is `EffectChoice` with `is_optional == true` and one entry.
    - Resolve `HAND_EFFECT_START`. Pending is `OwnField` with `is_optional == false`, whose `valid_action_ids` include `encode_attack(0, OTHER.idx)` and `encode_attack(0, GREEN3.idx)`.
    - Pick OTHER. OTHER is suspended, G4 is on top of the GREEN3 stack, memory is 10 - (6 - 5) = 9, and the reducer list is empty.
  - **Negatives:**
    1. Digivolve onto `RED3` first: no prompt and full cost 6 paid. The reducer is still installed (len 1) because a wrong colour doesn't consume.
    2. PASS at the EffectChoice: full cost and the reducer is consumed (single_fire). A second green digivolve this turn has no prompt.
    3. `r.pass_turn()` / end turn: reducer list is empty (expiry).
    4. Variant B: no prompt at all, memory goes down by 6 - 1 = 5 (mandatory auto-apply), then consumed.
    5. Variant C (`single_fire: false`): two digivolutions both reduced.
  - **Selection surface:** the accept prompt is optional; the suspend pick is mandatory and lists only unsuspended own Digimon. Never auto-picked.
- **Suspected engine/DSL defects:**
  1. Consumption identity uses `(player, source_card, amount)` (`digivolve.rs:1781-1800`). Two copies of BT3-103 used the same turn are distinct cards, which is fine. Two arms from the same card handle in one turn (re-use via ST12-15's `<Delay>` + refire) would be indistinguishable, a minor concern.
  2. Not verified: whether the player reducer also applies to DNA digivolution (§8-2-2-5) and App Fusion (§8-4-2-3: "effects that affect digivolution also affect" them). `cost.rs` is reached from `digivolve_from_hand` and the effect-digivolve declaration. The DNA path was not traced.

### `timing:when_linked`

- **Lowering site(s):**
  - Parse: `dsl/clause.rs:307` (`Timing::WhenLinked`). Compile: `dsl/compile.rs:366`. IR: `dsl/compiled.rs:1501`.
  - Timing map: `eng/dsl_cards/timing_map.rs:94` (`WhenLinked → EffectTiming::OnLink`).
  - Builder: `eng/dsl_cards/lower_triggered.rs:110`, `:186-200` (forces `.linked()` even without `scope: linked`; OPT keyed like inherited).
  - Self-filter: `:335-339`.
  - Dispatch: OnLink enqueued once per player as `TriggerSource::Linked { player, host, card }` at:
    - `eng/game_actions/link.rs:812-822` (standing absorb);
    - `:941-951` (chosen-card / hand / effect link);
    - `eng/game_actions/options.rs:1135-1145` (Plug-In Option);
    - `eng/option_lifecycle.rs:223-233`.
    The scan is at `eng/effect_queue.rs:567-580` and the context at `:2343-2353`.
  ```rust
  if is_when_linked && rctx.event_card() != Some(rctx.source_card) { return false; }   // lower_triggered.rs:337
  // TriggerContext for Linked: event_permanent = host, event_card = just-linked card,
  //   event_host_permanent = host, source_player = scanned player
  ```
- **YAML shape** (BT25-007):
  ```yaml
  - scope: linked
    when: when_linked
    process:
      - select_opponent_permanent: { bind_as: del_target, filter: { all_of: [ {kind: digimon}, {dp_lte: 3000} ] } }
      - delete_permanent: { target: del_target }
  ```
- **Semantics:**
  - The printed `[When Linking]` on a card's link box. It fires when THIS card becomes a link card.
  - The effect lives on the linked card and runs as the host's effect: `source_permanent` = host, so "this Digimon" = the host.
  - Fires once per link event in which the just-linked card is the source card. It does not fire when another card links to the same host, and it does not fire from hand or on play.
  - Fires for every attach path (player link from field or hand, `link_cards` effect link, Plug-In Option).
  - Not optional unless the clause says so.
  - OPT is keyed as an inherited/linked effect.
- **Rule basis:**
  - §15-16-6-1: [When Linking] triggers when the card with that effect becomes a link card.
  - §4-3-5: a Digimon gains the link effects on its link card.
  - §2-3-11-4-6: the link effect is the link-box text.
  - §10-1-3-3: link procedure end.
  - §15-8-3: trigger-type.
- **Production users:**
  - **BT25-007 Gatchmon** (Rebootmon).
    - Link effect: "[When Linking] Delete 1 of your opponent's Digimon with 3000 DP or less."
    - YAML shown under YAML shape.
  - **BT25-052 Logimon** (Rebootmon).
    - Link effect: "[When Linking] Suspend 1 of your opponent's Digimon or Tamers."
    - YAML: `scope: linked, when: when_linked` → `select_opponent_permanent { filter: { any_of: [ {kind: digimon}, {kind: tamer} ] } }` → `suspend`.
- **Existing near-miss tests:**
  - The index lists only `src/dsl_cards/lower_triggered.rs` and `timing_map.rs`, the lowering itself.
  - Real coverage is in `tests/cards_behavioral/bt21/bt21_018.rs`, `bt25/bt25_007.rs` and others. They are excluded roots and earn no credit.
- **Proposed behavioural test** (`tests/dsl/link_timings.rs`, shared with the next three constructs):
  - **Fixtures:**
    ```yaml
    card: TEST-LINKER
    name: Test Linker
    kind: digimon
    level: 3
    color: [green]
    cost: 3
    dp: 2000
    traits: [Social]
    effects:
      - kind: link_condition
        cost: 1
        filter: { trait_has: LinkHost }
      - scope: linked
        when: when_linked
        process:
          - draw: { of: you, count: 1 }
    ---
    card: TEST-LINKER-2        # identical but NO when_linked clause (a sibling)
    name: Test Linker Two
    kind: digimon
    level: 3
    color: [green]
    cost: 3
    dp: 2000
    traits: [Beast]
    effects:
      - kind: link_condition
        cost: 1
        filter: { trait_has: LinkHost }
    ```
  - **Setup:**
    - `HOST-A`: CardData Lv.4 Digimon, `traits = ["LinkHost"]`. `.memory(5)`, `.deck(0, &["DECK-PAD"; 12])`.
    - Place HOST-A and TEST-LINKER on the field, `enter_main_phase()`.
    - Link TEST-LINKER (field link bit), resolve the host pick `encode_attack(0, HOST-A.idx)`, then `auto_resolve()`.
  - **Positive:** hand size +1 (the draw ran once). HOST-A `linked_cards` == [TEST-LINKER]. Memory is `before - 1`.
  - **Negatives:**
    1. **Adjacent timing on_play.** `place_on_field` TEST-LINKER and fire its `OnPlay`: no draw. Playing is not linking.
    2. **Sibling links to the same host.** `give_link_plus(HOST-A, 1)` first so both fit. Link TEST-LINKER (+1 draw), then link TEST-LINKER-2 onto the same host. Hand +0 for the second link: the already-linked TEST-LINKER's `when_linked` must not re-fire.
    3. **Effect-path parity.** `r.game.link_chosen_card_into_host(HOST-A, linker_hand_card, LinkCardSource::Hand(0))` with TEST-LINKER in hand also draws exactly 1.
- **Suspected engine/DSL defects:** none seen.

### `timing:when_card_linked_to_this`

- **Lowering site(s):**
  - Parse: `dsl/clause.rs:314`. Compile: `dsl/compile.rs:367`. IR: `dsl/compiled.rs:1507`.
  - Map: `eng/dsl_cards/timing_map.rs:98` (`→ OnLink`). Flag: `eng/dsl_cards/lower_triggered.rs:115-118`. Host self-filter: `:340-345`.
  - Same dispatch sites as `when_linked`.
  ```rust
  if is_host_linked && rctx.event_permanent() != rctx.source_permanent { return false; }
  // event_permanent = the host the card attached to (TriggerSource::Linked.host)
  ```
- **YAML shape** (BT25-060):
  ```yaml
  - when: [when_card_linked_to_this, on_unsuspend]
    once_per_turn: true
    process: [ grant_keyword: { target: self, keyword: Piercing, expiry: end_of_your_turn }, ... ]
  ```
- **Semantics:**
  - A face-up (or inherited) effect on the HOST. Fires once when any card becomes a link card of THIS permanent (`event_permanent == source_permanent`).
  - It does not fire when a card links to a different permanent, and it does not fire when this permanent itself becomes a link card of something else (it is then no longer on the field).
  - Fires for every attach path.
  - Any trait or turn gating is authored separately (`active_when`).
- **Rule basis:**
  - Card-text idiom "When this Digimon gets linked" (trigger-type, §15-8-3-1/-3).
  - §4-10-1: a linked card = a card that has a link card.
  - §10-1-3-3: link resolution.
  - §15-16-8: [Your Turn] / [All Turns] gates are authored via `active_when`.
- **Production users:**
  - **BT25-060 Rebootmon** (Rebootmon deck).
    - Printed: "[All Turns] [Once Per Turn] When this Digimon gets linked or unsuspends, until your turn ends, this Digimon gains <Piercing> and <Blocker>, and your opponent's Digimon effects don't affect it."
    - YAML shown under YAML shape.
  - **BT26-086 Dantemon** (Rebootmon).
    - Printed: "[All Turns] [Once Per Turn] When this Digimon gets linked, you may delete 1 of your opponent's Digimon. Then, if this Digimon has 7 link cards, …"
    - YAML: `when: when_card_linked_to_this, once_per_turn: true` → `select_opponent_permanent { optional: true, continue_on_decline: true }` → …
- **Existing near-miss tests:** index lists only `lower_triggered.rs` and `timing_map.rs`. Card-level coverage (bt21_018, bt25_060, bt26_086) is in excluded `cards_behavioral`.
- **Proposed behavioural test** (in `tests/dsl/link_timings.rs`):
  - **Fixture:** both hosts are this YAML (two physical copies):
    ```yaml
    card: TEST-HOST-OBS
    name: Host Observer
    kind: digimon
    level: 4
    color: [green]
    cost: 4
    dp: 4000
    traits: [LinkHost]
    effects:
      - when: when_card_linked_to_this
        process:
          - gain_memory: 3
    ```
    The linker is TEST-LINKER-2 from above (no link effects of its own).
  - **Setup:** two copies of TEST-HOST-OBS (A, B) and TEST-LINKER-2 on the field, `.memory(2)`, main phase.
  - **Positive:** link onto A. Memory is 2 - 1 + 3 = 4. Exactly one host fired: +3 once, not +6.
  - **Negatives:**
    1. Sibling host. Covered by the +3-not-+6 assertion; additionally link a second linker onto B and see +3 again (B fires for its own link).
    2. **Adjacent timing.** A `when_card_linked_to_this` host whose *own* card is the linker: make TEST-HOST-OBS also carry `kind: link_condition { cost: 0, filter: { trait_has: LinkHost } }`. Link A onto B. Only B's +3 fires; A became a link card and must not fire.
    3. `on_play` of the host does not fire.
- **Suspected engine/DSL defects:** none seen.

### `timing:on_any_link`

- **Lowering site(s):**
  - Parse: `dsl/clause.rs:334`. Compile: `dsl/compile.rs:369`. IR: `dsl/compiled.rs:1518`.
  - Map: `eng/dsl_cards/timing_map.rs:107` (`→ OnLink`, no forced filter).
  - Dispatch: same sites; the battle-area scan of every player is `eng/effect_queue.rs:567-580`.
  - `event_target_owner` reads `event_permanent` (the host) first: `eng/dsl_cards/predicate.rs:2882-2895`.
  ```rust
  TriggerSource::Linked { player, .. } => { for i in 0..battle_area(player).len() {
      enqueue_from_permanent(OnLink, PermanentHandle{player, index: i}, Some(ctx)) } }
  // enqueued for pid in 0..players.len() at every attach site
  fn event_target_owner(rctx) { ... if let Some(h) = trigger.event_permanent { return Some(h.player) } ... }
  ```
- **YAML shape** (P-217):
  ```yaml
  - when: on_any_link
    optional: true
    active_when: { all_of: [ { your_turn: true }, { source_is_unsuspended: true } ] }
    trigger_if:
      all_of:
        - event_target_owner: you
        - any_of: [ { event_card_trait_has: Social }, { event_card_trait_has: Navi }, { event_card_trait_has: Tool } ]
    process:
      - activation_cost: { suspend_self: true }
      - gain_memory: 1
  ```
- **Semantics:**
  - A board-wide observer on any battle-area permanent (Tamers included) of either player. Fires for every link event on either side.
  - It has no built-in self, host or owner filter: scope it with `trigger_if` (decided when the link triggers it) and `active_when`.
  - In the trigger context, `event_target_owner` is the host's controller, `event_card_*` is the just-linked card, and `event_target` binds to the host permanent.
  - Optionality comes from the clause.
- **Rule basis:**
  - Trigger-type §15-8-3-1/-3/-4. The `trigger_if` qualifier is checked at trigger time (§15-8-3-4).
  - §15-16-8: [Your Turn].
  - Otherwise a card-text idiom ("When your Digimon get linked").
- **Production users:**
  - **BT21-084 Haru Shinkai** (Rebootmon).
    - Printed: "[Your Turn] When your Digimon get linked, by suspending this Tamer, <Draw 1>. Then, 1 of your Digimon may app fuse into a Digimon card in the hand."
    - YAML: `when: on_any_link, optional: true, active_when: { your_turn: true }, condition: { source_is_unsuspended: true }, trigger_if: { event_target_owner: you }` → `activation_cost: { suspend_self: true }`, `draw`, `app_fuse`.
  - **P-217 Haru Shinkai** (Rebootmon). Printed: "[Your Turn] When any of your Digimon get linked to a [Social], [Creation], [Navi] or [Tool] trait card, by suspending this Tamer, gain 1 memory." YAML shown under YAML shape.
- **Existing near-miss tests:** index lists only `timing_map.rs`. Card tests bt21_084 and bt21_101 are in excluded `cards_behavioral`.
- **Proposed behavioural test** (in `tests/dsl/link_timings.rs`):
  - **Fixtures:**
    ```yaml
    card: TEST-ANYLINK
    name: Any Link Observer
    kind: tamer
    color: [red]
    cost: 0
    effects:
      - when: on_any_link
        trigger_if: { event_target_owner: you }
        process:
          - draw: { of: you, count: 1 }
    ---
    card: TEST-ANYLINK-RAW     # no trigger_if — proves no hidden filter
    name: Any Link Raw
    kind: tamer
    color: [red]
    cost: 0
    effects:
      - when: on_any_link
        process:
          - gain_memory: 1
    ```
  - **Setup:** P0 has TEST-ANYLINK, HOST-A (`LinkHost`) and TEST-LINKER-2 on the field. P1 has HOST-OPP (`LinkHost`) on the field and a `.link_requirement`'d card in hand.
  - **Positive:** P0 links TEST-LINKER-2 onto HOST-A. P0 hand +1.
  - **Negatives:**
    1. **Owner gate.** `r.game.link_chosen_card_into_host(HOST-OPP, p1_hand_card, LinkCardSource::Hand(1))`. P0's TEST-ANYLINK does not draw. With TEST-ANYLINK-RAW on P0's field instead, P0 memory +1 for that same opponent link (no hidden owner filter).
    2. **Observer on the opponent's side also sees P0's link.** Put TEST-ANYLINK-RAW on P1's field. P0 links: the effect resolves for P1 (memory moves toward P1, `source_player` = P1).
    3. **Adjacent timing.** A plain play of a Digimon (OnPlay) and a digivolve do not fire.
- **Suspected engine/DSL defects:** the construct itself looks correct (none seen). P-217 has a production-card error, which is not the construct: its printed text lists [Social], [Creation], [Navi] or [Tool], but the YAML `trigger_if` omits `event_card_trait_has: Creation` (`code/digimon-engine/cards/p/P-217.yaml`).

### `timing:when_would_link_to_this`

- **Lowering site(s):**
  - Parse: `dsl/clause.rs:324`. Compile: `dsl/compile.rs:368`. IR: `dsl/compiled.rs:1513`.
  - Map: `eng/dsl_cards/timing_map.rs:103` (`→ EffectTiming::WhenWouldLink`, a REPLACEMENT timing). Replacement builder accepted at `eng/dsl_cards/lower_replacement.rs:42`.
  - `lower_triggered.rs`:
    - Flag: `:124-128`.
    - The outer-optional prompt is suppressed: `:241-246`.
    - Host self-filter: `:346-349`.
    - The body runs via `.replacement_process(..)`: `:386-407`.
  - Fire site: `eng/game_actions/link.rs:562-617` (`begin_digimon_link`). It sets `pending_digimon_link` and `pending_link_host`, then calls `try_replace(WhenWouldLink, ReplacementSubject::Card(card, Hand|BattleArea), OwnEffect, Some(BattleArea))`. It parks on an interactive prompt; otherwise it commits.
  - Commit: `:621-677` (`commit_digimon_link`). The cost paid = `pending_digimon_link.cost + link_cost_delta_for_player`, after the window.
  ```rust
  if is_would_link_to_this && rctx.pending_link_host() != rctx.source_permanent { return false; }
  // begin_digimon_link: self.pending_digimon_link = Some(PendingDigimonLink{origin, host, cost, card});
  //                     self.pending_link_host = Some(host); let outcome = self.try_replace(WhenWouldLink, ..);
  //                     if self.pending_selection.is_some() { return; }  self.commit_digimon_link(outcome);
  ```
- **YAML shape** (BT25-045):
  ```yaml
  - when: when_would_link_to_this
    optional: true
    once_per_turn: true
    active_when: { your_turn: true, would_link_card_trait_any_of: [Social, Tool, Game] }
    process:
      - reduce_link_cost: { amount: 1 }
  ```
- **Semantics:**
  - An immediate-type (replacement-framework) effect on the HOST (face-up or inherited). It is offered after the host is chosen and before the link cost is paid, only for links attaching to THIS permanent.
  - The accept/decline is the replacement framework's own optional prompt. Accept with `REPLACEMENT_ACCEPT`, decline with `PASS`. No extra outer prompt is installed.
  - The body runs inside the window. Its outcome stays `None`, so the link still resolves.
  - Only the player-action Digimon link (`begin_digimon_link`, standing or from hand) opens this window. `link_cards` effect links and Plug-In Option links do not (see defects).
- **Rule basis:**
  - §10-1-2-3: an immediate-type effect that triggers "when linking would occur" triggers right after the card is revealed and the Digimon chosen.
  - §10-1-3-1..3: choose host → pay cost → plug in.
  - §15-8-5-1..4: immediate-type effects; each activates once per cause.
  - §15-14-1-4: OPT.
- **Production users:**
  - **BT25-045 Onmon** (Rebootmon). Printed: "[Your Turn] [Once Per Turn] When a [Social], [Tool] or [Game] trait card would link to this Digimon, you may reduce the cost by 1." YAML shown under YAML shape.
  - **BT25-004 Tapmon** (Rebootmon). Same text as an inherited effect; YAML is identical with `scope: inherited`.
- **Existing near-miss tests:** index lists only `lower_triggered.rs` and `timing_map.rs`. Card coverage is in excluded `tests/cards_behavioral/bt25/bt25_004.rs` (accept, decline, OPT, non-trait); that is the harness template.
- **Proposed behavioural test** (`tests/dsl/when_would_link_to_this.rs`; covers this construct, `would_link_card_trait_any_of` and `reduce_link_cost` in separate `#[test]`s):
  - **Fixtures:**
    ```yaml
    card: TEST-WLINK-HOST
    name: Would Link Host
    kind: digimon
    level: 4
    color: [green]
    cost: 4
    dp: 4000
    traits: [LinkHost]
    effects:
      - when: when_would_link_to_this
        optional: true
        process:
          - reduce_link_cost: { amount: 1 }
    ```
    The linker is TEST-LINKER (link cost 1 → bump to `cost: 2` for clearer arithmetic). PLAIN-HOST is a CardData `LinkHost` Lv.4 with no effects.
  - **Setup:** TEST-WLINK-HOST, PLAIN-HOST and the linker on P0's field. `.memory(5)`, main phase.
  - **Positive:**
    - Activate the linker's field link bit, then pick `encode_attack(0, WLINK-HOST.idx)`.
    - Now `pending_is_optional()` is true. This is the pre-commit "would" window, before payment: memory is still 5 and the host's `linked_cards` is empty.
    - Resolve `REPLACEMENT_ACCEPT` and `auto_resolve()`. Memory = 5 - (2 - 1) = 4 and `linked_cards.len() == 1`.
  - **Decline:** PASS. Memory = 3 (full cost) and the card is still linked.
  - **Negatives:**
    1. **Different host.** Link onto PLAIN-HOST: `pending_selection.is_none()` right after the host pick and memory = 3.
    2. **Adjacent timing.** The post-link `OnLink` must not re-offer: after the link resolves, no pending selection remains.
  - **Selection surface:** the prompt is `is_optional == true`, attributed to the host (`source_card` = WLINK-HOST's top card), and there is exactly one prompt (no outer + replacement double prompt).
- **Suspected engine/DSL defects:**
  1. The window exists only in `begin_digimon_link`. Effect links via `link_cards` (`eng/dsl_cards/step/link_cards.rs:1157-1164` pays `reduced_link_cost` then `link_chosen_card_into_host`) and Plug-In Option links (`finish_option_hand_link` → `play_option_core`) never open `WhenWouldLink` and never set `pending_digimon_link`. So BT25-004 / BT25-045 cannot reduce, for example, BT25-089's "[Main] … link … with the cost reduced by 2" or BT25-052's own `[Main]` link. DCGO's `GrantedReduceLinkCostClass` is a cost-calculation hook that should apply to any link that pays a cost. The `link_chosen_card_into_host` doc says "Cost payment and WhenWouldLink are the calling effect's responsibility", and `link_cards` does not take on that responsibility.
  2. `docs/RUST_ENGINE_API.md:2645` still says pre-link optional accept/decline "require a follow-up resume slot". This is stale: `pending_digimon_link` parks and resumes.

### `predicate:would_link_card_trait_any_of`

- **Lowering site(s):**
  - Parse: `dsl/predicate.rs:927` (`Option<Vec<String>>`). Compile: `dsl/compile.rs:1408`. IR: `dsl/compiled.rs:704`.
  - Runtime: `eng/dsl_cards/predicate.rs:1025-1042`, in `eval_replacement_fields` (`:999`, evaluated subject-independently at `:628`).
  - Reader: `eng/effect_context/action/link.rs:28-30` (`would_link_subject_card() = pending_digimon_link.map(|p| p.card)`).
  ```rust
  if let Some(traits) = pred.would_link_card_trait_any_of.as_ref() {
      let Some(card) = rctx.would_link_subject_card() else { return false; };
      let Some(data) = rctx.game.card_data_for_handle(card) else { return false; };
      if !data.traits.iter().any(|t| traits.iter().any(|want| want == t)) { return false; } }
  ```
- **YAML shape** (BT25-045 / BT25-004): `active_when: { your_turn: true, would_link_card_trait_any_of: [Social, Tool, Game] }`.
- **Semantics:**
  - True only while a player-action Digimon link is in flight (`pending_digimon_link` set, i.e. inside the `WhenWouldLink` window) and the linking card's `CardData.traits` contains at least one listed string.
  - The match is EXACT and case-sensitive. `trait_has` is case-insensitive.
  - False outside the window, including in any ordinary trigger or `[Main]`.
  - It does not inspect the host.
- **Rule basis:** card-text idiom ("When a [Social], [Tool] or [Game] trait card would link to this Digimon"); timing per §10-1-2-3.
- **Production users:** BT25-045 Onmon and BT25-004 Tapmon (both Rebootmon). The clause and YAML are under `timing:when_would_link_to_this`; these are the only two users.
- **Existing near-miss tests:** index lists only `src/dsl_cards/predicate.rs`, the evaluator. Card coverage is in excluded `cards_behavioral/bt25/bt25_004.rs::bt25_004_does_not_reduce_non_social_tool_game_link`.
- **Proposed behavioural test** (in `tests/dsl/when_would_link_to_this.rs`):
  - **Fixture:** TEST-WLINK-HOST with `active_when: { would_link_card_trait_any_of: [Social, Tool] }` added.
  - **Linkers:** TEST-LINKER (`[Social]`); a `[Tool]`-only linker; a `[Beast]` linker (TEST-LINKER-2); a linker with trait `social` (lower case). Each has `link_condition { cost: 2, filter: { trait_has: LinkHost } }`.
  - **Positive:** `[Social]` and `[Tool]` (any-of) both get the optional prompt after the host pick.
  - **Negatives:**
    1. The `[Beast]` linker gets no prompt; memory goes down by 2.
    2. **Outside the window.** A second fixture `TEST-WLT-OUTSIDE` (on_play `condition: { would_link_card_trait_any_of: [Social] }`, `process: [gain_memory: 1]`) fired via OnPlay does not gain memory.
    3. Lower-case `social` gets no prompt. Pin it explicitly so a future case-folding change is deliberate; see defects.
- **Suspected engine/DSL defects:**
  1. Case-sensitive `==`, while card-subject `trait_has` uses `eq_ignore_ascii_case` (`predicate.rs:3396-3398`). This inconsistency is harmless today, since printed traits are canonical-cased.
  2. Only standing/hand Digimon links populate `pending_digimon_link`, so the predicate is always false for effect links and Plug-In Option links. Same root cause as `when_would_link_to_this` defect 1.

### `predicate:link_card_count`

- **Lowering site(s):**
  - Parse: `dsl/predicate.rs:260` (`Option<SourceCountPredicate { filter: Box<PredicateSpec>, at_least: u8 }>`, struct at `:1465`). Compile: `dsl/compile.rs:1053` (validator `dsl/validator.rs:871`). IR: `dsl/compiled.rs:471` (`Option<(Box<CompiledPredicate>, u8)>`).
  - Runtime, permanent subject: `eng/dsl_cards/predicate.rs:4535-4556` in `eval_permanent_fields` (`:4077`).
  - Runtime, breeding permanent: `:4888-4905`. It is always 0 there.
  - It is listed as permanent-only in `formula_eval.rs:1614` and `lower_replacement.rs:473`.
  ```rust
  if let Some((filter, at_least)) = &pred.link_card_count {
      let matching = perm.linked_cards.iter().filter(|s| eval_predicate_ref(filter, rctx,
              PredicateSubject::Card(s.handle()), bindings, DualRef::Digimon)).count();
      if matching < usize::from(*at_least) { return false; } }
  ```
- **YAML shape:**
  - BT25-085: `any_permanent: { of: you, zone: [battle_area], kind: digimon, link_card_count: { filter: { kind: option }, at_least: 1 } }`.
  - BT26-086: `count_gte: { filter: { zone: [battle_area], is_source: true, link_card_count: { filter: {}, at_least: 7 } }, n: 1 }`.
- **Semantics:**
  - A permanent-subject leaf: counts `Permanent.linked_cards` whose card satisfies the nested card filter (full combinator eval). True when that count >= `at_least`.
  - Link cards are separate from digivolution sources (§4-9-2). `source_count` is the sibling for sources.
  - An empty filter `{}` counts all link cards. Exact-N is authored as `>= N` and not `>= N+1` (BT26-086).
  - A breeding-area permanent has no links: false for `at_least >= 1`, true for `at_least: 0`.
  - The leaf is not consulted for a card subject (see defects).
- **Rule basis:**
  - §4-9-1: link card definition.
  - §4-9-2: not a stacked card.
  - §4-9-4: not on the field.
  - §4-9-5: link limit, which decides how many can exist (Link +, §16-40).
  - §4-10-1: linked card.
  - Otherwise a card-text idiom ("has 7 link cards", "Option card from … link cards").
- **Production users:**
  - **BT25-085 BeelStarmon** (Beelstarmon (3M)).
    - Printed: "[When Digivolving] [When Attacking] [Counter] [Once Per Turn] By trashing 1 Option card from any of your Digimon's digivolution cards or link cards, this Digimon unsuspends."
    - YAML: clause `condition: any_of: [ any_permanent{… source_count{kind: option, 1}}, any_permanent{… link_card_count: { filter: { kind: option }, at_least: 1 }} ]`.
  - **BT26-086 Dantemon** (Rebootmon). Printed: "…Then, if this Digimon has 7 link cards, return your opponent's top security card to the bottom of the deck." YAML as above, `at_least: 7` with a `count_lte … at_least: 8, n: 0` cap.
- **Existing near-miss tests:** `src/dsl_cards/formula_eval.rs`, `src/dsl_cards/lower_replacement.rs` and `src/dsl_cards/predicate.rs`. These are only permanent-only-leaf tables and the evaluator; no test.
- **Proposed behavioural test** (`tests/dsl/link_card_count.rs`):
  - **Fixture:**
    ```yaml
    card: TEST-LCC
    name: Link Card Count
    kind: tamer
    color: [red]
    cost: 0
    effects:
      - when: on_play
        process:
          - select_own_permanent:
              bind_as: pick
              filter: { all_of: [ { kind: digimon }, { link_card_count: { filter: { trait_has: Social }, at_least: 2 } } ] }
              prompt: "Pick a Digimon with 2+ [Social] link cards"
    ```
  - **Setup:**
    - Builder `.link_requirement("SOC-L", 0)` and `.link_requirement("BEAST-L", 0)`. Both are CardData Digimon, traits `[Social]` and `[Beast]`.
    - Hosts H2, H1 and H0 on P0's field, each `give_link_plus(h, 2)`.
    - Links: H2 gets two `push_linked_owned(.., "SOC-L", 0)`; H1 gets one SOC-L and one BEAST-L; H0 gets none.
    - `tick_declarative_effects()`, then place TEST-LCC and fire OnPlay.
  - **Positive:** the pending own-field pick's `valid_action_ids == [encode_attack(0, H2.idx)]`.
  - **Negatives:** H1 (only 1 match) and H0 are absent. A second fixture with `at_least: 3` installs no selection (`pending_selection.is_none()`). With `filter: {}` and `at_least: 2`, both H2 and H1 qualify.
  - **Selection surface:** the leaf surfaces nothing itself. Assert the candidate set only.
- **Suspected engine/DSL defects:**
  - On a `Card` subject (a hand, trash or source pick filter) the leaf is silently ignored. `eval_card_fields` has no `link_card_count` arm, unlike `has_face_down_source`, which is forced false for non-permanents at `predicate.rs:962-972`. So `select_hand { filter: { link_card_count: … } }` would vacuously match.
  - No production user does this, and it is unclear whether the validator rejects it. A `card_zone_unsupported_leaf` guard exists only in `formula_eval.rs:1614` for card-zone counts.

### `verb:reduce_link_cost`

- **Lowering site(s):**
  - Parse: `dsl/step.rs:499`, args `dsl/step.rs:1418` (`amount: u16`). Compile: `dsl/compile.rs:4411`. IR: `dsl/compiled.rs:2878`.
  - Runtime: `eng/dsl_cards/step/replacement_outcome.rs:177-184` → `EffectContext::reduce_pending_link_cost` `eng/effect_context/action/link.rs:53-60`. Consumed in `commit_digimon_link` `eng/game_actions/link.rs:656-661`.
  ```rust
  CompiledStep::ReduceLinkCost { amount } => { ctx.reduce_pending_link_cost(*amount); true }  // no outcome set
  pub fn reduce_pending_link_cost(&mut self, n: u16) -> bool {
      if let Some(p) = self.game.pending_digimon_link.as_mut() { p.cost = p.cost.saturating_sub(n); true } else { false } }
  // commit: effective = (p.cost as i32 + link_cost_delta_for_player(owner)).max(0); pay_memory(effective)
  ```
- **YAML shape:** `- reduce_link_cost: { amount: 1 }` inside a `when: when_would_link_to_this` clause (BT25-004 / BT25-045).
- **Semantics:**
  - A one-shot mutation of the in-flight player link's cost (`pending_digimon_link.cost -= amount`, saturating at 0). It does not set a replacement outcome, so the link proceeds, and it installs no persistent `ChangeLinkCost` modifier, so nothing leaks to later links.
  - Static `ChangeLinkCost` modifiers are added on top at commit.
  - Outside a `WhenWouldLink` window it is a silent no-op.
  - It surfaces no selection.
- **Rule basis:**
  - §10-1-2-3 / §10-1-3-2: the cost is paid after the would-link window.
  - §15-8-5: immediate-type.
  - Otherwise the card-text idiom "you may reduce the cost by N".
- **Production users:** BT25-045 Onmon and BT25-004 Tapmon (both Rebootmon), only. Clause text and YAML are under `timing:when_would_link_to_this`.
- **Existing near-miss tests:** index lists only `src/dsl_cards/step/replacement_outcome.rs`, the lowering. Card coverage is in excluded `cards_behavioral/bt25/bt25_004.rs` / `bt25_045.rs`.
- **Proposed behavioural test** (in `tests/dsl/when_would_link_to_this.rs`):
  - **Fixture:** TEST-WLINK-HOST from above, in variants `amount: 1`, `amount: 5` (saturation), and a copy gated `active_when: { would_link_card_trait_any_of: [Social] }`.
  - **Setup:** linker cost 2, memory 5.
  - **Positive:** accept gives memory 4 (2 - 1). With `amount: 5`, accept gives memory 5 (saturates at 0, never negative or refund).
  - **Negatives:**
    1. **No leak.** Using the `[Social]`-gated host, link the `[Social]` linker (accept → pays 1), then `give_link_plus(host, 1)` and link a `[Beast]` linker. The second link pays the full 2: the first reduction did not persist.
    2. **Outside window.** A `TEST-RLC-OUTSIDE` fixture (on_play `process: [reduce_link_cost: { amount: 1 }]`) fired via OnPlay changes nothing and doesn't panic. A later player link at full cost still pays 2.
    3. **Interaction.** With a static `ChangeLinkCost` -1 modifier present (if easily added), accept gives 2 - 1 - 1 = 0.
- **Suspected engine/DSL defects:**
  1. Silent no-op outside the window. There is no validator rule tying `reduce_link_cost` to a `when_would_link_to_this` clause, so a mis-authored card would compile and do nothing.
  2. It inherits the window-coverage gap (effect links and Plug-In links can't be reduced). See `when_would_link_to_this` defect 1.

### `verb:app_fuse`

- **Lowering site(s):**
  - Parse: `dsl/step.rs:494`, args `dsl/step.rs:2993` (`from: hand|trash`, `result_filter`, `optional`). Compile: `dsl/compile.rs:3527`. IR: `dsl/compiled.rs:2174`.
  - Runtime step: `eng/dsl_cards/step/play_digivolve.rs:1172-1179`.
  - Orchestrator: `eng/effect_context/action/app_fuse.rs:55-191` (`initiate_effect_app_fuse`). Resume steps: `:193-238`. Result pick: `:242-300`.
  - Commit: `eng/game_actions/digivolve.rs:2030-2140` (`Game::commit_effect_app_fuse -> bool`).
  - Declinability: `eng/dsl_cards/lower_triggered.rs:834` (an `optional: true` app_fuse counts as the clause's decline path, so no outer prompt).
  ```rust
  CompiledStep::AppFuse { from_zone, result_filter, optional } => {
      ctx.initiate_effect_app_fuse(*from_zone, result_filter.as_ref(), *optional);
      true                                     // <- no outcome reported to the body/bindings
  }
  // initiate: result cards in zone ∩ result_filter; per own standing Digimon permanent, fusable =
  //   results.filter(|cs| game.can_app_fuse_onto(cs, host)); none => silent return
  // selection #1: SelectionKind::Target over encode_attack(controller, idx), is_optional: optional, on_decline: None
  // selection #2: SelectionKind::Target over HAND_EFFECT_START+i / TRASH_EFFECT_START+i, is_optional: optional
  //   callback: game.commit_effect_app_fuse(controller, host, card, zone);   // bool return dropped
  // commit: CannotDigivolve gate; take result from zone; host.digivolve(card); drain host.linked_cards
  //   (reversed) under the new top as sources; GameEvent::Digivolve{memory_paid:0};
  //   fire_digivolve_event_triggers(host, top, true, false)   // no memory paid, no draw
  ```
- **YAML shape:**
  - BT25-089: `- app_fuse: { from: hand, optional: true }`.
  - BT24-087: `{ from: trash, optional: true, result_filter: { any_of: [ { trait_has: System }, … ] } }`.
- **Semantics:**
  - An effect-initiated App Fuse surfacing two real, resume-driven selections:
    1. one of the controller's standard battle-area Digimon onto which some zone card (passing `result_filter`) can app-fuse, i.e. the host top plus a linked card match two distinct named App-Fusion materials of that card's `alt_paths: kind: app_fusion`;
    2. the result card, from among those eligible for the picked host.
  - Both picks are `SelectionKind::Target` with `is_optional = optional`. PASS ends the step silently (`on_decline: None`).
  - No eligible pair: silent no-op, no prompt.
  - Commit:
    - The result card is stacked on top.
    - All the host's link cards go under it as digivolution sources.
    - No memory is paid.
    - Digivolve triggers fire (effect-initiated).
  - The step reports nothing back to the DSL body. Nothing distinguishes "fused", "passed at pick 1", "passed at pick 2" and "nothing eligible".
- **Rule basis:**
  - §8-4-1: App fusion definition.
  - §8-4-2-1: a combination of 2 different specified cards.
  - §8-4-2-2: by effect only via effects that specifically perform app fusion.
  - §8-4-2-3: digivolution modifiers apply.
  - §8-4-3-1..3: procedure: choose requirement and Digimon, pay the specified cost, place the link card on top per the requirement, place the App Fusion card on top, **draw 1**.
  - §15-14-1-4 and official Q1818: a declined "may" [Once Per Turn] keeps its use (this governs the gap).
- **Production users:**
  - **BT25-089 Kazuki & Itsuki** (Rebootmon).
    - Printed: "[End of Your Turn] [Once Per Turn] 1 of your Digimon may app fuse into a Digimon card in the hand."
    - YAML: `when: end_of_your_turn, once_per_turn: true, process: [ app_fuse: { from: hand, optional: true } ]`.
    - Verdict: PARTIAL for this clause (`qa/qa-reports/validated_cards_dsl.json` BT25-089).
  - **BT21-084 Haru Shinkai** (Rebootmon). Printed: "…by suspending this Tamer, <Draw 1>. Then, 1 of your Digimon may app fuse into a Digimon card in the hand." YAML tail is `app_fuse: { from: hand, optional: true }`.
- **Existing near-miss tests:**
  - The index lists only the lowerings (`src/dsl_cards/lower_triggered.rs`, `step/play_digivolve.rs`). The real mechanics tests are outside scanned roots, so they earn no credit:
    - `tests/cards_behavioral/app_fuse_primitive.rs`: 8 tests driving inline `app_fuse` YAML fixtures — hand / trash happy paths, `result_filter`, ineligible host not offered, no eligible = no-op, decline at either pick, two-distinct-material requirement, clone faithfulness. It sits in the excluded `cards_behavioral` root.
    - `tests/app_fuse_commit.rs`: a top-level binary for the zone-parameterized commit, not in `SCAN_GLOBS`.
  - Porting `app_fuse_primitive.rs` into `tests/dsl/app_fuse.rs` is the cheapest path to credit.
- **Proposed behavioural test** (`tests/dsl/app_fuse.rs`; port of `cards_behavioral/app_fuse_primitive.rs`):
  - **Fixture:** the `SRC_HAND`, `SRC_TRASH` and `SRC_FILTER` YAMLs plus the `TEST-APPFUSE` result card (`alt_paths: [{ kind: app_fusion, materials: [{ filter: { name_in: [Kabemon, Gomimon] } }], cost: 0 }]`) from that file.
  - **Setup:** builder `.link_requirement("TEST-GOMIMON", 0)`. Host = `place_on_field("TEST-KABEMON")` + `push_linked_owned(host, "TEST-GOMIMON", 0)`. `.hand(0, &["DSL-APPFUSE-HAND", "TEST-APPFUSE"])`, then `r.play(0, idx)`.
  - **Positive:**
    - Pending `Target` with `is_optional == true` contains `encode_attack(0, host.idx)`. Pick it.
    - Pending `Target` contains `HAND_EFFECT_START + idx(TEST-APPFUSE)`. Pick it.
    - The top card is TEST-APPFUSE, `linked_cards` is empty and TEST-GOMIMON is in `card_sources`. Memory is unchanged.
  - **Negatives:**
    1. A host whose link is not a named material is not offered.
    2. With only one material, no selection installs.
    3. PASS at pick 1 or pick 2 leaves the host and hand unchanged.
    4. `result_filter` excludes a non-matching trash card.
  - **Selection surface:** both picks are real `pending_selection`s with `is_optional` == the YAML `optional`, and `pending_selection_resume.is_some()` at each.
  - **Do NOT** write a test that asserts the [Once Per Turn] is refunded after a PASS. That is the KNOWN GAP below; the plan routes it to a substrate task.
- **KNOWN substrate gap (BT25-089 PARTIAL; `docs/RUST_ENGINE_GAPS.md` "Remaining, blocks training: yes: BT25-089#2"):**
  - **Missing signal.** The DSL clause has no way to learn whether the app fuse executed. It cannot tell "fused" apart from "declined at the permanent pick", "declined at the result pick" or "no eligible pair (silent no-op)". Without that, a whole-effect "may" [Once Per Turn] cannot be refunded on a pass. DCGO `BT25_089.cs` calls `RemoveUse` when nothing executed (Q1818, §15-14-1-4). Neither the folded-may rewrite (`lower_triggered::then_may_refund_*`) nor an authored `if binding_absent → refund_opt` can see a pass.
  - **Where it is lost.** Three places:
    1. `CompiledStep::AppFuse` returns `true` immediately after installing selection #1 and writes no binding (`play_digivolve.rs:1172-1179`).
    2. Both selections have `on_decline: None` (`app_fuse.rs:174`, `:282`). A PASS never re-enters the DSL body with an outcome, and `AppFuseHostSelectionState` / `AppFuseResultSelectionState` carry `outer_conts` but no binding writer.
    3. The commit's `bool` from `Game::commit_effect_app_fuse` is dropped in both the closure callback (`app_fuse.rs:271-278`) and the resume step `run_app_fuse_result_selection_step` (`app_fuse.rs:225-237`).
  - **Where it would be produced.** The result-pick resolution (`run_app_fuse_result_selection_step` and its closure twin) knows success. The PASS paths (selection #1 / #2 decline) and the early `return`s in `initiate_effect_app_fuse` (`:100-102`, `:131-133`) know "nothing executed". A substrate fix would write an outcome into the clause's `Bindings` (for example an `AppFuseArgs.bind_as` receipt holding the fused permanent, absent on pass or no-op) from those sites through the resume frames' continuation. It would then let `body_first_step_is_declinable` / the folded-may refund treat an `optional` app_fuse like an optional pick.
- **Suspected engine/DSL defects (beyond the known gap):**
  1. **No draw on effect App Fuse.** §8-4-3-3 says "places it on top of the Digimon card to app fuse, draws 1 card". `commit_effect_app_fuse` never draws, while `effect_initiated_digivolve` explicitly draws (`digivolve.rs:3106-3113`, "the draw belongs to the PROCEDURE"). Verify against DCGO `SetAppFusion` before fixing.
  2. **App Fusion cost assumed 0.** "App Fusion is Cost 0 — no memory payment" (`digivolve.rs:2090`) ignores the `alt_paths` `cost:` and §8-4-3-2 "the digivolution cost … is paid". All current printings are Cost 0, so this is latent.
  3. **All link cards consumed.** The commit drains every `linked_cards` entry under the new top. §8-4-1 / §8-4-3-3 place only the link card of the chosen material pair. A host with `<Link +N>` holding a non-material link card would lose it into the sources. Uncertain; needs a ruling.
