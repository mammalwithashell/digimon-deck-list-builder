//! BT21-056 Vemmon — Digimon, Lv.3, Black, DP 1000, Cost 3.
//! Traits: Unknown/LIBERATOR. Attribute: Unknown.
//! Evo costs: Black Lv.2 / cost 0.
//!
//! # Card text (data/card_bundles/BT21-056.md — official Bandai DB, verbatim)
//!
//! **Effect:**
//! [On Play] By trashing 1 card with [Vemmon] in its text from your hand,
//! you may return 1 non-Digi-Egg card with [Vemmon] in its text from your
//! trash to the hand.
//!
//! **Inherited:**
//! [Your Turn] [Once Per Turn] When this Digimon would digivolve into a
//! Digimon card with [Vemmon] in its text, reduce the digivolution cost by 1.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT21/Black/BT21_056.cs
//!
//! # Patterns this test covers (RUST_DSL_TEST_API §4.3)
//! - A4 (trash-as-cost → return from trash to hand)
//! - E2-adjacent (two-tier optional: cost pick + separately optional return)
//! - D2 (cost reduction with BeforePayCost, `cost_target` +
//!   `source_is_cost_target_permanent`)
//! - G-DSL-PREDICATE-TEXT-CONTAINS (both selects filtered by
//!   `effect_text_contains: "Vemmon"`, matching DCGO `HasText("Vemmon")`)
//!
//! The hand-pick (cost) filter is `effect_text_contains: Vemmon` only (no
//! Digi-Egg exclusion — the printed text's Digi-Egg restriction applies only
//! to the trash-recover half). The trash-recover filter adds
//! `none_of: [{ kind: digi_egg }]` per "1 non-Digi-Egg card ... from your
//! trash".

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_dsl::compiled::{
    CompiledClause, CompiledDeclarativeClause, CompiledScope, CompiledTiming,
};
use digimon_engine::action::space::PASS;
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, CardKind, PlaySource};
use digimon_engine::selection::SelectionKind;

const CARD_ID: &str = "BT21-056";

// ── Fixture builders ──────────────────────────────────────────────────────

/// A filler Digimon with no "Vemmon" text — never a legal target for either
/// select on clause 0.
fn filler(id: &str) -> CardData {
    make_test_card(id, id)
}

/// A Digimon card whose `effect_text` contains "Vemmon" — a legal target for
/// both the hand-trash cost pick and the trash-recover pick (kind: digimon,
/// so also legal for the non-Digi-Egg recover filter).
fn vemmon_text_digimon(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.effect_text = "Place 1 [Vemmon] from your trash under this Digimon.".to_string();
    c
}

/// A Digi-Egg whose `inherited_text` contains "Vemmon" — legal for the
/// hand-trash cost pick (no Digi-Egg exclusion there), but NOT a legal
/// target for the trash-recover pick (excluded by `none_of: [{kind:
/// digi_egg}]`).
fn vemmon_text_digi_egg(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::DigiEgg;
    c.level = Some(2);
    c.inherited_text = "This Digimon may digivolve into [Vemmon].".to_string();
    c
}

/// A Digimon card with a `security_text` mention of "Vemmon" — exercises
/// the multi-field text scan (effect / inherited / security all count).
fn vemmon_security_text_digimon(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.security_text = "You may play 1 [Vemmon] from your trash.".to_string();
    c
}

/// Lv.4 Digimon whose `effect_text` contains "Vemmon" — the digivolve TARGET
/// used for clause 1's cost-reduction positive branch.
fn vemmon_text_evo_target(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(3000);
    c.play_cost = 5;
    c.colors = vec![CardColor::Black];
    c.effect_text = "[On Play] Place 1 [Vemmon] from your trash under this Digimon.".to_string();
    c.evo_costs = vec![EvoCost {
        level: 3,
        card_color: 5, // Black (mirrors action::mask::evo_color / cards.json card_colors: [5])
        memory_cost: 2,
    }];
    c
}

/// Lv.4 Digimon with NO "Vemmon" text anywhere — the negative-branch
/// digivolve target for clause 1.
fn non_vemmon_evo_target(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(3000);
    c.play_cost = 5;
    c.colors = vec![CardColor::Black];
    c.evo_costs = vec![EvoCost {
        level: 3,
        card_color: 5,
        memory_cost: 2,
    }];
    c
}

fn base() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT21-056 YAML parses and compiles")
        .add_card(filler("FILL"))
        .add_card(vemmon_text_digimon("VEM-TXT-DIGI"))
        .add_card(vemmon_text_digi_egg("VEM-TXT-EGG"))
        .add_card(vemmon_security_text_digimon("VEM-SEC-TXT"))
        .deck(0, &["FILL"; 10])
        .deck(1, &["FILL"; 10])
}

fn seed_trash(runner: &mut DebugRunner, player: u8, card_id: &str) {
    let idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .unwrap();
    let iid = runner.game.next_card_index();
    runner.game.players[player as usize]
        .trash
        .push(CardSource::new(idx, player, iid));
}

// ─────────────────────────────────────────────────────────────────────────
// SECTION 1 — Structural assertions
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn bt21_056_yaml_parses_and_compiles() {
    let _runner = base().start();
}

#[test]
fn bt21_056_is_digimon_lv3_cost3_black() {
    let runner = base().start();
    let compiled = runner
        .compiled_card(CARD_ID)
        .expect("BT21-056 compiled card present");

    assert_eq!(
        compiled.kind,
        digimon_dsl::compiled::CompiledCardKind::Digimon
    );
    assert_eq!(compiled.cost, Some(3));
    assert_eq!(compiled.dp, Some(1000));
    assert_eq!(compiled.level, Some(3));
    assert_eq!(compiled.name, "Vemmon");
}

/// Exactly 2 compiled clauses: clause 0 (On Play trash→recover, Triggered)
/// and clause 1 (cost_reduction, Declarative).
#[test]
fn bt21_056_has_two_clauses() {
    let runner = base().start();
    let compiled = runner.compiled_card(CARD_ID).expect("present");

    assert_eq!(
        compiled.effects.len(),
        2,
        "BT21-056 must have exactly 2 compiled clauses; got {}",
        compiled.effects.len()
    );
}

#[test]
fn bt21_056_on_play_clause_is_triggered_on_play() {
    let runner = base().start();
    let compiled = runner.compiled_card(CARD_ID).expect("present");

    let triggered = compiled
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .expect("BT21-056 must have a triggered On Play clause");
    assert!(
        triggered.when.contains(&CompiledTiming::OnPlay),
        "clause 0 must fire at OnPlay; got {:?}",
        triggered.when
    );
    assert_eq!(
        triggered.scope,
        CompiledScope::FaceUp,
        "On Play clause is a face-up (non-inherited) effect"
    );
}

/// Clause 0 has no clause-level `optional: true` — DCGO's outer
/// `ActivateClass` install is unconditional (`isOptional: false`); the two
/// inner selects each carry their own independent optionality.
#[test]
fn bt21_056_on_play_clause_not_optional_at_clause_level() {
    let runner = base().start();
    let compiled = runner.compiled_card(CARD_ID).expect("present");

    let triggered = compiled
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .expect("triggered clause present");
    assert!(
        !triggered.optional,
        "clause 0 has no clause-level optional flag; \
         optionality lives on the two inner selects"
    );
}

#[test]
fn bt21_056_inherited_cost_reduction_clause_present() {
    let runner = base().start();
    let compiled = runner.compiled_card(CARD_ID).expect("present");

    let found = compiled.effects.iter().any(|c| {
        matches!(
            c,
            CompiledClause::Declarative(CompiledDeclarativeClause::CostReduction {
                once_per_turn: true,
                amount: Some(1),
                ..
            })
        )
    });
    assert!(
        found,
        "BT21-056 must have a cost_reduction clause with once_per_turn=true, amount=1"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// SECTION 2 — Condition gating: On Play hand-pick filter (positive/negative)
// ─────────────────────────────────────────────────────────────────────────

/// POSITIVE: hand contains a card with "Vemmon" in its text — the trash-cost
/// prompt installs and its candidate set is restricted to that card.
#[test]
fn bt21_056_on_play_offers_hand_pick_when_vemmon_text_card_present() {
    let mut runner = base()
        .hand(0, &[CARD_ID, "VEM-TXT-DIGI", "FILL"])
        .memory(10)
        .start();
    runner.play(0, 0);

    let view = runner
        .pending_selection_view()
        .expect("On Play installs the hand-pick cost prompt");
    assert_eq!(view.kind, SelectionKind::Hand);
    assert!(
        runner.pending_is_optional(),
        "the trash cost is declinable (By X-ing... you may)"
    );
}

/// NEGATIVE: hand has zero cards with "Vemmon" in their text — the select
/// no-ops silently (zero candidates), so no pending_selection installs.
#[test]
fn bt21_056_on_play_no_selection_when_no_vemmon_text_card_in_hand() {
    let mut runner = base().hand(0, &[CARD_ID, "FILL"]).memory(10).start();
    runner.play(0, 0);

    assert!(
        runner.pending_selection().is_none(),
        "no [Vemmon]-text card in hand -> the select degrades to a silent no-op"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// SECTION 3 — Behavioral outcome per branch
// ─────────────────────────────────────────────────────────────────────────

/// Paying the cost (trash the [Vemmon]-text hand card) then accepting the
/// recover offers exactly that just-trashed card as the (only) legal target
/// and returns it to hand. No other trash card is seeded, so the recover
/// candidate set is unambiguous (§11.4: avoid multi-candidate prompts when
/// asserting a specific branch — here the constraint is structural, not
/// index-based).
#[test]
fn bt21_056_pay_cost_then_recover_vemmon_text_digimon_from_trash() {
    let mut runner = base()
        .hand(0, &[CARD_ID, "VEM-TXT-DIGI", "FILL"])
        .memory(10)
        .start();

    runner.play(0, 0); // BT21-056 leaves hand

    // Step 1: pay the cost — trash the [Vemmon]-text hand card (VEM-TXT-DIGI).
    let view = runner
        .pending_selection_view()
        .expect("hand-pick cost prompt installs");
    assert_eq!(view.kind, SelectionKind::Hand);
    let (player, action_id) = (0u8, view.valid_action_ids[0]);
    runner
        .execute_action(player, action_id)
        .expect("pay the trash cost");

    // Step 2: recover — the just-trashed VEM-TXT-DIGI is the only [Vemmon]-text
    // non-Digi-Egg card in trash.
    let view2 = runner
        .pending_selection_view()
        .expect("recover prompt installs after paying the cost");
    assert_eq!(view2.kind, SelectionKind::Trash);
    assert_eq!(
        view2.valid_action_ids.len(),
        1,
        "the just-trashed VEM-TXT-DIGI is the only legal recover target"
    );
    runner
        .execute_action(0, view2.valid_action_ids[0])
        .expect("recover VEM-TXT-DIGI");
    runner.auto_resolve().expect("resolve remaining effect");

    let hand_ids: Vec<String> = runner.game.players[0]
        .hand
        .iter()
        .map(|c| c.card_id(&runner.game.card_data).to_string())
        .collect();
    assert!(
        hand_ids.iter().any(|id| id == "VEM-TXT-DIGI"),
        "the [Vemmon]-text card is returned from trash to hand; hand={hand_ids:?}"
    );
    assert_eq!(
        runner.trash_size(0),
        0,
        "the recovered card left the trash; nothing else was seeded there"
    );
}

/// A DIFFERENT [Vemmon]-text card, seeded in trash before the play (not the
/// paid-cost card), is also offered and recoverable — the recover pool is
/// not limited to the just-trashed card.
#[test]
fn bt21_056_pay_cost_then_recover_a_different_pre_seeded_vemmon_text_card() {
    let mut runner = base()
        .hand(0, &[CARD_ID, "VEM-TXT-DIGI"])
        .memory(10)
        .start();
    seed_trash(&mut runner, 0, "VEM-SEC-TXT");

    runner.play(0, 0);
    let view = runner
        .pending_selection_view()
        .expect("hand-pick cost prompt installs");
    runner
        .execute_action(0, view.valid_action_ids[0])
        .expect("pay the trash cost (VEM-TXT-DIGI)");

    // Now trash holds both VEM-TXT-DIGI (paid cost) and VEM-SEC-TXT (pre-seeded).
    let view2 = runner
        .pending_selection_view()
        .expect("recover prompt installs");
    assert_eq!(view2.kind, SelectionKind::Trash);
    assert_eq!(
        view2.valid_action_ids.len(),
        2,
        "both the paid-cost card and the pre-seeded trash card are legal recover targets"
    );
    // Auto-resolve is safe here: the test only asserts SOME [Vemmon]-text
    // card was recovered, not which specific one (that's the prior test).
    runner.auto_resolve().expect("resolve the recover pick");

    assert_eq!(
        runner.trash_size(0),
        1,
        "exactly one of the two candidates was recovered, one remains in trash"
    );
}

/// Declining the recover pick (even after paying the cost) leaves the trash
/// card in place — the return half is separately optional.
#[test]
fn bt21_056_pay_cost_then_decline_recover_leaves_trash_untouched() {
    let mut runner = base()
        .hand(0, &[CARD_ID, "VEM-TXT-DIGI", "FILL"])
        .memory(10)
        .start();
    seed_trash(&mut runner, 0, "VEM-SEC-TXT");

    runner.play(0, 0);

    let view = runner
        .pending_selection_view()
        .expect("hand-pick cost prompt installs");
    runner
        .execute_action(0, view.valid_action_ids[0])
        .expect("pay the trash cost");

    let view2 = runner
        .pending_selection_view()
        .expect("recover prompt installs");
    assert!(
        runner.pending_is_optional(),
        "the recover pick is independently declinable"
    );
    assert_eq!(view2.kind, SelectionKind::Trash);
    runner
        .execute_action(0, PASS)
        .expect("decline the recover pick");
    runner.auto_resolve().ok();

    assert_eq!(
        runner.trash_size(0),
        2,
        "declining the recover: the paid-cost card AND the seeded trash card both remain in trash"
    );
    let hand_ids: Vec<String> = runner.game.players[0]
        .hand
        .iter()
        .map(|c| c.card_id(&runner.game.card_data).to_string())
        .collect();
    assert!(
        !hand_ids.iter().any(|id| id == "VEM-SEC-TXT"),
        "declining the recover leaves the trash card in trash, not hand"
    );
}

/// Declining the cost entirely (PASS at the hand-pick prompt) means nothing
/// is trashed and no recover prompt ever installs.
#[test]
fn bt21_056_decline_cost_aborts_whole_clause() {
    let mut runner = base()
        .hand(0, &[CARD_ID, "VEM-TXT-DIGI", "FILL"])
        .memory(10)
        .start();
    seed_trash(&mut runner, 0, "VEM-SEC-TXT");

    runner.play(0, 0);
    let hand_before = runner.hand_size(0);
    let trash_before = runner.trash_size(0);

    runner
        .execute_action(0, PASS)
        .expect("decline the trash cost");

    assert!(
        runner.pending_selection().is_none(),
        "declining the cost aborts the whole clause -- no recover prompt installs"
    );
    assert_eq!(
        runner.hand_size(0),
        hand_before,
        "no hand card was trashed"
    );
    assert_eq!(
        runner.trash_size(0),
        trash_before,
        "trash is unchanged -- nothing was recovered"
    );
}

/// The hand-pick candidate set is restricted to [Vemmon]-text cards only —
/// a filler card in hand is never offered as the trash cost.
#[test]
fn bt21_056_hand_pick_filter_excludes_non_vemmon_text_cards() {
    let mut runner = base()
        .hand(0, &[CARD_ID, "VEM-TXT-DIGI", "FILL"])
        .memory(10)
        .start();
    runner.play(0, 0);

    let view = runner
        .pending_selection_view()
        .expect("hand-pick prompt installs");
    assert_eq!(
        view.valid_action_ids.len(),
        1,
        "only the [Vemmon]-text card is a legal cost pick; FILL is excluded"
    );
}

/// The trash-recover candidate set excludes Digi-Egg cards even when they
/// have "[Vemmon]" in their text — "1 non-Digi-Egg card with [Vemmon]".
/// The Digi-Egg itself is used as the paid COST (the hand-pick filter has no
/// Digi-Egg exclusion), so after paying, the ONLY trash card is that
/// Digi-Egg — isolating the recover filter's exclusion cleanly.
#[test]
fn bt21_056_recover_filter_excludes_digi_egg_with_vemmon_text() {
    let mut runner = base()
        .hand(0, &[CARD_ID, "VEM-TXT-EGG", "FILL"])
        .memory(10)
        .start();

    runner.play(0, 0);
    let view = runner
        .pending_selection_view()
        .expect("hand-pick prompt installs");
    assert_eq!(
        view.valid_action_ids.len(),
        1,
        "the Digi-Egg is a legal cost pick (no Digi-Egg exclusion on the hand half)"
    );
    runner
        .execute_action(0, view.valid_action_ids[0])
        .expect("pay the trash cost (VEM-TXT-EGG)");

    // The only trash card is the just-trashed Digi-Egg with Vemmon text —
    // the recover select must have zero candidates and no-op (no
    // pending_selection).
    assert!(
        runner.pending_selection().is_none(),
        "a Digi-Egg with [Vemmon] text is not a legal recover target -> select no-ops"
    );
    let hand_ids: Vec<String> = runner.game.players[0]
        .hand
        .iter()
        .map(|c| c.card_id(&runner.game.card_data).to_string())
        .collect();
    assert!(
        !hand_ids.iter().any(|id| id == "VEM-TXT-EGG"),
        "the Digi-Egg must not be recovered to hand"
    );
    assert_eq!(
        runner.trash_size(0),
        1,
        "the Digi-Egg remains in trash -- it was never recovered"
    );
}

/// The recover filter admits a card whose ONLY "Vemmon" mention is in
/// `security_text` (not just `effect_text`) — the DSL predicate scans all
/// three printed-text fields. VEM-SEC-TXT itself is used as the paid cost so
/// the post-cost trash contains exactly that one card, isolating the field
/// match.
#[test]
fn bt21_056_recover_filter_matches_security_text_field() {
    let mut runner = base()
        .hand(0, &[CARD_ID, "VEM-SEC-TXT", "FILL"])
        .memory(10)
        .start();

    runner.play(0, 0);
    let view = runner
        .pending_selection_view()
        .expect("hand-pick prompt installs");
    runner
        .execute_action(0, view.valid_action_ids[0])
        .expect("pay the trash cost (VEM-SEC-TXT)");

    let view2 = runner
        .pending_selection_view()
        .expect("recover prompt installs -- security_text match counts");
    assert_eq!(view2.kind, SelectionKind::Trash);
    assert_eq!(
        view2.valid_action_ids.len(),
        1,
        "exactly the security_text-Vemmon card is offered"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// SECTION 4 — OPT lockout (clause 1: cost reduction)
// ─────────────────────────────────────────────────────────────────────────

/// POSITIVE: digivolving into a [Vemmon]-text Digimon card reduces the
/// digivolution cost by 1 (once per turn).
#[test]
fn bt21_056_cost_reduction_fires_for_vemmon_text_target() {
    let mut runner = base()
        .add_card(vemmon_text_evo_target("VEM-TARGET"))
        .memory(10)
        .start();
    runner.game.turn_count = 1;
    runner.game.turn_player_idx = 0;

    let vemmon = runner.place_on_field(0, CARD_ID, Some(0));
    {
        let data_idx = runner
            .game
            .card_data
            .iter()
            .position(|c| c.card_id == "VEM-TARGET")
            .unwrap();
        let card_index = runner.game.next_card_index();
        runner.game.players[0]
            .hand
            .push(CardSource::new(data_idx, 0, card_index));
    }
    let hand_idx = runner.game.player(0).hand.len() - 1;

    let memory_before = runner.game.memory;
    let digivolved =
        runner
            .game
            .digivolve_from_hand(0, hand_idx, vemmon.index as usize, PlaySource::ByHand);
    assert!(
        digivolved,
        "BT21-056 must digivolve into VEM-TARGET (printed cost 2 - 1 = 1)"
    );
    assert_eq!(
        runner.game.memory,
        memory_before - 1,
        "digivolution cost 2 reduced by 1 -> only 1 memory paid"
    );
}

/// NEGATIVE: digivolving into a Digimon with NO "Vemmon" text does not
/// trigger the reduction — full printed cost is paid.
#[test]
fn bt21_056_cost_reduction_does_not_fire_for_non_vemmon_target() {
    let mut runner = base()
        .add_card(non_vemmon_evo_target("PLAIN-TARGET"))
        .memory(10)
        .start();
    runner.game.turn_count = 1;
    runner.game.turn_player_idx = 0;

    let vemmon = runner.place_on_field(0, CARD_ID, Some(0));
    {
        let data_idx = runner
            .game
            .card_data
            .iter()
            .position(|c| c.card_id == "PLAIN-TARGET")
            .unwrap();
        let card_index = runner.game.next_card_index();
        runner.game.players[0]
            .hand
            .push(CardSource::new(data_idx, 0, card_index));
    }
    let hand_idx = runner.game.player(0).hand.len() - 1;

    let memory_before = runner.game.memory;
    let digivolved =
        runner
            .game
            .digivolve_from_hand(0, hand_idx, vemmon.index as usize, PlaySource::ByHand);
    assert!(digivolved, "digivolution must still succeed at full cost");
    assert_eq!(
        runner.game.memory,
        memory_before - 2,
        "no [Vemmon] text on target -> full printed cost 2 is paid"
    );
}

/// NEGATIVE: the reduction only fires on the OWNER's turn.
#[test]
fn bt21_056_cost_reduction_does_not_fire_on_opponent_turn() {
    let mut runner = base()
        .add_card(vemmon_text_evo_target("VEM-TARGET"))
        .memory(10)
        .start();
    runner.game.turn_count = 1;
    // Opponent's turn from player 0's perspective.
    runner.game.turn_player_idx = 1;

    let vemmon = runner.place_on_field(0, CARD_ID, Some(0));
    {
        let data_idx = runner
            .game
            .card_data
            .iter()
            .position(|c| c.card_id == "VEM-TARGET")
            .unwrap();
        let card_index = runner.game.next_card_index();
        runner.game.players[0]
            .hand
            .push(CardSource::new(data_idx, 0, card_index));
    }
    let hand_idx = runner.game.player(0).hand.len() - 1;

    let memory_before = runner.game.memory;
    let digivolved =
        runner
            .game
            .digivolve_from_hand(0, hand_idx, vemmon.index as usize, PlaySource::ByHand);
    assert!(digivolved, "digivolution must still be allowed (Main Phase check is separate)");
    assert_eq!(
        runner.game.memory,
        memory_before - 2,
        "not owner's turn -> no reduction, full printed cost 2 is paid"
    );
}

/// OPT lockout: a second qualifying digivolve in the SAME turn does not
/// receive a second reduction.
#[test]
fn bt21_056_cost_reduction_opt_locks_second_digivolve_same_turn() {
    let mut runner = base()
        .add_card(vemmon_text_evo_target("VEM-TARGET-1"))
        .add_card(vemmon_text_evo_target("VEM-TARGET-2"))
        .memory(10)
        .start();
    runner.game.turn_count = 1;
    runner.game.turn_player_idx = 0;

    let vemmon1 = runner.place_on_field(0, CARD_ID, Some(0));
    let vemmon2 = runner.place_on_field(0, CARD_ID, Some(0));

    let hand_idx_of = |runner: &mut DebugRunner, card_id: &str| -> usize {
        let data_idx = runner
            .game
            .card_data
            .iter()
            .position(|c| c.card_id == card_id)
            .unwrap();
        let card_index = runner.game.next_card_index();
        runner.game.players[0]
            .hand
            .push(CardSource::new(data_idx, 0, card_index));
        runner.game.player(0).hand.len() - 1
    };

    // First digivolve: reduction fires, consumes the OPT slot.
    let hand_idx_1 = hand_idx_of(&mut runner, "VEM-TARGET-1");
    let memory_before_1 = runner.game.memory;
    let digivolved_1 = runner.game.digivolve_from_hand(
        0,
        hand_idx_1,
        vemmon1.index as usize,
        PlaySource::ByHand,
    );
    assert!(digivolved_1);
    assert_eq!(
        runner.game.memory,
        memory_before_1 - 1,
        "first digivolve this turn: reduction fires"
    );

    // Second digivolve (different permanent, same turn): OPT is a
    // per-card-copy lockout (DCGO OPT hash is per source-card instance via
    // `activateClass2` on that specific `card`), so VEMMON2 -- a DIFFERENT
    // BT21-056 copy -- still has its own OPT slot available. Assert the
    // reduction fires again for the second copy (not a shared cross-copy
    // lock), matching DCGO's per-`card` (per CardSource instance) hash.
    let hand_idx_2 = hand_idx_of(&mut runner, "VEM-TARGET-2");
    let memory_before_2 = runner.game.memory;
    let digivolved_2 = runner.game.digivolve_from_hand(
        0,
        hand_idx_2,
        vemmon2.index as usize,
        PlaySource::ByHand,
    );
    assert!(digivolved_2);
    assert_eq!(
        runner.game.memory,
        memory_before_2 - 1,
        "a second, DIFFERENT BT21-056 copy has its own independent OPT slot"
    );
}
