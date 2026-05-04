//! AD1-010 Garurumon — Digimon, Lv.4, Blue, Beast / ADVENTURE.
//!
//! # Card text (cards.json)
//!
//! [On Play] [When Digivolving] <Draw 1> (Draw 1 card from your deck.)
//! [All Turns] When your Digimon or Tamers are played or digivolve, if any of
//! them have [Greymon] or [Matt Ishida] in their names, this Digimon may
//! digivolve into a Digimon card with [Garurumon] in its name in the hand
//! without paying the cost.
//!
//! Inherited Effect <Jamming>
//!   (This Digimon can't be deleted in battles against Security Digimon.)
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/AD1/Blue/AD1_010.cs
//!
//! # Patterns this test covers
//! - Alt-digivolve OR-shaped: Lv.3 with [Omnimon] in name OR [ADVENTURE] trait,
//!   cost 2, ignoreDigivolutionRequirement=false (BT24-011 idiom — two alt_paths).
//! - On Play / When Digivolving shared <Draw 1>.
//! - All-Turns observer firing on play (`on_enter_field_anyone`) and
//!   digivolve (`on_digivolve`) with `event_card_name_contains` filter for
//!   [Greymon] or [Matt Ishida] (RESOLVED 2026-04-29 for hand-played battle-area
//!   permanents and battle-area digivolves).
//! - Inherited <Jamming> via `scope: inherited / kind: grant_keyword` (BT24-011 idiom).
//!
//! # Known engine and DSL gaps affecting these tests
//!
//! G-EVENT-CARD-EFFECT-INITIATED-DIGIVOLVE [hybrid]:
//!   `event_card` is wired for `Game::digivolve_from_hand` (user action), but
//!   effect-initiated digivolve, DNA digivolve, and breeding-area digivolve
//!   event-context coverage are open follow-ups (engine-gaps.md, 2026-04-29).
//!   Tests that assert the All-Turns observer fires off effect-initiated
//!   digivolves are scoped to user-action plays/digivolves only.
//!
//! G-EVENT-CARD-TAMER-PLAY [hybrid, suspected]:
//!   `OnEnterFieldAnyone` with TriggerSource::EnteredField is documented as
//!   "normal hand-played battle-area permanents" (engine-gaps.md, 2026-04-29).
//!   Tamers fire `OnEnterFieldAnyone` (DCGO trigger), but it has not been
//!   independently confirmed that `event_card` carries the tamer's data in
//!   `TriggerContext` for tamer plays. The Matt Ishida test is `#[ignore]`'d
//!   pending confirmation.
//!
//! G-OPT-NOTE: The card has no [Once Per Turn] cap on either clause — the
//! All-Turns observer is repeatable per qualifying event. No OPT lockout test
//! is required.

#![allow(dead_code, unused_imports)]

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledDeclarativeClause, CompiledScope, CompiledTiming,
};
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{EffectTiming, PlaySource};
use digimon_engine::events::GameEvent;
#[allow(unused_imports)]
use digimon_engine::selection::TriggerSource;

// ─── helpers ──────────────────────────────────────────────────────────────────

fn make_filler(id: &str) -> CardData {
    make_test_card(id, id)
}

fn make_digimon(id: &str, name: &str, level: u8, dp: i32) -> CardData {
    let mut c = make_test_card(id, name);
    c.dp = Some(dp);
    c.level = Some(level);
    c
}

fn make_named_digimon(id: &str, name: &str) -> CardData {
    make_digimon(id, name, 4, 5000)
}

/// A Lv.3 ADVENTURE-trait Digimon, suitable as an alt-digivolve base for
/// AD1-010 (which alt-digivolves from Lv.3 with [Omnimon] in name OR
/// [ADVENTURE] trait, cost 2).
fn make_adventure_lvl3(id: &str, name: &str) -> CardData {
    let mut c = make_digimon(id, name, 3, 3000);
    c.traits = vec!["ADVENTURE".to_string()];
    // Also a normal Lv.3 Red evo target, so a generic Lv.4 with
    // EvoCost{lvl=3, color=Red} can digivolve onto it via the printed path.
    c
}

/// A Lv.4 Digimon with a printed `Lv.3 Red, cost 2` evo_cost, suitable for
/// digivolving onto a stock `make_adventure_lvl3` base via the standard
/// `printed_digivolve_memory_cost` path.
fn make_lvl4_red_evo3(id: &str, name: &str) -> CardData {
    let mut c = make_digimon(id, name, 4, 5000);
    c.evo_costs = vec![EvoCost {
        card_color: 0, // Red
        level: 3,
        memory_cost: 2,
    }];
    c
}

/// Find a card's index in a player's hand by card_id. `CardSource::card_id`
/// requires `&card_data`, so we look up the card_data slice through the runner.
fn find_hand_index(runner: &DebugRunner, player: u8, card_id: &str) -> Option<usize> {
    let data = &runner.game.card_data;
    runner.game.players[player as usize]
        .hand
        .iter()
        .position(|c| c.card_id(data) == card_id)
}

/// Drain any pending selection by repeatedly picking the first legal action.
/// Capped at 30 iterations to prevent infinite loops in test failures.
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

/// AD1-010 must compile with at least:
///   - 1+ triggered "draw" clauses covering on_play and when_digivolving
///   - 1+ triggered All-Turns observer covering on_enter_field_anyone and
///     on_digivolve
///   - 1 declarative inherited keyword grant (Jamming)
#[test]
fn ad1_010_compiles_with_expected_clause_shape() {
    let runner = DebugRunner::builder()
        .dsl_card("AD1-010")
        .expect("AD1-010 in embedded pack")
        .memory(0)
        .start();

    let compiled = runner
        .compiled_card("AD1-010")
        .expect("compiled card present");

    let triggered: Vec<_> = compiled
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();

    let declarative: Vec<_> = compiled
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Declarative(d) => Some(d),
            _ => None,
        })
        .collect();

    let has_on_play = triggered
        .iter()
        .any(|t| t.when.contains(&CompiledTiming::OnPlay));
    let has_when_digi = triggered
        .iter()
        .any(|t| t.when.contains(&CompiledTiming::WhenDigivolving));
    let has_all_turns_play = triggered
        .iter()
        .any(|t| t.when.contains(&CompiledTiming::OnEnterFieldAnyone));
    let has_all_turns_digi = triggered
        .iter()
        .any(|t| t.when.contains(&CompiledTiming::OnDigivolve));

    assert!(has_on_play, "must have an on_play clause (draw)");
    assert!(has_when_digi, "must have a when_digivolving clause (draw)");
    assert!(
        has_all_turns_play,
        "must have an on_enter_field_anyone observer for the All-Turns clause"
    );
    assert!(
        has_all_turns_digi,
        "must have an on_digivolve observer for the All-Turns clause"
    );
    assert!(
        !declarative.is_empty(),
        "must have at least one declarative clause (inherited Jamming)"
    );
}

/// The All-Turns observer clauses must be optional (printed: "may digivolve").
#[test]
fn ad1_010_all_turns_observer_is_optional() {
    let runner = DebugRunner::builder()
        .dsl_card("AD1-010")
        .expect("AD1-010 in embedded pack")
        .memory(0)
        .start();

    let compiled = runner.compiled_card("AD1-010").expect("compiled card");

    let observers: Vec<_> = compiled
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .filter(|t| {
            t.when.contains(&CompiledTiming::OnEnterFieldAnyone)
                || t.when.contains(&CompiledTiming::OnDigivolve)
        })
        .collect();

    assert!(
        !observers.is_empty(),
        "must have All-Turns observer clause(s)"
    );
    for obs in observers {
        assert!(
            obs.optional,
            "All-Turns observer must be optional (printed: 'may digivolve')"
        );
    }
}

/// The inherited Jamming clause must have inherited scope and the Jamming
/// keyword. (`CompiledDeclarativeClause::GrantKeyword` stores keyword as a
/// String — match on the string "Jamming".)
#[test]
fn ad1_010_inherited_jamming_clause_shape() {
    let runner = DebugRunner::builder()
        .dsl_card("AD1-010")
        .expect("AD1-010 in embedded pack")
        .memory(0)
        .start();

    let compiled = runner.compiled_card("AD1-010").expect("compiled card");

    let inherited_jamming = compiled.effects.iter().any(|c| match c {
        CompiledClause::Declarative(d) => match d {
            CompiledDeclarativeClause::GrantKeyword { keyword, scope, .. } => {
                *scope == CompiledScope::Inherited && keyword.eq_ignore_ascii_case("Jamming")
            }
            _ => false,
        },
        _ => false,
    });

    assert!(
        inherited_jamming,
        "must have an inherited grant_keyword Jamming clause"
    );
}

/// Alt-digivolve compiled state: must have at least one alt_path of kind
/// `Digivolve` with a Lv.3 source predicate.
#[test]
fn ad1_010_alt_digivolve_lv3_present() {
    let runner = DebugRunner::builder()
        .dsl_card("AD1-010")
        .expect("AD1-010 in embedded pack")
        .memory(0)
        .start();

    let compiled = runner.compiled_card("AD1-010").expect("compiled card");

    assert!(
        !compiled.alt_paths.is_empty(),
        "AD1-010 must declare at least one alt-digivolve path"
    );

    let any_lvl3 = compiled
        .alt_paths
        .iter()
        .filter(|p| matches!(p.kind, CompiledAltPathKind::Digivolve))
        .any(|p| {
            // Cost must be present.
            let cost_ok = p.cost.is_some();
            // The source predicate must require level 3.
            let level_ok = p
                .from
                .as_ref()
                .and_then(|f| f.level_eq)
                .map(|l| l == 3)
                .unwrap_or(false);
            cost_ok && level_ok
        });

    assert!(
        any_lvl3,
        "must have at least one Lv.3 alt-digivolve path with a cost (Omnimon-name and/or ADVENTURE-trait)"
    );
}

// ─── SECTION 2 — Behavioral: <Draw 1> on play ────────────────────────────────

/// On Play (player resolves the play action): the controller draws 1 card.
#[test]
fn ad1_010_on_play_draws_one_card() {
    let mut runner = DebugRunner::builder()
        .dsl_card("AD1-010")
        .expect("AD1-010 in embedded pack")
        .add_card(make_filler("FILL"))
        .hand(0, &["AD1-010"])
        .deck(0, &["FILL", "FILL", "FILL", "FILL", "FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();

    let hand_before = runner.game.players[0].hand.len();
    let deck_before = runner.game.players[0].deck.len();

    let _perm = runner.play(0, 0);
    runner.game.drain_effect_queue();
    drain_pending(&mut runner);

    // hand: -1 (played AD1-010) +1 (drew 1) = unchanged; deck: -1.
    assert_eq!(
        runner.game.players[0].hand.len(),
        hand_before,
        "hand size must be unchanged after play (-1 played, +1 drawn from <Draw 1>)"
    );
    assert_eq!(
        runner.game.players[0].deck.len(),
        deck_before - 1,
        "deck must shrink by 1 from <Draw 1>"
    );
}

/// When Digivolving: digivolving INTO AD1-010 from a Lv.3 source must fire
/// the shared <Draw 1>.
#[test]
fn ad1_010_when_digivolving_draws_one_card() {
    let mut runner = DebugRunner::builder()
        .dsl_card("AD1-010")
        .expect("AD1-010 in embedded pack")
        .add_card(make_adventure_lvl3("LVL3", "TestRookieEvolver"))
        .add_card(make_filler("FILL"))
        .hand(0, &["AD1-010"])
        .deck(0, &["FILL", "FILL", "FILL", "FILL", "FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();

    let base = runner.place_on_field(0, "LVL3", Some(0));
    let hand_before = runner.game.players[0].hand.len();
    let deck_before = runner.game.players[0].deck.len();

    let hand_idx = find_hand_index(&runner, 0, "AD1-010").expect("AD1-010 in hand");
    let ok = runner.game.digivolve_from_hand(
        0,
        hand_idx,
        base.index as usize,
        PlaySource::ByDigivolve,
    );
    assert!(ok, "digivolve_from_hand must succeed");
    runner.game.drain_effect_queue();
    drain_pending(&mut runner);

    // The engine's `digivolve_from_hand` always calls `draw()` (the rule that
    // every digivolve draws 1 card). The card's own [When Digivolving]
    // <Draw 1> draws an ADDITIONAL card. Hand: -1 (stacked) +2 (rule draw +
    // effect draw) = +1 net. Deck: -2.
    assert_eq!(
        runner.game.players[0].hand.len(),
        hand_before + 1,
        "hand must net +1 after digivolve (-1 stacked, +1 rule draw, +1 <Draw 1>)"
    );
    assert_eq!(
        runner.game.players[0].deck.len(),
        deck_before - 2,
        "deck must shrink by 2 (rule draw + <Draw 1> on When Digivolving)"
    );
}

// ─── SECTION 3 — All-Turns observer: condition gating ────────────────────────

/// Negative gate: when an irrelevant Digimon is played (no Greymon / Matt Ishida
/// in the name), the All-Turns observer must not install a digivolve prompt.
#[test]
fn ad1_010_all_turns_observer_does_not_fire_on_unrelated_play() {
    let mut runner = DebugRunner::builder()
        .dsl_card("AD1-010")
        .expect("AD1-010 in embedded pack")
        .add_card(make_named_digimon("UNREL", "RandomOther"))
        .add_card(make_filler("FILL"))
        .hand(0, &["UNREL"])
        .deck(0, &["FILL", "FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();

    runner.place_on_field(0, "AD1-010", Some(0));
    runner.game.drain_effect_queue();
    drain_pending(&mut runner);

    let _ = runner.play(0, 0);
    runner.game.drain_effect_queue();

    assert!(
        runner.game.pending_selection.is_none(),
        "no observer prompt should install when no [Greymon]/[Matt Ishida] is involved"
    );
}

/// Positive gate: when an ally Digimon containing "Greymon" in its name is
/// played, the All-Turns observer fires its optional digivolve prompt.
#[test]
fn ad1_010_all_turns_observer_fires_on_greymon_play() {
    let mut runner = DebugRunner::builder()
        .dsl_card("AD1-010")
        .expect("AD1-010 in embedded pack")
        .add_card(make_named_digimon("GREY", "Greymon"))
        .add_card(make_named_digimon("GARU", "Garurumon"))
        .add_card(make_filler("FILL"))
        .hand(0, &["GREY", "GARU"])
        .deck(0, &["FILL", "FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();

    runner.place_on_field(0, "AD1-010", Some(0));
    runner.game.drain_effect_queue();
    drain_pending(&mut runner);

    let grey_idx = find_hand_index(&runner, 0, "GREY").expect("GREY in hand");
    let _ = runner.play(0, grey_idx);
    runner.game.drain_effect_queue();

    assert!(
        runner.game.pending_selection.is_some(),
        "observer must install a pending selection when a [Greymon] is played"
    );
}

/// Positive gate (tamer play): when a tamer named "Matt Ishida" is played,
/// the observer should fire. IGNORED pending engine confirmation that
/// `event_card` is populated for tamer plays via TriggerSource::EnteredField.
#[test]
#[ignore = "pending: G-EVENT-CARD-TAMER-PLAY — event_card population for tamer plays via TriggerSource::EnteredField is unconfirmed in engine-gaps.md"]
fn ad1_010_all_turns_observer_fires_on_matt_ishida_tamer_play() {
    unimplemented!("blocked on G-EVENT-CARD-TAMER-PLAY")
}

// ─── SECTION 4 — All-Turns observer: digivolve trigger path ──────────────────

/// Positive gate: when an ally Digimon digivolves into a card containing
/// "Greymon", the on_digivolve observer fires. (Engine-gaps.md confirms
/// `event_card_name_contains` is wired for `Game::digivolve_from_hand`.)
#[test]
fn ad1_010_all_turns_observer_fires_on_greymon_digivolve() {
    let mut runner = DebugRunner::builder()
        .dsl_card("AD1-010")
        .expect("AD1-010 in embedded pack")
        .add_card(make_adventure_lvl3("LVL3", "RookieBase"))
        .add_card(make_lvl4_red_evo3("GREY4", "Greymon"))
        .add_card(make_named_digimon("GARU", "Garurumon"))
        .add_card(make_filler("FILL"))
        .hand(0, &["GREY4", "GARU"])
        .deck(0, &["FILL", "FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();

    runner.place_on_field(0, "AD1-010", Some(0));
    let base = runner.place_on_field(0, "LVL3", Some(0));
    runner.game.drain_effect_queue();
    drain_pending(&mut runner);

    let hand_idx = find_hand_index(&runner, 0, "GREY4").expect("GREY4 in hand");
    let ok = runner.game.digivolve_from_hand(
        0,
        hand_idx,
        base.index as usize,
        PlaySource::ByDigivolve,
    );
    assert!(ok, "digivolve_from_hand must succeed");
    runner.game.drain_effect_queue();

    assert!(
        runner.game.pending_selection.is_some(),
        "observer must install a pending selection when a [Greymon] is digivolved into"
    );
}

/// Negative gate (digivolve path): digivolving into an unrelated Lv.4 must
/// not install the observer prompt.
#[test]
fn ad1_010_all_turns_observer_does_not_fire_on_unrelated_digivolve() {
    let mut runner = DebugRunner::builder()
        .dsl_card("AD1-010")
        .expect("AD1-010 in embedded pack")
        .add_card(make_adventure_lvl3("LVL3", "RookieBase"))
        .add_card(make_lvl4_red_evo3("UNRELLV4", "Mostlyharmless"))
        .add_card(make_filler("FILL"))
        .hand(0, &["UNRELLV4"])
        .deck(0, &["FILL", "FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();

    runner.place_on_field(0, "AD1-010", Some(0));
    let base = runner.place_on_field(0, "LVL3", Some(0));
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
        runner.game.pending_selection.is_none(),
        "observer must NOT fire when an unrelated Lv.4 is digivolved into"
    );
}

// ─── SECTION 5 — Event log assertions ─────────────────────────────────────────

/// On Play emits a `Play` event for AD1-010. (The `GameEvent` enum currently
/// has no `Draw` variant; the closest stable signal is the `Play` event for
/// the played card itself. The hand/deck delta tested in Section 2 covers
/// the actual draw side-effect.)
#[test]
fn ad1_010_on_play_emits_play_event() {
    let mut runner = DebugRunner::builder()
        .dsl_card("AD1-010")
        .expect("AD1-010 in embedded pack")
        .add_card(make_filler("FILL"))
        .hand(0, &["AD1-010"])
        .deck(0, &["FILL", "FILL", "FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();

    let cp = runner.event_checkpoint();
    let _ = runner.play(0, 0);
    runner.game.drain_effect_queue();
    drain_pending(&mut runner);

    let play_event_count = runner
        .events_since(cp)
        .iter()
        .filter(|e| matches!(e, GameEvent::Play { player: 0, .. }))
        .count();

    assert!(
        play_event_count >= 1,
        "at least one Play event must be emitted for player 0"
    );
}

/// When Digivolving emits a `Digivolve` event for the digivolving stack.
#[test]
fn ad1_010_when_digivolving_emits_digivolve_event() {
    let mut runner = DebugRunner::builder()
        .dsl_card("AD1-010")
        .expect("AD1-010 in embedded pack")
        .add_card(make_adventure_lvl3("LVL3", "TestRookieEvolver"))
        .add_card(make_filler("FILL"))
        .hand(0, &["AD1-010"])
        .deck(0, &["FILL", "FILL", "FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();

    let base = runner.place_on_field(0, "LVL3", Some(0));
    let cp = runner.event_checkpoint();

    let hand_idx = find_hand_index(&runner, 0, "AD1-010").expect("AD1-010 in hand");
    let ok = runner.game.digivolve_from_hand(
        0,
        hand_idx,
        base.index as usize,
        PlaySource::ByDigivolve,
    );
    assert!(ok, "digivolve_from_hand must succeed");
    runner.game.drain_effect_queue();
    drain_pending(&mut runner);

    let digi_count = runner
        .events_since(cp)
        .iter()
        .filter(|e| matches!(e, GameEvent::Digivolve { player: 0, .. }))
        .count();

    assert!(
        digi_count >= 1,
        "at least one Digivolve event must be emitted for player 0"
    );
}

// ─── SECTION 6 — Inherited Jamming presence ───────────────────────────────────

/// Inherited <Jamming> is structurally present (compiled into a declarative
/// `GrantKeyword` clause with `scope: inherited`). Behavioral combat semantics
/// (no deletion in battles vs Security Digimon) are covered by the engine
/// keyword test suite (`tests/keyword_parsing.rs`, `tests/combat/...`).
#[test]
fn ad1_010_inherited_jamming_is_present_at_runtime() {
    let runner = DebugRunner::builder()
        .dsl_card("AD1-010")
        .expect("AD1-010 in embedded pack")
        .memory(0)
        .start();

    let compiled = runner.compiled_card("AD1-010").expect("compiled card");

    let has_inherited_jamming = compiled.effects.iter().any(|c| match c {
        CompiledClause::Declarative(d) => match d {
            CompiledDeclarativeClause::GrantKeyword { keyword, scope, .. } => {
                *scope == CompiledScope::Inherited && keyword.eq_ignore_ascii_case("Jamming")
            }
            _ => false,
        },
        _ => false,
    });

    assert!(
        has_inherited_jamming,
        "AD1-010 must declare an inherited <Jamming> keyword grant"
    );
}
