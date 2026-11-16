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
/// - 16-17-1 "＜Delay＞ is a keyword effect. While a card with this effect is
///   in the battle area, by trashing that card, the effect specified in
///   ＜Delay＞ will activate." — availability is bounded by presence in the
///   battle area, and by nothing else. There is no expiry clause anywhere in
///   16-17.
/// - 16-17-3 bounds it only from BELOW ("can't be activated the same turn the
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
                "placing turn is recorded for the 16-17-3 gate"
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
                "16-17-1 bounds availability by presence in the battle area only \
                 — an event-window <Delay> parks indefinitely"
            );
        }
        other => panic!("EX12-070 must park as OptionState::Delayed; got {other:?}"),
    }
}

/// The behavioural half of G-DELAY-EVENT-WINDOW-AUTOTRASHED: the carrier
/// survives the turn-end scan that the `EndOfYourNextTurn` fallback used to
/// schedule it into (`placing_turn + 2`). See the doc comment above for the
/// 16-17-1 / DCGO citations.
#[test]
fn ex12_070_delay_option_is_not_auto_trashed_after_the_owners_next_turn() {
    let mut runner = builder_with_deck_fuel();
    let placing_turn = place_via_main(&mut runner);

    // Walk well past the owner's NEXT turn — the window the old
    // `EndOfYourNextTurn` fallback scheduled (`placing_turn + 2`).
    for _ in 0..5 {
        runner.end_turn();
        runner
            .auto_resolve()
            .expect("no selection is owed by a turn flip");
        assert!(
            field_contains(&runner, 0, CARD_ID),
            "EX12-070 must stay in the battle area (16-17-1): turn {} (placed on {placing_turn})",
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

    // Advance to the owner's next turn, where 16-17-3 no longer blocks.
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

// ─── <Delay> hand pick: only [Sanmyojin] Digimon the free play can place ─────
//
// DCGO EX12_070.cs:108-114 offers `IsDigimon && HasPlayCost &&
// EqualsTraits("Sanmyojin") && CanPlayAsNewPermanent(payCost: false)` and plays
// through `PlayByEffect(canTargetCondition: CanSelectCardCondition, ...)`
// (EX12_070.cs:126-131), so a Digimon the play would refuse (an effect-play
// prohibition, no free battle-area frame) is never a candidate, and a hand with
// no candidate opens no pick. CR 4.3 7-1-1: a Digi-Egg is never played from the
// hand. A pick the play would then refuse is an illegal RL choice.

/// A [Sanmyojin] Lv.5 Digimon (play cost 7) — a legal free-play target.
fn sanmyojin(id: &str) -> digimon_engine::card_data::CardData {
    super::support::digimon(id, CardColor::Yellow, 5, 7, 7000, &["Sanmyojin"])
}

/// A [Sanmyojin] Digi-Egg: no play cost, never playable from the hand.
fn sanmyojin_egg(id: &str) -> digimon_engine::card_data::CardData {
    let mut egg = super::support::digimon(id, CardColor::Yellow, 2, 0, 0, &["Sanmyojin"]);
    egg.card_kind = digimon_engine::enums::CardKind::DigiEgg;
    egg.dp = None;
    egg
}

/// Seat EX12-070 through its real [Main], advance to the owner's next turn
/// (CR 4.3 16-17-3: not on the placing turn), put a Lv.5 [TB] Digimon in the
/// battle area and push `hand` into P0's hand. Returns the [TB] Digimon.
fn delay_board(
    hand: &[&str],
) -> (DebugRunner, digimon_engine::permanent::PermanentHandle) {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-070 YAML loads")
        .add_card(tb_digimon("TB-DISCARD", CardColor::Blue, 4, 5000))
        .add_card(tb_digimon("FILL", CardColor::Blue, 3, 3000))
        .add_card(tb_digimon("TB-LV5", CardColor::Yellow, 5, 7000))
        .add_card(sanmyojin("SANMYO"))
        .add_card(sanmyojin_egg("SANMYO-EGG"))
        .hand(0, &[CARD_ID, "TB-DISCARD"])
        .deck(0, &["FILL"; 12])
        .deck(1, &["FILL"; 12])
        .memory(3)
        .start();
    place_via_main(&mut runner);
    runner.end_turn();
    runner.auto_resolve().expect("no selection owed");
    runner.end_turn();
    runner.auto_resolve().expect("no selection owed");
    assert_eq!(runner.game.turn_player(), 0);
    let tb = runner.place_on_field(0, "TB-LV5", None);
    for id in hand {
        runner.add_to_hand(0, id);
    }
    (runner, tb)
}

/// The [TB] Digimon would leave the battle area: accept the optional <Delay>.
fn leave_and_accept_delay(runner: &mut DebugRunner, tb: digimon_engine::permanent::PermanentHandle) {
    runner
        .game
        .delete_permanent_with_cause(tb, digimon_engine::replacement::ReplacementCause::OwnEffect);
    let view = runner
        .pending_selection_view()
        .expect("the optional <Delay> replacement is offered");
    let accept = view
        .valid_action_ids
        .iter()
        .copied()
        .find(|&a| a != digimon_engine::action::space::PASS)
        .expect("an accept action");
    runner
        .execute_action(view.selecting_player, accept)
        .expect("accept the <Delay>");
}

/// The (non-PASS) entries of a pending replacement-body hand pick, which the
/// replacement flow surfaces as an `EffectChoice` labelled by card id.
fn effect_choice_entries(runner: &DebugRunner) -> Vec<(String, u16)> {
    use digimon_engine::action::space::PASS;
    use digimon_engine::selection::SelectionKind;
    let Some(view) = runner.pending_selection_view() else {
        return Vec::new();
    };
    if view.kind != SelectionKind::EffectChoice {
        return Vec::new();
    }
    view.effect_choices
        .unwrap_or_default()
        .into_iter()
        .filter(|e| e.action_id != PASS && view.valid_action_ids.contains(&e.action_id))
        .map(|e| (e.label.clone(), e.action_id))
        .collect()
}

/// Hand card ids offered at a pending hand pick, or empty when none is pending.
fn offered_hand_pick(runner: &DebugRunner) -> Vec<String> {
    use digimon_engine::action::space::{HAND_EFFECT_START, PASS, PLAY_HAND_START};
    use digimon_engine::selection::SelectionKind;
    let Some(view) = runner.pending_selection_view() else {
        return Vec::new();
    };
    if view.kind == SelectionKind::EffectChoice {
        let mut out: Vec<String> = effect_choice_entries(runner).into_iter().map(|(l, _)| l).collect();
        out.sort();
        return out;
    }
    if view.kind != SelectionKind::Hand {
        return Vec::new();
    }
    let hand: Vec<String> = runner.game.players[0]
        .hand
        .iter()
        .map(|c| c.card_id(&runner.game.card_data).to_string())
        .collect();
    let mut out: Vec<String> = view
        .valid_action_ids
        .iter()
        .filter(|&&a| a != PASS)
        .filter_map(|&a| {
            [PLAY_HAND_START, HAND_EFFECT_START]
                .iter()
                .filter_map(|&s| a.checked_sub(s))
                .find(|&i| (i as usize) < hand.len() && i < 30)
        })
        .map(|i| hand[i as usize].clone())
        .collect();
    out.sort();
    out.dedup();
    out
}

/// Positive guard: a playable [Sanmyojin] Digimon is offered and played free.
#[test]
fn ex12_070_delay_offers_and_plays_a_sanmyojin_digimon_free() {
    let (mut runner, tb) = delay_board(&["SANMYO"]);
    leave_and_accept_delay(&mut runner, tb);
    assert_eq!(offered_hand_pick(&runner), vec!["SANMYO".to_string()]);
    let memory_before = runner.memory();
    let (_, action) = effect_choice_entries(&runner)
        .into_iter()
        .find(|(label, _)| label == "SANMYO")
        .expect("SANMYO is an entry");
    runner.execute_action(0, action).expect("pick SANMYO");
    let _ = runner.auto_resolve();
    assert!(field_contains(&runner, 0, "SANMYO"), "played without paying the cost");
    assert_eq!(runner.memory(), memory_before, "without paying the cost");
}

/// "You can't play Digimon by effects" (DCGO `CanPlayAsNewPermanent` false):
/// the only [Sanmyojin] Digimon is not a candidate, so no pick opens.
#[test]
fn ex12_070_delay_offers_no_sanmyojin_digimon_under_effect_play_prohibition() {
    use digimon_engine::enums::{Expiry, ModifierType};
    use digimon_engine::modifiers::PlayerModifierEntry;
    let (mut runner, tb) = delay_board(&["SANMYO"]);
    runner.game.modifiers.add_player_modifier(
        0,
        PlayerModifierEntry::simple(ModifierType::CannotPlayDigimonByEffect, 0, Expiry::Permanent, None, 1),
    );
    leave_and_accept_delay(&mut runner, tb);
    assert!(
        offered_hand_pick(&runner).is_empty(),
        "a prohibited play is not offered: {:?}",
        runner.pending_selection_view()
    );
    let _ = runner.auto_resolve();
    assert!(hand_contains(&runner, "SANMYO"), "SANMYO stays in hand");
}

/// No free battle-area frame once the <Delay> card is trashed (DCGO
/// `CanPlayAsNewPermanent` needs an empty frame): no pick opens.
#[test]
fn ex12_070_delay_offers_no_sanmyojin_digimon_when_battle_area_full() {
    let (mut runner, tb) = delay_board(&["SANMYO"]);
    // EX12-070 + TB-LV5 occupy the area; the <Delay> trashes EX12-070, after
    // which TB-LV5 (still in the area: it only WOULD leave) fills a 1-slot area.
    runner.game.rules.field_slots = runner.game.players[0].battle_area.len() as u8 - 1;
    leave_and_accept_delay(&mut runner, tb);
    assert!(
        offered_hand_pick(&runner).is_empty(),
        "no free frame: nothing is offered: {:?}",
        runner.pending_selection_view()
    );
    let _ = runner.auto_resolve();
    assert!(hand_contains(&runner, "SANMYO"), "SANMYO stays in hand");
}

/// Regression guard (CR 4.3 7-1-1): a [Sanmyojin] Digi-Egg is never offered.
#[test]
fn ex12_070_delay_never_offers_a_sanmyojin_digi_egg() {
    let (mut runner, tb) = delay_board(&["SANMYO-EGG", "SANMYO"]);
    leave_and_accept_delay(&mut runner, tb);
    assert_eq!(offered_hand_pick(&runner), vec!["SANMYO".to_string()]);
}

fn hand_contains(runner: &DebugRunner, card_id: &str) -> bool {
    runner.game.players[0]
        .hand
        .iter()
        .any(|c| c.card_id(&runner.game.card_data) == card_id)
}
