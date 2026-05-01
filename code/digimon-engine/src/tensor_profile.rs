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
pub struct TensorProfile {
    pub id: &'static str,
    pub version: u32,
    pub tensor_size: usize,
    pub field_slots: usize,
    pub slot_size: usize,
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
                    for slot_base in (section.start..section.start + section.len).step_by(SLOT_SIZE)
                    {
                        permanent_slot_positions(
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
    card_id_slot_count: CARD_ID_SLOT_COUNT,
    scalar_slot_count: SCALAR_SLOT_COUNT,
    sections: STANDARD_V1_SECTIONS,
};

const PERMANENT_SLOT_SCALAR_OFFSETS: &[usize] = &[
    SLOT_DP_OFFSET,
    SLOT_SUSPENDED_OFFSET,
    SLOT_OPT_TOTAL_OFFSET,
    SLOT_OPT_USED_OFFSET,
    SLOT_LINKED_COUNT_OFFSET,
    SLOT_SOURCE_COUNT_OFFSET,
];

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
    slot_base: usize,
    card_positions: &mut Vec<usize>,
    scalar_positions: &mut Vec<usize>,
) {
    card_positions.push(slot_base + SLOT_TOP_CARD_OFFSET);
    scalar_positions.extend(
        PERMANENT_SLOT_SCALAR_OFFSETS
            .iter()
            .map(|offset| slot_base + offset),
    );

    let source_base = slot_base + SLOT_SOURCE_START_OFFSET;
    for source_index in 0..MAX_SOURCES {
        let source_offset = source_base + source_index * SOURCE_ENTRY_SIZE;
        card_positions.push(source_offset + SOURCE_CARD_ID_OFFSET);
        scalar_positions.push(source_offset + SOURCE_OPT_STATE_OFFSET);
        scalar_positions.push(source_offset + SOURCE_DP_CONTRIBUTION_OFFSET);
    }
}
