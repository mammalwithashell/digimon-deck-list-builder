//! Named-binding environment for DSL process steps.

use std::collections::{BTreeSet, HashMap};

use crate::card_source::CardHandle;
use crate::enums::PlayerId;
use crate::permanent::PermanentHandle;
use crate::selection::{BreedingPermanentSelectionRef, SourceSelectionRef, UnionZoneOrigin};
use crate::trigger_context::ProvenanceToken;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BindingValue {
    Permanent(PermanentHandle),
    Card(CardHandle),
    HandIndex(PlayerId, u16),
    TrashIndex(PlayerId, u16),
    Literal(i64),
    PermanentList(Vec<PermanentHandle>),
    CardList(Vec<CardHandle>),
    SourceRefs(Vec<SourceSelectionRef>),
    BreedingPermanentRef(BreedingPermanentSelectionRef),
    /// A card picked via `select_union_zone` (hand ∪ trash ∪ material), tagged with the
    /// `origin` zone it was in and the `owner` of that zone. Carrying the
    /// origin lets a downstream consumer (`play_union_bound_free`) replay
    /// the card from its *true* zone — a bare `CardHandle` cannot.
    UnionCard {
        card: CardHandle,
        origin: UnionZoneOrigin,
        owner: PlayerId,
    },
    /// A permanent binding produced by a *play verb* (`play_from_hand_free`,
    /// `play_from_revealed_free`, `play_from_materials`, `play_union_bound_free`,
    /// `play_token`). Unlike `Permanent`, this variant tracks the played card
    /// by stable identity (`ProvenanceToken` derived from the played card's
    /// `CardHandle`) and is resolved at consume time via the engine, not at
    /// bind time. The strict resolver
    /// [`crate::game::Game::resolve_token_as_battle_area_top`] yields a
    /// `PermanentHandle` only when the played card is still the top card of a
    /// battle-area permanent; the permissive sibling
    /// [`crate::game::Game::resolve_provenance_token`] yields the carrier
    /// handle for cards anywhere in a stack (top or digivolution card).
    ///
    /// The binding silently fails to resolve once the played card is no
    /// longer a battle-area top (strict path) — i.e. once it has been
    /// consumed in DNA digivolve, buried under a new top by regular
    /// digivolve, or removed from play entirely. See change
    /// `fix-played-binding-uses-provenance`.
    ///
    /// `fallback` carries the schedule-time `PermanentHandle` for
    /// diagnostics only. Consumers MUST always route through provenance
    /// resolution and treat resolution failure as a silent no-op; `fallback`
    /// is never used as a substitute for the resolver in production code.
    PlayedPermanent {
        token: ProvenanceToken,
        fallback: PermanentHandle,
    },
}

/// Reserved slot key holding a trash binding's stable-identity pin.
fn hand_pin_key(name: &str) -> String {
    format!("\u{0}hand_pin:{name}")
}

fn trash_pin_key(name: &str) -> String {
    format!("\u{0}trash_pin:{name}")
}

pub(crate) fn is_private_link_pin_name(name:&str)->bool {
    name.starts_with("\0link_source_carrier:") || name.starts_with("\0link_material_carrier:")
}

#[derive(Default, Clone)]
pub struct Bindings {
    slots: HashMap<String, BindingValue>,
    // Explicit receipt clears must survive overlays of pre-selection snapshots.
    // Ordinary `remove` remains local; only `clear_for_merge` creates a tombstone.
    cleared_slots: BTreeSet<String>,
    result_log: EffectResultLog,
    played_identity_pins: HashMap<String, (PlayerId, u64)>,
    // Singular battle-area selections retain the permanent, not its field slot
    // or top card. Digivolving that same permanent does not invalidate the pin.
    selected_identity_pins: HashMap<String, (PlayerId, u64)>,
}

// Some archival verification paths debug enclosing declarations. Preserve the
// pre-selected-pin field projection there; current verification hashes the new
// identity metadata explicitly, in sorted order and with continuation paths.
impl std::fmt::Debug for Bindings {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let cleared: BTreeSet<_>=self.cleared_slots.iter().filter(|name|!is_private_link_pin_name(name)).collect();
        f.debug_struct("Bindings")
            .field("slots", &self.slots)
            .field("cleared_slots", &cleared)
            .field("result_log", &self.result_log)
            .field("played_identity_pins", &self.played_identity_pins)
            .finish()
    }
}

#[derive(Debug, Default, Clone)]
pub struct EffectResultLog {
    pub suspended: Vec<PermanentHandle>,
    pub returned_cards: Vec<CardHandle>,
    pub returned_to_deck: Vec<CardHandle>,
    /// Completed permanent deletions attributed to THIS effect. The receipt
    /// captures effective DP at the actual removal boundary, before modifiers
    /// are cleared. Rows include non-Digimon; kind-sensitive predicates consult
    /// the parallel classification instead of inferring card kind from DP.
    pub deleted: Vec<(PermanentHandle, Option<i32>)>,
    /// Parallel classification captured before removal, never inferred from DP.
    pub deleted_is_digimon: Vec<bool>,
    pub played: Vec<PermanentHandle>,
    pub digivolved: Vec<PermanentHandle>,
    pub added_to_hand: Vec<CardHandle>,
    /// Cards trashed FROM A HAND by this effect (`trash_from_hand_by_index`,
    /// hand-origin `trash_union_bound`). Read by the
    /// `effect_trashed_any_hand_card` / `trashed_hand_card_matching` result
    /// predicates (P-212). G-DSL-EFFECT-TRASHED-HAND-CARD.
    pub trashed_from_hand: Vec<CardHandle>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EffectResultLogCursor {
    suspended: usize,
    returned_cards: usize,
    returned_to_deck: usize,
    deleted: usize,
    played: usize,
    digivolved: usize,
    added_to_hand: usize,
    trashed_from_hand: usize,
}

impl Bindings {
    pub(crate) fn belief_safe(&self, viewer: PlayerId, allowed: &std::collections::HashSet<CardHandle>) -> bool {
        self.slots.values().all(|v| match v {
            BindingValue::Card(h) | BindingValue::UnionCard { card:h, .. } => allowed.contains(h),
            BindingValue::CardList(h) => h.iter().all(|h| allowed.contains(h)),
            BindingValue::HandIndex(owner, _) => *owner == viewer,
            _ => true,
        }) && self.result_log.returned_cards.iter().chain(&self.result_log.returned_to_deck)
            .chain(&self.result_log.added_to_hand).chain(&self.result_log.trashed_from_hand)
            .all(|h| allowed.contains(h))
    }
    /// Stable projection for new typed declarations; archival Debug stays unchanged.
    pub(crate) fn canonical_verification_debug(&self) -> String {
        let slots: std::collections::BTreeMap<_, _> = self.slots.iter().collect();
        let played: std::collections::BTreeMap<_, _> = self.played_identity_pins.iter().collect();
        let selected: std::collections::BTreeMap<_, _> = self.selected_identity_pins.iter().collect();
        format!("{:?}", (slots, &self.cleared_slots, &self.result_log, played, selected))
    }
    /// V3 DNA declarations predate the private Link continuation namespace.
    pub(crate) fn legacy_canonical_verification_debug(&self)->String {
        let mut legacy=self.clone();
        legacy.selected_identity_pins.retain(|name,_|!is_private_link_pin_name(name));
        legacy.cleared_slots.retain(|name|!is_private_link_pin_name(name));
        legacy.canonical_verification_debug()
    }
    pub(crate) fn private_link_clears_for_verification(&self)->Vec<String> {
        self.cleared_slots.iter().filter(|name|is_private_link_pin_name(name)).cloned().collect()
    }
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, name: &str, value: BindingValue) {
        self.cleared_slots.remove(name);
        self.played_identity_pins.remove(name);
        self.selected_identity_pins.remove(name);
        // Re-binding a name drops any stable-identity pin from a previous
        // trash pick under the same name.
        self.slots.remove(&trash_pin_key(name));
        self.slots.remove(&hand_pin_key(name));
        self.slots.insert(name.to_string(), value);
    }

    pub(crate) fn remove(&mut self, name: &str) {
        self.slots.remove(name);
        self.slots.remove(&trash_pin_key(name));
        self.slots.remove(&hand_pin_key(name));
        self.played_identity_pins.remove(name);
        self.selected_identity_pins.remove(name);
    }
    /// Publish an explicit absence through nested continuation snapshots.
    /// Unlike a missing key, this clears an older value during an overlay.
    pub(crate) fn clear_for_merge(&mut self, name: &str) {
        self.remove(name);
        self.cleared_slots.insert(name.to_owned());
    }

    pub fn get(&self, name: &str) -> Option<BindingValue> {
        self.get_ref(name).cloned()
    }

    /// Overlay every named slot from `other` onto self (`other` wins on
    /// conflict). Used when resuming a parked outer tail: the inner
    /// resolution's bindings are the park-time snapshot's lineage plus any
    /// picks made since, and the outer tail must see those picks — a sibling
    /// `binding_exists` / `binding_absent` after a nested select would
    /// otherwise read a stale absence (G-OPT-REFUND-ON-DECLINE fallout).
    pub fn merge_slots_from(&mut self, other: &Bindings) {
        for name in &other.cleared_slots {
            self.clear_for_merge(name);
        }
        for (k, v) in &other.slots {
            self.cleared_slots.remove(k);
            self.slots.insert(k.clone(), v.clone());
            self.played_identity_pins.remove(k);
            if let Some(pin) = other.played_identity_pins.get(k) {
                self.played_identity_pins.insert(k.clone(), *pin);
            }
            self.selected_identity_pins.remove(k);
            if let Some(pin) = other.selected_identity_pins.get(k) {
                self.selected_identity_pins.insert(k.clone(), *pin);
            }
        }
        for (name, pin) in &other.selected_identity_pins {
            if (name.starts_with("\0link_source_carrier:") || name.starts_with("\0link_material_carrier:"))
                && !other.cleared_slots.contains(name) && !other.slots.contains_key(name) {
                self.cleared_slots.remove(name);
                self.selected_identity_pins.insert(name.clone(), *pin);
            }
        }
    }

    /// Borrowing variant of `get`. Prefer this when the caller only needs to
    /// inspect the value — avoids cloning the inner `Vec` for `PermanentList`
    /// / `CardList`. `get` remains the convenience method for callers that
    /// need to take ownership (e.g. `ResolvedBinding::PermanentList(v)`
    /// destructuring in `binding_ref::resolve_named`).
    pub fn get_ref(&self, name: &str) -> Option<&BindingValue> {
        self.slots.get(name)
    }

    pub fn result_log(&self) -> &EffectResultLog {
        &self.result_log
    }

    pub fn result_log_cursor(&self) -> EffectResultLogCursor {
        EffectResultLogCursor {
            suspended: self.result_log.suspended.len(),
            returned_cards: self.result_log.returned_cards.len(),
            returned_to_deck: self.result_log.returned_to_deck.len(),
            deleted: self.result_log.deleted.len(),
            played: self.result_log.played.len(),
            digivolved: self.result_log.digivolved.len(),
            added_to_hand: self.result_log.added_to_hand.len(),
            trashed_from_hand: self.result_log.trashed_from_hand.len(),
        }
    }

    pub fn record_suspended(&mut self, handle: PermanentHandle) {
        self.result_log.suspended.push(handle);
    }

    pub fn record_returned_to_deck(&mut self, card: CardHandle) {
        self.record_returned_card(card);
        self.result_log.returned_to_deck.push(card);
    }

    pub fn record_returned_card(&mut self, card: CardHandle) {
        self.result_log.returned_cards.push(card);
    }

    /// Record a permanent deleted by this effect alongside its pre-removal
    /// effective DP (captured by the caller BEFORE the carrier moves to trash;
    /// `None` when the deleted permanent had no DP). See `EffectResultLog::deleted`.
    pub fn record_deleted(&mut self, handle: PermanentHandle, dp: Option<i32>, is_digimon: bool) {
        self.result_log.deleted.push((handle, dp));
        self.result_log.deleted_is_digimon.push(is_digimon);
    }

    pub fn record_played(&mut self, handle: PermanentHandle) {
        self.result_log.played.push(handle);
    }

    pub fn record_digivolved(&mut self, handle: PermanentHandle) {
        self.result_log.digivolved.push(handle);
    }

    pub fn record_added_to_hand(&mut self, card: CardHandle) {
        self.result_log.added_to_hand.push(card);
    }

    /// Record a card this effect trashed from a hand. Consumed by the
    /// `effect_trashed_any_hand_card` / `trashed_hand_card_matching` result
    /// predicates. G-DSL-EFFECT-TRASHED-HAND-CARD.
    pub fn record_trashed_from_hand(&mut self, card: CardHandle) {
        self.result_log.trashed_from_hand.push(card);
    }

    pub fn merge_result_log_from_since(&mut self, other: &Bindings, cursor: EffectResultLogCursor) {
        self.result_log.suspended.extend(
            other.result_log.suspended[cursor.suspended..]
                .iter()
                .copied(),
        );
        self.result_log.returned_cards.extend(
            other.result_log.returned_cards[cursor.returned_cards..]
                .iter()
                .copied(),
        );
        self.result_log.returned_to_deck.extend(
            other.result_log.returned_to_deck[cursor.returned_to_deck..]
                .iter()
                .copied(),
        );
        self.result_log
            .deleted
            .extend(other.result_log.deleted[cursor.deleted..].iter().copied());
        self.result_log.deleted_is_digimon.extend(
            other.result_log.deleted_is_digimon[cursor.deleted..]
                .iter()
                .copied(),
        );
        self.result_log
            .played
            .extend(other.result_log.played[cursor.played..].iter().copied());
        self.result_log.digivolved.extend(
            other.result_log.digivolved[cursor.digivolved..]
                .iter()
                .copied(),
        );
        self.result_log.added_to_hand.extend(
            other.result_log.added_to_hand[cursor.added_to_hand..]
                .iter()
                .copied(),
        );
        self.result_log.trashed_from_hand.extend(
            other.result_log.trashed_from_hand[cursor.trashed_from_hand..]
                .iter()
                .copied(),
        );
    }

    pub fn get_permanent(&self, name: &str) -> Option<PermanentHandle> {
        match self.get(name)? {
            BindingValue::Permanent(h) => Some(h),
            _ => None,
        }
    }

    pub fn get_card(&self, name: &str) -> Option<CardHandle> {
        match self.get(name)? {
            BindingValue::Card(h) => Some(h),
            _ => None,
        }
    }

    pub fn get_hand_index(&self, name: &str) -> Option<u16> {
        match self.get(name)? {
            BindingValue::HandIndex(_, i) => Some(i),
            _ => None,
        }
    }

    pub fn get_hand_index_with_player(&self, name: &str) -> Option<(PlayerId, u16)> {
        match self.get(name)? {
            BindingValue::HandIndex(p, i) => Some((p, i)),
            _ => None,
        }
    }

    pub fn get_trash_index(&self, name: &str) -> Option<u16> {
        match self.get(name)? {
            BindingValue::TrashIndex(_, i) => Some(i),
            _ => None,
        }
    }

    pub fn get_trash_index_with_player(&self, name: &str) -> Option<(PlayerId, u16)> {
        match self.get(name)? {
            BindingValue::TrashIndex(p, i) => Some((p, i)),
            _ => None,
        }
    }

    pub fn get_literal(&self, name: &str) -> Option<i64> {
        match self.get(name)? {
            BindingValue::Literal(v) => Some(v),
            _ => None,
        }
    }

    /// Every named plain `Permanent` binding (reserved pin slots excluded).
    pub(crate) fn permanent_slots(&self) -> Vec<(String, PermanentHandle)> {
        let mut out: Vec<_> = self
            .slots
            .iter()
            .filter(|(k, _)| !k.starts_with('\u{0}'))
            .filter_map(|(k, v)| match v {
                BindingValue::Permanent(h) => Some((k.clone(), *h)),
                _ => None,
            })
            .collect();
        out.sort_by(|a, b| a.0.cmp(&b.0));
        out
    }

    pub fn insert_permanent(&mut self, name: &str, h: PermanentHandle) {
        self.insert(name, BindingValue::Permanent(h));
    }

    /// Bind a selected battle-area permanent by stable identity. The raw slot
    /// remains available for legacy consumers; `resolve_binding_ref` must use
    /// the pin and never fall back to that slot when the permanent has left.
    pub(crate) fn insert_selected_permanent_with_identity(
        &mut self,
        name: &str,
        handle: PermanentHandle,
        identity: u64,
    ) {
        self.insert_permanent(name, handle);
        self.selected_identity_pins
            .insert(name.to_owned(), (handle.player, identity));
    }

    pub(crate) fn selected_identity_pin(&self, name: &str) -> Option<(PlayerId, u64)> {
        self.selected_identity_pins.get(name).copied()
    }

    /// Private link continuation metadata, never an authored binding value.
    pub(crate) fn pin_private_link_identity(&mut self, name: &str, player: PlayerId, identity: u64) {
        assert!(name.starts_with("\0link_source_carrier:") || name.starts_with("\0link_material_carrier:"));
        self.cleared_slots.remove(name);
        self.slots.remove(name);
        self.played_identity_pins.remove(name);
        self.selected_identity_pins.insert(name.to_owned(), (player, identity));
    }

    pub(crate) fn selected_identity_pins_for_verification(&self) -> Vec<(String, PlayerId, u64)> {
        let mut pins: Vec<_> = self.selected_identity_pins.iter()
            .map(|(name, (player, identity))| (name.clone(), *player, *identity))
            .collect();
        pins.sort_unstable();
        pins
    }

    /// Insert a played-permanent binding tagged with stable identity
    /// (`ProvenanceToken` for the played top card). See
    /// [`BindingValue::PlayedPermanent`] for semantics; called by play verbs
    /// (`PlayFromHandFree`, `PlayFromRevealedFree`, `PlayFromMaterials`,
    /// `PlayUnionBoundFree`, `PlayToken`) at their `bind_as` insert sites.
    pub fn insert_played_permanent(
        &mut self,
        name: &str,
        token: ProvenanceToken,
        fallback: PermanentHandle,
    ) {
        self.insert(name, BindingValue::PlayedPermanent { token, fallback });
    }

    pub(crate) fn insert_played_permanent_with_identity(
        &mut self,
        name: &str,
        token: ProvenanceToken,
        fallback: PermanentHandle,
        identity: u64,
    ) {
        self.insert_played_permanent(name, token, fallback);
        self.played_identity_pins
            .insert(name.to_owned(), (fallback.player, identity));
    }

    pub(crate) fn played_identity_pin(&self, name: &str) -> Option<(PlayerId, u64)> {
        self.played_identity_pins.get(name).copied()
    }

    /// Diagnostic getter for a played-permanent binding. Returns the raw
    /// `(token, fallback)` pair WITHOUT resolving the token. Production
    /// consumers MUST NOT use this — they SHALL route through
    /// `resolve_binding_ref` (strict) or `resolve_played_permanent_permissive`
    /// (carrier-aware). Intended for tests and tracing only.
    pub fn get_played_permanent(&self, name: &str) -> Option<(ProvenanceToken, PermanentHandle)> {
        match self.get(name)? {
            BindingValue::PlayedPermanent { token, fallback } => Some((token, fallback)),
            _ => None,
        }
    }

    pub fn insert_card(&mut self, name: &str, h: CardHandle) {
        self.insert(name, BindingValue::Card(h));
    }

    /// Record a union-zone (hand ∪ trash) pick together with its origin zone
    /// and owner. Used by `select_union_zone`'s success callback so a later
    /// `play_union_bound_free` step can replay the card from its true zone.
    pub fn insert_union_card(
        &mut self,
        name: &str,
        card: CardHandle,
        origin: UnionZoneOrigin,
        owner: PlayerId,
    ) {
        self.insert(
            name,
            BindingValue::UnionCard {
                card,
                origin,
                owner,
            },
        );
    }

    /// Fetch a union-zone pick `(card, origin, owner)`. Returns `None` if the
    /// binding is absent or holds a different `BindingValue` kind.
    pub fn get_union_card(&self, name: &str) -> Option<(CardHandle, UnionZoneOrigin, PlayerId)> {
        match self.get(name)? {
            BindingValue::UnionCard {
                card,
                origin,
                owner,
            } => Some((card, origin, owner)),
            _ => None,
        }
    }

    pub fn insert_hand_index(&mut self, name: &str, player: PlayerId, i: u16) {
        self.insert(name, BindingValue::HandIndex(player, i));
    }

    /// Keep the public HandIndex shape, with stable identity in a reserved slot.
    /// Like trash pins, this survives cloned continuations and slot merges.
    pub fn insert_hand_index_pinned(
        &mut self,
        name: &str,
        player: PlayerId,
        i: u16,
        card: Option<CardHandle>,
    ) {
        self.insert_hand_index(name, player, i);
        if let Some(card) = card {
            self.slots
                .insert(hand_pin_key(name), BindingValue::Card(card));
        }
    }

    pub fn hand_pin(&self, name: &str) -> Option<CardHandle> {
        match self.slots.get(&hand_pin_key(name))? {
            BindingValue::Card(card) => Some(*card),
            _ => None,
        }
    }

    pub fn resolve_pinned_hand_index(
        &self,
        name: &str,
        hand: &[crate::card_source::CardSource],
    ) -> Option<Option<u16>> {
        let pin = self.hand_pin(name)?;
        Some(
            hand.iter()
                .position(|card| card.handle() == pin)
                .map(|i| i as u16),
        )
    }

    pub fn insert_hand_index_for(&mut self, name: &str, player: PlayerId, i: u16) {
        self.insert_hand_index(name, player, i);
    }

    pub fn insert_trash_index(&mut self, name: &str, player: PlayerId, i: u16) {
        self.insert(name, BindingValue::TrashIndex(player, i));
    }

    pub fn insert_trash_index_for(&mut self, name: &str, player: PlayerId, i: u16) {
        self.insert_trash_index(name, player, i);
    }

    /// `insert_trash_index` plus a stable-identity PIN of the picked card
    /// (G-DSL-TRASH-INDEX-BINDING-STALE). A trash INDEX goes stale as soon as
    /// an earlier trash card moves (e.g. two `select_trash` picks, then two
    /// placements): consumers re-resolve the index from the pinned handle via
    /// [`Bindings::trash_pin`], and a pinned card that has left the trash
    /// resolves to nothing instead of silently addressing a different card.
    /// The pin lives in the ordinary slot map under a reserved key, so it
    /// survives `clone` / `merge_slots_from` with the binding it belongs to.
    pub fn insert_trash_index_pinned(
        &mut self,
        name: &str,
        player: PlayerId,
        i: u16,
        card: Option<CardHandle>,
    ) {
        self.insert_trash_index(name, player, i);
        if let Some(card) = card {
            self.slots
                .insert(trash_pin_key(name), BindingValue::Card(card));
        }
    }

    /// The card a `select_trash` pick bound to `name` was pinned to, if any.
    pub fn trash_pin(&self, name: &str) -> Option<CardHandle> {
        match self.slots.get(&trash_pin_key(name))? {
            BindingValue::Card(h) => Some(*h),
            _ => None,
        }
    }

    /// Current trash position of a pinned trash binding: `Some(Some(i))` when
    /// the pinned card is still in `player`'s trash, `Some(None)` when it has
    /// left, `None` when the binding carries no pin (legacy index-only). `trash`
    /// is the trash of the player the binding names.
    pub fn resolve_pinned_trash_index(
        &self,
        name: &str,
        trash: &[crate::card_source::CardSource],
    ) -> Option<Option<u16>> {
        let pin = self.trash_pin(name)?;
        Some(
            trash
                .iter()
                .position(|c| c.handle() == pin)
                .map(|i| i as u16),
        )
    }

    pub fn insert_literal(&mut self, name: &str, v: i64) {
        self.insert(name, BindingValue::Literal(v));
    }

    pub fn insert_permanent_list(&mut self, name: &str, list: Vec<PermanentHandle>) {
        self.insert(name, BindingValue::PermanentList(list));
    }

    pub fn insert_card_list(&mut self, name: &str, list: Vec<CardHandle>) {
        self.insert(name, BindingValue::CardList(list));
    }

    pub fn insert_source_refs(&mut self, name: &str, refs: Vec<SourceSelectionRef>) {
        self.insert(name, BindingValue::SourceRefs(refs));
    }

    pub fn insert_breeding_permanent_ref(
        &mut self,
        name: &str,
        breeding_ref: BreedingPermanentSelectionRef,
    ) {
        self.insert(name, BindingValue::BreedingPermanentRef(breeding_ref));
    }

    pub(crate) fn insert_breeding_permanent_with_identity(
        &mut self, name: &str, breeding_ref: BreedingPermanentSelectionRef, identity: u64,
    ) {
        self.insert_breeding_permanent_ref(name, breeding_ref);
        self.selected_identity_pins.insert(name.to_owned(), (breeding_ref.player, identity));
    }

    /// The binding's zone is part of the pin: a breeder moved to battle cannot
    /// become the original breeding target of a parked effect.
    pub(crate) fn resolve_selected_identity(
        &self, game: &crate::game::Game, name: &str,
    ) -> Option<PermanentHandle> {
        let (player, identity) = self.selected_identity_pin(name)?;
        if self.get_breeding_permanent_ref(name).is_some() {
            game.resolve_source_play_host_identity(PermanentHandle {
                player, index: crate::action::space::BREEDING_TARGET as u8,
            }, identity)
        } else {
            game.resolve_digivolve_host_identity(player, identity)
        }
    }

    pub fn get_permanent_list(&self, name: &str) -> Option<Vec<PermanentHandle>> {
        match self.get(name)? {
            BindingValue::PermanentList(v) => Some(v),
            _ => None,
        }
    }

    pub fn get_card_list(&self, name: &str) -> Option<Vec<CardHandle>> {
        match self.get(name)? {
            BindingValue::CardList(v) => Some(v),
            _ => None,
        }
    }

    pub fn get_source_refs(&self, name: &str) -> Option<Vec<SourceSelectionRef>> {
        match self.get(name)? {
            BindingValue::SourceRefs(v) => Some(v),
            _ => None,
        }
    }

    pub fn get_breeding_permanent_ref(&self, name: &str) -> Option<BreedingPermanentSelectionRef> {
        match self.get(name)? {
            BindingValue::BreedingPermanentRef(v) => Some(v),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selected_permanent_pin_survives_clone_and_overlay() {
        let handle = PermanentHandle {
            player: 1,
            index: 2,
        };
        let mut selected = Bindings::new();
        selected.insert_selected_permanent_with_identity("target", handle, 17);
        let cloned = selected.clone();
        selected.insert_selected_permanent_with_identity("target", handle, 18);
        assert_eq!(cloned.selected_identity_pin("target"), Some((1, 17)));

        let mut outer = Bindings::new();
        outer.insert_selected_permanent_with_identity("target", handle, 99);
        outer.merge_slots_from(&cloned);
        assert_eq!(outer.selected_identity_pin("target"), Some((1, 17)));

        let mut rebound = Bindings::new();
        rebound.insert_permanent("target", handle);
        outer.merge_slots_from(&rebound);
        assert_eq!(
            outer.selected_identity_pin("target"),
            None,
            "an unpinned rebind must not inherit the old selection's identity"
        );
    }

    #[test]
    fn selected_permanent_rebind_remove_and_merged_clear_drop_the_pin() {
        let handle = PermanentHandle {
            player: 0,
            index: 0,
        };
        let mut bindings = Bindings::new();
        bindings.insert_selected_permanent_with_identity("target", handle, 7);
        bindings.insert_literal("target", 3);
        assert_eq!(bindings.selected_identity_pin("target"), None);

        bindings.insert_selected_permanent_with_identity("target", handle, 7);
        bindings.remove("target");
        assert_eq!(bindings.selected_identity_pin("target"), None);

        bindings.insert_selected_permanent_with_identity("target", handle, 7);
        let mut cleared = bindings.clone();
        cleared.clear_for_merge("target");
        bindings.merge_slots_from(&cleared);
        assert_eq!(bindings.selected_identity_pin("target"), None);
        assert_eq!(bindings.get("target"), None);
    }

    #[test]
    fn clone_preserves_slots() {
        let mut original = Bindings::new();
        original.insert_hand_index("card_a", 0, 3);
        original.insert_trash_index("card_b", 1, 7);
        original.insert("lit", BindingValue::Literal(42));

        let cloned = original.clone();

        assert_eq!(cloned.get_hand_index("card_a"), Some(3));
        assert_eq!(cloned.get_hand_index_with_player("card_a"), Some((0, 3)));
        assert_eq!(cloned.get_trash_index("card_b"), Some(7));
        assert_eq!(cloned.get_trash_index_with_player("card_b"), Some((1, 7)));
        assert_eq!(cloned.get_literal("lit"), Some(42));
        // Mutating original does not affect clone
        let mut original = original;
        original.insert_hand_index("card_a", 0, 99);
        assert_eq!(cloned.get_hand_index("card_a"), Some(3));
    }

    #[test]
    fn permanent_list_round_trip() {
        let mut b = Bindings::new();
        let h0 = PermanentHandle {
            player: 0,
            index: 0,
        };
        let h1 = PermanentHandle {
            player: 1,
            index: 3,
        };
        b.insert_permanent_list("targets", vec![h0, h1]);
        let got = b.get_permanent_list("targets").expect("set");
        assert_eq!(got, vec![h0, h1]);
    }

    #[test]
    fn card_list_round_trip() {
        let c0 = CardHandle(0);
        let c1 = CardHandle(1);

        let mut b = Bindings::new();
        b.insert_card_list("picks", vec![c0, c1]);
        let got = b.get_card_list("picks").expect("set");
        assert_eq!(got, vec![c0, c1]);
    }

    #[test]
    fn list_clone_is_deep() {
        let mut b = Bindings::new();
        let h0 = PermanentHandle {
            player: 0,
            index: 0,
        };
        b.insert_permanent_list("xs", vec![h0]);
        let cloned = b.clone();
        b.insert_permanent_list("xs", vec![]); // mutate original
        assert_eq!(cloned.get_permanent_list("xs").unwrap().len(), 1);
    }

    #[test]
    fn literal_insert_round_trip() {
        let mut b = Bindings::new();
        b.insert_literal("branch", 1);
        assert_eq!(b.get_literal("branch"), Some(1));
    }

    #[test]
    fn played_permanent_round_trip() {
        let mut b = Bindings::new();
        let token = ProvenanceToken(42);
        let fallback = PermanentHandle {
            player: 0,
            index: 3,
        };
        b.insert_played_permanent("played", token, fallback);
        let (got_token, got_fallback) = b.get_played_permanent("played").expect("binding present");
        assert_eq!(got_token, token);
        assert_eq!(got_fallback, fallback);
        // `get_permanent` (which expects a plain Permanent variant) must NOT
        // accept a PlayedPermanent — production consumers must route through
        // the resolver, not the positional getter.
        assert_eq!(b.get_permanent("played"), None);
    }

    #[test]
    fn played_permanent_clone_isolation() {
        let mut b = Bindings::new();
        let token = ProvenanceToken(7);
        let fallback = PermanentHandle {
            player: 1,
            index: 2,
        };
        b.insert_played_permanent("played", token, fallback);
        let cloned = b.clone();
        // Mutate original — should not affect clone.
        let new_token = ProvenanceToken(99);
        let new_fallback = PermanentHandle {
            player: 0,
            index: 0,
        };
        b.insert_played_permanent("played", new_token, new_fallback);
        let (clone_token, clone_fallback) = cloned
            .get_played_permanent("played")
            .expect("clone preserved");
        assert_eq!(clone_token, token);
        assert_eq!(clone_fallback, fallback);
    }
}

#[cfg(test)]
mod private_link_identity_tests {
    use super::*;
    #[test]
    fn private_link_pins_survive_clone_merge_and_resolve_compacted_carriers() {
        let mut r=crate::debug_runner::DebugRunner::builder().add_card(crate::debug_runner::make_test_card("PIN-HOST","Pin Host")).start();
        r.place_on_field(0,"PIN-HOST",None);let host=r.place_on_field(0,"PIN-HOST",None);
        let identity=r.game.capture_digivolve_host_identity(host);
        for key in ["\0link_source_carrier:7","\0link_material_carrier:8"] {
            let mut original=Bindings::new();let archival=format!("{:?}",original);
            original.pin_private_link_identity(key,0,identity);assert!(original.get(key).is_none());assert_eq!(format!("{:?}",original),archival);
            let cloned=original.clone();let mut merged=Bindings::new();merged.merge_slots_from(&cloned);
            assert_eq!(merged.selected_identity_pin(key),Some((0,identity)));
            r.game.player_mut(0).battle_area.swap(0,1);
            assert_eq!(merged.resolve_selected_identity(&r.game,key),Some(PermanentHandle{player:0,index:0}));
            assert_eq!(original.selected_identity_pin(key),Some((0,identity)));
            r.game.player_mut(0).battle_area.swap(0,1);
        }
    }
    #[test]
    fn private_link_pin_clear_and_explicit_rebind_override_prior_metadata() {
        for key in ["\0link_source_carrier:7","\0link_material_carrier:8"] {
            let mut prior=Bindings::new();prior.pin_private_link_identity(key,0,42);
            let mut cleared=prior.clone();cleared.clear_for_merge(key);prior.merge_slots_from(&cleared);
            assert!(prior.selected_identity_pin(key).is_none());assert!(prior.get(key).is_none());
            prior.pin_private_link_identity(key,0,42);
            let mut rebound=prior.clone();rebound.insert(key,BindingValue::Literal(9));prior.merge_slots_from(&rebound);
            assert!(prior.selected_identity_pin(key).is_none());assert_eq!(prior.get(key),Some(BindingValue::Literal(9)));
            assert!(cleared.selected_identity_pin(key).is_none());
        }
    }
}
