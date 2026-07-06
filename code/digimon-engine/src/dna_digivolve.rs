//! DNA digivolve validation — port of Python's
//! `digimon_gym/engine/validation/digivolve_validator.py::can_dna_digivolve`
//! and `has_valid_dna_targets`.
//!
//! A hand card with one or more `DnaCost` entries is a "DNA digivolve"
//! candidate. Each entry carries two `DnaRequirement`s; the evolution is
//! legal if some ordered pair of battle-area permanents satisfies one
//! entry in either direction (requirement1/requirement2 or swapped).
//!
//! Data population (`dna_costs` in cards.json) is §4.5b and is still
//! deferred — these helpers are inert until the ingest pipeline emits
//! DNA costs. They're covered by the mask tests which hand-build
//! `CardData` with `dna_costs` populated.

use crate::card_data::{CardData, DnaCost, DnaRequirement};
use crate::card_source::CardSource;
use crate::digixros::matches_digixros_name_requirement;
use crate::dsl_cards::formula_eval;
use crate::dsl_cards::predicate::{eval_predicate, PredicateSubject};
use crate::effect_context::EffectReadContext;
use crate::enums::{CardColor, CardKind, EffectTiming, GamePhase, ModifierType, PlayerId};
use crate::game::Game;
use crate::permanent::Permanent;
use crate::permanent::PermanentHandle;
use digimon_dsl::compiled::{
    CompiledAltPath, CompiledAltPathKind, CompiledCost, CompiledMaterial, CompiledZone,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DnaRouteWindow {
    Main,
    EndOfTurnAction,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DnaRouteMatch {
    pub first_is_top: bool,
    pub memory_cost: i16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct DigivolveRouteMatch {
    pub memory_cost: u16,
    /// True when this route is an **App Fusion** alt-play (Appmon
    /// "App Fusion" mechanic). Unlike a normal/alt digivolve, App Fusion
    /// (a) ignores printed evo-cost / level / colour requirements (it is
    /// gated only by the host having ≥2 of the named cards across its
    /// top card + linked cards), and (b) after stacking the App-Fusion
    /// card on top, the host's existing **linked cards** are moved under
    /// the new top as digivolution sources (consumed). The commit path
    /// (`commit_digivolve_from_hand_no_replace`) inspects this flag to run
    /// the linked-card consumption step. See `general_rule.pdf` §App
    /// Fusion and DCGO `AddAppfusionMethod.cs` / `CardController.cs`.
    pub app_fusion: bool,
}

impl DigivolveRouteMatch {
    /// A normal / non-App-Fusion digivolve route (printed or alt-digivolve).
    pub fn digivolve(memory_cost: u16) -> Self {
        Self {
            memory_cost,
            app_fusion: false,
        }
    }

    /// An App-Fusion alt-play route.
    pub fn app_fusion(memory_cost: u16) -> Self {
        Self {
            memory_cost,
            app_fusion: true,
        }
    }
}

impl Game {
    pub fn card_data_by_id(&self, card_id: &str) -> Option<&CardData> {
        self.card_id_index
            .get(card_id)
            .and_then(|&idx| self.card_data.get(idx))
    }

    pub fn card_can_satisfy_digixros_name(&self, card_id: &str, required_name: &str) -> bool {
        let Some(data) = self.card_data_by_id(card_id) else {
            return false;
        };
        matches_digixros_name_requirement(data, required_name)
    }

    pub fn permanent_can_satisfy_digixros_name(
        &self,
        handle: PermanentHandle,
        required_name: &str,
    ) -> bool {
        let Some(player) = self.players.get(handle.player as usize) else {
            return false;
        };
        let Some(perm) = player.battle_area.get(handle.index as usize) else {
            return false;
        };
        let top = perm.top_card();
        let data = &self.card_data[top.data_index];
        matches_digixros_name_requirement(data, required_name)
            || self
                .modifiers
                .get(handle, ModifierType::ChangeCardNamesForDigiXros)
                .into_iter()
                .any(|entry| match &entry.payload {
                    crate::modifiers::ModifierPayload::DigiXrosNames { aliases } => aliases
                        .iter()
                        .any(|alias| alias.to_lowercase().contains(&required_name.to_lowercase())),
                    _ => false,
                })
    }

    pub fn card_matches_generic_name(&self, card_id: &str, required_name: &str) -> bool {
        self.card_data_by_id(card_id)
            .map(|data| data.card_name.eq_ignore_ascii_case(required_name))
            .unwrap_or(false)
    }

    pub fn has_valid_dna_route_for_hand_card(&self, player: PlayerId, hand_index: usize) -> bool {
        let Some(window) = self.current_dna_route_window() else {
            return false;
        };
        self.valid_dna_first_targets_for_hand_card(player, hand_index, window)
            .next()
            .is_some()
    }

    pub(crate) fn has_valid_blast_dna_route_for_hand_card(
        &self,
        player: PlayerId,
        hand_index: usize,
    ) -> bool {
        self.valid_blast_dna_field_targets_for_hand_card(player, hand_index)
            .next()
            .is_some()
    }

    pub(crate) fn hand_card_has_registered_blast_dna_paths(
        &self,
        player: PlayerId,
        hand_index: usize,
    ) -> bool {
        let Some(result) = self
            .players
            .get(player as usize)
            .and_then(|state| state.hand.get(hand_index))
        else {
            return false;
        };
        self.has_registered_blast_dna_paths(result)
    }

    pub fn has_registered_end_of_turn_dna_action(&self, player: PlayerId) -> bool {
        let hand_len = self
            .players
            .get(player as usize)
            .map(|p| p.hand.len())
            .unwrap_or(0);
        (0..hand_len).any(|hand_index| {
            self.valid_dna_first_targets_for_hand_card(
                player,
                hand_index,
                DnaRouteWindow::EndOfTurnAction,
            )
            .next()
            .is_some()
        })
    }

    pub(crate) fn current_dna_route_window(&self) -> Option<DnaRouteWindow> {
        match self.current_phase {
            GamePhase::Main => Some(DnaRouteWindow::Main),
            GamePhase::EndOfTurnAction => Some(DnaRouteWindow::EndOfTurnAction),
            _ => None,
        }
    }

    pub(crate) fn normal_digivolve_route_for_hand_card(
        &self,
        player: PlayerId,
        hand_index: usize,
        base_handle: PermanentHandle,
    ) -> Option<DigivolveRouteMatch> {
        if base_handle.player != player {
            return None;
        }
        let player_state = self.players.get(player as usize)?;
        let card = player_state.hand.get(hand_index)?;
        self.normal_digivolve_route_for_card(card, base_handle)
    }

    pub(crate) fn normal_digivolve_route_for_card(
        &self,
        card: &CardSource,
        base_handle: PermanentHandle,
    ) -> Option<DigivolveRouteMatch> {
        // The cheapest applicable route. The mask, the Blast counter path, and
        // the digivolve-execution validity check all consult this (they only
        // need "is there a route, and the floor cost"). The *interactive*
        // player-action path (`digivolve_from_hand_inner`) instead consults
        // `all_digivolve_routes_for_card` so the player can CHOOSE among
        // distinct costs (rule 17) rather than have the min auto-selected.
        self.all_digivolve_routes_for_card(card, base_handle)
            .into_iter()
            .min_by_key(|route| route.memory_cost)
    }

    /// Every applicable normal-digivolve route for `card` onto `base_handle`,
    /// deduplicated by `(memory_cost, app_fusion)`. Unlike
    /// `normal_digivolve_route_for_card` (which collapses to the cheapest), this
    /// enumerates each distinct way the base satisfies the card's digivolution
    /// requirements — printed evo-cost circles, DSL alt-digivolve paths, and App
    /// Fusion — so the player can be offered the choice of which cost to pay when
    /// more than one applies (e.g. BT16-040 Wormmon over Minomon: "[Minomon]:
    /// Cost 0" vs "any Lv.2: Cost 1"). DNA digivolve is NOT included (it is a
    /// separate action). DCGO registers each requirement with its own
    /// `digivolutionCost` (`AddSelfDigivolutionRequirementStaticEffect`).
    pub(crate) fn all_digivolve_routes_for_card(
        &self,
        card: &CardSource,
        base_handle: PermanentHandle,
    ) -> Vec<DigivolveRouteMatch> {
        // Q3 (G-DIGIVOLVE-TARGET-RESTRICTION): a base carrying a
        // `CanOnlyDigivolveInto` restriction (e.g. EX10-020 "can only digivolve
        // into [Apocalymon]") offers NO digivolve route into a non-matching card.
        if self.digivolve_target_blocked_by_restriction(base_handle, card) {
            return Vec::new();
        }
        let Some(base) = self
            .players
            .get(base_handle.player as usize)
            .and_then(|p| p.battle_area.get(base_handle.index as usize))
        else {
            return Vec::new();
        };

        let mut routes = self.collect_rules_digivolve_routes(card, base_handle, base);
        // App Fusion (cost 0, ignores evo-cost/level/colour) and the DSL
        // alt-digivolve paths are folded in so they surface through the same
        // digivolve action + mask + commit path; the commit path checks
        // `route.app_fusion` to also consume the host's linked cards.
        routes.extend(self.collect_dsl_alt_digivolve_routes(card, base_handle));
        routes.extend(self.collect_app_fusion_routes(card, base_handle));
        routes.sort_by_key(|r| (r.memory_cost, r.app_fusion));
        routes.dedup();
        routes
    }

    /// `all_digivolve_routes_for_card` for a hand card by index — the entry the
    /// interactive digivolve path uses to decide whether to prompt for a cost
    /// choice. Returns empty if the base isn't the acting player's or indices
    /// are out of range.
    pub(crate) fn distinct_digivolve_routes_for_hand_card(
        &self,
        player: PlayerId,
        hand_index: usize,
        base_handle: PermanentHandle,
    ) -> Vec<DigivolveRouteMatch> {
        if base_handle.player != player {
            return Vec::new();
        }
        let Some(card) = self
            .players
            .get(player as usize)
            .and_then(|p| p.hand.get(hand_index))
        else {
            return Vec::new();
        };
        self.all_digivolve_routes_for_card(card, base_handle)
    }

    fn collect_dsl_alt_digivolve_routes(
        &self,
        card: &CardSource,
        base_handle: PermanentHandle,
    ) -> Vec<DigivolveRouteMatch> {
        #[cfg(feature = "dsl-yaml-loader")]
        {
            let card_id = card.card_id(&self.card_data);
            let rctx = EffectReadContext::new(self, card.handle(), Some(base_handle), card.owner);
            let Some(base) = self
                .players
                .get(base_handle.player as usize)
                .and_then(|p| p.battle_area.get(base_handle.index as usize))
            else {
                return Vec::new();
            };
            let base_top = base.top_card();
            let base_meta = &self.card_data[base_top.data_index];
            let base_requires_treated_as = !base_top.is_digimon_card_for_search(&self.card_data)
                && base_meta.card_kind != CardKind::DigiEgg;
            let mut routes: Vec<DigivolveRouteMatch> = Vec::new();

            // Lookup-side direction: tracks where the path is registered.
            // From-side paths live on the HAND card; Into-side paths live
            // on the SOURCE permanent (carrier). Each path's
            // `path.direction` must agree with the lookup side or it is
            // not applicable.
            #[derive(Copy, Clone, PartialEq)]
            enum LookupDirection {
                From,
                Into,
            }

            // ── Direction::From (default): paths registered on the HAND
            // card; `from:` filters the SOURCE (base) permanent.
            let from_paths = self.alt_path_registry.get(card_id);
            // ── Direction::Into (Phase 2 Track F): paths registered on the
            // SOURCE (carrier) card; `from:` filters the HAND-card
            // candidate. We resolve the base's top-card id to look these up.
            let into_paths = self
                .alt_path_registry
                .get(base_top.card_id(&self.card_data));

            for (path, direction) in from_paths
                .into_iter()
                .flatten()
                .map(|p| (p, LookupDirection::From))
                .chain(
                    into_paths
                        .into_iter()
                        .flatten()
                        .map(|p| (p, LookupDirection::Into)),
                )
            {
                if !matches!(path.kind, CompiledAltPathKind::Digivolve) {
                    continue;
                }
                if !path.materials.is_empty()
                    || !path.extra_cost.is_empty()
                    || !path.on_burst_turn_end.is_empty()
                    || path.stacks_unsuspended
                    || path.marker
                {
                    continue;
                }
                // Filter each path by its declared direction. Mismatches are
                // dropped silently — a `From` path looked up from the
                // base-top side or an `Into` path looked up from the hand
                // side is not applicable to this digivolve attempt.
                let path_direction = match path.direction {
                    digimon_dsl::compiled::CompiledAltPathDirection::From => LookupDirection::From,
                    digimon_dsl::compiled::CompiledAltPathDirection::Into => LookupDirection::Into,
                };
                if path_direction != direction {
                    continue;
                }
                let Some(from) = path.from.as_ref() else {
                    continue;
                };
                // For LookupDirection::Into, `from:` filters the destination
                // hand-card candidate (subject = the source CardHandle that
                // we're digivolving into). Use PredicateSubject::Card to
                // point the evaluator at the hand-card data. For the legacy
                // LookupDirection::From, `from:` filters the source
                // permanent (the existing semantic).
                let from_matches = match direction {
                    LookupDirection::From => {
                        eval_predicate(from, &rctx, PredicateSubject::Permanent(base_handle))
                    }
                    LookupDirection::Into => {
                        eval_predicate(from, &rctx, PredicateSubject::Card(card.handle()))
                    }
                };
                if !from_matches {
                    continue;
                }
                // G-ALT-PATH-CONDITION: alt-paths may carry an extra
                // activation predicate (e.g. "if you have [Owen
                // Dreadnought]") evaluated on top of the source-filter
                // and material/extra-cost gates.
                if let Some(condition) = path.condition.as_ref() {
                    if !eval_predicate(condition, &rctx, PredicateSubject::Permanent(base_handle)) {
                        continue;
                    }
                }

                let treated_as_cost = if let Some(profile) = path.source_treated_as.as_deref() {
                    let Some(profile) = parse_treated_as_profile(profile) else {
                        continue;
                    };
                    if profile.kind != CardKind::Digimon && profile.kind != CardKind::DigiEgg {
                        continue;
                    }
                    if profile.level == 0 || profile.colors.is_empty() {
                        continue;
                    }
                    let Some(matching_cost) =
                        matching_evo_cost(card, profile.level, &profile.colors, &self.card_data)
                    else {
                        continue;
                    };
                    Some(matching_cost)
                } else {
                    if base_requires_treated_as {
                        continue;
                    }
                    printed_digivolve_memory_cost(card, base, &self.card_data)
                };

                let memory_cost = match &path.cost {
                    Some(CompiledCost::Literal(n)) => {
                        let Some(memory_cost) = u16::try_from(*n).ok() else {
                            continue;
                        };
                        memory_cost
                    }
                    Some(CompiledCost::Formula(formula)) => {
                        let value = formula_eval::evaluate_read(formula, &rctx, base_handle);
                        let Some(memory_cost) = u16::try_from(value).ok() else {
                            continue;
                        };
                        memory_cost
                    }
                    None => {
                        let Some(memory_cost) = treated_as_cost else {
                            continue;
                        };
                        memory_cost
                    }
                };
                routes.push(DigivolveRouteMatch::digivolve(memory_cost));
            }
            routes
        }
        #[cfg(not(feature = "dsl-yaml-loader"))]
        {
            let _ = (card, base_handle);
            Vec::new()
        }
    }

    /// App Fusion route lookup (Appmon "App Fusion" mechanic).
    ///
    /// The hand `card` carries an `app_fusion` alt-path whose `materials`
    /// list the named cards (e.g. Kabemon / Gomimon / Ecomon / Puzzlemon).
    /// The play is legal onto `base_handle` when that host Digimon has
    /// **2 distinct named cards linked together**: the host's TOP card
    /// matches one named condition AND one of the host's LINKED cards
    /// matches a *different* named condition. This mirrors DCGO's
    /// `GetAppFusion` `digimonCondition` (`AddAppfusionMethod.cs`): for
    /// each `i`, top matches condition[i]; for some `j != i`, a linked
    /// card matches condition[j].
    ///
    /// The cost is the path's `cost` (App Fusion is printed `Cost 0`).
    /// App Fusion ignores printed evo-cost / level / colour requirements,
    /// so — unlike a normal digivolve — there is no evo-cost gate here.
    fn collect_app_fusion_routes(
        &self,
        card: &CardSource,
        base_handle: PermanentHandle,
    ) -> Vec<DigivolveRouteMatch> {
        #[cfg(feature = "dsl-yaml-loader")]
        {
            if base_handle.player != card.owner {
                return Vec::new();
            }
            let card_id = card.card_id(&self.card_data);
            let Some(paths) = self.alt_path_registry.get(card_id) else {
                return Vec::new();
            };
            let Some(base) = self
                .players
                .get(base_handle.player as usize)
                .and_then(|p| p.battle_area.get(base_handle.index as usize))
            else {
                return Vec::new();
            };

            let mut routes: Vec<DigivolveRouteMatch> = Vec::new();
            for path in paths
                .iter()
                .filter(|path| matches!(path.kind, CompiledAltPathKind::AppFusion))
            {
                if !self.app_fusion_host_eligible(path, base) {
                    continue;
                }
                let memory_cost = match &path.cost {
                    Some(CompiledCost::Literal(n)) => match u16::try_from(*n).ok() {
                        Some(c) => c,
                        None => continue,
                    },
                    // App Fusion is printed Cost 0; non-literal costs are
                    // not defined for the mechanic. Treat a missing cost as
                    // 0 (the printed value for every App-Fusion card).
                    None => 0,
                    Some(CompiledCost::Formula(_)) => continue,
                };
                routes.push(DigivolveRouteMatch::app_fusion(memory_cost));
            }
            routes
        }
        #[cfg(not(feature = "dsl-yaml-loader"))]
        {
            let _ = (card, base_handle);
            Vec::new()
        }
    }

    /// Cheapest App-Fusion route, if any — the `is_some()` gate behind
    /// `can_app_fuse_onto`.
    fn app_fusion_digivolve_route_for_card(
        &self,
        card: &CardSource,
        base_handle: PermanentHandle,
    ) -> Option<DigivolveRouteMatch> {
        self.collect_app_fusion_routes(card, base_handle)
            .into_iter()
            .min_by_key(|route| route.memory_cost)
    }

    /// Whether `result` (a Digimon card in hand/trash) can App-Fuse onto `host`
    /// — i.e. `host` carries the named App-Fusion materials of `result`'s
    /// `app_fusion` alt-path (top matches one name, a linked card matches a
    /// distinct name). Public entry for the effect-initiated App Fuse
    /// (`EffectContext::initiate_effect_app_fuse`); the alt-play path uses the
    /// private route fn directly. Returns false when the feature that holds the
    /// alt-path registry is disabled.
    pub(crate) fn can_app_fuse_onto(&self, result: &CardSource, host: PermanentHandle) -> bool {
        #[cfg(feature = "dsl-yaml-loader")]
        {
            self.app_fusion_digivolve_route_for_card(result, host)
                .is_some()
        }
        #[cfg(not(feature = "dsl-yaml-loader"))]
        {
            let _ = (result, host);
            false
        }
    }

    /// DCGO `GetAppFusion.digimonCondition` parity: the host has 2 distinct
    /// named cards linked together — its TOP card matches one named
    /// condition and one of its LINKED cards matches a *different* named
    /// condition.
    ///
    /// DCGO builds one `EqualsCardName` condition per listed name, then
    /// requires `top == name[i]` and `linked == name[j]` with `i != j`.
    /// We mirror that at the **name** granularity (not the material-slot
    /// granularity), so the four App-Fusion names may be authored either as
    /// four `name_is` materials OR as a single material with a four-entry
    /// `name_in` list — both yield the same distinct-name conditions.
    #[cfg(feature = "dsl-yaml-loader")]
    fn app_fusion_host_eligible(&self, path: &CompiledAltPath, base: &Permanent) -> bool {
        let names = app_fusion_condition_names(path);
        // Need at least two distinct named conditions to fuse.
        if names.len() < 2 {
            return false;
        }
        let top = base.top_card();
        let top_data = &self.card_data[top.data_index];
        for (i, name_i) in names.iter().enumerate() {
            if !card_name_equals(top_data, name_i) {
                continue;
            }
            for (j, name_j) in names.iter().enumerate() {
                if i == j {
                    continue;
                }
                if base.linked_cards.iter().any(|linked| {
                    let linked_data = &self.card_data[linked.data_index];
                    card_name_equals(linked_data, name_j)
                }) {
                    return true;
                }
            }
        }
        false
    }

    pub(crate) fn valid_dna_first_targets_for_hand_card(
        &self,
        player: PlayerId,
        hand_index: usize,
        window: DnaRouteWindow,
    ) -> impl Iterator<Item = u16> + '_ {
        let battle_len = self
            .players
            .get(player as usize)
            .map(|p| p.battle_area.len())
            .unwrap_or(0);
        (0..battle_len).filter_map(move |first_idx| {
            self.valid_dna_second_targets_for_hand_card(player, hand_index, first_idx, window)
                .next()
                .map(|_| first_idx as u16)
        })
    }

    pub(crate) fn valid_dna_second_targets_for_hand_card(
        &self,
        player: PlayerId,
        hand_index: usize,
        first_idx: usize,
        window: DnaRouteWindow,
    ) -> impl Iterator<Item = u16> + '_ {
        let battle_len = self
            .players
            .get(player as usize)
            .map(|p| p.battle_area.len())
            .unwrap_or(0);
        (0..battle_len).filter_map(move |second_idx| {
            if first_idx == second_idx {
                return None;
            }
            self.dna_route_for_hand_card(player, hand_index, first_idx, second_idx, window)
                .map(|_| second_idx as u16)
        })
    }

    pub(crate) fn dna_route_for_hand_card(
        &self,
        player: PlayerId,
        hand_index: usize,
        first_idx: usize,
        second_idx: usize,
        window: DnaRouteWindow,
    ) -> Option<DnaRouteMatch> {
        let player_state = self.players.get(player as usize)?;
        let hand_card = player_state.hand.get(hand_index)?;
        let battle = &player_state.battle_area;
        let first = battle.get(first_idx)?;
        let second = battle.get(second_idx)?;

        if matches!(window, DnaRouteWindow::Main) {
            let evo_meta = &self.card_data[hand_card.data_index];
            if let Some((first_is_top, dna_cost)) =
                get_dna_stacking_order(evo_meta, first, second, &self.card_data)
            {
                return Some(DnaRouteMatch {
                    first_is_top,
                    memory_cost: dna_cost.memory_cost,
                });
            }
        }

        if matches!(window, DnaRouteWindow::EndOfTurnAction) {
            return self.registered_end_of_turn_dna_route_for_hand_card(
                player, hand_index, first_idx, second_idx,
            );
        }
        None
    }

    pub(crate) fn valid_blast_dna_field_targets_for_hand_card(
        &self,
        player: PlayerId,
        result_hand_index: usize,
    ) -> impl Iterator<Item = u16> + '_ {
        let battle_len = self
            .players
            .get(player as usize)
            .map(|p| p.battle_area.len())
            .unwrap_or(0);
        (0..battle_len).filter_map(move |field_idx| {
            // Q18 (G-BLAST-DIGIVOLVE-IMMUNITY): a field Digimon immune to its own
            // controller's Digimon effects cannot be the base of a <Blast DNA
            // Digivolve> — Blast DNA digivolve is a Digimon effect. (Quantumon
            // LM-020: immune to ALL Digimon effects incl. its own.)
            let base = crate::permanent::PermanentHandle {
                player,
                index: field_idx as u8,
            };
            if self.permanent_is_unaffected_by_effect(
                base,
                player,
                crate::enums::EffectSourceKind::Digimon,
            ) {
                return None;
            }
            self.valid_blast_dna_hand_materials_for_hand_card(player, result_hand_index, field_idx)
                .next()
                .map(|_| field_idx as u16)
        })
    }

    pub(crate) fn valid_blast_dna_hand_materials_for_hand_card(
        &self,
        player: PlayerId,
        result_hand_index: usize,
        field_idx: usize,
    ) -> impl Iterator<Item = u16> + '_ {
        let hand_len = self
            .players
            .get(player as usize)
            .map(|p| p.hand.len())
            .unwrap_or(0);
        (0..hand_len).filter_map(move |material_idx| {
            if material_idx == result_hand_index {
                return None;
            }
            self.blast_dna_route_for_hand_card(player, result_hand_index, field_idx, material_idx)
                .map(|_| material_idx as u16)
        })
    }

    pub(crate) fn blast_dna_route_for_hand_card(
        &self,
        player: PlayerId,
        result_hand_index: usize,
        field_idx: usize,
        material_hand_index: usize,
    ) -> Option<DnaRouteMatch> {
        if result_hand_index == material_hand_index {
            return None;
        }
        let player_state = self.players.get(player as usize)?;
        let result = player_state.hand.get(result_hand_index)?;
        let material = player_state.hand.get(material_hand_index)?;
        let field = player_state.battle_area.get(field_idx)?;

        if let Some(route) =
            self.registered_blast_dna_route_for_hand_card(player, result, material, field_idx)
        {
            return Some(route);
        }

        if self.has_registered_blast_dna_paths(result) {
            return None;
        }

        let result_meta = &self.card_data[result.data_index];
        let material_meta = &self.card_data[material.data_index];

        for cost in &result_meta.dna_costs {
            if perm_matches_req(field, &cost.requirement1, &self.card_data)
                && card_data_matches_req(material_meta, &cost.requirement2)
            {
                return Some(DnaRouteMatch {
                    first_is_top: true,
                    memory_cost: cost.memory_cost,
                });
            }
            if perm_matches_req(field, &cost.requirement2, &self.card_data)
                && card_data_matches_req(material_meta, &cost.requirement1)
            {
                return Some(DnaRouteMatch {
                    first_is_top: false,
                    memory_cost: cost.memory_cost,
                });
            }
        }
        None
    }

    fn has_registered_blast_dna_paths(&self, result: &CardSource) -> bool {
        #[cfg(feature = "dsl-yaml-loader")]
        {
            let card_id = result.card_id(&self.card_data);
            self.alt_path_registry
                .get(card_id)
                .map(|paths| {
                    paths
                        .iter()
                        .any(|path| matches!(path.kind, CompiledAltPathKind::BlastDnaDigivolve))
                })
                .unwrap_or(false)
        }
        #[cfg(not(feature = "dsl-yaml-loader"))]
        {
            let _ = result;
            false
        }
    }

    fn registered_blast_dna_route_for_hand_card(
        &self,
        player: PlayerId,
        result: &CardSource,
        material: &CardSource,
        field_idx: usize,
    ) -> Option<DnaRouteMatch> {
        #[cfg(feature = "dsl-yaml-loader")]
        {
            let card_id = result.card_id(&self.card_data);
            let paths = self.alt_path_registry.get(card_id)?;
            let field_handle = PermanentHandle {
                player,
                index: field_idx as u8,
            };
            let rctx = EffectReadContext::new(self, result.handle(), Some(field_handle), player);
            for path in paths {
                if !matches!(path.kind, CompiledAltPathKind::BlastDnaDigivolve)
                    || path.materials.len() != 2
                {
                    continue;
                }
                let cost = registered_blast_dna_literal_cost(path)?;
                if blast_material_matches_permanent(&path.materials[0], &rctx, field_handle)
                    && blast_material_matches_card(&path.materials[1], &rctx, material)
                {
                    return Some(DnaRouteMatch {
                        first_is_top: true,
                        memory_cost: cost,
                    });
                }
                if blast_material_matches_permanent(&path.materials[1], &rctx, field_handle)
                    && blast_material_matches_card(&path.materials[0], &rctx, material)
                {
                    return Some(DnaRouteMatch {
                        first_is_top: false,
                        memory_cost: cost,
                    });
                }
            }
            None
        }
        #[cfg(not(feature = "dsl-yaml-loader"))]
        {
            let _ = (player, result, material, field_idx);
            None
        }
    }

    fn registered_end_of_turn_dna_route_for_hand_card(
        &self,
        player: PlayerId,
        hand_index: usize,
        first_idx: usize,
        second_idx: usize,
    ) -> Option<DnaRouteMatch> {
        let player_state = self.players.get(player as usize)?;
        let hand_card = player_state.hand.get(hand_index)?;
        let hand_subject = PredicateSubject::Card(hand_card.handle());

        for (source_card_id, source_card, source_permanent, controller, inherited_source) in
            live_stack_sources(player_state, player, &self.card_data)
        {
            let Some(effects) = self.effects_for_card(&source_card_id, source_card) else {
                continue;
            };
            for effect in effects.iter() {
                if effect.inherited != inherited_source
                    || effect.timing != EffectTiming::EndOfYourTurn
                {
                    continue;
                }
                let Some(registration) = effect.alt_path_registration.as_ref() else {
                    continue;
                };
                let rctx =
                    EffectReadContext::new(self, source_card, Some(source_permanent), controller);
                if let Some(condition) = &effect.condition {
                    if !condition(&rctx) {
                        continue;
                    }
                }
                if let Some(applies_to) = registration.applies_to.as_ref() {
                    if !eval_predicate(applies_to, &rctx, hand_subject) {
                        continue;
                    }
                }
                if let Some(matched) = registered_dna_route_match(
                    registration.registers.as_ref(),
                    &rctx,
                    player,
                    first_idx,
                    second_idx,
                ) {
                    return Some(matched);
                }
            }
        }
        None
    }

    /// Every printed standard-digivolve (evo-cost) route the base satisfies, as
    /// `DigivolveRouteMatch`es. Unlike the old `rules_digivolve_memory_cost`
    /// (which returned only the cheapest matching cost), this enumerates each
    /// distinct printed cost so the player can be offered the choice.
    fn collect_rules_digivolve_routes(
        &self,
        card: &CardSource,
        base_handle: PermanentHandle,
        base: &Permanent,
    ) -> Vec<DigivolveRouteMatch> {
        let identity = base.synth_identity(&self.card_data, &self.modifiers, base_handle);
        if !matches!(
            identity.kind,
            CardKind::Digimon | CardKind::Dual | CardKind::DigiEgg
        ) {
            return Vec::new();
        }
        let Some(base_level) = identity.level else {
            return Vec::new();
        };
        all_matching_evo_costs(
            card.digivolution_costs(&self.card_data),
            base_level,
            &identity.colors,
        )
        .into_iter()
        .map(DigivolveRouteMatch::digivolve)
        .collect()
    }
}

fn printed_digivolve_memory_cost(
    card: &CardSource,
    base: &Permanent,
    card_data: &[CardData],
) -> Option<u16> {
    let base_top = base.top_card();
    let base_meta = &card_data[base_top.data_index];

    if !base_top.is_digimon_card_for_search(card_data) && base_meta.card_kind != CardKind::DigiEgg {
        return None;
    }

    let base_level = base_top.digimon_level(card_data)?;
    let base_colors = base_top.digimon_colors(card_data);
    matching_evo_cost(card, base_level, &base_colors, card_data)
}

fn matching_evo_cost(
    card: &CardSource,
    base_level: u8,
    base_colors: &[CardColor],
    card_data: &[CardData],
) -> Option<u16> {
    matching_evo_cost_from_evo_costs(card.digivolution_costs(card_data), base_level, base_colors)
}

fn matching_evo_cost_from_evo_costs(
    evo_costs: &[crate::card_data::EvoCost],
    base_level: u8,
    base_colors: &[CardColor],
) -> Option<u16> {
    evo_costs
        .iter()
        .filter(|ec| {
            ec.level == base_level
                && crate::action::mask::evo_color(ec.card_color)
                    .map(|c| base_colors.contains(&c))
                    .unwrap_or(false)
        })
        .map(|ec| ec.memory_cost)
        .min()
}

/// All DISTINCT memory costs among `evo_costs` entries that match the base's
/// level + colours (sorted ascending). Unlike `matching_evo_cost_from_evo_costs`
/// (cheapest only), this enumerates every printed standard-digivolve cost the
/// base satisfies — the basis for the player's cost choice (rule 17). In
/// practice a card's two printed circles are usually different colours at the
/// same cost (deduped to one here); distinct costs arise when an evo-cost and a
/// DSL alt-path overlap on the same base.
fn all_matching_evo_costs(
    evo_costs: &[crate::card_data::EvoCost],
    base_level: u8,
    base_colors: &[CardColor],
) -> Vec<u16> {
    let mut costs: Vec<u16> = evo_costs
        .iter()
        .filter(|ec| {
            ec.level == base_level
                && crate::action::mask::evo_color(ec.card_color)
                    .map(|c| base_colors.contains(&c))
                    .unwrap_or(false)
        })
        .map(|ec| ec.memory_cost)
        .collect();
    costs.sort_unstable();
    costs.dedup();
    costs
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct TreatedAsProfile {
    level: u8,
    colors: Vec<CardColor>,
    kind: CardKind,
}

fn parse_treated_as_profile(profile: &str) -> Option<TreatedAsProfile> {
    let mut parts = profile.split('_');
    if parts.next()? != "level" {
        return None;
    }
    let level = parts.next()?.parse::<u8>().ok()?;
    let rest: Vec<_> = parts.collect();
    if rest.len() < 2 {
        return None;
    }
    let kind = match *rest.last()? {
        "digimon" => CardKind::Digimon,
        "digiegg" | "digi_egg" => CardKind::DigiEgg,
        _ => return None,
    };
    let color_parts = &rest[..rest.len() - 1];
    let mut colors = Vec::new();
    for color in color_parts {
        colors.push(parse_profile_color(color)?);
    }
    Some(TreatedAsProfile {
        level,
        colors,
        kind,
    })
}

fn parse_profile_color(color: &str) -> Option<CardColor> {
    match color {
        "red" => Some(CardColor::Red),
        "blue" => Some(CardColor::Blue),
        "yellow" => Some(CardColor::Yellow),
        "green" => Some(CardColor::Green),
        "black" => Some(CardColor::Black),
        "purple" => Some(CardColor::Purple),
        "white" => Some(CardColor::White),
        _ => None,
    }
}

fn live_stack_sources(
    player_state: &crate::player::Player,
    player: PlayerId,
    data: &[CardData],
) -> Vec<(
    String,
    crate::card_source::CardHandle,
    PermanentHandle,
    PlayerId,
    bool,
)> {
    let mut out = Vec::new();
    for (index, permanent) in player_state.battle_area.iter().enumerate() {
        let host = PermanentHandle {
            player,
            index: index as u8,
        };
        let stack_size = permanent.card_sources.len();
        for (source_index, source) in permanent.card_sources.iter().enumerate() {
            out.push((
                source.card_id(data).to_string(),
                source.handle(),
                host,
                player,
                source_index + 1 < stack_size,
            ));
        }
    }
    out
}

fn registered_dna_route_match(
    path: &CompiledAltPath,
    rctx: &EffectReadContext<'_>,
    player: PlayerId,
    first_idx: usize,
    second_idx: usize,
) -> Option<DnaRouteMatch> {
    if !matches!(path.kind, CompiledAltPathKind::DnaDigivolve) || path.materials.len() != 2 {
        return None;
    }
    // Existing DNA action IDs select exactly two battle-area permanents.
    // Broader alt-path shapes need their own selection flow before they can
    // be exposed without approximation.
    let cost = registered_path_literal_cost(path)?;
    let first = PermanentHandle {
        player,
        index: first_idx as u8,
    };
    let second = PermanentHandle {
        player,
        index: second_idx as u8,
    };
    if material_matches(&path.materials[0], rctx, first)
        && material_matches(&path.materials[1], rctx, second)
    {
        return Some(DnaRouteMatch {
            first_is_top: true,
            memory_cost: cost,
        });
    }
    if material_matches(&path.materials[0], rctx, second)
        && material_matches(&path.materials[1], rctx, first)
    {
        return Some(DnaRouteMatch {
            first_is_top: false,
            memory_cost: cost,
        });
    }
    None
}

fn registered_path_literal_cost(path: &CompiledAltPath) -> Option<i16> {
    if path.from.is_some()
        || !path.extra_cost.is_empty()
        || !path.on_burst_turn_end.is_empty()
        || path.stacks_unsuspended
        || path.ignore_requirements
        || path.source_treated_as.is_some()
        || path.marker
    {
        return None;
    }
    match &path.cost {
        None => Some(0),
        Some(CompiledCost::Literal(n)) => i16::try_from(*n).ok(),
        Some(CompiledCost::Formula(_)) => None,
    }
}

fn registered_blast_dna_literal_cost(path: &CompiledAltPath) -> Option<i16> {
    if path.from.is_some()
        || !path.extra_cost.is_empty()
        || !path.on_burst_turn_end.is_empty()
        || path.ignore_requirements
        || path.source_treated_as.is_some()
        || path.marker
    {
        return None;
    }
    match &path.cost {
        None => Some(0),
        Some(CompiledCost::Literal(n)) => i16::try_from(*n).ok(),
        Some(CompiledCost::Formula(_)) => None,
    }
}

/// Collect the distinct App-Fusion condition names from a compiled
/// `app_fusion` alt-path. DCGO's `AddAppfuseMethodByName` builds one
/// `EqualsCardName` condition per listed name; we gather those names from
/// each material's `name_is` / `name_in` (including names nested under
/// `all_of` / `any_of`). App Fusion conditions are purely name-based — no
/// level / trait / colour gates — matching the printed mechanic. Names are
/// de-duplicated case-insensitively so the distinct-condition requirement
/// (`i != j`) is meaningful regardless of authoring style (four `name_is`
/// materials vs one `name_in` list).
#[cfg(feature = "dsl-yaml-loader")]
fn app_fusion_condition_names(path: &CompiledAltPath) -> Vec<String> {
    let mut names: Vec<String> = Vec::new();
    let mut push = |name: &str| {
        if !names.iter().any(|n| n.eq_ignore_ascii_case(name)) {
            names.push(name.to_string());
        }
    };
    for material in &path.materials {
        collect_predicate_names(&material.filter, &mut push);
    }
    names
}

#[cfg(feature = "dsl-yaml-loader")]
fn collect_predicate_names<F: FnMut(&str)>(
    pred: &digimon_dsl::compiled::CompiledPredicate,
    push: &mut F,
) {
    if let Some(name) = pred.name_is.as_ref() {
        push(name);
    }
    if let Some(list) = pred.name_in.as_ref() {
        for name in list {
            push(name);
        }
    }
    for child in pred.all_of.iter().chain(pred.any_of.iter()) {
        collect_predicate_names(child, push);
    }
}

/// Exact card-name equality (DCGO `EqualsCardName`).
#[cfg(feature = "dsl-yaml-loader")]
fn card_name_equals(card: &CardData, name: &str) -> bool {
    card.card_name.eq_ignore_ascii_case(name)
}

fn material_matches(
    material: &CompiledMaterial,
    rctx: &EffectReadContext<'_>,
    handle: PermanentHandle,
) -> bool {
    if material.repeat.is_some()
        || material.distinct_by.is_some()
        || material.stack_under
        || !material
            .zones
            .iter()
            .all(|z| *z == CompiledZone::BattleArea)
    {
        return false;
    }
    eval_predicate(&material.filter, rctx, PredicateSubject::Permanent(handle))
}

fn blast_material_matches_permanent(
    material: &CompiledMaterial,
    rctx: &EffectReadContext<'_>,
    handle: PermanentHandle,
) -> bool {
    if material.repeat.is_some()
        || material.distinct_by.is_some()
        || material.stack_under
        || !material
            .zones
            .iter()
            .all(|z| matches!(*z, CompiledZone::BattleArea | CompiledZone::Material))
    {
        return false;
    }
    eval_predicate(&material.filter, rctx, PredicateSubject::Permanent(handle))
}

fn blast_material_matches_card(
    material: &CompiledMaterial,
    rctx: &EffectReadContext<'_>,
    card: &CardSource,
) -> bool {
    if material.repeat.is_some()
        || material.distinct_by.is_some()
        || material.stack_under
        || !material
            .zones
            .iter()
            .all(|z| matches!(*z, CompiledZone::Hand | CompiledZone::Material))
    {
        return false;
    }
    eval_predicate(
        &material.filter,
        rctx,
        PredicateSubject::Card(card.handle()),
    )
}

fn perm_matches_req(perm: &Permanent, req: &DnaRequirement, data: &[CardData]) -> bool {
    let top = perm.top_card();
    let meta = &data[top.data_index];
    card_data_matches_req(meta, req)
}

fn card_data_matches_req(meta: &CardData, req: &DnaRequirement) -> bool {
    if req.level > 0 {
        match meta.level {
            Some(l) if l == req.level => {}
            _ => return false,
        }
    }
    // Slash-color reqs like "Blue/Purple Lv.6" accept any listed color on
    // the material. Empty `card_colors` means "any color" (name/level
    // gated only). Match the Python semantics at
    // `digivolve_validator.py::perm_matches_req`.
    if !req.card_colors.is_empty() && !req.card_colors.iter().any(|c| meta.colors.contains(c)) {
        return false;
    }
    if !req.name_contains.is_empty()
        && !meta
            .card_name
            .to_lowercase()
            .contains(&req.name_contains.to_lowercase())
    {
        return false;
    }
    if !req.text_contains.is_empty() {
        // Python's `_perm_matches_dna_req` searches effect + inherited +
        // security text concatenated (digivolve_validator.py:189-199).
        let needle = req.text_contains.to_lowercase();
        let haystack = [
            meta.effect_text.as_str(),
            meta.inherited_text.as_str(),
            meta.security_text.as_str(),
        ]
        .join(" ")
        .to_lowercase();
        if !haystack.contains(&needle) {
            return false;
        }
    }
    true
}

/// Returns true if `(perm_a, perm_b)` satisfy some DNA cost on `evo_meta`.
/// Tries both orderings (a↔req1 / b↔req2 AND a↔req2 / b↔req1).
pub fn can_dna_digivolve(
    evo_meta: &CardData,
    perm_a: &Permanent,
    perm_b: &Permanent,
    data: &[CardData],
) -> bool {
    matching_dna_cost(evo_meta, perm_a, perm_b, data).is_some()
}

/// Returns the first DNA cost whose material requirements are satisfied by
/// `(perm_a, perm_b)`, accepting either printed material order.
pub fn matching_dna_cost<'a>(
    evo_meta: &'a CardData,
    perm_a: &Permanent,
    perm_b: &Permanent,
    data: &[CardData],
) -> Option<&'a DnaCost> {
    for cost in &evo_meta.dna_costs {
        let orderings = [
            (&cost.requirement1, &cost.requirement2),
            (&cost.requirement2, &cost.requirement1),
        ];
        for (ra, rb) in orderings {
            if perm_matches_req(perm_a, ra, data) && perm_matches_req(perm_b, rb, data) {
                return Some(cost);
            }
        }
    }
    None
}

/// Returns the first DNA cost on `evo_meta` satisfied by a `(field_perm,
/// card_material)` pair — one material is a battle-area permanent, the other a
/// card (in hand or trash). Accepts either printed material order. This is the
/// recipe oracle for the hand-partner (BT17-095) and trash-partner
/// (BT18-015/073) DNA verbs, where the second material is not yet a permanent.
/// DCGO parity: `CanJogressFromTargetPermanent` after `CreateNewPermanent`
/// materialises the card as a temp permanent — we match the card's `CardData`
/// against the requirement directly rather than round-tripping through a
/// permanent.
pub fn matching_dna_cost_perm_and_card<'a>(
    evo_meta: &'a CardData,
    field_perm: &Permanent,
    card_material: &CardData,
    data: &[CardData],
) -> Option<&'a DnaCost> {
    for cost in &evo_meta.dna_costs {
        let orderings = [
            (&cost.requirement1, &cost.requirement2),
            (&cost.requirement2, &cost.requirement1),
        ];
        for (r_field, r_card) in orderings {
            if perm_matches_req(field_perm, r_field, data)
                && card_data_matches_req(card_material, r_card)
            {
                return Some(cost);
            }
        }
    }
    None
}

/// Returns true if any unordered pair in `battle_area` is a valid DNA pair
/// for `evo_meta`. O(n²) but `battle_area` is tiny (≤ FIELD_SLOTS).
pub fn has_valid_dna_targets(
    evo_meta: &CardData,
    battle_area: &[Permanent],
    data: &[CardData],
) -> bool {
    if evo_meta.dna_costs.is_empty() {
        return false;
    }
    for i in 0..battle_area.len() {
        for j in (i + 1)..battle_area.len() {
            if can_dna_digivolve(evo_meta, &battle_area[i], &battle_area[j], data) {
                return true;
            }
        }
    }
    false
}

/// Returns `Some((top_is_perm_a, &DnaCost))` for the matching cost on `evo_meta`.
/// `top_is_perm_a` is true when `perm_a` matches `requirement1` (so `perm_a`
/// is the "top half" of the bottom material stack); false when `perm_b` does.
///
/// Tries each cost in order; for each cost tries `(perm_a, perm_b)` mapped to
/// `(req1, req2)` first, then `(req2, req1)`. Returns `None` if no orientation
/// of any cost is satisfied.
///
/// Port of Python's `digivolve_validator.py::get_dna_stacking_order`.
pub fn get_dna_stacking_order<'a>(
    evo_meta: &'a CardData,
    perm_a: &Permanent,
    perm_b: &Permanent,
    data: &[CardData],
) -> Option<(bool, &'a DnaCost)> {
    for cost in &evo_meta.dna_costs {
        if perm_matches_req(perm_a, &cost.requirement1, data)
            && perm_matches_req(perm_b, &cost.requirement2, data)
        {
            return Some((true, cost));
        }
        if perm_matches_req(perm_a, &cost.requirement2, data)
            && perm_matches_req(perm_b, &cost.requirement1, data)
        {
            return Some((false, cost));
        }
    }
    None
}

/// Returns battle-area indices that can be the second material when the
/// first material is `first_idx`. The first index itself is excluded.
///
/// Port of Python's `digivolve_validator.py::get_valid_dna_second_targets`.
pub fn get_valid_dna_second_targets(
    evo_meta: &CardData,
    first_idx: usize,
    battle_area: &[Permanent],
    data: &[CardData],
) -> Vec<u16> {
    if first_idx >= battle_area.len() {
        return Vec::new();
    }
    let first_perm = &battle_area[first_idx];
    let mut out = Vec::new();
    for j in 0..battle_area.len() {
        if j == first_idx {
            continue;
        }
        if can_dna_digivolve(evo_meta, first_perm, &battle_area[j], data) {
            out.push(j as u16);
        }
    }
    out
}

#[cfg(test)]
mod tests_stacking {
    use super::*;
    use crate::card_data::{CardData, DnaCost};
    use crate::card_source::CardSource;
    use crate::debug_runner::{dna_req_lv, make_test_card};
    use crate::permanent::Permanent;

    fn lvl_card(idx: usize, level: u8) -> CardData {
        let mut d = make_test_card(&format!("LVL{}-{}", level, idx), "TestMon");
        d.level = Some(level);
        d
    }

    fn perm_at(data_index: usize) -> Permanent {
        // Owner / card_index don't matter for these helpers — they only
        // read CardData via `data[top.data_index]`.
        Permanent::new(CardSource::new(data_index, 0, 0), 0)
    }

    #[test]
    fn stacking_order_picks_correct_orientation() {
        // evo wants req1=Lv5, req2=Lv6
        let mut evo = make_test_card("EVO-1", "Evo");
        evo.dna_costs = vec![DnaCost {
            memory_cost: 1,
            requirement1: dna_req_lv(5),
            requirement2: dna_req_lv(6),
        }];
        let data = vec![evo, lvl_card(0, 5), lvl_card(1, 6)];
        let p_lv5 = perm_at(1);
        let p_lv6 = perm_at(2);

        // Pass perms in (Lv6, Lv5) order — helper should report top=Lv5, bottom=Lv6.
        let order = get_dna_stacking_order(&data[0], &p_lv6, &p_lv5, &data);
        let (top_is_a, cost) = order.expect("should match");
        assert!(!top_is_a, "passed (Lv6, Lv5); top should be perm_b (Lv5)");
        assert_eq!(cost.memory_cost, 1);
    }

    #[test]
    fn second_targets_excludes_first_index() {
        let mut evo = make_test_card("EVO-2", "Evo");
        evo.dna_costs = vec![DnaCost {
            memory_cost: 0,
            requirement1: dna_req_lv(5),
            requirement2: dna_req_lv(5),
        }];
        let data = vec![evo, lvl_card(0, 5)];
        let battle = vec![perm_at(1), perm_at(1), perm_at(1)];

        let valid = get_valid_dna_second_targets(&data[0], 1, &battle, &data);
        assert_eq!(valid, vec![0, 2], "first idx (1) must be excluded");
    }
}

#[cfg(test)]
mod tests_q3_digivolve_target_restriction {
    //! Q3 (`G-DIGIVOLVE-TARGET-RESTRICTION`): a base permanent carrying a
    //! `CanOnlyDigivolveInto` modifier offers a normal-digivolve route ONLY into
    //! a card whose name matches the allowed name (DCGO `CanNotDigivolveStaticSelfEffect`,
    //! EX10-020 Puppetmon "[All Turns] this Digimon can only digivolve into [Apocalymon]").
    use crate::card_data::{CardData, EvoCost};
    use crate::debug_runner::{make_test_card, DebugRunner};
    use crate::enums::{CardColor, CardKind, Expiry, ModifierType};
    use crate::modifiers::{ModifierEntry, ModifierPayload};

    fn lv4_base() -> CardData {
        let mut c = make_test_card("BASE", "Base");
        c.card_kind = CardKind::Digimon;
        c.level = Some(4);
        c.dp = Some(4000);
        c.colors = vec![CardColor::Red];
        c
    }

    fn lv5_evo(id: &str, name: &str) -> CardData {
        let mut c = make_test_card(id, name);
        c.card_kind = CardKind::Digimon;
        c.level = Some(5);
        c.dp = Some(6000);
        c.colors = vec![CardColor::Red];
        c.evo_costs = vec![EvoCost {
            card_color: CardColor::Red as u8,
            level: 4,
            memory_cost: 0,
        }];
        c
    }

    #[test]
    fn can_only_digivolve_into_blocks_nonmatching_name() {
        let mut r = DebugRunner::builder()
            .add_card(lv4_base())
            .add_card(lv5_evo("ALLOWED", "Apocalymon"))
            .add_card(lv5_evo("OTHER", "Megadramon"))
            .hand(0, &["ALLOWED", "OTHER"])
            .memory(10)
            .start();
        let base = r.place_on_field(0, "BASE", Some(0));

        // Control: both Lv4→Lv5 routes are valid BEFORE any restriction.
        assert!(
            r.game
                .normal_digivolve_route_for_hand_card(0, 0, base)
                .is_some(),
            "control: Apocalymon digivolve route is valid"
        );
        assert!(
            r.game
                .normal_digivolve_route_for_hand_card(0, 1, base)
                .is_some(),
            "control: Megadramon digivolve route is valid"
        );

        // Install "[All Turns] this Digimon can only digivolve into [Apocalymon]".
        r.game.modifiers.add(
            base,
            ModifierEntry::simple(ModifierType::CanOnlyDigivolveInto, 0, Expiry::Permanent, 0)
                .with_payload(ModifierPayload::Name {
                    value: "Apocalymon".to_string(),
                    base: false,
                }),
        );

        // Now only the Apocalymon route survives; the non-matching route is gone.
        assert!(
            r.game
                .normal_digivolve_route_for_hand_card(0, 0, base)
                .is_some(),
            "Apocalymon (allowed name) must still be a valid digivolve target"
        );
        assert!(
            r.game
                .normal_digivolve_route_for_hand_card(0, 1, base)
                .is_none(),
            "Megadramon must be BLOCKED — base may only digivolve into [Apocalymon]"
        );
    }

    #[test]
    fn no_restriction_is_a_noop() {
        // Sanity: a base WITHOUT the modifier offers the route normally (the
        // common case — existing cards are unaffected by the new consult).
        let mut r = DebugRunner::builder()
            .add_card(lv4_base())
            .add_card(lv5_evo("OTHER", "Megadramon"))
            .hand(0, &["OTHER"])
            .memory(10)
            .start();
        let base = r.place_on_field(0, "BASE", Some(0));
        assert!(
            r.game
                .normal_digivolve_route_for_hand_card(0, 0, base)
                .is_some(),
            "no CanOnlyDigivolveInto modifier ⇒ digivolve route unaffected"
        );
    }
}
