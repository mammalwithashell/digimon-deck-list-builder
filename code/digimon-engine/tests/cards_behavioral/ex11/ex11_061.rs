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
use digimon_engine::enums::{CardKind, PlaySource};

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

/// A Digimon of a given level / trait set, used for digivolve fixtures.
/// Levels above 3 carry an `evo_cost` from `level - 1` so `digivolve_from_hand`
/// accepts the digivolution onto a one-level-lower base.
fn digimon_lvl(id: &str, level: u8, traits: &[&str]) -> digimon_engine::card_data::CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.level = Some(level);
    card.dp = Some(3000);
    card.traits = traits.iter().map(|t| t.to_string()).collect();
    if level > 3 {
        card.evo_costs = vec![digimon_engine::card_data::EvoCost {
            card_color: 0,
            level: level - 1,
            memory_cost: 0,
        }];
    }
    card
}

fn card_ids_on_field(runner: &DebugRunner, player: usize) -> Vec<String> {
    runner.game.players[player]
        .battle_area
        .iter()
        .map(|perm| perm.top_card().card_id(&runner.game.card_data).to_string())
        .collect()
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

/// Build a runner with EX11-061 (Tamer) + a Lv.3 base + a Lv.4 Puppet to
/// digivolve into + a Lv.3 Puppet to play from hand.
fn digivolve_observer_runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("EX11-061")
        .expect("EX11-061 YAML loads")
        .add_card(digimon_lvl("BASE-LV3", 3, &[]))
        .add_card(digimon_lvl("PUPPET-LV4", 4, &["Puppet"]))
        .add_card(digimon_lvl("PUPPET-LV3", 3, &["Puppet"]))
        .add_card(digimon_lvl("NON-PUPPET-LV3", 3, &["Beast"]))
        .hand(0, &["PUPPET-LV4", "PUPPET-LV3", "NON-PUPPET-LV3"])
        .memory(20)
        .start()
}

/// PUPPETS-G003 (Task 11): the [Your Turn] digivolve observer fires when an
/// own Digimon digivolves into a Puppet; accepting suspends the Tamer and
/// offers a level-3 Puppet from hand.
#[test]
fn ex11_061_puppet_digivolve_observer_suspends_tamer_and_offers_level3_puppet_from_hand() {
    let mut runner = digivolve_observer_runner();
    let tamer = runner.place_on_field(0, "EX11-061", Some(0));
    let base = runner.place_on_field(0, "BASE-LV3", Some(0));
    runner.game.enter_main_phase();

    // Digivolve BASE-LV3 → PUPPET-LV4 (a Puppet) — fires the observer.
    let puppet_lv4_idx = runner.game.players[0]
        .hand
        .iter()
        .position(|c| c.card_id(&runner.game.card_data) == "PUPPET-LV4")
        .expect("PUPPET-LV4 in hand");
    let ok = runner.game.digivolve_from_hand(
        0,
        puppet_lv4_idx,
        base.index as usize,
        PlaySource::ByDigivolve,
    );
    assert!(ok, "digivolve into a Puppet must succeed");
    runner.game.drain_effect_queue();

    // The optional activation-cost trigger installs a pre-cost decline prompt.
    let activation = runner
        .pending_selection_view()
        .expect("digivolve observer installs a pre-cost activation prompt");
    assert!(
        activation.is_optional,
        "suspending the Tamer is optional — the prompt must expose PASS"
    );
    // Accept the trigger (first action is the trigger, not PASS).
    runner
        .execute_action(activation.selecting_player, activation.valid_action_ids[0])
        .expect("accept suspend-tamer activation");
    runner.game.drain_effect_queue();

    assert!(
        runner.game.players[0].battle_area[tamer.index as usize].is_suspended,
        "accepting the activation suspends this Tamer"
    );

    // The play selection must offer only the level-3 Puppet from hand.
    let pick = runner
        .pending_selection_view()
        .expect("level-3 Puppet hand selection installs");
    assert!(pick.is_optional, "'you may play' is optional");
    use digimon_engine::action::space::{PASS, PLAY_HAND_START};
    // PUPPET-LV3 is in hand; NON-PUPPET-LV3 and the absent PUPPET-LV4 are not
    // eligible. Exactly one hand candidate must be offered.
    let hand_candidates: Vec<u16> = pick
        .valid_action_ids
        .iter()
        .copied()
        .filter(|id| *id != PASS && *id < PLAY_HAND_START + 50)
        .collect();
    assert_eq!(
        hand_candidates.len(),
        1,
        "only the level-3 Puppet may be offered; got {:?}",
        pick.valid_action_ids
    );
}

/// PUPPETS-G003 (Task 11): end-of-turn cleanup deletes exactly the Digimon
/// this effect played, surviving an index shift.
#[test]
fn ex11_061_turn_end_deletes_only_the_digimon_this_effect_played() {
    let mut runner = digivolve_observer_runner();
    runner.place_on_field(0, "EX11-061", Some(0));
    let base = runner.place_on_field(0, "BASE-LV3", Some(0));
    runner.game.enter_main_phase();

    let puppet_lv4_idx = runner.game.players[0]
        .hand
        .iter()
        .position(|c| c.card_id(&runner.game.card_data) == "PUPPET-LV4")
        .expect("PUPPET-LV4 in hand");
    let ok = runner.game.digivolve_from_hand(
        0,
        puppet_lv4_idx,
        base.index as usize,
        PlaySource::ByDigivolve,
    );
    assert!(ok, "digivolve into a Puppet must succeed");
    runner.game.drain_effect_queue();

    // Accept the activation, then pick PUPPET-LV3 from hand.
    let activation = runner.pending_selection_view().expect("activation prompt");
    runner
        .execute_action(activation.selecting_player, activation.valid_action_ids[0])
        .expect("accept activation");
    runner.game.drain_effect_queue();

    let pick = runner
        .pending_selection_view()
        .expect("level-3 Puppet hand selection");
    // Pick the first eligible (non-PASS) candidate — the level-3 Puppet.
    let chosen = pick
        .valid_action_ids
        .iter()
        .copied()
        .find(|id| *id != digimon_engine::action::space::PASS)
        .expect("a level-3 Puppet is offered");
    runner
        .execute_action(pick.selecting_player, chosen)
        .expect("pick the level-3 Puppet");
    runner.auto_resolve().expect("finish the play body");

    // The played level-3 Puppet is on field.
    let played_card_index = runner.game.players[0]
        .battle_area
        .iter()
        .find(|p| p.top_card().card_id(&runner.game.card_data) == "PUPPET-LV3")
        .map(|p| p.top_card().card_index)
        .expect("PUPPET-LV3 was played to the field");

    // End the turn — the cleanup deletes exactly the effect-played Puppet.
    runner.end_turn();
    runner.auto_resolve().expect("finish end-of-turn cleanup");

    assert!(
        !runner.game.players[0]
            .battle_area
            .iter()
            .any(|p| p.top_card().card_index == played_card_index),
        "the effect-played level-3 Puppet must be deleted at turn end"
    );
    assert!(
        card_ids_on_field(&runner, 0).contains(&"EX11-061".to_string()),
        "the Tamer EX11-061 itself must survive"
    );
}
