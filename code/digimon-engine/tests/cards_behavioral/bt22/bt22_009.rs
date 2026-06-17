//! BT22-009 Effecmon
//!
//! Implemented slice:
//! - [On Play] [When Digivolving] Delete 1 opponent Digimon with 4000 DP or less.
//!
//! Gap-routed slice:
//! - [Security] At end of battle, play this card free.
//! - Link Requirements text.

use digimon_dsl::compiled::{CompiledAltPathKind, CompiledClause, CompiledCost, CompiledTiming};
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};

#[test]
fn bt22_009_has_stnd_trait_digivolve_path() {
    let runner = runner();
    let card = runner.compiled_card("BT22-009").expect("compiled");
    // Second digivolve circle on the card face ("Stnd. 2"): digivolve onto a
    // Digimon with the [Stnd.] form-trait for cost 2 (DCGO EqualsTraits("Stnd.")).
    assert!(
        card.alt_paths.iter().any(|path| {
            path.kind == CompiledAltPathKind::Digivolve
                && path.cost == Some(CompiledCost::Literal(2))
                && path
                    .from
                    .as_ref()
                    .is_some_and(|f| f.trait_has.as_deref() == Some("Stnd."))
        }),
        "BT22-009 must have a [Stnd.]-trait digivolve path at cost 2"
    );
}

fn runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("BT22-009")
        .expect("BT22-009 must load from embedded DSL pack")
        .memory(5)
        .start()
}

#[test]
fn bt22_009_has_on_play_and_when_digivolving_delete_clause() {
    let runner = runner();
    let card = runner
        .compiled_card("BT22-009")
        .expect("BT22-009 must be compiled");
    let clause = card
        .effects
        .iter()
        .find_map(|clause| match clause {
            CompiledClause::Triggered(triggered)
                if triggered.when.contains(&CompiledTiming::OnPlay)
                    && triggered.when.contains(&CompiledTiming::WhenDigivolving) =>
            {
                Some(triggered)
            }
            _ => None,
        })
        .expect("BT22-009 must have a shared OnPlay/WhenDigivolving clause");

    assert!(
        !clause.optional,
        "delete clause is mandatory when a target exists"
    );
}

#[test]
fn bt22_009_on_play_deletes_4000_dp_or_lower_target() {
    let mut low = make_test_card("BT22-009-LOW", "LowTarget");
    low.dp = Some(4000);

    let mut runner = DebugRunner::builder()
        .dsl_card("BT22-009")
        .expect("BT22-009 must load")
        .add_card(low)
        .hand(0, &["BT22-009"])
        .memory(5)
        .start();

    runner.place_on_field(1, "BT22-009-LOW", None);
    runner.play(0, 0);
    runner.auto_resolve();

    assert!(
        runner.game.players[1].battle_area.is_empty(),
        "BT22-009 should delete the eligible low-DP opponent Digimon"
    );
}

/// Structural: BT22-009 must carry an [Security] (`on_security`) clause that is
/// mandatory (the card plays itself automatically — no PASS choice).
#[test]
fn bt22_009_has_security_play_self_clause() {
    let runner = runner();
    let card = runner
        .compiled_card("BT22-009")
        .expect("BT22-009 must be compiled");
    let sec = card
        .effects
        .iter()
        .find_map(|clause| match clause {
            CompiledClause::Triggered(triggered)
                if triggered.when == vec![CompiledTiming::OnSecurity] =>
            {
                Some(triggered)
            }
            _ => None,
        })
        .expect("BT22-009 must have an OnSecurity play-self clause");

    assert!(
        !sec.optional,
        "[Security] play-self must NOT be optional (the card plays automatically)"
    );
}

/// Integration: place BT22-009 in P1's security, attack with P0's Digimon.
/// At the end of the battle the [Security] clause plays Effecmon onto P1's
/// battle area for free (play_from_security), so security drops to 0 and P1's
/// battle area gains the card.
#[test]
fn bt22_009_security_plays_self_at_end_of_battle() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT22-009")
        .expect("BT22-009 must load")
        .add_card(make_test_card("ATTACKER", "Attacker"))
        .memory(10)
        .start();

    // Place attacker on P0's field (turn_played=0 to skip summoning sickness).
    let attacker = runner.place_on_field(0, "ATTACKER", Some(0));

    // Put Effecmon at the top of P1's security stack.
    {
        let eff_idx = runner
            .game
            .card_data
            .iter()
            .position(|c| c.card_id == "BT22-009")
            .expect("BT22-009 in card_data");
        let next = runner.game.next_card_index();
        let eff_src = CardSource::new(eff_idx, 1, next);
        runner.game.players[1].security.push(eff_src);
    }

    assert_eq!(
        runner.security_count(1),
        1,
        "pre: P1 has exactly 1 security (Effecmon)"
    );

    // P0 attacks P1's security.
    runner.attack_player(attacker, 1, false);
    // Resolve any pending selections from the security effect (the On Play delete
    // clause resolves immediately when there is no eligible target).
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.security_count(1),
        0,
        "security card (Effecmon) must be removed by the attack"
    );
    assert_eq!(
        runner.battle_area_size(1),
        1,
        "Effecmon must be played onto P1's battle area for free by its [Security] effect at end of battle"
    );
}

#[ignore = "pending: G-LINK-REQUIREMENTS — Link text requires plug-in/link action and attachment lifecycle support"]
#[test]
fn bt22_009_link_requirements_are_available_as_link_actions() {}
