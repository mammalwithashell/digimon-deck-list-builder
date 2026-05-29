use digimon_dsl::compiled::{CompiledAltPathKind, CompiledClause, CompiledCost};
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, make_test_card_with_level, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, Keyword, PlaySource};
use digimon_engine::replacement::ReplacementCause;
use digimon_engine::selection::SelectionKind;

const CARD_ID: &str = "BT21-011";

fn lv4_target(card_id: &str, traits: &[&str]) -> CardData {
    let mut card = make_test_card_with_level(card_id, card_id, 4);
    card.colors = vec![CardColor::Red];
    card.traits = traits
        .iter()
        .map(|trait_name| (*trait_name).to_string())
        .collect();
    card.evo_costs = vec![EvoCost {
        card_color: 0,
        level: 3,
        memory_cost: 1,
    }];
    card
}

fn xros_carrier(card_id: &str) -> CardData {
    let mut card = make_test_card_with_level(card_id, card_id, 4);
    card.colors = vec![CardColor::Red];
    card.traits = vec!["Xros Heart".to_string()];
    card
}

fn plain_carrier(card_id: &str) -> CardData {
    let mut card = make_test_card_with_level(card_id, card_id, 4);
    card.colors = vec![CardColor::Red];
    card.traits = vec!["Plain".to_string()];
    card
}

fn plain_source(card_id: &str) -> CardData {
    let mut card = make_test_card_with_level(card_id, card_id, 3);
    card.colors = vec![CardColor::Red];
    card.traits = vec!["Plain".to_string()];
    card
}

fn tamer(card_id: &str) -> CardData {
    let mut card = make_test_card(card_id, card_id);
    card.card_kind = CardKind::Tamer;
    card.level = None;
    card.dp = None;
    card.play_cost = 4;
    card
}

fn push_to_hand(runner: &mut DebugRunner, player: u8, card_id: &str) -> usize {
    let data_index = runner
        .game
        .card_data
        .iter()
        .position(|card| card.card_id == card_id)
        .unwrap_or_else(|| panic!("unknown card id {card_id}"));
    let instance_id = runner.game.next_card_index();
    runner.game.players[player as usize]
        .hand
        .push(CardSource::new(data_index, player, instance_id));
    runner.game.players[player as usize].hand.len() - 1
}

fn runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT21-011 YAML loads")
        .add_card(lv4_target("XROS-LV4", &["Xros Heart"]))
        .add_card(lv4_target("HERO-LV4", &["Hero"]))
        .add_card(lv4_target("PLAIN-LV4", &["Plain"]))
        .add_card(xros_carrier("XROS-CARRIER"))
        .add_card(plain_carrier("PLAIN-CARRIER"))
        .add_card(plain_source("PLAIN-SOURCE"))
        .add_card(tamer("XROS-TAMER"))
        .memory(5)
        .start()
}

#[test]
fn bt21_011_declares_alt_paths_and_save_keyword() {
    let runner = runner();
    let card = runner
        .compiled_card(CARD_ID)
        .expect("BT21-011 compiled card present");

    assert!(
        card.alt_paths.iter().any(|path| {
            path.kind == CompiledAltPathKind::Digivolve
                && path.cost == Some(CompiledCost::Literal(0))
                && path.from.as_ref().is_some_and(|filter| {
                    filter.level_eq == Some(2)
                        && filter.any_of.iter().any(|p| {
                            p.trait_has.as_deref() == Some("Xros Heart")
                                || p.trait_has.as_deref() == Some("Hero")
                        })
                })
        }),
        "Shoutmon should digivolve from Lv.2 Xros Heart/Hero for 0"
    );
    assert!(
        card.effects.iter().any(|clause| matches!(
            clause,
            CompiledClause::Declarative(d)
                if format!("{d:?}").contains("Save")
        )),
        "BT21-011 must declare <Save>"
    );
}

#[test]
fn bt21_011_reduces_cost_when_this_digivolves_into_xros_heart_or_hero() {
    for (target_id, trait_label) in [("XROS-LV4", "Xros Heart"), ("HERO-LV4", "Hero")] {
        let mut runner = runner();
        runner.game.turn_count = 1;
        let shoutmon = runner.place_on_field(0, CARD_ID, Some(0));
        let hand_index = push_to_hand(&mut runner, 0, target_id);

        let memory_before = runner.game.memory;
        let digivolved = runner.game.digivolve_from_hand(
            0,
            hand_index,
            shoutmon.index as usize,
            PlaySource::ByHand,
        );

        assert!(digivolved, "BT21-011 should digivolve into {trait_label}");
        assert_eq!(
            runner.game.memory, memory_before,
            "base cost 1 should be reduced to 0 when digivolving into {trait_label}"
        );
    }
}

#[test]
fn bt21_011_cost_reduction_does_not_fire_for_unmatched_target_or_other_source() {
    let mut unmatched_runner = runner();
    unmatched_runner.game.turn_count = 1;
    let shoutmon = unmatched_runner.place_on_field(0, CARD_ID, Some(0));
    let plain_target = push_to_hand(&mut unmatched_runner, 0, "PLAIN-LV4");

    let memory_before = unmatched_runner.game.memory;
    let digivolved = unmatched_runner.game.digivolve_from_hand(
        0,
        plain_target,
        shoutmon.index as usize,
        PlaySource::ByHand,
    );
    assert!(
        digivolved,
        "plain target should still be a legal digivolution"
    );
    assert_eq!(
        unmatched_runner.game.memory,
        memory_before - 1,
        "non-Xros/Hero target should pay the full cost"
    );

    let mut other_source_runner = runner();
    other_source_runner.game.turn_count = 1;
    let _shoutmon_bystander = other_source_runner.place_on_field(0, CARD_ID, Some(0));
    let other_source = other_source_runner.place_on_field(0, "PLAIN-SOURCE", Some(0));
    let xros_target = push_to_hand(&mut other_source_runner, 0, "XROS-LV4");

    let memory_before = other_source_runner.game.memory;
    let digivolved = other_source_runner.game.digivolve_from_hand(
        0,
        xros_target,
        other_source.index as usize,
        PlaySource::ByHand,
    );
    assert!(
        digivolved,
        "other Lv.3 source should be able to digivolve into the Xros target"
    );
    assert_eq!(
        other_source_runner.game.memory,
        memory_before - 1,
        "BT21-011 should reduce only when it is the digivolving source"
    );
}

#[test]
fn bt21_011_save_and_inherited_rush_work_for_xros_heart_carrier() {
    let mut runner = runner();
    let carrier = runner.place_stack(0, &[CARD_ID, "XROS-CARRIER"]);
    assert!(
        runner.game.has_keyword(carrier, Keyword::Rush),
        "BT21-011 inherited text should grant Rush to a Xros Heart carrier"
    );

    let plain = runner.place_stack(0, &[CARD_ID, "PLAIN-CARRIER"]);
    assert!(
        !runner.game.has_keyword(plain, Keyword::Rush),
        "BT21-011 inherited text should not grant Rush to non-Xros Heart carriers"
    );

    let shoutmon = runner.place_on_field(0, CARD_ID, Some(0));
    runner.place_on_field(0, "XROS-TAMER", Some(0));
    runner
        .game
        .delete_permanent_with_cause(shoutmon, ReplacementCause::OpponentEffect);

    let pending = runner
        .pending_selection_view()
        .expect("Save should install a Tamer destination prompt");
    assert_eq!(pending.kind, SelectionKind::OwnField);
}
