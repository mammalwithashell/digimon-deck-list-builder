# DSL Option Use Requirement and Field/DP Primitives Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add first-class engine and DSL primitives for Option Use Requirements, field-wide permanent predicates, source-kind effect immunity, and lowest/highest-DP selection so real DUAL cards such as ST23-09 can be expressed without hand-written card logic.

**Architecture:** Keep reusable rule concepts in the engine/DSL boundary instead of card-specific Rust. Use Req becomes an Option-use legality bypass for color requirements, backed by a compiled predicate. Field predicates get a breeding-aware subject path. DP extrema are applied when installing the pending selection so effects like "suspend, then bottom-deck highest suspended" see the current board state.

**Tech Stack:** Rust engine (`code/digimon-engine`), Rust DSL crate (`code/digimon-dsl`), YAML card scripts, `cargo test`.

---

## File Structure

- `code/digimon-dsl/src/predicate.rs`: add authored `any_field_permanent`.
- `code/digimon-dsl/src/compiled.rs`: add compiled `any_field_permanent`; add `CompiledFieldSelector`; add selector fields to permanent-selection steps.
- `code/digimon-dsl/src/compile.rs`: lower new predicate and selection fields.
- `code/digimon-dsl/src/spec.rs`: add `use_requirement` to normal Option specs and DUAL Option face metadata.
- `code/digimon-dsl/src/step.rs`: add authored `selector` to `SelectFieldArgs`; add `grant_effect_immunity` step.
- `code/digimon-engine/src/dsl_cards/predicate.rs`: evaluate `any_field_permanent` across battle area plus breeding area, including Digi-Eggs in breeding.
- `code/digimon-engine/src/dsl_cards/step/selections.rs`: narrow pending permanent selections to lowest/highest DP among currently valid candidates.
- `code/digimon-engine/src/dsl_cards/step/modifiers.rs`: lower `grant_effect_immunity`.
- `code/digimon-engine/src/effect.rs`: add an Option color-requirement bypass condition to `Effect`.
- `code/digimon-engine/src/dsl_cards/mod.rs`: attach DSL Use Req predicates to OptionMain effects.
- `code/digimon-engine/src/action/mask.rs`: use a shared Option-use legality helper instead of color-only checks.
- `code/digimon-engine/src/game_actions.rs`: use the same helper in `play_option_core`.
- `code/digimon-engine/src/effect_context/mod.rs`: add a small helper for installing source-kind immunity modifiers from DSL.
- `code/digimon-engine/tests/dsl/option_use_req.rs`: new behavior tests for Use Req and `any_field_permanent`.
- `code/digimon-engine/tests/dsl/selection_dp_extrema.rs`: new behavior tests for lowest/highest-DP selection narrowing.
- `code/digimon-engine/tests/dsl/effect_immunity_step.rs`: new behavior tests for DSL-granted source-kind immunity.
- `docs/examples/ST23-09-dual-card.yaml`: update the example to use the new primitives.
- `docs/RUST_ENGINE_API.md`: document Use Req, field predicates, DP selectors, and `grant_effect_immunity`.
- `docs/RULES_CONTEXT.md`: add the Use Req rule note, including breeding-area Digi-Eggs.

---

### Task 1: Parse And Compile `any_field_permanent`

**Files:**
- Modify: `code/digimon-dsl/src/predicate.rs`
- Modify: `code/digimon-dsl/src/compiled.rs`
- Modify: `code/digimon-dsl/src/compile.rs`
- Test: `code/digimon-engine/tests/dsl/option_use_req.rs`
- Modify: `code/digimon-engine/tests/dsl/main.rs`

- [ ] **Step 1: Add the failing parser/compile test**

Create `code/digimon-engine/tests/dsl/option_use_req.rs`:

```rust
use digimon_dsl::compiled::CompiledCardKind;

#[test]
fn parses_and_compiles_any_field_permanent() {
    let yaml = r#"
card: TST-USE-REQ
name: Use Req Test
kind: option
color: [green]
cost: 5
use_requirement:
  any_field_permanent:
    of: you
    any_of:
      - kind: digimon
      - kind: tamer
    trait_has: BEATBREAK
effects:
  - when: main_from_hand
    process:
      - draw: 1
"#;

    let spec: digimon_dsl::CardSpec =
        serde_yml::from_str(yaml).expect("parse use requirement yaml");
    let compiled = digimon_dsl::compile::compile(&spec).expect("compile use requirement yaml");
    let use_req = compiled
        .use_requirement
        .as_ref()
        .expect("compiled option use requirement");
    let ex = use_req
        .any_field_permanent
        .as_ref()
        .expect("compiled any_field_permanent");

    assert!(ex.predicate.any_of.iter().any(|p| p.kind == Some(CompiledCardKind::Digimon)));
    assert!(ex.predicate.any_of.iter().any(|p| p.kind == Some(CompiledCardKind::Tamer)));
    assert_eq!(ex.predicate.trait_has.as_deref(), Some("BEATBREAK"));
}
```

Add the module to `code/digimon-engine/tests/dsl/main.rs`:

```rust
mod option_use_req;
```

- [ ] **Step 2: Run the focused test and confirm it fails**

Run:

```bash
cargo test --test dsl parses_and_compiles_any_field_permanent -- --nocapture
```

Expected: fail because `CardSpec.use_requirement`, `PredicateSpec.any_field_permanent`, and `CompiledPredicate.any_field_permanent` do not exist yet.

- [ ] **Step 3: Add authored and compiled fields**

In `code/digimon-dsl/src/predicate.rs`, add this beside the existing existential fields:

```rust
    #[serde(skip_serializing_if = "Option::is_none")]
    pub any_field_permanent: Option<Box<ExistentialPredicate>>,
```

In `code/digimon-dsl/src/compiled.rs`, add this beside `any_permanent`:

```rust
    pub any_field_permanent: Option<Box<CompiledExistential>>,
```

In `code/digimon-dsl/src/spec.rs`, add this to `CardSpec` near `dual`:

```rust
    /// Optional Option-use requirement that can satisfy color requirements.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub use_requirement: Option<crate::predicate::PredicateSpec>,
```

Add the same shape to `DualOptionSpec`:

```rust
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub use_requirement: Option<crate::predicate::PredicateSpec>,
```

In `code/digimon-dsl/src/compiled.rs`, add to `CompiledCard`:

```rust
    pub use_requirement: Option<CompiledPredicate>,
```

Add to `CompiledDualOption`:

```rust
    pub use_requirement: Option<Box<CompiledPredicate>>,
```

- [ ] **Step 4: Update existing compiled-card fixture literals**

Adding `CompiledCard.use_requirement` requires every direct `CompiledCard { ... }` test fixture to state the default. In each existing literal, add this field next to `dual: None`:

```rust
        use_requirement: None,
```

Apply that edit to the direct compiled fixtures in:

```text
code/digimon-engine/tests/cards_behavioral/bt17/bt17_018.rs
code/digimon-engine/tests/cards_behavioral/bt20/bt20_016.rs
code/digimon-engine/tests/cards_behavioral/bt24/bt24_017.rs
code/digimon-engine/tests/cards_behavioral/ex11/ex11_012.rs
code/digimon-engine/tests/cards_behavioral/ex11/ex11_054.rs
code/digimon-engine/tests/cards_behavioral/ex8/ex8_074.rs
code/digimon-engine/tests/cards_behavioral/ex9/ex9_013.rs
code/digimon-engine/tests/dsl/delay.rs
code/digimon-engine/tests/dsl/partition.rs
code/digimon-engine/tests/dsl/phase1c_lowering.rs
code/digimon-engine/tests/dsl/phase1c_scaffold.rs
code/digimon-engine/tests/dsl/phase2a_triggered.rs
code/digimon-engine/tests/dsl/phase3_reducer_replacement.rs
code/digimon-engine/tests/dsl/phase4_raw_rust_clause.rs
code/digimon-engine/tests/dsl/replacement.rs
code/digimon-engine/tests/dsl/standalone_declaratives_exit.rs
code/digimon-engine/tests/support/dsl_card_data.rs
code/digimon-dsl/src/compiled.rs
code/digimon-dsl/src/pack.rs
```

Run this check afterward:

```bash
cargo test --test dsl parses_and_compiles_any_field_permanent -- --nocapture
```

Expected: compilation reaches the new `use_requirement` parser failure instead of failing on missing `CompiledCard.use_requirement` fields.

- [ ] **Step 5: Compile the new fields**

In `code/digimon-dsl/src/compile.rs`, include `use_requirement` when building `CompiledCard`:

```rust
        use_requirement: spec.use_requirement.as_ref().map(|p| {
            compile_predicate(p, "use_requirement", &spec.card, &mut errors)
        }),
```

In `compile_dual`, set:

```rust
            use_requirement: dual.option.use_requirement.as_ref().map(|p| {
                Box::new(compile_predicate(
                    p,
                    "dual.option.use_requirement",
                    "<dual-option>",
                    &mut Vec::new(),
                ))
            }),
```

Replace that temporary-error shape with a local helper that receives `card_id` and `errors`:

```rust
fn compile_dual(
    dual: &crate::spec::DualSpec,
    card_id: &str,
    errors: &mut Vec<ValidationError>,
) -> CompiledDual {
    CompiledDual {
        digimon: CompiledDualDigimon {
            level: dual.digimon.level,
            dp: dual.digimon.dp,
            colors: dual.digimon.colors.iter().map(|c| compile_color(*c)).collect(),
            traits: dual.digimon.traits.clone(),
            effect_text: dual.digimon.effect_text.clone(),
            inherited_text: dual.digimon.inherited_text.clone(),
        },
        option: CompiledDualOption {
            use_cost: dual.option.use_cost,
            colors: dual.option.colors.iter().map(|c| compile_color(*c)).collect(),
            effect_text: dual.option.effect_text.clone(),
            security_text: dual.option.security_text.clone(),
            keywords: dual.option.keywords.clone(),
            use_requirement: dual.option.use_requirement.as_ref().map(|p| {
                Box::new(compile_predicate(
                    p,
                    "dual.option.use_requirement",
                    card_id,
                    errors,
                ))
            }),
        },
    }
}
```

Then update the caller near the top of `compile`:

```rust
    let dual = spec
        .dual
        .as_ref()
        .map(|dual| compile_dual(dual, &spec.card, &mut errors));
```

In `compile_predicate`, add:

```rust
        any_field_permanent: p.any_field_permanent.as_ref().map(|e| {
            Box::new(CompiledExistential {
                of: compile_player_ref(e.of),
                predicate: compile_predicate(
                    &e.predicate,
                    &format!("{prefix}.any_field_permanent"),
                    card_id,
                    errors,
                ),
            })
        }),
```

- [ ] **Step 6: Run the focused parser/compile test**

Run:

```bash
cargo test --test dsl parses_and_compiles_any_field_permanent -- --nocapture
```

Expected: pass.

- [ ] **Step 7: Commit this task**

```bash
git add code/digimon-dsl/src/predicate.rs code/digimon-dsl/src/compiled.rs code/digimon-dsl/src/compile.rs code/digimon-dsl/src/spec.rs code/digimon-dsl/src/pack.rs code/digimon-engine/tests/dsl/main.rs code/digimon-engine/tests/dsl/option_use_req.rs code/digimon-engine/tests/cards_behavioral code/digimon-engine/tests/dsl code/digimon-engine/tests/support/dsl_card_data.rs
git commit -m "feat(dsl): parse option use field predicates"
```

---

### Task 2: Evaluate `any_field_permanent` Across Battle And Breeding

**Files:**
- Modify: `code/digimon-engine/src/dsl_cards/predicate.rs`
- Test: `code/digimon-engine/tests/dsl/option_use_req.rs`

- [ ] **Step 1: Add failing runtime predicate tests**

Append to `code/digimon-engine/tests/dsl/option_use_req.rs`:

```rust
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::{CardData, CardKind, EffectReadContext};

fn beatbreak_egg(id: &str) -> CardData {
    let mut card = make_test_card(id, "Beatbreak Egg");
    card.card_kind = CardKind::DigiEgg;
    card.level = Some(2);
    card.traits = vec!["BEATBREAK".to_string()];
    card
}

fn off_trait_lv3(id: &str) -> CardData {
    let mut card = make_test_card(id, "Plain Rookie");
    card.card_kind = CardKind::Digimon;
    card.level = Some(3);
    card.traits = vec!["Plain".to_string()];
    card
}

fn beatbreak_lv3(id: &str) -> CardData {
    let mut card = make_test_card(id, "Beatbreak Rookie");
    card.card_kind = CardKind::Digimon;
    card.level = Some(3);
    card.traits = vec!["BEATBREAK".to_string()];
    card
}

#[test]
fn any_field_permanent_sees_traited_egg_in_breeding() {
    let yaml = r#"
card: TST-PRED-SOURCE
name: Predicate Source
kind: option
color: [green]
cost: 5
use_requirement:
  any_field_permanent:
    of: you
    any_of:
      - kind: digimon
      - kind: tamer
    trait_has: BEATBREAK
effects:
  - when: main_from_hand
    process:
      - draw: 1
"#;
    let spec: digimon_dsl::CardSpec =
        serde_yml::from_str(yaml).expect("parse predicate carrier card");
    let compiled = digimon_dsl::compile::compile(&spec).expect("compile predicate carrier card");
    let use_req = compiled.use_requirement.as_ref().expect("compiled use requirement");

    let mut runner = DebugRunner::builder()
        .add_card(beatbreak_egg("EGG-BEAT"))
        .add_card(off_trait_lv3("PLAIN-LV3"))
        .digitama(0, &["EGG-BEAT"])
        .start();
    assert!(runner.game.hatch(0), "egg should move into breeding");
    let source = runner
        .game
        .players[0]
        .breeding_area
        .as_ref()
        .expect("breeding permanent")
        .top_card()
        .handle();
    let ctx = EffectReadContext::new(&runner.game, source, None, 0);

    assert!(digimon_engine::dsl_cards::predicate::eval_predicate(
        use_req,
        &ctx,
        digimon_engine::dsl_cards::predicate::PredicateSubject::None,
    ));
}

#[test]
fn any_field_permanent_sees_traited_breeding_stack() {
    let yaml = r#"
card: TST-PRED-SOURCE
name: Predicate Source
kind: option
color: [green]
cost: 5
use_requirement:
  any_field_permanent:
    of: you
    any_of:
      - kind: digimon
      - kind: tamer
    trait_has: BEATBREAK
effects:
  - when: main_from_hand
    process:
      - draw: 1
"#;
    let spec: digimon_dsl::CardSpec =
        serde_yml::from_str(yaml).expect("parse predicate carrier card");
    let compiled = digimon_dsl::compile::compile(&spec).expect("compile predicate carrier card");
    let use_req = compiled.use_requirement.as_ref().expect("compiled use requirement");

    let mut evo = beatbreak_lv3("LV3-BEAT");
    evo.evo_costs = vec![digimon_engine::card_data::EvoCost {
        card_color: 0,
        level: 2,
        memory_cost: 0,
    }];
    let mut runner = DebugRunner::builder()
        .add_card(beatbreak_egg("EGG-BEAT"))
        .add_card(evo)
        .digitama(0, &["EGG-BEAT"])
        .hand(0, &["LV3-BEAT"])
        .deck(0, &["LV3-BEAT"])
        .memory(10)
        .start();
    assert!(runner.game.hatch(0), "egg should move into breeding");
    assert!(
        runner.game.digivolve_from_hand_onto_breeding(0, 0, digimon_engine::PlaySource::ByDigivolve),
        "LV3 should digivolve onto the egg in breeding"
    );
    let source = runner
        .game
        .players[0]
        .breeding_area
        .as_ref()
        .expect("breeding stack")
        .top_card()
        .handle();
    let ctx = EffectReadContext::new(&runner.game, source, None, 0);

    assert!(digimon_engine::dsl_cards::predicate::eval_predicate(
        use_req,
        &ctx,
        digimon_engine::dsl_cards::predicate::PredicateSubject::None,
    ));
}
```

- [ ] **Step 2: Run the focused test and confirm it fails**

Run:

```bash
cargo test --test dsl any_field_permanent_sees_traited_egg_in_breeding -- --nocapture
```

Expected: fail because `any_field_permanent` is not evaluated in the engine predicate module.

- [ ] **Step 3: Add a field predicate subject**

In `code/digimon-engine/src/dsl_cards/predicate.rs`, extend `PredicateSubject`:

```rust
pub enum PredicateSubject {
    None,
    Card(CardHandle),
    Permanent(PermanentHandle),
    BreedingPermanent(PlayerId),
}
```

Add this helper:

```rust
fn eval_breeding_permanent_fields(
    pred: &CompiledPredicate,
    rctx: &EffectReadContext<'_>,
    player: PlayerId,
) -> bool {
    let Some(perm) = rctx.game.player(player).breeding_area.as_ref() else {
        return false;
    };
    let top_handle = perm.top_card().handle();
    if !eval_card_fields(pred, rctx, top_handle) {
        return false;
    }
    if let Some(want) = pred.kind {
        let Some(data) = rctx.game.card_data_for_handle(top_handle) else {
            return false;
        };
        let matches_kind = match (want, data.card_kind) {
            // Use Req scans the field; eggs in breeding can satisfy "Digimon"
            // requirements because the breeding area is part of that field scan.
            (CompiledCardKind::Digimon, CardKind::DigiEgg) => true,
            _ => kind_matches_field(want, data.card_kind),
        };
        if !matches_kind {
            return false;
        }
    }
    if let Some(want) = pred.in_breeding {
        if !want {
            return false;
        }
    }
    if !pred.zone.is_empty() && !pred.zone.contains(&CompiledZone::Breeding) {
        return false;
    }
    if let Some(want) = pred.owner {
        let matches = match want {
            CompiledPlayerRef::You => player == rctx.player,
            CompiledPlayerRef::Opponent => player == rctx.opponent_id(),
            CompiledPlayerRef::Active => player == rctx.game.turn_player(),
            CompiledPlayerRef::Any => true,
        };
        if !matches {
            return false;
        }
    }
    pred.is_suspended.is_none()
        && pred.is_unsuspended.is_none()
        && pred.stack_size_lte.is_none()
        && pred.stack_size_gte.is_none()
}
```

This intentionally evaluates the breeding permanent as the current stack's top card. A lone Digi-Egg in breeding is allowed to satisfy `kind: digimon` for Use Req field scans. A breeding stack also satisfies the predicate when the current top Digimon matches the requirement. Do not scan inherited sources as independent field permanents; inherited cards are part of the stack, not separate permanents with their own traits.

Update the `match subject` arm:

```rust
        PredicateSubject::BreedingPermanent(player) => {
            eval_breeding_permanent_fields(pred, rctx, player)
        }
```

- [ ] **Step 4: Add field existential scanning**

In `eval_predicate_with_bindings`, add:

```rust
    if let Some(ex) = &pred.any_field_permanent {
        if !field_existential_any(ex, rctx, bindings) {
            return false;
        }
    }
```

Add this helper:

```rust
fn field_existential_any(
    ex: &CompiledExistential,
    rctx: &EffectReadContext<'_>,
    bindings: Option<&Bindings>,
) -> bool {
    for p in existential_players(ex.of, rctx) {
        let n = rctx.game.player(p).battle_area.len();
        for i in 0..n {
            let handle = PermanentHandle {
                player: p,
                index: i as u8,
            };
            if eval_predicate_with_bindings(
                &ex.predicate,
                rctx,
                PredicateSubject::Permanent(handle),
                bindings,
            ) {
                return true;
            }
        }
        if rctx.game.player(p).breeding_area.is_some()
            && eval_predicate_with_bindings(
                &ex.predicate,
                rctx,
                PredicateSubject::BreedingPermanent(p),
                bindings,
            )
        {
            return true;
        }
    }
    false
}
```

- [ ] **Step 5: Run the field predicate test**

Run:

```bash
cargo test --test dsl any_field_permanent_sees_traited_egg_in_breeding -- --nocapture
```

Expected: pass.

- [ ] **Step 6: Commit this task**

```bash
git add code/digimon-engine/src/dsl_cards/predicate.rs code/digimon-engine/tests/dsl/option_use_req.rs
git commit -m "feat(engine): evaluate field permanents in DSL predicates"
```

---

### Task 3: Wire Use Req Into Option Use Legality

**Files:**
- Modify: `code/digimon-engine/src/effect.rs`
- Modify: `code/digimon-engine/src/dsl_cards/mod.rs`
- Modify: `code/digimon-engine/src/action/mask.rs`
- Modify: `code/digimon-engine/src/game_actions.rs`
- Test: `code/digimon-engine/tests/dsl/option_use_req.rs`

- [ ] **Step 1: Add failing Use Req action tests**

Append to `code/digimon-engine/tests/dsl/option_use_req.rs`:

```rust
#[test]
fn use_req_allows_option_without_matching_color_when_field_trait_exists() {
    let yaml = r#"
card: TST-USE-REQ
name: Beatbreak Option
kind: option
color: [green]
cost: 5
use_requirement:
  any_field_permanent:
    of: you
    any_of:
      - kind: digimon
      - kind: tamer
    trait_has: BEATBREAK
effects:
  - when: main_from_hand
    process:
      - draw: 1
"#;
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(yaml)
        .expect("inline DSL compiles")
        .add_card(beatbreak_egg("EGG-BEAT"))
        .add_card(off_trait_lv3("PLAIN-LV3"))
        .digitama(0, &["EGG-BEAT"])
        .hand(0, &["TST-USE-REQ"])
        .memory(10)
        .start();
    assert!(runner.game.hatch(0), "egg should move into breeding");
    runner.game.enter_main_phase();

    let mask = digimon_engine::action::mask::build_action_mask(&runner.game, 0);
    assert_eq!(
        mask[digimon_engine::action::PLAY_HAND_START as usize],
        1.0,
        "Use Req should satisfy Option use legality"
    );
}

#[test]
fn use_req_does_not_count_option_permanents_in_battle_area() {
    let yaml = r#"
card: TST-USE-REQ
name: Beatbreak Option
kind: option
color: [green]
cost: 5
use_requirement:
  any_field_permanent:
    of: you
    any_of:
      - kind: digimon
      - kind: tamer
    trait_has: BEATBREAK
effects:
  - when: main_from_hand
    process:
      - draw: 1
"#;
    let option_perm = r#"
card: TST-BEAT-OPTION-PERM
name: Beatbreak Placed Option
kind: option
color: [purple]
cost: 3
traits: [BEATBREAK]
effects: []
"#;
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(yaml)
        .expect("inline option DSL compiles")
        .from_dsl_yaml(option_perm)
        .expect("inline permanent DSL compiles")
        .hand(0, &["TST-USE-REQ"])
        .memory(10)
        .start();
    runner.place_on_field(0, "TST-BEAT-OPTION-PERM", Some(0));
    runner.game.enter_main_phase();

    let mask = digimon_engine::action::mask::build_action_mask(&runner.game, 0);
    assert_eq!(
        mask[digimon_engine::action::PLAY_HAND_START as usize],
        0.0,
        "Use Req must not count Option permanents"
    );
}
```

- [ ] **Step 2: Run the focused tests and confirm they fail**

Run:

```bash
cargo test --test dsl use_req_ -- --nocapture
```

Expected: fail because Option legality only checks color.

- [ ] **Step 3: Add Option color-bypass condition to effects**

In `code/digimon-engine/src/effect.rs`, add a field to `Effect`:

```rust
    pub option_color_requirement_bypass: Option<ConditionFn>,
```

Initialize it in `EffectBuilder::new`:

```rust
                option_color_requirement_bypass: None,
```

Add this builder method:

```rust
    pub fn option_color_requirement_bypass(mut self, cond: ConditionFn) -> Self {
        self.inner.option_color_requirement_bypass = Some(cond);
        self
    }
```

- [ ] **Step 4: Attach DSL Use Req predicates to OptionMain effects**

In `code/digimon-engine/src/dsl_cards/mod.rs`, where triggered clauses are lowered into `Effect`, detect `CompiledTiming::MainFromHand` for an Option or DUAL card. After the normal process closure is attached, add:

```rust
if matches!(compiled.kind, digimon_dsl::compiled::CompiledCardKind::Option)
    && matches!(clause.when, digimon_dsl::compiled::CompiledTiming::MainFromHand)
{
    if let Some(use_req) = compiled.use_requirement.clone() {
        builder = builder.option_color_requirement_bypass(Box::new(move |ctx| {
            crate::dsl_cards::predicate::eval_predicate(
                &use_req,
                ctx,
                crate::dsl_cards::predicate::PredicateSubject::None,
            )
        }));
    }
}
if matches!(compiled.kind, digimon_dsl::compiled::CompiledCardKind::Dual)
    && matches!(clause.when, digimon_dsl::compiled::CompiledTiming::MainFromHand)
{
    if let Some(dual) = &compiled.dual {
        if let Some(use_req) = dual.option.use_requirement.as_ref().map(|p| (**p).clone()) {
            builder = builder.option_color_requirement_bypass(Box::new(move |ctx| {
                crate::dsl_cards::predicate::eval_predicate(
                    &use_req,
                    ctx,
                    crate::dsl_cards::predicate::PredicateSubject::None,
                )
            }));
        }
    }
}
```

- [ ] **Step 5: Add a shared Option legality helper**

In `code/digimon-engine/src/action/mask.rs`, add:

```rust
pub(crate) fn option_use_requirement_available(
    game: &Game,
    card: &crate::card_source::CardSource,
    player_id: PlayerId,
) -> bool {
    let card_id = card.card_id(&game.card_data);
    let Some(effects) = game.effects_for_card(card_id, card.handle()) else {
        return false;
    };
    effects.iter().any(|effect| {
        let Some(cond) = &effect.option_color_requirement_bypass else {
            return false;
        };
        let ctx = EffectReadContext::new(game, card.handle(), None, player_id);
        cond(&ctx)
    })
}

pub(crate) fn option_use_requirement_or_color_available(
    game: &Game,
    card: &crate::card_source::CardSource,
    player_id: PlayerId,
) -> bool {
    let player = game.player(player_id);
    option_color_match_available(card, player, &game.card_data)
        || option_use_requirement_available(game, card, player_id)
}
```

Replace the Main-phase mask color check with:

```rust
                if is_option_use {
                    if !option_use_requirement_or_color_available(game, card, player_id) {
                        continue;
                    }
                }
```

In `code/digimon-engine/src/game_actions.rs`, replace the color-only check in `play_option_core` with:

```rust
            if !crate::action::mask::option_use_requirement_or_color_available(
                self,
                card,
                player_id,
            ) {
                return OptionPlayResult::Invalid;
            }
```

- [ ] **Step 6: Run Use Req tests**

Run:

```bash
cargo test --test dsl use_req_ -- --nocapture
```

Expected: both Use Req tests pass.

- [ ] **Step 7: Commit this task**

```bash
git add code/digimon-engine/src/effect.rs code/digimon-engine/src/dsl_cards/mod.rs code/digimon-engine/src/action/mask.rs code/digimon-engine/src/game_actions.rs code/digimon-engine/tests/dsl/option_use_req.rs
git commit -m "feat(engine): support DSL option use requirements"
```

---

### Task 4: Add Source-Kind Effect Immunity DSL Step

**Files:**
- Modify: `code/digimon-dsl/src/step.rs`
- Modify: `code/digimon-dsl/src/compiled.rs`
- Modify: `code/digimon-dsl/src/compile.rs`
- Modify: `code/digimon-engine/src/effect_context/mod.rs`
- Modify: `code/digimon-engine/src/dsl_cards/step/modifiers.rs`
- Test: `code/digimon-engine/tests/dsl/effect_immunity_step.rs`
- Modify: `code/digimon-engine/tests/dsl/main.rs`

- [ ] **Step 1: Add failing DSL immunity test**

Create `code/digimon-engine/tests/dsl/effect_immunity_step.rs`:

```rust
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::{EffectSourceKind, EffectTiming, TriggerSource};

#[test]
fn dsl_grants_opponent_digimon_effect_immunity_to_self() {
    let yaml = r#"
card: TST-IMMUNE
name: Immune Digimon
kind: digimon
level: 6
color: [green]
cost: 12
dp: 12000
traits: [BEATBREAK]
effects:
  - when: when_digivolving
    process:
      - grant_effect_immunity:
          target: self
          source_kind: digimon
          source_controller: opponent
          expiry: end_of_opponents_turn
"#;
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(yaml)
        .expect("inline DSL compiles")
        .add_card(make_test_card("BASE-LV5", "Base"))
        .start();
    let handle = runner.place_on_field(0, "TST-IMMUNE", Some(0));

    runner
        .game
        .enqueue_triggered(EffectTiming::WhenDigivolving, TriggerSource::Permanent(handle));
    runner.game.drain_effect_queue();

    assert!(
        runner
            .game
            .permanent_is_unaffected_by_effect(handle, 1, EffectSourceKind::Digimon),
        "opponent Digimon effects should not affect the target"
    );
    assert!(
        !runner
            .game
            .permanent_is_unaffected_by_effect(handle, 1, EffectSourceKind::Option),
        "opponent Option effects should still affect the target"
    );
}
```

Add to `code/digimon-engine/tests/dsl/main.rs`:

```rust
mod effect_immunity_step;
```

- [ ] **Step 2: Run the focused test and confirm it fails**

Run:

```bash
cargo test --test dsl dsl_grants_opponent_digimon_effect_immunity_to_self -- --nocapture
```

Expected: fail because `grant_effect_immunity` is unknown.

- [ ] **Step 3: Add DSL and compiled step types**

In `code/digimon-dsl/src/step.rs`, add:

```rust
    GrantEffectImmunity(GrantEffectImmunityArgs),
```

Add serialization/deserialization names beside other modifier steps:

```rust
            StepSpec::GrantEffectImmunity(v) => kv!(s, "grant_effect_immunity", v),
```

and:

```rust
            "grant_effect_immunity" => StepSpec::GrantEffectImmunity(map.next_value()?),
```

Add the args:

```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum EffectSourceKindSpec {
    Digimon,
    Tamer,
    Option,
    Rule,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum EffectControllerSpec {
    Any,
    Opponent,
    Own,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GrantEffectImmunityArgs {
    pub target: BindingRef,
    pub source_kind: EffectSourceKindSpec,
    #[serde(default = "default_effect_controller")]
    pub source_controller: EffectControllerSpec,
    pub expiry: String,
}

fn default_effect_controller() -> EffectControllerSpec {
    EffectControllerSpec::Opponent
}
```

In `code/digimon-dsl/src/compiled.rs`, add equivalent compiled enums and step:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CompiledEffectSourceKind {
    Digimon,
    Tamer,
    Option,
    Rule,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CompiledEffectController {
    Any,
    Opponent,
    Own,
}
```

Add to `CompiledStep`:

```rust
    GrantEffectImmunity {
        target: CompiledBindingRef,
        source_kind: CompiledEffectSourceKind,
        source_controller: CompiledEffectController,
        expiry: String,
    },
```

- [ ] **Step 4: Compile the new step**

In `code/digimon-dsl/src/compile.rs`, add mapping helpers:

```rust
fn compile_effect_source_kind(
    kind: crate::step::EffectSourceKindSpec,
) -> CompiledEffectSourceKind {
    match kind {
        crate::step::EffectSourceKindSpec::Digimon => CompiledEffectSourceKind::Digimon,
        crate::step::EffectSourceKindSpec::Tamer => CompiledEffectSourceKind::Tamer,
        crate::step::EffectSourceKindSpec::Option => CompiledEffectSourceKind::Option,
        crate::step::EffectSourceKindSpec::Rule => CompiledEffectSourceKind::Rule,
    }
}

fn compile_effect_controller(
    controller: crate::step::EffectControllerSpec,
) -> CompiledEffectController {
    match controller {
        crate::step::EffectControllerSpec::Any => CompiledEffectController::Any,
        crate::step::EffectControllerSpec::Opponent => CompiledEffectController::Opponent,
        crate::step::EffectControllerSpec::Own => CompiledEffectController::Own,
    }
}
```

In `compile_step`, add:

```rust
        S::GrantEffectImmunity(a) => CompiledStep::GrantEffectImmunity {
            target: compile_binding_ref(&a.target),
            source_kind: compile_effect_source_kind(a.source_kind),
            source_controller: compile_effect_controller(a.source_controller),
            expiry: a.expiry.clone(),
        },
```

- [ ] **Step 5: Add engine helper and lowering**

In `code/digimon-engine/src/effect_context/mod.rs`, add:

```rust
pub fn add_effect_immunity_modifier(
    &mut self,
    target: PermanentHandle,
    source_kind: EffectSourceKind,
    controller: crate::modifiers::EffectControllerFilter,
    expiry: Expiry,
) -> bool {
    if !self.can_affect_permanent(target) {
        return false;
    }
    self.game.modifiers.add(
        target,
        ModifierEntry::new(ModifierType::CannotBeAffected, 0, expiry)
            .with_effect_immunity_filter(
                crate::modifiers::EffectImmunityFilter {
                    source_kind: Some(source_kind),
                    controller,
                },
            ),
    );
    true
}
```

In `code/digimon-engine/src/dsl_cards/step/modifiers.rs`, add match handling:

```rust
        CompiledStep::GrantEffectImmunity {
            target,
            source_kind,
            source_controller,
            expiry,
        } => {
            let Some(expiry) = resolve_expiry("grant_effect_immunity", expiry) else {
                return true;
            };
            let Some(h) = bindings.resolve_permanent(target, ctx) else {
                return true;
            };
            let source_kind = match source_kind {
                digimon_dsl::compiled::CompiledEffectSourceKind::Digimon => EffectSourceKind::Digimon,
                digimon_dsl::compiled::CompiledEffectSourceKind::Tamer => EffectSourceKind::Tamer,
                digimon_dsl::compiled::CompiledEffectSourceKind::Option => EffectSourceKind::Option,
                digimon_dsl::compiled::CompiledEffectSourceKind::Rule => EffectSourceKind::Rule,
            };
            let controller = match source_controller {
                digimon_dsl::compiled::CompiledEffectController::Any => EffectControllerFilter::Any,
                digimon_dsl::compiled::CompiledEffectController::Opponent => EffectControllerFilter::OpponentOnly,
                digimon_dsl::compiled::CompiledEffectController::Own => EffectControllerFilter::OwnOnly,
            };
            ctx.add_effect_immunity_modifier(h, source_kind, controller, expiry);
            true
        }
```

Add imports:

```rust
use crate::enums::EffectSourceKind;
use crate::modifiers::EffectControllerFilter;
```

- [ ] **Step 6: Run the focused immunity test**

Run:

```bash
cargo test --test dsl dsl_grants_opponent_digimon_effect_immunity_to_self -- --nocapture
```

Expected: pass.

- [ ] **Step 7: Commit this task**

```bash
git add code/digimon-dsl/src/step.rs code/digimon-dsl/src/compiled.rs code/digimon-dsl/src/compile.rs code/digimon-engine/src/effect_context/mod.rs code/digimon-engine/src/dsl_cards/step/modifiers.rs code/digimon-engine/tests/dsl/main.rs code/digimon-engine/tests/dsl/effect_immunity_step.rs
git commit -m "feat(dsl): grant source-kind effect immunity"
```

---

### Task 5: Add Lowest/Highest DP Selection Narrowing

**Files:**
- Modify: `code/digimon-dsl/src/step.rs`
- Modify: `code/digimon-dsl/src/compiled.rs`
- Modify: `code/digimon-dsl/src/compile.rs`
- Modify: `code/digimon-engine/src/dsl_cards/step/selections.rs`
- Test: `code/digimon-engine/tests/dsl/selection_dp_extrema.rs`
- Modify: `code/digimon-engine/tests/dsl/main.rs`

- [ ] **Step 1: Add failing selection tests**

Create `code/digimon-engine/tests/dsl/selection_dp_extrema.rs`:

```rust
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::{EffectTiming, TriggerSource};

fn digimon(id: &str, dp: i32) -> digimon_engine::CardData {
    let mut card = make_test_card(id, id);
    card.dp = Some(dp);
    card
}

#[test]
fn selector_lowest_dp_only_offers_lowest_current_candidates() {
    let yaml = r#"
card: TST-LOWEST
name: Lowest Test
kind: digimon
level: 6
color: [red]
cost: 12
dp: 12000
effects:
  - when: when_digivolving
    process:
      - select_opponent_permanent:
          bind_as: tgt
          filter: { kind: digimon }
          selector: lowest_dp
          prompt: "Delete lowest DP"
      - delete_permanent: { target: tgt }
"#;
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(yaml)
        .expect("inline DSL compiles")
        .add_card(digimon("LOW", 3000))
        .add_card(digimon("HIGH", 9000))
        .start();
    let source = runner.place_on_field(0, "TST-LOWEST", Some(0));
    runner.place_on_field(1, "LOW", Some(0));
    runner.place_on_field(1, "HIGH", Some(0));

    runner
        .game
        .enqueue_triggered(EffectTiming::WhenDigivolving, TriggerSource::Permanent(source));
    runner.game.drain_effect_queue();
    let (selecting_player, action_id) = {
        let pending = runner.game.pending_selection.as_ref().expect("selection");
        assert_eq!(pending.valid_action_ids.len(), 1, "only LOW should be offered");
        (pending.selecting_player, pending.valid_action_ids[0])
    };
    runner
        .execute_action(selecting_player, action_id)
        .expect("selection resolves");
    assert_eq!(runner.battle_area_size(1), 1);
    assert_eq!(
        runner.game.players[1].battle_area[0].top_card().card_id(&runner.game.card_data),
        "HIGH"
    );
}

#[test]
fn selector_highest_dp_recomputes_after_prior_suspend_step() {
    let yaml = r#"
card: TST-HIGHEST-SUSPENDED
name: Highest Suspended Test
kind: option
color: [red]
cost: 5
effects:
  - when: main_from_hand
    process:
      - select_opponent_permanent:
          bind_as: first
          filter: { kind: digimon }
          selector: lowest_dp
          prompt: "Suspend a Digimon"
      - suspend: { target: first }
      - select_opponent_permanent:
          bind_as: second
          filter: { kind: digimon, is_suspended: true }
          selector: highest_dp
          prompt: "Bottom-deck highest suspended"
      - return_to_deck: { target: second, position: bottom, include_sources: true }
"#;
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(yaml)
        .expect("inline DSL compiles")
        .add_card(digimon("LOW", 3000))
        .add_card(digimon("HIGH", 9000))
        .hand(0, &["TST-HIGHEST-SUSPENDED"])
        .memory(10)
        .start();
    runner.place_on_field(0, "LOW", Some(0));
    runner.place_on_field(1, "LOW", Some(0));
    runner.place_on_field(1, "HIGH", Some(0));
    runner.game.enter_main_phase();

    assert!(matches!(
        runner.game.play_option_from_hand(0, 0),
        digimon_engine::selection::OptionPlayResult::Pending
    ));
    let (selecting_player, first_action) = {
        let pending = runner.game.pending_selection.as_ref().expect("first selection");
        assert_eq!(pending.valid_action_ids.len(), 1, "only LOW should be offered");
        (pending.selecting_player, pending.valid_action_ids[0])
    };
    runner
        .execute_action(selecting_player, first_action)
        .expect("first selection resolves");
    let (selecting_player, second_action) = {
        let pending = runner.game.pending_selection.as_ref().expect("second selection");
        assert_eq!(
            pending.valid_action_ids.len(),
            1,
            "only the now-suspended LOW Digimon should be offered"
        );
        (pending.selecting_player, pending.valid_action_ids[0])
    };
    runner
        .execute_action(selecting_player, second_action)
        .expect("second selection resolves");
    assert_eq!(runner.battle_area_size(1), 1);
    assert_eq!(
        runner.game.players[1].battle_area[0].top_card().card_id(&runner.game.card_data),
        "HIGH"
    );
}
```

Add to `code/digimon-engine/tests/dsl/main.rs`:

```rust
mod selection_dp_extrema;
```

- [ ] **Step 2: Run focused tests and confirm they fail**

Run:

```bash
cargo test --test dsl selector_ -- --nocapture
```

Expected: fail because `selector` is not accepted on `select_opponent_permanent`.

- [ ] **Step 3: Add selector fields to DSL and compiled selection args**

In `code/digimon-dsl/src/step.rs`, add:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum FieldSelector {
    LowestDp,
    HighestDp,
}
```

Add to `SelectFieldArgs`:

```rust
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selector: Option<FieldSelector>,
```

In `code/digimon-dsl/src/compiled.rs`, add:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CompiledFieldSelector {
    LowestDp,
    HighestDp,
}
```

Add `selector: Option<CompiledFieldSelector>` to `CompiledStep::SelectOwnPermanent`, `SelectOpponentPermanent`, and `SelectAnyPermanent`.

- [ ] **Step 4: Compile selector fields**

In `code/digimon-dsl/src/compile.rs`, add:

```rust
fn compile_field_selector(s: crate::step::FieldSelector) -> CompiledFieldSelector {
    match s {
        crate::step::FieldSelector::LowestDp => CompiledFieldSelector::LowestDp,
        crate::step::FieldSelector::HighestDp => CompiledFieldSelector::HighestDp,
    }
}
```

In the three selection compile arms, add:

```rust
            selector: a.selector.map(compile_field_selector),
```

- [ ] **Step 5: Apply selector narrowing at selection install time**

In `code/digimon-engine/src/dsl_cards/step/selections.rs`, update `install_select_own_permanent`, `install_select_opponent_permanent`, and `install_select_any_permanent` signatures to accept:

```rust
selector: Option<digimon_dsl::compiled::CompiledFieldSelector>,
```

Add helper:

```rust
fn selected_dp_extreme(
    game: &crate::game::Game,
    handles: &[PermanentHandle],
    selector: digimon_dsl::compiled::CompiledFieldSelector,
) -> Option<i32> {
    let values = handles.iter().filter_map(|h| game.effective_dp(*h));
    match selector {
        digimon_dsl::compiled::CompiledFieldSelector::LowestDp => values.min(),
        digimon_dsl::compiled::CompiledFieldSelector::HighestDp => values.max(),
    }
}
```

Before installing the pending selection, compute candidate handles by applying the existing predicate to current battle-area state, then restrict the filter closure:

```rust
let candidates = collect_matching_handles(ctx, owner, &filter);
let selected_dp = selector.and_then(|s| selected_dp_extreme(ctx.game, &candidates, s));

ctx.select_opponent_permanent(
    &prompt,
    optional,
    move |game, handle| {
        let read_ctx = EffectReadContext::new_with_source_kind(
            game,
            source_card,
            source_permanent,
            player,
            source_kind,
        );
        if !eval_predicate(&filter, &read_ctx, PredicateSubject::Permanent(handle)) {
            return false;
        }
        if let Some(dp) = selected_dp {
            return game.effective_dp(handle).is_some_and(|candidate_dp| candidate_dp == dp);
        }
        true
    },
    callback,
);
```

Use the same pattern for own and any permanent selection. For `select_any_permanent`, compute candidates across both battle areas after excluding any `excluded` handle.

- [ ] **Step 6: Run DP selector tests**

Run:

```bash
cargo test --test dsl selector_ -- --nocapture
```

Expected: both selector tests pass.

- [ ] **Step 7: Commit this task**

```bash
git add code/digimon-dsl/src/step.rs code/digimon-dsl/src/compiled.rs code/digimon-dsl/src/compile.rs code/digimon-engine/src/dsl_cards/step/selections.rs code/digimon-engine/tests/dsl/main.rs code/digimon-engine/tests/dsl/selection_dp_extrema.rs
git commit -m "feat(dsl): narrow selections by DP extrema"
```

---

### Task 6: Rewrite ST23-09 Example With The New Primitives

**Files:**
- Modify: `docs/examples/ST23-09-dual-card.yaml`

- [ ] **Step 1: Replace Use Req and raw effect placeholders**

Update `docs/examples/ST23-09-dual-card.yaml` so the relevant parts read:

```yaml
dual:
  digimon:
    level: 6
    dp: 12000
    colors: [green, black]
    traits: [Mutant, Glowing Dawn, BEATBREAK]
    effect_text: "Security A. +1, Reboot, Blocker. When digivolving or attacking once per turn: this Digimon is protected from opponent Digimon effects until their turn ends, then deletes an opponent's lowest-DP Digimon."
    inherited_text: ""
  option:
    use_cost: 5
    colors: [green]
    use_requirement:
      any_field_permanent:
        of: you
        any_of:
          - kind: digimon
          - kind: tamer
        trait_has: BEATBREAK
    effect_text: "Use requirement: BEATBREAK trait. Main: suspend an opposing Digimon, then return an opposing suspended highest-DP Digimon to the bottom of the deck. Arts Digivolve."
    security_text: ""
    keywords: [ArtsDigivolve]
```

Replace the Digimon-face process:

```yaml
    process:
      - grant_effect_immunity:
          target: self
          source_kind: digimon
          source_controller: opponent
          expiry: end_of_opponents_turn
      - select_opponent_permanent:
          bind_as: tgt
          filter: { kind: digimon }
          selector: lowest_dp
          prompt: "Delete opponent's lowest-DP Digimon"
      - delete_permanent: { target: tgt }
```

Replace the Option-face process:

```yaml
    process:
      - select_opponent_permanent:
          bind_as: first
          filter: { kind: digimon }
          prompt: "Suspend opponent Digimon"
      - suspend: { target: first }
      - select_opponent_permanent:
          bind_as: second
          filter: { kind: digimon, is_suspended: true }
          selector: highest_dp
          prompt: "Return opponent's highest-DP suspended Digimon"
      - return_to_deck: { target: second, position: bottom, include_sources: true }
```

- [ ] **Step 2: Parse the example through the DSL**

Run:

```bash
cargo test --test dsl parses_dual_card_metadata -- --nocapture
```

Expected: existing dual metadata test still passes. If a dedicated example parse test exists, add `docs/examples/ST23-09-dual-card.yaml` to it; do not put ST23-09 under `cards/_examples` until `data/cards.json` contains the card.

- [ ] **Step 3: Commit this task**

```bash
git add docs/examples/ST23-09-dual-card.yaml
git commit -m "docs(engine): express ST23-09 with DSL primitives"
```

---

### Task 7: Document The Rule And DSL Surface

**Files:**
- Modify: `docs/RULES_CONTEXT.md`
- Modify: `docs/RUST_ENGINE_API.md`

- [ ] **Step 1: Add rules context**

In `docs/RULES_CONTEXT.md`, add:

```markdown
### Option Use Requirement

`<Use Req. (...)>` is an Option-use permission that can satisfy the color requirement for using that Option. The predicate scans the player's field, which includes the battle area and breeding area. Digimon and Tamers can satisfy the requirement in the battle area. Digimon and Digi-Eggs can satisfy it in the breeding area. Option permanents in the battle area do not satisfy a Use Req that asks for Digimon or Tamers.
```

- [ ] **Step 2: Add DSL API documentation**

In `docs/RUST_ENGINE_API.md`, add:

```markdown
### DSL Option Use Requirements

Normal Option cards may declare:

```yaml
use_requirement:
  any_field_permanent:
    of: you
    trait_has: BEATBREAK
```

DUAL Option faces may declare the same predicate under `dual.option.use_requirement`.

`any_field_permanent` scans battle area plus breeding area. It should be used when printed text says "field" rather than "battle area".

### DSL Effect Immunity

Use `grant_effect_immunity` for effects such as "your opponent's Digimon effects don't affect this Digimon":

```yaml
- grant_effect_immunity:
    target: self
    source_kind: digimon
    source_controller: opponent
    expiry: end_of_opponents_turn
```

### DSL DP-Extreme Selection

Permanent selections can narrow candidates to the current lowest or highest DP:

```yaml
- select_opponent_permanent:
    bind_as: tgt
    filter: { kind: digimon, is_suspended: true }
    selector: highest_dp
    prompt: "Return opponent's highest-DP suspended Digimon"
```

The selector is evaluated when the selection is installed, after prior process steps have mutated the board.
```
```

- [ ] **Step 3: Commit this task**

```bash
git add docs/RULES_CONTEXT.md docs/RUST_ENGINE_API.md
git commit -m "docs(engine): document option use and DSL targeting primitives"
```

---

### Task 8: Full Verification

**Files:**
- No source edits.

- [ ] **Step 1: Run focused suites**

Run:

```bash
cargo test --test dsl option_use_req -- --nocapture
cargo test --test dsl selection_dp_extrema -- --nocapture
cargo test --test dsl effect_immunity_step -- --nocapture
cargo test --test effect_source_kind -- --nocapture
cargo test --test dual_cards -- --nocapture
```

Expected: all listed tests pass.

- [ ] **Step 2: Run full engine tests**

Run:

```bash
cargo test
```

Expected: exit code 0. Existing warnings are acceptable if they match the pre-existing warning classes from the effect-source-kind work: unused/dead code warnings in tests and engine internals.

- [ ] **Step 3: Review worktree scope**

Run:

```bash
git status --short
git diff --stat
```

Expected: changed files match the file list in this plan plus any formatting changes caused by `cargo fmt`.

- [ ] **Step 4: Commit verification cleanup**

If verification forced small fixes, commit them:

```bash
git add code/digimon-dsl code/digimon-engine docs
git commit -m "test(engine): verify DSL option primitives"
```

---

## Self-Review

Spec coverage:
- Use Req as a color-requirement bypass is covered in Tasks 1-3.
- `any_field_permanent` including breeding-area Digi-Eggs is covered in Tasks 1-2.
- Source-kind Digimon effect immunity is covered in Task 4.
- Lowest/highest DP selection narrowing is covered in Task 5.
- ST23-09 can drop its raw placeholders for these primitives in Task 6.
- Docs are covered in Task 7.

Type consistency:
- Authored names: `any_field_permanent`, `use_requirement`, `grant_effect_immunity`, `selector: lowest_dp`, `selector: highest_dp`.
- Compiled names: `CompiledFieldSelector`, `CompiledEffectSourceKind`, `CompiledEffectController`.
- Engine source-kind names reuse existing `EffectSourceKind::{Digimon,Tamer,Option,Rule}`.

Testing:
- Each behavior has a failing test before implementation.
- Focused DSL tests run before full `cargo test`.
