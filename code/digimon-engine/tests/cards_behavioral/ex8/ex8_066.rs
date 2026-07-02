//! EX8-066 Suzune Kazuki — Tamer, Blue, Cost 3. Trait: LIBERATOR.
//!
//! # Card text (card image EX8-066 — authoritative; cards.json agrees)
//!
//! ```text
//! [Start of Your Main Phase] If your opponent has a Digimon, gain 1 memory.
//! [All Turns] When your Digimon are played or digivolve, if any of them have
//! the [Ice-Snow] trait, by suspending this Tamer, trash any 1 digivolution
//! card from your opponent's Digimon.
//! ```
//!
//! # Security effect text
//!
//! ```text
//! Security Effect [Security] Play this card without paying the cost.
//! ```
//!
//! # DCGO C# reference
//!
//! DCGO/Assets/Scripts/CardEffect/EX8/Blue/EX8_066.cs:
//! - Clause 1 (OnStartMainPhase): CanActivateCondition = on battle area AND
//!   opponent has >= 1 battle-area Digimon AND CanAddMemory; MANDATORY
//!   (isOptional=false); body = AddMemory(1).
//! - Clause 2 (OnEnterFieldAnyone): CanUseCondition =
//!   (CanTriggerOnPermanentPlay || CanTriggerWhenPermanentDigivolving) over
//!   `EnterFieldPermanent` = own battle-area Digimon whose
//!   TopCard.EqualsTraits("Ice-Snow"); NO turn gate ([All Turns]).
//!   CanActivateCondition = on field AND CanActivateSuspendCostEffect — note:
//!   NOT gated on opponent sources existing. isOptional=true.
//!   Body = suspend self, then SelectTrashDigivolutionCards(permanentCondition:
//!   opponent battle-area Digimon, maxCount: 1, canNoTrash: false,
//!   isFromOnly1Permanent: false) — a cross-permanent mandatory pick of
//!   min(total, 1); with zero available sources the sub-flow no-ops (the
//!   suspend cost CAN still be paid).
//! - Clause 3 (SecuritySkill): PlaySelfTamerSecurityEffect — standard
//!   mandatory tamer security self-play.
//!
//! # Coverage
//!
//! - Metadata: tamer kind, cost 3, blue, LIBERATOR trait.
//! - Clause 1: SoMP +1 memory with opponent Digimon present; no gain without;
//!   opponent Tamer does not satisfy the gate; mandatory (structural).
//! - Clause 2: observer fires on own Ice-Snow Digimon PLAY and own Ice-Snow
//!   DIGIVOLVE; not for non-Ice-Snow subjects; not for opponent plays;
//!   accept suspends + trashes exactly 1 chosen opponent source (player picks
//!   among several, cross-permanent); decline leaves the tamer unsuspended;
//!   already-suspended tamer gets no prompt; zero opponent sources still
//!   allow the suspend (DCGO parity); fires on the opponent's turn
//!   ([All Turns]).
//! - Clause 3: structural shape + no-panic smoke.

#![allow(dead_code)]

use digimon_dsl::compiled::{
    CompiledCardKind, CompiledClause, CompiledColor, CompiledScope, CompiledStep, CompiledTiming,
};
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardKind, EffectTiming, PlaySource};
use digimon_engine::selection::{SelectionKind, TriggerSource};

use crate::dsl_card_data::compiled;

const CARD_ID: &str = "EX8-066";

// ─── Card-data factories ─────────────────────────────────────────────────────

/// Lv.3 Digimon WITH the [Ice-Snow] trait — playing it satisfies the
/// observer's `event_target_trait_has: Ice-Snow` gate.
fn make_ice_snow_lv3(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(3);
    c.dp = Some(2000);
    c.play_cost = 3;
    c.traits = vec!["Ice-Snow".to_string()];
    c
}

/// Lv.3 Digimon WITHOUT the [Ice-Snow] trait — the observer must not fire.
fn make_plain_lv3(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(3);
    c.dp = Some(2000);
    c.play_cost = 3;
    c
}

/// Lv.4 Ice-Snow Digimon that digivolves from Lv.3 for 0 memory — for the
/// digivolve half of the observer. Explicit evo_costs per the DebugRunner
/// empty-evo_costs pitfall (DSL-loaded cards carry no printed cost table).
fn make_ice_snow_lv4(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(4000);
    c.play_cost = 5;
    c.traits = vec!["Ice-Snow".to_string()];
    c.evo_costs = vec![EvoCost {
        card_color: 0,
        level: 3,
        memory_cost: 0,
    }];
    c
}

/// Lv.4 Digimon WITHOUT Ice-Snow — digivolving into this must NOT fire the
/// observer.
fn make_plain_lv4(id: &str) -> CardData {
    let mut c = make_ice_snow_lv4(id);
    c.traits = vec![];
    c
}

fn make_filler(id: &str) -> CardData {
    make_test_card(id, id)
}

fn filler_deck() -> Vec<&'static str> {
    vec!["FILLER", "FILLER", "FILLER"]
}

/// Standard behavioral runner: Suzune Kazuki + the synthetic test pool.
fn base_runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX8-066 YAML parses and compiles")
        .add_card(make_filler("FILLER"))
        .add_card(make_ice_snow_lv3("ICE-LV3"))
        .add_card(make_plain_lv3("PLAIN-LV3"))
        .add_card(make_ice_snow_lv4("ICE-LV4"))
        .add_card(make_plain_lv4("PLAIN-LV4"))
        .add_card(make_test_card("OPP-SRC", "Opp Source"))
        .add_card(make_test_card("OPP-SRC-B", "Opp Source B"))
        .add_card(make_plain_lv3("OPP-TOP"))
        .deck(0, &filler_deck())
        .deck(1, &filler_deck())
        .memory(3)
        .start()
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 1 — Structural / metadata assertions
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn ex8_066_metadata_matches_printed_card() {
    let card = compiled(CARD_ID);
    assert_eq!(card.card, CARD_ID);
    assert_eq!(card.name, "Suzune Kazuki");
    assert_eq!(card.kind, CompiledCardKind::Tamer);
    assert_eq!(card.cost, Some(3), "play cost 3");
    assert_eq!(card.level, None, "Tamers have no level");
    assert_eq!(card.dp, None, "Tamers have no DP");
    assert_eq!(card.color, vec![CompiledColor::Blue], "mono-blue Tamer");
    assert!(
        card.traits.iter().any(|t| t == "LIBERATOR"),
        "must carry the LIBERATOR trait"
    );
}

/// Exactly 3 triggered clauses: SoMP memory gain, the play/digivolve
/// observer, and the security self-play.
#[test]
fn ex8_066_has_three_triggered_clauses() {
    let card = compiled(CARD_ID);
    let triggered = card
        .effects
        .iter()
        .filter(|c| matches!(c, CompiledClause::Triggered(_)))
        .count();
    assert_eq!(
        triggered, 3,
        "SoMP + play/digivolve observer + security = 3 triggered clauses"
    );
}

/// Clause 1 ([Start of Your Main Phase]) is MANDATORY — DCGO isOptional=false
/// and the printed text has no "you may" — and not [Once Per Turn].
#[test]
fn ex8_066_clause1_somp_is_mandatory() {
    let card = compiled(CARD_ID);
    let clause = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t)
                if t.when.contains(&CompiledTiming::StartOfYourMainPhase) =>
            {
                Some(t)
            }
            _ => None,
        })
        .next()
        .expect("start_of_your_main_phase clause must exist");
    assert!(!clause.optional, "SoMP memory gain is mandatory");
    assert!(!clause.once_per_turn, "no printed [Once Per Turn]");
}

/// Clause 2 covers BOTH own-play and own-digivolve in one observer clause
/// ("When your Digimon are played or digivolve") and is optional ("by
/// suspending this Tamer" is a declinable activation cost). No [Once Per
/// Turn] is printed — the Tamer suspension is the natural limiter.
#[test]
fn ex8_066_observer_clause_covers_play_and_digivolve_and_is_optional() {
    let card = compiled(CARD_ID);
    let clause = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::OnAllyPlayed) => {
                Some(t)
            }
            _ => None,
        })
        .next()
        .expect("on_ally_played observer clause must exist");
    assert!(
        clause.when.contains(&CompiledTiming::OnDigivolve),
        "the same clause must also observe on_digivolve (\"are played or digivolve\")"
    );
    assert!(
        clause.optional,
        "\"by suspending this Tamer\" is an optional activation cost"
    );
    assert!(
        !clause.once_per_turn,
        "no printed [Once Per Turn] — do not add one"
    );
}

/// Clause 3 ([Security]) is mandatory, FaceUp scope, and plays the card from
/// security without paying the cost.
#[test]
fn ex8_066_security_clause_is_mandatory_faceup_play_from_security() {
    let card = compiled(CARD_ID);
    let clause = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::OnSecurity) => Some(t),
            _ => None,
        })
        .next()
        .expect("on_security clause must exist");
    assert!(!clause.optional, "[Security] effects are mandatory");
    assert_eq!(clause.scope, CompiledScope::FaceUp);
    assert!(
        clause
            .process
            .iter()
            .any(|s| matches!(s, CompiledStep::PlayFromSecurity)),
        "security clause must contain play_from_security"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 2 — Clause 1 behavioral: [Start of Your Main Phase] +1 memory
// ═══════════════════════════════════════════════════════════════════════════════

/// Opponent has a Digimon at the start of P0's main phase → P0 gains 1 memory.
#[test]
fn ex8_066_somp_gains_memory_when_opponent_has_digimon() {
    let mut runner = base_runner();
    runner.place_on_field(0, CARD_ID, Some(0));
    runner.place_on_field(1, "OPP-TOP", Some(0));
    runner.game.memory = 0;

    runner.game.enter_main_phase();
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.memory(),
        1,
        "P0 must gain exactly 1 memory when the opponent has a Digimon"
    );
}

/// Opponent has NO Digimon → the gate fails → memory unchanged.
#[test]
fn ex8_066_somp_no_gain_when_opponent_has_no_digimon() {
    let mut runner = base_runner();
    runner.place_on_field(0, CARD_ID, Some(0));
    runner.game.memory = 0;

    runner.game.enter_main_phase();
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.memory(),
        0,
        "no memory gain when the opponent controls no Digimon"
    );
}

/// An opponent TAMER does not satisfy "if your opponent has a Digimon".
#[test]
fn ex8_066_somp_no_gain_when_opponent_only_has_a_tamer() {
    let mut opp_tamer = make_test_card("OPP-TAMER", "Opp Tamer");
    opp_tamer.card_kind = CardKind::Tamer;

    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX8-066 YAML parses and compiles")
        .add_card(make_filler("FILLER"))
        .add_card(opp_tamer)
        .deck(0, &filler_deck())
        .deck(1, &filler_deck())
        .memory(0)
        .start();

    runner.place_on_field(0, CARD_ID, Some(0));
    runner.place_on_field(1, "OPP-TAMER", Some(0));
    runner.game.memory = 0;

    runner.game.enter_main_phase();
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.memory(),
        0,
        "an opponent Tamer must not satisfy the \"has a Digimon\" gate"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 3 — Clause 2 behavioral: PLAY half of the observer
// ═══════════════════════════════════════════════════════════════════════════════

/// Own Ice-Snow Digimon is PLAYED → the outer optional accept/decline prompt
/// installs; accepting suspends the Tamer and trashes the chosen opponent
/// digivolution card.
#[test]
fn ex8_066_observer_fires_on_own_ice_snow_play_accept_suspends_and_trashes() {
    let mut runner = base_runner();
    let tamer = runner.place_on_field(0, CARD_ID, Some(0));
    let opp = runner.place_on_field(1, "OPP-TOP", Some(0));
    runner.push_source(opp, "OPP-SRC");

    let played = runner.place_on_field(0, "ICE-LV3", None);
    runner.fire_play_event_triggers(0, played.index as usize, false, false);
    runner.game.drain_effect_queue();

    // A single optional trigger WITH an activation cost surfaces as a
    // pre-cost TriggerOrder prompt (PASS declines, picking the entry pays the
    // cost and runs the body) — the engine's decline gate for "by suspending
    // this Tamer" (effect_queue.rs `needs_pre_cost_prompt`).
    let view = runner
        .pending_selection_view()
        .expect("pre-cost optional prompt must install when an own Ice-Snow Digimon is played");
    assert_eq!(view.kind, SelectionKind::TriggerOrder);
    assert!(view.is_optional, "PASS must be able to decline");
    assert_eq!(view.selecting_player, 0);
    assert!(
        !runner.game.players[0].battle_area[tamer.index as usize].is_suspended,
        "the suspend cost must not be paid before accept"
    );

    runner.accept_optional_trigger().expect("accept");
    runner.game.drain_effect_queue();

    // Drive the source-pick selection (exactly 1 opponent source available).
    let mut steps = 0;
    while let Some(view) = runner.pending_selection_view() {
        if steps > 8 {
            panic!("selection loop did not terminate");
        }
        runner
            .execute_action(view.selecting_player, view.valid_action_ids[0])
            .expect("pick the opponent digivolution card");
        runner.game.drain_effect_queue();
        steps += 1;
    }

    assert!(
        runner.game.players[0].battle_area[tamer.index as usize].is_suspended,
        "the Tamer must be suspended after accepting (cost paid)"
    );
    assert_eq!(
        runner.game.players[1].battle_area[opp.index as usize]
            .card_sources
            .len(),
        1,
        "the opponent Digimon must have its 1 digivolution card trashed (only top remains)"
    );
    assert_eq!(
        runner.game.players[1].trash.len(),
        1,
        "the trashed digivolution card goes to the opponent's trash"
    );
}

/// A NON-Ice-Snow own Digimon played → the observer must not fire.
#[test]
fn ex8_066_observer_does_not_fire_for_non_ice_snow_play() {
    let mut runner = base_runner();
    let tamer = runner.place_on_field(0, CARD_ID, Some(0));
    let opp = runner.place_on_field(1, "OPP-TOP", Some(0));
    runner.push_source(opp, "OPP-SRC");

    let played = runner.place_on_field(0, "PLAIN-LV3", None);
    runner.fire_play_event_triggers(0, played.index as usize, false, false);
    runner.game.drain_effect_queue();

    assert!(
        runner.pending_selection().is_none(),
        "no prompt for a non-Ice-Snow subject"
    );
    assert!(
        !runner.game.players[0].battle_area[tamer.index as usize].is_suspended,
        "the Tamer stays unsuspended"
    );
    assert!(
        runner.game.players[1].trash.is_empty(),
        "nothing is trashed"
    );
}

/// An OPPONENT'S Ice-Snow Digimon being played must NOT fire the observer
/// ("your Digimon"). OnAllyPlayed scans only the playing player's zones.
#[test]
fn ex8_066_observer_does_not_fire_for_opponent_play() {
    let mut runner = base_runner();
    let tamer = runner.place_on_field(0, CARD_ID, Some(0));

    let played = runner.place_on_field(1, "ICE-LV3", None);
    runner.fire_play_event_triggers(1, played.index as usize, false, false);
    runner.game.drain_effect_queue();

    assert!(
        runner.pending_selection().is_none(),
        "no prompt when the OPPONENT plays an Ice-Snow Digimon"
    );
    assert!(
        !runner.game.players[0].battle_area[tamer.index as usize].is_suspended,
        "the Tamer stays unsuspended"
    );
}

/// Declining the outer optional prompt leaves the Tamer unsuspended and the
/// opponent's digivolution cards intact.
#[test]
fn ex8_066_observer_decline_leaves_tamer_unsuspended_and_sources_intact() {
    let mut runner = base_runner();
    let tamer = runner.place_on_field(0, CARD_ID, Some(0));
    let opp = runner.place_on_field(1, "OPP-TOP", Some(0));
    runner.push_source(opp, "OPP-SRC");

    let played = runner.place_on_field(0, "ICE-LV3", None);
    runner.fire_play_event_triggers(0, played.index as usize, false, false);
    runner.game.drain_effect_queue();

    assert!(
        runner.pending_selection().is_some(),
        "outer optional prompt must install before the decline"
    );
    runner.decline_optional_trigger().expect("decline");
    runner.game.drain_effect_queue();

    assert!(
        !runner.game.players[0].battle_area[tamer.index as usize].is_suspended,
        "decline → the Tamer must NOT be suspended"
    );
    assert_eq!(
        runner.game.players[1].battle_area[opp.index as usize]
            .card_sources
            .len(),
        2,
        "decline → the opponent's digivolution card stays under its Digimon"
    );
    assert!(
        runner.game.players[1].trash.is_empty(),
        "decline → nothing is trashed"
    );
}

/// An already-suspended Tamer cannot pay the suspend cost → no prompt.
#[test]
fn ex8_066_observer_no_prompt_when_tamer_already_suspended() {
    let mut runner = base_runner();
    let tamer = runner.place_on_field(0, CARD_ID, Some(0));
    runner.game.players[0].battle_area[tamer.index as usize].is_suspended = true;
    let opp = runner.place_on_field(1, "OPP-TOP", Some(0));
    runner.push_source(opp, "OPP-SRC");

    let played = runner.place_on_field(0, "ICE-LV3", None);
    runner.fire_play_event_triggers(0, played.index as usize, false, false);
    runner.game.drain_effect_queue();

    assert!(
        runner.pending_selection().is_none(),
        "no prompt — the suspend cost is unpayable on a suspended Tamer"
    );
    assert!(
        runner.game.players[1].trash.is_empty(),
        "nothing is trashed"
    );
}

/// DCGO parity: with ZERO opponent digivolution cards the activation is still
/// offered (DCGO's CanActivateCondition checks only the suspend cost) —
/// accepting suspends the Tamer and the trash sub-flow silently no-ops.
#[test]
fn ex8_066_observer_zero_opponent_sources_still_allows_suspend() {
    let mut runner = base_runner();
    let tamer = runner.place_on_field(0, CARD_ID, Some(0));
    let opp = runner.place_on_field(1, "OPP-TOP", Some(0));
    // No sources pushed — the opponent Digimon has zero digivolution cards.

    let played = runner.place_on_field(0, "ICE-LV3", None);
    runner.fire_play_event_triggers(0, played.index as usize, false, false);
    runner.game.drain_effect_queue();

    let _view = runner
        .pending_selection_view()
        .expect("DCGO parity: the optional prompt installs even with zero opponent sources");
    runner.accept_optional_trigger().expect("accept");
    runner.game.drain_effect_queue();

    assert!(
        runner.pending_selection().is_none(),
        "no source-pick prompt — there is nothing to trash"
    );
    assert!(
        runner.game.players[0].battle_area[tamer.index as usize].is_suspended,
        "the suspend cost is still payable with zero opponent sources (DCGO parity)"
    );
    assert_eq!(
        runner.game.players[1].battle_area[opp.index as usize]
            .card_sources
            .len(),
        1,
        "the opponent stack is untouched"
    );
    assert!(runner.game.players[1].trash.is_empty(), "nothing trashed");
}

/// The trash pick is a real player choice ("trash ANY 1") and is
/// cross-permanent: with two opponent Digimon each carrying one digivolution
/// card, the picker must offer both, and exactly the chosen one is trashed.
#[test]
fn ex8_066_observer_trash_pick_is_cross_permanent_player_choice() {
    let mut runner = base_runner();
    let _tamer = runner.place_on_field(0, CARD_ID, Some(0));
    let opp_a = runner.place_on_field(1, "OPP-TOP", Some(0));
    runner.push_source(opp_a, "OPP-SRC");
    let opp_b = runner.place_on_field(1, "PLAIN-LV3", Some(0));
    runner.push_source(opp_b, "OPP-SRC-B");

    let played = runner.place_on_field(0, "ICE-LV3", None);
    runner.fire_play_event_triggers(0, played.index as usize, false, false);
    runner.game.drain_effect_queue();

    runner
        .pending_selection_view()
        .expect("outer optional prompt");
    runner.accept_optional_trigger().expect("accept");
    runner.game.drain_effect_queue();

    let view = runner
        .pending_selection_view()
        .expect("source-pick prompt must install");
    assert!(
        view.valid_action_ids.len() >= 2,
        "the picker must expose BOTH opponent Digimon's digivolution cards \
         (cross-permanent, RL action space sees every choice); got {}",
        view.valid_action_ids.len()
    );
    runner
        .execute_action(view.selecting_player, view.valid_action_ids[0])
        .expect("pick one source");
    runner.game.drain_effect_queue();
    // Drain any residual confirm step.
    let _ = runner.auto_resolve();

    let total_sources_left = runner.game.players[1].battle_area[opp_a.index as usize]
        .card_sources
        .len()
        + runner.game.players[1].battle_area[opp_b.index as usize]
            .card_sources
            .len();
    assert_eq!(
        total_sources_left,
        3, // 2 tops + exactly 1 surviving digivolution card
        "exactly 1 digivolution card must be trashed across the two stacks"
    );
    assert_eq!(
        runner.game.players[1].trash.len(),
        1,
        "exactly 1 card lands in the opponent's trash"
    );
}

/// [All Turns]: the observer also fires on the OPPONENT'S turn (e.g. an own
/// Digimon played by an effect during the opponent's turn).
#[test]
fn ex8_066_observer_fires_on_opponents_turn() {
    let mut runner = base_runner();
    let tamer = runner.place_on_field(0, CARD_ID, Some(0));
    let opp = runner.place_on_field(1, "OPP-TOP", Some(0));
    runner.push_source(opp, "OPP-SRC");

    // Hand the turn to player 1 — it is now the opponent's turn.
    runner.end_turn();
    let _ = runner.auto_resolve();
    assert_eq!(runner.game.turn_player(), 1, "must be P1's turn");

    // P0's Ice-Snow Digimon enters play via an effect during P1's turn.
    let played = runner.place_on_field(0, "ICE-LV3", None);
    runner.fire_play_event_triggers(0, played.index as usize, true, false);
    runner.game.drain_effect_queue();

    let view = runner
        .pending_selection_view()
        .expect("[All Turns] — the prompt must install on the opponent's turn too");
    assert_eq!(view.selecting_player, 0, "P0 owns the optional trigger");
    assert!(view.is_optional, "PASS must be able to decline");

    runner.accept_optional_trigger().expect("accept");
    runner.game.drain_effect_queue();

    // Assert the suspend BEFORE driving the source pick: resolving the last
    // selection lets the harness advance to P0's turn, whose unsuspend phase
    // legitimately clears the suspension again.
    assert!(
        runner.game.players[0].battle_area[tamer.index as usize].is_suspended,
        "the Tamer suspends on the opponent's turn as well"
    );

    let mut steps = 0;
    while let Some(view) = runner.pending_selection_view() {
        if steps > 8 {
            panic!("selection loop did not terminate");
        }
        runner
            .execute_action(view.selecting_player, view.valid_action_ids[0])
            .expect("drive selection");
        runner.game.drain_effect_queue();
        steps += 1;
    }

    assert_eq!(
        runner.game.players[1].trash.len(),
        1,
        "the opponent's digivolution card is trashed"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 4 — Clause 2 behavioral: DIGIVOLVE half of the observer
// ═══════════════════════════════════════════════════════════════════════════════

/// Own Digimon DIGIVOLVES into an Ice-Snow Digimon → prompt installs;
/// accepting suspends the Tamer and trashes the chosen opponent source.
#[test]
fn ex8_066_observer_fires_on_own_ice_snow_digivolve() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX8-066 YAML parses and compiles")
        .add_card(make_filler("FILLER"))
        .add_card(make_plain_lv3("BASE"))
        .add_card(make_ice_snow_lv4("ICE-LV4"))
        .add_card(make_test_card("OPP-SRC", "Opp Source"))
        .add_card(make_plain_lv3("OPP-TOP"))
        .hand(0, &["ICE-LV4"])
        .deck(0, &filler_deck())
        .deck(1, &filler_deck())
        .memory(3)
        .start();

    let tamer = runner.place_on_field(0, CARD_ID, Some(0));
    let base = runner.place_on_field(0, "BASE", Some(0));
    let opp = runner.place_on_field(1, "OPP-TOP", Some(0));
    runner.push_source(opp, "OPP-SRC");

    runner.game.enter_main_phase();
    let _ = runner.auto_resolve(); // SoMP clause resolves (+1 memory, mandatory)

    let hand_idx = runner
        .game
        .player(0)
        .hand
        .iter()
        .position(|c| c.card_id(&runner.game.card_data) == "ICE-LV4")
        .expect("ICE-LV4 in hand");
    assert!(
        runner
            .game
            .digivolve_from_hand(0, hand_idx, base.index as usize, PlaySource::ByDigivolve),
        "digivolve_from_hand must succeed"
    );
    runner.game.drain_effect_queue();

    // Pre-cost TriggerOrder decline gate (same contract as the play half).
    let view = runner
        .pending_selection_view()
        .expect("pre-cost optional prompt must install after the Ice-Snow digivolve");
    assert_eq!(view.kind, SelectionKind::TriggerOrder);
    assert!(view.is_optional);
    assert!(
        !runner.game.players[0].battle_area[tamer.index as usize].is_suspended,
        "cost unpaid before accept"
    );

    runner.accept_optional_trigger().expect("accept");
    runner.game.drain_effect_queue();
    let mut steps = 0;
    while let Some(view) = runner.pending_selection_view() {
        if steps > 8 {
            panic!("selection loop did not terminate");
        }
        runner
            .execute_action(view.selecting_player, view.valid_action_ids[0])
            .expect("pick the opponent digivolution card");
        runner.game.drain_effect_queue();
        steps += 1;
    }

    assert!(
        runner.game.players[0].battle_area[tamer.index as usize].is_suspended,
        "the Tamer suspends after accepting"
    );
    assert_eq!(
        runner.game.players[1].battle_area[opp.index as usize]
            .card_sources
            .len(),
        1,
        "the opponent's digivolution card is trashed (only top remains)"
    );
    assert_eq!(runner.game.players[1].trash.len(), 1);
}

/// Digivolving into a NON-Ice-Snow Digimon must not fire the observer.
#[test]
fn ex8_066_observer_does_not_fire_on_non_ice_snow_digivolve() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX8-066 YAML parses and compiles")
        .add_card(make_filler("FILLER"))
        .add_card(make_plain_lv3("BASE"))
        .add_card(make_plain_lv4("PLAIN-LV4"))
        .add_card(make_test_card("OPP-SRC", "Opp Source"))
        .add_card(make_plain_lv3("OPP-TOP"))
        .hand(0, &["PLAIN-LV4"])
        .deck(0, &filler_deck())
        .deck(1, &filler_deck())
        .memory(3)
        .start();

    let tamer = runner.place_on_field(0, CARD_ID, Some(0));
    let base = runner.place_on_field(0, "BASE", Some(0));
    let opp = runner.place_on_field(1, "OPP-TOP", Some(0));
    runner.push_source(opp, "OPP-SRC");

    runner.game.enter_main_phase();
    let _ = runner.auto_resolve();

    let hand_idx = runner
        .game
        .player(0)
        .hand
        .iter()
        .position(|c| c.card_id(&runner.game.card_data) == "PLAIN-LV4")
        .expect("PLAIN-LV4 in hand");
    assert!(
        runner
            .game
            .digivolve_from_hand(0, hand_idx, base.index as usize, PlaySource::ByDigivolve),
        "digivolve_from_hand must succeed"
    );
    runner.game.drain_effect_queue();

    assert!(
        runner.pending_selection().is_none(),
        "no prompt — the digivolve subject has no Ice-Snow trait"
    );
    assert!(
        !runner.game.players[0].battle_area[tamer.index as usize].is_suspended,
        "the Tamer stays unsuspended"
    );
    assert!(runner.game.players[1].trash.is_empty(), "nothing trashed");
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 5 — Clause 3: [Security] smoke
// ═══════════════════════════════════════════════════════════════════════════════

/// Pushing EX8-066 into P0's security and enqueueing the security timing does
/// not panic — trigger registration is sound. (Full security-play behavior is
/// exercised end-to-end by sibling Tamer suites sharing `play_from_security`.)
#[test]
fn ex8_066_security_clause_fires_without_panic() {
    let mut runner = base_runner();

    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == CARD_ID)
        .expect("EX8-066 in card data");
    let next_idx = runner.game.next_card_index();
    let card = digimon_engine::card_source::CardSource::new(data_idx, 0, next_idx);
    runner.game.players[0].security.push(card);

    runner.game.enqueue_triggered(
        EffectTiming::SecuritySkill,
        TriggerSource::PlayerBattleArea(0),
    );
    runner.game.drain_effect_queue();
    let _ = runner.auto_resolve();
}
