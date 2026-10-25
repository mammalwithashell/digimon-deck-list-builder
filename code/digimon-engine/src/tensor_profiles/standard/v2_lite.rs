use crate::tensor_profiles::{
    TensorFieldKind, TensorProfile, TensorSection, TensorSectionKind, TensorSlotField,
    TensorSlotLayout,
};

pub const PROFILE_ID: &str = "standard_lite_v2";
pub const GAME_MODE: &str = "standard";
pub const VERSION: u32 = 2;
pub const TENSOR_VERSION: u16 = 2;
// Task S1.4: PERM_MAX_SOURCES 11 -> 12 so every action-space source slot is
// observable. SOURCES_PER_FIELD (12) governs the `SOURCE_SELECT` and
// `BREEDING_SOURCE_SELECT` action ranges; the v2 tensor must surface all 12
// source slots (incl. a breeding-carrier's 12th source via
// BREEDING_SOURCE_SELECT) so the RL agent never picks a slot blind.
// This grows each permanent slot by 3 floats (96 -> 99), shifting every
// section after `permanent_slots` and bumping tensor_size 8320 -> 8410.
pub const FEATURE_SCHEMA_VERSION: &str = "standard_lite_v2.4";
pub const LAYOUT_HASH: &str =
    "sha256:b7916a3c03487daa53a7dffe87ce2ce44f8c43fa79e208726102f0f3811eee1a";

pub const GLOBAL_FEATURES_SIZE: usize = 64;
pub const PLAYER_SUMMARY_PLAYERS: usize = 2;
pub const PLAYER_SUMMARY_ROW_SIZE: usize = 32;
pub const PLAYER_SUMMARY_SIZE: usize = PLAYER_SUMMARY_PLAYERS * PLAYER_SUMMARY_ROW_SIZE;
pub const PERMANENT_PLAYERS: usize = 2;
pub const PERMANENT_SLOTS_PER_PLAYER: usize = 15;
pub const PERMANENT_ROW_COUNT: usize = PERMANENT_PLAYERS * PERMANENT_SLOTS_PER_PLAYER;
// Derived from the source-block layout: 63 header floats + one
// PERM_SOURCE_ENTRY_SIZE-wide entry per PERM_MAX_SOURCES slot.
pub const PERMANENT_SLOT_SIZE: usize =
    PERM_SOURCE_START_OFFSET + PERM_MAX_SOURCES * PERM_SOURCE_ENTRY_SIZE;
pub const PERMANENT_SLOTS_SIZE: usize = PERMANENT_ROW_COUNT * PERMANENT_SLOT_SIZE;
pub const OWN_HAND_ROWS: usize = 30;
pub const OWN_HAND_ROW_SIZE: usize = 32;
pub const OWN_HAND_SIZE: usize = OWN_HAND_ROWS * OWN_HAND_ROW_SIZE;
pub const KNOWN_ZONE_ROWS: usize = 120;
pub const KNOWN_ZONE_ROW_SIZE: usize = 8;
pub const KNOWN_ZONE_SIZE: usize = KNOWN_ZONE_ROWS * KNOWN_ZONE_ROW_SIZE;
pub const DECISION_CONTEXT_SIZE: usize = 64;
pub const PENDING_CHOICE_ROWS: usize = 32;
pub const PENDING_CHOICE_ROW_SIZE: usize = 128;
pub const PENDING_CHOICE_SIZE: usize = PENDING_CHOICE_ROWS * PENDING_CHOICE_ROW_SIZE;
pub const RESERVED_SIZE: usize = 256;

pub const OFF_GLOBAL_FEATURES: usize = 0;
pub const OFF_PLAYER_SUMMARY: usize = OFF_GLOBAL_FEATURES + GLOBAL_FEATURES_SIZE;
pub const OFF_PERMANENT_SLOTS: usize = OFF_PLAYER_SUMMARY + PLAYER_SUMMARY_SIZE;
pub const OFF_OWN_HAND: usize = OFF_PERMANENT_SLOTS + PERMANENT_SLOTS_SIZE;
pub const OFF_KNOWN_ZONE_CARDS: usize = OFF_OWN_HAND + OWN_HAND_SIZE;
pub const OFF_DECISION_CONTEXT: usize = OFF_KNOWN_ZONE_CARDS + KNOWN_ZONE_SIZE;
pub const OFF_PENDING_CHOICE_FEATURES: usize = OFF_DECISION_CONTEXT + DECISION_CONTEXT_SIZE;
pub const OFF_RESERVED: usize = OFF_PENDING_CHOICE_FEATURES + PENDING_CHOICE_SIZE;
pub const TENSOR_SIZE: usize = OFF_RESERVED + RESERVED_SIZE;

pub const PERM_TOP_CARD_ID_OFFSET: usize = 8;
pub const PERM_DP_OFFSET: usize = 21;
pub const PERM_SUSPENDED_OFFSET: usize = 22;
pub const PERM_SOURCE_COUNT_OFFSET: usize = 23;
pub const PERM_LINKED_COUNT_OFFSET: usize = 24;
pub const PERM_OPT_TOTAL_OFFSET: usize = 25;
pub const PERM_OPT_USED_OFFSET: usize = 26;
pub const PERM_SOURCE_START_OFFSET: usize = 63;
pub const PERM_SOURCE_ENTRY_SIZE: usize = 3;
pub const PERM_MAX_SOURCES: usize = 12;
pub const PERM_SOURCE_CARD_ID_OFFSET: usize = 0;
pub const PERM_SOURCE_OPT_STATE_OFFSET: usize = 1;
pub const PERM_SOURCE_DP_CONTRIBUTION_OFFSET: usize = 2;
pub const OWN_HAND_CARD_ID_OFFSET: usize = 1;
pub const KNOWN_ZONE_CARD_ID_OFFSET: usize = 1;
pub const PENDING_SOURCE_CARD_ID_OFFSET: usize = 44;

// Dedicated count-choice contract: explicitly allocated reserved fields.
pub const PENDING_COUNT_COMMIT_OFFSET: usize = 60;
pub const PENDING_COUNT_PREVIOUS_OFFSET: usize = 61;
pub const PENDING_COUNT_NEXT_OFFSET: usize = 62;
pub const PENDING_COUNT_DECLARED_OFFSET: usize = 63;
pub const PENDING_COUNT_PAYABLE_OFFSET: usize = 67;
pub const PENDING_COUNT_PAGE_INDEX_OFFSET: usize = 71;
pub const PENDING_COUNT_PAGE_COUNT_OFFSET: usize = 75;
pub const PENDING_COUNT_HOST_ID_OFFSET: usize = 79;
pub const PENDING_COUNT_HOST_PLAYER_OFFSET: usize = 83;
pub const PENDING_COUNT_HOST_SLOT_OFFSET: usize = 84;
pub const PENDING_COUNT_HOST_PRESENT_OFFSET: usize = 85;
pub const PENDING_COUNT_LIMBS: usize = 4;
pub const PENDING_COUNT_LIMB_DENOMINATOR: f32 = 65535.0;

pub const PENDING_CARD_PRESENT_OFFSET: usize = 86;
pub const PENDING_CARD_COMMIT_OFFSET: usize = 87;
pub const PENDING_CARD_PREVIOUS_OFFSET: usize = 88;
pub const PENDING_CARD_NEXT_OFFSET: usize = 89;
pub const PENDING_CARD_ZONE_OFFSET: usize = 90;
pub const PENDING_CARD_OWNER_OFFSET: usize = 91;
pub const PENDING_CARD_CARRIER_SLOT_OFFSET: usize = 92;
pub const PENDING_CARD_ID_OFFSET: usize = 93;
pub const PENDING_CARD_PROMPT_ID_OFFSET: usize = 94;
pub const PENDING_CARD_GENERATION_OFFSET: usize = 98;
pub const PENDING_CARD_PAGE_INDEX_OFFSET: usize = 102;
pub const PENDING_CARD_PAGE_COUNT_OFFSET: usize = 106;
pub const PENDING_CARD_POSITION_OFFSET: usize = 110;
pub const PENDING_CARD_CARRIER_ID_OFFSET: usize = 114;
pub const PENDING_CARD_CARRIER_PRESENT_OFFSET: usize = 118;
pub const PENDING_CARD_IDENTITY_VISIBLE_OFFSET: usize = 119;
pub const PENDING_CARD_DECLINE_OFFSET: usize = 120;
pub const PENDING_CARD_INSPECT_OFFSET: usize = 121;
pub const PENDING_CARD_ROW_OFFSETS: &[(&str, usize)] = &[
    ("present", PENDING_CARD_PRESENT_OFFSET), ("commit", PENDING_CARD_COMMIT_OFFSET),
    ("previous_page", PENDING_CARD_PREVIOUS_OFFSET), ("next_page", PENDING_CARD_NEXT_OFFSET),
    ("zone", PENDING_CARD_ZONE_OFFSET), ("relative_owner", PENDING_CARD_OWNER_OFFSET),
    ("carrier_slot", PENDING_CARD_CARRIER_SLOT_OFFSET), ("card_id", PENDING_CARD_ID_OFFSET),
    ("prompt_id_limbs", PENDING_CARD_PROMPT_ID_OFFSET), ("page_generation_limbs", PENDING_CARD_GENERATION_OFFSET),
    ("page_index_limbs", PENDING_CARD_PAGE_INDEX_OFFSET), ("page_count_limbs", PENDING_CARD_PAGE_COUNT_OFFSET),
    ("position_limbs", PENDING_CARD_POSITION_OFFSET), ("carrier_identity_limbs", PENDING_CARD_CARRIER_ID_OFFSET),
    ("carrier_present", PENDING_CARD_CARRIER_PRESENT_OFFSET), ("identity_visible", PENDING_CARD_IDENTITY_VISIBLE_OFFSET),
    ("decline", PENDING_CARD_DECLINE_OFFSET), ("inspect_only_row", PENDING_CARD_INSPECT_OFFSET),
];

pub const SHAPE_GLOBAL_FEATURES: &[usize] = &[GLOBAL_FEATURES_SIZE];
pub const SHAPE_PLAYER_SUMMARY: &[usize] = &[PLAYER_SUMMARY_PLAYERS, PLAYER_SUMMARY_ROW_SIZE];
pub const SHAPE_PERMANENT_SLOTS: &[usize] = &[
    PERMANENT_PLAYERS,
    PERMANENT_SLOTS_PER_PLAYER,
    PERMANENT_SLOT_SIZE,
];
pub const SHAPE_OWN_HAND: &[usize] = &[OWN_HAND_ROWS, OWN_HAND_ROW_SIZE];
pub const SHAPE_KNOWN_ZONE_CARDS: &[usize] = &[KNOWN_ZONE_ROWS, KNOWN_ZONE_ROW_SIZE];
pub const SHAPE_DECISION_CONTEXT: &[usize] = &[DECISION_CONTEXT_SIZE];
pub const SHAPE_PENDING_CHOICE_FEATURES: &[usize] = &[PENDING_CHOICE_ROWS, PENDING_CHOICE_ROW_SIZE];
pub const SHAPE_RESERVED: &[usize] = &[RESERVED_SIZE];

pub const SECTIONS: &[TensorSection] = &[
    TensorSection {
        id: "global_features",
        start: OFF_GLOBAL_FEATURES,
        len: GLOBAL_FEATURES_SIZE,
        shape: SHAPE_GLOBAL_FEATURES,
        kind: TensorSectionKind::Scalars,
    },
    TensorSection {
        id: "player_summary",
        start: OFF_PLAYER_SUMMARY,
        len: PLAYER_SUMMARY_SIZE,
        shape: SHAPE_PLAYER_SUMMARY,
        kind: TensorSectionKind::Scalars,
    },
    TensorSection {
        id: "permanent_slots",
        start: OFF_PERMANENT_SLOTS,
        len: PERMANENT_SLOTS_SIZE,
        shape: SHAPE_PERMANENT_SLOTS,
        kind: TensorSectionKind::StandardLiteV2Rows,
    },
    TensorSection {
        id: "own_hand",
        start: OFF_OWN_HAND,
        len: OWN_HAND_SIZE,
        shape: SHAPE_OWN_HAND,
        kind: TensorSectionKind::StandardLiteV2Rows,
    },
    TensorSection {
        id: "known_zone_cards",
        start: OFF_KNOWN_ZONE_CARDS,
        len: KNOWN_ZONE_SIZE,
        shape: SHAPE_KNOWN_ZONE_CARDS,
        kind: TensorSectionKind::StandardLiteV2Rows,
    },
    TensorSection {
        id: "decision_context",
        start: OFF_DECISION_CONTEXT,
        len: DECISION_CONTEXT_SIZE,
        shape: SHAPE_DECISION_CONTEXT,
        kind: TensorSectionKind::Scalars,
    },
    TensorSection {
        id: "pending_choice_features",
        start: OFF_PENDING_CHOICE_FEATURES,
        len: PENDING_CHOICE_SIZE,
        shape: SHAPE_PENDING_CHOICE_FEATURES,
        kind: TensorSectionKind::StandardLiteV2Rows,
    },
    TensorSection {
        id: "reserved",
        start: OFF_RESERVED,
        len: RESERVED_SIZE,
        shape: SHAPE_RESERVED,
        kind: TensorSectionKind::Scalars,
    },
];

pub const SLOT_HEADER_FIELDS: &[TensorSlotField] = &[
    TensorSlotField {
        id: "top_card_id",
        offset: PERM_TOP_CARD_ID_OFFSET,
        kind: TensorFieldKind::CardId,
    },
    TensorSlotField {
        id: "dp",
        offset: PERM_DP_OFFSET,
        kind: TensorFieldKind::Scalar,
    },
    TensorSlotField {
        id: "suspended",
        offset: PERM_SUSPENDED_OFFSET,
        kind: TensorFieldKind::Scalar,
    },
    TensorSlotField {
        id: "opt_total",
        offset: PERM_OPT_TOTAL_OFFSET,
        kind: TensorFieldKind::Scalar,
    },
    TensorSlotField {
        id: "opt_used",
        offset: PERM_OPT_USED_OFFSET,
        kind: TensorFieldKind::Scalar,
    },
    TensorSlotField {
        id: "linked_count",
        offset: PERM_LINKED_COUNT_OFFSET,
        kind: TensorFieldKind::Scalar,
    },
    TensorSlotField {
        id: "source_count",
        offset: PERM_SOURCE_COUNT_OFFSET,
        kind: TensorFieldKind::Scalar,
    },
];

pub const SOURCE_FIELDS: &[TensorSlotField] = &[
    TensorSlotField {
        id: "card_id",
        offset: PERM_SOURCE_CARD_ID_OFFSET,
        kind: TensorFieldKind::CardId,
    },
    TensorSlotField {
        id: "opt_state",
        offset: PERM_SOURCE_OPT_STATE_OFFSET,
        kind: TensorFieldKind::Scalar,
    },
    TensorSlotField {
        id: "dp_contribution",
        offset: PERM_SOURCE_DP_CONTRIBUTION_OFFSET,
        kind: TensorFieldKind::Scalar,
    },
];

pub const SLOT_LAYOUT: TensorSlotLayout = TensorSlotLayout {
    size: PERMANENT_SLOT_SIZE,
    source_start: PERM_SOURCE_START_OFFSET,
    source_entry_size: PERM_SOURCE_ENTRY_SIZE,
    max_sources: PERM_MAX_SOURCES,
    header_fields: SLOT_HEADER_FIELDS,
    source_fields: SOURCE_FIELDS,
};

pub const CARD_ID_SLOT_COUNT: usize = PERMANENT_ROW_COUNT * (1 + PERM_MAX_SOURCES)
    + OWN_HAND_ROWS
    + KNOWN_ZONE_ROWS
    + 2 * PENDING_CHOICE_ROWS;
pub const SCALAR_SLOT_COUNT: usize = TENSOR_SIZE - CARD_ID_SLOT_COUNT;

pub const PROFILE: TensorProfile = TensorProfile {
    id: PROFILE_ID,
    game_mode: GAME_MODE,
    version: VERSION,
    tensor_version: TENSOR_VERSION,
    feature_schema_version: FEATURE_SCHEMA_VERSION,
    layout_hash: LAYOUT_HASH,
    tensor_size: TENSOR_SIZE,
    field_slots: PERMANENT_SLOTS_PER_PLAYER,
    slot_size: PERMANENT_SLOT_SIZE,
    max_sources: PERM_MAX_SOURCES,
    slot_layout: SLOT_LAYOUT,
    card_id_slot_count: CARD_ID_SLOT_COUNT,
    scalar_slot_count: SCALAR_SLOT_COUNT,
    sections: SECTIONS,
};

pub fn positions_for_section(
    section: &TensorSection,
    card_positions: &mut Vec<usize>,
    scalar_positions: &mut Vec<usize>,
) {
    let mut card_markers = vec![false; section.len];
    let mut mark_card = |position: usize| {
        assert!(
            (section.start..section.start + section.len).contains(&position),
            "card position {position} is outside section {}",
            section.id
        );
        let local = position - section.start;
        assert!(
            !card_markers[local],
            "duplicate card position {position} in section {}",
            section.id
        );
        card_markers[local] = true;
        card_positions.push(position);
    };

    match section.id {
        "permanent_slots" => {
            for row in 0..PERMANENT_ROW_COUNT {
                let row_base = section.start + row * PERMANENT_SLOT_SIZE;
                mark_card(row_base + PERM_TOP_CARD_ID_OFFSET);
                for source_index in 0..PERM_MAX_SOURCES {
                    mark_card(
                        row_base
                            + PERM_SOURCE_START_OFFSET
                            + source_index * PERM_SOURCE_ENTRY_SIZE
                            + PERM_SOURCE_CARD_ID_OFFSET,
                    );
                }
            }
        }
        "own_hand" => {
            for row in 0..OWN_HAND_ROWS {
                mark_card(section.start + row * OWN_HAND_ROW_SIZE + OWN_HAND_CARD_ID_OFFSET);
            }
        }
        "known_zone_cards" => {
            for row in 0..KNOWN_ZONE_ROWS {
                mark_card(section.start + row * KNOWN_ZONE_ROW_SIZE + KNOWN_ZONE_CARD_ID_OFFSET);
            }
        }
        "pending_choice_features" => {
            for row in 0..PENDING_CHOICE_ROWS {
                mark_card(
                    section.start + row * PENDING_CHOICE_ROW_SIZE + PENDING_SOURCE_CARD_ID_OFFSET,
                );
                mark_card(section.start + row * PENDING_CHOICE_ROW_SIZE + PENDING_CARD_ID_OFFSET);
            }
        }
        "own_original_decklist" => {
            for row in 0..super::v2_lite_deck::DECKLIST_ROWS {
                mark_card(
                    section.start
                        + row * super::v2_lite_deck::DECKLIST_ROW_SIZE
                        + super::v2_lite_deck::DECKLIST_CARD_ID_OFFSET,
                );
            }
        }
        _ => panic!("unsupported standard_lite_v2 row section {}", section.id),
    }

    drop(mark_card);
    for (offset, is_card) in card_markers.iter().enumerate() {
        if !is_card {
            scalar_positions.push(section.start + offset);
        }
    }
}
