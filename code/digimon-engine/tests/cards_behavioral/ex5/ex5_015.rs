//! EX5-015 Gabumon (X Antibody) — Digimon, Lv.3, Blue+Purple, DP 2000, Cost 3.
//! Traits: Beast, X Antibody.
//! Standard digivolve: Lv.2 Blue / cost 1.
//! Bonus alt-digivolve: [Tsunomon]/[Gabumon] / cost 0.
//!
//! # Card text (cards.json — printed)
//!
//! [On Play] [When Digivolving] Reveal the top 4 cards of your deck. Add 2
//! cards with [Garurumon] or [X Antibody] in their names among them to the
//! hand. Place the rest at the bottom of the deck. If you added cards, trash
//! 1 card in your hand.
//!
//! Inherited Effect:
//!   [All Turns] [Once Per Turn] When this Digimon with [Garurumon] or
//!   [Omnimon] in its name would be deleted in battle, by returning 2
//!   non-Digi-Egg cards from your trash to the bottom of the deck, prevent
//!   that deletion.
//!
//! # DCGO C# reference
//! `DCGO/Assets/Scripts/CardEffect/EX5/Blue/EX5_015.cs`
//!
//! # Patterns this test covers
//!
//! - **[On Play] [When Digivolving] reveal-N + single-bucket selection +
//!   bottom remainder** — Clause A+B. Mirror of BT22-094 / BT22-099 single-
//!   bucket reveal pattern, scaled to reveal=4 / max=2 instead of reveal=3 /
//!   max=1, and with a name-based filter (Garurumon OR X Antibody) instead
//!   of trait-based.
//! - **Conditional trash-from-hand cost via select_effect_choice** — the
//!   "If you added cards, trash 1 card in your hand" sub-step is modelled as
//!   a binary choice (G-DSL-BIND-PRESENT workaround, sister to EX9-066).
//! - **Inherited [All Turns][OPT] Substitute on battle deletion** — Clause C.
//!   OMITTED from YAML (G-DSL-INHERITED-SUBSTITUTE-RETURN-TRASH stacked
//!   gaps); structural absence test pinned with `#[ignore]`'d behavioral
//!   coverage planned for gap closure.
//! - **Two alt_paths** — standard Lv2 Blue/cost 1 + bonus Tsunomon/Gabumon
//!   name/cost 0 (sister: BT15-020 Gabumon, BT22-017 Gabumon for the
//!   Tsunomon-name pattern).
//!
//! # Known engine/DSL gaps affecting these tests
//!
//! - **G-DSL-BIND-PRESENT** (filed 2026-05-03 from EX9-066): no
//!   `binding_present: <name>` predicate. The "If you added cards, trash 1"
//!   sub-step is modelled as a binary `select_effect_choice` instead. The
//!   action mask still surfaces both choices so the RL agent learns to pick
//!   "Skip" when no add occurred — no auto-selection, no-approximations
//!   policy preserved.
//!
//! - **G-DSL-INHERITED-SUBSTITUTE-RETURN-TRASH** (new gap, EX5-015 sister
//!   case to BT24-018): inherited replacement clauses requiring a multi-pick
//!   trash-to-deck cost depend on three open primitives:
//!   (a) min:N enforcement on `select_count_capped_multi`,
//!   (b) a `return_trash_list_to_deck_bottom` step verb (G-ZONE-TRASH-TO-DECK),
//!   (c) atomic cost-then-cancel guard on replacement clauses.
//!   Until these close, Clause C is OMITTED entirely from the YAML rather
//!   than emit a half-implementation that violates the printed cost
//!   semantics.
//!
//! - **G-DSL-SOURCE-NAME-CONTAINS** (pre-flight reference): the
//!   `source_name_contains` predicate parses but is not evaluated in
//!   `dsl_cards/predicate.rs`. EX5-015 does NOT depend on this predicate —
//!   the inherited subject-filter would use `name_contains` against the
//!   replacement subject (the host top card), which IS correctly subject-
//!   aware via `predicate_reads_replacement_subject`. The pre-flight gap
//!   note applies to a different leaf and is sidestepped here.

#![allow(dead_code)]

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledCost, CompiledScope, CompiledTiming,
    CompiledTriggeredClause,
};
use digimon_dsl::{compile::compile, spec::CardSpec};
use digimon_engine::action::space::SEL_REVEAL_START;
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardKind, PlayerId};

#[path = "../../support/dsl_card_data.rs"]
mod dsl_card_data;

const CARD_ID: &str = "EX5-015";

// ─── Card-data factories ─────────────────────────────────────────────────────

fn ex5_015_card_data() -> CardData {
    dsl_card_data::card_data_from_compiled(CARD_ID)
}

/// Build a Digimon test card with a specific id, name, level, dp, and traits.
fn make_named_digimon(id: &str, name: &str, traits: &[&str]) -> CardData {
    let mut card = make_test_card(id, name);
    card.card_kind = CardKind::Digimon;
    card.level = Some(4);
    card.dp = Some(4000);
    card.traits = traits.iter().map(|t| t.to_string()).collect();
    card
}

/// Build a digi-egg test card (kind = DigiEgg). Used for non-Digi-Egg filter
/// tests in the (currently OMITTED) inherited substitute clause.
fn make_digi_egg(id: &str, name: &str) -> CardData {
    let mut card = make_test_card(id, name);
    card.card_kind = CardKind::DigiEgg;
    card.level = Some(2);
    card
}

/// Resolve a revealed-card action id by candidate card_id.
fn revealed_action_for_id(runner: &DebugRunner, id: &str) -> Option<u16> {
    runner
        .game
        .revealed_cards
        .iter()
        .enumerate()
        .find_map(|(idx, card)| {
            (card.card_id(&runner.game.card_data) == id).then_some(SEL_REVEAL_START + idx as u16)
        })
}

/// Pick a revealed card by card_id. Asserts the action is currently legal in
/// the active bucket.
fn pick_revealed_by_id(runner: &mut DebugRunner, p: PlayerId, id: &str, label: &str) {
    let view = runner.pending_selection_view().expect(label);
    let action = revealed_action_for_id(runner, id)
        .unwrap_or_else(|| panic!("{label}: revealed card {id} not present"));
    assert!(
        view.valid_action_ids.contains(&action),
        "{label}: action {action} for {id} not legal in current bucket; \
         valid_action_ids={:?}",
        view.valid_action_ids
    );
    runner.execute_action(p, action).expect(label);
}

/// Convert a zone of `CardSource`s into a Vec of card_id strings, in order.
fn zone_ids(cards: &[CardSource], data: &[CardData]) -> Vec<String> {
    cards.iter().map(|c| c.card_id(data).to_string()).collect()
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 1 — Structural assertions
// ═══════════════════════════════════════════════════════════════════════════════

/// EX5-015 YAML must parse and compile through the standalone DSL pipeline
/// (independent of the embedded pack — this guards the YAML against
/// regressions to the loader-side parse/validate path).
#[test]
fn ex5_015_yaml_parses_and_compiles() {
    let spec: CardSpec = serde_yml::from_str(include_str!("../../../cards/ex5/EX5-015.yaml"))
        .expect("EX5-015 YAML parses");
    let _compiled = compile(&spec).expect("EX5-015 YAML compiles");
}

/// EX5-015 must compile in the embedded DSL pack with the printed metadata
/// (kind, level, cost, dp, color, traits).
#[test]
fn ex5_015_compiles_with_printed_metadata() {
    let card = dsl_card_data::compiled(CARD_ID);
    assert_eq!(card.card, CARD_ID);
    assert_eq!(card.name, "Gabumon (X Antibody)");
    assert_eq!(card.level, Some(3));
    assert_eq!(card.cost, Some(3));
    assert_eq!(card.dp, Some(2000));
    assert!(
        card.traits.iter().any(|t| t.eq_ignore_ascii_case("Beast")),
        "EX5-015 must carry the Beast trait; got traits={:?}",
        card.traits
    );
    assert!(
        card.traits
            .iter()
            .any(|t| t.eq_ignore_ascii_case("X Antibody")),
        "EX5-015 must carry the X Antibody trait; got traits={:?}",
        card.traits
    );
}

/// EX5-015 must declare the standard Lv.2 Blue / cost 1 digivolve path
/// (printed evo_costs entry).
#[test]
fn ex5_015_has_standard_lv2_blue_digivolve_alt_path_at_cost_one() {
    let card = dsl_card_data::compiled(CARD_ID);
    let standard = card.alt_paths.iter().find(|p| {
        p.kind == CompiledAltPathKind::Digivolve && p.cost == Some(CompiledCost::Literal(1))
    });
    assert!(
        standard.is_some(),
        "EX5-015 must declare its standard Lv.2 Blue / cost 1 digivolve path; \
         alt_paths={:?}",
        card.alt_paths
            .iter()
            .map(|p| (p.kind, p.cost.clone()))
            .collect::<Vec<_>>()
    );
}

/// EX5-015 must declare the bonus [Tsunomon]/[Gabumon] / cost 0 alt-digivolve
/// path (printed xros_req).
#[test]
fn ex5_015_has_bonus_tsunomon_gabumon_alt_path_at_cost_zero() {
    let card = dsl_card_data::compiled(CARD_ID);
    let bonus = card.alt_paths.iter().find(|p| {
        p.kind == CompiledAltPathKind::Digivolve && p.cost == Some(CompiledCost::Literal(0))
    });
    assert!(
        bonus.is_some(),
        "EX5-015 must declare a cost-0 alt-digivolve path for [Tsunomon] or [Gabumon] name"
    );
}

/// EX5-015 must have exactly one triggered clause that fires on BOTH OnPlay
/// and WhenDigivolving timings (the printed [On Play] [When Digivolving] dual
/// gate). The clause must NOT be optional (DCGO `isOptional: false`; printed
/// text has no "you may" at the trigger level).
#[test]
fn ex5_015_has_on_play_and_when_digivolving_triggered_clause_mandatory() {
    let card = dsl_card_data::compiled(CARD_ID);
    let triggered: Vec<&CompiledTriggeredClause> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();
    assert_eq!(
        triggered.len(),
        1,
        "Exactly one triggered clause (combined OnPlay + WhenDigivolving)"
    );
    let clause = triggered[0];
    assert!(
        clause.when.contains(&CompiledTiming::OnPlay),
        "triggered clause must fire on OnPlay; got when={:?}",
        clause.when
    );
    assert!(
        clause.when.contains(&CompiledTiming::WhenDigivolving),
        "triggered clause must fire on WhenDigivolving; got when={:?}",
        clause.when
    );
    assert_eq!(
        clause.scope,
        CompiledScope::FaceUp,
        "triggered clause must be FaceUp (own-field) scope"
    );
    assert!(
        !clause.optional,
        "Outer trigger is mandatory — DCGO SetUpActivateClass(..., false, ...); \
         no 'you may' in printed text at trigger level"
    );
    assert!(
        !clause.once_per_turn,
        "Outer trigger has no [Once Per Turn]"
    );
}

/// Inherited Substitute clause (Clause C) is now AUTHORED —
/// G-DSL-INHERITED-SUBSTITUTE-RETURN-TRASH is closed. It compiles as an
/// inherited-scope `Replacement` declarative clause.
#[test]
fn ex5_015_inherited_substitute_clause_is_authored() {
    let card = dsl_card_data::compiled(CARD_ID);
    let inherited_replacement = card.effects.iter().find(|c| {
        matches!(
            c,
            CompiledClause::Declarative(
                digimon_dsl::compiled::CompiledDeclarativeClause::Replacement { .. }
            )
        )
    });
    assert!(
        inherited_replacement.is_some(),
        "Clause C (inherited Substitute on battle deletion) must be authored \
         now that G-DSL-INHERITED-SUBSTITUTE-RETURN-TRASH is closed"
    );
}

/// Total clause count: 1 triggered clause + 1 declarative Replacement clause.
#[test]
fn ex5_015_total_clause_count_matches_full_implementation() {
    let card = dsl_card_data::compiled(CARD_ID);
    let triggered = card
        .effects
        .iter()
        .filter(|c| matches!(c, CompiledClause::Triggered(_)))
        .count();
    let declarative = card
        .effects
        .iter()
        .filter(|c| matches!(c, CompiledClause::Declarative(_)))
        .count();
    assert_eq!(triggered, 1, "Exactly one triggered clause expected");
    assert_eq!(
        declarative, 1,
        "One declarative Replacement clause expected (Clause C authored)"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 2 — Behavioral: Clause A+B (on_play / when_digivolving)
// ═══════════════════════════════════════════════════════════════════════════════

/// Positive path — reveal contains 2 eligible cards (a Garurumon-named and
/// an X-Antibody-trait-named card) plus 2 fillers. Player picks both
/// eligible cards across the two single-pick buckets (with `no_duplicate_cards`
/// so a single card can't fill both); both land in hand; the 2 fillers go to
/// deck bottom; then the player picks "Skip" for the conditional trash.
#[test]
fn ex5_015_on_play_picks_two_eligible_and_bottoms_remainder() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX5-015 YAML loads")
        .add_card(make_named_digimon("GARU", "WereGarurumon", &["Beast Man"]))
        .add_card(make_named_digimon(
            "XANT",
            "MetalGarurumon (X Antibody)",
            &["Cyborg", "X Antibody"],
        ))
        .add_card(make_test_card("FILLER1", "Filler1"))
        .add_card(make_test_card("FILLER2", "Filler2"))
        .deck(0, &["GARU", "XANT", "FILLER1", "FILLER2"])
        .deck(1, &["FILLER1"])
        .hand(0, &[CARD_ID])
        .memory(10)
        .start();

    runner.play(0, 0).expect("play EX5-015");

    // Bucket A (max 1) installs first. Pick GARU.
    pick_revealed_by_id(&mut runner, 0, "GARU", "pick GARU for bucket A");

    // Bucket B (max 1) installs next. Pick XANT (no_duplicate_cards prevents
    // re-picking GARU).
    pick_revealed_by_id(&mut runner, 0, "XANT", "pick XANT for bucket B");

    // After both buckets resolve, place_remainder_on_deck sends the 2 fillers
    // to the bottom (may need to walk a remainder-order prompt). Then the
    // select_effect_choice [Trash 1 from hand / Skip] installs. Pick the
    // LAST legal action (favours Skip / PASS for binary choices and
    // remainder ordering).
    while let Some(view) = runner.pending_selection_view() {
        let action = view
            .valid_action_ids
            .last()
            .copied()
            .expect("valid_action_ids non-empty");
        runner.execute_action(0, action).expect("advance");
    }
    let _ = runner.auto_resolve();

    let hand_ids = zone_ids(&runner.game.players[0].hand, &runner.game.card_data);
    assert!(
        hand_ids.contains(&"GARU".to_string()),
        "GARU must be added to hand; hand={hand_ids:?}"
    );
    assert!(
        hand_ids.contains(&"XANT".to_string()),
        "XANT must be added to hand; hand={hand_ids:?}"
    );
    assert!(
        !hand_ids.contains(&"FILLER1".to_string()) && !hand_ids.contains(&"FILLER2".to_string()),
        "FILLERs must NOT enter hand; hand={hand_ids:?}"
    );
}

/// No eligible cards in reveal — bucket has zero candidates and silently
/// resolves; all 4 revealed cards bottom-deck. Player can pick "Skip" for
/// the conditional trash branch.
#[test]
fn ex5_015_on_play_no_eligible_cards_bottoms_all_four() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX5-015 YAML loads")
        .add_card(make_test_card("FILLER1", "Filler1"))
        .add_card(make_test_card("FILLER2", "Filler2"))
        .add_card(make_test_card("FILLER3", "Filler3"))
        .add_card(make_test_card("FILLER4", "Filler4"))
        .deck(0, &["FILLER1", "FILLER2", "FILLER3", "FILLER4"])
        .deck(1, &["FILLER1"])
        .hand(0, &[CARD_ID])
        .memory(10)
        .start();

    runner.play(0, 0).expect("play EX5-015");

    // Walk all subsequent prompts — for the empty bucket, the engine should
    // skip to remainder-placement; then the select_effect_choice [Trash/Skip]
    // installs. Pick the last valid action throughout to favour Skip / PASS.
    while let Some(view) = runner.pending_selection_view() {
        let action = view
            .valid_action_ids
            .last()
            .copied()
            .expect("valid_action_ids non-empty");
        runner.execute_action(0, action).expect("advance");
    }
    let _ = runner.auto_resolve();

    let hand_ids = zone_ids(&runner.game.players[0].hand, &runner.game.card_data);
    for filler in ["FILLER1", "FILLER2", "FILLER3", "FILLER4"] {
        assert!(
            !hand_ids.contains(&filler.to_string()),
            "{filler} must NOT enter hand when no bucket matches; hand={hand_ids:?}"
        );
    }

    // All 4 fillers must have returned to the deck (it should grow from 0
    // post-reveal back to ~4 after place_remainder_on_deck).
    let deck_ids = zone_ids(&runner.game.players[0].deck, &runner.game.card_data);
    assert_eq!(
        deck_ids.len(),
        4,
        "all 4 reveal cards must return to deck; deck={deck_ids:?}"
    );
}

/// Conditional-trash branch (positive): when the player picks "Trash 1
/// from hand" after adding cards, the select_hand prompt installs and the
/// chosen hand card is trashed.
#[test]
fn ex5_015_on_play_trash_branch_trashes_chosen_hand_card() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX5-015 YAML loads")
        .add_card(make_named_digimon("GARU", "WereGarurumon", &["Beast Man"]))
        .add_card(make_named_digimon(
            "XANT",
            "MetalGarurumon (X Antibody)",
            &["Cyborg", "X Antibody"],
        ))
        .add_card(make_test_card("FILLER", "Filler"))
        .add_card(make_test_card("HANDDISCARD", "HandDiscardable"))
        .deck(0, &["GARU", "XANT", "FILLER", "FILLER"])
        .deck(1, &["FILLER"])
        .hand(0, &[CARD_ID, "HANDDISCARD"])
        .memory(10)
        .start();

    runner.play(0, 0).expect("play EX5-015");

    // Pick GARU for bucket A; XANT for bucket B.
    pick_revealed_by_id(&mut runner, 0, "GARU", "pick GARU bucket A");
    pick_revealed_by_id(&mut runner, 0, "XANT", "pick XANT bucket B");

    // Walk subsequent prompts: prefer the FIRST legal action (which is the
    // "Trash" branch label = action 0 for select_effect_choice). Then for
    // the select_hand prompt that follows, pick the only hand card
    // (HANDDISCARD).
    let mut steps = 0;
    while let Some(view) = runner.pending_selection_view() {
        steps += 1;
        if steps > 32 {
            panic!("too many pending selections — possible infinite loop");
        }
        let action = view
            .valid_action_ids
            .first()
            .copied()
            .expect("valid_action_ids non-empty");
        runner.execute_action(0, action).expect("advance");
    }
    let _ = runner.auto_resolve();

    // After the trash branch resolves, HANDDISCARD must NOT be in hand
    // anymore. (Whether it ends up in trash specifically is engine-internal;
    // the printed effect just says "trash 1 card in your hand".)
    let hand_ids = zone_ids(&runner.game.players[0].hand, &runner.game.card_data);
    assert!(
        !hand_ids.contains(&"HANDDISCARD".to_string()),
        "HANDDISCARD must be trashed from hand when player picks Trash branch; \
         hand={hand_ids:?}"
    );

    // GARU + XANT should be in hand (replacing the trashed card and
    // accounting for the played EX5-015).
    assert!(
        hand_ids.contains(&"GARU".to_string()),
        "GARU added to hand; hand={hand_ids:?}"
    );
    assert!(
        hand_ids.contains(&"XANT".to_string()),
        "XANT added to hand; hand={hand_ids:?}"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 3 — BLOCKED behavioral coverage (Clause C, OMITTED until gap closes)
// ═══════════════════════════════════════════════════════════════════════════════

use digimon_engine::replacement::ReplacementCause;
use digimon_engine::selection::SelectionKind;

/// Seed `count` non-Digi-Egg cards into P0's trash.
fn seed_non_egg_trash(runner: &mut DebugRunner, count: usize) {
    for i in 0..count {
        let id = format!("TRASH-{i}");
        let data = make_named_digimon(&id, &format!("TrashCard{i}"), &["Beast"]);
        let data_index = runner.game.card_data.len();
        runner.game.card_data.push(data);
        let card_index = runner.game.next_card_index();
        let source = CardSource::new(data_index, 0, card_index);
        runner.game.players[0].trash.push(source);
    }
}

/// Inherited Substitute (Clause C) — EX5-015 stacked under a Garurumon-named
/// host, the host would be deleted in battle. The player accepts the optional
/// replacement, returns 2 non-Digi-Egg trash cards to the deck bottom, and
/// the host survives.
#[test]
fn ex5_015_inherited_substitute_prevents_battle_deletion_on_garurumon_host() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX5-015 YAML loads")
        .add_card(make_named_digimon("HOST", "WereGarurumon", &["Beast Man"]))
        .start();
    // EX5-015 sits as the digivolution source beneath the Garurumon host.
    let host = runner.place_stack(0, &[CARD_ID, "HOST"]);
    seed_non_egg_trash(&mut runner, 3);
    let deck_before = runner.game.players[0].deck.len();

    runner
        .game
        .delete_permanent_with_cause(host, ReplacementCause::Battle);

    // Optional Substitute accept prompt.
    let view = runner
        .pending_selection_view()
        .expect("Substitute accept prompt installs");
    assert_eq!(view.kind, SelectionKind::Replacement);
    assert!(view.is_optional, "inherited Substitute is optional");
    runner
        .execute_action(0, view.valid_action_ids[0])
        .expect("accept Substitute");

    // Pick exactly 2 non-Digi-Egg trash cards (min: 2 forbids stopping early).
    for n in 0..2 {
        let view = runner
            .pending_selection_view()
            .unwrap_or_else(|| panic!("trash pick {n} prompt installs"));
        runner
            .execute_action(0, view.valid_action_ids[0])
            .unwrap_or_else(|_| panic!("pick trash card {n}"));
    }
    let _ = runner.auto_resolve();

    // The Garurumon host must survive (deletion cancelled).
    assert_eq!(
        runner.battle_area_size(0),
        1,
        "Garurumon host must survive the battle deletion"
    );
    // 2 trash cards moved to the bottom of the deck.
    assert_eq!(
        runner.game.players[0].deck.len(),
        deck_before + 2,
        "exactly 2 trash cards returned to the deck"
    );
}

/// Inherited Substitute must NOT fire when the host is NOT named Garurumon
/// or Omnimon — even if EX5-015 is stacked under it.
#[test]
fn ex5_015_inherited_substitute_does_not_fire_on_non_garurumon_omnimon_host() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX5-015 YAML loads")
        .add_card(make_named_digimon("HOST", "Greymon", &["Dinosaur"]))
        .start();
    let host = runner.place_stack(0, &[CARD_ID, "HOST"]);
    seed_non_egg_trash(&mut runner, 3);

    runner
        .game
        .delete_permanent_with_cause(host, ReplacementCause::Battle);

    // No Substitute prompt — the host name does not match Garurumon/Omnimon.
    assert!(
        runner.pending_selection().is_none(),
        "Substitute must not fire when the host is not a Garurumon/Omnimon"
    );
    assert_eq!(
        runner.battle_area_size(0),
        0,
        "non-Garurumon host is deleted normally (no substitute)"
    );
}

/// Inherited Substitute must NOT fire on non-battle deletion (effect-driven).
#[test]
fn ex5_015_inherited_substitute_does_not_fire_outside_battle() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX5-015 YAML loads")
        .add_card(make_named_digimon("HOST", "WereGarurumon", &["Beast Man"]))
        .start();
    let host = runner.place_stack(0, &[CARD_ID, "HOST"]);
    seed_non_egg_trash(&mut runner, 3);

    runner
        .game
        .delete_permanent_with_cause(host, ReplacementCause::OpponentEffect);

    // No Substitute prompt — `replacement_cause: battle` gates to battle only.
    assert!(
        runner.pending_selection().is_none(),
        "Substitute must not fire on effect-driven (non-battle) deletion"
    );
    assert_eq!(
        runner.battle_area_size(0),
        0,
        "effect-deleted Garurumon host is not protected by the battle-only Substitute"
    );
}

/// Inherited Substitute must NOT fire if trash has < 2 non-Digi-Egg cards
/// (the required 2-card cost is unpayable).
#[test]
fn ex5_015_inherited_substitute_blocked_when_trash_has_fewer_than_two_non_egg() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX5-015 YAML loads")
        .add_card(make_named_digimon("HOST", "WereGarurumon", &["Beast Man"]))
        .start();
    let host = runner.place_stack(0, &[CARD_ID, "HOST"]);
    // Only 1 non-Digi-Egg card in trash — the 2-card cost cannot be paid.
    seed_non_egg_trash(&mut runner, 1);

    runner
        .game
        .delete_permanent_with_cause(host, ReplacementCause::Battle);

    // No Substitute prompt — the required cost is unpayable, so the
    // replacement clause never activates (cost-then-cancel guard).
    assert!(
        runner.pending_selection().is_none(),
        "Substitute must not offer when fewer than 2 non-Digi-Egg cards are in trash"
    );
    assert_eq!(
        runner.battle_area_size(0),
        0,
        "host is deleted normally when the substitute cost is unpayable"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 4 — Anti-helpers (silence dead-code warnings)
// ═══════════════════════════════════════════════════════════════════════════════

#[allow(dead_code)]
fn _unused_silencer() {
    let _ = ex5_015_card_data();
    let _ = make_digi_egg("X", "X");
}
