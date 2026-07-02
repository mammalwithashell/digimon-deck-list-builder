//! BT15-084 Kari Kamiya — Tamer, Yellow, Cost 4.
//!
//! # Card text (card image — authoritative)
//!
//! When an effect trashes this card from the security stack, 1 of your
//! opponent's Digimon gains [Security A. -1] until the end of their turn.
//! [Start of Your Turn] If you have 2 or less memory, set it to 3.
//! [All Turns] When an effect removes cards from your security stack, by
//! suspending this Tamer, 1 of your opponent's Digimon gains [Security A. -1]
//! until the end of their turn.
//! [Security] Play this card without paying the cost.
//!
//! # DCGO C# reference — DCGO/Assets/Scripts/CardEffect/BT15/Yellow/BT15_084.cs
//! - OnDiscardSecurity (this card effect-trashed from security): MANDATORY
//!   (canNoSelect:false), gated on >=1 opponent Digimon → SecurityAttackChange(-1).
//! - OnLoseSecurity (any own security removed): OPTIONAL, suspend-self cost,
//!   then mandatory opponent-Digimon SecurityAttackChange(-1).
//! - OnStartTurn: set memory to 3 if <= 2. SecuritySkill: play self free.
//!
//! Both [All Turns] suspend-cost AND [Security-trash] clauses are implemented
//! and tested here (merged 2026-06-15).

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_dsl::compiled::{CompiledClause, CompiledScope, CompiledTiming};
use digimon_engine::action::space::{encode_attack, PASS};
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardHandle;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::effect_context::EffectContext;
use digimon_engine::enums::{CardColor, CardKind, ModifierType, PlayerId};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::SelectionKind;

const CARD_ID: &str = "BT15-084";

// An OnPlay card that trashes the controller's top security by effect — the
// event both BT15-084 security clauses key on (OnDiscardSecurity when Kari is
// the trashed card; OnOwnSecurityRemoved for the broader observer).
const TRASHER_YAML: &str = r#"
card: DSL-SEC-TRASHER
name: Sec Trasher
kind: digimon
level: 4
color: [yellow]
cost: 0
dp: 3000
effects:
  - when: on_play
    summary: "[On Play] Trash your top security card"
    process:
      - trash_top_security: { of: you }
"#;

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
        .from_dsl_yaml(TRASHER_YAML)
        .expect("inline trasher fixture compiles")
        .add_card(make_test_card("PAD", "PAD"))
        .add_card(opp_digimon("OPP-A"))
        .add_card(opp_digimon("OPP-B"))
        .deck(0, &["PAD"; 12])
        .deck(1, &["PAD"; 12])
}

fn encode_permanent(handle: PermanentHandle) -> u16 {
    encode_attack(0, handle.index as u16)
}

/// Drive a real "an effect removes a card from your security stack" event so the
/// own-security-removed observer fires (trashes player 0's top security via an
/// EffectContext, then drains the queue).
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

// ─── Structural ──────────────────────────────────────────────────────────────

#[test]
fn bt15_084_has_all_four_clauses() {
    let runner = base().memory(5).start();
    let card = runner.compiled_card(CARD_ID).expect("compiled card");

    let has = |f: &dyn Fn(&digimon_dsl::compiled::CompiledTriggeredClause) -> bool| {
        card.effects
            .iter()
            .any(|c| matches!(c, CompiledClause::Triggered(t) if f(t)))
    };
    assert!(
        has(&|t| t.when.contains(&CompiledTiming::StartOfYourTurn)),
        "must have start-of-turn memory floor"
    );
    assert!(
        has(&|t| t.when.contains(&CompiledTiming::OnSecurity)),
        "must have a Security play clause"
    );
    assert!(
        has(&|t| t.when.contains(&CompiledTiming::OnOwnSecurityRemoved)),
        "must have an own-security-removed (suspend-cost) observer"
    );
    assert!(
        has(&|t| t.scope == CompiledScope::Security
            && t.when.contains(&CompiledTiming::OnDiscardSecurity)),
        "must have a security-scope OnDiscardSecurity clause"
    );
}

// ─── [All Turns] own-security-removed → suspend self → opp Security A. -1 ─────

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
    assert_eq!(
        got, expected,
        "both opponent Digimon are legal Security A. -1 targets"
    );

    runner
        .execute_action(view.selecting_player, encode_permanent(opp_a))
        .expect("choose opponent Digimon");
    runner
        .auto_resolve()
        .expect("finish security-removed effect");

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
        .execute_action(0, PASS)
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
    let mut runner = base().security(0, &["PAD"; 3]).memory(5).start();
    runner.place_on_field(0, CARD_ID, Some(0));

    fire_own_security_removed(&mut runner);

    assert!(
        runner.pending_selection().is_none(),
        "no opponent Digimon → no Security A. -1 prompt"
    );
}

// ─── [Security-trash] this card effect-trashed from security → opp Sec A. -1 ──

/// When Kari (top security) is trashed BY an effect and the opponent controls a
/// Digimon, the on_discard_security clause installs a MANDATORY opponent-Digimon
/// prompt; resolving it applies Security A. -1.
#[test]
fn bt15_084_when_trashed_from_security_applies_security_attack_minus() {
    let mut runner = base()
        .hand(0, &["DSL-SEC-TRASHER"])
        .security(0, &["PAD", "PAD", CARD_ID])
        .memory(8)
        .start();

    let opp = runner.place_on_field(1, "OPP-A", Some(0));

    runner.play(0, 0).expect("play the trasher");

    let view = runner
        .pending_selection_view()
        .expect("effect-trashing Kari from security must install a target prompt");
    assert_eq!(
        view.kind,
        SelectionKind::OppField,
        "select 1 opponent Digimon"
    );
    assert!(
        !runner.pending_is_optional(),
        "the [Security-trash] effect is mandatory (DCGO canNoSelect:false)"
    );

    runner
        .execute_action(view.selecting_player, encode_permanent(opp))
        .expect("pick the opponent Digimon");
    runner.auto_resolve().expect("resolve");

    assert_eq!(
        runner
            .game
            .modifiers
            .sum(opp, ModifierType::SecurityAttackChange),
        -1,
        "chosen opponent Digimon gets Security A. -1"
    );
}

/// Kari lands in trash after the effect-trash and the security stack shrinks.
#[test]
fn bt15_084_effect_trash_moves_kari_to_trash() {
    let mut runner = base()
        .hand(0, &["DSL-SEC-TRASHER"])
        .security(0, &["PAD", "PAD", CARD_ID])
        .memory(8)
        .start();
    let _opp = runner.place_on_field(1, "OPP-A", Some(0));
    let sec_before = runner.security_count(0);

    runner.play(0, 0).expect("play the trasher");
    runner.auto_resolve().ok();

    assert_eq!(
        runner.security_count(0),
        sec_before - 1,
        "the effect-trashed Kari leaves the security stack"
    );
    assert!(
        runner.game.players[0]
            .trash
            .iter()
            .any(|c| c.card_id(&runner.game.card_data) == CARD_ID),
        "Kari must be in the controller's trash after the effect-trash"
    );
}

/// Negative: the on-discard clause is gated on >=1 opponent Digimon.
#[test]
fn bt15_084_discard_no_prompt_when_opponent_has_no_digimon() {
    let mut runner = base()
        .hand(0, &["DSL-SEC-TRASHER"])
        .security(0, &["PAD", "PAD", CARD_ID])
        .memory(8)
        .start();

    runner.play(0, 0).expect("play the trasher");

    assert!(
        runner.pending_selection().is_none(),
        "with no opponent Digimon, the on-discard Security A. -1 clause installs no prompt"
    );
}
