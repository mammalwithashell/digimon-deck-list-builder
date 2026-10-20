"""Official-first canonical normalization with structural card identities."""

from __future__ import annotations

import json
import re
from dataclasses import dataclass, replace
from typing import Any, Iterable, Mapping

from .errors import AdjudicationError, SourceSchemaError, UnsupportedLayoutError, ValidationError
from .models import (
    SCHEMA_VERSION,
    AdjudicationKind,
    AdmissionStatus,
    AuthorityClass,
    CanonicalCard,
    CardFace,
    FaceKind,
    FieldConflict,
    FieldConflictKind,
    FieldProvenance,
    MigrationAdjudication,
    Printing,
    SourceName,
    SourceObservation,
)
from .serialization import canonicalize, content_digest, natural_card_id_key


_KNOWN_FIELD_KEYS = frozenset(
    {
        "attribute_eng",
        "card_colors",
        "card_kind",
        "card_name_eng",
        "dp",
        "effect_description_eng",
        "evo_costs",
        "form_eng",
        "inherited_effect_description_eng",
        "level",
        "link_effect_description_eng",
        "max_count_in_deck",
        "play_cost",
        "security_effect_description_eng",
        "type_eng",
    }
)

_GENERIC_TEXT_SECTION_LABELS = frozenset(
    {
        "effect",
        "inherited effect",
        "inheritable effect",
        "security effect",
        "dual effect",
        "dual rule",
    }
)
_STRUCTURAL_TEXT_SECTION_LABELS = (
    "special digivolution condition",
    "special evolution condition",
    "special play condition",
    "dna digivolution",
    "digixros",
    "app fusion",
)


@dataclass(frozen=True, slots=True)
class ObservationCandidate:
    observation: SourceObservation
    gameplay_card_id: str
    printing_id: str
    language: str
    product: str | None
    alternate_art: bool
    fields: Mapping[str, Any]
    faces: tuple[CardFace, ...]
    selection_priority: tuple[int, int, int, str]

    @property
    def authority(self) -> AuthorityClass:
        return (
            AuthorityClass.OFFICIAL
            if self.observation.source is SourceName.OFFICIAL
            else AuthorityClass.SECONDARY
        )


@dataclass(frozen=True, slots=True)
class CanonicalConflictBinding:
    """Exact review boundary for one deterministic printing-field choice."""

    card_id: str
    field_path: str
    old_input_revision: str
    new_input_revision: str
    selected_value: Any
    selected_value_digest: str
    selected_observation_id: str
    candidate_observation_ids: tuple[str, ...]
    candidate_values: tuple[Mapping[str, Any], ...]
    candidate_digest: str


def _split_printed_values(value: Any) -> list[str]:
    """Project slash-delimited official detail rows into legacy list values."""

    raw_values = value if isinstance(value, (list, tuple)) else (value,)
    output: list[str] = []
    for raw in raw_values:
        for item in str(raw or "").split("/"):
            item = item.strip()
            if item and item not in output:
                output.append(item)
    return output


_BRACED_TIMING_MARKERS = {
    "breeding": "Breeding",
    "hand": "Hand",
    "security": "Security",
    "trash": "Trash",
}
_BRACED_TIMING_PATTERN = re.compile(
    r"\{(" + "|".join(_BRACED_TIMING_MARKERS) + r")\}",
    re.IGNORECASE,
)


def _normalize_printed_effect_text(text: str) -> str:
    """Project official icon placeholders into the printed timing-marker ABI."""

    # Official empty effect boxes use a standalone dash (including Japanese
    # prolonged-sound and fullwidth forms). Dashes within rules text remain.
    if text.strip() in {"ー", "－", "—", "-"}:
        return ""

    return _BRACED_TIMING_PATTERN.sub(
        lambda match: f"[{_BRACED_TIMING_MARKERS[match.group(1).lower()]}]",
        text,
    )


_SECURITY_EFFECT_PREFIX = re.compile(r"^Security Effect\s+(?=\[Security\])", re.I)


def _misplaced_security_text(text: str) -> str | None:
    """Return security text stored in a non-inheritable card's inherited slot."""

    normalized = _normalize_printed_effect_text(text).strip()
    normalized = _SECURITY_EFFECT_PREFIX.sub("", normalized)
    return normalized if normalized.startswith("[Security]") else None


def _normalize_candidate_fields(raw_fields: Mapping[str, Any]) -> dict[str, Any]:
    """Normalize source transport artifacts before field selection."""

    fields = dict(raw_fields)
    for field_path in (
        "effect_description_eng",
        "inherited_effect_description_eng",
        "link_effect_description_eng",
        "security_effect_description_eng",
    ):
        if isinstance(fields.get(field_path), str):
            fields[field_path] = _normalize_printed_effect_text(fields[field_path])
    card_kind = str(fields.get("card_kind", "")).casefold()
    inherited = fields.get("inherited_effect_description_eng")
    security = fields.get("security_effect_description_eng")
    if (
        card_kind in {"option", "tamer"}
        and isinstance(inherited, str)
        and not security
        and (recovered := _misplaced_security_text(inherited)) is not None
    ):
        fields["inherited_effect_description_eng"] = ""
        fields["security_effect_description_eng"] = recovered
    return fields


def _render_effect_section(label: str, text: str) -> str | None:
    """Render effect-body text without leaking source presentation headers.

    Generic official headings are container labels, not printed effect text.
    Structural condition sections are retained on the source observation and
    projected through their dedicated route/condition fields rather than being
    prepended to the compatibility effect body.
    """

    text = _normalize_printed_effect_text(text)
    normalized_label = " ".join(label.lower().split())
    if any(marker in normalized_label for marker in _STRUCTURAL_TEXT_SECTION_LABELS):
        return None
    if not label or normalized_label in _GENERIC_TEXT_SECTION_LABELS:
        return text
    return f"[{label}] {text}".strip()


def _normalized_section_label(label: str) -> str:
    return " ".join(label.lower().split())


_LINK_HEADER = re.compile(
    r"^＜Link＞ .+?: Cost \d+(?: \(Plug this card from the hand or battle area "
    r"sideways into the specified Digimon in the battle area\.\))?$"
)
_DECK_COPY_RULE = re.compile(
    r"^(?:(?:\[Rule\]|\(Rule\))\s*)?You can include up to ([1-9]\d*) copies "
    r"of cards with this card's card number in your deck\.$"
)


def _link_section_effect(label: str, text: str) -> str | None:
    """Recognize explicit Link boxes and the older combined inherited HTML box.

    The condition remains on the immutable observation. Only the text below
    that condition is a linked effect; mentions of linking are not sufficient.
    """

    lowered = _normalized_section_label(label)
    if lowered == "link effect":
        return _normalize_printed_effect_text(text)
    if lowered not in {"inherited effect", "inheritable effect"}:
        return None
    lines = text.strip().splitlines()
    if not lines or not lines[0].startswith("＜Link＞"):
        return None
    if not _LINK_HEADER.fullmatch(lines[0]):
        raise SourceSchemaError("unsupported combined official Link condition", source="official")
    return _normalize_printed_effect_text("\n".join(lines[1:]).strip())


def _printed_deck_copy_limit(sections: Iterable[Mapping[str, Any]]) -> int | None:
    limits = {
        int(match.group(1))
        for section in sections
        if _normalized_section_label(str(section.get("label", ""))) in {"effect", "rule", "special rule"}
        for line in str(section.get("text", "")).splitlines()
        if (match := _DECK_COPY_RULE.fullmatch(line.strip()))
    }
    if len(limits) > 1:
        raise SourceSchemaError("conflicting printed deck copy limits", source="official")
    return next(iter(limits), None)


def _split_one_shot_security_lines(text: str) -> tuple[str, list[str]]:
    """Separate embedded one-shot security clauses from a generic effect box.

    Some official rows place a one-shot ``[Security]`` clause and the main
    effect in one HTML ``Effect`` section.  Security-area continuous effects,
    such as ``[Security] [All Turns] ...``, remain main text because the second
    timing marker makes them non-one-shot effects.
    """

    effect_lines: list[str] = []
    security_lines: list[str] = []
    for line in text.splitlines():
        stripped = line.strip()
        if stripped.startswith("[Security]"):
            remainder = stripped[len("[Security]") :].lstrip()
            if remainder and not remainder.startswith("["):
                security_lines.append(stripped)
                continue
        effect_lines.append(line)
    return "\n".join(effect_lines).strip(), security_lines


def _render_face_section(label: str, text: str) -> str:
    """Render face text while stripping source-only container headings."""

    text = _normalize_printed_effect_text(text)
    normalized_label = _normalized_section_label(label)
    if not label or normalized_label in _GENERIC_TEXT_SECTION_LABELS:
        return text
    return f"[{label}] {text}".strip()


def _face_kind(value: Any, *, observation: SourceObservation, ordinal: int) -> FaceKind:
    if observation.source is SourceName.OFFICIAL and value == "\u30c7\u30b8\u30e2\u30f3":
        value = "Digimon"
    try:
        return FaceKind(str(value).lower().replace("-", "_").replace(" ", "_"))
    except ValueError as exc:
        raise UnsupportedLayoutError(
            "source observation contains an unsupported card face kind",
            card_id=str(observation.extracted.get("card_id") or observation.source_record_key),
            source=observation.source.value,
            field_path=f"faces.{ordinal}.kind",
            details={"observation_id": observation.observation_id, "kind": value},
        ) from exc


def _parse_faces(
    raw_faces: Any,
    *,
    observation: SourceObservation,
) -> tuple[CardFace, ...]:
    if raw_faces in (None, []):
        return ()
    if not isinstance(raw_faces, list) or not all(
        isinstance(face, Mapping) for face in raw_faces
    ):
        raise SourceSchemaError(
            "faces must be an array of objects",
            card_id=observation.source_record_key.split("#", 1)[0],
            source=observation.source.value,
            field_path="faces",
        )
    if len(raw_faces) > 2:
        raise UnsupportedLayoutError(
            "canonical schema supports at most two card faces",
            card_id=observation.source_record_key.split("#", 1)[0],
            source=observation.source.value,
            field_path="faces",
            details={
                "observation_id": observation.observation_id,
                "face_count": len(raw_faces),
            },
        )

    parsed: list[CardFace] = []
    for ordinal, raw in enumerate(raw_faces):
        kind = _face_kind(raw.get("kind"), observation=observation, ordinal=ordinal)
        colors = raw.get("colors", raw.get("card_colors", []))
        if isinstance(colors, str):
            colors = [colors]
        if not isinstance(colors, list):
            raise SourceSchemaError(
                "face colors must be an array",
                source=observation.source.value,
                field_path=f"faces.{ordinal}.colors",
            )
        if not colors:
            raise SourceSchemaError(
                "face colors must not be empty",
                source=observation.source.value,
                field_path=f"faces.{ordinal}.colors",
            )
        text_sections = raw.get("text_sections", [])
        if not isinstance(text_sections, list):
            raise SourceSchemaError(
                "face text_sections must be an array",
                source=observation.source.value,
                field_path=f"faces.{ordinal}.text_sections",
            )
        effect_parts: list[str] = []
        inherited_parts: list[str] = []
        security_parts: list[str] = []
        has_explicit_security = any(
            isinstance(section, Mapping)
            and "security" in _normalized_section_label(str(section.get("label", "")))
            for section in text_sections
        )
        for section in text_sections:
            if not isinstance(section, Mapping):
                raise SourceSchemaError(
                    "face text section must be an object",
                    source=observation.source.value,
                    field_path=f"faces.{ordinal}.text_sections",
                )
            label = str(section.get("label", ""))
            text = str(section.get("text", ""))
            if observation.source is SourceName.OFFICIAL and _link_section_effect(label, text) is not None:
                # Link effects have their own canonical field, not a main or
                # inherited effect on the Digimon's structural face.
                continue
            rendered = _render_face_section(label, text)
            lowered = _normalized_section_label(label)
            if "security" in lowered:
                security_parts.append(rendered)
            elif "inherited" in lowered or "inheritable" in lowered:
                inherited_parts.append(rendered)
            else:
                embedded_security: list[str] = []
                if not has_explicit_security and lowered in {"", "effect", "dual effect"}:
                    rendered, embedded_security = _split_one_shot_security_lines(rendered)
                security_parts.extend(embedded_security)
                if not rendered:
                    continue
                if kind is FaceKind.DIGI_EGG and lowered in {"", "effect"}:
                    inherited_parts.append(rendered)
                else:
                    effect_parts.append(rendered)
        effect_text = (
            _normalize_printed_effect_text(str(raw.get("effect_text", "")))
            or "\n".join(effect_parts)
        )
        inherited_text = (
            _normalize_printed_effect_text(str(raw.get("inherited_text", "")))
            or "\n".join(inherited_parts)
        )
        security_text = (
            _normalize_printed_effect_text(str(raw.get("security_text", "")))
            or "\n".join(security_parts)
        )
        if (
            kind in {FaceKind.OPTION, FaceKind.TAMER}
            and not security_text
            and (recovered := _misplaced_security_text(inherited_text)) is not None
        ):
            inherited_text = ""
            security_text = recovered
        parsed.append(
            CardFace(
                face_id=str(raw.get("face_id", ordinal)),
                kind=kind,
                colors=tuple(str(color) for color in colors),
                play_cost=raw.get("play_cost", raw.get("use_cost")),
                level=raw.get("level"),
                dp=raw.get("dp"),
                effect_text=effect_text,
                inherited_text=inherited_text,
                security_text=security_text,
            )
        )
    if len(parsed) == 2 and {face.kind for face in parsed} != {
        FaceKind.DIGIMON,
        FaceKind.OPTION,
    }:
        raise UnsupportedLayoutError(
            "two-face cards must contain one Digimon face and one Option face",
            card_id=observation.source_record_key.split("#", 1)[0],
            source=observation.source.value,
            field_path="faces",
            details={
                "observation_id": observation.observation_id,
                "kinds": [face.kind.value for face in parsed],
            },
        )
    return tuple(parsed)


def _official_compatibility_play_cost(extracted: Mapping[str, Any]) -> Any:
    """Return the official value for the legacy top-level cost field.

    A structural Digimon/Option Dual has no play cost on its Digimon face, but
    the compatibility ABI represents the Option face's explicit use cost in
    the single top-level ``play_cost`` slot.  Keep that projection tied to the
    official observation instead of allowing the secondary source to become
    canonical provenance for the same printed value.
    """

    play_cost = extracted.get("play_cost")
    if play_cost is not None:
        return play_cost

    card_types = tuple(
        part.strip().casefold().replace("-", "_")
        for part in str(extracted.get("card_type", "")).split("/")
        if part.strip()
    )
    if card_types != ("digimon", "option"):
        return None

    raw_faces = extracted.get("faces")
    if not isinstance(raw_faces, list) or len(raw_faces) != 2:
        return None
    face_kinds = [
        str(face.get("kind", "")).casefold().replace("-", "_").replace(" ", "_")
        if isinstance(face, Mapping)
        else ""
        for face in raw_faces
    ]
    if sorted(face_kinds) != ["digimon", "option"]:
        return None

    option_face = raw_faces[face_kinds.index("option")]
    assert isinstance(option_face, Mapping)
    option_play_cost = option_face.get("play_cost")
    return (
        option_play_cost
        if option_play_cost is not None
        else option_face.get("use_cost")
    )


def _official_fields(extracted: Mapping[str, Any]) -> dict[str, Any]:
    """Project the official source-shaped record into canonical field names."""

    rule_grants = extracted.get("rule_grants", {})
    if not isinstance(rule_grants, Mapping):
        rule_grants = {}
    forms = _split_printed_values(extracted.get("form", ""))
    for granted in rule_grants.get("forms", []):
        for item in _split_printed_values(granted):
            if item not in forms:
                forms.append(item)
    types = _split_printed_values(extracted.get("type", ""))
    # The official database represents the Appmon mechanic in the Form row
    # (for example ``Stnd./Appmon``), while compatibility consumers expect it
    # in the trait list as well.
    if any(item.casefold() == "appmon" for item in forms) and "Appmon" not in types:
        types.append("Appmon")
    for granted in rule_grants.get("traits", []):
        for item in _split_printed_values(granted):
            if item not in types:
                types.append(item)
    attributes = _split_printed_values(extracted.get("attribute", ""))
    for granted in rule_grants.get("attributes", []):
        for item in _split_printed_values(granted):
            if item not in attributes:
                attributes.append(item)

    card_kind = str(extracted.get("card_type", "")).split("/", 1)[0].strip()
    if card_kind == "\u30c7\u30b8\u30e2\u30f3":
        card_kind = "Digimon"
    if card_kind.lower() == "digi-egg":
        card_kind = "Digi-Egg"

    sections = extracted.get("text_sections", [])
    raw_faces = extracted.get("faces", [])
    # A structural Dual record exposes both faces in the aggregate source
    # section list.  Top-level compatibility fields describe the primary
    # Digimon face; the Option face remains independent in ``faces`` and is
    # flattened only by the documented compatibility projection.
    if (
        isinstance(raw_faces, list)
        and len(raw_faces) == 2
        and isinstance(raw_faces[0], Mapping)
    ):
        sections = raw_faces[0].get("text_sections", sections)
    effect_parts: list[str] = []
    inherited_parts: list[str] = []
    security_parts: list[str] = []
    link_parts: list[str] = []
    if isinstance(sections, list):
        has_explicit_security = any(
            isinstance(section, Mapping)
            and "security" in _normalized_section_label(str(section.get("label", "")))
            for section in sections
        )
        for section in sections:
            if not isinstance(section, Mapping):
                continue
            label = str(section.get("label", ""))
            text = str(section.get("text", ""))
            lowered = _normalized_section_label(label)
            link_effect = _link_section_effect(label, text)
            if link_effect is not None:
                link_parts.append(link_effect)
            elif "security" in lowered:
                rendered = _render_effect_section(label, text)
                if rendered:
                    security_parts.append(rendered)
            elif "inherited" in lowered or "inheritable" in lowered:
                rendered = _render_effect_section(label, text)
                if rendered:
                    inherited_parts.append(rendered)
            else:
                rendered = _render_effect_section(label, text)
                embedded_security: list[str] = []
                if (
                    rendered
                    and not has_explicit_security
                    and lowered in {"", "effect", "dual effect"}
                ):
                    rendered, embedded_security = _split_one_shot_security_lines(
                        rendered
                    )
                security_parts.extend(embedded_security)
                if not rendered:
                    continue
                if card_kind == "Digi-Egg" and lowered in {"", "effect"}:
                    inherited_parts.append(rendered)
                else:
                    effect_parts.append(rendered)
    mapped = {
        "card_name_eng": extracted.get("name", ""),
        # An empty card-type cell is a source gap, not a competing gameplay
        # value. Omitting it lets another retained printing provide the field
        # and avoids manufacturing an empty-string conflict.
        "card_kind": card_kind or None,
        "card_colors": extracted.get("colors", []),
        "play_cost": _official_compatibility_play_cost(extracted),
        "level": extracted.get("level"),
        "dp": extracted.get("dp"),
        "form_eng": forms,
        "attribute_eng": attributes,
        "type_eng": list(dict.fromkeys(types)),
        "evo_costs": extracted.get("evolution_routes", []),
        "effect_description_eng": "\n".join(effect_parts),
        "inherited_effect_description_eng": "\n".join(inherited_parts),
        "security_effect_description_eng": "\n".join(security_parts),
    }
    if link_parts:
        # The official page can mirror one printed Link box in both legacy
        # inherited and dedicated Link containers (e.g. P-190). Compare whole
        # normalized containers, never individual lines or merely timings.
        # Conflicting containers need source review, not concatenated triggers.
        if any(part != link_parts[0] for part in link_parts[1:]):
            raise SourceSchemaError("conflicting official Link effect containers", source="official")
        mapped["link_effect_description_eng"] = link_parts[0]
    if isinstance(sections, list):
        limit = _printed_deck_copy_limit(section for section in sections if isinstance(section, Mapping))
        if limit is not None:
            mapped["max_count_in_deck"] = limit
    return {key: value for key, value in mapped.items() if value is not None}


def observation_candidate(observation: SourceObservation) -> ObservationCandidate:
    extracted = observation.extracted
    card_id = str(
        extracted.get("card_id") or observation.source_record_key.split("#", 1)[0]
    ).upper()
    raw_fields = extracted.get("canonical_fields", extracted.get("fields"))
    if raw_fields is None and observation.source is SourceName.OFFICIAL:
        raw_fields = _official_fields(extracted)
    if raw_fields is None:
        source_record = extracted.get("source_record", extracted)
        if not isinstance(source_record, Mapping):
            raise SourceSchemaError(
                "source observation has no structured field mapping",
                card_id=card_id,
                source=observation.source.value,
            )
        raw_fields = {
            key: source_record[key] for key in _KNOWN_FIELD_KEYS if key in source_record
        }
    if not isinstance(raw_fields, Mapping):
        raise SourceSchemaError(
            "canonical_fields must be an object",
            card_id=card_id,
            source=observation.source.value,
            field_path="canonical_fields",
        )
    printing = extracted.get("printing", {})
    if not isinstance(printing, Mapping):
        raise SourceSchemaError(
            "printing metadata must be an object",
            card_id=card_id,
            source=observation.source.value,
            field_path="printing",
        )
    printing_id = str(
        printing.get("printing_id") or extracted.get("printing_id") or observation.source_record_key
    )
    language = str(printing.get("language", extracted.get("language", "en"))).lower()
    is_current = bool(printing.get("is_current", False))
    is_errata = bool(printing.get("is_errata", False))
    effective_date = str(printing.get("effective_date", ""))
    return ObservationCandidate(
        observation=observation,
        gameplay_card_id=card_id,
        printing_id=printing_id,
        language=language,
        product=(str(printing["product"]) if printing.get("product") is not None else None),
        alternate_art=bool(printing.get("alternate_art", False)),
        fields=_normalize_candidate_fields(raw_fields),
        faces=_parse_faces(extracted.get("faces"), observation=observation),
        selection_priority=(
            1 if language == "en" else 0,
            1 if is_current or is_errata else 0,
            1 if not bool(printing.get("alternate_art", False)) else 0,
            effective_date,
        ),
    )


def _value_key(value: Any) -> str:
    return json.dumps(canonicalize(value), ensure_ascii=False, sort_keys=True, separators=(",", ":"))


def _candidate_value_rows(
    field_path: str,
    candidates: Iterable[ObservationCandidate],
) -> tuple[Mapping[str, Any], ...]:
    """Group exact candidate values with their immutable printing provenance."""

    grouped: dict[str, dict[str, Any]] = {}
    for candidate in candidates:
        value = canonicalize(candidate.fields[field_path])
        key = _value_key(value)
        row = grouped.setdefault(
            key,
            {
                "value": value,
                "value_digest": content_digest(value),
                "observation_ids": [],
                "printing_ids": [],
            },
        )
        row["observation_ids"].append(candidate.observation.observation_id)
        row["printing_ids"].append(candidate.printing_id)
    return tuple(
        {
            **grouped[key],
            "observation_ids": sorted(set(grouped[key]["observation_ids"])),
            "printing_ids": sorted(set(grouped[key]["printing_ids"])),
        }
        for key in sorted(grouped)
    )


def _canonical_conflict_binding(
    *,
    card_id: str,
    field_path: str,
    candidates: list[ObservationCandidate],
    selected_value: Any,
    selected_observation_id: str,
    old_input_revision: str,
    new_input_revision: str,
) -> CanonicalConflictBinding:
    candidate_values = _candidate_value_rows(field_path, candidates)
    return CanonicalConflictBinding(
        card_id=card_id,
        field_path=field_path,
        old_input_revision=old_input_revision,
        new_input_revision=new_input_revision,
        selected_value=canonicalize(selected_value),
        selected_value_digest=content_digest(selected_value),
        selected_observation_id=selected_observation_id,
        candidate_observation_ids=tuple(
            sorted(
                {
                    candidate.observation.observation_id
                    for candidate in candidates
                }
            )
        ),
        candidate_values=candidate_values,
        candidate_digest=content_digest(candidate_values),
    )


def _canonical_adjudication_matches(
    item: MigrationAdjudication,
    binding: CanonicalConflictBinding,
) -> bool:
    """Require every reviewed source/value dimension to remain exact."""

    return (
        item.kind is AdjudicationKind.CANONICAL_FIELD_SELECTION
        and item.card_id == binding.card_id
        and item.field_path == binding.field_path
        and item.selected_observation_id == binding.selected_observation_id
        and tuple(sorted(item.provenance_observation_ids))
        == binding.candidate_observation_ids
        and canonicalize(item.new_value) == binding.selected_value
        and (
            item.old_value is None
            or canonicalize(item.old_value) == canonicalize(binding.candidate_values)
        )
        and item.is_current(
            old_input_revision=binding.old_input_revision,
            new_input_revision=binding.new_input_revision,
            old_digest=binding.candidate_digest,
            new_digest=binding.selected_value_digest,
        )
    )


def _current_canonical_conflict_adjudication(
    binding: CanonicalConflictBinding,
    adjudications: Iterable[MigrationAdjudication],
) -> MigrationAdjudication | None:
    matches = [
        item
        for item in adjudications
        if _canonical_adjudication_matches(item, binding)
    ]
    if len(matches) > 1:
        raise AdjudicationError(
            "multiple current canonical field-selection adjudications match one conflict",
            card_id=binding.card_id,
            field_path=binding.field_path,
            details={
                "adjudication_ids": sorted(item.adjudication_id for item in matches)
            },
        )
    return matches[0] if matches else None


def _ranked_candidates(
    candidates: Iterable[ObservationCandidate],
) -> list[ObservationCandidate]:
    """Rank candidates with an order-independent final tie-break."""

    return sorted(
        candidates,
        key=lambda candidate: (
            candidate.selection_priority,
            candidate.printing_id,
            candidate.observation.observation_id,
        ),
        reverse=True,
    )


def _official_card_kind_candidates(
    candidates: list[ObservationCandidate],
) -> tuple[list[ObservationCandidate], bool]:
    """Choose the trustworthy official printing set for ``card_kind``.

    Card kind is structural and cannot legitimately vary by alternate art.
    A uniquely supported value across distinct English printings therefore
    outranks the ordinary non-alt preference. Explicit current/errata
    observations remain stronger than historical consensus, and a tied
    consensus falls back to the normal deterministic printing priority.
    """

    language_priority = max(candidate.selection_priority[0] for candidate in candidates)
    language_candidates = [
        candidate
        for candidate in candidates
        if candidate.selection_priority[0] == language_priority
    ]
    current_priority = max(
        candidate.selection_priority[1] for candidate in language_candidates
    )
    if current_priority:
        return (
            [
                candidate
                for candidate in language_candidates
                if candidate.selection_priority[1] == current_priority
            ],
            False,
        )

    printing_support: dict[str, set[str]] = {}
    for candidate in language_candidates:
        printing_support.setdefault(_value_key(candidate.fields["card_kind"]), set()).add(
            candidate.printing_id
        )
    strongest = max(len(printing_ids) for printing_ids in printing_support.values())
    strongest_values = [
        value_key
        for value_key, printing_ids in printing_support.items()
        if len(printing_ids) == strongest
    ]
    if len(strongest_values) != 1:
        return language_candidates, False

    selected_value = strongest_values[0]
    used_consensus = len(printing_support) > 1
    return (
        [
            candidate
            for candidate in language_candidates
            if _value_key(candidate.fields["card_kind"]) == selected_value
        ],
        used_consensus,
    )


def _select_field(
    card_id: str,
    field_path: str,
    candidates: list[ObservationCandidate],
    *,
    admission: AdmissionStatus,
    adjudications: Iterable[MigrationAdjudication],
    field_selection_input_revision: str,
) -> tuple[Any, FieldProvenance, list[FieldConflict]]:
    official = [candidate for candidate in candidates if candidate.authority is AuthorityClass.OFFICIAL]
    eligible = official or candidates
    authority = AuthorityClass.OFFICIAL if official else AuthorityClass.SECONDARY
    by_value: dict[str, list[ObservationCandidate]] = {}
    for candidate in eligible:
        by_value.setdefault(_value_key(candidate.fields[field_path]), []).append(candidate)

    used_card_kind_consensus = False
    selection_candidates = eligible
    if field_path == "card_kind" and official:
        selection_candidates, used_card_kind_consensus = _official_card_kind_candidates(
            eligible
        )
        ranked = _ranked_candidates(selection_candidates)
    else:
        ranked = _ranked_candidates(selection_candidates)
    best = ranked[0]
    top_priority = best.selection_priority
    top = [candidate for candidate in ranked if candidate.selection_priority == top_priority]
    equally_preferred_disagreement = (
        len({_value_key(candidate.fields[field_path]) for candidate in top}) > 1
    )

    all_observation_ids = tuple(
        sorted(candidate.observation.observation_id for candidate in candidates)
    )
    conflicts: list[FieldConflict] = []
    approval: MigrationAdjudication | None = None
    if len({_value_key(candidate.fields[field_path]) for candidate in candidates}) > 1:
        official_values = {
            _value_key(candidate.fields[field_path]) for candidate in official
        }
        conflict_kind = (
            FieldConflictKind.OFFICIAL_PRINTING
            if len(official_values) > 1
            else FieldConflictKind.OFFICIAL_SECONDARY
            if official
            else FieldConflictKind.SECONDARY_PRINTING
        )
        conflict = FieldConflict(
            field_path=field_path,
            candidate_observation_ids=all_observation_ids,
            kind=conflict_kind,
            reason=(
                "equally preferred printings disagree; deterministic identity tie-break selected"
                if equally_preferred_disagreement
                else "official printing consensus selected"
                if used_card_kind_consensus
                else "official value selected over conflicting secondary observation"
                if official and len(official) != len(candidates)
                else "printing observations disagree; deterministic English/errata priority selected"
            ),
        )
        if conflict.requires_adjudication(admission):
            binding = _canonical_conflict_binding(
                card_id=card_id,
                field_path=field_path,
                candidates=candidates,
                selected_value=best.fields[field_path],
                selected_observation_id=best.observation.observation_id,
                old_input_revision=field_selection_input_revision,
                new_input_revision=field_selection_input_revision,
            )
            approval = _current_canonical_conflict_adjudication(
                binding, adjudications
            )
        conflicts.append(conflict)
    return (
        best.fields[field_path],
        FieldProvenance(
            selected_observation_id=best.observation.observation_id,
            observation_ids=all_observation_ids,
            authority=authority,
            adjudicated=approval is not None,
        ),
        conflicts,
    )


def _current_adjudication(
    adjudications: Iterable[MigrationAdjudication],
    *,
    kind: AdjudicationKind,
    card_id: str,
    old_input_revision: str,
    new_input_revision: str,
    old_digest: str,
    new_digest: str,
) -> MigrationAdjudication | None:
    matches = [
        item
        for item in adjudications
        if item.kind is kind
        and item.card_id == card_id
        and item.is_current(
            old_input_revision=old_input_revision,
            new_input_revision=new_input_revision,
            old_digest=old_digest,
            new_digest=new_digest,
        )
    ]
    if len(matches) > 1:
        raise AdjudicationError(
            "multiple current migration adjudications match one card decision",
            card_id=card_id,
            details={"kind": kind.value},
        )
    return matches[0] if matches else None


def normalize_card(
    card_id: str,
    observations: Iterable[SourceObservation],
    *,
    adjudications: Iterable[MigrationAdjudication] = (),
    prior_card: CanonicalCard | None = None,
    old_input_revision: str = "0" * 64,
    new_input_revision: str = "0" * 64,
    field_selection_input_revision: str | None = None,
    old_digest: str = "0" * 64,
    new_digest: str = "0" * 64,
    defer_admission_if_missing_faces: bool = False,
    preserve_reviewed_admission: bool = False,
) -> CanonicalCard:
    """Normalize all observations for one gameplay card number."""

    candidates = [observation_candidate(item) for item in observations]
    if any(candidate.gameplay_card_id != card_id.upper() for candidate in candidates):
        raise ValidationError("normalization input contains another gameplay card ID", card_id=card_id)
    adjudications = tuple(adjudications)
    selection_input_revision = (
        field_selection_input_revision or new_input_revision
    )
    if not candidates:
        if prior_card is None:
            raise ValidationError("cannot normalize a card without observations", card_id=card_id)
        retirement = _current_adjudication(
            adjudications,
            kind=AdjudicationKind.SOURCE_RETIREMENT,
            card_id=card_id.upper(),
            old_input_revision=old_input_revision,
            new_input_revision=new_input_revision,
            old_digest=old_digest,
            new_digest=new_digest,
        )
        if retirement is None:
            raise AdjudicationError(
                "upstream removal requires a current source-retirement adjudication",
                card_id=card_id,
            )
        return replace(
            prior_card,
            admission=AdmissionStatus.RETIRED,
            tombstone_reason=retirement.reason,
        )

    official_present = any(
        candidate.authority is AuthorityClass.OFFICIAL for candidate in candidates
    )
    if preserve_reviewed_admission and (
        prior_card is None or prior_card.admission is not AdmissionStatus.ADMITTED
    ):
        raise ValidationError(
            "reviewed admission continuity requires a previously admitted card",
            card_id=card_id,
        )
    admission = AdmissionStatus.ADMITTED if official_present else AdmissionStatus.PROVISIONAL
    if not official_present:
        current_admission = _current_adjudication(
            adjudications,
            kind=AdjudicationKind.SOURCE_ADMISSION,
            card_id=card_id.upper(),
            old_input_revision=old_input_revision,
            new_input_revision=new_input_revision,
            old_digest=old_digest,
            new_digest=new_digest,
        )
        if current_admission is not None or preserve_reviewed_admission:
            admission = AdmissionStatus.ADMITTED

    field_names = sorted({field for candidate in candidates for field in candidate.fields})
    fields: dict[str, Any] = {}
    provenance: dict[str, FieldProvenance] = {}
    conflicts: list[FieldConflict] = []
    for field_name in field_names:
        field_candidates = [candidate for candidate in candidates if field_name in candidate.fields]
        value, source, field_conflicts = _select_field(
            card_id.upper(),
            field_name,
            field_candidates,
            admission=admission,
            adjudications=adjudications,
            field_selection_input_revision=selection_input_revision,
        )
        fields[field_name] = value
        provenance[field_name] = source
        conflicts.extend(field_conflicts)

    all_face_candidates = [candidate for candidate in candidates if candidate.faces]
    official_faces = [
        candidate for candidate in candidates
        if candidate.faces and candidate.authority is AuthorityClass.OFFICIAL
    ]
    face_candidates = official_faces or all_face_candidates
    faces = ()
    if face_candidates:
        ranked_faces = _ranked_candidates(face_candidates)
        top_priority = ranked_faces[0].selection_priority
        top_faces = [candidate for candidate in ranked_faces if candidate.selection_priority == top_priority]
        selected_face = ranked_faces[0]
        faces = selected_face.faces
        face_observation_ids = tuple(
            sorted(
                {
                    candidate.observation.observation_id
                    for candidate in all_face_candidates
                }
            )
        )
        face_approval: MigrationAdjudication | None = None
        # Ordinary one-face rows mirror top-level canonical fields, whose
        # individual conflicts are already reviewed independently.  A second
        # faces-level conflict would duplicate every DP/text errata decision.
        # Preserve a structural faces decision only when at least one retained
        # observation actually exposes a multi-face layout.
        structural_faces_disagree = (
            any(len(candidate.faces) > 1 for candidate in all_face_candidates)
            and len({_value_key(candidate.faces) for candidate in all_face_candidates})
            > 1
        )
        if structural_faces_disagree:
            official_face_values = {
                _value_key(candidate.faces) for candidate in official_faces
            }
            face_conflict_kind = (
                FieldConflictKind.OFFICIAL_PRINTING
                if len(official_face_values) > 1
                else FieldConflictKind.OFFICIAL_SECONDARY
                if official_faces
                else FieldConflictKind.SECONDARY_PRINTING
            )
            binding_candidates = [
                replace(
                    candidate,
                    fields={**candidate.fields, "faces": candidate.faces},
                )
                for candidate in all_face_candidates
            ]
            face_conflict = FieldConflict(
                field_path="faces",
                candidate_observation_ids=face_observation_ids,
                kind=face_conflict_kind,
                reason=(
                    "equally preferred printings disagree on structural card faces; deterministic identity tie-break selected"
                    if len({_value_key(candidate.faces) for candidate in top_faces}) > 1
                    else "official structural face selected over conflicting secondary observation"
                    if official_faces
                    and len(official_faces) != len(all_face_candidates)
                    else "printing observations disagree on structural card faces; deterministic English/errata priority selected"
                ),
            )
            if face_conflict.requires_adjudication(admission):
                face_binding = _canonical_conflict_binding(
                    card_id=card_id.upper(),
                    field_path="faces",
                    candidates=binding_candidates,
                    selected_value=selected_face.faces,
                    selected_observation_id=selected_face.observation.observation_id,
                    old_input_revision=selection_input_revision,
                    new_input_revision=selection_input_revision,
                )
                face_approval = _current_canonical_conflict_adjudication(
                    face_binding, adjudications
                )
            conflicts.append(face_conflict)
        provenance["faces"] = FieldProvenance(
            selected_observation_id=selected_face.observation.observation_id,
            observation_ids=face_observation_ids,
            authority=(
                AuthorityClass.OFFICIAL
                if official_faces
                else AuthorityClass.SECONDARY
            ),
            adjudicated=face_approval is not None,
        )

    printing_groups: dict[str, list[ObservationCandidate]] = {}
    for candidate in candidates:
        printing_groups.setdefault(candidate.printing_id, []).append(candidate)
    printing_rows: list[Printing] = []
    for printing_id, printing_candidates in sorted(printing_groups.items()):
        selected_printing = _ranked_candidates(printing_candidates)[0]
        printing_rows.append(
            Printing(
                printing_id=printing_id,
                observation_ids=tuple(
                    sorted(
                        {
                            candidate.observation.observation_id
                            for candidate in printing_candidates
                        }
                    )
                ),
                language=selected_printing.language,
                product=selected_printing.product,
                alternate_art=selected_printing.alternate_art,
            )
        )
    printings = tuple(printing_rows)
    # A secondary source can identify a structural card while omitting its
    # faces.  Complete-candidate assembly applies reviewed sparse corrections
    # after normalization, so it needs a valid intermediate record before a
    # current source-admission approval can be finalized.  Keep the effective
    # ``admission`` above for conflict classification, but defer the model-level
    # promotion until the reviewed face correction has been applied.
    record_admission = admission
    if (
        defer_admission_if_missing_faces
        and admission is AdmissionStatus.ADMITTED
        and not faces
    ):
        record_admission = AdmissionStatus.PROVISIONAL

    return CanonicalCard(
        schema_version=SCHEMA_VERSION,
        card_id=card_id,
        admission=record_admission,
        fields=fields,
        provenance=provenance,
        printings=printings,
        faces=faces,
        conflicts=tuple(sorted(conflicts, key=lambda item: item.field_path)),
    )


def normalize_cards(
    observations: Iterable[SourceObservation],
    **options: Any,
) -> dict[str, CanonicalCard]:
    grouped: dict[str, list[SourceObservation]] = {}
    for observation in observations:
        candidate = observation_candidate(observation)
        grouped.setdefault(candidate.gameplay_card_id, []).append(observation)
    return {
        card_id: normalize_card(card_id, grouped[card_id], **options)
        for card_id in sorted(grouped, key=natural_card_id_key)
    }


def canonical_conflict_review_rows(
    cards: Mapping[str, CanonicalCard],
    observations: Iterable[SourceObservation],
    *,
    adjudications: Iterable[MigrationAdjudication] = (),
    input_revision: str,
) -> list[dict[str, Any]]:
    """Describe every retained conflict and its exact adjudication boundary.

    Rows intentionally contain normalized field values rather than whole raw
    source payloads.  They are therefore actionable for review without
    duplicating immutable observation files in the candidate report.
    """

    grouped: dict[str, list[ObservationCandidate]] = {}
    for observation in observations:
        candidate = observation_candidate(observation)
        grouped.setdefault(candidate.gameplay_card_id, []).append(candidate)
    adjudications = tuple(adjudications)
    rows: list[dict[str, Any]] = []
    for card_id in sorted(cards, key=natural_card_id_key):
        card = cards[card_id]
        for conflict in sorted(card.conflicts, key=lambda item: item.field_path):
            if conflict.field_path == "faces":
                source_candidates = [
                    replace(
                        candidate,
                        fields={**candidate.fields, "faces": candidate.faces},
                    )
                    for candidate in grouped.get(card_id, ())
                    if candidate.faces
                ]
                selected_value: Any = card.faces
            else:
                source_candidates = list(grouped.get(card_id, ()))
                selected_value = card.fields[conflict.field_path]
            field_candidates = [
                candidate
                for candidate in source_candidates
                if conflict.field_path in candidate.fields
                and candidate.observation.observation_id
                in conflict.candidate_observation_ids
            ]
            provenance = card.provenance[conflict.field_path]
            binding = _canonical_conflict_binding(
                card_id=card_id,
                field_path=conflict.field_path,
                candidates=field_candidates,
                selected_value=selected_value,
                selected_observation_id=provenance.selected_observation_id,
                old_input_revision=input_revision,
                new_input_revision=input_revision,
            )
            approval_required = conflict.requires_adjudication(card.admission)
            approval = (
                _current_canonical_conflict_adjudication(binding, adjudications)
                if approval_required
                else None
            )
            relevant = (
                sorted(
                    item.adjudication_id
                    for item in adjudications
                    if item.kind is AdjudicationKind.CANONICAL_FIELD_SELECTION
                    and item.card_id == card_id
                    and item.field_path == conflict.field_path
                )
                if approval_required
                else []
            )
            status = (
                "informational"
                if not approval_required
                else "current"
                if approval
                else "stale"
                if relevant
                else "missing"
            )
            alternatives = [
                row
                for row in binding.candidate_values
                if row["value_digest"] != binding.selected_value_digest
            ]
            rows.append(
                {
                    "card_id": card_id,
                    "field_path": conflict.field_path,
                    "conflict_kind": conflict.kind.value,
                    "approval_required": approval_required,
                    "resolution_requirement": (
                        "approval_required" if approval_required else "informational"
                    ),
                    "reason": conflict.reason,
                    "status": status,
                    "adjudication_id": (
                        approval.adjudication_id if approval is not None else None
                    ),
                    "stale_adjudication_ids": (
                        [] if approval is not None else relevant
                    ),
                    "selected_value": binding.selected_value,
                    "selected_value_digest": binding.selected_value_digest,
                    "selected_observation_id": binding.selected_observation_id,
                    "alternative_values": alternatives,
                    "candidate_values": list(binding.candidate_values),
                    "candidate_digest": binding.candidate_digest,
                    "candidate_observation_ids": list(
                        binding.candidate_observation_ids
                    ),
                    "old_input_revision": binding.old_input_revision,
                    "new_input_revision": binding.new_input_revision,
                    "adjudication_template": {
                        "kind": AdjudicationKind.CANONICAL_FIELD_SELECTION.value,
                        "card_id": card_id,
                        "field_path": conflict.field_path,
                        "old_input_revision": binding.old_input_revision,
                        "new_input_revision": binding.new_input_revision,
                        "old_digest": binding.candidate_digest,
                        "new_digest": binding.selected_value_digest,
                        "new_value": binding.selected_value,
                        "selected_observation_id": (
                            binding.selected_observation_id
                        ),
                        "provenance_observation_ids": list(
                            binding.candidate_observation_ids
                        ),
                    }
                    if approval_required
                    else None,
                }
            )
    return rows
