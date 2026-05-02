use crate::tensor_profiles::{
    TensorFieldKind, TensorProfile, TensorSection, TensorSectionKind, TensorSlotField,
    TensorSlotLayout,
};

pub const PROFILE_ID: &str = "standard_lite_v2";
pub const GAME_MODE: &str = "standard";
pub const VERSION: u32 = 2;
pub const TENSOR_VERSION: u16 = 2;
pub const FEATURE_SCHEMA_VERSION: &str = "standard_lite_v2.1";
pub const LAYOUT_HASH: &str =
    "sha256:f9249e6af39248d8f44074b5709ec5b30c665c0978c79a4328510dc2784541f0";

pub const GLOBAL_FEATURES_SIZE: usize = 64;
pub const PLAYER_SUMMARY_PLAYERS: usize = 2;
pub const PLAYER_SUMMARY_ROW_SIZE: usize = 32;
pub const PLAYER_SUMMARY_SIZE: usize = PLAYER_SUMMARY_PLAYERS * PLAYER_SUMMARY_ROW_SIZE;
pub const PERMANENT_PLAYERS: usize = 2;
pub const PERMANENT_SLOTS_PER_PLAYER: usize = 15;
pub const PERMANENT_ROW_COUNT: usize = PERMANENT_PLAYERS * PERMANENT_SLOTS_PER_PLAYER;
pub const PERMANENT_SLOT_SIZE: usize = 96;
pub const PERMANENT_SLOTS_SIZE: usize = PERMANENT_ROW_COUNT * PERMANENT_SLOT_SIZE;
pub const OWN_HAND_ROWS: usize = 30;
pub const OWN_HAND_ROW_SIZE: usize = 32;
pub const OWN_HAND_SIZE: usize = OWN_HAND_ROWS * OWN_HAND_ROW_SIZE;
pub const KNOWN_ZONE_ROWS: usize = 120;
pub const KNOWN_ZONE_ROW_SIZE: usize = 8;
pub const KNOWN_ZONE_SIZE: usize = KNOWN_ZONE_ROWS * KNOWN_ZONE_ROW_SIZE;
pub const DECISION_CONTEXT_SIZE: usize = 64;
pub const PENDING_CHOICE_ROWS: usize = 32;
pub const PENDING_CHOICE_ROW_SIZE: usize = 96;
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
pub const PERM_SOURCE_START_OFFSET: usize = 63;
pub const PERM_SOURCE_ENTRY_SIZE: usize = 3;
pub const PERM_MAX_SOURCES: usize = 11;
pub const OWN_HAND_CARD_ID_OFFSET: usize = 1;
pub const KNOWN_ZONE_CARD_ID_OFFSET: usize = 1;
pub const PENDING_SOURCE_CARD_ID_OFFSET: usize = 44;

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
        kind: TensorSectionKind::Custom,
    },
    TensorSection {
        id: "own_hand",
        start: OFF_OWN_HAND,
        len: OWN_HAND_SIZE,
        shape: SHAPE_OWN_HAND,
        kind: TensorSectionKind::Custom,
    },
    TensorSection {
        id: "known_zone_cards",
        start: OFF_KNOWN_ZONE_CARDS,
        len: KNOWN_ZONE_SIZE,
        shape: SHAPE_KNOWN_ZONE_CARDS,
        kind: TensorSectionKind::Custom,
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
        kind: TensorSectionKind::Custom,
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
        id: "source_count",
        offset: PERM_SOURCE_START_OFFSET - 1,
        kind: TensorFieldKind::Scalar,
    },
];

pub const SOURCE_FIELDS: &[TensorSlotField] = &[
    TensorSlotField {
        id: "card_id",
        offset: 0,
        kind: TensorFieldKind::CardId,
    },
    TensorSlotField {
        id: "flags",
        offset: 1,
        kind: TensorFieldKind::Scalar,
    },
    TensorSlotField {
        id: "age_or_dp",
        offset: 2,
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
    + PENDING_CHOICE_ROWS;
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
