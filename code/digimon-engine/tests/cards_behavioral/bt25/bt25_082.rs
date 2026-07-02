//! BT25-082 BlackGatomon — Digimon, Lv.4, Purple/Black, DP 6000, Cost 6.
//! Traits: Dark Animal, Iliad, TS. Attribute: Virus.
//! Evo (printed): Lv.3 Purple / Cost 3 (+ DCGO self-digivolution-requirement on
//! a Lv.3 base whose top card has [Three Musketeers] text OR the [TS] trait,
//! cost 2 — printed xros_req box).
//!
//! # Card text (cards.json — verbatim)
//!
//! [On Play] [When Digivolving] If you have 1 or fewer Tamers, you may play 1
//! Tamer card with [Three Musketeers] in its text from your hand without paying
//! the cost.
//! [All Turns] While you have a Tamer with [Three Musketeers] in its text, this
//! Digimon may digivolve into a [Three Musketeers] trait Digimon card for a
//! digivolution cost of 4, ignoring digivolution requirements.
//!
//! Inherited Effect:
//! [When Attacking] [Once Per Turn] By placing 1 [Three Musketeers] trait card
//! from your hand or trash as this Digimon's bottom digivolution card,
//! ＜Draw 1＞ (Draw 1 card from your deck.)
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT25/Purple/BT25_082.cs
//!
//! - Region "Digivolution Condition" (None): AddSelfDigivolutionRequirement-
//!   StaticEffect(level: 3, permanentCondition: TopCard.HasText("Three
//!   Musketeers") || TopCard.EqualsTraits("TS"), digivolutionCost: 2,
//!   ignoreDigivolutionRequirement: false). This is the printed xros_req box.
//! - Region "Shared OP/WD": ActivateClassesForSharedEffects(onPlay: true,
//!   whenDigivolving: true, optional: false) with AdditionalActivateCondition =
//!   owner has a TM-text Tamer in HAND && owner battle-area Tamer count <= 1.
//!   Body: SelectHandEffect over TM-text Tamers, canNoSelect: true, mode
//!   PlayForFree.
//! - Region "All Turns" (None): AddSelfDigivolutionRequirementStaticEffect(
//!   permanentCondition: target == self, cardCondition: IsDigimon &&
//!   EqualsTraits("Three Musketeers"), digivolutionCost: 4,
//!   ignoreDigivolutionRequirement: true, condition: self-on-field && owner has
//!   a TM-text Tamer on battle area). The "into" semantic: this Digimon (source)
//!   digivolves into a TM-trait Digimon card in hand for cost 4.
//! - Region "Inherited When Attacking" (OnAllyAttack): inherited, oncePerTurn 1.
//!   Body: choose hand-or-trash, SelectHand/SelectCard over TM-trait cards,
//!   AddDigivolutionCardsBottom(selected) then Draw 1.
//!
//! # Patterns this test covers (RUST_DSL_TEST_API §4.3)
//! - B2 tamer-play anchor (play TM-text Tamer free from hand)
//! - E2 optional + condition gate ("you may", "1 or fewer Tamers")
//! - H/G alt digivolution path (direction: into, conditional)
//! - G4 inherited [When Attacking] OPT + cost (place source) + Draw

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledCost, CompiledScope, CompiledStep, CompiledTiming,
};
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardKind, PlayerId};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::SelectionKind;

const CARD_ID: &str = "BT25-082";

// ─── Fixture helpers ─────────────────────────────────────────────────────────

/// A Tamer whose printed effect text contains the given substring. We feed the
/// "Three Musketeers" text into `effect_text` so the `effect_text_contains`
/// predicate matches (or not, for the negative fixture).
fn make_tamer(id: &str, effect_text: &str) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Tamer;
    card.level = None;
    card.dp = None;
    card.effect_text = effect_text.to_string();
    card
}

/// A Digimon with the given traits. Used both for inherited-source TM cards and
/// as a digivolution-into target.
fn make_digimon(id: &str, level: u8, traits: &[&str]) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.level = Some(level);
    card.dp = Some(5000);
    card.traits = traits.iter().map(|t| t.to_string()).collect();
    card.evo_costs = vec![EvoCost {
        card_color: 0,
        level: level.saturating_sub(1),
        memory_cost: 2,
    }];
    card
}

fn base() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT25-082 YAML parses and compiles")
        .add_card(make_test_card("DECK-PAD", "Filler"))
        .add_card(make_tamer(
            "TM-TAMER",
            "[Three Musketeers] Once per turn ...",
        ))
        .add_card(make_tamer("PLAIN-TAMER", "A boring tamer with no keywords"))
        .add_card(make_digimon("TM-DIGI", 4, &["Three Musketeers"]))
        .add_card(make_digimon("TM-DIGI-LV5", 5, &["Three Musketeers"]))
        .add_card(make_digimon("PLAIN-DIGI", 4, &["Beast"]))
        .deck(0, &["DECK-PAD"; 10])
        .deck(1, &["DECK-PAD"; 10])
}

/// Place BlackGatomon on P0's field as a standalone face-up Digimon.
fn place_blackgatomon(runner: &mut DebugRunner) -> PermanentHandle {
    runner.place_on_field(0, CARD_ID, Some(0))
}

// ─── Section 1 — Structural assertions ───────────────────────────────────────

#[test]
fn bt25_082_yaml_has_printed_metadata() {
    let runner = base().start();
    let card = runner
        .compiled_card(CARD_ID)
        .expect("BT25-082 must be present in embedded DSL pack");

    assert_eq!(card.name, "BlackGatomon");
    assert_eq!(card.level, Some(4));
    assert_eq!(card.cost, Some(6));
    assert_eq!(card.dp, Some(6000));
    for trait_name in ["Dark Animal", "Iliad", "TS"] {
        assert!(
            card.traits.contains(&trait_name.to_string()),
            "BT25-082 metadata must include trait {trait_name}"
        );
    }
}

#[test]
fn bt25_082_has_conditional_into_alt_path_cost4() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled present");

    // The [All Turns] clause is the `direction: into` digivolve alt-path:
    // cost 4, ignore_requirements, and a `condition` (have a TM-text Tamer).
    let path = card
        .alt_paths
        .iter()
        .find(|p| {
            p.kind == CompiledAltPathKind::Digivolve && p.cost == Some(CompiledCost::Literal(4))
        })
        .expect("BT25-082 must include a cost-4 digivolve alt-path (the [All Turns] into-path)");

    assert!(
        path.ignore_requirements,
        "the [All Turns] into-path ignores digivolution requirements"
    );
    assert!(
        path.condition.is_some(),
        "the [All Turns] into-path is gated by 'while you have a TM-text Tamer'"
    );
    // `from:` filters the destination card (the TM-trait Digimon).
    let from = path.from.as_ref().expect("into-path has a from: filter");
    assert_eq!(
        from.trait_has.as_deref(),
        Some("Three Musketeers"),
        "into-path destination must be a [Three Musketeers] trait Digimon"
    );
}

#[test]
fn bt25_082_op_wd_clause_is_optional_and_tamer_gated() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled present");

    let clause = card
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
        .expect("BT25-082 must have an [On Play][When Digivolving] clause");

    assert_eq!(clause.scope, CompiledScope::FaceUp);
    assert!(
        clause.optional,
        "printed 'you may' → clause must be optional"
    );
    // The body plays a Tamer for free from hand.
    let has_free_play = clause
        .process
        .iter()
        .any(|s| matches!(s, CompiledStep::PlayFromHandFree { .. }));
    assert!(
        has_free_play,
        "OP/WD clause body must contain a play_from_hand_free step"
    );
}

#[test]
fn bt25_082_has_inherited_when_attacking_opt_clause() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled present");

    let clause = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t)
                if t.scope == CompiledScope::Inherited
                    && t.when.contains(&CompiledTiming::WhenAttacking) =>
            {
                Some(t)
            }
            _ => None,
        })
        .expect("BT25-082 must have an inherited [When Attacking] clause");

    assert!(
        clause.once_per_turn,
        "printed [Once Per Turn] → once_per_turn must be true"
    );
    let has_bottom_source = clause
        .process
        .iter()
        .any(|s| matches!(s, CompiledStep::PlaceAsBottomSource { .. }));
    let has_draw = clause
        .process
        .iter()
        .any(|s| matches!(s, CompiledStep::Draw { .. }));
    assert!(
        has_bottom_source,
        "inherited WA body must place a card as a bottom source"
    );
    assert!(has_draw, "inherited WA body must Draw 1");
}

// ─── Section 2 — Behavior: [On Play][When Digivolving] play TM-text Tamer free ─

/// Positive: 0 Tamers, a TM-text Tamer in hand → on play the clause installs an
/// optional prompt; accepting + picking plays the Tamer for free.
#[test]
fn bt25_082_op_plays_tm_text_tamer_free_from_hand() {
    let mut runner = base().hand(0, &[CARD_ID, "TM-TAMER"]).memory(8).start();

    let tamers_before = runner.game.players[0]
        .battle_area
        .iter()
        .filter(|p| p.is_tamer(&runner.game.card_data))
        .count();
    assert_eq!(tamers_before, 0, "no Tamers on field at start");

    // Play BlackGatomon from hand (index 0) — its OnPlay should fire.
    let _bg = runner.play(0, 0).expect("play BlackGatomon");

    assert!(
        runner.pending_selection().is_some(),
        "OnPlay clause must install the optional 'may play a Tamer' prompt"
    );
    runner
        .accept_optional_trigger()
        .expect("accept the optional OnPlay trigger");
    runner
        .auto_resolve()
        .expect("pick the TM Tamer + free-play it");

    let tamers_after = runner.game.players[0]
        .battle_area
        .iter()
        .filter(|p| p.is_tamer(&runner.game.card_data))
        .count();
    assert_eq!(
        tamers_after, 1,
        "the TM-text Tamer must now be on the battle area"
    );
}

/// Negative: 2 Tamers already on field → the "1 or fewer Tamers" gate blocks the
/// clause entirely (no prompt).
#[test]
fn bt25_082_op_blocked_with_two_tamers() {
    let mut runner = base().hand(0, &[CARD_ID, "TM-TAMER"]).memory(8).start();
    // Seed two Tamers onto P0's field.
    runner.place_on_field(0, "PLAIN-TAMER", Some(0));
    runner.place_on_field(0, "PLAIN-TAMER", Some(0));

    let _bg = runner.play(0, 0).expect("play BlackGatomon");

    assert!(
        runner.pending_selection().is_none(),
        "with 2 Tamers, the '1 or fewer Tamers' gate blocks the OnPlay clause"
    );
}

/// Negative: ≤1 Tamer but only a plain (non-TM-text) Tamer in hand → no eligible
/// target, clause is a no-op.
#[test]
fn bt25_082_op_no_fire_without_tm_text_tamer_in_hand() {
    let mut runner = base().hand(0, &[CARD_ID, "PLAIN-TAMER"]).memory(8).start();

    let _bg = runner.play(0, 0).expect("play BlackGatomon");

    assert!(
        runner.pending_selection().is_none(),
        "no [Three Musketeers]-text Tamer in hand → OnPlay clause is a no-op"
    );
}

/// Optional decline: the player may decline the OnPlay trigger.
#[test]
fn bt25_082_op_decline_keeps_field_unchanged() {
    let mut runner = base().hand(0, &[CARD_ID, "TM-TAMER"]).memory(8).start();

    let _bg = runner.play(0, 0).expect("play BlackGatomon");
    assert!(runner.pending_selection().is_some(), "prompt installed");

    runner
        .decline_optional_trigger()
        .expect("declining the optional trigger is legal");

    let tamers_after = runner.game.players[0]
        .battle_area
        .iter()
        .filter(|p| p.is_tamer(&runner.game.card_data))
        .count();
    assert_eq!(tamers_after, 0, "declining plays no Tamer");
}

// ─── Section 3 — Behavior: inherited [When Attacking] place source + Draw ─────

/// Positive: BlackGatomon (as a source under a carrier) is attacking; a TM-trait
/// card in hand can be placed as the carrier's bottom source, then Draw 1.
#[test]
fn bt25_082_inherited_wa_places_tm_card_and_draws() {
    let mut carrier = make_digimon("CARRIER", 5, &["Dark Animal"]);
    carrier.dp = Some(7000);

    let mut runner = base()
        .add_card(carrier)
        .hand(0, &["TM-DIGI"])
        .deck(0, &["DECK-PAD"; 10])
        .memory(10)
        .start();

    // Stack: BlackGatomon UNDER CARRIER, so BlackGatomon's inherited WA fires
    // when CARRIER attacks.
    let stack = runner.place_stack(0, &[CARD_ID, "CARRIER"]);
    let defender = runner.place_on_field(1, "PLAIN-DIGI", Some(0));

    let hand_before = runner.hand_size(0);
    let sources_before = runner.game.players[0].battle_area[stack.index as usize]
        .card_sources
        .len();

    runner.attack_digimon(stack, defender, false);

    assert!(
        runner.pending_selection().is_some(),
        "inherited WA must install a prompt to place a TM card as a bottom source"
    );
    runner
        .accept_optional_trigger()
        .expect("accept the inherited WA trigger");
    runner.auto_resolve().expect("place + draw resolves");

    let sources_after = runner.game.players[0].battle_area[stack.index as usize]
        .card_sources
        .len();
    assert_eq!(
        sources_after,
        sources_before + 1,
        "the TM card from hand must be added as a bottom digivolution source"
    );
    assert_eq!(
        runner.hand_size(0),
        hand_before, // -1 placed as source, +1 drawn = net 0
        "place 1 from hand (-1) then Draw 1 (+1) nets the same hand size"
    );
}

/// Negative: no TM-trait card in hand or trash → the inherited WA clause is a
/// no-op (the cost cannot be paid).
#[test]
fn bt25_082_inherited_wa_no_fire_without_tm_card() {
    let mut carrier = make_digimon("CARRIER", 5, &["Dark Animal"]);
    carrier.dp = Some(7000);

    let mut runner = base()
        .add_card(carrier)
        .hand(0, &["PLAIN-DIGI"])
        .memory(10)
        .start();

    let stack = runner.place_stack(0, &[CARD_ID, "CARRIER"]);
    let defender = runner.place_on_field(1, "PLAIN-DIGI", Some(0));

    runner.attack_digimon(stack, defender, false);

    assert!(
        runner.pending_selection().is_none(),
        "no [Three Musketeers] card in hand or trash → inherited WA is a no-op"
    );
}

/// OPT lockout: the inherited WA clause is [Once Per Turn]; a second attack the
/// same turn must not re-install the prompt.
#[test]
fn bt25_082_inherited_wa_opt_locks_second_attack() {
    let mut carrier = make_digimon("CARRIER", 5, &["Dark Animal"]);
    carrier.dp = Some(7000);

    let mut runner = base()
        .add_card(carrier)
        .hand(0, &["TM-DIGI", "TM-DIGI-LV5"])
        .deck(0, &["DECK-PAD"; 10])
        .memory(10)
        .start();

    let stack = runner.place_stack(0, &[CARD_ID, "CARRIER"]);
    let defender = runner.place_on_field(1, "PLAIN-DIGI", Some(0));
    let defender2 = runner.place_on_field(1, "PLAIN-DIGI", Some(0));

    runner.attack_digimon(stack, defender, false);
    assert!(
        runner.pending_selection().is_some(),
        "first WA installs prompt"
    );
    runner.accept_optional_trigger().expect("accept");
    runner.auto_resolve().expect("resolve first WA");

    // Unsuspend the attacker so it can attack again, then attack the 2nd target.
    runner.game.players[0].battle_area[stack.index as usize].is_suspended = false;
    runner.attack_digimon(stack, defender2, false);

    assert!(
        runner.pending_selection().is_none(),
        "[Once Per Turn] must lock the inherited WA on the second attack this turn"
    );
}
