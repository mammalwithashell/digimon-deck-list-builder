//! EX11-069 Yuuki
//!
//! Implemented slice:
//! - [Start of Your Main Phase] [On Play] By trashing 1 card in hand, gain 1 memory.
//! - [Security] Play this card free.
//!
//! Gap-routed slice:
//! - Attack-triggered trash digivolve and end-of-all-turns trash recursion.

use digimon_dsl::compiled::{CompiledClause, CompiledTiming};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};

#[test]
fn ex11_069_has_start_main_on_play_and_security_clauses() {
    let runner = DebugRunner::builder()
        .dsl_card("EX11-069")
        .expect("EX11-069 must load from embedded DSL pack")
        .memory(5)
        .start();
    let card = runner.compiled_card("EX11-069").expect("compiled card");

    assert!(
        card.effects.iter().any(|clause| matches!(
            clause,
            CompiledClause::Triggered(t)
                if t.when.contains(&CompiledTiming::StartOfYourMainPhase)
                    && t.when.contains(&CompiledTiming::OnPlay)
        )),
        "EX11-069 must share one trash-for-memory body across StartOfYourMainPhase and OnPlay"
    );
    assert!(
        card.effects.iter().any(|clause| matches!(
            clause,
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::OnSecurity)
        )),
        "EX11-069 must have Security play"
    );
}

#[test]
fn ex11_069_on_play_trashes_one_card_and_gains_memory() {
    let discard = make_test_card("EX11-069-DISCARD", "Discard");
    let mut runner = DebugRunner::builder()
        .dsl_card("EX11-069")
        .expect("EX11-069 must load")
        .add_card(discard)
        .hand(0, &["EX11-069", "EX11-069-DISCARD"])
        .memory(4)
        .start();

    runner.play(0, 0);
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.memory(),
        1,
        "Yuuki should gain 1 memory after paying the 4 play cost"
    );
    assert_eq!(
        runner.game.players[0].trash.len(),
        1,
        "one hand card should be trashed"
    );
}

#[ignore = "pending: G-EFFECT-INITIATED-DIGIVOLVE-FROM-TRASH-ON-ATTACK — attack observer needs trash digivolve into Dark/Evil Dragon with reduced cost"]
#[test]
fn ex11_069_attack_observer_digivolves_from_trash() {}

#[ignore = "pending: G-END-OF-ALL-TURNS-SUSPEND-COST-TRASH-RECURSION — end-of-all-turns Tamer suspend cost plus union trait return from trash"]
#[test]
fn ex11_069_end_of_all_turns_returns_evil_or_dragon_trait_card() {}
