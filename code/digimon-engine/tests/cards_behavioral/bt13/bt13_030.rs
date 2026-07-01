//! BT13-030 UlforceVeedramon — Digimon, Lv.6, Blue, DP 11000, Cost 11.
//! Traits: Holy Warrior / Royal Knight.
//!
//! Printed text (card image, verbatim):
//!   [On Play][When Digivolving] For each of your Digimon with the
//!     [Royal Knight] trait and each of your blue Tamers, trash the top 2
//!     digivolution cards of 1 of your opponent's Digimon.
//!   [Your Turn][Once Per Turn] When you play a Digimon with the
//!     [Royal Knight] trait or a blue Tamer, return 1 of your opponent's
//!     Digimon with no digivolution cards to the hand.
//!
//! DCGO behavioral oracle: DCGO/Assets/Scripts/CardEffect/BT13/Blue/BT13_030.cs
//!   Clause 1 ([On Play] & [When Digivolving], two identical OnEnterFieldAnyone
//!     blocks): ONE mandatory SelectPermanentEffect (maxCount = min(1, ...),
//!     canNoSelect:false) over opponent Digimon that have >=1 trashable
//!     digivolution card, then TrashDigivolutionCardsFromTopOrBottom with
//!     trashCount = `2 * Owner.GetBattleAreaPermanents().Count(p =>
//!       (p.IsDigimon && p.TopCard.HasRoyalKnightTraits)
//!       || (p.TopCard.CardColors.Contains(Blue) && p.IsTamer))`.
//!     => SINGLE pick, trash 2N from the top (NOT N separate picks).
//!   Clause 2 ([Your Turn][Once Per Turn] OnPermanentPlay observer): when an
//!     own Royal-Knight Digimon OR own blue Tamer is played, return 1 opponent
//!     Digimon with NO digivolution cards to the hand (Mode.Bounce, maxCount
//!     min(1,...), canNoSelect:false). Owner-turn gated.

use digimon_engine::action::space::encode_attack;
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, make_test_card_with_level, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming};
use digimon_engine::selection::{SelectionKind, TriggerSource};

const CARD_ID: &str = "BT13-030";

fn digimon(id: &str, name: &str, level: u8, dp: i32) -> CardData {
    let mut card = make_test_card_with_level(id, name, level);
    card.card_kind = CardKind::Digimon;
    card.dp = Some(dp);
    card
}

/// A Digimon carrying the [Royal Knight] trait (counts toward N).
fn royal_knight(id: &str, name: &str, level: u8, dp: i32) -> CardData {
    let mut card = digimon(id, name, level, dp);
    card.traits = vec!["Holy Warrior".to_string(), "Royal Knight".to_string()];
    card
}

/// A Tamer with the given colors (a blue one counts toward N).
fn tamer_with_colors(id: &str, name: &str, colors: &[CardColor]) -> CardData {
    let mut card = make_test_card(id, name);
    card.card_kind = CardKind::Tamer;
    card.level = None;
    card.dp = None;
    card.colors = colors.to_vec();
    card
}

fn runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT13-030 YAML loads")
        // Stack-source cards for the opponent victim (filler digivolution cards).
        .add_card(digimon("SRC-A", "Source A", 3, 1000))
        .add_card(digimon("SRC-B", "Source B", 4, 4000))
        .add_card(digimon("SRC-C", "Source C", 4, 4000))
        .add_card(digimon("SRC-D", "Source D", 4, 4000))
        .add_card(digimon("SRC-E", "Source E", 4, 4000))
        .add_card(digimon("SRC-F", "Source F", 4, 4000))
        .add_card(digimon("SRC-G", "Source G", 4, 4000))
        .add_card(digimon("TOP-OPP", "Opp Top", 6, 9000))
        // No-source opponent Digimon (bounce target / ineligible trash target).
        .add_card(digimon("OPP-NOSRC", "Opp NoSrc", 5, 5000))
        // Own counted objects.
        .add_card(royal_knight("ALLY-RK", "Ally Royal Knight", 5, 6000))
        .add_card(tamer_with_colors(
            "BLUE-TAMER",
            "Blue Tamer",
            &[CardColor::Blue],
        ))
        // Own NON-counted objects (must NOT count toward N).
        .add_card(digimon("ALLY-PLAIN", "Plain Ally", 5, 6000))
        .add_card(tamer_with_colors(
            "RED-TAMER",
            "Red Tamer",
            &[CardColor::Red],
        ))
        .memory(10)
        .start()
}

/// Build the opponent victim with `source_count` digivolution cards beneath an
/// opponent top card. Returns its handle.
fn place_opp_victim(
    runner: &mut DebugRunner,
    source_count: usize,
) -> digimon_engine::permanent::PermanentHandle {
    let pool = [
        "SRC-A", "SRC-B", "SRC-C", "SRC-D", "SRC-E", "SRC-F", "SRC-G",
    ];
    let mut ids: Vec<&str> = pool[..source_count].to_vec();
    ids.push("TOP-OPP");
    runner.place_stack(1, &ids)
}

// ───────────────────── Clause 1 — [On Play]/[When Digivolving] trash 2N ─────

#[test]
fn bt13_030_loads_with_gap_stub() {
    DebugRunner::builder()
        .dsl_card("BT13-030")
        .expect("load")
        .start();
}

/// N = 3 (UlforceVeedramon itself is a Royal Knight + 1 other RK Digimon +
/// 1 blue Tamer) => trash 2*3 = 6 digivolution cards from the single chosen
/// opponent Digimon.
#[test]
fn bt13_030_trashes_six_sources_when_n_is_three() {
    let mut runner = runner();
    let victim = place_opp_victim(&mut runner, 7); // 7 sources + 1 top
    let ulforce = runner.place_on_field(0, CARD_ID, Some(0));
    runner.place_on_field(0, "ALLY-RK", Some(0));
    runner.place_on_field(0, "BLUE-TAMER", Some(0));
    // Non-counted decoys.
    runner.place_on_field(0, "ALLY-PLAIN", Some(0));
    runner.place_on_field(0, "RED-TAMER", Some(0));

    let before = runner.game.players[1].battle_area[victim.index as usize]
        .card_sources
        .len();
    assert_eq!(before, 8, "victim starts with 7 sources + 1 top card");

    runner
        .game
        .enqueue_triggered(EffectTiming::OnPlay, TriggerSource::Permanent(ulforce));
    runner.game.drain_effect_queue();

    let view = runner
        .pending_selection_view()
        .expect("On Play must install a single mandatory opponent-Digimon pick");
    assert_eq!(view.kind, SelectionKind::OppField);
    assert!(!view.is_optional, "DCGO canNoSelect:false — mandatory pick");
    runner
        .execute_action(0, encode_attack(0, victim.index as u16))
        .expect("pick the opponent victim");
    runner.auto_resolve().expect("resolve trash");

    let after = runner.game.players[1].battle_area[victim.index as usize]
        .card_sources
        .len();
    assert_eq!(
        before - after,
        6,
        "N=3 (self RK + ally RK + blue Tamer) => trash 2*3 = 6 sources"
    );
}

/// N = 1 (only UlforceVeedramon itself qualifies — no extra RK, no blue Tamer)
/// => trash exactly 2.
#[test]
fn bt13_030_trashes_two_sources_when_n_is_one() {
    let mut runner = runner();
    let victim = place_opp_victim(&mut runner, 5);
    let ulforce = runner.place_on_field(0, CARD_ID, Some(0));
    // Only non-counted allies present.
    runner.place_on_field(0, "ALLY-PLAIN", Some(0));
    runner.place_on_field(0, "RED-TAMER", Some(0));

    let before = runner.game.players[1].battle_area[victim.index as usize]
        .card_sources
        .len();

    runner
        .game
        .enqueue_triggered(EffectTiming::OnPlay, TriggerSource::Permanent(ulforce));
    runner.game.drain_effect_queue();
    runner
        .execute_action(0, encode_attack(0, victim.index as u16))
        .expect("pick the opponent victim");
    runner.auto_resolve().expect("resolve trash");

    let after = runner.game.players[1].battle_area[victim.index as usize]
        .card_sources
        .len();
    assert_eq!(
        before - after,
        2,
        "N=1 (only UlforceVeedramon is a Royal Knight) => trash 2 sources"
    );
}

/// The [When Digivolving] timing fires the same trash clause.
#[test]
fn bt13_030_when_digivolving_also_trashes() {
    let mut runner = runner();
    let victim = place_opp_victim(&mut runner, 5);
    let ulforce = runner.place_on_field(0, CARD_ID, Some(0));

    let before = runner.game.players[1].battle_area[victim.index as usize]
        .card_sources
        .len();

    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(ulforce),
    );
    runner.game.drain_effect_queue();
    runner
        .execute_action(0, encode_attack(0, victim.index as u16))
        .expect("pick the opponent victim");
    runner.auto_resolve().expect("resolve trash");

    let after = runner.game.players[1].battle_area[victim.index as usize]
        .card_sources
        .len();
    assert_eq!(
        before - after,
        2,
        "[When Digivolving] also trashes 2N (N=1)"
    );
}

/// Eligibility: a clamp test — trashing 2N off a short stack stops at the top
/// card and never removes the visible Digimon.
#[test]
fn bt13_030_trash_clamps_to_available_sources() {
    let mut runner = runner();
    // Only 1 source beneath the top; N=1 wants to trash 2 but only 1 exists.
    let victim = place_opp_victim(&mut runner, 1);
    let ulforce = runner.place_on_field(0, CARD_ID, Some(0));

    runner
        .game
        .enqueue_triggered(EffectTiming::OnPlay, TriggerSource::Permanent(ulforce));
    runner.game.drain_effect_queue();
    runner
        .execute_action(0, encode_attack(0, victim.index as u16))
        .expect("pick the opponent victim");
    runner.auto_resolve().expect("resolve trash");

    let after = runner.game.players[1].battle_area[victim.index as usize]
        .card_sources
        .len();
    assert_eq!(
        after, 1,
        "trash clamps at the visible top card (no over-trash)"
    );
    assert_eq!(
        runner.battle_area_size(1),
        1,
        "victim is not deleted by source trashing"
    );
}

/// Eligibility: an opponent Digimon with NO digivolution cards is not a legal
/// trash target (DCGO CanSelectPermanentCondition requires >=1 source).
#[test]
fn bt13_030_no_source_opp_is_not_a_trash_target() {
    let mut runner = runner();
    let victim = place_opp_victim(&mut runner, 3);
    let no_src = runner.place_on_field(1, "OPP-NOSRC", Some(0));
    let ulforce = runner.place_on_field(0, CARD_ID, Some(0));

    runner
        .game
        .enqueue_triggered(EffectTiming::OnPlay, TriggerSource::Permanent(ulforce));
    runner.game.drain_effect_queue();

    let view = runner
        .pending_selection_view()
        .expect("On Play installs a pick");
    assert!(
        view.valid_action_ids
            .contains(&encode_attack(0, victim.index as u16)),
        "the stacked victim is a legal target"
    );
    assert!(
        !view
            .valid_action_ids
            .contains(&encode_attack(0, no_src.index as u16)),
        "an opponent Digimon with no digivolution cards is NOT a legal trash target"
    );
}

/// No opponent Digimon with sources => no prompt installs (gate fails).
#[test]
fn bt13_030_no_trash_prompt_without_eligible_opp() {
    let mut runner = runner();
    runner.place_on_field(1, "OPP-NOSRC", Some(0)); // 0 sources, ineligible
    let ulforce = runner.place_on_field(0, CARD_ID, Some(0));

    runner
        .game
        .enqueue_triggered(EffectTiming::OnPlay, TriggerSource::Permanent(ulforce));
    runner.game.drain_effect_queue();

    assert!(
        runner.pending_selection().is_none(),
        "no eligible opponent Digimon (>=1 source) => no trash prompt"
    );
}

// ───────────────────── Clause 2 — [Your Turn][OPT] bounce ───────────────────

/// Playing an own blue Tamer triggers the bounce observer: return 1 opponent
/// Digimon with NO digivolution cards to the hand.
#[test]
fn bt13_030_bounce_fires_on_blue_tamer_play() {
    let mut runner = runner();
    let no_src = runner.place_on_field(1, "OPP-NOSRC", Some(0));
    runner.place_on_field(0, CARD_ID, Some(0)); // UlforceVeedramon on field, observer armed
    let opp_hand_before = runner.game.players[1].hand.len();

    // Play a blue Tamer — fires the full play-event bundle (OnEnterFieldAnyone)
    // with the Tamer as event target.
    let tamer = runner.place_on_field(0, "BLUE-TAMER", Some(0));
    runner.fire_play_event_triggers(0, tamer.index as usize, false, false);

    let view = runner
        .pending_selection_view()
        .expect("blue-Tamer play must arm the bounce observer");
    assert_eq!(view.kind, SelectionKind::OppField);
    assert!(
        view.valid_action_ids
            .contains(&encode_attack(0, no_src.index as u16)),
        "the no-source opponent Digimon is a legal bounce target"
    );
    runner.auto_resolve().expect("resolve bounce");

    assert_eq!(
        runner.battle_area_size(1),
        0,
        "the no-source opponent Digimon left the field (bounced)"
    );
    assert_eq!(
        runner.game.players[1].hand.len(),
        opp_hand_before + 1,
        "bounced Digimon returned to its owner's hand"
    );
}

/// Playing an own Royal-Knight Digimon also triggers the bounce observer.
#[test]
fn bt13_030_bounce_fires_on_royal_knight_play() {
    let mut runner = runner();
    let no_src = runner.place_on_field(1, "OPP-NOSRC", Some(0));
    runner.place_on_field(0, CARD_ID, Some(0));

    let rk = runner.place_on_field(0, "ALLY-RK", Some(0));
    runner.fire_play_event_triggers(0, rk.index as usize, false, false);

    let view = runner
        .pending_selection_view()
        .expect("Royal-Knight play must arm the bounce observer");
    assert!(view
        .valid_action_ids
        .contains(&encode_attack(0, no_src.index as u16)));
}

/// The bounce observer only targets opponent Digimon with NO digivolution
/// cards; a stacked opponent Digimon is not a legal bounce target.
#[test]
fn bt13_030_bounce_targets_only_no_source_opp() {
    let mut runner = runner();
    let stacked = place_opp_victim(&mut runner, 3);
    let no_src = runner.place_on_field(1, "OPP-NOSRC", Some(0));
    runner.place_on_field(0, CARD_ID, Some(0));

    let rk = runner.place_on_field(0, "ALLY-RK", Some(0));
    runner.fire_play_event_triggers(0, rk.index as usize, false, false);

    let view = runner
        .pending_selection_view()
        .expect("bounce observer arms");
    assert!(
        view.valid_action_ids
            .contains(&encode_attack(0, no_src.index as u16)),
        "no-source opponent Digimon is a legal bounce target"
    );
    assert!(
        !view
            .valid_action_ids
            .contains(&encode_attack(0, stacked.index as u16)),
        "a stacked opponent Digimon is NOT a legal bounce target"
    );
}

/// Playing a NON-Royal-Knight Digimon and a NON-blue Tamer must NOT arm the
/// bounce observer.
#[test]
fn bt13_030_bounce_ignores_non_rk_non_blue_play() {
    let mut runner = runner();
    runner.place_on_field(1, "OPP-NOSRC", Some(0));
    runner.place_on_field(0, CARD_ID, Some(0));

    // Plain (non-RK) Digimon — no trigger (observer broadcast fires, but the
    // condition filters it out).
    let plain = runner.place_on_field(0, "ALLY-PLAIN", Some(0));
    runner.fire_play_event_triggers(0, plain.index as usize, false, false);
    assert!(
        runner.pending_selection().is_none(),
        "playing a non-Royal-Knight Digimon must not arm the bounce observer"
    );

    // Red Tamer — no trigger.
    let red = runner.place_on_field(0, "RED-TAMER", Some(0));
    runner.fire_play_event_triggers(0, red.index as usize, false, false);
    assert!(
        runner.pending_selection().is_none(),
        "playing a non-blue Tamer must not arm the bounce observer"
    );
}

/// The bounce observer is [Once Per Turn]: a second qualifying play in the
/// same turn does not re-arm it.
#[test]
fn bt13_030_bounce_is_once_per_turn() {
    let mut runner = runner();
    // Two no-source opponent Digimon so a second bounce *would* have a target.
    runner.place_on_field(1, "OPP-NOSRC", Some(0));
    runner.place_on_field(1, "OPP-NOSRC", Some(0));
    runner.place_on_field(0, CARD_ID, Some(0));
    assert_eq!(runner.battle_area_size(1), 2, "two opp Digimon to start");

    // First qualifying play — fire + resolve the bounce (removes 1 of 2).
    let rk = runner.place_on_field(0, "ALLY-RK", Some(0));
    runner.fire_play_event_triggers(0, rk.index as usize, false, false);
    assert!(
        runner.pending_selection().is_some(),
        "first qualifying play arms the bounce"
    );
    runner.auto_resolve().expect("resolve first bounce");
    assert_eq!(
        runner.battle_area_size(1),
        1,
        "first bounce removed exactly one opponent Digimon"
    );

    // Second qualifying play in the SAME turn — must NOT re-arm.
    let tamer = runner.place_on_field(0, "BLUE-TAMER", Some(0));
    runner.fire_play_event_triggers(0, tamer.index as usize, false, false);
    assert!(
        runner.pending_selection().is_none(),
        "[Once Per Turn] — a second qualifying play in the same turn must not re-arm"
    );
    assert_eq!(
        runner.battle_area_size(1),
        1,
        "second qualifying play does not bounce again (OPT consumed)"
    );
}
