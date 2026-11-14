//! ST23-07 Armalizamon — Digimon, Lv.4, Green, DP 6000, Cost 5.
//! Traits: Reptile, Glowing Dawn, BEATBREAK. Attribute: Data.
//!
//! # Card text (data/cards.json — verbatim)
//! [On Play] [When Digivolving] If you have 1 or fewer Tamers, you may play 1
//! Tamer card with the [Glowing Dawn] trait from your hand without paying the
//! cost.
//! Inherited: ＜Piercing＞.
//!
//! Printed digivolve box: [Digivolve] Lv.3 w/[Glowing Dawn] trait: Cost 2.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/ST23/Green/ST23_07.cs
//!
//! # Patterns (RUST_DSL_TEST_API §4.3)
//! - conditional (Tamer-count gate) optional free play of a filtered Tamer
//! - H3 inherited <Piercing> keyword grant
//! - alt-path Lv.3 [Glowing Dawn] cost 2

#![allow(dead_code, unused_imports)]

use digimon_dsl::compiled::{
    CompiledCardKind, CompiledClause, CompiledDeclarativeClause, CompiledScope, CompiledTiming,
};
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind};

const CARD_ID: &str = "ST23-07";

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn gd_tamer(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Tamer;
    c.level = None;
    c.dp = None;
    c.play_cost = 3;
    c.colors = vec![CardColor::Green];
    c.traits = vec!["Glowing Dawn".to_string(), "BEATBREAK".to_string()];
    c
}

fn plain_tamer(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Tamer;
    c.level = None;
    c.dp = None;
    c.play_cost = 3;
    c.colors = vec![CardColor::Green];
    c
}

fn filler(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Green];
    c.level = Some(3);
    c.dp = Some(2000);
    c.play_cost = 3;
    c
}

fn push_to_hand(runner: &mut DebugRunner, p: u8, card_id: &str) {
    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .unwrap_or_else(|| panic!("unknown card_id {card_id}"));
    let next_idx = runner.game.next_card_index();
    runner.game.players[p as usize]
        .hand
        .push(CardSource::new(data_idx, p, next_idx));
}

fn base() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("ST23-07 in embedded DSL pack")
        .add_card(gd_tamer("GD-TAMER"))
        .add_card(plain_tamer("PLAIN-TAMER"))
        .add_card(filler("FILLER"))
        .deck(0, &["FILLER"; 5])
        .deck(1, &["FILLER"; 5])
        .memory(8)
        .start()
}

// ─── Section 1 — Structural ──────────────────────────────────────────────────

#[test]
fn st23_07_metadata_and_alt_path() {
    let runner = base();
    let card = runner.compiled_card(CARD_ID).expect("compiled");
    assert_eq!(card.name, "Armalizamon");
    assert_eq!(card.level, Some(4));
    assert_eq!(card.cost, Some(5));
    assert_eq!(card.dp, Some(6000));
    let gd = card.alt_paths.iter().any(|p| {
        p.from
            .as_ref()
            .and_then(|f| f.trait_has.as_deref())
            .map(|t| t == "Glowing Dawn")
            .unwrap_or(false)
    });
    assert!(gd, "Lv.3 [Glowing Dawn] alt-path present");
}

#[test]
fn st23_07_has_op_wd_clause_and_inherited_piercing() {
    let runner = base();
    let card = runner.compiled_card(CARD_ID).expect("compiled");

    let has_op_wd = card.effects.iter().any(|c| {
        matches!(
            c,
            CompiledClause::Triggered(t)
                if t.when.contains(&CompiledTiming::OnPlay)
                    && t.when.contains(&CompiledTiming::WhenDigivolving)
        )
    });
    assert!(has_op_wd, "[On Play][When Digivolving] clause present");

    let has_pierce = card.effects.iter().any(|c| matches!(
        c,
        CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword { keyword, scope, .. })
            if keyword == "Piercing" && *scope == CompiledScope::Inherited
    ));
    assert!(has_pierce, "inherited <Piercing> present");
}

// ─── Section 2 — Behavioral ──────────────────────────────────────────────────

/// POSITIVE: with 0 Tamers on field and a [Glowing Dawn] Tamer in hand, playing
/// ST23-07 offers the free Tamer play; taking it puts the Tamer on the field at
/// no memory cost.
#[test]
fn st23_07_plays_glowing_dawn_tamer_free_when_zero_tamers() {
    let mut runner = base();
    runner.set_first_player(0);
    push_to_hand(&mut runner, 0, "GD-TAMER");

    // Put Armalizamon directly on field and fire its OnPlay.
    let arma = runner.place_on_field(0, CARD_ID, Some(0));
    let mem_before = runner.memory();
    let tamers_before = runner.game.players[0]
        .battle_area
        .iter()
        .filter(|p| p.top_card().card_kind(&runner.game.card_data) == CardKind::Tamer)
        .count();
    runner.fire_on_play(0, arma.index as usize);

    let view = runner
        .pending_selection_view()
        .expect("the optional free-Tamer play must surface");
    assert!(view.is_optional, "'you may play' ⇒ declinable");
    runner.execute_action(0, view.valid_action_ids[0]).unwrap();
    let _ = runner.auto_resolve();

    let tamers_after = runner.game.players[0]
        .battle_area
        .iter()
        .filter(|p| p.top_card().card_kind(&runner.game.card_data) == CardKind::Tamer)
        .count();
    assert_eq!(tamers_after, tamers_before + 1, "the GD Tamer was played");
    assert_eq!(
        runner.memory(),
        mem_before,
        "played without paying the cost"
    );
}

/// DECLINE: declining the optional play leaves the board and memory unchanged.
#[test]
fn st23_07_free_tamer_play_is_declinable() {
    let mut runner = base();
    runner.set_first_player(0);
    push_to_hand(&mut runner, 0, "GD-TAMER");
    let arma = runner.place_on_field(0, CARD_ID, Some(0));
    let mem_before = runner.memory();
    let hand_before = runner.hand_size(0);
    runner.fire_on_play(0, arma.index as usize);

    let view = runner
        .pending_selection_view()
        .expect("optional pick surfaces");
    assert!(view.is_optional);
    runner
        .execute_action(0, digimon_engine::action::space::PASS)
        .expect("decline");
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.hand_size(0),
        hand_before,
        "declined ⇒ Tamer stays in hand"
    );
    assert_eq!(runner.memory(), mem_before, "declined ⇒ no change");
}

/// NEGATIVE (Tamer-count gate): with 2 Tamers already on field, the clause is
/// gated out — no free-play prompt even with a [Glowing Dawn] Tamer in hand.
#[test]
fn st23_07_no_play_when_two_or_more_tamers() {
    let mut runner = base();
    runner.set_first_player(0);
    push_to_hand(&mut runner, 0, "GD-TAMER");
    // Two Tamers already on field → "1 or fewer Tamers" is false.
    runner.place_on_field(0, "PLAIN-TAMER", Some(0));
    runner.place_on_field(0, "PLAIN-TAMER", Some(0));
    let arma = runner.place_on_field(0, CARD_ID, Some(0));
    let hand_before = runner.hand_size(0);
    runner.fire_on_play(0, arma.index as usize);
    let _ = runner.auto_resolve();

    assert!(
        runner.game.pending_selection.is_none(),
        ">1 Tamer ⇒ the clause is gated out, no free-play prompt"
    );
    assert_eq!(runner.hand_size(0), hand_before, "no Tamer played");
}

/// NEGATIVE (trait filter): a non-[Glowing Dawn] Tamer in hand is not a legal
/// pick — clean no-op (no selectable action).
#[test]
fn st23_07_non_glowing_dawn_tamer_is_not_a_legal_pick() {
    let mut runner = base();
    runner.set_first_player(0);
    push_to_hand(&mut runner, 0, "PLAIN-TAMER");
    let arma = runner.place_on_field(0, CARD_ID, Some(0));
    let hand_before = runner.hand_size(0);
    runner.fire_on_play(0, arma.index as usize);
    let _ = runner.auto_resolve();

    // Either no prompt at all, or an optional prompt with no legal target that
    // resolves to a no-op. Either way no Tamer is played.
    assert_eq!(
        runner.hand_size(0),
        hand_before,
        "a non-[Glowing Dawn] Tamer is not playable by this effect"
    );
}

// ─── Section 3 — pick legality ───────────────────────────────────────────────
//
// DCGO ST23_07.cs:54-59 `HasTamerInHand = EqualsTraits("Glowing Dawn") &&
// IsTamer && CanPlayAsNewPermanent(source, false, activateClass)` is the
// SelectHandEffect candidate filter (:64): only a [Glowing Dawn] Tamer that can
// actually be placed is offered — a free frame and no "can't play Tamers by
// effects" lock. CR 4.3 1-3-11-1: a card can be declared for play only if it
// is able to be played (the lock). The frame cap is the engine's
// `rules.field_slots`, mirroring DCGO's frame limit; CR 4.3 3-4-8-2 itself
// sets no battle-area limit.

/// Armalizamon on the field with a [Glowing Dawn] Tamer in hand; fire [On Play]
/// and report whether GD-TAMER is offered at the hand pick.
fn gd_tamer_offered(runner: &mut DebugRunner) -> bool {
    let arma = runner
        .game
        .players[0]
        .battle_area
        .iter()
        .position(|p| p.top_card().card_id(&runner.game.card_data) == CARD_ID)
        .expect("Armalizamon on the field");
    runner.fire_on_play(0, arma);
    let Some(view) = runner.pending_selection_view() else {
        return false;
    };
    let slot = runner.game.players[0]
        .hand
        .iter()
        .position(|c| c.card_id(&runner.game.card_data) == "GD-TAMER")
        .expect("GD-TAMER in hand") as u16;
    [
        digimon_engine::action::space::PLAY_HAND_START + slot,
        digimon_engine::action::space::HAND_EFFECT_START + slot,
    ]
    .iter()
    .any(|a| view.valid_action_ids.contains(a))
}

fn armalizamon_with_gd_tamer_in_hand() -> DebugRunner {
    let mut runner = base();
    runner.set_first_player(0);
    push_to_hand(&mut runner, 0, "GD-TAMER");
    runner.place_on_field(0, CARD_ID, Some(0));
    runner
}

fn tamer_on_field(runner: &DebugRunner) -> bool {
    runner.game.players[0]
        .battle_area
        .iter()
        .any(|p| p.top_card().card_id(&runner.game.card_data) == "GD-TAMER")
}

/// "You can't play Tamers by effects": the [Glowing Dawn] Tamer is not offered.
#[test]
fn st23_07_gd_tamer_not_offered_under_tamer_effect_play_prohibition() {
    use digimon_engine::enums::{Expiry, ModifierType};
    use digimon_engine::modifiers::PlayerModifierEntry;
    let mut runner = armalizamon_with_gd_tamer_in_hand();
    runner.game.modifiers.add_player_modifier(
        0,
        PlayerModifierEntry::simple(ModifierType::CannotPlayTamerByEffect, 0, Expiry::Permanent, None, 1),
    );
    assert!(!gd_tamer_offered(&mut runner), "Tamer effect plays are locked");
    let _ = runner.auto_resolve();
    assert!(!tamer_on_field(&runner));
}

/// No free battle-area frame: the [Glowing Dawn] Tamer is not offered.
/// The frame cap is the engine's `rules.field_slots` (mirroring DCGO's frame
/// limit); CR 4.3 3-4-8-2 itself sets no battle-area limit.
#[test]
fn st23_07_gd_tamer_not_offered_when_battle_area_full() {
    let mut runner = armalizamon_with_gd_tamer_in_hand();
    runner.game.rules.field_slots = runner.game.players[0].battle_area.len() as u8;
    assert!(!gd_tamer_offered(&mut runner), "no free frame for the Tamer");
    let _ = runner.auto_resolve();
    assert!(!tamer_on_field(&runner));
}

/// Regression guard: with a free frame and no lock the Tamer is offered.
#[test]
fn st23_07_gd_tamer_offered_with_free_frame_and_no_lock() {
    let mut runner = armalizamon_with_gd_tamer_in_hand();
    assert!(gd_tamer_offered(&mut runner));
}
