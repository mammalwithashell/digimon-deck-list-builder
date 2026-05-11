//! P-194 Aegiomon — Digimon, Lv.4, Yellow, Cost 4, DP 4000.
//!
//! # Card text (cards/P/P-194.json)
//!
//! <Blocker>. <Barrier> (When this Digimon would be deleted in battle, by
//! trashing the top card of your security stack, prevent that deletion.)
//!
//! Inherited: <Barrier> (When this Digimon would be deleted in battle, by
//! trashing the top card of your security stack, prevent that deletion.)
//!
//! # Patterns
//! - H5 Blocker printed keyword
//! - H14 Barrier printed keyword, face-up and inherited

use std::path::Path;

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledCardKind, CompiledClause, CompiledColor, CompiledCost,
    CompiledDeclarativeClause, CompiledScope,
};
use digimon_engine::action::space::REPLACEMENT_ACCEPT;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::Keyword;
use digimon_engine::replacement::ReplacementCause;
use digimon_engine::selection::SelectionKind;

const CARD_ID: &str = "P-194";

fn p_194_yaml() -> String {
    std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("cards/p/P-194.yaml"))
        .expect("P-194 YAML must exist at cards/p/P-194.yaml")
}

fn p_194_runner() -> DebugRunner {
    let yaml = p_194_yaml();
    DebugRunner::builder()
        .from_dsl_yaml(&yaml)
        .expect("P-194 YAML must parse and compile")
        .start()
}

fn grant_keywords(runner: &DebugRunner) -> Vec<(CompiledScope, String)> {
    runner
        .compiled_card(CARD_ID)
        .expect("P-194 compiled")
        .effects
        .iter()
        .filter_map(|clause| match clause {
            CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword {
                scope,
                keyword,
                ..
            }) => Some((*scope, keyword.clone())),
            _ => None,
        })
        .collect()
}

#[test]
fn p_194_has_printed_metadata_and_yellow_lv3_evo_path() {
    let runner = p_194_runner();
    let compiled = runner.compiled_card(CARD_ID).expect("P-194 compiled");

    assert_eq!(compiled.name, "Aegiomon");
    assert_eq!(compiled.kind, CompiledCardKind::Digimon);
    assert_eq!(compiled.level, Some(4));
    assert_eq!(compiled.color, vec![CompiledColor::Yellow]);
    assert_eq!(compiled.cost, Some(4));
    assert_eq!(compiled.dp, Some(4000));
    assert_eq!(compiled.traits, vec!["Shaman".to_string(), "TS".to_string()]);

    let has_yellow_lv3_cost_2 = compiled.alt_paths.iter().any(|path| {
        path.kind == CompiledAltPathKind::Digivolve
            && path.cost == Some(CompiledCost::Literal(2))
            && path
                .from
                .as_ref()
                .is_some_and(|predicate| {
                    predicate.level_eq == Some(3)
                        && predicate.color_is == Some(CompiledColor::Yellow)
                })
    });
    assert!(
        has_yellow_lv3_cost_2,
        "P-194 must digivolve from yellow level 3 for cost 2"
    );
    let has_ts_lv3_cost_2 = compiled.alt_paths.iter().any(|path| {
        path.kind == CompiledAltPathKind::Digivolve
            && path.cost == Some(CompiledCost::Literal(2))
            && path.from.as_ref().is_some_and(|predicate| {
                predicate.all_of.iter().any(|p| p.level_eq == Some(3))
                    && predicate
                        .all_of
                        .iter()
                        .any(|p| p.trait_has.as_deref() == Some("TS"))
            })
    });
    assert!(
        has_ts_lv3_cost_2,
        "P-194 must also digivolve from level 3 with [TS] trait for cost 2"
    );
}

#[test]
fn p_194_declares_face_blocker_face_barrier_and_inherited_barrier() {
    let runner = p_194_runner();
    let grants = grant_keywords(&runner);

    assert!(
        grants
            .iter()
            .any(|(scope, keyword)| *scope == CompiledScope::FaceUp
                && keyword.eq_ignore_ascii_case("Blocker")),
        "P-194 must grant face-up <Blocker>"
    );
    assert!(
        grants
            .iter()
            .any(|(scope, keyword)| *scope == CompiledScope::FaceUp
                && keyword.eq_ignore_ascii_case("Barrier")),
        "P-194 must grant face-up <Barrier>"
    );
    assert!(
        grants
            .iter()
            .any(|(scope, keyword)| *scope == CompiledScope::Inherited
                && keyword.eq_ignore_ascii_case("Barrier")),
        "P-194 must grant inherited <Barrier>"
    );
}

#[test]
fn p_194_face_keywords_are_available_on_field() {
    let mut runner = p_194_runner();
    let aegiomon = runner.place_on_field(0, CARD_ID, Some(0));

    assert!(
        runner.game.has_keyword(aegiomon, Keyword::Blocker),
        "Aegiomon has face-up <Blocker>"
    );
    assert!(
        runner.game.has_keyword(aegiomon, Keyword::Barrier),
        "Aegiomon has face-up <Barrier>"
    );
}

#[test]
fn p_194_inherited_barrier_is_available_from_stack() {
    let yaml = p_194_yaml();
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(&yaml)
        .expect("P-194 YAML parses")
        .add_card(make_test_card("CARRIER", "Carrier"))
        .start();
    let carrier = runner.place_stack(0, &[CARD_ID, "CARRIER"]);

    assert!(
        runner.game.has_keyword(carrier, Keyword::Barrier),
        "carrier inherits <Barrier> from P-194"
    );
}

#[test]
fn p_194_face_barrier_trashes_top_security_to_prevent_battle_deletion() {
    let yaml = p_194_yaml();
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(&yaml)
        .expect("P-194 YAML parses")
        .add_card(make_test_card("SEC", "Security Filler"))
        .security(0, &["SEC", "SEC"])
        .start();
    let aegiomon = runner.place_on_field(0, CARD_ID, Some(0));

    runner
        .game
        .delete_permanent_with_cause(aegiomon, ReplacementCause::Battle);

    let view = runner
        .pending_selection_view()
        .expect("Barrier replacement prompt");
    assert_eq!(view.kind, SelectionKind::Replacement);
    assert!(view.is_optional, "Barrier may be declined");

    runner
        .execute_action(0, REPLACEMENT_ACCEPT)
        .expect("accept Barrier replacement");
    runner.auto_resolve().expect("finish Barrier replacement");

    assert_eq!(
        runner.battle_area_size(0),
        1,
        "Barrier prevents Aegiomon from being deleted in battle"
    );
    assert_eq!(
        runner.security_count(0),
        1,
        "Barrier trashes the top security card as its cost"
    );
    assert_eq!(runner.trash_size(0), 1, "trashed security lands in trash");
}

#[test]
fn p_194_inherited_barrier_trashes_top_security_to_prevent_battle_deletion() {
    let yaml = p_194_yaml();
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(&yaml)
        .expect("P-194 YAML parses")
        .add_card(make_test_card("CARRIER", "Carrier"))
        .add_card(make_test_card("SEC", "Security Filler"))
        .security(0, &["SEC", "SEC"])
        .start();
    let carrier = runner.place_stack(0, &[CARD_ID, "CARRIER"]);

    runner
        .game
        .delete_permanent_with_cause(carrier, ReplacementCause::Battle);

    let view = runner
        .pending_selection_view()
        .expect("inherited Barrier replacement prompt");
    assert_eq!(view.kind, SelectionKind::Replacement);
    assert!(view.is_optional, "Barrier may be declined");

    runner
        .execute_action(0, REPLACEMENT_ACCEPT)
        .expect("accept inherited Barrier replacement");
    runner
        .auto_resolve()
        .expect("finish inherited Barrier replacement");

    assert_eq!(
        runner.battle_area_size(0),
        1,
        "inherited Barrier prevents the carrier's battle deletion"
    );
    assert_eq!(
        runner.security_count(0),
        1,
        "inherited Barrier trashes top security"
    );
}
