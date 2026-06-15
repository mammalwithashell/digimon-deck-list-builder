//! EX4-065 Trident Gaia — Option, Red/Green, Cost 8.
//!
//! # Card text (cards.json / card image)
//!
//! ```text
//! [Main] Delete 1 of your opponent's Digimon with the highest DP. If a Digimon
//!   with 13000 DP or more is deleted by this effect, trash the top card of
//!   your opponent's security stack.
//! [Security] Activate this card's [Main] effects.
//! ```
//!
//! # Implemented slice
//! - [Main] / [Security] delete 1 of the opponent's highest-DP Digimon
//!   (`highest_dp` aggregate selector gates the legal target set).
//!
//! # Gap-routed
//! - The conditional tail "If a Digimon with 13000 DP or more is deleted by
//!   this effect, trash the opponent's top security card" needs a
//!   deleted-permanent DP threshold on the effect-deleted result payload. The
//!   `effect_deleted_any_opponent_digimon` predicate is a bare bool and the
//!   result log (`EffectResultLog.deleted`) stores only `PermanentHandle`s —
//!   no DP snapshot — and after deletion the carrier has moved to trash, so a
//!   live DP read is unavailable. G-HIGHEST-DP-DELETE-WITH-EFFECT-PAYLOAD.

use digimon_dsl::compiled::{CompiledClause, CompiledTiming};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind};

const CARD_ID: &str = "EX4-065";

fn opp_digimon(id: &str, dp: i32) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(6);
    c.dp = Some(dp);
    c.colors = vec![CardColor::Purple];
    c
}

// ════════════════════════════════════════════════════════════════════════════
// Structural
// ════════════════════════════════════════════════════════════════════════════

#[test]
fn ex4_065_has_main_and_security_shells() {
    let runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX4-065 must load from embedded DSL pack")
        .memory(8)
        .start();
    let card = runner.compiled_card(CARD_ID).expect("compiled card");

    assert!(card.effects.iter().any(|clause| matches!(
        clause,
        CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::MainFromHand)
    )));
    assert!(card.effects.iter().any(|clause| matches!(
        clause,
        CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::OnSecurity)
    )));
}

// ════════════════════════════════════════════════════════════════════════════
// [Main] delete 1 opponent Digimon with the highest DP
// ════════════════════════════════════════════════════════════════════════════

/// POSITIVE: only the highest-DP opponent Digimon is a legal target, and it is
/// the one deleted.
#[test]
fn ex4_065_main_deletes_highest_dp_opponent_digimon() {
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX4-065 loads")
        .add_card(opp_digimon("OPP-LOW", 5000))
        .add_card(opp_digimon("OPP-HIGH", 14000))
        .add_card(make_test_card("FILL", "Fill"))
        .hand(0, &[CARD_ID])
        .deck(0, &["FILL"; 5])
        .deck(1, &["FILL"; 5])
        .memory(8)
        .start();

    let low = r.place_on_field(1, "OPP-LOW", Some(0));
    let high = r.place_on_field(1, "OPP-HIGH", Some(0));
    assert_eq!(r.battle_area_size(1), 2);

    // Activate the Option's [Main] effect from hand.
    assert!(r.game.activate_hand_main(0, 0), "activate Trident Gaia [Main]");

    // Only the highest-DP Digimon should be a legal delete target.
    let view = r
        .pending_selection_view()
        .expect("highest-DP delete target prompt installs");
    assert_eq!(
        view.valid_action_ids.len(),
        1,
        "only the single highest-DP opponent Digimon is a legal target"
    );

    // Drive the (only) legal pick and confirm the HIGH-DP Digimon is gone and
    // the LOW-DP Digimon survives.
    let _ = high; // documented expectation: the 14000-DP Digimon is the target
    r.execute_action(view.selecting_player, view.valid_action_ids[0])
        .expect("delete the highest-DP opponent Digimon");
    let _ = r.auto_resolve();

    assert_eq!(
        r.battle_area_size(1),
        1,
        "exactly one opponent Digimon was deleted"
    );
    assert!(
        r.game.players[1]
            .battle_area
            .iter()
            .any(|p| p.top_card().card_id(&r.game.card_data) == "OPP-LOW"),
        "the lower-DP Digimon survives"
    );
    assert!(
        r.game.players[1]
            .battle_area
            .iter()
            .all(|p| p.top_card().card_id(&r.game.card_data) != "OPP-HIGH"),
        "the highest-DP Digimon was the one deleted"
    );
    let _ = low;
}

/// NEGATIVE: with no opponent Digimon, the [Main] effect installs no prompt and
/// is a no-op (the Option still resolves).
#[test]
fn ex4_065_main_no_target_is_a_noop() {
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX4-065 loads")
        .add_card(make_test_card("FILL", "Fill"))
        .hand(0, &[CARD_ID])
        .deck(0, &["FILL"; 5])
        .deck(1, &["FILL"; 5])
        .memory(8)
        .start();

    assert!(r.game.activate_hand_main(0, 0), "activate Trident Gaia [Main]");
    let _ = r.auto_resolve();
    assert_eq!(r.battle_area_size(1), 0, "no opponent Digimon to delete");
}

// ════════════════════════════════════════════════════════════════════════════
// Gap-routed tail
// ════════════════════════════════════════════════════════════════════════════

#[ignore = "pending: G-HIGHEST-DP-DELETE-WITH-EFFECT-PAYLOAD — the conditional 'if a 13000+ DP Digimon was deleted by this effect, trash opp top security' needs a deleted-permanent DP threshold on the effect-deleted result payload (the log stores only PermanentHandles, no DP, and the carrier has moved to trash); tracker: qa/dsl-vocab-gaps.md"]
#[test]
fn ex4_065_trashes_security_when_deleted_dp_13000_or_more() {}
