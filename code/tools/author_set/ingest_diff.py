"""Pull a release set from digimoncard.io, diff it against the local snapshot,
and merge the differences before authoring (Phase 1, tasks 1.2 / 1.3).

The newest set is the one most likely to be stale or partial in
``data/cards.json`` (observed: EX12 = 3 local vs 43 live), so authoring must
begin with a pull -> diff -> merge rather than trusting the snapshot.

Structure:
- ``diff_sets``      — PURE: compare a local cards dict vs pulled rows. No I/O.
- ``merge_diff``     — PURE: apply added/changed onto a copy of the local dict.
- ``pull_and_diff``  — network wrapper around ``ingest_cards.fetch_set_by_card_prefix``
                       with a loud offline fallback to the local snapshot.

``removed`` cards are reported but NOT deleted from the local snapshot — a card
absent from the live prefix query is far more likely an API hiccup than a real
de-print, and silently dropping a card we may have already authored is worse
than a stale extra entry. Deletion stays a human decision.
"""

from __future__ import annotations

import sys
from dataclasses import dataclass, field

from .set_resolver import normalize_prefix

# Fields whose drift is meaningful for authoring. Volatile/identity fields
# (date_added, tcgplayer_id, rarity, artist, pretty_url) are excluded so a
# re-pull does not report spurious "changed" noise.
SIGNIFICANT_FIELDS = (
    "card_name_eng",
    "card_kind",
    "level",
    "dp",
    "play_cost",
    "card_colors",
    "type_eng",
    "evo_costs",
    "effect_description_eng",
    "inherited_effect_description_eng",
    "security_effect_description_eng",
)


@dataclass
class SetDiff:
    set_prefix: str
    added: list[str] = field(default_factory=list)        # ids in pulled, not local
    removed: list[str] = field(default_factory=list)      # ids in local, not pulled
    changed: list[dict] = field(default_factory=list)     # [{id, field, local, pulled}]
    unchanged: int = 0
    offline: bool = False                                  # True if pull failed -> snapshot used

    @property
    def is_empty(self) -> bool:
        return not self.added and not self.removed and not self.changed

    def summary(self) -> str:
        if self.offline:
            return (f"[{self.set_prefix}] OFFLINE — used local snapshot unverified")
        return (
            f"[{self.set_prefix}] +{len(self.added)} added, "
            f"-{len(self.removed)} removed, ~{len(self.changed)} changed, "
            f"{self.unchanged} unchanged"
        )


def _set_ids(prefix: str, ids) -> set[str]:
    import re

    pat = re.compile(rf"^{re.escape(prefix)}-\d")
    return {str(i) for i in ids if pat.match(str(i))}


def diff_sets(local_cards: dict, pulled_cards: list[dict], set_prefix: str) -> SetDiff:
    """Compare the local snapshot against freshly pulled (converted) rows.

    Args:
        local_cards: cards.json dict, keyed by card_id (full DB or just the set).
        pulled_cards: list of cards in cards.json schema (already converted by
            ``ingest_cards.convert_card``), one per distinct card_id.
        set_prefix: e.g. ``"BT17"`` — both sides are restricted to this set.
    """
    pre = normalize_prefix(set_prefix)
    pulled = {str(c["card_id"]): c for c in pulled_cards}
    local_ids = _set_ids(pre, local_cards.keys())
    pulled_ids = _set_ids(pre, pulled.keys())

    diff = SetDiff(set_prefix=pre)
    diff.added = sorted(pulled_ids - local_ids)
    diff.removed = sorted(local_ids - pulled_ids)

    for cid in sorted(pulled_ids & local_ids):
        loc, pul = local_cards[cid], pulled[cid]
        card_changed = False
        for f in SIGNIFICANT_FIELDS:
            lv, pv = loc.get(f), pul.get(f)
            if lv != pv:
                diff.changed.append({"id": cid, "field": f, "local": lv, "pulled": pv})
                card_changed = True
        if not card_changed:
            diff.unchanged += 1
    return diff


def merge_diff(local_cards: dict, pulled_cards: list[dict], diff: SetDiff) -> tuple[dict, int]:
    """Return a copy of ``local_cards`` with added + changed cards merged in.

    Removed cards are left in place by design (see module docstring). Returns
    ``(merged_dict, n_cards_written)``.
    """
    pulled = {str(c["card_id"]): c for c in pulled_cards}
    merged = dict(local_cards)
    written = 0
    changed_ids = {c["id"] for c in diff.changed}
    for cid in list(diff.added) + sorted(changed_ids):
        if cid in pulled:
            merged[cid] = pulled[cid]
            written += 1
    return merged, written


def pull_and_diff(set_prefix: str, local_cards: dict) -> tuple[SetDiff, list[dict]]:
    """Pull the set from digimoncard.io and diff it against ``local_cards``.

    On any network failure, emits a loud warning and returns an ``offline=True``
    diff with no pulled rows so the caller can proceed against the local
    snapshot (settled sets match the source exactly; only the newest drifts).

    Returns ``(diff, pulled_cards)``. ``pulled_cards`` is ``[]`` when offline.
    """
    pre = normalize_prefix(set_prefix)
    try:
        from tools.ingest_cards import fetch_set_by_card_prefix

        pulled = fetch_set_by_card_prefix(pre)
    except Exception as exc:  # network / parse / DNS — fall back loudly
        print(
            f"\n*** WARNING: could not reach digimoncard.io for {pre} ({exc!r}).\n"
            f"*** Proceeding against the LOCAL snapshot UNVERIFIED. Settled sets\n"
            f"*** match the source exactly, but a newly-released set may be stale.\n",
            file=sys.stderr,
        )
        return SetDiff(set_prefix=pre, offline=True), []
    return diff_sets(local_cards, pulled, pre), pulled
