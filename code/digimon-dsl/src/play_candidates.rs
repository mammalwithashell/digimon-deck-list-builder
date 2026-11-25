//! Compile-time "an effect-play pick offers only cards the play would accept" marking.
//!
//! An effect that plays a card it lets the player pick is authored as a selection with a `bind_as`
//! (`select_trash`, `select_hand`, `select_union_zone`, `select_material(s)`, `select_own_sources`,
//! `select_reveal_buckets`, `search_own_security_stack`, ...) followed by a play verb that consumes that
//! binding (`play_from_trash_free`, `play_union_bound_free`, `play_from_materials`,
//! `play_or_use_from_sources`, `play_from_revealed_free`, `play_security_card`, ...). While a "can't play
//! Digimon / Tamers by effects" restriction is active (Crimson Blaze (BT8-097), Pomumon (BT9-047)), a
//! "can't play from the trash" lock holds (Gallantmon (BT23-014)), the battle area is full, or the card can
//! never be played (a DUAL card, an Option, a Digi-Egg: CR 4.3 7-1-1, playing needs a play cost), the play
//! verb refuses the card and the pick did nothing: a dead choice in the action space. DCGO filters the
//! candidates instead -- its scripts end `CanSelectCardCondition` with
//! `CardEffectCommons.CanPlayAsNewPermanent(cardSource, false, activateClass)` (BT24_088.cs, LM_029.cs,
//! EX12_065.cs, EX12_077.cs, BT13_019.cs, BT23_099.cs, BT24_007.cs, the shared `<Decode>` in
//! `CardEffectCommons/KeyWordEffects/Decode.cs`), so under the lock it shows no pick at all. Found in
//! logged games (bug scans 2026-11: Asuna Shiroki (BT24-088), Yellow Scramble (LM-029), Kaguyamon
//! (EX12-065) each offered a prohibited Digimon from the trash; Proximamon (EX12-077) offered the Digi-Egg
//! Nyaromon (EX12-001) from digivolution cards 34 times; seven union-zone picks and a `<Decode>` made under
//! Pomumon (BT9-047) offered cards the play refused).
//!
//! This pass adds a play-legality leaf to the filter of every selection whose binding a play verb
//! consumes, unless the card authored one itself (`can_play_from_effect_without_cost` or
//! `can_play_or_use_from_effect`, anywhere in that filter):
//! - `can_play_from_effect_without_cost: true` when only play verbs consume the binding;
//! - `can_play_or_use_from_effect: true` when the binding is "played or used" (`play_or_use_from_hand`,
//!   `play_or_use_from_sources`, or a play verb and a `use_option_*` verb on the same binding), so a usable
//!   Option stays a candidate (DCGO EX12_077.cs `(IsOption && !CanNotPlayThisOption) || (HasPlayCost &&
//!   CanPlayAsNewPermanent(...))`).
//!
//! A `select_union_zone` gets the leaf on its shared `filter` AND on every per-zone override, because an
//! override replaces the shared filter for its zone. A `select_reveal_buckets` bucket with no filter gets a
//! filter holding only the leaf.
//!
//! Why this is right for EVERY user of these lowerings, not only the cards above: the leaf
//! (`eval_card_fields` -> `Game::can_play_card_from_effect_without_cost_for_controller` + the trash lock)
//! reads exactly the refusals the play verbs make before they move the card -- the trash-play lock, a full
//! battle area, the effect-play lock for the card's kind, and the card-level play prohibition (no play
//! cost, DUAL, the DP ceiling) -- with the same destination (the card's owner) and the same controller (the
//! effect's). So a card the leaf rejects is one the verb is certain to refuse; no playable candidate is
//! ever removed. The leaf does NOT check that a paid play can afford its cost (that is
//! `affordable_with_cost`, authored per card), nor that N simultaneous plays have N free slots.
//!
//! When the leaf leaves no candidate, the selection behaves as it does for any empty candidate set: an
//! optional "you may play" pick is simply not offered and the rest of the effect resolves; a pick marked as
//! a cost (`cost: true`, "By playing ...") cannot be paid and the text after "by" does not run (CR 4.3
//! 15-7-2).
//!
//! The leaves read continuous effects ("can't play by effects"), so the engine settles continuous state
//! before it installs a step whose filter carries one ([`step_reads_play_legality`]; the Eye of the Gorgon
//! (BT9-108) trap, G-ENGINE-PLAY-PICK-STALE-CONTINUOUS-STATE).
//!
//! CR 4.3 does not forbid choosing a card that then can't be played (1-3-2: an impossible action simply
//! isn't performed), so this is a no-dead-choice / DCGO-fidelity rule, the same principle as the
//! digivolve-candidate pass (`digivolve_candidates.rs`) and `qa/dsl-vocab-gaps.md` "Effect-pick candidate
//! legality". G-DSL-PLAY-FROM-TRASH-PICK-LEGALITY, G-DSL-EFFECT-PLAY-PICK-LEGALITY.
//!
//! Out of scope: picks only a `use_option_*` verb consumes (no play), and breeding-area plays
//! (`play_to_breeding_from_*`, which have their own `can_play_to_breeding_*` leaves).

use std::collections::{BTreeMap, BTreeSet};

use crate::compiled::{
    CompiledBindingRef, CompiledCard, CompiledClause, CompiledDeclarativeClause, CompiledPredicate, CompiledStep,
};
use crate::deletion_cap::{child_bodies, child_bodies_mut};

/// Which play-legality leaf a binding needs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Leaf {
    /// Only played: `can_play_from_effect_without_cost`.
    Play,
    /// Played or used: `can_play_or_use_from_effect`.
    PlayOrUse,
}

/// How a step consumes a binding: as a play, a play-or-use, or an Option use.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Consumer {
    Play,
    PlayOrUse,
    Use,
}

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
    let wanted = wanted_leaves(steps);
    if wanted.is_empty() {
        return;
    }
    mark_seq(steps, &wanted);
}

/// The bindings a play verb under `steps` consumes, with the leaf each needs.
fn wanted_leaves(steps: &[CompiledStep]) -> BTreeMap<String, Leaf> {
    let mut consumers: BTreeMap<String, Vec<Consumer>> = BTreeMap::new();
    for step in steps {
        collect(step, &mut consumers);
    }
    consumers
        .into_iter()
        .filter_map(|(name, uses)| {
            if !uses.iter().any(|c| matches!(c, Consumer::Play | Consumer::PlayOrUse)) {
                return None; // only used, never played: out of scope
            }
            let leaf = if uses.iter().any(|c| matches!(c, Consumer::PlayOrUse | Consumer::Use)) {
                Leaf::PlayOrUse
            } else {
                Leaf::Play
            };
            Some((name, leaf))
        })
        .collect()
}

/// The binding a play step names, when it is a pick made by the effect (not `self` / `event_card`).
fn card_binding(b: &CompiledBindingRef) -> Option<&str> {
    match b {
        CompiledBindingRef::Named(n) | CompiledBindingRef::Binding(n) => Some(n.as_str()),
        _ => None,
    }
}

/// The binding `step` plays or uses, and how. Every effect play verb that consumes a pick is listed here;
/// `PLAY_VERBS` below is its serialized twin for the pool guard.
fn consumed(step: &CompiledStep) -> Option<(&str, Consumer)> {
    use CompiledStep as S;
    match step {
        S::PlayFromHand { hand_index: b, .. }
        | S::PlayFromHandFree { hand_index: b, .. }
        | S::PlayFromTrash { trash_index: b, .. }
        | S::PlayFromTrashFree { trash_index: b, .. }
        | S::PlayFromRevealedFree { card: b, .. }
        | S::PlayFromMaterials { source_index: b, .. }
        | S::PlayCardsFree { cards: b }
        | S::PlaySecurityCard { card: b, .. } => card_binding(b).map(|n| (n, Consumer::Play)),
        S::PlayUnionBoundFree { binding, .. } => Some((binding.as_str(), Consumer::Play)),
        S::PlaySelectedSourcesFree { source_refs } | S::PlayUnderTamerSource { source_refs, .. } => {
            Some((source_refs.as_str(), Consumer::Play))
        }
        S::PlayOrUseFromHand { hand_index: b, .. } | S::PlayOrUseFromSources { card: b, .. } => {
            card_binding(b).map(|n| (n, Consumer::PlayOrUse))
        }
        S::UseOptionFromRevealed { card: b, .. } | S::UseOptionFromSources { card: b, .. } => {
            card_binding(b).map(|n| (n, Consumer::Use))
        }
        S::UseOptionBound { binding, .. } => Some((binding.as_str(), Consumer::Use)),
        _ => None,
    }
}

fn collect(step: &CompiledStep, out: &mut BTreeMap<String, Vec<Consumer>>) {
    if let Some((name, how)) = consumed(step) {
        out.entry(name.to_string()).or_default().push(how);
    }
    for body in child_bodies(step) {
        for s in body {
            collect(s, out);
        }
    }
}

fn add_leaf(filter: &mut CompiledPredicate, leaf: Leaf) {
    if has_play_leaf(filter) {
        return;
    }
    match leaf {
        Leaf::Play => filter.can_play_from_effect_without_cost = Some(true),
        Leaf::PlayOrUse => filter.can_play_or_use_from_effect = Some(true),
    }
}

fn mark_seq(steps: &mut [CompiledStep], wanted: &BTreeMap<String, Leaf>) {
    use CompiledStep as S;
    let leaf_for = |bind_as: &Option<String>| bind_as.as_deref().and_then(|b| wanted.get(b)).copied();
    for step in steps.iter_mut() {
        match step {
            S::SelectHand { bind_as, filter, .. }
            | S::SelectTrash { bind_as, filter, .. }
            | S::SelectReveal { bind_as, filter, .. }
            | S::SelectSecurity { bind_as, filter, .. }
            | S::SelectMaterial { bind_as, filter, .. }
            | S::SelectMaterials { bind_as, filter, .. }
            | S::SelectOwnSources { bind_as, filter, .. }
            | S::SelectOpponentSources { bind_as, filter, .. }
            | S::SelectZoneCards { bind_as, filter, .. }
            | S::SelectCountCappedMulti { bind_as, filter, .. } => {
                if let Some(leaf) = leaf_for(bind_as) {
                    add_leaf(filter, leaf);
                }
            }
            S::SearchOwnSecurityStack { bind_as, filter, .. } => {
                if let Some(leaf) = leaf_for(bind_as) {
                    add_leaf(filter, leaf);
                }
            }
            S::SelectUnionZone { bind_as, filter, zone_filters, .. } => {
                if let Some(leaf) = leaf_for(bind_as) {
                    add_leaf(filter, leaf);
                    for f in [&mut zone_filters.hand, &mut zone_filters.trash, &mut zone_filters.material]
                        .into_iter()
                        .flatten()
                    {
                        add_leaf(f, leaf);
                    }
                }
            }
            S::SelectRevealBuckets { buckets, .. } => {
                for bucket in buckets.iter_mut() {
                    if let Some(&leaf) = wanted.get(bucket.bind_as.as_str()) {
                        add_leaf(bucket.filter.get_or_insert_with(CompiledPredicate::default), leaf);
                    }
                }
            }
            _ => {}
        }
        for body in child_bodies_mut(step) {
            mark_seq(body, wanted);
        }
    }
}

/// Whether `filter` authors a play-legality leaf anywhere (itself, a conjunct, a disjunct, or under a negation).
/// An authored leaf is the card's own statement of which candidates are playable, so the pass leaves it alone.
pub fn has_play_leaf(filter: &CompiledPredicate) -> bool {
    filter.can_play_from_effect_without_cost.is_some()
        || filter.can_play_or_use_from_effect.is_some()
        || filter.all_of.iter().any(has_play_leaf)
        || filter.any_of.iter().any(has_play_leaf)
        || filter.none_of.iter().any(has_play_leaf)
        || filter.not.as_deref().is_some_and(has_play_leaf)
}

/// Whether `step`'s own candidate filters (not its child bodies) read play legality, so the engine must
/// settle continuous state before it evaluates them: a "can't play by effects" lock is a continuous effect
/// that an earlier step of the same effect may have removed (Eye of the Gorgon (BT9-108) deletes Pillomon
/// (BT9-033), then offers the pick). Used by the engine's step dispatcher.
pub fn step_reads_play_legality(step: &CompiledStep) -> bool {
    use CompiledStep as S;
    match step {
        S::SelectHand { filter, .. }
        | S::SelectTrash { filter, .. }
        | S::SelectReveal { filter, .. }
        | S::SelectSecurity { filter, .. }
        | S::SelectMaterial { filter, .. }
        | S::SelectMaterials { filter, .. }
        | S::SelectOwnSources { filter, .. }
        | S::SelectOpponentSources { filter, .. }
        | S::SelectZoneCards { filter, .. }
        | S::SelectCountCappedMulti { filter, .. } => has_play_leaf(filter),
        S::SearchOwnSecurityStack { filter, .. } => has_play_leaf(filter),
        S::SelectUnionZone { filter, zone_filters, .. } => {
            has_play_leaf(filter)
                || [&zone_filters.hand, &zone_filters.trash, &zone_filters.material]
                    .into_iter()
                    .flatten()
                    .any(has_play_leaf)
        }
        S::SelectRevealBuckets { buckets, .. } => {
            buckets.iter().filter_map(|b| b.filter.as_ref()).any(has_play_leaf)
        }
        _ => false,
    }
}

/// The serialized step names that play a picked card, with the field naming the binding: the serialized
/// twin of [`consumed`]'s play arms, read by the pool guard.
const PLAY_VERBS: &[(&str, &str)] = &[
    ("PlayFromHand", "hand_index"),
    ("PlayFromHandFree", "hand_index"),
    ("PlayOrUseFromHand", "hand_index"),
    ("PlayOrUseFromSources", "card"),
    ("PlayFromRevealedFree", "card"),
    ("PlayFromTrash", "trash_index"),
    ("PlayFromTrashFree", "trash_index"),
    ("PlayUnionBoundFree", "binding"),
    ("PlayFromMaterials", "source_index"),
    ("PlaySelectedSourcesFree", "source_refs"),
    ("PlayUnderTamerSource", "source_refs"),
    ("PlayCardsFree", "cards"),
    ("PlaySecurityCard", "card"),
];

/// Pool guard: the picks of `card` whose binding a play verb consumes and whose candidate filter carries no
/// play-legality leaf after compilation, as binding names. Empty for every card the pass reaches.
///
/// A pick is any object with a `bind_as` naming a played binding and a `filter` (a selection step or a
/// reveal bucket); every filter it applies -- the shared one and each per-zone override -- must carry a
/// leaf. It walks the SERIALIZED card rather than the typed tree on purpose: the pass above reaches the step
/// bodies `deletion_cap::child_bodies` knows and the selection variants it lists, and this check must also
/// see a pick in a body or a selection variant a new step adds before the pass learns about it.
pub fn unmarked_play_picks(card: &CompiledCard) -> Vec<String> {
    use serde_json::Value;

    fn binding_name(v: &Value) -> Option<&str> {
        v.as_str()
            .or_else(|| ["Named", "Binding"].iter().find_map(|tag| v.get(*tag).and_then(Value::as_str)))
    }

    fn played(value: &Value, out: &mut BTreeSet<String>) {
        match value {
            Value::Object(map) => {
                for (key, child) in map {
                    if let Some((_, field)) = PLAY_VERBS.iter().find(|(verb, _)| verb == key) {
                        if let Some(name) = binding_name(&child[*field]) {
                            out.insert(name.to_string());
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
                if let (Some(binding), Some(filter)) =
                    (map.get("bind_as").and_then(Value::as_str), map.get("filter"))
                {
                    if wanted.contains(binding) {
                        let overrides_ok = map
                            .get("zone_filters")
                            .and_then(Value::as_object)
                            .is_none_or(|zones| zones.values().all(|f| f.is_null() || leaf(f)));
                        if !leaf(filter) || !overrides_ok {
                            out.push(binding.to_string());
                        }
                    }
                }
                for child in map.values() {
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

/// Whether any clause of `card` plays a picked card (for the pool guard's "did the walk see anything"
/// sanity check).
pub fn plays_a_pick(card: &CompiledCard) -> bool {
    let any = |steps: &[CompiledStep]| !wanted_leaves(steps).is_empty();
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
        assert!(plays_a_pick(&card));
        assert!(unmarked_play_picks(&card).is_empty());

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
        assert_eq!(unmarked_play_picks(&stripped), vec!["revive".to_string()]);
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

    /// The serialized object that binds `binding` (a selection step or a reveal bucket).
    fn pick_json(card: &CompiledCard, binding: &str) -> serde_json::Value {
        fn find(v: &serde_json::Value, binding: &str) -> Option<serde_json::Value> {
            match v {
                serde_json::Value::Object(m) => {
                    if m.get("bind_as").and_then(serde_json::Value::as_str) == Some(binding) && m.contains_key("filter")
                    {
                        return Some(v.clone());
                    }
                    m.values().find_map(|c| find(c, binding))
                }
                serde_json::Value::Array(a) => a.iter().find_map(|c| find(c, binding)),
                _ => None,
            }
        }
        card.effects
            .iter()
            .find_map(|c| find(&serde_json::to_value(c).unwrap(), binding))
            .unwrap_or_else(|| panic!("no pick binds `{binding}`"))
    }

    fn top_leaf(filter: &serde_json::Value, leaf: &str) -> bool {
        filter.get(leaf) == Some(&serde_json::Value::Bool(true))
    }

    /// Proximamon (EX12-077): "play or use 1 ... card from your Digimon's digivolution cards". A Digi-Egg
    /// (Nyaromon (EX12-001)) has no play cost (CR 4.3 7-1-1) and must not be offered; a usable Option must.
    #[test]
    fn play_or_use_source_pick_gets_the_play_or_use_leaf() {
        let card = compile(
            "  - when: when_digivolving
    process:
      - select_own_sources:
          min: 0
          max: 1
          bind_as: source_card
          filter: { play_or_use_cost_lte: 10 }
          prompt: \"You may play or use 1 source\"
          then:
            - play_or_use_from_sources: { of: you, card: source_card, cost: free }
",
        );
        let pick = pick_json(&card, "source_card");
        assert!(top_leaf(&pick["filter"], "can_play_or_use_from_effect"), "{pick}");
        assert!(pick["filter"]["can_play_from_effect_without_cost"].is_null(), "Options stay candidates: {pick}");
    }

    /// A union pick gets the leaf on its shared filter AND on each per-zone override (an override replaces the
    /// shared filter for its zone): Gankoomon (BT13-019).
    #[test]
    fn union_pick_gets_the_leaf_on_its_filter_and_every_override() {
        let card = compile(
            "  - when: on_play
    process:
      - select_union_zone:
          of: you
          zones: [hand, trash]
          optional: true
          bind_as: pick
          filter: { kind: digimon }
          zone_filters:
            trash: { kind: digimon, level_lte: 4 }
          prompt: \"Play 1 Digimon from your hand or trash\"
      - play_union_bound_free: { binding: pick }
",
        );
        let pick = pick_json(&card, "pick");
        assert!(top_leaf(&pick["filter"], "can_play_from_effect_without_cost"), "{pick}");
        assert!(top_leaf(&pick["zone_filters"]["trash"], "can_play_from_effect_without_cost"), "{pick}");
        assert!(pick["zone_filters"]["hand"].is_null(), "no override is invented: {pick}");
    }

    /// `<Decode>`-shaped pick: `select_material` feeding `play_from_materials` (WereGarurumon (EX12-032)).
    #[test]
    fn material_pick_feeding_play_from_materials_gets_the_leaf() {
        let card = compile(
            "  - when: when_digivolving
    process:
      - select_material:
          of_permanent: self
          bind_as: decode_source
          optional: true
          filter: { kind: digimon, level_lte: 4 }
          prompt: \"Play 1 level 4 or lower source\"
      - play_from_materials:
          target: self
          source_index: decode_source
          cost_delta: free
",
        );
        let pick = pick_json(&card, "decode_source");
        assert!(top_leaf(&pick["filter"], "can_play_from_effect_without_cost"), "{pick}");
    }

    /// A reveal bucket that is played (and, on another branch, used) gets the play-or-use leaf even when it
    /// was authored with no filter.
    #[test]
    fn reveal_bucket_feeding_play_and_use_gets_the_play_or_use_leaf() {
        let card = compile(
            "  - when: on_play
    process:
      - reveal_top_deck: { of: you, count: 3, bind_as: revealed }
      - select_reveal_buckets:
          from: revealed
          buckets:
            - bind_as: copied
              min: 0
              max: 1
          prompt: \"Play or use 1 card among them\"
      - if:
          condition: { binding_card_kind: { binding: copied, kind: option } }
          then:
            - use_option_from_revealed: { of: you, card: copied, cost: free }
          else:
            - play_from_revealed_free: { of: you, card: copied }
",
        );
        let pick = pick_json(&card, "copied");
        assert!(top_leaf(&pick["filter"], "can_play_or_use_from_effect"), "{pick}");
    }

    /// The pool guard checks every filter a union pick applies: a per-zone override without the leaf is
    /// named even when the shared filter carries one.
    #[test]
    fn the_pool_guard_names_a_union_override_without_the_leaf() {
        let card = compile(
            "  - when: on_play
    process:
      - select_union_zone:
          of: you
          zones: [hand, trash]
          optional: true
          bind_as: pick
          filter: { kind: digimon }
          zone_filters:
            trash: { kind: digimon, level_lte: 4 }
          prompt: \"Play 1 Digimon from your hand or trash\"
      - play_union_bound_free: { binding: pick }
",
        );
        assert!(plays_a_pick(&card));
        assert!(unmarked_play_picks(&card).is_empty());
        let mut stripped = card.clone();
        for clause in stripped.effects.iter_mut() {
            if let CompiledClause::Triggered(t) = clause {
                for step in t.process.iter_mut() {
                    if let CompiledStep::SelectUnionZone { zone_filters, .. } = step {
                        if let Some(f) = zone_filters.trash.as_mut() {
                            f.can_play_from_effect_without_cost = None;
                        }
                    }
                }
            }
        }
        assert_eq!(unmarked_play_picks(&stripped), vec!["pick".to_string()]);
    }

    /// A pick only an Option-use verb consumes is out of scope (no play).
    #[test]
    fn a_use_only_pick_is_left_alone() {
        let card = compile(
            "  - when: on_play
    process:
      - reveal_top_deck: { of: you, count: 3, bind_as: revealed }
      - select_reveal_buckets:
          from: revealed
          buckets:
            - bind_as: picked
              min: 0
              max: 1
              filter: { kind: option }
          prompt: \"Use 1 Option among them\"
      - use_option_from_revealed: { of: you, card: picked, cost: free }
",
        );
        let pick = pick_json(&card, "picked");
        assert!(pick["filter"]["can_play_or_use_from_effect"].is_null(), "{pick}");
        assert!(pick["filter"]["can_play_from_effect_without_cost"].is_null(), "{pick}");
    }

    /// A security search whose pick is played (GeoGreymon (BT13-012) shape).
    #[test]
    fn security_search_pick_feeding_play_security_card_gets_the_leaf() {
        let card = compile(
            "  - when: when_digivolving
    process:
      - search_own_security_stack:
          bind_as: sec_tamer
          optional: true
          on_select: []
          filter: { kind: tamer }
          prompt: \"Play 1 Tamer from security\"
      - if:
          condition: { binding_present: sec_tamer }
          then:
            - play_security_card: { of: you, card: sec_tamer }
",
        );
        let pick = pick_json(&card, "sec_tamer");
        assert!(top_leaf(&pick["filter"], "can_play_from_effect_without_cost"), "{pick}");
    }
}
