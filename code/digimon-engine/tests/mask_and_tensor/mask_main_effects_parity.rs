//! Action-mask parity tests for [Main] activated effects (§4.5c).
//!
//! Covers the three zone-scoped timing variants added in the effect-listing
//! infrastructure work:
//!
//! * `EffectTiming::MainFromHand`  — bits 30-59
//! * `EffectTiming::MainOnField`   — bits 1000-1149 at sub-slot
//!                                   `FIELD_EFFECT_SLOT_FOR_MAIN`
//! * `EffectTiming::MainFromTrash` — bits 1150-1194
//!
//! The semantics mirror Python's `action_mask.py:176-225`: first-match-wins
//! per zone slot, condition closures gate eligibility, OPT on field respects
//! `Permanent::activation_count`, and hand/trash OPT defers to the condition
//! closure (Python's mask doesn't check `_turn_activate_count` either).

use std::sync::Arc;

use digimon_engine::action::{
    build_action_mask, EFFECTS_PER_PERMANENT, FIELD_EFFECT_SLOT_FOR_MAIN, FIELD_EFFECT_START,
    HAND_EFFECT_START, TRASH_EFFECT_START,
};
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::{CardHandle, CardSource};
use digimon_engine::cards::CardEffectRegistry;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::effect::{CardEffect, Effect};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming};

// ─── Card factories ────────────────────────────────────────────────────

fn make_digimon(id: &str, color: CardColor) -> CardData {
    CardData {
        card_id: id.to_string(),
        card_name: id.to_string(),
        card_kind: CardKind::Digimon,
        level: Some(4),
        dp: Some(4000),
        play_cost: 5,
        colors: vec![color],
        traits: Vec::new(),
        evo_costs: Vec::new(),
        dna_costs: Vec::new(),
        effect_text: String::new(),
        inherited_text: String::new(),
        security_text: String::new(),
        keywords: Vec::new(),
        effect_class_name: id.replace('-', "_"),
        index: 0,
        norm_id: 0.0,
    }
}

fn make_option(id: &str, color: CardColor) -> CardData {
    CardData {
        card_id: id.to_string(),
        card_name: id.to_string(),
        card_kind: CardKind::Option,
        level: None,
        dp: None,
        play_cost: 3,
        colors: vec![color],
        traits: Vec::new(),
        evo_costs: Vec::new(),
        dna_costs: Vec::new(),
        effect_text: String::new(),
        inherited_text: String::new(),
        security_text: String::new(),
        keywords: Vec::new(),
        effect_class_name: id.replace('-', "_"),
        index: 0,
        norm_id: 0.0,
    }
}

// ─── Test card effects ────────────────────────────────────────────────

/// Unconditional [Hand] [Main].
struct HandMainAlways;
impl CardEffect for HandMainAlways {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::declarative(card)
            .name("Hand Main — always")
            .timing(EffectTiming::MainFromHand)
            .build()]
    }
}

/// [Hand] [Main] gated on memory >= 0.
struct HandMainMemGate;
impl CardEffect for HandMainMemGate {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::declarative(card)
            .name("Hand Main — memory >= 0")
            .timing(EffectTiming::MainFromHand)
            .condition(|ctx| ctx.memory() >= 0)
            .build()]
    }
}

/// Non-inherited [Field] [Main] with OPT.
struct FieldMainOptTop;
impl CardEffect for FieldMainOptTop {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::declarative(card)
            .name("Field Main — top, OPT")
            .timing(EffectTiming::MainOnField)
            .once_per_turn()
            .build()]
    }
}

/// Inherited [Field] [Main] — only fires when this card is *under* another.
struct FieldMainInherited;
impl CardEffect for FieldMainInherited {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::inherited(card)
            .name("Field Main — inherited")
            .timing(EffectTiming::MainOnField)
            .build()]
    }
}

/// [Field] [Main] gated on turn_count >= 5 — lets a single test flip the
/// condition by advancing turns.
struct FieldMainTurnGate;
impl CardEffect for FieldMainTurnGate {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::declarative(card)
            .name("Field Main — turn >= 5")
            .timing(EffectTiming::MainOnField)
            .condition(|ctx| ctx.turn_count() >= 5)
            .build()]
    }
}

/// Unconditional [Trash] [Main].
struct TrashMainAlways;
impl CardEffect for TrashMainAlways {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::declarative(card)
            .name("Trash Main — always")
            .timing(EffectTiming::MainFromTrash)
            .build()]
    }
}

/// [Trash] [Main] gated on memory >= 0.
struct TrashMainMemGate;
impl CardEffect for TrashMainMemGate {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::declarative(card)
            .name("Trash Main — memory >= 0")
            .timing(EffectTiming::MainFromTrash)
            .condition(|ctx| ctx.memory() >= 0)
            .build()]
    }
}

fn test_registry() -> CardEffectRegistry {
    let mut r = CardEffectRegistry::default();
    r.insert("HAND-ALWAYS", Arc::new(HandMainAlways));
    r.insert("HAND-MEM", Arc::new(HandMainMemGate));
    r.insert("FIELD-OPT", Arc::new(FieldMainOptTop));
    r.insert("FIELD-INH", Arc::new(FieldMainInherited));
    r.insert("FIELD-TURN", Arc::new(FieldMainTurnGate));
    r.insert("TRASH-ALWAYS", Arc::new(TrashMainAlways));
    r.insert("TRASH-MEM", Arc::new(TrashMainMemGate));
    // Plain vanilla digimon used as the top card in stack-based tests.
    // (No effect registered — effects() returns empty when impl absent.)
    r
}

/// Push a card into a player's trash zone directly.
fn plant_trash(r: &mut DebugRunner, player: u8, card_id: &str) {
    let data_idx = r
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .unwrap_or_else(|| panic!("plant_trash: unknown card_id {}", card_id));
    let next = r.game.next_card_index();
    let card = CardSource::new(data_idx, player, next);
    r.game.players[player as usize].trash.push(card);
}

// Helper: decode the Field [Main] bit for permanent at `field_index`.
fn field_main_bit(field_index: usize) -> usize {
    (FIELD_EFFECT_START + field_index as u16 * EFFECTS_PER_PERMANENT + FIELD_EFFECT_SLOT_FOR_MAIN)
        as usize
}

// ─── [Hand] [Main] ────────────────────────────────────────────────────

#[test]
fn hand_main_effect_emits_when_condition_passes() {
    let mut r = DebugRunner::builder()
        .add_card(make_option("HAND-ALWAYS", CardColor::Red))
        .with_registry(test_registry())
        .hand(0, &["HAND-ALWAYS"])
        .start();

    r.game.enter_main_phase();
    let mask = build_action_mask(&r.game, 0);

    assert_eq!(
        mask[(HAND_EFFECT_START) as usize],
        1.0,
        "unconditional Hand [Main] at hand index 0 should emit"
    );
}

#[test]
fn hand_main_effect_suppressed_when_condition_fails() {
    let mut r = DebugRunner::builder()
        .add_card(make_option("HAND-MEM", CardColor::Red))
        .with_registry(test_registry())
        .hand(0, &["HAND-MEM"])
        .start();

    r.game.enter_main_phase();
    r.game.set_memory(-2);

    let mask = build_action_mask(&r.game, 0);
    assert_eq!(
        mask[(HAND_EFFECT_START) as usize],
        0.0,
        "memory<0 condition must suppress Hand [Main] bit"
    );

    // Flip the condition: memory >= 0 → bit lights up.
    r.game.set_memory(3);
    let mask_ok = build_action_mask(&r.game, 0);
    assert_eq!(
        mask_ok[(HAND_EFFECT_START) as usize],
        1.0,
        "memory>=0 condition must re-emit Hand [Main] bit"
    );
}

#[test]
fn hand_main_bit_is_per_slot_not_per_effect() {
    // Two cards with the same Hand [Main] effect, at indices 0 and 1 of
    // hand — each should emit its own bit independently.
    let mut r = DebugRunner::builder()
        .add_card(make_option("HAND-ALWAYS", CardColor::Red))
        .with_registry(test_registry())
        .hand(0, &["HAND-ALWAYS", "HAND-ALWAYS"])
        .start();

    r.game.enter_main_phase();
    let mask = build_action_mask(&r.game, 0);

    assert_eq!(mask[(HAND_EFFECT_START) as usize], 1.0);
    assert_eq!(mask[(HAND_EFFECT_START + 1) as usize], 1.0);
    assert_eq!(
        mask[(HAND_EFFECT_START + 2) as usize],
        0.0,
        "no hand card at slot 2 → no bit"
    );
}

// ─── [Field] [Main] ───────────────────────────────────────────────────

#[test]
fn field_main_effect_emits_from_top_card() {
    let mut r = DebugRunner::builder()
        .add_card(make_digimon("FIELD-OPT", CardColor::Red))
        .with_registry(test_registry())
        .start();

    let tp = r.game.turn_player();
    r.place_on_field(tp, "FIELD-OPT", Some(0));
    r.game.enter_main_phase();

    let mask = build_action_mask(&r.game, tp);
    assert_eq!(
        mask[field_main_bit(0)],
        1.0,
        "Field [Main] on a top-card digimon should emit at sub-slot +2"
    );
}

#[test]
fn field_main_effect_respects_opt_via_activation_count() {
    let mut r = DebugRunner::builder()
        .add_card(make_digimon("FIELD-OPT", CardColor::Red))
        .with_registry(test_registry())
        .start();

    let tp = r.game.turn_player();
    let handle = r.place_on_field(tp, "FIELD-OPT", Some(0));
    r.game.enter_main_phase();

    // Pre-check: bit is live.
    let mask_before = build_action_mask(&r.game, tp);
    assert_eq!(mask_before[field_main_bit(0)], 1.0);

    // Simulate the OPT having already fired this turn: activation_count
    // reaches the effect's max_per_turn (1). Use the source handle + slot 0.
    let source_handle = r.game.players[tp as usize].battle_area[handle.index as usize]
        .top_card()
        .handle();
    r.game.players[tp as usize].battle_area[handle.index as usize]
        .record_activation(source_handle, 0);

    let mask_after = build_action_mask(&r.game, tp);
    assert_eq!(
        mask_after[field_main_bit(0)],
        0.0,
        "Field [Main] bit must disappear once OPT is exhausted"
    );
}

#[test]
fn field_main_effect_inherited_only_fires_when_under() {
    // Single FIELD-INH as top card: inherited effect should NOT emit.
    let mut r = DebugRunner::builder()
        .add_card(make_digimon("FIELD-INH", CardColor::Red))
        .add_card(make_digimon("VANILLA", CardColor::Red))
        .with_registry(test_registry())
        .start();

    let tp = r.game.turn_player();
    let handle = r.place_on_field(tp, "FIELD-INH", Some(0));
    r.game.enter_main_phase();

    let mask_top = build_action_mask(&r.game, tp);
    assert_eq!(
        mask_top[field_main_bit(handle.index as usize)],
        0.0,
        "inherited Field [Main] on a top card must not emit"
    );

    // Digivolve VANILLA on top → FIELD-INH is now under → should emit.
    let vanilla_idx = r
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == "VANILLA")
        .unwrap();
    let next = r.game.next_card_index();
    let top = CardSource::new(vanilla_idx, tp, next);
    r.game.digivolve_onto(tp, handle.index as usize, top);

    let mask_stacked = build_action_mask(&r.game, tp);
    assert_eq!(
        mask_stacked[field_main_bit(handle.index as usize)],
        1.0,
        "inherited Field [Main] must emit once under another source"
    );
}

#[test]
fn field_main_effect_condition_gate_responds_to_state() {
    // FIELD-TURN fires iff turn_count >= 5. Start of the game is turn 1.
    let mut r = DebugRunner::builder()
        .add_card(make_digimon("FIELD-TURN", CardColor::Red))
        .with_registry(test_registry())
        .start();

    let tp = r.game.turn_player();
    r.place_on_field(tp, "FIELD-TURN", Some(0));
    r.game.enter_main_phase();

    let mask_t1 = build_action_mask(&r.game, tp);
    assert_eq!(
        mask_t1[field_main_bit(0)],
        0.0,
        "turn_count<5 should suppress the Field [Main] bit"
    );

    // Bypass the turn state machine: directly advance turn_count.
    r.game.turn_count = 5;
    let mask_t5 = build_action_mask(&r.game, tp);
    assert_eq!(
        mask_t5[field_main_bit(0)],
        1.0,
        "turn_count>=5 should unmask the Field [Main] bit"
    );
}

#[test]
fn field_main_bit_is_per_permanent() {
    let mut r = DebugRunner::builder()
        .add_card(make_digimon("FIELD-OPT", CardColor::Red))
        .with_registry(test_registry())
        .start();

    let tp = r.game.turn_player();
    r.place_on_field(tp, "FIELD-OPT", Some(0));
    r.place_on_field(tp, "FIELD-OPT", Some(0));
    r.game.enter_main_phase();

    let mask = build_action_mask(&r.game, tp);
    assert_eq!(mask[field_main_bit(0)], 1.0);
    assert_eq!(mask[field_main_bit(1)], 1.0);
    assert_eq!(
        mask[field_main_bit(2)],
        0.0,
        "no permanent at index 2 → no bit"
    );
}

// ─── [Trash] [Main] ───────────────────────────────────────────────────

#[test]
fn trash_main_effect_emits_when_condition_passes() {
    let mut r = DebugRunner::builder()
        .add_card(make_option("TRASH-ALWAYS", CardColor::Red))
        .with_registry(test_registry())
        .start();

    let tp = r.game.turn_player();
    plant_trash(&mut r, tp, "TRASH-ALWAYS");
    r.game.enter_main_phase();

    let mask = build_action_mask(&r.game, tp);
    assert_eq!(
        mask[(TRASH_EFFECT_START) as usize],
        1.0,
        "unconditional Trash [Main] at trash index 0 should emit"
    );
}

#[test]
fn trash_main_effect_suppressed_when_condition_fails() {
    let mut r = DebugRunner::builder()
        .add_card(make_option("TRASH-MEM", CardColor::Red))
        .with_registry(test_registry())
        .start();

    let tp = r.game.turn_player();
    plant_trash(&mut r, tp, "TRASH-MEM");
    r.game.enter_main_phase();
    r.game.set_memory(-4);

    let mask_fail = build_action_mask(&r.game, tp);
    assert_eq!(
        mask_fail[(TRASH_EFFECT_START) as usize],
        0.0,
        "memory<0 condition must suppress Trash [Main] bit"
    );

    r.game.set_memory(2);
    let mask_ok = build_action_mask(&r.game, tp);
    assert_eq!(
        mask_ok[(TRASH_EFFECT_START) as usize],
        1.0,
        "memory>=0 must re-emit Trash [Main] bit"
    );
}

#[test]
fn trash_main_bit_is_per_slot() {
    let mut r = DebugRunner::builder()
        .add_card(make_option("TRASH-ALWAYS", CardColor::Red))
        .with_registry(test_registry())
        .start();

    let tp = r.game.turn_player();
    plant_trash(&mut r, tp, "TRASH-ALWAYS");
    plant_trash(&mut r, tp, "TRASH-ALWAYS");
    plant_trash(&mut r, tp, "TRASH-ALWAYS");
    r.game.enter_main_phase();

    let mask = build_action_mask(&r.game, tp);
    assert_eq!(mask[(TRASH_EFFECT_START) as usize], 1.0);
    assert_eq!(mask[(TRASH_EFFECT_START + 1) as usize], 1.0);
    assert_eq!(mask[(TRASH_EFFECT_START + 2) as usize], 1.0);
    assert_eq!(
        mask[(TRASH_EFFECT_START + 3) as usize],
        0.0,
        "no trash card at slot 3 → no bit"
    );
}

// ─── Phase gating ─────────────────────────────────────────────────────

#[test]
fn main_effects_do_not_emit_outside_main_phase() {
    // All three zone categories populated; verify all bits are 0 while
    // the phase is Breeding.
    let mut r = DebugRunner::builder()
        .add_card(make_option("HAND-ALWAYS", CardColor::Red))
        .add_card(make_digimon("FIELD-OPT", CardColor::Red))
        .add_card(make_option("TRASH-ALWAYS", CardColor::Red))
        .with_registry(test_registry())
        .hand(0, &["HAND-ALWAYS"])
        .start();

    let tp = r.game.turn_player();
    r.place_on_field(tp, "FIELD-OPT", Some(0));
    plant_trash(&mut r, tp, "TRASH-ALWAYS");

    // Force Breeding phase (the default for the initial turn after
    // start_game is Main; step back to Breeding for this assertion).
    r.game.current_phase = digimon_engine::enums::GamePhase::Breeding;

    let mask = build_action_mask(&r.game, tp);
    assert_eq!(mask[(HAND_EFFECT_START) as usize], 0.0);
    assert_eq!(mask[field_main_bit(0)], 0.0);
    assert_eq!(mask[(TRASH_EFFECT_START) as usize], 0.0);
}
