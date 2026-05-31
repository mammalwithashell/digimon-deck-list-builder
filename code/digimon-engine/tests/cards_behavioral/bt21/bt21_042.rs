//! BT21-042 GeoGreymon — Digimon, Lv.4, Yellow, DP 5000, Cost 5.
//! Traits: Dinosaur. Attribute: Vaccine.
//!
//! # Card text (cards.json — printed)
//!
//! [All Turns] [Once Per Turn] When any of your [Marcus Damon]s are played,
//! this Digimon may digivolve into a yellow Digimon card with [RizeGreymon]
//! in its name in the hand without paying the cost.
//!
//! Inherited Effect:
//! [Your Turn] This Digimon gets +2000 DP.
//!
//! Standard / alt digivolve: Lv.3 w/[Agumon] in name and [Dinosaur] trait /
//! Cost 2 (per cards.json `evo_costs` + `xros_req`).
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT21/Yellow/BT21_042.cs
//!
//! # Clause decomposition / DCGO mapping
//!
//! Clause 1 — Alt digivolution (DCGO EffectTiming.None,
//!   AddSelfDigivolutionRequirementStaticEffect): PermanentCondition =
//!   TopCard.ContainsCardName("Agumon") && EqualsTraits("Dinosaur") &&
//!   Level == 3; digivolutionCost: 2; ignoreDigivolutionRequirement: false.
//!   → `alt_paths: kind: digivolve, from: {level_eq: 3, name_contains:
//!   "Agumon", trait_has: Dinosaur}, cost: 2`.
//!
//! Clause 2 — [All Turns][Once Per Turn] cross-permanent observer (DCGO
//!   EffectTiming.OnEnterFieldAnyone, single ActivateClass, oncePerTurn: 1):
//!   CanUseCondition = IsExistOnBattleAreaDigimon(card) &&
//!     CanTriggerOnPermanentPlay(PlayedPermanentCondition).
//!   PlayedPermanentCondition = IsPermanentExistsOnOwnerBattleArea(permanent,
//!     card) && permanent.TopCard.EqualsCardName("Marcus Damon").
//!   Body: DigivolveIntoHandOrTrashCard(target=card.PermanentOfThisCard(),
//!     cardCondition = HasCardColor(Yellow) && ContainsCardName("RizeGreymon"),
//!     payCost: false, isHand: true). NOTE: Marcus Damon is a TAMER name —
//!     there is NO digivolve half (you cannot digivolve INTO Marcus Damon),
//!     so this is `on_enter_field_anyone` ONLY (unlike AD1-010/EX9-012 which
//!     also have an `on_digivolve` half).
//!   → triggered clause: `when: on_enter_field_anyone`,
//!     `active_when: { all_turns: true }`, `optional: true`,
//!     `once_per_turn: true`,
//!     condition all_of [event_target_owner: you,
//!       event_card_name_contains: "Marcus Damon"],
//!     process: select_hand (yellow + digimon + name_contains "RizeGreymon")
//!       → effect_initiated_digivolve { target: source, cost: 0,
//!       ignore_requirements: true }.
//!
//! Clause 3 — Inherited [Your Turn] +2000 DP self-aura (DCGO EffectTiming.None,
//!   ChangeSelfDPStaticEffect(changeValue: 2000, isInheritedEffect: true,
//!   condition: IsOwnerTurn)). → `scope: inherited / kind: aura /
//!   active_when: { your_turn: true } / target: {} / dp_modifier: 2000`
//!   (EX9-012 idiom).
//!
//! # Patterns this test covers (hybrid §11 + positive-rules checklist)
//! - B3: [All Turns] cross-permanent observer on ally Tamer play
//! - E2: OPT + optional "may" (with explicit lockout test + clear-after-end_turn)
//! - A5-adjacent: effect-initiated free digivolve from hand onto self
//! - D4: inherited [Your Turn] +2000 DP self-aura (on vs off your turn)
//!
//! # Known engine/DSL gaps affecting these tests
//!
//! - **G-EVENT-CARD-TAMER-PLAY** (per AD1-010 / EX9-012 docstrings): Tamer-play
//!   `event_card` parity through `TriggerSource::EnteredField` was historically
//!   unconfirmed. EX9-012's live (non-ignored) Tai-Kamiya tamer-play test
//!   passes, so the path is exercised; the Marcus-Damon observer here rides
//!   the same path. If the tamer-play event-card path regresses, the
//!   `bt21_042_observer_fires_on_own_marcus_damon_play` test will flag it.

#![allow(dead_code, unused_imports)]

use digimon_dsl::compiled::{
    CompiledAltPath, CompiledAltPathKind, CompiledClause, CompiledCost, CompiledDeclarativeClause,
    CompiledScope, CompiledTiming, CompiledTriggeredClause,
};
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming, Keyword, PlaySource, PlayerId};
use digimon_engine::events::GameEvent;
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
    c.play_cost = 0;
    c
}

/// A yellow [RizeGreymon]-named Lv.5 hand Digimon with an evo_cost matching
/// Lv.4 → Lv.5 so it can be free-digivolved onto BT21-042 (Lv.4) by the
/// observer. (`ignore_color: true` under `effect_initiated_digivolve` bypasses
/// color match; the level entry must align with the carrier's current level.)
fn make_rizegrey_lv4(id: &str) -> CardData {
    let mut c = make_digimon(id, "RizeGreymon", 5, 9000);
    c.colors = vec![CardColor::Yellow];
    c.evo_costs = vec![EvoCost {
        level: 4,
        memory_cost: 4,
        card_color: 2, // Yellow — not consulted under ignore_color
    }];
    c
}

/// A RED (non-yellow) [RizeGreymon]-named hand Digimon — must NOT satisfy the
/// `color_is: yellow` candidate filter.
fn make_red_rizegrey_lv4(id: &str) -> CardData {
    let mut c = make_digimon(id, "RizeGreymon", 5, 9000);
    c.colors = vec![CardColor::Red];
    c.evo_costs = vec![EvoCost {
        level: 4,
        memory_cost: 4,
        card_color: 0,
    }];
    c
}

/// Standard fixture: BT21-042 registered + filler. Memory pre-set high so
/// Tamer plays don't seesaw memory mid-test.
fn geogrey_runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("BT21-042")
        .expect("BT21-042 found in embedded DSL pack")
        .add_card(make_filler("FILL"))
        .memory(12)
        .start()
}

/// Like `geogrey_runner` but stocks both players' decks with filler so the
/// start-of-turn draw doesn't deck a player out across a full turn rotation
/// (needed by the OPT-clear half of the lockout test).
fn geogrey_runner_with_decks() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("BT21-042")
        .expect("BT21-042 found in embedded DSL pack")
        .add_card(make_filler("FILL"))
        .deck(0, &["FILL", "FILL", "FILL", "FILL", "FILL"])
        .deck(1, &["FILL", "FILL", "FILL", "FILL", "FILL"])
        .memory(12)
        .start()
}

/// Build a runner with BT21-042 registered and stacked under a Lv.5 dummy
/// Digimon on player 0's battle area. Used by inherited DP tests.
fn runner_with_bt21_042_as_source() -> (DebugRunner, PermanentHandle) {
    let bt21_data = card_data_from_compiled("BT21-042");
    let mut carrier = make_test_card("CARRIER-LV5", "CarrierLv5");
    carrier.level = Some(5);
    carrier.dp = Some(9000);

    let mut runner = DebugRunner::builder()
        .add_card(bt21_data)
        .add_card(carrier)
        .memory(10)
        .start();

    let bt21_handle = runner.place_on_field(0, "BT21-042", Some(0));
    let carrier_data_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == "CARRIER-LV5")
        .expect("CARRIER-LV5 in card_data");
    let next_idx = runner.game.next_card_index();
    let carrier_source = CardSource::new(carrier_data_idx, 0, next_idx);
    let turn = runner.game.turn_count;
    runner.game.players[0].battle_area[bt21_handle.index as usize].digivolve(carrier_source, turn);

    (runner, bt21_handle)
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
fn bt21_042_compiles_in_embedded_pack() {
    let runner = geogrey_runner();
    assert!(
        runner.compiled_card("BT21-042").is_some(),
        "BT21-042 must be present in embedded DSL pack"
    );
}

#[test]
fn bt21_042_card_metadata_matches_print() {
    let runner = geogrey_runner();
    let card = runner.compiled_card("BT21-042").expect("BT21-042 compiles");

    assert_eq!(card.level, Some(4), "GeoGreymon is Lv.4");
    assert_eq!(card.dp, Some(5000), "BT21-042 is DP 5000");
    assert_eq!(card.cost, Some(5), "BT21-042 costs 5 to play");
}

/// Alt-digivolve: Lv.3 / [Agumon]-name / [Dinosaur]-trait / Cost 2.
#[test]
fn bt21_042_agumon_alt_digivolve_path_present() {
    let runner = geogrey_runner();
    let card = runner.compiled_card("BT21-042").expect("BT21-042 compiles");

    let std_path = card
        .alt_paths
        .iter()
        .find(|p| {
            p.kind == CompiledAltPathKind::Digivolve && p.cost == Some(CompiledCost::Literal(2))
        })
        .expect("BT21-042 must declare a Lv.3 Agumon/Dinosaur digivolve path with cost 2");

    let from = std_path
        .from
        .as_ref()
        .expect("alt digivolve must declare a `from:` predicate");
    assert_eq!(
        from.level_eq,
        Some(3),
        "alt digivolve must require level_eq: 3"
    );
    assert_eq!(
        from.name_contains.as_deref(),
        Some("Agumon"),
        "alt digivolve must require [Agumon] in name"
    );
    assert!(
        from.trait_has
            .as_deref()
            .map(|t| t.eq_ignore_ascii_case("Dinosaur"))
            .unwrap_or(false),
        "alt digivolve must require the [Dinosaur] trait; got {:?}",
        from.trait_has
    );
}

/// The [All Turns][Once Per Turn] observer must:
/// - fire on `on_enter_field_anyone` (Tamer play half) ONLY (no on_digivolve —
///   Marcus Damon is a Tamer name and cannot be digivolved into),
/// - be optional ("may digivolve"),
/// - be once-per-turn ([Once Per Turn]),
/// - be face-up scope,
/// - declare active_when (all_turns: true).
#[test]
fn bt21_042_has_all_turns_opt_observer_clause() {
    let runner = geogrey_runner();
    let card = runner.compiled_card("BT21-042").expect("BT21-042 compiles");

    let triggered: Vec<_> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();

    let observer = triggered
        .iter()
        .find(|t| t.when.contains(&CompiledTiming::OnEnterFieldAnyone))
        .expect("BT21-042 must have an on_enter_field_anyone observer clause");

    assert_eq!(
        observer.scope,
        CompiledScope::FaceUp,
        "observer must be face-up scope"
    );
    assert!(
        observer.optional,
        "observer is optional (\"may digivolve\")"
    );
    assert!(
        observer.once_per_turn,
        "observer is printed [Once Per Turn]"
    );
    assert!(
        observer.active_when.is_some(),
        "observer must declare active_when (all_turns: true)"
    );
    // Marcus Damon is a Tamer name — no digivolve half.
    assert!(
        !triggered
            .iter()
            .any(|t| t.when.contains(&CompiledTiming::OnDigivolve)),
        "BT21-042 must NOT have an on_digivolve half (Marcus Damon is a Tamer name)"
    );
}

/// The inherited [Your Turn] +2000 DP self-aura must be present with the
/// right scope, dp_modifier, and active_when.
#[test]
fn bt21_042_has_inherited_your_turn_dp_aura() {
    let runner = geogrey_runner();
    let card = runner.compiled_card("BT21-042").expect("BT21-042 compiles");

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
        aura.expect("BT21-042 must have a Declarative::Aura clause (inherited +2000 DP)");
    assert_eq!(
        scope,
        CompiledScope::Inherited,
        "the aura clause must have Inherited scope"
    );
    assert_eq!(dp, Some(2000), "the aura dp_modifier must be +2000 DP");
    assert!(
        active_when.is_some(),
        "the aura must declare active_when (your_turn: true)"
    );
}

// ─── SECTION 2 — Behavioral: observer trigger gating (play half) ─────────────

/// Positive gate: when an own Tamer named "Marcus Damon" is played on the
/// controller's turn AND a yellow [RizeGreymon]-named hand card is available,
/// the observer fires its optional digivolve prompt.
#[test]
fn bt21_042_observer_fires_on_own_marcus_damon_play() {
    let mut runner = geogrey_runner();
    runner.game.card_data.push(make_rizegrey_lv4("RIZE"));
    runner
        .game
        .card_data
        .push(make_named_tamer("MARCUS", "Marcus Damon"));
    push_to_hand(&mut runner, 0, "RIZE");
    push_to_hand(&mut runner, 0, "MARCUS");
    let _bt21 = runner.place_on_field(0, "BT21-042", Some(0));
    runner.game.drain_effect_queue();
    drain_pending(&mut runner);

    let hand_idx = find_hand_index(&runner, 0, "MARCUS").expect("MARCUS in hand");
    runner.play(0, hand_idx).expect("Marcus Damon tamer plays");
    runner.game.drain_effect_queue();

    let pending = runner
        .pending_selection()
        .expect("observer must install a prompt when own [Marcus Damon] plays");
    assert!(
        pending.is_optional,
        "observer prompt is optional (\"may digivolve\")"
    );
}

/// Negative gate (name filter): an own UNRELATED Tamer play must NOT fire the
/// observer (name filter requires "Marcus Damon").
#[test]
fn bt21_042_observer_does_not_fire_on_unrelated_own_tamer_play() {
    let mut runner = geogrey_runner();
    runner.game.card_data.push(make_rizegrey_lv4("RIZE"));
    runner
        .game
        .card_data
        .push(make_named_tamer("OTHER-TAMER", "Some Other Tamer"));
    push_to_hand(&mut runner, 0, "RIZE");
    push_to_hand(&mut runner, 0, "OTHER-TAMER");
    let _bt21 = runner.place_on_field(0, "BT21-042", Some(0));
    runner.game.drain_effect_queue();
    drain_pending(&mut runner);

    let hand_idx = find_hand_index(&runner, 0, "OTHER-TAMER").expect("OTHER-TAMER in hand");
    runner.play(0, hand_idx).expect("unrelated tamer plays");
    runner.game.drain_effect_queue();

    assert!(
        runner.pending_selection().is_none(),
        "unrelated-name own Tamer play must not fire the observer"
    );
}

/// Negative gate (event ownership): the observer must NOT fire on an
/// OPPONENT's Marcus Damon play. Card text scopes to "your [Marcus Damon]s".
#[test]
fn bt21_042_observer_does_not_fire_on_opponent_marcus_damon_play() {
    let mut runner = geogrey_runner();
    runner.game.card_data.push(make_rizegrey_lv4("RIZE"));
    runner
        .game
        .card_data
        .push(make_named_tamer("OPP-MARCUS", "Marcus Damon"));
    push_to_hand(&mut runner, 0, "RIZE");
    push_to_hand(&mut runner, 1, "OPP-MARCUS");
    let _bt21 = runner.place_on_field(0, "BT21-042", Some(0));
    runner.game.drain_effect_queue();
    drain_pending(&mut runner);

    runner.end_turn();
    assert_eq!(runner.turn_player(), 1, "precondition: now P1's turn");

    let hand_idx = find_hand_index(&runner, 1, "OPP-MARCUS").expect("OPP-MARCUS in hand");
    runner
        .play(1, hand_idx)
        .expect("opponent's Marcus Damon plays");
    runner.game.drain_effect_queue();

    assert!(
        runner.pending_selection().is_none(),
        "opponent's Marcus Damon play must not fire the observer"
    );
}

/// [All Turns] positive: the observer fires even on the OPPONENT'S turn, as
/// long as it's the controller's OWN Marcus Damon entering their battle area.
/// (Distinguishes [All Turns] from [Your Turn] — EX9-012's sister observer is
/// [Your Turn] and is SUPPRESSED off-turn; this one is NOT.)
///
/// `place_on_field` does NOT fire the play-trigger bundle (it is post-play
/// setup state only, per positive-rule 12), so we place an own Marcus Damon
/// onto P0's field while it is P1's turn, then drive the real play-event
/// trigger bundle via `fire_play_event_triggers` (which sets
/// `TriggerSource::EnteredField` and populates `event_card`). The
/// `active_when: { all_turns: true }` gate must NOT suppress.
#[test]
fn bt21_042_observer_fires_on_opponents_turn_all_turns() {
    let mut runner = geogrey_runner();
    runner.game.card_data.push(make_rizegrey_lv4("RIZE"));
    push_to_hand(&mut runner, 0, "RIZE");
    let _bt21 = runner.place_on_field(0, "BT21-042", Some(0));
    runner.game.drain_effect_queue();
    drain_pending(&mut runner);

    // Switch to the opponent's turn.
    runner.game.set_memory(3);
    runner.end_turn();
    assert_eq!(runner.turn_player(), 1, "precondition: now P1's turn");

    // Place an own Marcus Damon, then fire the real play-event trigger bundle.
    runner
        .game
        .card_data
        .push(make_named_tamer("MARCUS", "Marcus Damon"));
    let marcus = runner.place_on_field(0, "MARCUS", Some(0));
    runner.fire_play_event_triggers(0, marcus.index as usize, false, false);
    runner.game.drain_effect_queue();

    assert!(
        runner.pending_selection().is_some(),
        "[All Turns] observer must fire on opponent's turn for own Marcus Damon play"
    );
}

// ─── SECTION 3 — Behavioral: hand-pick candidate filter ──────────────────────

/// Negative gate (no eligible hand card): when NO yellow [RizeGreymon]-named
/// Digimon is in hand, playing Marcus Damon must NOT install a prompt
/// (the observer's CanActivateCondition fails — HasMatchConditionOwnersHand).
#[test]
fn bt21_042_observer_no_prompt_without_eligible_hand_card() {
    let mut runner = geogrey_runner();
    runner
        .game
        .card_data
        .push(make_named_tamer("MARCUS", "Marcus Damon"));
    push_to_hand(&mut runner, 0, "MARCUS");
    let _bt21 = runner.place_on_field(0, "BT21-042", Some(0));
    runner.game.drain_effect_queue();
    drain_pending(&mut runner);

    let hand_idx = find_hand_index(&runner, 0, "MARCUS").expect("MARCUS in hand");
    runner.play(0, hand_idx).expect("Marcus Damon plays");
    runner.game.drain_effect_queue();

    assert!(
        runner.pending_selection().is_none(),
        "no yellow [RizeGreymon] in hand → no observer prompt"
    );
}

/// Negative gate (color filter): a RED [RizeGreymon] in hand must NOT satisfy
/// the `color_is: yellow` candidate filter → no prompt.
#[test]
fn bt21_042_observer_excludes_non_yellow_rizegreymon() {
    let mut runner = geogrey_runner();
    runner.game.card_data.push(make_red_rizegrey_lv4("RED-RIZE"));
    runner
        .game
        .card_data
        .push(make_named_tamer("MARCUS", "Marcus Damon"));
    push_to_hand(&mut runner, 0, "RED-RIZE");
    push_to_hand(&mut runner, 0, "MARCUS");
    let _bt21 = runner.place_on_field(0, "BT21-042", Some(0));
    runner.game.drain_effect_queue();
    drain_pending(&mut runner);

    let hand_idx = find_hand_index(&runner, 0, "MARCUS").expect("MARCUS in hand");
    runner.play(0, hand_idx).expect("Marcus Damon plays");
    runner.game.drain_effect_queue();

    assert!(
        runner.pending_selection().is_none(),
        "a non-yellow [RizeGreymon] must NOT satisfy color_is: yellow → no prompt"
    );
}

// ─── SECTION 4 — Behavioral: observer end-to-end (free digivolve) ────────────

/// Accept the observer's optional prompt and select the yellow RizeGreymon hand
/// card. BT21-042's slot must end with a RizeGreymon-named top card; the
/// digivolve must NOT pay memory (cost: 0). Hand decreases by 2 (Marcus played
/// + RizeGreymon free-digivolved onto BT21-042).
#[test]
fn bt21_042_observer_accept_free_digivolves_self_into_rizegreymon() {
    let mut runner = geogrey_runner();
    runner.game.card_data.push(make_rizegrey_lv4("RIZE"));
    runner
        .game
        .card_data
        .push(make_named_tamer("MARCUS", "Marcus Damon"));
    push_to_hand(&mut runner, 0, "RIZE");
    push_to_hand(&mut runner, 0, "MARCUS");
    let bt21 = runner.place_on_field(0, "BT21-042", Some(0));
    runner.game.drain_effect_queue();
    drain_pending(&mut runner);

    let hand_before = runner.hand_size(0);

    let hand_idx = find_hand_index(&runner, 0, "MARCUS").expect("MARCUS in hand");
    runner.play(0, hand_idx).expect("Marcus Damon plays");
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

    // After resolution, BT21-042's slot top card must contain "RizeGreymon".
    let perm = &runner.game.players[0].battle_area[bt21.index as usize];
    let top_name = perm.top_card().card_name(&runner.game.card_data);
    assert!(
        top_name.contains("RizeGreymon"),
        "after free-digivolve, BT21-042 slot top card must contain \"RizeGreymon\"; got {:?}",
        top_name
    );
    // No memory payment from cost-0 digivolve.
    assert!(
        runner.memory() >= memory_after_play,
        "free-digivolve must not pay memory; memory_after_play={}, memory_after_observer={}",
        memory_after_play,
        runner.memory()
    );
    // Hand: -2 (Marcus Damon played + RizeGreymon free-digivolved on top).
    assert_eq!(
        runner.hand_size(0),
        hand_before - 2,
        "hand decreased by 2: Marcus Damon played, RizeGreymon free-digivolved"
    );
}

/// Decline path: passing the observer leaves BT21-042's slot untransformed.
#[test]
fn bt21_042_observer_decline_leaves_self_untouched() {
    let mut runner = geogrey_runner();
    runner.game.card_data.push(make_rizegrey_lv4("RIZE"));
    runner
        .game
        .card_data
        .push(make_named_tamer("MARCUS", "Marcus Damon"));
    push_to_hand(&mut runner, 0, "RIZE");
    push_to_hand(&mut runner, 0, "MARCUS");
    let bt21 = runner.place_on_field(0, "BT21-042", Some(0));
    runner.game.drain_effect_queue();
    drain_pending(&mut runner);

    let hand_idx = find_hand_index(&runner, 0, "MARCUS").expect("MARCUS in hand");
    runner.play(0, hand_idx).expect("Marcus Damon plays");
    runner.game.drain_effect_queue();

    if let Some(s) = runner.pending_selection() {
        assert!(s.is_optional, "observer prompt must be optional");
        let player = s.selecting_player;
        runner
            .execute_action(player, digimon_engine::action::space::PASS)
            .expect("PASS to decline");
    }
    runner.auto_resolve().ok();

    let perm = &runner.game.players[0].battle_area[bt21.index as usize];
    assert_eq!(
        perm.top_card().card_id(&runner.game.card_data),
        "BT21-042",
        "after decline BT21-042 slot top card must remain BT21-042"
    );
}

/// Event-log: the [Marcus Damon] play that fires this observer must emit a
/// `Play` event for player 0 (cost-firing-clause evidence via
/// `events_since(checkpoint)`, per §6 of the positive-rules checklist). The
/// Marcus-Damon `Play` event IS the trigger this clause observes.
///
/// NOTE: We deliberately assert on the `Play` event (the trigger), NOT a
/// `Digivolve` event for the free-digivolve. Effect-initiated digivolve fires
/// `WhenDigivolving` + `OnDigivolve` triggers but does NOT push a
/// `GameEvent::Digivolve` (it stacks the card directly via `.digivolve(...)`
/// without emitting the recorder event — see
/// `Game::effect_initiated_digivolve_from_source_inner` in game_actions.rs).
/// That is the open G-GAME-EVENT-DIGIVOLVE gap for effect-initiated paths; the
/// transformed-state assertion lives in
/// `bt21_042_observer_accept_free_digivolves_self_into_rizegreymon`.
#[test]
fn bt21_042_observer_marcus_play_emits_play_event() {
    let mut runner = geogrey_runner();
    runner.game.card_data.push(make_rizegrey_lv4("RIZE"));
    runner
        .game
        .card_data
        .push(make_named_tamer("MARCUS", "Marcus Damon"));
    push_to_hand(&mut runner, 0, "RIZE");
    push_to_hand(&mut runner, 0, "MARCUS");
    let _bt21 = runner.place_on_field(0, "BT21-042", Some(0));
    runner.game.drain_effect_queue();
    drain_pending(&mut runner);

    let cp = runner.event_checkpoint();

    let hand_idx = find_hand_index(&runner, 0, "MARCUS").expect("MARCUS in hand");
    runner.play(0, hand_idx).expect("Marcus Damon plays");
    runner.game.drain_effect_queue();
    // Affirmative branch through the observer prompts (the digivolve resolves).
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

    let marcus_play_events = runner
        .events_since(cp)
        .iter()
        .filter(|e| matches!(e, GameEvent::Play { player: 0, card_id, .. } if card_id == "MARCUS"))
        .count();
    assert!(
        marcus_play_events >= 1,
        "the [Marcus Damon] play (the observer's trigger) must emit a Play event for player 0"
    );
}

// ─── SECTION 5 — Behavioral: [Once Per Turn] lockout ─────────────────────────

/// OPT lockout: the observer fires at most once per turn. A SECOND own Marcus
/// Damon play in the same turn must NOT install a second prompt. After
/// end_turn the gate clears and a fresh play fires again.
#[test]
fn bt21_042_observer_once_per_turn_lockout_and_clear() {
    let mut runner = geogrey_runner_with_decks();
    runner.game.card_data.push(make_rizegrey_lv4("RIZE-A"));
    runner.game.card_data.push(make_rizegrey_lv4("RIZE-B"));
    runner
        .game
        .card_data
        .push(make_named_tamer("MARCUS-A", "Marcus Damon"));
    runner
        .game
        .card_data
        .push(make_named_tamer("MARCUS-B", "Marcus Damon"));
    push_to_hand(&mut runner, 0, "RIZE-A");
    push_to_hand(&mut runner, 0, "RIZE-B");
    push_to_hand(&mut runner, 0, "MARCUS-A");
    push_to_hand(&mut runner, 0, "MARCUS-B");
    let _bt21 = runner.place_on_field(0, "BT21-042", Some(0));
    runner.game.drain_effect_queue();
    drain_pending(&mut runner);

    // First Marcus Damon play → observer fires; DECLINE so BT21-042 stays a
    // legal carrier for the second test (and so we isolate the OPT gate, not
    // the transformed-state).
    let hand_idx = find_hand_index(&runner, 0, "MARCUS-A").expect("MARCUS-A in hand");
    runner.play(0, hand_idx).expect("first Marcus Damon plays");
    runner.game.drain_effect_queue();
    {
        let s = runner
            .pending_selection()
            .expect("first Marcus Damon play must install the observer prompt");
        let player = s.selecting_player;
        runner
            .execute_action(player, digimon_engine::action::space::PASS)
            .expect("decline first activation");
    }
    runner.auto_resolve().ok();

    // Second Marcus Damon play SAME turn → [Once Per Turn] suppresses.
    let hand_idx = find_hand_index(&runner, 0, "MARCUS-B").expect("MARCUS-B in hand");
    runner.play(0, hand_idx).expect("second Marcus Damon plays");
    runner.game.drain_effect_queue();
    assert!(
        runner.pending_selection().is_none(),
        "[Once Per Turn] must suppress a second observer activation in the same turn"
    );

    // After a full turn cycle the OPT gate clears. Set positive memory before
    // each turn-end and re-enter Main so the memory-swing-back guard in
    // `end_turn` does not stall the rotation (bt13_088 idiom).
    runner.game.set_memory(3);
    runner.end_turn(); // P0 ends → P1's turn
    drain_pending(&mut runner);
    assert_eq!(runner.turn_player(), 1, "rotated to opponent's turn");
    runner.game.enter_main_phase();
    runner.game.set_memory(3);
    runner.end_turn(); // P1 ends → back to P0
    drain_pending(&mut runner);
    assert_eq!(runner.turn_player(), 0, "back to P0's turn");

    // Fresh Marcus Damon play → observer fires again.
    runner
        .game
        .card_data
        .push(make_named_tamer("MARCUS-C", "Marcus Damon"));
    push_to_hand(&mut runner, 0, "MARCUS-C");
    let hand_idx = find_hand_index(&runner, 0, "MARCUS-C").expect("MARCUS-C in hand");
    runner.play(0, hand_idx).expect("third Marcus Damon plays");
    runner.game.drain_effect_queue();
    assert!(
        runner.pending_selection().is_some(),
        "[Once Per Turn] gate must clear after the turn ends — observer fires again"
    );
}

// ─── SECTION 6 — Inherited [Your Turn] +2000 DP ──────────────────────────────

/// When BT21-042 is a digivolution source under a carrier on the controller's
/// own turn, `source_dp_contribution` must return +2000 for its slot.
#[test]
fn bt21_042_inherited_dp_active_on_your_turn() {
    let (runner, carrier_handle) = runner_with_bt21_042_as_source();
    let bt21_src_idx = 0;

    assert_eq!(runner.turn_player(), 0, "precondition: it is P0's turn");

    let contribution = runner
        .game
        .source_dp_contribution(carrier_handle, bt21_src_idx);

    assert_eq!(
        contribution, 2000,
        "BT21-042's inherited +2000 DP must contribute on the controller's own turn; got {}",
        contribution
    );
}

/// On the opponent's turn, the [Your Turn] gate must suppress the DP buff.
#[test]
fn bt21_042_inherited_dp_inactive_on_opponents_turn() {
    let (mut runner, carrier_handle) = runner_with_bt21_042_as_source();
    runner.end_turn();
    assert_eq!(runner.turn_player(), 1, "precondition: it is now P1's turn");

    let bt21_src_idx = 0;
    let contribution = runner
        .game
        .source_dp_contribution(carrier_handle, bt21_src_idx);

    assert_eq!(
        contribution, 0,
        "BT21-042's inherited +2000 DP must NOT contribute on opponent's turn; got {}",
        contribution
    );
}
