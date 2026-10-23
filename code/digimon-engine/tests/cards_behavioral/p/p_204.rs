//! P-204 Release of the Sealed Knight! — image and official Q5199 verified.
//! Q5199: both Draw 2 AND placement require the optional hand-trash cost.
//! https://world.digimoncard.com/cards/?card_no=P-204&search=true
use digimon_engine::action::{build_action_mask, space};
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, CardKind, Expiry, ModifierType};
use digimon_engine::modifiers::{ModifierEntry, PlayerModifierEntry};
use digimon_engine::permanent::{OptionState, PermanentHandle};
use digimon_engine::selection::SelectionKind;

const ID: &str = "P-204";

fn digimon(id: &str, name: &str, level: u8, traits: &[&str]) -> CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Black];
    c.level = Some(level);
    c.dp = Some(12000);
    c.play_cost = 4;
    c.traits = traits.iter().map(|s| (*s).into()).collect();
    c
}

fn evolution(id: &str, name: &str, level: u8, traits: &[&str]) -> CardData {
    let mut c = digimon(id, name, level, traits);
    c.evo_costs.push(EvoCost {
        card_color: CardColor::Black as u8,
        level: 5,
        memory_cost: 5,
    });
    c
}

fn builder() -> DebugRunnerBuilder {
    let mut x_option = make_test_card("X-OPTION", "X Trait Option");
    x_option.card_kind = CardKind::Option;
    x_option.level = None;
    x_option.dp = None;
    x_option.traits = vec!["X Antibody".into()];
    let mut chron_tamer = make_test_card("CHRON-TAMER", "Chronicle Tamer");
    chron_tamer.card_kind = CardKind::Tamer;
    chron_tamer.level = None;
    chron_tamer.dp = None;
    chron_tamer.traits = vec!["Chronicle".into()];
    let mut wrong_color = digimon("WRONG-COLOR", "Chronicle Wrong Color", 5, &["Chronicle"]);
    wrong_color.colors = vec![CardColor::Yellow];
    DebugRunner::builder()
        .dsl_card(ID)
        .expect("P-204 must be implemented")
        .add_card(digimon("FILL", "Filler", 3, &[]))
        .add_card(digimon("ATTACKER", "Attacker", 3, &[]))
        .add_card(digimon("CHRON", "Chronicle Host", 5, &["Chronicle"]))
        .add_card(digimon("GRADE", "Grademon", 5, &[]))
        .add_card(digimon("NEAR-GRADE", "Grademon Variant", 5, &[]))
        .add_card(digimon("OTHER", "Other Host", 5, &[]))
        .add_card(digimon(
            "CHRON-L4",
            "Chronicle Wrong Level",
            4,
            &["Chronicle"],
        ))
        .add_card(wrong_color)
        .add_card(x_option)
        .add_card(chron_tamer)
        // Exact Alphamon is exempt from the Chronicle and level <= 6 gates.
        .add_card(evolution("ALPHA", "Alphamon", 7, &[]))
        .add_card(evolution("NEAR-ALPHA", "Alphamon Variant", 6, &[]))
        .add_card(evolution("CHRON6", "Chronicle Six", 6, &["Chronicle"]))
        .add_card(evolution("CHRON3", "Chronicle Three", 3, &["Chronicle"]))
        .add_card(evolution("CHRON7", "Chronicle Seven", 7, &["Chronicle"]))
        .add_card(evolution("OTHER6", "Other Six", 6, &[]))
        .add_card(digimon("NO-ROUTE", "Chronicle No Route", 6, &["Chronicle"]))
        .deck(0, &["FILL"; 30])
        .deck(1, &["FILL"; 30])
        .security(0, &["FILL"; 5])
        .security(1, &["FILL"; 5])
        .memory(10)
}

fn start(mut r: DebugRunner) -> DebugRunner {
    r.set_first_player(0);
    r.skip_mulligan();
    r.game.enter_main_phase();
    r
}

fn hand_index(r: &DebugRunner, p: u8, id: &str) -> usize {
    r.game.players[p as usize]
        .hand
        .iter()
        .position(|c| c.card_id(&r.game.card_data) == id)
        .unwrap_or_else(|| panic!("{id} absent from player {p} hand"))
}

fn hand_has(r: &DebugRunner, p: u8, id: &str) -> bool {
    r.game.players[p as usize]
        .hand
        .iter()
        .any(|c| c.card_id(&r.game.card_data) == id)
}

fn trash_has(r: &DebugRunner, p: u8, id: &str) -> bool {
    r.game.players[p as usize]
        .trash
        .iter()
        .any(|c| c.card_id(&r.game.card_data) == id)
}

fn find(r: &DebugRunner, p: u8, id: &str) -> PermanentHandle {
    let index = r.game.players[p as usize]
        .battle_area
        .iter()
        .position(|x| x.top_card().card_id(&r.game.card_data) == id)
        .unwrap_or_else(|| panic!("{id} absent from player {p} battle area"));
    PermanentHandle {
        player: p,
        index: index as u8,
    }
}

fn parked(r: &DebugRunner, p: u8) -> bool {
    r.game.players[p as usize].battle_area.iter().any(|x| {
        x.top_card().card_id(&r.game.card_data) == ID
            && matches!(x.option_state, OptionState::Delayed { .. })
    })
}

fn choose(r: &mut DebugRunner, action: u16) {
    let v = r.pending_selection_view().expect("player-visible choice");
    assert_eq!(
        build_action_mask(&r.game, v.selecting_player)[action as usize],
        1.0,
        "action {action} must be exposed in the mask"
    );
    r.execute_action(v.selecting_player, action).unwrap();
}

fn choose_hand(r: &mut DebugRunner, p: u8, id: &str) {
    assert_eq!(r.pending_selection_view().unwrap().selecting_player, p);
    let action = space::PLAY_HAND_START + hand_index(r, p, id) as u16;
    choose(r, action);
}

fn hand_legal(r: &DebugRunner, p: u8, id: &str) -> bool {
    let action = space::PLAY_HAND_START + hand_index(r, p, id) as u16;
    build_action_mask(&r.game, p)[action as usize] == 1.0
}

fn choose_host(r: &mut DebugRunner, p: u8, id: &str) {
    let h = find(r, p, id);
    assert_eq!(r.pending_selection_view().unwrap().selecting_player, p);
    choose(r, space::encode_attack(0, h.index as u16));
}

fn use_main(r: &mut DebugRunner) {
    let action = space::PLAY_HAND_START + hand_index(r, 0, ID) as u16;
    assert_eq!(build_action_mask(&r.game, 0)[action as usize], 1.0);
    r.game.decode_action(action, 0);
}

fn main_runner(hand: &[&str]) -> DebugRunner {
    let mut r = start(builder().hand(0, hand).start());
    r.place_on_field(0, "FILL", Some(0));
    r
}

fn delayed_runner(attack_owner: u8, hand: &[&str]) -> DebugRunner {
    let mut r = main_runner(&[ID, "X-OPTION"]);
    use_main(&mut r);
    choose_hand(&mut r, 0, "X-OPTION");
    assert!(parked(&r, 0));
    r.pass_turn();
    r.auto_resolve().unwrap();
    r.game.enter_main_phase();
    if attack_owner == 0 {
        r.pass_turn();
        r.auto_resolve().unwrap();
        r.game.enter_main_phase();
    }
    assert_eq!(r.turn_player(), attack_owner);
    r.game.memory = 3;
    for id in hand {
        r.add_to_hand(0, id);
    }
    r
}

fn attack_player(r: &mut DebugRunner, owner: u8) {
    let attacker = r.place_on_field(owner, "ATTACKER", Some(0));
    let action = space::encode_attack(attacker.index as u16, 14);
    assert_eq!(build_action_mask(&r.game, owner)[action as usize], 1.0);
    r.game.decode_action(action, owner);
}

fn accept_delay(r: &mut DebugRunner) {
    let v = r
        .pending_selection_view()
        .expect("Delay acceptance offered");
    assert_eq!(v.selecting_player, 0);
    assert_eq!(v.kind, SelectionKind::Replacement);
    assert!(v.is_optional);
    choose(r, space::REPLACEMENT_ACCEPT);
    assert!(trash_has(r, 0, ID), "carrier paid before evolution choices");
    assert!(!parked(r, 0));
}

#[test]
fn p_204_main_accepts_either_trait_on_any_card_kind_and_draws_two_then_places() {
    for discard in ["X-OPTION", "CHRON-TAMER"] {
        let mut r = main_runner(&[ID, discard, "OTHER"]);
        let deck = r.deck_size(0);
        let memory = r.memory();
        use_main(&mut r);
        assert_eq!(r.memory(), memory - 3);
        let v = r.pending_selection_view().unwrap();
        assert!(v.is_optional);
        assert!(hand_legal(&r, 0, discard));
        assert!(!hand_legal(&r, 0, "OTHER"));
        choose_hand(&mut r, 0, discard);
        assert_eq!(r.deck_size(0), deck - 2);
        assert!(trash_has(&r, 0, discard));
        assert!(hand_has(&r, 0, "OTHER"));
        assert!(parked(&r, 0));
        assert!(!trash_has(&r, 0, ID));
        assert!(r.pending_selection().is_none());
    }
}

#[test]
fn p_204_main_declining_cost_neither_draws_nor_places_q5199() {
    let mut r = main_runner(&[ID, "X-OPTION"]);
    let deck = r.deck_size(0);
    use_main(&mut r);
    choose(&mut r, space::PASS);
    assert_eq!(r.deck_size(0), deck);
    assert!(hand_has(&r, 0, "X-OPTION"));
    assert!(!parked(&r, 0));
    assert!(trash_has(&r, 0, ID));
    assert!(r.pending_selection().is_none());
}

#[test]
fn p_204_main_without_discard_candidate_neither_draws_nor_places_q5199() {
    let mut r = main_runner(&[ID, "OTHER"]);
    let deck = r.deck_size(0);
    use_main(&mut r);
    assert!(r.pending_selection().is_none());
    assert_eq!(r.deck_size(0), deck);
    assert!(hand_has(&r, 0, "OTHER"));
    assert!(!parked(&r, 0));
    assert!(trash_has(&r, 0, ID));
}

#[test]
fn p_204_delay_evolves_on_either_players_attack_without_charging_memory() {
    for owner in [0, 1] {
        for (base, result) in [("GRADE", "ALPHA"), ("CHRON", "CHRON6"), ("CHRON", "CHRON3")] {
            let mut r = delayed_runner(owner, &[result]);
            r.place_on_field(0, base, Some(0));
            let memory = r.memory();
            let deck = r.deck_size(0);
            attack_player(&mut r, owner);
            accept_delay(&mut r);
            assert!(r.pending_selection_view().unwrap().is_optional);
            choose_host(&mut r, 0, base);
            choose_hand(&mut r, 0, result);
            r.auto_resolve().unwrap();
            let h = find(&r, 0, result);
            assert!(r.game.players[0].battle_area[h.index as usize]
                .card_sources
                .iter()
                .any(|c| c.card_id(&r.game.card_data) == base));
            assert_eq!(r.memory(), memory, "free on either turn");
            assert_eq!(r.deck_size(0), deck - 1, "normal evolution bonus draw");
            assert!(r.pending_selection().is_none(), "single Delay per attack");
        }
    }
}

#[test]
fn p_204_delay_host_mask_uses_exact_grademon_or_chronicle_and_legal_routes() {
    let mut r = delayed_runner(0, &["ALPHA"]);
    for id in [
        "GRADE",
        "CHRON",
        "NEAR-GRADE",
        "OTHER",
        "CHRON-L4",
        "WRONG-COLOR",
        "CHRON-TAMER",
    ] {
        r.place_on_field(0, id, Some(0));
    }
    r.place_in_breeding(0, "CHRON");
    r.place_on_field(1, "GRADE", Some(0));
    attack_player(&mut r, 0);
    accept_delay(&mut r);
    let v = r.pending_selection_view().unwrap();
    let mut actual = v.valid_action_ids;
    actual.retain(|a| *a != space::PASS);
    actual.sort_unstable();
    let mut expected: Vec<_> = ["GRADE", "CHRON"]
        .iter()
        .map(|id| space::encode_attack(0, find(&r, 0, id).index as u16))
        .collect();
    expected.sort_unstable();
    assert_eq!(
        actual, expected,
        "only own battle Digimon with a legal result"
    );
    assert_eq!(build_action_mask(&r.game, 0)[space::PASS as usize], 1.0);
}

#[test]
fn p_204_delay_result_mask_uses_exact_alphamon_or_chronicle_level_six_or_less() {
    let ids = [
        "ALPHA",
        "CHRON6",
        "CHRON3",
        "CHRON7",
        "NEAR-ALPHA",
        "OTHER6",
        "NO-ROUTE",
        "CHRON-TAMER",
        "X-OPTION",
    ];
    let mut r = delayed_runner(0, &ids);
    r.place_on_field(0, "GRADE", Some(0));
    attack_player(&mut r, 0);
    accept_delay(&mut r);
    choose_host(&mut r, 0, "GRADE");
    for id in ids {
        assert_eq!(
            hand_legal(&r, 0, id),
            ["ALPHA", "CHRON6", "CHRON3"].contains(&id),
            "{id}"
        );
    }
    assert_eq!(build_action_mask(&r.game, 0)[space::PASS as usize], 1.0);
}

#[test]
fn p_204_delay_decline_preserves_option_for_a_later_attack() {
    let mut r = delayed_runner(0, &["ALPHA"]);
    r.place_on_field(0, "GRADE", Some(0));
    attack_player(&mut r, 0);
    choose(&mut r, space::PASS);
    r.auto_resolve().unwrap();
    assert!(parked(&r, 0));
    assert!(!trash_has(&r, 0, ID));
    attack_player(&mut r, 0);
    accept_delay(&mut r);
    choose_host(&mut r, 0, "GRADE");
    choose_hand(&mut r, 0, "ALPHA");
    r.auto_resolve().unwrap();
    find(&r, 0, "ALPHA");
}

#[test]
fn p_204_delay_host_or_result_decline_keeps_paid_carrier_in_trash() {
    for decline_result in [false, true] {
        let mut r = delayed_runner(1, &["ALPHA"]);
        r.place_on_field(0, "GRADE", Some(0));
        attack_player(&mut r, 1);
        accept_delay(&mut r);
        if decline_result {
            choose_host(&mut r, 0, "GRADE");
        }
        choose(&mut r, space::PASS);
        r.auto_resolve().unwrap();
        assert!(trash_has(&r, 0, ID));
        assert!(hand_has(&r, 0, "ALPHA"));
        find(&r, 0, "GRADE");
    }
}

#[test]
fn p_204_delay_can_be_paid_without_an_evolution_candidate() {
    let mut r = delayed_runner(1, &["NO-ROUTE"]);
    r.place_on_field(0, "CHRON", Some(0));
    attack_player(&mut r, 1);
    accept_delay(&mut r);
    r.auto_resolve().unwrap();
    assert!(r.pending_selection().is_none());
    assert!(hand_has(&r, 0, "NO-ROUTE"));
    find(&r, 0, "CHRON");
}

#[test]
fn p_204_delay_masks_host_and_player_evolution_prohibitions() {
    for player_gate in [false, true] {
        let mut r = delayed_runner(1, &["ALPHA"]);
        let host = r.place_on_field(0, "GRADE", Some(0));
        if player_gate {
            r.game.modifiers.add_player_modifier(
                0,
                PlayerModifierEntry::simple(
                    ModifierType::CannotDigivolveDigimonByEffect,
                    0,
                    Expiry::Permanent,
                    None,
                    1,
                ),
            );
        } else {
            r.game.modifiers.add(
                host,
                ModifierEntry::simple(ModifierType::CannotDigivolve, 0, Expiry::Permanent, 1),
            );
        }
        attack_player(&mut r, 1);
        accept_delay(&mut r);
        r.auto_resolve().unwrap();
        assert!(r.pending_selection().is_none());
        assert!(hand_has(&r, 0, "ALPHA"));
        find(&r, 0, "GRADE");
    }
}

#[test]
fn p_204_delay_does_not_trigger_on_attacks_against_digimon() {
    for owner in [0, 1] {
        let mut r = delayed_runner(owner, &["ALPHA"]);
        r.place_on_field(0, "GRADE", Some(0));
        let attacker = r.place_on_field(owner, "ATTACKER", Some(0));
        let defender = r.place_on_field(1 - owner, "FILL", Some(0));
        r.game.players[(1 - owner) as usize].battle_area[defender.index as usize].is_suspended =
            true;
        let action = space::encode_attack(attacker.index as u16, defender.index as u16);
        assert_eq!(build_action_mask(&r.game, owner)[action as usize], 1.0);
        r.game.decode_action(action, owner);
        r.auto_resolve().unwrap();
        assert!(r.pending_selection().is_none());
        assert!(parked(&r, 0));
    }
}

#[test]
fn p_204_delay_cannot_activate_during_the_placing_turn() {
    let mut r = main_runner(&[ID, "X-OPTION", "ALPHA"]);
    r.place_on_field(0, "GRADE", Some(0));
    use_main(&mut r);
    choose_hand(&mut r, 0, "X-OPTION");
    attack_player(&mut r, 0);
    r.auto_resolve().unwrap();
    assert!(r.pending_selection().is_none());
    assert!(parked(&r, 0));
    assert!(hand_has(&r, 0, "ALPHA"));
}

fn security_runner(hand: &[&str]) -> DebugRunner {
    let mut r = start(builder().security(1, &["FILL", ID]).hand(1, hand).start());
    // No black card on the defending board: security's Main ignores use color/cost.
    attack_player(&mut r, 0);
    r
}

#[test]
fn p_204_security_main_pays_trait_discard_draws_two_and_places_without_use_cost() {
    for discard in ["X-OPTION", "CHRON-TAMER"] {
        let mut r = security_runner(&[discard, "OTHER"]);
        let deck = r.deck_size(1);
        let memory = r.memory();
        assert_eq!(r.pending_selection_view().unwrap().selecting_player, 1);
        assert!(!hand_legal(&r, 1, "OTHER"));
        choose_hand(&mut r, 1, discard);
        r.auto_resolve().unwrap();
        assert_eq!(r.deck_size(1), deck - 2);
        assert_eq!(r.memory(), memory);
        assert!(trash_has(&r, 1, discard));
        assert!(parked(&r, 1));
        assert!(!trash_has(&r, 1, ID));
    }
}

#[test]
fn p_204_security_decline_or_missing_discard_cannot_place_q5199() {
    for can_discard in [false, true] {
        let mut r = security_runner(if can_discard {
            &["X-OPTION"]
        } else {
            &["OTHER"]
        });
        let deck = r.deck_size(1);
        if can_discard {
            choose(&mut r, space::PASS);
        }
        r.auto_resolve().unwrap();
        assert_eq!(r.deck_size(1), deck);
        assert!(!parked(&r, 1));
        assert!(trash_has(&r, 1, ID));
        assert!(r.pending_selection().is_none());
    }
}
