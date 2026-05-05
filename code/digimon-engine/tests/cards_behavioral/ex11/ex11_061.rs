//! EX11-061 Mirai Kinosaki — Tamer, Yellow/Purple, Cost 4.
//!
//! # Card text (cards.json)
//!
//! [Start of Your Main Phase] If your opponent has a Digimon, gain 1 memory.
//!
//! [Your Turn] When any of your Digimon digivolve into a [Puppet] trait Digimon,
//! by suspending this Tamer, you may play 1 level 3 [Puppet] trait Digimon card
//! from your hand without paying the cost. At turn end, delete the Digimon this
//! effect played.
//!
//! Security Effect [Security] Play this card without paying the cost.
//!
//! # Legacy reference
//!
//! code/engine_py_legacy/engine/data/scripts/ex11/ex11_061.py
//!
//! # Patterns
//!
//! - B1 Start-of-main Tamer memory gain with opponent-Digimon condition
//! - Security Tamer self-play
//! - PUPPETS-G003 blocked observer: exact effect-played permanent cleanup

use digimon_dsl::compiled::{CompiledCardKind, CompiledClause, CompiledColor, CompiledTiming};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};

fn load_runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("EX11-061")
        .expect("EX11-061 YAML loads")
        .memory(5)
        .start()
}

fn digimon_card(id: &str) -> digimon_engine::card_data::CardData {
    let mut card = make_test_card(id, id);
    card.level = Some(3);
    card.dp = Some(2000);
    card
}

#[test]
fn ex11_061_has_printed_metadata_and_supported_clauses() {
    let runner = load_runner();
    let compiled = runner
        .compiled_card("EX11-061")
        .expect("EX11-061 must be compiled");

    assert_eq!(compiled.name, "Mirai Kinosaki");
    assert_eq!(compiled.kind, CompiledCardKind::Tamer);
    assert_eq!(compiled.cost, Some(4));
    assert_eq!(
        compiled.color,
        vec![CompiledColor::Yellow, CompiledColor::Purple]
    );
    assert!(compiled
        .traits
        .iter()
        .any(|trait_name| trait_name == "LIBERATOR"));

    let timings: Vec<_> = compiled
        .effects
        .iter()
        .filter_map(|clause| match clause {
            CompiledClause::Triggered(triggered) => Some(triggered.when.clone()),
            _ => None,
        })
        .collect();

    assert!(
        timings
            .iter()
            .any(|when| when.contains(&CompiledTiming::StartOfYourMainPhase)),
        "supported slice must include the start-of-main memory clause"
    );
    assert!(
        timings
            .iter()
            .any(|when| when.contains(&CompiledTiming::OnSecurity)),
        "supported slice must include the printed Security play clause"
    );
}

#[test]
fn ex11_061_start_of_main_gains_memory_when_opponent_has_digimon() {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX11-061")
        .expect("EX11-061 YAML loads")
        .add_card(digimon_card("OPP-DIGIMON"))
        .memory(5)
        .start();
    runner.place_on_field(0, "EX11-061", Some(0));
    runner.place_on_field(1, "OPP-DIGIMON", Some(0));

    runner.game.enter_main_phase();

    assert_eq!(
        runner.memory(),
        6,
        "Mirai should gain exactly 1 memory when opponent has a Digimon"
    );
}

#[test]
fn ex11_061_start_of_main_does_not_gain_memory_without_opponent_digimon() {
    let mut runner = load_runner();
    runner.place_on_field(0, "EX11-061", Some(0));

    runner.game.enter_main_phase();

    assert_eq!(
        runner.memory(),
        5,
        "Mirai should not gain memory when opponent has no Digimon"
    );
}

#[test]
fn ex11_061_security_plays_itself_without_paying_cost() {
    let mut attacker = make_test_card("ATTACKER", "Attacker");
    attacker.level = Some(4);
    attacker.dp = Some(9000);

    let mut runner = DebugRunner::builder()
        .dsl_card("EX11-061")
        .expect("EX11-061 YAML loads")
        .add_card(attacker)
        .security(1, &["EX11-061"])
        .memory(10)
        .start();
    let attacker = runner.place_on_field(0, "ATTACKER", Some(0));

    runner.attack_player(attacker, 1, false);
    runner.auto_resolve().expect("resolve security play");

    assert!(
        runner.game.players[1]
            .battle_area
            .iter()
            .any(|perm| perm.top_card().card_id(&runner.game.card_data) == "EX11-061"),
        "Mirai should be played from the defender's security"
    );
}

#[test]
#[ignore = "pending: PUPPETS-G003 — effect-played permanent provenance and turn-end cleanup are required for this observer"]
fn ex11_061_puppet_digivolve_observer_suspends_tamer_and_offers_level3_puppet_from_hand() {
    let _runner = load_runner();
    // Required behavior:
    // - normal on_digivolve observer fires only for your Digimon digivolving
    //   into a Puppet trait Digimon during your turn;
    // - activation is optional and only legal while this Tamer is unsuspended;
    // - accepting suspends exactly this Tamer as the cost;
    // - a pending hand selection exposes only level 3 Puppet Digimon cards;
    // - PASS is legal for the optional "you may play" choice.
}

#[test]
#[ignore = "pending: PUPPETS-G003 — exact identity-stable cleanup for the Digimon this effect played"]
fn ex11_061_turn_end_deletes_only_the_digimon_this_effect_played() {
    let _runner = load_runner();
    // Required behavior:
    // - the free-play helper must return or record the exact PermanentHandle
    //   created by this effect;
    // - end-of-turn cleanup deletes that permanent only;
    // - cleanup no-ops if that permanent already left;
    // - field-index shifts before turn end must not delete a different Puppet.
}
