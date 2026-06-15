//! BT15-084 Kari Kamiya — Tamer, Yellow, Cost 4.
//!
//! # Card text (cards.json — authoritative for printed text)
//!
//! ```text
//! When an effect trashes this card from the security stack, 1 of your
//!   opponent's Digimon gains <Security A. -1> until the end of their turn.
//! [Start of Your Turn] If you have 2 or less memory, set it to 3.
//! [All Turns] When an effect removes cards from your security stack, by
//!   suspending this Tamer, 1 of your opponent's Digimon gains <Security A. -1>
//!   until the end of their turn.
//! [Security] Play this card without paying the cost.
//! ```
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT15/Yellow/BT15_084.cs
//!
//! # Implemented slice
//! - [Start of Your Turn] memory floor.
//! - [Security] play free.
//! - [All Turns] on-own-security-removed observer: by suspending this Tamer
//!   (the activation cost), give 1 opponent Digimon Security Attack -1 until
//!   end of their turn.
//!
//! # Gap-routed slice (left stubbed + ignored)
//! - "When an effect trashes THIS CARD from the security stack, ..." — there is
//!   no DSL trigger for the carrier itself being discarded from its own
//!   security stack (G-DSL-ON-DISCARD-SECURITY-TRIGGER).
//!
//! # Patterns this test covers (RUST_DSL_TEST_API §4.3)
//! - F9 security-removed-conditioned Tamer (suspend-self activation cost)
//! - H4 Security A. -1 (SecurityAttackChange modifier)

use digimon_dsl::compiled::{CompiledClause, CompiledTiming};
use digimon_engine::action::space::encode_attack;
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardHandle;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::effect_context::EffectContext;
use digimon_engine::enums::{CardKind, ModifierType};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::SelectionKind;

const CARD_ID: &str = "BT15-084";

fn opp_digimon(id: &str) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.level = Some(4);
    card.dp = Some(4000);
    card
}

fn base() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT15-084 must load from embedded DSL pack")
        .add_card(make_test_card("PAD", "PAD"))
        .add_card(opp_digimon("OPP-A"))
        .add_card(opp_digimon("OPP-B"))
        .deck(0, &["PAD"; 10])
        .deck(1, &["PAD"; 10])
}

fn encode_permanent(handle: PermanentHandle) -> u16 {
    encode_attack(0, handle.index as u16)
}

// ─── Structural ──────────────────────────────────────────────────────────────

#[test]
fn bt15_084_has_memory_floor_security_play_and_security_removed_clauses() {
    let runner = base().memory(5).start();
    let card = runner.compiled_card(CARD_ID).expect("compiled card");

    assert!(
        card.effects.iter().any(|clause| matches!(
            clause,
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::StartOfYourTurn)
        )),
        "BT15-084 must have a start-of-turn memory floor"
    );
    assert!(
        card.effects.iter().any(|clause| matches!(
            clause,
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::OnSecurity)
        )),
        "BT15-084 must have a Security play clause"
    );
    assert!(
        card.effects.iter().any(|clause| matches!(
            clause,
            CompiledClause::Triggered(t)
                if t.when.contains(&CompiledTiming::OnOwnSecurityRemoved)
        )),
        "BT15-084 must have an own-security-removed observer"
    );
}

// ─── On-own-security-removed: behavior + negative gating ─────────────────────

/// Drive a real "an effect removes a card from your security stack" event so the
/// observer fires. Trashes player 0's top security card through an
/// `EffectContext` (player-0 effect), then drains the effect queue so the
/// `OnOwnSecurityRemoved` observers dispatch — the medusamon.rs idiom.
fn fire_own_security_removed(runner: &mut DebugRunner) {
    {
        let mut ctx = EffectContext::new(&mut runner.game, CardHandle(0), None, 0);
        assert!(
            ctx.trash_top_security(0),
            "player 0 had a top security card to trash"
        );
    }
    runner.game.drain_effect_queue();
}

/// Accept the outer optional activation prompt ("by suspending this Tamer").
/// Mirrors the BT23-079 Eri Karan idiom.
fn accept_optional_activation(runner: &mut DebugRunner) {
    let action = runner
        .game
        .pending_selection
        .as_ref()
        .expect("outer optional activation prompt present")
        .valid_action_ids[0];
    let _ = runner.game.resolve_selection(0, action);
    runner.game.drain_effect_queue();
}

#[test]
fn bt15_084_security_removed_installs_optional_activation_prompt() {
    let mut runner = base().security(0, &["PAD"; 3]).memory(5).start();
    let _kari = runner.place_on_field(0, CARD_ID, Some(0));
    runner.place_on_field(1, "OPP-A", Some(0));

    fire_own_security_removed(&mut runner);

    assert!(
        runner.pending_is_optional(),
        "'by suspending this Tamer' surfaces an opt-in (declinable) activation prompt"
    );
}

#[test]
fn bt15_084_security_removed_accept_suspends_self_and_applies_security_attack_minus_1() {
    let mut runner = base().security(0, &["PAD"; 3]).memory(5).start();
    let kari = runner.place_on_field(0, CARD_ID, Some(0));
    let opp_a = runner.place_on_field(1, "OPP-A", Some(0));
    let opp_b = runner.place_on_field(1, "OPP-B", Some(0));
    assert!(!runner.game.players[0].battle_area[kari.index as usize].is_suspended);

    fire_own_security_removed(&mut runner);
    accept_optional_activation(&mut runner);

    // After accepting, Kari has paid the suspend cost and the mandatory
    // opponent-Digimon selection installs.
    assert!(
        runner.game.players[0].battle_area[kari.index as usize].is_suspended,
        "suspending Kari is the activation cost"
    );
    let view = runner
        .pending_selection_view()
        .expect("opponent Digimon selection installs after paying the cost");
    assert_eq!(view.kind, SelectionKind::OppField);
    let mut expected = vec![encode_permanent(opp_a), encode_permanent(opp_b)];
    expected.sort();
    let mut got = view.valid_action_ids.clone();
    got.sort();
    assert_eq!(got, expected, "both opponent Digimon are legal Security A. -1 targets");

    runner
        .execute_action(view.selecting_player, encode_permanent(opp_a))
        .expect("choose opponent Digimon");
    runner.auto_resolve().expect("finish security-removed effect");

    assert_eq!(
        runner
            .game
            .modifiers
            .sum(opp_a, ModifierType::SecurityAttackChange),
        -1,
        "chosen opponent Digimon gets Security A. -1"
    );
    assert_eq!(
        runner
            .game
            .modifiers
            .sum(opp_b, ModifierType::SecurityAttackChange),
        0,
        "the unchosen opponent Digimon is unaffected"
    );
}

#[test]
fn bt15_084_security_removed_decline_does_nothing() {
    let mut runner = base().security(0, &["PAD"; 3]).memory(5).start();
    let kari = runner.place_on_field(0, CARD_ID, Some(0));
    let opp = runner.place_on_field(1, "OPP-A", Some(0));

    fire_own_security_removed(&mut runner);
    assert!(runner.pending_is_optional(), "optional prompt appears");
    runner
        .execute_action(0, digimon_engine::action::space::PASS)
        .expect("decline the optional activation");
    let _ = runner.auto_resolve();

    assert!(
        !runner.game.players[0].battle_area[kari.index as usize].is_suspended,
        "declining leaves Kari unsuspended"
    );
    assert_eq!(
        runner
            .game
            .modifiers
            .sum(opp, ModifierType::SecurityAttackChange),
        0,
        "declining applies no Security A. -1"
    );
}

#[test]
fn bt15_084_security_removed_with_kari_already_suspended_installs_no_prompt() {
    // NEGATIVE: the suspend-self activation cost can't be paid when Kari is
    // already suspended, so the observer installs no selection.
    let mut runner = base().security(0, &["PAD"; 3]).memory(5).start();
    let kari = runner.place_on_field(0, CARD_ID, Some(0));
    runner.place_on_field(1, "OPP-A", Some(0));
    runner.game.suspend(kari);

    fire_own_security_removed(&mut runner);

    assert!(
        runner.pending_selection().is_none(),
        "already-suspended Kari cannot pay the suspend cost → no prompt"
    );
}

#[test]
fn bt15_084_security_removed_with_no_opponent_digimon_installs_no_prompt() {
    // NEGATIVE: no opponent Digimon to debuff → no selection.
    let mut runner = base().security(0, &["PAD"; 3]).memory(5).start();
    runner.place_on_field(0, CARD_ID, Some(0));

    fire_own_security_removed(&mut runner);

    assert!(
        runner.pending_selection().is_none(),
        "no opponent Digimon → no Security A. -1 prompt"
    );
}

// ─── Gap-routed clause (left stubbed) ────────────────────────────────────────

#[ignore = "pending: G-DSL-ON-DISCARD-SECURITY-TRIGGER — needs observer for this card being trashed from security by an effect"]
#[test]
fn bt15_084_when_trashed_from_security_applies_security_attack_minus() {}
