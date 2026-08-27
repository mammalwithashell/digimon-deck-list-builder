use digimon_engine::enums::{CardColor, Keyword, ModifierType};
use digimon_engine::replacement::ReplacementCause;
use digimon_engine::selection::SelectionKind;

use super::support::{
    field_contains, hand_contains, plain_digimon, push_to_trash, select_first_non_pass,
    select_hand_card, tb_digimon, DebugRunner,
};

const CARD_ID: &str = "EX12-047";

fn resolve_first_action(runner: &mut DebugRunner) {
    let (player, action) = {
        let pending = runner
            .game
            .pending_selection
            .as_ref()
            .expect("selection must be pending");
        (pending.selecting_player, pending.valid_action_ids[0])
    };
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
        vec!["TRASH-Y".to_string(), "TRASH-R".to_string()],
        "the two selected trash cards should move to opponent deck before the DP target prompt"
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
        vec!["TRASH-Y".to_string(), "TRASH-R".to_string()],
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
    // A RED Lv.5 [Shambala] base satisfies ONLY the alternative trait circle
    // ("Lv.5 w/[Shambala] trait: Cost 3") — the standard circle requires a
    // yellow base. The player main-phase digivolve action accepts this base;
    // an effect-initiated digivolve must too.
    let base = super::support::digimon("SHAMBALA-BASE", CardColor::Red, 5, 8, 7000, &["Shambala"]);
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
