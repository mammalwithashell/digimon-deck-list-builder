use digimon_engine::action::space::{HAND_EFFECT_START, PASS, PLAY_HAND_END, PLAY_HAND_START};
use digimon_engine::enums::{CardColor, Expiry, Keyword, ModifierType};
use digimon_engine::modifiers::PlayerModifierEntry;
use digimon_engine::replacement::ReplacementCause;
use digimon_engine::selection::SelectionKind;

use super::support::{
    field_contains, hand_contains, plain_digimon, push_to_trash, select_first_non_pass,
    select_hand_card, tb_digimon, DebugRunner,
};

const CARD_ID: &str = "EX12-047";

/// Walk the prompts (accepting each non-hand prompt with its first non-PASS
/// action) to the hand pick and return the card ids it offers, sorted. Empty
/// when no hand pick installs.
fn offered_hand_ids(runner: &mut DebugRunner, player: u8) -> Vec<String> {
    for _ in 0..8 {
        let Some(view) = runner.pending_selection_view() else {
            return Vec::new();
        };
        if view.kind == SelectionKind::Hand {
            let hand: Vec<String> = runner.game.players[player as usize]
                .hand
                .iter()
                .map(|c| c.card_id(&runner.game.card_data).to_string())
                .collect();
            let mut ids: Vec<String> = view
                .valid_action_ids
                .iter()
                .copied()
                .filter(|&a| a != PASS)
                .map(|a| {
                    let slot = if a < PLAY_HAND_END { a - PLAY_HAND_START } else { a - HAND_EFFECT_START };
                    hand[slot as usize].clone()
                })
                .collect();
            ids.sort();
            return ids;
        }
        let Some(action) = view.valid_action_ids.iter().copied().find(|&a| a != PASS) else {
            return Vec::new();
        };
        runner.execute_action(view.selecting_player, action).expect("advance prompt");
    }
    panic!("never reached the hand pick");
}

/// Amaterasumon (EX12-047) plus a filler on the field, TB-PLAY in hand,
/// TB-RETURN in the trash; returns the runner and Amaterasumon's handle.
fn on_deletion_runner() -> (DebugRunner, digimon_engine::permanent::PermanentHandle) {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-047 YAML loads")
        .add_card(tb_digimon("TB-RETURN", CardColor::Yellow, 4, 5000))
        .add_card(tb_digimon("TB-PLAY", CardColor::Yellow, 5, 7000))
        .add_card(plain_digimon("FILLER", CardColor::Yellow, 3, 3000))
        .hand(0, &["TB-PLAY"])
        .memory(10)
        .start();
    push_to_trash(&mut runner, 0, "TB-RETURN");
    let amaterasumon = runner.place_on_field(0, CARD_ID, Some(0));
    runner.place_on_field(0, "FILLER", Some(0));
    (runner, amaterasumon)
}

// ── [On Deletion] free play: candidate legality ──────────────────────────────
// DCGO EX12_047.cs:212-219 `CanSelectPlayCardCondition`: IsDigimon, Lv.5 or
// lower, [TB], AND `CardEffectCommons.CanPlayAsNewPermanent(cardSource,
// payCost: false, activateClass)` (GameContextDeterminarion.cs:419-432): not an
// Option (never a DUAL, CR 4.3 4-6-2), an empty battle-area frame, no play
// prohibition. A card the free play would refuse is not a legal choice
// (CR 4.3 1-3-2). DCGO also skips the play pick entirely when no card
// qualifies (EX12_047.cs:248).

/// "You can't play Digimon by effects": the returned and the held [TB] Digimon
/// can't be played, so neither is offered.
#[test]
fn ex12_047_on_deletion_play_pick_offers_nothing_under_effect_play_prohibition() {
    let (mut runner, amaterasumon) = on_deletion_runner();
    runner.game.modifiers.add_player_modifier(
        0,
        PlayerModifierEntry::simple(ModifierType::CannotPlayDigimonByEffect, 0, Expiry::Permanent, None, 1),
    );
    runner
        .game
        .delete_permanents_batch(vec![amaterasumon], ReplacementCause::OpponentEffect);
    assert!(offered_hand_ids(&mut runner, 0).is_empty());
    assert!(hand_contains(&runner, 0, "TB-PLAY"));
}

/// A full battle area (the filler fills a 1-slot area once Amaterasumon is
/// gone): nothing can be played, so nothing is offered.
#[test]
fn ex12_047_on_deletion_play_pick_offers_nothing_when_battle_area_full() {
    let (mut runner, amaterasumon) = on_deletion_runner();
    runner.game.rules.field_slots = 1;
    runner
        .game
        .delete_permanents_batch(vec![amaterasumon], ReplacementCause::OpponentEffect);
    assert!(offered_hand_ids(&mut runner, 0).is_empty());
    assert!(hand_contains(&runner, 0, "TB-PLAY"));
    assert!(!field_contains(&runner, 0, "TB-PLAY"));
}

fn resolve_first_action(runner: &mut DebugRunner) {
    let (player, action) = {
        let pending = runner
            .game
            .pending_selection
            .as_ref()
            .expect("selection must be pending");
        (pending.selecting_player, pending.valid_action_ids[0])
    };
    assert_eq!(
        digimon_engine::action::build_action_mask(&runner.game, player)[action as usize],
        1.0
    );
    runner
        .game
        .resolve_selection(player, action)
        .expect("selection resolves");
}

#[test]
fn ex12_047_has_printed_keywords() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-047 YAML loads")
        .start();
    let amaterasumon = runner.place_on_field(0, CARD_ID, Some(0));

    assert!(runner.game.has_keyword(amaterasumon, Keyword::Piercing));
    assert!(runner
        .game
        .has_keyword(amaterasumon, Keyword::SecurityAttackPlus(1)));
    assert!(runner.game.has_keyword(amaterasumon, Keyword::Ascension));
}

#[test]
fn ex12_047_on_play_deletes_lowest_dp_returns_two_trash_and_applies_dp_modifiers() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-047 YAML loads")
        .add_card(plain_digimon("LOW", CardColor::Red, 3, 1000))
        .add_card(plain_digimon("DP-TARGET", CardColor::Purple, 4, 15000))
        .add_card(plain_digimon("TRASH-R", CardColor::Red, 3, 3000))
        .add_card(plain_digimon("TRASH-Y", CardColor::Yellow, 3, 3000))
        .start();

    let amaterasumon = runner.place_on_field(0, CARD_ID, Some(0));
    let dp_target = runner.place_on_field(1, "DP-TARGET", Some(0));
    runner.place_on_field(1, "LOW", Some(0));
    push_to_trash(&mut runner, 1, "TRASH-R");
    push_to_trash(&mut runner, 1, "TRASH-Y");
    assert_eq!(
        runner.game.players[1]
            .trash
            .iter()
            .map(|card| card.card_id(&runner.game.card_data).to_string())
            .collect::<Vec<_>>(),
        vec!["TRASH-R".to_string(), "TRASH-Y".to_string()]
    );

    runner.fire_on_play(0, amaterasumon.index as usize);

    assert_eq!(
        runner.pending_kind(),
        Some(SelectionKind::OppField),
        "lowest-DP delete prompt should install first"
    );
    resolve_first_action(&mut runner);

    assert!(!field_contains(&runner, 1, "LOW"));
    assert_eq!(
        runner.game.players[1]
            .trash
            .iter()
            .map(|card| card.card_id(&runner.game.card_data).to_string())
            .collect::<Vec<_>>(),
        vec![
            "TRASH-R".to_string(),
            "TRASH-Y".to_string(),
            "LOW".to_string()
        ]
    );
    assert!(
        matches!(
            runner.pending_kind(),
            Some(SelectionKind::CountCappedMultiSelect {
                max: 2,
                picked: 0,
                ..
            })
        ),
        "return-2 trash multi-select should follow the delete; got {:?}",
        runner.pending_kind()
    );
    resolve_first_action(&mut runner);
    assert!(
        matches!(
            runner.pending_kind(),
            Some(SelectionKind::CountCappedMultiSelect {
                max: 2,
                picked: 1,
                ..
            })
        ),
        "return-2 trash multi-select should keep accumulating after one pick; got {:?}",
        runner.pending_kind()
    );
    assert_eq!(
        runner.game.players[1]
            .trash
            .iter()
            .map(|card| card.card_id(&runner.game.card_data).to_string())
            .collect::<Vec<_>>(),
        vec![
            "TRASH-R".to_string(),
            "TRASH-Y".to_string(),
            "LOW".to_string()
        ],
        "multi-pick should not move trash cards before the max is reached"
    );
    resolve_first_action(&mut runner);

    assert_eq!(
        runner.game.players[1]
            .deck
            .iter()
            .map(|card| card.card_id(&runner.game.card_data).to_string())
            .collect::<Vec<_>>(),
        vec!["TRASH-R".to_string(), "TRASH-Y".to_string()],
        "CR 4.3 3-1-3-4: selection order chooses placement order; first pick is deepest (deck index 0)"
    );
    assert_eq!(
        runner.pending_kind(),
        Some(SelectionKind::OppField),
        "DP reduction target prompt should install after returning two trash cards"
    );
    resolve_first_action(&mut runner);

    runner.auto_resolve().expect("resolve EX12-047 On Play");

    assert!(
        !field_contains(&runner, 1, "LOW"),
        "the selected lowest-DP Digimon should leave the field"
    );
    assert_eq!(
        runner.game.players[1]
            .deck
            .iter()
            .map(|card| card.card_id(&runner.game.card_data).to_string())
            .collect::<Vec<_>>(),
        vec!["TRASH-R".to_string(), "TRASH-Y".to_string()],
        "the two selected trash cards should remain on the bottom of the opponent's deck"
    );
    assert_eq!(
        runner
            .game
            .modifiers
            .sum(amaterasumon, ModifierType::ChangeDp),
        6000
    );
    assert_eq!(
        runner.game.modifiers.sum(dp_target, ModifierType::ChangeDp),
        -10000,
        "two distinct returned colors should apply -10000 DP"
    );
}

#[test]
fn ex12_047_on_deletion_returns_tb_from_trash_and_plays_level5_tb_from_hand() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-047 YAML loads")
        .add_card(tb_digimon("TB-RETURN", CardColor::Yellow, 4, 5000))
        .add_card(tb_digimon("TB-PLAY", CardColor::Yellow, 5, 7000))
        .hand(0, &["TB-PLAY"])
        .memory(10)
        .start();
    push_to_trash(&mut runner, 0, "TB-RETURN");

    let amaterasumon = runner.place_on_field(0, CARD_ID, Some(0));
    runner
        .game
        .delete_permanents_batch(vec![amaterasumon], ReplacementCause::OpponentEffect);

    select_first_non_pass(&mut runner);
    select_hand_card(&mut runner, 0, "TB-PLAY");
    runner.auto_resolve().expect("resolve EX12-047 On Deletion");

    assert!(hand_contains(&runner, 0, "TB-RETURN"));
    assert!(field_contains(&runner, 0, "TB-PLAY"));
}

// ── Effect-initiated digivolve vs the printed digivolution circles ──────────
//
// EX12-047 prints TWO digivolve circles:
//   1. "Lv.5 from Yellow: Cost 4"            (standard color circle)
//   2. "Lv.5 w/[Shambala] trait: Cost 3"     (alternative trait circle)
// Both are authored as `alt_paths: kind: digivolve` in EX12-047.yaml. The
// player main-phase action honors both via `all_digivolve_routes_for_card`
// (which folds in `collect_dsl_alt_digivolve_routes`). An EFFECT-initiated
// digivolve (`Game::effect_initiated_digivolve`, the target of the DSL
// `effect_initiated_digivolve:` step) must honor the same printed circles.

#[test]
fn ex12_047_effect_initiated_digivolve_uses_standard_yellow_circle() {
    // Control: a Lv.5 YELLOW base satisfies the standard circle (Cost 4).
    let base = super::support::digimon("YELLOW-BASE", CardColor::Yellow, 5, 8, 7000, &[]);
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-047 YAML loads")
        .add_card(base)
        .hand(0, &[CARD_ID])
        .memory(10)
        .start();
    let target = runner.place_on_field(0, "YELLOW-BASE", Some(0));

    let ok = runner.game.effect_initiated_digivolve(
        0,
        0,
        target,
        digimon_engine::enums::CostDelta::Reduce(0),
        false,
        digimon_engine::enums::PlaySource::ByEffect,
    );
    assert!(
        ok,
        "effect-initiated digivolve over a Lv.5 yellow base must use the standard circle"
    );
    assert_eq!(
        runner.game.players[0].battle_area[target.index as usize]
            .top_card()
            .card_id(&runner.game.card_data),
        CARD_ID
    );
    assert_eq!(runner.memory(), 6, "standard circle costs 4 (10 - 4 = 6)");
}

#[test]
fn ex12_047_effect_initiated_digivolve_honors_shambala_trait_circle() {
    // A GREEN Lv.5 [Shambala] base satisfies ONLY the alternative trait circle
    // ("Lv.5 w/[Shambala] trait: Cost 3") — the printed circle is split
    // Yellow/Red (card image), so neither colour matches. The player main-phase digivolve action accepts this base;
    // an effect-initiated digivolve must too.
    let base = super::support::digimon("SHAMBALA-BASE", CardColor::Green, 5, 8, 7000, &["Shambala"]);
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-047 YAML loads")
        .add_card(base)
        .hand(0, &[CARD_ID])
        .memory(10)
        .start();
    let target = runner.place_on_field(0, "SHAMBALA-BASE", Some(0));

    let ok = runner.game.effect_initiated_digivolve(
        0,
        0,
        target,
        digimon_engine::enums::CostDelta::Reduce(0),
        false,
        digimon_engine::enums::PlaySource::ByEffect,
    );
    assert!(
        ok,
        "effect-initiated digivolve must honor the alternative trait circle \
         (Lv.5 w/[Shambala] trait: Cost 3) — the base qualifies for the player \
         main-phase action but is rejected by Game::effect_initiated_digivolve, \
         which only matches printed `evo_costs` (level + color)"
    );
    assert_eq!(
        runner.game.players[0].battle_area[target.index as usize]
            .top_card()
            .card_id(&runner.game.card_data),
        CARD_ID
    );
    assert_eq!(runner.memory(), 7, "trait circle costs 3 (10 - 3 = 7)");
}

/// §15-4-4-3: "When a card with an effect that's pending activation becomes a
/// NEW CARD before the effect activates, the effect can no longer be
/// activated."
///
/// Deleting EX12-047 triggers `<Ascension>` and `[On Deletion]` SIMULTANEOUSLY,
/// and §15-4-3-5-1 lets the controller choose which activates first. That
/// choice is a FORFEIT, not a preference: `<Ascension>` places the card on top
/// of the security stack, so it changes areas, becomes a new card, and the
/// still-pending `[On Deletion]` is lost.
///
/// `queued_effect_source_is_live` has a blanket bypass for batched `OnDeletion`
/// entries -- correct in general, because the batched flow trashes the carrier
/// BEFORE the drain, so a "still on field" test would fail for every one of
/// them -- but it returned `true` without asking WHERE the card now is. The
/// `[On Deletion]` therefore fired even after Ascension had moved the card out
/// of the trash, handing the controller both halves of a choice the rules make
/// exclusive.
#[test]
fn ex12_047_ascension_first_makes_the_on_deletion_miss_timing() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-047 YAML loads")
        .add_card(tb_digimon("TB-RETURN", CardColor::Yellow, 4, 5000))
        .add_card(tb_digimon("TB-PLAY", CardColor::Yellow, 5, 7000))
        .hand(0, &["TB-PLAY"])
        .memory(10)
        .start();
    push_to_trash(&mut runner, 0, "TB-RETURN");
    let sec_before = runner.security_count(0);

    let amaterasumon = runner.place_on_field(0, CARD_ID, Some(0));
    runner
        .game
        .delete_permanents_batch(vec![amaterasumon], ReplacementCause::OpponentEffect);

    // The trigger-order prompt lists this card's two simultaneous triggers.
    // Our engine offers [On Deletion] at slot 0 and <Ascension> at slot 1, so
    // take slot 1 to put ASCENSION FIRST -- the branch under test. Asserted,
    // not assumed: if the slots ever swap, the security assertion below fails
    // rather than the test quietly exercising the other order.
    {
        let (player, action) = {
            let pending = runner
                .game
                .pending_selection
                .as_ref()
                .expect("the two simultaneous triggers raise an order prompt");
            assert!(
                pending.valid_action_ids.len() >= 2,
                "both <Ascension> and [On Deletion] must be offered (15-4-3-5-1)"
            );
            (pending.selecting_player, pending.valid_action_ids[1])
        };
        runner
            .game
            .resolve_selection(player, action)
            .expect("choose <Ascension> to activate first");
    }
    // Accept Ascension's "place this card as the top security card?" and let
    // anything else settle.
    for _ in 0..8 {
        if runner.game.pending_selection.is_none() {
            break;
        }
        resolve_first_action(&mut runner);
    }
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.security_count(0),
        sec_before + 1,
        "<Ascension> placed EX12-047 as the top security card"
    );
    // §15-4-4-3: the [On Deletion] is LOST -- neither half of it may resolve.
    assert!(
        !hand_contains(&runner, 0, "TB-RETURN"),
        "the pending [On Deletion] must not activate after <Ascension> moved its \
         card out of the trash -- TB-RETURN stays in the trash"
    );
    assert!(
        !field_contains(&runner, 0, "TB-PLAY"),
        "the [On Deletion]'s follow-on play must not happen either"
    );
}

/// "Then, by returning 2 cards from your opponent's trash ..." is optional
/// (DCGO canNoSelect: true): declining returns nothing and gives no +6000 DP.
#[test]
fn ex12_047_return_two_can_be_declined() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-047 YAML loads")
        .add_card(plain_digimon("LOW", CardColor::Red, 3, 1000))
        .add_card(plain_digimon("TRASH-R", CardColor::Red, 3, 3000))
        .add_card(plain_digimon("TRASH-Y", CardColor::Yellow, 3, 3000))
        .start();
    let amaterasumon = runner.place_on_field(0, CARD_ID, Some(0));
    runner.place_on_field(1, "LOW", Some(0));
    push_to_trash(&mut runner, 1, "TRASH-R");
    push_to_trash(&mut runner, 1, "TRASH-Y");
    let dp_before = runner.effective_dp(amaterasumon);

    runner.fire_on_play(0, amaterasumon.index as usize);
    resolve_first_action(&mut runner); // delete LOW
    let pending = runner.pending_selection().expect("return-2 pick");
    assert!(pending.is_optional, "the return-2 pick can be declined");
    let player = pending.selecting_player;
    runner
        .execute_action(player, digimon_engine::action::space::PASS)
        .expect("decline the return");
    let _ = runner.auto_resolve();

    assert_eq!(runner.game.players[1].trash.len(), 3, "nothing returned");
    assert_eq!(runner.effective_dp(amaterasumon), dp_before, "no +6000 DP");
}
