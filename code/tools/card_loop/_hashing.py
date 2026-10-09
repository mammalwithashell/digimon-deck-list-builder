"""Seeded, platform-stable hashing into [0, 1).

Python's built-in `hash()` is salted per process (PYTHONHASHSEED), so it cannot
drive anything that must reproduce across runs. Every seeded draw in the card
loop — the router's exploration share, the audit sample — goes through
`stable_unit`, which hashes a namespaced tuple with SHA-256.
"""
from __future__ import annotations

import hashlib

_SEP = "\x1f"  # unit separator: cannot appear in ids, so ("a", "bc") != ("ab", "c")


def stable_unit(namespace: str, *parts: object) -> float:
    """Map `(namespace, *parts)` to a float uniformly distributed in [0, 1).

    The namespace keeps independent draws independent: the router and the
    audit sampler hash the same attempt or item ids, and must not select the
    same subset because of it.
    """
    key = _SEP.join([namespace, *(str(p) for p in parts)]).encode("utf-8")
    digest = hashlib.sha256(key).digest()
    return int.from_bytes(digest[:8], "big") / float(1 << 64)
