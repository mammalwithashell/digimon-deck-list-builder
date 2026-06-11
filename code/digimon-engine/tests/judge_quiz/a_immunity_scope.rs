//! Cluster A — immunity scope ("affects ME" vs "affects the BATTLE",
//! granted-effect ownership, self-immunity).
//!
//! Questions (see `card-resolution.md`):
//!   Q1  Belphemon: Sleep Mode (BT13-088) [Opp Turn] ends an attack vs
//!       Medusamon (BT24-017) `<Progress>` — judge: YES (Progress guards the
//!       Digimon, not the battle).
//!   Q2  Medusamon (BT24-017) `<Progress>` vs Ice Wall! (EX1-068) "[When
//!       Attacking] lose 2 memory" — judge: NO memory loss.  [READY: both impl]
//!   Q17 Magnamon (X Antibody) (BT16-102) `[When Digivolving]` immunity removes
//!       a Lilithmon (EX6-057)-granted `[EoT] Delete` — judge: NO.
//!   Q18 Quantumon (LM-020) immunity blocks its OWN `<Blast Digivolve>` into
//!       Imperialdramon: Paladin Mode ACE (BT17-077) — judge: NO.
//!   Q28 Gankoomon (X Antibody) (BT20-059) protection beats Dragomon (EX5-060)
//!       "[On Play] don't activate" on Sistermon Ciel (BT6-084) — judge: YES,
//!       plays AND activates.
//!
//! Discover-then-pin: assert the judge answer; a failing assertion is a logged
//! engine gap, never weakened. Scenarios authored under tasks §3.

#![allow(unused_imports)]

use digimon_engine::action::PLAY_HAND_START;
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, EffectSourceKind, EffectTiming, GamePhase};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::{AttackState, AttackTarget, PendingAttack, TriggerSource};

/// A plain Blue Lv4 Digimon (no effects) — used both as the grantor's
/// color-requirement enabler and as a non-`<Progress>` control carrier.
fn blue_digimon(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Blue];
    c.level = Some(4);
    c.dp = Some(4000);
    c.play_cost = 4;
    c
}

// ── Cluster-A immunity-machinery probe (de-risks Q17/Q18/Q28) ────────────────
//
// Q18 needs the rare "immune to ALL Digimon effects INCLUDING your own"
// (Quantumon). `Game::permanent_is_unaffected_by_effect` (game.rs:3468) supports
// `EffectControllerFilter::{Any, OpponentOnly, OwnOnly}`, and a bare
// `CannotBeAffected` (no filter) means unconditional immunity = Any. This probe
// confirms an own-controller effect IS blocked by Any immunity — i.e. the
// self-immunity machinery EXISTS. (Q18 remains BLOCKED-CARD on Quantumon LM-020
// AND needs verification that the `<Blast Digivolve>` path consults
// `can_affect_permanent` on the digivolving Digimon — a separate, card-gated check.)

/// Self-immunity machinery PRESENT: a `CannotBeAffected` (Any) target is
/// unaffected even by its OWN controller's effect. Confirmed via the canonical
/// `permanent_is_unaffected_by_effect` predicate.
#[test]
fn cluster_a_self_immunity_blocks_own_controller_effect() {
    use digimon_engine::enums::{EffectSourceKind, ModifierType};
    let mut r = DebugRunner::builder()
        .add_card(digimon_engine::debug_runner::make_test_card("IMMUNE", "Immune"))
        .memory(10)
        .start();
    let h = r.place_on_field(0, "IMMUNE", Some(0));
    r.game.modifiers.add(
        h,
        digimon_engine::modifiers::ModifierEntry::simple(
            ModifierType::CannotBeAffected,
            0,
            digimon_engine::enums::Expiry::Permanent,
            0,
        ),
    );
    // effect_controller == target.player (0) — the target's OWN side.
    assert!(
        r.game
            .permanent_is_unaffected_by_effect(h, 0, EffectSourceKind::Digimon),
        "unconditional CannotBeAffected (Any) must block even the target's own \
         controller's effect — the machinery Q18 needs exists"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Q2 — Medusamon (BT24-017) `<Progress>` vs Ice Wall! (EX1-068)
// ─────────────────────────────────────────────────────────────────────────────
//
// Board (card-resolution.md Q2):
//   1. Player A has Medusamon (BT24-017) in the battle area — it carries
//      `<Progress>` ("While attacking, your opponent's effects don't affect
//      this Digimon").
//   2. On Player B's turn, Player B plays Ice Wall! (EX1-068): "[Main] All of
//      your opponent's Digimon gain '[When Attacking] lose 2 memory' until the
//      end of their next turn." → Medusamon gains that effect.
//   3. On Player A's turn, Medusamon declares an attack at Player B.
//   4. JUDGE ANSWER: Player A does NOT lose 2 memory. Medusamon's `<Progress>`
//      makes it immune to the opponent's effects while attacking, so the
//      Ice-Wall-granted "[When Attacking] lose 2 memory" does not fire.
//
// DCGO: DCGO/Assets/Scripts/CardEffect/EX1/Blue/EX1_068.cs (UntilOpponentTurnEnd
// grant of an OnAllyAttack `AddMemory(-2)` ActivateClass on each opp Digimon);
// Progress consumption at the opponent-effect mutation points (combat.rs:2667,
// docs/DCGO_KEYWORD_PARITY.md "Progress").
//
// ── DISCOVERY-WAVE FINDING (2026-05-28) ──────────────────────────────────────
// One half is implemented, the other is BLOCKED:
//   • `<Progress>` immunity IS implemented — `Game::progress_excludes(...)`,
//     consumed at the opponent-effect mutation sites (combat.rs:2667).
//   • Ice Wall! (EX1-068)'s `[Main]` grant is OMITTED from its YAML, BLOCKED on
//     `G-DSL-GRANT-TRIGGERED-EFFECT-TO-OPPONENT` (qa/dsl-vocab-gaps.md). The DSL
//     has no verb to install a triggered effect on an opponent's permanent with
//     a turn-scoped expiry. Only EX1-068's `[Security] gain 2 memory` clause is
//     implemented.
// Because the grant is a no-op today, a test that "played Ice Wall then
// attacked and asserted no memory loss" would PASS FOR THE WRONG REASON (the
// effect was never granted — not because Progress blocked it). Per the suite's
// discover-then-pin rule we do NOT write that false-passing test; the scenario
// is `#[ignore]`-blocked on the named primitive instead.

/// Q2 — RESOLVED 2026-05-29 (G-DSL-GRANT-TRIGGERED-EFFECT-TO-OPPONENT closed by
/// change `add-grant-triggered-effect-dsl`): EX1-068's `[Main]` grant is now
/// authored, and the granted-trigger dispatch consults `progress_excludes`, so
/// a `<Progress>` opponent Digimon does not fire the granted "[When Attacking]
/// lose 2 memory" while it attacks. Player 1 (grantor) plays Ice Wall on its
/// turn → Player 0's Medusamon (`<Progress>`) and a vanilla control Digimon
/// both receive the grant → on Player 0's turn each attacks: the vanilla loses
/// 2 memory, Medusamon (Progress) loses none.
#[test]
fn q2_medusamon_progress_blocks_ice_wall_memory_loss() {
    let mut r = DebugRunner::builder()
        .dsl_card("EX1-068")
        .expect("EX1-068 Ice Wall! loads")
        .dsl_card("BT24-017")
        .expect("BT24-017 Medusamon (<Progress>) loads")
        .add_card(blue_digimon("P0BLUE")) // grantor's color-requirement Digimon
        .add_card(blue_digimon("VANILLA")) // non-Progress control carrier (P1)
        .hand(0, &["EX1-068"]) // Player 0 (grantor) holds Ice Wall, plays on turn 0
        .memory(10)
        .start();
    r.skip_mulligan();

    // Grantor (Player 0) needs a Blue Digimon to play the Blue Option.
    let _p0_blue = r.place_on_field(0, "P0BLUE", Some(0));
    // Player 1's board (the GRANTEE side): Medusamon (<Progress>) + a vanilla
    // control Digimon.
    let medusamon = r.place_on_field(1, "BT24-017", Some(0));
    let vanilla = r.place_on_field(1, "VANILLA", Some(0));

    // Player 0 plays Ice Wall! on its own turn (already in Main) → grants every
    // Player-1 Digimon "[When Attacking] lose 2 memory".
    assert_eq!(r.turn_player(), 0, "Player 0 plays Ice Wall on the start turn");
    r.game.decode_action(PLAY_HAND_START, 0);
    assert!(
        r.game.player(0).hand.is_empty(),
        "Ice Wall must leave Player 0's hand (the [Main] play resolved)"
    );

    // Both opponent (Player 1) Digimon received the grant snapshot.
    assert!(
        !r.game
            .modifiers
            .granted_triggered_for_timing(medusamon, EffectTiming::WhenAttacking)
            .is_empty(),
        "Medusamon must carry the Ice-Wall-granted [When Attacking] effect"
    );
    assert!(
        !r.game
            .modifiers
            .granted_triggered_for_timing(vanilla, EffectTiming::WhenAttacking)
            .is_empty(),
        "the vanilla control Digimon must also carry the grant"
    );

    // Control: the non-Progress Digimon is the active attacker → firing
    // WhenAttacking runs the granted body → the turn player loses 2 memory.
    set_active_attacker(&mut r, vanilla);
    r.game.set_memory(5);
    r.game
        .enqueue_triggered(EffectTiming::WhenAttacking, TriggerSource::Permanent(vanilla));
    r.game.drain_effect_queue();
    assert_eq!(
        r.game.memory, 3,
        "a non-Progress carrier firing [When Attacking] runs the granted \
         'lose 2 memory' (control); got {}",
        r.game.memory
    );

    // Q2: the <Progress> carrier is the active attacker → the granted effect is
    // the opponent's effect, and Progress makes Medusamon immune to opponent
    // effects while attacking → the granted clause does NOT fire → NO loss.
    set_active_attacker(&mut r, medusamon);
    r.game.set_memory(5);
    r.game
        .enqueue_triggered(EffectTiming::WhenAttacking, TriggerSource::Permanent(medusamon));
    r.game.drain_effect_queue();
    assert_eq!(
        r.game.memory, 5,
        "Medusamon's <Progress> blocks the Ice-Wall-granted memory loss while \
         it is attacking (judge-quiz Q2: NO memory loss); got {}",
        r.game.memory
    );
}

/// Mark `attacker` as the active attacker via a minimal `PendingAttack` so
/// `progress_excludes` engages — the same staging the `progress_mutation_gates`
/// combat tests use to exercise the Progress gate without driving the full
/// attack state machine.
fn set_active_attacker(r: &mut DebugRunner, attacker: PermanentHandle) {
    let target = AttackTarget::Player(if attacker.player == 0 { 1 } else { 0 });
    r.game.pending_attack = Some(PendingAttack {
        attacker,
        original_target: target,
        effective_target: target,
        is_blocked: false,
        blocker: None,
        is_vortex: false,
        is_overclock: false,
        declaration_committed: true,
        cancelled: false,
        battle_occurred: false,
        return_phase: GamePhase::Main,
        state: AttackState::Declared,
        counter_depth: 0,
    });
}

// ── Q1 / Q17 / Q18 / Q28 — BLOCKED-CARD (cards not yet implemented) ───────────
// Each scenario's faithful staging needs ≥1 unimplemented card (see
// card-resolution.md §"Implementation status"). Stubs assert the judge answer in
// their docstring and are `#[ignore]`-d on the specific missing card(s); promote
// to a real test once authored (cluster-A authoring, tasks §3).

/// Q1 — Belphemon: Sleep Mode (BT13-088) [Opp Turn] CAN end Medusamon's
/// (BT24-017) attack; `<Progress>` guards the Digimon, not the battle. Judge: YES.
///
/// Board (card-resolution.md Q1): Player 0 controls Belphemon: Sleep Mode
/// (BT13-088) with 2 cards in hand. On Player 1's turn, Player 1's Medusamon
/// (BT24-017, carrying `<Progress>`) attacks Player 0's security. Belphemon's
/// `[Opponent's Turn] [Once Per Turn]` clause "trash 2 cards in hand to end the
/// attack" activates. `<Progress>` makes Medusamon unaffected by the opponent's
/// (Player 0's) effects while it attacks — but "end the attack" ends the BATTLE
/// (it cancels the pending attack; it does not target / affect Medusamon), so
/// Belphemon CAN end the attack.
///
/// What this pins (so it cannot pass for the wrong reason):
///   1. Medusamon really carries `<Progress>` (declarative keyword, materialized
///      via `tick_declarative_effects`).
///   2. Progress is genuinely IN SCOPE at the moment Belphemon's effect fires —
///      `progress_excludes(medusamon, Some(0))` is `true`, i.e. an opponent
///      (Player 0) effect that *affected* Medusamon WOULD be blocked right now.
///   3. Despite (2), Belphemon's end-attack succeeds: the attack is cancelled
///      and NO security card is checked (Progress guards the Digimon, not the
///      battle — judge: YES).
#[test]
fn q1_belphemon_opp_turn_ends_attack_through_progress() {
    use digimon_engine::enums::Keyword;

    // A Lv.5 Red digivolution source so Medusamon (Lv.6 Red, evo from Lv.5 Red)
    // can sit as a real stack top — its `<Progress>` rides the BT24-017 face card.
    let mut red_base = make_test_card("RED-BASE", "Red Base");
    red_base.card_kind = CardKind::Digimon;
    red_base.colors = vec![CardColor::Red];
    red_base.level = Some(5);
    red_base.dp = Some(6000);

    let mut r = DebugRunner::builder()
        .dsl_card("BT13-088")
        .expect("BT13-088 Belphemon: Sleep Mode loads")
        .dsl_card("BT24-017")
        .expect("BT24-017 Medusamon (<Progress>) loads")
        .add_card(red_base)
        .add_card(make_test_card("HAND1", "Hand 1"))
        .add_card(make_test_card("HAND2", "Hand 2"))
        .add_card(make_test_card("SECURITY", "Security"))
        // Player 1 needs a deck so the turn rotation to its turn does not deck-out.
        .add_card(make_test_card("DECK1", "Deck 1"))
        .hand(0, &["HAND1", "HAND2"]) // Belphemon's controller pays the 2-card cost
        .security(0, &["SECURITY"]) // the single card the attack would otherwise hit
        .deck(1, &["DECK1"; 20])
        .memory(10)
        .start();
    r.skip_mulligan();

    // Player 0: Belphemon: Sleep Mode (the defender with the [Opp Turn] cancel).
    let _belphemon = r.place_on_field(0, "BT13-088", Some(0));
    // Player 1: Medusamon stacked on a Lv.5 Red source → the <Progress> attacker.
    let medusamon = r.place_stack(1, &["RED-BASE", "BT24-017"]);

    // Materialize the declarative <Progress> grant so the keyword is live.
    r.game.tick_declarative_effects();
    assert!(
        r.game.has_keyword(medusamon, Keyword::Progress),
        "precondition: Medusamon must carry the <Progress> keyword"
    );

    // Hand to Player 1 (the attacker's turn).
    r.end_turn();
    assert_eq!(
        r.turn_player(),
        1,
        "Q1 plays out on the attacker (Player 1)'s turn"
    );

    // Player 1's Medusamon attacks Player 0's security.
    r.attack_player(medusamon, 0, false);

    // Progress is genuinely IN SCOPE right now: Medusamon is the current attacker
    // and an opponent (Player 0) effect that *affected* it would be blocked.
    // This is the load-bearing precondition that prevents a wrong-reason pass —
    // the immunity is active, yet the end-attack must still go through.
    assert_eq!(
        r.game.current_attacker(),
        Some(medusamon),
        "Medusamon must be the active attacker so <Progress> is in scope"
    );
    assert!(
        r.game.progress_excludes(medusamon, Some(0)),
        "precondition: <Progress> must currently block Player 0's effects that \
         would affect Medusamon (so the only way the attack ends is if end-attack \
         does NOT 'affect' Medusamon)"
    );

    // Belphemon's [Opp Turn][OPT] "trash 2 cards in hand to end the attack" — pay
    // the cost by trashing the 2 hand cards (two sequential picks).
    let view = r
        .pending_selection_view()
        .expect("Belphemon's attack-cancel cost prompt installs (hand >= 2)");
    assert_eq!(view.kind, digimon_engine::selection::SelectionKind::Hand);
    r.execute_action(0, view.valid_action_ids[0])
        .expect("trash first hand card");
    let view2 = r
        .pending_selection_view()
        .expect("second hand-trash cost prompt installs");
    r.execute_action(0, view2.valid_action_ids[0])
        .expect("trash second hand card");
    r.auto_resolve().expect("finish the attack-cancel");

    // Judge: YES — the attack is ended through Progress.
    assert!(
        r.game.pending_attack.is_none(),
        "Belphemon ended the attack — `end_attack` cancels the BATTLE (it does not \
         'affect' Medusamon), so <Progress> does not block it (judge-quiz Q1: YES)"
    );
    assert_eq!(
        r.security_count(0),
        1,
        "the attack ended before any security check — Player 0's single security \
         card is untouched"
    );
    assert_eq!(
        r.hand_size(0),
        0,
        "exactly the 2 hand cards were trashed as the end-attack cost"
    );
    // Medusamon is unharmed and still on the field (the battle simply ended).
    assert!(
        r.game.players[1]
            .battle_area
            .iter()
            .any(|p| p.top_card().card_id(&r.game.card_data) == "BT24-017"),
        "Medusamon remains on the field — the attack was ended, not redirected"
    );
}

/// Q17 — RESOLVED 2026-05-29 (change `add-grant-triggered-effect-dsl`): a
/// Lilithmon (EX6-057)-granted "[End of Your Turn] Delete this" is an
/// OPPONENT-sourced granted effect. Magnamon (X Antibody) (BT16-102)'s
/// [When Digivolving] makes it "isn't affected by your opponent's effects until
/// the end of your opponent's turn", so when the granted [EoT] delete would
/// fire the granted-trigger dispatch suppresses it
/// (`permanent_is_unaffected_by_effect`) → the delete does NOT activate.
/// (Staged with a synthetic [Armor Form] digivolution source — BT21-036
/// Magnamon's only role in the judge scenario is being that Armor-Form source;
/// its own effects don't fire as a digivolution card.)
#[test]
fn q17_magnamon_x_immunity_removes_granted_eot_delete() {
    use digimon_engine::enums::ModifierType;

    let armor_src = {
        let mut c = make_test_card("ARMORSRC", "Armor Src");
        c.card_kind = CardKind::Digimon;
        c.level = Some(5);
        c.traits = vec!["Armor Form".to_string()];
        c
    };

    let mut r = DebugRunner::builder()
        .dsl_card("BT16-102")
        .expect("BT16-102 Magnamon (X Antibody) loads")
        .dsl_card("EX6-057")
        .expect("EX6-057 Lilithmon loads")
        .add_card(armor_src)
        .memory(10)
        .start();
    r.skip_mulligan();

    // Player 1's Magnamon X, stacked on an [Armor Form] source so its
    // [When Digivolving] immunity condition passes.
    let magnamon_x = r.place_stack(1, &["ARMORSRC", "BT16-102"]);

    // Player 0 plays Lilithmon → grants the (only) opponent Digimon (Magnamon X)
    // the "[End of Your Turn] Delete this" — installed BEFORE the immunity, like
    // the judge scenario (grant on Magnamon, then it digivolves into Magnamon X).
    let lilithmon = r.place_on_field(0, "EX6-057", None);
    r.fire_on_play(0, lilithmon.index as usize);
    if let Some(sel) = r.game.pending_selection.as_ref() {
        let who = sel.selecting_player;
        let aid = sel.valid_action_ids[0];
        let _ = r.game.resolve_selection(who, aid);
    }
    assert!(
        !r.game
            .modifiers
            .granted_triggered_for_timing(magnamon_x, EffectTiming::EndOfYourTurn)
            .is_empty(),
        "Lilithmon grant installed the [EoT] delete on Magnamon X"
    );

    // Magnamon X's [When Digivolving] fires → "isn't affected by your opponent's
    // effects until end of opponent's turn" (+ DP + unsuspend).
    r.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(magnamon_x),
    );
    r.game.drain_effect_queue();
    assert!(
        r.game
            .permanent_is_unaffected_by_effect(magnamon_x, 0, EffectSourceKind::Digimon),
        "Magnamon X's [When Digivolving] must grant opponent-effect immunity"
    );

    // Fire the granted "[End of Your Turn] Delete this" → it is the opponent's
    // (Lilithmon's) effect, and Magnamon X is immune → SUPPRESSED, not deleted.
    r.game.enqueue_triggered(
        EffectTiming::EndOfYourTurn,
        TriggerSource::Permanent(magnamon_x),
    );
    r.game.drain_effect_queue();

    let magnamon_x_alive = r.game.players[1]
        .battle_area
        .iter()
        .any(|p| p.top_card().card_id(&r.game.card_data) == "BT16-102");
    assert!(
        magnamon_x_alive,
        "Magnamon X's opponent-effect immunity must suppress the Lilithmon-granted \
         [EoT] delete (judge-quiz Q17: does NOT activate)"
    );
    let _ = ModifierType::CannotBeAffected;
}

/// Q18 — Quantumon (LM-020) immunity is to ALL Digimon effects incl. its own;
/// `<Blast Digivolve>` into Imperialdramon: PM ACE (BT17-077) is an effect. Judge: NO.
///
/// # Board / rationale
/// At the start of the opponent's turn, Quantumon's [Start of Opponent's Turn]
/// clause declares a card category, reveals the opponent's deck-top, and — if it
/// matches — makes Quantumon "isn't affected by the effects of that card category
/// for the turn." Declaring **Digimon** (and revealing a Digimon) grants immunity
/// to Digimon effects from ANY controller, **including its own**. `<Blast
/// Digivolve>` is itself a Digimon effect, and combat gates blast-digivolve
/// candidacy AND execution on exactly
/// `permanent_is_unaffected_by_effect(target, target.player, Digimon)`
/// (`combat.rs` ~1507 candidate filter + ~1744 execute guard). So Quantumon
/// cannot `<Blast Digivolve>` into BT17-077. Judge: **NO**.
///
/// # Load-bearing assertion
/// This pins the operative gate using the REAL LM-020 clause-2 effect (not a
/// synthetic modifier): after the declaration, Quantumon is unaffected by its own
/// controller's Digimon effects. A control asserts it is affectable BEFORE the
/// declaration, so the immunity — not some unrelated route gap — is what produces
/// the judge's NO. (The end-to-end "immune base ⇒ no blast counter candidate"
/// mechanism is separately proven in
/// `combat::counter_interrupt::blast_target_immune_to_own_effects_is_not_a_counter_candidate`.)
#[test]
fn q18_quantumon_self_immunity_blocks_own_blast_digivolve() {
    let dgm = make_test_card("DGM", "Deck Digimon");
    let mut r = DebugRunner::builder()
        .dsl_card("LM-020")
        .expect("LM-020 Quantumon loads")
        .add_card(make_test_card("PAD", "PAD"))
        .add_card(dgm)
        // opponent (player 1) deck top = Vec end = a Digimon.
        .deck(1, &["PAD", "PAD", "DGM"])
        .deck(0, &["PAD", "PAD", "PAD"])
        .memory(0)
        .start();
    let quantumon = r.place_on_field(0, "LM-020", Some(0));

    // Control: before the declaration, Quantumon is NOT immune to its own
    // Digimon effects — absent the immunity, a <Blast Digivolve> would be legal.
    assert!(
        !r.game
            .permanent_is_unaffected_by_effect(quantumon, 0, EffectSourceKind::Digimon),
        "control: Quantumon is affectable by its own Digimon effects before the declaration"
    );

    // [Start of Opponent's Turn]: declare Digimon, reveal a Digimon → immunity.
    r.game.enqueue_triggered(
        EffectTiming::StartOfOpponentsTurn,
        TriggerSource::Permanent(quantumon),
    );
    r.game.drain_effect_queue();
    r.execute_branch(0).expect("declare Digimon category");
    r.execute_branch(0).expect("return revealed card to deck top");

    // Judge: NO. Quantumon is now unaffected by Digimon effects from ANY
    // controller, including its OWN — the exact condition combat uses to forbid
    // <Blast Digivolve> into Imperialdramon: PM ACE (BT17-077).
    assert!(
        r.game
            .permanent_is_unaffected_by_effect(quantumon, 0, EffectSourceKind::Digimon),
        "Q18: Quantumon's declared Digimon immunity blocks its OWN <Blast Digivolve> (judge: NO)"
    );
}

/// Q28 — Gankoomon (X Antibody) (BT20-059) protection beats Dragomon (EX5-060)
/// "[On Play] don't activate" on Sistermon Ciel (BT6-084).
/// **Judge: YES — Ciel is played AND activates its [On Play].**
///
/// Board (card-resolution.md Q28 / PDF p.62-64): Player A has Gankoomon
/// (X Antibody) over Gankoomon in the battle area, with ONLY Sistermon Ciel
/// in their trash. Player B has Dragomon in hand. On A's turn, Gankoomon X
/// activated its [When Digivolving] — "…until the end of your opponent's
/// turn, none of your Digimon are affected by your opponent's Digimon's
/// effects" (a PERSISTENT effect per the judge feedback). On B's turn, B
/// plays Dragomon and activates its [On Play]: A plays 1 Lv≤4 Digimon from
/// their trash suspended; "[On Play] effects on Digimon played by this
/// effect don't activate."
///
/// Judge rationale: the protection is persistent — when Ciel is played INTO
/// the battle area (it was in the TRASH at grant time), the protection
/// covers it immediately, making it immune to Dragomon's don't-activate
/// rider (an opponent-Digimon effect). Ciel's [On Play] (gain 1 memory)
/// activates.
///
/// Engine: the protection is the CONTINUOUS `grant_effect_immunity` form
/// (G-DSL-CONTINUOUS-CONTROLLED-IMMUNITY-AURA — a floating descriptor
/// re-scanned per tick, covering later entrants); the suppression rider is
/// consult-gated on `permanent_is_unaffected_by_effect` (Q28 fix in
/// `fire_play_event_triggers`). The companion control
/// (`q28_control_no_protection_on_play_suppressed`) proves the rider is
/// real — no false-pass.
///
/// DCGO: `EX5_060.cs` (the played card's `CanNotBeAffected` is checked
/// before applying the don't-activate); `BT20_059.cs` `CanNotAffectedClass`
/// (re-evaluated CardCondition).
#[test]
fn q28_gankoomon_x_protection_beats_dragomon_on_play_lock() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT20-059")
        .expect("BT20-059 Gankoomon (X Antibody) loads")
        .dsl_card("BT23-057")
        .expect("BT23-057 Gankoomon loads")
        .dsl_card("EX5-060")
        .expect("EX5-060 Dragomon loads")
        .dsl_card("BT6-084")
        .expect("BT6-084 Sistermon Ciel loads")
        .add_card(make_test_card("FILLER", "Filler"))
        .deck(0, &["FILLER"; 6])
        .deck(1, &["FILLER"; 6])
        .memory(10)
        .start();
    r.skip_mulligan();
    r.set_first_player(0);
    assert_eq!(r.turn_player(), 0, "Player A's turn first");

    // Player A: Gankoomon (X Antibody) over Gankoomon (the [Gankoomon]
    // digivolution source satisfies the protection's source gate).
    let gank_x = r.place_stack(0, &["BT23-057", "BT20-059"]);
    // A's trash: ONLY Sistermon Ciel.
    r.inject_trash(0, "BT6-084");
    // Player B: Dragomon staged on field; its [On Play] is fired as the
    // trigger (the hard-play cost mechanics are not the ruling under test).
    let dragomon = r.place_on_field(1, "EX5-060", Some(0));

    // On A's turn: Gankoomon X's [When Digivolving] resolves — B has no
    // Digimon eligible for the <De-Digivolve 2> pick beyond Dragomon; drive
    // any picks to completion; the persistent protection installs
    // (continuous, until the end of B's turn).
    r.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(gank_x),
    );
    r.game.drain_effect_queue();
    let _ = r.auto_resolve();

    // A's turn ends → B's turn (the protection window is still live).
    r.end_turn();
    let _ = r.auto_resolve();
    assert_eq!(r.turn_player(), 1, "Player B's turn");
    // Give B gauge headroom so Ciel's +1 for A does NOT cross the gauge
    // (crossing would faithfully END B's turn — rule 8-3 — and A's
    // turn-start unsuspend would mask the suspended-entry assertion).
    r.game.set_memory(5);

    let memory_before = r.memory();
    let a_field_before = r.battle_area_size(0);

    // B activates Dragomon's [On Play].
    r.game
        .enqueue_triggered(EffectTiming::OnPlay, TriggerSource::Permanent(dragomon));
    r.game.drain_effect_queue();

    // A (the opponent of Dragomon's controller) picks from A's OWN trash —
    // Ciel is the only candidate.
    let view = r
        .pending_selection_view()
        .expect("Dragomon's opponent-trash pick must surface");
    assert_eq!(
        view.selecting_player, 0,
        "the OPPONENT (Player A) chooses from their own trash"
    );
    r.execute_action(0, view.valid_action_ids[0])
        .expect("A picks Sistermon Ciel");
    let _ = r.auto_resolve();

    // ── Judge assertions ────────────────────────────────────────────────
    // Ciel was played onto A's field, SUSPENDED.
    assert_eq!(
        r.battle_area_size(0),
        a_field_before + 1,
        "Sistermon Ciel is played to Player A's battle area"
    );
    let ciel_idx = r.game.players[0]
        .battle_area
        .iter()
        .position(|p| p.top_card().card_id(&r.game.card_data) == "BT6-084")
        .expect("Ciel on A's field");
    assert!(
        r.game.players[0].battle_area[ciel_idx].is_suspended,
        "Dragomon plays the Digimon SUSPENDED"
    );
    assert_eq!(r.trash_size(0), 0, "Ciel left A's trash");

    // The rider is beaten: Ciel's [On Play] "Gain 1 memory" ACTIVATED —
    // memory moved 1 toward Player A (negative for turn player B).
    assert_eq!(
        r.memory(),
        memory_before - 1,
        "judge Q28: the protected Ciel's [On Play] activates through \
         Dragomon's \"don't activate\" rider (gain 1 memory for A)"
    );
}

/// Q28 CONTROL — same staging WITHOUT Gankoomon X's protection: Dragomon's
/// rider suppresses Ciel's [On Play] (no memory gain) while the play itself
/// still happens, suspended. Proves the Q28 pin cannot false-pass on a
/// broken (never-suppressing) rider.
#[test]
fn q28_control_no_protection_on_play_suppressed() {
    let mut r = DebugRunner::builder()
        .dsl_card("EX5-060")
        .expect("EX5-060 Dragomon loads")
        .dsl_card("BT6-084")
        .expect("BT6-084 Sistermon Ciel loads")
        .add_card(make_test_card("FILLER", "Filler"))
        .deck(0, &["FILLER"; 6])
        .deck(1, &["FILLER"; 6])
        .memory(10)
        .start();
    r.skip_mulligan();
    r.set_first_player(1); // B's turn; no protection was ever granted
    r.inject_trash(0, "BT6-084");
    let dragomon = r.place_on_field(1, "EX5-060", Some(0));

    let memory_before = r.memory();
    r.game
        .enqueue_triggered(EffectTiming::OnPlay, TriggerSource::Permanent(dragomon));
    r.game.drain_effect_queue();
    let view = r
        .pending_selection_view()
        .expect("Dragomon's opponent-trash pick must surface");
    assert_eq!(view.selecting_player, 0);
    r.execute_action(0, view.valid_action_ids[0])
        .expect("A picks Sistermon Ciel");
    let _ = r.auto_resolve();

    let ciel_idx = r.game.players[0]
        .battle_area
        .iter()
        .position(|p| p.top_card().card_id(&r.game.card_data) == "BT6-084")
        .expect("Ciel on A's field");
    assert!(
        r.game.players[0].battle_area[ciel_idx].is_suspended,
        "played suspended"
    );
    assert_eq!(
        r.memory(),
        memory_before,
        "without protection the rider SUPPRESSES Ciel's [On Play] — no \
         memory gain (control for Q28)"
    );
}
