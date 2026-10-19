use std::collections::HashSet;

use rand::seq::SliceRandom;
use rand::Rng;

use crate::card_data::CardData;
use crate::card_source::CardSource;
use crate::enums::PlayerId;
use crate::opaque_deck::OpaqueDeckState;
use crate::permanent::Permanent;
use crate::rules::Rules;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OriginalDeckCardCount {
    pub card_id: String,
    pub count: u16,
    pub is_digitama: bool,
}

/// A player in the game with all their zones.
#[derive(Debug, Clone)]
pub struct Player {
    pub id: PlayerId,
    pub hand: Vec<CardSource>,
    pub deck: Vec<CardSource>,
    pub digitama_deck: Vec<CardSource>,
    pub security: Vec<CardSource>,
    pub trash: Vec<CardSource>,
    pub battle_area: Vec<Permanent>,
    pub breeding_area: Option<Permanent>,
    pub original_deck: Vec<OriginalDeckCardCount>,

    /// `CardSource.card_index` of security cards currently face-up (visible
    /// to this player and encoded into the observation tensor). Security is
    /// laid face-down by default; effects that reveal security flip entries
    /// in here. Matches Python's `Player.face_up_security`.
    pub face_up_security: HashSet<u16>,

    /// Snapshot of the most recently revealed security card, set by
    /// `Game::resolve_security_card` before firing `SecuritySkill` effects.
    /// Consumed by `OnSecurityCheck` observer effects so they can inspect
    /// the revealed card even after `pending_security` has been cleared.
    /// Mirrors Python's `_last_security_card` / `_last_security_was_face_up`
    /// pair (RUST_PYTHON_PARITY §2.5l).
    pub last_security_reveal: Option<crate::selection::SecurityRevealSnapshot>,

    // Commander/multiplayer fields
    pub commander_zone: Option<CardSource>,
    pub commander_tax: u16,
    pub is_eliminated: bool,

    /// `Some` when this player is in **opaque-deck mode** — the engine
    /// does not know this player's deck order, only its composition.
    /// Draws and security pops consult `Game::reveal_source` instead of
    /// reading from `self.deck`. The `deck` field remains empty in
    /// opaque mode (kept as `Vec<CardSource>` only to avoid widening
    /// the type; `deck.is_empty()` does NOT imply deck-out when
    /// `opaque_deck_state.is_some()` — callers must check
    /// `opaque_deck_state` first).
    ///
    /// See `crate::opaque_deck` for the contract. Set only by
    /// `Game::new_with_opaque_opponent`.
    pub opaque_deck_state: Option<OpaqueDeckState>,
}

impl Player {
    /// Create a new player with empty zones.
    pub fn new(id: PlayerId) -> Self {
        Self {
            id,
            hand: Vec::new(),
            deck: Vec::new(),
            digitama_deck: Vec::new(),
            security: Vec::new(),
            trash: Vec::new(),
            battle_area: Vec::new(),
            breeding_area: None,
            original_deck: Vec::new(),
            face_up_security: HashSet::new(),
            last_security_reveal: None,
            commander_zone: None,
            commander_tax: 0,
            is_eliminated: false,
            opaque_deck_state: None,
        }
    }

    /// True when this player's deck composition is known but its order is
    /// not — draws must consult `Game::reveal_source` rather than `self.deck`.
    pub fn is_opaque_deck(&self) -> bool {
        self.opaque_deck_state.is_some()
    }

    /// Draw one card from deck to hand. Returns false if deck is empty (deck-out).
    pub fn draw(&mut self) -> bool {
        if let Some(card) = self.deck.pop() {
            self.put_in_hand(card);
            true
        } else {
            false
        }
    }

    /// Remove the first card in `deck` matching `handle`. Returns the removed
    /// card if found.
    pub fn remove_from_deck_by_handle(
        &mut self,
        handle: crate::card_source::CardHandle,
    ) -> Option<crate::card_source::CardSource> {
        let pos = self.deck.iter().position(|c| c.handle() == handle)?;
        Some(self.deck.remove(pos))
    }

    /// Remove the first card in `trash` matching `handle`.
    pub fn remove_from_trash_by_handle(
        &mut self,
        handle: crate::card_source::CardHandle,
    ) -> Option<crate::card_source::CardSource> {
        let pos = self.trash.iter().position(|c| c.handle() == handle)?;
        Some(self.trash.remove(pos))
    }

    /// Append `card` to hand.
    pub fn add_to_hand(&mut self, card: crate::card_source::CardSource) {
        self.put_in_hand(card);
    }

    /// Draw multiple cards. Returns number actually drawn.
    pub fn draw_many(&mut self, count: u8) -> u8 {
        let mut drawn = 0;
        for _ in 0..count {
            if self.draw() {
                drawn += 1;
            } else {
                break;
            }
        }
        drawn
    }

    /// Shuffle the main deck.
    pub fn shuffle_deck(&mut self, rng: &mut impl Rng) {
        self.deck.shuffle(rng);
    }

    /// Shuffle the digitama deck.
    pub fn shuffle_digitama_deck(&mut self, rng: &mut impl Rng) {
        self.digitama_deck.shuffle(rng);
    }

    pub fn contains_card(&self, handle: crate::card_source::CardHandle) -> bool {
        self.hand.iter().any(|card| card.handle() == handle)
            || self.deck.iter().any(|card| card.handle() == handle)
            || self.trash.iter().any(|card| card.handle() == handle)
            || self.security.iter().any(|card| card.handle() == handle)
            || self
                .digitama_deck
                .iter()
                .any(|card| card.handle() == handle)
            || self
                .breeding_area
                .as_ref()
                .is_some_and(|perm| permanent_contains_card(perm, handle))
            || self
                .battle_area
                .iter()
                .any(|perm| permanent_contains_card(perm, handle))
    }

    // ── Zone entry ───────────────────────────────────────────────────────
    //
    // Every card entering this player's trash, hand, security stack, deck or
    // Digi-Egg deck goes
    // through one of these, so each area's placement rule holds by
    // construction. A card moved to another area is a new card that keeps
    // none of its previous statuses or states (§3-1-3-1-1 / §3-1-3-1-2) and is
    // placed by the rules of its new area unless specified otherwise
    // (§3-1-3-5). `CardSource::face_down` is the state a digivolution card
    // stashed face-down under a permanent carries (§4-7-9); it must not leak
    // into another area. A source-scan guard
    // (`player::tests::zone_entries_go_through_the_player_helpers`) keeps
    // direct pushes into those zones out of `src/`.

    /// Put `card` on top of the trash. The trash is a public area; cards are
    /// placed face-up in it (§3-6-3).
    pub fn put_in_trash(&mut self, mut card: CardSource) {
        card.face_down = false;
        self.trash.push(card);
    }

    /// As [`Self::put_in_trash`], at `index` (a rollback restoring a card to
    /// the position it was taken from, §3-6-2 lets the order change anyway).
    pub fn insert_in_trash(&mut self, index: usize, mut card: CardSource) {
        card.face_down = false;
        self.trash.insert(index, card);
    }

    /// [`Self::put_in_trash`] each card in order.
    pub fn put_all_in_trash(&mut self, cards: impl IntoIterator<Item = CardSource>) {
        for card in cards {
            self.put_in_trash(card);
        }
    }

    /// Put `card` in the hand. The hand is the owner's private area, which
    /// they may always look at (§3-5-3): a hand card carries no face-down
    /// placement.
    pub fn put_in_hand(&mut self, mut card: CardSource) {
        card.face_down = false;
        self.hand.push(card);
    }

    /// As [`Self::put_in_hand`], at `index`.
    pub fn insert_in_hand(&mut self, index: usize, mut card: CardSource) {
        card.face_down = false;
        self.hand.insert(index, card);
    }

    /// [`Self::put_in_hand`] each card in order.
    pub fn put_all_in_hand(&mut self, cards: impl IntoIterator<Item = CardSource>) {
        for card in cards {
            self.put_in_hand(card);
        }
    }

    /// Put `card` on top of the security stack. Security cards are placed
    /// face-down (§3-7-2, §3-1-3-5's example); the engine records a security
    /// card's face by `face_up_security` membership (absent = face-down, the
    /// state a face-up placement (§3-7-4) adds explicitly), so a stale
    /// `face_down` flag from a digivolution card is cleared rather than
    /// duplicated. An opaque-deck placeholder keeps its own marker.
    pub fn put_on_security_top(&mut self, card: CardSource) {
        let index = self.security.len();
        self.insert_in_security(index, card);
    }

    /// As [`Self::put_on_security_top`], at `index` (0 = bottom).
    pub fn insert_in_security(&mut self, index: usize, mut card: CardSource) {
        if !card.is_opaque_placeholder {
            card.face_down = false;
        }
        // A new card arrives face down: drop any stale face-up record for its
        // instance. A face-up placement (§3-7-4) marks it after this insert.
        self.face_up_security.remove(&card.card_index);
        self.security.insert(index, card);
    }

    /// Put `card` on top of the deck (the end drawn first). The deck is a
    /// private area whose cards are all placed face-down (§3-2-2): the area
    /// carries that face, so the per-card `face_down` state a digivolution
    /// card had under a permanent is dropped (§3-1-3-1-2) -- otherwise a later
    /// placement under a permanent that does not set a face would count it as
    /// a face-down card (§4-7-9).
    pub fn put_on_deck_top(&mut self, card: CardSource) {
        let index = self.deck.len();
        self.insert_in_deck(index, card);
    }

    /// As [`Self::put_on_deck_top`], at `index` (0 = bottom).
    pub fn insert_in_deck(&mut self, index: usize, mut card: CardSource) {
        card.face_down = false;
        self.deck.insert(index, card);
    }

    /// [`Self::put_on_deck_top`] each card in order (the last ends on top).
    pub fn put_all_on_deck_top(&mut self, cards: impl IntoIterator<Item = CardSource>) {
        for card in cards {
            self.put_on_deck_top(card);
        }
    }

    /// Insert `cards` at `index` of the deck in order (0 = bottom), as one
    /// block: `cards[0]` ends at `index`.
    pub fn insert_all_in_deck(&mut self, index: usize, cards: impl IntoIterator<Item = CardSource>) {
        for (offset, card) in cards.into_iter().enumerate() {
            self.insert_in_deck(index + offset, card);
        }
    }

    /// Put `card` on top of the Digi-Egg deck. The Digi-Egg deck is a private
    /// area whose cards are placed face-down (§3-3-2); as for the deck, the
    /// per-card `face_down` state is dropped (§3-1-3-1-2).
    pub fn put_on_digi_egg_deck_top(&mut self, card: CardSource) {
        let index = self.digitama_deck.len();
        self.insert_in_digi_egg_deck(index, card);
    }

    /// As [`Self::put_on_digi_egg_deck_top`], at `index` (0 = bottom).
    pub fn insert_in_digi_egg_deck(&mut self, index: usize, mut card: CardSource) {
        card.face_down = false;
        self.digitama_deck.insert(index, card);
    }

    /// Insert `cards` at `index` of the Digi-Egg deck in order, as one block.
    pub fn insert_all_in_digi_egg_deck(
        &mut self,
        index: usize,
        cards: impl IntoIterator<Item = CardSource>,
    ) {
        for (offset, card) in cards.into_iter().enumerate() {
            self.insert_in_digi_egg_deck(index + offset, card);
        }
    }

    /// Route a card going to "the deck": a Digi-Egg card goes to the Digi-Egg
    /// deck instead. `to_bottom` picks the bottom (index 0) over the top.
    pub fn put_in_deck_or_digi_egg_deck(&mut self, card: CardSource, is_egg: bool, to_bottom: bool) {
        match (is_egg, to_bottom) {
            (true, true) => self.insert_in_digi_egg_deck(0, card),
            (true, false) => self.put_on_digi_egg_deck_top(card),
            (false, true) => self.insert_in_deck(0, card),
            (false, false) => self.put_on_deck_top(card),
        }
    }

    /// Set up the security stack by moving cards from the top of the deck.
    pub fn setup_security(&mut self, count: u8) {
        for _ in 0..count {
            if let Some(card) = self.deck.pop() {
                self.put_on_security_top(card);
            }
        }
    }

    /// Hatch: move top card from digitama deck to breeding area.
    /// Returns true if successful (digitama deck not empty, breeding area empty).
    pub fn hatch(&mut self, turn: u16) -> bool {
        if self.breeding_area.is_some() {
            return false;
        }
        if let Some(egg) = self.digitama_deck.pop() {
            self.breeding_area = Some(Permanent::new(egg, turn));
            true
        } else {
            false
        }
    }

    /// Move from breeding area to battle area.
    /// Returns true if successful (breeding area occupied and has room).
    pub fn move_from_breeding(&mut self, rules: &Rules) -> bool {
        if self.battle_area.len() >= rules.field_slots as usize {
            return false;
        }
        if let Some(mut perm) = self.breeding_area.take() {
            // §17-1-3-2-3's marker describes the breeding area only.
            perm.placed_by_effect = false;
            self.battle_area.push(perm);
            true
        } else {
            false
        }
    }

    /// Play a card from hand to the field (creates a new Permanent).
    /// Returns the index in battle_area, or None if hand index is invalid or field is full.
    pub fn play_from_hand(&mut self, hand_index: usize, turn: u16, rules: &Rules) -> Option<usize> {
        if hand_index >= self.hand.len() {
            return None;
        }
        if self.battle_area.len() >= rules.field_slots as usize {
            return None;
        }
        let card = self.hand.remove(hand_index);
        let perm = Permanent::new(card, turn);
        self.battle_area.push(perm);
        Some(self.battle_area.len() - 1)
    }

    /// Remove a permanent from the battle area and send all its cards to trash.
    ///
    /// Tolerates an empty `card_sources`: a permanent whose stack was fully
    /// drained mid-deletion (e.g. printed `<Save>` lifted the top card
    /// under a Tamer before the deferred finalizer ran) has nothing left
    /// to trash; the slot is simply removed.
    pub fn delete_permanent(&mut self, field_index: usize) {
        if field_index >= self.battle_area.len() {
            return;
        }
        let perm = self.battle_area.remove(field_index);
        // Empty-stack guard: nothing to inspect (no top card to check
        // for token semantic), nothing to trash. Just drop the slot.
        if perm.card_sources.is_empty() {
            // Linked cards still need to flow to trash so they don't
            // leak — but in practice a permanent with no top will have
            // had its linked cards already drained by the surrounding
            // deletion-finalizer. Defensive parity with the non-empty
            // branch below.
            for card in perm.linked_cards {
                self.put_in_trash(card);
            }
            return;
        }
        // Token semantic: remove from game. Drop the whole stack on the
        // floor — no trash entry, no zone ever again. Parity with
        // Python's `player.py::delete_permanent` is_token branch.
        if perm.top_card().is_token {
            return;
        }
        for card in perm.card_sources {
            self.put_in_trash(card);
        }
        for card in perm.linked_cards {
            self.put_in_trash(card);
        }
    }

    /// Number of cards in hand.
    pub fn hand_size(&self) -> usize {
        self.hand.len()
    }

    /// Number of permanents on the field (battle area).
    pub fn field_count(&self) -> usize {
        self.battle_area.len()
    }

    /// Number of security cards.
    pub fn security_count(&self) -> usize {
        self.security.len()
    }

    /// Total DP across all Digimon on the field (for reward shaping).
    pub fn total_field_dp(&self, data: &[CardData]) -> i32 {
        self.battle_area
            .iter()
            .filter_map(|p| p.base_dp(data))
            .sum()
    }

    /// Reset per-turn state for all permanents.
    pub fn new_turn(&mut self) {
        for perm in &mut self.battle_area {
            perm.new_turn();
        }
    }

    /// Reset only the [Once Per Turn] activation counters. "Per turn" means
    /// per GAME turn, so an [All Turns][Once Per Turn] effect re-arms at the
    /// start of EVERY turn — including the opponent's — while attack
    /// counters remain owner-turn-scoped (cleared via `new_turn`).
    pub fn reset_effect_activations(&mut self) {
        for perm in &mut self.battle_area {
            perm.effect_activations.clear();
        }
    }

    /// Unsuspend all permanents (start of turn).
    pub fn unsuspend_all(&mut self) {
        for perm in &mut self.battle_area {
            perm.is_suspended = false;
        }
        if let Some(ref mut perm) = self.breeding_area {
            perm.is_suspended = false;
        }
    }
}

fn permanent_contains_card(
    permanent: &crate::permanent::Permanent,
    handle: crate::card_source::CardHandle,
) -> bool {
    permanent
        .card_sources
        .iter()
        .any(|card| card.handle() == handle)
        || permanent
            .linked_cards
            .iter()
            .any(|card| card.handle() == handle)
}

#[cfg(test)]
mod tests {
    /// Every card entering a trash, hand, security stack, deck or Digi-Egg
    /// deck goes through the `Player` zone-entry helpers (`put_in_trash`,
    /// `put_in_hand`, `insert_in_security`, `insert_in_deck`,
    /// `insert_in_digi_egg_deck`, ...), which apply the area's placement rule
    /// (§3-6-3 / §3-5-3 / §3-7-2 / §3-2-2 / §3-3-2) and drop the `face_down`
    /// state a card carried as a digivolution card (§3-1-3-1-2). This scan of
    /// `src/` fails on a direct push into one of those `Vec`s -- also when the
    /// method chain spans lines -- and on a local `&mut` alias of one (the way
    /// a push used to hide from a line scan); the helper bodies in this file
    /// are the only pushes allowed.
    ///
    /// Not covered, on purpose: whole-zone rebuilds that move a zone's own
    /// cards (`std::mem::take` + reassign for a shuffle or a mulligan) and the
    /// state builders that create fresh cards (`runners::replay` recordings,
    /// `determinize` / `search` re-deals of hidden cards).
    #[test]
    fn zone_entries_go_through_the_player_helpers() {
        let push = regex::Regex::new(
            r"\.\s*(trash|hand|security|deck|digitama_deck)\s*\.\s*(push|insert|extend|append|splice|extend_from_slice)\s*\(",
        )
        .expect("regex");
        let alias = regex::Regex::new(
            r"(=|\{)\s*&mut\s+[^;{}=]*?\.(trash|hand|security|deck|digitama_deck)\s*(;|\})",
        )
        .expect("regex");
        fn walk(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
            for entry in std::fs::read_dir(dir).expect("read src dir") {
                let path = entry.expect("dir entry").path();
                if path.is_dir() {
                    walk(&path, out);
                } else if path.extension().is_some_and(|e| e == "rs") {
                    out.push(path);
                }
            }
        }
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let this_file = root.join("player.rs");
        let mut files = Vec::new();
        walk(&root, &mut files);
        let mut direct = Vec::new();
        let mut in_helpers = 0;
        for file in files {
            let text = std::fs::read_to_string(&file).expect("read source");
            let line_of = |at: usize| text[..at].matches('\n').count() + 1;
            for m in push.find_iter(&text).chain(alias.find_iter(&text)) {
                if file == this_file {
                    in_helpers += 1;
                } else {
                    direct.push(format!("{}:{}: {}", file.display(), line_of(m.start()), m.as_str()));
                }
            }
        }
        assert!(
            direct.is_empty(),
            "zone entries that bypass the Player zone-entry helpers:\n{}",
            direct.join("\n")
        );
        // The bodies of insert_in_trash, put_in_trash, put_in_hand,
        // insert_in_hand, insert_in_security, insert_in_deck and
        // insert_in_digi_egg_deck -- nothing else in this file.
        assert_eq!(in_helpers, 7, "player.rs pushes outside the zone-entry helpers");
    }
}
