# Arts Digivolve and DUAL Cards Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add first-class DUAL card support and Arts Digivolve to the Rust engine so DUAL cards can be used as Options, digivolved as Digimon, and optionally stacked after Option use through action-mask-visible choices.

**Architecture:** Represent DUAL cards as `CardKind::Dual` plus explicit `DualCardData` faces. Route DUAL play bits through the existing Option pipeline with an `OptionUseSource`, and add an Arts selection/execution branch before normal Standard Option trash disposal. Keep all user choices in `PendingSelection`; keep DUAL face semantics behind helper methods rather than scattering `CardKind::Dual` checks.

**Tech Stack:** Rust workspace, `digimon-engine`, `digimon-dsl`, Cargo integration tests, existing `DebugRunner`, existing action mask/decoder, existing Option flow.

---

## File Structure

Core engine data:

- `code/digimon-engine/src/enums.rs` - add `CardKind::Dual` and `Keyword::ArtsDigivolve`.
- `code/digimon-engine/src/card_data.rs` - add DUAL face structs, deserialization, keyword parsing, face-aware metadata helpers.
- `code/digimon-engine/src/card_source.rs` - add face-aware instance helpers that delegate to `CardData`.
- `code/digimon-engine/src/permanent.rs` - make field identity treat a DUAL top card as a Digimon.
- `code/digimon-engine/src/debug_runner.rs` - add `dual: None` to synthetic `CardData` constructors and compiled-card conversion.

Action and flow:

- `code/digimon-engine/src/action/mask.rs` - make play/digivolve masks DUAL-aware and expose `BREEDING_SELECTION_TARGET`.
- `code/digimon-engine/src/action/decode.rs` - route DUAL play bits through Option use.
- `code/digimon-engine/src/selection.rs` - add `OptionUseSource`, extend `PendingOption`, add Arts phases.
- `code/digimon-engine/src/game_actions.rs` - add DUAL Option use wrappers, Arts eligibility, selection install, Arts execution primitive, DUAL-aware digivolution cost calculation.
- `code/digimon-engine/src/effect_queue.rs` - resume pending Options into Arts selection before normal disposal.

Tests:

- `code/digimon-engine/tests/dual_cards/main.rs` - new integration test crate.
- `code/digimon-engine/tests/dual_cards/data_model.rs` - DUAL parsing/helper tests.
- `code/digimon-engine/tests/dual_cards/mask_and_use.rs` - DUAL play/digivolve mask and Option-use tests.
- `code/digimon-engine/tests/dual_cards/arts_flow.rs` - Arts decline/accept/breeding/timing tests.
- `code/digimon-engine/tests/option_flow/main.rs` - add `mod dual_regression;` if any Option-flow regression belongs beside existing Option tests.

Importer/DSL follow-up:

- `code/digimon-dsl/src/spec.rs`, `code/digimon-dsl/src/compiled.rs`, `code/digimon-dsl/src/compile.rs` - add DSL DUAL metadata after engine substrate is green.
- `code/digimon-engine/src/debug_runner.rs` - update DSL compiled card conversion once `digimon-dsl` can emit DUAL cards.
- `code/tools/ingest_cards.py` - map live digimoncard.io `type: "Dual"` rows into the explicit Rust JSON shape if this tool currently owns `data/cards.json` generation.
- `docs/TENSOR_SPEC.md` and `docs/ACTION_SPEC.md` - update only if the implementation changes tensor-encoded card-kind values or selection constants.

## Task 1: Add DUAL Data Model and Keyword Parsing

**Files:**
- Modify: `code/digimon-engine/src/enums.rs`
- Modify: `code/digimon-engine/src/card_data.rs`
- Modify: `code/digimon-engine/src/debug_runner.rs`
- Test: `code/digimon-engine/src/card_data.rs`

- [ ] **Step 1: Write failing card-data tests**

Add these tests inside the existing `#[cfg(test)] mod tests` in `code/digimon-engine/src/card_data.rs`:

```rust
#[test]
fn parses_dual_card_payload() {
    let json = r#"{
        "DUAL-001": {
            "card_id": "DUAL-001",
            "card_name_eng": "Dual Test",
            "card_kind": 4,
            "play_cost": 5,
            "dp": 12000,
            "level": 6,
            "card_colors": [0, 2],
            "type_eng": ["TestTrait"],
            "form_eng": ["Mega"],
            "attribute_eng": ["Vaccine"],
            "effect_description_eng": "＜Raid＞\\n[When Digivolving] Draw 1.",
            "inherited_effect_description_eng": "",
            "security_effect_description_eng": "",
            "evo_costs": [{"card_color": 0, "level": 5, "memory_cost": 3}],
            "dual": {
                "digimon": {
                    "level": 6,
                    "dp": 12000,
                    "colors": ["Red", "Yellow"],
                    "traits": ["Mega", "Vaccine", "TestTrait"],
                    "evo_costs": [{"card_color": 0, "level": 5, "memory_cost": 3}],
                    "effect_text": "＜Raid＞\\n[When Digivolving] Draw 1.",
                    "inherited_text": ""
                },
                "option": {
                    "use_cost": 5,
                    "colors": ["Purple"],
                    "effect_text": "Use Requirement: TestTrait trait\\n[Main] Delete 1 Digimon.",
                    "security_text": ""
                }
            }
        }
    }"#;

    let cards = CardData::load_from_str(json).expect("dual card parses");
    let card = cards.get("DUAL-001").expect("card exists");
    assert_eq!(card.card_kind, CardKind::Dual);
    assert_eq!(card.level, Some(6));
    assert_eq!(card.dp, Some(12000));
    assert_eq!(card.play_cost, 5);
    assert!(card.dual.is_some());
    let dual = card.dual.as_ref().unwrap();
    assert_eq!(dual.option.use_cost, 5);
    assert_eq!(dual.option.colors, vec![CardColor::Purple]);
    assert_eq!(dual.digimon.colors, vec![CardColor::Red, CardColor::Yellow]);
}

#[test]
fn parses_arts_digivolve_keyword() {
    let kws = parse_printed_keywords("＜Arts Digivolve＞", "", "");
    assert!(kws.contains(&Keyword::ArtsDigivolve));
}
```

- [ ] **Step 2: Run the failing tests**

Run:

```powershell
cargo test -p digimon-engine card_data::tests::parses_dual_card_payload -- --nocapture
cargo test -p digimon-engine card_data::tests::parses_arts_digivolve_keyword -- --nocapture
```

Expected: compile failure mentioning missing `CardKind::Dual`, missing `CardData::dual`, missing DUAL structs, or missing `Keyword::ArtsDigivolve`.

- [ ] **Step 3: Add enum variants**

In `code/digimon-engine/src/enums.rs`, add:

```rust
pub enum CardKind {
    Digimon,
    Tamer,
    Option,
    DigiEgg,
    Token,
    Dual,
}
```

and in `Keyword` near the other printed keywords:

```rust
ArtsDigivolve,
```

- [ ] **Step 4: Add DUAL structs and field**

In `code/digimon-engine/src/card_data.rs`, add these structs near `CardData`:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DualCardData {
    pub digimon: DualDigimonFace,
    pub option: DualOptionFace,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DualDigimonFace {
    pub level: u8,
    pub dp: i32,
    pub colors: Vec<CardColor>,
    pub traits: Vec<String>,
    pub evo_costs: Vec<EvoCost>,
    #[serde(default)]
    pub effect_text: String,
    #[serde(default)]
    pub inherited_text: String,
    #[serde(default)]
    pub keywords: Vec<crate::enums::Keyword>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DualOptionFace {
    pub use_cost: u16,
    pub colors: Vec<CardColor>,
    #[serde(default)]
    pub effect_text: String,
    #[serde(default)]
    pub security_text: String,
    #[serde(default)]
    pub keywords: Vec<crate::enums::Keyword>,
}
```

Add to `CardData`:

```rust
#[serde(default, skip_serializing_if = "Option::is_none")]
pub dual: Option<DualCardData>,
```

Add to `RawCard`:

```rust
#[serde(default)]
dual: Option<DualCardData>,
```

- [ ] **Step 5: Wire parsing**

Update `parse_card_kind`:

```rust
fn parse_card_kind(raw: u8) -> CardKind {
    match raw {
        0 => CardKind::Digimon,
        1 => CardKind::Tamer,
        2 => CardKind::Option,
        3 => CardKind::DigiEgg,
        4 => CardKind::Dual,
        _ => CardKind::Digimon,
    }
}
```

Update `CardData` construction:

```rust
dual: raw_card.dual,
```

Update `parse_printed_keywords` non-parametric keyword table to include:

```rust
("Arts Digivolve", Keyword::ArtsDigivolve),
```

- [ ] **Step 6: Update synthetic constructors**

In `code/digimon-engine/src/debug_runner.rs`, add `dual: None,` to every `CardData { ... }` literal:

- `card_data_from_compiled`
- `make_test_card`
- `make_test_egg`

Example:

```rust
CardData {
    card_id: card_id.to_string(),
    // existing fields...
    norm_id: 0.0,
    dual: None,
}
```

- [ ] **Step 7: Run data model tests**

Run:

```powershell
cargo test -p digimon-engine card_data::tests::parses_dual_card_payload -- --nocapture
cargo test -p digimon-engine card_data::tests::parses_arts_digivolve_keyword -- --nocapture
```

Expected: both tests pass.

- [ ] **Step 8: Commit**

```powershell
git add code/digimon-engine/src/enums.rs code/digimon-engine/src/card_data.rs code/digimon-engine/src/debug_runner.rs
git commit -m "feat(engine): add dual card metadata"
```

## Task 2: Add Face-Aware Card Helpers

**Files:**
- Modify: `code/digimon-engine/src/card_data.rs`
- Modify: `code/digimon-engine/src/card_source.rs`
- Modify: `code/digimon-engine/src/permanent.rs`
- Test: `code/digimon-engine/tests/dual_cards/main.rs`
- Test: `code/digimon-engine/tests/dual_cards/data_model.rs`

- [ ] **Step 1: Create dual_cards test crate**

Create `code/digimon-engine/tests/dual_cards/main.rs`:

```rust
mod data_model;
mod mask_and_use;
mod arts_flow;
```

Create `code/digimon-engine/tests/dual_cards/data_model.rs`:

```rust
use digimon_engine::card_data::{CardData, DualCardData, DualDigimonFace, DualOptionFace, EvoCost};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind};

fn base_lv5(card_id: &str, color: CardColor) -> CardData {
    let mut card = make_test_card(card_id, card_id);
    card.level = Some(5);
    card.dp = Some(7000);
    card.colors = vec![color];
    card
}

fn dual_card() -> CardData {
    let mut card = make_test_card("DUAL-HELPER", "Dual Helper");
    card.card_kind = CardKind::Dual;
    card.level = Some(6);
    card.dp = Some(12000);
    card.play_cost = 5;
    card.colors = vec![CardColor::Red];
    card.traits = vec!["DigimonTrait".to_string()];
    card.evo_costs = vec![EvoCost {
        card_color: CardColor::Red as u8,
        level: 5,
        memory_cost: 3,
    }];
    card.dual = Some(DualCardData {
        digimon: DualDigimonFace {
            level: 6,
            dp: 12000,
            colors: vec![CardColor::Red],
            traits: vec!["DigimonTrait".to_string()],
            evo_costs: card.evo_costs.clone(),
            effect_text: "[When Digivolving] Draw 1.".to_string(),
            inherited_text: String::new(),
            keywords: Vec::new(),
        },
        option: DualOptionFace {
            use_cost: 5,
            colors: vec![CardColor::Purple],
            effect_text: "Use Requirement: Test trait\n[Main] Delete 1 Digimon.".to_string(),
            security_text: String::new(),
            keywords: Vec::new(),
        },
    });
    card
}

#[test]
fn dual_card_helpers_expose_separate_faces() {
    let r = DebugRunner::builder()
        .add_card(base_lv5("BASE", CardColor::Red))
        .add_card(dual_card())
        .hand(0, &["DUAL-HELPER"])
        .start();

    let card = &r.game.player(0).hand[0];
    assert_eq!(card.card_kind(&r.game.card_data), CardKind::Dual);
    assert_eq!(card.digimon_level(&r.game.card_data), Some(6));
    assert_eq!(card.digimon_dp(&r.game.card_data), Some(12000));
    assert_eq!(card.option_use_cost(&r.game.card_data), Some(5));
    assert_eq!(card.digimon_colors(&r.game.card_data), &[CardColor::Red]);
    assert_eq!(card.option_colors(&r.game.card_data), &[CardColor::Purple]);
    assert!(card.is_digimon_card_for_search(&r.game.card_data));
    assert!(card.is_option_card_for_search(&r.game.card_data));
    assert!(card
        .text_for_search_all_faces(&r.game.card_data)
        .contains("Delete 1 Digimon"));
}

#[test]
fn dual_on_field_is_digimon_not_option() {
    let mut r = DebugRunner::builder()
        .add_card(base_lv5("BASE", CardColor::Red))
        .add_card(dual_card())
        .start();
    let h = r.place_on_field(0, "DUAL-HELPER", Some(0));
    let perm = &r.game.player(0).battle_area[h.index as usize];
    assert!(perm.is_digimon(&r.game.card_data));
    assert!(!perm.is_option(&r.game.card_data));
}
```

- [ ] **Step 2: Run the failing helper tests**

Run:

```powershell
cargo test -p digimon-engine --test dual_cards data_model -- --nocapture
```

Expected: compile failure for missing helper methods and `Permanent::is_option`.

- [ ] **Step 3: Add `CardData` helper methods**

In `code/digimon-engine/src/card_data.rs`, add:

```rust
impl CardData {
    pub fn is_digimon_card_for_search(&self) -> bool {
        matches!(self.card_kind, CardKind::Digimon | CardKind::DigiEgg | CardKind::Dual)
    }

    pub fn is_option_card_for_search(&self) -> bool {
        matches!(self.card_kind, CardKind::Option | CardKind::Dual)
    }

    pub fn digimon_level(&self) -> Option<u8> {
        self.dual.as_ref().map(|d| d.digimon.level).or(self.level)
    }

    pub fn digimon_dp(&self) -> Option<i32> {
        self.dual.as_ref().map(|d| d.digimon.dp).or(self.dp)
    }

    pub fn digimon_colors(&self) -> &[CardColor] {
        self.dual
            .as_ref()
            .map(|d| d.digimon.colors.as_slice())
            .unwrap_or(self.colors.as_slice())
    }

    pub fn option_colors(&self) -> &[CardColor] {
        self.dual
            .as_ref()
            .map(|d| d.option.colors.as_slice())
            .unwrap_or(self.colors.as_slice())
    }

    pub fn option_use_cost(&self) -> Option<u16> {
        match self.card_kind {
            CardKind::Option => Some(self.play_cost),
            CardKind::Dual => self.dual.as_ref().map(|d| d.option.use_cost),
            _ => None,
        }
    }

    pub fn digivolution_costs(&self) -> &[EvoCost] {
        self.dual
            .as_ref()
            .map(|d| d.digimon.evo_costs.as_slice())
            .unwrap_or(self.evo_costs.as_slice())
    }

    pub fn text_for_search_all_faces(&self) -> String {
        if let Some(dual) = &self.dual {
            return [
                dual.digimon.effect_text.as_str(),
                dual.digimon.inherited_text.as_str(),
                dual.option.effect_text.as_str(),
                dual.option.security_text.as_str(),
            ]
            .join("\n");
        }
        [
            self.effect_text.as_str(),
            self.inherited_text.as_str(),
            self.security_text.as_str(),
        ]
        .join("\n")
    }
}
```

- [ ] **Step 4: Add `CardSource` helper methods**

In `code/digimon-engine/src/card_source.rs`, add:

```rust
pub fn is_digimon_card_for_search(&self, data: &[CardData]) -> bool {
    data[self.data_index].is_digimon_card_for_search()
}

pub fn is_option_card_for_search(&self, data: &[CardData]) -> bool {
    data[self.data_index].is_option_card_for_search()
}

pub fn digimon_level(&self, data: &[CardData]) -> Option<u8> {
    data[self.data_index].digimon_level()
}

pub fn digimon_dp(&self, data: &[CardData]) -> Option<i32> {
    data[self.data_index].digimon_dp()
}

pub fn digimon_colors<'a>(&self, data: &'a [CardData]) -> &'a [CardColor] {
    data[self.data_index].digimon_colors()
}

pub fn option_colors<'a>(&self, data: &'a [CardData]) -> &'a [CardColor] {
    data[self.data_index].option_colors()
}

pub fn option_use_cost(&self, data: &[CardData]) -> Option<u16> {
    data[self.data_index].option_use_cost()
}

pub fn digivolution_costs<'a>(&self, data: &'a [CardData]) -> &'a [crate::card_data::EvoCost] {
    data[self.data_index].digivolution_costs()
}

pub fn text_for_search_all_faces(&self, data: &[CardData]) -> String {
    data[self.data_index].text_for_search_all_faces()
}
```

- [ ] **Step 5: Add field identity helpers on `Permanent`**

In `code/digimon-engine/src/permanent.rs`, update:

```rust
pub fn is_digimon(&self, data: &[CardData]) -> bool {
    matches!(self.top_card().card_kind(data), CardKind::Digimon | CardKind::Dual)
}
```

Add:

```rust
pub fn is_option(&self, data: &[CardData]) -> bool {
    self.top_card().card_kind(data) == CardKind::Option
}
```

Leave `is_tamer` and `is_digi_egg` unchanged.

- [ ] **Step 6: Run helper tests**

Run:

```powershell
cargo test -p digimon-engine --test dual_cards data_model -- --nocapture
```

Expected: tests pass.

- [ ] **Step 7: Commit**

```powershell
git add code/digimon-engine/src/card_data.rs code/digimon-engine/src/card_source.rs code/digimon-engine/src/permanent.rs code/digimon-engine/tests/dual_cards
git commit -m "feat(engine): add dual face helpers"
```

## Task 3: Make Action Mask and Decoder DUAL-Aware

**Files:**
- Modify: `code/digimon-engine/src/action/mask.rs`
- Modify: `code/digimon-engine/src/action/decode.rs`
- Modify: `code/digimon-engine/src/game.rs`
- Modify: `code/digimon-engine/src/game_actions.rs`
- Test: `code/digimon-engine/tests/dual_cards/mask_and_use.rs`

- [ ] **Step 1: Add mask tests**

Create `code/digimon-engine/tests/dual_cards/mask_and_use.rs`:

```rust
use std::sync::Arc;

use digimon_engine::action::mask::build_action_mask;
use digimon_engine::card_data::{CardData, DualCardData, DualDigimonFace, DualOptionFace, EvoCost};
use digimon_engine::card_source::CardHandle;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::effect::{CardEffect, Effect};
use digimon_engine::enums::{CardColor, CardKind};
use digimon_engine::selection::OptionPlayResult;

fn advance_to_main(r: &mut DebugRunner) {
    r.game.enter_main_phase();
}

fn base_lv5(card_id: &str, color: CardColor) -> CardData {
    let mut card = make_test_card(card_id, card_id);
    card.level = Some(5);
    card.dp = Some(7000);
    card.colors = vec![color];
    card
}

fn color_anchor(card_id: &str, color: CardColor) -> CardData {
    let mut card = make_test_card(card_id, card_id);
    card.level = Some(3);
    card.colors = vec![color];
    card
}

fn dual_card() -> CardData {
    let mut card = make_test_card("DUAL-MASK", "Dual Mask");
    card.card_kind = CardKind::Dual;
    card.level = Some(6);
    card.dp = Some(12000);
    card.play_cost = 5;
    card.colors = vec![CardColor::Red];
    card.traits = vec!["DigimonTrait".to_string()];
    card.evo_costs = vec![EvoCost {
        card_color: CardColor::Red as u8,
        level: 5,
        memory_cost: 3,
    }];
    card.dual = Some(DualCardData {
        digimon: DualDigimonFace {
            level: 6,
            dp: 12000,
            colors: vec![CardColor::Red],
            traits: vec!["DigimonTrait".to_string()],
            evo_costs: card.evo_costs.clone(),
            effect_text: "[When Digivolving] Draw 1.".to_string(),
            inherited_text: String::new(),
            keywords: Vec::new(),
        },
        option: DualOptionFace {
            use_cost: 5,
            colors: vec![CardColor::Purple],
            effect_text: "[Main] Gain 2 memory.".to_string(),
            security_text: String::new(),
            keywords: Vec::new(),
        },
    });
    card
}

struct GainTwo;
impl CardEffect for GainTwo {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::on_play(card)
            .name("Dual option main")
            .option_main()
            .process(|ctx| ctx.gain_memory(2))
            .build()]
    }
}

#[test]
fn dual_play_bit_uses_option_face_color_and_cost() {
    let mut r = DebugRunner::builder()
        .add_card(dual_card())
        .add_card(color_anchor("PURPLE-ANCHOR", CardColor::Purple))
        .hand(0, &["DUAL-MASK"])
        .memory(5)
        .start();
    r.register_effect("DUAL-MASK", Arc::new(GainTwo));
    r.place_on_field(0, "PURPLE-ANCHOR", Some(0));
    advance_to_main(&mut r);

    let mask = build_action_mask(&r.game, 0);
    assert_eq!(mask[0], 1.0, "DUAL play bit means use as Option");

    let result = r.game.decode_action(0, 0);
    assert!(result.is_ok());
    assert_eq!(r.hand_size(0), 0);
    assert_eq!(r.trash_size(0), 1);
    assert_eq!(r.memory(), 2, "paid 5 from 5, then gained 2");
}

#[test]
fn dual_option_use_does_not_accept_digimon_face_color() {
    let mut r = DebugRunner::builder()
        .add_card(dual_card())
        .add_card(color_anchor("RED-ANCHOR", CardColor::Red))
        .hand(0, &["DUAL-MASK"])
        .memory(5)
        .start();
    r.register_effect("DUAL-MASK", Arc::new(GainTwo));
    r.place_on_field(0, "RED-ANCHOR", Some(0));
    advance_to_main(&mut r);

    let mask = build_action_mask(&r.game, 0);
    assert_eq!(mask[0], 0.0, "Digimon-face red must not satisfy purple Option face");
    assert_eq!(r.game.play_option_from_hand(0, 0), OptionPlayResult::Invalid);
}

#[test]
fn dual_emits_digivolve_bit_using_digimon_face() {
    use digimon_engine::action::space::encode_digivolve;

    let mut r = DebugRunner::builder()
        .add_card(dual_card())
        .add_card(base_lv5("BASE-RED", CardColor::Red))
        .hand(0, &["DUAL-MASK"])
        .memory(5)
        .start();
    r.place_on_field(0, "BASE-RED", Some(0));
    advance_to_main(&mut r);

    let mask = build_action_mask(&r.game, 0);
    let bit = encode_digivolve(0, 0) as usize;
    assert_eq!(mask[bit], 1.0, "DUAL can digivolve as a Digimon");
}
```

- [ ] **Step 2: Run failing mask tests**

Run:

```powershell
cargo test -p digimon-engine --test dual_cards mask_and_use -- --nocapture
```

Expected: failures showing DUAL does not yet emit play/digivolve bits and `decode_action` does not route DUAL.

- [ ] **Step 3: Add Option-use source enum and pending field**

In `code/digimon-engine/src/selection.rs`, add:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptionUseSource {
    UsedFromHand,
    UsedFromTrash,
    UsedFromSecurity,
    DirectMainActivation,
}
```

Extend `PendingOption`:

```rust
pub struct PendingOption {
    pub owner: PlayerId,
    pub card: CardSource,
    pub resolution_phase: OptionResolutionPhase,
    pub source_kind: OptionUseSource,
}
```

Update all existing `PendingOption { ... }` literals in `game_actions.rs` to include `source_kind`.

- [ ] **Step 4: Extend OptionSource and play wrappers**

In `code/digimon-engine/src/game_actions.rs`, change:

```rust
enum OptionSource {
    Hand(usize),
    Trash(usize),
}
```

to:

```rust
enum OptionSource {
    Hand(usize),
    Trash(usize),
}

impl OptionSource {
    fn use_source(&self) -> crate::selection::OptionUseSource {
        match self {
            OptionSource::Hand(_) => crate::selection::OptionUseSource::UsedFromHand,
            OptionSource::Trash(_) => crate::selection::OptionUseSource::UsedFromTrash,
        }
    }
}
```

Keep public `play_option_from_hand` and `play_option_from_trash`; both delegate to `play_option_core`.

When installing `PendingOption`, set:

```rust
source_kind: source.use_source(),
```

- [ ] **Step 5: Make Option validation accept DUAL**

In `play_option_core`, replace the card-kind gate:

```rust
if card.card_kind(&self.card_data) != CardKind::Option {
    return OptionPlayResult::Invalid;
}
```

with:

```rust
if !matches!(card.card_kind(&self.card_data), CardKind::Option | CardKind::Dual) {
    return OptionPlayResult::Invalid;
}
```

Replace printed cost lookup:

```rust
card.play_cost(&self.card_data)
```

with:

```rust
card.option_use_cost(&self.card_data).unwrap_or(card.play_cost(&self.card_data))
```

- [ ] **Step 6: Make mask Option-color checks use Option face**

In `code/digimon-engine/src/action/mask.rs`, update main play loop:

```rust
let cost = if matches!(card.card_kind(&game.card_data), CardKind::Option | CardKind::Dual) {
    card.option_use_cost(&game.card_data)
        .unwrap_or(card.play_cost(&game.card_data)) as i16
} else {
    card.play_cost(&game.card_data) as i16
};
```

Update the Option color gate:

```rust
if matches!(card.card_kind(&game.card_data), CardKind::Option | CardKind::Dual) {
    if !option_color_match_available(card, me, &game.card_data) {
        continue;
    }
}
```

Update `option_color_match_available`:

```rust
let option_colors = card.option_colors(card_data);
```

- [ ] **Step 7: Make digivolve mask accept DUAL**

In the digivolve loop in `action/mask.rs`, replace:

```rust
if card.card_kind(&game.card_data) != CardKind::Digimon {
    continue;
}
```

with:

```rust
if !matches!(card.card_kind(&game.card_data), CardKind::Digimon | CardKind::Dual) {
    continue;
}
```

Update `can_basic_digivolve` to read `card_meta.digivolution_costs()` and `base.top_card().digimon_level(...)` / `digimon_colors(...)`.

- [ ] **Step 8: Make decoder route DUAL as Option use**

In `code/digimon-engine/src/action/decode.rs`, update:

```rust
match card_kind {
    Some(CardKind::Option) | Some(CardKind::Dual) => {
        let _ = self.play_option_from_hand(tp, hand_idx);
    }
    Some(CardKind::Digimon) | Some(CardKind::Tamer) => {
        let _ = self.play_from_hand(tp, hand_idx);
    }
    _ => {}
}
```

- [ ] **Step 9: Make `Game::can_digivolve` DUAL-aware**

In `code/digimon-engine/src/game.rs`, update `can_digivolve` to use:

```rust
let Some(base_level) = perm.top_card().digimon_level(&self.card_data) else {
    return false;
};
let base_colors = perm.top_card().digimon_colors(&self.card_data);
let evo_costs = card.digivolution_costs(&self.card_data);
```

- [ ] **Step 10: Update digivolve cost extraction in game actions**

In both `digivolve_from_hand` and `digivolve_from_hand_onto_breeding`, replace direct reads of `self.card_data[card.data_index].evo_costs` with:

```rust
let evo_costs = card.digivolution_costs(&self.card_data);
```

and replace base level/colors reads with `digimon_level` / `digimon_colors`.

- [ ] **Step 11: Run mask/use tests**

Run:

```powershell
cargo test -p digimon-engine --test dual_cards mask_and_use -- --nocapture
```

Expected: tests pass.

- [ ] **Step 12: Run existing Option flow tests**

Run:

```powershell
cargo test -p digimon-engine --test option_flow
```

Expected: all existing Option flow tests pass.

- [ ] **Step 13: Commit**

```powershell
git add code/digimon-engine/src/action/mask.rs code/digimon-engine/src/action/decode.rs code/digimon-engine/src/game.rs code/digimon-engine/src/game_actions.rs code/digimon-engine/src/selection.rs code/digimon-engine/tests/dual_cards/mask_and_use.rs
git commit -m "feat(engine): route dual cards through option and digivolve actions"
```

## Task 4: Add Arts Eligibility and Decline Flow

**Files:**
- Modify: `code/digimon-engine/src/selection.rs`
- Modify: `code/digimon-engine/src/game_actions.rs`
- Modify: `code/digimon-engine/src/effect_queue.rs`
- Test: `code/digimon-engine/tests/dual_cards/arts_flow.rs`

- [ ] **Step 1: Add Arts decline tests**

Create `code/digimon-engine/tests/dual_cards/arts_flow.rs` with the shared helpers and the first test:

```rust
use std::sync::{Arc, Mutex};

use digimon_engine::action::space::{encode_attack, PASS};
use digimon_engine::card_data::{CardData, DualCardData, DualDigimonFace, DualOptionFace, EvoCost};
use digimon_engine::card_source::CardHandle;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::effect::{CardEffect, Effect};
use digimon_engine::enums::{CardColor, CardKind, Keyword};
use digimon_engine::selection::{OptionPlayResult, SelectionKind};

fn advance_to_main(r: &mut DebugRunner) {
    r.game.enter_main_phase();
}

fn base_lv5(card_id: &str, color: CardColor) -> CardData {
    let mut card = make_test_card(card_id, card_id);
    card.level = Some(5);
    card.dp = Some(7000);
    card.colors = vec![color];
    card
}

fn purple_anchor() -> CardData {
    let mut card = make_test_card("PURPLE-ANCHOR", "Purple Anchor");
    card.level = Some(3);
    card.colors = vec![CardColor::Purple];
    card
}

fn arts_dual() -> CardData {
    let mut card = make_test_card("DUAL-ARTS", "Dual Arts");
    card.card_kind = CardKind::Dual;
    card.level = Some(6);
    card.dp = Some(12000);
    card.play_cost = 5;
    card.colors = vec![CardColor::Red];
    card.evo_costs = vec![EvoCost {
        card_color: CardColor::Red as u8,
        level: 5,
        memory_cost: 3,
    }];
    card.dual = Some(DualCardData {
        digimon: DualDigimonFace {
            level: 6,
            dp: 12000,
            colors: vec![CardColor::Red],
            traits: vec!["DualTrait".to_string()],
            evo_costs: card.evo_costs.clone(),
            effect_text: "[When Digivolving] Draw 1.".to_string(),
            inherited_text: String::new(),
            keywords: vec![Keyword::ArtsDigivolve],
        },
        option: DualOptionFace {
            use_cost: 5,
            colors: vec![CardColor::Purple],
            effect_text: "[Main] Gain 2 memory.".to_string(),
            security_text: String::new(),
            keywords: vec![Keyword::ArtsDigivolve],
        },
    });
    card
}

struct GainTwo;
impl CardEffect for GainTwo {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::on_play(card)
            .name("Dual Option Main")
            .option_main()
            .process(|ctx| ctx.gain_memory(2))
            .build()]
    }
}

#[test]
fn arts_prompt_installs_after_option_main_and_pass_declines_to_trash() {
    let mut r = DebugRunner::builder()
        .add_card(arts_dual())
        .add_card(base_lv5("BASE-RED", CardColor::Red))
        .add_card(purple_anchor())
        .hand(0, &["DUAL-ARTS"])
        .memory(5)
        .start();
    r.register_effect("DUAL-ARTS", Arc::new(GainTwo));
    r.place_on_field(0, "PURPLE-ANCHOR", Some(0));
    r.place_on_field(0, "BASE-RED", Some(0));
    advance_to_main(&mut r);

    let result = r.game.play_option_from_hand(0, 0);
    assert_eq!(result, OptionPlayResult::Pending);
    let sel = r.game.pending_selection.as_ref().expect("arts selection");
    assert_eq!(sel.kind, SelectionKind::OwnField);
    assert!(sel.is_optional, "PASS declines Arts");
    assert!(sel.valid_action_ids.contains(&encode_attack(0, 1)));

    r.game.resolve_selection(0, PASS).expect("decline arts");
    assert!(r.game.pending_option.is_none());
    assert!(r.game.pending_selection.is_none());
    assert_eq!(r.trash_size(0), 1, "declining Arts trashes normally");
    assert_eq!(r.battle_area_size(0), 2, "no Arts stack was created");
}
```

- [ ] **Step 2: Run the failing Arts decline test**

Run:

```powershell
cargo test -p digimon-engine --test dual_cards arts_prompt_installs_after_option_main_and_pass_declines_to_trash -- --nocapture
```

Expected: failure because Arts prompt and `Keyword::ArtsDigivolve` behavior are not wired.

- [ ] **Step 3: Add Arts resolution phases**

In `code/digimon-engine/src/selection.rs`, extend:

```rust
pub enum OptionResolutionPhase {
    LinkSelectHost,
    MainEffectDrain,
    ArtsSelectTarget,
    Disposing,
    Done,
}
```

- [ ] **Step 4: Add Arts eligibility helpers**

In `code/digimon-engine/src/game_actions.rs`, add inside `impl Game`:

```rust
fn pending_option_can_arts_digivolve(&self) -> bool {
    let Some(pending) = self.pending_option.as_ref() else {
        return false;
    };
    if pending.card.card_kind(&self.card_data) != CardKind::Dual {
        return false;
    }
    if pending.source_kind == crate::selection::OptionUseSource::DirectMainActivation {
        return false;
    }
    let data = &self.card_data[pending.card.data_index];
    data.dual
        .as_ref()
        .map(|dual| {
            dual.option.keywords.contains(&Keyword::ArtsDigivolve)
                || dual.digimon.keywords.contains(&Keyword::ArtsDigivolve)
                || data.keywords.contains(&Keyword::ArtsDigivolve)
        })
        .unwrap_or(false)
}
```

Ensure `Keyword` is imported at the top:

```rust
use crate::enums::{CardKind, EffectTiming, GamePhase, Keyword, ModifierType, PlaySource, PlayerId};
```

- [ ] **Step 5: Add Arts target discovery**

In `game_actions.rs`, add:

```rust
fn arts_digivolve_battle_targets(&self, owner: PlayerId) -> Vec<PermanentHandle> {
    let Some(pending) = self.pending_option.as_ref() else {
        return Vec::new();
    };
    let player = self.player(owner);
    player
        .battle_area
        .iter()
        .enumerate()
        .filter_map(|(i, perm)| {
            let handle = PermanentHandle {
                player: owner,
                index: i as u8,
            };
            if self.modifiers.has(handle, ModifierType::CannotDigivolve) {
                return None;
            }
            if self.can_digivolve(&pending.card, perm) {
                Some(handle)
            } else {
                None
            }
        })
        .collect()
}
```

- [ ] **Step 6: Add Arts selection installer**

In `game_actions.rs`, add:

```rust
fn install_arts_digivolve_selection(&mut self) -> bool {
    use crate::action::space::encode_attack;

    let Some(pending) = self.pending_option.as_ref() else {
        return false;
    };
    let owner = pending.owner;
    let source_card = pending.card.handle();
    let targets = self.arts_digivolve_battle_targets(owner);
    if targets.is_empty() {
        return false;
    }
    let valid_action_ids: Vec<u16> = targets
        .iter()
        .map(|h| encode_attack(0, h.index as u16))
        .collect();
    let target_snapshot = targets.clone();
    let previous_phase = self.current_phase;

    self.pending_option.as_mut().unwrap().resolution_phase =
        OptionResolutionPhase::ArtsSelectTarget;
    self.current_phase = GamePhase::SelectTarget;
    self.pending_selection = Some(PendingSelection {
        kind: SelectionKind::OwnField,
        selecting_player: owner,
        previous_phase,
        valid_action_ids,
        is_optional: true,
        prompt: "Choose a card for Arts Digivolve, or pass to trash this Option".to_string(),
        effect_choices: None,
        source_card,
        source_permanent: None,
        callback: Box::new(move |game: &mut Game, action_id: u16| {
            use crate::action::space::{ATTACK_START, TARGETS_PER_ATTACKER};
            let offset = action_id.saturating_sub(ATTACK_START);
            let target_index = (offset % TARGETS_PER_ATTACKER) as u8;
            if target_snapshot.iter().any(|h| h.index == target_index) {
                let target = PermanentHandle {
                    player: owner,
                    index: target_index,
                };
                let _ = game.arts_digivolve_pending_option_onto_battle(target);
            }
        }),
        on_decline: Some(Box::new(|game: &mut Game| {
            game.dispose_option();
            game.check_turn_end();
        })),
    });
    true
}
```

The `arts_digivolve_pending_option_onto_battle` function will be implemented in Task 5. For this task, add a stub that returns `false` so decline flow can compile:

```rust
pub(crate) fn arts_digivolve_pending_option_onto_battle(
    &mut self,
    _target: PermanentHandle,
) -> bool {
    false
}
```

- [ ] **Step 7: Re-enter Arts after OptionMain**

In `effect_queue.rs`, update `advance_pending_option` for `MainEffectDrain`:

```rust
crate::selection::OptionResolutionPhase::MainEffectDrain => {
    if self.pending_option_can_arts_digivolve() && self.install_arts_digivolve_selection() {
        return;
    }
    self.dispose_option();
    self.check_turn_end();
}
```

This requires making `pending_option_can_arts_digivolve` and `install_arts_digivolve_selection` `pub(crate)`.

In `play_option_core`, after `self.drain_effect_queue()` and before `self.dispose_option()`, add the same check:

```rust
if self.pending_option_can_arts_digivolve() && self.install_arts_digivolve_selection() {
    return OptionPlayResult::Pending;
}
```

- [ ] **Step 8: Run Arts decline test**

Run:

```powershell
cargo test -p digimon-engine --test dual_cards arts_prompt_installs_after_option_main_and_pass_declines_to_trash -- --nocapture
```

Expected: test passes.

- [ ] **Step 9: Run existing Option flow tests**

Run:

```powershell
cargo test -p digimon-engine --test option_flow
```

Expected: all existing Option flow tests pass.

- [ ] **Step 10: Commit**

```powershell
git add code/digimon-engine/src/selection.rs code/digimon-engine/src/game_actions.rs code/digimon-engine/src/effect_queue.rs code/digimon-engine/tests/dual_cards/arts_flow.rs
git commit -m "feat(engine): add arts digivolve decline flow"
```

## Task 5: Implement Arts Battle-Area Execution

**Files:**
- Modify: `code/digimon-engine/src/game_actions.rs`
- Test: `code/digimon-engine/tests/dual_cards/arts_flow.rs`

- [ ] **Step 1: Add Arts accept test**

Append to `arts_flow.rs`:

```rust
struct DrawOnDigivolve(Arc<Mutex<u32>>);
impl CardEffect for DrawOnDigivolve {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        let witness = self.0.clone();
        vec![
            Effect::on_play(card)
                .name("Dual Option Main")
                .option_main()
                .process(|ctx| ctx.gain_memory(2))
                .build(),
            Effect::when_digivolving(card)
                .name("When Digivolving witness")
                .process(move |_ctx| {
                    *witness.lock().unwrap() += 1;
                })
                .build(),
        ]
    }
}

#[test]
fn arts_accept_stacks_pending_dual_draws_and_fires_when_digivolving() {
    let witness = Arc::new(Mutex::new(0));
    let mut r = DebugRunner::builder()
        .add_card(arts_dual())
        .add_card(base_lv5("BASE-RED", CardColor::Red))
        .add_card(purple_anchor())
        .deck(0, &["PURPLE-ANCHOR"])
        .hand(0, &["DUAL-ARTS"])
        .memory(5)
        .start();
    r.register_effect("DUAL-ARTS", Arc::new(DrawOnDigivolve(witness.clone())));
    r.place_on_field(0, "PURPLE-ANCHOR", Some(0));
    r.place_on_field(0, "BASE-RED", Some(0));
    advance_to_main(&mut r);

    let result = r.game.play_option_from_hand(0, 0);
    assert_eq!(result, OptionPlayResult::Pending);
    let action_id = encode_attack(0, 1);
    r.game.resolve_selection(0, action_id).expect("accept Arts");

    assert!(r.game.pending_option.is_none());
    assert!(r.game.pending_selection.is_none());
    assert_eq!(r.trash_size(0), 0, "Arts prevents normal Option trash");
    assert_eq!(r.hand_size(0), 1, "digivolution bonus draw happened");
    let perm = &r.game.player(0).battle_area[1];
    assert_eq!(perm.stack_size(), 2);
    assert_eq!(perm.top_card().card_id(&r.game.card_data), "DUAL-ARTS");
    assert_eq!(*witness.lock().unwrap(), 1, "When Digivolving fired");
    assert_eq!(r.memory(), 2, "paid Option use cost, no digivolution cost");
}
```

- [ ] **Step 2: Run failing Arts accept test**

Run:

```powershell
cargo test -p digimon-engine --test dual_cards arts_accept_stacks_pending_dual_draws_and_fires_when_digivolving -- --nocapture
```

Expected: failure because the Arts execution stub returns false.

- [ ] **Step 3: Implement battle-area Arts execution**

Replace the stub in `game_actions.rs`:

```rust
pub(crate) fn arts_digivolve_pending_option_onto_battle(
    &mut self,
    target: PermanentHandle,
) -> bool {
    if !self.pending_option_can_arts_digivolve() {
        return false;
    }
    let Some(pending_ref) = self.pending_option.as_ref() else {
        return false;
    };
    if pending_ref.owner != target.player {
        return false;
    }
    let Some(perm) = self
        .player(target.player)
        .battle_area
        .get(target.index as usize)
    else {
        return false;
    };
    if self.modifiers.has(target, ModifierType::CannotDigivolve) {
        return false;
    }
    if !self.can_digivolve(&pending_ref.card, perm) {
        return false;
    }

    let pending = self.pending_option.take().expect("checked above");
    let turn = self.turn_count;
    self.player_mut(target.player).battle_area[target.index as usize]
        .digivolve(pending.card, turn);
    self.player_mut(target.player).draw();

    self.run_rule_check_after_arts();

    self.enqueue_triggered(EffectTiming::WhenDigivolving, TriggerSource::Permanent(target));
    self.drain_effect_queue();
    for pid in 0..self.players.len() {
        self.enqueue_triggered(
            EffectTiming::OnDigivolve,
            TriggerSource::PlayerBattleArea(pid as PlayerId),
        );
    }
    self.drain_effect_queue();
    self.check_turn_end();
    true
}
```

- [ ] **Step 4: Add minimal rule-check hook**

In `game_actions.rs`, add:

```rust
pub(crate) fn run_rule_check_after_arts(&mut self) {
    let mut to_delete: Vec<PermanentHandle> = Vec::new();
    for pid in 0..self.players.len() {
        for (idx, perm) in self.players[pid].battle_area.iter().enumerate() {
            let handle = PermanentHandle {
                player: pid as PlayerId,
                index: idx as u8,
            };
            if perm.is_digimon(&self.card_data) && self.effective_dp(handle).unwrap_or(1) <= 0 {
                to_delete.push(handle);
            }
        }
    }
    for handle in to_delete.into_iter().rev() {
        self.delete_permanent_with_effects(handle);
    }
}
```

This minimal hook covers the official Arts 0-DP rule-check example. Broader rule checks are outside this substrate plan.

- [ ] **Step 5: Run Arts accept test**

Run:

```powershell
cargo test -p digimon-engine --test dual_cards arts_accept_stacks_pending_dual_draws_and_fires_when_digivolving -- --nocapture
```

Expected: test passes.

- [ ] **Step 6: Run all dual cards tests**

Run:

```powershell
cargo test -p digimon-engine --test dual_cards -- --nocapture
```

Expected: all DUAL tests pass.

- [ ] **Step 7: Commit**

```powershell
git add code/digimon-engine/src/game_actions.rs code/digimon-engine/tests/dual_cards/arts_flow.rs
git commit -m "feat(engine): execute arts digivolve onto battle area"
```

## Task 6: Add Breeding-Area Arts Target Support

**Files:**
- Modify: `code/digimon-engine/src/action/space.rs`
- Modify: `code/digimon-engine/src/game_actions.rs`
- Test: `code/digimon-engine/tests/dual_cards/arts_flow.rs`
- Docs: `docs/ACTION_SPEC.md`

- [ ] **Step 1: Add breeding target test**

Append to `arts_flow.rs`:

```rust
#[test]
fn arts_can_target_legal_breeding_area_digimon() {
    use digimon_engine::action::space::BREEDING_SELECTION_TARGET;

    let mut r = DebugRunner::builder()
        .add_card(arts_dual())
        .add_card(base_lv5("BASE-RED", CardColor::Red))
        .add_card(purple_anchor())
        .deck(0, &["PURPLE-ANCHOR"])
        .hand(0, &["DUAL-ARTS"])
        .memory(5)
        .start();
    r.register_effect("DUAL-ARTS", Arc::new(GainTwo));
    r.place_on_field(0, "PURPLE-ANCHOR", Some(0));

    let data_idx = r
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == "BASE-RED")
        .unwrap();
    let next_idx = r.game.next_card_index();
    let base = digimon_engine::card_source::CardSource::new(data_idx, 0, next_idx);
    r.game.players[0].breeding_area = Some(digimon_engine::permanent::Permanent::new(base, 0));
    advance_to_main(&mut r);

    let result = r.game.play_option_from_hand(0, 0);
    assert_eq!(result, OptionPlayResult::Pending);
    let sel = r.game.pending_selection.as_ref().expect("arts selection");
    assert!(
        sel.valid_action_ids.contains(&BREEDING_SELECTION_TARGET),
        "breeding target appears as legal Arts target"
    );

    r.game
        .resolve_selection(0, BREEDING_SELECTION_TARGET)
        .expect("accept breeding Arts");
    let breeding = r.game.player(0).breeding_area.as_ref().expect("breeding remains");
    assert_eq!(breeding.stack_size(), 2);
    assert_eq!(breeding.top_card().card_id(&r.game.card_data), "DUAL-ARTS");
    assert_eq!(r.hand_size(0), 1, "bonus draw happened");
    assert_eq!(r.trash_size(0), 0);
}
```

- [ ] **Step 2: Run failing breeding test**

Run:

```powershell
cargo test -p digimon-engine --test dual_cards arts_can_target_legal_breeding_area_digimon -- --nocapture
```

Expected: compile failure for missing `BREEDING_SELECTION_TARGET` or runtime failure because breeding target is not installed.

- [ ] **Step 3: Add breeding selection constant**

In `code/digimon-engine/src/action/space.rs`, add:

```rust
/// Selection-only action id for the controller's breeding-area permanent.
/// `docs/ACTION_SPEC.md` already reserves 99 for this convention.
pub const BREEDING_SELECTION_TARGET: u16 = 99;
```

- [ ] **Step 4: Add breeding target discovery**

In `game_actions.rs`, add:

```rust
fn arts_digivolve_has_breeding_target(&self, owner: PlayerId) -> bool {
    let Some(pending) = self.pending_option.as_ref() else {
        return false;
    };
    let Some(breeding) = self.player(owner).breeding_area.as_ref() else {
        return false;
    };
    self.can_digivolve(&pending.card, breeding)
}
```

Update `install_arts_digivolve_selection`:

```rust
let has_breeding = self.arts_digivolve_has_breeding_target(owner);
if targets.is_empty() && !has_breeding {
    return false;
}
let mut valid_action_ids: Vec<u16> = targets
    .iter()
    .map(|h| encode_attack(0, h.index as u16))
    .collect();
if has_breeding {
    valid_action_ids.push(crate::action::space::BREEDING_SELECTION_TARGET);
}
```

In the callback, before battle target decoding:

```rust
if action_id == crate::action::space::BREEDING_SELECTION_TARGET {
    let _ = game.arts_digivolve_pending_option_onto_breeding(owner);
    return;
}
```

- [ ] **Step 5: Add breeding Arts execution**

In `game_actions.rs`, add:

```rust
pub(crate) fn arts_digivolve_pending_option_onto_breeding(
    &mut self,
    owner: PlayerId,
) -> bool {
    if !self.pending_option_can_arts_digivolve() {
        return false;
    }
    let Some(pending_ref) = self.pending_option.as_ref() else {
        return false;
    };
    if pending_ref.owner != owner {
        return false;
    }
    let Some(breeding) = self.player(owner).breeding_area.as_ref() else {
        return false;
    };
    if !self.can_digivolve(&pending_ref.card, breeding) {
        return false;
    }

    let pending = self.pending_option.take().expect("checked above");
    let turn = self.turn_count;
    if let Some(breeding) = self.player_mut(owner).breeding_area.as_mut() {
        breeding.digivolve(pending.card, turn);
    }
    self.player_mut(owner).draw();
    self.check_turn_end();
    true
}
```

This follows the existing Rust rule for breeding digivolution: draw happens, `WhenDigivolving` does not fire from breeding.

- [ ] **Step 6: Update action spec**

In `docs/ACTION_SPEC.md`, ensure the selection convention table includes:

```markdown
| `99` | Own breeding permanent |
```

If the row already exists, add a note under Arts/selection conventions:

```markdown
Arts Digivolve uses `99` for the controller's breeding-area target during its optional selection prompt.
```

- [ ] **Step 7: Run breeding test**

Run:

```powershell
cargo test -p digimon-engine --test dual_cards arts_can_target_legal_breeding_area_digimon -- --nocapture
```

Expected: test passes.

- [ ] **Step 8: Commit**

```powershell
git add code/digimon-engine/src/action/space.rs code/digimon-engine/src/game_actions.rs code/digimon-engine/tests/dual_cards/arts_flow.rs docs/ACTION_SPEC.md
git commit -m "feat(engine): allow arts digivolve onto breeding"
```

## Task 7: Cover Direct OptionMain Activation Exclusion

**Files:**
- Modify: `code/digimon-engine/tests/dual_cards/arts_flow.rs`
- Modify: `code/digimon-engine/src/game_actions.rs`

- [ ] **Step 1: Add direct activation regression test**

Append to `arts_flow.rs`:

```rust
struct HandMainDirect;
impl CardEffect for HandMainDirect {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::on_play(card)
            .name("Direct hand main")
            .timing(digimon_engine::enums::EffectTiming::MainFromHand)
            .process(|ctx| ctx.gain_memory(2))
            .build()]
    }
}

#[test]
fn direct_hand_main_activation_does_not_enable_arts() {
    let mut r = DebugRunner::builder()
        .add_card(arts_dual())
        .add_card(base_lv5("BASE-RED", CardColor::Red))
        .add_card(purple_anchor())
        .hand(0, &["DUAL-ARTS"])
        .memory(5)
        .start();
    r.register_effect("DUAL-ARTS", Arc::new(HandMainDirect));
    r.place_on_field(0, "PURPLE-ANCHOR", Some(0));
    r.place_on_field(0, "BASE-RED", Some(0));
    advance_to_main(&mut r);

    assert!(r.game.activate_hand_main(0, 0));
    assert!(
        r.game.pending_selection.is_none(),
        "direct MainFromHand activation must not open Arts selection"
    );
    assert_eq!(r.hand_size(0), 1, "direct activation does not use/trash the card");
    assert_eq!(r.trash_size(0), 0);
}
```

- [ ] **Step 2: Run direct activation test**

Run:

```powershell
cargo test -p digimon-engine --test dual_cards direct_hand_main_activation_does_not_enable_arts -- --nocapture
```

Expected: test passes with Arts restricted to the pending Option pipeline. Keep `activate_hand_main` free of any call to `install_arts_digivolve_selection`.

- [ ] **Step 3: Commit**

```powershell
git add code/digimon-engine/tests/dual_cards/arts_flow.rs code/digimon-engine/src/game_actions.rs
git commit -m "test(engine): lock arts out of direct main activation"
```

## Task 8: Add Rule-Check and Trigger-Ordering Regression

**Files:**
- Modify: `code/digimon-engine/tests/dual_cards/arts_flow.rs`
- Modify: `code/digimon-engine/src/game_actions.rs`

- [ ] **Step 1: Add 0-DP rule-check test**

Append to `arts_flow.rs`:

```rust
fn zero_dp_base() -> CardData {
    let mut card = base_lv5("ZERO-BASE", CardColor::Red);
    card.dp = Some(0);
    card
}

struct OnDeletionWitness(Arc<Mutex<u32>>);
impl CardEffect for OnDeletionWitness {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        let witness = self.0.clone();
        vec![Effect::on_deletion(card)
            .name("On Deletion witness")
            .process(move |_ctx| {
                *witness.lock().unwrap() += 1;
            })
            .build()]
    }
}

#[test]
fn arts_runs_rule_check_before_trigger_resolution() {
    let deletion_witness = Arc::new(Mutex::new(0));
    let digivolve_witness = Arc::new(Mutex::new(0));
    let mut r = DebugRunner::builder()
        .add_card(arts_dual())
        .add_card(zero_dp_base())
        .add_card(purple_anchor())
        .deck(0, &["PURPLE-ANCHOR"])
        .hand(0, &["DUAL-ARTS"])
        .memory(5)
        .start();
    r.register_effect("ZERO-BASE", Arc::new(OnDeletionWitness(deletion_witness.clone())));
    r.register_effect("DUAL-ARTS", Arc::new(DrawOnDigivolve(digivolve_witness.clone())));
    r.place_on_field(0, "PURPLE-ANCHOR", Some(0));
    r.place_on_field(0, "ZERO-BASE", Some(0));
    advance_to_main(&mut r);

    let result = r.game.play_option_from_hand(0, 0);
    assert_eq!(result, OptionPlayResult::Pending);
    r.game
        .resolve_selection(0, encode_attack(0, 1))
        .expect("accept Arts");

    assert_eq!(r.battle_area_size(0), 1, "0-DP Arts stack was deleted");
    assert_eq!(r.trash_size(0), 2, "base and DUAL moved to trash by deletion");
    assert_eq!(*deletion_witness.lock().unwrap(), 1, "On Deletion fired");
    assert_eq!(*digivolve_witness.lock().unwrap(), 1, "When Digivolving fired");
}
```

- [ ] **Step 2: Run rule-check test**

Run:

```powershell
cargo test -p digimon-engine --test dual_cards arts_runs_rule_check_before_trigger_resolution -- --nocapture
```

Expected: pass after Task 5's `run_rule_check_after_arts`. `arts_digivolve_pending_option_onto_battle` must enqueue both the `WhenDigivolving` trigger and any rule-check deletion triggers before the first `drain_effect_queue`.

- [ ] **Step 3: Enforce no early drain in Arts execution**

If Step 2 fails due to early drain ordering, change `arts_digivolve_pending_option_onto_battle` to:

```rust
self.enqueue_triggered(EffectTiming::WhenDigivolving, TriggerSource::Permanent(target));
self.run_rule_check_after_arts();
self.drain_effect_queue();
for pid in 0..self.players.len() {
    self.enqueue_triggered(
        EffectTiming::OnDigivolve,
        TriggerSource::PlayerBattleArea(pid as PlayerId),
    );
}
self.drain_effect_queue();
```

Run the test again after changing.

- [ ] **Step 4: Commit**

```powershell
git add code/digimon-engine/src/game_actions.rs code/digimon-engine/tests/dual_cards/arts_flow.rs
git commit -m "test(engine): cover arts rule-check timing"
```

## Task 9: Add DUAL Predicate/Search Helpers

**Files:**
- Modify: `code/digimon-engine/src/dsl_cards/predicate.rs`
- Modify: `code/digimon-dsl/src/compiled.rs`
- Modify: `code/digimon-dsl/src/predicate.rs`
- Test: `code/digimon-engine/tests/dual_cards/data_model.rs`
- Test: `code/digimon-engine/tests/dsl/parse_predicates.rs`

- [ ] **Step 1: Add predicate context enum in engine**

In `code/digimon-engine/src/dsl_cards/predicate.rs`, add:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PredicateContext {
    CardSearchAny,
    DigimonCardSearch,
    OptionCardSearch,
    FieldDigimon,
    OptionUse,
    DigivolutionRequirement,
}
```

- [ ] **Step 2: Add helper tests for search semantics**

Append to `dual_cards/data_model.rs`:

```rust
#[test]
fn dual_text_search_sees_both_faces() {
    let r = DebugRunner::builder()
        .add_card(dual_card())
        .hand(0, &["DUAL-HELPER"])
        .start();
    let card = &r.game.player(0).hand[0];
    let text = card.text_for_search_all_faces(&r.game.card_data);
    assert!(text.contains("When Digivolving"));
    assert!(text.contains("Delete 1 Digimon"));
}
```

- [ ] **Step 3: Run helper test**

Run:

```powershell
cargo test -p digimon-engine --test dual_cards dual_text_search_sees_both_faces -- --nocapture
```

Expected: pass using Task 2 helper. This locks in the minimum predicate substrate.

- [ ] **Step 4: Audit predicate call sites**

Run:

```powershell
git grep -n "card_kind\\|CardKind::Digimon\\|CardKind::Option\\|effect_text\\|inherited_text\\|security_text" -- code/digimon-engine/src/dsl_cards code/digimon-dsl/src
```

For every call site that checks `CardKind::Digimon` or `CardKind::Option` on a searched card, route through the face-specific helper:

```rust
card.is_digimon_card_for_search(card_data)
card.is_option_card_for_search(card_data)
card.text_for_search_all_faces(card_data)
```

For field permanents, keep using `perm.is_digimon(card_data)` because a DUAL top card is a Digimon on field after Task 2.

- [ ] **Step 5: Run DSL predicate tests**

Run:

```powershell
cargo test -p digimon-engine --test dsl parse_predicates -- --nocapture
```

Expected: existing predicate tests pass.

- [ ] **Step 6: Commit**

```powershell
git add code/digimon-engine/src/dsl_cards/predicate.rs code/digimon-dsl/src/compiled.rs code/digimon-dsl/src/predicate.rs code/digimon-engine/tests/dual_cards/data_model.rs code/digimon-engine/tests/dsl/parse_predicates.rs
git commit -m "feat(engine): add dual-aware predicate helpers"
```

## Task 10: Add digimoncard.io DUAL Import Mapping

**Files:**
- Modify: `code/tools/ingest_cards.py`
- Test: create or modify the closest existing ingest-card test under `code/tests` or `code/tools` after locating it with the command below.
- Docs: `docs/superpowers/specs/2026-04-28-arts-digivolve-dual-cards-design.md`

- [ ] **Step 1: Locate the ingest tests**

Run:

```powershell
git grep -n "ingest_cards\\|card_kind\\|digimoncard.io\\|type.*Dual" -- code tests docs | Select-Object -First 80
```

Append this test to the existing ingest test module:

```text
code/engine_py_legacy/tests/tools/test_ingest_cards.py
```

- [ ] **Step 2: Add ingest mapping test**

Append:

```python
from tools.ingest_cards import COLOR_MAP, convert_card


def test_digimoncard_io_dual_row_maps_to_dual_payload():
    row = {
        "id": "ST24-07",
        "name": "ShineGreymon",
        "type": "Dual",
        "level": 6,
        "play_cost": 5,
        "evolution_cost": 4,
        "color": "Yellow",
        "color2": "Red",
        "digi_type": "Light Dragon",
        "digi_type2": "DATA SQUAD",
        "dp": 12000,
        "main_effect": "[When Digivolving] Do the Digimon side.",
        "source_effect": "Use Requirement: DATA SQUAD trait\n[Main] Do the Option side.",
        "alt_effect": "[Digivolve] Lv.5 w/[RizeGreymon] in name or w/[DATA SQUAD] trait: Cost 3",
    }

    card = convert_card(row)

    assert card["card_kind"] == 4
    assert card["play_cost"] == 5
    assert card["level"] == 6
    assert card["dp"] == 12000
    assert card["effect_description_eng"] == "[When Digivolving] Do the Digimon side."
    assert card["inherited_effect_description_eng"] == ""
    assert card["dual"]["option"]["use_cost"] == 5
    assert card["dual"]["option"]["effect_text"].startswith("Use Requirement")
    assert card["dual"]["digimon"]["effect_text"].startswith("[When Digivolving]")
    assert card["dual"]["option"]["colors"] == [COLOR_MAP["Yellow"]]
```

- [ ] **Step 3: Run failing ingest test**

Run:

```powershell
python -m pytest code/engine_py_legacy/tests/tools/test_ingest_cards.py -v
```

Expected: failure because DUAL mapping is not present.

- [ ] **Step 4: Implement normalized DUAL mapping**

In `code/tools/ingest_cards.py`, extend `KIND_MAP` and `convert_card` with this behavior:

```python
KIND_MAP = {
    "Digimon": 0,
    "Tamer": 1,
    "Option": 2,
    "Digi-Egg": 3,
    "Dual": 4,
}


def convert_card(api_card: dict) -> dict:
    ...
    card_kind = KIND_MAP.get(api_card.get("type", "Digimon"), 0)
    ...
    if card_kind == KIND_MAP["Dual"]:
        out["inherited_effect_description_eng"] = ""
        out["security_effect_description_eng"] = ""
        out["dual"] = {
            "digimon": {
                "level": api_card.get("level"),
                "dp": api_card.get("dp"),
                "colors": parse_color_names(api_card),
                "traits": parse_traits(api_card),
                "evo_costs": parse_evo_costs(api_card),
                "effect_text": api_card.get("main_effect") or "",
                "inherited_text": "",
                "keywords": ["ArtsDigivolve"],
            },
            "option": {
                "use_cost": api_card.get("play_cost") or 0,
                "colors": parse_option_color_names(api_card),
                "effect_text": api_card.get("source_effect") or "",
                "security_text": "",
                "keywords": ["ArtsDigivolve"],
            },
        }
    return out
```

Use the existing parser names where the file already has equivalent helpers; preserve the same behavior. `parse_option_color_names` must use a real source field when the API exposes one. For the current live API shape, read a card-specific override map keyed by card id. Do not silently copy Digimon colors into the Option face.

- [ ] **Step 5: Add manual option-color override map**

In `ingest_cards.py`, add:

```python
DUAL_OPTION_COLOR_OVERRIDES = {
    "ST23-09": ["Purple"],
    "ST24-07": ["Yellow"],
}
```

Extend this map for every DUAL card imported into local data. If a DUAL card is not in the map and no explicit Option color can be parsed, `convert_card` must raise:

```python
raise ValueError(f"Missing Option-face color override for DUAL card {row['id']}")
```

- [ ] **Step 6: Run ingest test**

Run:

```powershell
python -m pytest code/engine_py_legacy/tests/tools/test_ingest_cards.py -v
```

Expected: test passes.

- [ ] **Step 7: Commit**

```powershell
git add code/tools/ingest_cards.py code/engine_py_legacy/tests/tools/test_ingest_cards.py
git commit -m "feat(tools): map digimoncard dual rows to dual metadata"
```

## Task 11: Add Minimal DSL DUAL Metadata Support

**Files:**
- Modify: `code/digimon-dsl/src/spec.rs`
- Modify: `code/digimon-dsl/src/compiled.rs`
- Modify: `code/digimon-dsl/src/compile.rs`
- Modify: `code/digimon-engine/src/debug_runner.rs`
- Test: `code/digimon-engine/tests/dsl/parse_minimal.rs`

- [ ] **Step 1: Add DSL parse test**

Add to `code/digimon-engine/tests/dsl/parse_minimal.rs`:

```rust
#[test]
fn parses_dual_card_metadata() {
    let yaml = r#"
card: DUAL-DSL
name: Dual DSL
kind: dual
dual:
  digimon:
    level: 6
    dp: 12000
    colors: [Red]
    traits: [DualTrait]
    effect_text: "[When Digivolving] Draw 1."
    inherited_text: ""
  option:
    use_cost: 5
    colors: [Purple]
    effect_text: "[Main] Gain 2 memory."
    security_text: ""
    keywords: [ArtsDigivolve]
effects: []
"#;
    let spec: digimon_dsl::CardSpec = serde_yml::from_str(yaml).expect("parse dual yaml");
    let compiled = digimon_dsl::compile::compile(&spec)
        .expect("compile dual yaml");
    assert_eq!(compiled.card, "DUAL-DSL");
    assert!(compiled.dual.is_some());
}
```

- [ ] **Step 2: Run failing DSL test**

Run:

```powershell
cargo test -p digimon-engine --test dsl parses_dual_card_metadata -- --nocapture
```

Expected: parse or compile failure because `kind: dual` and `dual` metadata are not supported.

- [ ] **Step 3: Add DSL spec structs**

In `code/digimon-dsl/src/spec.rs`, add:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DualSpec {
    pub digimon: DualDigimonSpec,
    pub option: DualOptionSpec,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DualDigimonSpec {
    pub level: u8,
    pub dp: i32,
    #[serde(default)]
    pub colors: Vec<Color>,
    #[serde(default)]
    pub traits: Vec<String>,
    #[serde(default)]
    pub effect_text: String,
    #[serde(default)]
    pub inherited_text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DualOptionSpec {
    pub use_cost: u16,
    #[serde(default)]
    pub colors: Vec<Color>,
    #[serde(default)]
    pub effect_text: String,
    #[serde(default)]
    pub security_text: String,
    #[serde(default)]
    pub keywords: Vec<String>,
}
```

Add to `CardSpec`:

```rust
#[serde(default, skip_serializing_if = "Option::is_none")]
pub dual: Option<DualSpec>,
```

Add `Dual` to the DSL card kind enum.

- [ ] **Step 4: Add compiled structs**

In `code/digimon-dsl/src/compiled.rs`, add compiled equivalents:

```rust
#[derive(Debug, Clone, PartialEq)]
pub struct CompiledDual {
    pub digimon: CompiledDualDigimon,
    pub option: CompiledDualOption,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CompiledDualDigimon {
    pub level: u8,
    pub dp: i32,
    pub colors: Vec<CompiledColor>,
    pub traits: Vec<String>,
    pub effect_text: String,
    pub inherited_text: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CompiledDualOption {
    pub use_cost: u16,
    pub colors: Vec<CompiledColor>,
    pub effect_text: String,
    pub security_text: String,
    pub keywords: Vec<String>,
}
```

Add to `CompiledCard`:

```rust
pub dual: Option<CompiledDual>,
```

Add `Dual` to `CompiledCardKind`.

- [ ] **Step 5: Compile dual metadata**

In `code/digimon-dsl/src/compile.rs`, map `kind: dual` to `CompiledCardKind::Dual` and copy the `dual` payload into `CompiledCard`.

Use explicit conversion of colors with the existing color conversion helper. Reuse the match expression already used for `CompiledColor` when the conversion helper is local to another module.

- [ ] **Step 6: Update engine compiled-card conversion**

In `code/digimon-engine/src/debug_runner.rs`, update `card_data_from_compiled`:

```rust
CompiledCardKind::Dual => CardKind::Dual,
```

Map `compiled.dual` into `CardData.dual` using engine `DualCardData`, `DualDigimonFace`, and `DualOptionFace`.

- [ ] **Step 7: Run DSL test**

Run:

```powershell
cargo test -p digimon-engine --test dsl parses_dual_card_metadata -- --nocapture
```

Expected: test passes.

- [ ] **Step 8: Commit**

```powershell
git add code/digimon-dsl/src/spec.rs code/digimon-dsl/src/compiled.rs code/digimon-dsl/src/compile.rs code/digimon-engine/src/debug_runner.rs code/digimon-engine/tests/dsl/parse_minimal.rs
git commit -m "feat(dsl): parse dual card metadata"
```

## Task 12: Final Verification and Documentation Sync

**Files:**
- Modify: `docs/ACTION_SPEC.md`
- Modify: `docs/TENSOR_SPEC.md` only if tensor card-kind encoding changes.
- Modify: `docs/RUST_ENGINE_API.md`
- Modify: `docs/superpowers/specs/2026-04-28-arts-digivolve-dual-cards-design.md` if implementation decisions differ from spec.

- [ ] **Step 1: Update Rust engine API docs**

In `docs/RUST_ENGINE_API.md`, add a short section:

```markdown
### DUAL Cards and Arts Digivolve

DUAL cards are represented as `CardKind::Dual` with explicit `dual.digimon` and `dual.option` faces. Use face-aware helpers such as `CardSource::option_use_cost`, `CardSource::option_colors`, and `CardSource::digivolution_costs`; do not read `play_cost`, `colors`, or `evo_costs` directly for DUAL Option or Digimon behavior.

When a DUAL card is used as an Option, `PendingOption.source_kind` records the use source. Arts Digivolve is offered only after true Option use, never after direct `MainFromHand` activation. Arts target selection is represented by `PendingSelection` with PASS as decline.
```

- [ ] **Step 2: Run focused Rust tests**

Run:

```powershell
cargo test -p digimon-engine --test dual_cards -- --nocapture
cargo test -p digimon-engine --test option_flow
cargo test -p digimon-engine --test dsl parses_dual_card_metadata -- --nocapture
```

Expected: all three commands pass.

- [ ] **Step 3: Run broader engine checks**

Run:

```powershell
cargo test -p digimon-engine
```

Expected: all tests pass. If unrelated known failures exist, capture the failing test names and confirm the DUAL-focused test suite still passes.

- [ ] **Step 4: Check git diff for generated churn**

Run:

```powershell
git status --short
git diff --stat
```

Expected: only files from this plan changed. No generated card-data churn unless Task 10 intentionally regenerated a fixture.

- [ ] **Step 5: Commit final docs**

```powershell
git add docs/RUST_ENGINE_API.md docs/ACTION_SPEC.md docs/TENSOR_SPEC.md docs/superpowers/specs/2026-04-28-arts-digivolve-dual-cards-design.md
git commit -m "docs(engine): document dual cards and arts digivolve"
```

Skip unchanged docs in `git add` if Git reports no modifications.

## Self-Review Notes

- Spec coverage: data model, live API shape, action mask, decoder, pending Option source tracking, Arts decline, Arts accept, breeding target, rule check, predicate/search, importer, DSL, and docs are covered by Tasks 1-12.
- Scope decision: individual real DUAL card scripts remain outside this plan. The importer substrate and synthetic card tests are enough for a working engine feature.
- Type consistency: this plan uses `CardKind::Dual`, `Keyword::ArtsDigivolve`, `DualCardData`, `DualDigimonFace`, `DualOptionFace`, `OptionUseSource`, `OptionResolutionPhase::ArtsSelectTarget`, and `BREEDING_SELECTION_TARGET` consistently across tasks.
- Verification: every implementation task includes a failing-test command, a passing-test command, and a commit step.
