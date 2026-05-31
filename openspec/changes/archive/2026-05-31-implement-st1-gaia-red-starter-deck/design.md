## Context

ST-1 Gaia Red has complete local card metadata in `data/cards.json` and JSON card records under `code/digimon-engine/cards/st1/`, but only `ST1-07` has production Rust DSL YAML and behavioral tests. The Fandom deck page identifies the worldwide starter deck as 54 playable cards: 4 `ST1-01` Digi-Eggs plus a 50-card main deck across `ST1-02` through `ST1-16`.

Most ST-1 effects fit existing DSL idioms:

- Inherited DP and keyword auras use `scope: inherited`, `kind: aura`, `dp_modifier`, `grant_keyword`, and dynamic formulas.
- Digimon/tamer/option security effects use existing `when: on_security` patterns.
- Option `[Main]` effects can use `when: main_from_hand` and mirror the body under `on_security`.
- Targeted deletion and DP buffs use existing pending-selection steps and modifiers.

Two card-text shapes are not safe to approximate:

- `ST1-09` MetalGreymon: inherited `[Your Turn] When this Digimon is blocked, gain 3 memory.`
- `ST1-14` Starlight Explosion: `[Main] All of your Security Digimon get +7000 DP until the end of your opponent's next turn` and `[Security] All of your Security Digimon get +7000 DP for the turn.`

The implementation must preserve the repository's no-approximations policy: every real player choice must flow through action masks or pending selections, and reusable gaps must be tracked as reusable primitives rather than card-local TODOs.

## Goals / Non-Goals

**Goals:**

- Add faithful production YAML for every unique Gaia Red card `ST1-01` through `ST1-16`.
- Add behavioral tests for every effect-bearing ST-1 card and registry/deck tests for no-effect cards.
- Add a printed Gaia Red starter deck fixture with exact card counts.
- Add reusable DSL/engine support for `ST1-09` blocked-attack triggers.
- Add reusable DSL/engine support for `ST1-14` defender-side Security Digimon DP buffs.
- Keep the active action space and tensor contracts unchanged.

**Non-Goals:**

- Do not implement Korean-only ST-1 bonus promo cards.
- Do not add legacy Python card scripts.
- Do not change frontend deckbuilding UX beyond consuming a fixture if the existing data path already surfaces it.
- Do not expand `ACTION_SPACE_SIZE`, tensor profiles, PyO3 constants, or model metadata as part of this change.
- Do not claim archetype/deck readiness while any ST-1 card still has an ignored test for a real open gap.

## Decisions

### Use production YAML for every card, including vanilla cards

Add YAML for `ST1-01` through `ST1-16`, not only effect-bearing cards. Vanilla cards such as `ST1-02`, `ST1-04`, `ST1-05`, and `ST1-10` should compile as production DSL cards with metadata, alt digivolution paths, and empty effects. This ensures `load_implemented_card_ids()` treats the complete printed starter deck as implemented and lets training/deck tools use the full deck without filtering away no-effect cards.

Alternative considered: leave vanilla cards as JSON-only metadata. That keeps the implementation smaller, but the candidate/deck filters rely on registered Rust effects, so JSON-only cards would make the deck unusable as a complete implemented pool.

### Model `ST1-09` as a block-event observer, not as attack-target-change shorthand

`ST1-09` should resolve from the carrier's inherited source when that carrier is the attacking Digimon and the defender declares a blocker. The engine already has `EffectTiming::OnBlock` and `TriggerSource::PlayerBattleArea`, but the DSL must expose enough predicate context to distinguish:

- the blocked attacker from other allied Digimon,
- a real blocker declaration from other target changes,
- inherited source effects under the attacking carrier.

Preferred shape: a triggered inherited clause using an `on_block` timing plus a predicate equivalent to "event attacker is source permanent", then `gain_memory: 3`. If the current timing/predicate surface cannot express that exactly, add the smallest reusable DSL predicate or trigger payload plumbing needed.

Alternative considered: use `on_attack_target_change` with reason `blocker` and `event_target_was_self`. That naturally observes the original target, not the attacker, so it is the wrong semantic for "this Digimon is blocked."

### Add defender-side Security Digimon DP modification as a separate primitive

Existing `applies_to_opponent_security_dp: true` covers attacker-carried effects like "opponent's Security Digimon get -3000 DP." `ST1-14` is the mirror: the defender's own Security Digimon gain DP during future security battles, including effects sourced by an Option card. This should be represented as a player-scoped or defender-scoped security-DP modifier, not by modifying card metadata or attacker's stack.

The primitive should support:

- positive and negative DP deltas,
- duration scopes `end_of_turn` and `end_of_opponents_next_turn`,
- security battle consult when a security card with DP is revealed,
- no action/tensor shape changes.

Alternative considered: treat `ST1-14` as a no-op because Security Digimon DP rarely matters in starter-deck mirrors. That violates printed behavior and would hide a reusable yellow/red-era card shape already visible in local metadata.

### Implement option bodies explicitly instead of generic "activate main" indirection

For `ST1-15` and `ST1-16`, author the `[Main]` body under `main_from_hand` and repeat the same steps under `on_security`, matching existing cards such as `BT8-097`. This avoids depending on a broad `activate_own_main_effects` helper and keeps the behavioral tests inspectable.

Alternative considered: add a generic "activate this card's [Main] effects" DSL macro. That is useful longer-term, but ST-1 can be completed with the current explicit mirror pattern.

### Represent "up to 2" as two optional sequential picks with duplicate prevention

`ST1-15` Giga Destroyer should let the player delete zero, one, or two eligible opponent Digimon with 4000 DP or less. Use existing optional target selection and binding exclusion if available. If duplicate prevention is not ergonomic for sequential permanent selections, add the smallest reusable `not_in_binding` or multi-select support needed.

Alternative considered: delete all eligible Digimon up to two automatically. That hides choices and breaks the action-mask contract.

## Risks / Trade-offs

- `ST1-09` may require a small trigger-context extension → mitigate with a focused DSL/combat test that proves only the blocked attacker carrier gains memory.
- `ST1-14` may touch security battle DP resolution → mitigate with combat/security tests covering positive defender buffs, expiration, and absence after expiry.
- Repeating option main bodies under security can drift → mitigate by keeping YAML comments explicit and adding tests for both hand-play and security paths.
- Vanilla YAML can look like fake "implemented" coverage → mitigate by asserting those cards have no printed effects and by keeping effect-bearing cards covered by behavior tests.
- Starter deck fixtures can become inconsistent with card counts → mitigate with a count test that asserts the Fandom/worldwide list exactly: 4 eggs and 50 main-deck cards.
