//! BT5-112 Omnimon Zwart Defeat - Digimon, Lv.7, White/Purple.
//!
//! # Card text (cards.json)
//!
//! [Security] Play this card without battling and without paying the cost.
//! [When Digivolving] Delete 1 of your opponent's Tamers.
//! [On Deletion] Delete 1 of your opponent's Digimon.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT5/White/BT5_112.cs (submodule not initialized)
//!
//! # Patterns this test covers
//! - Security play-self via `play_from_security`.
//! - Mandatory [When Digivolving] opponent Tamer target selection.
//! - Mandatory [On Deletion] opponent Digimon target selection.

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledCardKind, CompiledClause, CompiledColor, CompiledCost,
    CompiledTiming,
};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardKind, EffectTiming};
use digimon_engine::selection::{SelectionKind, TriggerSource};

fn tamer(id: &str) -> digimon_engine::card_data::CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Tamer;
    card
}

fn digimon(id: &str) -> digimon_engine::card_data::CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card
}

fn runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("BT5-112")
        .expect("BT5-112 YAML parses and compiles")
        .add_card(tamer("OPP-TAMER"))
        .add_card(digimon("OPP-DIGIMON"))
        .add_card(digimon("OPP-OTHER-DIGIMON"))
        .add_card(digimon("ATTACKER"))
        .memory(20)
        .start()
}

fn resolve_first_pending(runner: &mut DebugRunner) {
    let view = runner
        .pending_selection_view()
        .expect("pending selection must be installed");
    runner
        .execute_action(view.selecting_player, view.valid_action_ids[0])
        .expect("first legal pending action resolves");
}

#[test]
fn bt5_112_metadata_and_digivolve_paths_match_printed_card() {
    let runner = runner();
    let compiled = runner
        .compiled_card("BT5-112")
        .expect("BT5-112 compiled card");

    assert_eq!(compiled.kind, CompiledCardKind::Digimon);
    assert_eq!(compiled.level, Some(7));
    assert_eq!(compiled.cost, Some(15));
    assert_eq!(compiled.dp, Some(13000));
    assert!(
        compiled.color.contains(&CompiledColor::White),
        "BT5-112 must be white"
    );
    assert!(
        compiled.color.contains(&CompiledColor::Purple),
        "BT5-112 must include purple for the requested white/purple metadata"
    );

    let digivolve_paths: Vec<_> = compiled
        .alt_paths
        .iter()
        .filter(|path| matches!(path.kind, CompiledAltPathKind::Digivolve))
        .collect();
    assert_eq!(
        digivolve_paths.len(),
        2,
        "BT5-112 must have black Lv6/cost 3 and purple Lv6/cost 3 digivolve paths"
    );

    for color in [CompiledColor::Black, CompiledColor::Purple] {
        assert!(
            digivolve_paths.iter().any(|path| {
                let from = path
                    .from
                    .as_deref()
                    .expect("digivolve path has source predicate");
                from.level_eq == Some(6)
                    && from.color_is == Some(color)
                    && path.cost == Some(CompiledCost::Literal(3))
            }),
            "missing Lv6/{color:?}/cost 3 digivolve path; paths={digivolve_paths:?}"
        );
    }
}

#[test]
fn bt5_112_has_security_when_digivolving_and_on_deletion_clauses() {
    let runner = runner();
    let compiled = runner
        .compiled_card("BT5-112")
        .expect("BT5-112 compiled card");

    let triggered: Vec<_> = compiled
        .effects
        .iter()
        .filter_map(|clause| match clause {
            CompiledClause::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .collect();

    assert!(
        triggered
            .iter()
            .any(|clause| clause.when == vec![CompiledTiming::OnSecurity] && !clause.optional),
        "BT5-112 must have a mandatory [Security] play-self clause"
    );
    assert!(
        triggered.iter().any(|clause| {
            clause.when == vec![CompiledTiming::WhenDigivolving] && !clause.optional
        }),
        "BT5-112 must have a mandatory [When Digivolving] delete-Tamer clause"
    );
    assert!(
        triggered
            .iter()
            .any(|clause| clause.when == vec![CompiledTiming::OnDeletion] && !clause.optional),
        "BT5-112 must have a mandatory [On Deletion] delete-Digimon clause"
    );
}

#[test]
fn bt5_112_when_digivolving_selects_only_opponent_tamers() {
    let mut runner = runner();
    let zwart_defeat = runner.place_on_field(0, "BT5-112", Some(0));
    runner.place_on_field(1, "OPP-TAMER", Some(0));
    runner.place_on_field(1, "OPP-DIGIMON", Some(0));

    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(zwart_defeat),
    );
    runner.game.drain_effect_queue();

    let view = runner
        .pending_selection_view()
        .expect("When Digivolving must install opponent Tamer delete selection");
    assert_eq!(view.kind, SelectionKind::OppField);
    assert!(
        !view.is_optional,
        "delete-Tamer selection is mandatory when a legal target exists"
    );
    assert_eq!(
        view.valid_action_ids.len(),
        1,
        "only the opponent Tamer should be targetable, not opponent Digimon"
    );

    resolve_first_pending(&mut runner);
    assert_eq!(
        runner.battle_area_size(1),
        1,
        "the selected Tamer is deleted and the opponent Digimon remains"
    );
}

#[test]
fn bt5_112_when_digivolving_no_prompt_without_opponent_tamer() {
    let mut runner = runner();
    let zwart_defeat = runner.place_on_field(0, "BT5-112", Some(0));
    runner.place_on_field(1, "OPP-DIGIMON", Some(0));

    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(zwart_defeat),
    );
    runner.game.drain_effect_queue();

    assert!(
        runner.pending_selection().is_none(),
        "no [When Digivolving] prompt should install without an opponent Tamer"
    );
    assert_eq!(
        runner.battle_area_size(1),
        1,
        "opponent Digimon must not be deleted by the Tamer-only clause"
    );
}

#[test]
fn bt5_112_on_deletion_selects_only_opponent_digimon() {
    let mut runner = runner();
    let zwart_defeat = runner.place_on_field(0, "BT5-112", Some(0));
    runner.place_on_field(1, "OPP-TAMER", Some(0));
    runner.place_on_field(1, "OPP-DIGIMON", Some(0));

    runner.game.delete_permanent_with_effects(zwart_defeat);

    let view = runner
        .pending_selection_view()
        .expect("On Deletion must install opponent Digimon delete selection");
    assert_eq!(view.kind, SelectionKind::OppField);
    assert!(
        !view.is_optional,
        "delete-Digimon selection is mandatory when a legal target exists"
    );
    assert_eq!(
        view.valid_action_ids.len(),
        1,
        "only the opponent Digimon should be targetable, not opponent Tamers"
    );

    resolve_first_pending(&mut runner);
    assert_eq!(
        runner.battle_area_size(1),
        1,
        "the selected Digimon is deleted and the opponent Tamer remains"
    );
}

#[test]
fn bt5_112_on_deletion_no_prompt_without_opponent_digimon() {
    let mut runner = runner();
    let zwart_defeat = runner.place_on_field(0, "BT5-112", Some(0));
    runner.place_on_field(1, "OPP-TAMER", Some(0));

    runner.game.delete_permanent_with_effects(zwart_defeat);

    assert!(
        runner.pending_selection().is_none(),
        "no [On Deletion] prompt should install without an opponent Digimon"
    );
    assert_eq!(
        runner.battle_area_size(1),
        1,
        "opponent Tamer must not be deleted by the Digimon-only clause"
    );
}

#[test]
fn bt5_112_security_plays_itself_without_paying_cost() {
    let mut runner = runner();
    let card_idx = runner
        .game
        .card_data
        .iter()
        .position(|card| card.card_id == "BT5-112")
        .expect("BT5-112 in card data");
    let card =
        digimon_engine::card_source::CardSource::new(card_idx, 1, runner.game.next_card_index());
    runner.game.players[1].security.push(card);
    let attacker = runner.place_on_field(0, "ATTACKER", Some(0));
    let memory_before = runner.memory();

    runner.attack_player(attacker, 1, false);
    runner.auto_resolve().expect("resolve security play");

    assert_eq!(
        runner.memory(),
        memory_before,
        "security play must not pay BT5-112's 15 memory play cost"
    );
    assert_eq!(
        runner.security_count(1),
        0,
        "BT5-112 must leave security after its [Security] effect resolves"
    );
    assert!(
        runner.game.players[1]
            .battle_area
            .iter()
            .any(|perm| perm.top_card().card_id(&runner.game.card_data) == "BT5-112"),
        "BT5-112 must be played to its owner's battle area from security"
    );
}
