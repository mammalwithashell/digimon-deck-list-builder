//! BT26-085 Giant Slayer — Digimon (no level), White, DP 14000, cost 12 (TS).
//!
//! <Collision> <Reboot> <Blocker> [On Play] Until your opponent's turn ends,
//! your opponent's effects can't reduce this Digimon's DP or trash its stacked
//! cards. [All Turns] When this Digimon would leave the battle area, by
//! digivolving it into [Chronomon: Destroy Mode] in the hand or trash without
//! paying the cost, it doesn't leave.
//! Assembly -5: 5 different-level cards w/[Chronomon] in text or w/[Shaman]
//! trait.
//!
//! DCGO: BT26/White/BT26_085.cs.

use super::support::*;
use digimon_engine::action::{
    build_action_mask,
    space::{PASS, PLAY_HAND_START, TRASH_EFFECT_START},
};
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::{CardColor, EffectTiming, Expiry, Keyword, ModifierType};
use digimon_engine::modifiers::ModifierEntry;
use digimon_engine::replacement::ReplacementCause;

const CARD_ID: &str = "BT26-085";

const DESTROY_MODE: &str = "BT26-060";

fn setup() -> DebugRunner {
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT26-085")
        .dsl_card(DESTROY_MODE)
        .expect("production Chronomon: Destroy Mode")
        .add_card(filler("FILLER"))
        .add_card(digimon("SH2", "Sh Two", CardColor::Red, 2, 0, &["Shaman"]))
        .add_card(digimon("CH3", "Chronomon Three", CardColor::Red, 3, 3, &[]))
        .add_card(digimon("SH4", "Sh Four", CardColor::Red, 4, 5, &["Shaman"]))
        .add_card(digimon(
            "SH4B",
            "Sh Four B",
            CardColor::Red,
            4,
            5,
            &["Shaman"],
        ))
        .add_card(digimon("CH5", "Chronomon Five", CardColor::Red, 5, 7, &[]))
        .add_card(digimon("SH6", "Sh Six", CardColor::Red, 6, 12, &["Shaman"]))
        .deck(0, &["FILLER"; 6])
        .deck(1, &["FILLER"; 6])
        .security(0, &["FILLER"; 3])
        .security(1, &["FILLER"; 3])
        .memory(10)
        .start();
    r.set_first_player(0);
    r
}

fn assert_choice_mask(r: &DebugRunner, expected: &[u16], optional: bool) {
    let selection = r.game.pending_selection.as_ref().expect("player choice");
    assert_eq!(selection.selecting_player, 0);
    assert_eq!(selection.is_optional, optional);
    let actual: Vec<_> = selection
        .valid_action_ids
        .iter()
        .copied()
        .filter(|a| *a != PASS)
        .collect();
    assert_eq!(actual, expected, "{}", selection.prompt);
    let mask = build_action_mask(&r.game, 0);
    for (a, enabled) in mask.iter().enumerate() {
        assert_eq!(
            *enabled,
            if expected.contains(&(a as u16)) || (optional && a == PASS as usize) {
                1.0
            } else {
                0.0
            },
            "mask at {a}"
        );
    }
    assert!(build_action_mask(&r.game, 1).iter().all(|v| *v == 0.0));
}

fn destroy_actions(r:&DebugRunner)->[u16;2] {
    use digimon_engine::owned_card_choice::CardChoiceZone;
    assert_eq!(physical_menu::commits(&r.game).len(),2,"exactly the named Destroy Mode in hand and trash are legal");
    let hand=r.game.player(0).hand.iter().position(|card|card.card_id(&r.game.card_data)==DESTROY_MODE).unwrap();
    let trash=r.game.player(0).trash.iter().position(|card|card.card_id(&r.game.card_data)==DESTROY_MODE).unwrap();
    [physical_menu::zone_action(&r.game,0,hand,CardChoiceZone::Hand),physical_menu::zone_action(&r.game,0,trash,CardChoiceZone::Trash)]
}

fn accept_replacement(r: &mut DebugRunner) {
    let ids: Vec<_> = r
        .pending_selection_view()
        .expect("replacement offer")
        .valid_action_ids
        .into_iter()
        .filter(|a| *a != PASS)
        .collect();
    assert_eq!(ids.len(), 1);
    assert_choice_mask(r, &ids, true);
    r.execute_action(0, ids[0]).unwrap();
}

#[test]
fn bt26_085_keywords_and_assembly_shape() {
    let mut r = setup();
    let h = r.place_on_field(0, CARD_ID, Some(0));
    for kw in [Keyword::Collision, Keyword::Reboot, Keyword::Blocker] {
        assert!(r.game.has_keyword(h, kw), "missing {kw:?}");
    }
    let alt = format!("{:?}", r.compiled_card(CARD_ID).unwrap().alt_paths);
    assert!(alt.contains("Assembly") && alt.contains("Level"), "{alt}");
}

#[test]
fn bt26_085_assembly_needs_five_distinct_levels() {
    let mut r = setup();
    r.skip_mulligan();
    push_hand(&mut r, 0, CARD_ID);
    for id in ["SH2", "CH3", "SH4", "SH4B", "CH5"] {
        push_trash(&mut r, 0, id);
    }
    // Only 4 distinct levels (2,3,4,5) → no assembly reduction; cost 12 > 10
    // memory... the play itself is still declarable, but no assembly gate.
    let idx = r.game.players[0].hand.len() - 1;
    r.game.decode_action(PLAY_HAND_START + idx as u16, 0);
    let gate = r
        .game
        .pending_selection
        .as_ref()
        .map(|p| p.prompt.contains("Assembly"))
        .unwrap_or(false);
    assert!(
        !gate,
        "4 distinct levels cannot pay a 5-different-level Assembly"
    );
}

#[test]
fn bt26_085_assembly_picks_five_different_levels_and_reduces_cost_by_five() {
    let mut r = setup();
    r.skip_mulligan();
    push_hand(&mut r, 0, CARD_ID);
    for id in ["SH2", "CH3", "SH4", "SH4B", "CH5", "SH6"] {
        push_trash(&mut r, 0, id);
    }
    let mem = r.memory();
    let idx = r.game.players[0].hand.len() - 1;
    r.game.decode_action(PLAY_HAND_START + idx as u16, 0);
    assert!(r.game.pending_selection.is_some(), "assembly gate");
    // Pick SH2, CH3, SH4 (trash 0..2).
    for t in 0..3u16 {
        r.game.decode_action(TRASH_EFFECT_START + t, 0);
    }
    // SH4B (index 3) shares level 4 with SH4 → not offered.
    let v = r.pending_selection_view().expect("still selecting");
    assert!(
        !v.valid_action_ids.contains(&(TRASH_EFFECT_START + 3)),
        "same level as an already-picked card"
    );
    r.game.decode_action(TRASH_EFFECT_START + 4, 0); // CH5
    r.game.decode_action(TRASH_EFFECT_START + 5, 0); // SH6
    let _ = r.auto_resolve();
    let field = field_ids(&r, 0);
    assert_eq!(field, vec![CARD_ID.to_string()]);
    assert_eq!(
        sources(
            &r,
            digimon_engine::permanent::PermanentHandle {
                player: 0,
                index: 0
            }
        ),
        5
    );
    assert_eq!(r.memory(), mem - 7, "12 - 5");
    assert_eq!(trash_ids(&r, 0), vec!["SH4B"]);
}

#[test]
fn bt26_085_on_play_grants_opponent_scoped_dp_and_stack_trash_immunity() {
    let mut r = setup();
    let h = r.place_on_field(0, CARD_ID, None);
    fire(&mut r, EffectTiming::OnPlay, h);
    assert!(r.modifiers().has(h, ModifierType::ImmuneFromDPMinus));
    assert!(r.modifiers().has(h, ModifierType::ImmuneFromStackTrashing));
    assert!(r.modifiers().has(h, ModifierType::CannotBeDeDigivolved));
    assert!(
        !r.modifiers().has(h, ModifierType::CannotBeReturnedToHand),
        "returning to hand is not covered"
    );
    // Lasts until the end of the opponent's turn.
    r.end_turn();
    assert!(r.modifiers().has(h, ModifierType::ImmuneFromStackTrashing));
    r.end_turn();
    assert!(!r.modifiers().has(h, ModifierType::ImmuneFromStackTrashing));
}

#[test]
fn bt26_085_would_leave_chooses_production_destroy_mode_from_hand_or_trash_free() {
    for choose_trash in [false, true] {
        let mut r = setup();
        let h = r.place_on_field(0, CARD_ID, Some(0));
        push_hand(&mut r, 0, DESTROY_MODE);
        push_trash(&mut r, 0, DESTROY_MODE);
        push_hand(&mut r, 0, "SH6");
        push_trash(&mut r, 0, "SH6");
        let mem = r.memory();
        r.game
            .delete_permanent_with_cause(h, ReplacementCause::OpponentEffect);
        accept_replacement(&mut r);
        // Both printed zones remain visible, and non-Destroy Mode cards are excluded.
        let choices=destroy_actions(&r);
        assert_choice_mask(&r, &choices, false);
        r.execute_action(
            0,
            choices[usize::from(choose_trash)],
        )
        .unwrap();
        assert_eq!(
            field_ids(&r, 0),
            vec![DESTROY_MODE],
            "production card replaces the leave"
        );
        assert_eq!(sources(&r, h), 1);
        assert_eq!(r.memory(), mem, "without paying the cost");
        assert_eq!(
            hand_ids(&r, 0)
                .iter()
                .filter(|id| id.as_str() == DESTROY_MODE)
                .count(),
            usize::from(choose_trash)
        );
        assert_eq!(
            trash_ids(&r, 0)
                .iter()
                .filter(|id| id.as_str() == DESTROY_MODE)
                .count(),
            usize::from(!choose_trash)
        );
        assert!(hand_ids(&r, 0).contains(&"SH6".to_string()));
        assert!(trash_ids(&r, 0).contains(&"SH6".to_string()));
        assert!(!trash_ids(&r, 0).contains(&CARD_ID.to_string()));
        assert!(r.game.pending_selection.is_none());
    }
}

#[test]
fn bt26_085_would_leave_without_destroy_mode_or_declined_leaves() {
    let mut r = setup();
    let h = r.place_on_field(0, CARD_ID, Some(0));
    r.game
        .delete_permanent_with_cause(h, ReplacementCause::OpponentEffect);
    assert!(
        r.game.pending_selection.is_none(),
        "no candidate, no prompt"
    );
    assert!(field_ids(&r, 0).is_empty());

    let mut r = setup();
    let h = r.place_on_field(0, CARD_ID, Some(0));
    push_hand(&mut r, 0, DESTROY_MODE);
    push_trash(&mut r, 0, DESTROY_MODE);
    r.game
        .delete_permanent_with_cause(h, ReplacementCause::OpponentEffect);
    let ids: Vec<_> = r
        .pending_selection_view()
        .unwrap()
        .valid_action_ids
        .into_iter()
        .filter(|a| *a != PASS)
        .collect();
    assert_choice_mask(&r, &ids, true);
    pass(&mut r, 0);
    let _ = r.auto_resolve();
    assert!(field_ids(&r, 0).is_empty());
    assert_eq!(hand_ids(&r, 0), vec![DESTROY_MODE]);
    assert!(trash_ids(&r, 0).contains(&DESTROY_MODE.to_string()));
    assert!(trash_ids(&r, 0).contains(&CARD_ID.to_string()));
}

#[test]
fn bt26_085_would_leave_is_not_once_per_turn() {
    let mut r = setup();
    let h = r.place_on_field(0, CARD_ID, Some(0));
    let other = r.place_on_field(0, CARD_ID, Some(0));
    push_trash(&mut r, 0, DESTROY_MODE);
    push_hand(&mut r, 0, DESTROY_MODE);
    r.game
        .delete_permanent_with_cause(h, ReplacementCause::OpponentEffect);
    drain_first(&mut r);
    let _ = r.auto_resolve();
    let other_idx = r.game.players[0]
        .battle_area
        .iter()
        .position(|p| p.top_card().card_id(&r.game.card_data) == CARD_ID)
        .unwrap();
    let _ = other;
    r.game.delete_permanent_with_cause(
        digimon_engine::permanent::PermanentHandle {
            player: 0,
            index: other_idx as u8,
        },
        ReplacementCause::OpponentEffect,
    );
    drain_first(&mut r);
    let _ = r.auto_resolve();
    assert_eq!(field_ids(&r, 0), vec![DESTROY_MODE, DESTROY_MODE]);
}

#[test]
fn bt26_085_rejected_digivolution_does_not_cancel_the_pending_leave() {
    let mut r = setup();
    let h = r.place_on_field(0, CARD_ID, Some(0));
    push_hand(&mut r, 0, DESTROY_MODE);
    push_trash(&mut r, 0, DESTROY_MODE);
    let memory = r.memory();
    r.game
        .delete_permanent_with_cause(h, ReplacementCause::OpponentEffect);
    accept_replacement(&mut r);
    let choices=destroy_actions(&r);
    assert_choice_mask(&r, &choices, false);
    // The selected payment is legal at offer time, but cannot commit after this gate.
    r.game.modifiers.add(
        h,
        ModifierEntry::simple(ModifierType::CannotDigivolve, 0, Expiry::Permanent, 0),
    );
    r.execute_action(0, choices[1]).unwrap();
    assert!(field_ids(&r, 0).is_empty());
    assert_eq!(hand_ids(&r, 0), vec![DESTROY_MODE]);
    assert!(trash_ids(&r, 0).contains(&DESTROY_MODE.to_string()));
    assert!(trash_ids(&r, 0).contains(&CARD_ID.to_string()));
    assert_eq!(r.memory(), memory);
    assert!(r.game.pending_selection.is_none());
}

#[path = "../../support/owned_card_choice.rs"]
mod physical_menu;
