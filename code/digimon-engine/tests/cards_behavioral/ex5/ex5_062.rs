//! EX5-062 Anubismon — Digimon, Lv.6, Purple, DP 12000, cost 12.
//! Trait: Shaman. Form: Mega. Attribute: Vaccine.
//!
//! Printed text (official Bandai DB — data/card_bundles/EX5-062.md; card
//! image agrees):
//!   Digivolve: Purple Lv.5 / cost 4.
//!   [When Digivolving] [Main] [Once Per Turn] You may trash up to 3 cards in
//!   your hand. Then, play 1 purple Digimon card from your trash with the play
//!   cost reduced by 3. For each card trashed by this effect, further reduce
//!   it by 1.
//!   [Your Turn] When an effect plays one of your Digimon, delete 1 of your
//!   opponent's level 5 or lower Digimon. If this effect didn't delete,
//!   <Draw 1>.
//!   Official Q&A: trashing 2 cards reduces the play cost by 5.
//!
//! DCGO C# reference: DCGO/Assets/Scripts/CardEffect/EX5/Purple/EX5_062.cs —
//! [When Digivolving] and [Main] are two ActivateClasses sharing the hash
//! "PlayDigimon_EX5_062" (maxCountPerTurn 1 → one shared counter). Hand pick:
//! up to min(3, hand) cards, canNoSelect, Mode.Discard. Then a MANDATORY
//! (canNoSelect false) trash pick of 1 purple Digimon card, played paying
//! printed cost - (3 + trashed). [Main] CanUse: hand >= 1 OR a playable
//! purple Digimon in the trash. Second clause: OnEnterFieldAnyone over own
//! battle-area Digimon, IsByEffect, IsOwnerTurn; a mandatory pick of 1
//! opponent level <= 5 Digimon (HasLevel) → delete; failureProcess → Draw 1.
//!
//! Pattern rows: shared WD/[Main] once-per-turn clause, "up to N" hand trash
//! counted into a cost reduction, effect-play observer with "if this effect
//! didn't delete" fallback.

#[path = "../bt26/support.rs"]
mod support;

use digimon_dsl::compiled::{CompiledClause, CompiledTiming};
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::{CardColor, CardKind, EffectTiming, GamePhase, PlaySource};
use digimon_engine::permanent::PermanentHandle;
use support::*;

const CARD_ID: &str = "EX5-062";

/// Test-only [On Play] "play 1 Digimon card from your trash without paying
/// the cost" — an EFFECT that plays one of its controller's Digimon.
const EFFECT_PLAYER_YAML: &str = r#"
card: T-EFFPLAY
name: Effect Player
kind: digimon
level: 3
color: [white]
cost: 3
dp: 3000
traits: []
effects:
  - when: on_play
    summary: "[On Play] Play 1 Digimon card from your trash without paying the cost"
    process:
      - select_trash:
          of: you
          bind_as: back
          filter: { kind: digimon }
          prompt: "Play 1 Digimon card from your trash without paying the cost"
      - play_from_trash_free: { of: you, hand_index: back }
"#;

fn setup() -> DebugRunner {
    let mut tamer_p = tamer("PTAMER", "Purple Tamer", CardColor::Purple, &[]);
    tamer_p.card_kind = CardKind::Tamer;
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX5-062 in embedded DSL pack")
        .from_dsl_yaml(EFFECT_PLAYER_YAML)
        .expect("effect player")
        .add_card(filler("FILLER"))
        .add_card(digimon("PURPLE5", "Purple Five", CardColor::Purple, 5, 7, &[]))
        .add_card(digimon("RED5", "Red Five", CardColor::Red, 5, 7, &[]))
        .add_card(digimon("P6C", "Purple Cost Six", CardColor::Purple, 4, 6, &[]))
        .add_card(digimon("P9C", "Purple Cost Nine", CardColor::Purple, 5, 9, &[]))
        .add_card(digimon("P2C", "Purple Cost Two", CardColor::Purple, 3, 2, &[]))
        .add_card(digimon("P12C", "Purple Cost Twelve", CardColor::Purple, 6, 12, &[]))
        .add_card(digimon("P14C", "Purple Cost Fourteen", CardColor::Purple, 6, 14, &[]))
        .add_card(digimon("R6C", "Red Cost Six", CardColor::Red, 4, 6, &[]))
        .add_card(tamer_p)
        .add_card(digimon("OPP5", "Opp Five", CardColor::Red, 5, 7, &[]))
        .add_card(digimon("OPP6", "Opp Six", CardColor::Red, 6, 10, &[]))
        .deck(0, &["FILLER"; 10])
        .deck(1, &["FILLER"; 10])
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

/// Digivolve EX5-062 from the hand onto `base` (pays 4); true on success.
fn digivolve_onto(r: &mut DebugRunner, base: PermanentHandle) -> bool {
    let idx = r.add_to_hand(0, CARD_ID);
    r.game.digivolve_from_hand(0, idx, base.index as usize, PlaySource::ByDigivolve)
}

/// Anubismon on the field via a real digivolve from PURPLE5 (memory 10 → 6).
fn anubis_by_digivolve(r: &mut DebugRunner) -> PermanentHandle {
    let base = r.place_on_field(0, "PURPLE5", Some(0));
    assert!(digivolve_onto(r, base));
    base
}

/// Copies of `id` in player 0's trash. (Hand counts are noisy here: the
/// digivolve itself draws 1, and an effect play with no level 5 or lower
/// opponent Digimon draws 1 through the second clause.)
fn trash_count(r: &DebugRunner, id: &str) -> usize {
    trash_ids(r, 0).iter().filter(|x| x.as_str() == id).count()
}

// ─── Section 1 — Structural ──────────────────────────────────────────────────

#[test]
fn ex5_062_metadata_and_clause_shape() {
    let r = setup();
    let card = r.compiled_card(CARD_ID).expect("compiled");
    assert_eq!(card.name, "Anubismon");
    assert_eq!(card.level, Some(6));
    assert_eq!(card.cost, Some(12));
    assert_eq!(card.dp, Some(12000));
    assert!(card.traits.iter().any(|t| t == "Shaman"));
    let trig: Vec<_> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();
    assert_eq!(trig.len(), 2);
    let shared = trig
        .iter()
        .find(|t| t.when.contains(&CompiledTiming::WhenDigivolving))
        .expect("WD/[Main] clause");
    assert_eq!(
        shared.when,
        vec![CompiledTiming::WhenDigivolving, CompiledTiming::MainOnField],
        "ONE clause, ONE shared once-per-turn counter (DCGO shared hash)"
    );
    assert!(shared.once_per_turn);
    assert!(!shared.optional, "only the hand trash is a 'may'; the play is mandatory");
    let obs = trig
        .iter()
        .find(|t| t.when.contains(&CompiledTiming::OnAnyDigimonPlayed))
        .expect("effect-play observer");
    assert!(!obs.optional && !obs.once_per_turn);
}

#[test]
fn ex5_062_digivolves_from_purple_lv5_for_4() {
    let mut r = setup();
    let base = r.place_on_field(0, "PURPLE5", Some(0));
    assert!(digivolve_onto(&mut r, base));
    assert_eq!(top_id(&r, base), CARD_ID);
    assert_eq!(r.memory(), 10 - 4);
}

#[test]
fn ex5_062_cannot_digivolve_from_red_lv5() {
    let mut r = setup();
    let base = r.place_on_field(0, "RED5", Some(0));
    assert!(!digivolve_onto(&mut r, base));
}

// ─── Section 2 — [When Digivolving]: trash up to 3, play with -3 -1/card ─────

#[test]
fn ex5_062_wd_trash_none_plays_purple_with_cost_reduced_by_3() {
    let mut r = setup();
    push_trash(&mut r, 0, "P6C");
    push_hand(&mut r, 0, "FILLER");
    anubis_by_digivolve(&mut r);
    let v = r.pending_selection_view().expect("hand-trash pick");
    assert!(v.is_optional, "\"You may trash up to 3\"");
    pass(&mut r, 0);
    let v = r.pending_selection_view().expect("trash play pick");
    assert!(!v.is_optional, "\"Then, play 1\" is mandatory");
    pick_trash(&mut r, 0, "P6C");
    drain_first(&mut r);
    assert!(field_ids(&r, 0).contains(&"P6C".to_string()));
    assert_eq!(trash_count(&r, "FILLER"), 0, "nothing trashed");
    assert_eq!(r.memory(), 6 - (6 - 3), "printed 6 reduced by 3");
}

#[test]
fn ex5_062_wd_trash_two_reduces_by_five() {
    // Official Q&A: trashing 2 cards reduces the play cost by 5.
    let mut r = setup();
    push_trash(&mut r, 0, "P9C");
    for _ in 0..3 {
        push_hand(&mut r, 0, "FILLER");
    }
    anubis_by_digivolve(&mut r);
    pick_hand(&mut r, 0, "FILLER");
    let v = r.pending_selection_view().expect("second hand pick");
    assert!(v.is_optional, "\"up to\" — may stop after any pick");
    pick_hand(&mut r, 0, "FILLER");
    pass(&mut r, 0); // stop at 2
    pick_trash(&mut r, 0, "P9C");
    drain_first(&mut r);
    assert!(field_ids(&r, 0).contains(&"P9C".to_string()));
    assert_eq!(trash_count(&r, "FILLER"), 2, "2 trashed");
    assert_eq!(r.memory(), 6 - (9 - 5));
}

#[test]
fn ex5_062_wd_trash_three_reduces_by_six_and_stops_at_three() {
    let mut r = setup();
    push_trash(&mut r, 0, "P9C");
    for _ in 0..4 {
        push_hand(&mut r, 0, "FILLER");
    }
    anubis_by_digivolve(&mut r);
    pick_hand(&mut r, 0, "FILLER");
    pick_hand(&mut r, 0, "FILLER");
    pick_hand(&mut r, 0, "FILLER");
    let v = r.pending_selection_view().expect("next prompt is the trash play");
    assert!(!v.is_optional, "no fourth hand pick: it goes straight to the play");
    pick_trash(&mut r, 0, "P9C");
    drain_first(&mut r);
    assert_eq!(trash_count(&r, "FILLER"), 3, "exactly 3 trashed");
    assert_eq!(r.memory(), 6 - (9 - 6));
}

#[test]
fn ex5_062_wd_one_trashed_reduces_by_four_floor_zero() {
    let mut r = setup();
    push_trash(&mut r, 0, "P2C");
    push_hand(&mut r, 0, "FILLER");
    push_hand(&mut r, 0, "FILLER");
    anubis_by_digivolve(&mut r);
    pick_hand(&mut r, 0, "FILLER");
    pass(&mut r, 0);
    pick_trash(&mut r, 0, "P2C");
    drain_first(&mut r);
    assert!(field_ids(&r, 0).contains(&"P2C".to_string()));
    assert_eq!(r.memory(), 6, "cost 2 reduced by 4 is 0");
}

#[test]
fn ex5_062_wd_only_purple_digimon_cards_are_playable() {
    let mut r = setup();
    for id in ["P6C", "R6C", "PTAMER"] {
        push_trash(&mut r, 0, id);
    }
    anubis_by_digivolve(&mut r);
    pass(&mut r, 0); // decline trashing the card the digivolve drew
    assert_eq!(non_pass(&r).len(), 1, "only the purple Digimon card (not red, not a Tamer)");
    pick_trash(&mut r, 0, "P6C");
    drain_first(&mut r);
    assert!(field_ids(&r, 0).contains(&"P6C".to_string()));
}

#[test]
fn ex5_062_wd_a_card_trashed_by_it_can_be_the_played_card() {
    let mut r = setup();
    push_hand(&mut r, 0, "P6C");
    anubis_by_digivolve(&mut r);
    pick_hand(&mut r, 0, "P6C");
    pass(&mut r, 0);
    pick_trash(&mut r, 0, "P6C");
    drain_first(&mut r);
    assert!(field_ids(&r, 0).contains(&"P6C".to_string()));
    assert_eq!(r.memory(), 6 - (6 - 4), "1 trashed: -4");
}

#[test]
fn ex5_062_wd_no_purple_digimon_in_trash_plays_nothing() {
    let mut r = setup();
    push_trash(&mut r, 0, "R6C");
    push_hand(&mut r, 0, "FILLER");
    anubis_by_digivolve(&mut r);
    pick_hand(&mut r, 0, "FILLER");
    pass(&mut r, 0);
    drain_first(&mut r);
    assert!(r.pending_selection_view().is_none());
    assert_eq!(r.battle_area_size(0), 1, "only Anubismon");
    assert_eq!(r.memory(), 6);
}

// ─── Section 2b — the play pick offers only payable cards (CR 4.3 §1-3-11-1) ─
//
// DCGO's pick filter is CanPlayAsNewPermanent(payCost: true) with the
// -(3 + trashed) ChangeCostClass registered: a card whose reduced cost would
// take memory past the -10 floor is not offered. Otherwise a player could
// pick it, fail the payment, and dodge the mandatory play.

/// Player 0's memory, whoever's turn it is (memory below 0 passes the turn
/// once the effects resolve, and `r.memory()` is turn-player relative).
fn p0_memory(r: &DebugRunner) -> i16 {
    if r.game.turn_player() == 0 {
        r.memory()
    } else {
        -r.memory()
    }
}

/// Memory 2 before the 4-memory digivolve → -2 when [When Digivolving] fires.
fn low_memory_digivolve(r: &mut DebugRunner) {
    r.game.memory = 2;
    anubis_by_digivolve(r);
    assert_eq!(r.memory(), -2);
}

#[test]
fn ex5_062_wd_low_memory_offers_only_the_affordable_card_and_forces_the_play() {
    let mut r = setup();
    push_trash(&mut r, 0, "P12C"); // 12 - 3 = 9 → -11: past the floor
    push_trash(&mut r, 0, "P6C"); //  6 - 3 = 3 → -5
    low_memory_digivolve(&mut r);
    pass(&mut r, 0); // decline trashing the card the digivolve drew
    let v = r.pending_selection_view().expect("the play pick");
    assert!(!v.is_optional, "the play is mandatory");
    assert_eq!(
        non_pass(&r).len(),
        1,
        "only the payable purple Digimon is offered (not the cost-12 one)"
    );
    pick_trash(&mut r, 0, "P6C");
    drain_first(&mut r);
    assert!(field_ids(&r, 0).contains(&"P6C".to_string()));
    assert_eq!(trash_count(&r, "P12C"), 1, "the unpayable card stays in the trash");
    assert_eq!(p0_memory(&r), -2 - (6 - 3));
}

#[test]
fn ex5_062_wd_low_memory_trashing_three_makes_the_expensive_card_payable() {
    let mut r = setup();
    push_trash(&mut r, 0, "P12C"); // 12 - 6 = 6 → -8: payable after 3 trashed
    push_trash(&mut r, 0, "P6C");
    for _ in 0..3 {
        push_hand(&mut r, 0, "FILLER");
    }
    low_memory_digivolve(&mut r);
    pick_hand(&mut r, 0, "FILLER");
    pick_hand(&mut r, 0, "FILLER");
    pick_hand(&mut r, 0, "FILLER");
    let v = r.pending_selection_view().expect("the play pick");
    assert!(!v.is_optional);
    assert_eq!(non_pass(&r).len(), 2, "-6: both purple Digimon are payable");
    pick_trash(&mut r, 0, "P12C");
    drain_first(&mut r);
    assert!(field_ids(&r, 0).contains(&"P12C".to_string()));
    assert_eq!(p0_memory(&r), -2 - (12 - 6));
}

#[test]
fn ex5_062_wd_only_unpayable_purple_digimon_skips_the_play() {
    let mut r = setup();
    push_trash(&mut r, 0, "P12C");
    low_memory_digivolve(&mut r);
    pass(&mut r, 0); // decline the hand trash: -3 leaves P12C unpayable
    assert!(
        r.pending_selection_view().is_none(),
        "no payable card: the play pick is skipped (not offered and then failed)"
    );
    drain_first(&mut r);
    assert_eq!(trash_count(&r, "P12C"), 1);
    assert_eq!(r.battle_area_size(0), 1, "only Anubismon");
    assert_eq!(p0_memory(&r), -2);
}

// ─── Section 3 — [Main] + the shared [Once Per Turn] ─────────────────────────

#[test]
fn ex5_062_main_activates_same_effect() {
    let mut r = setup();
    let me = r.place_on_field(0, CARD_ID, Some(0));
    push_trash(&mut r, 0, "P6C");
    push_hand(&mut r, 0, "FILLER");
    assert!(r.game.activate_field_main(0, me.index as usize), "[Main] activates");
    pick_hand(&mut r, 0, "FILLER");
    // The hand is now empty: no second hand prompt.
    pick_trash(&mut r, 0, "P6C");
    drain_first(&mut r);
    assert!(field_ids(&r, 0).contains(&"P6C".to_string()));
    assert_eq!(r.memory(), 10 - (6 - 4));
    // Same turn: [Once Per Turn].
    push_hand(&mut r, 0, "FILLER");
    push_trash(&mut r, 0, "P9C");
    assert!(
        !r.game.activate_field_main(0, find(&r, 0, CARD_ID).index as usize),
        "[Main] is spent for the turn"
    );
}

#[test]
fn ex5_062_when_digivolving_spends_the_main_once_per_turn() {
    let mut r = setup();
    push_trash(&mut r, 0, "P6C");
    let me = anubis_by_digivolve(&mut r);
    drain_first(&mut r);
    push_hand(&mut r, 0, "FILLER");
    push_trash(&mut r, 0, "P9C");
    assert!(
        !r.game.activate_field_main(0, me.index as usize),
        "[When Digivolving] and [Main] share one [Once Per Turn]"
    );
}

#[test]
fn ex5_062_main_needs_a_hand_card_or_a_purple_digimon_in_trash() {
    let mut r = setup();
    let me = r.place_on_field(0, CARD_ID, Some(0));
    push_trash(&mut r, 0, "R6C");
    assert!(
        !r.game.activate_field_main(0, me.index as usize),
        "empty hand and no purple Digimon in the trash: nothing to activate"
    );
}

#[test]
fn ex5_062_main_with_only_a_hand_card_is_activatable() {
    let mut r = setup();
    let me = r.place_on_field(0, CARD_ID, Some(0));
    push_hand(&mut r, 0, "FILLER");
    assert!(r.game.activate_field_main(0, me.index as usize));
    pick_hand(&mut r, 0, "FILLER");
    drain_first(&mut r);
    assert!(trash_ids(&r, 0).contains(&"FILLER".to_string()));
}

#[test]
fn ex5_062_main_with_only_a_purple_digimon_in_trash_is_activatable() {
    let mut r = setup();
    let me = r.place_on_field(0, CARD_ID, Some(0));
    push_trash(&mut r, 0, "P6C");
    assert_eq!(r.hand_size(0), 0);
    assert!(r.game.activate_field_main(0, me.index as usize));
    pick_trash(&mut r, 0, "P6C");
    drain_first(&mut r);
    assert!(field_ids(&r, 0).contains(&"P6C".to_string()));
    assert_eq!(r.memory(), 10 - (6 - 3));
}

#[test]
fn ex5_062_main_trash_gate_counts_only_a_card_payable_at_minus_3() {
    // Empty hand: the reduction is exactly 3. A cost-14 purple Digimon at
    // memory 0 would cost 11 (→ -11, past the floor): nothing to play.
    let mut r = setup();
    r.game.memory = 0;
    let me = r.place_on_field(0, CARD_ID, Some(0));
    push_trash(&mut r, 0, "P14C");
    assert!(
        !r.game.activate_field_main(0, me.index as usize),
        "no hand card and no purple Digimon payable at -3"
    );
    // A cost-12 one costs 9 at -3 (→ -9): payable, so [Main] activates.
    push_trash(&mut r, 0, "P12C");
    assert!(r.game.activate_field_main(0, me.index as usize));
    assert_eq!(non_pass(&r).len(), 1, "only the payable card is offered");
    pick_trash(&mut r, 0, "P12C");
    drain_first(&mut r);
    assert!(field_ids(&r, 0).contains(&"P12C".to_string()));
    assert_eq!(p0_memory(&r), -9);
}

#[test]
fn ex5_062_main_resets_next_turn() {
    let mut r = setup();
    let me = r.place_on_field(0, CARD_ID, Some(0));
    push_hand(&mut r, 0, "FILLER");
    assert!(r.game.activate_field_main(0, me.index as usize));
    drain_first(&mut r);
    r.end_turn();
    let _ = r.auto_resolve();
    r.end_turn();
    let _ = r.auto_resolve();
    assert_eq!(r.game.turn_player(), 0);
    r.game.current_phase = GamePhase::Main;
    push_hand(&mut r, 0, "FILLER");
    assert!(
        r.game.activate_field_main(0, find(&r, 0, CARD_ID).index as usize),
        "a new turn re-arms the once-per-turn"
    );
}

// ─── Section 4 — [Your Turn] effect plays your Digimon → delete / draw ───────

fn effect_play(r: &mut DebugRunner, p: u8, id: &str) {
    push_trash(r, p, id);
    let t = r.place_on_field(p, "T-EFFPLAY", Some(0));
    fire(r, EffectTiming::OnPlay, t);
    pick_trash(r, p, id);
}

#[test]
fn ex5_062_effect_play_deletes_opponent_level_5_or_lower() {
    let mut r = setup();
    r.place_on_field(0, CARD_ID, Some(0));
    let o5 = r.place_on_field(1, "OPP5", Some(0));
    r.place_on_field(1, "OPP6", Some(0));
    let deck_before = r.deck_size(0);
    effect_play(&mut r, 0, "P6C");
    let v = r.pending_selection_view().expect("delete pick");
    assert!(!v.is_optional, "mandatory delete");
    assert_eq!(non_pass(&r).len(), 1, "only the level 5");
    pick_side_field(&mut r, 0, o5);
    drain_first(&mut r);
    assert_eq!(field_ids(&r, 1), vec!["OPP6".to_string()]);
    assert_eq!(r.deck_size(0), deck_before, "it deleted ⇒ no draw");
}

#[test]
fn ex5_062_effect_play_with_nothing_to_delete_draws() {
    let mut r = setup();
    r.place_on_field(0, CARD_ID, Some(0));
    r.place_on_field(1, "OPP6", Some(0));
    let deck_before = r.deck_size(0);
    effect_play(&mut r, 0, "P6C");
    drain_first(&mut r);
    assert_eq!(field_ids(&r, 1), vec!["OPP6".to_string()]);
    assert_eq!(r.deck_size(0), deck_before - 1, "didn't delete ⇒ <Draw 1>");
}

#[test]
fn ex5_062_play_from_hand_is_not_an_effect_play() {
    let mut r = setup();
    r.place_on_field(0, CARD_ID, Some(0));
    r.place_on_field(1, "OPP5", Some(0));
    let deck_before = r.deck_size(0);
    let idx = r.add_to_hand(0, "P2C");
    r.play(0, idx).expect("P2C plays");
    let _ = r.auto_resolve();
    assert!(r.pending_selection_view().is_none());
    assert_eq!(field_ids(&r, 1), vec!["OPP5".to_string()]);
    assert_eq!(r.deck_size(0), deck_before);
}

#[test]
fn ex5_062_not_on_opponents_turn() {
    let mut r = setup();
    r.place_on_field(0, CARD_ID, Some(0));
    r.place_on_field(1, "OPP5", Some(0));
    r.game.turn_player_idx = 1;
    let deck_before = r.deck_size(0);
    effect_play(&mut r, 0, "P6C");
    drain_first(&mut r);
    assert_eq!(field_ids(&r, 1), vec!["OPP5".to_string()], "[Your Turn] only");
    assert_eq!(r.deck_size(0), deck_before);
}

#[test]
fn ex5_062_effect_playing_an_opponents_digimon_does_not_trigger() {
    let mut r = setup();
    r.place_on_field(0, CARD_ID, Some(0));
    let o5 = r.place_on_field(1, "OPP5", Some(0));
    let deck_before = r.deck_size(0);
    effect_play(&mut r, 1, "P6C");
    drain_first(&mut r);
    assert!(field_ids(&r, 1).contains(&"OPP5".to_string()), "{o5:?} survives");
    assert_eq!(r.deck_size(0), deck_before, "not one of YOUR Digimon");
}

#[test]
fn ex5_062_its_own_play_triggers_the_delete() {
    // Combo: the [When Digivolving] play is an effect play of your Digimon.
    let mut r = setup();
    let o5 = r.place_on_field(1, "OPP5", Some(0));
    push_trash(&mut r, 0, "P6C");
    anubis_by_digivolve(&mut r);
    pass(&mut r, 0); // decline trashing the card the digivolve drew
    pick_trash(&mut r, 0, "P6C");
    let v = r.pending_selection_view().expect("delete pick after the play");
    assert!(!v.is_optional);
    pick_side_field(&mut r, 0, o5);
    drain_first(&mut r);
    assert!(field_ids(&r, 1).is_empty());
}
