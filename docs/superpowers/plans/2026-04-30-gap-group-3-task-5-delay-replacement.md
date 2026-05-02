# Group 3 Task 5 Delay Replacement Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add BT17-097-style Delay-as-replacement support where a Delay option pays itself, parks pending hand selection, performs the follow-up action, and cancels the original deletion only after the player completes the printed choice.

**Architecture:** This plan builds on Group 3 replacement context and parked selection continuation. The implementation lowers Delay replacement DSL into native replacement process code that trashes the Delay source, prompts for a hand card, resolves the printed follow-up action, and writes the parked replacement outcome.

**Tech Stack:** Rust engine, DSL lowering, option flow regression tests, replacement dispatcher.

---

## Session Boundary

Suggested branch: `codex/group-3-task-5-delay-replacement`.

This session owns:
- `code/digimon-engine/tests/option_flow/replacement_integration.rs`
- `code/digimon-engine/src/dsl_cards/lower_replacement.rs`

This session may add helper methods in:
- `code/digimon-engine/src/effect_context/mod.rs`
- `code/digimon-engine/src/replacement.rs`

Dependency: Task 4 should provide `EffectContext::cancel_current_replacement()` and `ReplacementSubject::permanent()`. If this branch starts before Task 4 lands, add only the smallest identical helper signatures shown here and expect a merge cleanup.

---

### Task 1: Add Delay Replacement Regression

**Files:**
- Modify: `code/digimon-engine/tests/option_flow/replacement_integration.rs`

- [ ] **Step 1: Add the failing test**

Append this test to `code/digimon-engine/tests/option_flow/replacement_integration.rs`:

```rust
#[test]
fn bt17_097_delay_prevents_deletion_and_digivolves_from_hand() {
    use digimon_engine::action::space::{HAND_EFFECT_START, PASS};

    let mut r = DebugRunner::builder()
        .load_dsl_card("BT17-097")
        .add_card(make_test_card("FREE-TARGET", "Free Target"))
        .add_card(make_test_card("IMPERIAL-HAND", "Imperial Hand"))
        .hand(0, &["IMPERIAL-HAND"])
        .memory(0)
        .start();

    let target = r.place_on_field(0, "FREE-TARGET", Some(0));
    r.place_delay_option(0, "BT17-097");

    r.game
        .delete_permanent_with_cause(target, ReplacementCause::OpponentEffect);

    assert!(r.game.pending_selection.is_some(), "Delay prevention prompt is exposed");
    r.game
        .resolve_selection(0, HAND_EFFECT_START)
        .expect("select Imperialdramon-like hand target");

    assert_eq!(r.battle_area_len(0), 2, "target survived and hand card resolved");
    assert_eq!(r.trash_contains(0, "BT17-097"), true, "Delay option paid itself to trash");

    let target = r
        .find_battle_permanent(0, "FREE-TARGET")
        .expect("target still present");
    r.game
        .delete_permanent_with_cause(target, ReplacementCause::OpponentEffect);
    r.game
        .resolve_selection(0, PASS)
        .expect("decline when no Delay remains");
    assert!(r.find_battle_permanent(0, "FREE-TARGET").is_none());
}
```

- [ ] **Step 2: Verify the regression fails**

Run:

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test option_flow -- bt17_097_delay_prevents_deletion_and_digivolves_from_hand --nocapture
```

Expected: FAIL because the Delay replacement flow is not lowered or does not preserve the parked replacement until the hand selection resolves.

---

### Task 2: Add Native Helpers

**Files:**
- Modify: `code/digimon-engine/src/effect_context/mod.rs`
- Modify: `code/digimon-engine/src/replacement.rs`

- [ ] **Step 1: Add Delay cost helper**

Add this method to `impl<'a> EffectContext<'a>`:

```rust
pub fn trash_delay_source(&mut self) -> bool {
    let Some(source) = self.source_permanent else {
        return false;
    };
    self.game.delete_permanent_with_cause(source, ReplacementCause::Cost);
    true
}
```

- [ ] **Step 2: Add replacement-subject follow-up helper**

Add this method to `impl<'a> EffectContext<'a>`:

```rust
pub fn digivolve_replacement_subject_without_cost(
    &mut self,
    subject: ReplacementSubject,
    card: CardHandle,
) {
    if let Some(target) = subject.permanent() {
        self.game
            .digivolve_without_cost_from_hand(self.player, target, card);
    }
}
```

- [ ] **Step 3: Add unhandled parked replacement helper**

Add this method on `Game` in `code/digimon-engine/src/replacement.rs`:

```rust
pub fn resume_parked_replacement_unhandled(&mut self) {
    if let Some(parked) = self.parked_replacement.as_mut() {
        parked.outcome = ReplacementOutcome::None;
    }
}
```

This helper preserves the original event: for deletion, the parked replacement drain should commit the original deletion.

---

### Task 3: Lower Delay Replacement DSL

**Files:**
- Modify: `code/digimon-engine/src/dsl_cards/lower_replacement.rs`

- [ ] **Step 1: Recognize Delay replacement shape**

Lower a DSL replacement with this shape:

```yaml
replacement:
  timing: when_would_be_deleted
  active_when:
    all:
      - subject_trait: Free
      - replacement_cause: opponent_effect
  cost:
    delay_self: true
  choose:
    from: hand
    card_filter:
      trait: Imperialdramon
    min: 1
    max: 1
  outcome:
    prevent
  then:
    - digivolve_without_cost:
        target: replacement_subject
        card: chosen
```

into a replacement process equivalent to:

```rust
builder = builder.replacement_process(move |rctx| {
    if !rctx.effect.trash_delay_source() {
        return;
    }

    let subject = rctx.subject;
    rctx.effect.select_hand(
        "Choose a card for BT17-097",
        1,
        1,
        |game, card| game.card(card).has_trait("Imperialdramon"),
        move |ctx, chosen| {
            let Some(card) = chosen.first().copied() else {
                ctx.game.resume_parked_replacement_unhandled();
                return;
            };
            ctx.digivolve_replacement_subject_without_cost(subject, card);
            ctx.cancel_current_replacement();
        },
    );
});
```

- [ ] **Step 2: Decline path preserves deletion**

Ensure the hand-selection decline callback calls:

```rust
ctx.game.resume_parked_replacement_unhandled();
```

Expected behavior: if no card is selected, the target is deleted through the existing parked replacement drain.

---

### Task 4: Verify and Commit

**Files:**
- Stage all files from this plan.

- [ ] **Step 1: Run focused tests**

Run:

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test option_flow -- replacement_integration --nocapture
cargo test --manifest-path code/digimon-engine/Cargo.toml --test replacements -- nested_select --nocapture
```

Expected: PASS.

- [ ] **Step 2: Check status**

Run:

```bash
git status --short
```

Expected: only files listed in this plan are modified.

- [ ] **Step 3: Commit**

Run:

```bash
git add code/digimon-engine/src/effect_context/mod.rs code/digimon-engine/src/replacement.rs code/digimon-engine/src/dsl_cards/lower_replacement.rs code/digimon-engine/tests/option_flow/replacement_integration.rs
git commit -m "feat: add delay prevention replacement flow"
```

Expected: commit succeeds.
