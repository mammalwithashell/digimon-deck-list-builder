## ADDED Requirements

### Requirement: `may_dna_digivolve_now` step verb for inline DNA digivolve at trigger fire

The DSL SHALL provide a `may_dna_digivolve_now` step verb that, when executed inside a triggered clause's body, surfaces the DNA digivolve UI inline at trigger fire time and (on player accept) merges two on-field permanents plus a target hand card into a new Digimon. The step's contract is:

- `anchor`: PermanentRef (defaults to `source`) — one DNA material is fixed to this permanent.
- `partner_filter`: PermanentFilter — predicate over own-field permanents for the OTHER DNA material; the anchor SHALL be excluded automatically by the step implementation regardless of whether the filter mentions the exclusion.
- `target_filter`: CardFilter — predicate over the controller's hand for the result Digimon card.
- `cost`: u16 (defaults to 0) — memory cost paid before the merge.
- `ignore_requirements`: bool (defaults to false) — when true, bypasses the digivolve target's normal requirement checks.
- `optional`: bool (defaults to false) — when true, the step prompts the controller accept/decline before any material selection.
- `prompt`: Option<String> — optional override for the accept/decline prompt copy.

The step SHALL call `EffectContext::effect_initiated_dna_digivolve(anchor, partner, target_hand_card, cost, ignore_requirements)` after both selections resolve. The post-merge trigger cascade (`WhenDigivolving → OnDnaDigivolve → OnDigivolve` per the existing primitive's docstring) executes as part of the step's resolution, so the new Digimon's own enter-field effects fire before control returns to the surrounding trigger batch.

#### Scenario: Step prompts accept/decline when `optional: true`

- **WHEN** a triggered clause's body executes `may_dna_digivolve_now` with `optional: true`
- **THEN** the controller is prompted accept/decline via the engine's standard optional-step surface
- **AND** picking decline resolves the step with no state mutation

#### Scenario: Step selects partner from own field excluding the anchor

- **WHEN** the controller accepts the optional prompt (or the step has `optional: false`)
- **AND** the anchor permanent exists on the controller's battle area
- **THEN** the next pending selection is a `SelectionKind::SelectPermanent` over own-field permanents matching `partner_filter`
- **AND** the anchor permanent is excluded from the selection candidates regardless of whether `partner_filter` references the exclusion

#### Scenario: Step selects target from controller's hand

- **WHEN** the controller has selected a partner permanent
- **THEN** the next pending selection is a `SelectionKind::Hand` over the controller's hand matching `target_filter`
- **AND** only Digimon cards in the controller's hand are eligible (the verb's printed-text contract presumes a Digimon target)

#### Scenario: Step calls `effect_initiated_dna_digivolve` after both selections

- **WHEN** both partner and target selections resolve
- **THEN** the engine calls `EffectContext::effect_initiated_dna_digivolve(anchor, partner, target_hand_card.handle(), cost, ignore_requirements)`
- **AND** the post-merge trigger cascade (`WhenDigivolving → OnDnaDigivolve → OnDigivolve`) fires and drains as part of the step's resolution
- **AND** the new merged Digimon's own `[On Play]` / `[When Digivolving]` effects resolve before the outer trigger batch resumes

#### Scenario: Step is a clean no-op when no eligible partner or target exists

- **WHEN** the step executes with no own-field permanent matching `partner_filter` (other than the anchor)
- **OR** with no hand card matching `target_filter`
- **THEN** the step does NOT install a pending selection — no accept/decline prompt, no partner prompt, no target prompt
- **AND** the surrounding trigger resolves with no body effect (silent skip)

### Requirement: `alt_path_registration { kind: dna_digivolve }` is deprecated for `[End of Your Turn]` printed-text patterns

When a card's printed text reads "[End of Your Turn] This Digimon and any of your other Digimon may DNA digivolve into a Digimon card in the hand" (or a structurally equivalent inherited end-of-turn "may DNA digivolve" clause), the card YAML SHALL use `may_dna_digivolve_now` inside a triggered `end_of_your_turn` clause and SHALL NOT use `alt_path_registration { kind: dna_digivolve, scope: inherited, trigger: end_of_your_turn }` to express the same printed text. The `alt_path_registration` mechanism remains valid for cross-turn registrations or for alt-paths whose printed-text semantic genuinely deferred-availability; it is deprecated only for the inline at-EoT printed-text pattern.

#### Scenario: New card with [EoT] DNA digivolve inherited authors via `may_dna_digivolve_now`

- **WHEN** a card author adds a new YAML for a card whose printed inherited text reads "[End of Your Turn] This Digimon and any of your other Digimon may DNA digivolve into a Digimon card in the hand"
- **THEN** the YAML's inherited end-of-turn clause uses `may_dna_digivolve_now`
- **AND** the YAML does NOT use `alt_path_registration { kind: dna_digivolve }` for this clause

#### Scenario: Migration of legacy alt_path_registration cards

- **WHEN** a previously authored card uses `alt_path_registration { kind: dna_digivolve, scope: inherited, trigger: end_of_your_turn }` for the printed inherited EoT DNA digivolve pattern
- **THEN** the card's YAML is migrated to `may_dna_digivolve_now` as part of this change (or a follow-up audit)
- **AND** the card's behavioral test is updated to assert the new clause shape
