//! Focused new selector regressions plus the existing zone-movement controls.
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

#[path = "dsl/constructs/mixed_zone_distinct.rs"]
mod constructs;
#[path = "dsl/constructs/zone_moves.rs"]
mod existing_zone_moves;

// The included canonical zone-movement module uses the same physical fixture
// resolver as tests/dsl/main.rs; old coordinates never reach engine resolution.
fn union_fixture_rows(game: &digimon_engine::game::Game) -> Option<Vec<(u16, u16)>> {
    use digimon_engine::{action::space::*, owned_card_choice::{CardChoiceOperation, CardChoiceZone}};
    let pending = game.pending_selection.as_ref()?;
    if !matches!(pending.kind, digimon_engine::selection::SelectionKind::UnionZone { .. }) { return None; }
    let owner = pending.card_choice.as_ref()?;
    Some(owner.view(game, pending.selecting_player).rows.into_iter().filter_map(|row| {
        if !matches!(row.operation, CardChoiceOperation::CommitCard) { return None; }
        let position: u16 = row.position.as_deref()?.parse().ok()?;
        let fixture = match row.zone? {
            CardChoiceZone::Hand => PLAY_HAND_START.checked_add(position)?,
            CardChoiceZone::Trash => TRASH_EFFECT_START.checked_add(position)?,
            CardChoiceZone::BattleSource => encode_source_select(u16::from(row.carrier_slot?), position)?,
            CardChoiceZone::BreedingSource => encode_breeding_source_select(row.owner?, position)?,
            _ => return None,
        };
        Some((fixture, row.action_id))
    }).collect())
}
fn union_fixture_action(game: &digimon_engine::game::Game, fixture: u16) -> u16 {
    let Some(rows) = union_fixture_rows(game) else { return fixture; };
    let action = if fixture == digimon_engine::action::space::PASS { fixture }
        else { rows.into_iter().find(|(old, _)| *old == fixture).expect("fixture card must be in typed menu").1 };
    let actor = game.pending_selection.as_ref().unwrap().selecting_player;
    assert_eq!(digimon_engine::action::build_action_mask(game, actor)[action as usize], 1.0);
    action
}
