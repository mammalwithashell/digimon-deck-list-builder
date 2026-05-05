//! EX11-023 Kaguyamon.
//!
//! Printed text covered here:
//! - <Alliance>.
//! - <Scapegoat>.
//! - [When Digivolving] [End of Opponent's Turn] [Once Per Turn] delete
//!   1 opponent Digimon with the lowest level.
//! - [All Turns] [Once Per Turn] when other Digimon are deleted, optionally
//!   play 1 level 4 or lower Puppet Digimon from trash.

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledCardKind, CompiledClause, CompiledColor, CompiledCost,
    CompiledDeclarativeClause, CompiledReplacementCause, CompiledScope, CompiledStep,
    CompiledTiming,
};
use digimon_engine::action::space::{PASS, TRASH_EFFECT_START};
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming, Keyword};
use digimon_engine::replacement::ReplacementCause;
use digimon_engine::selection::{SelectionKind, TriggerSource};

fn runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("EX11-023")
        .expect("EX11-023 YAML loads")
        .add_card(digimon("BASE-L5", 5, 7000, &[]))
        .add_card(digimon("OTHER-DIGIMON", 3, 3000, &[]))
        .add_card(digimon("OPP-L3", 3, 3000, &[]))
        .add_card(digimon("OPP-L4", 4, 5000, &[]))
        .add_card(digimon("OPP-L5", 5, 7000, &[]))
        .add_card(digimon("TRASH-PUPPET-L4", 4, 4000, &["Puppet"]))
        .add_card(digimon("TRASH-PUPPET-L5", 5, 7000, &["Puppet"]))
        .add_card(digimon("TRASH-BEAST-L3", 3, 3000, &["Beast"]))
        .memory(20)
        .start()
}

#[test]
fn ex11_023_has_printed_metadata_and_evolution_paths() {
    let runner = runner();
    let card = runner
        .compiled_card("EX11-023")
        .expect("EX11-023 must be compiled");

    assert_eq!(card.name, "Kaguyamon");
    assert_eq!(card.kind, CompiledCardKind::Digimon);
    assert_eq!(card.level, Some(6));
    assert_eq!(card.cost, Some(12));
    assert_eq!(card.dp, Some(12000));
    assert_eq!(
        card.color,
        vec![CompiledColor::Yellow, CompiledColor::Purple]
    );
    assert!(card.traits.iter().any(|trait_name| trait_name == "Puppet"));
    assert!(card
        .traits
        .iter()
        .any(|trait_name| trait_name == "LIBERATOR"));
    assert_eq!(card.attribute.as_deref(), Some("Data"));

    assert!(
        card.alt_paths.iter().any(|path| {
            path.kind == CompiledAltPathKind::Digivolve
                && matches!(path.cost, Some(CompiledCost::Literal(4)))
                && path.from.as_ref().is_some_and(|from| {
                    from.level_eq == Some(5) && from.color_is == Some(CompiledColor::Yellow)
                })
        }),
        "printed yellow Lv.5 digivolution should cost 4"
    );
    assert!(
        card.alt_paths.iter().any(|path| {
            path.kind == CompiledAltPathKind::Digivolve
                && matches!(path.cost, Some(CompiledCost::Literal(3)))
                && path.from.as_ref().is_some_and(|from| {
                    from.level_eq == Some(5) && from.trait_has.as_deref() == Some("Puppet")
                })
        }),
        "alternate Lv.5 Puppet digivolution should cost 3"
    );
}

#[test]
fn ex11_023_has_alliance_scapegoat_and_lowest_level_delete_clauses() {
    let runner = runner();
    let card = runner
        .compiled_card("EX11-023")
        .expect("EX11-023 must be compiled");

    assert!(
        card.effects.iter().any(|clause| matches!(
            clause,
            CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword {
                keyword,
                ..
            }) if keyword == "Alliance"
        )),
        "Alliance should be a native keyword grant"
    );

    let replacement = card
        .effects
        .iter()
        .find_map(|clause| match clause {
            CompiledClause::Declarative(CompiledDeclarativeClause::Replacement {
                scope,
                trigger,
                optional,
                active_when,
                process,
                ..
            }) if *scope == CompiledScope::FaceUp && trigger == "when_would_be_deleted" => {
                Some((optional, active_when, process))
            }
            _ => None,
        })
        .expect("Scapegoat should lower as a face-up would-delete replacement");

    assert!(*replacement.0, "Scapegoat is optional");
    assert!(
        replacement.1.as_ref().is_some_and(|pred| pred
            .none_of
            .iter()
            .any(|inner| inner.replacement_cause == Some(CompiledReplacementCause::OwnEffect))),
        "Scapegoat must not trigger for own-effect deletion"
    );
    assert!(
        replacement
            .2
            .iter()
            .any(|step| matches!(step, CompiledStep::SubstituteReplacement { .. })),
        "Scapegoat should substitute the selected other Digimon"
    );

    let delete_clause = card
        .effects
        .iter()
        .find_map(|clause| match clause {
            CompiledClause::Triggered(triggered)
                if triggered.when.contains(&CompiledTiming::WhenDigivolving)
                    && triggered.when.contains(&CompiledTiming::EndOfOpponentsTurn) =>
            {
                Some(triggered)
            }
            _ => None,
        })
        .expect("lowest-level delete clause should exist");
    assert!(delete_clause.once_per_turn);
    assert!(!delete_clause.optional, "printed delete is mandatory");
}

#[test]
fn ex11_023_does_not_ship_approximating_on_any_deletion_observer() {
    let runner = runner();
    let card = runner
        .compiled_card("EX11-023")
        .expect("EX11-023 must be compiled");

    assert!(
        !card.effects.iter().any(|clause| matches!(
            clause,
            CompiledClause::Triggered(triggered)
                if triggered.when.contains(&CompiledTiming::OnAnyDeletion)
        )),
        "do not ship the trash-play observer until PUPPETS-G011 can exclude \
         Kaguyamon itself from the deleted-object trigger"
    );
}

#[test]
fn ex11_023_alliance_keyword_is_present_while_face_up() {
    let mut runner = runner();
    let kaguyamon = runner.place_on_field(0, "EX11-023", Some(0));

    assert!(runner.game.has_keyword(kaguyamon, Keyword::Alliance));
}

#[test]
fn ex11_023_scapegoat_deletes_another_digimon_and_keeps_kaguyamon() {
    let mut runner = runner();
    let kaguyamon = runner.place_on_field(0, "EX11-023", Some(0));
    runner.place_on_field(0, "OTHER-DIGIMON", Some(0));

    runner
        .game
        .delete_permanent_with_cause(kaguyamon, ReplacementCause::OpponentEffect);

    let accept = runner
        .pending_selection_view()
        .expect("Scapegoat accept prompt");
    assert_eq!(accept.kind, SelectionKind::Replacement);
    assert!(accept.is_optional);
    runner
        .execute_action(0, accept.valid_action_ids[0])
        .expect("accept Scapegoat");

    let pick = runner
        .pending_selection_view()
        .expect("choose another Digimon to delete");
    assert_eq!(pick.kind, SelectionKind::OwnField);
    assert_eq!(pick.valid_action_ids.len(), 1);
    runner
        .execute_action(0, pick.valid_action_ids[0])
        .expect("choose replacement subject");
    runner.auto_resolve().expect("finish Scapegoat");

    let field = field_ids(&runner, 0);
    assert!(field.contains(&"EX11-023".to_string()));
    assert!(!field.contains(&"OTHER-DIGIMON".to_string()));
}

#[test]
fn ex11_023_scapegoat_is_not_offered_for_own_effect_deletion() {
    let mut runner = runner();
    let kaguyamon = runner.place_on_field(0, "EX11-023", Some(0));
    runner.place_on_field(0, "OTHER-DIGIMON", Some(0));

    runner
        .game
        .delete_permanent_with_cause(kaguyamon, ReplacementCause::OwnEffect);

    assert!(
        runner.pending_selection_view().is_none(),
        "own-effect deletion must not offer Scapegoat"
    );
    let field = field_ids(&runner, 0);
    assert!(!field.contains(&"EX11-023".to_string()));
    assert!(field.contains(&"OTHER-DIGIMON".to_string()));
}

#[test]
fn ex11_023_when_digivolving_deletes_one_opponent_lowest_level_digimon() {
    let mut runner = runner();
    let kaguyamon = runner.place_stack(0, &["BASE-L5", "EX11-023"]);
    runner.place_on_field(1, "OPP-L3", Some(0));
    runner.place_on_field(1, "OPP-L4", Some(0));
    runner.place_on_field(1, "OPP-L5", Some(0));

    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(kaguyamon),
    );
    runner.game.drain_effect_queue();

    let target = runner
        .pending_selection_view()
        .expect("lowest-level delete prompt");
    assert_eq!(target.kind, SelectionKind::OppField);
    assert_eq!(
        target.valid_action_ids.len(),
        1,
        "only the lowest-level opponent Digimon should be legal"
    );
    runner
        .execute_action(0, target.valid_action_ids[0])
        .expect("select lowest-level target");
    runner.auto_resolve().expect("finish delete");

    let opp_field = field_ids(&runner, 1);
    assert!(!opp_field.contains(&"OPP-L3".to_string()));
    assert!(opp_field.contains(&"OPP-L4".to_string()));
    assert!(opp_field.contains(&"OPP-L5".to_string()));
}

#[test]
fn ex11_023_end_of_opponents_turn_deletes_one_opponent_lowest_level_digimon() {
    let mut runner = runner();
    let kaguyamon = runner.place_on_field(0, "EX11-023", Some(0));
    runner.place_on_field(1, "OPP-L4", Some(0));
    runner.place_on_field(1, "OPP-L5", Some(0));

    runner.game.enqueue_triggered(
        EffectTiming::EndOfOpponentsTurn,
        TriggerSource::Permanent(kaguyamon),
    );
    runner.game.drain_effect_queue();

    let target = runner
        .pending_selection_view()
        .expect("lowest-level delete prompt");
    assert_eq!(target.kind, SelectionKind::OppField);
    assert_eq!(target.valid_action_ids.len(), 1);
    runner
        .execute_action(0, target.valid_action_ids[0])
        .expect("select lowest-level target");
    runner.auto_resolve().expect("finish delete");

    let opp_field = field_ids(&runner, 1);
    assert!(!opp_field.contains(&"OPP-L4".to_string()));
    assert!(opp_field.contains(&"OPP-L5".to_string()));
}

#[test]
fn ex11_023_lowest_level_delete_allows_choice_among_tied_lowest() {
    let mut runner = runner();
    let kaguyamon = runner.place_on_field(0, "EX11-023", Some(0));
    runner.place_on_field(1, "OPP-L3", Some(0));
    runner.place_on_field(1, "OTHER-DIGIMON", Some(0));
    runner.place_on_field(1, "OPP-L5", Some(0));

    runner.game.enqueue_triggered(
        EffectTiming::EndOfOpponentsTurn,
        TriggerSource::Permanent(kaguyamon),
    );
    runner.game.drain_effect_queue();

    let target = runner
        .pending_selection_view()
        .expect("lowest-level delete prompt");
    assert_eq!(target.kind, SelectionKind::OppField);
    assert_eq!(
        target.valid_action_ids.len(),
        2,
        "both tied level-3 Digimon should be valid choices"
    );
}

#[test]
fn ex11_023_lowest_level_delete_no_opp_digimon_does_not_prompt() {
    let mut runner = runner();
    let kaguyamon = runner.place_on_field(0, "EX11-023", Some(0));

    runner.game.enqueue_triggered(
        EffectTiming::EndOfOpponentsTurn,
        TriggerSource::Permanent(kaguyamon),
    );
    runner.game.drain_effect_queue();

    assert!(
        runner.pending_selection_view().is_none(),
        "no target means no player-visible selection"
    );
}

#[test]
#[ignore = "pending: PUPPETS-G011 — OnAnyDeletion needs deleted-object context plus source-exclusion for 'other Digimon'"]
fn ex11_023_other_deletion_may_play_level_4_or_lower_puppet_from_trash() {
    let mut runner = runner();
    let kaguyamon = runner.place_on_field(0, "EX11-023", Some(0));
    let deleted = runner.place_on_field(0, "OTHER-DIGIMON", Some(0));
    push_to_trash(&mut runner, 0, "TRASH-PUPPET-L4");
    push_to_trash(&mut runner, 0, "TRASH-PUPPET-L5");
    push_to_trash(&mut runner, 0, "TRASH-BEAST-L3");

    runner.game.delete_permanent_with_effects(deleted);
    runner.game.drain_effect_queue();

    let target = runner
        .pending_selection_view()
        .expect("optional trash-play prompt");
    assert_eq!(target.kind, SelectionKind::Trash);
    assert!(target.is_optional);
    assert_eq!(target.valid_action_ids, vec![TRASH_EFFECT_START]);
    runner
        .execute_action(0, target.valid_action_ids[0])
        .expect("play Puppet from trash");
    runner.auto_resolve().expect("finish trash play");

    assert_eq!(count_field_card(&runner, 0, "TRASH-PUPPET-L4"), 1);
    assert!(
        field_ids(&runner, 0).contains(&"EX11-023".to_string()),
        "Kaguyamon remains the source of the observer"
    );
    assert_eq!(kaguyamon.player, 0);
}

#[test]
#[ignore = "pending: PUPPETS-G011 — OnAnyDeletion needs deleted-object context plus source-exclusion for 'other Digimon'"]
fn ex11_023_other_deletion_trash_play_can_be_declined() {
    let mut runner = runner();
    runner.place_on_field(0, "EX11-023", Some(0));
    let deleted = runner.place_on_field(0, "OTHER-DIGIMON", Some(0));
    push_to_trash(&mut runner, 0, "TRASH-PUPPET-L4");

    runner.game.delete_permanent_with_effects(deleted);
    runner.game.drain_effect_queue();

    let target = runner.pending_selection_view().expect("trash prompt");
    assert_eq!(target.kind, SelectionKind::Trash);
    assert!(target.is_optional);
    runner.execute_action(0, PASS).expect("decline trash play");
    runner.auto_resolve().expect("finish decline");

    assert_eq!(count_field_card(&runner, 0, "TRASH-PUPPET-L4"), 0);
    assert!(trash_ids(&runner, 0).contains(&"TRASH-PUPPET-L4".to_string()));
}

#[test]
#[ignore = "pending: PUPPETS-G011 — self-deletion exclusion needs deleted-object/source comparison"]
fn ex11_023_does_not_trigger_trash_play_when_kaguyamon_itself_is_deleted() {
    let mut runner = runner();
    let kaguyamon = runner.place_on_field(0, "EX11-023", Some(0));
    push_to_trash(&mut runner, 0, "TRASH-PUPPET-L4");

    runner.game.delete_permanent_with_effects(kaguyamon);
    runner.game.drain_effect_queue();

    assert!(
        runner.pending_selection_view().is_none(),
        "self-deletion is not 'other Digimon are deleted'"
    );
}

#[test]
#[ignore = "pending: G-OPT-TRIGGERED + PUPPETS-G011 — OPT lockout depends on faithful observer dispatch"]
fn ex11_023_other_deletion_trash_play_is_once_per_turn() {
    let _runner = runner();
}

fn digimon(id: &str, level: u8, dp: i32, traits: &[&str]) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.level = Some(level);
    card.dp = Some(dp);
    card.play_cost = 3;
    card.colors = vec![CardColor::Yellow];
    card.traits = traits
        .iter()
        .map(|trait_name| trait_name.to_string())
        .collect();
    card
}

fn field_ids(runner: &DebugRunner, player: usize) -> Vec<String> {
    runner.game.players[player]
        .battle_area
        .iter()
        .map(|perm| perm.top_card().card_id(&runner.game.card_data).to_string())
        .collect()
}

fn push_to_trash(runner: &mut DebugRunner, player: u8, card_id: &str) {
    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|card| card.card_id == card_id)
        .expect("card data registered");
    let card = CardSource::new(data_idx, player, runner.game.next_card_index());
    runner.game.players[player as usize].trash.push(card);
}

fn count_field_card(runner: &DebugRunner, player: usize, card_id: &str) -> usize {
    runner.game.players[player]
        .battle_area
        .iter()
        .filter(|perm| perm.top_card().card_id(&runner.game.card_data) == card_id)
        .count()
}

fn trash_ids(runner: &DebugRunner, player: usize) -> Vec<String> {
    runner.game.players[player]
        .trash
        .iter()
        .map(|card| card.card_id(&runner.game.card_data).to_string())
        .collect()
}
