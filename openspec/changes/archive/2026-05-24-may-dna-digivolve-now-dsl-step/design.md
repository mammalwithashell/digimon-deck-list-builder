## Context

Six cards print the inherited effect "[End of Your Turn] This Digimon and any of your other Digimon may DNA digivolve into a Digimon card in the hand": BT12-021 Veemon, BT12-047 Wormmon, BT17-007 Agumon, BT17-019 Gabumon, BT22-008 Agumon, BT22-017 Gabumon.

DCGO (`BT22_008.cs:104-185` and siblings) implements this via an `EffectTiming.OnEndTurn` `ActivateClass` with `isOptional: true` whose coroutine calls `DNADigivolvePermanentsIntoHandOrTrashCard(...)`. When the trigger fires at end-of-turn, the player is prompted accept/decline; on accept, the DNA digivolve UI surfaces inline — pick partner permanent, pick target hand card — and the merged Digimon enters during the same EoT batch. Subsequent EoT triggers (e.g. Tai & Matt's `[End of Your Turn] [Once Per Turn] 1 of your Omnimon may attack a player`) resolve afterward with the new Digimon on field, completing chains like "play MG cost-reduced → Agumon→WG via MG effect → end-turn DNA digivolve into Omnimon → T&M EoT attack-a-player on Omnimon" on a single turn.

The Rust DSL currently authors these clauses with `alt_path_registration { kind: dna_digivolve, scope: inherited, trigger: end_of_your_turn }`. The mechanism's semantic is "register an alternative way to bring a card into play, available as a player-callable action in the action mask". When the registration trigger fires at EoT, it simply records the alt-path; the action becomes legal in the NEXT P0 main phase. The DNA digivolve never surfaces AT the trigger fire time. As a result, T&M's slot 2 EoT trigger fizzles (Omnimon isn't on field yet), and the user-intended chain can never complete on a single turn.

The engine has the underlying primitive (`EffectContext::effect_initiated_dna_digivolve` and variants — `code/digimon-engine/src/effect_context/mod.rs:4546`+). What's missing is a DSL step verb that wraps the primitive with the inline accept/decline + partner + target selection flow at trigger fire time.

The relevant ground-truth findings from the 2026-05-24 engine-MCP exploration:

- All 6 cards print **identical** EoT inherited text. The fix generalizes uniformly.
- `effect_initiated_dna_digivolve` fires `WhenDigivolving → OnDnaDigivolve → OnDigivolve (global)` each followed by a queue drain (per the docstring at `mod.rs:4624-4628`). Cascade behavior is already correct when called from inside an EoT drain — the post-DNA triggers (including the new Digimon's own enter-field triggers like Omnimon's On Play / All Turns) fire and drain before control returns to the outer EoT batch.
- No other YAML uses `alt_path_registration` for any non-EoT-DNA-digivolve purpose. ST20-10 matched on `alt_path` (without `_registration`) — a different mechanism for digivolve-under-condition that is unaffected.

## Goals / Non-Goals

**Goals:**

- Author `[End of Your Turn]` inherited DNA digivolve effects so the printed "may DNA digivolve into a Digimon card in the hand" surfaces **at EoT trigger fire** as a player choice, not as a registration for a later turn.
- Make the full Omnimon-line EoT chain (DNA digivolve → Omnimon On Play → T&M EoT attack-a-player → Omnimon attack → WG/MG inherited When Attacking) work on a single turn, matching DCGO.
- Provide a reusable DSL step verb `may_dna_digivolve_now` that future cards with similar printed text can use without re-implementing the orchestration.
- Migrate all 6 affected cards in a single change so their behavior aligns with the printed text uniformly.

**Non-Goals:**

- Removing `alt_path_registration` from the engine. The mechanism may have other valid uses (cross-turn registrations, conditional alt-paths) that are not in this proposal's scope. We deprecate it for the specific printed-text pattern this proposal addresses, not as a whole.
- Adding a generic "may digivolve / DNA digivolve / play" inline-action infrastructure. `may_attack_now` and `may_play_now` exist as parallel verbs; this proposal adds the matching DNA-digivolve verb but does not unify them into a single abstraction.
- Changing the DNA digivolve cost model. All 6 affected cards use `cost: 0` with `ignore_requirements: true`, matching DCGO. The step verb takes cost + ignore_requirements parameters; broader cost/restriction permutations are out of scope.
- Supporting hand-partner DNA digivolve (the BT17-095 shape from `effect_initiated_dna_digivolve_with_hand_partner`). All 6 affected cards require both materials to be on-field permanents.
- Touching the BT22-008/-017 `[On Play]` reveal-bucket clauses. Those are already correctly authored and unaffected.

## Decisions

**Decision 1: Author as a single inline DSL step (`may_dna_digivolve_now`), not as a multi-step composition of select_own_permanent + select_hand + effect_initiated_dna_digivolve.**

A multi-step composition would expose the partner-then-target selection sequence in YAML, which is closer to the printed text's mechanism. But it would:

- Require duplicating the orchestration across 6 cards.
- Surface awkward state if the player declines partway (does the partial selection roll back?).
- Diverge from `may_attack_now` / `may_play_now`'s precedent of single-step inline orchestration.

The single-step verb keeps the YAML compact and parallels existing inline-action steps. The step orchestrates all three sub-selections internally and either commits the DNA digivolve fully or rolls back fully.

**Decision 2: The `anchor` parameter defaults to `source` and constrains one material to the trigger's source permanent.**

The printed text reads "**This Digimon** and any of your other Digimon may DNA digivolve". DCGO encodes this via `permanentConditions: new Func<Permanent, bool>[] { (permanent) => permanent == card.PermanentOfThisCard() }` — the source permanent IS one of the two materials. The step verb makes this explicit via the `anchor` field rather than letting both partners be free-form selections that the player has to remember includes the source.

**Decision 3: Reuse existing `PermanentFilter` and `CardFilter` predicate types for `partner_filter` and `target_filter`.**

Both predicate types already exist in the DSL with mature lowering and predicate evaluation. The step verb just plugs them into its sub-selection prompts.

**Decision 4: The step is no-op (silent skip) when no eligible partner exists OR no eligible target exists.**

DCGO's `CanActivateCondition` returns false in these cases and the trigger does not surface a prompt. We mirror this: if `partner_filter` matches zero own-field permanents (excluding anchor) OR `target_filter` matches zero hand cards, the step is a clean no-op. The outer trigger's `optional: true` does not auto-decline in this case — it just resolves with no body run.

**Decision 5: Leave `alt_path_registration` in the engine but deprecate its use for the EoT-DNA-digivolve printed-text pattern.**

The 6 affected cards are the only YAML consumers of `alt_path_registration` for this specific printed text. After migration, the mechanism still exists but has zero card-script consumers. Removing it requires a separate audit (engine code may reference it for non-card-script paths). This proposal explicitly deprecates the YAML usage and updates the DSL spec to redirect authors to `may_dna_digivolve_now` for this pattern.

**Decision 6: Add the chain test to `bt22_008.rs` (Agumon) rather than a new integration test file.**

The chain originates from the BT22-008 inherited trigger; the natural test home is the BT22-008 behavioral file. Other Omnimon-line cards (BT22-017, BT17-007, BT17-019) get their own focused tests on the new step's contract (accept/decline, partner filter, target filter, no-op when empty); the full integration scenario lives with the card that drives it.

## Risks / Trade-offs

**[Risk] The 6 existing alt_path_registration tests will fail after the YAML migration.** → Mitigation: this is expected and explicit in the proposal. The tests pin observed-not-printed semantics; they get rewritten as part of this change to pin the correct semantics via `MayDnaDigivolveNow` step shape. Each test gets a single-commit update alongside its corresponding YAML migration.

**[Risk] The new step may interact poorly with the EoT trigger batch's `Once Per Turn` markers on other triggers.** → Mitigation: the step's call to `effect_initiated_dna_digivolve` fires `WhenDigivolving / OnDnaDigivolve / OnDigivolve` triggers, each draining individually. None of those timings should re-fire the EoT batch — they have different timings. Worth verifying via the chain test, which exercises T&M's slot 2 `[End of Your Turn] [Once Per Turn]` firing after the DNA digivolve. If the EoT batch re-fires the BT22 inherited via the new On-Digivolve trigger from Omnimon's entry, that's a recursion bug we'd surface here.

**[Risk] The partner_filter excludes the anchor, but the engine has to compute "permanent is anchor" reliably.** → Mitigation: anchor resolves to the trigger source's `PermanentHandle`. `PermanentHandle` equality is well-defined. The filter passes a `not: { same_as: anchor }` predicate to the underlying SelectPermanent installer. Existing `select_own_permanent` already supports binding-based exclusions for chained selections (e.g. picking material_a then excluding it from material_b).

**[Risk] The new step is a substantial DSL surface addition.** → Mitigation: the verb is narrowly scoped to "may DNA digivolve at trigger fire". It does not generalize to all possible DNA-digivolve patterns. Cards with materially different printed text (e.g. one material from hand, target from trash) will need different verbs or a future generalization. Documented in the DSL vocabulary spec as a single-purpose verb, not a general primitive.

**[Risk] Six YAMLs change behavior simultaneously.** → Mitigation: each YAML's test pins the new behavior with the chain test (for BT22-008) or focused tests (for the others). Behavioral surface is tractable. Tests run as part of the cards_behavioral suite.

## Migration Plan

1. **Extend the compiled DSL surface.**
   - `code/digimon-dsl/src/compiled/step.rs`: add `CompiledStep::MayDnaDigivolveNow { anchor, partner_filter, target_filter, cost, ignore_requirements, optional, prompt }`.
   - `code/digimon-dsl/src/parse/step.rs`: parse `may_dna_digivolve_now:` YAML keyword into the new variant.
   - `code/digimon-dsl/src/compile.rs`: lowering hook (resolve bindings, compile predicates).

2. **Implement the engine API.**
   - `code/digimon-engine/src/effect_context/mod.rs`: add `EffectContext::may_dna_digivolve_now(anchor, partner_filter, target_filter, cost, ignore_requirements, optional, prompt)`. Implementation orchestrates accept/decline → partner SelectPermanent → target Hand selection → call to existing `effect_initiated_dna_digivolve`.
   - `code/digimon-engine/src/dsl_cards/step/dna_digivolve.rs` (new file): step lowering that resolves `anchor` from bindings, compiles filter predicates, and calls `ctx.may_dna_digivolve_now(...)`.
   - `code/digimon-engine/src/dsl_cards/step/mod.rs`: register the new step in the dispatch table.

3. **Migrate the 6 YAMLs.**
   - For each of BT12-021, BT12-047, BT17-007, BT17-019, BT22-008, BT22-017:
     - Replace the `alt_path_registration` clause with a triggered clause:
       ```yaml
       - when: end_of_your_turn
         scope: inherited
         optional: true
         summary: "[EoT] (printed text)"
         process:
           - may_dna_digivolve_now:
               # anchor defaults to source (the inherited carrier)
               partner_filter:
                 all_of:
                   - of: you
                   - kind: digimon
                   - not: { same_as: source }
               target_filter:
                 all_of:
                   - of: you
                   - kind: digimon
                   # BT12-021, BT12-047 also add: name_contains: "Imperialdramon"
               cost: 0
               ignore_requirements: true
               prompt: "DNA digivolve at end of turn?"
       ```
   - BT12-021 and BT12-047 narrow the `target_filter` to include `name_contains: "Imperialdramon"` per printed text (their inherited specifies the partner-then-evolution into an Imperialdramon line). BT17-007/-019 and BT22-008/-017 leave `target_filter` at any own Digimon card in hand per printed text.

4. **Update existing behavioral tests.**
   - For each of the 6 cards, locate the test asserting the `AltPathRegistration` compiled-clause shape. Replace with assertions on the new triggered-clause shape containing a `MayDnaDigivolveNow` step. Where the printed text restricts the target, verify `target_filter` carries the restriction.

5. **Add the chain integration test.**
   - In `code/digimon-engine/tests/cards_behavioral/bt22/bt22_008.rs`, add a new test that runs the full scenario: T&M + BT22-008 pre-placed → play MG (cost-reduced via Matt) → Agumon→WG via MG mandatory effect → end_turn → DNA digivolve prompt surfaces inline → accept → pick WG as partner → pick Omnimon as target → assert Omnimon on field with stack `[Agumon, WG, MG, Omnimon]` → T&M's slot 2 prompts attack-a-player → accept → declare attack → assert opp security drops, WG inherited fires (sec trash), MG inherited fires (Omnimon unsuspend).

6. **Verify via engine-MCP QA replay** of the user's original Omnimon scenario from the 2026-05-24 session.

7. **Run the full `cards_behavioral` and `dsl` test suites** to catch any regressions, particularly around the cards whose inheriteds were affected.

**Rollback:** Revert the YAML migrations and engine/DSL additions. The `alt_path_registration` machinery returns to being the canonical authoring path for these 6 cards.

## Open Questions

- Should the YAML keyword be `may_dna_digivolve_now` (parallel to existing `may_attack_now`) or something shorter like `dna_digivolve_now`? → Use `may_dna_digivolve_now` for consistency with the existing `may_*_now` pattern, even though the printed text's "may" is captured by the outer `optional: true` flag.
- Where should the partner exclusion be encoded — inside the step's implementation as a hard exclusion of `anchor`, or via the YAML's `partner_filter` predicate? → Encode it as a hard exclusion inside the step's implementation. The `partner_filter` should not have to repeat the anchor-exclusion at every call site; that's an invariant of the verb, not a per-card choice.
- Does `effect_initiated_dna_digivolve` need extending to support the trigger-source provenance correctly for the new step's call? → Likely no — `effect_initiated_dna_digivolve_with_provenance` exists for this case (`mod.rs:4708`). The step uses the variant that threads `EffectSourceKind::Digimon` provenance from the trigger source.
- After migration, should the `alt_path_registration { kind: dna_digivolve }` machinery be removed in a follow-up change? → Yes, if no consumers emerge in the meantime. Track this as a follow-up note in `qa/dsl-vocab-gaps.md`.
- Should the BT22-008/-017 `[On Play]` reveal-bucket clauses be re-examined while we're touching these cards? → Out of scope. Those clauses are correctly authored per the 2026-05-20 sweep; touch nothing that isn't broken.
