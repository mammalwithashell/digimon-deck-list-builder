//! AD1-018 LordKnightmon — Digimon, Lv.6, Purple/Black, DP 11000, Cost 11.
//! Traits: [Holy Warrior, Royal Knight]. Attribute: Virus.
//!
//! # Printed card text (dossier — authoritative)
//!
//! When this card would be played, if you have 4 or more cards with [Knightmon]
//!   or [Lucemon] in its text in your trash, reduce the play cost by 5.
//! [On Play] [When Digivolving] Until your opponent's turn ends, their Digimon's
//!   effects don't affect 1 of your Digimon.
//! [All Turns] [Once Per Turn] When any of your Digimon with [Knightmon] or
//!   [Lucemon] in its text are played, <De-Digivolve 2> 1 of your opponent's
//!   Digimon.
//! Inherited [Security]: <De-Digivolve 1> 1 of your opponent's Digimon. Then,
//!   delete 1 of your opponent's Digimon with a play cost of 3 or less.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/AD1/Purple/AD1_018.cs
//!
//! # Source-priority note (cost reduction)
//! The DCGO `Reduce Play Cost` region checks `HasMatchConditionOwnersPermanent`
//! over FIELD Digimon whose TopCard name contains Knightmon/Lucemon — which
//! diverges from BOTH the printed card text (image: "4 or more cards with
//! [Knightmon] or [Lucemon] in its text in your trash") AND its sibling AD1-017
//! (whose DCGO uses `MatchConditionOwnersCardCountInTrash >= 4`). Per CLAUDE.md
//! source priority the printed card face governs WHAT THE CARD SAYS, so the
//! trash-text-count reading is implemented (matching the AD1-017 cost clause).

#![allow(dead_code, unused_imports)]

use digimon_dsl::compiled::{
    CompiledClause, CompiledCountAggregate, CompiledDeclarativeClause, CompiledDpConstraint,
    CompiledPredicate, CompiledScope, CompiledStep, CompiledTiming,
};
use digimon_engine::action::space::{encode_attack, PASS};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, make_test_card_with_level, DebugRunner};
use digimon_engine::enums::{CardKind, EffectTiming, ModifierType};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::{SelectionKind, TriggerSource};

// ─── Fixtures ────────────────────────────────────────────────────────────────

fn ally_digimon(id: &str) -> CardData {
    let mut c = make_test_card_with_level(id, id, 5);
    c.card_kind = CardKind::Digimon;
    c.dp = Some(5000);
    c
}

fn opp_digimon(id: &str, cost: u16) -> CardData {
    let mut c = make_test_card_with_level(id, id, 4);
    c.card_kind = CardKind::Digimon;
    c.dp = Some(4000);
    c.play_cost = cost;
    c
}

fn pred_any<F: Fn(&CompiledPredicate) -> bool + Copy>(p: &CompiledPredicate, f: F) -> bool {
    f(p)
        || p.all_of.iter().any(|q| pred_any(q, f))
        || p.any_of.iter().any(|q| pred_any(q, f))
        || p.none_of.iter().any(|q| pred_any(q, f))
        || p.not.as_ref().map(|q| pred_any(q, f)).unwrap_or(false)
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 0 — structure
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn ad1_018_has_on_play_immunity_and_inherited_security_clause() {
    let runner = DebugRunner::builder()
        .dsl_card("AD1-018")
        .expect("AD1-018 must load from embedded DSL pack")
        .start();
    let card = runner.compiled_card("AD1-018").expect("compiled card");

    assert!(card.effects.iter().any(|clause| matches!(
        clause,
        CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::OnPlay)
    )));
    assert!(card.effects.iter().any(|clause| matches!(
        clause,
        CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::OnSecurity)
    )));
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 1 — Cost reduction: 4+ [Knightmon]/[Lucemon]-text cards in trash → -5
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn ad1_018_cost_reduction_clause_shape() {
    let runner = DebugRunner::builder()
        .dsl_card("AD1-018")
        .expect("AD1-018 loads")
        .start();
    let card = runner.compiled_card("AD1-018").expect("compiled");

    let (when_playing_this, condition, amount) = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Declarative(CompiledDeclarativeClause::CostReduction {
                when_playing_this,
                condition,
                amount,
                ..
            }) => Some((*when_playing_this, condition.clone(), *amount)),
            _ => None,
        })
        .expect("AD1-018 must carry a play-cost reduction clause");

    assert!(when_playing_this, "cost reduction applies to THIS card");
    assert_eq!(amount, Some(5), "reduce the play cost by 5");

    let cond = condition.expect("cost reduction must be conditional on the trash count");
    let agg: &CompiledCountAggregate = cond
        .count_gte
        .as_ref()
        .or_else(|| cond.all_of.iter().find_map(|p| p.count_gte.as_ref()))
        .expect("cost reduction condition must use count_gte over trash");
    assert_eq!(
        agg.n,
        CompiledDpConstraint::Literal(4),
        "count threshold must be 4 ([4 or more])"
    );
    assert!(
        pred_any(&agg.filter, |q| q.effect_text_contains.as_deref() == Some("Knightmon")),
        "count filter must include effect_text_contains: Knightmon"
    );
    assert!(
        pred_any(&agg.filter, |q| {
            q.effect_text_contains.as_deref() == Some("Lucemon")
        }),
        "count filter must include effect_text_contains: Lucemon"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 2 — [On Play][When Digivolving] opponent-effect immunity (behavioral)
// ═══════════════════════════════════════════════════════════════════════════

/// [On Play] picks 1 of your Digimon, which gains immunity to opponent Digimon
/// effects until the opponent's turn ends (CannotBeAffected modifier).
#[test]
fn ad1_018_on_play_grants_one_digimon_opponent_effect_immunity() {
    let mut runner = DebugRunner::builder()
        .dsl_card("AD1-018")
        .expect("AD1-018 loads")
        .add_card(ally_digimon("LK-ALLY"))
        .memory(15)
        .start();

    let ally = runner.place_on_field(0, "LK-ALLY", Some(0));
    let lk = runner.place_on_field(0, "AD1-018", Some(0));

    runner.fire_on_play(0, lk.index as usize);

    let view = runner
        .pending_selection_view()
        .expect("the [On Play] protect-target select must surface");
    assert_eq!(
        view.kind,
        SelectionKind::OwnField,
        "select_own_permanent installs OwnField selection"
    );
    let want = encode_attack(ally.player as u16, ally.index as u16);
    assert!(
        view.valid_action_ids.contains(&want),
        "the ally must be a valid protect target"
    );
    runner.execute_action(0, want).expect("pick the ally");
    let _ = runner.auto_resolve();

    assert!(
        runner.game.modifiers.has(ally, ModifierType::CannotBeAffected),
        "the chosen Digimon must gain opponent-effect immunity (CannotBeAffected)"
    );
}

/// [When Digivolving] fires the same immunity clause.
#[test]
fn ad1_018_when_digivolving_fires_immunity_clause() {
    let mut runner = DebugRunner::builder()
        .dsl_card("AD1-018")
        .expect("AD1-018 loads")
        .add_card(ally_digimon("LK-WD-ALLY"))
        .memory(15)
        .start();

    let _ally = runner.place_on_field(0, "LK-WD-ALLY", Some(0));
    let lk = runner.place_on_field(0, "AD1-018", Some(0));

    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(lk),
    );
    runner.game.drain_effect_queue();

    assert!(
        runner.pending_selection().is_some(),
        "[When Digivolving] must surface the immunity protect-target select"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 3 — Inherited [Security]: De-Digivolve 1 + delete cost-≤3 (behavioral)
// ═══════════════════════════════════════════════════════════════════════════

/// The inherited [Security] clause De-Digivolves 1 opponent Digimon (pops its
/// top source), then deletes 1 opponent Digimon with play cost <= 3.
#[test]
fn ad1_018_inherited_security_de_digivolves_then_deletes_cost_le_3() {
    let mut host_top = make_test_card_with_level("LK-HOST", "Host carrier", 6);
    host_top.card_kind = CardKind::Digimon;
    host_top.dp = Some(11000);

    let mut runner = DebugRunner::builder()
        .dsl_card("AD1-018")
        .expect("AD1-018 loads")
        .add_card(host_top)
        .add_card(opp_digimon("OPP-STACK", 5))
        .add_card(opp_digimon("OPP-BASE", 5))
        .add_card(opp_digimon("OPP-CHEAP", 3))
        .memory(15)
        .start();

    // Security owner p0 carries AD1-018 as a buried source so the INHERITED
    // [Security] clause fires.
    let host = runner.place_on_field(0, "LK-HOST", Some(0));
    runner.push_source(host, "AD1-018");

    // Opponent (p1) Digimon: a stacked one to De-Digivolve, plus a cost-3 one
    // (deletable) and a cost-5 one (not deletable by the cost-≤3 step).
    let stack = runner.place_on_field(1, "OPP-STACK", None);
    runner.push_source(stack, "OPP-BASE"); // give it a digivolution source to pop
    let cheap = runner.place_on_field(1, "OPP-CHEAP", None);
    let stack_sources_before = runner.game.players[1].battle_area[stack.index as usize]
        .card_sources
        .len();
    let opp_before = runner.battle_area_size(1);

    runner
        .game
        .enqueue_triggered(EffectTiming::SecuritySkill, TriggerSource::Permanent(host));
    runner.game.drain_effect_queue();

    // First prompt: De-Digivolve target (OppField, mandatory).
    let v1 = runner
        .pending_selection_view()
        .expect("De-Digivolve target prompt must install");
    assert_eq!(v1.kind, SelectionKind::OppField);
    let pick_stack = encode_attack(0, stack.index as u16);
    runner
        .execute_action(v1.selecting_player, pick_stack)
        .expect("De-Digivolve the stacked opponent Digimon");
    runner.game.drain_effect_queue();

    // Second prompt: delete target (cost <= 3) — only the cheap one qualifies.
    let v2 = runner
        .pending_selection_view()
        .expect("cost-≤3 delete prompt must install");
    assert_eq!(v2.kind, SelectionKind::OppField);
    let pick_cheap = encode_attack(0, cheap.index as u16);
    assert!(
        v2.valid_action_ids.contains(&pick_cheap),
        "the cost-3 opponent Digimon must be a valid delete target"
    );
    runner
        .execute_action(v2.selecting_player, pick_cheap)
        .expect("delete the cost-3 opponent Digimon");
    runner.game.drain_effect_queue();

    // The stacked Digimon lost a source (De-Digivolve 1), and the cheap one
    // was deleted.
    assert_eq!(
        runner.game.players[1].battle_area[stack.index as usize]
            .card_sources
            .len(),
        stack_sources_before - 1,
        "De-Digivolve 1 must pop the top source of the chosen opponent Digimon"
    );
    assert_eq!(
        runner.battle_area_size(1),
        opp_before - 1,
        "the cost-≤3 opponent Digimon must be deleted"
    );
}

/// Structural: the inherited [Security] clause must contain a de_digivolve step
/// (amount 1) and a delete_permanent step gated to play_cost <= 3.
#[test]
fn ad1_018_inherited_security_clause_shape() {
    let runner = DebugRunner::builder()
        .dsl_card("AD1-018")
        .expect("AD1-018 loads")
        .start();
    let card = runner.compiled_card("AD1-018").expect("compiled");

    let clause = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| {
            t.scope == CompiledScope::Inherited && t.when.contains(&CompiledTiming::OnSecurity)
        })
        .expect("AD1-018 must carry an inherited [Security] clause");

    assert!(
        clause
            .process
            .iter()
            .any(|s| matches!(s, CompiledStep::DeDigivolve { amount: Some(1), .. })),
        "inherited [Security] must De-Digivolve 1; got {:?}",
        clause.process
    );
    assert!(
        clause
            .process
            .iter()
            .any(|s| matches!(s, CompiledStep::DeletePermanent { .. })),
        "inherited [Security] must delete a permanent; got {:?}",
        clause.process
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 4 — GAP: [All Turns][OPT] De-Digivolve-2 observer keyed on the PLAYED
//             card's effect TEXT containing [Knightmon]/[Lucemon].
// ═══════════════════════════════════════════════════════════════════════════
//
// "When any of your Digimon with [Knightmon] or [Lucemon] in its text are
// played, <De-Digivolve 2> 1 of your opponent's Digimon." This needs a NEW
// predicate leaf — `event_card_text_contains` — that inspects the TRIGGERING
// (played) card's printed effect text. The DSL today has `event_card_name_
// contains` (AD1-001 / AD1-016 idiom) but no TEXT-contains sibling for the
// event card. Logged as G-DSL-EVENT-CARD-TEXT-CONTAINS. Left stubbed; the
// observer clause is OMITTED from the YAML (an approximation keyed on the
// played card's NAME would under-fire vs the printed "in its text").

#[ignore = "pending: G-DSL-EVENT-CARD-TEXT-CONTAINS — [All Turns][OPT] De-Digivolve-2 observer must key on the PLAYED card's effect text containing [Knightmon]/[Lucemon]"]
#[test]
fn ad1_018_all_turns_de_digivolve_2_observer_on_knightmon_lucemon_text_play() {}
