//! BT6-082 Sistermon Blanc - Digimon, Lv.3, White.
//!
//! Printed text:
//! - [All Turns] While you have a Digimon with [Huckmon] in its name or
//!   [Royal Knight] trait in play, all of your Digimon with [Sistermon] in
//!   their name gain <Blocker>.
//! - [On Play] <Draw 1>.

use digimon_dsl::compiled::{
    CompiledCardKind, CompiledClause, CompiledColor, CompiledDeclarativeClause, CompiledPlayerRef,
    CompiledScope, CompiledZone,
};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, Keyword};

fn predicate_has_all_turns(predicate: &digimon_dsl::compiled::CompiledPredicate) -> bool {
    predicate.all_turns == Some(true)
        || predicate.all_of.iter().any(predicate_has_all_turns)
        || predicate.any_of.iter().any(predicate_has_all_turns)
}

fn make_digimon(id: &str, name: &str, traits: &[&str]) -> CardData {
    let mut card = make_test_card(id, name);
    card.colors = vec![CardColor::White];
    card.dp = Some(3000);
    card.traits = traits.iter().map(|name| name.to_string()).collect();
    card
}

fn runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("BT6-082")
        .expect("BT6-082 YAML loads")
        .add_card(make_digimon("ALLY-HUCKMON", "Huckmon", &[]))
        .add_card(make_digimon(
            "ALLY-SISTERMON",
            "Sistermon Noir",
            &["Puppet"],
        ))
        .add_card(make_digimon("ALLY-NON-SISTER", "Puppetmon", &["Puppet"]))
        .add_card(make_digimon("OPP-SISTERMON", "Sistermon Ciel", &["Puppet"]))
        .add_card(make_digimon("FILLER", "Filler", &[]))
        .memory(10)
        .start()
}

#[test]
fn bt6_082_has_printed_metadata() {
    let runner = runner();
    let card = runner
        .compiled_card("BT6-082")
        .expect("BT6-082 compiled card present");

    assert_eq!(card.name, "Sistermon Blanc");
    assert_eq!(card.kind, CompiledCardKind::Digimon);
    assert_eq!(card.level, Some(3));
    assert_eq!(card.cost, Some(3));
    assert_eq!(card.dp, Some(3000));
    assert_eq!(card.color, vec![CompiledColor::White]);
    assert!(card.traits.iter().any(|name| name == "Puppet"));
    assert_eq!(card.attribute.as_deref(), Some("Vaccine"));
}

#[test]
fn bt6_082_compiles_filtered_blocker_aura_and_on_play_draw() {
    let runner = runner();
    let card = runner
        .compiled_card("BT6-082")
        .expect("BT6-082 compiled card present");

    assert_eq!(
        card.effects.len(),
        2,
        "only the printed face-up aura and On Play draw should be authored"
    );

    let aura = card
        .effects
        .iter()
        .find_map(|clause| match clause {
            CompiledClause::Declarative(CompiledDeclarativeClause::Aura {
                scope,
                active_when,
                target,
                grant_keyword,
                ..
            }) => Some((*scope, active_when, target, grant_keyword)),
            _ => None,
        })
        .expect("BT6-082 Blocker aura clause exists");

    assert_eq!(aura.0, CompiledScope::FaceUp);
    assert!(
        aura.1.as_ref().is_some_and(predicate_has_all_turns),
        "printed timing is [All Turns]"
    );
    assert_eq!(aura.2.owner, Some(CompiledPlayerRef::You));
    assert!(aura.2.zone.contains(&CompiledZone::BattleArea));
    assert_eq!(aura.2.kind, Some(CompiledCardKind::Digimon));
    assert_eq!(aura.2.name_contains.as_deref(), Some("Sistermon"));
    assert_eq!(
        aura.3.as_ref().map(|grant| grant.keyword.as_str()),
        Some("Blocker")
    );
}

#[test]
fn bt6_082_on_play_draws_one_after_paying_cost() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT6-082")
        .expect("BT6-082 YAML loads")
        .add_card(make_digimon("FILLER", "Filler", &[]))
        .hand(0, &["BT6-082"])
        .deck(0, &["FILLER"])
        .memory(10)
        .start();

    runner.play(0, 0).expect("Sistermon Blanc plays from hand");

    assert_eq!(runner.memory(), 7, "pay 3 memory, then draw 1");
    assert_eq!(
        runner.hand_size(0),
        1,
        "draw replaces the played card in hand"
    );
    assert!(
        runner.pending_selection().is_none(),
        "Draw 1 has no player-visible choice"
    );
}

#[test]
fn bt6_082_aura_grants_blocker_to_own_sistermon_only_when_anchor_exists() {
    let mut runner = runner();
    runner.place_on_field(0, "BT6-082", None);
    let sistermon = runner.place_on_field(0, "ALLY-SISTERMON", None);
    let non_sistermon = runner.place_on_field(0, "ALLY-NON-SISTER", None);
    let opponent_sistermon = runner.place_on_field(1, "OPP-SISTERMON", None);

    runner.game.tick_declarative_effects();
    assert!(
        !runner.game.has_keyword(sistermon, Keyword::Blocker),
        "without Huckmon/Royal Knight in play, the aura is inactive"
    );

    runner.place_on_field(0, "ALLY-HUCKMON", None);
    runner.game.tick_declarative_effects();

    assert!(runner.game.has_keyword(sistermon, Keyword::Blocker));
    assert!(
        !runner.game.has_keyword(non_sistermon, Keyword::Blocker),
        "own non-Sistermon Digimon is not granted Blocker"
    );
    assert!(
        !runner
            .game
            .has_keyword(opponent_sistermon, Keyword::Blocker),
        "opponent Sistermon is not your Digimon"
    );
}
