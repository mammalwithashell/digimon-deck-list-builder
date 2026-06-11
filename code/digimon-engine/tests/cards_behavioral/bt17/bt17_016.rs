//! BT17-016 Gallantmon — Digimon, Lv.6, Red, Cost 11, DP 11000.
//! Traits: Holy Warrior / Royal Knight. Attribute: Virus.
//!
//! # Card text (card image — verbatim)
//!
//! ```text
//! [When Digivolving] [When Attacking] Delete 1 of your opponent's Digimon
//!   with 8000 DP or less. If this effect didn't delete, this Digimon gets
//!   +3000 DP and gains <Blocker> until the end of your opponent's turn.
//! [Your Turn] While you have 0 or less memory, this Digimon isn't affected
//!   by your opponent's effects.
//! ```
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT17/Red/BT17_016.cs — identical
//! WD/WA bodies (select <=8000 → delete; FailureProcess: +3000 DP + Blocker,
//! both UntilOpponentTurnEnd) + a continuous CanNotAffectedClass gated on
//! IsOwnerTurn && MemoryForPlayer <= 0, SkillCondition = IsOpponentEffect
//! (ANY opponent effect).
//!
//! # DSL YAML
//! code/digimon-engine/cards/bt17/BT17-016.yaml

#![allow(dead_code, unused_imports)]

use digimon_dsl::compiled::{CompiledClause, CompiledTiming};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::effect_context::EffectContext;
use digimon_engine::enums::{
    CardColor, CardKind, EffectSourceKind, EffectTiming, Expiry, Keyword, ModifierType,
};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::TriggerSource;

use super::super::dsl_card_data::compiled;

const CARD_ID: &str = "BT17-016";

fn base_runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT17-016 in embedded DSL pack")
        .add_card({
            let mut c = make_test_card("OPP-SMALL", "OppSmall");
            c.card_kind = CardKind::Digimon;
            c.level = Some(5);
            c.dp = Some(8000);
            c
        })
        .add_card({
            let mut c = make_test_card("OPP-BIG", "OppBig");
            c.card_kind = CardKind::Digimon;
            c.level = Some(6);
            c.dp = Some(9000);
            c
        })
        .add_card(make_test_card("FILLER", "Filler"))
        .deck(0, &["FILLER"; 5])
        .deck(1, &["FILLER"; 5])
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
fn bt17_016_compiles_from_dsl_pack() {
    let card = compiled(CARD_ID);
    assert_eq!(card.level, Some(6));
    assert_eq!(card.cost, Some(11));
    assert_eq!(card.dp, Some(11000));
    assert!(card.traits.iter().any(|t| t == "Royal Knight"));
}

/// One clause carries BOTH [When Digivolving] and [When Attacking].
#[test]
fn bt17_016_delete_clause_covers_both_timings() {
    let card = compiled(CARD_ID);
    let both = card
        .effects
        .iter()
        .filter(|c| matches!(
            c,
            CompiledClause::Triggered(t)
                if t.when.contains(&CompiledTiming::WhenDigivolving)
                    && t.when.contains(&CompiledTiming::WhenAttacking)
        ))
        .count();
    assert_eq!(both, 1, "one shared [WD][WA] clause");
}

// ════════════════════════════════════════════════════════════════════════════
// Section 2 — [WD][WA] delete <=8000 or +3000/<Blocker>
// ════════════════════════════════════════════════════════════════════════════

/// POSITIVE delete branch: an 8000-DP opponent Digimon is deleted; the
/// fallback buff must NOT apply.
#[test]
fn bt17_016_when_digivolving_deletes_8000_or_less() {
    let mut r = base_runner();
    let gallant = r.place_on_field(0, CARD_ID, Some(0));
    let _opp = r.place_on_field(1, "OPP-SMALL", Some(0));

    fire(&mut r, EffectTiming::WhenDigivolving, gallant);
    let view = r
        .pending_selection_view()
        .expect("mandatory delete pick when a <=8000 DP target exists");
    assert!(!view.is_optional);
    r.execute_action(0, view.valid_action_ids[0]).unwrap();
    let _ = r.auto_resolve();

    assert_eq!(r.battle_area_size(1), 0, "the 8000-DP Digimon is deleted");
    assert_eq!(
        r.game.modifiers.sum(gallant, ModifierType::ChangeDp),
        0,
        "the effect deleted ⇒ no +3000 DP fallback"
    );
    assert!(!r.game.has_keyword(gallant, Keyword::Blocker));
}

/// FALLBACK branch: no legal target (only a 9000-DP Digimon) ⇒ +3000 DP and
/// <Blocker> until the end of the opponent's turn.
#[test]
fn bt17_016_buffs_self_when_nothing_deleted() {
    let mut r = base_runner();
    let gallant = r.place_on_field(0, CARD_ID, Some(0));
    let opp = r.place_on_field(1, "OPP-BIG", Some(0));

    fire(&mut r, EffectTiming::WhenDigivolving, gallant);
    let _ = r.auto_resolve();

    assert_eq!(
        r.battle_area_size(1),
        1,
        "the 9000-DP Digimon is not a legal target"
    );
    let _ = opp;
    assert_eq!(
        r.game.modifiers.sum(gallant, ModifierType::ChangeDp),
        3000,
        "didn't delete ⇒ +3000 DP"
    );
    assert!(
        r.game.has_keyword(gallant, Keyword::Blocker),
        "didn't delete ⇒ gains <Blocker>"
    );
}

/// FALLBACK also fires with an empty opponent board.
#[test]
fn bt17_016_buffs_self_with_no_opponent_digimon() {
    let mut r = base_runner();
    let gallant = r.place_on_field(0, CARD_ID, Some(0));
    fire(&mut r, EffectTiming::WhenDigivolving, gallant);
    let _ = r.auto_resolve();
    assert_eq!(r.game.modifiers.sum(gallant, ModifierType::ChangeDp), 3000);
    assert!(r.game.has_keyword(gallant, Keyword::Blocker));
}

/// The same clause fires on [When Attacking].
#[test]
fn bt17_016_when_attacking_fires_same_clause() {
    let mut r = base_runner();
    let gallant = r.place_on_field(0, CARD_ID, Some(0));
    let _opp = r.place_on_field(1, "OPP-SMALL", Some(0));

    fire(&mut r, EffectTiming::WhenAttacking, gallant);
    let view = r
        .pending_selection_view()
        .expect("[When Attacking] also surfaces the delete pick");
    r.execute_action(0, view.valid_action_ids[0]).unwrap();
    let _ = r.auto_resolve();
    assert_eq!(r.battle_area_size(1), 0);
}

// ════════════════════════════════════════════════════════════════════════════
// Section 3 — [Your Turn] memory-gated opponent-effect immunity
// ════════════════════════════════════════════════════════════════════════════

/// POSITIVE: on your turn with the gauge at 0 (you have 0 memory), the
/// Digimon is unaffected by opponent effects of EVERY source kind.
#[test]
fn bt17_016_immune_on_own_turn_at_zero_or_less_memory() {
    let mut r = base_runner();
    r.set_first_player(0);
    let gallant = r.place_on_field(0, CARD_ID, Some(0));
    r.game.set_memory(0);
    r.game.tick_declarative_effects();

    for kind in [
        EffectSourceKind::Digimon,
        EffectSourceKind::Tamer,
        EffectSourceKind::Option,
    ] {
        assert!(
            r.game.permanent_is_unaffected_by_effect(gallant, 1, kind),
            "[Your Turn] at <=0 memory: unaffected by opponent {kind:?} effects"
        );
    }
    // ... but still affected by its OWN controller's effects.
    assert!(
        !r.game
            .permanent_is_unaffected_by_effect(gallant, 0, EffectSourceKind::Digimon),
        "the immunity is opponent-scoped only"
    );
}

/// NEGATIVE (memory gate): with positive memory the immunity is off.
#[test]
fn bt17_016_not_immune_with_positive_memory() {
    let mut r = base_runner();
    r.set_first_player(0);
    let gallant = r.place_on_field(0, CARD_ID, Some(0));
    r.game.set_memory(3); // own turn, 3 memory ⇒ gate off
    r.game.tick_declarative_effects();
    assert!(
        !r.game
            .permanent_is_unaffected_by_effect(gallant, 1, EffectSourceKind::Digimon),
        "with positive memory the [Your Turn] immunity must be off"
    );
}

/// NEGATIVE (turn gate): on the OPPONENT's turn the immunity is off even at
/// 0-or-less memory.
#[test]
fn bt17_016_not_immune_on_opponents_turn() {
    let mut r = base_runner();
    r.set_first_player(1); // opponent's turn
    let gallant = r.place_on_field(0, CARD_ID, Some(0));
    // Gauge at 3 for the turn player (P1) ⇒ P0 has -3 memory (<= 0), but it
    // is not P0's turn.
    r.game.set_memory(3);
    r.game.tick_declarative_effects();
    assert!(
        !r.game
            .permanent_is_unaffected_by_effect(gallant, 1, EffectSourceKind::Digimon),
        "[Your Turn] immunity must not apply on the opponent's turn"
    );
}

/// End-to-end: an opponent's deletion effect fizzles against the immune
/// Gallantmon.
#[test]
fn bt17_016_opponent_delete_effect_fizzles_while_immune() {
    let mut r = base_runner();
    r.set_first_player(0);
    let gallant = r.place_on_field(0, CARD_ID, Some(0));
    let opp = r.place_on_field(1, "OPP-SMALL", Some(0));
    r.game.set_memory(0);
    r.game.tick_declarative_effects();

    let opp_card = r.top_card(opp);
    {
        let mut ctx = EffectContext::new(&mut r.game, opp_card, Some(opp), 1);
        ctx.delete_permanent(gallant);
    }
    r.game.drain_effect_queue();
    assert_eq!(
        r.battle_area_size(0),
        1,
        "the opponent's effect deletion must fizzle against the immunity"
    );
}
