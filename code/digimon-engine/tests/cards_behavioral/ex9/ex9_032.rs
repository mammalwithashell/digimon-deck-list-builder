//! EX9-032 Karakurumon — Lv.5, Yellow/Purple, Puppet/LIBERATOR, Data.
//!
//! # Card text (cards.json)
//!
//! ```text
//! [Digivolve] Lv.4 w/[Puppet] trait: Cost 3
//!
//! [On Play] [When Digivolving] By deleting 1 of your Tokens or other
//! [Puppet] trait Digimon, this Digimon may digivolve into a Digimon card
//! with the [Puppet] trait in the hand without paying the cost.
//!
//! Inherited: [All Turns] [Once Per Turn] When this Digimon would leave the
//! battle area other than by your effects, by deleting 1 of your Tokens or
//! other [Puppet] trait Digimon, prevent it from leaving.
//! ```
//!
//! # Legacy reference
//!
//! `code/engine_py_legacy/engine/data/scripts/ex9/ex9_032.py`
//!
//! # Coverage
//!
//! - Structural metadata and Lv.4 yellow / Lv.4 Puppet digivolution paths.
//! - Gap-routed tests for the active [On Play] / [When Digivolving] costed
//!   free Puppet hand digivolve.
//!
//! # Omitted
//!
//! - Inherited leave-prevention uses the shared would-leave replacement
//!   framework and the Token/other-Puppet cost body.

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledColor, CompiledCost, CompiledTiming,
};
use digimon_engine::action::space::{encode_attack, PASS};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardKind, EffectTiming};
use digimon_engine::replacement::ReplacementCause;
use digimon_engine::selection::{SelectionKind, TriggerSource};

#[test]
fn ex9_032_metadata_and_digivolution_paths_match_printed_text() {
    let runner = karakurumon_runner();
    let compiled = runner.compiled_card("EX9-032").expect("EX9-032 compiles");

    assert_eq!(compiled.name, "Karakurumon");
    assert_eq!(compiled.level, Some(5));
    assert_eq!(compiled.cost, Some(7));
    assert_eq!(compiled.dp, Some(7000));
    assert_eq!(compiled.attribute.as_deref(), Some("Data"));
    assert!(compiled.color.contains(&CompiledColor::Yellow));
    assert!(compiled.color.contains(&CompiledColor::Purple));
    assert!(compiled
        .traits
        .iter()
        .any(|trait_name| trait_name == "Puppet"));
    assert!(compiled
        .traits
        .iter()
        .any(|trait_name| trait_name == "LIBERATOR"));

    let digivolve_paths: Vec<_> = compiled
        .alt_paths
        .iter()
        .filter(|path| path.kind == CompiledAltPathKind::Digivolve)
        .collect();

    assert!(
        digivolve_paths.iter().any(|path| {
            path.cost == Some(CompiledCost::Literal(4))
                && path.from.as_ref().is_some_and(|from| {
                    from.level_eq == Some(4) && from.color_is == Some(CompiledColor::Yellow)
                })
        }),
        "standard yellow Lv.4 digivolve path must cost 4; paths={digivolve_paths:?}"
    );
    assert!(
        digivolve_paths.iter().any(|path| {
            path.cost == Some(CompiledCost::Literal(3))
                && path.from.as_ref().is_some_and(|from| {
                    from.level_eq == Some(4) && from.trait_has.as_deref() == Some("Puppet")
                })
        }),
        "alternate Puppet Lv.4 digivolve path must cost 3; paths={digivolve_paths:?}"
    );
}

#[test]
fn ex9_032_active_costed_digivolve_clause_is_compiled() {
    let runner = karakurumon_runner();
    let compiled = runner.compiled_card("EX9-032").expect("EX9-032 compiles");

    assert!(
        compiled.effects.iter().any(|clause| matches!(
            clause,
            CompiledClause::Triggered(triggered)
                if triggered.when.contains(&CompiledTiming::OnPlay)
                    && triggered.when.contains(&CompiledTiming::WhenDigivolving)
        )),
        "active costed self-digivolve [On Play]/[When Digivolving] clause must compile"
    );
}

#[test]
fn ex9_032_on_play_deletes_token_or_other_puppet_then_free_digivolves_into_puppet() {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX9-032")
        .expect("EX9-032 YAML loads")
        .add_card(make_puppet_digimon("PUPPET-COST", 3))
        .add_card(make_plain_digimon("PLAIN-COST", 3))
        .add_card(make_token("TOKEN-COST"))
        .add_card(make_puppet_digimon("PUPPET-EVO", 6))
        .add_card(make_plain_digimon("NONPUPPET-EVO", 6))
        .hand(0, &["EX9-032", "NONPUPPET-EVO", "PUPPET-EVO"])
        // The memory seesaw caps at 10; Karakurumon's 7-cost play leaves 3.
        .memory(10)
        .start();

    let puppet_cost = runner.place_on_field(0, "PUPPET-COST", Some(0));
    let plain_cost = runner.place_on_field(0, "PLAIN-COST", Some(0));
    let token_cost = runner.place_on_field(0, "TOKEN-COST", Some(0));
    let karakurumon_index = runner.play(0, 0).expect("play Karakurumon");

    let cost_view = runner
        .pending_selection_view()
        .expect("cost selection should be pending");
    assert_eq!(cost_view.kind, SelectionKind::OwnField);
    assert!(
        runner.pending_is_optional(),
        "cost selection must allow declining the effect"
    );
    assert!(cost_view
        .valid_action_ids
        .contains(&encode_permanent(puppet_cost)));
    assert!(cost_view
        .valid_action_ids
        .contains(&encode_permanent(token_cost)));
    assert!(
        !cost_view
            .valid_action_ids
            .contains(&encode_permanent(plain_cost)),
        "non-token non-Puppet Digimon must not be legal cost"
    );
    assert!(
        !cost_view
            .valid_action_ids
            .contains(&encode_attack(0, karakurumon_index as u16)),
        "Karakurumon itself must not be legal as the 'other Puppet' cost"
    );

    runner
        .execute_action(0, encode_permanent(puppet_cost))
        .expect("delete Puppet cost");
    assert!(
        !permanent_exists(&runner, 0, "PUPPET-COST"),
        "selected cost body is deleted before the digivolve"
    );

    let hand_view = runner
        .pending_selection_view()
        .expect("Puppet hand digivolve selection should be pending");
    assert_eq!(hand_view.kind, SelectionKind::Hand);
    assert_eq!(
        hand_view.valid_action_ids.len(),
        1,
        "only Puppet Digimon cards in hand can be selected"
    );
    runner
        .execute_action(0, hand_view.valid_action_ids[0])
        .expect("select Puppet evolution");
    runner.auto_resolve().expect("finish free digivolve");

    assert!(
        permanent_exists(&runner, 0, "PUPPET-EVO"),
        "Karakurumon digivolved into the selected Puppet card"
    );
    assert!(
        hand_contains(&runner, 0, "NONPUPPET-EVO"),
        "non-Puppet hand card was not consumed"
    );
    assert_eq!(
        runner.memory(),
        3,
        "only the 7-cost Karakurumon play was paid (10 - 7); the effect \
         digivolve was free"
    );
}

#[test]
fn ex9_032_declining_on_play_cost_does_not_delete_or_digivolve() {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX9-032")
        .expect("EX9-032 YAML loads")
        .add_card(make_puppet_digimon("PUPPET-COST", 3))
        .add_card(make_puppet_digimon("PUPPET-EVO", 6))
        .hand(0, &["EX9-032", "PUPPET-EVO"])
        .memory(20)
        .start();

    runner.place_on_field(0, "PUPPET-COST", Some(0));
    runner.play(0, 0).expect("play Karakurumon");

    assert!(runner.pending_is_optional(), "PASS must decline the cost");
    runner
        .execute_action(0, PASS)
        .expect("decline Karakurumon effect");
    runner.auto_resolve().expect("finish decline");

    assert!(permanent_exists(&runner, 0, "PUPPET-COST"));
    assert!(permanent_exists(&runner, 0, "EX9-032"));
    assert!(hand_contains(&runner, 0, "PUPPET-EVO"));
}

#[test]
fn ex9_032_when_digivolving_uses_same_cost_and_free_puppet_hand_digivolve_flow() {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX9-032")
        .expect("EX9-032 YAML loads")
        .add_card(make_plain_digimon("BASE", 4))
        .add_card(make_puppet_digimon("PUPPET-COST", 3))
        .add_card(make_puppet_digimon("PUPPET-EVO", 6))
        .hand(0, &["PUPPET-EVO"])
        .memory(10)
        .start();

    let karakurumon = runner.place_stack(0, &["BASE", "EX9-032"]);
    let puppet_cost = runner.place_on_field(0, "PUPPET-COST", Some(0));
    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(karakurumon),
    );
    runner.game.drain_effect_queue();

    let cost_view = runner.pending_selection_view().expect("cost selection");
    assert_eq!(cost_view.kind, SelectionKind::OwnField);
    assert_eq!(
        cost_view.valid_action_ids,
        vec![encode_permanent(puppet_cost)],
        "only the other Puppet is a legal cost"
    );
    runner
        .execute_action(0, cost_view.valid_action_ids[0])
        .expect("delete Puppet cost");

    let hand_view = runner
        .pending_selection_view()
        .expect("Puppet hand digivolve selection");
    assert_eq!(hand_view.kind, SelectionKind::Hand);
    runner
        .execute_action(0, hand_view.valid_action_ids[0])
        .expect("select Puppet evolution");
    runner.auto_resolve().expect("finish digivolve");

    assert!(permanent_exists(&runner, 0, "PUPPET-EVO"));
    assert!(!permanent_exists(&runner, 0, "PUPPET-COST"));
    assert_eq!(runner.memory(), 10, "effect digivolve is free");
}

#[test]
fn ex9_032_does_not_prompt_without_legal_cost_body() {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX9-032")
        .expect("EX9-032 YAML loads")
        .add_card(make_plain_digimon("PLAIN-COST", 3))
        .add_card(make_puppet_digimon("PUPPET-EVO", 6))
        .hand(0, &["EX9-032", "PUPPET-EVO"])
        .memory(20)
        .start();

    runner.place_on_field(0, "PLAIN-COST", Some(0));
    runner.play(0, 0).expect("play Karakurumon");

    assert!(
        runner.pending_selection().is_none(),
        "no Token or other Puppet means no cost prompt"
    );
    assert!(permanent_exists(&runner, 0, "PLAIN-COST"));
    assert!(hand_contains(&runner, 0, "PUPPET-EVO"));
}

#[test]
fn ex9_032_inherited_prevents_leaving_by_deleting_token_or_other_puppet() {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX9-032")
        .expect("EX9-032 YAML loads")
        .add_card(make_plain_digimon("CARRIER", 6))
        .add_card(make_token("TOKEN-COST"))
        .start();

    let carrier = runner.place_stack(0, &["EX9-032", "CARRIER"]);
    let token = runner.place_on_field(0, "TOKEN-COST", Some(0));

    runner
        .game
        .delete_permanent_with_cause(carrier, ReplacementCause::OpponentEffect);

    let accept = runner
        .pending_selection_view()
        .expect("inherited replacement accept prompt");
    assert_eq!(accept.kind, SelectionKind::Replacement);
    assert!(accept.is_optional);
    runner
        .execute_action(0, accept.valid_action_ids[0])
        .expect("accept inherited replacement");

    let view = runner
        .pending_selection_view()
        .expect("inherited replacement cost selection");
    assert_eq!(view.kind, SelectionKind::OwnField);
    assert_eq!(view.valid_action_ids, vec![encode_permanent(token)]);
    runner
        .execute_action(0, view.valid_action_ids[0])
        .expect("delete token cost");
    runner.auto_resolve().expect("finish replacement");

    assert!(permanent_exists(&runner, 0, "CARRIER"));
    assert!(!permanent_exists(&runner, 0, "TOKEN-COST"));
}

fn karakurumon_runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("EX9-032")
        .expect("EX9-032 YAML loads")
        .start()
}

fn encode_permanent(handle: digimon_engine::permanent::PermanentHandle) -> u16 {
    encode_attack(handle.player as u16, handle.index as u16)
}

fn make_puppet_digimon(id: &str, level: u8) -> CardData {
    let mut card = make_plain_digimon(id, level);
    card.traits = vec!["Puppet".to_string()];
    card
}

fn make_plain_digimon(id: &str, level: u8) -> CardData {
    let mut card = make_test_card(id, id);
    card.level = Some(level);
    card.card_kind = CardKind::Digimon;
    card
}

fn make_token(id: &str) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Token;
    card.traits = vec!["Token".to_string()];
    card
}

fn permanent_exists(runner: &DebugRunner, player: usize, card_id: &str) -> bool {
    runner.game.players[player]
        .battle_area
        .iter()
        .any(|permanent| permanent.top_card().card_id(&runner.game.card_data) == card_id)
}

fn hand_contains(runner: &DebugRunner, player: usize, card_id: &str) -> bool {
    runner.game.players[player]
        .hand
        .iter()
        .any(|card| card.card_id(&runner.game.card_data) == card_id)
}
