//! BT24-045 Ogremon — Lv.4 Green/Purple, Demon/Titan/TS.
//!
//! When this card is trashed from the hand, if you have 5 or fewer cards in
//! your hand, <Draw 1>.
//! [On Play] [When Attacking] [Once Per Turn] By trashing 1 card in your hand,
//! suspend 1 of your opponent's Digimon. It can't unsuspend in their next
//! unsuspend phase.
//! Inherited: [Your Turn] [Once Per Turn] When your hand is trashed from, this
//! [Demon] or [Titan] trait Digimon may digivolve into [Titamon] or a [Titan]
//! trait Digimon card in the trash with the digivolution cost reduced by 1.
//!
//! DCGO: BT24/Green/BT24_045.cs.

#[path = "../bt26/support.rs"]
mod support;

use digimon_dsl::compiled::{CompiledClause, CompiledScope, CompiledTiming};
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::{CardColor, EffectTiming, GamePhase, Keyword, ModifierType, PlaySource};
use digimon_engine::permanent::PermanentHandle;
use support::*;

const CARD_ID: &str = "BT24-045";

fn setup() -> DebugRunner {
    let mut opp6 = digimon("OPP6K", "Opp Six", CardColor::Red, 5, 7, &[]);
    opp6.dp = Some(6000);
    let mut opp7 = digimon("OPP7K", "Opp Seven", CardColor::Red, 5, 7, &[]);
    opp7.dp = Some(7000);
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT24-045 in embedded DSL pack")
        .from_dsl_yaml(HAND_TRASHER_YAML)
        .expect("trasher")
        .add_card(filler("FILLER"))
        .add_card(digimon("TITAN3", "Titan Three", CardColor::Purple, 3, 3, &["Titan"]))
        .add_card(digimon("SHAMAN3", "Shaman Three", CardColor::Yellow, 3, 3, &["Shaman"]))
        .add_card(digimon("BEAST3", "Beast Three", CardColor::Yellow, 3, 3, &["Beast"]))
        .add_card(digimon("DEMON3", "Demon Three", CardColor::Yellow, 3, 3, &["Demon"]))
        .add_card(digimon("TS3", "TS Three", CardColor::Yellow, 3, 3, &["TS"]))
        .add_card(digimon("PLAIN3", "Plain Three", CardColor::Yellow, 3, 3, &[]))
        .add_card(digimon("GREEN3", "Colour Three", CardColor::Green, 3, 3, &[]))
        .add_card(digimon("PURPLE3", "Purple Three", CardColor::Purple, 3, 3, &[]))
        .add_card(digimon("DEMON4", "Demon Four", CardColor::Purple, 4, 5, &["Demon"]))
        .add_card(digimon("BEAST4", "Beast Four", CardColor::Purple, 4, 5, &["Beast"]))
        .add_card(digimon_evo("TITAN5", "Titan Five", CardColor::Purple, 5, 7, &["Titan"], 4, 3))
        .add_card(digimon("OPP3", "Opp Three", CardColor::Red, 3, 3, &[]))
        .add_card(tamer("OPPT", "Opp Tamer", CardColor::Red, &[]))
        .add_card(opp6)
        .add_card(opp7)
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

fn hand_idx(r: &DebugRunner, p: u8, id: &str) -> usize {
    r.game.players[p as usize]
        .hand
        .iter()
        .position(|c| c.card_id(&r.game.card_data) == id)
        .unwrap_or_else(|| panic!("{id} not in hand"))
}

fn find(r: &DebugRunner, p: u8, id: &str) -> PermanentHandle {
    let index = r.game.players[p as usize]
        .battle_area
        .iter()
        .position(|perm| perm.top_card().card_id(&r.game.card_data) == id)
        .unwrap_or_else(|| panic!("{id} not on player {p}'s field"));
    PermanentHandle { player: p, index: index as u8 }
}

fn suspended(r: &DebugRunner, h: PermanentHandle) -> bool {
    r.game.players[h.player as usize].battle_area[h.index as usize].is_suspended
}

/// Play BT24-045 from the hand for real (pays 4) and return its handle.
fn play_card(r: &mut DebugRunner) -> PermanentHandle {
    let idx = r.add_to_hand(0, CARD_ID);
    r.play(0, idx).expect("BT24-045 plays");
    let h = find(r, 0, CARD_ID);
    // Lift summoning sickness so tests can drive a real [When Attacking]
    // in the same turn (the play itself already happened for real).
    r.game.players[0].battle_area[h.index as usize].turn_played = 0;
    h
}

/// Digivolve BT24-045 from the hand onto `base`; true on success.
fn digivolve_onto(r: &mut DebugRunner, base: PermanentHandle) -> bool {
    let idx = r.add_to_hand(0, CARD_ID);
    r.game.digivolve_from_hand(0, idx, base.index as usize, PlaySource::ByDigivolve)
}

// ─── Section 1 — Structural ──────────────────────────────────────────────────

#[test]
fn bt24_045_metadata_and_clause_shape() {
    let r = setup();
    let card = r.compiled_card(CARD_ID).expect("compiled");
    assert_eq!(card.name, "Ogremon");
    assert_eq!(card.level, Some(4));
    assert_eq!(card.cost, Some(4));
    assert_eq!(card.dp, Some(4000));
    assert_eq!(card.color.len(), 2, "Green/Purple");
    for t in ["Demon", "Titan", "TS"] {
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
    assert_eq!(triggered.len(), 3);
    let draw = triggered
        .iter()
        .find(|t| t.scope == CompiledScope::Trash)
        .expect("trash-scoped self draw");
    assert_eq!(draw.when, vec![CompiledTiming::OnDiscardHand]);
    assert!(!draw.once_per_turn && !draw.optional);
    let shared = triggered
        .iter()
        .find(|t| t.when.contains(&CompiledTiming::OnPlay))
        .expect("OP/WA clause");
    assert_eq!(
        shared.when,
        vec![CompiledTiming::OnPlay, CompiledTiming::WhenAttacking],
        "ONE clause, ONE shared once-per-turn counter"
    );
    assert!(shared.once_per_turn);
    assert!(!shared.optional, "the cost pick is the decline surface");
    assert_eq!(shared.scope, CompiledScope::FaceUp);
    let inh = triggered
        .iter()
        .find(|t| t.scope == CompiledScope::Inherited)
        .expect("inherited");
    assert_eq!(inh.when, vec![CompiledTiming::OnDiscardHand]);
    assert!(inh.once_per_turn);
}

// ─── Section 2 — Digivolution requirements ───────────────────────────────────

#[test]
fn bt24_045_alt_digivolve_from_demon_lv3_costs_2() {
    let mut r = setup();
    let base = r.place_on_field(0, "DEMON3", Some(0));
    assert!(digivolve_onto(&mut r, base));
    assert_eq!(top_id(&r, base), CARD_ID);
    assert_eq!(r.memory(), 10 - 2);
}

#[test]
fn bt24_045_alt_digivolve_from_ts_lv3_costs_2() {
    let mut r = setup();
    let base = r.place_on_field(0, "TS3", Some(0));
    assert!(digivolve_onto(&mut r, base));
    assert_eq!(r.memory(), 10 - 2);
}

#[test]
fn bt24_045_colour_circles_cost_3() {
    for base_id in ["GREEN3", "PURPLE3"] {
        let mut r = setup();
        let base = r.place_on_field(0, base_id, Some(0));
        assert!(digivolve_onto(&mut r, base), "{base_id}");
        assert_eq!(r.memory(), 10 - 3, "{base_id}");
    }
}

#[test]
fn bt24_045_cannot_digivolve_from_unrelated_lv3() {
    let mut r = setup();
    let base = r.place_on_field(0, "PLAIN3", Some(0));
    assert!(!digivolve_onto(&mut r, base));
    assert_eq!(top_id(&r, base), "PLAIN3");
}

// ─── Section 3 — When trashed from the hand → <Draw 1> ───────────────────────

#[test]
fn bt24_045_trashed_from_hand_draws_one() {
    let mut r = setup();
    push_hand(&mut r, 0, CARD_ID);
    push_hand(&mut r, 0, "FILLER");
    let t = r.place_on_field(0, "T-HANDTRASH", Some(0));
    let deck_before = r.deck_size(0);
    fire(&mut r, EffectTiming::OnPlay, t);
    pick_hand(&mut r, 0, CARD_ID);
    let _ = r.auto_resolve();
    assert_eq!(r.deck_size(0), deck_before - 1, "<Draw 1>");
    assert_eq!(r.hand_size(0), 2);
    assert!(trash_ids(&r, 0).contains(&CARD_ID.to_string()));
}

#[test]
fn bt24_045_trashed_with_six_left_does_not_draw() {
    let mut r = setup();
    push_hand(&mut r, 0, CARD_ID);
    for _ in 0..6 {
        push_hand(&mut r, 0, "FILLER");
    }
    let t = r.place_on_field(0, "T-HANDTRASH", Some(0));
    let deck_before = r.deck_size(0);
    fire(&mut r, EffectTiming::OnPlay, t);
    pick_hand(&mut r, 0, CARD_ID);
    let _ = r.auto_resolve();
    assert_eq!(r.deck_size(0), deck_before, "6 cards left ⇒ no draw");
}

#[test]
fn bt24_045_trashed_with_exactly_five_left_draws() {
    let mut r = setup();
    push_hand(&mut r, 0, CARD_ID);
    for _ in 0..5 {
        push_hand(&mut r, 0, "FILLER");
    }
    let t = r.place_on_field(0, "T-HANDTRASH", Some(0));
    let deck_before = r.deck_size(0);
    fire(&mut r, EffectTiming::OnPlay, t);
    pick_hand(&mut r, 0, CARD_ID);
    let _ = r.auto_resolve();
    assert_eq!(r.deck_size(0), deck_before - 1, "5 or fewer ⇒ draw");
}

#[test]
fn bt24_045_other_hand_trash_does_not_draw_from_trash() {
    let mut r = setup();
    push_trash(&mut r, 0, CARD_ID);
    push_hand(&mut r, 0, "FILLER");
    let t = r.place_on_field(0, "T-HANDTRASH", Some(0));
    let deck_before = r.deck_size(0);
    fire(&mut r, EffectTiming::OnPlay, t);
    pick_hand(&mut r, 0, "FILLER");
    let _ = r.auto_resolve();
    assert_eq!(r.deck_size(0), deck_before);
}

#[test]
fn bt24_045_own_cost_trashing_a_second_copy_draws() {
    // The [On Play] cost trashes another copy of this card from the hand →
    // that copy's "when this card is trashed from the hand" draws 1.
    let mut r = setup();
    push_hand(&mut r, 0, CARD_ID);
    push_hand(&mut r, 0, "FILLER");
    r.place_on_field(1, "OPP3", Some(0));
    let deck_before = r.deck_size(0);
    let _me = play_card(&mut r);
    pick_hand(&mut r, 0, CARD_ID);
    drain_first(&mut r);
    let _ = r.auto_resolve();
    assert_eq!(r.deck_size(0), deck_before - 1, "the trashed copy drew 1");
}

// ─── Section 4 — [On Play][When Attacking][Once Per Turn] ────────────────────

fn phase_locked(r: &DebugRunner, h: PermanentHandle) -> bool {
    r.game
        .modifiers
        .has(h, ModifierType::CannotUnsuspendInUnsuspendPhase)
}

#[test]
fn bt24_045_on_play_suspends_and_locks_their_next_unsuspend_phase() {
    let mut r = setup();
    let opp = r.place_on_field(1, "OPP3", Some(0));
    r.place_on_field(1, "OPP7K", Some(0));
    r.place_on_field(1, "OPPT", Some(0));
    push_hand(&mut r, 0, "FILLER");
    let _me = play_card(&mut r);
    assert_eq!(r.memory(), 10 - 4);
    pick_hand(&mut r, 0, "FILLER");
    let v = r.pending_selection_view().expect("suspend pick");
    assert!(!v.is_optional, "the suspend is mandatory once paid");
    assert_eq!(non_pass(&r).len(), 2, "opponent's Digimon only — not the Tamer");
    pick_side_field(&mut r, 0, opp);
    let _ = r.auto_resolve();
    assert!(suspended(&r, opp));
    assert!(phase_locked(&r, opp));
    assert!(
        !r.game.modifiers.has(opp, ModifierType::CannotUnsuspend),
        "only the unsuspend PHASE is locked, not effect unsuspends"
    );
    r.end_turn();
    let _ = r.auto_resolve();
    assert_eq!(r.turn_player(), 1);
    assert!(suspended(&r, opp), "skipped their next unsuspend phase");
    // Effects may still unsuspend it.
    r.game.unsuspend_with_cause(opp, true);
    assert!(!suspended(&r, opp));
}

#[test]
fn bt24_045_lock_lasts_only_one_unsuspend_phase() {
    let mut r = setup();
    let opp = r.place_on_field(1, "OPP3", Some(0));
    push_hand(&mut r, 0, "FILLER");
    let _me = play_card(&mut r);
    pick_hand(&mut r, 0, "FILLER");
    pick_side_field(&mut r, 0, opp);
    let _ = r.auto_resolve();
    r.end_turn(); // → their turn: stays suspended
    let _ = r.auto_resolve();
    assert!(suspended(&r, opp));
    r.end_turn(); // → our turn
    let _ = r.auto_resolve();
    assert!(!phase_locked(&r, opp), "expired with their turn");
    r.end_turn(); // → their following turn: unsuspends normally
    let _ = r.auto_resolve();
    assert!(!suspended(&r, opp));
}

#[test]
fn bt24_045_when_attacking_suspends_and_locks() {
    let mut r = setup();
    let me = r.place_on_field(0, CARD_ID, Some(0));
    let opp = r.place_on_field(1, "OPP3", Some(0));
    push_hand(&mut r, 0, "FILLER");
    r.attack_player(me, 1, false);
    let v = r.pending_selection_view().expect("[When Attacking] cost");
    assert!(v.is_optional);
    pick_hand(&mut r, 0, "FILLER");
    pick_side_field(&mut r, 0, opp);
    assert!(suspended(&r, opp));
    assert!(phase_locked(&r, opp));
}

#[test]
fn bt24_045_lock_installed_on_their_turn_survives_to_their_next_unsuspend_phase() {
    // e.g. played by SkullGreymon's [On Deletion] during the opponent's
    // attack: "their NEXT unsuspend phase" is the one of their following turn.
    let mut r = setup();
    let opp = r.place_on_field(1, "OPP3", Some(0));
    push_hand(&mut r, 0, "FILLER");
    r.game.turn_player_idx = 1;
    let me = r.place_on_field(0, CARD_ID, Some(0));
    fire(&mut r, EffectTiming::OnPlay, me);
    pick_hand(&mut r, 0, "FILLER");
    pick_side_field(&mut r, 0, opp);
    let _ = r.auto_resolve();
    assert!(suspended(&r, opp));
    r.game.modifiers.expire_end_of_turn(1); // their current turn ends
    assert!(phase_locked(&r, opp), "survives their current turn-end");
    r.game.modifiers.expire_end_of_turn(0); // our turn ends
    assert!(phase_locked(&r, opp), "present at their next unsuspend phase");
    r.game.modifiers.expire_end_of_turn(1); // their next turn ends
    assert!(!phase_locked(&r, opp));
}

#[test]
fn bt24_045_no_opponent_digimon_still_pays_cost_silently() {
    let mut r = setup();
    r.place_on_field(1, "OPPT", Some(0));
    push_hand(&mut r, 0, "FILLER");
    let _me = play_card(&mut r);
    pick_hand(&mut r, 0, "FILLER");
    let _ = r.auto_resolve();
    assert!(r.pending_selection_view().is_none());
    assert_eq!(r.hand_size(0), 0);
}


#[test]
fn bt24_045_decline_cost_does_nothing_and_spends_the_once_per_turn() {
    let mut r = setup();
    let opp = r.place_on_field(1, "OPP3", Some(0));
    push_hand(&mut r, 0, "FILLER");
    let me = play_card(&mut r);
    let v = r.pending_selection_view().expect("optional cost");
    assert!(v.is_optional, "'By trashing' — the cost is optional");
    pass(&mut r, 0);
    let _ = r.auto_resolve();
    assert!(r.pending_selection_view().is_none(), "declining ends the clause");
    assert_eq!(r.hand_size(0), 1, "nothing trashed");
    assert!(!suspended(&r, opp));
    // DCGO counts the activation before the cost pick (no RemoveUse): the
    // shared [Once Per Turn] is spent, so [When Attacking] stays silent.
    r.attack_player(me, 1, false);
    assert!(r.pending_selection_view().is_none(), "OPT already spent");
    let _ = r.auto_resolve();
    assert_eq!(r.security_count(1), 2, "the attack really happened");
}

#[test]
fn bt24_045_empty_hand_does_not_activate_or_spend_the_once_per_turn() {
    let mut r = setup();
    let opp = r.place_on_field(1, "OPP3", Some(0));
    let me = play_card(&mut r);
    assert_eq!(r.hand_size(0), 0);
    assert!(r.pending_selection_view().is_none(), "no hand card ⇒ no activation");
    push_hand(&mut r, 0, "FILLER");
    r.attack_player(me, 1, false);
    let v = r.pending_selection_view().expect("[When Attacking] still available");
    assert!(v.is_optional);
}

#[test]
fn bt24_045_once_per_turn_shared_and_resets_next_turn() {
    let mut r = setup();
    let opp = r.place_on_field(1, "OPP3", Some(0));
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
    assert!(!suspended(&r, me));
    r.attack_player(me, 1, false);
    assert!(
        r.pending_selection_view().is_some(),
        "a new turn re-arms the once-per-turn"
    );
}

// ─── Section 5 — Inherited: digivolve into a [Titan] card in the trash ───────

#[test]
fn bt24_045_inherited_demon_carrier_digivolves_on_own_hand_trash() {
    let mut r = setup();
    let host = r.place_stack(0, &[CARD_ID, "DEMON4"]);
    push_trash(&mut r, 0, "TITAN5");
    push_hand(&mut r, 0, "FILLER");
    let t = r.place_on_field(0, "T-HANDTRASH", Some(0));
    fire(&mut r, EffectTiming::OnPlay, t);
    pick_hand(&mut r, 0, "FILLER");
    let v = r.pending_selection_view().expect("inherited digivolve pick");
    assert!(v.is_optional, "'may digivolve'");
    pick_trash(&mut r, 0, "TITAN5");
    let _ = r.auto_resolve();
    assert_eq!(top_id(&r, host), "TITAN5");
    assert_eq!(r.memory(), 10 - (3 - 1), "cost reduced by 1");
}

#[test]
fn bt24_045_inherited_requires_demon_or_titan_carrier() {
    let mut r = setup();
    let host = r.place_stack(0, &[CARD_ID, "BEAST4"]);
    push_trash(&mut r, 0, "TITAN5");
    push_hand(&mut r, 0, "FILLER");
    let t = r.place_on_field(0, "T-HANDTRASH", Some(0));
    fire(&mut r, EffectTiming::OnPlay, t);
    pick_hand(&mut r, 0, "FILLER");
    let _ = r.auto_resolve();
    assert!(r.pending_selection_view().is_none());
    assert_eq!(top_id(&r, host), "BEAST4");
}

#[test]
fn bt24_045_inherited_only_on_your_turn() {
    let mut r = setup();
    let host = r.place_stack(0, &[CARD_ID, "DEMON4"]);
    push_trash(&mut r, 0, "TITAN5");
    push_hand(&mut r, 0, "FILLER");
    r.game.turn_player_idx = 1;
    let t = r.place_on_field(0, "T-HANDTRASH", Some(0));
    fire(&mut r, EffectTiming::OnPlay, t);
    pick_hand(&mut r, 0, "FILLER");
    let _ = r.auto_resolve();
    assert!(r.pending_selection_view().is_none(), "[Your Turn] only");
    assert_eq!(top_id(&r, host), "DEMON4");
}

#[test]
fn bt24_045_inherited_ignores_opponent_hand_trash() {
    let mut r = setup();
    let host = r.place_stack(0, &[CARD_ID, "DEMON4"]);
    push_trash(&mut r, 0, "TITAN5");
    push_hand(&mut r, 1, "FILLER");
    let t = r.place_on_field(1, "T-HANDTRASH", Some(0));
    fire(&mut r, EffectTiming::OnPlay, t);
    pick_hand(&mut r, 1, "FILLER");
    let _ = r.auto_resolve();
    assert!(r.pending_selection_view().is_none(), "YOUR hand only");
    assert_eq!(top_id(&r, host), "DEMON4");
}

#[test]
fn bt24_045_inherited_decline_refunds_and_is_once_per_turn() {
    let mut r = setup();
    let host = r.place_stack(0, &[CARD_ID, "DEMON4"]);
    push_trash(&mut r, 0, "TITAN5");
    for _ in 0..3 {
        push_hand(&mut r, 0, "FILLER");
    }
    let t = r.place_on_field(0, "T-HANDTRASH", Some(0));
    fire(&mut r, EffectTiming::OnPlay, t);
    pick_hand(&mut r, 0, "FILLER");
    pass(&mut r, 0);
    let _ = r.auto_resolve();
    assert_eq!(top_id(&r, host), "DEMON4");
    // Declined → refunded: the next hand trash re-offers it.
    fire(&mut r, EffectTiming::OnPlay, t);
    pick_hand(&mut r, 0, "FILLER");
    pick_trash(&mut r, 0, "TITAN5");
    let _ = r.auto_resolve();
    assert_eq!(top_id(&r, host), "TITAN5");
}

#[allow(dead_code)]
fn _unused(_: Keyword, _: ModifierType) {}
