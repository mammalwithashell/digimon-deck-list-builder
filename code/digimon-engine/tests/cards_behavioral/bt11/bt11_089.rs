//! BT11-089 Akiho Rindou — Tamer, Red, Cost 3.
//!
//! # Card text (official Bandai DB / card image; cards.json agrees)
//!
//! [On Play] Reveal the top 4 cards of your deck. Add 1 red Digimon card with
//! the [Vaccine] trait among them to your hand. Place the rest at the bottom
//! of your deck in any order.
//! [Your Turn] When an effect plays any of your red Digimon with [Avian],
//! [Bird], [Beast], [Animal] or [Sovereign] in any of their traits (other than
//! [Sea Animal] trait), by suspending this Tamer, those Digimon gain <Rush>
//! for the turn.
//!
//! Security Effect [Security] Play this card without paying the cost.
//!
//! Official Q&A: "Yes. Other than [Sea Animal], all traits with [Avian],
//! [Bird], [Beast], [Animal] or [Sovereign] are applicable regardless of other
//! words or pluralizations." → modeled with the substring/pluralization-
//! tolerant `event_target_trait_contains` leaf (G-DSL-EVENT-TARGET-TRAIT-CONTAINS),
//! not the exact-match `event_target_trait_has`.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT11/Red/BT11_089.cs
//!
//! # Patterns this test covers
//! - Reveal top 4 / choose-from-reveal (add-to-hand) / order_remainder (deck
//!   bottom, "in any order" exposed as an OrderedPermutation selection).
//! - `on_any_digimon_played` observer + `event_is_effect_initiated: true` +
//!   `event_target_owner` + `event_target_kind` + `event_target_color_any_of`
//!   + `event_target_trait_contains` (5x any_of) + `not: event_target_trait_has`
//!   negative-trait exclusion.
//! - `activation_cost: { suspend_self: true }` gating a `grant_keyword` (Rush,
//!   `expiry: end_of_turn`) applied to `event_target`.
//! - [Security] play self free.
//!
//! DCGO note: `Filter(PermanentCondition).Clone()` loops over every matched
//! permanent from the SAME enter-field hashtable, but the DSL's
//! `TriggerSource::EnteredField` dispatch already fires the observer once per
//! entered permanent (see `effect_queue.rs`), so a single YAML clause without
//! an inner loop is a faithful match: two qualifying Digimon entering via the
//! SAME effect call each get their own firing/suspend-cost/grant.

use digimon_dsl::compiled::{
    CompiledCardKind, CompiledClause, CompiledColor, CompiledScope, CompiledStep, CompiledTiming,
};
use digimon_engine::action::space::PASS;
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card_with_level, DebugRunner};
use digimon_engine::effect_context::EffectContext;
use digimon_engine::enums::{CardColor, CardKind, Keyword};
use digimon_engine::selection::SelectionKind;

const CARD_ID: &str = "BT11-089";

// ─── Card fixtures ──────────────────────────────────────────────────────────

fn red_vaccine_digimon(id: &str, level: u8) -> CardData {
    let mut c = make_test_card_with_level(id, id, level);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Red];
    c.traits = vec!["Vaccine".to_string()];
    c.dp = Some(i32::from(level) * 1000);
    c.play_cost = level as u16;
    c
}

fn non_vaccine_red_digimon(id: &str, level: u8) -> CardData {
    let mut c = make_test_card_with_level(id, id, level);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Red];
    c.traits = vec!["Dragon".to_string()];
    c.dp = Some(i32::from(level) * 1000);
    c.play_cost = level as u16;
    c
}

fn blue_vaccine_digimon(id: &str, level: u8) -> CardData {
    let mut c = make_test_card_with_level(id, id, level);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Blue];
    c.traits = vec!["Vaccine".to_string()];
    c.dp = Some(i32::from(level) * 1000);
    c.play_cost = level as u16;
    c
}

fn filler_option(id: &str) -> CardData {
    let mut c = make_test_card_with_level(id, id, 0);
    c.card_kind = CardKind::Option;
    c.colors = vec![CardColor::Red];
    c.traits = Vec::new();
    c.play_cost = 1;
    c
}

/// Played-target fixture for the [Your Turn] Rush-grant observer: a red
/// Digimon whose traits vector carries a token that must match one of the
/// five trigger traits by SUBSTRING (pluralization-tolerant), per the
/// official Q&A.
fn red_trait_digimon(id: &str, level: u8, trait_str: &str) -> CardData {
    let mut c = make_test_card_with_level(id, id, level);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Red];
    c.traits = vec![trait_str.to_string()];
    c.dp = Some(i32::from(level) * 1000);
    c.play_cost = level as u16;
    c
}

fn akiho_runner() -> digimon_engine::debug_runner::DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT11-089 YAML loads")
        .add_card(red_vaccine_digimon("RED-VAX-3", 3))
        .add_card(red_vaccine_digimon("RED-VAX-4", 4))
        .add_card(non_vaccine_red_digimon("RED-NONVAX-3", 3))
        .add_card(blue_vaccine_digimon("BLUE-VAX-3", 3))
        .add_card(filler_option("FILLER-OPT-1"))
        .add_card(filler_option("FILLER-OPT-2"))
        .add_card(filler_option("FILLER-OPT-3"))
        .add_card(filler_option("FILLER-OPT-4"))
        // Trigger-side fixtures for clause 2 (event target trait matches).
        .add_card(red_trait_digimon("RED-AVIAN", 3, "Avian"))
        .add_card(red_trait_digimon("RED-BIRDS", 3, "Birds")) // pluralization
        .add_card(red_trait_digimon("RED-SEABEAST", 3, "Sea Beast")) // compound "Beast"
        .add_card(red_trait_digimon("RED-SEA-ANIMAL", 3, "Sea Animal")) // excluded
        .add_card(red_trait_digimon("RED-DRAGON", 3, "Dragon")) // no match
}

// ─── Structural ─────────────────────────────────────────────────────────────

#[test]
fn bt11_089_structural_identity() {
    let runner = akiho_runner().start();
    let compiled = runner.compiled_card(CARD_ID).expect("compiled card");

    assert_eq!(compiled.card, "BT11-089");
    assert_eq!(compiled.name, "Akiho Rindou");
    assert_eq!(compiled.kind, CompiledCardKind::Tamer);
    assert_eq!(compiled.cost, Some(3));
    assert_eq!(compiled.color, vec![CompiledColor::Red]);
}

#[test]
fn bt11_089_has_on_play_your_turn_and_security_clauses() {
    let runner = akiho_runner().start();
    let compiled = runner.compiled_card(CARD_ID).expect("compiled card");

    let triggered: Vec<_> = compiled
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();

    assert_eq!(
        triggered.len(),
        3,
        "On Play, Your-Turn observer, and Security clauses must all be present"
    );

    assert!(
        triggered
            .iter()
            .any(|t| t.when == vec![CompiledTiming::OnPlay]),
        "must have an [On Play] clause"
    );
    assert!(
        triggered
            .iter()
            .any(|t| t.when.contains(&CompiledTiming::OnSecurity)),
        "must have a [Security] clause"
    );

    let security = triggered
        .iter()
        .find(|t| t.when.contains(&CompiledTiming::OnSecurity))
        .unwrap();
    assert_eq!(security.scope, CompiledScope::FaceUp);
    assert!(!security.optional, "[Security] play self is mandatory");

    let observer = triggered
        .iter()
        .find(|t| t.when.contains(&CompiledTiming::OnAnyDigimonPlayed))
        .expect("[Your Turn] played-observer clause must be present");
    assert!(
        observer
            .active_when
            .as_ref()
            .is_some_and(|p| p.your_turn == Some(true)),
        "observer is [Your Turn]-gated"
    );
}

#[test]
fn bt11_089_on_play_reveals_4_and_places_remainder_at_deck_bottom() {
    let runner = akiho_runner().start();
    let compiled = runner.compiled_card(CARD_ID).expect("compiled card");

    let on_play = compiled
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t) if t.when == vec![CompiledTiming::OnPlay] => Some(t),
            _ => None,
        })
        .expect("[On Play] clause present");

    assert!(
        on_play
            .process
            .iter()
            .any(|s| matches!(s, CompiledStep::RevealTopDeck { count: 4, .. })),
        "must reveal exactly the top 4 cards"
    );
    assert!(
        on_play
            .process
            .iter()
            .any(|s| matches!(s, CompiledStep::ChooseFromReveal { .. })),
        "must offer a choose-from-reveal add-to-hand pick"
    );
    assert!(
        on_play
            .process
            .iter()
            .any(|s| matches!(s, CompiledStep::OrderRemainder { .. })),
        "must place the unreveal-picked remainder back on the deck (bottom, in any order)"
    );
}

#[test]
fn bt11_089_your_turn_observer_gates_on_suspend_self_activation_cost() {
    let runner = akiho_runner().start();
    let compiled = runner.compiled_card(CARD_ID).expect("compiled card");

    let observer = compiled
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t)
                if t.when.contains(&CompiledTiming::OnAnyDigimonPlayed) =>
            {
                Some(t)
            }
            _ => None,
        })
        .expect("[Your Turn] played-observer clause present");

    use digimon_dsl::compiled::CompiledActivationCostKind;
    assert!(
        observer.process.iter().any(|s| matches!(
            s,
            CompiledStep::ActivationCost {
                kind: CompiledActivationCostKind::SuspendSelf
            }
        )),
        "the observer body must lead with a suspend_self activation cost"
    );
    assert!(
        observer
            .process
            .iter()
            .any(|s| matches!(s, CompiledStep::GrantKeyword { keyword, .. } if keyword == "Rush")),
        "the observer body must grant Rush"
    );
}

// ─── On Play behavioral: reveal 4 / add matching / bottom remainder ────────

#[test]
fn bt11_089_on_play_adds_matching_red_vaccine_and_bottoms_the_rest() {
    // Deck top→bottom order: last id in the array is popped first (top).
    // Top 4 = [BLUE-VAX-3(top), RED-NONVAX-3, FILLER-OPT-2, RED-VAX-3(bottom
    // of the 4)] once reversed for pop() semantics — see helper doc below.
    // Exactly ONE qualifying card (RED-VAX-3: red + Vaccine) is present among
    // the 4 revealed.
    let mut runner = akiho_runner()
        .hand(0, &[CARD_ID])
        .deck(
            0,
            &[
                "FILLER-OPT-1", // stays in deck (5th from top, untouched)
                "RED-VAX-3",    // revealed (bottom of the 4) — the ONLY match
                "FILLER-OPT-2", // revealed (Option, not a Digimon)
                "RED-NONVAX-3", // revealed (red, but no Vaccine trait)
                "BLUE-VAX-3",   // revealed (TOP — popped first; wrong color)
            ],
        )
        .memory(10)
        .start();

    let deck_before = runner.deck_size(0);
    runner.play(0, 0).expect("Akiho plays from hand");

    let view = runner
        .pending_selection_view()
        .expect("choose-from-reveal prompt must install");
    assert_eq!(view.kind, SelectionKind::Reveal);
    // Only RED-VAX-3 (red + Vaccine) is a legal add; the others (blue, an
    // Option, or red non-Vaccine) must be excluded from the pick.
    let non_pass: Vec<_> = view
        .valid_action_ids
        .iter()
        .copied()
        .filter(|&a| a != PASS)
        .collect();
    assert_eq!(
        non_pass.len(),
        1,
        "only the red [Vaccine] Digimon among the 4 revealed must be a legal pick"
    );
    runner
        .execute_action(0, non_pass[0])
        .expect("add the red Vaccine Digimon to hand");
    let _ = runner.auto_resolve();

    assert!(
        runner.game.players[0]
            .hand
            .iter()
            .any(|c| c.card_id(&runner.game.card_data) == "RED-VAX-3"),
        "the red [Vaccine] Digimon must be added to hand"
    );
    assert_eq!(
        runner.deck_size(0),
        deck_before - 1,
        "deck shrinks by exactly 1 (the added card); the other 3 return to the deck"
    );
    // The 3 non-picked revealed cards must still be in the deck (not trashed,
    // not left in a reveal pool).
    for id in ["FILLER-OPT-2", "RED-NONVAX-3", "BLUE-VAX-3"] {
        assert!(
            runner.game.players[0]
                .deck
                .iter()
                .any(|c| c.card_id(&runner.game.card_data) == id),
            "{id} must be returned to the deck bottom (not the picked card)"
        );
    }
}

#[test]
fn bt11_089_on_play_with_no_matching_card_bottoms_all_4() {
    let mut runner = akiho_runner()
        .hand(0, &[CARD_ID])
        .deck(
            0,
            &[
                "FILLER-OPT-1",
                "RED-NONVAX-3", // no Vaccine trait
                "BLUE-VAX-3",   // wrong color
                "RED-NONVAX-3",
                "BLUE-VAX-3",
            ],
        )
        .memory(10)
        .start();

    let deck_before = runner.deck_size(0);
    runner.play(0, 0).expect("Akiho plays from hand");

    // No candidate among the 4 revealed cards satisfies "red + Vaccine" →
    // choose_from_reveal silently fizzles (no PendingSelection installed) and
    // the remainder-ordering step runs directly.
    if let Some(view) = runner.pending_selection_view() {
        assert_eq!(
            view.kind,
            SelectionKind::OrderedPermutation { remaining: 4 },
            "with no eligible add-to-hand candidate, only the remainder ordering \
             prompt (not a hand-add pick) should be pending"
        );
        for _ in 0..4 {
            if let Some(v) = runner.pending_selection_view() {
                runner
                    .execute_action(0, v.valid_action_ids[0])
                    .expect("order remainder pick");
            }
        }
    }
    let _ = runner.auto_resolve();

    // Akiho (the only card that started in hand) has left hand to be played;
    // none of the 4 revealed cards should have joined it in hand.
    assert!(
        runner.game.players[0].hand.is_empty(),
        "no card should be added to hand when no candidate qualifies"
    );
    assert_eq!(
        runner.deck_size(0),
        deck_before,
        "all 4 revealed cards return to the deck when none qualify"
    );
}

// ─── [Your Turn] observer — positive: qualifying red Digimon gains Rush ────

fn observer_runner() -> DebugRunner {
    akiho_runner().memory(10).start()
}

#[test]
fn bt11_089_effect_play_of_red_avian_grants_rush_via_suspend_cost() {
    let mut runner = observer_runner();
    let akiho = runner.place_on_field(0, CARD_ID, Some(0));

    // Put RED-AVIAN in hand and have an EFFECT play it (not a hard play).
    runner.game.players[0].hand.clear();
    let hand_card = {
        let idx = runner
            .game
            .card_data
            .iter()
            .position(|c| c.card_id == "RED-AVIAN")
            .unwrap();
        let next = runner.game.next_card_index();
        digimon_engine::card_source::CardSource::new(idx, 0, next)
    };
    runner.game.players[0].hand.push(hand_card);

    let source_card = runner.top_card(akiho);
    let mut ctx = EffectContext::new(&mut runner.game, source_card, Some(akiho), 0);
    let played = ctx
        .play_from_hand_free(0, 0)
        .expect("effect plays RED-AVIAN from hand");

    runner
        .auto_resolve()
        .expect("resolve suspend-cost + Rush grant");

    assert!(
        runner.game.player(0).battle_area[akiho.index as usize].is_suspended,
        "Akiho must suspend as the activation cost"
    );
    assert!(
        runner.game.has_keyword(played, Keyword::Rush),
        "the effect-played red [Avian] Digimon must gain <Rush>"
    );
}

#[test]
fn bt11_089_pluralized_trait_token_still_matches_substring() {
    // "Birds" must match the `Bird` trigger token per the official Q&A
    // ("regardless of other words or pluralizations").
    let mut runner = observer_runner();
    let akiho = runner.place_on_field(0, CARD_ID, Some(0));
    runner.game.players[0].hand.clear();
    let hand_card = {
        let idx = runner
            .game
            .card_data
            .iter()
            .position(|c| c.card_id == "RED-BIRDS")
            .unwrap();
        let next = runner.game.next_card_index();
        digimon_engine::card_source::CardSource::new(idx, 0, next)
    };
    runner.game.players[0].hand.push(hand_card);

    let source_card = runner.top_card(akiho);
    let mut ctx = EffectContext::new(&mut runner.game, source_card, Some(akiho), 0);
    let played = ctx
        .play_from_hand_free(0, 0)
        .expect("effect plays RED-BIRDS from hand");
    runner
        .auto_resolve()
        .expect("resolve suspend-cost + Rush grant");

    assert!(
        runner.game.has_keyword(played, Keyword::Rush),
        "the pluralized [Birds] trait token must still match the [Bird] trigger \
         via substring tolerance"
    );
}

#[test]
fn bt11_089_compound_trait_sea_beast_matches_beast_substring() {
    // "Sea Beast" contains the substring "Beast" and is NOT the excluded
    // "Sea Animal" token, so it must still match (per official Q&A: matches
    // regardless of other words, EXCEPT the explicit Sea Animal carve-out).
    let mut runner = observer_runner();
    let akiho = runner.place_on_field(0, CARD_ID, Some(0));
    runner.game.players[0].hand.clear();
    let hand_card = {
        let idx = runner
            .game
            .card_data
            .iter()
            .position(|c| c.card_id == "RED-SEABEAST")
            .unwrap();
        let next = runner.game.next_card_index();
        digimon_engine::card_source::CardSource::new(idx, 0, next)
    };
    runner.game.players[0].hand.push(hand_card);

    let source_card = runner.top_card(akiho);
    let mut ctx = EffectContext::new(&mut runner.game, source_card, Some(akiho), 0);
    let played = ctx
        .play_from_hand_free(0, 0)
        .expect("effect plays RED-SEABEAST from hand");
    runner
        .auto_resolve()
        .expect("resolve suspend-cost + Rush grant");

    assert!(
        runner.game.has_keyword(played, Keyword::Rush),
        "[Sea Beast] must match via the [Beast] substring (not excluded)"
    );
}

// ─── [Your Turn] observer — negative branches ──────────────────────────────

#[test]
fn bt11_089_sea_animal_trait_is_explicitly_excluded() {
    let mut runner = observer_runner();
    let akiho = runner.place_on_field(0, CARD_ID, Some(0));
    runner.game.players[0].hand.clear();
    let hand_card = {
        let idx = runner
            .game
            .card_data
            .iter()
            .position(|c| c.card_id == "RED-SEA-ANIMAL")
            .unwrap();
        let next = runner.game.next_card_index();
        digimon_engine::card_source::CardSource::new(idx, 0, next)
    };
    runner.game.players[0].hand.push(hand_card);

    let source_card = runner.top_card(akiho);
    let mut ctx = EffectContext::new(&mut runner.game, source_card, Some(akiho), 0);
    let played = ctx
        .play_from_hand_free(0, 0)
        .expect("effect plays RED-SEA-ANIMAL from hand");
    let _ = runner.auto_resolve();

    assert!(
        !runner.game.has_keyword(played, Keyword::Rush),
        "[Sea Animal] is the explicit carve-out and must NOT gain <Rush>"
    );
    assert!(
        !runner.game.player(0).battle_area[akiho.index as usize].is_suspended,
        "Akiho must not suspend when the observer does not fire"
    );
}

#[test]
fn bt11_089_non_matching_trait_does_not_trigger() {
    let mut runner = observer_runner();
    let akiho = runner.place_on_field(0, CARD_ID, Some(0));
    runner.game.players[0].hand.clear();
    let hand_card = {
        let idx = runner
            .game
            .card_data
            .iter()
            .position(|c| c.card_id == "RED-DRAGON")
            .unwrap();
        let next = runner.game.next_card_index();
        digimon_engine::card_source::CardSource::new(idx, 0, next)
    };
    runner.game.players[0].hand.push(hand_card);

    let source_card = runner.top_card(akiho);
    let mut ctx = EffectContext::new(&mut runner.game, source_card, Some(akiho), 0);
    let played = ctx
        .play_from_hand_free(0, 0)
        .expect("effect plays RED-DRAGON from hand");
    let _ = runner.auto_resolve();

    assert!(
        !runner.game.has_keyword(played, Keyword::Rush),
        "[Dragon] is not one of the five trigger traits and must not gain <Rush>"
    );
}

#[test]
fn bt11_089_hard_play_by_the_controller_does_not_trigger_observer() {
    // A normal (non-effect) play from hand must NOT satisfy
    // `event_is_effect_initiated: true`.
    let mut runner = akiho_runner().hand(0, &["RED-AVIAN"]).memory(10).start();
    let akiho = runner.place_on_field(0, CARD_ID, Some(0));

    runner.play(0, 0).expect("hard play RED-AVIAN from hand");
    let _ = runner.auto_resolve();

    assert!(
        !runner.game.player(0).battle_area[akiho.index as usize].is_suspended,
        "Akiho must not suspend for a normal (non-effect) play"
    );
    let played = runner
        .game
        .player(0)
        .battle_area
        .iter()
        .find(|p| p.top_card().card_id(&runner.game.card_data) == "RED-AVIAN")
        .expect("RED-AVIAN is on field");
    assert!(
        !played.top_card().card_id(&runner.game.card_data).is_empty(),
        "sanity: played card is present"
    );
}

#[test]
fn bt11_089_opponents_effect_play_does_not_trigger_your_turn_observer() {
    // The observer is [Your Turn]-scoped: an opponent's red qualifying Digimon
    // played by an effect during the opponent's own turn must not trigger it
    // even though it is "your" (the opponent's own) Digimon on their turn —
    // this test targets the case where AKIHO's controller is NOT the acting
    // player (i.e., it's the opponent's turn and Akiho's controller is
    // inactive). Use `set_first_player` so player 1 is active and player 0
    // (Akiho's controller) is not.
    let mut runner = observer_runner();
    runner.set_first_player(1);
    let akiho = runner.place_on_field(0, CARD_ID, Some(0));
    runner.game.players[0].hand.clear();
    let hand_card = {
        let idx = runner
            .game
            .card_data
            .iter()
            .position(|c| c.card_id == "RED-AVIAN")
            .unwrap();
        let next = runner.game.next_card_index();
        digimon_engine::card_source::CardSource::new(idx, 0, next)
    };
    runner.game.players[0].hand.push(hand_card);

    let source_card = runner.top_card(akiho);
    let mut ctx = EffectContext::new(&mut runner.game, source_card, Some(akiho), 0);
    let played = ctx
        .play_from_hand_free(0, 0)
        .expect("effect plays RED-AVIAN from hand");
    let _ = runner.auto_resolve();

    assert!(
        !runner.game.has_keyword(played, Keyword::Rush),
        "the observer must not fire on the opponent's turn"
    );
    assert!(
        !runner.game.player(0).battle_area[akiho.index as usize].is_suspended,
        "Akiho must not suspend on the opponent's turn"
    );
}

#[test]
fn bt11_089_observer_does_not_fire_when_akiho_already_suspended() {
    let mut runner = observer_runner();
    let akiho = runner.place_on_field(0, CARD_ID, Some(0));
    runner.game.player_mut(0).battle_area[akiho.index as usize].is_suspended = true;
    runner.game.players[0].hand.clear();
    let hand_card = {
        let idx = runner
            .game
            .card_data
            .iter()
            .position(|c| c.card_id == "RED-AVIAN")
            .unwrap();
        let next = runner.game.next_card_index();
        digimon_engine::card_source::CardSource::new(idx, 0, next)
    };
    runner.game.players[0].hand.push(hand_card);

    let source_card = runner.top_card(akiho);
    let mut ctx = EffectContext::new(&mut runner.game, source_card, Some(akiho), 0);
    let played = ctx
        .play_from_hand_free(0, 0)
        .expect("effect plays RED-AVIAN from hand");
    let _ = runner.auto_resolve();

    assert!(
        !runner.game.has_keyword(played, Keyword::Rush),
        "an already-suspended Akiho cannot pay the suspend cost again, so no grant"
    );
}

// ─── Rush is turn-scoped (expires end of turn) ─────────────────────────────

#[test]
fn bt11_089_rush_grant_expires_at_end_of_turn() {
    let mut runner = observer_runner();
    let akiho = runner.place_on_field(0, CARD_ID, Some(0));
    runner.game.players[0].hand.clear();
    let hand_card = {
        let idx = runner
            .game
            .card_data
            .iter()
            .position(|c| c.card_id == "RED-AVIAN")
            .unwrap();
        let next = runner.game.next_card_index();
        digimon_engine::card_source::CardSource::new(idx, 0, next)
    };
    runner.game.players[0].hand.push(hand_card);

    let source_card = runner.top_card(akiho);
    let mut ctx = EffectContext::new(&mut runner.game, source_card, Some(akiho), 0);
    let played = ctx
        .play_from_hand_free(0, 0)
        .expect("effect plays RED-AVIAN from hand");
    runner.auto_resolve().expect("resolve grant");

    assert!(runner.game.has_keyword(played, Keyword::Rush));

    runner.end_turn();
    runner.end_turn();

    assert!(
        !runner.game.has_keyword(played, Keyword::Rush),
        "<Rush> must expire once the granting turn ends"
    );
}

// ─── [Security] play self free ─────────────────────────────────────────────

#[test]
fn bt11_089_security_plays_itself_without_paying_cost() {
    let mut attacker = make_test_card_with_level("ATTACKER-BT11-089", "Attacker", 4);
    attacker.card_kind = CardKind::Digimon;
    attacker.dp = Some(9000);

    let mut runner = akiho_runner()
        .add_card(attacker)
        .security(1, &[CARD_ID])
        .memory(10)
        .start();
    let attacker = runner.place_on_field(0, "ATTACKER-BT11-089", Some(0));

    runner.attack_player(attacker, 1, false);
    runner.auto_resolve().expect("resolve security play");

    assert!(
        runner.game.players[1]
            .battle_area
            .iter()
            .any(|p| p.top_card().card_id(&runner.game.card_data) == CARD_ID),
        "BT11-089 is played from the defender's security without paying cost"
    );
}
