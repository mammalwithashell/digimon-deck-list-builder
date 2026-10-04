//! BT24-072 SkullGreymon — Lv.5 Purple, Undead/Titan/TS.
//!
//! [On Play] [When Digivolving] By trashing 1 card in your hand, until your
//! opponent's turn ends, 1 of your Digimon with the [Demon], [Shaman] or
//! [Titan] trait gains <Blocker> and <Retaliation>.
//! [On Deletion] You may play 1 level 4 or lower [Demon] or [Titan] trait
//! Digimon card from your trash without paying the cost.
//! Inherited: [Your Turn] While this Digimon is [Titamon] or has the [Titan]
//! trait, it gains <Security A. +1>.
//!
//! DCGO: BT24/Purple/BT24_072.cs.

#[path = "../bt26/support.rs"]
mod support;

use digimon_dsl::compiled::{CompiledClause, CompiledScope, CompiledTiming};
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::{CardColor, GamePhase, Keyword, PlaySource};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::replacement::ReplacementCause;
use support::*;

const CARD_ID: &str = "BT24-072";

fn setup() -> DebugRunner {
    let mut titamon = digimon("TITAMON", "Titamon", CardColor::Purple, 6, 13, &["Shaman"]);
    titamon.dp = Some(13000);
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT24-072 in embedded DSL pack")
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
    r.play(0, idx).expect("BT24-072 plays");
    find(r, 0, CARD_ID)
}

fn digivolve_onto(r: &mut DebugRunner, base: PermanentHandle) -> bool {
    let idx = r.add_to_hand(0, CARD_ID);
    r.game.digivolve_from_hand(0, idx, base.index as usize, PlaySource::ByDigivolve)
}

// ─── Structural ──────────────────────────────────────────────────────────────

#[test]
fn bt24_072_metadata_and_clause_shape() {
    let r = setup();
    let card = r.compiled_card(CARD_ID).expect("compiled");
    assert_eq!(card.name, "SkullGreymon");
    assert_eq!(card.level, Some(5));
    assert_eq!(card.cost, Some(7));
    assert_eq!(card.dp, Some(7000));
    for t in ["Undead", "Titan", "TS"] {
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
    assert_eq!(triggered.len(), 2);
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
fn bt24_072_digivolves_from_demon_or_ts_lv4_and_purple_lv4_for_3() {
    for base_id in ["DEMON4Y", "TS4Y", "PURPLE4"] {
        let mut r = setup();
        let base = r.place_on_field(0, base_id, Some(0));
        assert!(digivolve_onto(&mut r, base), "{base_id}");
        assert_eq!(top_id(&r, base), CARD_ID);
        assert_eq!(r.memory(), 10 - 3, "{base_id}");
    }
}

#[test]
fn bt24_072_cannot_digivolve_from_unrelated_lv4() {
    let mut r = setup();
    let base = r.place_on_field(0, "PLAIN4Y", Some(0));
    assert!(!digivolve_onto(&mut r, base));
    assert_eq!(top_id(&r, base), "PLAIN4Y");
}

// ─── Inherited: [Your Turn] while [Titamon] / [Titan] → <Security A. +1> ────

#[test]
fn bt24_072_inherited_security_attack_plus_on_titan_carrier_your_turn() {
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
fn bt24_072_inherited_security_attack_plus_on_titamon_carrier() {
    let mut r = setup();
    let host = r.place_stack(0, &[CARD_ID, "TITAMON"]);
    r.game.tick_declarative_effects();
    assert_eq!(r.game.security_attack_keyword_bonus(host), 1, "is [Titamon]");
}

#[test]
fn bt24_072_inherited_no_bonus_on_non_titan_carrier() {
    let mut r = setup();
    let host = r.place_stack(0, &[CARD_ID, "BEAST6"]);
    r.game.tick_declarative_effects();
    assert_eq!(r.game.security_attack_keyword_bonus(host), 0);
    r.attack_player(host, 1, false);
    let _ = r.auto_resolve();
    assert_eq!(r.security_count(1), 4 - 1, "single check");
}

#[test]
fn bt24_072_inherited_no_bonus_on_opponents_turn() {
    let mut r = setup();
    let host = r.place_stack(0, &[CARD_ID, "TITAN5"]);
    r.game.turn_player_idx = 1;
    r.game.tick_declarative_effects();
    assert_eq!(r.game.security_attack_keyword_bonus(host), 0, "[Your Turn] only");
}

#[test]
fn bt24_072_face_up_card_has_no_printed_security_attack() {
    // The <Security A. +1> is inherited-only; face-up it does nothing.
    let mut r = setup();
    let me = r.place_on_field(0, CARD_ID, Some(0));
    r.game.tick_declarative_effects();
    assert_eq!(r.game.security_attack_keyword_bonus(me), 0);
}

// ─── [On Play][When Digivolving] By trashing 1 card in your hand ─────────────

#[test]
fn bt24_072_decline_cost_does_nothing() {
    let mut r = setup();
    let titan = r.place_on_field(0, "TITAN3", Some(0));
    push_hand(&mut r, 0, "FILLER");
    let base = r.place_on_field(0, "DEMON4Y", Some(0));
    assert!(digivolve_onto(&mut r, base));
    let v = r.pending_selection_view().expect("optional cost");
    assert!(v.is_optional, "'By trashing' — the cost is optional");
    pass(&mut r, 0);
    let _ = r.auto_resolve();
    assert!(r.pending_selection_view().is_none());
    assert!(hand_ids(&r, 0).contains(&"FILLER".to_string()), "nothing trashed");
    assert!(!r.game.has_keyword(titan, Keyword::Blocker));
}

#[test]
fn bt24_072_empty_hand_does_not_activate() {
    let mut r = setup();
    let titan = r.place_on_field(0, "TITAN3", Some(0));
    let _me = play_card(&mut r);
    assert_eq!(r.hand_size(0), 0);
    assert!(r.pending_selection_view().is_none());
    assert!(!r.game.has_keyword(titan, Keyword::Blocker));
}

#[test]
fn bt24_072_when_digivolving_grants_blocker_and_retaliation_until_opponent_turn_ends() {
    let mut r = setup();
    let titan = r.place_on_field(0, "TITAN3", Some(0));
    r.place_on_field(0, "SHAMAN3", Some(0));
    let beast = r.place_on_field(0, "BEAST3", Some(0));
    push_hand(&mut r, 0, "FILLER");
    let base = r.place_on_field(0, "DEMON4Y", Some(0));
    assert!(digivolve_onto(&mut r, base));
    let v = r.pending_selection_view().expect("optional cost");
    assert!(v.is_optional);
    pick_hand(&mut r, 0, "FILLER");
    let v = r.pending_selection_view().expect("Digimon pick");
    assert!(!v.is_optional, "the grant is mandatory once paid");
    // SkullGreymon ([Titan]), TITAN3, SHAMAN3 — not BEAST3.
    assert_eq!(non_pass(&r).len(), 3);
    pick_side_field(&mut r, 0, titan);
    let _ = r.auto_resolve();
    assert!(r.game.has_keyword(titan, Keyword::Blocker));
    assert!(r.game.has_keyword(titan, Keyword::Retaliation));
    assert!(!r.game.has_keyword(beast, Keyword::Blocker));
    r.end_turn();
    let _ = r.auto_resolve();
    assert_eq!(r.turn_player(), 1);
    assert!(r.game.has_keyword(titan, Keyword::Retaliation), "lasts through their turn");
    r.end_turn();
    let _ = r.auto_resolve();
    assert!(!r.game.has_keyword(titan, Keyword::Blocker), "expired");
    assert!(!r.game.has_keyword(titan, Keyword::Retaliation), "expired");
}

#[test]
fn bt24_072_on_play_fires_too_and_can_pick_itself() {
    let mut r = setup();
    push_hand(&mut r, 0, "FILLER");
    let me = play_card(&mut r);
    assert_eq!(r.memory(), 10 - 7);
    pick_hand(&mut r, 0, "FILLER");
    assert_eq!(non_pass(&r).len(), 1, "only SkullGreymon itself qualifies");
    pick_side_field(&mut r, 0, me);
    let _ = r.auto_resolve();
    assert!(r.game.has_keyword(me, Keyword::Blocker));
    assert!(r.game.has_keyword(me, Keyword::Retaliation));
}

#[test]
fn bt24_072_on_deletion_plays_lv4_demon_or_titan_from_trash_free() {
    let mut r = setup();
    push_trash(&mut r, 0, "DEMON4Y");
    push_trash(&mut r, 0, "TITAN4Y");
    push_trash(&mut r, 0, "TITAN5");
    push_trash(&mut r, 0, "BEAST4");
    let me = r.place_on_field(0, CARD_ID, Some(0));
    r.game
        .delete_permanent_with_cause(me, ReplacementCause::OpponentEffect);
    let v = r.pending_selection_view().expect("[On Deletion] trash pick");
    assert!(v.is_optional, "'You may play'");
    assert_eq!(non_pass(&r).len(), 2, "DEMON4Y + TITAN4Y (not Lv.5 Titan, not Beast, not itself)");
    pick_trash(&mut r, 0, "TITAN4Y");
    let _ = r.auto_resolve();
    assert_eq!(field_ids(&r, 0), vec!["TITAN4Y".to_string()]);
    assert_eq!(r.memory(), 10, "without paying the cost");
}

#[test]
fn bt24_072_on_deletion_can_replay_its_own_lv4_digivolution_card() {
    // Rule 25: [On Deletion] resolves post-trash, so a Lv.4 [Demon] that was
    // under SkullGreymon is already in the trash and a legal pick.
    let mut r = setup();
    let me = r.place_stack(0, &["DEMON4Y", CARD_ID]);
    r.game
        .delete_permanent_with_cause(me, ReplacementCause::OpponentEffect);
    assert_eq!(non_pass(&r).len(), 1);
    pick_trash(&mut r, 0, "DEMON4Y");
    let _ = r.auto_resolve();
    assert_eq!(field_ids(&r, 0), vec!["DEMON4Y".to_string()]);
    assert!(trash_ids(&r, 0).contains(&CARD_ID.to_string()));
}

#[test]
fn bt24_072_on_deletion_decline_plays_nothing() {
    let mut r = setup();
    push_trash(&mut r, 0, "DEMON4Y");
    let me = r.place_on_field(0, CARD_ID, Some(0));
    r.game
        .delete_permanent_with_cause(me, ReplacementCause::OpponentEffect);
    pass(&mut r, 0);
    let _ = r.auto_resolve();
    assert!(field_ids(&r, 0).is_empty());
    assert!(trash_ids(&r, 0).contains(&"DEMON4Y".to_string()));
}

#[test]
fn bt24_072_on_deletion_in_battle_on_opponents_turn() {
    // Deleted by an opponent's attack: the revive happens on their turn.
    let mut r = setup();
    push_trash(&mut r, 0, "TITAN4Y");
    let me = r.place_on_field(0, CARD_ID, Some(0));
    let opp = r.place_on_field(1, "OPP5", Some(0));
    r.game.turn_player_idx = 1;
    {
        let perm = &mut r.game.players[0].battle_area[me.index as usize];
        perm.is_suspended = true;
    }
    r.force_base_dp("OPP5", 9000);
    r.attack_digimon(opp, me, false);
    drain_first(&mut r);
    let _ = r.auto_resolve();
    assert!(field_ids(&r, 0).contains(&"TITAN4Y".to_string()), "{:?}", field_ids(&r, 0));
}


#[allow(dead_code)]
fn _unused(_: Keyword, _: ReplacementCause) {}
