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
//! All roles — named combo pieces AND fillers / neutral targets — are loaded as
//! **real implemented DSL cards** (no synthetic `make_test_card`): vanilla
//! Plesiomon / Phoenixmon as the DNA material pair, vanilla Lv.5 Digimon as
//! removal victims, ST/BT WarGreymon / MetalGarurumon / Gabumon / Omnimon for
//! the named combo lanes.
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
//! - E — Nokia cost-6 Lv.6 jump (BT22-013 [Hand][Main]).                        [AUTHORED]

#![allow(dead_code)]

use digimon_engine::action::space::{DNA_DIGIVOLVE_START, PLAY_HAND_START};
use digimon_engine::combat::AttackResult;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::effect_context::EffectContext;
use digimon_engine::enums::{CardColor, EffectSourceKind, EffectTiming};
use digimon_engine::permanent::OptionState;
use digimon_engine::replacement::ReplacementCause;
use digimon_engine::selection::{AttackTarget, OptionPlayResult, TriggerSource};

use super::support::snapshot;

// ─── Real-card cast (all DSL-loadable) ──────────────────────────────────────
//
// Combo A — DNA material pair + removal victims:
//   ST2-10 Plesiomon          Blue Lv.6 DP 12000, vanilla (no effect text).
//   ST1-10 Phoenixmon         Red  Lv.6 DP 12000, vanilla.
//   BT13-112 Omnimon          Black Lv.7 DP 14000 — [When Digivolving]-only
//                             (no On Play/On Deletion), safe deletion victim.
//   BT13-060 Rosemon: Burst Mode  Green Lv.7 DP 15000 — [When Digivolving]/
//                             [When Attacking]-only, safe deletion victim.
//   ST5-10 MetalTyrannomon    Black Lv.5 DP 9000, vanilla survivor.
//
// Combo B — Delay → reactive DNA into an Omnimon:
//   ST1-11 WarGreymon         Red Lv.6 DP 12000 — [Your Turn] passive aura only;
//                             a clean leaving [Greymon] subject.
//   EX4-060 Omnimon Alter-S   Black Lv.7 — [When Digivolving] arms target the
//                             opponent's Digimon (none present in the combo → no
//                             prompt); the DNA result pulled from hand.
//   ST2-10 Plesiomon          Lv.6 hand DNA partner.
//   ST1-04 Dracomon           Red Lv.3 vanilla deck filler.
//
// Combo C — Blast DNA Counter:
//   ST1-11 WarGreymon         field Blast-DNA material A.
//   ST2-11 MetalGarurumon     Blue Lv.6 DP 11000 — [When Attacking]-only;
//                             hand Blast-DNA material B.
//   ST4-09 Okuwamon           Green Lv.5 DP 7000 vanilla attacker (level anchor).
//   ST5-10 MetalTyrannomon    Black Lv.5 DP 9000 vanilla same-level peer.
//   ST2-10 Plesiomon          Blue Lv.6 DP 12000 vanilla non-matching survivor.
//   ST1-07 Greymon / ST2-06 Garurumon  Lv.4 broad-name pair (inherited-only) for
//                             the marker-exactness rejection path.
//
// Combo D — free cross-tribe assembly:
//   BT17-015 WarGreymon       Red Lv.6 payoff (free-digivolve arm).
//   ST2-03 Gabumon            Blue Lv.3 field base.
//   ST2-11 MetalGarurumon     Blue Lv.6 hand digivolve target.

// ═══════════════════════════════════════════════════════════════════════════
// Combo A — DNA Omnimon Alter-S blowout (Blue 6 + Red 6 → EX9-021)
// ═══════════════════════════════════════════════════════════════════════════
//
// Cards: EX9-021 Omnimon Alter-S + ST2-10 Plesiomon (Blue Lv.6) + ST1-10
//   Phoenixmon (Red Lv.6) — the DNA pair — + real opponent Digimon victims.
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
        .dsl_card("ST2-10") // Plesiomon — Blue Lv.6 DNA material
        .expect("ST2-10 (Plesiomon) in embedded DSL pack")
        .dsl_card("ST1-10") // Phoenixmon — Red Lv.6 DNA material
        .expect("ST1-10 (Phoenixmon) in embedded DSL pack")
        // Two opp Digimon tied at the highest level (7) + one strictly lower (5).
        .dsl_card("BT13-112") // Omnimon — Lv.7 victim
        .expect("BT13-112 (Omnimon) in embedded DSL pack")
        .dsl_card("BT13-060") // Rosemon: Burst Mode — Lv.7 victim
        .expect("BT13-060 (Rosemon: Burst Mode) in embedded DSL pack")
        .dsl_card("ST5-10") // MetalTyrannomon — Lv.5 survivor
        .expect("ST5-10 (MetalTyrannomon) in embedded DSL pack")
        .hand(0, &["EX9-021"])
        .memory(10)
        .start();
    runner.game.turn_count = 1;

    runner.place_on_field(1, "BT13-112", None);
    runner.place_on_field(1, "BT13-060", None);
    runner.place_on_field(1, "ST5-10", None);
    let blue = runner.place_on_field(0, "ST2-10", None);
    let red = runner.place_on_field(0, "ST1-10", None);

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
        survivors.iter().any(|n| n == "MetalTyrannomon"),
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
// Cards: BT17-095 (Option, played from hand → seats itself as a Delay) + an own
//   Lv.6 [Greymon] (ST1-11 WarGreymon) on field + an [Omnimon]-name Lv.7
//   (EX4-060 Omnimon Alter-S) in hand + a Lv.6 DNA partner (ST2-10 Plesiomon)
//   in hand + a Red colour anchor (ST1-04 Dracomon) on field for the
//   Option-play colour requirement.
// Expected outcome (model Combo B): BT17-095's [Main] (Clause A) plays no body
//   here (no [Agumon]/[Gabumon] is eligible to recur — the optional union pick
//   declines) and runs its mandatory "Then, place this card in the battle area"
//   tail, seating BT17-095 as a Delay-Option. Then, when an own Lv.6
//   [Greymon]/[Garurumon] *would leave the battle area outside of battle*, the
//   Delay (Clause B) fires: that leaving Lv.6 + a hand card DNA digivolve into
//   an [Omnimon]-name Lv.7 in hand. The leaving Lv.6 is CONSUMED as a DNA
//   material under the merged Omnimon — it does NOT go to trash.
// Rules basis: §8-2 (DNA digivolution); §16 <Delay>. DCGO
//   `BT17/Red/BT17_095.cs` (Clause A place-as-Delay; Clause B WhenRemoveField).
//   Engine: cards/bt17/BT17-095.yaml.
//   The DNA-into-Omnimon body uses `effect_initiated_dna_digivolve_hand_partner`
//   (G-DSL-DNA-FROM-HAND-PARTNER CLOSED 2026-05-20) — a hand card is the 2nd material.
//   The merge runs with ignore_requirements, so any real [Omnimon]-name Lv.7
//   result is a faithful DNA target for ST1-11 WarGreymon + a Lv.6 hand partner.
//
// REAL play path (B3, 2026-06-16): BT17-095 is seated as a Delay through its
//   REAL Option-play lifecycle — `play_option_from_hand` → Clause A [Main] body
//   → `place_self_as_delay_option`. This was previously scaffolded by directly
//   stamping `OptionState::Delayed` (the `seat_as_delay_option` helper, removed)
//   because the engine's `place_self_as_delay_option_permanent` did not claim
//   the in-flight Option from `pending_option` on the real play path
//   (G-OPTION-PLACE-SELF-AS-DELAY-ON-PLAY-PATH). That gap is now RESOLVED
//   (qa/resolved-gaps.md) — the same fix that greens
//   `omnimon_ace::combo1_mega_knight_*` — so the Option-seating scaffold is gone
//   and BT17-095 seats itself through its true [Main] body. The Lv.6 "leaving"
//   trigger is still driven by `delete_permanent_with_cause(_, OwnEffect)` — the
//   ability UNDER TEST (BT17-095's Clause-B reaction) still fires through its
//   real replacement-observer trigger path; only the Option-seating scaffold is
//   removed in B3.

/// True if any of `player`'s battle-area permanents tops with `card_id` and is
/// currently a seated Delay-Option. Used to confirm BT17-095 seated itself as a
/// Delay through its REAL [Main] body (Clause A `place_self_as_delay_option`).
fn delay_option_present(runner: &DebugRunner, player: u8, card_id: &str) -> bool {
    runner.game.players[player as usize].battle_area.iter().any(|p| {
        p.top_card().card_id(&runner.game.card_data) == card_id
            && matches!(p.option_state, OptionState::Delayed { .. })
    })
}

/// Play BT17-095 from `player`'s hand through its REAL Option-play path and let
/// it seat itself as a Delay-Option via Clause A's `place_self_as_delay_option`
/// tail. The optional [Agumon]/[Gabumon] union recursion has no eligible target
/// in the Combo B fixtures, so every installed prompt is driven to its first
/// valid action (which, for the empty optional union pick, is PASS) — leaving
/// the mandatory place-self tail to run. Asserts the Delay actually seated.
fn play_and_seat_bt17_095_as_delay(runner: &mut DebugRunner, player: u8) {
    let gh_idx = runner.game.players[player as usize]
        .hand
        .iter()
        .position(|c| c.card_id(&runner.game.card_data) == "BT17-095")
        .expect("BT17-095 must be in hand to play it via the real Option-play path");
    // Real Option-play lifecycle: pays BT17-095's own cost (2), runs Clause A's
    // [Main] body, and seats it as a Delay via place_self_as_delay_option. The
    // optional union pick parks a `Pending` selection (it offers PASS even with
    // no eligible Agumon/Gabumon); a fully-synchronous seat would return
    // `Delayed`. Either is a legal entry — only `Invalid` is a failure.
    let res = runner.game.play_option_from_hand(player, gh_idx);
    assert_ne!(
        res,
        OptionPlayResult::Invalid,
        "BT17-095 must enter its real Option-play lifecycle (got {res:?})"
    );
    // Drain Clause A's optional union pick (declines — no eligible Agumon/Gabumon)
    // so the mandatory place-self tail runs.
    drive_first_valid(runner, 20);
    assert!(
        delay_option_present(runner, player, "BT17-095"),
        "BT17-095 must seat itself as a Delay-Option through its REAL [Main] body \
         (Clause A place_self_as_delay_option on the real play path)"
    );
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

/// Happy path: an own Lv.6 [Greymon] (ST1-11 WarGreymon) leaving the field
/// outside battle fires the BT17-095 Delay; the leaving WarGreymon + a hand
/// partner DNA digivolve into the [Omnimon]-name Lv.7 (EX4-060) in hand. The
/// merged Omnimon exists with the WarGreymon as a source, and the WarGreymon is
/// NOT in the trash (consumed, not destroyed).
#[test]
fn combo_b_delay_consumes_leaving_lv6_into_merged_omnimon() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT17-095")
        .expect("BT17-095 (Miraculous Mega Knight) in embedded DSL pack")
        .dsl_card("ST1-11") // WarGreymon — own field [Greymon] (leaving subject)
        .expect("ST1-11 (WarGreymon) in embedded DSL pack")
        .dsl_card("EX4-060") // Omnimon Alter-S — [Omnimon]-name Lv.7 result
        .expect("EX4-060 (Omnimon Alter-S) in embedded DSL pack")
        .dsl_card("ST2-10") // Plesiomon — Lv.6 hand DNA partner
        .expect("ST2-10 (Plesiomon) in embedded DSL pack")
        .dsl_card("ST1-04") // Dracomon — Red Lv.3 colour anchor + deck filler
        .expect("ST1-04 (Dracomon) in embedded DSL pack")
        // BT17-095 is in HAND so it is played through its real Option-play path.
        .hand(0, &["BT17-095", "EX4-060", "ST2-10"])
        .deck(0, &["ST1-04"; 5])
        .deck(1, &["ST1-04"; 5])
        .memory(10)
        .start();
    runner.game.turn_count = 1;

    // Red colour anchor on field (satisfies BT17-095's Red+Blue colour
    // requirement at Option-play time, mirroring omnimon_ace combo 1).
    runner.place_on_field(0, "ST1-04", Some(0));
    runner.game.enter_main_phase();

    // Seat BT17-095 as a Delay through its REAL Option-play [Main] body (Clause A
    // `place_self_as_delay_option`) — no `OptionState::Delayed` scaffold.
    play_and_seat_bt17_095_as_delay(&mut runner, 0);

    let greymon_handle = runner.place_on_field(0, "ST1-11", None);
    let greymon_card = runner.top_card(greymon_handle);

    // Trigger the leave of the own field WarGreymon (own-effect, outside battle).
    runner
        .game
        .delete_permanent_with_cause(greymon_handle, ReplacementCause::OwnEffect);

    // The optional replacement installs an accept prompt; accept it, then resolve
    // the Omnimon-result + hand-partner picks.
    drive_first_valid(&mut runner, 20);

    // A merged permanent topped with the Omnimon now exists on P0's field, with
    // the leaving WarGreymon as one of its digivolution sources.
    let merged = runner.game.players[0]
        .battle_area
        .iter()
        .find(|p| p.top_card().card_id(&runner.game.card_data) == "EX4-060")
        .expect("merged Omnimon permanent must exist after the Delay DNA digivolve");
    assert!(
        merged.card_sources.iter().any(|s| s.handle() == greymon_card),
        "the leaving WarGreymon must be a digivolution source under the merged Omnimon"
    );
    assert!(
        merged.card_sources.len() >= 3,
        "merged stack must hold WarGreymon + hand partner + Omnimon result; got {}",
        merged.card_sources.len()
    );

    // The leaving WarGreymon was CONSUMED, not destroyed: it must NOT be in trash.
    let greymon_in_trash = runner.game.players[0]
        .trash
        .iter()
        .any(|c| c.handle() == greymon_card);
    assert!(
        !greymon_in_trash,
        "a successful DNA merge consumes the leaving WarGreymon into the merged \
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
/// `replacement_subject_is_mine`). The opponent's WarGreymon proceeds to trash
/// normally and no merged Omnimon is created — the combo is gated on the leaving
/// Lv.6 being YOURS.
#[test]
fn combo_b_delay_does_not_fire_for_opponent_lv6_leaving() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT17-095")
        .expect("BT17-095 (Miraculous Mega Knight) in embedded DSL pack")
        .dsl_card("ST1-11") // WarGreymon — opponent's field [Greymon]
        .expect("ST1-11 (WarGreymon) in embedded DSL pack")
        .dsl_card("EX4-060") // Omnimon Alter-S — result that must stay in hand
        .expect("EX4-060 (Omnimon Alter-S) in embedded DSL pack")
        .dsl_card("ST2-10") // Plesiomon — DNA partner that must stay in hand
        .expect("ST2-10 (Plesiomon) in embedded DSL pack")
        .dsl_card("ST1-04") // Dracomon — Red Lv.3 colour anchor + deck filler
        .expect("ST1-04 (Dracomon) in embedded DSL pack")
        // BT17-095 is in HAND so it is played through its real Option-play path.
        .hand(0, &["BT17-095", "EX4-060", "ST2-10"])
        .deck(0, &["ST1-04"; 5])
        .deck(1, &["ST1-04"; 5])
        .memory(10)
        .start();
    runner.game.turn_count = 1;

    // Red colour anchor on field for BT17-095's Option-play colour requirement.
    runner.place_on_field(0, "ST1-04", Some(0));
    runner.game.enter_main_phase();

    // Seat BT17-095 as a Delay through its REAL Option-play [Main] body, arming
    // Clause B's leave-observer — no `OptionState::Delayed` scaffold.
    play_and_seat_bt17_095_as_delay(&mut runner, 0);

    // The leaving Lv.6 WarGreymon belongs to the OPPONENT (player 1).
    let opp_greymon = runner.place_on_field(1, "ST1-11", None);

    // Snapshot AFTER the play+seat: P0's hand now holds only the result + partner
    // (EX4-060, ST2-10); the assertions below verify the Delay does NOT fire.
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
        .any(|p| p.top_card().card_id(&runner.game.card_data) == "EX4-060");
    assert!(
        !merged_exists,
        "no merged Omnimon may be created for an opponent's leaving Digimon"
    );
    assert_eq!(
        after.hand[0], before.hand[0],
        "the Omnimon result + DNA partner must stay in hand (Delay did not fire)"
    );
    // The opponent's WarGreymon goes to trash normally.
    assert_eq!(
        after.field[1],
        before.field[1] - 1,
        "the opponent's leaving WarGreymon should leave its battle area as normal"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Combo C — Blast DNA Omnimon off the opponent's turn (BT17-078 Counter)
// ═══════════════════════════════════════════════════════════════════════════
//
// Cards: BT17-078 (hand) + ST1-11 WarGreymon field material + ST2-11
//   MetalGarurumon hand material; opponent attacking + opponent Digimon victims.
// Expected outcome (model Combo C): at Counter timing (when attacked), Blast DNA
//   Digivolve — field WarGreymon + hand MetalGarurumon become BT17-078's sources
//   at no cost. [When Digivolving] (DNA-gated): choose 1 opp Digimon, bottom-deck
//   ALL opp Digimon of that level, then a mandatory delete of 1 more opp Digimon.
// Rules basis: §16 <Blast DNA Digivolve>; §8-2. DCGO `BT17/White/BT17_078.cs`.
//   Engine: cards/bt17/BT17-078.yaml. The Blast DNA marker requires the EXACT
//   pair name_is "WarGreymon" + name_is "MetalGarurumon" — ST1-11 + ST2-11 fit.
//
// Driven through the REAL combat/Counter route (`begin_attack` → Counter window
// → Blast-DNA material selection), then the real DNA-gated bottom-deck + delete.

/// Happy path: BT17-078 Blast DNA at Counter timing (field WarGreymon + hand
/// MetalGarurumon) bottom-decks every opponent Digimon at the chosen level, then
/// installs the mandatory extra-delete prompt — all on the opponent's turn.
#[test]
fn combo_c_blast_dna_counter_bottom_decks_same_level_then_prompts_delete() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT17-078")
        .expect("BT17-078 (Omnimon) in embedded DSL pack")
        .dsl_card("ST1-11") // WarGreymon — field Blast-DNA material
        .expect("ST1-11 (WarGreymon) in embedded DSL pack")
        .dsl_card("ST2-11") // MetalGarurumon — hand Blast-DNA material
        .expect("ST2-11 (MetalGarurumon) in embedded DSL pack")
        .dsl_card("ST4-09") // Okuwamon — Lv.5 attacker (level anchor)
        .expect("ST4-09 (Okuwamon) in embedded DSL pack")
        .dsl_card("ST5-10") // MetalTyrannomon — Lv.5 same-level peer
        .expect("ST5-10 (MetalTyrannomon) in embedded DSL pack")
        .dsl_card("ST2-10") // Plesiomon — Lv.6 non-matching survivor
        .expect("ST2-10 (Plesiomon) in embedded DSL pack")
        .hand(1, &["BT17-078", "ST2-11"])
        .memory(0)
        .start();
    runner.game.turn_count = 1;

    // Player 0 (the attacker's controller) attacks; player 1 holds the Counter.
    let attacking = runner.place_on_field(0, "ST4-09", Some(0));
    let _peer = runner.place_on_field(0, "ST5-10", Some(0));
    let _survivor = runner.place_on_field(0, "ST2-10", Some(0));
    let target = runner.place_on_field(1, "ST1-11", Some(0));
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
/// EXACT named pair — ST1-07 "Greymon" + ST2-06 "Garurumon" are real cards whose
/// names do not match the `name_is` "WarGreymon"/"MetalGarurumon" filter. A
/// per-card test sees the marker, but the system fact (the Counter never even
/// offers) is what the combo relies on.
#[test]
fn combo_c_blast_dna_rejects_broad_greymon_garurumon_names() {
    use digimon_engine::enums::GamePhase;
    let mut runner = DebugRunner::builder()
        .dsl_card("BT17-078")
        .expect("BT17-078 (Omnimon) in embedded DSL pack")
        .dsl_card("ST1-07") // Greymon — broad name, NOT "WarGreymon"
        .expect("ST1-07 (Greymon) in embedded DSL pack")
        .dsl_card("ST2-06") // Garurumon — broad name, NOT "MetalGarurumon"
        .expect("ST2-06 (Garurumon) in embedded DSL pack")
        .dsl_card("ST4-09") // Okuwamon — neutral attacker
        .expect("ST4-09 (Okuwamon) in embedded DSL pack")
        .hand(1, &["BT17-078", "ST2-06"])
        .memory(0)
        .start();
    runner.game.turn_count = 1;

    let attacking = runner.place_on_field(0, "ST4-09", Some(0));
    let target = runner.place_on_field(1, "ST1-07", Some(0));

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
//   field [Gabumon] (ST2-03) + a hand [MetalGarurumon] (ST2-11).
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

/// Happy path: firing BT17-015's [On Play] branch 1 with a field Gabumon + a
/// hand MetalGarurumon assembles the Blue Lv.6 alongside the Red Lv.6 — the DNA
/// material pair, in one turn.
#[test]
fn combo_d_free_cross_tribe_assembly_yields_both_lv6_materials() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT17-015")
        .expect("BT17-015 (WarGreymon) in embedded DSL pack")
        .dsl_card("ST2-03") // Gabumon — field base
        .expect("ST2-03 (Gabumon) in embedded DSL pack")
        .dsl_card("ST2-11") // MetalGarurumon — hand digivolve target
        .expect("ST2-11 (MetalGarurumon) in embedded DSL pack")
        .hand(0, &["ST2-11"])
        .memory(15)
        .start();
    runner.game.turn_count = 1;

    let gabu = runner.place_on_field(0, "ST2-03", None);
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
        gabu_top, "ST2-11",
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
        .dsl_card("ST2-11") // MetalGarurumon — hand digivolve target (no base)
        .expect("ST2-11 (MetalGarurumon) in embedded DSL pack")
        .hand(0, &["ST2-11"])
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
        .any(|p| p.top_card().card_id(&runner.game.card_data) == "ST2-11");
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
// Cards (model Combo E): BT22-084 Nokia Shiramine (Tamer) + BT22-013 WarGreymon
//   (hand) + a real [Agumon] (BT22-008) on field. Claimed outcome: with Nokia in
//   play, BT22-013's [Hand][Main] lets an [Agumon] digivolve into BT22-013 for a
//   digivolution cost of 6, ignoring requirements — and the resulting digivolve
//   fires BT22-013's [When Digivolving] branch-choice.
//
// AUTHORED — G-ACTIVATED-DIGIVOLVE-EXECUTION is RESOLVED (qa/resolved-gaps.md).
//   The jump was re-modelled (gap-closure Tasks A1–A3) off the unreachable
//   `kind: activated_digivolve` alt-path onto a `when: main_from_hand` triggered
//   clause whose `condition:` enforces BOTH the Nokia "If you have [Nokia
//   Shiramine]" precondition AND the [Agumon]-target existence; its body runs
//   `select_own_permanent { Agumon } → effect_initiated_digivolve { from_hand:
//   self, cost: 6, ignore_requirements: true }` (mirrors BT24-016 Lamiamon
//   clause 1). The engine now offers a Hand [Main] action for the card whose
//   gate passes; `activate_hand_main` runs it. So the named combo IS driveable
//   through the card's REAL ability — no `activated_digivolve` execution route
//   is needed, and zero engine code changed. The per-card mechanism is pinned by
//   `tests/cards_behavioral/bt22/bt22_013.rs::bt22_013_hand_main_jump_*`; this
//   interaction test pins the combined-system outcome (REAL Nokia + REAL Agumon
//   stack → cost-6 jump → [When Digivolving] branch-choice fires) plus the
//   Nokia-absent gate that a per-card test can isolate but the combo relies on.
//
// Driven through the REAL ability (`activate_hand_main` → resolve the Agumon
// select), not a low-level digivolve helper — so the cost-6 deduction and the
// [When Digivolving] branch-choice both run as in play.

/// Hand index of `card_id` in `player`'s hand (local helper — the archetype
/// harness does not export one).
fn hand_index_of_e(runner: &DebugRunner, player: u8, card_id: &str) -> usize {
    runner.game.players[player as usize]
        .hand
        .iter()
        .position(|c| c.card_id(&runner.game.card_data) == card_id)
        .unwrap_or_else(|| panic!("{card_id} must be in player {player}'s hand"))
}

/// Happy path: REAL Nokia Shiramine (BT22-084) + a REAL [Agumon] (BT22-008) on
/// P0's field + BT22-013 WarGreymon in hand. Activating BT22-013's [Hand][Main]
/// jump digivolves the Agumon into WarGreymon at digivolution cost 6, ignoring
/// requirements — the Agumon stack is now topped by BT22-013, exactly 6 memory
/// was paid, and BT22-013's [When Digivolving] branch-choice fired as a result.
#[test]
fn combo_e_nokia_cost6_lv6_jump() {
    use digimon_engine::selection::SelectionKind;

    let mut runner = DebugRunner::builder()
        .dsl_card("BT22-013") // WarGreymon — the [Hand][Main] cost-6 jump payoff
        .expect("BT22-013 (WarGreymon) in embedded DSL pack")
        .dsl_card("BT22-084") // Nokia Shiramine — the real Tamer precondition
        .expect("BT22-084 (Nokia Shiramine) in embedded DSL pack")
        .dsl_card("BT22-008") // Agumon — the real digivolve base
        .expect("BT22-008 (Agumon) in embedded DSL pack")
        .dsl_card("ST1-04") // Dracomon — vanilla deck filler
        .expect("ST1-04 (Dracomon) in embedded DSL pack")
        .hand(0, &["BT22-013"])
        .deck(0, &["ST1-04"; 5])
        .deck(1, &["ST1-04"; 5])
        .memory(15)
        .start();
    runner.game.turn_count = 1;

    // REAL Nokia Shiramine (Tamer) + REAL Agumon (BT22-008) on player 0's field.
    // `place_on_field` does NOT fire On Play, so Nokia's [On Play] free-play and
    // the Agumon's own clauses stay dormant — only the static gate matters here.
    runner.place_on_field(0, "BT22-084", Some(0));
    runner.place_on_field(0, "BT22-008", Some(0));

    let mem_before = runner.memory();
    let bt22_013_idx = hand_index_of_e(&runner, 0, "BT22-013");

    // Drive the REAL [Hand][Main] Nokia jump.
    assert!(
        runner.game.activate_hand_main(0, bt22_013_idx),
        "the [Hand][Main] Nokia jump must fire (real Nokia Shiramine + real Agumon present)"
    );

    // The jump body asks which Agumon to digivolve into; pick the (only) one.
    let view = runner
        .pending_selection_view()
        .expect("the Agumon select prompt must install");
    runner
        .execute_action(view.selecting_player, view.valid_action_ids[0])
        .expect("select the Agumon");

    // System-level fact: completing the digivolve fires BT22-013's
    // [When Digivolving] branch-choice (a 2-way EffectChoice). With no opp
    // Digimon and no own Gabumon, both sub-branches no-op, but the choice
    // prompt itself MUST install — proving the jump was a real digivolve, not a
    // board mutation.
    assert_eq!(
        runner.pending_kind(),
        Some(SelectionKind::EffectChoice),
        "the cost-6 jump must be a real digivolve → BT22-013's [When Digivolving] \
         branch-choice fires as a result"
    );
    runner.auto_resolve().expect("branch-choice + follow-ups resolve");

    // The Agumon stack is now topped by WarGreymon (BT22-013).
    let agumon_perm = runner
        .game
        .player(0)
        .battle_area
        .iter()
        .find(|p| {
            p.card_sources
                .iter()
                .any(|s| s.card_id(&runner.game.card_data) == "BT22-008")
        })
        .expect("the Agumon permanent must still be on the field");
    assert_eq!(
        agumon_perm.top_card().card_id(&runner.game.card_data),
        "BT22-013",
        "WarGreymon must be the top card of the Agumon stack after the [Hand][Main] jump"
    );

    // WarGreymon left the hand.
    assert_eq!(
        runner.hand_size(0),
        0,
        "WarGreymon must leave the hand after digivolving onto the Agumon"
    );

    // The cost-6 digivolve actually deducted 6 memory — the memory delta proves
    // the cost was paid (a silently-ignored cost would leave the delta at 0).
    assert_eq!(
        mem_before - runner.memory(),
        6,
        "the [Hand][Main] jump must pay digivolution cost 6 (before={}, after={})",
        mem_before,
        runner.memory(),
    );
}

/// Unhappy path (Nokia precondition gate): with the REAL [Agumon] (BT22-008) on
/// field but NO Nokia Shiramine, the masked [Hand][Main] action is NOT offered —
/// `activate_hand_main` returns false, no selection installs, and no digivolve
/// happens. The whole jump is gated on the Nokia precondition; this is the
/// system fact the combo depends on but a per-card test isolates.
#[test]
fn combo_e_nokia_jump_gated_on_nokia_precondition() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT22-013")
        .expect("BT22-013 (WarGreymon) in embedded DSL pack")
        .dsl_card("BT22-008") // Agumon present — but no Nokia
        .expect("BT22-008 (Agumon) in embedded DSL pack")
        .dsl_card("ST1-04")
        .expect("ST1-04 (Dracomon) in embedded DSL pack")
        .hand(0, &["BT22-013"])
        .deck(0, &["ST1-04"; 5])
        .deck(1, &["ST1-04"; 5])
        .memory(15)
        .start();
    runner.game.turn_count = 1;

    // Agumon on field, but NO Nokia Shiramine — the Nokia gate must block the jump.
    runner.place_on_field(0, "BT22-008", Some(0));

    let before = snapshot(&runner);
    let bt22_013_idx = hand_index_of_e(&runner, 0, "BT22-013");

    assert!(
        !runner.game.activate_hand_main(0, bt22_013_idx),
        "without Nokia Shiramine the [Hand][Main] condition fails — the jump must not fire"
    );
    assert!(
        runner.pending_selection().is_none(),
        "no selection installs when the Nokia-gated [Hand][Main] jump is not offered"
    );

    // The Agumon stack is untouched (WarGreymon never digivolved onto it).
    let agumon_perm = runner
        .game
        .player(0)
        .battle_area
        .iter()
        .find(|p| {
            p.card_sources
                .iter()
                .any(|s| s.card_id(&runner.game.card_data) == "BT22-008")
        })
        .expect("the Agumon permanent must still be on the field");
    assert_eq!(
        agumon_perm.top_card().card_id(&runner.game.card_data),
        "BT22-008",
        "no digivolve — the Agumon must still be the top card of its own stack"
    );
    let after = snapshot(&runner);
    assert_eq!(
        after.hand[0], before.hand[0],
        "WarGreymon must remain in hand — the jump was not offered"
    );
    assert_eq!(
        after.memory, before.memory,
        "no cost is paid when the Nokia-gated jump is not offered"
    );
}
