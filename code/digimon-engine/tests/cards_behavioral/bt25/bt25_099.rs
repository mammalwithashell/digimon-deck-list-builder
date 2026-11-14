//! BT25-099 Gear Forest Village — Option, Green/Black, Cost 3.
//! Traits: Iliad, TS.
//!
//! # Card text (cards.json — verbatim)
//!
//! While you have no face-up security cards, you can ignore this card's color
//! requirements.
//! [Security] [Your Turn] All of your green or black [TS] trait Digimon gain
//! ＜Alliance＞ (When this Digimon attacks, by suspending 1 of your other
//! Digimon, add the suspended Digimon's DP to this Digimon and it gains
//! ＜Security A. +1＞ for the attack.) While you have [Bacchusmon] or [Ceresmon],
//! they also gain ＜Piercing＞ (When this Digimon attacks and deletes an
//! opponent's Digimon in battle, it checks security before the attack ends.)
//! [Main] Add your bottom security card to the hand and place this card face up
//! as the bottom security card. Then, you may play 1 green or black [TS] trait
//! Digimon card from your hand with the cost reduced by 3.
//!
//! Inherited (Security) effect:
//! [Security] You may play 1 level 4 or lower green or black [TS] trait Digimon
//! card from your hand or trash without paying the cost.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT25/Green/BT25_099.cs
//!
//! - Region "Ignore Color Requirement" (None): IgnoreColorConditionClass gated
//!   on owner having 0 non-flipped (face-up) security cards.
//! - Region "Alliance" (OnAllyAttack): AllianceStaticEffect over owner
//!   battle-area Digimon that are green OR black AND HasTSTraits, gated on this
//!   card being IN SECURITY (face-down).
//! - Region "Piercing" (None): PiercingStaticEffect over the same target set,
//!   gated on in-security AND owner has a permanent named "Bacchusmon" OR
//!   "Ceresmon".
//! - Region "Main Effect" (OptionSkill): ReplaceBottomSecurityWithFaceUpOption-
//!   Effect, then SelectHandEffect (PlayForCost) over green/black TS Digimon
//!   with cost reduced by 3, canNoSelect: true.
//! - Region "Security Effect" (SecuritySkill): SelectHand/SelectCard over level
//!   4- green/black TS Digimon from hand OR trash, PlayForFree, canNoSelect: true.
//!
//! # Patterns this test covers (RUST_DSL_TEST_API §4.3)
//! - D3 color ignore / bypass (floodgate, face-down-security gate)
//! - D4/D5 security aura (Alliance keyword) with conditional keyword grant (Piercing)
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

const CARD_ID: &str = "BT25-099";

// ─── Fixture helpers ─────────────────────────────────────────────────────────

fn area_runner() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT25-099 YAML loads")
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
    card.colors = vec![CardColor::Green];
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
fn bt25_099_metadata_floodgate_auras_main_and_security_compile() {
    let runner = area_runner().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled BT25-099");

    assert_eq!(card.name, "Gear Forest Village");
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

    // Security keyword auras: two grant_keyword auras (Alliance + conditional Piercing).
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
        "two security keyword auras (Alliance + conditional Piercing) must compile"
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
fn bt25_099_main_replaces_bottom_security_with_self_and_plays_reduced_ts() {
    let mut runner = area_runner()
        .add_card(filler("BOTTOM"))
        .add_card(filler("TOP"))
        .add_card(ts_digimon("GREEN-TS", CardColor::Green, 6))
        .add_card(ts_digimon("BLACK-TS", CardColor::Black, 4))
        .add_card(ts_digimon("RED-TS", CardColor::Red, 4))
        .hand(0, &[CARD_ID, "GREEN-TS", "BLACK-TS", "RED-TS"])
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
        "Gear Forest Village should be placed as the bottom security card"
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

    // Green/black TS are eligible; red TS is not.
    let green_action = hand_action_for_id(&runner, "GREEN-TS");
    assert!(hand_prompt.valid_action_ids.contains(&green_action));
    assert!(
        !hand_prompt
            .valid_action_ids
            .contains(&hand_action_for_id(&runner, "RED-TS")),
        "red TS is not a green/black TS target"
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

// ─── Section 3 — Behavior: security auras ─────────────────────────────────────

/// Alliance is granted to green/black TS Digimon while Gear Forest Village is
/// face-up in security; Piercing is granted only while a [Bacchusmon] or
/// [Ceresmon] is present.
#[test]
fn bt25_099_security_aura_grants_alliance_and_conditional_piercing() {
    let mut runner = area_runner()
        .add_card(ts_digimon("GREEN-TS", CardColor::Green, 4))
        .add_card(ts_digimon("BLACK-TS", CardColor::Black, 4))
        .add_card(ts_digimon("RED-TS", CardColor::Red, 4))
        .add_card(named_digimon("BACCHUS", "Bacchusmon", CardColor::Green))
        .security(0, &[CARD_ID])
        .start();

    // Gear Forest Village sits FACE-UP in P0's security (its [Main] effect would
    // place it there; security-scope auras materialize only from face-up
    // security sources — see BT25-095 / BT24-090 idiom).
    let src_index = runner.game.players[0].security[0].card_index;
    runner.game.players[0].face_up_security.insert(src_index);

    let green = runner.place_on_field(0, "GREEN-TS", Some(0));
    let black = runner.place_on_field(0, "BLACK-TS", Some(0));
    let red = runner.place_on_field(0, "RED-TS", Some(0));
    // Bacchusmon must be ON the field for the conditional Piercing gate to open.
    runner.place_on_field(0, "BACCHUS", Some(0));

    runner.game.tick_declarative_effects();

    // Alliance on green/black TS, not on red.
    assert!(
        runner.game.has_keyword(green, Keyword::Alliance),
        "green TS must gain Alliance"
    );
    assert!(
        runner.game.has_keyword(black, Keyword::Alliance),
        "black TS must gain Alliance"
    );
    assert!(
        !runner.game.has_keyword(red, Keyword::Alliance),
        "red TS gets no aura (not green/black)"
    );

    // Piercing granted because Bacchusmon is present.
    assert!(
        runner.game.has_keyword(green, Keyword::Piercing),
        "green TS must gain Piercing while Bacchusmon is present"
    );
    assert!(!runner.game.has_keyword(red, Keyword::Piercing));
}

/// Negative: without Bacchusmon or Ceresmon, green/black TS Digimon gain
/// Alliance but NOT Piercing.
#[test]
fn bt25_099_security_aura_no_piercing_without_bacchusmon_or_ceresmon() {
    let mut runner = area_runner()
        .add_card(ts_digimon("GREEN-TS", CardColor::Green, 4))
        .security(0, &[CARD_ID])
        .start();

    let src_index = runner.game.players[0].security[0].card_index;
    runner.game.players[0].face_up_security.insert(src_index);

    let green = runner.place_on_field(0, "GREEN-TS", Some(0));
    runner.game.tick_declarative_effects();

    assert!(
        runner.game.has_keyword(green, Keyword::Alliance),
        "Alliance still applies without Bacchusmon/Ceresmon"
    );
    assert!(
        !runner.game.has_keyword(green, Keyword::Piercing),
        "no Bacchusmon/Ceresmon → no Piercing grant"
    );
}

// ─── Section 4 — Behavior: inherited [Security] effect ───────────────────────

#[test]
fn bt25_099_security_effect_plays_level_four_green_or_black_ts_from_hand_or_trash_free() {
    let mut runner = area_runner()
        .add_card(ts_digimon("HAND-TS", CardColor::Black, 4))
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

// ─── Section 5 — Pick legality (select-hand audit 2026-11-13) ─────────────────
//
// [Main] DCGO BT25_099.cs:144-150 `CanSelectCardCondition`: `IsDigimon &&
// (Green || Black) && HasTSTraits && CanPlayAsNewPermanent(card, payCost: true,
// activateClass, fixedCost: GetCostItself - 3)`. [Security] BT25_099.cs:189-195
// `CanPlayCondition`: `IsDigimon && Level <= 4 && (Green || Black) && TS &&
// CanPlayAsNewPermanent(card, payCost: false, activateClass)`, checked against
// the hand and the trash separately. `CanPlayAsNewPermanent`
// (GameContextDeterminarion.cs:419-432) refuses an Option-kind card (a DUAL's
// kinds include Option: it is digivolved or used, never played, CR 4.3 4-6-2,
// 7-1-1), and needs an empty frame, no play prohibition, and (when paying) the
// reduced cost payable (CardSource.cs:1150-1160). A card that can't be played
// stays where it is (CR 4.3 7-1-2-3, 1-3-2). YAML: [Main]
// `can_play_from_effect_without_cost` + `affordable_with_cost: {reduce: 3}`;
// [Security] `can_play_from_effect_without_cost` (zone-aware: a trash copy
// fails under "can't play from the trash").

fn ts_with_cost(id: &str, cost: u16) -> CardData {
    let mut card = ts_digimon(id, CardColor::Green, 5);
    card.play_cost = cost;
    card
}

/// The non-PASS hand candidates of the pending pick, as sorted card ids; empty
/// when no hand pick is pending.
fn offered_hand_ids(runner: &DebugRunner, p: u8) -> Vec<String> {
    use digimon_engine::owned_card_choice::{CardChoiceOperation, CardChoiceZone};
    let Some(view) = runner.pending_selection_view() else {
        return Vec::new();
    };
    let hand = &runner.game.players[p as usize].hand;
    let id_at = |i: usize| hand.get(i).map(|c| c.card_id(&runner.game.card_data).to_string());
    let mut ids: Vec<String> = match view.card_choice {
        Some(choice) => choice
            .rows
            .iter()
            .filter(|row| {
                matches!(row.operation, CardChoiceOperation::CommitCard)
                    && matches!(row.zone, Some(CardChoiceZone::Hand))
            })
            .filter_map(|row| row.position.as_deref()?.parse::<usize>().ok())
            .filter_map(id_at)
            .collect(),
        None => view
            .valid_action_ids
            .iter()
            .copied()
            .filter(|&a| a != PASS)
            .filter_map(|a| a.checked_sub(digimon_engine::action::space::PLAY_HAND_START))
            .filter_map(|i| id_at(i as usize))
            .collect(),
    };
    ids.sort();
    ids
}

/// The non-PASS rows of the pending hand-or-trash union pick, as sorted
/// `"hand:<id>"` / `"trash:<id>"`; empty when no union pick is pending.
fn offered_union(runner: &DebugRunner, p: u8) -> Vec<String> {
    use digimon_engine::owned_card_choice::{CardChoiceOperation, CardChoiceZone};
    let Some(view) = runner.pending_selection_view() else {
        return Vec::new();
    };
    if !matches!(view.kind, SelectionKind::UnionZone { .. }) {
        return Vec::new();
    }
    let Some(choice) = view.card_choice else {
        return Vec::new();
    };
    let player = &runner.game.players[p as usize];
    let mut ids: Vec<String> = choice
        .rows
        .iter()
        .filter(|row| matches!(row.operation, CardChoiceOperation::CommitCard))
        .filter_map(|row| {
            let pos: usize = row.position.as_deref()?.parse().ok()?;
            let (zone, cards) = match row.zone.as_ref()? {
                CardChoiceZone::Hand => ("hand", &player.hand),
                CardChoiceZone::Trash => ("trash", &player.trash),
                _ => return None,
            };
            Some(format!("{zone}:{}", cards.get(pos)?.card_id(&runner.game.card_data)))
        })
        .collect();
    ids.sort();
    ids
}

/// Use BT25-099 from hand (with `extra` also in hand) at `memory`, run `setup`
/// first, and return the reduced-play hand candidates.
fn main_offered(extra: &[&str], memory: i16, setup: impl FnOnce(&mut DebugRunner)) -> Vec<String> {
    let mut hand = vec![CARD_ID];
    hand.extend_from_slice(extra);
    let mut runner = area_runner()
        .dsl_card("BT25-085")
        .expect("BeelStarmon (BT25-085) loads")
        .add_card(filler("BOTTOM"))
        .add_card(filler("TOP"))
        .add_card(filler("FIELD"))
        .add_card(ts_digimon("GREEN-TS", CardColor::Green, 6))
        .add_card(ts_with_cost("TS-13", 13))
        .add_card(ts_with_cost("TS-14", 14))
        .hand(0, &hand)
        .security(0, &["BOTTOM", "TOP"])
        .memory(memory)
        .start();
    runner.game.enter_main_phase();
    setup(&mut runner);
    let _ = runner.game.play_option_from_hand(0, 0);
    offered_hand_ids(&runner, 0)
}

/// BeelStarmon (BT25-085) is a black [TS] DUAL card: it is never played (CR 4.3
/// 4-6-2 + 7-1-1), so the reduced play offers only the Digimon card.
#[test]
fn bt25_099_main_pick_excludes_dual_card() {
    assert_eq!(
        main_offered(&["BT25-085", "GREEN-TS"], 10, |_| {}),
        vec!["GREEN-TS".to_string()]
    );
}

/// Memory 3 pays the Option (3) down to 0; the reduced play must still be
/// payable (down to -10): TS-13 (13 - 3 = 10) yes, TS-14 (11) no.
#[test]
fn bt25_099_main_pick_excludes_unaffordable_digimon() {
    assert_eq!(
        main_offered(&["TS-13", "TS-14"], 3, |_| {}),
        vec!["TS-13".to_string()]
    );
}

/// "You can't play Digimon by effects": nothing is offered.
#[test]
fn bt25_099_main_pick_excludes_digimon_under_effect_play_prohibition() {
    use digimon_engine::enums::{Expiry, ModifierType};
    use digimon_engine::modifiers::PlayerModifierEntry;
    let offered = main_offered(&["GREEN-TS"], 10, |r| {
        r.game.modifiers.add_player_modifier(
            0,
            PlayerModifierEntry::simple(ModifierType::CannotPlayDigimonByEffect, 0, Expiry::Permanent, None, 1),
        );
    });
    assert!(offered.is_empty(), "{offered:?}");
}

/// A full battle area: nothing is offered.
#[test]
fn bt25_099_main_pick_excludes_digimon_when_battle_area_full() {
    let offered = main_offered(&["GREEN-TS"], 10, |r| {
        r.place_on_field(0, "FIELD", None);
        r.game.rules.field_slots = r.game.players[0].battle_area.len() as u8;
    });
    assert!(offered.is_empty(), "{offered:?}");
}

/// Trigger BT25-099's [Security] for player 1 (HAND-TS in hand, TRASH-TS in
/// trash, both level 4 [TS]), after `setup`; return the union rows.
fn security_offered(setup: impl FnOnce(&mut DebugRunner)) -> Vec<String> {
    let mut runner = area_runner()
        .add_card(ts_digimon("HAND-TS", CardColor::Black, 4))
        .add_card(ts_digimon("TRASH-TS", CardColor::Green, 4))
        .add_card(attacker("ATTACKER"))
        .add_card(filler("FILL"))
        .hand(1, &["HAND-TS"])
        .deck(1, &["TRASH-TS"])
        .security(1, &[CARD_ID])
        .memory(10)
        .start();
    let trash_card = runner.game.players[1].deck.pop().expect("trash seed");
    runner.game.players[1].trash.push(trash_card);
    let attacker = runner.place_on_field(0, "ATTACKER", Some(0));
    setup(&mut runner);
    let _ = runner.attack_player(attacker, 1, false);
    offered_union(&runner, 1)
}

/// Regression guard: with nothing in the way both copies are offered.
#[test]
fn bt25_099_security_union_offers_hand_and_trash_copies() {
    assert_eq!(
        security_offered(|_| {}),
        vec!["hand:HAND-TS".to_string(), "trash:TRASH-TS".to_string()]
    );
}

/// "Can't play Digimon from the trash" (BT23-014 style lock): only the hand
/// copy is offered.
#[test]
fn bt25_099_security_union_excludes_trash_copy_under_trash_play_lock() {
    use digimon_engine::enums::{Expiry, ModifierType};
    use digimon_engine::modifiers::PlayerModifierEntry;
    let offered = security_offered(|r| {
        r.game.modifiers.add_player_modifier(
            1,
            PlayerModifierEntry::simple(ModifierType::CannotPlayFromTrash, 0, Expiry::Permanent, None, 1),
        );
    });
    assert_eq!(offered, vec!["hand:HAND-TS".to_string()]);
}

/// A full battle area: no copy is offered.
#[test]
fn bt25_099_security_union_offers_nothing_when_battle_area_full() {
    let offered = security_offered(|r| {
        r.place_on_field(1, "FILL", None);
        r.game.rules.field_slots = r.game.players[1].battle_area.len() as u8;
    });
    assert!(offered.is_empty(), "{offered:?}");
}

/// "You can't play Digimon by effects": no copy is offered.
#[test]
fn bt25_099_security_union_offers_nothing_under_effect_play_prohibition() {
    use digimon_engine::enums::{Expiry, ModifierType};
    use digimon_engine::modifiers::PlayerModifierEntry;
    let offered = security_offered(|r| {
        r.game.modifiers.add_player_modifier(
            1,
            PlayerModifierEntry::simple(ModifierType::CannotPlayDigimonByEffect, 0, Expiry::Permanent, None, 1),
        );
    });
    assert!(offered.is_empty(), "{offered:?}");
}
