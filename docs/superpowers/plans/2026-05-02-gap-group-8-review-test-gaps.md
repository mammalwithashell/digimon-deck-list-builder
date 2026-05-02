# Gap Group 8 Review Test Gaps Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Close the three review findings by strengthening targeted Rust engine tests for ACE overflow movement routes, DigiXros alias runtime bridging, and BT23-014 When Attacking deletion assertions.

**Architecture:** This is test-only coverage. Reuse existing `DebugRunner` helpers and public engine test APIs, avoid production code changes unless a new test exposes a real behavioral bug, and keep each assertion next to the existing test file it strengthens.

**Tech Stack:** Rust, Cargo integration tests under `code/digimon-engine/tests`, default `dsl-yaml-loader` feature, `DebugRunner`, YAML DSL fixtures.

---

## File Structure

Files to modify:

- `code/digimon-engine/tests/ace_overflow.rs`
- `code/digimon-engine/tests/dsl/digixros_aliases.rs`
- `code/digimon-engine/tests/cards_behavioral/bt23/bt23_014.rs`

Validation commands from repo root:

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test ace_overflow
cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- digixros_aliases
cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- bt23_014_when_attacking_with_target_reduces_opp_field
cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- bt23_014
```

---

## Task 1: ACE Overflow Non-Trash Leave Routes

**Finding:** `ace_overflow.rs` proves deletion of a top ACE card and direct source trashing, but not whole-stack return routes where `return_to_hand` / `return_to_deck` apply ACE overflow to the top card and all sources leaving with the stack.

**Files:**

- Modify: `code/digimon-engine/tests/ace_overflow.rs`

- [ ] **Step 1: Add the missing import**

Add `StackPosition` beside the existing imports:

```rust
use digimon_engine::enums::StackPosition;
```

- [ ] **Step 2: Add a tiny ACE fixture helper**

Place this after the imports:

```rust
fn ace_card(card_id: &str, card_name: &str) -> digimon_engine::CardData {
    let mut ace = make_test_card(card_id, card_name);
    ace.ace_overflow = Some(-4);
    ace
}
```

- [ ] **Step 3: Convert existing duplicated ACE setup to the helper**

In both existing tests, replace:

```rust
let mut ace = make_test_card("ACE-RUNTIME", "Ace Runtime");
ace.ace_overflow = Some(-4);
```

with:

```rust
let ace = ace_card("ACE-RUNTIME", "Ace Runtime");
```

Do the same for `"ACE-SOURCE"`.

- [ ] **Step 4: Add top-card return-to-hand coverage**

Append this test to `ace_overflow.rs`:

```rust
#[test]
fn ace_overflow_loses_memory_when_top_card_returns_to_hand() {
    let mut runner = DebugRunner::builder()
        .add_card(ace_card("ACE-HAND", "Ace Hand"))
        .memory(3)
        .start();

    let handle = runner.place_on_field(0, "ACE-HAND", Some(0));

    assert!(
        runner.game.return_to_hand(handle).is_some(),
        "ACE top card should return to hand"
    );
    assert_eq!(runner.game.memory, -1);
}
```

- [ ] **Step 5: Add top-card return-to-deck coverage**

Append this test:

```rust
#[test]
fn ace_overflow_loses_memory_when_top_card_returns_to_deck() {
    let mut runner = DebugRunner::builder()
        .add_card(ace_card("ACE-DECK", "Ace Deck"))
        .memory(3)
        .start();

    let handle = runner.place_on_field(0, "ACE-DECK", Some(0));

    assert!(
        runner.game.return_to_deck(handle, StackPosition::Bottom),
        "ACE top card should return to deck"
    );
    assert_eq!(runner.game.memory, -1);
}
```

- [ ] **Step 6: Add ACE source coverage through return-to-hand**

Append this test:

```rust
#[test]
fn ace_overflow_loses_memory_when_source_leaves_via_return_to_hand_stack_cleanup() {
    let mut runner = DebugRunner::builder()
        .add_card(ace_card("ACE-SOURCE-HAND", "Ace Source Hand"))
        .add_card(make_test_card("TOP-HAND", "Top Hand"))
        .memory(3)
        .start();

    let stack = runner.place_stack(0, &["ACE-SOURCE-HAND", "TOP-HAND"]);

    assert!(
        runner.game.return_to_hand(stack).is_some(),
        "top card should return to hand and sources should leave the stack"
    );
    assert_eq!(runner.game.memory, -1);
}
```

- [ ] **Step 7: Add ACE source coverage through return-to-deck**

Append this test:

```rust
#[test]
fn ace_overflow_loses_memory_when_source_leaves_via_return_to_deck_stack_cleanup() {
    let mut runner = DebugRunner::builder()
        .add_card(ace_card("ACE-SOURCE-DECK", "Ace Source Deck"))
        .add_card(make_test_card("TOP-DECK", "Top Deck"))
        .memory(3)
        .start();

    let stack = runner.place_stack(0, &["ACE-SOURCE-DECK", "TOP-DECK"]);

    assert!(
        runner.game.return_to_deck(stack, StackPosition::Bottom),
        "top card should return to deck and sources should leave the stack"
    );
    assert_eq!(runner.game.memory, -1);
}
```

- [ ] **Step 8: Run the focused test**

Run:

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test ace_overflow
```

Expected: all ACE overflow tests pass.

---

## Task 2: DigiXros Alias Runtime Bridge Coverage

**Finding:** Existing tests prove YAML compile output and manual `CardData` matching separately, but not the bridge that turns a compiled DSL card into runtime `CardData`.

**Files:**

- Modify: `code/digimon-engine/tests/dsl/digixros_aliases.rs`

- [ ] **Step 1: Import DebugRunner**

Add:

```rust
use digimon_engine::debug_runner::DebugRunner;
```

- [ ] **Step 2: Add a runtime bridge test**

Append this test:

```rust
#[test]
fn digixros_aliases_flow_from_dsl_yaml_into_runtime_card_data() {
    let yaml = r#"
card: XROS-BRIDGE
name: Bridge Alias Carrier
kind: digimon
level: 4
color: [red]
cost: 5
dp: 4000
digixros_aliases: ["Shoutmon"]
alt_paths:
  - kind: digixros
    materials:
      - filter: { name_contains: "Shoutmon" }
        repeat: { min: 1, max: 1 }
    cost: 3
"#;

    let runner = DebugRunner::builder()
        .from_dsl_yaml(yaml)
        .expect("inline DigiXros alias DSL should compile")
        .start();

    let material = runner
        .game
        .card_data
        .iter()
        .find(|card| card.card_id == "XROS-BRIDGE")
        .expect("runtime CardData should include DSL card");

    assert_eq!(material.digixros_aliases, vec!["Shoutmon"]);
    assert!(
        digimon_engine::digixros::matches_digixros_name_requirement_for_test(
            material,
            "Shoutmon",
        ),
        "DigiXros recipe matching must see aliases copied by the DSL bridge"
    );
    assert!(
        !digimon_engine::digixros::matches_generic_name_requirement_for_test(
            material,
            "Shoutmon",
        ),
        "generic name predicates must still ignore scoped DigiXros aliases"
    );
}
```

- [ ] **Step 3: Run the focused DSL test filter**

Run:

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- digixros_aliases
```

Expected: both DigiXros alias tests pass.

---

## Task 3: Strengthen BT23-014 When Attacking Deletion Assertion

**Finding:** `bt23_014_when_attacking_with_target_reduces_opp_field` says an eligible target should be deleted, but `<=` allows no deletion.

**Files:**

- Modify: `code/digimon-engine/tests/cards_behavioral/bt23/bt23_014.rs`

- [ ] **Step 1: Tighten the assertion**

Replace the assertion at the end of `bt23_014_when_attacking_with_target_reduces_opp_field`:

```rust
assert!(
    opp_count_after <= opp_count_before,
    "After [When Attacking] with eligible target, opp count should not increase; \
     before={}, after={}",
    opp_count_before,
    opp_count_after
);
```

with:

```rust
assert!(
    opp_count_after < opp_count_before,
    "After [When Attacking] delete fires and auto-resolves, opp battle area should shrink; \
     before={}, after={}",
    opp_count_before,
    opp_count_after
);
```

- [ ] **Step 2: Keep the low-DP target fixture**

Leave this setup as-is:

```rust
let _target = runner.place_on_field(1, LOW_DP_TARGET, None);
```

The low-DP target is what makes the strict shrink assertion deterministic.

- [ ] **Step 3: Run the focused test**

Run:

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- bt23_014_when_attacking_with_target_reduces_opp_field
```

Expected: the tightened test passes.

- [ ] **Step 4: Run the BT23-014 behavioral slice**

Run:

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- bt23_014
```

Expected: all BT23-014 tests pass.

---

## Task 4: Final Verification

- [ ] Run all focused commands:

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test ace_overflow
cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- digixros_aliases
cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- bt23_014
```

- [ ] Run the broader changed-test binaries:

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl
cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral
```

- [ ] If a newly strict test exposes missing engine behavior, stop and use `superpowers:systematic-debugging` before modifying production code.

---

## Self-Review

Spec coverage:

- Finding 1 is covered by Task 1 with top-card and source-in-stack return-to-hand/deck paths.
- Finding 2 is covered by Task 2 with `DebugRunner::builder().from_dsl_yaml(...)` runtime `CardData`.
- Finding 3 is covered by Task 3 with a strict shrink assertion.

Placeholder scan:

- No TBD/TODO placeholders.
- All code edits include exact snippets.
- Validation commands are exact.

Type consistency:

- `StackPosition`, `DebugRunner`, `make_test_card`, `return_to_hand`, `return_to_deck`, `place_stack`, and DigiXros test helpers all match existing repo APIs inspected before writing the plan.
