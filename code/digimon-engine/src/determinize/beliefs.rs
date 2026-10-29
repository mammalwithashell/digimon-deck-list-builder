//! Closed-list belief primitives. This module deliberately separates evidence
//! extraction from the provider: `WeightedDeckPrior` has no access to a Game.
//! Runtime knowledge-event wiring and hypothetical-world construction are
//! separate integration steps; these types alone do not enable fair self-play.

use std::collections::{BTreeMap, BTreeSet};

use rand::{rngs::StdRng, Rng, SeedableRng};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::CardMultiset;
use crate::deck_tools::{
    card_database, card_legality_for_descriptor, validate_deck_for_descriptor,
};
use crate::enums::PlayerId;
use crate::format::{self, FormatDescriptor};
use crate::game::Game;

/// Field identities stay public while a DigiXros leave window holds the
/// physical stack in limbo; it has not disappeared from deck accounting.
pub(crate) fn field_sources(game: &Game) -> impl Iterator<Item = &crate::card_source::CardSource> {
    field_sources_with_controller(game).map(|(_, c)| c)
}

pub(crate) fn field_sources_with_controller(
    game: &Game,
) -> impl Iterator<Item = (PlayerId, &crate::card_source::CardSource)> {
    game.players
        .iter()
        .flat_map(|p| {
            p.battle_area
                .iter()
                .chain(p.breeding_area.iter())
                .map(move |f| (p.id, f))
        })
        .chain(
            game.digixros_leaving_limbo
                .iter()
                .map(|(controller, _, p)| (*controller, p)),
        )
        .flat_map(|(controller, p)| {
            p.card_sources
                .iter()
                .chain(&p.linked_cards)
                .map(move |c| (controller, c))
        })
}

/// `pending_security` also carries unrevealed effect removals. It is not a
/// blanket reveal entitlement: a hidden-to-hidden move stays concealed.
pub(crate) fn public_pending_security(
    game: &Game,
) -> impl Iterator<Item = &crate::card_source::CardSource> {
    let checked = |p: &crate::selection::PendingSecurity| {
        game.security_resolution
            .as_ref()
            .is_some_and(|s| s.revealed_card == p.card.handle())
    };
    let trashed = |i: usize| {
        matches!(
            game.pending_effect_security_removal[i].destination,
            crate::selection::SecurityRemovalDestination::Trash
        )
    };
    let mut public = Vec::new();
    if let Some(p) = &game.pending_security {
        if !p.played
            && (checked(p)
                || (!game.pending_effect_security_removal.is_empty()
                    && trashed(game.pending_effect_security_removal.len() - 1)))
        {
            public.push(&p.card);
        }
    }
    for (i, scope) in game.pending_effect_security_removal.iter().enumerate() {
        if let Some(p) = &scope.previous_pending_security {
            if !p.played && (checked(p) || (i > 0 && trashed(i - 1))) {
                public.push(&p.card);
            }
        }
    }
    public.into_iter()
}

pub(crate) fn physical_security_transits(
    game: &Game,
) -> impl Iterator<Item = &crate::card_source::CardSource> {
    game.pending_security
        .iter()
        .chain(
            game.pending_effect_security_removal
                .iter()
                .filter_map(|s| s.previous_pending_security.as_ref()),
        )
        .filter(|p| !p.played)
        .map(|p| &p.card)
}

pub const BELIEF_SCHEMA_VERSION: u32 = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InformationRegime {
    OpenDecklist,
    ClosedDecklist,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KnowledgeZone {
    Public,
    Hand,
    Deck,
    Security,
    Digitama,
    /// Publicly located face-down source; identity is owner-private (4-7-10).
    Field,
    /// A concealed security card between its removal and committed destination.
    InFlight,
}

/// A publicly tracked physical move without any observed card identity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct ConcealedPosition {
    pub owner: PlayerId,
    pub instance: u16,
    pub zone: KnowledgeZone,
    pub position: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct KnownCard {
    owner: PlayerId,
    instance: u16,
    id: String,
    zone: KnowledgeZone,
    position: Option<usize>,
}

/// A single viewer's entitled memory. Events must be delivered only to
/// entitled seats. Instance keys are internal bookkeeping, not neural input.
/// No caller can infer remembered cards by examining the live hidden zones.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KnowledgeLedger {
    schema_version: u32,
    viewer: PlayerId,
    cards: Vec<KnownCard>,
    #[serde(default)]
    history_minimum: BTreeMap<PlayerId, CardMultiset>,
    #[serde(default)]
    last_counts: Option<[[usize; 4]; 2]>,
    /// Public token provenance survives leave-field removal. Tokens never
    /// enter deck-copy accounting, but queued effects may still name them.
    #[serde(default)]
    public_tokens: BTreeSet<u16>,
    #[serde(default)]
    concealed_positions: Vec<ConcealedPosition>,
}

impl KnowledgeLedger {
    pub fn new(viewer: PlayerId) -> Result<Self, String> {
        if viewer > 1 {
            return Err("beliefs support two players only".into());
        }
        Ok(Self {
            schema_version: BELIEF_SCHEMA_VERSION,
            viewer,
            cards: Vec::new(),
            history_minimum: BTreeMap::new(),
            last_counts: None,
            public_tokens: BTreeSet::new(),
            concealed_positions: Vec::new(),
        })
    }

    pub(crate) fn observe_public_token(&mut self, instance: u16) {
        self.public_tokens.insert(instance);
    }
    pub(crate) fn public_tokens(&self) -> &BTreeSet<u16> {
        &self.public_tokens
    }
    pub(crate) fn concealed_positions(&self) -> &[ConcealedPosition] {
        &self.concealed_positions
    }
    pub(crate) fn known_identity(&self, owner: PlayerId, instance: u16) -> Option<String> {
        self.cards
            .iter()
            .find(|c| c.owner == owner && c.instance == instance)
            .map(|c| c.id.clone())
    }
    pub(crate) fn remember_concealed_position(
        &mut self,
        owner: PlayerId,
        instance: u16,
        zone: KnowledgeZone,
        position: Option<usize>,
    ) {
        self.concealed_positions
            .retain(|c| c.owner != owner || c.instance != instance);
        self.concealed_positions.push(ConcealedPosition {
            owner,
            instance,
            zone,
            position,
        });
    }

    pub(crate) fn conceal_security_departure(
        &mut self,
        owner: PlayerId,
        instance: u16,
        position: usize,
    ) {
        // Face-up security is already identified publicly, even though its
        // ordinary ledger zone is Public rather than Security.
        if let Some(c) = self
            .cards
            .iter_mut()
            .find(|c| c.owner == owner && c.instance == instance && c.zone == KnowledgeZone::Public)
        {
            c.zone = KnowledgeZone::Security;
            c.position = Some(position);
        }
        self.transfer_position(
            owner,
            KnowledgeZone::Security,
            position,
            KnowledgeZone::InFlight,
        );
        self.concealed_positions
            .retain(|c| c.owner != owner || c.instance != instance);
        self.concealed_positions.push(ConcealedPosition {
            owner,
            instance,
            zone: KnowledgeZone::InFlight,
            position: None,
        });
    }

    pub(crate) fn commit_concealed_return(
        &mut self,
        owner: PlayerId,
        instance: u16,
        position: usize,
    ) {
        if let Some(c) = self.cards.iter_mut().find(|c| {
            c.owner == owner && c.instance == instance && c.zone == KnowledgeZone::InFlight
        }) {
            c.zone = KnowledgeZone::Deck;
            c.position = Some(position);
        }
        if let Some(c) = self
            .concealed_positions
            .iter_mut()
            .find(|c| c.owner == owner && c.instance == instance)
        {
            c.zone = KnowledgeZone::Deck;
            c.position = Some(position);
        }
        if let Some(counts) = &mut self.last_counts {
            counts[owner as usize][1] += 1;
        }
    }
    pub(crate) fn commit_concealed_hand(&mut self, owner: PlayerId, instance: u16) {
        if let Some(c) = self.cards.iter_mut().find(|c| {
            c.owner == owner && c.instance == instance && c.zone == KnowledgeZone::InFlight
        }) {
            c.zone = KnowledgeZone::Hand;
            c.position = None;
        }
        if let Some(c) = self
            .concealed_positions
            .iter_mut()
            .find(|c| c.owner == owner && c.instance == instance)
        {
            c.zone = KnowledgeZone::Hand;
            c.position = None;
        }
        if let Some(counts) = &mut self.last_counts {
            counts[owner as usize][0] += 1;
        }
    }

    pub fn observe(
        &mut self,
        owner: PlayerId,
        instance: u16,
        id: &str,
        zone: KnowledgeZone,
        position: Option<usize>,
    ) -> Result<(), String> {
        if owner > 1 || id.is_empty() {
            return Err("invalid entitled observation".into());
        }
        if let Some(card) = self
            .cards
            .iter_mut()
            .find(|c| c.owner == owner && c.instance == instance)
        {
            if card.id != id {
                return Err("known physical instance changed identity".into());
            }
            card.zone = zone;
            card.position = position;
        } else {
            self.cards.push(KnownCard {
                owner,
                instance,
                id: id.into(),
                zone,
                position,
            });
        }
        self.cards.sort_by_key(|c| (c.owner, c.instance));
        if zone != KnowledgeZone::InFlight {
            self.concealed_positions
                .retain(|c| c.owner != owner || c.instance != instance);
        }
        self.remember_minimum();
        Ok(())
    }

    /// A revealed declaration is not proof that it is a second physical copy
    /// of a card remembered somewhere in an opponent's unpositioned hand.
    pub fn observe_public_hand_declaration(
        &mut self,
        owner: PlayerId,
        instance: u16,
        id: &str,
    ) -> Result<(), String> {
        if self.viewer != owner {
            self.cards
                .retain(|c| c.owner != owner || c.zone != KnowledgeZone::Hand || c.id != id);
            self.concealed_positions
                .retain(|c| c.owner != owner || c.zone != KnowledgeZone::Hand);
        }
        self.observe(owner, instance, id, KnowledgeZone::Hand, None)
    }

    pub(crate) fn public_hand_departure(&mut self, owner: PlayerId, instance: u16, id: &str) {
        self.cards.retain(|c| {
            c.owner != owner
                || c.zone != KnowledgeZone::Hand
                || if self.viewer == owner {
                    c.instance != instance
                } else {
                    c.id != id
                }
        });
        self.concealed_positions
            .retain(|c| c.owner != owner || c.zone != KnowledgeZone::Hand);
        if let Some(counts) = &mut self.last_counts {
            counts[owner as usize][0] = counts[owner as usize][0].saturating_sub(1);
        }
    }

    pub fn move_known(
        &mut self,
        owner: PlayerId,
        instance: u16,
        zone: KnowledgeZone,
        position: Option<usize>,
    ) -> Result<(), String> {
        let card = self
            .cards
            .iter_mut()
            .find(|c| c.owner == owner && c.instance == instance)
            .ok_or("move of an unknown instance requires an entitled observation")?;
        card.zone = zone;
        card.position = position;
        Ok(())
    }

    /// A within-zone shuffle destroys positions, not knowledge that the
    /// observed physical cards are still somewhere in that zone.
    pub fn shuffle(&mut self, owner: PlayerId, zone: KnowledgeZone) {
        for c in self
            .cards
            .iter_mut()
            .filter(|c| c.owner == owner && c.zone == zone)
        {
            c.position = None;
        }
        for c in self
            .concealed_positions
            .iter_mut()
            .filter(|c| c.owner == owner && c.zone == zone)
        {
            c.position = None;
        }
    }
    pub(crate) fn insert_known_top(&mut self, owner: PlayerId, zone: KnowledgeZone) {
        for c in self
            .cards
            .iter_mut()
            .filter(|c| c.owner == owner && c.zone == zone)
        {
            if let Some(p) = &mut c.position {
                *p += 1;
            }
        }
        for c in self
            .concealed_positions
            .iter_mut()
            .filter(|c| c.owner == owner && c.zone == zone)
        {
            if let Some(p) = &mut c.position {
                *p += 1;
            }
        }
    }

    /// A publicly identified position can be tracked without reading the hidden
    /// source's identity. Unpositioned cards cannot be matched by native handle.
    pub(crate) fn transfer_position(
        &mut self,
        owner: PlayerId,
        from: KnowledgeZone,
        position: usize,
        to: KnowledgeZone,
    ) -> Option<u16> {
        let known = self
            .cards
            .iter()
            .find(|c| c.owner == owner && c.zone == from && c.position == Some(position))
            .map(|c| c.instance);
        self.cards.retain(|c| {
            c.owner != owner || c.zone != from || c.position.is_some() || known.is_some()
        });
        for c in self
            .cards
            .iter_mut()
            .filter(|c| c.owner == owner && c.zone == from)
        {
            if Some(c.instance) == known {
                c.zone = to;
                c.position = None;
            } else if let Some(p) = &mut c.position {
                if *p > position {
                    *p -= 1;
                }
            }
        }
        self.concealed_positions.retain_mut(|c| {
            if c.owner != owner || c.zone != from {
                return true;
            }
            match c.position {
                Some(p) if p == position => {
                    if matches!(to, KnowledgeZone::Hand | KnowledgeZone::Field) {
                        c.zone = to;
                        c.position = None;
                        true
                    } else {
                        false
                    } // Public reveals are learned on refresh.
                }
                Some(p) if p > position => {
                    c.position = Some(p - 1);
                    true
                }
                Some(_) => true,
                None => false,
            }
        });
        let index = |z| match z {
            KnowledgeZone::Hand => Some(0),
            KnowledgeZone::Deck => Some(1),
            KnowledgeZone::Security => Some(2),
            KnowledgeZone::Digitama => Some(3),
            _ => None,
        };
        if let Some(counts) = &mut self.last_counts {
            if let Some(i) = index(from) {
                counts[owner as usize][i] = counts[owner as usize][i].saturating_sub(1);
            }
            if let Some(i) = index(to) {
                counts[owner as usize][i] += 1;
            }
        }
        known
    }

    /// An unobserved transfer cannot be resolved using the true instance that
    /// moved. Conservatively invalidate membership facts in affected zones.
    /// A future probabilistic ledger can preserve stronger lower bounds.
    pub fn forget_zone(&mut self, owner: PlayerId, zone: KnowledgeZone) {
        self.cards.retain(|c| c.owner != owner || c.zone != zone);
        self.concealed_positions
            .retain(|c| c.owner != owner || c.zone != zone);
    }

    pub fn known_positions(&self, owner: PlayerId) -> Vec<(String, KnowledgeZone, usize)> {
        self.cards
            .iter()
            .filter(|c| c.owner == owner)
            .filter_map(|c| c.position.map(|p| (c.id.clone(), c.zone, p)))
            .collect()
    }

    pub fn known_hidden(&self, owner: PlayerId) -> Vec<(String, KnowledgeZone)> {
        self.cards
            .iter()
            .filter(|c| c.owner == owner && c.zone != KnowledgeZone::Public)
            .map(|c| (c.id.clone(), c.zone))
            .collect()
    }
    pub(crate) fn hidden_instances(
        &self,
        owner: PlayerId,
    ) -> Vec<(u16, String, KnowledgeZone, Option<usize>)> {
        self.cards
            .iter()
            .filter(|c| c.owner == owner && c.zone != KnowledgeZone::Public)
            .map(|c| (c.instance, c.id.clone(), c.zone, c.position))
            .collect()
    }

    fn remember_minimum(&mut self) {
        for owner in 0..2 {
            let mut current = CardMultiset::new();
            for c in self.cards.iter().filter(|c| c.owner == owner) {
                *current.entry(c.id.clone()).or_default() += 1;
            }
            let history = self.history_minimum.entry(owner).or_default();
            for (id, n) in current {
                let old = history.entry(id).or_default();
                *old = (*old).max(n);
            }
        }
    }

    /// Refresh entitled state only. Unknown departures invalidate membership
    /// conservatively; native hidden handles are never consulted to resolve them.
    pub(crate) fn refresh(&mut self, game: &Game) -> Result<(), String> {
        if game.players.len() != 2 {
            return Err("knowledge is two-player only".into());
        }
        let counts: [[usize; 4]; 2] = std::array::from_fn(|i| {
            let p = &game.players[i];
            [
                p.hand.len(),
                p.deck.len(),
                p.security.len(),
                p.digitama_deck.len(),
            ]
        });
        if let Some(previous) = self.last_counts {
            for owner in 0..2 {
                for (i, zone) in [
                    KnowledgeZone::Hand,
                    KnowledgeZone::Deck,
                    KnowledgeZone::Security,
                    KnowledgeZone::Digitama,
                ]
                .into_iter()
                .enumerate()
                {
                    if counts[owner][i] < previous[owner][i] {
                        self.forget_zone(owner as u8, zone);
                    }
                }
            }
        }
        // A field source has a public physical location even when its identity
        // is hidden. Track only identities already learned, plus the owner's
        // inspection entitlement; never learn an opponent's unknown identity.
        let field: Vec<_> = field_sources(game).filter(|c| c.face_down).collect();
        self.cards.retain(|c| {
            c.zone != KnowledgeZone::Field
                || field
                    .iter()
                    .any(|f| f.owner == c.owner && f.card_index == c.instance)
        });
        self.concealed_positions.retain(|c| {
            c.zone != KnowledgeZone::Field
                || field
                    .iter()
                    .any(|f| f.owner == c.owner && f.card_index == c.instance)
        });
        for c in &field {
            if let Some(known) = self.cards.iter_mut().find(|k| {
                k.owner == c.owner && k.instance == c.card_index && k.zone == KnowledgeZone::Public
            }) {
                known.zone = KnowledgeZone::Field;
                known.position = None;
            }
            if c.owner == self.viewer
                && !c.is_opaque_placeholder
                && field_sources_with_controller(game).any(|(controller, source)| {
                    source.card_index == c.card_index && controller == self.viewer
                })
            {
                self.observe(
                    c.owner,
                    c.card_index,
                    c.card_id(&game.card_data),
                    KnowledgeZone::Field,
                    None,
                )?;
            }
        }
        let mut public = Vec::new();
        for p in &game.players {
            for c in p
                .trash
                .iter()
                .chain(
                    p.battle_area
                        .iter()
                        .flat_map(|p| p.card_sources.iter().chain(&p.linked_cards)),
                )
                .chain(
                    p.breeding_area
                        .iter()
                        .flat_map(|p| p.card_sources.iter().chain(&p.linked_cards)),
                )
                .chain(p.commander_zone.iter())
                .chain(
                    p.security
                        .iter()
                        .filter(|c| p.face_up_security.contains(&c.card_index)),
                )
            {
                if !c.is_token && !c.face_down && !c.is_opaque_placeholder {
                    public.push((
                        c.owner,
                        c.card_index,
                        c.card_id(&game.card_data).to_string(),
                    ));
                }
            }
        }
        for c in game
            .revealed_cards
            .iter()
            .chain(game.pending_option.iter().map(|p| &p.card))
            .chain(public_pending_security(game))
        {
            if !c.is_token && !c.is_opaque_placeholder {
                public.push((
                    c.owner,
                    c.card_index,
                    c.card_id(&game.card_data).to_string(),
                ));
            }
        }
        for c in game
            .digixros_leaving_limbo
            .iter()
            .flat_map(|(_, _, p)| p.card_sources.iter().chain(&p.linked_cards))
        {
            if !c.is_token && !c.face_down && !c.is_opaque_placeholder {
                public.push((
                    c.owner,
                    c.card_index,
                    c.card_id(&game.card_data).to_string(),
                ));
            }
        }
        let present: BTreeSet<_> = public
            .iter()
            .map(|(owner, instance, _)| (*owner, *instance))
            .collect();
        // Remove vanished public observations BEFORE observing new instances.
        // This avoids counting recycled indistinguishable copies twice.
        self.cards.retain(|c| {
            c.zone != KnowledgeZone::Public || present.contains(&(c.owner, c.instance))
        });
        for (owner, instance, id) in public {
            self.observe(owner, instance, &id, KnowledgeZone::Public, None)?;
        }
        self.forget_zone(self.viewer, KnowledgeZone::Hand);
        for c in &game.players[self.viewer as usize].hand {
            self.observe(
                c.owner,
                c.card_index,
                c.card_id(&game.card_data),
                KnowledgeZone::Hand,
                None,
            )?;
        }
        // A declared play/digivolution is face-up information while payment
        // and replacement choices are pending, even before the card enters
        // the field. Read only that explicitly declared physical card.
        let mut declarations: Vec<_> = game
            .pending_would_play_resume
            .iter()
            .filter(|p| matches!(p.origin, crate::game::PendingWouldPlayOrigin::Hand))
            .map(|p| (p.player, p.card))
            .chain(
                game.pending_would_digivolve_resume
                    .iter()
                    .map(|p| (p.player, p.card)),
            )
            .collect();
        if let Some(transaction) = &game.pending_digixros_transaction {
            declarations.push((transaction.controller, transaction.played_card));
            for m in transaction
                .selected_materials
                .iter()
                .chain(&transaction.pre_attached_materials)
            {
                if let crate::digixros::DigiXrosMaterialOrigin::Hand { player, card, .. } = m.origin
                {
                    declarations.push((player, card));
                }
            }
        }
        for (owner, handle) in declarations {
            if let Some(c) = game.players[owner as usize]
                .hand
                .iter()
                .find(|c| c.handle() == handle)
            {
                self.observe_public_hand_declaration(
                    c.owner,
                    c.card_index,
                    c.card_id(&game.card_data),
                )?;
            }
        }
        if let Some(owner) = game
            .pending_selection
            .as_ref()
            .and_then(|s| s.card_choice.as_ref())
            .and_then(|c| c.inspected_security_owner(self.viewer))
        {
            for (i, c) in game.players[owner as usize]
                .security
                .iter()
                .rev()
                .enumerate()
            {
                self.observe(
                    c.owner,
                    c.card_index,
                    c.card_id(&game.card_data),
                    KnowledgeZone::Security,
                    Some(i),
                )?;
            }
        }
        self.last_counts = Some(counts);
        Ok(())
    }
}

/// Only counts and currently entitled identities. No exact opposing list,
/// original-deck metadata, hidden CardSource, or hidden RNG enters the provider.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BeliefEvidence {
    pub viewer: PlayerId,
    pub opponent: PlayerId,
    pub hand_count: usize,
    pub deck_count: usize,
    pub security_count: usize,
    pub digitama_count: usize,
    #[serde(default)]
    pub field_count: usize,
    pub accounted: CardMultiset,
    pub known_hidden: Vec<(String, KnowledgeZone)>,
    #[serde(default)]
    pub history_minimum: CardMultiset,
}

impl BeliefEvidence {
    pub fn from_counts(
        viewer: PlayerId,
        opponent: PlayerId,
        hand_count: usize,
        deck_count: usize,
        security_count: usize,
        digitama_count: usize,
        accounted: CardMultiset,
    ) -> Result<Self, String> {
        if viewer > 1 || opponent != 1 - viewer {
            return Err("invalid viewer/opponent pair".into());
        }
        Ok(Self {
            viewer,
            opponent,
            hand_count,
            deck_count,
            security_count,
            digitama_count,
            field_count: 0,
            accounted,
            known_hidden: Vec::new(),
            history_minimum: CardMultiset::new(),
        })
    }

    /// Read current public identities and hidden-zone counts only. Face-down
    /// field locations are public; their identities come from entitled memory
    /// or the prior. Opaque placeholder games still fail explicitly.
    pub fn extract(
        game: &Game,
        viewer: PlayerId,
        memory: &KnowledgeLedger,
    ) -> Result<Self, String> {
        if game.players.len() != 2
            || viewer > 1
            || memory.viewer != viewer
            || memory.schema_version != BELIEF_SCHEMA_VERSION
        {
            return Err("incompatible viewer knowledge".into());
        }
        if game.players.iter().any(|p| p.opaque_deck_state.is_some()) {
            return Err("opaque game accounting is not supported yet".into());
        }
        let opponent = 1 - viewer;
        let p = &game.players[opponent as usize];
        let inspected = game
            .pending_selection
            .as_ref()
            .and_then(|s| s.card_choice.as_ref())
            .and_then(|c| c.inspected_security_owner(viewer));
        let mut accounted = CardMultiset::new();
        let mut public_instances = BTreeSet::new();
        let mut add = |c: &crate::card_source::CardSource| -> Result<(), String> {
            if c.is_token || c.owner != opponent {
                return Ok(());
            }
            if c.is_opaque_placeholder {
                return Err("opaque public source is unsupported".into());
            }
            if c.face_down {
                return Ok(());
            }
            if !public_instances.insert(c.card_index) {
                return Ok(()); // An in-flight public snapshot may reference the same instance.
            }
            *accounted
                .entry(c.card_id(&game.card_data).into())
                .or_default() += 1;
            Ok(())
        };
        // Original ownership is independent of which player controls a public
        // stack or zone containing the card.
        for public_player in &game.players {
            for c in &public_player.trash {
                add(c)?;
            }
            for perm in public_player
                .battle_area
                .iter()
                .chain(public_player.breeding_area.iter())
            {
                for c in perm.card_sources.iter().chain(&perm.linked_cards) {
                    add(c)?;
                }
            }
            if let Some(c) = &public_player.commander_zone {
                add(c)?;
            }
            for c in public_player.security.iter().filter(|c| {
                inspected == Some(public_player.id)
                    || public_player.face_up_security.contains(&c.card_index)
            }) {
                add(c)?;
            }
        }
        for c in game
            .revealed_cards
            .iter()
            .chain(game.pending_option.iter().map(|p| &p.card))
            .chain(public_pending_security(game))
        {
            add(c)?;
        }
        for c in game
            .digixros_leaving_limbo
            .iter()
            .flat_map(|(_, _, p)| p.card_sources.iter().chain(&p.linked_cards))
        {
            add(c)?;
        }
        let mut result = Self::from_counts(
            viewer,
            opponent,
            p.hand.len(),
            p.deck.len(),
            p.security
                .iter()
                .filter(|c| {
                    inspected != Some(opponent) && !p.face_up_security.contains(&c.card_index)
                })
                .count(),
            p.digitama_deck.len(),
            accounted,
        )?;
        result.field_count = field_sources(game)
            .filter(|c| c.owner == opponent && c.face_down && !c.is_token)
            .count();
        // Do not subtract a ledger entry twice if its instance is now public.
        result.known_hidden = memory
            .cards
            .iter()
            .filter(|c| {
                c.owner == opponent
                    && c.zone != KnowledgeZone::Public
                    && !public_instances.contains(&c.instance)
            })
            .map(|c| (c.id.clone(), c.zone))
            .collect();
        result.history_minimum = memory
            .history_minimum
            .get(&opponent)
            .cloned()
            .unwrap_or_default();
        Ok(result)
    }

    fn required(&self) -> CardMultiset {
        let mut required = self.accounted.clone();
        for (id, _) in &self.known_hidden {
            *required.entry(id.clone()).or_default() += 1;
        }
        for (id, n) in &self.history_minimum {
            let current = required.entry(id.clone()).or_default();
            *current = (*current).max(*n);
        }
        required
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CandidateDeck {
    pub cards: Vec<String>,
    pub weight: f64,
}

/// A declared admitted population, not an automatic claim that every supplied
/// card is implemented. Production snapshot preparation must intersect this
/// set with the engine's implemented pool and freeze its provenance.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PriorSnapshot {
    pub schema_version: u32,
    pub format_id: String,
    pub provenance: String,
    pub legality_hash: String,
    pub population: BTreeSet<String>,
    pub candidates: Vec<CandidateDeck>,
    pub unseen_mass: f64,
    pub fallback_attempts: usize,
}

fn hash_bytes(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn legality_hash() -> String {
    // Includes printed per-card copy allowances and DSL deck-rule overrides,
    // not only the restriction table. Runtime card metadata is compiled in.
    static HASH: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    HASH.get_or_init(|| {
        let mut h = Sha256::new();
        h.update(include_bytes!("../../../../data/deck_formats.json"));
        let mut cards: Vec<_> = card_database().iter().collect();
        cards.sort_by_key(|(id, _)| *id);
        for (id, card) in cards {
            h.update(id.as_bytes());
            h.update([0, card.card_kind, card.rarity as u8]);
            h.update(card.max_count_in_deck.to_le_bytes());
        }
        format!("{:x}", h.finalize())
    })
    .clone()
}

impl PriorSnapshot {
    pub fn new(
        format_id: &str,
        provenance: &str,
        population: BTreeSet<String>,
        candidates: Vec<CandidateDeck>,
        unseen_mass: f64,
        fallback_attempts: usize,
    ) -> Result<Self, String> {
        let snapshot = Self {
            schema_version: BELIEF_SCHEMA_VERSION,
            format_id: format_id.into(),
            provenance: provenance.into(),
            legality_hash: legality_hash(),
            population,
            candidates,
            unseen_mass,
            fallback_attempts,
        };
        // Normalize/validate through exactly the same path used on file load.
        Ok(WeightedDeckPrior::new(snapshot)?.snapshot)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Posterior {
    /// Canonical candidate index and posterior mass (not renormalized to 1
    /// over candidates alone; the unseen component retains its mass).
    pub candidates: Vec<(usize, f64)>,
    pub fallback_mass: f64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeckHypothesis {
    pub deck: Vec<String>,
    /// Entire hidden pool, INCLUDING entitled known-hidden cards.
    pub hidden_main: CardMultiset,
    pub hidden_eggs: CardMultiset,
    pub used_fallback: bool,
}

pub struct WeightedDeckPrior {
    snapshot: PriorSnapshot,
    fmt: &'static FormatDescriptor,
    hash: String,
}

fn counts(cards: &[String]) -> CardMultiset {
    let mut result = CardMultiset::new();
    for id in cards {
        *result.entry(id.clone()).or_default() += 1;
    }
    result
}

fn expanded(cards: &CardMultiset) -> Vec<String> {
    cards
        .iter()
        .flat_map(|(id, n)| std::iter::repeat(id.clone()).take(*n))
        .collect()
}

fn egg(id: &str) -> bool {
    card_database().get(id).is_some_and(|c| c.card_kind == 3)
}

impl WeightedDeckPrior {
    pub fn new(mut snapshot: PriorSnapshot) -> Result<Self, String> {
        if snapshot.schema_version != BELIEF_SCHEMA_VERSION
            || snapshot.legality_hash != legality_hash()
        {
            return Err("unsupported prior schema or changed compiled legality snapshot".into());
        }
        if snapshot.provenance.trim().is_empty()
            || snapshot.population.is_empty()
            || !snapshot.unseen_mass.is_finite()
            || snapshot.unseen_mass <= 0.0
            || snapshot.unseen_mass > 1.0
            || snapshot.fallback_attempts == 0
        {
            return Err(
                "invalid prior provenance, population, unseen mass or attempt budget".into(),
            );
        }
        let fmt = format::descriptor(&snapshot.format_id).ok_or("unknown prior format")?;
        if snapshot
            .population
            .iter()
            .any(|id| !card_database().contains_key(id))
        {
            return Err("unknown card in admitted prior population".into());
        }
        let mut dedup: BTreeMap<Vec<String>, f64> = BTreeMap::new();
        for mut c in std::mem::take(&mut snapshot.candidates) {
            if !c.weight.is_finite() || c.weight <= 0.0 {
                return Err("candidate weights must be finite and positive".into());
            }
            c.cards.sort();
            if c.cards.iter().any(|id| !snapshot.population.contains(id)) {
                return Err("candidate outside admitted population".into());
            }
            let validation = validate_deck_for_descriptor(&c.cards, fmt);
            if !validation.is_valid {
                return Err(validation.errors.join("; "));
            }
            if let Some(previous) = dedup.insert(c.cards, c.weight) {
                if previous != c.weight {
                    return Err("conflicting weights for duplicate deck".into());
                }
            }
        }
        snapshot.candidates = dedup
            .into_iter()
            .map(|(cards, weight)| CandidateDeck { cards, weight })
            .collect();
        let bytes = serde_json::to_vec(&snapshot).map_err(|e| e.to_string())?;
        Ok(Self {
            snapshot,
            fmt,
            hash: hash_bytes(&bytes),
        })
    }

    pub fn snapshot_hash(&self) -> &str {
        &self.hash
    }
    pub fn candidate_count(&self) -> usize {
        self.snapshot.candidates.len()
    }
    pub fn snapshot(&self) -> &PriorSnapshot {
        &self.snapshot
    }

    fn validate_evidence(&self, e: &BeliefEvidence) -> Result<(), String> {
        if e.viewer > 1 || e.opponent != 1 - e.viewer {
            return Err("invalid evidence seats".into());
        }
        let required = e.required();
        if required.iter().any(|(id, n)| {
            *n == 0
                || !self.snapshot.population.contains(id)
                || *n > card_legality_for_descriptor(id, self.fmt).max_copies as usize
        }) {
            return Err("evidence incompatible with admitted population or copy limits".into());
        }
        let public_main: usize = e
            .accounted
            .iter()
            .filter(|(id, _)| !egg(id))
            .map(|(_, n)| *n)
            .sum();
        let public_eggs: usize = e
            .accounted
            .iter()
            .filter(|(id, _)| egg(id))
            .map(|(_, n)| *n)
            .sum();
        if public_main
            .checked_add(e.hand_count)
            .and_then(|n| n.checked_add(e.deck_count))
            .and_then(|n| n.checked_add(e.security_count))
            .and_then(|n| n.checked_add(e.field_count))
            != Some(self.fmt.deck_size as usize)
            || public_eggs
                .checked_add(e.digitama_count)
                .is_none_or(|n| n > self.fmt.egg_max as usize)
        {
            return Err("public and hidden counts do not reconcile with format".into());
        }
        for (zone, capacity) in [
            (KnowledgeZone::Hand, e.hand_count),
            (KnowledgeZone::Deck, e.deck_count),
            (KnowledgeZone::Security, e.security_count),
            (KnowledgeZone::Digitama, e.digitama_count),
            (KnowledgeZone::Field, e.field_count),
        ] {
            let facts: Vec<_> = e.known_hidden.iter().filter(|(_, z)| *z == zone).collect();
            if facts.len() > capacity
                || facts
                    .iter()
                    .any(|(id, _)| egg(id) != (zone == KnowledgeZone::Digitama))
            {
                return Err("known hidden identities exceed zone count or kind".into());
            }
        }
        if e.known_hidden
            .iter()
            .any(|(_, z)| *z == KnowledgeZone::Public)
        {
            return Err("public facts must be accounted once".into());
        }
        Ok(())
    }

    fn consistent(&self, cards: &[String], e: &BeliefEvidence) -> bool {
        let c = counts(cards);
        let n_eggs = cards.iter().filter(|id| egg(id)).count();
        let public_eggs: usize = e
            .accounted
            .iter()
            .filter(|(id, _)| egg(id))
            .map(|(_, n)| n)
            .sum();
        n_eggs == public_eggs + e.digitama_count
            && e.required()
                .iter()
                .all(|(id, n)| c.get(id).copied().unwrap_or(0) >= *n)
    }

    pub fn posterior(&self, e: &BeliefEvidence) -> Result<Posterior, String> {
        self.validate_evidence(e)?;
        let compatible: Vec<_> = self
            .snapshot
            .candidates
            .iter()
            .enumerate()
            .filter(|(_, c)| self.consistent(&c.cards, e))
            .collect();
        // Divide by the largest weight first so finite large weights cannot overflow.
        let max = compatible
            .iter()
            .map(|(_, c)| c.weight)
            .fold(0.0_f64, f64::max);
        let total: f64 = compatible.iter().map(|(_, c)| c.weight / max).sum();
        let fallback_mass = if compatible.is_empty() {
            1.0
        } else {
            self.snapshot.unseen_mass
        };
        Ok(Posterior {
            candidates: compatible
                .iter()
                .map(|(i, c)| (*i, (1.0 - fallback_mass) * (c.weight / max) / total))
                .collect(),
            fallback_mass,
        })
    }

    pub fn sample(&self, e: &BeliefEvidence, seed: u64) -> Result<DeckHypothesis, String> {
        let posterior = self.posterior(e)?;
        let mut rng = StdRng::seed_from_u64(seed);
        let mut draw = rng.gen::<f64>();
        for (idx, mass) in &posterior.candidates {
            if draw < *mass {
                return self.hypothesis(self.snapshot.candidates[*idx].cards.clone(), e, false);
            }
            draw -= mass;
        }
        self.complete(e, &mut rng)
    }

    fn hypothesis(
        &self,
        deck: Vec<String>,
        e: &BeliefEvidence,
        used_fallback: bool,
    ) -> Result<DeckHypothesis, String> {
        let mut hidden = counts(&deck);
        for (id, n) in &e.accounted {
            let count = hidden
                .get_mut(id)
                .ok_or("accounted card missing from hypothesis")?;
            *count = count
                .checked_sub(*n)
                .ok_or("public copies exceed hypothesis")?;
        }
        hidden.retain(|_, n| *n > 0);
        let (hidden_eggs, hidden_main) = hidden.into_iter().partition(|(id, _)| egg(id));
        Ok(DeckHypothesis {
            deck,
            hidden_main,
            hidden_eggs,
            used_fallback,
        })
    }

    /// Uniform per eligible ID at each completion step, with full existing
    /// deck validation as the acceptance gate. Bounded rejection handles
    /// combination/rarity constraints without a second deck validator.
    fn complete(&self, e: &BeliefEvidence, rng: &mut StdRng) -> Result<DeckHypothesis, String> {
        let required = e.required();
        let public_eggs: usize = e
            .accounted
            .iter()
            .filter(|(id, _)| egg(id))
            .map(|(_, n)| n)
            .sum();
        for _ in 0..self.snapshot.fallback_attempts {
            let mut c = required.clone();
            let mut failed = false;
            for (is_egg, target) in [
                (false, self.fmt.deck_size as usize),
                (true, public_eggs + e.digitama_count),
            ] {
                let have: usize = c
                    .iter()
                    .filter(|(id, _)| egg(id) == is_egg)
                    .map(|(_, n)| n)
                    .sum();
                if have > target {
                    failed = true;
                    break;
                }
                for _ in have..target {
                    let options: Vec<_> = self
                        .snapshot
                        .population
                        .iter()
                        .filter(|id| {
                            let limit = card_legality_for_descriptor(id, self.fmt);
                            egg(id) == is_egg
                                && limit.legal
                                && c.get(*id).copied().unwrap_or(0) < limit.max_copies as usize
                        })
                        .collect();
                    if options.is_empty() {
                        failed = true;
                        break;
                    }
                    let id = options[rng.gen_range(0..options.len())];
                    *c.entry(id.clone()).or_default() += 1;
                }
                if failed {
                    break;
                }
            }
            if failed {
                continue;
            }
            let deck = expanded(&c);
            if validate_deck_for_descriptor(&deck, self.fmt).is_valid && self.consistent(&deck, e) {
                return self.hypothesis(deck, e, true);
            }
        }
        Err(format!(
            "legal completion exhausted {} attempts; no hidden-truth fallback",
            self.snapshot.fallback_attempts
        ))
    }
}
