//! EX10-032 Proganomon — Digimon, Black, Level 5, Cost 7, 7000 DP.
//! Traits: [Mineral, LIBERATOR]. Attribute: Virus.
//!
//! # Card text (cards.json — verbatim)
//!
//! [Hand] [Main] If you have [Close], by placing 1 [Landramon] from your
//!   trash as any of your [Sunarizamon]'s bottom digivolution card, it
//!   digivolves into this card for a digivolution cost of 3, ignoring
//!   digivolution requirements.
//!
//! [On Play] [When Digivolving] [When Attacking] By trashing any 1 [Mineral]
//!   or [Rock] trait card from your Digimon's digivolution cards, 1 of your
//!   such Digimon gains ＜Collision＞, ＜Piercing＞ and +3000 DP until your
//!   opponent's turn ends.
//!
//! Inherited: When effects trash this card from a [Mineral] or [Rock] trait
//!   Digimon's digivolution cards, ＜De-Digivolve 1＞ 1 of your opponent's
//!   Digimon.
//!
//! # Patterns covered
//! - [Hand][Main] alt-digivolve via Close-on-field gate + place Landramon from
//!   trash under Sunarizamon → effect_initiated_digivolve at cost 3
//! - [OP][WD][WA] select_own_sources (Mineral/Rock filter) + trash_selected_sources
//!   + select_own_permanent (Mineral/Rock Digimon) + Collision/Piercing/+3000DP
//!   with end_of_opponents_turn expiry
//! - Multi-timing: all three timings (on_play, when_digivolving, when_attacking)
//!   fire Clause 2
//! - Expiry: buffs persist through own turn end, expire after opponent's turn ends
//! - Inherited source-trash De-Digivolve 1 (retained from prior pass)

use digimon_dsl::compiled::{CompiledClause, CompiledScope, CompiledTiming};
use digimon_engine::action::build_action_mask;
use digimon_engine::action::space::HAND_EFFECT_START;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::effect_context::EffectContext;
use digimon_engine::enums::{CardKind, EffectTiming, Keyword};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::{SelectionKind, TriggerSource};

// ─── Card data helpers ────────────────────────────────────────────────────────

/// A Mineral-trait Lv4 Digimon usable as a digivolution source.
fn make_mineral_source(id: &str) -> digimon_engine::card_data::CardData {
    let mut c = make_test_card(id, &format!("{id} Mineral"));
    c.traits.push("Mineral".to_string());
    c.level = Some(4);
    c.dp = Some(5000);
    c
}

/// A Rock-trait Lv4 Digimon usable as a digivolution source.
fn make_rock_source(id: &str) -> digimon_engine::card_data::CardData {
    let mut c = make_test_card(id, &format!("{id} Rock"));
    c.traits.push("Rock".to_string());
    c.level = Some(4);
    c.dp = Some(5000);
    c
}

/// A plain Lv3 source card with no special traits.
fn make_plain_source(id: &str) -> digimon_engine::card_data::CardData {
    let mut c = make_test_card(id, &format!("{id} Plain"));
    c.level = Some(3);
    c.dp = Some(2000);
    c
}

/// A Sunarizamon Digimon on field (used by Hand/Main clause).
fn make_sunarizamon(id: &str) -> digimon_engine::card_data::CardData {
    let mut c = make_test_card(id, "Sunarizamon");
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(4000);
    c
}

/// A Close Tamer (EX11-065 name check).
fn make_close_tamer(id: &str) -> digimon_engine::card_data::CardData {
    let mut c = make_test_card(id, "Close");
    c.card_kind = CardKind::Tamer;
    c
}

/// A Landramon card (placed into trash for Hand/Main clause).
fn make_landramon(id: &str) -> digimon_engine::card_data::CardData {
    let mut c = make_test_card(id, "Landramon");
    c.card_kind = CardKind::Digimon;
    c.level = Some(3);
    c.dp = Some(2000);
    c
}

/// Push a registered card_id into player `player`'s trash.
fn push_to_trash(runner: &mut DebugRunner, player: u8, card_id: &str) {
    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .unwrap_or_else(|| panic!("{card_id} must be registered"));
    let idx = runner.game.next_card_index();
    runner.game.players[player as usize]
        .trash
        .push(CardSource::new(data_idx, player, idx));
}

/// Build the minimal runner for Clause 2 buff tests (one Mineral Digimon with
/// one Mineral source — ready for select_own_sources + select_own_permanent).
fn buff_runner_with_mineral_host() -> (DebugRunner, PermanentHandle) {
    let mineral_host = make_mineral_source("MINERAL-HOST");
    let mineral_src = make_mineral_source("MINERAL-SRC");

    let mut runner = DebugRunner::builder()
        .dsl_card("EX10-032")
        .expect("EX10-032 YAML parses and compiles")
        .add_card(mineral_host)
        .add_card(mineral_src)
        .memory(10)
        .build();

    let host = runner.place_on_field(0, "MINERAL-HOST", None);
    runner.push_source(host, "MINERAL-SRC");
    (runner, host)
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 1 — Structural assertions
// ═══════════════════════════════════════════════════════════════════════════════

/// EX10-032 YAML must compile and be present in the embedded DSL pack.
#[test]
fn ex10_032_yaml_compiles_without_error() {
    let runner = DebugRunner::builder()
        .dsl_card("EX10-032")
        .expect("EX10-032 YAML parses and compiles")
        .build();
    assert!(
        runner.compiled_card("EX10-032").is_some(),
        "EX10-032 must be present in the embedded DSL pack"
    );
}

/// EX10-032 must have exactly three triggered clauses:
///   - Clause 1: main_from_hand
///   - Clause 2: on_play + when_digivolving + when_attacking
///   - Clause 3: on_digivolution_card_trashed (inherited)
#[test]
fn ex10_032_has_three_triggered_clauses() {
    let runner = DebugRunner::builder()
        .dsl_card("EX10-032")
        .expect("EX10-032 YAML parses and compiles")
        .build();
    let card = runner
        .compiled_card("EX10-032")
        .expect("EX10-032 compiled card present");

    let triggered_count = card
        .effects
        .iter()
        .filter(|c| matches!(c, CompiledClause::Triggered(_)))
        .count();

    assert_eq!(
        triggered_count, 3,
        "EX10-032 must have exactly 3 triggered clauses (hand_main + buff + inherited)"
    );
}

/// Clause 1 fires on MainFromHand timing.
#[test]
fn ex10_032_has_main_from_hand_clause() {
    let runner = DebugRunner::builder()
        .dsl_card("EX10-032")
        .expect("EX10-032 YAML parses and compiles")
        .build();
    let card = runner
        .compiled_card("EX10-032")
        .expect("EX10-032 compiled card present");

    assert!(
        card.effects.iter().any(|c| matches!(
            c,
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::MainFromHand)
        )),
        "EX10-032 must have a MainFromHand clause"
    );
}

/// Clause 2 fires on OnPlay, WhenDigivolving, and WhenAttacking.
#[test]
fn ex10_032_has_on_play_when_digivolving_when_attacking_clause() {
    let runner = DebugRunner::builder()
        .dsl_card("EX10-032")
        .expect("EX10-032 YAML parses and compiles")
        .build();
    let card = runner
        .compiled_card("EX10-032")
        .expect("EX10-032 compiled card present");

    let has_clause_2 = card.effects.iter().any(|c| {
        matches!(
            c,
            CompiledClause::Triggered(t)
                if t.when.contains(&CompiledTiming::OnPlay)
                    && t.when.contains(&CompiledTiming::WhenDigivolving)
                    && t.when.contains(&CompiledTiming::WhenAttacking)
        )
    });
    assert!(
        has_clause_2,
        "EX10-032 must have a clause with OnPlay + WhenDigivolving + WhenAttacking timings"
    );
}

/// Clause 2 is own scope (FaceUp).
#[test]
fn ex10_032_buff_clause_is_own_scope() {
    let runner = DebugRunner::builder()
        .dsl_card("EX10-032")
        .expect("EX10-032 YAML parses and compiles")
        .build();
    let card = runner
        .compiled_card("EX10-032")
        .expect("EX10-032 compiled card present");

    let clause = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| t.when.contains(&CompiledTiming::OnPlay))
        .expect("buff clause must exist");

    assert_eq!(
        clause.scope,
        CompiledScope::FaceUp,
        "EX10-032 buff clause must be FaceUp (own) scope"
    );
}

/// Clause 3 is inherited (OnDigivolutionCardTrashed).
#[test]
fn ex10_032_has_inherited_source_trash_dedigivolve_clause() {
    let runner = DebugRunner::builder()
        .dsl_card("EX10-032")
        .expect("EX10-032 YAML parses and compiles")
        .build();
    let card = runner
        .compiled_card("EX10-032")
        .expect("EX10-032 compiled card present");

    assert!(card.effects.iter().any(|clause| matches!(
        clause,
        CompiledClause::Triggered(triggered)
            if triggered.scope == CompiledScope::Inherited
                && triggered.when == vec![CompiledTiming::OnDigivolutionCardTrashed]
    )));
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 2 — [Hand][Main] alt-digivolve via Close + Landramon
// ═══════════════════════════════════════════════════════════════════════════════

/// [Hand][Main] is masked legal when Close Tamer, Sunarizamon, and a Landramon
/// in trash are all present.
#[test]
fn ex10_032_hand_main_is_legal_when_all_conditions_met() {
    let sunarizamon = make_sunarizamon("SUNA-1");
    let close = make_close_tamer("CLOSE-1");
    let landramon = make_landramon("LANDR-1");

    let mut runner = DebugRunner::builder()
        .dsl_card("EX10-032")
        .expect("EX10-032 YAML parses and compiles")
        .add_card(sunarizamon)
        .add_card(close)
        .add_card(landramon)
        .hand(0, &["EX10-032"])
        .memory(10)
        .start();

    runner.place_on_field(0, "SUNA-1", None);
    runner.place_on_field(0, "CLOSE-1", None);
    push_to_trash(&mut runner, 0, "LANDR-1");

    let mask = build_action_mask(&runner.game, 0);
    assert_eq!(
        mask[HAND_EFFECT_START as usize], 1.0,
        "[Hand][Main] must be legal when all conditions are met (Close + Sunarizamon + Landramon in trash)"
    );
}

/// [Hand][Main] is masked illegal when no Close Tamer is on field.
#[test]
fn ex10_032_hand_main_masked_without_close() {
    let sunarizamon = make_sunarizamon("SUNA-NC");
    let landramon = make_landramon("LANDR-NC");

    let mut runner = DebugRunner::builder()
        .dsl_card("EX10-032")
        .expect("EX10-032 YAML parses and compiles")
        .add_card(sunarizamon)
        .add_card(landramon)
        .hand(0, &["EX10-032"])
        .memory(10)
        .start();

    runner.place_on_field(0, "SUNA-NC", None);
    push_to_trash(&mut runner, 0, "LANDR-NC");

    let mask = build_action_mask(&runner.game, 0);
    assert_eq!(
        mask[HAND_EFFECT_START as usize], 0.0,
        "[Hand][Main] must be masked without a Close Tamer on field"
    );
}

/// [Hand][Main] is masked illegal when no Sunarizamon is on field.
#[test]
fn ex10_032_hand_main_masked_without_sunarizamon() {
    let close = make_close_tamer("CLOSE-NS");
    let landramon = make_landramon("LANDR-NS");

    let mut runner = DebugRunner::builder()
        .dsl_card("EX10-032")
        .expect("EX10-032 YAML parses and compiles")
        .add_card(close)
        .add_card(landramon)
        .hand(0, &["EX10-032"])
        .memory(10)
        .start();

    runner.place_on_field(0, "CLOSE-NS", None);
    push_to_trash(&mut runner, 0, "LANDR-NS");

    let mask = build_action_mask(&runner.game, 0);
    assert_eq!(
        mask[HAND_EFFECT_START as usize], 0.0,
        "[Hand][Main] must be masked without a Sunarizamon on field"
    );
}

/// [Hand][Main] is masked illegal when no Landramon is in trash.
#[test]
fn ex10_032_hand_main_masked_without_landramon_in_trash() {
    let sunarizamon = make_sunarizamon("SUNA-NL");
    let close = make_close_tamer("CLOSE-NL");

    let mut runner = DebugRunner::builder()
        .dsl_card("EX10-032")
        .expect("EX10-032 YAML parses and compiles")
        .add_card(sunarizamon)
        .add_card(close)
        .hand(0, &["EX10-032"])
        .memory(10)
        .start();

    runner.place_on_field(0, "SUNA-NL", None);
    runner.place_on_field(0, "CLOSE-NL", None);

    let mask = build_action_mask(&runner.game, 0);
    assert_eq!(
        mask[HAND_EFFECT_START as usize], 0.0,
        "[Hand][Main] must be masked without a Landramon in trash"
    );
}

/// [Hand][Main] execution: places Landramon as Sunarizamon's bottom source
/// and digivolves into EX10-032 at cost 3.
#[test]
fn ex10_032_hand_main_places_landramon_and_digivolves() {
    let sunarizamon = make_sunarizamon("SUNA-EXEC");
    let close = make_close_tamer("CLOSE-EXEC");
    let landramon = make_landramon("LANDR-EXEC");

    let mut runner = DebugRunner::builder()
        .dsl_card("EX10-032")
        .expect("EX10-032 YAML parses and compiles")
        .add_card(sunarizamon)
        .add_card(close)
        .add_card(landramon)
        .hand(0, &["EX10-032"])
        .memory(10)
        .start();

    let suna = runner.place_on_field(0, "SUNA-EXEC", None);
    runner.place_on_field(0, "CLOSE-EXEC", None);
    push_to_trash(&mut runner, 0, "LANDR-EXEC");

    // Activate [Hand][Main].
    runner.game.decode_action(HAND_EFFECT_START, 0);

    // Prompt 1: choose Landramon from trash.
    let trash_view = runner
        .pending_selection_view()
        .expect("Landramon trash selection must appear");
    runner
        .execute_action(0, trash_view.valid_action_ids[0])
        .expect("choose Landramon");

    // Prompt 2: choose Sunarizamon on field.
    let field_view = runner
        .pending_selection_view()
        .expect("Sunarizamon field selection must appear");
    runner
        .execute_action(0, field_view.valid_action_ids[0])
        .expect("choose Sunarizamon");

    // Resolve remaining prompts (effect_initiated_digivolve may prompt for
    // WhenDigivolving clause 2 — resolve all).
    runner.auto_resolve().ok();

    // Assert: Landramon is now the bottom source of the stack.
    let stack = &runner.game.players[0].battle_area[suna.index as usize].card_sources;
    assert!(
        stack.len() >= 2,
        "stack must have at least 2 cards after digivolve (sources + top)"
    );
    assert_eq!(
        stack[0].card_id(&runner.game.card_data),
        "LANDR-EXEC",
        "Landramon must be placed as the bottom source"
    );

    // Assert: EX10-032 is now the top card.
    let top_id = runner.game.players[0].battle_area[suna.index as usize]
        .top_card()
        .card_id(&runner.game.card_data);
    assert_eq!(
        top_id, "EX10-032",
        "EX10-032 must be the top card after effect_initiated_digivolve"
    );

    // Assert: memory decreased by 3 (started at 10, net = 10 − 3 = 7).
    assert_eq!(
        runner.memory(),
        7,
        "effect digivolve at cost 3 must reduce memory by 3"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 3 — [OP][WD][WA] source-trash buff: condition gating
// ═══════════════════════════════════════════════════════════════════════════════

/// Positive: firing on_play with a Mineral source on field prompts the player
/// to select a source card (SourceMulti selection).
#[test]
fn ex10_032_on_play_prompts_source_selection_with_mineral_source() {
    let (mut runner, _host) = buff_runner_with_mineral_host();
    let proganomon = runner.place_on_field(0, "EX10-032", None);

    runner
        .game
        .enqueue_triggered(EffectTiming::OnPlay, TriggerSource::Permanent(proganomon));
    runner.game.drain_effect_queue();

    let kind = runner
        .pending_kind()
        .expect("on_play must install a pending selection");
    assert!(
        matches!(kind, SelectionKind::SourceMulti { .. }),
        "on_play buff must prompt for a SourceMulti (source-trash cost), got: {kind:?}"
    );
}

/// Positive: firing on_play with a Rock source on field also prompts source selection.
#[test]
fn ex10_032_on_play_prompts_source_selection_with_rock_source() {
    let rock_host = make_rock_source("ROCK-HOST-OP");
    let rock_src = make_rock_source("ROCK-SRC-OP");

    let mut runner = DebugRunner::builder()
        .dsl_card("EX10-032")
        .expect("EX10-032 YAML parses and compiles")
        .add_card(rock_host)
        .add_card(rock_src)
        .memory(10)
        .build();

    let host = runner.place_on_field(0, "ROCK-HOST-OP", None);
    runner.push_source(host, "ROCK-SRC-OP");
    let proganomon = runner.place_on_field(0, "EX10-032", None);

    runner
        .game
        .enqueue_triggered(EffectTiming::OnPlay, TriggerSource::Permanent(proganomon));
    runner.game.drain_effect_queue();

    let kind = runner
        .pending_kind()
        .expect("on_play with Rock source must install pending selection");
    assert!(
        matches!(kind, SelectionKind::SourceMulti { .. }),
        "on_play buff must prompt for SourceMulti with Rock source, got: {kind:?}"
    );
}

/// Negative: firing on_play when no Mineral/Rock source is available installs
/// no selection (silent skip).
#[test]
fn ex10_032_on_play_no_selection_when_no_mineral_rock_source() {
    let plain_host = make_test_card("PLAIN-H-OP", "Plain Host");
    let plain_src = make_plain_source("PLAIN-SRC-OP");

    let mut runner = DebugRunner::builder()
        .dsl_card("EX10-032")
        .expect("EX10-032 YAML parses and compiles")
        .add_card(plain_host)
        .add_card(plain_src)
        .memory(10)
        .build();

    let host = runner.place_on_field(0, "PLAIN-H-OP", None);
    runner.push_source(host, "PLAIN-SRC-OP");
    let proganomon = runner.place_on_field(0, "EX10-032", None);

    runner
        .game
        .enqueue_triggered(EffectTiming::OnPlay, TriggerSource::Permanent(proganomon));
    runner.game.drain_effect_queue();

    assert!(
        runner.pending_selection().is_none(),
        "no selection must install when no Mineral/Rock source exists"
    );
}

/// Negative: firing on_play when there are no digivolution sources at all
/// installs no selection.
#[test]
fn ex10_032_on_play_no_selection_when_no_sources_at_all() {
    let mineral_host = make_mineral_source("MIN-ALONE");
    // Place the host with no pushed source (single-card stack).

    let mut runner = DebugRunner::builder()
        .dsl_card("EX10-032")
        .expect("EX10-032 YAML parses and compiles")
        .add_card(mineral_host)
        .memory(10)
        .build();

    // Even though the top card has Mineral trait, it has no digivolution sources.
    runner.place_on_field(0, "MIN-ALONE", None);
    let proganomon = runner.place_on_field(0, "EX10-032", None);

    runner
        .game
        .enqueue_triggered(EffectTiming::OnPlay, TriggerSource::Permanent(proganomon));
    runner.game.drain_effect_queue();

    assert!(
        runner.pending_selection().is_none(),
        "no selection must install when Mineral host has no digivolution sources to trash"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 4 — [OP][WD][WA] buff: keyword and DP grants
// ═══════════════════════════════════════════════════════════════════════════════

/// After trashing a Mineral source via on_play, the chosen Mineral/Rock Digimon
/// gains Collision.
#[test]
fn ex10_032_on_play_grants_collision() {
    let (mut runner, host) = buff_runner_with_mineral_host();
    let proganomon = runner.place_on_field(0, "EX10-032", None);

    runner
        .game
        .enqueue_triggered(EffectTiming::OnPlay, TriggerSource::Permanent(proganomon));
    runner.game.drain_effect_queue();

    // Source selection — pick the first valid action.
    assert!(matches!(
        runner.pending_kind(),
        Some(SelectionKind::SourceMulti { .. })
    ));
    runner.auto_resolve().expect("source selection resolves");

    // Field selection for buff target (Mineral Digimon).
    if runner.pending_selection().is_some() {
        runner.auto_resolve().expect("field selection resolves");
    }

    assert!(
        runner.game.has_keyword(host, Keyword::Collision),
        "EX10-032 on_play must grant Collision to the selected Mineral/Rock Digimon"
    );
}

/// After trashing, the target gains Piercing.
#[test]
fn ex10_032_on_play_grants_piercing() {
    let (mut runner, host) = buff_runner_with_mineral_host();
    let proganomon = runner.place_on_field(0, "EX10-032", None);

    runner
        .game
        .enqueue_triggered(EffectTiming::OnPlay, TriggerSource::Permanent(proganomon));
    runner.game.drain_effect_queue();

    assert!(matches!(
        runner.pending_kind(),
        Some(SelectionKind::SourceMulti { .. })
    ));
    runner.auto_resolve().expect("source selection resolves");
    if runner.pending_selection().is_some() {
        runner.auto_resolve().expect("field selection resolves");
    }

    assert!(
        runner.game.has_keyword(host, Keyword::Piercing),
        "EX10-032 on_play must grant Piercing to the selected Mineral/Rock Digimon"
    );
}

/// After trashing, the target gains +3000 DP.
#[test]
fn ex10_032_on_play_grants_plus_3000_dp() {
    let (mut runner, host) = buff_runner_with_mineral_host();
    let dp_before = runner.dp_of(host).unwrap_or(0);

    let proganomon = runner.place_on_field(0, "EX10-032", None);

    runner
        .game
        .enqueue_triggered(EffectTiming::OnPlay, TriggerSource::Permanent(proganomon));
    runner.game.drain_effect_queue();

    assert!(matches!(
        runner.pending_kind(),
        Some(SelectionKind::SourceMulti { .. })
    ));
    runner.auto_resolve().expect("source selection resolves");
    if runner.pending_selection().is_some() {
        runner.auto_resolve().expect("field selection resolves");
    }

    let dp_after = runner.dp_of(host).unwrap_or(0);
    assert_eq!(
        dp_after - dp_before,
        3000,
        "EX10-032 on_play must grant +3000 DP (before: {dp_before}, after: {dp_after})"
    );
}

/// The trashed source is removed from the Digimon's stack and moved to trash.
#[test]
fn ex10_032_on_play_removes_source_from_stack() {
    let (mut runner, host) = buff_runner_with_mineral_host();

    let sources_before = runner.game.player(0).battle_area[host.index as usize]
        .card_sources
        .len();
    let trash_before = runner.trash_size(0);
    assert!(
        sources_before >= 2,
        "host must have a source before the test"
    );

    let proganomon = runner.place_on_field(0, "EX10-032", None);

    runner
        .game
        .enqueue_triggered(EffectTiming::OnPlay, TriggerSource::Permanent(proganomon));
    runner.game.drain_effect_queue();

    assert!(matches!(
        runner.pending_kind(),
        Some(SelectionKind::SourceMulti { .. })
    ));
    runner.auto_resolve().expect("source selection resolves");
    if runner.pending_selection().is_some() {
        runner.auto_resolve().expect("field selection resolves");
    }

    let sources_after = runner.game.player(0).battle_area[host.index as usize]
        .card_sources
        .len();
    let trash_after = runner.trash_size(0);

    assert_eq!(
        sources_after,
        sources_before - 1,
        "one source must be removed from the Digimon's stack"
    );
    assert!(
        trash_after > trash_before,
        "trashed source must land in trash (before: {trash_before}, after: {trash_after})"
    );
}

/// EX10-032 does NOT grant Reboot (unlike EX8-070). Verify that Reboot is absent.
#[test]
fn ex10_032_does_not_grant_reboot() {
    let (mut runner, host) = buff_runner_with_mineral_host();
    let proganomon = runner.place_on_field(0, "EX10-032", None);

    runner
        .game
        .enqueue_triggered(EffectTiming::OnPlay, TriggerSource::Permanent(proganomon));
    runner.game.drain_effect_queue();

    assert!(matches!(
        runner.pending_kind(),
        Some(SelectionKind::SourceMulti { .. })
    ));
    runner.auto_resolve().expect("source selection resolves");
    if runner.pending_selection().is_some() {
        runner.auto_resolve().expect("field selection resolves");
    }

    assert!(
        !runner.game.has_keyword(host, Keyword::Reboot),
        "EX10-032 must NOT grant Reboot (only Collision + Piercing)"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 5 — Multi-timing: when_digivolving and when_attacking also fire buff
// ═══════════════════════════════════════════════════════════════════════════════

/// Clause 2 fires on WhenDigivolving timing.
#[test]
fn ex10_032_when_digivolving_fires_buff_clause() {
    let mineral_host = make_mineral_source("MIN-WD-H");
    let mineral_src = make_mineral_source("MIN-WD-S");

    let mut runner = DebugRunner::builder()
        .dsl_card("EX10-032")
        .expect("EX10-032 YAML parses and compiles")
        .add_card(mineral_host)
        .add_card(mineral_src)
        .memory(10)
        .build();

    let host = runner.place_on_field(0, "MIN-WD-H", None);
    runner.push_source(host, "MIN-WD-S");
    let proganomon = runner.place_on_field(0, "EX10-032", None);

    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(proganomon),
    );
    runner.game.drain_effect_queue();

    let kind = runner
        .pending_kind()
        .expect("WhenDigivolving must install a pending selection");
    assert!(
        matches!(kind, SelectionKind::SourceMulti { .. }),
        "WhenDigivolving must prompt for SourceMulti (Clause 2 source-trash cost), got: {kind:?}"
    );

    // Resolve fully to verify buff arrives.
    runner.auto_resolve().expect("source selection resolves");
    if runner.pending_selection().is_some() {
        runner.auto_resolve().expect("field selection resolves");
    }

    assert!(
        runner.game.has_keyword(host, Keyword::Collision),
        "WhenDigivolving must grant Collision"
    );
}

/// Clause 2 fires on WhenAttacking timing.
#[test]
fn ex10_032_when_attacking_fires_buff_clause() {
    let mineral_host = make_mineral_source("MIN-WA-H");
    let mineral_src = make_mineral_source("MIN-WA-S");

    let mut runner = DebugRunner::builder()
        .dsl_card("EX10-032")
        .expect("EX10-032 YAML parses and compiles")
        .add_card(mineral_host)
        .add_card(mineral_src)
        .memory(10)
        .build();

    let host = runner.place_on_field(0, "MIN-WA-H", None);
    runner.push_source(host, "MIN-WA-S");
    let proganomon = runner.place_on_field(0, "EX10-032", None);

    runner.game.enqueue_triggered(
        EffectTiming::WhenAttacking,
        TriggerSource::Permanent(proganomon),
    );
    runner.game.drain_effect_queue();

    let kind = runner
        .pending_kind()
        .expect("WhenAttacking must install a pending selection");
    assert!(
        matches!(kind, SelectionKind::SourceMulti { .. }),
        "WhenAttacking must prompt for SourceMulti (Clause 2 source-trash cost), got: {kind:?}"
    );

    runner.auto_resolve().expect("source selection resolves");
    if runner.pending_selection().is_some() {
        runner.auto_resolve().expect("field selection resolves");
    }

    assert!(
        runner.game.has_keyword(host, Keyword::Piercing),
        "WhenAttacking must grant Piercing"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 6 — Expiry: end_of_opponents_turn
// ═══════════════════════════════════════════════════════════════════════════════

/// Buffs (Collision, Piercing, +3000 DP) persist through the end of the
/// controller's own turn — they last until end of OPPONENT's turn.
#[test]
fn ex10_032_buffs_persist_through_own_turn_end() {
    let (mut runner, host) = buff_runner_with_mineral_host();
    let proganomon = runner.place_on_field(0, "EX10-032", None);

    runner
        .game
        .enqueue_triggered(EffectTiming::OnPlay, TriggerSource::Permanent(proganomon));
    runner.game.drain_effect_queue();

    assert!(matches!(
        runner.pending_kind(),
        Some(SelectionKind::SourceMulti { .. })
    ));
    runner.auto_resolve().expect("source selection");
    if runner.pending_selection().is_some() {
        runner.auto_resolve().expect("field selection");
    }

    // Confirm buffs are active immediately.
    assert!(runner.game.has_keyword(host, Keyword::Collision));
    assert!(runner.game.has_keyword(host, Keyword::Piercing));

    // End player 0's turn — buffs must still be active at start of opp's turn.
    runner.end_turn();

    assert!(
        runner.game.has_keyword(host, Keyword::Collision),
        "Collision must persist at start of opponent's turn (expiry is end_of_opponents_turn)"
    );
    assert!(
        runner.game.has_keyword(host, Keyword::Piercing),
        "Piercing must persist at start of opponent's turn"
    );
}

/// After opponent's turn ends, all EX10-032 buffs expire.
#[test]
fn ex10_032_buffs_expire_after_opponents_turn_ends() {
    let mineral_host = make_mineral_source("MIN-EXP");
    let mineral_src = make_mineral_source("MIN-EXP-SRC");
    let p1_filler = make_test_card("EXP-PAD", "Filler");

    let mut runner = DebugRunner::builder()
        .dsl_card("EX10-032")
        .expect("EX10-032 YAML parses and compiles")
        .add_card(mineral_host)
        .add_card(mineral_src)
        .add_card(p1_filler)
        .deck(1, &["EXP-PAD", "EXP-PAD", "EXP-PAD"])
        .memory(10)
        .build();

    let host = runner.place_on_field(0, "MIN-EXP", None);
    runner.push_source(host, "MIN-EXP-SRC");
    let dp_base = runner.dp_of(host).unwrap_or(0);

    let proganomon = runner.place_on_field(0, "EX10-032", None);

    runner
        .game
        .enqueue_triggered(EffectTiming::OnPlay, TriggerSource::Permanent(proganomon));
    runner.game.drain_effect_queue();

    assert!(matches!(
        runner.pending_kind(),
        Some(SelectionKind::SourceMulti { .. })
    ));
    runner.auto_resolve().expect("source selection");
    if runner.pending_selection().is_some() {
        runner.auto_resolve().expect("field selection");
    }

    // End player 0's turn, then player 1's turn (back to player 0).
    runner.end_turn(); // → player 1's turn
    runner.end_turn(); // → player 0's turn; modifiers should be expired

    assert!(
        !runner.game.has_keyword(host, Keyword::Collision),
        "Collision must expire after opponent's turn ends"
    );
    assert!(
        !runner.game.has_keyword(host, Keyword::Piercing),
        "Piercing must expire after opponent's turn ends"
    );
    let dp_after = runner.dp_of(host).unwrap_or(0);
    assert_eq!(
        dp_after, dp_base,
        "DP must return to base after opponent's turn ends (base: {dp_base}, got: {dp_after})"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 7 — Inherited: source-trash De-Digivolve 1
// ═══════════════════════════════════════════════════════════════════════════════

/// When EX10-032 is trashed as a source from a Mineral/Rock Digimon via an
/// effect, De-Digivolve 1 fires — target selection prompt installs.
#[test]
fn ex10_032_source_trash_dedigivolves_one_opponent_digimon() {
    // Use DSL cards for the opponent digimon stack.
    let mut rock_host = make_test_card("ROCK-IHOST", "Rock Host");
    rock_host.traits.push("Rock".to_string());
    rock_host.card_kind = CardKind::Digimon;
    rock_host.level = Some(5);
    rock_host.dp = Some(6000);

    let mut opp_top = make_test_card("OPP-TOP-DD", "Opp Top");
    opp_top.card_kind = CardKind::Digimon;
    opp_top.level = Some(5);
    opp_top.dp = Some(7000);

    let mut opp_src = make_test_card("OPP-SRC-DD", "Opp Src");
    opp_src.card_kind = CardKind::Digimon;
    opp_src.level = Some(4);
    opp_src.dp = Some(5000);

    let mut runner = DebugRunner::builder()
        .dsl_card("EX10-032")
        .expect("EX10-032 YAML parses and compiles")
        .add_card(rock_host)
        .add_card(opp_top)
        .add_card(opp_src)
        .build();

    let host = runner.place_on_field(0, "ROCK-IHOST", None);
    let source = runner.push_source(host, "EX10-032");
    let opponent = runner.place_stack(1, &["OPP-SRC-DD", "OPP-TOP-DD"]);

    let top_card = runner.top_card(host);
    {
        let mut ctx = EffectContext::new(&mut runner.game, top_card, Some(host), 0);
        ctx.trash_card_source(host, source);
    }

    // De-Digivolve target selection should be pending.
    assert_eq!(
        runner.pending_kind(),
        Some(SelectionKind::OppField),
        "inherited source-trash must install OppField selection for De-Digivolve 1 target"
    );

    runner
        .auto_resolve()
        .expect("De-Digivolve target selection resolves");

    // The opponent's stack must have lost its top source (de-digivolved by 1).
    let opp_sources = runner.game.player(1).battle_area[opponent.index as usize]
        .card_sources
        .len();
    assert_eq!(
        opp_sources, 1,
        "opponent's Digimon must lose 1 stack level via De-Digivolve 1 (expected 1 source remaining)"
    );
}

/// No De-Digivolve fires when EX10-032 is trashed from a non-Mineral/non-Rock
/// Digimon's digivolution cards.
#[test]
fn ex10_032_inherited_no_dedigivolve_from_non_mineral_rock_host() {
    let mut plain_host = make_test_card("PLAIN-IHOST", "Plain Host");
    plain_host.card_kind = CardKind::Digimon;
    plain_host.level = Some(5);
    plain_host.dp = Some(6000);
    // No Mineral or Rock trait.

    let mut opp_top = make_test_card("OPP-TOP-NDD", "Opp Top NDD");
    opp_top.card_kind = CardKind::Digimon;
    opp_top.level = Some(5);
    opp_top.dp = Some(7000);

    let mut opp_src = make_test_card("OPP-SRC-NDD", "Opp Src NDD");
    opp_src.card_kind = CardKind::Digimon;
    opp_src.level = Some(4);
    opp_src.dp = Some(5000);

    let mut runner = DebugRunner::builder()
        .dsl_card("EX10-032")
        .expect("EX10-032 YAML parses and compiles")
        .add_card(plain_host)
        .add_card(opp_top)
        .add_card(opp_src)
        .build();

    let host = runner.place_on_field(0, "PLAIN-IHOST", None);
    let source = runner.push_source(host, "EX10-032");
    let _opponent = runner.place_stack(1, &["OPP-SRC-NDD", "OPP-TOP-NDD"]);

    let top_card = runner.top_card(host);
    {
        let mut ctx = EffectContext::new(&mut runner.game, top_card, Some(host), 0);
        ctx.trash_card_source(host, source);
    }

    assert!(
        runner.pending_selection().is_none(),
        "no De-Digivolve should fire when host has no Mineral/Rock trait"
    );
}

/// No De-Digivolve fires when the opponent has no Digimon.
#[test]
fn ex10_032_inherited_no_dedigivolve_when_no_opp_digimon() {
    let mut mineral_host = make_test_card("MIN-IHOST-EMPTY", "Mineral Host Empty");
    mineral_host.traits.push("Mineral".to_string());
    mineral_host.card_kind = CardKind::Digimon;
    mineral_host.level = Some(5);
    mineral_host.dp = Some(6000);

    let mut runner = DebugRunner::builder()
        .dsl_card("EX10-032")
        .expect("EX10-032 YAML parses and compiles")
        .add_card(mineral_host)
        .build();

    let host = runner.place_on_field(0, "MIN-IHOST-EMPTY", None);
    let source = runner.push_source(host, "EX10-032");

    let top_card = runner.top_card(host);
    {
        let mut ctx = EffectContext::new(&mut runner.game, top_card, Some(host), 0);
        ctx.trash_card_source(host, source);
    }

    assert!(
        runner.pending_selection().is_none(),
        "no De-Digivolve prompt when opponent has no Digimon"
    );
}
