# Track K: Cross-Card Effect Re-Firing

Saved from the 2026-05-10 prompt for the deferred Track K Rust engine work.

## Goal

Land the one-shot cross-card effect re-firing mechanic in `code/digimon-engine/`, using BT24-102 Homeros as the canonical card-shaped fixture and DCGO as a reference for dispatch shape only.

This mechanic covers printed text shaped like: choose another permanent and activate one of its `[On Play]` or `[When Digivolving]` effects.

It is not a fake play and not a fake digivolution:

- The target permanent stays in play.
- The target permanent is not digivolved and does not consume a source.
- The chosen effect runs with carrier semantics: "this Digimon" resolves to the target permanent.
- Source attribution remains the refire grantor: "this card's effect" resolves to the grantor.
- The target's once-per-turn effect slots are respected unless printed text explicitly bypasses them.

## Source Priority

Use sources in this order:

1. Printed card text in `data/cards.json`.
2. `docs/RULES_CONTEXT.md` and the canonical rules PDF if needed.
3. Fandom ruling notes and errata context.
4. DCGO C# source as an implementation reference and tiebreaker for processing shape only.

Printed text and rules win over DCGO.

## Required Reading

Read in order before implementation:

1. `CLAUDE.md`, especially no-approximations, TDD, and parity tracker rules.
2. `docs/RUST_ENGINE_API.md`.
3. `code/digimon-engine/src/effect.rs`.
4. `code/digimon-engine/src/effect_context/mod.rs`.
5. `code/digimon-engine/src/effect_queue.rs`.
6. `code/digimon-engine/src/permanent.rs`.
7. `code/digimon-engine/src/selection.rs`.
8. `code/digimon-engine/src/action/mask.rs`.
9. `code/digimon-engine/src/action/decoder.rs`.
10. `code/digimon-engine/src/enums.rs`.
11. Existing tests under `code/digimon-engine/tests/effect_context/`, especially `effect_refiring.rs`.
12. Existing BT24 behavioral fixture patterns, especially `tests/cards_behavioral/bt24/bt24_062.rs`.
13. BT24-102 Homeros printed text in `data/cards.json`.
14. DCGO reference files:
    - `DCGO/Assets/Scripts/Script/Effects.cs`
    - `DCGO/Assets/Scripts/Script/MultipleSkills.cs`
    - `DCGO/Assets/Scripts/Script/OptionalSkill.cs`
    - `DCGO/Assets/Scripts/Script/CardEffects/ActivateClass.cs`
    - `DCGO/Assets/Scripts/Script/MainPhaseAction/ActivateCardAction.cs`
    - `DCGO/Assets/Scripts/Script/MainPhaseAction/ActivatePermanentAction.cs`
    - `DCGO/Assets/Scripts/Script/CardEffects/AddSkillClass.cs`
15. `qa/archetype-qa/dsl/ts-olympos-2026-05-03-dsl-engine-gaps.md`.

## Engine Helper

Add an `EffectContext` helper with naming matched to local conventions:

```rust
pub fn refire_target_effect(
    &mut self,
    target: PermanentHandle,
    timing_filter: TimingFilter,
    selecting_player: PlayerId,
    bypass_once_per_turn: bool,
) -> bool;
```

Semantics:

- Return `true` when at least one eligible effect exists and is queued or completed.
- Return `false` when no eligible effect exists.
- Do not panic on no eligible effect.
- If one effect is eligible, invoke directly.
- If multiple effects are eligible, install a pending selection for `selecting_player`.
- Carrier/source split:
  - `source_card` remains the refire grantor.
  - `source_permanent` becomes the target permanent.
- Once-per-turn:
  - Consult the target permanent's existing accounting.
  - Mark the chosen target effect consumed through the same accounting path.
  - Only bypass when `bypass_once_per_turn` is true and the printed source permits it.

Document the helper in `docs/RUST_ENGINE_API.md`, including BT24-102 Homeros as the motivating fixture.

## Permanent Effect Enumeration

Add or expose a permanent effect query if absent:

- Walk top card plus inherited source effects.
- Filter by timing (`OnPlay`, `WhenDigivolving`, either, or all as required).
- Exclude already-consumed once-per-turn slots unless bypassing.
- Return stable effect identifiers usable for selection and invocation.

The inherited-stack walk must match established stack-walking patterns such as Track G keyword checks.

## Pending Selection

For multi-effect targets, reuse existing pending-selection and action-mask machinery.

Requirements:

- No new `ACTION_SPACE_SIZE`.
- One discrete action per eligible effect.
- Human-readable labels come from `Effect::name` or the local equivalent.
- The target choice and effect choice must both surface through pending selection and the mask.

If the current selection system cannot express this, stop and file a separate contract gap instead of adding a private mask path.

## Invocation Rules

Refire reuses the normal effect dispatch path with attribution overrides. It must not route through play-by-effect or digivolve-by-effect movement.

Pseudocode shape:

```rust
let mut sub_ctx = EffectContext::new(
    self.game,
    self.source_card,
    Some(target),
    selecting_player,
);
chosen_effect.process(&mut sub_ctx);
```

Exact construction must follow actual engine APIs.

Important invariants:

- Refired `[On Play]` does not fire `OnAnyDigimonPlayed`.
- Refired `[When Digivolving]` does not fire `OnDigivolve`.
- Refire is one-shot and distinct from Track H granted-triggered abilities.

## DSL Surface

Add schema and lowering for:

```yaml
effects:
  - kind: refire_target_effect
    target:
      filter:
        all_of:
          - trait: olympos_xii
          - controller: self
    timing:
      any_of:
        - on_play
        - when_digivolving
    selecting_player: self
    bypass_once_per_turn: false
```

The lowerer should call the Rust helper and reuse existing permanent selector lowering.

## BT24-102 Homeros Fixture

Add:

- `code/digimon-engine/cards/bt24/BT24-102.yaml`
- `code/digimon-engine/tests/cards_behavioral/bt24/bt24_102.rs`

Behavioral coverage:

- Printed cost is paid according to Homeros text.
- Choosing an Olympos XII target with multiple eligible effects opens effect-choice selection.
- The chosen effect resolves with carrier = target, not Homeros.
- Grantor attribution remains Homeros.
- Once-per-turn slots on the target effect are consumed.
- A second same-turn refire excludes consumed target effects.
- No eligible effects returns false and does not panic.
- Refired `[On Play]` does not dispatch `OnAnyDigimonPlayed`.
- Refired `[When Digivolving]` does not dispatch `OnDigivolve`.

## Helper Unit Tests

Use TDD. Cover:

- No eligible effects returns false.
- Exactly one eligible effect invokes directly.
- Multiple eligible effects installs pending selection on the selecting player.
- Carrier semantics resolve target permanent properties.
- Source attribution resolves grantor card properties.
- Once-per-turn slots are consumed after invocation.
- `bypass_once_per_turn: true` invokes despite consumed slots.
- Inherited effects are enumerable.
- Timing filters narrow correctly.
- Refire does not fire play or digivolve observers.
- Modifier interactions are documented or filed as gaps if unsupported.

## Cross-Track Tests

Cover or explicitly document:

- Track H granted-triggered abilities are not conflated with one-shot refire.
- Track A payloads distinguish grantor and target.
- Track G keyword operations applied by a refired effect target the carrier while attributing to the grantor.
- Track C modifier rejection behavior is respected or filed as a gap.

## Trackers and Docs

Update:

- `docs/RUST_ENGINE_GAPS.md`
- `qa/archetype-qa/engine-gaps.md`
- `qa/dsl-vocab-gaps.md`
- `qa/archetype-qa/dsl/ts-olympos-2026-05-03-dsl-engine-gaps.md`
- `docs/RUST_ENGINE_API.md`

Mark entries closed, narrowed, or follow-up blocked with the verification command that proves the new status.

## Verification

Run as much as possible in this order:

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test effect_context effect_refiring
cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral bt24_102
cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl refire
cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral
cargo test --manifest-path code/digimon-engine/Cargo.toml
```

Watch for action-mask or pending-selection parity expectation changes. Do not change tensor/action contracts as part of this track.
