use crate::dsl_card_data::compiled;

use digimon_engine::enums::CardColor;
use digimon_engine::permanent::OptionState;

use super::support::{field_contains, hand_index, red_sw, select_hand_card, DebugRunner};

const CARD_ID: &str = "EX12-071";

#[test]
fn ex12_071_has_sw_use_requirement() {
    let card = compiled(CARD_ID);
    let requirement = card
        .use_requirement
        .as_ref()
        .and_then(|pred| pred.any_field_permanent.as_ref())
        .expect("EX12-071 prints Use Req. ([SW] trait)");

    assert_eq!(
        requirement.predicate.trait_has.as_deref(),
        Some("SW"),
        "Use Req. should look for an own [SW] trait permanent"
    );
}

// ─── <Use Req. ([SW] trait)> met by a lone Saneiketsu Mega ──────────────────
//
// Nezhamon (EX12-019), Erlangmon (EX12-034) and Takutoumon (EX12-057) print
// .../Shambala/SW on the card face. Each has two of Saneiketsu Invitation's three
// colours (black/blue/red), so alone on the field it fails the colour requirement
// (CR 4.3 4-22-3: all of an Option's colours) and only the <Use Req.> (16-42-3)
// lets the Option be used. cards.json and the YAML left [SW] out, so the Option
// stayed in hand. DCGO EX12_071.cs:16-20 `UseRequirements` gates on
// `cardSource.EqualsTraits("SW")`.

const SANEIKETSU_MEGAS: [&str; 3] = ["EX12-019", "EX12-034", "EX12-057"];

/// EX12-071 in hand, `mega` alone in the battle area, enough memory.
fn lone_mega_board(runner: DebugRunner, mega: &str) -> DebugRunner {
    let mut runner = runner;
    runner.place_on_field(0, mega, Some(0));
    runner
}

fn invitation_usable(runner: &mut DebugRunner) -> bool {
    let slot = hand_index(runner, 0, CARD_ID);
    runner.game.play_option_from_hand(0, slot)
        != digimon_engine::selection::OptionPlayResult::Invalid
}

/// The production card (built from data/cards.json, as in every real game).
#[test]
fn ex12_071_use_req_is_met_by_a_lone_production_saneiketsu_mega() {
    let production = digimon_engine::deck_tools::full_card_data();
    for mega in SANEIKETSU_MEGAS {
        let card = production
            .get(mega)
            .unwrap_or_else(|| panic!("{mega} in production cards.json"))
            .clone();
        assert!(
            card.traits.iter().any(|t| t == "SW"),
            "{mega} prints the [SW] trait; production traits {:?}",
            card.traits
        );
        let runner = DebugRunner::builder()
            .dsl_card(CARD_ID)
            .expect("EX12-071 YAML loads")
            .add_card(card)
            .hand(0, &[CARD_ID])
            .memory(5)
            .start();
        let mut runner = lone_mega_board(runner, mega);
        assert!(
            invitation_usable(&mut runner),
            "a lone {mega} meets Saneiketsu Invitation's <Use Req. ([SW] trait)>"
        );
    }
}

/// The DSL spec (what DebugRunner fixtures see).
#[test]
fn ex12_071_use_req_is_met_by_a_lone_dsl_saneiketsu_mega() {
    for mega in SANEIKETSU_MEGAS {
        let runner = DebugRunner::builder()
            .dsl_card(CARD_ID)
            .expect("EX12-071 YAML loads")
            .dsl_card(mega)
            .expect("Saneiketsu Mega YAML loads")
            .hand(0, &[CARD_ID])
            .memory(5)
            .start();
        let mut runner = lone_mega_board(runner, mega);
        assert!(
            invitation_usable(&mut runner),
            "a lone {mega} (DSL spec) meets Saneiketsu Invitation's <Use Req. ([SW] trait)>"
        );
    }
}

/// Negative control: a red/black Mega without [SW] does not meet the colour
/// requirement (blue missing) nor the <Use Req.>.
#[test]
fn ex12_071_use_req_is_not_met_by_a_lone_red_black_digimon_without_sw() {
    let mut no_sw = super::support::digimon("NO-SW", CardColor::Red, 6, 12, 12000, &["Shambala"]);
    no_sw.colors = vec![CardColor::Red, CardColor::Black];
    let runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-071 YAML loads")
        .add_card(no_sw)
        .hand(0, &[CARD_ID])
        .memory(5)
        .start();
    let mut runner = lone_mega_board(runner, "NO-SW");
    assert!(!invitation_usable(&mut runner));
}

#[test]
fn ex12_071_main_trashes_sw_card_draws_two_and_places_delay_option() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-071 YAML loads")
        .add_card(red_sw("SW-DISCARD", 4))
        .add_card(super::support::digimon(
            "DRAW-A",
            CardColor::Black,
            3,
            3,
            3000,
            &["SW"],
        ))
        .add_card(super::support::digimon(
            "DRAW-B",
            CardColor::Blue,
            3,
            3,
            3000,
            &["SW"],
        ))
        .hand(0, &[CARD_ID, "SW-DISCARD"])
        .deck(0, &["DRAW-A", "DRAW-B"])
        .memory(3)
        .start();

    let option_slot = hand_index(&runner, 0, CARD_ID);
    assert!(runner.game.activate_hand_main(0, option_slot));
    select_hand_card(&mut runner, 0, "SW-DISCARD");
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

// ─── <Delay> digivolve: only [SW] Digimon a [Saneiketsu] card can digivolve onto ─
//
// DCGO EX12_071.cs:112-125 `CanSelectPermanentCondition` offers an own [SW]
// Digimon only when some hand card passing `CanSelectCardCondition` (IsDigimon
// && [Saneiketsu], :106-110) `CanPlayCardTargetFrame(permanent.PermanentFrame,
// false, ...)`, and opens no pick at all when no such Digimon exists
// (`HasMatchConditionPermanent`, :137). The hand pick that follows
// (`DigivolveIntoHandOrTrashCard`, CardEffectCommons.cs:858-882) offers only the
// cards that can digivolve onto the chosen Digimon. Choosing a Digimon nothing
// can digivolve onto would be a dead RL choice.

/// A [Saneiketsu] Lv.4 that digivolves from a red Lv.3 for 2.
fn saneiketsu_from_red_l3() -> digimon_engine::card_data::CardData {
    super::support::with_evo_cost(
        super::support::digimon("SANE-L4", CardColor::Red, 4, 5, 5000, &["Saneiketsu"]),
        CardColor::Red,
        3,
        2,
    )
}

/// A [Saneiketsu] Lv.6 that digivolves only from a black Lv.5.
fn saneiketsu_from_black_l5() -> digimon_engine::card_data::CardData {
    super::support::with_evo_cost(
        super::support::digimon("SANE-L6", CardColor::Black, 6, 10, 10000, &["Saneiketsu"]),
        CardColor::Black,
        5,
        3,
    )
}

/// Seat EX12-071 through its real [Main], advance to the owner's next turn
/// (CR 4.3 16-17-3), put a red Lv.5 [SW] Digimon in the battle area, push
/// `saneiketsu` into the hand, then play a red Lv.3 [SW] Digimon from the hand
/// (the <Delay> trigger) and accept the <Delay>. Returns (runner, Lv.5, Lv.3).
fn delay_digivolve_board(
    saneiketsu: &str,
) -> (
    DebugRunner,
    digimon_engine::permanent::PermanentHandle,
    digimon_engine::permanent::PermanentHandle,
) {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-071 YAML loads")
        .add_card(red_sw("SW-DISCARD", 4))
        .add_card(super::support::plain_digimon("FILL", CardColor::Red, 3, 3000))
        .add_card(red_sw("SW-PLAY", 3))
        .add_card(red_sw("SW-L5", 5))
        .add_card(saneiketsu_from_red_l3())
        .add_card(saneiketsu_from_black_l5())
        .hand(0, &[CARD_ID, "SW-DISCARD"])
        .deck(0, &["FILL"; 12])
        .deck(1, &["FILL"; 12])
        .memory(3)
        .start();
    let option_slot = hand_index(&runner, 0, CARD_ID);
    assert!(runner.game.activate_hand_main(0, option_slot));
    select_hand_card(&mut runner, 0, "SW-DISCARD");
    runner.auto_resolve().expect("resolve main");
    assert!(field_contains(&runner, 0, CARD_ID));
    runner.end_turn();
    runner.auto_resolve().expect("no selection owed");
    runner.end_turn();
    runner.auto_resolve().expect("no selection owed");
    assert_eq!(runner.game.turn_player(), 0);
    runner.game.enter_main_phase();
    runner.game.memory = 10;

    let l5 = runner.place_on_field(0, "SW-L5", None);
    runner.add_to_hand(0, saneiketsu);
    runner.add_to_hand(0, "SW-PLAY");
    let slot = hand_index(&runner, 0, "SW-PLAY");
    assert!(runner.play(0, slot).is_some(), "SW-PLAY playable");
    let view = runner
        .pending_selection_view()
        .expect("the <Delay> is offered when an [SW] Digimon is played");
    let accept = view
        .valid_action_ids
        .iter()
        .copied()
        .find(|&a| a != digimon_engine::action::space::PASS)
        .expect("an accept action");
    runner
        .execute_action(view.selecting_player, accept)
        .expect("accept the <Delay>");
    let l3 = runner.game.players[0]
        .battle_area
        .iter()
        .position(|p| p.top_card().card_id(&runner.game.card_data) == "SW-PLAY")
        .map(|index| digimon_engine::permanent::PermanentHandle { player: 0, index: index as u8 })
        .expect("SW-PLAY in the battle area");
    let l5 = runner.game.players[0]
        .battle_area
        .iter()
        .position(|p| p.top_card().card_id(&runner.game.card_data) == "SW-L5")
        .map(|index| digimon_engine::permanent::PermanentHandle { player: 0, index: index as u8 })
        .unwrap_or(l5);
    (runner, l5, l3)
}

/// Whether field permanent `h` is offered at the pending own-field pick.
fn field_offered(runner: &DebugRunner, h: digimon_engine::permanent::PermanentHandle) -> bool {
    runner.pending_selection_view().is_some_and(|v| {
        v.valid_action_ids
            .contains(&(digimon_engine::action::space::ATTACK_START + h.index as u16))
    })
}

/// Only the red Lv.3 [SW] Digimon can take SANE-L4; the Lv.5 is not offered.
#[test]
fn ex12_071_delay_offers_only_sw_digimon_a_saneiketsu_card_can_digivolve_onto() {
    let (runner, l5, l3) = delay_digivolve_board("SANE-L4");
    assert!(field_offered(&runner, l3), "the Lv.3 [SW] has a route: {:?}", runner.pending_selection_view());
    assert!(
        !field_offered(&runner, l5),
        "no [Saneiketsu] card in hand can digivolve onto the Lv.5: {:?}",
        runner.pending_selection_view()
    );
}

/// No [Saneiketsu] card can digivolve onto any [SW] Digimon: no pick opens
/// (DCGO EX12_071.cs:137); the <Delay> card is still trashed (its cost).
#[test]
fn ex12_071_delay_offers_no_target_when_no_saneiketsu_card_can_digivolve() {
    let (mut runner, l5, l3) = delay_digivolve_board("SANE-L6");
    assert!(
        !field_offered(&runner, l5) && !field_offered(&runner, l3),
        "nothing can digivolve: no target is offered: {:?}",
        runner.pending_selection_view()
    );
    let _ = runner.auto_resolve();
    assert!(!field_contains(&runner, 0, CARD_ID), "the <Delay> trashed EX12-071");
    assert!(
        runner.game.players[0]
            .hand
            .iter()
            .any(|c| c.card_id(&runner.game.card_data) == "SANE-L6"),
        "SANE-L6 stays in hand"
    );
}

/// Positive guard: choose the Lv.3, then SANE-L4, digivolving free.
#[test]
fn ex12_071_delay_digivolves_the_chosen_sw_digimon_free() {
    let (mut runner, _l5, l3) = delay_digivolve_board("SANE-L4");
    let memory_before = runner.memory();
    let view = runner.pending_selection_view().expect("target pick");
    runner
        .execute_action(
            view.selecting_player,
            digimon_engine::action::space::ATTACK_START + l3.index as u16,
        )
        .expect("choose the Lv.3 [SW]");
    select_hand_card(&mut runner, 0, "SANE-L4");
    let _ = runner.auto_resolve();
    let top = runner.game.players[0].battle_area[l3.index as usize]
        .top_card()
        .card_id(&runner.game.card_data)
        .to_string();
    assert_eq!(top, "SANE-L4");
    assert_eq!(runner.memory(), memory_before, "without paying the cost");
}
