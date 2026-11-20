//! BT22-093 Ami Aiba -- Tamer, White, Cost 4, traits: [CS].
//!
//! # Card text (official Bandai DB, data/card_bundles/BT22-093.md)
//!
//! [Start of Your Main Phase] If your opponent has a Digimon, gain 1 memory.
//! [Your Turn] When any of your Digimon digivolve into a Digimon with the [CS]
//! trait, if it has a digivolution card with the same level as the digivolved
//! Digimon, by suspending this Tamer, that Digimon may digivolve into a Digimon
//! card with the [CS] trait in the hand without paying the cost.
//! [Security] Play this card without paying the cost.
//!
//! DCGO: BT22/*/BT22_093.cs. The hunt-2 replay of two real games ended the turn
//! one action early because this Tamer's mandatory [Start of Your Main Phase]
//! memory gain never fired (the card was unimplemented).
//!
//! New substrate: `event_target_has_same_level_source`
//! (G-DSL-EVENT-TARGET-HAS-SAME-LEVEL-SOURCE).

#[path = "../bt26/support.rs"]
mod support;

use digimon_dsl::compiled::{CompiledClause, CompiledTiming};
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::{CardColor, PlaySource};
use digimon_engine::permanent::PermanentHandle;
use support::*;

const CARD_ID: &str = "BT22-093";

fn builder() -> digimon_engine::debug_runner::DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT22-093 must load from the embedded DSL pack")
        .add_card(filler("FILLER"))
        .add_card(digimon("OPP-D", "Opp Digimon", CardColor::Red, 4, 4, &[]))
        .add_card(digimon("BASE-L4", "Base L4", CardColor::White, 4, 4, &[]))
        // The digivolve target: a Lv.5 [CS] Digimon digivolving from a Lv.4.
        .add_card(digimon_evo(
            "CS-L5",
            "CS Lv5",
            CardColor::White,
            5,
            5,
            &["CS"],
            4,
            0,
        ))
        // Same level as CS-L5 -- the "digivolution card with the same level".
        .add_card(digimon("SRC-L5", "Source L5", CardColor::White, 5, 5, &[]))
        .add_card(digimon("SRC-L3", "Source L3", CardColor::White, 3, 3, &[]))
        // The free hand digivolve: a Lv.6 [CS] Digimon digivolving from Lv.5.
        .add_card(digimon_evo(
            "CS-L6",
            "CS Lv6",
            CardColor::White,
            6,
            9,
            &["CS"],
            5,
            9,
        ))
        // A Lv.6 NON-[CS] Digimon: not eligible for the free digivolve.
        .add_card(digimon_evo(
            "PLAIN-L6",
            "Plain Lv6",
            CardColor::White,
            6,
            9,
            &[],
            5,
            9,
        ))
        // Lv.5 non-[CS] result: the digivolve trigger gate must reject it.
        .add_card(digimon_evo(
            "PLAIN-L5",
            "Plain Lv5",
            CardColor::White,
            5,
            5,
            &[],
            4,
            0,
        ))
        .deck(0, &["FILLER"; 8])
        .deck(1, &["FILLER"; 8])
        .memory(5)
}

// ─── Structure ───────────────────────────────────────────────────────────────

#[test]
fn bt22_093_clause_shape_matches_the_card_text() {
    let runner = builder().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled card");
    let triggered: Vec<_> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();
    assert_eq!(triggered.len(), 3, "start-of-main, on-digivolve, security");
    let start = triggered
        .iter()
        .find(|t| t.when.contains(&CompiledTiming::StartOfYourMainPhase))
        .expect("start-of-main clause");
    assert!(!start.optional, "the memory gain has no \"may\": mandatory");
    let evo = triggered
        .iter()
        .find(|t| t.when.contains(&CompiledTiming::OnDigivolve))
        .expect("[Your Turn] digivolve observer");
    assert!(evo.optional, "DCGO isOptional: true on the observer");
    assert!(triggered
        .iter()
        .any(|t| t.when.contains(&CompiledTiming::OnSecurity)));
}

// ─── Clause 1: [Start of Your Main Phase] gain 1 memory ─────────────────────

/// The hunt-2 regression: with an opponent Digimon in play the main phase
/// opens with +1 memory, so a following digivolve no longer pushes memory
/// negative one action early.
#[test]
fn bt22_093_start_of_main_gains_one_memory_when_opponent_has_a_digimon() {
    let mut r = builder().start();
    r.set_first_player(0);
    r.place_on_field(0, CARD_ID, Some(0));
    r.place_on_field(1, "OPP-D", Some(0));
    let before = r.memory();

    r.game.enter_main_phase();
    let _ = r.auto_resolve();

    assert_eq!(
        r.memory(),
        before + 1,
        "[Start of Your Main Phase] with an opposing Digimon gains exactly 1 memory"
    );
}

#[test]
fn bt22_093_start_of_main_gains_nothing_when_opponent_has_no_digimon() {
    let mut r = builder().start();
    r.set_first_player(0);
    r.place_on_field(0, CARD_ID, Some(0));
    let before = r.memory();

    r.game.enter_main_phase();
    let _ = r.auto_resolve();

    assert_eq!(
        r.memory(),
        before,
        "the memory gain is conditional on the opponent having a Digimon"
    );
}

/// A Tamer of the opponent is not a Digimon: the gate counts Digimon only.
#[test]
fn bt22_093_start_of_main_ignores_an_opponent_with_only_a_tamer() {
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .unwrap()
        .add_card(tamer("OPP-T", "Opp Tamer", CardColor::Red, &[]))
        .add_card(filler("FILLER"))
        .deck(0, &["FILLER"; 4])
        .deck(1, &["FILLER"; 4])
        .memory(5)
        .start();
    r.set_first_player(0);
    r.place_on_field(0, CARD_ID, Some(0));
    r.place_on_field(1, "OPP-T", Some(0));
    let before = r.memory();

    r.game.enter_main_phase();
    let _ = r.auto_resolve();

    assert_eq!(r.memory(), before);
}

// ─── Clause 2: [Your Turn] same-level digivolve into a [CS] Digimon ─────────

/// Stage P0: Ami Aiba, a Lv.4 base carrying `extra_source` as a digivolution
/// card, and `evo` + the free-digivolve candidates in hand; digivolve the base
/// into `evo`. Returns the Tamer and base handles.
fn digivolve_base_into(
    r: &mut DebugRunner,
    extra_source: Option<&str>,
    evo: &str,
) -> (PermanentHandle, PermanentHandle) {
    r.set_first_player(0);
    let tamer_h = r.place_on_field(0, CARD_ID, Some(0));
    let base = r.place_on_field(0, "BASE-L4", Some(0));
    if let Some(src) = extra_source {
        r.push_source(base, src);
    }
    push_hand(r, 0, evo);
    push_hand(r, 0, "CS-L6");
    push_hand(r, 0, "PLAIN-L6");
    let idx = r.game.players[0]
        .hand
        .iter()
        .position(|c| c.card_id(&r.game.card_data) == evo)
        .unwrap();
    assert!(
        r.game
            .digivolve_from_hand(0, idx, base.index as usize, PlaySource::ByDigivolve),
        "{evo} must digivolve onto BASE-L4"
    );
    (tamer_h, base)
}

#[test]
fn bt22_093_digivolve_observer_offers_a_free_cs_digivolve_when_a_same_level_source_exists() {
    let mut r = builder().start();
    let (tamer_h, base) = digivolve_base_into(&mut r, Some("SRC-L5"), "CS-L5");

    assert!(
        r.pending_is_optional(),
        "the observer is a \"by suspending\" optional trigger (DCGO isOptional)"
    );
    r.accept_optional_trigger().expect("accept the trigger");

    // The suspension is the activation cost: paid before the hand pick.
    assert!(
        r.game.players[0].battle_area[tamer_h.index as usize].is_suspended,
        "by suspending this Tamer"
    );
    // The digivolve itself is a \"may\": PASS is legal.
    let view = r.pending_selection_view().expect("hand pick");
    assert!(view.is_optional, "that Digimon MAY digivolve");
    let memory_before = r.memory();
    // Only the [CS] Lv.6 is offered; the non-[CS] Lv.6 is not.
    let hand_idx = |id: &str| {
        r.game.players[0]
            .hand
            .iter()
            .position(|c| c.card_id(&r.game.card_data) == id)
            .unwrap() as u16
    };
    let cs = digimon_engine::action::space::PLAY_HAND_START + hand_idx("CS-L6");
    let plain = digimon_engine::action::space::PLAY_HAND_START + hand_idx("PLAIN-L6");
    assert!(view.valid_action_ids.contains(&cs), "[CS] Lv.6 offered");
    assert!(
        !view.valid_action_ids.contains(&plain),
        "a non-[CS] Digimon must not be offered"
    );

    pick_hand(&mut r, 0, "CS-L6");
    let _ = r.auto_resolve();

    assert_eq!(top_id(&r, base), "CS-L6", "the base digivolved into CS-L6");
    assert_eq!(
        r.memory(),
        memory_before,
        "\"without paying the cost\": memory unchanged"
    );
}

/// Without a digivolution card of the digivolved Digimon's level, the
/// observer must not trigger at all.
#[test]
fn bt22_093_digivolve_observer_needs_a_same_level_digivolution_card() {
    let mut r = builder().start();
    // BASE-L4 (Lv.4) is the only source under the new Lv.5 top; SRC-L3 is Lv.3.
    let (tamer_h, _) = digivolve_base_into(&mut r, Some("SRC-L3"), "CS-L5");

    assert!(
        r.pending_selection().is_none(),
        "no digivolution card of level 5 -> no trigger"
    );
    assert!(!r.game.players[0].battle_area[tamer_h.index as usize].is_suspended);
}

/// The gate compares the sources against the digivolved Digimon's CURRENT
/// level: a Lv.4 source under a Lv.5 top does not count.
#[test]
fn bt22_093_a_lower_level_previous_top_does_not_satisfy_the_gate() {
    let mut r = builder().start();
    let _ = digivolve_base_into(&mut r, None, "CS-L5");
    assert!(
        r.pending_selection().is_none(),
        "the previous Lv.4 top is a digivolution card of a DIFFERENT level"
    );
}

/// A face-down digivolution card has no characteristics (CR 4.3 4-7-9), so a
/// face-down Lv.5 card does not satisfy "a digivolution card with the same
/// level as the digivolved Digimon".
#[test]
fn bt22_093_a_face_down_same_level_source_does_not_count() {
    let mut r = builder().start();
    r.set_first_player(0);
    r.place_on_field(0, CARD_ID, Some(0));
    let base = r.place_on_field(0, "BASE-L4", Some(0));
    r.push_source(base, "SRC-L5");
    // Flip the Lv.5 source face down.
    {
        let perm = &mut r.game.players[0].battle_area[base.index as usize];
        let under = perm.card_sources.len() - 1;
        for source in perm.card_sources[..under].iter_mut() {
            source.face_down = true;
        }
    }
    push_hand(&mut r, 0, "CS-L5");
    push_hand(&mut r, 0, "CS-L6");
    let idx = r.game.players[0]
        .hand
        .iter()
        .position(|c| c.card_id(&r.game.card_data) == "CS-L5")
        .unwrap();
    assert!(r
        .game
        .digivolve_from_hand(0, idx, base.index as usize, PlaySource::ByDigivolve));
    assert!(
        r.pending_selection().is_none(),
        "a face-down Lv.5 card is not a same-level digivolution card"
    );
}

/// The digivolved Digimon must have the [CS] trait.
#[test]
fn bt22_093_digivolve_observer_requires_the_result_to_be_cs() {
    let mut r = builder().start();
    let _ = digivolve_base_into(&mut r, Some("SRC-L5"), "PLAIN-L5");
    assert!(
        r.pending_selection().is_none(),
        "a non-[CS] result digivolve must not offer the trigger"
    );
}

#[test]
fn bt22_093_declining_the_observer_leaves_the_tamer_unsuspended() {
    let mut r = builder().start();
    let (tamer_h, base) = digivolve_base_into(&mut r, Some("SRC-L5"), "CS-L5");
    r.decline_optional_trigger().expect("decline");
    let _ = r.auto_resolve();
    assert!(!r.game.players[0].battle_area[tamer_h.index as usize].is_suspended);
    assert_eq!(top_id(&r, base), "CS-L5");
}

/// "By suspending ... may digivolve": after paying the suspension the player
/// may still decline the digivolve (PASS the hand pick). The cost stays paid.
#[test]
fn bt22_093_passing_the_hand_pick_keeps_the_suspension_and_does_not_digivolve() {
    let mut r = builder().start();
    let (tamer_h, base) = digivolve_base_into(&mut r, Some("SRC-L5"), "CS-L5");
    r.accept_optional_trigger().expect("accept");
    pass(&mut r, 0);
    let _ = r.auto_resolve();
    assert!(r.game.players[0].battle_area[tamer_h.index as usize].is_suspended);
    assert_eq!(top_id(&r, base), "CS-L5");
}

/// A suspended Tamer cannot pay the cost: the observer does not trigger.
#[test]
fn bt22_093_a_suspended_tamer_does_not_trigger() {
    let mut r = builder().start();
    r.set_first_player(0);
    let tamer_h = r.place_on_field(0, CARD_ID, Some(0));
    r.game.players[0].battle_area[tamer_h.index as usize].is_suspended = true;
    let base = r.place_on_field(0, "BASE-L4", Some(0));
    r.push_source(base, "SRC-L5");
    push_hand(&mut r, 0, "CS-L5");
    push_hand(&mut r, 0, "CS-L6");
    let idx = r.game.players[0]
        .hand
        .iter()
        .position(|c| c.card_id(&r.game.card_data) == "CS-L5")
        .unwrap();
    assert!(r
        .game
        .digivolve_from_hand(0, idx, base.index as usize, PlaySource::ByDigivolve));
    assert!(r.pending_selection().is_none());
}

/// [Your Turn] only: the opponent digivolving on their own turn is not P0's
/// trigger either way (the clause is also scoped to "your Digimon").
#[test]
fn bt22_093_does_not_trigger_for_the_opponents_digimon() {
    let mut r = builder().start();
    r.set_first_player(0);
    r.place_on_field(0, CARD_ID, Some(0));
    let base = r.place_on_field(1, "BASE-L4", Some(0));
    r.push_source(base, "SRC-L5");
    push_hand(&mut r, 1, "CS-L5");
    let idx = r.game.players[1]
        .hand
        .iter()
        .position(|c| c.card_id(&r.game.card_data) == "CS-L5")
        .unwrap();
    assert!(r
        .game
        .digivolve_from_hand(1, idx, base.index as usize, PlaySource::ByDigivolve));
    assert!(r.pending_selection().is_none());
}

// ─── Clause 3: [Security] play self ─────────────────────────────────────────

#[test]
fn bt22_093_security_effect_plays_the_tamer_without_paying() {
    let r = builder().start();
    let card = r.compiled_card(CARD_ID).unwrap();
    assert!(card.effects.iter().any(|c| matches!(
        c,
        CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::OnSecurity)
    )));
}
