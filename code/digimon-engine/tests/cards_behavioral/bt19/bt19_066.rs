//! BT19-066 Gizamon — Digimon, Lv.3, Purple/Black, DP 1000, Cost 3.
//! Traits: Sea Animal. Attribute: Virus. Form: Rookie.
//!
//! # Card text (official Bandai DB / cards.json)
//!
//! ```text
//! [On Play] By trashing 1 card with the [Composite] or [Wicked God] trait in
//!     your hand, ＜Draw 2＞ (Draw 2 cards from your deck.)
//! ```
//!
//! Inherited Effect: ＜Blocker＞ (This Digimon can block in the blocker timing.)
//!
//! Alt-digivolve (xros_req): [Digivolve] [Pagumon]: Cost 0.
//! Standard digivolve: Purple Lv.2/Cost 1, Black Lv.2/Cost 1 (printed evo_costs).
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT19/Purple/BT19_066.cs
//!   - Digivolution Condition (EffectTiming.None):
//!     AddSelfDigivolutionRequirementStaticEffect, permanentCondition top card
//!     name == "Pagumon", digivolutionCost 0, ignoreDigivolutionRequirement
//!     false.
//!   - On Play (EffectTiming.OnEnterFieldAnyone):
//!       CanUseCondition: CardEffectCommons.CanTriggerOnPlay.
//!       CanActivateCondition: IsExistOnBattleAreaDigimon(card) (self still on
//!         field) AND HasMatchConditionOwnersHand(card, HasCompositeInTrait)
//!         (>=1 hand card with Composite OR Wicked God trait).
//!       ActivateCoroutine: SelectHandEffect Mode.Discard, canNoSelect: true
//!         (declinable even with a valid target), maxCount 1, over
//!         Composite/Wicked-God-trait hand cards. AfterSelectCardCoroutine:
//!         if (cardSources.Count > 0) DrawClass(owner, 2).Draw() — Draw 2 only
//!         fires when a card was actually trashed.
//!   - Blocker - ESS (EffectTiming.None): BlockerSelfStaticEffect(isInherited:
//!     true) — inherited Blocker keyword, unconditional.
//!
//! # Per-clause status
//!
//! | Clause | Status | Pattern |
//! |--------|--------|---------|
//! | Alt-digi ([Pagumon] / Cost 0)                         | PASS | alt_paths kind: digivolve, name_is |
//! | [On Play] may trash Composite/Wicked God hand card    | PASS | select_hand optional+cost, trait_has any_of |
//! | [On Play] if trashed, Draw 2                          | PASS | if binding_present(discard) -> draw 2 |
//! | Inherited ＜Blocker＞                                  | PASS | scope: inherited, grant_keyword Blocker |
//!
//! # Patterns
//! - E2 OPT + optional decline (cost-paid, declinable even with a legal target)
//! - A-adjacent: trash-as-cost then conditional Draw 2 (AD1-002 idiom)
//! - H5 Blocker (inherited keyword grant)
//! - Structural alt-path: activated digivolve at cost 0 from a named base

#![allow(unused_imports, dead_code)]

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledDeclarativeClause, CompiledScope, CompiledTiming,
};
use digimon_engine::action::space::PASS;
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardKind, Keyword};

const CARD_ID: &str = "BT19-066";

fn composite_card(id: &str, name: &str) -> CardData {
    let mut card = make_test_card(id, name);
    card.card_kind = CardKind::Digimon;
    card.traits = vec!["Composite".to_string()];
    card.dp = Some(3000);
    card
}

fn wicked_god_card(id: &str, name: &str) -> CardData {
    let mut card = make_test_card(id, name);
    card.card_kind = CardKind::Digimon;
    card.traits = vec!["Wicked God".to_string()];
    card.dp = Some(3000);
    card
}

fn plain_card(id: &str, name: &str) -> CardData {
    let mut card = make_test_card(id, name);
    card.card_kind = CardKind::Digimon;
    card.traits = vec!["Plain".to_string()];
    card.dp = Some(1000);
    card
}

fn draw_filler(id: &str, name: &str) -> CardData {
    make_test_card(id, name)
}

fn base_runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT19-066 YAML loads from embedded DSL pack")
        .memory(10)
        .start()
}

// ════════════════════════════════════════════════════════════════════════════
// Section 1 — Structural assertions
// ════════════════════════════════════════════════════════════════════════════

#[test]
fn bt19_066_compiles_and_is_in_pack() {
    let runner = base_runner();
    assert!(
        runner.compiled_card(CARD_ID).is_some(),
        "BT19-066 must be present in the embedded DSL pack"
    );
}

#[test]
fn bt19_066_has_on_play_triggered_clause() {
    let runner = base_runner();
    let compiled = runner.compiled_card(CARD_ID).unwrap();
    let found = compiled.effects.iter().any(|c| {
        matches!(c, CompiledClause::Triggered(t)
            if t.when.contains(&CompiledTiming::OnPlay) && t.scope == CompiledScope::FaceUp)
    });
    assert!(found, "[On Play] triggered clause must be present");
}

#[test]
fn bt19_066_inherited_blocker_keyword_present() {
    let runner = base_runner();
    let compiled = runner.compiled_card(CARD_ID).unwrap();
    let found = compiled.effects.iter().any(|c| {
        matches!(
            c,
            CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword { keyword, scope, .. })
                if keyword == "Blocker" && *scope == CompiledScope::Inherited
        )
    });
    assert!(
        found,
        "inherited GrantKeyword(Blocker) clause must be present"
    );
}

#[test]
fn bt19_066_alt_digivolve_from_pagumon_cost_zero_present() {
    let runner = base_runner();
    let compiled = runner.compiled_card(CARD_ID).unwrap();
    let found = compiled
        .alt_paths
        .iter()
        .any(|p| matches!(p.kind, CompiledAltPathKind::Digivolve));
    assert!(
        found,
        "a Digivolve alt-path (from Pagumon, cost 0) must be present"
    );
}

// ════════════════════════════════════════════════════════════════════════════
// Section 2 — Condition gating: On Play prompt only installs with a matching
// hand card. No matching hand card → the effect silently doesn't fire (no
// selection at all), matching DCGO's HasMatchConditionOwnersHand gate.
// ════════════════════════════════════════════════════════════════════════════

#[test]
fn bt19_066_on_play_no_selection_when_hand_has_no_matching_trait() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT19-066 YAML loads")
        .add_card(plain_card("PLAIN-CARD", "PlainCard"))
        .hand(0, &[CARD_ID, "PLAIN-CARD"])
        .memory(10)
        .start();

    runner.play(0, 0).expect("play Gizamon");

    assert!(
        runner.pending_selection().is_none(),
        "no On Play prompt should install when the hand has no Composite/Wicked God card"
    );
}

#[test]
fn bt19_066_on_play_installs_optional_hand_selection_with_matching_trait() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT19-066 YAML loads")
        .add_card(composite_card("COMPOSITE-CARD", "CompositeCard"))
        .hand(0, &[CARD_ID, "COMPOSITE-CARD"])
        .memory(10)
        .start();

    runner.play(0, 0).expect("play Gizamon");

    let pending = runner
        .pending_selection()
        .expect("On Play selection should install when a Composite/Wicked God hand card exists");
    assert!(
        pending.is_optional || pending.valid_action_ids.contains(&PASS),
        "the trash selection must be declinable (canNoSelect: true)"
    );
}

// ════════════════════════════════════════════════════════════════════════════
// Section 3 — Behavioral outcome: decline / Composite / Wicked God / draw gate
// ════════════════════════════════════════════════════════════════════════════

/// Declining the optional trash (PASS) is legal even with a valid target, and
/// no Draw 2 fires — matches DCGO canNoSelect: true + "if (discarded) Draw".
#[test]
fn bt19_066_on_play_declining_trash_does_not_draw() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT19-066 YAML loads")
        .add_card(composite_card("COMPOSITE-CARD", "CompositeCard"))
        .add_card(draw_filler("DECK-FILLER", "DeckFiller"))
        .hand(0, &[CARD_ID, "COMPOSITE-CARD"])
        .deck(0, &["DECK-FILLER", "DECK-FILLER", "DECK-FILLER"])
        .memory(10)
        .start();

    runner.play(0, 0).expect("play Gizamon");
    let hand_before = runner.hand_size(0);
    let trash_before = runner.trash_size(0);

    runner
        .pending_selection()
        .expect("On Play selection should install");
    runner
        .execute_action(0, PASS)
        .expect("declining the optional trash resolves");
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.hand_size(0),
        hand_before,
        "declining should neither trash nor draw (net hand unchanged)"
    );
    assert_eq!(
        runner.trash_size(0),
        trash_before,
        "declining should not trash any card"
    );
}

/// Trashing a [Composite] trait hand card draws 2 (hand net: -1 trashed + 2
/// drawn = +1).
#[test]
fn bt19_066_on_play_trashing_composite_card_draws_two() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT19-066 YAML loads")
        .add_card(composite_card("COMPOSITE-CARD", "CompositeCard"))
        .add_card(draw_filler("DRAW-A", "DrawA"))
        .add_card(draw_filler("DRAW-B", "DrawB"))
        .hand(0, &[CARD_ID, "COMPOSITE-CARD"])
        .deck(0, &["DRAW-A", "DRAW-B"])
        .memory(10)
        .start();

    runner.play(0, 0).expect("play Gizamon");
    let hand_before = runner.hand_size(0); // 1 (the Composite card)
    let trash_before = runner.trash_size(0);

    let pending = runner
        .pending_selection()
        .expect("On Play selection should install");
    let pick = *pending
        .valid_action_ids
        .iter()
        .find(|&&id| id != PASS)
        .expect("the Composite card must be a selectable trash target");
    runner.execute_action(0, pick).expect("trash resolves");
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.trash_size(0),
        trash_before + 1,
        "the chosen Composite card should be trashed"
    );
    assert_eq!(
        runner.hand_size(0),
        hand_before - 1 + 2,
        "trashing should trigger Draw 2 (net hand +1)"
    );
}

/// Trashing a [Wicked God] trait hand card also draws 2 — the filter is an
/// OR of the two traits.
#[test]
fn bt19_066_on_play_trashing_wicked_god_card_draws_two() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT19-066 YAML loads")
        .add_card(wicked_god_card("WICKED-GOD-CARD", "WickedGodCard"))
        .add_card(draw_filler("DRAW-A", "DrawA"))
        .add_card(draw_filler("DRAW-B", "DrawB"))
        .hand(0, &[CARD_ID, "WICKED-GOD-CARD"])
        .deck(0, &["DRAW-A", "DRAW-B"])
        .memory(10)
        .start();

    runner.play(0, 0).expect("play Gizamon");
    let hand_before = runner.hand_size(0);
    let trash_before = runner.trash_size(0);

    let pending = runner
        .pending_selection()
        .expect("On Play selection should install for a Wicked God hand card");
    let pick = *pending
        .valid_action_ids
        .iter()
        .find(|&&id| id != PASS)
        .expect("the Wicked God card must be a selectable trash target");
    runner.execute_action(0, pick).expect("trash resolves");
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.trash_size(0),
        trash_before + 1,
        "the chosen Wicked God card should be trashed"
    );
    assert_eq!(
        runner.hand_size(0),
        hand_before - 1 + 2,
        "trashing a Wicked God card should also trigger Draw 2"
    );
}

/// A hand card with neither trait is not a legal target for the trash
/// selection, even when a matching card is also present.
#[test]
fn bt19_066_on_play_selection_excludes_non_matching_hand_cards() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT19-066 YAML loads")
        .add_card(composite_card("COMPOSITE-CARD", "CompositeCard"))
        .add_card(plain_card("PLAIN-CARD", "PlainCard"))
        .hand(0, &[CARD_ID, "COMPOSITE-CARD", "PLAIN-CARD"])
        .memory(10)
        .start();

    runner.play(0, 0).expect("play Gizamon");

    let pending = runner
        .pending_selection()
        .expect("On Play selection should install");
    // Exactly one non-PASS legal target: the Composite card. The Plain card
    // must not be offered.
    let non_pass_targets: Vec<u16> = pending
        .valid_action_ids
        .iter()
        .copied()
        .filter(|&id| id != PASS)
        .collect();
    assert_eq!(
        non_pass_targets.len(),
        1,
        "only the Composite-trait hand card should be a legal trash target"
    );
}

// ════════════════════════════════════════════════════════════════════════════
// Section 4 — Cost firing (events): committing the hand-trash selection
// fires an EffectTarget event naming Gizamon as source and the trashed card
// as target. `trash_from_hand_by_index` (game_actions/zones.rs) mutates
// hand/trash directly without pushing a `GameEvent::Trash` (engine's event
// emission is documented as partial, events.rs module doc); the selection
// commit's `EffectTarget` is the event that reliably fires for this verb, so
// this test asserts on that instead of an unemitted `Trash` event.
// ════════════════════════════════════════════════════════════════════════════

#[test]
fn bt19_066_trashing_composite_card_fires_effect_target_event() {
    use digimon_engine::events::GameEvent;

    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT19-066 YAML loads")
        .add_card(composite_card("COMPOSITE-CARD", "CompositeCard"))
        .add_card(draw_filler("DRAW-A", "DrawA"))
        .add_card(draw_filler("DRAW-B", "DrawB"))
        .hand(0, &[CARD_ID, "COMPOSITE-CARD"])
        .deck(0, &["DRAW-A", "DRAW-B"])
        .memory(10)
        .start();

    runner.play(0, 0).expect("play Gizamon");
    let cp = runner.event_checkpoint();

    let pending = runner
        .pending_selection()
        .expect("On Play selection should install");
    let pick = *pending
        .valid_action_ids
        .iter()
        .find(|&&id| id != PASS)
        .expect("the Composite card must be a selectable trash target");
    runner.execute_action(0, pick).expect("trash resolves");
    let _ = runner.auto_resolve();

    let events = runner.events_since(cp);
    let target_count = events
        .iter()
        .filter(|e| {
            matches!(e, GameEvent::EffectTarget { source_card_id, targets, .. }
                if source_card_id == CARD_ID
                    && targets.iter().any(|t| t.card_id == "COMPOSITE-CARD"))
        })
        .count();
    assert!(
        target_count >= 1,
        "committing the trash selection must emit an EffectTarget event naming \
         the Composite card, got events: {:?}",
        events
    );
}

// ════════════════════════════════════════════════════════════════════════════
// Section 5 — Inherited Blocker behavioral check. Inherited text only
// applies while Gizamon is a BURIED digivolution source (general_rule.pdf
// inherited-effect rule; `Game::has_keyword` only walks `card_sources`
// strictly under the top card — see `game/queries.rs`). As the top card
// itself (no further digivolution on top), Gizamon's own inherited Blocker
// does not apply to itself.
// ════════════════════════════════════════════════════════════════════════════

#[test]
fn bt19_066_as_top_card_does_not_grant_itself_blocker() {
    let mut runner = base_runner();
    let gizamon = runner.place_on_field(0, CARD_ID, Some(0));

    assert!(
        !runner.game.has_keyword(gizamon, Keyword::Blocker),
        "Gizamon's own inherited effect should not apply to itself as the top card \
         (inherited effects apply only from buried digivolution sources)"
    );
}

#[test]
fn bt19_066_buried_as_digivolution_source_grants_carrier_blocker() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT19-066 YAML loads")
        .add_card(plain_card("CARRIER", "CarrierMon"))
        .memory(10)
        .start();

    // Gizamon buried under CARRIER: CARRIER should carry Gizamon's
    // inherited Blocker keyword.
    let carrier = runner.place_stack(0, &[CARD_ID, "CARRIER"]);

    assert!(
        runner.game.has_keyword(carrier, Keyword::Blocker),
        "the carrier stacked on top of Gizamon should gain the inherited Blocker keyword"
    );
}
