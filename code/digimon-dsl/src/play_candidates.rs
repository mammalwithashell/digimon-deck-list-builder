//! Compile-time "a play-from-trash pick offers only cards the play would accept" marking.
//!
//! An effect that plays a card from a trash is authored as a `select_trash { bind_as: card }` followed by
//! `play_from_trash_free { hand_index: card }` (or the paying `play_from_trash`). While a "can't play Digimon /
//! Tamers by effects" restriction is active (Crimson Blaze (BT8-097), Pomumon (BT9-047)), a "can't play from the
//! trash" lock holds (Gallantmon (BT23-014)), the battle area is full, or the card can never be played (a DUAL
//! card, an Option, a Digi-Egg), the play verb refuses the card (`Game::trash_play_refusal`) and the pick did
//! nothing: a dead choice in the action space. DCGO filters the candidates instead -- every such script ends its
//! `CanSelectCardCondition` with `CardEffectCommons.CanPlayAsNewPermanent(cardSource, false, activateClass)`
//! (BT24_088.cs, LM_029.cs:120/265, EX12_065.cs:67) -- so under the lock it shows no pick at all. Found in logged
//! games (bug scan: Asuna Shiroki (BT24-088), Yellow Scramble (LM-029), Kaguyamon (EX12-065) each offered a
//! prohibited Digimon and the pick fizzled).
//!
//! This pass adds `can_play_from_effect_without_cost: true` to the filter of every `select_trash` whose binding
//! feeds a `play_from_trash_free` / `play_from_trash`, unless the card authored a play-legality leaf itself
//! (`can_play_from_effect_without_cost` or `can_play_or_use_from_effect`, anywhere in the filter).
//!
//! Why this is right for EVERY user of that lowering, not only the three cards above: the leaf
//! (`eval_card_fields` -> `Game::can_play_card_from_effect_without_cost_for_controller` + the trash lock) reads
//! exactly the refusals `Game::trash_play_refusal` makes before the play verb moves the card -- the trash-play
//! lock, a full battle area, the effect-play lock for the card's kind, and the card-level play prohibition -- with
//! the same destination (the trash's owner) and the same controller (the effect's). So a card the leaf rejects is
//! one the verb is certain to refuse; no playable candidate is ever removed. The leaf does NOT check that a paid
//! `play_from_trash` can afford its cost (that is `affordable_with_cost`, authored per card), nor that N
//! simultaneous plays have N free slots.
//!
//! CR 4.3 does not forbid choosing a card that then can't be played (1-3-2: an impossible action simply isn't
//! performed), so this is a no-dead-choice / DCGO-fidelity rule, the same principle as the digivolve-candidate
//! pass (`digivolve_candidates.rs`) and `qa/dsl-vocab-gaps.md` "Effect-pick candidate legality".
//! G-DSL-PLAY-FROM-TRASH-PICK-LEGALITY.

use std::collections::BTreeSet;

use crate::compiled::{
    CompiledBindingRef, CompiledCard, CompiledClause, CompiledDeclarativeClause, CompiledPredicate, CompiledStep,
};
use crate::deletion_cap::{child_bodies, child_bodies_mut};

/// Run the pass over every effect body of `card`.
pub fn mark_card(card: &mut CompiledCard) {
    for clause in card.effects.iter_mut() {
        match clause {
            CompiledClause::Triggered(t) => mark_body(&mut t.process),
            CompiledClause::Declarative(
                CompiledDeclarativeClause::Replacement { process, .. }
                | CompiledDeclarativeClause::Partition { process, .. }
                | CompiledDeclarativeClause::Delay { process, .. },
            ) => mark_body(process),
            CompiledClause::Declarative(_) => {}
        }
    }
}

fn mark_body(steps: &mut [CompiledStep]) {
    let mut wanted = BTreeSet::new();
    for step in steps.iter() {
        collect(step, &mut wanted);
    }
    if wanted.is_empty() {
        return;
    }
    mark_seq(steps, &wanted);
}

/// The binding a play-from-trash step names, when it is a pick made by the effect (not `self` / `event_card`).
fn card_binding(b: &CompiledBindingRef) -> Option<&str> {
    match b {
        CompiledBindingRef::Named(n) | CompiledBindingRef::Binding(n) => Some(n.as_str()),
        _ => None,
    }
}

/// Every binding under `step` that a play-from-trash verb plays.
fn collect(step: &CompiledStep, out: &mut BTreeSet<String>) {
    if let CompiledStep::PlayFromTrashFree { trash_index, .. } | CompiledStep::PlayFromTrash { trash_index, .. } =
        step
    {
        if let Some(name) = card_binding(trash_index) {
            out.insert(name.to_string());
        }
    }
    for body in child_bodies(step) {
        for s in body {
            collect(s, out);
        }
    }
}

fn mark_seq(steps: &mut [CompiledStep], wanted: &BTreeSet<String>) {
    for step in steps.iter_mut() {
        if let CompiledStep::SelectTrash { bind_as: Some(binding), filter, .. } = step {
            if wanted.contains(binding.as_str()) && !has_play_leaf(filter) {
                filter.can_play_from_effect_without_cost = Some(true);
            }
        }
        for body in child_bodies_mut(step) {
            mark_seq(body, wanted);
        }
    }
}

/// Whether `filter` authors a play-legality leaf anywhere (itself, a conjunct, a disjunct, or under a negation).
/// An authored leaf is the card's own statement of which candidates are playable, so the pass leaves it alone.
///
/// Public for the engine's pick installer: a filter that carries the leaf reads the "can't play by effects" /
/// trash-play locks, which continuous effects materialize, so the installer settles continuous state before it
/// evaluates the candidates (`dsl_cards::step::selections::try_install`).
pub fn has_play_leaf(filter: &CompiledPredicate) -> bool {
    filter.can_play_from_effect_without_cost.is_some()
        || filter.can_play_or_use_from_effect.is_some()
        || filter.all_of.iter().any(has_play_leaf)
        || filter.any_of.iter().any(has_play_leaf)
        || filter.none_of.iter().any(has_play_leaf)
        || filter.not.as_deref().is_some_and(has_play_leaf)
}

/// Pool guard: the `select_trash` picks of `card` that feed a play-from-trash verb and carry no play-legality leaf
/// after compilation, as binding names. Empty for every card the pass reaches.
///
/// It walks the SERIALIZED card rather than the typed tree on purpose: the pass above reaches the step bodies
/// `deletion_cap::child_bodies` knows, and this check must also see a pick in a body a new step variant adds
/// before that helper learns about it.
pub fn unmarked_play_from_trash_picks(card: &CompiledCard) -> Vec<String> {
    use serde_json::Value;

    fn played(value: &Value, out: &mut BTreeSet<String>) {
        match value {
            Value::Object(map) => {
                for (key, child) in map {
                    if key == "PlayFromTrashFree" || key == "PlayFromTrash" {
                        for tag in ["Named", "Binding"] {
                            if let Some(name) = child["trash_index"].get(tag).and_then(Value::as_str) {
                                out.insert(name.to_string());
                            }
                        }
                    }
                    played(child, out);
                }
            }
            Value::Array(items) => items.iter().for_each(|item| played(item, out)),
            _ => {}
        }
    }

    fn leaf(filter: &Value) -> bool {
        match filter {
            Value::Object(map) => map.iter().any(|(key, child)| {
                ((key == "can_play_from_effect_without_cost" || key == "can_play_or_use_from_effect")
                    && !child.is_null())
                    || leaf(child)
            }),
            Value::Array(items) => items.iter().any(leaf),
            _ => false,
        }
    }

    fn unmarked(value: &Value, wanted: &BTreeSet<String>, out: &mut Vec<String>) {
        match value {
            Value::Object(map) => {
                for (key, child) in map {
                    if key == "SelectTrash" {
                        if let Some(binding) = child["bind_as"].as_str() {
                            if wanted.contains(binding) && !leaf(&child["filter"]) {
                                out.push(binding.to_string());
                            }
                        }
                    }
                    unmarked(child, wanted, out);
                }
            }
            Value::Array(items) => items.iter().for_each(|item| unmarked(item, wanted, out)),
            _ => {}
        }
    }

    let mut out = Vec::new();
    for clause in &card.effects {
        let Ok(json) = serde_json::to_value(clause) else {
            continue;
        };
        let mut wanted = BTreeSet::new();
        played(&json, &mut wanted);
        if !wanted.is_empty() {
            unmarked(&json, &wanted, &mut out);
        }
    }
    out
}

/// Whether any clause of `card` plays a picked card from a trash (for the pool guard's "did the walk see
/// anything" sanity check).
pub fn plays_a_pick_from_trash(card: &CompiledCard) -> bool {
    fn any(steps: &[CompiledStep]) -> bool {
        let mut wanted = BTreeSet::new();
        for step in steps {
            collect(step, &mut wanted);
        }
        !wanted.is_empty()
    }
    card.effects.iter().any(|clause| match clause {
        CompiledClause::Triggered(t) => any(&t.process),
        CompiledClause::Declarative(
            CompiledDeclarativeClause::Replacement { process, .. }
            | CompiledDeclarativeClause::Partition { process, .. }
            | CompiledDeclarativeClause::Delay { process, .. },
        ) => any(process),
        CompiledClause::Declarative(_) => false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn compile(effects: &str) -> CompiledCard {
        let yaml = format!(
            "card: PLAY-PICK\nname: Play pick witness\nkind: digimon\nlevel: 5\ncolor: [purple]\ncost: 7\ndp: 7000\neffects:\n{effects}"
        );
        let spec: crate::CardSpec = serde_yml::from_str(&yaml).expect("witness YAML parses");
        crate::compile::compile(&spec).expect("witness YAML compiles")
    }

    /// Every `select_trash` filter under `steps`, in order, with its binding.
    fn trash_picks<'a>(steps: &'a [CompiledStep], out: &mut Vec<(&'a str, &'a CompiledPredicate)>) {
        for step in steps {
            if let CompiledStep::SelectTrash { bind_as: Some(b), filter, .. } = step {
                out.push((b.as_str(), filter));
            }
            for body in child_bodies(step) {
                trash_picks(body, out);
            }
        }
    }

    fn picks(card: &CompiledCard) -> Vec<(&str, &CompiledPredicate)> {
        let mut out = Vec::new();
        for clause in &card.effects {
            if let CompiledClause::Triggered(t) = clause {
                trash_picks(&t.process, &mut out);
            }
        }
        out
    }

    #[test]
    fn free_play_from_trash_pick_gets_the_play_legality_leaf() {
        let card = compile(
            "  - when: when_digivolving\n    process:\n      - select_trash:\n          of: you\n          bind_as: revive\n          optional: true\n          filter: { kind: digimon, level_lte: 4 }\n          prompt: \"Play 1 level 4 or lower Digimon from your trash\"\n      - play_from_trash_free: { of: you, hand_index: revive }\n",
        );
        let picks = picks(&card);
        assert_eq!(picks.len(), 1);
        assert_eq!(picks[0].1.can_play_from_effect_without_cost, Some(true));
        assert!(picks[0].1.level_lte.is_some() && picks[0].1.kind.is_some(), "the authored filter is kept");
    }

    #[test]
    fn paid_play_from_trash_pick_inside_an_if_gets_the_leaf() {
        let card = compile(
            "  - when: on_play\n    process:\n      - select_trash:\n          of: you\n          bind_as: tamer_pick\n          optional: true\n          filter: { kind: tamer }\n          prompt: \"Play 1 Tamer from your trash\"\n      - if:\n          condition: { binding_absent: tamer_pick }\n          then: []\n          else:\n            - play_from_trash: { of: you, hand_index: tamer_pick, cost_delta: { reduce: 2 } }\n",
        );
        let picks = picks(&card);
        assert_eq!(picks.len(), 1);
        assert_eq!(picks[0].1.can_play_from_effect_without_cost, Some(true));
    }

    #[test]
    fn a_trash_pick_that_is_not_played_is_left_alone() {
        let card = compile(
            "  - when: on_play\n    process:\n      - select_trash:\n          of: you\n          bind_as: to_hand\n          optional: true\n          filter: { kind: digimon }\n          prompt: \"Return 1 Digimon card from your trash to the hand\"\n      - add_to_hand_from_trash: { of: you, card: to_hand }\n",
        );
        let picks = picks(&card);
        assert_eq!(picks.len(), 1);
        assert_eq!(picks[0].1.can_play_from_effect_without_cost, None);
    }

    #[test]
    fn the_pool_guard_sees_a_marked_pick_as_marked() {
        let card = compile(
            "  - when: when_digivolving
    process:
      - select_trash:
          of: you
          bind_as: revive
          optional: true
          filter: { kind: digimon }
          prompt: \"Play 1 Digimon from your trash\"
      - play_from_trash_free: { of: you, hand_index: revive }
",
        );
        assert!(plays_a_pick_from_trash(&card));
        assert!(unmarked_play_from_trash_picks(&card).is_empty());

        // Strip the leaf the pass added: the guard must then name the pick.
        let mut stripped = card.clone();
        for clause in stripped.effects.iter_mut() {
            if let CompiledClause::Triggered(t) = clause {
                for step in t.process.iter_mut() {
                    if let CompiledStep::SelectTrash { filter, .. } = step {
                        filter.can_play_from_effect_without_cost = None;
                    }
                }
            }
        }
        assert_eq!(unmarked_play_from_trash_picks(&stripped), vec!["revive".to_string()]);
    }

    #[test]
    fn an_authored_play_leaf_is_kept_as_authored() {
        let card = compile(
            "  - when: on_play\n    process:\n      - select_trash:\n          of: you\n          bind_as: revive\n          optional: true\n          filter:\n            any_of:\n              - { kind: tamer }\n              - { kind: digimon, can_play_from_effect_without_cost: true }\n          prompt: \"Play 1 card from your trash\"\n      - play_from_trash_free: { of: you, hand_index: revive }\n",
        );
        let picks = picks(&card);
        assert_eq!(picks.len(), 1);
        assert_eq!(
            picks[0].1.can_play_from_effect_without_cost, None,
            "the card authored its own leaf in a branch; the pass adds none on top"
        );
    }
}
