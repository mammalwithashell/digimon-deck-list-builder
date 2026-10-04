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
    assert_eq!(card.attribute.as_deref(), Some("Virus"));
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
