//! ST21-15 Gennai's House — Option, White, Cost 2, traits: [ADVENTURE].
//!
//! # Card text (official Bandai DB bundle / cards.json — verbatim)
//!
//! While you have no face-up [Gennai's House] security cards, you can ignore
//! this card's color requirements.
//!
//! **[Security] [Your Turn]** All of your level 3 or higher Digimon get
//! +3000 DP.
//!
//! **[Main]** Add your bottom security card to the hand. Then, place this
//! card face up as the bottom security card.
//!
//! **Inherited (Security):** [Security] You may play 1 level 3 Digimon card
//! from your trash without paying the cost.
//!
//! # DCGO C# reference
//! `DCGO/Assets/Scripts/CardEffect/ST21/White/ST21_15.cs`
//!
//! - `IgnoreColorConditionClass` gated on no face-up (`!IsFlipped`) security
//!   card named "Gennai's House".
//! - `ChangeDPStaticEffect(+3000)` over own battle-area Digimon of level >= 3,
//!   gated on `IsExistInSecurity(card, false)` (face up) and `IsOwnerTurn`.
//! - `ReplaceBottomSecurityWithFaceUpOptionMainEffect`.
//! - `SecuritySkill`: optional `SelectCardEffect` over trash Digimon with
//!   `Level == 3`, then `PlayPermanentCards(payCost: false)`.
//!
//! # Patterns this test covers
//!
//! - **D3 colour bypass gated on "no face-up <named> security"**
//!   (`use_requirement` + `no_face_up_security_named { name_is }`), with the
//!   face-down and differently-named negatives. (ST20-15 / BT24-094 siblings.)
//! - **Pink `{Security}` filtered aura** (`scope: security`, `[Your Turn]`
//!   gate): positive, level floor, own-only, face-down, opponent's turn.
//! - **Replace-bottom-security-with-self Main** (BT24-094 / BT25-095 idiom),
//!   including the empty-security edge case and the gate closing afterwards.
//! - **Inherited blue [Security] optional play from trash**, level-3 filter,
//!   accept / decline, driven through a real security check.

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_dsl::compiled::{
    CompiledCardKind, CompiledClause, CompiledColor, CompiledDeclarativeClause, CompiledScope,
    CompiledStackPosition, CompiledStep, CompiledTiming,
};
use digimon_engine::action::space::{PASS, PLAY_HAND_START};
use digimon_engine::build_action_mask;
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind};
use digimon_engine::selection::{OptionPlayResult, SelectionKind};

use crate::dsl_card_data::compiled;

const CARD_ID: &str = "ST21-15";

// ─── Card-data factories ─────────────────────────────────────────────────────

fn filler(id: &str) -> CardData {
    make_test_card(id, id)
}

fn digimon_lv(id: &str, level: u8, dp: i32) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(level);
    c.dp = Some(dp);
    c.play_cost = u16::from(level);
    c.colors = vec![CardColor::White];
    c
}

fn tamer(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Tamer;
    c.level = None;
    c.dp = None;
    c.play_cost = 3;
    c.colors = vec![CardColor::White];
    c
}

/// Mark the security card at `idx` (storage order, 0 = bottom) face up.
fn flip_face_up(runner: &mut DebugRunner, player: usize, idx: usize) -> u32 {
    let card_index = runner.game.players[player].security[idx].card_index;
    runner.game.players[player]
        .face_up_security
        .insert(card_index);
    card_index as u32
}

fn playable(runner: &DebugRunner) -> f32 {
    build_action_mask(&runner.game, 0)[PLAY_HAND_START as usize]
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 1 — Structural assertions
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn st21_15_compiles_as_white_option_cost_2_with_adventure_trait() {
    let card = compiled(CARD_ID);
    assert_eq!(card.card, CARD_ID);
    assert_eq!(card.name, "Gennai's House");
    assert_eq!(card.kind, CompiledCardKind::Option);
    assert_eq!(card.color, vec![CompiledColor::White]);
    assert_eq!(card.cost, Some(2), "ST21-15 prints Cost 2");
    assert!(
        card.traits
            .iter()
            .any(|t| t.eq_ignore_ascii_case("ADVENTURE")),
        "ST21-15 must carry the ADVENTURE trait"
    );
    assert!(
        card.use_requirement.is_some(),
        "colour bypass is carried by the card-level use_requirement"
    );
}

#[test]
fn st21_15_has_four_clauses_in_expected_order() {
    let card = compiled(CARD_ID);
    assert_eq!(card.effects.len(), 4);

    // 0: flood_gate IgnoreColorRequirement
    assert!(matches!(
        &card.effects[0],
        CompiledClause::Declarative(CompiledDeclarativeClause::FloodGate { modifier, .. })
            if modifier == "IgnoreColorRequirement"
    ));

    // 1: security-scope aura, +3000 DP, [Your Turn]
    match &card.effects[1] {
        CompiledClause::Declarative(CompiledDeclarativeClause::Aura {
            scope,
            dp_modifier,
            active_when,
            ..
        }) => {
            assert_eq!(*scope, CompiledScope::Security, "pink {{Security}} aura");
            assert_eq!(*dp_modifier, Some(3000));
            assert!(
                active_when
                    .as_ref()
                    .is_some_and(|p| p.your_turn == Some(true)),
                "aura must be gated to [Your Turn]; got {active_when:?}"
            );
        }
        other => panic!("clause 1 must be the security aura; got {other:?}"),
    }

    // 2: main_from_hand: add bottom security, place self face-up bottom
    match &card.effects[2] {
        CompiledClause::Triggered(t) => {
            assert_eq!(t.when, vec![CompiledTiming::MainFromHand]);
            assert!(!t.optional, "[Main] has no 'may'");
            assert!(
                matches!(
                    t.process.as_slice(),
                    [
                        CompiledStep::AddBottomSecurityToHand { .. },
                        CompiledStep::PlaceSelfOptionAtSecurity { .. },
                    ]
                ),
                "[Main] body shape; got {:?}",
                t.process
            );
        }
        other => panic!("clause 2 must be Triggered(main_from_hand); got {other:?}"),
    }

    // 3: inherited on_security, optional, select_trash + play_from_trash_free
    match &card.effects[3] {
        CompiledClause::Triggered(t) => {
            assert_eq!(t.scope, CompiledScope::Inherited);
            assert_eq!(t.when, vec![CompiledTiming::OnSecurity]);
            assert!(t.optional, "printed 'You may play'");
            assert!(t
                .process
                .iter()
                .any(|s| matches!(s, CompiledStep::SelectTrash { .. })));
            assert!(t
                .process
                .iter()
                .any(|s| matches!(s, CompiledStep::PlayFromTrashFree { .. })));
        }
        other => panic!("clause 3 must be Triggered(inherited on_security); got {other:?}"),
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 2 — Colour bypass: "no face-up [Gennai's House] security cards"
// ═══════════════════════════════════════════════════════════════════════════════

/// Positive: no face-up [Gennai's House] in own security → playable even
/// though no white colour source is on the field.
#[test]
fn st21_15_playable_when_no_face_up_gennais_house_in_security() {
    let runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("ST21-15 in embedded DSL pack")
        .add_card(filler("FILL"))
        .hand(0, &[CARD_ID])
        .security(0, &["FILL", "FILL"])
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();

    assert_eq!(
        playable(&runner),
        1.0,
        "colour requirement bypassed while no face-up [Gennai's House] sits in own security"
    );
}

/// Negative: a face-up [Gennai's House] in own security closes the gate; a
/// face-DOWN one does not (printed "face-up" qualifier).
#[test]
fn st21_15_gate_closes_for_face_up_copy_but_not_face_down_copy() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("ST21-15 in embedded DSL pack")
        .add_card(filler("FILL"))
        .hand(0, &[CARD_ID])
        .security(0, &[CARD_ID, "FILL"])
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();

    // Face-down copy in security: gate still open.
    assert_eq!(
        playable(&runner),
        1.0,
        "a face-DOWN [Gennai's House] must not close the colour-bypass gate"
    );

    // Flip it face up: gate closes (no white source on the field → unplayable).
    flip_face_up(&mut runner, 0, 0);
    assert_eq!(
        playable(&runner),
        0.0,
        "a face-up [Gennai's House] in own security closes the gate"
    );
}

/// Negative: a face-up security card with a DIFFERENT name does not close it.
#[test]
fn st21_15_gate_ignores_face_up_security_cards_with_other_names() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("ST21-15 in embedded DSL pack")
        .add_card(filler("OTHER"))
        .hand(0, &[CARD_ID])
        .security(0, &["OTHER", "OTHER"])
        .deck(0, &["OTHER"])
        .deck(1, &["OTHER"])
        .memory(10)
        .start();
    flip_face_up(&mut runner, 0, 0);
    flip_face_up(&mut runner, 0, 1);

    assert_eq!(
        playable(&runner),
        1.0,
        "only a face-up [Gennai's House] closes the gate, not other face-up security cards"
    );
}

/// "While YOU have": a face-up [Gennai's House] in the OPPONENT's security
/// does not close the gate.
#[test]
fn st21_15_gate_ignores_opponents_face_up_gennais_house() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("ST21-15 in embedded DSL pack")
        .add_card(filler("FILL"))
        .hand(0, &[CARD_ID])
        .security(0, &["FILL"])
        .security(1, &[CARD_ID])
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();
    flip_face_up(&mut runner, 1, 0);

    assert_eq!(
        playable(&runner),
        1.0,
        "the opponent's face-up [Gennai's House] must not close MY gate"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 3 — [Security][Your Turn] +3000 DP aura
// ═══════════════════════════════════════════════════════════════════════════════

/// While face up in P0's security, on P0's turn: own Lv3+ Digimon get +3000;
/// a Lv2 does not; the opponent's Digimon do not.
#[test]
fn st21_15_security_aura_buffs_own_level_3_plus_digimon_only() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("ST21-15 in embedded DSL pack")
        .add_card(digimon_lv("LV2", 2, 2000))
        .add_card(digimon_lv("LV3", 3, 3000))
        .add_card(digimon_lv("LV4", 4, 4000))
        .add_card(digimon_lv("OPP-LV3", 3, 3000))
        .security(0, &[CARD_ID])
        .memory(10)
        .start();
    flip_face_up(&mut runner, 0, 0);

    let lv2 = runner.place_on_field(0, "LV2", Some(0));
    let lv3 = runner.place_on_field(0, "LV3", Some(0));
    let lv4 = runner.place_on_field(0, "LV4", Some(0));
    let opp = runner.place_on_field(1, "OPP-LV3", Some(0));
    assert_eq!(runner.turn_player(), 0, "precondition: P0's turn");

    runner.game.tick_declarative_effects();

    assert_eq!(runner.effective_dp(lv2), Some(2000), "Lv2 is below the floor");
    assert_eq!(
        runner.effective_dp(lv3),
        Some(6000),
        "Lv3 boundary: 3000 + 3000"
    );
    assert_eq!(runner.effective_dp(lv4), Some(7000), "Lv4: 4000 + 3000");
    assert_eq!(
        runner.effective_dp(opp),
        Some(3000),
        "the opponent's Digimon are not 'your' Digimon"
    );
}

/// Face-down: the pink {Security} aura is never active.
#[test]
fn st21_15_security_aura_inactive_while_face_down() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("ST21-15 in embedded DSL pack")
        .add_card(digimon_lv("LV3", 3, 3000))
        .security(0, &[CARD_ID])
        .memory(10)
        .start();
    let lv3 = runner.place_on_field(0, "LV3", Some(0));

    runner.game.tick_declarative_effects();
    assert_eq!(
        runner.effective_dp(lv3),
        Some(3000),
        "a face-down [Gennai's House] must not buff"
    );

    flip_face_up(&mut runner, 0, 0);
    runner.game.tick_declarative_effects();
    assert_eq!(
        runner.effective_dp(lv3),
        Some(6000),
        "once face up the aura applies"
    );
}

/// [Your Turn] gate: on the opponent's turn the buff is off.
#[test]
fn st21_15_security_aura_inactive_on_opponents_turn() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("ST21-15 in embedded DSL pack")
        .add_card(digimon_lv("LV3", 3, 3000))
        .add_card(filler("FILL"))
        .security(0, &[CARD_ID])
        .deck(0, &["FILL", "FILL", "FILL"])
        .deck(1, &["FILL", "FILL", "FILL"])
        .memory(0) // 0 memory: end_turn hands the turn to P1
        .start();
    flip_face_up(&mut runner, 0, 0);
    let lv3 = runner.place_on_field(0, "LV3", Some(0));

    runner.game.tick_declarative_effects();
    assert_eq!(
        runner.effective_dp(lv3),
        Some(6000),
        "precondition: buffed on P0's own turn"
    );

    runner.end_turn();
    assert_eq!(runner.turn_player(), 1, "precondition: P1's turn");
    runner.game.tick_declarative_effects();
    assert_eq!(
        runner.effective_dp(lv3),
        Some(3000),
        "[Your Turn] aura must be off during the opponent's turn"
    );
}

/// The aura belongs to the security card's OWNER: P1's face-up Gennai's House
/// never buffs P0's Digimon (and is off on P0's turn for P1's own Digimon).
#[test]
fn st21_15_security_aura_of_opponents_copy_does_not_buff_my_digimon() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("ST21-15 in embedded DSL pack")
        .add_card(digimon_lv("LV3", 3, 3000))
        .add_card(digimon_lv("OPP-LV3", 3, 3000))
        .security(1, &[CARD_ID])
        .memory(10)
        .start();
    flip_face_up(&mut runner, 1, 0);
    let mine = runner.place_on_field(0, "LV3", Some(0));
    let theirs = runner.place_on_field(1, "OPP-LV3", Some(0));
    assert_eq!(runner.turn_player(), 0, "precondition: P0's turn");

    runner.game.tick_declarative_effects();

    assert_eq!(runner.effective_dp(mine), Some(3000));
    assert_eq!(
        runner.effective_dp(theirs),
        Some(3000),
        "P1's copy is [Your Turn] for P1 only — inactive on P0's turn"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 4 — [Main] add bottom security to hand, place self face up at bottom
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn st21_15_main_swaps_bottom_security_for_self_face_up() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("ST21-15 in embedded DSL pack")
        .add_card(filler("SEC-BOTTOM"))
        .add_card(filler("SEC-TOP"))
        .hand(0, &[CARD_ID])
        .security(0, &["SEC-BOTTOM", "SEC-TOP"])
        .deck(0, &["SEC-TOP"])
        .deck(1, &["SEC-TOP"])
        .memory(10)
        .start();
    runner.game.enter_main_phase();
    let mem_before = runner.memory();

    let result = runner.game.play_option_from_hand(0, 0);
    assert_ne!(
        result,
        OptionPlayResult::Invalid,
        "ST21-15 must be playable (colour bypass)"
    );
    let _ = runner.auto_resolve();

    let hand: Vec<String> = runner.game.players[0]
        .hand
        .iter()
        .map(|c| c.card_id(&runner.game.card_data).to_string())
        .collect();
    assert_eq!(
        hand,
        vec!["SEC-BOTTOM".to_string()],
        "the former BOTTOM security card moves to the hand; got {hand:?}"
    );

    let sec = &runner.game.players[0].security;
    assert_eq!(sec.len(), 2, "security count is unchanged (swap)");
    assert_eq!(
        sec[0].card_id(&runner.game.card_data),
        CARD_ID,
        "ST21-15 is the new BOTTOM security card"
    );
    assert_eq!(
        sec[1].card_id(&runner.game.card_data),
        "SEC-TOP",
        "the top security card is untouched"
    );
    assert!(
        runner.game.players[0]
            .face_up_security
            .contains(&sec[0].card_index),
        "ST21-15 must be placed FACE UP"
    );
    assert!(
        !runner.game.players[0]
            .face_up_security
            .contains(&sec[1].card_index),
        "the other security card stays face down"
    );
    assert_eq!(
        mem_before - runner.memory(),
        2,
        "using the Option costs its printed 2 memory"
    );
}

/// Edge: with an empty security stack nothing is added to the hand, but the
/// card is still placed face up as the (only) security card (DCGO guards the
/// add with `SecurityCards.Count >= 1`, the placement with `CanAddSecurity`).
#[test]
fn st21_15_main_with_empty_security_still_places_self_face_up() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("ST21-15 in embedded DSL pack")
        .add_card(filler("FILL"))
        .hand(0, &[CARD_ID])
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();
    runner.game.enter_main_phase();

    let result = runner.game.play_option_from_hand(0, 0);
    assert_ne!(result, OptionPlayResult::Invalid);
    let _ = runner.auto_resolve();

    assert!(
        runner.game.players[0].hand.is_empty(),
        "nothing to add to the hand from an empty security stack"
    );
    let sec = &runner.game.players[0].security;
    assert_eq!(sec.len(), 1, "ST21-15 becomes the only security card");
    assert_eq!(sec[0].card_id(&runner.game.card_data), CARD_ID);
    assert!(runner.game.players[0]
        .face_up_security
        .contains(&sec[0].card_index));
}

/// After [Main] places a face-up copy, the colour-bypass gate closes for a
/// second copy still in hand.
#[test]
fn st21_15_main_followup_closes_the_colour_gate_for_a_second_copy() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("ST21-15 in embedded DSL pack")
        .add_card(filler("SEC-BOTTOM"))
        .add_card(filler("SEC-TOP"))
        .hand(0, &[CARD_ID, CARD_ID])
        .security(0, &["SEC-BOTTOM", "SEC-TOP"])
        .deck(0, &["SEC-TOP"])
        .deck(1, &["SEC-TOP"])
        .memory(10)
        .start();
    runner.game.enter_main_phase();

    assert_eq!(
        playable(&runner),
        1.0,
        "precondition: second copy playable before any face-up copy exists"
    );
    let result = runner.game.play_option_from_hand(0, 0);
    assert_ne!(result, OptionPlayResult::Invalid);
    let _ = runner.auto_resolve();

    // The remaining copy is now hand index 0 (SEC-BOTTOM was appended after it).
    let idx = runner.game.players[0]
        .hand
        .iter()
        .position(|c| c.card_id(&runner.game.card_data) == CARD_ID)
        .expect("second copy still in hand");
    let mask = build_action_mask(&runner.game, 0);
    assert_eq!(
        mask[PLAY_HAND_START as usize + idx],
        0.0,
        "a face-up [Gennai's House] in security closes the colour gate"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 5 — Inherited [Security]: may play 1 Lv3 Digimon from trash free
// ═══════════════════════════════════════════════════════════════════════════════

fn security_runner(trash_cards: &[&str]) -> DebugRunner {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("ST21-15 in embedded DSL pack")
        .add_card(filler("ATTACKER"))
        .add_card(digimon_lv("TRASH-LV3", 3, 3000))
        .add_card(digimon_lv("TRASH-LV4", 4, 4000))
        .add_card(digimon_lv("TRASH-LV2", 2, 2000))
        .add_card(tamer("TRASH-TAMER"))
        .security(1, &[CARD_ID])
        .memory(10)
        .start();
    for id in trash_cards {
        runner.inject_trash(1, id);
    }
    runner
}

fn p1_field_ids(runner: &DebugRunner) -> Vec<String> {
    runner
        .game
        .player(1)
        .battle_area
        .iter()
        .map(|p| p.top_card().card_id(&runner.game.card_data).to_string())
        .collect()
}

/// Accept: only the Lv3 Digimon is offered (Lv4 / Lv2 Digimon and a Tamer are
/// not); choosing it plays it onto P1's field for free.
#[test]
fn st21_15_security_effect_plays_a_level_3_digimon_from_trash_free() {
    let mut runner = security_runner(&["TRASH-LV4", "TRASH-LV3", "TRASH-LV2", "TRASH-TAMER"]);
    let attacker = runner.place_on_field(0, "ATTACKER", Some(0));
    let mem_before = runner.memory();

    let _ = runner.attack_player(attacker, 1, false);
    let view = runner
        .pending_selection_view()
        .expect("[Security] trash selection must be offered to the security owner");
    assert_eq!(view.kind, SelectionKind::Trash);
    assert_eq!(view.selecting_player, 1, "the security card's owner chooses");
    assert!(view.is_optional, "printed 'You may play'");
    let picks: Vec<u16> = view
        .valid_action_ids
        .iter()
        .copied()
        .filter(|&a| a != PASS)
        .collect();
    assert_eq!(
        picks.len(),
        1,
        "only the level 3 Digimon card is a legal pick; got {picks:?}"
    );

    runner
        .execute_action(view.selecting_player, picks[0])
        .expect("play the level 3 Digimon");
    let _ = runner.auto_resolve();

    let field = p1_field_ids(&runner);
    assert!(
        field.contains(&"TRASH-LV3".to_string()),
        "TRASH-LV3 must be played; P1 field: {field:?}"
    );
    for not_played in ["TRASH-LV4", "TRASH-LV2", "TRASH-TAMER"] {
        assert!(
            !field.contains(&not_played.to_string()),
            "{not_played} is not a legal pick; P1 field: {field:?}"
        );
    }
    assert_eq!(runner.memory(), mem_before, "the play is free");
}

/// Decline: PASS leaves the trash untouched.
#[test]
fn st21_15_security_effect_may_be_declined() {
    let mut runner = security_runner(&["TRASH-LV3"]);
    let attacker = runner.place_on_field(0, "ATTACKER", Some(0));
    let trash_before = runner.trash_size(1);

    let _ = runner.attack_player(attacker, 1, false);
    let view = runner
        .pending_selection_view()
        .expect("[Security] trash selection offered");
    assert!(view.is_optional);
    runner
        .execute_action(view.selecting_player, PASS)
        .expect("decline the optional play");
    let _ = runner.auto_resolve();

    assert!(
        !p1_field_ids(&runner).contains(&"TRASH-LV3".to_string()),
        "declining leaves TRASH-LV3 in the trash"
    );
    assert!(
        runner
            .game
            .player(1)
            .trash
            .iter()
            .any(|c| c.card_id(&runner.game.card_data) == "TRASH-LV3"),
        "TRASH-LV3 remains in P1's trash"
    );
}

/// No legal Lv3 Digimon in trash: nothing can be played.
#[test]
fn st21_15_security_effect_plays_nothing_without_a_level_3_digimon_in_trash() {
    let mut runner = security_runner(&["TRASH-LV4", "TRASH-LV2", "TRASH-TAMER"]);
    let attacker = runner.place_on_field(0, "ATTACKER", Some(0));

    let _ = runner.attack_player(attacker, 1, false);
    let _ = runner.auto_resolve();

    let field = p1_field_ids(&runner);
    for id in ["TRASH-LV4", "TRASH-LV2", "TRASH-TAMER", "TRASH-LV3"] {
        assert!(
            !field.contains(&id.to_string()),
            "{id} must not be played; P1 field: {field:?}"
        );
    }
}

#[allow(dead_code)]
fn _unused_silencer() {
    let _ = tamer("X");
    let _ = SelectionKind::Trash;
}
