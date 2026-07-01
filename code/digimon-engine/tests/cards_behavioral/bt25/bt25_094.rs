//! BT25-094 Cosmic Area — Option, Red/Blue, Cost 3.
//! Traits: Iliad, TS.
//!
//! # Card text (cards.json — verbatim)
//!
//! While you have no face-up security cards, you can ignore this card's color
//! requirements.
//! [Security] [Your Turn] All of your red or blue [TS] trait Digimon gain
//! ＜Alliance＞ (When this Digimon attacks, by suspending 1 of your other
//! Digimon, add the suspended Digimon's DP to this Digimon and it gains
//! ＜Security A. +1＞ for the attack.). While you have [Apollomon] or [Dianamon],
//! they also gain ＜Rush＞ (This Digimon can attack the turn it comes into play.)
//! [Main] Add your bottom security card to the hand and place this card face up
//! as the bottom security card. Then, you may play 1 red or blue [TS] trait
//! Digimon card from your hand with the cost reduced by 3.
//!
//! Inherited (Security) effect:
//! [Security] You may play 1 level 4 or lower red or blue [TS] trait Digimon
//! card from your hand or trash without paying the cost.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT25/Red/BT25_094.cs
//!
//! - Region "Ignore Color Requirement" (None): IgnoreColorConditionClass gated
//!   on owner having 0 non-flipped (face-up) security cards.
//! - Region "Alliance" (OnAllyAttack): AllianceStaticEffect over owner
//!   battle-area Digimon that are red OR blue AND HasTSTraits, gated on this
//!   card being IN SECURITY (face-down).
//! - Region "Rush" (None): RushStaticEffect over the same target set, gated on
//!   in-security AND owner has a permanent named "Apollomon" OR "Dianamon".
//! - Region "Main Effect" (OptionSkill): ReplaceBottomSecurityWithFaceUpOption-
//!   Effect, then SelectHandEffect (PlayForCost) over red/blue TS Digimon with
//!   cost reduced by 3, canNoSelect: true.
//! - Region "Security Effect" (SecuritySkill): SelectHand/SelectCard over level
//!   4- red/blue TS Digimon from hand OR trash, PlayForFree, canNoSelect: true.
//!
//! # Patterns this test covers (RUST_DSL_TEST_API §4.3)
//! - D3 color ignore / bypass (floodgate, face-down-security gate)
//! - D4/D5 security aura (Alliance keyword) with conditional keyword grant (Rush)
//! - C5/B2 option Main: replace-bottom-security-with-self + reduced play
//! - F9-adjacent security effect: play lvl4- TS from hand/trash free

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_dsl::compiled::{
    CompiledCardKind, CompiledClause, CompiledDeclarativeClause, CompiledScope, CompiledStep,
    CompiledTiming,
};
use digimon_engine::action::space::PASS;
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, CardKind, Keyword};
use digimon_engine::selection::{OptionPlayResult, SelectionKind};

const CARD_ID: &str = "BT25-094";

// ─── Fixture helpers ─────────────────────────────────────────────────────────

fn area_runner() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT25-094 YAML loads")
}

fn ts_digimon(id: &str, color: CardColor, level: u8) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.colors = vec![color];
    card.traits = vec!["TS".to_string()];
    card.level = Some(level);
    card.dp = Some(4000);
    card.play_cost = u16::from(level);
    card
}

fn named_digimon(id: &str, name: &str, color: CardColor) -> CardData {
    let mut card = ts_digimon(id, color, 6);
    card.card_name = name.to_string();
    card
}

fn attacker(id: &str) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.colors = vec![CardColor::Red];
    card.level = Some(4);
    card.dp = Some(9000);
    card.play_cost = 4;
    card
}

fn filler(id: &str) -> CardData {
    make_test_card(id, id)
}

fn hand_action_for_id(runner: &DebugRunner, id: &str) -> u16 {
    runner
        .game
        .player(0)
        .hand
        .iter()
        .enumerate()
        .find_map(|(idx, card)| {
            (card.card_id(&runner.game.card_data) == id)
                .then_some(digimon_engine::action::space::PLAY_HAND_START + idx as u16)
        })
        .expect("hand card exists")
}

// ─── Section 1 — Structural assertions ───────────────────────────────────────

#[test]
fn bt25_094_metadata_floodgate_auras_main_and_security_compile() {
    let runner = area_runner().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled BT25-094");

    assert_eq!(card.name, "Cosmic Area");
    assert_eq!(card.kind, CompiledCardKind::Option);
    assert_eq!(card.cost, Some(3));

    // Color-ignore floodgate (gated on no face-up security).
    assert!(
        card.effects.iter().any(|clause| matches!(
            clause,
            CompiledClause::Declarative(CompiledDeclarativeClause::FloodGate { modifier, .. })
                if modifier == "IgnoreColorRequirement"
        )),
        "color-ignore floodgate must compile"
    );

    // Security keyword auras: two grant_keyword auras (Alliance + conditional Rush).
    let kw_auras = card
        .effects
        .iter()
        .filter(|clause| {
            matches!(
                clause,
                CompiledClause::Declarative(CompiledDeclarativeClause::Aura {
                    scope: CompiledScope::Security,
                    grant_keyword: Some(_),
                    ..
                })
            )
        })
        .count();
    assert_eq!(
        kw_auras, 2,
        "two security keyword auras (Alliance + conditional Rush) must compile"
    );

    // Main clause: replace-bottom-security-with-self + reduced play.
    let main = card
        .effects
        .iter()
        .find_map(|clause| match clause {
            CompiledClause::Triggered(t) if t.when == vec![CompiledTiming::MainFromHand] => Some(t),
            _ => None,
        })
        .expect("MainFromHand clause");
    assert!(
        main.process
            .iter()
            .any(|s| matches!(s, CompiledStep::AddBottomSecurityToHand { .. })),
        "main must add bottom security to hand"
    );
    assert!(
        main.process
            .iter()
            .any(|s| matches!(s, CompiledStep::PlaceSelfOptionAtSecurity { .. })),
        "main must place self face-up at bottom security"
    );
    assert!(
        main.process
            .iter()
            .any(|s| matches!(s, CompiledStep::PlayFromHand { .. })),
        "main must play a reduced-cost TS Digimon from hand"
    );

    // Inherited [Security] clause.
    assert!(
        card.effects.iter().any(|clause| matches!(
            clause,
            CompiledClause::Triggered(t)
                if t.scope == CompiledScope::Inherited
                    && t.when == vec![CompiledTiming::OnSecurity]
        )),
        "inherited [Security] play clause must compile"
    );
}

// ─── Section 2 — Behavior: Main replace-bottom-security + reduced play ────────

#[test]
fn bt25_094_main_replaces_bottom_security_with_self_and_plays_reduced_ts() {
    let mut runner = area_runner()
        .add_card(filler("BOTTOM"))
        .add_card(filler("TOP"))
        .add_card(ts_digimon("RED-TS", CardColor::Red, 6))
        .add_card(ts_digimon("BLUE-TS", CardColor::Blue, 4))
        .add_card(ts_digimon("GREEN-TS", CardColor::Green, 4))
        .hand(0, &[CARD_ID, "RED-TS", "BLUE-TS", "GREEN-TS"])
        .security(0, &["BOTTOM", "TOP"])
        .memory(10)
        .start();
    runner.game.enter_main_phase();
    let memory_before = runner.memory();

    assert_eq!(
        runner.game.play_option_from_hand(0, 0),
        OptionPlayResult::Pending,
        "no face-up security should satisfy the option's color bypass"
    );

    let hand_prompt = runner
        .pending_selection_view()
        .expect("reduced play prompt");
    assert_eq!(hand_prompt.kind, SelectionKind::Hand);
    assert!(hand_prompt.is_optional, "the 'you may play' is optional");

    // Self placed face-up as the bottom security card.
    assert_eq!(
        runner.game.players[0].security[0].card_id(&runner.game.card_data),
        CARD_ID,
        "Cosmic Area should be placed as the bottom security card"
    );
    assert!(
        runner.game.players[0]
            .face_up_security
            .contains(&runner.game.players[0].security[0].card_index),
        "placed security card must be face-up"
    );
    // The former bottom security card moved to hand.
    assert!(runner.game.players[0]
        .hand
        .iter()
        .any(|card| card.card_id(&runner.game.card_data) == "BOTTOM"));

    // Red/blue TS are eligible; green TS is not.
    let red_action = hand_action_for_id(&runner, "RED-TS");
    assert!(hand_prompt.valid_action_ids.contains(&red_action));
    assert!(
        !hand_prompt
            .valid_action_ids
            .contains(&hand_action_for_id(&runner, "GREEN-TS")),
        "green TS is not a red/blue TS target"
    );

    runner
        .execute_action(hand_prompt.selecting_player, red_action)
        .expect("play red TS with reduced cost");

    assert!(runner.game.players[0]
        .battle_area
        .iter()
        .any(|perm| perm.top_card().card_id(&runner.game.card_data) == "RED-TS"));
    assert_eq!(
        runner.memory(),
        memory_before - 6,
        "option cost 3 + RED-TS play cost 6 reduced by 3 = 6 memory total"
    );
}

// ─── Section 3 — Behavior: security auras ─────────────────────────────────────

/// Alliance is granted to red/blue TS Digimon while Cosmic Area is face-up in
/// security; Rush is granted only while an [Apollomon] or [Dianamon] is present.
#[test]
fn bt25_094_security_aura_grants_alliance_and_conditional_rush() {
    let mut runner = area_runner()
        .add_card(ts_digimon("RED-TS", CardColor::Red, 4))
        .add_card(ts_digimon("BLUE-TS", CardColor::Blue, 4))
        .add_card(ts_digimon("GREEN-TS", CardColor::Green, 4))
        .add_card(named_digimon("APOLLO", "Apollomon", CardColor::Red))
        .security(0, &[CARD_ID])
        .start();

    // Cosmic Area sits FACE-UP in P0's security (its [Main] effect would place
    // it there; security-scope auras materialize only from face-up security
    // sources — see BT25-095 / BT24-090 idiom).
    let src_index = runner.game.players[0].security[0].card_index;
    runner.game.players[0].face_up_security.insert(src_index);

    let red = runner.place_on_field(0, "RED-TS", Some(0));
    let blue = runner.place_on_field(0, "BLUE-TS", Some(0));
    let green = runner.place_on_field(0, "GREEN-TS", Some(0));
    // Apollomon must be ON the field for the conditional Rush gate to open.
    runner.place_on_field(0, "APOLLO", Some(0));

    runner.game.tick_declarative_effects();

    // Alliance on red/blue TS, not on green.
    assert!(
        runner.game.has_keyword(red, Keyword::Alliance),
        "red TS must gain Alliance"
    );
    assert!(
        runner.game.has_keyword(blue, Keyword::Alliance),
        "blue TS must gain Alliance"
    );
    assert!(
        !runner.game.has_keyword(green, Keyword::Alliance),
        "green TS gets no aura (not red/blue)"
    );

    // Rush granted because Apollomon is present.
    assert!(
        runner.game.has_keyword(red, Keyword::Rush),
        "red TS must gain Rush while Apollomon is present"
    );
    assert!(!runner.game.has_keyword(green, Keyword::Rush));
}

/// Negative: without Apollomon or Dianamon, red/blue TS Digimon gain Alliance
/// but NOT Rush.
#[test]
fn bt25_094_security_aura_no_rush_without_apollomon_or_dianamon() {
    let mut runner = area_runner()
        .add_card(ts_digimon("RED-TS", CardColor::Red, 4))
        .security(0, &[CARD_ID])
        .start();

    let src_index = runner.game.players[0].security[0].card_index;
    runner.game.players[0].face_up_security.insert(src_index);

    let red = runner.place_on_field(0, "RED-TS", Some(0));
    runner.game.tick_declarative_effects();

    assert!(
        runner.game.has_keyword(red, Keyword::Alliance),
        "Alliance still applies without Apollomon/Dianamon"
    );
    assert!(
        !runner.game.has_keyword(red, Keyword::Rush),
        "no Apollomon/Dianamon → no Rush grant"
    );
}

// ─── Section 4 — Behavior: inherited [Security] effect ───────────────────────

#[test]
fn bt25_094_security_effect_plays_level_four_red_or_blue_ts_from_hand_or_trash_free() {
    let mut runner = area_runner()
        .add_card(ts_digimon("HAND-TS", CardColor::Blue, 4))
        .add_card(ts_digimon("TRASH-TS", CardColor::Red, 4))
        .add_card(ts_digimon("HIGH-TS", CardColor::Red, 5))
        .add_card(attacker("ATTACKER"))
        .add_card(filler("FILL"))
        .hand(1, &["HAND-TS", "HIGH-TS"])
        .deck(1, &["TRASH-TS"])
        .security(1, &[CARD_ID])
        .memory(10)
        .start();
    let trash_card = runner.game.players[1].deck.pop().expect("trash seed");
    runner.game.players[1].trash.push(trash_card);
    let attacker = runner.place_on_field(0, "ATTACKER", Some(0));
    let memory_before = runner.memory();

    let _ = runner.attack_player(attacker, 1, false);
    let union = runner
        .pending_selection_view()
        .expect("hand/trash union selection");
    assert!(union.is_optional, "the security play is a 'you may'");
    let chosen = union
        .valid_action_ids
        .iter()
        .copied()
        .find(|action| *action != PASS)
        .expect("eligible hand or trash card");
    runner
        .execute_action(union.selecting_player, chosen)
        .expect("play eligible TS card");
    runner.auto_resolve().expect("settle security effect");

    assert!(runner.game.players[1].battle_area.iter().any(|perm| {
        let id = perm.top_card().card_id(&runner.game.card_data);
        id == "HAND-TS" || id == "TRASH-TS"
    }));
    assert_eq!(runner.memory(), memory_before, "security play is free");
    assert!(
        !runner.game.players[1]
            .battle_area
            .iter()
            .any(|perm| perm.top_card().card_id(&runner.game.card_data) == "HIGH-TS"),
        "level 5 TS is not eligible (level 4 or lower only)"
    );
}
