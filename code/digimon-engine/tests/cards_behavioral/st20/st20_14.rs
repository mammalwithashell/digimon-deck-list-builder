//! ST20-14 Our Courage United — Option, Black, Cost 3, traits: [ADVENTURE].
//!
//! # Card text (official Bandai DB / card image / cards.json)
//!
//! While you have a Digimon or Tamer with the [ADVENTURE] trait on the field,
//! you can ignore this card's color requirements.
//!
//! **[Main]** <Draw 2> (Draw 2 cards from your deck.) Then, place this card in
//! the battle area.
//!
//! **[All Turns]** When any of your level 5 or higher Digimon would leave the
//! battle area, <Delay> (By trashing this card after the placing turn, activate
//! the effect below.)
//! ・You may play 1 level 5 or lower Digimon card with the [ADVENTURE] trait
//!   from your hand without paying the cost.
//!
//! **Inherited (Security):** Security Effect [Security] Place this card in the
//! battle area.
//!
//! # DCGO C# reference
//! `DCGO/Assets/Scripts/CardEffect/ST20/Black/ST20_14.cs`
//!
//! # Path taken — B (replacement-shape, no new substrate)
//!
//! ST20-14's <Delay> is EVENT-triggered ("When your Lv5+ Digimon would leave
//! the battle area"), which DCGO models as `EffectTiming.WhenRemoveField` — the
//! same timing BT17-095 uses. The faithful DSL expression is `kind: replacement`
//! + `trigger: when_would_leave_battle_area` (BT17-095 Clause B / BT20-091
//! Clause B), with the body paying the <Delay> cost via
//! `delete_permanent { target: source }` then optionally playing 1 card from
//! hand (`select_hand` + `play_from_hand_free`). The leave is NOT cancelled.
//! No engine/DSL widening was required — every verb already existed.
//!
//! # Patterns this test covers
//!
//! - **Color bypass via `use_requirement: any_field_permanent`** counting the
//!   OWNER's battle area AND breeding area — the printed "on the field" counts a
//!   breeding-area [ADVENTURE] Digimon (REQUIRED breeding-area regression).
//! - **[Main] <Draw 2> then place self as Delay-Option.**
//! - **Clause B leave-watcher**: `replacement_subject_is_mine` + `level_gte: 5`
//!   subject filter; on accept, pay the <Delay> cost (self-trash) then optional
//!   play 1 Lv5- [ADVENTURE] Digimon from hand free; the leave still proceeds.
//! - **[Security] (inherited) place self.**

#![allow(dead_code, unused_imports)]

use digimon_dsl::compiled::{
    CompiledCardKind, CompiledClause, CompiledColor, CompiledDeclarativeClause, CompiledScope,
    CompiledStep, CompiledTiming,
};
use digimon_engine::action::mask::build_action_mask;
use digimon_engine::action::space::{HAND_EFFECT_START, PASS, PLAY_HAND_START, REPLACEMENT_ACCEPT};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind};
use digimon_engine::permanent::OptionState;
use digimon_engine::replacement::ReplacementCause;
use digimon_engine::selection::SelectionKind;

const CARD_ID: &str = "ST20-14";
const YAML: &str = include_str!("../../../cards/st20/ST20-14.yaml");

// ── Card-data factories ──────────────────────────────────────────────────────

fn filler(id: &str) -> CardData {
    make_test_card(id, id)
}

/// A level-N Digimon carrying `traits`, with an explicit color (default Black
/// so it does NOT overlap ST20-14's own color for the bypass tests unless we
/// deliberately want a color match).
fn digimon(id: &str, name: &str, level: u8, color: CardColor, traits: &[&str]) -> CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = CardKind::Digimon;
    c.level = Some(level);
    c.dp = Some(i32::from(level) * 1000);
    c.play_cost = u16::from(level);
    c.colors = vec![color];
    c.traits = traits.iter().map(|t| t.to_string()).collect();
    c
}

/// A Tamer carrying `traits`, with an explicit color.
fn tamer(id: &str, name: &str, color: CardColor, traits: &[&str]) -> CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = CardKind::Tamer;
    c.play_cost = 3;
    c.colors = vec![color];
    c.traits = traits.iter().map(|t| t.to_string()).collect();
    c
}

fn runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("ST20-14 YAML loads")
        .memory(10)
        .start()
}

fn field_contains(r: &DebugRunner, player: u8, card_id: &str) -> bool {
    r.game.player(player).battle_area.iter().any(|perm| {
        perm.card_sources
            .iter()
            .any(|card| card.card_id(&r.game.card_data) == card_id)
    })
}

fn field_has_delay_option(r: &DebugRunner, player: u8) -> bool {
    r.game.player(player)
        .battle_area
        .iter()
        .any(|perm| matches!(perm.option_state, OptionState::Delayed { .. }))
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 1 — Structural assertions
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn st20_14_yaml_parses_without_error() {
    let _ = runner();
}

#[test]
fn st20_14_has_printed_metadata() {
    let r = runner();
    let card = r.compiled_card(CARD_ID).expect("ST20-14 present");
    assert_eq!(card.name, "Our Courage United");
    assert_eq!(card.kind, CompiledCardKind::Option);
    assert_eq!(card.cost, Some(3));
    assert_eq!(card.color, vec![CompiledColor::Black]);
    assert!(
        card.traits.iter().any(|t| t == "ADVENTURE"),
        "ST20-14 carries the ADVENTURE trait"
    );
}

/// Three compiled clauses: Main (triggered), All-Turns (declarative
/// replacement), Security (triggered, inherited scope). The color-bypass is a
/// top-level `use_requirement`, NOT a separate clause.
#[test]
fn st20_14_has_three_clauses() {
    let r = runner();
    let card = r.compiled_card(CARD_ID).expect("ST20-14 present");
    assert_eq!(
        card.effects.len(),
        3,
        "ST20-14 must have exactly 3 clauses (Main, Replacement, Security); got {}",
        card.effects.len()
    );
    assert!(
        card.use_requirement.is_some(),
        "ST20-14 must carry a top-level use_requirement for the color bypass"
    );
}

/// Clause A: MainFromHand, draws 2, ends by placing self as a Delay-Option.
#[test]
fn st20_14_main_clause_draws_two_then_places_self() {
    let r = runner();
    let card = r.compiled_card(CARD_ID).expect("ST20-14 present");
    let main = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| t.when.contains(&CompiledTiming::MainFromHand))
        .expect("Main clause with MainFromHand timing must exist");
    assert!(
        main.process
            .iter()
            .any(|s| matches!(s, CompiledStep::Draw { count: 2, .. })),
        "Main clause must <Draw 2>"
    );
    assert!(
        main.process
            .iter()
            .any(|s| matches!(s, CompiledStep::PlaceSelfAsDelayOption)),
        "Main clause must place this card as a Delay-Option"
    );
}

/// Clause B is a declarative replacement targeting `when_would_leave_battle_area`.
#[test]
fn st20_14_has_when_would_leave_replacement_clause() {
    let r = runner();
    let card = r.compiled_card(CARD_ID).expect("ST20-14 present");
    let has_leave = card.effects.iter().any(|c| {
        matches!(
            c,
            CompiledClause::Declarative(CompiledDeclarativeClause::Replacement { trigger, .. })
                if trigger == "when_would_leave_battle_area"
        )
    });
    assert!(
        has_leave,
        "ST20-14 must have a declarative when_would_leave_battle_area replacement clause"
    );
}

/// Clause C: inherited [Security] that places self.
#[test]
fn st20_14_security_clause_places_self_inherited() {
    let r = runner();
    let card = r.compiled_card(CARD_ID).expect("ST20-14 present");
    let sec = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| t.when.contains(&CompiledTiming::OnSecurity))
        .expect("Security clause with OnSecurity timing must exist");
    assert_eq!(
        sec.scope,
        CompiledScope::Inherited,
        "Security clause must be inherited-scope"
    );
    assert!(
        sec.process
            .iter()
            .any(|s| matches!(s, CompiledStep::PlaceSelfAsDelayOption)),
        "Security clause must place this card in the battle area"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 2 — Color bypass (use_requirement: any_field_permanent)
// ═══════════════════════════════════════════════════════════════════════════

/// REQUIRED breeding-area regression: with an [ADVENTURE] Digimon seated in the
/// OWNER's BREEDING area (empty battle area), and ST20-14 in hand, the [Main]
/// play must be legal — the printed "on the field" counts the breeding area.
/// The seeded Digimon is Red (off-color) so ordinary color matching cannot
/// account for the legality; only the use_requirement bypass can.
#[test]
fn st20_14_color_bypass_counts_breeding_area_adventure_digimon() {
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("ST20-14 YAML loads")
        .add_card(digimon("ADV-BREED", "Adventure Breeder", 3, CardColor::Red, &["ADVENTURE"]))
        .add_card(filler("FILL"))
        .hand(0, &[CARD_ID])
        .deck(0, &["FILL"; 5])
        .deck(1, &["FILL"; 5])
        .memory(10)
        .start();

    // Seat the off-color [ADVENTURE] Digimon in P0's BREEDING area; leave the
    // battle area empty.
    r.place_in_breeding(0, "ADV-BREED");
    assert!(
        r.game.players[0].battle_area.is_empty(),
        "precondition: P0 battle area is empty"
    );

    let mask = build_action_mask(&r.game, 0);
    assert_eq!(
        mask[PLAY_HAND_START as usize], 1.0,
        "ST20-14 [Main] play must be legal — a breeding-area [ADVENTURE] Digimon \
         satisfies the color-bypass use_requirement (any_field_permanent counts \
         the breeding area)"
    );
}

/// Positive: an [ADVENTURE] Digimon in the OWNER's BATTLE area (off-color)
/// satisfies the bypass too.
#[test]
fn st20_14_color_bypass_counts_battle_area_adventure_digimon() {
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("ST20-14 YAML loads")
        .add_card(digimon("ADV-FIELD", "Adventure Field", 4, CardColor::Red, &["ADVENTURE"]))
        .add_card(filler("FILL"))
        .hand(0, &[CARD_ID])
        .deck(0, &["FILL"; 5])
        .deck(1, &["FILL"; 5])
        .memory(10)
        .start();

    r.place_on_field(0, "ADV-FIELD", None);

    let mask = build_action_mask(&r.game, 0);
    assert_eq!(
        mask[PLAY_HAND_START as usize], 1.0,
        "an off-color [ADVENTURE] Digimon in the battle area satisfies the bypass"
    );
}

/// Positive: an [ADVENTURE] TAMER on the field (off-color) satisfies the bypass
/// ("Digimon or Tamer with the [ADVENTURE] trait").
#[test]
fn st20_14_color_bypass_counts_adventure_tamer() {
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("ST20-14 YAML loads")
        .add_card(tamer("ADV-TAMER", "Adventure Tamer", CardColor::Red, &["ADVENTURE"]))
        .add_card(filler("FILL"))
        .hand(0, &[CARD_ID])
        .deck(0, &["FILL"; 5])
        .deck(1, &["FILL"; 5])
        .memory(10)
        .start();

    r.place_on_field(0, "ADV-TAMER", None);

    let mask = build_action_mask(&r.game, 0);
    assert_eq!(
        mask[PLAY_HAND_START as usize], 1.0,
        "an off-color [ADVENTURE] Tamer satisfies the bypass"
    );
}

/// Negative: with NO [ADVENTURE] permanent anywhere and no color match, the
/// [Main] play must be ILLEGAL (the bypass is not satisfied and ST20-14 is a
/// mono-Black option with no Black permanent on the field).
#[test]
fn st20_14_color_bypass_absent_when_no_adventure_permanent() {
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("ST20-14 YAML loads")
        // A Red, NON-adventure Digimon: neither a color match (ST20-14 is
        // Black) nor an [ADVENTURE] permanent.
        .add_card(digimon("RED-PLAIN", "Red Plain", 3, CardColor::Red, &[]))
        .add_card(filler("FILL"))
        .hand(0, &[CARD_ID])
        .deck(0, &["FILL"; 5])
        .deck(1, &["FILL"; 5])
        .memory(10)
        .start();

    r.place_on_field(0, "RED-PLAIN", None);

    let mask = build_action_mask(&r.game, 0);
    assert_eq!(
        mask[PLAY_HAND_START as usize], 0.0,
        "without any [ADVENTURE] permanent and no Black color match, ST20-14 \
         [Main] play must be illegal"
    );
}

/// A Black permanent (color match) makes the play legal even without any
/// [ADVENTURE] permanent — ordinary color matching still applies.
#[test]
fn st20_14_ordinary_color_match_still_works() {
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("ST20-14 YAML loads")
        .add_card(digimon("BLACK-PLAIN", "Black Plain", 3, CardColor::Black, &[]))
        .add_card(filler("FILL"))
        .hand(0, &[CARD_ID])
        .deck(0, &["FILL"; 5])
        .deck(1, &["FILL"; 5])
        .memory(10)
        .start();

    r.place_on_field(0, "BLACK-PLAIN", None);

    let mask = build_action_mask(&r.game, 0);
    assert_eq!(
        mask[PLAY_HAND_START as usize], 1.0,
        "a Black (matching-color) permanent makes ST20-14 playable via ordinary \
         color matching"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 3 — Clause A: [Main] <Draw 2>; then place self as a Delay-Option
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn st20_14_main_draws_two_and_places_self_as_delay_option() {
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("ST20-14 YAML loads")
        // Seat an ADVENTURE permanent so the color-bypass allows the play.
        .add_card(digimon("ADV-FIELD", "Adventure Field", 4, CardColor::Red, &["ADVENTURE"]))
        .add_card(filler("FILL"))
        .hand(0, &[CARD_ID])
        .deck(0, &["FILL"; 5])
        .deck(1, &["FILL"; 5])
        .memory(10)
        .start();
    r.place_on_field(0, "ADV-FIELD", None);

    let hand_before = r.game.players[0].hand.len();
    let deck_before = r.game.players[0].deck.len();

    let _ = r.game.play_option_from_hand(0, 0);
    // Drain any queued effects (OnOptionPlaced etc.).
    r.game.drain_effect_queue();

    // <Draw 2>: hand had [ST20-14]; playing it removes it (-1) then draws 2
    // (+2) → net +1 relative to the pre-play hand, and deck shrinks by 2.
    assert_eq!(
        r.game.players[0].deck.len(),
        deck_before - 2,
        "ST20-14 [Main] must draw 2 cards from the deck"
    );
    assert_eq!(
        r.game.players[0].hand.len(),
        hand_before - 1 + 2,
        "hand = (played ST20-14: -1) + (drew 2: +2)"
    );
    // "Then, place this card in the battle area" as a Delay-Option.
    assert!(
        field_has_delay_option(&r, 0),
        "ST20-14 must be placed in the battle area as a Delay-Option"
    );
    assert!(
        field_contains(&r, 0, CARD_ID),
        "the placed permanent is ST20-14 itself"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 4 — Clause B: [All Turns] <Delay> own-Lv5+-would-leave observer
// ═══════════════════════════════════════════════════════════════════════════

/// Seat the ST20-14 option permanent at `handle` as a Delay-Option so its
/// Clause B `when_would_leave_battle_area` replacement can fire on a later
/// turn. Placing via `place_on_field` yields a Standard option, so the Delayed
/// state is set here directly (mirrors BT17-095's `seat_as_delay_option`).
fn seat_as_delay_option(
    r: &mut DebugRunner,
    handle: digimon_engine::permanent::PermanentHandle,
) {
    let turn = r.game.turn_count;
    let perm = &mut r.game.players[handle.player as usize].battle_area[handle.index as usize];
    perm.option_state = OptionState::Delayed {
        owner: handle.player,
        trash_on_turn: turn + 2,
        trigger: digimon_engine::enums::DelayTrigger::EndOfYourNextTurn,
        placed_on_turn: turn,
    };
}

/// Positive: with ST20-14 seated as a Delay-Option and an own Lv5+ Digimon on
/// the field, deleting that Digimon fires the optional would-leave response.
/// Accepting pays the <Delay> cost (ST20-14 self-trashes) and prompts to play a
/// Lv5- [ADVENTURE] Digimon from hand; the leaving Digimon still leaves.
#[test]
fn st20_14_delay_plays_adventure_from_hand_when_own_lv5_leaves() {
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("ST20-14 YAML loads")
        .add_card(digimon("LV5-LEAVER", "Big Adventure", 5, CardColor::Black, &[]))
        .add_card(digimon("ADV-HAND", "Small Adventure", 4, CardColor::Black, &["ADVENTURE"]))
        .add_card(filler("FILL"))
        .hand(0, &["ADV-HAND"])
        .deck(0, &["FILL"; 5])
        .deck(1, &["FILL"; 5])
        .memory(10)
        .start();

    let opt = r.place_on_field(0, CARD_ID, Some(0));
    seat_as_delay_option(&mut r, opt);
    let leaver = r.place_on_field(0, "LV5-LEAVER", None);
    let leaver_card = r.top_card(leaver);

    r.game
        .delete_permanent_with_cause(leaver, ReplacementCause::OwnEffect);

    // Optional replacement installs an accept prompt first.
    let pending = r
        .game
        .pending_selection
        .as_ref()
        .expect("ST20-14 should offer the optional would-leave <Delay> response");
    assert_eq!(pending.kind, SelectionKind::Replacement);
    assert!(pending.valid_action_ids.contains(&REPLACEMENT_ACCEPT));

    r.game
        .resolve_selection(0, REPLACEMENT_ACCEPT)
        .expect("accept the <Delay> response");

    // Paying the <Delay> cost trashes ST20-14 from the field.
    assert!(
        !field_contains(&r, 0, CARD_ID),
        "the <Delay> cost trashes ST20-14 from the battle area"
    );

    // Now a data-driven EffectChoice prompt (hand-card action IDs at
    // HAND_EFFECT_START+i, matching the sister Delay-cost hand flows in
    // lower_replacement.rs) to play the Lv5- [ADVENTURE] Digimon.
    let pending = r
        .game
        .pending_selection
        .as_ref()
        .expect("accepting prompts which [ADVENTURE] Digimon to play from hand");
    assert_eq!(pending.kind, SelectionKind::EffectChoice);
    assert!(pending.valid_action_ids.contains(&HAND_EFFECT_START));

    r.game
        .resolve_selection(0, HAND_EFFECT_START)
        .expect("play the [ADVENTURE] Digimon from hand");
    r.game.drain_effect_queue();

    assert!(
        field_contains(&r, 0, "ADV-HAND"),
        "ST20-14 plays the selected Lv5- [ADVENTURE] Digimon from hand"
    );
    // The leaving Lv5 Digimon still leaves (not cancelled).
    assert!(
        r.game.players[0]
            .trash
            .iter()
            .any(|c| c.handle() == leaver_card),
        "the leaving Lv5 Digimon still leaves the field (the <Delay> does not cancel it)"
    );
    assert!(
        !field_contains(&r, 0, "LV5-LEAVER"),
        "the Lv5 Digimon is no longer on the field"
    );
}

/// Declining the would-leave response leaves ST20-14 on the field (the <Delay>
/// cost is not paid), no card is played, and the Digimon still leaves.
#[test]
fn st20_14_decline_delay_keeps_option_and_plays_nothing() {
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("ST20-14 YAML loads")
        .add_card(digimon("LV5-LEAVER", "Big Adventure", 5, CardColor::Black, &[]))
        .add_card(digimon("ADV-HAND", "Small Adventure", 4, CardColor::Black, &["ADVENTURE"]))
        .add_card(filler("FILL"))
        .hand(0, &["ADV-HAND"])
        .deck(0, &["FILL"; 5])
        .deck(1, &["FILL"; 5])
        .memory(10)
        .start();

    let opt = r.place_on_field(0, CARD_ID, Some(0));
    seat_as_delay_option(&mut r, opt);
    let leaver = r.place_on_field(0, "LV5-LEAVER", None);

    r.game
        .delete_permanent_with_cause(leaver, ReplacementCause::OwnEffect);
    r.game
        .resolve_selection(0, PASS)
        .expect("decline the <Delay> response");
    r.game.drain_effect_queue();

    assert!(
        field_contains(&r, 0, CARD_ID),
        "declining keeps ST20-14 on the field (the <Delay> cost is not paid)"
    );
    assert!(
        !field_contains(&r, 0, "ADV-HAND"),
        "declining plays nothing — the [ADVENTURE] Digimon stays in hand"
    );
    assert!(
        !field_contains(&r, 0, "LV5-LEAVER"),
        "the Lv5 Digimon still leaves"
    );
}

/// Negative subject filter: an own Lv4 Digimon leaving must NOT fire the
/// response (`level_gte: 5` rejects it).
#[test]
fn st20_14_does_not_fire_for_own_lv4_leaving() {
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("ST20-14 YAML loads")
        .add_card(digimon("LV4-LEAVER", "Low Adventure", 4, CardColor::Black, &[]))
        .add_card(digimon("ADV-HAND", "Small Adventure", 4, CardColor::Black, &["ADVENTURE"]))
        .add_card(filler("FILL"))
        .hand(0, &["ADV-HAND"])
        .deck(0, &["FILL"; 5])
        .deck(1, &["FILL"; 5])
        .memory(10)
        .start();

    let opt = r.place_on_field(0, CARD_ID, Some(0));
    seat_as_delay_option(&mut r, opt);
    let leaver = r.place_on_field(0, "LV4-LEAVER", None);

    r.game
        .delete_permanent_with_cause(leaver, ReplacementCause::OwnEffect);
    r.game.drain_effect_queue();

    assert!(
        r.game.pending_selection.is_none(),
        "an own Lv4 Digimon leaving must NOT fire ST20-14 (level_gte:5 rejects)"
    );
    assert!(
        field_contains(&r, 0, CARD_ID),
        "ST20-14 remains on the field (no <Delay> cost paid)"
    );
}

/// Negative subject filter: an OPPONENT's Lv5+ Digimon leaving must NOT fire the
/// response (`replacement_subject_is_mine` rejects it).
#[test]
fn st20_14_does_not_fire_for_opponent_lv5_leaving() {
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("ST20-14 YAML loads")
        .add_card(digimon("OPP-LV5", "Enemy Adventure", 5, CardColor::Black, &[]))
        .add_card(filler("FILL"))
        .deck(0, &["FILL"; 5])
        .deck(1, &["FILL"; 5])
        .memory(10)
        .start();

    let opt = r.place_on_field(0, CARD_ID, Some(0));
    seat_as_delay_option(&mut r, opt);
    let opp = r.place_on_field(1, "OPP-LV5", None);

    r.game
        .delete_permanent_with_cause(opp, ReplacementCause::OwnEffect);
    r.game.drain_effect_queue();

    assert!(
        r.game.pending_selection.is_none(),
        "an OPPONENT's Lv5 Digimon leaving must NOT fire ST20-14 \
         (replacement_subject_is_mine rejects)"
    );
    assert!(field_contains(&r, 0, CARD_ID), "ST20-14 remains on the field");
}

/// With no eligible Lv5- [ADVENTURE] Digimon in hand, the <Delay> accept prompt
/// is STILL offered — DCGO ST20_14.cs:83-87 `CanUseCondition` is only
/// `CanTriggerWhenPermanentRemoveField(..) && CanDeclareOptionDelayEffect(card)`
/// (no hand-candidate gate); the candidate scan
/// (`card.Owner.HandCards.Count(CanPlayAdventureCondition) >= 1`, line 125)
/// happens inside `ActivateCoroutine` AFTER the self-trash. Accepting pays the
/// <Delay> cost (ST20-14 self-trashes), the reward whiffs (nothing plays), and
/// the leaving Digimon still completes its leave. Mirrors the union-flow sibling
/// `bt19_099_offer_installs_even_with_zero_reward_candidates`.
#[test]
fn st20_14_offer_installs_even_with_no_eligible_adventure_in_hand() {
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("ST20-14 YAML loads")
        .add_card(digimon("LV5-LEAVER", "Big Adventure", 5, CardColor::Black, &[]))
        // A hand Digimon that is NOT [ADVENTURE] and Lv6 (both filters fail).
        .add_card(digimon("BAD-HAND", "No Adventure", 6, CardColor::Black, &[]))
        .add_card(filler("FILL"))
        .hand(0, &["BAD-HAND"])
        .deck(0, &["FILL"; 5])
        .deck(1, &["FILL"; 5])
        .memory(10)
        .start();

    let opt = r.place_on_field(0, CARD_ID, Some(0));
    seat_as_delay_option(&mut r, opt);
    let leaver = r.place_on_field(0, "LV5-LEAVER", None);
    let leaver_card = r.top_card(leaver);

    r.game
        .delete_permanent_with_cause(leaver, ReplacementCause::OwnEffect);

    // The accept prompt installs even with zero eligible hand candidates
    // (DCGO CanUseCondition has no candidate gate — ST20_14.cs:83-87).
    let pending = r.game.pending_selection.as_ref().expect(
        "the <Delay> accept prompt installs even with no eligible Lv5- \
         [ADVENTURE] Digimon in hand",
    );
    assert_eq!(pending.kind, SelectionKind::Replacement);
    assert!(pending.valid_action_ids.contains(&REPLACEMENT_ACCEPT));

    r.game
        .resolve_selection(0, REPLACEMENT_ACCEPT)
        .expect("accept the <Delay> response");
    r.game.drain_effect_queue();

    // Accepting pays the <Delay> cost even though nothing can be played
    // (DCGO's post-trash candidate whiff — ST20_14.cs:125).
    assert!(
        !field_contains(&r, 0, CARD_ID),
        "accepting pays the <Delay> cost (self-trash) even though nothing can be played"
    );
    assert!(
        r.game.players[0]
            .trash
            .iter()
            .any(|c| c.card_id(&r.game.card_data) == CARD_ID),
        "the trashed ST20-14 is in its owner's trash"
    );
    // The reward whiffs: the ineligible hand card is NOT played.
    assert!(
        !field_contains(&r, 0, "BAD-HAND"),
        "with zero eligible candidates nothing is played from hand"
    );
    assert!(
        r.game.pending_selection.is_none(),
        "with zero eligible candidates no reward prompt installs (DCGO post-trash whiff)"
    );
    // The leaving Lv5 Digimon still completes its leave.
    assert!(
        r.game.players[0]
            .trash
            .iter()
            .any(|c| c.handle() == leaver_card),
        "the leaving Lv5 Digimon still completes its leave to the trash"
    );
    assert!(
        !field_contains(&r, 0, "LV5-LEAVER"),
        "the Lv5 Digimon still leaves"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 5 — Clause C: [Security] (inherited) place self
// ═══════════════════════════════════════════════════════════════════════════

/// Attacking into a defender whose security tops with ST20-14: the inherited
/// [Security] clause places ST20-14 into the DEFENDER's battle area as a
/// Delay-Option (printed "Place this card in the battle area").
#[test]
fn st20_14_security_places_self_in_defender_battle_area() {
    let mut attacker = filler("ATK");
    attacker.dp = Some(6000);

    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("ST20-14 YAML loads")
        .add_card(attacker.clone())
        .add_card(filler("FILL"))
        .deck(0, &["FILL"; 5])
        .deck(1, &["FILL"; 5])
        .security(1, &[CARD_ID])
        .memory(10)
        .start();

    let atk = r.place_on_field(0, "ATK", Some(0));
    assert_eq!(r.security_count(1), 1, "precondition: ST20-14 in defender security");

    let _ = r.attack_player(atk, 1, false);
    r.auto_resolve().expect("security selections resolve");

    assert_eq!(
        r.security_count(1),
        0,
        "ST20-14 left the defender's security stack"
    );
    assert!(
        field_contains(&r, 1, CARD_ID),
        "ST20-14 was placed into the defender's battle area"
    );
    assert!(
        field_has_delay_option(&r, 1),
        "ST20-14 is seated as a Delay-Option in the defender's battle area"
    );
}
