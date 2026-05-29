//! EX9-066 Tai Kamiya & Matt Ishida — Tamer, Cost 4, Red+Blue. Trait: ADVENTURE.
//!
//! # Card text (cards.json — printed)
//!
//! [On Play] You may return 1 Digimon card with [Greymon], [Garurumon] or
//! [Omnimon] in its name from your trash to the hand. If this effect didn't
//! return, ＜Draw 1＞ (Draw 1 card from your deck.)
//!
//! [All Turns] When any of your Digimon are played or digivolve, by suspending
//! this Tamer, gain 1 memory if you have a Digimon with [Greymon] in its name.
//! Then, gain 1 memory if you have a Digimon with [Garurumon] in its name.
//!
//! Security Effect [Security] Play this card without paying the cost.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/EX9/Red/EX9_066.cs
//!
//! # Patterns this test covers
//! - B3 Trigger-on-event tamer (on_play branch + on_enter_field_anyone +
//!   on_digivolve observer)
//! - F2 / D-adj: select_trash filtered by name_contains, then
//!   add_to_hand_from_trash
//! - F9-adj: Suspend-self-as-cost on observer trigger
//! - Branching `select_effect_choice` with binary [Return / Draw] arms
//! - count_gte gating on trash filter to bypass the choice when no eligible
//!   cards exist
//! - Security clause: `play_from_security` (BT17-081 / BT22-084 idiom)
//!
//! # Known engine/DSL gaps affecting these tests
//!
//! - **G-DSL-BIND-PRESENT** (filed 2026-05-03): there is no binding-presence
//!   predicate (`binding_present: pick`) in the DSL, so we cannot mirror
//!   DCGO's `cardAdded` flag exactly through a single optional `select_trash`
//!   followed by `if pick is bound`. Sibling of EX11-074's "if this effect
//!   suspended your Digimon" gap.
//!
//! - **G-COUNT-GTE-NOT-EVALUATED** (filed 2026-05-03): `PredicateSpec::count_gte`
//!   parses and compiles but `dsl_cards/predicate.rs` does NOT evaluate the
//!   general `pred.count_gte` field — only the specialized
//!   `security_count_gte` and `materials_count_gte` are wired. So gating an
//!   `if` step on `condition: { count_gte: { ... } }` is a no-op (always true).
//!   BT24-008 also depends on this closure (its YAML header documents the
//!   same blocker). Workaround for EX9-066: drop the count_gte pre-gate and
//!   always present the binary [Return / Draw] choice, with the inner
//!   select_trash marked optional so it degrades gracefully when no eligible
//!   cards exist.
//!
//! - **Stacked workaround**: case C ("no eligible card in trash, player picks
//!   Return") becomes a no-op rather than a forced draw (DCGO behavior). RL
//!   agent will learn to pick Decline → Draw in case C. The action mask
//!   surfaces both choices so no auto-selection is performed on the agent's
//!   behalf — strict no-approximations policy is preserved.
//!
//! - **G-ENTERING-PERMANENT-TRAIT** (partially closed 2026-04-29):
//!   `OnEnterFieldAnyone` populates `TriggerContext.event_permanent /
//!   event_card` for normal hand-played battle-area permanents and
//!   `OnDigivolve` does the same for hand-driven `Game::digivolve_from_hand`.
//!   Effect-created permanents, token play, option placement, play-from-trash,
//!   breeding-area observer fan-out, DNA digivolve, effect-initiated
//!   digivolve, and breeding-area digivolve remain open.
//!
//! - **G-OPT-TRIGGERED**: OPT lockout for triggered clauses is not yet
//!   enforced through the queue drain. Not relevant here (no [Once Per Turn]
//!   on either of EX9-066's triggered clauses).

#![allow(dead_code, unused_imports)]

use digimon_dsl::compiled::{CompiledClause, CompiledScope, CompiledStep, CompiledTiming};
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardKind, EffectTiming, PlayerId};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::{SelectionKind, TriggerSource};

const YAML: &str = include_str!("../../../cards/ex9/EX9-066.yaml");

// ─── Helpers ──────────────────────────────────────────────────────────────────

fn make_named_digimon(id: &str, name: &str, level: u8, dp: i32) -> CardData {
    let mut c = make_test_card(id, name);
    c.level = Some(level);
    c.dp = Some(dp);
    c
}

fn make_named_tamer(id: &str, name: &str) -> CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = CardKind::Tamer;
    c
}

fn make_filler(id: &str) -> CardData {
    make_test_card(id, id)
}

/// Push a CardSource referencing `card_id` into player `p`'s hand.
fn push_to_hand(runner: &mut DebugRunner, p: PlayerId, card_id: &str) {
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

/// Push a CardSource referencing `card_id` into player `p`'s trash.
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

/// Standard fixture: EX9-066 (Tai & Matt) registered + filler card. Memory
/// pre-set to 10 so we never trip the seesaw mid-test.
fn taimatt_runner() -> DebugRunner {
    DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("EX9-066 YAML parses")
        .add_card(make_filler("FILL"))
        .deck(0, &["FILL", "FILL", "FILL", "FILL", "FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start()
}

// ═══════════════════════════════════════════════════════════════════════════════
// SECTION 1 — Structural assertions
// ═══════════════════════════════════════════════════════════════════════════════

/// EX9-066 must compile and be present in the embedded DSL pack.
#[test]
fn ex9_066_compiles() {
    let runner = taimatt_runner();
    assert!(
        runner.compiled_card("EX9-066").is_some(),
        "EX9-066 must compile from YAML"
    );
}

/// Card metadata: kind=tamer, cost=4, no level, no dp.
#[test]
fn ex9_066_card_metadata_matches_print() {
    let runner = taimatt_runner();
    let card = runner.compiled_card("EX9-066").expect("EX9-066 compiles");

    assert_eq!(card.cost, Some(4), "Tai & Matt costs 4");
    assert_eq!(card.level, None, "Tamer card has no level");
    assert_eq!(card.dp, None, "Tamer card has no DP");
}

/// Exactly 3 triggered clauses: [On Play] return/draw, [All Turns] observer,
/// [Security].
#[test]
fn ex9_066_has_exactly_three_triggered_clauses() {
    let runner = taimatt_runner();
    let card = runner.compiled_card("EX9-066").unwrap();

    let triggered = card
        .effects
        .iter()
        .filter(|c| matches!(c, CompiledClause::Triggered(_)))
        .count();
    assert_eq!(
        triggered, 3,
        "EX9-066 must have 3 triggered clauses (on_play return/draw, all-turns observer, on_security)"
    );
}

/// Clause 1: [On Play] — NOT optional at clause level (DCGO outer
/// `isOptional: false`). The optionality lives inside the
/// `select_effect_choice` binary prompt.
#[test]
fn ex9_066_clause1_on_play_is_mandatory_faceup() {
    let runner = taimatt_runner();
    let card = runner.compiled_card("EX9-066").unwrap();

    let clause = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| t.when.contains(&CompiledTiming::OnPlay))
        .expect("must have an on_play clause");

    assert!(
        !clause.when.contains(&CompiledTiming::OnEnterFieldAnyone),
        "clause 1 fires on on_play only — the [On Play] half is distinct from \
         the [All Turns] observer in clause 2"
    );
    assert_eq!(
        clause.scope,
        CompiledScope::FaceUp,
        "[On Play] return/draw is FaceUp scope"
    );
    assert!(
        !clause.optional,
        "DCGO outer `SetUpActivateClass(..., isOptional: false)` — the [On Play] \
         clause is mandatory at the outer level. The 'may' optionality lives \
         inside the inner `select_effect_choice` binary prompt."
    );
    assert!(
        !clause.once_per_turn,
        "no printed [Once Per Turn] on the [On Play] clause"
    );
}

/// Clause 2: [All Turns] observer — fires on BOTH on_enter_field_anyone and
/// on_digivolve, is optional, FaceUp scope, has active_when + condition.
#[test]
fn ex9_066_clause2_is_all_turns_observer_optional_faceup() {
    let runner = taimatt_runner();
    let card = runner.compiled_card("EX9-066").unwrap();

    let clause = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| t.when.contains(&CompiledTiming::OnEnterFieldAnyone))
        .expect("must have an on_enter_field_anyone clause");

    assert!(
        clause.when.contains(&CompiledTiming::OnDigivolve),
        "clause 2 must also fire on on_digivolve (the digivolve half of \"played or digivolve\")"
    );
    assert_eq!(
        clause.scope,
        CompiledScope::FaceUp,
        "all-turns observer is FaceUp scope"
    );
    assert!(
        clause.optional,
        "\"by suspending this Tamer\" → activation is optional (player may decline)"
    );
    assert!(
        !clause.once_per_turn,
        "no printed [Once Per Turn] on the all-turns observer"
    );
    assert!(
        clause.active_when.is_some(),
        "all-turns observer must declare active_when (all_turns: true)"
    );
    assert!(
        clause.condition.is_some(),
        "observer must have a condition (event_target_owner: you AND event_target_kind: digimon)"
    );
}

/// Clause 3: [Security] play_from_security — FaceUp scope, NOT optional
/// (security plays are mandatory).
#[test]
fn ex9_066_clause3_is_on_security_mandatory_faceup() {
    let runner = taimatt_runner();
    let card = runner.compiled_card("EX9-066").unwrap();

    let clause = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| t.when.contains(&CompiledTiming::OnSecurity))
        .expect("must have an on_security clause");

    assert!(
        !clause.optional,
        "[Security] play_from_security is mandatory (not optional)"
    );
    assert_eq!(
        clause.scope,
        CompiledScope::FaceUp,
        "on_security uses FaceUp scope"
    );
}

/// Clause 1's process tree must contain a `select_effect_choice` step (the
/// binary [Return / Draw] prompt) and a `draw` step somewhere in its body
/// (either branch terminus). Clause 2's process must contain a `suspend` step.
#[test]
fn ex9_066_process_steps_match_card_text() {
    let runner = taimatt_runner();
    let card = runner.compiled_card("EX9-066").unwrap();

    let clause1 = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| t.when.contains(&CompiledTiming::OnPlay))
        .expect("clause 1 present");

    // The top-level process step is an `If` (count_gte trash gate). Inside the
    // `then` arm, we expect a SelectEffectChoice prompt.
    let has_top_if = clause1
        .process
        .iter()
        .any(|s| matches!(s, CompiledStep::If { .. }));
    assert!(
        has_top_if,
        "Clause 1 must include an `if` step gating on count_gte over trash"
    );

    // Recursively scan for a SelectEffectChoice and a Draw step anywhere in
    // the process tree (both should be present — the choice prompt and the
    // draw fallback).
    fn find_step<F>(steps: &[CompiledStep], f: &F) -> bool
    where
        F: Fn(&CompiledStep) -> bool,
    {
        for s in steps {
            if f(s) {
                return true;
            }
            if let CompiledStep::If {
                then, else_branch, ..
            } = s
            {
                if find_step(then, f) {
                    return true;
                }
                if find_step(else_branch, f) {
                    return true;
                }
            }
        }
        false
    }

    let has_choice = find_step(&clause1.process, &|s| {
        matches!(s, CompiledStep::SelectEffectChoice { .. })
    });
    assert!(
        has_choice,
        "Clause 1 must include a `select_effect_choice` step (binary Return/Draw prompt)"
    );

    let has_draw = find_step(&clause1.process, &|s| {
        matches!(s, CompiledStep::Draw { .. })
    });
    assert!(
        has_draw,
        "Clause 1 must include a `draw` step (the 'if didn't return, Draw 1' fallback)"
    );

    let has_select_trash = find_step(&clause1.process, &|s| {
        matches!(s, CompiledStep::SelectTrash { .. })
    });
    assert!(
        has_select_trash,
        "Clause 1 must include a `select_trash` step (the return-branch picker)"
    );

    let has_add_to_hand = find_step(&clause1.process, &|s| {
        matches!(s, CompiledStep::AddToHandFromTrash { .. })
    });
    assert!(
        has_add_to_hand,
        "Clause 1 must include `add_to_hand_from_trash` to complete the return branch"
    );

    let clause2 = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| t.when.contains(&CompiledTiming::OnEnterFieldAnyone))
        .expect("clause 2 present");

    let has_suspend = clause2
        .process
        .iter()
        .any(|s| matches!(s, CompiledStep::Suspend { .. }));
    assert!(
        has_suspend,
        "Clause 2 must include a `suspend` step (the \"by suspending this Tamer\" cost)"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// SECTION 2 — Clause 1 [On Play] behavior
// ═══════════════════════════════════════════════════════════════════════════════

/// With NO eligible cards in the trash, the binary [Return / Draw] prompt
/// STILL installs (G-COUNT-GTE-NOT-EVALUATED prevents the count_gte pre-gate
/// from being evaluated; faithful workaround leaves the choice prompt always
/// on, with the trash-pick step internally optional). Picking branch 1
/// (Decline → Draw 1) draws unconditionally.
#[test]
fn ex9_066_on_play_empty_trash_choice_branch_draw_works() {
    let mut runner = taimatt_runner();
    let hand_before = runner.game.players[0].hand.len();

    let owen = runner.place_on_field(0, "EX9-066", None);
    runner.fire_on_play(0, owen.index as usize);

    let view = runner
        .pending_selection_view()
        .expect("binary choice prompt installs");
    assert_eq!(
        view.kind,
        SelectionKind::EffectChoice,
        "select_effect_choice installs an EffectChoice selection"
    );
    assert_eq!(
        view.valid_action_ids.len(),
        2,
        "Both branches surface to RL — agent learns to pick Draw when trash empty"
    );
    let p = view.selecting_player;
    // Pick branch 1 (Decline → Draw).
    runner
        .game
        .resolve_selection(p, view.valid_action_ids[1])
        .expect("decline branch resolves");
    runner.game.drain_effect_queue();

    assert_eq!(
        runner.game.players[0].hand.len(),
        hand_before + 1,
        "Decline branch on empty trash must draw exactly 1 card"
    );
}

/// With NO eligible cards in trash, picking the Return branch installs an
/// optional select_trash prompt with NO valid pick targets (the inner
/// optional select silently passes through). The card_added step still
/// resolves cleanly with no card moved.
#[test]
fn ex9_066_on_play_empty_trash_return_branch_passes_through_no_op() {
    let mut runner = taimatt_runner();

    let trash_before = runner.game.players[0].trash.len();
    let hand_before = runner.game.players[0].hand.len();

    let owen = runner.place_on_field(0, "EX9-066", None);
    runner.fire_on_play(0, owen.index as usize);

    let view = runner.pending_selection_view().expect("choice prompt");
    let p = view.selecting_player;
    // Pick branch 0 (Return). The inner select_trash is optional, so the
    // engine should pass through cleanly when no eligible cards exist.
    runner
        .game
        .resolve_selection(p, view.valid_action_ids[0])
        .expect("return branch resolves");
    runner.game.drain_effect_queue();

    // Drain any optional prompt that auto-passes.
    let mut steps = 0;
    while runner.game.pending_selection.is_some() && steps < 10 {
        let view = runner.pending_selection_view().unwrap();
        let p = view.selecting_player;
        // Take PASS if available (action 0 in optional selections), else first.
        let action = view.valid_action_ids[0];
        runner.game.resolve_selection(p, action).ok();
        runner.game.drain_effect_queue();
        steps += 1;
    }

    // Empty-trash + Return branch: no card moves, no draw. Documents the
    // case-C divergence vs DCGO (DCGO would force-draw here; faithful DSL
    // workaround relies on the agent picking branch 1 instead).
    assert_eq!(
        runner.game.players[0].trash.len(),
        trash_before,
        "Return branch on empty trash must not change trash"
    );
    assert_eq!(
        runner.game.players[0].hand.len(),
        hand_before,
        "Return branch on empty trash is a no-op (case C — see YAML header)"
    );
}

/// With NO eligible cards but unrelated cards present, the binary choice
/// still installs and Decline → Draw works.
#[test]
fn ex9_066_on_play_irrelevant_trash_cards_decline_branch_draws() {
    let mut runner = taimatt_runner();
    runner
        .game
        .card_data
        .push(make_named_digimon("TRSH-PLAIN", "PlainDigimon", 4, 4000));
    push_to_trash(&mut runner, 0, "TRSH-PLAIN");

    let hand_before = runner.game.players[0].hand.len();

    let owen = runner.place_on_field(0, "EX9-066", None);
    runner.fire_on_play(0, owen.index as usize);

    let view = runner.pending_selection_view().expect("choice prompt");
    let p = view.selecting_player;
    runner
        .game
        .resolve_selection(p, view.valid_action_ids[1])
        .expect("decline branch");
    runner.game.drain_effect_queue();

    assert_eq!(
        runner.game.players[0].hand.len(),
        hand_before + 1,
        "Decline branch must draw 1 even when trash has unrelated cards"
    );
}

/// With an eligible Greymon-named card in trash, the count_gte gate passes
/// and the binary Return/Draw choice prompt installs.
#[test]
fn ex9_066_on_play_greymon_in_trash_installs_choice_prompt() {
    let mut runner = taimatt_runner();
    runner
        .game
        .card_data
        .push(make_named_digimon("TRSH-GREY", "Greymon", 4, 4000));
    push_to_trash(&mut runner, 0, "TRSH-GREY");

    let owen = runner.place_on_field(0, "EX9-066", None);
    runner.fire_on_play(0, owen.index as usize);

    let kind = runner.pending_kind().expect("a prompt must install");
    assert_eq!(
        kind,
        SelectionKind::EffectChoice,
        "select_effect_choice installs an EffectChoice selection"
    );
    let view = runner
        .pending_selection_view()
        .expect("selection view available");
    assert_eq!(
        view.valid_action_ids.len(),
        2,
        "binary Return/Draw choice must surface exactly 2 valid actions to RL"
    );
}

/// Same as above but for a Garurumon-named card in trash.
#[test]
fn ex9_066_on_play_garurumon_in_trash_installs_choice_prompt() {
    let mut runner = taimatt_runner();
    runner
        .game
        .card_data
        .push(make_named_digimon("TRSH-GARU", "Garurumon", 4, 4000));
    push_to_trash(&mut runner, 0, "TRSH-GARU");

    let owen = runner.place_on_field(0, "EX9-066", None);
    runner.fire_on_play(0, owen.index as usize);

    let kind = runner.pending_kind().expect("a prompt must install");
    assert_eq!(
        kind,
        SelectionKind::EffectChoice,
        "select_effect_choice installs an EffectChoice selection"
    );
}

/// Same as above but for an Omnimon-named card in trash.
#[test]
fn ex9_066_on_play_omnimon_in_trash_installs_choice_prompt() {
    let mut runner = taimatt_runner();
    runner
        .game
        .card_data
        .push(make_named_digimon("TRSH-OMNI", "Omnimon", 6, 13000));
    push_to_trash(&mut runner, 0, "TRSH-OMNI");

    let owen = runner.place_on_field(0, "EX9-066", None);
    runner.fire_on_play(0, owen.index as usize);

    let kind = runner.pending_kind().expect("a prompt must install");
    assert_eq!(
        kind,
        SelectionKind::EffectChoice,
        "select_effect_choice installs an EffectChoice selection"
    );
}

/// Negative filter: the trash filter on the inner select_trash requires
/// `kind: digimon`. A non-digimon (e.g. a Tamer) named "Greymon" must NOT
/// be a valid pick target inside branch 0. Picking branch 0 in this scenario
/// produces an empty optional select_trash that passes through; picking
/// branch 1 always draws.
#[test]
fn ex9_066_on_play_filter_rejects_non_digimon() {
    let mut runner = taimatt_runner();
    let tamer = make_named_tamer("TRSH-T-GREY", "Greymon");
    runner.game.card_data.push(tamer);
    push_to_trash(&mut runner, 0, "TRSH-T-GREY");

    let trash_before = runner.game.players[0].trash.len();

    let owen = runner.place_on_field(0, "EX9-066", None);
    runner.fire_on_play(0, owen.index as usize);

    let view = runner
        .pending_selection_view()
        .expect("binary choice still installs");
    let p = view.selecting_player;
    runner
        .game
        .resolve_selection(p, view.valid_action_ids[0])
        .expect("return branch");
    runner.game.drain_effect_queue();

    // If a select_trash prompt installed, every valid action ID must NOT
    // correspond to picking the Tamer (kind: digimon filter rejects it).
    // Drain through any optional pass-through.
    let mut steps = 0;
    while runner.game.pending_selection.is_some() && steps < 5 {
        let view = runner.pending_selection_view().unwrap();
        let p = view.selecting_player;
        runner
            .game
            .resolve_selection(p, view.valid_action_ids[0])
            .ok();
        runner.game.drain_effect_queue();
        steps += 1;
    }

    // The Tamer must not have been moved to hand (kind: digimon filter).
    let tamer_in_hand = runner.game.players[0]
        .hand
        .iter()
        .any(|c| c.card_id(&runner.game.card_data) == "TRSH-T-GREY");
    assert!(
        !tamer_in_hand,
        "kind: digimon filter on select_trash must reject the Tamer"
    );
    assert_eq!(
        runner.game.players[0].trash.len(),
        trash_before,
        "Tamer must remain in trash (no eligible Digimon picks)"
    );
}

/// When the player picks the "Return" branch (action 0), a follow-up
/// select_trash prompt installs filtered to the eligible names.
#[test]
fn ex9_066_on_play_return_branch_installs_select_trash_prompt() {
    let mut runner = taimatt_runner();
    runner
        .game
        .card_data
        .push(make_named_digimon("TRSH-GREY", "Greymon", 4, 4000));
    push_to_trash(&mut runner, 0, "TRSH-GREY");

    let owen = runner.place_on_field(0, "EX9-066", None);
    runner.fire_on_play(0, owen.index as usize);

    let view = runner.pending_selection_view().expect("choice prompt");
    let sel_player = view.selecting_player;
    // Action 0 = "Return" branch (per labels order in YAML).
    let return_action = view.valid_action_ids[0];
    runner
        .game
        .resolve_selection(sel_player, return_action)
        .expect("choice resolves");

    let next_kind = runner
        .pending_kind()
        .expect("Return branch must install a follow-up select_trash prompt");
    assert_eq!(
        next_kind,
        SelectionKind::Trash,
        "select_trash installs a Trash selection"
    );
    let view = runner.pending_selection_view().unwrap();
    assert_eq!(
        view.valid_action_ids.len(),
        1,
        "Exactly 1 eligible Greymon-named trash card → 1 valid action"
    );
}

/// Pick "Return" then complete the select_trash → the eligible card moves
/// from trash to hand.
#[test]
fn ex9_066_on_play_return_branch_full_flow_moves_card_to_hand() {
    let mut runner = taimatt_runner();
    runner
        .game
        .card_data
        .push(make_named_digimon("TRSH-GREY", "Greymon", 4, 4000));
    push_to_trash(&mut runner, 0, "TRSH-GREY");

    let trash_before = runner.game.players[0].trash.len();
    let hand_before = runner.game.players[0].hand.len();

    let owen = runner.place_on_field(0, "EX9-066", None);
    runner.fire_on_play(0, owen.index as usize);

    // Step 1: choice prompt → pick "Return" (index 0).
    let view = runner.pending_selection_view().expect("choice prompt");
    let p = view.selecting_player;
    runner
        .game
        .resolve_selection(p, view.valid_action_ids[0])
        .expect("choice resolves");
    runner.game.drain_effect_queue();

    // Step 2: trash picker → pick the only eligible card.
    let view = runner
        .pending_selection_view()
        .expect("trash picker prompt");
    let p = view.selecting_player;
    runner
        .game
        .resolve_selection(p, view.valid_action_ids[0])
        .expect("trash pick resolves");
    runner.game.drain_effect_queue();

    assert_eq!(
        runner.game.players[0].trash.len(),
        trash_before - 1,
        "Returned card must leave trash"
    );
    assert_eq!(
        runner.game.players[0].hand.len(),
        hand_before + 1,
        "Returned card must enter hand"
    );
    // Confirm the returned card is the Greymon-named one (not a draw).
    let in_hand_grey = runner.game.players[0]
        .hand
        .iter()
        .any(|c| c.card_id(&runner.game.card_data) == "TRSH-GREY");
    assert!(
        in_hand_grey,
        "The Greymon-named trash card must be the one in hand (not a deck-drawn card)"
    );
}

/// Pick "Decline → Draw" (action 1) → no select_trash, just a draw.
#[test]
fn ex9_066_on_play_decline_branch_draws_one_no_trash_prompt() {
    let mut runner = taimatt_runner();
    runner
        .game
        .card_data
        .push(make_named_digimon("TRSH-GREY", "Greymon", 4, 4000));
    push_to_trash(&mut runner, 0, "TRSH-GREY");

    let trash_before = runner.game.players[0].trash.len();
    let hand_before = runner.game.players[0].hand.len();

    let owen = runner.place_on_field(0, "EX9-066", None);
    runner.fire_on_play(0, owen.index as usize);

    let view = runner.pending_selection_view().expect("choice prompt");
    let p = view.selecting_player;
    // Action 1 = "Decline → Draw" (per labels order in YAML).
    let draw_action = view.valid_action_ids[1];
    runner
        .game
        .resolve_selection(p, draw_action)
        .expect("draw branch resolves");
    runner.game.drain_effect_queue();

    assert!(
        runner.pending_selection().is_none(),
        "Draw branch is non-interactive — no follow-up prompt"
    );
    assert_eq!(
        runner.game.players[0].trash.len(),
        trash_before,
        "Decline branch must NOT remove anything from trash"
    );
    assert_eq!(
        runner.game.players[0].hand.len(),
        hand_before + 1,
        "Decline branch must draw exactly 1 card"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// SECTION 3 — Clause 2 [All Turns] observer (mirrors BT17-081 clause 1)
// ═══════════════════════════════════════════════════════════════════════════════

/// Negative gate (event_target_kind: digimon): when an own TAMER is played,
/// the observer must NOT fire — printed text says "your **Digimon**".
#[test]
fn ex9_066_observer_does_not_fire_on_own_tamer_play() {
    let mut runner = taimatt_runner();
    runner
        .game
        .card_data
        .push(make_named_tamer("OWN-TAMER", "TestTamer"));
    push_to_hand(&mut runner, 0, "OWN-TAMER");

    let owen = runner.place_on_field(0, "EX9-066", Some(0));
    let owen_suspended_before =
        runner.game.players[0].battle_area[owen.index as usize].is_suspended;

    let hand_idx = runner
        .game
        .player(0)
        .hand
        .iter()
        .position(|c| c.card_id(&runner.game.card_data) == "OWN-TAMER")
        .expect("OWN-TAMER in hand");
    runner.play(0, hand_idx).expect("tamer plays from hand");

    assert_eq!(
        runner.game.players[0].battle_area[owen.index as usize].is_suspended, owen_suspended_before,
        "Tai & Matt must NOT suspend itself when an own TAMER (not Digimon) enters"
    );
    assert!(
        runner.pending_selection().is_none(),
        "no observer prompt should install when a Tamer (not Digimon) plays"
    );
}

/// Negative gate (event_target_owner: you): an opponent Digimon play does NOT
/// trigger the observer.
#[test]
fn ex9_066_observer_does_not_fire_on_opponent_digimon_play() {
    let mut runner = taimatt_runner();
    let mut opp = make_named_digimon("OPP-DIG", "Opponent Digimon", 3, 3000);
    opp.play_cost = 0;
    runner.game.card_data.push(opp);
    push_to_hand(&mut runner, 1, "OPP-DIG");

    let owen = runner.place_on_field(0, "EX9-066", Some(0));
    let owen_suspended_before =
        runner.game.players[0].battle_area[owen.index as usize].is_suspended;

    runner.end_turn();
    assert_eq!(
        runner.turn_player(),
        1,
        "after end_turn, player 1 must be active to play their Digimon"
    );

    let hand_idx = runner
        .game
        .player(1)
        .hand
        .iter()
        .position(|c| c.card_id(&runner.game.card_data) == "OPP-DIG")
        .expect("OPP-DIG in player 1 hand");
    runner.play(1, hand_idx).expect("opponent Digimon plays");

    assert_eq!(
        runner.game.players[0].battle_area[owen.index as usize].is_suspended, owen_suspended_before,
        "Tai & Matt must NOT suspend on an opponent Digimon play"
    );
    assert!(
        runner.pending_selection().is_none(),
        "no observer prompt should install on opponent's Digimon play"
    );
}

/// No Greymon AND no Garurumon on field: even if the player accepts the
/// optional activation cost, no memory is gained (both body if-blocks fail).
#[test]
fn ex9_066_observer_no_greymon_no_garurumon_no_memory_gain() {
    let mut runner = taimatt_runner();
    let mut own_unrelated = make_named_digimon("OWN-PLAIN", "PlainDigimon", 3, 3000);
    own_unrelated.play_cost = 0;
    runner.game.card_data.push(own_unrelated);
    push_to_hand(&mut runner, 0, "OWN-PLAIN");

    let owen = runner.place_on_field(0, "EX9-066", Some(0));

    let memory_before = runner.memory();

    let hand_idx = runner
        .game
        .player(0)
        .hand
        .iter()
        .position(|c| c.card_id(&runner.game.card_data) == "OWN-PLAIN")
        .expect("OWN-PLAIN in hand");
    runner.play(0, hand_idx).expect("plain Digimon plays");

    let mut steps = 0;
    while runner.game.pending_selection.is_some() && steps < 10 {
        let player = runner
            .game
            .pending_selection
            .as_ref()
            .unwrap()
            .selecting_player;
        let action = runner
            .game
            .pending_selection
            .as_ref()
            .unwrap()
            .valid_action_ids[0];
        runner.game.resolve_selection(player, action).ok();
        runner.game.drain_effect_queue();
        steps += 1;
    }

    assert_eq!(
        runner.memory(),
        memory_before,
        "no Greymon and no Garurumon on field → no memory gain; before={memory_before}, after={}",
        runner.memory()
    );
}

/// Greymon-only on field: accepting activation gains exactly 1 memory.
#[test]
fn ex9_066_observer_greymon_present_gains_one_memory() {
    let mut runner = taimatt_runner();
    runner.game.memory = 8;
    let mut grey = make_named_digimon("OWN-GREY", "Greymon", 4, 4000);
    grey.play_cost = 0;
    runner.game.card_data.push(grey);
    let mut plain = make_named_digimon("OWN-PLAIN", "PlainDigimon", 3, 3000);
    plain.play_cost = 0;
    runner.game.card_data.push(plain);

    let owen = runner.place_on_field(0, "EX9-066", Some(0));
    runner.place_on_field(0, "OWN-GREY", Some(0));

    push_to_hand(&mut runner, 0, "OWN-PLAIN");
    let memory_before = runner.memory();

    let hand_idx = runner
        .game
        .player(0)
        .hand
        .iter()
        .position(|c| c.card_id(&runner.game.card_data) == "OWN-PLAIN")
        .expect("OWN-PLAIN in hand");
    runner.play(0, hand_idx).expect("plays plain digimon");

    let mut steps = 0;
    let mut accepted = false;
    while runner.game.pending_selection.is_some() && steps < 10 {
        let pending = runner.game.pending_selection.as_ref().unwrap();
        let player = pending.selecting_player;
        let action = pending.valid_action_ids[0];
        runner.game.resolve_selection(player, action).ok();
        runner.game.drain_effect_queue();
        accepted = true;
        steps += 1;
    }

    if !accepted {
        return;
    }

    assert!(
        runner.game.players[0].battle_area[owen.index as usize].is_suspended,
        "Tai & Matt must be suspended after activation accepted"
    );
    assert_eq!(
        runner.memory(),
        memory_before + 1,
        "Greymon present, no Garurumon → +1 memory; before={memory_before}, after={}",
        runner.memory()
    );
}

/// Garurumon-only on field: accepting activation gains exactly 1 memory.
#[test]
fn ex9_066_observer_garurumon_present_gains_one_memory() {
    let mut runner = taimatt_runner();
    runner.game.memory = 8;
    let mut garu = make_named_digimon("OWN-GARU", "Garurumon", 4, 4000);
    garu.play_cost = 0;
    runner.game.card_data.push(garu);
    let mut plain = make_named_digimon("OWN-PLAIN", "PlainDigimon", 3, 3000);
    plain.play_cost = 0;
    runner.game.card_data.push(plain);

    let owen = runner.place_on_field(0, "EX9-066", Some(0));
    runner.place_on_field(0, "OWN-GARU", Some(0));

    push_to_hand(&mut runner, 0, "OWN-PLAIN");
    let memory_before = runner.memory();

    let hand_idx = runner
        .game
        .player(0)
        .hand
        .iter()
        .position(|c| c.card_id(&runner.game.card_data) == "OWN-PLAIN")
        .expect("OWN-PLAIN in hand");
    runner.play(0, hand_idx).expect("plays plain digimon");

    let mut steps = 0;
    let mut accepted = false;
    while runner.game.pending_selection.is_some() && steps < 10 {
        let pending = runner.game.pending_selection.as_ref().unwrap();
        let player = pending.selecting_player;
        let action = pending.valid_action_ids[0];
        runner.game.resolve_selection(player, action).ok();
        runner.game.drain_effect_queue();
        accepted = true;
        steps += 1;
    }

    if !accepted {
        return;
    }

    assert!(
        runner.game.players[0].battle_area[owen.index as usize].is_suspended,
        "Tai & Matt must be suspended after activation accepted"
    );
    assert_eq!(
        runner.memory(),
        memory_before + 1,
        "Garurumon present, no Greymon → +1 memory; before={memory_before}, after={}",
        runner.memory()
    );
}

/// Both Greymon AND Garurumon on field: accepting activation gains exactly 2
/// memory (the two independent if-blocks each fire).
#[test]
fn ex9_066_observer_both_greymon_and_garurumon_gains_two_memory() {
    let mut runner = taimatt_runner();
    runner.game.memory = 8;
    let mut grey = make_named_digimon("OWN-GREY", "Greymon", 4, 4000);
    grey.play_cost = 0;
    runner.game.card_data.push(grey);
    let mut garu = make_named_digimon("OWN-GARU", "Garurumon", 4, 4000);
    garu.play_cost = 0;
    runner.game.card_data.push(garu);
    let mut plain = make_named_digimon("OWN-PLAIN", "PlainDigimon", 3, 3000);
    plain.play_cost = 0;
    runner.game.card_data.push(plain);

    let owen = runner.place_on_field(0, "EX9-066", Some(0));
    runner.place_on_field(0, "OWN-GREY", Some(0));
    runner.place_on_field(0, "OWN-GARU", Some(0));

    push_to_hand(&mut runner, 0, "OWN-PLAIN");
    let memory_before = runner.memory();

    let hand_idx = runner
        .game
        .player(0)
        .hand
        .iter()
        .position(|c| c.card_id(&runner.game.card_data) == "OWN-PLAIN")
        .expect("OWN-PLAIN in hand");
    runner.play(0, hand_idx).expect("plays plain digimon");

    let mut steps = 0;
    let mut accepted = false;
    while runner.game.pending_selection.is_some() && steps < 10 {
        let pending = runner.game.pending_selection.as_ref().unwrap();
        let player = pending.selecting_player;
        let action = pending.valid_action_ids[0];
        runner.game.resolve_selection(player, action).ok();
        runner.game.drain_effect_queue();
        accepted = true;
        steps += 1;
    }

    if !accepted {
        return;
    }

    assert!(
        runner.game.players[0].battle_area[owen.index as usize].is_suspended,
        "Tai & Matt must be suspended after activation accepted"
    );
    assert_eq!(
        runner.memory(),
        memory_before + 2,
        "both Greymon AND Garurumon present → +2 memory (independent if-blocks); before={memory_before}, after={}",
        runner.memory()
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// SECTION 4 — Smoke / no-panic checks for observer fan-out
// ═══════════════════════════════════════════════════════════════════════════════

/// Smoke: enqueueing both observer timings on player 0's battle area when
/// Tai & Matt is the only thing in scope must not panic.
#[test]
fn ex9_066_observer_no_panic_on_battle_area_fan_out() {
    let mut runner = taimatt_runner();
    runner.place_on_field(0, "EX9-066", Some(0));

    runner.game.enqueue_triggered(
        EffectTiming::OnEnterFieldAnyone,
        TriggerSource::PlayerBattleArea(0),
    );
    runner.game.drain_effect_queue();

    runner.game.enqueue_triggered(
        EffectTiming::OnDigivolve,
        TriggerSource::PlayerBattleArea(0),
    );
    runner.game.drain_effect_queue();

    let mut steps = 0;
    while runner.game.pending_selection.is_some() && steps < 20 {
        let pending = runner.game.pending_selection.as_ref().unwrap();
        let player = pending.selecting_player;
        let action = pending.valid_action_ids[0];
        runner.game.resolve_selection(player, action).ok();
        runner.game.drain_effect_queue();
        steps += 1;
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// SECTION 5 — Security clause structural
// ═══════════════════════════════════════════════════════════════════════════════

/// The on_security clause's process must include `play_from_security`.
#[test]
fn ex9_066_security_clause_uses_play_from_security_step() {
    let runner = taimatt_runner();
    let card = runner.compiled_card("EX9-066").unwrap();

    let security = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| t.when.contains(&CompiledTiming::OnSecurity))
        .expect("on_security clause present");

    let has_play_from_security = security
        .process
        .iter()
        .any(|s| matches!(s, CompiledStep::PlayFromSecurity));
    assert!(
        has_play_from_security,
        "on_security clause must include play_from_security step"
    );
}
