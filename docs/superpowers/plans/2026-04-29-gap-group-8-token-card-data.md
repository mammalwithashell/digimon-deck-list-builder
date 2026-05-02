# Gap Group 8 Token Card Data Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Complete Group 8 token and card-data gaps so tokens are real effect-bearing cards, authored DNA and alias metadata drive legal engine actions, ACE Overflow applies from card metadata, and reveal-zone card overlays preserve printed identity without leaking across unrelated matching systems.

**Architecture:** Keep printed/card metadata in `CardData` or compiled DSL registry data, keep all player-visible decisions in `PendingSelection` plus the existing action mask/decoder, and keep subsystem-specific semantics scoped to their subsystem. Token effects are normal `CardEffect`s. DNA costs feed the existing DNA action range. DigiXros aliases are only consulted by DigiXros material matching. ACE Overflow is a metadata-driven leave-zone penalty, not a one-off card effect closure.

**Tech Stack:** Rust `digimon-engine`, Rust `digimon-dsl`, YAML card specs in `code/digimon-engine/cards/`, Cargo integration tests, and markdown gap trackers.

---

## Scope Notes

Group 8 covers:

1. Familiar Token `[On Deletion]`.
2. Token definitions and `CardKind::Token` invariants.
3. `CardData.dna_costs` YAML and loaded-data population.
4. DigiXros scoped aliases.
5. ACE Overflow metadata and leave-zone penalty.
6. Reveal-zone overlays.

Do not expand `ACTION_SPACE_SIZE` or `TENSOR_SIZE` for this group. The expected implementation uses existing selection ranges, existing DNA action IDs `63..93`, existing reveal selection IDs `30..39`, and existing tensor reveal slots. If a task proves a contract change is unavoidable, stop after the failing test and update `docs/ACTION_SPEC.md`, `docs/TENSOR_SPEC.md`, Rust constants, PyO3 exports, `code/digimon_gym/digimon_gym.py`, and frontend constants in the same task before claiming it is complete.

Serialize tasks that change shared metadata structs:

- Run Task 1 and Task 2 independently from the card-data schema tasks.
- Run Task 3 before Task 4 and Task 5 if all three touch `CardData` or DSL compiled IR.
- Run Task 6 after Task 4, because reveal overlays must not accidentally reuse generic aliases.

Group 2 selection primitives are already present in this worktree (`EffectContext::select_opponent_permanent`, reveal selection, ordered permutation, breeding permanent selection). If this plan is run from an older branch, Task 1 is blocked until `code/digimon-engine/src/effect_context/selections.rs` exposes callback-based opponent permanent selection through `PendingSelection`.

---

## File Structure

Read before editing:

- `AGENTS.md`
- `CLAUDE.md`
- `docs/RUST_ENGINE_API.md`
- `docs/RUST_DSL_TEST_API.md`
- `docs/ACTION_SPEC.md`
- `docs/TENSOR_SPEC.md`
- `docs/RUST_ENGINE_GAPS.md`
- `qa/archetype-qa/engine-gaps.md`
- `qa/dsl-vocab-gaps.md`
- `docs/superpowers/specs/2026-04-29-archetype-engine-dsl-gap-roadmap-design.md`
- `docs/superpowers/plans/2026-04-29-archetype-engine-dsl-gap-roadmap.md`

Expected engine files:

- `code/digimon-engine/src/token_registry.rs`
- `code/digimon-engine/src/cards/tokens/familiar.rs`
- `code/digimon-engine/src/cards/tokens/petrification.rs`
- `code/digimon-engine/src/cards/tokens/mod.rs`
- `code/digimon-engine/src/cards/test/mod.rs`
- `code/digimon-engine/src/cards/test/test_023.rs`
- `code/digimon-engine/src/card_data.rs`
- `code/digimon-engine/src/card_registry.rs`
- `code/digimon-engine/src/card_source.rs`
- `code/digimon-engine/src/dsl_bridge.rs`
- `code/digimon-engine/src/game.rs`
- `code/digimon-engine/src/combat.rs`
- `code/digimon-engine/src/dna_digivolve.rs`
- `code/digimon-engine/src/action/mask.rs`
- `code/digimon-engine/src/action/decode.rs`
- `code/digimon-engine/src/action/explain.rs`
- `code/digimon-engine/src/effect_context/selections.rs`
- `code/digimon-engine/src/dsl_cards/mod.rs`
- `code/digimon-engine/src/dsl_cards/predicate.rs`

Expected DSL files:

- `code/digimon-dsl/src/spec.rs`
- `code/digimon-dsl/src/compiled.rs`
- `code/digimon-dsl/src/compile.rs`
- `code/digimon-dsl/src/alt_path.rs`
- `code/digimon-dsl/src/identity.rs`
- `code/digimon-dsl/src/predicate.rs`
- `code/digimon-dsl/src/validator.rs`
- `code/digimon-dsl/src/schema.rs`

Expected card data files:

- `code/digimon-engine/cards/bt20/BT20-016.yaml`
- `code/digimon-engine/cards/_examples/BT10-111.yaml`
- ACE sample YAML already present under `code/digimon-engine/cards/bt17/BT17-018.yaml`, `code/digimon-engine/cards/ex10/EX10-010.yaml`, `code/digimon-engine/cards/ex9/EX9-013.yaml`, and `code/digimon-engine/cards/lm/LM-021.yaml`

Expected tests:

- `code/digimon-engine/tests/cards_behavioral/tokens.rs`
- `code/digimon-engine/tests/dna_digivolve_user_action.rs`
- `code/digimon-engine/tests/dsl/phase3_reducer_costs.rs`
- `code/digimon-engine/tests/dsl/parse_identity.rs`
- `code/digimon-engine/tests/dsl/phase1c_lowering.rs`
- `code/digimon-engine/tests/dsl/phase2e_select_reveal.rs`
- `code/digimon-engine/tests/keyword_parsing.rs`
- `code/digimon-engine/tests/mask_and_tensor/card_registry_parity.rs`

Expected trackers:

- `docs/RUST_ENGINE_GAPS.md`
- `qa/archetype-qa/engine-gaps.md`
- `qa/dsl-vocab-gaps.md`
- relevant archetype files under `qa/archetype-qa/`

---

## Task 1: Familiar Token On Deletion

- [ ] **Step 1: Add the failing behavioral test**

Add a new test card for Familiar token creation so the test exercises `ctx.play_token`, the token registry, token effect registration, deletion, pending selection, action mask, decoder, and DP modifier path.

Create `code/digimon-engine/src/cards/test/test_029.rs`:

```rust
//! TEST-029: "OnPlay: play 1 Familiar Token."

use crate::card_source::CardHandle;
use crate::effect::{CardEffect, Effect};

pub struct Test029;

impl CardEffect for Test029 {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::on_play(card)
            .name("Play a Familiar Token")
            .process(|ctx| {
                let me = ctx.player;
                ctx.play_token(me, "familiar");
            })
            .build()]
    }
}
```

Register it in `code/digimon-engine/src/cards/test/mod.rs`:

```rust
mod test_029;
registry.insert("TEST-029", Arc::new(test_029::Test029));
```

Append this test to `code/digimon-engine/tests/cards_behavioral/tokens.rs`:

```rust
#[test]
fn familiar_on_deletion_prompts_opponent_target_and_applies_minus_3000() {
    use digimon_engine::action::space::{encode_attack, PASS};
    use digimon_engine::permanent::PermanentHandle;
    use digimon_engine::selection::SelectionKind;

    let mut opp_low = make_test_card("OPP-LOW", "OppLow");
    opp_low.dp = Some(4000);
    let mut opp_high = make_test_card("OPP-HIGH", "OppHigh");
    opp_high.dp = Some(7000);

    let mut r = DebugRunner::builder()
        .add_card(make_test_card("TEST-029", "PlayFamiliarToken"))
        .add_card(opp_low)
        .add_card(opp_high)
        .hand(0, &["TEST-029"])
        .memory(5)
        .start();

    r.place_on_field(1, "OPP-LOW", Some(0));
    r.place_on_field(1, "OPP-HIGH", Some(0));
    r.play(0, 0).expect("TEST-029 plays");

    let token_idx = r
        .game
        .player(0)
        .battle_area
        .iter()
        .position(|p| p.top_card().card_name(&r.game.card_data) == "Familiar Token")
        .expect("Familiar Token missing");

    r.game.delete_permanent_with_effects(PermanentHandle {
        player: 0,
        index: token_idx as u8,
    });

    let pending = r
        .game
        .pending_selection
        .as_ref()
        .expect("Familiar OnDeletion must ask for an opponent Digimon");
    assert_eq!(pending.kind, SelectionKind::OppField);
    assert_eq!(pending.selecting_player, 0);
    assert_eq!(pending.valid_action_ids.len(), 2);
    assert!(!pending.valid_action_ids.contains(&PASS));

    let target_action = encode_attack(0, 1);
    let mask = r.game.get_action_mask(0);
    assert_eq!(mask[target_action as usize], 1.0);

    r.game.decode_action(target_action, 0);

    let target = PermanentHandle { player: 1, index: 1 };
    assert_eq!(
        r.game.effective_dp(target),
        Some(4000),
        "OPP-HIGH should be 7000 DP with a -3000 DP until-turn-end modifier"
    );
}
```

Run to fail:

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- familiar_on_deletion_prompts_opponent_target_and_applies_minus_3000
```

- [ ] **Step 2: Implement the token effect**

Modify `code/digimon-engine/src/cards/tokens/familiar.rs` so `FamiliarToken::effects` returns an `Effect::on_deletion(card)` that:

- Calls `ctx.select_opponent_permanent`.
- Uses `is_optional = false`.
- Filters to opponent battle-area Digimon only.
- Applies `ctx.give_dp_modifier(target, -3000)` or the existing until-turn-end DP modifier helper used by other cards in this tree.
- Returns immediately after installing the selection.

Use the existing selection helper in `code/digimon-engine/src/effect_context/selections.rs`; do not special-case single-target boards and do not auto-pick.

Run to pass:

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- familiar_on_deletion_prompts_opponent_target_and_applies_minus_3000
```

- [ ] **Step 3: Contract review**

Confirm no action or tensor constants changed:

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- familiar_on_deletion
cargo test --manifest-path code/digimon-engine/Cargo.toml --test mask_and_tensor
```

Only update `docs/ACTION_SPEC.md` or `docs/TENSOR_SPEC.md` if a constant actually changed.

---

## Task 2: Token Definitions and `CardKind::Token` Invariants

- [ ] **Step 1: Add invariant tests**

Extend `code/digimon-engine/src/token_registry.rs` tests:

```rust
#[test]
fn all_registered_tokens_synthesize_token_card_data() {
    let registry = TokenRegistry::default();
    for name in registry.names() {
        let token = registry.get(name).expect("registered token exists");
        let data = token.to_card_data();
        assert_eq!(data.card_kind, CardKind::Token, "{name}");
        assert_eq!(data.play_cost, 0, "{name}");
        assert!(data.evo_costs.is_empty(), "{name}");
        assert!(data.dna_costs.is_empty(), "{name}");
        assert_eq!(data.effect_class_name, data.card_id, "{name}");
    }
}
```

If `TokenRegistry::names()` does not exist, add it as:

```rust
pub fn names(&self) -> impl Iterator<Item = &str> {
    self.tokens.keys().map(String::as_str)
}
```

Add an integration assertion to `code/digimon-engine/tests/cards_behavioral/tokens.rs` that both `Petrification Token` and `Familiar Token` are removed from game rather than moved to trash when deleted.

Run to fail:

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml token_registry
cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- token_delete_removes_from_game_not_trash
```

- [ ] **Step 2: Close registry gaps**

Modify `code/digimon-engine/src/token_registry.rs`, `code/digimon-engine/src/cards/tokens/mod.rs`, and token card files so:

- Every token definition has a registered `CardEffect`.
- `to_card_data()` consistently emits `CardKind::Token`, zero play cost, empty evolution and DNA costs, and the token card id as `effect_class_name`.
- Token deletion continues to fire `[On Deletion]` before the token is removed from game.

Run to pass:

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml token_registry
cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- token
```

- [ ] **Step 3: Documentation tracker update**

Update `docs/RUST_ENGINE_API.md` token section to list both registered tokens and their implemented effects. Update `docs/RUST_ENGINE_GAPS.md` only after tests pass.

---

## Task 3: `CardData.dna_costs` Authored Data and Action Masks

Current worktree note: `code/digimon-engine/src/dsl_bridge.rs` already has `enrich_card_data_with_dsl_alt_paths`, and `code/digimon-engine/src/game.rs` calls it under the `dsl-yaml-loader` feature. This task locks that behavior with action-mask and production-YAML coverage, then closes stale tracker entries.

- [ ] **Step 1: Add an action-mask regression for real YAML DNA metadata**

Append this test to `code/digimon-engine/tests/dna_digivolve_user_action.rs`:

```rust
#[test]
fn authored_dna_alt_path_makes_dna_action_legal_for_bt20_016() {
    use digimon_engine::action::space::DNA_DIGIVOLVE_START;
    use digimon_engine::debug_runner::{make_test_card, DebugRunner};

    const YAML: &str = include_str!("../cards/bt20/BT20-016.yaml");

    let mut red_lv4 = make_test_card("RED-LV4", "RedLv4");
    red_lv4.level = Some(4);
    red_lv4.colors = vec![digimon_engine::enums::CardColor::Red];
    let mut purple_lv4 = make_test_card("PURPLE-LV4", "PurpleLv4");
    purple_lv4.level = Some(4);
    purple_lv4.colors = vec![digimon_engine::enums::CardColor::Purple];

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("BT20-016 YAML loads")
        .add_card(red_lv4)
        .add_card(purple_lv4)
        .hand(0, &["BT20-016"])
        .memory(5)
        .build();

    runner.place_on_field(0, "RED-LV4", Some(0));
    runner.place_on_field(0, "PURPLE-LV4", Some(0));

    let mask = runner.game.get_action_mask(0);
    assert_eq!(mask[DNA_DIGIVOLVE_START as usize], 1.0);
}
```

Run to fail:

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test dna_digivolve_user_action -- authored_dna_alt_path_makes_dna_action_legal_for_bt20_016
```

- [ ] **Step 2: Fill the missing bridge pieces**

If the new test fails, modify only the missing bridge:

- `code/digimon-engine/src/debug_runner.rs`: `card_data_from_compiled` must include DNA costs from compiled `alt_paths` for test-loaded YAML cards, matching `dsl_bridge::compiled_dna_costs`.
- `code/digimon-engine/src/dsl_bridge.rs`: expose a small helper returning `Vec<DnaCost>` from a `CompiledCard` if needed by both `Game::new` and `DebugRunner`.
- `code/digimon-engine/src/game.rs`: keep runtime enrichment in `Game::new`.
- `code/digimon-engine/src/action/mask.rs`: do not add new action bits; keep `DNA_DIGIVOLVE_START + hand_idx`.

Run to pass:

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- dsl_dna_alt_path_enriches_card_data_dna_costs
cargo test --manifest-path code/digimon-engine/Cargo.toml --test dna_digivolve_user_action -- authored_dna_alt_path_makes_dna_action_legal_for_bt20_016
```

- [ ] **Step 3: Production data sweep**

Search for all `kind: dna_digivolve` YAML files and run their compile tests:

```bash
Get-ChildItem -Path code/digimon-engine/cards -Recurse -Filter *.yaml | Select-String -Pattern 'kind: dna_digivolve'
cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- bt20_016_has_dna_digivolve_alt_path
```

Update `qa/dsl-vocab-gaps.md` entry "BG Imperial DNA cards - YAML `dna_costs` authoring / production data population" with the passing command and mark the stale missing-bridge sentence as resolved.

---

## Task 4: DigiXros Scoped Aliases

- [ ] **Step 1: Add DSL and predicate tests for scoped aliasing**

Add a test to `code/digimon-engine/tests/dsl/parse_identity.rs`:

```rust
#[test]
fn digixros_aliases_parse_without_generic_identity_leakage() {
    let yaml = r#"
card: XROS-ALIAS
name: Alias Carrier
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

    let spec: digimon_dsl::spec::CardSpec = serde_yml::from_str(yaml).expect("parse");
    let compiled = digimon_dsl::compile::compile(&spec).expect("compile");
    assert_eq!(compiled.digixros_aliases, vec!["Shoutmon"]);
    assert!(compiled.identity.is_none());
}
```

Add a runtime matching test near DigiXros tests or create `code/digimon-engine/tests/dsl/digixros_aliases.rs`:

```rust
#[test]
fn digixros_material_matching_sees_scoped_alias_but_name_predicates_do_not() {
    use digimon_engine::card_data::CardData;
    use digimon_engine::enums::{CardColor, CardKind};

    let material = CardData {
        card_id: "MATERIAL-A".to_string(),
        card_name: "Alias Carrier".to_string(),
        card_kind: CardKind::Digimon,
        level: Some(3),
        dp: Some(3000),
        play_cost: 3,
        colors: vec![CardColor::Red],
        traits: vec!["Xros Heart".to_string()],
        evo_costs: Vec::new(),
        dna_costs: Vec::new(),
        effect_text: String::new(),
        inherited_text: String::new(),
        security_text: String::new(),
        keywords: Vec::new(),
        dual: None,
        effect_class_name: "MATERIAL_A".to_string(),
        index: 0,
        norm_id: 0.0,
        digixros_aliases: vec!["Shoutmon".to_string()],
    };

    assert!(
        digimon_engine::digixros::matches_digixros_name_requirement_for_test(
            &material,
            "Shoutmon"
        ),
        "DigiXros recipe matching must see scoped aliases"
    );
    assert!(
        !digimon_engine::digixros::matches_generic_name_requirement_for_test(
            &material,
            "Shoutmon"
        ),
        "generic name predicates must not see DigiXros aliases"
    );
}
```

Run to fail:

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- digixros_aliases_parse_without_generic_identity_leakage
cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- digixros_material_matching_sees_scoped_alias_but_name_predicates_do_not
```

- [ ] **Step 2: Add scoped metadata**

Modify DSL and engine metadata:

- `code/digimon-dsl/src/spec.rs`: add `digixros_aliases: Vec<String>` with serde default and skip-when-empty.
- `code/digimon-dsl/src/compiled.rs`: add the same field to `CompiledCard`.
- `code/digimon-dsl/src/compile.rs`: copy from spec to compiled card.
- `code/digimon-engine/src/card_data.rs`: add `digixros_aliases: Vec<String>` to `CardData` and `RawCard` with serde default.
- `code/digimon-engine/src/debug_runner.rs`: copy compiled aliases into test `CardData`.
- `code/digimon-engine/src/dsl_bridge.rs`: copy compiled aliases into loaded runtime `CardData` if runtime compiled cards need enrichment.

Do not put DigiXros aliases into `identity.name_aliases`. Do not make `CardSource::card_name`, `CardData::text_for_search_all_faces`, or generic `name_contains` see these aliases.

- [ ] **Step 3: Use aliases only in DigiXros matching**

Modify the DigiXros material matching code so the candidate material names are:

```text
[printed card_name] + [card_data.digixros_aliases]
```

for DigiXros recipe matching only.

Run to pass:

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- digixros_aliases
cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- parse_identity
```

- [ ] **Step 4: Documentation tracker update**

Update `docs/RUST_ENGINE_GAPS.md` entry for DigiXros scoped aliases with the passing command. If any archetype QA file lists this alias gap, mark only the scoped alias part resolved.

---

## Task 5: ACE Overflow Metadata and Leave-Zone Penalty

- [ ] **Step 1: Add failing card-data and leave-zone tests**

Add metadata coverage to `code/digimon-engine/tests/keyword_parsing.rs` or `code/digimon-engine/tests/dsl/phase1c_lowering.rs`:

```rust
#[test]
fn dsl_ace_overflow_populates_runtime_card_data() {
    let yaml = r#"
card: ACE-RUNTIME
name: Ace Runtime
kind: digimon
level: 5
color: [red]
cost: 7
dp: 7000
ace_overflow: -4
"#;
    let spec: digimon_dsl::spec::CardSpec = serde_yml::from_str(yaml).expect("parse");
    let compiled = digimon_dsl::compile::compile(&spec).expect("compile");
    let card_data = digimon_engine::debug_runner::card_data_for_test_from_compiled(&compiled);
    assert_eq!(card_data.ace_overflow, Some(-4));
}
```

If `card_data_for_test_from_compiled` is not public, expose a small test-only helper rather than duplicating card-data lowering in the test.

Add behavioral tests in a new `code/digimon-engine/tests/ace_overflow.rs`:

```rust
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::SourceSelectionRef;

#[test]
fn ace_overflow_loses_memory_when_top_card_leaves_battle_area() {
    let mut ace = make_test_card("ACE-RUNTIME", "Ace Runtime");
    ace.ace_overflow = Some(-4);

    let mut runner = DebugRunner::builder()
        .add_card(ace)
        .memory(3)
        .build();

    let handle = runner.place_on_field(0, "ACE-RUNTIME", Some(0));
    runner.game.delete_permanent_with_effects(handle);

    assert_eq!(runner.game.memory, -1);
}

#[test]
fn ace_overflow_loses_memory_when_source_leaves_under_stack() {
    let mut ace = make_test_card("ACE-SOURCE", "Ace Source");
    ace.ace_overflow = Some(-4);
    let top = make_test_card("TOP", "Top");

    let mut runner = DebugRunner::builder()
        .add_card(ace)
        .add_card(top)
        .memory(3)
        .build();

    let perm = runner.place_on_field(0, "ACE-SOURCE", Some(0));
    let top_data = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == "TOP")
        .expect("TOP card data");
    let top_card = CardSource::new(top_data, 0, runner.game.next_card_index());
    runner.game.players[0].battle_area[perm.index as usize]
        .card_sources
        .push(top_card);

    let source_card = runner.game.players[0].battle_area[perm.index as usize].card_sources[0]
        .handle();
    let source_ref = SourceSelectionRef {
        permanent: PermanentHandle {
            player: 0,
            index: perm.index,
        },
        field_index: perm.index,
        source_index: 0,
        card: source_card,
    };

    assert!(runner.game.trash_source_ref(source_ref));
    assert_eq!(runner.game.memory, -1);
}
```

Run to fail:

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test ace_overflow
cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- dsl_ace_overflow_populates_runtime_card_data
```

- [ ] **Step 2: Add runtime metadata**

Modify:

- `code/digimon-engine/src/card_data.rs`: add `ace_overflow: Option<i32>` to `CardData` and `RawCard` with serde default and skip behavior consistent with other optional fields.
- `code/digimon-engine/src/debug_runner.rs`: copy compiled `ace_overflow`.
- `code/digimon-engine/src/dsl_bridge.rs`: enrich runtime `CardData` from compiled `ace_overflow` when present.
- `code/digimon-engine/src/token_registry.rs`: emit `ace_overflow: None` for tokens.
- Tests that construct `CardData` literals: add `ace_overflow: None`.

Keep the existing `DslCardEffect::ace_overflow()` accessor for compiled-card tests.

- [ ] **Step 3: Apply leave-zone penalty**

Implement one shared helper in `code/digimon-engine/src/combat.rs` or `code/digimon-engine/src/game_actions.rs`:

```rust
fn apply_ace_overflow_for_sources(&mut self, sources: &[CardSource]) {
    let penalty: i32 = sources
        .iter()
        .filter_map(|source| self.card_data[source.data_index].ace_overflow)
        .sum();
    if penalty != 0 {
        self.memory += penalty;
    }
}
```

Call it from every path that removes a permanent or card source from the battle area or from under a battle-area permanent:

- `delete_permanent_with_effects` normal deletion finalization.
- return-to-hand replacement finalization.
- return-to-deck replacement finalization.
- source-removal or material-move helpers that take cards out from under a permanent.

Do not apply ACE Overflow when cards move within the same battle-area stack as digivolution material or when a token is removed from game.

Run to pass:

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test ace_overflow
cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- ace_overflow
cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- bt17_018_ace_overflow_is_minus_5
```

- [ ] **Step 4: Tracker update**

Update `docs/RUST_ENGINE_GAPS.md` entry "Ace Overflow: inherited memory penalty on zone-change from field / under-card" with the passing command. Keep any unrelated ACE card-effect gaps open.

---

## Task 6: Reveal-Zone Overlays

- [ ] **Step 1: Add failing overlay tests**

Add tests to `code/digimon-engine/tests/dsl/phase2e_select_reveal.rs`:

```rust
#[test]
fn revealed_card_overlay_affects_reveal_predicates_only() {
    // Build a revealed CardSource with a temporary overlay name or kind.
    // Assert select_reveal can match the overlay.
    // Assert generic hand/trash/battle predicates cannot see the overlay
    // after the card leaves the reveal zone.
}

#[test]
fn reveal_overlay_survives_select_reveal_until_destination_move() {
    // Reveal two cards, attach overlay metadata to one revealed source,
    // select it through SEL_REVEAL_START + index, move it to hand or deck,
    // and assert the overlay is cleared from the moved CardSource.
}
```

Use existing helpers from `code/digimon-engine/tests/zone_manipulation.rs` for pushing `CardSource`s into `game.revealed_cards` and moving from reveal.

Run to fail:

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- revealed_card_overlay
```

- [ ] **Step 2: Implement scoped overlay storage**

Modify:

- `code/digimon-engine/src/card_source.rs`: add an overlay field that is explicit and scoped, for example `reveal_overlay: Option<RevealOverlay>`.
- `code/digimon-engine/src/card_data.rs`: add `RevealOverlay` only if it belongs in shared data; prefer `CardSource` for per-instance reveal overlays.
- Reveal-zone move helpers in `code/digimon-engine/src/effect_context/selections.rs` and zone movement code: preserve overlay while the card remains in `game.revealed_cards`; clear overlay as the card moves to hand, trash, deck, security, field, or under a permanent.
- `code/digimon-engine/src/dsl_cards/predicate.rs`: consult reveal overlays only for predicates that are evaluating `CompiledZone::Reveal` candidates.

Do not let reveal overlays affect generic name matching, DigiXros matching, permanent predicates, trash predicates, or hand predicates after zone movement.

Run to pass:

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- revealed_card_overlay
cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- select_reveal_binds_picked_card_handle
cargo test --manifest-path code/digimon-engine/Cargo.toml --test zone_manipulation -- reveal
```

- [ ] **Step 3: Tensor/action review**

Confirm reveal overlays do not change tensor size:

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test mask_and_tensor
```

If overlay visibility is intentionally added to tensors, update `docs/TENSOR_SPEC.md`, Rust tensor offsets, PyO3 constants, `code/digimon_gym/digimon_gym.py`, and frontend tensor consumers in the same task.

---

## Task 7: Full Regression and Tracker Closure

- [ ] **Step 1: Run targeted group tests**

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- token
cargo test --manifest-path code/digimon-engine/Cargo.toml --test dna_digivolve_user_action
cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- digixros
cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- reveal
cargo test --manifest-path code/digimon-engine/Cargo.toml --test ace_overflow
cargo test --manifest-path code/digimon-engine/Cargo.toml --test keyword_parsing
cargo test --manifest-path code/digimon-engine/Cargo.toml --test mask_and_tensor
```

- [ ] **Step 2: Run contract smoke tests**

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml
DIGIMON_BACKEND=rust python -m pytest code/engine_py_legacy/tests/engine/test_rust_backend_parity.py -v
python -m pytest code/tests/rl -v
```

If Python dependencies or the PyO3 wheel are unavailable, record the exact failure and run:

```bash
cd code/digimon-engine-py
maturin develop
cd ../..
DIGIMON_BACKEND=rust python -m pytest code/engine_py_legacy/tests/engine/test_rust_backend_parity.py -v
```

- [ ] **Step 3: Update trackers with evidence**

Update only entries proven by passing tests:

- `docs/RUST_ENGINE_GAPS.md`: token completion, ACE Overflow, DigiXros scoped aliases, reveal overlays, DNA metadata as applicable.
- `qa/archetype-qa/engine-gaps.md`: `G-FAMILIAR-TOKEN-ON-DELETION` and any ACE/reveal entries proven by this group.
- `qa/dsl-vocab-gaps.md`: BG Imperial DNA metadata entry and scoped alias vocabulary entry if present.

Each tracker update must include:

- The date of closure.
- The exact passing command.
- Any residual limits, such as remaining card-specific raw Rust bodies or unrelated timing gaps.

- [ ] **Step 4: Self-review**

Review the diff for:

- No player-visible choice is auto-selected.
- No illegal action can be selected through masks or decoder.
- No DigiXros alias leaks into generic name predicates.
- No reveal overlay survives after leaving reveal.
- No ACE Overflow penalty applies twice for the same moved source.
- No `ACTION_SPACE_SIZE` or `TENSOR_SIZE` change is undocumented.
- No new production imports from `engine_py_legacy`.

Run:

```bash
git diff --check
git status --short
```

- [ ] **Step 5: Commit**

```bash
git add code/digimon-engine code/digimon-dsl docs/RUST_ENGINE_API.md docs/RUST_ENGINE_GAPS.md qa/archetype-qa qa/dsl-vocab-gaps.md
git commit -m "engine: complete token card data gaps"
```

---

## Completion Criteria

Group 8 is complete when:

- Familiar Token deletion always routes the opponent Digimon choice through `PendingSelection` and applies `-3000 DP for the turn`.
- Token registry invariants prove all tokens synthesize real `CardKind::Token` card data and registered token effects.
- Authored `alt_paths: kind: dna_digivolve` metadata makes the normal DNA action legal in masks without changing action-space size.
- DigiXros aliases are visible only to DigiXros material matching.
- ACE Overflow metadata reaches runtime `CardData` and applies exactly once when ACE cards leave from field or under-card states.
- Reveal-zone overlays are scoped to reveal-zone predicate checks and cleared on zone movement.
- Trackers are updated with commands from passing tests.
