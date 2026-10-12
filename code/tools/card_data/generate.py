"""Deterministic generation of the ``data/cards.json`` compatibility ABI.

Before producer cutover, repositories without a committed revision marker use
the explicit legacy shadow adapter at the bottom of this module. A terminal
rollback to the empty revision returns to that mode while retaining its audit
journal. In every other metadata-present state, generation is canonical-only:
the tool loads the marker-owned canonical records, source manifests, reviewed
corrections, stable registry, and the narrow compatibility-projection sidecar
through one consistent-revision read. ``cards.json`` is then only the output
being checked; it is never bootstrapped as a generation input.

Structural Dual projection
--------------------------

The current engine ABI uses ``card_kind = 4`` and independent ``dual``
Digimon/Option objects. The top-level cost is the Option use cost; level, DP,
main effect and evolution routes describe the Digimon. Option text must never
be placed in the inherited-effect slot. Both faces expose Arts Digivolve as
required by the existing engine structural-Dual contract.
"""

from __future__ import annotations

import argparse
import copy
import json
import sys
from dataclasses import dataclass
from pathlib import Path
from types import MappingProxyType
from typing import Any, Iterable, Mapping

from card_data_reader import load_json_artifacts

from .corrections import (
    apply_corrections,
    correction_is_already_applied,
    validate_correction_set,
)
from .errors import CardDataError, ValidationError
from .models import (
    SCHEMA_VERSION,
    AdmissionStatus,
    AuthorityClass,
    CanonicalCard,
    CardFace,
    FaceKind,
    FieldProvenance,
    Printing,
    ReviewedCorrection,
    ScopeManifest,
    ScopeObservationRef,
    SourceObservation,
    SourceName,
)
from .registry import (
    StableCardRegistry,
    compatibility_values,
    is_gameplay_card_id,
)
from .serialization import (
    canonicalize,
    content_digest,
    natural_card_id_key,
    sha256_hex,
)
from .source_manifest import build_full_pool_source_manifest
from .transaction import (
    committed_revision_protocol_active,
    read_consistent_revision,
)


COLOR_TO_LEGACY = {
    "red": 0,
    "blue": 1,
    "yellow": 2,
    "green": 3,
    "white": 4,
    "black": 5,
    "purple": 6,
}
LEGACY_TO_COLOR = {value: key.title() for key, value in COLOR_TO_LEGACY.items()}
KIND_TO_LEGACY = {
    "digimon": 0,
    "tamer": 1,
    "option": 2,
    "digi-egg": 3,
    "digi_egg": 3,
    "dual": 4,
}
LEGACY_TO_KIND = {0: "Digimon", 1: "Tamer", 2: "Option", 3: "Digi-Egg", 4: "Dual"}
RARITY_TO_LEGACY = {"C": 0, "U": 1, "R": 2, "SR": 3, "SEC": 4, "P": 5}

LEGACY_BOOTSTRAP_FIELDS = (
    "card_name_jpn",
    "rarity",
    "xros_req",
    "dna_costs",
    "digixros_costs",
)

DNA_COST_FIELD_ORDER = ("requirement1", "requirement2", "memory_cost")
DNA_REQUIREMENT_FIELD_ORDER = (
    "level",
    "card_colors",
    "name_contains",
    "text_contains",
)
DIGIXROS_COST_FIELD_ORDER = (
    "elements",
    "reduce_cost_per_card",
    "max_materials",
    "different_card_numbers",
    "different_names",
    "different_levels",
    "different_colors",
    "has_text",
    "source_zones",
)
DIGIXROS_ELEMENT_FIELD_ORDER = (
    "name_contains",
    "trait_match",
    "trait_alternatives",
    "level_max",
    "count",
    "is_digimon_only",
    "color",
    "level_exact",
    "any_of",
    "name_any",
    "text_any",
    "keyword",
)

COMPATIBILITY_PROJECTION_PATH = "data/card_compatibility_projection.json"
COMPATIBILITY_PROJECTION_SCHEMA_VERSION = 1
CANONICAL_CARDS_PATH = "data/cards_canonical.json"
CARDS_PATH = "data/cards.json"
CORRECTIONS_PATH = "data/card_corrections.json"
REGISTRY_PATH = "data/card_registry.json"
SOURCE_MANIFEST_PATH = "data/card_sources/manifest.json"
SOURCE_SCOPE_PATHS = {
    SourceName.OFFICIAL: "data/card_sources/scopes/official/full-pool.json",
    SourceName.DIGIMONCARD_IO: (
        "data/card_sources/scopes/digimoncard_io/full-pool.json"
    ),
}


@dataclass(frozen=True, slots=True)
class CompatibilityProjection:
    """Marker-owned inputs intentionally absent from the canonical schema.

    Real-card rows contain only the legacy compatibility fields listed in
    :data:`LEGACY_BOOTSTRAP_FIELDS`.  Full rows are permitted only for
    non-gameplay fixtures which intentionally sit outside the canonical real
    card universe while retaining stable registry assignments.
    """

    canonical_revision: str
    source_manifest_revision: str
    corrections_revision: str
    registry_revision: str
    input_revision: str
    projection_revision: str
    projected_cards_sha256: str
    compatibility_fields: Mapping[str, Mapping[str, Any]]
    fixture_rows: Mapping[str, Mapping[str, Any]]

    def __post_init__(self) -> None:
        object.__setattr__(
            self,
            "compatibility_fields",
            MappingProxyType(
                {
                    card_id: MappingProxyType(copy.deepcopy(dict(row)))
                    for card_id, row in self.compatibility_fields.items()
                }
            ),
        )
        object.__setattr__(
            self,
            "fixture_rows",
            MappingProxyType(
                {
                    card_id: MappingProxyType(copy.deepcopy(dict(row)))
                    for card_id, row in self.fixture_rows.items()
                }
            ),
        )

    def to_dict(self) -> dict[str, Any]:
        return {
            "schema_version": COMPATIBILITY_PROJECTION_SCHEMA_VERSION,
            "generator": "tools.card_data.generate",
            "canonical_revision": self.canonical_revision,
            "source_manifest_revision": self.source_manifest_revision,
            "corrections_revision": self.corrections_revision,
            "registry_revision": self.registry_revision,
            "input_revision": self.input_revision,
            "projection_revision": self.projection_revision,
            "projected_cards_sha256": self.projected_cards_sha256,
            "compatibility_fields": {
                card_id: dict(row)
                for card_id, row in self.compatibility_fields.items()
            },
            "fixture_rows": {
                card_id: dict(row) for card_id, row in self.fixture_rows.items()
            },
        }


@dataclass(frozen=True, slots=True)
class CommittedGenerationStack:
    """One validated marker-owned input stack for compatibility generation.

    Candidate assembly also consumes this type so every committed input it
    carries forward comes from the same marker-before/after read.  The
    committed ``cards.json`` bytes remain available only as the output being
    compared; generation uses ``canonical_cards`` and
    ``compatibility_projection``.
    """

    revision: str
    artifact_bytes: Mapping[str, bytes]
    committed_cards_bytes: bytes
    generated_cards_bytes: bytes
    canonical_cards: Mapping[str, CanonicalCard]
    corrections: tuple[ReviewedCorrection, ...]
    registry: StableCardRegistry
    source_manifest_revision: str
    compatibility_projection: CompatibilityProjection

    def __post_init__(self) -> None:
        object.__setattr__(
            self,
            "artifact_bytes",
            MappingProxyType(dict(self.artifact_bytes)),
        )
        object.__setattr__(
            self,
            "canonical_cards",
            MappingProxyType(dict(self.canonical_cards)),
        )


def _ordered_compatibility_mapping(
    value: Any,
    field_order: tuple[str, ...],
    *,
    card_id: str,
    field_path: str,
) -> dict[str, Any]:
    if not isinstance(value, Mapping):
        raise ValidationError(
            "legacy compatibility structure must be an object",
            card_id=card_id,
            field_path=field_path,
        )
    unknown = set(value) - set(field_order)
    if unknown:
        raise ValidationError(
            "legacy compatibility structure contains unknown fields",
            card_id=card_id,
            field_path=field_path,
            details={"fields": sorted(unknown)},
        )
    return {
        field_name: copy.deepcopy(value[field_name])
        for field_name in field_order
        if field_name in value
    }


def _ordered_compatibility_list(
    value: Any,
    *,
    card_id: str,
    field_path: str,
) -> list[Any]:
    if not isinstance(value, (list, tuple)):
        raise ValidationError(
            "legacy compatibility structure must be an array",
            card_id=card_id,
            field_path=field_path,
        )
    return list(value)


def _normalize_dna_costs(value: Any, *, card_id: str) -> list[dict[str, Any]]:
    normalized: list[dict[str, Any]] = []
    for index, raw_cost in enumerate(
        _ordered_compatibility_list(
            value, card_id=card_id, field_path="dna_costs"
        )
    ):
        cost = _ordered_compatibility_mapping(
            raw_cost,
            DNA_COST_FIELD_ORDER,
            card_id=card_id,
            field_path=f"dna_costs[{index}]",
        )
        for requirement in ("requirement1", "requirement2"):
            if requirement in cost:
                cost[requirement] = _ordered_compatibility_mapping(
                    cost[requirement],
                    DNA_REQUIREMENT_FIELD_ORDER,
                    card_id=card_id,
                    field_path=f"dna_costs[{index}].{requirement}",
                )
        normalized.append(cost)
    return normalized


def _normalize_digixros_costs(
    value: Any, *, card_id: str
) -> list[dict[str, Any]]:
    normalized: list[dict[str, Any]] = []
    for cost_index, raw_cost in enumerate(
        _ordered_compatibility_list(
            value, card_id=card_id, field_path="digixros_costs"
        )
    ):
        cost = _ordered_compatibility_mapping(
            raw_cost,
            DIGIXROS_COST_FIELD_ORDER,
            card_id=card_id,
            field_path=f"digixros_costs[{cost_index}]",
        )
        if "elements" in cost:
            elements = _ordered_compatibility_list(
                cost["elements"],
                card_id=card_id,
                field_path=f"digixros_costs[{cost_index}].elements",
            )
            cost["elements"] = [
                _ordered_compatibility_mapping(
                    element,
                    DIGIXROS_ELEMENT_FIELD_ORDER,
                    card_id=card_id,
                    field_path=(
                        f"digixros_costs[{cost_index}].elements[{element_index}]"
                    ),
                )
                for element_index, element in enumerate(elements)
            ]
        normalized.append(cost)
    return normalized


def _normalize_compatibility_field(
    field_name: str, value: Any, *, card_id: str
) -> Any:
    if field_name == "dna_costs":
        return _normalize_dna_costs(value, card_id=card_id)
    if field_name == "digixros_costs":
        return _normalize_digixros_costs(value, card_id=card_id)
    return copy.deepcopy(value)


def build_compatibility_projection(
    canonical_cards: Mapping[str, CanonicalCard],
    registry: StableCardRegistry,
    *,
    corrections: Iterable[ReviewedCorrection],
    source_manifest_revision: str,
    compatibility_fields: Mapping[str, Mapping[str, Any]],
    fixture_rows: Mapping[str, Mapping[str, Any]],
    projected_cards: bytes,
) -> CompatibilityProjection:
    """Build the exact, versioned compatibility-only cutover sidecar."""

    ordered_corrections = validate_correction_set(corrections)
    eligible_ids = {
        card_id
        for card_id, card in canonical_cards.items()
        if card.admission in {AdmissionStatus.ADMITTED, AdmissionStatus.RETIRED}
    }
    registered_ids = set(registry.assignments)
    fixture_ids = registered_ids - eligible_ids
    hidden_gameplay_fixtures = sorted(
        card_id for card_id in fixture_ids if is_gameplay_card_id(card_id)
    )
    if hidden_gameplay_fixtures:
        raise ValidationError(
            "gameplay cards cannot use compatibility fixture rows",
            details={"card_ids": hidden_gameplay_fixtures},
        )
    if set(fixture_rows) != fixture_ids:
        raise ValidationError(
            "compatibility fixture rows do not match the non-canonical registry universe",
            details={
                "missing": sorted(fixture_ids - set(fixture_rows)),
                "unknown": sorted(set(fixture_rows) - fixture_ids),
            },
        )
    if set(compatibility_fields) != registered_ids:
        raise ValidationError(
            "compatibility projection fields do not match the stable registry universe",
            details={
                "missing": sorted(registered_ids - set(compatibility_fields)),
                "unknown": sorted(set(compatibility_fields) - registered_ids),
            },
        )

    allowed_fields = {*LEGACY_BOOTSTRAP_FIELDS, "legacy_card_index"}
    normalized_fields: dict[str, dict[str, Any]] = {}
    for card_id in sorted(compatibility_fields, key=natural_card_id_key):
        row = compatibility_fields[card_id]
        if not isinstance(row, Mapping):
            raise ValidationError(
                "compatibility projection card entry must be an object",
                card_id=card_id,
            )
        unknown = set(row) - allowed_fields
        if unknown:
            raise ValidationError(
                "compatibility projection contains canonical or unknown fields",
                card_id=card_id,
                details={"fields": sorted(unknown)},
            )
        if is_gameplay_card_id(card_id) and "legacy_card_index" in row:
            raise ValidationError(
                "real card_index cannot be stored in the compatibility projection",
                card_id=card_id,
            )
        normalized_fields[card_id] = {
            field_name: _normalize_compatibility_field(
                field_name, field_value, card_id=card_id
            )
            for field_name, field_value in row.items()
        }

    normalized_fixtures: dict[str, dict[str, Any]] = {}
    for card_id in sorted(fixture_rows, key=natural_card_id_key):
        row = fixture_rows[card_id]
        if not isinstance(row, Mapping) or row.get("card_id") != card_id:
            raise ValidationError(
                "compatibility fixture row does not match its registry ID",
                card_id=card_id,
            )
        # Parsing the fixture through the same temporary adapter validates the
        # full legacy value shape without making cards.json an input later.
        bootstrap_legacy_canonical_cards({card_id: row})
        normalized_fixtures[card_id] = copy.deepcopy(dict(row))

    canonical_revision = content_digest(canonical_cards)
    corrections_revision = content_digest(ordered_corrections)
    registry_revision = content_digest(registry)
    projection_revision = content_digest(
        {
            "compatibility_fields": normalized_fields,
            "fixture_rows": normalized_fixtures,
        }
    )
    input_revision = content_digest(
        {
            "canonical_revision": canonical_revision,
            "source_manifest_revision": source_manifest_revision,
            "corrections_revision": corrections_revision,
            "registry_revision": registry_revision,
            "projection_revision": projection_revision,
        }
    )
    return CompatibilityProjection(
        canonical_revision=canonical_revision,
        source_manifest_revision=source_manifest_revision,
        corrections_revision=corrections_revision,
        registry_revision=registry_revision,
        input_revision=input_revision,
        projection_revision=projection_revision,
        projected_cards_sha256=sha256_hex(projected_cards),
        compatibility_fields=normalized_fields,
        fixture_rows=normalized_fixtures,
    )


def _legacy_color(value: Any) -> int:
    if isinstance(value, int) and not isinstance(value, bool) and value in LEGACY_TO_COLOR:
        return value
    if isinstance(value, str) and value.strip().lower() in COLOR_TO_LEGACY:
        return COLOR_TO_LEGACY[value.strip().lower()]
    raise ValidationError(
        "canonical color cannot be projected into cards.json",
        details={"color": value},
    )


def _legacy_kind(value: Any) -> int:
    if isinstance(value, int) and not isinstance(value, bool) and value in LEGACY_TO_KIND:
        return value
    key = str(value).strip().lower().replace(" ", "_")
    if key in KIND_TO_LEGACY:
        return KIND_TO_LEGACY[key]
    raise ValidationError(
        "canonical card kind cannot be projected into cards.json",
        details={"card_kind": value},
    )


def _legacy_rarity(value: Any) -> int:
    if isinstance(value, int) and not isinstance(value, bool) and 0 <= value <= 5:
        return value
    if isinstance(value, str) and value.upper() in RARITY_TO_LEGACY:
        return RARITY_TO_LEGACY[value.upper()]
    # The legacy API mapped unrecognized display rarities (for example UR) to C.
    if value in (None, "") or isinstance(value, str):
        return 0
    raise ValidationError(
        "canonical rarity cannot be projected into cards.json",
        details={"rarity": value},
    )


def _ordered_color_union(*groups: Iterable[Any]) -> list[int]:
    colors: list[int] = []
    for group in groups:
        for value in group:
            color = _legacy_color(value)
            if color not in colors:
                colors.append(color)
    if not colors:
        raise ValidationError("canonical card requires at least one compatibility color")
    return colors


def _legacy_evo_costs(value: Any) -> list[dict[str, int]]:
    if value in (None, ()):
        return []
    if not isinstance(value, (list, tuple)):
        raise ValidationError("canonical evo_costs must be an array")
    output: list[dict[str, int]] = []
    for route in value:
        if not isinstance(route, Mapping):
            raise ValidationError("canonical evolution route must be an object")
        if {"card_color", "level", "memory_cost"} <= set(route):
            output.append(
                {
                    "card_color": _legacy_color(route["card_color"]),
                    "level": int(route["level"]),
                    "memory_cost": int(route["memory_cost"]),
                }
            )
            continue
        colors = route.get("colors", ())
        if isinstance(colors, (str, int)):
            colors = (colors,)
        level = route.get("from_level", route.get("level"))
        cost = route.get("cost", route.get("memory_cost"))
        if not colors or level is None or cost is None:
            raise ValidationError(
                "canonical evolution route lacks colors, from level, or cost"
            )
        for color in colors:
            output.append(
                {
                    "card_color": _legacy_color(color),
                    "level": int(level),
                    "memory_cost": int(cost),
                }
            )
    return output


def _legacy_text(value: Any) -> str:
    """Preserve the shipped CRLF convention inside compatibility text values."""

    text = str(value or "")
    # Bootstrap rows already contain the exact shipped compatibility spelling,
    # including one historical mixed-newline value.  Preserve those bytes while
    # converting normalized canonical LF text for newly sourced records.
    if "\r\n" in text:
        return text
    return text.replace("\r", "\n").replace("\n", "\r\n")


def _dual_faces(card: CanonicalCard) -> tuple[CardFace, CardFace] | None:
    if len(card.faces) != 2:
        return None
    digimon = next((face for face in card.faces if face.kind is FaceKind.DIGIMON), None)
    option = next((face for face in card.faces if face.kind is FaceKind.OPTION), None)
    if digimon is None or option is None:
        raise ValidationError(
            "two-face compatibility projection requires Digimon and Option faces",
            card_id=card.card_id,
            field_path="faces",
        )
    return digimon, option


def project_canonical_card(
    card: CanonicalCard,
    *,
    legacy_bootstrap: Mapping[str, Any] | None = None,
) -> dict[str, Any]:
    """Project one corrected canonical record before stable index injection."""

    fields = card.fields
    legacy_bootstrap = legacy_bootstrap or {}
    allowed_bootstrap = {*LEGACY_BOOTSTRAP_FIELDS, "legacy_card_index"}
    unknown_bootstrap = set(legacy_bootstrap) - allowed_bootstrap
    if unknown_bootstrap:
        raise ValidationError(
            "legacy bootstrap sidecar contains unsupported fields",
            card_id=card.card_id,
            details={"fields": sorted(unknown_bootstrap)},
        )
    if is_gameplay_card_id(card.card_id) and "legacy_card_index" in legacy_bootstrap:
        raise ValidationError(
            "real card_index must be derived from the numeric suffix",
            card_id=card.card_id,
            field_path="card_index",
        )
    dual = _dual_faces(card)
    if dual is None:
        colors = _ordered_color_union(fields.get("card_colors", ()))
        play_cost = fields.get("play_cost", 0)
        level = fields.get("level")
        dp = fields.get("dp")
        kind = _legacy_kind(fields.get("card_kind", card.faces[0].kind.value))
        effect_text = _legacy_text(fields.get("effect_description_eng", ""))
        inherited_text = _legacy_text(fields.get("inherited_effect_description_eng", ""))
        security_text = _legacy_text(fields.get("security_effect_description_eng", ""))
    else:
        digimon, option = dual
        colors = _ordered_color_union(digimon.colors, option.colors)
        play_cost = option.play_cost
        level = digimon.level
        dp = digimon.dp
        kind = KIND_TO_LEGACY["dual"]
        # Top-level canonical text and structural face text have independent
        # provenance. In particular, the pre-cutover shadow adapter retains
        # existing top-level compatibility strings while preserving both faces.
        effect_text = _legacy_text(fields.get("effect_description_eng", digimon.effect_text))
        inherited_text = _legacy_text(fields.get("inherited_effect_description_eng", digimon.inherited_text))
        security_text = _legacy_text(fields.get("security_effect_description_eng", digimon.security_text))

    if play_cost is None:
        play_cost = 0
    row: dict[str, Any] = {
        "card_id": card.card_id,
        "card_name_eng": str(fields.get("card_name_eng", "")),
        "card_name_jpn": str(legacy_bootstrap.get("card_name_jpn", "")),
        "card_effect_class_name": card.card_id.replace("-", "_"),
        "play_cost": int(play_cost),
        "dp": dp,
        "level": level,
        "card_kind": kind,
        "rarity": _legacy_rarity(legacy_bootstrap.get("rarity")),
        "card_colors": colors,
        "type_eng": list(fields.get("type_eng", ())),
        "form_eng": list(fields.get("form_eng", ())),
        "attribute_eng": list(fields.get("attribute_eng", ())),
        "effect_description_eng": effect_text,
        "inherited_effect_description_eng": inherited_text,
        "security_effect_description_eng": security_text,
        "evo_costs": _legacy_evo_costs(fields.get("evo_costs", ())),
        "xros_req": str(legacy_bootstrap.get("xros_req", "")),
    }
    for optional in ("dna_costs", "digixros_costs"):
        if optional in legacy_bootstrap:
            row[optional] = _normalize_compatibility_field(
                optional, legacy_bootstrap[optional], card_id=card.card_id
            )
    for field_name in ("link_effect_description_eng", "max_count_in_deck"):
        if field_name in fields:
            row[field_name] = copy.deepcopy(fields[field_name])
    if dual is not None:
        digimon, option = dual
        row["dual"] = {
            "digimon": {
                "level": digimon.level,
                "dp": digimon.dp,
                "colors": list(digimon.colors),
                "traits": list(fields.get("type_eng", ())),
                "evo_costs": _legacy_evo_costs(fields.get("evo_costs", ())),
                "effect_text": _legacy_text(digimon.effect_text),
                "inherited_text": _legacy_text(digimon.inherited_text),
                "keywords": ["ArtsDigivolve"],
            },
            "option": {
                "use_cost": option.play_cost,
                "colors": list(option.colors),
                "effect_text": _legacy_text(option.effect_text),
                "security_text": _legacy_text(option.security_text),
                "keywords": ["ArtsDigivolve"],
            },
        }
    if "legacy_card_index" in legacy_bootstrap:
        row["legacy_card_index"] = legacy_bootstrap["legacy_card_index"]
    return row


def generate_cards_json(
    canonical_cards: Mapping[str, CanonicalCard],
    registry: StableCardRegistry,
    *,
    corrections: Iterable[ReviewedCorrection] = (),
    observations: Mapping[str, SourceObservation] | None = None,
    legacy_bootstrap: Mapping[str, Mapping[str, Any]] | None = None,
    card_order: Iterable[str] | None = None,
) -> dict[str, dict[str, Any]]:
    """Generate compatibility rows without writing the producer destination."""

    corrections = tuple(corrections)
    legacy_bootstrap = legacy_bootstrap or {}
    if corrections:
        if observations is None:
            raise ValidationError("reviewed correction projection requires observations")
        canonical_cards = apply_corrections(
            canonical_cards, corrections, observations=observations
        )

    eligible = {
        card_id: card
        for card_id, card in canonical_cards.items()
        if card.admission in {AdmissionStatus.ADMITTED, AdmissionStatus.RETIRED}
    }
    provisional_ids = {
        card_id
        for card_id, card in canonical_cards.items()
        if card.admission is AdmissionStatus.PROVISIONAL
    }
    registered_ids = set(registry.assignments)
    if registered_ids & provisional_ids:
        raise ValidationError(
            "provisional canonical cards cannot have stable global assignments",
            details={"card_ids": sorted(registered_ids & provisional_ids)},
        )
    if set(eligible) != registered_ids:
        raise ValidationError(
            "canonical compatibility universe does not match stable registry",
            details={
                "missing_canonical": sorted(registered_ids - set(eligible)),
                "missing_registry": sorted(set(eligible) - registered_ids),
            },
        )
    unknown_bootstrap_ids = set(legacy_bootstrap) - set(eligible)
    if unknown_bootstrap_ids:
        raise ValidationError(
            "legacy bootstrap sidecar references unknown canonical cards",
            details={"card_ids": sorted(unknown_bootstrap_ids)},
        )

    # A pre-cutover shadow may preserve the input file's append history.
    # Normal canonical generation remains naturally ordered. Neither order
    # affects the append-only registry assignments used by engine tensors.
    ordered_ids = sorted(eligible, key=natural_card_id_key) if card_order is None else list(card_order)
    if len(ordered_ids) != len(eligible) or set(ordered_ids) != set(eligible):
        raise ValidationError("card_order must contain each eligible card exactly once")
    output: dict[str, dict[str, Any]] = {}
    for card_id in ordered_ids:
        card = eligible[card_id]
        if card.card_id != card_id:
            raise ValidationError(
                "canonical mapping key does not match card ID", card_id=card.card_id
            )
        projected = project_canonical_card(
            card, legacy_bootstrap=legacy_bootstrap.get(card_id)
        )
        index = registry.assignments[card_id]
        derived = compatibility_values(card_id, index, capacity=registry.capacity)
        card_index = derived["card_index"]
        if not is_gameplay_card_id(card_id) and "legacy_card_index" in projected:
            card_index = projected.pop("legacy_card_index")

        # Field insertion order is part of the current byte-level shadow contract.
        row = {
            "index": derived["index"],
            "norm_id": derived["norm_id"],
            "card_id": projected.pop("card_id"),
            "card_index": card_index,
            **projected,
        }
        output[card_id] = row
    return output


def bootstrap_legacy_canonical_cards(
    legacy_cards: Mapping[str, Mapping[str, Any]],
) -> dict[str, CanonicalCard]:
    """Create explicit temporary canonical records from the frozen legacy ABI.

    This adapter exists only to prove shadow equivalence before a committed
    canonical snapshot covers the whole pool.  It does not infer structural
    Dual faces from flattened rows; explicit current-ABI Dual objects retain
    their independently recorded face data.
    """

    result: dict[str, CanonicalCard] = {}
    for card_id in sorted(legacy_cards, key=natural_card_id_key):
        row = legacy_cards[card_id]
        if row.get("card_id") != card_id:
            raise ValidationError(
                "legacy bootstrap key does not match card_id", card_id=card_id
            )
        try:
            kind_name = LEGACY_TO_KIND[row["card_kind"]]
            colors = tuple(LEGACY_TO_COLOR[value] for value in row["card_colors"])
        except (KeyError, TypeError) as exc:
            raise ValidationError(
                "legacy bootstrap row has unsupported kind or color",
                card_id=card_id,
            ) from exc

        fields: dict[str, Any] = {
            "card_name_eng": row["card_name_eng"],
            "card_kind": kind_name,
            "card_colors": list(colors),
            "play_cost": row["play_cost"],
            "dp": row["dp"],
            "level": row["level"],
            "type_eng": copy.deepcopy(row["type_eng"]),
            "form_eng": copy.deepcopy(row["form_eng"]),
            "attribute_eng": copy.deepcopy(row["attribute_eng"]),
            "effect_description_eng": row["effect_description_eng"],
            "inherited_effect_description_eng": row[
                "inherited_effect_description_eng"
            ],
            "security_effect_description_eng": row[
                "security_effect_description_eng"
            ],
            "evo_costs": copy.deepcopy(row["evo_costs"]),
        }
        for field_name in ("link_effect_description_eng", "max_count_in_deck"):
            if field_name in row:
                fields[field_name] = copy.deepcopy(row[field_name])
        observation_id = content_digest(
            {"source": "legacy-cards-json-bootstrap", "card_id": card_id, "row": row}
        )
        provenance = {
            field_name: FieldProvenance(
                selected_observation_id=observation_id,
                observation_ids=(observation_id,),
                authority=AuthorityClass.DERIVED,
                derived=True,
            )
            for field_name in fields
        }
        face_kind = FaceKind.DIGIMON if kind_name == "Dual" else FaceKind(kind_name.lower().replace("-", "_"))
        face = CardFace(
            face_id="legacy-bootstrap",
            kind=face_kind,
            colors=colors,
            play_cost=row["play_cost"],
            level=row["level"],
            dp=row["dp"],
            effect_text=row["effect_description_eng"],
            inherited_text=row["inherited_effect_description_eng"],
            security_text=row["security_effect_description_eng"],
        )
        faces = (face,)
        if kind_name == "Dual":
            payload = row.get("dual")
            if not isinstance(payload, Mapping) or set(payload) != {"digimon", "option"}:
                raise ValidationError("Dual bootstrap requires both explicit faces", card_id=card_id)
            d, o = payload["digimon"], payload["option"]
            faces = (
                CardFace(face_id="digimon", kind=FaceKind.DIGIMON,
                         colors=tuple(d["colors"]), level=d["level"], dp=d["dp"],
                         effect_text=d["effect_text"], inherited_text=d["inherited_text"]),
                CardFace(face_id="option", kind=FaceKind.OPTION,
                         colors=tuple(o["colors"]), play_cost=o["use_cost"],
                         effect_text=o["effect_text"], security_text=o["security_text"]),
            )
        result[card_id] = CanonicalCard(
            schema_version=SCHEMA_VERSION,
            card_id=card_id,
            admission=AdmissionStatus.ADMITTED,
            fields=fields,
            provenance=provenance,
            printings=(
                Printing(
                    printing_id=f"legacy-bootstrap:{card_id}",
                    observation_ids=(observation_id,),
                ),
            ),
            faces=faces,
        )
    return result


def bootstrap_legacy_compatibility_fields(
    legacy_cards: Mapping[str, Mapping[str, Any]],
) -> dict[str, dict[str, Any]]:
    """Extract values absent from the canonical schema into a shadow sidecar."""

    result: dict[str, dict[str, Any]] = {}
    for card_id in sorted(legacy_cards, key=natural_card_id_key):
        row = legacy_cards[card_id]
        compatibility = {
            field_name: copy.deepcopy(row[field_name])
            for field_name in LEGACY_BOOTSTRAP_FIELDS
            if field_name in row
        }
        if not is_gameplay_card_id(card_id):
            compatibility["legacy_card_index"] = row["card_index"]
        result[card_id] = compatibility
    return result


def render_legacy_cards_json(cards: Mapping[str, Mapping[str, Any]]) -> bytes:
    """Render the exact current CRLF/no-final-newline compatibility bytes."""

    text = json.dumps(cards, ensure_ascii=False, indent=2)
    return text.replace("\n", "\r\n").encode("utf-8")


def shadow_legacy_snapshot(
    legacy_cards: Mapping[str, Mapping[str, Any]],
    registry: StableCardRegistry,
) -> tuple[dict[str, dict[str, Any]], bytes]:
    canonical = bootstrap_legacy_canonical_cards(legacy_cards)
    compatibility = bootstrap_legacy_compatibility_fields(legacy_cards)
    generated = generate_cards_json(
        canonical, registry, legacy_bootstrap=compatibility, card_order=legacy_cards
    )
    return generated, render_legacy_cards_json(generated)


def _json_object(raw: bytes, *, path: str) -> Mapping[str, Any]:
    try:
        payload = json.loads(raw.decode("utf-8"))
    except (UnicodeDecodeError, json.JSONDecodeError) as exc:
        raise ValidationError(
            "committed card-data artifact is not valid JSON",
            path=path,
            details={"error": str(exc)},
        ) from exc
    if not isinstance(payload, Mapping):
        raise ValidationError(
            "committed card-data artifact root must be an object",
            path=path,
        )
    return payload


def _scope_manifest_from_payload(
    payload: Mapping[str, Any], *, path: str
) -> ScopeManifest:
    try:
        refs = payload["observation_refs"]
        if not isinstance(refs, list):
            raise TypeError("observation_refs must be an array")
        return ScopeManifest(
            schema_version=payload["schema_version"],
            source=SourceName(payload["source"]),
            scope=payload["scope"],
            source_locations=tuple(payload["source_locations"]),
            first_acquired_at=payload["first_acquired_at"],
            parser_version=payload["parser_version"],
            source_schema_version=payload["source_schema_version"],
            observation_ids=tuple(payload["observation_ids"]),
            observation_refs=tuple(
                ScopeObservationRef(
                    observation_id=row["observation_id"],
                    gameplay_card_id=row["gameplay_card_id"],
                    printing_id=row["printing_id"],
                )
                for row in refs
            ),
            complete=payload["complete"],
            expected_count=payload["expected_count"],
            failed_ids=tuple(payload.get("failed_ids", ())),
        )
    except (KeyError, TypeError, ValueError) as exc:
        if isinstance(exc, CardDataError):
            raise
        raise ValidationError(
            "committed source scope manifest is invalid",
            path=path,
            details={"error": str(exc)},
        ) from exc


def _corrections_from_payload(
    payload: Mapping[str, Any], *, path: str
) -> tuple[ReviewedCorrection, ...]:
    if set(payload) != {"schema_version", "corrections"}:
        raise ValidationError(
            "committed correction document has missing or unknown fields",
            path=path,
        )
    if payload.get("schema_version") != SCHEMA_VERSION:
        raise ValidationError(
            "committed correction document has an unsupported schema",
            path=path,
        )
    raw_entries = payload.get("corrections")
    if not isinstance(raw_entries, list):
        raise ValidationError(
            "committed correction document requires a corrections array",
            path=path,
        )
    required = {
        "schema_version",
        "correction_id",
        "card_id",
        "field_path",
        "value",
        "evidence_observation_ids",
        "reason",
        "reviewer",
        "reviewed_at",
    }
    corrections: list[ReviewedCorrection] = []
    for index, row in enumerate(raw_entries):
        if not isinstance(row, Mapping) or set(row) != required:
            raise ValidationError(
                "committed correction entry has missing or unknown fields",
                path=path,
                details={"index": index},
            )
        try:
            evidence = row["evidence_observation_ids"]
            if not isinstance(evidence, list):
                raise TypeError("evidence_observation_ids must be an array")
            corrections.append(
                ReviewedCorrection(
                    schema_version=row["schema_version"],
                    correction_id=row["correction_id"],
                    card_id=row["card_id"],
                    field_path=row["field_path"],
                    value=row["value"],
                    evidence_observation_ids=tuple(evidence),
                    reason=row["reason"],
                    reviewer=row["reviewer"],
                    reviewed_at=row["reviewed_at"],
                )
            )
        except (KeyError, TypeError, ValueError) as exc:
            if isinstance(exc, CardDataError):
                raise
            raise ValidationError(
                "committed correction entry is invalid",
                path=path,
                details={"index": index, "error": str(exc)},
            ) from exc
    return validate_correction_set(corrections)


def _observation_from_payload(
    payload: Mapping[str, Any], *, path: str
) -> SourceObservation:
    try:
        observation = SourceObservation(
            schema_version=payload["schema_version"],
            observation_id=payload["observation_id"],
            source=SourceName(payload["source"]),
            source_record_key=payload["source_record_key"],
            source_url=payload["source_url"],
            raw_payload_digest=payload["raw_payload_digest"],
            parser_version=payload["parser_version"],
            source_schema_version=payload["source_schema_version"],
            first_acquired_at=payload["first_acquired_at"],
            extracted=payload["extracted"],
        )
    except (KeyError, TypeError, ValueError) as exc:
        if isinstance(exc, CardDataError):
            raise
        raise ValidationError(
            "committed source observation is invalid",
            path=path,
            details={"error": str(exc)},
        ) from exc
    if content_digest(observation.identity_payload()) != observation.observation_id:
        raise ValidationError(
            "committed source observation identity is stale",
            path=path,
            details={"observation_id": observation.observation_id},
        )
    return observation


def _required_raw(
    values: Mapping[str, Any], relative_path: str
) -> bytes:
    value = values.get(relative_path)
    if not isinstance(value, bytes):
        raise ValidationError(
            "committed revision does not declare a required generation input",
            path=relative_path,
        )
    return value


def _load_correction_observations(
    values: Mapping[str, Any],
    corrections: Iterable[ReviewedCorrection],
) -> dict[str, SourceObservation]:
    observations: dict[str, SourceObservation] = {}
    for observation_id in sorted(
        {
            observation_id
            for correction in corrections
            for observation_id in correction.evidence_observation_ids
        }
    ):
        matches = [
            relative
            for relative in values
            if relative.startswith("data/card_sources/observations/")
            and relative.endswith(f"/{observation_id}.json")
        ]
        if len(matches) != 1:
            raise ValidationError(
                "reviewed correction evidence is not uniquely marker-owned",
                details={
                    "observation_id": observation_id,
                    "paths": sorted(matches),
                },
            )
        relative = matches[0]
        payload = _json_object(_required_raw(values, relative), path=relative)
        observation = _observation_from_payload(payload, path=relative)
        if observation.observation_id != observation_id:
            raise ValidationError(
                "source observation filename does not match its identity",
                path=relative,
            )
        observations[observation_id] = observation
    return observations


def load_committed_generation_stack(
    repo_root: Path,
    *,
    marker_path: Path,
    journal_path: Path,
) -> CommittedGenerationStack:
    """Load and validate one strict marker-owned generation stack.

    The returned raw artifacts are the exact bytes from the same consistent
    revision.  Callers that need other marker-owned inputs, such as candidate
    assembly, must parse those bytes instead of rereading fixed paths.
    """

    committed = read_consistent_revision(
        repo_root,
        marker_path=marker_path,
        journal_path=journal_path,
    )
    values = committed.values
    cards_raw = _required_raw(values, CARDS_PATH)
    canonical_raw = _required_raw(values, CANONICAL_CARDS_PATH)
    corrections_raw = _required_raw(values, CORRECTIONS_PATH)
    registry_raw = _required_raw(values, REGISTRY_PATH)
    projection_raw = _required_raw(values, COMPATIBILITY_PROJECTION_PATH)
    aggregate_raw = _required_raw(values, SOURCE_MANIFEST_PATH)

    canonical_payload = _json_object(canonical_raw, path=CANONICAL_CARDS_PATH)
    # Kept local to avoid making complete-candidate assembly depend on a
    # second canonical deserializer while this module remains its projector.
    from .complete_candidate import load_prior_canonical_cards

    canonical_cards = load_prior_canonical_cards(
        repo_root / CANONICAL_CARDS_PATH,
        payload=canonical_payload,
    )
    from .compatibility_corrections import validate_correction_artifacts
    document = validate_correction_artifacts(values)
    corrections = document.corrections
    registry = StableCardRegistry.from_dict(
        _json_object(registry_raw, path=REGISTRY_PATH)
    )

    scopes = tuple(
        _scope_manifest_from_payload(
            _json_object(
                _required_raw(values, relative), path=relative
            ),
            path=relative,
        )
        for relative in SOURCE_SCOPE_PATHS.values()
    )
    aggregate = build_full_pool_source_manifest(scopes)
    aggregate_payload = _json_object(aggregate_raw, path=SOURCE_MANIFEST_PATH)
    if canonicalize(aggregate_payload) != canonicalize(aggregate.to_dict()):
        raise ValidationError(
            "committed aggregate source manifest is stale",
            path=SOURCE_MANIFEST_PATH,
        )
    if canonical_payload.get("source_manifest_digest") != aggregate.manifest_revision:
        raise ValidationError(
            "canonical cards are bound to another source-manifest revision",
            path=CANONICAL_CARDS_PATH,
        )
    expected_source_bindings = {
        binding.source.value: binding.scope_manifest_digest
        for binding in aggregate.sources
    }
    if canonical_payload.get("source_manifests") != expected_source_bindings:
        raise ValidationError(
            "canonical cards have stale source-scope bindings",
            path=CANONICAL_CARDS_PATH,
        )

    observations = _load_correction_observations(values, corrections)
    for correction in corrections:
        card = canonical_cards.get(correction.card_id)
        if card is None or not correction_is_already_applied(
            card, correction, observations=observations
        ):
            raise ValidationError(
                "committed canonical card does not preserve its reviewed correction",
                card_id=correction.card_id,
                field_path=correction.field_path,
                details={"correction_id": correction.correction_id},
            )

    projection_payload = _json_object(
        projection_raw, path=COMPATIBILITY_PROJECTION_PATH
    )
    compatibility_fields = projection_payload.get("compatibility_fields")
    fixture_rows = projection_payload.get("fixture_rows")
    if not isinstance(compatibility_fields, Mapping) or not isinstance(
        fixture_rows, Mapping
    ):
        raise ValidationError(
            "compatibility projection requires field and fixture mappings",
            path=COMPATIBILITY_PROJECTION_PATH,
        )
    if document.compatibility_corrections:
        from .complete_candidate import _observations_from_committed_artifacts
        from .compatibility_conditions import refresh_official_conditions
        from .compatibility_selections import SELECTION_LEDGER_PATH, selections_from_bytes
        all_observations = _observations_from_committed_artifacts(values)
        selections = selections_from_bytes(_required_raw(values, SELECTION_LEDGER_PATH))
        checked_fields, _ = refresh_official_conditions(
            {cid: card for cid, card in canonical_cards.items() if card.admission is AdmissionStatus.ADMITTED},
            all_observations, compatibility_fields, selections=selections,
            source_manifest_revision=aggregate.manifest_revision,
            compatibility_corrections=document.compatibility_corrections,
            correction_image_artifacts=values,
        )
        for row in document.compatibility_corrections:
            if row.source_manifest_revision == aggregate.manifest_revision and (
                compatibility_fields.get(row.card_id, {}).get('xros_req') != checked_fields[row.card_id]['xros_req']
            ):
                raise ValidationError('committed compatibility projection lost its reviewed correction',
                                      card_id=row.card_id, field_path=row.field_path)
    expected_projection = build_compatibility_projection(
        canonical_cards,
        registry,
        corrections=corrections,
        source_manifest_revision=aggregate.manifest_revision,
        compatibility_fields=compatibility_fields,
        fixture_rows=fixture_rows,
        projected_cards=cards_raw,
    )
    if canonicalize(projection_payload) != canonicalize(
        expected_projection.to_dict()
    ):
        raise ValidationError(
            "committed compatibility projection is stale or malformed",
            path=COMPATIBILITY_PROJECTION_PATH,
        )

    generation_cards = dict(canonical_cards)
    generation_cards.update(bootstrap_legacy_canonical_cards(fixture_rows))
    generated = generate_cards_json(
        generation_cards,
        registry,
        legacy_bootstrap=compatibility_fields,
    )
    generated = dict(
        sorted(generated.items(), key=lambda item: natural_card_id_key(item[0]))
    )
    generated_cards_bytes = render_legacy_cards_json(generated)
    return CommittedGenerationStack(
        revision=committed.revision,
        artifact_bytes={
            relative: raw
            for relative, raw in values.items()
            if isinstance(raw, bytes)
        },
        committed_cards_bytes=cards_raw,
        generated_cards_bytes=generated_cards_bytes,
        canonical_cards=canonical_cards,
        corrections=tuple(corrections),
        registry=registry,
        source_manifest_revision=aggregate.manifest_revision,
        compatibility_projection=expected_projection,
    )


def regenerate_committed_cards(
    repo_root: Path,
    *,
    marker_path: Path,
    journal_path: Path,
) -> tuple[bytes, bytes, int, str]:
    """Regenerate cards.json from one strict marker-owned canonical stack."""

    stack = load_committed_generation_stack(
        repo_root,
        marker_path=marker_path,
        journal_path=journal_path,
    )
    return (
        stack.generated_cards_bytes,
        stack.committed_cards_bytes,
        len(stack.registry.assignments),
        stack.revision,
    )


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repo-root", type=Path, default=Path.cwd())
    parser.add_argument(
        "--legacy-snapshot",
        type=Path,
        default=None,
        help="explicit pre-cutover shadow input; unavailable after marker ownership",
    )
    parser.add_argument("--registry", type=Path, default=None)
    parser.add_argument("--output", type=Path, default=None)
    parser.add_argument(
        "--check",
        action="store_true",
        help="network-free freshness check; never writes output",
    )
    args = parser.parse_args(argv)

    repo_root = args.repo_root.resolve()
    cards_path = repo_root / CARDS_PATH
    registry_path = repo_root / REGISTRY_PATH
    marker_path = repo_root / "data/card_data_revision.json"
    journal_path = repo_root / "data/.card_data_transaction.json"

    def resolved(value: Path | None, default: Path) -> Path:
        if value is None:
            return default
        return (value if value.is_absolute() else repo_root / value).resolve()

    legacy_snapshot = resolved(args.legacy_snapshot, cards_path)
    requested_registry = resolved(args.registry, registry_path)
    output_path = resolved(args.output, repo_root / "cards.shadow.json") if args.output else None

    if args.check and args.output is not None:
        print("error: --check is write-free and cannot be combined with --output", file=sys.stderr)
        return 2
    if output_path is not None and output_path == cards_path.resolve():
        print("error: generation cannot replace the transaction-owned cards.json", file=sys.stderr)
        return 2

    try:
        revision_protocol_active = committed_revision_protocol_active(
            marker_path=marker_path,
            journal_path=journal_path,
        )
    except (OSError, json.JSONDecodeError, CardDataError, ValueError) as exc:
        print(f"error: {exc}", file=sys.stderr)
        return 1
    if revision_protocol_active:
        if legacy_snapshot != cards_path.resolve():
            print(
                "error: --legacy-snapshot is unavailable after canonical producer cutover",
                file=sys.stderr,
            )
            return 2
        if requested_registry != registry_path.resolve():
            print(
                "error: --registry cannot bypass the committed canonical revision",
                file=sys.stderr,
            )
            return 2
        try:
            rendered, committed_cards, card_count, revision = regenerate_committed_cards(
                repo_root,
                marker_path=marker_path,
                journal_path=journal_path,
            )
        except (OSError, json.JSONDecodeError, CardDataError, ValueError) as exc:
            print(f"error: {exc}", file=sys.stderr)
            return 1
        if rendered != committed_cards:
            print(
                "error: regenerated canonical cards.json differs from the committed compatibility artifact",
                file=sys.stderr,
            )
            return 1
        if output_path is not None:
            output_path.parent.mkdir(parents=True, exist_ok=True)
            output_path.write_bytes(rendered)
        print(
            f"canonical cards.json is fresh ({card_count} cards, revision {revision})"
        )
        return 0

    try:
        loaded = load_json_artifacts(
            (legacy_snapshot, requested_registry),
            repository_root=repo_root,
            marker_path=marker_path,
            journal_path=journal_path,
        )
        legacy = loaded[legacy_snapshot].value
        registry = StableCardRegistry.from_dict(loaded[requested_registry].value)
        _generated, rendered = shadow_legacy_snapshot(legacy, registry)
    except (OSError, json.JSONDecodeError, CardDataError, ValueError) as exc:
        print(f"error: {exc}", file=sys.stderr)
        return 1

    legacy_bytes = loaded[legacy_snapshot].raw_bytes
    if rendered != legacy_bytes:
        print("error: shadow cards.json differs from the legacy snapshot", file=sys.stderr)
        return 1
    if output_path is not None:
        output_path.parent.mkdir(parents=True, exist_ok=True)
        output_path.write_bytes(rendered)
    print(f"shadow cards.json is byte-equivalent ({len(legacy)} cards)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
