//! EX12-003 Kapurimon — Digi-Egg, Lv.2, Black. Traits: Lesser, ME.
//!
//! Inherited: [All Turns] When any of your [ME] trait Digimon would leave the
//! battle area other than by your effects, 1 of them and any of your other
//! Digimon may DNA digivolve into an [ME] trait Digimon card in the hand.
//!
//! DCGO `EX12_003.cs`: `DNADigivolvePermanentsIntoHandOrTrashCard` asks the
//! hand card FIRST (`SelectHandEffect`), then the two roots (one of them a
//! leaving member; an unambiguous pair is taken silently). It never asks which
//! leaving Digimon "anchors" the digivolve. The engine's per-member replacement
//! window must not invent that prompt when every leaving member reaches the same
//! pairs (raw-20261113T025510Z#g17: Jupitermon BT26-033 deleting EX12-064 and
//! EX12-016 together asked a non-DCGO "Choose the impending deletion" pick).

use digimon_engine::action::space::ATTACK_START;
use digimon_engine::enums::CardColor;
use digimon_engine::replacement::ReplacementCause;
use digimon_engine::selection::SelectionKind;

use super::support::{
    digimon, field_contains, hand_contains, select_hand_card, source_count, trash_contains,
    top_card_id, DebugRunner,
};

const CARD_ID: &str = "EX12-003";
const RESULT: &str = "EX12-035"; // MetalGarurumon: DNA = Lv.5 blue/black + Lv.5 purple/yellow, [ME].

fn runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-003 YAML loads")
        .dsl_card(RESULT)
        .expect("EX12-035 YAML loads")
        // Lv.5 [ME] Digimon: black ones fill the first DNA slot, purple ones the second.
        .add_card(digimon("ME-BLK-A", CardColor::Black, 5, 7, 7000, &["ME"]))
        .add_card(digimon("ME-BLK-B", CardColor::Black, 5, 7, 7000, &["ME"]))
        .add_card(digimon("ME-PUR-X", CardColor::Purple, 5, 7, 7000, &["ME"]))
        .add_card(digimon("ME-PUR-Y", CardColor::Purple, 5, 7, 7000, &["ME"]))
        .add_card(digimon("ME-PUR-LEAVE", CardColor::Purple, 5, 7, 7000, &["ME"]))
        .hand(0, &[RESULT])
        .memory(10)
        .start()
}

fn leave_together(r: &mut DebugRunner, a: digimon_engine::permanent::PermanentHandle, b: digimon_engine::permanent::PermanentHandle) {
    let _ = r
        .game
        .delete_permanents_batch(vec![a, b], ReplacementCause::OpponentEffect);
    r.game.drain_effect_queue();
}

fn non_pass(r: &DebugRunner) -> Vec<u16> {
    let v = r.pending_selection_view().expect("pending selection");
    v.valid_action_ids
        .into_iter()
        .filter(|a| *a != digimon_engine::action::space::PASS)
        .collect()
}

/// Two leaving [ME] Digimon whose only reachable DNA pair is each other: the
/// subject is irrelevant, so DCGO's hand-card -> pair order is all the engine asks.
#[test]
fn ex12_003_two_leaving_members_with_one_reachable_pair_ask_no_subject_prompt() {
    let mut r = runner();
    // The leaving pair: black (carrier of Kapurimon) + purple. Nothing else on
    // the field, so {black, purple} is the only legal DNA pair for either.
    let black = r.place_stack(0, &[CARD_ID, "ME-BLK-A"]);
    let purple = r.place_on_field(0, "ME-PUR-LEAVE", Some(0));
    leave_together(&mut r, black, purple);

    let view = r.pending_selection_view().expect("the DNA digivolve prompt");
    assert_ne!(
        view.kind,
        SelectionKind::AnyField,
        "no 'which leaving Digimon' prompt when the pair is unambiguous: {view:?}"
    );
    assert!(
        !view.prompt.contains("impending deletion"),
        "non-DCGO subject prompt: {}",
        view.prompt
    );

    // The lone partner is taken, then the hand card is chosen.
    let partner = non_pass(&r);
    assert_eq!(partner.len(), 1, "exactly one reachable partner: {view:?}");
    r.execute_action(0, partner[0]).unwrap();
    select_hand_card(&mut r, 0, RESULT);
    let _ = r.auto_resolve();

    assert!(field_contains(&r, 0, RESULT), "MetalGarurumon DNA-digivolved in");
    assert!(!hand_contains(&r, 0, RESULT));
    assert!(
        !trash_contains(&r, 0, "ME-BLK-A") && !trash_contains(&r, 0, "ME-PUR-LEAVE"),
        "both leaving Digimon became DNA materials, neither was deleted"
    );
    assert_eq!(r.battle_area_size(0), 1);
    assert_eq!(top_card_id(&r, 0, 0), RESULT);
    assert!(source_count(&r, 0, 0) >= 2, "both materials sit under the result");
}

/// Two leaving members that reach DIFFERENT pairs ({A,X}/{A,Y} vs {B,X}/{B,Y}):
/// which one anchors the digivolve is a real choice, so the engine still asks
/// (the no-approximations policy: every choice stays a pending selection).
#[test]
fn ex12_003_two_leaving_members_reaching_different_pairs_still_ask_which_anchors() {
    let mut r = runner();
    let a = r.place_stack(0, &[CARD_ID, "ME-BLK-A"]);
    let b = r.place_on_field(0, "ME-BLK-B", Some(0));
    r.place_on_field(0, "ME-PUR-X", Some(0));
    r.place_on_field(0, "ME-PUR-Y", Some(0));
    leave_together(&mut r, a, b);

    let view = r.pending_selection_view().expect("subject choice");
    assert_eq!(view.kind, SelectionKind::AnyField, "{view:?}");
    assert!(view.prompt.contains("impending deletion"), "{}", view.prompt);
    assert_eq!(view.valid_action_ids.len(), 2, "either leaving Digimon may anchor");
    // Anchor on the first leaving member (A) and finish a DNA with a stayer.
    let first = view.valid_action_ids[0];
    r.execute_action(0, first).unwrap();
    let partner = non_pass(&r);
    assert!(!partner.is_empty(), "a stayer partner is offered");
    r.execute_action(0, partner[0]).unwrap();
    select_hand_card(&mut r, 0, RESULT);
    let _ = r.auto_resolve();
    assert!(field_contains(&r, 0, RESULT), "the DNA digivolve resolves");
    let _ = ATTACK_START;
}

/// A single leaving [ME] Digimon never had a subject prompt: unchanged.
#[test]
fn ex12_003_single_leaving_member_goes_straight_to_the_dna_partner_pick() {
    let mut r = runner();
    let a = r.place_stack(0, &[CARD_ID, "ME-BLK-A"]);
    r.place_on_field(0, "ME-PUR-X", Some(0));
    let _ = r
        .game
        .delete_permanents_batch(vec![a], ReplacementCause::OpponentEffect);
    r.game.drain_effect_queue();
    let view = r.pending_selection_view().expect("DNA partner pick");
    assert_ne!(view.kind, SelectionKind::AnyField, "{view:?}");
}

/// "other than by your effects": the owner's own effect does not open the window.
#[test]
fn ex12_003_does_not_trigger_when_the_owner_s_own_effect_removes_the_digimon() {
    let mut r = runner();
    let a = r.place_stack(0, &[CARD_ID, "ME-BLK-A"]);
    r.place_on_field(0, "ME-PUR-X", Some(0));
    let _ = r
        .game
        .delete_permanents_batch(vec![a], ReplacementCause::OwnEffect);
    r.game.drain_effect_queue();
    assert!(
        r.pending_selection_view().is_none(),
        "own-effect deletion does not offer the DNA digivolve"
    );
    assert!(trash_contains(&r, 0, "ME-BLK-A"));
}
