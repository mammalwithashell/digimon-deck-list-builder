use crate::tensor_profiles::{
    TensorFieldKind, TensorProfile, TensorSection, TensorSectionKind, TensorSlotField,
    TensorSlotLayout,
};

pub const PROFILE_ID: &str = "standard_v1";
pub const GAME_MODE: &str = "standard";
pub const VERSION: u32 = 1;

pub const FIELD_SLOTS: usize = 14;
pub const MAX_HAND: usize = 20;
pub const MAX_TRASH: usize = 45;
pub const MAX_SECURITY: usize = 10;
pub const MAX_SOURCES: usize = 11;
pub const MAX_REVEALED: usize = 10;

pub const SOURCE_ENTRY_SIZE: usize = 3;
pub const SLOT_TOP_CARD_OFFSET: usize = 0;
pub const SLOT_DP_OFFSET: usize = 1;
pub const SLOT_SUSPENDED_OFFSET: usize = 2;
pub const SLOT_OPT_TOTAL_OFFSET: usize = 3;
pub const SLOT_OPT_USED_OFFSET: usize = 4;
pub const SLOT_LINKED_COUNT_OFFSET: usize = 5;
pub const SLOT_SOURCE_COUNT_OFFSET: usize = 6;
pub const SLOT_SOURCE_START_OFFSET: usize = 7;
pub const SLOT_HEADER_SIZE: usize = SLOT_SOURCE_START_OFFSET;
pub const SLOT_SIZE: usize = SLOT_HEADER_SIZE + MAX_SOURCES * SOURCE_ENTRY_SIZE;
pub const SOURCE_CARD_ID_OFFSET: usize = 0;
pub const SOURCE_OPT_STATE_OFFSET: usize = 1;
pub const SOURCE_DP_CONTRIBUTION_OFFSET: usize = 2;

pub const GLOBAL_SIZE: usize = 10;
pub const BATTLE_SIZE: usize = FIELD_SLOTS * SLOT_SIZE;
pub const HAND_SIZE: usize = MAX_HAND;
pub const TRASH_SIZE: usize = MAX_TRASH;
pub const SECURITY_SIZE: usize = MAX_SECURITY;
pub const BREEDING_SIZE: usize = SLOT_SIZE;
pub const REVEALED_SIZE: usize = MAX_REVEALED;
pub const SELECTION_SIZE: usize = 5;

pub const OFF_GLOBAL: usize = 0;
pub const OFF_MY_BATTLE: usize = OFF_GLOBAL + GLOBAL_SIZE;
pub const OFF_OPP_BATTLE: usize = OFF_MY_BATTLE + BATTLE_SIZE;
pub const OFF_MY_HAND: usize = OFF_OPP_BATTLE + BATTLE_SIZE;
pub const OFF_OPP_HAND: usize = OFF_MY_HAND + HAND_SIZE;
pub const OFF_MY_TRASH: usize = OFF_OPP_HAND + HAND_SIZE;
pub const OFF_OPP_TRASH: usize = OFF_MY_TRASH + TRASH_SIZE;
pub const OFF_MY_SECURITY: usize = OFF_OPP_TRASH + TRASH_SIZE;
pub const OFF_OPP_SECURITY: usize = OFF_MY_SECURITY + SECURITY_SIZE;
pub const OFF_MY_BREEDING: usize = OFF_OPP_SECURITY + SECURITY_SIZE;
pub const OFF_OPP_BREEDING: usize = OFF_MY_BREEDING + BREEDING_SIZE;
pub const OFF_REVEALED: usize = OFF_OPP_BREEDING + BREEDING_SIZE;
pub const OFF_SELECTION: usize = OFF_REVEALED + REVEALED_SIZE;

pub const TENSOR_SIZE: usize = OFF_SELECTION + SELECTION_SIZE;

pub const SECTIONS: &[TensorSection] = &[
    TensorSection {
        id: "global",
        start: OFF_GLOBAL,
        len: GLOBAL_SIZE,
        kind: TensorSectionKind::Scalars,
    },
    TensorSection {
        id: "my_battle",
        start: OFF_MY_BATTLE,
        len: BATTLE_SIZE,
        kind: TensorSectionKind::PermanentSlots,
    },
    TensorSection {
        id: "opponent_battle",
        start: OFF_OPP_BATTLE,
        len: BATTLE_SIZE,
        kind: TensorSectionKind::PermanentSlots,
    },
    TensorSection {
        id: "my_hand",
        start: OFF_MY_HAND,
        len: HAND_SIZE,
        kind: TensorSectionKind::CardIds,
    },
    TensorSection {
        id: "opponent_hand",
        start: OFF_OPP_HAND,
        len: HAND_SIZE,
        kind: TensorSectionKind::CardIds,
    },
    TensorSection {
        id: "my_trash",
        start: OFF_MY_TRASH,
        len: TRASH_SIZE,
        kind: TensorSectionKind::CardIds,
    },
    TensorSection {
        id: "opponent_trash",
        start: OFF_OPP_TRASH,
        len: TRASH_SIZE,
        kind: TensorSectionKind::CardIds,
    },
    TensorSection {
        id: "my_security",
        start: OFF_MY_SECURITY,
        len: SECURITY_SIZE,
        kind: TensorSectionKind::CardIds,
    },
    TensorSection {
        id: "opponent_security",
        start: OFF_OPP_SECURITY,
        len: SECURITY_SIZE,
        kind: TensorSectionKind::CardIds,
    },
    TensorSection {
        id: "my_breeding",
        start: OFF_MY_BREEDING,
        len: BREEDING_SIZE,
        kind: TensorSectionKind::PermanentSlots,
    },
    TensorSection {
        id: "opponent_breeding",
        start: OFF_OPP_BREEDING,
        len: BREEDING_SIZE,
        kind: TensorSectionKind::PermanentSlots,
    },
    TensorSection {
        id: "revealed",
        start: OFF_REVEALED,
        len: REVEALED_SIZE,
        kind: TensorSectionKind::CardIds,
    },
    TensorSection {
        id: "selection",
        start: OFF_SELECTION,
        len: SELECTION_SIZE,
        kind: TensorSectionKind::Scalars,
    },
];

pub const SLOT_HEADER_FIELDS: &[TensorSlotField] = &[
    TensorSlotField {
        id: "top_card_id",
        offset: SLOT_TOP_CARD_OFFSET,
        kind: TensorFieldKind::CardId,
    },
    TensorSlotField {
        id: "dp",
        offset: SLOT_DP_OFFSET,
        kind: TensorFieldKind::Scalar,
    },
    TensorSlotField {
        id: "suspended",
        offset: SLOT_SUSPENDED_OFFSET,
        kind: TensorFieldKind::Scalar,
    },
    TensorSlotField {
        id: "opt_total",
        offset: SLOT_OPT_TOTAL_OFFSET,
        kind: TensorFieldKind::Scalar,
    },
    TensorSlotField {
        id: "opt_used",
        offset: SLOT_OPT_USED_OFFSET,
        kind: TensorFieldKind::Scalar,
    },
    TensorSlotField {
        id: "linked_count",
        offset: SLOT_LINKED_COUNT_OFFSET,
        kind: TensorFieldKind::Scalar,
    },
    TensorSlotField {
        id: "source_count",
        offset: SLOT_SOURCE_COUNT_OFFSET,
        kind: TensorFieldKind::Scalar,
    },
];

pub const SOURCE_FIELDS: &[TensorSlotField] = &[
    TensorSlotField {
        id: "card_id",
        offset: SOURCE_CARD_ID_OFFSET,
        kind: TensorFieldKind::CardId,
    },
    TensorSlotField {
        id: "opt_state",
        offset: SOURCE_OPT_STATE_OFFSET,
        kind: TensorFieldKind::Scalar,
    },
    TensorSlotField {
        id: "dp_contribution",
        offset: SOURCE_DP_CONTRIBUTION_OFFSET,
        kind: TensorFieldKind::Scalar,
    },
];

pub const SLOT_LAYOUT: TensorSlotLayout = TensorSlotLayout {
    size: SLOT_SIZE,
    source_start: SLOT_SOURCE_START_OFFSET,
    source_entry_size: SOURCE_ENTRY_SIZE,
    max_sources: MAX_SOURCES,
    header_fields: SLOT_HEADER_FIELDS,
    source_fields: SOURCE_FIELDS,
};

pub const PERMANENT_SLOT_CARD_ID_COUNT: usize = 1 + MAX_SOURCES;
pub const PERMANENT_SLOT_SCALAR_COUNT: usize =
    SLOT_HEADER_SIZE - 1 + MAX_SOURCES * (SOURCE_ENTRY_SIZE - 1);
pub const PERMANENT_SLOT_COUNT: usize = FIELD_SLOTS * 2 + 2;
pub const CARD_ID_SLOT_COUNT: usize = PERMANENT_SLOT_COUNT * PERMANENT_SLOT_CARD_ID_COUNT
    + HAND_SIZE * 2
    + TRASH_SIZE * 2
    + SECURITY_SIZE * 2
    + REVEALED_SIZE;
pub const SCALAR_SLOT_COUNT: usize =
    PERMANENT_SLOT_COUNT * PERMANENT_SLOT_SCALAR_COUNT + GLOBAL_SIZE + SELECTION_SIZE;

pub const PROFILE: TensorProfile = TensorProfile {
    id: PROFILE_ID,
    game_mode: GAME_MODE,
    version: VERSION,
    tensor_size: TENSOR_SIZE,
    field_slots: FIELD_SLOTS,
    slot_size: SLOT_SIZE,
    max_sources: MAX_SOURCES,
    slot_layout: SLOT_LAYOUT,
    card_id_slot_count: CARD_ID_SLOT_COUNT,
    scalar_slot_count: SCALAR_SLOT_COUNT,
    sections: SECTIONS,
};
