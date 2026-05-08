//! BT4-097 Kari Kamiya
//!
//! Printed text:
//! [All Turns] When a card is removed from your security stack, by suspending
//! this Tamer, gain 1 memory.
//!
//! Inherited/security text in data: [Security] Play this card without paying
//! the cost.

use digimon_dsl::compiled::{CompiledClause, CompiledScope, CompiledTiming};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::selection::SelectionKind;

const KARI_YAML: &str = include_str!("../../../cards/bt4/BT4-097.yaml");

fn make_filler(id: &str) -> CardData {
    make_test_card(id, id)
}

fn kari_runner() -> DebugRunner {
    DebugRunner::builder()
        .from_dsl_yaml(KARI_YAML)
        .expect("BT4-097 YAML parses")
        .add_card(make_filler("ATTACKER"))
        .add_card(make_filler("SEC"))
        .add_card(make_filler("FILLER-DECK"))
        .security(1, &["SEC", "SEC"])
        .deck(0, &["FILLER-DECK"])
        .deck(1, &["FILLER-DECK"])
        .memory(5)
        .start()
}

#[test]
fn bt4_097_has_own_security_removed_and_security_clauses() {
    let runner = kari_runner();
    let compiled = runner.compiled_card("BT4-097").expect("compiled Kari");

    let triggered: Vec<_> = compiled
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();

    assert_eq!(triggered.len(), 2);
    let own_sec = triggered
        .iter()
        .find(|t| t.when.contains(&CompiledTiming::OnOwnSecurityRemoved))
        .expect("own security removed clause");
    assert_eq!(own_sec.scope, CompiledScope::FaceUp);
    assert!(
        own_sec.active_when.is_some(),
        "Kari's [All Turns] clause should carry an active_when gate"
    );

    assert!(
        triggered
            .iter()
            .any(|t| t.when.contains(&CompiledTiming::OnSecurity)),
        "Kari should retain her [Security] play clause"
    );
}

#[test]
fn bt4_097_own_security_removed_suspends_kari_and_gains_memory() {
    let mut runner = kari_runner();
    let kari = runner.place_on_field(1, "BT4-097", Some(0));
    let attacker = runner.place_on_field(0, "ATTACKER", Some(0));

    assert!(
        !runner.game.players[1].battle_area[kari.index as usize].is_suspended,
        "Kari starts unsuspended"
    );
    let before = runner.memory();

    runner.attack_player(attacker, 1, true);
    let _ = runner.auto_resolve();

    assert!(
        runner.game.players[1].battle_area[kari.index as usize].is_suspended,
        "Kari should suspend herself as the activation cost"
    );
    assert_eq!(
        runner.memory(),
        before - 1,
        "P1 gaining 1 memory moves the memory counter one step toward P1"
    );
}

#[test]
fn bt4_097_does_not_fire_for_opponents_security_removed() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(KARI_YAML)
        .expect("BT4-097 YAML parses")
        .add_card(make_filler("ATTACKER-P1"))
        .add_card(make_filler("SEC-P0"))
        .add_card(make_filler("FILLER-DECK"))
        .security(0, &["SEC-P0"])
        .deck(0, &["FILLER-DECK"])
        .deck(1, &["FILLER-DECK"])
        .memory(5)
        .start();

    let kari = runner.place_on_field(1, "BT4-097", Some(0));
    let attacker = runner.place_on_field(1, "ATTACKER-P1", Some(0));
    let before = runner.memory();

    runner.attack_player(attacker, 0, true);
    let _ = runner.auto_resolve();

    assert!(
        !runner.game.players[1].battle_area[kari.index as usize].is_suspended,
        "Kari should not react when the opponent's security is removed"
    );
    assert_eq!(runner.memory(), before);
    assert!(
        !matches!(runner.pending_kind(), Some(SelectionKind::OwnField)),
        "no hidden Kari follow-up selection should be pending"
    );
}

#[test]
fn bt4_097_already_suspended_does_not_fire() {
    let mut runner = kari_runner();
    let kari = runner.place_on_field(1, "BT4-097", Some(0));
    runner.game.players[1].battle_area[kari.index as usize].is_suspended = true;
    let attacker = runner.place_on_field(0, "ATTACKER", Some(0));
    let before = runner.memory();

    runner.attack_player(attacker, 1, true);
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.memory(),
        before,
        "suspended Kari cannot pay the suspend cost and should not gain memory"
    );
}
