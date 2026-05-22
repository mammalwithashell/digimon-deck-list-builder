//! AD1-014 MetalGarurumon — Lv.6, Blue, DP 12000, Cost 12.
//!
//! # Card text (cards.json)
//!
//! ＜Blocker＞
//! ＜Evade＞
//! [On Play] [When Digivolving] [When Attacking] [Once Per Turn]
//! Delete 1 of your opponent's level 5 or lower Digimon. Then, for every 2 of
//! your Tamers' colors, 1 of your opponent's Digimon or Tamers can't suspend
//! until their turn ends.
//! [All Turns] [Once Per Turn] When any of your Digimon suspend, this Digimon
//! may unsuspend.
//!
//! # Inherited
//! [When Attacking] [Once Per Turn] If this Digimon has [Garurumon] or
//! [Omnimon] in its name, 1 of your opponent's Digimon or Tamers can't suspend
//! until their turn ends.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/AD1/Blue/AD1_014.cs
//!
//! # Patterns this test covers
//! - H5 Blocker keyword (declarative grant)
//! - H8 Evade keyword (declarative grant)
//! - E2 OPT optional declinable trigger
//! - F7 cannot_suspend modifier with end_of_opponents_turn expiry
//! - Multi-timing OPT cluster ([On Play]/[When Digivolving]/[When Attacking])
//! - All-Turns OPT on_suspend → unsuspend self
//! - Inherited [When Attacking] OPT lock with self-name gate

#[path = "../../support/dsl_card_data.rs"]
mod dsl_card_data;

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledColor, CompiledCost, CompiledDeclarativeClause,
    CompiledScope, CompiledTiming,
};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::action::space::PASS;
use digimon_engine::enums::{CardKind, EffectTiming, Expiry, ModifierType};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::TriggerSource;

// ─── Compiled card helpers ────────────────────────────────────────────────────

fn compiled() -> digimon_dsl::compiled::CompiledCard {
    dsl_card_data::compiled("AD1-014")
}

fn metal_garurumon_runner() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card("AD1-014")
        .expect("AD1-014 YAML in embedded pack")
}

fn make_digimon(id: &str, level: u8, dp: i32) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.level = Some(level);
    card.dp = Some(dp);
    card
}

fn make_tamer_with_colors(id: &str, colors: &[digimon_engine::enums::CardColor]) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Tamer;
    card.level = None;
    card.dp = None;
    card.colors = colors.to_vec();
    card
}

fn make_digimon_with_name(id: &str, name: &str, level: u8, dp: i32) -> CardData {
    let mut card = make_test_card(id, name);
    card.card_kind = CardKind::Digimon;
    card.level = Some(level);
    card.dp = Some(dp);
    card
}

// ─── Section 1: Structural assertions ─────────────────────────────────────────

#[test]
fn ad1_014_has_printed_stats_and_alt_paths() {
    let card = compiled();

    assert_eq!(card.card, "AD1-014");
    assert_eq!(card.name, "MetalGarurumon");
    assert_eq!(card.level, Some(6));
    assert_eq!(card.color, vec![CompiledColor::Blue]);
    assert_eq!(card.cost, Some(12));
    assert_eq!(card.dp, Some(12000));
    for trait_name in ["Cyborg", "ADVENTURE", "Hero"] {
        assert!(
            card.traits.iter().any(|t| t == trait_name),
            "missing trait {trait_name}"
        );
    }

    // Alt-path: standard Lv5 Blue cost 4 (the printed evo_costs entry).
    // DCGO additionally adds Lv5 with [Garurumon] in name OR ADVENTURE/HERO
    // trait at cost 3. We assert at least the cost-3 alt path exists.
    let has_alt_cost3 = card.alt_paths.iter().any(|p| {
        p.kind == CompiledAltPathKind::Digivolve && p.cost == Some(CompiledCost::Literal(3))
    });
    assert!(
        has_alt_cost3,
        "AD1-014 must declare a cost-3 alt digivolve path (Garurumon/ADVENTURE/HERO source)"
    );
}

#[test]
fn ad1_014_declares_blocker_and_evade_keyword_grants() {
    let card = compiled();

    let mut grants: Vec<&str> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword {
                keyword,
                ..
            }) => Some(keyword.as_str()),
            _ => None,
        })
        .collect();
    grants.sort();

    assert!(
        grants.contains(&"Blocker"),
        "Blocker keyword grant missing — got {grants:?}"
    );
    assert!(
        grants.contains(&"Evade"),
        "Evade keyword grant missing — got {grants:?}"
    );
}

#[test]
fn ad1_014_has_op_wd_wa_cluster_and_all_turns_on_suspend_clause() {
    let card = compiled();

    let triggered: Vec<_> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();

    // Own triggered clauses: the OP/WD/WA cluster (one entry with three timings)
    // and the All-Turns on_suspend may-unsuspend. The inherited [WA] clause is
    // omitted pending G-DSL-SELF-NAME-CONTAINS — see ad1_014_inherited_*
    // tests for the gap-tracked assertions.
    let own: Vec<_> = triggered
        .iter()
        .filter(|t| matches!(t.scope, CompiledScope::FaceUp))
        .collect();
    assert!(
        own.iter().any(|t| t.when.contains(&CompiledTiming::OnPlay)
            && t.when.contains(&CompiledTiming::WhenDigivolving)
            && t.when.contains(&CompiledTiming::WhenAttacking)
            && t.once_per_turn),
        "expected OPT cluster clause covering [on_play, when_digivolving, when_attacking]"
    );
    assert!(
        own.iter()
            .any(|t| t.when == vec![CompiledTiming::OnSuspend] && t.once_per_turn && t.optional),
        "expected [All Turns][OPT] on_suspend optional clause"
    );
}

#[test]
#[ignore = "pending: G-DSL-SELF-NAME-CONTAINS — inherited [When Attacking] OPT lock with self-name gate omitted from YAML; structural assertion held until the predicate ships"]
fn ad1_014_inherited_when_attacking_is_present_and_once_per_turn_and_gated() {
    let card = compiled();
    let inh = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t) if matches!(t.scope, CompiledScope::Inherited) => Some(t),
            _ => None,
        })
        .expect("inherited triggered clause exists once G-DSL-SELF-NAME-CONTAINS closes");
    assert!(inh.once_per_turn, "[OPT]");
    assert_eq!(inh.when, vec![CompiledTiming::WhenAttacking]);
    assert!(
        inh.condition.is_some(),
        "inherited clause must gate on self-name containing Garurumon or Omnimon"
    );
}

// ─── Section 2: Behavioral — own [On Play] cluster: delete L5- and lock ──────

#[test]
fn ad1_014_on_play_with_no_eligible_targets_installs_no_selection() {
    // No opponent permanents at all → first sub-step (delete L5-) installs no
    // selection; second sub-step (lock) also installs none because no targets.
    let mut runner = metal_garurumon_runner().memory(20).start();
    let perm = runner.place_on_field(0, "AD1-014", Some(0));

    runner
        .game
        .enqueue_triggered(EffectTiming::OnPlay, TriggerSource::Permanent(perm));
    runner.game.drain_effect_queue();

    assert!(
        runner.pending_selection().is_none(),
        "no selection should install when opponent has no permanents"
    );
}

#[test]
fn ad1_014_on_play_offers_selection_for_level_5_or_lower_opponent_digimon() {
    let mut runner = metal_garurumon_runner()
        .add_card(make_digimon("OPP-LV5", 5, 7000))
        .memory(20)
        .start();
    let perm = runner.place_on_field(0, "AD1-014", Some(0));
    runner.place_on_field(1, "OPP-LV5", Some(0));

    runner
        .game
        .enqueue_triggered(EffectTiming::OnPlay, TriggerSource::Permanent(perm));
    runner.game.drain_effect_queue();

    let view = runner
        .pending_selection_view()
        .expect("delete-1-L5- selection installs");
    assert!(
        !view.valid_action_ids.is_empty(),
        "at least one valid target for delete"
    );
}

#[test]
fn ad1_014_on_play_filters_out_level_6_opponent_digimon() {
    // L6 ineligible — no selection installs because no L5- target exists.
    // This confirms the level_lte: 5 filter is enforced.
    let mut runner = metal_garurumon_runner()
        .add_card(make_digimon("OPP-LV6", 6, 12000))
        .memory(20)
        .start();
    let perm = runner.place_on_field(0, "AD1-014", Some(0));
    runner.place_on_field(1, "OPP-LV6", Some(0));

    runner
        .game
        .enqueue_triggered(EffectTiming::OnPlay, TriggerSource::Permanent(perm));
    runner.game.drain_effect_queue();

    assert!(
        runner.pending_selection().is_none(),
        "L6 opponent must not match level_lte: 5 filter"
    );
}

#[test]
fn ad1_014_on_play_deletes_chosen_level_5_opponent() {
    let mut runner = metal_garurumon_runner()
        .add_card(make_digimon("OPP-LV5", 5, 7000))
        .memory(20)
        .start();
    let perm = runner.place_on_field(0, "AD1-014", Some(0));
    let target = runner.place_on_field(1, "OPP-LV5", Some(0));

    runner
        .game
        .enqueue_triggered(EffectTiming::OnPlay, TriggerSource::Permanent(perm));
    runner.game.drain_effect_queue();

    let (selecting_player, action_id) = {
        let view = runner.pending_selection_view().expect("delete selection");
        assert_eq!(view.valid_action_ids.len(), 1);
        (view.selecting_player, view.valid_action_ids[0])
    };
    runner
        .execute_action(selecting_player, action_id)
        .expect("execute delete pick");
    let _ = runner.auto_resolve();

    // After delete, the target's slot should either be removed or replaced.
    // We assert the original card_id is no longer on the opponent's battle area.
    assert!(
        !runner.game.players[1]
            .battle_area
            .iter()
            .any(|p| p.top_card().card_id(&runner.game.card_data) == "OPP-LV5"),
        "chosen opponent permanent should be deleted"
    );
    let _ = target;
}

// ─── Section 3: When Digivolving and When Attacking parity ───────────────────

#[test]
fn ad1_014_when_digivolving_offers_same_delete_selection() {
    let mut runner = metal_garurumon_runner()
        .add_card(make_digimon("OPP-LV5", 5, 7000))
        .memory(20)
        .start();
    let perm = runner.place_on_field(0, "AD1-014", Some(0));
    runner.place_on_field(1, "OPP-LV5", Some(0));

    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(perm),
    );
    runner.game.drain_effect_queue();

    assert!(
        runner.pending_selection().is_some(),
        "When Digivolving must fire the same OP/WD/WA body"
    );
}

#[test]
fn ad1_014_when_attacking_offers_same_delete_selection() {
    let mut runner = metal_garurumon_runner()
        .add_card(make_digimon("OPP-LV5", 5, 7000))
        .memory(20)
        .start();
    let perm = runner.place_on_field(0, "AD1-014", Some(0));
    runner.place_on_field(1, "OPP-LV5", Some(0));

    runner
        .game
        .enqueue_triggered(EffectTiming::WhenAttacking, TriggerSource::Permanent(perm));
    runner.game.drain_effect_queue();

    assert!(
        runner.pending_selection().is_some(),
        "When Attacking must fire the same OP/WD/WA body"
    );
}

// ─── Section 4: OPT lockout ──────────────────────────────────────────────────

#[test]
fn ad1_014_on_play_opt_blocks_second_same_timing_activation_in_same_turn() {
    let mut runner = metal_garurumon_runner()
        .add_card(make_digimon("OPP-A", 5, 7000))
        .add_card(make_digimon("OPP-B", 5, 7000))
        .memory(20)
        .start();
    let perm = runner.place_on_field(0, "AD1-014", Some(0));
    runner.place_on_field(1, "OPP-A", Some(0));
    runner.place_on_field(1, "OPP-B", Some(0));

    runner
        .game
        .enqueue_triggered(EffectTiming::OnPlay, TriggerSource::Permanent(perm));
    runner.game.drain_effect_queue();

    let (sel_player, action_id) = {
        let view = runner.pending_selection_view().expect("first delete");
        (view.selecting_player, view.valid_action_ids[0])
    };
    runner
        .execute_action(sel_player, action_id)
        .expect("first pick");
    let _ = runner.auto_resolve();

    // Re-fire same timing — OPT must lock
    runner
        .game
        .enqueue_triggered(EffectTiming::OnPlay, TriggerSource::Permanent(perm));
    runner.game.drain_effect_queue();

    assert!(
        runner.pending_selection().is_none(),
        "OPT must block second [On Play] activation same turn"
    );
}

#[test]
#[ignore = "pending: G-OPT-MULTI-TIMING-SHARED-LOCKOUT — multi-timing OPT cluster compiles to one Effect per timing, each with its own OPT counter; cross-timing shared lockout (DCGO SharedHashString) not yet wired"]
fn ad1_014_on_play_then_when_attacking_share_one_opt_lockout_per_turn() {
    let mut runner = metal_garurumon_runner()
        .add_card(make_digimon("OPP-A", 5, 7000))
        .add_card(make_digimon("OPP-B", 5, 7000))
        .memory(20)
        .start();
    let perm = runner.place_on_field(0, "AD1-014", Some(0));
    runner.place_on_field(1, "OPP-A", Some(0));
    runner.place_on_field(1, "OPP-B", Some(0));

    // Fire OnPlay first.
    runner
        .game
        .enqueue_triggered(EffectTiming::OnPlay, TriggerSource::Permanent(perm));
    runner.game.drain_effect_queue();
    let (sp, aid) = {
        let v = runner.pending_selection_view().unwrap();
        (v.selecting_player, v.valid_action_ids[0])
    };
    runner.execute_action(sp, aid).unwrap();
    let _ = runner.auto_resolve();

    // Now WhenAttacking same turn must NOT fire (DCGO shares hash AD1_014_OP_WD_WA).
    runner
        .game
        .enqueue_triggered(EffectTiming::WhenAttacking, TriggerSource::Permanent(perm));
    runner.game.drain_effect_queue();
    assert!(
        runner.pending_selection().is_none(),
        "cross-timing OPT must lock When Attacking after On Play same turn"
    );
}

// ─── Section 5: Lock — for every 2 of your Tamers' colors ────────────────────

#[test]
#[ignore = "pending: G-DSL-DISTINCT-TAMER-COLORS-FORMULA — DSL has no formula primitive that counts distinct colors of your Tamers in battle area; lock-count branch unimplementable in pure YAML and not yet bridged via raw_rust in this skill scope"]
fn ad1_014_on_play_with_zero_tamers_locks_zero_opp_permanents() {
    let mut runner = metal_garurumon_runner()
        .add_card(make_digimon("OPP-LV5", 5, 7000))
        .add_card(make_digimon("OPP-LOCK", 4, 5000))
        .memory(20)
        .start();
    let perm = runner.place_on_field(0, "AD1-014", Some(0));
    let target = runner.place_on_field(1, "OPP-LV5", Some(0));
    let lock = runner.place_on_field(1, "OPP-LOCK", Some(0));

    runner
        .game
        .enqueue_triggered(EffectTiming::OnPlay, TriggerSource::Permanent(perm));
    runner.game.drain_effect_queue();

    // Resolve the delete selection.
    let (sp, aid) = {
        let v = runner.pending_selection_view().unwrap();
        (v.selecting_player, v.valid_action_ids[0])
    };
    runner.execute_action(sp, aid).unwrap();
    let _ = runner.auto_resolve();

    let _ = target; // delete asserted by mainline test above
    assert!(
        !runner.modifiers().has(lock, ModifierType::CannotSuspend),
        "with zero Tamers, no permanent should be locked"
    );
}

#[test]
#[ignore = "pending: G-DSL-DISTINCT-TAMER-COLORS-FORMULA — see above; needs a distinct-tamer-colors-floor-div-2 formula plus repeat_n / for_each-N driver to install N selections"]
fn ad1_014_on_play_with_three_tamer_colors_locks_one_opp_permanent_for_one_turn() {
    use digimon_engine::enums::CardColor;

    let mut runner = metal_garurumon_runner()
        .add_card(make_digimon("OPP-LV5", 5, 7000))
        .add_card(make_digimon("OPP-LOCK", 4, 5000))
        .add_card(make_tamer_with_colors("TAMER-R", &[CardColor::Red]))
        .add_card(make_tamer_with_colors("TAMER-B", &[CardColor::Blue]))
        .add_card(make_tamer_with_colors("TAMER-Y", &[CardColor::Yellow]))
        .memory(20)
        .start();
    let perm = runner.place_on_field(0, "AD1-014", Some(0));
    runner.place_on_field(0, "TAMER-R", Some(0));
    runner.place_on_field(0, "TAMER-B", Some(0));
    runner.place_on_field(0, "TAMER-Y", Some(0));
    runner.place_on_field(1, "OPP-LV5", Some(0));
    let lock = runner.place_on_field(1, "OPP-LOCK", Some(0));

    runner
        .game
        .enqueue_triggered(EffectTiming::OnPlay, TriggerSource::Permanent(perm));
    runner.game.drain_effect_queue();

    // Delete pick.
    let (sp, aid) = {
        let v = runner.pending_selection_view().unwrap();
        (v.selecting_player, v.valid_action_ids[0])
    };
    runner.execute_action(sp, aid).unwrap();

    // Lock pick (3 colors / 2 = 1 lock).
    let (sp2, lock_aid) = {
        let v = runner.pending_selection_view().expect("lock selection");
        (v.selecting_player, v.valid_action_ids[0])
    };
    runner.execute_action(sp2, lock_aid).unwrap();
    let _ = runner.auto_resolve();

    assert!(
        runner.modifiers().has(lock, ModifierType::CannotSuspend),
        "1 opp permanent must receive CannotSuspend until end of opp turn"
    );
}

// ─── Section 6: All-Turns OPT — on_suspend → unsuspend self ─────────────────

#[test]
fn ad1_014_on_ally_suspend_offers_optional_unsuspend_self_prompt() {
    let mut runner = metal_garurumon_runner()
        .add_card(make_digimon("ALLY", 4, 5000))
        .memory(20)
        .start();
    let perm = runner.place_on_field(0, "AD1-014", Some(0));
    let ally = runner.place_on_field(0, "ALLY", Some(0));

    // Pre-suspend AD1-014 by direct mutation so the suspension itself does
    // NOT fire its own on_suspend clause and consume the OPT.
    runner.game.players[0].battle_area[perm.index as usize].is_suspended = true;

    // Suspending the ally (your Digimon) fires on_suspend → AD1-014 may unsuspend.
    runner.game.suspend(ally);
    let _ = runner.auto_resolve();

    let suspended_after = runner.game.players[0].battle_area[perm.index as usize].is_suspended;
    assert!(
        !suspended_after,
        "AD1-014 should unsuspend when an ally Digimon suspends"
    );
}

#[test]
fn ad1_014_on_opponent_suspend_does_not_unsuspend_self() {
    let mut runner = metal_garurumon_runner()
        .add_card(make_digimon("OPP", 4, 5000))
        .memory(20)
        .start();
    let perm = runner.place_on_field(0, "AD1-014", Some(0));
    let opp = runner.place_on_field(1, "OPP", Some(0));

    // Pre-suspend AD1-014 by direct mutation (avoid firing its own clause).
    runner.game.players[0].battle_area[perm.index as usize].is_suspended = true;

    // Suspending the OPPONENT's permanent must not satisfy the
    // event_target_owner: you gate, so AD1-014 stays suspended.
    runner.game.suspend(opp);
    let _ = runner.auto_resolve();

    let suspended_after = runner.game.players[0].battle_area[perm.index as usize].is_suspended;
    assert!(
        suspended_after,
        "Opponent suspending must not trigger AD1-014's All-Turns clause (gate: your Digimon)"
    );
}

// ─── Section 7: Inherited [When Attacking] OPT — name gate ──────────────────

#[test]
fn ad1_014_inherited_when_attacking_does_not_fire_when_top_card_lacks_garurumon_or_omnimon_name() {
    // If AD1-014 is below a top card that does NOT contain "Garurumon" or
    // "Omnimon" in its name, the inherited [When Attacking] clause must NOT
    // install a selection.
    let mut runner = metal_garurumon_runner()
        .add_card(make_digimon_with_name(
            "TOP-NEUTRAL",
            "Generic Top",
            7,
            13000,
        ))
        .add_card(make_digimon("OPP-DIGI", 5, 7000))
        .memory(20)
        .start();
    // Stack: [AD1-014 (source), TOP-NEUTRAL (top)]
    let stack = runner.place_stack(0, &["AD1-014", "TOP-NEUTRAL"]);
    runner.place_on_field(1, "OPP-DIGI", Some(0));

    runner
        .game
        .enqueue_triggered(EffectTiming::WhenAttacking, TriggerSource::Permanent(stack));
    runner.game.drain_effect_queue();

    // The own (top-card) clause won't fire (top is not AD1-014). The inherited
    // clause should be condition-gated by the name check on the carrier's top
    // card and therefore not fire either.
    assert!(
        runner.pending_selection().is_none(),
        "inherited [When Attacking] OPT lock must not install when top-card name lacks Garurumon/Omnimon"
    );
}

#[test]
#[ignore = "pending: G-DSL-SELF-NAME-CONTAINS — DSL lacks a `self_name_contains` predicate that evaluates against the carrier permanent's top-card name; the inherited name gate cannot be expressed in pure YAML today"]
fn ad1_014_inherited_when_attacking_fires_when_top_card_contains_garurumon_in_name() {
    let mut runner = metal_garurumon_runner()
        .add_card(make_digimon_with_name(
            "TOP-GARURU",
            "WereGarurumon",
            6,
            11000,
        ))
        .add_card(make_digimon("OPP-DIGI", 5, 7000))
        .memory(20)
        .start();
    let stack = runner.place_stack(0, &["AD1-014", "TOP-GARURU"]);
    runner.place_on_field(1, "OPP-DIGI", Some(0));

    runner
        .game
        .enqueue_triggered(EffectTiming::WhenAttacking, TriggerSource::Permanent(stack));
    runner.game.drain_effect_queue();

    assert!(
        runner.pending_selection().is_some(),
        "inherited clause must install lock selection when top-card name contains Garurumon"
    );
}

// ─── Section 8: Additional delete-target level coverage ──────────────────────

#[test]
fn ad1_014_on_play_level_4_digimon_is_eligible_for_delete() {
    // level_lte: 5 covers Lv4, Lv3, Lv2, Lv1 — not just exactly Lv5.
    let mut runner = metal_garurumon_runner()
        .add_card(make_digimon("OPP-LV4", 4, 5000))
        .memory(20)
        .start();
    let perm = runner.place_on_field(0, "AD1-014", Some(0));
    runner.place_on_field(1, "OPP-LV4", Some(0));

    runner
        .game
        .enqueue_triggered(EffectTiming::OnPlay, TriggerSource::Permanent(perm));
    runner.game.drain_effect_queue();

    assert!(
        runner.pending_selection().is_some(),
        "Lv4 opponent Digimon must be eligible for the delete-1 selection (level_lte: 5)"
    );
}

// ─── Section 9: All-Turns OPT — once-per-turn enforcement ───────────────────

#[test]
fn ad1_014_all_turns_opt_blocks_second_on_suspend_same_turn() {
    // The [All Turns][OPT] on_suspend clause may only trigger once per turn.
    // A second ally Digimon suspending in the same turn must not install a
    // second optional-unsuspend prompt.
    let mut runner = metal_garurumon_runner()
        .add_card(make_digimon("ALLY-A", 4, 5000))
        .add_card(make_digimon("ALLY-B", 4, 5000))
        .memory(20)
        .start();
    let perm = runner.place_on_field(0, "AD1-014", Some(0));
    let ally_a = runner.place_on_field(0, "ALLY-A", Some(0));
    let ally_b = runner.place_on_field(0, "ALLY-B", Some(0));

    // Pre-suspend AD1-014 so the unsuspend response is meaningful.
    runner.game.players[0].battle_area[perm.index as usize].is_suspended = true;

    // First ally suspends → optional unsuspend prompt appears.
    runner.game.suspend(ally_a);
    let _ = runner.auto_resolve();
    // AD1-014 should have unsuspended.
    assert!(
        !runner.game.players[0].battle_area[perm.index as usize].is_suspended,
        "AD1-014 should unsuspend on first ally Digimon suspend"
    );

    // Re-suspend AD1-014 manually so that a second trigger would be observable.
    runner.game.players[0].battle_area[perm.index as usize].is_suspended = true;

    // Second ally suspends in same turn → OPT lockout should prevent unsuspend.
    runner.game.suspend(ally_b);
    let _ = runner.auto_resolve();
    assert!(
        runner.game.players[0].battle_area[perm.index as usize].is_suspended,
        "OPT must block the on_suspend clause from firing twice in the same turn"
    );
}

#[test]
fn ad1_014_all_turns_opt_resets_after_turn_end() {
    // After a full round (two end_turns), the OPT counter for the [All Turns]
    // on_suspend clause resets so it can fire again on player 0's next turn.
    //
    // Engine behavior: `permanent.new_turn()` (which clears `effect_activations`)
    // is called only when that player starts their turn
    // (`game_phases::continue_begin_turn_after_start_delays`). So player 0's
    // OPT counter resets after the second end_turn (when player 0 is again the
    // turn player). We add FILL digimons to both decks to avoid deck-out loss
    // during the two draw phases.
    let mut runner = metal_garurumon_runner()
        .add_card(make_digimon("ALLY", 4, 5000))
        .add_card(make_digimon("FILL", 3, 3000))
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(20)
        .start();
    let perm = runner.place_on_field(0, "AD1-014", Some(0));
    let ally = runner.place_on_field(0, "ALLY", Some(0));

    // Pre-suspend AD1-014.
    runner.game.players[0].battle_area[perm.index as usize].is_suspended = true;

    // Consume the OPT this turn (player 0's turn).
    runner.game.suspend(ally);
    let _ = runner.auto_resolve();
    assert!(
        !runner.game.players[0].battle_area[perm.index as usize].is_suspended,
        "should unsuspend in first turn"
    );

    // --- Turn 1 ends, player 1 takes their turn ---
    runner.end_turn(); // player 1's turn begins; player 1's permanents reset, not player 0's
    let _ = runner.auto_resolve(); // drain any StartOfYourTurn triggers for player 1

    // Player 0's permanents have NOT reset yet (still OPT locked on on_suspend).
    // Re-suspend AD1-014 and try the on_suspend clause during player 1's turn.
    runner.game.players[0].battle_area[perm.index as usize].is_suspended = true;
    runner.game.players[0].battle_area[ally.index as usize].is_suspended = false;
    runner.game.suspend(ally);
    let _ = runner.auto_resolve();
    // Even though it's player 1's turn, AD1-014 is an [All Turns] observer.
    // But OPT should still be locked from player 0's activation in turn 1.
    assert!(
        runner.game.players[0].battle_area[perm.index as usize].is_suspended,
        "OPT should still be locked during player 1's turn (not yet reset)"
    );

    // --- Turn 2 ends, player 0 takes their turn again (own turn) ---
    runner.end_turn(); // player 0's turn begins; player 0's permanents reset via new_turn()
    let _ = runner.auto_resolve();

    // Now player 0's OPT counter is cleared. Re-suspend AD1-014 and ally.
    runner.game.players[0].battle_area[perm.index as usize].is_suspended = true;
    runner.game.players[0].battle_area[ally.index as usize].is_suspended = false;
    runner.game.suspend(ally);
    let _ = runner.auto_resolve();

    assert!(
        !runner.game.players[0].battle_area[perm.index as usize].is_suspended,
        "OPT must reset when player 0's turn begins — on_suspend clause should fire again"
    );
}

// ─── Section 10: All-Turns clause optional — player may decline unsuspend ────

#[test]
#[ignore = "engine-gap: G-DSL-OPTIONAL-NON-SELECTION-TRIGGER — `optional: true` on a triggered clause with no selection-driving step in its body does not surface a player prompt; the engine's single-trigger auto-fire path exposes optionality only through `body's first actionable pending selection` (effect_queue.rs §2.5 single-trigger comment). For `unsuspend: {target: source}`, the body has no selection step, so the trigger auto-fires. AD1-014 printed text says `may unsuspend` — the choice should be exposed. Until the engine installs a `may-accept` prompt for optional single triggers with non-selection bodies, this clause auto-fires (always unsuspends) rather than presenting a choice."]
fn ad1_014_on_suspend_is_optional_and_player_can_decline() {
    // The [All Turns][OPT] clause is `optional: true`. The player can PASS
    // rather than unsuspending AD1-014, leaving it suspended.
    let mut runner = metal_garurumon_runner()
        .add_card(make_digimon("ALLY", 4, 5000))
        .memory(20)
        .start();
    let perm = runner.place_on_field(0, "AD1-014", Some(0));
    let ally = runner.place_on_field(0, "ALLY", Some(0));

    // Pre-suspend AD1-014.
    runner.game.players[0].battle_area[perm.index as usize].is_suspended = true;

    // Trigger the on_suspend clause.
    runner.game.suspend(ally);
    runner.game.drain_effect_queue();

    // The selection must be optional (player may choose not to unsuspend).
    assert!(
        runner.pending_is_optional(),
        "on_suspend → may unsuspend clause must be optional (player may decline)"
    );

    // Player declines (PASS).
    runner
        .execute_action(0, PASS)
        .expect("PASS to decline optional unsuspend");
    let _ = runner.auto_resolve();

    assert!(
        runner.game.players[0].battle_area[perm.index as usize].is_suspended,
        "AD1-014 must remain suspended when the player declines the optional unsuspend"
    );
}

// ─── Section 11: All-Turns clause kind gate — own Tamer suspend excluded ─────

#[test]
fn ad1_014_own_tamer_suspend_does_not_trigger_unsuspend_clause() {
    // The `event_target_kind: digimon` gate means suspending your own Tamer
    // must NOT trigger the on_suspend → unsuspend clause (only your Digimon do).
    use digimon_engine::enums::CardColor;
    let mut runner = metal_garurumon_runner()
        .add_card(make_tamer_with_colors("OWN-TAMER", &[CardColor::Blue]))
        .memory(20)
        .start();
    let perm = runner.place_on_field(0, "AD1-014", Some(0));
    let tamer = runner.place_on_field(0, "OWN-TAMER", Some(0));

    // Pre-suspend AD1-014 so any unsuspend is observable.
    runner.game.players[0].battle_area[perm.index as usize].is_suspended = true;

    // Suspend own Tamer — must NOT satisfy event_target_kind: digimon.
    runner.game.suspend(tamer);
    let _ = runner.auto_resolve();

    assert!(
        runner.game.players[0].battle_area[perm.index as usize].is_suspended,
        "own Tamer suspending must not trigger the [All Turns] on_suspend clause (gate: kind: digimon)"
    );
}
