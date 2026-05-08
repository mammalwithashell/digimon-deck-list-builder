//! BT12-002 DemiVeemon — Digi-Egg, Lv.2, Blue.
//! Traits: Baby Dragon.
//!
//! # Card text (cards.json)
//!
//! Inherited Effect [When Attacking] [Once Per Turn]
//! If you have a green Digimon in play, <Draw 1>
//! (Draw 1 card from your deck.)
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT12/Blue/BT12_002.cs
//!
//! # Patterns this test covers (RUST_DSL_TEST_API.md §4 / §5)
//! - G4 Inherited When Attacking on DigiEgg (OPT + board-state condition)
//! - E2 OPT — once_per_turn enforced; clears after end_turn round-trip
//! - Condition gating: green Digimon present → fires; no green Digimon → no fire
//! - Negative: green Tamer only (kind: digimon filter) → no fire
//! - Behavioral: draw fires (hand+1, deck-1); mandatory (no optional prompt)
//! - Event-log note: GameEvent::Draw does not exist in events.rs; zone-delta
//!   observation (hand/deck size) is used instead.

use digimon_dsl::compiled::{CompiledClause, CompiledScope, CompiledTiming};
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind};

const DEMIVOEMON_YAML: &str = include_str!("../../../cards/bt12/BT12-002.yaml");

// ─── Card-data factories ─────────────────────────────────────────────────────

/// Build a green Digimon test card.
fn make_green_digimon(id: &str) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.colors = vec![CardColor::Green];
    card
}

/// Build a green Tamer (not a Digimon) — used for negative kind-filter test.
fn make_green_tamer(id: &str) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Tamer;
    card.colors = vec![CardColor::Green];
    card
}

/// Build a filler card (red Digimon, no special properties).
fn make_filler(id: &str) -> CardData {
    make_test_card(id, id)
}

// ─── Runner factories ─────────────────────────────────────────────────────────

/// Minimal runner: DemiVeemon registered, carrier filler, deck card for drawing.
/// Does NOT place any green Digimon on the board — caller handles board state.
fn base_runner() -> DebugRunner {
    DebugRunner::builder()
        .from_dsl_yaml(DEMIVOEMON_YAML)
        .expect("BT12-002 YAML parses")
        .add_card(make_filler("CARRIER"))
        .add_card(make_filler("DECK-CARD"))
        .add_card(make_green_digimon("GREEN-DIGI"))
        .add_card(make_green_tamer("GREEN-TAMER"))
        .deck(0, &["DECK-CARD", "DECK-CARD", "DECK-CARD"])
        .memory(5)
        .start()
}

/// Place DemiVeemon as a bottom digivolution source under CARRIER on P0's field.
/// Returns the PermanentHandle of the stacked permanent (the carrier).
fn place_demivoemon_as_source(
    runner: &mut DebugRunner,
) -> digimon_engine::permanent::PermanentHandle {
    let handle = runner.place_on_field(0, "CARRIER", Some(0));

    {
        let game = runner.game_mut();
        let data_idx = game
            .card_data
            .iter()
            .position(|c| c.card_id == "BT12-002")
            .expect("BT12-002 registered in card_data");
        let next = game.next_card_index();
        let mut demivoemon_src = CardSource::new(data_idx, 0, next);
        demivoemon_src.card_index = next;
        let perm = &mut game.players[0].battle_area[handle.index as usize];
        // Insert at position 0 so CARRIER stays on top (last = top).
        perm.card_sources.insert(0, demivoemon_src);
    }

    handle
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 1 — Structural assertions
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn bt12_002_yaml_parses_and_compiles() {
    let spec: digimon_dsl::spec::CardSpec =
        serde_yml::from_str(DEMIVOEMON_YAML).expect("BT12-002 YAML parses as CardSpec");
    let _compiled =
        digimon_dsl::compile::compile(&spec).expect("BT12-002 YAML compiles without errors");
}

#[test]
fn bt12_002_has_exactly_one_triggered_clause() {
    let runner = base_runner();
    let compiled = runner
        .compiled_card("BT12-002")
        .expect("BT12-002 compiled card present");

    let triggered: Vec<&digimon_dsl::compiled::CompiledTriggeredClause> = compiled
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
        "BT12-002 must have exactly one triggered clause"
    );
}

#[test]
fn bt12_002_clause_is_scope_inherited() {
    let runner = base_runner();
    let compiled = runner
        .compiled_card("BT12-002")
        .expect("BT12-002 compiled card present");

    let clause = compiled
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .expect("triggered clause must exist");

    assert_eq!(
        clause.scope,
        CompiledScope::Inherited,
        "BT12-002 clause must have scope: Inherited"
    );
}

#[test]
fn bt12_002_clause_when_is_when_attacking() {
    let runner = base_runner();
    let compiled = runner
        .compiled_card("BT12-002")
        .expect("BT12-002 compiled card present");

    let clause = compiled
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .expect("triggered clause must exist");

    assert!(
        clause.when.contains(&CompiledTiming::WhenAttacking),
        "BT12-002 clause must have when: WhenAttacking; got {:?}",
        clause.when
    );
}

#[test]
fn bt12_002_clause_is_once_per_turn() {
    let runner = base_runner();
    let compiled = runner
        .compiled_card("BT12-002")
        .expect("BT12-002 compiled card present");

    let clause = compiled
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .expect("triggered clause must exist");

    assert!(
        clause.once_per_turn,
        "BT12-002 clause must be once_per_turn ([Once Per Turn] in printed text)"
    );
}

#[test]
fn bt12_002_clause_is_not_optional() {
    // Printed text: "If you have a green Digimon in play, <Draw 1>" — no "you may".
    // DCGO: isOptional=false. Effect is mandatory when condition is met.
    let runner = base_runner();
    let compiled = runner
        .compiled_card("BT12-002")
        .expect("BT12-002 compiled card present");

    let clause = compiled
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .expect("triggered clause must exist");

    assert!(
        !clause.optional,
        "BT12-002 clause must NOT be optional (no 'you may' in printed text; DCGO isOptional=false)"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 2 — Condition gating
// ═══════════════════════════════════════════════════════════════════════════════

/// POSITIVE: Green Digimon on own battle area → inherited When Attacking fires → Draw 1.
#[test]
fn bt12_002_fires_draw_when_green_digimon_in_play() {
    let mut runner = base_runner();

    // Place DemiVeemon as inherited source under CARRIER.
    let carrier = place_demivoemon_as_source(&mut runner);

    // Place a green Digimon on P0's battle area (as a separate permanent).
    runner.place_on_field(0, "GREEN-DIGI", Some(1));

    let hand_before = runner.hand_size(0);
    let deck_before = runner.deck_size(0);

    // CARRIER (with DemiVeemon underneath) attacks P1.
    runner.attack_player(carrier, 1, false);
    let _ = runner.auto_resolve();

    let hand_after = runner.hand_size(0);
    let deck_after = runner.deck_size(0);

    assert_eq!(
        hand_after,
        hand_before + 1,
        "hand must grow by 1 when green Digimon is in play; \
         before={hand_before}, after={hand_after}"
    );
    assert_eq!(
        deck_after,
        deck_before - 1,
        "deck must shrink by 1 after Draw 1; \
         before={deck_before}, after={deck_after}"
    );
}

/// NEGATIVE: No green Digimon on field → condition not met → inherited clause does NOT fire.
#[test]
fn bt12_002_no_fire_when_no_green_digimon_in_play() {
    let mut runner = base_runner();

    // Place DemiVeemon as inherited source under CARRIER, but do NOT place any green Digimon.
    let carrier = place_demivoemon_as_source(&mut runner);

    let hand_before = runner.hand_size(0);
    let deck_before = runner.deck_size(0);

    runner.attack_player(carrier, 1, false);
    let _ = runner.auto_resolve();

    let hand_after = runner.hand_size(0);
    let deck_after = runner.deck_size(0);

    assert_eq!(
        hand_after, hand_before,
        "hand must NOT grow when no green Digimon is in play; \
         before={hand_before}, after={hand_after}"
    );
    assert_eq!(
        deck_after, deck_before,
        "deck must NOT shrink when condition fails; \
         before={deck_before}, after={deck_after}"
    );
}

/// NEGATIVE (kind filter): Green Tamer only (no green Digimon) → condition not met.
/// The `kind: digimon` predicate must exclude Tamers even if they are green.
#[test]
fn bt12_002_no_fire_with_green_tamer_but_no_green_digimon() {
    let mut runner = base_runner();

    let carrier = place_demivoemon_as_source(&mut runner);

    // Place a green Tamer — color matches but kind does NOT (must be kind: digimon).
    runner.place_on_field(0, "GREEN-TAMER", Some(1));

    let hand_before = runner.hand_size(0);
    let deck_before = runner.deck_size(0);

    runner.attack_player(carrier, 1, false);
    let _ = runner.auto_resolve();

    let hand_after = runner.hand_size(0);
    let deck_after = runner.deck_size(0);

    assert_eq!(
        hand_after, hand_before,
        "green Tamer must NOT satisfy the green Digimon condition; \
         hand before={hand_before}, after={hand_after}"
    );
    assert_eq!(
        deck_after, deck_before,
        "deck must NOT shrink when condition fails via kind filter; \
         before={deck_before}, after={deck_after}"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 3 — Behavioral
// ═══════════════════════════════════════════════════════════════════════════════

/// Draw is deterministic (no pending_selection installed by the effect itself).
/// The effect fires automatically without requiring player confirmation.
#[test]
fn bt12_002_draw_requires_no_pending_selection() {
    let mut runner = base_runner();

    let carrier = place_demivoemon_as_source(&mut runner);
    runner.place_on_field(0, "GREEN-DIGI", Some(1));

    runner.attack_player(carrier, 1, false);

    // After attack (which fires the inherited trigger and its draw immediately),
    // there must be no pending selection left from the draw itself.
    // (The attack may produce combat-resolution prompts; drain those.)
    let _ = runner.auto_resolve();

    assert!(
        runner.pending_selection().is_none(),
        "Draw 1 is deterministic and must not leave a pending selection after resolve"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 4 — Event-log (hand/deck delta as proxy; no GameEvent::Draw variant)
// ═══════════════════════════════════════════════════════════════════════════════

/// Verifies the draw delta precisely: hand+1, deck-1 when effect fires.
/// (GameEvent::Draw is not a variant in events.rs, so we use zone sizes.)
#[test]
fn bt12_002_draw_delta_is_exactly_one_card() {
    let mut runner = base_runner();

    let carrier = place_demivoemon_as_source(&mut runner);
    runner.place_on_field(0, "GREEN-DIGI", Some(1));

    let hand_before = runner.hand_size(0);
    let deck_before = runner.deck_size(0);

    runner.attack_player(carrier, 1, false);
    let _ = runner.auto_resolve();

    let hand_delta = runner.hand_size(0) as i32 - hand_before as i32;
    let deck_delta = runner.deck_size(0) as i32 - deck_before as i32;

    assert_eq!(
        hand_delta, 1,
        "Draw 1 must add exactly 1 card to hand; delta={hand_delta}"
    );
    assert_eq!(
        deck_delta, -1,
        "Draw 1 must remove exactly 1 card from deck; delta={deck_delta}"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 5 — OPT lockout
// ═══════════════════════════════════════════════════════════════════════════════

/// OPT: second attack same turn with green Digimon present → clause is locked out.
/// The hand must grow by exactly 1 total (first attack), not 2 (both attacks).
#[test]
fn bt12_002_opt_locks_out_second_attack_same_turn() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(DEMIVOEMON_YAML)
        .expect("BT12-002 YAML parses")
        .add_card(make_filler("CARRIER"))
        .add_card(make_filler("CARRIER2"))
        .add_card(make_filler("DECK-CARD"))
        .add_card(make_green_digimon("GREEN-DIGI"))
        .deck(0, &["DECK-CARD", "DECK-CARD", "DECK-CARD"])
        .memory(10)
        .start();

    // Place DemiVeemon under CARRIER as the primary attacker.
    let carrier = runner.place_on_field(0, "CARRIER", Some(0));
    {
        let game = runner.game_mut();
        let data_idx = game
            .card_data
            .iter()
            .position(|c| c.card_id == "BT12-002")
            .expect("BT12-002 registered");
        let next = game.next_card_index();
        let mut src = CardSource::new(data_idx, 0, next);
        src.card_index = next;
        game.players[0].battle_area[carrier.index as usize]
            .card_sources
            .insert(0, src);
    }

    // Place green Digimon on field to satisfy the condition.
    runner.place_on_field(0, "GREEN-DIGI", Some(1));

    let hand_before = runner.hand_size(0);

    // First attack: inherited When Attacking OPT fires → Draw 1.
    runner.attack_player(carrier, 1, false);
    let _ = runner.auto_resolve();
    let hand_after_first = runner.hand_size(0);

    let first_delta = hand_after_first as i32 - hand_before as i32;
    assert!(
        first_delta >= 1,
        "first attack must fire the OPT inherited draw; delta={first_delta}"
    );

    if runner.game_over() {
        return; // P1 had 0 security; game may end on first attack
    }

    // Re-attack with the same carrier (or get its handle back at index 0).
    let carrier2 = runner.perm_handle(0, 0);
    let hand_before_second = runner.hand_size(0);

    runner.attack_player(carrier2, 1, false);
    let _ = runner.auto_resolve();

    let hand_after_second = runner.hand_size(0);
    let second_delta = hand_after_second as i32 - hand_before_second as i32;

    assert_eq!(
        second_delta, 0,
        "second attack same turn: OPT must block the inherited draw; delta={second_delta}"
    );
}

/// OPT clears after end_turn round-trip: attack on the next turn fires again.
#[test]
fn bt12_002_opt_clears_after_end_turn() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(DEMIVOEMON_YAML)
        .expect("BT12-002 YAML parses")
        .add_card(make_filler("CARRIER"))
        .add_card(make_filler("DECK-CARD"))
        .add_card(make_green_digimon("GREEN-DIGI"))
        .deck(0, &["DECK-CARD", "DECK-CARD", "DECK-CARD", "DECK-CARD"])
        .deck(1, &["DECK-CARD"])
        .memory(10)
        .start();

    // Place DemiVeemon under CARRIER.
    let carrier = runner.place_on_field(0, "CARRIER", Some(0));
    {
        let game = runner.game_mut();
        let data_idx = game
            .card_data
            .iter()
            .position(|c| c.card_id == "BT12-002")
            .expect("BT12-002 registered");
        let next = game.next_card_index();
        let mut src = CardSource::new(data_idx, 0, next);
        src.card_index = next;
        game.players[0].battle_area[carrier.index as usize]
            .card_sources
            .insert(0, src);
    }
    runner.place_on_field(0, "GREEN-DIGI", Some(1));

    // Turn 1 attack: OPT fires.
    let hand_t1_before = runner.hand_size(0);
    runner.attack_player(carrier, 1, false);
    let _ = runner.auto_resolve();
    let hand_t1_after = runner.hand_size(0);
    let t1_delta = hand_t1_after as i32 - hand_t1_before as i32;
    assert!(
        t1_delta >= 1,
        "turn 1 attack must fire the OPT draw; delta={t1_delta}"
    );

    if runner.game_over() {
        return;
    }

    // End P0's turn, advance to P1's turn, then back to P0's turn.
    runner.end_turn(); // P1's turn begins
    runner.end_turn(); // P0's turn begins again

    // Carrier may have moved/been deleted during combat — re-fetch the first P0 perm.
    let battle_area_len = runner.game.players[0].battle_area.len();
    if battle_area_len == 0 {
        // No carrier survived — OPT test cannot proceed (game continued ok).
        return;
    }
    let carrier2 = runner.perm_handle(0, 0);

    let hand_t2_before = runner.hand_size(0);
    runner.attack_player(carrier2, 1, false);
    let _ = runner.auto_resolve();
    let hand_t2_after = runner.hand_size(0);
    let t2_delta = hand_t2_after as i32 - hand_t2_before as i32;

    assert!(
        t2_delta >= 1,
        "OPT must clear after end_turn: turn 2 attack must fire again; delta={t2_delta}"
    );
}
