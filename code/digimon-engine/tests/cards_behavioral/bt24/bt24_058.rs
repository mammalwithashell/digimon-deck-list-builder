//! BT24-058 Blimpmon — Digimon, Lv.4, Black, DP 5000, Cost 5.
//! Traits: Machine, Iliad, TS. Attribute: Data.
//!
//! # Card text (data/card_bundles/BT24-058.md — official Bandai DB, verbatim)
//!
//! Digivolve: Black Lv.3 / Cost 2 (standard circle)
//! Special digivolution condition: [Digivolve] Lv.3 w/[TS] trait: Cost 2
//!
//! [On Play] [When Digivolving] Reveal the top 3 cards of your deck. Among
//! them, add 1 [Machine], [Cyborg] or [TS] Digimon card or Tamer card to the
//! hand or place it as any of your [Machine], [Cyborg] or [TS] trait
//! Digimon's bottom digivolution card. Return the rest to the top or bottom
//! of the deck.
//!
//! Inherited Effect: ＜Reboot＞ (This Digimon also unsuspends in your
//! opponent's unsuspend phase.)
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT24/Black/BT24_058.cs
//!
//! DCGO confirms:
//! - Alt digivolve: `AddSelfDigivolutionRequirementStaticEffect` gated on
//!   `TopCard.IsLevel3 && TopCard.HasTSTraits`, cost 2, `ignoreDigivolutionRequirement:
//!   false` (still requires a genuine Lv.3 TS-trait material; does not bypass
//!   into arbitrary materials).
//! - Shared On Play / When Digivolving body: `SimplifiedRevealDeckTopCardsAndSelect`
//!   (reveal 3, `canNoSelect: false` i.e. the pick is mandatory whenever a
//!   valid candidate exists in the top 3, `maxCount: 1`) filtered to
//!   `(IsDigimon || IsTamer) && (EqualsTraits("Machine") || EqualsTraits("Cyborg")
//!   || EqualsTraits("TS"))`. For the picked card: a bool selection
//!   ("Add to hand" / "Add to digivolution cards"). If hand: `AddHandCards`.
//!   If digivolution cards: `SelectPermanentEffect` with
//!   `canTargetCondition = IsPermanentExistsOnOwnerBattleAreaDigimon && (Machine
//!   || Cyborg || TS trait)`, `maxCount = Min(1, matching permanent count)`,
//!   `canNoSelect: false` — mandatory once ≥1 valid permanent exists (Blimpmon
//!   itself always qualifies while on the field, since it has the TS trait, so
//!   this can never structurally short-circuit to zero targets in a reachable
//!   game state) — then `AddDigivolutionCardsBottom`.
//! - Remainder: `RemainingCardsPlace.DeckTopOrBottom` — a single "top or
//!   bottom" bool choice for the whole remainder pile (not per-card).
//! - `ActivateClass.SetUpActivateClass(..., maxCountPerTurn: -1, isOptional:
//!   false, ...)` — the clause itself is NOT wrapped in an overall "you may"
//!   or an OPT lock; only the inner pick naturally fizzles when the top 3
//!   contain no eligible card (DSL: `choose_from_reveal` short-circuits with
//!   no PendingSelection installed when the filter matches nothing).
//! - Inherited: `RebootSelfStaticEffect(isInheritedEffect: true)` → grants
//!   `<Reboot>`.
//!
//! # DSL authoring note
//!
//! `choose_from_reveal`'s `BottomSourceOf { target }` resolves the target
//! permanent binding at INSTALL time (before the reveal pick UI opens), so
//! the DSL body must select the destination permanent BEFORE the
//! `choose_from_reveal` step that routes to it — unlike DCGO's UI order
//! (pick card first, then ask hand-vs-source, then pick permanent). Both
//! orders are faithful to the printed text: the printed effect makes no claim
//! about UI sequencing, only about what the final state must be (1 eligible
//! card added to hand OR placed as bottom source of an eligible Digimon).
//! Structured as: reveal 3 -> effect_choice (hand vs place) -> [hand branch:
//! choose_from_reveal -> hand] / [place branch: select_own_permanent (Machine/
//! Cyborg/TS filter) -> choose_from_reveal -> bottom_source_of(that binding)]
//! -> order_remainder([deck_top, deck_bottom]).
//!
//! # Patterns (docs/RUST_DSL_TEST_API.md §4.3)
//! - A2 Two-pass reveal (reveal, sort, add or place) — BT18-060/BT15-077-shaped
//! - E1 Branch-choice OnPlay ("choose: A or B")
//! - G-ALT-PATH level+trait alt-digivolve (mirrors BT24-011's `trait_has: TS`
//!   alt path)
//! - H7 Reboot (inherited keyword grant)

use digimon_dsl::compiled::{
    CompiledClause, CompiledColor, CompiledCost, CompiledScope, CompiledStep, CompiledTiming,
};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::CardKind;
use digimon_engine::selection::SelectionKind;
use digimon_engine::{EffectTiming, TriggerSource};

// ─────────────────────────────────────────────────────────────────────────
// Fixtures
// ─────────────────────────────────────────────────────────────────────────

/// Digimon card with the given traits (defaults: Lv3/2000DP/cost3/red, per
/// `make_test_card`). Used for eligible / non-eligible reveal-pool fillers.
fn make_trait_digimon(id: &str, traits: &[&str]) -> digimon_engine::card_data::CardData {
    let mut card = make_test_card(id, id);
    card.traits = traits.iter().map(|s| s.to_string()).collect();
    card
}

/// Tamer card with the given traits — used for the "or Tamer card" half of
/// the printed eligibility filter.
fn make_trait_tamer(id: &str, traits: &[&str]) -> digimon_engine::card_data::CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Tamer;
    card.traits = traits.iter().map(|s| s.to_string()).collect();
    card
}

fn zone_ids(
    cards: &[digimon_engine::card_source::CardSource],
    data: &[digimon_engine::card_data::CardData],
) -> Vec<String> {
    cards.iter().map(|c| c.card_id(data).to_string()).collect()
}

/// Push `id` onto the deck top (deck is a `Vec` popped from the back, so the
/// LAST id pushed ends up on top / drawn or revealed first). Mirrors the
/// convention `tests/cards_behavioral/p/p_167.rs` uses.
fn stack_deck_top_to_bottom(runner: &mut DebugRunner, player: u8, ids_top_to_bottom: &[&str]) {
    use digimon_engine::card_source::CardSource;
    for id in ids_top_to_bottom.iter().rev() {
        let data_idx = runner
            .game
            .card_data
            .iter()
            .position(|c| c.card_id == *id)
            .unwrap_or_else(|| panic!("stack_deck_top_to_bottom: unknown card_id {id}"));
        let card_index = runner.game.next_card_index();
        runner.game.players[player as usize]
            .deck
            .push(CardSource::new(data_idx, player, card_index));
    }
}

/// Base runner: Blimpmon in hand, memory high enough to play it (cost 5),
/// deck stocked with 3 non-eligible filler cards on top so the reveal-3
/// naturally comes back empty of matches unless a test overrides the deck.
fn base_runner_no_match() -> DebugRunner {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT24-058")
        .expect("BT24-058 YAML parses and compiles")
        .add_card(make_trait_digimon("FILL-A", &[]))
        .add_card(make_trait_digimon("FILL-B", &[]))
        .add_card(make_trait_digimon("FILL-C", &[]))
        .hand(0, &["BT24-058"])
        .memory(10)
        .build();
    stack_deck_top_to_bottom(&mut runner, 0, &["FILL-A", "FILL-B", "FILL-C"]);
    runner
}

// ─────────────────────────────────────────────────────────────────────────
// Section 1 — Structural assertions
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn bt24_058_has_standard_black_lv3_and_ts_trait_lv3_alt_paths_same_cost() {
    let runner = base_runner_no_match();
    let compiled = runner
        .compiled_card("BT24-058")
        .expect("BT24-058 compiled card present");

    assert_eq!(
        compiled.alt_paths.len(),
        2,
        "one standard black Lv3 path + one TS-trait Lv3 alt path"
    );
    assert!(
        compiled
            .alt_paths
            .iter()
            .all(|p| p.cost == Some(CompiledCost::Literal(2))),
        "both circles are printed at cost 2"
    );
    assert!(
        compiled.alt_paths.iter().any(|p| {
            let Some(from) = p.from.as_ref() else {
                return false;
            };
            from.level_eq == Some(3) && from.color_is == Some(CompiledColor::Black)
        }),
        "BT24-058 must retain the standard black level-3 digivolve path"
    );
    assert!(
        compiled.alt_paths.iter().any(|p| {
            let Some(from) = p.from.as_ref() else {
                return false;
            };
            from.level_eq == Some(3) && from.trait_has.as_deref() == Some("TS")
        }),
        "BT24-058 must also digivolve from any level-3 [TS] trait card"
    );
}

#[test]
fn bt24_058_has_on_play_and_when_digivolving_shared_reveal_clause() {
    let runner = base_runner_no_match();
    let compiled = runner
        .compiled_card("BT24-058")
        .expect("BT24-058 compiled card present");

    let clause = compiled
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t)
                if t.when.contains(&CompiledTiming::OnPlay)
                    && t.when.contains(&CompiledTiming::WhenDigivolving) =>
            {
                Some(t)
            }
            _ => None,
        })
        .expect("BT24-058 must have a shared On Play + When Digivolving clause");

    assert_eq!(clause.scope, CompiledScope::FaceUp);
    assert!(
        !clause.optional,
        "the clause itself is not a 'you may' — DCGO SetUpActivateClass isOptional=false"
    );
    assert!(
        !clause.once_per_turn,
        "DCGO SetUpActivateClass maxCountPerTurn=-1 — no OPT lock"
    );
}

#[test]
fn bt24_058_reveal_clause_uses_reveal_effect_choice_and_order_remainder() {
    let runner = base_runner_no_match();
    let compiled = runner
        .compiled_card("BT24-058")
        .expect("BT24-058 compiled card present");

    let clause = compiled
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t)
                if t.when.contains(&CompiledTiming::OnPlay)
                    && t.when.contains(&CompiledTiming::WhenDigivolving) =>
            {
                Some(t)
            }
            _ => None,
        })
        .expect("shared reveal clause present");

    let has_reveal_top_deck = clause
        .process
        .iter()
        .any(|s| matches!(s, CompiledStep::RevealTopDeck { .. }));
    let has_effect_choice = clause
        .process
        .iter()
        .any(|s| matches!(s, CompiledStep::SelectEffectChoice { .. }));
    let has_choose_from_reveal = walk_steps_for_variant(&clause.process.iter().collect::<Vec<_>>(), |s| {
        matches!(s, CompiledStep::ChooseFromReveal { .. })
    });
    let has_order_remainder = clause
        .process
        .iter()
        .any(|s| matches!(s, CompiledStep::OrderRemainder { .. }));

    assert!(has_reveal_top_deck, "clause must reveal top 3");
    assert!(
        has_effect_choice,
        "clause must offer the hand-vs-source effect choice"
    );
    assert!(
        has_choose_from_reveal,
        "clause must route the picked card via choose_from_reveal"
    );
    assert!(
        has_order_remainder,
        "clause must place the remainder via order_remainder (top-or-bottom)"
    );
}

#[test]
fn bt24_058_has_inherited_reboot() {
    let runner = base_runner_no_match();
    let compiled = runner
        .compiled_card("BT24-058")
        .expect("BT24-058 compiled card present");

    let has_inherited_reboot = compiled.effects.iter().any(|c| match c {
        CompiledClause::Declarative(d) => matches!(
            d,
            digimon_dsl::compiled::CompiledDeclarativeClause::GrantKeyword {
                scope: CompiledScope::Inherited,
                keyword,
                ..
            } if keyword == "Reboot"
        ),
        _ => false,
    });
    assert!(has_inherited_reboot, "BT24-058 must inherit <Reboot>");
}

fn walk_steps_for_variant<F>(steps: &[&CompiledStep], pred: F) -> bool
where
    F: Fn(&CompiledStep) -> bool + Copy,
{
    for s in steps {
        if pred(s) {
            return true;
        }
        if let CompiledStep::If {
            then, else_branch, ..
        } = s
        {
            let then_refs: Vec<&CompiledStep> = then.iter().collect();
            let else_refs: Vec<&CompiledStep> = else_branch.iter().collect();
            if walk_steps_for_variant(&then_refs, pred) {
                return true;
            }
            if walk_steps_for_variant(&else_refs, pred) {
                return true;
            }
        }
    }
    false
}

// ─────────────────────────────────────────────────────────────────────────
// Section 2 — Condition gating: eligible candidate present vs absent
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn bt24_058_on_play_no_eligible_candidate_reveals_and_skips_the_pick_prompt() {
    // Negative: none of the top 3 match Machine/Cyborg/TS Digimon-or-Tamer.
    // The reveal + effect_choice for "hand vs place" is only meaningful once
    // there's a card to route, so with zero eligible candidates in the
    // revealed pool the natural `choose_from_reveal` fizzle inside BOTH
    // branches means no card is ever routed — but note the effect_choice
    // itself still installs (it does not know the reveal contents up front
    // in DCGO's own UI flow the pick happens first). Assert instead on the
    // FINAL state: no card added to hand, no card placed as a source, and
    // the deck ends up back at 3 cards via order_remainder.
    let mut runner = base_runner_no_match();
    let deck_before = runner.game.players[0].deck.len();

    runner.play(0, 0).expect("play Blimpmon");
    runner.auto_resolve().expect("resolve reveal clause with no eligible candidates");

    let hand_ids = zone_ids(&runner.game.players[0].hand, &runner.game.card_data);
    assert!(
        !hand_ids.iter().any(|id| id.starts_with("FILL")),
        "no filler card should land in hand"
    );
    assert_eq!(
        runner.game.players[0].deck.len(),
        deck_before,
        "all 3 revealed filler cards return to the deck"
    );
}

#[test]
fn bt24_058_on_play_no_eligible_candidate_place_branch_also_places_nothing() {
    // Same negative as above, but explicitly driving branch 1 (Place) to
    // exercise the OTHER `choose_from_reveal` fizzle path: `select_own_permanent`
    // still finds Blimpmon itself as a valid target (it has the TS trait), but
    // the nested `choose_from_reveal` for placement has no eligible revealed
    // card to route, so it must not place anything under Blimpmon.
    let mut runner = base_runner_no_match();
    let deck_before = runner.game.players[0].deck.len();

    let blimpmon_index = runner.play(0, 0).expect("play Blimpmon");
    runner
        .execute_branch(1)
        .expect("branch 1 = Place as bottom digivolution card");

    // select_own_permanent still installs (Blimpmon itself qualifies).
    assert_eq!(runner.pending_kind(), Some(SelectionKind::OwnField));
    let view = runner.pending_selection_view().expect("own-field prompt");
    runner
        .execute_action(0, view.valid_action_ids[0])
        .expect("pick Blimpmon as the (unused) placement target");

    runner
        .auto_resolve()
        .expect("resolve remainder even though no card was routed");

    let sources = &runner.game.players[0].battle_area[blimpmon_index].card_sources;
    assert_eq!(
        sources.len(),
        1,
        "Blimpmon must have no extra bottom source when no card in the reveal matched"
    );
    assert_eq!(
        runner.game.players[0].deck.len(),
        deck_before,
        "all 3 revealed filler cards return to the deck"
    );
}

#[test]
fn bt24_058_on_play_eligible_machine_digimon_in_reveal_installs_mandatory_effect_choice() {
    // Positive: an eligible Machine-trait Digimon is among the revealed 3.
    let mut runner = DebugRunner::builder()
        .dsl_card("BT24-058")
        .expect("BT24-058 YAML parses and compiles")
        .add_card(make_trait_digimon("MACHINE-PICK", &["Machine"]))
        .add_card(make_trait_digimon("FILL-A", &[]))
        .add_card(make_trait_digimon("FILL-B", &[]))
        .hand(0, &["BT24-058"])
        .memory(10)
        .build();
    stack_deck_top_to_bottom(&mut runner, 0, &["MACHINE-PICK", "FILL-A", "FILL-B"]);

    runner.play(0, 0).expect("play Blimpmon");

    let kind = runner
        .pending_kind()
        .expect("hand-vs-place effect choice must install when an eligible card is revealed");
    assert_eq!(kind, SelectionKind::EffectChoice);
    assert!(
        !runner.pending_is_optional(),
        "the hand-vs-place choice is not a 'you may' — PASS must not be legal"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Section 3 — Behavioral outcome: hand branch
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn bt24_058_on_play_hand_branch_adds_eligible_machine_digimon_to_hand() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT24-058")
        .expect("BT24-058 YAML parses and compiles")
        .add_card(make_trait_digimon("MACHINE-PICK", &["Machine"]))
        .add_card(make_trait_digimon("FILL-A", &[]))
        .add_card(make_trait_digimon("FILL-B", &[]))
        .hand(0, &["BT24-058"])
        .memory(10)
        .build();
    stack_deck_top_to_bottom(&mut runner, 0, &["MACHINE-PICK", "FILL-A", "FILL-B"]);

    runner.play(0, 0).expect("play Blimpmon");
    runner.execute_branch(0).expect("branch 0 = Add to hand");
    runner.auto_resolve().expect("resolve reveal pick + order_remainder");

    let hand_ids = zone_ids(&runner.game.players[0].hand, &runner.game.card_data);
    assert!(
        hand_ids.contains(&"MACHINE-PICK".to_string()),
        "MACHINE-PICK must land in hand via the hand branch"
    );
    assert_eq!(
        runner.game.players[0].deck.len(),
        2,
        "the other 2 revealed cards return to the deck"
    );
}

#[test]
fn bt24_058_on_play_hand_branch_adds_eligible_ts_tamer_to_hand() {
    // Covers the "... or Tamer card" half of the eligibility filter.
    let mut runner = DebugRunner::builder()
        .dsl_card("BT24-058")
        .expect("BT24-058 YAML parses and compiles")
        .add_card(make_trait_tamer("TS-TAMER-PICK", &["TS"]))
        .add_card(make_trait_digimon("FILL-A", &[]))
        .add_card(make_trait_digimon("FILL-B", &[]))
        .hand(0, &["BT24-058"])
        .memory(10)
        .build();
    stack_deck_top_to_bottom(&mut runner, 0, &["TS-TAMER-PICK", "FILL-A", "FILL-B"]);

    runner.play(0, 0).expect("play Blimpmon");
    runner.execute_branch(0).expect("branch 0 = Add to hand");
    runner.auto_resolve().expect("resolve reveal pick + order_remainder");

    let hand_ids = zone_ids(&runner.game.players[0].hand, &runner.game.card_data);
    assert!(
        hand_ids.contains(&"TS-TAMER-PICK".to_string()),
        "TS-TAMER-PICK (a Tamer card with the TS trait) must land in hand"
    );
}

#[test]
fn bt24_058_on_play_non_eligible_cyborg_named_card_without_kind_match_is_never_pickable() {
    // Negative: an Option card with the Machine trait (traits are ignored for
    // non Digimon/Tamer kinds per DCGO's `(IsDigimon || IsTamer) && ...`
    // gate) must never be offered as a pick target.
    let mut option_card = make_trait_digimon("OPT-MACHINE", &["Machine"]);
    option_card.card_kind = CardKind::Option;

    let mut runner = DebugRunner::builder()
        .dsl_card("BT24-058")
        .expect("BT24-058 YAML parses and compiles")
        .add_card(option_card)
        .add_card(make_trait_digimon("FILL-A", &[]))
        .add_card(make_trait_digimon("FILL-B", &[]))
        .hand(0, &["BT24-058"])
        .memory(10)
        .build();
    stack_deck_top_to_bottom(&mut runner, 0, &["OPT-MACHINE", "FILL-A", "FILL-B"]);

    let deck_before = runner.game.players[0].deck.len();
    runner.play(0, 0).expect("play Blimpmon");
    runner
        .auto_resolve()
        .expect("resolve reveal clause with no eligible Digimon/Tamer candidates");

    let hand_ids = zone_ids(&runner.game.players[0].hand, &runner.game.card_data);
    assert!(
        !hand_ids.contains(&"OPT-MACHINE".to_string()),
        "an Option card must never be picked even if it has a matching trait"
    );
    assert_eq!(runner.game.players[0].deck.len(), deck_before);
}

// ─────────────────────────────────────────────────────────────────────────
// Section 3 (cont.) — Behavioral outcome: place-as-bottom-source branch
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn bt24_058_on_play_place_branch_offers_own_machine_cyborg_ts_digimon_as_targets() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT24-058")
        .expect("BT24-058 YAML parses and compiles")
        .add_card(make_trait_digimon("MACHINE-PICK", &["Machine"]))
        .add_card(make_trait_digimon("FILL-A", &[]))
        .add_card(make_trait_digimon("FILL-B", &[]))
        .hand(0, &["BT24-058"])
        .memory(10)
        .build();
    stack_deck_top_to_bottom(&mut runner, 0, &["MACHINE-PICK", "FILL-A", "FILL-B"]);

    runner.play(0, 0).expect("play Blimpmon");
    runner
        .execute_branch(1)
        .expect("branch 1 = Place as bottom digivolution card");

    // The target-permanent picker must offer Blimpmon itself (Machine/Iliad/TS).
    let kind = runner
        .pending_kind()
        .expect("select_own_permanent prompt must install for the place branch");
    assert_eq!(kind, SelectionKind::OwnField);
    let view = runner.pending_selection_view().expect("view");
    assert!(
        !view.valid_action_ids.is_empty(),
        "Blimpmon itself (Machine/Iliad/TS) is always a valid own-field target"
    );
    assert!(
        !runner.pending_is_optional(),
        "the target-permanent pick is mandatory once ≥1 valid Digimon exists"
    );
}

#[test]
fn bt24_058_on_play_place_branch_places_picked_card_as_bottom_source_of_chosen_permanent() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT24-058")
        .expect("BT24-058 YAML parses and compiles")
        .add_card(make_trait_digimon("MACHINE-PICK", &["Machine"]))
        .add_card(make_trait_digimon("FILL-A", &[]))
        .add_card(make_trait_digimon("FILL-B", &[]))
        .hand(0, &["BT24-058"])
        .memory(10)
        .build();
    stack_deck_top_to_bottom(&mut runner, 0, &["MACHINE-PICK", "FILL-A", "FILL-B"]);

    let blimpmon_index = runner.play(0, 0).expect("play Blimpmon");
    runner
        .execute_branch(1)
        .expect("branch 1 = Place as bottom digivolution card");

    // Drive the own-field target pick (Blimpmon is the only candidate here).
    let view = runner.pending_selection_view().expect("own-field prompt");
    let action_id = view.valid_action_ids[0];
    runner
        .execute_action(0, action_id)
        .expect("pick Blimpmon as the placement target");

    runner
        .auto_resolve()
        .expect("resolve reveal pick routing + order_remainder");

    let blimpmon_handle = digimon_engine::permanent::PermanentHandle {
        player: 0,
        index: blimpmon_index as u8,
    };
    let sources = &runner.game.players[0].battle_area[blimpmon_handle.index as usize].card_sources;
    let source_ids = zone_ids(sources, &runner.game.card_data);
    assert!(
        source_ids.contains(&"MACHINE-PICK".to_string()),
        "MACHINE-PICK must be placed as Blimpmon's bottom digivolution card, got {source_ids:?}"
    );
    assert_eq!(
        sources.first().map(|c| c.card_id(&runner.game.card_data)),
        Some("MACHINE-PICK"),
        "the placed card must be the BOTTOM source (index 0)"
    );

    let hand_ids = zone_ids(&runner.game.players[0].hand, &runner.game.card_data);
    assert!(
        !hand_ids.contains(&"MACHINE-PICK".to_string()),
        "MACHINE-PICK must not also land in hand"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Section 3 (cont.) — Remainder placement: shared top-or-bottom choice
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn bt24_058_remainder_is_a_single_top_or_bottom_choice_for_the_whole_pile() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT24-058")
        .expect("BT24-058 YAML parses and compiles")
        .add_card(make_trait_digimon("MACHINE-PICK", &["Machine"]))
        .add_card(make_trait_digimon("FILL-A", &[]))
        .add_card(make_trait_digimon("FILL-B", &[]))
        .hand(0, &["BT24-058"])
        .memory(10)
        .build();
    stack_deck_top_to_bottom(&mut runner, 0, &["MACHINE-PICK", "FILL-A", "FILL-B"]);

    runner.play(0, 0).expect("play Blimpmon");
    runner.execute_branch(0).expect("branch 0 = Add to hand");

    // Drive the mandatory inner reveal pick (only MACHINE-PICK matches).
    let pick_sel = runner.pending_selection_view().expect("inner reveal pick");
    assert_eq!(pick_sel.kind, SelectionKind::Reveal);
    runner
        .execute_action(0, pick_sel.valid_action_ids[0])
        .expect("pick MACHINE-PICK");

    // Now the remainder-destination effect_choice (top vs bottom) must
    // install exactly once for BOTH remaining filler cards together.
    let remainder_kind = runner
        .pending_kind()
        .expect("order_remainder destination prompt must install");
    assert_eq!(remainder_kind, SelectionKind::EffectChoice);

    let deck_len_before_remainder = runner.game.players[0].deck.len();
    runner.execute_branch(0).expect("choose deck top");
    runner
        .auto_resolve()
        .expect("resolve ordered placement of the remainder");

    assert_eq!(
        runner.game.players[0].deck.len(),
        deck_len_before_remainder + 2,
        "both filler cards (FILL-A, FILL-B) return to the deck together"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Section 3 (cont.) — When Digivolving path fires the same shared clause
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn bt24_058_when_digivolving_also_fires_the_shared_reveal_clause() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT24-058")
        .expect("BT24-058 YAML parses and compiles")
        .add_card(make_trait_digimon("MACHINE-PICK", &["Machine"]))
        .add_card(make_trait_digimon("FILL-A", &[]))
        .add_card(make_trait_digimon("FILL-B", &[]))
        .memory(10)
        .build();
    stack_deck_top_to_bottom(&mut runner, 0, &["MACHINE-PICK", "FILL-A", "FILL-B"]);

    // Blimpmon already stacked (as if digivolved into) — fire its
    // WhenDigivolving trigger directly, mirroring the bt24_017-style idiom
    // for behavioral WhenDigivolving coverage.
    let blimpmon = runner.place_on_field(0, "BT24-058", Some(0));
    runner
        .game
        .enqueue_triggered(EffectTiming::WhenDigivolving, TriggerSource::Permanent(blimpmon));
    runner.game.drain_effect_queue();

    let kind = runner
        .pending_kind()
        .expect("When Digivolving must also install the hand-vs-place effect choice");
    assert_eq!(kind, SelectionKind::EffectChoice);
}

// ─────────────────────────────────────────────────────────────────────────
// Section 4 — Event log: reveal fires Reveal events; no card is ever trashed
// ─────────────────────────────────────────────────────────────────────────
//
// BT24-058's clause has no cost (unlike P-167's "by trashing 1 source" —
// there is no trash/security-loss side effect to assert here). Per test API
// §5 Section 4 the event-log test applies to cost-firing clauses; this
// clause has none, so instead we assert the two things the event log DOES
// promise for a reveal-and-route effect: (1) the 3 top-deck cards each emit
// a `Reveal` event, and (2) nothing is ever trashed by this clause — not the
// hand-routed card, not the source-placed card, not the remainder (which
// goes back to the deck, never to trash).

#[test]
fn bt24_058_on_play_reveal_emits_three_reveal_events_and_never_trashes() {
    use digimon_engine::events::GameEvent;

    let mut runner = DebugRunner::builder()
        .dsl_card("BT24-058")
        .expect("BT24-058 YAML parses and compiles")
        .add_card(make_trait_digimon("MACHINE-PICK", &["Machine"]))
        .add_card(make_trait_digimon("FILL-A", &[]))
        .add_card(make_trait_digimon("FILL-B", &[]))
        .hand(0, &["BT24-058"])
        .memory(10)
        .build();
    stack_deck_top_to_bottom(&mut runner, 0, &["MACHINE-PICK", "FILL-A", "FILL-B"]);

    let cp = runner.event_checkpoint();
    runner.play(0, 0).expect("play Blimpmon");
    runner.execute_branch(0).expect("branch 0 = Add to hand");
    runner.auto_resolve().expect("resolve reveal pick + order_remainder");

    let reveal_count = runner
        .events_of_kind(cp, |e| matches!(e, GameEvent::Reveal { .. }))
        .len();
    assert_eq!(reveal_count, 3, "all 3 top-deck cards must emit a Reveal event");

    let trash_count = runner
        .events_of_kind(cp, |e| matches!(e, GameEvent::Trash { .. }))
        .len();
    assert_eq!(
        trash_count, 0,
        "BT24-058's reveal clause never trashes a card (hand / bottom-source / deck-return only)"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Section 5 — No OPT lock (clause has no [Once Per Turn] marker)
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn bt24_058_when_digivolving_can_fire_again_after_a_second_digivolution_same_turn() {
    // BT24-058's shared clause carries no [Once Per Turn] tag (DCGO
    // maxCountPerTurn = -1). On Play only fires once per play by
    // construction; When Digivolving is not OPT-gated, so digivolving twice
    // in the same turn (e.g. two separate Blimpmon copies digivolving) must
    // both offer the reveal clause. We assert this structurally (see
    // `bt24_058_has_on_play_and_when_digivolving_shared_reveal_clause`
    // above: `once_per_turn == false`) plus behaviorally by firing
    // WhenDigivolving twice on two different Blimpmon copies in one turn.
    let mut runner = DebugRunner::builder()
        .dsl_card("BT24-058")
        .expect("BT24-058 YAML parses and compiles")
        .add_card(make_trait_digimon("FILL-A", &[]))
        .add_card(make_trait_digimon("FILL-B", &[]))
        .add_card(make_trait_digimon("FILL-C", &[]))
        .add_card(make_trait_digimon("FILL-D", &[]))
        .add_card(make_trait_digimon("FILL-E", &[]))
        .add_card(make_trait_digimon("FILL-F", &[]))
        .memory(20)
        .build();
    stack_deck_top_to_bottom(
        &mut runner,
        0,
        &["FILL-A", "FILL-B", "FILL-C", "FILL-D", "FILL-E", "FILL-F"],
    );

    let blimpmon_a = runner.place_on_field(0, "BT24-058", Some(0));
    let blimpmon_b = runner.place_on_field(0, "BT24-058", Some(0));

    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(blimpmon_a),
    );
    runner.game.drain_effect_queue();
    assert_eq!(runner.pending_kind(), Some(SelectionKind::EffectChoice));
    runner
        .auto_resolve()
        .expect("resolve first Blimpmon's When Digivolving clause");

    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(blimpmon_b),
    );
    runner.game.drain_effect_queue();
    assert_eq!(
        runner.pending_kind(),
        Some(SelectionKind::EffectChoice),
        "a second Blimpmon digivolving the same turn must also trigger the reveal clause (no OPT lock)"
    );
}
