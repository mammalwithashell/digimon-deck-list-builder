//! BT18-013 Deltamon — Digimon, Lv.4, Red/Purple, DP 5000, Cost 5.
//! Traits: Composite. Attribute: Virus.
//!
//! # Card text (code/digimon-engine/cards/bt18/BT18-013.json — verbatim)
//!
//! ＜Raid＞ (When this Digimon attacks, you may switch the target of attack
//! to 1 of your opponent's unsuspended Digimon with the highest DP.)
//! [On Play] [When Digivolving] By trashing 1 card in your hand, return 1
//! card with the [Composite] or [Wicked God] trait from your trash to the
//! hand.
//!
//! Inherited Effect:
//! ＜Retaliation＞ (When this Digimon is deleted after losing a battle,
//! delete the Digimon it was battling.)
//!
//! Digivolution: standard Lv.3 Red / cost 3 or Lv.3 Purple / cost 3
//! (printed evo_costs). Alt-digivolve: [Digivolve] [Gazimon]/[Gizamon]:
//! Cost 2 (xros_req).
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT18/Red/BT18_013.cs
//!   - EffectTiming.None: AddSelfDigivolutionRequirementStaticEffect over a
//!     permanent whose TopCard name equals "Gazimon" OR "Gizamon", cost 2.
//!   - EffectTiming.OnAllyAttack: RaidSelfEffect(isInheritedEffect: false)
//!     — own-scope (face-up) Raid, NOT inherited.
//!   - EffectTiming.OnEnterFieldAnyone (x2, one gated CanTriggerOnPlay, one
//!     CanTriggerWhenDigivolving, identical body):
//!       SelectHandEffect Mode.Discard, maxCount: 1, canNoSelect: true
//!       ("By trashing 1 card in your hand" — optional cost). If a card was
//!       discarded: SelectCardEffect Root.Trash Mode.AddHand,
//!       canTargetCondition = EqualsTraits("Composite") ||
//!       EqualsTraits("Wicked God"), maxCount: 1, canNoSelect: () => false
//!       (MANDATORY once an eligible trash card exists — gated by
//!       HasMatchConditionOwnersCardInTrash so it silently no-ops with none).
//!   - EffectTiming.OnDestroyedAnyone: RetaliationSelfEffect(isInheritedEffect: true).
//!
//! # Patterns this test covers (RUST_DSL_TEST_API §4.3)
//! - H9 Raid (own-scope printed keyword — declarative grant_keyword, no scope)
//! - A4 Trash → hand recursion, cost-gated (select_hand cost:true → abort on
//!   decline; select_trash MANDATORY-if-match, not optional, matching DCGO's
//!   canNoSelect: () => false)
//! - G-ALT-DIGIVOLVE-BY-NAME alt-digivolve gated on base-permanent top-card
//!   name (Gazimon OR Gizamon), cost 2
//! - H-inherited Retaliation (inherited-scope printed keyword — declarative
//!   grant_keyword, scope: inherited)
//!
//! # No printed [Once Per Turn] on this card. Section 5 (OPT lockout) is
//! intentionally skipped — DCGO's maxUseCount is -1 (unlimited) on both the
//! On Play and When Digivolving activate classes.

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledCost, CompiledDeclarativeClause, CompiledScope,
    CompiledTiming,
};
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, make_test_card_with_level, DebugRunner};
use digimon_engine::enums::{CardColor, EffectTiming, Keyword, PlayerId};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::TriggerSource;

const CARD_ID: &str = "BT18-013";

// ─── Fixture helpers ─────────────────────────────────────────────────────────

/// A generic level-3 permanent named "Gazimon" — the alt-digivolve base.
fn gazimon_lv3(card_id: &str) -> CardData {
    let mut c = make_test_card_with_level(card_id, "Gazimon", 3);
    c.colors = vec![CardColor::Black];
    c.dp = Some(1000);
    c
}

/// A generic level-3 permanent named "Gizamon" — the alt-digivolve base
/// (a distinct printed name from "Gazimon").
fn gizamon_lv3(card_id: &str) -> CardData {
    let mut c = make_test_card_with_level(card_id, "Gizamon", 3);
    c.colors = vec![CardColor::Black];
    c.dp = Some(1000);
    c
}

/// A level-3 permanent that is neither Gazimon nor Gizamon — must NOT be a
/// legal alt-digivolve base.
fn unrelated_lv3(card_id: &str) -> CardData {
    let mut c = make_test_card_with_level(card_id, "Unrelated Rookie", 3);
    c.colors = vec![CardColor::Black];
    c.dp = Some(1000);
    c
}

/// A [Composite]-trait Digimon card usable as trash-recursion bait.
fn composite_card(card_id: &str) -> CardData {
    let mut c = make_test_card(card_id, "Composite Fodder");
    c.traits = vec!["Composite".to_string()];
    c
}

/// A [Wicked God]-trait Digimon card usable as trash-recursion bait.
fn wicked_god_card(card_id: &str) -> CardData {
    let mut c = make_test_card(card_id, "Wicked God Fodder");
    c.traits = vec!["Wicked God".to_string()];
    c
}

/// A Digimon card with neither trait — must NOT be a legal recovery target.
fn plain_card(card_id: &str) -> CardData {
    make_test_card(card_id, "Plain Fodder")
}

fn push_to_trash(runner: &mut DebugRunner, p: PlayerId, card_id: &str) {
    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .unwrap_or_else(|| panic!("push_to_trash: unknown card_id {}", card_id));
    let next_idx = runner.game.next_card_index();
    let card = CardSource::new(data_idx, p, next_idx);
    runner.game.players[p as usize].trash.push(card);
}

fn push_to_hand(runner: &mut DebugRunner, p: PlayerId, card_id: &str) -> usize {
    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .unwrap_or_else(|| panic!("push_to_hand: unknown card_id {}", card_id));
    let next_idx = runner.game.next_card_index();
    let card = CardSource::new(data_idx, p, next_idx);
    runner.game.players[p as usize].hand.push(card);
    runner.game.players[p as usize].hand.len() - 1
}

fn base() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT18-013 YAML parses and compiles")
        .add_card(gazimon_lv3("GAZIMON-LV3"))
        .add_card(gizamon_lv3("GIZAMON-LV3"))
        .add_card(unrelated_lv3("UNRELATED-LV3"))
        .add_card(composite_card("COMPOSITE-A"))
        .add_card(composite_card("COMPOSITE-B"))
        .add_card(wicked_god_card("WICKED-GOD-A"))
        .add_card(plain_card("PLAIN-A"))
        .add_card(make_test_card("FILLER", "Filler"))
        .add_card(make_test_card_with_level("OPP-ROOKIE", "Opp Rookie", 3))
        .deck(0, &["FILLER"; 8])
        .deck(1, &["FILLER"; 8])
        .memory(12)
        .start()
}

fn place_deltamon(runner: &mut DebugRunner) -> PermanentHandle {
    runner.place_on_field(0, CARD_ID, Some(0))
}

// ─── SECTION 1 — Structural assertions ───────────────────────────────────────

#[test]
fn bt18_013_compiles_in_embedded_pack() {
    let runner = base();
    assert!(
        runner.compiled_card(CARD_ID).is_some(),
        "BT18-013 must be present in embedded DSL pack"
    );
}

#[test]
fn bt18_013_card_metadata_matches_print() {
    let runner = base();
    let card = runner.compiled_card(CARD_ID).expect("compiled present");

    assert_eq!(card.name, "Deltamon");
    assert_eq!(card.level, Some(4));
    assert_eq!(card.dp, Some(5000));
    assert!(card.traits.contains(&"Composite".to_string()));
}

#[test]
fn bt18_013_has_own_scope_raid_grant() {
    let runner = base();
    let card = runner.compiled_card(CARD_ID).expect("compiled present");

    let has_own_raid = card.effects.iter().any(|c| {
        matches!(
            c,
            CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword {
                keyword,
                scope: CompiledScope::FaceUp,
                ..
            }) if keyword == "Raid"
        )
    });
    assert!(
        has_own_raid,
        "BT18-013 must grant own-scope (face-up) Raid — DCGO uses \
         RaidSelfEffect(isInheritedEffect: false)"
    );
}

#[test]
fn bt18_013_has_inherited_retaliation_grant() {
    let runner = base();
    let card = runner.compiled_card(CARD_ID).expect("compiled present");

    let has_inherited_retaliation = card.effects.iter().any(|c| {
        matches!(
            c,
            CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword {
                keyword,
                scope: CompiledScope::Inherited,
                ..
            }) if keyword == "Retaliation"
        )
    });
    assert!(
        has_inherited_retaliation,
        "BT18-013 must grant inherited Retaliation — DCGO uses \
         RetaliationSelfEffect(isInheritedEffect: true)"
    );
}

#[test]
fn bt18_013_has_on_play_and_when_digivolving_clause() {
    let runner = base();
    let card = runner.compiled_card(CARD_ID).expect("compiled present");

    let triggered: Vec<_> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();

    let has_op_wd = triggered.iter().any(|t| {
        t.when.contains(&CompiledTiming::OnPlay)
            && t.when.contains(&CompiledTiming::WhenDigivolving)
    });
    assert!(
        has_op_wd,
        "must have a single [On Play][When Digivolving] clause for the trash→recover body"
    );
}

#[test]
fn bt18_013_on_play_clause_is_not_once_per_turn() {
    let runner = base();
    let card = runner.compiled_card(CARD_ID).expect("compiled present");

    let clause = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::OnPlay) => Some(t),
            _ => None,
        })
        .expect("On Play clause exists");
    assert!(
        !clause.once_per_turn,
        "DCGO uses maxUseCount -1 (unlimited) — no [Once Per Turn] on this card"
    );
}

#[test]
fn bt18_013_has_gazimon_gizamon_alt_digivolve_path() {
    let runner = base();
    let card = runner.compiled_card(CARD_ID).expect("compiled present");

    let has_gazimon = card.alt_paths.iter().any(|p| {
        p.kind == CompiledAltPathKind::Digivolve
            && p.cost == Some(CompiledCost::Literal(2))
            && p.from
                .as_ref()
                .is_some_and(|f| f.name_is.as_deref() == Some("Gazimon"))
    });
    let has_gizamon = card.alt_paths.iter().any(|p| {
        p.kind == CompiledAltPathKind::Digivolve
            && p.cost == Some(CompiledCost::Literal(2))
            && p.from
                .as_ref()
                .is_some_and(|f| f.name_is.as_deref() == Some("Gizamon"))
    });
    assert!(
        has_gazimon,
        "must have a Gazimon → cost 2 alt-digivolve path"
    );
    assert!(
        has_gizamon,
        "must have a Gizamon → cost 2 alt-digivolve path"
    );
}

// ─── SECTION 2 — Runtime keyword presence (grant_keyword materialization) ────

#[test]
fn bt18_013_carries_raid_keyword_at_runtime() {
    let mut runner = base();
    let handle = place_deltamon(&mut runner);
    assert!(
        runner.game.has_keyword(handle, Keyword::Raid),
        "BT18-013 must carry Raid as a runtime (own-scope) keyword"
    );
}

#[test]
fn bt18_013_grants_inherited_retaliation_to_the_stack_top_when_it_is_a_source() {
    // Inherited text only matters when BT18-013 is a digivolution SOURCE
    // underneath a later top card — `Game::has_keyword` deliberately does not
    // surface inherited grants when the card is itself the top of the stack
    // (matches `card_data_from_compiled`, which skips `Inherited`-scope
    // grants when baking a DSL card's own-face `keywords`).
    let mut runner = base();
    let top = runner.place_stack(0, &[CARD_ID, "GAZIMON-LV3"]);
    assert!(
        runner.game.has_keyword(top, Keyword::Retaliation),
        "the stack top must inherit Retaliation from BT18-013 sitting underneath it"
    );
}

#[test]
fn bt18_013_as_top_card_does_not_report_retaliation_via_its_own_inherited_grant() {
    // Sanity check for the asymmetry above: as the TOP card, BT18-013 must
    // not report Retaliation through the inherited-scope grant (own-scope
    // Raid is unaffected and is covered by the sibling test above).
    let mut runner = base();
    let handle = place_deltamon(&mut runner);
    assert!(
        !runner.game.has_keyword(handle, Keyword::Retaliation),
        "as the top card, BT18-013's inherited Retaliation grant must not \
         self-apply (inherited text only affects a later digivolution built \
         on top of it)"
    );
}

// ─── SECTION 3 — [On Play] trash-cost gating ─────────────────────────────────

#[test]
fn bt18_013_on_play_offers_optional_hand_trash_cost() {
    let mut runner = base();
    let _fodder = push_to_hand(&mut runner, 0, "FILLER");
    let hand_index = push_to_hand(&mut runner, 0, CARD_ID);

    runner.play(0, hand_index);

    // The first prompt after playing is the hand-trash cost selection.
    assert!(
        runner.pending_selection().is_some(),
        "playing BT18-013 with a hand card available must prompt the optional trash cost"
    );
    assert!(
        runner.pending_is_optional(),
        "the hand-trash cost is optional (\"By trashing\" — DCGO canNoSelect: true)"
    );
}

#[test]
fn bt18_013_on_play_no_prompt_with_empty_hand() {
    let mut runner = base();
    // Deltamon itself is the only card that will leave the hand once played;
    // ensure the hand is otherwise empty at play time.
    let hand_index = push_to_hand(&mut runner, 0, CARD_ID);

    runner.play(0, hand_index);

    assert!(
        runner.pending_selection().is_none(),
        "with no cards left in hand, the trash-cost prompt must not install \
         (DCGO CanActivateCondition requires HandCards.Count >= 1)"
    );
}

#[test]
fn bt18_013_declining_the_trash_cost_does_nothing() {
    use digimon_engine::action::space::PASS;

    let mut runner = base();
    let _fodder = push_to_hand(&mut runner, 0, "FILLER");
    push_to_trash(&mut runner, 0, "COMPOSITE-A");
    let hand_index = push_to_hand(&mut runner, 0, CARD_ID);

    runner.play(0, hand_index);
    assert!(runner.pending_is_optional());

    let hand_before = runner.hand_size(0);
    let trash_before = runner.trash_size(0);

    runner
        .execute_action(0, PASS)
        .expect("decline the trash cost");

    assert!(
        runner.pending_selection().is_none(),
        "declining the cost must not chain into the recovery prompt"
    );
    assert_eq!(
        runner.hand_size(0),
        hand_before,
        "declining leaves the hand untouched"
    );
    assert_eq!(
        runner.trash_size(0),
        trash_before,
        "declining must not trash the Composite fodder either"
    );
}

// ─── SECTION 4 — Paying the cost → mandatory recovery pick ───────────────────

#[test]
fn bt18_013_paying_cost_trashes_the_chosen_hand_card() {
    let mut runner = base();
    let _fodder = push_to_hand(&mut runner, 0, "FILLER");
    let hand_index = push_to_hand(&mut runner, 0, CARD_ID);

    runner.play(0, hand_index);
    assert!(runner.pending_selection().is_some());

    let view = runner
        .pending_selection_view()
        .expect("trash-cost prompt pending");
    // Only one card ("FILLER") remains in hand at this point.
    assert_eq!(view.valid_action_ids.len(), 1);

    let trash_before = runner.trash_size(0);
    runner
        .execute_action(0, view.valid_action_ids[0])
        .expect("pay the trash cost");
    runner
        .auto_resolve()
        .expect("resolve the recovery flow (no eligible trash targets)");

    assert_eq!(
        runner.trash_size(0),
        trash_before + 1,
        "paying the cost must move the chosen hand card to trash"
    );
}

#[test]
fn bt18_013_paying_cost_with_no_eligible_trash_target_silently_skips_recovery() {
    let mut runner = base();
    let _fodder = push_to_hand(&mut runner, 0, "FILLER");
    push_to_trash(&mut runner, 0, "PLAIN-A"); // ineligible: no Composite/Wicked God trait
    let hand_index = push_to_hand(&mut runner, 0, CARD_ID);

    runner.play(0, hand_index);
    let view = runner.pending_selection_view().unwrap();
    runner
        .execute_action(0, view.valid_action_ids[0])
        .expect("pay the trash cost");

    assert!(
        runner.pending_selection().is_none(),
        "with no [Composite]/[Wicked God] card in trash, the recovery prompt \
         must not install (DCGO HasMatchConditionOwnersCardInTrash gate)"
    );
}

#[test]
fn bt18_013_recovery_prompt_is_mandatory_when_a_target_exists() {
    let mut runner = base();
    let _fodder = push_to_hand(&mut runner, 0, "FILLER");
    push_to_trash(&mut runner, 0, "COMPOSITE-A");
    let hand_index = push_to_hand(&mut runner, 0, CARD_ID);

    runner.play(0, hand_index);
    let cost_view = runner.pending_selection_view().unwrap();
    runner
        .execute_action(0, cost_view.valid_action_ids[0])
        .expect("pay the trash cost");

    let recovery_view = runner
        .pending_selection_view()
        .expect("recovery prompt must install once an eligible trash card exists");
    assert!(
        !recovery_view.is_optional,
        "DCGO's SelectCardEffect uses canNoSelect: () => false — the recovery \
         pick is mandatory once at least one [Composite]/[Wicked God] card is \
         in the trash"
    );
}

#[test]
fn bt18_013_recovery_returns_composite_trait_card_to_hand() {
    let mut runner = base();
    let _fodder = push_to_hand(&mut runner, 0, "FILLER");
    push_to_trash(&mut runner, 0, "COMPOSITE-A");
    let hand_index = push_to_hand(&mut runner, 0, CARD_ID);

    runner.play(0, hand_index);
    let cost_view = runner.pending_selection_view().unwrap();
    runner
        .execute_action(0, cost_view.valid_action_ids[0])
        .expect("pay the trash cost");

    let recovery_view = runner
        .pending_selection_view()
        .expect("recovery prompt installs");
    assert_eq!(
        recovery_view.valid_action_ids.len(),
        1,
        "exactly one eligible [Composite] card in trash"
    );

    let hand_before = runner.hand_size(0);
    let trash_before = runner.trash_size(0);
    runner
        .execute_action(0, recovery_view.valid_action_ids[0])
        .expect("pick the Composite card");

    assert_eq!(runner.hand_size(0), hand_before + 1);
    assert_eq!(runner.trash_size(0), trash_before - 1);
    assert!(
        runner.game.players[0]
            .hand
            .iter()
            .any(|c| c.card_id(&runner.game.card_data) == "COMPOSITE-A"),
        "the returned card must be the [Composite] fodder"
    );
}

#[test]
fn bt18_013_recovery_returns_wicked_god_trait_card_to_hand() {
    let mut runner = base();
    let _fodder = push_to_hand(&mut runner, 0, "FILLER");
    push_to_trash(&mut runner, 0, "WICKED-GOD-A");
    let hand_index = push_to_hand(&mut runner, 0, CARD_ID);

    runner.play(0, hand_index);
    let cost_view = runner.pending_selection_view().unwrap();
    runner
        .execute_action(0, cost_view.valid_action_ids[0])
        .expect("pay the trash cost");

    let recovery_view = runner
        .pending_selection_view()
        .expect("recovery prompt installs for [Wicked God] fodder too");
    assert_eq!(recovery_view.valid_action_ids.len(), 1);

    runner
        .execute_action(0, recovery_view.valid_action_ids[0])
        .expect("pick the Wicked God card");

    assert!(
        runner.game.players[0]
            .hand
            .iter()
            .any(|c| c.card_id(&runner.game.card_data) == "WICKED-GOD-A"),
        "the returned card must be the [Wicked God] fodder"
    );
}

#[test]
fn bt18_013_recovery_excludes_plain_trash_cards() {
    let mut runner = base();
    let _fodder = push_to_hand(&mut runner, 0, "FILLER");
    push_to_trash(&mut runner, 0, "COMPOSITE-A");
    push_to_trash(&mut runner, 0, "PLAIN-A");
    let hand_index = push_to_hand(&mut runner, 0, CARD_ID);

    runner.play(0, hand_index);
    let cost_view = runner.pending_selection_view().unwrap();
    runner
        .execute_action(0, cost_view.valid_action_ids[0])
        .expect("pay the trash cost");

    let recovery_view = runner
        .pending_selection_view()
        .expect("recovery prompt installs");
    assert_eq!(
        recovery_view.valid_action_ids.len(),
        1,
        "only the [Composite] card is a legal pick; the plain card must be excluded"
    );
}

// ─── SECTION 5 — [When Digivolving] shares the same trash→recover body ──────

#[test]
fn bt18_013_when_digivolving_also_offers_the_trash_cost() {
    let mut runner = base();
    let deltamon = place_deltamon(&mut runner);
    let _fodder = push_to_hand(&mut runner, 0, "FILLER");

    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(deltamon),
    );
    runner.game.drain_effect_queue();

    assert!(
        runner.pending_selection().is_some(),
        "[When Digivolving] must also offer the optional hand-trash cost"
    );
    assert!(runner.pending_is_optional());
}

#[test]
fn bt18_013_when_digivolving_no_prompt_with_empty_hand() {
    let mut runner = base();
    let deltamon = place_deltamon(&mut runner);
    // Hand is empty.

    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(deltamon),
    );
    runner.game.drain_effect_queue();

    assert!(
        runner.pending_selection().is_none(),
        "[When Digivolving] must not prompt the trash cost with an empty hand"
    );
}

// ─── SECTION 6 — Alt-digivolve base name gating ──────────────────────────────

#[test]
fn bt18_013_alt_digivolve_rejects_unrelated_level_3_base() {
    let runner = base();
    let card = runner.compiled_card(CARD_ID).expect("compiled present");

    let matches_unrelated = card.alt_paths.iter().any(|p| {
        p.kind == CompiledAltPathKind::Digivolve
            && p.from
                .as_ref()
                .is_some_and(|f| f.name_is.as_deref() == Some("Unrelated Rookie"))
    });
    assert!(
        !matches_unrelated,
        "the alt-digivolve base filter must be name-scoped to Gazimon/Gizamon only"
    );
}
