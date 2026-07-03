//! EX7-073 BeelStarmon (X Antibody) — Digimon, Lv.6, Purple, DP 12000,
//! Cost 12. Traits: Wizard, X Antibody, Three Musketeers. Attribute: Virus.
//!
//! # Card text (per-card JSON `code/digimon-engine/cards/ex7/EX7-073.json`,
//! `effect_description_eng` — verbatim)
//!
//! [When Digivolving] You may use 1 Option card with [Three Musketeers] in
//! its text from your hand without paying the cost.
//! [When Digivolving] [When Attacking] By trashing 2 cards with the [Three
//! Musketeers] trait in this Digimon's digivolution cards, delete 1 of your
//! opponent's Digimon with the highest level and trash the top card of your
//! opponent's security stack.
//!
//! Standard digivolve: Purple Lv.5 / cost 4 (`evo_costs`).
//! Alt-source (`xros_req`): "[Digivolve] Lv.6 w/[Three Musketeers] trait
//! w/o [X Antibody] trait: Cost 1".
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/EX7/Purple/EX7_073.cs
//!
//! - Digivolution Condition (None): `AddSelfDigivolutionRequirementStaticEffect`
//!   with `PermanentCondition = TopCard.EqualsTraits("Three Musketeers") &&
//!   Level == 6 && !EqualsTraits("X Antibody")`, `digivolutionCost: 1`.
//! - Clause A [When Digivolving] (OnEnterFieldAnyone, ActivateClass):
//!   optional ("you may") `SelectHandEffect` over hand Options that
//!   `HasText("Three Musketeers")`, `HasUseCost`, `!CanNotPlayThisOption`;
//!   max 1, `canNoSelect: true`; accepted pick is played via
//!   `PlayOptionCards(payCost: false)`.
//! - Clause B [When Digivolving] (OnEnterFieldAnyone, ActivateClass):
//!   `CanActivateCondition` requires >= 1 digivolution-card with
//!   `EqualsTraits("Three Musketeers")`; the coroutine body then requires
//!   `DigivolutionCards.Count >= 2` (total) before offering
//!   `SelectCardEffect` (root: DigivolutionCards, filter: Three-Musketeers
//!   trait, `maxCount: 2`, `canNoSelect: true`, `canEndNotMax: false`).
//!   Only when exactly 2 were selected (`selectedCards.Count >= 2`) does
//!   `trashed = true` gate the tail: optional `SelectPermanentEffect`
//!   (`canNoSelect: false`, `maxCount: Math.Min(1, count)`) over opponent
//!   Digimon matching `IsMaxLevel` (highest level), mode `Destroy`; then
//!   unconditional `IDestroySecurity(destroySecurityCount: 1, fromTop:
//!   true)` on the opponent.
//! - Clause C [When Attacking] (OnAllyAttack): identical body/gates to
//!   Clause B, fired on `CanTriggerOnAttack`.
//!
//! # Patterns this test covers (docs/RUST_DSL_TEST_API.md §4.3)
//! - E2 OPT + optional decline (Clause A: use-from-hand-free is a "you may")
//! - D2-adjacent: `use_option_from_hand` free-use verb (BT24-085 idiom)
//! - Group C/F5-adjacent: `select_own_sources` cost-trash → `delete_permanent`
//!   (highest-level aggregate) + `trash_top_security` composite (Clause B/C)
//! - G1: alt-path `from:` predicate with `not: { trait_has }` negation
//!   (Digivolve Lv.6 Three-Musketeers-without-X-Antibody, cost 1)
//! - Multi-timing clause: identical body fires on both [When Digivolving]
//!   and [When Attacking] (two separate compiled clauses, independent OPT
//!   surfaces — DCGO installs two distinct `ActivateClass`/timing pairs,
//!   not a shared OPT hash)

#![allow(dead_code)]

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledCardKind, CompiledClause, CompiledColor, CompiledCost,
    CompiledScope, CompiledStep, CompiledTiming,
};
use digimon_engine::action::space::{encode_source_select, PASS, PLAY_HAND_START};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, CardKind, PlaySource};
use digimon_engine::events::GameEvent;
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::SelectionKind;

const CARD_ID: &str = "EX7-073";
const YAML: &str = include_str!("../../../cards/ex7/EX7-073.yaml");

// ─── Fixture builders ─────────────────────────────────────────────────────────

/// The standard digivolve base: Purple Lv.5 (cost 4 per `evo_costs`).
fn purple_lv5(id: &str) -> CardData {
    let mut c = make_test_card(id, "PurpleChampion");
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Purple];
    c.level = Some(5);
    c.dp = Some(6000);
    c
}

/// A [Three Musketeers]-trait, level-6 Digimon WITHOUT [X Antibody] — the
/// alt-source base ("Lv.6 w/[Three Musketeers] trait w/o [X Antibody]
/// trait: Cost 1").
fn three_musketeers_lv6_no_x_antibody(id: &str) -> CardData {
    let mut c = make_test_card(id, "ThreeMusketeersBase");
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Purple];
    c.level = Some(6);
    c.dp = Some(10000);
    c.traits = vec!["Three Musketeers".to_string()];
    c
}

/// A [Three Musketeers]-trait, level-6 Digimon WITH [X Antibody] — must NOT
/// satisfy the alt-source ("w/o [X Antibody]").
fn three_musketeers_lv6_with_x_antibody(id: &str) -> CardData {
    let mut c = make_test_card(id, "ThreeMusketeersXAntibodyBase");
    c.card_kind = CardKind::Digimon;
    c.level = Some(6);
    c.dp = Some(10000);
    c.traits = vec!["Three Musketeers".to_string(), "X Antibody".to_string()];
    c
}

/// A digivolution-source Digimon carrying the [Three Musketeers] trait —
/// used to build BeelStarmon's own stack for the trash-cost clause.
fn three_musketeers_source(id: &str) -> CardData {
    let mut c = make_test_card(id, "ThreeMusketeersSource");
    c.card_kind = CardKind::Digimon;
    c.level = Some(5);
    c.dp = Some(5000);
    c.traits = vec!["Three Musketeers".to_string()];
    c
}

/// A digivolution-source Digimon WITHOUT the [Three Musketeers] trait — the
/// trash-cost filter must exclude it.
fn plain_source(id: &str) -> CardData {
    let mut c = make_test_card(id, "PlainSource");
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(4000);
    c
}

/// Generic opponent Digimon, parameterized by level.
fn opp_digimon(id: &str, level: u8, dp: i32) -> CardData {
    let mut c = make_test_card(id, "OppDigimon");
    c.card_kind = CardKind::Digimon;
    c.level = Some(level);
    c.dp = Some(dp);
    c
}

/// An Option card carrying "Three Musketeers" in its printed text (matches
/// DCGO's `HasText("Three Musketeers")` filter for Clause A).
fn three_musketeers_option(id: &str, cost: u16) -> CardData {
    let mut c = make_test_card(id, "ThreeMusketeersOption");
    c.card_kind = CardKind::Option;
    c.play_cost = cost;
    c.effect_text = "A card with Three Musketeers text.".to_string();
    c
}

/// An Option card with NO "Three Musketeers" text — must be excluded from
/// Clause A's free-use candidate pool.
fn plain_option(id: &str, cost: u16) -> CardData {
    let mut c = make_test_card(id, "PlainOption");
    c.card_kind = CardKind::Option;
    c.play_cost = cost;
    c.effect_text = "Some unrelated option text.".to_string();
    c
}

fn base_builder() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("EX7-073 YAML parses")
}

fn zone_ids(
    cards: &[digimon_engine::card_source::CardSource],
    data: &[digimon_engine::card_data::CardData],
) -> Vec<String> {
    cards
        .iter()
        .map(|card| card.card_id(data).to_string())
        .collect()
}

/// Digivolve EX7-073 (hand index 0) onto the given base slot via the
/// standard Purple Lv.5 path; the [When Digivolving] triggers fire inside
/// this call.
fn digivolve_beelstarmon(runner: &mut DebugRunner, base: PermanentHandle) {
    assert!(
        runner
            .game
            .digivolve_from_hand(0, 0, base.index as usize, PlaySource::ByHand),
        "EX7-073 must digivolve from a Purple Lv.5 base (cost 4)"
    );
    assert_eq!(
        runner.game.players[0].battle_area[base.index as usize]
            .top_card()
            .card_id(&runner.game.card_data),
        CARD_ID,
        "EX7-073 is now the top card of the digivolved stack"
    );
    // EX7-073 carries TWO independent [When Digivolving] clauses (the free
    // Option-use clause + the trash-cost clause) that fire simultaneously,
    // so the engine installs a TriggerOrder prompt letting the controller
    // choose resolution order. Deterministically resolve it to "slot 0"
    // (the free-Option-use clause, declared first in the YAML `effects:`
    // list) first, so downstream assertions see a stable clause ordering.
    resolve_trigger_order_preferring_slot(runner, 0);
}

/// If a `TriggerOrder` prompt is pending, resolve it by picking the choice
/// whose label names `preferred_slot` (the compiled clause's zero-based
/// position in the `effects:` list, per `EffectChoiceEntry`'s `"{card_id}
/// slot {effect_slot} (...)"` label format). Falls back to the first choice
/// if the preferred slot isn't present. No-op if no `TriggerOrder` prompt is
/// pending.
fn resolve_trigger_order_preferring_slot(runner: &mut DebugRunner, preferred_slot: u8) {
    if runner.pending_kind() != Some(SelectionKind::TriggerOrder) {
        return;
    }
    let view = runner
        .pending_selection_view()
        .expect("TriggerOrder prompt must carry a view");
    let choices = view
        .effect_choices
        .as_ref()
        .expect("TriggerOrder prompt must carry effect_choices");
    let needle = format!("slot {preferred_slot} ");
    let action_id = choices
        .iter()
        .find(|c| c.label.contains(&needle))
        .or_else(|| choices.first())
        .map(|c| c.action_id)
        .expect("TriggerOrder prompt must offer >= 1 choice");
    runner
        .execute_action(view.selecting_player, action_id)
        .expect("resolve TriggerOrder pick");
}

/// If Clause A's own "you may use 1 Option" prompt (`SelectionKind::Hand`)
/// is currently pending, decline it via PASS. Used by Clause B/C tests that
/// don't care about Clause A and must NOT reach for `auto_resolve()` here —
/// `auto_resolve()` would greedily walk INTO Clause B/C's own `SourceMulti`
/// prompt (its `min: 0` makes candidate picks "legal first actions" too),
/// consuming the very prompt the test wants to inspect.
fn decline_clause_a_if_pending(runner: &mut DebugRunner) {
    if runner.pending_kind() == Some(SelectionKind::Hand) {
        let player = runner
            .pending_selection_view()
            .expect("Hand prompt view")
            .selecting_player;
        runner
            .execute_action(player, PASS)
            .expect("decline Clause A's free-Option-use prompt");
    }
}

/// Compile the card once for structural assertions.
fn compiled_card() -> digimon_dsl::compiled::CompiledCard {
    let spec: digimon_dsl::CardSpec = serde_yml::from_str(YAML).expect("EX7-073.yaml parses");
    let registry =
        digimon_dsl::CardRegistry::from_specs("test", &[spec]).expect("EX7-073.yaml compiles");
    registry
        .lookup(CARD_ID)
        .expect("EX7-073 in registry")
        .clone()
}

// ─────────────────────────────────────────────────────────────────────────────
// SECTION 1 — Structural assertions
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn ex7_073_compiles_with_printed_stats() {
    let card = compiled_card();
    assert_eq!(card.card, CARD_ID);
    assert_eq!(card.name, "BeelStarmon (X Antibody)");
    assert_eq!(card.kind, CompiledCardKind::Digimon);
    assert_eq!(card.level, Some(6));
    assert_eq!(card.dp, Some(12000));
    assert_eq!(card.cost, Some(12));
    assert_eq!(card.color, vec![CompiledColor::Purple]);
    for trait_name in ["Wizard", "X Antibody", "Three Musketeers"] {
        assert!(
            card.traits.iter().any(|t| t == trait_name),
            "missing trait {trait_name}"
        );
    }
}

#[test]
fn ex7_073_has_standard_digivolve_lv5_purple_cost4() {
    let card = compiled_card();
    let path = card
        .alt_paths
        .iter()
        .find(|p| {
            p.kind == CompiledAltPathKind::Digivolve
                && matches!(p.cost, Some(CompiledCost::Literal(4)))
                && p.from
                    .as_ref()
                    .and_then(|f| f.level_eq)
                    .map_or(false, |l| l == 5)
        })
        .expect("EX7-073 must have a Lv.5 Purple digivolve path with cost 4");
    assert!(!path.ignore_requirements);
}

#[test]
fn ex7_073_has_three_musketeers_no_x_antibody_alt_source_lv6_cost1() {
    let card = compiled_card();
    let digi_paths: Vec<_> = card
        .alt_paths
        .iter()
        .filter(|p| p.kind == CompiledAltPathKind::Digivolve)
        .collect();
    assert!(
        digi_paths.len() >= 2,
        "EX7-073 must have >= 2 digivolve alt-paths (standard Purple + Three Musketeers alt-source)"
    );
    let alt_source = digi_paths
        .iter()
        .find(|p| matches!(p.cost, Some(CompiledCost::Literal(1))))
        .expect("EX7-073 must have the cost-1 Three-Musketeers-without-X-Antibody alt-source");
    let from = alt_source
        .from
        .as_ref()
        .expect("alt-source path must carry a `from` predicate");
    // The `from` predicate compiles to an `all_of` compound (level_eq +
    // trait_has + not:{trait_has}); the leaves live inside `all_of`, not on
    // the top-level struct.
    assert!(
        from.all_of.iter().any(|leaf| leaf.level_eq == Some(6)),
        "alt-source requires level 6 (all_of leaves: {:?})",
        from.all_of
    );
    assert!(
        from.all_of
            .iter()
            .any(|leaf| leaf.trait_has.as_deref() == Some("Three Musketeers")),
        "alt-source requires [Three Musketeers] trait"
    );
    assert!(
        from.all_of.iter().any(|leaf| leaf
            .not
            .as_ref()
            .is_some_and(|inner| inner.trait_has.as_deref() == Some("X Antibody"))),
        "alt-source requires the negated [X Antibody] trait check"
    );
}

/// POSITIVE (behavioral): a Lv.6 [Three Musketeers]-trait base WITHOUT
/// [X Antibody] satisfies the cost-1 alt-source and the digivolve succeeds.
#[test]
fn ex7_073_digivolves_via_alt_source_from_three_musketeers_lv6_no_x_antibody_base() {
    let mut runner = base_builder()
        .add_card(three_musketeers_lv6_no_x_antibody("ALT-BASE"))
        .hand(0, &[CARD_ID])
        .deck(0, &["ALT-BASE"; 1])
        .memory(2)
        .start();

    let base = runner.place_on_field(0, "ALT-BASE", Some(0));
    let memory_before = runner.memory();
    assert!(
        runner
            .game
            .digivolve_from_hand(0, 0, base.index as usize, PlaySource::ByHand),
        "EX7-073 must digivolve via the cost-1 alt-source from a Lv.6 \
         [Three Musketeers]-trait base without [X Antibody]"
    );
    assert_eq!(
        runner.game.players[0].battle_area[base.index as usize]
            .top_card()
            .card_id(&runner.game.card_data),
        CARD_ID
    );
    assert_eq!(
        runner.memory(),
        memory_before - 1,
        "the alt-source path costs 1 memory (printed xros_req cost)"
    );
}

/// NEGATIVE (behavioral): a Lv.6 [Three Musketeers]-trait base WITH
/// [X Antibody] does NOT satisfy the alt-source (its printed requirement is
/// explicitly "w/o [X Antibody] trait") and does not satisfy the standard
/// Purple Lv.5 path either (wrong level/color) — the digivolve must fail.
#[test]
fn ex7_073_does_not_digivolve_via_alt_source_from_x_antibody_base() {
    let mut runner = base_builder()
        .add_card(three_musketeers_lv6_with_x_antibody("XA-BASE"))
        .hand(0, &[CARD_ID])
        .deck(0, &["XA-BASE"; 1])
        .memory(2)
        .start();

    let base = runner.place_on_field(0, "XA-BASE", Some(0));
    assert!(
        !runner
            .game
            .digivolve_from_hand(0, 0, base.index as usize, PlaySource::ByHand),
        "EX7-073 must NOT digivolve from a [Three Musketeers]+[X Antibody] \
         base — the alt-source explicitly requires the ABSENCE of [X Antibody]"
    );
}

#[test]
fn ex7_073_has_three_face_up_triggered_clauses() {
    let card = compiled_card();
    let triggered: Vec<_> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) if t.scope == CompiledScope::FaceUp => Some(t),
            _ => None,
        })
        .collect();
    assert_eq!(
        triggered.len(),
        3,
        "EX7-073 has 3 face-up triggered clauses: the free-Option-use clause \
         (WhenDigivolving), the WhenDigivolving trash-cost delete+security \
         clause, and the WhenAttacking trash-cost delete+security clause \
         (two SEPARATE clauses per printed [When Digivolving][When Attacking] \
         multi-timing header, not one clause with two `when` entries — DCGO \
         registers two independent ActivateClass instances)"
    );
}

#[test]
fn ex7_073_clause_a_is_when_digivolving_optional_use_option_from_hand() {
    let card = compiled_card();
    let clause = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t)
                if t.when == vec![CompiledTiming::WhenDigivolving]
                    && t.process.iter().any(|s| {
                        matches!(s, CompiledStep::UseOptionFromHand { .. })
                    }) =>
            {
                Some(t)
            }
            _ => None,
        })
        .expect("Clause A: [When Digivolving] use-Option-from-hand-free clause must exist");
    assert!(
        clause.optional,
        "'you may use 1 Option card ... without paying the cost' must be optional"
    );
}

#[test]
fn ex7_073_clause_b_fires_on_when_digivolving_and_when_attacking() {
    let card = compiled_card();
    let trash_clauses: Vec<_> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t)
                if t.process.iter().any(|s| {
                    matches!(s, CompiledStep::SelectOwnSources { .. })
                }) =>
            {
                Some(t)
            }
            _ => None,
        })
        .collect();
    assert_eq!(
        trash_clauses.len(),
        2,
        "the trash-2-Three-Musketeers-sources body must appear as two \
         separate clauses: one WhenDigivolving, one WhenAttacking"
    );
    assert!(
        trash_clauses
            .iter()
            .any(|c| c.when == vec![CompiledTiming::WhenDigivolving]),
        "one trash-cost clause must be [When Digivolving]"
    );
    assert!(
        trash_clauses
            .iter()
            .any(|c| c.when == vec![CompiledTiming::WhenAttacking]),
        "one trash-cost clause must be [When Attacking]"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// SECTION 2 — Condition gating: Clause A candidate filter (Three Musketeers
// text in hand Options)
// ─────────────────────────────────────────────────────────────────────────────

/// POSITIVE: a hand Option with "Three Musketeers" in its text is a legal
/// pick for the free-use prompt.
#[test]
fn ex7_073_clause_a_offers_three_musketeers_option_in_hand() {
    let mut runner = base_builder()
        .add_card(purple_lv5("BASE"))
        .add_card(three_musketeers_option("TM-OPT", 3))
        .hand(0, &[CARD_ID, "TM-OPT"])
        .deck(0, &["BASE"; 1])
        .memory(10)
        .start();

    let base = runner.place_on_field(0, "BASE", Some(0));
    // Move EX7-073 to hand index 0 (it's already there by construction).
    digivolve_beelstarmon(&mut runner, base);

    let view = runner
        .pending_selection_view()
        .expect("Clause A optional Option-use prompt must install");
    assert_eq!(view.kind, SelectionKind::Hand);
    assert!(view.is_optional, "'you may' must expose PASS");
    assert!(
        view.valid_action_ids.contains(&PLAY_HAND_START),
        "the Three-Musketeers-text Option must be a legal free-use pick"
    );
}

/// NEGATIVE: a hand Option with NO "Three Musketeers" text must not be
/// offered by Clause A (the candidate filter excludes it, but the
/// hand-selection prompt still installs with zero legal picks and
/// remains declineable).
#[test]
fn ex7_073_clause_a_excludes_hand_option_without_three_musketeers_text() {
    let mut runner = base_builder()
        .add_card(purple_lv5("BASE"))
        .add_card(plain_option("PLAIN-OPT", 3))
        .hand(0, &[CARD_ID, "PLAIN-OPT"])
        .deck(0, &["BASE"; 1])
        .memory(10)
        .start();

    let base = runner.place_on_field(0, "BASE", Some(0));
    digivolve_beelstarmon(&mut runner, base);

    // Either no prompt installs (zero candidates -> immediate skip to the
    // next clause) or a declineable prompt installs with the plain Option
    // NOT among the legal picks. Both are faithful outcomes; the invariant
    // under test is that the plain Option is never a legal free-use pick.
    if let Some(view) = runner.pending_selection_view() {
        if view.kind == SelectionKind::Hand {
            assert!(
                !view.valid_action_ids.contains(&PLAY_HAND_START),
                "a non-Three-Musketeers-text Option must never be a legal free-use pick"
            );
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// SECTION 3 — Behavioral outcome: Clause A (use Option from hand free)
// ─────────────────────────────────────────────────────────────────────────────

/// Accepting Clause A plays the chosen Three-Musketeers Option from hand
/// without paying its cost (memory unaffected by the Option's own cost).
#[test]
fn ex7_073_clause_a_accept_plays_option_without_paying_cost() {
    let mut runner = base_builder()
        .add_card(purple_lv5("BASE"))
        .add_card(three_musketeers_option("TM-OPT", 5))
        .hand(0, &[CARD_ID, "TM-OPT"])
        .deck(0, &["BASE"; 1])
        .memory(10)
        .start();

    let base = runner.place_on_field(0, "BASE", Some(0));
    digivolve_beelstarmon(&mut runner, base);

    let memory_before = runner.memory();
    let view = runner
        .pending_selection_view()
        .expect("Clause A prompt must install");
    assert_eq!(view.kind, SelectionKind::Hand);

    runner
        .execute_action(0, PLAY_HAND_START)
        .expect("use the Three Musketeers Option for free");

    assert_eq!(
        runner.memory(),
        memory_before,
        "using the Option 'without paying the cost' must not change memory"
    );
    assert!(
        !runner.game.players[0]
            .hand
            .iter()
            .any(|c| c.card_id(&runner.game.card_data) == "TM-OPT"),
        "the used Option must leave hand"
    );
}

/// Declining Clause A leaves hand and memory untouched.
#[test]
fn ex7_073_clause_a_decline_does_nothing() {
    let mut runner = base_builder()
        .add_card(purple_lv5("BASE"))
        .add_card(three_musketeers_option("TM-OPT", 5))
        .hand(0, &[CARD_ID, "TM-OPT"])
        .deck(0, &["BASE"; 1])
        .memory(10)
        .start();

    let base = runner.place_on_field(0, "BASE", Some(0));
    digivolve_beelstarmon(&mut runner, base);

    let memory_before = runner.memory();
    let hand_before = runner.hand_size(0);
    assert!(runner.pending_is_optional());

    runner
        .execute_action(0, PASS)
        .expect("decline the free Option-use prompt");

    assert_eq!(runner.memory(), memory_before);
    assert_eq!(runner.hand_size(0), hand_before);
}

// ─────────────────────────────────────────────────────────────────────────────
// SECTION 4 — Behavioral outcome: Clause B ([When Digivolving] trash-2 ->
// delete highest-level + trash top security)
// ─────────────────────────────────────────────────────────────────────────────

/// Build a runner with EX7-073 in hand atop a Purple Lv.5 base carrying 2
/// [Three Musketeers]-trait sources beneath it, plus an opponent board with
/// two Digimon at different levels (so "highest level" is unambiguous).
fn wd_trash_builder() -> DebugRunnerBuilder {
    base_builder()
        .add_card(purple_lv5("BASE"))
        .add_card(three_musketeers_source("TM-SRC-1"))
        .add_card(three_musketeers_source("TM-SRC-2"))
        .add_card(opp_digimon("OPP-LOW", 4, 5000))
        .add_card(opp_digimon("OPP-HIGH", 6, 9000))
        .add_card(make_test_card("SEC-FILLER", "SecFiller"))
        .hand(0, &[CARD_ID])
        .deck(0, &["BASE"; 1])
        .security(1, &["SEC-FILLER", "SEC-FILLER", "SEC-FILLER"])
        .memory(10)
}

/// Place a base stack with 2 Three-Musketeers-trait sources beneath the top
/// (BASE), place both opponent Digimon on the field, then digivolve
/// EX7-073 onto the base. Returns the stacked handle.
fn place_stacked_base_and_digivolve(runner: &mut DebugRunner) -> PermanentHandle {
    runner.place_on_field(1, "OPP-LOW", Some(0));
    runner.place_on_field(1, "OPP-HIGH", Some(0));
    let base = runner.place_stack(0, &["TM-SRC-1", "TM-SRC-2", "BASE"]);
    digivolve_beelstarmon(runner, base);
    base
}

/// POSITIVE: with 2 [Three Musketeers] sources available and 2 distinct-level
/// opponent Digimon, the player can trash both sources, is then offered a
/// single-pick over the highest-level opponent Digimon (only), and after
/// deleting it the opponent's top security is trashed.
#[test]
fn ex7_073_wd_trash_two_sources_deletes_highest_level_and_trashes_top_security() {
    let mut runner = wd_trash_builder().start();
    let base = place_stacked_base_and_digivolve(&mut runner);

    // Clause A's optional Option-use prompt has nothing to offer (no hand
    // Options) — auto_resolve past any zero-candidate declineable prompt.
    decline_clause_a_if_pending(&mut runner);

    let sec_before = runner.security_count(1);
    let opp_before = runner.battle_area_size(1);
    assert_eq!(opp_before, 2, "fixture: 2 opp Digimon on field");

    let view = runner
        .pending_selection_view()
        .expect("Clause B source-trash prompt must install");
    assert!(
        matches!(view.kind, SelectionKind::SourceMulti { min: 0, max: 2, .. }),
        "expected an up-to-2 own-source trash prompt, got {:?}",
        view.kind
    );
    assert!(
        view.is_optional,
        "min:0 means PASS is legal from the start (DCGO canNoSelect: true)"
    );

    // Source candidates keep their STABLE original `source_index` within the
    // stack (0 = TM-SRC-1, 1 = TM-SRC-2) even after an earlier pick removes
    // one from the candidate list — see `source_multi_candidates`, which
    // filters already-picked cards out by identity rather than
    // renumbering the survivors.
    let base_field = base.index as u16;
    let pick_1 = encode_source_select(base_field, 0).expect("source slot 0 encodes");
    runner
        .execute_action(0, pick_1)
        .expect("pick the first Three Musketeers source");
    let pick_2 = encode_source_select(base_field, 1).expect("source slot 1 encodes");
    runner
        .execute_action(0, pick_2)
        .expect("pick the second Three Musketeers source");

    // With exactly 2 trashed, the opponent-Digimon-delete prompt must
    // install, restricted to the highest-level candidate only.
    let target_view = runner
        .pending_selection_view()
        .expect("delete-highest-level prompt must install after 2 sources trashed");
    assert_eq!(target_view.kind, SelectionKind::OppField);
    assert_eq!(
        target_view.valid_action_ids.len(),
        1,
        "only the unique highest-level opponent Digimon should be selectable"
    );
    runner
        .execute_action(0, target_view.valid_action_ids[0])
        .expect("delete the highest-level opponent Digimon");

    runner.auto_resolve().expect("finish Clause B");

    // The two Three Musketeers sources are gone from the stack (only the
    // digivolve foundation "BASE" and the EX7-073 top card remain — BASE
    // itself carries no [Three Musketeers] trait so it is never a
    // candidate) and both are now in trash.
    assert_eq!(
        runner.game.players[0].battle_area[base.index as usize]
            .card_sources
            .len(),
        2,
        "the stack shrinks from 4 (TM-SRC-1, TM-SRC-2, BASE, EX7-073) to 2 (BASE, EX7-073)"
    );
    let trash_ids = zone_ids(&runner.game.players[0].trash, &runner.game.card_data);
    assert!(trash_ids.contains(&"TM-SRC-1".to_string()));
    assert!(trash_ids.contains(&"TM-SRC-2".to_string()));

    // OPP-HIGH (level 6, the highest) is deleted; OPP-LOW (level 4) remains.
    assert_eq!(
        runner.battle_area_size(1),
        opp_before - 1,
        "exactly 1 opponent Digimon deleted"
    );
    assert!(
        !runner.game.players[1]
            .battle_area
            .iter()
            .any(|p| p.top_card().card_id(&runner.game.card_data) == "OPP-HIGH"),
        "the highest-level opponent Digimon (OPP-HIGH) must be deleted"
    );
    assert!(
        runner.game.players[1]
            .battle_area
            .iter()
            .any(|p| p.top_card().card_id(&runner.game.card_data) == "OPP-LOW"),
        "the lower-level opponent Digimon (OPP-LOW) must remain"
    );

    // Opponent's top security card is trashed.
    assert_eq!(
        runner.security_count(1),
        sec_before - 1,
        "trashing 2 sources must also trash the opponent's top security card"
    );
}

/// NEGATIVE: declining the trash-cost prompt (PASS at 0 picks) does nothing
/// — no source trashed, no opponent Digimon deleted, no security lost.
#[test]
fn ex7_073_wd_decline_trash_cost_does_nothing() {
    let mut runner = wd_trash_builder().start();
    let base = place_stacked_base_and_digivolve(&mut runner);
    decline_clause_a_if_pending(&mut runner);

    let sec_before = runner.security_count(1);
    let opp_before = runner.battle_area_size(1);
    let sources_before = runner.game.players[0].battle_area[base.index as usize]
        .card_sources
        .len();

    assert!(runner.pending_is_optional());
    runner
        .execute_action(0, PASS)
        .expect("decline the trash-cost prompt immediately");

    runner.auto_resolve().expect("no further prompts remain");

    assert_eq!(
        runner.game.players[0].battle_area[base.index as usize]
            .card_sources
            .len(),
        sources_before,
        "declining must not trash any digivolution source"
    );
    assert_eq!(
        runner.battle_area_size(1),
        opp_before,
        "declining must not delete any opponent Digimon"
    );
    assert_eq!(
        runner.security_count(1),
        sec_before,
        "declining must not trash opponent security"
    );
}

/// NEGATIVE: partial commit (pick exactly 1 of the 2 required Three
/// Musketeers sources, then decline) still results in NO deletion and NO
/// security loss — DCGO's `if (selectedCards.Count >= 2)` gate means a
/// 1-card pick never fires the tail, even though the 1 picked card IS
/// trashed as part of the (declined-early) multi-select.
#[test]
fn ex7_073_wd_picking_only_one_source_then_declining_does_not_trigger_delete_or_security() {
    let mut runner = wd_trash_builder().start();
    let base = place_stacked_base_and_digivolve(&mut runner);
    decline_clause_a_if_pending(&mut runner);

    let sec_before = runner.security_count(1);
    let opp_before = runner.battle_area_size(1);

    let base_field = base.index as u16;
    let pick_1 = encode_source_select(base_field, 0).expect("source slot 0 encodes");
    runner
        .execute_action(0, pick_1)
        .expect("pick only 1 of the 2 sources");

    // PASS should still be legal (min: 0 satisfied) after only 1 pick.
    let view = runner
        .pending_selection_view()
        .expect("prompt remains after 1 of 2 picks");
    assert!(
        view.valid_action_ids.contains(&PASS),
        "PASS must remain legal after only 1 pick (min: 0)"
    );
    runner
        .execute_action(0, PASS)
        .expect("decline after only 1 pick");
    runner.auto_resolve().expect("no further prompts remain");

    assert_eq!(
        runner.battle_area_size(1),
        opp_before,
        "a 1-card pick must NOT trigger the delete (DCGO requires exactly 2 trashed)"
    );
    assert_eq!(
        runner.security_count(1),
        sec_before,
        "a 1-card pick must NOT trigger the security trash"
    );
}

/// NEGATIVE: a digivolution source WITHOUT the [Three Musketeers] trait must
/// never be offered as a trash candidate.
#[test]
fn ex7_073_wd_trash_candidates_exclude_non_three_musketeers_sources() {
    let mut runner = base_builder()
        .add_card(purple_lv5("BASE"))
        .add_card(three_musketeers_source("TM-SRC-1"))
        .add_card(plain_source("PLAIN-SRC"))
        .add_card(opp_digimon("OPP-A", 5, 5000))
        .add_card(make_test_card("SEC-FILLER", "SecFiller"))
        .hand(0, &[CARD_ID])
        .deck(0, &["BASE"; 1])
        .security(1, &["SEC-FILLER"])
        .memory(10)
        .start();

    let base = runner.place_stack(0, &["TM-SRC-1", "PLAIN-SRC", "BASE"]);
    digivolve_beelstarmon(&mut runner, base);
    decline_clause_a_if_pending(&mut runner);

    let view = runner
        .pending_selection_view()
        .expect("Clause B source-trash prompt must install");
    assert!(
        matches!(view.kind, SelectionKind::SourceMulti { min: 0, max: 2, .. })
    );
    // Only 1 matching candidate (TM-SRC-1) can ever be picked; the picker
    // must not expose PLAIN-SRC's slot as a legal action.
    assert_eq!(
        view.valid_action_ids.len(),
        2,
        "exactly 1 matching candidate + PASS should be legal"
    );
    assert!(view.valid_action_ids.contains(&PASS));
}

// ─────────────────────────────────────────────────────────────────────────────
// SECTION 5 — Behavioral outcome: Clause C ([When Attacking] trash-2 ->
// delete highest-level + trash top security)
// ─────────────────────────────────────────────────────────────────────────────

/// Build a runner with EX7-073 already on the field (not digivolved this
/// turn) carrying 2 [Three Musketeers] sources, ready to attack.
fn attack_builder() -> DebugRunnerBuilder {
    base_builder()
        .add_card(three_musketeers_source("TM-SRC-1"))
        .add_card(three_musketeers_source("TM-SRC-2"))
        .add_card(opp_digimon("OPP-LOW", 4, 5000))
        .add_card(opp_digimon("OPP-HIGH", 6, 9000))
        .add_card(make_test_card("DEF-FILLER", "DefFiller"))
        .add_card(make_test_card("SEC-FILLER", "SecFiller"))
        .security(1, &["SEC-FILLER", "SEC-FILLER", "SEC-FILLER"])
        .memory(10)
}

/// POSITIVE: attacking with EX7-073 (2 Three Musketeers sources beneath it)
/// offers the same trash-2 -> delete-highest + trash-security effect.
/// Attacks DEF-FILLER (not OPP-LOW/OPP-HIGH) so combat resolution doesn't
/// itself delete one of the two level-tracked opponents — isolating the
/// card-effect's own delete from ordinary combat DP comparison.
#[test]
fn ex7_073_when_attacking_trash_two_sources_deletes_highest_level_and_trashes_security() {
    let mut runner = attack_builder().start();
    runner.game.turn_count = 1;

    let attacker = runner.place_stack(0, &["TM-SRC-1", "TM-SRC-2", CARD_ID]);
    let defender = runner.place_on_field(1, "DEF-FILLER", Some(0));
    runner.place_on_field(1, "OPP-LOW", Some(0));
    runner.place_on_field(1, "OPP-HIGH", Some(0));

    let sec_before = runner.security_count(1);
    let opp_before = runner.battle_area_size(1);

    runner.attack_digimon(attacker, defender, false);

    let view = runner
        .pending_selection_view()
        .expect("[When Attacking] source-trash prompt must install");
    assert!(matches!(
        view.kind,
        SelectionKind::SourceMulti { min: 0, max: 2, .. }
    ));

    let attacker_field = attacker.index as u16;
    let pick_1 =
        encode_source_select(attacker_field, 0).expect("attacker source slot 0 encodes");
    runner
        .execute_action(0, pick_1)
        .expect("pick the first Three Musketeers source");
    let pick_2 =
        encode_source_select(attacker_field, 1).expect("attacker source slot 1 encodes");
    runner
        .execute_action(0, pick_2)
        .expect("pick the second Three Musketeers source");

    let target_view = runner
        .pending_selection_view()
        .expect("delete-highest-level prompt must install");
    assert_eq!(target_view.kind, SelectionKind::OppField);
    assert_eq!(
        target_view.valid_action_ids.len(),
        1,
        "only OPP-HIGH (level 6, the unique highest) is a legal delete target \
         (DEF-FILLER level 3, OPP-LOW level 4 are excluded)"
    );
    runner
        .execute_action(0, target_view.valid_action_ids[0])
        .expect("delete the highest-level opponent Digimon");
    runner.auto_resolve().expect("finish Clause C");

    // 3 opponents on field before: DEF-FILLER (dies to combat, DP 2000 vs
    // 12000), OPP-LOW (level 4, survives), OPP-HIGH (level 6, deleted by
    // the card effect). Net: opp_before - 2 remain (just OPP-LOW).
    assert_eq!(opp_before, 3, "fixture: DEF-FILLER + OPP-LOW + OPP-HIGH");
    assert_eq!(
        runner.battle_area_size(1),
        opp_before - 2,
        "DEF-FILLER dies to combat, OPP-HIGH dies to the card effect, OPP-LOW survives"
    );
    assert!(
        runner.game.players[1]
            .battle_area
            .iter()
            .any(|p| p.top_card().card_id(&runner.game.card_data) == "OPP-LOW"),
        "OPP-LOW (not the highest level) must remain"
    );
    assert!(
        !runner.game.players[1]
            .battle_area
            .iter()
            .any(|p| p.top_card().card_id(&runner.game.card_data) == "OPP-HIGH"),
        "OPP-HIGH (the highest level) must be deleted by the card effect"
    );
    assert_eq!(runner.security_count(1), sec_before - 1);
}

/// OPT-adjacent: [When Attacking] and [When Digivolving] are independent
/// activation windows (DCGO installs 2 distinct `ActivateClass`/timing
/// pairs). Triggering the [When Attacking] clause in the SAME turn as an
/// already-resolved [When Digivolving] clause must still offer the
/// trash-cost prompt (no shared once-per-turn lock across the two timings).
#[test]
fn ex7_073_when_digivolving_and_when_attacking_are_independent_activation_windows() {
    let mut runner = wd_trash_builder()
        .add_card(make_test_card("DEF-FILLER", "DefFiller"))
        .start();
    let base = place_stacked_base_and_digivolve(&mut runner);
    decline_clause_a_if_pending(&mut runner);

    // Decline Clause B (WhenDigivolving) immediately.
    assert!(runner.pending_is_optional());
    runner
        .execute_action(0, PASS)
        .expect("decline the WhenDigivolving trash-cost prompt");
    runner.auto_resolve().expect("no further prompts remain");

    // Attacking the SAME turn must still offer Clause C's independent
    // trash-cost prompt (the WhenDigivolving decline does not lock it out).
    let defender = runner.place_on_field(1, "OPP-LOW", Some(0));
    runner.attack_digimon(base, defender, false);

    let view = runner.pending_selection_view();
    assert!(
        view.is_some(),
        "[When Attacking]'s independent trash-cost prompt must still install \
         after declining [When Digivolving]'s"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// SECTION 6 — Event-log assertions (cost-firing: source trash + security
// trash both move cards to the trash zone)
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn ex7_073_wd_trash_cost_and_security_loss_fire_trash_events() {
    let mut runner = wd_trash_builder().start();
    let base = place_stacked_base_and_digivolve(&mut runner);
    decline_clause_a_if_pending(&mut runner);

    let cp = runner.event_checkpoint();

    let base_field = base.index as u16;
    let pick_1 = encode_source_select(base_field, 0).expect("source slot 0 encodes");
    runner.execute_action(0, pick_1).expect("pick source 1");
    let pick_2 = encode_source_select(base_field, 1).expect("source slot 1 encodes");
    runner.execute_action(0, pick_2).expect("pick source 2");

    let target_view = runner
        .pending_selection_view()
        .expect("delete-highest-level prompt must install");
    let deleted_id = {
        // Identify which opponent Digimon is about to be deleted, by
        // decoding the (sole) legal action's field index. `OppField`
        // selections reuse the attack action-ID encoding via
        // `encode_attack(0, field_index)` (`EffectContext::
        // install_field_selection`), so the FIELD index is the `target`
        // half of `decode_attack`'s `(attacker, target)` pair, not the
        // (always-0 placeholder) `attacker` half.
        use digimon_engine::action::space::decode_attack;
        let (_, field) = decode_attack(target_view.valid_action_ids[0]);
        runner.game.players[1].battle_area[field as usize]
            .top_card()
            .card_id(&runner.game.card_data)
            .to_string()
    };
    runner
        .execute_action(0, target_view.valid_action_ids[0])
        .expect("delete the highest-level opponent Digimon");
    runner.auto_resolve().expect("finish Clause B");

    // NOTE on source-trashing / security-trashing and the event log:
    // `trash_card_source` (backing `trash_selected_sources`) routes through
    // `Game::trash_source_and_fire`, and `trash_top_security` routes through
    // `complete_effect_security_removal` — BOTH push directly onto
    // `player.trash` without calling `Game::trash_card`, so NEITHER emits a
    // `GameEvent::Trash` (only `Game::trash_card` / `trash_permanent_stack`
    // emit that event — see `code/digimon-engine/src/game_actions/helpers.rs`
    // and `code/digimon-engine/src/effect_queue.rs::complete_effect_security_removal`).
    // The single faithfully-observable `GameEvent::Trash` for this clause
    // comes from `delete_permanent`, which DOES route through
    // `trash_permanent_stack`. Confirm the source-trash and security-trash
    // cost payments via direct trash-zone state inspection (not the event
    // log), and confirm the deletion's `GameEvent::Trash` via the event log.
    let sources_trashed = zone_ids(&runner.game.players[0].trash, &runner.game.card_data);
    assert!(sources_trashed.contains(&"TM-SRC-1".to_string()));
    assert!(sources_trashed.contains(&"TM-SRC-2".to_string()));
    let opp_trashed = zone_ids(&runner.game.players[1].trash, &runner.game.card_data);
    assert!(
        opp_trashed.contains(&"SEC-FILLER".to_string()),
        "the opponent's top security card must land in their trash"
    );

    let events = runner.events_since(cp);
    let trashed_ids: Vec<&str> = events
        .iter()
        .filter_map(|e| match e {
            GameEvent::Trash { card_id, .. } => Some(card_id.as_str()),
            _ => None,
        })
        .collect();
    assert!(
        trashed_ids.contains(&deleted_id.as_str()),
        "deleting the highest-level opponent Digimon must emit a GameEvent::Trash; got {trashed_ids:?}"
    );
}
