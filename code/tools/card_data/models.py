"""Versioned, dependency-free models for the canonical card-data pipeline.

These records deliberately model repository artifacts, not runtime game objects.
They keep source observations, normalized canonical values, corrections, and
migration approvals separate so generated ``cards.json`` remains an ABI rather
than an authoring source.
"""

from __future__ import annotations

import re
from dataclasses import dataclass, field
from enum import StrEnum
from typing import Any, Mapping

from .errors import ValidationError


SCHEMA_VERSION = 1
SHA256_RE = re.compile(r"^[0-9a-f]{64}$")
CARD_ID_RE = re.compile(r"^(?P<prefix>[A-Z]+)(?P<set_no>\d*)-(?P<card_no>\d+)(?P<suffix>[A-Z]*)$")
FIELD_PATH_RE = re.compile(r"^[a-z][a-z0-9_]*(?:\.(?:[a-z][a-z0-9_]*|\d+))*$")

CORRECTABLE_FIELD_ROOTS = frozenset(
    {
        "attribute_eng",
        "card_colors",
        "card_kind",
        "card_name_eng",
        "dp",
        "effect_description_eng",
        "evo_costs",
        "faces",
        "form_eng",
        "inherited_effect_description_eng",
        "level",
        "link_effect_description_eng",
        "max_count_in_deck",
        "play_cost",
        "printings",
        "security_effect_description_eng",
        "type_eng",
    }
)


class ModelValidationError(ValidationError, ValueError):
    """Raised when a versioned artifact record violates its schema."""


class SourceName(StrEnum):
    OFFICIAL = "official"
    DIGIMONCARD_IO = "digimoncard_io"


class AuthorityClass(StrEnum):
    OFFICIAL = "official"
    SECONDARY = "secondary"
    ADJUDICATED = "adjudicated"
    DERIVED = "derived"


class AdmissionStatus(StrEnum):
    ADMITTED = "admitted"
    PROVISIONAL = "provisional"
    RETIRED = "retired"


class FaceKind(StrEnum):
    DIGI_EGG = "digi_egg"
    DIGIMON = "digimon"
    TAMER = "tamer"
    OPTION = "option"


class FieldConflictKind(StrEnum):
    """Authority relationship behind a retained canonical disagreement."""

    OFFICIAL_PRINTING = "official_printing"
    OFFICIAL_SECONDARY = "official_secondary"
    SECONDARY_PRINTING = "secondary_printing"


class AdjudicationKind(StrEnum):
    SOURCE_ADMISSION = "source_admission"
    SOURCE_RETIREMENT = "source_retirement"
    CANONICAL_FIELD_SELECTION = "canonical_field_selection"
    RUNTIME_FIELD_CUTOVER = "runtime_field_cutover"
    COMPATIBILITY_POOL_REMOVAL = "compatibility_pool_removal"


def _require_nonempty(value: str, field_name: str) -> str:
    if not isinstance(value, str) or not value.strip():
        raise ModelValidationError(f"{field_name} must be a non-empty string")
    return value


def _require_sha256(value: str, field_name: str) -> str:
    if not isinstance(value, str) or not SHA256_RE.fullmatch(value):
        raise ModelValidationError(f"{field_name} must be a lowercase sha256 digest")
    return value


def _require_card_id(value: str) -> str:
    if not isinstance(value, str) or not CARD_ID_RE.fullmatch(value.upper()):
        raise ModelValidationError(f"invalid canonical card ID: {value!r}")
    return value.upper()


def _require_schema_version(value: int) -> int:
    if value != SCHEMA_VERSION:
        raise ModelValidationError(
            f"unsupported schema_version {value!r}; expected {SCHEMA_VERSION}"
        )
    return value


@dataclass(frozen=True, slots=True)
class SourceObservation:
    schema_version: int
    observation_id: str
    source: SourceName
    source_record_key: str
    source_url: str
    raw_payload_digest: str
    parser_version: str
    source_schema_version: str
    first_acquired_at: str
    extracted: Mapping[str, Any]

    def __post_init__(self) -> None:
        _require_schema_version(self.schema_version)
        _require_sha256(self.observation_id, "observation_id")
        _require_sha256(self.raw_payload_digest, "raw_payload_digest")
        _require_nonempty(self.source_record_key, "source_record_key")
        _require_nonempty(self.source_url, "source_url")
        _require_nonempty(self.parser_version, "parser_version")
        _require_nonempty(self.source_schema_version, "source_schema_version")
        _require_nonempty(self.first_acquired_at, "first_acquired_at")
        if not isinstance(self.extracted, Mapping):
            raise ModelValidationError("extracted must be a mapping")

    def identity_payload(self) -> dict[str, Any]:
        """Return the content-addressed identity payload.

        Source URL, record key, and acquisition time are provenance metadata;
        identity follows the design contract of raw bytes + parser/schema +
        canonicalized extracted content.
        """

        return {
            "raw_payload_digest": self.raw_payload_digest,
            "parser_version": self.parser_version,
            "source_schema_version": self.source_schema_version,
            "extracted": self.extracted,
        }


@dataclass(frozen=True, slots=True)
class ScopeObservationRef:
    observation_id: str
    gameplay_card_id: str
    printing_id: str

    def __post_init__(self) -> None:
        _require_sha256(self.observation_id, "observation_id")
        object.__setattr__(self, "gameplay_card_id", _require_card_id(self.gameplay_card_id))
        _require_nonempty(self.printing_id, "printing_id")


@dataclass(frozen=True, slots=True)
class ScopeManifest:
    schema_version: int
    source: SourceName
    scope: str
    source_locations: tuple[str, ...]
    first_acquired_at: str
    parser_version: str
    source_schema_version: str
    observation_ids: tuple[str, ...]
    observation_refs: tuple[ScopeObservationRef, ...]
    complete: bool
    expected_count: int
    failed_ids: tuple[str, ...] = ()

    def __post_init__(self) -> None:
        _require_schema_version(self.schema_version)
        _require_nonempty(self.scope, "scope")
        if not self.source_locations:
            raise ModelValidationError("a scope manifest requires source_locations")
        for value in self.source_locations:
            _require_nonempty(value, "source_locations[]")
        _require_nonempty(self.first_acquired_at, "first_acquired_at")
        _require_nonempty(self.parser_version, "parser_version")
        _require_nonempty(self.source_schema_version, "source_schema_version")
        if self.expected_count < 0:
            raise ModelValidationError("expected_count cannot be negative")
        if len(set(self.observation_ids)) != len(self.observation_ids):
            raise ModelValidationError("observation_ids must be unique")
        for value in self.observation_ids:
            _require_sha256(value, "observation_ids[]")
        referenced_ids = tuple(ref.observation_id for ref in self.observation_refs)
        if len(set(referenced_ids)) != len(referenced_ids):
            raise ModelValidationError("scope observation_refs must be unique")
        if set(referenced_ids) != set(self.observation_ids):
            raise ModelValidationError(
                "scope observation_refs must resolve every observation_id exactly once"
            )
        if self.complete and self.failed_ids:
            raise ModelValidationError("a complete scope cannot contain failed_ids")


@dataclass(frozen=True, slots=True)
class FieldProvenance:
    selected_observation_id: str
    observation_ids: tuple[str, ...]
    authority: AuthorityClass
    derived: bool = False
    adjudicated: bool = False

    def __post_init__(self) -> None:
        _require_sha256(self.selected_observation_id, "selected_observation_id")
        if self.selected_observation_id not in self.observation_ids:
            raise ModelValidationError(
                "selected_observation_id must appear in observation_ids"
            )
        if len(set(self.observation_ids)) != len(self.observation_ids):
            raise ModelValidationError("provenance observation_ids must be unique")
        for value in self.observation_ids:
            _require_sha256(value, "observation_ids[]")
        if self.authority is AuthorityClass.DERIVED and not self.derived:
            raise ModelValidationError("derived authority must set derived=true")
        if self.authority is AuthorityClass.ADJUDICATED and not self.adjudicated:
            raise ModelValidationError("adjudicated authority must set adjudicated=true")


@dataclass(frozen=True, slots=True)
class Printing:
    printing_id: str
    observation_ids: tuple[str, ...]
    language: str = "en"
    product: str | None = None
    alternate_art: bool = False

    def __post_init__(self) -> None:
        _require_nonempty(self.printing_id, "printing_id")
        _require_nonempty(self.language, "language")
        if not self.observation_ids:
            raise ModelValidationError("a printing requires at least one observation")
        for value in self.observation_ids:
            _require_sha256(value, "observation_ids[]")


@dataclass(frozen=True, slots=True)
class CardFace:
    face_id: str
    kind: FaceKind
    colors: tuple[str, ...]
    play_cost: int | None = None
    level: int | None = None
    dp: int | None = None
    effect_text: str = ""
    inherited_text: str = ""
    security_text: str = ""

    def __post_init__(self) -> None:
        _require_nonempty(self.face_id, "face_id")
        if not self.colors:
            raise ModelValidationError("a card face requires at least one color")
        for color in self.colors:
            _require_nonempty(color, "colors[]")
        for name in ("play_cost", "level", "dp"):
            value = getattr(self, name)
            if value is not None and (not isinstance(value, int) or value < 0):
                raise ModelValidationError(f"{name} must be a non-negative integer or null")


@dataclass(frozen=True, slots=True)
class FieldConflict:
    field_path: str
    candidate_observation_ids: tuple[str, ...]
    kind: FieldConflictKind
    reason: str

    def __post_init__(self) -> None:
        validate_field_path(self.field_path)
        if len(self.candidate_observation_ids) < 2:
            raise ModelValidationError("a conflict requires at least two observations")
        for value in self.candidate_observation_ids:
            _require_sha256(value, "candidate_observation_ids[]")
        _require_nonempty(self.reason, "reason")

    def requires_adjudication(self, admission: AdmissionStatus) -> bool:
        """Return whether this conflict blocks the card's current admission."""

        if self.kind is FieldConflictKind.OFFICIAL_SECONDARY:
            return False
        if self.kind is FieldConflictKind.SECONDARY_PRINTING:
            return admission is AdmissionStatus.ADMITTED
        return True


@dataclass(frozen=True, slots=True)
class CanonicalCard:
    schema_version: int
    card_id: str
    admission: AdmissionStatus
    fields: Mapping[str, Any]
    provenance: Mapping[str, FieldProvenance]
    printings: tuple[Printing, ...]
    faces: tuple[CardFace, ...]
    conflicts: tuple[FieldConflict, ...] = ()
    tombstone_reason: str | None = None

    def __post_init__(self) -> None:
        _require_schema_version(self.schema_version)
        object.__setattr__(self, "card_id", _require_card_id(self.card_id))
        if not isinstance(self.fields, Mapping):
            raise ModelValidationError("fields must be a mapping")
        if not isinstance(self.provenance, Mapping):
            raise ModelValidationError("provenance must be a mapping")
        if self.admission is AdmissionStatus.ADMITTED and not self.faces:
            raise ModelValidationError("an admitted card requires at least one face")
        if self.admission is AdmissionStatus.RETIRED and not self.tombstone_reason:
            raise ModelValidationError("a retired card requires tombstone_reason")
        for path in self.provenance:
            validate_field_path(path)


def validate_field_path(value: str) -> str:
    if not isinstance(value, str) or not FIELD_PATH_RE.fullmatch(value):
        raise ModelValidationError(f"invalid canonical field path: {value!r}")
    root = value.split(".", 1)[0]
    if root not in CORRECTABLE_FIELD_ROOTS:
        raise ModelValidationError(f"unsupported canonical field root: {root!r}")
    return value


@dataclass(frozen=True, slots=True)
class ReviewedCorrection:
    schema_version: int
    correction_id: str
    card_id: str
    field_path: str
    value: Any
    evidence_observation_ids: tuple[str, ...]
    reason: str
    reviewer: str
    reviewed_at: str

    def __post_init__(self) -> None:
        _require_schema_version(self.schema_version)
        _require_nonempty(self.correction_id, "correction_id")
        object.__setattr__(self, "card_id", _require_card_id(self.card_id))
        validate_field_path(self.field_path)
        if not self.evidence_observation_ids:
            raise ModelValidationError("a correction requires evidence")
        for value in self.evidence_observation_ids:
            _require_sha256(value, "evidence_observation_ids[]")
        _require_nonempty(self.reason, "reason")
        _require_nonempty(self.reviewer, "reviewer")
        _require_nonempty(self.reviewed_at, "reviewed_at")


@dataclass(frozen=True, slots=True)
class MigrationAdjudication:
    schema_version: int
    adjudication_id: str
    kind: AdjudicationKind
    card_id: str
    old_input_revision: str
    new_input_revision: str
    old_digest: str
    new_digest: str
    reason: str
    reviewer: str
    reviewed_at: str
    field_path: str | None = None
    selected_observation_id: str | None = None
    provenance_observation_ids: tuple[str, ...] = ()
    affected_deck_ids: tuple[str, ...] = ()
    old_value: Any = field(default=None, compare=False)
    new_value: Any = field(default=None, compare=False)

    def __post_init__(self) -> None:
        _require_schema_version(self.schema_version)
        _require_nonempty(self.adjudication_id, "adjudication_id")
        object.__setattr__(self, "card_id", _require_card_id(self.card_id))
        for name in (
            "old_input_revision",
            "new_input_revision",
            "old_digest",
            "new_digest",
        ):
            _require_sha256(getattr(self, name), name)
        if self.field_path is not None:
            # These generated compatibility fields are not canonical correction
            # targets. Only a whole-field runtime cutover may bind them.
            if not (
                self.kind is AdjudicationKind.RUNTIME_FIELD_CUTOVER
                and self.field_path in {"dual", "xros_req"}
            ):
                validate_field_path(self.field_path)
        if self.kind in {
            AdjudicationKind.CANONICAL_FIELD_SELECTION,
            AdjudicationKind.RUNTIME_FIELD_CUTOVER,
        } and not self.field_path:
            raise ModelValidationError(f"{self.kind.value} requires field_path")
        if self.selected_observation_id is not None:
            _require_sha256(self.selected_observation_id, "selected_observation_id")
        if len(set(self.provenance_observation_ids)) != len(
            self.provenance_observation_ids
        ):
            raise ModelValidationError(
                "provenance_observation_ids must be unique"
            )
        for value in self.provenance_observation_ids:
            _require_sha256(value, "provenance_observation_ids[]")
        if self.kind is AdjudicationKind.CANONICAL_FIELD_SELECTION:
            if self.selected_observation_id is None:
                raise ModelValidationError(
                    "canonical_field_selection requires selected_observation_id"
                )
            if not self.provenance_observation_ids:
                raise ModelValidationError(
                    "canonical_field_selection requires candidate observation provenance"
                )
            if self.selected_observation_id not in self.provenance_observation_ids:
                raise ModelValidationError(
                    "selected_observation_id must appear in provenance_observation_ids"
                )
        _require_nonempty(self.reason, "reason")
        _require_nonempty(self.reviewer, "reviewer")
        _require_nonempty(self.reviewed_at, "reviewed_at")

    def is_current(
        self,
        *,
        old_input_revision: str,
        new_input_revision: str,
        old_digest: str,
        new_digest: str,
    ) -> bool:
        """Return whether this approval still matches the exact reviewed inputs."""

        return (
            self.old_input_revision == old_input_revision
            and self.new_input_revision == new_input_revision
            and self.old_digest == old_digest
            and self.new_digest == new_digest
        )
