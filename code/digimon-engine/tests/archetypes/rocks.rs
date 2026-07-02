//! Rocks — archetype interaction tests (exemplar).
//!
//! Model: `qa/archetype-qa/Rocks-model.md` (the digivolution-into-removal
//! payoff line). These exemplar tests pin the multi-card interaction pattern
//! and prove the `support.rs` fixtures; the `/archetype-interaction-test-author`
//! skill extends this file with the rest of the archetype's combos.
//!
//! # Combo under test — "Greymon removal, Koromon-enabled"
//!
//! BT17-102 Greymon, `[When Digivolving]`: *"If this Digimon has [Koromon] in
//! its digivolution cards, it gains +3000 DP for the turn. Then, delete 1 of
//! your opponent's Digimon with as much or less DP as this Digimon."*
//!
//! This is a genuine **three-card interaction**: the enabler (Koromon in the
//! stack) raises Greymon's effective DP from its base 5000 to 8000, which
//! widens the removal's DP window — so a 6000-DP opponent Digimon is deletable
//! *only when the enabler is present*. The payoff (Greymon) and the target
//! (the opponent Digimon) are the other two cards.
//!
//! - Card text: cards.json BT17-102.
//! - DCGO C# reference: `$BASE_DCGO/Assets/Scripts/CardEffect/BT17/Red/BT17_102.cs`.
//! - Rules basis: deletion semantics + DP comparison (`general_rule.pdf` §11 DP,
//!   §6-2-x deletion). The `[When Digivolving]` timing is the keyword window.
//!
//! The per-card behavioral coverage lives in
//! `tests/cards_behavioral/bt17/bt17_102.rs`; this file asserts the combo as a
//! *system* (enabler ⇒ flipped deletability), which no per-card test sees.

#![allow(dead_code)]

use digimon_engine::action::build_action_mask;
use digimon_engine::action::space::{HAND_EFFECT_START, PASS};
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardKind, EffectTiming};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::TriggerSource;

use super::support::snapshot;

// ─── Combo-piece fixtures ────────────────────────────────────────────────────

/// Koromon — the digivolution-stack enabler whose presence grants Greymon the
/// +3000 buff (Greymon's clause condition is `self_digivolution_contains_name:
/// "Koromon"`).
fn make_koromon(id: &str) -> CardData {
    let mut c = make_test_card(id, "Koromon");
    c.card_kind = CardKind::Digimon;
    c.level = Some(2);
    c.dp = Some(1000);
    c
}

/// A Lv.3 non-Koromon stack base — same legal stack position, but it does NOT
/// satisfy Greymon's enabler condition.
fn make_lv3_filler(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(3);
    c.dp = Some(2000);
    c
}

/// An opponent Digimon target with a chosen DP.
fn make_opp_digimon(id: &str, name: &str, dp: i32) -> CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(dp);
    c
}

/// Fire Greymon's `[When Digivolving]` timing for `handle` (direct enqueue +
/// drain — the proven harness pattern; `place_stack` builds the stack without
/// firing the trigger, so we fire it explicitly to model the digivolve).
fn fire_when_digivolving(runner: &mut DebugRunner, handle: PermanentHandle) {
    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(handle),
    );
    runner.game.drain_effect_queue();
}

// ─── Combo: Greymon removal, Koromon-enabled ─────────────────────────────────

/// Happy path: with Koromon in the stack, Greymon's effective DP is 8000, so
/// its `[When Digivolving]` removal deletes the 6000-DP opponent Digimon while
/// the 12000-DP one survives the DP filter.
#[test]
fn greymon_removal_with_koromon_deletes_mid_dp_target() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT17-102")
        .expect("BT17-102 (Greymon) in embedded DSL pack")
        .add_card(make_koromon("KORO"))
        .add_card(make_opp_digimon("OPP-MID", "OppMid", 6000))
        .add_card(make_opp_digimon("OPP-HIGH", "OppHigh", 12000))
        .memory(0)
        .start();
    runner.game.turn_count = 1;

    runner.place_on_field(1, "OPP-MID", None);
    runner.place_on_field(1, "OPP-HIGH", None);
    // Stack [Koromon, Greymon] — the enabler sits under the payoff.
    let stack = runner.place_stack(0, &["KORO", "BT17-102"]);

    let before = snapshot(&runner);
    fire_when_digivolving(&mut runner, stack);
    let _ = runner.auto_resolve();
    let after = snapshot(&runner);

    // Exactly one opponent Digimon (the 6000-DP one) is removed: opp field −1,
    // opp trash +1.
    assert_eq!(
        after.field[1],
        before.field[1] - 1,
        "exactly one opponent Digimon should be deleted by the Koromon-enabled removal"
    );
    assert_eq!(
        after.trash[1],
        before.trash[1] + 1,
        "the deleted Digimon should land in the opponent's trash"
    );
    // The 12000-DP target is above Greymon's 8000 effective DP — it survives.
    let surviving: Vec<String> = runner.game.players[1]
        .battle_area
        .iter()
        .map(|p| p.top_card().card_name(&runner.game.card_data).to_string())
        .collect();
    assert!(
        surviving.iter().any(|n| n == "OppHigh"),
        "the 12000-DP Digimon is outside the DP window and must survive; survivors={surviving:?}"
    );
}

/// Enabler-absent path: the SAME board without Koromon leaves Greymon at its
/// base 5000 DP, so the 6000-DP target is outside the removal's DP window and
/// survives. The combo's removal is gated on the enabler — exactly the
/// system-level fact a per-card test can't express.
#[test]
fn greymon_removal_without_koromon_spares_mid_dp_target() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT17-102")
        .expect("BT17-102 (Greymon) in embedded DSL pack")
        .add_card(make_lv3_filler("LV3-FILLER"))
        .add_card(make_opp_digimon("OPP-MID", "OppMid", 6000))
        .add_card(make_opp_digimon("OPP-HIGH", "OppHigh", 12000))
        .memory(0)
        .start();
    runner.game.turn_count = 1;

    runner.place_on_field(1, "OPP-MID", None);
    runner.place_on_field(1, "OPP-HIGH", None);
    // Stack [Lv3 filler, Greymon] — no Koromon, so no +3000 buff.
    let stack = runner.place_stack(0, &["LV3-FILLER", "BT17-102"]);

    let before = snapshot(&runner);
    fire_when_digivolving(&mut runner, stack);
    let _ = runner.auto_resolve();
    let after = snapshot(&runner);

    // Greymon is at base 5000; neither the 6000- nor the 12000-DP Digimon is a
    // legal target, so the opponent's board is untouched.
    assert_eq!(
        after.field[1], before.field[1],
        "with no enabler, Greymon's 5000 DP window deletes nothing — opp field unchanged"
    );
    assert_eq!(
        after.trash[1], before.trash[1],
        "no opponent Digimon should be trashed without the Koromon enabler"
    );
}

// ─── Shared fixtures for the source-trash engine combos ──────────────────────

/// A neutral opponent Digimon at a chosen play cost. Used as removal targets so
/// a combo's delete clause has something to bite. Costs ≤4 are eligible for the
/// inherited "delete opp cost ≤4" fan-out (EX8-047 / EX8-048).
fn make_opp_target(id: &str, name: &str, play_cost: u16, dp: i32) -> CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(dp);
    c.play_cost = play_cost;
    c
}

/// A neutral carrier Digimon that holds the Mineral/Rock combo sources in its
/// digivolution stack. It is itself Mineral-trait so the `host_permanent_trait`
/// gate on the inherited triggers (`Mineral` or `Rock`) is satisfied.
fn make_mineral_carrier(id: &str, name: &str) -> CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = CardKind::Digimon;
    c.level = Some(6);
    c.dp = Some(10000);
    c.traits.push("Mineral".to_string());
    c
}

/// Drive every pending selection by always taking the first valid action (or
/// PASS on a candidate-less optional prompt), bounded so a logic bug surfaces as
/// a loop-exhaustion panic rather than a hang. This mirrors the bounded driver
/// loops used in the per-card behavioral tests for these multi-trigger cards.
fn drive_first_valid(runner: &mut DebugRunner, max_steps: usize) {
    for _ in 0..max_steps {
        let Some(view) = runner.pending_selection_view() else {
            return;
        };
        let player = view.selecting_player;
        let action = view.valid_action_ids.first().copied().unwrap_or(PASS);
        // Errors are swallowed: a rejected action (e.g. PASS at a mandatory
        // empty pick) means the engine has nothing more to do for this combo.
        if runner.execute_action(player, action).is_err() {
            return;
        }
    }
    panic!("drive_first_valid exhausted {max_steps} steps without draining the selection queue");
}

// ─── Combo C1: Magneticdramon source-trash double removal ────────────────────

/// C1 — "Magneticdramon source-trash double removal".
///
/// - Cards: EX10-036 Magneticdramon (payoff) + EX8-048 Landramon (the trashable
///   Mineral-trait inherited-delete source) + EX8-047 Sunarizamon (loaded, but
///   see the MODEL CORRECTION below).
/// - Expected mechanical outcome: Magneticdramon's `[When Digivolving]` Clause A
///   trashes exactly 3 [Mineral]/[Rock] sources from your stacks, then deletes 1
///   chosen opponent Digimon AND trashes the opponent's top security card. When
///   EX8-048 Landramon is among the 3 trashed sources, its inherited "when my
///   source is trashed → delete 1 opp Digimon cost ≤4" body ALSO fires, deleting
///   a *second* cost-≤4 opponent Digimon. Net opponent board diff: −2 Digimon
///   (1 active + 1 inherited fan-out), security −1.
/// - Rules/keyword basis: "by trashing 3 … sources" pays the cost, which fires
///   `OnDigivolutionCardTrashed` once per trashed source, dispatching each
///   trashed source's inherited body. Card text: cards.json EX10-036 / EX8-048;
///   DCGO C#: `$BASE_DCGO/.../EX10/Black/EX10_036.cs`, `.../EX8/Black/EX8_048.cs`.
///
/// MODEL CORRECTION (filed): the `Rocks-model.md` C1 claim that EX8-047
/// Sunarizamon can be one of the "3 [Mineral]/[Rock]" trashed sources is
/// mechanically impossible — **EX8-047's traits are `[Reptile, LIBERATOR]`**
/// (cards/ex8/EX8-047.yaml), not Mineral/Rock, so Magneticdramon's
/// trait-filtered cost can never select it and its inherited delete cannot fire
/// via this line. Only Mineral/Rock-trait inherited sources (EX8-048 here, or
/// EX8-005 Tumblemon for the +memory body) participate. The faithful fan-out is
/// therefore a DOUBLE delete (active + EX8-048), not a triple. This is the
/// system-level fact a per-card test can't see; the model doc is wrong on the
/// card identity, not the engine.
#[test]
fn c1_magneticdramon_source_trash_triggers_inherited_fanout_double_delete() {
    // The other two buried sources are plain Mineral fillers so they complete the
    // 3-card cost without contributing extra inherited deletes — the opponent loss
    // attributable to fan-out is exactly the EX8-048 (Mineral) body. Keeping P0's
    // trash empty of Mineral/Rock cards means Magneticdramon's Clause B re-bury
    // silently skips and does not muddy the board diff.
    let mut mk_filler = |id: &str| -> CardData {
        let mut c = make_test_card(id, id);
        c.card_kind = CardKind::Digimon;
        c.level = Some(3);
        c.dp = Some(2000);
        c.traits.push("Mineral".to_string());
        c
    };

    let mut runner = DebugRunner::builder()
        .dsl_card("EX10-036")
        .expect("EX10-036 (Magneticdramon) in embedded DSL pack")
        .dsl_card("EX8-048")
        .expect("EX8-048 (Landramon) in embedded DSL pack")
        .add_card(mk_filler("C1-MIN-F1"))
        .add_card(mk_filler("C1-MIN-F2"))
        .add_card(make_mineral_carrier("C1-CARRIER", "C1 Carrier"))
        .add_card(make_opp_target("C1-OPP-A", "C1 OppA", 3, 4000))
        .add_card(make_opp_target("C1-OPP-B", "C1 OppB", 4, 4000))
        .add_card(make_opp_target("C1-OPP-C", "C1 OppC", 2, 3000))
        .memory(10)
        .start();
    runner.game.turn_count = 1;

    // Opponent fields 3 cost-≤4 Digimon (1 active-clause victim + 1 fan-out
    // victim + 1 survivor, proving the fan-out removes exactly one extra).
    runner.place_on_field(1, "C1-OPP-A", None);
    runner.place_on_field(1, "C1-OPP-B", None);
    runner.place_on_field(1, "C1-OPP-C", None);
    // Opponent's security stack — Clause A trashes its top card.
    let sec = make_test_card("C1-SEC", "C1 Security");
    runner.game.card_data.push(sec);
    let sec_idx = runner.game.card_data.len() - 1;
    for _ in 0..2 {
        let cs = CardSource::new(sec_idx, 1, runner.game.next_card_index());
        runner.game.players[1].security.push(cs);
    }

    // Carrier stack: bury the Mineral inherited-delete source (EX8-048) + 2
    // Mineral fillers under a Mineral carrier so Clause A's "trash 3" picks all
    // three Mineral/Rock-trait cards.
    let carrier = runner.place_on_field(0, "C1-CARRIER", None);
    runner.push_source(carrier, "EX8-048");
    runner.push_source(carrier, "C1-MIN-F1");
    runner.push_source(carrier, "C1-MIN-F2");

    let magnet = runner.place_on_field(0, "EX10-036", None);

    let before = snapshot(&runner);
    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(magnet),
    );
    runner.game.drain_effect_queue();
    drive_first_valid(&mut runner, 40);
    let after = snapshot(&runner);

    // Active delete (1) + EX8-048 inherited fan-out delete (1) = 2 removed.
    assert_eq!(
        after.field[1],
        before.field[1] - 2,
        "Clause A active delete + EX8-048 inherited fan-out must remove 2 opponent Digimon \
         (before={}, after={})",
        before.field[1],
        after.field[1],
    );
    // Clause A trashes the opponent's top security card.
    assert_eq!(
        after.security[1],
        before.security[1] - 1,
        "Clause A must trash the opponent's top security card"
    );
}

/// C1 enabler-absent path: with NO inherited-delete source among the trashed 3
/// (three plain Mineral fillers instead of EX8-047/EX8-048), only Clause A's
/// single active delete fires — opponent field −1, not −3. This is the
/// system-level fact a per-card test can't show: the second/third deletes are
/// *gated on which sources were trashed*.
#[test]
fn c1_without_inherited_delete_sources_only_active_delete_fires() {
    let mk_filler = |id: &str| -> CardData {
        let mut c = make_test_card(id, id);
        c.card_kind = CardKind::Digimon;
        c.level = Some(3);
        c.dp = Some(2000);
        c.traits.push("Mineral".to_string());
        c
    };

    let mut runner = DebugRunner::builder()
        .dsl_card("EX10-036")
        .expect("EX10-036 (Magneticdramon) in embedded DSL pack")
        .add_card(mk_filler("C1N-F1"))
        .add_card(mk_filler("C1N-F2"))
        .add_card(mk_filler("C1N-F3"))
        .add_card(make_mineral_carrier("C1N-CARRIER", "C1N Carrier"))
        .add_card(make_opp_target("C1N-OPP-A", "C1N OppA", 3, 4000))
        .add_card(make_opp_target("C1N-OPP-B", "C1N OppB", 4, 4000))
        .add_card(make_opp_target("C1N-OPP-C", "C1N OppC", 2, 3000))
        .memory(10)
        .start();
    runner.game.turn_count = 1;

    runner.place_on_field(1, "C1N-OPP-A", None);
    runner.place_on_field(1, "C1N-OPP-B", None);
    runner.place_on_field(1, "C1N-OPP-C", None);
    let sec = make_test_card("C1N-SEC", "C1N Security");
    runner.game.card_data.push(sec);
    let sec_idx = runner.game.card_data.len() - 1;
    let cs = CardSource::new(sec_idx, 1, runner.game.next_card_index());
    runner.game.players[1].security.push(cs);

    let carrier = runner.place_on_field(0, "C1N-CARRIER", None);
    runner.push_source(carrier, "C1N-F1");
    runner.push_source(carrier, "C1N-F2");
    runner.push_source(carrier, "C1N-F3");

    let magnet = runner.place_on_field(0, "EX10-036", None);

    let before = snapshot(&runner);
    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(magnet),
    );
    runner.game.drain_effect_queue();
    drive_first_valid(&mut runner, 40);
    let after = snapshot(&runner);

    assert_eq!(
        after.field[1],
        before.field[1] - 1,
        "with no inherited-delete source trashed, only Clause A's single active delete fires \
         (before={}, after={})",
        before.field[1],
        after.field[1],
    );
}

// ─── Combo C2: Proganomon cheat-evolve, Close-gated ──────────────────────────

/// Build the C2 board: a real EX8-047 Sunarizamon on field, an EX8-048 Landramon
/// in trash, EX10-032 Proganomon in hand, plus a [Close] (here EX8-067) on field
/// when `with_close` is true. Returns the runner.
fn c2_runner(with_close: bool) -> DebugRunner {
    let mut b = DebugRunner::builder()
        .dsl_card("EX10-032")
        .expect("EX10-032 (Proganomon) in embedded DSL pack")
        .dsl_card("EX8-047")
        .expect("EX8-047 (Sunarizamon) in embedded DSL pack")
        .dsl_card("EX8-048")
        .expect("EX8-048 (Landramon) in embedded DSL pack");
    if with_close {
        b = b
            .dsl_card("EX8-067")
            .expect("EX8-067 (Close) in embedded DSL pack");
    }
    let mut runner = b.hand(0, &["EX10-032"]).memory(10).start();
    runner.game.turn_count = 1;

    // A real Sunarizamon (EX8-047) on field — the cheat-evolve base.
    runner.place_on_field(0, "EX8-047", None);
    if with_close {
        runner.place_on_field(0, "EX8-067", None);
    }
    // A real Landramon (EX8-048) seeded in trash — placed under Sunarizamon.
    runner.inject_trash(0, "EX8-048");
    runner
}

/// C2 — "Proganomon cheat-evolve, Close-gated".
///
/// - Cards: EX10-032 Proganomon (hand) + EX8-067 Close (field) + EX8-047
///   Sunarizamon (field) + EX8-048 Landramon (trash).
/// - Expected mechanical outcome: EX10-032's `[Hand][Main]` is legal because a
///   [Close] is on field, a [Sunarizamon] is on field, and a [Landramon] is in
///   trash. Activating it places the Landramon from trash as the Sunarizamon's
///   bottom source, then that Sunarizamon digivolves into Proganomon for
///   digivolution cost 3 ignoring requirements. Diff: trash −1 (Landramon
///   leaves), the field stack's top card becomes EX10-032, memory −3.
/// - Rules/keyword basis: cost-paid-by-placing + ignore-digivolution-requirement.
///   DCGO C#: `$BASE_DCGO/.../EX10/Black/EX10_032.cs` (CanUseCondition requires
///   Close + Sunarizamon on field + Landramon in trash).
///
/// This is a four-card tempo line per-card TDD can't express as a system: the
/// hand card's legality is *gated on the rest of the board*.
#[test]
fn c2_proganomon_cheat_evolve_with_close_places_landramon_and_digivolves() {
    let mut runner = c2_runner(true);

    // The Sunarizamon permanent is the digivolve base.
    let suna = runner.perm_handle(0, 0);
    assert_eq!(
        runner.game.players[0].battle_area[suna.index as usize]
            .top_card()
            .card_id(&runner.game.card_data),
        "EX8-047",
        "field index 0 must be the Sunarizamon base"
    );

    // [Hand][Main] must be masked legal with all three preconditions present.
    let mask = build_action_mask(&runner.game, 0);
    assert_eq!(
        mask[HAND_EFFECT_START as usize], 1.0,
        "EX10-032 [Hand][Main] must be legal with Close + Sunarizamon on field and Landramon in trash"
    );

    let before = snapshot(&runner);

    // Activate the cheat-evolve and resolve its selections (Landramon pick →
    // Sunarizamon pick → effect digivolve, plus any WhenDigivolving follow-up).
    runner.game.decode_action(HAND_EFFECT_START, 0);
    drive_first_valid(&mut runner, 30);
    let after = snapshot(&runner);

    // The Landramon left the trash to become a digivolution source.
    assert_eq!(
        after.trash[0],
        before.trash[0] - 1,
        "the Landramon must leave trash to be placed as Sunarizamon's bottom source \
         (before={}, after={})",
        before.trash[0],
        after.trash[0],
    );
    // The base stack's top card is now Proganomon.
    assert_eq!(
        runner.game.players[0].battle_area[suna.index as usize]
            .top_card()
            .card_id(&runner.game.card_data),
        "EX10-032",
        "Sunarizamon must have digivolved into EX10-032 Proganomon"
    );
    // The placed Landramon sits beneath Proganomon as a source.
    let stack: Vec<&str> = runner.game.players[0].battle_area[suna.index as usize]
        .card_sources
        .iter()
        .map(|s| s.card_id(&runner.game.card_data))
        .collect();
    assert!(
        stack.contains(&"EX8-048"),
        "the placed Landramon must be in the resulting stack as a source; stack={stack:?}"
    );
    // Cost-3 effect digivolve reduced memory by exactly 3 (10 → 7).
    assert_eq!(
        after.memory,
        before.memory - 3,
        "effect-initiated digivolve at cost 3 must reduce memory by 3 (before={}, after={})",
        before.memory,
        after.memory,
    );
}

/// C2 enabler-absent path: with NO [Close] on field, EX10-032's `[Hand][Main]`
/// is gated off — the action is masked illegal, so the cheat line cannot be
/// activated and the board is untouched. The Close-gate is the system-level
/// precondition the combo depends on.
#[test]
fn c2_proganomon_cheat_evolve_masked_without_close() {
    let runner = c2_runner(false);

    let mask = build_action_mask(&runner.game, 0);
    assert_eq!(
        mask[HAND_EFFECT_START as usize], 0.0,
        "EX10-032 [Hand][Main] must be masked illegal without a [Close] on field, \
         so the cheat-evolve line is unavailable"
    );
}

// ─── Combo C3: Pyramidimon trash-3 highest-cost delete + re-bury recursion ───

/// C3 — "Pyramidimon trash-3 highest-cost delete + re-bury recursion".
///
/// - Cards: EX11-044 Pyramidimon (payoff) + EX8-005 Tumblemon (Rock-trait
///   inherited +1-memory source, buried) + Mineral fillers; EX8-047 Sunarizamon
///   is loaded but, per the C1 model correction, cannot be a trashed Mineral/Rock
///   source (it is Reptile-trait).
/// - Expected mechanical outcome: Pyramidimon's `[OP][WD][WA][OPT]` Clause A
///   trashes 3 [Mineral]/[Rock] sources from its own stack → deletes the
///   opponent's single highest-play-cost Digimon. EX8-005 Tumblemon among the
///   trashed sources fans out its inherited +1 memory. Pyramidimon's
///   `[All Turns][OPT]` Clause B then refuels by placing 3 [Mineral]/[Rock] cards
///   from trash as its own bottom sources, so its source count is restored. Net:
///   opp field −1 (highest cost), P0 memory nets the Tumblemon +1, and
///   Pyramidimon's stack source-count returns to its pre-activation size.
/// - Rules/keyword basis: "by trashing any 3 … sources" cost; highest-play-cost
///   selector; the trash event dispatches each trashed source's inherited body
///   and Pyramidimon's own re-bury observer (gated `event_host_permanent_is_source`).
///   DCGO C#: `$BASE_DCGO/.../EX11/Black/EX11_044.cs`, `.../EX8/Black/EX8_005.cs`.
///
/// This proves the deck's signature recursion: a single payoff activation both
/// removes board (and gains memory via the buried Tumblemon) AND restocks its own
/// fuel from trash — invisible to per-card tests that fire one clause at a time.
#[test]
fn c3_pyramidimon_trash_three_fanout_and_rebury_restores_sources() {
    // Two plain Mineral fillers + EX8-005 (Rock) complete the 3-card cost; the
    // fan-out body that fires is EX8-005's inherited +1 memory.
    let mut mk_filler = |id: &str| -> CardData {
        let mut c = make_test_card(id, id);
        c.card_kind = CardKind::Digimon;
        c.level = Some(3);
        c.dp = Some(2000);
        c.traits.push("Mineral".to_string());
        c
    };

    // Three Mineral/Rock cards in trash for Clause B to re-bury.
    let mk_trash = |id: &str| -> CardData {
        let mut c = make_test_card(id, id);
        c.card_kind = CardKind::Digimon;
        c.level = Some(3);
        c.dp = Some(2000);
        c.traits.push("Mineral".to_string());
        c
    };

    let mut runner = DebugRunner::builder()
        .dsl_card("EX11-044")
        .expect("EX11-044 (Pyramidimon) in embedded DSL pack")
        .dsl_card("EX8-005")
        .expect("EX8-005 (Tumblemon) in embedded DSL pack")
        .add_card(mk_filler("C3-MIN-F1"))
        .add_card(mk_filler("C3-MIN-F2"))
        .add_card(mk_trash("C3-TR1"))
        .add_card(mk_trash("C3-TR2"))
        .add_card(mk_trash("C3-TR3"))
        .add_card(make_opp_target("C3-OPP-LOW", "C3 OppLow", 3, 3000))
        .add_card(make_opp_target("C3-OPP-HIGH", "C3 OppHigh", 11, 12000))
        .memory(5)
        .start();
    runner.game.turn_count = 1;

    runner.place_on_field(1, "C3-OPP-LOW", None);
    runner.place_on_field(1, "C3-OPP-HIGH", None);

    // Pyramidimon with the three combo sources buried in ITS OWN stack (so the
    // re-bury observer, gated to this Digimon, fires off the trash event).
    let pyra = runner.place_on_field(0, "EX11-044", None);
    runner.push_source(pyra, "EX8-005");
    runner.push_source(pyra, "C3-MIN-F1");
    runner.push_source(pyra, "C3-MIN-F2");

    // Re-bury fuel in trash.
    runner.inject_trash(0, "C3-TR1");
    runner.inject_trash(0, "C3-TR2");
    runner.inject_trash(0, "C3-TR3");

    let sources_before = runner.game.players[0].battle_area[pyra.index as usize]
        .card_sources
        .len();
    let before = snapshot(&runner);

    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(pyra),
    );
    runner.game.drain_effect_queue();
    drive_first_valid(&mut runner, 60);
    let after = snapshot(&runner);

    // Active highest-cost delete (the cost-11 OppHigh) removes exactly 1 opponent
    // Digimon (the buried EX8-005 contributes memory, not a delete).
    assert_eq!(
        after.field[1],
        before.field[1] - 1,
        "Clause A highest-play-cost delete must remove 1 opponent Digimon \
         (before={}, after={})",
        before.field[1],
        after.field[1],
    );
    // The cost-11 OppHigh is the highest-play-cost target and must be gone.
    let opp_names: Vec<&str> = runner.game.players[1]
        .battle_area
        .iter()
        .map(|p| p.top_card().card_id(&runner.game.card_data))
        .collect();
    assert!(
        !opp_names.contains(&"C3-OPP-HIGH"),
        "the highest-play-cost opponent Digimon (cost 11) must be the active-clause victim; \
         remaining={opp_names:?}"
    );
    // EX8-005 Tumblemon's inherited +1 memory fired (net memory rose by ≥1 above
    // the pre-activation value).
    assert!(
        after.memory >= before.memory + 1,
        "EX8-005 Tumblemon inherited trash must gain ≥1 memory (before={}, after={})",
        before.memory,
        after.memory,
    );
    // Clause B re-bury restores Pyramidimon's source count: −3 trashed, +3
    // re-buried from trash → net source-count unchanged.
    let sources_after = runner.game.players[0].battle_area[pyra.index as usize]
        .card_sources
        .len();
    assert_eq!(
        sources_after, sources_before,
        "Clause B must re-bury 3 from trash to offset the 3 trashed sources \
         (before={sources_before}, after={sources_after})"
    );
}

// ─── Combo C4: Close suspend-refuel on Mineral/Rock digivolve ────────────────

/// Pop `card_id` from player 0's hand and raw-digivolve it onto `base`, then
/// fire WhenDigivolving + OnDigivolve so face-up observers (Close) see the
/// digivolve event. Mirrors the proven `fire_digivolve_onto` helper from
/// `tests/cards_behavioral/ex8/ex8_067.rs`, narrowed to player 0.
fn fire_digivolve_onto(runner: &mut DebugRunner, base: PermanentHandle, card_id: &str) {
    let card = {
        let data_idx = runner
            .game
            .card_data
            .iter()
            .position(|c| c.card_id == card_id)
            .expect("digivolve card must be in card_data");
        let hand = &mut runner.game.players[0].hand;
        let pos = hand
            .iter()
            .position(|cs| cs.data_index == data_idx)
            .expect("digivolve card must be in hand");
        hand.remove(pos)
    };
    let card_handle = card.handle();
    let turn = runner.game.turn_count;
    {
        let perm = runner.game.players[0]
            .battle_area
            .get_mut(base.index as usize)
            .expect("base permanent must be on field");
        perm.digivolve(card, turn);
    }
    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(base),
    );
    runner.game.drain_effect_queue();
    runner.game.enqueue_triggered(
        EffectTiming::OnDigivolve,
        TriggerSource::Digivolved {
            player: 0,
            permanent: base,
            card: card_handle,
            effect_initiated: false,
            dna_origin: false,
        },
    );
    runner.game.drain_effect_queue();
}

/// C4 — "Close suspend-refuel on Mineral/Rock digivolve".
///
/// - Cards: EX8-067 Close (unsuspended, field) + EX8-047 Sunarizamon (field
///   base) + EX8-048 Landramon (the Mineral Digimon digivolved into).
/// - Expected mechanical outcome: when your Sunarizamon digivolves into Landramon
///   (a [Mineral] Digimon) on your turn, Close's `[Your Turn]` observer offers to
///   suspend Close and place up to 2 Mineral/Rock cards from trash as that
///   Digimon's bottom sources. Accepting: Close → suspended; trash shrinks by the
///   placed count; the digivolved stack's source count grows by the same count.
///   This pre-loads the very fuel that C1/C3 later trash.
/// - Rules/keyword basis: "by suspending this Tamer" = suspend-as-cost; placing
///   from trash as bottom digivolution sources. DCGO C#:
///   `$BASE_DCGO/.../EX8/Black/EX8_067.cs`.
///
/// The per-card test fires the observer with synthetic Mineral cards; this
/// interaction test wires the *real* Sunarizamon→Landramon digivolution line to
/// the *real* Close, proving the engine half of the refuel loop end to end.
#[test]
fn c4_close_suspends_to_refuel_sources_on_mineral_digivolve() {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX8-067")
        .expect("EX8-067 (Close) in embedded DSL pack")
        .dsl_card("EX8-047")
        .expect("EX8-047 (Sunarizamon) in embedded DSL pack")
        .dsl_card("EX8-048")
        .expect("EX8-048 (Landramon) in embedded DSL pack")
        .memory(10)
        .start();
    runner.game.turn_count = 1;

    // Close on field (unsuspended). Sunarizamon on field as the digivolve base.
    let close = runner.place_on_field(0, "EX8-067", None);
    let base = runner.place_on_field(0, "EX8-047", Some(0));
    // Two Mineral cards in trash to be placed as the refuel.
    runner.inject_trash(0, "EX8-047");
    runner.inject_trash(0, "EX8-048");
    // Landramon in hand to digivolve into.
    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == "EX8-048")
        .expect("EX8-048 registered");
    let next = runner.game.next_card_index();
    runner.game.players[0]
        .hand
        .push(CardSource::new(data_idx, 0, next));

    fire_digivolve_onto(&mut runner, base, "EX8-048");

    // Close's observer must offer the (optional) suspend-to-refuel prompt.
    assert!(
        runner.pending_selection().is_some(),
        "Close must offer the suspend-refuel prompt when a Mineral Digimon digivolves on your turn"
    );
    assert!(
        runner.pending_is_optional(),
        "the suspend-refuel activation prompt must be optional"
    );
    // Baseline the stack source-count AFTER the digivolve (which pushes the
    // EX8-048 top card) but BEFORE the refuel resolves, so the delta isolates
    // the cards placed from trash by Close's effect.
    let sources_before = runner.game.players[0].battle_area[base.index as usize]
        .card_sources
        .len();
    let before = snapshot(&runner);
    // Accept and resolve the placements (pick first valid each prompt).
    drive_first_valid(&mut runner, 20);
    let after = snapshot(&runner);

    // Close became suspended (the cost was paid).
    assert!(
        runner.game.players[0].battle_area[close.index as usize].is_suspended,
        "Close must be suspended after the refuel activation is accepted"
    );
    // Sources moved out of trash onto the digivolved stack: net trash down,
    // source count up by the same amount (1 or 2 — "up to 2").
    let sources_after = runner.game.players[0].battle_area[base.index as usize]
        .card_sources
        .len();
    let placed = sources_after - sources_before;
    assert!(
        placed >= 1,
        "at least 1 Mineral/Rock card must be placed as a bottom source \
         (before={sources_before}, after={sources_after})"
    );
    assert_eq!(
        before.trash[0] - after.trash[0],
        placed,
        "the cards placed as bottom sources must have left trash (placed={placed}, \
         trash before={}, after={})",
        before.trash[0],
        after.trash[0],
    );
}

/// C4 unhappy path: with Close ALREADY suspended, the suspend cost cannot be
/// paid, so the observer installs no prompt and no sources move — the system
/// fact that the refuel is gated on Close being unsuspended.
#[test]
fn c4_close_already_suspended_offers_no_refuel() {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX8-067")
        .expect("EX8-067 (Close) in embedded DSL pack")
        .dsl_card("EX8-047")
        .expect("EX8-047 (Sunarizamon) in embedded DSL pack")
        .dsl_card("EX8-048")
        .expect("EX8-048 (Landramon) in embedded DSL pack")
        .memory(10)
        .start();
    runner.game.turn_count = 1;

    let close = runner.place_on_field(0, "EX8-067", None);
    // Pre-suspend Close so the cost is unpayable.
    runner.game.players[0].battle_area[close.index as usize].is_suspended = true;
    let base = runner.place_on_field(0, "EX8-047", Some(0));
    runner.inject_trash(0, "EX8-047");
    runner.inject_trash(0, "EX8-048");
    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == "EX8-048")
        .expect("EX8-048 registered");
    let next = runner.game.next_card_index();
    runner.game.players[0]
        .hand
        .push(CardSource::new(data_idx, 0, next));

    fire_digivolve_onto(&mut runner, base, "EX8-048");

    // No refuel prompt fires (the suspend cost is unpayable), so the digivolve
    // is the only state change. Baseline the post-digivolve stack/trash and
    // confirm nothing further moves.
    assert!(
        runner.pending_selection().is_none(),
        "no refuel prompt when Close is already suspended (suspend cost unpayable)"
    );
    let sources_after_digivolve = runner.game.players[0].battle_area[base.index as usize]
        .card_sources
        .len();
    let trash_after_digivolve = runner.trash_size(0);
    // Drive once more — with no pending prompt this is a no-op and must not
    // place any source or touch trash.
    drive_first_valid(&mut runner, 4);
    assert_eq!(
        runner.game.players[0].battle_area[base.index as usize]
            .card_sources
            .len(),
        sources_after_digivolve,
        "no sources may be placed when Close cannot pay the suspend cost"
    );
    assert_eq!(
        runner.trash_size(0),
        trash_after_digivolve,
        "trash must be unchanged when the refuel cannot activate"
    );
}

// ─── Combo C5: Gravel Hearts cheat-play + Delay cost-reduced digivolve ────────

/// C5 — "Gravel Hearts cheat-play + Delay cost-reduced digivolve".
///
/// - Cards: EX10-069 Unique Emblem: Gravel Hearts (Option) + EX8-047 Sunarizamon
///   (the free-played body) + EX8-067 Close (arms the Delay by suspending).
/// - Expected mechanical outcome (Main): Gravel Hearts' `[Main]` plays 1
///   [Sunarizamon] from hand without paying the cost, then places itself in the
///   battle area as a Delay Option. Diff: hand −1 (Sunarizamon enters play free),
///   Gravel Hearts → battle area as an `OnSuspend`-gated Delay.
/// - Rules/keyword basis: free-play + Delay placement. The `<Delay>` cannot
///   activate the turn it is placed (§16-16-3) — exercised in the unhappy path
///   below. DCGO C#: `$BASE_DCGO/.../EX10/Black/EX10_069.cs`.
///
/// This pins the cheat-play half of the combo as a board diff (free body in
/// play + Delay armed), the multi-card setup a per-card test under-specifies.
#[test]
fn c5_gravel_hearts_main_free_plays_sunarizamon_and_arms_delay() {
    use digimon_engine::enums::{CardColor, DelayTrigger};
    use digimon_engine::permanent::OptionState;
    use digimon_engine::selection::OptionPlayResult;

    // A Black anchor satisfies the Black Option's colour requirement at play
    // time (make_test_card defaults to Red, so set Black explicitly).
    let mut anchor = make_test_card("C5-ANCHOR", "C5 Anchor");
    anchor.card_kind = CardKind::Digimon;
    anchor.level = Some(3);
    anchor.dp = Some(3000);
    anchor.play_cost = 3;
    anchor.colors = vec![CardColor::Black];
    let filler = make_test_card("C5-FILL", "C5 Fill");

    let mut runner = DebugRunner::builder()
        .dsl_card("EX10-069")
        .expect("EX10-069 (Gravel Hearts) in embedded DSL pack")
        .dsl_card("EX8-047")
        .expect("EX8-047 (Sunarizamon) in embedded DSL pack")
        .add_card(anchor)
        .add_card(filler)
        .hand(0, &["EX10-069", "EX8-047"])
        .deck(0, &["C5-FILL"])
        .deck(1, &["C5-FILL"])
        .memory(10)
        .start();
    runner.game.turn_count = 1;

    runner.place_on_field(0, "C5-ANCHOR", Some(0));
    runner.game.enter_main_phase();

    let before = snapshot(&runner);

    // Gravel Hearts [Main] starts with a zone-choice selection, so the option
    // pipeline parks. Find EX10-069's hand index.
    let gh_idx = runner.game.players[0]
        .hand
        .iter()
        .position(|c| c.card_id(&runner.game.card_data) == "EX10-069")
        .expect("Gravel Hearts in hand");
    assert_eq!(
        runner.game.play_option_from_hand(0, gh_idx),
        OptionPlayResult::Pending,
        "EX10-069 [Main] must park on its zone-choice selection"
    );
    // Choose "From hand" (the first effect-choice), then free-play Sunarizamon.
    drive_first_valid(&mut runner, 20);
    let after = snapshot(&runner);

    // Sunarizamon was played for free from hand: hand net −2 (Gravel Hearts +
    // the free-played Sunarizamon both left hand), and Sunarizamon is on field.
    assert_eq!(
        after.hand[0],
        before.hand[0] - 2,
        "both Gravel Hearts and the free-played Sunarizamon must leave hand \
         (before={}, after={})",
        before.hand[0],
        after.hand[0],
    );
    let suna_on_field = runner.game.players[0]
        .battle_area
        .iter()
        .any(|p| p.top_card().card_id(&runner.game.card_data) == "EX8-047");
    assert!(
        suna_on_field,
        "the free-played EX8-047 Sunarizamon must be on the field"
    );
    // The free-played Sunarizamon costs NO memory: the only memory spent is
    // Gravel Hearts' own play cost (3). If the body had been charged its cost-3
    // too, memory would have dropped by 6 — proving "without paying the cost".
    assert_eq!(
        before.memory - after.memory,
        3,
        "only Gravel Hearts' own cost (3) may be paid; the free-played Sunarizamon \
         must add no further memory drain (before={}, after={})",
        before.memory,
        after.memory,
    );
    // Gravel Hearts placed itself as an OnSuspend-gated Delay Option.
    let gh = runner.game.players[0]
        .battle_area
        .iter()
        .find(|p| p.top_card().card_id(&runner.game.card_data) == "EX10-069")
        .expect("Gravel Hearts must be placed in the battle area as a Delay option");
    assert!(
        matches!(
            gh.option_state,
            OptionState::Delayed {
                trigger: DelayTrigger::OnEvent(EffectTiming::OnSuspend),
                ..
            }
        ),
        "Gravel Hearts must be parked as an OnSuspend Delay; got {:?}",
        gh.option_state
    );
}

/// C5 unhappy path (§16-16-3): the `<Delay>` cannot activate on the turn Gravel
/// Hearts is placed. Suspending a [Close] the SAME turn must NOT fire the Delay
/// — Gravel Hearts stays parked in the battle area and no digivolve is offered.
#[test]
fn c5_gravel_hearts_delay_does_not_fire_on_placing_turn() {
    use digimon_engine::enums::CardColor;
    use digimon_engine::permanent::OptionState;
    use digimon_engine::selection::OptionPlayResult;

    let mut anchor = make_test_card("C5P-ANCHOR", "C5P Anchor");
    anchor.card_kind = CardKind::Digimon;
    anchor.level = Some(3);
    anchor.dp = Some(3000);
    anchor.play_cost = 3;
    anchor.colors = vec![CardColor::Black];
    let filler = make_test_card("C5P-FILL", "C5P Fill");

    let mut runner = DebugRunner::builder()
        .dsl_card("EX10-069")
        .expect("EX10-069 (Gravel Hearts) in embedded DSL pack")
        .dsl_card("EX8-047")
        .expect("EX8-047 (Sunarizamon) in embedded DSL pack")
        .dsl_card("EX8-067")
        .expect("EX8-067 (Close) in embedded DSL pack")
        .add_card(anchor)
        .add_card(filler)
        .hand(0, &["EX10-069", "EX8-047"])
        .deck(0, &["C5P-FILL"])
        .deck(1, &["C5P-FILL"])
        .memory(10)
        .start();
    runner.game.turn_count = 1;

    runner.place_on_field(0, "C5P-ANCHOR", Some(0));
    let close = runner.place_on_field(0, "EX8-067", Some(0));
    runner.game.enter_main_phase();

    let gh_idx = runner.game.players[0]
        .hand
        .iter()
        .position(|c| c.card_id(&runner.game.card_data) == "EX10-069")
        .expect("Gravel Hearts in hand");
    assert_eq!(
        runner.game.play_option_from_hand(0, gh_idx),
        OptionPlayResult::Pending,
        "EX10-069 [Main] must park on its zone-choice selection"
    );
    drive_first_valid(&mut runner, 20);

    // Suspend Close on the SAME (placing) turn.
    runner.game.suspend(close);

    assert!(
        runner.pending_selection().is_none(),
        "the Gravel Hearts <Delay> must NOT fire on its placing turn (§16-16-3)"
    );
    let gh_parked = runner.game.players[0].battle_area.iter().any(|p| {
        p.top_card().card_id(&runner.game.card_data) == "EX10-069"
            && matches!(p.option_state, OptionState::Delayed { .. })
    });
    assert!(
        gh_parked,
        "Gravel Hearts must remain parked as a Delay Option on its placing turn"
    );
}
