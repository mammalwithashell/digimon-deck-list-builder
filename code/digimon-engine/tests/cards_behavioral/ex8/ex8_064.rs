//! EX8-064 Boltboutamon.
//!
//! Covers the new aggregate visible-zone play-cost budget selector used by
//! the DNA digivolve branch, plus the All Turns deletion observer.

use digimon_engine::action::space::TRASH_EFFECT_START;
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::{CardHandle, CardSource};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::TriggerSource;

fn digimon(id: &str, name: &str, play_cost: u16, traits: &[&str]) -> CardData {
    let mut card = make_test_card(id, name);
    card.card_kind = CardKind::Digimon;
    card.level = Some(4);
    card.dp = Some(4000);
    card.play_cost = play_cost;
    card.traits = traits
        .iter()
        .map(|trait_name| trait_name.to_string())
        .collect();
    card.colors = vec![CardColor::Purple];
    card
}

fn boltboutamon_runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("EX8-064")
        .expect("EX8-064 in embedded DSL pack")
        .add_card(digimon("NSO-6", "NSoCost6", 6, &["NSo"]))
        .add_card(digimon("NSO-4", "NSoCost4", 4, &["NSo"]))
        .add_card(digimon("NSO-11", "NSoCost11", 11, &["NSo"]))
        .add_card(digimon("NON-NSO", "NonNSo", 3, &["Wizard"]))
        .add_card(digimon("OPP-DIGI", "OpponentDigimon", 3, &[]))
        .add_card(make_test_card("SEC", "SecurityCard"))
        .memory(5)
        .start()
}

fn push_to_trash(runner: &mut DebugRunner, player: u8, card_id: &str) -> CardHandle {
    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|card| card.card_id == card_id)
        .unwrap_or_else(|| panic!("{card_id} missing from card data"));
    let next = runner.game.next_card_index();
    let card = CardSource::new(data_idx, player, next);
    let handle = card.handle();
    runner.game.players[player as usize].trash.push(card);
    handle
}

fn push_to_security(runner: &mut DebugRunner, player: u8, card_id: &str) -> CardHandle {
    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|card| card.card_id == card_id)
        .unwrap_or_else(|| panic!("{card_id} missing from card data"));
    let next = runner.game.next_card_index();
    let card = CardSource::new(data_idx, player, next);
    let handle = card.handle();
    runner.game.players[player as usize].security.push(card);
    handle
}

fn field_ids(runner: &DebugRunner, player: u8) -> Vec<String> {
    runner.game.players[player as usize]
        .battle_area
        .iter()
        .map(|perm| perm.top_card().card_id(&runner.game.card_data).to_string())
        .collect()
}

fn trash_ids(runner: &DebugRunner, player: u8) -> Vec<String> {
    runner.game.players[player as usize]
        .trash
        .iter()
        .map(|card| card.card_id(&runner.game.card_data).to_string())
        .collect()
}

fn fire_on_dna_digivolve(runner: &mut DebugRunner, handle: PermanentHandle) {
    let card = runner.game.players[handle.player as usize].battle_area[handle.index as usize]
        .top_card()
        .handle();
    runner.game.enqueue_triggered(
        EffectTiming::OnDnaDigivolve,
        TriggerSource::Digivolved {
            player: handle.player,
            permanent: handle,
            card,
            effect_initiated: false,
            dna_origin: true,
        },
    );
    runner.game.drain_effect_queue();
}

#[test]
fn ex8_064_dna_branch_plays_nso_trash_cards_within_total_play_cost_budget() {
    let mut runner = boltboutamon_runner();
    let bolt = runner.place_on_field(0, "EX8-064", Some(0));
    push_to_trash(&mut runner, 0, "NSO-6");
    push_to_trash(&mut runner, 0, "NSO-4");
    push_to_trash(&mut runner, 0, "NSO-11");
    push_to_trash(&mut runner, 0, "NON-NSO");

    fire_on_dna_digivolve(&mut runner, bolt);

    let view = runner
        .pending_selection()
        .expect("DNA branch should offer trash budget selection");
    assert!(
        view.valid_action_ids.contains(&TRASH_EFFECT_START),
        "cost-6 NSo card starts eligible"
    );
    assert!(
        view.valid_action_ids.contains(&(TRASH_EFFECT_START + 1)),
        "cost-4 NSo card starts eligible"
    );
    assert!(
        !view.valid_action_ids.contains(&(TRASH_EFFECT_START + 2)),
        "cost-11 NSo card exceeds total budget"
    );
    assert!(
        !view.valid_action_ids.contains(&(TRASH_EFFECT_START + 3)),
        "non-NSo Digimon is filtered out"
    );

    runner
        .game
        .resolve_selection(0, TRASH_EFFECT_START)
        .expect("pick cost-6 NSo");
    let view = runner
        .pending_selection()
        .expect("selection should continue with remaining budget 4");
    assert!(
        view.valid_action_ids.contains(&(TRASH_EFFECT_START + 1)),
        "cost-4 NSo still fits remaining budget"
    );
    assert!(
        !view.valid_action_ids.contains(&(TRASH_EFFECT_START + 2)),
        "cost-11 remains masked"
    );

    runner
        .game
        .resolve_selection(0, TRASH_EFFECT_START + 1)
        .expect("pick cost-4 NSo");
    runner.auto_resolve().expect("free plays resolve");

    let played = field_ids(&runner, 0);
    assert!(played.contains(&"NSO-6".to_string()));
    assert!(played.contains(&"NSO-4".to_string()));
    assert_eq!(
        trash_ids(&runner, 0),
        vec!["NSO-11".to_string(), "NON-NSO".to_string()],
        "unselected cards remain in trash"
    );
}

#[test]
fn ex8_064_other_digimon_deletion_trashes_opponent_top_security_once_per_turn() {
    let mut runner = boltboutamon_runner();
    let _bolt = runner.place_on_field(0, "EX8-064", Some(0));
    let opp_a = runner.place_on_field(1, "OPP-DIGI", Some(0));
    let opp_b = runner.place_on_field(1, "NON-NSO", Some(0));
    let bottom_security = push_to_security(&mut runner, 1, "SEC");
    let top_security = push_to_security(&mut runner, 1, "SEC");

    runner.game.delete_permanent_with_effects(opp_a);
    runner.game.drain_effect_queue();
    runner.auto_resolve().expect("first deletion resolves");

    assert_eq!(runner.security_count(1), 1);
    assert!(
        runner
            .game
            .player(1)
            .trash
            .iter()
            .any(|card| card.handle() == top_security),
        "top security card should be trashed"
    );

    runner.game.delete_permanent_with_effects(opp_b);
    runner.game.drain_effect_queue();
    runner.auto_resolve().expect("second deletion resolves");

    assert_eq!(
        runner.security_count(1),
        1,
        "once-per-turn observer must not trash a second security card"
    );
    assert!(
        runner
            .game
            .player(1)
            .security
            .iter()
            .any(|card| card.handle() == bottom_security),
        "bottom security card should remain"
    );
}
