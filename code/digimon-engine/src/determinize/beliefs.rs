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

pub const BELIEF_SCHEMA_VERSION: u32 = 1;

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
        })
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
        Ok(())
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
    }

    /// An unobserved transfer cannot be resolved using the true instance that
    /// moved. Conservatively invalidate membership facts in affected zones.
    /// A future probabilistic ledger can preserve stronger lower bounds.
    pub fn forget_zone(&mut self, owner: PlayerId, zone: KnowledgeZone) {
        self.cards.retain(|c| c.owner != owner || c.zone != zone);
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
    pub accounted: CardMultiset,
    pub known_hidden: Vec<(String, KnowledgeZone)>,
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
            accounted,
            known_hidden: Vec::new(),
        })
    }

    /// Read CURRENT public zones only. Memory must come from entitled events.
    /// Face-down sources and opaque games require additional knowledge plumbing;
    /// fail closed rather than exposing their identities.
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
        let mut accounted = CardMultiset::new();
        let mut public_instances = BTreeSet::new();
        let mut add = |c: &crate::card_source::CardSource| -> Result<(), String> {
            if c.is_token || c.owner != opponent {
                return Ok(());
            }
            if c.face_down || c.is_opaque_placeholder {
                return Err(
                    "public zone contains a hidden source; knowledge integration required".into(),
                );
            }
            if !public_instances.insert(c.card_index) {
                return Err("physical card appears in two public zones".into());
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
            for c in public_player
                .security
                .iter()
                .filter(|c| public_player.face_up_security.contains(&c.card_index))
            {
                add(c)?;
            }
        }
        for c in &game.revealed_cards {
            add(c)?;
        }
        let mut result = Self::from_counts(
            viewer,
            opponent,
            p.hand.len(),
            p.deck.len(),
            p.security
                .iter()
                .filter(|c| !p.face_up_security.contains(&c.card_index))
                .count(),
            p.digitama_deck.len(),
            accounted,
        )?;
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
        Ok(result)
    }

    fn required(&self) -> CardMultiset {
        let mut required = self.accounted.clone();
        for (id, _) in &self.known_hidden {
            *required.entry(id.clone()).or_default() += 1;
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
