"""The `Clause` record — the atomic (card, clause) unit this package tracks."""

from __future__ import annotations

from dataclasses import asdict, dataclass

# The only three zones a clause may be tagged with (task spec). Zone reflects
# WHICH SOURCE FIELD the clause's text came from (effect_description_eng /
# inherited_effect_description_eng / security_effect_description_eng, or the
# bundle's equivalent sections) -- not what timing markers appear within its
# text. A clause whose text happens to contain a "[Security]" timing marker
# but was sourced from the main effect field (e.g. EX12-069) is still zone
# "effect"; see the README.
VALID_ZONES: tuple[str, ...] = ("effect", "inherited", "security")

# Where a clause's text came from, in source-priority order (README).
VALID_SOURCES: tuple[str, ...] = (
    "bundle",
    "cards_json",
    "card_overrides",
    "image-cache",
    "image-required",
)


@dataclass
class Clause:
    """One independently-testable (card, clause) unit.

    `id` is stable across runs for the same underlying data (deterministic
    function of card_id + zone + processing order), so coverage results can
    be diffed run over run: e.g. "EX12-073#security#0".
    """

    id: str
    card_id: str
    zone: str
    label: str
    kind: str  # "timing" | "keyword" | "untimed"
    timings: list[str]
    keyword: str | None
    text: str
    source: str
    image_path: str | None = None

    def to_dict(self) -> dict:
        return asdict(self)


def clause_from_dict(data: dict) -> Clause:
    """Inverse of `Clause.to_dict` — used when reading back an `extract` output."""
    return Clause(
        id=data["id"],
        card_id=data["card_id"],
        zone=data["zone"],
        label=data.get("label", ""),
        kind=data.get("kind", "untimed"),
        timings=list(data.get("timings") or []),
        keyword=data.get("keyword"),
        text=data.get("text", ""),
        source=data["source"],
        image_path=data.get("image_path"),
    )
