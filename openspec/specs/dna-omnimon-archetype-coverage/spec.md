# dna-omnimon-archetype-coverage Specification

## Purpose

Define the end-state coverage guarantees for the DNA Omnimon archetype in the
Rust DSL engine: every card in the pool has a faithful DSL implementation and
behavioral test coverage, the verdict ledger and gap trackers reflect the
verified state, and no test is ignored for an already-closed substrate gap.
## Requirements
### Requirement: Every DNA Omnimon card has faithful DSL implementation

Every unique card in the DNA Omnimon decklist pool (as resolved from `data/deck_library.json`) SHALL have a production DSL YAML file under `code/digimon-engine/cards/<set>/` whose effects faithfully implement the full printed card text from `data/cards.json` — every clause, timing, and player choice. No clause may be omitted, stubbed, hidden behind `raw_rust`, auto-resolved, or represented by a coarser proxy.

#### Scenario: Card pool fully authored

- **WHEN** the DNA Omnimon card pool is resolved from `data/deck_library.json`
- **THEN** each unique card ID has a corresponding `code/digimon-engine/cards/<set>/<CARD-ID>.yaml` file
- **AND** the previously unauthored cards BT22-084, BT17-007, ST2-13, BT5-093, and AD1-019 each have production YAML

#### Scenario: Card clauses are complete, not approximated

- **WHEN** a DNA Omnimon card's YAML is reviewed against its printed text in `data/cards.json`
- **THEN** every printed clause (main, inherited, security, when-digivolving, alt-path) is represented in the YAML
- **AND** no clause is replaced by a no-op, a hidden auto-selection, a `raw_rust` escape, or a coarser proxy

#### Scenario: BT17-102 dynamic source names are implemented

- **WHEN** BT17-102 Greymon has level 3 or lower cards in its digivolution cards
- **THEN** the engine treats that Digimon as having all names of those source cards for relevant name checks
- **AND** the DSL implementation does not rely on a hardcoded Koromon-source proxy for the all-turns name behavior

#### Scenario: BT23-096 Delay fires from ally CS attack

- **WHEN** BT23-096 Comet Hammer is in the battle area as a delayed option during the player's turn
- **AND** one of that player's `[CS]` trait Digimon attacks
- **THEN** the Delay effect can be declared through normal pending-selection/action-mask flow
- **AND** resolving it trashes BT23-096 and performs the printed de-digivolve effect

### Requirement: Every DNA Omnimon card has behavioral test coverage

Every DNA Omnimon card SHALL have a behavioral test file under `code/digimon-engine/tests/cards_behavioral/<set>/` that exercises its card text via `DebugRunner`. Tests SHALL be written before or alongside the YAML they cover, and the previously partial BT17-102 and BT23-096 clauses SHALL have enabled behavioral coverage.

#### Scenario: Behavioral test exists per card

- **WHEN** the DNA Omnimon card pool is enumerated
- **THEN** each card has a behavioral test file covering its printed clauses
- **AND** the test suites `cards_behavioral`, `dsl`, `dna_digivolve`, and `digivolve` pass with no regressions

#### Scenario: Partial-card tests are enabled

- **WHEN** the change is complete
- **THEN** BT17-102's dynamic source-name alias test is not ignored
- **AND** BT23-096's Delay-on-ally-attack test is not ignored
- **AND** both tests pass against production DSL YAML

### Requirement: No behavioral test is ignored for an already-closed gap

No DNA Omnimon behavioral test SHALL carry an `#[ignore]` marker that cites a substrate gap which is already closed in the current engine/DSL. Each `#[ignore]` marker that remains SHALL cite a substrate gap that is verifiably still open, confirmed by inspecting the current engine code — not by trusting a tracker document.

#### Scenario: Stale ignore markers are re-enabled

- **WHEN** a DNA Omnimon behavioral test is ignored citing `pending: G-XYZ`
- **AND** the engine/DSL primitive `G-XYZ` is confirmed present in `code/digimon-engine/src/` or `code/digimon-dsl/src/`
- **THEN** the test is re-enabled, its card clause is authored, and the test passes

#### Scenario: Genuinely-blocked tests carry accurate references

- **WHEN** a DNA Omnimon behavioral test remains ignored after the reconciliation sweep
- **THEN** its `#[ignore]` reason cites a substrate gap verified as still open against current code
- **AND** that gap has a corresponding open entry in `qa/dsl-vocab-gaps.md` or `docs/RUST_ENGINE_GAPS.md`

### Requirement: An accurate per-card verdict ledger exists

A `validated_cards_dsl.json` verdict ledger SHALL contain an entry for every DNA Omnimon card, and every entry SHALL have a verdict of `IMPLEMENTED` after the change completes. No DNA Omnimon entry may remain `PARTIAL` or `BLOCKED` after BT17-102 and BT23-096 pass their behavioral tests.

#### Scenario: Ledger covers the full pool

- **WHEN** the reconciliation sweep completes
- **THEN** `validated_cards_dsl.json` has one entry per DNA Omnimon card
- **AND** every entry has verdict `IMPLEMENTED`

#### Scenario: Former partial cards are promoted

- **WHEN** BT17-102 and BT23-096 behavioral tests pass with their omitted clauses enabled
- **THEN** their ledger entries are updated from `PARTIAL` to `IMPLEMENTED`
- **AND** their former gap IDs are recorded as closed in the appropriate tracker updates

### Requirement: raw_rust escapes are minimized and documented

DNA Omnimon card YAML SHALL contain zero live `raw_rust` escapes. Historical comments may mention retired raw-Rust migrations, but no DNA Omnimon production YAML may use `kind: raw_rust` or reference a raw Rust function to implement card behavior.

#### Scenario: Now-expressible escapes are migrated

- **WHEN** a DNA Omnimon card YAML contains a live `raw_rust` escape
- **AND** the behavior is expressible with current DSL vocabulary
- **THEN** the escape is rewritten as pure DSL and the card's behavioral test still passes

#### Scenario: No live raw_rust remains

- **WHEN** the DNA Omnimon card pool YAML files are scanned
- **THEN** no non-comment YAML entry contains `kind: raw_rust`
- **AND** no DNA Omnimon card behavior depends on a raw Rust card-function registry entry

### Requirement: DNA Omnimon trackers reflect verified state

After the change, `qa/dsl-vocab-gaps.md`, `qa/resolved-gaps.md`, and `qa/archetype-qa/dsl/2026-05-03-dna-omnimon-dsl-engine-gaps.md` SHALL reflect the verified end state: closed gaps moved to `resolved-gaps.md`, still-open gaps left open with accurate card attributions.

#### Scenario: Closed gaps relocated

- **WHEN** a DNA Omnimon gap is verified closed during the change
- **THEN** its entry is moved to `qa/resolved-gaps.md` with a closure note
- **AND** the per-archetype gap doc annotates the closed item

#### Scenario: No closed gap left marked open

- **WHEN** the change completes
- **THEN** no DNA Omnimon gap that is verified closed remains listed as open in `qa/dsl-vocab-gaps.md`

### Requirement: BT17-081 activation-cost gate enforces per-trigger suspend payability

BT17-081 Tai Kamiya & Matt Ishida's `[All Turns]` triggered clause SHALL encode its "by suspending this Tamer" cost as a leading `activation_cost: { suspend_self: true }` body step (the BT13-101 / P-136 idiom), which the engine's `lower_triggered::lower_for_kind_with_clause_index` lifts onto `EffectBuilder::activation_cost(...)`. The clause SHALL NOT use a body-step `suspend: { target: source }` for this cost. The engine SHALL evaluate the lifted activation cost per-queued-trigger via `EffectContext::suspend_self_as_cost`, so simultaneous triggers from the same event chain grant the printed memory reward **at most once** — subsequent triggers inert when BT17-081 is already suspended and cannot pay the cost.

#### Scenario: Single trigger pays cost and grants memory

- **WHEN** one of the controller's Digimon is played or digivolves AND BT17-081 is unsuspended
- **AND** the trigger is resolved (either by picking it from a `TriggerOrder` bundle, or via the engine's auto-fire path when only one trigger is queued for the active chooser)
- **THEN** `EffectContext::suspend_self_as_cost` returns true and BT17-081 suspends as the cost payment
- **AND** the controller gains +1 memory for each Greymon-name Digimon present on their battle area at resolution time
- **AND** the controller gains +1 memory for each Garurumon-name Digimon present on their battle area at resolution time

#### Scenario: Second sequential trigger inerts when cost cannot be paid

- **WHEN** two BT17-081 `[All Turns]` triggers fire sequentially on the same turn (e.g. two own Digimon plays, or a play plus a same-chain digivolve)
- **AND** the first trigger resolves: `suspend_self_as_cost` returns true, BT17-081 suspends, body runs, memory is granted
- **AND** the second trigger then resolves
- **THEN** `EffectContext::suspend_self_as_cost` returns false because BT17-081 is already suspended
- **AND** the second trigger's body does not run — no additional memory is granted, BT17-081 is not double-suspended

#### Scenario: Trigger inerts when source is pre-suspended

- **WHEN** BT17-081 is suspended at the moment a play or digivolve event fires its `[All Turns]` trigger
- **AND** the trigger's activation_cost_fn is invoked (`suspend_self_as_cost`)
- **THEN** the cost call returns false and the body silently skips
- **AND** no memory is granted, no state corruption occurs, the engine does not panic

#### Scenario: Test fixture has memory headroom for gains

- **WHEN** a behavioral test exercises BT17-081's `[All Turns]` memory grants
- **THEN** the test fixture sets the starting memory to a value with at least 2 points of headroom inside `Rules::standard().memory_range` (e.g. memory == 0), so `gain_memory(+1)` calls don't clamp at the seesaw boundary and produce false-positive zero deltas

### Requirement: BT17-081 simultaneous-trigger behavior has regression coverage

A behavioral test in `code/digimon-engine/tests/cards_behavioral/bt17/bt17_081.rs` SHALL exercise the simultaneous-trigger case end-to-end, asserting the controller's memory delta is exactly +2 (not +4) when two BT17-081 triggers queue from the same event chain with one Greymon-name and one Garurumon-name Digimon on field.

#### Scenario: Two simultaneous triggers grant memory once

- **WHEN** a behavioral test constructs a board with BT17-081 (unsuspended), a Greymon-named Digimon, and a Garurumon-named Digimon on the controller's field
- **AND** the test triggers two BT17-081 `[All Turns]` activations in a single event chain
- **AND** the test resolves both triggers in TriggerOrder
- **THEN** the controller's memory delta attributable to BT17-081 is exactly +2
- **AND** BT17-081 is suspended exactly once

### Requirement: BT22-008, BT22-017, BT17-007, BT17-019 inherited EoT DNA digivolve surfaces inline at trigger fire

The four Omnimon-line inherited carriers — BT22-008 Agumon, BT22-017 Gabumon, BT17-007 Agumon (Tai-themed), and BT17-019 Gabumon (Matt-themed) — SHALL author their `[End of Your Turn]` inherited DNA digivolve clause using a triggered clause with `scope: inherited`, `optional: true`, and a body invoking `may_dna_digivolve_now`. The clause SHALL surface the DNA digivolve player choice inline at end-of-turn trigger resolution, NOT defer it via `alt_path_registration` to a subsequent turn.

#### Scenario: BT22-008 inherited prompts inline at EoT

- **WHEN** BT22-008 (or a permanent stack with BT22-008 in its digivolution cards) is on the controller's field at end of the controller's turn
- **AND** the controller has at least one other own-field Digimon eligible as a DNA digivolve partner
- **AND** the controller's hand contains at least one Digimon card eligible as the DNA digivolve target
- **THEN** the engine surfaces an accept/decline prompt for the BT22-008 inherited EoT DNA digivolve as part of the EoT trigger batch resolution
- **AND** on accept, the controller picks partner and target inline, and the merged Digimon enters the battle area as part of the same EoT batch

#### Scenario: BT22-017 inherited prompts inline at EoT

- **WHEN** BT22-017 (or a permanent stack with BT22-017 in its digivolution cards) is on the controller's field at end of the controller's turn
- **AND** the controller has at least one other own-field Digimon eligible as a DNA digivolve partner
- **AND** the controller's hand contains at least one Digimon card eligible as the DNA digivolve target
- **THEN** the same inline prompt sequence as BT22-008's scenario fires

#### Scenario: BT17-007 inherited prompts inline at EoT

- **WHEN** BT17-007 is on field at end of controller's turn AND a partner + target are eligible
- **THEN** the same inline prompt sequence fires

#### Scenario: BT17-019 inherited prompts inline at EoT

- **WHEN** BT17-019 is on field at end of controller's turn AND a partner + target are eligible
- **THEN** the same inline prompt sequence fires

### Requirement: Omnimon-line EoT chain completes on a single turn

After the controller plays MetalGarurumon (cost-reduced via a Tamer with Matt Ishida in its name), uses MG's mandatory `[On Play] [When Digivolving]` effect to digivolve their Agumon (a BT22-008 carrier) into WarGreymon, and ends their turn, the engine SHALL surface and resolve the following EoT chain in a single turn:

1. BT22-008 inherited DNA digivolve prompt — accept → pick WG as partner → pick Omnimon as target.
2. The merged Omnimon enters the battle area; its `[On Play] [When Digivolving]` effect returns opponent Digimon (with ≤ Omnimon's digivolution-card count) and may delete an opponent Digimon.
3. Omnimon's `[All Turns] [Once Per Turn]` triggers if an opponent Digimon leaves the battle area, trashing one of their Option cards in the battle area and trashing their top security card.
4. Tai & Matt's `[End of Your Turn] [Once Per Turn] 1 of your Omnimon may attack a player` trigger fires — accept → designate Omnimon to attack.
5. Omnimon attacks opponent security; the BT17-015 WG inherited `[When Attacking] [Once Per Turn]` trashes the top of opponent security; the BT17-027 MG inherited `[When Attacking] [Once Per Turn]` unsuspends Omnimon (allowing follow-up attacks before turn rotates).

The full chain SHALL complete before the turn rotates to the opponent.

#### Scenario: Single-turn Omnimon EoT chain via Agumon line

- **WHEN** a behavioral test constructs the Agumon-line scenario as described
- **AND** the controller resolves each prompt in the EoT chain in the order listed above
- **THEN** Omnimon is on the field at end of resolution with stack `[Agumon, WG, MG, Omnimon]`
- **AND** opponent security count has decreased by at least 2 (one from Omnimon's All Turns, one from WG inherited When Attacking; plus the actual attack consumption)
- **AND** Omnimon is unsuspended (MG inherited unsuspend resolved)
- **AND** the turn has not yet rotated to the opponent when the chain completes
