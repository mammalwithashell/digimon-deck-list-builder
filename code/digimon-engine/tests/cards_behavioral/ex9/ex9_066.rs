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
use digimon_engine::action::space::PASS;
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
        clause.trigger_if.is_some(),
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
                if find_step(then, f) || find_step(else_branch, f) {
                    return true;
                }
            }
        }
        false
    }

    // "You may return 1 ... If this effect didn't return, <Draw 1>": an optional trash pick, then the return when a
    // card was picked and the draw when none was. No separate "return or draw" choice: DCGO asks only the pick
    // (canNoSelect) and draws when it added nothing, with no prompt at all when the trash holds no eligible card.
    assert!(
        !find_step(&clause1.process, &|s| matches!(s, CompiledStep::SelectEffectChoice { .. })),
        "Clause 1 asks no Return/Draw branch choice: the optional pick is the choice"
    );
    let checks = [
        ("select_trash", find_step(&clause1.process, &|s| matches!(s, CompiledStep::SelectTrash { .. }))),
        ("add_to_hand_from_trash", find_step(&clause1.process, &|s| matches!(s, CompiledStep::AddToHandFromTrash { .. }))),
        ("draw", find_step(&clause1.process, &|s| matches!(s, CompiledStep::Draw { .. }))),
    ];
    for (what, found) in checks {
        assert!(found, "Clause 1 must include `{what}`");
    }

    let clause2 = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| t.when.contains(&CompiledTiming::OnEnterFieldAnyone))
        .expect("clause 2 present");
    assert!(
        clause2.process.iter().any(|s| matches!(s, CompiledStep::Suspend { .. })),
        "Clause 2 must include a `suspend` step (the \"by suspending this Tamer\" cost)"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// SECTION 2 — Clause 1 [On Play] behavior
// ═══════════════════════════════════════════════════════════════════════════════

/// Play EX9-066 for P0 and fire its [On Play].
fn play_taimatt(runner: &mut DebugRunner) {
    let owen = runner.place_on_field(0, "EX9-066", None);
    runner.fire_on_play(0, owen.index as usize);
}

fn hand_has(runner: &DebugRunner, id: &str) -> bool {
    runner.game.players[0].hand.iter().any(|c| c.card_id(&runner.game.card_data) == id)
}

/// Nothing to return (empty trash): no prompt, and "If this effect didn't return, <Draw 1>" draws. DCGO
/// (`EX9_066.cs`) opens no selection when `HasMatchConditionOwnersCardInTrash` is false, then draws. Found by the
/// DCGO playback of Glowing Dawn vs DNA Omnimon (2026-11-12): the engine offered a "return" branch that then did
/// nothing - no draw.
#[test]
fn ex9_066_on_play_nothing_to_return_draws_without_a_prompt() {
    let mut runner = taimatt_runner();
    let hand_before = runner.game.players[0].hand.len();
    play_taimatt(&mut runner);
    assert!(runner.pending_selection_view().is_none(), "no eligible card: no prompt");
    assert_eq!(runner.game.players[0].hand.len(), hand_before + 1, "didn't return -> draw 1");
}

/// Unrelated cards in the trash are not eligible: still no prompt, still the draw.
#[test]
fn ex9_066_on_play_irrelevant_trash_cards_draw_without_a_prompt() {
    let mut runner = taimatt_runner();
    runner.game.card_data.push(make_named_digimon("TRSH-PLAIN", "PlainDigimon", 4, 4000));
    push_to_trash(&mut runner, 0, "TRSH-PLAIN");
    let hand_before = runner.game.players[0].hand.len();
    play_taimatt(&mut runner);
    assert!(runner.pending_selection_view().is_none(), "no eligible card: no prompt");
    assert_eq!(runner.game.players[0].hand.len(), hand_before + 1);
    assert!(!hand_has(&runner, "TRSH-PLAIN"));
}

/// A non-Digimon named "Greymon" is not eligible ("1 Digimon card with [Greymon]..."): it stays, and the effect draws.
#[test]
fn ex9_066_on_play_filter_rejects_non_digimon() {
    let mut runner = taimatt_runner();
    runner.game.card_data.push(make_named_tamer("TRSH-T-GREY", "Greymon"));
    push_to_trash(&mut runner, 0, "TRSH-T-GREY");
    let hand_before = runner.game.players[0].hand.len();
    play_taimatt(&mut runner);
    assert!(runner.pending_selection_view().is_none(), "a Tamer is not eligible: no prompt");
    assert!(!hand_has(&runner, "TRSH-T-GREY"), "the Tamer stays in the trash");
    assert_eq!(runner.game.players[0].hand.len(), hand_before + 1, "nothing returned -> draw 1");
}

/// Each named card makes the optional trash pick appear, with exactly that card to pick.
#[test]
fn ex9_066_on_play_eligible_names_install_an_optional_trash_pick() {
    let cases = [("TRSH-GREY", "Greymon", 4, 4000), ("TRSH-GARU", "Garurumon", 4, 4000), ("TRSH-OMNI", "Omnimon", 6, 13000)];
    for (id, name, lv, dp) in cases {
        let mut runner = taimatt_runner();
        runner.game.card_data.push(make_named_digimon(id, name, lv, dp));
        push_to_trash(&mut runner, 0, id);
        play_taimatt(&mut runner);
        let view = runner.pending_selection_view().unwrap_or_else(|| panic!("{name}: a trash pick installs"));
        assert_eq!(view.kind, SelectionKind::Trash, "{name}: the pick is a trash selection, not a branch choice");
        assert!(view.is_optional, "{name}: \"You may return\" - the pick can be declined");
        let picks = view.valid_action_ids.iter().filter(|&&a| a != PASS).count();
        assert_eq!(picks, 1, "{name}: exactly the one eligible card");
    }
}

/// Returning the card: it moves to the hand and there is no draw.
#[test]
fn ex9_066_on_play_return_moves_the_card_and_does_not_draw() {
    let mut runner = taimatt_runner();
    runner.game.card_data.push(make_named_digimon("TRSH-GREY", "Greymon", 4, 4000));
    push_to_trash(&mut runner, 0, "TRSH-GREY");
    let trash_before = runner.game.players[0].trash.len();
    let hand_before = runner.game.players[0].hand.len();
    let deck_before = runner.game.players[0].deck.len();
    play_taimatt(&mut runner);
    let view = runner.pending_selection_view().expect("trash pick");
    let pick = *view.valid_action_ids.iter().find(|&&a| a != PASS).unwrap();
    runner.game.resolve_selection(view.selecting_player, pick).expect("pick resolves");
    runner.game.drain_effect_queue();
    assert!(runner.pending_selection_view().is_none());
    assert!(hand_has(&runner, "TRSH-GREY"), "the Greymon card is returned");
    assert_eq!(runner.game.players[0].trash.len(), trash_before - 1);
    assert_eq!(runner.game.players[0].hand.len(), hand_before + 1, "returned, so no draw");
    assert_eq!(runner.game.players[0].deck.len(), deck_before, "no card drawn");
}

/// Declining the return: "If this effect didn't return, <Draw 1>" - the card stays and 1 card is drawn.
#[test]
fn ex9_066_on_play_declining_the_return_draws_one() {
    let mut runner = taimatt_runner();
    runner.game.card_data.push(make_named_digimon("TRSH-GREY", "Greymon", 4, 4000));
    push_to_trash(&mut runner, 0, "TRSH-GREY");
    let trash_before = runner.game.players[0].trash.len();
    let hand_before = runner.game.players[0].hand.len();
    play_taimatt(&mut runner);
    let view = runner.pending_selection_view().expect("trash pick");
    assert!(view.is_optional, "the decline (PASS) is offered");
    runner.game.resolve_selection(view.selecting_player, PASS).expect("decline resolves");
    runner.game.drain_effect_queue();
    assert!(runner.pending_selection_view().is_none());
    assert!(!hand_has(&runner, "TRSH-GREY"), "declined: the card stays in the trash");
    assert_eq!(runner.game.players[0].trash.len(), trash_before);
    assert_eq!(runner.game.players[0].hand.len(), hand_before + 1, "didn't return -> draw 1");
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
    let mut grey = make_named_digimon("OWN-GREY", "Greymon", 4, 4000);
    grey.play_cost = 0;
    runner.game.card_data.push(grey);
    let mut plain = make_named_digimon("OWN-PLAIN", "PlainDigimon", 3, 3000);
    plain.play_cost = 0;
    runner.game.card_data.push(plain);

    let owen = runner.place_on_field(0, "EX9-066", Some(0));
    runner.place_on_field(0, "OWN-GREY", Some(0));

    push_to_hand(&mut runner, 0, "OWN-PLAIN");
    // Lower starting memory below the +10 cap so the observer's +1 gain is
    // observable (`fix-outer-optional-prompt-trigger-ctx`).
    runner.game.set_memory(5);
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
    while runner.game.pending_selection.is_some() && steps < 10 {
        let pending = runner.game.pending_selection.as_ref().unwrap();
        let player = pending.selecting_player;
        let action = pending.valid_action_ids[0];
        runner.game.resolve_selection(player, action).ok();
        runner.game.drain_effect_queue();
        steps += 1;
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
    let mut garu = make_named_digimon("OWN-GARU", "Garurumon", 4, 4000);
    garu.play_cost = 0;
    runner.game.card_data.push(garu);
    let mut plain = make_named_digimon("OWN-PLAIN", "PlainDigimon", 3, 3000);
    plain.play_cost = 0;
    runner.game.card_data.push(plain);

    let owen = runner.place_on_field(0, "EX9-066", Some(0));
    runner.place_on_field(0, "OWN-GARU", Some(0));

    push_to_hand(&mut runner, 0, "OWN-PLAIN");
    runner.game.set_memory(5);
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
    while runner.game.pending_selection.is_some() && steps < 10 {
        let pending = runner.game.pending_selection.as_ref().unwrap();
        let player = pending.selecting_player;
        let action = pending.valid_action_ids[0];
        runner.game.resolve_selection(player, action).ok();
        runner.game.drain_effect_queue();
        steps += 1;
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
    // Lower starting memory so the +2 gain stays below the cap.
    runner.game.set_memory(5);
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
    while runner.game.pending_selection.is_some() && steps < 10 {
        let pending = runner.game.pending_selection.as_ref().unwrap();
        let player = pending.selecting_player;
        let action = pending.valid_action_ids[0];
        runner.game.resolve_selection(player, action).ok();
        runner.game.drain_effect_queue();
        steps += 1;
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
