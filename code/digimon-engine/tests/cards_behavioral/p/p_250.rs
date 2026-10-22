//! P-250 Ogremon (X Antibody) — Digimon, Lv.4, Purple/Black, DP 7000, cost 6.
//! Traits: Demon / X Antibody. Form: Champion. Attribute: Virus.
//!
//! Printed text (official Bandai DB — data/card_bundles/P-250.md; no local
//! card image):
//!   Digivolve: Purple Lv.3 / cost 3; Black Lv.3 / cost 3;
//!     [Digivolve] [Ogremon]/[Fugamon]/[Hyogamon]: Cost 1.
//!   {Trash} [End of Your Turn] If your hand has 5 or fewer cards, 1 of your
//!     [Demon] trait Digimon may digivolve into this card.
//!   [On Play] [When Digivolving] [When Attacking] [Once Per Turn] By trashing
//!     1 card in your hand, 1 of your [Demon], [Shaman] or [Undead] trait
//!     Digimon gains <Blocker> and <Retaliation> until your opponent's turn
//!     ends.
//!   Inherited: [On Deletion] Delete 1 of your opponent's Digimon with a play
//!     cost of 6 or less.
//!   Official Q&A: a {Trash} effect triggers and activates only while its card
//!     is in the trash.
//!
//! DCGO C# reference: DCGO/Assets/Scripts/CardEffect/P/Purple/P_250.cs
//!
//! Pattern rows: alt-digivolve by name, {Trash} zone trigger at
//! [End of Your Turn] (G-ENGINE-TRASH-ZONE-END-OF-YOUR-TURN), shared
//! OP/WD/WA once-per-turn cost clause (BT24-026 idiom), inherited
//! [On Deletion] removal.

#[path = "../bt26/support.rs"]
mod support;

use digimon_dsl::compiled::{CompiledClause, CompiledScope, CompiledTiming};
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::{CardColor, GamePhase, Keyword, PlaySource};
use digimon_engine::permanent::PermanentHandle;
use support::*;

const CARD_ID: &str = "P-250";

fn setup() -> DebugRunner {
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("P-250 in embedded DSL pack")
        .add_card(filler("FILLER"))
        .add_card(digimon("OGREMON", "Ogremon", CardColor::Red, 4, 5, &["Demon"]))
        .add_card(digimon("FUGAMON", "Fugamon", CardColor::Red, 4, 5, &["Demon"]))
        .add_card(digimon("HYOGAMON", "Hyogamon", CardColor::Red, 4, 5, &["Demon"]))
        .add_card(digimon("PURPLE3", "Purple Three", CardColor::Purple, 3, 3, &[]))
        .add_card(digimon("BLACK3", "Black Three", CardColor::Black, 3, 3, &[]))
        .add_card(digimon("RED3", "Red Three", CardColor::Red, 3, 3, &[]))
        .add_card(digimon("DEMON3", "Demon Three", CardColor::Purple, 3, 3, &["Demon"]))
        .add_card(digimon("DEMON5", "Demon Five", CardColor::Purple, 5, 7, &["Demon"]))
        .add_card(digimon("BEAST3", "Beast Three", CardColor::Purple, 3, 3, &["Beast"]))
        .add_card(digimon("SHAMAN3", "Shaman Three", CardColor::Yellow, 3, 3, &["Shaman"]))
        .add_card(digimon("UNDEAD3", "Undead Three", CardColor::Yellow, 3, 3, &["Undead"]))
        .add_card(digimon("PLAIN3", "Plain Three", CardColor::Yellow, 3, 3, &[]))
        .add_card(digimon("OPPC6", "Opp Cost Six", CardColor::Red, 4, 6, &[]))
        .add_card(digimon("OPPC7", "Opp Cost Seven", CardColor::Red, 5, 7, &[]))
        .deck(0, &["FILLER"; 10])
        .deck(1, &["FILLER"; 10])
        .security(1, &["FILLER"; 3])
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

/// Play P-250 from the hand for real (pays 6) and return its handle.
fn play_card(r: &mut DebugRunner) -> PermanentHandle {
    let idx = r.add_to_hand(0, CARD_ID);
    r.play(0, idx).expect("P-250 plays");
    let h = find(r, 0, CARD_ID);
    r.game.players[0].battle_area[h.index as usize].turn_played = 0;
    h
}

/// Digivolve P-250 from the hand onto `base`; true on success.
fn digivolve_onto(r: &mut DebugRunner, base: PermanentHandle) -> bool {
    let idx = r.add_to_hand(0, CARD_ID);
    r.game.digivolve_from_hand(0, idx, base.index as usize, PlaySource::ByDigivolve)
}

fn triggered(r: &DebugRunner) -> Vec<digimon_dsl::compiled::CompiledTriggeredClause> {
    r.compiled_card(CARD_ID)
        .expect("compiled")
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t.clone()),
            _ => None,
        })
        .collect()
}

// ─── Section 1 — Structural ──────────────────────────────────────────────────

#[test]
fn p_250_metadata_and_clause_shape() {
    let r = setup();
    let card = r.compiled_card(CARD_ID).expect("compiled");
    assert_eq!(card.name, "Ogremon (X Antibody)");
    assert_eq!(card.level, Some(4));
    assert_eq!(card.cost, Some(6));
    assert_eq!(card.dp, Some(7000));
    assert_eq!(card.color.len(), 2, "Purple/Black");
    for t in ["Demon", "X Antibody"] {
        assert!(card.traits.iter().any(|x| x == t), "trait {t}");
    }
    let trig = triggered(&r);
    assert_eq!(trig.len(), 3);
    let eot = trig
        .iter()
        .find(|t| t.scope == CompiledScope::Trash)
        .expect("{Trash} clause");
    assert_eq!(eot.when, vec![CompiledTiming::EndOfYourTurn]);
    assert!(!eot.once_per_turn);
    let shared = trig
        .iter()
        .find(|t| t.when.contains(&CompiledTiming::OnPlay))
        .expect("OP/WD/WA clause");
    assert_eq!(
        shared.when,
        vec![
            CompiledTiming::OnPlay,
            CompiledTiming::WhenDigivolving,
            CompiledTiming::WhenAttacking
        ],
        "ONE clause, ONE shared once-per-turn counter"
    );
    assert!(shared.once_per_turn);
    assert!(!shared.optional, "the cost pick is the decline surface");
    let inh = trig
        .iter()
        .find(|t| t.scope == CompiledScope::Inherited)
        .expect("inherited");
    assert_eq!(inh.when, vec![CompiledTiming::OnDeletion]);
    assert!(!inh.optional, "the deletion is mandatory");
}

// ─── Section 2 — Digivolution requirements ───────────────────────────────────

#[test]
fn p_250_alt_digivolve_from_ogremon_fugamon_hyogamon_costs_1() {
    for base_id in ["OGREMON", "FUGAMON", "HYOGAMON"] {
        let mut r = setup();
        let base = r.place_on_field(0, base_id, Some(0));
        assert!(digivolve_onto(&mut r, base), "{base_id}");
        assert_eq!(top_id(&r, base), CARD_ID, "{base_id}");
        assert_eq!(r.memory(), 10 - 1, "{base_id}");
    }
}

#[test]
fn p_250_colour_circles_lv3_cost_3() {
    for base_id in ["PURPLE3", "BLACK3"] {
        let mut r = setup();
        let base = r.place_on_field(0, base_id, Some(0));
        assert!(digivolve_onto(&mut r, base), "{base_id}");
        assert_eq!(r.memory(), 10 - 3, "{base_id}");
    }
}

#[test]
fn p_250_cannot_digivolve_from_off_colour_lv3() {
    let mut r = setup();
    let base = r.place_on_field(0, "RED3", Some(0));
    assert!(!digivolve_onto(&mut r, base));
    assert_eq!(top_id(&r, base), "RED3");
}

// ─── Section 3 — {Trash} [End of Your Turn] ──────────────────────────────────

#[test]
fn p_250_trash_end_of_turn_demon_may_digivolve_into_it() {
    let mut r = setup();
    let demon = r.place_on_field(0, "DEMON3", Some(0));
    r.place_on_field(0, "BEAST3", Some(0));
    r.place_on_field(0, "DEMON5", Some(0));
    push_trash(&mut r, 0, CARD_ID);
    r.game.memory = 5;
    r.end_turn();
    let v = r.pending_selection_view().expect("{Trash}[End of Your Turn] pick");
    assert!(v.is_optional, "\"may digivolve\"");
    assert_eq!(
        non_pass(&r).len(),
        1,
        "only the [Demon] Digimon that P-250 can digivolve onto (not BEAST3, not the Lv.5)"
    );
    pick_side_field(&mut r, 0, demon);
    drain_first(&mut r);
    assert_eq!(top_id(&r, demon), CARD_ID, "digivolved from the trash");
    assert!(!trash_ids(&r, 0).contains(&CARD_ID.to_string()));
}

#[test]
fn p_250_trash_end_of_turn_pays_the_digivolve_cost() {
    let mut r = setup();
    let demon = r.place_on_field(0, "DEMON3", Some(0));
    push_trash(&mut r, 0, CARD_ID);
    r.game.memory = 5;
    r.end_turn();
    pick_side_field(&mut r, 0, demon);
    // The cost is paid before the turn passes: 5 - 3 = 2 left for player 0,
    // i.e. the gauge stands at 2 on player 0's side when the turn rotates.
    drain_first(&mut r);
    assert_eq!(top_id(&r, demon), CARD_ID);
    assert_eq!(r.game.turn_player(), 1, "the turn passed");
    assert_eq!(r.memory(), -2, "player 0 kept 2 memory ⇒ player 1 starts at -2 from their view");
}

#[test]
fn p_250_trash_end_of_turn_decline_leaves_it_in_trash() {
    let mut r = setup();
    let demon = r.place_on_field(0, "DEMON3", Some(0));
    push_trash(&mut r, 0, CARD_ID);
    r.end_turn();
    pass(&mut r, 0);
    drain_first(&mut r);
    assert_eq!(top_id(&r, demon), "DEMON3");
    assert!(trash_ids(&r, 0).contains(&CARD_ID.to_string()));
}

#[test]
fn p_250_trash_end_of_turn_needs_five_or_fewer_hand_cards() {
    let mut r = setup();
    let demon = r.place_on_field(0, "DEMON3", Some(0));
    push_trash(&mut r, 0, CARD_ID);
    for _ in 0..6 {
        push_hand(&mut r, 0, "FILLER");
    }
    r.end_turn();
    assert!(r.pending_selection_view().is_none(), "6 cards in hand ⇒ no effect");
    assert_eq!(top_id(&r, demon), "DEMON3");
    assert!(trash_ids(&r, 0).contains(&CARD_ID.to_string()));
}

#[test]
fn p_250_trash_end_of_turn_exactly_five_hand_cards_still_works() {
    let mut r = setup();
    let demon = r.place_on_field(0, "DEMON3", Some(0));
    push_trash(&mut r, 0, CARD_ID);
    for _ in 0..5 {
        push_hand(&mut r, 0, "FILLER");
    }
    r.end_turn();
    assert!(r.pending_selection_view().is_some(), "5 or fewer ⇒ the pick is offered");
    pick_side_field(&mut r, 0, demon);
    drain_first(&mut r);
    assert_eq!(top_id(&r, demon), CARD_ID);
}

#[test]
fn p_250_trash_end_of_turn_not_from_the_hand() {
    let mut r = setup();
    let demon = r.place_on_field(0, "DEMON3", Some(0));
    push_hand(&mut r, 0, CARD_ID);
    r.end_turn();
    assert!(r.pending_selection_view().is_none(), "{{Trash}} only works from the trash");
    assert_eq!(top_id(&r, demon), "DEMON3");
}

#[test]
fn p_250_trash_end_of_turn_not_at_the_opponents_turn_end() {
    let mut r = setup();
    r.place_on_field(0, "DEMON3", Some(0));
    push_trash(&mut r, 0, CARD_ID);
    // Hand over to player 1 with nothing for player 0 to answer...
    r.game.players[0].trash.clear();
    r.end_turn();
    let _ = r.auto_resolve();
    assert_eq!(r.game.turn_player(), 1);
    // ...then put P-250 in player 0's trash and end player 1's turn.
    push_trash(&mut r, 0, CARD_ID);
    r.end_turn();
    assert!(
        r.pending_selection_view().is_none(),
        "[End of YOUR Turn]: silent at the opponent's turn end"
    );
}

#[test]
fn p_250_trash_end_of_turn_ogremon_route_costs_1() {
    let mut r = setup();
    let ogre = r.place_on_field(0, "OGREMON", Some(0));
    push_trash(&mut r, 0, CARD_ID);
    r.game.memory = 5;
    r.end_turn();
    pick_side_field(&mut r, 0, ogre);
    drain_first(&mut r);
    assert_eq!(top_id(&r, ogre), CARD_ID);
    assert_eq!(r.memory(), -4, "paid 1 of 5 ⇒ player 1 sees -4");
}

// ─── Section 4 — [On Play][When Digivolving][When Attacking][OPT] ────────────

#[test]
fn p_250_on_play_grants_blocker_and_retaliation_until_opponent_turn_ends() {
    let mut r = setup();
    let shaman = r.place_on_field(0, "SHAMAN3", Some(0));
    r.place_on_field(0, "UNDEAD3", Some(0));
    let plain = r.place_on_field(0, "PLAIN3", Some(0));
    push_hand(&mut r, 0, "FILLER");
    let me = play_card(&mut r);
    assert_eq!(r.memory(), 10 - 6, "played for its cost");
    let v = r.pending_selection_view().expect("optional cost");
    assert!(v.is_optional, "'By trashing' — the cost is optional");
    pick_hand(&mut r, 0, "FILLER");
    assert!(trash_ids(&r, 0).contains(&"FILLER".to_string()), "cost paid");
    let v = r.pending_selection_view().expect("Digimon pick");
    assert!(!v.is_optional, "the grant is mandatory once paid");
    // P-250 itself ([Demon]), SHAMAN3, UNDEAD3 — not PLAIN3.
    assert_eq!(non_pass(&r).len(), 3);
    pick_side_field(&mut r, 0, shaman);
    let _ = r.auto_resolve();
    assert!(r.game.has_keyword(shaman, Keyword::Blocker));
    assert!(r.game.has_keyword(shaman, Keyword::Retaliation));
    assert!(!r.game.has_keyword(plain, Keyword::Blocker));
    assert!(!r.game.has_keyword(me, Keyword::Retaliation));
    r.end_turn();
    let _ = r.auto_resolve();
    assert_eq!(r.turn_player(), 1);
    assert!(r.game.has_keyword(shaman, Keyword::Blocker), "lasts through their turn");
    assert!(r.game.has_keyword(shaman, Keyword::Retaliation));
    r.end_turn();
    let _ = r.auto_resolve();
    assert!(!r.game.has_keyword(shaman, Keyword::Blocker), "expired at their turn end");
    assert!(!r.game.has_keyword(shaman, Keyword::Retaliation));
}

#[test]
fn p_250_when_digivolving_grants_to_undead() {
    let mut r = setup();
    let base = r.place_on_field(0, "PURPLE3", Some(0));
    let undead = r.place_on_field(0, "UNDEAD3", Some(0));
    push_hand(&mut r, 0, "FILLER");
    assert!(digivolve_onto(&mut r, base));
    let v = r.pending_selection_view().expect("[When Digivolving] cost");
    assert!(v.is_optional);
    pick_hand(&mut r, 0, "FILLER");
    pick_side_field(&mut r, 0, undead);
    let _ = r.auto_resolve();
    assert!(r.game.has_keyword(undead, Keyword::Blocker));
    assert!(r.game.has_keyword(undead, Keyword::Retaliation));
}

#[test]
fn p_250_when_attacking_can_target_itself() {
    let mut r = setup();
    let me = r.place_on_field(0, CARD_ID, Some(0));
    push_hand(&mut r, 0, "FILLER");
    r.attack_player(me, 1, false);
    let v = r.pending_selection_view().expect("[When Attacking] cost");
    assert!(v.is_optional);
    pick_hand(&mut r, 0, "FILLER");
    assert_eq!(non_pass(&r).len(), 1, "only P-250 itself qualifies");
    pick_side_field(&mut r, 0, me);
    assert!(r.game.has_keyword(me, Keyword::Blocker));
    assert!(r.game.has_keyword(me, Keyword::Retaliation));
}

#[test]
fn p_250_decline_cost_does_nothing_and_spends_the_once_per_turn() {
    let mut r = setup();
    let shaman = r.place_on_field(0, "SHAMAN3", Some(0));
    push_hand(&mut r, 0, "FILLER");
    let me = play_card(&mut r);
    pass(&mut r, 0);
    let _ = r.auto_resolve();
    assert!(r.pending_selection_view().is_none(), "declining ends the clause");
    assert_eq!(r.hand_size(0), 1, "nothing trashed");
    assert!(!r.game.has_keyword(shaman, Keyword::Blocker));
    // DCGO counts the activation before the cost pick (no RemoveUse).
    r.attack_player(me, 1, false);
    assert!(r.pending_selection_view().is_none(), "OPT already spent");
}

#[test]
fn p_250_once_per_turn_shared_and_resets_next_turn() {
    let mut r = setup();
    r.place_on_field(0, "SHAMAN3", Some(0));
    push_hand(&mut r, 0, "FILLER");
    push_hand(&mut r, 0, "FILLER");
    let me = play_card(&mut r);
    pick_hand(&mut r, 0, "FILLER");
    drain_first(&mut r);
    let _ = r.auto_resolve();
    r.attack_player(me, 1, false);
    assert!(
        r.pending_selection_view().is_none(),
        "[When Attacking] shares the [On Play] once-per-turn"
    );
    let _ = r.auto_resolve();
    r.end_turn();
    let _ = r.auto_resolve();
    r.end_turn();
    let _ = r.auto_resolve();
    assert_eq!(r.turn_player(), 0);
    r.game.current_phase = GamePhase::Main;
    push_hand(&mut r, 0, "FILLER");
    let me = find(&r, 0, CARD_ID);
    r.attack_player(me, 1, false);
    assert!(
        r.pending_selection_view().is_some(),
        "a new turn re-arms the once-per-turn"
    );
}

#[test]
fn p_250_empty_hand_never_activates_so_the_once_per_turn_is_kept() {
    // CR 4.3 §15-7-4 / §15-8-4-4-1: an unpayable optional condition can't be
    // chosen, so the clause doesn't activate and the once-per-turn is not spent
    // (dcgo_quirk: DCGO offers the cost pick and spends the use).
    let mut r = setup();
    let shaman = r.place_on_field(0, "SHAMAN3", Some(0));
    let me = play_card(&mut r);
    assert_eq!(r.hand_size(0), 0, "empty hand after the play");
    assert!(
        r.pending_selection_view().is_none(),
        "the unpayable 'by trashing 1 card in your hand' is not offered"
    );
    assert!(!r.game.has_keyword(shaman, Keyword::Blocker));
    push_hand(&mut r, 0, "FILLER");
    r.attack_player(me, 1, false);
    let v = r.pending_selection_view().expect("[When Attacking] still has its once-per-turn");
    assert!(v.is_optional, "the cost pick is offered now that it is payable");
}

// ─── Section 5 — Inherited [On Deletion] ─────────────────────────────────────

#[test]
fn p_250_inherited_on_deletion_deletes_cost_six_or_less() {
    let mut r = setup();
    let host = r.place_stack(0, &[CARD_ID, "DEMON5"]);
    let c6 = r.place_on_field(1, "OPPC6", Some(0));
    r.place_on_field(1, "OPPC7", Some(0));
    r.game.delete_permanent_with_effects(host);
    let v = r.pending_selection_view().expect("[On Deletion] pick");
    assert!(!v.is_optional, "mandatory deletion");
    assert_eq!(non_pass(&r).len(), 1, "only the cost-6 Digimon");
    pick_side_field(&mut r, 0, c6);
    let _ = r.auto_resolve();
    assert_eq!(field_ids(&r, 1), vec!["OPPC7".to_string()]);
    assert!(trash_ids(&r, 1).contains(&"OPPC6".to_string()));
}

#[test]
fn p_250_inherited_on_deletion_no_target_above_six() {
    let mut r = setup();
    let host = r.place_stack(0, &[CARD_ID, "DEMON5"]);
    r.place_on_field(1, "OPPC7", Some(0));
    r.game.delete_permanent_with_effects(host);
    let _ = r.auto_resolve();
    assert!(r.pending_selection_view().is_none());
    assert_eq!(field_ids(&r, 1), vec!["OPPC7".to_string()]);
}

#[test]
fn p_250_top_card_on_deletion_is_not_its_own_text() {
    // The [On Deletion] is inherited: P-250 as the TOP card does not use it.
    let mut r = setup();
    let me = r.place_on_field(0, CARD_ID, Some(0));
    r.place_on_field(1, "OPPC6", Some(0));
    r.game.delete_permanent_with_effects(me);
    let _ = r.auto_resolve();
    assert_eq!(field_ids(&r, 1), vec!["OPPC6".to_string()]);
}
