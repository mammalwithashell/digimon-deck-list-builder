//! BG Imperial (Imperialdramon) — archetype interaction tests.
//!
//! Model: `qa/archetype-qa/bg-imperial-model.md`. Each `#[test]` maps 1:1 to a
//! named combo there. These exercise the **real** archetype cards working as a
//! *system* — the DNA digivolution line (ExVeemon + Stingmon → Paildramon →
//! Imperialdramon) and the colour/cost gates that define the deck — which the
//! per-card `cards_behavioral/` tests (synthetic neutral materials) can't see.
//!
//! The three things this file pins, in the user's words:
//!   1. **Paildramon effects work** — the DNA-conditional clauses on ST9-05 and
//!      BT16-025 fire on the DNA path and *not* on a regular digivolve.
//!   2. **Effect descriptions bubble to the UI** — the `PendingSelection.prompt`
//!      at each Paildramon/Imperialdramon choice carries the card's descriptive
//!      YAML text (serialized to the UI by `digimon-engine-py` `lib.rs:804/1129`,
//!      rendered by `action/explain.rs`).
//!   3. **Evolution costs & colour requirements** — the compiled `alt_paths`
//!      enforce the Blue Lv.4 + Green Lv.4 DNA materials at cost 0 and the
//!      standard-digivolve costs.
//!
//! Sources (CLAUDE.md priority: `general_rule.pdf` + DCGO C# > card-text JSON):
//!   - Card text: `data/cards.json` (ST9-05, BT16-025, ST9-06, EX1-014, ST9-09).
//!   - DCGO C#: `$BASE_DCGO/Assets/Scripts/CardEffect/{ST9,BT16}/Blue/...cs`.
//!   - Rules: DNA digivolution + suspend/unsuspend (`general_rule.pdf` §6/§16).
//!   - YAML: `code/digimon-engine/cards/{st9,bt16}/...yaml`.
//!
//! Real archetype pieces used (all implemented — verified via combo-presence
//! gate, `qa/qa-reports/archetype_interactions.json`):
//!   - EX1-014 ExVeemon (Blue Lv.4)  — the blue DNA leg.
//!   - ST9-09  Stingmon (Green Lv.4) — the green DNA leg.
//!   - ST9-05  Paildramon (Blue+Green Lv.5) — DNA bounce payoff.
//!   - BT16-025 Paildramon (Blue/Green Lv.5) — meta Paildramon (DNA lockdown).
//!   - ST9-06  Imperialdramon Dragon Mode (Blue+Green Lv.6) — colour source replay.

#![allow(dead_code)]

use digimon_dsl::compiled::{CompiledAltPathKind, CompiledColor, CompiledCost, CompiledMaterial};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{
    CardColor, CardKind, EffectTiming, GamePhase, ModifierType, PlaySource,
};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::{SelectionKind, TriggerSource};

// ─── Combo-piece fixtures ────────────────────────────────────────────────────

/// An opponent Digimon (no digivolution sources) with a chosen DP. Used both as
/// a bounce target (Combo 1) and a suspend/lock target (Combo 2).
fn make_opp_digimon(id: &str, name: &str, dp: i32) -> CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(dp);
    c
}

/// A Lv.5 Blue digimon that is a *legal regular-digivolve base* into a Lv.5
/// Paildramon? No — Paildramon is Lv.5, so the regular-digivolve base must be a
/// Lv.4. This builds that base with a permissive evo-cost so a regular
/// digivolve into the Paildramon succeeds. Blue so it satisfies ST9-05's
/// `from: { level_eq: 4, color_is: blue }` standard alt-path.
fn make_lv4_blue_base(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.colors = vec![CardColor::Blue];
    c.dp = Some(4000);
    c
}

/// Build a fresh runner with the named DSL cards loaded and `result` in P0's
/// hand at index 0, ready to drive a DNA digivolve. The two real DNA legs
/// (EX1-014 blue, ST9-09 green) and any extra cards are loaded too.
fn dna_runner(result_id: &str, extra: &[CardData]) -> DebugRunner {
    let mut b = DebugRunner::builder()
        .dsl_card(result_id)
        .unwrap_or_else(|e| panic!("{result_id} must be in embedded DSL pack: {e}"))
        .dsl_card("EX1-014")
        .expect("EX1-014 ExVeemon (blue Lv.4) must be in embedded DSL pack")
        .dsl_card("ST9-09")
        .expect("ST9-09 Stingmon (green Lv.4) must be in embedded DSL pack");
    for c in extra {
        b = b.add_card(c.clone());
    }
    b.hand(0, &[result_id]).memory(10).start()
}

/// Drive the DNA digivolve of the hand card at index 0 into the Paildramon,
/// selecting EX1-014 (field slot 0) then ST9-09 (field slot 1) as the two
/// materials — the proven per-card sequence (`st9_05.rs`). Materials must
/// already be on P0's field at slots 0 and 1.
fn drive_dna_into_paildramon(runner: &mut DebugRunner) {
    assert!(
        runner.game.initiate_dna_digivolve(0, 0),
        "initiate_dna_digivolve must succeed with a blue Lv.4 + green Lv.4 on field"
    );
    assert_eq!(
        runner.pending_kind(),
        Some(SelectionKind::Material),
        "DNA digivolve must prompt for the first material"
    );
    runner
        .execute_action(0, 0)
        .expect("select first DNA material (EX1-014 ExVeemon, slot 0)");
    assert_eq!(
        runner.pending_kind(),
        Some(SelectionKind::Material),
        "DNA digivolve must prompt for the second material"
    );
    runner
        .execute_action(0, 1)
        .expect("select second DNA material (ST9-09 Stingmon, slot 1)");
}

// ═══════════════════════════════════════════════════════════════════════════════
// Combo 1 — "DNA Bounce" (ST9-05 Paildramon)
//
// EX1-014 ExVeemon (Blue Lv.4) + ST9-09 Stingmon (Green Lv.4) DNA digivolve into
// ST9-05 Paildramon. ST9-05: "[When Digivolving] When DNA digivolving, return 1
// of your opponent's Digimon with 6000 DP or less to the bottom of its owner's
// deck." The clause is gated on the DNA path — the system fact a per-card test
// (synthetic materials, no real ExVeemon/Stingmon) can't express.
//
// Sources: cards.json ST9-05; DCGO ST9_05.cs (`IsJogress` + `PutLibraryBottom`,
// `DP<=6000`); YAML `when: on_dna_digivolve` (st9/ST9-05.yaml).
// ═══════════════════════════════════════════════════════════════════════════════

/// Happy path: the real ExVeemon + Stingmon DNA digivolve satisfies Paildramon's
/// blue+green colour requirement, and the DNA-only bounce returns the opponent's
/// 5000-DP Digimon (≤6000) to the bottom of its deck.
#[test]
fn dna_bounce_real_exveemon_stingmon_returns_low_dp_opponent() {
    let mut runner = dna_runner("ST9-05", &[make_opp_digimon("OPP-LOW", "OppLow", 5000)]);
    runner.game.turn_count = 1;
    runner.game.current_phase = GamePhase::Main;

    // The two real DNA legs on P0's field (slots 0, 1) and the opp target.
    runner.place_on_field(0, "EX1-014", Some(0));
    runner.place_on_field(0, "ST9-09", Some(0));
    runner.place_on_field(1, "OPP-LOW", Some(0));

    let before = snapshot_min(&runner);
    drive_dna_into_paildramon(&mut runner);

    // ST9-05 is the only permanent on P0's field after the two legs merged.
    assert_eq!(
        runner.game.players[0].battle_area.len(),
        1,
        "after DNA digivolve, ST9-05 Paildramon is the lone P0 permanent (both legs merged in)"
    );

    // The DNA-only [When Digivolving] clause installs an OppField selection.
    assert_eq!(
        runner.pending_kind(),
        Some(SelectionKind::OppField),
        "on_dna_digivolve must install the bounce selection for the DP<=6000 opp Digimon"
    );

    // ── User concern #2: the description bubbles to the UI ──
    let prompt = runner
        .pending_selection_view()
        .expect("a selection must be pending")
        .prompt;
    assert!(
        prompt.contains("6000") && prompt.to_lowercase().contains("bottom"),
        "the UI prompt must describe the bounce (got: {prompt:?})"
    );

    // Resolve: pick the only opp target.
    let action = first_valid_action(&runner);
    runner
        .execute_action(0, action)
        .expect("resolve the bounce");

    let after = snapshot_min(&runner);
    assert_eq!(
        after.opp_field,
        before.opp_field - 1,
        "the 5000-DP opp Digimon must leave the field (returned to deck)"
    );
    assert!(
        after.opp_deck > before.opp_deck,
        "the bounced Digimon must land at the bottom of the opp deck"
    );
    assert!(
        runner.pending_selection().is_none(),
        "no further selection should be pending after the bounce resolves"
    );
}

/// Unhappy / system-fork path: reaching the SAME ST9-05 Paildramon via a
/// *regular* digivolve (a single Blue Lv.4 base, not the ExVeemon+Stingmon DNA)
/// must NOT fire the DNA-only bounce — the opponent's low-DP Digimon survives.
#[test]
fn regular_digivolve_into_paildramon_does_not_fire_dna_bounce() {
    let mut runner = DebugRunner::builder()
        .dsl_card("ST9-05")
        .expect("ST9-05 in pack")
        .add_card(make_lv4_blue_base("BLUE-BASE"))
        .add_card(make_opp_digimon("OPP-LOW", "OppLow", 5000))
        .hand(0, &["ST9-05"])
        .memory(10)
        .start();
    runner.game.turn_count = 1;
    runner.game.current_phase = GamePhase::Main;

    runner.place_on_field(0, "BLUE-BASE", Some(0));
    runner.place_on_field(1, "OPP-LOW", Some(0));

    let before = snapshot_min(&runner);
    // Regular digivolve: P0 hand index 0 (ST9-05) onto field slot 0, pay from hand.
    assert!(
        runner.game.digivolve_from_hand(0, 0, 0, PlaySource::ByHand),
        "regular digivolve into ST9-05 must succeed from a Lv.4 blue base"
    );
    let after = snapshot_min(&runner);

    assert!(
        runner.pending_selection().is_none(),
        "the DNA-only clause must NOT install a bounce on a regular (non-DNA) digivolve"
    );
    assert_eq!(
        after.opp_field, before.opp_field,
        "the opp Digimon must survive — no bounce fires off a regular digivolve"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Combo 2 — "DNA Lockdown" (BT16-025 meta Paildramon)
//
// EX1-014 + ST9-09 DNA digivolve into BT16-025 Paildramon. BT16-025:
// "[When Digivolving] Suspend all of your opponent's Digimon with as many or
// fewer digivolution cards as this Digimon. Then, if DNA digivolving, none of
// their Digimon can unsuspend until the end of their turn."
//
// The when_digivolving suspend fires on ANY digivolve; the CannotUnsuspend lock
// fires ONLY on the DNA path (YAML `when: on_dna_digivolve`). System fork.
//
// Sources: cards.json BT16-025; DCGO BT16_025.cs (`IsJogress` →
// `GainCanNotUnsuspendPlayerEffect`); YAML bt16/BT16-025.yaml.
// ═══════════════════════════════════════════════════════════════════════════════

/// Happy path: the DNA digivolve suspends the opp Digimon AND applies the
/// `CannotUnsuspend` lock (DP/source-count under the threshold).
#[test]
fn dna_lockdown_suspends_and_locks_opponent() {
    let mut runner = dna_runner("BT16-025", &[make_opp_digimon("OPP", "Opp", 3000)]);
    runner.game.turn_count = 1;
    runner.game.current_phase = GamePhase::Main;

    runner.place_on_field(0, "EX1-014", Some(0));
    runner.place_on_field(0, "ST9-09", Some(0));
    let opp = runner.place_on_field(1, "OPP", Some(0));

    drive_dna_into_paildramon(&mut runner);
    let _ = runner.auto_resolve();

    // when_digivolving: opp Digimon (0 sources ≤ Paildramon's 2 sources) suspended.
    assert!(
        opp_is_suspended(&runner, opp),
        "[When Digivolving] must suspend the opp Digimon with ≤ digivolution cards"
    );
    // on_dna_digivolve: the DNA-only lock is applied.
    assert!(
        runner.modifiers().has(opp, ModifierType::CannotUnsuspend),
        "the DNA path must apply CannotUnsuspend to the opp Digimon"
    );
}

/// Unhappy / system-fork path: a *regular* digivolve into BT16-025 still
/// suspends the opp Digimon (when_digivolving fires on any digivolve) but does
/// NOT apply the DNA-only `CannotUnsuspend` lock.
#[test]
fn regular_digivolve_into_meta_paildramon_suspends_but_does_not_lock() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT16-025")
        .expect("BT16-025 in pack")
        .add_card(make_lv4_blue_base("BLUE-BASE"))
        .add_card(make_opp_digimon("OPP", "Opp", 3000))
        .hand(0, &["BT16-025"])
        .memory(10)
        .start();
    runner.game.turn_count = 1;
    runner.game.current_phase = GamePhase::Main;

    runner.place_on_field(0, "BLUE-BASE", Some(0));
    let opp = runner.place_on_field(1, "OPP", Some(0));

    assert!(
        runner.game.digivolve_from_hand(0, 0, 0, PlaySource::ByHand),
        "regular digivolve into BT16-025 must succeed"
    );
    let _ = runner.auto_resolve();

    assert!(
        opp_is_suspended(&runner, opp),
        "[When Digivolving] suspend fires on a regular digivolve too"
    );
    assert!(
        !runner.modifiers().has(opp, ModifierType::CannotUnsuspend),
        "the CannotUnsuspend lock must NOT apply off a regular (non-DNA) digivolve"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Combo 3 — "Colour-Gated Source Replay" (ST9-06 Imperialdramon Dragon Mode)
//
// ST9-06: "[When Digivolving] You may play 1 level 4 or lower blue Digimon card
// and 1 level 4 or lower green Digimon card from this Digimon's digivolution
// cards without paying their memory costs." The replay is gated on the *colours*
// present in the stack — the archetype reason ExVeemon (blue) + Stingmon (green)
// sit under the Imperialdramon line.
//
// Sources: cards.json ST9-06; DCGO ST9_06.cs; YAML st9/ST9-06.yaml.
// ═══════════════════════════════════════════════════════════════════════════════

/// Fire the [When Digivolving] timing for `handle` directly (the proven harness
/// pattern from `rocks.rs` — `place_stack` builds the stack without firing the
/// trigger, so we enqueue + drain it to model the digivolve).
fn fire_when_digivolving(runner: &mut DebugRunner, handle: PermanentHandle) {
    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(handle),
    );
    runner.game.drain_effect_queue();
}

/// Happy path: a stack carrying BOTH a blue Lv.4 source (ExVeemon) and a green
/// Lv.4 source (Stingmon) lets ST9-06 replay both — P0's field gains 2 Digimon.
#[test]
fn colour_source_replay_with_blue_and_green_sources_replays_both() {
    // Stack [ExVeemon(blue Lv4), Stingmon(green Lv4), Paildramon(ST9-05 Lv5), ST9-06].
    // Sources under ST9-06 = the lower three; only the two Lv.4 colour legs qualify.
    let mut runner = DebugRunner::builder()
        .dsl_card("ST9-06")
        .expect("ST9-06 in pack")
        .dsl_card("ST9-05")
        .expect("ST9-05 in pack")
        .dsl_card("EX1-014")
        .expect("EX1-014 in pack")
        .dsl_card("ST9-09")
        .expect("ST9-09 in pack")
        .memory(10)
        .start();
    runner.game.turn_count = 1;
    runner.game.current_phase = GamePhase::Main;

    let dm = runner.place_stack(0, &["EX1-014", "ST9-09", "ST9-05", "ST9-06"]);
    let field_before = runner.battle_area_size(0);

    fire_when_digivolving(&mut runner, dm);

    // The clause is "you may" — accept the outer optional if one is parked.
    accept_if_optional(&mut runner);

    // ── User concern #2: the first source-replay prompt describes the blue pick ──
    if let Some(view) = runner.pending_selection_view() {
        assert!(
            view.prompt.to_lowercase().contains("blue")
                || view.prompt.to_lowercase().contains("green"),
            "the source-replay prompt must describe which colour to play (got: {:?})",
            view.prompt
        );
    }

    // Drive the two colour picks (blue source, then green source).
    select_each_offered_source(&mut runner, 2);

    let field_after = runner.battle_area_size(0);
    // The DM stays on field; the blue + green sources re-enter as 2 new permanents.
    assert_eq!(
        field_after,
        field_before + 2,
        "ST9-06 must replay BOTH the blue and green Lv.4 sources (field +2)"
    );
}

/// Unhappy / colour-gated path: a stack with a blue Lv.4 source but NO green
/// source lets ST9-06 replay only the blue one — P0's field gains exactly 1.
#[test]
fn colour_source_replay_without_green_source_replays_only_blue() {
    // Stack [ExVeemon(blue Lv4), Paildramon(ST9-05 Lv5), ST9-06]. No green Lv.4 source.
    let mut runner = DebugRunner::builder()
        .dsl_card("ST9-06")
        .expect("ST9-06 in pack")
        .dsl_card("ST9-05")
        .expect("ST9-05 in pack")
        .dsl_card("EX1-014")
        .expect("EX1-014 in pack")
        .memory(10)
        .start();
    runner.game.turn_count = 1;
    runner.game.current_phase = GamePhase::Main;

    let dm = runner.place_stack(0, &["EX1-014", "ST9-05", "ST9-06"]);
    let field_before = runner.battle_area_size(0);

    fire_when_digivolving(&mut runner, dm);
    accept_if_optional(&mut runner);
    select_each_offered_source(&mut runner, 2);

    let field_after = runner.battle_area_size(0);
    assert_eq!(
        field_after,
        field_before + 1,
        "with no green Lv.4 source, ST9-06 replays ONLY the blue source (field +1)"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Combo 4 — "Evolution-cost & colour gate" (compiled alt-paths)
//
// The archetype's defining requirement: Paildramon is reached by DNA digivolving
// a Blue Lv.4 + a Green Lv.4 (cost 0), or by a standard digivolve from a Lv.4
// (cost 4); Imperialdramon Dragon Mode standard-digivolves from a Blue Lv.5
// (cost 4). These are pinned from the compiled `alt_paths`.
//
// Source: cards.json evo_costs / dna_costs / xros_req; YAML alt_paths.
// ═══════════════════════════════════════════════════════════════════════════════

/// ST9-05 Paildramon: the DNA alt-path requires a Blue Lv.4 + a Green Lv.4
/// material at cost 0, and the standard digivolve is from a Lv.4 at cost 4.
#[test]
fn paildramon_st9_05_dna_alt_path_requires_blue_lv4_and_green_lv4_at_cost_0() {
    let runner = DebugRunner::builder()
        .dsl_card("ST9-05")
        .expect("ST9-05 in pack")
        .memory(0)
        .start();
    let compiled = runner.compiled_card("ST9-05").expect("ST9-05 compiled");

    let dna = compiled
        .alt_paths
        .iter()
        .find(|p| p.kind == CompiledAltPathKind::DnaDigivolve)
        .expect("ST9-05 must have a DNA digivolve alt-path");

    assert_eq!(dna.materials.len(), 2, "DNA needs exactly 2 materials");
    // Colour requirement: one Blue Lv.4, one Green Lv.4.
    assert!(
        dna.materials
            .iter()
            .any(|m| material_is(m, CompiledColor::Blue, 4)),
        "one DNA material must be a Blue Lv.4"
    );
    assert!(
        dna.materials
            .iter()
            .any(|m| material_is(m, CompiledColor::Green, 4)),
        "one DNA material must be a Green Lv.4"
    );
    assert_eq!(
        dna.cost,
        Some(CompiledCost::Literal(0)),
        "DNA digivolve into Paildramon costs 0"
    );

    let std = compiled
        .alt_paths
        .iter()
        .find(|p| p.kind == CompiledAltPathKind::Digivolve)
        .expect("ST9-05 must have a standard digivolve alt-path");
    assert_eq!(
        std.cost,
        Some(CompiledCost::Literal(4)),
        "standard digivolve into Paildramon costs 4"
    );
}

/// ST9-06 Imperialdramon Dragon Mode standard-digivolves from a Blue Lv.5 at
/// cost 4 — the colour/level/cost gate up the Imperialdramon line.
#[test]
fn imperialdramon_dm_st9_06_standard_digivolve_is_blue_lv5_cost_4() {
    let runner = DebugRunner::builder()
        .dsl_card("ST9-06")
        .expect("ST9-06 in pack")
        .memory(0)
        .start();
    let compiled = runner.compiled_card("ST9-06").expect("ST9-06 compiled");

    let std = compiled
        .alt_paths
        .iter()
        .find(|p| p.kind == CompiledAltPathKind::Digivolve)
        .expect("ST9-06 must have a standard digivolve alt-path");
    let from = std
        .from
        .as_ref()
        .expect("ST9-06 standard digivolve must specify a `from` predicate");
    assert_eq!(
        from.level_eq,
        Some(5),
        "Imperialdramon Dragon Mode digivolves from a Lv.5"
    );
    assert_eq!(
        from.color_is,
        Some(CompiledColor::Blue),
        "Imperialdramon Dragon Mode digivolves from a Blue source"
    );
    assert_eq!(
        std.cost,
        Some(CompiledCost::Literal(4)),
        "Imperialdramon Dragon Mode standard digivolve costs 4"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Combo 6 — "Digivolution colour gate" (ExVeemon / Stingmon / Wormmon)
//
// A standard digivolve evo-cost is a (level, colour) pair: the engine permits the
// digivolution only when the base Digimon's colour list CONTAINS the evo-cost
// colour AND its level matches — `Game::can_digivolve` (game.rs:3390) →
// `matching_evo_cost`. This answers "can Wormmon digivolve over a blue egg?" and
// pins the colour requirement up both DNA legs.
//
// HARNESS NOTE: the embedded DSL pack builds CardData from the YAML only, and
// `card_data_from_compiled` sets `evo_costs: Vec::new()` (debug_runner.rs:1133) —
// so a real DSL-loaded card carries NO printed digivolution cost unless the YAML
// declares a `digivolve` alt-path. The Imperialdramon line's printed evo-costs
// live in cards.json (loaded in production), not the YAML. So we exercise the
// engine's colour/level gate with synthetic cards that ENCODE the real printed
// evo-costs (cited per card), mirroring the per-card-test convention
// (`st9_05.rs::make_lv4_regular_base` sets `evo_costs` the same way). This is the
// faithful way to test the *rule* the question is about; it is NOT an engine bug.
//
// Real printed evo-costs mirrored below (data/cards.json `evo_costs`). The
// "dual colour" cards show TWO digivolve circles on the printed face (the
// authoritative source); the digimoncard.io ingest dropped the off-colour
// circle for several of them — fixed 2026-06-15 (see the
// `imperial_line_cards_carry_both_digivolve_colours_in_cards_json` regression
// test below; only BT12-022/BT12-050 had been patched before then):
//   BT12-002 DemiVeemon  Lv.2 Blue   (egg — base only)
//   BT12-021 Veemon      Lv.3 Blue   ← Blue Lv.2
//   EX1-014  ExVeemon    Lv.4 Blue   ← Blue Lv.3 OR Green Lv.3 (dual colour — face shows BOTH circles)
//   BT12-022 ExVeemon    Lv.4 Blue   ← Blue Lv.3 OR Green Lv.3 (dual colour)
//   BT12-047 Wormmon     Lv.3 Green  ← Green Lv.2
//   ST9-09   Stingmon    Lv.4 Green  ← Green Lv.3 OR Blue Lv.3 (dual colour — face shows BOTH circles)
//   BT12-050 Stingmon    Lv.4 Green  ← Green Lv.3 OR Blue Lv.3 (dual colour)
//
// Evo-cost colour codes (action/mask.rs::evo_color): Red=0, Blue=1, Green=3.
// ═══════════════════════════════════════════════════════════════════════════════

const EVO_BLUE: u8 = 1;
const EVO_GREEN: u8 = 3;
// For the BT12-043 ShineGreymon cross-archetype regression below (same
// API-drop bug class; codes per action/mask.rs::evo_color).
const EVO_RED: u8 = 0;
const EVO_YELLOW: u8 = 2;

/// A digivolve BASE (egg / lower Digimon): a Digimon of a given level + single
/// colour. No evo-cost needed — only the digivolving (top) card's evo-cost is
/// consulted against the base's level+colour.
fn base_digimon(id: &str, level: u8, color: CardColor) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(level);
    c.colors = vec![color];
    c.dp = Some(1000);
    c
}

/// A digivolving (TOP) card carrying explicit printed evo-costs — each entry a
/// `(colour_code, base_level)` pair mirroring the real card's `cards.json`
/// evo_costs (the harness can't load those, see HARNESS NOTE above).
fn evolver(id: &str, name: &str, level: u8, color: CardColor, evo: &[(u8, u8)]) -> CardData {
    use digimon_engine::card_data::EvoCost;
    let mut c = make_test_card(id, name);
    c.card_kind = CardKind::Digimon;
    c.level = Some(level);
    c.colors = vec![color];
    c.dp = Some(3000);
    c.evo_costs = evo
        .iter()
        .map(|&(card_color, level)| EvoCost {
            card_color,
            level,
            memory_cost: 0,
        })
        .collect();
    c
}

/// Does the engine's colour/level evo gate permit `top` to digivolve over `base`?
/// Uses `Game::can_digivolve` directly — isolating the (level, colour) rule from
/// phase/memory/route concerns.
fn colour_gate_allows(top: CardData, base: CardData) -> bool {
    let top_id = top.card_id.clone();
    let base_id = base.card_id.clone();
    let mut r = DebugRunner::builder()
        .add_card(top)
        .add_card(base)
        .hand(0, &[top_id.as_str()])
        .memory(10)
        .start();
    r.place_on_field(0, &base_id, Some(0));
    let card = r.game.players[0].hand[0].clone();
    let perm = &r.game.players[0].battle_area[0];
    r.game.can_digivolve(&card, perm)
}

/// Veemon (Blue, evo Blue Lv.2) CAN digivolve over the blue DemiVeemon egg.
#[test]
fn veemon_digivolves_over_blue_egg() {
    let veemon = evolver("BT12-021", "Veemon", 3, CardColor::Blue, &[(EVO_BLUE, 2)]);
    let blue_egg = base_digimon("BT12-002", 2, CardColor::Blue); // DemiVeemon
    assert!(
        colour_gate_allows(veemon, blue_egg),
        "Veemon (evo Blue Lv.2) must digivolve over the blue DemiVeemon egg"
    );
}

/// THE QUESTION: Wormmon (Green, evo Green Lv.2) CANNOT digivolve over a blue
/// egg — Green is not present on the blue-only DemiVeemon, so the gate rejects
/// it even though the level (2) matches.
#[test]
fn wormmon_cannot_digivolve_over_blue_egg() {
    let wormmon = evolver(
        "BT12-047",
        "Wormmon",
        3,
        CardColor::Green,
        &[(EVO_GREEN, 2)],
    );
    let blue_egg = base_digimon("BT12-002", 2, CardColor::Blue); // DemiVeemon
    assert!(
        !colour_gate_allows(wormmon, blue_egg),
        "Wormmon (evo Green Lv.2) must NOT digivolve over a blue-only egg — colour gate"
    );
}

/// …and the SAME Wormmon CAN digivolve over a GREEN Lv.2 — proving the rejection
/// above is the colour, not the level. (The deck supplies this via a green egg /
/// hard-play, not the blue DemiVeemon.)
#[test]
fn wormmon_digivolves_over_green_egg() {
    let wormmon = evolver(
        "BT12-047",
        "Wormmon",
        3,
        CardColor::Green,
        &[(EVO_GREEN, 2)],
    );
    let green_egg = base_digimon("GREEN-EGG", 2, CardColor::Green);
    assert!(
        colour_gate_allows(wormmon, green_egg),
        "Wormmon (evo Green Lv.2) must digivolve over a green Lv.2 egg"
    );
}

/// EX1-014 ExVeemon's printed face shows TWO digivolve circles — Blue Lv.3 AND
/// Green Lv.3 (cost 2 each). So it digivolves from EITHER a blue Lv.3 (Veemon)
/// OR a green Lv.3 (Wormmon) — the blue↔green flexibility that lets the deck
/// build the ExVeemon DNA leg off a shared green base. (cards.json carried only
/// the Blue path until the 2026-06-15 evo-colour data fix; the synthetic evolver
/// here mirrors the corrected dual evo_costs, and
/// `imperial_line_cards_carry_both_digivolve_colours_in_cards_json` pins the
/// real data.)
#[test]
fn dual_colour_exveemon_ex1_014_takes_blue_or_green_lv3() {
    let exv = || {
        evolver(
            "EX1-014",
            "ExVeemon",
            4,
            CardColor::Blue,
            &[(EVO_BLUE, 3), (EVO_GREEN, 3)],
        )
    };
    assert!(
        colour_gate_allows(exv(), base_digimon("VEEMON-B3", 3, CardColor::Blue)),
        "EX1-014 ExVeemon must digivolve from a blue Lv.3"
    );
    assert!(
        colour_gate_allows(exv(), base_digimon("WORMMON-G3", 3, CardColor::Green)),
        "EX1-014 ExVeemon must ALSO digivolve from a green Lv.3 — its face shows a Green Lv.3 circle"
    );
}

/// ST9-09 Stingmon's printed face shows TWO digivolve circles — Green Lv.3 AND
/// Blue Lv.3 (cost 2 each). So a GREEN Stingmon digivolves over a BLUE Lv.3
/// (Veemon) — the exact "green card over a blue card" the deck relies on.
/// (cards.json carried only the Green path until the 2026-06-15 data fix.)
#[test]
fn dual_colour_stingmon_st9_09_takes_green_or_blue_lv3() {
    let sting = || {
        evolver(
            "ST9-09",
            "Stingmon",
            4,
            CardColor::Green,
            &[(EVO_GREEN, 3), (EVO_BLUE, 3)],
        )
    };
    assert!(
        colour_gate_allows(sting(), base_digimon("WORMMON-G3", 3, CardColor::Green)),
        "ST9-09 Stingmon must digivolve from a green Lv.3"
    );
    assert!(
        colour_gate_allows(sting(), base_digimon("VEEMON-B3", 3, CardColor::Blue)),
        "ST9-09 Stingmon must ALSO digivolve from a blue Lv.3 — green-over-blue; its face shows a Blue Lv.3 circle"
    );
}

/// The dual-colour champions accept EITHER colour Lv.3: BT12-022 ExVeemon (Blue
/// OR Green) takes a green Lv.3, and BT12-050 Stingmon (Green OR Blue) takes a
/// blue Lv.3 — the flexibility that lets the deck build both DNA legs off a
/// shared-colour Lv.3 base.
#[test]
fn dual_colour_exveemon_and_stingmon_accept_the_off_colour_lv3() {
    assert!(
        colour_gate_allows(
            evolver(
                "BT12-022",
                "ExVeemon",
                4,
                CardColor::Blue,
                &[(EVO_BLUE, 3), (EVO_GREEN, 3)]
            ),
            base_digimon("WORMMON-G3", 3, CardColor::Green)
        ),
        "BT12-022 ExVeemon (evo Blue OR Green Lv.3) must accept a green Lv.3"
    );
    assert!(
        colour_gate_allows(
            evolver(
                "BT12-050",
                "Stingmon",
                4,
                CardColor::Green,
                &[(EVO_GREEN, 3), (EVO_BLUE, 3)]
            ),
            base_digimon("VEEMON-B3", 3, CardColor::Blue)
        ),
        "BT12-050 Stingmon (evo Green OR Blue Lv.3) must accept a blue Lv.3"
    );
}

/// BT12-043 ShineGreymon (DATA SQUAD, Yellow/Red Lv.6) — same API-drop bug
/// class as the Imperial line, caught 2026-07-02: its printed face shows TWO
/// digivolve circles (Yellow Lv.5 AND Red Lv.5, cost 4 each) but cards.json
/// carried only the Yellow one. Pinned here because this file is the canonical
/// dual-circle regression home; the synthetic evolver mirrors the corrected
/// evo_costs and the Combo-6b data test below pins the real data.
#[test]
fn dual_colour_shinegreymon_bt12_043_takes_yellow_or_red_lv5() {
    let shine = || {
        evolver(
            "BT12-043",
            "ShineGreymon",
            6,
            CardColor::Yellow,
            &[(EVO_YELLOW, 5), (EVO_RED, 5)],
        )
    };
    assert!(
        colour_gate_allows(shine(), base_digimon("RIZE-Y5", 5, CardColor::Yellow)),
        "BT12-043 ShineGreymon must digivolve from a yellow Lv.5"
    );
    assert!(
        colour_gate_allows(shine(), base_digimon("RIZE-R5", 5, CardColor::Red)),
        "BT12-043 ShineGreymon must ALSO digivolve from a red Lv.5 — its face shows a Red Lv.5 circle"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Combo 6b — "Real-data digivolve-colour regression" (user-reported 2026-06-15)
//
// The synthetic gate tests above prove the engine's colour/level rule. THIS test
// proves the production card DATA is correct: every blue+green Imperialdramon-line
// card whose printed face shows TWO digivolve circles must carry BOTH colours in
// `data/cards.json`. The digimoncard.io ingest dropped the off-colour circle on
// import; only BT12-022/BT12-050 had been patched (via card_overrides.json), so
// the rest were single-colour and the engine's gate (correctly) rejected the legal
// off-colour digivolve (e.g. green ST9-09 Stingmon over a blue Lv.3 Veemon).
//
// Expected circles are read off the authoritative card images (printed text >
// cards.json). Colour codes (action/mask.rs::evo_color): Blue=1, Green=3.
// Mirrors the per-card precedent `bt12_022_data_cards_json_has_blue_and_green_evo_paths`.
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn imperial_line_cards_carry_both_digivolve_colours_in_cards_json() {
    use digimon_engine::card_data::CardData;
    use std::path::PathBuf;

    // CARGO_MANIFEST_DIR = code/digimon-engine; data/cards.json is two up.
    let cards_json = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("data")
        .join("cards.json");
    if !cards_json.exists() {
        eprintln!(
            "Skipping: data/cards.json not found at {} — only runs in a full repo checkout",
            cards_json.display()
        );
        return;
    }
    let cards = CardData::load_from_file(&cards_json).expect("load data/cards.json");

    // (card_id, [(colour_code, level, memory_cost), ...]) — the FULL set of
    // printed digivolve circles per card, verified against the card image.
    let expected: &[(&str, &[(u8, u8, u16)])] = &[
        ("EX1-014", &[(1, 3, 2), (3, 3, 2)]), // ExVeemon: Blue L3 + Green L3
        ("ST9-09", &[(3, 3, 2), (1, 3, 2)]),  // Stingmon: Green L3 + Blue L3
        ("ST9-05", &[(1, 4, 4), (3, 4, 4)]),  // Paildramon: Blue L4 + Green L4
        ("ST9-06", &[(1, 5, 4), (3, 5, 4)]),  // Imperialdramon DM: Blue L5 + Green L5
        ("BT12-028", &[(1, 4, 4), (3, 4, 4)]), // Paildramon: Blue L4 + Green L4
        ("BT12-030", &[(1, 5, 4), (3, 5, 4)]), // Imperialdramon DM: Blue L5 + Green L5
        ("BT12-031", &[(1, 5, 5), (3, 5, 5)]), // Imperialdramon FM: Blue L5 + Green L5 (cost 5)
        // Cross-archetype pin (DATA SQUAD): same dual-circle API-drop class,
        // caught + fixed 2026-07-02. Yellow=2, Red=0.
        ("BT12-043", &[(2, 5, 4), (0, 5, 4)]), // ShineGreymon: Yellow L5 + Red L5
    ];

    for (id, circles) in expected {
        let card = cards
            .get(*id)
            .unwrap_or_else(|| panic!("{id} must be in data/cards.json"));
        for &(colour, level, cost) in *circles {
            assert!(
                card.evo_costs.iter().any(|ec| ec.card_color == colour
                    && ec.level == level
                    && ec.memory_cost == cost),
                "{id} must carry digivolve circle (colour={colour}, Lv.{level}, cost {cost}) — \
                 printed on the card face; got evo_costs={:?}",
                card.evo_costs
            );
        }
    }
}

// ─── Local helpers ───────────────────────────────────────────────────────────

/// Does a compiled DNA material's filter pin exactly this colour + level?
fn material_is(m: &CompiledMaterial, color: CompiledColor, level: u8) -> bool {
    m.filter.color_is == Some(color) && m.filter.level_eq == Some(level)
}

/// A minimal opponent-zone snapshot (this file only needs opp field/deck deltas).
#[derive(Clone, Copy)]
struct MinSnap {
    opp_field: usize,
    opp_deck: usize,
}

fn snapshot_min(r: &DebugRunner) -> MinSnap {
    MinSnap {
        opp_field: r.battle_area_size(1),
        opp_deck: r.deck_size(1),
    }
}

/// Is the opponent permanent at `handle` currently suspended?
fn opp_is_suspended(r: &DebugRunner, handle: PermanentHandle) -> bool {
    r.game.players[handle.player as usize]
        .battle_area
        .get(handle.index as usize)
        .map(|p| p.is_suspended)
        .unwrap_or(false)
}

/// The first valid action id of the pending selection (panics if none pending).
fn first_valid_action(r: &DebugRunner) -> u16 {
    *r.pending_selection()
        .expect("a selection must be pending")
        .valid_action_ids
        .first()
        .expect("the pending selection must offer at least one action")
}

/// If an outer optional-trigger prompt (`Replacement` accept/decline) is parked,
/// accept it so the effect body runs. No-op otherwise.
fn accept_if_optional(r: &mut DebugRunner) {
    if r.pending_kind() == Some(SelectionKind::Replacement) {
        let _ = r.accept_optional_trigger();
    }
}

/// Drive up to `max` source-replay picks: for each parked source selection, pick
/// its first non-PASS action (the source). Stops when no selectable source
/// remains (a colour with no candidate parks no mandatory pick / parks PASS).
fn select_each_offered_source(r: &mut DebugRunner, max: usize) {
    use digimon_engine::action::space::PASS;
    for _ in 0..max {
        let Some(sel) = r.pending_selection() else {
            break;
        };
        let player = sel.selecting_player;
        // Prefer a real source pick over PASS; fall back to whatever is offered.
        let action = sel
            .valid_action_ids
            .iter()
            .copied()
            .find(|&a| a != PASS)
            .or_else(|| sel.valid_action_ids.first().copied());
        let Some(action) = action else { break };
        let _ = r.execute_action(player, action);
    }
}
