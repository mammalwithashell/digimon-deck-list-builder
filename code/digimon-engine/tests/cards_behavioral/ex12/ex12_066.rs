use digimon_engine::action::space::{HAND_EFFECT_START, PASS, PLAY_HAND_END, PLAY_HAND_START};
use digimon_engine::card_data::CardData;
use digimon_engine::enums::{CardColor, CardKind, EffectTiming, Expiry, ModifierType};
use digimon_engine::modifiers::PlayerModifierEntry;
use digimon_engine::permanent::OptionState;
use digimon_engine::selection::{PendingSelectionView, SelectionKind, TriggerSource};

use super::support::{
    digimon, field_contains, hand_contains, hand_index, is_suspended, plain_digimon,
    select_first_non_pass,
    select_hand_card, top_card_id, vb_digimon, with_evo_cost, DebugRunner,
};

const CARD_ID: &str = "EX12-066";
const GIANT_MEAT: &str = "EX12-073";

// DCGO EX12_066.cs:84-100 asks ONE int menu, numbered from 1, whose first two
// entries are offered only when legal: 1 "Attacking Digimon Digivolves"
// (a hand card passing `HasDigivolveCondition`), 2 "Use an Option" (a hand
// Option passing `CanSelectOptionCondition`), then always 3 "Suspend only" and
// 4 "Do not use". The engine keeps each surviving branch's original label id
// (`action_id - 30`), so these are the menu's fixed action ids.
const MENU_DIGIVOLVE: u16 = 30;
const MENU_USE_OPTION: u16 = 31;
const MENU_SUSPEND_ONLY: u16 = 32;
const MENU_DO_NOT_USE: u16 = 33;

/// A [VB] Option card of `color` with use cost `cost` (no text of its own).
fn vb_option(id: &str, color: CardColor, cost: u16) -> CardData {
    let mut card = digimon(id, color, 0, cost, 0, &["VB"]);
    card.card_kind = CardKind::Option;
    card.level = None;
    card.dp = None;
    card
}

/// Hiro (EX12-066) and a Red Lv.4 [VB] attacker on the field, `memory` memory,
/// `hand` in hand; the attacker attacks and Hiro's menu entry for `branch`
/// (0 = digivolve, 1 = use an Option) is chosen.
fn hiro_branch(extra: Vec<CardData>, hand: &[&str], memory: i16, branch: usize) -> DebugRunner {
    let mut builder = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-066 YAML loads")
        .dsl_card("EX12-069")
        .expect("EX12-069 YAML loads")
        .add_card(vb_digimon("ATTACKER", CardColor::Red, 4, 5000))
        .add_card(plain_digimon("SEC", CardColor::Purple, 3, 2000));
    for card in extra {
        builder = builder.add_card(card);
    }
    let mut runner = builder
        .hand(0, hand)
        .security(1, &["SEC"])
        .memory(memory)
        .start();
    runner.set_turn(1);
    runner.place_on_field(0, CARD_ID, Some(0));
    let attacker = runner.place_on_field(0, "ATTACKER", Some(0));
    runner.attack_player(attacker, 1, false);
    // DCGO asks ONE menu (EX12_066.cs:84-100): entry 1 digivolves, 2 uses an
    // Option; the branch index maps onto those fixed menu action ids.
    let _ = hiro_menu(&runner);
    choose(
        &mut runner,
        if branch == 0 {
            MENU_DIGIVOLVE
        } else {
            MENU_USE_OPTION
        },
    );
    runner
}

/// The card ids the pending hand pick offers, sorted (empty when no hand pick
/// is pending).
fn offered_hand_ids(runner: &DebugRunner, player: u8) -> Vec<String> {
    let Some(view) = runner.pending_selection_view() else {
        return Vec::new();
    };
    if view.kind != SelectionKind::Hand {
        return Vec::new();
    }
    let hand: Vec<String> = runner.game.players[player as usize]
        .hand
        .iter()
        .map(|c| c.card_id(&runner.game.card_data).to_string())
        .collect();
    let mut ids: Vec<String> = view
        .valid_action_ids
        .iter()
        .copied()
        .filter(|&a| a != PASS)
        .map(|a| {
            let slot = if a < PLAY_HAND_END { a - PLAY_HAND_START } else { a - HAND_EFFECT_START };
            hand[slot as usize].clone()
        })
        .collect();
    ids.sort();
    ids
}

/// The pending hand pick is optional, the action mask allows PASS, and passing
/// leaves `card_id` in hand.
fn assert_declinable_and_decline(runner: &mut DebugRunner, card_id: &str) {
    let view = runner.pending_selection_view().expect("hand pick");
    assert_eq!(view.kind, SelectionKind::Hand);
    assert!(view.is_optional, "printed 'may' must be declinable: {view:?}");
    let mask = digimon_engine::action::build_action_mask(&runner.game, view.selecting_player);
    assert_eq!(mask[PASS as usize], 1.0, "PASS must be a legal action");
    runner.execute_action(view.selecting_player, PASS).expect("decline the pick");
    let _ = runner.auto_resolve();
    assert!(hand_contains(runner, 0, card_id), "{card_id} stays in hand");
}

// ── Option branch: "You may use 1 Option card ... with the cost reduced by 2" ─
// DCGO EX12_066.cs:64-69 `CanSelectOptionCondition` is only `IsOption && (text
// Gammamon || [VB])` and EX12_066.cs:205-209 then `PlayOptionCards(payCost:
// true)`, so DCGO offers Options its own use would refuse. That is looser than
// the rules: using an Option needs its colour requirement met (CR 4.3 4-22-1..3)
// or a <Use Req.> (16-42), no "can't use Option cards" lock, and the use cost
// paid (9-1-9-2; 9-1-8: an unpayable card is not used). An impossible action is
// not carried out (1-3-2), so such a card is not a legal choice here.

/// A Purple [VB] Option with no Purple Digimon/Tamer on the field fails its
/// colour requirement: not offered; Virus Busters (EX12-069, <Use Req.> met by
/// the [VB] Tamer/Digimon) is.
#[test]
fn ex12_066_option_pick_excludes_option_whose_colour_requirement_is_unmet() {
    let runner = hiro_branch(
        vec![vb_option("VB-PURPLE-OPT", CardColor::Purple, 1)],
        &["EX12-069", "VB-PURPLE-OPT"],
        5,
        1,
    );
    assert_eq!(offered_hand_ids(&runner, 0), vec!["EX12-069".to_string()]);
}

/// A Red [VB] Option costing 15 (13 after the -2) can't be paid from 0 memory
/// (memory would fall below -10): not offered.
#[test]
fn ex12_066_option_pick_excludes_unaffordable_option() {
    let runner = hiro_branch(
        vec![vb_option("VB-PRICEY-OPT", CardColor::Red, 15)],
        &["EX12-069", "VB-PRICEY-OPT"],
        0,
        1,
    );
    assert_eq!(offered_hand_ids(&runner, 0), vec!["EX12-069".to_string()]);
}

/// "You can't use Option cards": no Option is offered and Virus Busters stays
/// in hand.
#[test]
fn ex12_066_option_pick_offers_nothing_when_options_cannot_be_used() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-066 YAML loads")
        .dsl_card("EX12-069")
        .expect("EX12-069 YAML loads")
        .add_card(vb_digimon("ATTACKER", CardColor::Red, 4, 5000))
        .add_card(plain_digimon("SEC", CardColor::Purple, 3, 2000))
        .hand(0, &["EX12-069"])
        .security(1, &["SEC"])
        .memory(5)
        .start();
    runner.set_turn(1);
    runner.place_on_field(0, CARD_ID, Some(0));
    let attacker = runner.place_on_field(0, "ATTACKER", Some(0));
    runner.game.modifiers.add_player_modifier(
        0,
        PlayerModifierEntry::simple(ModifierType::CannotUseOptionCards, 0, Expiry::Permanent, None, 1),
    );
    runner.attack_player(attacker, 1, false);
    // DCGO's menu entry 2 is offered on text alone (EX12_066.cs:64-69), so the
    // lock bites at the hand pick, not at the menu.
    let _ = hiro_menu(&runner);
    choose(&mut runner, MENU_USE_OPTION);
    assert!(offered_hand_ids(&runner, 0).is_empty());
    let _ = runner.auto_resolve();
    assert!(hand_contains(&runner, 0, "EX12-069"));
}

/// "You may use": the Option pick can be declined (DCGO EX12_066.cs:132
/// `canNoSelect: true`).
#[test]
fn ex12_066_option_pick_is_optional() {
    let mut runner = hiro_branch(Vec::new(), &["EX12-069"], 5, 1);
    assert_declinable_and_decline(&mut runner, "EX12-069");
}

// ── Digivolve branch: "It may digivolve into a Lv.6 or lower card ... -1" ────
// DCGO EX12_066.cs:113-123 `DigivolveIntoHandOrTrashCard(AttackingPermanent,
// payCost: true, reduceCostTuple: (1, null), isHand: true)`, whose
// `CanSelectCardCondition` (CardEffectCommons.cs:858-881) offers only cards that
// `CanPlayCardTargetFrame(attacker frame, PayCost: true)` with the -1: a real
// route onto the attacker AND a payable cost (CR 4.3 8-1-3-2; 1-3-2).

/// A [VB] Lv.6 whose only route onto the Red Lv.4 attacker costs 15 (14 after
/// the -1) can't be paid from 0 memory: not offered; the payable one is.
#[test]
fn ex12_066_digivolve_pick_excludes_unaffordable_card() {
    let evo = with_evo_cost(vb_digimon("VB-EVO", CardColor::Red, 6, 12000), CardColor::Red, 4, 2);
    let pricey = with_evo_cost(vb_digimon("VB-PRICEY", CardColor::Red, 6, 12000), CardColor::Red, 4, 15);
    let runner = hiro_branch(vec![evo, pricey], &["VB-EVO", "VB-PRICEY"], 0, 0);
    assert_eq!(offered_hand_ids(&runner, 0), vec!["VB-EVO".to_string()]);
}

/// A [VB] Lv.6 with no route onto the Red Lv.4 attacker (its circle is Purple)
/// is not offered. Regression guard: the compile pass adds the route check.
#[test]
fn ex12_066_digivolve_pick_excludes_card_with_no_route_onto_attacker() {
    let evo = with_evo_cost(vb_digimon("VB-EVO", CardColor::Red, 6, 12000), CardColor::Red, 4, 2);
    let no_route = with_evo_cost(vb_digimon("VB-PURPLE", CardColor::Purple, 6, 12000), CardColor::Purple, 4, 2);
    let runner = hiro_branch(vec![evo, no_route], &["VB-EVO", "VB-PURPLE"], 5, 0);
    assert_eq!(offered_hand_ids(&runner, 0), vec!["VB-EVO".to_string()]);
}

/// "It may digivolve": the digivolve pick can be declined (DCGO
/// `DigivolveIntoHandOrTrashCard` defaults `isOptional: true`).
#[test]
fn ex12_066_digivolve_pick_is_optional() {
    let evo = with_evo_cost(vb_digimon("VB-EVO", CardColor::Red, 6, 12000), CardColor::Red, 4, 2);
    let mut runner = hiro_branch(vec![evo], &["VB-EVO"], 5, 0);
    assert_declinable_and_decline(&mut runner, "VB-EVO");
}

#[test]
fn ex12_066_start_of_turn_sets_memory_to_three_when_two_or_less() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-066 YAML loads")
        .memory(2)
        .start();

    let hiro = runner.place_on_field(0, CARD_ID, Some(0));
    runner.game.enqueue_triggered(
        EffectTiming::StartOfYourTurn,
        TriggerSource::Permanent(hiro),
    );
    runner.game.drain_effect_queue();

    assert_eq!(runner.memory(), 3);
}

/// The pending Hiro menu, with its offered action ids.
fn hiro_menu(runner: &DebugRunner) -> (PendingSelectionView, Vec<u16>) {
    let view = runner
        .pending_selection_view()
        .expect("Hiro's [Your Turn] attack trigger should ask its effect menu");
    assert_eq!(
        view.kind,
        SelectionKind::EffectChoice,
        "Hiro asks one effect menu (EX12_066.cs:84-100), got {view:?}"
    );
    assert_eq!(
        runner.game.card_data_for_handle(view.source_card).map(|d| d.card_id.as_str()),
        Some(CARD_ID),
        "the menu is Hiro's"
    );
    let ids = view
        .effect_choices
        .as_ref()
        .expect("an effect menu lists its branches")
        .iter()
        .map(|entry| entry.action_id)
        .collect();
    (view, ids)
}

fn choose(runner: &mut DebugRunner, action_id: u16) {
    let view = runner.pending_selection_view().expect("a pending menu");
    assert!(
        view.valid_action_ids.contains(&action_id),
        "menu entry {action_id} should be offered; view={view:?}"
    );
    runner
        .execute_action(view.selecting_player, action_id)
        .expect("choose Hiro's menu entry");
}

fn is_delayed_option_on_field(runner: &DebugRunner, player: u8, card_id: &str) -> bool {
    runner.game.players[player as usize]
        .battle_area
        .iter()
        .any(|permanent| {
            permanent.top_card().card_id(&runner.game.card_data) == card_id
                && matches!(permanent.option_state, OptionState::Delayed { .. })
        })
}

/// The g3 shape (DCGO PvP raw-20261113T025510Z#g3, turn 6): a level-5 [VB]
/// attacker with nothing in the hand it can digivolve into, and [VB] Giant
/// Meat (EX12-073) in the hand.
fn g3_shape_runner() -> DebugRunner {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-066 YAML loads")
        .dsl_card(GIANT_MEAT)
        .expect("EX12-073 YAML loads")
        .add_card(vb_digimon("ANGEL", CardColor::White, 5, 7000))
        // A [VB] level-4 card: level 6 or lower, but it cannot digivolve onto
        // the level-5 attacker, so DCGO does not offer menu entry 1.
        .add_card(with_evo_cost(
            vb_digimon("VB-LV4", CardColor::White, 4, 4000),
            CardColor::White,
            3,
            2,
        ))
        .add_card(vb_digimon("HIT", CardColor::White, 4, 4000))
        .add_card(plain_digimon("MISS-A", CardColor::Purple, 3, 2000))
        .add_card(plain_digimon("MISS-B", CardColor::Purple, 3, 2000))
        .add_card(plain_digimon("SEC", CardColor::Purple, 3, 2000))
        .hand(0, &[GIANT_MEAT, "VB-LV4"])
        .deck(0, &["HIT", "MISS-A", "MISS-B"])
        .security(1, &["SEC"])
        .memory(4)
        .start();
    runner.set_turn(1);
    runner
}

#[test]
fn ex12_066_attack_menu_offers_use_option_suspend_only_and_do_not_use_without_a_digivolve_card() {
    let mut runner = g3_shape_runner();
    let hiro = runner.place_on_field(0, CARD_ID, Some(0));
    let attacker = runner.place_on_field(0, "ANGEL", Some(0));

    runner.attack_player(attacker, 1, false);

    let (view, ids) = hiro_menu(&runner);
    assert_eq!(
        ids,
        vec![MENU_USE_OPTION, MENU_SUSPEND_ONLY, MENU_DO_NOT_USE],
        "no hand card can digivolve onto the attacker, so DCGO offers 2 / 3 / 4 only"
    );
    assert!(
        !view.is_optional,
        "DCGO's activation is not optional (SetUpActivateClass isOptional=false); \
         declining is the menu's own \"Do not use\""
    );
    assert!(
        !is_suspended(&runner, 0, hiro.index as usize),
        "the suspend is paid after the menu, and only for entries 1-3"
    );
}

#[test]
fn ex12_066_use_option_suspends_hiro_then_asks_the_hand_option_then_uses_it_at_cost_minus_two() {
    let mut runner = g3_shape_runner();
    let hiro = runner.place_on_field(0, CARD_ID, Some(0));
    let attacker = runner.place_on_field(0, "ANGEL", Some(0));
    let memory_before = runner.memory();

    runner.attack_player(attacker, 1, false);
    let _ = hiro_menu(&runner);
    choose(&mut runner, MENU_USE_OPTION);

    assert!(is_suspended(&runner, 0, hiro.index as usize));
    let pick = runner
        .pending_selection_view()
        .expect("Hiro should ask which Option to use");
    assert_eq!(pick.kind, SelectionKind::Hand);
    assert!(
        pick.is_optional,
        "\"You may use 1 Option card\": the hand pick can choose nothing (EX12_066.cs:127 canNoSelect)"
    );
    select_hand_card(&mut runner, 0, GIANT_MEAT);

    let reveal = runner
        .pending_selection_view()
        .expect("Giant Meat's [Main] asks which revealed card to add");
    assert_ne!(reveal.kind, SelectionKind::Hand);
    select_first_non_pass(&mut runner);
    runner.auto_resolve().expect("resolve Giant Meat and the attack");

    assert!(hand_contains(&runner, 0, "HIT"));
    assert!(is_delayed_option_on_field(&runner, 0, GIANT_MEAT));
    assert_eq!(memory_before - runner.memory(), 1, "Giant Meat's cost 3 reduced by 2");
}

#[test]
fn ex12_066_use_option_hand_pick_may_choose_nothing_and_hiro_stays_suspended() {
    let mut runner = g3_shape_runner();
    let hiro = runner.place_on_field(0, CARD_ID, Some(0));
    let attacker = runner.place_on_field(0, "ANGEL", Some(0));
    let memory_before = runner.memory();

    runner.attack_player(attacker, 1, false);
    choose(&mut runner, MENU_USE_OPTION);
    let pick = runner.pending_selection_view().expect("hand pick");
    runner.execute_action(pick.selecting_player, PASS).expect("choose nothing");
    runner.auto_resolve().expect("resolve the attack");

    assert!(is_suspended(&runner, 0, hiro.index as usize));
    assert!(hand_contains(&runner, 0, GIANT_MEAT));
    assert_eq!(memory_before, runner.memory());
}

#[test]
fn ex12_066_suspend_only_suspends_hiro_and_does_nothing_else() {
    let mut runner = g3_shape_runner();
    let hiro = runner.place_on_field(0, CARD_ID, Some(0));
    let attacker = runner.place_on_field(0, "ANGEL", Some(0));
    let memory_before = runner.memory();

    runner.attack_player(attacker, 1, false);
    choose(&mut runner, MENU_SUSPEND_ONLY);
    runner.auto_resolve().expect("resolve the attack");

    assert!(
        is_suspended(&runner, 0, hiro.index as usize),
        "CR 4.3 15-7-5: the suspend may be executed whatever follows"
    );
    assert!(hand_contains(&runner, 0, GIANT_MEAT));
    assert_eq!(top_card_id(&runner, 0, attacker.index as usize), "ANGEL");
    assert_eq!(memory_before, runner.memory());
}

#[test]
fn ex12_066_do_not_use_leaves_hiro_unsuspended() {
    let mut runner = g3_shape_runner();
    let hiro = runner.place_on_field(0, CARD_ID, Some(0));
    let attacker = runner.place_on_field(0, "ANGEL", Some(0));

    runner.attack_player(attacker, 1, false);
    choose(&mut runner, MENU_DO_NOT_USE);
    runner.auto_resolve().expect("resolve the attack");

    assert!(
        !is_suspended(&runner, 0, hiro.index as usize),
        "\"Do not use\" pays no suspend (EX12_066.cs:106)"
    );
    assert!(hand_contains(&runner, 0, GIANT_MEAT));
    assert_eq!(runner.security_count(1), 0, "the attack still checks security");
}

#[test]
fn ex12_066_already_suspended_hiro_asks_nothing() {
    let mut runner = g3_shape_runner();
    let hiro = runner.place_on_field(0, CARD_ID, Some(0));
    let attacker = runner.place_on_field(0, "ANGEL", Some(0));
    runner.game.players[0].battle_area[hiro.index as usize].is_suspended = true;

    runner.attack_player(attacker, 1, false);

    let pending_from_hiro = runner.pending_selection_view().is_some_and(|view| {
        runner.game.card_data_for_handle(view.source_card).map(|d| d.card_id.as_str())
            == Some(CARD_ID)
    });
    assert!(
        !pending_from_hiro,
        "CR 4.3 15-7-4 / DCGO CanActivateSuspendCostEffect: a suspended Hiro cannot pay"
    );
}

#[test]
fn ex12_066_attack_menu_offers_all_four_entries_and_digivolves_with_cost_reduced_by_one() {
    let evo = with_evo_cost(
        vb_digimon("VB-EVO", CardColor::Red, 6, 12000),
        CardColor::Red,
        4,
        2,
    );

    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-066 YAML loads")
        .dsl_card(GIANT_MEAT)
        .expect("EX12-073 YAML loads")
        .add_card(vb_digimon("ATTACKER", CardColor::Red, 4, 5000))
        .add_card(evo)
        .add_card(plain_digimon("SEC", CardColor::Purple, 3, 2000))
        .hand(0, &["VB-EVO", GIANT_MEAT])
        .security(1, &["SEC"])
        .memory(5)
        .start();
    runner.set_turn(1);
    let hiro = runner.place_on_field(0, CARD_ID, Some(0));
    let attacker = runner.place_on_field(0, "ATTACKER", Some(0));
    let memory_before = runner.memory();

    runner.attack_player(attacker, 1, false);
    let (_, ids) = hiro_menu(&runner);
    assert_eq!(
        ids,
        vec![MENU_DIGIVOLVE, MENU_USE_OPTION, MENU_SUSPEND_ONLY, MENU_DO_NOT_USE]
    );
    choose(&mut runner, MENU_DIGIVOLVE);

    assert!(is_suspended(&runner, 0, hiro.index as usize));
    let pick = runner
        .pending_selection_view()
        .expect("Hiro should ask which hand card to digivolve into");
    assert_eq!(pick.kind, SelectionKind::Hand);
    assert!(
        pick.is_optional,
        "\"It may digivolve\": DigivolveIntoHandOrTrashCard isOptional defaults to true"
    );
    let meat_slot = hand_index(&runner, 0, GIANT_MEAT) as u16;
    assert!(
        !pick.valid_action_ids.contains(&(PLAY_HAND_START + meat_slot))
            && !pick.valid_action_ids.contains(&(HAND_EFFECT_START + meat_slot)),
        "an Option is not a digivolve candidate"
    );
    select_hand_card(&mut runner, 0, "VB-EVO");
    runner.auto_resolve().expect("resolve Hiro digivolve");

    assert_eq!(top_card_id(&runner, 0, attacker.index as usize), "VB-EVO");
    assert_eq!(memory_before - runner.memory(), 1, "digivolve cost 2 reduced by 1");
    assert!(field_contains(&runner, 0, CARD_ID));
}

#[test]
fn ex12_066_attack_trigger_can_use_a_vb_option_from_hand_with_cost_reduced() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-066 YAML loads")
        .dsl_card("EX12-069")
        .expect("EX12-069 YAML loads")
        .add_card(vb_digimon("ATTACKER", CardColor::Red, 4, 5000))
        .add_card(plain_digimon("SEC", CardColor::Purple, 3, 2000))
        .hand(0, &["EX12-069"])
        .security(1, &["SEC"])
        .memory(5)
        .start();
    runner.set_turn(1);
    let hiro = runner.place_on_field(0, CARD_ID, Some(0));
    let attacker = runner.place_on_field(0, "ATTACKER", Some(0));
    let memory_before = runner.memory();

    runner.attack_player(attacker, 1, false);
    let (_, ids) = hiro_menu(&runner);
    assert_eq!(ids, vec![MENU_USE_OPTION, MENU_SUSPEND_ONLY, MENU_DO_NOT_USE]);
    choose(&mut runner, MENU_USE_OPTION);

    let option_pick = runner
        .pending_selection_view()
        .expect("Hiro should offer the eligible VB Option in hand");
    assert_eq!(option_pick.kind, SelectionKind::Hand);
    select_hand_card(&mut runner, 0, "EX12-069");
    runner.auto_resolve().expect("resolve Hiro option use");

    assert!(is_suspended(&runner, 0, hiro.index as usize));
    assert!(runner.game.players[0]
        .security
        .iter()
        .any(|card| card.card_id(&runner.game.card_data) == "EX12-069"));
    assert_eq!(
        memory_before,
        runner.memory(),
        "cost 2 reduced by 2 is free"
    );
}

#[test]
fn ex12_066_security_plays_hiro_without_paying_cost() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-066 YAML loads")
        .add_card(plain_digimon("ATTACKER", CardColor::Purple, 4, 6000))
        .security(1, &[CARD_ID])
        .memory(0)
        .start();
    let attacker = runner.place_on_field(0, "ATTACKER", Some(0));

    runner.attack_player(attacker, 1, false);
    runner.auto_resolve().expect("resolve Hiro security effect");

    assert!(field_contains(&runner, 1, CARD_ID));
    assert_eq!(runner.security_count(1), 0);
}
