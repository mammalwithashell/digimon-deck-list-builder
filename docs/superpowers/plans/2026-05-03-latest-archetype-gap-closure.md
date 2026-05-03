# Latest Archetype Gap Closure Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Close only the remaining Rust engine and DSL capability gaps identified in `docs/superpowers/specs/2026-05-03-latest-archetype-dsl-engine-gap-closure-design.md`, without rebuilding primitives the audit found already implemented.

**Architecture:** Implement one capability slice at a time with failing Rust tests first, then DSL schema/lowering, then engine behavior, then card-shaped regressions and tracker updates. Reuse existing `PendingSelection`, action-mask, reveal-pool, replacement, effect-digivolve, and formula infrastructure instead of adding action-space or tensor-contract changes.

**Tech Stack:** Rust, Cargo integration tests, `digimon-dsl`, `digimon-engine`, YAML card specs, existing PyO3/RL contracts.

---

## Scope Check

This roadmap touches multiple engine subsystems. Treat this as a master plan with independent implementation tasks. Execute tasks sequentially unless two tasks have disjoint write sets; most tasks touch shared DSL files such as `code/digimon-dsl/src/step.rs`, `code/digimon-dsl/src/compile.rs`, and `code/digimon-engine/src/dsl_cards/step/selections.rs`, so parallel code edits should be avoided for those files.

The first four tasks are the highest leverage work. Later tasks should be re-audited before implementation because earlier tasks may demote some remaining card notes to card-local authoring.

## File Structure

- Modify `code/digimon-dsl/src/step.rs`: add new DSL step structs and serde names such as `select_reveal_buckets` and `may_attack_now`.
- Modify `code/digimon-dsl/src/compiled.rs`: add compiled step and predicate/formula fields that the engine can lower without reading YAML-specific structs.
- Modify `code/digimon-dsl/src/compile.rs`: lower new DSL shapes into compiled steps and validate high-level replacement/alt-path bodies.
- Modify `code/digimon-dsl/src/validator.rs`: reject malformed YAML at authoring time, especially invalid reveal buckets and missing effect-attack bind names.
- Modify `code/digimon-dsl/src/formula.rs` and `code/digimon-dsl/src/predicate.rs`: add only residual formula/predicate vocabulary proven by blocked card text.
- Modify `code/digimon-engine/src/effect_context/selections.rs`: add reusable selection helpers and prompt-order candidate plumbing.
- Modify `code/digimon-engine/src/dsl_cards/step/selections.rs`: lower compiled selection steps into `EffectContext` calls.
- Modify `code/digimon-engine/src/dsl_cards/step/zone_moves.rs`: consume selected reveal buckets and selected source/material bindings.
- Modify `code/digimon-engine/src/effect_context/mod.rs`: add reusable effect operations such as immediate attack, source-stack movement, and option disposition.
- Modify `code/digimon-engine/src/action/mask.rs` and `code/digimon-engine/src/action/decode.rs`: expose and execute legal attack choices only when a slice needs action-mask behavior.
- Modify `code/digimon-engine/src/dsl_cards/lower_replacement.rs`: complete generic cross-permanent replacement lowering.
- Modify `code/digimon-engine/src/dna_digivolve.rs` and `code/digimon-engine/src/dsl_cards/lower_alt_path_registration.rs`: consume `source_treated_as` for Tamer-as-base/hybrid routes.
- Modify `code/digimon-engine/src/effect_queue.rs` and `code/digimon-engine/src/events.rs`: preserve event payload/result binding context where card-shaped tests require it.
- Create focused tests under `code/digimon-engine/tests/dsl/`, `code/digimon-engine/tests/selection/`, `code/digimon-engine/tests/replacements/`, `code/digimon-engine/tests/effect_context/`, and `code/digimon-engine/tests/cards_behavioral/`.
- Update `docs/RUST_ENGINE_GAPS.md`, `qa/dsl-vocab-gaps.md`, `qa/archetype-qa/engine-gaps.md`, and the relevant `qa/archetype-qa/dsl/*.md` source notes when a task closes or narrows a gap.

## Task 1: True Multi-Bucket Reveal Selection

**Files:**
- Modify: `code/digimon-dsl/src/step.rs`
- Modify: `code/digimon-dsl/src/compiled.rs`
- Modify: `code/digimon-dsl/src/compile.rs`
- Modify: `code/digimon-dsl/src/validator.rs`
- Modify: `code/digimon-engine/src/effect_context/selections.rs`
- Modify: `code/digimon-engine/src/dsl_cards/step/selections.rs`
- Modify: `code/digimon-engine/src/dsl_cards/step/zone_moves.rs`
- Test: `code/digimon-engine/tests/dsl/reveal_buckets.rs`
- Test: `code/digimon-engine/tests/selection/reveal_buckets.rs`
- Docs: `qa/dsl-vocab-gaps.md`

- [ ] **Step 1: Add the failing DSL compile test**

Add `code/digimon-engine/tests/dsl/reveal_buckets.rs` and register it in `code/digimon-engine/tests/dsl/main.rs` with `mod reveal_buckets;`.

```rust
use digimon_dsl::compiled::CompiledStep;
use digimon_dsl::compile;
use digimon_dsl::spec::CardSpec;

fn compile_steps(yaml: &str) -> Vec<CompiledStep> {
    let spec: CardSpec = serde_yml::from_str(yaml).expect("card yaml parses");
    let compiled = compile(&spec).expect("card yaml compiles");
    compiled.effects[0].clauses[0].process.clone()
}

#[test]
fn select_reveal_buckets_compiles_named_buckets() {
    let steps = compile_steps(
        r#"
card_id: TEST-REVEAL-BUCKETS
name: Reveal Buckets Test
card_type: option
colors: [red]
play_cost: 2
effects:
  - timing: main
    process:
      - reveal_top_deck: { of: you, count: 3, bind_as: revealed }
      - select_reveal_buckets:
          from: revealed
          buckets:
            - bind_as: hybrid
              filter: { trait_has: Hybrid }
              min: 0
              max: 1
            - bind_as: tamer
              filter: { kind: tamer }
              min: 0
              max: 1
          no_duplicate_cards: true
          prompt: "Choose cards to add"
      - add_to_hand_from_reveal: { card: hybrid }
      - add_to_hand_from_reveal: { card: tamer }
"#,
    );

    assert!(
        matches!(steps[1], CompiledStep::SelectRevealBuckets { .. }),
        "second step should lower to CompiledStep::SelectRevealBuckets: {steps:#?}"
    );
}
```

- [ ] **Step 2: Run the compile test and verify it fails**

Run:

```powershell
cargo test --manifest-path code\digimon-engine\Cargo.toml --test dsl -- reveal_buckets --nocapture
```

Expected: FAIL because `select_reveal_buckets` is not a known DSL step and `CompiledStep::SelectRevealBuckets` does not exist.

- [ ] **Step 3: Add DSL and compiled data types**

Add these shapes in `code/digimon-dsl/src/step.rs` and `code/digimon-dsl/src/compiled.rs`. Keep field names aligned with the YAML shape.

```rust
// code/digimon-dsl/src/step.rs
SelectRevealBuckets(SelectRevealBucketsArgs),

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SelectRevealBucketsArgs {
    pub from: String,
    pub buckets: Vec<SelectRevealBucketArgs>,
    #[serde(default)]
    pub no_duplicate_cards: bool,
    #[serde(default)]
    pub prompt: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SelectRevealBucketArgs {
    pub bind_as: String,
    #[serde(default)]
    pub filter: Option<PredicateSpec>,
    #[serde(default)]
    pub min: Option<u8>,
    #[serde(default)]
    pub max: Option<u8>,
}
```

```rust
// code/digimon-dsl/src/compiled.rs
SelectRevealBuckets {
    from: String,
    buckets: Vec<CompiledRevealBucket>,
    no_duplicate_cards: bool,
    prompt: Option<String>,
},

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompiledRevealBucket {
    pub bind_as: String,
    pub filter: Option<CompiledPredicate>,
    pub min: u8,
    pub max: u8,
}
```

Wire serde dispatch in `StepSpec` using the name `select_reveal_buckets`, and lower in `compile.rs` by compiling each bucket predicate through the existing predicate compiler.

- [ ] **Step 4: Add validation for bucket shape**

In `code/digimon-dsl/src/validator.rs`, reject empty buckets, duplicate `bind_as`, and `min > max`.

```rust
StepSpec::SelectRevealBuckets(args) => {
    if args.buckets.is_empty() {
        errors.push("select_reveal_buckets requires at least one bucket".into());
    }
    let mut seen = std::collections::BTreeSet::new();
    for bucket in &args.buckets {
        if !seen.insert(bucket.bind_as.clone()) {
            errors.push(format!(
                "select_reveal_buckets duplicate bucket bind_as: {}",
                bucket.bind_as
            ));
        }
        let min = bucket.min.unwrap_or(0);
        let max = bucket.max.unwrap_or(1);
        if min > max {
            errors.push(format!(
                "select_reveal_buckets bucket {} has min greater than max",
                bucket.bind_as
            ));
        }
    }
}
```

- [ ] **Step 5: Run the compile test and verify it passes**

Run:

```powershell
cargo test --manifest-path code\digimon-engine\Cargo.toml --test dsl -- reveal_buckets --nocapture
```

Expected: PASS for the compile test, with no behavior test added yet.

- [ ] **Step 6: Add the failing engine selection behavior test**

Add this test to `code/digimon-engine/tests/selection/reveal_buckets.rs` and register it in `code/digimon-engine/tests/selection/main.rs` with `mod reveal_buckets;`.

```rust
use digimon_engine::action::space::PASS;
use digimon_engine::debug_runner::DebugGameRunner;
use digimon_engine::effect_context::EffectContext;
use digimon_engine::selection::SelectionKind;
use std::sync::{Arc, Mutex};

#[test]
fn reveal_buckets_prevent_cross_bucket_duplicate_pick() {
    let mut r = DebugGameRunner::new();
    let p0 = 0;
    let first = r.add_card_to_reveal_for_test(p0, "BT17-009");
    let second = r.add_card_to_reveal_for_test(p0, "BT17-010");

    let picked = Arc::new(Mutex::new(Vec::new()));
    let picked_slot = Arc::clone(&picked);

    {
        let mut ctx = EffectContext::new(&mut r.game, p0, None);
        ctx.select_reveal_buckets_for_test(
            vec![
                ("hybrid".to_string(), 0, 1, vec![first, second]),
                ("tamer".to_string(), 0, 1, vec![first]),
            ],
            "Choose cards",
            true,
            move |_ctx, buckets| {
                *picked_slot.lock().unwrap() = buckets
                    .into_iter()
                    .flat_map(|(_, cards)| cards)
                    .collect::<Vec<_>>();
            },
        );
    }

    let pending = r.game.pending_selection.as_ref().expect("first bucket prompt");
    assert!(matches!(pending.kind, SelectionKind::RevealBucket { .. }));
    let first_action = pending.valid_action_ids[0];
    r.game.resolve_selection(p0, first_action).expect("pick first card");

    let pending = r.game.pending_selection.as_ref().expect("second bucket prompt");
    assert!(
        !pending.valid_action_ids.contains(&first_action),
        "same reveal card must not be legal for the second bucket"
    );
    r.game.resolve_selection(p0, PASS).expect("decline optional second bucket");

    assert_eq!(*picked.lock().unwrap(), vec![first]);
}
```

Expected: this may need small helper adjustments to use the repo's exact reveal test helpers. Preserve the assertion shape: pick once, verify the same action/card is absent from the next bucket, then pass.

- [ ] **Step 7: Implement the engine helper and DSL lowerer**

In `code/digimon-engine/src/effect_context/selections.rs`, add a production helper equivalent to the test helper. It should install one `PendingSelection` per bucket, carry chosen reveal handles in a parked accumulator, and remove already-chosen handles from later bucket candidate lists when `no_duplicate_cards` is true.

```rust
pub fn select_reveal_buckets<C>(
    &mut self,
    buckets: Vec<RevealBucketSelection>,
    prompt: &str,
    no_duplicate_cards: bool,
    callback: C,
)
where
    C: FnOnce(&mut EffectContext<'_>, Vec<(String, Vec<CardHandle>)>) + 'static,
{
    self.install_next_reveal_bucket(
        buckets,
        prompt.to_string(),
        no_duplicate_cards,
        Vec::new(),
        Box::new(callback),
    );
}
```

In `code/digimon-engine/src/dsl_cards/step/selections.rs`, lower `CompiledStep::SelectRevealBuckets` by evaluating each compiled bucket predicate against the current reveal overlay and binding each bucket result under its `bind_as` name.

- [ ] **Step 8: Run focused and regression tests**

Run:

```powershell
cargo test --manifest-path code\digimon-engine\Cargo.toml --test selection -- reveal_buckets --nocapture
cargo test --manifest-path code\digimon-engine\Cargo.toml --test dsl -- reveal_buckets --nocapture
cargo test --manifest-path code\digimon-engine\Cargo.toml --test dsl -- phase2e_select_reveal phase2e_select_ordered_permutation phase2b_zone_moves_extra --nocapture
```

Expected: all PASS. Existing ordered-remainder tests must keep passing because this task must not rebuild `place_remainder_on_deck`.

- [ ] **Step 9: Update trackers and commit**

Update `qa/dsl-vocab-gaps.md` and each source archetype note that named a reveal bucket gap with the focused test command that passed. Then commit:

```powershell
git add code/digimon-dsl/src/step.rs code/digimon-dsl/src/compiled.rs code/digimon-dsl/src/compile.rs code/digimon-dsl/src/validator.rs code/digimon-engine/src/effect_context/selections.rs code/digimon-engine/src/dsl_cards/step/selections.rs code/digimon-engine/src/dsl_cards/step/zone_moves.rs code/digimon-engine/tests/dsl/reveal_buckets.rs code/digimon-engine/tests/selection/reveal_buckets.rs qa/dsl-vocab-gaps.md qa/archetype-qa/dsl
git commit -m "feat: add multi-bucket reveal selection"
```

## Task 2: Immediate Effect-Granted Attack Flow

**Files:**
- Modify: `code/digimon-dsl/src/step.rs`
- Modify: `code/digimon-dsl/src/compiled.rs`
- Modify: `code/digimon-dsl/src/compile.rs`
- Modify: `code/digimon-engine/src/effect_context/mod.rs`
- Modify: `code/digimon-engine/src/action/mask.rs`
- Modify: `code/digimon-engine/src/action/decode.rs`
- Modify: `code/digimon-engine/src/dsl_cards/step/combat.rs`
- Test: `code/digimon-engine/tests/combat/effect_granted_attack.rs`
- Test: `code/digimon-engine/tests/dsl/effect_granted_attack.rs`
- Docs: `docs/RUST_ENGINE_GAPS.md`
- Docs: `qa/dsl-vocab-gaps.md`

- [ ] **Step 1: Add a failing engine behavior test**

Create `code/digimon-engine/tests/combat/effect_granted_attack.rs` and register it in the combat test module used by this repo.

```rust
use digimon_engine::debug_runner::DebugGameRunner;
use digimon_engine::effect_context::{AttackTargetRestriction, EffectContext};
use digimon_engine::enums::GamePhase;

#[test]
fn may_attack_now_installs_attack_prompt_with_player_only_target() {
    let mut r = DebugGameRunner::new();
    let p0 = 0;
    let attacker = r.play_digimon_for_test(p0, "BT17-009", 5000);
    r.unsuspend_for_test(attacker);

    {
        let mut ctx = EffectContext::new(&mut r.game, p0, Some(attacker));
        ctx.may_attack_now(
            attacker,
            AttackTargetRestriction::PlayerOnly,
            false,
            "Attack with this Digimon?",
        )
        .expect("install attack prompt");
    }

    assert_eq!(r.game.current_phase, GamePhase::SelectAttackTarget);
    let pending = r.game.pending_selection.as_ref().expect("attack prompt");
    assert!(
        pending.valid_action_ids.iter().any(|id| r.game.explain_action(*id).contains("Attack Player")),
        "player target should be legal"
    );
    assert!(
        pending.valid_action_ids.iter().all(|id| !r.game.explain_action(*id).contains("Attack Digimon")),
        "digimon targets should be filtered out"
    );
}
```

- [ ] **Step 2: Run the focused test and verify it fails**

Run:

```powershell
cargo test --manifest-path code\digimon-engine\Cargo.toml --test combat -- effect_granted_attack --nocapture
```

Expected: FAIL because `may_attack_now`, `AttackTargetRestriction`, or `SelectAttackTarget` plumbing does not exist.

- [ ] **Step 3: Add the DSL shape**

Add `may_attack_now` to `code/digimon-dsl/src/step.rs` and compile it into `CompiledStep::MayAttackNow`.

```rust
MayAttackNow(MayAttackNowArgs),

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MayAttackNowArgs {
    pub attacker: String,
    #[serde(default)]
    pub targets: AttackTargetSpec,
    #[serde(default)]
    pub without_suspending: bool,
    #[serde(default)]
    pub optional: bool,
    #[serde(default)]
    pub prompt: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum AttackTargetSpec {
    #[default]
    Any,
    Player,
    Digimon,
}
```

- [ ] **Step 4: Add a failing DSL lowerer test**

Add to `code/digimon-engine/tests/dsl/effect_granted_attack.rs`:

```rust
use digimon_dsl::compiled::CompiledStep;
use digimon_dsl::{compile, spec::CardSpec};

#[test]
fn may_attack_now_yaml_lowers_to_compiled_step() {
    let yaml = r#"
card_id: TEST-MAY-ATTACK-NOW
name: May Attack Now Test
card_type: digimon
colors: [red]
level: 4
play_cost: 4
dp: 5000
effects:
  - timing: when_digivolving
    process:
      - may_attack_now:
          attacker: self
          targets: player
          without_suspending: true
          optional: true
"#;
    let spec: CardSpec = serde_yml::from_str(yaml).expect("yaml parses");
    let compiled = compile(&spec).expect("yaml compiles");
    let process = &compiled.effects[0].clauses[0].process;
    assert!(matches!(process[0], CompiledStep::MayAttackNow { .. }));
}
```

- [ ] **Step 5: Implement the engine operation**

In `code/digimon-engine/src/effect_context/mod.rs`, implement `may_attack_now` by installing a pending target selection that reuses normal attack legality and calls existing attack execution after resolution. Do not call `battle:` or direct effect battle helpers.

```rust
pub enum AttackTargetRestriction {
    Any,
    PlayerOnly,
    DigimonOnly,
}

pub fn may_attack_now(
    &mut self,
    attacker: PermanentHandle,
    targets: AttackTargetRestriction,
    without_suspending: bool,
    prompt: &str,
) -> Result<(), crate::combat::AttackError> {
    if !without_suspending && !self.game.can_attack(attacker, false) {
        return Ok(());
    }
    self.game.install_effect_attack_selection(
        self.controller,
        attacker,
        targets,
        without_suspending,
        prompt.to_string(),
    );
    Ok(())
}
```

Add `install_effect_attack_selection` near existing attack mask/decode helpers. It must use existing encoded attack IDs when possible and a pending-selection callback when an action ID needs phase disambiguation. If no legal targets exist and the effect is optional, continue the effect tail without a prompt.

- [ ] **Step 6: Lower DSL to the engine operation**

In `code/digimon-engine/src/dsl_cards/step/combat.rs`, map `CompiledStep::MayAttackNow` to `ctx.may_attack_now(...)`. Resolve `attacker: self` and named bindings through existing handle-binding utilities.

- [ ] **Step 7: Run focused attack tests**

Run:

```powershell
cargo test --manifest-path code\digimon-engine\Cargo.toml --test combat -- effect_granted_attack --nocapture
cargo test --manifest-path code\digimon-engine\Cargo.toml --test dsl -- effect_granted_attack --nocapture
cargo test --manifest-path code\digimon-engine\Cargo.toml --test mask_and_tensor -- mask_end_of_turn_parity --nocapture
```

Expected: all PASS. `mask_end_of_turn_parity` must remain unchanged because `may_attack_now` is immediate effect flow, not the end-of-turn `MayAttack` modifier.

- [ ] **Step 8: Update trackers and commit**

Update `docs/RUST_ENGINE_GAPS.md` and `qa/dsl-vocab-gaps.md` to say end-of-turn attack modifiers remain separate from immediate `may_attack_now`.

```powershell
git add code/digimon-dsl/src/step.rs code/digimon-dsl/src/compiled.rs code/digimon-dsl/src/compile.rs code/digimon-engine/src/effect_context/mod.rs code/digimon-engine/src/action/mask.rs code/digimon-engine/src/action/decode.rs code/digimon-engine/src/dsl_cards/step/combat.rs code/digimon-engine/tests/combat/effect_granted_attack.rs code/digimon-engine/tests/dsl/effect_granted_attack.rs docs/RUST_ENGINE_GAPS.md qa/dsl-vocab-gaps.md
git commit -m "feat: add immediate effect attack flow"
```

## Task 3: Generic Cross-Permanent Replacement Authoring

**Files:**
- Modify: `code/digimon-dsl/src/clause.rs`
- Modify: `code/digimon-dsl/src/predicate.rs`
- Modify: `code/digimon-dsl/src/compile.rs`
- Modify: `code/digimon-engine/src/dsl_cards/lower_replacement.rs`
- Modify: `code/digimon-engine/src/dsl_cards/predicate.rs`
- Modify: `code/digimon-engine/src/replacement.rs`
- Test: `code/digimon-engine/tests/replacements/cross_permanent.rs`
- Test: `code/digimon-engine/tests/dsl/replacement_context.rs`
- Docs: `docs/RUST_ENGINE_GAPS.md`
- Docs: `qa/archetype-qa/engine-gaps.md`

- [ ] **Step 1: Add the failing engine replacement test**

Add `code/digimon-engine/tests/replacements/cross_permanent.rs` and register it in the replacements test module.

```rust
use digimon_engine::action::space::REPLACEMENT_ACCEPT;
use digimon_engine::debug_runner::DebugGameRunner;
use digimon_engine::replacement::ReplacementCause;

#[test]
fn source_permanent_can_protect_a_different_subject() {
    let mut r = DebugGameRunner::new();
    let p0 = 0;
    let protector = r.play_digimon_for_test(p0, "BT24-101", 12000);
    let protected = r.play_digimon_for_test(p0, "BT24-040", 11000);
    r.install_test_cross_permanent_delete_prevention(protector, protected);

    r.game
        .delete_permanent_with_cause(protected, ReplacementCause::OpponentEffect);

    let pending = r.game.pending_selection.as_ref().expect("replacement prompt");
    assert_eq!(pending.valid_action_ids, vec![REPLACEMENT_ACCEPT]);
    r.game.resolve_selection(p0, REPLACEMENT_ACCEPT).expect("accept replacement");

    assert!(
        r.game.find_permanent(protected).is_some(),
        "protected subject should remain in battle area"
    );
    assert!(
        r.game.find_permanent(protector).is_some(),
        "replacement source should not be treated as the subject"
    );
}
```

- [ ] **Step 2: Run the replacement test and verify it fails**

Run:

```powershell
cargo test --manifest-path code\digimon-engine\Cargo.toml --test replacements -- cross_permanent --nocapture
```

Expected: FAIL because the test helper and/or generic cross-permanent lowering is absent.

- [ ] **Step 3: Add a DSL context test**

Add `code/digimon-engine/tests/dsl/replacement_context.rs`:

```rust
use digimon_dsl::{compile, spec::CardSpec};

#[test]
fn replacement_subject_and_source_predicates_compile_together() {
    let yaml = r#"
card_id: TEST-CROSS-REPLACEMENT
name: Cross Replacement Test
card_type: digimon
colors: [yellow]
level: 6
play_cost: 11
dp: 11000
effects:
  - kind: replacement
    timing: when_would_be_deleted
    active_when:
      replacement_subject_is_mine: true
      replacement_source_is_opponent: false
      replacement_cause: opponent_effect
    outcome: prevent
"#;
    let spec: CardSpec = serde_yml::from_str(yaml).expect("yaml parses");
    let compiled = compile(&spec).expect("replacement compiles");
    assert_eq!(compiled.effects[0].card_id, "TEST-CROSS-REPLACEMENT");
}
```

- [ ] **Step 4: Complete lowerer support**

In `code/digimon-engine/src/dsl_cards/lower_replacement.rs`, remove self-only assumptions for replacement subject matching. The lowerer should build a condition closure equivalent to:

```rust
move |ctx, subject| {
    predicate_matches_replacement_context(ctx, subject, &compiled_active_when)
        && source_permanent_is_still_active(ctx.source())
}
```

The condition must use the current replacement context:

```rust
ctx.replacement_cause()
ctx.replacement_source()
ctx.replacement_subject()
```

If those exact accessors do not exist, add them to `code/digimon-engine/src/replacement.rs` or the replacement context wrapper where existing tests already expose `replacement_cause()`.

- [ ] **Step 5: Run replacement regression tests**

Run:

```powershell
cargo test --manifest-path code\digimon-engine\Cargo.toml --test replacements -- cross_permanent context_predicates route_replacements nested_select_substrate --nocapture
cargo test --manifest-path code\digimon-engine\Cargo.toml --test dsl -- replacement_context --nocapture
```

Expected: all PASS. Existing nested replacement behavior must keep passing.

- [ ] **Step 6: Update trackers and commit**

```powershell
git add code/digimon-dsl/src/clause.rs code/digimon-dsl/src/predicate.rs code/digimon-dsl/src/compile.rs code/digimon-engine/src/dsl_cards/lower_replacement.rs code/digimon-engine/src/dsl_cards/predicate.rs code/digimon-engine/src/replacement.rs code/digimon-engine/tests/replacements/cross_permanent.rs code/digimon-engine/tests/dsl/replacement_context.rs docs/RUST_ENGINE_GAPS.md qa/archetype-qa/engine-gaps.md
git commit -m "feat: support cross-permanent replacement authoring"
```

## Task 4: Source-Stack Residual Operations

**Files:**
- Modify: `code/digimon-dsl/src/step.rs`
- Modify: `code/digimon-dsl/src/formula.rs`
- Modify: `code/digimon-engine/src/effect_context/mod.rs`
- Modify: `code/digimon-engine/src/dsl_cards/step/permanent_mutations.rs`
- Modify: `code/digimon-engine/src/dsl_cards/step/zone_moves.rs`
- Modify: `code/digimon-engine/src/dsl_cards/formula_eval.rs`
- Test: `code/digimon-engine/tests/effect_context/source_stack_operations.rs`
- Test: `code/digimon-engine/tests/dsl/source_stack_aggregates.rs`
- Docs: `docs/RUST_ENGINE_GAPS.md`

- [ ] **Step 1: Add failing source-stack operation tests**

Create `code/digimon-engine/tests/effect_context/source_stack_operations.rs`.

```rust
use digimon_engine::debug_runner::DebugGameRunner;
use digimon_engine::effect_context::EffectContext;

#[test]
fn trash_all_sources_preserves_top_card_and_trashes_each_source() {
    let mut r = DebugGameRunner::new();
    let p0 = 0;
    let stack = r.play_digimon_for_test(p0, "BT24-040", 11000);
    let source_a = r.attach_source_for_test(stack, "BT24-031");
    let source_b = r.attach_source_for_test(stack, "BT24-037");

    {
        let mut ctx = EffectContext::new(&mut r.game, p0, Some(stack));
        ctx.trash_all_sources(stack).expect("trash sources");
    }

    assert!(r.game.find_permanent(stack).is_some());
    assert!(r.game.card_in_trash(p0, source_a));
    assert!(r.game.card_in_trash(p0, source_b));
    assert_eq!(r.game.source_count(stack), 0);
}
```

- [ ] **Step 2: Run and verify failure**

Run:

```powershell
cargo test --manifest-path code\digimon-engine\Cargo.toml --test effect_context -- source_stack_operations --nocapture
```

Expected: FAIL because `trash_all_sources`, `card_in_trash`, or exact helpers are missing. If helper names differ, use existing source/material test helpers and preserve the assertions.

- [ ] **Step 3: Implement source-stack helpers**

In `code/digimon-engine/src/effect_context/mod.rs`, add:

```rust
pub fn trash_all_sources(&mut self, target: PermanentHandle) -> Result<(), EffectError> {
    let sources = self.game.sources_under(target).to_vec();
    for source in sources {
        self.game.move_source_to_trash(target, source)?;
        self.game.dispatch_source_trashed_event(target, source, self.controller);
    }
    Ok(())
}

pub fn play_selected_source_free(
    &mut self,
    target: PermanentHandle,
    source: CardHandle,
) -> Result<Option<PermanentHandle>, EffectError> {
    self.game.remove_source_card(target, source)?;
    self.play_card_by_effect(source, self.controller, 0)
}
```

Use existing internal movement functions if they already exist under different names. The important contract is stable source identity before movement and event dispatch after each source leaves.

- [ ] **Step 4: Add DSL tests for `trash_all_sources` and selected-source play**

Add to `code/digimon-engine/tests/dsl/source_stack_aggregates.rs`:

```rust
use digimon_dsl::{compile, spec::CardSpec};

#[test]
fn source_stack_steps_compile() {
    let yaml = r#"
card_id: TEST-SOURCE-STACK
name: Source Stack Test
card_type: digimon
colors: [green]
level: 6
play_cost: 11
dp: 11000
effects:
  - timing: main
    process:
      - select_opponent_permanent:
          filter: { kind: digimon }
          bind_as: target
      - trash_all_sources: { target: target }
"#;
    let spec: CardSpec = serde_yml::from_str(yaml).expect("yaml parses");
    compile(&spec).expect("source stack yaml compiles");
}
```

- [ ] **Step 5: Wire DSL lowerers**

Add `trash_all_sources` and `play_selected_sources_free` steps in `code/digimon-dsl/src/step.rs`, compile them in `code/digimon-dsl/src/compile.rs`, and lower them in `code/digimon-engine/src/dsl_cards/step/permanent_mutations.rs` or `zone_moves.rs` depending on the existing file split.

- [ ] **Step 6: Run focused tests**

Run:

```powershell
cargo test --manifest-path code\digimon-engine\Cargo.toml --test effect_context -- source_stack_operations --nocapture
cargo test --manifest-path code\digimon-engine\Cargo.toml --test dsl -- source_stack_aggregates group7_formula_batch group7_predicate_batch --nocapture
```

Expected: all PASS.

- [ ] **Step 7: Update trackers and commit**

```powershell
git add code/digimon-dsl/src/step.rs code/digimon-dsl/src/formula.rs code/digimon-engine/src/effect_context/mod.rs code/digimon-engine/src/dsl_cards/step/permanent_mutations.rs code/digimon-engine/src/dsl_cards/step/zone_moves.rs code/digimon-engine/src/dsl_cards/formula_eval.rs code/digimon-engine/tests/effect_context/source_stack_operations.rs code/digimon-engine/tests/dsl/source_stack_aggregates.rs docs/RUST_ENGINE_GAPS.md
git commit -m "feat: complete source-stack residual operations"
```

## Task 5: Hybrid/Tamer Alt-Path Lowering and Union-Zone Effect Digivolve

**Files:**
- Modify: `code/digimon-engine/src/dna_digivolve.rs`
- Modify: `code/digimon-engine/src/dsl_cards/lower_alt_path_registration.rs`
- Modify: `code/digimon-engine/src/dsl_cards/step/play_digivolve.rs`
- Test: `code/digimon-engine/tests/dsl/hybrid_tamer_digivolve.rs`
- Test: `code/digimon-engine/tests/effect_context/effect_digivolve_union_zones.rs`
- Docs: `qa/dsl-vocab-gaps.md`

- [ ] **Step 1: Add failing hybrid/Tamer mask test**

Create `code/digimon-engine/tests/dsl/hybrid_tamer_digivolve.rs`.

```rust
use digimon_engine::debug_runner::DebugGameRunner;

#[test]
fn tamer_alt_path_emits_digivolve_action_for_hybrid_card() {
    let mut r = DebugGameRunner::new();
    let p0 = 0;
    let tamer = r.play_tamer_for_test(p0, "BT17-085");
    r.add_card_to_hand_for_test(p0, "BT17-009");
    r.register_test_alt_path_source_treated_as("BT17-009", "BT17-085", "level_3_red_digimon");

    let mask = r.game.get_action_mask(p0);
    assert!(
        r.action_mask_has_digivolve_from_base(&mask, tamer),
        "hybrid alt path should expose the Tamer base as a legal digivolution base"
    );
}
```

- [ ] **Step 2: Run and verify failure**

Run:

```powershell
cargo test --manifest-path code\digimon-engine\Cargo.toml --test dsl -- hybrid_tamer_digivolve --nocapture
```

Expected: FAIL because `source_treated_as` is compile data but not fully consumed for normal action masks.

- [ ] **Step 3: Consume `source_treated_as` in action legality**

In `code/digimon-engine/src/dna_digivolve.rs` and `code/digimon-engine/src/dsl_cards/lower_alt_path_registration.rs`, treat `source_treated_as` as a typed base profile during candidate generation. The base profile must be used only for legality/cost checks and must not mutate the Tamer card's printed kind.

```rust
if let Some(profile) = &path.source_treated_as {
    if permanent_matches_treated_as_profile(game, base, profile) {
        candidates.push(DigivolutionCandidate {
            base,
            card,
            cost: path.cost,
            source_treated_as: Some(profile.clone()),
        });
    }
}
```

- [ ] **Step 4: Add union-zone into effect-digivolve test**

Create `code/digimon-engine/tests/effect_context/effect_digivolve_union_zones.rs`.

```rust
use digimon_engine::debug_runner::DebugGameRunner;
use digimon_engine::effect_context::EffectContext;
use digimon_engine::zones::UnionZoneSet;

#[test]
fn selected_union_zone_card_can_be_used_for_effect_digivolve() {
    let mut r = DebugGameRunner::new();
    let p0 = 0;
    let base = r.play_digimon_for_test(p0, "BT24-037", 6000);
    let hand_card = r.add_card_to_hand_for_test(p0, "BT24-085");

    {
        let mut ctx = EffectContext::new(&mut r.game, p0, Some(base));
        ctx.select_union_zone(
            p0,
            UnionZoneSet::hand_only(),
            "Choose card",
            false,
            move |_ctx, card| card == hand_card,
            move |ctx, chosen| {
                ctx.effect_digivolve_from_card(base, chosen, 0).expect("digivolve");
            },
        );
    }

    let action = r.game.pending_selection.as_ref().unwrap().valid_action_ids[0];
    r.game.resolve_selection(p0, action).expect("resolve union selection");
    assert_eq!(r.game.top_card_id(base), "BT24-085");
}
```

- [ ] **Step 5: Run focused tests**

Run:

```powershell
cargo test --manifest-path code\digimon-engine\Cargo.toml --test dsl -- hybrid_tamer_digivolve phase2e_select_union_zone --nocapture
cargo test --manifest-path code\digimon-engine\Cargo.toml --test effect_context -- effect_digivolve_union_zones effect_digivolve_from_zones --nocapture
```

Expected: all PASS.

- [ ] **Step 6: Update docs and commit**

```powershell
git add code/digimon-engine/src/dna_digivolve.rs code/digimon-engine/src/dsl_cards/lower_alt_path_registration.rs code/digimon-engine/src/dsl_cards/step/play_digivolve.rs code/digimon-engine/tests/dsl/hybrid_tamer_digivolve.rs code/digimon-engine/tests/effect_context/effect_digivolve_union_zones.rs qa/dsl-vocab-gaps.md
git commit -m "feat: lower hybrid tamer and union effect digivolve"
```

## Task 6: Event Payload and Result Binding Residuals

**Files:**
- Modify: `code/digimon-engine/src/events.rs`
- Modify: `code/digimon-engine/src/effect_queue.rs`
- Modify: `code/digimon-engine/src/dsl_cards/predicate.rs`
- Modify: `code/digimon-engine/src/dsl_cards/formula_eval.rs`
- Modify: `code/digimon-dsl/src/predicate.rs`
- Modify: `code/digimon-dsl/src/formula.rs`
- Test: `code/digimon-engine/tests/dsl/event_context_bindings.rs`
- Docs: `qa/dsl-vocab-gaps.md`

- [ ] **Step 1: Add failing event binding test**

Create `code/digimon-engine/tests/dsl/event_context_bindings.rs`.

```rust
use digimon_engine::debug_runner::DebugGameRunner;

#[test]
fn source_trashed_event_exposes_trashed_source_card_to_followup_predicate() {
    let mut r = DebugGameRunner::new();
    let p0 = 0;
    let stack = r.play_digimon_for_test(p0, "BT24-040", 11000);
    let source = r.attach_source_for_test(stack, "BT24-031");
    r.install_test_source_trash_observer_that_records_source_id(p0);

    r.trash_source_for_test(stack, source);

    assert_eq!(
        r.last_recorded_event_card_id_for_test(),
        Some("BT24-031".to_string())
    );
}
```

- [ ] **Step 2: Run and verify failure**

Run:

```powershell
cargo test --manifest-path code\digimon-engine\Cargo.toml --test dsl -- event_context_bindings --nocapture
```

Expected: FAIL if the source-trash event payload does not expose the exact source card to predicates/formulas.

- [ ] **Step 3: Add exact payload fields**

In `code/digimon-engine/src/events.rs`, add typed fields instead of deriving from later board scans.

```rust
pub struct SourceTrashedPayload {
    pub stack: PermanentHandle,
    pub source_card: CardHandle,
    pub source_card_id: String,
    pub controller: PlayerId,
    pub owner: PlayerId,
}
```

Pass this payload through `effect_queue.rs`, then expose DSL predicate reads in `code/digimon-engine/src/dsl_cards/predicate.rs`.

- [ ] **Step 4: Run event and existing trigger tests**

Run:

```powershell
cargo test --manifest-path code\digimon-engine\Cargo.toml --test dsl -- event_context_bindings group6_dynamic_formulas group7_predicate_batch --nocapture
cargo test --manifest-path code\digimon-engine\Cargo.toml --test timing_dispatch -- source --nocapture
```

Expected: all PASS.

- [ ] **Step 5: Update docs and commit**

```powershell
git add code/digimon-engine/src/events.rs code/digimon-engine/src/effect_queue.rs code/digimon-engine/src/dsl_cards/predicate.rs code/digimon-engine/src/dsl_cards/formula_eval.rs code/digimon-dsl/src/predicate.rs code/digimon-dsl/src/formula.rs code/digimon-engine/tests/dsl/event_context_bindings.rs qa/dsl-vocab-gaps.md
git commit -m "feat: complete event payload bindings"
```

## Task 7: Option and Security Disposition Cleanup

**Files:**
- Modify: `code/digimon-engine/src/effect_context/mod.rs`
- Modify: `code/digimon-engine/src/dsl_cards/step/zone_moves.rs`
- Modify: `code/digimon-dsl/src/step.rs`
- Test: `code/digimon-engine/tests/dsl/option_security_disposition.rs`
- Docs: `qa/dsl-vocab-gaps.md`

- [ ] **Step 1: Add failing security option disposition test**

Create `code/digimon-engine/tests/dsl/option_security_disposition.rs`.

```rust
use digimon_engine::debug_runner::DebugGameRunner;

#[test]
fn resolving_security_option_can_move_self_to_hand_without_trashing() {
    let mut r = DebugGameRunner::new();
    let p0 = 0;
    let option = r.place_security_for_test(p0, "EX7-074");
    r.install_test_security_effect_move_self_to_hand("EX7-074");

    r.resolve_security_option_for_test(p0, option);

    assert!(r.game.card_in_hand(p0, option));
    assert!(!r.game.card_in_trash(p0, option));
}
```

- [ ] **Step 2: Run and verify failure**

Run:

```powershell
cargo test --manifest-path code\digimon-engine\Cargo.toml --test dsl -- option_security_disposition --nocapture
```

Expected: FAIL if security option default trashing races the explicit movement.

- [ ] **Step 3: Centralize option disposition**

In `code/digimon-engine/src/effect_context/mod.rs`, add a resolving-option disposition marker.

```rust
pub enum ResolvingOptionDisposition {
    DefaultTrash,
    Moved(CardHandle),
    Suppressed,
}

pub fn move_resolving_option_to_hand(&mut self) -> Result<(), EffectError> {
    let option = self.current_resolving_option()?;
    self.game.move_card_to_hand(option, self.controller)?;
    self.mark_resolving_option_disposition(ResolvingOptionDisposition::Moved(option));
    Ok(())
}
```

Ensure the default option cleanup checks this marker before moving the option to trash.

- [ ] **Step 4: Wire DSL step names**

Add steps such as `move_resolving_option_to_hand` and `suppress_security_effect_disposition` in `code/digimon-dsl/src/step.rs`, compile them, and lower them in `zone_moves.rs`.

- [ ] **Step 5: Run focused and option-flow tests**

Run:

```powershell
cargo test --manifest-path code\digimon-engine\Cargo.toml --test dsl -- option_security_disposition delay phase2f4_schedule_delayed --nocapture
cargo test --manifest-path code\digimon-engine\Cargo.toml --test option_flow -- --nocapture
```

Expected: all PASS. Delay tests must keep passing because this task must not rebuild Delay.

- [ ] **Step 6: Update docs and commit**

```powershell
git add code/digimon-engine/src/effect_context/mod.rs code/digimon-engine/src/dsl_cards/step/zone_moves.rs code/digimon-dsl/src/step.rs code/digimon-engine/tests/dsl/option_security_disposition.rs qa/dsl-vocab-gaps.md
git commit -m "feat: centralize option security disposition"
```

## Task 8: Residual Formula and Predicate Vocabulary

**Files:**
- Modify: `code/digimon-dsl/src/formula.rs`
- Modify: `code/digimon-dsl/src/predicate.rs`
- Modify: `code/digimon-dsl/src/compile.rs`
- Modify: `code/digimon-engine/src/dsl_cards/formula_eval.rs`
- Modify: `code/digimon-engine/src/dsl_cards/predicate.rs`
- Test: `code/digimon-engine/tests/dsl/residual_formula_predicate_vocab.rs`
- Docs: `qa/dsl-vocab-gaps.md`

- [ ] **Step 1: Add failing formula/predicate tests for one proven residual**

Create `code/digimon-engine/tests/dsl/residual_formula_predicate_vocab.rs`.

```rust
use digimon_engine::debug_runner::DebugGameRunner;

#[test]
fn source_stack_dp_sum_formula_counts_matching_sources() {
    let mut r = DebugGameRunner::new();
    let p0 = 0;
    let stack = r.play_digimon_for_test(p0, "BT24-040", 11000);
    r.attach_source_for_test(stack, "BT24-031");
    r.attach_source_for_test(stack, "BT24-037");

    let sum = r.evaluate_test_formula(
        p0,
        stack,
        r#"{ source_stack_dp_sum: { target: self, filter: { trait_has: "Iliad" } } }"#,
    );

    assert_eq!(sum, 9000);
}
```

- [ ] **Step 2: Run and verify failure**

Run:

```powershell
cargo test --manifest-path code\digimon-engine\Cargo.toml --test dsl -- residual_formula_predicate_vocab --nocapture
```

Expected: FAIL because `source_stack_dp_sum` is absent.

- [ ] **Step 3: Add only the proven formula leaf**

In `code/digimon-dsl/src/formula.rs`, add:

```rust
SourceStackDpSum {
    target: String,
    #[serde(default)]
    filter: Option<PredicateSpec>,
},
```

Compile to a matching `CompiledFormula::SourceStackDpSum` and evaluate in `code/digimon-engine/src/dsl_cards/formula_eval.rs` by iterating the live source stack under the target permanent.

- [ ] **Step 4: Run formula batches**

Run:

```powershell
cargo test --manifest-path code\digimon-engine\Cargo.toml --test dsl -- residual_formula_predicate_vocab group7_formula_batch group7_predicate_batch group6_dynamic_formulas --nocapture
```

Expected: all PASS.

- [ ] **Step 5: Update docs and commit**

```powershell
git add code/digimon-dsl/src/formula.rs code/digimon-dsl/src/predicate.rs code/digimon-dsl/src/compile.rs code/digimon-engine/src/dsl_cards/formula_eval.rs code/digimon-engine/src/dsl_cards/predicate.rs code/digimon-engine/tests/dsl/residual_formula_predicate_vocab.rs qa/dsl-vocab-gaps.md
git commit -m "feat: add residual source-stack formula vocabulary"
```

## Task 9: Cross-Card Effect Enumeration and Re-Firing

**Files:**
- Modify: `code/digimon-engine/src/card_effects.rs`
- Modify: `code/digimon-engine/src/effect_context/mod.rs`
- Modify: `code/digimon-engine/src/effect_queue.rs`
- Modify: `code/digimon-dsl/src/step.rs`
- Modify: `code/digimon-engine/src/dsl_cards/step/effects.rs`
- Test: `code/digimon-engine/tests/effect_context/effect_refiring.rs`
- Test: `code/digimon-engine/tests/dsl/effect_refiring.rs`
- Docs: `docs/RUST_ENGINE_GAPS.md`

- [ ] **Step 1: Add failing engine effect enumeration test**

Create `code/digimon-engine/tests/effect_context/effect_refiring.rs`.

```rust
use digimon_engine::debug_runner::DebugGameRunner;
use digimon_engine::effect_context::EffectContext;

#[test]
fn refire_selected_when_digivolving_effect_preserves_source_identity() {
    let mut r = DebugGameRunner::new();
    let p0 = 0;
    let source_stack = r.play_digimon_for_test(p0, "BT22-042", 8000);
    let target = r.play_digimon_for_test(p0, "BT22-043", 9000);
    r.install_test_when_digivolving_gain_memory(target, 1);

    {
        let mut ctx = EffectContext::new(&mut r.game, p0, Some(source_stack));
        ctx.refire_effect_from_permanent(target, "when_digivolving")
            .expect("refire effect");
    }

    assert_eq!(r.game.memory(), 1);
    assert_eq!(r.last_effect_source_for_test(), Some(target));
}
```

- [ ] **Step 2: Run and verify failure**

Run:

```powershell
cargo test --manifest-path code\digimon-engine\Cargo.toml --test effect_context -- effect_refiring --nocapture
```

Expected: FAIL because reusable effect enumeration/refiring is absent.

- [ ] **Step 3: Add a constrained enumeration API**

In `code/digimon-engine/src/card_effects.rs`, expose only effects that are safe to refire by timing/kind.

```rust
pub struct ReFireableEffect {
    pub effect_id: EffectId,
    pub source: PermanentHandle,
    pub timing_key: String,
}

pub fn enumerate_refireable_effects(
    game: &Game,
    source: PermanentHandle,
    timing_key: &str,
) -> Vec<ReFireableEffect> {
    game.effects_for_permanent(source)
        .filter(|effect| effect.timing_key() == timing_key)
        .filter(|effect| effect.can_be_refired())
        .map(|effect| ReFireableEffect {
            effect_id: effect.id(),
            source,
            timing_key: timing_key.to_string(),
        })
        .collect()
}
```

- [ ] **Step 4: Add effect-context execution**

In `code/digimon-engine/src/effect_context/mod.rs`, execute through the normal effect queue with explicit source identity and once-per-turn accounting.

```rust
pub fn refire_effect_from_permanent(
    &mut self,
    source: PermanentHandle,
    timing_key: &str,
) -> Result<(), EffectError> {
    let effects = enumerate_refireable_effects(self.game, source, timing_key);
    if effects.len() == 1 {
        self.game.queue_refired_effect(effects[0].clone(), self.controller)?;
    } else if !effects.is_empty() {
        self.install_refire_effect_selection(effects)?;
    }
    Ok(())
}
```

- [ ] **Step 5: Add DSL step and test**

Add `refire_effect` to `code/digimon-dsl/src/step.rs`:

```rust
RefireEffect(RefireEffectArgs),

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RefireEffectArgs {
    pub source: String,
    pub timing: String,
    #[serde(default)]
    pub optional: bool,
}
```

Add `code/digimon-engine/tests/dsl/effect_refiring.rs` with a compile test for `refire_effect: { source: target, timing: when_digivolving, optional: true }`.

- [ ] **Step 6: Run focused tests**

Run:

```powershell
cargo test --manifest-path code\digimon-engine\Cargo.toml --test effect_context -- effect_refiring --nocapture
cargo test --manifest-path code\digimon-engine\Cargo.toml --test dsl -- effect_refiring --nocapture
```

Expected: all PASS.

- [ ] **Step 7: Update docs and commit**

```powershell
git add code/digimon-engine/src/card_effects.rs code/digimon-engine/src/effect_context/mod.rs code/digimon-engine/src/effect_queue.rs code/digimon-dsl/src/step.rs code/digimon-engine/src/dsl_cards/step/effects.rs code/digimon-engine/tests/effect_context/effect_refiring.rs code/digimon-engine/tests/dsl/effect_refiring.rs docs/RUST_ENGINE_GAPS.md
git commit -m "feat: add constrained effect refiring"
```

## Task 10: Production YAML Authoring Gates

**Files:**
- Modify: `code/digimon-engine/cards/**/*.yaml`
- Modify: `code/digimon-engine/tests/cards_behavioral/**/*.rs`
- Modify: `qa/qa-reports/validated_cards_dsl.json`
- Modify: `qa/archetype-qa/dsl/*.md`

- [ ] **Step 1: Pick one unblocked card per closed primitive**

Create a short list in the relevant archetype source note. Use only cards whose remaining blockers are closed by Tasks 1-9. A good first batch after Tasks 1-5 is:

```text
BT17-009 Flamemon - reveal bucket plus hybrid/Tamer route
BT24-031 Elecmon - reveal bucket
BT24-040 Venusmon - source-stack operation
BT24-101 Jupitermon - cross-permanent replacement
```

- [ ] **Step 2: Add one failing card-shaped test per card**

For each selected card, add or unignore a test under `code/digimon-engine/tests/cards_behavioral/<set>/<card>.rs`. The test name must name the printed clause.

```rust
#[test]
fn bt24_031_on_play_adds_iliad_and_ts_without_duplicate_reveal_pick() {
    let mut r = DebugGameRunner::new();
    r.stack_deck_for_test(0, vec!["BT24-031", "BT24-037", "BT24-085"]);
    r.play_card_from_hand_for_test(0, "BT24-031");

    let first_prompt = r.game.pending_selection.as_ref().expect("first bucket");
    let first_pick = first_prompt.valid_action_ids[0];
    r.game.resolve_selection(0, first_pick).expect("resolve first bucket");

    let second_prompt = r.game.pending_selection.as_ref().expect("second bucket");
    assert!(!second_prompt.valid_action_ids.contains(&first_pick));
}
```

- [ ] **Step 3: Run the card test and verify it fails before YAML**

Run one focused card test:

```powershell
cargo test --manifest-path code\digimon-engine\Cargo.toml --test cards_behavioral -- bt24_031_on_play_adds_iliad_and_ts_without_duplicate_reveal_pick --nocapture
```

Expected: FAIL because the production YAML is missing or incomplete.

- [ ] **Step 4: Author production YAML using only supported DSL**

Add the card YAML under `code/digimon-engine/cards/<set>/<card>.yaml`. For BT24-031 shape it like:

```yaml
card_id: BT24-031
name: Elecmon
card_type: digimon
colors: [yellow]
level: 3
play_cost: 3
dp: 1000
effects:
  - timing: on_play
    process:
      - reveal_top_deck: { of: you, count: 3, bind_as: revealed }
      - select_reveal_buckets:
          from: revealed
          buckets:
            - bind_as: iliad
              filter: { trait_has: Iliad }
              min: 0
              max: 1
            - bind_as: ts
              filter: { trait_has: TS }
              min: 0
              max: 1
          no_duplicate_cards: true
          prompt: "Choose cards to add"
      - add_to_hand_from_reveal: { card: iliad }
      - add_to_hand_from_reveal: { card: ts }
      - place_remainder_on_deck:
          from: revealed
          position: bottom
          order: any
```

- [ ] **Step 5: Run card and DSL gates**

Run:

```powershell
cargo test --manifest-path code\digimon-engine\Cargo.toml --test cards_behavioral -- bt24_031 --nocapture
cargo test --manifest-path code\digimon-engine\Cargo.toml --test dsl -- reveal_buckets hybrid_tamer_digivolve replacement_context source_stack_aggregates --nocapture
```

Expected: all PASS for the authored card and prerequisite DSL gates.

- [ ] **Step 6: Update validation report and archetype notes**

Update `qa/qa-reports/validated_cards_dsl.json` for each card whose behavioral tests pass. In each `qa/archetype-qa/dsl/*.md` source note, change the gap status to `card-local authoring complete` or keep the exact remaining blocker name if one remains.

- [ ] **Step 7: Commit the batch**

```powershell
git add code/digimon-engine/cards code/digimon-engine/tests/cards_behavioral qa/qa-reports/validated_cards_dsl.json qa/archetype-qa/dsl
git commit -m "feat: author unblocked archetype card yaml"
```

## Final Verification

- [ ] **Step 1: Run focused suite for all implemented slices**

```powershell
cargo test --manifest-path code\digimon-engine\Cargo.toml --test dsl -- reveal_buckets effect_granted_attack replacement_context source_stack_aggregates hybrid_tamer_digivolve event_context_bindings option_security_disposition residual_formula_predicate_vocab effect_refiring --nocapture
cargo test --manifest-path code\digimon-engine\Cargo.toml --test selection -- reveal_buckets union_zone ordered_permutation count_capped --nocapture
cargo test --manifest-path code\digimon-engine\Cargo.toml --test replacements -- cross_permanent context_predicates route_replacements nested_select_substrate --nocapture
cargo test --manifest-path code\digimon-engine\Cargo.toml --test effect_context -- source_stack_operations effect_digivolve_union_zones effect_refiring --nocapture
```

Expected: all PASS.

- [ ] **Step 2: Run broader gates if action masks or PyO3 metadata changed**

Run only if a task changed action IDs, mask metadata, PyO3 exports, tensor metadata, or RL wrapper assumptions:

```powershell
$env:DIGIMON_BACKEND='rust'; python -m pytest code\tests\rl -v
$env:DIGIMON_BACKEND='rust'; python -m pytest code\engine_py_legacy\tests\engine\test_rust_backend_parity.py -v
```

Expected: all PASS. If these fail because a contract actually changed, stop and write a separate action/tensor contract plan before merging.

## Self-Review

- **Spec coverage:** Tasks map to all ten narrowed slices in `docs/superpowers/specs/2026-05-03-latest-archetype-dsl-engine-gap-closure-design.md`.
- **Implementation audit honored:** Existing reveal remainder, Delay, union-zone selection, end-of-turn attack modifiers, replacement predicates, and formula batches are reused rather than rebuilt.
- **Contract safety:** No task expands `ACTION_SPACE_SIZE` or tensor layouts. Any discovered need for that work is routed to a separate contract plan.
- **TDD shape:** Every capability task starts with a failing focused test, then implementation, then focused regression commands.
- **Tracker hygiene:** Each task includes tracker updates before commit.
