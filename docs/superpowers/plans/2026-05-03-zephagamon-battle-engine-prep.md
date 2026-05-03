# Zephagamon Battle Engine Prep Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Prepare the Rust engine and YAML DSL to model Zephagamon/Vortexdramon effects that cause a Digimon to battle without declaring an attack, while preserving the rule that battle alone does not trigger Piercing.

**Architecture:** Add a public direct-battle engine primitive that reuses the DP comparison and battle deletion machinery but never creates `PendingAttack`, never fires attack timings, and never enters the Piercing security-check path. Then expose it through a minimal DSL `battle` step that consumes two permanent bindings, and fix Vortex end-of-turn action decoding/target gating so Zephagamon attack effects remain distinct from effect battles.

**Tech Stack:** Rust engine in `code/digimon-engine`, Rust DSL crate in `code/digimon-dsl`, YAML card specs in `code/digimon-engine/cards`, tests via `cargo test` from `code/digimon-engine`.

---

## File Structure

- Modify `code/digimon-engine/src/combat.rs`: add `Game::battle_digimon(attacker, defender)` as the public effect-battle primitive; keep `resolve_battle` private and shared.
- Modify `code/digimon-engine/src/effect_context/mod.rs`: expose `EffectContext::battle_digimon`.
- Modify `code/digimon-engine/src/debug_runner.rs`: expose `DebugRunner::battle_digimon` for tests.
- Create `code/digimon-engine/tests/combat/effect_battle.rs`: tests for direct effect battle, no attack timings, no Piercing, and battle-trigger compatibility.
- Modify `code/digimon-dsl/src/step.rs`: add YAML `battle:` step args.
- Modify `code/digimon-dsl/src/compiled.rs`: add `CompiledStep::Battle`.
- Modify `code/digimon-dsl/src/compile.rs`: compile `StepSpec::Battle` into `CompiledStep::Battle`.
- Modify `code/digimon-engine/src/dsl_cards/step/combat.rs`: execute compiled `Battle` by resolving attacker/defender bindings and calling `EffectContext::battle_digimon`.
- Modify `code/digimon-dsl/src/validator.rs`: admit the new `battle` step and validate both bindings syntactically.
- Create `code/digimon-engine/tests/dsl/effect_battle_step.rs`: YAML-backed integration tests for `select_opponent_permanent` followed by `battle`.
- Modify `code/digimon-engine/src/action/decode.rs`: make EndOfTurnAction decoded attacks pass `vortex=true` only when the attacker actually has `<Vortex>`.
- Modify `code/digimon-engine/src/action/mask.rs`: stop base Vortex from emitting player/security target bits unless a dedicated player-target permission is present.
- Modify `code/digimon-engine/src/enums.rs`, `code/digimon-engine/src/dsl_cards/modifier_map.rs`, and `code/digimon-dsl/src/validator.rs`: add `ModifierType::VortexCanAttackPlayer` so EX11-062 Shoto can grant the player-target extension explicitly.
- Modify `docs/RUST_PYTHON_PARITY.md`, `docs/RUST_ENGINE_GAPS.md`, and `qa/dsl-vocab-gaps.md`: record the new battle primitive, Vortex decode fix, and remaining Zephagamon-specific card authoring gaps.

---

### Task 1: Add the Direct Effect-Battle Primitive

**Files:**
- Modify: `code/digimon-engine/src/combat.rs`
- Modify: `code/digimon-engine/src/debug_runner.rs`
- Test: `code/digimon-engine/tests/combat/effect_battle.rs`

- [ ] **Step 1: Write failing tests for battle without attack timing or Piercing**

Create `code/digimon-engine/tests/combat/effect_battle.rs`:

```rust
use std::sync::{Arc, Mutex};

use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::effect::{CardEffect, Effect, EffectBuilder};
use digimon_engine::effect_context::EffectContext;
use digimon_engine::enums::{CardColor, CardKind, EffectTiming, Expiry, Keyword};
use digimon_engine::permanent::PermanentHandle;

fn digimon(id: &str, dp: i32) -> CardData {
    CardData {
        card_id: id.to_string(),
        card_name: id.to_string(),
        card_kind: CardKind::Digimon,
        level: Some(5),
        dp: Some(dp),
        play_cost: 6,
        colors: vec![CardColor::Green],
        traits: Vec::new(),
        evo_costs: Vec::new(),
        dna_costs: Vec::new(),
        effect_text: String::new(),
        inherited_text: String::new(),
        security_text: String::new(),
        keywords: Vec::new(),
        dual: None,
        effect_class_name: id.replace('-', "_"),
        index: 0,
        norm_id: 0.0,
        ace_overflow: None,
        digixros_aliases: Vec::new(),
    }
}

struct TimingWitness {
    on_attack: Arc<Mutex<u32>>,
    when_attacking: Arc<Mutex<u32>>,
    end_of_attack: Arc<Mutex<u32>>,
    end_of_battle: Arc<Mutex<u32>>,
}

impl CardEffect for TimingWitness {
    fn effects(&self, c: digimon_engine::card_source::CardHandle) -> Vec<Effect> {
        let on_attack = self.on_attack.clone();
        let when_attacking = self.when_attacking.clone();
        let end_of_attack = self.end_of_attack.clone();
        let end_of_battle = self.end_of_battle.clone();

        vec![
            EffectBuilder::new(c, EffectTiming::OnAttack)
                .name("record OnAttack")
                .mandatory()
                .build(move |_ctx: &mut EffectContext| {
                    *on_attack.lock().unwrap() += 1;
                }),
            EffectBuilder::new(c, EffectTiming::WhenAttacking)
                .name("record WhenAttacking")
                .mandatory()
                .build(move |_ctx: &mut EffectContext| {
                    *when_attacking.lock().unwrap() += 1;
                }),
            Effect::end_of_attack(c)
                .name("record EndOfAttack")
                .mandatory()
                .build(move |_ctx: &mut EffectContext| {
                    *end_of_attack.lock().unwrap() += 1;
                }),
            Effect::end_of_battle(c)
                .name("record EndOfBattle")
                .mandatory()
                .build(move |_ctx: &mut EffectContext| {
                    *end_of_battle.lock().unwrap() += 1;
                }),
        ]
    }
}

#[test]
fn direct_battle_deletes_loser_but_does_not_trigger_piercing() {
    let mut r = DebugRunner::builder()
        .add_card(digimon("ATK", 12000))
        .add_card(digimon("DEF", 3000))
        .add_card(make_test_card("SEC", "Security filler"))
        .deck(1, &["SEC"; 5])
        .security(1, &["SEC", "SEC", "SEC"])
        .start();

    let attacker = r.place_on_field(0, "ATK", Some(0));
    let defender = r.place_on_field(1, "DEF", Some(0));
    r.game
        .modifiers
        .grant_keyword(attacker, Keyword::Piercing, Expiry::Permanent, 0);

    let security_before = r.security_count(1);
    let result = r.battle_digimon(attacker, defender);

    assert_eq!(result, digimon_engine::combat::AttackResult::AttackerWins);
    assert_eq!(r.battle_area_size(1), 0, "defender loses the battle");
    assert_eq!(
        r.security_count(1),
        security_before,
        "effect battle is not an attack, so Piercing must not perform a security check"
    );
    assert!(r.game.pending_attack.is_none(), "effect battle must not install PendingAttack");
}

#[test]
fn direct_battle_fires_end_of_battle_but_no_attack_timings() {
    let on_attack = Arc::new(Mutex::new(0));
    let when_attacking = Arc::new(Mutex::new(0));
    let end_of_attack = Arc::new(Mutex::new(0));
    let end_of_battle = Arc::new(Mutex::new(0));

    let mut r = DebugRunner::builder()
        .add_card(digimon("ATK", 8000))
        .add_card(digimon("DEF", 3000))
        .add_card(digimon("OBS", 1000))
        .start();

    r.register_effect(
        "OBS",
        Arc::new(TimingWitness {
            on_attack: on_attack.clone(),
            when_attacking: when_attacking.clone(),
            end_of_attack: end_of_attack.clone(),
            end_of_battle: end_of_battle.clone(),
        }),
    );

    let attacker = r.place_on_field(0, "ATK", Some(0));
    let defender = r.place_on_field(1, "DEF", Some(0));
    r.place_on_field(0, "OBS", Some(0));

    let result = r.battle_digimon(attacker, defender);

    assert_eq!(result, digimon_engine::combat::AttackResult::AttackerWins);
    assert_eq!(*on_attack.lock().unwrap(), 0, "effect battle is not OnAttack");
    assert_eq!(*when_attacking.lock().unwrap(), 0, "effect battle is not WhenAttacking");
    assert_eq!(*end_of_attack.lock().unwrap(), 0, "effect battle does not end an attack");
    assert_eq!(*end_of_battle.lock().unwrap(), 1, "effect battle still ends a battle");
}

#[test]
fn direct_battle_rejects_same_controller_targets() {
    let mut r = DebugRunner::builder()
        .add_card(digimon("ATK", 8000))
        .add_card(digimon("ALLY", 3000))
        .start();

    let attacker = r.place_on_field(0, "ATK", Some(0));
    let ally = r.place_on_field(0, "ALLY", Some(0));

    let result = r.battle_digimon(attacker, ally);

    assert_eq!(result, digimon_engine::combat::AttackResult::Invalid);
    assert_eq!(r.battle_area_size(0), 2, "invalid direct battle changes nothing");
}
```

- [ ] **Step 2: Run the failing test**

Run:

```bash
cd code/digimon-engine
cargo test --test effect_battle -- --nocapture
```

Expected: FAIL because `DebugRunner::battle_digimon` and `Game::battle_digimon` do not exist.

- [ ] **Step 3: Add `Game::battle_digimon`**

In `code/digimon-engine/src/combat.rs`, add this public method inside `impl Game`, immediately after `attack_player`:

```rust
    /// Resolve an effect-driven Digimon-vs-Digimon battle without declaring
    /// an attack.
    ///
    /// This is for printed text such as "this Digimon may battle 1 of your
    /// opponent's Digimon." It intentionally does not install `PendingAttack`,
    /// suspend the source, mark `is_attacking`, fire attack timings, open
    /// interrupt windows, or perform the Piercing post-battle security check.
    /// It only runs the DP/Iceclad battle resolver, battle-caused deletion
    /// replacements, OnDeletion, and EndOfBattle timing.
    pub fn battle_digimon(
        &mut self,
        attacker: PermanentHandle,
        defender: PermanentHandle,
    ) -> AttackResult {
        if attacker.player == defender.player {
            return AttackResult::Invalid;
        }
        if !self.handle_valid(attacker) || !self.handle_valid(defender) {
            return AttackResult::Invalid;
        }

        self.resolve_battle(attacker, defender)
    }
```

Do not call `begin_attack`, `attack_digimon`, `advance_pending_attack`, `cleanup_attack`, or `enter_piercing_security_check` from this method.

- [ ] **Step 4: Add DebugRunner helper**

In `code/digimon-engine/src/debug_runner.rs`, add this method next to the existing `attack_digimon` helper:

```rust
    /// Resolve an effect-driven battle. Unlike `attack_digimon`, this does
    /// not declare an attack and must not trigger Piercing.
    pub fn battle_digimon(
        &mut self,
        attacker: PermanentHandle,
        defender: PermanentHandle,
    ) -> crate::combat::AttackResult {
        self.game.battle_digimon(attacker, defender)
    }
```

- [ ] **Step 5: Run the combat test**

Run:

```bash
cd code/digimon-engine
cargo test --test effect_battle -- --nocapture
```

Expected: PASS.

- [ ] **Step 6: Run adjacent combat tests**

Run:

```bash
cd code/digimon-engine
cargo test --test piercing_security -- --nocapture
cargo test --test timing_dispatch -- --nocapture
```

Expected: PASS. Piercing should still work after normal attacks, and EndOfBattle timing should remain intact.

- [ ] **Step 7: Commit**

```bash
git add code/digimon-engine/src/combat.rs code/digimon-engine/src/debug_runner.rs code/digimon-engine/tests/combat/effect_battle.rs
git commit -m "feat(engine): add direct digimon battle primitive"
```

---

### Task 2: Expose Direct Battle Through EffectContext and DSL

**Files:**
- Modify: `code/digimon-engine/src/effect_context/mod.rs`
- Modify: `code/digimon-dsl/src/step.rs`
- Modify: `code/digimon-dsl/src/compiled.rs`
- Modify: `code/digimon-dsl/src/compile.rs`
- Modify: `code/digimon-engine/src/dsl_cards/step/combat.rs`
- Modify: `code/digimon-engine/src/dsl_cards/step/mod.rs`
- Modify: `code/digimon-dsl/src/validator.rs`
- Test: `code/digimon-engine/tests/dsl/effect_battle_step.rs`

- [ ] **Step 1: Add a failing DSL integration test**

Create `code/digimon-engine/tests/dsl/effect_battle_step.rs`:

```rust
use std::fs;
use std::path::PathBuf;

use digimon_engine::action::{encode_attack, encode_field_effect, PASS, SECURITY_TARGET};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, GamePhase};

fn digimon(id: &str, dp: i32) -> CardData {
    CardData {
        card_id: id.to_string(),
        card_name: id.to_string(),
        card_kind: CardKind::Digimon,
        level: Some(6),
        dp: Some(dp),
        play_cost: 11,
        colors: vec![CardColor::Green],
        traits: Vec::new(),
        evo_costs: Vec::new(),
        dna_costs: Vec::new(),
        effect_text: String::new(),
        inherited_text: String::new(),
        security_text: String::new(),
        keywords: Vec::new(),
        dual: None,
        effect_class_name: id.replace('-', "_"),
        index: 0,
        norm_id: 0.0,
        ace_overflow: None,
        digixros_aliases: Vec::new(),
    }
}

fn write_temp_card_yaml(card_id: &str, body: &str) -> PathBuf {
    let dir = std::env::temp_dir().join("digimon_engine_dsl_effect_battle_tests");
    fs::create_dir_all(&dir).unwrap();
    let path = dir.join(format!("{card_id}.yaml"));
    fs::write(&path, body).unwrap();
    path
}

#[test]
fn dsl_battle_step_resolves_battle_without_piercing_security() {
    let yaml = r#"
card_id: TEST-BATTLE
clauses:
  - timing: main
    mandatory: false
    steps:
      - select_opponent_permanent:
          prompt: "Choose an opponent's Digimon to battle"
          bind_as: battle_target
          optional: false
          filter:
            card_kind: Digimon
            zone: battle_area
      - battle:
          attacker: this
          defender: battle_target
"#;
    let path = write_temp_card_yaml("TEST-BATTLE", yaml);

    let mut r = DebugRunner::builder()
        .add_card(digimon("TEST-BATTLE", 12000))
        .add_card(digimon("DEF", 3000))
        .add_card(make_test_card("SEC", "Security filler"))
        .deck(1, &["SEC"; 5])
        .security(1, &["SEC", "SEC"])
        .dsl_card("TEST-BATTLE", path)
        .start();

    let attacker = r.place_on_field(0, "TEST-BATTLE", Some(0));
    let defender = r.place_on_field(1, "DEF", Some(0));
    r.game
        .modifiers
        .grant_keyword(attacker, digimon_engine::enums::Keyword::Piercing, digimon_engine::enums::Expiry::Permanent, 0);

    let security_before = r.security_count(1);
    let action = encode_field_effect(attacker.index as u16, 0);
    r.game.decode_action(action, 0);

    assert_eq!(r.game.current_phase, GamePhase::SelectTarget);
    r.game.decode_action(encode_attack(attacker.index as u16, defender.index as u16), 0);

    assert_eq!(r.battle_area_size(1), 0, "selected defender loses the effect battle");
    assert_eq!(r.security_count(1), security_before, "Piercing does not trigger from battle step");
    assert!(r.game.pending_attack.is_none());
}

#[test]
fn optional_dsl_battle_selection_can_be_declined() {
    let yaml = r#"
card_id: TEST-BATTLE-OPT
clauses:
  - timing: main
    mandatory: false
    steps:
      - select_opponent_permanent:
          prompt: "Choose an opponent's Digimon to battle"
          bind_as: battle_target
          optional: true
          filter:
            card_kind: Digimon
            zone: battle_area
      - battle:
          attacker: this
          defender: battle_target
"#;
    let path = write_temp_card_yaml("TEST-BATTLE-OPT", yaml);

    let mut r = DebugRunner::builder()
        .add_card(digimon("TEST-BATTLE-OPT", 12000))
        .add_card(digimon("DEF", 3000))
        .dsl_card("TEST-BATTLE-OPT", path)
        .start();

    let attacker = r.place_on_field(0, "TEST-BATTLE-OPT", Some(0));
    r.place_on_field(1, "DEF", Some(0));

    let action = encode_field_effect(attacker.index as u16, 0);
    r.game.decode_action(action, 0);
    assert_eq!(r.game.current_phase, GamePhase::SelectTarget);

    r.game.decode_action(PASS, 0);

    assert_eq!(r.battle_area_size(1), 1, "declining optional battle leaves defender in play");
    assert!(r.game.pending_attack.is_none());
}
```

- [ ] **Step 2: Run the failing DSL test**

Run:

```bash
cd code/digimon-engine
cargo test --test effect_battle_step -- --nocapture
```

Expected: FAIL because the YAML parser/compiler does not know `battle`.

- [ ] **Step 3: Add DSL argument type and StepSpec variant**

In `code/digimon-dsl/src/step.rs`, add the variant under `// Combat / replacement process outcomes`:

```rust
    Battle(BattleArgs),
```

Add this argument struct near `TargetArg`:

```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct BattleArgs {
    pub attacker: BindingRef,
    pub defender: BindingRef,
}
```

In the custom serializer match, add:

```rust
            StepSpec::Battle(v) => kv!(s, "battle", v),
```

In the custom deserializer match, add:

```rust
            "battle" => StepSpec::Battle(map.next_value()?),
```

- [ ] **Step 4: Add compiled step**

In `code/digimon-dsl/src/compiled.rs`, add this enum variant near `EndAttack`:

```rust
    Battle {
        attacker: CompiledBindingRef,
        defender: CompiledBindingRef,
    },
```

- [ ] **Step 5: Compile Battle step**

In `code/digimon-dsl/src/compile.rs`, add this match arm near `S::EndAttack`:

```rust
        S::Battle(args) => CompiledStep::Battle {
            attacker: compile_binding_ref(&args.attacker),
            defender: compile_binding_ref(&args.defender),
        },
```

Use the existing binding-ref compile helper already used by permanent mutation steps. If the local helper has a different name, use that existing helper rather than introducing a duplicate conversion path.

- [ ] **Step 6: Expose EffectContext battle helper**

In `code/digimon-engine/src/effect_context/mod.rs`, add this method in the combat mutations section:

```rust
    /// Resolve an effect-driven battle between two Digimon without declaring
    /// an attack. This must not trigger attack timings or Piercing.
    pub fn battle_digimon(
        &mut self,
        attacker: crate::permanent::PermanentHandle,
        defender: crate::permanent::PermanentHandle,
    ) -> crate::combat::AttackResult {
        self.game.battle_digimon(attacker, defender)
    }
```

- [ ] **Step 7: Execute compiled Battle step**

Replace `code/digimon-engine/src/dsl_cards/step/combat.rs` with:

```rust
use digimon_dsl::compiled::CompiledStep;

use crate::dsl_cards::binding_ref::{resolve_binding_ref, ResolvedBinding};
use crate::dsl_cards::bindings::Bindings;
use crate::effect_context::EffectContext;

pub fn try_run(
    step: &CompiledStep,
    ctx: &mut EffectContext<'_>,
    bindings: &mut Bindings,
) -> bool {
    match step {
        CompiledStep::Battle { attacker, defender } => {
            let Some(ResolvedBinding::Permanent(attacker)) =
                resolve_binding_ref(attacker, ctx, bindings)
            else {
                return true;
            };
            let Some(ResolvedBinding::Permanent(defender)) =
                resolve_binding_ref(defender, ctx, bindings)
            else {
                return true;
            };
            let _ = ctx.battle_digimon(attacker, defender);
            true
        }
        CompiledStep::EndAttack { enabled } => {
            if *enabled {
                ctx.cancel_pending_attack();
            }
            true
        }
        _ => false,
    }
}
```

Then update `code/digimon-engine/src/dsl_cards/step/mod.rs` where `combat::try_run` is called so it passes `bindings`:

```rust
        if combat::try_run(step, ctx, bindings) {
            continue;
        }
```

- [ ] **Step 8: Admit `battle` in DSL validation**

In `code/digimon-dsl/src/validator.rs`, update the step validation match to recognize `StepSpec::Battle(args)`. The validation should only require that both binding refs are syntactically valid references; runtime type checking remains in the executor.

Use this shape in the existing validation function:

```rust
            StepSpec::Battle(args) => {
                validate_binding_ref(&args.attacker, "battle.attacker", errors);
                validate_binding_ref(&args.defender, "battle.defender", errors);
            }
```

If the file uses a different helper name than `validate_binding_ref`, call the existing helper that validates `TargetArg.target` for `suspend`, `unsuspend`, and `delete_permanent`.

- [ ] **Step 9: Run DSL and combat tests**

Run:

```bash
cd code/digimon-engine
cargo test --test effect_battle_step -- --nocapture
cargo test --test effect_battle -- --nocapture
```

Expected: PASS.

- [ ] **Step 10: Run DSL parser/compiler tests**

Run:

```bash
cd code/digimon-dsl
cargo test
```

Expected: PASS.

- [ ] **Step 11: Commit**

```bash
git add code/digimon-engine/src/effect_context/mod.rs code/digimon-engine/src/dsl_cards/step/combat.rs code/digimon-engine/src/dsl_cards/step/mod.rs code/digimon-engine/tests/dsl/effect_battle_step.rs code/digimon-dsl/src/step.rs code/digimon-dsl/src/compiled.rs code/digimon-dsl/src/compile.rs code/digimon-dsl/src/validator.rs
git commit -m "feat(dsl): add effect battle step"
```

---

### Task 3: Fix Vortex End-of-Turn Decode and Player Target Gating

**Files:**
- Modify: `code/digimon-engine/src/action/decode.rs`
- Modify: `code/digimon-engine/src/action/mask.rs`
- Modify: `code/digimon-engine/src/enums.rs`
- Modify: `code/digimon-engine/src/dsl_cards/modifier_map.rs`
- Modify: `code/digimon-dsl/src/validator.rs`
- Test: `code/digimon-engine/tests/mask_and_tensor/mask_end_of_turn_parity.rs`

- [ ] **Step 1: Add failing tests for decoded Vortex attacks and player target gating**

Append to `code/digimon-engine/tests/mask_and_tensor/mask_end_of_turn_parity.rs`:

```rust
#[test]
fn decode_vortex_attack_uses_vortex_flag_for_fresh_attacker() {
    let mut r = DebugRunner::builder()
        .add_card(make_digimon("ATK", CardColor::Red, 5000))
        .add_card(make_digimon("DEF", CardColor::Blue, 3000))
        .start();

    let tp = r.game.turn_player();
    let opp = 1 - tp;
    let attacker = r.place_on_field(tp, "ATK", None);
    let defender = r.place_on_field(opp, "DEF", Some(0));

    r.game
        .modifiers
        .grant_keyword(attacker, Keyword::Vortex, Expiry::EndOfTurn, tp);
    r.game.current_phase = GamePhase::EndOfTurnAction;

    r.game
        .decode_action(encode_attack(attacker.index as u16, defender.index as u16), tp);

    assert_eq!(
        r.battle_area_size(opp),
        0,
        "decoded EndOfTurnAction Vortex attack must bypass summoning sickness"
    );
}

#[test]
fn base_vortex_does_not_emit_security_target_without_player_extension() {
    let mut r = DebugRunner::builder()
        .add_card(make_digimon("ATK", CardColor::Red, 5000))
        .start();

    let tp = r.game.turn_player();
    let attacker = r.place_on_field(tp, "ATK", Some(0));

    r.game
        .modifiers
        .grant_keyword(attacker, Keyword::Vortex, Expiry::EndOfTurn, tp);
    r.game.current_phase = GamePhase::EndOfTurnAction;

    let mask = build_action_mask(&r.game, tp);

    assert_eq!(
        mask[encode_attack(attacker.index as u16, SECURITY_TARGET) as usize],
        0.0,
        "base Vortex attacks opponent Digimon, not players"
    );
}

#[test]
fn vortex_player_extension_emits_security_target() {
    let mut r = DebugRunner::builder()
        .add_card(make_digimon("ATK", CardColor::Red, 5000))
        .start();

    let tp = r.game.turn_player();
    let attacker = r.place_on_field(tp, "ATK", Some(0));

    r.game
        .modifiers
        .grant_keyword(attacker, Keyword::Vortex, Expiry::EndOfTurn, tp);
    r.game.modifiers.add(
        attacker,
        ModifierEntry::simple(
            ModifierType::VortexCanAttackPlayer,
            1,
            Expiry::EndOfTurn,
            tp,
        ),
    );
    r.game.current_phase = GamePhase::EndOfTurnAction;

    let mask = build_action_mask(&r.game, tp);

    assert_eq!(
        mask[encode_attack(attacker.index as u16, SECURITY_TARGET) as usize],
        1.0,
        "EX11-062-style extension lets Vortex attack the player"
    );
}
```

Also update the existing `mask_vortex_emits_attacks_in_end_of_turn_phase` expectation so it no longer expects security/player attack from base Vortex. Keep the Digimon target assertion.

- [ ] **Step 2: Run the failing Vortex tests**

Run:

```bash
cd code/digimon-engine
cargo test --test mask_end_of_turn_parity -- --nocapture
```

Expected: FAIL because decoded EndOfTurnAction attacks pass `vortex=false`, and base Vortex currently emits security/player targets.

- [ ] **Step 3: Add explicit Vortex player-target modifier**

In `code/digimon-engine/src/enums.rs`, add under the Attack modifier group:

```rust
    VortexCanAttackPlayer,
```

In `code/digimon-engine/src/dsl_cards/modifier_map.rs`, add:

```rust
        "VortexCanAttackPlayer" => ModifierType::VortexCanAttackPlayer,
```

In `code/digimon-dsl/src/validator.rs`, add `"VortexCanAttackPlayer"` to the known modifier string list.

- [ ] **Step 4: Fix EndOfTurnAction decode**

In `code/digimon-engine/src/action/decode.rs`, replace the EndOfTurnAction attack call:

```rust
            self.execute_attack(tp, attacker_idx, target_idx);
```

with:

```rust
            self.execute_end_of_turn_attack(tp, attacker_idx, target_idx);
```

Add this helper next to `execute_attack`:

```rust
    fn execute_end_of_turn_attack(&mut self, player: PlayerId, attacker_idx: u8, target_idx: u8) {
        let attacker_handle = PermanentHandle {
            player,
            index: attacker_idx,
        };
        if (attacker_idx as usize) >= self.player(player).battle_area.len() {
            return;
        }

        let vortex = self.has_keyword(attacker_handle, crate::enums::Keyword::Vortex);
        if target_idx == SECURITY_TARGET as u8 {
            let opponent = self.next_clockwise(player);
            let _ = self.attack_player(attacker_handle, opponent, vortex);
            return;
        }

        let opponent = self.next_clockwise(player);
        if (target_idx as usize) >= self.player(opponent).battle_area.len() {
            return;
        }
        let defender = PermanentHandle {
            player: opponent,
            index: target_idx,
        };
        let _ = self.attack_digimon(attacker_handle, defender, vortex);
    }
```

Leave Main-phase `execute_attack` unchanged so normal attacks still pass `vortex=false`.

- [ ] **Step 5: Gate Vortex player/security target bits**

In `code/digimon-engine/src/action/mask.rs`, replace the unconditional EndOfTurnAction security emission:

```rust
                if !game.modifiers.has(handle, ModifierType::CannotAttackPlayer) {
                    mask[encode_attack(i as u16, SECURITY_TARGET) as usize] = 1.0;
                }
```

with:

```rust
                let can_attack_player_from_eot =
                    !game.modifiers.has(handle, ModifierType::CannotAttackPlayer)
                        && (!vortex || game.modifiers.has(handle, ModifierType::VortexCanAttackPlayer));
                if can_attack_player_from_eot {
                    mask[encode_attack(i as u16, SECURITY_TARGET) as usize] = 1.0;
                }
```

This preserves player targets for `MayAttack` and `ForceAttack`, but requires `VortexCanAttackPlayer` for Vortex-only player targets.

- [ ] **Step 6: Run Vortex tests**

Run:

```bash
cd code/digimon-engine
cargo test --test mask_end_of_turn_parity -- --nocapture
```

Expected: PASS.

- [ ] **Step 7: Run action decoder and combat regression tests**

Run:

```bash
cd code/digimon-engine
cargo test --test action_explain -- --nocapture
cargo test --test piercing_security -- --nocapture
cargo test --test effect_battle -- --nocapture
```

Expected: PASS.

- [ ] **Step 8: Commit**

```bash
git add code/digimon-engine/src/action/decode.rs code/digimon-engine/src/action/mask.rs code/digimon-engine/src/enums.rs code/digimon-engine/src/dsl_cards/modifier_map.rs code/digimon-dsl/src/validator.rs code/digimon-engine/tests/mask_and_tensor/mask_end_of_turn_parity.rs
git commit -m "fix(engine): preserve vortex attack semantics"
```

---

### Task 4: Add a Zephagamon Readiness Fixture

**Files:**
- Create: `code/digimon-engine/cards/ex11/EX11-074.yaml`
- Test: `code/digimon-engine/tests/cards/ex11_074_vortexdramon.rs`
- Modify: `docs/RUST_ENGINE_GAPS.md`
- Modify: `qa/dsl-vocab-gaps.md`

- [ ] **Step 1: Write a focused EX11-074 test fixture**

Create `code/digimon-engine/tests/cards/ex11_074_vortexdramon.rs`:

```rust
use digimon_engine::action::{encode_attack, encode_field_effect};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, Expiry, Keyword};

fn digimon(id: &str, dp: i32) -> CardData {
    CardData {
        card_id: id.to_string(),
        card_name: id.to_string(),
        card_kind: CardKind::Digimon,
        level: Some(6),
        dp: Some(dp),
        play_cost: 12,
        colors: vec![CardColor::Green],
        traits: Vec::new(),
        evo_costs: Vec::new(),
        dna_costs: Vec::new(),
        effect_text: String::new(),
        inherited_text: String::new(),
        security_text: String::new(),
        keywords: Vec::new(),
        dual: None,
        effect_class_name: id.replace('-', "_"),
        index: 0,
        norm_id: 0.0,
        ace_overflow: None,
        digixros_aliases: Vec::new(),
    }
}

#[test]
fn ex11_074_may_battle_effect_does_not_pierce() {
    let mut r = DebugRunner::builder()
        .add_card(digimon("EX11-074", 12000))
        .add_card(digimon("DEF", 3000))
        .add_card(make_test_card("SEC", "Security filler"))
        .deck(1, &["SEC"; 5])
        .security(1, &["SEC", "SEC"])
        .yaml_card("EX11-074")
        .start();

    let vortexdramon = r.place_on_field(0, "EX11-074", Some(0));
    let defender = r.place_on_field(1, "DEF", Some(0));
    r.game
        .modifiers
        .grant_keyword(vortexdramon, Keyword::Piercing, Expiry::Permanent, 0);

    let security_before = r.security_count(1);

    r.game.decode_action(encode_field_effect(vortexdramon.index as u16, 0), 0);
    r.game
        .decode_action(encode_attack(vortexdramon.index as u16, defender.index as u16), 0);

    assert_eq!(r.battle_area_size(1), 0);
    assert_eq!(
        r.security_count(1),
        security_before,
        "EX11-074's effect battle is not an attack and must not trigger Piercing"
    );
}
```

If the test harness does not expose `.yaml_card("EX11-074")`, use the same temporary YAML pattern from Task 2 and leave a follow-up note in `docs/RUST_ENGINE_GAPS.md` to add a common YAML card loader helper.

- [ ] **Step 2: Run the failing EX11-074 test**

Run:

```bash
cd code/digimon-engine
cargo test --test ex11_074_vortexdramon -- --nocapture
```

Expected: FAIL until the card YAML exists and the harness loads it.

- [ ] **Step 3: Add minimal EX11-074 YAML for the battle pathway**

Create `code/digimon-engine/cards/ex11/EX11-074.yaml`:

```yaml
card_id: EX11-074
clauses:
  - timing: static
    grant_keyword:
      keyword: Piercing
  - timing: static
    grant_keyword:
      keyword: Vortex
  - timing: static
    grant_keyword:
      keyword: Blocker

  # Readiness slice for:
  # [All Turns][Once Per Turn] When any Digimon suspend, this Digimon may
  # unsuspend. Then, this Digimon may battle 1 of your opponent's Digimon.
  #
  # This YAML intentionally models the battle as `battle`, not `attack`.
  # It must not trigger Piercing.
  - timing: on_suspend
    once_per_turn: true
    mandatory: false
    steps:
      - optional:
          - unsuspend:
              target: this
          - select_opponent_permanent:
              prompt: "Choose 1 of your opponent's Digimon to battle"
              bind_as: battle_target
              optional: true
              filter:
                card_kind: Digimon
                zone: battle_area
          - battle:
              attacker: this
              defender: battle_target
```

If current YAML schema requires `scope`/`source` fields for static keywords, mirror the exact pattern from an existing keyword YAML such as `code/digimon-engine/cards/ex8/EX8-074.yaml`.

- [ ] **Step 4: Run EX11-074 test**

Run:

```bash
cd code/digimon-engine
cargo test --test ex11_074_vortexdramon -- --nocapture
```

Expected: PASS.

- [ ] **Step 5: Document remaining Zephagamon gaps**

In `docs/RUST_ENGINE_GAPS.md`, add:

```markdown
## Zephagamon / Vortexdramon Follow-Ups

- EX11-074 direct effect battle is supported by `Game::battle_digimon` and DSL `battle`.
- Effect battle is intentionally not an attack: it does not create `PendingAttack`, fire attack timings, open attack interrupt windows, or trigger Piercing.
- Remaining card-authoring gaps:
  - Conditional "if this effect suspended your Digimon" branches need a reusable binding predicate for "selected permanent was controlled by you and was successfully suspended by this effect".
  - BT20-101 bottom-deck count requires count of all suspended Digimon divided by 2 and capped multi-selection.
  - EX11-035 DP threshold requires a formula-based DP cap for selecting and playing a green Avian/Bird from hand.
  - EX11-062 Shoto should grant `VortexCanAttackPlayer` only while the opponent has no unsuspended Digimon.
```

In `qa/dsl-vocab-gaps.md`, add:

```markdown
## Resolved: Effect Battle Verb

- Added `battle:` DSL step for "this Digimon may battle 1 of your opponent's Digimon".
- Rule boundary: `battle:` resolves DP battle and EndOfBattle but does not declare an attack and does not trigger Piercing.
```

- [ ] **Step 6: Commit**

```bash
git add code/digimon-engine/cards/ex11/EX11-074.yaml code/digimon-engine/tests/cards/ex11_074_vortexdramon.rs docs/RUST_ENGINE_GAPS.md qa/dsl-vocab-gaps.md
git commit -m "test(cards): add ex11 vortexdramon battle readiness"
```

---

### Task 5: Full Verification

**Files:**
- No new files.

- [ ] **Step 1: Run targeted engine tests**

Run:

```bash
cd code/digimon-engine
cargo test --test effect_battle -- --nocapture
cargo test --test effect_battle_step -- --nocapture
cargo test --test mask_end_of_turn_parity -- --nocapture
cargo test --test ex11_074_vortexdramon -- --nocapture
cargo test --test piercing_security -- --nocapture
```

Expected: PASS for every test.

- [ ] **Step 2: Run broader Rust engine tests**

Run:

```bash
cd code/digimon-engine
cargo test
```

Expected: PASS. If unrelated pre-existing failures appear, capture the failing test names and confirm the new targeted tests still pass.

- [ ] **Step 3: Run DSL tests**

Run:

```bash
cd code/digimon-dsl
cargo test
```

Expected: PASS.

- [ ] **Step 4: Check git status**

Run:

```bash
git status --short
```

Expected: only the planned changes are present.

- [ ] **Step 5: Final commit if verification-only docs changed**

If docs were adjusted during verification, run:

```bash
git add docs/RUST_ENGINE_GAPS.md qa/dsl-vocab-gaps.md docs/RUST_PYTHON_PARITY.md
git commit -m "docs(engine): record zephagamon battle readiness"
```

Skip this commit if no files changed after Task 4.

---

## Self-Review

Spec coverage:
- Direct battle not attack: covered by Task 1 and Task 2 tests.
- Battle does not trigger Piercing: covered by Task 1, Task 2, and Task 4 tests.
- Legal choice surfaces through action/pending-selection contracts: covered by Task 2 DSL selection test and Task 4 EX11-074 fixture.
- Vortex decode bug: covered by Task 3.
- Base Vortex versus EX11-062 player-target extension: covered by Task 3 modifier and tests.
- Zephagamon readiness documentation: covered by Task 4.

Placeholder scan:
- No task contains placeholder instructions or unspecified "write tests" instructions.
- Every code-editing task includes concrete code or exact replacement text.

Type consistency:
- Engine primitive is consistently named `battle_digimon`.
- DSL verb is consistently named `battle`.
- Explicit Vortex player extension is consistently named `VortexCanAttackPlayer`.
