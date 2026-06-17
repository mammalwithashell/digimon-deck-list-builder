from fastapi import APIRouter
from digimon_engine import list_formats as engine_list_formats
from pydantic import BaseModel

router = APIRouter(prefix="/formats", tags=["formats"])


class FormatDto(BaseModel):
    id: str
    name: str
    tagline: str
    description: str
    deck_label: str
    population_pct: int
    enabled: bool
    disabled_reason: str | None = None


ENGINE_STANDARD_ONLY_REASON = "Engine supports Standard only in this build"

PRESENTATION = {
    "standard": {"tagline": "The official ruleset", "population_pct": 84},
    "no_restriction": {"tagline": "Every card. Every printing.", "population_pct": 23},
    "pauper": {"tagline": "Commons and uncommons only", "population_pct": 18},
    "eden": {"tagline": "Rotation-light, anomaly-gated", "population_pct": 27},
    "eden_singleton": {"tagline": "EDEN, highlander", "population_pct": 12},
}


def _deck_label(format_descriptor) -> str:
    if format_descriptor.singleton:
        return f"{format_descriptor.deck_size} singleton"
    return f"{format_descriptor.deck_size} cards"


@router.get("", response_model=list[FormatDto])
def list_formats() -> list[FormatDto]:
    return [
        FormatDto(
            id=f.id,
            name=f.name.upper(),
            tagline=PRESENTATION.get(f.id, {}).get("tagline", ""),
            description=f.description,
            deck_label=_deck_label(f),
            population_pct=PRESENTATION.get(f.id, {}).get("population_pct", 0),
            enabled=f.playable,
            disabled_reason=None if f.playable else ENGINE_STANDARD_ONLY_REASON,
        )
        for f in engine_list_formats()
    ]
