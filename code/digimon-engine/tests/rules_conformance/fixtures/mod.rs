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

/// §15-4 "Effect States" chapter cards (`ch15_04_effect_states.rs`).
const CH15_04: &[&str] = &[
    include_str!("ch15_04/BIG_LV6.yaml"),
    include_str!("ch15_04/HYBRID_LV5.yaml"),
    include_str!("ch15_04/OBS_OPP_DEL_MEM.yaml"),
    include_str!("ch15_04/OBS_PLAY_LE4K.yaml"),
    include_str!("ch15_04/OD_RECOVER_LE3.yaml"),
    include_str!("ch15_04/PLAY_DP_UP.yaml"),
    include_str!("ch15_04/RET_PLAYED.yaml"),
    include_str!("ch15_04/SWEEPER.yaml"),
    include_str!("ch15_04/WA_ALLIANCE.yaml"),
    include_str!("ch15_04/WA_DEL4K.yaml"),
    include_str!("ch15_04/WA_DRAW_INH.yaml"),
    include_str!("ch15_04/WA_HYBRID_DIGI.yaml"),
    include_str!("ch15_04/WA_MEM_INH.yaml"),
    include_str!("ch15_04/WD_DEL_RET.yaml"),
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
    include_str!("ch15_07/WA_TRASH_SEC_MINUS5000.yaml"),
    include_str!("ch15_07/WD_TRASH2_UNSUSPEND_THEN.yaml"),
    include_str!("ch15_07/WEREGARU_LV5.yaml"),
];

/// §15-8 "Effect Categories" chapter cards (`ch15_08_effect_categories.rs`).
const CH15_08: &[&str] = &[
    include_str!("ch15_08/EOT_MEM.yaml"),
    include_str!("ch15_08/EOT_MEM3.yaml"),
    include_str!("ch15_08/IMM_COND.yaml"),
    include_str!("ch15_08/IMM_TWO_MAY.yaml"),
    include_str!("ch15_08/INH_OD_MEM.yaml"),
    include_str!("ch15_08/LV4_PLAIN.yaml"),
    include_str!("ch15_08/MAIN_DEL1.yaml"),
    include_str!("ch15_08/MAIN_DEL2.yaml"),
    include_str!("ch15_08/MAIN_DEL_ALL.yaml"),
    include_str!("ch15_08/MAIN_PLAY_LV4.yaml"),
    include_str!("ch15_08/MAIN_TRASH_UNDER_MEM.yaml"),
    include_str!("ch15_08/OBS_EFFPLAY_DEL_LE.yaml"),
    include_str!("ch15_08/OBS_OPP_PLAY_GE4.yaml"),
    include_str!("ch15_08/OBS_OPP_PLAY_LVL.yaml"),
    include_str!("ch15_08/OBS_TAMER_TRASH.yaml"),
    include_str!("ch15_08/OD_REPLAY_SRC.yaml"),
    include_str!("ch15_08/OD_RET_SELF.yaml"),
    include_str!("ch15_08/OP_DIGI_LV3.yaml"),
    include_str!("ch15_08/OP_DIGI_LV4.yaml"),
    include_str!("ch15_08/OP_TRASH_UNDER_MEM.yaml"),
    include_str!("ch15_08/PLAIN_TAMER.yaml"),
    include_str!("ch15_08/PLAY_RET_LE_SEC.yaml"),
    include_str!("ch15_08/REC_PLAY.yaml"),
    include_str!("ch15_08/SCAPEGOAT_LIKE.yaml"),
    include_str!("ch15_08/SUKAMON.yaml"),
    include_str!("ch15_08/TRAIN.yaml"),
    include_str!("ch15_08/WOULD_DEL_MEM.yaml"),
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

/// Every shared card's YAML source.
pub fn common_cards() -> &'static [&'static str] {
    COMMON
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

/// Every `ch17/` card's YAML source.
pub fn ch17_cards() -> &'static [&'static str] {
    CH17
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
        ("ch15_04", CH15_04.len()),
        ("ch15_07", CH15_07.len()),
        ("ch15_08", CH15_08.len()),
        ("ch17", CH17.len()),
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
