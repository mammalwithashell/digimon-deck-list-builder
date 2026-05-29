//! P-187 Mastemon

use digimon_engine::action::space::{encode_attack, PASS, TRASH_EFFECT_START};
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, make_test_card_with_level, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::{SelectionKind, TriggerSource};

const YAML: &str = include_str!("../../../cards/p/P-187.yaml");

#[test]
fn p_187_dna_digivolve_recovers_places_top_security_and_trashes_opponent_security() {
    let mut tamer = make_test_card("HELPER-TAMER", "Helper Tamer");
    tamer.card_kind = CardKind::Tamer;
    tamer.level = None;
    tamer.dp = None;
    tamer.colors = vec![CardColor::Yellow];

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("P-187 YAML loads")
        .add_card(tamer)
        .add_card(make_test_card("RECOVER", "Recover"))
        .add_card(make_test_card("OPP-A", "Opp A"))
        .add_card(make_test_card("OPP-B", "Opp B"))
        .deck(0, &["RECOVER"])
        .security(1, &["OPP-A", "OPP-B"])
        .memory(0)
        .start();
    let mastemon = runner.place_on_field(0, "P-187", Some(0));
    let target = runner.place_on_field(0, "HELPER-TAMER", Some(0));

    fire_when_digivolving(&mut runner, mastemon, true);

    assert_eq!(runner.pending_kind(), Some(SelectionKind::TriggerOrder));
    runner
        .execute_branch(0)
        .expect("resolve mandatory recovery/placement trigger first");
    assert_eq!(runner.security_count(0), 1, "Recovery +1 resolves first");
    let target_prompt = runner
        .pending_selection_view()
        .expect("DNA placement target prompt");
    assert!(target_prompt
        .valid_action_ids
        .contains(&encode_attack(target.player as u16, target.index as u16)));
    runner
        .execute_action(
            target_prompt.selecting_player,
            encode_attack(target.player as u16, target.index as u16),
        )
        .expect("choose security cost target");

    let branch = runner
        .pending_selection_view()
        .expect("top/bottom choice prompt");
    assert_eq!(branch.kind, SelectionKind::EffectChoice);
    runner.execute_branch(0).expect("choose top placement");

    assert_eq!(
        runner.game.players[0]
            .security
            .last()
            .expect("top security")
            .card_id(&runner.game.card_data),
        "HELPER-TAMER"
    );
    assert_eq!(
        runner.security_count(1),
        1,
        "opponent top security is trashed only after placement succeeds"
    );
}

#[test]
fn p_187_security_trash_cost_gates_hand_trash_play_branch() {
    let mut small = make_test_card_with_level("SMALL-YELLOW", "Small Yellow", 4);
    small.colors = vec![CardColor::Yellow];
    small.dp = Some(6000);

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("P-187 YAML loads")
        .add_card(small)
        .add_card(make_test_card("SECURITY", "Security"))
        .security(0, &["SECURITY"])
        .memory(0)
        .start();
    push_to_trash(&mut runner, 0, "SMALL-YELLOW");
    let mastemon = runner.place_on_field(0, "P-187", Some(0));

    runner.game.enqueue_triggered(
        EffectTiming::WhenAttacking,
        TriggerSource::Permanent(mastemon),
    );
    runner.game.drain_effect_queue();

    assert!(
        runner.pending_is_optional(),
        "the security-trash-cost branch must be declinable"
    );
    choose_first_non_pass(&mut runner);

    assert_eq!(
        runner.security_count(0),
        0,
        "accepting pays the top-security trash cost before the play prompt"
    );
    let pick = runner
        .pending_selection_view()
        .expect("small Digimon hand/trash prompt");
    assert_eq!(pick.valid_action_ids, vec![TRASH_EFFECT_START]);
    runner
        .execute_action(pick.selecting_player, TRASH_EFFECT_START)
        .expect("play small Digimon from trash");

    assert!(battle_area_contains(&runner, 0, "SMALL-YELLOW"));
    assert!(!trash_ids(&runner, 0).contains(&"SMALL-YELLOW".to_string()));
}

fn fire_when_digivolving(runner: &mut DebugRunner, handle: PermanentHandle, dna_origin: bool) {
    let card = runner.game.players[handle.player as usize].battle_area[handle.index as usize]
        .top_card()
        .handle();
    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Digivolved {
            player: handle.player,
            permanent: handle,
            card,
            effect_initiated: false,
            dna_origin,
        },
    );
    runner.game.drain_effect_queue();
}

fn choose_first_non_pass(runner: &mut DebugRunner) {
    let view = runner.pending_selection_view().expect("pending selection");
    let action = *view
        .valid_action_ids
        .iter()
        .find(|&&action| action != PASS)
        .expect("non-PASS action");
    runner
        .execute_action(view.selecting_player, action)
        .expect("choose first non-PASS action");
}

fn push_to_trash(runner: &mut DebugRunner, player: u8, card_id: &str) {
    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .expect("card exists");
    let card_index = runner.game.next_card_index();
    runner.game.players[player as usize]
        .trash
        .push(CardSource::new(data_idx, player, card_index));
}

fn trash_ids(runner: &DebugRunner, player: u8) -> Vec<String> {
    zone_ids(
        &runner.game.players[player as usize].trash,
        &runner.game.card_data,
    )
}

fn zone_ids(cards: &[CardSource], data: &[CardData]) -> Vec<String> {
    cards
        .iter()
        .map(|card| card.card_id(data).to_string())
        .collect()
}

fn battle_area_contains(runner: &DebugRunner, player: u8, card_id: &str) -> bool {
    runner.game.players[player as usize]
        .battle_area
        .iter()
        .any(|permanent| permanent.top_card().card_id(&runner.game.card_data) == card_id)
}
