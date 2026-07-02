//! Engine substrate test for the DIGIVOLVE half of
//! `G-COST-REDUCTION-INTERACTIVE-PAY-COST`.
//!
//! Models ST23-03 Cougarmon / ST23-11 Wolvermon clause:
//!   "[Your Turn] When this Digimon would digivolve into a [Glowing Dawn] trait
//!    Digimon card, by trashing the bottom face-down card from under any of your
//!    Tamers, reduce the cost by 2."
//!
//! The "by trashing the bottom face-down card under a Tamer" cost is the
//! INTERACTIVE `trash_bottom_face_down_source_under_tamer` step: it always
//! installs a 1-option Tamer pick (the no-approximations contract never
//! auto-resolves even a single eligible Tamer), so the `pay_cost_fn` PARKS on a
//! `pending_selection`. The synchronous digivolve cost scan
//! (`scan_before_pay_cost_reduction_with_target`) cannot host a parking
//! pay_cost, so before this fix the reducer was skipped (no discount) — or
//! worse, parked a dangling selection mid-digivolve.
//!
//! This fixture's `before_pay_cost` reducer mirrors the real DSL
//! `trash_bottom_face_down_source_under_tamer` lowering exactly:
//!   - pre-filter the controller's Tamers carrying a bottom face-down source;
//!   - if NONE, set `ctx.cost_unpayable = true` and return `false` (unpayable —
//!     the reduction must NOT be credited);
//!   - else install a `select_own_permanent` whose callback trashes the bottom
//!     face-down source; the `pay_cost_fn` returns `false` because it parked.
//!
//! The engine must credit the -2 reduction when (and only when) the parked
//! Tamer pick resolves and the face-down source is actually trashed.

use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::card_source::CardHandle;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::effect::{CardEffect, Effect};
use digimon_engine::enums::{CardColor, CardKind, PlaySource};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::SelectionKind;

// Evo-color byte: Green == 3 (matches `card_data::parse_card_color`).
const EVO_GREEN: u8 = 3;

// ─── Fixtures ────────────────────────────────────────────────────────────────

/// A Lv.3 green base Digimon that hosts the interactive digivolve cost reducer
/// (the ST23-03 Cougarmon shape — the digivolving permanent itself carries the
/// reducer). It digivolves into a Lv.4 green [Glowing Dawn] card.
fn cougarmon_base(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(3);
    c.dp = Some(3000);
    c.colors = vec![CardColor::Green];
    c
}

/// A Lv.4 green [Glowing Dawn] Digimon with a printed `Lv.3 green, cost 4` evo
/// cost — the digivolve TARGET (the hand card being digivolved INTO).
fn glowing_dawn_lv4(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(5000);
    c.colors = vec![CardColor::Green];
    c.play_cost = 4;
    c.traits = vec!["Glowing Dawn".to_string()];
    c.evo_costs = vec![EvoCost {
        card_color: EVO_GREEN,
        level: 3,
        memory_cost: 4,
    }];
    c
}

/// A Lv.4 green non-[Glowing Dawn] Digimon (same evo cost) — the negative
/// digivolve target (reducer must NOT fire).
fn plain_lv4(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(5000);
    c.colors = vec![CardColor::Green];
    c.play_cost = 4;
    c.evo_costs = vec![EvoCost {
        card_color: EVO_GREEN,
        level: 3,
        memory_cost: 4,
    }];
    c
}

/// A Tamer that can host a face-down stash (the FD-trash cost target).
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

/// The interactive digivolve cost reducer hosted on the base Digimon.
///
/// `before_pay_cost`, NOT optional (printed "reduce the cost by 2", no "you
/// may"), fires for a DIGIVOLVE cost whose target card carries the
/// "Glowing Dawn" trait. The pay_cost is the interactive
/// `trash_bottom_face_down_source_under_tamer` idiom (mirrored faithfully).
struct InteractiveDigivolveReducer;
impl CardEffect for InteractiveDigivolveReducer {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::before_pay_cost(card)
            .name("ST23-03-style digivolve cost reducer")
            .condition(|rctx| {
                // Fire only for a DIGIVOLVE cost into a [Glowing Dawn] target.
                if !rctx.cost_is_digivolve {
                    return false;
                }
                let Some(target) = rctx.cost_target_card else {
                    return false;
                };
                rctx.game
                    .card_data_for_handle(target)
                    .is_some_and(|d| d.traits.iter().any(|t| t == "Glowing Dawn"))
            })
            .cost_reduction(2)
            .pay_cost_interactive(true)
            .pay_cost_fn(|ctx| {
                // Mirror `trash_bottom_face_down_source_under_tamer`: pre-filter
                // the controller's Tamers carrying a bottom face-down source.
                let player = ctx.player;
                let eligible: Vec<PermanentHandle> = ctx.game.players[player as usize]
                    .battle_area
                    .iter()
                    .enumerate()
                    .filter(|(_, perm)| {
                        perm.top_card().card_kind(&ctx.game.card_data) == CardKind::Tamer
                            && perm
                                .card_sources
                                .first()
                                .is_some_and(|src| src.face_down)
                    })
                    .map(|(idx, _)| PermanentHandle {
                        player,
                        index: idx as u8,
                    })
                    .collect();
                if eligible.is_empty() {
                    // Unpayable: no Tamer has a face-down stash. Flag so the
                    // engine does NOT credit a reduction that was never paid.
                    ctx.cost_unpayable = true;
                    return false;
                }
                // Install the (parking) Tamer pick — a 1-option selection even
                // for a single eligible Tamer (no auto-resolve).
                ctx.select_own_permanent(
                    "Trash the bottom face-down card under a Tamer (digivolution cost)",
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
                // Parked — paid on resolution.
                false
            })
            .build()]
    }
}

/// Build a runner: green base on field (hosting the reducer), a Tamer with one
/// face-down stash source, and a Lv.4 green [Glowing Dawn] target in hand.
/// Returns (runner, base field index, tamer field index).
fn digivolve_runner(target_id: &str, target_is_glowing_dawn: bool) -> (DebugRunner, usize, usize) {
    let target = if target_is_glowing_dawn {
        glowing_dawn_lv4(target_id)
    } else {
        plain_lv4(target_id)
    };
    let mut r = DebugRunner::builder()
        .add_card(cougarmon_base("BASE"))
        .add_card(target)
        .add_card(tamer("TAMER"))
        .add_card(make_filler("STASH"))
        .add_card(make_filler("FILLER-DECK"))
        .hand(0, &[target_id])
        .deck(0, &["FILLER-DECK"; 5])
        .deck(1, &["FILLER-DECK"; 5])
        .memory(10)
        .start();
    r.register_effect("BASE", std::sync::Arc::new(InteractiveDigivolveReducer));

    let base = r.place_on_field(0, "BASE", Some(0));
    // Tamer (top of [STASH, TAMER]) with one face-down stash source beneath.
    let tamer_perm = r.place_stack(0, &["STASH", "TAMER"]);
    r.game.players[0].battle_area[tamer_perm.index as usize].card_sources[0].face_down = true;

    (r, base.index as usize, tamer_perm.index as usize)
}

// ─── Tests ───────────────────────────────────────────────────────────────────

/// RED→GREEN: digivolving the green base into a [Glowing Dawn] card with one
/// eligible Tamer (one face-down stash) parks the interactive Tamer pick;
/// resolving it trashes the face-down source AND credits the -2 reduction
/// (printed digivolve cost 4 → effective 2).
#[test]
fn interactive_digivolve_reducer_credits_reduction_on_paid_park() {
    let (mut r, base, _tamer) = digivolve_runner("GD", true);

    let memory_before = r.memory();
    let trash_before = r.trash_size(0);

    // GD is hand index 0.
    let ok = r
        .game
        .digivolve_from_hand(0, 0, base, PlaySource::ByDigivolve);
    assert!(
        !ok,
        "digivolve must return false while the interactive reducer pay_cost is pending"
    );

    // The Tamer pick must be pending (OwnField, 1 option, mandatory).
    let pending = r
        .game
        .pending_selection
        .as_ref()
        .expect("interactive reducer parks on the Tamer pick");
    assert!(matches!(pending.kind, SelectionKind::OwnField));
    // No memory spent yet — the digivolve cost is paid only after the park.
    assert_eq!(
        r.memory(),
        memory_before,
        "no cost paid before the park resolves"
    );

    // Resolve the (single-eligible) Tamer pick.
    let _ = r.auto_resolve();
    assert!(r.game.pending_selection.is_none(), "digivolution completes");

    assert_eq!(
        r.trash_size(0),
        trash_before + 1,
        "the bottom face-down stash source was trashed as the cost"
    );
    assert_eq!(
        memory_before - r.memory(),
        2,
        "digivolve cost 4 reduced by 2 → paid 2 memory (the reduction is credited \
         even though the pay_cost parked on the Tamer pick)"
    );
}

/// CONTROL: with NO face-down stash under any Tamer the pay_cost is unpayable,
/// so the digivolve proceeds at the FULL printed cost (4) and nothing is
/// trashed. Guards against crediting a reduction that was never paid for.
#[test]
fn interactive_digivolve_reducer_no_reduction_when_unpayable() {
    // Same setup, but make the Tamer's only source face-UP (no FD stash).
    let (mut r, base, tamer_idx) = digivolve_runner("GD", true);
    r.game.players[0].battle_area[tamer_idx].card_sources[0].face_down = false;

    let memory_before = r.memory();
    let trash_before = r.trash_size(0);

    let ok = r
        .game
        .digivolve_from_hand(0, 0, base, PlaySource::ByDigivolve);
    // Unpayable cost → no park; the digivolve completes synchronously at full
    // cost.
    assert!(
        ok,
        "digivolve completes synchronously when the reducer is unpayable"
    );
    let _ = r.auto_resolve();

    assert_eq!(
        r.trash_size(0),
        trash_before,
        "no face-down stash → nothing trashed"
    );
    assert_eq!(
        memory_before - r.memory(),
        4,
        "unpayable reducer → full digivolve cost 4 paid, no reduction credited"
    );
}

/// NEGATIVE: digivolving into a NON-[Glowing Dawn] target never fires the
/// reducer (condition fails) — full cost, FD stash untouched, no prompt.
#[test]
fn interactive_digivolve_reducer_inactive_on_non_glowing_dawn_target() {
    let (mut r, base, _tamer) = digivolve_runner("PLAIN", false);

    let memory_before = r.memory();
    let trash_before = r.trash_size(0);

    let ok = r
        .game
        .digivolve_from_hand(0, 0, base, PlaySource::ByDigivolve);
    assert!(ok, "non-Glowing-Dawn digivolve completes synchronously");
    assert!(
        r.game.pending_selection.is_none(),
        "no reducer prompt for a non-[Glowing Dawn] target"
    );
    let _ = r.auto_resolve();

    assert_eq!(r.trash_size(0), trash_before, "FD stash untouched");
    assert_eq!(
        memory_before - r.memory(),
        4,
        "non-[Glowing Dawn] target pays the full digivolve cost 4 (no reduction)"
    );
}
