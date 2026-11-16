//! AD1-002 Aldamon — Lv.5, Red, DP 8000, Cost 8.
//! Traits: Hybrid / Variable / Wizard.
//!
//! # Card text (card face / cards.json)
//!
//! ```text
//! ＜Rush＞ (This Digimon can attack the turn it comes into play.)
//! [When Digivolving] Delete 1 of your opponent's Digimon with as much or
//!     less DP as this Digimon.
//! [End of Attack] [On Deletion] You may trash 1 [Hybrid] or [Ten Warriors]
//!     trait card from your hand. If this effect trashed, ＜Draw 2＞. Then,
//!     you may play 1 red, blue or green Tamer card with inherited effects
//!     from your hand or trash without paying the cost.
//! ```
//!
//! Inherited Effect: [Your Turn] This Digimon gets +4000 DP.
//!
//! Alt-digivolve (xros_req): [Takuya Kanbara] w/2+ [Hybrid] trait cards under,
//! Cost 3. Standard evo cost Lv4 Red Cost 3.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/AD1/Red/AD1_002.cs
//!   - Alt-digivolve: AddSelfDigivolutionRequirementStaticEffect, Takuya Kanbara
//!     top card + ≥2 Hybrid digivolution sources, cost 3.
//!   - Rush: RushSelfStaticEffect (face-up).
//!   - When Digivolving: OnEnterFieldAnyone, mandatory SelectPermanentEffect
//!     Mode.Destroy over opponent Digimon with DP <= card.PermanentOfThisCard().DP
//!     (canNoSelect: false), gated on a match existing.
//!   - Shared EoA/OD (endOfAttack + onDeletion, optional: false outer):
//!       1. SelectHandEffect Mode.Discard, max 1, canNoSelect: true, over
//!          Hybrid OR Ten Warriors cards. `discarded` set iff a card chosen.
//!       2. if (discarded) DrawClass(owner, 2).
//!       3. CanSelectPlayCondition: IsTamer && (Red|Blue|Green) &&
//!          HasInheritedEffect && CanPlayAsNewPermanent(payCost: false).
//!          3-way hand/trash/don't-play int selection, then free play from the
//!          chosen zone. Runs regardless of whether the trash/draw happened.
//!   - Inherited: ChangeSelfDPStaticEffect(+4000, isInherited) gated IsOwnerTurn.
//!
//! # Per-clause status
//!
//! | Clause | Status | Pattern |
//! |--------|--------|---------|
//! | ＜Rush＞                                          | PASS | grant_keyword Rush (face-up) |
//! | Alt-digi (Takuya Kanbara + 2 Hybrid / cost 3)    | PASS (structural) | alt_paths kind: digivolve |
//! | [When Digivolving] delete opp DP <= self          | PASS | select_opponent_permanent dp_lte source_dp + delete |
//! | [EoA][OD] may trash Hybrid/Ten Warriors           | PASS | optional select_hand cost + trash_from_hand_by_index |
//! | [EoA][OD] if trashed, Draw 2                       | PASS | if binding_present(trashed) -> draw 2 |
//! | [EoA][OD] may free-play R/U/G Tamer w/ inherited   | PASS | select_union_zone hand/trash + play_union_bound_free |
//! | Inherited [Your Turn] +4000 DP                     | PASS | scope: inherited aura dp_modifier_fn 4000, your_turn |

#![allow(unused_imports, dead_code)]

use digimon_dsl::compiled::{
    CompiledClause, CompiledDeclarativeClause, CompiledScope, CompiledTiming,
};
use digimon_engine::action::space::{encode_attack, PASS};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, make_test_card_with_level, DebugRunner};
use digimon_engine::enums::{CardKind, EffectTiming};
use digimon_engine::selection::{SelectionKind, TriggerSource};

const CARD_ID: &str = "AD1-002";

fn digimon(id: &str, name: &str, level: u8, dp: i32) -> CardData {
    let mut card = make_test_card_with_level(id, name, level);
    card.card_kind = CardKind::Digimon;
    card.dp = Some(dp);
    card
}

fn tamer(id: &str, name: &str) -> CardData {
    let mut card = make_test_card(id, name);
    card.card_kind = CardKind::Tamer;
    card
}

fn aldamon_runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("AD1-002 YAML loads from embedded DSL pack")
        // Opponent Digimon: one eligible (DP <= 8000), one too big (DP > 8000).
        .add_card(digimon("ELIGIBLE", "Eligible", 5, 7000))
        .add_card(digimon("TOO-BIG", "TooBig", 6, 10000))
        // Hand fillers for the EoA/OD trash.
        .add_card({
            let mut c = make_test_card("HYBRID-CARD", "HybridCard");
            c.card_kind = CardKind::Digimon;
            c.traits = vec!["Hybrid".to_string()];
            c.dp = Some(3000);
            c
        })
        .add_card(make_test_card("PLAIN-CARD", "PlainCard"))
        .memory(10)
        .start()
}

// ════════════════════════════════════════════════════════════════════════════
// Section 1 — Structural assertions
// ════════════════════════════════════════════════════════════════════════════

#[test]
fn ad1_002_compiles_and_is_in_pack() {
    let runner = aldamon_runner();
    assert!(
        runner.compiled_card(CARD_ID).is_some(),
        "AD1-002 must be present in the embedded DSL pack"
    );
}

#[test]
fn ad1_002_grant_keyword_rush_present() {
    let runner = aldamon_runner();
    let compiled = runner.compiled_card(CARD_ID).unwrap();
    let found = compiled.effects.iter().any(|c| {
        matches!(
            c,
            CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword { keyword, scope, .. })
                if keyword == "Rush" && *scope == CompiledScope::FaceUp
        )
    });
    assert!(found, "face-up GrantKeyword(Rush) clause must be present");
}

#[test]
fn ad1_002_when_digivolving_clause_present() {
    let runner = aldamon_runner();
    let compiled = runner.compiled_card(CARD_ID).unwrap();
    let found = compiled.effects.iter().any(|c| {
        matches!(c, CompiledClause::Triggered(t)
            if t.when.contains(&CompiledTiming::WhenDigivolving))
    });
    assert!(found, "[When Digivolving] triggered clause must be present");
}

#[test]
fn ad1_002_end_of_attack_on_deletion_clause_present() {
    let runner = aldamon_runner();
    let compiled = runner.compiled_card(CARD_ID).unwrap();
    let found = compiled.effects.iter().any(|c| {
        matches!(c, CompiledClause::Triggered(t)
            if t.when.contains(&CompiledTiming::EndOfAttack)
                && t.when.contains(&CompiledTiming::OnDeletion))
    });
    assert!(
        found,
        "the shared [End of Attack][On Deletion] clause must carry BOTH timings"
    );
}

#[test]
fn ad1_002_inherited_your_turn_dp_aura_present() {
    let runner = aldamon_runner();
    let compiled = runner.compiled_card(CARD_ID).unwrap();
    let found = compiled.effects.iter().any(|c| {
        matches!(
            c,
            CompiledClause::Declarative(CompiledDeclarativeClause::Aura { scope, active_when, .. })
                if *scope == CompiledScope::Inherited && active_when.is_some()
        )
    });
    assert!(
        found,
        "inherited [Your Turn] DP aura (scope: inherited, gated) must be present"
    );
}

// ════════════════════════════════════════════════════════════════════════════
// Section 2 — Behavioral: [When Digivolving] delete
// ════════════════════════════════════════════════════════════════════════════

#[test]
fn ad1_002_when_digivolving_deletes_opp_digimon_dp_le_self() {
    let mut runner = aldamon_runner();
    let aldamon = runner.place_on_field(0, CARD_ID, Some(0));
    let eligible = runner.place_on_field(1, "ELIGIBLE", Some(0));
    let too_big = runner.place_on_field(1, "TOO-BIG", Some(0));

    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(aldamon),
    );
    runner.game.drain_effect_queue();

    let pending = runner
        .pending_selection()
        .expect("[When Digivolving] delete target selection should install");
    // Eligible (7000 <= 8000) is a legal target; TooBig (10000) is not.
    assert!(pending
        .valid_action_ids
        .contains(&encode_attack(0, eligible.index as u16)));
    assert!(!pending
        .valid_action_ids
        .contains(&encode_attack(0, too_big.index as u16)));

    // Resolve the delete and confirm the eligible Digimon left the battle area.
    let before = runner.game.players[1].battle_area.len();
    runner
        .execute_action(0, encode_attack(0, eligible.index as u16))
        .expect("delete resolves");
    let _ = runner.auto_resolve();
    assert!(
        runner.game.players[1].battle_area.len() < before,
        "the eligible opponent Digimon should be deleted"
    );
}

// ════════════════════════════════════════════════════════════════════════════
// Section 3 — Behavioral: [EoA][OD] optional trash + conditional draw
// ════════════════════════════════════════════════════════════════════════════

/// Declining the optional trash is legal (PASS available), and no Draw 2 fires.
#[test]
fn ad1_002_eoa_od_optional_trash_can_be_declined() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("AD1-002 YAML loads")
        .add_card({
            let mut c = make_test_card("HYBRID-CARD", "HybridCard");
            c.card_kind = CardKind::Digimon;
            c.traits = vec!["Hybrid".to_string()];
            c.dp = Some(3000);
            c
        })
        .add_card(make_test_card("DECK-FILLER", "DeckFiller"))
        .hand(0, &["HYBRID-CARD"])
        .deck(0, &["DECK-FILLER", "DECK-FILLER", "DECK-FILLER"])
        .memory(10)
        .start();

    let aldamon = runner.place_on_field(0, CARD_ID, Some(0));
    let hand_before = runner.hand_size(0);

    runner
        .game
        .enqueue_triggered(EffectTiming::OnDeletion, TriggerSource::Permanent(aldamon));
    runner.game.drain_effect_queue();

    // The first prompt is the optional trash — it must expose a PASS / decline.
    let pending = runner
        .pending_selection()
        .expect("[EoA][OD] optional trash prompt should install");
    assert!(
        pending.is_optional || pending.valid_action_ids.contains(&PASS),
        "the trash selection must be declinable"
    );

    // Explicitly DECLINE the trash (PASS), then auto-resolve any later optional
    // step (the Tamer free-play, which has no eligible target here).
    runner
        .execute_action(0, PASS)
        .expect("declining the optional trash resolves");
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.hand_size(0),
        hand_before,
        "declining the trash should not draw (no Draw 2) and not discard"
    );
}

/// Positive path: trashing a [Hybrid] card draws 2 (hand net +1: -1 trashed, +2 drawn).
#[test]
fn ad1_002_eoa_od_trash_hybrid_draws_two() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("AD1-002 YAML loads")
        .add_card({
            let mut c = make_test_card("HYBRID-CARD", "HybridCard");
            c.card_kind = CardKind::Digimon;
            c.traits = vec!["Hybrid".to_string()];
            c.dp = Some(3000);
            c
        })
        .add_card(make_test_card("DRAW-A", "DrawA"))
        .add_card(make_test_card("DRAW-B", "DrawB"))
        .hand(0, &["HYBRID-CARD"])
        .deck(0, &["DRAW-A", "DRAW-B"])
        .memory(10)
        .start();

    let aldamon = runner.place_on_field(0, CARD_ID, Some(0));
    let hand_before = runner.hand_size(0); // 1 (the Hybrid card)
    let trash_before = runner.trash_size(0);

    runner
        .game
        .enqueue_triggered(EffectTiming::OnDeletion, TriggerSource::Permanent(aldamon));
    runner.game.drain_effect_queue();

    // Accept the optional trash by selecting the Hybrid card (first valid id),
    // then auto-resolve the remaining steps (draw + optional play decline).
    let pending = runner.pending_selection().expect("trash prompt installs");
    let pick = *pending
        .valid_action_ids
        .iter()
        .find(|&&id| id != PASS)
        .expect("the Hybrid card must be a selectable trash target");
    runner.execute_action(0, pick).expect("trash resolves");
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.trash_size(0),
        trash_before + 1,
        "the chosen Hybrid card should be trashed"
    );
    // Net hand: -1 (trashed Hybrid) + 2 (Draw 2) = +1.
    assert_eq!(
        runner.hand_size(0),
        hand_before - 1 + 2,
        "trashing should trigger Draw 2 (net hand +1)"
    );
}

// ════════════════════════════════════════════════════════════════════════════
// Section 4 — [EoA][OD] Tamer pick legality: only Tamers that can be played
// ════════════════════════════════════════════════════════════════════════════
//
// DCGO AD1_002.cs:158-166 CanSelectPlayCondition = `IsTamer && (Red || Blue ||
// Green) && HasInheritedEffect && CanPlayAsNewPermanent(cardSource, payCost:
// false, activateClass)`, shared by the hand and the trash branch (:168-169).
// A Tamer is offered only if the effect can place it: a free battle-area
// slot, no "can't play Tamers by effects" lock, and — for the trash copy — no
// "can't play cards from the trash" lock. CR 4.3 1-3-11-1: a card can be
// declared for play only if it is able to be played; 1-3-2: an impossible
// action is not carried out.

/// A red Tamer with an inherited effect — the printed target.
fn red_tamer_with_inherited(id: &str) -> CardData {
    let mut card = tamer(id, id);
    card.colors = vec![digimon_engine::enums::CardColor::Red];
    card.play_cost = 2;
    card.level = None;
    card.dp = None;
    card.inherited_text = "[Your Turn] This Digimon gets +1000 DP.".to_string();
    card
}

/// Aldamon on P0's field, `RED-TAMER` in hand and trash (per flags).
fn aldamon_with_tamers(in_hand: bool, in_trash: bool) -> (DebugRunner, digimon_engine::permanent::PermanentHandle) {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("AD1-002 YAML loads")
        .add_card(red_tamer_with_inherited("RED-TAMER"))
        .add_card(make_test_card("DECK-FILLER", "DeckFiller"))
        .deck(0, &["DECK-FILLER"; 5])
        .deck(1, &["DECK-FILLER"; 5])
        .memory(10)
        .start();
    let aldamon = runner.place_on_field(0, CARD_ID, Some(0));
    if in_hand {
        runner.add_to_hand(0, "RED-TAMER");
    }
    if in_trash {
        runner.inject_trash(0, "RED-TAMER");
    }
    (runner, aldamon)
}

/// Fire [On Deletion], skip the optional trash if it is offered, and return
/// the union-pick candidates as sorted "zone:card_id" strings (empty when no
/// union pick installs).
fn tamer_union_offered(runner: &mut DebugRunner, aldamon: digimon_engine::permanent::PermanentHandle) -> Vec<String> {
    use digimon_engine::owned_card_choice::CardChoiceOperation;
    runner
        .game
        .enqueue_triggered(EffectTiming::OnDeletion, TriggerSource::Permanent(aldamon));
    runner.game.drain_effect_queue();
    if runner.pending_kind() == Some(SelectionKind::Hand) {
        runner.execute_action(0, PASS).expect("decline the optional trash");
    }
    let Some(pending) = runner.pending_selection() else {
        return Vec::new();
    };
    if !matches!(pending.kind, SelectionKind::UnionZone { .. }) {
        return Vec::new();
    }
    let owner = pending.card_choice.as_ref().expect("union pick owns a card-choice page");
    let mut out: Vec<String> = owner
        .view(&runner.game, pending.selecting_player)
        .rows
        .into_iter()
        .filter(|row| matches!(row.operation, CardChoiceOperation::CommitCard))
        .map(|row| format!("{:?}:{}", row.zone.expect("zone"), row.card_id.unwrap_or_default()))
        .collect();
    out.sort();
    out
}

fn add_player_lock(runner: &mut DebugRunner, modifier: digimon_engine::enums::ModifierType) {
    use digimon_engine::enums::Expiry;
    use digimon_engine::modifiers::PlayerModifierEntry;
    runner.game.modifiers.add_player_modifier(
        0,
        PlayerModifierEntry::simple(modifier, 0, Expiry::Permanent, None, 1),
    );
}

/// Regression guard: hand and trash copies are both offered.
#[test]
fn ad1_002_eoa_od_offers_tamer_from_hand_and_trash() {
    let (mut runner, aldamon) = aldamon_with_tamers(true, true);
    assert_eq!(
        tamer_union_offered(&mut runner, aldamon),
        vec!["Hand:RED-TAMER".to_string(), "Trash:RED-TAMER".to_string()]
    );
}

/// Aldamon fills the only slot: neither copy can be placed, none is offered.
#[test]
fn ad1_002_eoa_od_tamer_not_offered_when_battle_area_full() {
    let (mut runner, aldamon) = aldamon_with_tamers(true, true);
    runner.game.rules.field_slots = runner.game.players[0].battle_area.len() as u8;
    let offered = tamer_union_offered(&mut runner, aldamon);
    assert!(offered.is_empty(), "no free slot → nothing can be chosen (got {offered:?})");
}

/// "You can't play Tamers by effects": neither copy is offered.
#[test]
fn ad1_002_eoa_od_tamer_not_offered_under_tamer_effect_play_prohibition() {
    let (mut runner, aldamon) = aldamon_with_tamers(true, true);
    add_player_lock(&mut runner, digimon_engine::enums::ModifierType::CannotPlayTamerByEffect);
    let offered = tamer_union_offered(&mut runner, aldamon);
    assert!(offered.is_empty(), "Tamer effect plays are locked (got {offered:?})");
}

/// "You can't play cards from the trash": the trash copy is dropped, the hand
/// copy is still offered.
#[test]
fn ad1_002_eoa_od_trash_tamer_not_offered_under_cannot_play_from_trash() {
    let (mut runner, aldamon) = aldamon_with_tamers(true, true);
    add_player_lock(&mut runner, digimon_engine::enums::ModifierType::CannotPlayFromTrash);
    assert_eq!(
        tamer_union_offered(&mut runner, aldamon),
        vec!["Hand:RED-TAMER".to_string()]
    );
}
