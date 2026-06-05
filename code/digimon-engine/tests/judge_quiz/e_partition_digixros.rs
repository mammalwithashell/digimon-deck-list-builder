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
#[test]
#[ignore = "BLOCKED-CARD: needs BT19-073 (LordKnightmon X), BT17-016 (Gallantmon), BT12-016 (WarGrowlmon), EX3-057 (Growlmon). BT19-072, BT20-102, EX8-073, EX4-006 implemented."]
fn q15_sequential_de_digivolve_halted_by_x_antibody_immunity() {}

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

/// Q29 — Yuu Amano (BT10-093) top-placement (either order) + DigiXros bottom
/// placement (spec order): 3 legal DarknessBagramon (EX10-059) stacks. Judge:
/// the 3 specific orderings.
#[test]
#[ignore = "BLOCKED-CARD: needs BT10-093 (Yuu Amano), EX10-039 (ChuuChuumon), EX10-044 (Damemon), EX10-059 (DarknessBagramon), EX10-056 (Bagramon), EX10-031 (DarkKnightmon)."]
fn q29_legal_digixros_stack_orderings_with_yuu_amano() {}

// Q30 spans clusters C and E — its test lives in `c_declare_then_pay.rs`
// (`q30_partition_interruptive_suspends_both_with_cost_reduction`).
