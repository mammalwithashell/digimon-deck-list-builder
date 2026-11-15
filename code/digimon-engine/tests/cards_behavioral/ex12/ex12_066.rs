use digimon_engine::action::space::{HAND_EFFECT_START, PASS, PLAY_HAND_START};
use digimon_engine::enums::{CardColor, EffectTiming};
use digimon_engine::permanent::OptionState;
use digimon_engine::selection::{PendingSelectionView, SelectionKind, TriggerSource};

use super::support::{
    field_contains, hand_contains, hand_index, is_suspended, plain_digimon, select_first_non_pass,
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
