"""Card feature vectorizer for autoencoder embedding generation.

Converts structured card attributes from cards.json into fixed-size float vectors
suitable for training a card autoencoder. Each card is represented as ~106 floats
encoding its kind, colors, stats, form, attribute, keywords, and evolution costs.
"""

import json
import re
from pathlib import Path
from typing import Dict, List, Optional

import numpy as np

# ─── Canonical orderings (append-only to preserve trained embeddings) ─────

CARD_KINDS = ['Digimon', 'Tamer', 'Option', 'DigiEgg']  # 4

CARD_COLORS = ['Red', 'Blue', 'Yellow', 'Green', 'White', 'Black', 'Purple', 'NoColor']  # 8

FORMS = [
    'Rookie', 'Champion', 'Ultimate', 'Mega', 'In-Training',
    'Armor Form', 'Hybrid', 'D-Reaper', 'God', 'Unknown',
    'Appmon', 'Stnd.', 'Sup.', 'Ult.', 'Eater', 'Virus',
]  # 16

ATTRIBUTES = [
    'Vaccine', 'Virus', 'Data', 'Free', 'Variable', 'Unknown',
    'God', 'System', 'Tool', 'Navi', 'Social', 'Game',
    'Entertainment', 'Minor', 'Life', 'Ultimate', 'NO DATA', 'LIBERATOR',
]  # 18

BASE_KEYWORDS = [
    'Alliance', 'Armor Purge', 'Barrier', 'Blast DNA Digivolve',
    'Blast Digivolve', 'Blitz', 'Blocker', 'Collision',
    'De-Digivolve', 'Decode', 'Decoy', 'Delay',
    'Digi-Burst', 'Digisorption', 'Draw', 'Evade',
    'Execute', 'Fortitude', 'Fragment', 'Iceclad',
    'Jamming', 'Link', 'Material Save', 'Mind Link',
    'Overclock', 'Partition', 'Piercing', 'Progress',
    'Raid', 'Reboot', 'Recovery', 'Retaliation',
    'Rush', 'Save', 'Scapegoat', 'Security A.',
    'Training', 'Use Req.', 'Vortex',
]  # 39

# Normalization constants (fixed, not learned)
MAX_LEVEL = 7
MAX_PLAY_COST = 20
MAX_DP = 17000
MAX_EVO_COST = 10  # memory cost for evolution

# Feature vector dimensions
DIM_KIND = len(CARD_KINDS)           # 4
DIM_COLORS = len(CARD_COLORS)        # 8
DIM_LEVEL = 1                        # 1
DIM_PLAY_COST = 1                    # 1
DIM_DP = 1                           # 1
DIM_FORMS = len(FORMS)               # 16
DIM_ATTRIBUTES = len(ATTRIBUTES)     # 18
DIM_KEYWORDS = len(BASE_KEYWORDS)    # 39
DIM_FLAGS = 3                        # has_inherited, has_security, is_ace
DIM_EVO_SLOT = len(CARD_COLORS) + 2  # color_8 + level + cost = 10
NUM_EVO_SLOTS = 2                    # up to 2 evolution cost slots
DIM_EVO = DIM_EVO_SLOT * NUM_EVO_SLOTS  # 20

FEATURE_DIM = (DIM_KIND + DIM_COLORS + DIM_LEVEL + DIM_PLAY_COST + DIM_DP +
               DIM_FORMS + DIM_ATTRIBUTES + DIM_KEYWORDS + DIM_FLAGS + DIM_EVO)  # 112

_KW_PATTERN = re.compile(r'[＜<]([^＞>]+)[＞>]')


class CardFeatureVectorizer:
    """Converts card entries from cards.json into fixed-size feature vectors."""

    def __init__(self):
        self._form_idx = {f: i for i, f in enumerate(FORMS)}
        self._attr_idx = {a: i for i, a in enumerate(ATTRIBUTES)}
        self._kw_idx = {k: i for i, k in enumerate(BASE_KEYWORDS)}
        self._color_idx = {i: i for i in range(len(CARD_COLORS))}

    def vectorize(self, card: Dict) -> np.ndarray:
        """Convert a single card entry to a feature vector.

        Args:
            card: A card entry dict from cards.json.

        Returns:
            np.ndarray of shape (FEATURE_DIM,) with float32 values in [0, 1].
        """
        vec = np.zeros(FEATURE_DIM, dtype=np.float32)
        offset = 0

        # card_kind: one-hot (4)
        kind = card.get('card_kind', 0)
        if 0 <= kind < DIM_KIND:
            vec[offset + kind] = 1.0
        offset += DIM_KIND

        # card_colors: multi-hot (8)
        for color in card.get('card_colors', []):
            if 0 <= color < DIM_COLORS:
                vec[offset + color] = 1.0
        offset += DIM_COLORS

        # level: normalized (1)
        level = card.get('level')
        if level is not None:
            vec[offset] = min(float(level) / MAX_LEVEL, 1.0)
        offset += DIM_LEVEL

        # play_cost: normalized (1)
        play_cost = card.get('play_cost', 0)
        if play_cost is not None:
            vec[offset] = min(float(play_cost) / MAX_PLAY_COST, 1.0)
        offset += DIM_PLAY_COST

        # dp: normalized (1)
        dp = card.get('dp')
        if dp is not None:
            vec[offset] = min(float(dp) / MAX_DP, 1.0)
        offset += DIM_DP

        # form: one-hot (16)
        for form in card.get('form_eng', []):
            idx = self._form_idx.get(form)
            if idx is not None:
                vec[offset + idx] = 1.0
        offset += DIM_FORMS

        # attribute: one-hot (18)
        for attr in card.get('attribute_eng', []):
            clean_attr = attr.rstrip('}')  # clean "Free}}" → "Free"
            idx = self._attr_idx.get(clean_attr)
            if idx is not None:
                vec[offset + idx] = 1.0
        offset += DIM_ATTRIBUTES

        # keywords: multi-hot (39)
        keywords = self._extract_base_keywords(card)
        for kw in keywords:
            idx = self._kw_idx.get(kw)
            if idx is not None:
                vec[offset + idx] = 1.0
        offset += DIM_KEYWORDS

        # flags (3)
        vec[offset] = 1.0 if card.get('inherited_effect_description_eng', '').strip() else 0.0
        vec[offset + 1] = 1.0 if card.get('security_effect_description_eng', '').strip() else 0.0
        effect_text = card.get('effect_description_eng', '')
        vec[offset + 2] = 1.0 if 'Overflow' in effect_text or '＜Overflow' in effect_text else 0.0
        offset += DIM_FLAGS

        # evo_costs: up to 2 slots of (color_8 + level + cost) = 10 each
        evo_costs = card.get('evo_costs', [])
        for slot_i in range(NUM_EVO_SLOTS):
            slot_offset = offset + slot_i * DIM_EVO_SLOT
            if slot_i < len(evo_costs):
                ec = evo_costs[slot_i]
                color = ec.get('card_color', 0)
                if 0 <= color < DIM_COLORS:
                    vec[slot_offset + color] = 1.0
                vec[slot_offset + DIM_COLORS] = min(float(ec.get('level', 0)) / MAX_LEVEL, 1.0)
                vec[slot_offset + DIM_COLORS + 1] = min(float(ec.get('memory_cost', 0)) / MAX_EVO_COST, 1.0)
        offset += DIM_EVO

        assert offset == FEATURE_DIM, f"Expected offset {FEATURE_DIM}, got {offset}"
        return vec

    def vectorize_all(self, cards_json_path: Optional[str] = None) -> np.ndarray:
        """Vectorize all cards from cards.json.

        Args:
            cards_json_path: Path to cards.json. Defaults to the engine data directory.

        Returns:
            np.ndarray of shape (max_index + 1, FEATURE_DIM). Index 0 is zero-padded.
        """
        if cards_json_path is None:
            cards_json_path = str(Path(__file__).parent / 'cards.json')

        with open(cards_json_path) as f:
            cards = json.load(f)

        max_index = max(card['index'] for card in cards.values())
        features = np.zeros((max_index + 1, FEATURE_DIM), dtype=np.float32)

        for card in cards.values():
            idx = card['index']
            features[idx] = self.vectorize(card)

        return features

    @staticmethod
    def _extract_base_keywords(card: Dict) -> List[str]:
        """Extract base keyword names from card text, collapsing parametric variants."""
        keywords = set()
        for field in ('effect_description_eng', 'inherited_effect_description_eng',
                      'security_effect_description_eng'):
            text = card.get(field, '')
            for match in _KW_PATTERN.findall(text):
                raw = match.strip()
                # Collapse parametric variants to base keyword
                for base_kw in BASE_KEYWORDS:
                    if raw == base_kw or raw.startswith(base_kw + ' ') or raw.startswith(base_kw + '('):
                        keywords.add(base_kw)
                        break
        return list(keywords)
