//! BT26-025 Liollmon — Digimon Lv.3, Yellow, DP 1000, Cost 3.
//! Traits: Holy Beast, Glowing Dawn, BEATBREAK.
//! Digivolve: Yellow Lv.2 / cost 0; [Digivolve] Lv.2 w/[Glowing Dawn]: cost 0.
//!
//! [When Moving] [On Play] By placing your top security card face down under
//! any of your [Glowing Dawn] trait Tamers, <Recovery +1>.
//! Inherited: [When Attacking] [Once Per Turn] You may add your top security
//! card to the hand. Then, if you have 0 security cards, <Recovery +1>.
//!
//! DCGO: BT26/Yellow/BT26_025.cs. Interaction: the placement removes a
//! security card BY AN EFFECT, so BT26-089 Kyo Sawashiro (a [Glowing Dawn]
//! Tamer) reacts to it with his "if removed from by effects" branch.

use super::support::*;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::{CardColor, EffectTiming, GamePhase, ModifierType};

const CARD_ID: &str = "BT26-025";

fn setup() -> DebugRunner {
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT26-025")
        .dsl_card("BT26-089")
        .expect("Kyo")
        .add_card(filler("FILLER"))
        .add_card(tamer("S1", "S1", CardColor::Red, &[]))
        .add_card(tamer("S2", "S2", CardColor::Red, &[]))
        .add_card(tamer("S3", "S3", CardColor::Red, &[]))
        .add_card(tamer(
            "GDT",
            "GD Tamer",
            CardColor::Yellow,
            &["Glowing Dawn", "BEATBREAK"],
        ))
        .add_card(tamer("PLAINT", "Plain Tamer", CardColor::Yellow, &[]))
        .add_card(digimon("HOST", "Host", CardColor::Yellow, 4, 4, &[]))
        .add_card(digimon(
            "GD2",
            "GD2",
            CardColor::Green,
            2,
            0,
            &["Glowing Dawn"],
        ))
        .add_card(digimon("OPP", "Opp", CardColor::Red, 4, 5, &[]))
        .add_card(digimon("DECKTOP", "DeckTop", CardColor::Red, 3, 3, &[]))
        .deck(0, &["FILLER", "FILLER", "DECKTOP"])
        .deck(1, &["FILLER"; 6])
        .security(0, &["S1", "S2", "S3"])
        .security(1, &["S1", "S1", "S1"])
        .memory(3)
        .start();
    r.set_first_player(0);
    r
}

fn top_security(r: &DebugRunner) -> String {
    security_ids(r, 0).last().cloned().expect("security")
}

// ─── Alt digivolution ───────────────────────────────────────────────────────

#[test]
fn bt26_025_has_glowing_dawn_lv2_alt_path() {
    let r = setup();
    let card = r.compiled_card(CARD_ID).unwrap();
    let dbg = format!("{:?}", card.alt_paths);
    assert!(dbg.contains("Glowing Dawn"), "{dbg}");
}

// ─── [On Play] / [When Moving] ──────────────────────────────────────────────

#[test]
fn bt26_025_on_play_places_top_security_face_down_and_recovers() {
    let mut r = setup();
    let gdt = r.place_on_field(0, "GDT", Some(0));
    let top = top_security(&r);
    let l = r.place_on_field(0, CARD_ID, Some(0));
    fire(&mut r, EffectTiming::OnPlay, l);
    let v = r.pending_selection_view().expect("optional Tamer pick");
    assert!(v.is_optional, "the 'by placing' cost may be declined");
    pick_first(&mut r, 0);
    let _ = r.auto_resolve();
    assert_eq!(source_ids(&r, gdt), vec![top], "top security placed");
    assert_eq!(face_down_flags(&r, gdt), vec![true], "face down");
    assert_eq!(r.security_count(0), 3, "3 - 1 + <Recovery +1>");
    assert_eq!(top_security(&r), "DECKTOP", "recovered from the deck top");
}

#[test]
fn bt26_025_on_play_declined_does_nothing() {
    let mut r = setup();
    let gdt = r.place_on_field(0, "GDT", Some(0));
    let l = r.place_on_field(0, CARD_ID, Some(0));
    fire(&mut r, EffectTiming::OnPlay, l);
    pass(&mut r, 0);
    let _ = r.auto_resolve();
    assert_eq!(sources(&r, gdt), 0);
    assert_eq!(security_ids(&r, 0), vec!["S1", "S2", "S3"]);
}

#[test]
fn bt26_025_on_play_needs_a_glowing_dawn_tamer() {
    let mut r = setup();
    let pt = r.place_on_field(0, "PLAINT", Some(0));
    let l = r.place_on_field(0, CARD_ID, Some(0));
    fire(&mut r, EffectTiming::OnPlay, l);
    let _ = r.auto_resolve();
    assert!(r.game.pending_selection.is_none());
    assert_eq!(sources(&r, pt), 0);
    assert_eq!(r.security_count(0), 3);
}

#[test]
fn bt26_025_on_play_needs_a_security_card() {
    let mut r = setup();
    let gdt = r.place_on_field(0, "GDT", Some(0));
    r.game.players[0].security.clear();
    let l = r.place_on_field(0, CARD_ID, Some(0));
    fire(&mut r, EffectTiming::OnPlay, l);
    let _ = r.auto_resolve();
    assert_eq!(sources(&r, gdt), 0);
    assert_eq!(r.security_count(0), 0, "no recovery without the cost");
}

#[test]
fn bt26_025_when_moving_from_breeding_fires() {
    let mut r = setup();
    let gdt = r.place_on_field(0, "GDT", Some(0));
    r.set_phase(GamePhase::Breeding);
    r.place_in_breeding(0, CARD_ID);
    assert!(r.move_from_breeding(0));
    let v = r
        .pending_selection_view()
        .expect("[When Moving] Tamer pick");
    assert!(v.is_optional);
    pick_first(&mut r, 0);
    let _ = r.auto_resolve();
    assert_eq!(sources(&r, gdt), 1);
    assert_eq!(r.security_count(0), 3);
}

#[test]
fn bt26_025_other_digimon_moving_does_not_fire() {
    let mut r = setup();
    let gdt = r.place_on_field(0, "GDT", Some(0));
    r.place_on_field(0, CARD_ID, Some(0));
    r.set_phase(GamePhase::Breeding);
    r.place_in_breeding(0, "HOST");
    assert!(r.move_from_breeding(0));
    let _ = r.auto_resolve();
    assert_eq!(sources(&r, gdt), 0);
}

#[test]
fn bt26_025_placement_is_an_effect_removal_for_kyo() {
    let mut r = setup();
    let kyo = r.place_on_field(0, "BT26-089", Some(0));
    let opp = r.place_on_field(1, "OPP", Some(0));
    let l = r.place_on_field(0, CARD_ID, Some(0));
    fire(&mut r, EffectTiming::OnPlay, l);
    pick_first(&mut r, 0); // Kyo is the only [Glowing Dawn] Tamer
    r.accept_optional_trigger()
        .expect("Kyo reacts to the effect removal");
    pick_first(&mut r, 0); // <Security A. -1> target
    let _ = r.auto_resolve();
    // Kyo: top security (placed by Liollmon) + deck top (Kyo's own placement).
    assert_eq!(sources(&r, kyo), 2);
    assert_eq!(face_down_flags(&r, kyo), vec![true, true]);
    assert_eq!(
        r.modifiers().sum(opp, ModifierType::SecurityAttackChange),
        -1
    );
    assert_eq!(r.security_count(0), 3, "Liollmon's <Recovery +1> still ran");
}

// ─── Inherited [When Attacking] [Once Per Turn] ─────────────────────────────

fn carrier(r: &mut DebugRunner) -> digimon_engine::permanent::PermanentHandle {
    r.place_stack(0, &[CARD_ID, "HOST"])
}

#[test]
fn bt26_025_inherited_may_add_top_security_to_hand() {
    let mut r = setup();
    let c = carrier(&mut r);
    let top = top_security(&r);
    fire(&mut r, EffectTiming::WhenAttacking, c);
    r.execute_branch(0).expect("add");
    let _ = r.auto_resolve();
    assert_eq!(hand_ids(&r, 0), vec![top]);
    assert_eq!(r.security_count(0), 2, "no recovery while security remains");
}

#[test]
fn bt26_025_inherited_recovers_when_security_hits_zero() {
    let mut r = setup();
    r.game.players[0].security.truncate(1);
    let c = carrier(&mut r);
    fire(&mut r, EffectTiming::WhenAttacking, c);
    r.execute_branch(0).expect("add");
    let _ = r.auto_resolve();
    assert_eq!(r.hand_size(0), 1);
    assert_eq!(security_ids(&r, 0), vec!["DECKTOP"], "<Recovery +1>");
}

#[test]
fn bt26_025_inherited_recovers_with_zero_security_without_a_prompt() {
    let mut r = setup();
    r.game.players[0].security.clear();
    let c = carrier(&mut r);
    fire(&mut r, EffectTiming::WhenAttacking, c);
    assert!(r.game.pending_selection.is_none(), "nothing to add");
    assert_eq!(security_ids(&r, 0), vec!["DECKTOP"]);
}

#[test]
fn bt26_025_inherited_declined_keeps_once_per_turn_available() {
    let mut r = setup();
    let c = carrier(&mut r);
    fire(&mut r, EffectTiming::WhenAttacking, c);
    r.execute_branch(1).expect("don't add");
    let _ = r.auto_resolve();
    assert_eq!(r.hand_size(0), 0);
    // Nothing happened ⇒ the [Once Per Turn] use is not consumed (DCGO RemoveUse).
    fire(&mut r, EffectTiming::WhenAttacking, c);
    assert!(
        r.game.pending_selection.is_some(),
        "offered again after a decline"
    );
}

#[test]
fn bt26_025_inherited_is_once_per_turn() {
    let mut r = setup();
    let c = carrier(&mut r);
    fire(&mut r, EffectTiming::WhenAttacking, c);
    r.execute_branch(0).expect("add");
    let _ = r.auto_resolve();
    fire(&mut r, EffectTiming::WhenAttacking, c);
    assert!(r.game.pending_selection.is_none(), "OPT spent");
    assert_eq!(r.hand_size(0), 1);
}

#[test]
fn bt26_025_face_effect_is_not_inherited() {
    let mut r = setup();
    let gdt = r.place_on_field(0, "GDT", Some(0));
    let c = carrier(&mut r);
    fire(&mut r, EffectTiming::OnPlay, c);
    let _ = r.auto_resolve();
    assert_eq!(sources(&r, gdt), 0);
}
