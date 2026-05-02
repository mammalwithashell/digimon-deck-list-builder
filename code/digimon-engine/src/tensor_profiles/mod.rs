//! Registry metadata for observation tensor layouts.

pub mod standard;

pub const STANDARD_COMPACT_V1_PROFILE_ID: &str = standard::v1::PROFILE_ID;
pub const STANDARD_LITE_V2_PROFILE_ID: &str = standard::v2_lite::PROFILE_ID;
pub const STANDARD_FULL_V2_PROFILE_ID: &str = standard::v2_full::PROFILE_ID;
pub const STANDARD_V1_LEGACY_PROFILE_ID: &str = "standard_v1";
pub const COMPACT_V1_LEGACY_PROFILE_ID: &str = "compact_v1";
pub const V2_LITE_TRANSITION_PROFILE_ID: &str = "v2_lite";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TensorSectionKind {
    Scalars,
    CardIds,
    PermanentSlots,
    StandardLiteV2Rows,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TensorSection {
    pub id: &'static str,
    pub start: usize,
    pub len: usize,
    pub shape: &'static [usize],
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
    pub tensor_version: u16,
    pub feature_schema_version: &'static str,
    pub layout_hash: &'static str,
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
                TensorSectionKind::StandardLiteV2Rows => {
                    standard::v2_lite::positions_for_section(
                        section,
                        &mut card_positions,
                        &mut scalar_positions,
                    );
                }
            }
        }

        card_positions.sort();
        scalar_positions.sort();
        (card_positions, scalar_positions)
    }

    pub fn layout_hash_with_schema_version_for_test(&self, schema_version: &str) -> String {
        let (card_id_positions, scalar_positions) = self.positions();
        compute_layout_hash(
            self.id,
            self.tensor_version,
            schema_version,
            self.tensor_size,
            self.sections,
            &card_id_positions,
            &scalar_positions,
        )
    }
}

pub fn compute_layout_hash(
    profile_id: &str,
    tensor_version: u16,
    feature_schema_version: &str,
    tensor_size: usize,
    sections: &[TensorSection],
    card_id_positions: &[usize],
    scalar_positions: &[usize],
) -> String {
    use sha2::{Digest, Sha256};

    let mut canonical = String::new();
    canonical.push_str(profile_id);
    canonical.push('|');
    canonical.push_str(&tensor_version.to_string());
    canonical.push('|');
    canonical.push_str(feature_schema_version);
    canonical.push('|');
    canonical.push_str(&tensor_size.to_string());
    for section in sections {
        canonical.push('|');
        canonical.push_str(section.id);
        canonical.push(':');
        canonical.push_str(&section.start.to_string());
        canonical.push(':');
        canonical.push_str(&section.len.to_string());
        canonical.push(':');
        push_usize_slice(&mut canonical, section.shape);
        canonical.push(':');
        canonical.push_str(section_kind_label(section.kind));
    }
    canonical.push('|');
    push_usize_slice(&mut canonical, card_id_positions);
    canonical.push('|');
    push_usize_slice(&mut canonical, scalar_positions);

    let digest = Sha256::digest(canonical.as_bytes());
    format!("sha256:{digest:x}")
}

fn section_kind_label(kind: TensorSectionKind) -> &'static str {
    match kind {
        TensorSectionKind::Scalars => "scalars",
        TensorSectionKind::CardIds => "card_ids",
        TensorSectionKind::PermanentSlots => "permanent_slots",
        TensorSectionKind::StandardLiteV2Rows => "standard_lite_v2_rows",
    }
}

fn push_usize_slice(output: &mut String, values: &[usize]) {
    output.push('[');
    for (index, value) in values.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        output.push_str(&value.to_string());
    }
    output.push(']');
}

pub fn default_profile() -> TensorProfile {
    standard::DEFAULT_PROFILE
}

pub fn all_profile_ids() -> Vec<&'static str> {
    vec![
        standard::v1::PROFILE_ID,
        standard::v2_lite::PROFILE_ID,
        standard::v2_full::PROFILE_ID,
    ]
}

pub fn profile_by_id(id: &str) -> Option<TensorProfile> {
    match id {
        standard::v1::PROFILE_ID | STANDARD_V1_LEGACY_PROFILE_ID | COMPACT_V1_LEGACY_PROFILE_ID => {
            Some(standard::v1::PROFILE)
        }
        standard::v2_lite::PROFILE_ID | V2_LITE_TRANSITION_PROFILE_ID => {
            Some(standard::v2_lite::PROFILE)
        }
        standard::v2_full::PROFILE_ID => Some(standard::v2_full::PROFILE),
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
