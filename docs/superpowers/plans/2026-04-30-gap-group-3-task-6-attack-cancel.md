# Group 3 Task 6 Attack Cancellation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add EX10-003-style effects that pay a printed source cost through pending selection, resume the effect process, and cancel the active attack without checking security or resolving battle.

**Architecture:** This plan adds a combat-facing effect helper that marks the in-flight attack cancelled and routes through the existing attack cleanup path. The source payment uses the already-implemented pay-cost continuation behavior from Group 3 Tasks 1-2.

**Tech Stack:** Rust engine, combat state machine, native `CardEffect` test fixtures, DSL step lowering.

---

## Session Boundary

Suggested branch: `codex/group-3-task-6-attack-cancel`.

This session owns:
- `code/digimon-engine/tests/replacements/attack_cancel.rs`
- `code/digimon-engine/tests/replacements/main.rs`
- `code/digimon-engine/src/combat.rs`

This session may add helper methods in:
- `code/digimon-engine/src/effect_context/mod.rs`
- `code/digimon-engine/src/dsl_cards/lower_replacement.rs`

Dependency: Group 3 Tasks 1-2 must already be present so `.pay_cost()` resumes `.process()` after source selections.

---

### Task 1: Add Attack Cancellation Regression

**Files:**
- Create: `code/digimon-engine/tests/replacements/attack_cancel.rs`
- Modify: `code/digimon-engine/tests/replacements/main.rs`

- [ ] **Step 1: Register the test module**

Add this module declaration to `code/digimon-engine/tests/replacements/main.rs`:

```rust
mod attack_cancel;
```

- [ ] **Step 2: Create the failing test**

Create `code/digimon-engine/tests/replacements/attack_cancel.rs`:

```rust
use std::sync::{Arc, Mutex};

use digimon_engine::action::space::encode_source_select;
use digimon_engine::card_source::CardHandle;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::effect::{CardEffect, Effect};

struct TumblemonCancelAttack(Arc<Mutex<u32>>);

impl CardEffect for TumblemonCancelAttack {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        let fired = self.0.clone();
        vec![Effect::when_opponent_attacks(card)
            .name("EX10-003 Tumblemon attack cancel")
            .pay_cost(move |ctx| {
                let fired = fired.clone();
                ctx.select_own_sources(
                    "Trash 3 Mineral/Rock sources to end the attack",
                    3,
                    3,
                    |game, source_ref| {
                        game.card(source_ref.card).has_trait("Mineral")
                            || game.card(source_ref.card).has_trait("Rock")
                    },
                    move |ctx, refs| {
                        for source_ref in refs {
                            ctx.game.trash_source_ref(source_ref);
                        }
                        *fired.lock().unwrap() += 1;
                    },
                );
                true
            })
            .process(|ctx| {
                ctx.cancel_pending_attack();
            })
            .build()]
    }
}

#[test]
fn ex10_003_pay_cost_can_end_pending_attack() {
    let fired = Arc::new(Mutex::new(0));
    let mut r = DebugRunner::builder()
        .add_card(make_test_card("EX10-003", "Tumblemon"))
        .add_card(make_test_card("ATTACKER", "Attacker"))
        .add_card(make_test_card("SRC1", "Mineral Source 1"))
        .add_card(make_test_card("SRC2", "Mineral Source 2"))
        .add_card(make_test_card("SRC3", "Rock Source 3"))
        .security(0, &["SECURITY"])
        .start();
    r.register_effect("EX10-003", Arc::new(TumblemonCancelAttack(fired.clone())));

    let blocker = r.place_on_field(0, "EX10-003", Some(0));
    r.add_source(blocker, "SRC1");
    r.add_source(blocker, "SRC2");
    r.add_source(blocker, "SRC3");
    let attacker = r.place_on_field(1, "ATTACKER", Some(0));

    r.attack_player(1, attacker);

    assert!(r.game.pending_selection.is_some(), "Tumblemon pay-cost prompt is exposed");
    r.game.resolve_selection(0, encode_source_select(0, 0)).unwrap();
    r.game.resolve_selection(0, encode_source_select(0, 0)).unwrap();
    r.game.resolve_selection(0, encode_source_select(0, 0)).unwrap();

    assert_eq!(*fired.lock().unwrap(), 1);
    assert!(r.game.pending_attack.is_none(), "attack state is fully cleared");
    assert_eq!(r.security_count(0), 1, "security was not checked");
    assert_eq!(r.trash_size(0), 3, "three sources were paid");
}
```

- [ ] **Step 3: Verify the regression fails**

Run:

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test replacements -- attack_cancel --nocapture
```

Expected: FAIL because `cancel_pending_attack` does not exist or does not clear the attack after a pay-cost selection resumes.

---

### Task 2: Add Attack Cancellation Helper

**Files:**
- Modify: `code/digimon-engine/src/effect_context/mod.rs`
- Modify: `code/digimon-engine/src/combat.rs`

- [ ] **Step 1: Add effect-context method**

Add this method to `impl<'a> EffectContext<'a>`:

```rust
pub fn cancel_pending_attack(&mut self) {
    self.game.cancel_pending_attack_from_effect();
}
```

- [ ] **Step 2: Add combat implementation**

Add this method to `impl Game` in `code/digimon-engine/src/combat.rs`:

```rust
pub fn cancel_pending_attack_from_effect(&mut self) {
    if let Some(pending) = self.pending_attack.as_mut() {
        pending.cancelled = true;
    }

    if self.pending_selection.is_none() {
        let _ = self.cleanup_attack(AttackResult::Cancelled);
    }
}
```

Expected behavior: the helper does not resolve battle, does not check security, and still lets `cleanup_attack` fire end-of-attack observers and clear `pending_attack`.

- [ ] **Step 3: Verify focused regression**

Run:

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test replacements -- attack_cancel --nocapture
```

Expected: PASS.

---

### Task 3: Add DSL Step Mapping

**Files:**
- Modify: `code/digimon-engine/src/dsl_cards/lower_replacement.rs`

- [ ] **Step 1: Map `end_attack`**

Where DSL replacement/process steps are lowered, map this step:

```yaml
- end_attack: true
```

to this native process action:

```rust
ctx.cancel_pending_attack();
```

- [ ] **Step 2: Run combat-adjacent regressions**

Run:

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test replacements -- attack_cancel --nocapture
cargo test --manifest-path code/digimon-engine/Cargo.toml --test mask_and_tensor -- attack --nocapture
cargo test --manifest-path code/digimon-engine/Cargo.toml --test selection -- source --nocapture
```

Expected: PASS.

---

### Task 4: Commit

**Files:**
- Stage all files from this plan.

- [ ] **Step 1: Check status**

Run:

```bash
git status --short
```

Expected: only files listed in this plan are modified or created.

- [ ] **Step 2: Commit**

Run:

```bash
git add code/digimon-engine/src/combat.rs code/digimon-engine/src/effect_context/mod.rs code/digimon-engine/src/dsl_cards/lower_replacement.rs code/digimon-engine/tests/replacements/main.rs code/digimon-engine/tests/replacements/attack_cancel.rs
git commit -m "feat: allow effects to cancel pending attacks"
```

Expected: commit succeeds.
