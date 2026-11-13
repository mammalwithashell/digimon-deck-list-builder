//! The optional-ACTIVATION mark (`Game::pending_is_optional_activation`):
//! an optional effect with no separate accept/decline prompt folds its "you
//! may" into its FIRST decision, so that decision's PASS declines the whole
//! effect. The DCGO replay answers DCGO's `OptionalSkill.SetUseOptional`
//! ("use this optional effect?", DCGO `Script/OptionalSkill.cs`, asked by
//! `ICardEffect.Activate_Optional_Effect_Execute` for every `IsOptional`
//! effect before its body runs) against exactly that decision.
//!
//! Contract pinned here:
//! - the mark names the effect's FIRST decision only: when that decision is
//!   skipped (no candidates), no later pick of the same effect inherits it;
//! - the mark names one prompt (player, kind, prompt text, source card,
//!   source permanent): a different prompt never matches it, and a prompt
//!   that left `pending_selection` without resolving is not marked again
//!   after a later decision boundary;
//! - keyword effects DCGO implements as an `IsOptional` `ActivateClass`
//!   whose engine body folds the "may" into its one decision are marked by
//!   the same contract (<Execute>, <Engage>, <Vortex>, <Overclock>, <Blitz>,
//!   <Save>); keywords DCGO activates as non-optional are not (<Alliance>).
//!
//! Rules: Comprehensive Rules Ver. 4.3 §16-38-3 (<Execute> optional),
//! §16-44-3 (<Engage>), §16-33-3 (<Vortex>), §16-34-3 (<Overclock>: the
//! deletion is an optional processing condition), §16-16-3 (<Blitz> "may
//! attack"), §16-20-3 (<Save> optional), §16-24-3 (<Alliance>: the trigger
//! is mandatory, only its suspension condition is optional).

use digimon_engine::action::space::{encode_attack, encode_digivolve, SECURITY_TARGET};
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardKind, EffectTiming, Expiry, Keyword};
use digimon_engine::replacement::ReplacementCause;
use digimon_engine::selection::SelectionKind;

// ─── DSL folded "you may" (first pick is the decline) ────────────────────────

/// `[On Play] You may delete 1 of your opponent's Digimon with 1000 DP or
/// less. Then, you may delete 1 of your opponent's Digimon.` One optional
/// clause; its first step is an optional pick, so the clause has no outer
/// accept/decline (`body_first_step_is_declinable`) and that pick's PASS is
/// the "may".
const MAYTWO_YAML: &str = r#"
card: MAYTWO
name: Maytwomon
kind: digimon
level: 4
color: [red]
cost: 3
dp: 4000
effects:
  - when: on_play
    optional: true
    summary: "[On Play] You may delete 1 of your opponent's Digimon with 1000 DP or less. Then, you may delete 1 of your opponent's Digimon."
    process:
      - select_opponent_permanent:
          bind_as: first_pick
          optional: true
          filter:
            all_of:
              - kind: digimon
              - dp_lte: 1000
          prompt: "first pick: delete 1 of your opponent's Digimon with 1000 DP or less"
      - delete_permanent: { target: first_pick }
      - select_opponent_permanent:
          bind_as: second_pick
          optional: true
          filter: { kind: digimon }
          prompt: "second pick: delete 1 of your opponent's Digimon"
      - delete_permanent: { target: second_pick }
"#;

fn maytwo_board() -> DebugRunner {
    let mut small = make_test_card("SMALL", "Smallmon");
    small.dp = Some(1000);
    let mut big = make_test_card("BIG", "Bigmon");
    big.dp = Some(6000);
    DebugRunner::builder()
        .from_dsl_yaml(MAYTWO_YAML)
        .expect("MAYTWO loads")
        .add_card(small)
        .add_card(big)
        .hand(0, &["MAYTWO"])
        .deck(0, &["SMALL"; 6])
        .deck(1, &["SMALL"; 6])
        .security(0, &["SMALL"; 3])
        .security(1, &["SMALL"; 3])
        .memory(5)
        .start()
}

/// Regression guard: the first pick of a folded "may" clause is its
/// activation decision.
#[test]
fn folded_first_pick_is_the_activation_decision() {
    let mut r = maytwo_board();
    r.place_on_field(1, "SMALL", Some(0));
    r.place_on_field(1, "BIG", Some(0));
    r.play(0, 0).expect("MAYTWO is played");
    let sel = r.pending_selection().expect("the first pick parks");
    assert!(sel.prompt.starts_with("first pick"), "{sel:?}");
    assert!(sel.is_optional);
    assert!(r.game.pending_is_optional_activation(), "the first pick IS the may");

    // Its decision resolves; the second pick is a choice inside the body.
    r.decline_optional_trigger().expect("decline the first pick");
}

/// The first pick has no candidate (the opponent's only Digimon has 6000
/// DP), so it is skipped and the body parks on its SECOND optional pick from
/// the same card. That pick is not the effect's first decision and must not
/// be read as its activation gate.
#[test]
fn skipped_first_pick_does_not_lend_the_mark_to_a_later_pick() {
    let mut r = maytwo_board();
    r.place_on_field(1, "BIG", Some(0));
    r.play(0, 0).expect("MAYTWO is played");
    let sel = r.pending_selection().expect("the second pick parks");
    assert!(sel.prompt.starts_with("second pick"), "{sel:?}");
    assert!(sel.is_optional);
    assert!(
        !r.game.pending_is_optional_activation(),
        "the first decision was skipped: no later pick inherits the mark"
    );
}

/// As above through an `optional:` substep: its leading hand pick has no
/// candidate, so the whole substep is skipped (control_flow
/// G-SELECT-EMPTY-OUTER-TAIL) and the body parks on a later pick. That pick
/// is not the effect's first decision.
const MAYWRAP_YAML: &str = r#"
card: MAYWRAP
name: Maywrapmon
kind: digimon
level: 4
color: [red]
cost: 3
dp: 4000
effects:
  - when: on_play
    optional: true
    summary: "[On Play] You may trash 1 Option card in your hand. Then, you may delete 1 of your opponent's Digimon."
    process:
      - optional:
          - select_hand:
              of: you
              bind_as: discard
              optional: true
              filter: { kind: option }
              prompt: "first pick: trash 1 Option card in your hand"
          - trash_from_hand_by_index: { of: you, hand_index: discard }
      - select_opponent_permanent:
          bind_as: later_pick
          optional: true
          filter: { kind: digimon }
          prompt: "later pick: delete 1 of your opponent's Digimon"
      - delete_permanent: { target: later_pick }
"#;

#[test]
fn skipped_optional_substep_does_not_lend_the_mark_to_a_later_pick() {
    let mut big = make_test_card("BIG", "Bigmon");
    big.dp = Some(6000);
    let mut r = DebugRunner::builder()
        .from_dsl_yaml(MAYWRAP_YAML)
        .expect("MAYWRAP loads")
        .add_card(big)
        .hand(0, &["MAYWRAP"])
        .deck(0, &["BIG"; 6])
        .deck(1, &["BIG"; 6])
        .security(0, &["BIG"; 3])
        .security(1, &["BIG"; 3])
        .memory(5)
        .start();
    r.place_on_field(1, "BIG", Some(0));
    r.play(0, 0).expect("MAYWRAP is played");
    let sel = r.pending_selection().expect("the later pick parks");
    assert!(sel.prompt.starts_with("later pick"), "{sel:?}");
    assert!(
        !r.game.pending_is_optional_activation(),
        "the skipped substep was the first decision: no later pick inherits the mark"
    );
}

/// A prompt with the marked prompt's player, kind and text but a different
/// source card is a different prompt: it never matches the mark.
#[test]
fn a_different_prompt_with_the_same_text_is_not_the_marked_one() {
    let mut r = maytwo_board();
    r.place_on_field(1, "SMALL", Some(0));
    let big = r.place_on_field(1, "BIG", Some(0));
    r.play(0, 0).expect("MAYTWO is played");
    assert!(r.game.pending_is_optional_activation(), "witness: marked");
    let other_card = r.game.players[1].battle_area[big.index as usize]
        .top_card()
        .handle();
    r.game.pending_selection.as_mut().unwrap().source_card = other_card;
    assert!(
        !r.game.pending_is_optional_activation(),
        "same (player, kind, prompt), another source card: not the marked prompt"
    );
}

/// The marked prompt leaves `pending_selection` without resolving, a
/// decision boundary passes, and an identical prompt is parked again: the
/// stale mark must not match it.
#[test]
fn a_mark_does_not_survive_its_prompt_leaving_across_a_decision() {
    let mut r = maytwo_board();
    r.place_on_field(1, "SMALL", Some(0));
    r.place_on_field(1, "BIG", Some(0));
    r.play(0, 0).expect("MAYTWO is played");
    assert!(r.game.pending_is_optional_activation(), "witness: marked");
    let parked = r.game.pending_selection.take();
    let resume = r.game.pending_selection_resume.take();
    // A player decision boundary with nothing pending (no breeding Digimon:
    // the move is a no-op).
    assert!(!r.move_from_breeding(0));
    r.game.pending_selection = parked;
    r.game.pending_selection_resume = resume;
    assert!(
        !r.game.pending_is_optional_activation(),
        "the mark was for a prompt that had left; it is not re-attached"
    );
}

// ─── Keyword effects DCGO asks an OptionalSkill for ──────────────────────────

fn keyword_card(id: &str, keywords: Vec<Keyword>) -> CardData {
    let mut c = make_test_card(id, id);
    c.level = Some(4);
    c.dp = Some(5000);
    c.play_cost = 5;
    c.keywords = keywords;
    c
}

fn keyword_board(cards: Vec<CardData>, memory: i16) -> DebugRunner {
    let mut fill = make_test_card("FILL", "FILL");
    fill.dp = Some(1000);
    let mut tamer = make_test_card("TAMER", "TAMER");
    tamer.card_kind = CardKind::Tamer;
    tamer.level = None;
    tamer.dp = None;
    let mut b = DebugRunner::builder().add_card(fill).add_card(tamer);
    for c in cards {
        b = b.add_card(c);
    }
    b.deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .security(0, &["FILL"; 3])
        .security(1, &["FILL"; 3])
        .memory(memory)
        .start()
}

/// DCGO `CardEffectFactory/KeyWordEffects/Execute.cs:36` (`isOptional: true`)
/// then `CardEffectCommons/KeyWordEffects/Execute.cs:60-68`
/// (`SelectAttackEffect`): an OptionalSkill, then the attack. The engine folds
/// both into one attack-declaration decision (DCGO PvP 2026-11-13 g4 row 39).
#[test]
fn execute_attack_offer_is_the_activation_decision() {
    let mut r = keyword_board(vec![keyword_card("EXEC", vec![Keyword::Execute])], 3);
    let exec = r.place_on_field(0, "EXEC", Some(0));
    r.end_turn();
    let sel = r.pending_selection().expect("<Execute> offers its attack");
    assert_eq!(sel.kind, SelectionKind::Target, "{sel:?}");
    assert!(sel.is_optional);
    assert!(sel
        .valid_action_ids
        .contains(&encode_attack(exec.index as u16, SECURITY_TARGET)));
    assert!(r.game.pending_is_optional_activation());
    // The mark names the queued effect it belongs to.
    let exec_card = r.game.players[0].battle_area[exec.index as usize].top_card().handle();
    let effect = r
        .game
        .pending_optional_activation_effect()
        .expect("the marked effect");
    assert_eq!(effect.controller, 0);
    assert_eq!(effect.source_card, exec_card);
    assert_eq!(effect.timing, EffectTiming::EndOfYourTurn);
}

/// DCGO `KeyWordEffects/Engage.cs:37` (`isOptional: true`), then
/// `CardEffectCommons/KeyWordEffects/Engage.cs:20` (`SelectAttackEffect`).
#[test]
fn engage_attack_offer_is_the_activation_decision() {
    let mut r = keyword_board(vec![keyword_card("ENG", vec![Keyword::Engage])], 3);
    r.place_on_field(0, "ENG", Some(0));
    r.end_turn();
    let sel = r.pending_selection().expect("<Engage> offers its attack");
    assert!(sel.is_optional, "{sel:?}");
    assert!(r.game.pending_is_optional_activation());
}

/// DCGO `KeyWordEffects/Vortex.cs:36` (`isOptional: true`), then
/// `CardEffectCommons/KeyWordEffects/Vortex.cs:64` (`SelectAttackEffect`).
#[test]
fn vortex_attack_offer_is_the_activation_decision() {
    let mut r = keyword_board(vec![keyword_card("VTX", vec![Keyword::Vortex])], 3);
    r.place_on_field(0, "VTX", Some(0));
    let target = r.place_on_field(1, "FILL", Some(0));
    r.game.players[1].battle_area[target.index as usize].is_suspended = true;
    r.end_turn();
    let sel = r.pending_selection().expect("<Vortex> offers its attack");
    assert!(sel.is_optional, "{sel:?}");
    assert!(r.game.pending_is_optional_activation());
}

/// DCGO `KeyWordEffects/Overclock.cs:37` (`isOptional: true`), then
/// `CardEffectCommons/KeyWordEffects/Overclock.cs` `OverclockProcess`
/// (`SelectPermanentEffect`, `canNoSelect: true`): the deletion pick.
#[test]
fn overclock_deletion_pick_is_the_activation_decision() {
    let mut r = keyword_board(vec![keyword_card("OC", vec![])], 3);
    let oc = r.place_on_field(0, "OC", Some(0));
    r.place_on_field(0, "FILL", Some(0));
    r.game
        .modifiers
        .grant_keyword(oc, Keyword::Overclock, Expiry::EndOfTurn, 0);
    r.end_turn();
    let sel = r.pending_selection().expect("<Overclock> offers its deletion");
    assert_eq!(sel.kind, SelectionKind::OwnField, "{sel:?}");
    assert!(sel.is_optional);
    assert!(r.game.pending_is_optional_activation());
}

/// <Engage> attacks first; <Overclock>'s entry activates while that attack
/// waits for its counter timing, so its deletion pick is offered only after
/// the attack has ended (`Game::deferred_end_of_turn_attack_offers`). That
/// deferred pick is still <Overclock>'s first decision.
#[test]
fn deferred_overclock_pick_is_still_the_activation_decision() {
    let mut r = keyword_board(
        vec![
            keyword_card("ENG", vec![Keyword::Engage]),
            keyword_card("OC", vec![]),
        ],
        3,
    );
    let engage = r.place_on_field(0, "ENG", Some(0));
    let oc = r.place_on_field(0, "OC", Some(0));
    r.place_on_field(0, "FILL", Some(0));
    r.game
        .modifiers
        .grant_keyword(oc, Keyword::Overclock, Expiry::EndOfTurn, 0);
    r.end_turn();
    // TriggerOrder: activate <Engage> first.
    let (player, action) = {
        let sel = r.pending_selection().expect("a TriggerOrder prompt");
        assert_eq!(sel.kind, SelectionKind::TriggerOrder, "{sel:?}");
        let choice = sel
            .effect_choices
            .as_ref()
            .unwrap()
            .iter()
            .find(|c| c.label.starts_with("ENG"))
            .expect("an <Engage> branch");
        (sel.selecting_player, choice.action_id)
    };
    r.execute_action(player, action).expect("activate <Engage>");
    assert!(r.game.pending_is_optional_activation(), "<Engage>'s offer");
    r.execute_action(0, encode_attack(engage.index as u16, SECURITY_TARGET))
        .expect("declare the <Engage> attack");
    assert!(r.game.pending_attack.is_none(), "the <Engage> attack ended");
    let sel = r.pending_selection().expect("then <Overclock>'s deletion");
    assert_eq!(sel.kind, SelectionKind::OwnField, "{sel:?}");
    assert!(r.game.pending_is_optional_activation());
}

/// DCGO `KeyWordEffects/Blitz.cs:53` (`isOptional: true`), then
/// `CardEffectCommons/KeyWordEffects/Blitz.cs:35-45` (`SelectAttackEffect`).
#[test]
fn printed_blitz_attack_offer_is_the_activation_decision() {
    let mut blitz = keyword_card("PBLITZ", vec![Keyword::Blitz]);
    blitz.evo_costs = vec![EvoCost {
        card_color: 0,
        level: 3,
        memory_cost: 3,
    }];
    let mut base = make_test_card("PBASE", "PBASE");
    base.level = Some(3);
    let mut r = DebugRunner::builder()
        .add_card(base)
        .add_card(blitz)
        .hand(0, &["PBLITZ"])
        .deck(0, &["PBASE"; 6])
        .deck(1, &["PBASE"; 6])
        .security(0, &["PBASE"; 3])
        .security(1, &["PBASE"; 3])
        .memory(2)
        .start();
    let base = r.place_on_field(0, "PBASE", Some(0));
    r.game
        .decode_action(encode_digivolve(0, base.index as u16), 0);
    let sel = r.pending_selection().expect("<Blitz> offers its attack");
    assert!(sel.is_optional, "{sel:?}");
    assert!(r.game.pending_is_optional_activation());
}

/// DCGO `KeyWordEffects/Save.cs:14` (`isOptional: true`), then
/// `CardEffectCommons/KeyWordEffects/Save.cs:31-42` (`SelectPermanentEffect`,
/// `canNoSelect: true`): the Tamer pick.
#[test]
fn save_tamer_pick_is_the_activation_decision() {
    let mut r = keyword_board(vec![keyword_card("SAVE", vec![Keyword::Save])], 3);
    let save = r.place_on_field(0, "SAVE", Some(0));
    r.place_on_field(0, "TAMER", Some(0));
    r.game
        .delete_permanents_batch(vec![save], ReplacementCause::OpponentEffect);
    let sel = r.pending_selection().expect("<Save> offers its Tamer pick");
    assert!(sel.is_optional, "{sel:?}");
    assert!(r.game.pending_is_optional_activation());
}

/// Regression guard: <Raid> is an optional effect in the engine already
/// (DCGO `KeyWordEffects/Raid.cs:40`, `isOptional: true`).
#[test]
fn raid_switch_pick_is_the_activation_decision() {
    let mut r = keyword_board(vec![keyword_card("RAID", vec![Keyword::Raid])], 3);
    let raider = r.place_on_field(0, "RAID", Some(0));
    r.place_on_field(1, "FILL", Some(0));
    r.attack_player(raider, 1, false);
    let sel = r.pending_selection().expect("<Raid> offers its switch");
    assert!(sel.is_optional, "{sel:?}");
    assert!(r.game.pending_is_optional_activation());
}

/// Regression guard: DCGO activates <Alliance> as a NON-optional effect
/// (`KeyWordEffects/Alliance.cs:35`, `isOptional: false`); its ally pick is a
/// choice inside the body, never an OptionalSkill gate.
#[test]
fn alliance_ally_pick_is_not_an_activation_decision() {
    let mut r = keyword_board(vec![keyword_card("ALLY", vec![Keyword::Alliance])], 3);
    let attacker = r.place_on_field(0, "ALLY", Some(0));
    r.place_on_field(0, "FILL", Some(0));
    r.attack_player(attacker, 1, false);
    let sel = r.pending_selection().expect("<Alliance> offers its ally pick");
    assert!(sel.is_optional, "{sel:?}");
    assert!(!r.game.pending_is_optional_activation());
}
