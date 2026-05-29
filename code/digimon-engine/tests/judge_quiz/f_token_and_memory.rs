//! Cluster F — token lifecycle & multi-effect memory arithmetic.
//!
//! Questions (see `card-resolution.md`):
//!   Q10 Memory math across Akihiro Kurata (BT13-103), MirageGaogamon
//!       (BT11-033), Gravity Crush (BT1-090), Mental Training (P-104) — judge: 0.
//!   Q11 Follow-up with a 2nd Mental Training; Gravity Crush is NOT
//!       `[Once Per Turn]` so it fires again — judge: 4.
//!   Q12 Venusmon (BT24-040) uses Sharkmon (BT24-059) inherited `[When
//!       Attacking]` to place a Petrification token as a digivolution card to
//!       unsuspend — judge: YES, will unsuspend (token placeable though it
//!       won't remain).
//!   Q22 Proganomon (EX8-051)+Tumblemon (EX8-005) vs Medusamon (BT24-017):
//!       Digi-Eggs to the egg deck still satisfy "send 2 to the bottom of the
//!       deck" — judge: YES, 2 Petrification tokens.  [READY: all impl]
//!
//! Scenarios authored under tasks §8.

#![allow(unused_imports)]

use digimon_engine::card_source::{CardHandle, CardSource};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::effect_context::EffectContext;
use digimon_engine::enums::CardKind;

/// Push `card_id` (must already be in `card_data`) onto player `p`'s trash and
/// return its `CardHandle`.
fn push_to_trash(runner: &mut DebugRunner, p: u8, card_id: &str) -> CardHandle {
    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .expect("card in card_data");
    let idx = runner.game.next_card_index();
    let cs = CardSource::new(data_idx, p, idx);
    let handle = cs.handle();
    runner.game.players[p as usize].trash.push(cs);
    handle
}

// ─────────────────────────────────────────────────────────────────────────────
// Q22 — Digi-Egg routing on "return to the bottom of the deck"
// ─────────────────────────────────────────────────────────────────────────────
//
// Full board (card-resolution.md Q22): Proganomon (EX8-051, Rock) with 3
// Tumblemon (EX8-005, kind: digi_egg) sources; Player B's Medusamon (BT24-017)
// [When Digivolving] targets Proganomon, who uses `[Fragment <3>]` to trash all
// 3 Tumblemon to survive; Medusamon then "by returning 2 cards from their trash
// to the bottom of the deck, they play 2 [Petrification] Tokens." The 2 returned
// Tumblemon are Digi-Eggs. JUDGE ANSWER: YES — even though "Digi-Eggs can't go
// to the bottom of the deck, they instead get sent to the bottom of the Digi-Egg
// deck," that still satisfies Medusamon's "send 2 to the bottom of the deck"
// requirement, so 2 Tokens spawn.
//
// This test pins the LOAD-BEARING rule at its tightest scope: a Digi-Egg card
// returned via `EffectContext::return_trash_cards_to_deck_bottom` must route to
// the Digi-Egg (digitama) deck, NOT the main deck. (The full Proganomon→Medusamon
// chain — Fragment, the return cost, token spawn — is deferred to an end-to-end
// Q22 test; the token-count outcome would pass regardless of routing, masking
// the bug, so the routing destination is the assertion that actually matters.)
//
// ── DISCOVERY-WAVE FINDING (2026-05-29) ──────────────────────────────────────
// GENUINE ENGINE GAP. `return_trash_cards_to_deck_bottom`
// (effect_context/mod.rs:5554-5556) unconditionally does
// `self.game.player_mut(owner).deck.insert(0, card)` for EVERY returned card —
// no `CardKind::DigiEgg` check, no digitama routing. A Digi-Egg returned this way
// lands in the MAIN deck, violating the rule Q22 tests. Logged in
// qa/archetype-qa/engine-gaps.md as G-RETURN-TRASH-DIGI-EGG-ROUTING.

/// Q22 core rule — a Digi-Egg returned "to the bottom of the deck" must route to
/// the digitama deck. CONFIRMED FAILING against current engine (digitama_deck
/// stays empty; the egg lands in the main deck). `#[ignore]`-d citing the logged
/// gap so the suite stays green; un-ignore when G-RETURN-TRASH-DIGI-EGG-ROUTING
/// is fixed.
#[test]
#[ignore = "DISCOVERED BUG (proven failing 2026-05-29): return_trash_cards_to_deck_bottom (effect_context/mod.rs:5554) inserts Digi-Eggs into the MAIN deck — no digitama routing. Logged G-RETURN-TRASH-DIGI-EGG-ROUTING in qa/archetype-qa/engine-gaps.md. Un-ignore when fixed."]
fn q22_digi_egg_returned_to_deck_bottom_routes_to_digitama_deck() {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX8-005")
        .expect("EX8-005 (Tumblemon, kind: digi_egg) loads")
        .add_card(make_test_card("SRC", "Src"))
        .memory(10)
        .start();

    let src = runner.place_on_field(0, "SRC", None);
    let src_card = runner.game.player(0).battle_area[0].top_card().handle();

    // Sanity: EX8-005 is a Digi-Egg in card_data.
    let egg_is_digi_egg = runner
        .game
        .card_data
        .iter()
        .find(|c| c.card_id == "EX8-005")
        .map(|c| c.card_kind == CardKind::DigiEgg)
        .unwrap_or(false);
    assert!(
        egg_is_digi_egg,
        "EX8-005 Tumblemon must load as CardKind::DigiEgg"
    );

    let egg_handle = push_to_trash(&mut runner, 0, "EX8-005");
    assert_eq!(runner.trash_size(0), 1, "egg seeded into trash");
    let deck_before = runner.deck_size(0);

    {
        let mut ctx = EffectContext::new(&mut runner.game, src_card, Some(src), 0);
        ctx.return_trash_cards_to_deck_bottom(0, &[egg_handle]);
    }

    // Judge-correct (Q22): the Digi-Egg routes to the digitama deck.
    assert_eq!(
        runner.game.player(0).digitama_deck.len(),
        1,
        "a Digi-Egg returned to the bottom of the deck must route to the \
         digitama deck (G-RETURN-TRASH-DIGI-EGG-ROUTING)"
    );
    assert_eq!(
        runner.deck_size(0),
        deck_before,
        "the Digi-Egg must NOT be inserted into the main deck"
    );
}

// ── Q10 / Q11 / Q12 — BLOCKED-CARD ───────────────────────────────────────────

/// Q10 — memory math across Akihiro Kurata (BT13-103), MirageGaogamon (BT11-033),
/// Gravity Crush (BT1-090), Mental Training (P-104). Judge: gauge ends at 0.
#[test]
#[ignore = "BLOCKED-CARD: needs BT13-103 (Akihiro Kurata), BT11-033 (MirageGaogamon), P-104 (Mental Training). BT1-090 implemented."]
fn q10_multi_effect_memory_arithmetic_ends_at_zero() {}

/// Q11 — follow-up with a 2nd Mental Training; Gravity Crush (BT1-090) is NOT
/// [Once Per Turn] so it fires again. Judge: Player A starts turn at 4.
#[test]
#[ignore = "BLOCKED-CARD: needs BT13-103, BT11-033, P-104. BT1-090 implemented."]
fn q11_non_opt_gravity_crush_refires_memory_four() {}

/// Q12 — Venusmon (BT24-040) uses Sharkmon (BT24-059) inherited [When Attacking]
/// to place a Petrification token as a digivolution card to unsuspend. Judge:
/// YES, will unsuspend (token placeable though it won't remain).
#[test]
#[ignore = "BLOCKED-CARD: needs BT24-059 (Sharkmon). BT24-040 and Petrification token implemented."]
fn q12_token_placeable_as_digivolution_card_unsuspends() {}
