//! Cluster C — cost payability at DECLARATION vs after.
//!
//! Questions (see `card-resolution.md`):
//!   Q5  Omnimon (AD1-025) `[Assembly]` is a legal declaration even if the full
//!       cost isn't currently affordable, as long as it can be MADE payable —
//!       judge: YES.  [READY: AD1-025 impl; needs WarGreymon/MetalGarurumon in
//!       trash — AD1-004/AD1-014 are implemented.]
//!   Q26 Dorbickmon (EX3-014) with [DigiXros] targeting WarGreymon; Miraculous
//!       Mega Knight (BT17-095) DNA-evolves mid-resolution, removing the
//!       WarGreymon ⇒ Dorbickmon's cost becomes unpayable — judge: returns to
//!       hand.
//!   Q27 Same board — judge: pays 0 memory.
//!   Q30 (shared with cluster E) MedievalGallantmon (EX8-074) `<Partition>` is
//!       interruptive; cost-reduction lets it suspend Imperialdramon: Dragon
//!       Mode (EX3-063) + Chaosmon: Valdur Arm (BT20-037).
//!
//! Scenarios authored under tasks §5.

#![allow(unused_imports)]

use digimon_engine::action::{build_action_mask, PLAY_HAND_START};
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind};
use digimon_engine::permanent::PermanentHandle;

/// Push a card already present in `card_data` onto player `p`'s trash.
fn seed_trash(runner: &mut DebugRunner, p: u8, card_id: &str) {
    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .unwrap_or_else(|| panic!("{card_id} present in card_data"));
    let idx = runner.game.next_card_index();
    runner.game.players[p as usize]
        .trash
        .push(CardSource::new(data_idx, p, idx));
}

/// Seat the BT17-095 Option permanent at `handle` as a Delay-Option so its
/// Clause B `when_would_leave_battle_area` observer can fire (gated on
/// `source_is_delayed_option`). Mirrors the helper in
/// `tests/cards_behavioral/bt17/bt17_095.rs`.
fn seat_as_delay_option(runner: &mut DebugRunner, handle: PermanentHandle) {
    use digimon_engine::permanent::OptionState;
    let turn = runner.game.turn_count;
    let perm = &mut runner.game.players[handle.player as usize].battle_area[handle.index as usize];
    perm.option_state = OptionState::Delayed {
        owner: handle.player,
        trash_on_turn: turn + 2,
        trigger: digimon_engine::enums::DelayTrigger::EndOfYourNextTurn,
        placed_on_turn: turn,
    };
}

/// Stage the shared Q26/Q27 board: Player A plays Dorbickmon (EX3-014) via
/// [DigiXros] with WarGreymon (AD1-004) as one material; BT17-095 is seated as a
/// Delay-Option watching WarGreymon's departure; AD1-025 (Omnimon DNA result)
/// and a Lv6 [Garurumon] partner sit in hand for BT17-095's <Delay> DNA-evo.
/// Returns the runner with the DigiXros play DECLARED and all 5 materials
/// selected (WarGreymon + 4 hand Dragons), poised to commit.
fn stage_q26_board() -> (DebugRunner, i16) {
    let mut builder = DebugRunner::builder()
        .dsl_card("EX3-014")
        .expect("EX3-014 Dorbickmon loads")
        .dsl_card("AD1-004")
        .expect("AD1-004 WarGreymon loads")
        .dsl_card("AD1-025")
        .expect("AD1-025 Omnimon loads")
        .from_dsl_yaml(include_str!("../../cards/bt17/BT17-095.yaml"))
        .expect("BT17-095 Miraculous Mega Knight loads")
        .add_card({
            // Lv6 [Garurumon] DNA partner in hand for BT17-095's DNA-evo.
            let mut c = make_test_card("Q26-GARU", "MetalGarurumon");
            c.card_kind = CardKind::Digimon;
            c.level = Some(6);
            c.dp = Some(11000);
            c.play_cost = 11;
            c.colors = vec![CardColor::Blue];
            c
        });
    for i in 0..4 {
        builder = builder.add_card({
            let mut c = make_test_card(&format!("Q26-DRG{i}"), &format!("Dragon{i}"));
            c.card_kind = CardKind::Digimon;
            c.level = Some(4);
            c.dp = Some(4000);
            c.play_cost = 4;
            c.colors = vec![CardColor::Red];
            c.traits = vec!["Dragon".to_string()];
            c
        });
    }
    let mut r = builder
        .add_card({
            let mut c = make_test_card("Q26-FILL", "Filler");
            c.card_kind = CardKind::Digimon;
            c
        })
        // Dorbickmon (idx 0) + 4 Dragon materials + Omnimon DNA result + L6 partner.
        .hand(
            0,
            &[
                "EX3-014", "Q26-DRG0", "Q26-DRG1", "Q26-DRG2", "Q26-DRG3", "AD1-025", "Q26-GARU",
            ],
        )
        .deck(0, &["Q26-FILL"; 5])
        .deck(1, &["Q26-FILL"; 5])
        .memory(13)
        .start();
    r.skip_mulligan();

    let wargreymon = r.place_on_field(0, "AD1-004", Some(0));
    let mmk = r.place_on_field(0, "BT17-095", Some(0));
    seat_as_delay_option(&mut r, mmk);

    let memory_before = r.memory();

    // Declare the DigiXros play of Dorbickmon and select WarGreymon + 4 hand
    // Dragons as the 5 materials (no auto-select — surfaced material actions).
    let _ = r.play(0, 0);
    let wargreymon_action = wargreymon.index as u16;
    assert!(
        r.pending_selection()
            .is_some_and(|s| s.valid_action_ids.contains(&wargreymon_action)),
        "precondition: WarGreymon must be a legal DigiXros material"
    );
    r.execute_action(0, wargreymon_action)
        .expect("select WarGreymon as DigiXros material");
    for _ in 0..4 {
        let Some(sel) = r.pending_selection() else { break };
        if sel.kind != digimon_engine::selection::SelectionKind::Material {
            break;
        }
        let Some(pick) = sel
            .valid_action_ids
            .iter()
            .copied()
            .find(|&a| a != digimon_engine::action::space::PASS)
        else {
            break;
        };
        r.execute_action(0, pick).expect("select DigiXros material");
    }
    (r, memory_before)
}

// ─────────────────────────────────────────────────────────────────────────────
// Q5 — Omnimon (AD1-025) `[Assembly]` declare-then-pay legality
// ─────────────────────────────────────────────────────────────────────────────
//
// Board (card-resolution.md Q5): memory gauge 0 on Player A's side; Player A has
// a WarGreymon and a MetalGarurumon in trash; Player A declares to play Omnimon
// (AD1-025) via `[Assembly]`. JUDGE ANSWER: YES, legal — you may declare a play
// if it is possible to MAKE the cost become payable; declaring `[Assembly]`
// (reduce cost 6, materials WarGreymon × MetalGarurumon) makes it payable.
//
// DCGO: AD1_025.cs:214-255 — `AddAssemblyConditionClass`, elements
// `[WarGreymon]` ×1 + `[MetalGarurumon]` ×1, `reduceCost: 6`.
//
// ── DISCOVERY-WAVE FINDING (2026-05-28) ──────────────────────────────────────
// BLOCKED-DATA at the source-data layer (distinct from Q2's engine-primitive
// block): AD1-025's entry in `data/cards.json` has NO `[Assembly]` keyword — it
// captures only `<Raid>/<Blocker>/<Partition>`, the `[On Play]` body, and
// `xros_req = [DNA Digivolve] … Cost 0`. The real card HAS `[Assembly]`
// (confirmed in DCGO AD1_025.cs:214-255), so this is a card-data INGEST GAP.
// Consequently `cards/ad1/AD1-025.yaml` has only a `dna_digivolve` alt_path and
// no `assembly` alt_path. The `assembly` alt-path KIND itself is supported by
// the DSL (`CompiledAltPathKind::Assembly`; BT18-102 uses it), so once the data
// is corrected this is authorable without an engine change.
//
// ── RESOLVED (2026-05-29, change `fix-ad1-025-assembly-data`) ────────────────
// The un-block sequence completed: (1) `[Assembly]` added to AD1-025 in
// `data/card_overrides.json`; (2) the `assembly` alt_path authored in
// `cards/ad1/AD1-025.yaml` (materials WarGreymon × MetalGarurumon, zones
// [trash], stack_under, reduce cost 6); (3) the engine Assembly executor wired
// (G-ASSEMBLY-PLAY-EXECUTION — eligibility from trash, surfaced per-element
// selection, bottom placement, reduced cost, declare-then-pay mask). Q5 now
// pins as a live mask assertion.

/// Q5 — Omnimon (AD1-025) `[Assembly]` is a legal declaration at memory 0
/// because the reduced cost (15 − 6 = 9) can be MADE payable: with both
/// materials in trash, 0 − 9 = −9 ≥ the −10 floor, so the mask offers the play.
/// Without the assembly reduction the full cost (15) would overdraw past −10
/// and the play would be illegal — confirming the judge's declare-then-pay rule.
#[test]
fn q5_assembly_declaration_legal_when_cost_can_be_made_payable() {
    let mut r = DebugRunner::builder()
        .dsl_card("AD1-025")
        .expect("AD1-025 Omnimon loads from the embedded pack")
        .dsl_card("AD1-004")
        .expect("AD1-004 WarGreymon loads")
        .dsl_card("AD1-014")
        .expect("AD1-014 MetalGarurumon loads")
        .hand(0, &["AD1-025"])
        .memory(0)
        .start();
    r.skip_mulligan();

    // Player A has a WarGreymon and a MetalGarurumon in trash.
    seed_trash(&mut r, 0, "AD1-004");
    seed_trash(&mut r, 0, "AD1-014");

    let mask = build_action_mask(&r.game, 0);
    assert_eq!(
        mask[PLAY_HAND_START as usize], 1.0,
        "Q5: declaring Omnimon via [Assembly] is legal at memory 0 — the \
         reduced cost (15 − 6 = 9) can be made payable (judge: YES)"
    );
}

// ── Q26 / Q27 / Q30 — BLOCKED-CARD ───────────────────────────────────────────

/// Q26 — Dorbickmon (EX3-014) [DigiXros] targeting WarGreymon; Miraculous Mega
/// Knight (BT17-095) DNA-evolves mid-resolution, removing WarGreymon ⇒
/// Dorbickmon's cost becomes unpayable. Judge: returns to hand.
///
/// ── PARTIAL (2026-06-03) — leave-trigger LANDED, redirect-extraction REMAINS ──
/// G-DIGIXROS-DEPARTURE-LEAVE-TRIGGER is now CLOSED: a DigiXros battle-area
/// material consumption fires the `WhenWouldLeaveBattleArea` replacement window
/// with the new `ReplacementCause::DigiXros` (not `Battle`), so BT17-095's
/// `[All Turns]` leave observer DOES now see WarGreymon's DigiXros departure and
/// installs its `<Delay>` accept (judge-quiz Q25 PASSES on the same substrate).
///
/// What still blocks Q26/Q27 (G-DIGIXROS-REDIRECT-EXTRACTION +
/// G-DIGIXROS-UNPAYABLE-RETURN-TO-HAND): when the leave observer parks its
/// optional `<Delay>` accept, the engine commits the DigiXros host immediately
/// (so the departing WarGreymon stops being a standalone top card — what Q25/Q26
/// preconditions require) by tucking WarGreymon UNDER Dorbickmon. But BT17-095's
/// DNA-evo (driven later via `auto_resolve`) must instead pull WarGreymon OUT of
/// the transaction and INTO an Omnimon — which needs WarGreymon to remain a
/// resolvable, EXTRACTABLE subject (not buried under the already-committed host)
/// at the moment the player accepts the Delay. The two preconditions are in
/// direct tension: Q25 (no `auto_resolve`) wants WarGreymon already gone while
/// the accept is parked; Q26 wants the parked accept's DNA-evo to still extract
/// WarGreymon. Satisfying both requires a dedicated "leaving / limbo" holding
/// slot — a battle-area-departing material that is (a) no longer any permanent's
/// top card and (b) still resolvable by `resolve_stable_replacement_subject` and
/// extractable by the DNA-evo. That zone primitive does not exist yet.
///
/// Once a material can be DNA-extracted from the in-flight transaction, the
/// declare-then-pay recompute + return-to-hand machinery is ALREADY in place
/// (`finalize_digixros_play_after_leave_windows`: prunes vanished battle-area
/// materials, recomputes `(final_cost - total_reduction)`, and returns the
/// played card to hand for 0 memory when unpayable). It just never runs on the
/// park path because the host is committed eagerly on park (above).
///
/// Captured real failure (ran WITHOUT #[ignore], 2026-06-03, on the new
/// substrate): the leave window fires and BT17-095's Delay accept surfaces, but
/// because the host is committed on park, Dorbickmon (EX3-014) entered Player
/// A's battle area with WarGreymon tucked under it; `auto_resolve`'s DNA-evo
/// could not extract WarGreymon. Dorbickmon did NOT return to hand
/// (`dorbickmon_returned_to_hand` failed: in_hand=false, on_field=true).
///
/// Logged to qa/archetype-qa/engine-gaps.md
/// (G-DIGIXROS-REDIRECT-EXTRACTION, G-DIGIXROS-UNPAYABLE-RETURN-TO-HAND).
#[test]
#[ignore = "PARTIAL G-DIGIXROS-REDIRECT-EXTRACTION: the DigiXros leave window now fires (Q25 PASS) and the cost-recompute / return-to-hand machinery exists, but on a parked <Delay> accept the host is committed eagerly (so the departing material is no longer a standalone top card), which leaves BT17-095's DNA-evo unable to EXTRACT WarGreymon from the in-flight transaction. Needs a leaving/limbo holding slot. See qa/archetype-qa/engine-gaps.md."]
fn q26_dorbickmon_returns_to_hand_when_cost_unpayable_after_dna_evo() {
    let (mut r, _memory_before) = stage_q26_board();

    // Finish material selection — the DigiXros play would commit here. In a
    // faithful engine BT17-095's [All Turns] observer fires on WarGreymon's
    // departure mid-DigiXros, its <Delay> DNA-evo pulls WarGreymon (+ the hand
    // partner) into Omnimon, removing WarGreymon as a Dorbickmon material ⇒
    // Dorbickmon can no longer pay its DigiXros cost ⇒ Dorbickmon returns to
    // hand (judge Q26).
    if r
        .pending_selection()
        .is_some_and(|s| s.kind == digimon_engine::selection::SelectionKind::Material)
    {
        let _ = r.execute_action(0, digimon_engine::action::space::PASS);
    }
    // Drive any DNA-evo / Delay flow the observer would surface (none in the
    // current engine — the observer never fires).
    let _ = r.auto_resolve();

    // Load-bearing precondition: the play was genuinely declared with WarGreymon
    // selected as a material (WarGreymon is no longer a standalone permanent).
    let wargreymon_standalone = r.game.players[0]
        .battle_area
        .iter()
        .any(|p| p.top_card().card_id(&r.game.card_data) == "AD1-004");
    assert!(
        !wargreymon_standalone,
        "precondition: WarGreymon must have left the battle area as a DigiXros material"
    );

    // JUDGE Q26: Dorbickmon returns to hand (unpayable play returns to hand).
    let dorbickmon_in_hand = r.game.players[0]
        .hand
        .iter()
        .any(|c| c.card_id(&r.game.card_data) == "EX3-014");
    let dorbickmon_on_field = r.game.players[0]
        .battle_area
        .iter()
        .any(|p| p.top_card().card_id(&r.game.card_data) == "EX3-014");
    assert!(
        dorbickmon_in_hand && !dorbickmon_on_field,
        "dorbickmon_returned_to_hand — Dorbickmon must RETURN TO HAND when its \
         DigiXros cost becomes unpayable after BT17-095's DNA-evo pulls \
         WarGreymon out (judge Q26); instead in_hand={dorbickmon_in_hand}, \
         on_field={dorbickmon_on_field}"
    );
}

/// Q27 — Same board. Judge: pays 0 memory (cost unpayable ⇒ no payment).
///
/// ── PARTIAL (same residual as Q26: G-DIGIXROS-REDIRECT-EXTRACTION) ────────────
/// The DigiXros leave window now fires (Q25 PASS) and the unpayable-return-to-
/// hand / 0-memory branch exists in `finalize_digixros_play_after_leave_windows`
/// — but it only runs on the non-park resolution path. On a parked `<Delay>`
/// accept the host is committed eagerly (paying the reduced cost) so BT17-095's
/// DNA-evo cannot extract WarGreymon and trigger the recompute. See Q26's note.
///
/// Captured real failure (ran WITHOUT #[ignore], 2026-06-03, new substrate):
/// the reduced DigiXros cost WAS paid (memory 10→7) because the host committed
/// on the parked accept; judge requires 0 memory (`zero_memory_paid` failed).
///
/// Logged to qa/archetype-qa/engine-gaps.md (G-DIGIXROS-REDIRECT-EXTRACTION,
/// G-DIGIXROS-UNPAYABLE-RETURN-TO-HAND).
#[test]
#[ignore = "PARTIAL G-DIGIXROS-REDIRECT-EXTRACTION: see Q26. The leave window fires and the 0-memory return-to-hand branch exists, but the host commits eagerly on the parked <Delay> accept (paying the reduced cost), so the DNA-evo can't extract WarGreymon to trigger the recompute. See qa/archetype-qa/engine-gaps.md."]
fn q27_dorbickmon_pays_zero_memory_when_returned_to_hand() {
    let (mut r, memory_before) = stage_q26_board();

    if r
        .pending_selection()
        .is_some_and(|s| s.kind == digimon_engine::selection::SelectionKind::Material)
    {
        let _ = r.execute_action(0, digimon_engine::action::space::PASS);
    }
    let _ = r.auto_resolve();

    // Load-bearing precondition: the play was genuinely declared (WarGreymon
    // selected as a material; it is no longer a standalone permanent).
    let wargreymon_standalone = r.game.players[0]
        .battle_area
        .iter()
        .any(|p| p.top_card().card_id(&r.game.card_data) == "AD1-004");
    assert!(
        !wargreymon_standalone,
        "precondition: WarGreymon must have left the battle area as a DigiXros material"
    );

    // JUDGE Q27: the unpayable play pays 0 memory — memory is unchanged.
    let memory_after = r.memory();
    assert_eq!(
        memory_after, memory_before,
        "zero_memory_paid — an unpayable Dorbickmon DigiXros play (cost becomes \
         unpayable after BT17-095's DNA-evo pulls WarGreymon out) must pay 0 \
         memory and leave memory unchanged (judge Q27); before={memory_before}, \
         after={memory_after}"
    );
}

/// Q30 (also cluster E) — MedievalGallantmon (EX8-074) `<Partition>` is
/// interruptive; cost-reduction lets it suspend Imperialdramon: Dragon Mode
/// (EX3-063) + Chaosmon: Valdur Arm (BT20-037) (BanchoLeomon BT20-036 not yet in
/// play). Judge: suspend both with cost reduction.
#[test]
#[ignore = "BLOCKED-CARD: needs BT20-037 (Chaosmon: Valdur Arm), BT20-036 (BanchoLeomon), EX3-063 (Imperialdramon: Dragon Mode), BT16-077 (Dinobeemon), EX3-008 (Flamedramon). EX8-074 implemented."]
fn q30_partition_interruptive_suspends_both_with_cost_reduction() {}
