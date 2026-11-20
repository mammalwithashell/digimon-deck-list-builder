//! BT17-080 Takato Matsuki -- Tamer, Red, Cost 3.
//!
//! # Card text (official Bandai DB, data/card_bundles/BT17-080.md)
//!
//! [Start of Your Main Phase] If you have a Digimon with [Guilmon], [Growlmon]
//! or [Gallantmon] in its name, gain 1 memory.
//! [End of Your Turn] By placing this Tamer and 1 [Growlmon] and 1
//! [WarGrowlmon] from your trash as 1 of your [Guilmon]'s bottom digivolution
//! cards, that Digimon may digivolve into [Gallantmon] in the hand, ignoring
//! its digivolution requirements and without paying the cost.
//! [Security] Play this card without paying the cost.
//! Official Q&A: the digivolve is a "may" (you can choose not to digivolve).
//!
//! DCGO: BT17/*/BT17_080.cs. The hunt-2 replay of two real games ended the turn
//! one action early because the mandatory [Start of Your Main Phase] memory
//! gain never fired (the card was unimplemented).
//!
//! New substrate: `is_source_card` (G-DSL-IS-SOURCE-CARD) and `card_in_zone`
//! (G-DSL-CARD-IN-ZONE) so ONE ordered `select_zone_cards` pick can range over
//! this Tamer and the two trash cards.

#[path = "../bt26/support.rs"]
mod support;

use digimon_dsl::compiled::{CompiledClause, CompiledTiming};
use digimon_engine::action::space::{encode_source_select, PASS, TRASH_EFFECT_START};
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::{CardColor, EffectTiming};
use digimon_engine::permanent::PermanentHandle;
use support::*;

const CARD_ID: &str = "BT17-080";

fn builder() -> digimon_engine::debug_runner::DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT17-080 must load from the embedded DSL pack")
        .add_card(filler("FILLER"))
        .add_card(digimon("GUILMON", "Guilmon", CardColor::Red, 3, 3, &[]))
        .add_card(digimon("GROWLMON", "Growlmon", CardColor::Red, 4, 5, &[]))
        .add_card(digimon("WARGROWLMON", "WarGrowlmon", CardColor::Red, 5, 7, &[]))
        .add_card(digimon("GALLANTMON", "Gallantmon", CardColor::Red, 6, 12, &[]))
        // Decoys that must never be pickable / matching.
        .add_card(digimon("GUILMON-X", "Guilmon (X Antibody)", CardColor::Red, 3, 3, &[]))
        .add_card(digimon("GALLANTMON-CM", "Gallantmon: Crimson Mode", CardColor::Red, 6, 12, &[]))
        .add_card(digimon("GALLANTMON-L5", "Gallantmon", CardColor::Red, 5, 8, &[]))
        .add_card(digimon("OTHER", "Agumon", CardColor::Red, 3, 3, &[]))
        .add_card(tamer("OTHER-T", "Other Tamer", CardColor::Red, &[]))
        .deck(0, &["FILLER"; 8])
        .deck(1, &["FILLER"; 8])
        .memory(5)
}

fn place_tamer(r: &mut DebugRunner) -> PermanentHandle {
    r.place_on_field(0, CARD_ID, Some(0))
}

// ─── Structure ───────────────────────────────────────────────────────────────

#[test]
fn bt17_080_clause_shape_matches_the_card_text() {
    let r = builder().start();
    let card = r.compiled_card(CARD_ID).expect("compiled card");
    let triggered: Vec<_> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();
    assert_eq!(triggered.len(), 3, "start-of-main, end-of-turn, security");
    let start = triggered
        .iter()
        .find(|t| t.when.contains(&CompiledTiming::StartOfYourMainPhase))
        .expect("start-of-main clause");
    assert!(!start.optional, "the memory gain is mandatory");
    let eot = triggered
        .iter()
        .find(|t| t.when.contains(&CompiledTiming::EndOfYourTurn))
        .expect("[End of Your Turn] clause");
    assert!(eot.optional, "DCGO isOptional: true on the end-of-turn effect");
    assert!(triggered
        .iter()
        .any(|t| t.when.contains(&CompiledTiming::OnSecurity)));
}

// ─── Clause 1: [Start of Your Main Phase] ───────────────────────────────────

fn start_of_main_memory_delta(setup: impl FnOnce(&mut DebugRunner)) -> i16 {
    let mut r = builder().start();
    r.set_first_player(0);
    place_tamer(&mut r);
    setup(&mut r);
    let before = r.memory();
    r.game.enter_main_phase();
    let _ = r.auto_resolve();
    r.memory() - before
}

/// The hunt-2 regression: a [Guilmon]-line Digimon makes the main phase open
/// with +1 memory.
#[test]
fn bt17_080_start_of_main_gains_one_memory_with_a_guilmon_line_digimon() {
    for id in ["GUILMON", "GROWLMON", "GALLANTMON", "GUILMON-X", "GALLANTMON-CM"] {
        let delta = start_of_main_memory_delta(|r| {
            r.place_on_field(0, id, Some(0));
        });
        assert_eq!(delta, 1, "a Digimon with {id}'s name must enable the gain");
    }
}

#[test]
fn bt17_080_start_of_main_gains_nothing_without_a_matching_digimon() {
    let none = start_of_main_memory_delta(|_| {});
    assert_eq!(none, 0, "no Digimon at all");
    let other = start_of_main_memory_delta(|r| {
        r.place_on_field(0, "OTHER", Some(0));
    });
    assert_eq!(other, 0, "an unrelated Digimon does not count");
    // The opponent's [Guilmon] is not "a Digimon you have".
    let opp = start_of_main_memory_delta(|r| {
        r.place_on_field(1, "GUILMON", Some(0));
    });
    assert_eq!(opp, 0, "the Digimon must be your own");
}

// ─── Clause 2: [End of Your Turn] ───────────────────────────────────────────

/// P0: Takato (field slot 0), Guilmon (slot 1); trash [Growlmon] and
/// [WarGrowlmon]; hand [Gallantmon].
fn eot_setup() -> (DebugRunner, PermanentHandle, PermanentHandle) {
    let mut r = builder().start();
    r.set_first_player(0);
    let tamer_h = place_tamer(&mut r);
    let guilmon = r.place_on_field(0, "GUILMON", Some(0));
    push_trash(&mut r, 0, "GROWLMON");
    push_trash(&mut r, 0, "WARGROWLMON");
    push_hand(&mut r, 0, "GALLANTMON");
    (r, tamer_h, guilmon)
}

fn trash_action(r: &DebugRunner, id: &str) -> u16 {
    TRASH_EFFECT_START + trash_ids(r, 0).iter().position(|t| t == id).unwrap() as u16
}

/// Fire the real [End of Your Turn] timing for the Tamer.
fn fire_eot(r: &mut DebugRunner, tamer_h: PermanentHandle) {
    fire(r, EffectTiming::EndOfYourTurn, tamer_h);
}

/// Pick the three placed cards in `order` (ids: "TAMER", "GROWLMON",
/// "WARGROWLMON"). The Tamer is addressed by its battle-area top card.
fn pick_materials(r: &mut DebugRunner, tamer_h: PermanentHandle, order: &[&str]) {
    for id in order {
        let action = match *id {
            "TAMER" => encode_source_select(tamer_h.index as u16, 0).unwrap(),
            other => trash_action(r, other),
        };
        let v = r.pending_selection_view().expect("material pick pending");
        assert!(
            v.valid_action_ids.contains(&action),
            "{id} must be a legal material pick (valid {:?})",
            v.valid_action_ids
        );
        r.execute_action(0, action).unwrap();
    }
}

/// End-to-end: accept, pick the Guilmon, place Tamer + Growlmon + WarGrowlmon
/// in the chosen order, then digivolve into the hand [Gallantmon] free.
#[test]
fn bt17_080_end_of_turn_places_three_cards_under_guilmon_and_digivolves_into_gallantmon() {
    let (mut r, tamer_h, guilmon) = eot_setup();
    let memory_before = r.memory();
    fire_eot(&mut r, tamer_h);

    assert!(r.pending_is_optional(), "[End of Your Turn] is a \"may\" trigger");
    r.accept_optional_trigger().expect("accept");

    // Guilmon pick (the only candidate).
    let v = r.pending_selection_view().expect("Guilmon pick");
    assert!(!v.is_optional, "once accepted the picks are mandatory");
    pick_first(&mut r, 0);

    // One ordered pick of exactly the three materials.
    pick_materials(&mut r, tamer_h, &["WARGROWLMON", "TAMER", "GROWLMON"]);

    // Optional free digivolve into the hand [Gallantmon].
    let v = r.pending_selection_view().expect("Gallantmon pick");
    assert!(v.is_optional, "\"may digivolve\" (official Q&A)");
    pick_hand(&mut r, 0, "GALLANTMON");
    let _ = r.auto_resolve();

    let host = find_top(&r, "GALLANTMON");
    assert_eq!(top_id(&r, host), "GALLANTMON");
    // The stack is the Guilmon, then the three placed cards below it. The
    // first pick is the topmost of the three (CR 4.3 3-1-3-3).
    let ids = source_ids(&r, host);
    for id in [CARD_ID, "GROWLMON", "WARGROWLMON", "GUILMON"] {
        assert!(ids.contains(&id.to_string()), "{id} in the stack: {ids:?}");
    }
    let pos = |id: &str| ids.iter().position(|s| s == id).unwrap();
    // `card_sources` is bottom-first: the LAST pick is the bottom-most.
    assert!(
        pos("GROWLMON") < pos(CARD_ID) && pos(CARD_ID) < pos("WARGROWLMON"),
        "picked WarGrowlmon, Takato, Growlmon -> WarGrowlmon is the highest of \
         the three and Growlmon the bottom card: {ids:?}"
    );
    assert!(
        !field_ids(&r, 0).contains(&CARD_ID.to_string()),
        "the Tamer left the field as a digivolution card"
    );
    assert!(
        trash_ids(&r, 0).is_empty(),
        "Growlmon and WarGrowlmon left the trash: {:?}",
        trash_ids(&r, 0)
    );
    assert!(
        !hand_ids(&r, 0).contains(&"GALLANTMON".to_string()),
        "Gallantmon left the hand (the hand now holds only the digivolve draw): {:?}",
        hand_ids(&r, 0)
    );
    assert_eq!(
        r.memory(),
        memory_before,
        "ignoring requirements and without paying the cost: memory unchanged"
    );
    let _ = guilmon;
}

/// The ordering is a real choice: the opposite pick order stacks the other way.
#[test]
fn bt17_080_the_pick_order_decides_the_stack_order() {
    let (mut r, tamer_h, _) = eot_setup();
    fire_eot(&mut r, tamer_h);
    r.accept_optional_trigger().unwrap();
    pick_first(&mut r, 0);
    pick_materials(&mut r, tamer_h, &["GROWLMON", "WARGROWLMON", "TAMER"]);
    pick_hand(&mut r, 0, "GALLANTMON");
    let _ = r.auto_resolve();
    let host = find_top(&r, "GALLANTMON");
    let ids = source_ids(&r, host);
    let pos = |id: &str| ids.iter().position(|s| s == id).unwrap();
    assert!(
        pos(CARD_ID) < pos("WARGROWLMON") && pos("WARGROWLMON") < pos("GROWLMON"),
        "first pick Growlmon is on top of the three, the Tamer last (bottom): {ids:?}"
    );
}

/// Official Q&A: "may digivolve" -- the cards stay placed if you decline.
#[test]
fn bt17_080_declining_the_digivolve_keeps_the_cards_placed() {
    let (mut r, tamer_h, guilmon) = eot_setup();
    fire_eot(&mut r, tamer_h);
    r.accept_optional_trigger().unwrap();
    pick_first(&mut r, 0);
    pick_materials(&mut r, tamer_h, &["TAMER", "GROWLMON", "WARGROWLMON"]);
    pass(&mut r, 0);
    let _ = r.auto_resolve();

    let g = find_top(&r, "GUILMON");
    assert_eq!(top_id(&r, g), "GUILMON", "no digivolve happened");
    let ids = source_ids(&r, g);
    for id in [CARD_ID, "GROWLMON", "WARGROWLMON"] {
        assert!(ids.contains(&id.to_string()), "{id} stays placed: {ids:?}");
    }
    assert_eq!(hand_ids(&r, 0), vec!["GALLANTMON".to_string()]);
    let _ = guilmon;
}

#[test]
fn bt17_080_declining_the_trigger_places_nothing() {
    let (mut r, tamer_h, _) = eot_setup();
    fire_eot(&mut r, tamer_h);
    r.decline_optional_trigger().unwrap();
    let _ = r.auto_resolve();
    assert!(field_ids(&r, 0).contains(&CARD_ID.to_string()));
    assert_eq!(trash_ids(&r, 0).len(), 2);
}

/// Every requirement the effect names is checked before it can be activated.
#[test]
fn bt17_080_end_of_turn_requires_every_named_card() {
    // No hand [Gallantmon].
    let (mut r, tamer_h, _) = eot_setup();
    r.game.players[0].hand.clear();
    fire_eot(&mut r, tamer_h);
    assert!(r.pending_selection().is_none(), "no Gallantmon in hand");

    // The hand Gallantmon is level 5 (DCGO requires Level == 6).
    let mut r = builder().start();
    r.set_first_player(0);
    let tamer_h = place_tamer(&mut r);
    r.place_on_field(0, "GUILMON", Some(0));
    push_trash(&mut r, 0, "GROWLMON");
    push_trash(&mut r, 0, "WARGROWLMON");
    push_hand(&mut r, 0, "GALLANTMON-L5");
    fire_eot(&mut r, tamer_h);
    assert!(r.pending_selection().is_none(), "level 5 Gallantmon");

    // A differently named Gallantmon is not [Gallantmon].
    let mut r = builder().start();
    r.set_first_player(0);
    let tamer_h = place_tamer(&mut r);
    r.place_on_field(0, "GUILMON", Some(0));
    push_trash(&mut r, 0, "GROWLMON");
    push_trash(&mut r, 0, "WARGROWLMON");
    push_hand(&mut r, 0, "GALLANTMON-CM");
    fire_eot(&mut r, tamer_h);
    assert!(r.pending_selection().is_none(), "Gallantmon: Crimson Mode");

    // Missing WarGrowlmon in the trash.
    let (mut r, tamer_h, _) = eot_setup();
    r.game.players[0]
        .trash
        .retain(|c| c.card_id(&r.game.card_data) != "WARGROWLMON");
    fire_eot(&mut r, tamer_h);
    assert!(r.pending_selection().is_none(), "no WarGrowlmon in trash");

    // Missing Growlmon in the trash.
    let (mut r, tamer_h, _) = eot_setup();
    r.game.players[0]
        .trash
        .retain(|c| c.card_id(&r.game.card_data) != "GROWLMON");
    fire_eot(&mut r, tamer_h);
    assert!(r.pending_selection().is_none(), "no Growlmon in trash");

    // The Digimon must be named exactly [Guilmon].
    let mut r = builder().start();
    r.set_first_player(0);
    let tamer_h = place_tamer(&mut r);
    r.place_on_field(0, "GUILMON-X", Some(0));
    push_trash(&mut r, 0, "GROWLMON");
    push_trash(&mut r, 0, "WARGROWLMON");
    push_hand(&mut r, 0, "GALLANTMON");
    fire_eot(&mut r, tamer_h);
    assert!(r.pending_selection().is_none(), "Guilmon (X Antibody) is not [Guilmon]");
}

/// Only the trash cards count: a [Growlmon] / [WarGrowlmon] on the FIELD is
/// never a legal material (the pick is over this Tamer and the TRASH).
#[test]
fn bt17_080_field_growlmon_is_not_a_legal_material() {
    let (mut r, tamer_h, _) = eot_setup();
    let field_growl = r.place_on_field(0, "GROWLMON", Some(0));
    fire_eot(&mut r, tamer_h);
    r.accept_optional_trigger().unwrap();
    pick_first(&mut r, 0);
    let v = r.pending_selection_view().expect("material pick");
    let field_top = encode_source_select(field_growl.index as u16, 0).unwrap();
    assert!(
        !v.valid_action_ids.contains(&field_top),
        "a Growlmon on the field is not a trash card"
    );
    assert!(
        v.valid_action_ids
            .contains(&encode_source_select(tamer_h.index as u16, 0).unwrap()),
        "this Tamer is a legal material"
    );
}

/// "This Tamer": another copy of the Tamer (field or trash) is not it.
#[test]
fn bt17_080_only_this_tamer_is_the_tamer_material() {
    let (mut r, tamer_h, _) = eot_setup();
    let second = place_tamer(&mut r);
    push_trash(&mut r, 0, CARD_ID);
    fire(&mut r, EffectTiming::EndOfYourTurn, tamer_h);
    r.accept_optional_trigger().unwrap();
    pick_first(&mut r, 0);
    let v = r.pending_selection_view().expect("material pick");
    let this = encode_source_select(tamer_h.index as u16, 0).unwrap();
    let other_field = encode_source_select(second.index as u16, 0).unwrap();
    let other_trash = trash_action(&r, CARD_ID);
    assert!(v.valid_action_ids.contains(&this));
    assert!(
        !v.valid_action_ids.contains(&other_field),
        "a second copy of the Tamer on the field is not THIS Tamer"
    );
    assert!(
        !v.valid_action_ids.contains(&other_trash),
        "a copy of the Tamer in the trash is not THIS Tamer"
    );
    assert!(!v.valid_action_ids.contains(&PASS), "mandatory once accepted");
}

fn find_top(r: &DebugRunner, card_id: &str) -> PermanentHandle {
    let idx = r.game.players[0]
        .battle_area
        .iter()
        .position(|p| p.top_card().card_id(&r.game.card_data) == card_id)
        .unwrap_or_else(|| panic!("{card_id} not on P0's field"));
    PermanentHandle {
        player: 0,
        index: idx as u8,
    }
}
