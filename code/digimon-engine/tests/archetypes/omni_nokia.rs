//! Omni Nokia — archetype interaction tests.
//!
//! Model combos: `qa/archetype-qa/Omni Nokia-model.md` (the
//! Agumon/Gabumon → Greymon/Garurumon → DNA-Omnimon line, ramped by the
//! Nokia Shiramine + Tai & Matt tamers and recurred by Miraculous Mega Knight).
//!
//! Each `#[test]` maps 1:1 to a named model combo and asserts the combo's
//! *system-level* mechanical outcome via a before/after `BoardSnapshot` diff —
//! the cross-card facts a per-card behavioral test cannot see. Every card a
//! role — named combo pieces AND neutral fillers / DNA partners / removal
//! targets — is loaded by its real ID through the DSL pack (`dsl_builder` /
//! `dsl_card`), with no synthetic `make_test_card`: real ST Agumon / Greymon /
//! Garurumon / MetalGarurumon / Plesiomon / vanilla Lv4 rookies fill the neutral
//! roles.
//!
//! Source priority (CLAUDE.md): printed text (cards.json) + DCGO C#
//! (`$BASE_DCGO/Assets/Scripts/CardEffect/<SET>/<COLOR>/<CARD_ID>.cs`) outrank
//! the card-text JSON. Each card's YAML header
//! (`code/digimon-engine/cards/<set>/<ID>.yaml`) carries the per-clause DCGO
//! crosscheck these tests rely on.

#![allow(dead_code)]

use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::EffectTiming;
use digimon_engine::permanent::{OptionState, PermanentHandle};
use digimon_engine::replacement::ReplacementCause;
use digimon_engine::selection::TriggerSource;

use super::support::{dsl_builder, snapshot};

// ─── Shared fixtures ─────────────────────────────────────────────────────────

/// Push an `Agumon`/`Gabumon`-named card source directly into a player's trash
/// (the recur source for Miraculous Mega Knight). Returns nothing; the card
/// must already exist in `card_data`.
fn seed_trash(runner: &mut DebugRunner, player: usize, card_id: &str) {
    let data_index = runner
        .game
        .card_data
        .iter()
        .position(|d| d.card_id == card_id)
        .unwrap_or_else(|| panic!("{card_id} must be registered in card_data"));
    let idx = runner.game.next_card_index();
    runner.game.players[player]
        .trash
        .push(CardSource::new(data_index, player as u8, idx));
}

/// Count the controller's Delay-Option permanents (BT17-095 once placed).
fn delay_option_count(runner: &DebugRunner, player: usize) -> usize {
    runner.game.players[player]
        .battle_area
        .iter()
        .filter(|p| matches!(p.option_state, OptionState::Delayed { .. }))
        .count()
}

/// Drain pending selections by taking the first valid action each prompt,
/// bounded so a logic bug surfaces as a panic rather than a hang.
fn drive_first_valid(runner: &mut DebugRunner, max_steps: usize) {
    for _ in 0..max_steps {
        let Some(view) = runner.pending_selection_view() else {
            return;
        };
        let player = view.selecting_player;
        let action = view
            .valid_action_ids
            .first()
            .copied()
            .unwrap_or(digimon_engine::action::space::PASS);
        if runner.execute_action(player, action).is_err() {
            return;
        }
    }
    panic!("drive_first_valid exhausted {max_steps} steps without draining the selection queue");
}

/// Fire a permanent's `[When Digivolving]` timing directly (the proven harness
/// pattern from the per-card tests — `place_on_field` builds the permanent
/// without firing the trigger, so we enqueue it explicitly to model the
/// digivolve).
fn fire_when_digivolving(runner: &mut DebugRunner, handle: PermanentHandle) {
    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(handle),
    );
    runner.game.drain_effect_queue();
}

fn fire_on_play(runner: &mut DebugRunner, handle: PermanentHandle) {
    runner
        .game
        .enqueue_triggered(EffectTiming::OnPlay, TriggerSource::Permanent(handle));
    runner.game.drain_effect_queue();
}

// ═════════════════════════════════════════════════════════════════════════════
// Combo 1 — "Miraculous Mega Knight — recur an Agumon/Gabumon from trash"
//   Cards: BT17-095 (Miraculous Mega Knight, Option), BT22-008 (Agumon).
//
//   BT17-095 [Main]: "You may play 1 [Agumon] or [Gabumon] from your hand or
//   trash without paying the cost. Then, place this card in the battle area."
//
//   System fact: with NO eligible rookie in hand but BT22-008 Agumon in trash,
//   [Main] recurs the trash Agumon into play for 0 memory (own field +1, that
//   card's trash count −1), THEN seats BT17-095 itself as a face-up Delay
//   Option (Delay-Option count +1). A per-card test pins the union-zone pick in
//   isolation; this asserts the full board diff of the recur engine.
//
//   Sources: cards.json BT17-095 / BT22-008;
//   DCGO `$BASE_DCGO/.../BT17/Red/BT17_095.cs` (Clause A: union play free +
//   PlaceDelayOptionCards), `$BASE_DCGO/.../BT22/Red/BT22_008.cs`.
// ═════════════════════════════════════════════════════════════════════════════

/// Happy path: BT22-008 Agumon seeded ONLY in trash (none in hand). BT17-095
/// [Main] free-plays the trash Agumon and places itself as a Delay Option.
#[test]
fn combo1_miraculous_mega_knight_recurs_agumon_from_trash_and_arms_delay() {
    let mut runner = dsl_builder(&["BT17-095", "BT22-008"])
        .hand(0, &["BT17-095"])
        .memory(10)
        .start();
    runner.game.turn_count = 1;

    // BT22-008 Agumon lives ONLY in P0's trash — the recur source.
    seed_trash(&mut runner, 0, "BT22-008");

    let before = snapshot(&runner);
    assert_eq!(
        before.field[0], 0,
        "precondition: no own Digimon on field before the recur"
    );
    assert_eq!(
        delay_option_count(&runner, 0),
        0,
        "precondition: no Delay Option placed yet"
    );

    // Activate BT17-095 [Main] from hand (the card's own OptionMain body — the
    // path the per-card test exercises). It parks on the union-zone pick; drive
    // the single-eligible (trash) pick to completion.
    let gh_idx = runner.game.players[0]
        .hand
        .iter()
        .position(|c| c.card_id(&runner.game.card_data) == "BT17-095")
        .expect("BT17-095 in hand");
    assert!(
        runner.game.activate_hand_main(0, gh_idx),
        "BT17-095 [Main] must fire its OptionMain body"
    );
    drive_first_valid(&mut runner, 20);
    let after = snapshot(&runner);

    // The trash Agumon was recurred to the battle area: trash −1.
    assert_eq!(
        after.trash[0],
        before.trash[0] - 1,
        "the recurred Agumon must leave the trash (before={}, after={})",
        before.trash[0],
        after.trash[0],
    );
    // BT22-008 Agumon is now a battle-area permanent.
    let agumon_on_field = runner.game.players[0]
        .battle_area
        .iter()
        .any(|p| p.top_card().card_id(&runner.game.card_data) == "BT22-008");
    assert!(
        agumon_on_field,
        "BT22-008 Agumon must be on the field after the from-trash recur"
    );
    // BT17-095 placed itself as a face-up Delay Option permanent.
    let gh = runner.game.players[0]
        .battle_area
        .iter()
        .find(|p| p.top_card().card_id(&runner.game.card_data) == "BT17-095")
        .expect("BT17-095 must be placed in the battle area");
    assert!(
        matches!(gh.option_state, OptionState::Delayed { .. }),
        "BT17-095 must be seated as a Delay Option; got {:?}",
        gh.option_state
    );
    // Net own field: Agumon recurred + BT17-095 placed = +2 permanents,
    // exactly one of which is a Delay Option.
    assert_eq!(
        after.field[0],
        before.field[0] + 2,
        "own field must grow by 2 (recurred Agumon + placed BT17-095)"
    );
    assert_eq!(
        delay_option_count(&runner, 0),
        1,
        "exactly one Delay Option (BT17-095) must be placed"
    );
    // The recurred Agumon is played "without paying the cost": activating the
    // OptionMain body recurs it for 0 memory (no rookie cost is charged).
    assert_eq!(
        after.memory, before.memory,
        "the from-trash Agumon must be played for 0 memory (before={}, after={})",
        before.memory, after.memory,
    );
}

/// Eligible rookies in BOTH hand and trash: BT17-095 [Main] must surface a
/// zone choice (no auto-select). With BT22-008 Agumon in both hand and trash,
/// the union-zone pick offers more than one card action — proving the engine
/// exposes the from-hand/from-trash decision rather than auto-collapsing.
#[test]
fn combo1_miraculous_mega_knight_surfaces_zone_choice_with_rookies_in_both_zones() {
    let mut runner = dsl_builder(&["BT17-095", "BT22-008"])
        .hand(0, &["BT17-095", "BT22-008"])
        .memory(10)
        .start();
    runner.game.turn_count = 1;

    // BT22-008 Agumon also in trash — now eligible in BOTH zones.
    seed_trash(&mut runner, 0, "BT22-008");

    let gh_idx = runner.game.players[0]
        .hand
        .iter()
        .position(|c| c.card_id(&runner.game.card_data) == "BT17-095")
        .expect("BT17-095 in hand");
    assert!(
        runner.game.activate_hand_main(0, gh_idx),
        "BT17-095 [Main] must fire its OptionMain body and park on the union-zone pick"
    );

    let view = runner
        .pending_selection_view()
        .expect("union-zone pick installs");
    let card_actions: Vec<u16> = view
        .valid_action_ids
        .iter()
        .copied()
        .filter(|&a| a != digimon_engine::action::space::PASS)
        .collect();
    // Two eligible Agumon (one in hand, one in trash) → the player must be
    // offered both — the zone choice is NOT auto-selected away.
    assert!(
        card_actions.len() >= 2,
        "with an eligible Agumon in BOTH hand and trash, the union pick must \
         surface both options (no auto-select); got {card_actions:?}"
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// Combo 2 — "Mega Knight Delay — off-stack Omnimon when a Lv6 leaves outside
//            battle"
//   Cards: BT17-095 (Delay Option, placed), BT22-013 (WarGreymon, the leaving
//   Lv6 Greymon-named), BT22-015 (Omnimon, the DNA result in hand).
//
//   BT17-095 [All Turns]: "When one of your level 6 Digimon with [Greymon] or
//   [Garurumon] in its name would leave the battle area outside of a battle,
//   <Delay> … That Digimon and a card in the hand may DNA digivolve into a
//   Digimon card with [Omnimon] in its name in the hand."
//
//   System fact: with BT17-095 already seated as a Delay Option (past its
//   placing turn), the off-stack of BT22-013 WarGreymon (Lv6, "WarGreymon"
//   contains "Greymon") OUTSIDE battle pays the Delay cost (BT17-095 trashed:
//   Options −1, trash +1) and DNA-digivolves the leaving WarGreymon + a chosen
//   hand card into BT22-015 Omnimon from hand: a new Omnimon-named Lv7
//   permanent appears with the WarGreymon's stack underneath; the partner card
//   leaves hand. Unhappy paths must NOT fire.
//
//   Sources: cards.json BT17-095 / BT22-013 / BT22-015;
//   DCGO `$BASE_DCGO/.../BT17/Red/BT17_095.cs` (Clause B: WhenRemoveField
//   leave-watcher, !IsByBattle, DNA into Omnimon hand card).
// ═════════════════════════════════════════════════════════════════════════════

/// Seat BT17-095 at `handle` as a Delay Option past its placing turn so its
/// Clause B `when_would_leave_battle_area` replacement can fire (the clause is
/// gated on `source_is_delayed_option`). Mirrors the per-card
/// `seat_as_delay_option` helper in `tests/cards_behavioral/bt17/bt17_095.rs`.
fn seat_bt17_095_as_delay(runner: &mut DebugRunner, handle: PermanentHandle) {
    let turn = runner.game.turn_count;
    let perm = &mut runner.game.players[handle.player as usize].battle_area[handle.index as usize];
    perm.option_state = OptionState::Delayed {
        owner: handle.player,
        trash_on_turn: turn + 2,
        trigger: digimon_engine::enums::DelayTrigger::EndOfYourNextTurn,
        placed_on_turn: turn,
    };
}

/// Build the happy-path board: BT17-095 seated as a Delay Option, BT22-013
/// WarGreymon on field as the leaving Lv6, BT22-015 Omnimon + a neutral Lv6 DNA
/// partner in hand. Returns `(runner, wargreymon_handle, bt17_095_handle)`.
fn combo2_board() -> (DebugRunner, PermanentHandle, PermanentHandle) {
    // ST2-11 MetalGarurumon — a real Blue Lv6 DNA hand partner (the merge ignores
    // requirements, so any real Lv6 Digimon is a faithful partner).
    let mut runner = dsl_builder(&["BT17-095", "BT22-013", "BT22-015", "ST2-11"])
        .hand(0, &["BT22-015", "ST2-11"])
        .memory(10)
        .start();
    runner.game.turn_count = 1;

    let bt17_095 = runner.place_on_field(0, "BT17-095", Some(0));
    seat_bt17_095_as_delay(&mut runner, bt17_095);
    // BT22-013 WarGreymon — Lv6, name contains "Greymon" — the leaving subject.
    let wargreymon = runner.place_on_field(0, "BT22-013", None);
    (runner, wargreymon, bt17_095)
}

/// Happy path: WarGreymon leaving outside battle fires BT17-095's Delay, which
/// trashes BT17-095 (the Delay cost) and DNA-digivolves WarGreymon + the hand
/// partner into BT22-015 Omnimon from hand.
#[test]
fn combo2_mega_knight_delay_offstack_dna_into_omnimon() {
    let (mut runner, wargreymon, _bt17_095) = combo2_board();
    let wargreymon_card = runner.top_card(wargreymon);

    let before = snapshot(&runner);
    assert_eq!(
        delay_option_count(&runner, 0),
        1,
        "precondition: BT17-095 seated as a Delay Option"
    );

    // WarGreymon leaves the battle area outside battle (own effect).
    runner
        .game
        .delete_permanent_with_cause(wargreymon, ReplacementCause::OwnEffect);
    // The optional replacement installs an accept prompt, then the Omnimon +
    // partner selections — drive them all.
    drive_first_valid(&mut runner, 20);
    runner.game.drain_effect_queue();
    let after = snapshot(&runner);

    // A merged Omnimon (BT22-015) permanent now exists with the leaving
    // WarGreymon as one of its digivolution sources.
    let merged = runner.game.players[0]
        .battle_area
        .iter()
        .find(|p| p.top_card().card_id(&runner.game.card_data) == "BT22-015")
        .expect("merged BT22-015 Omnimon permanent must exist after the DNA digivolve");
    assert!(
        merged
            .card_sources
            .iter()
            .any(|s| s.handle() == wargreymon_card),
        "the leaving WarGreymon must be a digivolution source under the merged Omnimon"
    );
    // BT17-095 paid the Delay cost: it is no longer a Delay Option on field and
    // landed in the trash.
    assert_eq!(
        delay_option_count(&runner, 0),
        0,
        "BT17-095 must be trashed (Delay cost paid) — no Delay Option remains"
    );
    let bt17_in_trash = runner.game.players[0]
        .trash
        .iter()
        .any(|c| c.card_id(&runner.game.card_data) == "BT17-095");
    assert!(
        bt17_in_trash,
        "BT17-095 must be in the trash after paying its Delay cost"
    );
    // The hand partner left the hand (consumed into the DNA merge). Hand had
    // {BT22-015, partner}; both are consumed (Omnimon result + partner).
    assert_eq!(
        after.hand[0],
        before.hand[0] - 2,
        "both the Omnimon result and the DNA partner must leave the hand \
         (before={}, after={})",
        before.hand[0],
        after.hand[0],
    );
}

/// Unhappy path A — opponent's Lv6 Greymon leaving must NOT fire BT17-095's
/// Delay (the clause is gated on `replacement_subject_is_mine`). BT17-095 stays
/// a Delay Option; no merge happens.
#[test]
fn combo2_delay_does_not_fire_for_opponents_lv6_greymon() {
    // ST1-11 WarGreymon — a real own/opponent Lv6 "Greymon"-name Digimon (here on
    // the OPPONENT, to prove the owner gate).
    let mut runner = dsl_builder(&["BT17-095", "BT22-015", "ST1-11"])
        .hand(0, &["BT22-015"])
        .memory(10)
        .start();
    runner.game.turn_count = 1;

    let bt17_095 = runner.place_on_field(0, "BT17-095", Some(0));
    seat_bt17_095_as_delay(&mut runner, bt17_095);
    let opp = runner.place_on_field(1, "ST1-11", None);

    runner
        .game
        .delete_permanent_with_cause(opp, ReplacementCause::OwnEffect);
    runner.game.drain_effect_queue();

    assert_eq!(
        delay_option_count(&runner, 0),
        1,
        "opponent's Lv6 leaving must NOT trigger BT17-095's owner-gated Delay"
    );
    let no_omnimon = runner.game.players[0]
        .battle_area
        .iter()
        .all(|p| p.top_card().card_id(&runner.game.card_data) != "BT22-015");
    assert!(
        no_omnimon,
        "no Omnimon may be off-stacked for an opponent's leave"
    );
}

/// Unhappy path B — an OWN Lv6 with a non-Greymon/Garurumon name leaving must
/// NOT fire the Delay (the name filter rejects it). BT17-095 stays a Delay
/// Option.
#[test]
fn combo2_delay_does_not_fire_for_own_lv6_non_matching_name() {
    // ST2-10 Plesiomon — a real own Lv6 Digimon whose name is NOT Greymon/
    // Garurumon (so the name filter rejects it).
    let mut runner = dsl_builder(&["BT17-095", "BT22-015", "ST2-10"])
        .hand(0, &["BT22-015"])
        .memory(10)
        .start();
    runner.game.turn_count = 1;

    let bt17_095 = runner.place_on_field(0, "BT17-095", Some(0));
    seat_bt17_095_as_delay(&mut runner, bt17_095);
    let own = runner.place_on_field(0, "ST2-10", None);

    runner
        .game
        .delete_permanent_with_cause(own, ReplacementCause::OwnEffect);
    runner.game.drain_effect_queue();

    assert_eq!(
        delay_option_count(&runner, 0),
        1,
        "own Lv6 with a non-Greymon/Garurumon name must NOT trigger the Delay"
    );
}

/// Unhappy path C — an own Lv6 Greymon-named Digimon leaving IN BATTLE must NOT
/// fire the Delay (the printed "outside of a battle" filter,
/// `none_of: [replacement_cause: battle]`). BT17-095 stays a Delay Option.
#[test]
fn combo2_delay_does_not_fire_when_lv6_leaves_in_battle() {
    let mut runner = dsl_builder(&["BT17-095", "BT22-013", "BT22-015"])
        .hand(0, &["BT22-015"])
        .memory(10)
        .start();
    runner.game.turn_count = 1;

    let bt17_095 = runner.place_on_field(0, "BT17-095", Some(0));
    seat_bt17_095_as_delay(&mut runner, bt17_095);
    let wargreymon = runner.place_on_field(0, "BT22-013", None);

    // Leave caused BY battle — the "outside of a battle" filter must reject it.
    runner
        .game
        .delete_permanent_with_cause(wargreymon, ReplacementCause::Battle);
    runner.game.drain_effect_queue();

    assert_eq!(
        delay_option_count(&runner, 0),
        1,
        "a Lv6 Greymon leaving IN battle must NOT trigger the outside-battle-gated Delay"
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// Combo 3 — "Nokia free-play into Tai & Matt memory ramp"
//   Cards: BT22-084 (Nokia Shiramine), BT17-081 (Tai Kamiya & Matt Ishida),
//   BT22-008 (Agumon).
//
//   BT22-084 [On Play]/[Start of Main]: "If you have 1 or fewer Digimon, you
//   may play 1 [Agumon] or [Gabumon] from your hand without paying the cost."
//   BT17-081 [All Turns]: "When one of your Digimon is played or digivolves, by
//   suspending this Tamer, if you have a Digimon with [Greymon] in its name,
//   gain 1 memory. If you have a Digimon with [Garurumon] in its name, gain 1
//   memory."
//
//   System fact: Nokia free-plays BT22-008 Agumon (own field +1, 0 memory for
//   the rookie). That play triggers Tai & Matt: suspend the Tamer; gain +1 iff
//   a Greymon-named Digimon is present and +1 more iff a Garurumon-named is
//   present. The gate is on Greymon/Garurumon NAMES — an Agumon/Gabumon alone
//   grants 0. This is the cross-tamer engine the per-card tests can't see.
//
//   Sources: cards.json BT22-084 / BT17-081 / BT22-008;
//   DCGO `$BASE_DCGO/.../BT22/Red/BT22_084.cs`, `.../BT17/Red/BT17_081.cs`.
//
//   NOTE on the BT17-081 double-pay regression (its YAML header, 2026-05-24):
//   two *simultaneous* play/digivolve triggers double-grant memory. These
//   tests drive a SINGLE play event (one Agumon entering), so exactly one
//   Tai & Matt trigger fires — the regression is out of scope here.
// ═════════════════════════════════════════════════════════════════════════════

/// Build the Nokia-ramp board: Nokia + Tai & Matt on field, BT22-008 Agumon in
/// hand. `with_greymon` / `with_garurumon` add an own Greymon/Garurumon-named
/// marker Digimon (the names that gate the memory grant). Returns the runner.
fn combo3_board(with_greymon: bool, with_garurumon: bool) -> DebugRunner {
    // ST1-07 Greymon / ST2-06 Garurumon — real Lv4 cards literally named
    // "Greymon"/"Garurumon" (inherited-only effects; never perturb memory) — the
    // names that gate the Tai & Matt grant.
    let mut b = dsl_builder(&["BT22-084", "BT17-081", "BT22-008", "ST1-07", "ST2-06"])
        .hand(0, &["BT22-008"])
        .memory(5);
    b = b.deck(0, &["BT22-008"]).deck(1, &["BT22-008"]);
    let mut runner = b.start();
    runner.game.turn_count = 1;

    runner.place_on_field(0, "BT22-084", Some(0)); // Nokia Shiramine
    runner.place_on_field(0, "BT17-081", Some(0)); // Tai & Matt
    if with_greymon {
        runner.place_on_field(0, "ST1-07", Some(0)); // Greymon name marker
    }
    if with_garurumon {
        runner.place_on_field(0, "ST2-06", Some(0)); // Garurumon name marker
    }
    runner.game.memory = 5;
    runner
}

/// Free-play the Agumon from hand via Nokia's [On Play]-shape body, modelling
/// the rookie entering the field. Both Tamers are already on field, so the
/// Agumon's entry fires Tai & Matt's [All Turns] play observer. Drives the
/// outer accept + the rookie pick + the Tai & Matt activation.
fn nokia_free_play_agumon(runner: &mut DebugRunner) {
    // Fire Nokia's free-play clause directly (its OnPlay/StartOfMain body).
    let nokia = runner.game.players[0]
        .battle_area
        .iter()
        .position(|p| p.top_card().card_id(&runner.game.card_data) == "BT22-084")
        .map(|i| runner.perm_handle(0, i))
        .expect("Nokia on field");
    runner.game.enqueue_triggered(
        EffectTiming::StartOfYourMainPhase,
        TriggerSource::Permanent(nokia),
    );
    runner.game.drain_effect_queue();
    drive_first_valid(runner, 20);
}

/// No Greymon/Garurumon names present: Nokia free-plays Agumon, Tai & Matt
/// suspends, but the name gate grants 0 memory. Net: rookie played free, Tamer
/// suspended, memory unchanged (Agumon alone is NOT a Greymon/Garurumon name).
#[test]
fn combo3_nokia_freeplay_taimatt_zero_memory_without_named_digimon() {
    let mut runner = combo3_board(false, false);
    let before = snapshot(&runner);

    nokia_free_play_agumon(&mut runner);
    let after = snapshot(&runner);

    // Agumon entered the field for free.
    let agumon_on_field = runner.game.players[0]
        .battle_area
        .iter()
        .any(|p| p.top_card().card_id(&runner.game.card_data) == "BT22-008");
    assert!(
        agumon_on_field,
        "BT22-008 Agumon must be free-played onto the field"
    );
    assert_eq!(
        after.field[0],
        before.field[0] + 1,
        "own field grows by exactly 1 (the free-played Agumon)"
    );
    // Tai & Matt's name gate (Greymon/Garurumon) is NOT met by Agumon alone:
    // no memory is gained from the ramp clause.
    assert_eq!(
        after.memory, before.memory,
        "with no Greymon/Garurumon name present, Tai & Matt grant 0 memory \
         (before={}, after={})",
        before.memory, after.memory,
    );
}

/// A Greymon-named Digimon present: the same free-play now grants exactly +1
/// memory (Greymon arm fires; Garurumon arm does not). This pins the gate on
/// the NAME, not on the rookie that triggered the play.
#[test]
fn combo3_nokia_freeplay_taimatt_plus_one_with_greymon_name() {
    let mut runner = combo3_board(true, false);
    let before = snapshot(&runner);

    nokia_free_play_agumon(&mut runner);
    let after = snapshot(&runner);

    assert_eq!(
        after.memory,
        before.memory + 1,
        "with a Greymon-named Digimon present, Tai & Matt grant +1 memory \
         (before={}, after={})",
        before.memory,
        after.memory,
    );
}

/// Both a Greymon- and a Garurumon-named Digimon present: an own-Digimon play
/// fires Tai & Matt's two independent name arms for +2 memory total.
///
/// SYSTEM NOTE: with both name-markers on field there are already 2 own
/// Digimon, so Nokia's free-play is correctly gated OFF (its printed
/// "1 or fewer Digimon" condition fails — `count_lte: 1`). The +2 ramp is
/// therefore driven by hard-playing the named BT22-008 Agumon (a real
/// own-Digimon play event, the same trigger Nokia's free-play would have
/// produced), with Nokia still on field as the combo's enabler/aura context.
/// BT22-008's printed cost (3) is paid by the hard play; the net memory swing
/// is −3 (Agumon cost) + 2 (Tai & Matt) = −1, isolating the +2 grant.
#[test]
fn combo3_taimatt_plus_two_with_both_names_on_own_digimon_play() {
    let mut runner = combo3_board(true, true);
    // 2 own Digimon present → Nokia's free-play is gated off (count_lte: 1).
    let tm = runner.game.players[0]
        .battle_area
        .iter()
        .position(|p| p.top_card().card_id(&runner.game.card_data) == "BT17-081")
        .map(|i| runner.perm_handle(0, i))
        .expect("Tai & Matt on field");

    let mem_before = runner.memory();
    // Hard-play the named BT22-008 Agumon from hand — a real own-Digimon play
    // event that fires Tai & Matt's [All Turns] observer.
    let agu_idx = runner.game.players[0]
        .hand
        .iter()
        .position(|c| c.card_id(&runner.game.card_data) == "BT22-008")
        .expect("BT22-008 Agumon in hand");
    runner
        .play(0, agu_idx)
        .expect("BT22-008 Agumon plays from hand");
    drive_first_valid(&mut runner, 20);

    // Tai & Matt suspended (the activation cost was paid).
    assert!(
        runner.game.players[0].battle_area[tm.index as usize].is_suspended,
        "Tai & Matt must be suspended after the ramp activation"
    );
    // Net: −3 (BT22-008 printed cost) + 2 (both name arms) = −1.
    assert_eq!(
        mem_before - runner.memory(),
        1,
        "with both Greymon- and Garurumon-named Digimon present, the play nets −1 \
         memory (Agumon cost 3, Tai & Matt +2); before={mem_before}, after={}",
        runner.memory(),
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// Combo 4 — "Nokia +1000 aura + DNA Omnimon swing"
//   Cards: BT22-084 (Nokia Shiramine), BT22-015 (Omnimon).
//
//   BT22-084 [All Turns]: "All your Digimon with [Greymon], [Garurumon] or
//   [Omnimon] in their names get +1000 DP."
//   BT22-015 [On Play]/[When Attacking]: "Delete 1 of your opponent's Digimon
//   with the lowest DP."
//
//   System fact: while Nokia is on field, every own Greymon/Garurumon/Omnimon-
//   named Digimon gets +1000 DP — so BT22-015 Omnimon reads 16000, not its base
//   15000. The aura must NOT touch the opponent's Greymon-named Digimon nor own
//   non-matching Digimon. Paired with BT22-015's own delete-lowest-DP to clear
//   a blocker ahead of the swing.
//
//   Sources: cards.json BT22-084 / BT22-015;
//   DCGO `$BASE_DCGO/.../BT22/Red/BT22_084.cs` (ChangeDPStaticEffect),
//   `.../BT22/Red/BT22_015.cs` (On Play / When Attacking delete-min-DP).
// ═════════════════════════════════════════════════════════════════════════════

/// Aura: Nokia buffs the real BT22-015 Omnimon to 16000, leaves opponent's
/// Greymon and own non-matching Digimon untouched.
#[test]
fn combo4_nokia_aura_buffs_own_omnimon_and_spares_others() {
    // ST1-07 Greymon — a real "Greymon"-name opp Lv4 (DP 4000); ST3-06 Gatomon —
    // a real own Lv4 (DP 5000) whose name matches no aura keyword.
    let mut runner = dsl_builder(&["BT22-084", "BT22-015", "ST1-07", "ST3-06"])
        .memory(15)
        .start();
    runner.game.turn_count = 1;

    runner.place_on_field(0, "BT22-084", Some(0)); // Nokia
    let omnimon = runner.place_on_field(0, "BT22-015", Some(0)); // real Omnimon, base 15000
    let opp = runner.place_on_field(1, "ST1-07", Some(0));
    let veemon = runner.place_on_field(0, "ST3-06", Some(0));
    runner.game.tick_declarative_effects();

    // BT22-015 Omnimon: 15000 base + 1000 Nokia aura = 16000.
    let omni_dp = runner.effective_dp(omnimon).expect("Omnimon has DP");
    assert_eq!(
        omni_dp, 16000,
        "BT22-015 Omnimon (15000) must read 16000 under Nokia's +1000 aura; got {omni_dp}"
    );
    // Opponent's Greymon-named Digimon is NOT buffed (owner: you predicate).
    let opp_dp = runner.effective_dp(opp).expect("Opp Greymon has DP");
    assert_eq!(
        opp_dp, 4000,
        "opponent's Greymon (ST1-07, DP 4000) must NOT receive Nokia's aura; got {opp_dp}"
    );
    // Own non-matching name (Gatomon) is NOT buffed.
    let veemon_dp = runner.effective_dp(veemon).expect("Gatomon has DP");
    assert_eq!(
        veemon_dp, 5000,
        "own Gatomon (DP 5000, no Greymon/Garurumon/Omnimon name) must NOT be buffed; got {veemon_dp}"
    );
}

/// Swing prep: BT22-015's own [On Play] delete-lowest-DP clears the opponent's
/// (only / lowest-DP) blocker ahead of the Omnimon swing — exercised through
/// the card's real trigger, not a low-level delete helper.
#[test]
fn combo4_omnimon_on_play_clears_a_blocker() {
    // ST1-05 Birdramon — a real opp Lv4 (the only / lowest-DP opp Digimon, the
    // delete-lowest-DP victim).
    let mut runner = dsl_builder(&["BT22-084", "BT22-015", "ST1-05"])
        .memory(15)
        .start();
    runner.game.turn_count = 1;

    runner.place_on_field(0, "BT22-084", Some(0)); // Nokia (aura source)
    let omnimon = runner.place_on_field(0, "BT22-015", Some(0));
    runner.place_on_field(1, "ST1-05", None);

    let before = snapshot(&runner);
    fire_on_play(&mut runner, omnimon);
    let _ = runner.auto_resolve();
    let after = snapshot(&runner);

    assert_eq!(
        after.field[1],
        before.field[1] - 1,
        "BT22-015 [On Play] must delete the opponent's lowest-DP blocker \
         (before={}, after={})",
        before.field[1],
        after.field[1],
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// Combo 5 — "WarGreymon/MetalGarurumon branch — free cross-line digivolve"
//   Cards: BT22-013 (WarGreymon), BT22-026 (MetalGarurumon).
//
//   Both share a [When Digivolving] "Activate 1 of the effects below":
//   ・1 of your [Agumon]/[Gabumon] may digivolve into the opposite Lv6 in hand,
//     ignoring digivolution requirements and without paying the cost.
//   ・(BT22-013) Delete opp lowest-DP / (BT22-026) Return opp lowest-level.
//
//   System fact: choosing the digivolve branch lets a named partner digivolve
//   into the opposite Lv6 from hand WITHOUT paying the cost and IGNORING
//   requirements — a second Lv6 enters with no memory spend (the chosen base
//   permanent gains the Lv6 top card; hand −1). Choosing the OTHER branch
//   instead resolves the removal arm. Exactly ONE branch resolves per
//   "Activate 1 of the effects below".
//
//   Sources: cards.json BT22-013 / BT22-026;
//   DCGO `$BASE_DCGO/.../BT22/Red/BT22_013.cs`, `.../BT22/Blue/BT22_026.cs`.
// ═════════════════════════════════════════════════════════════════════════════

/// Digivolve branch: BT22-026 MetalGarurumon [When Digivolving] branch 0 lets
/// an own Agumon digivolve into BT22-013 WarGreymon from hand for free,
/// ignoring requirements. A second Lv6 enters with no memory spend; the Agumon
/// stack's top card becomes WarGreymon; hand −1.
#[test]
fn combo5_metalgarurumon_branch_digivolves_agumon_into_wargreymon_free() {
    // ST1-03 Agumon — a real Lv3 [Agumon]-name base (inherited-only effect).
    let mut runner = dsl_builder(&["BT22-026", "BT22-013", "ST1-03"])
        .hand(0, &["BT22-013"]) // WarGreymon in hand — the cross-line result
        .memory(15)
        .start();
    runner.game.turn_count = 1;

    let agu = runner.place_on_field(0, "ST1-03", None);
    let mg = runner.place_on_field(0, "BT22-026", Some(0));

    let before = snapshot(&runner);
    fire_when_digivolving(&mut runner, mg);
    // Pick branch 0 (the cross-line digivolve), then resolve its sub-picks.
    runner
        .execute_branch(0)
        .expect("pick the cross-line digivolve branch");
    let _ = runner.auto_resolve();
    let after = snapshot(&runner);

    // The Agumon stack's top card is now BT22-013 WarGreymon.
    let agu_top = runner.game.players[0].battle_area[agu.index as usize]
        .top_card()
        .card_id(&runner.game.card_data);
    assert_eq!(
        agu_top, "BT22-013",
        "the own ST1-03 Agumon must have digivolved into WarGreymon (free cross-line)"
    );
    // The WarGreymon left the hand to become the digivolve top.
    assert_eq!(
        after.hand[0],
        before.hand[0] - 1,
        "WarGreymon must leave hand to digivolve onto the Agumon (before={}, after={})",
        before.hand[0],
        after.hand[0],
    );
    // Free, ignore-cost digivolve: no memory was spent on the branch.
    assert_eq!(
        after.memory, before.memory,
        "the cross-line digivolve is free — no memory may be spent (before={}, after={})",
        before.memory, after.memory,
    );
}

/// Removal branch (mutual exclusion): choosing BT22-026's OTHER branch returns
/// the opponent's lowest-level Digimon to hand and does NOT digivolve the
/// Agumon. Exactly one branch resolves per "Activate 1 of the effects below".
#[test]
fn combo5_metalgarurumon_other_branch_bounces_and_leaves_agumon_intact() {
    // ST1-05 Birdramon — a real opp Lv4 (the lowest-level opp Digimon, the bounce
    // victim). ST1-03 Agumon — the own [Agumon] base that must stay intact.
    let mut runner = dsl_builder(&["BT22-026", "BT22-013", "ST1-03", "ST1-05"])
        .hand(0, &["BT22-013"])
        .memory(15)
        .start();
    runner.game.turn_count = 1;

    let agu = runner.place_on_field(0, "ST1-03", None);
    let mg = runner.place_on_field(0, "BT22-026", Some(0));
    runner.place_on_field(1, "ST1-05", None);

    let before = snapshot(&runner);
    fire_when_digivolving(&mut runner, mg);
    runner.execute_branch(1).expect("pick the bounce branch");
    let _ = runner.auto_resolve();
    let after = snapshot(&runner);

    // The opponent Digimon was returned to hand: opp field −1, opp hand +1.
    assert_eq!(
        after.field[1],
        before.field[1] - 1,
        "the bounce branch must return the opp lowest-level Digimon to hand"
    );
    assert_eq!(
        after.hand[1],
        before.hand[1] + 1,
        "the bounced Digimon must land in the opponent's hand"
    );
    // The other branch did NOT resolve: the Agumon is untouched (still Agumon
    // on top), and the WarGreymon stayed in hand.
    let agu_top = runner.game.players[0].battle_area[agu.index as usize]
        .top_card()
        .card_id(&runner.game.card_data);
    assert_eq!(
        agu_top, "ST1-03",
        "the digivolve branch must NOT also resolve — Agumon stays put"
    );
    assert_eq!(
        after.hand[0], before.hand[0],
        "WarGreymon must remain in hand when the removal branch is chosen"
    );
}
