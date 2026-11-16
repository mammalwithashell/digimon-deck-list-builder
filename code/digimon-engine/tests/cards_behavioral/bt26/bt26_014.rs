//! BT26-014 Darumamon — Lv.5 Red/Yellow (Mutant / Shambala / TB).
//!
//! [On Play] [When Digivolving] Delete 1 of your opponent's Digimon with 7000
//! DP or less. [On Deletion] You may return 1 [Shambala] trait card from your
//! trash to the hand. Then, you may play 1 [TB] trait Digimon card with 6000
//! DP or less from your hand without paying the cost.
//! Assembly -2: Lv.4 or lower [TB] trait card.
//! Inherited: [On Deletion] You may play 1 [TB] trait Digimon card with 6000
//! DP or less from your hand without paying the cost.
//!
//! DCGO: BT26/Red/BT26_014.cs.

use super::support::*;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::{CardColor, EffectTiming};
use digimon_engine::replacement::ReplacementCause;

const CARD_ID: &str = "BT26-014";

fn setup() -> DebugRunner {
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT26-014")
        .add_card(filler("FILLER"))
        .add_card(digimon(
            "SHAMT",
            "ShamTrash",
            CardColor::Red,
            3,
            3,
            &["Shambala"],
        ))
        // Lv.4 TB → DP 4000 (eligible); Lv.6 TB → DP 6000 (eligible boundary);
        // Lv.7 TB → DP 7000 (too big).
        .add_card(digimon("TB4", "TB4", CardColor::Red, 4, 5, &["TB"]))
        .add_card(digimon("TB6", "TB6", CardColor::Red, 6, 12, &["TB"]))
        .add_card(digimon("TB7", "TB7", CardColor::Red, 7, 13, &["TB"]))
        .add_card(tamer("TBTAMER", "TB Tamer", CardColor::Red, &["TB"]))
        .add_card(digimon("PLAIN4", "Plain4", CardColor::Red, 4, 5, &[]))
        .add_card(digimon("HOST", "Host", CardColor::Red, 6, 12, &[]))
        .add_card(digimon("OPP7", "Opp7", CardColor::Blue, 7, 13, &[]))
        .add_card(digimon("OPP8", "Opp8", CardColor::Blue, 8, 14, &[]))
        .deck(0, &["FILLER"; 6])
        .deck(1, &["FILLER"; 6])
        .security(0, &["FILLER"; 3])
        .security(1, &["FILLER"; 3])
        .memory(3)
        .start();
    r.set_first_player(0);
    r
}

#[test]
fn bt26_014_alt_path_and_assembly() {
    let r = setup();
    let alt = format!("{:?}", r.compiled_card(CARD_ID).unwrap().alt_paths);
    assert!(
        alt.contains("Shambala") && alt.contains("level_eq: Some(4)"),
        "{alt}"
    );
    assert!(alt.contains("Assembly") && alt.contains("TB"), "{alt}");
}

#[test]
fn bt26_014_on_play_deletes_opp_digimon_with_7000_or_less() {
    let mut r = setup();
    r.place_on_field(1, "OPP7", Some(0));
    r.place_on_field(1, "OPP8", Some(0));
    let d = r.place_on_field(0, CARD_ID, None);
    fire(&mut r, EffectTiming::OnPlay, d);
    let v = r.pending_selection_view().expect("delete pick");
    assert!(!v.is_optional);
    assert_eq!(non_pass(&r).len(), 1, "only OPP7");
    pick_first(&mut r, 0);
    assert_eq!(field_ids(&r, 1), vec!["OPP8"]);
}

#[test]
fn bt26_014_when_digivolving_no_target_no_prompt() {
    let mut r = setup();
    r.place_on_field(1, "OPP8", Some(0));
    let d = r.place_stack(0, &["PLAIN4", CARD_ID]);
    fire(&mut r, EffectTiming::WhenDigivolving, d);
    assert!(r.game.pending_selection.is_none());
    assert_eq!(field_ids(&r, 1), vec!["OPP8"]);
}

#[test]
fn bt26_014_on_deletion_returns_shambala_then_plays_tb_free() {
    let mut r = setup();
    push_trash(&mut r, 0, "SHAMT");
    push_hand(&mut r, 0, "TB6");
    push_hand(&mut r, 0, "TB7");
    push_hand(&mut r, 0, "TBTAMER");
    let d = r.place_on_field(0, CARD_ID, Some(0));
    let mem = r.memory();
    r.game
        .delete_permanents_batch(vec![d], ReplacementCause::OpponentEffect);
    // (a) return a [Shambala] card — Darumamon itself is also eligible now.
    let v = r.pending_selection_view().expect("trash pick");
    assert!(v.is_optional);
    assert_eq!(non_pass(&r).len(), 2, "SHAMT + Darumamon itself");
    pick_trash(&mut r, 0, "SHAMT");
    assert!(hand_ids(&r, 0).contains(&"SHAMT".to_string()));
    // (b) play a [TB] Digimon with <= 6000 DP: only TB6 (not TB7, not the Tamer,
    // not SHAMT).
    let v = r.pending_selection_view().expect("hand pick");
    assert!(v.is_optional);
    assert_eq!(non_pass(&r).len(), 1, "{:?}", hand_ids(&r, 0));
    pick_hand(&mut r, 0, "TB6");
    let _ = r.auto_resolve();
    assert_eq!(field_ids(&r, 0), vec!["TB6"]);
    assert_eq!(r.memory(), mem, "without paying the cost");
}

#[test]
fn bt26_014_on_deletion_decline_return_still_offers_play() {
    let mut r = setup();
    push_trash(&mut r, 0, "SHAMT");
    push_hand(&mut r, 0, "TB4");
    let d = r.place_on_field(0, CARD_ID, Some(0));
    r.game
        .delete_permanents_batch(vec![d], ReplacementCause::OpponentEffect);
    pass(&mut r, 0);
    assert!(trash_ids(&r, 0).contains(&"SHAMT".to_string()));
    pick_hand(&mut r, 0, "TB4");
    let _ = r.auto_resolve();
    assert_eq!(field_ids(&r, 0), vec!["TB4"]);
}

#[test]
fn bt26_014_inherited_on_deletion_plays_tb_free() {
    let mut r = setup();
    push_hand(&mut r, 0, "TB4");
    push_hand(&mut r, 0, "PLAIN4");
    let c = r.place_stack(0, &[CARD_ID, "HOST"]);
    r.game
        .delete_permanents_batch(vec![c], ReplacementCause::OpponentEffect);
    let v = r.pending_selection_view().expect("inherited play pick");
    assert!(v.is_optional);
    assert_eq!(non_pass(&r).len(), 1);
    pick_hand(&mut r, 0, "TB4");
    let _ = r.auto_resolve();
    assert_eq!(field_ids(&r, 0), vec!["TB4"]);
}

#[test]
fn bt26_014_inherited_may_decline() {
    let mut r = setup();
    push_hand(&mut r, 0, "TB4");
    let c = r.place_stack(0, &[CARD_ID, "HOST"]);
    r.game
        .delete_permanents_batch(vec![c], ReplacementCause::OpponentEffect);
    pass(&mut r, 0);
    let _ = r.auto_resolve();
    assert!(field_ids(&r, 0).is_empty());
    assert_eq!(hand_ids(&r, 0), vec!["TB4"]);
}

// ─── Free-play pick legality (select-hand audit 2026-11-13) ──────────────────
//
// DCGO BT26_014.cs:132-136 ([On Deletion]) and :200-204 (inherited
// [On Deletion]) `CanSelectHandCardCondition`: `IsDigimon && EqualsTraits("TB")
// && HasDP && CardDP <= 6000 && CanPlayAsNewPermanent(card, payCost: false,
// activateClass)`. `CanPlayAsNewPermanent` (GameContextDeterminarion.cs:
// 419-432) refuses an Option-kind card (a DUAL's kinds include Option: it is
// digivolved or used, never played, CR 4.3 4-6-2, 7-1-1), and needs an empty
// battle-area frame and no play prohibition. A card that can't be played stays
// in hand (CR 4.3 7-1-2-3, 1-3-2). YAML: both picks add
// `can_play_from_effect_without_cost: true`.

/// A [TB] DUAL card whose Digimon face is level 4 / 5000 DP (no real [TB] DUAL
/// with 6000 DP or less exists yet; the pick must not offer one either way).
fn tb_dual(id: &str) -> digimon_engine::card_data::CardData {
    use digimon_engine::card_data::{DualCardData, DualDigimonFace, DualOptionFace};
    let mut c = digimon(id, id, CardColor::Red, 4, 0, &["TB"]);
    c.card_kind = digimon_engine::enums::CardKind::Dual;
    c.dp = Some(5000);
    c.dual = Some(DualCardData {
        digimon: DualDigimonFace {
            level: 4,
            dp: 5000,
            colors: vec![CardColor::Red],
            traits: vec!["TB".to_string()],
            evo_costs: vec![],
            effect_text: String::new(),
            inherited_text: String::new(),
            keywords: vec![],
        },
        option: DualOptionFace {
            use_cost: 3,
            colors: vec![CardColor::Red],
            effect_text: String::new(),
            security_text: String::new(),
            keywords: vec![],
        },
    });
    c
}

/// The non-PASS hand candidates of the pending pick, as sorted card ids; empty
/// when no hand pick is pending.
fn offered_hand_ids(r: &DebugRunner, p: u8) -> Vec<String> {
    use digimon_engine::action::space::{PASS, PLAY_HAND_START};
    use digimon_engine::owned_card_choice::{CardChoiceOperation, CardChoiceZone};
    use digimon_engine::selection::SelectionKind;
    let Some(view) = r.pending_selection_view() else {
        return Vec::new();
    };
    if !matches!(view.kind, SelectionKind::Hand) {
        return Vec::new();
    }
    let hand = &r.game.players[p as usize].hand;
    let id_at = |i: usize| hand.get(i).map(|c| c.card_id(&r.game.card_data).to_string());
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
            .filter_map(|a| a.checked_sub(PLAY_HAND_START))
            .filter_map(|i| id_at(i as usize))
            .collect(),
    };
    ids.sort();
    ids
}

/// `setup()` plus the [TB] DUAL fixture.
fn audit_setup() -> DebugRunner {
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT26-014")
        .add_card(filler("FILLER"))
        .add_card(digimon("TB4", "TB4", CardColor::Red, 4, 5, &["TB"]))
        .add_card(digimon("HOST", "Host", CardColor::Red, 6, 12, &[]))
        .add_card(tb_dual("TB-DUAL"))
        .deck(0, &["FILLER"; 6])
        .deck(1, &["FILLER"; 6])
        .security(0, &["FILLER"; 3])
        .security(1, &["FILLER"; 3])
        .memory(3)
        .start();
    r.set_first_player(0);
    r
}

fn prohibit_digimon_effect_play(r: &mut DebugRunner) {
    use digimon_engine::enums::{Expiry, ModifierType};
    use digimon_engine::modifiers::PlayerModifierEntry;
    r.game.modifiers.add_player_modifier(
        0,
        PlayerModifierEntry::simple(ModifierType::CannotPlayDigimonByEffect, 0, Expiry::Permanent, None, 1),
    );
}

/// The battle area is full once the deleted permanent has left.
fn fill_after_deletion(r: &mut DebugRunner) {
    r.place_on_field(0, "FILLER", None);
    r.game.rules.field_slots = r.game.players[0].battle_area.len() as u8 - 1;
}

/// Delete a Darumamon with `hand` in hand after `setup`; decline the
/// [Shambala] return (Darumamon itself is a candidate) and return the play
/// candidates.
fn on_deletion_offered(hand: &[&str], setup: impl FnOnce(&mut DebugRunner)) -> Vec<String> {
    let mut r = audit_setup();
    for id in hand {
        push_hand(&mut r, 0, id);
    }
    let d = r.place_on_field(0, CARD_ID, Some(0));
    setup(&mut r);
    r.game
        .delete_permanents_batch(vec![d], ReplacementCause::OpponentEffect);
    pass(&mut r, 0);
    offered_hand_ids(&r, 0)
}

/// Delete a Digimon carrying Darumamon as a source with `hand` in hand after
/// `setup`; return the inherited play candidates.
fn inherited_offered(hand: &[&str], setup: impl FnOnce(&mut DebugRunner)) -> Vec<String> {
    let mut r = audit_setup();
    for id in hand {
        push_hand(&mut r, 0, id);
    }
    let c = r.place_stack(0, &[CARD_ID, "HOST"]);
    setup(&mut r);
    r.game
        .delete_permanents_batch(vec![c], ReplacementCause::OpponentEffect);
    offered_hand_ids(&r, 0)
}

#[test]
fn bt26_014_on_deletion_pick_excludes_tb_dual_card() {
    assert_eq!(on_deletion_offered(&["TB-DUAL", "TB4"], |_| {}), vec!["TB4".to_string()]);
}

#[test]
fn bt26_014_on_deletion_pick_excludes_digimon_under_effect_play_prohibition() {
    let offered = on_deletion_offered(&["TB4"], prohibit_digimon_effect_play);
    assert!(offered.is_empty(), "{offered:?}");
}

#[test]
fn bt26_014_on_deletion_pick_excludes_digimon_when_battle_area_full() {
    let offered = on_deletion_offered(&["TB4"], fill_after_deletion);
    assert!(offered.is_empty(), "{offered:?}");
}

#[test]
fn bt26_014_inherited_pick_excludes_tb_dual_card() {
    assert_eq!(inherited_offered(&["TB-DUAL", "TB4"], |_| {}), vec!["TB4".to_string()]);
}

#[test]
fn bt26_014_inherited_pick_excludes_digimon_under_effect_play_prohibition() {
    let offered = inherited_offered(&["TB4"], prohibit_digimon_effect_play);
    assert!(offered.is_empty(), "{offered:?}");
}

#[test]
fn bt26_014_inherited_pick_excludes_digimon_when_battle_area_full() {
    let offered = inherited_offered(&["TB4"], fill_after_deletion);
    assert!(offered.is_empty(), "{offered:?}");
}
