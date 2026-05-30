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
/// the digitama deck. RESOLVED 2026-05-29 (G-RETURN-TRASH-DIGI-EGG-ROUTING,
/// change `fix-judge-quiz-engine-gaps`): `return_trash_cards_to_deck_bottom` now
/// routes through `EffectContext::move_card_to_deck`, which sends a
/// `CardKind::DigiEgg` to the digitama deck instead of the main deck.
#[test]
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

/// Regression sibling — the deck-TOP return verb must apply the same Digi-Egg
/// routing (`return_trash_cards_to_deck_top` → digitama deck, not main deck).
#[test]
fn digi_egg_returned_to_deck_top_routes_to_digitama_deck() {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX8-005")
        .expect("EX8-005 (Tumblemon, kind: digi_egg) loads")
        .add_card(make_test_card("SRC", "Src"))
        .memory(10)
        .start();

    let src = runner.place_on_field(0, "SRC", None);
    let src_card = runner.game.player(0).battle_area[0].top_card().handle();

    let egg_handle = push_to_trash(&mut runner, 0, "EX8-005");
    let deck_before = runner.deck_size(0);

    {
        let mut ctx = EffectContext::new(&mut runner.game, src_card, Some(src), 0);
        ctx.return_trash_cards_to_deck_top(0, &[egg_handle]);
    }

    assert_eq!(
        runner.game.player(0).digitama_deck.len(),
        1,
        "a Digi-Egg returned to the TOP of the deck must also route to the \
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
///
/// Board (card-resolution.md Q12): a Venusmon (BT24-040) carrier with Sharkmon
/// (BT24-059) as a digivolution source contributes the inherited
/// "[When Attacking] [Once Per Turn] By placing 1 of your other Digimon as this
/// Digimon's bottom digivolution card, it unsuspends." The "other Digimon"
/// chosen is a **Petrification token** ("Digimon/White/3000 DP/[Your Turn] This
/// Digimon can't suspend."). Judge: YES — the token COUNTS as one of your
/// Digimon and is placeable as a digivolution card, so the placement happens and
/// Venusmon unsuspends.
///
/// ── DISCOVERED ENGINE GAP (2026-05-29) — pinned-but-blocked ──────────────────
/// A faithful Q12 test was authored (carrier Venusmon + Sharkmon source, with the
/// REAL `TOKEN_PETRIFICATION` permanent — `CardKind::Token` — as the "other
/// Digimon") and PROVES a real faithfulness gap: BT24-059's inherited
/// placement filter is `kind: digimon` (`select_own_permanent` → the field
/// candidate predicate `kind_matches_field`, predicate.rs:2826), which matches
/// only `CardKind::Digimon | CardKind::Dual` — NOT `CardKind::Token`. So the
/// Petrification token is filtered out of the candidate set, the placement
/// selection never installs, and the unsuspend cannot occur. The judge rule is
/// precisely that a token counts as a Digimon and IS placeable; the engine does
/// not honor that for field selection filters.
///
/// This is a "could-pass-for-the-wrong-reason" trap: the existing per-card
/// fixture `bt24::bt24_059::inherited_q12_token_source_counts_and_unsuspends`
/// passes ONLY because it uses a stand-in `CardKind::Digimon` ("TOKEN-LIKE")
/// rather than an actual token — so it proves "any 1-card permanent placed as a
/// source counts", NOT the token-as-Digimon rule Q12 turns on. We therefore do
/// NOT substitute a Digimon stand-in here (that would false-pass); the scenario
/// stays `#[ignore]`-blocked on the named gap.
///
/// Fix (out of scope for the test-only change that surfaced this): the
/// field-permanent `kind: digimon` matcher must treat a battle-area
/// `CardKind::Token` permanent as a Digimon (tokens ARE Digimon per the rules
/// manual / glossary). Once that lands, restore the authored body (spawn the
/// real `TOKEN_PETRIFICATION` permanent, fire the carrier's inherited
/// [When Attacking], assert the token is a legal placement pick → carrier
/// unsuspends → token has become a digivolution source).
#[test]
#[ignore = "ENGINE GAP G-TOKEN-NOT-DIGIMON-FOR-FIELD-SELECT: BT24-059's inherited \
place filter `kind: digimon` (kind_matches_field) rejects CardKind::Token, so a \
Petrification token is not offered as 'one of your other Digimon'. Judge Q12 says a \
token counts. Pinning faithfully requires the real token (no Digimon stand-in), \
which the engine filters out — refusing to false-pass per the suite's discover-\
then-pin rule. Promote once tokens match `kind: digimon` for field selection."]
fn q12_token_placeable_as_digivolution_card_unsuspends() {}
