//! Visibility-filtered physical page rows; private card handles never enter tensors.
use crate::{card_registry::CardRegistry, enums::PlayerId,
    owned_card_choice::{CardChoiceOperation as Op, CardChoiceRowView, CardChoiceView, CardChoiceZone},
    tensor_profiles::standard::v2_lite as layout};

pub(super) fn write_row(t: &mut [f32], base: usize, row: &CardChoiceRowView,
    menu: &CardChoiceView, registry: &CardRegistry, observer: PlayerId) {
    t[base + layout::PENDING_CARD_PRESENT_OFFSET] = 1.0;
    let flag = match row.operation {
        Op::CommitCard => layout::PENDING_CARD_COMMIT_OFFSET,
        Op::InspectCard => layout::PENDING_CARD_INSPECT_OFFSET,
        Op::PreviousPage => layout::PENDING_CARD_PREVIOUS_OFFSET,
        Op::NextPage | Op::RefreshPage => layout::PENDING_CARD_NEXT_OFFSET,
        Op::Decline | Op::FinishImpossible => layout::PENDING_CARD_DECLINE_OFFSET,
    };
    t[base + flag] = 1.0;
    t[base + layout::PENDING_CARD_ZONE_OFFSET] = match row.zone {
        Some(CardChoiceZone::BattleSource) => 1.0,
        Some(CardChoiceZone::BreedingSource) => 2.0,
        Some(CardChoiceZone::Security) => 3.0,
        Some(CardChoiceZone::Hand) => 4.0,
        Some(CardChoiceZone::Trash) => 5.0,
        Some(CardChoiceZone::BattleTop) => 6.0,
        None => 0.0,
    };
    if let Some(owner) = row.owner {
        t[base + layout::PENDING_CARD_OWNER_OFFSET] = if owner == observer { 1.0 } else { -1.0 };
    }
    if let Some(slot) = row.carrier_slot {
        t[base + layout::PENDING_CARD_CARRIER_SLOT_OFFSET] = f32::from(slot);
        t[base + layout::PENDING_CARD_CARRIER_PRESENT_OFFSET] = 1.0;
    }
    if row.identity_visible {
        t[base + layout::PENDING_CARD_IDENTITY_VISIBLE_OFFSET] = 1.0;
        if let Some(id) = &row.card_id {
            t[base + layout::PENDING_CARD_ID_OFFSET] = f32::from(registry.get_index(id));
        }
    }
    for (offset, decimal) in [
        (layout::PENDING_CARD_PROMPT_ID_OFFSET, Some(menu.prompt_id.as_str())),
        (layout::PENDING_CARD_GENERATION_OFFSET, Some(menu.page_generation.as_str())),
        (layout::PENDING_CARD_PAGE_INDEX_OFFSET, Some(menu.page_index.as_str())),
        (layout::PENDING_CARD_PAGE_COUNT_OFFSET, Some(menu.page_count.as_str())),
        (layout::PENDING_CARD_POSITION_OFFSET, row.position.as_deref()),
        (layout::PENDING_CARD_CARRIER_ID_OFFSET, row.carrier_identity.as_deref()),
    ] {
        if let Some(value) = decimal.and_then(|s| s.parse::<u64>().ok()) {
            t[base + offset..base + offset + 4].copy_from_slice(&crate::count_choice::encode_count_u64(value));
        }
    }
}
