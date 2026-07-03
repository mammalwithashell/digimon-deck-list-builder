//! BT24-094 Central Town: Throne Room — Option, Green/Yellow, Cost 3.
//! Traits: Iliad, TS.
//!
//! # Card text (official Bandai DB bundle data/card_bundles/BT24-094.md —
//! verbatim)
//!
//! While you have no face-up security cards, you can ignore this card's
//! color requirements.
//! [Security] [All Turns] All of your green or yellow [TS] trait Digimon get
//! +2000 DP. While you have [Merukimon] or [Minervamon], all of your green
//! or yellow [TS] trait Digimon gain ＜Alliance＞ (When this Digimon attacks,
//! by suspending 1 of your other Digimon, add the suspended Digimon's DP to
//! this Digimon and it gains ＜Security A. +1＞ for the attack.)
//! [Main] Add your bottom security card to the hand and place this card face
//! up as the bottom security card. Then, you may play 1 green or yellow
//! [TS] trait Digimon card from your hand with the play cost reduced by 3.
//!
//! Inherited (Security) effect:
//! [Security] You may play 1 level 4 or lower green or yellow [TS] trait
//! Digimon card from your hand or trash without paying the cost.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT24/Green/BT24_094.cs
//!
//! - Region "Ignore Color Requirement" (None): IgnoreColorConditionClass
//!   gated on owner having 0 non-flipped (face-up) security cards.
//! - Region "All Turns - Security DP" (None): ChangeDPStaticEffect(+2000)
//!   over owner battle-area Digimon that are green OR yellow AND
//!   HasTSTraits, gated on this card being IN SECURITY.
//! - Region "All Turns - Security Alliance" (OnAllyAttack):
//!   AllianceStaticEffect over the same target set, gated on in-security AND
//!   owner has a permanent named "Merukimon" OR "Minervamon".
//! - Region "Main Effect" (OptionSkill): ReplaceBottomSecurityWithFaceUpOption-
//!   Effect, then SelectHandEffect (PlayForCost) over green/yellow TS
//!   Digimon with cost reduced by 3, canNoSelect: true.
//! - Region "Security Effect" (SecuritySkill): SelectHand/SelectCard over
//!   level 4- green/yellow TS Digimon from hand OR trash, PlayForFree,
//!   canNoSelect: true.
//!
//! # Patterns this test covers (RUST_DSL_TEST_API §4.3)
//! - D3 color ignore / bypass (floodgate, face-up-security gate)
//! - D4/D5 security aura (+2000 DP, unconditional) with conditional keyword
//!   grant (Alliance, gated on named permanents)
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

const CARD_ID: &str = "BT24-094";

// ─── Fixture helpers ─────────────────────────────────────────────────────────

fn throne_room_runner() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT24-094 YAML loads")
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
fn bt24_094_metadata_floodgate_auras_main_and_security_compile() {
    let runner = throne_room_runner().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled BT24-094");

    assert_eq!(card.name, "Central Town: Throne Room");
    assert_eq!(card.kind, CompiledCardKind::Option);
    assert_eq!(card.cost, Some(3));
    assert!(card.use_requirement.is_some());

    // Color-ignore floodgate (gated on no face-up security).
    assert!(
        card.effects.iter().any(|clause| matches!(
            clause,
            CompiledClause::Declarative(CompiledDeclarativeClause::FloodGate { modifier, .. })
                if modifier == "IgnoreColorRequirement"
        )),
        "color-ignore floodgate must compile"
    );

    // Security +2000 DP aura.
    assert!(
        card.effects.iter().any(|clause| matches!(
            clause,
            CompiledClause::Declarative(CompiledDeclarativeClause::Aura {
                scope: CompiledScope::Security,
                dp_modifier: Some(2000),
                ..
            })
        )),
        "security +2000 DP aura must compile"
    );

    // Security conditional Alliance aura.
    assert!(
        card.effects.iter().any(|clause| matches!(
            clause,
            CompiledClause::Declarative(CompiledDeclarativeClause::Aura {
                scope: CompiledScope::Security,
                grant_keyword: Some(_),
                ..
            })
        )),
        "security conditional Alliance aura must compile"
    );

    // Exactly two security-scope auras (DP + conditional Alliance).
    assert_eq!(
        card.effects
            .iter()
            .filter(|clause| matches!(
                clause,
                CompiledClause::Declarative(CompiledDeclarativeClause::Aura {
                    scope: CompiledScope::Security,
                    ..
                })
            ))
            .count(),
        2,
        "security DP aura and conditional Alliance aura should both compile"
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
    let has_add_bottom = main
        .process
        .iter()
        .any(|s| matches!(s, CompiledStep::AddBottomSecurityToHand { .. }));
    let has_place_self = main
        .process
        .iter()
        .any(|s| matches!(s, CompiledStep::PlaceSelfOptionAtSecurity { .. }));
    let has_play = main
        .process
        .iter()
        .any(|s| matches!(s, CompiledStep::PlayFromHand { .. }));
    assert!(has_add_bottom, "main must add bottom security to hand");
    assert!(
        has_place_self,
        "main must place self face-up at bottom security"
    );
    assert!(
        has_play,
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

// ─── Section 2 — Behavior: use_requirement color bypass ──────────────────────

#[test]
fn bt24_094_color_bypass_allows_play_with_no_face_up_security() {
    let mut runner = throne_room_runner()
        .add_card(filler("BOTTOM"))
        .add_card(ts_digimon("GREEN-TS", CardColor::Green, 4))
        .hand(0, &[CARD_ID, "GREEN-TS"])
        .security(0, &["BOTTOM"])
        .memory(10)
        .start();
    runner.game.enter_main_phase();

    // No face-up security cards → use_requirement.face_up_security_count_lte(0)
    // is satisfied, so the option can be played despite an off-color deck
    // (no colored permanents needed to demonstrate this — the option's own
    // color check is bypassed by the floodgate/use_requirement). An eligible
    // green/yellow TS candidate must be in hand for the Main "you may play"
    // step to install a Pending selection rather than auto-resolving with
    // zero candidates (see `bt24_094_main_replaces_bottom_security_with_self_
    // and_plays_reduced_ts` for the full flow).
    assert_eq!(
        runner.game.play_option_from_hand(0, 0),
        OptionPlayResult::Pending,
        "no face-up security should satisfy the option's color bypass"
    );
}

// ─── Section 3 — Behavior: Main replace-bottom-security + reduced play ───────

#[test]
fn bt24_094_main_replaces_bottom_security_with_self_and_plays_reduced_ts() {
    let mut runner = throne_room_runner()
        .add_card(filler("BOTTOM"))
        .add_card(filler("TOP"))
        .add_card(ts_digimon("GREEN-TS", CardColor::Green, 6))
        .add_card(ts_digimon("YELLOW-TS", CardColor::Yellow, 4))
        .add_card(ts_digimon("BLUE-TS", CardColor::Blue, 4))
        .hand(0, &[CARD_ID, "GREEN-TS", "YELLOW-TS", "BLUE-TS"])
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
        "Central Town: Throne Room should be placed as the bottom security card"
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

    // Green/yellow TS are eligible; blue TS is not.
    let green_action = hand_action_for_id(&runner, "GREEN-TS");
    assert!(hand_prompt.valid_action_ids.contains(&green_action));
    assert!(
        !hand_prompt
            .valid_action_ids
            .contains(&hand_action_for_id(&runner, "BLUE-TS")),
        "blue TS is not a green/yellow TS target"
    );

    runner
        .execute_action(hand_prompt.selecting_player, green_action)
        .expect("play green TS with reduced cost");

    assert!(runner.game.players[0]
        .battle_area
        .iter()
        .any(|perm| perm.top_card().card_id(&runner.game.card_data) == "GREEN-TS"));
    assert_eq!(
        runner.memory(),
        memory_before - 6,
        "option cost 3 + GREEN-TS play cost 6 reduced by 3 = 6 memory total"
    );
}

/// The Main "you may play" step is optional — declining leaves the reduced
/// play unresolved with no permanent played and no extra memory spent.
#[test]
fn bt24_094_main_may_decline_the_reduced_ts_play() {
    let mut runner = throne_room_runner()
        .add_card(filler("BOTTOM"))
        .add_card(ts_digimon("GREEN-TS", CardColor::Green, 4))
        .hand(0, &[CARD_ID, "GREEN-TS"])
        .security(0, &["BOTTOM"])
        .memory(10)
        .start();
    runner.game.enter_main_phase();
    let memory_before = runner.memory();

    assert_eq!(
        runner.game.play_option_from_hand(0, 0),
        OptionPlayResult::Pending
    );
    let hand_prompt = runner
        .pending_selection_view()
        .expect("reduced play prompt");
    assert!(hand_prompt.is_optional);
    // PASS is not enumerated in `valid_action_ids` (that vec holds only the
    // concrete hand-card picks) but is always legal on an optional selection
    // — see `Game::resolve_generic_selection`'s `is_pass && sel.is_optional`
    // gate in code/digimon-engine/src/effect_queue.rs.

    runner
        .execute_action(hand_prompt.selecting_player, PASS)
        .expect("decline the optional reduced play");
    runner.auto_resolve().expect("settle main effect");

    assert!(
        !runner.game.players[0]
            .battle_area
            .iter()
            .any(|perm| perm.top_card().card_id(&runner.game.card_data) == "GREEN-TS"),
        "declining leaves GREEN-TS unplayed"
    );
    assert_eq!(
        runner.memory(),
        memory_before - 3,
        "only the option's own cost (3) is spent when the play is declined"
    );
}

// ─── Section 4 — Behavior: security auras ─────────────────────────────────────

/// +2000 DP applies to green/yellow TS Digimon while Central Town: Throne
/// Room is in security (face-up); Alliance is granted only while a
/// [Merukimon] or [Minervamon] is present.
#[test]
fn bt24_094_security_auras_grant_dp_and_conditional_alliance() {
    let mut runner = throne_room_runner()
        .add_card(ts_digimon("GREEN-TS", CardColor::Green, 4))
        .add_card(ts_digimon("YELLOW-TS", CardColor::Yellow, 4))
        .add_card(ts_digimon("BLUE-TS", CardColor::Blue, 4))
        .add_card(named_digimon("MERUKI", "Merukimon", CardColor::Green))
        .security(0, &[CARD_ID])
        .start();

    // Central Town: Throne Room sits FACE-UP in P0's security (its [Main]
    // effect would place it there; security-scope auras materialize from
    // face-up security sources — see ST20-15 / BT24-090 / BT25-095 idiom).
    let src_index = runner.game.players[0].security[0].card_index;
    runner.game.players[0].face_up_security.insert(src_index);

    let green = runner.place_on_field(0, "GREEN-TS", Some(0));
    let yellow = runner.place_on_field(0, "YELLOW-TS", Some(0));
    let blue = runner.place_on_field(0, "BLUE-TS", Some(0));
    // Merukimon must be ON the field for the conditional Alliance gate to open.
    runner.place_on_field(0, "MERUKI", Some(0));

    runner.game.tick_declarative_effects();

    // +2000 DP on green/yellow TS, not on blue.
    assert_eq!(
        runner.effective_dp(green),
        Some(6000),
        "green TS base 4000 + 2000 security aura"
    );
    assert_eq!(
        runner.effective_dp(yellow),
        Some(6000),
        "yellow TS base 4000 + 2000 security aura"
    );
    assert_eq!(
        runner.effective_dp(blue),
        Some(4000),
        "blue TS gets no aura (not green/yellow)"
    );

    // Alliance granted because Merukimon is present.
    assert!(
        runner.game.has_keyword(green, Keyword::Alliance),
        "green TS must gain Alliance while Merukimon is present"
    );
    assert!(
        runner.game.has_keyword(yellow, Keyword::Alliance),
        "yellow TS must gain Alliance while Merukimon is present"
    );
    assert!(!runner.game.has_keyword(blue, Keyword::Alliance));
}

/// Negative: without Merukimon or Minervamon, the green/yellow TS Digimon
/// get +2000 DP but NOT Alliance.
#[test]
fn bt24_094_security_aura_no_alliance_without_merukimon_or_minervamon() {
    let mut runner = throne_room_runner()
        .add_card(ts_digimon("GREEN-TS", CardColor::Green, 4))
        .security(0, &[CARD_ID])
        .start();

    let src_index = runner.game.players[0].security[0].card_index;
    runner.game.players[0].face_up_security.insert(src_index);

    let green = runner.place_on_field(0, "GREEN-TS", Some(0));
    runner.game.tick_declarative_effects();

    assert_eq!(
        runner.effective_dp(green),
        Some(6000),
        "+2000 DP still applies without Merukimon/Minervamon"
    );
    assert!(
        !runner.game.has_keyword(green, Keyword::Alliance),
        "no Merukimon/Minervamon → no Alliance grant"
    );
}

/// Alliance is also granted while [Minervamon] (rather than [Merukimon]) is
/// present, matching the printed "or" gate.
#[test]
fn bt24_094_security_aura_alliance_with_minervamon_alone() {
    let mut runner = throne_room_runner()
        .add_card(ts_digimon("YELLOW-TS", CardColor::Yellow, 4))
        .add_card(named_digimon("MINERVA", "Minervamon", CardColor::Yellow))
        .security(0, &[CARD_ID])
        .start();

    let src_index = runner.game.players[0].security[0].card_index;
    runner.game.players[0].face_up_security.insert(src_index);

    let yellow = runner.place_on_field(0, "YELLOW-TS", Some(0));
    runner.place_on_field(0, "MINERVA", Some(0));
    runner.game.tick_declarative_effects();

    assert!(
        runner.game.has_keyword(yellow, Keyword::Alliance),
        "yellow TS must gain Alliance while Minervamon is present"
    );
}

/// The security auras are sourced from Central Town: Throne Room being IN
/// the security zone; once it is no longer there (e.g. picked up to hand),
/// neither aura should apply.
#[test]
fn bt24_094_security_auras_do_not_apply_when_not_in_security() {
    let mut runner = throne_room_runner()
        .add_card(ts_digimon("GREEN-TS", CardColor::Green, 4))
        .hand(0, &[CARD_ID])
        .start();

    let green = runner.place_on_field(0, "GREEN-TS", Some(0));
    runner.game.tick_declarative_effects();

    assert_eq!(
        runner.effective_dp(green),
        Some(4000),
        "no DP aura while Throne Room sits in hand, not security"
    );
    assert!(!runner.game.has_keyword(green, Keyword::Alliance));
}

// ─── Section 5 — Behavior: inherited [Security] effect ───────────────────────

#[test]
fn bt24_094_security_effect_plays_level_four_green_or_yellow_ts_from_hand_or_trash_free() {
    let mut runner = throne_room_runner()
        .add_card(ts_digimon("HAND-TS", CardColor::Yellow, 4))
        .add_card(ts_digimon("TRASH-TS", CardColor::Green, 4))
        .add_card(ts_digimon("HIGH-TS", CardColor::Green, 5))
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

/// The inherited [Security] play is optional — the opponent's security check
/// may decline it, leaving both candidates untouched.
#[test]
fn bt24_094_security_effect_may_be_declined() {
    let mut runner = throne_room_runner()
        .add_card(ts_digimon("HAND-TS", CardColor::Green, 4))
        .add_card(attacker("ATTACKER"))
        .hand(1, &["HAND-TS"])
        .security(1, &[CARD_ID])
        .memory(10)
        .start();
    let attacker = runner.place_on_field(0, "ATTACKER", Some(0));
    let memory_before = runner.memory();

    let _ = runner.attack_player(attacker, 1, false);
    let union = runner
        .pending_selection_view()
        .expect("hand/trash union selection");
    assert!(union.is_optional, "the security play is a 'you may'");
    // PASS is always legal on an optional selection even though it is not
    // enumerated in `valid_action_ids` — see the sibling comment in
    // `bt24_094_main_may_decline_the_reduced_ts_play`.

    runner
        .execute_action(union.selecting_player, PASS)
        .expect("decline the optional security play");
    runner.auto_resolve().expect("settle security effect");

    assert!(
        !runner.game.players[1]
            .battle_area
            .iter()
            .any(|perm| perm.top_card().card_id(&runner.game.card_data) == "HAND-TS"),
        "declining leaves HAND-TS unplayed"
    );
    assert_eq!(runner.memory(), memory_before, "no memory spent on decline");
}
