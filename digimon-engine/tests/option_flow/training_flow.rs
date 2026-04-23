//! Phase 8 Task 5 — Training Option flow tests.
//!
//! A Training Option plays like a Standard Option (cost + body drain) but
//! instead of trashing on dispose it **parks on the owner's battle_area** as
//! a Permanent in `OptionState::Training { owner }`. It provides sideways
//! inheritance to the owner's other permanents via `.inherited()` effects
//! on the Training card, scoped by the Training permanent being the source.
//! When the owner's breeding egg hatches (via `move_from_breeding`), every
//! Training permanent the owner controls is trashed and `OnTrainingTrash`
//! fires from each of them. Training permanents are not legal attack
//! targets (same as Delay and any non-Digimon permanent).

use std::sync::{Arc, Mutex};

use digimon_engine::card_source::CardHandle;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::effect::{CardEffect, Effect};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming};
use digimon_engine::permanent::OptionState;
use digimon_engine::selection::OptionPlayResult;

// ─── Inline test-effect helpers ──────────────────────────────────────

/// Minimal Training Option body — no side effects. Used for parking/shape
/// tests where the body content is irrelevant.
struct TrainingNoop;
impl CardEffect for TrainingNoop {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::on_play(card)
            .name("Training noop")
            .training()
            .process(|_ctx| {})
            .build()]
    }
}

/// Training Option that installs an `OnTrainingTrash` observer on itself.
/// The witness increments when the Training card is trashed (via the hatch
/// hook in `move_from_breeding`).
struct TrainingWithOnTrashWitness(Arc<Mutex<u32>>);
impl CardEffect for TrainingWithOnTrashWitness {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        let slot = self.0.clone();
        vec![
            Effect::on_play(card)
                .name("Training body")
                .training()
                .process(|_ctx| {})
                .build(),
            Effect::on_play(card)
                .name("OnTrainingTrash witness")
                .timing(EffectTiming::OnTrainingTrash)
                .process(move |_ctx| {
                    *slot.lock().unwrap() += 1;
                })
                .build(),
        ]
    }
}

/// Training Option with a sideways-inherited `OnHatch` witness. The
/// `.inherited()` flag on the secondary effect opts into the sideways scan
/// from any same-owner permanent firing a timing. When the owner hatches
/// an egg, the promoted permanent's OnHatch fires and — via the sideways
/// scan — this Training card's inherited effect fires too, incrementing
/// the witness.
struct TrainingWithInheritedHatch(Arc<Mutex<u32>>);
impl CardEffect for TrainingWithInheritedHatch {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        let slot = self.0.clone();
        vec![
            Effect::on_play(card)
                .name("Training body")
                .training()
                .process(|_ctx| {})
                .build(),
            Effect::on_hatch(card)
                .name("Sideways OnHatch witness")
                .inherited()
                .process(move |_ctx| {
                    *slot.lock().unwrap() += 1;
                })
                .build(),
        ]
    }
}

// ─── Fixture helpers ──────────────────────────────────────────────────

fn option_card(card_id: &str, cost: u16, color: CardColor) -> digimon_engine::CardData {
    let mut cd = make_test_card(card_id, card_id);
    cd.card_kind = CardKind::Option;
    cd.level = None;
    cd.dp = None;
    cd.play_cost = cost;
    cd.colors = vec![color];
    cd
}

fn digimon_card(card_id: &str, color: CardColor) -> digimon_engine::CardData {
    let mut cd = make_test_card(card_id, card_id);
    cd.colors = vec![color];
    cd
}

fn digi_egg(card_id: &str, color: CardColor) -> digimon_engine::CardData {
    let mut cd = make_test_card(card_id, card_id);
    cd.card_kind = CardKind::DigiEgg;
    cd.colors = vec![color];
    cd
}

fn advance_to_main(r: &mut DebugRunner) {
    r.game.enter_main_phase();
}

// ─── Tests ─────────────────────────────────────────────────────────────

/// Test 1: playing a Training Option pushes it onto the owner's battle_area
/// with `OptionState::Training { owner }`. It does NOT trash on dispose.
#[test]
fn training_parks_alongside_breeding() {
    let mut r = DebugRunner::builder()
        .add_card(option_card("TRAIN-PARK", 0, CardColor::Red))
        .add_card(digimon_card("RED-MATCH", CardColor::Red))
        .hand(0, &["TRAIN-PARK"])
        .memory(0)
        .start();
    r.register_effect("TRAIN-PARK", Arc::new(TrainingNoop));
    r.place_on_field(0, "RED-MATCH", Some(0));
    advance_to_main(&mut r);

    let battle_before = r.battle_area_size(0);
    let result = r.game.play_option_from_hand(0, 0);

    assert_eq!(result, OptionPlayResult::Trashed);
    assert!(r.game.pending_option.is_none(), "pending_option cleared");
    assert_eq!(r.hand_size(0), 0, "option left hand");
    assert_eq!(r.trash_size(0), 0, "training does NOT trash — it parks on field");
    assert_eq!(
        r.battle_area_size(0),
        battle_before + 1,
        "training option parks as a permanent on the battle_area"
    );

    // The training permanent is the newest addition.
    let idx = r.battle_area_size(0) - 1;
    let perm = &r.game.player(0).battle_area[idx];
    match perm.option_state {
        OptionState::Training { owner } => {
            assert_eq!(owner, 0, "training state records the controller");
        }
        other => panic!("expected Training option_state, got {:?}", other),
    }
}

/// Test 2: when the owner hatches an egg from breeding, every Training
/// permanent the owner controls is trashed and `OnTrainingTrash` fires
/// once per trashed Training card.
#[test]
fn training_trashes_on_breeding_promotion() {
    let witness = Arc::new(Mutex::new(0u32));

    let mut r = DebugRunner::builder()
        .add_card(option_card("TRAIN-TRSH", 0, CardColor::Red))
        .add_card(digimon_card("RED-MATCH", CardColor::Red))
        .add_card(digi_egg("EGG", CardColor::Red))
        .hand(0, &["TRAIN-TRSH"])
        .digitama(0, &["EGG"])
        .memory(0)
        .start();
    r.register_effect(
        "TRAIN-TRSH",
        Arc::new(TrainingWithOnTrashWitness(witness.clone())),
    );
    r.place_on_field(0, "RED-MATCH", Some(0));
    advance_to_main(&mut r);

    // Play the Training Option — parks on field.
    let result = r.game.play_option_from_hand(0, 0);
    assert_eq!(result, OptionPlayResult::Trashed);
    assert_eq!(r.trash_size(0), 0, "training parked, not trashed");
    assert!(
        r.game.player(0).battle_area.iter().any(|p| matches!(
            p.option_state,
            OptionState::Training { .. }
        )),
        "training permanent on field"
    );

    // Hatch an egg into breeding, then promote it. The promotion must
    // trash every Training permanent the owner controls.
    assert!(r.game.hatch(0), "hatch egg into breeding");
    let trash_before = r.trash_size(0);
    assert!(r.game.move_from_breeding(0), "promote egg to battle_area");

    assert!(
        !r.game.player(0).battle_area.iter().any(|p| matches!(
            p.option_state,
            OptionState::Training { .. }
        )),
        "training permanent gone after hatch"
    );
    assert_eq!(
        r.trash_size(0),
        trash_before + 1,
        "training card trashed on breeding promotion"
    );
    assert_eq!(
        *witness.lock().unwrap(),
        1,
        "OnTrainingTrash fired on the trashed Training permanent"
    );
}

/// Test 3: with an empty breeding area, ending a turn does NOT trash
/// Training permanents — they persist across turn rollovers until the
/// owner actually promotes an egg.
#[test]
fn training_persists_if_breeding_empty() {
    let mut r = DebugRunner::builder()
        .add_card(option_card("TRAIN-KEEP", 0, CardColor::Red))
        .add_card(digimon_card("RED-MATCH", CardColor::Red))
        .add_card(digimon_card("FILLER", CardColor::Red))
        .hand(0, &["TRAIN-KEEP"])
        .deck(0, &["FILLER"; 5])
        .deck(1, &["FILLER"; 5])
        .memory(0)
        .start();
    r.register_effect("TRAIN-KEEP", Arc::new(TrainingNoop));
    r.place_on_field(0, "RED-MATCH", Some(0));
    advance_to_main(&mut r);

    let result = r.game.play_option_from_hand(0, 0);
    assert_eq!(result, OptionPlayResult::Trashed);
    let training_present = r.game.player(0).battle_area.iter().any(|p| matches!(
        p.option_state,
        OptionState::Training { .. }
    ));
    assert!(training_present, "parked");

    // End P0 turn; P1 plays through; back to P0.
    r.end_turn();
    r.game.enter_main_phase();
    r.end_turn();
    r.game.enter_main_phase();

    // Training still on field — no breeding promotion happened.
    let training_still = r.game.player(0).battle_area.iter().any(|p| matches!(
        p.option_state,
        OptionState::Training { .. }
    ));
    assert!(training_still, "training persists with empty breeding");
    assert_eq!(r.trash_size(0), 0);
}

/// Test 4: a Training card with an `.inherited()` effect at some timing
/// fires sideways when the owner's hatched permanent fires that timing.
/// Verifies the effect_queue sideways-inheritance scan wiring. Uses
/// OnHatch as the timing so the fire happens during the same
/// `move_from_breeding` call that will then trash the Training card.
#[test]
fn training_inherited_effect_applies_to_breeding_permanent() {
    let witness = Arc::new(Mutex::new(0u32));

    let mut r = DebugRunner::builder()
        .add_card(option_card("TRAIN-INH", 0, CardColor::Red))
        .add_card(digimon_card("RED-MATCH", CardColor::Red))
        .add_card(digi_egg("EGG", CardColor::Red))
        .hand(0, &["TRAIN-INH"])
        .digitama(0, &["EGG"])
        .memory(0)
        .start();
    r.register_effect(
        "TRAIN-INH",
        Arc::new(TrainingWithInheritedHatch(witness.clone())),
    );
    r.place_on_field(0, "RED-MATCH", Some(0));
    advance_to_main(&mut r);

    let result = r.game.play_option_from_hand(0, 0);
    assert_eq!(result, OptionPlayResult::Trashed);
    assert_eq!(*witness.lock().unwrap(), 0, "not fired on play");

    // Hatch + promote — OnHatch fires, sideways scan includes the Training
    // card's inherited effect, witness increments. Training card then
    // trashes (not asserted here — covered by test 2).
    assert!(r.game.hatch(0));
    assert!(r.game.move_from_breeding(0));

    assert_eq!(
        *witness.lock().unwrap(),
        1,
        "sideways-inherited OnHatch effect fired from the Training permanent"
    );
}

/// Test 5: a Training permanent is not a legal attack target from the
/// opponent's perspective. The action mask emits no attack bit for it and
/// a direct attack is rejected.
#[test]
fn training_excluded_from_attack_targeting() {
    use digimon_engine::action::mask::build_action_mask;
    use digimon_engine::action::space::encode_attack;

    let mut r = DebugRunner::builder()
        .add_card(option_card("TRAIN-TGT", 0, CardColor::Red))
        .add_card(digimon_card("RED-MATCH", CardColor::Red))
        .add_card(digimon_card("P1-ATKR", CardColor::Red))
        .hand(0, &["TRAIN-TGT"])
        .memory(0)
        .start();
    r.register_effect("TRAIN-TGT", Arc::new(TrainingNoop));
    r.place_on_field(0, "RED-MATCH", Some(0));
    advance_to_main(&mut r);
    let result = r.game.play_option_from_hand(0, 0);
    assert_eq!(result, OptionPlayResult::Trashed);

    let training_idx = r.game.player(0).battle_area.len() - 1;
    assert!(matches!(
        r.game.player(0).battle_area[training_idx].option_state,
        OptionState::Training { .. }
    ));

    r.end_turn(); // → P1's turn.
    let attacker = r.place_on_field(1, "P1-ATKR", Some(0));
    r.game.enter_main_phase();

    // Mask: no attack bit for attacker → training_idx.
    let mask = build_action_mask(&r.game, 1);
    let bit = encode_attack(attacker.index as u16, training_idx as u16) as usize;
    assert_eq!(
        mask[bit], 0.0,
        "attack bit for Training target must NOT be set in the mask"
    );

    // Direct call: attack rejected as Invalid.
    let training_handle = r.perm_handle(0, training_idx);
    let attack = r.game.attack_digimon(attacker, training_handle, false);
    assert_eq!(attack, digimon_engine::combat::AttackResult::Invalid);
}
