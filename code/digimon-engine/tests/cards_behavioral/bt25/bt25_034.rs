//! BT25-034 Angemon — Digimon, Lv.4, Yellow, DP 5000, Cost 5.
//! Traits: Angel, Iliad, TS.
//!
//! # Card text (cards.json — verbatim)
//!
//! When effects trash this card from the security stack, you may play 1 level 4
//! or lower [Angel] or [Iliad] trait card from your hand without paying the
//! cost.
//! <Ascension> (When this Digimon is deleted, you may place this card as the
//! top security card.)
//!
//! Inherited: <Barrier> (When this Digimon would be deleted in battle, by
//! trashing your top security card, it isn't deleted.)
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT25/Yellow/BT25_034.cs
//!
//! - When Trashed (OnDiscardSecurity, CanTriggerOnTrashSelfSecurity): optional
//!   SelectHand PlayForFree over HasPlayCost && Level<=4 && (Angel||Iliad).
//! - Ascension (OnDestroyedAnyone): AscensionSelfEffect.
//! - Barrier (WhenPermanentWouldBeDeleted, inherited): BarrierSelfEffect.
//!
//! # Patterns this test covers (RUST_DSL_TEST_API §4.3)
//! - security-zone self-trigger on OnDiscardSecurity (effect-trash)
//! - play-from-hand free
//! - F-keyword Ascension (own) + Barrier (inherited)

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledDeclarativeClause, CompiledScope, CompiledStep,
    CompiledTiming,
};
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardKind, PlayerId};
use digimon_engine::permanent::PermanentHandle;

const CARD_ID: &str = "BT25-034";

// Inline fixture: an OnPlay card that trashes the controller's top security by
// effect — the trigger BT25-034's security-scope clause keys on.
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

fn make_angel_hand_card(id: &str) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.level = Some(4);
    card.dp = Some(4000);
    card.play_cost = Some(4);
    card.traits = vec!["Angel".to_string()];
    card
}

fn base() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT25-034 YAML parses and compiles")
        .from_dsl_yaml(TRASHER_YAML)
        .expect("inline trasher fixture compiles")
        .add_card(make_test_card("DECK-PAD", "Filler"))
        .add_card(make_angel_hand_card("ANGEL-HAND"))
        .deck(0, &["DECK-PAD"; 12])
        .deck(1, &["DECK-PAD"; 12])
}

// ─── Section 1 — Structural assertions ───────────────────────────────────────

#[test]
fn bt25_034_yaml_has_printed_metadata() {
    let runner = base().start();
    let card = runner
        .compiled_card(CARD_ID)
        .expect("BT25-034 present in pack");
    assert_eq!(card.name, "Angemon");
    assert_eq!(card.level, Some(4));
    assert_eq!(card.dp, Some(5000));
    for t in ["Angel", "Iliad", "TS"] {
        assert!(card.traits.contains(&t.to_string()), "trait {t}");
    }
}

#[test]
fn bt25_034_has_security_scope_discard_clause() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled present");
    let has_sec_clause = card.effects.iter().any(|c| matches!(
        c,
        CompiledClause::Triggered(t)
            if t.scope == CompiledScope::Security
                && t.when.contains(&CompiledTiming::OnDiscardSecurity)
    ));
    assert!(
        has_sec_clause,
        "BT25-034 must have a security-scope OnDiscardSecurity clause"
    );
}

#[test]
fn bt25_034_grants_ascension_and_inherited_barrier() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled present");

    let mut ascension_own = false;
    let mut barrier_inherited = false;
    for c in &card.effects {
        if let CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword {
            keyword,
            scope,
            ..
        }) = c
        {
            if keyword.eq_ignore_ascii_case("Ascension") && *scope == CompiledScope::Own {
                ascension_own = true;
            }
            if keyword.eq_ignore_ascii_case("Barrier") && *scope == CompiledScope::Inherited {
                barrier_inherited = true;
            }
        }
    }
    assert!(ascension_own, "BT25-034 grants own <Ascension>");
    assert!(barrier_inherited, "BT25-034 grants inherited <Barrier>");
}

#[test]
fn bt25_034_security_clause_is_optional() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled present");
    let clause = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t)
                if t.scope == CompiledScope::Security
                    && t.when.contains(&CompiledTiming::OnDiscardSecurity) =>
            {
                Some(t)
            }
            _ => None,
        })
        .expect("security clause present");
    assert!(clause.optional, "the play-free clause is optional (you may)");
}

// ─── Section 2 — Behavior: effect-trash from security triggers play-free ─────

/// Positive: when Angemon (top security) is trashed BY an effect, its
/// security-scope clause installs the optional play-free prompt over the
/// eligible Angel hand card.
#[test]
fn bt25_034_effect_trash_from_security_installs_play_free_prompt() {
    let mut runner = base()
        .hand(0, &["DSL-SEC-TRASHER", "ANGEL-HAND"])
        // Angemon is the TOP security card (last element = top).
        .security(0, &["DECK-PAD", "DECK-PAD", CARD_ID])
        .memory(8)
        .start();

    // Play the trasher → it trashes the controller's top security (Angemon).
    let _t = runner.play(0, 0).expect("play the trasher");

    assert!(
        runner.pending_selection().is_some(),
        "effect-trashing Angemon from security must install its play-free prompt"
    );
    assert!(
        runner.pending_is_optional(),
        "the play-free prompt must be optional (you may play)"
    );
}

/// The trashed Angemon lands in trash regardless of whether the play is taken.
#[test]
fn bt25_034_effect_trash_moves_angemon_to_trash() {
    let mut runner = base()
        .hand(0, &["DSL-SEC-TRASHER"])
        .security(0, &["DECK-PAD", "DECK-PAD", CARD_ID])
        .memory(8)
        .start();

    let sec_before = runner.security_count(0);
    let _t = runner.play(0, 0).expect("play the trasher");
    runner.auto_resolve().ok();

    assert_eq!(
        runner.security_count(0),
        sec_before - 1,
        "the effect-trashed Angemon leaves the security stack"
    );
    assert!(
        runner.game.players[0]
            .trash
            .iter()
            .any(|c| c.card_id(&runner.game.card_data) == CARD_ID),
        "Angemon must be in the controller's trash after the effect-trash"
    );
}
