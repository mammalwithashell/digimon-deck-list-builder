# Group 4 Test Review Fixes Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Tighten the group 4 zone-movement regression tests so they prove the exact restore, trigger-order, and no-duplication behavior they claim to cover.

**Architecture:** This is a test-only cleanup. Production engine and DSL behavior should not change; each task strengthens assertions in the existing group 4 tests and reruns the smallest relevant `cargo test` target before the final full engine suite.

**Tech Stack:** Rust integration tests under `code/digimon-engine/tests/`, `DebugRunner`, `EffectContext`, and `cargo test --manifest-path code/digimon-engine/Cargo.toml`.

---

## File Structure

- Modify `code/digimon-engine/tests/effect_context/effect_digivolve_from_zones.rs`
  - Strengthen source restoration coverage to force a post-take restore path.
  - Add explicit trigger-order recording for security-origin effect digivolve.
- Modify `code/digimon-engine/tests/effect_context/security_stack_operations.rs`
  - Tighten final zone assertions in the nested direct security removal test.
- No production files should change.
- No docs need updates beyond this plan.

---

### Task 1: Make The Restore Regression Exercise A Post-Take Failure

**Files:**
- Modify: `code/digimon-engine/tests/effect_context/effect_digivolve_from_zones.rs`
- Test: `code/digimon-engine/tests/effect_context/effect_digivolve_from_zones.rs`

- [ ] **Step 1: Replace the weak restore test**

Replace the current `failed_effect_digivolve_restores_source_zone` test with this version. It uses a valid target and a valid trash source, then fails after the source is taken by making the memory cost unaffordable.

```rust
#[test]
fn failed_effect_digivolve_after_take_restores_source_zone() {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("BASE3", "Base"))
        .add_card(evo_lv4("EVO4"))
        .add_card(make_test_card("OTHER", "Other"))
        .memory(-9)
        .start();

    let target = runner.place_on_field(0, "BASE3", None);
    let evo_handle = add_to_trash(&mut runner, 0, "EVO4");
    let other_handle = add_to_trash(&mut runner, 0, "OTHER");
    let source_card = runner.game.players[0].battle_area[target.index as usize]
        .top_card()
        .handle();

    let ok = {
        let mut ctx = EffectContext::new(&mut runner.game, source_card, Some(target), 0);
        ctx.effect_initiated_digivolve_from_source(
            0,
            CardSourceRef::Trash(0, 0),
            target,
            CostDelta::Fixed(2),
            false,
        )
    };

    assert!(!ok);
    assert_eq!(runner.game.memory, -9, "failed payment must not move memory");
    assert_eq!(
        runner.game.players[0].battle_area[target.index as usize]
            .top_card()
            .card_id(&runner.game.card_data),
        "BASE3",
        "target should not digivolve after the post-take failure"
    );
    let trash_handles: Vec<_> = runner.game.players[0]
        .trash
        .iter()
        .map(|c| c.handle())
        .collect();
    assert_eq!(
        trash_handles,
        vec![evo_handle, other_handle],
        "taken source must be restored at its original trash index"
    );
}
```

- [ ] **Step 2: Run the focused test**

Run:

```powershell
cargo test --manifest-path code/digimon-engine/Cargo.toml --test effect_context -- effect_digivolve_from_zones::failed_effect_digivolve_after_take_restores_source_zone --nocapture
```

Expected: PASS. If it fails because `DebugRunner::builder().memory(-9)` does not accept negative values, keep the same test but set `.memory(0)` and use `CostDelta::Fixed(20)`.

- [ ] **Step 3: Commit after Task 1 if working independently**

```powershell
git add code/digimon-engine/tests/effect_context/effect_digivolve_from_zones.rs
git commit -m "test: cover post-take effect digivolve restore"
```

---

### Task 2: Prove Security Loss Resolves Before Digivolve Triggers

**Files:**
- Modify: `code/digimon-engine/tests/effect_context/effect_digivolve_from_zones.rs`
- Test: `code/digimon-engine/tests/effect_context/effect_digivolve_from_zones.rs`

- [ ] **Step 1: Update imports**

Change:

```rust
use std::sync::Arc;
```

to:

```rust
use std::sync::{Arc, Mutex};
```

- [ ] **Step 2: Replace the effect helper with an order-recording helper**

Replace `SecurityLossAndDigivolveGain` with this version.

```rust
struct SecurityLossAndDigivolveOrder {
    seen: Arc<Mutex<Vec<&'static str>>>,
}

impl CardEffect for SecurityLossAndDigivolveOrder {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        let lose_seen = self.seen.clone();
        let digivolve_seen = self.seen.clone();
        vec![
            Effect::on_lose_security(card)
                .name("record lose security")
                .process(move |_ctx| {
                    lose_seen.lock().unwrap().push("lose_security");
                })
                .build(),
            Effect::when_digivolving(card)
                .name("record when digivolving")
                .process(move |_ctx| {
                    digivolve_seen.lock().unwrap().push("when_digivolving");
                })
                .build(),
        ]
    }
}
```

- [ ] **Step 3: Replace the trigger-order test body**

Replace `effect_digivolve_from_security_fires_security_loss_before_digivolve_triggers` with this version.

```rust
#[test]
fn effect_digivolve_from_security_fires_security_loss_before_digivolve_triggers() {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("BASE3", "Base"))
        .add_card(evo_lv4("EVO4"))
        .security(0, &["EVO4"])
        .memory(5)
        .start();
    let seen = Arc::new(Mutex::new(Vec::new()));
    runner.register_effect(
        "EVO4",
        Arc::new(SecurityLossAndDigivolveOrder { seen: seen.clone() }),
    );

    let target = runner.place_on_field(0, "BASE3", None);
    let evo_handle = runner.game.players[0].security[0].handle();
    let source_card = runner.game.players[0].battle_area[target.index as usize]
        .top_card()
        .handle();

    let ok = {
        let mut ctx = EffectContext::new(&mut runner.game, source_card, Some(target), 0);
        ctx.effect_initiated_digivolve_from_source(
            0,
            CardSourceRef::Security(0, 0),
            target,
            CostDelta::Free,
            false,
        )
    };

    assert!(ok);
    assert_eq!(runner.game.players[0].security.len(), 0);
    assert_eq!(
        runner.game.players[0].battle_area[target.index as usize]
            .top_card()
            .handle(),
        evo_handle
    );
    assert_eq!(
        seen.lock().unwrap().as_slice(),
        ["lose_security", "when_digivolving"],
        "security-loss observer must finish before the final digivolve trigger dispatch"
    );
}
```

- [ ] **Step 4: Run the focused test**

Run:

```powershell
cargo test --manifest-path code/digimon-engine/Cargo.toml --test effect_context -- effect_digivolve_from_zones::effect_digivolve_from_security_fires_security_loss_before_digivolve_triggers --nocapture
```

Expected: PASS and the assertion compares exact sequence labels, not the final memory total.

- [ ] **Step 5: Commit after Task 2 if working independently**

```powershell
git add code/digimon-engine/tests/effect_context/effect_digivolve_from_zones.rs
git commit -m "test: assert security digivolve trigger order"
```

---

### Task 3: Tighten Nested Security Removal Zone Assertions

**Files:**
- Modify: `code/digimon-engine/tests/effect_context/security_stack_operations.rs`
- Test: `code/digimon-engine/tests/effect_context/security_stack_operations.rs`

- [ ] **Step 1: Add a tiny zone-id helper near the existing helper functions**

Place this after `add_card_to_security_owned`.

```rust
fn ids_in_zone(cards: &[CardSource], card_data: &[digimon_engine::card_data::CardData]) -> Vec<String> {
    cards
        .iter()
        .map(|card| card.card_id(card_data).to_string())
        .collect()
}
```

If `cargo fmt` wraps the signature, accept the formatter output.

- [ ] **Step 2: Replace the final assertions in the nested test**

In `nested_deferred_security_removals_resume_outer_after_inner_selection`, replace the assertions from `assert_eq!(runner.game.players[0].security.len(), 0);` through the end of the test with this exact block.

```rust
    let hand_ids = ids_in_zone(&runner.game.players[0].hand, &runner.game.card_data);
    let trash_ids = ids_in_zone(&runner.game.players[0].trash, &runner.game.card_data);
    let security_ids = ids_in_zone(&runner.game.players[0].security, &runner.game.card_data);

    assert_eq!(security_ids, Vec::<String>::new());
    assert_eq!(
        hand_ids,
        vec![
            "CHOICE-A".to_string(),
            "CHOICE-B".to_string(),
            "INNER".to_string(),
        ],
        "INNER should appear only in hand alongside the untouched choice cards"
    );
    assert_eq!(
        trash_ids,
        vec!["OUTER".to_string()],
        "OUTER should appear only in trash after the outer continuation resumes"
    );

    let battle_ids: Vec<String> = runner.game.players[0]
        .battle_area
        .iter()
        .flat_map(|perm| {
            perm.card_sources
                .iter()
                .map(|card| card.card_id(&runner.game.card_data).to_string())
        })
        .collect();
    assert!(
        !battle_ids.iter().any(|id| id == "INNER" || id == "OUTER"),
        "removed security cards must not be duplicated into battle-area stacks"
    );
```

- [ ] **Step 3: Run the focused nested test**

Run:

```powershell
cargo test --manifest-path code/digimon-engine/Cargo.toml --test effect_context -- security_stack_operations::nested_deferred_security_removals_resume_outer_after_inner_selection --nocapture
```

Expected: PASS. If hand order differs, do not loosen the assertion to counts only. Instead sort a cloned `hand_ids` vector before comparison and compare to sorted expected IDs, while keeping exact trash/security/battle assertions.

- [ ] **Step 4: Commit after Task 3 if working independently**

```powershell
git add code/digimon-engine/tests/effect_context/security_stack_operations.rs
git commit -m "test: tighten nested security removal assertions"
```

---

### Task 4: Final Verification And Packaging

**Files:**
- Verify: `code/digimon-engine/tests/effect_context/effect_digivolve_from_zones.rs`
- Verify: `code/digimon-engine/tests/effect_context/security_stack_operations.rs`

- [ ] **Step 1: Format**

Run:

```powershell
cargo fmt --manifest-path code/digimon-engine/Cargo.toml
```

Expected: exit code 0.

- [ ] **Step 2: Run all touched effect-context tests**

Run:

```powershell
cargo test --manifest-path code/digimon-engine/Cargo.toml --test effect_context -- effect_digivolve_from_zones security_stack_operations --nocapture
```

Expected: PASS. Confirm the output shows all tests in both modules pass and no failures.

- [ ] **Step 3: Run the full engine suite**

Run:

```powershell
cargo test --manifest-path code/digimon-engine/Cargo.toml
```

Expected: PASS. Existing warnings and ignored tests are acceptable only if there are zero failures.

- [ ] **Step 4: Inspect status and keep unrelated files unstaged**

Run:

```powershell
git status --short
```

Expected: only the two touched test files are modified, plus any pre-existing unrelated files such as:

```text
 M code/src-tauri/gen/schemas/desktop-schema.json
 M code/src-tauri/gen/schemas/windows-schema.json
```

Do not stage unrelated Tauri schema files.

- [ ] **Step 5: Commit final combined fix if Tasks 1-3 were not committed separately**

Run:

```powershell
git add code/digimon-engine/tests/effect_context/effect_digivolve_from_zones.rs code/digimon-engine/tests/effect_context/security_stack_operations.rs
git commit -m "test: tighten group 4 zone movement regressions"
```

Expected: commit succeeds with only test-file changes.

---

## Self-Review

- Spec coverage: Finding 1 is covered by Task 1; Finding 2 is covered by Task 2; Finding 3 is covered by Task 3.
- Placeholder scan: No `TBD`, `TODO`, or vague “write tests” instructions remain. Each code-edit step includes exact replacement code.
- Type consistency: The plan uses existing `CardSourceRef`, `CostDelta`, `DebugRunner`, `EffectContext`, `CardEffect`, `Effect`, `CardHandle`, and `CardSource` names already present in the touched test files. The new helper references `digimon_engine::card_data::CardData` by fully qualified path, so no extra import is required.
