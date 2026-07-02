//! BT9-092 Cool Boy — Tamer, White, Cost 2.
//!
//! # Card text (image-authoritative — Card/BT9-092.webp)
//!   [On Play] Reveal the top 3 cards of your deck. Add 1 Digimon card with
//!     [X Antibody] in its traits and 1 Option card with [X Antibody] in its
//!     traits among them to your hand. Place the rest at the bottom of your
//!     deck in any order.
//!   [Your Turn] When one of your Digimon digivolves into a same-level Digimon
//!     with [X Antibody] in its traits, you may suspend this Tamer to gain 1
//!     memory and <Draw 1>.
//!   [Security] Play this card without paying its memory cost.
//!
//! # Implemented slice (all three clauses)
//! - [On Play] reveal 3; add 1 X Antibody Digimon and 1 X Antibody Option;
//!   bottom the rest.
//! - [Your Turn] same-level X-Antibody digivolve observer: by suspending this
//!   Tamer (activation cost), gain 1 memory and draw 1. "You may" → optional
//!   outer trigger. Gated to same-level + [X Antibody] result on your turn.
//! - [Security] Play this card free.
//!
//! DCGO behavioral reference: DCGO/.../BT9/White/BT9_092.cs — clause 2
//!   `PermanentCondition` = owner battle-area Digimon whose TopCard
//!   `HasXAntibodyTraits` and `IsDigivolvedFromSameLevelFromEnterFieldHashtable`;
//!   `CanUseCondition` = on battle area + `IsOwnerTurn`; `CanActivateCondition`
//!   = `CanActivateSuspendCostEffect` (Tamer must be unsuspended); body =
//!   suspend self → draw 1 → add memory 1.

use digimon_dsl::compiled::{CompiledClause, CompiledTiming};
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::{CardColor, CardKind, PlaySource};

const CARD_ID: &str = "BT9-092";

// ─── Card-data factories ─────────────────────────────────────────────────────

/// A bare filler card so each player's deck is non-empty (digivolve draws 1).
fn make_filler(card_id: &str) -> CardData {
    CardData {
        card_id: card_id.to_string(),
        card_name: card_id.to_string(),
        card_kind: CardKind::Digimon,
        level: Some(3),
        dp: Some(3000),
        play_cost: 3,
        colors: vec![CardColor::White],
        traits: Vec::new(),
        evo_costs: Vec::new(),
        dna_costs: Vec::new(),
        effect_text: String::new(),
        inherited_text: String::new(),
        security_text: String::new(),
        keywords: Vec::new(),
        dual: None,
        effect_class_name: card_id.replace('-', "_"),
        index: 0,
        norm_id: 0.0,
        ace_overflow: None,
        digixros_aliases: Vec::new(),
        also_treated_as: Vec::new(),
    }
}

/// A Level-N Digimon usable as a digivolve base on the battle area.
fn make_base(card_id: &str, level: u8) -> CardData {
    let mut c = make_filler(card_id);
    c.level = Some(level);
    c.dp = Some(4000);
    c
}

/// A Level-N [X Antibody] Digimon to digivolve INTO. Carries an evo-cost whose
/// `level` equals `base_level` so `digivolve_from_hand` accepts it onto a
/// same-level (or one-below) base, and the digivolve-result top card matches
/// the same-level / [X Antibody] predicates.
fn make_x_evo(card_id: &str, level: u8, base_level: u8) -> CardData {
    let mut c = make_filler(card_id);
    c.level = Some(level);
    c.dp = Some(6000);
    c.traits = vec!["X Antibody".to_string()];
    c.evo_costs = vec![EvoCost {
        card_color: 4, // White (evo_color: 4) — matches make_filler's color
        level: base_level,
        memory_cost: 0,
    }];
    c
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 1 — structural assertions
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn bt9_092_has_on_play_search_and_security_play() {
    let runner = DebugRunner::builder()
        .dsl_card("BT9-092")
        .expect("BT9-092 must load from embedded DSL pack")
        .memory(5)
        .start();
    let card = runner.compiled_card("BT9-092").expect("compiled card");

    assert!(
        card.effects.iter().any(|clause| matches!(
            clause,
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::OnPlay)
        )),
        "BT9-092 must have OnPlay search"
    );
    assert!(
        card.effects.iter().any(|clause| matches!(
            clause,
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::OnSecurity)
        )),
        "BT9-092 must have Security play"
    );
    assert!(
        card.effects.iter().any(|clause| matches!(
            clause,
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::OnDigivolve)
        )),
        "BT9-092 must have the [Your Turn] on-digivolve observer clause"
    );
}

/// The [On Play] clause is player-optional only in its add-to-hand picks
/// ("Add 1 ... among them"), and the digivolve observer ("you may suspend")
/// is optional — but the reveal itself and bottoming the rest are mandatory.
#[test]
fn bt9_092_on_digivolve_clause_is_optional() {
    let runner = DebugRunner::builder()
        .dsl_card("BT9-092")
        .expect("BT9-092 must load from embedded DSL pack")
        .start();
    let card = runner.compiled_card("BT9-092").expect("compiled card");

    let on_digivolve = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| t.when.contains(&CompiledTiming::OnDigivolve))
        .expect("on_digivolve clause present");
    assert!(
        on_digivolve.optional,
        "the digivolve observer is a 'you may suspend' optional trigger"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 2 — [On Play] reveal-top-3 / add X-Antibody Digimon + Option /
//             bottom the rest.
// ═══════════════════════════════════════════════════════════════════════════

/// End-to-end On Play: reveal the top 3 of deck, add the 1 [X Antibody]
/// Digimon and the 1 [X Antibody] Option among them to hand, and bottom the
/// remaining revealed card. The non-X cards must be rejected by the filters.
#[test]
fn bt9_092_on_play_reveals_three_adds_x_digimon_and_option_bottoms_rest() {
    // Build the X-Antibody Digimon / Option to be added, plus non-matching
    // decoys that must be bottomed (not addable).
    let mut x_digimon = make_filler("X-DIGI");
    x_digimon.traits = vec!["X Antibody".to_string()];

    let mut x_option = make_filler("X-OPT");
    x_option.card_kind = CardKind::Option;
    x_option.level = None;
    x_option.dp = None;
    x_option.traits = vec!["X Antibody".to_string()];

    // A plain (non-X) Digimon — the only card that should be bottomed.
    let plain = make_filler("PLAIN-DIGI");

    let mut runner = DebugRunner::builder()
        .dsl_card("BT9-092")
        .expect("BT9-092 must load from embedded DSL pack")
        .add_card(x_digimon)
        .add_card(x_option)
        .add_card(plain)
        .add_card(make_filler("DECK-BOTTOM"))
        // Deck top = end of vec; the top 3 revealed are PLAIN-DIGI, X-OPT, X-DIGI.
        .deck(0, &["DECK-BOTTOM", "PLAIN-DIGI", "X-OPT", "X-DIGI"])
        // BT9-092 played from hand (the production On Play path).
        .hand(0, &[CARD_ID])
        .memory(5)
        .start();

    // Cool Boy is the only card in hand; playing it fires the On Play search.
    runner.play(0, 0).expect("play BT9-092 from hand");
    runner.auto_resolve().expect("resolve On Play reveal/add");

    // The deck started with 4 cards; BT9-092 was the only hand card and it left
    // to the battle area, so hand began at 0 post-play and gains the 2 adds.
    assert_eq!(
        runner.hand_size(0),
        2,
        "On Play must add exactly the X-Antibody Digimon and X-Antibody Option to hand"
    );
    assert!(
        runner.game.players[0]
            .hand
            .iter()
            .any(|c| c.card_id(&runner.game.card_data) == "X-DIGI"),
        "the [X Antibody] Digimon must be added to hand"
    );
    assert!(
        runner.game.players[0]
            .hand
            .iter()
            .any(|c| c.card_id(&runner.game.card_data) == "X-OPT"),
        "the [X Antibody] Option must be added to hand"
    );
    // Deck began with 4 (DECK-BOTTOM, PLAIN-DIGI, X-OPT, X-DIGI). Reveal top 3
    // (-3), add the 2 X cards to hand, bottom the 1 non-X card back (+1) →
    // 4 - 3 + 1 = 2 cards left (DECK-BOTTOM + the re-bottomed PLAIN-DIGI).
    assert_eq!(
        runner.deck_size(0),
        2,
        "deck shrinks by the 2 added cards; the non-X revealed card is bottomed back"
    );
    // The plain (non-X) Digimon must NOT have been addable to hand.
    assert!(
        !runner.game.players[0]
            .hand
            .iter()
            .any(|c| c.card_id(&runner.game.card_data) == "PLAIN-DIGI"),
        "a non-[X Antibody] card must not be addable to hand"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 3 — [Your Turn] same-level [X Antibody] digivolve observer:
//             by suspending this Tamer, gain 1 memory and draw 1.
// ═══════════════════════════════════════════════════════════════════════════

/// Positive: P0 digivolves a Lv.4 base into a same-level (Lv.4) [X Antibody]
/// Digimon on their own turn. BT9-092's [Your Turn] clause offers the optional
/// "you may suspend" trigger; accepting it suspends Cool Boy (activation cost),
/// draws 1, and gains 1 memory.
#[test]
fn bt9_092_same_level_x_antibody_digivolve_observer_suspends_draws_and_gains_memory() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT9-092")
        .expect("BT9-092 must load from embedded DSL pack")
        .add_card(make_base("BASE-L4", 4))
        .add_card(make_x_evo("X-EVO-L4", 4, 4))
        .add_card(make_filler("DRAW-1"))
        .add_card(make_filler("DRAW-2"))
        // Deck top = end of vec; digivolve draws 1, the observer draws 1 more.
        .deck(0, &["DRAW-2", "DRAW-1"])
        .deck(1, &["DRAW-2"])
        .hand(0, &["X-EVO-L4"])
        // Start below the +10 memory cap so the observer's +1 gain is
        // observable (memory clamps at rules.memory_range.1).
        .memory(5)
        .start();

    let tamer = runner.place_on_field(0, CARD_ID, Some(0));
    let base = runner.place_on_field(0, "BASE-L4", Some(0));
    assert!(
        !runner.game.player(0).battle_area[tamer.index as usize].is_suspended,
        "Cool Boy must start unsuspended"
    );

    let evo_idx = runner.game.players[0]
        .hand
        .iter()
        .position(|c| c.card_id(&runner.game.card_data) == "X-EVO-L4")
        .expect("X-EVO-L4 in P0 hand");
    let ok =
        runner
            .game
            .digivolve_from_hand(0, evo_idx, base.index as usize, PlaySource::ByDigivolve);
    assert!(ok, "same-level L4->L4 X-Antibody digivolve must succeed");

    // Snapshot AFTER the digivolve itself (digivolve_from_hand draws 1 of its
    // own) so we measure only the observer's draw + memory.
    let mem_before = runner.memory();
    let hand_before = runner.hand_size(0);
    let deck_before = runner.deck_size(0);

    // The observer is a "you may suspend" optional trigger — the choice must be
    // exposed to the action space.
    assert!(
        runner.pending_selection().is_some(),
        "the optional 'you may suspend' trigger must surface a pending selection"
    );
    assert!(
        runner.pending_is_optional(),
        "the digivolve observer prompt must be optional (you may)"
    );

    runner
        .accept_optional_trigger()
        .expect("accept the 'you may suspend' trigger");
    runner.auto_resolve().expect("resolve suspend/draw/gain");

    // Cool Boy suspended by the activation cost.
    assert!(
        runner.game.player(0).battle_area[tamer.index as usize].is_suspended,
        "Cool Boy must be suspended (the 'by suspending this Tamer' activation cost)"
    );
    // Exactly +1 memory and +1 card drawn from the observer body.
    assert_eq!(
        runner.memory(),
        mem_before + 1,
        "the observer must gain exactly 1 memory"
    );
    assert_eq!(
        runner.hand_size(0),
        hand_before + 1,
        "the observer must draw exactly 1 card"
    );
    assert_eq!(
        runner.deck_size(0),
        deck_before - 1,
        "the drawn card leaves the deck"
    );
}

/// Negative (player choice): the same-level X digivolve offers the optional
/// trigger, but DECLINING it leaves Cool Boy unsuspended and memory/hand
/// unchanged (PASS must be a real, exposed choice).
#[test]
fn bt9_092_observer_decline_leaves_tamer_unsuspended() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT9-092")
        .expect("BT9-092 must load from embedded DSL pack")
        .add_card(make_base("BASE-L4", 4))
        .add_card(make_x_evo("X-EVO-L4", 4, 4))
        .add_card(make_filler("DRAW-1"))
        .add_card(make_filler("DRAW-2"))
        .deck(0, &["DRAW-2", "DRAW-1"])
        .deck(1, &["DRAW-2"])
        .hand(0, &["X-EVO-L4"])
        .memory(10)
        .start();

    let tamer = runner.place_on_field(0, CARD_ID, Some(0));
    let base = runner.place_on_field(0, "BASE-L4", Some(0));

    let evo_idx = runner.game.players[0]
        .hand
        .iter()
        .position(|c| c.card_id(&runner.game.card_data) == "X-EVO-L4")
        .expect("X-EVO-L4 in P0 hand");
    assert!(runner.game.digivolve_from_hand(
        0,
        evo_idx,
        base.index as usize,
        PlaySource::ByDigivolve,
    ));

    let mem_before = runner.memory();
    let hand_before = runner.hand_size(0);

    assert!(
        runner.pending_is_optional(),
        "the digivolve observer prompt must be optional (you may)"
    );
    runner
        .decline_optional_trigger()
        .expect("decline the optional trigger");
    runner.auto_resolve().expect("settle after decline");

    assert!(
        !runner.game.player(0).battle_area[tamer.index as usize].is_suspended,
        "declining must leave Cool Boy unsuspended"
    );
    assert_eq!(
        runner.memory(),
        mem_before,
        "declining must not gain memory"
    );
    assert_eq!(runner.hand_size(0), hand_before, "declining must not draw");
}

/// Negative (level gate): digivolving into a DIFFERENT-level (Lv.5) [X
/// Antibody] Digimon must NOT offer the trigger — `event_target_
/// same_level_as_previous` rejects, Cool Boy stays unsuspended, no
/// memory/draw. Mirrors the live precedent
/// `digivolve_event_can_match_same_level_x_antibody_transition`.
#[test]
fn bt9_092_observer_does_not_fire_for_different_level_x_digivolve() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT9-092")
        .expect("BT9-092 must load from embedded DSL pack")
        .add_card(make_base("BASE-L4", 4))
        // Lv.5 X-Antibody result on a Lv.4 base — DIFFERENT level. evo_cost
        // level 4 so the digivolve itself is legal; the same-level predicate
        // must still reject (top L5 != prior L4).
        .add_card(make_x_evo("X-EVO-L5", 5, 4))
        .add_card(make_filler("DRAW-1"))
        .add_card(make_filler("DRAW-2"))
        .deck(0, &["DRAW-2", "DRAW-1"])
        .deck(1, &["DRAW-2"])
        .hand(0, &["X-EVO-L5"])
        .memory(10)
        .start();

    let tamer = runner.place_on_field(0, CARD_ID, Some(0));
    let base = runner.place_on_field(0, "BASE-L4", Some(0));

    let evo_idx = runner.game.players[0]
        .hand
        .iter()
        .position(|c| c.card_id(&runner.game.card_data) == "X-EVO-L5")
        .expect("X-EVO-L5 in P0 hand");
    assert!(runner.game.digivolve_from_hand(
        0,
        evo_idx,
        base.index as usize,
        PlaySource::ByDigivolve,
    ));

    let mem_before = runner.memory();
    let hand_before = runner.hand_size(0);
    let _ = runner.auto_resolve();

    assert!(
        !runner.game.player(0).battle_area[tamer.index as usize].is_suspended,
        "Cool Boy must NOT be suspended — a different-level X digivolve must not fire the observer"
    );
    assert_eq!(
        runner.memory(),
        mem_before,
        "no memory gain for a different-level digivolve"
    );
    assert_eq!(
        runner.hand_size(0),
        hand_before,
        "no draw for a different-level digivolve"
    );
}

/// Negative (trait gate): digivolving into a same-level NON-[X Antibody]
/// Digimon must NOT fire the observer.
#[test]
fn bt9_092_observer_does_not_fire_for_non_x_same_level_digivolve() {
    let mut plain_evo = make_x_evo("PLAIN-EVO-L4", 4, 4);
    plain_evo.traits = vec![]; // strip X Antibody — must not satisfy the gate

    let mut runner = DebugRunner::builder()
        .dsl_card("BT9-092")
        .expect("BT9-092 must load from embedded DSL pack")
        .add_card(make_base("BASE-L4", 4))
        .add_card(plain_evo)
        .add_card(make_filler("DRAW-1"))
        .add_card(make_filler("DRAW-2"))
        .deck(0, &["DRAW-2", "DRAW-1"])
        .deck(1, &["DRAW-2"])
        .hand(0, &["PLAIN-EVO-L4"])
        .memory(10)
        .start();

    let tamer = runner.place_on_field(0, CARD_ID, Some(0));
    let base = runner.place_on_field(0, "BASE-L4", Some(0));

    let evo_idx = runner.game.players[0]
        .hand
        .iter()
        .position(|c| c.card_id(&runner.game.card_data) == "PLAIN-EVO-L4")
        .expect("PLAIN-EVO-L4 in P0 hand");
    assert!(runner.game.digivolve_from_hand(
        0,
        evo_idx,
        base.index as usize,
        PlaySource::ByDigivolve,
    ));

    let mem_before = runner.memory();
    let _ = runner.auto_resolve();

    assert!(
        !runner.game.player(0).battle_area[tamer.index as usize].is_suspended,
        "Cool Boy must NOT be suspended — a non-[X Antibody] digivolve must not fire the observer"
    );
    assert_eq!(
        runner.memory(),
        mem_before,
        "no memory gain for a non-[X Antibody] digivolve"
    );
}
