use std::collections::BTreeMap;

use crate::card_data::CardData;
use crate::permanent::Permanent;

pub(crate) fn same_level_pairs_in_sources(perm: &Permanent, data: &[CardData]) -> i32 {
    let mut counts: BTreeMap<u8, i32> = BTreeMap::new();
    for source in perm.card_sources.iter().rev().skip(1) {
        if let Some(level) = source.level(data) {
            *counts.entry(level).or_default() += 1;
        }
    }
    counts.values().map(|count| count / 2).sum()
}
