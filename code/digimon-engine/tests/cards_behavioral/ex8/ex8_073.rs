//! EX8-073 Gallantmon (X Antibody) — Digimon, Lv.6, Red/Blue, Cost 12, DP 12000.
//! Traits: Holy Warrior / X Antibody / Royal Knight. Attribute: Virus.
//!
//! # Card text (card image — verbatim)
//!
//! ```text
//! Digivolve: [Gallantmon]: Cost 1
//! [When Digivolving] [When Attacking] If [Gallantmon] or [X Antibody] is in
//!   this Digimon's digivolution cards, this Digimon gets +4000 DP and 1 of
//!   your opponent's Digimon gets -4000 DP until the end of their turn.
//! [When Digivolving] [End of Attack] [Once Per Turn] Delete 1 of your
//!   opponent's Digimon with 10000 DP or less. If this didn't delete, trash
//!   your opponent's top security card and this Digimon unsuspends.
//! [All Turns] While you have 0 or less memory, this Digimon isn't affected
//!   by the effects of your opponent's Digimon.
//! ```
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/EX8/Red/EX8_073.cs — the [All Turns]
//! clause is a continuous CanNotAffectedClass gated on MemoryForPlayer <= 0
//! with SkillCondition = IsOpponentEffect && IsDigimonEffect. It is the
//! load-bearing clause for judge-quiz Q15.
//!
//! # DSL YAML
//! code/digimon-engine/cards/ex8/EX8-073.yaml

#![allow(dead_code, unused_imports)]

use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardKind, EffectSourceKind, EffectTiming, ModifierType};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::TriggerSource;

use super::super::dsl_card_data::compiled;

const CARD_ID: &str = "EX8-073";

fn base_runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX8-073 in embedded DSL pack")
        .add_card({
            let mut c = make_test_card("GALLANT-SRC", "Gallantmon");
            c.card_kind = CardKind::Digimon;
            c.level = Some(6);
            c.dp = Some(11000);
            c
        })
        .add_card({
            let mut c = make_test_card("PLAIN-SRC", "PlainBase");
            c.card_kind = CardKind::Digimon;
            c.level = Some(5);
            c.dp = Some(8000);
            c
        })
        .add_card({
            let mut c = make_test_card("OPP-MID", "OppMid");
            c.card_kind = CardKind::Digimon;
            c.level = Some(5);
            c.dp = Some(10000);
            c
        })
        .add_card({
            let mut c = make_test_card("OPP-HUGE", "OppHuge");
            c.card_kind = CardKind::Digimon;
            c.level = Some(6);
            c.dp = Some(13000);
            c
        })
        .add_card({
            // Big enough that even after clause 1's -4000 it stays above the
            // 10000-DP delete ceiling of clause 2 (15000 - 4000 = 11000).
            let mut c = make_test_card("OPP-GIANT", "OppGiant");
            c.card_kind = CardKind::Digimon;
            c.level = Some(7);
            c.dp = Some(15000);
            c
        })
        .add_card({
            let mut c = make_test_card("SEC", "Sec");
            c.card_kind = CardKind::Digimon;
            c
        })
        .add_card(make_test_card("FILLER", "Filler"))
        .deck(0, &["FILLER"; 5])
        .deck(1, &["FILLER"; 5])
        .security(1, &["SEC", "SEC"])
        .memory(5)
        .start()
}

fn fire(r: &mut DebugRunner, timing: EffectTiming, handle: PermanentHandle) {
    r.game
        .enqueue_triggered(timing, TriggerSource::Permanent(handle));
    r.game.drain_effect_queue();
}

// ════════════════════════════════════════════════════════════════════════════
// Section 1 — structural
// ════════════════════════════════════════════════════════════════════════════

#[test]
fn ex8_073_compiles_from_dsl_pack() {
    let card = compiled(CARD_ID);
    assert_eq!(card.level, Some(6));
    assert_eq!(card.cost, Some(12));
    assert_eq!(card.dp, Some(12000));
    assert!(card.traits.iter().any(|t| t == "X Antibody"));
}

// ════════════════════════════════════════════════════════════════════════════
// Section 2 — [WD][WA] source-gated ±4000 DP swing
// ════════════════════════════════════════════════════════════════════════════

/// POSITIVE: with a [Gallantmon] digivolution source, +4000 to self and
/// -4000 to a selected opponent Digimon, both until end of their turn.
/// (The opponent Digimon is 15000 DP so the OTHER [WD] clause — delete
/// <=10000 — cannot remove it even after the -4000 lands, and falls to its
/// security-trash branch instead.)
#[test]
fn ex8_073_dp_swing_with_gallantmon_source() {
    let mut r = base_runner();
    let gx = r.place_stack(0, &["GALLANT-SRC", CARD_ID]);
    let opp = r.place_on_field(1, "OPP-GIANT", Some(0));

    fire(&mut r, EffectTiming::WhenDigivolving, gx);
    // Two [When Digivolving] clauses are pending ⇒ a TriggerOrder pick comes
    // first; either order works for this scenario.
    let view = r
        .pending_selection_view()
        .expect("a trigger-order / target selection must surface");
    r.execute_action(0, view.valid_action_ids[0]).unwrap();
    let _ = r.auto_resolve();

    assert_eq!(
        r.battle_area_size(1),
        1,
        "the 15000-DP Digimon survives the delete clause even after -4000"
    );
    assert_eq!(r.game.modifiers.sum(gx, ModifierType::ChangeDp), 4000);
    assert_eq!(
        r.game.modifiers.sum(opp, ModifierType::ChangeDp),
        -4000,
        "the selected opponent Digimon gets -4000 until the end of their turn"
    );
}

/// The +4000 still applies with NO opponent Digimon (the -4000 pick is
/// skipped).
#[test]
fn ex8_073_self_buff_without_opponent_digimon() {
    let mut r = base_runner();
    let gx = r.place_stack(0, &["GALLANT-SRC", CARD_ID]);
    fire(&mut r, EffectTiming::WhenDigivolving, gx);
    let _ = r.auto_resolve();
    assert_eq!(r.game.modifiers.sum(gx, ModifierType::ChangeDp), 4000);
}

/// NEGATIVE: without [Gallantmon]/[X Antibody] in the digivolution cards the
/// clause does not fire.
#[test]
fn ex8_073_dp_swing_gated_on_sources() {
    let mut r = base_runner();
    let gx = r.place_stack(0, &["PLAIN-SRC", CARD_ID]);
    let opp = r.place_on_field(1, "OPP-MID", Some(0));
    fire(&mut r, EffectTiming::WhenDigivolving, gx);
    let _ = r.auto_resolve();
    assert_eq!(
        r.game.modifiers.sum(gx, ModifierType::ChangeDp),
        0,
        "no [Gallantmon]/[X Antibody] source ⇒ the clause is gated off"
    );
    assert_eq!(r.game.modifiers.sum(opp, ModifierType::ChangeDp), 0);
}

// ════════════════════════════════════════════════════════════════════════════
// Section 3 — [WD][EoA][OPT] delete <=10000 / security-trash + unsuspend
// ════════════════════════════════════════════════════════════════════════════

/// POSITIVE delete branch: a 10000-DP opponent Digimon is deleted; security
/// untouched; no unsuspend.
#[test]
fn ex8_073_deletes_10000_or_less() {
    let mut r = base_runner();
    let gx = r.place_on_field(0, CARD_ID, Some(0));
    let _opp = r.place_on_field(1, "OPP-MID", Some(0));
    let sec_before = r.security_count(1);

    fire(&mut r, EffectTiming::WhenDigivolving, gx);
    let view = r
        .pending_selection_view()
        .expect("mandatory delete pick when a <=10000 DP target exists");
    r.execute_action(0, view.valid_action_ids[0]).unwrap();
    let _ = r.auto_resolve();

    assert_eq!(r.battle_area_size(1), 0);
    assert_eq!(
        r.security_count(1),
        sec_before,
        "deleted ⇒ no security trash"
    );
}

/// FALLBACK branch: only a 13000-DP Digimon ⇒ trash the opponent's top
/// security card and unsuspend this Digimon.
#[test]
fn ex8_073_security_trash_and_unsuspend_when_nothing_deleted() {
    let mut r = base_runner();
    let gx = r.place_on_field(0, CARD_ID, Some(0));
    r.game.players[0].battle_area[gx.index as usize].is_suspended = true;
    let _opp = r.place_on_field(1, "OPP-HUGE", Some(0));
    let sec_before = r.security_count(1);

    fire(&mut r, EffectTiming::WhenDigivolving, gx);
    let _ = r.auto_resolve();

    assert_eq!(
        r.security_count(1),
        sec_before - 1,
        "didn't delete ⇒ the opponent's top security card is trashed"
    );
    assert!(
        !r.game.players[0].battle_area[gx.index as usize].is_suspended,
        "didn't delete ⇒ this Digimon unsuspends"
    );
}

/// Shared [Once Per Turn]: after the [When Digivolving] firing, the
/// [End of Attack] firing in the same turn is locked out.
#[test]
fn ex8_073_delete_clause_shared_once_per_turn() {
    let mut r = base_runner();
    let gx = r.place_on_field(0, CARD_ID, Some(0));
    let _opp = r.place_on_field(1, "OPP-HUGE", Some(0));
    let sec_before = r.security_count(1);

    fire(&mut r, EffectTiming::WhenDigivolving, gx);
    let _ = r.auto_resolve();
    assert_eq!(r.security_count(1), sec_before - 1);

    fire(&mut r, EffectTiming::EndOfAttack, gx);
    let _ = r.auto_resolve();
    assert_eq!(
        r.security_count(1),
        sec_before - 1,
        "[Once Per Turn] is SHARED across [WD] and [End of Attack]"
    );
}

// ════════════════════════════════════════════════════════════════════════════
// Section 4 — [All Turns] memory-gated opponent-DIGIMON-effect immunity
// ════════════════════════════════════════════════════════════════════════════

/// POSITIVE: owner at 0-or-less memory ⇒ unaffected by opponent DIGIMON
/// effects — but still affected by opponent Option/Tamer effects and by its
/// own controller's effects (DCGO SkillCondition: IsOpponentEffect &&
/// IsDigimonEffect).
#[test]
fn ex8_073_immunity_scope_while_memory_zero_or_less() {
    let mut r = base_runner();
    r.set_first_player(1); // P1's turn; gauge >= 0 means P0 has <= 0 memory
    let gx = r.place_on_field(0, CARD_ID, Some(0));
    r.game.set_memory(0);
    r.game.tick_declarative_effects();

    assert!(
        r.game
            .permanent_is_unaffected_by_effect(gx, 1, EffectSourceKind::Digimon),
        "owner at 0 memory ⇒ unaffected by opponent Digimon effects"
    );
    assert!(
        !r.game
            .permanent_is_unaffected_by_effect(gx, 1, EffectSourceKind::Option),
        "the immunity is DIGIMON-effect-scoped — Option effects still apply"
    );
    assert!(
        !r.game
            .permanent_is_unaffected_by_effect(gx, 1, EffectSourceKind::Tamer),
        "Tamer effects still apply"
    );
    assert!(
        !r.game
            .permanent_is_unaffected_by_effect(gx, 0, EffectSourceKind::Digimon),
        "own-controller effects still apply"
    );
}

/// The immunity holds on EITHER turn ([All Turns]) as long as the owner's
/// memory is 0 or less.
#[test]
fn ex8_073_immunity_is_all_turns() {
    let mut r = base_runner();
    r.set_first_player(0); // the owner's own turn
    let gx = r.place_on_field(0, CARD_ID, Some(0));
    r.game.set_memory(0); // own turn at gauge 0 ⇒ owner has 0 memory
    r.game.tick_declarative_effects();
    assert!(
        r.game
            .permanent_is_unaffected_by_effect(gx, 1, EffectSourceKind::Digimon),
        "[All Turns]: the gate is the memory, not the turn"
    );
}

/// NEGATIVE: with positive owner memory the immunity is off.
#[test]
fn ex8_073_no_immunity_with_positive_memory() {
    let mut r = base_runner();
    r.set_first_player(1);
    let gx = r.place_on_field(0, CARD_ID, Some(0));
    // Gauge at -3 on P1's turn ⇒ P0 (the owner) has 3 memory.
    r.game.set_memory(-3);
    r.game.tick_declarative_effects();
    assert!(
        !r.game
            .permanent_is_unaffected_by_effect(gx, 1, EffectSourceKind::Digimon),
        "positive owner memory ⇒ the immunity gate is off"
    );
}
