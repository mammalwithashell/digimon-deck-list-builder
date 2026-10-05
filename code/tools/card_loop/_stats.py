"""Small interval helpers for the scorecard and router (no numpy / scipy)."""
from __future__ import annotations

import math

Z95 = 1.959963984540054


def wilson(successes: int, n: int, z: float = Z95) -> tuple[float, float]:
    """Wilson score interval for a binomial proportion.

    `n == 0` returns the uninformative (0.0, 1.0) rather than raising: an empty
    cell is a normal state for a new stage or prompt version.
    """
    if n <= 0:
        return (0.0, 1.0)
    if not 0 <= successes <= n:
        raise ValueError(f"successes={successes} outside [0, {n}]")
    p = successes / n
    z2 = z * z
    denom = 1.0 + z2 / n
    centre = (p + z2 / (2 * n)) / denom
    half = (z / denom) * math.sqrt(p * (1 - p) / n + z2 / (4 * n * n))
    # the exact bound at the extremes is 0 / 1; don't let rounding leave 0.9999999999999999
    lo = 0.0 if successes == 0 else max(0.0, centre - half)
    hi = 1.0 if successes == n else min(1.0, centre + half)
    return (lo, hi)


def mean_interval(values: list[float], z: float = Z95) -> tuple[float, float, float]:
    """(lo, mean, hi) normal-approximation interval on a mean, floored at 0.

    Fewer than two values cannot estimate spread, so the interval is
    unbounded above: such a cell can never "separate" from another.
    """
    n = len(values)
    if n == 0:
        return (0.0, 0.0, math.inf)
    mean = sum(values) / n
    if n < 2:
        return (0.0, mean, math.inf)
    var = sum((v - mean) ** 2 for v in values) / (n - 1)
    half = z * math.sqrt(var / n)
    return (max(0.0, mean - half), mean, mean + half)


def cost_per_good_unit(costs: list[float], good: int, z: float = Z95
                       ) -> tuple[float, float, float]:
    """(lo, point, hi) for total cost / good units.

    Cost per good unit = mean cost per attempt / good-unit rate. The interval
    combines the mean-cost interval with the Wilson interval on the rate at
    their conservative extremes (lo = mean_lo / p_hi, hi = mean_hi / p_lo), so
    two cells only "separate" when they differ under both sources of noise.
    An undefined bound (no good units, a single attempt) is `math.inf`.
    """
    n = len(costs)
    point = (sum(costs) / good) if good > 0 else math.inf
    m_lo, _, m_hi = mean_interval(costs, z)
    p_lo, p_hi = wilson(good, n, z)
    lo = m_lo / p_hi if p_hi > 0 else math.inf
    hi = m_hi / p_lo if p_lo > 0 else math.inf
    return (lo, point, hi)


def intervals_separate(a: tuple[float, float], b: tuple[float, float]) -> int:
    """-1 if `a` lies entirely below `b`, 1 if entirely above, 0 if they overlap."""
    if a[1] < b[0]:
        return -1
    if b[1] < a[0]:
        return 1
    return 0
