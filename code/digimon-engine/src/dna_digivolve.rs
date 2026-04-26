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

use crate::card_data::{CardData, DnaRequirement};
use crate::permanent::Permanent;

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
    if !req.card_colors.is_empty()
        && !req.card_colors.iter().any(|c| meta.colors.contains(c))
    {
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
    for cost in &evo_meta.dna_costs {
        let orderings = [
            (&cost.requirement1, &cost.requirement2),
            (&cost.requirement2, &cost.requirement1),
        ];
        for (ra, rb) in orderings {
            if perm_matches_req(perm_a, ra, data) && perm_matches_req(perm_b, rb, data) {
                return true;
            }
        }
    }
    false
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
