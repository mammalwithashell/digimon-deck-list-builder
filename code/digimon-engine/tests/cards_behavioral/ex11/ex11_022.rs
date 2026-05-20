//! EX11-022 Karakurumon — Lv.5, Yellow/Purple.
//!
//! # Card text (cards.json)
//!
//! <Scapegoat> (When this Digimon would be deleted other than by your effects,
//! by deleting 1 of your other Digimon, prevent that deletion.)
//!
//! [On Play] [When Digivolving] You may play 1 [Puppet] trait Digimon card
//! with 4000 DP or less from your hand or trash without paying the cost. At
//! turn end, delete the Digimon this effect played.
//!
//! Inherited: [All Turns] [Once Per Turn] When this Digimon would leave the
//! battle area other than by your effects, by deleting 1 of your Tokens or
//! other [Puppet] trait Digimon, it doesn't leave.
//!
//! # Legacy reference
//!
//! code/engine_py_legacy/engine/data/scripts/ex11/ex11_022.py
//!
//! # Patterns
//!
//! - H15 Scapegoat replacement
//! - Replacement with selected cost body and cancel/substitute outcome
//! - Hand/trash effect-play cleanup gap coverage

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledCardKind, CompiledClause, CompiledColor, CompiledCost,
    CompiledDeclarativeClause, CompiledReplacementCause, CompiledScope, CompiledStep,
};
use digimon_engine::action::space::{PLAY_HAND_START, TRASH_EFFECT_START};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::CardKind;
use digimon_engine::replacement::ReplacementCause;
use digimon_engine::selection::{SelectionKind, UnionZoneSet};

fn digimon(id: &str, level: u8, dp: i32, traits: &[&str]) -> digimon_engine::card_data::CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.level = Some(level);
    card.dp = Some(dp);
    card.traits = traits
        .iter()
        .map(|trait_name| trait_name.to_string())
        .collect();
    card
}

fn token(id: &str) -> digimon_engine::card_data::CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Token;
    card.dp = Some(3000);
    card
}

fn card_ids_on_field(runner: &DebugRunner, player: usize) -> Vec<String> {
    runner.game.players[player]
        .battle_area
        .iter()
        .map(|perm| perm.top_card().card_id(&runner.game.card_data).to_string())
        .collect()
}

fn load_runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("EX11-022")
        .expect("EX11-022 YAML loads")
        .add_card(digimon("CARRIER", 5, 7000, &[]))
        .add_card(digimon("OTHER-DIGIMON", 3, 2000, &[]))
        .add_card(digimon("PUPPET-COST", 4, 4000, &["Puppet"]))
        .add_card(digimon("NON-PUPPET", 4, 4000, &["Beast"]))
        .add_card(token("TOKEN-COST"))
        .start()
}

#[test]
fn ex11_022_has_printed_metadata_and_evolution_paths() {
    let runner = load_runner();
    let compiled = runner
        .compiled_card("EX11-022")
        .expect("EX11-022 must be compiled");

    assert_eq!(compiled.name, "Karakurumon");
    assert_eq!(compiled.kind, CompiledCardKind::Digimon);
    assert_eq!(compiled.level, Some(5));
    assert_eq!(compiled.cost, Some(7));
    assert_eq!(compiled.dp, Some(7000));
    assert_eq!(
        compiled.color,
        vec![CompiledColor::Yellow, CompiledColor::Purple]
    );
    assert!(compiled
        .traits
        .iter()
        .any(|trait_name| trait_name == "Puppet"));
    assert!(compiled
        .traits
        .iter()
        .any(|trait_name| trait_name == "LIBERATOR"));
    assert_eq!(compiled.attribute.as_deref(), Some("Data"));

    assert!(
        compiled.alt_paths.iter().any(|path| {
            path.kind == CompiledAltPathKind::Digivolve
                && matches!(path.cost, Some(CompiledCost::Literal(4)))
                && path.from.as_ref().is_some_and(|from| {
                    from.level_eq == Some(4) && from.color_is == Some(CompiledColor::Yellow)
                })
        }),
        "printed yellow Lv.4 digivolution should cost 4"
    );
    assert!(
        compiled.alt_paths.iter().any(|path| {
            path.kind == CompiledAltPathKind::Digivolve
                && matches!(path.cost, Some(CompiledCost::Literal(2)))
                && path.from.as_ref().is_some_and(|from| {
                    from.level_eq == Some(3) && from.trait_has.as_deref() == Some("Puppet")
                })
        }),
        "alternate Lv.3 Puppet digivolution should cost 2"
    );
}

#[test]
fn ex11_022_scape_goat_replacement_is_optional_and_not_own_effect() {
    let runner = load_runner();
    let compiled = runner
        .compiled_card("EX11-022")
        .expect("EX11-022 must be compiled");

    let replacement = compiled
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
        "Scapegoat should substitute the selected other Digimon as the deletion subject"
    );
}

#[test]
fn ex11_022_scape_goat_deletes_another_digimon_and_keeps_karakurumon() {
    let mut runner = load_runner();
    let kara = runner.place_on_field(0, "EX11-022", Some(0));
    runner.place_on_field(0, "OTHER-DIGIMON", Some(0));

    runner
        .game
        .delete_permanent_with_cause(kara, ReplacementCause::OpponentEffect);

    let accept = runner
        .pending_selection_view()
        .expect("Scapegoat accept prompt");
    assert_eq!(accept.kind, SelectionKind::Replacement);
    assert!(accept.is_optional, "Scapegoat may be declined");
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
        .expect("choose substitute Digimon");
    runner.auto_resolve().expect("finish Scapegoat");

    let field = card_ids_on_field(&runner, 0);
    assert!(field.contains(&"EX11-022".to_string()));
    assert!(!field.contains(&"OTHER-DIGIMON".to_string()));
}

#[test]
fn ex11_022_scape_goat_is_not_offered_for_own_effect_deletion() {
    let mut runner = load_runner();
    let kara = runner.place_on_field(0, "EX11-022", Some(0));
    runner.place_on_field(0, "OTHER-DIGIMON", Some(0));

    runner
        .game
        .delete_permanent_with_cause(kara, ReplacementCause::OwnEffect);

    assert!(
        runner.pending_selection_view().is_none(),
        "own-effect deletion must not install Scapegoat"
    );
    let field = card_ids_on_field(&runner, 0);
    assert!(!field.contains(&"EX11-022".to_string()));
    assert!(field.contains(&"OTHER-DIGIMON".to_string()));
}

#[test]
fn ex11_022_inherited_leave_prevention_uses_token_or_other_puppet_and_is_once_per_turn() {
    let mut runner = load_runner();
    let carrier = runner.place_stack(0, &["EX11-022", "CARRIER"]);
    runner.place_on_field(0, "TOKEN-COST", Some(0));
    runner.place_on_field(0, "PUPPET-COST", Some(0));
    runner.place_on_field(0, "NON-PUPPET", Some(0));

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

    let cost = runner
        .pending_selection_view()
        .expect("select Token or other Puppet cost");
    assert_eq!(cost.kind, SelectionKind::OwnField);
    assert_eq!(
        cost.valid_action_ids.len(),
        2,
        "only the Token and other Puppet are valid costs"
    );
    runner
        .execute_action(0, cost.valid_action_ids[0])
        .expect("pay replacement cost");
    runner
        .auto_resolve()
        .expect("finish inherited leave prevention");

    let field = card_ids_on_field(&runner, 0);
    assert!(field.contains(&"CARRIER".to_string()));
    assert!(field.contains(&"NON-PUPPET".to_string()));

    let carrier_after_first = runner.game.players[0]
        .battle_area
        .iter()
        .position(|perm| perm.top_card().card_id(&runner.game.card_data) == "CARRIER")
        .expect("carrier survived first leave event");
    let carrier_handle = runner.perm_handle(0, carrier_after_first);

    runner
        .game
        .delete_permanent_with_cause(carrier_handle, ReplacementCause::OpponentEffect);
    assert!(
        runner.pending_selection_view().is_none(),
        "once-per-turn replacement should not be offered a second time"
    );
    let field = card_ids_on_field(&runner, 0);
    assert!(
        !field.contains(&"CARRIER".to_string()),
        "second leave event is not prevented"
    );
}

/// PUPPETS-G021 (Task 6): union-zone DP filter for hidden-zone candidates.
///
/// The selection must offer ONLY Puppet Digimon cards with DP ≤ 4000 from
/// both hand and trash. Non-Puppet cards and Puppet cards above 4000 DP must
/// be absent from `valid_action_ids`. The play consumer and turn-end cleanup
/// are deferred to later tasks (PUPPETS-G014 / PUPPETS-G003, Tasks 7/8).
#[test]
fn ex11_022_on_play_union_selection_filter_excludes_non_puppet_and_high_dp() {
    // Cards in hand (after EX11-022 is played from index 0):
    //   index 0: PUPPET-HAND   — Puppet, DP 3000  → should be OFFERED   (action PLAY_HAND_START+0)
    //   index 1: PUPPET-HIGH   — Puppet, DP 5000  → should be EXCLUDED  (DP > 4000)
    //   index 2: NON-PUPPET    — Beast,  DP 2000  → should be EXCLUDED  (not Puppet)
    // Cards in trash:
    //   index 0: PUPPET-TRASH  — Puppet, DP 4000  → should be OFFERED   (action TRASH_EFFECT_START+0)
    //   index 1: PUPPET-HIGH-T — Puppet, DP 6000  → should be EXCLUDED  (DP > 4000)
    let mut runner = DebugRunner::builder()
        .dsl_card("EX11-022")
        .expect("EX11-022 YAML loads")
        .add_card(digimon("PUPPET-HAND", 4, 3000, &["Puppet"]))
        .add_card(digimon("PUPPET-HIGH", 4, 5000, &["Puppet"]))
        .add_card(digimon("NON-PUPPET", 4, 2000, &["Beast"]))
        .add_card(digimon("PUPPET-TRASH", 4, 4000, &["Puppet"]))
        .add_card(digimon("PUPPET-HIGH-T", 5, 6000, &["Puppet"]))
        .hand(0, &["EX11-022", "PUPPET-HAND", "PUPPET-HIGH", "NON-PUPPET"])
        .memory(10)
        .start();

    // Push cards to player 0's trash.
    let puppet_trash_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == "PUPPET-TRASH")
        .expect("PUPPET-TRASH in registry");
    let puppet_high_t_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == "PUPPET-HIGH-T")
        .expect("PUPPET-HIGH-T in registry");
    use digimon_engine::card_source::CardSource;
    let ci0 = runner.game.next_card_index();
    runner.game.players[0]
        .trash
        .push(CardSource::new(puppet_trash_idx, 0, ci0));
    let ci1 = runner.game.next_card_index();
    runner.game.players[0]
        .trash
        .push(CardSource::new(puppet_high_t_idx, 0, ci1));

    // Play EX11-022 from hand index 0. This triggers the On Play effect.
    runner.play(0, 0).expect("EX11-022 plays from hand");

    let view = runner
        .pending_selection_view()
        .expect("union zone selection must be installed by On Play");

    assert_eq!(
        view.kind,
        SelectionKind::UnionZone {
            zones: UnionZoneSet::HAND | UnionZoneSet::TRASH
        },
        "pending selection must be a union-zone prompt"
    );
    assert!(view.is_optional, "the selection must be optional ('you may')");

    // Exactly 2 eligible candidates: PUPPET-HAND (hand[0]) + PUPPET-TRASH (trash[0]).
    assert_eq!(
        view.valid_action_ids.len(),
        2,
        "only PUPPET-HAND and PUPPET-TRASH should be eligible; got {:?}",
        view.valid_action_ids
    );

    // Hand index 0 maps to PLAY_HAND_START + 0.
    assert!(
        view.valid_action_ids.contains(&(PLAY_HAND_START + 0)),
        "PUPPET-HAND (hand index 0, DP 3000, Puppet) must be offered"
    );
    // Trash index 0 maps to TRASH_EFFECT_START + 0.
    assert!(
        view.valid_action_ids.contains(&(TRASH_EFFECT_START + 0)),
        "PUPPET-TRASH (trash index 0, DP 4000, Puppet) must be offered"
    );

    // Verify the excluded cards are absent.
    assert!(
        !view.valid_action_ids.contains(&(PLAY_HAND_START + 1)),
        "PUPPET-HIGH (DP 5000) must be excluded"
    );
    assert!(
        !view.valid_action_ids.contains(&(PLAY_HAND_START + 2)),
        "NON-PUPPET (Beast trait) must be excluded"
    );
    assert!(
        !view.valid_action_ids.contains(&(TRASH_EFFECT_START + 1)),
        "PUPPET-HIGH-T (DP 6000) must be excluded"
    );
}

#[test]
#[ignore = "pending: PUPPETS-G014 (Task 7) + PUPPETS-G003 (Task 7) - play consumer and turn-end cleanup are required"]
fn ex11_022_on_play_selected_puppet_is_played_free_and_deleted_at_turn_end() {
    let _runner = load_runner();
    // Required behavior:
    // - the selected card is played from its original zone without cost;
    // - at turn end, the exact played permanent is deleted even if field
    //   indices shift before cleanup.
}

#[test]
#[ignore = "pending: PUPPETS-G014 + PUPPETS-G003 - same blocked body as On Play must fire on When Digivolving"]
fn ex11_022_when_digivolving_uses_same_optional_play_and_cleanup_body() {
    let _runner = load_runner();
}
