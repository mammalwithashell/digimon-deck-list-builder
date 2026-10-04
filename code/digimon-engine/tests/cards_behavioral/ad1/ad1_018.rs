//! AD1-018 LordKnightmon — Digimon, Lv.6, Purple/Black, DP 11000, Cost 11.
//! Traits: [Holy Warrior, Royal Knight]. Attribute: Virus.
//!
//! # Printed card text (card image — authoritative)
//!
//! When this card would be played, if you have a Digimon with [Knightmon] or
//!   [Lucemon] in its NAME, reduce the play cost by 5.
//! [On Play] [When Digivolving] Until your opponent's turn ends, their Digimon's
//!   effects don't affect 1 of your Digimon.
//! [All Turns] [Once Per Turn] When any of your Digimon with [Knightmon] or
//!   [Lucemon] in their TEXTS are played, <De-Digivolve 2> 1 of your opponent's
//!   Digimon.
//! Inherited [Security]: <De-Digivolve 1> 1 of your opponent's Digimon. Then,
//!   delete 1 of your opponent's Digimon with a play cost of 3 or less.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/AD1/Purple/AD1_018.cs
//!
//! # Corrected 2026-06-15
//! The cost reduction was previously implemented as "4+ [Knightmon]/[Lucemon]-
//! TEXT cards in TRASH" — an inaccurate cards.json reading that contradicted the
//! printed card face. The image (authoritative) reads "if you have a Digimon
//! with [Knightmon] or [Lucemon] in its NAME" — a field-presence check, matching
//! DCGO's HasMatchConditionOwnersPermanent. The De-Digivolve-2 observer keys on
//! the PLAYED card's TEXT via `event_card_text_contains` (G-DSL-EVENT-CARD-TEXT-
//! CONTAINS, closed 2026-06-15).

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
    f(p) || p.all_of.iter().any(|q| pred_any(q, f))
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
// Section 1 — Cost reduction: own field Digimon NAMED [Knightmon]/[Lucemon] → -5
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn ad1_018_cost_reduction_requires_knightmon_lucemon_named_field_digimon() {
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

    let cond = condition.expect("cost reduction must be conditional on a field presence");
    let agg: &CompiledCountAggregate = cond
        .count_gte
        .as_ref()
        .or_else(|| cond.all_of.iter().find_map(|p| p.count_gte.as_ref()))
        .expect("cost reduction condition must use count_gte over the field");
    assert_eq!(
        agg.n,
        CompiledDpConstraint::Literal(1),
        "presence check: >=1 named Digimon in play (card face: 'if you have a Digimon')"
    );
    assert!(
        pred_any(&agg.filter, |q| q.name_contains.as_deref()
            == Some("Knightmon")),
        "count filter must match a field Digimon NAMED Knightmon"
    );
    assert!(
        pred_any(&agg.filter, |q| q.name_contains.as_deref()
            == Some("Lucemon")),
        "count filter must match a field Digimon NAMED Lucemon"
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
        runner
            .game
            .modifiers
            .has(ally, ModifierType::CannotBeAffected),
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

    runner
        .game
        .enqueue_triggered(EffectTiming::WhenDigivolving, TriggerSource::Permanent(lk));
    runner.game.drain_effect_queue();

    assert!(
        runner.pending_selection().is_some(),
        "[When Digivolving] must surface the immunity protect-target select"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 3 — [Security]: De-Digivolve 1 + delete cost-≤3 (behavioral)
// ═══════════════════════════════════════════════════════════════════════════

/// The [Security] clause De-Digivolves 1 opponent Digimon (pops its top
/// source), then deletes 1 opponent Digimon with play cost <= 3. Driven through
/// the real security check: AD1-018 is P1's top security card and P0 attacks.
#[test]
fn ad1_018_security_de_digivolves_then_deletes_cost_le_3() {
    let mut attacker_card = make_test_card_with_level("LK-ATK", "Attacker", 6);
    attacker_card.card_kind = CardKind::Digimon;
    attacker_card.dp = Some(20000);
    attacker_card.play_cost = 8;

    let mut runner = DebugRunner::builder()
        .dsl_card("AD1-018")
        .expect("AD1-018 loads")
        .add_card(attacker_card)
        .add_card(opp_digimon("OPP-STACK", 5))
        .add_card(opp_digimon("OPP-BASE", 5))
        .add_card(opp_digimon("OPP-CHEAP", 3))
        .security(1, &["AD1-018"])
        .memory(15)
        .start();

    // The security owner (P1)'s opponent is P0: the attacker, a stacked
    // Digimon to De-Digivolve, plus a cost-3 one (deletable by the cost-≤3 step).
    let attacker = runner.place_on_field(0, "LK-ATK", Some(0));
    let stack = runner.place_on_field(0, "OPP-STACK", None);
    runner.push_source(stack, "OPP-BASE"); // give it a digivolution source to pop
    let cheap = runner.place_on_field(0, "OPP-CHEAP", None);
    let stack_sources_before = runner.game.players[0].battle_area[stack.index as usize]
        .card_sources
        .len();
    let opp_before = runner.battle_area_size(0);

    runner.attack_player(attacker, 1, false);

    // First prompt: De-Digivolve target (OppField, mandatory).
    let v1 = runner
        .pending_selection_view()
        .expect("De-Digivolve target prompt must install");
    assert_eq!(v1.kind, SelectionKind::OppField);
    assert_eq!(v1.selecting_player, 1, "the security owner chooses");
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
    // was deleted (the 20000-DP attacker survives the security battle).
    assert_eq!(
        runner.game.players[0].battle_area[stack.index as usize]
            .card_sources
            .len(),
        stack_sources_before - 1,
        "De-Digivolve 1 must pop the top source of the chosen opponent Digimon"
    );
    assert_eq!(
        runner.battle_area_size(0),
        opp_before - 1,
        "the cost-≤3 opponent Digimon must be deleted"
    );
}

/// Structural: the [Security] clause (default face-up scope) must contain a
/// de_digivolve step (amount 1) and a delete_permanent step gated to
/// play_cost <= 3.
#[test]
fn ad1_018_security_clause_shape() {
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
        .find(|t| t.when.contains(&CompiledTiming::OnSecurity))
        .expect("AD1-018 must carry a [Security] clause");
    assert_eq!(
        clause.scope,
        CompiledScope::FaceUp,
        "[Security] clause must use the default face-up scope"
    );

    assert!(
        clause.process.iter().any(|s| matches!(
            s,
            CompiledStep::DeDigivolve {
                amount: Some(1),
                ..
            }
        )),
        "[Security] must De-Digivolve 1; got {:?}",
        clause.process
    );
    assert!(
        clause
            .process
            .iter()
            .any(|s| matches!(s, CompiledStep::DeletePermanent { .. })),
        "[Security] must delete a permanent; got {:?}",
        clause.process
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 4 — [All Turns][Once Per Turn] De-Digivolve-2 observer keyed on the
//             PLAYED card's printed TEXT containing [Knightmon]/[Lucemon].
//             G-DSL-EVENT-CARD-TEXT-CONTAINS (closed 2026-06-15).
// ═══════════════════════════════════════════════════════════════════════════

/// An own Digimon fixture whose printed effect text carries `text`.
fn ally_with_text(id: &str, text: &str) -> CardData {
    let mut c = make_test_card_with_level(id, id, 4);
    c.card_kind = CardKind::Digimon;
    c.dp = Some(3000);
    c.play_cost = 0;
    c.effect_text = text.to_string();
    c
}

/// Structural: the observer clause must key on `event_card_text_contains` for
/// BOTH Knightmon and Lucemon (not the played card's name) and De-Digivolve 2.
#[test]
fn ad1_018_dedigivolve_observer_keys_on_played_card_text() {
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
        .find(|t| t.when.contains(&CompiledTiming::OnAnyDigimonPlayed))
        .expect("AD1-018 must carry an OnAnyDigimonPlayed De-Digivolve observer");

    let cond = clause
        .condition
        .as_ref()
        .expect("observer must be gated on the played card's text");
    assert!(
        pred_any(cond, |q| q.event_card_text_contains.as_deref()
            == Some("Knightmon")),
        "observer must gate on event_card_text_contains: Knightmon"
    );
    assert!(
        pred_any(cond, |q| q.event_card_text_contains.as_deref()
            == Some("Lucemon")),
        "observer must gate on event_card_text_contains: Lucemon"
    );
    assert!(
        clause.process.iter().any(|s| matches!(
            s,
            CompiledStep::DeDigivolve {
                amount: Some(2),
                ..
            }
        )),
        "observer body must De-Digivolve 2; got {:?}",
        clause.process
    );
}

/// Behavioral: playing an own Digimon whose TEXT contains "Lucemon" fires the
/// observer and De-Digivolves 2 sources off a chosen opponent Digimon.
#[test]
fn ad1_018_dedigivolve_fires_when_played_ally_has_lucemon_in_text() {
    let mut runner = DebugRunner::builder()
        .dsl_card("AD1-018")
        .expect("AD1-018 loads")
        .add_card(ally_with_text(
            "LUCE-ALLY",
            "When this Digimon attacks, gain 1 memory. [Lucemon] only.",
        ))
        .add_card(opp_digimon("OPP-TOP", 5))
        .add_card(opp_digimon("OPP-SRC-A", 5))
        .add_card(opp_digimon("OPP-SRC-B", 5))
        .hand(0, &["LUCE-ALLY"])
        .memory(20)
        .start();
    runner.game.turn_count = 5;

    let _lk = runner.place_on_field(0, "AD1-018", Some(0));
    let opp = runner.place_on_field(1, "OPP-TOP", None);
    runner.push_source(opp, "OPP-SRC-A");
    runner.push_source(opp, "OPP-SRC-B");
    let sources_before = runner.game.players[1].battle_area[opp.index as usize]
        .card_sources
        .len();

    runner.play(0, 0).expect("play the Lucemon-text ally");

    let view = runner
        .pending_selection_view()
        .expect("De-Digivolve-2 observer must surface an opponent target prompt");
    assert_eq!(view.kind, SelectionKind::OppField);
    let pick = encode_attack(0, opp.index as u16);
    assert!(
        view.valid_action_ids.contains(&pick),
        "the opponent Digimon must be a valid De-Digivolve target"
    );
    runner
        .execute_action(view.selecting_player, pick)
        .expect("De-Digivolve it");
    runner.game.drain_effect_queue();

    assert_eq!(
        runner.game.players[1].battle_area[opp.index as usize]
            .card_sources
            .len(),
        sources_before - 2,
        "De-Digivolve 2 must remove 2 digivolution sources"
    );
}

/// Negative: playing an own Digimon whose text contains NEITHER name does NOT
/// fire the observer (it must key on text, not merely on a Digimon being played).
#[test]
fn ad1_018_dedigivolve_does_not_fire_for_unrelated_text() {
    let mut runner = DebugRunner::builder()
        .dsl_card("AD1-018")
        .expect("AD1-018 loads")
        .add_card(ally_with_text(
            "PLAIN-ALLY",
            "When this Digimon attacks, draw 1.",
        ))
        .add_card(opp_digimon("OPP-TOP2", 5))
        .add_card(opp_digimon("OPP-SRC2", 5))
        .hand(0, &["PLAIN-ALLY"])
        .memory(20)
        .start();
    runner.game.turn_count = 5;

    let _lk = runner.place_on_field(0, "AD1-018", Some(0));
    let opp = runner.place_on_field(1, "OPP-TOP2", None);
    runner.push_source(opp, "OPP-SRC2");

    runner.play(0, 0).expect("play the unrelated-text ally");

    assert!(
        runner.pending_selection().is_none(),
        "observer must NOT fire when the played card's text lacks Knightmon/Lucemon"
    );
}
