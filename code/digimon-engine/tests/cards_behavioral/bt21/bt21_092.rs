use digimon_engine::action::space::{HAND_EFFECT_START, PASS, PLAY_HAND_START};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, make_test_card_with_level, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, PlayerId};
use digimon_engine::selection::SelectionKind;

const CARD_ID: &str = "BT21-092";

fn xros_digimon(card_id: &str, name: &str, level: u8, cost: u16) -> CardData {
    let mut card = make_test_card_with_level(card_id, name, level);
    card.colors = vec![CardColor::Red];
    card.traits = vec!["Xros Heart".to_string()];
    card.play_cost = cost;
    card
}

fn tamer(card_id: &str) -> CardData {
    let mut card = make_test_card(card_id, card_id);
    card.card_kind = CardKind::Tamer;
    card.level = None;
    card.dp = None;
    card.play_cost = 3;
    card
}

fn hand_index(runner: &DebugRunner, player: PlayerId, card_id: &str) -> usize {
    runner.game.players[player as usize]
        .hand
        .iter()
        .position(|card| card.card_id(&runner.game.card_data) == card_id)
        .unwrap_or_else(|| panic!("{card_id} must be in player {player}'s hand"))
}

fn drive_until_hand_play(runner: &mut DebugRunner, player: PlayerId, card_id: &str) {
    for _ in 0..12 {
        let view = runner
            .pending_selection_view()
            .expect("BT21-092 should keep a pending selection until the hand play");
        let hand_index = hand_index(runner, player, card_id) as u16;
        let play_action = PLAY_HAND_START + hand_index;
        let hand_effect_action = HAND_EFFECT_START + hand_index;
        if matches!(view.kind, SelectionKind::Hand)
            || view.valid_action_ids.contains(&play_action)
            || view.valid_action_ids.contains(&hand_effect_action)
        {
            let action = if view.valid_action_ids.contains(&play_action) {
                play_action
            } else {
                hand_effect_action
            };
            assert!(
                view.valid_action_ids.contains(&action),
                "{card_id} should be legal to play after BT21-092 moves sources; view={view:?}"
            );
            runner
                .execute_action(player, action)
                .expect("select BT21-092 reduced hand play");
            return;
        }

        let action = view
            .valid_action_ids
            .iter()
            .copied()
            .find(|&action| action != PASS)
            .unwrap_or_else(|| panic!("BT21-092 prompt had no non-PASS action: {view:?}"));
        runner
            .execute_action(player, action)
            .expect("advance BT21-092 prompt");
    }

    panic!("BT21-092 never reached the reduced hand-play prompt");
}

#[test]
fn bt21_092_moves_all_digimon_sources_under_tamer_and_reduces_followup_play_by_moved_count() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT21-092 YAML loads")
        .add_card(xros_digimon("XH-HOST", "Xros Host", 4, 4))
        .add_card(xros_digimon("SOURCE-A", "Source A", 3, 3))
        .add_card(xros_digimon("SOURCE-B", "Source B", 3, 3))
        .add_card(xros_digimon("SOURCE-C", "Source C", 3, 3))
        .add_card(xros_digimon("XH-HAND", "Xros Hand", 5, 6))
        .add_card(tamer("TAMER"))
        .hand(0, &[CARD_ID, "XH-HAND"])
        .memory(10)
        .start();

    let host = runner.place_on_field(0, "XH-HOST", Some(0));
    runner.push_source(host, "SOURCE-A");
    runner.push_source(host, "SOURCE-B");
    runner.push_source(host, "SOURCE-C");
    let tamer = runner.place_on_field(0, "TAMER", Some(0));
    let host_sources_before = runner.game.players[0].battle_area[host.index as usize]
        .card_sources
        .len();
    let tamer_sources_before = runner.game.players[0].battle_area[tamer.index as usize]
        .card_sources
        .len();

    runner.game.enter_main_phase();
    let memory_before = runner.memory();
    let _ = runner.game.play_option_from_hand(0, 0);
    drive_until_hand_play(&mut runner, 0, "XH-HAND");
    runner.auto_resolve().expect("finish BT21-092");

    let host_sources_after = runner.game.players[0].battle_area[host.index as usize]
        .card_sources
        .len();
    let tamer_sources_after = runner.game.players[0].battle_area[tamer.index as usize]
        .card_sources
        .len();
    assert_eq!(
        host_sources_before - host_sources_after,
        3,
        "BT21-092 should move all three Digimon source cards off the chosen Xros Heart stack"
    );
    assert_eq!(
        tamer_sources_after - tamer_sources_before,
        3,
        "BT21-092 should place the moved source cards under the chosen Tamer"
    );
    assert_eq!(
        memory_before - runner.memory(),
        5,
        "BT21-092 should pay option cost 2 plus XH-HAND cost 6 reduced by three moved cards"
    );
    assert!(
        runner.game.players[0]
            .battle_area
            .iter()
            .any(|permanent| permanent.top_card().card_id(&runner.game.card_data) == "XH-HAND"),
        "BT21-092 should play the selected Xros Heart card from hand"
    );
}

// ─── Candidate legality ─────────────────────────────────────────────────────
// [Main]: DCGO BT21_092.cs:52-61 `CanSelectCardCondiition` = `IsDigimon &&
// EqualsTraits("Xros Heart")`, then `PlayPermanentCards(payCost: true,
// fixedCost: BasePlayCostFromEntity - cardsMoved)` (BT21_092.cs:176-187),
// which refuses a card it can't place or pay for. DCGO is looser than the
// rules here: the pick is for "play", and a card that can't be played (no
// free slot, a play prohibition, or a cost the player can't pay) can't be
// chosen for it (CR 4.3 1-3-11-1: a card can be declared for play only if it
// can be played, and not if its cost can't be paid). The filter adds `can_play_from_effect_without_cost` +
// `affordable_with_cost` (the verb's own `reduce: moved_count`).
//
// [Security]: DCGO BT21_092.cs:231-245 `HasPlayCost && BasePlayCostFromEntity
// <= 5 && EqualsTraits("Xros Heart") && !IsDigiEgg`. `HasPlayCost` is false
// for an Option (CEntity_Base.cs:340), and a Digi-Egg has no play cost (CR 4.3
// 7-1-1), so neither is offered; DCGO has no `CanPlayAsNewPermanent` check, so
// the playability half follows the rules (CR 4.3 1-3-11-1).

fn xros_card(card_id: &str, kind: CardKind, cost: u16) -> CardData {
    let mut card = make_test_card(card_id, card_id);
    card.card_kind = kind;
    card.colors = vec![CardColor::Red];
    card.traits = vec!["Xros Heart".to_string()];
    card.play_cost = cost;
    if !matches!(kind, CardKind::Digimon) {
        card.level = None;
        card.dp = None;
    }
    card
}

fn hand_ids_of(runner: &DebugRunner, player: PlayerId) -> Vec<String> {
    runner.game.players[player as usize]
        .hand
        .iter()
        .map(|card| card.card_id(&runner.game.card_data).to_string())
        .collect()
}

fn lock_player(runner: &mut DebugRunner, player: PlayerId, modifier: digimon_engine::enums::ModifierType) {
    use digimon_engine::enums::Expiry;
    use digimon_engine::modifiers::PlayerModifierEntry;
    runner.game.modifiers.add_player_modifier(
        player,
        PlayerModifierEntry::simple(modifier, 0, Expiry::Permanent, None, 1 - player),
    );
}

/// [Main] fixture: a Xros Heart host with three Digimon sources + a Tamer on
/// the field, BT21-092 first in hand followed by `hand`, 10 memory.
fn main_runner(hand: &[&str]) -> DebugRunner {
    let mut ids = vec![CARD_ID];
    ids.extend_from_slice(hand);
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT21-092 YAML loads")
        .add_card(xros_digimon("XH-HOST", "Xros Host", 4, 4))
        .add_card(xros_digimon("SOURCE-A", "Source A", 3, 3))
        .add_card(xros_digimon("SOURCE-B", "Source B", 3, 3))
        .add_card(xros_digimon("SOURCE-C", "Source C", 3, 3))
        .add_card(xros_digimon("XH-HAND", "Xros Hand", 5, 6))
        // 40 - 3 moved = 37 against 8 memory after the Option: unpayable.
        .add_card(xros_digimon("XH-PRICEY", "Xros Pricey", 7, 40))
        // 20 printed: 8 - 20 = -12 is past the -10 floor, but the 3 moved
        // cards bring it to 17 (8 - 17 = -9), so it is payable.
        .add_card(xros_digimon("XH-REACH", "Xros Reach", 7, 20))
        .add_card(tamer("TAMER"))
        .hand(0, &ids)
        .memory(10)
        .start();
    let host = runner.place_on_field(0, "XH-HOST", Some(0));
    runner.push_source(host, "SOURCE-A");
    runner.push_source(host, "SOURCE-B");
    runner.push_source(host, "SOURCE-C");
    runner.place_on_field(0, "TAMER", Some(0));
    runner.game.enter_main_phase();
    runner
}

/// Use BT21-092, answer the field prompts, and return the card ids the [Main]
/// hand-play pick offers (empty when it does not install).
fn main_hand_offered(runner: &mut DebugRunner) -> Vec<String> {
    let _ = runner.game.play_option_from_hand(0, 0);
    for _ in 0..12 {
        let Some(view) = runner.pending_selection_view() else {
            return Vec::new();
        };
        if matches!(view.kind, SelectionKind::Hand) {
            let hand = hand_ids_of(runner, 0);
            let mut offered: Vec<String> = view
                .valid_action_ids
                .iter()
                .filter(|&&a| a != PASS)
                .map(|&a| {
                    let i = if a < HAND_EFFECT_START { a - PLAY_HAND_START } else { a - HAND_EFFECT_START };
                    hand[i as usize].clone()
                })
                .collect();
            offered.sort();
            return offered;
        }
        let action = view
            .valid_action_ids
            .iter()
            .copied()
            .find(|&action| action != PASS)
            .unwrap_or_else(|| panic!("BT21-092 prompt had no non-PASS action: {view:?}"));
        runner
            .execute_action(view.selecting_player, action)
            .expect("advance BT21-092 prompt");
    }
    panic!("BT21-092 [Main] never settled");
}

/// A Xros Heart Digimon whose reduced cost the player can't pay is not offered.
#[test]
fn bt21_092_main_excludes_unaffordable_xros_heart_digimon() {
    let mut runner = main_runner(&["XH-HAND", "XH-PRICEY"]);
    assert_eq!(main_hand_offered(&mut runner), vec!["XH-HAND".to_string()]);
}

/// The affordability check prices the card with THIS effect's reduction (the
/// number of cards moved), exactly as the play charges it: a card payable only
/// after the reduction is offered, and the play charges the reduced cost.
#[test]
fn bt21_092_main_affordability_counts_moved_cards() {
    let mut runner = main_runner(&["XH-REACH", "XH-PRICEY"]);
    assert_eq!(main_hand_offered(&mut runner), vec!["XH-REACH".to_string()]);
    let reach = PLAY_HAND_START + hand_index(&runner, 0, "XH-REACH") as u16;
    let view = runner.pending_selection_view().expect("hand pick pending");
    let action = if view.valid_action_ids.contains(&reach) {
        reach
    } else {
        HAND_EFFECT_START + hand_index(&runner, 0, "XH-REACH") as u16
    };
    runner.execute_action(0, action).expect("play XH-REACH");
    let _ = runner.auto_resolve();
    assert!(runner.game.players[0]
        .battle_area
        .iter()
        .any(|p| p.top_card().card_id(&runner.game.card_data) == "XH-REACH"));
    // `game.memory` is stored from the turn player's side; going below 0
    // passes the turn, so read it back from player 0's side.
    let p0_memory = if runner.game.turn_player() == 0 {
        runner.memory()
    } else {
        -runner.memory()
    };
    assert_eq!(p0_memory, 8 - 17, "printed 20 - 3 moved = 17 paid");
}

/// Full battle area: no Xros Heart Digimon can be played, so none is offered.
#[test]
fn bt21_092_main_excludes_digimon_when_battle_area_full() {
    let mut runner = main_runner(&["XH-HAND"]);
    runner.game.rules.field_slots = runner.game.players[0].battle_area.len() as u8;
    assert!(main_hand_offered(&mut runner).is_empty());
    let _ = runner.auto_resolve();
    assert!(hand_ids_of(&runner, 0).contains(&"XH-HAND".to_string()));
}

/// "You can't play Digimon by effects": none is offered.
#[test]
fn bt21_092_main_excludes_digimon_under_effect_play_prohibition() {
    let mut runner = main_runner(&["XH-HAND"]);
    lock_player(&mut runner, 0, digimon_engine::enums::ModifierType::CannotPlayDigimonByEffect);
    assert!(main_hand_offered(&mut runner).is_empty());
}

/// The host pick is "1 of your [Xros Heart] trait Digimon" with no source
/// requirement (printed text; DCGO BT21_092.cs:64-74 `CanSelectDigimonCondition`
/// checks only the trait). A Xros Heart Digimon with no digivolution cards is a
/// legal host: choosing it places 0 cards, so the follow-up play gets no
/// reduction and the sourced stack keeps its cards.
#[test]
fn bt21_092_main_host_pick_offers_xros_heart_digimon_without_sources() {
    let mut runner = main_runner(&["XH-HAND"]);
    let bare = runner.place_on_field(0, "XH-HOST", Some(0));
    let sourced = runner.game.players[0]
        .battle_area
        .iter()
        .position(|p| p.card_sources.len() > 1)
        .expect("sourced host on field");
    let _ = runner.game.play_option_from_hand(0, 0);
    let view = runner.pending_selection_view().expect("host pick pending");
    let bare_action = digimon_engine::action::space::ATTACK_START + bare.index as u16;
    let sourced_action = digimon_engine::action::space::ATTACK_START + sourced as u16;
    assert!(!view.valid_action_ids.contains(&PASS), "mandatory host pick");
    assert!(
        view.valid_action_ids.contains(&bare_action) && view.valid_action_ids.contains(&sourced_action),
        "both Xros Heart Digimon are legal hosts; view={view:?}"
    );
    runner.execute_action(0, bare_action).expect("pick the bare host");
    let sourced_before = runner.game.players[0].battle_area[sourced].card_sources.len();
    drive_until_hand_play(&mut runner, 0, "XH-HAND");
    let _ = runner.auto_resolve();
    assert_eq!(
        runner.game.players[0].battle_area[sourced].card_sources.len(),
        sourced_before,
        "the unchosen stack keeps its sources"
    );
    // 10 - 2 (Option) - 6 (XH-HAND, 0 cards placed so no reduction) = 2.
    assert_eq!(runner.memory(), 2);
}

// ─── [Security] ─────────────────────────────────────────────────────────────

fn security_runner(hand: &[&str], trash: &[&str]) -> DebugRunner {
    let mut attacker = make_test_card_with_level("ATK", "Attacker", 4);
    attacker.colors = vec![CardColor::Blue];
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT21-092 YAML loads")
        .add_card(attacker)
        .add_card(make_test_card("FILL", "Filler"))
        .add_card(xros_card("XH-EGG", CardKind::DigiEgg, 0))
        .add_card(xros_card("XH-OPT", CardKind::Option, 2))
        .add_card(xros_card("XH-TAMER", CardKind::Tamer, 3))
        .add_card(xros_card("XH-DIGI", CardKind::Digimon, 5))
        .add_card(xros_card("XH-DIGI6", CardKind::Digimon, 6))
        .hand(1, hand)
        .security(1, &[CARD_ID])
        .deck(0, &["FILL", "FILL"])
        .deck(1, &["FILL", "FILL"])
        .memory(0)
        .start();
    for id in trash {
        runner.inject_trash(1, id);
    }
    runner
}

fn attack_into_security(runner: &mut DebugRunner) {
    let atk = runner.place_on_field(0, "ATK", Some(0));
    runner.attack_player(atk, 1, false);
}

/// The (zone, card id) rows the [Security] union pick offers. `true` = trash.
fn union_offered(runner: &DebugRunner, owner: PlayerId) -> Vec<(bool, String)> {
    use digimon_engine::owned_card_choice::{CardChoiceOperation, CardChoiceZone};
    let Some(choice) = runner
        .game
        .pending_selection
        .as_ref()
        .and_then(|pending| pending.card_choice.as_ref())
    else {
        return Vec::new();
    };
    let mut rows: Vec<(bool, String)> = choice
        .view(&runner.game, owner)
        .rows
        .iter()
        .filter(|row| matches!(row.operation, CardChoiceOperation::CommitCard))
        .filter_map(|row| {
            let trash = matches!(row.zone, Some(CardChoiceZone::Trash));
            row.card_id.clone().map(|id| (trash, id))
        })
        .collect();
    rows.sort();
    rows
}

/// Pickmons-style Xros Heart Digi-Egg (BT10-003) and a Xros Heart Option
/// (Hero of the Skies! BT10-095 shape) are never "play" candidates; the cost-6
/// Digimon is over the cap. Only the Tamer and the cost-5 Digimon are offered.
#[test]
fn bt21_092_security_excludes_digi_egg_option_and_cost_six() {
    let mut runner = security_runner(&["XH-EGG", "XH-OPT", "XH-TAMER", "XH-DIGI6"], &["XH-DIGI"]);
    attack_into_security(&mut runner);
    assert_eq!(
        union_offered(&runner, 1),
        vec![(false, "XH-TAMER".to_string()), (true, "XH-DIGI".to_string())]
    );
}

/// Full battle area: nothing is offered.
#[test]
fn bt21_092_security_excludes_all_when_battle_area_full() {
    let mut runner = security_runner(&["XH-TAMER"], &["XH-DIGI"]);
    runner.place_on_field(1, "FILL", Some(0));
    let atk = runner.place_on_field(0, "ATK", Some(0));
    runner.game.rules.field_slots = 1;
    runner.attack_player(atk, 1, false);
    assert!(union_offered(&runner, 1).is_empty(), "{:?}", union_offered(&runner, 1));
}

/// "Can't play cards from the trash": the trash copy is not offered.
#[test]
fn bt21_092_security_excludes_trash_card_while_trash_play_locked() {
    let mut runner = security_runner(&["XH-TAMER"], &["XH-DIGI"]);
    lock_player(&mut runner, 1, digimon_engine::enums::ModifierType::CannotPlayFromTrash);
    attack_into_security(&mut runner);
    assert_eq!(union_offered(&runner, 1), vec![(false, "XH-TAMER".to_string())]);
}

/// "Can't play Tamers by effects": the Tamer is not offered, the Digimon is.
#[test]
fn bt21_092_security_excludes_tamer_under_tamer_play_prohibition() {
    let mut runner = security_runner(&["XH-TAMER"], &["XH-DIGI"]);
    lock_player(&mut runner, 1, digimon_engine::enums::ModifierType::CannotPlayTamerByEffect);
    attack_into_security(&mut runner);
    assert_eq!(union_offered(&runner, 1), vec![(true, "XH-DIGI".to_string())]);
}

/// Regression guard: the chosen card is played free from its zone.
#[test]
fn bt21_092_security_plays_trash_digimon_free() {
    let mut runner = security_runner(&["XH-TAMER"], &["XH-DIGI"]);
    attack_into_security(&mut runner);
    let action = owned_union_fixture::action(&runner.game, 1, "XH-DIGI", true)
        .expect("eligible trash row");
    runner.execute_action(1, action).expect("play XH-DIGI");
    let _ = runner.auto_resolve();
    assert!(runner.game.players[1]
        .battle_area
        .iter()
        .any(|p| p.top_card().card_id(&runner.game.card_data) == "XH-DIGI"));
    assert_eq!(runner.memory(), 0, "without paying the cost");
}

// ─── Colour-requirement bypass ──────────────────────────────────────────────
// "While you have a Digimon with the [Xros Heart] trait, you can ignore this
// card's color requirements." DCGO BT21_092.cs:24-25 `CanUseCondition =
// HasMatchConditionOwnersPermanent(TopCard.IsDigimon && EqualsTraits("Xros
// Heart"))`.

fn colour_runner(field: CardData) -> DebugRunner {
    let id = field.card_id.clone();
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT21-092 YAML loads")
        .add_card(field)
        .hand(0, &[CARD_ID])
        .memory(10)
        .start();
    runner.set_first_player(0);
    runner.place_on_field(0, &id, Some(0));
    runner.game.enter_main_phase();
    runner
}

#[test]
fn bt21_092_blue_xros_heart_digimon_lets_red_option_ignore_colour() {
    let mut blue = xros_digimon("XH-BLUE", "Blue Xros", 4, 4);
    blue.colors = vec![CardColor::Blue];
    let runner = colour_runner(blue);
    let mask = digimon_engine::build_action_mask(&runner.game, 0);
    assert_eq!(mask[PLAY_HAND_START as usize], 1.0);
}

#[test]
fn bt21_092_no_colour_bypass_without_xros_heart_digimon() {
    let mut blue = make_test_card_with_level("PLAIN-BLUE", "Plain Blue", 4);
    blue.colors = vec![CardColor::Blue];
    let runner = colour_runner(blue);
    let mask = digimon_engine::build_action_mask(&runner.game, 0);
    assert_eq!(mask[PLAY_HAND_START as usize], 0.0);
}

#[path = "../../support/owned_union_fixture.rs"]
mod owned_union_fixture;
