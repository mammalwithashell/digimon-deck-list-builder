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
use crate::digixros::matches_digixros_name_requirement;
use crate::game::Game;
use crate::permanent::Permanent;
use crate::permanent::PermanentHandle;

impl Game {
    pub fn card_data_by_id(&self, card_id: &str) -> Option<&CardData> {
        self.card_data.iter().find(|card| card.card_id == card_id)
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
    }

    pub fn card_matches_generic_name(&self, card_id: &str, required_name: &str) -> bool {
        self.card_data_by_id(card_id)
            .map(|data| data.card_name.eq_ignore_ascii_case(required_name))
            .unwrap_or(false)
    }
}

fn perm_matches_req(perm: &Permanent, req: &DnaRequirement, data: &[CardData]) -> bool {
    let top = perm.top_card();
    let meta = &data[top.data_index];

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
