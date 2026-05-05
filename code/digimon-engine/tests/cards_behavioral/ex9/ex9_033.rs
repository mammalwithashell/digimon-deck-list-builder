//! EX9-033 Kaguyamon — Lv.6, Yellow/Purple, Puppet/LIBERATOR.
//!
//! # Card text (cards.json)
//!
//! [All Turns] All of your Tokens and [Puppet] trait Digimon gain <Alliance>
//! and <Blocker>.
//!
//! [All Turns] [Once Per Turn] When other Digimon are deleted, delete 1 of
//! your opponent's lowest level Digimon.
//!
//! [End of Your Turn] [Once Per Turn] You may play 1 level 4 or lower
//! [Puppet] trait Digimon card from your trash without paying the cost.
//!
//! Alternate requirement: [Digivolve] Lv.5 w/[Puppet] trait: Cost 3.
//!
//! # Legacy reference
//! code/engine_py_legacy/engine/data/scripts/ex9/ex9_033.py
//!
//! # Patterns
//! - D4/H5/H10: continuous aura granting Blocker and Alliance
//! - Event observer: on_any_deletion with deleted-object "other Digimon" gate
//! - Aggregate targeting: opponent's lowest-level Digimon
//! - E2: optional EndOfYourTurn trash play

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledCost, CompiledDeclarativeClause, CompiledTiming,
};
use digimon_engine::action::space::{PASS, TRASH_EFFECT_START};
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming, Keyword};
use digimon_engine::selection::{SelectionKind, TriggerSource};

fn runner_with_kaguyamon() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("EX9-033")
        .expect("EX9-033 YAML loads")
        .add_card(make_digimon("ALLY-PUPPET", 4, &["Puppet"]))
        .add_card(make_digimon("ALLY-BEAST", 4, &["Beast"]))
        .add_card(make_token("ALLY-TOKEN"))
        .add_card(make_digimon("TRASH-PUPPET-L4", 4, &["Puppet"]))
        .add_card(make_digimon("TRASH-PUPPET-L5", 5, &["Puppet"]))
        .add_card(make_digimon("TRASH-BEAST-L3", 3, &["Beast"]))
        .memory(20)
        .start()
}

#[test]
fn ex9_033_has_printed_metadata_and_digivolution_paths() {
    let runner = runner_with_kaguyamon();
    let card = runner
        .compiled_card("EX9-033")
        .expect("EX9-033 compiled card present");

    assert_eq!(card.name, "Kaguyamon");
    assert_eq!(card.kind, digimon_dsl::compiled::CompiledCardKind::Digimon);
    assert_eq!(card.level, Some(6));
    assert_eq!(card.cost, Some(12));
    assert_eq!(card.dp, Some(12000));
    assert!(card
        .color
        .contains(&digimon_dsl::compiled::CompiledColor::Yellow));
    assert!(card
        .color
        .contains(&digimon_dsl::compiled::CompiledColor::Purple));
    assert!(card.traits.iter().any(|t| t == "Puppet"));
    assert!(card.traits.iter().any(|t| t == "LIBERATOR"));

    assert!(
        card.alt_paths.iter().any(|path| {
            path.kind == CompiledAltPathKind::Digivolve
                && path.cost == Some(CompiledCost::Literal(4))
        }),
        "standard yellow Lv.5 digivolution for cost 4 must be present"
    );
    assert!(
        card.alt_paths.iter().any(|path| {
            path.kind == CompiledAltPathKind::Digivolve
                && path.cost == Some(CompiledCost::Literal(3))
        }),
        "alternate Lv.5 Puppet digivolution for cost 3 must be present"
    );
}

#[test]
fn ex9_033_has_two_keyword_auras_and_eot_trash_play_clause() {
    let runner = runner_with_kaguyamon();
    let card = runner
        .compiled_card("EX9-033")
        .expect("EX9-033 compiled card present");

    let aura_keywords: Vec<_> = card
        .effects
        .iter()
        .filter_map(|clause| match clause {
            CompiledClause::Declarative(CompiledDeclarativeClause::Aura {
                grant_keyword: Some(keyword),
                ..
            }) => Some(keyword.keyword.as_str()),
            _ => None,
        })
        .collect();
    assert!(aura_keywords.contains(&"Alliance"));
    assert!(aura_keywords.contains(&"Blocker"));

    let eot = card.effects.iter().find_map(|clause| match clause {
        CompiledClause::Triggered(triggered)
            if triggered.when.contains(&CompiledTiming::EndOfYourTurn) =>
        {
            Some(triggered)
        }
        _ => None,
    });
    let eot = eot.expect("End of Your Turn trash-play clause exists");
    assert!(eot.optional, "printed 'You may play' must be optional");
    assert!(
        eot.once_per_turn,
        "printed [Once Per Turn] must be retained"
    );
}

#[test]
fn ex9_033_deletion_observer_is_omitted_pending_deleted_event_context() {
    let runner = runner_with_kaguyamon();
    let card = runner
        .compiled_card("EX9-033")
        .expect("EX9-033 compiled card present");

    assert!(
        !card.effects.iter().any(|clause| {
            matches!(
                clause,
                CompiledClause::Triggered(triggered)
                    if triggered.when.contains(&CompiledTiming::OnAnyDeletion)
            )
        }),
        "do not ship an approximating OnAnyDeletion clause until the deleted \
         Digimon can be distinguished from EX9-033 itself"
    );
}

#[test]
fn ex9_033_aura_grants_alliance_and_blocker_to_own_puppet_and_token_only() {
    let mut runner = runner_with_kaguyamon();
    runner.place_on_field(0, "EX9-033", None);
    let puppet = runner.place_on_field(0, "ALLY-PUPPET", None);
    let token = runner.place_on_field(0, "ALLY-TOKEN", None);
    let non_puppet = runner.place_on_field(0, "ALLY-BEAST", None);
    let opponent_puppet = runner.place_on_field(1, "ALLY-PUPPET", None);

    runner.game.tick_declarative_effects();

    for handle in [puppet, token] {
        assert!(
            runner.game.has_keyword(handle, Keyword::Alliance),
            "own Puppet Digimon and Tokens gain Alliance"
        );
        assert!(
            runner.game.has_keyword(handle, Keyword::Blocker),
            "own Puppet Digimon and Tokens gain Blocker"
        );
    }
    for handle in [non_puppet, opponent_puppet] {
        assert!(
            !runner.game.has_keyword(handle, Keyword::Alliance),
            "non-Puppet own Digimon and opponent Digimon do not gain Alliance"
        );
        assert!(
            !runner.game.has_keyword(handle, Keyword::Blocker),
            "non-Puppet own Digimon and opponent Digimon do not gain Blocker"
        );
    }
}

#[test]
fn ex9_033_eot_may_play_level_4_or_lower_puppet_from_trash() {
    let mut runner = runner_with_kaguyamon();
    let kaguyamon = runner.place_on_field(0, "EX9-033", None);
    push_to_trash(&mut runner, 0, "TRASH-PUPPET-L4");
    push_to_trash(&mut runner, 0, "TRASH-PUPPET-L5");
    push_to_trash(&mut runner, 0, "TRASH-BEAST-L3");

    runner.game.enqueue_triggered(
        EffectTiming::EndOfYourTurn,
        TriggerSource::Permanent(kaguyamon),
    );
    runner.game.drain_effect_queue();

    let view = runner
        .pending_selection_view()
        .expect("EOT trash-play prompt should install");
    assert_eq!(view.kind, SelectionKind::Trash);
    assert!(
        runner.pending_is_optional(),
        "printed 'You may' is declinable"
    );
    assert_eq!(
        view.valid_action_ids,
        vec![TRASH_EFFECT_START],
        "only the level 4 Puppet in trash is a legal play target"
    );

    runner
        .execute_action(0, view.valid_action_ids[0])
        .expect("play level 4 Puppet from trash");
    runner.auto_resolve().expect("finish EOT trash play");

    assert_eq!(
        count_field_card(&runner, 0, "TRASH-PUPPET-L4"),
        1,
        "selected level 4 Puppet is played for free"
    );
    assert!(
        trash_ids(&runner, 0).contains(&"TRASH-PUPPET-L5".to_string()),
        "level 5 Puppet remains in trash"
    );
    assert!(
        trash_ids(&runner, 0).contains(&"TRASH-BEAST-L3".to_string()),
        "non-Puppet level 3 remains in trash"
    );
}

#[test]
fn ex9_033_eot_trash_play_can_be_declined() {
    let mut runner = runner_with_kaguyamon();
    let kaguyamon = runner.place_on_field(0, "EX9-033", None);
    push_to_trash(&mut runner, 0, "TRASH-PUPPET-L4");

    runner.game.enqueue_triggered(
        EffectTiming::EndOfYourTurn,
        TriggerSource::Permanent(kaguyamon),
    );
    runner.game.drain_effect_queue();

    assert_eq!(runner.pending_kind(), Some(SelectionKind::Trash));
    assert!(runner.pending_is_optional(), "trash play is optional");
    runner
        .execute_action(0, PASS)
        .expect("decline EOT trash play");
    runner.auto_resolve().expect("finish decline");

    assert_eq!(count_field_card(&runner, 0, "TRASH-PUPPET-L4"), 0);
    assert!(
        trash_ids(&runner, 0).contains(&"TRASH-PUPPET-L4".to_string()),
        "declined card stays in trash"
    );
}

#[test]
#[ignore = "pending: G-ON-ANY-DELETION-EVENT-CONTEXT — OnAnyDeletion lacks deleted permanent/card TriggerContext for 'other Digimon'"]
fn ex9_033_deletes_one_opponent_lowest_level_digimon_when_other_digimon_deleted() {
    todo!("unignore once OnAnyDeletion carries the deleted permanent/card");
}

#[test]
#[ignore = "pending: G-ON-ANY-DELETION-EVENT-CONTEXT — self-deletion exclusion needs deleted-object context"]
fn ex9_033_does_not_trigger_when_kaguyamon_itself_is_deleted() {
    todo!("unignore once OnAnyDeletion can distinguish EX9-033 from the deleted object");
}

#[test]
#[ignore = "pending: G-ON-ANY-DELETION-EVENT-CONTEXT — OPT coverage depends on faithful deleted-object gating"]
fn ex9_033_deletion_observer_is_once_per_turn() {
    todo!("unignore once the OnAnyDeletion clause can be authored faithfully");
}

fn make_digimon(id: &str, level: u8, traits: &[&str]) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.level = Some(level);
    card.dp = Some(3000);
    card.play_cost = 3;
    card.colors = vec![CardColor::Yellow];
    card.traits = traits
        .iter()
        .map(|trait_name| trait_name.to_string())
        .collect();
    card
}

fn make_token(id: &str) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Token;
    card.level = None;
    card.dp = Some(3000);
    card.play_cost = 0;
    card.colors = vec![CardColor::White];
    card
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
