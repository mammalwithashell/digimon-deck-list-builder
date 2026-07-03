//! Viewer-relative [`Infoset`] extraction — the inverse of
//! `server/state_filter.py`'s PvP redaction (design D2).
//!
//! The infoset carries exactly what the viewing player is entitled to
//! observe:
//!
//! - **Public state** verbatim: battle areas, breeding, trash, revealed
//!   cards, memory/turn/phase scalars, per-zone counts, and face-up
//!   security pins (face-up security is public to both players).
//! - **Viewer's own hand**: exact card ids, in order.
//! - **Viewer's own hidden model**: one JOINT unseen pool — own deck ∪ own
//!   face-down security — plus the count partition (deck count vs face-down
//!   security count). Own security is hidden from its *owner* too
//!   (`state_filter` redacts the viewer's own `securityIds`), so the two
//!   zones are NOT independent multisets (D2 sharpening, 2026-07-01). The
//!   own digitama deck is a separate multiset (egg cards are kind-segregated
//!   — they can never occupy main-deck/hand/security slots — so a joint pool
//!   with the main zones would never mix them anyway); only its ORDER is
//!   unknown.
//! - **Opponent hidden model**: per-zone counts + a [`DeckPrior`]. In the
//!   self-play regime (task 1.6, the only regime in Phase 1) the prior is
//!   `DeckPrior::Exact`: the opponent's decklist minus everything the viewer
//!   has observed — equivalently, the multiset of the opponent's
//!   hand ∪ deck ∪ face-down security. Extraction reads the true hidden
//!   zones only to form that *multiset* (entitled knowledge under a known
//!   decklist); no concealed identity-to-position assignment is stored, and
//!   the type system has nowhere to put one.
//!
//! **Pins (v1):** the only per-card revelation state the engine tracks is
//! `Player::face_up_security`, so v1 pins = face-up security cards (slot
//! index + id). The engine does NOT track "opponent revealed this hand card
//! earlier" or "player X peeked at the top 3 of their deck and returned
//! them" — that knowledge is lost at extraction (documented belief-staleness
//! gap; feeds the deck-prior work, Phase 6). Cards mid-reveal live in
//! `Game::revealed_cards` and are captured as public state.

use std::collections::BTreeMap;

use crate::card_source::CardSource;
use crate::enums::{GamePhase, PlayerId};
use crate::game::Game;
use crate::permanent::Permanent;
use crate::player::Player;

/// Order-free composition of a card pool: card_id → count. `BTreeMap` so
/// `Debug`/`PartialEq`/iteration are deterministic.
pub type CardMultiset = BTreeMap<String, usize>;

pub(crate) fn multiset_insert(ms: &mut CardMultiset, id: &str) {
    *ms.entry(id.to_string()).or_insert(0) += 1;
}

pub(crate) fn zone_ids(cards: &[CardSource], game: &Game) -> Vec<String> {
    cards
        .iter()
        .map(|c| c.card_id(&game.card_data).to_string())
        .collect()
}

pub(crate) fn zone_multiset(cards: &[CardSource], game: &Game) -> CardMultiset {
    let mut ms = CardMultiset::new();
    for c in cards {
        multiset_insert(&mut ms, c.card_id(&game.card_data));
    }
    ms
}

/// Public view of one permanent (digivolution stack on the field or in
/// breeding). Everything here is face-up / public.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PermanentView {
    /// Card ids of the digivolution stack, base → top.
    pub stack: Vec<String>,
    /// Card ids of sideways-linked cards (Options, DigiLink).
    pub linked: Vec<String>,
    pub is_suspended: bool,
    pub top_is_token: bool,
}

/// Public per-player state — what BOTH players (and spectators) see.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublicPlayerView {
    pub battle_area: Vec<PermanentView>,
    pub breeding: Option<PermanentView>,
    /// Trash card ids, in stack order (trash is public and ordered).
    pub trash: Vec<String>,
    pub hand_count: usize,
    pub deck_count: usize,
    /// Total security count (face-up pins + face-down).
    pub security_count: usize,
    pub digitama_count: usize,
    /// Face-up security cards: (slot index in the security stack, card_id).
    /// Face-up security is public knowledge — these are the v1 "pins".
    pub security_pins: Vec<(usize, String)>,
}

/// The viewer's own hidden model (D2 sharpening): one joint unseen pool
/// with a count partition, NOT independent per-zone multisets.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OwnHiddenModel {
    /// Exact hand card ids, in hand order (fully visible to the owner).
    pub hand: Vec<String>,
    /// Joint unseen pool: own deck ∪ own face-down security.
    pub unseen_pool: CardMultiset,
    /// Count partition of `unseen_pool`: how many sit in the deck…
    pub deck_count: usize,
    /// …and how many sit face-down in security.
    pub face_down_security_count: usize,
    /// Remaining digitama (egg) deck composition; order unknown.
    pub digitama_pool: CardMultiset,
}

/// Belief over the opponent's unseen cards. Phase 1 implements only the
/// self-play regime (D7: both decklists come from the known training pool),
/// where the prior is exact. PvP inference (archetype classifier etc.) lands
/// in Phase 6 as additional variants.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeckPrior {
    /// The exact unseen multiset: decklist minus everything observed
    /// (= opponent hand ∪ deck ∪ face-down security).
    Exact(CardMultiset),
}

impl DeckPrior {
    pub fn multiset(&self) -> &CardMultiset {
        match self {
            DeckPrior::Exact(ms) => ms,
        }
    }

    pub fn total(&self) -> usize {
        self.multiset().values().sum()
    }
}

/// The opponent's hidden model: per-zone counts + prior. Pins (face-up
/// security) live in the PUBLIC view — they are visible to everyone.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HiddenModel {
    pub hand_count: usize,
    pub deck_count: usize,
    pub face_down_security_count: usize,
    pub deck_prior: DeckPrior,
    /// Remaining digitama (egg) deck composition; order unknown.
    pub digitama_pool: CardMultiset,
}

/// Everything the viewing player may observe of a 2-player game — the
/// information set determinized search samples worlds from.
///
/// Round-trip law (spec requirement 3): for any world sampled from this
/// infoset for the same viewer, re-extraction yields an equal `Infoset`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Infoset {
    pub viewer: PlayerId,
    pub opponent: PlayerId,

    // ── public scalars ──
    pub turn_count: u16,
    pub current_phase: GamePhase,
    pub memory: i16,
    pub memory_pair: (PlayerId, PlayerId),
    pub turn_order: Vec<PlayerId>,
    pub turn_player_idx: usize,
    pub game_over: bool,
    pub winner: Option<PlayerId>,
    pub mulligan_pending: Vec<PlayerId>,
    pub mulligan_used: Vec<bool>,

    /// Cards currently revealed to all players: (owner, card_id).
    pub revealed_cards: Vec<(PlayerId, String)>,
    /// Parked player-choice prompt, summarized as (selecting player, kind).
    /// Valid indices are intentionally NOT captured: an index set over a
    /// hidden zone can itself leak identity information.
    pub pending_selection: Option<(PlayerId, String)>,
    pub pending_attack: bool,

    /// Public per-player views, indexed by absolute `PlayerId`.
    pub public: Vec<PublicPlayerView>,
    /// The viewer's own hidden model (joint pool + partition).
    pub own: OwnHiddenModel,
    /// The opponent's hidden model (counts + prior).
    pub opponent_model: HiddenModel,
}

fn permanent_view(perm: &Permanent, game: &Game) -> PermanentView {
    PermanentView {
        stack: zone_ids(&perm.card_sources, game),
        linked: zone_ids(&perm.linked_cards, game),
        is_suspended: perm.is_suspended,
        top_is_token: perm.card_sources.last().is_some_and(|c| c.is_token),
    }
}

/// Face-up security entries of `p`: (slot index, card_id).
pub(crate) fn security_pins(p: &Player, game: &Game) -> Vec<(usize, String)> {
    p.security
        .iter()
        .enumerate()
        .filter(|(_, c)| p.face_up_security.contains(&c.card_index))
        .map(|(i, c)| (i, c.card_id(&game.card_data).to_string()))
        .collect()
}

fn public_view(game: &Game, pid: PlayerId) -> PublicPlayerView {
    let p = &game.players[pid as usize];
    PublicPlayerView {
        battle_area: p.battle_area.iter().map(|x| permanent_view(x, game)).collect(),
        breeding: p.breeding_area.as_ref().map(|x| permanent_view(x, game)),
        trash: zone_ids(&p.trash, game),
        hand_count: p.hand.len(),
        deck_count: p.deck.len(),
        security_count: p.security.len(),
        digitama_count: p.digitama_deck.len(),
        security_pins: security_pins(p, game),
    }
}

impl Infoset {
    /// Extract the information set of `viewer` from a 2-player, non-opaque
    /// (self-play regime) game.
    ///
    /// Panics if the game is not 2-player or if either player is in
    /// opaque-deck mode (opaque games are PvP-replay territory — their
    /// hidden model comes from `OpaqueDeckState`, deferred to the Phase 6
    /// deck-prior work).
    pub fn extract(game: &Game, viewer: PlayerId) -> Infoset {
        assert_eq!(
            game.players.len(),
            2,
            "Infoset::extract supports 2-player games only"
        );
        assert!((viewer as usize) < 2, "viewer out of range");
        assert!(
            game.players.iter().all(|p| p.opaque_deck_state.is_none()),
            "Phase-1 determinization supports self-play (non-opaque) games only"
        );

        let opponent = 1 - viewer;
        let me = &game.players[viewer as usize];
        let opp = &game.players[opponent as usize];

        // Own joint unseen pool: deck ∪ face-down security.
        let mut own_pool = zone_multiset(&me.deck, game);
        let mut own_face_down = 0usize;
        for c in &me.security {
            if !me.face_up_security.contains(&c.card_index) {
                multiset_insert(&mut own_pool, c.card_id(&game.card_data));
                own_face_down += 1;
            }
        }

        // Opponent unseen pool: hand ∪ deck ∪ face-down security. Only the
        // MULTISET is retained (see module docs for the entitlement
        // argument); the assignment is discarded here.
        let mut opp_pool = zone_multiset(&opp.hand, game);
        for c in &opp.deck {
            multiset_insert(&mut opp_pool, c.card_id(&game.card_data));
        }
        let mut opp_face_down = 0usize;
        for c in &opp.security {
            if !opp.face_up_security.contains(&c.card_index) {
                multiset_insert(&mut opp_pool, c.card_id(&game.card_data));
                opp_face_down += 1;
            }
        }

        Infoset {
            viewer,
            opponent,
            turn_count: game.turn_count,
            current_phase: game.current_phase,
            memory: game.memory,
            memory_pair: game.memory_pair,
            turn_order: game.turn_order.clone(),
            turn_player_idx: game.turn_player_idx,
            game_over: game.game_over,
            winner: game.winner,
            mulligan_pending: game.mulligan_pending.clone(),
            mulligan_used: game.mulligan_used.clone(),
            revealed_cards: game
                .revealed_cards
                .iter()
                .map(|c| (c.owner, c.card_id(&game.card_data).to_string()))
                .collect(),
            pending_selection: game
                .pending_selection
                .as_ref()
                .map(|s| (s.selecting_player, format!("{:?}", s.kind))),
            pending_attack: game.pending_attack.is_some(),
            public: (0..2).map(|pid| public_view(game, pid as PlayerId)).collect(),
            own: OwnHiddenModel {
                hand: zone_ids(&me.hand, game),
                unseen_pool: own_pool,
                deck_count: me.deck.len(),
                face_down_security_count: own_face_down,
                digitama_pool: zone_multiset(&me.digitama_deck, game),
            },
            opponent_model: HiddenModel {
                hand_count: opp.hand.len(),
                deck_count: opp.deck.len(),
                face_down_security_count: opp_face_down,
                deck_prior: DeckPrior::Exact(opp_pool),
                digitama_pool: zone_multiset(&opp.digitama_deck, game),
            },
        }
    }
}
