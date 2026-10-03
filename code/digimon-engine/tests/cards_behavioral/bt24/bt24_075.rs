//! BT24-075 SkullBaluchimon — Lv.5 Purple, Undead/X Antibody/Titan/TS.
//!
//! [On Play] [When Digivolving] By trashing 1 card in your hand, delete 1 each
//! of your opponent's level 3 and level 4 Digimon.
//! Inherited: [Your Turn] While this Digimon is [Titamon] or has the [Titan]
//! trait, it gains <Security A. +1>.
//!
//! DCGO: BT24/Purple/BT24_075.cs.

#[path = "../bt26/support.rs"]
mod support;

use digimon_dsl::compiled::{CompiledClause, CompiledScope, CompiledTiming};
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::{CardColor, GamePhase, Keyword, PlaySource};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::replacement::ReplacementCause;
use support::*;

const CARD_ID: &str = "BT24-075";

fn setup() -> DebugRunner {
    let mut titamon = digimon("TITAMON", "Titamon", CardColor::Purple, 6, 13, &["Shaman"]);
    titamon.dp = Some(13000);
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT24-075 in embedded DSL pack")
        .add_card(filler("FILLER"))
        .add_card(digimon("TITAN3", "Titan Three", CardColor::Purple, 3, 3, &["Titan"]))
        .add_card(digimon("SHAMAN3", "Shaman Three", CardColor::Yellow, 3, 3, &["Shaman"]))
        .add_card(digimon("BEAST3", "Beast Three", CardColor::Yellow, 3, 3, &["Beast"]))
        .add_card(digimon("DEMON4Y", "Demon Four", CardColor::Yellow, 4, 5, &["Demon"]))
        .add_card(digimon("TS4Y", "TS Four", CardColor::Yellow, 4, 5, &["TS"]))
        .add_card(digimon("TITAN4Y", "Titan Four", CardColor::Yellow, 4, 5, &["Titan"]))
        .add_card(digimon("PLAIN4Y", "Plain Four", CardColor::Yellow, 4, 5, &[]))
        .add_card(digimon("PURPLE4", "Purple Four", CardColor::Purple, 4, 5, &[]))
        .add_card(digimon("BEAST4", "Beast Four", CardColor::Purple, 4, 5, &["Beast"]))
        .add_card(digimon("TITAN5", "Titan Five", CardColor::Purple, 5, 7, &["Titan"]))
        .add_card(digimon("BEAST6", "Beast Six", CardColor::Purple, 6, 9, &["Beast"]))
        .add_card(titamon)
        .add_card(digimon("OPP3", "Opp Three", CardColor::Red, 3, 3, &[]))
        .add_card(digimon("OPP3B", "Opp Three B", CardColor::Red, 3, 3, &[]))
        .add_card(digimon("OPP4", "Opp Four", CardColor::Red, 4, 5, &[]))
        .add_card(digimon("OPP5", "Opp Five", CardColor::Red, 5, 7, &[]))
        .deck(0, &["FILLER"; 10])
        .deck(1, &["FILLER"; 10])
        .security(1, &["FILLER"; 4])
        .memory(10)
        .start();
    r.skip_mulligan();
    r.set_first_player(0);
    r.game.turn_player_idx = 0;
    r.game.current_phase = GamePhase::Main;
    r
}

fn find(r: &DebugRunner, p: u8, id: &str) -> PermanentHandle {
    let index = r.game.players[p as usize]
        .battle_area
        .iter()
        .position(|perm| perm.top_card().card_id(&r.game.card_data) == id)
        .unwrap_or_else(|| panic!("{id} not on player {p}'s field"));
    PermanentHandle { player: p, index: index as u8 }
}

fn play_card(r: &mut DebugRunner) -> PermanentHandle {
    let idx = r.add_to_hand(0, CARD_ID);
    r.play(0, idx).expect("BT24-075 plays");
    find(r, 0, CARD_ID)
}

fn digivolve_onto(r: &mut DebugRunner, base: PermanentHandle) -> bool {
    let idx = r.add_to_hand(0, CARD_ID);
    r.game.digivolve_from_hand(0, idx, base.index as usize, PlaySource::ByDigivolve)
}

// ─── Structural ──────────────────────────────────────────────────────────────

#[test]
fn bt24_075_metadata_and_clause_shape() {
    let r = setup();
    let card = r.compiled_card(CARD_ID).expect("compiled");
    assert_eq!(card.name, "SkullBaluchimon");
    assert_eq!(card.level, Some(5));
    assert_eq!(card.cost, Some(6));
    assert_eq!(card.dp, Some(7000));
    for t in ["Undead", "X Antibody", "Titan", "TS"] {
        assert!(card.traits.iter().any(|x| x == t), "trait {t}");
    }
    let triggered: Vec<_> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();
    assert_eq!(triggered.len(), 1);
    let shared = triggered
        .iter()
        .find(|t| t.when.contains(&CompiledTiming::OnPlay))
        .expect("OP/WD clause");
    assert_eq!(
        shared.when,
        vec![CompiledTiming::OnPlay, CompiledTiming::WhenDigivolving]
    );
    assert!(!shared.once_per_turn, "no [Once Per Turn] printed");
    assert!(!shared.optional, "the cost pick is the decline surface");
    let inherited_sa = card.effects.iter().any(|c| {
        matches!(c, CompiledClause::Declarative(_))
            && format!("{c:?}").contains("SecurityAttackPlus")
            && format!("{c:?}").contains("Inherited")
    });
    assert!(inherited_sa, "inherited <Security A. +1>");
    let _ = CompiledScope::Inherited;
}

// ─── Digivolution requirements ──────────────────────────────────────────────

#[test]
fn bt24_075_digivolves_from_demon_or_ts_lv4_and_purple_lv4_for_3() {
    for base_id in ["DEMON4Y", "TS4Y", "PURPLE4"] {
        let mut r = setup();
        let base = r.place_on_field(0, base_id, Some(0));
        assert!(digivolve_onto(&mut r, base), "{base_id}");
        assert_eq!(top_id(&r, base), CARD_ID);
        assert_eq!(r.memory(), 10 - 3, "{base_id}");
    }
}

#[test]
fn bt24_075_cannot_digivolve_from_unrelated_lv4() {
    let mut r = setup();
    let base = r.place_on_field(0, "PLAIN4Y", Some(0));
    assert!(!digivolve_onto(&mut r, base));
    assert_eq!(top_id(&r, base), "PLAIN4Y");
}

// ─── Inherited: [Your Turn] while [Titamon] / [Titan] → <Security A. +1> ────

#[test]
fn bt24_075_inherited_security_attack_plus_on_titan_carrier_your_turn() {
    let mut r = setup();
    let host = r.place_stack(0, &[CARD_ID, "TITAN5"]);
    r.game.tick_declarative_effects();
    assert_eq!(r.game.security_attack_keyword_bonus(host), 1);
    // Driven through a real attack: 2 security checks.
    r.attack_player(host, 1, false);
    let _ = r.auto_resolve();
    assert_eq!(r.security_count(1), 4 - 2, "<Security A. +1> → 2 checks");
}

#[test]
fn bt24_075_inherited_security_attack_plus_on_titamon_carrier() {
    let mut r = setup();
    let host = r.place_stack(0, &[CARD_ID, "TITAMON"]);
    r.game.tick_declarative_effects();
    assert_eq!(r.game.security_attack_keyword_bonus(host), 1, "is [Titamon]");
}

#[test]
fn bt24_075_inherited_no_bonus_on_non_titan_carrier() {
    let mut r = setup();
    let host = r.place_stack(0, &[CARD_ID, "BEAST6"]);
    r.game.tick_declarative_effects();
    assert_eq!(r.game.security_attack_keyword_bonus(host), 0);
    r.attack_player(host, 1, false);
    let _ = r.auto_resolve();
    assert_eq!(r.security_count(1), 4 - 1, "single check");
}

#[test]
fn bt24_075_inherited_no_bonus_on_opponents_turn() {
    let mut r = setup();
    let host = r.place_stack(0, &[CARD_ID, "TITAN5"]);
    r.game.turn_player_idx = 1;
    r.game.tick_declarative_effects();
    assert_eq!(r.game.security_attack_keyword_bonus(host), 0, "[Your Turn] only");
}

#[test]
fn bt24_075_face_up_card_has_no_printed_security_attack() {
    // The <Security A. +1> is inherited-only; face-up it does nothing.
    let mut r = setup();
    let me = r.place_on_field(0, CARD_ID, Some(0));
    r.game.tick_declarative_effects();
    assert_eq!(r.game.security_attack_keyword_bonus(me), 0);
}

// ─── [On Play][When Digivolving] By trashing 1 card in your hand ─────────────

#[test]
fn bt24_075_decline_cost_does_nothing() {
    let mut r = setup();
    r.place_on_field(1, "OPP3", Some(0));
    push_hand(&mut r, 0, "FILLER");
    let base = r.place_on_field(0, "DEMON4Y", Some(0));
    assert!(digivolve_onto(&mut r, base));
    let v = r.pending_selection_view().expect("optional cost");
    assert!(v.is_optional, "'By trashing' — the cost is optional");
    pass(&mut r, 0);
    let _ = r.auto_resolve();
    assert!(r.pending_selection_view().is_none());
    assert!(hand_ids(&r, 0).contains(&"FILLER".to_string()), "nothing trashed");
    assert!(field_ids(&r, 1).contains(&"OPP3".to_string()));
}

#[test]
fn bt24_075_empty_hand_does_not_activate() {
    let mut r = setup();
    r.place_on_field(1, "OPP3", Some(0));
    let _me = play_card(&mut r);
    assert_eq!(r.hand_size(0), 0);
    assert!(r.pending_selection_view().is_none());
    assert!(field_ids(&r, 1).contains(&"OPP3".to_string()));
}

#[test]
fn bt24_075_when_digivolving_deletes_one_level3_and_one_level4() {
    let mut r = setup();
    let o3 = r.place_on_field(1, "OPP3", Some(0));
    r.place_on_field(1, "OPP3B", Some(0));
    let o4 = r.place_on_field(1, "OPP4", Some(0));
    r.place_on_field(1, "OPP5", Some(0));
    r.place_on_field(0, "TITAN3", Some(0));
    push_hand(&mut r, 0, "FILLER");
    let base = r.place_on_field(0, "DEMON4Y", Some(0));
    assert!(digivolve_onto(&mut r, base));
    let v = r.pending_selection_view().expect("optional cost");
    assert!(v.is_optional);
    pick_hand(&mut r, 0, "FILLER");
    let v = r.pending_selection_view().expect("level 3 pick");
    assert!(!v.is_optional, "mandatory once paid");
    assert_eq!(non_pass(&r).len(), 2, "both opponent level 3s, never own TITAN3");
    pick_side_field(&mut r, 0, o3);
    // Not deleted yet — both picks are collected first, then one deletion.
    assert!(field_ids(&r, 1).contains(&"OPP3".to_string()));
    let v = r.pending_selection_view().expect("level 4 pick");
    assert!(!v.is_optional);
    assert_eq!(non_pass(&r).len(), 1, "only OPP4");
    pick_side_field(&mut r, 0, o4);
    let _ = r.auto_resolve();
    let mut left = field_ids(&r, 1);
    left.sort();
    assert_eq!(left, vec!["OPP3B".to_string(), "OPP5".to_string()]);
    assert!(field_ids(&r, 0).contains(&"TITAN3".to_string()));
}

#[test]
fn bt24_075_on_play_with_only_level4_deletes_it() {
    let mut r = setup();
    let o4 = r.place_on_field(1, "OPP4", Some(0));
    r.place_on_field(1, "OPP5", Some(0));
    push_hand(&mut r, 0, "FILLER");
    let _me = play_card(&mut r);
    assert_eq!(r.memory(), 10 - 6);
    pick_hand(&mut r, 0, "FILLER");
    // No level 3 → that arm is skipped; straight to the level 4 pick.
    assert_eq!(non_pass(&r).len(), 1);
    pick_side_field(&mut r, 0, o4);
    let _ = r.auto_resolve();
    assert_eq!(field_ids(&r, 1), vec!["OPP5".to_string()]);
}

#[test]
fn bt24_075_on_play_with_only_level3_deletes_it() {
    let mut r = setup();
    let o3 = r.place_on_field(1, "OPP3", Some(0));
    r.place_on_field(1, "OPP5", Some(0));
    push_hand(&mut r, 0, "FILLER");
    let _me = play_card(&mut r);
    pick_hand(&mut r, 0, "FILLER");
    pick_side_field(&mut r, 0, o3);
    let _ = r.auto_resolve();
    assert!(r.pending_selection_view().is_none());
    assert_eq!(field_ids(&r, 1), vec!["OPP5".to_string()]);
}

#[test]
fn bt24_075_no_targets_still_pays_cost_silently() {
    let mut r = setup();
    r.place_on_field(1, "OPP5", Some(0));
    push_hand(&mut r, 0, "FILLER");
    let _me = play_card(&mut r);
    pick_hand(&mut r, 0, "FILLER");
    let _ = r.auto_resolve();
    assert!(r.pending_selection_view().is_none());
    assert_eq!(r.hand_size(0), 0);
    assert_eq!(field_ids(&r, 1), vec!["OPP5".to_string()]);
}


#[allow(dead_code)]
fn _unused(_: Keyword, _: ReplacementCause) {}
