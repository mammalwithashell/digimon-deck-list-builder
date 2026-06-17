//! BT13-087 Dynasmon — Digimon, Lv.6, Purple, DP 11000, Cost 10.
//! Traits: Holy Warrior, Royal Knight. Form: Mega. Attribute: Data.
//! Evo: Lv.5 Purple / Cost 3 (alt-path).
//!
//! # Card text (card image BT13-087.webp — authoritative)
//!
//! ```text
//! [On Play] [When Digivolving] Reveal the top 4 cards of your deck. Add 2 cards
//! with [Lucemon] in their names or the [Royal Knight] trait among them to the
//! hand. Trash the rest.
//!
//! [Your Turn] When you play another Digimon with [Lucemon] in its name or the
//! [Royal Knight] trait, delete all of your opponent's level 4 or lower Digimon.
//! ```
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT13/Purple/BT13_087.cs
//!   - Clause A/B (OnEnterFieldAnyone ×2): reveal 4, add ≤2 Lucemon/Royal Knight
//!     (SimplifiedRevealDeckTopCardsAndSelect, RemainingCardsPlace.Trash).
//!   - Clause C (OnEnterFieldAnyone): CanUseCondition = IsExistOnBattleAreaDigimon
//!     && IsOwnerTurn && CanTriggerOnPermanentPlay(PermanentCondition); where
//!     PermanentCondition = own battle-area Digimon, != this card's permanent
//!     ("another"), ContainsCardName("Lucemon") || HasRoyalKnightTraits.
//!     ActivateCoroutine deletes Enemy.GetBattleAreaDigimons() filtered to
//!     Level <= 4 && TopCard.HasLevel.
//!
//! # Patterns this test covers
//! - [On Play][When Digivolving] reveal-4 / add-up-to-2 (two buckets) / trash rest
//! - on_ally_played observer with active_when: { your_turn: true }
//! - event_target_owner: you + event_target_is_source: false ("another")
//! - name-OR-trait filter on the played card (event_card_name_contains /
//!   event_card_trait_has)
//! - for_each delete-all opponent Digimon level_lte: 4 (no player choice)
//! - negative: non-matching played card does not fire; self-play does not fire;
//!   opponent Lv5 survives

use digimon_dsl::compiled::{CompiledClause, CompiledStep, CompiledTiming};
use digimon_engine::action::space::SEL_REVEAL_START;
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardKind, EffectTiming};
use digimon_engine::selection::TriggerSource;

// ─── Card-data factories ─────────────────────────────────────────────────────

/// Build a Digimon test card with a specific id, name, level, dp, and traits.
fn make_digimon(id: &str, name: &str, level: u8, dp: i32, traits: &[&str]) -> CardData {
    let mut card = make_test_card(id, name);
    card.card_kind = CardKind::Digimon;
    card.level = Some(level);
    card.dp = Some(dp);
    card.traits = traits.iter().map(|t| t.to_string()).collect();
    card
}

/// Resolve a revealed-card action id by candidate card_id. Returns None if the
/// named card is not currently in the reveal overlay.
fn revealed_action_for_id(runner: &DebugRunner, id: &str) -> Option<u16> {
    runner
        .game
        .revealed_cards
        .iter()
        .enumerate()
        .find_map(|(idx, card)| {
            (card.card_id(&runner.game.card_data) == id).then_some(SEL_REVEAL_START + idx as u16)
        })
}

/// Pick a revealed card by card_id; asserts the action is legal in the current
/// bucket before executing.
fn pick_revealed_by_id(runner: &mut DebugRunner, id: &str, label: &str) {
    let view = runner.pending_selection_view().expect(label);
    let action = revealed_action_for_id(runner, id)
        .unwrap_or_else(|| panic!("{label}: revealed card {id} not present"));
    assert!(
        view.valid_action_ids.contains(&action),
        "{label}: action {action} for {id} not legal in current bucket; valid={:?}",
        view.valid_action_ids
    );
    runner.execute_action(0, action).expect(label);
}

/// Card ids currently in player `p`'s hand.
fn hand_ids(runner: &DebugRunner, p: u8) -> Vec<String> {
    runner
        .game
        .player(p)
        .hand
        .iter()
        .map(|c| c.card_id(&runner.game.card_data).to_string())
        .collect()
}

/// Card ids currently in player `p`'s trash.
fn trash_ids(runner: &DebugRunner, p: u8) -> Vec<String> {
    runner
        .game
        .player(p)
        .trash
        .iter()
        .map(|c| c.card_id(&runner.game.card_data).to_string())
        .collect()
}

/// Card ids currently on player `p`'s battle area (top card of each permanent).
fn field_ids(runner: &DebugRunner, p: u8) -> Vec<String> {
    runner
        .game
        .player(p)
        .battle_area
        .iter()
        .map(|perm| {
            runner.game.card_data[perm.top_card().data_index]
                .card_id
                .clone()
        })
        .collect()
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 1 — Structural assertions
// ═══════════════════════════════════════════════════════════════════════════

/// BT13-087 has a shared On Play / When Digivolving reveal-search clause.
#[test]
fn bt13_087_has_reveal_search_clause() {
    let runner = DebugRunner::builder()
        .dsl_card("BT13-087")
        .expect("load")
        .start();
    let card = runner.compiled_card("BT13-087").expect("compiled card");
    assert!(card.effects.iter().any(|clause| matches!(
        clause,
        CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::OnPlay)
            && t.when.contains(&CompiledTiming::WhenDigivolving)
    )));
}

/// BT13-087 has an OnAllyPlayed observer clause that deletes opponent Digimon.
#[test]
fn bt13_087_has_on_ally_played_delete_observer() {
    let runner = DebugRunner::builder()
        .dsl_card("BT13-087")
        .expect("load")
        .start();
    let card = runner.compiled_card("BT13-087").expect("compiled card");
    let observer = card
        .effects
        .iter()
        .find_map(|clause| match clause {
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::OnAllyPlayed) => {
                Some(t)
            }
            _ => None,
        })
        .expect("on_ally_played observer clause exists");
    // The delete-all is a for_each (no SelectOpponentPermanent player choice);
    // assert the clause carries a DeletePermanent step somewhere in its process,
    // possibly nested in a ForEach.
    fn contains_delete(steps: &[CompiledStep]) -> bool {
        steps.iter().any(|s| match s {
            CompiledStep::DeletePermanent { .. } => true,
            CompiledStep::ForEach { body, .. } => contains_delete(body),
            _ => false,
        })
    }
    assert!(
        contains_delete(&observer.process),
        "observer must delete opponent Digimon"
    );
    assert!(
        !observer.optional,
        "delete-all is mandatory (no 'you may' in printed text)"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 2 — [On Play][When Digivolving] reveal-4 / add ≤2 / trash rest
// ═══════════════════════════════════════════════════════════════════════════

/// Positive path — reveal 4 = [LUCEMON-NAMED, ROYAL-KNIGHT, FILLER1, FILLER2].
/// Add the Lucemon-named card (bucket A) and the Royal Knight card (bucket B)
/// to hand; trash the two fillers.
#[test]
fn bt13_087_on_play_adds_two_matching_cards_trashes_rest() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT13-087")
        .expect("BT13-087 YAML loads")
        .add_card(make_digimon("LUCE", "Lucemon", 4, 3000, &["Demon Lord"]))
        .add_card(make_digimon("RK", "Examon", 6, 12000, &["Royal Knight"]))
        .add_card(make_test_card("FILL1", "Filler1"))
        .add_card(make_test_card("FILL2", "Filler2"))
        .deck(0, &["LUCE", "RK", "FILL1", "FILL2"])
        .hand(0, &["BT13-087"])
        .memory(10)
        .start();

    runner.play(0, 0).expect("play Dynasmon BT13-087");

    // Bucket A: Lucemon in name.
    pick_revealed_by_id(&mut runner, "LUCE", "pick Lucemon (name)");
    // Bucket B: Royal Knight trait.
    pick_revealed_by_id(&mut runner, "RK", "pick Royal Knight (trait)");

    runner.auto_resolve().expect("trash the rest");

    let hand = hand_ids(&runner, 0);
    assert!(
        hand.contains(&"LUCE".to_string()),
        "Lucemon-named card must enter hand; hand={:?}",
        hand
    );
    assert!(
        hand.contains(&"RK".to_string()),
        "Royal Knight card must enter hand; hand={:?}",
        hand
    );

    let trash = trash_ids(&runner, 0);
    assert!(
        trash.contains(&"FILL1".to_string()) && trash.contains(&"FILL2".to_string()),
        "the two non-matching revealed cards must be trashed (printed 'Trash the rest'); trash={:?}",
        trash
    );
    assert!(
        !trash.contains(&"LUCE".to_string()) && !trash.contains(&"RK".to_string()),
        "added cards must NOT be trashed; trash={:?}",
        trash
    );
}

/// No matching cards in the reveal — both buckets skip, all 4 revealed cards
/// are trashed, nothing enters hand.
#[test]
fn bt13_087_on_play_no_match_trashes_all_four() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT13-087")
        .expect("BT13-087 YAML loads")
        .add_card(make_test_card("F1", "Filler1"))
        .add_card(make_test_card("F2", "Filler2"))
        .add_card(make_test_card("F3", "Filler3"))
        .add_card(make_test_card("F4", "Filler4"))
        .deck(0, &["F1", "F2", "F3", "F4"])
        .hand(0, &["BT13-087"])
        .memory(10)
        .start();

    runner.play(0, 0).expect("play Dynasmon BT13-087");
    runner
        .auto_resolve()
        .expect("resolve through both empty buckets");

    let hand = hand_ids(&runner, 0);
    for f in ["F1", "F2", "F3", "F4"] {
        assert!(
            !hand.contains(&f.to_string()),
            "{f} must NOT enter hand when no bucket matches; hand={:?}",
            hand
        );
    }
    let trash = trash_ids(&runner, 0);
    for f in ["F1", "F2", "F3", "F4"] {
        assert!(
            trash.contains(&f.to_string()),
            "{f} must be trashed when unchosen; trash={:?}",
            trash
        );
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 3 — [Your Turn] observer: another Lucemon/Royal Knight played →
//             delete all opponent level 4 or lower Digimon
// ═══════════════════════════════════════════════════════════════════════════

/// Positive: Dynasmon on field; opponent has a Lv3 and a Lv5 Digimon. Playing
/// ANOTHER own Royal Knight Digimon during your turn deletes the opponent Lv3
/// (≤4) but spares the Lv5.
#[test]
fn bt13_087_observer_deletes_level_4_or_lower_when_another_matching_digimon_played() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT13-087")
        .expect("BT13-087 YAML loads")
        .add_card(make_digimon("ALLY-RK", "Gankoomon", 6, 11000, &["Royal Knight"]))
        .add_card(make_digimon("OPP-LV3", "OppLv3", 3, 3000, &[]))
        .add_card(make_digimon("OPP-LV5", "OppLv5", 5, 7000, &[]))
        .hand(0, &["ALLY-RK"])
        .memory(20)
        .start();
    // Player 0's turn (turn_count odd → first player active).
    runner.game.turn_count = 5;
    runner.place_on_field(0, "BT13-087", Some(0));
    runner.place_on_field(1, "OPP-LV3", Some(0));
    runner.place_on_field(1, "OPP-LV5", Some(0));

    assert_eq!(runner.battle_area_size(1), 2);

    runner.play(0, 0).expect("play another Royal Knight ally");

    // The delete-all is automatic (for_each, no player choice).
    assert!(
        runner.pending_selection().is_none(),
        "delete-all installs no selection (no player choice)"
    );

    let opp_field = field_ids(&runner, 1);
    assert_eq!(
        runner.battle_area_size(1),
        1,
        "opponent Lv3 (≤4) deleted, Lv5 survives; opp field={:?}",
        opp_field
    );
    assert!(
        opp_field.contains(&"OPP-LV5".to_string()),
        "the Lv5 opponent Digimon must survive; opp field={:?}",
        opp_field
    );
    assert!(
        !opp_field.contains(&"OPP-LV3".to_string()),
        "the Lv3 opponent Digimon must be deleted; opp field={:?}",
        opp_field
    );
}

/// Positive (name match): playing another own Digimon with [Lucemon] in its
/// name also fires the observer.
#[test]
fn bt13_087_observer_fires_on_lucemon_named_ally() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT13-087")
        .expect("BT13-087 YAML loads")
        .add_card(make_digimon("ALLY-LUCE", "Lucemon", 4, 3000, &["Demon Lord"]))
        .add_card(make_digimon("OPP-LV4", "OppLv4", 4, 4000, &[]))
        .hand(0, &["ALLY-LUCE"])
        .memory(20)
        .start();
    runner.game.turn_count = 5;
    runner.place_on_field(0, "BT13-087", Some(0));
    runner.place_on_field(1, "OPP-LV4", Some(0));

    runner.play(0, 0).expect("play a Lucemon-named ally");

    assert!(runner.pending_selection().is_none());
    assert_eq!(
        runner.battle_area_size(1),
        0,
        "opponent Lv4 Digimon must be deleted by the Lucemon-named trigger"
    );
}

/// Negative: playing a non-Lucemon / non-Royal-Knight Digimon does NOT fire the
/// observer; both opponent Digimon survive.
#[test]
fn bt13_087_observer_does_not_fire_on_non_matching_ally() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT13-087")
        .expect("BT13-087 YAML loads")
        .add_card(make_digimon("ALLY-PLAIN", "Agumon", 3, 2000, &["Reptile"]))
        .add_card(make_digimon("OPP-LV3", "OppLv3", 3, 3000, &[]))
        .add_card(make_digimon("OPP-LV4", "OppLv4", 4, 4000, &[]))
        .hand(0, &["ALLY-PLAIN"])
        .memory(20)
        .start();
    runner.game.turn_count = 5;
    runner.place_on_field(0, "BT13-087", Some(0));
    runner.place_on_field(1, "OPP-LV3", Some(0));
    runner.place_on_field(1, "OPP-LV4", Some(0));

    runner.play(0, 0).expect("play a plain (non-matching) ally");

    assert!(runner.pending_selection().is_none());
    assert_eq!(
        runner.battle_area_size(1),
        2,
        "non-matching ally must NOT trigger the delete; both opponent Digimon survive"
    );
}

/// Negative (self-play exclusion): firing the OnAllyPlayed event with Dynasmon
/// itself as the played source must NOT delete (event_target_is_source: false).
#[test]
fn bt13_087_observer_ignores_own_play_event() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT13-087")
        .expect("BT13-087 YAML loads")
        .add_card(make_digimon("OPP-LV3", "OppLv3", 3, 3000, &[]))
        .memory(20)
        .start();
    runner.game.turn_count = 5;
    let dynasmon = runner.place_on_field(0, "BT13-087", Some(0));
    runner.place_on_field(1, "OPP-LV3", Some(0));

    let card = runner.top_card(dynasmon);
    runner.game.enqueue_triggered(
        EffectTiming::OnAllyPlayed,
        TriggerSource::EnteredField {
            player: 0,
            permanent: dynasmon,
            card,
            effect_initiated: false,
        },
    );
    runner.game.drain_effect_queue();

    assert!(
        runner.pending_selection().is_none(),
        "event_target_is_source: false keeps Dynasmon from reacting to its own play"
    );
    assert_eq!(
        runner.battle_area_size(1),
        1,
        "self-play event must not delete an opponent Digimon"
    );
}

/// Negative (opponent-played): an OPPONENT playing a Royal Knight Digimon does
/// NOT fire Dynasmon's observer (event_target_owner: you).
#[test]
fn bt13_087_observer_ignores_opponent_played_matching_digimon() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT13-087")
        .expect("BT13-087 YAML loads")
        .add_card(make_digimon("OPP-RK", "Examon", 6, 12000, &["Royal Knight"]))
        .add_card(make_digimon("OPP-LV3", "OppLv3", 3, 3000, &[]))
        .memory(20)
        .start();
    runner.game.turn_count = 5;
    runner.place_on_field(0, "BT13-087", Some(0));
    let opp_rk = runner.place_on_field(1, "OPP-RK", Some(0));
    runner.place_on_field(1, "OPP-LV3", Some(0));

    let card = runner.top_card(opp_rk);
    runner.game.enqueue_triggered(
        EffectTiming::OnAllyPlayed,
        TriggerSource::EnteredField {
            player: 1,
            permanent: opp_rk,
            card,
            effect_initiated: false,
        },
    );
    runner.game.drain_effect_queue();

    assert!(runner.pending_selection().is_none());
    assert_eq!(
        runner.battle_area_size(1),
        2,
        "opponent playing a Royal Knight must NOT trigger our observer (event_target_owner: you)"
    );
}
