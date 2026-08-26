use crate::dsl_card_data::compiled;

use digimon_engine::permanent::OptionState;

use super::support::{field_contains, hand_index, select_hand_card, tb_digimon, DebugRunner};
use digimon_engine::action::mask::build_action_mask;
use digimon_engine::action::space::{
    EFFECTS_PER_PERMANENT, FIELD_EFFECT_SLOT_FOR_MAIN, FIELD_EFFECT_START,
};
use digimon_engine::enums::{CardColor, DelayTrigger};

const CARD_ID: &str = "EX12-070";

#[test]
fn ex12_070_has_tb_use_requirement() {
    let card = compiled(CARD_ID);
    let requirement = card
        .use_requirement
        .as_ref()
        .and_then(|pred| pred.any_field_permanent.as_ref())
        .expect("EX12-070 prints Use Req. ([TB] trait)");

    assert_eq!(
        requirement.predicate.trait_has.as_deref(),
        Some("TB"),
        "Use Req. should look for an own [TB] trait permanent"
    );
}

#[test]
fn ex12_070_main_trashes_tb_card_draws_two_and_places_delay_option() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-070 YAML loads")
        .add_card(tb_digimon("TB-DISCARD", CardColor::Blue, 4, 5000))
        .add_card(tb_digimon("DRAW-A", CardColor::Blue, 3, 3000))
        .add_card(tb_digimon("DRAW-B", CardColor::Blue, 3, 3000))
        .hand(0, &[CARD_ID, "TB-DISCARD"])
        .deck(0, &["DRAW-A", "DRAW-B"])
        .memory(3)
        .start();

    let option_slot = hand_index(&runner, 0, CARD_ID);
    assert!(runner.game.activate_hand_main(0, option_slot));
    select_hand_card(&mut runner, 0, "TB-DISCARD");
    runner.auto_resolve().expect("resolve main");

    assert!(field_contains(&runner, 0, CARD_ID));
    assert_eq!(runner.hand_size(0), 2);
    let option = runner.game.players[0]
        .battle_area
        .iter()
        .find(|perm| perm.top_card().card_id(&runner.game.card_data) == CARD_ID)
        .expect("placed option");
    assert!(matches!(option.option_state, OptionState::Delayed { .. }));
}

/// Seat EX12-070 in P0's battle area through its real printed `[Main]` body
/// (trash 1 [TB] from hand → `<Draw 2>` → "place this card in the battle
/// area"), and return the turn it was placed on.
fn place_via_main(runner: &mut DebugRunner) -> u16 {
    let option_slot = hand_index(runner, 0, CARD_ID);
    assert!(runner.game.activate_hand_main(0, option_slot));
    select_hand_card(runner, 0, "TB-DISCARD");
    runner.auto_resolve().expect("resolve main");
    assert!(
        field_contains(runner, 0, CARD_ID),
        "the [Main] body places EX12-070 in the battle area"
    );
    runner.game.turn_count
}

fn builder_with_deck_fuel() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-070 YAML loads")
        .add_card(tb_digimon("TB-DISCARD", CardColor::Blue, 4, 5000))
        .add_card(tb_digimon("FILL", CardColor::Blue, 3, 3000))
        .hand(0, &[CARD_ID, "TB-DISCARD"])
        .deck(0, &["FILL"; 12])
        .deck(1, &["FILL"; 12])
        .memory(3)
        .start()
}

fn trash_contains(runner: &DebugRunner, player: u8, card_id: &str) -> bool {
    runner.game.players[player as usize]
        .trash
        .iter()
        .any(|card| card.card_id(&runner.game.card_data) == card_id)
}

/// G-DELAY-EVENT-WINDOW-AUTOTRASHED.
///
/// EX12-070's `<Delay>` window is an EVENT owned by its `kind: replacement`
/// clause ("[All Turns] When any of your level 5 or higher [TB] trait Digimon
/// would leave the battle area, ＜Delay＞"), so the card carries NO
/// `delay_trigger` for the delay machinery to find. The engine must therefore
/// park it indefinitely, not invent a turn-scheduled expiry.
///
/// Rules manual (`general_rule.pdf` p.35):
/// - 16-16-1 "＜Delay＞ is a keyword effect. While a card with this effect is
///   in the battle area, by trashing that card, the effect specified in
///   ＜Delay＞ will activate." — availability is bounded by presence in the
///   battle area, and by nothing else. There is no expiry clause anywhere in
///   16-16.
/// - 16-16-3 bounds it only from BELOW ("can't be activated the same turn the
///   card ... is placed").
///
/// DCGO agrees: `EX12_070.cs`'s Delay registers at
/// `EffectTiming.WhenRemoveField` gated by `CanDeclareOptionDelayEffect(card)`,
/// which is `IsExistOnBattleArea(card) && EnterFieldTurnCount != TurnCount` —
/// presence + not-the-placing-turn, with no scheduled trash anywhere.
#[test]
fn ex12_070_delay_option_parks_with_no_scheduled_expiry() {
    let mut runner = builder_with_deck_fuel();
    let placing_turn = place_via_main(&mut runner);

    // The parked carrier must carry NO scheduled expiry.
    let option = runner.game.players[0]
        .battle_area
        .iter()
        .find(|perm| perm.top_card().card_id(&runner.game.card_data) == CARD_ID)
        .expect("EX12-070 parked in the battle area");
    match option.option_state {
        OptionState::Delayed {
            trigger,
            trash_on_turn,
            placed_on_turn,
            ..
        } => {
            assert_eq!(
                placed_on_turn, placing_turn,
                "placing turn is recorded for the 16-16-3 gate"
            );
            assert_ne!(
                trigger,
                DelayTrigger::EndOfYourNextTurn,
                "EX12-070 prints no turn-scheduled <Delay> window; the delay \
                 machinery must not invent one"
            );
            assert_eq!(
                trash_on_turn,
                u16::MAX,
                "16-16-1 bounds availability by presence in the battle area only \
                 — an event-window <Delay> parks indefinitely"
            );
        }
        other => panic!("EX12-070 must park as OptionState::Delayed; got {other:?}"),
    }
}

/// The behavioural half of G-DELAY-EVENT-WINDOW-AUTOTRASHED: the carrier
/// survives the turn-end scan that the `EndOfYourNextTurn` fallback used to
/// schedule it into (`placing_turn + 2`). See the doc comment above for the
/// 16-16-1 / DCGO citations.
#[test]
fn ex12_070_delay_option_is_not_auto_trashed_after_the_owners_next_turn() {
    let mut runner = builder_with_deck_fuel();
    let placing_turn = place_via_main(&mut runner);

    // Walk well past the owner's NEXT turn — the window the old
    // `EndOfYourNextTurn` fallback scheduled (`placing_turn + 2`).
    for _ in 0..5 {
        runner.end_turn();
        runner.auto_resolve().expect("no selection is owed by a turn flip");
        assert!(
            field_contains(&runner, 0, CARD_ID),
            "EX12-070 must stay in the battle area (16-16-1): turn {} (placed on {placing_turn})",
            runner.game.turn_count
        );
        assert!(
            !trash_contains(&runner, 0, CARD_ID),
            "EX12-070 must never be auto-trashed: turn {} (placed on {placing_turn})",
            runner.game.turn_count
        );
    }
    assert!(
        runner.game.turn_count >= placing_turn + 4,
        "the walk must cross the owner's next turn end at placing_turn + 2; \
         reached turn {}",
        runner.game.turn_count
    );
}

/// Guard on the SHAPE of the fix: parking the carrier indefinitely must not be
/// bought by reusing `DelayTrigger::MainPhaseActivated`, which would expose a
/// `[Main]`-phase FIELD_EFFECT activation the printed card never offers.
/// EX12-070's `<Delay>` is reachable ONLY through the "would leave the battle
/// area" replacement window.
#[test]
fn ex12_070_parked_delay_option_offers_no_main_phase_activation() {
    let mut runner = builder_with_deck_fuel();
    place_via_main(&mut runner);

    // Advance to the owner's next turn, where 16-16-3 no longer blocks.
    runner.end_turn();
    runner.auto_resolve().expect("no selection owed");
    runner.end_turn();
    runner.auto_resolve().expect("no selection owed");
    assert_eq!(runner.game.turn_player(), 0);
    runner.game.enter_main_phase();

    let slot = runner.game.players[0]
        .battle_area
        .iter()
        .position(|perm| perm.top_card().card_id(&runner.game.card_data) == CARD_ID)
        .expect("EX12-070 still parked on the owner's next turn");
    let main_bit = (FIELD_EFFECT_START
        + slot as u16 * EFFECTS_PER_PERMANENT
        + FIELD_EFFECT_SLOT_FOR_MAIN) as usize;
    assert_eq!(
        build_action_mask(&runner.game, 0)[main_bit],
        0.0,
        "an event-window <Delay> must not expose a [Main]-phase activation action"
    );
}
