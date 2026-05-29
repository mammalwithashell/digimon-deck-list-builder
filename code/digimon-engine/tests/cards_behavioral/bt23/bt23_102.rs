//! BT23-102 Mastemon

use digimon_engine::action::space::{encode_attack, PASS, TRASH_EFFECT_START};
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::{CardHandle, CardSource};
use digimon_engine::debug_runner::{make_test_card, make_test_card_with_level, DebugRunner};
use digimon_engine::enums::{CardColor, EffectTiming, Keyword};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::TriggerSource;
use digimon_engine::trigger_context::EventCause;

const YAML: &str = include_str!("../../../cards/bt23/BT23-102.yaml");

#[test]
fn bt23_102_grants_barrier_and_partition() {
    let runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("BT23-102 YAML loads")
        .start();
    let compiled = runner.compiled_card("BT23-102").expect("compiled card");

    assert!(compiled.effects.iter().any(|clause| matches!(
        clause,
        digimon_dsl::compiled::CompiledClause::Declarative(
            digimon_dsl::compiled::CompiledDeclarativeClause::GrantKeyword { keyword, .. }
        ) if keyword == "Barrier"
    )));
    assert!(compiled.effects.iter().any(|clause| matches!(
        clause,
        digimon_dsl::compiled::CompiledClause::Declarative(
            digimon_dsl::compiled::CompiledDeclarativeClause::Partition { .. }
        )
    )));

    let mastemon = runner
        .game
        .card_data
        .iter()
        .find(|card| card.card_id == "BT23-102")
        .expect("card data registered");
    assert!(mastemon.keywords.contains(&Keyword::Barrier));
}

#[test]
fn bt23_102_when_digivolving_plays_from_trash_and_trashes_both_security_to_three() {
    let mut small = make_test_card_with_level("TRASH-YELLOW", "Trash Yellow", 5);
    small.colors = vec![CardColor::Yellow];
    small.dp = Some(6000);

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("BT23-102 YAML loads")
        .add_card(small)
        .add_card(make_test_card("LV5-A", "Lv5 A"))
        .add_card(make_test_card("LV5-B", "Lv5 B"))
        .add_card(make_test_card("MY1", "My 1"))
        .add_card(make_test_card("MY2", "My 2"))
        .add_card(make_test_card("MY3", "My 3"))
        .add_card(make_test_card("MY4", "My 4"))
        .add_card(make_test_card("MY5", "My 5"))
        .add_card(make_test_card("OPP1", "Opp 1"))
        .add_card(make_test_card("OPP2", "Opp 2"))
        .add_card(make_test_card("OPP3", "Opp 3"))
        .add_card(make_test_card("OPP4", "Opp 4"))
        .add_card(make_test_card("OPP5", "Opp 5"))
        .security(0, &["MY1", "MY2", "MY3", "MY4", "MY5"])
        .security(1, &["OPP1", "OPP2", "OPP3", "OPP4", "OPP5"])
        .memory(0)
        .start();
    push_to_trash(&mut runner, 0, "TRASH-YELLOW");
    let mastemon = runner.place_stack(0, &["LV5-A", "LV5-B", "BT23-102"]);

    fire_when_digivolving(&mut runner, mastemon, false);

    let pick = runner
        .pending_selection_view()
        .expect("Lv.5 hand/trash play prompt");
    assert_eq!(pick.valid_action_ids, vec![TRASH_EFFECT_START]);
    runner
        .execute_action(pick.selecting_player, TRASH_EFFECT_START)
        .expect("play Lv.5 from trash");

    if runner.pending_is_optional() {
        runner
            .execute_action(0, PASS)
            .expect("decline all-turns security-loss trigger");
    }

    assert!(battle_area_contains(&runner, 0, "TRASH-YELLOW"));
    assert_eq!(runner.security_count(0), 3);
    assert_eq!(runner.security_count(1), 3);
}

#[test]
fn bt23_102_when_digivolving_without_same_level_pair_does_not_trash_security() {
    let mut small = make_test_card_with_level("TRASH-PURPLE", "Trash Purple", 5);
    small.colors = vec![CardColor::Purple];
    small.dp = Some(6000);
    let mut lv4 = make_test_card_with_level("LV4", "Lv4", 4);
    lv4.colors = vec![CardColor::Yellow];

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("BT23-102 YAML loads")
        .add_card(small)
        .add_card(lv4)
        .add_card(make_test_card("LV5", "Lv5"))
        .add_card(make_test_card("MY1", "My 1"))
        .add_card(make_test_card("MY2", "My 2"))
        .add_card(make_test_card("MY3", "My 3"))
        .add_card(make_test_card("MY4", "My 4"))
        .add_card(make_test_card("OPP1", "Opp 1"))
        .add_card(make_test_card("OPP2", "Opp 2"))
        .add_card(make_test_card("OPP3", "Opp 3"))
        .add_card(make_test_card("OPP4", "Opp 4"))
        .security(0, &["MY1", "MY2", "MY3", "MY4"])
        .security(1, &["OPP1", "OPP2", "OPP3", "OPP4"])
        .memory(0)
        .start();
    push_to_trash(&mut runner, 0, "TRASH-PURPLE");
    let mastemon = runner.place_stack(0, &["LV4", "LV5", "BT23-102"]);

    fire_when_digivolving(&mut runner, mastemon, false);
    let pick = runner
        .pending_selection_view()
        .expect("Lv.5 hand/trash play prompt");
    runner
        .execute_action(pick.selecting_player, TRASH_EFFECT_START)
        .expect("play Lv.5 from trash");

    assert_eq!(runner.security_count(0), 4);
    assert_eq!(runner.security_count(1), 4);
}

#[test]
fn bt23_102_security_removed_trigger_places_selected_digimon_bottom_security() {
    let mut other = make_test_card("OTHER-DIGIMON", "Other Digimon");
    other.colors = vec![CardColor::Purple];

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("BT23-102 YAML loads")
        .add_card(other)
        .memory(0)
        .start();
    let _mastemon = runner.place_on_field(0, "BT23-102", Some(0));
    let target = runner.place_on_field(0, "OTHER-DIGIMON", Some(0));

    runner.game.enqueue_triggered(
        EffectTiming::OnOpponentSecurityRemoved,
        TriggerSource::SecurityRemoved {
            affected_player: 1,
            observer_player: 0,
            source_player: 0,
            card: CardHandle(0),
            cause: EventCause::SecurityRemoval,
        },
    );
    runner.game.drain_effect_queue();

    let pick = runner
        .pending_selection_view()
        .expect("security-loss bottom-security prompt");
    assert!(pick
        .valid_action_ids
        .contains(&encode_attack(target.player as u16, target.index as u16)));
    runner
        .execute_action(
            pick.selecting_player,
            encode_attack(target.player as u16, target.index as u16),
        )
        .expect("place selected Digimon");

    assert_eq!(runner.security_count(0), 1);
    assert_eq!(
        runner.game.players[0].security[0].card_id(&runner.game.card_data),
        "OTHER-DIGIMON"
    );
    assert!(battle_area_contains(&runner, 0, "BT23-102"));
    assert!(!battle_area_contains(&runner, 0, "OTHER-DIGIMON"));
    assert_eq!(runner.battle_area_size(0), 1, "Mastemon remains on field");
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

fn battle_area_contains(runner: &DebugRunner, player: u8, card_id: &str) -> bool {
    runner.game.players[player as usize]
        .battle_area
        .iter()
        .any(|permanent| permanent.top_card().card_id(&runner.game.card_data) == card_id)
}

#[allow(dead_code)]
fn zone_ids(cards: &[CardSource], data: &[CardData]) -> Vec<String> {
    cards
        .iter()
        .map(|card| card.card_id(data).to_string())
        .collect()
}
