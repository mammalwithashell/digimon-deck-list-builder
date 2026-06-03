//! Puppet-Sister — archetype interaction tests (Puppets [Puppet]+[LIBERATOR]).
//!
//! Model: `qa/archetype-qa/Puppets-model.md` (canonical name **Puppets**; the
//! "Puppet-Sister" slug is an alias of the same [Puppet]+[LIBERATOR] system).
//! There is no separate `Puppet-Sister-model.md`; the durable system model for
//! this archetype is the Puppets model doc, and each `#[test]` below maps 1:1 to
//! one of its **named combos** (C1–C4 of "Named combos"):
//!
//! - C1 — Fable Waltz → Arisa engine, then Delay cost-reduced digivolve
//!   (BT22-098 + an Arisa Kinosaki Tamer + a [Puppet]+[LIBERATOR] hand body).
//! - C2 — Vortex Resonance: dig a LIBERATOR, then digivolve into hand for −4,
//!   with the LIBERATOR colour-ignore floodgate (EX7-074, both branches).
//! - C3 — Karakurumon: delete a Token/other [Puppet] to free-digivolve into a
//!   [Puppet] hand card, firing the Kyaromon death-draw (EX9-032 + BT22-002).
//! - C4 — Overclock + Token loop: extra attack by deleting a Token, the cost
//!   deletion fanning out into Kaguyamon removal + Kyaromon draw (BT22-042 +
//!   EX9-033 + BT22-002).
//!
//! Every card a combo NAMES is loaded by its real ID via `dsl_card`; synthetic
//! `make_test_card` bodies are only neutral fillers / targets / anchors a combo
//! needs but does not name. These exercise the *real* cards through their *real*
//! effect paths (the Option pipeline, the `<Delay>` event gate, the `<Overclock>`
//! keyword, deletion-as-cost) and assert the combined board diff a per-card test
//! can't see.
//!
//! Source priority (CLAUDE.md): `general_rule.pdf` (canonical, §16 keyword
//! semantics) + DCGO C# (battle-tested) outrank the card-text JSON. DCGO base
//! copy: `$BASE_DCGO/Assets/Scripts/CardEffect/<SET>/<COLOR>/<CARD_ID>.cs`
//! (underscores in filenames). The `<Delay>` not-on-placing-turn rule is
//! `general_rule.pdf` §16-16; Overclock is §16-33.
//!
//! Fixtures: `support.rs` (`dsl_builder`, `snapshot`/`BoardSnapshot`); pattern
//! reference: `tests/archetypes/rocks.rs` and the sibling `puppets.rs`.

#![allow(dead_code)]

use digimon_engine::action::space::{encode_attack, PASS};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, DelayTrigger, EffectTiming, GamePhase, Keyword};
use digimon_engine::permanent::{OptionState, PermanentHandle};
use digimon_engine::selection::OptionPlayResult;

use super::support::snapshot;

// ─── Shared helpers ──────────────────────────────────────────────────────────

/// A yellow [Puppet] Digimon at a chosen level/DP — the deck's fodder/body type.
/// Used as an *unnamed* neutral combo piece (deletion-cost body, evolution base,
/// removal target) — never a substitute for a card a combo names.
fn make_puppet(id: &str, level: u8, dp: i32) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Yellow];
    c.level = Some(level);
    c.dp = Some(dp);
    c.play_cost = level as u16;
    c.traits = vec!["Puppet".to_string()];
    c
}

/// A neutral non-Puppet Digimon (a non-trigger body / colour anchor). Not a
/// legal Puppet/Token cost body, so it never disturbs the unique cost target.
fn make_plain(id: &str, level: u8, dp: i32) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Yellow];
    c.level = Some(level);
    c.dp = Some(dp);
    c.play_cost = level as u16;
    c.traits = vec!["Beast".to_string()];
    c
}

/// Drive every pending selection by taking the first valid action (or PASS on a
/// candidate-less optional prompt), bounded so a logic bug surfaces as a
/// loop-exhaustion panic rather than a hang. Mirrors `rocks.rs`/`puppets.rs`.
fn drive_first_valid(runner: &mut DebugRunner, max_steps: usize) {
    for _ in 0..max_steps {
        let Some(view) = runner.pending_selection_view() else {
            return;
        };
        let player = view.selecting_player;
        let action = view.valid_action_ids.first().copied().unwrap_or(PASS);
        if runner.execute_action(player, action).is_err() {
            return;
        }
    }
    panic!("drive_first_valid exhausted {max_steps} steps without draining the selection queue");
}

fn field_ids(runner: &DebugRunner, player: usize) -> Vec<String> {
    runner.game.players[player]
        .battle_area
        .iter()
        .map(|p| p.top_card().card_id(&runner.game.card_data).to_string())
        .collect()
}

fn hand_has(runner: &DebugRunner, player: usize, card_id: &str) -> bool {
    runner.game.players[player]
        .hand
        .iter()
        .any(|c| c.card_id(&runner.game.card_data) == card_id)
}

fn encode_perm(handle: PermanentHandle) -> u16 {
    encode_attack(handle.player as u16, handle.index as u16)
}

/// True iff a card with `id` is parked in `player`'s battle area as a Delay
/// Option (any DelayTrigger).
fn is_parked_delay(runner: &DebugRunner, player: usize, id: &str) -> bool {
    runner.game.players[player].battle_area.iter().any(|p| {
        p.top_card().card_id(&runner.game.card_data) == id
            && matches!(p.option_state, OptionState::Delayed { .. })
    })
}

// ─── Combo C1: Fable Waltz free-play Arisa + Arisa-suspend Delay digivolve ─────

/// Combo C1 (Main half) — "Fable Waltz → Arisa engine, then Delay digivolve".
///
/// - Cards: BT22-098 Unique Emblem: Fable Waltz (Option) + EX11-060 Arisa
///   Kinosaki (the named Tamer whose suspend later arms the Delay).
/// - Expected mechanical outcome ([Main]): BT22-098 `[Main]` plays 1
///   [Shoemon]/[Arisa Kinosaki] from hand **or trash** WITHOUT paying the cost
///   (the chosen body — here the real EX11-060 Arisa from hand — leaves hand and
///   enters the battle area). Because BT22-098 carries a `kind: delay` clause,
///   the Option pipeline then auto-places Fable Waltz itself in the battle area
///   as an `OnSuspend` Delay (not trashed). Memory drains only Fable Waltz's own
///   cost (3); the free-played Arisa adds no further drain (proves "without
///   paying the cost").
/// - Rules/keyword basis: union-zone free-play (no cost paid) + `<Delay>`
///   placement (`general_rule.pdf` §16-16). Printed text + DCGO gate:
///   `cards/bt22/BT22-098.yaml` (Delay `active_when.event_card_name_contains:
///   "Arisa Kinosaki"`); DCGO C#:
///   `$BASE_DCGO/Assets/Scripts/CardEffect/BT22/Yellow/BT22_098.cs`
///   (`OnTappedAnyone` Delay gated on `IsArisaKinosaki`).
///
/// System-level fact a per-card test misses: the cheat-played body + the armed
/// Delay are one combined board state, and only the *named* Arisa suspend can
/// later arm the −3 digivolve (exercised in the happy path below).
#[test]
fn combo1_fable_waltz_main_free_plays_arisa_and_arms_delay() {
    // Yellow anchor satisfies Fable Waltz's yellow Option colour requirement.
    let mut anchor = make_plain("PS1-ANCHOR", 3, 3000);
    anchor.colors = vec![CardColor::Yellow];
    let filler = make_test_card("PS1-FILL", "Ps1 Fill");

    let mut runner = DebugRunner::builder()
        .dsl_card("BT22-098")
        .expect("BT22-098 (Fable Waltz) in embedded DSL pack")
        .dsl_card("EX11-060")
        .expect("EX11-060 (Arisa Kinosaki) in embedded DSL pack")
        .add_card(anchor)
        .add_card(filler)
        .hand(0, &["BT22-098", "EX11-060"])
        .deck(0, &["PS1-FILL", "PS1-FILL", "PS1-FILL", "PS1-FILL"])
        .deck(1, &["PS1-FILL"])
        .memory(10)
        .start();
    runner.game.turn_count = 1;

    runner.place_on_field(0, "PS1-ANCHOR", Some(0));
    runner.game.enter_main_phase();

    let before = snapshot(&runner);

    // Fable Waltz [Main] parks on its union-zone (hand ∪ trash) free-play pick.
    let fw_idx = runner.game.players[0]
        .hand
        .iter()
        .position(|c| c.card_id(&runner.game.card_data) == "BT22-098")
        .expect("Fable Waltz in hand");
    assert_eq!(
        runner.game.play_option_from_hand(0, fw_idx),
        OptionPlayResult::Pending,
        "BT22-098 [Main] must park on its union-zone free-play selection"
    );
    // Free-play the real Arisa Kinosaki, resolving any follow-up.
    drive_first_valid(&mut runner, 30);
    let after = snapshot(&runner);

    // Both Fable Waltz and the free-played Arisa left hand.
    assert_eq!(
        after.hand[0],
        before.hand[0] - 2,
        "Fable Waltz and the free-played Arisa Kinosaki must both leave hand \
         (before={}, after={})",
        before.hand[0],
        after.hand[0],
    );
    // The real Arisa is on the field.
    assert!(
        field_ids(&runner, 0).contains(&"EX11-060".to_string()),
        "the free-played EX11-060 Arisa Kinosaki must be on the field; field={:?}",
        field_ids(&runner, 0),
    );
    // The free-played Arisa cost NO memory: the only drain is Fable Waltz's own
    // cost (3). If Arisa (cost 4) had been charged too the drop would be 7 —
    // proving "without paying the cost".
    assert_eq!(
        before.memory - after.memory,
        3,
        "only Fable Waltz's own cost (3) may be paid; the free-played Arisa adds no \
         further memory drain (before={}, after={})",
        before.memory,
        after.memory,
    );
    // Fable Waltz placed itself in the battle area as an OnSuspend Delay (not trashed).
    let fw = runner.game.players[0]
        .battle_area
        .iter()
        .find(|p| p.top_card().card_id(&runner.game.card_data) == "BT22-098")
        .expect("Fable Waltz must be placed in the battle area as a Delay option");
    assert!(
        matches!(
            fw.option_state,
            OptionState::Delayed {
                trigger: DelayTrigger::OnEvent(EffectTiming::OnSuspend),
                ..
            }
        ),
        "Fable Waltz must be parked as an OnSuspend Delay; got {:?}",
        fw.option_state
    );
}

/// Shared setup for the C1 full-chain tests: place a Lv.4 [Puppet] base + the
/// real EX11-060 Arisa, play Fable Waltz to arm its `OnSuspend` Delay, then
/// advance PAST the placing turn back to player 0's main phase (so §16-16 no
/// longer blocks the Delay). The EX11-022 Karakurumon ([Puppet]+[LIBERATOR])
/// hand body is the only legal armed-Delay evolution body. Returns the runner
/// and the Arisa handle whose later suspend fires the Delay.
fn combo1_chain_runner() -> (DebugRunner, PermanentHandle) {
    let base = make_puppet("PS1C-BASE", 4, 5000);
    let filler = make_test_card("PS1C-FILL", "Ps1c Fill");

    let mut runner = DebugRunner::builder()
        .dsl_card("BT22-098")
        .expect("BT22-098 (Fable Waltz) in embedded DSL pack")
        .dsl_card("EX11-060")
        .expect("EX11-060 (Arisa Kinosaki) in embedded DSL pack")
        .dsl_card("EX11-022")
        .expect("EX11-022 (Karakurumon) in embedded DSL pack")
        .add_card(base)
        .add_card(filler)
        // Fable Waltz + the EX11-022 Karakurumon Puppet+LIBERATOR evo body in hand.
        .hand(0, &["BT22-098", "EX11-022"])
        .deck(0, &["PS1C-FILL", "PS1C-FILL", "PS1C-FILL", "PS1C-FILL"])
        .deck(1, &["PS1C-FILL"])
        .memory(10)
        .start();
    runner.game.turn_count = 1;

    // A Lv.4 [Puppet] base to receive the Delay's digivolve.
    runner.place_on_field(0, "PS1C-BASE", Some(0));
    // The real Arisa already on field — its later suspend arms the Delay body.
    let arisa = runner.place_on_field(0, "EX11-060", Some(0));
    runner.game.enter_main_phase();

    // [Main]: place Fable Waltz. With no [Shoemon]/[Arisa Kinosaki] in hand or
    // trash to free-play, the union-zone pick has no candidate and the [Main]
    // body resolves; the `kind: delay` clause still seats Fable Waltz as a Delay
    // Option. (`play_option_from_hand` may return `Pending` or `Trashed`; either
    // way we drive to completion and assert the parked-Delay state.)
    let fw_idx = runner.game.players[0]
        .hand
        .iter()
        .position(|c| c.card_id(&runner.game.card_data) == "BT22-098")
        .expect("Fable Waltz in hand");
    let _ = runner.game.play_option_from_hand(0, fw_idx);
    drive_first_valid(&mut runner, 20);
    assert!(
        is_parked_delay(&runner, 0, "BT22-098"),
        "Fable Waltz must be parked as a Delay Option after [Main]"
    );

    // Advance past the placing turn (general_rule.pdf §16-16) back to P0's main.
    runner.end_turn();
    runner.game.enter_main_phase();
    runner.end_turn();
    assert_eq!(runner.game.turn_player(), 0);
    runner.game.enter_main_phase();
    // Ensure memory can cover the −3-reduced digivolve.
    runner.game.set_memory(10);

    (runner, arisa)
}

/// Combo C1 (full chain, happy path) — on a LATER turn the real EX11-060 Arisa
/// suspends, FIRING Fable Waltz's armed `<Delay>`: it pays the activation cost
/// by trashing Fable Waltz (battle area → trash) and opens the armed digivolve,
/// whose base selection offers the [Puppet] base and whose hand-evolution
/// selection offers the only legal body — the real EX11-022 Karakurumon
/// ([Puppet]+[LIBERATOR]). This is the entire point of model combo C1: one card
/// (Fable Waltz) arms a window that a *different* card's later-turn Arisa
/// suspend fires, gated to a Puppet base + a Puppet+LIBERATOR hand card across
/// two turns — the multi-card / multi-turn fact a per-card test cannot see.
///
/// The −3 digivolve *completion into the real DSL EX11-022 body* is split into
/// the `#[ignore]`d test below (a documented DebugRunner-fixture `evo_costs`
/// gap, G-DSL-FIXTURE-EVO-COSTS in `qa/archetype-qa/engine-gaps.md`); the −3
/// *mechanism* is proven in the per-card test
/// `tests/cards_behavioral/bt22/bt22_098.rs` with a synthetic evo body.
#[test]
fn combo1_fable_waltz_delay_fires_on_later_arisa_suspend() {
    use digimon_engine::selection::SelectionKind;

    let (mut runner, arisa) = combo1_chain_runner();

    // Suspend the real Arisa — the gating OnSuspend event for the armed Delay.
    runner.game.suspend(arisa);

    // The armed Delay opens on the Puppet-base digivolve-source selection.
    let base_view = runner
        .pending_selection_view()
        .expect("the later Arisa suspend must fire Fable Waltz's armed Delay base selection");
    assert_eq!(
        base_view.kind,
        SelectionKind::OwnField,
        "the armed Delay opens on the Puppet-base digivolve-source selection"
    );
    let base_pick = base_view
        .valid_action_ids
        .iter()
        .copied()
        .find(|&a| a != PASS)
        .expect("the Lv.4 [Puppet] base must be a legal digivolve source");
    runner
        .execute_action(base_view.selecting_player, base_pick)
        .expect("choose the Puppet base");

    // The hand-evolution selection: the only legal body is the real EX11-022
    // Karakurumon (Puppet+LIBERATOR) — proving the armed body filters correctly.
    let evo_view = runner
        .pending_selection_view()
        .expect("choosing the base must expose the Puppet+LIBERATOR hand-evolution selection");
    assert_eq!(
        evo_view.kind,
        SelectionKind::Hand,
        "the armed Delay then selects the Puppet+LIBERATOR hand body"
    );
    let evo_pick = evo_view
        .valid_action_ids
        .iter()
        .copied()
        .find(|&a| a != PASS)
        .expect("the real EX11-022 Karakurumon must be the legal Puppet+LIBERATOR evolution body");
    runner
        .execute_action(evo_view.selecting_player, evo_pick)
        .expect("choose the EX11-022 evolution body");
    runner.auto_resolve().expect("settle the Delay");

    // Faithful, un-weakened cross-card outcome: the <Delay> activation cost
    // trashed Fable Waltz (battle area → trash), fired by the real Arisa suspend
    // on a later turn — the multi-card / multi-turn chain.
    assert!(
        runner.game.players[0]
            .trash
            .iter()
            .any(|c| c.card_id(&runner.game.card_data) == "BT22-098"),
        "the <Delay> activation cost must trash Fable Waltz when the later Arisa suspend fires it"
    );
    assert!(
        !field_ids(&runner, 0).contains(&"BT22-098".to_string()),
        "Fable Waltz must leave the battle area once the Delay activation cost is paid"
    );
}

/// Combo C1 payoff completion — `#[ignore]`d, gated on a DebugRunner-fixture
/// limitation (NOT a live engine bug).
///
/// FINDING (G-DSL-FIXTURE-EVO-COSTS, `qa/archetype-qa/engine-gaps.md`): a
/// DSL-loaded card built by `DebugRunner`/`dsl_card` has EMPTY
/// `card_data.evo_costs` (`debug_runner.rs::card_data_from_compiled` sets
/// `evo_costs: Vec::new()`; YAML `alt_paths` lower into a separate alt-path
/// registration, not into `evo_costs`). `Game::effect_initiated_digivolve` with
/// `ignore_requirements: false` matches the base against the evo body's
/// `evo_costs`, so a cost-reduced Delay digivolve INTO a real DSL card body
/// (EX11-022 here) finds no matching evo cost and silently no-ops — the body
/// stays in hand. In a production game `card_data` comes from `cards.json`
/// (which DOES carry `evo_costs`), so this is a DebugRunner fixture gap. The −3
/// mechanism itself is proven in `bt22_098` with a synthetic evo body carrying
/// explicit `evo_costs`. Un-ignore once `card_data_from_compiled` backfills
/// `evo_costs` from the compiled `alt_paths` (or from cards.json).
#[test]
#[ignore = "DebugRunner-fixture evo_costs gap (G-DSL-FIXTURE-EVO-COSTS): dsl_card bodies have \
            empty evo_costs, so a cost-reduced effect_initiated_digivolve (ignore_requirements: \
            false) into a real DSL card no-ops. See qa/archetype-qa/engine-gaps.md. Mechanism \
            proven in tests/cards_behavioral/bt22/bt22_098.rs."]
fn combo1_fable_waltz_delay_digivolves_base_into_ex11_022() {
    let (mut runner, arisa) = combo1_chain_runner();
    let before = snapshot(&runner);

    runner.game.suspend(arisa);
    // Base pick → EX11-022 hand pick → cost-reduced digivolve.
    {
        let view = runner.pending_selection_view().expect("Delay base selection");
        let pick = view.valid_action_ids.iter().copied().find(|&a| a != PASS).unwrap();
        runner.execute_action(view.selecting_player, pick).unwrap();
    }
    {
        let view = runner.pending_selection_view().expect("Delay hand-evo selection");
        let pick = view.valid_action_ids.iter().copied().find(|&a| a != PASS).unwrap();
        runner.execute_action(view.selecting_player, pick).unwrap();
    }
    runner.auto_resolve().expect("complete the Delay digivolve");
    let after = snapshot(&runner);

    // The faithful payoff: EX11-022 leaves hand and becomes the base stack's top.
    assert!(
        !hand_has(&runner, 0, "EX11-022"),
        "the EX11-022 evolution body must leave hand via the Delay digivolve"
    );
    assert!(
        field_ids(&runner, 0).contains(&"EX11-022".to_string()),
        "the Puppet base must digivolve into the EX11-022 Karakurumon hand body; field={:?}",
        field_ids(&runner, 0),
    );
    assert_eq!(
        after.hand[0],
        before.hand[0] - 1,
        "the armed Delay consumes exactly the EX11-022 hand body (before={}, after={})",
        before.hand[0],
        after.hand[0],
    );
    // The base remains under the evolution body as a source — the digivolve stacked.
    let evo_stack: Vec<String> = runner.game.players[0]
        .battle_area
        .iter()
        .find(|p| p.top_card().card_id(&runner.game.card_data) == "EX11-022")
        .expect("the digivolved stack must exist")
        .card_sources
        .iter()
        .map(|s| s.card_id(&runner.game.card_data).to_string())
        .collect();
    assert!(
        evo_stack.contains(&"PS1C-BASE".to_string()),
        "the base must remain under the Karakurumon evo body as a source; stack={evo_stack:?}"
    );
}

/// Combo C1 unhappy path (general_rule.pdf §16-16): the `<Delay>` cannot
/// activate on the turn Fable Waltz is placed. Suspending the real EX11-060
/// Arisa — the *named* gating Tamer (the Delay is gated on
/// `event_card_name_contains: "Arisa Kinosaki"`) — on the SAME turn must NOT arm
/// the digivolve. Suspending the named enabler is what makes this exercise
/// §16-16 specifically (a non-Arisa suspend could never fire the Delay for an
/// unrelated reason, so it would not test the placing-turn rule). Fable Waltz
/// stays parked and no selection fires.
#[test]
fn combo1_fable_waltz_delay_does_not_fire_on_placing_turn() {
    let mut anchor = make_plain("PS1B-ANCHOR", 3, 3000);
    anchor.colors = vec![CardColor::Yellow];
    let filler = make_test_card("PS1B-FILL", "Ps1b Fill");

    let mut runner = DebugRunner::builder()
        .dsl_card("BT22-098")
        .expect("BT22-098 (Fable Waltz) in embedded DSL pack")
        .dsl_card("EX11-060")
        .expect("EX11-060 (Arisa Kinosaki) in embedded DSL pack")
        .add_card(anchor)
        .add_card(filler)
        .hand(0, &["BT22-098"])
        .deck(0, &["PS1B-FILL", "PS1B-FILL", "PS1B-FILL", "PS1B-FILL"])
        .deck(1, &["PS1B-FILL"])
        .memory(10)
        .start();
    runner.game.turn_count = 1;

    runner.place_on_field(0, "PS1B-ANCHOR", Some(0));
    // The real Arisa on field whose suspend would (on a LATER turn) arm the Delay.
    let arisa = runner.place_on_field(0, "EX11-060", Some(0));
    runner.game.enter_main_phase();

    let fw_idx = runner.game.players[0]
        .hand
        .iter()
        .position(|c| c.card_id(&runner.game.card_data) == "BT22-098")
        .expect("Fable Waltz in hand");
    // No Shoemon/Arisa in hand or trash to free-play → the union pick has no
    // candidate and the [Main] resolves; the `kind: delay` clause still seats
    // Fable Waltz in the battle area as a Delay Option.
    let _ = runner.game.play_option_from_hand(0, fw_idx);
    drive_first_valid(&mut runner, 30);
    assert!(
        is_parked_delay(&runner, 0, "BT22-098"),
        "Fable Waltz must seat as a Delay Option after [Main]"
    );

    // Suspend the NAMED Arisa on the SAME (placing) turn.
    runner.game.suspend(arisa);

    assert!(
        runner.pending_selection().is_none(),
        "the Fable Waltz <Delay> must NOT fire on its placing turn even on a real Arisa \
         Kinosaki suspend (general_rule.pdf §16-16)"
    );
    assert!(
        is_parked_delay(&runner, 0, "BT22-098"),
        "Fable Waltz must remain parked as a Delay Option on its placing turn"
    );
}

// ─── Combo C2: Vortex Resonance LIBERATOR colour-ignore + -4 digivolve ────────

/// Build a Vortex Resonance runner with an off-colour (Black) board so the
/// Green/Yellow Option is colour-illegal *unless* the LIBERATOR colour-ignore
/// applies. `with_liberator` controls whether the on-field Black anchor carries
/// the LIBERATOR trait (the colour-ignore enabler). Returns the runner and the
/// EX7-074 hand index.
fn combo2_runner(with_liberator: bool) -> (DebugRunner, usize) {
    let mk_lib = |id: &str| -> CardData {
        let mut c = make_puppet(id, 3, 2000);
        c.traits = vec!["Puppet".to_string(), "LIBERATOR".to_string()];
        c
    };
    // A BLACK anchor: Black does not overlap Vortex Resonance's Green/Yellow
    // colours, so ordinary Option colour-matching FAILS off this board. Only the
    // LIBERATOR colour-ignore (when the anchor is LIBERATOR) can make it legal.
    let mut anchor = make_puppet("PS2-ANCHOR", 3, 3000);
    anchor.colors = vec![CardColor::Black];
    anchor.traits = if with_liberator {
        vec!["Puppet".to_string(), "LIBERATOR".to_string()]
    } else {
        vec!["Puppet".to_string()]
    };

    let mut runner = DebugRunner::builder()
        .dsl_card("EX7-074")
        .expect("EX7-074 (Vortex Resonance) in embedded DSL pack")
        .add_card(mk_lib("PS2-LIB-A"))
        .add_card(mk_lib("PS2-LIB-B"))
        .add_card(mk_lib("PS2-LIB-C"))
        .add_card(anchor)
        .hand(0, &["EX7-074"])
        // Three LIBERATOR cards on top of deck so reveal-3 has an addable card.
        .deck(0, &["PS2-LIB-A", "PS2-LIB-B", "PS2-LIB-C"])
        .deck(1, &["PS2-LIB-A"])
        .memory(10)
        .start();
    runner.game.turn_count = 1;
    runner.place_on_field(0, "PS2-ANCHOR", Some(0));
    runner.game.enter_main_phase();

    let vr_idx = runner.game.players[0]
        .hand
        .iter()
        .position(|c| c.card_id(&runner.game.card_data) == "EX7-074")
        .expect("Vortex Resonance in hand");
    (runner, vr_idx)
}

/// Combo C2 (happy branch) — "Vortex Resonance: dig a LIBERATOR, then digivolve
/// into hand for −4, with the LIBERATOR colour-ignore floodgate".
///
/// - Cards: EX7-074 Vortex Resonance (Green/Yellow Option). The colour-ignore
///   source is a real LIBERATOR-trait anchor so the off-colour gate is the sole
///   variable.
/// - Expected mechanical outcome ([Main]): controlling a [LIBERATOR]
///   Digimon/Tamer grants `IgnoreColorRequirement` for this card, so the
///   Green/Yellow Option is playable off a Black (non-Green/Yellow) board.
///   Activating it reveals the top 3, adds 1 [LIBERATOR] card to hand, bottoms
///   the other 2 (deck net −1), then offers the optional cost-−4 digivolve.
/// - Rules/keyword basis: "While you have [LIBERATOR] Digimon/Tamer, ignore this
///   card's colour requirements" (`general_rule.pdf` §4 colour requirement; a
///   `flood_gate` granting `IgnoreColorRequirement`); reveal-add + cost-reduced
///   digivolve. Card text: `cards/ex7/EX7-074.yaml`; DCGO C#:
///   `$BASE_DCGO/Assets/Scripts/CardEffect/EX7/Green/EX7_074.cs`.
///
/// System-level fact: the Option's colour-legality is gated on controlling a
/// LIBERATOR permanent — a board-state dependency a per-card test reading the
/// Option in isolation can't express. Legality is asserted through the action
/// mask (where the colour-ignore is consumed), not the raw play API.
#[test]
fn combo2_vortex_resonance_with_liberator_is_legal_and_reveals_liberator() {
    use digimon_engine::action::build_action_mask;
    use digimon_engine::action::space::PLAY_HAND_START;

    let (mut runner, vr_idx) = combo2_runner(true);

    // Off a Black board, the Green/Yellow Option is mask-legal ONLY because the
    // LIBERATOR colour-ignore applies.
    let mask = build_action_mask(&runner.game, 0);
    assert_eq!(
        mask[(PLAY_HAND_START + vr_idx as u16) as usize],
        1.0,
        "with a LIBERATOR permanent the colour-ignore makes the Green/Yellow Option \
         playable off a Black board"
    );

    let before = snapshot(&runner);
    runner.game.play_option_from_hand(0, vr_idx);
    drive_first_valid(&mut runner, 30);
    let after = snapshot(&runner);

    // Reveal-3: 1 added to hand, 2 returned to bottom → deck net −1.
    assert_eq!(
        after.deck[0],
        before.deck[0] - 1,
        "reveal 3 adds 1 to hand and bottoms 2 → deck net −1 (before={}, after={})",
        before.deck[0],
        after.deck[0],
    );
    // The added card is a revealed LIBERATOR card now in hand.
    assert!(
        runner.game.players[0]
            .hand
            .iter()
            .any(|c| c.card_id(&runner.game.card_data).starts_with("PS2-LIB-")),
        "a revealed LIBERATOR card must be added to hand; hand={:?}",
        runner.game.players[0]
            .hand
            .iter()
            .map(|c| c.card_id(&runner.game.card_data))
            .collect::<Vec<_>>(),
    );
}

/// Combo C2 (unhappy branch): with NO LIBERATOR permanent on player 0's Black
/// board, the colour-ignore does not apply, so the Green/Yellow Option fails
/// ordinary colour-matching and is masked illegal — it cannot be played off the
/// non-Green/Yellow board. The combo's enabler is the LIBERATOR permanent.
#[test]
fn combo2_vortex_resonance_without_liberator_is_masked_illegal_off_color() {
    use digimon_engine::action::build_action_mask;
    use digimon_engine::action::space::PLAY_HAND_START;

    let (runner, vr_idx) = combo2_runner(false);

    let mask = build_action_mask(&runner.game, 0);
    assert_eq!(
        mask[(PLAY_HAND_START + vr_idx as u16) as usize],
        0.0,
        "with no LIBERATOR permanent the colour-ignore is inactive, so the Green/Yellow \
         Option is masked illegal off the Black board"
    );
}

// ─── Combo C3: Karakurumon delete-fodder cheat-digivolve → death-draw loop ────

/// Combo C3 (happy path) — "Karakurumon: delete a Token/other [Puppet] to
/// free-digivolve into a [Puppet] hand card → Kyaromon death-draw".
///
/// - Cards: EX9-032 Karakurumon (the free-tempo engine) + BT22-002 Kyaromon
///   (the death-draw payoff, inherited [Your Turn] Draw 1 on your Token/other-
///   Puppet deletion).
/// - Expected mechanical outcome: Karakurumon's `[On Play]/[When Digivolving]`
///   — driven through Karakurumon's *own* effect — deletes 1 of your Tokens or
///   *other* [Puppet] Digimon as a cost, then free-digivolves into a [Puppet]
///   hand card (cost 0, ignoring requirements). That cost deletion is a Puppet
///   DEATH that fires Kyaromon's inherited "[Your Turn] when your Token/other
///   Puppet is deleted, Draw 1". Diff: the cost-body Puppet leaves the field (→
///   trash), the [Puppet] hand body becomes Karakurumon's new top, controller
///   draws 1 off the death trigger (net hand −1).
/// - Rules/keyword basis: "by [deleting]" cost paid before the reward
///   (`general_rule.pdf` cost-then-effect); deletion-as-cost is a deletion event
///   that fires on-deletion observers (§6 deletion). Card text:
///   `cards/ex9/EX9-032.yaml`, `cards/bt22/BT22-002.yaml`; DCGO C#:
///   `$BASE_DCGO/.../EX9/Yellow/EX9_032.cs`, `.../BT22/Blue/BT22_002.cs`.
///
/// System-level fact: a single Karakurumon play chains a deletion (the Puppet
/// fodder), a recursion enabler (the free digivolve), and card advantage (the
/// Kyaromon draw) in one action — the deletion-as-cost that arms an off-card
/// death observer is invisible to a per-card test of Karakurumon alone.
#[test]
fn combo3_karakurumon_cost_deletion_fires_kyaromon_death_draw() {
    // Carrier holds Kyaromon (BT22-002) as a digivolution source so its
    // inherited [Your Turn] Draw 1 observer is live. Deliberately a NON-Puppet
    // (Beast) carrier so it is NOT a legal Karakurumon cost body — that keeps the
    // Puppet fodder the unique cost target.
    let carrier = make_plain("PS3-CARRIER", 4, 5000);

    let mut runner = DebugRunner::builder()
        .dsl_card("EX9-032")
        .expect("EX9-032 (Karakurumon) in embedded DSL pack")
        .dsl_card("BT22-002")
        .expect("BT22-002 (Kyaromon) in embedded DSL pack")
        .add_card(carrier)
        .add_card(make_puppet("PS3-FODDER", 3, 3000))
        .add_card(make_puppet("PS3-EVO", 6, 9000))
        .add_card(make_test_card("PS3-DRAW", "Ps3 Draw"))
        .hand(0, &["EX9-032", "PS3-EVO"])
        .deck(0, &["PS3-DRAW", "PS3-DRAW"])
        .deck(1, &["PS3-DRAW"])
        .memory(10)
        .start();
    runner.game.turn_count = 1;

    // Kyaromon under a non-Puppet carrier → its inherited death-draw is live.
    runner.place_stack(0, &["BT22-002", "PS3-CARRIER"]);
    // A separate Puppet fodder to pay Karakurumon's deletion cost.
    let fodder = runner.place_on_field(0, "PS3-FODDER", Some(0));

    let before = snapshot(&runner);

    // Play Karakurumon through its real action; its [On Play] effect installs the
    // cost-body selection (Token/other Puppet) and the hand-evo selection.
    let kk_idx = runner.game.players[0]
        .hand
        .iter()
        .position(|c| c.card_id(&runner.game.card_data) == "EX9-032")
        .expect("Karakurumon in hand");
    runner.play(0, kk_idx).expect("play Karakurumon");

    // Cost step: delete the Puppet fodder explicitly (its death fires Kyaromon).
    let cost_view = runner
        .pending_selection_view()
        .expect("Karakurumon [On Play] cost-body selection");
    assert!(
        cost_view.valid_action_ids.contains(&encode_perm(fodder)),
        "the Puppet fodder must be a legal Karakurumon deletion cost"
    );
    runner
        .execute_action(cost_view.selecting_player, encode_perm(fodder))
        .expect("delete the Puppet fodder cost body");
    // Resolve the remaining selections (hand-evo pick, Kyaromon draw prompt …).
    drive_first_valid(&mut runner, 30);
    let after = snapshot(&runner);

    // The Puppet fodder was deleted by Karakurumon's own cost.
    assert!(
        !field_ids(&runner, 0).contains(&"PS3-FODDER".to_string()),
        "Karakurumon's cost must delete the Puppet fodder; field={:?}",
        field_ids(&runner, 0),
    );
    // The Puppet hand body became Karakurumon's new top via the free digivolve.
    assert!(
        field_ids(&runner, 0).contains(&"PS3-EVO".to_string()),
        "Karakurumon must free-digivolve into the Puppet hand body; field={:?}",
        field_ids(&runner, 0),
    );
    // The fodder death fired Kyaromon's inherited Draw 1: a deck-seeded PS3-DRAW
    // (which enters hand ONLY via the death-trigger draw) must now be in hand.
    assert!(
        hand_has(&runner, 0, "PS3-DRAW"),
        "Kyaromon's inherited Draw 1 must fire off the Puppet-fodder death; hand={:?}",
        runner.game.players[0]
            .hand
            .iter()
            .map(|c| c.card_id(&runner.game.card_data))
            .collect::<Vec<_>>(),
    );
    // Net hand: −1 (Karakurumon played) −1 (PS3-EVO consumed) +1 (death draw) = −1.
    assert_eq!(
        after.hand[0],
        before.hand[0] - 1,
        "net hand: −2 consumed, +1 drawn off the death trigger (before={}, after={})",
        before.hand[0],
        after.hand[0],
    );
}

/// Combo C3 unhappy path: with NO Token/other-Puppet fodder besides Karakurumon
/// itself, the deletion cost is unpayable — Karakurumon's [On Play] installs no
/// cost prompt, it plays normally, and no cheat-digivolve / death fan-out
/// occurs. The combo's whole loop is *gated on a payable deletion cost*.
#[test]
fn combo3_without_fodder_no_cheat_digivolve_and_no_death_fanout() {
    let carrier = make_plain("PS3N-CARRIER", 4, 5000);

    let mut runner = DebugRunner::builder()
        .dsl_card("EX9-032")
        .expect("EX9-032 (Karakurumon) in embedded DSL pack")
        .dsl_card("BT22-002")
        .expect("BT22-002 (Kyaromon) in embedded DSL pack")
        .add_card(carrier)
        .add_card(make_puppet("PS3N-EVO", 6, 9000))
        .add_card(make_test_card("PS3N-DRAW", "Ps3n Draw"))
        .hand(0, &["EX9-032", "PS3N-EVO"])
        .deck(0, &["PS3N-DRAW", "PS3N-DRAW"])
        .deck(1, &["PS3N-DRAW"])
        .memory(10)
        .start();
    runner.game.turn_count = 1;

    // Kyaromon's death-draw is live, but there is NO Token/other-Puppet fodder.
    runner.place_stack(0, &["BT22-002", "PS3N-CARRIER"]);

    let before = snapshot(&runner);

    let kk_idx = runner.game.players[0]
        .hand
        .iter()
        .position(|c| c.card_id(&runner.game.card_data) == "EX9-032")
        .expect("Karakurumon in hand");
    runner.play(0, kk_idx).expect("play Karakurumon");

    // EX9-032's clause `condition` preflight gates the effect off when no legal
    // cost body exists: no cost prompt is installed.
    assert!(
        runner.pending_selection().is_none(),
        "with no Token/other-Puppet fodder, Karakurumon installs no cost prompt"
    );
    drive_first_valid(&mut runner, 5);
    let after = snapshot(&runner);

    // No cheat-digivolve: the Puppet hand body stays in hand, Karakurumon is the
    // (now top) played permanent, and no death draw happened.
    assert!(
        hand_has(&runner, 0, "PS3N-EVO"),
        "no cheat-digivolve: the Puppet hand body must remain in hand; hand={:?}",
        runner.game.players[0]
            .hand
            .iter()
            .map(|c| c.card_id(&runner.game.card_data))
            .collect::<Vec<_>>(),
    );
    assert!(
        field_ids(&runner, 0).contains(&"EX9-032".to_string()),
        "Karakurumon plays normally (its own top); field={:?}",
        field_ids(&runner, 0),
    );
    assert!(
        !hand_has(&runner, 0, "PS3N-DRAW"),
        "with no fodder death, Kyaromon's Draw 1 must NOT fire"
    );
    // Net hand: only Karakurumon left hand (PS3N-EVO stays, no draw) → −1.
    assert_eq!(
        after.hand[0],
        before.hand[0] - 1,
        "only Karakurumon leaves hand; no cheat-digivolve consumption, no draw \
         (before={}, after={})",
        before.hand[0],
        after.hand[0],
    );
}

// ─── Combo C4: Overclock self-sacrifice → death-trigger fan-out ───────────────

/// Combo C4 (happy path) — "Overclock + Token loop: extra attack by deleting a
/// Token, the cost deletion fanning out into Kaguyamon removal + Kyaromon draw".
///
/// - Cards: BT22-042 Nyabootmon (the <Overclock> Lv.7 Puppet whose end-of-turn
///   extra attack costs deleting 1 of your Tokens/other [Puppet] Digimon) +
///   EX9-033 Kaguyamon ("[All Turns][OPT] when other Digimon are deleted →
///   delete 1 opponent lowest-level Digimon") + BT22-002 Kyaromon (inherited
///   [Your Turn] Draw 1 on your Token/other-Puppet deletion).
/// - Expected mechanical outcome: driven through the *real* <Overclock> keyword
///   path (`activate_overclock` at end of turn), Nyabootmon's extra-attack cost
///   deletes a [Puppet] fodder — a DEATH event that fans out the controller's
///   death triggers in one shot. With Kaguyamon on field, that death fires its
///   "delete opponent lowest-level Digimon" removal (opp field −1), and
///   Kyaromon's inherited Draw 1 fires off the same Puppet death (hand +1). Diff:
///   own field −1 (the fodder), opponent field −1 (Kaguyamon's lowest-level
///   removal), controller hand +1 (Kyaromon draw).
/// - Rules/keyword basis: <Overclock> (`general_rule.pdf` §16-33) — at end of
///   your turn, by deleting 1 of your Tokens/other [Puppet] Digimon, attack a
///   player without suspending; the keyword cost deletion is a real deletion
///   event that dispatches on-deletion observers. Card text:
///   `cards/bt22/BT22-042.yaml`, `cards/ex9/EX9-033.yaml`,
///   `cards/bt22/BT22-002.yaml`; DCGO C#:
///   `$BASE_DCGO/.../BT22/Yellow/BT22_042.cs`, `.../EX9/Yellow/EX9_033.cs`.
///
/// System-level fact a per-card test misses: a single Overclock self-sacrifice
/// converts into BOTH opponent-board removal (Kaguyamon) AND card advantage
/// (Kyaromon) via the shared death event — exercised through Nyabootmon's own
/// keyword, not a low-level delete helper.
#[test]
fn combo4_overclock_cost_deletion_fans_out_to_kaguyamon_and_kyaromon() {
    // Kyaromon under a non-Puppet carrier so its inherited death-draw is live
    // and the carrier is NOT itself a legal Overclock sacrifice.
    let carrier = make_plain("PS4-CARRIER", 4, 5000);

    let mut runner = DebugRunner::builder()
        .dsl_card("BT22-042")
        .expect("BT22-042 (Nyabootmon) in embedded DSL pack")
        .dsl_card("EX9-033")
        .expect("EX9-033 (Kaguyamon) in embedded DSL pack")
        .dsl_card("BT22-002")
        .expect("BT22-002 (Kyaromon) in embedded DSL pack")
        .add_card(carrier)
        .add_card(make_puppet("PS4-FODDER", 3, 3000))
        .add_card(make_plain("PS4-OPP-L3", 3, 3000))
        .add_card(make_plain("PS4-OPP-L5", 5, 7000))
        .add_card(make_test_card("PS4-DRAW", "Ps4 Draw"))
        .deck(0, &["PS4-DRAW", "PS4-DRAW"])
        .deck(1, &["PS4-DRAW"])
        .memory(20)
        .start();

    let tp = runner.game.turn_player();
    let opp = 1 - tp;
    runner.game.turn_count = 3; // not the first turn; Kyaromon's OPT is fresh

    // Nyabootmon (Overclock body) at field index 0 for the turn player.
    let nyaboot = runner.place_on_field(tp, "BT22-042", Some(0));
    // Kaguyamon — the death-trigger fan-out body.
    runner.place_on_field(tp, "EX9-033", Some(0));
    // Kyaromon's inherited death-draw, live under a non-Puppet carrier.
    runner.place_stack(tp, &["BT22-002", "PS4-CARRIER"]);
    // A Puppet fodder — the Overclock sacrifice.
    let fodder = runner.place_on_field(tp, "PS4-FODDER", Some(0));
    // Opponent has a low- and a high-level Digimon; Kaguyamon removes the lowest.
    runner.place_on_field(opp, "PS4-OPP-L3", Some(0));
    runner.place_on_field(opp, "PS4-OPP-L5", Some(0));

    // Drive into the EndOfTurnAction window (where <Overclock> activates).
    runner.game.end_turn();
    assert_eq!(runner.game.current_phase, GamePhase::EndOfTurnAction);

    let before = snapshot(&runner);

    // Activate Nyabootmon's <Overclock> through its OWN keyword path.
    assert!(
        runner.game.has_keyword(nyaboot, Keyword::Overclock),
        "Nyabootmon must carry <Overclock> to pay the sacrifice cost"
    );
    runner
        .game
        .activate_overclock(nyaboot.index as usize)
        .expect("Nyabootmon <Overclock> with a Puppet sacrifice available");

    // The sacrifice prompt is installed: choose the Puppet fodder (its death is
    // what fans out). Kaguyamon is also a legal Puppet sacrifice, so target the
    // fodder explicitly rather than taking first-valid.
    let sac_view = runner
        .pending_selection_view()
        .expect("Overclock sacrifice selection");
    assert!(
        sac_view.valid_action_ids.contains(&encode_perm(fodder)),
        "the Puppet fodder must be a legal Overclock sacrifice; valid={:?}",
        sac_view.valid_action_ids,
    );
    runner
        .execute_action(sac_view.selecting_player, encode_perm(fodder))
        .expect("sacrifice the Puppet fodder for Overclock");
    // Resolve the death fan-out (Kaguyamon removal pick, Kyaromon draw, then the
    // unsuspended attack onto the opponent).
    drive_first_valid(&mut runner, 40);
    let after = snapshot(&runner);

    // Own field −1: the sacrificed fodder is gone.
    assert_eq!(
        after.field[tp as usize],
        before.field[tp as usize] - 1,
        "the Overclock cost must delete the Puppet fodder (own field −1: before={}, after={})",
        before.field[tp as usize],
        after.field[tp as usize],
    );
    // Opponent field −1: Kaguyamon's death fan-out deletes the lowest-level
    // opponent Digimon.
    assert_eq!(
        after.field[opp as usize],
        before.field[opp as usize] - 1,
        "Kaguyamon's death fan-out must delete 1 opponent Digimon (opp field −1: \
         before={}, after={})",
        before.field[opp as usize],
        after.field[opp as usize],
    );
    // It is specifically the LOWEST-level (L3) opponent Digimon that is removed.
    assert!(
        !field_ids(&runner, opp as usize).contains(&"PS4-OPP-L3".to_string()),
        "the lowest-level opponent Digimon (L3) must be the fan-out victim; opp field={:?}",
        field_ids(&runner, opp as usize),
    );
    assert!(
        field_ids(&runner, opp as usize).contains(&"PS4-OPP-L5".to_string()),
        "the higher-level opponent Digimon (L5) must survive; opp field={:?}",
        field_ids(&runner, opp as usize),
    );
    // Kyaromon's inherited Draw 1 fired off the same Puppet death: a deck-seeded
    // PS4-DRAW (which enters hand ONLY via the draw) must now be in hand.
    assert!(
        hand_has(&runner, tp as usize, "PS4-DRAW"),
        "Kyaromon's inherited Draw 1 must fire off the Overclock self-sacrifice death; hand={:?}",
        runner.game.players[tp as usize]
            .hand
            .iter()
            .map(|c| c.card_id(&runner.game.card_data))
            .collect::<Vec<_>>(),
    );
}

/// Combo C4 unhappy path: with NO Token/other-Puppet fodder besides the
/// Overclock body itself, the keyword's deletion cost cannot be paid —
/// `activate_overclock` rejects with `NoSacrifice`, so no extra attack, no death
/// event, and neither Kaguyamon's removal nor Kyaromon's draw fires. The whole
/// closing line is *gated on a payable sacrifice*.
#[test]
fn combo4_without_fodder_overclock_unpayable_no_fanout() {
    use digimon_engine::game::OverclockError;

    let carrier = make_plain("PS4N-CARRIER", 4, 5000);

    let mut runner = DebugRunner::builder()
        .dsl_card("BT22-042")
        .expect("BT22-042 (Nyabootmon) in embedded DSL pack")
        .dsl_card("EX9-033")
        .expect("EX9-033 (Kaguyamon) in embedded DSL pack")
        .dsl_card("BT22-002")
        .expect("BT22-002 (Kyaromon) in embedded DSL pack")
        .add_card(carrier)
        .add_card(make_plain("PS4N-OPP-L3", 3, 3000))
        .add_card(make_plain("PS4N-OPP-L5", 5, 7000))
        .add_card(make_test_card("PS4N-DRAW", "Ps4n Draw"))
        .deck(0, &["PS4N-DRAW", "PS4N-DRAW"])
        .deck(1, &["PS4N-DRAW"])
        .memory(20)
        .start();

    let tp = runner.game.turn_player();
    let opp = 1 - tp;
    runner.game.turn_count = 3;

    // Nyabootmon at index 0. NOTE: Kaguyamon is a [Puppet] Digimon and would be a
    // legal Overclock sacrifice, so to model "no spare fodder" we omit it here —
    // the only [Puppet] on board is Nyabootmon itself (not a legal 'other' cost).
    let nyaboot = runner.place_on_field(tp, "BT22-042", Some(0));
    runner.place_stack(tp, &["BT22-002", "PS4N-CARRIER"]);
    runner.place_on_field(opp, "PS4N-OPP-L3", Some(0));
    runner.place_on_field(opp, "PS4N-OPP-L5", Some(0));

    // With no sacrifice available, set the EOT <Overclock> window directly —
    // mirroring the per-card Overclock tests (ex7_030.rs).
    runner.game.current_phase = GamePhase::EndOfTurnAction;

    let before = snapshot(&runner);

    // Overclock has no legal sacrifice (no Token / no *other* Puppet Digimon).
    let err = runner
        .game
        .activate_overclock(nyaboot.index as usize)
        .expect_err("Overclock must reject when no sacrifice is available");
    assert_eq!(
        err,
        OverclockError::NoSacrifice,
        "with no spare Token/Puppet fodder, the Overclock cost is unpayable"
    );

    let after = snapshot(&runner);

    // No death event → no fan-out: opponent board untouched, no draw, no field loss.
    assert!(
        runner.pending_selection().is_none(),
        "no sacrifice means no death event, so no fan-out prompt is installed"
    );
    assert_eq!(
        after.field[opp as usize], before.field[opp as usize],
        "no death event means no Kaguyamon removal — opponent board untouched \
         (before={}, after={})",
        before.field[opp as usize],
        after.field[opp as usize],
    );
    assert!(
        !hand_has(&runner, tp as usize, "PS4N-DRAW"),
        "no death event means no Kyaromon draw"
    );
}
