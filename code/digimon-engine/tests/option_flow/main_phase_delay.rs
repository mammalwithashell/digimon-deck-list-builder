//! PUPPETS-G009 — Standard `<Delay>` `[Main]`-phase activation action.
//!
//! Per the printed `<Delay>` reminder text and RULES_CONTEXT 16-16:
//! "By trashing this card after the placing turn, activate the effect below."
//! Processing is optional (16-16-2) and cannot happen the turn the card was
//! placed (16-16-3).
//!
//! A standard `<Delay>` Option that has been placed on the battle area is NOT
//! an auto-firing scheduled effect. Its controller chooses, on any later main
//! phase, whether to trash it (as the activation cost) to run the stored
//! `<Delay>` body. These tests cover the player-visible `[Main]`-phase action:
//!
//! - the same placing turn exposes NO Delay activation,
//! - a later own main phase exposes a `FIELD_EFFECT` activation bit while PASS
//!   stays legal,
//! - activating trashes the Option as cost and runs the `<Delay>` body,
//! - declining (PASS) leaves the Option on the battle area for a later turn.

use std::sync::{Arc, Mutex};

use digimon_engine::action::mask::build_action_mask;
use digimon_engine::action::space::{
    EFFECTS_PER_PERMANENT, FIELD_EFFECT_SLOT_FOR_MAIN, FIELD_EFFECT_START, PASS, REPLACEMENT_ACCEPT,
};
use digimon_engine::card_source::CardHandle;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::effect::{CardEffect, Effect};
use digimon_engine::enums::{CardColor, CardKind, DelayTrigger, PlayerId};
use digimon_engine::permanent::OptionState;
use digimon_engine::replacement::ReplacementCause;
use digimon_engine::selection::SelectionKind;

// ─── Inline test-effect helpers ──────────────────────────────────────

/// Standard `<Delay>` body — gains 2 memory when activated. Uses the new
/// `DelayTrigger::MainPhaseActivated` lifecycle: the body never auto-fires;
/// it runs only when the controller takes the `[Main]`-phase activation.
struct StandardDelayGain2(Arc<Mutex<u32>>);
impl CardEffect for StandardDelayGain2 {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        let witness = self.0.clone();
        vec![Effect::on_play(card)
            .name("Standard Delay gain 2")
            .delay(DelayTrigger::MainPhaseActivated)
            .process(move |ctx| {
                *witness.lock().unwrap() += 1;
                ctx.gain_memory(2);
            })
            .build()]
    }
}

/// Standard `<Delay>` body that ALSO carries an optional
/// `WhenWouldBeDeleted` replacement which parks a player selection whenever
/// this card would be deleted as a `Cost` — i.e. when its own `[Main]`-phase
/// activation trashes it. The replacement process sets no outcome, so the
/// original cost trash proceeds once the selection resolves (mirrors
/// `replacement_integration.rs::PromptThenAllowDelayCost`). Used to drive the
/// `activate_delayed_option_main` post-delete parked-selection path.
struct StandardDelayWithCostReplacement(Arc<Mutex<u32>>);
impl CardEffect for StandardDelayWithCostReplacement {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        let witness = self.0.clone();
        vec![
            Effect::on_play(card)
                .name("Standard Delay gain 2 (with cost replacement)")
                .delay(DelayTrigger::MainPhaseActivated)
                .process(move |ctx| {
                    *witness.lock().unwrap() += 1;
                    ctx.gain_memory(2);
                })
                .build(),
            Effect::when_would_be_deleted(card)
                .name("Prompt on Cost-cause self-trash")
                .optional()
                .replacement_condition(|ctx, _subject| {
                    matches!(ctx.replacement_cause(), Some(ReplacementCause::Cost))
                })
                // No outcome — the original cost trash proceeds on accept or
                // decline. The window is purely a parked-selection probe.
                .replacement_process(|_| {})
                .build(),
        ]
    }
}

fn option_card(card_id: &str, color: CardColor) -> digimon_engine::CardData {
    let mut cd = make_test_card(card_id, card_id);
    cd.card_kind = CardKind::Option;
    cd.level = None;
    cd.dp = None;
    cd.play_cost = 0;
    cd.colors = vec![color];
    cd
}

fn digimon_card(card_id: &str, color: CardColor) -> digimon_engine::CardData {
    let mut cd = make_test_card(card_id, card_id);
    cd.colors = vec![color];
    cd
}

/// Field-effect activation bit for the `[Main]` activated ability of the
/// permanent at `field_index`.
fn field_main_bit(field_index: usize) -> usize {
    (FIELD_EFFECT_START
        + field_index as u16 * EFFECTS_PER_PERMANENT
        + FIELD_EFFECT_SLOT_FOR_MAIN) as usize
}

/// Park a standard `<Delay>` Option in `player`'s battle area as if it had
/// been placed on `placed_turn`. Returns the field index of the parked Option.
fn park_delayed_option(
    r: &mut DebugRunner,
    player: PlayerId,
    card_id: &str,
    placed_turn: u16,
) -> usize {
    let handle = r.place_on_field(player, card_id, Some(0));
    r.game.player_mut(player).battle_area[handle.index as usize].option_state =
        OptionState::Delayed {
            owner: player,
            // MainPhaseActivated never schedules a turn-keyed auto-trash.
            trash_on_turn: u16::MAX,
            trigger: DelayTrigger::MainPhaseActivated,
            placed_on_turn: placed_turn,
        };
    handle.index as usize
}

// ─── Tests ─────────────────────────────────────────────────────────────

/// On the same turn the standard `<Delay>` Option was placed, no activation
/// action is legal (RULES_CONTEXT 16-16-3).
#[test]
fn standard_delay_not_activatable_on_placing_turn() {
    let mut r = DebugRunner::builder()
        .add_card(option_card("DELAY-MAIN", CardColor::Red))
        .add_card(digimon_card("RED-MATCH", CardColor::Red))
        .memory(0)
        .start();
    r.register_effect(
        "DELAY-MAIN",
        Arc::new(StandardDelayGain2(Arc::new(Mutex::new(0)))),
    );
    r.place_on_field(0, "RED-MATCH", Some(0));
    r.game.enter_main_phase();

    // Park the Option as placed THIS turn.
    let placing_turn = r.game.turn_count;
    let idx = park_delayed_option(&mut r, 0, "DELAY-MAIN", placing_turn);

    let mask = build_action_mask(&r.game, 0);
    assert_eq!(
        mask[field_main_bit(idx)],
        0.0,
        "Delay activation must NOT be legal on the placing turn (16-16-3)"
    );
    assert_eq!(mask[PASS as usize], 1.0, "PASS stays legal");
}

/// After the placing turn, the controller's main phase exposes a field-effect
/// activation bit for the parked standard `<Delay>` Option while PASS stays
/// legal — the choice is genuinely player-visible (no-approximations §17).
#[test]
fn standard_delay_main_phase_action_visible_after_placing_turn() {
    let mut r = DebugRunner::builder()
        .add_card(option_card("DELAY-MAIN", CardColor::Red))
        .add_card(digimon_card("RED-MATCH", CardColor::Red))
        .add_card(digimon_card("FILLER", CardColor::Red))
        .deck(0, &["FILLER"; 6])
        .deck(1, &["FILLER"; 6])
        .memory(0)
        .start();
    r.register_effect(
        "DELAY-MAIN",
        Arc::new(StandardDelayGain2(Arc::new(Mutex::new(0)))),
    );
    r.place_on_field(0, "RED-MATCH", Some(0));

    // Pretend the Option was placed on a previous turn.
    let placing_turn = r.game.turn_count;
    let idx = park_delayed_option(&mut r, 0, "DELAY-MAIN", placing_turn);

    // Advance to the controller's next turn (turn rotation: 0 → 1 → 0).
    r.game.enter_main_phase();
    r.end_turn();
    r.game.enter_main_phase();
    r.end_turn();
    assert_eq!(r.game.turn_player(), 0, "back on the Option owner's turn");
    assert!(r.game.turn_count > placing_turn, "placing turn has passed");
    r.game.enter_main_phase();

    let mask = build_action_mask(&r.game, 0);
    assert_eq!(
        mask[field_main_bit(idx)],
        1.0,
        "Delay activation must be a legal [Main]-phase action after the placing turn"
    );
    assert_eq!(
        mask[PASS as usize], 1.0,
        "PASS stays legal — activating a Delay is optional (16-16-2)"
    );
}

/// Taking the `[Main]`-phase Delay action trashes the Option as the cost and
/// runs the stored `<Delay>` body.
#[test]
fn activating_standard_delay_trashes_option_and_runs_body() {
    let witness = Arc::new(Mutex::new(0u32));
    let mut r = DebugRunner::builder()
        .add_card(option_card("DELAY-MAIN", CardColor::Red))
        .add_card(digimon_card("RED-MATCH", CardColor::Red))
        .add_card(digimon_card("FILLER", CardColor::Red))
        .deck(0, &["FILLER"; 6])
        .deck(1, &["FILLER"; 6])
        .memory(0)
        .start();
    r.register_effect("DELAY-MAIN", Arc::new(StandardDelayGain2(witness.clone())));
    r.place_on_field(0, "RED-MATCH", Some(0));

    let placing_turn = r.game.turn_count;
    let idx = park_delayed_option(&mut r, 0, "DELAY-MAIN", placing_turn);

    r.game.enter_main_phase();
    r.end_turn();
    r.game.enter_main_phase();
    r.end_turn();
    assert_eq!(r.game.turn_player(), 0);
    r.game.enter_main_phase();
    r.game.set_memory(0);

    let trash_before = r.trash_size(0);
    // Decode the field-effect activation through the public action path.
    r.game.decode_action(field_main_bit(idx) as u16, 0);

    assert_eq!(*witness.lock().unwrap(), 1, "Delay body ran exactly once");
    assert_eq!(r.memory(), 2, "Delay body gained 2 memory");
    assert_eq!(
        r.trash_size(0),
        trash_before + 1,
        "the Option is trashed as the activation cost"
    );
    assert!(
        !r.game
            .player(0)
            .battle_area
            .iter()
            .any(|p| matches!(p.option_state, OptionState::Delayed { .. })),
        "no delayed Option remains after activation"
    );
}

/// Declining the Delay activation (PASS) leaves the standard `<Delay>` Option
/// on the battle area for a later turn — the body does NOT run.
#[test]
fn declining_standard_delay_leaves_option_on_field() {
    let witness = Arc::new(Mutex::new(0u32));
    let mut r = DebugRunner::builder()
        .add_card(option_card("DELAY-MAIN", CardColor::Red))
        .add_card(digimon_card("RED-MATCH", CardColor::Red))
        .add_card(digimon_card("FILLER", CardColor::Red))
        .deck(0, &["FILLER"; 6])
        .deck(1, &["FILLER"; 6])
        .memory(0)
        .start();
    r.register_effect("DELAY-MAIN", Arc::new(StandardDelayGain2(witness.clone())));
    r.place_on_field(0, "RED-MATCH", Some(0));

    let placing_turn = r.game.turn_count;
    let idx = park_delayed_option(&mut r, 0, "DELAY-MAIN", placing_turn);

    r.game.enter_main_phase();
    r.end_turn();
    r.game.enter_main_phase();
    r.end_turn();
    assert_eq!(r.game.turn_player(), 0);
    r.game.enter_main_phase();

    // Sanity: the activation IS available this turn.
    let mask = build_action_mask(&r.game, 0);
    assert_eq!(mask[field_main_bit(idx)], 1.0);

    // The controller declines — passes the turn instead.
    r.game.decode_action(PASS, 0);

    assert_eq!(*witness.lock().unwrap(), 0, "declined: Delay body never ran");
    assert!(
        r.game.player(0).battle_area.iter().any(|p| matches!(
            p.option_state,
            OptionState::Delayed {
                trigger: DelayTrigger::MainPhaseActivated,
                ..
            }
        )),
        "declined Delay Option stays parked on the battle area for a later turn"
    );
    assert_eq!(r.trash_size(0), 0, "declined Delay Option is not trashed");
}

/// A standard `<Delay>` Option never auto-fires at end of turn — only the
/// player-visible `[Main]` action activates it. This is the core no-
/// approximations correction: the old lifecycle auto-fired the body and hid
/// the choice from the action mask.
#[test]
fn standard_delay_does_not_auto_fire_at_end_of_turn() {
    let witness = Arc::new(Mutex::new(0u32));
    let mut r = DebugRunner::builder()
        .add_card(option_card("DELAY-MAIN", CardColor::Red))
        .add_card(digimon_card("RED-MATCH", CardColor::Red))
        .add_card(digimon_card("FILLER", CardColor::Red))
        .deck(0, &["FILLER"; 6])
        .deck(1, &["FILLER"; 6])
        .memory(0)
        .start();
    r.register_effect("DELAY-MAIN", Arc::new(StandardDelayGain2(witness.clone())));
    r.place_on_field(0, "RED-MATCH", Some(0));

    let placing_turn = r.game.turn_count;
    park_delayed_option(&mut r, 0, "DELAY-MAIN", placing_turn);

    // End several turns — the standard Delay must never auto-fire.
    r.game.enter_main_phase();
    r.end_turn();
    r.game.enter_main_phase();
    r.end_turn();
    r.game.enter_main_phase();
    r.end_turn();

    assert_eq!(
        *witness.lock().unwrap(),
        0,
        "standard <Delay> must not auto-fire — activation is a player [Main] action"
    );
    assert!(
        r.game
            .player(0)
            .battle_area
            .iter()
            .any(|p| matches!(p.option_state, OptionState::Delayed { .. })),
        "the Delay Option stays parked until the player activates it"
    );
}

/// PUPPETS-G009 review regression: when the `[Main]`-phase Delay activation's
/// cost trash itself enters a `WhenWouldBeDeleted` replacement window, the
/// activation must park a `Replacement` selection and re-drive its
/// continuation through `resume_pending_delayed_option_lifecycle` — exactly
/// like the sibling turn-scan path. The activation must NOT silently complete
/// while a parked replacement is outstanding.
///
/// Asserts: (a) the `<Delay>` body runs exactly once, (b) the cost trash
/// genuinely parks a `Replacement` selection (the Option is NOT yet trashed),
/// and (c) after resolving that parked selection the lifecycle completes
/// faithfully — the Option is trashed and the body's effect persists.
#[test]
fn activating_standard_delay_re_drives_trash_replacement_window() {
    let witness = Arc::new(Mutex::new(0u32));
    let mut r = DebugRunner::builder()
        .add_card(option_card("DELAY-MAIN", CardColor::Red))
        .add_card(digimon_card("RED-MATCH", CardColor::Red))
        .add_card(digimon_card("FILLER", CardColor::Red))
        .deck(0, &["FILLER"; 6])
        .deck(1, &["FILLER"; 6])
        .memory(0)
        .start();
    r.register_effect(
        "DELAY-MAIN",
        Arc::new(StandardDelayWithCostReplacement(witness.clone())),
    );
    r.place_on_field(0, "RED-MATCH", Some(0));

    let placing_turn = r.game.turn_count;
    let idx = park_delayed_option(&mut r, 0, "DELAY-MAIN", placing_turn);

    // Advance past the placing turn so the activation is legal.
    r.game.enter_main_phase();
    r.end_turn();
    r.game.enter_main_phase();
    r.end_turn();
    assert_eq!(r.game.turn_player(), 0);
    r.game.enter_main_phase();
    r.game.set_memory(0);

    let trash_before = r.trash_size(0);

    // Take the [Main]-phase Delay action through the public action path.
    r.game.decode_action(field_main_bit(idx) as u16, 0);

    // (a) The Delay body ran exactly once.
    assert_eq!(*witness.lock().unwrap(), 1, "Delay body ran exactly once");
    assert_eq!(r.memory(), 2, "Delay body gained 2 memory");

    // (b) The cost trash entered the replacement window: a Replacement
    // selection is parked and the Option is NOT yet in the trash.
    let pending = r
        .game
        .pending_selection
        .as_ref()
        .expect("cost trash must park the WhenWouldBeDeleted replacement selection");
    assert_eq!(
        pending.kind,
        SelectionKind::Replacement,
        "parked selection is the cost-trash replacement window"
    );
    assert!(
        pending.valid_action_ids.contains(&REPLACEMENT_ACCEPT),
        "replacement window exposes the accept action"
    );
    assert_eq!(
        r.trash_size(0),
        trash_before,
        "Option is not trashed while the replacement selection is outstanding"
    );
    assert!(
        r.game
            .player(0)
            .battle_area
            .iter()
            .any(|p| matches!(p.option_state, OptionState::Delayed { .. })),
        "Delay Option still on the battle area until the replacement resolves"
    );

    // (c) Resolve the parked replacement (accept — the process sets no
    // outcome, so the original cost trash proceeds). The lifecycle's
    // deferred continuation must be genuinely re-driven, leaving the engine
    // in a clean state with the Option trashed.
    r.game
        .resolve_selection(0, REPLACEMENT_ACCEPT)
        .expect("resolve the cost-trash replacement window");

    assert!(
        r.game.pending_selection.is_none(),
        "no selection outstanding after the replacement resolves"
    );
    assert_eq!(
        r.trash_size(0),
        trash_before + 1,
        "Option is trashed as the activation cost once the replacement resolves"
    );
    assert!(
        !r.game
            .player(0)
            .battle_area
            .iter()
            .any(|p| matches!(p.option_state, OptionState::Delayed { .. })),
        "no delayed Option remains — the deferred continuation was re-driven, not dropped"
    );
    assert_eq!(*witness.lock().unwrap(), 1, "Delay body did not re-run");
    assert_eq!(r.memory(), 2, "memory unchanged after the replacement resolves");
}
