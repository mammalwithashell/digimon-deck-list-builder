//! Action-execution parity tests for `[Main]` activated effects
//! (§4.5c-residual). Covers the three `Game::activate_*` helpers that the
//! decoder invokes when consuming the HAND_EFFECT / FIELD_EFFECT /
//! TRASH_EFFECT action bits.
//!
//! Mirrors the mask-side structure in
//! [`mask_main_effects_parity.rs`](./mask_main_effects_parity.rs): the same
//! `[Main]`-timing factories, condition-gate semantics, and inherited-vs-top
//! filter. Successful fires are observed via memory deltas inside each
//! effect's `process` closure (Python's equivalent lives in
//! `digimon_gym/engine/game/action_decoder.py:404-469`).

use std::sync::Arc;

use digimon_engine::action::{
    build_action_mask, EFFECTS_PER_PERMANENT, FIELD_EFFECT_SLOT_FOR_MAIN, FIELD_EFFECT_START,
    HAND_EFFECT_START,
};
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::{CardHandle, CardSource};
use digimon_engine::cards::CardEffectRegistry;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::effect::{CardEffect, Effect};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming};

// ─── Card factories ───────────────────────────────────────────────────

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
        dual: None,
        effect_class_name: id.replace('-', "_"),
        index: 0,
        norm_id: 0.0,
        ace_overflow: None,
        digixros_aliases: Vec::new(),
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
        dual: None,
        effect_class_name: id.replace('-', "_"),
        index: 0,
        norm_id: 0.0,
        ace_overflow: None,
        digixros_aliases: Vec::new(),
    }
}

// ─── Test card effects ────────────────────────────────────────────────
// Each effect's `process` closure gains +7 memory so a successful fire is
// visible as a +7 delta from whatever memory the test started with. Different
// deltas per zone let a single assertion distinguish which zone fired.

/// [Hand] [Main] — always fires; adds +7 memory.
struct HandMainAlwaysGain;
impl CardEffect for HandMainAlwaysGain {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::declarative(card)
            .name("Hand Main — +7 memory")
            .timing(EffectTiming::MainFromHand)
            .process(|ctx| ctx.gain_memory(7))
            .build()]
    }
}

/// [Hand] [Main] — fires only when memory >= 0; adds +7.
struct HandMainMemGateGain;
impl CardEffect for HandMainMemGateGain {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::declarative(card)
            .name("Hand Main — memory>=0, +7")
            .timing(EffectTiming::MainFromHand)
            .condition(|ctx| ctx.memory() >= 0)
            .process(|ctx| ctx.gain_memory(7))
            .build()]
    }
}

/// [Hand] — non-Main timing; should be ignored by `activate_hand_main`.
struct HandNonMain;
impl CardEffect for HandNonMain {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::on_play(card)
            .name("Hand — wrong timing")
            .process(|ctx| ctx.gain_memory(7))
            .build()]
    }
}

/// [Field] [Main] top-card, once per turn, +5 memory.
struct FieldMainOptTopGain;
impl CardEffect for FieldMainOptTopGain {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::declarative(card)
            .name("Field Main — top, OPT, +5")
            .timing(EffectTiming::MainOnField)
            .once_per_turn()
            .process(|ctx| ctx.gain_memory(5))
            .build()]
    }
}

/// Inherited [Field] [Main] — fires only when this card is *under*; +5 memory.
struct FieldMainInheritedGain;
impl CardEffect for FieldMainInheritedGain {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::inherited(card)
            .name("Field Main — inherited, +5")
            .timing(EffectTiming::MainOnField)
            .process(|ctx| ctx.gain_memory(5))
            .build()]
    }
}

/// [Field] [Main] gated on turn_count >= 5.
struct FieldMainTurnGateGain;
impl CardEffect for FieldMainTurnGateGain {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::declarative(card)
            .name("Field Main — turn>=5, +5")
            .timing(EffectTiming::MainOnField)
            .condition(|ctx| ctx.turn_count() >= 5)
            .process(|ctx| ctx.gain_memory(5))
            .build()]
    }
}

/// [Trash] [Main] — always fires; +3 memory.
struct TrashMainAlwaysGain;
impl CardEffect for TrashMainAlwaysGain {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::declarative(card)
            .name("Trash Main — +3 memory")
            .timing(EffectTiming::MainFromTrash)
            .process(|ctx| ctx.gain_memory(3))
            .build()]
    }
}

/// [Trash] [Main] — fires only when memory >= 0.
struct TrashMainMemGateGain;
impl CardEffect for TrashMainMemGateGain {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::declarative(card)
            .name("Trash Main — memory>=0, +3")
            .timing(EffectTiming::MainFromTrash)
            .condition(|ctx| ctx.memory() >= 0)
            .process(|ctx| ctx.gain_memory(3))
            .build()]
    }
}

fn test_registry() -> CardEffectRegistry {
    let mut r = CardEffectRegistry::default();
    r.insert("HAND-ALWAYS", Arc::new(HandMainAlwaysGain));
    r.insert("HAND-MEM", Arc::new(HandMainMemGateGain));
    r.insert("HAND-WRONG", Arc::new(HandNonMain));
    r.insert("FIELD-OPT", Arc::new(FieldMainOptTopGain));
    r.insert("FIELD-INH", Arc::new(FieldMainInheritedGain));
    r.insert("FIELD-TURN", Arc::new(FieldMainTurnGateGain));
    r.insert("TRASH-ALWAYS", Arc::new(TrashMainAlwaysGain));
    r.insert("TRASH-MEM", Arc::new(TrashMainMemGateGain));
    r
}

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

fn field_main_bit(field_index: usize) -> usize {
    (FIELD_EFFECT_START + field_index as u16 * EFFECTS_PER_PERMANENT + FIELD_EFFECT_SLOT_FOR_MAIN)
        as usize
}

// ─── [Hand] [Main] ────────────────────────────────────────────────────

#[test]
fn hand_main_fires_effect_and_returns_true() {
    let mut r = DebugRunner::builder()
        .add_card(make_option("HAND-ALWAYS", CardColor::Red))
        .with_registry(test_registry())
        .hand(0, &["HAND-ALWAYS"])
        .start();
    r.game.enter_main_phase();
    r.game.set_memory(0);

    let fired = r.game.activate_hand_main(0, 0);
    assert!(fired, "unconditional Hand [Main] must fire");
    assert_eq!(r.game.memory, 7, "process closure must run (+7 memory)");
}

#[test]
fn hand_main_returns_false_when_no_matching_timing() {
    // HAND-WRONG has an on_play effect, not a MainFromHand one.
    let mut r = DebugRunner::builder()
        .add_card(make_option("HAND-WRONG", CardColor::Red))
        .with_registry(test_registry())
        .hand(0, &["HAND-WRONG"])
        .start();
    r.game.enter_main_phase();
    r.game.set_memory(0);

    let fired = r.game.activate_hand_main(0, 0);
    assert!(
        !fired,
        "non-Main timing must not be consumed by activate_hand_main"
    );
    assert_eq!(r.game.memory, 0, "no effect must run");
}

#[test]
fn hand_main_returns_false_on_out_of_range_index() {
    let mut r = DebugRunner::builder()
        .add_card(make_option("HAND-ALWAYS", CardColor::Red))
        .with_registry(test_registry())
        .hand(0, &["HAND-ALWAYS"])
        .start();
    r.game.enter_main_phase();

    assert!(
        !r.game.activate_hand_main(0, 99),
        "out-of-range slot must return false"
    );
    assert!(
        !r.game.activate_hand_main(99, 0),
        "out-of-range player must return false"
    );
}

#[test]
fn hand_main_condition_gate_suppresses_fire() {
    let mut r = DebugRunner::builder()
        .add_card(make_option("HAND-MEM", CardColor::Red))
        .with_registry(test_registry())
        .hand(0, &["HAND-MEM"])
        .start();
    r.game.enter_main_phase();
    r.game.set_memory(-2);

    let fired = r.game.activate_hand_main(0, 0);
    assert!(!fired, "memory<0 condition must suppress fire");
    assert_eq!(r.game.memory, -2, "no effect must run when condition false");

    // Flip the condition: memory >= 0 → fires.
    r.game.set_memory(0);
    assert!(r.game.activate_hand_main(0, 0));
    assert_eq!(r.game.memory, 7);
}

// ─── [Field] [Main] ───────────────────────────────────────────────────

#[test]
fn field_main_fires_and_records_activation() {
    let mut r = DebugRunner::builder()
        .add_card(make_digimon("FIELD-OPT", CardColor::Red))
        .with_registry(test_registry())
        .start();
    let tp = r.game.turn_player();
    let handle = r.place_on_field(tp, "FIELD-OPT", Some(0));
    r.game.enter_main_phase();
    r.game.set_memory(0);

    let fired = r.game.activate_field_main(tp, handle.index as usize);
    assert!(fired);
    assert_eq!(r.game.memory, 5);

    // OPT bookkeeping: activation_count for (source handle, slot 0) is 1.
    let perm = &r.game.players[tp as usize].battle_area[handle.index as usize];
    let source_handle = perm.top_card().handle();
    assert_eq!(
        perm.activation_count(source_handle, 0),
        1,
        "field activation must increment the per-source, per-slot counter"
    );
}

#[test]
fn field_main_opt_exhausted_does_not_fire() {
    let mut r = DebugRunner::builder()
        .add_card(make_digimon("FIELD-OPT", CardColor::Red))
        .with_registry(test_registry())
        .start();
    let tp = r.game.turn_player();
    let handle = r.place_on_field(tp, "FIELD-OPT", Some(0));
    r.game.enter_main_phase();
    r.game.set_memory(0);

    // Fire once, then try again.
    assert!(r.game.activate_field_main(tp, handle.index as usize));
    assert_eq!(r.game.memory, 5);

    let fired_again = r.game.activate_field_main(tp, handle.index as usize);
    assert!(
        !fired_again,
        "OPT should gate a second activation in the same turn"
    );
    assert_eq!(
        r.game.memory, 5,
        "memory must not change on the second attempt"
    );
}

#[test]
fn field_main_respects_inherited_filter() {
    // FIELD-INH is inherited-only — fires only when under another card.
    let mut r = DebugRunner::builder()
        .add_card(make_digimon("FIELD-INH", CardColor::Red))
        .add_card(make_digimon("VANILLA", CardColor::Red))
        .with_registry(test_registry())
        .start();
    let tp = r.game.turn_player();
    let handle = r.place_on_field(tp, "FIELD-INH", Some(0));
    r.game.enter_main_phase();
    r.game.set_memory(0);

    // As a top card, the inherited effect must NOT fire.
    assert!(
        !r.game.activate_field_main(tp, handle.index as usize),
        "inherited Field [Main] on a top card must not fire"
    );
    assert_eq!(r.game.memory, 0);

    // Stack a vanilla on top → FIELD-INH is now under → fires.
    let vanilla_idx = r
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == "VANILLA")
        .unwrap();
    let next = r.game.next_card_index();
    let top = CardSource::new(vanilla_idx, tp, next);
    r.game.digivolve_onto(tp, handle.index as usize, top);

    assert!(
        r.game.activate_field_main(tp, handle.index as usize),
        "inherited Field [Main] must fire once under another source"
    );
    assert_eq!(r.game.memory, 5);
}

#[test]
fn field_main_condition_gate_suppresses_fire() {
    let mut r = DebugRunner::builder()
        .add_card(make_digimon("FIELD-TURN", CardColor::Red))
        .with_registry(test_registry())
        .start();
    let tp = r.game.turn_player();
    let handle = r.place_on_field(tp, "FIELD-TURN", Some(0));
    r.game.enter_main_phase();
    r.game.set_memory(0);

    assert!(
        !r.game.activate_field_main(tp, handle.index as usize),
        "turn_count<5 must suppress fire"
    );
    assert_eq!(r.game.memory, 0);

    // Advance turn_count directly (matches mask-side test).
    r.game.turn_count = 5;
    assert!(r.game.activate_field_main(tp, handle.index as usize));
    assert_eq!(r.game.memory, 5);
}

#[test]
fn field_main_out_of_range_index_returns_false() {
    let mut r = DebugRunner::builder()
        .add_card(make_digimon("FIELD-OPT", CardColor::Red))
        .with_registry(test_registry())
        .start();
    let tp = r.game.turn_player();
    r.game.enter_main_phase();

    assert!(!r.game.activate_field_main(tp, 99));
    assert!(!r.game.activate_field_main(99, 0));
}

// ─── [Trash] [Main] ───────────────────────────────────────────────────

#[test]
fn trash_main_fires_effect_and_returns_true() {
    let mut r = DebugRunner::builder()
        .add_card(make_option("TRASH-ALWAYS", CardColor::Red))
        .with_registry(test_registry())
        .start();
    let tp = r.game.turn_player();
    plant_trash(&mut r, tp, "TRASH-ALWAYS");
    r.game.enter_main_phase();
    r.game.set_memory(0);

    assert!(r.game.activate_trash_main(tp, 0));
    assert_eq!(r.game.memory, 3);
}

#[test]
fn trash_main_condition_gate_suppresses_fire() {
    let mut r = DebugRunner::builder()
        .add_card(make_option("TRASH-MEM", CardColor::Red))
        .with_registry(test_registry())
        .start();
    let tp = r.game.turn_player();
    plant_trash(&mut r, tp, "TRASH-MEM");
    r.game.enter_main_phase();
    r.game.set_memory(-4);

    assert!(!r.game.activate_trash_main(tp, 0));
    assert_eq!(r.game.memory, -4);

    r.game.set_memory(0);
    assert!(r.game.activate_trash_main(tp, 0));
    assert_eq!(r.game.memory, 3);
}

#[test]
fn trash_main_out_of_range_index_returns_false() {
    let mut r = DebugRunner::builder()
        .add_card(make_option("TRASH-ALWAYS", CardColor::Red))
        .with_registry(test_registry())
        .start();
    let tp = r.game.turn_player();
    plant_trash(&mut r, tp, "TRASH-ALWAYS");
    r.game.enter_main_phase();

    assert!(!r.game.activate_trash_main(tp, 99));
    assert!(!r.game.activate_trash_main(99, 0));
}

// ─── Mask ↔ decoder consistency ───────────────────────────────────────

#[test]
fn mask_and_field_decoder_agree_on_opt_exhaustion() {
    // Build a state where the Field [Main] bit is live, fire via the
    // decoder, rebuild the mask, and assert the bit is now suppressed by
    // the same activation_count that the decoder just recorded.
    let mut r = DebugRunner::builder()
        .add_card(make_digimon("FIELD-OPT", CardColor::Red))
        .with_registry(test_registry())
        .start();
    let tp = r.game.turn_player();
    let handle = r.place_on_field(tp, "FIELD-OPT", Some(0));
    r.game.enter_main_phase();

    let mask_before = build_action_mask(&r.game, tp);
    assert_eq!(mask_before[field_main_bit(handle.index as usize)], 1.0);

    assert!(r.game.activate_field_main(tp, handle.index as usize));

    let mask_after = build_action_mask(&r.game, tp);
    assert_eq!(
        mask_after[field_main_bit(handle.index as usize)],
        0.0,
        "mask must agree with decoder's OPT bookkeeping after a fire"
    );
}

#[test]
fn mask_and_hand_decoder_agree_without_opt_tracking() {
    // Hand [Main] has no per-turn activation counter (§4.5c-residual 🟡)
    // — after firing, the mask bit should still be live, matching Python.
    let mut r = DebugRunner::builder()
        .add_card(make_option("HAND-ALWAYS", CardColor::Red))
        .with_registry(test_registry())
        .hand(0, &["HAND-ALWAYS"])
        .start();
    r.game.enter_main_phase();

    let mask_before = build_action_mask(&r.game, 0);
    assert_eq!(mask_before[HAND_EFFECT_START as usize], 1.0);

    assert!(r.game.activate_hand_main(0, 0));

    let mask_after = build_action_mask(&r.game, 0);
    assert_eq!(
        mask_after[HAND_EFFECT_START as usize], 1.0,
        "hand [Main] has no OPT — bit must stay live post-fire"
    );
}
