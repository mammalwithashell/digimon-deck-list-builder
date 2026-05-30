//! ST-5 Machine Black — archetype interaction tests.
//!
//! Model: `qa/archetype-qa/st5-machine-black-model.md` (Greymon →
//! Machinedramon Black control wall). Each `#[test]` maps 1:1 to a ranked combo
//! in that model and asserts the *system-level* outcome the combo claims —
//! several real implemented ST-5 DSL cards working together — which no per-card
//! `cards_behavioral/st5/*.rs` test sees in isolation.
//!
//! Combos under test (model "Ranked interactions to test"):
//!   1. Blocker + Tai untap loop (rank 1) — the deck's signature engine.
//!   2. Kapurimon Blocker-DP synergy (rank 2) — `[Your Turn]` ∧ has-Blocker gate.
//!   3. Machinedramon Reboot persistent wall (rank 3) — up-to-2 grant + the
//!      opponent's-unsuspend-phase re-ready + expiry.
//!
//! Cards are loaded from the embedded DSL pack via `dsl_card("ST5-XX")`;
//! synthetic `make_test_card` bodies are used only as neutral filler bodies /
//! attackers. Source priority for expected outcomes: printed text
//! (`cards/st5/<ID>.json`) → `general_rule.pdf` §16 (Blocker §16-4, Reboot
//! §16-10) / §12 (Blocking) → DCGO C# (`DCGO/Assets/Scripts/CardEffect/ST5/Black/`).

#![allow(dead_code)]

use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardKind, EffectTiming, Keyword};
use digimon_engine::selection::{SelectionKind, TriggerSource};

// ─── Combo-piece fixtures ────────────────────────────────────────────────────

/// A neutral attacker for the opponent — small DP, no keywords, just something
/// to declare an attack with so the blocker flow / Tai trigger fires.
fn make_attacker(id: &str, dp: i32) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(dp);
    c
}

/// A neutral Lv.3 body to sit on top of Kapurimon in a stack — gives the
/// carrier a known base DP without any of its own DP / keyword effects, so the
/// only DP swing we measure is Kapurimon's inherited aura.
fn make_lv3_body(id: &str, dp: i32) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(3);
    c.dp = Some(dp);
    c
}

/// A neutral own Digimon to receive Machinedramon's Reboot grant.
fn make_own_digimon(id: &str, dp: i32) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(dp);
    c
}

// ─── Combo A (rank 1): Blocker + Tai untap loop ──────────────────────────────

/// Happy path — the deck's signature engine.
///
/// Combo: **ST5-14 Tai Kamiya** + **ST5-03 Agumon** (printed `<Blocker>`).
/// On the opponent's turn, the opponent attacks; player 0 declares Agumon as a
/// blocker (Agumon **suspends**, the attack redirects to it, §12-1-7-1). The
/// block-suspends-a-Digimon event fires Tai's `[Opponent's Turn]` trigger;
/// player 0 **may** suspend Tai to **unsuspend** Agumon. Expected outcome: after
/// resolving the trigger, Agumon is **unsuspended again** (ready to block a
/// second attack the same turn, §12-1-4) and Tai is suspended (paid the cost).
///
/// The redirected attack resolves as a Digimon-vs-Digimon battle once the Tai
/// selection chain clears (`advance_pending_attack` re-enters at
/// `AttackState::Battle`), so the attacker must be **weaker** than Agumon's
/// printed 1000 DP for the blocker to survive and the "ready for a second block"
/// assertion to be meaningful — mirroring `cards_behavioral/st5/st5_14.rs`'s
/// 12000-DP survivable blocker against its 1000-DP attacker (§14-4: a Digimon
/// with DP ≤ the attacker's is deleted).
///
/// Sources: Blocker `general_rule.pdf` §16-4 / §12-1; Tai `ST5_14.cs`
/// (`OnTappedAnyone` + `IsBlock` + `IsOpponentTurn`, `Mode.UnTap`); YAML
/// `cards/st5/ST5-14.yaml` + `ST5-03.yaml`. Mirrors `cards_behavioral/st5/st5_14.rs`
/// but as a true two-real-card combo with a real printed-Blocker body.
#[test]
fn tai_untap_loop_re_readies_a_blocker_for_a_second_block() {
    let mut runner = DebugRunner::builder()
        .dsl_card("ST5-14")
        .expect("ST5-14 (Tai Kamiya) in embedded DSL pack")
        .dsl_card("ST5-03")
        .expect("ST5-03 (Agumon, printed Blocker) in embedded DSL pack")
        // Weaker than Agumon's printed 1000 DP so the blocked battle leaves
        // Agumon alive (and thus able to block a second attack this turn).
        .add_card(make_attacker("ATTACKER", 500))
        .memory(0)
        .build();
    // Opponent's (player 1's) turn — Tai's trigger is [Opponent's Turn] from
    // player 0's perspective.
    runner.game.turn_count = 1;
    runner.game.turn_player_idx = 1;

    let tai = runner.place_on_field(0, "ST5-14", Some(0));
    let blocker = runner.place_on_field(0, "ST5-03", Some(0));
    let attacker = runner.place_on_field(1, "ATTACKER", Some(0));

    assert!(
        runner.game.has_keyword(blocker, Keyword::Blocker),
        "ST5-03 Agumon carries printed Blocker"
    );

    // Opponent attacks player 0 → player 0 is offered the block declaration.
    let _ = runner.attack_player(attacker, 0, false);
    assert_eq!(
        runner.pending_kind(),
        Some(SelectionKind::OwnField),
        "the defender is offered a Blocker declaration"
    );
    let block_action = runner.pending_selection_view().unwrap().valid_action_ids[0];
    runner
        .execute_action(0, block_action)
        .expect("declare Agumon as the blocker");

    // Blocking suspends the blocker and redirects the attack to it.
    assert!(
        runner.game.players[0].battle_area[blocker.index as usize].is_suspended,
        "declaring a block suspends the blocker (§12-1-7-1)"
    );

    // Tai's optional [Opponent's Turn] response is now offered.
    assert!(
        matches!(
            runner.pending_kind(),
            Some(SelectionKind::Replacement | SelectionKind::TriggerOrder)
        ),
        "Tai's optional blocker-response trigger should be offered, kind={:?}",
        runner.pending_kind()
    );
    runner
        .accept_optional_trigger()
        .expect("accept Tai's suspend-self-to-unsuspend response");

    // Resolve Tai's unsuspend target (the suspended Agumon).
    assert_eq!(
        runner.pending_kind(),
        Some(SelectionKind::OwnField),
        "Tai prompts for which of your Digimon to unsuspend"
    );
    let unsuspend_pick = runner.pending_selection_view().unwrap().valid_action_ids[0];
    runner
        .execute_action(0, unsuspend_pick)
        .expect("unsuspend Agumon with Tai");

    // Net combo outcome: Agumon ready again, Tai paid the cost.
    assert!(
        !runner.game.players[0].battle_area[blocker.index as usize].is_suspended,
        "Tai re-readies Agumon — it can block a second attack this turn"
    );
    assert!(
        runner.game.players[0].battle_area[tai.index as usize].is_suspended,
        "Tai pays the response cost by suspending itself"
    );
}

/// Unhappy path — no Tai (or Tai unavailable) means no re-ready.
///
/// Same board, but Tai cannot pay the suspend cost (it is already suspended),
/// so the `[Opponent's Turn]` trigger does not install. The blocker stays
/// suspended after blocking and therefore cannot block a second attack that turn
/// (§12-1-4: a Digimon that can't suspend can't block). This is the system fact
/// the combo depends on — the re-ready is *gated on Tai* being available.
///
/// As in the happy-path sibling, the attacker is weaker than Agumon's 1000 DP
/// so Agumon survives the redirected battle and its post-block suspended state
/// is observable (a stronger attacker would delete it before we could assert).
#[test]
fn blocker_stays_suspended_without_an_available_tai() {
    let mut runner = DebugRunner::builder()
        .dsl_card("ST5-14")
        .expect("ST5-14 (Tai Kamiya) in embedded DSL pack")
        .dsl_card("ST5-03")
        .expect("ST5-03 (Agumon, printed Blocker) in embedded DSL pack")
        // Weaker than Agumon's printed 1000 DP so the blocked battle leaves
        // Agumon alive, letting us observe it stays suspended without Tai.
        .add_card(make_attacker("ATTACKER", 500))
        .memory(0)
        .build();
    runner.game.turn_count = 1;
    runner.game.turn_player_idx = 1;

    let tai = runner.place_on_field(0, "ST5-14", Some(0));
    let blocker = runner.place_on_field(0, "ST5-03", Some(0));
    let attacker = runner.place_on_field(1, "ATTACKER", Some(0));
    // Tai is already suspended → cannot pay its own suspend cost.
    runner.game.players[0].battle_area[tai.index as usize].is_suspended = true;

    let _ = runner.attack_player(attacker, 0, false);
    assert_eq!(runner.pending_kind(), Some(SelectionKind::OwnField));
    let block_action = runner.pending_selection_view().unwrap().valid_action_ids[0];
    runner
        .execute_action(0, block_action)
        .expect("declare Agumon as the blocker");

    // No Tai response is installed, so the block resolves with the blocker left
    // suspended.
    assert!(
        runner.pending_selection_view().is_none(),
        "an already-suspended Tai cannot pay the cost → no response prompt"
    );
    assert!(
        runner.game.players[0].battle_area[blocker.index as usize].is_suspended,
        "without Tai, Agumon stays suspended and cannot block again this turn"
    );
}

// ─── Combo B (rank 2): Kapurimon Blocker-DP synergy ──────────────────────────

/// **ST5-01 Kapurimon** (egg) under a Blocker body grants **+1000 DP**, but only
/// while (your turn ∧ the bearer has `<Blocker>`).
///
/// Combo: `place_stack(0, ["ST5-01", "ST5-03"])` — Kapurimon's inherited aura
/// (`[Your Turn]` while the bearer has Blocker, +1000 DP) sits under printed-
/// Blocker Agumon. Expected outcome, measured via `effective_dp` (which folds in
/// inherited-source declarative DP auras through `static_dp_aura_bonus`):
///   - on your turn, with Blocker → base + 1000;
///   - on the opponent's turn → base (the `[Your Turn]` gate turns the aura off);
///   - a sibling stack `["ST5-01", <plain body>]` with no Blocker → base
///     (the aura's has-Blocker condition fails).
///
/// Sources: `ST5_01.cs` (`ChangeSelfDPStaticEffect 1000`, `IsOwnerTurn &&
/// HasBlocker`); Blocker §16-4. YAML `cards/st5/ST5-01.yaml`
/// (`scope: inherited`, `active_when: {your_turn, has_keyword: Blocker}`,
/// `dp_modifier: 1000`). The per-card test (`st5_static.rs`) checks
/// `source_dp_contribution`; this asserts the *bearer's combined effective DP*,
/// the value combat actually compares.
#[test]
fn kapurimon_buffs_a_blocker_only_on_your_turn_and_only_with_blocker() {
    let plain_body = make_lv3_body("PLAIN-BODY", 5000);

    let mut runner = DebugRunner::builder()
        .dsl_card("ST5-01")
        .expect("ST5-01 (Kapurimon) in embedded DSL pack")
        .dsl_card("ST5-03")
        .expect("ST5-03 (Agumon, printed Blocker) in embedded DSL pack")
        .add_card(plain_body)
        .memory(0)
        .build();
    // Your turn (player 0). place_stack places on player 0's field.
    runner.game.turn_count = 1;
    runner.game.turn_player_idx = 0;

    // Stack [Kapurimon (inherited aura), Agumon (printed Blocker)].
    let blocker_stack = runner.place_stack(0, &["ST5-01", "ST5-03"]);
    // Sibling stack [Kapurimon, plain Lv3 body] — same egg, no Blocker.
    let plain_stack = runner.place_stack(0, &["ST5-01", "PLAIN-BODY"]);
    runner.game.tick_declarative_effects();

    assert!(
        runner.game.has_keyword(blocker_stack, Keyword::Blocker),
        "ST5-03 body carries printed Blocker — the aura's condition is satisfiable"
    );
    assert!(
        !runner.game.has_keyword(plain_stack, Keyword::Blocker),
        "the plain body has no Blocker — the aura must not apply"
    );

    let agumon_base = runner.game.players[0].battle_area[blocker_stack.index as usize]
        .base_dp(&runner.game.card_data)
        .expect("Agumon base DP");
    let plain_base = runner.game.players[0].battle_area[plain_stack.index as usize]
        .base_dp(&runner.game.card_data)
        .expect("plain body base DP");

    // Your turn + has Blocker → +1000.
    assert_eq!(
        runner.effective_dp(blocker_stack),
        Some(agumon_base + 1000),
        "Kapurimon grants +1000 to a Blocker carrier on your turn"
    );
    // Your turn + no Blocker → base (aura condition fails on has_keyword).
    assert_eq!(
        runner.effective_dp(plain_stack),
        Some(plain_base),
        "Kapurimon does not buff a carrier without Blocker"
    );

    // Opponent's turn → the [Your Turn] gate turns the aura off even though the
    // carrier still has Blocker.
    runner.end_turn();
    runner.game.tick_declarative_effects();
    assert!(
        runner.game.has_keyword(blocker_stack, Keyword::Blocker),
        "Agumon still carries Blocker on the opponent's turn"
    );
    assert_eq!(
        runner.effective_dp(blocker_stack),
        Some(agumon_base),
        "Kapurimon's +1000 is [Your Turn]-only — it turns off on the opponent's turn"
    );
}

// ─── Combo C (rank 3): Machinedramon Reboot persistent wall ──────────────────

/// **ST5-12 Machinedramon** `[When Digivolving]`: up to 2 of your Digimon gain
/// `<Reboot>` until the end of your opponent's next turn — and those Digimon
/// **unsuspend on the opponent's unsuspend phase** (§16-10-4, mandatory).
///
/// Combo: place 2 own Digimon + a Machinedramon body, fire `WhenDigivolving`,
/// select both Digimon. Expected outcome:
///   - both chosen Digimon carry `<Reboot>` after the grant;
///   - after suspending them and passing to the opponent's turn (`end_turn`,
///     which runs the opponent's begin-turn unsuspend phase that consumes
///     Reboot on *your* Digimon), both are **unsuspended again**;
///   - the grant expires at the end of the opponent's next turn (a second
///     `end_turn` back to you), so `has_keyword(Reboot)` is then false.
///
/// Sources: Reboot `general_rule.pdf` §16-10 (unsuspend during the opponent's
/// unsuspend phase, mandatory §16-10-4); `ST5_12.cs` (`GainReboot`, `max 2`,
/// `EffectDuration.UntilOpponentTurnEnd`); YAML `cards/st5/ST5-12.yaml`
/// (`select_count_capped_multi max:2`, `grant_keyword Reboot expiry:
/// end_of_opponents_turn`). Extends `st5_effects.rs`'s grant-present per-card
/// test with the cross-phase unsuspend + expiry that defines the wall.
///
/// This test crosses two turn boundaries (`end_turn` to the opponent, then back
/// to you), so **both** decks are seeded: the default `SkipDraw::FirstPlayerOnly`
/// rule only skips the draw on turn 1, so each player draws at the start of their
/// turn here. With empty decks the first cross-turn `begin_turn` deck-outs the
/// drawing player, sets `game_over`, and freezes turn rotation (a no-op second
/// `end_turn`) — the same hazard the ST-2 Sorrow-Blue test guards against.
#[test]
fn machinedramon_reboot_re_readies_two_digimon_on_the_opponents_turn() {
    let ally_a = make_own_digimon("REBOOT-A", 6000);
    let ally_b = make_own_digimon("REBOOT-B", 6000);
    let deck_filler = make_own_digimon("REBOOT-DECK-FILLER", 3000);

    let mut runner = DebugRunner::builder()
        .dsl_card("ST5-12")
        .expect("ST5-12 (Machinedramon) in embedded DSL pack")
        .add_card(ally_a)
        .add_card(ally_b)
        .add_card(deck_filler)
        // Non-empty decks for both players so the per-turn draws across the two
        // `end_turn` rotations below never deck-out and freeze the turn machine.
        .deck(0, &["REBOOT-DECK-FILLER", "REBOOT-DECK-FILLER"])
        .deck(1, &["REBOOT-DECK-FILLER", "REBOOT-DECK-FILLER"])
        .memory(0)
        .build();
    // Your turn (player 0) — [When Digivolving] resolves on your turn.
    runner.game.turn_count = 1;
    runner.game.turn_player_idx = 0;

    let machinedramon = runner.place_on_field(0, "ST5-12", Some(0));
    let ally_a = runner.place_on_field(0, "REBOOT-A", Some(0));
    let ally_b = runner.place_on_field(0, "REBOOT-B", Some(0));

    // Fire Machinedramon's [When Digivolving] grant.
    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(machinedramon),
    );
    runner.game.drain_effect_queue();

    // Select both allies (up to 2).
    assert_eq!(
        runner.pending_kind(),
        Some(SelectionKind::CountCappedMultiSelect { max: 2, picked: 0 }),
        "Machinedramon offers an up-to-2 Reboot-target selection"
    );
    let first = runner.pending_selection_view().unwrap().valid_action_ids[1];
    runner.execute_action(0, first).expect("pick first Reboot target");
    let second = runner.pending_selection_view().unwrap().valid_action_ids[1];
    runner
        .execute_action(0, second)
        .expect("pick second Reboot target");
    runner.auto_resolve().expect("finish Reboot selection");

    // Both chosen Digimon carry the grant.
    assert!(
        runner.game.has_keyword(ally_a, Keyword::Reboot),
        "first selected Digimon gains Reboot"
    );
    assert!(
        runner.game.has_keyword(ally_b, Keyword::Reboot),
        "second selected Digimon gains Reboot"
    );

    // They blocked/attacked this turn → suspended. Reboot must re-ready them on
    // the opponent's unsuspend phase.
    runner.game.players[0].battle_area[ally_a.index as usize].is_suspended = true;
    runner.game.players[0].battle_area[ally_b.index as usize].is_suspended = true;

    // Pass to the opponent's turn: begin_turn runs the opponent's unsuspend
    // phase, which consumes Reboot on *your* (player 0's) Digimon.
    runner.end_turn();
    assert_eq!(
        runner.game.turn_player(),
        1,
        "control passed to the opponent"
    );
    assert!(
        !runner.game.players[0].battle_area[ally_a.index as usize].is_suspended,
        "Reboot unsuspends the first Digimon on the opponent's unsuspend phase (§16-10-4)"
    );
    assert!(
        !runner.game.players[0].battle_area[ally_b.index as usize].is_suspended,
        "Reboot unsuspends the second Digimon on the opponent's unsuspend phase (§16-10-4)"
    );
    // Grant still present *during* the opponent's turn.
    assert!(
        runner.game.has_keyword(ally_a, Keyword::Reboot),
        "Reboot persists through the opponent's turn"
    );

    // End the opponent's turn → grant expires (end of opponent's next turn).
    runner.end_turn();
    assert_eq!(
        runner.game.turn_player(),
        0,
        "control returned to you"
    );
    assert!(
        !runner.game.has_keyword(ally_a, Keyword::Reboot),
        "Reboot grant expires at the end of the opponent's next turn"
    );
    assert!(
        !runner.game.has_keyword(ally_b, Keyword::Reboot),
        "Reboot grant expires at the end of the opponent's next turn"
    );
}
