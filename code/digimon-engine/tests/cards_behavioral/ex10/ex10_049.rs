//! EX10-049 SkullSatamon - Digimon, Lv.5, Purple, DP 8000, Cost 8.
//! Traits: [Undead] + [Fallen Angel] (Rule box). Form: Ultimate. Attribute: Virus.
//!
//! # Card text (data/card_bundles/EX10-049.md - verbatim)
//!
//! ```text
//! <Blocker>
//! [When Digivolving] [On Deletion] If your opponent has 10 or fewer cards in
//!   their trash, trash the top 3 cards of both players' decks. Then, delete 1
//!   of your opponent's level 3 or lower Digimon. If your opponent has 10 or
//!   more cards in their trash, add 2 to this effect's level maximum.
//!   (Rule) Trait: Has [Fallen Angel] Type.
//! Inherited: [When Attacking] [Once Per Turn] This Digimon gains <Security
//!   A. +1> for the turn. If your opponent has 10 or fewer cards in their
//!   trash, instead trash the top 2 cards of both players' decks.
//! ```
//!
//! # DCGO C# reference
//! `DCGO/Assets/Scripts/CardEffect/EX10/Purple/EX10_049.cs`
//!   - WD/OD shared body: mill 3+3 iff opp trash <= 10; the level maximum
//!     (`10 or more -> 5`, else 3) is read AFTER the mill; the delete pick is
//!     mandatory (`canNoSelect: false`) and runs even when the mill was
//!     skipped.
//!   - Inherited WA OPT: `hasSecAttack = opp trash >= 11`; <Security A. +1>
//!     when true, otherwise ONLY the 2+2 mill ("instead" replaces).
//!
//! Regression origin: a recorded PvP game digivolved Devimon (EX10-046) into
//! SkullSatamon and the recorded delete pick met no prompt because neither
//! card had a spec. These tests use synthetic boards.

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_dsl::compiled::{CompiledClause, CompiledDeclarativeClause, CompiledScope, CompiledTiming};
use digimon_engine::action::space::{encode_digivolve, PASS};
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::{SelectionKind, TriggerSource};

const CARD_ID: &str = "EX10-049";

fn digimon(id: &str, level: u8, color: CardColor) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(level);
    c.dp = Some(3000 + 1000 * level as i32);
    c.play_cost = 3;
    c.colors = vec![color];
    c
}

/// Deletes the controller's first other Digimon (fires [On Deletion] of a
/// SkullSatamon placed before it).
const DELETER_YAML: &str = r#"
card: DSL-DELETER
name: Deleter
kind: digimon
level: 4
color: [red]
cost: 0
dp: 3000
effects:
  - when: on_play
    summary: "[On Play] Delete 1 of your other Digimon"
    process:
      - select_own_permanent:
          bind_as: victim
          filter: { kind: digimon, other: true }
          prompt: "Delete 1 of your other Digimon"
      - delete_permanent: { target: victim }
"#;

fn base() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX10-049 YAML parses and compiles")
        .from_dsl_yaml(DELETER_YAML)
        .expect("deleter fixture compiles")
        .add_card(make_test_card("PAD", "Filler"))
        .add_card(make_test_card("TRASH-FILLER", "Trash Filler"))
        .add_card(digimon("BASE-L4", 4, CardColor::Purple))
        .add_card(digimon("OPP-L3", 3, CardColor::Red))
        .add_card(digimon("OPP-L4", 4, CardColor::Red))
        .add_card(digimon("OPP-L5", 5, CardColor::Red))
        .add_card(digimon("OPP-L6", 6, CardColor::Red))
        .deck(0, &["PAD"; 20])
        .deck(1, &["PAD"; 20])
}

fn seed_trash(runner: &mut DebugRunner, player: u8, count: usize) {
    let idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == "TRASH-FILLER")
        .unwrap();
    for _ in 0..count {
        let next = runner.game.next_card_index();
        runner.game.players[player as usize]
            .trash
            .push(CardSource::new(idx, player, next));
    }
}

/// SkullSatamon on the field with its [When Digivolving] trigger fired (the
/// DebugRunner card data carries no standard evo circle, so the trigger is
/// queued directly, as the other `cards_behavioral` WD tests do).
fn digivolve_skullsatamon(runner: &mut DebugRunner) -> PermanentHandle {
    let h = runner.place_on_field(0, CARD_ID, Some(0));
    runner
        .game
        .enqueue_triggered(EffectTiming::WhenDigivolving, TriggerSource::Permanent(h));
    runner.game.drain_effect_queue();
    h
}

fn offered_count(runner: &DebugRunner) -> usize {
    runner
        .pending_selection()
        .expect("delete pick parked")
        .valid_action_ids
        .iter()
        .filter(|&&id| id != PASS)
        .count()
}

fn opp_ids(runner: &DebugRunner) -> Vec<String> {
    runner.game.players[1]
        .battle_area
        .iter()
        .map(|p| p.top_card().card_id(&runner.game.card_data).to_string())
        .collect()
}

fn opp_board() -> Vec<&'static str> {
    vec!["OPP-L3", "OPP-L4", "OPP-L5", "OPP-L6"]
}

fn with_opp_board(runner: &mut DebugRunner) {
    for id in opp_board() {
        runner.place_on_field(1, id, Some(0));
    }
}

// ─── Structure ────────────────────────────────────────────────────────────────

#[test]
fn ex10_049_has_printed_metadata_blocker_and_three_clauses() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled");
    assert_eq!(card.name, "SkullSatamon");
    assert_eq!(card.level, Some(5));
    assert_eq!(card.dp, Some(8000));
    assert!(card.traits.iter().any(|t| t == "Fallen Angel"), "(Rule) Trait: Has [Fallen Angel] Type");
    assert!(card.effects.iter().any(|c| matches!(
        c,
        CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword { keyword, scope, .. })
            if keyword.eq_ignore_ascii_case("Blocker") && *scope == CompiledScope::FaceUp
    )));
    let shared = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t)
                if t.when.contains(&CompiledTiming::WhenDigivolving) =>
            {
                Some(t)
            }
            _ => None,
        })
        .expect("[When Digivolving] clause");
    assert!(shared.when.contains(&CompiledTiming::OnDeletion), "shares the [On Deletion] timing");
    assert!(!shared.optional, "mandatory: no \"may\"");
}

// ─── [When Digivolving]: opponent trash well under 10 ─────────────────────────

#[test]
fn ex10_049_wd_mills_three_each_then_deletes_only_a_level_3_or_lower() {
    let mut runner = base().memory(10).start();
    with_opp_board(&mut runner);
    let d0 = runner.deck_size(0);
    let d1 = runner.deck_size(1);

    digivolve_skullsatamon(&mut runner);

    assert_eq!(runner.deck_size(0), d0 - 3, "your deck mills 3");
    assert_eq!(runner.deck_size(1), d1 - 3, "opponent's deck mills 3");
    // Opp trash is 3 after the mill (< 10): level maximum stays 3.
    assert_eq!(offered_count(&runner), 1, "only the level-3 Digimon is deletable");
    runner.auto_resolve().expect("delete the level 3");
    assert!(!opp_ids(&runner).contains(&"OPP-L3".to_string()));
    assert_eq!(opp_ids(&runner).len(), 3, "the other three survive");
}

#[test]
fn ex10_049_wd_mill_that_pushes_opponent_trash_to_10_raises_the_level_maximum() {
    // 9 cards: <= 10 so the mill happens; the mill moves the opponent's trash
    // to 12; the level maximum is re-read AFTER the mill, so it is 5.
    let mut runner = base().memory(10).start();
    with_opp_board(&mut runner);
    seed_trash(&mut runner, 1, 9);
    let d1 = runner.deck_size(1);

    digivolve_skullsatamon(&mut runner);

    assert_eq!(runner.deck_size(1), d1 - 3, "mill ran (9 <= 10)");
    assert_eq!(runner.game.players[1].trash.len(), 12);
    assert_eq!(
        offered_count(&runner),
        3,
        "level 3, 4 and 5 are deletable (post-mill trash >= 10 adds 2 to the maximum); level 6 is not"
    );
}

#[test]
fn ex10_049_wd_at_exactly_10_both_mills_and_raises_the_maximum() {
    let mut runner = base().memory(10).start();
    with_opp_board(&mut runner);
    seed_trash(&mut runner, 1, 10);
    let d0 = runner.deck_size(0);

    digivolve_skullsatamon(&mut runner);

    assert_eq!(runner.deck_size(0), d0 - 3, "10 is 'or fewer': the mill runs");
    assert_eq!(offered_count(&runner), 3, "10 is also 'or more': level maximum 5");
}

#[test]
fn ex10_049_wd_over_10_skips_the_mill_but_still_deletes_up_to_level_5() {
    let mut runner = base().memory(10).start();
    with_opp_board(&mut runner);
    seed_trash(&mut runner, 1, 11);
    let d0 = runner.deck_size(0);
    let d1 = runner.deck_size(1);

    digivolve_skullsatamon(&mut runner);

    assert_eq!(runner.deck_size(0), d0, "no mill when the opponent has more than 10");
    assert_eq!(runner.deck_size(1), d1);
    assert_eq!(offered_count(&runner), 3);
}

#[test]
fn ex10_049_wd_delete_pick_is_mandatory_and_skipped_when_nothing_qualifies() {
    let mut runner = base().memory(10).start();
    runner.place_on_field(1, "OPP-L6", Some(0));
    let d1 = runner.deck_size(1);

    digivolve_skullsatamon(&mut runner);

    assert_eq!(runner.deck_size(1), d1 - 3, "the mill still happens");
    assert!(
        runner.pending_selection().is_none(),
        "a level 6 is out of range: nothing to delete and no pick"
    );
    assert_eq!(opp_ids(&runner), vec!["OPP-L6".to_string()]);
}

#[test]
fn ex10_049_delete_pick_cannot_be_declined() {
    let mut runner = base().memory(10).start();
    runner.place_on_field(1, "OPP-L3", Some(0));
    digivolve_skullsatamon(&mut runner);
    assert!(!runner.pending_is_optional(), "canNoSelect: false");
}

// ─── [On Deletion] ────────────────────────────────────────────────────────────

#[test]
fn ex10_049_on_deletion_runs_the_same_body() {
    let mut runner = base().hand(0, &["DSL-DELETER"]).memory(5).start();
    // SkullSatamon first (so the deleter's pick offers it as the other Digimon).
    runner.place_on_field(0, CARD_ID, Some(0));
    with_opp_board(&mut runner);
    let d0 = runner.deck_size(0);
    let d1 = runner.deck_size(1);

    runner.play(0, 0).expect("play the deleter");
    // The deleter's own pick: SkullSatamon is the only "other" Digimon.
    let first = runner.pending_selection().expect("deleter pick").valid_action_ids[0];
    runner.execute_action(0, first).expect("delete SkullSatamon");

    assert_eq!(runner.deck_size(0), d0 - 3, "[On Deletion] mills 3 of your deck");
    assert_eq!(runner.deck_size(1), d1 - 3);
    assert_eq!(offered_count(&runner), 1, "level 3 or lower only (opp trash is 3 after the mill)");
}

// ─── Inherited [When Attacking] [Once Per Turn] ───────────────────────────────

fn carrier_runner() -> (DebugRunner, PermanentHandle) {
    let mut runner = base().add_card(digimon("CARRIER", 5, CardColor::Purple)).memory(5).start();
    let stack = runner.place_stack(0, &[CARD_ID, "CARRIER"]);
    (runner, stack)
}

fn fire_attack(runner: &mut DebugRunner, stack: PermanentHandle) {
    runner
        .game
        .enqueue_triggered(EffectTiming::WhenAttacking, TriggerSource::Permanent(stack));
    runner.game.drain_effect_queue();
    runner.auto_resolve().ok();
}

#[test]
fn ex10_049_inherited_attack_with_low_opponent_trash_mills_two_each_instead_of_security_attack() {
    let (mut runner, stack) = carrier_runner();
    let sa_before = runner.game.effective_security_strike(stack);
    let d0 = runner.deck_size(0);
    let d1 = runner.deck_size(1);

    fire_attack(&mut runner, stack);

    assert_eq!(runner.deck_size(0), d0 - 2);
    assert_eq!(runner.deck_size(1), d1 - 2);
    assert_eq!(
        runner.game.effective_security_strike(stack),
        sa_before,
        "\"instead\": the Security A. +1 is REPLACED by the mill"
    );
}

#[test]
fn ex10_049_inherited_attack_with_more_than_10_opponent_trash_gains_security_attack_plus_one() {
    let (mut runner, stack) = carrier_runner();
    seed_trash(&mut runner, 1, 11);
    let sa_before = runner.game.effective_security_strike(stack);
    let d0 = runner.deck_size(0);

    fire_attack(&mut runner, stack);

    assert_eq!(runner.deck_size(0), d0, "no mill when the opponent has more than 10");
    assert_eq!(runner.game.effective_security_strike(stack), sa_before + 1, "<Security A. +1> for the turn");
}

#[test]
fn ex10_049_inherited_attack_at_exactly_10_takes_the_instead_branch() {
    let (mut runner, stack) = carrier_runner();
    seed_trash(&mut runner, 1, 10);
    let sa_before = runner.game.effective_security_strike(stack);
    let d1 = runner.deck_size(1);

    fire_attack(&mut runner, stack);

    assert_eq!(runner.deck_size(1), d1 - 2, "10 is '10 or fewer': mill instead");
    assert_eq!(runner.game.effective_security_strike(stack), sa_before);
}
