//! BT23-077 Sistermon Ciel — Digimon, Lv.4, White, DP4000, Cost4.
//!
//! # Card text (cards.json)
//!
//! ```text
//! <Blocker> (This Digimon can block in the blocker timing.)
//! [On Play] Delete 1 of your opponent's Digimon with a play cost of 4 or less.
//! [All Turns] When this Digimon suspends, <De-Digivolve 1> 1 of your opponent's Digimon.
//! (Trash the top card. You can't trash past level 3 cards.)
//! ```
//!
//! # Ingestion note
//!
//! `inherited_effect_description_eng` duplicates "Card Effect(s) ..." for this card.
//! Treat that as data ingestion evidence only; BT23-077 must not gain inherited
//! versions of its face-up effects.
//!
//! # Legacy reference
//!
//! DCGO `OnSuspend.cs` models "when this permanent suspends" as a predicate
//! over the suspended permanent containing the effect's card source. The Rust
//! DSL equivalent is `event_permanent_is_source: true`.
//!
//! # Coverage
//!
//! - Metadata and static <Blocker>.
//! - Faithful [On Play] mandatory opponent Digimon selection filtered by play cost <= 4.
//! - Self-scoped [All Turns] OnSuspend <De-Digivolve 1>, gated so ally suspend
//!   events do not over-fire.

use digimon_dsl::compiled::{
    CompiledClause, CompiledColor, CompiledDeclarativeClause, CompiledPredicate, CompiledScope,
    CompiledTiming,
};
use digimon_engine::action::space::encode_attack;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardKind, Keyword};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::SelectionKind;

#[test]
fn bt23_077_metadata_blocker_and_supported_clause_shape_match_printed_text() {
    let runner = sistermon_runner().start();
    let compiled = runner.compiled_card("BT23-077").expect("BT23-077 compiles");

    assert_eq!(compiled.name, "Sistermon Ciel");
    assert_eq!(compiled.level, Some(4));
    assert_eq!(compiled.cost, Some(4));
    assert_eq!(compiled.dp, Some(4000));
    assert_eq!(compiled.color, vec![CompiledColor::White]);
    assert!(compiled
        .traits
        .iter()
        .any(|trait_name| trait_name == "Puppet"));
    assert!(compiled.traits.iter().any(|trait_name| trait_name == "CS"));

    let blocker_count = compiled
        .effects
        .iter()
        .filter(|clause| {
            matches!(
                clause,
                CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword {
                    keyword,
                    scope: CompiledScope::FaceUp,
                    ..
                }) if keyword == "Blocker"
            )
        })
        .count();
    assert_eq!(
        blocker_count, 1,
        "printed <Blocker> must be represented once as a face-up keyword"
    );

    let on_play_count = compiled
        .effects
        .iter()
        .filter(|clause| {
            matches!(
                clause,
                CompiledClause::Triggered(triggered)
                    if triggered.scope == CompiledScope::FaceUp
                        && triggered.when == vec![CompiledTiming::OnPlay]
            )
        })
        .count();
    assert_eq!(on_play_count, 1, "the On Play delete slice is active");

    let on_suspend = compiled
        .effects
        .iter()
        .find_map(|clause| match clause {
            CompiledClause::Triggered(triggered)
                if triggered.scope == CompiledScope::FaceUp
                    && triggered.when == vec![CompiledTiming::OnSuspend] =>
            {
                Some(triggered)
            }
            _ => None,
        })
        .expect("printed self-suspend De-Digivolve clause should be authored");
    assert!(
        on_suspend
            .condition
            .as_ref()
            .is_some_and(predicate_has_event_permanent_is_source),
        "BT23-077 OnSuspend must be gated to the suspending event permanent being this source"
    );

    assert!(
        !compiled.effects.iter().any(|clause| matches!(
            clause,
            CompiledClause::Triggered(triggered)
                if triggered.scope == CompiledScope::Inherited
        )),
        "do not turn the duplicated ingestion text into inherited effects"
    );
}

#[test]
fn bt23_077_has_blocker_at_runtime() {
    let mut runner = sistermon_runner().start();
    let ciel = runner.place_on_field(0, "BT23-077", Some(0));

    assert!(
        runner.game.has_keyword(ciel, Keyword::Blocker),
        "BT23-077 must have its printed Blocker keyword while face up"
    );
}

#[test]
fn bt23_077_on_play_prompts_only_opponent_digimon_with_play_cost_4_or_less() {
    let mut runner = sistermon_runner()
        .add_card(make_digimon("CHEAP-DIGIMON", 4))
        .add_card(make_digimon("EXPENSIVE-DIGIMON", 5))
        .hand(0, &["BT23-077"])
        .memory(10)
        .start();
    let cheap = runner.place_on_field(1, "CHEAP-DIGIMON", Some(0));
    let expensive = runner.place_on_field(1, "EXPENSIVE-DIGIMON", Some(0));

    runner.play(0, 0).expect("play Sistermon Ciel");

    let view = runner
        .pending_selection_view()
        .expect("mandatory cost-filtered opponent Digimon selection");
    assert_eq!(view.kind, SelectionKind::OppField);
    assert!(
        !view.is_optional,
        "printed On Play delete is mandatory when a legal target exists"
    );
    assert_eq!(
        view.valid_action_ids,
        vec![encode_permanent(cheap)],
        "only play-cost <=4 opponent Digimon should be legal; expensive={}",
        encode_permanent(expensive)
    );
}

#[test]
fn bt23_077_on_play_deletes_selected_cost_4_or_less_opponent_digimon() {
    let mut runner = sistermon_runner()
        .add_card(make_digimon("CHEAP-DIGIMON", 4))
        .add_card(make_digimon("EXPENSIVE-DIGIMON", 5))
        .hand(0, &["BT23-077"])
        .memory(10)
        .start();
    let cheap = runner.place_on_field(1, "CHEAP-DIGIMON", Some(0));
    runner.place_on_field(1, "EXPENSIVE-DIGIMON", Some(0));

    runner.play(0, 0).expect("play Sistermon Ciel");
    runner
        .execute_action(0, encode_permanent(cheap))
        .expect("choose cheap Digimon");
    runner.auto_resolve().expect("finish On Play");

    assert_eq!(
        runner.battle_area_size(1),
        1,
        "selected cheap Digimon is deleted; cost-5 Digimon remains"
    );
    assert_eq!(
        runner.game.players[1].trash.len(),
        1,
        "deleted target moves to its owner's trash"
    );
}

#[test]
fn bt23_077_on_play_with_only_cost_5_opponent_digimon_installs_no_prompt() {
    let mut runner = sistermon_runner()
        .add_card(make_digimon("EXPENSIVE-DIGIMON", 5))
        .hand(0, &["BT23-077"])
        .memory(10)
        .start();
    runner.place_on_field(1, "EXPENSIVE-DIGIMON", Some(0));

    runner.play(0, 0).expect("play Sistermon Ciel");

    assert!(
        runner.pending_selection().is_none(),
        "cost-5 opponent Digimon must not be selectable for the cost <=4 delete"
    );
    assert_eq!(
        runner.battle_area_size(1),
        1,
        "ineligible opponent Digimon remains"
    );
}

#[test]
fn bt23_077_self_suspend_de_digivolves_one_opponent_digimon_but_other_suspend_does_not_trigger() {
    let mut runner = sistermon_runner()
        .add_card(make_digimon("ALLY", 3))
        .add_card(make_digimon("BASE-LV3", 3))
        .add_card(make_digimon("TOP-LV4", 4))
        .start();
    let ciel = runner.place_on_field(0, "BT23-077", Some(0));
    let ally = runner.place_on_field(0, "ALLY", Some(0));
    let opponent_stack = runner.place_stack(1, &["BASE-LV3", "TOP-LV4"]);

    runner.game.suspend(ally);
    assert!(
        runner.pending_selection().is_none(),
        "another permanent suspending must not trigger Sistermon Ciel's self-suspend clause"
    );
    assert_eq!(stack_size(&runner, opponent_stack), 2);

    runner.game.suspend(ciel);
    let view = runner
        .pending_selection_view()
        .expect("self-suspend De-Digivolve target selection");
    assert_eq!(view.kind, SelectionKind::OppField);
    assert_eq!(
        view.valid_action_ids,
        vec![encode_permanent(opponent_stack)]
    );
    runner
        .execute_action(0, view.valid_action_ids[0])
        .expect("choose opponent stack");
    runner.auto_resolve().expect("finish De-Digivolve");

    assert_eq!(stack_size(&runner, opponent_stack), 1);
}

fn sistermon_runner() -> digimon_engine::debug_runner::DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card("BT23-077")
        .expect("BT23-077 YAML loads")
}

fn make_digimon(id: &str, play_cost: u16) -> digimon_engine::card_data::CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.level = Some(3);
    card.dp = Some(3000);
    card.play_cost = play_cost;
    card
}

fn encode_permanent(handle: PermanentHandle) -> u16 {
    encode_attack(0, handle.index as u16)
}

fn stack_size(runner: &DebugRunner, handle: PermanentHandle) -> usize {
    runner.game.players[handle.player as usize].battle_area[handle.index as usize]
        .card_sources
        .len()
}

fn predicate_has_event_permanent_is_source(predicate: &CompiledPredicate) -> bool {
    predicate.event_permanent_is_source == Some(true)
        || predicate
            .all_of
            .iter()
            .any(predicate_has_event_permanent_is_source)
        || predicate
            .any_of
            .iter()
            .any(predicate_has_event_permanent_is_source)
        || predicate
            .none_of
            .iter()
            .any(predicate_has_event_permanent_is_source)
        || predicate
            .not
            .as_ref()
            .is_some_and(|p| predicate_has_event_permanent_is_source(p))
}
