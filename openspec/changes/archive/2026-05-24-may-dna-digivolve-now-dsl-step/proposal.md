## Why

Six cards print the inherited effect **"[End of Your Turn] This Digimon and any of your other Digimon may DNA digivolve into a Digimon card in the hand"** (BT12-021 Veemon, BT12-047 Wormmon, BT17-007 Agumon, BT17-019 Gabumon, BT22-008 Agumon, BT22-017 Gabumon). DCGO (`BT22_008.cs:104-185` and siblings) implements this as an `EffectTiming.OnEndTurn` `ActivateClass` that, when activated, **synchronously** runs `DNADigivolvePermanentsIntoHandOrTrashCard(...)` — the DNA digivolve UI surfaces inline at EoT trigger fire, materials and target are picked, the new Digimon enters during the same EoT batch, and subsequent EoT triggers (e.g. Tai & Matt's `[End of Your Turn] [Once Per Turn] 1 of your Omnimon may attack a player`) fire after with the new Digimon on field.

The Rust DSL currently expresses these clauses with `alt_path_registration { kind: dna_digivolve, scope: inherited, trigger: end_of_your_turn }`, which **registers a `dna_digivolve` action** that becomes legal in the next P0 main phase. The DNA digivolve never surfaces AT EoT — the registration trigger resolves silently, and the player has to use a separate `step` action on a later turn to actually DNA digivolve.

A May 24 2026 engine-MCP QA pass walked the user-intended chain (play MG cost-reduced → Agumon→WG via MG's effect → end-of-turn DNA digivolve into Omnimon → T&M's EoT attack-a-player triggers Omnimon's attack on opp security) and observed the break: the BT22-008 inherited registered the alt-path action, T&M's slot 2 fizzled because Omnimon wasn't on field yet, and the chain only completed across two separate P0 turns. DCGO's chain completes on a single turn.

The engine already exposes the underlying primitive (`EffectContext::effect_initiated_dna_digivolve` and its hand-partner / provenance variants); what's missing is a **DSL step verb** that surfaces the optional player choice inline at trigger fire time and orchestrates the partner + target selections before calling the engine primitive. This change adds that verb (`may_dna_digivolve_now`), migrates all 6 cards to use it, and updates the existing tests that pin the alt_path_registration shape.

## What Changes

- **Add `CompiledStep::MayDnaDigivolveNow` to the compiled DSL surface** (`code/digimon-dsl/src/compiled/step.rs`). Fields:
  - `anchor: PermanentRef` — the source card's permanent handle (the BT22-008 / etc. inherited carrier). Mirrors the printed "This Digimon" — one of the two DNA materials is fixed to the trigger's source permanent.
  - `partner_filter: PermanentFilter` — predicate over own-field permanents for the OTHER DNA material. Excludes the anchor.
  - `target_filter: CardFilter` — predicate over the controller's hand for the result Digimon. The card the DNA-digivolve target lands as.
  - `cost: u16` — memory cost (zero for all 6 affected cards).
  - `ignore_requirements: bool` — bypasses normal digivolution requirement checks (true for all 6).
  - `optional: bool` — whether the outer activation prompts accept/decline (true for all 6 — printed "may").
  - `prompt: Option<String>` — optional override for the accept/decline prompt copy.
- **Parse `may_dna_digivolve_now:` as a YAML step keyword** in `code/digimon-dsl/src/parse/step.rs`. Schema matches the CompiledStep field set, with `anchor` defaulting to `source` (the trigger's source permanent ref) when omitted.
- **Add `EffectContext::may_dna_digivolve_now(anchor, partner_filter, target_filter, cost, ignore_requirements, optional, prompt)`** in `code/digimon-engine/src/effect_context/mod.rs`. Implementation:
  1. If `optional`, install accept/decline prompt; on decline the step is a no-op.
  2. On accept (or if not optional), install `SelectPermanent` over own-field permanents matching `partner_filter` (excluding `anchor`).
  3. After partner selected, install `Hand` selection over the controller's hand matching `target_filter`.
  4. After target hand card selected, call `effect_initiated_dna_digivolve(anchor, partner, target_hand_card.handle(), cost, ignore_requirements)` — the existing primitive fires `WhenDigivolving` → `OnDnaDigivolve` → `OnDigivolve` + drain in sequence.
  5. If no eligible partner OR no eligible target exists at the point of activation, the step is a no-op and the outer trigger resolves silently (matches DCGO's `CanActivateCondition` returning false).
- **Add step lowering** in `code/digimon-engine/src/dsl_cards/step/dna_digivolve.rs` (new file or extension of an existing step module) that resolves `anchor` / `partner_filter` / `target_filter` from compiled bindings and calls `ctx.may_dna_digivolve_now(...)`.
- **Migrate 6 card YAMLs** from `alt_path_registration { kind: dna_digivolve, scope: inherited, trigger: end_of_your_turn }` to a triggered `when: end_of_your_turn` clause with `scope: inherited`, `optional: true`, and a process body of `[ - may_dna_digivolve_now: { ... } ]`. Affected files:
  - `code/digimon-engine/cards/bt12/BT12-021.yaml` (target_filter narrowed to Imperialdramon-name)
  - `code/digimon-engine/cards/bt12/BT12-047.yaml` (target_filter narrowed to Imperialdramon-name)
  - `code/digimon-engine/cards/bt17/BT17-007.yaml` (target_filter: any own Digimon in hand)
  - `code/digimon-engine/cards/bt17/BT17-019.yaml` (target_filter: any own Digimon in hand)
  - `code/digimon-engine/cards/bt22/BT22-008.yaml` (target_filter: any own Digimon in hand)
  - `code/digimon-engine/cards/bt22/BT22-017.yaml` (target_filter: any own Digimon in hand)
- **Update existing behavioral tests** on the 6 affected cards that currently assert the `AltPathRegistration` compiled-clause shape. Replace those assertions with assertions on the new `MayDnaDigivolveNow` step shape inside a `Triggered` clause. Where the printed text restricts the DNA digivolve target (BT12-021/-047's Imperialdramon-name target), verify the `target_filter` carries the restriction.
- **Add a chain test** under `code/digimon-engine/tests/cards_behavioral/bt22/bt22_008.rs` (or a new integration test) reproducing the user's scenario: T&M + BT22-008 Agumon pre-placed → play MG (cost-reduced) → Agumon→WG via MG's effect → end_turn → DNA digivolve prompt surfaces inline → accept → pick WG as partner → pick Omnimon as target → assert Omnimon on field with stack `[Agumon, WG, MG, Omnimon]` → T&M's slot 2 prompts → declare attack-a-player → assert WG inherited sec-trash + MG inherited unsuspend resolve. This pins the full chain working on a single turn, matching DCGO.
- **Leave `alt_path_registration { kind: dna_digivolve }` in place as engine machinery** for now. No card script references it after this change, but other future cards or cross-turn DNA-digivolve registrations may want it. A follow-up change can remove it if no consumers emerge.

## Capabilities

### New Capabilities

(none — all changes modify existing capabilities)

### Modified Capabilities

- `dsl-card-scripting-vocabulary`: new `may_dna_digivolve_now` step verb. Documents the contract: outer `optional` controls accept/decline, `anchor` defaults to `source`, `partner_filter` excludes the anchor, `target_filter` selects from the controller's hand, `cost: 0 + ignore_requirements: true` is the common shape for inherited-printed "may DNA digivolve" triggers. The existing `alt_path_registration` mechanism is documented as **deprecated for the EoT DNA digivolve printed-text pattern** in favor of the new step.
- `dna-omnimon-archetype-coverage`: BT12-021, BT12-047, BT17-007, BT17-019, BT22-008, BT22-017 are migrated to the new step. The EoT DNA digivolve choice surfaces inline AT end of turn, matching DCGO and unblocking the user-intended Omnimon-line chain (DNA digivolve → T&M EoT attack → Omnimon attack-a-player → WG/MG inherited When Attacking effects) on a single turn.

## Impact

- **Rust DSL** — `code/digimon-dsl/src/compiled/step.rs` (new `CompiledStep` variant), `code/digimon-dsl/src/parse/step.rs` (new YAML keyword), `code/digimon-dsl/src/compile.rs` (lowering hook).
- **Rust engine** — `code/digimon-engine/src/effect_context/mod.rs` (new `may_dna_digivolve_now` method), `code/digimon-engine/src/dsl_cards/step/dna_digivolve.rs` (new step lowering, or extension of an existing step module), `code/digimon-engine/src/dsl_cards/step/mod.rs` (register the new step in the dispatch table).
- **Card YAMLs** — 6 files migrate from `alt_path_registration` to the new step (BT12-021, BT12-047, BT17-007, BT17-019, BT22-008, BT22-017).
- **Behavioral tests** — 6 existing tests update their compiled-clause assertions; 1 new integration test pins the full Omnimon-line EoT chain end-to-end.
- **Specs** — modified deltas to `dsl-card-scripting-vocabulary` (new verb) and `dna-omnimon-archetype-coverage` (BT12 / BT17 / BT22 inherited EoT DNA digivolve printed-text contract).
- **No agent retraining required immediately** — the action space gains a new step kind but only when one of the 6 cards is on field at EoT with valid materials and a target. Agents that previously couldn't trigger the alt_path_registration's effect now see an extra prompt at EoT. The expanded action surface is small and bounded.
- **No breaking API changes** to MCP / Python bindings. The new step surfaces through the same `pending_selection` machinery as existing optional triggered effects.
- **No new dependencies**.
