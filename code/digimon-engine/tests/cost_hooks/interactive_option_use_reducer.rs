//! Engine substrate test for the OPTION-USE half of
//! `G-COST-REDUCTION-INTERACTIVE-PAY-COST`.
//!
//! Models BT25-049 Armalizamon / BT25-090 Tomoro Tenma clause:
//!   "[Your Turn] [Once Per Turn] When you would use an Option card with the
//!    [Glowing Dawn] trait, by trashing the bottom face-down card under any of
//!    your Tamers, reduce the cost by N."
//!
//! Same interactive-pay_cost shape as the digivolve half (a parking Tamer
//! pick), but on the Option-use cost path
//! (`game_actions/options.rs::play_option_core` →
//! `scan_before_pay_cost_reduction_with_target(OptionUse)`). The synchronous
//! scan cannot host a parking pay_cost; the engine routes such a reducer
//! through a dedicated pre-scan interactive prompt that re-enters
//! `play_option_core` with the reduction pre-credited.

use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardHandle;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::effect::{CardEffect, Effect};
use digimon_engine::enums::CardKind;
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::{OptionPlayResult, SelectionKind};

// ─── Fixtures ────────────────────────────────────────────────────────────────

/// A field Digimon hosting the interactive Option-use cost reducer (the
/// BT25-049 Armalizamon shape).
fn armalizamon(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(4000);
    c.play_cost = 4;
    c
}

/// A Glowing Dawn Option (cost 3) with a benign [Main] body so it is playable.
fn glowing_dawn_option(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Option;
    c.level = None;
    c.dp = None;
    c.play_cost = 3;
    c.traits = vec!["Glowing Dawn".to_string()];
    c
}

/// A plain (non-Glowing-Dawn) Option (cost 3) with a [Main] body.
fn plain_option(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Option;
    c.level = None;
    c.dp = None;
    c.play_cost = 3;
    c
}

fn tamer(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Tamer;
    c.level = None;
    c.dp = None;
    c.play_cost = 3;
    c
}

fn make_filler(id: &str) -> CardData {
    make_test_card(id, id)
}

/// Benign [Main] body so the synthetic Option is playable (gains 0 memory).
struct OptionMainNoop;
impl CardEffect for OptionMainNoop {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::on_play(card)
            .option_main()
            .name("noop option main")
            .process(|_ctx| {})
            .build()]
    }
}

/// The interactive Option-use cost reducer hosted on the field Digimon.
///
/// `before_pay_cost`, NOT optional, fires for an OPTION-USE cost (not a
/// digivolve) whose target Option carries the "Glowing Dawn" trait. The
/// pay_cost is the interactive `trash_bottom_face_down_source_under_tamer`
/// idiom (mirrored faithfully). Reduces by 3.
struct InteractiveOptionUseReducer;
impl CardEffect for InteractiveOptionUseReducer {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::before_pay_cost(card)
            .name("BT25-049-style Option-use cost reducer")
            .condition(|rctx| {
                // Fire only for a NON-digivolve (Option-use) cost into a
                // [Glowing Dawn] Option target.
                if rctx.cost_is_digivolve {
                    return false;
                }
                let Some(target) = rctx.cost_target_card else {
                    return false;
                };
                rctx.game.card_data_for_handle(target).is_some_and(|d| {
                    d.card_kind == CardKind::Option
                        && d.traits.iter().any(|t| t == "Glowing Dawn")
                })
            })
            .cost_reduction(3)
            .pay_cost_interactive(true)
            .pay_cost_fn(|ctx| {
                let player = ctx.player;
                let eligible: Vec<PermanentHandle> = ctx.game.players[player as usize]
                    .battle_area
                    .iter()
                    .enumerate()
                    .filter(|(_, perm)| {
                        perm.top_card().card_kind(&ctx.game.card_data) == CardKind::Tamer
                            && perm.card_sources.first().is_some_and(|src| src.face_down)
                    })
                    .map(|(idx, _)| PermanentHandle {
                        player,
                        index: idx as u8,
                    })
                    .collect();
                if eligible.is_empty() {
                    ctx.cost_unpayable = true;
                    return false;
                }
                ctx.select_own_permanent(
                    "Trash the bottom face-down card under a Tamer (Option-use cost)",
                    false,
                    move |_game, handle| eligible.iter().any(|h| *h == handle),
                    move |cb_ctx, handle| {
                        let trashed = cb_ctx.trash_bottom_face_down_source(handle);
                        debug_assert!(
                            trashed,
                            "eligibility filter offered a Tamer whose bottom source is not face-down"
                        );
                    },
                );
                false
            })
            .build()]
    }
}

/// OPTIONAL variant of the interactive Option-use reducer — installs an
/// accept/DECLINE gate first (the "you may, by trashing ... reduce" shape).
/// Exercises the decline re-entry path (the soft-lock the play-from-hand /
/// digivolve siblings already avoid).
struct InteractiveOptionUseReducerOptional;
impl CardEffect for InteractiveOptionUseReducerOptional {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::before_pay_cost(card)
            .name("optional Option-use cost reducer")
            .optional()
            .condition(|rctx| {
                if rctx.cost_is_digivolve {
                    return false;
                }
                let Some(target) = rctx.cost_target_card else {
                    return false;
                };
                rctx.game.card_data_for_handle(target).is_some_and(|d| {
                    d.card_kind == CardKind::Option
                        && d.traits.iter().any(|t| t == "Glowing Dawn")
                })
            })
            .cost_reduction(3)
            .pay_cost_interactive(true)
            .pay_cost_fn(|ctx| {
                let player = ctx.player;
                let eligible: Vec<PermanentHandle> = ctx.game.players[player as usize]
                    .battle_area
                    .iter()
                    .enumerate()
                    .filter(|(_, perm)| {
                        perm.top_card().card_kind(&ctx.game.card_data) == CardKind::Tamer
                            && perm.card_sources.first().is_some_and(|src| src.face_down)
                    })
                    .map(|(idx, _)| PermanentHandle {
                        player,
                        index: idx as u8,
                    })
                    .collect();
                if eligible.is_empty() {
                    ctx.cost_unpayable = true;
                    return false;
                }
                ctx.select_own_permanent(
                    "Trash the bottom face-down card under a Tamer (Option-use cost)",
                    false,
                    move |_game, handle| eligible.iter().any(|h| *h == handle),
                    move |cb_ctx, handle| {
                        let _ = cb_ctx.trash_bottom_face_down_source(handle);
                    },
                );
                false
            })
            .build()]
    }
}

/// Build a runner: the reducer-hosting Digimon on field, a Tamer with one
/// face-down stash source, and a Glowing Dawn Option (cost 3) in hand.
fn option_runner(option_id: &str, option_is_glowing_dawn: bool) -> (DebugRunner, usize) {
    option_runner_with(option_id, option_is_glowing_dawn, false)
}

/// `optional_reducer = true` registers the optional (accept/decline-gated)
/// variant instead of the mandatory one.
fn option_runner_with(
    option_id: &str,
    option_is_glowing_dawn: bool,
    optional_reducer: bool,
) -> (DebugRunner, usize) {
    let option = if option_is_glowing_dawn {
        glowing_dawn_option(option_id)
    } else {
        plain_option(option_id)
    };
    let mut r = DebugRunner::builder()
        .add_card(armalizamon("ARMA"))
        .add_card(option)
        .add_card(tamer("TAMER"))
        .add_card(make_filler("STASH"))
        .add_card(make_filler("FILLER-DECK"))
        .hand(0, &[option_id])
        .deck(0, &["FILLER-DECK"; 5])
        .deck(1, &["FILLER-DECK"; 5])
        .memory(10)
        .start();
    if optional_reducer {
        r.register_effect("ARMA", std::sync::Arc::new(InteractiveOptionUseReducerOptional));
    } else {
        r.register_effect("ARMA", std::sync::Arc::new(InteractiveOptionUseReducer));
    }
    r.register_effect(option_id, std::sync::Arc::new(OptionMainNoop));

    r.place_on_field(0, "ARMA", Some(0));
    let tamer_perm = r.place_stack(0, &["STASH", "TAMER"]);
    r.game.players[0].battle_area[tamer_perm.index as usize].card_sources[0].face_down = true;

    (r, tamer_perm.index as usize)
}

// ─── Tests ───────────────────────────────────────────────────────────────────

/// RED→GREEN: using a [Glowing Dawn] Option (cost 3) with one eligible Tamer
/// (one face-down stash) parks the interactive Tamer pick; resolving it trashes
/// the face-down source AND credits the -3 reduction (cost 3 → effective 0).
#[test]
fn interactive_option_use_reducer_credits_reduction_on_paid_park() {
    let (mut r, _tamer) = option_runner("GD-OPT", true);

    let memory_before = r.memory();
    let trash_before = r.trash_size(0);

    // GD-OPT is hand index 0.
    let result = r.game.play_option_from_hand(0, 0);
    assert!(
        matches!(result, OptionPlayResult::Pending),
        "playing the Glowing Dawn Option must park on the interactive Tamer pick"
    );

    let pending = r
        .game
        .pending_selection
        .as_ref()
        .expect("interactive reducer parks on the Tamer pick");
    assert!(matches!(pending.kind, SelectionKind::OwnField));
    assert_eq!(r.memory(), memory_before, "no cost paid before the park resolves");

    let _ = r.auto_resolve();
    assert!(r.game.pending_selection.is_none(), "option resolves");

    assert_eq!(
        r.trash_size(0),
        // The Option itself trashes to the trash after resolution (Standard
        // subtype), plus the face-down stash source. So trash grows by 2.
        trash_before + 2,
        "the bottom face-down stash source was trashed as the cost (+1) and the \
         resolved Option went to trash (+1)"
    );
    assert_eq!(
        memory_before - r.memory(),
        0,
        "Option use cost 3 reduced by 3 → paid 0 memory (the reduction is credited \
         even though the pay_cost parked on the Tamer pick)"
    );
}

/// CONTROL: with NO face-down stash the pay_cost is unpayable, so the Option is
/// used at the FULL cost (3) and only the Option itself is trashed (no FD
/// trash). Guards against crediting a reduction that was never paid for.
#[test]
fn interactive_option_use_reducer_no_reduction_when_unpayable() {
    let (mut r, tamer_idx) = option_runner("GD-OPT", true);
    // Make the Tamer's only source face-UP (no FD stash).
    r.game.players[0].battle_area[tamer_idx].card_sources[0].face_down = false;

    let memory_before = r.memory();
    let trash_before = r.trash_size(0);

    let _ = r.game.play_option_from_hand(0, 0);
    let _ = r.auto_resolve();

    assert_eq!(
        r.trash_size(0),
        // Only the resolved Option goes to trash; no FD source trashed.
        trash_before + 1,
        "no face-down stash → only the Option is trashed (no FD trash)"
    );
    assert_eq!(
        memory_before - r.memory(),
        3,
        "unpayable reducer → full Option use cost 3 paid, no reduction credited"
    );
}

/// DECLINE (soft-lock regression guard): an OPTIONAL interactive Option-use
/// reducer installs an accept/decline gate; DECLINING it must let the Option
/// play complete at FULL cost — NOT re-prompt the same reducer forever. Before
/// the `interactive_option_use_reducer_prompted` flag fix, the `== 0` re-entry
/// guard never cleared on a 0-credit decline, so `on_decline` re-entered
/// `play_option_core` and re-installed the gate infinitely.
/// `G-COST-REDUCTION-INTERACTIVE-PAY-COST`.
#[test]
fn interactive_option_use_reducer_decline_does_not_soft_lock() {
    let (mut r, _tamer) = option_runner_with("GD-OPT", true, true);

    let memory_before = r.memory();
    let trash_before = r.trash_size(0);

    let result = r.game.play_option_from_hand(0, 0);
    assert!(
        matches!(result, OptionPlayResult::Pending),
        "the optional reducer installs an accept/decline gate"
    );
    assert!(
        r.game
            .pending_selection
            .as_ref()
            .is_some_and(|s| s.is_optional),
        "the gate is an optional accept/decline prompt"
    );

    // DECLINE once. With the fix the Option play completes; pre-fix it would
    // re-prompt the SAME gate (pending_selection would be Some again).
    r.decline_optional_trigger()
        .expect("decline the optional reducer");
    let _ = r.auto_resolve();

    assert!(
        r.game.pending_selection.is_none(),
        "after declining, the Option play completes — no re-prompt (no soft-lock)"
    );
    assert_eq!(
        r.trash_size(0),
        trash_before + 1,
        "declined → FD stash untouched; only the resolved Option is trashed"
    );
    assert_eq!(
        memory_before - r.memory(),
        3,
        "declined reducer → full Option use cost 3 paid, no reduction credited"
    );
}

/// NEGATIVE: using a NON-[Glowing Dawn] Option never fires the reducer
/// (condition fails) — full cost, FD stash untouched, no reducer prompt.
#[test]
fn interactive_option_use_reducer_inactive_on_non_glowing_dawn_option() {
    let (mut r, _tamer) = option_runner("PLAIN-OPT", false);

    let memory_before = r.memory();
    let trash_before = r.trash_size(0);

    let _ = r.game.play_option_from_hand(0, 0);
    let _ = r.auto_resolve();

    assert_eq!(
        r.trash_size(0),
        trash_before + 1,
        "non-Glowing-Dawn Option → FD stash untouched; only the Option is trashed"
    );
    assert_eq!(
        memory_before - r.memory(),
        3,
        "non-[Glowing Dawn] Option pays the full use cost 3 (no reduction)"
    );
}
