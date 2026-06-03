//! Omnimon ACE — archetype interaction tests.
//!
//! Model combos for the "Omnimon ACE" archetype (slug `omnimon_ace`). These
//! exercise the deck's signature multi-card lines the way it actually plays:
//! the Miraculous Mega Knight (BT17-095) Option engine (free Agumon/Gabumon
//! recursion, the L6-leave → Omnimon-from-hand DNA Delay, and the off-turn
//! Tamer security free-play) and the two board-clearing Omnimon payoffs
//! (BT17-078's DNA-gated level-wipe and BT20-102 X-Antibody's single-survivor
//! mass deletion).
//!
//! Every combo loads its NAMED cards via the embedded DSL pack (`dsl_card` /
//! `dsl_builder`); synthetic `make_test_card` is used only for unnamed neutral
//! fillers and removal targets. Each test drives the cards' REAL action paths
//! (`play_option_from_hand` for the Option, the engine's own
//! `effect_initiated_dna_digivolve` DNA primitive, `fire_on_play` for the
//! When-Digivolving body) and asserts the claimed mechanical outcome via a
//! before/after [`BoardSnapshot`] diff.
//!
//! Status (2026-06-02): Combos 2–5 are GREEN. **Combo 1** (the BT17-095 [Main]
//! free-play + Delay-seat) is `#[ignore]`d and BLOCKED on the engine gap
//! **G-OPTION-PLACE-SELF-AS-DELAY-ON-PLAY-PATH** (docs/RUST_ENGINE_GAPS.md): the
//! DSL `place_self_as_delay_option` step does not compose with the real
//! `play_option_from_hand` disposal lifecycle, so on the real play path BT17-095
//! is trashed as a Standard Option instead of seated as a Delay. The two Combo-1
//! tests are retained as the gap's repro; they are NOT routed through the
//! `activate_hand_main` bypass the per-card test uses (that skips the Option
//! lifecycle entirely). Combo 5 drives the BT20-102 own-protect pick
//! deterministically to BT20-102 itself (`drive_protecting_own_slot`) so the
//! "ally deleted, BT20-102 survives" outcome is faithful to "choose 1 of your
//! Digimon" rather than over-specified.
//!
//! Source priority (CLAUDE.md): printed text + `general_rule.pdf` §16
//! (keyword/timing) + DCGO C# outrank the card-text JSON. DCGO refs are cited
//! per-combo at `$BASE_DCGO/Assets/Scripts/CardEffect/<SET>/<COLOR>/<CARD>.cs`.

#![allow(dead_code)]

use digimon_engine::action::space::encode_attack;
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::effect_context::EffectContext;
use digimon_engine::enums::CardKind;
use digimon_engine::permanent::{OptionState, PermanentHandle};
use digimon_engine::selection::{OptionPlayResult, SelectionKind};

use super::support::snapshot;

// ─── Shared fixtures ─────────────────────────────────────────────────────────

/// A neutral opponent Digimon at a chosen level + DP. Used as removal /
/// bottom-deck targets so the Omnimon payoff clauses have something to bite.
/// `name` is an unnamed neutral (never one of the combo-named cards).
fn make_opp_digimon(id: &str, name: &str, level: u8, dp: i32) -> CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = CardKind::Digimon;
    c.level = Some(level);
    c.dp = Some(dp);
    c
}

/// Drive every pending selection by taking the first valid action, bounded so a
/// logic bug surfaces as a loop-exhaustion panic rather than a hang. Errors are
/// swallowed (a rejected action means the engine has nothing more to do).
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

/// Drive the pending selections, but at each prompt prefer the FIRST non-PASS
/// (card) action when one exists — so optional union/hand picks resolve to the
/// eligible card rather than declining. Bounded like `drive_first_valid`.
fn drive_taking_cards(runner: &mut DebugRunner, max_steps: usize) {
    use digimon_engine::action::space::PASS;
    for _ in 0..max_steps {
        let Some(view) = runner.pending_selection_view() else {
            return;
        };
        let player = view.selecting_player;
        let action = view
            .valid_action_ids
            .iter()
            .copied()
            .find(|&a| a != PASS)
            .unwrap_or(PASS);
        if runner.execute_action(player, action).is_err() {
            return;
        }
    }
    panic!("drive_taking_cards exhausted {max_steps} steps without draining the selection queue");
}

/// Drive the pending selections, but whenever an `OwnField` (select-own)
/// prompt appears, deterministically pick the action that targets the
/// permanent currently at `protect_index` on `protect_player`'s battle area
/// (the BT20-102 X-Antibody self-protect choice in Combo 5). All other
/// prompts take the first valid action. Field selections encode each
/// candidate slot as `encode_attack(0, slot)` (see
/// `EffectContext::install_field_selection`), so the action for a given slot
/// is `encode_attack(0, protect_index)`. Bounded like `drive_first_valid`.
///
/// This is required for faithfulness: BT20-102's printed wipe lets the
/// controller protect ANY ONE of their own Digimon — the choice is the
/// player's, so an interaction test that asserts "BT20-102 survives, the
/// unprotected ally is deleted" must DRIVE the own-protect pick to BT20-102
/// itself rather than relying on whichever Digimon a blind resolver offers
/// first (the over-specified mis-authoring the Reviewer flagged).
fn drive_protecting_own_slot(
    runner: &mut DebugRunner,
    protect_player: u8,
    protect_index: u8,
    max_steps: usize,
) {
    use digimon_engine::action::space::PASS;
    let protect_action = encode_attack(0, protect_index as u16);
    for _ in 0..max_steps {
        let Some(view) = runner.pending_selection_view() else {
            return;
        };
        let player = view.selecting_player;
        let action = if view.selecting_player == protect_player
            && matches!(view.kind, SelectionKind::OwnField)
            && view.valid_action_ids.contains(&protect_action)
        {
            // The own-protect prompt: protect BT20-102 itself, deterministically.
            protect_action
        } else {
            view.valid_action_ids.first().copied().unwrap_or(PASS)
        };
        if runner.execute_action(player, action).is_err() {
            return;
        }
    }
    panic!("drive_protecting_own_slot exhausted {max_steps} steps without draining the queue");
}

/// Hand index of `card_id` in `player`'s hand.
fn hand_index_of(runner: &DebugRunner, player: u8, card_id: &str) -> usize {
    runner.game.players[player as usize]
        .hand
        .iter()
        .position(|c| c.card_id(&runner.game.card_data) == card_id)
        .unwrap_or_else(|| panic!("{card_id} must be in player {player}'s hand"))
}

/// True if any of `player`'s battle-area permanents has `card_id` as its top
/// card.
fn field_has_top(runner: &DebugRunner, player: u8, card_id: &str) -> bool {
    runner.game.players[player as usize]
        .battle_area
        .iter()
        .any(|p| p.top_card().card_id(&runner.game.card_data) == card_id)
}

/// The single battle-area permanent on `player`'s side whose top card is
/// `card_id`, as a Delay-Option state matcher.
fn delay_option_present(runner: &DebugRunner, player: u8, card_id: &str) -> bool {
    runner.game.players[player as usize].battle_area.iter().any(|p| {
        p.top_card().card_id(&runner.game.card_data) == card_id
            && matches!(p.option_state, OptionState::Delayed { .. })
    })
}

// ═══════════════════════════════════════════════════════════════════════════
// Combo 1 — Miraculous Mega Knight: free Agumon/Gabumon from trash (recursion)
// ═══════════════════════════════════════════════════════════════════════════

/// Combo: "Miraculous Mega Knight — free Agumon/Gabumon from trash (Option
/// recursion)".
///
/// - Cards: BT17-095 Miraculous Mega Knight (Option, cost 2) + BT17-007 Agumon
///   (the free-played body, seeded in trash).
/// - Expected mechanical outcome: playing BT17-095 via its real Option-play
///   path pays only the Option's own cost (2). Its [Main] offers a
///   `select_union_zone` pick of eligible [Agumon]/[Gabumon] across hand AND
///   trash; picking the TRASH copy moves BT17-007 trash→battle area for 0 cost
///   (no memory paid for the played Digimon). BT17-095 is then seated in the
///   battle area as a Delay-Option permanent (NOT trashed). Board diff:
///   +1 Agumon permanent (origin trash), trash −1, BT17-095 hand→battle as
///   Delay, memory −2 (Option's own cost only).
/// - Rules/keyword basis: "play 1 [Agumon]/[Gabumon] from your hand or trash
///   without paying the cost. Then, place this card in the battle area."
///   union-zone free-play + place-self-as-delay (printed text;
///   `$BASE_DCGO/.../BT17/Red/BT17_095.cs` Clause A — `canNoSelect: true` per
///   zone + `PlaceDelayOptionCards`).
///
/// System-level fact a per-card test misses: the FREE body is a *second* real
/// card recurred from trash for 0 memory while the Option pays only its own
/// cost — the net memory swing is exactly the Option cost, not Option + body.
///
/// BLOCKED — G-OPTION-PLACE-SELF-AS-DELAY-ON-PLAY-PATH (filed 2026-06-02,
/// docs/RUST_ENGINE_GAPS.md). On the REAL `play_option_from_hand` lifecycle the
/// Option is moved into the single-occupancy `pending_option` slot BEFORE the
/// [Main] body runs, so the body's `place_self_as_delay_option` step (which only
/// scans the controller's hand/trash) no-ops; `dispose_option` then trashes the
/// Standard Option. Reproducible: P0 trash does not decrease and no `Delayed`
/// BT17-095 is seated. The per-card test sidesteps this by driving the [Main]
/// via `activate_hand_main` (card stays in hand, no Option disposal lifecycle) —
/// a harness shortcut, not the real play path. An interaction test MUST use the
/// real path, so this is `#[ignore]`d pending the engine fix rather than weakened
/// or routed through the bypass. This body is retained as the gap's repro.
#[test]
#[ignore = "BLOCKED: G-OPTION-PLACE-SELF-AS-DELAY-ON-PLAY-PATH — place_self_as_delay_option \
            no-ops on the real play_option_from_hand path (card is in pending_option, not \
            hand/trash); dispose_option then trashes the Standard Option. See \
            docs/RUST_ENGINE_GAPS.md."]
fn combo1_mega_knight_free_plays_agumon_from_trash_and_seats_as_delay() {
    // A Red anchor Digimon satisfies BT17-095's Red+Blue colour requirement at
    // Option-play time (make_test_card defaults to Red). It is an unnamed
    // neutral, not a combo card.
    let mut anchor = make_test_card("C1-ANCHOR", "C1 Anchor");
    anchor.card_kind = CardKind::Digimon;
    anchor.level = Some(3);
    anchor.dp = Some(3000);
    let filler = make_test_card("C1-FILL", "C1 Fill");

    let mut runner = DebugRunner::builder()
        .dsl_card("BT17-095")
        .expect("BT17-095 (Miraculous Mega Knight) in embedded DSL pack")
        .dsl_card("BT17-007")
        .expect("BT17-007 (Agumon) in embedded DSL pack")
        .add_card(anchor)
        .add_card(filler)
        .hand(0, &["BT17-095"])
        .deck(0, &["C1-FILL"; 4])
        .deck(1, &["C1-FILL"; 4])
        .memory(10)
        .start();
    runner.game.turn_count = 1;

    // Colour anchor on field; seed the real Agumon (BT17-007) ONLY into P0's
    // trash — the union pick must recur it from trash, not hand.
    runner.place_on_field(0, "C1-ANCHOR", Some(0));
    runner.inject_trash(0, "BT17-007");
    runner.game.enter_main_phase();

    let before = snapshot(&runner);
    assert_eq!(
        before.field[0], 1,
        "precondition: P0 has only the colour anchor on field"
    );

    // Play BT17-095 through its REAL Option-play path (pays the cost-2 Option
    // cost, runs the [Main] effect). The [Main] parks on the union-zone pick.
    let gh_idx = hand_index_of(&runner, 0, "BT17-095");
    assert_eq!(
        runner.game.play_option_from_hand(0, gh_idx),
        OptionPlayResult::Pending,
        "BT17-095 [Main] must park on its union-zone (hand/trash) selection"
    );
    // Take the eligible Agumon card at every prompt (rather than declining).
    drive_taking_cards(&mut runner, 20);
    let after = snapshot(&runner);

    // The trash Agumon was recurred onto the field: trash −1, +1 Agumon perm.
    assert_eq!(
        after.trash[0],
        before.trash[0] - 1,
        "the free-played Agumon (BT17-007) must leave P0's trash (before={}, after={})",
        before.trash[0],
        after.trash[0],
    );
    assert!(
        field_has_top(&runner, 0, "BT17-007"),
        "the free-played Agumon (BT17-007) must be on P0's battle area"
    );
    // BT17-095 left hand and seated itself as a Delay-Option permanent (NOT
    // trashed): hand −1, and a Delayed BT17-095 is on the field.
    assert_eq!(
        after.hand[0],
        before.hand[0] - 1,
        "BT17-095 must leave hand (before={}, after={})",
        before.hand[0],
        after.hand[0],
    );
    assert!(
        delay_option_present(&runner, 0, "BT17-095"),
        "BT17-095 must seat itself in the battle area as a Delay-Option permanent"
    );
    assert!(
        runner.game.players[0]
            .trash
            .iter()
            .all(|c| c.card_id(&runner.game.card_data) != "BT17-095"),
        "BT17-095 must NOT go to trash — the printed tail places it in the battle area"
    );
    // Net memory swing is exactly the Option's own cost (2). The free-played
    // Agumon adds no further drain — proving "without paying the cost". If the
    // body had been charged its cost-3, memory would have dropped by 5.
    assert_eq!(
        before.memory - after.memory,
        2,
        "only BT17-095's own cost (2) may be paid; the free Agumon adds no memory drain \
         (before={}, after={})",
        before.memory,
        after.memory,
    );
}

/// Combo 1 unhappy path: declining the OPTIONAL [Agumon]/[Gabumon] union pick
/// (the printed "You may play") plays no body, but the mandatory "Then, place
/// this card in the battle area" tail STILL seats BT17-095 as a Delay-Option.
/// The system-level fact: the Delay arm is set up regardless of whether the
/// free recursion happens.
///
/// BLOCKED — G-OPTION-PLACE-SELF-AS-DELAY-ON-PLAY-PATH (same root cause as the
/// happy-path test above). On the real play path the place-self tail no-ops and
/// `dispose_option` trashes the Standard Option, so BT17-095 ends in TRASH
/// (trash +1) instead of seated as a Delay — reproducible here. `#[ignore]`d
/// pending the engine fix; body retained as the gap's repro. See
/// docs/RUST_ENGINE_GAPS.md.
#[test]
#[ignore = "BLOCKED: G-OPTION-PLACE-SELF-AS-DELAY-ON-PLAY-PATH — on the real play path the \
            mandatory place-self tail no-ops and the Standard Option is trashed (trash +1) \
            instead of seated as a Delay. See docs/RUST_ENGINE_GAPS.md."]
fn combo1_mega_knight_declining_recursion_still_seats_delay() {
    let mut anchor = make_test_card("C1D-ANCHOR", "C1D Anchor");
    anchor.card_kind = CardKind::Digimon;
    anchor.level = Some(3);
    anchor.dp = Some(3000);
    let filler = make_test_card("C1D-FILL", "C1D Fill");

    let mut runner = DebugRunner::builder()
        .dsl_card("BT17-095")
        .expect("BT17-095 (Miraculous Mega Knight) in embedded DSL pack")
        .dsl_card("BT17-007")
        .expect("BT17-007 (Agumon) in embedded DSL pack")
        .add_card(anchor)
        .add_card(filler)
        .hand(0, &["BT17-095"])
        .deck(0, &["C1D-FILL"; 4])
        .deck(1, &["C1D-FILL"; 4])
        .memory(10)
        .start();
    runner.game.turn_count = 1;

    // Colour anchor on field; seed an eligible Agumon in trash so the pick is
    // genuinely OPTIONAL (there IS a target to decline), then enter main.
    runner.place_on_field(0, "C1D-ANCHOR", Some(0));
    runner.inject_trash(0, "BT17-007");
    runner.game.enter_main_phase();

    let before = snapshot(&runner);
    let gh_idx = hand_index_of(&runner, 0, "BT17-095");
    assert_eq!(
        runner.game.play_option_from_hand(0, gh_idx),
        OptionPlayResult::Pending,
        "BT17-095 [Main] must park on its union-zone selection"
    );
    // Decline at every prompt (take PASS first) — `drive_first_valid` takes the
    // first valid action which, for the optional union pick, is the PASS/decline
    // when offered; to be deterministic we PASS the optional pick explicitly.
    {
        let view = runner
            .pending_selection_view()
            .expect("union-zone pick installs");
        let player = view.selecting_player;
        // The optional union pick exposes PASS; decline it.
        runner
            .execute_action(player, digimon_engine::action::space::PASS)
            .expect("decline the optional [Agumon]/[Gabumon] union pick");
    }
    drive_first_valid(&mut runner, 10);
    let after = snapshot(&runner);

    // No body played: the Agumon stays in trash, P0's field holds only BT17-095.
    assert_eq!(
        after.trash[0], before.trash[0],
        "declining the optional pick must leave the Agumon in trash"
    );
    assert!(
        !field_has_top(&runner, 0, "BT17-007"),
        "no Agumon should be on the field when the union pick is declined"
    );
    // The mandatory place-self tail still ran: BT17-095 is seated as a Delay.
    assert!(
        delay_option_present(&runner, 0, "BT17-095"),
        "declining the optional recursion must STILL seat BT17-095 as a Delay-Option \
         (the 'Then, place this card' tail is mandatory)"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Combo 2 — Miraculous Mega Knight Delay: leave-trigger → Omnimon DNA from hand
// ═══════════════════════════════════════════════════════════════════════════

/// Seat the BT17-095 permanent at `handle` as a Delay-Option so its Clause B
/// `when_would_leave_battle_area` replacement can fire (Clause B is gated on
/// `source_is_delayed_option`). `place_on_field` yields a Standard option, so
/// the Delay state is set directly — mirroring the proven per-card helper in
/// `tests/cards_behavioral/bt17/bt17_095.rs`.
fn seat_as_delay_option(runner: &mut DebugRunner, handle: PermanentHandle) {
    let turn = runner.game.turn_count;
    let perm = &mut runner.game.players[handle.player as usize].battle_area[handle.index as usize];
    perm.option_state = OptionState::Delayed {
        owner: handle.player,
        trash_on_turn: turn + 2,
        trigger: digimon_engine::enums::DelayTrigger::EndOfYourNextTurn,
        placed_on_turn: turn,
    };
}

/// Combo: "Miraculous Mega Knight Delay — leave-trigger → Omnimon DNA from
/// hand".
///
/// - Cards: BT17-095 (seated Delay) + BT17-015 WarGreymon (the leaving L6) +
///   BT17-027 MetalGarurumon (L6 hand partner) + BT17-078 Omnimon (the result
///   card in hand).
/// - Expected mechanical outcome: with BT17-095 seated as a Delay, when your
///   own L6 Greymon-/Garurumon-named Digimon would leave the battle area
///   OUTSIDE of a battle, the [Delay] fires: BT17-095 trashes itself (Delay
///   cost), then DNA-digivolves the leaving L6 + an L6 Digimon card in hand
///   into the [Omnimon] card from hand. Board diff: BT17-095 → trash; new
///   Omnimon permanent whose stack = leaving-Digimon stack ++ [hand L6 partner]
///   ++ [Omnimon]; leaving Digimon consumed into the merge (replacement
///   Cancelled, does NOT go to trash); hand −2.
/// - Rules/keyword basis: printed [All Turns] <Delay> leave-watcher + DNA merge
///   with the second material drawn from hand;
///   `$BASE_DCGO/.../BT17/Red/BT17_095.cs` Clause B
///   (`WhenRemoveField` + `!IsByBattle` + `SetJogress`).
///
/// System-level fact a per-card test misses: the Delay observer is armed by
/// BT17-095 already on the board, fires off ANOTHER named card's leave, and
/// pulls TWO further named cards (the hand partner + the Omnimon result) into a
/// single merged permanent — a four-card chain.
#[test]
fn combo2_mega_knight_delay_dna_digivolves_into_omnimon_from_hand() {
    let filler = make_test_card("C2-FILL", "C2 Fill");

    let mut runner = DebugRunner::builder()
        .dsl_card("BT17-095")
        .expect("BT17-095 (Miraculous Mega Knight) in embedded DSL pack")
        .dsl_card("BT17-015")
        .expect("BT17-015 (WarGreymon) in embedded DSL pack")
        .dsl_card("BT17-027")
        .expect("BT17-027 (MetalGarurumon) in embedded DSL pack")
        .dsl_card("BT17-078")
        .expect("BT17-078 (Omnimon) in embedded DSL pack")
        .add_card(filler)
        // The Omnimon result + the L6 MetalGarurumon DNA partner are in hand.
        .hand(0, &["BT17-078", "BT17-027"])
        .deck(0, &["C2-FILL"; 5])
        .deck(1, &["C2-FILL"; 5])
        .memory(10)
        .start();
    runner.game.turn_count = 1;

    // BT17-095 seated as a Delay-Option; the leaving subject is a real L6
    // WarGreymon ("Greymon" in name) on the field.
    let mega = runner.place_on_field(0, "BT17-095", Some(0));
    seat_as_delay_option(&mut runner, mega);
    let wargreymon = runner.place_on_field(0, "BT17-015", None);
    let wargreymon_card = runner.top_card(wargreymon);

    let before = snapshot(&runner);

    // Trigger the leave of the field WarGreymon (own-effect, outside battle).
    runner.game.delete_permanent_with_cause(
        wargreymon,
        digimon_engine::replacement::ReplacementCause::OwnEffect,
    );
    // The optional replacement installs an accept prompt first; accept it, then
    // take the result Omnimon + hand-partner picks.
    drive_taking_cards(&mut runner, 25);
    let after = snapshot(&runner);

    // BT17-095 paid the Delay cost — it is no longer a battle-area permanent.
    assert!(
        !field_has_top(&runner, 0, "BT17-095"),
        "BT17-095 must trash itself (Delay cost) once Clause B fires"
    );
    // A merged Omnimon (BT17-078) permanent now exists, with WarGreymon as one
    // of its digivolution sources.
    let merged = runner.game.players[0]
        .battle_area
        .iter()
        .find(|p| p.top_card().card_id(&runner.game.card_data) == "BT17-078")
        .expect("a merged Omnimon (BT17-078) permanent must exist after the DNA digivolve");
    assert!(
        merged
            .card_sources
            .iter()
            .any(|s| s.handle() == wargreymon_card),
        "the leaving WarGreymon must be a digivolution source under the merged Omnimon"
    );
    assert!(
        merged.card_sources.len() >= 3,
        "merged stack must hold WarGreymon + hand MetalGarurumon + Omnimon result; got {}",
        merged.card_sources.len()
    );
    // The leaving WarGreymon was consumed into the merge — NOT trashed.
    assert!(
        !runner.game.players[0]
            .trash
            .iter()
            .any(|c| c.handle() == wargreymon_card),
        "the leaving WarGreymon must be consumed into the merged Omnimon, not sent to trash"
    );
    // Both hand cards (Omnimon result + L6 partner) left the hand.
    assert_eq!(
        after.hand[0],
        before.hand[0] - 2,
        "the Omnimon result and the L6 DNA partner must both leave hand (before={}, after={})",
        before.hand[0],
        after.hand[0],
    );
}

/// Combo 2 unhappy path: a BATTLE-cause leave does NOT trigger the Delay (the
/// printed "outside of a battle" gate). The WarGreymon leaves normally and no
/// Omnimon merge happens — the system-level fact that the Delay's enabler is
/// the non-battle cause, invisible to a per-card test that only fires one
/// trigger path.
#[test]
fn combo2_mega_knight_delay_does_not_fire_on_battle_leave() {
    use digimon_engine::replacement::ReplacementCause;

    let filler = make_test_card("C2B-FILL", "C2B Fill");

    let mut runner = DebugRunner::builder()
        .dsl_card("BT17-095")
        .expect("BT17-095 (Miraculous Mega Knight) in embedded DSL pack")
        .dsl_card("BT17-015")
        .expect("BT17-015 (WarGreymon) in embedded DSL pack")
        .dsl_card("BT17-027")
        .expect("BT17-027 (MetalGarurumon) in embedded DSL pack")
        .dsl_card("BT17-078")
        .expect("BT17-078 (Omnimon) in embedded DSL pack")
        .add_card(filler)
        .hand(0, &["BT17-078", "BT17-027"])
        .deck(0, &["C2B-FILL"; 5])
        .deck(1, &["C2B-FILL"; 5])
        .memory(10)
        .start();
    runner.game.turn_count = 1;

    let mega = runner.place_on_field(0, "BT17-095", Some(0));
    seat_as_delay_option(&mut runner, mega);
    let wargreymon = runner.place_on_field(0, "BT17-015", None);

    let before = snapshot(&runner);

    // A BATTLE-cause leave: the "outside of a battle" gate must reject it.
    runner
        .game
        .delete_permanent_with_cause(wargreymon, ReplacementCause::Battle);
    drive_first_valid(&mut runner, 10);
    let after = snapshot(&runner);

    // No Omnimon merge; BT17-095 still parked as a Delay; both hand cards remain.
    assert!(
        !field_has_top(&runner, 0, "BT17-078"),
        "a battle-cause leave must NOT trigger the DNA digivolve (outside-of-battle gate)"
    );
    assert!(
        delay_option_present(&runner, 0, "BT17-095"),
        "BT17-095 must remain seated as a Delay — the Delay cost is not paid on a battle leave"
    );
    assert_eq!(
        after.hand[0], before.hand[0],
        "neither hand card may be consumed when the Delay does not fire"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Combo 3 — Miraculous Mega Knight security: off-turn Tamer free-play → hand
// ═══════════════════════════════════════════════════════════════════════════

/// Combo: "Miraculous Mega Knight security — off-turn Tamer free-play, Option to
/// hand".
///
/// - Cards: BT17-095 (in the defender's security) + BT17-081 Tai Kamiya & Matt
///   Ishida (the [Matt Ishida]-named Tamer, free-played from hand or trash).
/// - Expected mechanical outcome: when BT17-095 is checked from security, its
///   inherited [Security] effect free-plays 1 [Tai Kamiya]/[Matt Ishida] Tamer
///   from hand OR trash (origin-zone selectable via the union pick), then adds
///   BT17-095 itself to the HAND (not trash). Board diff: +1 Tamer permanent
///   (origin hand or trash), that source zone −1, BT17-095 security→hand.
/// - Rules/keyword basis: printed inherited [Security] "play 1 card with [Tai
///   Kamiya] or [Matt Ishida] in its name from your hand or trash without
///   paying the cost. Then, add this card to the hand.";
///   `$BASE_DCGO/.../BT17/Red/BT17_095.cs` Clause C
///   (`SecuritySkill` + `AddThisCardToHand`).
///
/// Driven through the REAL combat / security-check path (`attack_player`) — the
/// `enqueue_triggered(SecuritySkill, ..)` shortcut is a no-op for inherited
/// [Security] clauses (PR #490), so this exercises the genuine security flip.
#[test]
fn combo3_mega_knight_security_free_plays_tamer_and_returns_self_to_hand() {
    let mut attacker = make_test_card("C3-ATK", "C3 Attacker");
    attacker.card_kind = CardKind::Digimon;
    attacker.dp = Some(6000);
    let filler = make_test_card("C3-FILL", "C3 Fill");

    let mut runner = DebugRunner::builder()
        .dsl_card("BT17-095")
        .expect("BT17-095 (Miraculous Mega Knight) in embedded DSL pack")
        .dsl_card("BT17-081")
        .expect("BT17-081 (Tai Kamiya & Matt Ishida) in embedded DSL pack")
        .add_card(attacker)
        .add_card(filler)
        .memory(10)
        .deck(0, &["C3-FILL"; 5])
        .deck(1, &["C3-FILL"; 5])
        // Defender (P1) holds the named Tamer in HAND; BT17-095 sits in security.
        .hand(1, &["BT17-081"])
        .security(1, &["BT17-095"])
        .start();
    runner.game.turn_count = 1;

    let attacker_handle = runner.place_on_field(0, "C3-ATK", Some(0));
    let before = snapshot(&runner);
    assert_eq!(before.security[1], 1, "precondition: BT17-095 in P1 security");
    assert_eq!(before.field[1], 0, "precondition: P1 has no Tamer on field");

    // Real attack into the defender's security → BT17-095 is checked and its
    // inherited [Security] clause runs for the defender. Take the eligible
    // Tamer at the union pick, then resolve the add-to-hand tail.
    let _ = runner.attack_player(attacker_handle, 1, false);
    drive_taking_cards(&mut runner, 20);
    let after = snapshot(&runner);

    // The named Tamer (BT17-081) was free-played from the defender's hand.
    assert!(
        field_has_top(&runner, 1, "BT17-081"),
        "the [Matt Ishida] Tamer (BT17-081) must be free-played onto the defender's field"
    );
    assert!(
        runner.game.players[1]
            .hand
            .iter()
            .all(|c| c.card_id(&runner.game.card_data) != "BT17-081"),
        "the free-played Tamer (BT17-081) must have left the defender's hand to enter play"
    );
    // BT17-095 left security and was added to the defender's HAND (not trash).
    assert_eq!(
        after.security[1], 0,
        "BT17-095 must leave the security stack"
    );
    assert!(
        runner.game.players[1]
            .hand
            .iter()
            .any(|c| c.card_id(&runner.game.card_data) == "BT17-095"),
        "BT17-095's mandatory add-to-hand tail must route it to the defender's hand"
    );
    assert!(
        runner.game.players[1]
            .trash
            .iter()
            .all(|c| c.card_id(&runner.game.card_data) != "BT17-095"),
        "BT17-095 must NOT go to trash from a security check — it returns to hand"
    );
}

/// Combo 3 unhappy path: with NO eligible [Tai Kamiya]/[Matt Ishida] card in
/// the defender's hand or trash, the optional Tamer play has no target (nothing
/// is played), but the MANDATORY "Then, add this card to the hand" tail STILL
/// returns BT17-095 to the defender's hand. The system-level fact: the
/// self-recursion is unconditional even when the free play is declined.
#[test]
fn combo3_mega_knight_security_returns_self_to_hand_with_no_tamer() {
    let mut attacker = make_test_card("C3N-ATK", "C3N Attacker");
    attacker.card_kind = CardKind::Digimon;
    attacker.dp = Some(6000);
    let filler = make_test_card("C3N-FILL", "C3N Fill");

    let mut runner = DebugRunner::builder()
        .dsl_card("BT17-095")
        .expect("BT17-095 (Miraculous Mega Knight) in embedded DSL pack")
        .add_card(attacker)
        .add_card(filler)
        .memory(10)
        .deck(0, &["C3N-FILL"; 5])
        .deck(1, &["C3N-FILL"; 5])
        .security(1, &["BT17-095"])
        .start();
    runner.game.turn_count = 1;

    let attacker_handle = runner.place_on_field(0, "C3N-ATK", Some(0));
    assert_eq!(runner.hand_size(1), 0, "precondition: defender hand empty");
    assert_eq!(runner.trash_size(1), 0, "precondition: defender trash empty");

    let _ = runner.attack_player(attacker_handle, 1, false);
    runner.auto_resolve().expect("security selections resolve");

    // Nothing free-played (no eligible Tamer anywhere).
    assert_eq!(
        runner.battle_area_size(1),
        0,
        "no Tamer played from an empty hand and trash"
    );
    // The mandatory tail still added BT17-095 to the defender's hand.
    assert_eq!(runner.security_count(1), 0, "BT17-095 left the security stack");
    assert_eq!(
        runner.hand_size(1),
        1,
        "BT17-095's mandatory add-to-hand tail runs even with no eligible Tamer"
    );
    assert_eq!(
        runner.game.players[1].hand[0].card_id(&runner.game.card_data),
        "BT17-095",
        "the card added to the defender's hand must be BT17-095 itself"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Combo 4 — Omnimon (BT17-078) DNA digivolve: board-level wipe of one level
// ═══════════════════════════════════════════════════════════════════════════

/// Combo: "Omnimon (BT17-078) DNA digivolve — board-level wipe of one level".
///
/// - Cards: BT17-078 Omnimon (payoff) + BT17-015 WarGreymon + BT17-027
///   MetalGarurumon (the DNA materials).
/// - Expected mechanical outcome: DNA-digivolving WarGreymon + MetalGarurumon
///   into BT17-078 fires its [When Digivolving] (DNA path): choose 1 opp
///   Digimon → return ALL opp Digimon of that same level to the bottom of the
///   deck, then delete 1 opp Digimon. Board diff: every opp Digimon at the
///   chosen level leaves field→deck-bottom; one further opp Digimon deleted to
///   trash.
/// - Rules/keyword basis: printed "[On Play][When Digivolving] If DNA
///   digivolving, choose 1 of your opponent's Digimon and return all of your
///   opponent's Digimon with the same level as it to the bottom of the deck.
///   Then, delete 1 of your opponent's Digimon."
///   `$BASE_DCGO/.../BT17/White/BT17_078.cs` (`IsJogress`-gated bottom-deck +
///   delete). The DNA digivolve is driven via the engine's own
///   `effect_initiated_dna_digivolve` primitive (the path BT17-078's DNA
///   alt-path uses), which fires WhenDigivolving with `dna_origin=true`.
///
/// System-level fact: the WHOLE same-level cohort is swept, not just the chosen
/// Digimon — plus a separate delete. A non-DNA play does NOT get this body
/// (asserted in the companion test below).
#[test]
fn combo4_omnimon_dna_digivolve_bottom_decks_chosen_level_then_deletes() {
    // Opponent board: two Lv.5 Digimon (the chosen-level cohort) + one Lv.6
    // (off-level survivor of the bottom-deck sweep, then a delete victim).
    let mut runner = DebugRunner::builder()
        .dsl_card("BT17-078")
        .expect("BT17-078 (Omnimon) in embedded DSL pack")
        .dsl_card("BT17-015")
        .expect("BT17-015 (WarGreymon) in embedded DSL pack")
        .dsl_card("BT17-027")
        .expect("BT17-027 (MetalGarurumon) in embedded DSL pack")
        .add_card(make_opp_digimon("C4-OPP-L5A", "C4 OppL5A", 5, 6000))
        .add_card(make_opp_digimon("C4-OPP-L5B", "C4 OppL5B", 5, 5000))
        .add_card(make_opp_digimon("C4-OPP-L6", "C4 OppL6", 6, 9000))
        .hand(0, &["BT17-078"])
        .deck(0, &["C4-OPP-L6"; 4])
        .deck(1, &["C4-OPP-L6"; 4])
        .memory(10)
        .start();
    runner.game.turn_count = 1;

    // The two DNA materials on P0's field; the Omnimon result card in hand.
    let wargreymon = runner.place_on_field(0, "BT17-015", None);
    let metalgarurumon = runner.place_on_field(0, "BT17-027", None);
    let omnimon_card = runner.game.players[0].hand[hand_index_of(&runner, 0, "BT17-078")].handle();

    // Opponent fields the cohort: 2× Lv.5 + 1× Lv.6.
    runner.place_on_field(1, "C4-OPP-L5A", None);
    runner.place_on_field(1, "C4-OPP-L5B", None);
    runner.place_on_field(1, "C4-OPP-L6", None);

    let before = snapshot(&runner);
    let p1_deck_before = before.deck[1];

    // DNA-digivolve WarGreymon + MetalGarurumon into Omnimon (cost 0, ignore
    // requirements) — the card's real DNA path; fires WhenDigivolving with
    // dna_origin=true.
    let merged = {
        let mut ctx = EffectContext::new(&mut runner.game, omnimon_card, None, 0);
        ctx.effect_initiated_dna_digivolve(wargreymon, metalgarurumon, omnimon_card, 0, true)
    };
    assert!(
        merged.is_some(),
        "the DNA digivolve into BT17-078 Omnimon must succeed"
    );
    // Resolve the When-Digivolving body: choose a Lv.5 anchor, sweep, delete.
    drive_first_valid(&mut runner, 25);
    let after = snapshot(&runner);

    // The chosen-level cohort (both Lv.5) went to the bottom of the opponent's
    // deck; the off-level Lv.6 was then the mandatory delete victim → opponent
    // field is empty.
    assert_eq!(
        after.field[1], 0,
        "all 3 opponent Digimon must leave: 2 same-level bottom-decked + 1 deleted \
         (after field[1]={})",
        after.field[1],
    );
    // Two same-level Digimon were returned to the opponent's deck bottom.
    assert_eq!(
        after.deck[1],
        p1_deck_before + 2,
        "both Lv.5 opponent Digimon must be returned to deck bottom (before={}, after={})",
        p1_deck_before,
        after.deck[1],
    );
    // The delete arm trashed exactly one opponent Digimon (the Lv.6 survivor).
    assert_eq!(
        after.trash[1],
        before.trash[1] + 1,
        "the 'Then, delete 1' arm must trash exactly one opponent Digimon \
         (before={}, after={})",
        before.trash[1],
        after.trash[1],
    );
}

/// Combo 4 unhappy path: BT17-078 played NORMALLY (not via DNA) does NOT get the
/// "If DNA digivolving" body — no bottom-deck, no delete. The system-level fact
/// that the entire payoff is gated on the DNA origin of the digivolve.
#[test]
fn combo4_omnimon_non_dna_play_does_not_fire_level_wipe() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT17-078")
        .expect("BT17-078 (Omnimon) in embedded DSL pack")
        .add_card(make_opp_digimon("C4N-OPP-L5", "C4N OppL5", 5, 6000))
        .add_card(make_opp_digimon("C4N-OPP-L6", "C4N OppL6", 6, 9000))
        .memory(20)
        .deck(0, &["C4N-OPP-L6"; 4])
        .deck(1, &["C4N-OPP-L6"; 4])
        .start();
    runner.game.turn_count = 1;

    let opp_l5 = runner.place_on_field(1, "C4N-OPP-L5", Some(0));
    let _opp_l6 = runner.place_on_field(1, "C4N-OPP-L6", Some(0));
    let _ = opp_l5;
    // Play Omnimon onto the field directly (no DNA digivolve), then fire On Play.
    let omni = runner.place_on_field(0, "BT17-078", None);
    let before = snapshot(&runner);
    runner.fire_on_play(0, omni.index as usize);

    // The DNA gate (`dna_origin: true`) fails for a normal play → no prompt, no
    // board change.
    assert!(
        runner.pending_selection().is_none(),
        "non-DNA BT17-078 must NOT install the level-wipe selection (DNA-origin gate)"
    );
    let after = snapshot(&runner);
    assert_eq!(
        after.field[1], before.field[1],
        "no opponent Digimon may be bottom-decked or deleted on a non-DNA play"
    );
    assert_eq!(
        after.deck[1], before.deck[1],
        "opponent deck must be unchanged on a non-DNA play"
    );
    assert_eq!(
        after.trash[1], before.trash[1],
        "opponent trash must be unchanged on a non-DNA play"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Combo 5 — BT20-102 X-Antibody: single-target mass deletion + bottom
// ═══════════════════════════════════════════════════════════════════════════

/// Combo: "BT20-102 X-Antibody — single-target mass deletion + bottom".
///
/// - Cards: BT20-102 Omnimon (X Antibody) (payoff) + BT17-078 Omnimon (the
///   [Omnimon]-named L6/L7 source beneath it that satisfies the digivolution-
///   cards condition).
/// - Expected mechanical outcome: digivolving into BT20-102 over a Digimon whose
///   digivolution cards contain [Omnimon]/[X Antibody] (here a BT17-078 source)
///   fires its [When Digivolving]: choose 1 of BOTH players' Digimon and delete
///   all OTHER Digimon (yours and opponent's except the chosen one), then return
///   1 opp Digimon to the bottom of the deck. Board diff: table reduced toward
///   the single chosen Digimon per side, one more opp Digimon to deck-bottom.
/// - Rules/keyword basis: printed "[On Play][When Digivolving] If [Omnimon] or
///   [X Antibody] is in this Digimon's digivolution cards, choose 1 of both
///   players' Digimon and delete all other Digimon. Then, return 1 of your
///   opponent's Digimon to the bottom of the deck."
///   `$BASE_DCGO/.../BT20/Blue/BT20_102.cs` (`IsOmniOrXAntiSource` gate +
///   protect-1-per-player wipe + `PutLibraryBottom`).
///
/// System-level fact: the wipe is gated on a NAMED card (BT17-078 Omnimon)
/// sitting in the digivolution stack — driven here by stacking the real
/// BT17-078 under BT20-102, then firing the real When-Digivolving body. The
/// companion test proves the body does NOT fire without an Omnimon/X-Antibody
/// source.
#[test]
fn combo5_x_antibody_with_omnimon_source_wipes_to_single_survivor_and_bottoms() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT20-102")
        .expect("BT20-102 (Omnimon (X Antibody)) in embedded DSL pack")
        .dsl_card("BT17-078")
        .expect("BT17-078 (Omnimon) in embedded DSL pack")
        .add_card(make_opp_digimon("C5-OPP-A", "C5 OppA", 4, 5000))
        .add_card(make_opp_digimon("C5-OPP-B", "C5 OppB", 4, 4000))
        .add_card(make_opp_digimon("C5-OWN-ALLY", "C5 OwnAlly", 5, 6000))
        .memory(10)
        .deck(0, &["C5-OPP-A"; 5])
        .deck(1, &["C5-OPP-A"; 5])
        .start();
    runner.game.turn_count = 1;

    // Opponent fields two Digimon; P0 fields one neutral ally (so the wipe has
    // own-side Digimon to delete too).
    runner.place_on_field(1, "C5-OPP-A", Some(0));
    runner.place_on_field(1, "C5-OPP-B", Some(0));
    let _ally = runner.place_on_field(0, "C5-OWN-ALLY", Some(0));

    // BT20-102 placed ON TOP of a real BT17-078 Omnimon source — the
    // sources-only digivolution-cards condition is genuinely satisfied.
    let xanti = runner.place_stack(0, &["BT17-078", "BT20-102"]);

    let before = snapshot(&runner);
    let p1_deck_before = before.deck[1];
    assert_eq!(before.field[1], 2, "precondition: opponent has 2 Digimon");

    // Fire the real [On Play]/[When Digivolving] body on the BT20-102 carrier.
    runner.fire_on_play(0, xanti.index as usize);
    // Resolve: protect-1 opp pick (first valid), protect-1 OWN pick driven
    // deterministically to BT20-102 itself (its slot is `xanti.index`; the
    // own-protect step runs BEFORE any deletion, so the index is stable), then
    // the for_each wipe and the return-to-bottom pick. Driving the own-protect
    // to BT20-102 is what makes the "ally deleted, BT20-102 survives" outcome
    // deterministic and faithful — the controller legally protects whichever
    // own Digimon they choose (BT20-102.yaml step 2 `select_own_permanent`).
    drive_protecting_own_slot(&mut runner, 0, xanti.index, 25);
    let after = snapshot(&runner);

    // The opponent board is reduced: after protecting 1 and bottom-decking the
    // survivor, the opponent has no Digimon left, and 1 went to deck bottom.
    assert!(
        after.field[1] < before.field[1],
        "the mass deletion must reduce the opponent's board (before={}, after={})",
        before.field[1],
        after.field[1],
    );
    assert_eq!(
        after.deck[1],
        p1_deck_before + 1,
        "exactly 1 opponent Digimon must be returned to the bottom of the deck \
         (before={}, after={})",
        p1_deck_before,
        after.deck[1],
    );
    // We protected BT20-102 itself on P0's side (deterministic, above), so the
    // unprotected neutral ally is the own Digimon deleted by "delete all other
    // Digimon", and BT20-102 survives its own wipe.
    assert!(
        !field_has_top(&runner, 0, "C5-OWN-ALLY"),
        "the unprotected own ally must be deleted by 'delete all other Digimon' \
         (BT20-102 itself was the protected own Digimon)"
    );
    assert!(
        field_has_top(&runner, 0, "BT20-102"),
        "BT20-102 itself (the own Digimon we deterministically protected) must \
         survive its own wipe"
    );
}

/// Combo 5 unhappy path (G-SELF-DIGIVOLUTION-CONTAINS-NAME-SOURCES-ONLY): a
/// BARE BT20-102 with NO Omnimon/X-Antibody in its digivolution SOURCES (its
/// own top-card name is excluded from the sources-only scan) does NOT fire the
/// body — no wipe, no bottom-deck. The system-level fact that the payoff is
/// gated on a named source in the stack.
#[test]
fn combo5_x_antibody_without_omnimon_source_does_not_fire_body() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT20-102")
        .expect("BT20-102 (Omnimon (X Antibody)) in embedded DSL pack")
        .add_card(make_opp_digimon("C5N-OPP-A", "C5N OppA", 4, 5000))
        .add_card(make_opp_digimon("C5N-OPP-B", "C5N OppB", 4, 4000))
        .memory(10)
        .deck(1, &["C5N-OPP-A"; 5])
        .start();
    runner.game.turn_count = 1;

    runner.place_on_field(1, "C5N-OPP-A", Some(0));
    runner.place_on_field(1, "C5N-OPP-B", Some(0));
    // BT20-102 placed BARE (no digivolution source). The carrier's own name
    // "Omnimon (X Antibody)" is excluded from the sources-only scan.
    let xanti = runner.place_on_field(0, "BT20-102", None);

    let before = snapshot(&runner);
    runner.fire_on_play(0, xanti.index as usize);

    assert!(
        runner.pending_selection().is_none(),
        "bare BT20-102 (no Omnimon/X-Antibody source) must NOT install the wipe selection"
    );
    let after = snapshot(&runner);
    assert_eq!(
        after.field[1], before.field[1],
        "no opponent Digimon may be deleted when the digivolution-cards condition fails"
    );
    assert_eq!(
        after.deck[1], before.deck[1],
        "no opponent Digimon may be bottom-decked when the condition fails"
    );
}
