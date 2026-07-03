//! P-094 Destromon — Digimon, Lv.5, Black, DP 10000, Cost 10.
//! Form: Ultimate. Type/Traits: Unknown. Attribute: Unknown.
//! Evo: Lv.4 Black / Cost 5.
//!
//! # Card text (official Bandai DB — data/card_bundles/P-094.md)
//!
//! ```text
//! [On Play][When Digivolving] Delete up to 3 play cost's total worth of your
//!   opponent's Digimon and Tamers. For each [Vemmon] in this Digimon's
//!   digivolution cards, add 1 to this effect's play cost maximum.
//! DigiXros -1: [Snatchmon] × 1 + [Vemmon] × 4  (materials Digimon-only)
//! [Inherited][Opponent's Turn][Once Per Turn] When one of your opponent's
//!   Digimon attacks, by returning 2 [Vemmon] from any of your [Galacticmon]'s
//!   digivolution cards to the bottom of the deck, change the attack target to
//!   this Digimon.
//! ```
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/P/Black/P_094.cs
//!
//! # Patterns this test covers
//! - Substrate widening: `select_opponent_play_cost_budget.play_cost_budget` is
//!   a FormulaSpec — the budget scales as `3 + 1 per [Vemmon]` in this Digimon's
//!   digivolution cards (mirrors the `dp_budget` formula path). Scalar users
//!   (EX4-073) unaffected.
//! - Substrate widening: `source_count` predicate leaf — "carries ≥N sources
//!   matching a filter" (DCGO `DigivolutionCards.Count(pred) >= N`).
//! - DigiXros alt-path with per-material `kind: digimon` (DCGO IsDigimon gate).
//! - Inherited attack-redirect that returns 2 named sources from a chosen
//!   FOREIGN carrier ([Galacticmon]) to deck bottom, then redirects to self.

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledCardKind, CompiledClause, CompiledPredicate, CompiledScope,
    CompiledStep, CompiledTiming,
};
use digimon_engine::action::space::{encode_attack, PASS};
use digimon_engine::combat::{AttackInitiator, AttackOpen, TargetConstraint};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardKind, EffectTiming};
use digimon_engine::selection::{AttackTarget, TriggerSource};

// ─── Card fixtures ────────────────────────────────────────────────────────────

fn vemmon(id: &str) -> digimon_engine::card_data::CardData {
    let mut c = make_test_card(id, "Vemmon");
    c.card_kind = CardKind::Digimon;
    c.level = Some(3);
    c.dp = Some(3000);
    c
}

fn snatchmon(id: &str) -> digimon_engine::card_data::CardData {
    let mut c = make_test_card(id, "Snatchmon");
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(5000);
    c
}

fn galacticmon(id: &str) -> digimon_engine::card_data::CardData {
    let mut c = make_test_card(id, "Galacticmon");
    c.card_kind = CardKind::Digimon;
    c.level = Some(7);
    c.dp = Some(15000);
    c
}

fn opp_permanent(
    id: &str,
    name: &str,
    kind: CardKind,
    play_cost: u16,
) -> digimon_engine::card_data::CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = kind;
    if kind == CardKind::Digimon {
        c.level = Some(4);
        c.dp = Some(4000);
    }
    c.play_cost = play_cost;
    c
}

fn base_runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("P-094")
        .expect("P-094 in embedded DSL pack")
        .memory(20)
        .start()
}

/// Make it player 1's turn (so `active_when: { opponents_turn: true }` holds for
/// player 0's inherited redirect). `turn_player() == turn_order[turn_player_idx]`.
fn make_it_player_ones_turn(r: &mut DebugRunner) {
    r.game.turn_count = 2;
    let idx = r
        .game
        .turn_order
        .iter()
        .position(|&p| p == 1)
        .expect("player 1 in turn_order");
    r.game.turn_player_idx = idx;
    assert_eq!(r.game.turn_player(), 1, "it must be player 1's turn");
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 1 — Compilation + structural assertions
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn p_094_yaml_compiles_and_has_printed_metadata() {
    let r = base_runner();
    let card = r.compiled_card("P-094").expect("P-094 present in embedded pack");
    assert_eq!(card.name, "Destromon");
    assert_eq!(card.kind, CompiledCardKind::Digimon);
    assert_eq!(card.level, Some(5));
    assert_eq!(card.cost, Some(10));
    assert_eq!(card.dp, Some(10000));
}

/// Two alt-paths: standard Lv.4 Black digivolve + DigiXros. The DigiXros recipe
/// must be Digimon-only per material (DCGO `cardSource.IsDigimon`).
#[test]
fn p_094_has_digivolve_and_digixros_paths_with_digimon_only_materials() {
    let r = base_runner();
    let card = r.compiled_card("P-094").expect("P-094 present");

    assert_eq!(card.alt_paths.len(), 2, "standard digivolve + DigiXros");

    let digixros = card
        .alt_paths
        .iter()
        .find(|p| p.kind == CompiledAltPathKind::DigiXros)
        .expect("a DigiXros alt-path");

    // Snatchmon ×1 slot + Vemmon slot (repeat 4). Every material filter must
    // require kind: digimon — the filter is authored as
    // `all_of: [{ kind: digimon }, { name_is: X }]`, so the kind constraint is
    // nested in `all_of` (DCGO `cardSource.IsDigimon`).
    fn predicate_requires_digimon(p: &CompiledPredicate) -> bool {
        p.kind == Some(CompiledCardKind::Digimon)
            || p.all_of.iter().any(predicate_requires_digimon)
    }
    fn predicate_names(p: &CompiledPredicate) -> Vec<String> {
        let mut out = Vec::new();
        if let Some(n) = &p.name_is {
            out.push(n.clone());
        }
        for sub in p.all_of.iter().chain(p.any_of.iter()) {
            out.extend(predicate_names(sub));
        }
        out
    }
    for material in &digixros.materials {
        assert!(
            predicate_requires_digimon(&material.filter),
            "every DigiXros material must require kind: digimon (DCGO IsDigimon); \
             filter={:?}",
            material.filter
        );
    }
    // A Snatchmon slot and a Vemmon slot exist.
    let has_snatchmon = digixros
        .materials
        .iter()
        .any(|m| predicate_names(&m.filter).iter().any(|n| n == "Snatchmon"));
    let has_vemmon = digixros
        .materials
        .iter()
        .any(|m| predicate_names(&m.filter).iter().any(|n| n == "Vemmon"));
    assert!(has_snatchmon && has_vemmon, "recipe has Snatchmon + Vemmon slots");
}

/// The [On Play][When Digivolving] deletion clause and the inherited attack
/// redirect both compile with the expected scope/timing.
#[test]
fn p_094_has_deletion_clause_and_inherited_redirect() {
    let r = base_runner();
    let card = r.compiled_card("P-094").expect("P-094 present");

    // [On Play][When Digivolving], face-up, carrying a play-cost-budget select.
    let deletion = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t)
                if t.scope == CompiledScope::FaceUp
                    && t.when.contains(&CompiledTiming::OnPlay)
                    && t.when.contains(&CompiledTiming::WhenDigivolving) =>
            {
                Some(t)
            }
            _ => None,
        })
        .expect("[On Play][When Digivolving] deletion clause");
    assert!(
        deletion
            .process
            .iter()
            .any(|s| matches!(s, CompiledStep::SelectOpponentPlayCostBudget { .. })),
        "deletion clause uses select_opponent_play_cost_budget"
    );

    // Inherited [Opponent's Turn] redirect.
    let inherited = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t)
                if t.scope == CompiledScope::Inherited
                    && t.when.contains(&CompiledTiming::OnOpponentAttack) =>
            {
                Some(t)
            }
            _ => None,
        })
        .expect("inherited [Opp Turn] redirect clause");
    assert!(inherited.once_per_turn, "inherited clause is [Once Per Turn]");
    assert!(inherited.optional, "inherited clause is a 'by paying' optional");
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 2 — [On Play][When Digivolving] budget scales by # [Vemmon] sources
// ═══════════════════════════════════════════════════════════════════════════════

/// Baseline: with ZERO [Vemmon] sources, the budget is 3. An opponent permanent
/// with play cost 3 is deletable; one with play cost 4 is NOT (single-card cap).
#[test]
fn p_094_deletion_budget_is_three_with_no_vemmon_sources() {
    let mut r = DebugRunner::builder()
        .dsl_card("P-094")
        .expect("P-094 loads")
        .add_card(opp_permanent("OPP-3", "OppThree", CardKind::Digimon, 3))
        .add_card(opp_permanent("OPP-4", "OppFour", CardKind::Digimon, 4))
        .memory(20)
        .start();

    // Destromon alone (no Vemmon sources) → budget = 3.
    let handle = r.place_on_field(0, "P-094", Some(0));
    r.place_on_field(1, "OPP-3", Some(0));
    r.place_on_field(1, "OPP-4", Some(0));

    r.game
        .enqueue_triggered(EffectTiming::OnPlay, TriggerSource::Permanent(handle));
    r.game.drain_effect_queue();

    let view = r
        .pending_selection_view()
        .expect("budget select installs (OPP-3 fits the base-3 budget)");
    // OPP-3 (field index 0) is selectable; OPP-4 (index 1, cost 4 > 3) is not.
    let opp3 = encode_attack(0, 0);
    let opp4 = encode_attack(0, 1);
    assert!(
        view.valid_action_ids.contains(&opp3),
        "cost-3 opponent Digimon fits the base budget of 3"
    );
    assert!(
        !view.valid_action_ids.contains(&opp4),
        "cost-4 opponent Digimon exceeds the base budget of 3 (no Vemmon sources); valid={:?}",
        view.valid_action_ids
    );
}

/// Scaling: with TWO [Vemmon] sources under Destromon, the budget is 3 + 2 = 5.
/// A cost-5 opponent permanent that was ineligible at budget 3 is now deletable.
#[test]
fn p_094_deletion_budget_scales_with_vemmon_sources() {
    let mut r = DebugRunner::builder()
        .dsl_card("P-094")
        .expect("P-094 loads")
        .add_card(vemmon("VEM-A"))
        .add_card(vemmon("VEM-B"))
        .add_card(opp_permanent("OPP-5", "OppFive", CardKind::Digimon, 5))
        .memory(20)
        .start();

    // Stack: [VEM-A, VEM-B, P-094] → 2 Vemmon sources → budget = 3 + 2 = 5.
    let handle = r.place_stack(0, &["VEM-A", "VEM-B", "P-094"]);
    r.place_on_field(1, "OPP-5", Some(0));

    r.game
        .enqueue_triggered(EffectTiming::OnPlay, TriggerSource::Permanent(handle));
    r.game.drain_effect_queue();

    let view = r
        .pending_selection_view()
        .expect("budget select installs with the scaled budget");
    let opp5 = encode_attack(0, 0);
    assert!(
        view.valid_action_ids.contains(&opp5),
        "cost-5 opponent Digimon fits the scaled budget 3 + (2 Vemmon) = 5; valid={:?}",
        view.valid_action_ids
    );

    // Actually delete it and confirm removal.
    r.game
        .resolve_selection(view.selecting_player, opp5)
        .expect("delete OPP-5");
    while let Some(v) = r.pending_selection_view() {
        let a = if v.valid_action_ids.contains(&PASS) {
            PASS
        } else {
            v.valid_action_ids[0]
        };
        r.game.resolve_selection(v.selecting_player, a).ok();
    }
    let survivors: Vec<String> = r
        .game
        .player(1)
        .battle_area
        .iter()
        .map(|p| p.top_card().card_id(&r.game.card_data).to_string())
        .collect();
    assert!(
        !survivors.contains(&"OPP-5".to_string()),
        "the cost-5 opponent Digimon within the scaled budget must be deleted; survivors={survivors:?}"
    );
}

/// Tamers are eligible targets ("Digimon and Tamers"). A cost-2 opponent Tamer
/// is deletable at the base budget of 3.
#[test]
fn p_094_deletion_targets_tamers_too() {
    let mut r = DebugRunner::builder()
        .dsl_card("P-094")
        .expect("P-094 loads")
        .add_card(opp_permanent("OPP-TAMER", "OppTamer", CardKind::Tamer, 2))
        .memory(20)
        .start();

    let handle = r.place_on_field(0, "P-094", Some(0));
    r.place_on_field(1, "OPP-TAMER", Some(0));

    r.game
        .enqueue_triggered(EffectTiming::OnPlay, TriggerSource::Permanent(handle));
    r.game.drain_effect_queue();

    let view = r
        .pending_selection_view()
        .expect("budget select installs for the opponent Tamer");
    let tamer = encode_attack(0, 0);
    assert!(
        view.valid_action_ids.contains(&tamer),
        "an opponent Tamer within the budget must be a legal deletion target; valid={:?}",
        view.valid_action_ids
    );
}

/// No eligible opponent permanent (nothing on the opponent field) → the clause's
/// outer condition fails and no selection installs.
#[test]
fn p_094_deletion_no_fire_when_opponent_empty() {
    let mut r = base_runner();
    let handle = r.place_on_field(0, "P-094", Some(0));

    r.game
        .enqueue_triggered(EffectTiming::OnPlay, TriggerSource::Permanent(handle));
    r.game.drain_effect_queue();

    assert!(
        r.pending_selection_view().is_none(),
        "no opponent permanent → the deletion clause must not fire"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 3 — Inherited [Opp Turn][OPT] return 2 [Vemmon] → redirect attack
// ═══════════════════════════════════════════════════════════════════════════════

/// The inherited clause: Destromon is a digivolution SOURCE under a carrier
/// Digimon (inherited effects only fire from sources). When an opponent Digimon
/// attacks, the controller returns 2 [Vemmon] from one of their own
/// [Galacticmon]'s digivolution cards to deck bottom, redirecting the attack to
/// the carrier ("this Digimon" = the Digimon Destromon is under).
#[test]
fn p_094_inherited_returns_two_vemmon_and_redirects_attack() {
    let mut carrier = make_test_card("CARRIER", "Carrier");
    carrier.card_kind = CardKind::Digimon;
    carrier.level = Some(6);
    carrier.dp = Some(9000);

    let mut attacker = make_test_card("ATK", "Attacker");
    attacker.card_kind = CardKind::Digimon;
    attacker.dp = Some(12000);

    let mut security = make_test_card("SEC", "Security");
    security.dp = Some(2000);

    let mut r = DebugRunner::builder()
        .dsl_card("P-094")
        .expect("P-094 loads")
        .add_card(carrier)
        .add_card(vemmon("VEM-A"))
        .add_card(vemmon("VEM-B"))
        .add_card(galacticmon("GAL"))
        .add_card(attacker)
        .add_card(security)
        .security(0, &["SEC"])
        .memory(20)
        .start();
    r.game.turn_count = 1;

    // Player 0's carrier Digimon has Destromon as a digivolution source:
    // stack [P-094, CARRIER]. Its inherited [Opp Turn] redirect is active.
    let carrier_perm = r.place_stack(0, &["P-094", "CARRIER"]);
    // A separate own [Galacticmon] carrying 2 [Vemmon] sources: [VEM-A, VEM-B, GAL].
    let gal = r.place_stack(0, &["VEM-A", "VEM-B", "GAL"]);
    let deck_before = r.game.players[0].deck.len();
    let gal_sources_before = r.game.players[0].battle_area[gal.index as usize]
        .card_sources
        .len();

    // Player 1's Digimon attacks player 0 directly.
    let attacker = r.place_on_field(1, "ATK", Some(0));
    let security_before = r.game.players[0].security.len();

    // It is the opponent's (player 1's) turn — the inherited clause is gated by
    // `active_when: { opponents_turn: true }`.
    make_it_player_ones_turn(&mut r);

    r.game.begin_attack_open(AttackOpen {
        attacker,
        initiator: AttackInitiator::Effect {
            source: None,
            optional: false,
        },
        suspend_attacker: false,
        ignore_summoning_sickness: false,
        target_constraint: TargetConstraint::Forced(AttackTarget::Player(0)),
        allow_cancel: false,
        cost_upgrade: None,
    });

    // Optional inherited trigger → accept the outer prompt.
    r.accept_optional_trigger()
        .expect("accept the inherited optional-trigger prompt");

    // Pick 1 of my [Galacticmon] (only GAL qualifies).
    let view = r
        .pending_selection_view()
        .expect("select the [Galacticmon] to return Vemmon from");
    let choose_gal = encode_attack(0, gal.index as u16);
    assert!(
        view.valid_action_ids.contains(&choose_gal),
        "the Galacticmon carrying 2 Vemmon must be a legal pick; valid={:?}",
        view.valid_action_ids
    );
    r.game
        .resolve_selection(view.selecting_player, choose_gal)
        .expect("choose the Galacticmon");

    // Return exactly 2 [Vemmon] sources from that Galacticmon (mandatory cost).
    // Auto-resolve the 2 source picks (and any trailing PASS on the redirect).
    let mut guard = 0;
    while let Some(v) = r.pending_selection_view() {
        guard += 1;
        assert!(guard < 12, "source-return + redirect must terminate");
        let a = v
            .valid_action_ids
            .iter()
            .copied()
            .find(|a| *a != PASS)
            .unwrap_or(PASS);
        r.game.resolve_selection(v.selecting_player, a).ok();
    }

    // The 2 [Vemmon] sources left the Galacticmon's stack. The redirect switched
    // the attack onto the carrier (a 9000-DP Digimon), so the 12000-DP attacker
    // deleted the carrier in battle — shifting battle-area indices. Locate the
    // Galacticmon by name rather than the stale handle.
    let gal_sources_after = r.game.players[0]
        .battle_area
        .iter()
        .find(|p| p.top_card().card_id(&r.game.card_data) == "GAL")
        .map(|p| p.card_sources.len())
        .expect("the Galacticmon is still on the field");
    assert_eq!(
        gal_sources_after,
        gal_sources_before - 2,
        "2 [Vemmon] digivolution sources were returned off the Galacticmon"
    );
    // …and went to the deck (bottom).
    assert_eq!(
        r.game.players[0].deck.len(),
        deck_before + 2,
        "the 2 returned [Vemmon] went to the deck (bottom)"
    );

    // The attack was redirected to the carrier (not the player): the player's
    // security stack was not checked, and the carrier took the redirected battle.
    assert_eq!(
        r.game.players[0].security.len(),
        security_before,
        "redirected attack must not check the player's security"
    );
    assert!(
        r.game.players[0]
            .battle_area
            .iter()
            .all(|p| p.top_card().card_id(&r.game.card_data) != "CARRIER"),
        "the carrier that took the redirected battle should have been deleted \
         (12000-DP attacker vs 9000-DP carrier)"
    );
    let _ = carrier_perm;
}

/// Gate: the inherited clause must NOT fire when no own [Galacticmon] carries ≥2
/// [Vemmon] sources (the `source_count` gate). A Galacticmon with only 1 Vemmon
/// source cannot pay the return-2 cost, so the clause is not offered.
#[test]
fn p_094_inherited_no_fire_without_galacticmon_carrying_two_vemmon() {
    let mut carrier = make_test_card("CARRIER", "Carrier");
    carrier.card_kind = CardKind::Digimon;
    carrier.level = Some(6);
    carrier.dp = Some(9000);

    let mut attacker = make_test_card("ATK", "Attacker");
    attacker.card_kind = CardKind::Digimon;
    attacker.dp = Some(12000);

    let mut security = make_test_card("SEC", "Security");
    security.dp = Some(2000);

    let mut r = DebugRunner::builder()
        .dsl_card("P-094")
        .expect("P-094 loads")
        .add_card(carrier)
        .add_card(vemmon("VEM-A"))
        .add_card(galacticmon("GAL"))
        .add_card(attacker)
        .add_card(security)
        .security(0, &["SEC"])
        .memory(20)
        .start();
    r.game.turn_count = 1;

    // Destromon under a carrier (inherited effect active). The only Galacticmon
    // carries only ONE Vemmon source → cannot pay the return-2 cost → the
    // source_count(≥2) gate fails, so the redirect must not be offered.
    r.place_stack(0, &["P-094", "CARRIER"]);
    let gal = r.place_stack(0, &["VEM-A", "GAL"]);

    let attacker = r.place_on_field(1, "ATK", Some(0));
    // It is player 1's turn — so the ONLY reason the clause is silent is the
    // source_count(≥2 Vemmon) gate, not the opponents_turn gate.
    make_it_player_ones_turn(&mut r);

    r.game.begin_attack_open(AttackOpen {
        attacker,
        initiator: AttackInitiator::Effect {
            source: None,
            optional: false,
        },
        suspend_attacker: false,
        ignore_summoning_sickness: false,
        target_constraint: TargetConstraint::Forced(AttackTarget::Player(0)),
        allow_cancel: false,
        cost_upgrade: None,
    });

    // No inherited redirect should be offered — the source_count gate (≥2 Vemmon)
    // fails, so the clause's outer condition is false and the attack resolves to
    // security instead of installing the redirect's outer optional prompt.
    if let Some(view) = r.pending_selection_view() {
        let choose_gal = encode_attack(0, gal.index as u16);
        assert!(
            !view.valid_action_ids.contains(&choose_gal),
            "with only 1 Vemmon source the inherited redirect must not be offered; valid={:?}",
            view.valid_action_ids
        );
    }

    // Direct-enqueue confirmation: firing OnOpponentAttack over player 0's field
    // installs NO redirect selection (the ≥2-Vemmon source_count gate blocks it).
    r.game.enqueue_triggered(
        EffectTiming::OnOpponentAttack,
        TriggerSource::PlayerBattleArea(0),
    );
    r.game.drain_effect_queue();
    assert!(
        r.game.pending_selection.is_none(),
        "the source_count(≥2 Vemmon) gate must keep the inherited redirect from firing"
    );
}
