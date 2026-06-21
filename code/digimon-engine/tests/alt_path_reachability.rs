//! Data-driven production-reachability guard for trait-gated alt-digivolutions.
//!
//! `digivolve_cost_choice_archetypes.rs` pins a handful of real cards' cost CHOICES by
//! hand. This guard is the exhaustive complement: it sweeps EVERY DSL card in the embedded
//! pack and, for each *simple* trait-gated digivolve alt-path it can model cleanly, proves
//! the route is actually REACHABLE in-engine by a base that carries the gating trait.
//!
//! Why this matters ("respected in game"): alt-paths reach production through the
//! `alt_path_registry` (populated from `dsl_registry::from_embedded_cached()` in
//! `Game::new`), and a trait gate (`from: { trait_has: X }`) is evaluated against the
//! BASE permanent's `CardData.traits`. The cost + existence of the route come from the
//! same embedded pack in tests and production, so the one place a trait alt can silently
//! become unreachable is the gate. This test isolates that surface: it builds a base
//! carrying ONLY the gating trait at an OFF-colour (so no standard colour route also
//! applies — on the DSL path standard routes come only from explicit `color_is`
//! alt-paths), then asserts the digivolve commits (or, if multiple trait routes collide,
//! offers and honors the alt cost). If the gate ever stops matching, the digivolve has no
//! applicable route and this test fails.
//!
//! Pairs with:
//!   - `tests/dsl/trait_parity.rs` — production `CardData` carries every DSL trait (the
//!     data feeding the gates is correct).
//!   - the Python authoring-parity guard (`code/tests/engine/test_alt_path_authoring_parity.py`)
//!     — the YAML gate + cost match the official Bandai DB (the route is authored right).
//!   - `digivolve_cost_choice_archetypes.rs` — the cost CHOICE surfaces on real archetype
//!     cards (including a production-CardData capstone).
//!
//! Coverage is intentionally scoped to *cleanly-modelable* gates (a flat `level_eq +
//! trait_has` predicate, no materials/extra-cost/condition/marker, a literal cost). Gates
//! with extra constraints this fixture can't synthesize a matching base for are skipped
//! and counted (printed to stderr) — they're covered by the authoring-parity guard, and a
//! floor assertion guards against the extractor silently skipping everything.

use std::collections::HashSet;

use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, GamePhase, PlaySource};
use digimon_engine::selection::SelectionKind;
use digimon_dsl::compiled::{
    CompiledAltPathDirection, CompiledAltPathKind, CompiledCardKind, CompiledColor, CompiledCost,
    CompiledPredicate,
};

const ALL_COLORS: [CardColor; 7] = [
    CardColor::Red,
    CardColor::Blue,
    CardColor::Yellow,
    CardColor::Green,
    CardColor::White,
    CardColor::Black,
    CardColor::Purple,
];

fn cc_to_card(c: CompiledColor) -> CardColor {
    match c {
        CompiledColor::Red => CardColor::Red,
        CompiledColor::Blue => CardColor::Blue,
        CompiledColor::Yellow => CardColor::Yellow,
        CompiledColor::Green => CardColor::Green,
        CompiledColor::Black => CardColor::Black,
        CompiledColor::Purple => CardColor::Purple,
        CompiledColor::White => CardColor::White,
    }
}

/// Accept ONLY a flat predicate whose sole discriminating constraints are `level_eq` +
/// `trait_has` (and an optional `kind: Digimon`). Implemented as equality against a
/// `Default` predicate with just those fields set, so any *other* constraint the fixture
/// can't satisfy (colour/name/dp/stack/etc., or `all_of`/`any_of` nesting) cleanly
/// disqualifies the gate — future predicate fields are covered automatically.
fn clean_trait_gate(p: &CompiledPredicate) -> Option<(u8, String)> {
    let level = p.level_eq?;
    let trait_name = p.trait_has.clone()?;
    if !matches!(p.kind, None | Some(CompiledCardKind::Digimon)) {
        return None;
    }
    let mut expected = CompiledPredicate::default();
    expected.level_eq = Some(level);
    expected.trait_has = Some(trait_name.clone());
    expected.kind = p.kind;
    if *p == expected {
        Some((level, trait_name))
    } else {
        None
    }
}

/// A synthetic Lv.`level` Digimon base of one colour carrying exactly `trait_name`.
fn make_base(level: u8, color: CardColor, trait_name: &str) -> CardData {
    let mut c = make_test_card("ALT-REACH-BASE", "AltReachBase");
    c.card_kind = CardKind::Digimon;
    c.level = Some(level);
    c.dp = Some(5000);
    c.colors = vec![color];
    c.traits = vec![trait_name.to_string()];
    c
}

/// Build a runner with the DSL `card_id` in hand atop `base_card` on the field.
fn runner(card_id: &str, base_card: CardData) -> (DebugRunner, usize) {
    let base_id = base_card.card_id.clone();
    let mut r = DebugRunner::builder()
        .dsl_card(card_id)
        .unwrap_or_else(|e| panic!("{card_id} must load from the embedded pack: {e}"))
        .add_card(base_card)
        .hand(0, &[card_id])
        .memory(15)
        .start();
    r.game.turn_count = 1;
    r.game.current_phase = GamePhase::Main;
    let slot = r.place_on_field(0, &base_id, Some(0)).index as usize;
    (r, slot)
}

/// Returns Ok(()) if digivolving `card_id` over a base carrying the gating trait is
/// reachable (commits, or offers+honors the alt cost). Err(reason) if the trait route is
/// unreachable — the bug this guard catches.
fn try_reach(card_id: &str, base: CardData, alt_cost: i32) -> Result<(), String> {
    let (mut r, slot) = runner(card_id, base);
    let on_top = |r: &DebugRunner| {
        r.game.players[0].battle_area[slot]
            .top_card()
            .card_id(&r.game.card_data)
            == card_id
    };
    let proceeded = r.game.digivolve_from_hand(0, 0, slot, PlaySource::ByHand);
    if proceeded {
        return if on_top(&r) {
            Ok(())
        } else {
            Err("digivolve_from_hand returned true but the evolver is not on top".to_string())
        };
    }
    // Not committed immediately: the only legitimate reason at an off-colour base is a
    // cost choice among multiple trait routes. Require it to be an EffectChoice that
    // offers the alt cost, then honor it.
    if let Some(view) = r.pending_selection_view() {
        if view.kind != SelectionKind::EffectChoice {
            return Err(format!(
                "expected a cost-choice EffectChoice, got pending selection {:?}",
                view.kind
            ));
        }
        let choices = view
            .effect_choices
            .clone()
            .ok_or_else(|| "cost choice carried no effect_choices".to_string())?;
        let token = format!("cost {alt_cost}");
        let opt = choices
            .iter()
            .find(|c| c.label.to_lowercase().contains(&token))
            .ok_or_else(|| {
                format!(
                    "cost choice offered but no option for the alt cost {alt_cost}; labels {:?}",
                    choices.iter().map(|c| c.label.clone()).collect::<Vec<_>>()
                )
            })?;
        r.execute_action(0, opt.action_id)
            .map_err(|e| format!("resolving the alt cost {alt_cost} option failed: {e}"))?;
        return if on_top(&r) {
            Ok(())
        } else {
            Err("picked the alt cost option but the evolver is not on top".to_string())
        };
    }
    Err(
        "digivolve did not proceed and no selection is pending — the trait route is \
         UNREACHABLE for a base carrying the gating trait"
            .to_string(),
    )
}

#[test]
fn every_simple_trait_gated_alt_digivolve_is_reachable_in_engine() {
    // Building the embedded registry + per-card games parses + lowers the full pack;
    // run on a large-stack thread so the guard is self-contained (no RUST_MIN_STACK dep).
    std::thread::Builder::new()
        .stack_size(256 * 1024 * 1024)
        .spawn(check_reachability)
        .expect("spawn large-stack thread")
        .join()
        .expect("alt-path reachability guard thread panicked");
}

fn check_reachability() {
    let registry =
        digimon_engine::dsl_registry::from_embedded_cached().expect("embedded cards.pack loads");

    let mut tested = 0usize;
    let mut skipped_complex_gate = 0usize;
    let mut skipped_no_off_colour = 0usize;
    let mut violations: Vec<String> = Vec::new();
    let mut seen: HashSet<(String, u8, String, i32)> = HashSet::new();

    // Deterministic order for stable diagnostics.
    let mut cards: Vec<(&String, &digimon_dsl::compiled::CompiledCard)> = registry.iter().collect();
    cards.sort_by(|a, b| a.0.cmp(b.0));

    for (card_id, compiled) in cards {
        // Colours to AVOID for the base so no standard colour route also applies: the
        // evolver's own colours plus any explicit standard `color_is`/`color_only`
        // digivolve alt-path colour.
        let mut off_limit: HashSet<CardColor> =
            compiled.color.iter().map(|c| cc_to_card(*c)).collect();
        for ap in &compiled.alt_paths {
            if !matches!(ap.kind, CompiledAltPathKind::Digivolve) {
                continue;
            }
            if let Some(from) = ap.from.as_deref() {
                if let Some(c) = from.color_is {
                    off_limit.insert(cc_to_card(c));
                }
                if let Some(cs) = &from.color_only {
                    for c in cs {
                        off_limit.insert(cc_to_card(*c));
                    }
                }
            }
        }
        let off_colour = ALL_COLORS.iter().copied().find(|c| !off_limit.contains(c));

        for ap in &compiled.alt_paths {
            if !matches!(ap.kind, CompiledAltPathKind::Digivolve) {
                continue;
            }
            if ap.direction != CompiledAltPathDirection::From {
                continue;
            }
            // Mirror the production collection filter (collect_dsl_alt_digivolve_routes):
            // these shapes carry extra cost/material/condition gates the synthetic base
            // doesn't satisfy and that production also treats specially.
            if !ap.materials.is_empty()
                || !ap.extra_cost.is_empty()
                || !ap.on_burst_turn_end.is_empty()
                || ap.stacks_unsuspended
                || ap.marker
                || ap.condition.is_some()
            {
                continue;
            }
            let Some(CompiledCost::Literal(cost)) = ap.cost else {
                continue;
            };
            let Some(from) = ap.from.as_deref() else {
                continue;
            };
            let Some((level, trait_name)) = clean_trait_gate(from) else {
                // Only count gates that ARE trait-keyed but carry extra constraints we
                // can't synthesize a base for — not plain colour/name standard routes.
                if from.trait_has.is_some() {
                    skipped_complex_gate += 1;
                }
                continue;
            };
            if !seen.insert((card_id.clone(), level, trait_name.clone(), cost)) {
                continue;
            }
            let Some(off) = off_colour else {
                skipped_no_off_colour += 1;
                continue;
            };
            let base = make_base(level, off, &trait_name);
            match try_reach(card_id, base, cost) {
                Ok(()) => tested += 1,
                Err(e) => violations.push(format!(
                    "  {card_id} [Lv.{level} w/{trait_name} trait -> cost {cost}]: {e}"
                )),
            }
        }
    }

    eprintln!(
        "[alt-path-reachability] exercised {tested} clean trait-gated routes; \
         skipped {skipped_complex_gate} complex-gate, {skipped_no_off_colour} no-off-colour"
    );

    assert!(
        violations.is_empty(),
        "{} trait-gated alt-digivolution route(s) are NOT reachable in-engine by a base \
         carrying the gating trait (the gate evaluated false at an off-colour base, so the \
         route has no applicable digivolve). This is the Ice-Snow class of \"alt path not \
         respected in game\" bug:\n{}",
        violations.len(),
        violations.join("\n"),
    );

    // Tripwire: if the extractor or registry regresses to exercising almost nothing, fail
    // loudly rather than passing vacuously. The clean trait-gated subset is ~100+ routes.
    assert!(
        tested >= 60,
        "expected to exercise >=60 clean trait-gated digivolve routes, only ran {tested} — \
         the extractor or the embedded pack likely regressed"
    );
}
