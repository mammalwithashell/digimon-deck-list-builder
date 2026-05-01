//! Registry metadata for observation tensor layouts.

use crate::tensor::{
    BATTLE_SIZE, BREEDING_SIZE, FIELD_SLOTS, GLOBAL_SIZE, HAND_SIZE, MAX_SOURCES, OFF_GLOBAL,
    OFF_MY_BATTLE, OFF_MY_BREEDING, OFF_MY_HAND, OFF_MY_SECURITY, OFF_MY_TRASH, OFF_OPP_BATTLE,
    OFF_OPP_BREEDING, OFF_OPP_HAND, OFF_OPP_SECURITY, OFF_OPP_TRASH, OFF_REVEALED, OFF_SELECTION,
    REVEALED_SIZE, SECURITY_SIZE, SELECTION_SIZE, SLOT_DP_OFFSET, SLOT_HEADER_SIZE,
    SLOT_LINKED_COUNT_OFFSET, SLOT_OPT_TOTAL_OFFSET, SLOT_OPT_USED_OFFSET, SLOT_SIZE,
    SLOT_SOURCE_COUNT_OFFSET, SLOT_SOURCE_START_OFFSET, SLOT_SUSPENDED_OFFSET,
    SLOT_TOP_CARD_OFFSET, SOURCE_CARD_ID_OFFSET, SOURCE_DP_CONTRIBUTION_OFFSET, SOURCE_ENTRY_SIZE,
    SOURCE_OPT_STATE_OFFSET, TENSOR_SIZE, TRASH_SIZE,
};

pub const STANDARD_V1_PROFILE_ID: &str = "standard_v1";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TensorSectionKind {
    Scalars,
    CardIds,
    PermanentSlots,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TensorSection {
    pub id: &'static str,
    pub start: usize,
    pub len: usize,
    pub kind: TensorSectionKind,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TensorFieldKind {
    CardId,
    Scalar,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TensorSlotField {
    pub id: &'static str,
    pub offset: usize,
    pub kind: TensorFieldKind,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TensorSlotLayout {
    pub size: usize,
    pub source_start: usize,
    pub max_sources: usize,
    pub header_fields: &'static [TensorSlotField],
    pub source_fields: &'static [TensorSlotField],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TensorSlotHeaderField {
    TopCardId,
    Dp,
    Suspended,
    OptTotal,
    OptUsed,
    LinkedCount,
    SourceCount,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TensorSourceField {
    CardId,
    OptState,
    DpContribution,
}

impl TensorSlotLayout {
    pub fn header_offset(&self, field: TensorSlotHeaderField) -> usize {
        self.header_fields[field as usize].offset
    }

    pub fn source_offset(&self, field: TensorSourceField) -> usize {
        self.source_fields[field as usize].offset
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TensorProfile {
    pub id: &'static str,
    pub version: u32,
    pub tensor_size: usize,
    pub field_slots: usize,
    pub slot_size: usize,
    pub max_sources: usize,
    pub slot_layout: TensorSlotLayout,
    pub card_id_slot_count: usize,
    pub scalar_slot_count: usize,
    pub sections: &'static [TensorSection],
}

impl TensorProfile {
    pub fn section(&self, id: &str) -> Option<&'static TensorSection> {
        self.sections.iter().find(|section| section.id == id)
    }

    pub fn positions(&self) -> (Vec<usize>, Vec<usize>) {
        let mut card_positions = Vec::with_capacity(self.card_id_slot_count);
        let mut scalar_positions = Vec::with_capacity(self.scalar_slot_count);

        for section in self.sections {
            match section.kind {
                TensorSectionKind::Scalars => {
                    scalar_positions.extend(section.start..section.start + section.len);
                }
                TensorSectionKind::CardIds => {
                    card_positions.extend(section.start..section.start + section.len);
                }
                TensorSectionKind::PermanentSlots => {
                    for slot_base in
                        (section.start..section.start + section.len).step_by(self.slot_layout.size)
                    {
                        permanent_slot_positions(
                            self.slot_layout,
                            slot_base,
                            &mut card_positions,
                            &mut scalar_positions,
                        );
                    }
                }
            }
        }

        card_positions.sort();
        scalar_positions.sort();
        (card_positions, scalar_positions)
    }
}

const STANDARD_V1_SECTIONS: &[TensorSection] = &[
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

const STANDARD_V1_SLOT_HEADER_FIELDS: &[TensorSlotField] = &[
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

const STANDARD_V1_SOURCE_FIELDS: &[TensorSlotField] = &[
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

const STANDARD_V1_SLOT_LAYOUT: TensorSlotLayout = TensorSlotLayout {
    size: SLOT_SIZE,
    source_start: SLOT_SOURCE_START_OFFSET,
    max_sources: MAX_SOURCES,
    header_fields: STANDARD_V1_SLOT_HEADER_FIELDS,
    source_fields: STANDARD_V1_SOURCE_FIELDS,
};

const PERMANENT_SLOT_CARD_ID_COUNT: usize = 1 + MAX_SOURCES;
const PERMANENT_SLOT_SCALAR_COUNT: usize =
    SLOT_HEADER_SIZE - 1 + MAX_SOURCES * (SOURCE_ENTRY_SIZE - 1);
const PERMANENT_SLOT_COUNT: usize = FIELD_SLOTS * 2 + 2;
const CARD_ID_SLOT_COUNT: usize = PERMANENT_SLOT_COUNT * PERMANENT_SLOT_CARD_ID_COUNT
    + HAND_SIZE * 2
    + TRASH_SIZE * 2
    + SECURITY_SIZE * 2
    + REVEALED_SIZE;
const SCALAR_SLOT_COUNT: usize =
    PERMANENT_SLOT_COUNT * PERMANENT_SLOT_SCALAR_COUNT + GLOBAL_SIZE + SELECTION_SIZE;

const STANDARD_V1_PROFILE: TensorProfile = TensorProfile {
    id: STANDARD_V1_PROFILE_ID,
    version: 1,
    tensor_size: TENSOR_SIZE,
    field_slots: FIELD_SLOTS,
    slot_size: SLOT_SIZE,
    max_sources: MAX_SOURCES,
    slot_layout: STANDARD_V1_SLOT_LAYOUT,
    card_id_slot_count: CARD_ID_SLOT_COUNT,
    scalar_slot_count: SCALAR_SLOT_COUNT,
    sections: STANDARD_V1_SECTIONS,
};

pub fn default_profile() -> TensorProfile {
    STANDARD_V1_PROFILE
}

pub fn all_profile_ids() -> Vec<&'static str> {
    vec![STANDARD_V1_PROFILE_ID]
}

pub fn profile_by_id(id: &str) -> Option<TensorProfile> {
    match id {
        STANDARD_V1_PROFILE_ID => Some(STANDARD_V1_PROFILE),
        _ => None,
    }
}

pub fn standard_v1_positions() -> (Vec<usize>, Vec<usize>) {
    STANDARD_V1_PROFILE.positions()
}

fn permanent_slot_positions(
    layout: TensorSlotLayout,
    slot_base: usize,
    card_positions: &mut Vec<usize>,
    scalar_positions: &mut Vec<usize>,
) {
    for field in layout.header_fields {
        match field.kind {
            TensorFieldKind::CardId => card_positions.push(slot_base + field.offset),
            TensorFieldKind::Scalar => scalar_positions.push(slot_base + field.offset),
        }
    }

    let source_base = slot_base + layout.source_start;
    for source_index in 0..layout.max_sources {
        let source_offset = source_base + source_index * SOURCE_ENTRY_SIZE;
        for field in layout.source_fields {
            match field.kind {
                TensorFieldKind::CardId => card_positions.push(source_offset + field.offset),
                TensorFieldKind::Scalar => scalar_positions.push(source_offset + field.offset),
            }
        }
    }
}
