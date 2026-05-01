//! Registry metadata for observation tensor layouts.

use crate::tensor::{FIELD_SLOTS, SLOT_SIZE, TENSOR_SIZE};

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
        start: 0,
        len: 10,
        kind: TensorSectionKind::Scalars,
    },
    TensorSection {
        id: "my_battle",
        start: 10,
        len: 560,
        kind: TensorSectionKind::PermanentSlots,
    },
    TensorSection {
        id: "opponent_battle",
        start: 570,
        len: 560,
        kind: TensorSectionKind::PermanentSlots,
    },
    TensorSection {
        id: "my_hand",
        start: 1130,
        len: 20,
        kind: TensorSectionKind::CardIds,
    },
    TensorSection {
        id: "opponent_hand",
        start: 1150,
        len: 20,
        kind: TensorSectionKind::CardIds,
    },
    TensorSection {
        id: "my_trash",
        start: 1170,
        len: 45,
        kind: TensorSectionKind::CardIds,
    },
    TensorSection {
        id: "opponent_trash",
        start: 1215,
        len: 45,
        kind: TensorSectionKind::CardIds,
    },
    TensorSection {
        id: "my_security",
        start: 1260,
        len: 10,
        kind: TensorSectionKind::CardIds,
    },
    TensorSection {
        id: "opponent_security",
        start: 1270,
        len: 10,
        kind: TensorSectionKind::CardIds,
    },
    TensorSection {
        id: "my_breeding",
        start: 1280,
        len: 40,
        kind: TensorSectionKind::PermanentSlots,
    },
    TensorSection {
        id: "opponent_breeding",
        start: 1320,
        len: 40,
        kind: TensorSectionKind::PermanentSlots,
    },
    TensorSection {
        id: "revealed",
        start: 1360,
        len: 10,
        kind: TensorSectionKind::CardIds,
    },
    TensorSection {
        id: "selection",
        start: 1370,
        len: 5,
        kind: TensorSectionKind::Scalars,
    },
];

const STANDARD_V1_PROFILE: TensorProfile = TensorProfile {
    id: STANDARD_V1_PROFILE_ID,
    version: 1,
    tensor_size: TENSOR_SIZE,
    field_slots: FIELD_SLOTS,
    slot_size: SLOT_SIZE,
    card_id_slot_count: 520,
    scalar_slot_count: 855,
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
    slot_base: usize,
    card_positions: &mut Vec<usize>,
    scalar_positions: &mut Vec<usize>,
) {
    card_positions.push(slot_base);
    scalar_positions.extend(slot_base + 1..slot_base + 7);

    let source_base = slot_base + 7;
    for source_index in 0..11 {
        let source_offset = source_base + source_index * 3;
        card_positions.push(source_offset);
        scalar_positions.push(source_offset + 1);
        scalar_positions.push(source_offset + 2);
    }
}
