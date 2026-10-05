//! BT25-097 Guardian Palace — Option, Yellow/Purple, Cost 3.
//! Traits: Iliad, TS.
//!
//! # Card text (official Bandai DB — data/card_bundles/BT25-097.md)
//!
//! While you have no face-up security cards, you can ignore this card's color
//! requirements.
//! {Security} [All Turns] All of your yellow or purple [TS] trait Digimon gain
//! ＜Alliance＞. While you have a Digimon with [Junomon] in its name, they also
//! gain ＜Scapegoat＞.
//! [Main] Add your bottom security card to the hand and place this card face up
//! as the bottom security card. Then, you may play 1 yellow or purple [TS] trait
//! Digimon card from your hand with the cost reduced by 3.
//!
//! Security effect:
//! [Security] You may play 1 level 4 or lower yellow or purple [TS] trait
//! Digimon card from your hand or trash without paying the cost.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT25/Yellow/BT25_097.cs
//!
//! - AllianceStaticEffect over owner Digimon (top card yellow OR purple, TS),
//!   gated on this card face-up in security.
//! - AddSkillClass (WhenPermanentWouldBeDeleted) granting ScapegoatSelfEffect
//!   to the same set, additionally gated on an own permanent whose top card
//!   contains "Junomon" in its name.
//! - Main / Security: the shared Area idiom (BT25-099 sibling).
//!
//! # Patterns this test covers (RUST_DSL_TEST_API §4.3)
//! - D3 color ignore (floodgate, no-face-up-security gate)
//! - D4/D5 security aura keyword grant (Alliance) + conditional replacement
//!   keyword grant (Scapegoat — G-ENGINE-AURA-GRANT-REPLACEMENT-KEYWORD)
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

const CARD_ID: &str = "BT25-097";

// ─── Fixture helpers ─────────────────────────────────────────────────────────

fn area_runner() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT25-097 YAML loads")
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
    card.colors = vec![CardColor::Yellow];
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
fn bt25_097_metadata_floodgate_auras_main_and_security_compile() {
    let runner = area_runner().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled BT25-097");

    assert_eq!(card.name, "Guardian Palace");
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

    // Security keyword auras: two grant_keyword auras (Alliance + conditional Scapegoat).
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
        "two security keyword auras (Alliance + conditional Scapegoat) must compile"
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
fn bt25_097_main_replaces_bottom_security_with_self_and_plays_reduced_ts() {
    let mut runner = area_runner()
        .add_card(filler("BOTTOM"))
        .add_card(filler("TOP"))
        .add_card(ts_digimon("YELLOW-TS", CardColor::Yellow, 6))
        .add_card(ts_digimon("PURPLE-TS", CardColor::Purple, 4))
        .add_card(ts_digimon("RED-TS", CardColor::Red, 4))
        .hand(0, &[CARD_ID, "YELLOW-TS", "PURPLE-TS", "RED-TS"])
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
        "Guardian Palace should be placed as the bottom security card"
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

    // Yellow/purple TS are eligible; red TS is not.
    let yellow_action = hand_action_for_id(&runner, "YELLOW-TS");
    assert!(hand_prompt.valid_action_ids.contains(&yellow_action));
    assert!(
        !hand_prompt
            .valid_action_ids
            .contains(&hand_action_for_id(&runner, "RED-TS")),
        "red TS is not a yellow/purple TS target"
    );

    runner
        .execute_action(hand_prompt.selecting_player, yellow_action)
        .expect("play yellow TS with reduced cost");

    assert!(runner.game.players[0]
        .battle_area
        .iter()
        .any(|perm| perm.top_card().card_id(&runner.game.card_data) == "YELLOW-TS"));
    assert_eq!(
        runner.memory(),
        memory_before - 6,
        "option cost 3 + YELLOW-TS play cost 6 reduced by 3 = 6 memory total"
    );
}

// ─── Section 3 — Behavior: security auras ─────────────────────────────────────

/// Seat Guardian Palace FACE-UP in P0's security (its [Main] effect would place
/// it there; security-scope auras materialize only from face-up security
/// sources — BT25-099 / BT24-090 idiom).
fn seat_face_up(runner: &mut DebugRunner) {
    let src_index = runner.game.players[0].security[0].card_index;
    runner.game.players[0].face_up_security.insert(src_index);
}

/// Alliance on yellow/purple TS Digimon while Guardian Palace is face-up in
/// security; red TS gets nothing.
#[test]
fn bt25_097_security_aura_grants_alliance_to_yellow_and_purple_ts() {
    let mut runner = area_runner()
        .add_card(ts_digimon("YELLOW-TS", CardColor::Yellow, 4))
        .add_card(ts_digimon("PURPLE-TS", CardColor::Purple, 4))
        .add_card(ts_digimon("RED-TS", CardColor::Red, 4))
        .security(0, &[CARD_ID])
        .start();
    seat_face_up(&mut runner);

    let yellow = runner.place_on_field(0, "YELLOW-TS", Some(0));
    let purple = runner.place_on_field(0, "PURPLE-TS", Some(0));
    let red = runner.place_on_field(0, "RED-TS", Some(0));
    runner.game.tick_declarative_effects();

    assert!(runner.game.has_keyword(yellow, Keyword::Alliance));
    assert!(runner.game.has_keyword(purple, Keyword::Alliance));
    assert!(
        !runner.game.has_keyword(red, Keyword::Alliance),
        "red TS gets no aura (not yellow/purple)"
    );
    assert!(
        !runner.game.has_keyword(yellow, Keyword::Scapegoat),
        "no [Junomon]-named Digimon → no Scapegoat grant"
    );
}

/// Negative: while Guardian Palace is face-DOWN in security, no aura applies.
#[test]
fn bt25_097_security_aura_inactive_while_face_down() {
    let mut runner = area_runner()
        .add_card(ts_digimon("YELLOW-TS", CardColor::Yellow, 4))
        .add_card(named_digimon("JUNO", "Junomon", CardColor::Yellow))
        .security(0, &[CARD_ID])
        .start();
    let yellow = runner.place_on_field(0, "YELLOW-TS", Some(0));
    runner.place_on_field(0, "JUNO", Some(0));
    runner.game.tick_declarative_effects();

    assert!(!runner.game.has_keyword(yellow, Keyword::Alliance));
    assert!(!runner.game.has_keyword(yellow, Keyword::Scapegoat));
}

/// With a [Junomon]-named Digimon (name CONTAINS Junomon), the same set also
/// gains <Scapegoat>.
#[test]
fn bt25_097_security_aura_grants_scapegoat_with_junomon_named_digimon() {
    let mut runner = area_runner()
        .add_card(ts_digimon("YELLOW-TS", CardColor::Yellow, 4))
        .add_card(ts_digimon("RED-TS", CardColor::Red, 4))
        .add_card(named_digimon("JUNO", "Junomon ACE", CardColor::Purple))
        .security(0, &[CARD_ID])
        .start();
    seat_face_up(&mut runner);
    let yellow = runner.place_on_field(0, "YELLOW-TS", Some(0));
    let red = runner.place_on_field(0, "RED-TS", Some(0));
    let juno = runner.place_on_field(0, "JUNO", Some(0));
    runner.game.tick_declarative_effects();

    assert!(runner.game.has_keyword(yellow, Keyword::Scapegoat));
    assert!(
        runner.game.has_keyword(juno, Keyword::Scapegoat),
        "the Junomon itself is a purple TS Digimon and gains Scapegoat too"
    );
    assert!(!runner.game.has_keyword(red, Keyword::Scapegoat));
}

/// The aura-granted <Scapegoat> is BEHAVIORAL (G-ENGINE-AURA-GRANT-
/// REPLACEMENT-KEYWORD): when the yellow TS Digimon would be deleted by an
/// opponent's effect, the controller may delete another of their Digimon
/// instead. Accept → pick → the substitute dies, the carrier survives.
#[test]
fn bt25_097_granted_scapegoat_substitutes_another_digimon_on_opponent_effect_deletion() {
    use digimon_engine::action::space::REPLACEMENT_ACCEPT;
    use digimon_engine::replacement::ReplacementCause;

    let mut runner = area_runner()
        .add_card(ts_digimon("YELLOW-TS", CardColor::Yellow, 4))
        .add_card(ts_digimon("RED-TS", CardColor::Red, 4))
        .add_card(named_digimon("JUNO", "Junomon", CardColor::Purple))
        .security(0, &[CARD_ID])
        .start();
    seat_face_up(&mut runner);
    let yellow = runner.place_on_field(0, "YELLOW-TS", Some(0));
    runner.place_on_field(0, "RED-TS", Some(0));
    runner.place_on_field(0, "JUNO", Some(0));
    runner.game.tick_declarative_effects();

    runner
        .game
        .delete_permanent_with_cause(yellow, ReplacementCause::OpponentEffect);

    {
        let pending = runner
            .game
            .pending_selection
            .as_ref()
            .expect("granted Scapegoat must park its optional accept dialog");
        assert!(pending.is_optional, "Scapegoat is a 'may'");
        assert_eq!(pending.selecting_player, 0);
        assert_eq!(pending.valid_action_ids, vec![REPLACEMENT_ACCEPT]);
    }
    runner
        .game
        .resolve_selection(0, REPLACEMENT_ACCEPT)
        .expect("accept Scapegoat");

    // Inner pick: another of your Digimon — RED-TS or JUNO (never the carrier).
    let red_action = {
        let pending = runner
            .game
            .pending_selection
            .as_ref()
            .expect("Scapegoat substitute pick must park");
        assert!(!pending.is_optional, "mandatory once accepted");
        assert_eq!(
            pending.valid_action_ids.len(),
            2,
            "both other own Digimon are legal substitutes (player choice)"
        );
        let red_idx = runner.game.players[0]
            .battle_area
            .iter()
            .position(|p| p.top_card().card_id(&runner.game.card_data) == "RED-TS")
            .unwrap();
        *pending
            .valid_action_ids
            .iter()
            .find(|a| {
                let off = (**a - digimon_engine::action::space::ATTACK_START)
                    % digimon_engine::action::space::TARGETS_PER_ATTACKER;
                off as usize == red_idx
            })
            .unwrap_or(&pending.valid_action_ids[0])
    };
    runner
        .game
        .resolve_selection(0, red_action)
        .expect("pick a substitute");
    runner.auto_resolve().expect("settle");

    let ids: Vec<String> = runner.game.players[0]
        .battle_area
        .iter()
        .map(|p| p.top_card().card_id(&runner.game.card_data).to_string())
        .collect();
    assert!(
        ids.contains(&"YELLOW-TS".to_string()),
        "the Scapegoat carrier survives: {ids:?}"
    );
    assert_eq!(ids.len(), 2, "exactly one substitute was deleted: {ids:?}");
    assert_eq!(runner.game.players[0].trash.len(), 1);
}

/// Negative: without a [Junomon]-named Digimon the deletion just happens —
/// no Scapegoat dialog parks.
#[test]
fn bt25_097_no_scapegoat_dialog_without_junomon() {
    use digimon_engine::replacement::ReplacementCause;

    let mut runner = area_runner()
        .add_card(ts_digimon("YELLOW-TS", CardColor::Yellow, 4))
        .add_card(ts_digimon("PURPLE-TS", CardColor::Purple, 4))
        .security(0, &[CARD_ID])
        .start();
    seat_face_up(&mut runner);
    let yellow = runner.place_on_field(0, "YELLOW-TS", Some(0));
    runner.place_on_field(0, "PURPLE-TS", Some(0));
    runner.game.tick_declarative_effects();

    runner
        .game
        .delete_permanent_with_cause(yellow, ReplacementCause::OpponentEffect);

    assert!(
        runner.game.pending_selection.is_none(),
        "no Junomon → no Scapegoat → no dialog"
    );
    assert_eq!(runner.game.players[0].battle_area.len(), 1);
    assert_eq!(
        runner.game.players[0].trash[0].card_id(&runner.game.card_data),
        "YELLOW-TS"
    );
}

/// Negative (16-31): a granted Scapegoat never answers your OWN effect's
/// deletion.
#[test]
fn bt25_097_granted_scapegoat_ignores_own_effect_deletion() {
    use digimon_engine::replacement::ReplacementCause;

    let mut runner = area_runner()
        .add_card(ts_digimon("YELLOW-TS", CardColor::Yellow, 4))
        .add_card(named_digimon("JUNO", "Junomon", CardColor::Purple))
        .security(0, &[CARD_ID])
        .start();
    seat_face_up(&mut runner);
    let yellow = runner.place_on_field(0, "YELLOW-TS", Some(0));
    runner.place_on_field(0, "JUNO", Some(0));
    runner.game.tick_declarative_effects();

    runner
        .game
        .delete_permanent_with_cause(yellow, ReplacementCause::OwnEffect);

    assert!(runner.game.pending_selection.is_none());
    assert_eq!(runner.game.players[0].battle_area.len(), 1);
}

// ─── Section 4 — Behavior: inherited [Security] effect ───────────────────────

#[test]
fn bt25_097_security_effect_plays_level_four_yellow_or_purple_ts_from_hand_or_trash_free() {
    let mut runner = area_runner()
        .add_card(ts_digimon("HAND-TS", CardColor::Purple, 4))
        .add_card(ts_digimon("TRASH-TS", CardColor::Yellow, 4))
        .add_card(ts_digimon("HIGH-TS", CardColor::Yellow, 5))
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
