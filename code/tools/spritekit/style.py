"""The Digimon Up style envelope, derived by measuring the reference library.

Numbers come from ``analyze.py`` over the 338 cached ``UI_<role>_<Name>.png``
sprites (283 of which join to a printed card level in ``data/cards.json``).
Re-run ``python code/tools/spritekit/analyze.py`` after refreshing the index if
the library ever changes.
"""
from __future__ import annotations

# level -> (height p10, height median, height p90, width median)
# Measured over the reference sprites' tight bounding boxes.
STAGE_ENVELOPE: dict[int, tuple[int, int, int, int]] = {
    2: (24, 33, 42, 40),   # extrapolated: only In-Training refs, few card levels
    3: (38, 51, 65, 52),
    4: (51, 57, 74, 64),
    5: (63, 82, 105, 79),
    6: (76, 90, 109, 87),
    7: (110, 119, 128, 142),
}

# Measured aggregates across the whole reference set.
ALPHA_IS_BINARY = True          # median 1 partial-alpha px/sprite; 152/338 exactly 0
OUTLINE_MAX_LUMA = 60.0         # "dark" threshold used for the keyline
DARKEST_MAX_LUMA = 26.0         # refs bottom out at luma 0 (median), max 10.8
FILL_RATIO_RANGE = (0.35, 0.85)  # opaque / bbox area; refs median 0.59
PALETTE_RANGE = (8, 32)         # authored sprites use a tight indexed ramp


def canvas_for(level: int | None) -> tuple[int, int]:
    """Suggested ``(width, height)`` canvas for a Digimon of this printed level."""
    lo, med, hi, wmed = STAGE_ENVELOPE.get(level or 4, STAGE_ENVELOPE[4])
    return wmed, med


def height_bounds(level: int | None, slack: float = 0.18) -> tuple[int, int]:
    lo, med, hi, _ = STAGE_ENVELOPE.get(level or 4, STAGE_ENVELOPE[4])
    return int(lo * (1 - slack)), int(hi * (1 + slack))
