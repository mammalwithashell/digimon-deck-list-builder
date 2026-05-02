"""Deck pool analysis, variant generation, and training wrapper.

Provides tools for decomposing a base deck into core vs flex cards,
generating valid deck variants via stochastic modifications, and a
Gymnasium wrapper that varies the agent's own deck per episode.

Core/flex analysis identifies which cards are essential to the deck's
strategy (core) and which can be swapped or adjusted (flex).  Variant
generation produces legal 50-card decks by making small, random
modifications to flex slots, optionally incorporating side-board cards.

DeckPoolWrapper integrates into the training wrapper chain to expose
the agent to its own deck variants during self-play, improving
robustness to minor deck composition changes.

Wrapper chain position:
    DigimonEnv -> OpponentWrapper -> DeckPoolWrapper -> GauntletWrapper -> ActionMasker
"""

from __future__ import annotations

import logging
from collections import Counter
from typing import Callable, Dict, List, Optional, Set, Tuple

import gymnasium
import numpy as np

from digimon_engine import CardDatabase, CardKind, expand_deck_dict, validate_deck

logger = logging.getLogger(__name__)


# ─── Core/Flex Analysis ─────────────────────────────────────────────


def analyze_core(
    card_ids: List[str],
    core_overrides: Optional[Dict[str, bool]] = None,
) -> Tuple[Dict[str, int], Dict[str, int]]:
    """Identify core vs flex cards in a deck list.

    Core cards are those present at their maximum allowed copy count
    (typically 4-of), indicating they are essential to the deck's
    strategy.  Flex cards have room for adjustment.

    Digi-Egg cards are excluded from both sets since eggs occupy a
    separate deck zone and are not subject to main-deck flex swaps.

    Args:
        card_ids: Flat list of card ID strings (one per copy).
        core_overrides: Optional mapping of card_id -> bool.
            ``True`` promotes a card to core, ``False`` demotes to flex.

    Returns:
        Tuple of (core_cards, flex_cards) where each is a
        ``{card_id: count}`` dict.
    """
    if core_overrides is None:
        core_overrides = {}

    db = CardDatabase()
    counts = Counter(card_ids)

    core_cards: Dict[str, int] = {}
    flex_cards: Dict[str, int] = {}

    for card_id, count in counts.items():
        entity = db.get_card(card_id)

        # Skip Digi-Egg cards entirely (separate deck zone)
        if entity is not None and entity.card_kind == CardKind.DigiEgg:
            continue

        # Check user override first
        if card_id in core_overrides:
            if core_overrides[card_id]:
                core_cards[card_id] = count
            else:
                flex_cards[card_id] = count
            continue

        # Auto-classify: at max copies -> core, otherwise flex
        if entity is not None:
            if count == entity.max_count_in_deck:
                core_cards[card_id] = count
            else:
                flex_cards[card_id] = count
        else:
            # Unknown card defaults to flex
            flex_cards[card_id] = count

    return core_cards, flex_cards


# ─── Variant Generation ─────────────────────────────────────────────


def generate_variants(
    base_deck: List[str],
    core_cards: Dict[str, int],
    flex_cards: Dict[str, int],
    side_cards: List[str],
    count: int = 4,
    seed: Optional[int] = None,
    egg_deck: Optional[List[str]] = None,
    vary_eggs: bool = False,
) -> List[List[str]]:
    """Generate valid deck variants from a base deck.

    Produces up to *count* unique, validated 50-card decks by making
    small stochastic modifications to flex card counts and optionally
    swapping in side-board cards.  Core cards are never touched.

    The algorithm over-generates candidates (up to ``count * 3``) to
    account for validation failures and deduplication, then returns
    at most *count* variants.

    Args:
        base_deck: Flat list of card IDs for the base deck.
        core_cards: ``{card_id: count}`` of core (untouchable) cards.
        flex_cards: ``{card_id: count}`` of flex (modifiable) cards.
        side_cards: Flat list of side-board card IDs available for swaps.
        count: Number of variants to produce.
        seed: RNG seed for reproducibility.
        egg_deck: Optional egg deck (reserved for future use).
        vary_eggs: Placeholder flag for future egg variation support.

    Returns:
        List of flat card-ID lists, each a valid 50-card deck.
    """
    rng = np.random.default_rng(seed)
    db = CardDatabase()

    variants: List[List[str]] = []
    max_attempts = count * 3

    flex_pool = list(flex_cards.keys())

    for _ in range(max_attempts):
        if len(variants) >= count:
            break

        deck_counts = Counter(base_deck)

        # Build side pool: side cards not already at max copies
        side_pool: List[str] = []
        for c in set(side_cards):
            entity = db.get_card(c)
            max_copies = entity.max_count_in_deck if entity else 4
            if deck_counts.get(c, 0) < max_copies:
                side_pool.append(c)

        # Number of modifications scales with flex pool size
        n_flex = len(flex_pool)
        if n_flex < 8:
            n_mods = int(rng.integers(1, 4))    # 1-3
        elif n_flex < 16:
            n_mods = int(rng.integers(2, 6))    # 2-5
        else:
            n_mods = int(rng.integers(3, 9))    # 3-8

        for _ in range(n_mods):
            if side_pool and rng.random() < 0.5:
                # Side-swap: remove 1 copy of a flex card, add 1 side card
                remove_card = str(rng.choice(flex_pool))
                if deck_counts.get(remove_card, 0) > 0:
                    add_card = str(rng.choice(side_pool))
                    entity = db.get_card(add_card)
                    max_c = entity.max_count_in_deck if entity else 4
                    if deck_counts.get(add_card, 0) < max_c:
                        deck_counts[remove_card] -= 1
                        deck_counts[add_card] = deck_counts.get(add_card, 0) + 1
            else:
                # Count-adjust: reduce one flex card, increase another
                candidates_down = [
                    c for c in flex_pool if deck_counts.get(c, 0) > 1
                ]
                candidates_up_filtered: List[str] = []
                for c in flex_pool:
                    entity = db.get_card(c)
                    max_c = entity.max_count_in_deck if entity else 4
                    if deck_counts.get(c, 0) < max_c:
                        candidates_up_filtered.append(c)

                if candidates_down and candidates_up_filtered:
                    down_card = str(rng.choice(candidates_down))
                    up_choices = [
                        c for c in candidates_up_filtered if c != down_card
                    ]
                    if up_choices:
                        up_card = str(rng.choice(up_choices))
                        deck_counts[down_card] -= 1
                        deck_counts[up_card] += 1

        # Remove zero-count entries
        deck_counts = {k: v for k, v in deck_counts.items() if v > 0}

        # Identify egg cards so we rebalance only main-deck cards
        egg_ids: Set[str] = set()
        for cid in deck_counts:
            entity = db.get_card(cid)
            if entity is not None and entity.card_kind == CardKind.DigiEgg:
                egg_ids.add(cid)

        main_total = sum(v for k, v in deck_counts.items() if k not in egg_ids)

        # Trim excess main-deck cards
        while main_total > 50:
            trimmable = [
                c for c in flex_pool
                if deck_counts.get(c, 0) > 1 and c not in egg_ids
            ]
            if not trimmable:
                break
            trim_card = str(rng.choice(trimmable))
            deck_counts[trim_card] -= 1
            main_total -= 1

        # Grow deficit main-deck cards
        while main_total < 50:
            growable: List[str] = []
            for c in flex_pool:
                if c in egg_ids:
                    continue
                entity = db.get_card(c)
                max_c = entity.max_count_in_deck if entity else 4
                if deck_counts.get(c, 0) < max_c:
                    growable.append(c)
            if not growable:
                break
            grow_card = str(rng.choice(growable))
            deck_counts[grow_card] = deck_counts.get(grow_card, 0) + 1
            main_total += 1

        # Convert to flat list
        flat_deck = expand_deck_dict(deck_counts)

        # Validate deck
        result = validate_deck(flat_deck)
        if not result.is_valid:
            continue

        # Deduplicate against base and existing variants
        sorted_deck = sorted(flat_deck)
        sorted_base = sorted(base_deck)
        if sorted_deck == sorted_base:
            continue

        is_dupe = False
        for existing in variants:
            if sorted(existing) == sorted_deck:
                is_dupe = True
                break
        if is_dupe:
            continue

        variants.append(flat_deck)

    return variants[:count]


# ─── DeckPoolWrapper ────────────────────────────────────────────────


class DeckPoolWrapper(gymnasium.Wrapper):
    """Varies the agent's own deck per-episode during training.

    On ``reset()``, samples a variant from the pool and injects it via
    ``options["deck1"]``.  Supports eager mode (sample from a fixed
    pre-generated set) and hybrid mode (fixed set + on-the-fly
    generation of new variants during training).

    Wrapper chain position::

        DigimonEnv -> OpponentWrapper -> DeckPoolWrapper -> GauntletWrapper -> ActionMasker
    """

    def __init__(
        self,
        env: gymnasium.Env,
        variants: List[List[str]],
        egg_deck: Optional[List[str]] = None,
        generation_mode: str = "eager",
        generate_fn: Optional[Callable[..., List[List[str]]]] = None,
        generate_kwargs: Optional[Dict] = None,
        hybrid_max_dynamic: int = 10,
        seed: Optional[int] = None,
    ) -> None:
        super().__init__(env)
        self._variants = list(variants)
        self._egg_deck = egg_deck
        self._generation_mode = generation_mode
        self._generate_fn = generate_fn
        self._generate_kwargs = generate_kwargs or {}
        self._hybrid_max_dynamic = hybrid_max_dynamic
        self._hybrid_generated = 0
        self._rng = np.random.default_rng(seed)
        self._current_variant_idx: Optional[int] = None

    def reset(self, **kwargs):
        """Reset the environment with a sampled deck variant.

        In eager mode, uniformly samples from the pre-loaded variant
        pool.  In hybrid mode, has a 20% chance of generating a new
        variant on-the-fly (up to ``hybrid_max_dynamic`` times) and
        appending it to the pool.
        """
        options = kwargs.get("options") or {}

        if self._variants:
            idx = int(self._rng.integers(0, len(self._variants)))
            options["deck1"] = self._variants[idx]
            self._current_variant_idx = idx

        # Hybrid mode: occasionally generate a new on-the-fly variant
        if (
            self._generation_mode == "hybrid"
            and self._generate_fn is not None
            and self._hybrid_generated < self._hybrid_max_dynamic
            and self._rng.random() < 0.2
        ):
            new_variants = self._generate_fn(count=1, **self._generate_kwargs)
            if new_variants:
                self._variants.extend(new_variants)
                self._hybrid_generated += 1
                options["deck1"] = new_variants[0]
                self._current_variant_idx = len(self._variants) - 1

        kwargs["options"] = options
        return self.env.reset(**kwargs)

    def step(self, action):
        """Step the wrapped environment (pass-through)."""
        return self.env.step(action)

    @property
    def current_variant_index(self) -> Optional[int]:
        """Index of the variant used in the current episode."""
        return self._current_variant_idx

    @property
    def variant_count(self) -> int:
        """Total number of variants in the pool."""
        return len(self._variants)
