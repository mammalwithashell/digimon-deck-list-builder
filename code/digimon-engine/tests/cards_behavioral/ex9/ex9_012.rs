//! EX9-012 MetalGreymon: Alterous Mode — Digimon, Lv.5, Red+Black, DP 8000, Cost 8.
//! Traits: Cyborg, ADVENTURE.
//!
//! # Card text (cards.json — printed)
//!
//! [On Play] [When Digivolving] Delete 1 of your opponent's Digimon with
//! 8000 DP or less.
//!
//! [Your Turn] When any of your Digimon or Tamers with [Garurumon] or
//! [Tai Kamiya] in their names are played or any of your Digimon digivolve
//! into a Digimon with [Garurumon] in its name, this Digimon may digivolve
//! into a Digimon card with [Greymon] in its name in the hand without paying
//! the cost.
//!
//! Inherited Effect:
//! [Your Turn] This Digimon gets +4000 DP.
//!
//! Standard digivolve: Lv.4 Red / Cost 4.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/EX9/Red/EX9_012.cs
//!
//! # Patterns this test covers
//! - [On Play][When Digivolving] mandatory delete with `dp_lte: 8000` literal
//!   filter (BT17-015 / EX4-060 idiom; G-PRED-DP-LTE literal form is wired)
//! - [Your Turn] cross-permanent observer → effect-initiated free digivolve
//!   with `target: self` (AD1-001 / AD1-010 idiom; bypasses
//!   G-EFFECT-INITIATED-DIGIVOLVE-FROM-HAND-WITH-PERMANENT-TARGET)
//! - Inherited [Your Turn] +4000 DP self-aura (BT23-008 idiom)
//! - `active_when: { your_turn: true }` gating on observer clauses
//!
//! # Known engine/DSL gaps affecting these tests
//!
//! - **G-EVENT-CARD-EFFECT-INITIATED-DIGIVOLVE** (per AD1-010 docstring):
//!   `event_card` is wired for normal hand-played battle-area permanents
//!   (`OnEnterFieldAnyone` with `TriggerSource::EnteredField`) and for
//!   `Game::digivolve_from_hand` battle-area digivolves. Effect-initiated /
//!   DNA / breeding-area digivolve `event_card` coverage is open.
//!
//! - **G-EVENT-CARD-TAMER-PLAY** (per AD1-010 docstring §G-EVENT-CARD-TAMER-PLAY):
//!   Tamer-play `event_card` parity through `TriggerSource::EnteredField` is
//!   unconfirmed. Test for the Tai-Kamiya tamer play is `#[ignore]`'d under
//!   the same gap tag.

#![allow(dead_code, unused_imports)]

use digimon_dsl::compiled::{
    CompiledAltPath, CompiledAltPathKind, CompiledClause, CompiledCost, CompiledDeclarativeClause,
    CompiledScope, CompiledTiming, CompiledTriggeredClause,
};
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardKind, EffectTiming, Keyword, PlayerId, PlaySource};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::{SelectionKind, TriggerSource};

use crate::dsl_card_data::card_data_from_compiled;

// ─── Fixtures ────────────────────────────────────────────────────────────────

fn make_digimon(id: &str, name: &str, level: u8, dp: i32) -> CardData {
    let mut c = make_test_card(id, name);
    c.level = Some(level);
    c.dp = Some(dp);
    c
}

fn make_filler(id: &str) -> CardData {
    make_test_card(id, id)
}

fn make_named_tamer(id: &str, name: &str) -> CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = CardKind::Tamer;
    c
}

/// A Lv.6 Greymon-named hand card with an evo_cost matching Lv.5 → Lv.6 so
/// it can be free-digivolved onto EX9-012 (Lv.5) by the [Your Turn] observer.
/// (`ignore_color: true` in `effect_initiated_digivolve` bypasses color match;
/// the level entry must align with the carrier's current level.)
fn make_grey_lv5(id: &str) -> CardData {
    let mut c = make_digimon(id, "Greymon", 6, 11000);
    c.evo_costs = vec![EvoCost {
        level: 5,
        memory_cost: 4,
        card_color: 0, // Red — not consulted under ignore_color
    }];
    c
}

/// A Lv.4 Garurumon-named hand card with a Lv.3 evo_cost (so it can digivolve
/// onto a Lv.3 platform). Used by the on_digivolve observer trigger.
fn make_garu_lv4_evo3(id: &str) -> CardData {
    let mut c = make_digimon(id, "Garurumon", 4, 4000);
    c.evo_costs = vec![EvoCost {
        level: 3,
        memory_cost: 0,
        card_color: 0, // Red
    }];
    c
}

/// Standard fixture: EX9-012 registered + a generic Lv.3 platform + filler.
/// Memory pre-set to 12 so plays don't seesaw mid-test.
fn metalgrey_runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("EX9-012")
        .expect("EX9-012 found in embedded DSL pack")
        .add_card(make_digimon("LV3-PLAT", "Lv3Platform", 3, 3000))
        .add_card(make_filler("FILL"))
        .memory(12)
        .start()
}

/// Build a runner with EX9-012 registered and stacked under a Lv.6 dummy
/// Digimon on player 0's battle area. Used by inherited DP tests.
fn runner_with_ex9_012_as_source() -> (DebugRunner, PermanentHandle) {
    let ex9_data = card_data_from_compiled("EX9-012");
    let mut carrier = make_test_card("CARRIER-LV6", "CarrierLv6");
    carrier.level = Some(6);
    carrier.dp = Some(9000);

    let mut runner = DebugRunner::builder()
        .add_card(ex9_data)
        .add_card(carrier)
        .memory(10)
        .start();

    let ex9_handle = runner.place_on_field(0, "EX9-012", Some(0));
    let carrier_data_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == "CARRIER-LV6")
        .expect("CARRIER-LV6 in card_data");
    let next_idx = runner.game.next_card_index();
    let carrier_source = CardSource::new(carrier_data_idx, 0, next_idx);
    let turn = runner.game.turn_count;
    runner.game.players[0].battle_area[ex9_handle.index as usize]
        .digivolve(carrier_source, turn);

    (runner, ex9_handle)
}

/// Push a CardSource referencing `card_id` into player `p`'s hand.
fn push_to_hand(runner: &mut DebugRunner, p: PlayerId, card_id: &str) {
    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .unwrap_or_else(|| panic!("push_to_hand: unknown card_id {}", card_id));
    let next_idx = runner.game.next_card_index();
    let card = CardSource::new(data_idx, p, next_idx);
    runner.game.players[p as usize].hand.push(card);
}

/// Find a card's index in a player's hand by card_id.
fn find_hand_index(runner: &DebugRunner, player: u8, card_id: &str) -> Option<usize> {
    let data = &runner.game.card_data;
    runner.game.players[player as usize]
        .hand
        .iter()
        .position(|c| c.card_id(data) == card_id)
}

/// Drain pending selection by repeatedly picking the first legal action.
fn drain_pending(runner: &mut DebugRunner) {
    let mut steps = 0;
    while runner.game.pending_selection.is_some() && steps < 30 {
        let p = runner
            .game
            .pending_selection
            .as_ref()
            .unwrap()
            .selecting_player;
        let a = runner
            .game
            .pending_selection
            .as_ref()
            .unwrap()
            .valid_action_ids[0];
        runner.game.resolve_selection(p, a).ok();
        runner.game.drain_effect_queue();
        steps += 1;
    }
}

// ─── SECTION 1 — Structural assertions ───────────────────────────────────────

#[test]
fn ex9_012_compiles_in_embedded_pack() {
    let runner = metalgrey_runner();
    assert!(
        runner.compiled_card("EX9-012").is_some(),
        "EX9-012 must be present in embedded DSL pack"
    );
}

#[test]
fn ex9_012_card_metadata_matches_print() {
    let runner = metalgrey_runner();
    let card = runner.compiled_card("EX9-012").expect("EX9-012 compiles");

    assert_eq!(card.level, Some(5), "MetalGreymon: Alterous Mode is Lv.5");
    assert_eq!(card.dp, Some(8000), "EX9-012 is DP 8000");
    assert_eq!(card.cost, Some(8), "EX9-012 costs 8 to play");
}

/// Standard alt-digivolve: Lv.4 Red / Cost 4 must be present.
#[test]
fn ex9_012_standard_digivolve_alt_path_present() {
    let runner = metalgrey_runner();
    let card = runner.compiled_card("EX9-012").expect("EX9-012 compiles");

    let std_path = card
        .alt_paths
        .iter()
        .find(|p| {
            p.kind == CompiledAltPathKind::Digivolve
                && p.cost == Some(CompiledCost::Literal(4))
        })
        .expect("EX9-012 must declare a Lv.4 digivolve path with cost 4");

    let from = std_path
        .from
        .as_ref()
        .expect("standard digivolve must declare a `from:` predicate");
    assert_eq!(
        from.level_eq,
        Some(4),
        "standard digivolve must require level_eq: 4"
    );
}

/// One triggered clause must fire on [OnPlay, WhenDigivolving] (the delete arm),
/// be NON-optional (no "may"), and NOT once-per-turn.
#[test]
fn ex9_012_has_on_play_when_digivolving_delete_clause() {
    let runner = metalgrey_runner();
    let card = runner.compiled_card("EX9-012").expect("EX9-012 compiles");

    let clause = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| {
            t.when.contains(&CompiledTiming::OnPlay)
                && t.when.contains(&CompiledTiming::WhenDigivolving)
        })
        .expect("EX9-012 must have a triggered clause covering OnPlay + WhenDigivolving");

    assert_eq!(
        clause.scope,
        CompiledScope::FaceUp,
        "OnPlay/WhenDigivolving delete clause must have FaceUp scope"
    );
    assert!(
        !clause.optional,
        "OnPlay/WhenDigivolving delete clause is mandatory (no \"may\" in printed text)"
    );
    assert!(
        !clause.once_per_turn,
        "OnPlay/WhenDigivolving delete clause is NOT printed [Once Per Turn]"
    );
}

/// At least one triggered clause must fire on the [Your Turn] observer
/// (on_enter_field_anyone OR on_digivolve), be optional ("may digivolve"),
/// face-up, and gated by `active_when: { your_turn: true }`.
#[test]
fn ex9_012_has_your_turn_observer_clauses() {
    let runner = metalgrey_runner();
    let card = runner.compiled_card("EX9-012").expect("EX9-012 compiles");

    let observers: Vec<_> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .filter(|t| {
            (t.when.contains(&CompiledTiming::OnEnterFieldAnyone)
                || t.when.contains(&CompiledTiming::OnDigivolve))
                && !t.when.contains(&CompiledTiming::OnPlay)
        })
        .collect();

    assert!(
        !observers.is_empty(),
        "EX9-012 must have at least one [Your Turn] observer clause"
    );

    let has_play_obs = observers
        .iter()
        .any(|t| t.when.contains(&CompiledTiming::OnEnterFieldAnyone));
    let has_digi_obs = observers
        .iter()
        .any(|t| t.when.contains(&CompiledTiming::OnDigivolve));
    assert!(
        has_play_obs,
        "must observe play half via on_enter_field_anyone"
    );
    assert!(has_digi_obs, "must observe digivolve half via on_digivolve");

    for obs in &observers {
        assert_eq!(
            obs.scope,
            CompiledScope::FaceUp,
            "[Your Turn] observer must be face-up scope"
        );
        assert!(
            obs.optional,
            "[Your Turn] observer is optional (\"may digivolve\")"
        );
        assert!(
            !obs.once_per_turn,
            "[Your Turn] observer is NOT printed [Once Per Turn]"
        );
        assert!(
            obs.active_when.is_some(),
            "[Your Turn] observer must declare active_when (your_turn: true)"
        );
    }
}

/// The inherited [Your Turn] +4000 DP self-aura must be present with the
/// right scope, dp_modifier, and active_when.
#[test]
fn ex9_012_has_inherited_your_turn_dp_aura() {
    let runner = metalgrey_runner();
    let card = runner.compiled_card("EX9-012").expect("EX9-012 compiles");

    let aura = card.effects.iter().find_map(|c| match c {
        CompiledClause::Declarative(CompiledDeclarativeClause::Aura {
            scope,
            dp_modifier,
            active_when,
            ..
        }) => Some((*scope, *dp_modifier, active_when.clone())),
        _ => None,
    });

    let (scope, dp, active_when) =
        aura.expect("EX9-012 must have a Declarative::Aura clause (inherited +4000 DP)");
    assert_eq!(
        scope,
        CompiledScope::Inherited,
        "the aura clause must have Inherited scope"
    );
    assert_eq!(dp, Some(4000), "the aura dp_modifier must be +4000 DP");
    assert!(
        active_when.is_some(),
        "the aura must declare active_when (your_turn: true)"
    );
}

// ─── SECTION 2 — Behavioral: [On Play] delete arm ───────────────────────────

/// Negative gate: when no opp Digimon ≤ 8000 DP exists, playing EX9-012 must
/// not install a select prompt.
#[test]
fn ex9_012_on_play_no_eligible_target_no_prompt() {
    let mut runner = metalgrey_runner();
    runner.game.card_data.push(make_digimon("BIG", "BigBoy", 6, 12000));
    push_to_hand(&mut runner, 0, "EX9-012");
    // Place an opp Digimon that's TOO LARGE to be deleted.
    runner.place_on_field(1, "BIG", None);

    let hand_idx = find_hand_index(&runner, 0, "EX9-012").expect("EX9-012 in hand");
    runner.play(0, hand_idx).expect("EX9-012 plays");
    runner.game.drain_effect_queue();

    assert!(
        runner.pending_selection().is_none(),
        "no opp Digimon ≤ 8000 DP → no select prompt"
    );
}

/// Positive gate: when an opp Digimon ≤ 8000 DP exists, playing EX9-012 must
/// install a MANDATORY select prompt (no "may" → not optional).
#[test]
fn ex9_012_on_play_with_eligible_target_installs_mandatory_prompt() {
    let mut runner = metalgrey_runner();
    runner.game.card_data.push(make_digimon("VICTIM", "Victim", 4, 5000));
    push_to_hand(&mut runner, 0, "EX9-012");
    runner.place_on_field(1, "VICTIM", None);

    let hand_idx = find_hand_index(&runner, 0, "EX9-012").expect("EX9-012 in hand");
    runner.play(0, hand_idx).expect("EX9-012 plays");
    runner.game.drain_effect_queue();

    let pending = runner
        .pending_selection()
        .expect("delete prompt must install when eligible target exists");
    assert!(
        !pending.is_optional,
        "delete prompt is mandatory (no \"may\" in printed text)"
    );
}

/// On Play accept-and-resolve: picking the eligible victim deletes it
/// (opp battle_area shrinks by 1).
#[test]
fn ex9_012_on_play_resolves_to_delete() {
    let mut runner = metalgrey_runner();
    runner.game.card_data.push(make_digimon("VICTIM", "Victim", 4, 5000));
    push_to_hand(&mut runner, 0, "EX9-012");
    runner.place_on_field(1, "VICTIM", None);

    let opp_field_before = runner.game.players[1].battle_area.len();

    let hand_idx = find_hand_index(&runner, 0, "EX9-012").expect("EX9-012 in hand");
    runner.play(0, hand_idx).expect("EX9-012 plays");
    runner.game.drain_effect_queue();
    drain_pending(&mut runner);

    assert_eq!(
        runner.game.players[1].battle_area.len(),
        opp_field_before - 1,
        "the eligible opp Digimon must be deleted"
    );
}

/// DP filter — opp Digimon EXACTLY at 8000 DP qualifies (≤ comparator).
#[test]
fn ex9_012_on_play_includes_dp_exactly_8000() {
    let mut runner = metalgrey_runner();
    runner.game.card_data.push(make_digimon("EQ8K", "Eq8k", 5, 8000));
    push_to_hand(&mut runner, 0, "EX9-012");
    runner.place_on_field(1, "EQ8K", None);

    let hand_idx = find_hand_index(&runner, 0, "EX9-012").expect("EX9-012 in hand");
    runner.play(0, hand_idx).expect("EX9-012 plays");
    runner.game.drain_effect_queue();

    assert!(
        runner.pending_selection().is_some(),
        "8000-DP target must qualify under dp_lte: 8000"
    );
}

/// DP filter — opp Digimon at 9000 DP does NOT qualify.
#[test]
fn ex9_012_on_play_excludes_dp_above_8000() {
    let mut runner = metalgrey_runner();
    runner.game.card_data.push(make_digimon("OVER", "Over", 5, 9000));
    push_to_hand(&mut runner, 0, "EX9-012");
    runner.place_on_field(1, "OVER", None);

    let hand_idx = find_hand_index(&runner, 0, "EX9-012").expect("EX9-012 in hand");
    runner.play(0, hand_idx).expect("EX9-012 plays");
    runner.game.drain_effect_queue();

    assert!(
        runner.pending_selection().is_none(),
        "9000-DP target must NOT qualify under dp_lte: 8000"
    );
}

// ─── SECTION 3 — Behavioral: [Your Turn] observer (play half) ───────────────

/// Negative gate (event ownership): the observer must NOT fire on an
/// opponent's Garurumon play. Card text scopes to "your Digimon or Tamers".
#[test]
fn ex9_012_observer_does_not_fire_on_opponent_garurumon_play() {
    let mut runner = metalgrey_runner();
    runner.game.card_data.push(make_grey_lv5("GREY-LV5"));
    let mut opp_garu = make_digimon("OPP-GARU", "Garurumon", 4, 4000);
    opp_garu.play_cost = 0;
    runner.game.card_data.push(opp_garu);
    push_to_hand(&mut runner, 0, "GREY-LV5");
    push_to_hand(&mut runner, 1, "OPP-GARU");
    let _ex9 = runner.place_on_field(0, "EX9-012", Some(0));
    runner.game.drain_effect_queue();
    drain_pending(&mut runner);

    runner.end_turn();
    assert_eq!(runner.turn_player(), 1, "precondition: now P1's turn");

    let hand_idx = find_hand_index(&runner, 1, "OPP-GARU").expect("OPP-GARU in hand");
    runner.play(1, hand_idx).expect("opponent's Garurumon plays");
    runner.game.drain_effect_queue();

    assert!(
        runner.pending_selection().is_none(),
        "opponent's Garurumon play must not fire the observer"
    );
}

/// Negative gate (your_turn): when it's the OPPONENT'S turn, even an own
/// Garurumon play (e.g. counter-timing or via opp effect) must not fire the
/// observer — the [Your Turn] gate suppresses it. We approximate this by
/// ending the turn so it's P1's turn, then placing an OWN Garurumon directly
/// onto P0's field via `place_on_field`. The trigger fires on the placement
/// path; the active_when gate must suppress.
#[test]
fn ex9_012_observer_suppressed_on_opponents_turn() {
    let mut runner = metalgrey_runner();
    runner.game.card_data.push(make_grey_lv5("GREY-LV5"));
    push_to_hand(&mut runner, 0, "GREY-LV5");
    let _ex9 = runner.place_on_field(0, "EX9-012", Some(0));
    runner.game.drain_effect_queue();
    drain_pending(&mut runner);

    // Switch to opponent's turn.
    runner.end_turn();
    assert_eq!(runner.turn_player(), 1, "precondition: now P1's turn");

    // Direct-place an own Garurumon (bypasses memory cost and still fires
    // the OnEnterFieldAnyone trigger).
    runner.game.card_data.push(make_digimon("GARU", "Garurumon", 4, 4000));
    let _g = runner.place_on_field(0, "GARU", Some(0));
    runner.game.drain_effect_queue();

    assert!(
        runner.pending_selection().is_none(),
        "[Your Turn] active_when must suppress observer on opponent's turn"
    );
}

/// Positive gate (play half): when an own [Garurumon]-named Digimon is
/// played on the controller's turn AND a Greymon-named hand card is
/// available, the observer fires its optional digivolve prompt.
#[test]
fn ex9_012_observer_fires_on_own_garurumon_play() {
    let mut runner = metalgrey_runner();
    runner.game.card_data.push(make_grey_lv5("GREY-LV5"));
    runner.game.card_data.push(make_digimon("GARU", "Garurumon", 4, 4000));
    push_to_hand(&mut runner, 0, "GREY-LV5");
    push_to_hand(&mut runner, 0, "GARU");
    let _ex9 = runner.place_on_field(0, "EX9-012", Some(0));
    runner.game.drain_effect_queue();
    drain_pending(&mut runner);

    let hand_idx = find_hand_index(&runner, 0, "GARU").expect("GARU in hand");
    runner.play(0, hand_idx).expect("Garurumon plays");
    runner.game.drain_effect_queue();

    let pending = runner
        .pending_selection()
        .expect("observer must install a prompt when own Garurumon plays on your turn");
    assert!(
        pending.is_optional,
        "observer prompt is optional (\"may digivolve\")"
    );
}

/// Negative gate (name filter): when an own UNRELATED Digimon is played, the
/// observer must NOT fire (name filter requires Garurumon or Tai Kamiya).
#[test]
fn ex9_012_observer_does_not_fire_on_unrelated_own_play() {
    let mut runner = metalgrey_runner();
    runner.game.card_data.push(make_grey_lv5("GREY-LV5"));
    runner
        .game
        .card_data
        .push(make_digimon("UNREL", "RandomMon", 3, 3000));
    push_to_hand(&mut runner, 0, "GREY-LV5");
    push_to_hand(&mut runner, 0, "UNREL");
    let _ex9 = runner.place_on_field(0, "EX9-012", Some(0));
    runner.game.drain_effect_queue();
    drain_pending(&mut runner);

    let hand_idx = find_hand_index(&runner, 0, "UNREL").expect("UNREL in hand");
    runner.play(0, hand_idx).expect("UNREL plays");
    runner.game.drain_effect_queue();

    assert!(
        runner.pending_selection().is_none(),
        "unrelated-name own play must not fire the observer"
    );
}

/// Positive gate (Tai Kamiya tamer play). IGNORED pending engine confirmation
/// that `event_card` is populated for tamer plays via TriggerSource::EnteredField
/// (per AD1-010 docstring §G-EVENT-CARD-TAMER-PLAY).
#[test]
#[ignore = "pending: G-EVENT-CARD-TAMER-PLAY — event_card population for tamer plays via TriggerSource::EnteredField is unconfirmed in engine-gaps.md"]
fn ex9_012_observer_fires_on_own_tai_kamiya_tamer_play() {
    unimplemented!("blocked on G-EVENT-CARD-TAMER-PLAY")
}

// ─── SECTION 4 — Behavioral: [Your Turn] observer (digivolve half) ──────────

/// Positive gate (digivolve half): when an own Digimon digivolves into a
/// Garurumon-named card on the controller's turn, the observer fires.
#[test]
fn ex9_012_observer_fires_on_own_garurumon_digivolve() {
    let mut runner = metalgrey_runner();
    runner.game.card_data.push(make_grey_lv5("GREY-LV5"));
    runner.game.card_data.push(make_garu_lv4_evo3("GARU4"));
    push_to_hand(&mut runner, 0, "GREY-LV5");
    push_to_hand(&mut runner, 0, "GARU4");
    let _ex9 = runner.place_on_field(0, "EX9-012", Some(0));
    let base = runner.place_on_field(0, "LV3-PLAT", Some(0));
    runner.game.drain_effect_queue();
    drain_pending(&mut runner);

    let hand_idx = find_hand_index(&runner, 0, "GARU4").expect("GARU4 in hand");
    let ok = runner.game.digivolve_from_hand(
        0,
        hand_idx,
        base.index as usize,
        PlaySource::ByDigivolve,
    );
    assert!(ok, "digivolve_from_hand must succeed");
    runner.game.drain_effect_queue();

    assert!(
        runner.pending_selection().is_some(),
        "observer must install a prompt when own Digimon digivolves into [Garurumon]"
    );
}

/// Negative gate (digivolve into unrelated): digivolving into a non-Garurumon
/// card must not fire the observer.
#[test]
fn ex9_012_observer_does_not_fire_on_unrelated_digivolve() {
    let mut runner = metalgrey_runner();
    runner.game.card_data.push(make_grey_lv5("GREY-LV5"));
    let mut unrel = make_digimon("UNRELLV4", "RandomLv4", 4, 4000);
    unrel.evo_costs = vec![EvoCost {
        level: 3,
        memory_cost: 0,
        card_color: 0,
    }];
    runner.game.card_data.push(unrel);
    push_to_hand(&mut runner, 0, "GREY-LV5");
    push_to_hand(&mut runner, 0, "UNRELLV4");
    let _ex9 = runner.place_on_field(0, "EX9-012", Some(0));
    let base = runner.place_on_field(0, "LV3-PLAT", Some(0));
    runner.game.drain_effect_queue();
    drain_pending(&mut runner);

    let hand_idx = find_hand_index(&runner, 0, "UNRELLV4").expect("UNRELLV4 in hand");
    let ok = runner.game.digivolve_from_hand(
        0,
        hand_idx,
        base.index as usize,
        PlaySource::ByDigivolve,
    );
    assert!(ok, "digivolve_from_hand must succeed");
    runner.game.drain_effect_queue();

    assert!(
        runner.pending_selection().is_none(),
        "unrelated-name digivolve must not fire the observer"
    );
}

// ─── SECTION 5 — Behavioral: observer end-to-end (free digivolve) ───────────

/// Accept the observer's optional prompt and select a Greymon-named hand
/// card. EX9-012's slot must end with a Greymon-named top card; the
/// digivolve cost must NOT come off memory (cost: 0).
#[test]
fn ex9_012_observer_accept_free_digivolves_self_into_greymon() {
    let mut runner = metalgrey_runner();
    runner.game.card_data.push(make_grey_lv5("GREY-LV5"));
    runner.game.card_data.push(make_digimon("GARU", "Garurumon", 4, 4000));
    push_to_hand(&mut runner, 0, "GREY-LV5");
    push_to_hand(&mut runner, 0, "GARU");
    let ex9 = runner.place_on_field(0, "EX9-012", Some(0));
    runner.game.drain_effect_queue();
    drain_pending(&mut runner);

    let hand_before = runner.hand_size(0);

    let hand_idx = find_hand_index(&runner, 0, "GARU").expect("GARU in hand");
    runner.play(0, hand_idx).expect("Garurumon plays");
    runner.game.drain_effect_queue();
    let memory_after_play = runner.memory();

    // Drive the affirmative branch through the observer prompts (AD1-001 idiom).
    let mut steps = 0;
    while runner.pending_selection().is_some() && steps < 10 {
        let (player, action) = {
            let s = runner.pending_selection().unwrap();
            (s.selecting_player, s.valid_action_ids[0])
        };
        runner
            .execute_action(player, action)
            .expect("resolve observer prompt step");
        steps += 1;
    }

    // After resolution, EX9-012's slot top card must contain "Greymon".
    let perm = &runner.game.players[0].battle_area[ex9.index as usize];
    let top_name = perm.top_card().card_name(&runner.game.card_data);
    assert!(
        top_name.contains("Greymon"),
        "after free-digivolve, EX9-012 slot top card must contain \"Greymon\"; got {:?}",
        top_name
    );
    // No memory payment from cost-0 digivolve.
    assert!(
        runner.memory() >= memory_after_play,
        "free-digivolve must not pay memory; memory_after_play={}, memory_after_observer={}",
        memory_after_play,
        runner.memory()
    );
    // Hand: -2 (Garurumon played + Greymon free-digivolved on top of EX9-012).
    assert_eq!(
        runner.hand_size(0),
        hand_before - 2,
        "hand decreased by 2: Garurumon played, Greymon free-digivolved"
    );
}

/// Decline path: passing the observer leaves EX9-012's slot untransformed.
#[test]
fn ex9_012_observer_decline_leaves_self_untouched() {
    let mut runner = metalgrey_runner();
    runner.game.card_data.push(make_grey_lv5("GREY-LV5"));
    runner.game.card_data.push(make_digimon("GARU", "Garurumon", 4, 4000));
    push_to_hand(&mut runner, 0, "GREY-LV5");
    push_to_hand(&mut runner, 0, "GARU");
    let ex9 = runner.place_on_field(0, "EX9-012", Some(0));
    runner.game.drain_effect_queue();
    drain_pending(&mut runner);

    let hand_idx = find_hand_index(&runner, 0, "GARU").expect("GARU in hand");
    runner.play(0, hand_idx).expect("Garurumon plays");
    runner.game.drain_effect_queue();

    if let Some(s) = runner.pending_selection() {
        assert!(s.is_optional, "observer prompt must be optional");
        let player = s.selecting_player;
        runner
            .execute_action(player, digimon_engine::action::space::PASS)
            .expect("PASS to decline");
    }
    runner.auto_resolve().ok();

    let perm = &runner.game.players[0].battle_area[ex9.index as usize];
    assert_eq!(
        perm.top_card().card_id(&runner.game.card_data),
        "EX9-012",
        "after decline EX9-012 slot top card must remain EX9-012"
    );
}

// ─── SECTION 6 — Inherited [Your Turn] +4000 DP ──────────────────────────────

/// When EX9-012 is a digivolution source under a carrier on the controller's
/// own turn, `source_dp_contribution` must return +4000 for EX9-012's slot.
#[test]
fn ex9_012_inherited_dp_active_on_your_turn() {
    let (runner, carrier_handle) = runner_with_ex9_012_as_source();
    let ex9_src_idx = 0;

    assert_eq!(runner.turn_player(), 0, "precondition: it is P0's turn");

    let contribution = runner
        .game
        .source_dp_contribution(carrier_handle, ex9_src_idx);

    assert_eq!(
        contribution, 4000,
        "EX9-012's inherited +4000 DP must contribute on the controller's own turn; got {}",
        contribution
    );
}

/// On the opponent's turn, the [Your Turn] gate must suppress the DP buff.
#[test]
fn ex9_012_inherited_dp_inactive_on_opponents_turn() {
    let (mut runner, carrier_handle) = runner_with_ex9_012_as_source();
    runner.end_turn();
    assert_eq!(runner.turn_player(), 1, "precondition: it is now P1's turn");

    let ex9_src_idx = 0;
    let contribution = runner
        .game
        .source_dp_contribution(carrier_handle, ex9_src_idx);

    assert_eq!(
        contribution, 0,
        "EX9-012's inherited +4000 DP must NOT contribute on opponent's turn; got {}",
        contribution
    );
}
