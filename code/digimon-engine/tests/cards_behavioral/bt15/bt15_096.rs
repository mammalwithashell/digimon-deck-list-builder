//! BT15-096 Supreme Connection! - Option, Black, Cost 3.
//!
//! Requested printed text for this batch:
//! - [Main] Reveal top 3 cards of your deck. Add 1 card with [Machine] or
//!   [Cyborg] trait and trash 1 card with [Machine] or [Cyborg] trait among
//!   them. Return the rest to the top of the deck. Then, place this card in
//!   the battle area.
//! - [Delay] 1 of your Digimon with [Machine] or [Cyborg] trait may play 1
//!   Digimon card with play cost <= its play cost from your hand with the play
//!   cost reduced by 3.
//! - Security: Place this card in battle area.

use std::fs;
use std::path::PathBuf;

use digimon_dsl::compiled::{
    CompiledCardKind, CompiledClause, CompiledCostDelta, CompiledDeclarativeClause, CompiledStep,
    CompiledTiming,
};
use digimon_engine::action::space::PASS;
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::combat::AttackResult;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, DelayTrigger};
use digimon_engine::permanent::OptionState;

fn yaml() -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("cards/bt15/BT15-096.yaml");
    fs::read_to_string(&path)
        .unwrap_or_else(|err| panic!("BT15-096 YAML should exist at {}: {err}", path.display()))
}

fn option_card(id: &str) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Option;
    card.colors = vec![CardColor::Black];
    card.play_cost = 3;
    card
}

fn digimon(id: &str, traits: &[&str], play_cost: u16) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.colors = vec![CardColor::Black];
    card.traits = traits.iter().map(|s| (*s).to_string()).collect();
    card.play_cost = play_cost;
    card
}

fn tamer(id: &str, traits: &[&str], play_cost: u16) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Tamer;
    card.colors = vec![CardColor::Black];
    card.traits = traits.iter().map(|s| (*s).to_string()).collect();
    card.play_cost = play_cost;
    card
}

fn runner_builder() -> digimon_engine::debug_runner::DebugRunnerBuilder {
    DebugRunner::builder()
        .from_dsl_yaml(&yaml())
        .expect("BT15-096 YAML must parse and compile")
        .add_card(option_card("BT15-096"))
        .add_card(digimon("MACHINE-HAND", &["Machine"], 7))
        .add_card(digimon("CYBORG-HAND", &["Cyborg"], 8))
        .add_card(digimon("CHEAP-MACHINE", &["Machine"], 5))
        .add_card(digimon("EXPENSIVE-CYBORG", &["Cyborg"], 9))
        .add_card(digimon("PLAIN-DIGIMON", &[], 4))
        .add_card(tamer("MACHINE-TAMER", &["Machine"], 3))
        .add_card(digimon("SOURCE-MACHINE", &["Machine"], 6))
        .add_card(digimon("SOURCE-CYBORG", &["Cyborg"], 8))
        .add_card(digimon("SOURCE-PLAIN", &[], 10))
        .add_card(digimon("ATTACKER", &[], 3))
        .add_card(digimon("FILL", &[], 3))
}

fn runner() -> DebugRunner {
    runner_builder().memory(10).start()
}

fn zone_ids(cards: &[CardSource], data: &[CardData]) -> Vec<String> {
    cards
        .iter()
        .map(|card| card.card_id(data).to_string())
        .collect()
}

#[test]
#[ignore = "pending: G-PLAY-COST-LTE-BINDING — BT15-096 Delay needs hand Digimon play_cost <= selected source Digimon play cost; current DSL only supports literal play_cost_lte"]
fn bt15_096_yaml_compiles_to_printed_main_delay_and_security_clauses() {
    let runner = runner();
    let card = runner.compiled_card("BT15-096").expect("BT15-096 compiled");

    assert_eq!(card.name, "Supreme Connection!");
    assert_eq!(card.kind, CompiledCardKind::Option);
    assert_eq!(card.cost, Some(3));
    assert_eq!(card.effects.len(), 3);

    let main = card
        .effects
        .iter()
        .find_map(|clause| match clause {
            CompiledClause::Triggered(triggered)
                if triggered.when.contains(&CompiledTiming::MainFromHand) =>
            {
                Some(triggered)
            }
            _ => None,
        })
        .expect("[Main] clause exists");
    assert!(
        main.process
            .iter()
            .any(|step| matches!(step, CompiledStep::RevealTopDeck { count: 3, .. })),
        "[Main] must reveal exactly top 3"
    );
    assert!(
        main.process
            .iter()
            .any(|step| matches!(step, CompiledStep::AddToHandFromReveal { .. })),
        "[Main] must add one matching revealed card to hand"
    );
    assert!(
        main.process
            .iter()
            .any(|step| matches!(step, CompiledStep::TrashFromReveal { .. })),
        "[Main] must trash one matching revealed card"
    );
    assert!(
        main.process.iter().any(|step| matches!(
            step,
            CompiledStep::PlaceRemainderOnDeck {
                position: digimon_dsl::compiled::CompiledStackPosition::Top,
                ..
            }
        )),
        "[Main] must return unchosen revealed cards to top of deck"
    );

    let delay = card
        .effects
        .iter()
        .find_map(|clause| match clause {
            CompiledClause::Declarative(CompiledDeclarativeClause::Delay {
                trigger,
                process,
                ..
            }) => Some((trigger, process)),
            _ => None,
        })
        .expect("[Delay] clause exists");
    assert_eq!(*delay.0, CompiledTiming::EndOfYourNextTurn);
    assert!(
        delay
            .1
            .iter()
            .any(|step| matches!(step, CompiledStep::SelectOwnPermanent { .. })),
        "[Delay] must first choose one of your Machine/Cyborg Digimon"
    );
    assert!(
        delay.1.iter().any(|step| matches!(
            step,
            CompiledStep::PlayFromHand {
                cost_delta: Some(CompiledCostDelta::Reduce(3)),
                ..
            }
        )),
        "[Delay] must play the selected hand Digimon with play cost reduced by 3"
    );

    assert!(
        card.effects.iter().any(|clause| match clause {
            CompiledClause::Triggered(triggered) => {
                triggered.when.contains(&CompiledTiming::OnSecurity)
                    && triggered
                        .process
                        .iter()
                        .any(|step| matches!(step, CompiledStep::PlaceSelfAsDelayOption))
            }
            _ => false,
        }),
        "[Security] must place this card in the battle area"
    );
}

#[test]
#[ignore = "pending: G-PLAY-COST-LTE-BINDING — BT15-096 Delay needs hand Digimon play_cost <= selected source Digimon play cost; current DSL only supports literal play_cost_lte"]
fn bt15_096_main_adds_and_trashes_distinct_machine_or_cyborg_cards_then_tops_rest() {
    let mut runner = runner_builder()
        .hand(0, &["BT15-096"])
        .deck(0, &["FILL", "PLAIN-DIGIMON", "CYBORG-HAND", "MACHINE-HAND"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();

    assert!(runner.game.activate_hand_main(0, 0));
    assert_eq!(runner.pending_action_count(), 2);
    runner
        .auto_resolve()
        .expect("pick add/trash reveal buckets");

    let hand_ids = zone_ids(&runner.game.player(0).hand, &runner.game.card_data);
    let trash_ids = zone_ids(&runner.game.player(0).trash, &runner.game.card_data);
    let deck_ids = zone_ids(&runner.game.player(0).deck, &runner.game.card_data);

    assert!(
        hand_ids
            .iter()
            .any(|id| id == "MACHINE-HAND" || id == "CYBORG-HAND"),
        "one revealed Machine/Cyborg card should enter hand"
    );
    assert!(
        trash_ids
            .iter()
            .any(|id| id == "MACHINE-HAND" || id == "CYBORG-HAND"),
        "one distinct revealed Machine/Cyborg card should be trashed"
    );
    assert!(
        deck_ids.ends_with(&["PLAIN-DIGIMON".to_string()]),
        "the non-picked revealed card should be returned to the top of the deck"
    );
    assert!(runner.game.player(0).battle_area.iter().any(|perm| {
        perm.top_card().card_id(&runner.game.card_data) == "BT15-096"
            && matches!(perm.option_state, OptionState::Delayed { .. })
    }));
}

#[test]
#[ignore = "pending: G-PLAY-COST-LTE-BINDING — BT15-096 Delay needs hand Digimon play_cost <= selected source Digimon play cost; current DSL only supports literal play_cost_lte"]
fn bt15_096_main_requires_machine_or_cyborg_and_skips_tamer_trait_matches() {
    let mut runner = runner_builder()
        .hand(0, &["BT15-096"])
        .deck(
            0,
            &["FILL", "PLAIN-DIGIMON", "MACHINE-TAMER", "MACHINE-HAND"],
        )
        .deck(1, &["FILL"])
        .memory(10)
        .start();

    assert!(runner.game.activate_hand_main(0, 0));
    assert_eq!(
        runner.pending_action_count(),
        1,
        "Machine/Cyborg trait alone is insufficient; the add/trash candidates must be cards matching the printed trait bucket"
    );
}

#[test]
#[ignore = "pending: G-PLAY-COST-LTE-BINDING — BT15-096 Delay needs hand Digimon play_cost <= selected source Digimon play cost; current DSL only supports literal play_cost_lte"]
fn bt15_096_delay_filters_source_trait_and_hand_play_cost_cap() {
    let mut runner = runner_builder()
        .hand(0, &["CHEAP-MACHINE", "EXPENSIVE-CYBORG"])
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();
    runner.place_on_field(0, "BT15-096", Some(0));
    runner.place_on_field(0, "SOURCE-MACHINE", Some(0));
    runner.place_on_field(0, "SOURCE-PLAIN", Some(0));

    runner.game.enter_main_phase();
    runner.auto_resolve().expect("activate BT15-096 delay");

    let source_pick = runner
        .pending_selection_view()
        .expect("Delay must expose a source-Digimon selection");
    assert_eq!(source_pick.valid_action_ids.len(), 1);
    runner
        .execute_action(
            source_pick.selecting_player,
            source_pick.valid_action_ids[0],
        )
        .expect("choose Machine source");

    let hand_pick = runner
        .pending_selection_view()
        .expect("Delay must expose a hand-card selection after choosing source");
    assert!(
        hand_pick.valid_action_ids.len() == 1,
        "only hand Digimon with play cost <= selected source play cost should be legal"
    );
    runner
        .execute_action(hand_pick.selecting_player, hand_pick.valid_action_ids[0])
        .expect("play cheap Machine with cost reduced by 3");

    assert!(runner
        .game
        .player(0)
        .battle_area
        .iter()
        .any(|perm| { perm.top_card().card_id(&runner.game.card_data) == "CHEAP-MACHINE" }));
    assert!(
        !runner
            .game
            .player(0)
            .battle_area
            .iter()
            .any(|perm| { perm.top_card().card_id(&runner.game.card_data) == "EXPENSIVE-CYBORG" }),
        "a hand Digimon above the selected source's play cost must remain illegal"
    );
}

#[test]
#[ignore = "pending: G-PLAY-COST-LTE-BINDING — BT15-096 Delay needs hand Digimon play_cost <= selected source Digimon play cost; current DSL only supports literal play_cost_lte"]
fn bt15_096_delay_can_be_declined_without_trashing_the_option() {
    let mut runner = runner_builder()
        .hand(0, &["CHEAP-MACHINE"])
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();
    runner.place_on_field(0, "BT15-096", Some(0));
    runner.place_on_field(0, "SOURCE-CYBORG", Some(0));

    runner.game.enter_main_phase();
    runner.auto_resolve().expect("activate BT15-096 delay");
    assert!(
        runner.pending_is_optional(),
        "Delay play effect is printed as may"
    );
    runner
        .execute_action(0, PASS)
        .expect("decline Delay source selection");

    assert!(runner.game.player(0).battle_area.iter().any(|perm| {
        perm.top_card().card_id(&runner.game.card_data) == "BT15-096"
            && matches!(perm.option_state, OptionState::Delayed { .. })
    }));
}

#[test]
#[ignore = "pending: G-PLAY-COST-LTE-BINDING — BT15-096 Delay needs hand Digimon play_cost <= selected source Digimon play cost; current DSL only supports literal play_cost_lte"]
fn bt15_096_security_places_self_in_battle_area() {
    let mut runner = runner_builder()
        .security(1, &["BT15-096"])
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(0)
        .start();
    let attacker = runner.place_on_field(0, "ATTACKER", Some(0));

    let result = runner.attack_player(attacker, 1, false);

    assert_eq!(result, AttackResult::SecurityCheckSurvived);
    assert_eq!(runner.security_count(1), 0);
    assert_eq!(runner.trash_size(1), 0);
    assert!(runner.game.player(1).battle_area.iter().any(|perm| {
        perm.top_card().card_id(&runner.game.card_data) == "BT15-096"
            && matches!(
                perm.option_state,
                OptionState::Delayed {
                    trigger: DelayTrigger::EndOfYourNextTurn,
                    ..
                }
            )
    }));
}
