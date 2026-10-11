"""Deterministic shadow generation for derived card-data compatibility artifacts.

The legacy data tools used to fetch or read their own inputs independently.
This module instead renders the official mirror, per-card bundles, the official
evolution-cost view, and the maintained author-set lexicons from one corrected
canonical card mapping and one explicit source-manifest revision.

Generation is deliberately pure: it returns bytes and never writes a fixed
production path.  Publication and consistent-revision reads are owned by the
transaction layer.  A deterministic sidecar binds every returned payload to
its schema and input revisions without changing the existing compatibility
payload shapes.
"""

from __future__ import annotations

import json
import re
from collections import Counter
from dataclasses import dataclass
from types import MappingProxyType
from typing import Any, Iterable, Mapping

from .corrections import (
    apply_corrections,
    correction_is_already_applied,
    validate_correction_set,
)
from .errors import ValidationError
from .generate import (
    LEGACY_TO_COLOR,
    CompatibilityProjection,
    bootstrap_legacy_canonical_cards,
    build_compatibility_projection,
    generate_cards_json,
    render_legacy_cards_json,
)
from .models import (
    SCHEMA_VERSION,
    CanonicalCard,
    ReviewedCorrection,
    SourceName,
    SourceObservation,
)
from .registry import StableCardRegistry
from .serialization import (
    canonical_json_bytes,
    canonicalize,
    content_digest,
    natural_card_id_key,
    sha256_hex,
)


DERIVED_ARTIFACT_SCHEMA_VERSION = 1
DERIVED_MANIFEST_PATH = "data/card_data_derived_artifacts.json"
OFFICIAL_MIRROR_PATH = "data/card_official.json"
EVOLUTION_COST_VIEW_PATH = "data/card_evolution_costs.json"
LEXICON_PATH = "data/author_set_lexicons.json"
BUNDLE_ROOT = "data/card_bundles"

_SHA256_RE = re.compile(r"^[0-9a-f]{64}$")
_SECTION_PREFIX_RE = re.compile(r"^\[([^\]]+)\]\s*(.*)$", re.DOTALL)
_OFFICIAL_BASE = "https://world.digimoncard.com/cards/?search=true&card_no="


@dataclass(frozen=True, slots=True)
class DerivedArtifactSet:
    """One immutable, revision-bound set of shadow-generated artifact bytes."""

    canonical_revision: str
    source_manifest_revision: str
    input_revision: str
    artifact_revision: str
    payloads: Mapping[str, bytes]
    manifest_bytes: bytes

    def __post_init__(self) -> None:
        for field_name in (
            "canonical_revision",
            "source_manifest_revision",
            "input_revision",
            "artifact_revision",
        ):
            _require_sha256(getattr(self, field_name), field_name)
        normalized = dict(sorted(self.payloads.items()))
        if DERIVED_MANIFEST_PATH in normalized:
            raise ValidationError(
                "derived payloads cannot contain their own sidecar manifest",
                path=DERIVED_MANIFEST_PATH,
            )
        for path, value in normalized.items():
            if not path.startswith("data/") or "\\" in path:
                raise ValidationError(
                    "derived artifact paths must be repository-relative data paths",
                    path=path,
                )
            if not isinstance(value, bytes):
                raise ValidationError(
                    "derived artifact payload must be bytes",
                    path=path,
                    details={"actual_type": type(value).__name__},
                )
        object.__setattr__(self, "payloads", MappingProxyType(normalized))

    def all_files(self) -> dict[str, bytes]:
        """Return payloads plus the deterministic revision/digest sidecar."""

        return {**self.payloads, DERIVED_MANIFEST_PATH: self.manifest_bytes}


def _require_sha256(value: str, field_name: str) -> str:
    if not isinstance(value, str) or not _SHA256_RE.fullmatch(value):
        raise ValidationError(f"{field_name} must be a lowercase sha256 digest")
    return value


def _clean_text(value: Any) -> str:
    return " ".join(str(value or "").split())


def _list_text(value: Any) -> str:
    if value is None:
        return ""
    if isinstance(value, (list, tuple)):
        return "/".join(_clean_text(item) for item in value if _clean_text(item))
    return _clean_text(value)


def _source_sections(extracted: Mapping[str, Any]) -> list[dict[str, str]]:
    sections: list[dict[str, str]] = []
    for raw in extracted.get("text_sections") or ():
        if not isinstance(raw, Mapping):
            continue
        label = _clean_text(raw.get("label"))
        text = _clean_text(raw.get("text"))
        if label or text:
            sections.append({"label": label or "Effect", "text": text})
    return sections


def _section_group(label: str) -> str:
    lowered = label.lower()
    if "security" in lowered:
        return "security"
    if "inherited" in lowered or "inheritable" in lowered:
        return "inherited"
    return "effect"


def _rendered_section_value(sections: Iterable[Mapping[str, str]]) -> str:
    return "\n".join(
        f"[{section['label']}] {section['text']}".strip() for section in sections
    )


def _parse_corrected_sections(value: Any, *, default_label: str) -> list[dict[str, str]]:
    text = str(value or "").strip()
    if not text:
        return []
    sections: list[dict[str, str]] = []
    for line in text.splitlines():
        line = line.strip()
        if not line:
            continue
        match = _SECTION_PREFIX_RE.fullmatch(line)
        if match:
            sections.append(
                {
                    "label": _clean_text(match.group(1)),
                    "text": _clean_text(match.group(2)),
                }
            )
        else:
            sections.append({"label": default_label, "text": _clean_text(line)})
    return sections


def _canonical_text_sections(
    row: Mapping[str, Any], extracted: Mapping[str, Any]
) -> list[dict[str, str]]:
    """Retain source labels unless a corrected canonical text differs."""

    source = _source_sections(extracted)
    grouped = {
        group: [item for item in source if _section_group(item["label"]) == group]
        for group in ("effect", "inherited", "security")
    }
    field_groups = (
        ("effect", "effect_description_eng", "Effect"),
        ("inherited", "inherited_effect_description_eng", "Inherited Effect"),
        ("security", "security_effect_description_eng", "Security Effect"),
    )
    result: list[dict[str, str]] = []
    for group, field_name, default_label in field_groups:
        canonical_value = str(row.get(field_name) or "").strip()
        source_value = _rendered_section_value(grouped[group])
        if canonical_value == source_value:
            result.extend(grouped[group])
        else:
            result.extend(
                _parse_corrected_sections(
                    canonical_value,
                    default_label=default_label,
                )
            )
    return result


def _referenced_source_observation(
    card: CanonicalCard,
    observations: Mapping[str, SourceObservation],
) -> SourceObservation:
    reference_ids: set[str] = set()
    for printing in card.printings:
        reference_ids.update(printing.observation_ids)
    for provenance in card.provenance.values():
        reference_ids.update(provenance.observation_ids)

    missing = sorted(reference_ids - set(observations))
    if missing:
        raise ValidationError(
            "canonical artifact generation has unresolved source observations",
            card_id=card.card_id,
            details={"observation_ids": missing},
        )
    candidates = [observations[item] for item in reference_ids]
    if not candidates:
        raise ValidationError(
            "canonical artifact generation requires a source observation",
            card_id=card.card_id,
        )
    selected = Counter(
        provenance.selected_observation_id for provenance in card.provenance.values()
    )
    candidates.sort(
        key=lambda observation: (
            0 if observation.source is SourceName.OFFICIAL else 1,
            -selected[observation.observation_id],
            observation.source_record_key,
            observation.observation_id,
        )
    )
    observation = candidates[0]
    extracted_card_id = str(observation.extracted.get("card_id") or card.card_id).upper()
    if extracted_card_id != card.card_id:
        raise ValidationError(
            "source observation card ID does not match canonical card",
            card_id=card.card_id,
            details={
                "observation_id": observation.observation_id,
                "source_card_id": extracted_card_id,
            },
        )
    return observation


def _official_record(
    card: CanonicalCard,
    row: Mapping[str, Any],
    observation: SourceObservation,
) -> dict[str, Any]:
    extracted = observation.extracted
    colors = " ".join(LEGACY_TO_COLOR[int(value)] for value in row["card_colors"])
    record: dict[str, Any] = {
        "card_id": card.card_id,
        "digivolve_costs": list(row.get("evo_costs") or ()),
        "colors": colors,
        "play_cost": str(row.get("play_cost", "")),
        "text_sections": _canonical_text_sections(row, extracted),
    }
    if row.get("dp") is not None:
        record["dp"] = str(row["dp"])
    for canonical_key, official_key in (
        ("form_eng", "form"),
        ("attribute_eng", "attribute"),
        ("type_eng", "type"),
    ):
        value = _list_text(row.get(canonical_key))
        if value:
            record[official_key] = value

    special = _clean_text(extracted.get("special_digivolution_condition"))
    if not special:
        special_routes = extracted.get("special_evolution_routes") or ()
        if isinstance(special_routes, str):
            special_routes = (special_routes,)
        if special_routes:
            special = _clean_text(special_routes[0])
    if special:
        record["special_digivolution_condition"] = special

    qa: list[str] = []
    for item in extracted.get("qa") or ():
        answer = item.get("answer") if isinstance(item, Mapping) else item
        answer = _clean_text(answer)
        if answer and answer not in qa:
            qa.append(answer)
    if qa:
        record["qa"] = qa
    return record


def _compatibility_fixture_official_record(
    card_id: str, row: Mapping[str, Any]
) -> dict[str, Any]:
    """Preserve the legacy mirror row for a declared non-source fixture.

    Synthetic engine fixtures intentionally have neither canonical printed data
    nor an official observation.  Keeping their old minimal mirror shape makes
    the compatibility universe stable without fabricating source provenance.
    """

    return {
        "card_id": card_id,
        "digivolve_costs": list(row.get("evo_costs") or ()),
        "text_sections": [],
    }


def _bundle_text(
    card_id: str,
    row: Mapping[str, Any],
    official: Mapping[str, Any],
    observation: SourceObservation,
) -> bytes:
    color_names = {
        0: "Red",
        1: "Blue",
        2: "Yellow",
        3: "Green",
        4: "White",
        5: "Black",
        6: "Purple",
    }
    evo_lines = [
        "  - "
        f"{color_names.get(item['card_color'], item['card_color'])} "
        f"Lv.{item['level']} / cost {item['memory_cost']}"
        for item in official.get("digivolve_costs") or ()
    ] or ["  - (none — Digi-Egg / Lv.2 base, or DNA/special only)"]
    lines = [
        f"# {row.get('card_name_eng') or card_id}  ({card_id})",
        "",
        "> Authoritative resource. Printed fields and text are projected from the",
        "> canonical card-data revision and its retained source observations.",
        "",
        "## Identity",
        f"- Colors: {official.get('colors', '')}",
        f"- Level: {row.get('level')}",
        f"- DP: {official.get('dp', row.get('dp'))}",
        f"- Play cost: {official.get('play_cost', row.get('play_cost'))}",
        f"- Type/Traits: {official.get('type', '')}",
        f"- Form: {official.get('form', '')}   Attribute: {official.get('attribute', '')}".rstrip(),
        "",
        "## Digivolution requirements (official source projection)",
        "### Standard digivolve cost circles",
        *evo_lines,
    ]
    if official.get("special_digivolution_condition"):
        lines.extend(
            [
                "### Special digivolution condition (DNA / alt-digivolve black text)",
                f"  {official['special_digivolution_condition']}",
            ]
        )
    lines.extend(["", "## Printed text (official source projection)"])
    for section in official.get("text_sections") or ():
        lines.extend([f"### {section['label']}", section["text"], ""])
    if official.get("qa"):
        lines.extend(["## Official Q&A", *[f"- {item}" for item in official["qa"]]])
    image_url = ""
    printing = observation.extracted.get("printing")
    if isinstance(printing, Mapping):
        image_url = _clean_text(printing.get("image_url"))
    lines.extend(
        [
            "",
            "## Image",
            f"- {image_url or '(not recorded by source observation)'}",
            "",
            f"_Source: {observation.source_url or _OFFICIAL_BASE + card_id}_",
        ]
    )
    return ("\n".join(lines) + "\n").encode("utf-8")


def _compatibility_fixture_bundle_text(
    card_id: str,
    row: Mapping[str, Any],
    official: Mapping[str, Any],
) -> bytes:
    """Render a bundle for a typed compatibility fixture without source claims."""

    color_names = {
        0: "Red",
        1: "Blue",
        2: "Yellow",
        3: "Green",
        4: "White",
        5: "Black",
        6: "Purple",
    }
    colors = " ".join(
        color_names.get(int(value), str(value))
        for value in row.get("card_colors") or ()
    )
    evo_lines = [
        "  - "
        f"{color_names.get(item['card_color'], item['card_color'])} "
        f"Lv.{item['level']} / cost {item['memory_cost']}"
        for item in official.get("digivolve_costs") or ()
    ] or ["  - (none — test fixture does not declare a standard route)"]
    lines = [
        f"# {row.get('card_name_eng') or card_id}  ({card_id})",
        "",
        "> Compatibility-only engine fixture. This bundle is projected from the",
        "> marker-owned compatibility sidecar, not from official printed data.",
        "",
        "## Identity",
        f"- Colors: {colors}",
        f"- Level: {row.get('level')}",
        f"- DP: {row.get('dp')}",
        f"- Play cost: {row.get('play_cost')}",
        f"- Type/Traits: {_list_text(row.get('type_eng'))}",
        f"- Form: {_list_text(row.get('form_eng'))}   "
        f"Attribute: {_list_text(row.get('attribute_eng'))}",
        "",
        "## Declared compatibility digivolution requirements",
        *evo_lines,
        "",
        "## Declared compatibility text",
    ]
    for label, field_name in (
        ("Effect", "effect_description_eng"),
        ("Inherited Effect", "inherited_effect_description_eng"),
        ("Security Effect", "security_effect_description_eng"),
    ):
        value = _clean_text(row.get(field_name))
        if value:
            lines.extend([f"### {label}", value, ""])
    lines.extend(
        [
            "_Source: marker-owned data/card_compatibility_projection.json fixture row_",
        ]
    )
    return ("\n".join(lines) + "\n").encode("utf-8")


def _lexicons(rows: Mapping[str, Mapping[str, Any]]) -> dict[str, Any]:
    traits: set[str] = set()
    card_names: set[str] = set()
    for row in rows.values():
        name = _clean_text(row.get("card_name_eng")).lower()
        if name:
            card_names.add(name)
        value = row.get("type_eng")
        values = value if isinstance(value, (list, tuple)) else (value,)
        for item in values:
            trait = _clean_text(item).lower()
            if trait:
                traits.add(trait)
    return {
        "traits": sorted(traits),
        "card_names": sorted(card_names),
        "_comment": (
            "Generated by tools.card_data.derived_artifacts from the full "
            "canonical compatibility universe. Never build from a per-set sample."
        ),
    }


def _artifact_schema(path: str) -> str:
    if path == OFFICIAL_MIRROR_PATH:
        return "legacy-card-official-json-v0"
    if path == EVOLUTION_COST_VIEW_PATH:
        return "legacy-official-evolution-cost-view-v0"
    if path == LEXICON_PATH:
        return "author-set-lexicons-v0"
    if path.startswith(f"{BUNDLE_ROOT}/") and path.endswith(".md"):
        return "card-bundle-markdown-v1"
    raise ValidationError("unclassified derived artifact schema", path=path)


def _manifest_payload(
    payloads: Mapping[str, bytes],
    *,
    canonical_revision: str,
    source_manifest_revision: str,
    corrections_revision: str,
    registry_revision: str,
    compatibility_projection_revision: str | None = None,
) -> tuple[dict[str, Any], str, str]:
    inputs = {
        "canonical_schema_version": SCHEMA_VERSION,
        "generator_schema_version": DERIVED_ARTIFACT_SCHEMA_VERSION,
        "canonical_revision": canonical_revision,
        "source_manifest_revision": source_manifest_revision,
        "corrections_revision": corrections_revision,
        "registry_revision": registry_revision,
    }
    if compatibility_projection_revision is not None:
        inputs["compatibility_projection_revision"] = (
            compatibility_projection_revision
        )
    input_revision = content_digest(inputs)
    artifacts = {
        path: {
            "schema": _artifact_schema(path),
            "sha256": sha256_hex(value),
            "size": len(value),
        }
        for path, value in sorted(payloads.items())
    }
    artifact_revision = content_digest(artifacts)
    return (
        {
            "schema_version": DERIVED_ARTIFACT_SCHEMA_VERSION,
            "generator": "tools.card_data.derived_artifacts",
            **inputs,
            "input_revision": input_revision,
            "artifact_revision": artifact_revision,
            "artifacts": artifacts,
        },
        input_revision,
        artifact_revision,
    )


def generate_derived_artifacts(
    canonical_cards: Mapping[str, CanonicalCard],
    registry: StableCardRegistry,
    observations: Mapping[str, SourceObservation],
    *,
    source_manifest_revision: str,
    corrections: Iterable[ReviewedCorrection] = (),
    legacy_bootstrap: Mapping[str, Mapping[str, Any]] | None = None,
    compatibility_projection: CompatibilityProjection | None = None,
    corrections_already_applied: bool = False,
) -> DerivedArtifactSet:
    """Render every non-``cards.json`` compatibility product from one revision.

    The caller supplies the digest of the complete source-manifest document.
    The corrected canonical, correction-ledger, and registry revisions are
    computed from the exact in-memory inputs.  No clock or environment-specific
    path participates in any output byte.
    """

    _require_sha256(source_manifest_revision, "source_manifest_revision")
    ordered_corrections = validate_correction_set(corrections)
    effective_cards = dict(canonical_cards)
    if corrections_already_applied:
        stale_corrections = [
            correction.correction_id
            for correction in ordered_corrections
            if correction.card_id not in effective_cards
            or not correction_is_already_applied(
                effective_cards[correction.card_id],
                correction,
                observations=observations,
            )
        ]
        if stale_corrections:
            raise ValidationError(
                "derived generation received unapplied reviewed corrections",
                details={"correction_ids": stale_corrections},
            )
    elif ordered_corrections:
        effective_cards = apply_corrections(
            effective_cards,
            ordered_corrections,
            observations=observations,
        )
    rows = generate_cards_json(
        (
            {
                **effective_cards,
                **bootstrap_legacy_canonical_cards(
                    compatibility_projection.fixture_rows
                ),
            }
            if compatibility_projection is not None
            else effective_cards
        ),
        registry,
        legacy_bootstrap=(
            compatibility_projection.compatibility_fields
            if compatibility_projection is not None
            else legacy_bootstrap
        ),
    )
    canonical_revision = content_digest(effective_cards)
    corrections_revision = content_digest(ordered_corrections)
    registry_revision = content_digest(registry)

    fixture_ids: frozenset[str] = frozenset()
    if compatibility_projection is not None:
        if legacy_bootstrap is not None:
            raise ValidationError(
                "derived generation accepts either a compatibility projection "
                "or a legacy bootstrap, not both"
            )
        expected_projection = build_compatibility_projection(
            effective_cards,
            registry,
            corrections=ordered_corrections,
            source_manifest_revision=source_manifest_revision,
            compatibility_fields=compatibility_projection.compatibility_fields,
            fixture_rows=compatibility_projection.fixture_rows,
            projected_cards=render_legacy_cards_json(rows),
        )
        if canonicalize(compatibility_projection.to_dict()) != canonicalize(
            expected_projection.to_dict()
        ):
            raise ValidationError(
                "compatibility projection is stale for derived inputs"
            )
        fixture_ids = frozenset(compatibility_projection.fixture_rows)

    official_cards: dict[str, dict[str, Any]] = {}
    source_by_card: dict[str, SourceObservation] = {}
    for card_id in sorted(rows, key=natural_card_id_key):
        if card_id in fixture_ids:
            official_cards[card_id] = _compatibility_fixture_official_record(
                card_id, rows[card_id]
            )
            continue
        card = effective_cards[card_id]
        observation = _referenced_source_observation(card, observations)
        source_by_card[card_id] = observation
        official_cards[card_id] = _official_record(card, rows[card_id], observation)

    payloads: dict[str, bytes] = {
        OFFICIAL_MIRROR_PATH: canonical_json_bytes(
            {"count": len(official_cards), "failed": [], "cards": official_cards}
        ),
        EVOLUTION_COST_VIEW_PATH: canonical_json_bytes(
            {
                "count": len(official_cards),
                "failed": [],
                "evo_costs": {
                    card_id: official_cards[card_id]["digivolve_costs"]
                    for card_id in sorted(official_cards, key=natural_card_id_key)
                },
            }
        ),
        LEXICON_PATH: canonical_json_bytes(_lexicons(rows)),
    }
    for card_id in sorted(rows, key=natural_card_id_key):
        if card_id in fixture_ids:
            payloads[f"{BUNDLE_ROOT}/{card_id}.md"] = (
                _compatibility_fixture_bundle_text(
                    card_id,
                    rows[card_id],
                    official_cards[card_id],
                )
            )
        else:
            payloads[f"{BUNDLE_ROOT}/{card_id}.md"] = _bundle_text(
                card_id,
                rows[card_id],
                official_cards[card_id],
                source_by_card[card_id],
            )

    manifest, input_revision, artifact_revision = _manifest_payload(
        payloads,
        canonical_revision=canonical_revision,
        source_manifest_revision=source_manifest_revision,
        corrections_revision=corrections_revision,
        registry_revision=registry_revision,
        compatibility_projection_revision=(
            compatibility_projection.projection_revision
            if compatibility_projection is not None
            else None
        ),
    )
    result = DerivedArtifactSet(
        canonical_revision=canonical_revision,
        source_manifest_revision=source_manifest_revision,
        input_revision=input_revision,
        artifact_revision=artifact_revision,
        payloads=payloads,
        manifest_bytes=canonical_json_bytes(manifest),
    )
    verify_derived_artifacts(result.all_files())
    return result


def verify_derived_artifacts(files: Mapping[str, bytes]) -> Mapping[str, Any]:
    """Verify the deterministic sidecar against an in-memory artifact set."""

    manifest_bytes = files.get(DERIVED_MANIFEST_PATH)
    if manifest_bytes is None:
        raise ValidationError(
            "derived artifact set is missing its revision sidecar",
            path=DERIVED_MANIFEST_PATH,
        )
    try:
        manifest = json.loads(manifest_bytes.decode("utf-8"))
    except (UnicodeDecodeError, json.JSONDecodeError) as exc:
        raise ValidationError(
            "derived artifact revision sidecar is invalid JSON",
            path=DERIVED_MANIFEST_PATH,
            details={"error": str(exc)},
        ) from exc
    if (
        not isinstance(manifest, Mapping)
        or manifest.get("schema_version") != DERIVED_ARTIFACT_SCHEMA_VERSION
    ):
        raise ValidationError(
            "unsupported derived artifact sidecar schema",
            path=DERIVED_MANIFEST_PATH,
        )
    artifacts = manifest.get("artifacts")
    if not isinstance(artifacts, Mapping):
        raise ValidationError(
            "derived artifact sidecar requires an artifacts mapping",
            path=DERIVED_MANIFEST_PATH,
        )
    payload_paths = set(files) - {DERIVED_MANIFEST_PATH}
    declared_paths = set(artifacts)
    if payload_paths != declared_paths:
        raise ValidationError(
            "derived artifact sidecar membership does not match payloads",
            path=DERIVED_MANIFEST_PATH,
            details={
                "undeclared": sorted(payload_paths - declared_paths),
                "missing": sorted(declared_paths - payload_paths),
            },
        )
    for path in sorted(declared_paths):
        declaration = artifacts[path]
        if not isinstance(declaration, Mapping):
            raise ValidationError("derived artifact declaration is invalid", path=path)
        actual_digest = sha256_hex(files[path])
        actual_size = len(files[path])
        if declaration.get("sha256") != actual_digest or declaration.get("size") != actual_size:
            raise ValidationError(
                "derived artifact payload does not match its sidecar",
                path=path,
                details={
                    "expected_sha256": declaration.get("sha256"),
                    "actual_sha256": actual_digest,
                    "expected_size": declaration.get("size"),
                    "actual_size": actual_size,
                },
            )
        if declaration.get("schema") != _artifact_schema(path):
            raise ValidationError(
                "derived artifact schema declaration is stale",
                path=path,
            )
    if manifest.get("artifact_revision") != content_digest(artifacts):
        raise ValidationError(
            "derived artifact revision does not match declared payloads",
            path=DERIVED_MANIFEST_PATH,
        )
    input_payload = {
        key: manifest.get(key)
        for key in (
            "canonical_schema_version",
            "generator_schema_version",
            "canonical_revision",
            "source_manifest_revision",
            "corrections_revision",
            "registry_revision",
        )
    }
    if "compatibility_projection_revision" in manifest:
        input_payload["compatibility_projection_revision"] = manifest[
            "compatibility_projection_revision"
        ]
    for key in (
        "canonical_revision",
        "source_manifest_revision",
        "corrections_revision",
        "registry_revision",
    ):
        _require_sha256(input_payload[key], key)
    if "compatibility_projection_revision" in input_payload:
        _require_sha256(
            input_payload["compatibility_projection_revision"],
            "compatibility_projection_revision",
        )
    if manifest.get("input_revision") != content_digest(input_payload):
        raise ValidationError(
            "derived artifact input revision does not match sidecar inputs",
            path=DERIVED_MANIFEST_PATH,
        )
    return manifest
