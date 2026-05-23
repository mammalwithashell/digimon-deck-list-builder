use crate::tensor_profiles::{TensorProfile, TensorSection, TensorSectionKind};

use super::v2_lite;

pub const PROFILE_ID: &str = "standard_lite_deck_v2";
pub const GAME_MODE: &str = v2_lite::GAME_MODE;
pub const VERSION: u32 = v2_lite::VERSION;
pub const TENSOR_VERSION: u16 = v2_lite::TENSOR_VERSION;
pub const FEATURE_SCHEMA_VERSION: &str = "standard_lite_deck_v2.1";
pub const LAYOUT_HASH: &str =
    "sha256:a20462fbfede51ad3b1585291ea1ec1259b1a6b9c6156cff033db5f9f1ead39e";

pub const DECKLIST_ROWS: usize = 55;
pub const DECKLIST_ROW_SIZE: usize = 8;
pub const DECKLIST_SIZE: usize = DECKLIST_ROWS * DECKLIST_ROW_SIZE;
pub const DECKLIST_CARD_ID_OFFSET: usize = 1;

pub const OFF_GLOBAL_FEATURES: usize = v2_lite::OFF_GLOBAL_FEATURES;
pub const OFF_PLAYER_SUMMARY: usize = v2_lite::OFF_PLAYER_SUMMARY;
pub const OFF_PERMANENT_SLOTS: usize = v2_lite::OFF_PERMANENT_SLOTS;
pub const OFF_OWN_HAND: usize = v2_lite::OFF_OWN_HAND;
pub const OFF_KNOWN_ZONE_CARDS: usize = v2_lite::OFF_KNOWN_ZONE_CARDS;
pub const OFF_DECISION_CONTEXT: usize = v2_lite::OFF_DECISION_CONTEXT;
pub const OFF_PENDING_CHOICE_FEATURES: usize = v2_lite::OFF_PENDING_CHOICE_FEATURES;
pub const OFF_OWN_ORIGINAL_DECKLIST: usize = v2_lite::OFF_RESERVED;
pub const OFF_RESERVED: usize = OFF_OWN_ORIGINAL_DECKLIST + DECKLIST_SIZE;
pub const TENSOR_SIZE: usize = OFF_RESERVED + v2_lite::RESERVED_SIZE;

pub const SHAPE_GLOBAL_FEATURES: &[usize] = v2_lite::SHAPE_GLOBAL_FEATURES;
pub const SHAPE_PLAYER_SUMMARY: &[usize] = v2_lite::SHAPE_PLAYER_SUMMARY;
pub const SHAPE_PERMANENT_SLOTS: &[usize] = v2_lite::SHAPE_PERMANENT_SLOTS;
pub const SHAPE_OWN_HAND: &[usize] = v2_lite::SHAPE_OWN_HAND;
pub const SHAPE_KNOWN_ZONE_CARDS: &[usize] = v2_lite::SHAPE_KNOWN_ZONE_CARDS;
pub const SHAPE_DECISION_CONTEXT: &[usize] = v2_lite::SHAPE_DECISION_CONTEXT;
pub const SHAPE_PENDING_CHOICE_FEATURES: &[usize] = v2_lite::SHAPE_PENDING_CHOICE_FEATURES;
pub const SHAPE_OWN_ORIGINAL_DECKLIST: &[usize] = &[DECKLIST_ROWS, DECKLIST_ROW_SIZE];
pub const SHAPE_RESERVED: &[usize] = v2_lite::SHAPE_RESERVED;

pub const SECTIONS: &[TensorSection] = &[
    TensorSection {
        id: "global_features",
        start: OFF_GLOBAL_FEATURES,
        len: v2_lite::GLOBAL_FEATURES_SIZE,
        shape: SHAPE_GLOBAL_FEATURES,
        kind: TensorSectionKind::Scalars,
    },
    TensorSection {
        id: "player_summary",
        start: OFF_PLAYER_SUMMARY,
        len: v2_lite::PLAYER_SUMMARY_SIZE,
        shape: SHAPE_PLAYER_SUMMARY,
        kind: TensorSectionKind::Scalars,
    },
    TensorSection {
        id: "permanent_slots",
        start: OFF_PERMANENT_SLOTS,
        len: v2_lite::PERMANENT_SLOTS_SIZE,
        shape: SHAPE_PERMANENT_SLOTS,
        kind: TensorSectionKind::StandardLiteV2Rows,
    },
    TensorSection {
        id: "own_hand",
        start: OFF_OWN_HAND,
        len: v2_lite::OWN_HAND_SIZE,
        shape: SHAPE_OWN_HAND,
        kind: TensorSectionKind::StandardLiteV2Rows,
    },
    TensorSection {
        id: "known_zone_cards",
        start: OFF_KNOWN_ZONE_CARDS,
        len: v2_lite::KNOWN_ZONE_SIZE,
        shape: SHAPE_KNOWN_ZONE_CARDS,
        kind: TensorSectionKind::StandardLiteV2Rows,
    },
    TensorSection {
        id: "decision_context",
        start: OFF_DECISION_CONTEXT,
        len: v2_lite::DECISION_CONTEXT_SIZE,
        shape: SHAPE_DECISION_CONTEXT,
        kind: TensorSectionKind::Scalars,
    },
    TensorSection {
        id: "pending_choice_features",
        start: OFF_PENDING_CHOICE_FEATURES,
        len: v2_lite::PENDING_CHOICE_SIZE,
        shape: SHAPE_PENDING_CHOICE_FEATURES,
        kind: TensorSectionKind::StandardLiteV2Rows,
    },
    TensorSection {
        id: "own_original_decklist",
        start: OFF_OWN_ORIGINAL_DECKLIST,
        len: DECKLIST_SIZE,
        shape: SHAPE_OWN_ORIGINAL_DECKLIST,
        kind: TensorSectionKind::StandardLiteV2Rows,
    },
    TensorSection {
        id: "reserved",
        start: OFF_RESERVED,
        len: v2_lite::RESERVED_SIZE,
        shape: SHAPE_RESERVED,
        kind: TensorSectionKind::Scalars,
    },
];

pub const CARD_ID_SLOT_COUNT: usize = v2_lite::CARD_ID_SLOT_COUNT + DECKLIST_ROWS;
pub const SCALAR_SLOT_COUNT: usize = TENSOR_SIZE - CARD_ID_SLOT_COUNT;

pub const PROFILE: TensorProfile = TensorProfile {
    id: PROFILE_ID,
    game_mode: GAME_MODE,
    version: VERSION,
    tensor_version: TENSOR_VERSION,
    feature_schema_version: FEATURE_SCHEMA_VERSION,
    layout_hash: LAYOUT_HASH,
    tensor_size: TENSOR_SIZE,
    field_slots: v2_lite::PERMANENT_SLOTS_PER_PLAYER,
    slot_size: v2_lite::PERMANENT_SLOT_SIZE,
    max_sources: v2_lite::PERM_MAX_SOURCES,
    slot_layout: v2_lite::SLOT_LAYOUT,
    card_id_slot_count: CARD_ID_SLOT_COUNT,
    scalar_slot_count: SCALAR_SLOT_COUNT,
    sections: SECTIONS,
};
