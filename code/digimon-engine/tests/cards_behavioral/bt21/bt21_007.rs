//! BT21-007 Agumon — Digimon, Lv.3, Red, DP 1000, Cost 3.
//! Traits: Reptile
//!
//! # Card text (cards.json)
//!
//! [On Play] You may return 1 Digimon card with the [Reptile] or [Dragonkin]
//! trait from your trash to the hand.
//!
//! Inherited: [Your Turn] This Digimon gets +2000 DP.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT21/Red/BT21_007.cs  (absent — not found in repo; rely on printed text)
//!
//! # Patterns this test covers
//! - A4  trash → hand recursion (optional, trait-filtered)
//! - D4  declarative aura inherited DP modifier (+2000, [Your Turn])
//! - E2-adjacent: optional OnPlay clause ("you may"; no OPT tag)
//!
//! # Clause summary
//!
//! | # | Timing    | Scope     | Optional | OPT   | DSL shape |
//! |---|-----------|-----------|----------|-------|-----------|
//! | 1 | on_play   | FaceUp    | yes      | no    | select_trash + add_to_hand_from_trash |
//! | 2 | (aura)    | Inherited | n/a      | n/a   | kind: aura, active_when: your_turn, dp_modifier: 2000 |
//!
//! # Known gaps
//! None — the inherited clause is an AURA (declarative dp_modifier), not a triggered
//! effect, so G-INHERITED-DISPATCH does not apply.  BT23-005 uses the identical
//! `scope: inherited / kind: aura / active_when: { your_turn: true } / dp_modifier: 2000`
//! shape and passes all structural + behavioral tests.

#[path = "../../support/dsl_card_data.rs"]
mod dsl_card_data;

use digimon_dsl::compiled::{CompiledClause, CompiledDeclarativeClause, CompiledScope, CompiledTiming};
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::selection::SelectionKind;

// ─── Helper builders ─────────────────────────────────────────────────────────

/// Build a Digimon card with the [Reptile] trait.
fn make_reptile(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.traits = vec!["Reptile".to_string()];
    c
}

/// Build a Digimon card with the [Dragonkin] trait.
fn make_dragonkin(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.traits = vec!["Dragonkin".to_string()];
    c
}

/// Build a card with no Reptile/Dragonkin trait (ineligible for OnPlay select).
fn make_filler(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.traits = vec![];
    c
}

/// Push a card (by ID) into P0's trash by direct CardSource injection.
///
/// Panics if `card_id` is not registered in `runner.game.card_data`.
fn push_to_trash(runner: &mut DebugRunner, card_id: &str) {
    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .unwrap_or_else(|| panic!("{card_id} not found in card_data"));
    let next = runner.game.next_card_index();
    let src = CardSource::new(data_idx, 0, next);
    runner.game.players[0].trash.push(src);
}

/// Standard base runner: Agumon in hand, memory=10, deck has one filler.
/// After building, add a [Reptile] card to trash via `push_to_trash`.
fn agumon_runner() -> DebugRunner {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT21-007")
        .expect("BT21-007 found in embedded DSL pack")
        .add_card(make_reptile("REPTILE-TRASH"))
        .add_card(make_filler("FILLER-DECK"))
        .hand(0, &["BT21-007"])
        .deck(0, &["FILLER-DECK"])
        .memory(10)
        .start();
    push_to_trash(&mut runner, "REPTILE-TRASH");
    runner
}

// ─── Section 1: Structural assertions ────────────────────────────────────────

/// BT21-007 must compile to exactly 2 clauses:
///   [0] Triggered OnPlay (FaceUp scope, optional, NOT once_per_turn)
///   [1] Declarative Aura (Inherited scope, dp_modifier = +2000)
#[test]
fn bt21_007_compiles_to_two_clauses() {
    let runner = DebugRunner::builder()
        .dsl_card("BT21-007")
        .expect("BT21-007 found in embedded DSL pack")
        .memory(0)
        .start();

    let compiled = runner
        .compiled_card("BT21-007")
        .expect("BT21-007 in compiled_cards");

    assert_eq!(
        compiled.effects.len(),
        2,
        "BT21-007 must have exactly 2 clauses: triggered OnPlay + inherited Aura; \
         got {}",
        compiled.effects.len()
    );
}

/// Clause 0 must be a triggered clause with OnPlay timing, FaceUp scope,
/// optional: true, once_per_turn: false.
#[test]
fn bt21_007_on_play_clause_is_optional_face_up_not_opt() {
    let runner = DebugRunner::builder()
        .dsl_card("BT21-007")
        .expect("BT21-007 found in embedded DSL pack")
        .memory(0)
        .start();

    let compiled = runner
        .compiled_card("BT21-007")
        .expect("BT21-007 in compiled_cards");

    let triggered: Vec<&digimon_dsl::compiled::CompiledTriggeredClause> = compiled
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();

    assert_eq!(triggered.len(), 1, "exactly one triggered clause");

    let on_play = triggered
        .iter()
        .find(|t| t.when.contains(&CompiledTiming::OnPlay))
        .expect("triggered clause must have OnPlay timing");

    assert_eq!(
        on_play.scope,
        CompiledScope::FaceUp,
        "OnPlay clause must be FaceUp (default own) scope"
    );
    assert!(
        on_play.optional,
        "OnPlay clause must be optional (\"You may\")"
    );
    assert!(
        !on_play.once_per_turn,
        "OnPlay clause must NOT be once_per_turn (not in printed text)"
    );
}

/// Clause 1 must be a declarative Aura with Inherited scope and dp_modifier = 2000.
#[test]
fn bt21_007_inherited_aura_clause_shape() {
    let runner = DebugRunner::builder()
        .dsl_card("BT21-007")
        .expect("BT21-007 found in embedded DSL pack")
        .memory(0)
        .start();

    let compiled = runner
        .compiled_card("BT21-007")
        .expect("BT21-007 in compiled_cards");

    let aura = compiled.effects.iter().find_map(|c| match c {
        CompiledClause::Declarative(CompiledDeclarativeClause::Aura {
            scope,
            dp_modifier,
            ..
        }) => Some((*scope, *dp_modifier)),
        _ => None,
    });

    let (scope, dp) = aura.expect("BT21-007 must have a Declarative::Aura clause");
    assert_eq!(
        scope,
        CompiledScope::Inherited,
        "aura clause must have Inherited scope"
    );
    assert_eq!(dp, Some(2000), "aura dp_modifier must be +2000");
}

// ─── Section 2: Condition gating (OnPlay optional — Trash selection) ──────────

/// Negative: OnPlay with an empty trash produces NO pending selection.
/// The clause is optional; no valid targets means no prompt.
#[test]
fn bt21_007_on_play_no_selection_when_trash_empty() {
    // No REPTILE-TRASH in the builder, so trash starts empty.
    let mut runner = DebugRunner::builder()
        .dsl_card("BT21-007")
        .expect("BT21-007 found in embedded DSL pack")
        .add_card(make_filler("FILLER-DECK"))
        .hand(0, &["BT21-007"])
        .deck(0, &["FILLER-DECK"])
        .memory(10)
        .start();

    runner.play(0, 0);
    let _ = runner.auto_resolve();

    assert!(
        runner.pending_selection().is_none(),
        "no selection must install when P0's trash has no eligible cards"
    );
    // Hand must be empty: Agumon played, nothing returned.
    assert_eq!(runner.hand_size(0), 0, "hand must be empty when nothing returned");
}

/// Negative: trash has only a non-trait card → no eligible selection.
#[test]
fn bt21_007_on_play_no_selection_when_trash_has_no_eligible_targets() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT21-007")
        .expect("BT21-007 found in embedded DSL pack")
        .add_card(make_filler("FILLER-TRASH"))
        .add_card(make_filler("FILLER-DECK"))
        .hand(0, &["BT21-007"])
        .deck(0, &["FILLER-DECK"])
        .memory(10)
        .start();
    push_to_trash(&mut runner, "FILLER-TRASH");

    runner.play(0, 0);
    let _ = runner.auto_resolve();

    assert!(
        runner.pending_selection().is_none(),
        "no selection when trash contains only non-Reptile/non-Dragonkin cards"
    );
}

/// Positive: trash has a [Reptile] Digimon → Trash selection installs.
#[test]
fn bt21_007_on_play_installs_trash_selection_when_reptile_in_trash() {
    let mut runner = agumon_runner(); // has REPTILE-TRASH in trash

    runner.play(0, 0);

    // The selection parks pending.
    assert!(
        runner.pending_selection().is_some(),
        "Trash selection must install when a Reptile card is in trash"
    );
    let view = runner.pending_selection_view().unwrap();
    assert_eq!(
        view.kind,
        SelectionKind::Trash,
        "pending selection must be a Trash selection"
    );
    assert!(
        runner.pending_is_optional(),
        "Trash selection must be optional (\"you may\")"
    );
}

/// Positive: trash has a [Dragonkin] Digimon → Trash selection installs.
#[test]
fn bt21_007_on_play_installs_trash_selection_when_dragonkin_in_trash() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT21-007")
        .expect("BT21-007 found in embedded DSL pack")
        .add_card(make_dragonkin("DRAGON-TRASH"))
        .add_card(make_filler("FILLER-DECK"))
        .hand(0, &["BT21-007"])
        .deck(0, &["FILLER-DECK"])
        .memory(10)
        .start();
    push_to_trash(&mut runner, "DRAGON-TRASH");

    runner.play(0, 0);

    assert!(
        runner.pending_selection().is_some(),
        "Trash selection must install when a Dragonkin card is in trash"
    );
    let view = runner.pending_selection_view().unwrap();
    assert_eq!(
        view.kind,
        SelectionKind::Trash,
        "pending selection must be a Trash selection"
    );
}

// ─── Section 3: Behavioral — OnPlay clause ────────────────────────────────────

/// Picking a [Reptile] card from trash moves it to hand; trash shrinks by 1.
#[test]
fn bt21_007_on_play_pick_reptile_moves_to_hand() {
    let mut runner = agumon_runner(); // trash: [REPTILE-TRASH], hand: [BT21-007]

    let trash_before = runner.trash_size(0); // 1

    runner.play(0, 0); // hand → 0, field +1, OnPlay fires

    // Selection must be pending (REPTILE-TRASH is eligible).
    assert!(
        runner.pending_selection().is_some(),
        "Trash selection must be pending after play"
    );
    let view = runner.pending_selection_view().unwrap();
    let _ = runner.execute_action(0, view.valid_action_ids[0]);
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.trash_size(0),
        trash_before - 1,
        "trash must shrink by 1 after returning card to hand"
    );
    assert_eq!(
        runner.hand_size(0),
        1, // REPTILE-TRASH returned
        "hand must have exactly 1 card (the returned Reptile)"
    );
}

/// Picking a [Dragonkin] card from trash moves it to hand.
#[test]
fn bt21_007_on_play_pick_dragonkin_moves_to_hand() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT21-007")
        .expect("BT21-007 found in embedded DSL pack")
        .add_card(make_dragonkin("DRAGON-TRASH"))
        .add_card(make_filler("FILLER-DECK"))
        .hand(0, &["BT21-007"])
        .deck(0, &["FILLER-DECK"])
        .memory(10)
        .start();
    push_to_trash(&mut runner, "DRAGON-TRASH");

    let trash_before = runner.trash_size(0);

    runner.play(0, 0);

    assert!(runner.pending_selection().is_some(), "selection must be pending");
    let view = runner.pending_selection_view().unwrap();
    let _ = runner.execute_action(0, view.valid_action_ids[0]);
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.trash_size(0),
        trash_before - 1,
        "trash must shrink by 1 (Dragonkin returned to hand)"
    );
    assert_eq!(
        runner.hand_size(0),
        1,
        "hand must have exactly 1 card (the returned Dragonkin)"
    );
}

/// Declining the optional selection (PASS) leaves trash and hand unchanged.
#[test]
fn bt21_007_on_play_decline_does_not_move_card() {
    let mut runner = agumon_runner();

    let trash_before = runner.trash_size(0);

    runner.play(0, 0);

    // Selection must be pending.
    assert!(
        runner.pending_selection().is_some(),
        "Trash selection must be pending (REPTILE-TRASH in trash)"
    );
    let view = runner.pending_selection_view().unwrap();
    assert_eq!(view.kind, SelectionKind::Trash);
    assert!(runner.pending_is_optional(), "must be declinable");

    // Decline by executing PASS.
    use digimon_engine::action::space::PASS;
    let _ = runner.execute_action(0, PASS);
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.trash_size(0),
        trash_before,
        "declining must leave trash unchanged"
    );
    assert_eq!(
        runner.hand_size(0),
        0, // Agumon played, no card returned
        "declining must leave hand empty"
    );
}

/// Trash with both Reptile and Dragonkin cards — both appear as valid targets.
#[test]
fn bt21_007_on_play_both_trait_cards_are_offered() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT21-007")
        .expect("BT21-007 found in embedded DSL pack")
        .add_card(make_reptile("REP-TRASH"))
        .add_card(make_dragonkin("DRG-TRASH"))
        .add_card(make_filler("FILLER-DECK"))
        .hand(0, &["BT21-007"])
        .deck(0, &["FILLER-DECK"])
        .memory(10)
        .start();
    push_to_trash(&mut runner, "REP-TRASH");
    push_to_trash(&mut runner, "DRG-TRASH");

    runner.play(0, 0);

    assert!(runner.pending_selection().is_some(), "selection must install");
    let view = runner.pending_selection_view().unwrap();
    assert_eq!(view.kind, SelectionKind::Trash);
    assert_eq!(
        view.valid_action_ids.len(),
        2,
        "both Reptile and Dragonkin cards must appear as valid targets; got {}",
        view.valid_action_ids.len()
    );
}

// ─── Section 4: Cost firing — no cost side-effects ───────────────────────────
// BT21-007's OnPlay has no cost-as-side-effect (no security trash, no life loss).
// No event-log assertions are needed.

// ─── Section 5: No OPT — no lockout test needed ───────────────────────────────
// "[On Play] You may return…" has no [Once Per Turn] tag.
// BT21-007 can play multiple copies per turn, each triggering independently.

// ─── Section 6: Behavioral — inherited aura (+2000 DP) ───────────────────────

/// On the controller's own turn, Agumon as a digivolution source contributes
/// +2000 DP via `source_dp_contribution` at stack index 0.
#[test]
fn bt21_007_inherited_dp_contributes_on_your_turn() {
    let agumon_data = dsl_card_data::card_data_from_compiled("BT21-007");
    let mut carrier = make_test_card("CARRIER", "Carrier");
    carrier.level = Some(4);
    carrier.dp = Some(5000);

    let mut runner = DebugRunner::builder()
        .add_card(agumon_data)
        .add_card(carrier)
        .memory(10)
        .start();

    // Place Agumon on field (becomes source[0]).
    let perm_handle = runner.place_on_field(0, "BT21-007", Some(0));

    // Digivolve carrier on top by pushing its CardSource.
    {
        let carrier_data_idx = runner
            .game
            .card_data
            .iter()
            .position(|c| c.card_id == "CARRIER")
            .expect("CARRIER in card_data");
        let next_idx = runner.game.next_card_index();
        let carrier_source = CardSource::new(carrier_data_idx, 0, next_idx);
        let turn = runner.game.turn_count;
        runner.game.players[0].battle_area[perm_handle.index as usize]
            .digivolve(carrier_source, turn);
    }

    assert_eq!(runner.turn_player(), 0, "precondition: P0's turn");

    // Source[0] = Agumon (base), source[1] = carrier (top).
    let contribution = runner.game.source_dp_contribution(perm_handle, 0);

    assert_eq!(
        contribution, 2000,
        "Agumon inherited +2000 DP must contribute on controller's turn; got {contribution}"
    );
}

/// On the opponent's turn, [Your Turn] suppresses the +2000 DP.
#[test]
fn bt21_007_inherited_dp_inactive_on_opponents_turn() {
    let agumon_data = dsl_card_data::card_data_from_compiled("BT21-007");
    let mut carrier = make_test_card("CARRIER", "Carrier");
    carrier.level = Some(4);
    carrier.dp = Some(5000);

    let mut runner = DebugRunner::builder()
        .add_card(agumon_data)
        .add_card(carrier)
        .memory(10)
        .start();

    let perm_handle = runner.place_on_field(0, "BT21-007", Some(0));

    {
        let carrier_data_idx = runner
            .game
            .card_data
            .iter()
            .position(|c| c.card_id == "CARRIER")
            .expect("CARRIER in card_data");
        let next_idx = runner.game.next_card_index();
        let carrier_source = CardSource::new(carrier_data_idx, 0, next_idx);
        let turn = runner.game.turn_count;
        runner.game.players[0].battle_area[perm_handle.index as usize]
            .digivolve(carrier_source, turn);
    }

    // Advance to opponent's turn.
    runner.end_turn();
    assert_eq!(runner.turn_player(), 1, "precondition: P1's turn");

    let contribution = runner.game.source_dp_contribution(perm_handle, 0);

    assert_eq!(
        contribution, 0,
        "Agumon inherited +2000 DP must NOT contribute on opponent's turn; got {contribution}"
    );
}

/// The carrier top-card slot (source[1]) does NOT carry Agumon's buff.
#[test]
fn bt21_007_top_card_slot_does_not_carry_agumon_dp() {
    let agumon_data = dsl_card_data::card_data_from_compiled("BT21-007");
    let mut carrier = make_test_card("CARRIER", "Carrier");
    carrier.level = Some(4);
    carrier.dp = Some(5000);

    let mut runner = DebugRunner::builder()
        .add_card(agumon_data)
        .add_card(carrier)
        .memory(10)
        .start();

    let perm_handle = runner.place_on_field(0, "BT21-007", Some(0));

    {
        let carrier_data_idx = runner
            .game
            .card_data
            .iter()
            .position(|c| c.card_id == "CARRIER")
            .expect("CARRIER in card_data");
        let next_idx = runner.game.next_card_index();
        let carrier_source = CardSource::new(carrier_data_idx, 0, next_idx);
        let turn = runner.game.turn_count;
        runner.game.players[0].battle_area[perm_handle.index as usize]
            .digivolve(carrier_source, turn);
    }

    // Source[1] = carrier (top card) — has no effects, contribution must be 0.
    let contribution = runner.game.source_dp_contribution(perm_handle, 1);

    assert_eq!(
        contribution, 0,
        "carrier top-card slot must not carry Agumon's +2000 DP"
    );
}
