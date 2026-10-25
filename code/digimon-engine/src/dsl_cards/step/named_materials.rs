//! Named-role selection uses ordinary source actions and a cloneable data frame.
use super::*;
use crate::card_source::CardHandle;
use crate::resume::{MultiPickState, NamedSourceGroups, ResumeProvenance};

/// Maximum bipartite matching while preserving every explicitly selected card.
/// Augmenting paths may reassign a dual-name card's role; that assignment is
/// bookkeeping, not a hidden gameplay choice. Only physical cards are selected.
pub(super) fn maximum_matching(groups: &NamedSourceGroups, required: &[CardHandle]) -> Option<usize> {
    fn augment(card: usize, groups: &NamedSourceGroups, matched: &mut [Option<usize>], seen: &mut [bool]) -> bool {
        for &role in &groups.members[card].1 {
            if seen[role] { continue; }
            seen[role] = true;
            let previous = matched[role];
            if previous.is_none() || augment(previous.unwrap(), groups, matched, seen) {
                matched[role] = Some(card);
                return true;
            }
        }
        false
    }
    let mut matched = vec![None; groups.role_count];
    let mut forced = Vec::new();
    for handle in required {
        let index = groups.members.iter().position(|(h, _)| h == handle)?;
        if forced.contains(&index) { return None; }
        if !augment(index, groups, &mut matched, &mut vec![false; groups.role_count]) { return None; }
        forced.push(index);
    }
    for index in 0..groups.members.len() {
        if !forced.contains(&index) {
            augment(index, groups, &mut matched, &mut vec![false; groups.role_count]);
        }
    }
    Some(matched.iter().filter(|x| x.is_some()).count())
}

#[allow(clippy::too_many_arguments)]
pub(super) fn install(
    ctx: &mut EffectContext<'_>, perm: PermanentHandle, min: u8, max: u8,
    filter: &CompiledPredicate, names: &[String], require_max: bool,
    bind_as: Option<String>, prompt: String, optional_zero: bool,
    tail: Vec<CompiledStep>, bindings: Bindings, runtime: StepRuntime,
) -> InstallResult {
    use crate::effect_context::selections::{material_zone_geometry, material_zone_slice};
    let candidates = count_capped_card_candidate_indices(
        ctx.game, perm.player, CountCappedZone::Material(perm), filter, &bindings,
        ctx.source_card, ctx.source_permanent, ctx.source_kind, ctx.player, None, 0,
    );
    let groups = groups_for_candidates(ctx.game, perm, &candidates, names);
    let attainable = maximum_matching(&groups, &[]).unwrap_or(0).min(max as usize) as u8;
    if min > attainable { return InstallResult::TailAlreadyRan; }
    let floor = if require_max { attainable } else { min };
    let candidate_indices: Vec<usize> = candidates.into_iter().filter(|&index| {
        let Some(card) = material_zone_slice(ctx.game, perm).and_then(|s| s.get(index)) else { return false; };
        maximum_matching(&groups, &[card.handle()])
            .is_some_and(|n| n >= floor.max(1) as usize)
    }).collect();
    let mut state = MultiPickState {
        prov: ResumeProvenance { source_card: ctx.source_card, source_permanent: ctx.source_permanent,
            source_kind: ctx.source_kind, controller: ctx.player, override_pin: ctx.override_selecting_player() },
        of_player: perm.player,
        selecting_player: ctx.override_selecting_player().unwrap_or(ctx.player),
        previous_phase: ctx.game.current_phase,
        zone: CountCappedZone::Material(perm),
        range_start: material_zone_geometry(ctx.game, perm).map(|(_, range)| range).unwrap_or(0),
        min: floor, max: attainable, is_optional_zero: optional_zero, distinct_by: None,
        source_pool: capture_material_pool(ctx.game, CountCappedZone::Material(perm), &candidate_indices),
        candidate_indices, accum: Vec::new(), named_groups: Some(groups),
        bind_as, inner_tail: Arc::new(tail), bindings, runtime,
        trigger_context: ctx.game.current_trigger_context.clone(), outer_conts: Vec::new(),
    };
    if attainable == 0 {
        state.accum.clear();
        run_multipick_terminal(ctx.game, state);
        return InstallResult::TailAlreadyRan;
    }
    install_multipick_step(ctx.game, state);
    if let Some(selection) = ctx.game.pending_selection.as_mut() { selection.prompt = prompt; }
    InstallResult::Parked
}

pub(super) fn groups_for_candidates(game: &crate::game::Game, perm: PermanentHandle, candidates: &[usize], names: &[String]) -> NamedSourceGroups {
    use crate::effect_context::selections::material_zone_slice;
    let mut groups = NamedSourceGroups { role_count: names.len(), members: Vec::new() };
    if let Some(sources) = material_zone_slice(game, perm) {
        for &index in candidates {
            let Some(card) = sources.get(index) else { continue; };
            let Some(data) = game.card_data.get(card.data_index) else { continue; };
            let roles: Vec<_> = names.iter().enumerate().filter_map(|(role, name)| {
                (data.card_name.eq_ignore_ascii_case(name)
                    || data.also_treated_as.iter().any(|alias| alias.eq_ignore_ascii_case(name))
                    || card.also_treated_as.iter().any(|alias| alias.eq_ignore_ascii_case(name)))
                    .then_some(role)
            }).collect();
            if !roles.is_empty() { groups.members.push((card.handle(), roles)); }
        }
    }
    groups
}
