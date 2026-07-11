//! P-215 Icemon — Digimon, Lv.4, Blue/Black, Cost 5, DP 6000, Form: Champion.
//! Traits: [Ice-Snow] + (Rule) Trait: Has [Mineral] Type (official Bandai DB;
//! encoded as `traits: [Ice-Snow, Mineral]` in the YAML).
//!
//! # Card text (official Bandai DB — data/card_bundles/P-215.md)
//!
//! [When Moving] [On Play] [When Digivolving] By placing 1 level 4 or lower
//! [Ice-Snow], [Mineral] or [Rock] trait card from your hand or trash as this
//! Digimon's bottom digivolution card, until your opponent's turn ends, their
//! effects can't return 1 of your [Ice-Snow], [Mineral] or [Rock] trait Digimon
//! to hands or decks or affect it with <De-Digivolve> effects.
//! (Rule) Trait: Has [Mineral] Type.
//!
//! Inherited Effect: <Blocker>
//!
//! Digivolve: Blue Lv.3 cost 3 / Black Lv.3 cost 3 (standard circles) +
//! [Digivolve] Lv.3 w/[Ice-Snow]/[Mineral]/[Rock] trait: Cost 2 (alt).
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/P/Blue/P_215.cs (base repo, rule 29):
//! shared WM/OP/WD coroutine — hand-or-trash pick (`canNoSelect: true`),
//! `AddDigivolutionCardsBottom` on this permanent, then a MANDATORY
//! (`canNoSelect: false`) protect-target pick; the hand/deck-return grants
//! are gated `IsOpponentEffect`; OnMove is self-gated
//! (`permanent == card.PermanentOfThisCard()`); cost filter requires
//! `HasLevel && Level <= 4` (level-less cards ineligible).
//!
//! # Known modelling caveat (DSL vocab gap — see YAML status table)
//! The three protection modifiers install cause-agnostic (`cause_filter:
//! None`); printed scope is opponent-only ("THEIR effects"). No DSL `cause:`
//! knob exists yet.
//!
//! # Patterns this test covers
//! - D6: CannotBeReturnedToHand / CannotBeReturnedToDeck / CannotBeDeDigivolved
//!   modifiers with end_of_opponents_turn expiry (EX8-070 / BT18-064 pattern)
//! - A5-adjacent: source-placement cost from hand or trash (select_union_zone +
//!   place_as_bottom_source, same idiom as EX11-065 Close)
//! - Multi-timing: on_play + when_digivolving + on_move (EX11-008 pattern)
//! - H5: Inherited <Blocker>

use digimon_dsl::compiled::{
    CompiledClause, CompiledDeclarativeClause, CompiledScope, CompiledTiming,
    CompiledTriggeredClause,
};
use digimon_engine::action::space::PASS;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardKind, Keyword, ModifierType, PlayerId};
use digimon_engine::selection::{SelectionError, SelectionKind};
use digimon_engine::CardData;

// ─── Helpers ────────────────────────────────────────────────────────────────

/// Push a card directly into a player's trash (mirrors union_zone.rs pattern).
fn push_to_trash(r: &mut DebugRunner, player: PlayerId, card_id: &str) {
    let data_idx = r
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .unwrap_or_else(|| panic!("push_to_trash: unknown card_id {card_id}"));
    let card_index = r.game.next_card_index();
    let card = CardSource::new(data_idx, player, card_index);
    r.game.players[player as usize].trash.push(card);
}

/// Resolve exactly ONE pending selection (picks first valid action, or PASS if
/// optional with no candidates). Returns an error if no selection is pending.
fn resolve_one(runner: &mut DebugRunner) -> Result<(), SelectionError> {
    let (player, action_id) = {
        let sel = runner
            .pending_selection()
            .ok_or(SelectionError::NoPendingSelection)?;
        let action_id = match sel.valid_action_ids.first().copied() {
            Some(id) => id,
            None if sel.is_optional => PASS,
            None => return Err(SelectionError::InvalidAction),
        };
        (sel.selecting_player, action_id)
    };
    runner.execute_action(player, action_id)
}

/// A level-3 Ice-Snow Digimon (eligible as source-placement cost card).
fn make_ice_snow_lv3(id: &str) -> CardData {
    let mut card = make_test_card(id, &format!("{id} IceSnow3"));
    card.traits.push("Ice-Snow".to_string());
    card.level = Some(3);
    card.dp = Some(3000);
    card
}

/// A level-4 Mineral Digimon (eligible as source-placement cost card).
fn make_mineral_lv4(id: &str) -> CardData {
    let mut card = make_test_card(id, &format!("{id} Mineral4"));
    card.traits.push("Mineral".to_string());
    card.level = Some(4);
    card.dp = Some(5000);
    card
}

/// A level-4 Rock Digimon (eligible as source-placement cost card AND as a
/// protect target on the field).
fn make_rock_lv4(id: &str) -> CardData {
    let mut card = make_test_card(id, &format!("{id} Rock4"));
    card.traits.push("Rock".to_string());
    card.level = Some(4);
    card.dp = Some(5000);
    card
}

/// A level-5 Ice-Snow Digimon — too high level to be a cost card, but valid
/// protect target on the field.
fn make_ice_snow_lv5(id: &str) -> CardData {
    let mut card = make_test_card(id, &format!("{id} IceSnow5"));
    card.traits.push("Ice-Snow".to_string());
    card.level = Some(5);
    card.dp = Some(7000);
    card
}

/// A plain Digimon with no Ice-Snow / Mineral / Rock traits.
fn make_plain_digi(id: &str) -> CardData {
    let mut card = make_test_card(id, &format!("{id} Plain"));
    card.level = Some(4);
    card.dp = Some(4000);
    card
}

/// A level-3 card with no matching traits (not eligible as cost card).
fn make_no_trait_lv3(id: &str) -> CardData {
    let mut card = make_test_card(id, &format!("{id} NoTrait"));
    card.level = Some(3);
    card.dp = Some(2000);
    card
}

fn runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("P-215")
        .expect("P-215 YAML parses and compiles")
        .build()
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 1 — Structural assertions
// ═══════════════════════════════════════════════════════════════════════════════

/// P-215 must compile and appear in the embedded DSL pack.
#[test]
fn p_215_yaml_compiles_without_error() {
    let runner = runner();
    assert!(
        runner.compiled_card("P-215").is_some(),
        "P-215 must be present in the embedded DSL pack (YAML must parse + compile)"
    );
}

/// P-215 must have exactly two compiled effects:
/// - one triggered clause (on_play / when_digivolving / on_move)
/// - one GrantKeyword declarative clause (inherited Blocker)
#[test]
fn p_215_has_triggered_and_inherited_clause() {
    let runner = runner();
    let card = runner.compiled_card("P-215").expect("P-215 in pack");

    let triggered_count = card
        .effects
        .iter()
        .filter(|c| matches!(c, CompiledClause::Triggered(_)))
        .count();

    let keyword_count = card
        .effects
        .iter()
        .filter(|c| {
            matches!(
                c,
                CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword { .. })
            )
        })
        .count();

    assert_eq!(
        triggered_count, 1,
        "P-215 must have exactly 1 triggered clause (on_play/when_digivolving/on_move)"
    );
    assert_eq!(
        keyword_count, 1,
        "P-215 must have exactly 1 GrantKeyword clause (inherited Blocker)"
    );
}

/// The triggered clause fires on OnPlay, WhenDigivolving, and OnMove timings.
#[test]
fn p_215_triggered_clause_has_three_timings() {
    let runner = runner();
    let card = runner.compiled_card("P-215").expect("P-215 in pack");

    let triggered: Vec<&CompiledTriggeredClause> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();

    assert_eq!(triggered.len(), 1, "must have 1 triggered clause");
    let t = triggered[0];

    assert!(
        t.when.contains(&CompiledTiming::OnPlay),
        "triggered clause must fire on OnPlay"
    );
    assert!(
        t.when.contains(&CompiledTiming::WhenDigivolving),
        "triggered clause must fire on WhenDigivolving"
    );
    assert!(
        t.when.contains(&CompiledTiming::OnMove),
        "triggered clause must fire on OnMove (when_moving)"
    );
    assert_eq!(
        t.when.len(),
        3,
        "triggered clause must have exactly 3 timings"
    );
}

/// Structural guard for the bug-sheet fix: the OnMove timing must carry the
/// `event_permanent_is_source: true` gate (printed [When Moving] is
/// SELF-scoped; the engine's on_move dispatch is a board-wide observer scan).
/// Behavioral coverage is in Section 9; this pins the YAML shape so a
/// refactor can't silently drop the gate.
#[test]
fn p_215_on_move_timing_carries_self_gate() {
    let runner = runner();
    let card = runner.compiled_card("P-215").expect("P-215 in pack");

    let t = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .expect("triggered clause must exist");

    let on_move_gate = t
        .timing_conditions
        .iter()
        .find(|tc| tc.when == CompiledTiming::OnMove)
        .expect("[When Moving] must have a timing_conditions entry");
    assert_eq!(
        on_move_gate.condition.event_permanent_is_source,
        Some(true),
        "the OnMove timing condition must gate on event_permanent_is_source \
         (self-scoped [When Moving], DCGO permanent == card.PermanentOfThisCard())"
    );
}

/// The triggered clause is optional (the cost is player-driven).
#[test]
fn p_215_triggered_clause_is_optional() {
    let runner = runner();
    let card = runner.compiled_card("P-215").expect("P-215 in pack");

    let t = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .expect("triggered clause must exist");

    assert!(
        t.optional,
        "P-215's main clause must be optional (player chooses to pay cost)"
    );
}

/// The triggered clause is own-scope (FaceUp, not inherited).
#[test]
fn p_215_triggered_clause_is_face_up_scope() {
    let runner = runner();
    let card = runner.compiled_card("P-215").expect("P-215 in pack");

    let t = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .expect("triggered clause must exist");

    assert_eq!(
        t.scope,
        CompiledScope::FaceUp,
        "main clause must be FaceUp (own) scope"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 2 — Inherited <Blocker>
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn p_215_inherited_blocker_is_available_from_source() {
    let mut host = make_test_card("HOST", "Host");
    host.dp = Some(5000);

    let mut runner = DebugRunner::builder()
        .dsl_card("P-215")
        .expect("P-215 YAML parses and compiles")
        .add_card(host)
        .build();
    let carrier = runner.place_stack(0, &["P-215", "HOST"]);

    assert!(runner.game.has_keyword(carrier, Keyword::Blocker));
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 3 — On Play: cost prompt and source placement
// ═══════════════════════════════════════════════════════════════════════════════

/// [On Play] with an eligible card in hand: prompts the player to pick
/// from the union zone (UnionZone selection kind).
#[test]
fn p_215_on_play_prompts_union_zone_when_eligible_card_in_hand() {
    let cost_card = make_ice_snow_lv3("COST-HAND");
    let protect_digi = make_ice_snow_lv5("PROTECT");

    let mut runner = DebugRunner::builder()
        .dsl_card("P-215")
        .expect("P-215 YAML parses and compiles")
        .add_card(cost_card)
        .add_card(protect_digi)
        .hand(0, &["P-215", "COST-HAND"])
        .memory(10)
        .start();

    runner.place_on_field(0, "PROTECT", None);

    // Play P-215 from hand (index 0).
    runner.play(0, 0);

    let kind = runner
        .pending_kind()
        .expect("a selection must install after playing P-215");
    assert!(
        matches!(kind, SelectionKind::UnionZone { .. }),
        "P-215 on_play must install a UnionZone selection (hand ∪ trash cost), got {kind:?}"
    );
}

/// [On Play] with an eligible card in trash: prompts the player to pick
/// from the union zone.
#[test]
fn p_215_on_play_prompts_union_zone_when_eligible_card_in_trash() {
    let cost_card = make_mineral_lv4("COST-TRASH");
    let protect_digi = make_ice_snow_lv5("PROT-T");

    let mut runner = DebugRunner::builder()
        .dsl_card("P-215")
        .expect("P-215 YAML parses and compiles")
        .add_card(cost_card)
        .add_card(protect_digi)
        .hand(0, &["P-215"])
        .memory(10)
        .start();

    push_to_trash(&mut runner, 0, "COST-TRASH");
    runner.place_on_field(0, "PROT-T", None);

    runner.play(0, 0);

    let kind = runner
        .pending_kind()
        .expect("a selection must install after playing P-215");
    assert!(
        matches!(kind, SelectionKind::UnionZone { .. }),
        "P-215 on_play must install a UnionZone selection when cost card is in trash, got {kind:?}"
    );
}

/// [On Play] optional: when no eligible cost card is available in hand or
/// trash, the effect fires but auto-resolves (no selection, no effect).
#[test]
fn p_215_on_play_no_prompt_when_no_eligible_cost_card() {
    let protect_digi = make_ice_snow_lv5("PROT-NONE");
    let non_eligible = make_no_trait_lv3("NO-TRAIT"); // no Ice-Snow/Mineral/Rock trait

    let mut runner = DebugRunner::builder()
        .dsl_card("P-215")
        .expect("P-215 YAML parses and compiles")
        .add_card(protect_digi)
        .add_card(non_eligible)
        .hand(0, &["P-215"])
        .memory(10)
        .start();

    push_to_trash(&mut runner, 0, "NO-TRAIT");
    runner.place_on_field(0, "PROT-NONE", None);

    runner.play(0, 0);

    assert!(
        runner.pending_selection().is_none(),
        "P-215 must not install a selection when no eligible cost card is available"
    );
}

/// After picking a cost card from hand, the card is placed as the bottom
/// digivolution source of Icemon, and a protect-target selection is installed.
#[test]
fn p_215_on_play_places_cost_card_as_bottom_source_and_then_prompts_protect() {
    let cost_card = make_ice_snow_lv3("COST-PLACE");
    let protect_digi = make_ice_snow_lv5("PROT-PLACE");

    let mut runner = DebugRunner::builder()
        .dsl_card("P-215")
        .expect("P-215 YAML parses and compiles")
        .add_card(cost_card)
        .add_card(protect_digi)
        .hand(0, &["P-215", "COST-PLACE"])
        .memory(10)
        .start();

    let protect_handle = runner.place_on_field(0, "PROT-PLACE", None);

    runner.play(0, 0);

    // Expect a UnionZone selection.
    assert!(
        matches!(runner.pending_kind(), Some(SelectionKind::UnionZone { .. })),
        "must install UnionZone selection after play"
    );

    // Record hand size before accepting cost.
    let hand_before = runner.hand_size(0);

    // Accept the cost by picking the first action (the hand card) — ONE step only.
    resolve_one(&mut runner).expect("union-zone cost pick must resolve");

    // The hand card should have been removed from hand (placed as bottom source).
    let hand_after = runner.hand_size(0);
    assert!(
        hand_after < hand_before,
        "cost card must leave hand after being placed as bottom source (before: {hand_before}, after: {hand_after})"
    );

    // The cost card must be THIS Icemon's bottom digivolution card ("as this
    // Digimon's bottom digivolution card" — DCGO AddDigivolutionCardsBottom
    // on card.PermanentOfThisCard()).
    let icemon = runner
        .game
        .player(0)
        .battle_area
        .iter()
        .find(|p| p.top_card().card_id(&runner.game.card_data) == "P-215")
        .expect("P-215 must be on the battle area");
    // digivolution_cards() is the WHOLE stack including the top card; a
    // freshly played P-215 has stack [P-215], so after the cost the stack is
    // [COST-PLACE, P-215] (bottom insert = index 0, Permanent::push_under).
    let sources = icemon.digivolution_cards();
    assert_eq!(
        sources.len(),
        2,
        "P-215's stack must be [cost card, P-215] after paying the cost"
    );
    assert_eq!(
        sources[0].card_id(&runner.game.card_data),
        "COST-PLACE",
        "the placed cost card must sit at the BOTTOM of the digivolution stack"
    );

    // The next selection should be the protect-target (OwnField kind).
    let kind = runner
        .pending_kind()
        .expect("protect-target selection must install after cost payment");
    assert!(
        matches!(kind, SelectionKind::OwnField),
        "after cost payment, P-215 must install an OwnField protect-target selection, got {kind:?}"
    );

    let _ = protect_handle; // suppress unused-variable warning
}

/// After placing the cost card and selecting a protect target, the target
/// gains CannotBeReturnedToHand, CannotBeReturnedToDeck, and
/// CannotBeDeDigivolved modifiers.
#[test]
fn p_215_on_play_grants_all_three_protection_modifiers() {
    let cost_card = make_rock_lv4("COST-MOD");
    let protect_digi = make_ice_snow_lv5("PROT-MOD");

    let mut runner = DebugRunner::builder()
        .dsl_card("P-215")
        .expect("P-215 YAML parses and compiles")
        .add_card(cost_card)
        .add_card(protect_digi)
        .hand(0, &["P-215", "COST-MOD"])
        .memory(10)
        .start();

    let protect_handle = runner.place_on_field(0, "PROT-MOD", None);

    runner.play(0, 0);

    // Resolve cost pick (UnionZone) — one step.
    assert!(matches!(
        runner.pending_kind(),
        Some(SelectionKind::UnionZone { .. })
    ));
    resolve_one(&mut runner).expect("cost pick resolves");

    // Resolve protect-target pick (OwnField) — one step.
    assert!(matches!(
        runner.pending_kind(),
        Some(SelectionKind::OwnField)
    ));
    resolve_one(&mut runner).expect("protect target pick resolves");

    assert!(
        runner
            .game
            .modifiers
            .has(protect_handle, ModifierType::CannotBeReturnedToHand),
        "P-215 must grant CannotBeReturnedToHand to the protect target"
    );
    assert!(
        runner
            .game
            .modifiers
            .has(protect_handle, ModifierType::CannotBeReturnedToDeck),
        "P-215 must grant CannotBeReturnedToDeck to the protect target"
    );
    assert!(
        runner
            .game
            .modifiers
            .has(protect_handle, ModifierType::CannotBeDeDigivolved),
        "P-215 must grant CannotBeDeDigivolved to the protect target"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 4 — Protect target filter: only Ice-Snow / Mineral / Rock Digimon
// ═══════════════════════════════════════════════════════════════════════════════

/// If the only Digimon on field has no matching trait, after the cost is paid,
/// the protect-target selection either auto-resolves with 0 candidates, or
/// the selection is absent. Either way the modifier must NOT be granted to
/// a non-matching Digimon.
#[test]
fn p_215_protection_not_granted_to_non_matching_digimon() {
    let cost_card = make_mineral_lv4("COST-SKIP");
    let plain_digi = make_plain_digi("PLAIN-SKIP");

    let mut runner = DebugRunner::builder()
        .dsl_card("P-215")
        .expect("P-215 YAML parses and compiles")
        .add_card(cost_card)
        .add_card(plain_digi)
        .hand(0, &["P-215", "COST-SKIP"])
        .memory(10)
        .start();

    let plain_handle = runner.place_on_field(0, "PLAIN-SKIP", None);

    runner.play(0, 0);

    // Pay cost.
    assert!(matches!(
        runner.pending_kind(),
        Some(SelectionKind::UnionZone { .. })
    ));
    resolve_one(&mut runner).expect("cost pick resolves");

    // After cost, P-215 (Ice-Snow) is on field but PLAIN-SKIP is not. The
    // protect-target filter includes P-215 itself (it has Ice-Snow trait).
    // Resolve any remaining selection.
    if runner.pending_selection().is_some() {
        runner.auto_resolve().ok();
    }

    assert!(
        !runner
            .game
            .modifiers
            .has(plain_handle, ModifierType::CannotBeReturnedToHand),
        "P-215 must not grant CannotBeReturnedToHand to a non-Ice-Snow/Mineral/Rock Digimon"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 5 — Cost filter: level ≤ 4 and trait constraint
// ═══════════════════════════════════════════════════════════════════════════════

/// A level-5 Ice-Snow card in hand is NOT eligible as the cost card.
/// The union zone selection must exclude it, so if no other eligible card
/// exists, no selection installs.
#[test]
fn p_215_level_5_card_not_eligible_as_cost() {
    let high_level = make_ice_snow_lv5("HIGH-COST");
    let protect_digi = make_ice_snow_lv5("PROT-LVL");

    let mut runner = DebugRunner::builder()
        .dsl_card("P-215")
        .expect("P-215 YAML parses and compiles")
        .add_card(high_level)
        .add_card(protect_digi)
        .hand(0, &["P-215", "HIGH-COST"])
        .memory(10)
        .start();

    runner.place_on_field(0, "PROT-LVL", None);

    runner.play(0, 0);

    // HIGH-COST is level 5 — must be filtered out. No eligible cost card →
    // no union zone selection installs AND the if-block skips (no binding) →
    // no OwnField selection either.
    assert!(
        runner.pending_selection().is_none(),
        "P-215 must not install a selection when the only hand card has level > 4"
    );
}

/// A level-4 card with NO matching trait is NOT eligible as the cost card.
#[test]
fn p_215_no_trait_card_not_eligible_as_cost() {
    let no_trait = make_no_trait_lv3("NO-TRAIT-COST");
    let protect_digi = make_ice_snow_lv5("PROT-NT");

    let mut runner = DebugRunner::builder()
        .dsl_card("P-215")
        .expect("P-215 YAML parses and compiles")
        .add_card(no_trait)
        .add_card(protect_digi)
        .hand(0, &["P-215", "NO-TRAIT-COST"])
        .memory(10)
        .start();

    runner.place_on_field(0, "PROT-NT", None);

    runner.play(0, 0);

    assert!(
        runner.pending_selection().is_none(),
        "P-215 must not install a selection when the only hand card has no matching trait"
    );
}

/// A LEVEL-LESS card (e.g. an Option card) with a matching trait is NOT
/// eligible as the cost card. Printed text requires a "level 4 or lower ...
/// trait card"; DCGO's CardSelectCondition requires `HasLevel && Level <= 4`,
/// and the DSL `level_lte` predicate fails closed on `level: None`.
#[test]
fn p_215_level_less_trait_card_not_eligible_as_cost() {
    let mut levelless = make_test_card("LVLESS-COST", "LevellessIceOption");
    levelless.card_kind = CardKind::Option;
    levelless.level = None;
    levelless.dp = None;
    levelless.traits.push("Ice-Snow".to_string());
    let protect_digi = make_ice_snow_lv5("PROT-LL");

    let mut runner = DebugRunner::builder()
        .dsl_card("P-215")
        .expect("P-215 YAML parses and compiles")
        .add_card(levelless)
        .add_card(protect_digi)
        .hand(0, &["P-215"])
        .memory(10)
        .start();

    push_to_trash(&mut runner, 0, "LVLESS-COST");
    runner.place_on_field(0, "PROT-LL", None);

    runner.play(0, 0);

    assert!(
        runner.pending_selection().is_none(),
        "P-215 must not install a selection when the only candidate is a \
         level-less trait card (printed cost requires 'level 4 or lower')"
    );
}

/// The cost is OPTIONAL ("By placing..." — DCGO `canNoSelect: true`): with an
/// eligible cost card available, the player may PASS the union-zone selection.
/// Declining must skip the whole effect — no source placed, no protect-target
/// selection, no modifiers granted.
#[test]
fn p_215_declining_optional_cost_skips_entire_effect() {
    let cost_card = make_ice_snow_lv3("COST-DECLINE");
    let protect_digi = make_ice_snow_lv5("PROT-DECLINE");

    let mut runner = DebugRunner::builder()
        .dsl_card("P-215")
        .expect("P-215 YAML parses and compiles")
        .add_card(cost_card)
        .add_card(protect_digi)
        .hand(0, &["P-215", "COST-DECLINE"])
        .memory(10)
        .start();

    let protect_handle = runner.place_on_field(0, "PROT-DECLINE", None);

    runner.play(0, 0);

    // The optional union-zone cost prompt installs (an eligible card exists).
    let selecting_player = {
        let sel = runner
            .pending_selection()
            .expect("union-zone cost selection must install");
        assert!(
            matches!(runner.pending_kind(), Some(SelectionKind::UnionZone { .. })),
            "first selection must be the UnionZone cost prompt"
        );
        assert!(sel.is_optional, "the cost selection must be declinable");
        sel.selecting_player
    };

    let hand_before = runner.hand_size(0);

    // Decline the cost.
    runner
        .execute_action(selecting_player, PASS)
        .expect("PASS on the optional cost selection must be legal");

    // Nothing else may happen: no protect-target selection, no hand change,
    // no source placed under P-215, no protection modifiers.
    assert!(
        runner.pending_selection().is_none(),
        "declining the cost must skip the protect-target selection entirely"
    );
    assert_eq!(
        runner.hand_size(0),
        hand_before,
        "declining the cost must not remove any card from hand"
    );
    let icemon = runner
        .game
        .player(0)
        .battle_area
        .iter()
        .find(|p| p.top_card().card_id(&runner.game.card_data) == "P-215")
        .expect("P-215 must be on the battle area");
    assert_eq!(
        icemon.digivolution_cards().len(),
        1,
        "declining the cost must leave P-215's stack untouched (top card only)"
    );
    for modifier in [
        ModifierType::CannotBeReturnedToHand,
        ModifierType::CannotBeReturnedToDeck,
        ModifierType::CannotBeDeDigivolved,
    ] {
        assert!(
            !runner.game.modifiers.has(protect_handle, modifier),
            "declining the cost must not grant {modifier:?}"
        );
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 6 — Expiry: modifiers expire at end of opponent's turn
// ═══════════════════════════════════════════════════════════════════════════════

/// Protection modifiers persist through the end of the current (your) turn —
/// they last until end of OPPONENT's turn, not your own.
#[test]
fn p_215_protection_persists_through_own_turn_end() {
    let cost_card = make_ice_snow_lv3("COST-EXP1");
    let protect_digi = make_ice_snow_lv5("PROT-EXP1");

    let mut runner = DebugRunner::builder()
        .dsl_card("P-215")
        .expect("P-215 YAML parses and compiles")
        .add_card(cost_card)
        .add_card(protect_digi)
        .hand(0, &["P-215", "COST-EXP1"])
        .memory(10)
        .start();

    let protect_handle = runner.place_on_field(0, "PROT-EXP1", None);

    runner.play(0, 0);
    assert!(matches!(
        runner.pending_kind(),
        Some(SelectionKind::UnionZone { .. })
    ));
    resolve_one(&mut runner).expect("cost pick resolves");
    assert!(matches!(
        runner.pending_kind(),
        Some(SelectionKind::OwnField)
    ));
    resolve_one(&mut runner).expect("protect target pick resolves");

    // Modifiers must be active immediately.
    assert!(runner
        .game
        .modifiers
        .has(protect_handle, ModifierType::CannotBeReturnedToHand));
    assert!(runner
        .game
        .modifiers
        .has(protect_handle, ModifierType::CannotBeDeDigivolved));

    // End player 0's turn — modifiers should still be active at start of opponent's turn.
    runner.end_turn();

    assert!(
        runner
            .game
            .modifiers
            .has(protect_handle, ModifierType::CannotBeReturnedToHand),
        "CannotBeReturnedToHand must still be active at start of opponent's turn"
    );
    assert!(
        runner
            .game
            .modifiers
            .has(protect_handle, ModifierType::CannotBeReturnedToDeck),
        "CannotBeReturnedToDeck must still be active at start of opponent's turn"
    );
    assert!(
        runner
            .game
            .modifiers
            .has(protect_handle, ModifierType::CannotBeDeDigivolved),
        "CannotBeDeDigivolved must still be active at start of opponent's turn"
    );
}

/// After opponent's turn ends, all three protection modifiers expire.
#[test]
fn p_215_protection_expires_after_opponents_turn_ends() {
    let cost_card = make_ice_snow_lv3("COST-EXP2");
    let protect_digi = make_ice_snow_lv5("PROT-EXP2");
    // Player 1 needs deck cards so end_turn draw does not abort the turn.
    let p1_filler = make_plain_digi("P1-PAD");

    let mut runner = DebugRunner::builder()
        .dsl_card("P-215")
        .expect("P-215 YAML parses and compiles")
        .add_card(cost_card)
        .add_card(protect_digi)
        .add_card(p1_filler)
        .hand(0, &["P-215", "COST-EXP2"])
        .deck(1, &["P1-PAD", "P1-PAD", "P1-PAD"])
        .memory(10)
        .start();

    let protect_handle = runner.place_on_field(0, "PROT-EXP2", None);

    runner.play(0, 0);
    assert!(matches!(
        runner.pending_kind(),
        Some(SelectionKind::UnionZone { .. })
    ));
    resolve_one(&mut runner).expect("cost pick resolves");
    assert!(matches!(
        runner.pending_kind(),
        Some(SelectionKind::OwnField)
    ));
    resolve_one(&mut runner).expect("protect target pick resolves");

    // End player 0's turn → player 1's turn.
    runner.end_turn();
    // End player 1's turn → back to player 0; modifiers should expire.
    runner.end_turn();

    assert!(
        !runner
            .game
            .modifiers
            .has(protect_handle, ModifierType::CannotBeReturnedToHand),
        "CannotBeReturnedToHand must expire after opponent's turn ends"
    );
    assert!(
        !runner
            .game
            .modifiers
            .has(protect_handle, ModifierType::CannotBeReturnedToDeck),
        "CannotBeReturnedToDeck must expire after opponent's turn ends"
    );
    assert!(
        !runner
            .game
            .modifiers
            .has(protect_handle, ModifierType::CannotBeDeDigivolved),
        "CannotBeDeDigivolved must expire after opponent's turn ends"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 7 — When Digivolving timing
// ═══════════════════════════════════════════════════════════════════════════════

/// [When Digivolving]: a digivolved P-215 stack fires the on_play/when_digivolving
/// clause. We use place_stack (top=P-215, bottom=BASE-WD) and fire_on_play —
/// this dispatches the when_digivolving path (DebugRunner has no dedicated
/// fire_when_digivolving helper; fire_on_play covers the same clause, matching
/// the p_186 test pattern).
#[test]
fn p_215_when_digivolving_fires_effect() {
    let cost_card = make_mineral_lv4("COST-WD");
    let base_digi = make_ice_snow_lv3("BASE-WD"); // lv3 base for the digivolution stack
    let protect_digi = make_ice_snow_lv5("PROT-WD");

    let mut runner = DebugRunner::builder()
        .dsl_card("P-215")
        .expect("P-215 YAML parses and compiles")
        .add_card(cost_card)
        .add_card(base_digi)
        .add_card(protect_digi)
        .hand(0, &["COST-WD"])
        .memory(10)
        .start();

    // Build a digivolution stack: BASE-WD at bottom, P-215 on top.
    // place_stack takes [bottom..., top] — last element is the top card.
    let stack = runner.place_stack(0, &["BASE-WD", "P-215"]);
    runner.place_on_field(0, "PROT-WD", None);

    // fire_on_play dispatches the on_play/when_digivolving clause.
    runner.fire_on_play(0, stack.index as usize);

    let kind = runner
        .pending_kind()
        .expect("when_digivolving must install a selection");
    assert!(
        matches!(kind, SelectionKind::UnionZone { .. }),
        "P-215 when_digivolving must install a UnionZone cost selection, got {kind:?}"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 9 — [When Moving] scope: fires ONLY when THIS Digimon moves
//
// The printed timing is [When Moving] — "when this Digimon moves from the
// breeding area to the battle area". DCGO P_215.cs gates the OnMove trigger
// with `permanent == card.PermanentOfThisCard()`. The engine's OnMove dispatch
// (`TriggerSource::MovedFromBreeding`) is a deliberate BOARD-WIDE observer
// scan (needed by "when one of your Digimon moves" cards like P-123 / P-130 /
// BT16-082), so a self-scoped [When Moving] clause must gate itself with
// `event_permanent_is_source: true` (the EX11-038 idiom).
// ═══════════════════════════════════════════════════════════════════════════════

/// Positive direction: P-215 ITSELF moves from breeding to battle → its
/// [When Moving] fires and installs the UnionZone cost selection.
#[test]
fn p_215_when_moving_fires_when_itself_moves_from_breeding() {
    let cost_card = make_ice_snow_lv3("COST-SELFMOVE");

    let mut runner = DebugRunner::builder()
        .dsl_card("P-215")
        .expect("P-215 YAML parses and compiles")
        .add_card(cost_card)
        .hand(0, &["COST-SELFMOVE"])
        .memory(10)
        .start();

    runner.place_in_breeding(0, "P-215");
    assert!(
        runner.move_from_breeding(0),
        "P-215 (level 4) must be able to move from breeding to battle"
    );

    let kind = runner
        .pending_kind()
        .expect("[When Moving] must fire when P-215 itself moves from breeding");
    assert!(
        matches!(kind, SelectionKind::UnionZone { .. }),
        "P-215 when_moving must install the UnionZone cost selection, got {kind:?}"
    );
}

/// Bug repro (user report): a DIFFERENT Digimon of the same player moves from
/// breeding — P-215's [When Moving] must NOT fire.
#[test]
fn p_215_when_moving_does_not_fire_when_another_own_digimon_moves() {
    let cost_card = make_ice_snow_lv3("COST-OTHERMOVE");
    let mover = make_ice_snow_lv3("MOVER-OWN"); // level 3 → movable from breeding

    let mut runner = DebugRunner::builder()
        .dsl_card("P-215")
        .expect("P-215 YAML parses and compiles")
        .add_card(cost_card)
        .add_card(mover)
        .hand(0, &["COST-OTHERMOVE"])
        .memory(10)
        .start();

    // P-215 already sits on the battle area (an eligible cost card is in hand,
    // so a mis-scoped trigger WOULD prompt).
    runner.place_on_field(0, "P-215", None);
    runner.place_in_breeding(0, "MOVER-OWN");
    assert!(
        runner.move_from_breeding(0),
        "the other Digimon must move from breeding to battle"
    );

    assert!(
        runner.pending_selection().is_none(),
        "P-215's [When Moving] must NOT fire when a DIFFERENT Digimon moves \
         from breeding (printed scope is 'this Digimon'), but a selection \
         was installed: {:?}",
        runner.pending_kind()
    );
}

/// Bug repro (worse direction): an OPPONENT's Digimon moves from breeding —
/// P-215's [When Moving] must NOT fire either.
#[test]
fn p_215_when_moving_does_not_fire_when_opponents_digimon_moves() {
    let cost_card = make_ice_snow_lv3("COST-OPPMOVE");
    let mover = make_ice_snow_lv3("MOVER-OPP");

    let mut runner = DebugRunner::builder()
        .dsl_card("P-215")
        .expect("P-215 YAML parses and compiles")
        .add_card(cost_card)
        .add_card(mover)
        .hand(0, &["COST-OPPMOVE"])
        .memory(10)
        .start();

    runner.place_on_field(0, "P-215", None);
    runner.place_in_breeding(1, "MOVER-OPP");
    assert!(
        runner.move_from_breeding(1),
        "the opponent's Digimon must move from breeding to battle"
    );

    assert!(
        runner.pending_selection().is_none(),
        "P-215's [When Moving] must NOT fire when an OPPONENT's Digimon moves \
         from breeding, but a selection was installed: {:?}",
        runner.pending_kind()
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 8 — Cost card from trash
// ═══════════════════════════════════════════════════════════════════════════════

/// The cost card can come from trash. After resolving the union-zone
/// selection with only a trash card available, the cost card is moved to
/// Icemon's digivolution sources, and the protect-target selection installs.
#[test]
fn p_215_on_play_can_use_trash_card_as_cost() {
    let cost_card = make_rock_lv4("COST-FROM-TRASH");
    let protect_digi = make_ice_snow_lv5("PROT-FT");

    let mut runner = DebugRunner::builder()
        .dsl_card("P-215")
        .expect("P-215 YAML parses and compiles")
        .add_card(cost_card)
        .add_card(protect_digi)
        .hand(0, &["P-215"])
        .memory(10)
        .start();

    push_to_trash(&mut runner, 0, "COST-FROM-TRASH");
    runner.place_on_field(0, "PROT-FT", None);

    let trash_before = runner.trash_size(0);

    runner.play(0, 0);
    assert!(matches!(
        runner.pending_kind(),
        Some(SelectionKind::UnionZone { .. })
    ));
    resolve_one(&mut runner).expect("cost pick from trash resolves");

    // Trash size must have decreased (card moved from trash to digi-source).
    let trash_after = runner.trash_size(0);
    assert!(
        trash_after < trash_before,
        "cost card from trash must be removed from trash (before: {trash_before}, after: {trash_after})"
    );

    // Protect-target selection must follow.
    assert!(
        matches!(runner.pending_kind(), Some(SelectionKind::OwnField)),
        "protect-target selection must install after trash cost card is placed"
    );
}
