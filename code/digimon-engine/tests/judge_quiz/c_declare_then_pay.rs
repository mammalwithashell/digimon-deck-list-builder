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
        let Some(sel) = r.pending_selection() else {
            break;
        };
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
/// ── RESOLVED (2026-06-03, G-DIGIXROS-REDIRECT-EXTRACTION) ─────────────────────
/// The leaving/limbo holding slot now exists (`Game::digixros_leaving_limbo`).
/// When a DigiXros battle-area material's `WhenWouldLeaveBattleArea` window parks
/// an optional reward (BT17-095's `<Delay>` accept), the material is moved OUT of
/// `battle_area` into the limbo slot — so it is no longer a standalone top card
/// (Q25/Q26 precondition) yet remains resolvable + EXTRACTABLE: the parked DNA-evo
/// re-materializes it from limbo into the Omnimon merge
/// (`rematerialize_digixros_limbo` inside `effect_initiated_dna_digivolve_with_hand_partner`).
/// The host stays in hand until every leave window resolves; a continuation
/// (`arm_digixros_resume_after_parked_leave`) re-enters the loop once the reward
/// settles and reaches `finalize_digixros_play_after_leave_windows`, which prunes
/// the extracted material, finds the DigiXros recipe no longer satisfied (a slot
/// dropped below its `min`), and returns Dorbickmon to hand for 0 memory.
///
/// Two supporting fixes made this faithful: (1) `run_candidate_inner` re-resolves
/// a parked replacement's `source_permanent`/subject by identity, since moving
/// the material out of `battle_area` shifts later indices (e.g. BT17-095's
/// Delay-Option carrier); (2) the in-flight DigiXros host + its selected hand
/// materials are excluded from intervening hand-selection candidates
/// (`card_reserved_by_pending_digixros`) so the DNA-evo partner pick can't grab
/// the host card itself.
#[test]
fn q26_dorbickmon_returns_to_hand_when_cost_unpayable_after_dna_evo() {
    let (mut r, _memory_before) = stage_q26_board();

    // Finish material selection — the DigiXros play would commit here. In a
    // faithful engine BT17-095's [All Turns] observer fires on WarGreymon's
    // departure mid-DigiXros, its <Delay> DNA-evo pulls WarGreymon (+ the hand
    // partner) into Omnimon, removing WarGreymon as a Dorbickmon material ⇒
    // Dorbickmon can no longer pay its DigiXros cost ⇒ Dorbickmon returns to
    // hand (judge Q26).
    if r.pending_selection()
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
/// ── RESOLVED (2026-06-03, G-DIGIXROS-REDIRECT-EXTRACTION) ─────────────────────
/// Same substrate as Q26. Because the host commits only at
/// `finalize_digixros_play_after_leave_windows` (after the leave windows resolve)
/// and the recipe is found unsatisfied once WarGreymon is DNA-extracted, the
/// declared play is abandoned with the card still in hand and NO memory paid —
/// memory is unchanged from before the (failed) play.
#[test]
fn q27_dorbickmon_pays_zero_memory_when_returned_to_hand() {
    let (mut r, memory_before) = stage_q26_board();

    if r.pending_selection()
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

/// Q30 (also cluster E) — interruptive `<Partition>` + MedievalGallantmon's
/// would-play cost reduction.
///
/// Board (quiz PDF p67): Player A has an UNSUSPENDED Chaosmon: Valdur Arm
/// (BT20-037) with BanchoLeomon (BT20-036) and MedievalGallantmon (EX8-074)
/// in its digivolution cards, plus 1 random SUSPENDED Digimon. Player B has
/// two Dinobeemon (BT16-077); one carries Flamedramon (EX3-008). B ends
/// their turn → Flamedramon's inherited [End of Your Turn] DNA digivolves
/// both Dinobeemon into EX3-063 Imperialdramon: Dragon Mode. Its [When
/// Digivolving] (DNA): A chooses 1 of their Digimon (the random one) — the
/// rest (Chaosmon) get deleted, triggering <Partition>, which A activates.
///
/// Judge feedback (PDF p69): <Partition> is INTERRUPTIVE — it activates
/// BEFORE Chaosmon is deleted (and before <Blitz> is declared). Both
/// BanchoLeomon and MedievalGallantmon are played out simultaneously;
/// Medieval's interruptive "When this card would be played, by suspending 2
/// Digimon, reduce the play cost by 4" is usable even on a free play.
/// BanchoLeomon isn't in the battle area yet, and Chaosmon IS still in the
/// battle area — so the only legal suspend targets are **Imperialdramon:
/// Dragon Mode & Chaosmon: Valdur Arm** (the suspended random Digimon is
/// not a legal target). Answer: YES — suspend those two.
/// Shared Q30 driver: set up the quiz board and drive to Medieval-
/// Gallantmon's first suspend pick (his would-play cost reduction accepted).
/// At the returned state the `<Partition>` SECOND-source play (BanchoLeomon)
/// is armed as a data [`AfterSelectionHook::PartitionSecondPlay`] waiting for
/// Medieval's interrupt chain to drain. Returns
/// `(runner, chaosmon, random, imperial)`.
fn q30_drive_to_medieval_suspend_prompt() -> (
    DebugRunner,
    digimon_engine::permanent::PermanentHandle,
    digimon_engine::permanent::PermanentHandle,
    digimon_engine::permanent::PermanentHandle,
) {
    use digimon_engine::action::space::encode_attack;

    let mut r = DebugRunner::builder()
        .dsl_card("BT20-037")
        .expect("BT20-037 Chaosmon: Valdur Arm loads")
        .dsl_card("BT20-036")
        .expect("BT20-036 BanchoLeomon loads")
        .dsl_card("EX8-074")
        .expect("EX8-074 MedievalGallantmon loads")
        .dsl_card("EX3-063")
        .expect("EX3-063 Imperialdramon: Dragon Mode loads")
        .dsl_card("BT16-077")
        .expect("BT16-077 Dinobeemon loads")
        .dsl_card("EX3-008")
        .expect("EX3-008 Flamedramon loads")
        .add_card({
            let mut c = make_test_card("Q30-RND", "Random Digimon");
            c.card_kind = CardKind::Digimon;
            c.level = Some(4);
            c.dp = Some(4000);
            c
        })
        .add_card({
            let mut c = make_test_card("Q30-FILL", "Filler");
            c.card_kind = CardKind::Digimon;
            c
        })
        .deck(0, &["Q30-FILL"; 5])
        .deck(1, &["Q30-FILL"; 5])
        .hand(1, &["EX3-063"])
        .memory(10)
        .start();
    r.skip_mulligan();
    // Player B's turn (their end-of-turn fires Flamedramon's inherited DNA).
    r.set_first_player(1);

    // Player A: Chaosmon stack (bottom→top: BanchoLeomon / Medieval / top),
    // unsuspended, + 1 random suspended Digimon.
    let chaosmon = r.place_stack(0, &["BT20-036", "EX8-074", "BT20-037"]);
    let random = r.place_on_field(0, "Q30-RND", Some(0));
    r.game.players[0].battle_area[random.index as usize].is_suspended = true;

    // Player B: two Dinobeemon; the first carries Flamedramon.
    let dino_a = r.place_on_field(1, "BT16-077", Some(0));
    r.push_source(dino_a, "EX3-008");
    let _dino_b = r.place_on_field(1, "BT16-077", Some(0));

    // ── B ends their turn → Flamedramon inherited [EoT] DNA digivolve ──────
    r.end_turn();

    // Drive: Flamedramon's optional [EoT] accept → partner pick (the other
    // Dinobeemon) → hand pick (EX3-063). Generic non-PASS walk until the
    // keep-pick surfaces for Player A.
    let pass = digimon_engine::action::space::PASS;
    for _ in 0..6 {
        let Some(view) = r.pending_selection_view() else {
            break;
        };
        if view.selecting_player == 0 {
            break; // Imperialdramon's [WD] keep-pick (A chooses)
        }
        let action = view
            .valid_action_ids
            .iter()
            .copied()
            .find(|&a| a != pass)
            .unwrap_or(pass);
        r.execute_action(view.selecting_player, action)
            .expect("drive Flamedramon EoT DNA digivolve");
    }

    // Imperialdramon: Dragon Mode must be on B's field (DNA digivolved).
    let imperial = r.game.players[1]
        .battle_area
        .iter()
        .position(|p| p.top_card().card_id(&r.game.card_data) == "EX3-063")
        .map(|i| digimon_engine::permanent::PermanentHandle {
            player: 1,
            index: i as u8,
        })
        .expect("Imperialdramon: Dragon Mode DNA digivolved onto B's field");
    assert!(
        !r.game.players[1].battle_area[imperial.index as usize].is_suspended,
        "DNA digivolution enters unsuspended"
    );

    // ── A's keep-pick: choose the random Digimon (Chaosmon gets deleted) ───
    let view = r
        .pending_selection_view()
        .expect("Imperialdramon [WD]: A's keep-pick must surface");
    assert_eq!(
        view.selecting_player, 0,
        "the OPPONENT (A) makes the keep-pick"
    );
    let keep_random = encode_attack(random.player as u16, random.index as u16);
    assert!(
        view.valid_action_ids.contains(&keep_random),
        "the random Digimon must be a keep candidate"
    );
    r.execute_action(0, keep_random)
        .expect("A keeps the random Digimon");

    // ── <Partition> interrupts Chaosmon's deletion (carrier still on field) ─
    let view = r
        .pending_selection_view()
        .expect("Partition accept dialog must surface");
    assert_eq!(view.selecting_player, 0);
    assert!(
        view.is_optional,
        "Partition activation is the player's choice"
    );
    assert!(
        r.game.players[0]
            .battle_area
            .iter()
            .any(|p| p.top_card().card_id(&r.game.card_data) == "BT20-037"),
        "judge Q30: <Partition> is interruptive — Chaosmon is STILL in the \
         battle area when it activates"
    );
    let accept = view.valid_action_ids[0];
    r.execute_action(0, accept).expect("A activates Partition");

    // 2-pick from the live stack: pick MedievalGallantmon (source index 1,
    // the higher action id) FIRST so its would-play interrupt is the next
    // flow, then BanchoLeomon.
    for want_max in [true, false] {
        let view = r
            .pending_selection_view()
            .expect("Partition source pick must surface");
        let mut acts: Vec<u16> = view.valid_action_ids.clone();
        acts.sort_unstable();
        let pick = if want_max {
            *acts.last().unwrap()
        } else {
            acts[0]
        };
        r.execute_action(0, pick).expect("pick a Partition source");
    }

    // ── MedievalGallantmon's interruptive suspend-2 cost reduction ─────────
    // Accept dialog for the would-play cost reduction.
    let view = r
        .pending_selection_view()
        .expect("Medieval's would-play cost-reduction dialog must surface");
    assert_eq!(view.selecting_player, 0);
    r.execute_action(0, view.valid_action_ids[0])
        .expect("A uses Medieval's cost reduction");

    (r, chaosmon, random, imperial)
}

#[test]
fn q30_partition_interruptive_suspends_both_with_cost_reduction() {
    use digimon_engine::action::space::encode_attack;

    let (mut r, chaosmon, _random, imperial) = q30_drive_to_medieval_suspend_prompt();
    let pass = digimon_engine::action::space::PASS;

    // THE JUDGE PIN: the legal suspend set is EXACTLY {Imperialdramon:
    // Dragon Mode, Chaosmon: Valdur Arm} — BanchoLeomon is not in the
    // battle area yet, and the random Digimon is already suspended.
    let want_imperial = encode_attack(imperial.player as u16, imperial.index as u16);
    let want_chaosmon = encode_attack(chaosmon.player as u16, chaosmon.index as u16);
    for pick_no in 1..=2 {
        let view = r
            .pending_selection_view()
            .expect("suspend pick must surface");
        let mut got: Vec<u16> = view
            .valid_action_ids
            .iter()
            .copied()
            .filter(|&a| a != pass)
            .collect();
        got.sort_unstable();
        let mut want: Vec<u16> = if pick_no == 1 {
            vec![want_imperial, want_chaosmon]
        } else {
            // After the first suspend, only the other remains.
            got.clone()
        };
        want.sort_unstable();
        if pick_no == 1 {
            assert_eq!(
                got, want,
                "judge Q30: the legal suspend targets are EXACTLY \
                 Imperialdramon: Dragon Mode and Chaosmon: Valdur Arm \
                 (BanchoLeomon not yet in play; the random Digimon already \
                 suspended)"
            );
        } else {
            assert_eq!(got.len(), 1, "exactly one suspend target remains");
        }
        r.execute_action(0, got[0]).expect("suspend a target");
    }
    // Drain anything residual (BanchoLeomon's play, deletion finalize,
    // Blitz), DECLINING trailing optional prompts — accepting them would
    // cascade into Medieval's [All Turns] re-activation (delete
    // Imperialdramon → its inherited <Partition> → a nested replacement
    // park, the engine's single-park boundary; see
    // G-NESTED-PARKED-REPLACEMENT). The judge line ends here.
    for _ in 0..10 {
        let Some(view) = r.pending_selection_view() else {
            break;
        };
        let action = if view.is_optional {
            pass
        } else {
            view.valid_action_ids[0]
        };
        let player = view.selecting_player;
        if r.execute_action(player, action).is_err() {
            break;
        }
    }

    // ── End state ───────────────────────────────────────────────────────────
    // Imperialdramon: Dragon Mode is suspended (the cost reduction's pick).
    assert!(
        r.game.players[1].battle_area[imperial.index as usize].is_suspended,
        "judge Q30: Imperialdramon: Dragon Mode ends suspended"
    );
    // Chaosmon was suspended DURING the interrupt, then its deletion
    // completed — it is in A's trash.
    assert!(
        r.game.players[0]
            .trash
            .iter()
            .any(|c| c.card_id(&r.game.card_data) == "BT20-037"),
        "Chaosmon: Valdur Arm's deletion proceeded after the interrupt"
    );
    // Both partition plays are on A's field; the kept random Digimon too.
    let a_ids: Vec<&str> = r.game.players[0]
        .battle_area
        .iter()
        .map(|p| p.top_card().card_id(&r.game.card_data))
        .collect();
    assert!(
        a_ids.contains(&"BT20-036") && a_ids.contains(&"EX8-074"),
        "both partition sources were played; got {a_ids:?}"
    );
    assert!(a_ids.contains(&"Q30-RND"), "the kept Digimon survived");
}

/// make-engine-cloneable — the `<Partition>` SECOND-source play is armed as a
/// data [`AfterSelectionHook::PartitionSecondPlay`] while the FIRST play's
/// would-play interrupt chain (Medieval's suspend-2 cost reduction) resolves.
/// A `Game` cloned mid-chain must carry the armed hook: driving the CLONE to
/// completion plays BanchoLeomon there too, and resolving the clone leaves
/// the original untouched (which then replays identically).
#[test]
fn q30_partition_second_play_hook_clones_faithfully_mid_chain() {
    let (r, _chaosmon, _random, _imperial) = q30_drive_to_medieval_suspend_prompt();
    let pass = digimon_engine::action::space::PASS;

    // At the suspend prompt the second-source play must be armed AS DATA.
    assert!(
        r.game.pending_selection.is_some(),
        "Medieval's suspend pick is parked"
    );
    assert!(
        r.game.pending_selection_resume.is_some(),
        "the suspend pick must be resume-driven (clone-safe)"
    );
    assert!(
        r.game
            .after_selection_resume_hooks
            .0
            .iter()
            .any(|h| matches!(
                h,
                digimon_engine::resume::AfterSelectionHook::PartitionSecondPlay { .. }
            )),
        "the <Partition> second-source play is armed as a data hook mid-chain"
    );

    // Fork. The clone must inherit the armed hook.
    let mut clone = r.game.clone();
    assert_eq!(
        clone.after_selection_resume_hooks.0.len(),
        r.game.after_selection_resume_hooks.0.len(),
        "clone inherits the armed after-selection hooks"
    );

    // Drive the CLONE to completion: both suspend picks, then decline
    // trailing optional prompts (mirrors the Q30 driver's drain).
    for _ in 0..12 {
        let Some(sel) = clone.pending_selection.as_ref() else {
            break;
        };
        let player = sel.selecting_player;
        let action = if sel.is_optional {
            pass
        } else {
            *sel.valid_action_ids
                .iter()
                .find(|&&a| a != pass)
                .unwrap_or(&pass)
        };
        if clone.resolve_selection(player, action).is_err() {
            break;
        }
    }
    let clone_ids: Vec<&str> = clone.players[0]
        .battle_area
        .iter()
        .map(|p| p.top_card().card_id(&clone.card_data))
        .collect();
    assert!(
        clone_ids.contains(&"BT20-036") && clone_ids.contains(&"EX8-074"),
        "clone: BOTH partition sources played after the chain drained \
         (the second play survived the fork as data); got {clone_ids:?}"
    );

    // INDEPENDENCE: the original still sits at the suspend prompt with the
    // hook armed — resolving the clone did not touch it.
    assert!(
        r.game.pending_selection.is_some(),
        "original's suspend pick survives resolving the clone"
    );
    assert!(
        r.game
            .after_selection_resume_hooks
            .0
            .iter()
            .any(|h| matches!(
                h,
                digimon_engine::resume::AfterSelectionHook::PartitionSecondPlay { .. }
            )),
        "original's armed hook survives resolving the clone"
    );

    // REPLAY: driving the ORIGINAL with the same inputs reaches the same
    // battle-area outcome.
    let mut r = r;
    for _ in 0..12 {
        let Some(view) = r.pending_selection_view() else {
            break;
        };
        let player = view.selecting_player;
        let action = if view.is_optional {
            pass
        } else {
            *view
                .valid_action_ids
                .iter()
                .find(|&&a| a != pass)
                .unwrap_or(&pass)
        };
        if r.execute_action(player, action).is_err() {
            break;
        }
    }
    let orig_ids: Vec<&str> = r.game.players[0]
        .battle_area
        .iter()
        .map(|p| p.top_card().card_id(&r.game.card_data))
        .collect();
    assert_eq!(
        orig_ids, clone_ids,
        "original replays identically to the clone"
    );
}
