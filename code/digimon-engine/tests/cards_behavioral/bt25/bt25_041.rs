//! BT25-041 Murasamemon — Digimon, Lv.5, Yellow, DP 7000, Cost 7.
//! Traits: Beastkin, Glowing Dawn, BEATBREAK.
//!
//! # Card text (data/cards.json, confirmed vs DCGO)
//! <Alliance>.
//! [When Digivolving] [When Attacking] [Once Per Turn] If it's your turn, by
//!   adding your top security card to the hand or trashing the bottom face-down
//!   card under any of your Tamers, you may play or use 1 [Glowing Dawn] card
//!   from your hand with the cost reduced by 3.            <-- BLOCKED (omitted)
//! Inherited: [End of Attack] [Once Per Turn] By trashing the bottom face-down
//!   card from under any of your Tamers, this [Glowing Dawn] Digimon unsuspends.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT25/Yellow/BT25_041.cs
//!
//! # Patterns this test covers (RUST_DSL_TEST_API.md §4.3)
//! - H10 Alliance (grant)
//! - alt-digivolve from Glowing Dawn Lv.4
//! - inherited End-of-Attack OPT: trash-FD-under-Tamer (process cost) → unsuspend self
//!
//! # Verdict — PARTIAL
//! The main [WD][WA][OPT] clause (pay one of two alternative costs — add top
//! security to hand OR trash a face-down card under a Tamer — then play/use a
//! [Glowing Dawn] card from hand at -3) is BLOCKED: the trash half is the
//! interactive-pay_cost family (G-COST-REDUCTION-INTERACTIVE-PAY-COST) and the
//! "pay one of two costs to unlock a cost-reduced play/use" composite has no DSL
//! verb. Omitted from the YAML. Alliance, the Glowing Dawn alt-digivolve, and
//! the inherited unsuspend (which uses the face-down trash as a PROCESS cost,
//! not a cost_reduction pay_cost) are IMPLEMENTED.

#![allow(dead_code)]

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledCardKind, CompiledClause, CompiledDeclarativeClause,
    CompiledScope, CompiledTiming,
};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming};
use digimon_engine::selection::TriggerSource;

use crate::dsl_card_data::{card_data_from_compiled, compiled};

const CARD_ID: &str = "BT25-041";

fn murasamemon() -> CardData {
    card_data_from_compiled(CARD_ID)
}

fn make_tamer(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Tamer;
    c.level = None;
    c.dp = None;
    c.play_cost = 3;
    c.colors = vec![CardColor::Yellow];
    c
}

fn make_filler(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Yellow];
    c.level = Some(3);
    c.dp = Some(3000);
    c.play_cost = 3;
    c
}

/// A Glowing Dawn HOST Digimon — Murasamemon's inherited [End of Attack] clause
/// applies to this host when Murasamemon is a digivolution source beneath it.
fn make_glowing_dawn_host(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Yellow];
    c.level = Some(6);
    c.dp = Some(9000);
    c.play_cost = 6;
    c.traits = vec!["Glowing Dawn".to_string()];
    c
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 1 — Structural
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn bt25_041_compiles_as_digimon() {
    let card = compiled(CARD_ID);
    assert_eq!(card.card, CARD_ID);
    assert_eq!(card.kind, CompiledCardKind::Digimon);
    assert_eq!(card.cost, Some(7));
    assert_eq!(card.dp, Some(7000));
}

#[test]
fn bt25_041_has_alliance_alt_digivolve_and_inherited_end_of_attack() {
    let card = compiled(CARD_ID);

    // Alliance: a non-inherited grant_keyword declarative.
    let has_alliance = card.effects.iter().any(|c| {
        matches!(
            c,
            CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword { scope, .. })
                if *scope == CompiledScope::FaceUp
        )
    });
    assert!(has_alliance, "<Alliance> grant present (face-up)");

    // Alt-digivolve (Glowing Dawn Lv.4).
    assert!(
        card.alt_paths
            .iter()
            .any(|p| matches!(p.kind, CompiledAltPathKind::Digivolve)),
        "alt-digivolve path present"
    );

    // Inherited End-of-Attack OPT unsuspend clause.
    let inherited_eoa = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| t.when == vec![CompiledTiming::EndOfAttack])
        .expect("inherited end_of_attack clause present");
    assert_eq!(inherited_eoa.scope, CompiledScope::Inherited);
    assert!(inherited_eoa.once_per_turn, "once per turn");
    assert!(inherited_eoa.optional, "'By trashing …' → optional");
}

#[test]
fn bt25_041_blocked_main_clause_is_omitted() {
    // The main [WD][WA] clause is omitted (no triggered clause with that timing
    // beyond the inherited end_of_attack). Assert there is no WhenDigivolving/
    // WhenAttacking face-up triggered clause.
    let card = compiled(CARD_ID);
    let has_main = card.effects.iter().any(|c| match c {
        CompiledClause::Triggered(t) => {
            t.scope == CompiledScope::FaceUp
                && (t.when.contains(&CompiledTiming::WhenAttacking)
                    || t.when.contains(&CompiledTiming::WhenDigivolving))
        }
        _ => false,
    });
    assert!(
        !has_main,
        "the BLOCKED main [WD][WA] play/use-reduced clause must stay omitted"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 2 — Inherited [End of Attack][OPT]: trash FD → unsuspend self
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn bt25_041_inherited_unsuspends_host_after_attack_by_trashing_face_down() {
    let mut runner = DebugRunner::builder()
        .add_card(murasamemon())
        .add_card(make_glowing_dawn_host("HOST"))
        .add_card(make_tamer("TAMER"))
        .add_card(make_filler("STASH"))
        .deck(0, &["STASH"; 3])
        .deck(1, &["STASH"; 3])
        .memory(10)
        .start();

    // Stack: [Murasamemon (inherited source), HOST (Glowing Dawn top)].
    let host = runner.place_stack(0, &[CARD_ID, "HOST"]);
    // Suspend the host (as if it just attacked).
    runner.game.players[0].battle_area[host.index as usize].is_suspended = true;
    // Tamer + face-down stash beneath it (the unsuspend cost).
    let tamer = runner.place_stack(0, &["STASH", "TAMER"]);
    runner.game.players[0].battle_area[tamer.index as usize].card_sources[0].face_down = true;

    let trash_before = runner.trash_size(0);

    // Fire the inherited End-of-Attack clause on the HOST (bt21_021 idiom).
    runner
        .game
        .enqueue_triggered(EffectTiming::EndOfAttack, TriggerSource::Permanent(host));
    runner.game.drain_effect_queue();

    assert!(
        runner.pending_selection().is_some(),
        "the inherited end_of_attack OPT must install a prompt (Glowing Dawn host + face-down stash)"
    );
    let _ = runner.auto_resolve();

    assert!(
        !runner.game.players[0].battle_area[host.index as usize].is_suspended,
        "the Glowing Dawn host must unsuspend after paying the face-down trash cost"
    );
    assert_eq!(
        runner.trash_size(0),
        trash_before + 1,
        "the bottom face-down stash source was trashed as the cost"
    );
}

#[test]
fn bt25_041_inherited_no_unsuspend_when_no_face_down_stash() {
    let mut runner = DebugRunner::builder()
        .add_card(murasamemon())
        .add_card(make_glowing_dawn_host("HOST"))
        .add_card(make_tamer("TAMER"))
        .add_card(make_filler("STASH"))
        .deck(0, &["STASH"; 3])
        .deck(1, &["STASH"; 3])
        .memory(10)
        .start();

    let host = runner.place_stack(0, &[CARD_ID, "HOST"]);
    runner.game.players[0].battle_area[host.index as usize].is_suspended = true;
    // Tamer with NO face-down stash (plain) → cost unpayable.
    runner.place_on_field(0, "TAMER", Some(0));

    runner
        .game
        .enqueue_triggered(EffectTiming::EndOfAttack, TriggerSource::Permanent(host));
    runner.game.drain_effect_queue();
    let _ = runner.auto_resolve();

    assert!(
        runner.game.players[0].battle_area[host.index as usize].is_suspended,
        "with no face-down stash the unsuspend cost is unpayable → host stays suspended"
    );
}
