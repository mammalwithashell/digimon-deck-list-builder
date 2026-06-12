//! BT16-101 Rapidmon (X Antibody) — Digimon, Lv.6, Yellow/Green, Cost 11, DP 11000.
//! Traits: Holy Warrior / X Antibody. Attribute: Vaccine.
//!
//! # Card text (BT16-101.json / cards.json — verbatim)
//!
//! ```text
//! <Armor Purge> (When this Digimon would be deleted, you may trash the top
//!   card of this Digimon to prevent that deletion.)
//! [When Digivolving] Suspend all of your opponent's Digimon. Then, this
//!   Digimon may attack.
//! [All Turns] While [Rapidmon] or [X Antibody] is in this Digimon's
//!   digivolution cards, all of your opponent's suspended Digimon get -4000 DP.
//! [All Turns] [Once Per Turn] When an opponent's Digimon is deleted in battle
//!   or by having 0 DP, gain 2 memory.
//! ```
//!
//! xros_req: [Digivolve] [Rapidmon]: Cost 4
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT16/Yellow/BT16_101.cs
//!
//! # DSL YAML
//! code/digimon-engine/cards/bt16/BT16-101.yaml
//!
//! # Patterns this test covers
//! - Track-B replacement keyword: <Armor Purge> (grant_keyword: ArmorPurge)
//! - [When Digivolving] for_each-suspend over opp Digimon + may_attack_now (self)
//! - D4/conditional aura: opp suspended Digimon -4000 DP, gated on the
//!   carrier's digivolution SOURCES containing [Rapidmon] / [X Antibody]
//! - F5/Track-A deletion observer: [All Turns][OPT] gain 2 memory when an opp
//!   Digimon is deleted in battle OR by having 0 DP (NOT by effect)
//! - E3/OPT lockout: the single once_per_turn gain clause caps at +2/turn
//!
//! # Clause map
//! | Clause | Timing | Effect |
//! |--------|--------|--------|
//! | 0 | <Armor Purge> | prevent-deletion-by-trashing-top keyword |
//! | 1 | [When Digivolving] | suspend all opp Digimon; then this Digimon may attack |
//! | 2 | [All Turns] aura | while [Rapidmon]/[X Antibody] in sources, opp SUSPENDED Digimon -4000 DP |
//! | 3 | [All Turns][OPT] | when opp Digimon deleted in battle / by 0 DP, gain 2 memory |

#![allow(dead_code, unused_imports)]

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledColor, CompiledCost, CompiledDeclarativeClause,
    CompiledPlayerRef, CompiledPredicate, CompiledScope, CompiledTiming, CompiledTriggeredClause,
};
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::{CardHandle, CardSource};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::effect_context::EffectContext;
use digimon_engine::enums::{CardColor, CardKind, Expiry, Keyword, ModifierType};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::replacement::ReplacementCause;
use digimon_engine::selection::SelectionKind;

use super::super::dsl_card_data::compiled;

const CARD_ID: &str = "BT16-101";

// ─── Card-data helpers ────────────────────────────────────────────────────────

/// A vanilla opponent Lv.6 Digimon (deletion / suspend / -DP target).
fn make_opp_digimon(id: &str, dp: i32) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(6);
    c.dp = Some(dp);
    c.play_cost = 6;
    c.colors = vec![CardColor::Red];
    c.traits = vec![];
    c
}

/// A Lv.5 yellow base for the standard digivolve onto BT16-101 (NOT named
/// Rapidmon, NO X Antibody trait — used to assert the aura stays OFF).
fn make_plain_base(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(5);
    c.dp = Some(8000);
    c.play_cost = 5;
    c.colors = vec![CardColor::Yellow];
    c.traits = vec![];
    c
}

/// A digivolution source whose NAME is exactly "Rapidmon" (matches the aura's
/// `self_digivolution_sources_contain_name: "Rapidmon"` gate).
fn make_rapidmon_source(id: &str) -> CardData {
    let mut c = make_test_card(id, "Rapidmon");
    c.card_kind = CardKind::Digimon;
    c.level = Some(5);
    c.dp = Some(8000);
    c.play_cost = 5;
    c.colors = vec![CardColor::Yellow];
    c.traits = vec![];
    c
}

/// A digivolution source with the [X Antibody] trait (and NOT named Rapidmon)
/// — matches the aura's trait arm.
fn make_x_antibody_source(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(5);
    c.dp = Some(8000);
    c.play_cost = 5;
    c.colors = vec![CardColor::Yellow];
    c.traits = vec!["X Antibody".to_string()];
    c
}

/// Base runner: BT16-101 registered with generous memory.
fn rapidmon_runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT16-101 in embedded DSL pack")
        .add_card(make_opp_digimon("OPP-A", 5000))
        .add_card(make_opp_digimon("OPP-B", 4000))
        .add_card(make_plain_base("PLAIN-LV5"))
        .add_card(make_rapidmon_source("RAPID-SRC"))
        .add_card(make_x_antibody_source("XANTI-SRC"))
        .add_card(make_test_card("FILLER", "Filler"))
        .hand(0, &[CARD_ID])
        .deck(0, &["FILLER", "FILLER", "FILLER", "FILLER"])
        .deck(1, &["FILLER", "FILLER"])
        .memory(10)
        .start()
}

// ════════════════════════════════════════════════════════════════════════════
// Section 1 — Structural assertions
// ════════════════════════════════════════════════════════════════════════════

#[test]
fn bt16_101_compiles_from_dsl_pack() {
    let _ = compiled(CARD_ID);
}

#[test]
fn bt16_101_is_yellow_green_lv6_holy_warrior_x_antibody() {
    let card = compiled(CARD_ID);
    assert_eq!(card.kind, digimon_dsl::compiled::CompiledCardKind::Digimon);
    assert_eq!(card.level, Some(6));
    assert_eq!(card.cost, Some(11));
    assert_eq!(card.dp, Some(11000));
    assert!(card.color.contains(&CompiledColor::Yellow));
    assert!(card.color.contains(&CompiledColor::Green));
    assert!(card.traits.iter().any(|t| t == "Holy Warrior"));
    assert!(card.traits.iter().any(|t| t == "X Antibody"));
}

/// Alt-path: [Digivolve] [Rapidmon]: Cost 4.
#[test]
fn bt16_101_has_rapidmon_alt_path_cost4() {
    let card = compiled(CARD_ID);
    let rapid = card
        .alt_paths
        .iter()
        .find(|p| p.kind == CompiledAltPathKind::Digivolve && p.cost == Some(CompiledCost::Literal(4)))
        .expect("must have a [Rapidmon] cost-4 digivolve alt-path");
    assert_eq!(rapid.cost, Some(CompiledCost::Literal(4)));
}

/// <Armor Purge> is declared as a keyword grant.
#[test]
fn bt16_101_grants_armor_purge_keyword() {
    let card = compiled(CARD_ID);
    let has_armor_purge = card.effects.iter().any(|c| matches!(
        c,
        CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword { keyword, .. })
            if format!("{keyword:?}").contains("ArmorPurge")
    ));
    assert!(
        has_armor_purge,
        "BT16-101 must declare <Armor Purge> via grant_keyword: ArmorPurge; clauses: {:?}",
        card.effects
    );
}

/// Exactly one [When Digivolving] clause.
#[test]
fn bt16_101_has_one_when_digivolving_clause() {
    let card = compiled(CARD_ID);
    let wd: Vec<&CompiledTriggeredClause> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::WhenDigivolving) => {
                Some(t)
            }
            _ => None,
        })
        .collect();
    assert_eq!(wd.len(), 1, "exactly one [When Digivolving] clause");
}

/// Exactly one OnAnyDeletion observer, [Once Per Turn], optional ("gain 2
/// memory" with no "may" — but the firing is gated; DCGO uses a single OPT
/// shell — the clause must be once_per_turn).
#[test]
fn bt16_101_has_one_once_per_turn_deletion_observer() {
    let card = compiled(CARD_ID);
    let del: Vec<&CompiledTriggeredClause> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::OnAnyDeletion) => {
                Some(t)
            }
            _ => None,
        })
        .collect();
    assert_eq!(
        del.len(),
        1,
        "exactly one OnAnyDeletion observer (the single [Once Per Turn] gain-2 clause)"
    );
    assert!(
        del[0].once_per_turn,
        "the gain-2-memory clause is [Once Per Turn] (must not gain 4 in a turn)"
    );
}

/// The deletion observer gates on the deleted object being an OPPONENT Digimon.
fn predicate_mentions_event_target_owner_opponent(p: &CompiledPredicate) -> bool {
    p.event_target_owner == Some(CompiledPlayerRef::Opponent)
        || p.all_of
            .iter()
            .any(predicate_mentions_event_target_owner_opponent)
        || p.any_of
            .iter()
            .any(predicate_mentions_event_target_owner_opponent)
}

fn predicate_mentions_event_cause(p: &CompiledPredicate) -> bool {
    p.event_cause.is_some()
        || p.all_of.iter().any(predicate_mentions_event_cause)
        || p.any_of.iter().any(predicate_mentions_event_cause)
        || p.none_of.iter().any(predicate_mentions_event_cause)
        || p.not
            .as_ref()
            .map(|n| predicate_mentions_event_cause(n))
            .unwrap_or(false)
}

#[test]
fn bt16_101_deletion_observer_gates_on_opponent_digimon_and_cause() {
    let card = compiled(CARD_ID);
    let del = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::OnAnyDeletion) => {
                Some(t)
            }
            _ => None,
        })
        .expect("OnAnyDeletion observer must exist");
    let cond = del
        .condition
        .as_ref()
        .expect("deletion observer must gate on the deleted object + cause");
    assert!(
        predicate_mentions_event_target_owner_opponent(cond),
        "must require the deleted Digimon to be the opponent's"
    );
    assert!(
        predicate_mentions_event_cause(cond),
        "must gate on the deletion CAUSE (battle / 0-DP rule, not effect)"
    );
}

/// The aura clause exists and is a declarative aura with a -4000 DP modifier.
#[test]
fn bt16_101_has_minus4000_aura() {
    let card = compiled(CARD_ID);
    let has_aura = card.effects.iter().any(|c| matches!(
        c,
        CompiledClause::Declarative(CompiledDeclarativeClause::Aura { dp_modifier: Some(-4000), .. })
    ));
    assert!(
        has_aura,
        "BT16-101 must carry an aura with dp_modifier -4000; clauses: {:?}",
        card.effects
    );
}

// ════════════════════════════════════════════════════════════════════════════
// Section 2 — [When Digivolving] suspend all opp + this Digimon may attack
// ════════════════════════════════════════════════════════════════════════════

/// Build BT16-101 atop a plain Lv.5 base, then enqueue WhenDigivolving.
fn digivolve_rapidmon_over_plain(runner: &mut DebugRunner) -> PermanentHandle {
    let rapid = runner.place_stack(0, &["PLAIN-LV5", CARD_ID]);
    runner.game.enqueue_triggered(
        digimon_engine::enums::EffectTiming::WhenDigivolving,
        digimon_engine::selection::TriggerSource::Permanent(rapid),
    );
    rapid
}

/// POSITIVE: [When Digivolving] suspends ALL of the opponent's Digimon.
#[test]
fn bt16_101_when_digivolving_suspends_all_opponent_digimon() {
    let mut runner = rapidmon_runner();
    let opp_a = runner.place_on_field(1, "OPP-A", Some(0));
    let opp_b = runner.place_on_field(1, "OPP-B", Some(0));
    // Sanity: opponents start unsuspended.
    assert!(!runner.game.player(1).battle_area[opp_a.index as usize].is_suspended);
    assert!(!runner.game.player(1).battle_area[opp_b.index as usize].is_suspended);

    let _rapid = digivolve_rapidmon_over_plain(&mut runner);
    runner.game.drain_effect_queue();
    // The for_each suspend runs with no selection. Then the may-attack-now is
    // optional; decline so we can inspect the suspend result.
    let _ = runner.auto_resolve();

    assert!(
        runner.game.player(1).battle_area[opp_a.index as usize].is_suspended,
        "opp A must be suspended by [When Digivolving]"
    );
    assert!(
        runner.game.player(1).battle_area[opp_b.index as usize].is_suspended,
        "opp B must be suspended by [When Digivolving]"
    );
}

/// "Then, this Digimon may attack" — after the suspend, an optional may-attack
/// selection (or attack prompt) targeting THIS Digimon (the source) is offered.
/// The printed "may" makes the attack optional (PASS legal).
#[test]
fn bt16_101_when_digivolving_then_this_digimon_may_attack() {
    let mut runner = rapidmon_runner();
    let _opp = runner.place_on_field(1, "OPP-A", Some(0));
    let _rapid = digivolve_rapidmon_over_plain(&mut runner);
    runner.game.drain_effect_queue();

    // The may-attack prompt installs after the suspend.
    let kind = runner
        .pending_kind()
        .expect("a may-attack prompt must install after suspending all opp Digimon");
    assert!(
        matches!(
            kind,
            SelectionKind::Target | SelectionKind::OwnField | SelectionKind::OppField
        ),
        "[When Digivolving] 'this Digimon may attack' installs an attack-style prompt; got {kind:?}"
    );
    assert!(
        runner.pending_is_optional(),
        "'may attack' -> the attack is optional (PASS legal)"
    );
}

/// NEGATIVE: with NO opponent Digimon, the suspend is a no-op and the attack is
/// still offered (this Digimon can attack the opponent's player). The clause
/// must not error or leave a stuck selection requiring an opponent target.
#[test]
fn bt16_101_when_digivolving_with_no_opponent_digimon_no_suspend_panic() {
    let mut runner = rapidmon_runner();
    let _rapid = digivolve_rapidmon_over_plain(&mut runner);
    runner.game.drain_effect_queue();
    let _ = runner.auto_resolve();
    // No panic, no stuck mandatory selection.
    assert!(
        runner.game.pending_selection.is_none(),
        "with no opp Digimon the clause resolves cleanly (suspend no-op, attack declinable)"
    );
}

// ════════════════════════════════════════════════════════════════════════════
// Section 3 — [All Turns] aura: opp suspended Digimon -4000 DP
//   gated on [Rapidmon] / [X Antibody] in the carrier's digivolution sources
// ════════════════════════════════════════════════════════════════════════════

/// POSITIVE (name arm): with a [Rapidmon]-named source, a SUSPENDED opp Digimon
/// gets -4000 DP.
#[test]
fn bt16_101_aura_minus4000_to_suspended_opp_when_rapidmon_source() {
    let mut runner = rapidmon_runner();
    // BT16-101 with a Rapidmon source beneath it.
    let _rapid = runner.place_stack(0, &["RAPID-SRC", CARD_ID]);
    let opp = runner.place_on_field(1, "OPP-A", Some(0)); // base DP 5000
    // Suspend the opponent Digimon (the aura targets only suspended ones).
    runner.game.player_mut(1).battle_area[opp.index as usize].is_suspended = true;

    runner.game.tick_declarative_effects();

    let delta = runner.modifiers().sum(opp, ModifierType::ChangeDp);
    assert_eq!(
        delta, -4000,
        "a suspended opp Digimon must get -4000 DP while [Rapidmon] is in sources"
    );
    assert_eq!(
        runner.effective_dp(opp),
        Some(1000),
        "5000 base - 4000 aura = 1000 effective DP"
    );
}

/// POSITIVE (trait arm): with an [X Antibody]-trait source (NOT named Rapidmon),
/// the aura still applies.
#[test]
fn bt16_101_aura_minus4000_to_suspended_opp_when_x_antibody_source() {
    let mut runner = rapidmon_runner();
    let _rapid = runner.place_stack(0, &["XANTI-SRC", CARD_ID]);
    let opp = runner.place_on_field(1, "OPP-A", Some(0));
    runner.game.player_mut(1).battle_area[opp.index as usize].is_suspended = true;

    runner.game.tick_declarative_effects();

    assert_eq!(
        runner.modifiers().sum(opp, ModifierType::ChangeDp),
        -4000,
        "an [X Antibody]-trait source must also enable the aura"
    );
}

/// The aura targets ONLY SUSPENDED opp Digimon — an UNSUSPENDED opp Digimon is
/// not affected.
#[test]
fn bt16_101_aura_does_not_affect_unsuspended_opp() {
    let mut runner = rapidmon_runner();
    let _rapid = runner.place_stack(0, &["RAPID-SRC", CARD_ID]);
    let opp = runner.place_on_field(1, "OPP-A", Some(0)); // left unsuspended
    runner.game.tick_declarative_effects();

    assert_eq!(
        runner.modifiers().sum(opp, ModifierType::ChangeDp),
        0,
        "an unsuspended opp Digimon must NOT receive the -4000 aura"
    );
}

/// The aura does NOT affect the controller's OWN suspended Digimon.
#[test]
fn bt16_101_aura_does_not_affect_own_suspended_digimon() {
    let mut runner = rapidmon_runner();
    let _rapid = runner.place_stack(0, &["RAPID-SRC", CARD_ID]);
    let own = runner.place_on_field(0, "PLAIN-LV5", Some(0));
    runner.game.player_mut(0).battle_area[own.index as usize].is_suspended = true;
    runner.game.tick_declarative_effects();

    assert_eq!(
        runner.modifiers().sum(own, ModifierType::ChangeDp),
        0,
        "the aura targets the OPPONENT's suspended Digimon, not your own"
    );
}

/// NEGATIVE (aura OFF): with NO [Rapidmon]/[X Antibody] source (plain base), a
/// suspended opp Digimon is NOT affected — the digivolution-source gate is off.
#[test]
fn bt16_101_aura_off_when_no_rapidmon_or_x_antibody_source() {
    let mut runner = rapidmon_runner();
    // BT16-101 atop a plain (non-Rapidmon, non-X-Antibody) base.
    let _rapid = runner.place_stack(0, &["PLAIN-LV5", CARD_ID]);
    let opp = runner.place_on_field(1, "OPP-A", Some(0));
    runner.game.player_mut(1).battle_area[opp.index as usize].is_suspended = true;

    runner.game.tick_declarative_effects();

    assert_eq!(
        runner.modifiers().sum(opp, ModifierType::ChangeDp),
        0,
        "without a [Rapidmon]/[X Antibody] digivolution source the aura must stay OFF"
    );
}

// ════════════════════════════════════════════════════════════════════════════
// Section 4 — [All Turns][OPT] gain 2 memory on battle / 0-DP opp deletion
// ════════════════════════════════════════════════════════════════════════════

/// Put BT16-101 on the field (its own [All Turns] observer active) and seed an
/// opponent Digimon to be deleted.
fn deletion_runner() -> (DebugRunner, PermanentHandle) {
    let mut runner = rapidmon_runner();
    // Memory headroom: the base runner starts at +10 (the max), where a +2 gain
    // clamps to a 0 delta. Reset to 0 so the gains are observable.
    runner.game.set_memory(0);
    let _rapid = runner.place_on_field(0, CARD_ID, Some(0));
    let opp = runner.place_on_field(1, "OPP-A", Some(0));
    (runner, opp)
}

/// POSITIVE (battle death): an opp Digimon deleted IN BATTLE grants +2 memory.
#[test]
fn bt16_101_gains_2_memory_on_opp_battle_deletion() {
    let (mut runner, opp) = deletion_runner();
    let mem_before = runner.memory();

    // A battle deletion of the opponent's Digimon (cause = BattleDeletion).
    runner.game.delete_permanent_with_cause(opp, ReplacementCause::Battle);
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.memory(),
        mem_before + 2,
        "an opp Digimon deleted in battle must grant +2 memory"
    );
}

/// POSITIVE (0-DP death): an opp Digimon driven to <=0 DP and deleted by the
/// state-based rules-check (cause = Rule) grants +2 memory.
#[test]
fn bt16_101_gains_2_memory_on_opp_zero_dp_deletion() {
    let (mut runner, opp) = deletion_runner();
    let mem_before = runner.memory();

    // Drive OPP-A (5000 DP) to -? via a -DP effect, then let the rules-check run.
    let rapid_card = runner.game.player(0).battle_area[0].top_card().handle();
    let rapid_perm = PermanentHandle { player: 0, index: 0 };
    {
        let mut ctx = EffectContext::new(&mut runner.game, rapid_card, Some(rapid_perm), 0);
        ctx.add_dp_modifier(opp, -6000, Expiry::Permanent); // 5000 -> -1000
    }
    assert_eq!(
        runner.game.effective_dp(opp),
        Some(0),
        "precondition: opp Digimon at 0 DP (5000 − 6000, floored at 0 per \
         rules 17-1-3-1 / DCGO Permanent.DP — still the deletable state)"
    );

    // Resolution boundary: the state-based rules-check deletes the <=0 DP Digimon
    // (tagged EventCause::Rule) and the observer fires.
    runner.game.drain_effect_queue();
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.battle_area_size(1),
        0,
        "the <=0-DP opp Digimon must be deleted by the rules-check"
    );
    assert_eq!(
        runner.memory(),
        mem_before + 2,
        "an opp Digimon deleted by having 0 DP must grant +2 memory"
    );
}

/// NEGATIVE (effect death): an opp Digimon deleted BY AN EFFECT must NOT grant
/// memory (DCGO predicate is `!IsByEffect`).
#[test]
fn bt16_101_no_memory_on_opp_effect_deletion() {
    let (mut runner, opp) = deletion_runner();
    let mem_before = runner.memory();

    // P0's effect deletes the opponent's Digimon (cause = OwnEffect / effect).
    let rapid_card = runner.game.player(0).battle_area[0].top_card().handle();
    let rapid_perm = PermanentHandle { player: 0, index: 0 };
    {
        let mut ctx = EffectContext::new(&mut runner.game, rapid_card, Some(rapid_perm), 0);
        ctx.delete_permanent(opp);
    }
    runner.game.drain_effect_queue();
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.battle_area_size(1),
        0,
        "precondition: the opp Digimon was deleted by the effect"
    );
    assert_eq!(
        runner.memory(),
        mem_before,
        "an opp Digimon deleted BY AN EFFECT must NOT grant memory (!IsByEffect)"
    );
}

/// NEGATIVE (own death): deleting YOUR OWN Digimon in battle must NOT grant
/// memory — only the OPPONENT's deletions count.
#[test]
fn bt16_101_no_memory_on_own_battle_deletion() {
    let mut runner = rapidmon_runner();
    let _rapid = runner.place_on_field(0, CARD_ID, Some(0));
    let own = runner.place_on_field(0, "PLAIN-LV5", Some(0));
    let mem_before = runner.memory();

    runner.game.delete_permanent_with_cause(own, ReplacementCause::Battle);
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.memory(),
        mem_before,
        "deleting your OWN Digimon must not grant memory (observer is opponent-only)"
    );
}

/// OPT LOCKOUT: a battle death THEN a 0-DP death in the SAME turn grant only +2
/// total (the single [Once Per Turn] clause must not fire twice → +4).
#[test]
fn bt16_101_opt_caps_gain_at_2_per_turn() {
    let mut runner = rapidmon_runner();
    runner.game.set_memory(0); // headroom for the +2 gain (base runner is at +10 cap)
    let _rapid = runner.place_on_field(0, CARD_ID, Some(0));
    let opp_a = runner.place_on_field(1, "OPP-A", Some(0));
    let opp_b = runner.place_on_field(1, "OPP-B", Some(0));
    let mem_before = runner.memory();

    // First trigger: battle death of OPP-A → +2.
    runner.game.delete_permanent_with_cause(opp_a, ReplacementCause::Battle);
    let _ = runner.auto_resolve();
    assert_eq!(
        runner.memory(),
        mem_before + 2,
        "first opp deletion grants +2"
    );

    // Second trigger same turn: 0-DP death of OPP-B → must NOT add another +2.
    let opp_b_handle = PermanentHandle {
        player: 1,
        // OPP-B is now the only remaining opp permanent (index 0 after OPP-A left).
        index: runner
            .game
            .player(1)
            .battle_area
            .iter()
            .position(|p| p.top_card().card_id(&runner.game.card_data) == "OPP-B")
            .expect("OPP-B still on field") as u8,
    };
    let rapid_card = runner.game.player(0).battle_area[0].top_card().handle();
    let rapid_perm = PermanentHandle { player: 0, index: 0 };
    {
        let mut ctx = EffectContext::new(&mut runner.game, rapid_card, Some(rapid_perm), 0);
        ctx.add_dp_modifier(opp_b_handle, -8000, Expiry::Permanent);
    }
    runner.game.drain_effect_queue();
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.memory(),
        mem_before + 2,
        "OPT lockout: a second opp deletion the same turn must NOT grant another +2 (total stays +2, not +4)"
    );
}
