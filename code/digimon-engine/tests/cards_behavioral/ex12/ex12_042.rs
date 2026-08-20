use digimon_engine::enums::{CardColor, Keyword};

use super::support::{hand_contains, plain_digimon, select_first_non_pass, vb_digimon, DebugRunner};

const CARD_ID: &str = "EX12-042";

#[test]
fn ex12_042_has_blocker_and_inherited_barrier() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-042 YAML loads")
        .add_card(vb_digimon("CARRIER", CardColor::Yellow, 5, 7000))
        .start();

    let gatomon = runner.place_on_field(0, CARD_ID, Some(0));
    assert!(
        runner.game.has_keyword(gatomon, Keyword::Blocker),
        "Gatomon should have printed Blocker"
    );

    let carrier = runner.place_stack(0, &[CARD_ID, "CARRIER"]);
    assert!(
        runner.game.has_keyword(carrier, Keyword::Barrier),
        "carrier inherits Barrier from EX12-042"
    );
}

#[test]
fn ex12_042_on_play_adds_top_security_to_hand_then_recovers() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-042 YAML loads")
        .add_card(plain_digimon("SECURITY-CARD", CardColor::Yellow, 3, 2000))
        .add_card(plain_digimon("RECOVERY-CARD", CardColor::Yellow, 3, 2000))
        .security(0, &["SECURITY-CARD"])
        .deck(0, &["RECOVERY-CARD"])
        .memory(5)
        .start();

    let gatomon = runner.place_on_field(0, CARD_ID, Some(0));
    runner.fire_on_play(0, gatomon.index as usize);
    runner.auto_resolve().expect("resolve Gatomon on-play");

    assert!(hand_contains(&runner, 0, "SECURITY-CARD"));
    assert_eq!(runner.security_count(0), 1);
    assert_eq!(
        runner.game.players[0].security[0].card_id(&runner.game.card_data),
        "RECOVERY-CARD",
        "Recovery +1 should replace the security card that was added to hand"
    );
}

/// Inherited `<Barrier>` must actually FIRE when the carrier would be deleted
/// in a real battle — not merely be present as a granted keyword.
///
/// `ex12_042_has_blocker_and_inherited_barrier` asserts the grant reaches the
/// carrier, which is necessary but not sufficient: it says nothing about the
/// deletion path ever consulting it.
///
/// The DCGO parity corpus found the gap. In recording `20260820T154511Z` a
/// 7000 DP attacker whose stack included EX12-042 attacked into a 12000 DP
/// security Digimon. DCGO offered Barrier, the player accepted (trashing their
/// top security), the attacker survived and went on to digivolve. Replaying the
/// same actions through this engine deleted it outright — the attacker and its
/// whole digivolution stack ended up in the trash and the follow-up digivolve
/// was rejected as illegal. Six recordings in that corpus share the signature.
#[test]
#[ignore = "ENGINE GAP: DSL-granted replacement-process keywords never install their handler -- see docs/RUST_ENGINE_GAPS.md. Un-ignore when fixed."]
fn ex12_042_inherited_barrier_prevents_a_lethal_battle_deletion() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-042 YAML loads")
        .add_card(vb_digimon("CARRIER", CardColor::Yellow, 5, 7000))
        .add_card(plain_digimon("SEC-BIG", CardColor::Red, 6, 12000))
        .add_card(plain_digimon("MY-SEC-A", CardColor::Blue, 4, 4000))
        .add_card(plain_digimon("MY-SEC-B", CardColor::Blue, 4, 4000))
        // Barrier trashes the carrier's OWN top security, so P0 needs a stack.
        .security(0, &["MY-SEC-A", "MY-SEC-B"])
        .security(1, &["SEC-BIG"])
        .start();

    let carrier = runner.place_stack(0, &[CARD_ID, "CARRIER"]);
    assert!(
        runner.game.has_keyword(carrier, Keyword::Barrier),
        "precondition: carrier inherits Barrier from EX12-042"
    );
    let my_security_before = runner.security_count(0);

    // 7000 into a 12000 security Digimon: lethal unless Barrier intervenes.
    let _ = runner.attack_player(carrier, 1, false);

    // Barrier is Opt-cost -> Mand (rules 16-24): paying is optional, so the
    // engine must PROMPT rather than auto-apply or auto-decline.
    if let Some(offer) = runner.pending_selection_view() {
        assert!(
            offer.is_optional,
            "Barrier's cost is optional, so the prompt must be declinable"
        );
        select_first_non_pass(&mut runner);
    }

    assert!(
        runner
            .game
            .player(0)
            .battle_area
            .get(carrier.index as usize)
            .is_some(),
        "inherited Barrier must prevent the battle deletion. P0 trash is {:?}",
        runner
            .game
            .player(0)
            .trash
            .iter()
            .map(|c| c.card_id(&runner.game.card_data).to_string())
            .collect::<Vec<_>>()
    );
    assert!(
        runner.security_count(0) < my_security_before,
        "using Barrier must trash the carrier's own top security card"
    );
}

/// The security check deletes a losing attacker WITHOUT offering
/// `WhenWouldBeDeleted` replacements.
///
/// Paired with `granted_barrier_prevents_a_lethal_digimon_battle_deletion`
/// below, which is the same card and the same lethal DP compare in a plain
/// Digimon battle and PASSES. The difference between the two isolates the bug:
/// Barrier works, the security-check path just never asks.
///
/// `combat/mod.rs` (SecurityPhase::BattleResolved) calls
/// `delete_permanent_with_effects(attacker)` directly. It special-cases the
/// PASSIVE keyword Jamming inline but never runs the replacement scan, so every
/// replacement keyword — Barrier, Evade, Armor Purge, Fragment, Scapegoat — is
/// silently skipped on a security battle. The sibling phase
/// `WhenWouldLoseSecurity` does it correctly.
#[test]
#[ignore = "ENGINE GAP: security-check battle skips the WhenWouldBeDeleted replacement scan -- see docs/RUST_ENGINE_GAPS.md. Un-ignore when fixed."]
fn granted_barrier_is_ignored_by_the_security_check() {
    const YAML: &str = r#"
card: PROBE-BARRIER
name: Probe Barrier
kind: digimon
level: 5
color: [blue]
cost: 5
dp: 7000
traits: [Probe]
attribute: Vaccine

effects:
  - kind: grant_keyword
    keyword: Barrier
    summary: "<Barrier>"
"#;

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("probe YAML compiles")
        .add_card(plain_digimon("SEC-BIG", CardColor::Red, 6, 12000))
        .add_card(plain_digimon("MY-SEC-A", CardColor::Blue, 4, 4000))
        .add_card(plain_digimon("MY-SEC-B", CardColor::Blue, 4, 4000))
        .security(0, &["MY-SEC-A", "MY-SEC-B"])
        .security(1, &["SEC-BIG"])
        .start();

    let carrier = runner.place_on_field(0, "PROBE-BARRIER", Some(0));
    assert!(
        runner.game.has_keyword(carrier, Keyword::Barrier),
        "precondition: face-up grant puts Barrier on the carrier"
    );

    let _ = runner.attack_player(carrier, 1, false);
    if runner.pending_selection_view().is_some() {
        select_first_non_pass(&mut runner);
    }

    assert!(
        runner
            .game
            .player(0)
            .battle_area
            .get(carrier.index as usize)
            .is_some(),
        "FACE-UP granted Barrier also failed to prevent the deletion — the gap \
         is not specific to `scope: inherited`. P0 trash: {:?}",
        runner
            .game
            .player(0)
            .trash
            .iter()
            .map(|c| c.card_id(&runner.game.card_data).to_string())
            .collect::<Vec<_>>()
    );
}

/// Control: the same granted Barrier DOES fire in a plain Digimon battle.
///
/// This is what makes the ignored security-check test above a precise finding
/// rather than a vague "Barrier is broken" — the grant machinery, the keyword
/// lookup, and the replacement scan are all working here. Worth keeping as a
/// standing regression test: no other test in the suite asserts that Barrier
/// actually fires, only that the clause compiles.
#[test]
fn granted_barrier_prevents_a_lethal_digimon_battle_deletion() {
    const YAML: &str = r#"
card: PROBE-BARRIER-BATTLE
name: Probe Barrier Battle
kind: digimon
level: 5
color: [blue]
cost: 5
dp: 7000
traits: [Probe]
attribute: Vaccine

effects:
  - kind: grant_keyword
    keyword: Barrier
    summary: "<Barrier>"
"#;

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("probe YAML compiles")
        .add_card(plain_digimon("BIG-BLOCKER", CardColor::Red, 6, 12000))
        .add_card(plain_digimon("MY-SEC-A", CardColor::Blue, 4, 4000))
        .security(0, &["MY-SEC-A"])
        .start();

    let carrier = runner.place_on_field(0, "PROBE-BARRIER-BATTLE", Some(0));
    let defender = runner.place_on_field(1, "BIG-BLOCKER", Some(0));
    assert!(runner.game.has_keyword(carrier, Keyword::Barrier));

    let _ = runner.attack_digimon(carrier, defender, false);
    if runner.pending_selection_view().is_some() {
        select_first_non_pass(&mut runner);
    }

    assert!(
        runner
            .game
            .player(0)
            .battle_area
            .get(carrier.index as usize)
            .is_some(),
        "granted Barrier failed in a plain Digimon battle too. P0 trash: {:?}",
        runner
            .game
            .player(0)
            .trash
            .iter()
            .map(|c| c.card_id(&runner.game.card_data).to_string())
            .collect::<Vec<_>>()
    );
}
