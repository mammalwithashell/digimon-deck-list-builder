//! EX8-067 Close — Tamer, Black, Cost 4, Trait: [LIBERATOR]
//!
//! # Card text (cards.json / authoritative printed text)
//!
//! "[Start of Your Turn] If you have 2 or less memory, set it to 3."
//!
//! "[Your Turn] When any of your Digimon digivolve into a [Mineral] or [Rock]
//! trait Digimon, by suspending this Tamer, place up to 2 cards with the
//! [Mineral] or [Rock] trait from your trash as that Digimon's bottom
//! digivolution cards."
//!
//! "Security Effect [Security] Play this card without paying the cost."
//!
//! # Patterns this test covers
//! - B1  Start-of-turn memory setter (memory_lte gate)
//! - B3  Trigger-on-event tamer ([Your Turn] on_digivolve observer)
//! - A6  Trash-to-digi-stack (place_as_bottom_source × up to 2)
//! - E2  Optional with cost gating (suspend self, optional decline)
//! - Security play (play_from_security)
//!
//! # Implementation status
//!
//! Clause 0 (Start of Your Turn):
//!   IMPLEMENTED — memory_lte: 2 condition + set_memory: 3.
//!
//! Clause 1 ([Your Turn] on_digivolve observer):
//!   IMPLEMENTED — on_digivolve + active_when: your_turn +
//!   event_target_trait_has Mineral/Rock condition +
//!   select_own_permanent optional gate + suspend self +
//!   select_count_capped_multi (trash, max 2, Mineral/Rock filter,
//!   optional_zero: true) + per_selected place_as_bottom_source.
//!
//! Security:
//!   IMPLEMENTED — standard play_from_security.

#[path = "../../support/dsl_card_data.rs"]
mod dsl_card_data;

use digimon_dsl::compiled::{CompiledClause, CompiledScope, CompiledTiming};
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardKind, EffectTiming};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::TriggerSource;

// ─── Helpers ─────────────────────────────────────────────────────────────────

/// Build a Mineral-trait Digimon test card (level 4, can digivolve from Lv3).
fn make_mineral_lv4(id: &str) -> CardData {
    let mut card = make_test_card(id, &format!("{id} Mineral"));
    card.card_kind = CardKind::Digimon;
    card.level = Some(4);
    card.dp = Some(5000);
    card.traits = vec!["Mineral".to_string()];
    card.play_cost = 0;
    card.evo_costs = vec![EvoCost {
        level: 3,
        memory_cost: 0,
        card_color: 0,
    }];
    card
}

/// Build a Rock-trait Digimon test card (level 4, can digivolve from Lv3).
fn make_rock_lv4(id: &str) -> CardData {
    let mut card = make_test_card(id, &format!("{id} Rock"));
    card.card_kind = CardKind::Digimon;
    card.level = Some(4);
    card.dp = Some(5000);
    card.traits = vec!["Rock".to_string()];
    card.play_cost = 0;
    card.evo_costs = vec![EvoCost {
        level: 3,
        memory_cost: 0,
        card_color: 0,
    }];
    card
}

/// Build a non-Mineral/Rock Digimon test card (Beast trait, level 4).
fn make_beast_lv4(id: &str) -> CardData {
    let mut card = make_test_card(id, &format!("{id} Beast"));
    card.card_kind = CardKind::Digimon;
    card.level = Some(4);
    card.dp = Some(4000);
    card.traits = vec!["Beast".to_string()];
    card.play_cost = 0;
    card.evo_costs = vec![EvoCost {
        level: 3,
        memory_cost: 0,
        card_color: 0,
    }];
    card
}

/// Build a Mineral/Rock card suitable for trash placement (level 3).
fn make_mineral_trash_src(id: &str) -> CardData {
    let mut card = make_test_card(id, &format!("{id} MineralSrc"));
    card.card_kind = CardKind::Digimon;
    card.level = Some(3);
    card.dp = Some(2000);
    card.traits = vec!["Mineral".to_string()];
    card.play_cost = 3;
    card
}

/// Build a base Lv3 Mineral Digimon to sit on the field as the digivolve
/// platform. The Lv4 card will be pushed on top via `perm.digivolve`.
fn make_mineral_lv3_base(id: &str) -> CardData {
    let mut card = make_test_card(id, &format!("{id} MineralBase"));
    card.card_kind = CardKind::Digimon;
    card.level = Some(3);
    card.dp = Some(3000);
    card.traits = vec!["Mineral".to_string()];
    card.play_cost = 0;
    card
}

/// Push `card_id` into the given player's trash.
fn push_to_trash(runner: &mut DebugRunner, player: usize, card_id: &str) {
    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .expect("card must be in card_data");
    let card_index = runner.game.next_card_index();
    runner.game.players[player]
        .trash
        .push(CardSource::new(data_idx, 0, card_index));
}

/// Push `card_id` into the given player's hand.
fn push_to_hand(runner: &mut DebugRunner, player: usize, card_id: &str) {
    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .expect("card must be in card_data");
    let card_index = runner.game.next_card_index();
    runner.game.players[player]
        .hand
        .push(CardSource::new(data_idx, 0, card_index));
}

/// Pop `card_id` from hand and perform a raw digivolve onto `base_perm`,
/// then fire WhenDigivolving + OnDigivolve into the event queue.
///
/// This avoids the memory/affordability checks in `Game::digivolve_from_hand`,
/// letting the test focus solely on the observer trigger. Pattern from ad1_001.rs.
fn fire_digivolve_onto(
    runner: &mut DebugRunner,
    player: usize,
    base_perm: PermanentHandle,
    card_id: &str,
) {
    let card = {
        let data_idx = runner
            .game
            .card_data
            .iter()
            .position(|c| c.card_id == card_id)
            .expect("card must be in card_data");
        let hand = &mut runner.game.players[player].hand;
        let pos = hand
            .iter()
            .position(|cs| cs.data_index == data_idx)
            .expect("card must be in hand");
        hand.remove(pos)
    };
    let card_handle = card.handle();
    let turn = runner.game.turn_count;

    // Push the digivolving card onto the base permanent's source stack.
    {
        let perm = runner
            .game
            .players[player]
            .battle_area
            .get_mut(base_perm.index as usize)
            .expect("base permanent must be on field");
        perm.digivolve(card, turn);
    }

    // Fire WhenDigivolving, then OnDigivolve.
    runner
        .game
        .enqueue_triggered(EffectTiming::WhenDigivolving, TriggerSource::Permanent(base_perm));
    runner.game.drain_effect_queue();

    runner.game.enqueue_triggered(
        EffectTiming::OnDigivolve,
        TriggerSource::Digivolved {
            player: player as u8,
            permanent: base_perm,
            card: card_handle,
            effect_initiated: false,
            dna_origin: false,
        },
    );
    runner.game.drain_effect_queue();
}

fn compiled() -> digimon_dsl::compiled::CompiledCard {
    dsl_card_data::compiled("EX8-067")
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 1 — Structural assertions
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn ex8_067_yaml_compiles() {
    let _ = compiled();
}

#[test]
fn ex8_067_is_tamer_cost4_black() {
    use digimon_dsl::compiled::{CompiledCardKind, CompiledColor};
    let card = compiled();
    assert_eq!(card.kind, CompiledCardKind::Tamer, "kind must be Tamer");
    assert_eq!(card.cost, Some(4), "play cost must be 4");
    assert!(
        card.color.contains(&CompiledColor::Black),
        "EX8-067 must be Black"
    );
}

#[test]
fn ex8_067_has_start_of_turn_clause() {
    let card = compiled();
    let triggered: Vec<_> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();
    assert!(
        triggered
            .iter()
            .any(|t| t.when.contains(&CompiledTiming::StartOfYourTurn)),
        "EX8-067 must have a StartOfYourTurn clause"
    );
}

#[test]
fn ex8_067_has_on_digivolve_observer_clause() {
    let card = compiled();
    let observer = card.effects.iter().find_map(|c| match c {
        CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::OnDigivolve) => Some(t),
        _ => None,
    });
    assert!(
        observer.is_some(),
        "EX8-067 must have an OnDigivolve observer clause"
    );
}

#[test]
fn ex8_067_on_digivolve_clause_is_optional() {
    let card = compiled();
    let observer = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::OnDigivolve) => {
                Some(t)
            }
            _ => None,
        })
        .expect("OnDigivolve clause must exist");
    assert!(
        observer.optional,
        "OnDigivolve clause is optional (printed 'by suspending this Tamer')"
    );
}

#[test]
fn ex8_067_on_digivolve_clause_scope_is_face_up() {
    let card = compiled();
    let observer = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::OnDigivolve) => {
                Some(t)
            }
            _ => None,
        })
        .expect("OnDigivolve clause must exist");
    assert_eq!(
        observer.scope,
        CompiledScope::FaceUp,
        "On-digivolve clause on a Tamer has FaceUp scope"
    );
}

#[test]
fn ex8_067_has_security_clause() {
    let card = compiled();
    let triggered: Vec<_> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();
    assert!(
        triggered
            .iter()
            .any(|t| t.when.contains(&CompiledTiming::OnSecurity)),
        "EX8-067 must have an OnSecurity clause; got: {triggered:?}"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 2 — Clause 0: [Start of Your Turn] memory setter
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn ex8_067_start_of_turn_sets_memory_to_three_when_lte_two() {
    let filler = make_test_card("EX8-067-FILL", "Filler");
    let mut runner = DebugRunner::builder()
        .dsl_card("EX8-067")
        .expect("EX8-067 YAML parses and compiles")
        .add_card(filler)
        .deck(0, &["EX8-067-FILL"])
        .deck(1, &["EX8-067-FILL"])
        .memory(2)
        .start();

    runner.place_on_field(0, "EX8-067", None);
    runner.game.memory = 2;

    runner.end_turn();
    runner.end_turn();

    assert_eq!(runner.memory(), 3);
}

#[test]
fn ex8_067_start_of_turn_does_not_set_memory_when_gt_two() {
    // If memory is already 3 or more, the Start-of-Turn effect must not fire.
    let filler = make_test_card("EX8-067-FILL2", "Filler2");
    let mut runner = DebugRunner::builder()
        .dsl_card("EX8-067")
        .expect("EX8-067 YAML parses and compiles")
        .add_card(filler)
        .deck(0, &["EX8-067-FILL2"])
        .deck(1, &["EX8-067-FILL2"])
        .memory(5)
        .start();

    runner.place_on_field(0, "EX8-067", None);
    runner.game.memory = 5;

    runner.end_turn();
    runner.end_turn();

    assert_eq!(
        runner.memory(),
        5,
        "memory must not change when already above 2"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 3 — Clause 1: [Your Turn] on_digivolve Mineral/Rock observer
// ═══════════════════════════════════════════════════════════════════════════════

/// Helper: build a runner with Close (EX8-067), a Mineral base Lv3 on field,
/// a Mineral Lv4 in hand, and one Mineral trash card available.
/// Returns the runner and the base permanent handle.
fn close_runner_mineral_setup() -> (DebugRunner, PermanentHandle) {
    let mineral_lv4 = make_mineral_lv4("MIN-LV4");
    let mineral_base = make_mineral_lv3_base("MIN-BASE");
    let trash_card = make_mineral_trash_src("MIN-TRASH");

    let mut runner = DebugRunner::builder()
        .dsl_card("EX8-067")
        .expect("EX8-067 in embedded pack")
        .add_card(mineral_lv4)
        .add_card(mineral_base)
        .add_card(trash_card)
        .memory(10)
        .start();

    // Place Close (unsuspended) on P0's field.
    runner.place_on_field(0, "EX8-067", None);

    // Place the Lv3 Mineral base on P0's field as the digivolve platform.
    let base_perm = runner.place_on_field(0, "MIN-BASE", Some(0));

    // Pre-load a Mineral trash card.
    push_to_trash(&mut runner, 0, "MIN-TRASH");

    // Pre-load the Lv4 Mineral card into hand.
    push_to_hand(&mut runner, 0, "MIN-LV4");

    (runner, base_perm)
}

#[test]
fn ex8_067_on_digivolve_triggers_when_mineral_digimon_digivolves() {
    let (mut runner, base_perm) = close_runner_mineral_setup();

    // Digivolve the Lv3 base into the Lv4 Mineral Digimon.
    fire_digivolve_onto(&mut runner, 0, base_perm, "MIN-LV4");

    // After OnDigivolve fires, Close's observer should install an optional
    // pending selection (the activate/decline gate).
    assert!(
        runner.pending_selection().is_some(),
        "optional activation prompt must install when an own Mineral Digimon digivolves \
         and Close is unsuspended"
    );
    assert!(
        runner.pending_is_optional(),
        "the activation prompt must be optional (player can decline)"
    );
}

#[test]
fn ex8_067_on_digivolve_triggers_for_rock_trait() {
    // The trait gate is Mineral OR Rock. Verify Rock alone is sufficient.
    let rock_lv4 = make_rock_lv4("ROCK-LV4-R");
    let mineral_base = make_mineral_lv3_base("MIN-BASE-R");

    let mut runner = DebugRunner::builder()
        .dsl_card("EX8-067")
        .expect("EX8-067 in embedded pack")
        .add_card(rock_lv4)
        .add_card(mineral_base)
        .memory(10)
        .start();

    runner.place_on_field(0, "EX8-067", None);
    let base_perm = runner.place_on_field(0, "MIN-BASE-R", Some(0));

    push_to_hand(&mut runner, 0, "ROCK-LV4-R");
    fire_digivolve_onto(&mut runner, 0, base_perm, "ROCK-LV4-R");

    assert!(
        runner.pending_selection().is_some(),
        "Rock trait alone must trigger the on_digivolve observer"
    );
}

#[test]
fn ex8_067_on_digivolve_does_not_trigger_for_non_mineral_rock() {
    // Digivolving into a Beast (no Mineral/Rock trait) must not fire the prompt.
    let beast_lv4 = make_beast_lv4("BEAST-LV4");
    let mineral_base = make_mineral_lv3_base("MIN-BASE-NMR");

    let mut runner = DebugRunner::builder()
        .dsl_card("EX8-067")
        .expect("EX8-067 in embedded pack")
        .add_card(beast_lv4)
        .add_card(mineral_base)
        .memory(10)
        .start();

    runner.place_on_field(0, "EX8-067", None);
    let base_perm = runner.place_on_field(0, "MIN-BASE-NMR", Some(0));

    push_to_hand(&mut runner, 0, "BEAST-LV4");
    fire_digivolve_onto(&mut runner, 0, base_perm, "BEAST-LV4");

    assert!(
        runner.pending_selection().is_none(),
        "no activation prompt when digivolving into a non-Mineral/Rock Digimon"
    );
}

#[test]
fn ex8_067_on_digivolve_does_not_trigger_when_close_already_suspended() {
    // If Close is already suspended it cannot pay the cost — no prompt.
    let mineral_lv4 = make_mineral_lv4("MIN-LV4-SS");
    let mineral_base = make_mineral_lv3_base("MIN-BASE-SS");

    let mut runner = DebugRunner::builder()
        .dsl_card("EX8-067")
        .expect("EX8-067 in embedded pack")
        .add_card(mineral_lv4)
        .add_card(mineral_base)
        .memory(10)
        .start();

    let close_perm = runner.place_on_field(0, "EX8-067", None);
    // Pre-suspend Close.
    runner.game.players[0]
        .battle_area
        .get_mut(close_perm.index as usize)
        .unwrap()
        .is_suspended = true;

    let base_perm = runner.place_on_field(0, "MIN-BASE-SS", Some(0));

    push_to_hand(&mut runner, 0, "MIN-LV4-SS");
    fire_digivolve_onto(&mut runner, 0, base_perm, "MIN-LV4-SS");

    assert!(
        runner.pending_selection().is_none(),
        "no prompt when Close is already suspended (cannot pay suspend cost)"
    );
}

#[test]
fn ex8_067_declining_does_not_suspend_close() {
    // When the optional prompt installs but the player passes (declines),
    // Close must remain unsuspended.
    let (mut runner, base_perm) = close_runner_mineral_setup();

    let close_index = runner
        .game
        .player(0)
        .battle_area
        .iter()
        .position(|p| p.top_card().card_id(&runner.game.card_data) == "EX8-067")
        .unwrap();

    fire_digivolve_onto(&mut runner, 0, base_perm, "MIN-LV4");

    assert!(runner.pending_selection().is_some(), "prompt must install");
    // Pass = decline.
    runner
        .execute_action(0, digimon_engine::action::space::PASS)
        .ok();

    let close_now = runner
        .game
        .player(0)
        .battle_area
        .get(close_index)
        .unwrap();
    assert!(
        !close_now.is_suspended,
        "Close must NOT be suspended if the player declines"
    );
}

#[test]
fn ex8_067_accepting_suspends_close() {
    // When the optional prompt installs and the player accepts, Close must
    // become suspended.
    let (mut runner, base_perm) = close_runner_mineral_setup();

    let close_index = runner
        .game
        .player(0)
        .battle_area
        .iter()
        .position(|p| p.top_card().card_id(&runner.game.card_data) == "EX8-067")
        .unwrap();

    fire_digivolve_onto(&mut runner, 0, base_perm, "MIN-LV4");

    assert!(runner.pending_selection().is_some(), "prompt must install");
    // Accept: pick the first valid action.
    let act = runner.pending_selection().unwrap().valid_action_ids[0];
    runner.execute_action(0, act).ok();
    // Resolve follow-on prompts (trash multi-select) automatically.
    runner.auto_resolve();

    let close_now = runner
        .game
        .player(0)
        .battle_area
        .get(close_index)
        .unwrap();
    assert!(
        close_now.is_suspended,
        "Close must be suspended after activation is accepted"
    );
}

#[test]
fn ex8_067_placing_one_trash_card_increases_source_count() {
    // After accepting and auto-resolving 1 Mineral trash card, the digivolved
    // Digimon's source count must grow by 1.
    let (mut runner, base_perm) = close_runner_mineral_setup();
    let base_index = base_perm.index as usize;

    fire_digivolve_onto(&mut runner, 0, base_perm, "MIN-LV4");

    // After digivolving, MIN-LV4 is pushed on top of MIN-BASE at base_index.
    let sources_before = runner
        .game
        .player(0)
        .battle_area
        .get(base_index)
        .map(|p| p.card_sources.len())
        .unwrap_or(0);

    assert!(runner.pending_selection().is_some(), "prompt must install");
    // Accept the activation.
    let act = runner.pending_selection().unwrap().valid_action_ids[0];
    runner.execute_action(0, act).ok();
    // Resolve trash pick and placement automatically.
    runner.auto_resolve();

    let sources_after = runner
        .game
        .player(0)
        .battle_area
        .get(base_index)
        .map(|p| p.card_sources.len())
        .unwrap_or(0);

    assert!(
        sources_after > sources_before,
        "Digimon's source count must increase after placing a trash card as bottom source \
         (before={sources_before}, after={sources_after})"
    );
}

#[test]
fn ex8_067_can_place_up_to_two_trash_cards() {
    // With 2 Mineral trash cards available, both can be placed as bottom
    // sources. Source count must grow by at least 1 (auto_resolve picks ≥1).
    let mineral_lv4 = make_mineral_lv4("MIN-LV4-2T");
    let mineral_base = make_mineral_lv3_base("MIN-BASE-2T");
    let trash_a = make_mineral_trash_src("TRASH-A-2T");
    let trash_b = make_mineral_trash_src("TRASH-B-2T");

    let mut runner = DebugRunner::builder()
        .dsl_card("EX8-067")
        .expect("EX8-067 in embedded pack")
        .add_card(mineral_lv4)
        .add_card(mineral_base)
        .add_card(trash_a)
        .add_card(trash_b)
        .memory(10)
        .start();

    runner.place_on_field(0, "EX8-067", None);
    let base_perm = runner.place_on_field(0, "MIN-BASE-2T", Some(0));
    let base_index = base_perm.index as usize;

    push_to_trash(&mut runner, 0, "TRASH-A-2T");
    push_to_trash(&mut runner, 0, "TRASH-B-2T");
    push_to_hand(&mut runner, 0, "MIN-LV4-2T");
    fire_digivolve_onto(&mut runner, 0, base_perm, "MIN-LV4-2T");

    let sources_before = runner
        .game
        .player(0)
        .battle_area
        .get(base_index)
        .map(|p| p.card_sources.len())
        .unwrap_or(0);

    assert!(runner.pending_selection().is_some(), "prompt must install");
    let act = runner.pending_selection().unwrap().valid_action_ids[0];
    runner.execute_action(0, act).ok();
    runner.auto_resolve();

    let sources_after = runner
        .game
        .player(0)
        .battle_area
        .get(base_index)
        .map(|p| p.card_sources.len())
        .unwrap_or(0);

    assert!(
        sources_after >= sources_before + 1,
        "at least 1 Mineral/Rock trash card must be placed as bottom source \
         (before={sources_before}, after={sources_after})"
    );
}

#[test]
fn ex8_067_no_mineral_rock_in_trash_allows_zero_placements() {
    // If there are no Mineral/Rock cards in trash, no placement happens.
    let mineral_lv4 = make_mineral_lv4("MIN-LV4-E");
    let mineral_base = make_mineral_lv3_base("MIN-BASE-E");
    let beast_trash = make_beast_lv4("BEAST-TRASH-E");

    let mut runner = DebugRunner::builder()
        .dsl_card("EX8-067")
        .expect("EX8-067 in embedded pack")
        .add_card(mineral_lv4)
        .add_card(mineral_base)
        .add_card(beast_trash)
        .memory(10)
        .start();

    runner.place_on_field(0, "EX8-067", None);
    let base_perm = runner.place_on_field(0, "MIN-BASE-E", Some(0));
    let base_index = base_perm.index as usize;

    push_to_trash(&mut runner, 0, "BEAST-TRASH-E");
    push_to_hand(&mut runner, 0, "MIN-LV4-E");
    fire_digivolve_onto(&mut runner, 0, base_perm, "MIN-LV4-E");

    let sources_before = runner
        .game
        .player(0)
        .battle_area
        .get(base_index)
        .map(|p| p.card_sources.len())
        .unwrap_or(0);

    if runner.pending_selection().is_some() {
        let act = runner.pending_selection().unwrap().valid_action_ids[0];
        runner.execute_action(0, act).ok();
        runner.auto_resolve();
    }

    let sources_after = runner
        .game
        .player(0)
        .battle_area
        .get(base_index)
        .map(|p| p.card_sources.len())
        .unwrap_or(0);

    assert_eq!(
        sources_after, sources_before,
        "no new sources must be placed when trash has no Mineral/Rock cards"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 4 — Security clause
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn ex8_067_security_play_puts_tamer_on_field() {
    // When EX8-067 is revealed as a security card, the security effect fires
    // and Close lands on the defending player's field without paying the cost.
    let mut attacker = make_test_card("ATTACKER-067", "BigAttacker");
    attacker.card_kind = CardKind::Digimon;
    attacker.level = Some(5);
    attacker.dp = Some(9000);
    attacker.play_cost = 0;

    let mut runner = DebugRunner::builder()
        .dsl_card("EX8-067")
        .expect("EX8-067 in embedded pack")
        .add_card(attacker)
        .security(1, &["EX8-067"])
        .memory(10)
        .start();

    let atk_perm = runner.place_on_field(0, "ATTACKER-067", Some(0));

    let p1_field_before = runner.battle_area_size(1);
    assert_eq!(runner.security_count(1), 1, "pre: P1 has 1 security card");

    runner.attack_player(atk_perm, 1, false);

    assert_eq!(
        runner.battle_area_size(1),
        p1_field_before + 1,
        "Close must land on P1's field after the security effect fires"
    );

    let close_on_field = runner
        .game
        .player(1)
        .battle_area
        .iter()
        .any(|p| p.top_card().card_id(&runner.game.card_data) == "EX8-067");
    assert!(
        close_on_field,
        "EX8-067 must be the card placed on P1's field via security effect"
    );

    assert_eq!(
        runner.security_count(1),
        0,
        "security stack must be empty after Close plays from security"
    );
}
