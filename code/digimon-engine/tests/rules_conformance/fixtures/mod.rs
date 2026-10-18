//! Synthetic YAML DSL fixture cards for the rules_conformance chapters.
//!
//! ONE CARD PER FILE, one YAML document each, so `dsl-lint` lints them
//! directly (`cargo run -p dsl-lint -- code/digimon-engine/tests/rules_conformance/fixtures`):
//! `common/<CARD_ID>.yaml` holds cards reused across chapters, and each
//! chapter adds `<chapter>/<CARD_ID>.yaml`. Every effect clause carries a
//! `summary:` with the printed-style text it stands for, and each file's
//! header comment gives the whole card face. Cards are authored to the PRINTED
//! phrasing (mandatory text stays mandatory; only "may" / "up to" is
//! optional), never to the engine's current behaviour. Change a `common/` card
//! only if every chapter using it still reads the same printed text.
//!
//! Adding a card: create the file and append its `include_str!` to the list
//! below (kept sorted by card id).

#![allow(dead_code)] // each chapter uses a subset

use digimon_engine::debug_runner::{DebugRunner, DebugRunnerBuilder};

/// Cards shared by several chapters.
const COMMON: &[&str] = &[
    include_str!("common/LV5_PLAIN.yaml"),
    include_str!("common/OD_MEM.yaml"),
    include_str!("common/OPP_WA_OBS.yaml"),
    include_str!("common/OP_PLAY_LV3.yaml"),
    include_str!("common/VANILLA_LV3.yaml"),
    include_str!("common/WA_DRAW.yaml"),
    include_str!("common/WA_MEM.yaml"),
];

/// §11-14 "Attacking", "Blocking", "Security Checks", "Battles" chapter
/// cards, with the §16-4/§16-5/§16-7/§16-9 keyword cards
/// (`ch11_14_attack_procedure.rs`).
const CH11_14: &[&str] = &[
    include_str!("ch11_14/BLOCKER.yaml"),
    include_str!("ch11_14/BLOCKER_CANNOT_SUSPEND.yaml"),
    include_str!("ch11_14/BLOCKER_WAKES.yaml"),
    include_str!("ch11_14/COUNTER_MEM.yaml"),
    include_str!("ch11_14/EOA_OBS.yaml"),
    include_str!("ch11_14/EOB_OBS.yaml"),
    include_str!("ch11_14/JAMMING.yaml"),
    include_str!("ch11_14/PIERCING_X2.yaml"),
    include_str!("ch11_14/SA_PLUS1.yaml"),
    include_str!("ch11_14/SEC_CHECK_OBS_MEM.yaml"),
    include_str!("ch11_14/SEC_CHECK_OBS_MINUS_DP.yaml"),
    include_str!("ch11_14/SEC_MEM_OPT.yaml"),
    include_str!("ch11_14/WA_DEL_SELF.yaml"),
    include_str!("ch11_14/WA_DP_UP_EOA_MAY.yaml"),
];

/// §15-4 "Effect States" chapter cards (`ch15_04_effect_states.rs`).
const CH15_04: &[&str] = &[
    include_str!("ch15_04/BIG_LV6.yaml"),
    include_str!("ch15_04/HYBRID_LV5.yaml"),
    include_str!("ch15_04/OBS_OPP_DEL_MEM.yaml"),
    include_str!("ch15_04/OBS_PLAY_LE4K.yaml"),
    include_str!("ch15_04/OD_PLAY_LV6.yaml"),
    include_str!("ch15_04/OD_RECOVER_LE3.yaml"),
    include_str!("ch15_04/PIERCE_IF_TOP.yaml"),
    include_str!("ch15_04/PLAY_DP_UP.yaml"),
    include_str!("ch15_04/RET_PLAYED.yaml"),
    include_str!("ch15_04/SWEEPER.yaml"),
    include_str!("ch15_04/WA_ALLIANCE.yaml"),
    include_str!("ch15_04/WA_DEL4K.yaml"),
    include_str!("ch15_04/WA_DRAW_INH.yaml"),
    include_str!("ch15_04/WA_HYBRID_DIGI.yaml"),
    include_str!("ch15_04/WA_MEM_INH.yaml"),
    include_str!("ch15_04/WD_DEL_RET.yaml"),
    include_str!("ch15_04/WIN_MEM.yaml"),
    include_str!("ch15_04/WOULD_DEL_STRIKE.yaml"),
];

/// §15-7 "Optional Processing Conditions" chapter cards
/// (`ch15_07_optional_conditions.rs`).
const CH15_07: &[&str] = &[
    include_str!("ch15_07/GARU_LV4.yaml"),
    include_str!("ch15_07/KIMERAMON.yaml"),
    include_str!("ch15_07/LEND_HAND_MEM.yaml"),
    include_str!("ch15_07/LEND_SEC_MEM.yaml"),
    include_str!("ch15_07/MACHINEDRAMON.yaml"),
    include_str!("ch15_07/MAIN_TRASH_SEC_DRAW.yaml"),
    include_str!("ch15_07/OD_RETURN_KIMERA_MACHINE_PLAY_MILLENNIUM.yaml"),
    include_str!("ch15_07/OD_TRASH_HAND_MEM.yaml"),
    include_str!("ch15_07/WA_BORROW_OP.yaml"),
    include_str!("ch15_07/WA_BORROW_OP_THEN_MEM.yaml"),
    include_str!("ch15_07/WA_TRASH_SEC_MINUS5000.yaml"),
    include_str!("ch15_07/WD_TRASH2_UNSUSPEND_THEN.yaml"),
    include_str!("ch15_07/WEREGARU_LV5.yaml"),
];

/// §15-8 "Effect Categories" chapter cards (`ch15_08_effect_categories.rs`).
const CH15_08: &[&str] = &[
    include_str!("ch15_08/AURA_TS_TAMER.yaml"),
    include_str!("ch15_08/DNA_LV6.yaml"),
    include_str!("ch15_08/EOT_MEM.yaml"),
    include_str!("ch15_08/EOT_MEM3.yaml"),
    include_str!("ch15_08/GREY_DIGICARD_DP.yaml"),
    include_str!("ch15_08/GREY_LV4.yaml"),
    include_str!("ch15_08/IMM_COND.yaml"),
    include_str!("ch15_08/IMM_TWO_MAY.yaml"),
    include_str!("ch15_08/INH_GREY_NAME_DP.yaml"),
    include_str!("ch15_08/INH_OD_MEM.yaml"),
    include_str!("ch15_08/INH_TRASHED_FROM_TS.yaml"),
    include_str!("ch15_08/LV4_PLAIN.yaml"),
    include_str!("ch15_08/MAIN_DEL1.yaml"),
    include_str!("ch15_08/MAIN_DEL2.yaml"),
    include_str!("ch15_08/MAIN_DEL_ALL.yaml"),
    include_str!("ch15_08/MAIN_PLAY_LV4.yaml"),
    include_str!("ch15_08/MAIN_TRASH_BOTTOM_SRC.yaml"),
    include_str!("ch15_08/MAIN_TRASH_UNDER_MEM.yaml"),
    include_str!("ch15_08/OBS_ATK_CHRON.yaml"),
    include_str!("ch15_08/OBS_ATK_GE10K.yaml"),
    include_str!("ch15_08/OBS_ATK_GE5.yaml"),
    include_str!("ch15_08/OBS_EFFPLAY_DEL_LE.yaml"),
    include_str!("ch15_08/OBS_OPP_PLAY_GE4.yaml"),
    include_str!("ch15_08/OBS_OPP_PLAY_IF_LE4K.yaml"),
    include_str!("ch15_08/OBS_OPP_PLAY_LE4K.yaml"),
    include_str!("ch15_08/OBS_OPP_PLAY_LVL.yaml"),
    include_str!("ch15_08/OBS_TAMER_TRASH.yaml"),
    include_str!("ch15_08/OBS_WIN_DIGI_TS.yaml"),
    include_str!("ch15_08/OBS_WIN_DNA.yaml"),
    include_str!("ch15_08/OBS_WIN_RET_LV3.yaml"),
    include_str!("ch15_08/OBS_WIN_TS.yaml"),
    include_str!("ch15_08/OD_DEL_ALL_OPP.yaml"),
    include_str!("ch15_08/OD_REPLAY_SRC.yaml"),
    include_str!("ch15_08/OD_RET_SELF.yaml"),
    include_str!("ch15_08/OP_DIGI_LV3.yaml"),
    include_str!("ch15_08/OP_DIGI_LV4.yaml"),
    include_str!("ch15_08/OP_TRASH_UNDER_MEM.yaml"),
    include_str!("ch15_08/PLAIN_TAMER.yaml"),
    include_str!("ch15_08/PLAY_DP_DOWN.yaml"),
    include_str!("ch15_08/PUMP_OPT.yaml"),
    include_str!("ch15_08/PLAY_RET_LE_SEC.yaml"),
    include_str!("ch15_08/REC_PLAY.yaml"),
    include_str!("ch15_08/SCAPEGOAT_LIKE.yaml"),
    include_str!("ch15_08/SUKAMON.yaml"),
    include_str!("ch15_08/TRAIN.yaml"),
    include_str!("ch15_08/TS_LV5.yaml"),
    include_str!("ch15_08/TS_LV6.yaml"),
    include_str!("ch15_08/WA_HAND7_DRAW.yaml"),
    include_str!("ch15_08/WA_RET_CHRON.yaml"),
    include_str!("ch15_08/WOULD_DEL_MEM.yaml"),
];

/// §15-15-5 "isn't affected by effects" chapter cards
/// (`ch15_15_unaffected.rs`).
const CH15_15: &[&str] = &[
    include_str!("ch15_15/DIGI_GRANT_EOT_DELETE.yaml"),
    include_str!("ch15_15/DIGI_GRANT_KW_SA_MINUS1.yaml"),
    include_str!("ch15_15/DIGI_GRANT_SA_MINUS1.yaml"),
    include_str!("ch15_15/DIGI_MINUS_3K.yaml"),
    include_str!("ch15_15/DIGI_SUSPEND_THEN_MINUS3K.yaml"),
    include_str!("ch15_15/HUGE_LV7.yaml"),
    include_str!("ch15_15/IMMUNE_ALL_OPP.yaml"),
    include_str!("ch15_15/IMMUNE_DIGI_ONLY.yaml"),
    include_str!("ch15_15/IMMUNE_WHILE_SUSPENDED.yaml"),
    include_str!("ch15_15/OPP_ATK_ALL_MINUS_12K.yaml"),
    include_str!("ch15_15/OPP_ATK_DELETE.yaml"),
    include_str!("ch15_15/OPP_ATK_TRASH_OPP_SEC.yaml"),
    include_str!("ch15_15/OPP_AURA_MINUS_PER_FIELD.yaml"),
    include_str!("ch15_15/OPT_DELETE_13K.yaml"),
    include_str!("ch15_15/OPT_MINUS_12K.yaml"),
    include_str!("ch15_15/OWN_MINUS_3K.yaml"),
    include_str!("ch15_15/PROGRESS_SEC4.yaml"),
    include_str!("ch15_15/SELF_IMM_THEN_UP.yaml"),
    include_str!("ch15_15/TAMER_MINUS_3K.yaml"),
    include_str!("ch15_15/TAMER_PROGRESS_SEC4.yaml"),
    include_str!("ch15_15/TOKEN_OD_MINUS_3K.yaml"),
];

/// §16-42..§16-47 Ver. 4.x keyword cards, with the §4-6 DUAL, §4-19
/// <Overflow> and §4-20 Arts Digivolve cards (`ch16_new_keywords.rs`).
const CH16_NEW: &[&str] = &[
    include_str!("ch16_new/ACE.yaml"),
    include_str!("ch16_new/ASCEND.yaml"),
    include_str!("ch16_new/BASEMON_A.yaml"),
    include_str!("ch16_new/BASEMON_B.yaml"),
    include_str!("ch16_new/DETACH.yaml"),
    include_str!("ch16_new/DUAL_ARTS.yaml"),
    include_str!("ch16_new/DUALSRC_DIGI.yaml"),
    include_str!("ch16_new/DUALSRC_OPT.yaml"),
    include_str!("ch16_new/DUALSRC_PURPLE.yaml"),
    include_str!("ch16_new/DUALSRC_RED.yaml"),
    include_str!("ch16_new/ENGAGE.yaml"),
    include_str!("ch16_new/GUARD.yaml"),
    include_str!("ch16_new/HAND_PICK_BLUE_ANY.yaml"),
    include_str!("ch16_new/HAND_PICK_NOT_RED_ANY.yaml"),
    include_str!("ch16_new/HAND_PICK_PURPLE_LV6_ANY.yaml"),
    include_str!("ch16_new/HAND_PICK_DIGI.yaml"),
    include_str!("ch16_new/HAND_PICK_OPT.yaml"),
    include_str!("ch16_new/HAND_PICK_PURPLE_DIGI.yaml"),
    include_str!("ch16_new/HAND_PICK_PURPLE_OPT.yaml"),
    include_str!("ch16_new/HAND_PICK_PURPLE_ANY.yaml"),
    include_str!("ch16_new/HAND_PICK_RED_OPT.yaml"),
    include_str!("ch16_new/HAND_PICK_RED_ANY.yaml"),
    include_str!("ch16_new/IMM_DIGI.yaml"),
    include_str!("ch16_new/IMM_OPT.yaml"),
    include_str!("ch16_new/LOWMON.yaml"),
    include_str!("ch16_new/MAIN_BOUNCE1.yaml"),
    include_str!("ch16_new/MAIN_DEL_ALL_ANY.yaml"),
    include_str!("ch16_new/MAIN_DEL_OWN_MEM.yaml"),
    include_str!("ch16_new/MAIN_PLACE_UNDER.yaml"),
    include_str!("ch16_new/MAIN_TRASH_OPP_OPT.yaml"),
    include_str!("ch16_new/PURPLE_LV3.yaml"),
    include_str!("ch16_new/SEVEN_LINK.yaml"),
    include_str!("ch16_new/SRC_PICK_PURPLE_ANY.yaml"),
    include_str!("ch16_new/SRC_PICK_RED_ANY.yaml"),
    include_str!("ch16_new/SUCC_TOP.yaml"),
    include_str!("ch16_new/TS_DIGI.yaml"),
    include_str!("ch16_new/TS_TAMER.yaml"),
    include_str!("ch16_new/USE_REQ_OPT.yaml"),
];

/// §16-18..§16-37 replacement (immediate-type) keyword cards
/// (`ch16_replacement_keywords.rs`).
const CH16_REPL: &[&str] = &[
    include_str!("ch16_repl/ARMOR.yaml"),
    include_str!("ch16_repl/BARRIER.yaml"),
    include_str!("ch16_repl/BARRIER_INH.yaml"),
    include_str!("ch16_repl/BLACK_LV3.yaml"),
    include_str!("ch16_repl/BLUE_LV4.yaml"),
    include_str!("ch16_repl/DECODE.yaml"),
    include_str!("ch16_repl/DECOY.yaml"),
    include_str!("ch16_repl/EVADE.yaml"),
    include_str!("ch16_repl/EVADE_FRAG1.yaml"),
    include_str!("ch16_repl/FRAG2.yaml"),
    include_str!("ch16_repl/GREEN_LV4.yaml"),
    include_str!("ch16_repl/MAIN_DEL_FRAG.yaml"),
    include_str!("ch16_repl/PARTITION.yaml"),
    include_str!("ch16_repl/SCAPEGOAT.yaml"),
];

/// §16-4..§16-39 attack keyword cards (`ch16_attack_keywords.rs`).
const CH16_ATK: &[&str] = &[
    include_str!("ch16_atk/ALLIANCE.yaml"),
    include_str!("ch16_atk/BLITZ.yaml"),
    include_str!("ch16_atk/BLITZ_WA.yaml"),
    include_str!("ch16_atk/BLOCKER_INH.yaml"),
    include_str!("ch16_atk/COLLISION.yaml"),
    include_str!("ch16_atk/COUNTER_DEL5K.yaml"),
    include_str!("ch16_atk/COUNTER_MINUS3000.yaml"),
    include_str!("ch16_atk/EXECUTE.yaml"),
    include_str!("ch16_atk/ICECLAD.yaml"),
    include_str!("ch16_atk/IMMUNE.yaml"),
    include_str!("ch16_atk/OBS_DIGI_LOSE2.yaml"),
    include_str!("ch16_atk/OBS_OWN_DEL_MEM.yaml"),
    include_str!("ch16_atk/OPP_WA_MINUS3000.yaml"),
    include_str!("ch16_atk/OVERCLOCK.yaml"),
    include_str!("ch16_atk/PIERCING.yaml"),
    include_str!("ch16_atk/PROGRESS.yaml"),
    include_str!("ch16_atk/PUPPET.yaml"),
    include_str!("ch16_atk/RAID.yaml"),
    include_str!("ch16_atk/RAID_WA.yaml"),
    include_str!("ch16_atk/RETALIATION.yaml"),
    include_str!("ch16_atk/RUSH.yaml"),
    include_str!("ch16_atk/RUSH_AURA.yaml"),
    include_str!("ch16_atk/SA_MINUS2.yaml"),
    include_str!("ch16_atk/VORTEX.yaml"),
    include_str!("ch16_atk/WA_GAIN_SA.yaml"),
    include_str!("ch16_atk/WA_ICECLAD.yaml"),
    include_str!("ch16_atk/WA_JAM.yaml"),
    include_str!("ch16_atk/WA_PROGRESS.yaml"),
];

/// §16-6..§16-41 processing keyword cards (`ch16_processing_keywords.rs`).
const CH16_PROC: &[&str] = &[
    include_str!("ch16_proc/BDNA.yaml"),
    include_str!("ch16_proc/BDNA_NOREQ.yaml"),
    include_str!("ch16_proc/BLAST.yaml"),
    include_str!("ch16_proc/BURST.yaml"),
    include_str!("ch16_proc/DEDIGI1X2_OPT.yaml"),
    include_str!("ch16_proc/DEDIGI1_OPT.yaml"),
    include_str!("ch16_proc/DEDIGI2_OPT.yaml"),
    include_str!("ch16_proc/DELAY_MEM.yaml"),
    include_str!("ch16_proc/DELAY_SOYT.yaml"),
    include_str!("ch16_proc/DIGISORB.yaml"),
    include_str!("ch16_proc/DRAW_OPT.yaml"),
    include_str!("ch16_proc/EFFLINK.yaml"),
    include_str!("ch16_proc/FORT.yaml"),
    include_str!("ch16_proc/IMMUNE_LV4.yaml"),
    include_str!("ch16_proc/LEFT4.yaml"),
    include_str!("ch16_proc/LEFT5.yaml"),
    include_str!("ch16_proc/LINKPLUS2.yaml"),
    include_str!("ch16_proc/LINKPLUS_AURA.yaml"),
    include_str!("ch16_proc/MINDLINK.yaml"),
    include_str!("ch16_proc/MSAVE2.yaml"),
    include_str!("ch16_proc/NOSUSP3.yaml"),
    include_str!("ch16_proc/OBS_SRC_TRASH_MEM.yaml"),
    include_str!("ch16_proc/OBS_TOP_TRASH_MEM.yaml"),
    include_str!("ch16_proc/OPT_TRASH_OWN_TOP.yaml"),
    include_str!("ch16_proc/OPT_TRASH_OWN_TOP2.yaml"),
    include_str!("ch16_proc/REBOOT.yaml"),
    include_str!("ch16_proc/REBOOT_INH.yaml"),
    include_str!("ch16_proc/REC_OPT.yaml"),
    include_str!("ch16_proc/RELINK.yaml"),
    include_str!("ch16_proc/RIGHT5.yaml"),
    include_str!("ch16_proc/SAVE.yaml"),
    include_str!("ch16_proc/UNSUSP_OBS_INH.yaml"),
    include_str!("ch16_proc/XROSSA.yaml"),
    include_str!("ch16_proc/XROSSB.yaml"),
];

/// §17 "Rule Checks" chapter cards (`ch17_rule_checks.rs`).
const CH17: &[&str] = &[
    include_str!("ch17/LINK_A.yaml"),
    include_str!("ch17/LINK_B.yaml"),
    include_str!("ch17/LINK_HOST_LV4.yaml"),
    include_str!("ch17/MAIN_PLACE_SELF.yaml"),
    include_str!("ch17/NO_DP_DIGIMON.yaml"),
    include_str!("ch17/OPP_TURN_MINUS3000.yaml"),
    include_str!("ch17/OPT_PLAIN.yaml"),
    include_str!("ch17/OP_MINUS3000_SA_MINUS1.yaml"),
    include_str!("ch17/SOYT_MEM.yaml"),
    include_str!("ch17/TAMER_PLAIN.yaml"),
    include_str!("ch17/WD_PLAY_ZEROMON.yaml"),
    include_str!("ch17/WOULD_DEL_MAY_MEM.yaml"),
    include_str!("ch17/ZEROMON.yaml"),
    include_str!("ch17/ZERO_OD_MEM.yaml"),
];

/// §18 "Other Information" chapter cards, with the §9-1-5 and §16-7 cards
/// (`ch18_pending_overwrite.rs`).
const CH18: &[&str] = &[
    include_str!("ch18/EOOT_MEM.yaml"),
    include_str!("ch18/EOT_MAY_MEM.yaml"),
    include_str!("ch18/DP_CHAIN.yaml"),
    include_str!("ch18/GREYMON_LV4.yaml"),
    include_str!("ch18/INH_PIERCE.yaml"),
    include_str!("ch18/LOOP_FLIP.yaml"),
    include_str!("ch18/LOOP_FLIP_CHOOSE.yaml"),
    include_str!("ch18/LOOP_FLIP_FINITE.yaml"),
    include_str!("ch18/LOOP_FLIP_MAY.yaml"),
    include_str!("ch18/LOOP_FLIP_PICK.yaml"),
    include_str!("ch18/LOOP_PICK_GROW.yaml"),
    include_str!("ch18/LOOP_TWIN_GUARD.yaml"),
    include_str!("ch18/MAIN_DEL_6000_OR_LOWEST_IF_GREYMON.yaml"),
    include_str!("ch18/MAIN_MEM_AT_EOONT.yaml"),
    include_str!("ch18/MAIN_ALL_MINUS3000.yaml"),
    include_str!("ch18/MAIN_MINUS3000.yaml"),
    include_str!("ch18/MAIN_PLAY_LV3.yaml"),
    include_str!("ch18/MAIN_PLAY_LV3_DEL_EOT.yaml"),
    include_str!("ch18/MAIN_TWO.yaml"),
    include_str!("ch18/OBS_OPP_PLAY_MEM.yaml"),
    include_str!("ch18/OD_DEL_OPP.yaml"),
    include_str!("ch18/OP_MAY_MEM.yaml"),
    include_str!("ch18/OP_MEM.yaml"),
    include_str!("ch18/OP_MEM_OR_DRAW_INSTEAD.yaml"),
    include_str!("ch18/OP_USE_OPT_FREE.yaml"),
    include_str!("ch18/PIERCE_EOA.yaml"),
    include_str!("ch18/WA_GAIN3_LOSE3_EOT.yaml"),
    include_str!("ch18/WA_LOOP.yaml"),
];

/// Every shared card's YAML source.
pub fn common_cards() -> &'static [&'static str] {
    COMMON
}

/// Every `ch11_14/` card's YAML source.
pub fn ch11_14_cards() -> &'static [&'static str] {
    CH11_14
}

/// Every `ch15_04/` card's YAML source.
pub fn ch15_04_cards() -> &'static [&'static str] {
    CH15_04
}

/// Every `ch15_07/` card's YAML source.
pub fn ch15_07_cards() -> &'static [&'static str] {
    CH15_07
}

/// Every `ch15_08/` card's YAML source.
pub fn ch15_08_cards() -> &'static [&'static str] {
    CH15_08
}

/// Every `ch15_15/` card's YAML source.
pub fn ch15_15_cards() -> &'static [&'static str] {
    CH15_15
}

/// Every `ch16_new/` card's YAML source.
pub fn ch16_new_cards() -> &'static [&'static str] {
    CH16_NEW
}

/// Every `ch16_repl/` card's YAML source.
pub fn ch16_repl_cards() -> &'static [&'static str] {
    CH16_REPL
}

/// Every `ch16_atk/` card's YAML source.
pub fn ch16_atk_cards() -> &'static [&'static str] {
    CH16_ATK
}

/// Every `ch16_proc/` card's YAML source.
pub fn ch16_proc_cards() -> &'static [&'static str] {
    CH16_PROC
}

/// Every `ch17/` card's YAML source.
pub fn ch17_cards() -> &'static [&'static str] {
    CH17
}

/// Every `ch18/` card's YAML source.
pub fn ch18_cards() -> &'static [&'static str] {
    CH18
}

/// Register each card of `cards` on `builder`. Panics with the card's compile
/// error, so a broken fixture fails loudly rather than silently missing.
pub fn load(mut builder: DebugRunnerBuilder, cards: &[&str]) -> DebugRunnerBuilder {
    for card in cards {
        builder = builder.from_dsl_yaml(card).unwrap_or_else(|e| {
            panic!(
                "fixture card failed to compile: {e}
---
{card}"
            )
        });
    }
    builder
}

/// A builder with the common cards plus each of `chapter_cards` loaded.
pub fn builder(chapter_cards: &[&[&str]]) -> DebugRunnerBuilder {
    let mut b = load(DebugRunner::builder(), common_cards());
    for cards in chapter_cards {
        b = load(b, cards);
    }
    b
}

/// A card file that exists but is missing from its `include_str!` list would
/// silently never load. Compare each list with the directory.
#[test]
fn every_fixture_file_is_listed() {
    let root = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/rules_conformance/fixtures"
    );
    for (dir, listed) in [
        ("common", COMMON.len()),
        ("ch11_14", CH11_14.len()),
        ("ch15_04", CH15_04.len()),
        ("ch15_07", CH15_07.len()),
        ("ch15_08", CH15_08.len()),
        ("ch15_15", CH15_15.len()),
        ("ch16_new", CH16_NEW.len()),
        ("ch16_repl", CH16_REPL.len()),
        ("ch16_atk", CH16_ATK.len()),
        ("ch16_proc", CH16_PROC.len()),
        ("ch17", CH17.len()),
        ("ch18", CH18.len()),
    ] {
        let on_disk = std::fs::read_dir(format!("{root}/{dir}"))
            .unwrap_or_else(|e| panic!("read fixtures/{dir}: {e}"))
            .filter_map(|e| e.ok())
            .filter(|e| e.file_name().to_string_lossy().ends_with(".yaml"))
            .count();
        assert_eq!(
            on_disk, listed,
            "fixtures/{dir} has {on_disk} .yaml files but {listed} are listed in fixtures/mod.rs"
        );
    }
}
