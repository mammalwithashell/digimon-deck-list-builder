//! EX9-008 Biyomon - Lv.3 Red Digimon, DP 1000, play cost 3.
//!
//! # Card text (data/card_meta/ex9/EX9-008.md)
//!
//! Traits: Bird, DM, Ver.4.
//!
//! Alt path:
//!   [Digivolve] Lv.2 with [DM] trait: cost 0.
//!
//! Effect:
//!   <Training> (In the main phase, by suspending this Digimon, place your
//!   deck's top card face down as this Digimon's bottom digivolution card.
//!   This effect can also activate in the breeding area.)
//!
//! Inherited Effect:
//!   <Raid> (When this Digimon attacks, you may switch the target of attack
//!   to 1 of your opponent's unsuspended Digimon with the highest DP.)
//!
//! # Patterns this test covers
//! - H9 Raid: inherited keyword grant
//! - Phase F Training: native/printed keyword grant backed by keyword auto-install
//! - D4 Declarative keyword grants

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledCost, CompiledDeclarativeClause, CompiledScope,
};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::Keyword;

fn biyomon_runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("EX9-008")
        .expect("EX9-008 must be present in the embedded DSL pack")
        .memory(10)
        .build()
}

#[test]
fn ex9_008_yaml_loads_and_compiles_from_embedded_pack() {
    let runner = biyomon_runner();
    let compiled = runner
        .compiled_card("EX9-008")
        .expect("EX9-008 compiled card must be available");

    assert_eq!(compiled.card, "EX9-008");
    assert_eq!(compiled.name, "Biyomon");
    assert_eq!(compiled.level, Some(3));
    assert_eq!(compiled.dp, Some(1000));
}

#[test]
fn ex9_008_alt_path_contains_lv2_dm_trait_cost_zero() {
    let runner = biyomon_runner();
    let compiled = runner
        .compiled_card("EX9-008")
        .expect("EX9-008 compiled card must be available");

    let dm_lv2_path = compiled.alt_paths.iter().find(|path| {
        path.kind == CompiledAltPathKind::Digivolve
            && path.cost == Some(CompiledCost::Literal(0))
            && path
                .from
                .as_ref()
                .map(|from| from.level_eq == Some(2) && from.trait_has.as_deref() == Some("DM"))
                .unwrap_or(false)
    });

    assert!(
        dm_lv2_path.is_some(),
        "EX9-008 must declare [Digivolve] Lv.2 with [DM] trait: cost 0"
    );
}

#[test]
fn ex9_008_declares_training_and_inherited_raid_keyword_grants() {
    let runner = biyomon_runner();
    let compiled = runner
        .compiled_card("EX9-008")
        .expect("EX9-008 compiled card must be available");

    let has_face_up_training = compiled.effects.iter().any(|clause| {
        matches!(
            clause,
            CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword {
                keyword,
                scope: CompiledScope::FaceUp,
                ..
            }) if keyword == "Training"
        )
    });
    let has_inherited_raid = compiled.effects.iter().any(|clause| {
        matches!(
            clause,
            CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword {
                keyword,
                scope: CompiledScope::Inherited,
                ..
            }) if keyword == "Raid"
        )
    });

    assert!(
        has_face_up_training,
        "EX9-008 must declare own-scope GrantKeyword(Training)"
    );
    assert!(
        has_inherited_raid,
        "EX9-008 must declare inherited-scope GrantKeyword(Raid)"
    );
}

#[test]
fn ex9_008_training_keyword_is_declared_and_uses_native_visibility_when_available() {
    let mut runner = biyomon_runner();
    let compiled = runner
        .compiled_card("EX9-008")
        .expect("EX9-008 compiled card must be available");

    let has_declared_training = compiled.effects.iter().any(|clause| {
        matches!(
            clause,
            CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword {
                keyword,
                scope: CompiledScope::FaceUp,
                ..
            }) if keyword == "Training"
        )
    });
    assert!(
        has_declared_training,
        "EX9-008 must visibly declare GrantKeyword(Training) in compiled DSL"
    );

    let card_data = runner
        .game
        .card_data
        .iter()
        .find(|card| card.card_id == "EX9-008")
        .expect("EX9-008 card data must be loaded");

    if card_data.keywords.contains(&Keyword::Training) {
        let biyomon = runner.place_on_field(0, "EX9-008", Some(0));
        assert!(
            runner.game.has_keyword(biyomon, Keyword::Training),
            "if current engine maps DSL Training into native card data, EX9-008 on field must report Keyword::Training"
        );
    }
}

#[test]
fn ex9_008_inherited_raid_is_visible_under_a_carrier() {
    let mut carrier = make_test_card("CARRIER-LV4", "CarrierLv4");
    carrier.level = Some(4);
    carrier.dp = Some(5000);

    let mut runner = DebugRunner::builder()
        .dsl_card("EX9-008")
        .expect("EX9-008 must be present in the embedded DSL pack")
        .add_card(carrier)
        .memory(10)
        .build();

    let carrier_handle = runner.place_stack(0, &["EX9-008", "CARRIER-LV4"]);

    assert!(
        runner.game.has_keyword(carrier_handle, Keyword::Raid),
        "a carrier with EX9-008 as a digivolution source must inherit Raid"
    );
}

#[test]
fn ex9_008_inherited_raid_is_absent_without_source() {
    let mut carrier = make_test_card("CARRIER-LV4", "CarrierLv4");
    carrier.level = Some(4);
    carrier.dp = Some(5000);

    let mut runner = DebugRunner::builder()
        .dsl_card("EX9-008")
        .expect("EX9-008 must be present in the embedded DSL pack")
        .add_card(carrier)
        .memory(10)
        .build();

    let carrier_handle = runner.place_on_field(0, "CARRIER-LV4", Some(0));

    assert!(
        !runner.game.has_keyword(carrier_handle, Keyword::Raid),
        "a carrier without EX9-008 in sources must not gain Raid"
    );
}
