# Construct semantics: cost-target-and-event-predicates

- **Family:** cost-target-and-event-predicates
- **Date:** 2026-10-22
- **Worktree read:** `D:/wt/p3-constructs` @ f24f26dfa (read-only; nothing built or run)
- **Constructs, in order:**
  1. `predicate:cost_target`
  2. `predicate:source_is_cost_target_permanent`
  3. `verb:trash_selected_sources_as_cost`
  4. `predicate:binding_count_eq`
  5. `predicate:binding_card_kind`
  6. `predicate:name_not_in_binding`
  7. `predicate:effect_deleted_any_own_digimon`
  8. `predicate:effect_deleted_any_opponent_digimon`
  9. `predicate:event_added_card_any`
  10. `predicate:event_target_name_contains`
  11. `timing:on_add_digivolution_cards`
  12. `predicate:during_attack`
  13. `timing:start_of_opponents_main_phase`

## Conventions

**Paths.** Paths are relative to `code/` unless they begin with `docs/`, `data/` or `qa/`. "DSL" means
`digimon-dsl/src` and "engine" means `digimon-engine/src`.

**Where the tests go.** Every proposed test belongs in `digimon-engine/tests/dsl/`. Use one file per construct, or one
family file such as `tests/dsl/cost_target_and_event_predicates.rs`. Each test drives the engine through
`DebugRunner::builder().from_dsl_yaml(..)`.

**Digivolve fixtures need no `EvoCost`.** `from_dsl_yaml` builds `evo_costs` itself from any `kind: digivolve`
alt-path whose `from:` has both `level_eq` and `color_is` (`debug_runner.rs:1421-1465`).

**Correction to the dispatch brief: what "cost target" means.**
- The brief said the "cost target" predicates read the permanent chosen while paying a "By <doing X>" optional
  processing condition (the `cost: true` selections). They do not.
- In this engine, the "cost target" is the **card whose play, digivolve or use cost is being calculated**. It is
  visible while BeforePayCost reducers and observers run.
- That is the immediate-type "When this Digimon would digivolve into X, reduce the cost by N" family (CR 4.3
  §15-8-5-1).
- The `cost: true` flag on selection steps is a different mechanism: the activated-cost refund rule in
  `lower_triggered.rs`.
- Sections 1 and 2 describe what the code actually does.

**Shared observables used in the tests below.**
- **Memory sign.** `r.memory()` returns `game.memory`, which is signed from the **turn player's** side
  (`game/memory.rs:120-126`). A gain by the non-turn player therefore *lowers* it. When the acting player is not the
  turn player, assert on hand size instead.
- **Firing a triggered clause.** Pattern from `tests/dsl/effect_battle.rs`:
  `r.game.enqueue_triggered(EffectTiming::OnPlay, TriggerSource::Permanent(h)); r.game.drain_effect_queue();`
- **Resolving a choice.** Call `r.execute_action(sel.selecting_player, action)`. The helper `choose(r, decline)` in
  `tests/dsl/battle_deleter.rs:178` picks the first non-PASS id, or PASS when `decline` is true.
- **Suspending a permanent.** `r.game_mut().suspend(h)` (`game/suspend.rs:26`) fires `OnSuspend` (see
  `tests/timing_dispatch.rs:1211`).

---

## 1. `predicate:cost_target`

### Lowering sites
- **Parse:** `DSL/predicate.rs:1067` (`pub cost_target: Option<Box<PredicateSpec>>`).
- **Compile:** `DSL/compile.rs:1560`.
- **IR:** `DSL/compiled.rs:807`.
- **Runtime:** `engine/dsl_cards/predicate.rs:597-611`.
  ```rust
  if let Some(inner) = &pred.cost_target {
      let Some(target) = rctx.cost_target_card else {
          return false;
      };
      if !eval_predicate_with_bindings(inner, rctx, PredicateSubject::Card(target), bindings) {
          return false;
      }
  }
  ```
- **Where the target comes from.**
  - The leaf reads `EffectReadContext.cost_target_card` (`engine/effect_context/mod.rs:144`).
  - That field is set **only** by `new_with_cost_target` (`mod.rs:212-238`).
  - The value comes from a `CostTargetContext { card, from_hand, is_digivolve, target_permanents }`
    (`engine/game_actions/mod.rs:321-342`).
- **Callers of `new_with_cost_target`:**
  - `game_actions/cost.rs`:
    - `inspect_cost_reduction_candidate` (lines 1324-1380): evaluates the reducer's `condition` and its
      `cost_reduction_fn`.
    - `apply_cost_reduction_candidate` (lines 1424-1433): runs `pay_cost`.
    - The digivolve reducer chain (lines 560-569).
    - The observers (about lines 1537-1563).
  - `options.rs:858`: the Option use cost.
  - `play_cost_assignment.rs:154`.

### YAML shape
From EX12-020 (`cards/ex12/EX12-020.yaml`):
```yaml
- kind: cost_reduction
  reduction_timing: before_pay_cost
  active_when:
    all_of:
      - your_turn: true
      - source_is_cost_target_permanent: true
      - cost_target:
          all_of:
            - kind: digimon
            - trait_has: TB
  amount: 1
```

### Semantics
- **What it tests.** A subject-free leaf. It evaluates the inner predicate as a **Card** subject against the card
  whose cost is being computed. That card is:
  - the card being played, or
  - the card being digivolved *into*, or
  - the Option being used.
- **Outside cost calculation it is always false.** Ordinary read contexts have `cost_target_card = None`.
- **Nothing is stored or cleared.** The target is not kept on `Game`. A fresh context is built from the in-flight
  `CostTargetContext` for each reducer evaluation, then dropped.
- **Edge case: interactive reducers that park.**
  - When an interactive reducer's `pay_cost` parks a selection, it resumes under a context rebuilt from
    `ResumeProvenance`.
  - That rebuilt context has no cost target, so a `cost_target` leaf placed **after the park** inside `pay_cost`
    would read false.
  - No current card does this.
- **Inner predicate.**
  - It receives the clause `bindings`, which are normally empty.
  - Any card-shape leaf works: kind, trait, colour, name, level.
  - Because the subject is the printed card in hand or trash, permanent-only leaves fail.

### Rule basis
- **§15-8-5-1.** An immediate-type effect such as "When this Digimon would digivolve into a card with [Greymon] in its
  name, reduce the digivolution cost…" interrupts right before the cost is paid.
- **§4-2-3-1 and §1-3-11-2.** A reduced cost becomes an alternate cost, and the declaration is legal only if that
  alternate cost can be paid.

### Production users
Toho Braves blocks on this construct.
- **EX12-020 Gasamon (Toho Braves).** Printed: "[Your Turn] When this Digimon would digivolve into a Digimon card with
  the [TB] trait, reduce the cost by 1." YAML is shown above.
- **EX12-039 Takinmon (Toho Braves).** Printed: "[Your Turn] When this Digimon would digivolve into a Digimon card
  with the [SW] trait, reduce the cost by 1." Same shape, with
  `cost_target: { all_of: [ {kind: digimon}, {trait_has: SW} ] }`.

### Existing near-miss tests
- The coverage index lists only `src/dsl_cards/predicate.rs`, which is the evaluator itself, not a test.
- `src/game_actions/digivolve.rs` (`q1157_ordinary_cost_state_tests`) is a lib unit test, so it does not count. It
  also uses `when_any_ally_digivolves_into`, not `cost_target`.

### Proposed behavioural test
**Fixture.** This version isolates the leaf, without the source gate.
```yaml
card: TST-CT-REDUCER
name: Cost Target Reducer
kind: tamer
color: [red]
cost: 2
effects:
  - kind: cost_reduction
    reduction_timing: before_pay_cost
    active_when:
      all_of:
        - your_turn: true
        - cost_target: { trait_has: Probe }
    amount: 1
    summary: "[Your Turn] When you would pay for a [Probe] card, reduce the cost by 1"
```

Supporting cards:

| Card | Kind | Details |
|---|---|---|
| `TST-CT-BASE` | Lv.3 red Digimon | cost 3, no effects |
| `TST-CT-EVO-PROBE` | Lv.4 red Digimon | cost 5, `traits: [Probe]`, `alt_paths: [{kind: digivolve, from: {level_eq: 3, color_is: red}, cost: 3}]` |
| `TST-CT-EVO-PLAIN` | Lv.4 red Digimon | same as above, no traits |
| `F` | filler | via `make_test_card` |

**Setup.**
1. Builder: `.from_dsl_yaml(...)` for each fixture, then `.hand(0, &["TST-CT-EVO-PROBE","TST-CT-EVO-PLAIN"])`,
   `.deck(0, &["F"; 10])`, `.deck(1, &["F"; 10])`, `.memory(5)`, `.start()`.
2. Set `r.game.current_phase = GamePhase::Main`.
3. `place_on_field(0, "TST-CT-REDUCER", Some(0))`.
4. `let base = place_on_field(0, "TST-CT-BASE", Some(0))`.
5. `r.game.tick_declarative_effects()`.

**Positive case.**
- Call `r.game.digivolve_from_hand(0, 0 /*PROBE*/, base.index as usize, PlaySource::ByDigivolve)`. It returns true.
- Assert `pending_selection.is_none()`: the reducer is mandatory, so there is no prompt.
- Assert `r.memory() == 5 - 2`.

**Negative cases.**
- In a fresh runner, digivolve `TST-CT-EVO-PLAIN` instead. Assert `r.memory() == 5 - 3`.
- Play a `[Probe]` Digimon from hand twice:
  - with the reducer **absent**, confirm the full play cost is paid;
  - with the reducer present, confirm the cost is reduced by 1.
  - This shows the leaf also reads **play** costs.

**Selection surface.**
- None: a mandatory reducer must not park.
- If the clause is made `optional: true`, assert `pending_selection.unwrap().is_optional`.

### Suspected engine/DSL defects
- None seen for the leaf.
- Note: `reduction_timing` is checked only as `None | "before_pay_cost"` (`engine/dsl_cards/mod.rs:347-360`).

---

## 2. `predicate:source_is_cost_target_permanent`

### Lowering sites
- **Parse:** `DSL/predicate.rs:1079` (`Option<bool>`).
- **Compile:** `DSL/compile.rs:1568`.
- **IR:** `DSL/compiled.rs:813`.
- **Runtime:** `engine/dsl_cards/predicate.rs:613-618`.
  ```rust
  if let Some(want) = pred.source_is_cost_target_permanent {
      if rctx.source_is_cost_target_permanent() != want {
          return false;
      }
  }
  ```
  It calls `engine/effect_context/mod.rs:259-264`:
  ```rust
  pub fn source_is_cost_target_permanent(&self) -> bool {
      let Some(source) = self.source_permanent else { return false; };
      self.cost_target_permanents.iter().any(|h| *h == source)
  }
  ```
- **Where `cost_target_permanents` comes from.** `CostTargetContext::target_permanents_vec()`
  (`game_actions/mod.rs:335-341`). It holds:
  - one handle for a normal digivolve;
  - both materials for a DNA digivolve;
  - nothing (**empty**) for a play from hand or an Option use.

### YAML shape
From BT14-044 Palmon (inherited):
```yaml
- kind: cost_reduction
  scope: inherited
  reduction_timing: before_pay_cost
  active_when:
    all_of:
      - your_turn: true
      - source_is_cost_target_permanent: true
      - any_permanent: { of: you, zone: [battle_area], kind: tamer, color_is: green }
  once_per_turn: true
  amount: 1
```

### Semantics
- **True only during cost calculation**, and only when the effect's carrier permanent (`source_permanent`) is the
  permanent being digivolved. That covers both:
  - a face-up effect on that permanent's top card (EX12-020);
  - an **inherited** effect inside that permanent's stack (BT14-044).
- **False in every other case:**
  - a play from hand;
  - an Option use;
  - an effect with no source permanent (carriers in hand or trash);
  - any time outside cost calculation.
- **`false` can be asserted explicitly:** `source_is_cost_target_permanent: false`.
- **Usage.** In practice it is always paired with `cost_target`. All 21 `cost_target` users also use this leaf
  (`qa/impact/lowering_coverage.json`).

### Rule basis
- **§15-8-5-1.** The "When **this** Digimon would digivolve" immediate timing.
- **§4-2-3-1.** Alternate cost.

### Production users
Toho Braves and Medusamon block on this construct.
- **EX12-020 Gasamon (Toho Braves).** YAML is in section 1.
- **BT14-044 Palmon.** Printed: "[Your Turn][Once Per Turn] When this Digimon would digivolve, if you have a green
  Tamer, reduce the digivolution cost by 1." Inherited; YAML is shown above.
- Medusamon's user is BT23-005.

### Existing near-miss tests
- `src/dsl_cards/predicate.rs` is the evaluator.
- `tests/cost_hooks/player_digivolve_reducer.rs` (`IdentityPaymentReducer`, about lines 955-1010) calls the Rust API
  `ctx.source_is_cost_target_permanent()` from a hand-written `CardEffect`. It exercises the context method, never the
  DSL leaf, so it earns no credit.

### Proposed behavioural test
**Fixtures.**
- `TST-SICTP-BASE`: a Lv.3 red Digimon, cost 3, carrying the same clause as `TST-CT-REDUCER` but with
  `active_when: { all_of: [ {your_turn: true}, {source_is_cost_target_permanent: true} ] }` and `amount: 1`.
- `TST-CT-BASE`: a plain Lv.3 red Digimon.
- `TST-CT-EVO-PLAIN`: Lv.4, digivolves from red Lv.3 for cost 3. Put two copies in hand.

**Setup.**
1. Main phase, memory 10.
2. Place `TST-SICTP-BASE` (index 0) and `TST-CT-BASE` (index 1).
3. Call `tick_declarative_effects()`.

**Positive case.** Digivolve hand[0] onto index 0. Assert memory drops by 2.

**Negative cases.**
1. Different permanent: digivolve onto index 1. Assert memory drops by 3, because the reducer on index 0 must not
   apply.
2. A play, not a digivolve: play a plain cost-3 Digimon from hand. Assert memory drops by 3, because
   `target_permanents` is empty.

**Optional inherited variant.**
- Give the reducer card `scope: inherited`.
- Build a stack with the reducer under a plain top: `place_stack(0, &["TST-SICTP-BASE", "TST-CT-BASE"])`.
- Digivolving that stack is reduced.
- Digivolving a different stack, whose sources do not include the reducer, is not.

**Selection surface.** None; the reducer is mandatory.

### Suspected engine/DSL defects
- None seen.
- This predicate does not distinguish DNA from normal digivolution. DNA-only wording (BT12-022) therefore needs the
  extra `dna_origin: true` leaf, and BT12-022's YAML adds it correctly.

---

## 3. `verb:trash_selected_sources_as_cost`

### Lowering sites
- **Parse:** `DSL/step.rs:215`.
- **Arguments:** `DSL/step.rs:2524-2528` (`source_refs: String`, `min_count: Option<u8>`).
- **Compile:** `DSL/compile.rs:3245`.
- **IR:** `DSL/compiled.rs:2689`.
- **`<Digi-Burst N>` sugar.** It lowers to `select_own_sources {target: source, min: N, max: N}`, with this step first
  in its `then:` (`DSL/compile.rs:4046-4060`).
- **Runtime:** `engine/dsl_cards/step/zone_moves.rs:276-302`.
  ```rust
  let Some(selected) = bindings.get_source_refs(source_refs) else {
      ctx.cost_unpayable = true; return true; };
  let required = min_count.map(usize::from).unwrap_or(selected.len());
  if selected.len() < required { ctx.cost_unpayable = true; return true; }
  let event = ctx.game.begin_source_move_event();
  let paid: Vec<_> = selected.into_iter()
      .filter(|source| ctx.trash_card_source(source.permanent, source.card)).collect();
  ctx.game.end_source_move_event(event);
  ctx.cost_unpayable = paid.len() < required;
  bindings.insert_source_refs(source_refs, paid);
  ```
- **How an unpaid cost aborts the rest of the clause.**
  - **Later steps in the same `then:`:** `step/mod.rs:831` returns immediately after any step that set
    `cost_unpayable`.
  - **Steps after the `select_own_sources`:** `run_source_multi_terminal` (`selections.rs:3143`) skips the outer
    continuations when `cost_unpayable` is set.
- **Candidate pre-filter (the §15-7-4 gate).**
  - It applies when the picker's tail starts with this verb (`source_selection_pays_trash_cost`,
    `selections.rs:3025-3030`).
  - Sources whose trashing is blocked (`source_card_trash_blocked`) are then not offered (`selections.rs:3091-3095`,
    `8780`, `8814-8816`).
  - This is the "can't choose what can't be done" gate.

### YAML shape
From EX7-073 BeelStarmon (X Antibody):
```yaml
- select_own_sources:
    from: source
    min: 0
    max: 2
    filter: { trait_has: "Three Musketeers" }
    bind_as: trashed_sources_wd
    then:
      - trash_selected_sources_as_cost: { source_refs: trashed_sources_wd, min_count: 2 }
      - if: { condition: { binding_count_eq: { binding: trashed_sources_wd, n: 2 } }, then: [...] }
```

### Semantics
- **Purpose.** Pays a "by trashing N of this Digimon's digivolution cards" cost from a `select_own_sources`
  source-ref binding.
- **When nothing is paid.** If the binding is missing, or holds fewer than `min_count` cards, nothing moves and the
  clause aborts. A short pick counts as a decline (§15-7-3).
- **Successful payment.**
  - Every picked card is trashed in **one** source-move event (§15-5-2).
  - The binding is then **rewritten to hold only the cards actually trashed**.
- **Partial refusal.** If any trash is refused and the survivors fall below `required`, the clause aborts, but the
  cards that did go have already been trashed.
- **Omitting `min_count`.**
  - `required` becomes `selected.len()`.
  - So an **empty** pick (`min: 0`, then PASS) counts as a successful zero-card payment, and the tail runs.
  - Authors must set `min_count` to the printed N, or to `0` only for "up to" wording.

**Selection surface.**
- `select_own_sources` installs `SelectionKind::SourceMulti`.
- PASS is offered only once `picked >= min` (`effect_context/selections.rs:2938-2962`). So at the first pick,
  `is_optional == (min == 0)`.
- With zero candidates and `min == 0`, the tail runs immediately with an empty list.
- With zero candidates and `min > 0`, nothing runs.

### Rule basis
- **§15-7-1 and §15-7-2.** In "By X, Y", Y happens only if X was executed.
- **§15-7-3.** No partial payment.
- **§15-7-4.** A cost that cannot be paid cannot be chosen.
- **§15-5-2.** Simultaneous trashes are one trigger event.
- **§16, `<Digi-Burst>`.** Governs the sugar.

### Production users
Beelstarmon (3M) blocks on this construct.
- **EX7-073 BeelStarmon (X Antibody).** Printed: "[When Digivolving] [When Attacking] By trashing 2 cards with the
  [Three Musketeers] trait in this Digimon's digivolution cards, delete 1 of your opponent's Digimon with the highest
  level and trash the top card of your opponent's security stack." YAML is shown above.
- **ST4-13 HerculesKabuterimon.** Printed: "[Main] <Digi-Burst 2> ・Suspend 1 of your opponent's Digimon."
  - YAML:
    `select_own_sources: {from: source, min: 2, max: 2, bind_as: burst_sources, then: [ {trash_selected_sources_as_cost: {source_refs: burst_sources}} ]}`
  - That is followed, **outside** `then:`, by `select_opponent_permanent` and `suspend`.

### Existing near-miss tests
- `src/dsl_cards/step/selections.rs` and `src/dsl_cards/step/zone_moves.rs` are the implementation, not tests.
- No test file names the verb.

### Proposed behavioural test
**Fixture.**
```yaml
card: TST-TSSC
name: Source Cost Probe
kind: digimon
level: 5
color: [red]
cost: 5
dp: 5000
effects:
  - when: on_play
    process:
      - select_own_sources:
          target: source
          min: 0
          max: 2
          filter: { trait_has: Fuel }
          bind_as: paid
          prompt: "Trash 2 [Fuel] digivolution cards"
          then:
            - trash_selected_sources_as_cost: { source_refs: paid, min_count: 2 }
            - draw: { of: you, count: 1 }
      - gain_memory: 1
```
Sources:
- `FUEL`: `make_test_card` with `traits = vec!["Fuel"]`.
- `PLAIN`: no traits.

**Setup.**
1. `let h = r.place_stack(0, &["FUEL","FUEL","PLAIN","TST-TSSC"])`. The last id is the top card.
2. Deck of 10 fillers, `memory(0)`.
3. Enqueue `OnPlay` on `h` and drain.

**Selection surface.**
- `pending_selection.kind` is `SourceMulti { min: 0, max: 2, .. }`.
- `is_optional == true`.
- `valid_action_ids` holds 3 ids: the 2 FUEL sources plus PASS. PLAIN is filtered out.

**Positive case.** Pick both FUEL ids; the second pick re-parks with a new candidate list. Then assert:
- the stack is 2 cards (`PLAIN` + top);
- trash +2;
- hand +1, so the inner `draw` ran;
- `r.memory() == 1`, so the outer continuation ran.

**Negative cases.**
- **Short pick.** Pick 1 FUEL, then PASS. Assert:
  - the stack is still 4;
  - trash and hand are unchanged;
  - memory is 0, so the inner tail **and** the outer `gain_memory` were both aborted.
- **Decline.** PASS immediately. Same assertions.

**Optional extra.** Add a `when: on_digivolution_card_trashed` observer fixture with
`trigger_if: { event_host_permanent_is_source: true }`. It should fire **once** for the two-card payment (§15-5-2).

### Suspected engine/DSL defects (read-only)
1. **Partial payment can execute.**
   - If one of several picked sources is refused at trash time, the others have already been trashed when
     `cost_unpayable` aborts.
   - §15-7-3 forbids partial payment.
   - The candidate pre-filter makes this rare, but the trash loop is not all-or-nothing.
2. **A forced no-op pick.**
   - This happens when there are fewer eligible sources than `min`, for example ST4-13 (`min: 2`) with one source
     trash-blocked.
   - The installer offers a **mandatory** pick with no PASS.
   - After the pick, the clause silently does nothing (`selections.rs` 2929-2933 and 3223-3226).
   - §15-7-4 says the cost should not be choosable at all. No harm is done, but a meaningless decision is surfaced to
     the policy.
3. **Footgun.** With `min_count` omitted and `min: 0`, an empty pick is treated as paid. No production card does this
   today; all 14 users were checked.

---

## 4. `predicate:binding_count_eq`

### Lowering sites
- **Parse:** `DSL/predicate.rs:949`.
- **Payload:** `DSL/predicate.rs:1537-1542` (`{ binding: String, n: u8 }`).
- **Compile:** `DSL/compile.rs:1291`.
- **IR:** `DSL/compiled.rs:612` (`Option<(String,u8)>`).
- **Runtime:** `engine/dsl_cards/predicate.rs:373-387`.
  ```rust
  if let Some((name, want)) = &pred.binding_count_eq {
      let count = bindings.and_then(|b| b.get_ref(name)).map(|value| match value {
          BindingValue::PermanentList(v) => v.len(),
          BindingValue::CardList(v) => v.len(),
          BindingValue::SourceRefs(v) => v.len(),
          _ => 1,
      }).unwrap_or(0);
      if count != usize::from(*want) { return false; }
  }
  ```
- **Synthesized use.** `lower_triggered.rs:725-735` builds `binding_count_eq: (b, 0)` leaves for the "then-may"
  Once Per Turn refund.

### YAML shape
From EX4-073: `binding_count_eq: { binding: trashed_sources, n: 3 }`.

### Semantics
An exact count of a named binding.

| Binding state | Count |
|---|---|
| list binding (permanent list, card list, source refs) | number of entries |
| any **single** binding (one permanent, card or literal) | 1 |
| **absent** binding | 0, so `n: 0` is true when nothing was bound |
| bindings unavailable (for example an `active_when` that the engine evaluates with `None` bindings) | 0 |

`trash_selected_sources_as_cost` rewrites its binding to the cards actually trashed, so a following
`binding_count_eq` counts real removals.

### Rule basis
- A card-text-only idiom: "if you trashed 3 cards", "if this effect placed 2".
- **§15-6-2.** A processing condition gates only its own process.

### Production users
Beelstarmon (3M), Chronomon, Toho Braves and Medusamon block on this construct.
- **EX4-073 Omnimon Alter-B.**
  - Printed: "…By trashing up to 3 level 6 or higher cards in this Digimon's digivolution cards, for each card
    trashed, activate the effect below. Then, if you trashed 3 cards, trash the top 2 cards of your opponent's
    security stack."
  - YAML:
    `- if: { condition: { binding_count_eq: { binding: trashed_sources, n: 3 } }, then: [ {trash_top_security: {of: opponent}}, {trash_top_security: {of: opponent}} ] }`
- **EX7-073 BeelStarmon (X Antibody) (Beelstarmon deck).** Uses `binding_count_eq: { binding: trashed_sources_wd,
  n: 2 }` after the cost. YAML is in section 3.

### Existing near-miss tests
- `tests/dsl/folded_may_opt_refund.rs` (`then_may_clause_refunds_when_both_halves_are_empty`, about line 230) only
  checks that the **lowering** inserts `binding_count_eq: (b, 0)` leaves into the compiled process. The engine is
  never run.
- `src/dsl_cards/lower_triggered.rs` and `src/dsl_cards/predicate.rs` are source files.

### Proposed behavioural test
**Fixture.**
```yaml
card: TST-BCE
name: Binding Count Probe
kind: digimon
level: 4
color: [red]
cost: 4
dp: 4000
effects:
  - when: on_play
    process:
      - select_opponent_permanent:
          bind_as: victim
          optional: true
          continue_on_decline: true
          filter: { kind: digimon }
          prompt: "You may choose 1 of your opponent's Digimon"
      - if:
          condition: { binding_count_eq: { binding: victim, n: 1 } }
          then:
            - draw: { of: you, count: 1 }
      - if:
          condition: { binding_count_eq: { binding: victim, n: 0 } }
          then:
            - gain_memory: 1
      - select_own_sources:
          target: source
          min: 0
          max: 2
          bind_as: picked
          prompt: "Choose up to 2 digivolution cards"
          then:
            - if:
                condition: { binding_count_eq: { binding: picked, n: 2 } }
                then:
                  - gain_memory: 2
```

**Setup.**
1. `place_stack(0, &["F","F","TST-BCE"])`.
2. One opponent Digimon: `place_on_field(1, "F", Some(0))`.
3. `memory(0)`, then enqueue `OnPlay` and drain.

**Positive case.**
1. Choose the opponent Digimon. Hand +1, memory unchanged.
2. At the SourceMulti prompt, pick 2. Memory +2.

**Negative case.** In a second runner:
1. PASS the optional pick. Memory +1 through `n: 0`; hand unchanged.
2. Pick 1 source, then PASS. Memory stays at +1, because 1 != 2.

**Selection surface.**
- The first prompt has `is_optional == true`, because of `optional: true`.
- The source picker has `is_optional == true`, because `min` is 0.

### Suspected engine/DSL defects
- None seen.
- One surprise, harmless: `_ => 1` also counts a *literal* binding as 1, for example the `bind_as` of
  `select_effect_choice`.

---

## 5. `predicate:binding_card_kind`

### Lowering sites
- **Parse:** `DSL/predicate.rs:442`.
- **Payload:** `DSL/predicate.rs:1182-1185` (`{ binding: String, kind: CardKind }`). `kind` is one of `digimon`,
  `tamer`, `option`, `digi_egg`, `token`, `dual` (`DSL/spec.rs:104-111`).
- **Compile:** `DSL/compile.rs:1158`.
- **IR:** `DSL/compiled.rs:554`.
- **Runtime:** `engine/dsl_cards/predicate.rs:854-881`.
  ```rust
  let Some(handle) = bindings.and_then(|b| {
      b.get_card(&binding_kind.binding)
          .or_else(|| b.get_union_card(&binding_kind.binding).map(|(c, _, _)| c))
          .or_else(|| b.get_card_list(&binding_kind.binding).filter(|l| l.len() == 1).map(|l| l[0]))
  }) else { return false; };
  let Some(data) = rctx.game.card_data_for_handle(handle) else { return false; };
  if !kind_matches_card_search(binding_kind.kind, data) { return false; }
  ```
- **Kind matching:** `kind_matches_card_search` (`predicate.rs:4971-4977`).

  | Requested kind | Matches |
  |---|---|
  | `digimon` | Digimon **or Dual** |
  | `option` | Option **or Dual** (`card_data.rs:303-305`) |
  | every other kind | exactly that kind |

### YAML shape
From LM-020: `{ binding_card_kind: { binding: revealed, kind: digimon } }`.

### Semantics
- **What it tests.** The *printed category* of a card held in a named binding.
- **Accepted binding shapes:**
  - a single `Card` binding;
  - an origin-tagged `select_union_zone` binding;
  - a `CardList` with **exactly one** element.
- **Fails closed** when the binding is missing, is a multi-card list, or does not resolve.
- **Reads card data, not the field.** It reads `CardData`, so a card that has since moved zones still resolves.
- **DUAL cards** satisfy **both** `digimon` and `option`.

### Rule basis
- §2 (card categories) and the DUAL card rule: in search contexts a DUAL is both a Digimon card and an Option card.
- Otherwise a card-text idiom: "if it's of that category".

### Production users
Examon blocks on this construct.
- **EX13-045 Examon.**
  - Printed: "[Your Turn] [Once Per Turn] When this Digimon wins a battle, you may play or use 1 play or use cost 12
    or lower [Dracomon] or [Examon] text card from your hand or its digivolution cards without paying the cost."
  - YAML:
    `- if: { condition: { binding_card_kind: { binding: examon_card, kind: option } }, then: [ {use_option_bound: {binding: examon_card}} ], else: [ {play_union_bound_free: {binding: examon_card}} ] }`
- **LM-020 Quantumon.**
  - Printed: "[Start of Opponent's Turn] Declare 1 card category. Then, reveal the top card of your opponent's deck.
    If it's of that category, …"
  - YAML: four `if` arms, each
    `all_of: [ {equals: [category, N]}, {binding_card_kind: {binding: revealed, kind: digimon|tamer|option|digi_egg}} ]`.

### Existing near-miss tests
`src/dsl_cards/predicate.rs` (the evaluator) only.

### Proposed behavioural test
**Fixture.**
```yaml
card: TST-BCK
name: Binding Kind Probe
kind: digimon
level: 3
color: [red]
cost: 3
dp: 3000
effects:
  - when: on_play
    process:
      - reveal_top_deck: { of: you, count: 1, bind_as: revealed }
      - if:
          condition: { binding_card_kind: { binding: revealed, kind: option } }
          then:
            - gain_memory: 1
      - if:
          condition: { binding_card_kind: { binding: revealed, kind: digimon } }
          then:
            - gain_memory: 2
      - place_remainder_on_deck: { of: you, position: bottom }
```
`reveal_top_deck` with `count: 1` binds a single `Card` (`zone_moves.rs:569-576`).

**Setup.**
1. `memory(0)`.
2. Deck `["F","F","F", TOP]`. Run once each with TOP as an Option card (`card_kind = Option`), a Digimon, a Tamer and
   a DUAL.
3. Place `TST-BCK`, enqueue `OnPlay`, drain.

**Expected results.**

| Top card | Expected memory | Why |
|---|---|---|
| Option | 1 | `option` arm only |
| Digimon | 2 | `digimon` arm only |
| DUAL | 3 | both arms match |
| Tamer | 0 | negative case |

**Further negative case.** Change `count` to 2, which binds a `CardList` of two cards. Memory stays 0 even when an
Option is on top, because multi-card lists fail closed.

**Selection surface.** None.

### Suspected engine/DSL defects
- EX13-045's `if option … else play` sends a **DUAL** card to `use_option_bound` with no choice, because a DUAL
  matches `option`.
- "Play or use" on a DUAL should probably let the player choose between play and use (no-auto-selection policy).
- Verify against DCGO EX13_045 before filing.

---

## 6. `predicate:name_not_in_binding`

### Lowering sites
- **Parse:** `DSL/predicate.rs:434` (`Option<String>`).
- **Compile:** `DSL/compile.rs:1150`.
- **IR:** `DSL/compiled.rs:552`.
- **Runtime** (inside `eval_card_fields`, Card subject): `engine/dsl_cards/predicate.rs:3306-3325`.
  ```rust
  if let Some(ref binding) = pred.name_not_in_binding {
      if let Some(list) = bindings.and_then(|b| b.get_card_list(binding)) {
          let names = |d: &CardData| { let mut v = vec![d.card_name.clone()];
              v.extend(d.also_treated_as.iter().cloned()); v };
          let mine = names(data);
          let clash = list.iter().filter(|h| **h != card).any(|h| rctx.game
              .card_data_for_handle(*h).is_some_and(|other| names(other).iter().any(|n| mine.contains(n))));
          if clash { return false; }
      }
  }
  ```
- **Producer of the growing list.** `link_cards` appends each linked card to its `bind_as` CardList after every pick
  (`engine/dsl_cards/step/link_cards.rs:1215-1223`). The next pick's filter sees the updated list.

### YAML shape
From BT26-086 Dantemon:
```yaml
- link_cards:
    from: [self_sources]
    filter:
      all_of:
        - trait_has: Appmon
        - name_not_in_binding: dante_links
    to: self
    count: { up_to: 7 }
    cost: free
    bind_as: dante_links
```

### Semantics
- **What it tests.** A card-subject leaf. It passes when the candidate shares **no** name with any card in the named
  **CardList** binding. The candidate itself is ignored.
- **Which names are compared.** The printed `card_name` plus the static `also_treated_as` aliases.
- **How names are compared.**
  - Exact match, case-sensitive, on the stored strings.
  - It does not apply the Language Standardization containment rules, because it tests equality, not "in its name".
- **Passes when the binding is absent or is not a CardList** (nothing chosen yet).

### Rule basis
- A card-text idiom: "with different names".
- §2-3 (card name) defines what counts as a name.
- Alias rule boxes ("also treated as [X]") are included.

### Production users
Rebootmon blocks on this construct.
- **BT26-086 Dantemon.** Printed: "[On Play] [When Digivolving] You may link up to 7 [Appmon] trait cards with
  different names from this Digimon's digivolution cards to this Digimon without paying the costs. …" YAML is shown
  above.
- This is the only user in the pool.

### Existing near-miss tests
`src/dsl_cards/predicate.rs` (the evaluator) only.

### Proposed behavioural test
This test isolates the leaf with a CardList built by a reveal and a hand picker. The `link_cards` loop needs much more
scaffolding.

**Fixture.**
```yaml
card: TST-NNIB
name: Different Names Probe
kind: digimon
level: 3
color: [red]
cost: 3
dp: 3000
effects:
  - when: on_play
    process:
      - reveal_top_deck: { of: you, count: 2, bind_as: seen }
      - select_hand:
          of: you
          bind_as: pick
          filter: { name_not_in_binding: seen }
          prompt: "Trash 1 card whose name was not revealed"
      - trash_from_hand_by_index: { of: you, hand_index: pick }
      - place_remainder_on_deck: { of: you, position: bottom }
```

**Setup.**
1. Deck: the top two cards are named "Agumon" and "Gabumon" (`make_test_card("AGU-D","Agumon")`,
   `make_test_card("GAB-D","Gabumon")`).
2. Hand, in this order:
   - `AGU-H`, named "Agumon";
   - `PAT-H`, named "Patamon";
   - `ALIAS-H`, named "Koromon" with `also_treated_as = ["Gabumon"]`.
3. Place `TST-NNIB`, enqueue `OnPlay`, drain.

**Positive case and selection surface.**
- There is a real `pending_selection` with `is_optional == false`.
- It has exactly **1** valid action: the hand-select id for the Patamon card at hand index 1. Assert the id set equals
  `{that id}`.
- Execute it. Patamon is in the trash.

**Negative case (no clash).** Reveal two cards named "Tentomon" and "Palmon" instead. All 3 hand cards are offered.

**Optional production-shape test.** Run a `link_cards` loop over two same-name `[Appmon]` sources. After the first is
linked, the second same-name card disappears from the candidates.

### Suspected engine/DSL defects
- None seen.
- Names are compared exactly, so `name_contains`-style exclusions are not relevant here.

---

## 7. `predicate:effect_deleted_any_own_digimon`

### Lowering sites
- **Parse:** `DSL/predicate.rs:989` (`Option<bool>`).
- **Compile:** `DSL/compile.rs:1440`.
- **IR:** `DSL/compiled.rs:727`.
- **Runtime:** `engine/dsl_cards/predicate.rs:1133-1141`, inside `eval_result_bound_fields`.
  ```rust
  if let Some(want) = pred.effect_deleted_any_own_digimon {
      let actual = log.deleted.iter().zip(&log.deleted_is_digimon)
          .any(|((h, _), digimon)| *digimon && h.player == rctx.player);
      if actual != want { return false; }
  }
  ```
- **Without bindings,** `true` fails and `false` passes (`predicate.rs:1051-1065`).
- **The log.** `Bindings::result_log().deleted` plus `deleted_is_digimon` (`engine/dsl_cards/bindings.rs:76-95`).
  Rows are added by `record_deleted` (line 191).

### What gets recorded
**The receipt path (current behaviour for three verbs).**
- `delete_permanent`, `delete_permanents` and `delete_all_permanents` are **expanded** (`step/mod.rs:741`,
  `step/deletion_results.rs:7-27`) into `DeleteCapturedBatch` + `DeleteReceiptFinalize`.
- Only deletions that **actually complete** inside the receipt scope are recorded (`combat/deletion.rs:759-783`).
- Each row stores the pre-removal DP and `permanent_is_digimon_for_rules`, so tokens count as Digimon.
- A deletion prevented by a replacement (`<Evade>`, `<Armor Purge>`, a paid prevention) records nothing.

**The legacy path (still records *attempts*).**
- `delete_bound_permanents` (`permanent_mutations.rs:135-161`).
- `delete_for_cost_reduction` (`permanent_mutations.rs:93-127`).

### YAML shape
From BT26-078 Cherubimon:
```yaml
- when: [on_play, when_digivolving]
  optional: true
  process:
    - delete_permanent: { target: source }
    - if:
        condition: { effect_deleted_any_own_digimon: true }
        then: [ ... select_trash + play ... ]
```

### Semantics
- **What it tests.** True when **this activation** has completed the deletion of at least one Digimon whose
  **controller is the effect's controller** (`h.player == rctx.player`).
  - Tokens count as Digimon.
  - Tamers and Options are excluded.
- **What "this activation" covers.** One clause resolution, sharing one `Bindings`. That includes:
  - all of the clause's steps;
  - `then:` bodies;
  - parked and resumed continuations;
  - `per_selected` and `for_each` sub-bodies, whose logs are merged back via `merge_result_log_from_since`
    (`iteration.rs:144-222`).
- **What it does not cover.** Deletions by other effects are separate activations and are not in this log. That
  includes On Deletion triggers it causes, and a `<Decoy>` substitute's own deletion.
- **Prevented deletions.** A deletion prevented by a replacement does not count on the receipt path.

### Rule basis
- **§15-7-1 and §15-7-2.** In "By deleting X, Y", Y requires that X was executed.
- **§4-15-1.** Deletion means the card is trashed.
- **§16-22-1, §16-19 and §16-25.** `<Evade>`, `<Armor Purge>` and `<Barrier>` *prevent* the deletion. `<Barrier>` is
  battle-only, so it never applies to effect deletion.
- **§15-8-5-2.** "Would be deleted" replacements interrupt before deletion.
- **CR 9-1-3 example** ("If this effect trashed a card…"): the result-of-effect idiom.
- **DCGO agrees.** `DeletePeremanentAndProcessAccordingToResult` passes only *successfully* destroyed permanents to its
  success callback (BT25_014.cs).

### Production users
DATA SQUAD Ravemon, TS Toolbox and Chronomon block on this construct.
- **BT26-078 Cherubimon (Chronomon).** Printed: "[On Play] [When Digivolving] By deleting this Digimon, you may play 1
  play cost 12 or lower [Chronomon] text or [Titan] trait card from your trash without paying the cost." YAML is
  shown above.
- **EX3-057 Growlmon (inherited).**
  - Printed: "[When Attacking] [Once Per Turn] By deleting 1 of your other Digimon, this Digimon gains
    <Security A. +1> for the turn."
  - YAML, in order:
    1. `select_own_permanent {bind_as: sacrifice, optional: true, filter: {all_of: [{kind: digimon}, {other: true}]}}`
    2. `delete_permanent: { target: sacrifice }`
    3. `if: { condition: { effect_deleted_any_own_digimon: true }, then: [ add_modifier SecurityAttackChange +1 ] }`
- TS Toolbox and Chronomon also use BT26-073.

### Existing near-miss tests
`src/dsl_cards/predicate.rs` (the evaluator) only.

### Proposed behavioural test
**Fixture.**
```yaml
card: TST-EDEL-OWN
name: Own Deletion Probe
kind: tamer
color: [red]
cost: 2
effects:
  - when: on_play
    process:
      - select_own_permanent:
          bind_as: sac
          filter: { other: true }
          prompt: "Delete 1 of your other permanents"
      - delete_permanent: { target: sac }
      - if:
          condition: { effect_deleted_any_own_digimon: true }
          then:
            - gain_memory: 1
```

**Setup.**
1. `memory(0)`.
2. Place `TST-EDEL-OWN`, enqueue `OnPlay`, drain.
3. Resolve the selection. It is mandatory with 1 candidate, and must **not** be auto-picked.

**Positive cases.**
- The only other permanent is an own Digimon: memory becomes 1 and the Digimon is in the trash.
- The same with an own Digimon **token**: memory also becomes 1.

**Negative cases.**
1. **Non-Digimon.** The only other permanent is an own Tamer. It is deleted, but memory stays 0.
2. **Prevented.**
   - Give the own Digimon Evade: `r.game.modifiers.grant_keyword(h, Keyword::Evade, Expiry::Permanent, 0)`.
   - Accept the Evade prompt with `choose(&mut r, false)`.
   - The Digimon stays on the field, suspended, and memory stays 0.
3. **Wrong side.** See the opponent fixture in section 8. Deleting an *opponent* Digimon leaves this leaf false.

### Suspected engine/DSL defects
1. **`delete_bound_permanents` logs prevented deletions** (`permanent_mutations.rs:135-161`).
   - It calls `delete_permanents_batch`, then records **every pre-snapshot**.
   - A target saved by `<Evade>` or `<Armor Purge>` is still logged as deleted.
   - So `effect_deleted_*` (and DP-threshold riders) read true for a prevented deletion.
   - Only the three migrated verbs use the receipt path (`deletion_results.rs:20`: "Preserve bound-list sequential
     semantics until separately migrated").
2. **`delete_for_cost_reduction` credits an unpaid cost** (`permanent_mutations.rs:93-127`).
   - It records the deletion and credits the cost reduction right after `ctx.delete_permanent(h)`.
   - It never checks that the deletion completed.
   - So a prevented "by deleting" cost would still reduce the cost (§15-7-2).

---

## 8. `predicate:effect_deleted_any_opponent_digimon`

### Lowering sites
Same machinery as section 7.
- **Parse:** `DSL/predicate.rs:991`.
- **Compile:** `DSL/compile.rs:1441`.
- **IR:** `DSL/compiled.rs:728`.
- **Runtime:** `engine/dsl_cards/predicate.rs:1143-1151`.
  ```rust
  if let Some(want) = pred.effect_deleted_any_opponent_digimon {
      let actual = log.deleted.iter().zip(&log.deleted_is_digimon)
          .any(|((h, _), digimon)| *digimon && h.player != rctx.player);
      if actual != want { return false; }
  }
  ```

### YAML shape
From EX13-013 WarGrowlmon:
```yaml
- if:
    condition: { effect_deleted_any_opponent_digimon: false }
    then:
      - grant_keyword: { target: source, keyword: Piercing, expiry: end_of_turn }
      - add_dp_modifier: { target: source, value: 3000, expiry: end_of_turn }
```

### Semantics
- **What it tests.** The mirror of section 7, for Digimon (tokens included) whose controller is **not** the effect's
  controller.
- **Most users test `false`** ("If this effect didn't delete…"). The important edges are:

  | Situation | Leaf value | Effect on a "didn't delete" rider |
  |---|---|---|
  | No target bound; the delete is a no-op | `false` | rider fires |
  | Target is immune (`can_affect_permanent` filters it out before the batch) | `false` | rider fires |
  | Target is saved by a replacement | `false` | rider fires |

- The same receipt/legacy split from section 7 applies.

### Rule basis
- The same rules as section 7: §15-6-2 (processing condition), §15-8-5-2, §16-22 `<Evade>`, §16-19
  `<Armor Purge>`.
- **§16-18 `<Decoy>`.** A Decoy's *own* deletion is the Decoy effect's, not this effect's, and the target itself was
  not deleted.

### Production users
Beelstarmon (3M) and Chronomon block on this construct.
- **BT25-014 Meramon (Chronomon list).**
  - Printed: "[Main] [Once Per Turn] By trashing 1 [Flame] or [TS] trait card from your hand, delete 1 of your
    opponent's Digimon with 4000 DP or less. If this effect didn't delete, <Draw 2>."
  - YAML: `delete_permanent: { target: victim }`, then
    `if: { condition: { effect_deleted_any_opponent_digimon: false }, then: [ { draw: { of: you, count: 2 } } ] }`.
- **EX13-013 WarGrowlmon.** Printed: "[When Digivolving] [When Attacking] Delete 1 of your opponent's Digimon with
  5000 DP or less. If this effect didn't delete, this Digimon gains <Piercing> and +3000 DP for the turn." YAML is
  shown above.

### Existing near-miss tests
`src/dsl_cards/predicate.rs` (the evaluator) only.

### Proposed behavioural test
**Fixture.**
```yaml
card: TST-EDEL-OPP
name: Opponent Deletion Probe
kind: tamer
color: [red]
cost: 2
effects:
  - when: on_play
    process:
      - select_opponent_permanent:
          bind_as: victim
          optional: true
          continue_on_decline: true
          filter: { kind: digimon }
          prompt: "Delete 1 of your opponent's Digimon"
      - delete_permanent: { target: victim }
      - if:
          condition: { effect_deleted_any_opponent_digimon: false }
          then:
            - draw: { of: you, count: 1 }
      - if:
          condition: { effect_deleted_any_opponent_digimon: true }
          then:
            - gain_memory: 1
      - if:
          condition: { effect_deleted_any_own_digimon: true }
          then:
            - gain_memory: 5
```

**Setup.**
1. `memory(0)`, deck fillers, one opponent Digimon `V`.
2. Place `TST-EDEL-OPP`, enqueue `OnPlay`, drain.

**Positive case.** Pick `V`. Then:
- `V` is in the opponent's trash;
- memory is 1, not 6, so the own-side leaf stayed false;
- hand is unchanged.

**Negative cases.**
1. **Declined.** PASS. Hand +1, memory 0.
2. **Prevented.**
   - Grant `V` `Keyword::Evade` and pick it.
   - The opponent's Evade prompt appears (`pending_selection.selecting_player == 1`); accept it with
     `choose(&mut r, false)`.
   - `V` is still on the field, suspended. Hand +1, memory 0.
   - This pins "prevented ≠ deleted" on the receipt path.

**Additional case (tokens count).** Place an opponent token and pick it. Memory becomes 1.

### Suspected engine/DSL defects
- **The same two legacy-path issues as section 7.**
- **Worth a test: `<Decoy>` and the receipt.**
  - Confirm that a `<Decoy>` substitution (§16-18) does **not** write into this effect's receipt.
  - It should not, because `delete_permanents_batch` (receipt `None`) resets `current_deletion_receipt` for nested
    deletions (`combat/deletion.rs:81-84`).
  - No test pins this yet.
- **Incidental, outside this construct.**
  - BT25-014's inherited `[When Attacking]` YAML carries `once_per_turn: true` (DCGO has `maxCount 1`).
  - The printed and official text has no [Once Per Turn].
  - Verify against the card image.

---

## 9. `predicate:event_added_card_any`

### Lowering sites
- **Parse:** `DSL/predicate.rs:820` (`Option<Box<PredicateSpec>>`).
- **Compile:** `DSL/compile.rs:1347`.
- **IR:** `DSL/compiled.rs:785`.
- **Runtime:** `engine/dsl_cards/predicate.rs:2093-2109`.
  ```rust
  if let Some(inner) = &pred.event_added_card_any {
      let added: Vec<CardHandle> = rctx.game.current_trigger_context.as_ref()
          .map(|trigger| trigger.added_source_cards().to_vec()).unwrap_or_default();
      let any_match = added.iter().any(|card|
          eval_predicate_with_bindings(inner, rctx, PredicateSubject::Card(*card), None));
      if !any_match { return false; }
  }
  ```
- **Batch source:** `TriggerSource::SourcesAddedToStack { cards, .. }` (`engine/game/mod.rs:2544-2559`).

### YAML shape
From EX7-005 Kapurimon:
```yaml
trigger_if:
  all_of:
    - event_host_permanent_is_source: true
    - event_added_card_any:
        kind: option
        trait_has: Three Musketeers
```

### Semantics
- **Only meaningful on `on_add_digivolution_cards`.** On any other timing the batch is empty and the leaf is false.
- **The batch.** All cards one effect placed under that host, grouped per host and per placing controller.
- **What it tests.** True when **at least one** card in the batch matches the inner card predicate.
  - The inner predicate is evaluated as a Card subject with **no bindings**.
  - Face-down placed cards are still evaluated on their card data.

### Rule basis
- **§15-5-2.** One trigger condition fires once for simultaneous cards, so "any of them" is checked across the batch.
- **§15-5-3.** Placing a card whose own inherited text watches this event triggers that text.

### Production users
Beelstarmon (3M) blocks on this construct.
- **EX7-005 Kapurimon (inherited).** Printed: "[Your Turn] [Once Per Turn] When an effect places an Option card with
  the [Three Musketeers] trait in this Digimon's digivolution cards, gain 1 memory." YAML is shown above; the process
  is `gain_memory: 1`.
- **BT25-005 Pagumon (inherited).**
  - Printed: "[Your Turn] [Once Per Turn] When [Three Musketeers] trait cards are placed in this Digimon's
    digivolution cards, it may digivolve into…".
  - YAML:
    `trigger_if: { all_of: [ {event_host_permanent_is_source: true}, {event_added_card_any: { trait_has: Three Musketeers }} ] }`.

### Existing near-miss tests
`src/dsl_cards/predicate.rs` (the evaluator) only.

### Proposed behavioural test
This fixture is shared with section 11.

**Fixture.**
```yaml
card: TST-ADD-HOST
name: Added Card Probe
kind: digimon
level: 4
color: [red]
cost: 4
dp: 4000
effects:
  - when: on_play
    process:
      - place_as_bottom_source: { source: { deck_top: you }, target: source, face_down: false }
      - place_as_bottom_source: { source: { deck_top: you }, target: source, face_down: false }
  - when: on_add_digivolution_cards
    trigger_if:
      all_of:
        - event_host_permanent_is_source: true
        - event_added_card_any: { kind: option }
    process:
      - gain_memory: 1
```

**Setup.**
1. `memory(0)`.
2. Deck `["F","F","DIGI","OPT"]`. The last id is the top, so OPT is placed first, then DIGI. OPT is an Option
   `CardData`.
3. Place the host, enqueue `OnPlay`, drain.

**Positive case.** The stack is 3 cards and memory is **1**. One batch containing an Option fires exactly once, not
twice.

**Negative cases.**
- Deck `["F","F","DIGI","DIGI"]`: memory stays 0.
- Evaluate the same leaf on a different timing, for example `when: on_play` with
  `condition: { event_added_card_any: {} }`: the leaf is false.

**Selection surface.** None.

### Suspected engine/DSL defects
- **No bindings inside the inner predicate.** Binding-referencing leaves inside it always fail. That is fine today.
- **BT25-005 wording versus timing.**
  - BT25-005's printed "When … are placed" is not restricted to effects, but the timing fires only for effect-driven
    placement.
  - This matches DCGO `CanTriggerOnAddDigivolutionCard`, since normal digivolution cannot place a card *under* an
    existing card.

---

## 10. `predicate:event_target_name_contains`

### Lowering sites
- **Parse:** `DSL/predicate.rs:693` (`Option<String>`).
- **Compile:** `DSL/compile.rs:1316`.
- **IR:** `DSL/compiled.rs:651`.
- **Runtime:** `engine/dsl_cards/predicate.rs:1898-1928`.
  ```rust
  let want = needle.to_lowercase();
  if let Some(snapshot) = rctx.game.current_trigger_context.as_ref().and_then(|t| t.deleted_object.as_ref()) {
      let name_match = snapshot.names_just_before.iter()
          .any(|name| crate::card_data::rules_name_contains(name, &want));
      if !name_match { return false; }
  } else {
      let Some(card) = event_target_card(rctx) else { return false; };
      if !rctx.game.card_rules_name_contains(card, &want) { return false; }
  }
  ```
- **How the event target is found.** `event_target_card` (`predicate.rs:2926-2970`) tries these in order and uses the
  first that exists:
  1. the deleted-object snapshot's top card;
  2. the new attack target;
  3. the live event permanent's top card;
  4. `target_permanent`;
  5. `target_card`;
  6. `event_card`.
- **How names are read.** `card_rules_name_contains` (`engine/game/handles.rs:21-37`) reads a field permanent's
  synthesized identity names, which include name-change modifiers. For a card not on the field it reads the card's
  own names.
- **How names are matched.** `rules_name_contains` (`card_data.rs:10-17`) is a lowercase substring test with one
  exception: `Indramon` does not contain `Dramon`.

### YAML shape
From EX3-037 Dracomon (inherited, `on_suspend`):
```yaml
trigger_if:
  all_of:
    - event_target_owner: you
    - event_target_kind: digimon
    - any_of:
        - event_target_name_contains: Dramon
        - event_target_name_contains: Examon
```

### Semantics
- **What it tests.** A case-insensitive "has [X] in its name" test on the **event's subject permanent**:

  | Timing | Subject |
  |---|---|
  | `on_digivolve` | the digivolved permanent's new top card |
  | `on_suspend` | the suspended permanent |
  | deletion timings | the deleted permanent's last-known names |

- **No event target → false.** That makes it effectively always false in a non-triggered clause, or after the
  trigger context is cleared.
- **Name changes count.** Name-change effects in force are honoured, and a deleted card is judged by its name as it
  was at deletion.

### Rule basis
- **§2-3, card name.** Defines "[X] in its name".
- **Bandai's Language Standardization Rules** (2026-05-29; outside the CR PDF). They exclude, among others:
  - BurningGreymon, DoruGreymon and DexDoruGreymon from [Greymon];
  - KendoGarurumon from [Garurumon];
  - Pagumon from [Agumon];
  - BeelStarmon from [Starmon].

### Production users
Examon blocks on this construct.
- **EX3-037 Dracomon (Examon list).** Printed: "[All Turns] [Once Per Turn] When one of your Digimon with [Dramon] or
  [Examon] in its name becomes suspended, this Digimon gets +1000 DP for the turn." YAML is shown above.
- **EX4-061 Matt Ishida & Tai Kamiya.**
  - Printed: "[Your Turn] [Once Per Turn] When one of your Digimon digivolves, if you have 1 or fewer Digimon, you
    may play 1 [Gabumon] if that Digimon has [Greymon] in its name or 1 [Agumon] if that Digimon has [Garurumon] in
    its name…".
  - YAML:
    `condition: { all_of: [ …, { any_of: [ {event_target_name_contains: "Greymon"}, {event_target_name_contains: "Garurumon"} ] } ] }`.

### Existing near-miss tests
`src/dsl_cards/predicate.rs` (the evaluator) only.

### Proposed behavioural test
**Fixture.** The needle is lower-case on purpose, to pin case-insensitivity.
```yaml
card: TST-ETNC
name: Event Name Probe
kind: tamer
color: [green]
cost: 2
effects:
  - when: on_suspend
    trigger_if:
      all_of:
        - event_target_owner: you
        - event_target_name_contains: dramon
    process:
      - gain_memory: 1
```

**Setup.**
1. `memory(0)`.
2. Place `TST-ETNC` and three own Digimon named `Wingdramon`, `Indramon` and `Agumon`.
3. Suspend each one in turn with `r.game_mut().suspend(h)`.

**Positive case.** Suspending Wingdramon: memory 1.

**Negative cases.**
- Suspending Agumon: no change.
- Suspending Indramon: no change. This pins the one exclusion that ships today.

**Deletion-snapshot variant.**
- Use an `on_any_deletion` observer with `event_target_name_contains: Sukamon`.
- It fires when a Digimon *named* `Sukamon` is deleted.
- It reads `names_just_before` after the card has left the field.

**Selection surface.** None.

### Suspected engine/DSL defects
- **Most Language Standardization exclusions are missing.**
  - `rules_name_contains` implements only the Indramon/Dramon exclusion.
  - So BurningGreymon satisfies `event_target_name_contains: Greymon`. EX4-061 and EX13-067 would let a BurningGreymon
    digivolution play Gabumon.
  - KendoGarurumon likewise satisfies Garurumon, and BeelStarmon satisfies Starmon.
  - These contradict the official Language Standardization Rules. This is known and pending owner sign-off, per
    project memory.
- **A test should not pin the current, wrong behaviour.**

---

## 11. `timing:on_add_digivolution_cards`

### Lowering sites
- **Parse:** `DSL/clause.rs:244` (`OnAddDigivolutionCards`; doc comment at 225-243).
- **Compile:** `DSL/compile.rs:336`.
- **Timing map:** `engine/dsl_cards/timing_map.rs:53` maps it to `EffectTiming::OnAddDigivolutionCards`.
- **Builder:** `lower_triggered.rs:1553` calls `Effect::on_add_digivolution_cards` (`effect.rs:827`).

### Dispatch
1. **Every effect-driven placement opens or extends a window.** It calls
   `note_effect_added_sources(host, cards, cause)` (`game/mod.rs:2499-2532`). There is one window per **host top card
   + placing controller**. Callers include:
   - `effect_context/action/sources.rs`;
   - `game_actions/mod.rs:2856` and `2884` (`place_as_source_observed`);
   - `game_actions/link.rs:1084`;
   - `option_lifecycle.rs:495`.
2. **A different controller flushes the earlier window.** If a later placement comes from a *different* controller,
   the earlier window is flushed first.
3. **Windows flush when the placing effect finishes.** Two call sites do this:
   - `release_event_window_triggers` (`effect_queue.rs:1640-1650`), only at the outermost drain and only when nothing
     is parked;
   - the post-queue loop (`effect_queue.rs:1537-1556`).
4. **The host is re-checked.** `fire_added_sources_batch` re-resolves the host by its top card. If the host has left,
   nothing fires.
5. **The trigger fans out to the whole board.** It is enqueued as `SourcesAddedToStack` and sent to **every
   battle-area and breeding permanent of both players** (`effect_queue.rs:1013-1047`). There is no forced host filter.

### YAML shape
From BT7-056 Dorumon:
```yaml
- scope: inherited
  when: on_add_digivolution_cards
  once_per_turn: true
  active_when: { your_turn: true }
  trigger_if:
    all_of:
      - event_host_permanent_is_source: true
      - event_caused_by_own_effect: true
  process:
    - gain_memory: 1
```

### Semantics
- **When it fires.** An observer that fires **once per host per placing effect**, after an *effect* has put one or
  more cards into a permanent's digivolution cards.
- **What placements count.**
  - top or bottom;
  - face up or face down;
  - from deck, hand, trash, reveal, link or material.
- **What never fires it** (doc comment `clause.rs:225-243`; these paths do not call `note_effect_added_sources`):
  - normal or DNA digivolution;
  - DigiXros or Assembly material consumption;
  - App Fusion.
- **Scope must be added explicitly.**
  - Use `trigger_if:` to limit it (this host, own effect, card shape).
  - Without `trigger_if:`, every permanent carrying the clause fires for every host.

### Rule basis
- **§15-5-2.** One trigger per simultaneous event.
- **§15-5-3.** The CR example "[Your Turn][Once Per Turn] When your effect places a digivolution card under this
  Digimon, gain 1 memory" triggers even when that card itself is the one placed.
- **§15-4-2.** Trigger, then pending.

### Production users
Beelstarmon (3M) blocks on this construct.
- **BT7-056 Dorumon (inherited).** Printed: "[Your Turn] [Once Per Turn] When one of your effects places a
  digivolution card under this Digimon, gain 1 memory." YAML is shown above.
- **EX7-005 Kapurimon (inherited, Beelstarmon list).** YAML is in section 9.

### Existing near-miss tests
- `src/dsl_cards/lower_triggered.rs` (the builder arm) and `src/dsl_cards/timing_map.rs` (the enum map).
- Both are source files that only name the timing.

### Proposed behavioural test
**Fixtures.**
- **`TST-ADD-HOST`** from section 9, with the observer clause replaced by:
  - `when: on_add_digivolution_cards`
  - `trigger_if: { event_host_permanent_is_source: true }`
  - `process: [ { gain_memory: 1 } ]`
- **`TST-ADD-OBS`:** a Tamer with `when: on_add_digivolution_cards`, no qualifier, process `gain_memory: 1`.
- **`TST-ADD-BASE`:** a Lv.3 red Digimon.
- **`TST-ADD-EVO`:** a Lv.4 red Digimon with
  `alt_paths: [{kind: digivolve, from: {level_eq: 3, color_is: red}, cost: 2}]`.

**Positive case.**
1. Place `TST-ADD-HOST` with memory 0 and a deck of 4 fillers.
2. Enqueue `OnPlay` and drain.
3. Assert:
   - the host stack grew by 2;
   - memory is exactly **1**, because two placements in one effect make one batch.
4. Repeat with `TST-ADD-OBS` also on the field. Memory is now 2: the board-wide fan-out reaches a non-host observer
   once.

**Negative and adjacent-timing cases.**
- **Normal digivolution is not an effect placement.**
  1. Put `TST-ADD-OBS` and `TST-ADD-BASE` on the field, Main phase, memory 5.
  2. Digivolve `TST-ADD-EVO` from hand onto the base with `digivolve_from_hand`.
  3. Memory must be exactly `5 - 2`. There must be no +1 from the observer.
- **Trashing a source is not an addition.** `r.trash_one_source(h)` fires `OnDigivolutionCardTrashed` and must not
  fire the observer.

**Ordering case.**
- Add a `- gain_memory: 10` step **after** the two placements in the host's `on_play`.
- The observer's +1 must arrive after the +10, because the batch flushes only when the effect finishes.
- To check: record memory inside a second observer, or assert the `effect_queue` is empty and memory == 11 only after
  the drain, and never 1 in between.

### Suspected engine/DSL defects
None seen.

---

## 12. `predicate:during_attack`

### Lowering sites
- **Parse:** `DSL/predicate.rs:617` (`Option<bool>`).
- **Compile:** `DSL/compile.rs:1300`.
- **IR:** `DSL/compiled.rs:621`.
- **Runtime:** `engine/dsl_cards/predicate.rs:414-418`.
  ```rust
  if let Some(want) = pred.during_attack {
      if rctx.game.pending_attack.is_some() != want {
          return false;
      }
  }
  ```

### Lifecycle of `pending_attack`
1. **Set** at the attack declaration, before `WhenWouldAttack` and `WhenAttacking` are dispatched
   (`engine/combat/mod.rs:719-738`).
2. **Stays set** through counter timing, block timing, the security check or checks, and battle.
3. **Stays set** through the `[End of Attack]` pool. `combat/mod.rs:4060-4090` drains `EndOfAttack` while the attack
   is still pending (§11-6-2).
4. **Cleared** in `finish_attack_after_end_of_attack_timing` (`combat/mod.rs:4123`), after the End of Attack
   processing has resolved.
5. **Never set by an effect battle** (a `battle:` step or `battle_digimon`). `tests/dsl/effect_battle.rs` asserts
   this.

### YAML shape
From EX13-057 Grademon:
```yaml
- if:
    condition: { during_attack: true }
    then: [ grant_effect_immunity …, add_dp_modifier … ]
```
EX3-024 uses `during_attack: false` as a guard.

### Semantics
A game-level flag meaning "an attack is in progress". It does not depend on which player is attacking: an attack by
either player makes it true.

| Moment | `during_attack` |
|---|---|
| Before the attack is declared | false |
| The attacker's own `[When Attacking]` effects | true |
| Counter timing | true |
| Block timing, and its "when blocked" triggers | true |
| Security checks, and the `[Security]` effects of the checked cards | true |
| Battle, and battle-deletion triggers | true |
| `[End of Attack]` effects | true |
| After the attack has fully ended, including effects of the next action | false |
| Effect-initiated battles outside an attack | false |
| The opponent's turn, unless the opponent is attacking | false |

### Rule basis
- **§11-1-3.** The attack timings are declaration, counter, block, confirmation, and end.
- **§11-1-4 and §11-1-5.** Once an attack is declared, every timing follows.
- **§11-6-1 and §11-6-2.** The attack ends only after the End of Attack processing resolves.
- **§13-1.** A security check is performed within the attack.
- **Note on numbering.** The brief cited §13 for the attack flow. In CR 4.3 the attack flow is §11; §13 is Security
  Checks.

### Production users
Examon blocks on this construct.
- **EX13-057 Grademon.** Printed: "[On Play] [When Digivolving] Until your opponent's turn ends, 1 of your [X Antibody]
  or [Chronicle] trait Digimon gains <Reboot> and <Blocker>. If during an attack, it also isn't affected by their
  Digimon effects and gets +5000 DP." YAML is shown above.
- **BT20-053 Grademon.**
  - Printed: "…Then, if during an attack until the end of your opponent's turn, 1 of your Digimon isn't affected…".
  - YAML:
    `- if: { condition: { during_attack: true }, then: [select_own_permanent …, grant_effect_immunity …, add_dp_modifier …] }`
- The Examon list carries EX3-024, which uses `during_attack: false`.

### Existing near-miss tests
`src/dsl_cards/predicate.rs` (the evaluator) only.

### Proposed behavioural test
**Fixture.**
```yaml
card: TST-DURATK
name: During Attack Probe
kind: digimon
level: 6
color: [red]
cost: 6
dp: 12000
effects:
  - when: when_attacking
    process:
      - if:
          condition: { during_attack: true }
          then:
            - gain_memory: 1
  - when: end_of_attack
    process:
      - if:
          condition: { during_attack: true }
          then:
            - draw: { of: you, count: 1 }
  - when: on_play
    process:
      - if:
          condition: { during_attack: false }
          then:
            - gain_memory: 2
```

**Setup.**
1. `memory(0)`, `.security(1, &["SECLOW"])` where SECLOW is a 1000-DP Digimon card, and deck fillers.
2. `set_phase(Main)` and `set_turn(2)`.
3. `let a = place_on_field(0, "TST-DURATK", Some(0))`.

**Positive case.**
1. `r.attack_player(a, 1, false)`.
2. Drain any prompts with `auto_resolve`, or pass optional ones explicitly.
3. Assert:
   - memory 1, so When Attacking saw `true`;
   - hand +1, so End of Attack still saw `true`;
   - `r.game.pending_attack.is_none()` afterwards.

**Negative cases.**
- **Outside an attack.** Enqueue `OnPlay` on `a` and drain. Memory +2, because `during_attack: false` is true. Also
  assert `pending_attack.is_none()`.
- **Effect battle (optional).** Run `r.battle_digimon(a, opp)` with a probe from an `end_of_battle` or other battle
  trigger. It should read `false`. `tests/dsl/effect_battle.rs` already asserts `pending_attack.is_none()`.

**Optional extensions.** Both should read `true`:
- a `[Security]` probe card in the opponent's security stack;
- an opponent `<Blocker>` with an on-block probe.

### Suspected engine/DSL defects
- None seen.
- The `pending_attack` lifecycle matches §11, and DCGO `attackProcess.IsAttacking` is the cited oracle.

---

## 13. `timing:start_of_opponents_main_phase`

### Lowering sites
- **Parse:** `DSL/clause.rs:283` (`StartOfOpponentsMainPhase`).
- **Compile:** `DSL/compile.rs:352`.
- **Timing map:** `engine/dsl_cards/timing_map.rs:65`.
- **Builder:** `lower_triggered.rs:1535` calls `Effect::start_of_opponents_main_phase` (`effect.rs:686-688`).
- **Granted-effect string map:** `step/grant_triggered.rs:216`.
- **Dispatch:** `engine/game_phases.rs:356-392` (`enter_main_phase`).
  ```rust
  self.set_turn_phase(GamePhase::Main);
  self.enqueue_triggered(EffectTiming::StartOfYourMainPhase, TriggerSource::PlayerBattleArea(tp));
  self.enqueue_triggered(EffectTiming::StartOfYourMainPhase, TriggerSource::PlayerBreedingArea(tp));
  for player in 0..self.players.len() {
      let player = player as PlayerId;
      if player != tp {
          self.enqueue_triggered(EffectTiming::StartOfOpponentsMainPhase,
              TriggerSource::PlayerBattleArea(player));
      }
  }
  self.drain_effect_queue();
  ```

### YAML shape
From EX3-024 Slayerdramon (face-up and inherited):
```yaml
- when: start_of_opponents_main_phase
  process:
    - select_own_permanent: { bind_as: cost_target, optional: true, filter: {...}, prompt: "..." }
    - suspend: { target: cost_target }
    - if: { condition: { all_of: [ {binding_present: cost_target}, {during_attack: false}, ... ] }, then: [...] }
```
EX3-024's `bind_as: cost_target` is just a binding name. It is unrelated to the `cost_target` predicate.

### Semantics
- **Who it fires for.** The **non-turn player's battle-area** permanents, both face-up and inherited effects.
- **When it fires.**
  - The moment the turn player's main phase begins, which is after breeding.
  - It is enqueued and drained together with the turn player's `[Start of Your Main Phase]`. Both sides' triggers
    therefore share one simultaneous pool, and the turn player orders first (§15-4-3-5).
- **Where it does not fire.**
  - The non-turn player's **breeding area**.
  - The turn player's own permanents.
- **Parked selections delay it.** If a breeding-phase selection is still parked, entering the main phase (and
  therefore this dispatch) waits until it resolves (`pending_enter_main_after_selection`).

### Rule basis
- **§15-16-13-2.** "[Start of Opponent's Main Phase] … triggered at the point when your opponent's main phase has
  arrived."
- **§6-5.** Main phase.
- **§6-1-3.** The next phase waits for processing to finish.
- **§15-4-3-5.** Ordering of simultaneous triggers.
- **§3-4-7-4.** Only [Breeding] effects work in the breeding area.

### Production users
Examon blocks on this construct.
- **EX3-024 Slayerdramon (face-up and inherited).** Printed: "[Start of Opponent's Main Phase] By suspending 1 of your
  Digimon with [Dramon] or [Examon] in its name, your opponent attacks with 1 of their Digimon." YAML is shown above.
- This is the only production user. Granted effects reach the timing through `grant_triggered.rs:216`.

### Existing near-miss tests
- `src/dsl_cards/lower_triggered.rs`, `src/dsl_cards/step/grant_triggered.rs` and `src/dsl_cards/timing_map.rs` are
  mapping code only.
- `tests/timing_dispatch.rs` tests `StartOfYourMainPhase` (lines 300-455), but not this timing.

### Proposed behavioural test
**Fixture.**
```yaml
card: TST-SOOMP
name: Opp Main Probe
kind: digimon
level: 3
color: [red]
cost: 3
dp: 3000
effects:
  - when: start_of_opponents_main_phase
    process:
      - draw: { of: you, count: 1 }
```

**Setup.**
1. Deck of 10 fillers for both players.
2. `place_on_field(0, "TST-SOOMP", Some(0))`.
3. `place_on_field(1, "TST-SOOMP", Some(0))`.
4. `place_in_breeding(1, "TST-SOOMP")`.
5. Record both hand sizes.

**Positive case.**
- Call `r.game_mut().enter_main_phase()` on player 0's turn.
- Player 1's hand is +1: exactly one draw, from the battle-area copy only.
- It is not +2, so the breeding-area copy did not fire.

**Negative and adjacent-timing cases.**
- Player 0's hand is unchanged. Its copy is on the turn player's side, which is `StartOfYourMainPhase` territory.
- Optional: a `when: start_of_your_main_phase` clause on player 1's card must not fire here either.

**Turn-swap check.**
- Switch turns, either with `r.set_first_player(1)` or with `end_turn` followed by an explicit `enter_main_phase()` on
  player 1's turn.
- Now player 0's copy draws and player 1's copy does not.

**Selection surface.**
- None.
- If two triggers in the pool both belong to the non-turn player, assert that the `TriggerOrder` selection is
  surfaced to the right player (as in `tests/timing_dispatch.rs:387-455`).

### Suspected engine/DSL defects
- None seen.
- EX3-024 also gates on `during_attack: false`. No attack can be in progress at this timing, so that leaf is redundant,
  but harmless.
