//! Phase 8 Task 2 — Standard Option play pipeline tests.
//!
//! Exercises `Game::play_option_from_hand` / `play_option_from_trash`
//! end-to-end for Standard Options (no Delay / Link / Training). Delay /
//! Link / Training dispatch arrives in Tasks 3 / 4 / 5. Phase 7
//! `WhenWouldBeTrashed` replacement during dispose lands in Task 6.

use std::sync::{Arc, Mutex};

use digimon_dsl::compiled::CompiledStep;
use digimon_engine::card_source::CardHandle;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::dsl_cards::bindings::Bindings;
use digimon_engine::dsl_cards::raw_rust::EngineRawRustRegistry;
use digimon_engine::dsl_cards::step::StepRuntime;
use digimon_engine::effect::{CardEffect, Effect};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming};
use digimon_engine::selection::OptionPlayResult;

// ─── Inline test-effect helpers ──────────────────────────────────────

/// Option body that gains 2 memory when played.
struct GainTwoMemory;
impl CardEffect for GainTwoMemory {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![
            Effect::on_play(card) // placeholder; overwritten by .option_main()
                .name("Gain 2 memory")
                .option_main()
                .process(|ctx| {
                    ctx.gain_memory(2);
                })
                .build(),
        ]
    }
}

/// Observer on a Digimon — fires `OnUseOption` and increments a sentinel
/// counter each time.
struct OnUseOptionCounter(Arc<Mutex<u32>>);
impl CardEffect for OnUseOptionCounter {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        let slot = self.0.clone();
        vec![
            Effect::on_play(card) // placeholder timing; overwritten below
                .name("OnUseOption witness")
                .timing(digimon_engine::enums::EffectTiming::OnUseOption)
                .process(move |_ctx| {
                    *slot.lock().unwrap() += 1;
                })
                .build(),
        ]
    }
}

/// Option body that installs a `select_opponent_permanent` — used to
/// exercise the selection-unwind re-entry path.
struct OptionSelectOppPermanent(Arc<Mutex<bool>>);
impl CardEffect for OptionSelectOppPermanent {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        let witness = self.0.clone();
        vec![Effect::on_play(card)
            .name("Pick an opponent permanent")
            .option_main()
            .process(move |ctx| {
                let witness = witness.clone();
                ctx.select_opponent_permanent(
                    "Pick an opponent",
                    false,
                    |_g, _h| true,
                    move |_ctx, _h| {
                        *witness.lock().unwrap() = true;
                    },
                );
            })
            .build()]
    }
}

struct StandardSchedulesEndOfTurn(Arc<Mutex<u32>>);
impl CardEffect for StandardSchedulesEndOfTurn {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        let seen = self.0.clone();
        vec![Effect::on_play(card)
            .name("Schedule end turn")
            .option_main()
            .process(move |ctx| {
                let seen = seen.clone();
                let mut raw = EngineRawRustRegistry::new();
                raw.register_step("increment_witness", move |ctx, _bindings| {
                    assert!(
                        ctx.source_is_option(),
                        "scheduled replay must preserve Option source kind"
                    );
                    *seen.lock().unwrap() += 1;
                });
                ctx.schedule_delayed_with_runtime(
                    EffectTiming::EndOfYourTurn,
                    vec![CompiledStep::RawRust {
                        fn_name: "increment_witness".to_string(),
                        consumes: vec![],
                        binds: vec![],
                    }],
                    Bindings::new(),
                    StepRuntime::new(Arc::new(raw)),
                );
            })
            .build()]
    }
}

// ─── Fixture helpers ──────────────────────────────────────────────────

/// Build a minimal Option CardData with the given id, cost, and color.
fn option_card(card_id: &str, cost: u16, color: CardColor) -> digimon_engine::CardData {
    let mut cd = make_test_card(card_id, card_id);
    cd.card_kind = CardKind::Option;
    cd.level = None;
    cd.dp = None;
    cd.play_cost = cost;
    cd.colors = vec![color];
    cd
}

/// Build a minimal Digimon CardData the tests can plant on the field to
/// satisfy the Option color-match gate.
fn digimon_card(card_id: &str, color: CardColor) -> digimon_engine::CardData {
    let mut cd = make_test_card(card_id, card_id);
    cd.colors = vec![color];
    cd
}

/// DebugRunner starts the game in Breeding phase; Option plays require
/// Main. Advance for every test that actually plays an Option.
fn advance_to_main(r: &mut DebugRunner) {
    r.game.enter_main_phase();
}

// ─── Tests ─────────────────────────────────────────────────────────────

/// Test 1: a Standard Option plays, fires its body, and is trashed.
/// Memory rises by 2 (from body), memory gate covers the cost, battle_area
/// stays unchanged.
#[test]
fn standard_option_trashes_after_resolve() {
    let mut r = DebugRunner::builder()
        .add_card(option_card("OPT-GAIN", 0, CardColor::Red))
        .add_card(digimon_card("RED-FIELD", CardColor::Red))
        .hand(0, &["OPT-GAIN"])
        .memory(0)
        .start();
    r.register_effect("OPT-GAIN", Arc::new(GainTwoMemory));
    r.place_on_field(0, "RED-FIELD", Some(0));
    advance_to_main(&mut r);

    let memory_before = r.memory();
    let battle_before = r.battle_area_size(0);

    let result = r.game.play_option_from_hand(0, 0);

    assert_eq!(result, OptionPlayResult::Trashed);
    assert_eq!(r.memory(), memory_before + 2, "body added 2 memory");
    assert_eq!(r.hand_size(0), 0, "option left hand");
    assert_eq!(r.trash_size(0), 1, "option landed in trash");
    assert_eq!(
        r.battle_area_size(0),
        battle_before,
        "standard options never enter the battle area"
    );
    assert!(r.game.pending_option.is_none(), "pending_option cleared");
}

/// Test 2: `OnUseOption` fires on every player's battle area when an
/// Option is played. Mirror of DCGO's "when any Option card is played"
/// observer.
#[test]
fn standard_option_fires_on_use_option_globally() {
    let witness = Arc::new(Mutex::new(0u32));

    let mut r = DebugRunner::builder()
        .add_card(option_card("OPT-USE", 0, CardColor::Red))
        .add_card(digimon_card("OBSERVER", CardColor::Red))
        .add_card(digimon_card("RED-MATCH", CardColor::Red))
        .hand(0, &["OPT-USE"])
        .memory(0)
        .start();
    r.register_effect("OPT-USE", Arc::new(GainTwoMemory));
    r.register_effect("OBSERVER", Arc::new(OnUseOptionCounter(witness.clone())));

    // Color-match digimon for P0 (so the option is playable).
    r.place_on_field(0, "RED-MATCH", Some(0));
    // Observer on P1 — OnUseOption is a global observer, scanning every
    // player's battle area (including the non-turn-player's).
    r.place_on_field(1, "OBSERVER", Some(0));
    advance_to_main(&mut r);

    let result = r.game.play_option_from_hand(0, 0);
    assert_eq!(result, OptionPlayResult::Trashed);
    assert_eq!(
        *witness.lock().unwrap(),
        1,
        "OnUseOption fires globally — P1's observer sees P0's play"
    );
}

/// Test 3: cost is paid from the memory seesaw using the printed cost.
#[test]
fn standard_option_pays_play_cost() {
    let mut r = DebugRunner::builder()
        .add_card(option_card("OPT-PAY", 3, CardColor::Red))
        .add_card(digimon_card("RED-MATCH", CardColor::Red))
        .hand(0, &["OPT-PAY"])
        .memory(5)
        .start();
    // Empty body — we just want to observe cost deduction.
    struct Noop;
    impl CardEffect for Noop {
        fn effects(&self, card: CardHandle) -> Vec<Effect> {
            vec![Effect::on_play(card)
                .name("noop")
                .option_main()
                .process(|_| {})
                .build()]
        }
    }
    r.register_effect("OPT-PAY", Arc::new(Noop));
    r.place_on_field(0, "RED-MATCH", Some(0));
    advance_to_main(&mut r);

    let result = r.game.play_option_from_hand(0, 0);
    assert_eq!(result, OptionPlayResult::Trashed);
    assert_eq!(r.memory(), 5 - 3, "memory deducted by printed cost");
}

/// Test 4: unaffordable cost returns `Invalid` and leaves the card in hand.
#[test]
fn standard_option_unaffordable_returns_invalid() {
    let mut r = DebugRunner::builder()
        .add_card(option_card("OPT-COSTY", 1, CardColor::Red))
        .add_card(digimon_card("RED-MATCH", CardColor::Red))
        .hand(0, &["OPT-COSTY"])
        .start();
    struct Noop;
    impl CardEffect for Noop {
        fn effects(&self, card: CardHandle) -> Vec<Effect> {
            vec![Effect::on_play(card).option_main().process(|_| {}).build()]
        }
    }
    r.register_effect("OPT-COSTY", Arc::new(Noop));
    r.place_on_field(0, "RED-MATCH", Some(0));
    advance_to_main(&mut r);

    // Pin memory at the lower bound (−10) so cost 1 pushes past the floor.
    r.game.set_memory(-10);

    let hand_before = r.hand_size(0);
    let trash_before = r.trash_size(0);
    let memory_before = r.memory();

    let result = r.game.play_option_from_hand(0, 0);

    assert_eq!(result, OptionPlayResult::Invalid);
    assert_eq!(r.hand_size(0), hand_before, "card must stay in hand");
    assert_eq!(r.trash_size(0), trash_before, "no trash movement");
    assert_eq!(r.memory(), memory_before, "memory untouched on reject");
    assert!(r.game.pending_option.is_none());
}

/// Test 5: color mismatch — the Option's color has no matching Digimon or
/// Tamer on the player's field. Mask emits 0 for the hand slot, and a
/// direct `play_option_from_hand` call returns `Invalid`.
#[test]
fn standard_option_color_mismatch_is_masked_and_rejected() {
    use digimon_engine::action::mask::build_action_mask;

    let mut r = DebugRunner::builder()
        .add_card(option_card("OPT-YELLOW", 0, CardColor::Yellow))
        .add_card(digimon_card("RED-ONLY", CardColor::Red))
        .hand(0, &["OPT-YELLOW"])
        .memory(3)
        .start();
    struct Noop;
    impl CardEffect for Noop {
        fn effects(&self, card: CardHandle) -> Vec<Effect> {
            vec![Effect::on_play(card).option_main().process(|_| {}).build()]
        }
    }
    r.register_effect("OPT-YELLOW", Arc::new(Noop));
    r.place_on_field(0, "RED-ONLY", Some(0));
    advance_to_main(&mut r);

    // The mask must NOT light up the PLAY_HAND bit for the mismatched Option.
    let mask = build_action_mask(&r.game, 0);
    assert_eq!(
        mask[0], 0.0,
        "color-mismatched Option must not be legal in mask"
    );

    // A direct call short-circuits at the color-match gate.
    let result = r.game.play_option_from_hand(0, 0);
    assert_eq!(result, OptionPlayResult::Invalid);
    assert_eq!(r.hand_size(0), 1);
    assert_eq!(r.trash_size(0), 0);
}

/// Test 6: `play_option_from_trash` resolves + fires the body. Since Task
/// 2 disposes Standard Options to trash, a trash-sourced play ends up back
/// in trash after resolve.
#[test]
fn standard_option_in_trash_plays_via_effect() {
    let mut r = DebugRunner::builder()
        .add_card(option_card("OPT-FROM-TRASH", 0, CardColor::Red))
        .add_card(digimon_card("RED-MATCH", CardColor::Red))
        .memory(0)
        .start();
    r.register_effect("OPT-FROM-TRASH", Arc::new(GainTwoMemory));
    r.place_on_field(0, "RED-MATCH", Some(0));
    advance_to_main(&mut r);

    // Seed the trash directly (no hand path).
    let data_idx = r
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == "OPT-FROM-TRASH")
        .unwrap();
    let next_idx = r.game.next_card_index();
    let cs = digimon_engine::card_source::CardSource::new(data_idx, 0, next_idx);
    r.game.players[0].trash.push(cs);

    let memory_before = r.memory();
    let result = r.game.play_option_from_trash(0, 0);

    assert_eq!(result, OptionPlayResult::Trashed);
    assert_eq!(r.memory(), memory_before + 2, "body fired from trash play");
    assert_eq!(
        r.trash_size(0),
        1,
        "trash-to-trash dispose leaves the card where it started"
    );
    assert!(r.game.pending_option.is_none());
}

/// Test 7: an `OptionMain` body that installs a `PendingSelection` returns
/// `Pending`. Resolving the selection re-enters dispose and trashes the
/// Option.
#[test]
fn standard_option_with_target_selection_returns_pending() {
    let witness = Arc::new(Mutex::new(false));

    let mut r = DebugRunner::builder()
        .add_card(option_card("OPT-PICK", 0, CardColor::Red))
        .add_card(digimon_card("RED-MATCH", CardColor::Red))
        .add_card(digimon_card("OPP-TARGET", CardColor::Red))
        .hand(0, &["OPT-PICK"])
        .memory(0)
        .start();
    r.register_effect(
        "OPT-PICK",
        Arc::new(OptionSelectOppPermanent(witness.clone())),
    );
    r.place_on_field(0, "RED-MATCH", Some(0));
    let target = r.place_on_field(1, "OPP-TARGET", Some(0));
    advance_to_main(&mut r);

    let result = r.game.play_option_from_hand(0, 0);
    assert_eq!(result, OptionPlayResult::Pending);
    assert!(r.game.pending_selection.is_some());
    assert!(r.game.pending_option.is_some());
    assert_eq!(
        r.game.pending_option.as_ref().unwrap().resolution_phase,
        digimon_engine::selection::OptionResolutionPhase::MainEffectDrain
    );

    // Resolve the selection — action_id for OppField is encoded by the
    // install helper; fish it out of valid_action_ids directly.
    let action_id = r.game.pending_selection.as_ref().unwrap().valid_action_ids[0];
    let _ = r.game.resolve_selection(0, action_id);

    assert!(*witness.lock().unwrap(), "target callback fired");
    assert!(
        r.game.pending_option.is_none(),
        "dispose re-entered after selection resolved"
    );
    assert!(r.game.pending_selection.is_none());
    assert_eq!(r.trash_size(0), 1, "option trashed after selection unwind");
    // target still on field — the test effect records the pick but doesn't mutate.
    let _ = target;
    assert_eq!(r.battle_area_size(1), 1);
}

#[test]
fn transient_option_scheduled_end_of_turn_effect_replays_with_option_source() {
    let witness = Arc::new(Mutex::new(0u32));
    let mut r = DebugRunner::builder()
        .add_card(option_card("SCHEDULED-OPT", 0, CardColor::Red))
        .add_card(digimon_card("RED-MATCH", CardColor::Red))
        .hand(0, &["SCHEDULED-OPT"])
        .memory(0)
        .start();
    r.register_effect(
        "SCHEDULED-OPT",
        Arc::new(StandardSchedulesEndOfTurn(witness.clone())),
    );
    r.place_on_field(0, "RED-MATCH", Some(0));
    advance_to_main(&mut r);

    assert_eq!(
        r.game.play_option_from_hand(0, 0),
        OptionPlayResult::Trashed
    );
    assert_eq!(r.trash_size(0), 1, "scheduled option is in trash before replay");
    assert_eq!(*witness.lock().unwrap(), 0);
    r.end_turn();
    assert_eq!(*witness.lock().unwrap(), 1);
}
