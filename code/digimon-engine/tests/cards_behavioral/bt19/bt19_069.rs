//! BT19-069 Deltamon — Digimon, Lv.4, Purple+Black, DP 5000, Cost 5.
//! Traits: Composite. Attribute: Virus.
//! Evo: Purple Lv.3 / cost 3, Black Lv.3 / cost 3 (printed evo_costs block).
//! Alt-digivolve: [Gazimon]/[Gizamon] / cost 2 (printed digivolve box).
//!
//! # Card text (data/card_bundles/BT19-069.md — official Bandai DB)
//!
//! [On Play] [When Digivolving] [On Deletion] By trashing 1 card in your
//! hand, delete 1 of your opponent's level 4 or lower Digimon.
//!
//! Inherited Effect: <Blocker> (This Digimon can block in the blocker
//! timing.)
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT19/Purple/BT19_069.cs
//!   - EffectTiming.None: AddSelfDigivolutionRequirementStaticEffect
//!     (permanentCondition: top card name == "Gazimon" || "Gizamon",
//!     digivolutionCost: 2).
//!   - Shared body (OnEnterFieldAnyone for OnPlay/WhenDigivolving,
//!     OnDestroyedAnyone for OnDeletion): CanActivateConditionShared gates on
//!     `card.Owner.HandCards.Count >= 1` (an activation-eligibility check,
//!     NOT itself a forced trash). SelectHandEffect with `canNoSelect: true`
//!     (Mode.Discard) — the "by trashing" cost is fully OPTIONAL/declinable.
//!     `AfterSelectCardCoroutine` sets `discarded = cardSources.Count > 0`.
//!     Only when `discarded == true` AND
//!     `CardEffectCommons.HasMatchConditionPermanent(CanSelectOpponentPermanentConditionShared)`
//!     (an opponent Digimon with `Level <= 4` exists) does a SECOND,
//!     MANDATORY (`canNoSelect: false`) SelectPermanentEffect (Destroy) run
//!     over opponent Digimon with `permanent.IsDigimon && permanent.TopCard.HasLevel
//!     && permanent.Level <= 4`. Declining the trash, or having no eligible
//!     opponent target, skips the delete entirely — no partial payment.
//!   - Blocker - ESS: BlockerSelfStaticEffect(isInheritedEffect: true).
//!   - All three timings (OnPlay / WhenDigivolving / OnDeletion) run the
//!     IDENTICAL shared body — one `ActivateCoroutine` closure reused across
//!     three `cardEffects.Add(activateClass)` registrations.
//!
//! # Patterns this test covers (RUST_DSL_TEST_API.md §4.3)
//! - Tri-timing shared body: on_play + when_digivolving + on_deletion in one
//!   clause (BT24-041-style `when: [on_play, when_digivolving, on_deletion]`)
//! - E2-adjacent: optional cost-as-trashing (select_hand optional) gating a
//!   mandatory conditional delete (BT25-017 / BT12-016 top-level-select idiom)
//! - Negative: no eligible opponent target after trashing → no delete prompt
//! - Negative: empty hand → cost prompt still installs (optional, no
//!   candidates) and the whole clause is a no-op
//! - Cost-firing event-log test (GameEvent::Trash on the hand-trash cost)
//! - Alt-digivolution: [Gazimon]/[Gizamon] name-gated / cost 2
//! - H5 inherited Blocker (native keyword parse, not scripted)

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledCost, CompiledScope, CompiledTiming,
};
use digimon_engine::action::space::{PASS, PLAY_HAND_START};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card_with_level, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, EffectTiming, Keyword};
use digimon_engine::events::GameEvent;
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::replacement::ReplacementCause;
use digimon_engine::selection::{SelectionKind, TriggerSource};

const CARD_ID: &str = "BT19-069";

// ─── Fixture helpers ─────────────────────────────────────────────────────────

/// Opponent Digimon at a given level (DP is irrelevant to this card's gate).
fn opp_digimon(card_id: &str, level: u8) -> CardData {
    let mut card = make_test_card_with_level(card_id, card_id, level);
    card.colors = vec![CardColor::Red];
    card.dp = Some(3000);
    card
}

fn base() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT19-069 YAML parses and compiles")
        .add_card(opp_digimon("OPP-LV4", 4))
        .add_card(opp_digimon("OPP-LV3", 3))
        .add_card(opp_digimon("OPP-LV5", 5))
        .add_card(make_test_card_with_level("FILL", "Filler", 3))
        .add_card(make_test_card_with_level("FODDER", "Fodder", 3))
        .add_card(make_test_card_with_level("FODDER2", "Fodder2", 3))
        .deck(0, &["FILL"; 8])
        .deck(1, &["FILL"; 8])
        .memory(10)
}

fn place_deltamon(runner: &mut DebugRunner) -> PermanentHandle {
    runner.place_on_field(0, CARD_ID, Some(0))
}

fn fire_on_play(runner: &mut DebugRunner, source: PermanentHandle) {
    runner
        .game
        .enqueue_triggered(EffectTiming::OnPlay, TriggerSource::Permanent(source));
    runner.game.drain_effect_queue();
}

fn fire_when_digivolving(runner: &mut DebugRunner, source: PermanentHandle) {
    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(source),
    );
    runner.game.drain_effect_queue();
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 1 — Structural assertions
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn bt19_069_yaml_has_printed_metadata() {
    let runner = base().start();
    let card = runner
        .compiled_card(CARD_ID)
        .expect("BT19-069 must be in the embedded DSL pack");

    assert_eq!(card.name, "Deltamon");
    assert_eq!(card.level, Some(4));
    assert_eq!(card.dp, Some(5000));
    assert!(
        card.traits.contains(&"Composite".to_string()),
        "BT19-069 metadata must include trait Composite"
    );
}

#[test]
fn bt19_069_has_gazimon_gizamon_alt_digivolution_path_cost_2() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled present");

    let has_alt = card.alt_paths.iter().any(|p| {
        p.kind == CompiledAltPathKind::Digivolve
            && p.cost == Some(CompiledCost::Literal(2))
            && p.from.as_ref().is_some_and(|f| {
                f.name_in
                    .as_ref()
                    .is_some_and(|names| names.contains(&"Gazimon".to_string()))
                    && f.name_in
                        .as_ref()
                        .is_some_and(|names| names.contains(&"Gizamon".to_string()))
            })
    });
    assert!(
        has_alt,
        "BT19-069 must have a [Gazimon]/[Gizamon] -> cost 2 alt-path; alt_paths = {:?}",
        card.alt_paths
    );
}

#[test]
fn bt19_069_has_single_tri_timing_shared_clause() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled present");

    let triggered: Vec<_> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();

    let shared = triggered.iter().filter(|t| {
        t.when.contains(&CompiledTiming::OnPlay)
            && t.when.contains(&CompiledTiming::WhenDigivolving)
            && t.when.contains(&CompiledTiming::OnDeletion)
    });
    assert_eq!(
        shared.count(),
        1,
        "BT19-069 must have exactly one clause covering On Play + When Digivolving + On Deletion"
    );
}

#[test]
fn bt19_069_shared_clause_is_self_scoped_not_inherited() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled present");

    let clause = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::OnPlay) => Some(t),
            _ => None,
        })
        .expect("On Play clause must exist");
    assert_ne!(
        clause.scope,
        CompiledScope::Inherited,
        "the printed effect is BT19-069's own card text (not digivolution-source inherited)"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 2 — [On Play] positive/negative gating + behavior
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn bt19_069_on_play_trashing_hand_deletes_eligible_opponent_lv4() {
    let mut runner = base().start();
    runner.game.turn_count = 1;

    let opp = runner.place_on_field(1, "OPP-LV4", Some(0)); // level 4 — eligible
    let _fodder = push_to_hand(&mut runner, 0, "FODDER");
    let hand_index = push_to_hand(&mut runner, 0, CARD_ID);

    let opp_count_before = runner.battle_area_size(1);
    runner.play(0, hand_index).expect("play BT19-069");
    runner.auto_resolve().expect("resolve on-play flow");

    assert_eq!(
        runner.battle_area_size(1),
        opp_count_before - 1,
        "trashing a hand card must delete the level-4 opponent Digimon"
    );
}

#[test]
fn bt19_069_on_play_trashing_hand_deletes_eligible_opponent_lv3() {
    let mut runner = base().start();
    runner.game.turn_count = 1;

    let opp = runner.place_on_field(1, "OPP-LV3", Some(0)); // level 3 — eligible (<=4)
    let _fodder = push_to_hand(&mut runner, 0, "FODDER");
    let hand_index = push_to_hand(&mut runner, 0, CARD_ID);

    let opp_count_before = runner.battle_area_size(1);
    runner.play(0, hand_index).expect("play BT19-069");
    runner.auto_resolve().expect("resolve on-play flow");

    assert_eq!(
        runner.battle_area_size(1),
        opp_count_before - 1,
        "trashing a hand card must delete the level-3 opponent Digimon"
    );
}

#[test]
fn bt19_069_on_play_cannot_delete_opponent_above_level_4() {
    let mut runner = base().start();
    runner.game.turn_count = 1;

    let opp = runner.place_on_field(1, "OPP-LV5", Some(0)); // level 5 — NOT eligible
    let _fodder = push_to_hand(&mut runner, 0, "FODDER");
    let hand_index = push_to_hand(&mut runner, 0, CARD_ID);

    let opp_count_before = runner.battle_area_size(1);
    runner.play(0, hand_index).expect("play BT19-069");
    runner.auto_resolve().expect("resolve on-play flow");

    assert_eq!(
        runner.battle_area_size(1),
        opp_count_before,
        "an opponent Digimon above level 4 must not be a legal delete target"
    );
}

#[test]
fn bt19_069_on_play_no_eligible_target_skips_delete_prompt_after_trashing() {
    let mut runner = base().start();
    runner.game.turn_count = 1;
    // No opponent Digimon placed at all.
    let _fodder = push_to_hand(&mut runner, 0, "FODDER");
    let hand_index = push_to_hand(&mut runner, 0, CARD_ID);

    runner.play(0, hand_index).expect("play BT19-069");

    // Drive the optional hand-trash cost (accept it).
    let hand_prompt = runner
        .pending_selection_view()
        .expect("optional hand-trash cost prompt must install even with no opponent target");
    assert!(
        hand_prompt.is_optional,
        "the trash cost is a 'by trashing' optional cost"
    );
    runner
        .execute_action(0, PLAY_HAND_START)
        .expect("trash the fodder hand card");

    assert!(
        runner.pending_selection().is_none(),
        "with no eligible opponent Digimon, no delete prompt should install after trashing"
    );
    let _ = runner.auto_resolve();
}

#[test]
fn bt19_069_on_play_declining_hand_trash_skips_delete_entirely() {
    let mut runner = base().start();
    runner.game.turn_count = 1;

    let opp = runner.place_on_field(1, "OPP-LV4", Some(0)); // eligible target present
    let _fodder = push_to_hand(&mut runner, 0, "FODDER");
    let hand_index = push_to_hand(&mut runner, 0, CARD_ID);

    let opp_count_before = runner.battle_area_size(1);
    runner.play(0, hand_index).expect("play BT19-069");

    let hand_prompt = runner
        .pending_selection_view()
        .expect("optional hand-trash cost prompt must install");
    assert!(hand_prompt.is_optional);
    runner
        .execute_action(0, PASS)
        .expect("decline the optional hand-trash cost");

    assert!(
        runner.pending_selection().is_none(),
        "declining the cost must skip the delete entirely — no half-paid cost"
    );
    assert_eq!(
        runner.battle_area_size(1),
        opp_count_before,
        "declining the trash cost must not delete the eligible opponent Digimon"
    );
}

#[test]
fn bt19_069_on_play_with_empty_hand_is_a_no_op() {
    let mut runner = base().start();
    runner.game.turn_count = 1;

    let opp = runner.place_on_field(1, "OPP-LV4", Some(0)); // eligible target present
    let hand_index = push_to_hand(&mut runner, 0, CARD_ID);
    // No other cards pushed to hand — after playing BT19-069 itself leaves
    // the hand, the hand is empty.

    let opp_count_before = runner.battle_area_size(1);
    runner.play(0, hand_index).expect("play BT19-069");
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.hand_size(0),
        0,
        "test setup: hand must be empty after playing BT19-069 with nothing else in hand"
    );
    assert_eq!(
        runner.battle_area_size(1),
        opp_count_before,
        "with no card to trash, the delete effect must never fire"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 3 — [When Digivolving] positive/negative gating + behavior
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn bt19_069_when_digivolving_trashing_hand_deletes_eligible_opponent() {
    let mut runner = base().start();
    runner.game.turn_count = 1;
    let deltamon = place_deltamon(&mut runner);
    let opp = runner.place_on_field(1, "OPP-LV4", Some(0));
    let _fodder = push_to_hand(&mut runner, 0, "FODDER");

    let opp_count_before = runner.battle_area_size(1);
    fire_when_digivolving(&mut runner, deltamon);
    runner
        .auto_resolve()
        .expect("resolve when-digivolving flow");

    assert_eq!(
        runner.battle_area_size(1),
        opp_count_before - 1,
        "[When Digivolving] trashing a hand card must delete the eligible opponent Digimon"
    );
}

#[test]
fn bt19_069_when_digivolving_cannot_delete_opponent_above_level_4() {
    let mut runner = base().start();
    runner.game.turn_count = 1;
    let deltamon = place_deltamon(&mut runner);
    let opp = runner.place_on_field(1, "OPP-LV5", Some(0));
    let _fodder = push_to_hand(&mut runner, 0, "FODDER");

    let opp_count_before = runner.battle_area_size(1);
    fire_when_digivolving(&mut runner, deltamon);
    runner
        .auto_resolve()
        .expect("resolve when-digivolving flow");

    assert_eq!(
        runner.battle_area_size(1),
        opp_count_before,
        "[When Digivolving] must not delete an opponent Digimon above level 4"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 4 — [On Deletion] positive/negative gating + behavior
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn bt19_069_on_deletion_trashing_hand_deletes_eligible_opponent() {
    let mut runner = base().start();
    runner.game.turn_count = 1;
    let deltamon = place_deltamon(&mut runner);
    let opp = runner.place_on_field(1, "OPP-LV4", Some(0));
    let _fodder = push_to_hand(&mut runner, 0, "FODDER");

    let opp_count_before = runner.battle_area_size(1);
    runner
        .game
        .delete_permanent_with_cause(deltamon, ReplacementCause::OpponentEffect);
    runner.auto_resolve().expect("resolve on-deletion flow");

    assert_eq!(
        runner.battle_area_size(1),
        opp_count_before - 1,
        "[On Deletion] trashing a hand card must delete the eligible opponent Digimon"
    );
}

#[test]
fn bt19_069_on_deletion_with_empty_hand_is_a_no_op() {
    let mut runner = base().start();
    runner.game.turn_count = 1;
    let deltamon = place_deltamon(&mut runner);
    let opp = runner.place_on_field(1, "OPP-LV4", Some(0));
    // No cards in hand at all.

    let opp_count_before = runner.battle_area_size(1);
    runner
        .game
        .delete_permanent_with_cause(deltamon, ReplacementCause::OpponentEffect);
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.battle_area_size(1),
        opp_count_before,
        "[On Deletion] with an empty hand must never delete the opponent's Digimon"
    );
}

#[test]
fn bt19_069_on_deletion_declining_hand_trash_skips_delete() {
    let mut runner = base().start();
    runner.game.turn_count = 1;
    let deltamon = place_deltamon(&mut runner);
    let opp = runner.place_on_field(1, "OPP-LV4", Some(0));
    let _fodder = push_to_hand(&mut runner, 0, "FODDER");

    let opp_count_before = runner.battle_area_size(1);
    runner
        .game
        .delete_permanent_with_cause(deltamon, ReplacementCause::OpponentEffect);

    let hand_prompt = runner
        .pending_selection_view()
        .expect("optional hand-trash cost prompt must install on On Deletion too");
    assert!(hand_prompt.is_optional);
    runner
        .execute_action(0, PASS)
        .expect("decline the optional hand-trash cost");

    assert!(runner.pending_selection().is_none());
    assert_eq!(
        runner.battle_area_size(1),
        opp_count_before,
        "declining the On Deletion trash cost must not delete the opponent Digimon"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 5 — Cost-firing event-log test (GameEvent::Trash on the hand cost)
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn bt19_069_on_play_hand_trash_cost_fires_trash_event() {
    let mut runner = base().start();
    runner.game.turn_count = 1;

    let opp = runner.place_on_field(1, "OPP-LV4", Some(0));
    let _fodder = push_to_hand(&mut runner, 0, "FODDER");
    let hand_index = push_to_hand(&mut runner, 0, CARD_ID);

    let cp = runner.event_checkpoint();
    runner.play(0, hand_index).expect("play BT19-069");
    runner.auto_resolve().expect("resolve on-play flow");

    let events = runner.events_since(cp);
    let trash_count = events
        .iter()
        .filter(|e| matches!(e, GameEvent::Trash { player: 0, .. }))
        .count();
    assert!(
        trash_count >= 1,
        "the 'by trashing 1 card in your hand' cost must fire GameEvent::Trash"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 6 — <Blocker> (both top-card and buried-source positions)
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn bt19_069_has_blocker_while_standing_alone_as_top_card() {
    let mut runner = base().start();
    let deltamon = place_deltamon(&mut runner);

    assert!(
        runner.game.has_keyword(deltamon, Keyword::Blocker),
        "BT19-069 must have <Blocker> while it is itself the active top card \
         (DCGO PermanentCondition targets card.PermanentOfThisCard(), true \
         whether the card is the top or buried)"
    );
}

#[test]
fn bt19_069_grants_blocker_to_host_when_buried_as_a_digivolution_source() {
    let mut runner = base().start();
    // Stack a HOST card on top of BT19-069 — BT19-069 becomes a buried
    // digivolution source, HOST becomes the top card.
    let host = runner.place_stack(0, &[CARD_ID, "FILL"]);

    assert!(
        runner.game.has_keyword(host, Keyword::Blocker),
        "BT19-069's inherited <Blocker> must propagate to the host permanent \
         when BT19-069 is buried under a later digivolution"
    );
}

// ─── Helpers (mirrors BT25-017/BT24-041 idiom) ───────────────────────────────

fn push_to_hand(runner: &mut DebugRunner, player: u8, card_id: &str) -> usize {
    let data_index = runner
        .game
        .card_data
        .iter()
        .position(|card| card.card_id == card_id)
        .unwrap_or_else(|| panic!("unknown card id {card_id}"));
    let instance_id = runner.game.next_card_index();
    runner.game.players[player as usize]
        .hand
        .push(digimon_engine::card_source::CardSource::new(
            data_index,
            player,
            instance_id,
        ));
    runner.game.players[player as usize].hand.len() - 1
}
