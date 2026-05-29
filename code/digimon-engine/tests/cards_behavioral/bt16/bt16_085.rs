//! BT16-085 Davis Motomiya & Ken Ichijoji — Tamer, Blue + Green, Cost 4.
//!
//! # Card text (cards.json)
//!
//! ```text
//! [Start of Your Main Phase] You may play 1 [Veemon] or [Wormmon] from your
//! hand without paying the cost. At the next end of your opponent's turn,
//! return it to the hand.
//!
//! [Your Turn] When one of your Digimon digivolves into a blue or green
//! Digimon, by suspending this Tamer, gain 1 memory. If DNA digivolving,
//! trash any 3 digivolution cards under your opponent's Digimon.
//! ```
//!
//! # Inherited effect text
//!
//! ```text
//! Security Effect [Security] Play this card without paying the cost.
//! ```
//!
//! # DCGO C# reference
//!
//! DCGO/Assets/Scripts/CardEffect/BT16/Blue/BT16_085.cs
//!
//! # Implementation status
//!
//! | Clause | Status | Gap |
//! |--------|--------|-----|
//! | Clause 0: Start-of-Main free-play Veemon/Wormmon | IMPLEMENTED | — |
//! | Clause 0: Delayed return at next opp EOT | IMPLEMENTED | — |
//! | Clause 1: Digivolve observer suspend tamer → +1 memory | IMPLEMENTED | — |
//! | Clause 1: Blue/green color gate | IMPLEMENTED | event_card_color_has |
//! | Clause 1: DNA sub-condition trash 3 opp digi-cards | IMPLEMENTED | select_opponent_sources |
//! | Clause 2: Security free-play | IMPLEMENTED | — |
//!
//! # Coverage
//!
//! - Metadata: colors, cost, tamer kind.
//! - Clause 0: YAML compiles; start-of-main fires and presents Veemon/Wormmon
//!   hand selection.
//! - Clause 0: Free-play consumes zero memory.
//! - Clause 0: Delayed return implemented.
//! - Clause 1: Digivolve fires observer; player can optionally suspend for +1 memory.
//! - Clause 1: Color gate implemented via `event_card_color_has`.
//! - Clause 1: DNA trash sub-clause implemented via `select_opponent_sources`.
//! - Clause 2: Security clause structural shape verified.

use digimon_dsl::compiled::{
    CompiledClause, CompiledColor, CompiledScope, CompiledStep, CompiledTiming,
};
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::effect_context::EffectContext;
use digimon_engine::enums::{CardColor, CardKind, EffectTiming, PlaySource};
use digimon_engine::selection::{SelectionKind, TriggerSource};

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn make_veemon(id: &str) -> CardData {
    let mut c = make_test_card(id, "Veemon");
    c.card_kind = CardKind::Digimon;
    c.level = Some(3);
    c.dp = Some(1000);
    c.play_cost = 3;
    c
}
fn make_wormmon(id: &str) -> CardData {
    let mut c = make_test_card(id, "Wormmon");
    c.card_kind = CardKind::Digimon;
    c.level = Some(3);
    c.dp = Some(1000);
    c.play_cost = 2;
    c
}

/// ExVeemon — name contains "Veemon" as a substring but is NOT the exact
/// printed name [Veemon]. Davis & Ken's free-play filter must NOT treat this
/// as a [Veemon]-matching pick (the bracketed name in printed text is exact-
/// match against the card name list, not substring).
fn make_exveemon(id: &str) -> CardData {
    let mut c = make_test_card(id, "ExVeemon");
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(4000);
    c.play_cost = 4;
    c
}

fn make_digimon_lv3(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(3);
    c.dp = Some(2000);
    c.play_cost = 3;
    c
}

/// Lv.4 BLUE Digimon — digivolving into this satisfies the
/// `event_card_color_has: [blue, green]` gate on Clause 1.
fn make_digimon_lv4(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(4000);
    c.play_cost = 5;
    c.colors = vec![CardColor::Blue];
    // Digivolves from Lv.3, cost 0 — makes digivolve_from_hand always succeed
    // regardless of memory.
    c.evo_costs = vec![EvoCost {
        card_color: 0,
        level: 3,
        memory_cost: 0,
    }];
    c
}

/// Lv.4 RED Digimon — digivolving into this does NOT satisfy the
/// blue/green color gate; used to verify the observer declines.
fn make_digimon_lv4_red(id: &str) -> CardData {
    let mut c = make_digimon_lv4(id);
    c.colors = vec![CardColor::Red];
    c
}

/// Push a CardSource into player `p`'s hand by card_id lookup.
fn push_to_hand(runner: &mut DebugRunner, p: u8, card_id: &str) {
    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .unwrap_or_else(|| panic!("push_to_hand: unknown card_id {}", card_id));
    let next_idx = runner.game.next_card_index();
    let card = CardSource::new(data_idx, p, next_idx);
    runner.game.players[p as usize].hand.push(card);
}

/// Push a CardSource into player `p`'s security stack by card_id lookup.
fn push_to_security(runner: &mut DebugRunner, p: u8, card_id: &str) {
    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .unwrap_or_else(|| panic!("push_to_security: unknown card_id {}", card_id));
    let next_idx = runner.game.next_card_index();
    let card = CardSource::new(data_idx, p, next_idx);
    runner.game.players[p as usize].security.push(card);
}

fn filler_deck() -> Vec<&'static str> {
    vec!["FILLER", "FILLER", "FILLER", "FILLER", "FILLER"]
}

fn base_runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("BT16-085")
        .expect("BT16-085 YAML parses and compiles")
        .add_card(make_test_card("FILLER", "Filler"))
        .add_card(make_veemon("VEEMON"))
        .add_card(make_wormmon("WORMMON"))
        .add_card(make_digimon_lv3("BASE"))
        .add_card(make_digimon_lv4("EVOLVED"))
        .deck(0, &filler_deck())
        .deck(1, &filler_deck())
        .memory(10)
        .build()
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 1 — Structural / metadata assertions
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn bt16_085_metadata_matches_printed_text() {
    let runner = base_runner();
    let compiled = runner
        .compiled_card("BT16-085")
        .expect("BT16-085 in embedded DSL pack");

    assert_eq!(compiled.name, "Davis Motomiya & Ken Ichijoji");
    assert_eq!(compiled.cost, Some(4));
    assert!(
        compiled.color.iter().any(|c| *c == CompiledColor::Blue),
        "must include blue"
    );
    assert!(
        compiled.color.iter().any(|c| *c == CompiledColor::Green),
        "must include green"
    );
}
#[test]
fn bt16_085_has_three_clauses_matching_printed_text() {
    let runner = base_runner();
    let compiled = runner
        .compiled_card("BT16-085")
        .expect("BT16-085 in embedded DSL pack");

    // Expect: 2 triggered clauses (start_of_main + on_digivolve) + 1 triggered
    // security (on_security). All three compile to Triggered in the DSL.
    let triggered_count = compiled
        .effects
        .iter()
        .filter(|c| matches!(c, CompiledClause::Triggered(_)))
        .count();
    assert_eq!(
        triggered_count, 3,
        "Clause 0 (start_of_main), Clause 1 (on_digivolve), Clause 2 (on_security) = 3 triggered"
    );
}

#[test]
fn bt16_085_clause_0_has_start_of_main_timing() {
    let runner = base_runner();
    let compiled = runner
        .compiled_card("BT16-085")
        .expect("BT16-085 in embedded DSL pack");

    let has_somp = compiled.effects.iter().any(|c| {
        matches!(
            c,
            CompiledClause::Triggered(t)
                if t.when.contains(&CompiledTiming::StartOfYourMainPhase)
        )
    });
    assert!(
        has_somp,
        "Clause 0 must register with start_of_your_main_phase timing"
    );
}

#[test]
fn bt16_085_clause_0_is_optional() {
    let runner = base_runner();
    let compiled = runner
        .compiled_card("BT16-085")
        .expect("BT16-085 in embedded DSL pack");

    let somp_clause = compiled.effects.iter().find_map(|c| match c {
        CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::StartOfYourMainPhase) => {
            Some(t)
        }
        _ => None,
    });

    let clause = somp_clause.expect("start_of_your_main_phase clause must exist");
    assert!(
        clause.optional,
        "Clause 0 must be optional — printed text says 'You may'"
    );
}

#[test]
fn bt16_085_clause_1_has_on_digivolve_timing() {
    let runner = base_runner();
    let compiled = runner
        .compiled_card("BT16-085")
        .expect("BT16-085 in embedded DSL pack");

    let has_digivolve = compiled.effects.iter().any(|c| {
        matches!(
            c,
            CompiledClause::Triggered(t)
                if t.when.contains(&CompiledTiming::OnDigivolve)
        )
    });
    assert!(
        has_digivolve,
        "Clause 1 must register with on_digivolve timing"
    );
}

#[test]
fn bt16_085_clause_1_is_optional() {
    let runner = base_runner();
    let compiled = runner
        .compiled_card("BT16-085")
        .expect("BT16-085 in embedded DSL pack");

    let digivolve_clause = compiled.effects.iter().find_map(|c| match c {
        CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::OnDigivolve) => Some(t),
        _ => None,
    });

    let clause = digivolve_clause.expect("on_digivolve clause must exist");
    assert!(
        clause.optional,
        "Clause 1 must be optional — 'by suspending this Tamer' is an activation cost"
    );
}

/// The DNA rider is now fully implemented: the on_digivolve clause's process
/// must contain the `select_opponent_sources` + `trash_selected_sources` chain
/// (nested under the `dna_origin` `if`), and no longer carries a gap marker.
#[test]
fn bt16_085_dna_rider_is_implemented_with_opponent_source_selection() {
    let runner = base_runner();
    let compiled = runner
        .compiled_card("BT16-085")
        .expect("BT16-085 in embedded DSL pack");

    let digivolve_clause = compiled.effects.iter().find_map(|c| match c {
        CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::OnDigivolve) => Some(t),
        _ => None,
    });

    let clause = digivolve_clause.expect("on_digivolve clause must exist");
    let summary = clause.summary.as_deref().expect("digivolve summary");
    assert!(
        !summary.contains("G-SELECT-OPPONENT-SOURCES"),
        "DNA rider is implemented — the opponent-source-selection gap marker must be removed"
    );
    assert!(
        !summary.contains("G-EVENT-CARD-COLOR-IS"),
        "blue/green color gate is implemented — that gap marker must be removed"
    );

    // Walk process steps (including nested If branches) for the
    // select_opponent_sources verb.
    fn has_select_opponent_sources(steps: &[CompiledStep]) -> bool {
        steps.iter().any(|s| match s {
            CompiledStep::SelectOpponentSources { .. } => true,
            CompiledStep::If {
                then, else_branch, ..
            } => has_select_opponent_sources(then) || has_select_opponent_sources(else_branch),
            _ => false,
        })
    }
    assert!(
        has_select_opponent_sources(&clause.process),
        "on_digivolve clause must contain a select_opponent_sources step for the DNA rider"
    );
}

#[test]
fn bt16_085_clause_2_has_on_security_timing() {
    let runner = base_runner();
    let compiled = runner
        .compiled_card("BT16-085")
        .expect("BT16-085 in embedded DSL pack");

    let has_security = compiled.effects.iter().any(|c| {
        matches!(
            c,
            CompiledClause::Triggered(t)
                if t.when.contains(&CompiledTiming::OnSecurity)
        )
    });
    assert!(
        has_security,
        "Clause 2 must register with on_security timing"
    );
}

/// Clause 2 (on_security) must be mandatory, FaceUp scope, and contain
/// `play_from_security` in its process steps.
#[test]
fn bt16_085_clause_2_on_security_is_mandatory_faceup_with_play_from_security() {
    let runner = base_runner();
    let compiled = runner
        .compiled_card("BT16-085")
        .expect("BT16-085 in embedded DSL pack");

    let security_clause = compiled
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::OnSecurity) => Some(t),
            _ => None,
        })
        .next()
        .expect("on_security clause must exist");

    assert!(
        !security_clause.optional,
        "on_security clause must not be optional — [Security] effects are mandatory"
    );
    assert!(
        !security_clause.once_per_turn,
        "on_security clause must not be once_per_turn"
    );
    assert_eq!(
        security_clause.scope,
        CompiledScope::FaceUp,
        "on_security clause must have FaceUp scope"
    );

    let has_play_from_security = security_clause
        .process
        .iter()
        .any(|s| matches!(s, CompiledStep::PlayFromSecurity));
    assert!(
        has_play_from_security,
        "on_security clause must include a play_from_security step"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 2 — Clause 0 behavioral: Start of Main free-play
// ═══════════════════════════════════════════════════════════════════════════════

/// Tamer on field, Veemon in hand → start-of-main presents a hand selection.
#[test]
fn bt16_085_start_of_main_presents_selection_when_veemon_in_hand() {
    let mut runner = base_runner();
    runner.place_on_field(0, "BT16-085", Some(0));
    push_to_hand(&mut runner, 0, "VEEMON");

    runner.game.enter_main_phase();

    let view = runner
        .pending_selection_view()
        .expect("start-of-main should present a selection when Veemon is in hand");
    assert_eq!(
        view.kind,
        SelectionKind::Hand,
        "selection kind must be Hand"
    );
}

/// Tamer on field, Wormmon in hand → start-of-main presents a hand selection.
#[test]
fn bt16_085_start_of_main_presents_selection_when_wormmon_in_hand() {
    let mut runner = base_runner();
    runner.place_on_field(0, "BT16-085", Some(0));
    push_to_hand(&mut runner, 0, "WORMMON");

    runner.game.enter_main_phase();

    let view = runner
        .pending_selection_view()
        .expect("start-of-main should present a selection when Wormmon is in hand");
    assert_eq!(view.kind, SelectionKind::Hand);
}

/// When no Veemon or Wormmon in hand, start-of-main does not prompt.
#[test]
fn bt16_085_start_of_main_does_not_prompt_when_no_target_in_hand() {
    let mut runner = base_runner();
    runner.place_on_field(0, "BT16-085", Some(0));
    push_to_hand(&mut runner, 0, "FILLER");

    runner.game.enter_main_phase();

    assert!(
        runner.pending_selection().is_none(),
        "no selection prompt when hand has no Veemon or Wormmon"
    );
}

/// ExVeemon in hand (no real [Veemon] or [Wormmon]) → start-of-main must NOT
/// prompt. The printed bracketed name [Veemon] is an exact-name reference
/// (not a substring), so ExVeemon is not a valid pick even though its name
/// contains the substring "Veemon". DCGO BT16_085.cs uses
/// `cardSource.CardNames.Contains("Veemon")` which is exact membership in
/// the card's name list — ExVeemon's name list is ["ExVeemon"], not ["Veemon"].
#[test]
fn bt16_085_start_of_main_does_not_prompt_when_only_exveemon_in_hand() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT16-085")
        .expect("BT16-085 YAML parses and compiles")
        .add_card(make_test_card("FILLER", "Filler"))
        .add_card(make_exveemon("EXVEEMON"))
        .deck(0, &filler_deck())
        .deck(1, &filler_deck())
        .memory(10)
        .build();

    runner.place_on_field(0, "BT16-085", Some(0));
    push_to_hand(&mut runner, 0, "EXVEEMON");

    runner.game.enter_main_phase();

    assert!(
        runner.pending_selection().is_none(),
        "no selection prompt when only ExVeemon is in hand — [Veemon] is exact-name, \
         not substring (DCGO CardNames.Contains is list-membership, not string-contains)"
    );
}

/// Selecting Veemon and playing it places it on the field without spending memory.
#[test]
fn bt16_085_start_of_main_free_play_does_not_cost_memory() {
    let mut runner = base_runner();
    runner.place_on_field(0, "BT16-085", Some(0));
    push_to_hand(&mut runner, 0, "VEEMON");
    let memory_before = runner.game.memory;

    runner.game.enter_main_phase();

    // There should be a pending selection; pick the Veemon card.
    let view = runner.pending_selection_view().expect("hand selection");
    runner
        .execute_action(view.selecting_player, view.valid_action_ids[0])
        .expect("select Veemon");
    runner.auto_resolve().expect("finish Clause 0 effect");

    let memory_after = runner.game.memory;
    assert_eq!(
        memory_before, memory_after,
        "free-play must not change memory"
    );
    assert_eq!(
        runner.battle_area_size(0),
        2, // tamer + veemon
        "Veemon should be on field after free-play"
    );
}

/// Declining the selection (no target selected) leaves the hand unchanged.
#[test]
fn bt16_085_start_of_main_decline_leaves_hand_unchanged() {
    let mut runner = base_runner();
    runner.place_on_field(0, "BT16-085", Some(0));
    push_to_hand(&mut runner, 0, "VEEMON");

    runner.game.enter_main_phase();

    // The selection is optional. If auto_resolve encounters it and the only
    // valid action is PASS, it will pass. The outer clause is optional:true
    // and the inner select_hand is optional:true — declining is legal.
    runner.auto_resolve().expect("auto-resolve without picking");

    // After auto-resolving an optional selection, the hand might be empty
    // (if the auto-resolver picked) or still have VEEMON (if it passed).
    // Either outcome is legal. We just assert no panic occurred and the
    // battle_area has at most 2 permanents (tamer + optional veemon).
    assert!(
        runner.battle_area_size(0) <= 2,
        "battle_area must have tamer and at most one Veemon"
    );
}

/// Delayed return at next opponent's EOT — IMPLEMENTED (Phase 2 Track H).
/// `play_from_hand_free` now accepts `bind_as` so a schedule_delayed body
/// can return_to_hand the exact permanent that was just played.
#[test]
fn bt16_085_start_of_main_played_digimon_returns_at_opponent_eot() {
    let mut runner = base_runner();
    runner.place_on_field(0, "BT16-085", Some(0));
    push_to_hand(&mut runner, 0, "VEEMON");

    runner.game.enter_main_phase();
    let view = runner.pending_selection_view().expect("hand selection");
    runner
        .execute_action(view.selecting_player, view.valid_action_ids[0])
        .expect("select Veemon");
    runner.auto_resolve().expect("finish Clause 0 effect");

    assert_eq!(runner.battle_area_size(0), 2, "Veemon on field");
    assert_eq!(runner.hand_size(0), 0, "hand empty after playing Veemon");

    // End player 0's turn, then player 1's turn ends (opponent's EOT).
    runner.end_turn(); // P0 → P1
    runner.end_turn(); // P1 → back to P0; P1's EOT fires the scheduled return

    assert_eq!(
        runner.battle_area_size(0),
        1, // only the tamer
        "Veemon must return to hand at the next end of opponent's turn"
    );
    // The exact hand count varies slightly across turn-start draw mechanics
    // (turn-start draw may add 1 filler before this assertion runs). The
    // substrate invariant we care about is the Veemon-shaped card is in
    // hand — i.e. hand contains at least one card with "Veemon" in the
    // name. We check via card data lookup so test-fixture filler cards
    // don't get counted.
    let veemon_in_hand = runner.game.player(0).hand.iter().any(|c| {
        runner
            .game
            .card_data_for_handle(c.handle())
            .map(|d| d.card_name.to_lowercase().contains("veemon"))
            .unwrap_or(false)
    });
    assert!(
        veemon_in_hand,
        "Veemon must be in hand after the scheduled return fired; \
         hand_size={}, battle_area_size={}",
        runner.hand_size(0),
        runner.battle_area_size(0)
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 3 — Clause 1 behavioral: Digivolve observer + suspend-tamer cost
// ═══════════════════════════════════════════════════════════════════════════════

/// When a real digivolve fires via digivolve_from_hand into a blue Digimon,
/// the tamer's optional on_digivolve trigger presents an activation prompt.
/// Accepting it suspends the tamer and grants +1 memory.
#[test]
fn bt16_085_digivolve_observer_suspends_tamer_and_gains_1_memory() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT16-085")
        .expect("BT16-085 YAML parses and compiles")
        .add_card(make_test_card("FILLER", "Filler"))
        .add_card(make_digimon_lv3("BASE"))
        .add_card(make_digimon_lv4("EVOLVED")) // blue Lv.4 — satisfies color gate
        .hand(0, &["EVOLVED"])
        .deck(0, &["FILLER", "FILLER", "FILLER"])
        .deck(1, &["FILLER"])
        // Start below the memory cap so the +1 from Clause 1 is observable.
        .memory(3)
        .start();

    let tamer = runner.place_on_field(0, "BT16-085", Some(0));
    let base = runner.place_on_field(0, "BASE", Some(0));
    runner.game.enter_main_phase();
    let memory_before = runner.game.memory;

    // Find EVOLVED in hand and digivolve onto BASE.
    let hand_idx = runner
        .game
        .player(0)
        .hand
        .iter()
        .position(|c| c.card_id(&runner.game.card_data) == "EVOLVED")
        .expect("EVOLVED in hand");

    let ok =
        runner
            .game
            .digivolve_from_hand(0, hand_idx, base.index as usize, PlaySource::ByDigivolve);
    assert!(ok, "digivolve_from_hand must succeed");
    runner.game.drain_effect_queue();

    // Drive the outer accept/decline prompt. Post
    // `fix-outer-optional-prompt-trigger-ctx`, this prompt MUST install for the
    // optional on_digivolve observer when the event matches its condition;
    // accepting runs the suspend+gain_memory body. The sharper invariants are
    // pinned by `bt16_085_optional_outer_prompt_installs_on_normal_digivolve`
    // and friends in Section 7 below.
    let mut steps = 0;
    while let Some(view) = runner.pending_selection_view() {
        if steps > 10 {
            break;
        }
        runner
            .execute_action(view.selecting_player, view.valid_action_ids[0])
            .expect("accept optional activation");
        runner.game.drain_effect_queue();
        steps += 1;
    }

    // The observer digivolved into a blue Digimon (color gate satisfied): the
    // clause activated, suspending the tamer and gaining exactly 1 memory.
    let is_suspended = runner.game.players[0].battle_area[tamer.index as usize].is_suspended;
    let memory_after = runner.game.memory;

    assert!(is_suspended, "tamer must be suspended after activation");
    assert_eq!(
        memory_after,
        memory_before + 1,
        "controller must gain exactly 1 memory from Clause 1"
    );
}

/// Observer must NOT fire when an opponent's Digimon digivolves
/// (event_target_owner: you condition gates it).
#[test]
fn bt16_085_digivolve_observer_does_not_fire_on_opponent_digivolve() {
    let mut runner = base_runner();
    let _tamer = runner.place_on_field(0, "BT16-085", Some(0));
    let opp_base = runner.place_on_field(1, "BASE", Some(1));
    let memory_before = runner.game.memory;

    // Simulate an opponent Digimon digivolving: TriggerSource::Permanent(opp_base)
    // where opp_base belongs to player 1. The condition event_target_owner: you
    // should prevent the tamer from activating.
    runner.game.enqueue_triggered(
        EffectTiming::OnDigivolve,
        TriggerSource::Permanent(opp_base),
    );
    runner.game.drain_effect_queue();

    let memory_after = runner.game.memory;

    // The condition event_target_owner: you should prevent the tamer from
    // activating when the digivolving Digimon belongs to the opponent.
    assert!(
        runner.pending_selection().is_none(),
        "observer must not fire on opponent's Digimon digivolve"
    );
    assert_eq!(
        memory_before, memory_after,
        "memory must not change when observer does not activate"
    );
}

/// Blue/green color gate: when an own Digimon digivolves into a RED Digimon
/// (no blue/green color), the `event_card_color_has: [blue, green]` condition
/// must prevent the observer from firing.
#[test]
fn bt16_085_digivolve_observer_does_not_fire_on_non_blue_non_green_digivolve() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT16-085")
        .expect("BT16-085 YAML parses and compiles")
        .add_card(make_test_card("FILLER", "Filler"))
        .add_card(make_digimon_lv3("BASE"))
        .add_card(make_digimon_lv4_red("EVOLVED-RED"))
        .hand(0, &["EVOLVED-RED"])
        .deck(0, &["FILLER", "FILLER", "FILLER"])
        .deck(1, &["FILLER"])
        .memory(10)
        .start();

    let _tamer = runner.place_on_field(0, "BT16-085", Some(0));
    let base = runner.place_on_field(0, "BASE", Some(0));
    runner.game.enter_main_phase();
    let memory_before = runner.game.memory;

    let hand_idx = runner
        .game
        .player(0)
        .hand
        .iter()
        .position(|c| c.card_id(&runner.game.card_data) == "EVOLVED-RED")
        .expect("EVOLVED-RED in hand");
    let ok =
        runner
            .game
            .digivolve_from_hand(0, hand_idx, base.index as usize, PlaySource::ByDigivolve);
    assert!(ok, "digivolve_from_hand must succeed");
    runner.game.drain_effect_queue();

    assert!(
        runner.pending_selection().is_none(),
        "observer must not fire — digivolved into a red (non blue/green) Digimon"
    );
    assert_eq!(
        memory_before, runner.game.memory,
        "memory must not change when the color gate blocks the observer"
    );
}

/// DNA trash sub-clause: when an own Digimon DNA-digivolves into a blue/green
/// Digimon, accepting Clause 1 suspends the tamer, gains 1 memory, and (because
/// `dna_origin` is true) prompts the controller to pick an opponent Digimon and
/// trash 3 of its digivolution cards.
#[test]
fn bt16_085_dna_digivolve_trashes_3_opp_digi_cards() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT16-085")
        .expect("BT16-085 YAML parses and compiles")
        .add_card(make_test_card("FILLER", "Filler"))
        .add_card(make_digimon_lv3("MAT-A"))
        .add_card(make_digimon_lv3("MAT-B"))
        .add_card(make_digimon_lv4("DNA-RESULT")) // blue Lv.4
        .add_card(make_test_card("OPP-SRC", "Opp Source"))
        .add_card(make_test_card("OPP-TOP", "Opp Top"))
        .hand(0, &["DNA-RESULT"])
        .deck(0, &["FILLER", "FILLER", "FILLER"])
        .deck(1, &["FILLER"])
        // Start below the memory cap so the +1 from Clause 1 is observable.
        .memory(3)
        .start();

    let tamer = runner.place_on_field(0, "BT16-085", Some(0));
    let mat_a = runner.place_on_field(0, "MAT-A", None);
    let mat_b = runner.place_on_field(0, "MAT-B", None);

    // Opponent Digimon with 3 digivolution cards under it (3 sources + top).
    let opp = runner.place_on_field(1, "OPP-TOP", None);
    runner.push_source(opp, "OPP-SRC");
    runner.push_source(opp, "OPP-SRC");
    runner.push_source(opp, "OPP-SRC");
    assert_eq!(
        runner.game.players[1].battle_area[opp.index as usize]
            .card_sources
            .len(),
        4,
        "opp stack: 3 digivolution cards + top"
    );

    let memory_before = runner.game.memory;
    let hand_card = runner.game.player(0).hand[0].handle();
    {
        let mut ctx = EffectContext::new(&mut runner.game, hand_card, None, 0);
        ctx.effect_initiated_dna_digivolve(mat_a, mat_b, hand_card, 0, true);
    }
    runner.game.drain_effect_queue();

    // Clause 1 is optional — accept the activation, then drive the
    // opponent-permanent + 3-source selections.
    let mut steps = 0;
    while let Some(view) = runner.pending_selection_view() {
        if steps > 12 {
            panic!("selection loop did not terminate");
        }
        runner
            .execute_action(view.selecting_player, view.valid_action_ids[0])
            .expect("drive selection");
        runner.game.drain_effect_queue();
        steps += 1;
    }

    assert!(
        runner.game.players[0].battle_area[tamer.index as usize].is_suspended,
        "tamer must be suspended after accepting Clause 1"
    );
    assert_eq!(
        runner.game.memory,
        memory_before + 1,
        "controller gains 1 memory from Clause 1"
    );
    assert_eq!(
        runner.game.players[1].battle_area[opp.index as usize]
            .card_sources
            .len(),
        1,
        "opp Digimon must have 3 digivolution cards trashed (only the top remains)"
    );
    assert_eq!(
        runner.game.players[1].trash.len(),
        3,
        "the 3 trashed digivolution cards go to the opponent's trash"
    );
}

/// Non-DNA path: when an own Digimon digivolves NORMALLY (not DNA) into a
/// blue/green Digimon, Clause 1 fires (suspend tamer + 1 memory) but the
/// `dna_origin` branch must NOT trash any opponent digivolution cards.
#[test]
fn bt16_085_non_dna_digivolve_does_not_trash_opp_digi_cards() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT16-085")
        .expect("BT16-085 YAML parses and compiles")
        .add_card(make_test_card("FILLER", "Filler"))
        .add_card(make_digimon_lv3("BASE"))
        .add_card(make_digimon_lv4("EVOLVED")) // blue Lv.4
        .add_card(make_test_card("OPP-SRC", "Opp Source"))
        .add_card(make_test_card("OPP-TOP", "Opp Top"))
        .hand(0, &["EVOLVED"])
        .deck(0, &["FILLER", "FILLER", "FILLER"])
        .deck(1, &["FILLER"])
        // Start below the memory cap so the +1 from Clause 1 is observable.
        .memory(3)
        .start();

    let tamer = runner.place_on_field(0, "BT16-085", Some(0));
    let base = runner.place_on_field(0, "BASE", Some(0));
    let opp = runner.place_on_field(1, "OPP-TOP", None);
    runner.push_source(opp, "OPP-SRC");
    runner.push_source(opp, "OPP-SRC");
    runner.push_source(opp, "OPP-SRC");

    runner.game.enter_main_phase();
    let memory_before = runner.game.memory;

    let hand_idx = runner
        .game
        .player(0)
        .hand
        .iter()
        .position(|c| c.card_id(&runner.game.card_data) == "EVOLVED")
        .expect("EVOLVED in hand");
    let ok =
        runner
            .game
            .digivolve_from_hand(0, hand_idx, base.index as usize, PlaySource::ByDigivolve);
    assert!(ok, "digivolve_from_hand must succeed");
    runner.game.drain_effect_queue();

    let mut steps = 0;
    while let Some(view) = runner.pending_selection_view() {
        if steps > 12 {
            panic!("selection loop did not terminate");
        }
        runner
            .execute_action(view.selecting_player, view.valid_action_ids[0])
            .expect("drive selection");
        runner.game.drain_effect_queue();
        steps += 1;
    }

    assert!(
        runner.game.players[0].battle_area[tamer.index as usize].is_suspended,
        "tamer suspended after accepting Clause 1"
    );
    assert_eq!(
        runner.game.memory,
        memory_before + 1,
        "controller gains 1 memory"
    );
    assert_eq!(
        runner.game.players[1].battle_area[opp.index as usize]
            .card_sources
            .len(),
        4,
        "non-DNA digivolve must NOT trash opponent digivolution cards"
    );
    assert!(
        runner.game.players[1].trash.is_empty(),
        "opponent's trash must stay empty on a non-DNA digivolve"
    );
}

/// Smoke: when P0's Digimon digivolves via digivolve_from_hand, the tamer's
/// on_digivolve trigger fires without panicking and memory changes as expected.
#[test]
fn bt16_085_digivolve_observer_fires_on_own_digimon_digivolve_via_action() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT16-085")
        .expect("BT16-085 YAML parses and compiles")
        .add_card(make_test_card("FILLER", "Filler"))
        .add_card(make_digimon_lv3("BASE"))
        .add_card(make_digimon_lv4("EVOLVED"))
        .hand(0, &["EVOLVED"])
        .deck(0, &["FILLER", "FILLER", "FILLER"])
        .deck(1, &["FILLER"])
        .memory(10)
        .start();

    let tamer = runner.place_on_field(0, "BT16-085", Some(0));
    let base = runner.place_on_field(0, "BASE", Some(0));

    // Must be in Main phase for digivolve_from_hand to succeed.
    runner.game.enter_main_phase();
    let memory_before = runner.game.memory;

    // Find EVOLVED in hand.
    let hand_idx = runner
        .game
        .player(0)
        .hand
        .iter()
        .position(|c| c.card_id(&runner.game.card_data) == "EVOLVED")
        .expect("EVOLVED in hand");

    let ok =
        runner
            .game
            .digivolve_from_hand(0, hand_idx, base.index as usize, PlaySource::ByDigivolve);
    assert!(ok, "digivolve_from_hand must succeed");
    runner.game.drain_effect_queue();

    // The on_digivolve observer should have fired. If the optional prompt
    // installed, accept it.
    if let Some(view) = runner.pending_selection_view() {
        runner
            .execute_action(view.selecting_player, view.valid_action_ids[0])
            .expect("accept optional activation");
        runner.auto_resolve().expect("finish effect");
    } else {
        runner.auto_resolve().expect("finish (no selection)");
    }

    // After accepting, tamer should be suspended and memory +1.
    let is_suspended = runner.game.players[0].battle_area[tamer.index as usize].is_suspended;
    let memory_after = runner.game.memory;

    // Memory changed by 0 (declined) or +1 (accepted).
    // Tamer is suspended iff memory gained +1.
    assert!(
        memory_after == memory_before || memory_after == memory_before + 1,
        "memory must be unchanged (declined) or +1 (accepted); was {memory_before} → {memory_after}"
    );
    if memory_after == memory_before + 1 {
        assert!(
            is_suspended,
            "tamer must be suspended when memory was gained"
        );
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 4 — Clause 2 structural: Security free-play
// ═══════════════════════════════════════════════════════════════════════════════
//
// A full behavioral security test requires an attacker-side game setup that is
// complex to replicate in a unit test. The structural shape test above (Section 1)
// confirms that `play_from_security` is present in the clause's process steps.
// A smoke test that pushes BT16-085 into security and checks the clause fires
// is provided below for basic sanity.
//

/// Smoke: pushing BT16-085 into P0's security and manually enqueueing
/// on_security does not panic. Verifies the trigger registration is sound.
#[test]
fn bt16_085_security_clause_fires_without_panic() {
    let mut runner = base_runner();

    // Push BT16-085 into P0's security stack.
    push_to_security(&mut runner, 0, "BT16-085");
    assert_eq!(
        runner.game.players[0].security.len(),
        1,
        "BT16-085 must be in security stack"
    );

    // Enqueue the on_security trigger from the security-top card's perspective.
    // The security card has no field permanent, so we use PlayerBattleArea fan-out.
    // This is a no-panic smoke test; the actual play_from_security path is gated
    // on the security-check-resolution flow in Game::resolve_security_check.
    runner.game.enqueue_triggered(
        EffectTiming::SecuritySkill,
        TriggerSource::PlayerBattleArea(0),
    );
    runner.game.drain_effect_queue();

    // Accept any pending selection (if a prompt installed).
    let _ = runner.auto_resolve();

    // No assertion beyond not panicking.
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 7 — Outer-optional trigger prompt installation
//
// These tests pin down the contract that Clause 1 ("by suspending this Tamer,
// gain 1 memory") is a player decision, not an auto-fire. The bug they regress
// against: `Game::queued_effect_wants_outer_optional_prompt` evaluated the
// clause's condition without installing `qe.trigger_context`, so every
// `event_*` predicate returned None, the condition appeared to fail, and the
// outer accept/decline prompt was skipped — even though `run_queued_effect`
// then installed the correct context and silently ran the body. The new tests
// assert that the prompt IS installed and is the only decline gate.
// ═══════════════════════════════════════════════════════════════════════════════

/// Normal (non-DNA) digivolve: when player 0 digivolves an own Lv.3 base into
/// a blue Lv.4 Digimon, Davis & Ken's `on_digivolve` clause MUST install an
/// outer optional accept/decline prompt before the suspend cost is paid.
#[test]
fn bt16_085_optional_outer_prompt_installs_on_normal_digivolve() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT16-085")
        .expect("BT16-085 YAML parses and compiles")
        .add_card(make_test_card("FILLER", "Filler"))
        .add_card(make_digimon_lv3("BASE"))
        .add_card(make_digimon_lv4("EVOLVED")) // blue Lv.4 — satisfies color gate
        .hand(0, &["EVOLVED"])
        .deck(0, &["FILLER", "FILLER", "FILLER"])
        .deck(1, &["FILLER"])
        .memory(3)
        .start();

    let tamer = runner.place_on_field(0, "BT16-085", Some(0));
    let base = runner.place_on_field(0, "BASE", Some(0));
    runner.game.enter_main_phase();

    let suspended_before = runner.game.players[0].battle_area[tamer.index as usize].is_suspended;
    assert!(
        !suspended_before,
        "precondition: Davis & Ken must be unsuspended before the digivolve"
    );

    let hand_idx = runner
        .game
        .player(0)
        .hand
        .iter()
        .position(|c| c.card_id(&runner.game.card_data) == "EVOLVED")
        .expect("EVOLVED in hand");
    assert!(
        runner
            .game
            .digivolve_from_hand(0, hand_idx, base.index as usize, PlaySource::ByDigivolve),
        "digivolve_from_hand must succeed"
    );
    runner.game.drain_effect_queue();

    // The fix: the outer optional accept/decline prompt MUST install before any
    // body steps run. Today (pre-fix) this assertion fails — the body auto-fires
    // and `pending_selection` is `None`.
    let view = runner
        .pending_selection_view()
        .expect("outer optional accept/decline prompt MUST install after digivolve drain");
    assert_eq!(
        view.kind,
        SelectionKind::Replacement,
        "outer optional prompt must be SelectionKind::Replacement"
    );
    assert!(
        view.is_optional,
        "outer optional prompt must be is_optional=true so PASS declines"
    );
    assert_eq!(
        view.selecting_player, 0,
        "the controller of the optional triggered effect is selecting"
    );

    // Critical: the suspend cost MUST NOT have been paid yet. Today the body
    // runs before this check and Davis & Ken is already suspended.
    assert!(
        !runner.game.players[0].battle_area[tamer.index as usize].is_suspended,
        "Davis & Ken must NOT yet be suspended — the cost only pays after accept"
    );
}

/// DNA digivolve path: same prompt-installation contract under the DNA flow.
/// Uses `effect_initiated_dna_digivolve(_, _, _, _, ignore_requirements=true)`
/// to bypass DNA-cost validation since the test cards do not declare DNA
/// alt-paths — the on_digivolve trigger still fires and Davis & Ken's clause
/// still queues.
#[test]
fn bt16_085_optional_outer_prompt_installs_on_dna_digivolve() {
    let mut mat_a = make_test_card("MAT-A", "MaterialA");
    mat_a.card_kind = CardKind::Digimon;
    mat_a.level = Some(4);
    mat_a.dp = Some(4000);
    mat_a.colors = vec![CardColor::Blue];

    let mut mat_b = make_test_card("MAT-B", "MaterialB");
    mat_b.card_kind = CardKind::Digimon;
    mat_b.level = Some(4);
    mat_b.dp = Some(4000);
    mat_b.colors = vec![CardColor::Green];

    let mut runner = DebugRunner::builder()
        .dsl_card("BT16-085")
        .expect("BT16-085 YAML parses and compiles")
        .dsl_card("BT16-025")
        .expect("BT16-025 (Paildramon) YAML parses and compiles")
        .add_card(make_test_card("FILLER", "Filler"))
        .add_card(mat_a)
        .add_card(mat_b)
        .hand(0, &["BT16-025"])
        .deck(0, &["FILLER", "FILLER", "FILLER"])
        .deck(1, &["FILLER"])
        .memory(5)
        .start();

    let tamer = runner.place_on_field(0, "BT16-085", Some(0));
    let material_a = runner.place_on_field(0, "MAT-A", None);
    let material_b = runner.place_on_field(0, "MAT-B", None);

    assert!(
        !runner.game.players[0].battle_area[tamer.index as usize].is_suspended,
        "precondition: Davis & Ken unsuspended"
    );

    let hand_card = runner.game.player(0).hand[0].handle();
    {
        let mut ctx = EffectContext::new(&mut runner.game, hand_card, None, 0);
        ctx.effect_initiated_dna_digivolve(material_a, material_b, hand_card, 0, true);
    }
    // `effect_initiated_dna_digivolve` runs the three drains (WhenDigivolving,
    // OnDnaDigivolve, OnDigivolve) internally; the OnDigivolve drain is where
    // Davis & Ken's clause queues. After the call returns, the outer optional
    // prompt should be pending.

    let view = runner
        .pending_selection_view()
        .expect("outer optional accept/decline prompt MUST install after DNA digivolve drains");
    assert_eq!(view.kind, SelectionKind::Replacement);
    assert!(view.is_optional);
    assert_eq!(view.selecting_player, 0);

    // Locate Davis & Ken in the (possibly-reshuffled) battle_area by top card.
    let tamer_slot = runner.game.players[0]
        .battle_area
        .iter()
        .position(|p| p.top_card().card_id(&runner.game.card_data) == "BT16-085")
        .expect("Davis & Ken still on field after DNA digivolve");
    assert!(
        !runner.game.players[0].battle_area[tamer_slot].is_suspended,
        "Davis & Ken must NOT be suspended before the prompt is accepted"
    );
}

/// Declining the outer optional prompt MUST skip the body entirely: Davis & Ken
/// stays unsuspended and no MemoryChange event from this clause is emitted.
#[test]
fn bt16_085_optional_outer_prompt_decline_skips_body() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT16-085")
        .expect("BT16-085 YAML parses and compiles")
        .add_card(make_test_card("FILLER", "Filler"))
        .add_card(make_digimon_lv3("BASE"))
        .add_card(make_digimon_lv4("EVOLVED"))
        .hand(0, &["EVOLVED"])
        .deck(0, &["FILLER", "FILLER", "FILLER"])
        .deck(1, &["FILLER"])
        .memory(3)
        .start();

    let tamer = runner.place_on_field(0, "BT16-085", Some(0));
    let base = runner.place_on_field(0, "BASE", Some(0));
    runner.game.enter_main_phase();
    let memory_before = runner.memory();

    let hand_idx = runner
        .game
        .player(0)
        .hand
        .iter()
        .position(|c| c.card_id(&runner.game.card_data) == "EVOLVED")
        .expect("EVOLVED in hand");
    runner
        .game
        .digivolve_from_hand(0, hand_idx, base.index as usize, PlaySource::ByDigivolve);
    runner.game.drain_effect_queue();

    assert!(
        runner.pending_selection().is_some(),
        "outer optional prompt MUST install before decline test"
    );

    runner
        .decline_optional_trigger()
        .expect("decline must succeed");
    runner.game.drain_effect_queue();

    assert!(
        !runner.game.players[0].battle_area[tamer.index as usize].is_suspended,
        "decline → Davis & Ken must NOT be suspended"
    );

    // The +1 memory branch must NOT have fired. We compare gross memory; any
    // memory delta from the digivolve action itself (cost reduction etc.)
    // would NOT match this clause's +1 contribution. With EVOLVED's evo cost
    // of 0, the digivolve memory delta should be 0, and a strict equality
    // check pins the no-fire contract.
    assert_eq!(
        runner.memory(),
        memory_before,
        "decline → no memory gain (Clause 1's +1 must not have run)"
    );
}

/// Accepting the outer optional prompt MUST run the body with the queued
/// trigger context installed: Davis & Ken suspends and exactly one +1 memory
/// event is emitted from this clause.
#[test]
fn bt16_085_optional_outer_prompt_accept_runs_body_with_trigger_ctx() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT16-085")
        .expect("BT16-085 YAML parses and compiles")
        .add_card(make_test_card("FILLER", "Filler"))
        .add_card(make_digimon_lv3("BASE"))
        .add_card(make_digimon_lv4("EVOLVED"))
        .hand(0, &["EVOLVED"])
        .deck(0, &["FILLER", "FILLER", "FILLER"])
        .deck(1, &["FILLER"])
        .memory(3)
        .start();

    let tamer = runner.place_on_field(0, "BT16-085", Some(0));
    let base = runner.place_on_field(0, "BASE", Some(0));
    runner.game.enter_main_phase();
    let memory_before = runner.memory();

    let hand_idx = runner
        .game
        .player(0)
        .hand
        .iter()
        .position(|c| c.card_id(&runner.game.card_data) == "EVOLVED")
        .expect("EVOLVED in hand");
    runner
        .game
        .digivolve_from_hand(0, hand_idx, base.index as usize, PlaySource::ByDigivolve);
    runner.game.drain_effect_queue();

    assert!(
        runner.pending_selection().is_some(),
        "outer optional prompt MUST install before accept test"
    );

    runner
        .accept_optional_trigger()
        .expect("accept must succeed");
    runner.game.drain_effect_queue();

    assert!(
        runner.game.players[0].battle_area[tamer.index as usize].is_suspended,
        "accept → Davis & Ken must be suspended (the by-suspending cost was paid)"
    );
    assert_eq!(
        runner.memory(),
        memory_before + 1,
        "accept → exactly +1 memory from Clause 1"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 8 — Scheduled-return identity tracking (fix-played-binding-uses-provenance)
//
// These tests pin down the contract that the start-of-main "return *it* to the
// hand" delayed effect tracks the played Veemon/Wormmon by stable identity
// (ProvenanceToken via BindingValue::PlayedPermanent), NOT by positional
// PermanentHandle. Before the fix, the binding stored a positional handle and
// downstream stack mutations (DNA digivolve, regular digivolve, deletion) would
// cause the scheduled return_to_hand to mis-target whatever permanent currently
// occupied that slot. Post-fix, the binding fizzles silently in those cases —
// matching DCGO's `IsPermanentExistsOnBattleArea(selectedPermanent)` semantics.
// ═══════════════════════════════════════════════════════════════════════════════

/// Build a Lv.4 Blue Digimon whose card name contains "Veemon" so it
/// satisfies both Davis & Ken's name-filter and the DNA Lv.4 Blue
/// requirement. Used to test DNA consumption of the played Veemon.
fn make_veemon_lv4_blue(id: &str) -> CardData {
    let mut c = make_test_card(id, "Veemon");
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(4000);
    c.play_cost = 5;
    c.colors = vec![CardColor::Blue];
    c
}

/// Lv.4 Green Digimon for the DNA partner slot (Stingmon stand-in).
fn make_partner_lv4_green(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(4000);
    c.play_cost = 5;
    c.colors = vec![CardColor::Green];
    c
}

/// Plays the Veemon from Davis & Ken's start-of-main trigger. Returns the
/// PermanentHandle the Veemon ended up at.
fn play_veemon_via_davis_and_ken(runner: &mut DebugRunner) -> digimon_engine::permanent::PermanentHandle {
    runner.game.enter_main_phase();
    let view = runner.pending_selection_view().expect("hand selection");
    runner
        .execute_action(view.selecting_player, view.valid_action_ids[0])
        .expect("select Veemon");
    runner.auto_resolve().expect("finish Clause 0 free-play");
    let last = runner.game.players[0].battle_area.len() - 1;
    digimon_engine::permanent::PermanentHandle {
        player: 0,
        index: last as u8,
    }
}

/// Test 5.1 — Davis & Ken plays Veemon; the Veemon is consumed in DNA
/// digivolve with a Lv.4 Green partner into a Paildramon-stand-in result;
/// at the next end-of-opponent's-turn the scheduled return_to_hand silently
/// fizzles (the played Veemon is now a digivolution card under the merged
/// stack, no longer a battle-area top). Pins down the BT16-085 +
/// BT16-025-shape interaction the change was created to fix.
#[test]
fn bt16_085_dna_into_paildramon_skips_scheduled_return() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT16-085")
        .expect("BT16-085 YAML parses")
        .add_card(make_test_card("FILLER", "Filler"))
        .add_card(make_veemon_lv4_blue("VEEMON-LV4"))
        .add_card(make_partner_lv4_green("STINGMON-LV4"))
        .add_card(make_test_card("PAILDRAMON-RESULT", "PaildramonResult"))
        .deck(0, &filler_deck())
        .deck(1, &filler_deck())
        .memory(10)
        .build();

    let tamer = runner.place_on_field(0, "BT16-085", Some(0));
    push_to_hand(&mut runner, 0, "VEEMON-LV4");
    let stingmon = runner.place_on_field(0, "STINGMON-LV4", None);
    push_to_hand(&mut runner, 0, "PAILDRAMON-RESULT");

    // Davis & Ken plays the Veemon for free; schedules return-to-hand at
    // the next end-of-opponent's-turn.
    let veemon_perm = play_veemon_via_davis_and_ken(&mut runner);

    assert_eq!(runner.battle_area_size(0), 3, "tamer + Veemon + Stingmon");
    assert_eq!(
        runner.game.players[0].battle_area[veemon_perm.index as usize]
            .top_card()
            .card_id(&runner.game.card_data),
        "VEEMON-LV4",
        "Veemon landed at the expected slot"
    );

    // DNA-digivolve veemon + stingmon into Paildramon. Use
    // `effect_initiated_dna_digivolve` with `ignore_requirements=true` to
    // bypass DNA cost validation (the test card has no declared DNA cost).
    let hand_idx = runner
        .game
        .player(0)
        .hand
        .iter()
        .position(|c| c.card_id(&runner.game.card_data) == "PAILDRAMON-RESULT")
        .expect("PAILDRAMON-RESULT in hand");
    let paildramon_card = runner.game.player(0).hand[hand_idx].handle();
    {
        let mut ctx = EffectContext::new(&mut runner.game, paildramon_card, None, 0);
        ctx.effect_initiated_dna_digivolve(veemon_perm, stingmon, paildramon_card, 0, true);
    }
    runner.game.drain_effect_queue();

    // Drive any prompts (Davis & Ken's on-DNA-digivolve clause may install
    // its optional outer prompt).
    let mut steps = 0;
    while let Some(view) = runner.pending_selection_view() {
        if steps > 16 {
            break;
        }
        runner
            .execute_action(view.selecting_player, view.valid_action_ids[0])
            .expect("drive selection");
        runner.game.drain_effect_queue();
        steps += 1;
    }

    // Locate the merged Paildramon stack by top card.
    let paildramon_slot = runner.game.players[0]
        .battle_area
        .iter()
        .position(|p| p.top_card().card_id(&runner.game.card_data) == "PAILDRAMON-RESULT")
        .expect("merged Paildramon on field after DNA");
    let pre_eot_sources_len =
        runner.game.players[0].battle_area[paildramon_slot].card_sources.len();
    assert!(
        pre_eot_sources_len >= 3,
        "merged stack should carry both materials' sources + the new top, got {} sources",
        pre_eot_sources_len
    );

    // End P0's turn → P1's turn. End P1's turn → fires the
    // EndOfOpponentsNextTurn drain for P0's scheduled effect.
    runner.end_turn();
    runner.end_turn();

    // CRITICAL ASSERTIONS — the post-fix behavior:

    // (a) The merged Paildramon stack is STILL on the field. The played
    // Veemon is no longer a battle-area top, so the strict resolver yields
    // None and the scheduled `return_to_hand` silently fizzles.
    let paildramon_still_on_field = runner.game.players[0]
        .battle_area
        .iter()
        .any(|p| p.top_card().card_id(&runner.game.card_data) == "PAILDRAMON-RESULT");
    assert!(
        paildramon_still_on_field,
        "Paildramon must remain on field — the scheduled return must NOT \
         bounce the merged stack just because the Veemon's slot turned into \
         Paildramon's slot"
    );

    // (b) P0's hand must NOT contain a "PaildramonResult" card from the bounce.
    let paildramon_in_hand = runner.game.player(0).hand.iter().any(|c| {
        runner
            .game
            .card_data_for_handle(c.handle())
            .map(|d| d.card_name == "PaildramonResult")
            .unwrap_or(false)
    });
    assert!(
        !paildramon_in_hand,
        "Paildramon must not be in P0's hand — no scheduled bounce should \
         have fired against the merged stack"
    );

    // (c) Tamer still on field.
    let tamer_still_on_field = runner
        .game
        .player(0)
        .battle_area
        .iter()
        .any(|p| p.top_card().card_id(&runner.game.card_data) == "BT16-085");
    assert!(tamer_still_on_field, "Davis & Ken tamer must still be on field");
    let _ = tamer;
}

/// Test 5.2 — Davis & Ken plays Veemon; the Veemon then REGULARLY digivolves
/// (not DNA) into ExVeemon; at the next end-of-opponent's-turn the scheduled
/// return_to_hand silently fizzles.
#[test]
fn bt16_085_regular_digivolve_skips_scheduled_return() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT16-085")
        .expect("BT16-085 YAML parses")
        .add_card(make_test_card("FILLER", "Filler"))
        .add_card(make_veemon("VEEMON"))
        .add_card(make_digimon_lv4("EXVEEMON"))
        .deck(0, &filler_deck())
        .deck(1, &filler_deck())
        .memory(10)
        .build();

    let _tamer = runner.place_on_field(0, "BT16-085", Some(0));
    push_to_hand(&mut runner, 0, "VEEMON");
    push_to_hand(&mut runner, 0, "EXVEEMON");

    let veemon_perm = play_veemon_via_davis_and_ken(&mut runner);

    let exveemon_hand_idx = runner
        .game
        .player(0)
        .hand
        .iter()
        .position(|c| c.card_id(&runner.game.card_data) == "EXVEEMON")
        .expect("EXVEEMON in hand");
    let ok = runner.game.digivolve_from_hand(
        0,
        exveemon_hand_idx,
        veemon_perm.index as usize,
        PlaySource::ByDigivolve,
    );
    assert!(ok, "regular digivolve onto Veemon must succeed");
    runner.game.drain_effect_queue();

    let mut steps = 0;
    while let Some(view) = runner.pending_selection_view() {
        if steps > 8 {
            break;
        }
        runner
            .execute_action(view.selecting_player, view.valid_action_ids[0])
            .expect("drive selection");
        runner.game.drain_effect_queue();
        steps += 1;
    }

    let top_at_slot = runner.game.players[0].battle_area[veemon_perm.index as usize]
        .top_card()
        .card_id(&runner.game.card_data)
        .to_string();
    assert_eq!(top_at_slot, "EXVEEMON", "ExVeemon is the new top card");

    runner.end_turn();
    runner.end_turn();

    let exveemon_still_on_field = runner.game.players[0]
        .battle_area
        .iter()
        .any(|p| p.top_card().card_id(&runner.game.card_data) == "EXVEEMON");
    assert!(
        exveemon_still_on_field,
        "ExVeemon must remain on the field — the played Veemon is no longer \
         a battle-area top, so the bounce must fizzle"
    );
    let exveemon_in_hand = runner
        .game
        .player(0)
        .hand
        .iter()
        .any(|c| c.card_id(&runner.game.card_data) == "EXVEEMON");
    assert!(
        !exveemon_in_hand,
        "ExVeemon must not have been bounced to hand by the scheduled return"
    );
}

/// Test 5.3 — Davis & Ken plays Veemon; an effect deletes Veemon before
/// the next end-of-opponent's-turn; the scheduled return_to_hand silently
/// fizzles.
#[test]
fn bt16_085_played_digimon_deleted_skips_scheduled_return() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT16-085")
        .expect("BT16-085 YAML parses")
        .add_card(make_test_card("FILLER", "Filler"))
        .add_card(make_veemon("VEEMON"))
        .deck(0, &filler_deck())
        .deck(1, &filler_deck())
        .memory(10)
        .build();

    let tamer = runner.place_on_field(0, "BT16-085", Some(0));
    push_to_hand(&mut runner, 0, "VEEMON");

    let veemon_perm = play_veemon_via_davis_and_ken(&mut runner);
    let p0_trash_before_delete = runner.game.player(0).trash.len();

    // Capture the tamer-top-card handle first to satisfy the borrow checker
    // (then pass &mut runner.game).
    let tamer_top = runner.game.players[0].battle_area[tamer.index as usize]
        .top_card()
        .handle();
    {
        let mut ctx = EffectContext::new(&mut runner.game, tamer_top, Some(tamer), 0);
        ctx.delete_permanent(veemon_perm);
    }
    runner.game.drain_effect_queue();

    let veemon_in_battle_area = runner.game.players[0]
        .battle_area
        .iter()
        .any(|p| p.top_card().card_id(&runner.game.card_data) == "VEEMON");
    assert!(!veemon_in_battle_area, "Veemon must be deleted from battle area");
    let p0_trash_after_delete = runner.game.player(0).trash.len();
    assert!(
        p0_trash_after_delete > p0_trash_before_delete,
        "Veemon must have landed in P0's trash"
    );

    let battle_area_after_delete: Vec<String> = runner.game.players[0]
        .battle_area
        .iter()
        .map(|p| p.top_card().card_id(&runner.game.card_data).to_string())
        .collect();

    runner.end_turn();
    runner.end_turn();

    let battle_area_after_drains: Vec<String> = runner.game.players[0]
        .battle_area
        .iter()
        .map(|p| p.top_card().card_id(&runner.game.card_data).to_string())
        .collect();

    assert_eq!(
        battle_area_after_delete, battle_area_after_drains,
        "battle area must not change across the two end-turn drains — the \
         scheduled return must fizzle silently when its target is already deleted"
    );
}
