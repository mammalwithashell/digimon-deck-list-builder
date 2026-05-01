//! Registry metadata for observation tensor layouts.

pub mod standard;

pub const STANDARD_V1_PROFILE_ID: &str = standard::v1::PROFILE_ID;

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
    pub source_entry_size: usize,
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
    pub game_mode: &'static str,
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

pub fn default_profile() -> TensorProfile {
    standard::DEFAULT_PROFILE
}

pub fn all_profile_ids() -> Vec<&'static str> {
    vec![standard::v1::PROFILE_ID]
}

pub fn profile_by_id(id: &str) -> Option<TensorProfile> {
    match id {
        standard::v1::PROFILE_ID => Some(standard::v1::PROFILE),
        _ => None,
    }
}

pub fn standard_v1_positions() -> (Vec<usize>, Vec<usize>) {
    standard::v1::PROFILE.positions()
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
        let source_offset = source_base + source_index * layout.source_entry_size;
        for field in layout.source_fields {
            match field.kind {
                TensorFieldKind::CardId => card_positions.push(source_offset + field.offset),
                TensorFieldKind::Scalar => scalar_positions.push(source_offset + field.offset),
            }
        }
    }
}
