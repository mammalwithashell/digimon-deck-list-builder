//! BT25-044 Junomon — Digimon, Lv.6, Yellow/Purple, DP 12000, Cost 12.
//! Traits: Shaman, Olympos XII, Iliad, TS. Attribute: Virus (official Bandai DB).
//!
//! # Card text (card image BT25-044 — authoritative for printed text)
//!
//! When this card would be played, if there are 6 or fewer total cards in both
//! players' security stacks, reduce the cost by 5.
//! [On Play] [When Digivolving] By placing 1 other Digimon as the top security
//! card, trash both players' top security cards.
//! [All Turns] [Once Per Turn] When your security stack is removed from, you may
//! play 1 play cost 8 or lower [Angel], [Archangel] or [Iliad] trait card from
//! your hand or trash without paying the cost.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT25/Yellow/BT25_044.cs
//!
//! # Patterns this test covers (RUST_DSL_TEST_API §4.3)
//! - D2 conditional cost reduction (security-sum gate)
//! - place-permanent-on-security as a cost + trash both top securities
//! - on_lose_security observer + C1 union-zone play-free (E2 OPT)

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_dsl::compiled::{
    CompiledClause, CompiledDeclarativeClause, CompiledScope, CompiledStep, CompiledTiming,
};
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardKind, PlayerId};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::SelectionKind;

const CARD_ID: &str = "BT25-044";

fn make_digimon(id: &str, level: u8, dp: i32, traits: &[&str]) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.level = Some(level);
    card.dp = Some(dp);
    card.traits = traits.iter().map(|t| t.to_string()).collect();
    card
}

fn base() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT25-044 YAML parses and compiles")
        .add_card(make_test_card("PAD", "Filler"))
        .add_card(make_digimon("OWN-OTHER", 4, 4000, &["Beast"]))
        .add_card(make_digimon("ANGEL-LOW", 4, 4000, &["Angel"]))
        .add_card(make_digimon("IRRELEVANT", 4, 9000, &["Beast"]))
        .deck(0, &["PAD"; 10])
        .deck(1, &["PAD"; 10])
}

// ─── Section 1 — structural ─────────────────────────────────────────────────

#[test]
fn bt25_044_metadata() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled present");
    assert_eq!(card.name, "Junomon");
    assert_eq!(card.level, Some(6));
    assert_eq!(card.cost, Some(12));
    assert_eq!(card.dp, Some(12000));
    for t in ["Shaman", "Olympos XII", "Iliad", "TS"] {
        assert!(card.traits.contains(&t.to_string()), "trait {t}");
    }
}

#[test]
fn bt25_044_has_cost_reduction() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled present");
    let has_cr = card.effects.iter().any(|c| {
        matches!(
            c,
            CompiledClause::Declarative(CompiledDeclarativeClause::CostReduction { .. })
        )
    });
    assert!(has_cr, "cost-reduction clause present");
}

#[test]
fn bt25_044_has_op_wd_place_security_clause() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled present");
    let clause = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t)
                if t.when.contains(&CompiledTiming::OnPlay)
                    && t.when.contains(&CompiledTiming::WhenDigivolving) =>
            {
                Some(t)
            }
            _ => None,
        })
        .expect("OP/WD place-security clause present");
    assert!(clause.optional, "printed 'By placing' -> optional");
}

#[test]
fn bt25_044_has_on_lose_security_play_free_clause() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled present");
    let clause = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::OnLoseSecurity) => {
                Some(t)
            }
            _ => None,
        })
        .expect("on_lose_security clause present");
    assert!(clause.once_per_turn, "[Once Per Turn]");
    assert!(clause.optional, "printed 'you may' -> optional");
}

// ─── Section 2 — behavior: place other Digimon as security + trash both ──────

#[test]
fn bt25_044_on_play_places_other_digimon_and_trashes_both_top_security() {
    let mut runner = base()
        .hand(0, &[CARD_ID])
        .security(0, &["PAD"; 2])
        .security(1, &["PAD"; 2])
        .memory(12)
        .start();
    // An "other" own Digimon to place into security.
    let other = runner.place_on_field(0, "OWN-OTHER", Some(0));

    let own_sec_before = runner.security_count(0);
    let opp_sec_before = runner.security_count(1);

    let _j = runner.play(0, 0).expect("play Junomon");

    // OnPlay installs the place-1-other selection (optional).
    let kind = runner
        .pending_kind()
        .expect("place-security selection installs");
    // Drive the placement of the other Digimon, then auto-resolve the trashes.
    let view = runner.pending_selection_view().unwrap();
    runner
        .execute_action(view.selecting_player, view.valid_action_ids[0])
        .expect("pick other Digimon to place");
    runner.auto_resolve().expect("resolve placement + trashes");

    // Placement (+1 own sec from the placed Digimon) then trash own top (-1) and
    // opp top (-1). Net own security == before; opp security == before - 1.
    assert_eq!(
        runner.security_count(1),
        opp_sec_before - 1,
        "opponent's top security card is trashed"
    );
    assert_eq!(
        runner.security_count(0),
        own_sec_before,
        "own: +1 placed Digimon, -1 trashed top = unchanged"
    );
}

#[test]
fn bt25_044_on_play_no_prompt_without_other_digimon() {
    let mut runner = base()
        .hand(0, &[CARD_ID])
        .security(0, &["PAD"; 2])
        .security(1, &["PAD"; 2])
        .memory(12)
        .start();

    // No other Digimon on field -> the clause condition fails, no prompt.
    let _j = runner.play(0, 0).expect("play Junomon");
    assert!(
        runner.pending_selection().is_none(),
        "no other Digimon -> place-security clause is a no-op"
    );
}

// ─── Section 3 — printed digivolve circles (G-DATA-BT25-044-COLOR-ATTRIBUTE) ─
//
// Official Bandai DB (data/card_bundles/BT25-044.md): Yellow/Purple, Virus,
// standard circles Yellow Lv.5 / cost 4 AND Purple Lv.5 / cost 4.

fn make_lv5_base(id: &str, color: digimon_engine::enums::CardColor) -> CardData {
    let mut c = make_digimon(id, 5, 7000, &["Beast"]);
    c.colors = vec![color];
    c
}

/// Digivolve Junomon from hand over `base_card`; returns (proceeded, memory delta).
fn try_digivolve_over(base_card: CardData) -> (bool, i16) {
    use digimon_engine::enums::{GamePhase, PlaySource};
    let base_id = base_card.card_id.clone();
    let mut r = base()
        .add_card(base_card)
        .hand(0, &[CARD_ID])
        .memory(10)
        .start();
    r.game.turn_count = 1;
    r.game.current_phase = GamePhase::Main;
    r.place_on_field(0, &base_id, Some(0));
    let mem_before = r.game.memory;
    let proceeded = r.game.digivolve_from_hand(0, 0, 0, PlaySource::ByHand);
    r.game.drain_effect_queue();
    (proceeded, r.game.memory - mem_before)
}

#[test]
fn bt25_044_digivolves_from_purple_lv5_for_4() {
    use digimon_engine::enums::CardColor;
    let (proceeded, delta) = try_digivolve_over(make_lv5_base("PURPLE-LV5", CardColor::Purple));
    assert!(proceeded, "printed Purple Lv.5 / cost 4 circle must allow the digivolve");
    assert_eq!(delta, -4, "Purple Lv.5 circle costs 4");
}

#[test]
fn bt25_044_digivolves_from_yellow_lv5_for_4() {
    use digimon_engine::enums::CardColor;
    let (proceeded, delta) = try_digivolve_over(make_lv5_base("YELLOW-LV5", CardColor::Yellow));
    assert!(proceeded, "printed Yellow Lv.5 / cost 4 circle");
    assert_eq!(delta, -4);
}

#[test]
fn bt25_044_no_white_lv5_circle() {
    use digimon_engine::enums::CardColor;
    let (proceeded, _) = try_digivolve_over(make_lv5_base("WHITE-LV5", CardColor::White));
    assert!(!proceeded, "Junomon prints no White circle");
}

#[test]
fn bt25_044_compiled_colors_are_yellow_purple_virus() {
    use digimon_dsl::compiled::CompiledColor;
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled present");
    assert_eq!(card.color, vec![CompiledColor::Yellow, CompiledColor::Purple]);
    assert_eq!(card.attribute, ["Virus"]);
}

// ─── Section 4 — DCGO-faithful OP/WD shape (BT25_044.cs "Shared OP / WD") ────
//
// DCGO `BT25_044.cs:48-57` gates on ANY other battle-area Digimon (either
// player's: `permanent != this && IsPermanentExistsOnBattleAreaDigimon`),
// `optional: false` (no OptionalSkill) with the "By placing" cost made
// declinable by the pick itself (`SelectPermanentEffect canNoSelect: true`,
// :64-75). The placed permanent goes to its OWNER's security
// (`CardObjectController.cs:1099-1120` AddSecurityCard into
// `cardSource.Owner.SecurityCards`), and the trash-both follow-up runs only
// if the placement succeeded (:83-101).

#[test]
fn bt25_044_on_play_offers_opponent_digimon_and_places_it_in_opponents_security() {
    let mut runner = base()
        .hand(0, &[CARD_ID])
        .security(0, &["PAD"; 2])
        .security(1, &["PAD"; 2])
        .memory(12)
        .start();
    // Only the OPPONENT has another Digimon.
    let _opp = runner.place_on_field(1, "OWN-OTHER", Some(0));

    let _j = runner.play(0, 0).expect("play Junomon");

    let view = runner
        .pending_selection_view()
        .expect("an opponent's Digimon is a valid 'other Digimon' -> the pick opens");
    assert_eq!(view.selecting_player, 0);
    assert!(
        matches!(view.kind, SelectionKind::AnyField),
        "the first prompt is the placement pick itself (no OptionalSkill gate), got {:?}",
        view.kind
    );
    assert!(view.is_optional, "the pick is declinable (canNoSelect: true)");
    assert_eq!(view.valid_action_ids.len(), 1, "only the opponent's Digimon");
    runner
        .execute_action(0, view.valid_action_ids[0])
        .expect("place the opponent's Digimon");
    runner.auto_resolve().expect("resolve placement + trashes");

    assert!(runner.game.players[1].battle_area.is_empty(), "opp Digimon left the field");
    // Opp: +1 placed into ITS OWNER's security, -1 trashed top = unchanged.
    assert_eq!(runner.security_count(1), 2, "placed into the owner's (opponent's) security, then top trashed");
    // Own: -1 trashed top.
    assert_eq!(runner.security_count(0), 1, "own top security trashed");
    // The opponent's trashed top security card is the placed Digimon itself.
    assert!(
        runner.game.players[1]
            .trash
            .iter()
            .any(|c| c.card_id(&runner.game.card_data) == "OWN-OTHER"),
        "the placed card was the opponent's top security and got trashed"
    );
}

#[test]
fn bt25_044_on_play_first_prompt_is_the_declinable_pick() {
    let mut runner = base()
        .hand(0, &[CARD_ID])
        .security(0, &["PAD"; 2])
        .security(1, &["PAD"; 2])
        .memory(12)
        .start();
    let _other = runner.place_on_field(0, "OWN-OTHER", Some(0));
    let _j = runner.play(0, 0).expect("play Junomon");

    let view = runner.pending_selection_view().expect("pick opens");
    assert!(
        matches!(view.kind, SelectionKind::AnyField),
        "no OptionalSkill gate before the pick (DCGO optional:false), got {:?}",
        view.kind
    );
    assert!(view.is_optional);
    // Decline: nothing placed, nothing trashed.
    runner
        .execute_action(0, digimon_engine::action::space::PASS)
        .expect("decline the pick");
    runner.auto_resolve().expect("settle");
    assert_eq!(runner.security_count(0), 2);
    assert_eq!(runner.security_count(1), 2);
    assert_eq!(runner.game.players[0].battle_area.len(), 2);
}

// ─── Section 5 — [All Turns][OPT] declined → use refunded (BT25_044.cs:208) ──
//
// DCGO `BT25_044.cs:208` `if (!executed) activateClass.RemoveUse()`: when the
// player plays nothing (chooses "Do not place" / picks nothing) the once-per-
// turn use is NOT consumed. Consistent with §15-14-1 (each ACTIVATION counts)
// -- declining a "you may" is not activating it -- and with the established
// DSL `refund_opt` convention (G-OPT-REFUND-ON-DECLINE).

const TRASHER_YAML: &str = r#"
card: TEST-SEC-TRASHER
name: Security Trasher
kind: digimon
level: 3
color: [red]
cost: 0
dp: 1000
traits: [Beast]
effects:
  - when: on_play
    summary: "[On Play] Trash your top security card"
    process:
      - trash_top_security: { of: you }
"#;

#[test]
fn bt25_044_all_turns_declined_play_does_not_consume_once_per_turn() {
    let mut runner = base()
        .from_dsl_yaml(TRASHER_YAML)
        .expect("trasher YAML")
        .hand(0, &["TEST-SEC-TRASHER", "TEST-SEC-TRASHER", "ANGEL-LOW"])
        .security(0, &["PAD"; 4])
        .memory(5)
        .start();
    let _j = runner.place_on_field(0, CARD_ID, Some(0));

    // First own-security removal -> Junomon's [All Turns] pick -> decline.
    runner.play(0, 0).expect("play trasher #1");
    let view = runner
        .pending_selection_view()
        .expect("Junomon [All Turns] play pick opens (ANGEL-LOW in hand)");
    assert!(view.is_optional);
    runner.execute_action(0, digimon_engine::action::space::PASS).expect("decline");
    runner.auto_resolve().expect("settle");
    assert_eq!(runner.security_count(0), 3);

    // Second own-security removal, same turn: the declined activation did
    // not consume the [Once Per Turn] use, so the pick is offered again.
    let idx = runner
        .game
        .player(0)
        .hand
        .iter()
        .position(|c| c.card_id(&runner.game.card_data) == "TEST-SEC-TRASHER")
        .unwrap();
    runner.play(0, idx).expect("play trasher #2");
    assert_eq!(runner.security_count(0), 2);
    let view = runner
        .pending_selection_view()
        .expect("OPT refunded on decline -> Junomon's pick is offered again");
    assert!(view.is_optional);
    runner.execute_action(0, view.valid_action_ids.iter().copied().find(|a| *a != digimon_engine::action::space::PASS).unwrap()).expect("play ANGEL-LOW");
    runner.auto_resolve().expect("settle");
    assert!(runner.game.player(0).battle_area.iter().any(|p| p.top_card().card_id(&runner.game.card_data) == "ANGEL-LOW"));
}

// ─── Section 6 — [All Turns] that cannot activate is not orderable ─────────
//
// DCGO `BT25_044.cs` [All Turns] `CanActivateCondition` needs a cost<=8
// [Angel]/[Archangel]/[Iliad] card in hand OR trash, and `MultipleSkills.cs`
// filters `skillInfos_active` by `CanActivate` (:216-220) -- with one
// activatable effect left it short-circuits (`Count == 1`, no prompt).

const LOSE_SEC_OBSERVER_YAML: &str = r#"
card: TEST-LOSE-SEC-OBS
name: Lose Security Observer
kind: digimon
level: 3
color: [red]
cost: 3
dp: 1000
traits: [Beast]
effects:
  - when: on_lose_security
    active_when: { all_turns: true }
    summary: "[All Turns] When your security stack is removed from, gain 2 memory"
    process:
      - gain_memory: 2
"#;

#[test]
fn bt25_044_all_turns_without_eligible_card_is_not_offered_in_trigger_order() {
    let mut runner = base()
        .from_dsl_yaml(TRASHER_YAML)
        .expect("trasher YAML")
        .from_dsl_yaml(LOSE_SEC_OBSERVER_YAML)
        .expect("observer YAML")
        // No Angel/Archangel/Iliad card in hand or trash.
        .hand(0, &["TEST-SEC-TRASHER", "IRRELEVANT"])
        .security(0, &["PAD"; 3])
        .memory(0)
        .start();
    let _j = runner.place_on_field(0, CARD_ID, Some(0));
    let _o = runner.place_on_field(0, "TEST-LOSE-SEC-OBS", Some(0));

    runner.play(0, 0).expect("play the trasher");
    assert!(
        runner.pending_selection().is_none(),
        "Junomon's [All Turns] cannot activate -> only the observer remains -> no TriggerOrder"
    );
    assert_eq!(runner.memory(), 2, "the observer resolved");
}

// ─── Section 4 — the cost reduction is scoped to Junomon's OWN play ─────────
//
// Printed: "When this card would be played, if there are 6 or fewer total
// cards in both players' security stacks, reduce the cost by 5." DCGO
// BT25_044.cs: `ChangeCostClass` whose card condition is `cardSource == card`
// with `(own + enemy security) <= 6` -- it never touches any other card.
//
// Regression (specialist-league game 245, 2026-11-08): the clause was lowered
// as an unscoped field reducer, so a Junomon on the board cut 5 off EVERY
// play/use by EITHER player while the stacks held <= 6 cards. The opposing
// TS Jupitermon seat then used Abyss Sanctuary: Throne Room (BT24-090, cost 3)
// for free, swapped it with the face-up Throne Room in security, and repeated
// that forever (1,000+ decisions at constant memory).

fn plain_tamer(id: &str, cost: u16) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Tamer;
    card.play_cost = cost;
    card
}

fn scoped_base() -> DebugRunnerBuilder {
    base().add_card(plain_tamer("TAMER-3", 3))
}

#[test]
fn bt25_044_on_opponent_field_does_not_reduce_my_play_cost() {
    // Both security stacks empty -> Junomon's printed condition holds.
    let mut runner = scoped_base().hand(0, &["TAMER-3"]).memory(5).start();
    runner.place_on_field(1, CARD_ID, Some(0));
    let before = runner.memory();
    runner.play(0, 0).expect("play the cost-3 Tamer");
    assert_eq!(
        runner.memory(),
        before - 3,
        "an opponent's Junomon must not discount my Tamer"
    );
}

#[test]
fn bt25_044_on_own_field_does_not_reduce_my_other_play_cost() {
    let mut runner = scoped_base().hand(0, &["TAMER-3"]).memory(5).start();
    runner.place_on_field(0, CARD_ID, Some(0));
    let before = runner.memory();
    runner.play(0, 0).expect("play the cost-3 Tamer");
    assert_eq!(
        runner.memory(),
        before - 3,
        "Junomon only reduces its own play cost"
    );
}

#[test]
fn bt25_044_play_from_hand_reduced_by_5_with_six_or_fewer_total_security() {
    let mut runner = base()
        .hand(0, &[CARD_ID])
        .security(0, &["PAD"; 3])
        .security(1, &["PAD"; 3])
        .memory(10)
        .start();
    let before = runner.memory();
    runner.play(0, 0).expect("play Junomon");
    assert_eq!(runner.memory(), before - 7, "printed 12 reduced by 5 (6 total security)");
}

#[test]
fn bt25_044_play_from_hand_full_cost_with_more_than_six_total_security() {
    let mut runner = base()
        .hand(0, &[CARD_ID])
        .security(0, &["PAD"; 4])
        .security(1, &["PAD"; 3])
        .memory(10)
        .start();
    let before = runner.memory();
    runner.play(0, 0).expect("play Junomon");
    assert_eq!(runner.memory(), before - 12, "7 total security -> no reduction");
}

/// The game-245 loop in miniature: next to an opposing Junomon, using Abyss
/// Sanctuary: Throne Room (BT24-090) must still cost its printed 3.
#[test]
fn bt25_044_opposing_junomon_does_not_make_throne_room_free() {
    let mut yellow = make_digimon("YELLOW-TS", 4, 5000, &["TS"]);
    yellow.colors = vec![digimon_engine::enums::CardColor::Yellow];
    let mut runner = base()
        .dsl_card("BT24-090")
        .expect("BT24-090 compiles")
        .add_card(yellow)
        .hand(0, &["BT24-090"])
        .memory(5)
        .start();
    runner.place_on_field(0, "YELLOW-TS", Some(0));
    runner.place_on_field(1, CARD_ID, Some(0));
    let before = runner.memory();
    let _ = runner.game.play_option_from_hand(0, 0);
    runner.auto_resolve().ok();
    assert_eq!(runner.memory(), before - 3, "Throne Room's use cost is 3");
}

// ─── Section 7 — [All Turns] pick offers only cards the play can take ───────
//
// DCGO BT25_044.cs:149-154 `CanSelectCardCondition`: `HasPlayCost &&
// GetCostItself <= 8 && (Angel || Archangel || Iliad) &&
// CanPlayAsNewPermanent(cardSource, false, activateClass, root)` -- used for
// both the hand (:157) and the trash (:159) candidates AND for the
// `CanActivateCondition` (:142-146), so without a playable candidate the
// effect does not activate. CR 4.3 7-1-1: playing places "a new card with a
// play cost" onto the field -- a Digi-Egg (hatched, CR 4.3 4-18) and an Option
// (used, with a use cost) have none, so neither is a "play" candidate even
// though the printed text says "card". CanPlayAsNewPermanent also needs an
// empty frame and no play prohibition; from the trash it honours "can't play
// from the trash" (CR 4.3 1-3-2: an impossible action is not carried out).

/// An [Angel] Digi-Egg: no play cost, never played.
fn angel_egg(id: &str) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::DigiEgg;
    card.level = Some(2);
    card.dp = None;
    card.play_cost = 0;
    card.traits = vec!["Angel".to_string()];
    card
}

fn pick_base() -> DebugRunnerBuilder {
    base()
        .from_dsl_yaml(TRASHER_YAML)
        .expect("trasher YAML")
        .dsl_card("BT24-090")
        .expect("BT24-090 compiles")
        .add_card(angel_egg("ANGEL-EGG"))
        .security(0, &["PAD"; 4])
        .memory(5)
}

/// Junomon on the field, `hand` (the trasher first) and `trash` staged, then
/// the trasher is played so the own-security removal fires Junomon's [All
/// Turns] clause. `setup` runs just before the trasher is played. Returns the
/// card ids the union pick offers (`None` when no prompt opened).
fn offered_after_security_loss(
    hand: &[&str],
    trash: &[&str],
    setup: impl FnOnce(&mut DebugRunner),
) -> Option<Vec<String>> {
    use digimon_engine::owned_card_choice::CardChoiceOperation;
    let mut runner = pick_base().hand(0, hand).start();
    for id in trash {
        runner.inject_trash(0, id);
    }
    let _j = runner.place_on_field(0, CARD_ID, Some(0));
    setup(&mut runner);
    runner.play(0, 0).expect("play the trasher");
    assert_eq!(runner.security_count(0), 3, "the trasher removed a security card");
    let view = runner.pending_selection_view()?;
    let rows = view.card_choice.expect("union pick is an owned card choice").rows;
    let mut ids: Vec<String> = rows
        .into_iter()
        .filter(|row| matches!(row.operation, CardChoiceOperation::CommitCard))
        .map(|row| row.card_id.expect("own hand/trash card identity visible"))
        .collect();
    ids.sort();
    Some(ids)
}

#[test]
fn bt25_044_all_turns_pick_excludes_angel_digi_egg() {
    assert_eq!(
        offered_after_security_loss(&["TEST-SEC-TRASHER", "ANGEL-EGG", "ANGEL-LOW"], &[], |_| {}),
        Some(vec!["ANGEL-LOW".to_string()]),
        "the [Angel] Digi-Egg has no play cost and is not offered"
    );
}

#[test]
fn bt25_044_all_turns_pick_excludes_iliad_option() {
    assert_eq!(
        offered_after_security_loss(&["TEST-SEC-TRASHER", "BT24-090", "ANGEL-LOW"], &["BT24-090"], |_| {}),
        Some(vec!["ANGEL-LOW".to_string()]),
        "Abyss Sanctuary: Throne Room (BT24-090) is an [Iliad] Option (use cost 3): it is used, never played"
    );
}

#[test]
fn bt25_044_all_turns_not_activated_with_only_unplayable_candidates() {
    assert_eq!(
        offered_after_security_loss(&["TEST-SEC-TRASHER", "ANGEL-EGG"], &["BT24-090"], |_| {}),
        None,
        "DCGO CanActivateCondition uses the same playability filter: no prompt"
    );
}

#[test]
fn bt25_044_all_turns_not_activated_when_battle_area_full() {
    assert_eq!(
        offered_after_security_loss(&["TEST-SEC-TRASHER", "ANGEL-LOW"], &["ANGEL-LOW"], |r| {
            // The trasher takes the last free slot.
            r.game.rules.field_slots = r.game.players[0].battle_area.len() as u8 + 1;
        }),
        None,
        "no empty frame: nothing can be played, so the effect does not activate"
    );
}

#[test]
fn bt25_044_all_turns_not_activated_under_effect_play_prohibition() {
    use digimon_engine::enums::{Expiry, ModifierType};
    use digimon_engine::modifiers::PlayerModifierEntry;
    assert_eq!(
        offered_after_security_loss(&["TEST-SEC-TRASHER", "ANGEL-LOW"], &["ANGEL-LOW"], |r| {
            r.game.modifiers.add_player_modifier(
                0,
                PlayerModifierEntry::simple(ModifierType::CannotPlayDigimonByEffect, 0, Expiry::Permanent, None, 1),
            );
        }),
        None,
        "\"you can't play Digimon by effects\": no candidate"
    );
}

#[test]
fn bt25_044_all_turns_pick_drops_trash_copy_when_trash_play_is_locked() {
    use digimon_engine::enums::{Expiry, ModifierType};
    use digimon_engine::modifiers::PlayerModifierEntry;
    let lock = |r: &mut DebugRunner| {
        r.game.modifiers.add_player_modifier(
            0,
            PlayerModifierEntry::simple(ModifierType::CannotPlayFromTrash, 0, Expiry::Permanent, None, 1),
        );
    };
    assert_eq!(
        offered_after_security_loss(&["TEST-SEC-TRASHER"], &["ANGEL-LOW"], lock),
        None,
        "the only candidate sits in a trash its owner can't play from"
    );
}
