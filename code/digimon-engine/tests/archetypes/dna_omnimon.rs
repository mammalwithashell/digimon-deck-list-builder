//! DNA Omnimon — archetype interaction tests.
//!
//! Model: `qa/archetype-qa/DNA Omnimon-model.md`. The deck converges two tribes
//! (Greymon / Garurumon) into [Omnimon]-name Lv.7s via **DNA digivolution**
//! (`general_rule.pdf` §8-2; the new DNA Digimon may attack the same turn,
//! §8-2-2-1-6). These interaction tests pin the cross-card combos a per-card
//! behavioral test cannot see — exercising the **real card abilities** (no
//! bypassing engine helper stands in for a named combo card).
//!
//! Per-card behavioral coverage lives under `tests/cards_behavioral/<set>/`
//! (ex9_021, bt17_078, bt17_095, bt17_015, bt22_013, …); this file asserts the
//! combined-system outcomes the model's named combos claim.
//!
//! Coverage caveat (model §"Coverage caveat"): the DNA Omnimon coverage gate
//! FAILS (66/98 = 67%, threshold 85%) — several digivolution-line connector
//! cards (BT14-014, BT15-024, EX9-014) are NOT implemented. They are line
//! context, NOT combo pieces; none of the combos below depend on them, which is
//! why the combo-presence gate passes even though the coverage gate fails.
//!
//! Combos covered:
//! - A — EX9-021 DNA blowout (delete-all-highest + opp-effect immunity).        [AUTHORED]
//! - B — BT17-095 Delay → reactive DNA (leaving Lv.6 consumed into an Omnimon). [AUTHORED]
//! - C — BT17-078 Blast DNA Counter (same-level bottom-deck + delete).          [AUTHORED]
//! - D — free cross-tribe Lv.6 assembly (BT17-015 free Gabumon→MetalGarurumon). [AUTHORED]
//! - E — Nokia cost-6 Lv.6 jump (BT22-013 [Hand][Main]).                        [#[ignore] — G-ACTIVATED-DIGIVOLVE-EXECUTION]

#![allow(dead_code)]

use digimon_engine::action::space::{DNA_DIGIVOLVE_START, PLAY_HAND_START};
use digimon_engine::card_data::CardData;
use digimon_engine::combat::AttackResult;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::effect_context::EffectContext;
use digimon_engine::enums::{CardColor, CardKind, EffectSourceKind, EffectTiming};
use digimon_engine::permanent::{OptionState, PermanentHandle};
use digimon_engine::replacement::ReplacementCause;
use digimon_engine::selection::{AttackTarget, TriggerSource};

use super::support::snapshot;

// ─── Shared combo-piece fixtures ─────────────────────────────────────────────

/// A colored Lv.6 Digimon — the raw DNA-material shape (any blue 6 + any red 6
/// is a legal EX9-021 DNA pair).
fn make_colored_lv6(id: &str, color: CardColor) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(6);
    c.dp = Some(11000);
    c.colors = vec![color];
    c
}

/// An opponent Digimon with an explicit level + DP, used as a removal victim.
fn make_opp_digimon(id: &str, name: &str, level: u8, dp: i32) -> CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = CardKind::Digimon;
    c.level = Some(level);
    c.dp = Some(dp);
    c
}

// ═══════════════════════════════════════════════════════════════════════════
// Combo A — DNA Omnimon Alter-S blowout (Blue 6 + Red 6 → EX9-021)
// ═══════════════════════════════════════════════════════════════════════════
//
// Cards: EX9-021 Omnimon Alter-S + a Blue Lv.6 + a Red Lv.6 (the DNA pair) +
//   opponent Digimon victims.
// Expected outcome (model Combo A): stacking a Blue Lv.6 + Red Lv.6 into EX9-021
//   (DNA, cost 0) fires [When Digivolving] → (a) DNA-gated opponent-effect
//   immunity for the turn, and (b) delete ALL opponent Digimon tied for the
//   highest level. The two materials become EX9-021's digivolution sources.
// Rules basis: `general_rule.pdf` §8-2 (DNA digivolution); DCGO
//   `EX9/Blue/EX9_021.cs` (DNA gate + delete-highest). Engine: cards/ex9/EX9-021.yaml.
//
// The DNA digivolve is fired with the REAL ability path
// (`effect_initiated_dna_digivolve`, the verb the DNA alt-path routes through),
// not a board-mutation shortcut — so the delete + immunity arms run as in play.

/// Happy path: DNA digivolve into EX9-021 deletes ALL opponent Digimon at the
/// (tied) highest level and grants self opponent-effect immunity for the turn,
/// while a strictly-lower-level opponent Digimon survives.
#[test]
fn combo_a_dna_blowout_deletes_all_highest_level_and_grants_immunity() {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX9-021")
        .expect("EX9-021 (Omnimon Alter-S) in embedded DSL pack")
        .add_card(make_colored_lv6("A-BLUE6", CardColor::Blue))
        .add_card(make_colored_lv6("A-RED6", CardColor::Red))
        // Two opp Digimon tied at the highest level (7) + one strictly lower (5).
        .add_card(make_opp_digimon("A-OPP-HIGH1", "OppHigh1", 7, 12000))
        .add_card(make_opp_digimon("A-OPP-HIGH2", "OppHigh2", 7, 9000))
        .add_card(make_opp_digimon("A-OPP-LOW", "OppLow", 5, 6000))
        .hand(0, &["EX9-021"])
        .memory(10)
        .start();
    runner.game.turn_count = 1;

    runner.place_on_field(1, "A-OPP-HIGH1", None);
    runner.place_on_field(1, "A-OPP-HIGH2", None);
    runner.place_on_field(1, "A-OPP-LOW", None);
    let blue = runner.place_on_field(0, "A-BLUE6", None);
    let red = runner.place_on_field(0, "A-RED6", None);

    let before = snapshot(&runner);
    assert_eq!(before.field[1], 3, "fixture: 3 opp Digimon (2 tied high + 1 low)");

    // Fire the REAL DNA digivolve: Blue Lv.6 + Red Lv.6 → EX9-021 at cost 0.
    let hand_card = runner.game.player(0).hand[0].handle();
    let evolved = {
        let mut ctx = EffectContext::new(&mut runner.game, hand_card, None, 0);
        ctx.effect_initiated_dna_digivolve(blue, red, hand_card, 0, true)
            .expect("Blue Lv.6 + Red Lv.6 must DNA digivolve into EX9-021")
    };
    runner.auto_resolve().expect("DNA trigger order resolves");
    let after = snapshot(&runner);

    // Both Lv.7 (highest, tied) opponent Digimon are deleted; the Lv.5 survives.
    let survivors: Vec<String> = runner.game.players[1]
        .battle_area
        .iter()
        .map(|p| p.top_card().card_name(&runner.game.card_data).to_string())
        .collect();
    assert_eq!(
        after.field[1], 1,
        "delete-all-highest must remove BOTH tied Lv.7 opp Digimon; survivors={survivors:?}"
    );
    assert!(
        survivors.iter().any(|n| n == "OppLow"),
        "the strictly-lower-level (Lv.5) opp Digimon must survive; survivors={survivors:?}"
    );
    assert_eq!(
        after.trash[1],
        before.trash[1] + 2,
        "both deleted opp Digimon land in the opponent's trash"
    );

    // DNA origin → EX9-021 is immune to opponent-controlled effects this turn
    // (Digimon / Tamer / Option source kinds), but not to its own controller's.
    for kind in [
        EffectSourceKind::Digimon,
        EffectSourceKind::Tamer,
        EffectSourceKind::Option,
    ] {
        assert!(
            runner.game.permanent_is_unaffected_by_effect(evolved, 1, kind),
            "DNA-origin EX9-021 must be immune to opponent {kind:?} effects for the turn"
        );
    }
    assert!(
        !runner
            .game
            .permanent_is_unaffected_by_effect(evolved, 0, EffectSourceKind::Digimon),
        "immunity is one-sided: EX9-021 must NOT be immune to its own controller's effects"
    );
}

/// Unhappy path: reaching EX9-021 by a STANDARD (non-DNA) digivolve still runs
/// the unconditional delete-highest arm, but the DNA-gated opponent-effect
/// immunity must NOT be granted (the "If DNA digivolving" antecedent fails).
/// This is the system fact a per-card test isolates but the combo depends on:
/// only the DNA line buys the protection.
#[test]
fn combo_a_standard_digivolve_does_not_grant_immunity() {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX9-021")
        .expect("EX9-021 (Omnimon Alter-S) in embedded DSL pack")
        .memory(10)
        .start();
    runner.game.turn_count = 1;

    let perm = runner.place_on_field(0, "EX9-021", None);
    let card = runner.game.players[0].battle_area[perm.index as usize]
        .top_card()
        .handle();
    // Standard (non-DNA) digivolve trigger: dna_origin = false.
    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Digivolved {
            player: 0,
            permanent: perm,
            card,
            effect_initiated: false,
            dna_origin: false,
        },
    );
    runner.game.drain_effect_queue();
    runner.auto_resolve().expect("standard trigger order resolves");

    assert!(
        !runner
            .game
            .permanent_is_unaffected_by_effect(perm, 1, EffectSourceKind::Digimon),
        "standard-digivolved EX9-021 must NOT receive the DNA-only opp-effect immunity"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Combo B — Miraculous Mega Knight Delay → reactive DNA Omnimon (BT17-095)
// ═══════════════════════════════════════════════════════════════════════════
//
// Cards: BT17-095 (Option, seated as a Delay) + an own Lv.6 [Greymon] on field
//   + an [Omnimon]-name Lv.7 in hand + a Lv.6 DNA partner in hand.
// Expected outcome (model Combo B): when an own Lv.6 [Greymon]/[Garurumon]
//   *would leave the battle area outside of battle*, the Delay fires: that
//   leaving Lv.6 + a hand card DNA digivolve into an [Omnimon]-name Lv.7 in hand.
//   The leaving Lv.6 is CONSUMED as a DNA material under the merged Omnimon —
//   it does NOT go to trash.
// Rules basis: §8-2 (DNA digivolution); §16 <Delay>. DCGO
//   `BT17/Red/BT17_095.cs` (WhenRemoveField Delay). Engine: cards/bt17/BT17-095.yaml.
//   The DNA-into-Omnimon body uses `effect_initiated_dna_digivolve_hand_partner`
//   (G-DSL-DNA-FROM-HAND-PARTNER CLOSED 2026-05-20) — a hand card is the 2nd material.

/// Seat a placed BT17-095 Option permanent as a Delay-Option so its [All Turns]
/// `when_would_leave_battle_area` replacement can fire (Clause B is gated on
/// `source_is_delayed_option`). Mirrors `place_self_as_delay_option_permanent`.
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

/// An own Lv.6 [Greymon] — the Delay's leaving subject (DNA material A).
fn make_l6_greymon(id: &str) -> CardData {
    let mut c = make_test_card(id, "Greymon");
    c.card_kind = CardKind::Digimon;
    c.level = Some(6);
    c.dp = Some(11000);
    c.play_cost = 11;
    c.colors = vec![CardColor::Red];
    c
}

/// An [Omnimon]-name Lv.7 result card (in hand).
fn make_l7_omnimon(id: &str) -> CardData {
    let mut c = make_test_card(id, "Omnimon");
    c.card_kind = CardKind::Digimon;
    c.level = Some(7);
    c.dp = Some(13000);
    c.play_cost = 13;
    c.colors = vec![CardColor::Red, CardColor::Blue];
    c
}

/// A Lv.6 hand-card DNA partner (DNA material B).
fn make_l6_hand_partner(id: &str) -> CardData {
    let mut c = make_test_card(id, "GarurumonPartner");
    c.card_kind = CardKind::Digimon;
    c.level = Some(6);
    c.dp = Some(11000);
    c.play_cost = 11;
    c.colors = vec![CardColor::Blue];
    c
}

/// Drive every installed selection by picking its first non-PASS valid action,
/// draining the effect queue between picks. Bounded so a logic bug surfaces as a
/// loop-exhaustion rather than a hang.
fn drive_first_valid(runner: &mut DebugRunner, max_steps: usize) {
    use digimon_engine::action::space::PASS;
    for _ in 0..max_steps {
        let Some(view) = runner.pending_selection_view() else {
            return;
        };
        let action = view
            .valid_action_ids
            .iter()
            .copied()
            .find(|&a| a != PASS)
            .unwrap_or(PASS);
        if runner.game.resolve_selection(view.selecting_player, action).is_err() {
            return;
        }
        runner.game.drain_effect_queue();
    }
    panic!("drive_first_valid exhausted {max_steps} steps without draining the selection queue");
}

/// Happy path: an own Lv.6 [Greymon] leaving the field outside battle fires the
/// BT17-095 Delay; the leaving Greymon + a hand partner DNA digivolve into the
/// [Omnimon]-name Lv.7 in hand. The merged Omnimon exists with the Greymon as a
/// source, and the Greymon is NOT in the trash (consumed, not destroyed).
#[test]
fn combo_b_delay_consumes_leaving_lv6_into_merged_omnimon() {
    let filler = make_test_card("B-FILL", "B Fill");
    let mut runner = DebugRunner::builder()
        .dsl_card("BT17-095")
        .expect("BT17-095 (Miraculous Mega Knight) in embedded DSL pack")
        .add_card(make_l6_greymon("B-FIELD-GREY"))
        .add_card(make_l7_omnimon("B-OMNI"))
        .add_card(make_l6_hand_partner("B-PARTNER"))
        .add_card(filler)
        .hand(0, &["B-OMNI", "B-PARTNER"])
        .deck(0, &["B-FILL"; 5])
        .deck(1, &["B-FILL"; 5])
        .memory(10)
        .start();
    runner.game.turn_count = 1;

    let bt17_095 = runner.place_on_field(0, "BT17-095", Some(0));
    seat_as_delay_option(&mut runner, bt17_095);
    let greymon_handle = runner.place_on_field(0, "B-FIELD-GREY", None);
    let greymon_card = runner.top_card(greymon_handle);

    // Trigger the leave of the own field Greymon (own-effect, outside battle).
    runner
        .game
        .delete_permanent_with_cause(greymon_handle, ReplacementCause::OwnEffect);

    // The optional replacement installs an accept prompt; accept it, then resolve
    // the Omnimon-result + hand-partner picks.
    drive_first_valid(&mut runner, 20);

    // A merged permanent topped with the Omnimon now exists on P0's field, with
    // the leaving Greymon as one of its digivolution sources.
    let merged = runner.game.players[0]
        .battle_area
        .iter()
        .find(|p| p.top_card().card_id(&runner.game.card_data) == "B-OMNI")
        .expect("merged Omnimon permanent must exist after the Delay DNA digivolve");
    assert!(
        merged.card_sources.iter().any(|s| s.handle() == greymon_card),
        "the leaving Greymon must be a digivolution source under the merged Omnimon"
    );
    assert!(
        merged.card_sources.len() >= 3,
        "merged stack must hold Greymon + hand partner + Omnimon result; got {}",
        merged.card_sources.len()
    );

    // The leaving Greymon was CONSUMED, not destroyed: it must NOT be in trash.
    let greymon_in_trash = runner.game.players[0]
        .trash
        .iter()
        .any(|c| c.handle() == greymon_card);
    assert!(
        !greymon_in_trash,
        "a successful DNA merge consumes the leaving Greymon into the merged \
         Omnimon — it must NOT proceed to the trash"
    );

    // Both hand cards (result + partner) were consumed.
    assert_eq!(
        runner.game.players[0].hand.len(),
        0,
        "the Omnimon result and the DNA partner must both leave the hand"
    );
}

/// Unhappy path (subject filter): an OPPONENT's Lv.6 [Greymon] leaving the field
/// must NOT fire BT17-095's Delay (Clause B is gated on
/// `replacement_subject_is_mine`). The opponent's Greymon proceeds to trash
/// normally and no merged Omnimon is created — the combo is gated on the leaving
/// Lv.6 being YOURS.
#[test]
fn combo_b_delay_does_not_fire_for_opponent_lv6_leaving() {
    let filler = make_test_card("BN-FILL", "BN Fill");
    let mut runner = DebugRunner::builder()
        .dsl_card("BT17-095")
        .expect("BT17-095 (Miraculous Mega Knight) in embedded DSL pack")
        .add_card(make_l6_greymon("BN-OPP-GREY"))
        .add_card(make_l7_omnimon("BN-OMNI"))
        .add_card(make_l6_hand_partner("BN-PARTNER"))
        .add_card(filler)
        .hand(0, &["BN-OMNI", "BN-PARTNER"])
        .deck(0, &["BN-FILL"; 5])
        .deck(1, &["BN-FILL"; 5])
        .memory(10)
        .start();
    runner.game.turn_count = 1;

    let bt17_095 = runner.place_on_field(0, "BT17-095", Some(0));
    seat_as_delay_option(&mut runner, bt17_095);
    // The leaving Lv.6 Greymon belongs to the OPPONENT (player 1).
    let opp_greymon = runner.place_on_field(1, "BN-OPP-GREY", None);

    let before = snapshot(&runner);
    runner
        .game
        .delete_permanent_with_cause(opp_greymon, ReplacementCause::OwnEffect);

    assert!(
        runner.pending_selection().is_none(),
        "the Delay must NOT install a prompt when the leaving Lv.6 belongs to the opponent"
    );
    let after = snapshot(&runner);
    // No merged Omnimon was created; hand untouched (result + partner stay).
    let merged_exists = runner.game.players[0]
        .battle_area
        .iter()
        .any(|p| p.top_card().card_id(&runner.game.card_data) == "BN-OMNI");
    assert!(
        !merged_exists,
        "no merged Omnimon may be created for an opponent's leaving Digimon"
    );
    assert_eq!(
        after.hand[0], before.hand[0],
        "the Omnimon result + DNA partner must stay in hand (Delay did not fire)"
    );
    // The opponent's Greymon goes to trash normally.
    assert_eq!(
        after.field[1],
        before.field[1] - 1,
        "the opponent's leaving Greymon should leave its battle area as normal"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Combo C — Blast DNA Omnimon off the opponent's turn (BT17-078 Counter)
// ═══════════════════════════════════════════════════════════════════════════
//
// Cards: BT17-078 (hand) + a [WarGreymon] field material + a [MetalGarurumon]
//   hand material; opponent attacking + opponent Digimon victims.
// Expected outcome (model Combo C): at Counter timing (when attacked), Blast DNA
//   Digivolve — field WarGreymon + hand MetalGarurumon become BT17-078's sources
//   at no cost. [When Digivolving] (DNA-gated): choose 1 opp Digimon, bottom-deck
//   ALL opp Digimon of that level, then a mandatory delete of 1 more opp Digimon.
// Rules basis: §16 <Blast DNA Digivolve>; §8-2. DCGO `BT17/White/BT17_078.cs`.
//   Engine: cards/bt17/BT17-078.yaml.
//
// Driven through the REAL combat/Counter route (`begin_attack` → Counter window
// → Blast-DNA material selection), then the real DNA-gated bottom-deck + delete.

/// A named Digimon material with explicit level/DP/color.
fn make_named_material(id: &str, name: &str, level: u8, dp: i32, color: CardColor) -> CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = CardKind::Digimon;
    c.level = Some(level);
    c.dp = Some(dp);
    c.colors = vec![color];
    c
}

/// Happy path: BT17-078 Blast DNA at Counter timing (field WarGreymon + hand
/// MetalGarurumon) bottom-decks every opponent Digimon at the chosen level, then
/// installs the mandatory extra-delete prompt — all on the opponent's turn.
#[test]
fn combo_c_blast_dna_counter_bottom_decks_same_level_then_prompts_delete() {
    let wargreymon = make_named_material("C-WAR", "WarGreymon", 6, 11000, CardColor::Red);
    let metalgarurumon =
        make_named_material("C-METAL", "MetalGarurumon", 6, 11000, CardColor::Blue);
    // The attacker is a Lv.5; a same-level (Lv.5) peer must also be bottom-decked.
    let attacker = make_named_material("C-ATK", "Attacker", 5, 7000, CardColor::Red);
    let peer_lv5 = make_named_material("C-PEER5", "Peer5", 5, 6000, CardColor::Red);
    let survivor_lv6 = make_named_material("C-SURV6", "Surv6", 6, 9000, CardColor::Red);

    let mut runner = DebugRunner::builder()
        .dsl_card("BT17-078")
        .expect("BT17-078 (Omnimon) in embedded DSL pack")
        .add_card(wargreymon)
        .add_card(metalgarurumon)
        .add_card(attacker)
        .add_card(peer_lv5)
        .add_card(survivor_lv6)
        .hand(1, &["BT17-078", "C-METAL"])
        .memory(0)
        .start();
    runner.game.turn_count = 1;

    // Player 0 (the attacker's controller) attacks; player 1 holds the Counter.
    let attacking = runner.place_on_field(0, "C-ATK", Some(0));
    let _peer = runner.place_on_field(0, "C-PEER5", Some(0));
    let _survivor = runner.place_on_field(0, "C-SURV6", Some(0));
    let target = runner.place_on_field(1, "C-WAR", Some(0));
    let p0_deck_before = runner.deck_size(0);

    let result = runner
        .game
        .begin_attack(attacking, AttackTarget::Digimon(target), false);
    assert_eq!(result, AttackResult::InProgress);

    // Counter window: select BT17-078 as the Blast-DNA result, then field
    // WarGreymon (id 0), then hand MetalGarurumon (PLAY_HAND_START + 1).
    runner
        .game
        .resolve_selection(1, DNA_DIGIVOLVE_START)
        .expect("select BT17-078 as Counter Blast DNA result");
    runner
        .game
        .resolve_selection(1, 0)
        .expect("select field WarGreymon as material");
    runner
        .game
        .resolve_selection(1, PLAY_HAND_START + 1)
        .expect("select hand MetalGarurumon as material");

    // The DNA-gated [When Digivolving] body asks for the level anchor; choosing
    // the Lv.5 anchor bottom-decks both Lv.5 opp Digimon (attacker + peer).
    let choose_level = runner
        .pending_selection_view()
        .expect("BT17-078 DNA-origin clause must ask for the level anchor");
    runner
        .game
        .resolve_selection(1, choose_level.valid_action_ids[0])
        .expect("choose a Lv.5 opponent Digimon as the level anchor");

    assert_eq!(
        runner.deck_size(0),
        p0_deck_before + 2,
        "both Lv.5 opp Digimon (attacker + same-level peer) must be bottom-decked"
    );
    assert_eq!(
        runner.battle_area_size(0),
        1,
        "only the non-matching Lv.6 opp Digimon should remain before the delete step"
    );

    // The mandatory "Then, delete 1 of your opponent's Digimon" prompt installs.
    let delete_prompt = runner
        .pending_selection_view()
        .expect("the delete prompt must install after the same-level bottom-deck");
    assert!(
        !delete_prompt.is_optional,
        "the printed delete step is mandatory once an opponent Digimon remains"
    );
    assert_eq!(
        delete_prompt.selecting_player, 1,
        "the BT17-078 controller (defender, player 1) makes the delete choice"
    );
}

/// Unhappy path (Blast marker exactness): broad [Greymon]/[Garurumon]-named
/// materials do NOT satisfy BT17-078's <Blast DNA Digivolve ([WarGreymon] +
/// [MetalGarurumon])> marker, so no Counter window opens. The combo requires the
/// EXACT named Lv.6 pair — a per-card test sees the marker, but the system fact
/// (the Counter never even offers) is what the combo relies on.
#[test]
fn combo_c_blast_dna_rejects_broad_greymon_garurumon_names() {
    use digimon_engine::enums::GamePhase;
    let greymon = make_named_material("C2-GREY", "Greymon", 6, 8000, CardColor::Red);
    let garurumon = make_named_material("C2-GARU", "Garurumon", 6, 8000, CardColor::Blue);
    let attacker = make_named_material("C2-ATK", "Attacker", 6, 17000, CardColor::Red);

    let mut runner = DebugRunner::builder()
        .dsl_card("BT17-078")
        .expect("BT17-078 (Omnimon) in embedded DSL pack")
        .add_card(greymon)
        .add_card(garurumon)
        .add_card(attacker)
        .hand(1, &["BT17-078", "C2-GARU"])
        .memory(0)
        .start();
    runner.game.turn_count = 1;

    let attacking = runner.place_on_field(0, "C2-ATK", Some(0));
    let target = runner.place_on_field(1, "C2-GREY", Some(0));

    let result = runner
        .game
        .begin_attack(attacking, AttackTarget::Digimon(target), false);
    assert_ne!(result, AttackResult::Invalid, "the attack itself is legal");
    assert_ne!(
        runner.current_phase(),
        GamePhase::CounterTiming,
        "a broad [Greymon] field + [Garurumon] hand must NOT satisfy the exact \
         <Blast DNA ([WarGreymon] + [MetalGarurumon])> marker — no Counter window opens"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Combo D — Free cross-tribe Lv.6 assembly (both DNA materials in one turn)
// ═══════════════════════════════════════════════════════════════════════════
//
// Cards: BT17-015 WarGreymon (Red Lv.6, payoff with the free-digivolve arm) + a
//   field [Gabumon] + a hand [MetalGarurumon].
// Expected outcome (model Combo D): BT17-015's [On Play] branch 1 — "1 of your
//   [Gabumon] may digivolve into [MetalGarurumon] in your hand, ignoring
//   requirements and without paying the cost" — turns a field Gabumon into a
//   Blue Lv.6 MetalGarurumon. Result: a Red Lv.6 (WarGreymon) AND a Blue Lv.6
//   (MetalGarurumon) on field in one turn — exactly the DNA material pair Combos
//   A/C consume.
// Rules basis: §8-1 (effect-driven digivolve, ignore requirements); §8-2. DCGO
//   `BT17/Red/BT17_015.cs`. Engine: cards/bt17/BT17-015.yaml.
//
// The free cross-tribe digivolve is fired through BT17-015's REAL branch-choice
// clause (not a synthetic digivolve), so the assembly is exercised as in play.

fn make_field_gabumon(id: &str) -> CardData {
    let mut c = make_test_card(id, "Gabumon");
    c.card_kind = CardKind::Digimon;
    c.level = Some(3);
    c.dp = Some(3000);
    c.colors = vec![CardColor::Blue];
    c
}

fn make_hand_metalgarurumon(id: &str) -> CardData {
    let mut c = make_test_card(id, "MetalGarurumon");
    c.card_kind = CardKind::Digimon;
    c.level = Some(6);
    c.dp = Some(11000);
    c.play_cost = 12;
    c.colors = vec![CardColor::Blue];
    c
}

/// Happy path: firing BT17-015's [On Play] branch 1 with a field Gabumon + a
/// hand MetalGarurumon assembles the Blue Lv.6 alongside the Red Lv.6 — the DNA
/// material pair, in one turn.
#[test]
fn combo_d_free_cross_tribe_assembly_yields_both_lv6_materials() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT17-015")
        .expect("BT17-015 (WarGreymon) in embedded DSL pack")
        .add_card(make_field_gabumon("D-GABU"))
        .add_card(make_hand_metalgarurumon("D-MG"))
        .hand(0, &["D-MG"])
        .memory(15)
        .start();
    runner.game.turn_count = 1;

    let gabu = runner.place_on_field(0, "D-GABU", None);
    // BT17-015 (the Red Lv.6 WarGreymon) is already in play as the payoff.
    let wargrey = runner.place_on_field(0, "BT17-015", None);

    // Fire BT17-015's [On Play]/[When Digivolving] branch-choice clause.
    runner
        .game
        .enqueue_triggered(EffectTiming::OnPlay, TriggerSource::Permanent(wargrey));
    runner.game.drain_effect_queue();

    // Branch 1 = "Digivolve Gabumon into MetalGarurumon free".
    runner.execute_branch(1).expect("pick the free cross-tribe digivolve branch");
    drive_first_valid(&mut runner, 20);

    // The former Gabumon stack is now topped by the Blue Lv.6 MetalGarurumon.
    let gabu_top = runner.game.players[0].battle_area[gabu.index as usize]
        .top_card()
        .card_id(&runner.game.card_data);
    assert_eq!(
        gabu_top, "D-MG",
        "the field Gabumon must have digivolved into the hand MetalGarurumon (free)"
    );

    // Assert the DNA material PAIR is now present: a Red Lv.6 WarGreymon AND a
    // Blue Lv.6 MetalGarurumon, both on field (read off each stack's top card).
    let data = &runner.game.card_data;
    let tops: Vec<(String, Option<u8>, Vec<CardColor>)> = runner.game.players[0]
        .battle_area
        .iter()
        .map(|p| {
            let top = p.top_card();
            (
                top.card_name(data).to_string(),
                top.level(data),
                top.colors(data).to_vec(),
            )
        })
        .collect();
    let has_red_lv6 = tops
        .iter()
        .any(|(name, lvl, cols)| name == "WarGreymon" && *lvl == Some(6) && cols.contains(&CardColor::Red));
    let has_blue_lv6 = tops
        .iter()
        .any(|(name, lvl, cols)| name == "MetalGarurumon" && *lvl == Some(6) && cols.contains(&CardColor::Blue));
    assert!(
        has_red_lv6 && has_blue_lv6,
        "after the free arm, both a Red Lv.6 (WarGreymon) and a Blue Lv.6 \
         (MetalGarurumon) must be on field — the DNA material pair; tops={tops:?}"
    );
}

/// Unhappy path (enabler absent): with NO [Gabumon] on field, BT17-015's free
/// cross-tribe arm has no base — the MetalGarurumon stays in hand and no second
/// Lv.6 is assembled. The combo's payoff is gated on having the off-tribe rookie
/// to convert, the system fact a per-card test can't express.
#[test]
fn combo_d_no_gabumon_yields_no_second_lv6() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT17-015")
        .expect("BT17-015 (WarGreymon) in embedded DSL pack")
        .add_card(make_hand_metalgarurumon("DN-MG"))
        .hand(0, &["DN-MG"])
        .memory(15)
        .start();
    runner.game.turn_count = 1;

    let wargrey = runner.place_on_field(0, "BT17-015", None);
    let before = snapshot(&runner);

    runner
        .game
        .enqueue_triggered(EffectTiming::OnPlay, TriggerSource::Permanent(wargrey));
    runner.game.drain_effect_queue();
    // Pick the free-digivolve branch; with no Gabumon on field it has no base.
    let _ = runner.execute_branch(1);
    drive_first_valid(&mut runner, 12);
    let after = snapshot(&runner);

    // MetalGarurumon never left the hand; no extra Lv.6 was assembled on field.
    let metalgaru_on_field = runner.game.players[0]
        .battle_area
        .iter()
        .any(|p| p.top_card().card_id(&runner.game.card_data) == "DN-MG");
    assert!(
        !metalgaru_on_field,
        "with no Gabumon base, MetalGarurumon must NOT reach the field"
    );
    assert_eq!(
        after.hand[0], before.hand[0],
        "MetalGarurumon must stay in hand when the free cross-tribe arm has no base"
    );
    assert_eq!(
        after.field[0], before.field[0],
        "no second Lv.6 may be assembled without the off-tribe rookie"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Combo E — Nokia accel into cheap Lv.6 (BT22-013 cost-6 jump)
// ═══════════════════════════════════════════════════════════════════════════
//
// Cards (model Combo E): BT22-084 Nokia Shiramine + BT22-013 WarGreymon + a
//   field [Agumon]. Claimed outcome: with Nokia in play, BT22-013's [Hand][Main]
//   lets an [Agumon] digivolve into BT22-013 for digivolution cost 6, ignoring
//   requirements.
//
// BLOCKED — `#[ignore]`'d on G-ACTIVATED-DIGIVOLVE-EXECUTION
//   (`qa/archetype-qa/engine-gaps.md`). BT22-013's [Hand][Main] cost-6 jump is
//   encoded only as a `kind: activated_digivolve` alt-path, which has NO engine
//   execution route: `dna_digivolve.rs` matches only Digivolve / DnaDigivolve /
//   BlastDnaDigivolve, and the action layer offers no action ID for an
//   activated-digivolve alt-path. The gap entry lists BT22-013 as a residual
//   card. The named cost-6 jump therefore cannot be played or behaviorally
//   driven — its board diff cannot be produced faithfully. (Separately, the
//   Nokia precondition is unenforced because BT22-013.yaml does not yet populate
//   the now-RESOLVED `AltPathSpec.condition:` — a card-local authoring follow-up,
//   distinct from this execution gap.) The executable BT22-013 components (the
//   [When Digivolving] delete / free-digivolve branches) are covered by
//   `tests/cards_behavioral/bt22/bt22_013.rs`; no faithful interaction test can
//   be authored for the named jump until the execution route lands.

#[test]
#[ignore = "G-ACTIVATED-DIGIVOLVE-EXECUTION: BT22-013 [Hand][Main] cost-6 jump is a \
            kind: activated_digivolve alt-path with no engine execution route \
            (qa/archetype-qa/engine-gaps.md); the named combo outcome cannot be \
            produced faithfully. Un-ignore when the execution route lands."]
fn combo_e_nokia_cost6_lv6_jump() {
    // Intentionally empty: there is no faithful way to drive BT22-013's
    // activated_digivolve [Hand][Main] until G-ACTIVATED-DIGIVOLVE-EXECUTION is
    // resolved. Authoring a body now would either bypass the real ability
    // (violating the no-approximations / real-ability rule) or assert a no-op.
    unimplemented!(
        "blocked on G-ACTIVATED-DIGIVOLVE-EXECUTION — see the module-level Combo E note"
    );
}
