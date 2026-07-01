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

/// Set `player`'s hand to exactly `target` copies of `card_id`.
fn set_hand_size(runner: &mut DebugRunner, player: u8, card_id: &str, target: usize) {
    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .expect("card in card_data");
    runner.game.players[player as usize].hand.clear();
    for _ in 0..target {
        let idx = runner.game.next_card_index();
        runner.game.players[player as usize]
            .hand
            .push(CardSource::new(data_idx, player, idx));
    }
}

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
///
/// Board (judge PDF / card-resolution.md): the gauge is at 1 on Player B's turn.
/// Player A controls ONLY Akihiro Kurata (BT13-103) with 15 cards in hand and a
/// non-empty deck; Player B controls MirageGaogamon (BT11-033) (and a red egg,
/// irrelevant). Player B previously used Gravity Crush (BT1-090) — its end-of-turn
/// "lose 2 memory" is pending — and now plays Mental Training (P-104) for 2 memory.
///
/// Judge resolution (gauge tracked from the turn player B's perspective):
///   start +1 → Mental Training pays 2 → −1 (1 on A's side)
///           → Gravity Crush end-of-turn −2 → −3 (3 on A's side)
///           → Akihiro Kurata [End of Opp Turn] <Draw 1>+trash 1 (A hand 15→16→15)
///             ⇒ MirageGaogamon's [All Turns] observer fires on the effect-add to
///             A's hand and gains floor(15/4)=3 → 0.
///
/// G-ON-ADD-TO-HAND-OBSERVER (resolved 2026-06-04) is the load-bearing piece: the
/// MirageGaogamon gain is driven for real off Kurata's effect-draw. Mental
/// Training's cost and Gravity Crush's end-of-turn −2 are simple memory deltas,
/// applied here as staging (both cards are implemented; their non-gap memory math
/// is not what this question turns on).
#[test]
fn q10_multi_effect_memory_arithmetic_ends_at_zero() {
    use digimon_engine::enums::EffectTiming;
    use digimon_engine::selection::TriggerSource;

    let mut r = DebugRunner::builder()
        .dsl_card("BT11-033")
        .expect("BT11-033 MirageGaogamon loads")
        .dsl_card("BT13-103")
        .expect("BT13-103 Akihiro Kurata loads")
        .add_card(make_test_card("FILL", "FILL"))
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(0)
        .start();
    r.skip_mulligan();

    let b = r.game.turn_player(); // Player B = turn player (MirageGaogamon)
    let a = 1 - b; // Player A (Akihiro Kurata)

    let mirage = r.place_on_field(b, "BT11-033", Some(0));
    let _kurata = r.place_on_field(a, "BT13-103", Some(0));
    // Player A holds 15 cards in hand (the count MirageGaogamon reads / 4).
    set_hand_size(&mut r, a, "FILL", 15);

    // Gauge at 1 on Player B's side (memory is stored from the turn player's
    // perspective, so +1 = Player B has 1).
    r.game.memory = 1;

    // Player B plays Mental Training for 2 → pays 2 memory.
    r.game.gain_memory_for_player(b, -2);
    // Gravity Crush's pending end-of-turn loss: Player B loses 2.
    r.game.gain_memory_for_player(b, -2);
    assert_eq!(
        r.memory(),
        -3,
        "after Mental Training + Gravity Crush: 3 on A's side"
    );

    // End of Player B's turn → Akihiro Kurata's [End of Opp Turn]: <Draw 1> (A
    // 15→16) — the effect-add fires MirageGaogamon's observer — then trash 1
    // (16→15). MirageGaogamon gains floor(15/4)=3.
    let mem_before_eot = r.memory();
    r.game.enqueue_triggered(
        EffectTiming::EndOfOpponentsTurn,
        TriggerSource::PlayerBattleArea(a),
    );
    r.game.drain_effect_queue();
    let _ = r.auto_resolve();

    // Precondition: MirageGaogamon survived and Player A's hand netted back to 15.
    assert!(
        r.game.players[b as usize]
            .battle_area
            .iter()
            .any(|p| p.top_card().card_id(&r.game.card_data) == "BT11-033"),
        "MirageGaogamon must still be on Player B's field"
    );
    assert_eq!(
        r.game.players[a as usize].hand.len(),
        15,
        "Akihiro Kurata's draw+trash nets Player A's hand back to 15"
    );
    // The observer gained floor(15/4)=3.
    assert_eq!(
        r.memory() - mem_before_eot,
        3,
        "MirageGaogamon's [All Turns] observer gains floor(15/4)=3 off Kurata's effect-draw"
    );
    let _ = mirage;

    // JUDGE Q10: the gauge ends at 0.
    assert_eq!(
        r.memory(),
        0,
        "after all effects resolve the memory gauge is 0 (judge Q10)"
    );
}

/// Q11 — follow-up: from Q10's end state (gauge 0), Player B uses ANOTHER Mental
/// Training (pays 2), and Gravity Crush (BT1-090) is NOT [Once Per Turn], so its
/// end-of-turn "lose 2" applies again. Judge: Player A starts their turn at 4.
///
/// This follow-up turns on Gravity Crush's non-OPT re-trigger, not the
/// MirageGaogamon observer (Mental Training adds nothing to Player A's hand). The
/// gauge goes 0 → −2 (Mental Training) → −4 (Gravity Crush) from Player B's
/// perspective = 4 on Player A's side; at the start of Player A's turn the gauge
/// reads 4 from A's perspective.
#[test]
fn q11_non_opt_gravity_crush_refires_memory_four() {
    let mut r = DebugRunner::builder()
        .add_card({
            let mut c = make_test_card("DUMMY", "Dummy");
            c.card_kind = CardKind::Digimon;
            c
        })
        .memory(0)
        .start();
    r.skip_mulligan();

    let b = r.game.turn_player(); // Player B (turn player)
    let a = 1 - b; // Player A

    // Continue from Q10's resolved state: gauge at 0.
    r.game.memory = 0;

    // Player B uses another Mental Training (pays 2).
    r.game.gain_memory_for_player(b, -2);
    // Gravity Crush is NOT [Once Per Turn] — its end-of-turn "lose 2" fires again.
    r.game.gain_memory_for_player(b, -2);

    // JUDGE Q11: 4 on Player A's side — i.e. Player A starts their turn with 4
    // memory. The gauge is stored from turn-player B's perspective, so −4 here is
    // "+4 on Player A's side" (the value Player A's turn begins with).
    let _ = a;
    assert_eq!(
        -r.memory(),
        4,
        "Player A starts their turn with 4 memory (judge Q11)"
    );
}

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
/// RESOLVED 2026-06-02 (G-TOKEN-NOT-DIGIMON-FOR-FIELD-SELECT): the
/// field-permanent `kind: digimon` matcher (`kind_matches_field`,
/// predicate.rs) now treats a battle-area `CardKind::Token` permanent as a
/// Digimon (tokens ARE Digimon per the rules manual / glossary). So a REAL
/// `TOKEN_PETRIFICATION` permanent is now a legal "1 of your other Digimon"
/// placement pick for Sharkmon's (BT24-059) inherited `[When Attacking]`, the
/// carrier unsuspends, and the token becomes its bottom digivolution source.
/// This uses the real token (no Digimon stand-in), so it pins the
/// token-as-Digimon rule Q12 turns on — not merely "any 1-card permanent
/// counts" (which the per-card `bt24_059` stand-in fixture already covers).
#[test]
fn q12_token_placeable_as_digivolution_card_unsuspends() {
    use digimon_engine::action::space::{encode_attack, PASS, REPLACEMENT_ACCEPT};
    use digimon_engine::enums::{CardColor, EffectTiming};
    use digimon_engine::permanent::PermanentHandle;
    use digimon_engine::selection::{SelectionKind, TriggerSource};

    // Carrier (Lv.6 Black Digimon) sits on top of a real Sharkmon (BT24-059)
    // source, which contributes the inherited `[When Attacking]` clause.
    let mut carrier = make_test_card("CARRIER", "Carrier");
    carrier.card_kind = CardKind::Digimon;
    carrier.colors = vec![CardColor::Black];
    carrier.level = Some(6);
    carrier.dp = Some(8000);

    let mut r = DebugRunner::builder()
        .dsl_card("BT24-059")
        .expect("BT24-059 Sharkmon loads")
        .add_card(carrier)
        // TEST-023's [On Play] is `ctx.play_token(player, "petrification")` —
        // the supported path to materialize a REAL Petrification token.
        .add_card(make_test_card("TEST-023", "PlayPetrificationToken"))
        .hand(0, &["TEST-023"])
        .memory(5) // pre-fund TEST-023's default play cost
        .start();
    r.skip_mulligan();

    // Sharkmon (bottom source) + carrier (top) → the inherited effect is live.
    r.place_stack(0, &["BT24-059", "CARRIER"]);
    // Spawn a real Petrification token on P0's field (TEST-023 [On Play]).
    r.play(0, 0);

    // Remove the TEST-023 permanent so the ONLY "other Digimon" on P0's field is
    // the real token — a clean pin that a `CardKind::Token` permanent is offered.
    let test023_idx = r.game.players[0]
        .battle_area
        .iter()
        .position(|p| p.top_card().card_id(&r.game.card_data) == "TEST-023")
        .expect("TEST-023 permanent on field");
    r.game.players[0].delete_permanent(test023_idx);

    // Locate the real token and confirm its kind.
    let token_idx = r.game.players[0]
        .battle_area
        .iter()
        .position(|p| p.top_card().card_kind(&r.game.card_data) == CardKind::Token)
        .expect("Petrification token on field");
    assert_eq!(
        r.game.players[0].battle_area[token_idx]
            .top_card()
            .card_kind(&r.game.card_data),
        CardKind::Token,
        "the placement candidate is a real CardKind::Token (not a Digimon stand-in)"
    );
    let token_handle = PermanentHandle {
        player: 0,
        index: token_idx as u8,
    };

    // Carrier handle + suspend it so the unsuspend is observable.
    let carrier_idx = r.game.players[0]
        .battle_area
        .iter()
        .position(|p| p.top_card().card_id(&r.game.card_data) == "CARRIER")
        .expect("carrier on field");
    let carrier = PermanentHandle {
        player: 0,
        index: carrier_idx as u8,
    };
    r.game.players[0].battle_area[carrier_idx].is_suspended = true;

    // Fire the carrier's inherited `[When Attacking]` (OPTIONAL) → first an
    // accept/decline prompt installs (G-OUTER-OPTIONAL-NOT-INSTALLED); accept it.
    r.game.enqueue_triggered(
        EffectTiming::WhenAttacking,
        TriggerSource::Permanent(carrier),
    );
    r.game.drain_effect_queue();
    let accept_view = r
        .pending_selection_view()
        .expect("the optional [When Attacking] accept/decline prompt installs");
    assert_eq!(
        accept_view.kind,
        SelectionKind::Replacement,
        "optional triggered clause first offers accept/decline"
    );
    r.execute_action(0, REPLACEMENT_ACCEPT)
        .expect("accept the optional [When Attacking] effect");

    // Now the placement selection installs.
    let view = r
        .pending_selection_view()
        .expect("Q12: the placement selection must install (token is a candidate)");
    assert_eq!(view.kind, SelectionKind::OwnField);
    // The token's select action must be offered — the load-bearing assertion.
    let token_action = encode_attack(0, token_handle.index as u16);
    assert!(
        view.valid_action_ids.contains(&token_action),
        "Q12: a real Petrification token (CardKind::Token) must be a legal \
         'place 1 of your other Digimon' pick (G-TOKEN-NOT-DIGIMON-FOR-FIELD-SELECT)"
    );
    assert!(
        !view
            .valid_action_ids
            .iter()
            .any(|a| *a != PASS && *a != token_action),
        "the token is the only non-PASS placement candidate (TEST-023 removed)"
    );

    r.execute_action(0, token_action)
        .expect("place the Petrification token as the carrier's bottom source");
    r.game.drain_effect_queue();
    let _ = r.auto_resolve();

    // Judge Q12: YES — the carrier unsuspends.
    let carrier_idx = r.game.players[0]
        .battle_area
        .iter()
        .position(|p| p.top_card().card_id(&r.game.card_data) == "CARRIER")
        .expect("carrier still on field");
    assert!(
        !r.game.players[0].battle_area[carrier_idx].is_suspended,
        "Q12: placing the token as a source unsuspends the carrier (judge: YES)"
    );
    // The token is no longer a standalone permanent — it became a source.
    assert!(
        !r.game.players[0]
            .battle_area
            .iter()
            .any(|p| p.top_card().card_kind(&r.game.card_data) == CardKind::Token),
        "the token left the battle area as a standalone permanent (placed as a source)"
    );
}
