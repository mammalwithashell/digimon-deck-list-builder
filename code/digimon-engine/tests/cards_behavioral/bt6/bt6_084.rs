//! BT6-084 Sistermon Ciel - Digimon, Lv.4, White.
//!
//! # Card text (cards.json)
//!
//! [All Turns] All of your Digimon with [Huckmon] in their name or [Royal Knight] trait get +2000 DP.
//! [On Play] Gain 1 memory.
//!
//! `inherited_effect_description_eng` duplicates "Card Effect(s)" for this card
//! in the ingested data. The printed text is not an inherited effect, so these
//! tests require only face-up effects.
//!
//! # Patterns
//! - D4 declarative aura: own battle-area Digimon filtered by name OR trait
//! - On Play memory gain with no player-visible choice

use digimon_dsl::compiled::{
    CompiledCardKind, CompiledClause, CompiledColor, CompiledDeclarativeClause, CompiledPlayerRef,
    CompiledScope, CompiledZone,
};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, ModifierType};

fn sistermon_runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("BT6-084")
        .expect("BT6-084 YAML loads")
        .add_card(make_digimon("ALLY-HUCKMON", "BaoHuckmon", &[]))
        .add_card(make_digimon(
            "ALLY-ROYAL-KNIGHT",
            "Gallantmon",
            &["Royal Knight"],
        ))
        .add_card(make_digimon("ALLY-PUPPET", "Puppetmon", &["Puppet"]))
        .add_card(make_digimon("OPP-HUCKMON", "Huckmon", &[]))
        .memory(10)
        .start()
}

fn make_digimon(id: &str, name: &str, traits: &[&str]) -> CardData {
    let mut card = make_test_card(id, name);
    card.colors = vec![CardColor::White];
    card.dp = Some(3000);
    card.traits = traits
        .iter()
        .map(|trait_name| trait_name.to_string())
        .collect();
    card
}

#[test]
fn bt6_084_has_printed_metadata() {
    let runner = sistermon_runner();
    let card = runner
        .compiled_card("BT6-084")
        .expect("BT6-084 compiled card present");

    assert_eq!(card.name, "Sistermon Ciel");
    assert_eq!(card.kind, CompiledCardKind::Digimon);
    assert_eq!(card.level, Some(4));
    assert_eq!(card.cost, Some(4));
    assert_eq!(card.dp, Some(5000));
    assert_eq!(card.color, vec![CompiledColor::White]);
    assert!(card.traits.iter().any(|trait_name| trait_name == "Puppet"));
    assert_eq!(
        card.attribute.as_deref(),
        Some("Data"),
        "BT6-084 is a Data attribute Digimon"
    );
}

#[test]
fn bt6_084_compiles_face_up_aura_and_on_play_only() {
    let runner = sistermon_runner();
    let card = runner
        .compiled_card("BT6-084")
        .expect("BT6-084 compiled card present");

    assert_eq!(
        card.effects.len(),
        2,
        "BT6-084 should compile exactly the printed face-up aura and On Play memory gain"
    );
    assert!(
        !card.effects.iter().any(|clause| match clause {
            CompiledClause::Declarative(CompiledDeclarativeClause::Aura { scope, .. }) => {
                *scope == CompiledScope::Inherited
            }
            CompiledClause::Triggered(triggered) => triggered.scope == CompiledScope::Inherited,
            _ => false,
        }),
        "duplicated ingestion text must not become an inherited effect"
    );
}

#[test]
fn bt6_084_aura_clause_targets_own_huckmon_or_royal_knight() {
    let runner = sistermon_runner();
    let card = runner
        .compiled_card("BT6-084")
        .expect("BT6-084 compiled card present");

    let aura = card
        .effects
        .iter()
        .find_map(|clause| match clause {
            CompiledClause::Declarative(CompiledDeclarativeClause::Aura {
                scope,
                active_when,
                target,
                dp_modifier,
                ..
            }) => Some((*scope, active_when, target, *dp_modifier)),
            _ => None,
        })
        .expect("BT6-084 declarative aura clause exists");

    let (scope, active_when, target, dp_modifier) = aura;
    assert_eq!(scope, CompiledScope::FaceUp);
    assert_eq!(dp_modifier, Some(2000));
    assert!(
        active_when
            .as_ref()
            .is_some_and(|predicate| predicate.all_turns == Some(true)),
        "the aura is active [All Turns]"
    );
    assert_eq!(target.owner, Some(CompiledPlayerRef::You));
    assert!(target.zone.contains(&CompiledZone::BattleArea));
    assert_eq!(
        target.kind,
        Some(CompiledCardKind::Digimon),
        "the aura is restricted to Digimon"
    );
    assert_eq!(target.any_of.len(), 2, "Huckmon name OR Royal Knight trait");
    assert!(
        target
            .any_of
            .iter()
            .any(|predicate| predicate.name_contains.as_deref() == Some("Huckmon")),
        "one aura branch must match [Huckmon] in name"
    );
    assert!(
        target
            .any_of
            .iter()
            .any(|predicate| predicate.trait_has.as_deref() == Some("Royal Knight")),
        "one aura branch must match [Royal Knight] trait"
    );
}

#[test]
fn bt6_084_on_play_gains_one_memory_after_paying_cost() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT6-084")
        .expect("BT6-084 YAML loads")
        .hand(0, &["BT6-084"])
        .memory(10)
        .start();

    runner.play(0, 0).expect("Sistermon Ciel plays from hand");

    assert_eq!(
        runner.memory(),
        7,
        "pay 4 memory for BT6-084, then its On Play effect gains 1 memory"
    );
    assert!(
        runner.pending_selection().is_none(),
        "On Play gain 1 memory has no player-visible choice"
    );
}

#[test]
fn bt6_084_aura_buffs_own_huckmon_and_royal_knight_only() {
    let mut runner = sistermon_runner();
    runner.place_on_field(0, "BT6-084", None);
    let huckmon = runner.place_on_field(0, "ALLY-HUCKMON", None);
    let royal_knight = runner.place_on_field(0, "ALLY-ROYAL-KNIGHT", None);
    let non_target = runner.place_on_field(0, "ALLY-PUPPET", None);
    let opponent_huckmon = runner.place_on_field(1, "OPP-HUCKMON", None);

    runner.game.tick_declarative_effects();

    assert_eq!(runner.effective_dp(huckmon), Some(5000));
    assert_eq!(runner.effective_dp(royal_knight), Some(5000));
    assert_eq!(
        runner.effective_dp(non_target),
        Some(3000),
        "own non-Huckmon non-Royal Knight Digimon is not buffed"
    );
    assert_eq!(
        runner.effective_dp(opponent_huckmon),
        Some(3000),
        "opponent Huckmon is not your Digimon"
    );
}

#[test]
fn bt6_084_aura_refreshes_when_source_leaves_field() {
    let mut runner = sistermon_runner();
    runner.place_on_field(0, "BT6-084", None);
    let huckmon = runner.place_on_field(0, "ALLY-HUCKMON", None);

    runner.game.tick_declarative_effects();
    assert_eq!(
        runner.game.modifiers.sum(huckmon, ModifierType::ChangeDp),
        2000
    );

    runner.game.players[0].battle_area.remove(0);
    runner.game.tick_declarative_effects();

    let shifted_huckmon = digimon_engine::permanent::PermanentHandle {
        player: 0,
        index: 0,
    };
    assert_eq!(
        runner.effective_dp(shifted_huckmon),
        Some(3000),
        "aura modifier must clear after Sistermon Ciel leaves the battle area"
    );
}
