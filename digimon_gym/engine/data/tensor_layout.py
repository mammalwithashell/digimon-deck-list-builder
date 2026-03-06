"""Tensor layout metadata: which positions hold card IDs vs scalars.

Used by the FeaturesExtractor to split the observation tensor into
card-index positions (for nn.Embedding lookup) and scalar positions.
All values are derived from the constants in game.py.
"""

from digimon_gym.engine.game import (
    FIELD_SLOTS, MAX_HAND, MAX_TRASH, MAX_SECURITY, MAX_SOURCES,
    MAX_REVEALED, SLOT_SIZE, SOURCE_ENTRY_SIZE, TENSOR_SIZE,
    _GLOBAL, _MY_BATTLE, _OPP_BATTLE, _MY_HAND, _OPP_HAND,
    _MY_TRASH, _OPP_TRASH, _MY_SECURITY, _OPP_SECURITY,
    _MY_BREEDING, _OPP_BREEDING, _REVEALED,
)


def _compute_positions():
    """Compute card ID and scalar position lists for the tensor layout."""
    card_positions = []
    scalar_positions = []

    # Global section: all scalars
    for i in range(_GLOBAL):
        scalar_positions.append(i)

    def _slot_positions(slot_base):
        """Enumerate card_id and scalar positions within one SLOT_SIZE block."""
        cids = []
        scals = []
        # +0: top card ID
        cids.append(slot_base)
        # +1..+6: scalars (DP, suspended, opt_total, opt_used, linked_count, source_count)
        for j in range(1, 7):
            scals.append(slot_base + j)
        # Sources: MAX_SOURCES × (card_id, opt_state, dp_contribution)
        src_base = slot_base + 7
        for s in range(MAX_SOURCES):
            off = src_base + s * SOURCE_ENTRY_SIZE
            cids.append(off)          # card_id
            scals.append(off + 1)     # opt_state
            scals.append(off + 2)     # dp_contribution
        return cids, scals

    off = _GLOBAL

    # My battle area (FIELD_SLOTS slots)
    for i in range(FIELD_SLOTS):
        cids, scals = _slot_positions(off + i * SLOT_SIZE)
        card_positions.extend(cids)
        scalar_positions.extend(scals)

    off += _MY_BATTLE

    # Opp battle area (FIELD_SLOTS slots)
    for i in range(FIELD_SLOTS):
        cids, scals = _slot_positions(off + i * SLOT_SIZE)
        card_positions.extend(cids)
        scalar_positions.extend(scals)

    off += _OPP_BATTLE

    # My hand (MAX_HAND card IDs)
    for i in range(MAX_HAND):
        card_positions.append(off + i)
    off += _MY_HAND

    # Opp hand (MAX_HAND card IDs)
    for i in range(MAX_HAND):
        card_positions.append(off + i)
    off += _OPP_HAND

    # My trash (MAX_TRASH card IDs)
    for i in range(MAX_TRASH):
        card_positions.append(off + i)
    off += _MY_TRASH

    # Opp trash (MAX_TRASH card IDs)
    for i in range(MAX_TRASH):
        card_positions.append(off + i)
    off += _OPP_TRASH

    # My security (MAX_SECURITY card IDs)
    for i in range(MAX_SECURITY):
        card_positions.append(off + i)
    off += _MY_SECURITY

    # Opp security (MAX_SECURITY card IDs)
    for i in range(MAX_SECURITY):
        card_positions.append(off + i)
    off += _OPP_SECURITY

    # My breeding (1 slot)
    cids, scals = _slot_positions(off)
    card_positions.extend(cids)
    scalar_positions.extend(scals)
    off += _MY_BREEDING

    # Opp breeding (1 slot)
    cids, scals = _slot_positions(off)
    card_positions.extend(cids)
    scalar_positions.extend(scals)
    off += _OPP_BREEDING

    # Revealed cards (MAX_REVEALED card IDs)
    for i in range(MAX_REVEALED):
        card_positions.append(off + i)
    off += _REVEALED

    # Selection context (5 scalars)
    for i in range(5):
        scalar_positions.append(off + i)

    return sorted(card_positions), sorted(scalar_positions)


CARD_ID_POSITIONS, SCALAR_POSITIONS = _compute_positions()
NUM_CARD_SLOTS = len(CARD_ID_POSITIONS)
NUM_SCALAR_SLOTS = len(SCALAR_POSITIONS)

# Sanity checks
assert NUM_CARD_SLOTS + NUM_SCALAR_SLOTS == TENSOR_SIZE, (
    f"Position counts ({NUM_CARD_SLOTS} + {NUM_SCALAR_SLOTS}) "
    f"don't sum to TENSOR_SIZE ({TENSOR_SIZE})"
)
assert len(set(CARD_ID_POSITIONS) | set(SCALAR_POSITIONS)) == TENSOR_SIZE
