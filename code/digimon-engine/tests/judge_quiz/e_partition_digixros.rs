//! Cluster E — `<Partition>` / DigiXros departure semantics / sequential
//! de-digivolve with mid-sequence immunity.
//!
//! Questions (see `card-resolution.md`):
//!   Q15 LordKnightmon (X Antibody) (BT19-073) does `<De-Digivolve 1>` repeatedly
//!       on a stack (Omnimon X BT20-102 / Gallantmon X EX8-073 / Gallantmon
//!       BT17-016 / WarGrowlmon BT12-016 / Growlmon EX3-057 / Guilmon EX4-006);
//!       after the first, Gallantmon (X Antibody)'s immunity halts the rest —
//!       judge: Gallantmon (X Antibody) topmost.
//!   Q16 Lilithmon (EX6-057)-granted "[EoT] Delete this" on Paildramon
//!       (BT16-025) counts as leaving by its OWN effect — judge: `<Partition>`
//!       does NOT trigger.
//!   Q25 Miraculous Mega Knight (BT17-095) `[All Turns]` fires on DigiXros
//!       departure of WarGreymon (AD1-004) — judge: YES (DigiXros ≠ battle).
//!   Q29 Yuu Amano (BT10-093) + DigiXros (DarknessBagramon EX10-059 etc.):
//!       3 legal stack orderings — judge: placement order rules.
//!   Q30 (shared with cluster C) interruptive `<Partition>`.
//!
//! Scenarios authored under tasks §7.

#![allow(unused_imports)]

use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::TriggerSource;

/// Seat the BT17-095 Option permanent at `handle` as a Delay-Option so its
/// Clause B `when_would_leave_battle_area` observer can fire — the observer is
/// gated on `source_is_delayed_option`. `place_on_field` yields a Standard
/// option, so the Delay state is set here directly (mirrors
/// `tests/cards_behavioral/bt17/bt17_095.rs::seat_as_delay_option`).
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

/// Q16 — RESOLVED 2026-05-29 (change `add-grant-triggered-effect-dsl`): a
/// Lilithmon (EX6-057)-granted "[End of Your Turn] Delete this Digimon" is the
/// GRANTEE's own effect (DCGO sources the granted ActivateClass from the
/// carrier's top card; the engine runs the granted body with
/// effect_source_player = carrier.player). So when the granted self-delete
/// removes Paildramon (BT16-025, `<Partition (Blue Lv.4 & Green Lv.4)>`), the
/// deletion's cause is OwnEffect → `<Partition>`'s cause-filter skips it →
/// Partition does NOT fire (no mandatory 2-source replay surfaces).
#[test]
fn q16_partition_not_triggered_when_leaving_by_own_granted_effect() {
    let mut r = DebugRunner::builder()
        .dsl_card("EX6-057")
        .expect("EX6-057 Lilithmon loads")
        .dsl_card("BT16-025")
        .expect("BT16-025 Paildramon (<Partition>) loads")
        .dsl_card("BT12-022")
        .expect("BT12-022 ExVeemon (Blue Lv4) loads")
        .dsl_card("BT12-050")
        .expect("BT12-050 Stingmon (Green Lv4) loads")
        .add_card({
            let mut c = make_test_card("SEC", "Sec");
            c.card_kind = CardKind::Digimon;
            c
        })
        .security(1, &["SEC", "SEC"])
        .memory(10)
        .start();
    r.skip_mulligan();

    // Player 1's Paildramon WITH its Partition sources (Blue Lv4 ExVeemon +
    // Green Lv4 Stingmon) so <Partition> is applicable when it leaves.
    let paildramon = r.place_stack(1, &["BT12-022", "BT12-050", "BT16-025"]);

    // Player 0 plays Lilithmon → [On Play] grants 1 opponent Digimon
    // "[End of Your Turn] Delete this Digimon."
    let lilithmon = r.place_on_field(0, "EX6-057", None);
    r.fire_on_play(0, lilithmon.index as usize);

    // Resolve the [On Play] target select — Paildramon is the only opponent
    // Digimon, so the first valid action targets it.
    if let Some(sel) = r.game.pending_selection.as_ref() {
        let who = sel.selecting_player;
        let aid = sel.valid_action_ids[0];
        let _ = r.game.resolve_selection(who, aid);
    }

    // The grant must be installed on Paildramon.
    assert!(
        !r.game
            .modifiers
            .granted_triggered_for_timing(paildramon, EffectTiming::EndOfYourTurn)
            .is_empty(),
        "Lilithmon's [On Play] must grant Paildramon the [End of Your Turn] delete"
    );

    // It's the grantee's turn (player 1) when the granted [EoT] fires.
    r.set_first_player(1);
    let p1_sec_before = r.security_count(1);

    // Fire the granted "[End of Your Turn] Delete this Digimon".
    r.game
        .enqueue_triggered(EffectTiming::EndOfYourTurn, TriggerSource::Permanent(paildramon));
    r.game.drain_effect_queue();

    // Paildramon was deleted by its OWN (granted) effect.
    let paildramon_alive = r
        .game
        .players[1]
        .battle_area
        .iter()
        .any(|p| p.top_card().card_id(&r.game.card_data) == "BT16-025");
    assert!(!paildramon_alive, "the granted [EoT] delete must remove Paildramon");

    // <Partition> must NOT fire: no mandatory 2-source replay selection, and
    // the partition sources land in trash rather than being replayed onto the
    // field. (Lilithmon's own [Opp Turn] clause 3 is mandatory/auto and does
    // not surface a selection.)
    assert!(
        r.game.pending_selection.is_none(),
        "Partition must NOT fire on a granted (own-effect) self-delete — no \
         2-source replay selection should surface (judge-quiz Q16)"
    );
    let sources_replayed = r.game.players[1].battle_area.iter().any(|p| {
        let id = p.top_card().card_id(&r.game.card_data);
        id == "BT12-022" || id == "BT12-050"
    });
    assert!(
        !sources_replayed,
        "Partition did not fire, so ExVeemon/Stingmon must NOT be replayed onto the field"
    );
    let _ = p1_sec_before;
}

/// Q15 — LordKnightmon (X Antibody) (BT19-073) does `<De-Digivolve 1>` repeatedly;
/// after the first, Gallantmon (X Antibody) (EX8-073)'s [All Turns] immunity halts
/// the rest. Judge: Gallantmon (X Antibody) is the topmost card.
///
/// Quiz board (judge_quiz p.30, card-resolution.md row 15): the memory gauge
/// is at 1 on Player A's side. Player A has LordKnightmon (BT19-072) and 4
/// other random Digimon. Player B's stack (top→bottom): Omnimon (X Antibody)
/// BT20-102 / Gallantmon (X Antibody) EX8-073 / Gallantmon BT17-016 /
/// WarGrowlmon BT12-016 / Growlmon EX3-057 / Guilmon EX4-006. Player A
/// digivolves LordKnightmon into BT19-073 for 1 memory (gauge → 0), then
/// resolves the [When Digivolving], targeting the Omnimon (X Antibody) stack.
///
/// Judge feedback: LordKnightmon (X Antibody) performs `<De-Digivolve 1>`
/// MULTIPLE TIMES (not one `<De-Digivolve X>`), each done 1 card at a time.
/// After the first, Gallantmon (X Antibody) becomes the top stacked card; its
/// [All Turns] immunity (opponent-Digimon effects while the gauge is at 0 or
/// higher on Player A's side, i.e. Player B has 0-or-less memory) applies
/// IMMEDIATELY, so the remaining `<De-Digivolve 1>` do not affect it.
/// Answer: **Gallantmon (X Antibody) is the topmost card.**
#[test]
fn q15_sequential_de_digivolve_halted_by_x_antibody_immunity() {
    let mut builder = DebugRunner::builder()
        .dsl_card("BT19-073")
        .expect("BT19-073 LordKnightmon (X Antibody) loads")
        .dsl_card("BT19-072")
        .expect("BT19-072 LordKnightmon loads")
        .dsl_card("BT20-102")
        .expect("BT20-102 Omnimon (X Antibody) loads")
        .dsl_card("EX8-073")
        .expect("EX8-073 Gallantmon (X Antibody) loads")
        .dsl_card("BT17-016")
        .expect("BT17-016 Gallantmon loads")
        .dsl_card("BT12-016")
        .expect("BT12-016 WarGrowlmon loads")
        .dsl_card("EX3-057")
        .expect("EX3-057 Growlmon loads")
        .dsl_card("EX4-006")
        .expect("EX4-006 Guilmon loads");
    // "4 other random Digimon" on Player A's side — the [When Digivolving]
    // counts 5 of A's Digimon ⇒ 5 separate <De-Digivolve 1> instances.
    for i in 0..4 {
        builder = builder.add_card({
            let mut c = make_test_card(&format!("Q15-RND{i}"), &format!("Random{i}"));
            c.card_kind = CardKind::Digimon;
            c.level = Some(4);
            c.dp = Some(4000);
            c
        });
    }
    let mut r = builder
        .add_card({
            let mut c = make_test_card("Q15-FILL", "Filler");
            c.card_kind = CardKind::Digimon;
            c
        })
        .deck(0, &["Q15-FILL"; 5])
        .deck(1, &["Q15-FILL"; 5])
        .memory(0)
        .start();
    r.skip_mulligan();
    r.set_first_player(0);
    // Player A paid 1 from a gauge of 1 ⇒ the gauge sits at 0 on A's side
    // while the [When Digivolving] resolves (Player B has 0 memory ⇒
    // EX8-073's "while you have 0 or less memory" gate holds for B).
    r.game.set_memory(0);

    // Player A: LordKnightmon (X Antibody) over LordKnightmon + 4 others.
    let lkx = r.place_stack(0, &["BT19-072", "BT19-073"]);
    for i in 0..4 {
        let _ = r.place_on_field(0, &format!("Q15-RND{i}"), Some(0));
    }
    // Player B: the 6-deep stack, bottom→top Guilmon … Omnimon (X Antibody).
    let omnimon_stack = r.place_stack(
        1,
        &[
            "EX4-006", "EX3-057", "BT12-016", "BT17-016", "EX8-073", "BT20-102",
        ],
    );
    assert_eq!(
        r.game.player(1).battle_area[omnimon_stack.index as usize].stack_size(),
        6,
        "precondition: Player B's stack is 6 deep"
    );
    let b_trash_before = r.trash_size(1);

    // Resolve LordKnightmon (X Antibody)'s [When Digivolving].
    r.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(lkx),
    );
    r.game.drain_effect_queue();

    // Selection 1 — the <De-Digivolve 1> target: Player A targets the
    // Omnimon (X Antibody) stack (the only opponent Digimon).
    let view = r
        .pending_selection_view()
        .expect("the De-Digivolve target selection must install");
    assert_eq!(view.selecting_player, 0);
    assert!(!view.is_optional, "the De-Digivolve pick is mandatory");
    r.execute_action(0, view.valid_action_ids[0])
        .expect("target the Omnimon (X Antibody) stack");

    // Selection 2 — "1 of their Digimon can't digivolve": mandatory pick of
    // the (sole) opponent Digimon. The APPLICATION is consult-gated by the
    // immunity (DCGO checks CanNotBeAffected at apply time), but the pick
    // itself surfaces — no auto-selection.
    if let Some(view) = r.pending_selection_view() {
        r.execute_action(0, view.valid_action_ids[0])
            .expect("pick the can't-digivolve Digimon");
    }
    let _ = r.auto_resolve();

    // ── THE JUDGE ANSWER ────────────────────────────────────────────────────
    // Only the FIRST <De-Digivolve 1> resolves (trashing Omnimon X Antibody);
    // the exposed Gallantmon (X Antibody)'s [All Turns] immunity halts the
    // remaining four. Gallantmon (X Antibody) is the topmost card.
    let perm = &r.game.player(1).battle_area[omnimon_stack.index as usize];
    assert_eq!(
        perm.top_card().card_id(&r.game.card_data),
        "EX8-073",
        "judge Q15: Gallantmon (X Antibody) must be the topmost card of \
         Player B's stack after the multi-step De-Digivolve"
    );
    assert_eq!(
        perm.stack_size(),
        5,
        "exactly ONE card (Omnimon (X Antibody)) was trashed"
    );
    assert_eq!(
        r.trash_size(1),
        b_trash_before + 1,
        "only Omnimon (X Antibody) landed in Player B's trash"
    );
    assert!(
        r.game.player(1).trash.iter().any(|c| c.card_id(&r.game.card_data) == "BT20-102"),
        "the trashed card is Omnimon (X Antibody) BT20-102"
    );
    // The rest of the stack is intact, in order (bottom→top).
    let ids: Vec<&str> = r.game.player(1).battle_area[omnimon_stack.index as usize]
        .card_sources
        .iter()
        .map(|c| c.card_id(&r.game.card_data))
        .collect();
    assert_eq!(
        ids,
        vec!["EX4-006", "EX3-057", "BT12-016", "BT17-016", "EX8-073"],
        "Guilmon / Growlmon / WarGrowlmon / Gallantmon / Gallantmon (X Antibody) all remain"
    );
}

/// Q25 — Miraculous Mega Knight (BT17-095) [All Turns] fires on DigiXros departure
/// of WarGreymon (AD1-004) (departure ≠ battle). Judge: YES, triggers.
///
/// ── BLOCKED-ENGINE (G-DIGIXROS-DEPARTURE-LEAVE-TRIGGER) ───────────────────
/// The DigiXros material-consumption path consumes a battle-area material via a
/// RAW `battle_area.remove(idx)` in `Game::take_digixros_material_origin`
/// (`src/game_actions.rs` BattleArea branch) — it NEVER fires the
/// `WhenWouldLeaveBattleArea` replacement window. Every faithful effect-driven
/// battle-area departure (e.g. `place_permanent_on_security_observed`,
/// `delete_permanents_batch`) routes through `try_replace(WhenWouldLeaveBattleArea,
/// …)` with a cause from `infer_effect_cause`; the DigiXros path does not. So
/// BT17-095's `[All Turns]` "would leave the battle area outside of a battle"
/// observer cannot see a DigiXros material departure, and its `<Delay>` is never
/// installed.
///
/// Captured real failure (ran WITHOUT #[ignore], 2026-06-03): after selecting
/// WarGreymon (AD1-004) as Dorbickmon's DigiXros material and finishing the
/// material selection, BT17-095's `[All Turns]` Delay was NOT installed — the
/// `OptionState::Delayed` carrier was never asked to activate, no DNA-evo /
/// Delay-activation `pending_selection` surfaced, and WarGreymon simply moved
/// under Dorbickmon as a material. The load-bearing assertion
/// (`bt17_095_delay_activation_offered`) failed: "BT17-095 [All Turns] must
/// observe WarGreymon's DigiXros departure (judge Q25: YES) — no Delay/DNA-evo
/// flow surfaced".
///
/// Logged to qa/archetype-qa/engine-gaps.md as
/// G-DIGIXROS-DEPARTURE-LEAVE-TRIGGER.
#[test]
fn q25_all_turns_fires_on_digixros_departure_not_battle() {
    // Board (card-resolution.md Q25): Player A controls WarGreymon (AD1-004,
    // Lv6, [Greymon] in name) and Miraculous Mega Knight (BT17-095) seated as a
    // Delay-Option. Player A plays Dorbickmon (EX3-014) via [DigiXros],
    // selecting WarGreymon as a material. WarGreymon LEAVES the battle area as a
    // DigiXros material (departure ≠ battle), so BT17-095's [All Turns] observer
    // must fire and install its <Delay> DNA-evo.
    // Four extra distinct-named [Dragon]-family Digimon in hand so Dorbickmon's
    // [DigiXros -2] 5-material recipe (5 distinct names) can complete with
    // WarGreymon as the 5th material. Each carries the [Dragon] trait.
    let mut builder = DebugRunner::builder()
        .dsl_card("EX3-014")
        .expect("EX3-014 Dorbickmon (DigiXros host) loads")
        .dsl_card("AD1-004")
        .expect("AD1-004 WarGreymon loads")
        .from_dsl_yaml(include_str!("../../cards/bt17/BT17-095.yaml"))
        .expect("BT17-095 Miraculous Mega Knight loads");
    for i in 0..4 {
        builder = builder.add_card({
            let mut c = make_test_card(&format!("Q25-DRG{i}"), &format!("Dragon{i}"));
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
            let mut c = make_test_card("Q25-FILL", "Filler");
            c.card_kind = CardKind::Digimon;
            c
        })
        .hand(0, &["EX3-014", "Q25-DRG0", "Q25-DRG1", "Q25-DRG2", "Q25-DRG3"])
        .deck(0, &["Q25-FILL"; 5])
        .deck(1, &["Q25-FILL"; 5])
        .memory(13)
        .start();
    r.skip_mulligan();

    // WarGreymon (AD1-004, Lv6 [Greymon]) on Player A's field — the 5th DigiXros
    // material AND the subject BT17-095 watches.
    let wargreymon = r.place_on_field(0, "AD1-004", Some(0));
    let wargreymon_card = r.top_card(wargreymon);

    // BT17-095 seated as a Delay-Option so its [All Turns] leave observer is live.
    let mmk = r.place_on_field(0, "BT17-095", Some(0));
    seat_as_delay_option(&mut r, mmk);

    // Load-bearing precondition: BT17-095 really is a live Delay-Option carrier.
    assert!(
        matches!(
            r.game.players[0].battle_area[mmk.index as usize].option_state,
            digimon_engine::permanent::OptionState::Delayed { .. }
        ),
        "precondition: BT17-095 must be seated as a Delay-Option carrier"
    );

    // Drive the real DigiXros play of Dorbickmon. Dorbickmon is hand index 0.
    // Select WarGreymon (battle-area) + the 4 hand Dragons as materials (NO
    // auto-select — each material is a surfaced action), then PASS to finish.
    let _ = r.play(0, 0);
    let material_prompt = r
        .pending_selection()
        .expect("DigiXros material prompt must install");
    assert_eq!(
        material_prompt.kind,
        digimon_engine::selection::SelectionKind::Material,
        "Dorbickmon's play must surface a DigiXros material selection"
    );
    let wargreymon_action = wargreymon.index as u16;
    assert!(
        material_prompt
            .valid_action_ids
            .contains(&wargreymon_action),
        "WarGreymon must be a legal DigiXros material for Dorbickmon"
    );
    // Select WarGreymon first (the leave-observed material).
    r.execute_action(0, wargreymon_action)
        .expect("select WarGreymon as DigiXros material");
    // Then select the four hand Dragons by walking each surfaced material prompt
    // (pick the first non-PASS action each time) until the recipe is full.
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
    // Finish material selection (PASS) so the DigiXros play commits and the
    // battle-area materials (WarGreymon) are pulled under Dorbickmon. The
    // Material selection is optional, so PASS is accepted even once the recipe
    // is full (empty candidate list).
    if r
        .pending_selection()
        .is_some_and(|sel| sel.kind == digimon_engine::selection::SelectionKind::Material)
    {
        let _ = r.execute_action(0, digimon_engine::action::space::PASS);
    }

    // After selecting WarGreymon, BT17-095's [All Turns] observer must fire
    // BECAUSE WarGreymon is leaving the battle area as a DigiXros material
    // (outside of a battle). The judge answer is YES — the observer triggers.
    // The load-bearing pin: a <Delay>/DNA-evo activation flow surfaces, OR the
    // BT17-095 Delay carrier was consumed (its self-trash paid). If neither, the
    // observer never saw the departure — the engine gap.
    let delay_or_dna_flow_offered = r.pending_selection().is_some_and(|sel| {
        // The observer's <Delay> activation surfaces an accept/DNA-evo prompt
        // that is NOT the DigiXros material prompt.
        sel.kind != digimon_engine::selection::SelectionKind::Material
    });
    let mmk_self_trashed = !r.game.players[0]
        .battle_area
        .iter()
        .any(|p| p.top_card().card_id(&r.game.card_data) == "BT17-095");

    // Confirm the scenario was genuinely staged: WarGreymon actually left the
    // battle area (it is no longer a standalone permanent).
    let wargreymon_still_standalone = r.game.players[0]
        .battle_area
        .iter()
        .any(|p| p.top_card().handle() == wargreymon_card);
    assert!(
        !wargreymon_still_standalone,
        "precondition: WarGreymon must have left the battle area as a DigiXros material"
    );

    assert!(
        delay_or_dna_flow_offered || mmk_self_trashed,
        "bt17_095_delay_activation_offered — BT17-095 [All Turns] must observe \
         WarGreymon's DigiXros departure (judge Q25: YES) — no Delay/DNA-evo \
         flow surfaced and the Delay carrier was not consumed"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Q29 — Yuu Amano + DarknessBagramon DigiXros: legal stack orderings
// ═══════════════════════════════════════════════════════════════════════════════
//
// Quiz board (judge_quiz, card-resolution.md row 29): Player A controls Yuu
// Amano (BT10-093) with ChuuChuumon (EX10-039) and Damemon (EX10-044) under
// it, and has DarknessBagramon (EX10-059), Bagramon (EX10-056) and
// DarkKnightmon (EX10-031) in hand. A plays DarknessBagramon via its
// [DigiXros -3: [Bagramon]×[DarkKnightmon]], using both hand materials, and
// accepts Yuu Amano's [Your Turn][Once Per Turn] would-play hook, placing
// purple Digimon cards from under the Tamer in the played card's digivolution
// cards (−2 cost each).
//
// Judge feedback: the DigiXros placement resolves after would-play interrupts;
// Yuu Amano's cards go on TOP of the digivolution cards (in either pick
// order); the DigiXros materials go to the BOTTOM in requirement-spec order
// ([Bagramon] above [DarkKnightmon]). The 3 legal stacks (top→bottom):
//   L1: DarknessBagramon / Damemon / ChuuChuumon / Bagramon / DarkKnightmon
//   L2: DarknessBagramon / ChuuChuumon / Damemon / Bagramon / DarkKnightmon
//   L3: DarknessBagramon / ChuuChuumon / Bagramon / DarkKnightmon
//       (only 1 of the up-to-3 purple cards placed)
//
// Cost arithmetic: 16 −3 (Bagramon) −3 (DarkKnightmon) −2×placed.

/// Build the Q29 board: Yuu Amano with ChuuChuumon (pushed first → lower
/// source slot) and Damemon under it; DarknessBagramon + both DigiXros
/// materials in Player A's hand; memory 10 (engine ceiling).
fn q29_runner() -> DebugRunner {
    let mut r = DebugRunner::builder()
        .dsl_card("BT10-093")
        .expect("BT10-093 Yuu Amano loads")
        .dsl_card("EX10-039")
        .expect("EX10-039 ChuuChuumon loads")
        .dsl_card("EX10-044")
        .expect("EX10-044 Damemon loads")
        .dsl_card("EX10-031")
        .expect("EX10-031 DarkKnightmon loads")
        .dsl_card("EX10-056")
        .expect("EX10-056 Bagramon loads")
        .dsl_card("EX10-059")
        .expect("EX10-059 DarknessBagramon loads")
        .memory(10)
        .hand(0, &["EX10-059", "EX10-056", "EX10-031"])
        .start();
    r.skip_mulligan();
    r.set_first_player(0);
    let yuu = r.place_on_field(0, "BT10-093", Some(0));
    r.push_source(yuu, "EX10-039"); // ChuuChuumon — first/lower under-Tamer slot
    r.push_source(yuu, "EX10-044"); // Damemon — second/higher under-Tamer slot
    r
}

/// Drive the DarknessBagramon DigiXros play end-to-end. `under_tamer_picks`
/// drives Yuu Amano's hook: each entry selects an under-Tamer source action
/// (`true` = the highest remaining source action id, `false` = the lowest);
/// an empty slice declines nothing — after the picks, the multi-select is
/// closed with PASS if it is still open. Then both DigiXros material slots
/// are filled from hand and the transaction committed.
fn q29_drive_darkness_bagramon_play(r: &mut DebugRunner, under_tamer_picks: &[bool]) {
    use digimon_engine::action::space::{PASS, SOURCE_SELECT_START};
    use digimon_engine::selection::SelectionKind;

    let _ = r.play(0, 0); // EX10-059 is hand index 0

    // ── Phase 1 — Yuu Amano's would-play hook (before the cost is fixed) ────
    // The optional [Your Turn][Once Per Turn] hook first surfaces an accept
    // prompt (the Taiki BT10-087 idiom); accepting opens its pay_cost
    // select_under_tamer_sources (min 0 / max 3, purple Digimon). All of it
    // must surface BEFORE the recipe Material prompts (judge: would-play
    // interrupts resolve first).
    {
        let view = r
            .pending_selection_view()
            .expect("Yuu Amano's would-play hook must surface before materials");
        assert_ne!(
            view.kind,
            SelectionKind::Material,
            "judge Q29: Yuu Amano's would-play hook resolves BEFORE the DigiXros \
             material selection"
        );
        if !view.valid_action_ids.iter().any(|&a| a >= SOURCE_SELECT_START) {
            assert!(view.is_optional, "Yuu Amano's hook accept prompt is optional");
            let accept = view.valid_action_ids[0];
            r.execute_action(0, accept)
                .expect("accept Yuu Amano's would-play hook");
        }
    }
    for &pick_highest in under_tamer_picks {
        let view = r
            .pending_selection_view()
            .expect("Yuu Amano's under-Tamer source pick must surface before materials");
        assert_ne!(
            view.kind,
            SelectionKind::Material,
            "judge Q29: Yuu Amano's would-play hook resolves BEFORE the DigiXros \
             material selection"
        );
        let mut source_actions: Vec<u16> = view
            .valid_action_ids
            .iter()
            .copied()
            .filter(|&a| a >= SOURCE_SELECT_START)
            .collect();
        source_actions.sort_unstable();
        assert!(
            !source_actions.is_empty(),
            "under-Tamer purple Digimon sources must be selectable; got {:?}",
            view.valid_action_ids
        );
        let action = if pick_highest {
            *source_actions.last().unwrap()
        } else {
            source_actions[0]
        };
        r.execute_action(0, action)
            .expect("pick an under-Tamer purple Digimon card for Yuu Amano's hook");
    }
    // Close the up-to-3 multi-select if it is still open (PASS is legal on a
    // min-0 selection).
    if r
        .pending_selection_view()
        .is_some_and(|v| v.kind != SelectionKind::Material && v.valid_action_ids.contains(&PASS))
    {
        r.execute_action(0, PASS)
            .expect("close Yuu Amano's up-to-3 multi-select");
    }

    // ── Phase 2 — DigiXros recipe materials ([Bagramon] × [DarkKnightmon]) ──
    for slot in 0..2 {
        let Some(view) = r.pending_selection_view() else {
            panic!("DigiXros material slot {slot} must surface a selection");
        };
        assert_eq!(
            view.kind,
            SelectionKind::Material,
            "slot {slot} must be a Material selection; got {:?}",
            view.kind
        );
        let pick = view
            .valid_action_ids
            .iter()
            .copied()
            .find(|&a| a != PASS)
            .unwrap_or_else(|| panic!("material slot {slot} must have a hand candidate"));
        r.execute_action(0, pick)
            .expect("select DigiXros material from hand");
    }
    // Close the material selection if the transaction is still collecting.
    if r
        .pending_selection_view()
        .is_some_and(|v| v.kind == SelectionKind::Material)
    {
        let _ = r.execute_action(0, PASS);
    }
    // EX10-059 has no [On Play] authored (PARTIAL — see the YAML); drain any
    // residual prompts defensively so assertions read settled state.
    let _ = r.auto_resolve();
}

/// Collect Player A's DarknessBagramon stack as card ids, bottom→top
/// (the top card is the LAST element of `card_sources`).
fn q29_db_stack(r: &DebugRunner) -> Vec<String> {
    let perm = r
        .game
        .players[0]
        .battle_area
        .iter()
        .find(|p| p.top_card().card_id(&r.game.card_data) == "EX10-059")
        .expect("DarknessBagramon must be on Player A's field after the DigiXros play");
    perm.card_sources
        .iter()
        .map(|c| c.card_id(&r.game.card_data).to_string())
        .collect()
}

/// Q29 (both purple cards placed, Damemon picked first): legal stack L1 —
/// DarknessBagramon / Damemon / ChuuChuumon / Bagramon / DarkKnightmon
/// (top→bottom), and the play costs 16 −3 −3 −2 −2 = 6.
#[test]
fn q29_legal_digixros_stack_orderings_with_yuu_amano() {
    let mut r = q29_runner();
    let memory_before = r.memory();
    assert_eq!(memory_before, 10, "staged memory must be at the engine ceiling");

    // Pick Damemon (the higher under-Tamer slot) first, then ChuuChuumon.
    q29_drive_darkness_bagramon_play(&mut r, &[true, false]);

    // ── THE JUDGE ANSWER (L1) ───────────────────────────────────────────────
    // Yuu Amano's cards sit on TOP of the digivolution cards in pick order
    // (first-picked directly under the top card); the DigiXros materials sit
    // at the BOTTOM in requirement order (Bagramon above DarkKnightmon).
    assert_eq!(
        q29_db_stack(&r),
        vec!["EX10-031", "EX10-056", "EX10-039", "EX10-044", "EX10-059"],
        "judge Q29 (L1, bottom→top): DarkKnightmon / Bagramon / ChuuChuumon / \
         Damemon / DarknessBagramon"
    );

    // Cost: 16 −3 (Bagramon) −3 (DarkKnightmon) −2 −2 (Yuu Amano) = 6.
    assert_eq!(
        r.turn_player(),
        0,
        "paying 6 from 10 memory keeps the turn with Player A"
    );
    assert_eq!(
        r.memory(),
        memory_before - 6,
        "the DigiXros play must cost 16 −3 −3 −2×2 = 6 memory"
    );

    // The placed cards left Yuu Amano's source stack.
    let yuu = r
        .game
        .players[0]
        .battle_area
        .iter()
        .find(|p| p.top_card().card_id(&r.game.card_data) == "BT10-093")
        .expect("Yuu Amano remains on field");
    assert!(
        !yuu.card_sources
            .iter()
            .any(|c| matches!(c.card_id(&r.game.card_data), "EX10-039" | "EX10-044")),
        "both purple cards must leave Yuu Amano's source stack"
    );
    // Both hand materials were consumed.
    assert_eq!(r.hand_size(0), 0, "Bagramon + DarkKnightmon left the hand as materials");
}

/// Q29 (only ChuuChuumon placed — "up to 3" allows fewer): legal stack L3 —
/// DarknessBagramon / ChuuChuumon / Bagramon / DarkKnightmon (top→bottom),
/// and the play costs 16 −3 −3 −2 = 8.
#[test]
fn q29_single_under_tamer_card_yields_third_legal_stack() {
    let mut r = q29_runner();
    let memory_before = r.memory();

    // Pick only ChuuChuumon (the lower under-Tamer slot), then PASS.
    q29_drive_darkness_bagramon_play(&mut r, &[false]);

    assert_eq!(
        q29_db_stack(&r),
        vec!["EX10-031", "EX10-056", "EX10-039", "EX10-059"],
        "judge Q29 (L3, bottom→top): DarkKnightmon / Bagramon / ChuuChuumon / \
         DarknessBagramon"
    );
    assert_eq!(
        r.memory(),
        memory_before - 8,
        "the DigiXros play must cost 16 −3 −3 −2 = 8 memory"
    );
    // Damemon stays under Yuu Amano.
    let yuu = r
        .game
        .players[0]
        .battle_area
        .iter()
        .find(|p| p.top_card().card_id(&r.game.card_data) == "BT10-093")
        .expect("Yuu Amano remains on field");
    assert!(
        yuu.card_sources
            .iter()
            .any(|c| c.card_id(&r.game.card_data) == "EX10-044"),
        "the unpicked Damemon must remain under Yuu Amano"
    );
}

// Q30 spans clusters C and E — its test lives in `c_declare_then_pay.rs`
// (`q30_partition_interruptive_suspends_both_with_cost_reduction`).
