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
/// (One card away — WarGreymon/MetalGarurumon/Omnimon/MMK implemented.)
#[test]
#[ignore = "BLOCKED-CARD: needs EX3-014 (Dorbickmon, the DigiXros host). AD1-004, AD1-014, AD1-025, BT17-095 implemented."]
fn q25_all_turns_fires_on_digixros_departure_not_battle() {}

/// Q29 — Yuu Amano (BT10-093) top-placement (either order) + DigiXros bottom
/// placement (spec order): 3 legal DarknessBagramon (EX10-059) stacks. Judge:
/// the 3 specific orderings.
#[test]
#[ignore = "BLOCKED-CARD: needs BT10-093 (Yuu Amano), EX10-039 (ChuuChuumon), EX10-044 (Damemon), EX10-059 (DarknessBagramon), EX10-056 (Bagramon), EX10-031 (DarkKnightmon)."]
fn q29_legal_digixros_stack_orderings_with_yuu_amano() {}

// Q30 spans clusters C and E — its test lives in `c_declare_then_pay.rs`
// (`q30_partition_interruptive_suspends_both_with_cost_reduction`).
