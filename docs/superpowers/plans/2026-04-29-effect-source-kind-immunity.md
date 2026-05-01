# Effect Source Kind Immunity Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use `superpowers:subagent-driven-development` (recommended) or `superpowers:executing-plans` to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add an explicit `EffectSourceKind` contract to the Rust engine so effects can be classified as Digimon, Tamer, Option, or Rule effects at enqueue time and then used reliably by immunity/targeting logic. This must cover inherited effects, DUAL cards, security effects, and opponent-turn/full-turn immunity windows.

**Architecture:** Source kind is decided when an effect is queued, preserved through pending selections, exposed through `EffectContext` and `EffectReadContext`, and consumed by centralized "can this effect affect this permanent?" checks before target-affecting mutations run. Security is an activation zone/timing, not a source kind.

**Tech Stack:** Rust engine under `code/digimon-engine/`, existing integration tests under `code/digimon-engine/tests/`, `cargo test` verification.

---

## Current Context

The design spec lives at:

- `docs/superpowers/specs/2026-04-29-effect-source-kind-immunity-design.md`

Relevant current engine surfaces:

- `code/digimon-engine/src/selection.rs` defines `QueuedEffect`.
- `code/digimon-engine/src/effect_queue.rs` constructs queued effects from permanents, security cards, option uses, and triggered timings.
- `code/digimon-engine/src/effect_context/mod.rs` defines `EffectContext` and `EffectReadContext`.
- `code/digimon-engine/src/effect_context/selections.rs` stores source metadata across pending selections.
- `code/digimon-engine/src/game_actions.rs` queues pending option/counter/Arts follow-up effects.
- `code/digimon-engine/src/enums.rs` defines `ModifierType::CannotBeAffected` but effect-execution enforcement is incomplete.
- Existing `source_is_tamer()` helpers infer from the source card/permanent. These must remain as compatibility wrappers over explicit source kind.

Classification rules to implement:

| Scenario | Source kind |
| --- | --- |
| Top Digimon stack effect | `Digimon` |
| Inherited effect under a Digimon stack | `Digimon` |
| DUAL card used as an Option from hand/trash | `Option` |
| DUAL card after Arts Digivolve and now on a Digimon stack | `Digimon` |
| Tamer field effect | `Tamer` |
| Standard Option from hand/trash | `Option` |
| Digimon card revealed in security with a security effect, e.g. AD01 LordKnightmon | `Digimon` |
| Option card revealed in security | `Option` |
| Tamer card revealed in security | `Tamer` |
| Engine bookkeeping/rules effects | `Rule` |

Important nuance:

- Inherited effects from digivolution cards belong to the top Digimon and are therefore Digimon effects.
- Security is not a source kind. A Digimon security effect remains a Digimon effect.
- Existing `game_actions.rs` has `OptionUseSource` for "hand vs trash" option-origin bookkeeping. Do not reuse it as effect source kind.

---

## Phase 1: Add Source-Kind Type And Backward-Compatible Plumbing

### Task 1.1: Add the enum and default classifier helpers

- [ ] Add `EffectSourceKind` to `code/digimon-engine/src/enums.rs`.

Expected shape:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EffectSourceKind {
    Digimon,
    Tamer,
    Option,
    Rule,
}
```

- [ ] If `Serialize`/`Deserialize` are not already in scope in `enums.rs`, follow the existing serde import style in that file.
- [ ] Add a helper in the most local existing engine helper module used by queue construction. Prefer an inherent/private helper in `effect_queue.rs` unless there is already a shared card-kind helper.

Expected helper behavior:

```rust
fn source_kind_for_card_kind(card_kind: CardKind) -> EffectSourceKind {
    match card_kind {
        CardKind::Digimon | CardKind::DigiEgg => EffectSourceKind::Digimon,
        CardKind::Tamer => EffectSourceKind::Tamer,
        CardKind::Option => EffectSourceKind::Option,
        // DUAL defaults must be decided by enqueue context, not only by card kind.
        CardKind::Dual => EffectSourceKind::Digimon,
        _ => EffectSourceKind::Rule,
    }
}
```

- [ ] If the actual `CardKind` variants differ, map the real variants without adding fake variants.
- [ ] Keep DUAL context-specific overrides out of this generic helper:
  - pending option use: `Option`
  - permanent/stack source: `Digimon`
  - security source: classify from the effect face/data used by the security effect; if the current data model lacks face metadata, use card kind for non-DUAL cards and add a targeted DUAL security TODO only if a real card requires it.

### Task 1.2: Add `source_kind` to queued effects

- [ ] Update `QueuedEffect` in `code/digimon-engine/src/selection.rs`.

Expected field:

```rust
pub source_kind: EffectSourceKind,
```

- [ ] Import `EffectSourceKind` beside the other enum imports.
- [ ] Update all `QueuedEffect` constructors to set `source_kind`.
- [ ] Do not use `EffectSourceKind::Rule` as a catch-all to silence compiler errors. Every queue site must classify intentionally.

Known queue sites to inspect:

- `code/digimon-engine/src/effect_queue.rs`
- `code/digimon-engine/src/game_actions.rs`
- any direct `QueuedEffect { ... }` construction found by:

```bash
rg "QueuedEffect\s*\{" code/digimon-engine/src code/digimon-engine/tests
```

### Task 1.3: Preserve source kind through pending selections

- [ ] Search selection state that stores `source_card` and `source_permanent`.

Command:

```bash
rg "source_card|source_permanent|PendingSelection" code/digimon-engine/src/effect_context code/digimon-engine/src/selection.rs
```

- [ ] Add `source_kind: EffectSourceKind` to pending selection data that rebuilds an `EffectContext` after a player choice.
- [ ] Update constructors in `code/digimon-engine/src/effect_context/selections.rs` to copy `ctx.source_kind`.
- [ ] Update selection resolution code so the callback context uses the preserved source kind, not a re-inferred card kind.

Acceptance condition:

- An Option effect that asks the player to choose a target must still be `Option` source kind inside the callback.
- A DUAL card used as an Option must not become `Digimon` just because the underlying card has a Digimon face.

---

## Phase 2: Expose Source Kind In Effect Contexts

### Task 2.1: Extend `EffectContext`

- [ ] Add `source_kind: EffectSourceKind` to `EffectContext` in `code/digimon-engine/src/effect_context/mod.rs`.
- [ ] Add a getter:

```rust
pub fn source_kind(&self) -> EffectSourceKind {
    self.source_kind
}
```

- [ ] Add convenience helpers:

```rust
pub fn source_is_digimon(&self) -> bool {
    self.source_kind == EffectSourceKind::Digimon
}

pub fn source_is_tamer(&self) -> bool {
    self.source_kind == EffectSourceKind::Tamer
}

pub fn source_is_option(&self) -> bool {
    self.source_kind == EffectSourceKind::Option
}
```

- [ ] Keep the existing `source_is_tamer()` public API, but replace inference with the explicit field.
- [ ] Add a constructor path that accepts explicit source kind. Preserve existing constructor names if that avoids broad churn, but the final context used for queued effects must come from `QueuedEffect.source_kind`.

Recommended pattern:

```rust
pub fn new_with_source_kind(
    game: &'a mut Game,
    player: usize,
    source_card: Option<CardSource>,
    source_permanent: Option<PermanentHandle>,
    source_kind: EffectSourceKind,
) -> Self
```

Then make the old `new(...)` call `new_with_source_kind(...)` with a narrow inference helper only for legacy/test callers.

### Task 2.2: Extend `EffectReadContext`

- [ ] Add `source_kind: EffectSourceKind` to `EffectReadContext`.
- [ ] Mirror `source_kind()`, `source_is_digimon()`, `source_is_tamer()`, and `source_is_option()`.
- [ ] Ensure any code that creates read contexts from write contexts copies the field.
- [ ] Update replacement/filter code that uses read contexts so it receives the explicit field.

Compatibility requirement:

- Existing tamer discrimination tests should still pass after being updated to assert explicit source kind.

### Task 2.3: Update effect execution to pass queued source kind

- [ ] Find where `QueuedEffect` becomes `EffectContext`.

Command:

```bash
rg "EffectContext::new|EffectContext::new_with_override|QueuedEffect" code/digimon-engine/src
```

- [ ] When resolving a queued effect, pass `queued.source_kind` into the context.
- [ ] If `new_with_override` is used for callbacks/selection resolution, add a `source_kind` argument or replace it with a clearer constructor.
- [ ] Keep caller updates mechanical and minimal.

---

## Phase 3: Classify Every Queue Path

### Task 3.1: Permanent field effects

- [ ] In `code/digimon-engine/src/effect_queue.rs`, classify effects enqueued from field permanents.
- [ ] Top-card Digimon stack effects must be `Digimon`.
- [ ] Tamer permanents must be `Tamer`.
- [ ] Option permanents that remain on board as Training/Delay/linked cards must be `Option` unless the effect is explicitly inherited from a digivolution card under the top Digimon.

Test first:

- [ ] Add `code/digimon-engine/tests/effect_source_kind/main.rs`.
- [ ] Add `code/digimon-engine/tests/effect_source_kind/classification.rs`.
- [ ] Register the module according to the existing integration test pattern under `code/digimon-engine/tests/`.
- [ ] Add focused tests:
  - top Digimon effect queues as `Digimon`
  - Tamer effect queues as `Tamer`
  - Training/linked Option field effect queues as `Option` if an existing test helper can create one without unrelated setup

Verification command:

```bash
cd code/digimon-engine
cargo test --test effect_source_kind -- classification --nocapture
```

### Task 3.2: Inherited effects under a Digimon stack

- [ ] Locate existing inherited effect enqueue/lowering paths.

Command:

```bash
rg "inherited|source_under|digivolution|stack" code/digimon-engine/src/effect_queue.rs code/digimon-engine/src/effect_context code/digimon-engine/src
```

- [ ] When an effect comes from a normal digivolution source under a Digimon, classify it as `Digimon`.
- [ ] Do not classify Training Option inherited-like effects as `Digimon` unless they are actually in the digivolution stack and rules/card text say the top Digimon owns that effect.

Test first:

- [ ] Add a regression test where an inherited effect is queued/resolved and `ctx.source_kind()` is `Digimon`.
- [ ] If the engine cannot currently queue inherited effects generically, use the smallest existing implemented card that already exercises inherited behavior. Do not add fake gameplay behavior just for this test.

### Task 3.3: Option and Counter effects from hand/trash

- [ ] In `code/digimon-engine/src/game_actions.rs`, update pending option/counter queue construction:
  - `enqueue_option_main_from_pending`
  - `enqueue_counter_effect_from_pending`
  - any similar pending option resolver found by `rg "pending_option|OptionMain|Counter" code/digimon-engine/src`
- [ ] Set `source_kind: EffectSourceKind::Option` for normal Options and DUAL cards used as Options.
- [ ] Do not infer DUAL option uses from card kind after the fact.

Test first:

- [ ] Add a test where a normal Option with a pending target selection remains `Option` in the callback.
- [ ] Add or update a DUAL test where using the card as an Option makes its effect `Option`.

Recommended existing test area:

- `code/digimon-engine/tests/dual_cards/`
- or the new `code/digimon-engine/tests/effect_source_kind/` suite if the test is source-kind-focused.

### Task 3.4: Arts Digivolve follow-up effects

- [ ] Inspect Arts Digivolve flow in `code/digimon-engine/src/game_actions.rs` and DUAL tests.

Command:

```bash
rg "Arts|arts|DUAL|dual" code/digimon-engine/src code/digimon-engine/tests/dual_cards
```

- [ ] Any effect queued after the DUAL card has become part of a Digimon stack must be `Digimon`.
- [ ] If Arts Digivolve queues a standard `WhenDigivolving` timing from the resulting permanent, rely on the permanent classifier from Task 3.1.
- [ ] Add a regression test: DUAL used as Option is `Option`; the same card after Arts Digivolve resolves stack effects as `Digimon`.

### Task 3.5: Security effects

- [ ] In `code/digimon-engine/src/effect_queue.rs`, update `enqueue_from_security_card` or equivalent security queue construction.
- [ ] Classify by the revealed card/effect face:
  - Digimon security effect: `Digimon`
  - Option security effect: `Option`
  - Tamer security effect: `Tamer`
- [ ] Specifically cover Digimon cards with security effects, such as AD01 LordKnightmon.

Test first:

- [ ] Add a test using an implemented Digimon security effect if AD01 LordKnightmon is available in `data/cards.json` and the Rust card loader supports it.
- [ ] If AD01 LordKnightmon is present in data but not implemented in Rust, create a minimal test card through the existing Rust test-card factory with `CardKind::Digimon` and a security effect. Name the test after the rule, not the fake card.

Verification command:

```bash
cd code/digimon-engine
cargo test --test effect_source_kind -- security --nocapture
```

---

## Phase 4: Implement Source-Kind-Aware Immunity Filters

### Task 4.1: Add immunity filter data to modifiers

- [ ] Inspect `ModifierEntry` and the code that installs `ModifierType::CannotBeAffected`.

Command:

```bash
rg "struct ModifierEntry|CannotBeAffected|ModifierType::CannotBeAffected" code/digimon-engine/src code/digimon-engine/tests
```

- [ ] Add a filter model for source-kind-aware immunity.

Expected shape, adjusted to fit existing modifier style:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EffectControllerFilter {
    Any,
    OpponentOnly,
    OwnOnly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EffectImmunityFilter {
    pub source_kind: Option<EffectSourceKind>,
    pub controller: EffectControllerFilter,
}
```

- [ ] Add an optional `effect_immunity_filter: Option<EffectImmunityFilter>` to permanent modifier entries, or an equivalent existing filter location if the engine already has modifier predicates.
- [ ] Preserve existing modifier behavior:
  - no filter means current broad behavior
  - `source_kind: Some(Digimon)` means "Digimon effects"
  - `controller: OpponentOnly` means "your opponent's effects"
- [ ] Add builder/helper methods rather than open-coding struct fields at every card implementation.

Recommended helpers:

```rust
ModifierEntry::cannot_be_affected_by_opponents_source_kind(EffectSourceKind::Digimon, expiry)
ModifierEntry::cannot_be_affected_by_any_source_kind(EffectSourceKind::Digimon, expiry)
```

### Task 4.2: Add central immunity query

- [ ] Add a central helper on `Game` or the most appropriate existing modifier query module.

Expected behavior:

```rust
pub fn permanent_is_unaffected_by_effect(
    &self,
    target: PermanentHandle,
    effect_controller: usize,
    source_kind: EffectSourceKind,
) -> bool
```

- [ ] It must return `true` only when:
  - the target has an active `CannotBeAffected` modifier
  - the modifier has not expired
  - controller filter matches `effect_controller` relative to the target controller
  - source-kind filter is empty or matches `source_kind`
- [ ] It must not block rule effects unless a modifier explicitly says it blocks `Rule`.
- [ ] It must not block effects that do not affect the protected permanent.

Test first:

- [ ] Add unit/integration tests for the query:
  - opponent Digimon effect blocked
  - own Digimon effect not blocked when filter is opponent-only
  - opponent Option effect not blocked by Digimon-only immunity
  - opponent Tamer effect not blocked by Digimon-only immunity
  - immunity expires at the existing expiry boundary

### Task 4.3: Gate target-affecting mutations

- [ ] Find target-affecting mutation helpers in `code/digimon-engine/src/effect_context/`.

Command:

```bash
rg "delete|trash|return_to|bottom|de_digivolve|suspend|unsuspend|dp|change|bounce|place.*security|move.*permanent" code/digimon-engine/src/effect_context code/digimon-engine/src
```

- [ ] Add an `EffectContext` helper:

```rust
fn can_affect_permanent(&self, target: PermanentHandle) -> bool
```

Expected helper behavior:

```rust
!self.game.permanent_is_unaffected_by_effect(target, self.player, self.source_kind)
```

- [ ] Call this helper before every effect-context operation that applies an effect to an opponent/own permanent.
- [ ] The blocked operation should be a no-op, not a crash.
- [ ] If an operation affects multiple permanents, filter protected targets individually.
- [ ] Do not use this gate for normal game-rule movement, battle deletion, digivolution housekeeping, or other rule actions that are not resolving card effects.

Minimum operations to cover in this pass:

- delete/trash a permanent by effect
- return a permanent to hand/deck/security by effect
- de-digivolve by effect
- suspend/unsuspend by effect
- DP changes by effect
- effect-driven source stripping or bottom-decking if exposed through context helpers

If the engine has a single lower-level movement helper with cause data, prefer gating at the effect-context entry point so battle/rule movement is not accidentally blocked.

### Task 4.4: Update card effect authoring helpers

- [ ] Add authoring/lowering helpers so card implementations can install "immune to Digimon effects" without custom closures.
- [ ] Replace any existing card-specific implementation that approximates Digimon immunity with the new filter.
- [ ] Update `source_is_tamer` callers to use `source_kind` helpers where clearer.

Search command:

```bash
rg "source_is_tamer|CannotBeAffected|immune|unaffected|affected by" code/digimon-engine/src code/digimon-engine/tests
```

---

## Phase 5: DUAL, Security, And Inherited Regression Coverage

### Task 5.1: DUAL source-kind regression tests

- [ ] Add or update tests so DUAL behavior is pinned in both modes.

Required assertions:

- [ ] DUAL used as Option from hand/trash queues/resolves with `EffectSourceKind::Option`.
- [ ] DUAL after Arts Digivolve queues/resolves stack effects with `EffectSourceKind::Digimon`.
- [ ] A Digimon-only immunity blocks the post-Arts Digimon effect.
- [ ] The same Digimon-only immunity does not block the DUAL Option effect before Arts Digivolve.

Suggested command:

```bash
cd code/digimon-engine
cargo test --test dual_cards -- --nocapture
```

### Task 5.2: Security Digimon source-kind regression tests

- [ ] Add a test for Digimon security effects being Digimon effects.
- [ ] Add a protection interaction test:
  - protected Digimon has "unaffected by opponent's Digimon effects"
  - opponent checks security and reveals a Digimon security effect that targets/affects the protected Digimon
  - effect is blocked because source kind is `Digimon`
- [ ] Add the corresponding Option security control if a small existing Option security effect can target/affect a Digimon:
  - same protection should not block an Option security effect

### Task 5.3: Inherited source-kind regression tests

- [ ] Add or update a test where an inherited effect attempts to affect a Digimon protected from opponent's Digimon effects.
- [ ] Assert it is blocked because inherited effects belong to the top Digimon.
- [ ] Add a control where Option-only immunity, if available, does not block the inherited Digimon effect.

---

## Phase 6: Documentation And Data/API Check

### Task 6.1: Update rules docs

- [ ] Update `docs/RULES_CONTEXT.md` with a short section:
  - effect source kind is Digimon/Tamer/Option/Rule
  - security is not a source kind
  - inherited effects are Digimon effects
  - Digimon security effects are Digimon effects
  - DUAL option-use vs Arts stack behavior

### Task 6.2: Update Rust API docs if needed

- [ ] If `docs/RUST_ENGINE_API.md` documents effect contexts, update it with:
  - `ctx.source_kind()`
  - `ctx.source_is_digimon()`
  - `ctx.source_is_tamer()`
  - `ctx.source_is_option()`
  - immunity helper/lowering usage

### Task 6.3: Preserve cards.json/API conclusion

- [ ] Confirm the plan does not require `data/cards.json` to expose a special DUAL marker beyond what already exists for card kind/faces.
- [ ] If implementation discovers that imported DUAL face data is insufficient for security-face classification, document the exact missing field in the spec rather than guessing from names or effect text.

---

## Phase 7: Verification

Run targeted tests after each phase, then broad checks at the end.

### Targeted commands

```bash
cd code/digimon-engine
cargo test --test effect_source_kind -- --nocapture
cargo test --test dual_cards -- --nocapture
cargo test --test flood_gates -- --nocapture
```

### Broad Rust engine command

```bash
cd code/digimon-engine
cargo test
```

### Python parity smoke, only if Rust bindings are already built or the change touches PyO3-visible contracts

```bash
DIGIMON_BACKEND=rust python -m pytest code/engine_py_legacy/tests/engine/test_rust_backend_parity.py -v
```

Do not run a long training job for this change.

---

## Self-Review Checklist

- [ ] No source-kind decisions are inferred from card kind after an effect is already queued.
- [ ] DUAL cards are `Option` when used as Options and `Digimon` after Arts Digivolve.
- [ ] Inherited effects under a Digimon are `Digimon`.
- [ ] Digimon security effects are `Digimon`, including the AD01 LordKnightmon rule case.
- [ ] `source_is_tamer()` still exists but reads explicit source kind.
- [ ] Pending selections preserve source kind through callbacks.
- [ ] Immunity checks only block effects that affect the protected Digimon.
- [ ] Opponent-only filters compare the effect controller against the target controller.
- [ ] Expiry semantics for full-turn/player-turn immunity remain controlled by existing modifier expiry.
- [ ] Rule effects are not accidentally blocked by Digimon/Tamer/Option immunity.
- [ ] Tests fail before implementation and pass after implementation.

---

## Suggested Implementation Order

1. Add enum and `QueuedEffect.source_kind`.
2. Update constructors until the code compiles.
3. Add context/read-context accessors and preserve through selection callbacks.
4. Write classification tests and implement queue classification.
5. Add immunity filter data and query tests.
6. Gate target-affecting mutations.
7. Add DUAL/security/inherited regression tests.
8. Update docs.
9. Run targeted and broad verification.

