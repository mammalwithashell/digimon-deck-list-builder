//! BT20-095 Fellowship of Hope's Keepers.
//! Official post-errata text: the moved Lv.3+ breeder may
//! have ANY trait. Chronicle still qualifies the deletion trigger and result.
//! The local original-print image retains the superseded move restriction.
//! https://world.digimoncard.com/rule/pdf/errata_card_list.pdf?20250516= (p.2)
//! CR 4.3 §§15-7-4/5 and 16-17: payable optional movement, independent optional
//! evolution, and optional Delay after the placing turn.

use digimon_engine::action::{build_action_mask, space};
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::card_source::{CardHandle, CardSource};
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::modifiers::{ModifierEntry, PlayerModifierEntry};
use digimon_engine::permanent::{OptionState, PermanentHandle};
use digimon_engine::selection::SelectionKind;
use digimon_engine::{CardColor, CardKind, Expiry, ModifierType};

const ID: &str = "BT20-095";

fn card(id: &str, kind: CardKind, chronicle: bool, cost: u16) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = kind;
    c.colors = vec![CardColor::Black];
    c.play_cost = cost;
    c.dp = Some(4000);
    if kind != CardKind::Digimon {
        c.level = None;
        c.dp = None;
    }
    if chronicle {
        c.traits = vec!["Chronicle".into()];
    }
    c
}

fn evolution(id: &str, chronicle: bool, level: u8, color: CardColor) -> CardData {
    let mut c = card(id, CardKind::Digimon, chronicle, 6);
    c.level = Some(4);
    c.evo_costs = vec![EvoCost {
        card_color: color as u8,
        level,
        memory_cost: 3,
    }];
    c
}

fn builder() -> DebugRunnerBuilder {
    let mut egg = card("EGG", CardKind::DigiEgg, true, 0);
    egg.level = Some(2);
    DebugRunner::builder()
        .dsl_card(ID)
        .expect("BT20-095 must be implemented")
        .add_card(card("FILL", CardKind::Digimon, false, 3))
        .add_card(card("TAIL", CardKind::Digimon, false, 3))
        .add_card(card("OTHER", CardKind::Digimon, false, 3))
        .add_card(card("CHRON", CardKind::Digimon, true, 3))
        .add_card(card("CHRON5", CardKind::Digimon, true, 5))
        .add_card(card("CHRON6", CardKind::Digimon, true, 6))
        .add_card(card("TAMER", CardKind::Tamer, true, 5))
        .add_card(card("OPTION", CardKind::Option, true, 3))
        .add_card(egg)
        .add_card(evolution("EVOLUTION", true, 3, CardColor::Black))
        .add_card(evolution("NON-CHRON-EVO", false, 3, CardColor::Black))
        .add_card(evolution("WRONG-LEVEL", true, 5, CardColor::Black))
        .add_card(evolution("WRONG-COLOR", true, 3, CardColor::Red))
        .deck(0, &["FILL"; 30])
        .deck(1, &["FILL"; 30])
        .security(0, &["FILL"; 5])
        .security(1, &["FILL"; 5])
        .memory(10)
}

fn ready(mut r: DebugRunner) -> DebugRunner {
    r.set_first_player(0);
    r.skip_mulligan();
    r.game.enter_main_phase();
    r
}

fn ids(r: &DebugRunner, zone: &[CardSource]) -> Vec<String> {
    zone.iter()
        .map(|c| c.card_id(&r.game.card_data).to_string())
        .collect()
}

fn has(r: &DebugRunner, player: u8, id: &str, trash: bool) -> bool {
    let p = &r.game.players[player as usize];
    ids(r, if trash { &p.trash } else { &p.hand })
        .iter()
        .any(|c| c == id)
}

fn find(r: &DebugRunner, player: u8, id: &str) -> PermanentHandle {
    let index = r.game.players[player as usize]
        .battle_area
        .iter()
        .position(|p| p.top_card().card_id(&r.game.card_data) == id)
        .expect("field card exists");
    PermanentHandle {
        player,
        index: index as u8,
    }
}

fn locate(r: &DebugRunner, player: u8, card: CardHandle) -> PermanentHandle {
    let index = r.game.players[player as usize]
        .battle_area
        .iter()
        .position(|p| p.card_sources.iter().any(|c| c.handle() == card))
        .expect("physical host survives");
    PermanentHandle {
        player,
        index: index as u8,
    }
}

fn top(r: &DebugRunner, host: PermanentHandle) -> &str {
    r.game.players[host.player as usize].battle_area[host.index as usize]
        .top_card()
        .card_id(&r.game.card_data)
}

fn parked(r: &DebugRunner, owner: u8) -> bool {
    r.game.players[owner as usize].battle_area.iter().any(|p| {
        p.top_card().card_id(&r.game.card_data) == ID
            && matches!(p.option_state, OptionState::Delayed { .. })
    })
}

fn choices(r: &DebugRunner, owner: u8, count: usize, optional: bool) -> Vec<u16> {
    let view = r.pending_selection_view().expect("real player choice");
    assert_eq!(view.selecting_player, owner);
    assert_eq!(view.is_optional, optional, "{}", view.prompt);
    let actions: Vec<_> = view
        .valid_action_ids
        .iter()
        .copied()
        .filter(|a| *a != space::PASS)
        .collect();
    assert_eq!(actions.len(), count, "{}: {:?}", view.prompt, actions);
    for (a, v) in build_action_mask(&r.game, owner).iter().enumerate() {
        assert_eq!(
            *v,
            if actions.contains(&(a as u16)) || (optional && a == space::PASS as usize) {
                1.0
            } else {
                0.0
            },
            "mask {a}"
        );
    }
    assert!(build_action_mask(&r.game, 1 - owner)
        .iter()
        .all(|v| *v == 0.0));
    actions
}

fn choose(r: &mut DebugRunner, action: u16) {
    let owner = r.pending_selection_view().unwrap().selecting_player;
    assert_eq!(build_action_mask(&r.game, owner)[action as usize], 1.0);
    r.execute_action(owner, action).unwrap();
}

fn choose_first(r: &mut DebugRunner, owner: u8, count: usize, optional: bool) {
    let action = choices(r, owner, count, optional)[0];
    choose(r, action);
}

fn union_action(r: &DebugRunner, owner: u8, id: &str, trash: bool) -> u16 {
    let p = &r.game.players[owner as usize];
    let zone = if trash { &p.trash } else { &p.hand };
    let index = zone
        .iter()
        .position(|c| c.card_id(&r.game.card_data) == id)
        .expect("zone card exists");
    (if trash {
        space::TRASH_EFFECT_START
    } else {
        space::PLAY_HAND_START
    }) + index as u16
}

fn use_main(r: &mut DebugRunner) {
    let action = union_action(r, 0, ID, false);
    assert_eq!(build_action_mask(&r.game, 0)[action as usize], 1.0);
    r.game.decode_action(action, 0);
}

fn main_runner(deck: &[&str]) -> DebugRunner {
    let mut r = ready(builder().hand(0, &[ID]).deck(0, deck).start());
    r.place_on_field(0, "FILL", Some(0));
    r
}

fn reveal_action(r: &DebugRunner, id: &str) -> u16 {
    space::SEL_REVEAL_START
        + r.game
            .revealed_cards
            .iter()
            .position(|c| c.card_id(&r.game.card_data) == id)
            .expect("revealed card") as u16
}

fn finish_remainder(r: &mut DebugRunner, bottom: bool, reverse: bool) {
    assert_eq!(r.pending_kind(), Some(SelectionKind::EffectChoice));
    let action = choices(r, 0, 2, false)[usize::from(bottom)];
    choose(r, action);
    while let Some(view) = r.pending_selection_view() {
        let SelectionKind::OrderedPermutation { remaining } = view.kind else {
            panic!("remainder ordering expected: {:?}", view.kind)
        };
        let actions = choices(r, 0, remaining as usize, false);
        choose(
            r,
            if reverse {
                *actions.last().unwrap()
            } else {
                actions[0]
            },
        );
    }
    assert!(r.game.revealed_cards.is_empty());
    assert!(parked(r, 0));
}

fn placed() -> DebugRunner {
    let mut r = main_runner(&["FILL"; 30]);
    use_main(&mut r);
    finish_remainder(&mut r, true, false);
    r
}

fn delayed(turn_player: u8) -> DebugRunner {
    let mut r = placed();
    r.pass_turn();
    r.auto_resolve().unwrap();
    r.game.enter_main_phase();
    if turn_player == 0 {
        r.pass_turn();
        r.auto_resolve().unwrap();
        r.game.enter_main_phase();
    }
    assert_eq!(r.turn_player(), turn_player);
    r.game.set_memory(5);
    r
}

fn delete(r: &mut DebugRunner, owner: u8, id: &str) {
    let victim = r.place_on_field(owner, id, Some(0));
    r.game.delete_permanent_with_effects(victim);
    r.game.drain_effect_queue();
}

fn accept_delay(r: &mut DebugRunner) {
    choices(r, 0, 1, true);
    choose(r, space::REPLACEMENT_ACCEPT);
    assert!(has(r, 0, ID, true), "Delay trash is paid before movement");
    assert!(!parked(r, 0));
}

fn accept_move(r: &mut DebugRunner) {
    assert_eq!(
        choices(r, 0, 1, true),
        vec![space::encode_breeding_select(0).unwrap()]
    );
    choose(r, space::encode_breeding_select(0).unwrap());
    assert!(r.game.players[0].breeding_area.is_none());
}

#[test]
fn bt20_095_main_mandatorily_adds_a_chronicle_card_of_each_kind() {
    for pick in ["CHRON", "TAMER", "OPTION", "EGG"] {
        let mut r = main_runner(&["TAIL", "OTHER", pick, "FILL"]);
        use_main(&mut r);
        assert_eq!(r.memory(), 7);
        assert!(matches!(
            r.pending_kind(),
            Some(SelectionKind::RevealBucket { .. })
        ));
        assert_eq!(choices(&r, 0, 1, false), vec![reveal_action(&r, pick)]);
        let action = reveal_action(&r, pick);
        choose(&mut r, action);
        finish_remainder(&mut r, true, false);
        assert_eq!(ids(&r, &r.game.players[0].hand), [pick]);
        assert!(!has(&r, 0, ID, true));
        assert_eq!(r.deck_size(0), 3);
    }
}

#[test]
fn bt20_095_main_exposes_all_matching_cards_and_both_remainder_orders_at_either_end() {
    for bottom in [false, true] {
        for reverse in [false, true] {
            let mut r = main_runner(&["TAIL", "CHRON", "TAMER", "OPTION"]);
            use_main(&mut r);
            assert_eq!(choices(&r, 0, 3, false).len(), 3);
            let action = reveal_action(&r, "TAMER");
            choose(&mut r, action);
            finish_remainder(&mut r, bottom, reverse);
            let ordered = if reverse {
                vec!["CHRON", "OPTION"]
            } else {
                vec!["OPTION", "CHRON"]
            };
            let mut expected: Vec<&str> = if bottom { vec!["TAIL"] } else { vec![] };
            expected.extend(ordered);
            if !bottom {
                expected.push("TAIL");
            }
            let actual: Vec<_> = r.game.players[0]
                .deck
                .iter()
                .rev()
                .map(|c| c.card_id(&r.game.card_data))
                .collect();
            assert_eq!(actual, expected, "bottom {bottom}, reverse {reverse}");
            assert!(has(&r, 0, "TAMER", false));
        }
    }
}

#[test]
fn bt20_095_main_without_a_match_returns_every_revealed_card_and_still_places() {
    let mut r = main_runner(&["TAIL", "OTHER", "FILL", "OTHER"]);
    use_main(&mut r);
    finish_remainder(&mut r, false, true);
    assert_eq!(r.deck_size(0), 4);
    assert!(r.game.players[0].hand.is_empty());
    assert!(r.game.players[0].trash.is_empty());
}

#[test]
fn bt20_095_main_reveals_only_available_cards_from_a_short_deck() {
    let mut r = main_runner(&["TAMER", "OPTION"]);
    use_main(&mut r);
    choices(&r, 0, 2, false);
    let action = reveal_action(&r, "TAMER");
    choose(&mut r, action);
    finish_remainder(&mut r, false, false);
    assert_eq!(ids(&r, &r.game.players[0].hand), ["TAMER"]);
    assert_eq!(ids(&r, &r.game.players[0].deck), ["OPTION"]);
    assert!(!r.game.game_over);
}

#[test]
fn bt20_095_delay_requires_own_chronicle_digimon_deletion_and_an_older_placement() {
    for (owner, victim, same_turn) in [
        (1, "CHRON", false),
        (0, "OTHER", false),
        (0, "TAMER", false),
        (0, "CHRON", true),
    ] {
        let mut r = if same_turn { placed() } else { delayed(0) };
        r.place_in_breeding(0, "CHRON");
        r.add_to_hand(0, "EVOLUTION");
        delete(&mut r, owner, victim);
        assert!(
            r.pending_selection().is_none(),
            "owner {owner}, victim {victim}, same turn {same_turn}"
        );
        assert!(parked(&r, 0));
        assert!(r.game.players[0].breeding_area.is_some());
    }
}

#[test]
fn bt20_095_delay_works_on_both_turns_from_hand_and_trash_for_free() {
    for turn in [0, 1] {
        for trash in [false, true] {
            let mut r = delayed(turn);
            let decoy = r.place_on_field(0, "CHRON", None);
            let decoy_card = r.top_card(decoy);
            let moved = r.place_in_breeding(0, "CHRON").card;
            if trash {
                r.inject_trash(0, "EVOLUTION");
            } else {
                r.add_to_hand(0, "EVOLUTION");
            }
            let deck = r.deck_size(0);
            delete(&mut r, 0, "CHRON");
            accept_delay(&mut r);
            accept_move(&mut r);
            choices(&r, 0, 1, true);
            let action = union_action(&r, 0, "EVOLUTION", trash);
            choose(&mut r, action);
            assert_eq!(top(&r, locate(&r, 0, moved)), "EVOLUTION");
            assert_eq!(top(&r, locate(&r, 0, decoy_card)), "CHRON");
            assert_eq!(r.memory(), 5);
            assert_eq!(r.deck_size(0), deck - 1);
            assert!(r.pending_selection().is_none());
        }
    }
}

#[test]
fn bt20_095_official_errata_allows_a_non_chronicle_lv3_breeder_to_move_and_evolve() {
    let mut r = delayed(1);
    let moved = r.place_in_breeding(0, "OTHER").card;
    r.add_to_hand(0, "EVOLUTION");
    delete(&mut r, 0, "CHRON");
    accept_delay(&mut r);
    accept_move(&mut r);
    choose_first(&mut r, 0, 1, true);
    assert_eq!(top(&r, locate(&r, 0, moved)), "EVOLUTION");
    assert_eq!(r.memory(), 5);
}

#[test]
fn bt20_095_movement_allows_a_level_four_non_chronicle_breeder_without_a_result() {
    let mut r = delayed(1);
    let moved = r.place_in_breeding(0, "NON-CHRON-EVO").card;
    delete(&mut r, 0, "CHRON");
    accept_delay(&mut r);
    accept_move(&mut r);
    assert_eq!(top(&r, locate(&r, 0, moved)), "NON-CHRON-EVO");
    assert!(r.pending_selection().is_none());
    assert_eq!(r.memory(), 5);
}

#[test]
fn bt20_095_delay_decline_keeps_option_for_the_next_matching_deletion() {
    let mut r = delayed(1);
    r.place_in_breeding(0, "CHRON");
    delete(&mut r, 0, "CHRON");
    choices(&r, 0, 1, true);
    choose(&mut r, space::PASS);
    assert!(parked(&r, 0));
    delete(&mut r, 0, "CHRON");
    accept_delay(&mut r);
    accept_move(&mut r);
    assert!(r.pending_selection().is_none());
}

#[test]
fn bt20_095_move_and_evolution_have_independent_declines_after_delay_payment() {
    for decline_evolution in [false, true] {
        let mut r = delayed(0);
        let moved = r.place_in_breeding(0, "CHRON").card;
        r.add_to_hand(0, "EVOLUTION");
        delete(&mut r, 0, "CHRON");
        accept_delay(&mut r);
        if decline_evolution {
            accept_move(&mut r);
        }
        choices(&r, 0, 1, true);
        choose(&mut r, space::PASS);
        assert!(has(&r, 0, ID, true));
        assert!(has(&r, 0, "EVOLUTION", false));
        if decline_evolution {
            assert_eq!(top(&r, locate(&r, 0, moved)), "CHRON");
        } else {
            assert!(r.game.players[0].breeding_area.is_some());
        }
        assert_eq!(r.memory(), 5);
        assert!(r.pending_selection().is_none());
    }
}

#[test]
fn bt20_095_movement_can_be_paid_without_a_legal_evolution_result() {
    let mut r = delayed(0);
    let moved = r.place_in_breeding(0, "CHRON").card;
    r.add_to_hand(0, "WRONG-LEVEL");
    delete(&mut r, 0, "CHRON");
    accept_delay(&mut r);
    accept_move(&mut r);
    assert!(r.pending_selection().is_none());
    assert_eq!(top(&r, locate(&r, 0, moved)), "CHRON");
    assert!(has(&r, 0, "WRONG-LEVEL", false));
}

#[test]
fn bt20_095_move_cost_does_not_offer_empty_egg_or_cannot_move_breeding() {
    for mode in ["empty", "egg", "cannot_move"] {
        let mut r = delayed(1);
        if mode != "empty" {
            r.place_in_breeding(0, if mode == "egg" { "EGG" } else { "CHRON" });
        }
        if mode == "cannot_move" {
            r.game.modifiers.add(
                PermanentHandle {
                    player: 0,
                    index: space::BREEDING_TARGET as u8,
                },
                ModifierEntry::simple(ModifierType::CannotMove, 0, Expiry::Permanent, 1),
            );
        }
        r.add_to_hand(0, "EVOLUTION");
        delete(&mut r, 0, "CHRON");
        accept_delay(&mut r);
        assert!(r.pending_selection().is_none(), "unpayable {mode}");
        assert!(has(&r, 0, "EVOLUTION", false));
        assert!(has(&r, 0, ID, true));
    }
}

#[test]
fn bt20_095_evolution_mask_keeps_printed_routes_and_chronicle_digimon_restriction() {
    let mut r = delayed(1);
    r.place_in_breeding(0, "CHRON");
    let hand = [
        "EVOLUTION",
        "NON-CHRON-EVO",
        "WRONG-LEVEL",
        "WRONG-COLOR",
        "OPTION",
        "TAMER",
    ];
    for id in hand {
        r.add_to_hand(0, id);
    }
    r.inject_trash(0, "EVOLUTION");
    delete(&mut r, 0, "CHRON");
    accept_delay(&mut r);
    accept_move(&mut r);
    choices(&r, 0, 2, true);
    for id in hand {
        let action = union_action(&r, 0, id, false);
        assert_eq!(
            build_action_mask(&r.game, 0)[action as usize],
            if id == "EVOLUTION" { 1.0 } else { 0.0 },
            "{id}"
        );
    }
    choose(&mut r, space::PASS);
}

#[test]
fn bt20_095_evolution_prohibition_does_not_prevent_paying_the_movement_cost() {
    let mut r = delayed(1);
    let moved = r.place_in_breeding(0, "CHRON").card;
    r.add_to_hand(0, "EVOLUTION");
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
    delete(&mut r, 0, "CHRON");
    accept_delay(&mut r);
    accept_move(&mut r);
    assert!(r.pending_selection().is_none());
    assert_eq!(top(&r, locate(&r, 0, moved)), "CHRON");
    assert!(has(&r, 0, "EVOLUTION", false));
}

#[test]
fn bt20_095_unpaid_move_after_capacity_changes_never_offers_evolution() {
    let mut r = delayed(0);
    let moved = r.place_in_breeding(0, "CHRON").card;
    r.add_to_hand(0, "EVOLUTION");
    delete(&mut r, 0, "CHRON");
    accept_delay(&mut r);
    let action = choices(&r, 0, 1, true)[0];
    while r.game.players[0].battle_area.len() < r.game.rules.field_slots as usize {
        r.place_on_field(0, "OTHER", None);
    }
    choose(&mut r, action);
    assert_eq!(
        r.game.players[0]
            .breeding_area
            .as_ref()
            .unwrap()
            .top_card()
            .handle(),
        moved
    );
    assert!(has(&r, 0, "EVOLUTION", false));
    assert!(has(&r, 0, ID, true));
    assert!(r.pending_selection().is_none());
}

#[test]
fn bt20_095_move_result_survives_compaction_and_clone_at_the_evolution_prompt() {
    let mut r = delayed(1);
    let moved = r.place_in_breeding(0, "CHRON").card;
    r.add_to_hand(0, "EVOLUTION");
    delete(&mut r, 0, "CHRON");
    accept_delay(&mut r);
    accept_move(&mut r);
    let action = choices(&r, 0, 1, true)[0];
    let front = r.game.players[0].battle_area.remove(0);
    r.game.players[0].trash.extend(front.card_sources);
    let parked_state = r.game.clone();
    for clone in [false, true] {
        if clone {
            r.game = parked_state.clone();
        }
        choose(&mut r, action);
        assert_eq!(top(&r, locate(&r, 0, moved)), "EVOLUTION");
        assert!(r.pending_selection().is_none());
    }
}

fn security_runner(hand: &[&str], trash: &[&str]) -> DebugRunner {
    let mut r = ready(builder().hand(1, hand).security(1, &["FILL", ID]).start());
    for id in trash {
        r.inject_trash(1, id);
    }
    r
}

fn attack_security(r: &mut DebugRunner) {
    let attacker = r.place_on_field(0, "FILL", Some(0));
    let action = space::encode_attack(attacker.index as u16, 14);
    assert_eq!(build_action_mask(&r.game, 0)[action as usize], 1.0);
    r.game.decode_action(action, 0);
}

#[test]
fn bt20_095_security_plays_cost_five_chronicle_digimon_or_tamer_from_hand_or_trash() {
    for id in ["CHRON5", "TAMER"] {
        for trash in [false, true] {
            let mut r = if trash {
                security_runner(&[], &[id])
            } else {
                security_runner(&[id], &[])
            };
            let memory = r.memory();
            attack_security(&mut r);
            choices(&r, 1, 1, true);
            let action = union_action(&r, 1, id, trash);
            choose(&mut r, action);
            r.auto_resolve().unwrap();
            let played = find(&r, 1, id);
            assert!(!r.game.players[1].battle_area[played.index as usize].is_suspended);
            assert_eq!(r.memory(), memory);
            assert!(parked(&r, 1));
            assert!(!has(&r, 1, ID, true));
        }
    }
}

#[test]
fn bt20_095_security_union_mask_excludes_wrong_trait_cost_options_and_eggs() {
    let hand = ["CHRON5", "TAMER", "CHRON6", "OTHER", "OPTION", "EGG"];
    let mut r = security_runner(&hand, &["CHRON5", "TAMER", "CHRON6", "OTHER"]);
    attack_security(&mut r);
    choices(&r, 1, 4, true);
    for id in hand {
        let action = union_action(&r, 1, id, false);
        assert_eq!(
            build_action_mask(&r.game, 1)[action as usize],
            if ["CHRON5", "TAMER"].contains(&id) {
                1.0
            } else {
                0.0
            },
            "{id}"
        );
    }
    choose(&mut r, space::PASS);
    r.auto_resolve().unwrap();
    assert!(parked(&r, 1));
}

#[test]
fn bt20_095_security_decline_or_no_candidate_still_places_delay() {
    for has_candidate in [false, true] {
        let mut r = security_runner(
            if has_candidate {
                &["CHRON5"]
            } else {
                &["OTHER"]
            },
            &[],
        );
        attack_security(&mut r);
        if has_candidate {
            choices(&r, 1, 1, true);
            choose(&mut r, space::PASS);
        }
        r.auto_resolve().unwrap();
        assert!(parked(&r, 1));
        assert!(!has(&r, 1, ID, true));
        assert!(has(
            &r,
            1,
            if has_candidate { "CHRON5" } else { "OTHER" },
            false
        ));
    }
}

#[test]
fn bt20_095_security_masks_effect_play_prohibitions_without_suppressing_delay_placement() {
    for (blocked, modifier) in [
        ("CHRON5", ModifierType::CannotPlayDigimonByEffect),
        ("TAMER", ModifierType::CannotPlayTamerByEffect),
    ] {
        let mut r = security_runner(&[blocked], &[]);
        r.game.modifiers.add_player_modifier(
            1,
            PlayerModifierEntry::simple(modifier, 0, Expiry::Permanent, None, 0),
        );
        attack_security(&mut r);
        assert!(
            r.pending_selection().is_none(),
            "unplayable {blocked} is not offered"
        );
        r.auto_resolve().unwrap();
        assert!(parked(&r, 1));
        assert!(has(&r, 1, blocked, false));
    }
}
